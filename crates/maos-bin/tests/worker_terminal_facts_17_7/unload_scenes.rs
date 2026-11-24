//! S5: planned unload remains owned when the Worker on_unload hook fails.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_bin::supervision::{OnUnloadFault, SupervisorSeams, WorkerSupervisor, WorkerTask};
use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::iac::FrameKind;
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

const BOOT_NONCE: u64 = 0x17_07_51;
type Row = (u32, String, serde_json::Value, FrameKind);

fn is_unload_budget_exceeded((_, intent, _, kind): &Row) -> bool {
    *kind == FrameKind::BudgetExceeded && intent == "hook.budget.on_unload"
}

#[test]
fn s5_budget_oracle_counts_exceeded_kind_not_a_delayed_warning() {
    let rows: Vec<Row> = [
        (FrameKind::BudgetWarning, 2.422539097),
        (FrameKind::BudgetExceeded, 2.622919372),
    ]
    .into_iter()
    .map(|(kind, ratio)| {
        (
            42,
            "hook.budget.on_unload".into(),
            serde_json::json!({"ratio_breached":ratio,"hook_name":"on_unload","cap_seconds":1}),
            kind,
        )
    })
    .collect();
    let exceeded: Vec<_> = rows
        .iter()
        .filter(|row| is_unload_budget_exceeded(row))
        .collect();
    assert_eq!(
        exceeded.len(),
        1,
        "a delayed warning is not another exceeded frame"
    );
    assert_eq!(exceeded[0].3, FrameKind::BudgetExceeded);
}

pub(super) struct World {
    pub(super) tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    pub(super) scheduler: Arc<SpiritSchedulerAdapter>,
    pub(super) halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    pub(super) crash_detector: Arc<maos_kernel_core::supervision::CrashDetector>,
    pub(super) journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    pub(super) iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
}

impl World {
    pub(super) fn new(dir: &Path) -> Self {
        super::init_clock();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
        let policy = Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new());
        let capability = Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0; 32]),
                BOOT_NONCE,
                Arc::clone(&policy),
                maos_kernel_core::capability::cap_audit::channel().0,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ),
        );
        let memory_db = dir.join("memory.sqlite");
        let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private-memory"),
                4,
            )),
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db).unwrap(),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .unwrap(),
            ),
            Arc::clone(&tl),
        ));
        let telemetry = Arc::new(IacRtMetrics::new());
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
            Arc::clone(&tl),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(SpiritSchedulerAdapter::new(
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
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson")).unwrap(),
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
            .expect("scheduler has a single owner")
            .set_crash_detector(Arc::clone(&crash_detector));
        Self {
            tl,
            capability,
            policy,
            scheduler,
            halt_registry,
            crash_detector,
            journal,
            iac,
        }
    }

    fn supervisor(&self, seams: SupervisorSeams) -> Arc<WorkerSupervisor> {
        Arc::new(
            WorkerSupervisor::new(
                Arc::clone(&self.scheduler),
                Arc::clone(&self.crash_detector),
                Arc::clone(&self.tl),
                Arc::clone(&self.journal),
                Arc::clone(&self.halt_registry),
                Arc::clone(&self.iac),
                tokio::runtime::Handle::current(),
                BOOT_NONCE,
                tokio_util::sync::CancellationToken::new(),
            )
            .with_seams(seams),
        )
    }

    fn rows(&self) -> Vec<Row> {
        self.tl
            .query_frames(FrameFilter::default())
            .expect("read transparency log")
            .into_iter()
            .map(|row| {
                (
                    row.spirit_pid,
                    row.intent,
                    serde_json::from_slice(&row.payload_redacted)
                        .unwrap_or(serde_json::Value::Null),
                    row.kind,
                )
            })
            .collect()
    }
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "maos-terminal-s5-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("create unique S5 scratch dir");
    dir
}

fn t3_manifest(dir: &Path) -> toml::Value {
    let manifest = dir.join("worker.toml");
    std::fs::write(&manifest,
        "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang\"]\noutput_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\nrecovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n[author]\nname = \"MAOS Project\"\n").expect("write T3 manifest");
    toml::from_str(&std::fs::read_to_string(manifest).unwrap()).expect("parse T3 manifest")
}

async fn exercise_unload(
    fault: OnUnloadFault,
    cap: Option<u32>,
    expected_budget_row: bool,
    scene_name: &str,
) {
    if !super::scene(scene_name) {
        return;
    }
    let dir = scratch();
    let runtime_home = dir.join("runtime-home");
    std::fs::create_dir_all(&runtime_home).expect("create isolated runtime home");
    let runtime_home = super::t3_substrate::isolated_runtime_home(&runtime_home).to_path_buf();
    let home = std::env::var_os("HOME");
    let maos_home = std::env::var_os("MAOS_HOME");
    let xdg_home = std::env::var_os("XDG_DATA_HOME");
    std::env::set_var("HOME", &runtime_home);
    std::env::set_var("MAOS_HOME", &runtime_home);
    std::env::set_var("XDG_DATA_HOME", &runtime_home);
    let world = World::new(&dir);
    let supervisor = world.supervisor(SupervisorSeams {
        on_unload_fault: Some(fault),
        hook_cap_override: cap,
        ..Default::default()
    });
    let manifest = t3_manifest(&dir);
    let target = PathBuf::from(env!("CARGO_BIN_EXE_maos"))
        .parent()
        .expect("test binary parent")
        .to_path_buf();
    let old_path = std::env::var_os("PATH");
    let mut paths = vec![target];
    if let Some(path) = old_path.as_ref() {
        paths.extend(std::env::split_paths(path));
    }
    std::env::set_var("PATH", std::env::join_paths(paths).expect("join test PATH"));
    let run = maos_bin::worker_spawn::RunArgs {
        manifest_path: dir.join("worker.toml").display().to_string(),
        live: false,
        once: false,
        scene: None,
        epic: None,
    };
    let tl = Arc::clone(&world.tl);
    let capability = Arc::clone(&world.capability);
    let policy = Arc::clone(&world.policy);
    let run_supervisor = Arc::clone(&supervisor);
    let run_task = tokio::task::spawn_blocking(move || {
        maos_bin::worker_spawn::run_cli_wrapper_manifest(
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
            run_supervisor.as_ref(),
            WorkerTask::Standalone,
            &maos_kernel_core::security::crypto::RingCryptoProvider,
            None,
        )
        .map_err(|error| error.to_string())
    });
    let bound = wait_for(Duration::from_secs(60), || {
        supervisor
            .bindings()
            .first()
            .is_some_and(|b| b.child_pid.is_some())
    })
    .await;
    assert!(bound, "real T3 Worker must bind before the planned unload");
    let binding = supervisor
        .bindings()
        .into_iter()
        .find(|b| b.child_pid.is_some())
        .expect("bound Worker");
    let pid = binding.spirit_pid;
    let cid = cid_from_lifecycle(&dir.join("lifecycle.ndjson"));
    assert!(
        cid.is_some(),
        "SandboxApplied lifecycle journal must identify the owned CID"
    );
    let cid = cid.unwrap();
    // The production root wrapper: unload, then the same post-unload registry
    // stop `main` installs, whatever the failing hook did (AC4.6).
    let unload_started = Instant::now();
    let stop_after_unload = |unloaded| supervisor.stop_binding(unloaded);
    let report = maos_bin::supervision::unload_all_loaded(
        &world.scheduler,
        &world.halt_registry,
        Some(&stop_after_unload),
    )
    .await;
    let unload_returned = Instant::now();
    let teardown_budget = Duration::from_secs(25)
        + if expected_budget_row {
            Duration::from_secs(1)
        } else {
            Duration::ZERO
        };
    let (_, unload_error) = report
        .failed
        .iter()
        .find(|(failed, _)| *failed == pid)
        .expect("on_unload fault must fail the scheduler unload");
    if expected_budget_row {
        assert!(
            unload_error.contains("exceeded budget: on_unload") && unload_error.contains("cap 1s"),
            "{unload_error}"
        );
    } else {
        assert!(
            unload_error.starts_with("hook on_unload panicked"),
            "{unload_error}"
        );
    }
    assert!(
        unload_started.elapsed() <= teardown_budget,
        "unload and the door's post-unload registry stop must fit the 25s teardown budget plus hook cap"
    );
    let runtime = std::env::var("MAOS_T3_RUNTIME").unwrap_or_else(|_| "podman".to_owned());
    let exists_cid = cid.clone();
    let exists_runtime = runtime.clone();
    let exists = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("timeout")
                .args(["5s", &exists_runtime, "container", "exists", &exists_cid])
                .status()
                .expect("check owned CID existence")
        }),
    )
    .await
    .expect("CID existence query bounded")
    .expect("exists task");
    match exists.code() {
        Some(0) => {
            let running_cid = cid.clone();
            let running_runtime = runtime.clone();
            let running = tokio::time::timeout(
                Duration::from_secs(5),
                tokio::task::spawn_blocking(move || {
                    std::process::Command::new("timeout")
                        .args([
                            "5s",
                            &running_runtime,
                            "inspect",
                            "--format",
                            "{{.State.Running}}",
                            &running_cid,
                        ])
                        .output()
                        .expect("inspect owned CID")
                }),
            )
            .await
            .expect("CID running-state query bounded")
            .expect("inspect task");
            assert!(
                running.status.success(),
                "existing owned CID must be inspectable"
            );
            assert_eq!(
                String::from_utf8_lossy(&running.stdout).trim(),
                "false",
                "CID must stop within five seconds of unload returning"
            );
        }
        Some(1) => {}
        code => {
            panic!("container exists query failed instead of proving presence/absence: {code:?}")
        }
    }
    let post_return_elapsed = unload_returned.elapsed();
    assert!(
        post_return_elapsed <= Duration::from_secs(5),
        "owned CID must be stopped or removed within five seconds of unload returning"
    );

    // wait_and_finalize persists the real T3 terminal row after the planned stop.
    let run_result = match tokio::time::timeout(Duration::from_secs(30), run_task).await {
        Ok(joined) => joined.expect("worker blocking task joined"),
        Err(_) => panic!(
            "the failed on_unload hook must not block runtime teardown: \
             run_cli_wrapper_manifest did not return within 30s of unload returning\n{}",
            stall_diagnostics(&runtime, &cid, binding.child_pid)
        ),
    };
    assert!(
        run_result.is_ok(),
        "real run_cli_wrapper_manifest should finalize: {run_result:?}"
    );
    if let Some(path) = old_path {
        std::env::set_var("PATH", path);
    } else {
        std::env::remove_var("PATH");
    }
    if let Some(value) = home {
        std::env::set_var("HOME", value);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(value) = maos_home {
        std::env::set_var("MAOS_HOME", value);
    } else {
        std::env::remove_var("MAOS_HOME");
    }
    if let Some(value) = xdg_home {
        std::env::set_var("XDG_DATA_HOME", value);
    } else {
        std::env::remove_var("XDG_DATA_HOME");
    }

    let rows = world.rows();
    let at_pid: Vec<_> = rows
        .iter()
        .filter(|(row_pid, _, _, _)| *row_pid == pid)
        .collect();
    let hook_or_lifecycle = at_pid.iter().any(|(_, intent, payload, _)| {
        intent == "lifecycle.unload" && payload["lifecycle_event"] == "Unload"
    });
    assert!(
        hook_or_lifecycle,
        "kernel unload/hook dispatch must be journaled at the Worker's real pid"
    );
    let receipts: Vec<_> = at_pid
        .iter()
        .filter(|(_, intent, payload, _)| intent == "planned_unload" && !payload.is_null())
        .collect();
    assert_eq!(
        receipts.len(),
        1,
        "planned unload writes exactly one receipt"
    );
    // S5: the `on_unload_hook_failure` writer may run when ledger entries
    // remain (`scheduler_loop.rs:689-723`); any other orphan/crash/fault row
    // would classify a planned stop as a crash.
    let faults: Vec<_> = at_pid
        .iter()
        .filter(|(_, intent, payload, _)| {
            intent.starts_with("fault.")
                || (intent == "task.orphaned" && payload["cause"] != "on_unload_hook_failure")
                || intent == "unplanned_crash"
                || intent == "lifecycle.crash"
                || payload["cause"]
                    .as_str()
                    .is_some_and(|cause| cause.starts_with("fault."))
        })
        .collect();
    assert!(
        faults.is_empty(),
        "planned stop must not create a crash/orphan row: {faults:?}"
    );
    let exceeded: Vec<_> = at_pid
        .iter()
        .filter(|row| is_unload_budget_exceeded(row))
        .collect();
    assert_eq!(
        exceeded.len(),
        usize::from(expected_budget_row),
        "budget-exceeded evidence must match the injected hook behavior: {exceeded:?}"
    );
    if expected_budget_row {
        assert!(exceeded
            .iter()
            .any(|(_, _, payload, _)| payload["hook_name"] == "on_unload"
                && payload["cap_seconds"] == 1));
    }
    let exits: Vec<_> = at_pid
        .iter()
        .filter(|(_, intent, _, _)| intent == "cli.subprocess.exit")
        .collect();
    assert_eq!(exits.len(), 1, "T3 bridge writes one exit record");
    let exit_container = &exits[0].2["container"];
    let removed = exit_container["removed"] == true && exit_container["residue"].is_null();
    let residue = exit_container["residue"].as_str();
    let one_residue = exit_container["removed"] == false
        && residue.is_some_and(|residue| {
            matches!(
                residue,
                "kill_failed"
                    | "termination_unconfirmed"
                    | "remove_failed"
                    | "remove_timeout"
                    | "exists_failed"
                    | "still_exists"
                    | "volume_cleanup_unconfirmed"
                    | "runtime_client_unconfirmed"
            )
        });
    assert!(
        removed || one_residue,
        "container must be removed or have one typed residue: {exit_container}"
    );
    if removed {
        let exists_cid = cid.clone();
        let exists = tokio::task::spawn_blocking(move || {
            std::process::Command::new("timeout")
                .args(["8s", &runtime, "container", "exists", &exists_cid])
                .status()
                .expect("check owned CID removal")
        })
        .await
        .expect("exists task");
        assert_eq!(
            exists.code(),
            Some(1),
            "exit-row CID absence requires container exists exit code 1"
        );
    } else {
        supervisor.report_unresolved_containers();
    }
    let _ = std::fs::remove_dir_all(dir);
}

fn cid_from_lifecycle(path: &Path) -> Option<String> {
    for line in std::fs::read_to_string(path).ok()?.lines() {
        let entry: serde_json::Value = serde_json::from_str(line).ok()?;
        if entry["lifecycle_event"] == "SandboxApplied" {
            let bytes = entry["payload"].as_array()?;
            let payload = bytes
                .iter()
                .map(|byte| byte.as_u64().and_then(|n| u8::try_from(n).ok()))
                .collect::<Option<Vec<_>>>()?;
            return serde_json::from_slice::<serde_json::Value>(&payload).ok()?["container_id"]
                .as_str()
                .map(str::to_owned);
        }
    }
    None
}

/// What a stalled `run_cli_wrapper_manifest` is waiting on, captured at the
/// moment the join bound expires: this test process's threads (kernel wait
/// channel), every other process that holds one of this process's pipes (the
/// bridge reader threads block until the last holder of a child pipe closes
/// it), the runtime client's `/proc` state, the process tree and the CID's
/// runtime state. Best effort and bounded; every probe failure is reported,
/// never hidden.
fn stall_diagnostics(runtime: &str, cid: &str, child_pid: Option<u32>) -> String {
    use std::fmt::Write as _;
    let mut report = String::new();
    let own = std::process::id();
    let _ = writeln!(report, "-- test process {own} threads (comm: wchan) --");
    match std::fs::read_dir("/proc/self/task") {
        Ok(tasks) => {
            for task in tasks.flatten() {
                let path = task.path();
                let read = |name: &str| {
                    std::fs::read_to_string(path.join(name))
                        .map(|text| text.trim().to_owned())
                        .unwrap_or_else(|error| format!("<{error}>"))
                };
                let _ = writeln!(
                    report,
                    "{}: {} : {}",
                    task.file_name().to_string_lossy(),
                    read("comm"),
                    read("wchan")
                );
            }
        }
        Err(error) => {
            let _ = writeln!(report, "<cannot list threads: {error}>");
        }
    }
    let pipe_ends: std::collections::BTreeSet<String> = std::fs::read_dir("/proc/self/fd")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|fd| std::fs::read_link(fd.path()).ok())
        .map(|target| target.to_string_lossy().into_owned())
        .filter(|target| target.starts_with("pipe:["))
        .collect();
    let _ = writeln!(
        report,
        "-- other processes sharing this process's {} pipe(s) --",
        pipe_ends.len()
    );
    for process in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
            .filter(|pid| *pid != own)
        else {
            continue;
        };
        let Ok(fds) = std::fs::read_dir(process.path().join("fd")) else {
            continue;
        };
        let held: Vec<String> = fds
            .flatten()
            .filter_map(|fd| {
                let target = std::fs::read_link(fd.path())
                    .ok()?
                    .to_string_lossy()
                    .into_owned();
                pipe_ends
                    .contains(&target)
                    .then(|| format!("fd{}={target}", fd.file_name().to_string_lossy()))
            })
            .collect();
        if !held.is_empty() {
            let cmdline = std::fs::read(process.path().join("cmdline"))
                .map(|bytes| String::from_utf8_lossy(&bytes).replace('\0', " "))
                .unwrap_or_default();
            let _ = writeln!(
                report,
                "pid {pid} [{}] holds {}",
                cmdline.chars().take(240).collect::<String>(),
                held.join(", ")
            );
        }
    }
    if let Some(pid) = child_pid {
        let _ = writeln!(
            report,
            "-- runtime client pid {pid} /proc stat --\n{}",
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .unwrap_or_else(|error| format!("<{error}>"))
        );
    }
    let probe = |program: &str, args: &[&str]| {
        let output = std::process::Command::new("timeout")
            .arg("5s")
            .arg(program)
            .args(args)
            .output();
        match output {
            Ok(output) => format!(
                "{:?}\n{}{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => format!("<cannot run {program}: {error}>"),
        }
    };
    let _ = writeln!(
        report,
        "-- process tree --\n{}",
        probe("ps", &["-eo", "pid,ppid,stat,etime,args", "--forest"])
            .chars()
            .take(12_000)
            .collect::<String>()
    );
    let _ = writeln!(
        report,
        "-- CID state ({cid}) --\n{}",
        probe(runtime, &["inspect", "--format", "{{json .State}}", cid])
    );
    report
}

async fn wait_for(budget: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    predicate()
}

/// Hang guard for one whole scene (setup, the 60 s bind wait, teardown, the
/// 30 s run join and the CID probes sum to well under half of this). It
/// bounds a scene that blocks, it is not an expectation about a healthy one.
const SCENE_HARD_BOUND: Duration = Duration::from_secs(240);

/// How long a failed scene waits for its blocking threads before abandoning
/// them. A `#[tokio::test]` runtime's `Drop` waits for every `spawn_blocking`
/// task with no bound, so a failure while `run_cli_wrapper_manifest` is still
/// blocked kept the test process alive until the CI job's own timeout (hosted
/// run 45447011202: the assertion failed at +30 s and the test was still
/// "running" 1000+ s later). Owning the runtime lets a failure return promptly.
const FAILED_SCENE_RUNTIME_REAP: Duration = Duration::from_secs(5);

fn run_scene(fault: OnUnloadFault, cap: Option<u32>, expected_budget_row: bool, scene_name: &str) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("build the S5 scene runtime");
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(async {
            tokio::time::timeout(
                SCENE_HARD_BOUND,
                exercise_unload(fault, cap, expected_budget_row, scene_name),
            )
            .await
            .expect("S5 scene exceeded its hard bound");
        })
    }));
    runtime.shutdown_timeout(FAILED_SCENE_RUNTIME_REAP);
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[test]
fn s5_on_unload_panic_before_stop_keeps_planned_container_ownership() {
    run_scene(OnUnloadFault::Panic, None, false, "S5 on_unload panic");
}

#[test]
fn s5_on_unload_over_budget_keeps_planned_container_ownership() {
    run_scene(
        OnUnloadFault::Sleep(Duration::from_secs(3)),
        Some(1),
        true,
        "S5 on_unload over-budget",
    );
}
