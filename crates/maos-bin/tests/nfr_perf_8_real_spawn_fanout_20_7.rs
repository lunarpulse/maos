#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 20-7 AC6 (D-20-7-06 A) — the J1 real-spawn fan-out bench, NFR-Perf-8:
//! real T3 `worker-cli-fixture` Workers through the real `WorkerSupervisor`
//! (the `worker_crash_corpus_16_3` harness shape) via the production entrypoint
//! `run_cli_wrapper_manifest`, dispatched at a fixed rate with up to N
//! concurrent. Task-dispatch latency is the harness's dispatch stamp (taken
//! immediately before the supervised run, in the stead of a delegated
//! `task.assign` journal row) to the Worker's `cli_wrapper_loaded` event. The
//! original in-process bench (`orchestrator_inproc_dispatch`) spawns no Worker.
//!
//! Two processes: the test (parent) re-execs itself as the fan-out child with
//! its stdout piped and reads the Workers' own `cli_wrapper_loaded` /
//! `cli_wrapper_exit` event lines (printed by `run_cli_wrapper_manifest`),
//! stamping each on arrival; the child prints one `MAOS-20-7-TASK` line per
//! finished task naming its dispatch stamp and Worker pid.
//!
//! Selector (`MAOS_20_7_FANOUT`), absent => a named SKIP so the workspace suite
//! stays fast and the claim is only ever the runner's or the smoke job's:
//!
//! * `smoke` — at most 5 Workers, 60 s, 1 task/s (hosted substrate): requires
//!   at least two concurrently loaded Workers, distinct identities, paired
//!   lifecycle events and lossless producer/admission/completion accounting.
//!   This proves harness mechanics, NOT NFR-Perf-8's required rate/population.
//! * `soak` — 50 Workers, 3600 s, 10 tasks/s (Tier W, weekly): additionally the
//!   `nfr-perf-8-fanout-dispatch-p99` row of `xtask/perf-budgets.toml`.
//!
//! `MAOS_20_7_FANOUT_WORKERS` / `_SECS` / `_RATE` override the profile.
//! `MAOS_20_7_FANOUT_IN_PROCESS=1` is the AC6.3 plant: it substitutes
//! in-process task handles for the T3 spawn, and the distinct-CID assertion reds.

#[path = "../../maos-bench/src/budgets.rs"]
mod budgets;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use maos_bin::supervision::{WorkerSupervisor, WorkerTask};
use maos_bin::worker_spawn::{parse_run_args, run_cli_wrapper_manifest};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;

const NONCE: u64 = 0x20_07;
const CHILD: &str = "MAOS_20_7_FANOUT_CHILD";
const TASK_PREFIX: &str = "MAOS-20-7-TASK ";

struct Profile {
    name: &'static str,
    workers: usize,
    secs: u64,
    rate: u64,
}

fn env_number(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
}

fn profile() -> Option<Profile> {
    let (name, workers, secs) = match std::env::var("MAOS_20_7_FANOUT").ok()?.as_str() {
        "smoke" => ("smoke", 5, 60),
        "soak" => ("soak", 50, 3600),
        other => panic!("MAOS_20_7_FANOUT must be `smoke` or `soak`, not {other:?}"),
    };
    Some(Profile {
        name,
        workers: env_number("MAOS_20_7_FANOUT_WORKERS").map_or(workers, |n| n as usize),
        secs: env_number("MAOS_20_7_FANOUT_SECS").unwrap_or(secs),
        rate: env_number("MAOS_20_7_FANOUT_RATE").unwrap_or(if name == "smoke" { 1 } else { 10 }),
    })
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos() as u64
}

fn write_manifest(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("worker-fanout.toml");
    std::fs::write(
        &path,
        "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
         argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=delay-then-complete\"]\n\
         output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
         recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
         stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
         shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
         [author]\nname = \"MAOS Project\"\n",
    )
    .expect("write worker manifest");
    path
}

fn supervisor_world(
    dir: &Path,
) -> (
    Arc<TransparencyLogAdapter>,
    Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    Arc<WorkerSupervisor>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let policy = Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new());
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x20; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let memory_db = dir.join("memory.sqlite");
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            dir.join("private"),
            4,
        )),
        Arc::new(
            maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                .expect("open shared memory"),
        ),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                .expect("open principal index"),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        memory,
        Arc::clone(&iac),
        Arc::clone(&halt_registry),
        Arc::clone(&telemetry),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson"))
            .expect("open lifecycle journal"),
    );
    let crash_detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
        scheduler.scbs(),
        Arc::clone(&tl),
        Arc::clone(&halt_registry),
        Arc::clone(&capability),
        Arc::clone(&iac),
        Arc::clone(&telemetry),
        Arc::clone(&journal),
    ));
    Arc::get_mut(&mut scheduler)
        .expect("scheduler Arc strong_count == 1")
        .set_crash_detector(Arc::clone(&crash_detector));
    let supervisor = Arc::new(WorkerSupervisor::new(
        scheduler,
        crash_detector,
        Arc::clone(&tl),
        journal,
        halt_registry,
        iac,
        tokio::runtime::Handle::current(),
        NONCE,
        tokio_util::sync::CancellationToken::new(),
    ));
    (tl, capability, policy, supervisor)
}

/// The fan-out child: dispatch `rate` tasks/s for `secs`, up to `workers`
/// concurrent, every task one real supervised T3 Worker. Prints one
/// `MAOS-20-7-TASK` line per finished task.
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn fan_out_child(profile: Profile) {
    let dir = std::env::temp_dir().join(format!("maos-fanout-20-7-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let manifest_path = write_manifest(&dir);
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    let run = parse_run_args(
        ["run", manifest_path.to_str().expect("utf-8 path"), "--once"].map(str::to_owned),
    )
    .expect("run args")
    .expect("a run invocation");
    let (tl, capability, policy, supervisor) = supervisor_world(&dir);
    let in_process = std::env::var_os("MAOS_20_7_FANOUT_IN_PROCESS").is_some();
    let slots = Arc::new(tokio::sync::Semaphore::new(profile.workers));
    assert!(profile.workers > 0 && profile.rate > 0 && profile.secs > 0);
    let started = tokio::time::Instant::now();
    let mut tasks = Vec::new();
    let scheduled = profile.secs * profile.rate;
    let mut admitted = 0u64;
    let mut peak = 0usize;
    for index in 0..scheduled {
        tokio::time::sleep_until(
            started + Duration::from_nanos(index * 1_000_000_000 / profile.rate),
        )
        .await;
        // Never let admission delay the producer and silently skip arrivals.
        let dispatch_ns = now_ns();
        let Ok(permit) = Arc::clone(&slots).try_acquire_owned() else {
            continue;
        };
        admitted += 1;
        peak = peak.max(profile.workers - slots.available_permits());
        let task_index = index as usize + 1;
        let (manifest, run) = (manifest.clone(), run.clone());
        let (tl, capability, policy) = (
            Arc::clone(&tl),
            Arc::clone(&capability),
            Arc::clone(&policy),
        );
        let supervisor = Arc::clone(&supervisor);
        tasks.push(tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let outcome = if in_process {
                // AC6.3 plant: an in-process stand-in spawns nothing real.
                std::thread::sleep(Duration::from_secs(2));
                Ok(0u32)
            } else {
                run_cli_wrapper_manifest(
                    &manifest,
                    &run,
                    tl,
                    capability,
                    policy,
                    None,
                    None,
                    None,
                    None,
                    false,
                    supervisor.as_ref(),
                    WorkerTask::TopologyEntry { index: task_index },
                    &maos_kernel_core::security::crypto::RingCryptoProvider,
                    None,
                )
                .map(|worker| worker.spirit_pid)
                .map_err(|error| error.to_string())
            };
            match outcome {
                Ok(pid) => println!(
                    "{TASK_PREFIX}{}",
                    serde_json::json!({"index": task_index, "spirit_pid": pid, "dispatch_ns": dispatch_ns})
                ),
                Err(error) => println!(
                    "{TASK_PREFIX}{}",
                    serde_json::json!({"index": task_index, "error": error, "dispatch_ns": dispatch_ns})
                ),
            }
        }));
    }
    for task in tasks {
        task.await.expect("task thread");
    }
    println!(
        "MAOS-20-7-LOAD {}",
        serde_json::json!({"scheduled":scheduled,"admitted":admitted,"peak":peak,"elapsed_ns":started.elapsed().as_nanos() as u64})
    );
    supervisor.stop_workers();
    let _ = std::fs::remove_dir_all(&dir);
}

#[derive(Default)]
struct Observed {
    /// spirit_pid -> (child_pid, container_id, loaded arrival ns).
    loaded: Vec<(u64, u64, String, u64)>,
    exited: Vec<(u64, u64)>,
    load: Option<(u64, u64, usize, u64)>,
    /// (index, spirit_pid, dispatch_ns); `error` tasks are dropped ones.
    tasks: Vec<(u64, Option<u64>, u64)>,
    errors: Vec<String>,
}

#[test]
fn real_t3_workers_fan_out_through_the_real_supervisor() {
    let Some(profile) = profile() else {
        eprintln!(
            "SKIP: NFR-Perf-8 real-spawn fan-out: set MAOS_20_7_FANOUT=smoke|soak (hosted smoke / Tier W soak only)"
        );
        return;
    };
    if !t3_substrate::require_t3("NFR-Perf-8 real-spawn fan-out") {
        return;
    }
    if std::env::var_os(CHILD).is_some() {
        if !t3_substrate::require_t3("NFR-Perf-8 real-spawn fan-out") {
            return;
        }
        fixtures::profile_binary_beside_test("worker-cli-fixture");
        fan_out_child(profile);
        return;
    }
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "real_t3_workers_fan_out_through_the_real_supervisor",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn the fan-out child");
    let observed = Arc::new(Mutex::new(Observed::default()));
    let reader = {
        let observed = Arc::clone(&observed);
        let stdout = child.stdout.take().expect("child stdout");
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let arrived = now_ns();
                let mut observed = observed.lock().expect("observed");
                if let Some(json) = line.strip_prefix("MAOS-20-7-LOAD ") {
                    let v: serde_json::Value = serde_json::from_str(json).expect("load line");
                    observed.load = Some((
                        v["scheduled"].as_u64().unwrap(),
                        v["admitted"].as_u64().unwrap(),
                        v["peak"].as_u64().unwrap() as usize,
                        v["elapsed_ns"].as_u64().unwrap(),
                    ));
                    continue;
                }
                if let Some(json) = line.strip_prefix(TASK_PREFIX) {
                    let value: serde_json::Value = serde_json::from_str(json).expect("task line");
                    match value["spirit_pid"].as_u64() {
                        Some(pid) => observed.tasks.push((
                            value["index"].as_u64().unwrap_or(0),
                            Some(pid),
                            value["dispatch_ns"].as_u64().unwrap_or(0),
                        )),
                        None => {
                            observed
                                .tasks
                                .push((value["index"].as_u64().unwrap_or(0), None, 0));
                            observed.errors.push(value["error"].to_string());
                        }
                    }
                    continue;
                }
                let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                match event["event"].as_str() {
                    Some("cli_wrapper_loaded") => observed.loaded.push((
                        event["spirit_pid"].as_u64().unwrap_or(0),
                        event["child_pid"].as_u64().unwrap_or(0),
                        event["container_id"].as_str().unwrap_or("").to_owned(),
                        arrived,
                    )),
                    Some("cli_wrapper_exit") => observed.exited.push((
                        event["child_pid"].as_u64().expect("exit child identity"),
                        arrived,
                    )),
                    _ => {}
                }
            }
        })
    };
    let status = child.wait().expect("wait for the fan-out child");
    reader.join().expect("reader thread");
    let observed = observed.lock().expect("observed");
    let ran = observed
        .tasks
        .iter()
        .filter(|(_, pid, _)| pid.is_some())
        .count();
    let dropped = observed.tasks.len() - ran;
    let cids: HashSet<&str> = observed
        .loaded
        .iter()
        .map(|(_, _, cid, _)| cid.as_str())
        .collect();
    let child_pids: HashSet<u64> = observed.loaded.iter().map(|(_, pid, _, _)| *pid).collect();
    println!(
        "fan-out {}: {} tasks, {ran} ran, {dropped} dropped, {} loaded events, {} exit events, {} distinct CIDs, {} distinct child pids",
        profile.name,
        observed.tasks.len(),
        observed.loaded.len(),
        observed.exited.len(),
        cids.len(),
        child_pids.len()
    );
    assert!(status.success(), "the fan-out child failed: {status:?}");
    assert!(ran > 0, "no Worker ran");
    // Establish real container identity before evaluating population/timing:
    // the in-process stand-in must fail this assertion, not a missing-event check.
    assert_eq!(
        cids.len(),
        ran,
        "AC6.2: every Worker is a distinct real container (CIDs {} != Workers {ran})",
        cids.len()
    );
    let required_population = if profile.name == "smoke" {
        profile.workers.min(2)
    } else {
        profile.workers
    };
    let (scheduled, admitted, peak, elapsed_ns) =
        observed.load.expect("producer accounting required");
    assert_load(
        (profile.secs * profile.rate, required_population),
        scheduled,
        admitted,
        ran,
        peak,
        elapsed_ns,
        profile.secs,
    );
    let exits: HashSet<_> = observed.exited.iter().map(|(pid, _)| *pid).collect();
    assert_eq!(
        exits, child_pids,
        "exit events must pair with each loaded Worker child identity"
    );
    let mut population = Vec::new();
    for (_, child, _, loaded) in &observed.loaded {
        let (_, exited) = observed
            .exited
            .iter()
            .find(|(p, _)| p == child)
            .expect("matching exit");
        assert!(loaded <= exited, "Worker exit preceded its load");
        population.push((*loaded, 1i64));
        population.push((*exited, -1i64));
    }
    population.sort_unstable();
    let (mut active, mut loaded_peak) = (0i64, 0i64);
    for (_, delta) in population {
        active += delta;
        loaded_peak = loaded_peak.max(active);
    }
    assert!(
        loaded_peak >= required_population as i64,
        "required concurrently loaded Worker population not reached"
    );
    assert_eq!(dropped, 0, "dropped tasks: {:?}", observed.errors);
    assert_eq!(
        child_pids.len(),
        ran,
        "AC6.2: distinct runtime-client child pids"
    );
    assert_eq!(
        observed.loaded.len(),
        ran,
        "one cli_wrapper_loaded per Worker"
    );
    assert_eq!(
        observed.exited.len(),
        ran,
        "one cli_wrapper_exit per Worker"
    );
    let mut latencies_us: Vec<f64> = observed
        .tasks
        .iter()
        .filter_map(|(_, pid, dispatch_ns)| {
            let pid = (*pid)?;
            let (_, _, _, loaded_ns) = observed.loaded.iter().find(|(p, _, _, _)| *p == pid)?;
            Some(loaded_ns.saturating_sub(*dispatch_ns) as f64 / 1_000.0)
        })
        .collect();
    latencies_us.sort_by(|a, b| a.partial_cmp(b).expect("finite latency"));
    let p99 = latencies_us[(latencies_us.len() * 99)
        .div_ceil(100)
        .clamp(1, latencies_us.len())
        - 1];
    println!(
        "fan-out {}: dispatch latency P99 = {p99:.0} us over {} tasks",
        profile.name,
        latencies_us.len()
    );
    assert!(
        budgets::report("nfr-perf-8-fanout-dispatch-p99", p99),
        "NFR-Perf-8 dispatch P99 {p99} us breaches its binding budget"
    );
}

fn assert_load(
    (expected, workers): (u64, usize),
    scheduled: u64,
    admitted: u64,
    completed: usize,
    peak: usize,
    elapsed_ns: u64,
    secs: u64,
) {
    assert_eq!(scheduled, expected, "fixed-rate scheduled population");
    assert_eq!(
        admitted, scheduled,
        "no arrivals may be silently dropped at admission"
    );
    assert_eq!(
        completed as u64, scheduled,
        "all scheduled tasks must complete"
    );
    assert!(peak >= workers, "required concurrent Worker population");
    assert!(
        elapsed_ns <= (secs + 30) * 1_000_000_000,
        "completion rate below required load"
    );
}

#[test]
#[should_panic(expected = "no arrivals may be silently dropped")]
fn slow_serial_admission_cannot_pass_the_fanout_contract() {
    assert_load((600, 5), 600, 60, 60, 1, 60_000_000_000, 60);
}

#[test]
#[should_panic(expected = "required concurrent Worker population")]
fn serial_population_cannot_pass_the_fanout_contract() {
    assert_load((600, 5), 600, 600, 600, 1, 60_000_000_000, 60);
}
