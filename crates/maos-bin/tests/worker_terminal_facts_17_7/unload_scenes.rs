//! S5: planned unload remains owned when the Worker on_unload hook fails.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_bin::supervision::{OnUnloadFault, SupervisorSeams, WorkerSupervisor, WorkerTask};
use maos_kernel_core::iac::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::iac::FrameKind;
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

use super::{capture_file, capture_text, diagnostic_tags, SceneChild};

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

fn trace(dir: &Path, record: std::fmt::Arguments<'_>) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("scene.trace"))
        .expect("open S5 phase trace");
    file.write_all(format!("{record}\n").as_bytes())
        .expect("write S5 phase trace");
}

fn t3_manifest(dir: &Path) -> toml::Value {
    let manifest = dir.join("worker.toml");
    std::fs::write(&manifest,
        "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang\"]\noutput_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\nrecovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n[author]\nname = \"MAOS Project\"\n").expect("write T3 manifest");
    toml::from_str(&std::fs::read_to_string(manifest).unwrap()).expect("parse T3 manifest")
}

async fn exercise_unload(
    dir: &Path,
    fault: OnUnloadFault,
    cap: Option<u32>,
    expected_budget_row: bool,
) {
    let expected_home = dir.join("runtime-home");
    let home = |name| {
        if std::env::var_os(name).as_deref() == Some(expected_home.as_os_str()) {
            "private"
        } else {
            "unexpected"
        }
    };
    let workspace = match maos_bin::worker_workspace::resolve_workspaces_root() {
        Ok(path) if path == expected_home.join("workspaces") => "private",
        Ok(_) => "unexpected",
        Err(_) => "unresolved",
    };
    let fixture = match maos_bin::worker_spawn::resolve_cli_binary("worker-cli-fixture") {
        Ok(path) if Path::new(&path) == super::layout::profile_dir().join("worker-cli-fixture") => {
            "profile/worker-cli-fixture"
        }
        Ok(_) => "elsewhere",
        Err(_) => "unresolved",
    };
    trace(
        dir,
        format_args!(
        "phase=setup HOME={} MAOS_HOME={} XDG_DATA_HOME={} workspace={workspace} fixture={fixture}",
        home("HOME"), home("MAOS_HOME"), home("XDG_DATA_HOME")
    ),
    );
    let world = World::new(dir);
    let mint_dir = dir.to_owned();
    let watch_dir = dir.to_owned();
    let supervisor = world.supervisor(SupervisorSeams {
        on_unload_fault: Some(fault),
        hook_cap_override: cap,
        after_mint: Some(Arc::new(move || {
            trace(&mint_dir, format_args!("phase=after-mint"))
        })),
        after_watch: Some(Arc::new(move |pid| {
            trace(
                &watch_dir,
                format_args!("phase=runtime-client-observed pid={pid}"),
            );
        })),
        ..Default::default()
    });
    let manifest = t3_manifest(dir);
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
    let launched = Instant::now();
    let trace_dir = dir.to_owned();
    trace(dir, format_args!("phase=launch"));
    let run_task = tokio::task::spawn_blocking(move || {
        let result = maos_bin::worker_spawn::run_cli_wrapper_manifest(
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
        .map_err(|error| diagnostic_tags(&error.to_string()));
        let elapsed = launched.elapsed();
        let receipt = cid_from_lifecycle(&trace_dir.join("lifecycle.ndjson")).is_some();
        match &result {
            Ok(_) => trace(
                &trace_dir,
                format_args!(
                    "phase=run-return outcome=ok elapsed={elapsed:?} cid-receipt={receipt}"
                ),
            ),
            Err(tags) => trace(
                &trace_dir,
                format_args!(
                "phase=run-return outcome=error elapsed={elapsed:?} cid-receipt={receipt}; {tags}"
            ),
            ),
        }
        result
    });
    let bind_by = launched + Duration::from_secs(60);
    loop {
        if run_task.is_finished() {
            let outcome = match run_task.await {
                Ok(Ok(_)) => "unexpected-ok",
                Ok(Err(_)) => "error",
                Err(error) if error.is_panic() => "panic",
                Err(_) => "cancelled",
            };
            trace(
                dir,
                format_args!("phase=before-bind early-return={outcome}"),
            );
            panic!("real T3 Worker returned before the planned unload");
        }
        if supervisor.bindings().iter().any(|b| b.child_pid.is_some()) {
            break;
        }
        if Instant::now() >= bind_by {
            let receipt = cid_from_lifecycle(&dir.join("lifecycle.ndjson")).is_some();
            trace(
                dir,
                format_args!(
                    "phase=await-bind timeout elapsed={:?} cid-receipt={receipt}",
                    launched.elapsed()
                ),
            );
            panic!("real T3 Worker must bind before the planned unload");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let binding = supervisor
        .bindings()
        .into_iter()
        .find(|b| b.child_pid.is_some())
        .expect("bound Worker");
    let pid = binding.spirit_pid;
    let cid = cid_from_lifecycle(&dir.join("lifecycle.ndjson"));
    trace(
        dir,
        format_args!(
            "phase=bound elapsed={:?} cid-receipt={}",
            launched.elapsed(),
            cid.is_some()
        ),
    );
    assert!(
        cid.is_some(),
        "SandboxApplied lifecycle journal must identify the owned CID"
    );
    let cid = cid.unwrap();
    // The production root wrapper: unload, then the same post-unload registry
    // stop `main` installs, whatever the failing hook did (AC4.6).
    let unload_started = Instant::now();
    trace(dir, format_args!("phase=planned-unload"));
    let stop_after_unload = |unloaded| supervisor.stop_binding(unloaded);
    let report = maos_bin::supervision::unload_all_loaded(
        &world.scheduler,
        &world.halt_registry,
        Some(&stop_after_unload),
    )
    .await;
    let unload_returned = Instant::now();
    trace(
        dir,
        format_args!("phase=unload-return elapsed={:?}", unload_started.elapsed()),
    );
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
                .args([
                    "--kill-after=1s",
                    "5s",
                    &exists_runtime,
                    "container",
                    "exists",
                    &exists_cid,
                ])
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
                    let (capture, stdout) = capture_file();
                    let status = std::process::Command::new("timeout")
                        .args([
                            "--kill-after=1s",
                            "5s",
                            &running_runtime,
                            "inspect",
                            "--format",
                            "{{.State.Running}}",
                            &running_cid,
                        ])
                        .stdout(stdout)
                        .status()
                        .expect("inspect owned CID");
                    (status, capture_text(&capture))
                }),
            )
            .await
            .expect("CID running-state query bounded")
            .expect("inspect task");
            assert!(
                running.0.success(),
                "existing owned CID must be inspectable"
            );
            assert_eq!(
                running.1.trim(),
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
        Err(_) => {
            let client_state = binding.child_pid.and_then(super::proc_state);
            trace(
                dir,
                format_args!(
                    "phase=await-run-return timeout cid-receipt=true client-state={client_state:?}"
                ),
            );
            panic!("run_cli_wrapper_manifest did not return within 30s of planned unload");
        }
    };
    assert!(
        run_result.is_ok(),
        "real run_cli_wrapper_manifest should finalize"
    );

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
                .args([
                    "--kill-after=1s",
                    "8s",
                    &runtime,
                    "container",
                    "exists",
                    &exists_cid,
                ])
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
}

fn cid_from_lifecycle(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let text = capture_text(&file);
    for line in text.lines() {
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if entry["lifecycle_event"] == "SandboxApplied" {
            let bytes = entry["payload"].as_array()?;
            let payload = bytes
                .iter()
                .map(|byte| byte.as_u64().and_then(|n| u8::try_from(n).ok()))
                .collect::<Option<Vec<_>>>()?;
            let receipt: serde_json::Value = serde_json::from_slice(&payload).ok()?;
            return receipt["container_id"]
                .as_str()
                .filter(|cid| super::valid_cid(cid))
                .map(str::to_owned);
        }
    }
    None
}

/// Parent-side hard bound for the entire child, including a synchronous stall
/// that prevents an in-runtime timer from being polled.
const SCENE_HARD_BOUND: Duration = Duration::from_secs(240);
const SCENE_CHILD: &str = "MAOS_17_7_UNLOAD_SCENE_CHILD";

/// How long a failed scene waits for its blocking threads before abandoning
/// them. A `#[tokio::test]` runtime's `Drop` waits for every `spawn_blocking`
/// task with no bound, so a failure while `run_cli_wrapper_manifest` is still
/// blocked kept the test process alive until the CI job's own timeout (hosted
/// run 45447011202: the assertion failed at +30 s and the test was still
/// "running" 1000+ s later). Owning the runtime lets a failure return promptly.
const FAILED_SCENE_RUNTIME_REAP: Duration = Duration::from_secs(5);

fn run_scene_child(dir: &Path, fault: OnUnloadFault, cap: Option<u32>, budget_row: bool) {
    trace(dir, format_args!("phase=child-start"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("build the S5 scene runtime");
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(exercise_unload(dir, fault, cap, budget_row))
    }));
    runtime.shutdown_timeout(FAILED_SCENE_RUNTIME_REAP);
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

fn run_scene(
    name: &str,
    fault: OnUnloadFault,
    cap: Option<u32>,
    expected_budget_row: bool,
    scene_name: &str,
) {
    if let Some(selected) = std::env::var_os(SCENE_CHILD) {
        assert_eq!(
            selected, name,
            "isolated S5 child must execute the selected case"
        );
        let home = PathBuf::from(std::env::var_os("MAOS_HOME").expect("isolated S5 home"));
        let dir = home.parent().expect("S5 parent scratch");
        run_scene_child(dir, fault, cap, expected_budget_row);
        trace(dir, format_args!("phase=complete"));
        return;
    }
    if !super::scene(scene_name) {
        return;
    }

    // Re-exec only this existing test key. The private completion record
    // rejects a zero-test child; its libtest results never enter the CI log.
    let scratch = tempfile::tempdir().expect("isolated S5 scratch");
    let dir = scratch.path();
    let home = dir.join("runtime-home");
    std::fs::create_dir(&home).expect("create isolated runtime home");
    let (stderr, err) = capture_file();
    let (stdout, out) = capture_file();
    let started = Instant::now();
    let mut child = SceneChild::spawn_worker(
        SceneChild::worker_command(std::env::current_exe().expect("S5 test executable"))
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env(SCENE_CHILD, name)
            .env("HOME", super::t3_substrate::isolated_runtime_home(&home))
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", &home)
            .env_remove(maos_bin::worker_workspace::WORKSPACES_ROOT_ENV)
            .env("PATH", super::layout::path_with_profile())
            .stdout(out)
            .stderr(err),
    )
    .unwrap_or_else(|error| panic!("{scene_name} phase=spawn failed ({:?})", error.kind()));
    let status = child
        .wait_until(started + SCENE_HARD_BOUND)
        .expect("poll S5 child");
    let phases = std::fs::File::open(dir.join("scene.trace"))
        .map(|file| capture_text(&file))
        .unwrap_or_default();
    if status.is_some_and(|status| status.success())
        && phases.lines().any(|line| line == "phase=complete")
    {
        child.reap().expect("reap isolated S5 child");
        return;
    }
    let outcome = match status {
        None => "hard-timeout",
        Some(status) if status.success() => "missing-completion",
        Some(_) => "failed",
    };
    let elapsed = started.elapsed();
    child.cid = cid_from_lifecycle(&dir.join("lifecycle.ndjson")).or_else(|| {
        capture_text(&stdout)
            .lines()
            .find_map(super::cid_receipt)
            .map(|receipt| receipt.cid)
    });
    let cid_received = child.cid.is_some();
    let cleanup = child.cleanup_failure();
    let stderr = capture_text(&stderr);
    let assertion_site = stderr.lines().find_map(|line| {
        let (_, site) = line.split_once("panicked at ")?;
        let (_, location) = site.split_once("worker_terminal_facts_17_7/unload_scenes.rs:")?;
        let (line, column) = location.trim_end_matches(':').split_once(':')?;
        Some((line.parse::<u32>().ok()?, column.parse::<u32>().ok()?))
    });
    panic!(
        "{scene_name} phase=child-result outcome={outcome} status={status:?}; \
         elapsed={elapsed:?}; cid-receipt={cid_received}; \
         cleanup={cleanup:?}; S5 assertion site={assertion_site:?}\n\
         {phases}stderr: {}",
        diagnostic_tags(&stderr)
    );
}

#[test]
fn s5_on_unload_panic_before_stop_keeps_planned_container_ownership() {
    run_scene(
        "unload_scenes::s5_on_unload_panic_before_stop_keeps_planned_container_ownership",
        OnUnloadFault::Panic,
        None,
        false,
        "S5 on_unload panic",
    );
}

#[test]
fn s5_on_unload_over_budget_keeps_planned_container_ownership() {
    run_scene(
        "unload_scenes::s5_on_unload_over_budget_keeps_planned_container_ownership",
        OnUnloadFault::Sleep(Duration::from_secs(3)),
        Some(1),
        true,
        "S5 on_unload over-budget",
    );
}
