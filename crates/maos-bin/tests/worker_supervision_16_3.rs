#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 16-3 — **the wiring, every path** (AC3), plus AC2's exclusive-cursor
//! vector and AC5's binary leg.
//!
//! What each vector here refuses:
//!
//! * A Worker that dies and reaches nothing — at `af96c907` a SIGKILLed
//!   standalone Worker's whole Transparency Log was three tokenless pid-0 rows
//!   (kind 21, kind 7 `cli.subprocess.exit signaled(sig=9)`, kind 4 verdict).
//!   No SCB, no `task.orphaned`, no disposition, no `lifecycle.crash`.
//! * A bound Worker leaked by an early return. The RAII guard is the mechanism;
//!   the injected-`?` vector is its proof, and its `signals_sent == 1`
//!   assertion is the ONLY observable that reds when the guard's signal is
//!   removed (the bridge's own `Drop` kills and reaps the child, so every other
//!   observable stays green — measured 200× per mutation by the story's
//!   validation round 9).
//! * A Worker that printed once and then hung, never stalling. That is what an
//!   inclusive `since_ns` cursor buys, and the cursor vector below is where it
//!   is visible: it is the one place where NOTHING ELSE prints.
//! * `maos audit query --spirit worker` exiting 1 on the Worker's own output
//!   rows once they carry a real pid.
//!
//! Linux-gated: the pidfd exit observer is the subject.

#[path = "support/t0_worker.rs"]
mod t0_worker;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use maos_bin::supervision::{
    BindingAction, BindingPhase, PhaseRefusal, WorkerSupervision, WorkerSupervisor, WorkerTask,
};
use maos_domain::supervision::{CrashCause, FaultCause};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;

const NONCE: u64 = 0x16_03;

static WORKER_FIXTURE_BUILT: LazyLock<()> = LazyLock::new(|| {
    let output = std::process::Command::new("cargo")
        .args(["build", "-q", "-p", "worker", "--bin", "worker-cli-fixture"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("run cargo build for worker fixture");
    assert!(
        output.status.success(),
        "build worker-cli-fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
});

// ────────────────────────────────────────────────────────────────────────────
// (1) The state machine — a table over every (phase × input) cell.
//
// D-16-3-Q (4)/(5): every transition is a pure function returning the next
// phase and the `BindingAction` the supervisor executes AFTER the lock is
// released. The (t) interleavings become this table: if a cell is wrong, one
// of the live vectors below is only accidentally right.
// ────────────────────────────────────────────────────────────────────────────

fn signaled(signal: i32) -> CrashCause {
    CrashCause::Fault(FaultCause::SignaledByKernel {
        signal,
        stderr_tail: None,
    })
}

#[test]
fn binding_phase_transitions_cover_every_cell() {
    use BindingAction as A;
    use BindingPhase as P;

    // on_pidfd_stored: the stop that landed during the mint is signalled HERE.
    assert_eq!(P::Running.on_pidfd_stored(), Ok((P::Running, A::Nothing)));
    assert_eq!(
        P::PlannedStop.on_pidfd_stored(),
        Ok((P::PlannedStop, A::Signal))
    );
    // Unreachable by construction — the pidfd is stored before the observer
    // starts and before the pump — so the refusal is asserted, not a panic.
    assert_eq!(
        P::CrashHandled.on_pidfd_stored(),
        Err(PhaseRefusal::PidfdStoredAfterExit(P::CrashHandled))
    );
    assert_eq!(
        P::ExitedClean.on_pidfd_stored(),
        Err(PhaseRefusal::PidfdStoredAfterExit(P::ExitedClean))
    );

    // begin_stop: only from Running, and only signals when a child is stored.
    assert_eq!(
        P::Running.begin_stop(true),
        (P::PlannedStop, A::Signal),
        "a stop with a pidfd must reach the process"
    );
    assert_eq!(
        P::Running.begin_stop(false),
        (P::PlannedStop, A::Nothing),
        "no pidfd ⇒ nothing to signal; the stated fallback cost"
    );
    for phase in [P::CrashHandled, P::PlannedStop, P::ExitedClean] {
        assert_eq!(phase.begin_stop(true), (phase, A::Nothing));
        assert_eq!(phase.begin_stop(false), (phase, A::Nothing));
    }

    // on_exit (the observer's single publish).
    let crash = signaled(9);
    assert_eq!(
        P::Running.on_exit(&crash),
        (P::CrashHandled, A::SpawnCrashHandler(crash.clone())),
        "the observer spawns the handler in-lock and stores its handle"
    );
    assert_eq!(
        P::Running.on_exit(&CrashCause::Voluntary),
        (P::Running, A::Nothing),
        "a clean exit is `finish`'s job: only the call stack knows the verdict"
    );
    for phase in [P::CrashHandled, P::PlannedStop, P::ExitedClean] {
        assert_eq!(phase.on_exit(&crash), (phase, A::Nothing));
    }

    // finish.
    assert_eq!(
        P::CrashHandled.finish(None),
        (P::CrashHandled, A::JoinHandlerThenMaybeDisposition)
    );
    assert_eq!(
        P::CrashHandled.finish(Some(&crash)),
        (P::CrashHandled, A::JoinHandlerThenMaybeDisposition),
        "the handler is already running: never a second handle_crash"
    );
    assert_eq!(
        P::Running.finish(Some(&crash)),
        (
            P::CrashHandled,
            A::RunCrashHandlerThenMaybeDisposition(crash.clone())
        ),
        "the observer lost the reap race (ECHILD) — this IS the crash path"
    );
    assert_eq!(
        P::Running.finish(None),
        (P::ExitedClean, A::UnloadClean),
        "exit 0 takes the record out of the ledger WITHOUT a disposition"
    );
    assert_eq!(
        P::PlannedStop.finish(None),
        (P::PlannedStop, A::DispositionAndUnload),
        "a stopped Worker's task is as dead to its originator as a crashed one's"
    );
    assert_eq!(
        P::PlannedStop.finish(Some(&crash)),
        (P::PlannedStop, A::DispositionAndUnload),
        "a signal the supervisor itself sent is never a crash"
    );
    assert_eq!(P::ExitedClean.finish(None), (P::ExitedClean, A::Nothing));

    // abandon (the guard's Drop).
    assert_eq!(
        P::Running.abandon(),
        (P::PlannedStop, A::SignalThenDispositionAndUnload),
        "NEVER PlannedStop alone: the unload's own on_unload yields Nothing, so \
         a child spawned after bind would never be signalled"
    );
    assert_eq!(
        P::CrashHandled.abandon(),
        (
            P::CrashHandled,
            A::JoinHandlerThenMaybeDispositionThenUnload
        ),
        "handle_crash already drained and dispositioned the ledger; a plain \
         disposition here would write a second task.nacked"
    );
    assert_eq!(
        P::PlannedStop.abandon(),
        (P::PlannedStop, A::DispositionAndUnload)
    );
    assert_eq!(P::ExitedClean.abandon(), (P::ExitedClean, A::Nothing));
}

#[test]
fn worker_task_ids_survive_redaction_and_name_their_originator() {
    // Trap 9: the Transparency Log's scrubber replaces any `sk-` substring and
    // any run of ≥32 contiguous hex chars. A redacted task id cannot be joined
    // to its disposition row, so the ids must dodge BOTH rules.
    let (task_id, originator) = WorkerTask::Standalone.record_ids("worker");
    assert_eq!(
        (task_id.as_str(), originator.as_str()),
        ("run:worker", "operator")
    );

    let (task_id, originator) = WorkerTask::TopologyEntry { index: 2 }.record_ids("worker-3");
    assert_eq!(
        (task_id.as_str(), originator.as_str()),
        ("topology:2", "topology")
    );

    let (task_id, _) = WorkerTask::Delegation {
        frame_id: [0xAB; 16],
    }
    .record_ids("worker");
    assert_eq!(
        task_id, "delegation:abababab:abababab:abababab:abababab",
        "four 8-hex groups, so no run reaches the 32-contiguous-hex rule"
    );
    for (id, _) in [
        WorkerTask::Standalone.record_ids("worker"),
        WorkerTask::TopologyEntry { index: 0 }.record_ids("worker"),
        WorkerTask::Delegation {
            frame_id: [0xFF; 16],
        }
        .record_ids("worker"),
    ] {
        assert!(
            !id.contains("sk-"),
            "{id} would be redacted by the sk- rule"
        );
        let longest_hex_run = id
            .split(|c: char| !c.is_ascii_hexdigit())
            .map(str::len)
            .max()
            .unwrap_or(0);
        assert!(
            longest_hex_run < 32,
            "{id} has a {longest_hex_run}-char hex run; the scrubber redacts ≥32"
        );
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The in-process kernel world (the shape `crash_detector_in_process_panic.rs`
// uses). Duplicated per test file by this repo's convention — integration tests
// are separate crates and cannot share a helper without inventing one.
// ────────────────────────────────────────────────────────────────────────────

struct KernelWorld {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    halt_registry: Arc<maos_kernel_core::halt::HaltRegistry>,
    crash_detector: Arc<maos_kernel_core::supervision::CrashDetector>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
    telemetry: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
    notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
}

impl KernelWorld {
    fn new(dir: &Path) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
        let policy = Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new());
        let capability = Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x16; 32]),
                NONCE,
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
        let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
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
        // Trap 1 — before any clone of the scheduler Arc.
        Arc::get_mut(&mut scheduler)
            .expect("scheduler Arc strong_count == 1")
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
            telemetry,
            notification_dispatcher: Arc::new(
                maos_director_surface::notification::NotificationDispatcher::default(),
            ),
        }
    }

    fn supervisor(&self, seams: maos_bin::supervision::SupervisorSeams) -> Arc<WorkerSupervisor> {
        Arc::new(
            WorkerSupervisor::new(
                Arc::clone(&self.scheduler),
                Arc::clone(&self.crash_detector),
                Arc::clone(&self.tl),
                Arc::clone(&self.journal),
                Arc::clone(&self.halt_registry),
                Arc::clone(&self.iac),
                tokio::runtime::Handle::current(),
                NONCE,
                tokio_util::sync::CancellationToken::new(),
            )
            .with_seams(seams),
        )
    }

    fn rows(&self, kind: FrameKind, spirit_pid: Option<u32>) -> Vec<TlRow> {
        self.tl
            .query_frames(FrameFilter {
                kind: Some(kind),
                spirit_pid,
                ..Default::default()
            })
            .expect("query transparency log")
            .into_iter()
            .map(|row| TlRow {
                spirit_pid: row.spirit_pid,
                intent: row.intent.clone(),
                payload: serde_json::from_slice(&row.payload_redacted).unwrap_or(
                    serde_json::Value::String(
                        String::from_utf8_lossy(&row.payload_redacted).to_string(),
                    ),
                ),
                has_token: row.capability_token.is_some(),
            })
            .collect()
    }

    fn intents(&self, kind: FrameKind, spirit_pid: Option<u32>) -> BTreeSet<String> {
        self.rows(kind, spirit_pid)
            .into_iter()
            .map(|row| row.intent)
            .collect()
    }
}

struct TlRow {
    spirit_pid: u32,
    intent: String,
    payload: serde_json::Value,
    has_token: bool,
}

fn scratch_dir(tag: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("maos-supervision-16-3-{tag}-{nonce}"));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The only known-admitting `[cli_wrapper]` shape: the built-in host grant
/// names `worker-cli-fixture` exactly, so no machine-local `MAOS_HOST_GRANTS`
/// file can change the outcome.
fn write_worker_manifest(dir: &Path, mode: Option<&str>, threshold_ms: Option<u32>) -> PathBuf {
    write_worker_manifest_tier(dir, mode, threshold_ms, "T0")
}

fn write_worker_manifest_t3(dir: &Path, mode: Option<&str>, threshold_ms: Option<u32>) -> PathBuf {
    write_worker_manifest_tier(dir, mode, threshold_ms, "T3")
}

fn write_worker_manifest_tier(
    dir: &Path,
    mode: Option<&str>,
    threshold_ms: Option<u32>,
    tier: &str,
) -> PathBuf {
    let name = format!(
        "worker-{}-{}.toml",
        mode.unwrap_or("default"),
        threshold_ms.unwrap_or(0)
    );
    let path = dir.join(name);
    let argv = match mode {
        Some(mode) => format!("[\"--maos-worker\", \"--maos-fixture-mode={mode}\"]"),
        None => "[\"--maos-worker\"]".to_string(),
    };
    let supervision = match threshold_ms {
        Some(ms) => format!("\n[supervision]\nprogress_threshold_ms = {ms}\n"),
        None => String::new(),
    };
    std::fs::write(
        &path,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = {argv}\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"{tier}\"\n\n\
             [author]\nname = \"MAOS Project\"\n{supervision}"
        ),
    )
    .expect("write worker manifest");
    path
}

/// Drive a Worker through the real T0 bridge primitive and kernel supervisor.
async fn run_worker(
    world: &KernelWorld,
    supervisor: &Arc<WorkerSupervisor>,
    manifest: PathBuf,
    task: WorkerTask,
) -> Result<String, String> {
    let manifest_root: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).expect("read manifest"))
            .expect("parse manifest");
    let tl = Arc::clone(&world.tl);
    let capability = Arc::clone(&world.capability);
    let policy = Arc::clone(&world.policy);
    let supervisor = Arc::clone(supervisor);
    tokio::task::spawn_blocking(move || {
        t0_worker::run_t0_worker(
            &manifest_root,
            tl,
            capability,
            policy,
            supervisor.as_ref(),
            task,
        )
        .map(|completion| completion.label().to_string())
    })
    .await
    .expect("Worker blocking thread joined")
}

/// Block until `predicate` holds or `budget` elapses. Returns whether it held.
async fn wait_for(budget: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + budget;
    loop {
        if predicate() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ────────────────────────────────────────────────────────────────────────────
// (3) The crash scene at a REAL pid — D-16-3-B, D, E, F.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_sigkilled_worker_reaches_handle_crash_at_its_own_pid() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("crash-scene");
    let world = KernelWorld::new(&dir);
    let bound_tokens = Arc::new(Mutex::new(Vec::new()));
    let observed_tokens = Arc::clone(&bound_tokens);
    let observed_scheduler = Arc::clone(&world.scheduler);
    let supervisor = world.supervisor(maos_bin::supervision::SupervisorSeams {
        after_mint: Some(Arc::new(move || {
            let scbs = observed_scheduler.scbs();
            let scbs = scbs.read().unwrap();
            let mut observed = observed_tokens.lock().unwrap();
            for scb in scbs.values() {
                let ledger = scb.task_assignments_in_flight.lock().unwrap();
                observed.extend(ledger.iter().filter_map(|record| record.capability_token));
            }
        })),
        ..Default::default()
    });
    let manifest = write_worker_manifest(&dir, Some("sigkill-self"), None);

    let outcome = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone).await;
    assert!(
        outcome.is_ok(),
        "the run itself must complete and report a verdict, got {outcome:?}"
    );

    let views = supervisor.bindings();
    assert_eq!(views.len(), 1, "one Worker, one binding");
    let pid = views[0].spirit_pid;
    assert_ne!(pid, 0, "every Worker row must move off pid 0");
    assert_eq!(views[0].spirit_id, "worker", "the first Worker of a root");
    assert_eq!(
        supervisor.strategy_name(),
        "pidfd",
        "this is the observer's own vector; a fallback root proves nothing here"
    );

    // The handler is spawned by the observer, so the rows land asynchronously.
    let crashed = wait_for(Duration::from_secs(5), || {
        world
            .intents(FrameKind::CapabilityInvocation, Some(pid))
            .contains("lifecycle.crash")
    })
    .await;
    assert!(crashed, "no `lifecycle.crash` row at the Worker's pid");

    let lifecycle = world.intents(FrameKind::CapabilityInvocation, Some(pid));
    for intent in ["lifecycle.load", "lifecycle.start", "lifecycle.crash"] {
        assert!(lifecycle.contains(intent), "missing {intent} at pid {pid}");
    }
    assert!(
        !world
            .rows(FrameKind::CliSubprocessOutput, Some(pid))
            .is_empty(),
        "the fixture's kind-21 output rows must be at the Worker's pid"
    );

    // The orphan event records the tokens bound while this task was in flight.
    // Compare the real mint observation, not a fabricated id or an empty default.
    let orphaned: Vec<TlRow> = world
        .rows(FrameKind::TaskComplete, Some(pid))
        .into_iter()
        .filter(|row| row.intent == "task.orphaned")
        .collect();
    assert_eq!(orphaned.len(), 1, "exactly one task.orphaned for one task");
    let payload = &orphaned[0].payload;
    assert_eq!(payload["cause"], "fault.signaled");
    assert_eq!(payload["exit_signal"], 9);
    assert_eq!(payload["task_id"], "run:worker");
    assert_eq!(payload["originator_spirit_id"], "operator");
    assert!(
        !orphaned[0].has_token,
        "task.orphaned is an event, not an invocation charged to one token"
    );
    {
        let bound_tokens = bound_tokens.lock().unwrap();
        assert_eq!(
            bound_tokens.len(),
            1,
            "the task binds its mediated CLI token"
        );
        assert_eq!(
            payload["in_flight_tokens"],
            serde_json::to_value(&*bound_tokens).unwrap()
        );
    }
    assert!(
        !world.capability.has_active_tokens_for_pid(pid),
        "crash handling must revoke the Worker's actual bound token"
    );

    // The FR50 disposition lands at pid 0, sender = originator, and is joined
    // to the Worker by payload `task_id` — never by pid.
    let dispositioned = wait_for(Duration::from_secs(5), || {
        world
            .rows(FrameKind::TaskComplete, None)
            .into_iter()
            .any(|row| row.intent == "task.nacked" && row.payload["task_id"] == "run:worker")
    })
    .await;
    assert!(
        dispositioned,
        "default [on_crash] is nack, so the orphaned task must be nacked"
    );

    // The SCB is gone: `handle_crash` removed it.
    assert!(
        supervisor
            .bindings()
            .iter()
            .all(|view| view.phase == BindingPhase::CrashHandled),
        "the binding must end in CrashHandled"
    );
    assert!(
        world.scheduler.resolve_pid("worker").is_none(),
        "the Worker's SCB must not survive its crash"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_escalating_manifest_escalates_instead_of_nacking() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("escalate");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    // `[on_crash]` is parsed on the run path and reaches the SCB bundle — at
    // `af96c907` neither section was parsed there at all, so FR50 could only
    // ever nack.
    let manifest = dir.join("escalate.toml");
    let base = std::fs::read_to_string(write_worker_manifest(&dir, Some("exit-nonzero"), None))
        .expect("read base manifest");
    std::fs::write(
        &manifest,
        format!("{base}\n[on_crash]\naction = \"escalate-to-operator\"\n"),
    )
    .expect("write escalating manifest");

    let _ = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone).await;
    let pid = supervisor.bindings()[0].spirit_pid;

    let escalated = wait_for(Duration::from_secs(5), || {
        world
            .rows(FrameKind::TaskComplete, None)
            .into_iter()
            .any(|row| row.intent == "task.escalated" && row.payload["task_id"] == "run:worker")
    })
    .await;
    assert!(
        escalated,
        "the action is captured AT BIND; falling back to the default Nack \
         after the SCB is gone would write task.nacked for an escalating Worker"
    );
    let orphaned: Vec<TlRow> = world
        .rows(FrameKind::TaskComplete, Some(pid))
        .into_iter()
        .filter(|row| row.intent == "task.orphaned")
        .collect();
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].payload["cause"], "fault.non-zero-exit");
    assert_eq!(
        orphaned[0].payload["exit_code"], 3,
        "the fixture's exit-nonzero mode exits 3"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_worker_that_exits_zero_is_unloaded_and_never_orphaned() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("clean-exit");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    // No mode flag: the fixture prints its three canned lines and exits 0.
    let manifest = write_worker_manifest(&dir, None, None);

    let label = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone)
        .await
        .expect("a clean fixture run completes");
    assert_eq!(label, "completed");

    let pid = supervisor.bindings()[0].spirit_pid;
    let lifecycle = world.intents(FrameKind::CapabilityInvocation, Some(pid));
    assert!(
        lifecycle.contains("lifecycle.unload"),
        "a clean exit is still unloaded: an SCB left Running would stall later"
    );
    assert!(
        !lifecycle.contains("lifecycle.crash"),
        "ADR-022: exit 0 is NOT a crash"
    );
    assert!(
        world
            .rows(FrameKind::TaskComplete, Some(pid))
            .into_iter()
            .all(|row| row.intent != "task.orphaned"),
        "a clean exit orphans nothing"
    );
    assert!(
        world
            .rows(FrameKind::TaskComplete, None)
            .into_iter()
            .all(|row| row.intent != "task.nacked" && row.intent != "task.escalated"),
        "a clean exit dispositions nothing — the verdict row IS the record"
    );
    // NFR-Rel-11's planned half: the unload produced a PlannedUnload receipt.
    // `terminate_spirit` writes its rows as `FrameKind::EpistemicHalt` with the
    // termination kind as the intent, so the intent is what makes this a
    // PLANNED unload receipt rather than any halt row that happens to exist.
    assert!(
        world
            .intents(FrameKind::EpistemicHalt, Some(pid))
            .contains("planned_unload"),
        "the planned unload must leave a receipt at the Worker's pid; saw {:?}",
        world.intents(FrameKind::EpistemicHalt, Some(pid))
    );
    assert_eq!(
        supervisor.bindings()[0].phase,
        BindingPhase::ExitedClean,
        "the clean-exit phase is terminal"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (4) The RAII guard — D-16-3-Q (6). The `signals_sent` counter is the point.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_injected_error_after_watch_leaves_no_bound_worker_behind() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("injected-return");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(maos_bin::supervision::SupervisorSeams {
        error_after_watch: Some("injected return after watch".to_string()),
        ..Default::default()
    });
    // `hang`, deliberately: `hang-with-grandchild` would block the bridge's
    // `Drop` forever on its reader-thread join (the grandchild inherits the
    // pipes), and `sigkill-self`/`exit-nonzero` can reach `CrashHandled`
    // before the injected `?` fires.
    let manifest = write_worker_manifest(&dir, Some("hang"), None);

    let error = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone)
        .await
        .expect_err("the injected return must propagate");
    assert!(error.contains("injected return after watch"), "got {error}");

    let view = supervisor
        .bindings()
        .into_iter()
        .next()
        .expect("one binding");
    let pid = view.spirit_pid;
    assert_eq!(
        view.phase,
        BindingPhase::PlannedStop,
        "abandon(Running) moves to PlannedStop"
    );
    // THE observable. With the signal removed from
    // `SignalThenDispositionAndUnload`, or with `LiveWorker`'s fields
    // reordered, every OTHER assertion in this vector still passes — the
    // bridge's own `Drop` kills and reaps the child. Only this counter reds.
    assert_eq!(
        supervisor.counters().signals_sent(),
        1,
        "the guard must signal the LIVE child before the bridge drops"
    );
    let lifecycle = world.intents(FrameKind::CapabilityInvocation, Some(pid));
    assert!(
        lifecycle.contains("lifecycle.unload"),
        "the guard's Drop unloads the SCB with no explicit abandon call"
    );
    assert!(
        !lifecycle.contains("lifecycle.crash"),
        "a signal the supervisor itself sent is never a crash"
    );
    assert!(
        world
            .intents(FrameKind::EpistemicHalt, Some(pid))
            .contains("planned_unload"),
        "a PlannedUnload receipt, so a pending halt would leave a receipt"
    );
    let stop_dispositioned = world
        .rows(FrameKind::TaskComplete, None)
        .into_iter()
        .any(|row| row.intent == "task.nacked" && row.payload["task_id"] == "run:worker");
    assert!(
        stop_dispositioned,
        "a stopped Worker's task is as dead to its originator as a crashed one's"
    );
    assert!(
        world.scheduler.resolve_pid("worker").is_none(),
        "the SCB must be gone from scbs()"
    );
    assert!(
        !child_is_alive(view.child_pid.expect("a child pid was recorded")),
        "the child must be gone"
    );
}

/// The in-process harness OWNS the Worker, so the pidfd's own `waitid` is the
/// oracle. (In a BINARY test the Worker is a child of `maos`, `waitid` returns
/// `ECHILD` at once and `kill(pid, 0)` succeeds on a zombie — neither is
/// evidence there; see `root_shutdown_unload_16_3.rs`.)
fn child_is_alive(child_pid: u32) -> bool {
    let status = std::fs::read_to_string(format!("/proc/{child_pid}/stat"));
    match status {
        Ok(stat) => {
            // field 3 is the state char; `Z` is a reaped-but-not-waited zombie.
            let state = stat
                .rsplit(')')
                .next()
                .and_then(|rest| rest.split_whitespace().next())
                .unwrap_or("Z");
            state != "Z"
        }
        Err(_) => false,
    }
}

struct ControlledRuntime {
    executable: PathBuf,
    invocations: PathBuf,
    environment: PathBuf,
    target_pid_file: PathBuf,
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn controlled_runtime(dir: &Path, cid: &str, kill_target: bool) -> ControlledRuntime {
    let executable = dir.join("controlled-runtime");
    let invocations = dir.join("runtime-invocations");
    let environment = dir.join("runtime-environment");
    let target_pid_file = dir.join("payload.pid");
    let script = format!(
        "#!/bin/sh\n\
         printf '%s %s\\n' \"$1\" \"$2\" >> {}\n\
         unset PWD SHLVL _\n\
         /usr/bin/env -u PWD -u SHLVL -u _ > {}\n\
         case \"$1\" in\n\
         kill) [ \"$2\" = {} ] || exit 64; {} ;;\n\
         inspect) st={}; [ -f \"$0.killed\" ] && st=exited; printf '{{\"Status\":\"%s\",\"ExitCode\":0,\"OOMKilled\":false,\"Error\":\"\",\"StartedAt\":\"0001-01-01T00:00:00Z\",\"FinishedAt\":\"0001-01-01T00:00:00Z\"}}\\n' \"$st\" ;;\n\
         rm) exit {} ;;\n\
         container) [ \"$2\" = exists ] || exit 64; exit {} ;;\n\
         *) exit 64 ;;\n\
         esac\n",
        shell_quote(&invocations.display().to_string()),
        shell_quote(&environment.display().to_string()),
        shell_quote(cid),
        if kill_target {
            format!(
                "read pid < {}; kill -KILL \"$pid\" && : > \"$0.killed\"",
                shell_quote(&target_pid_file.display().to_string())
            )
        } else {
            "exit 23".to_string()
        },
        // Terminal only after the CID kill actually happened.
        "running",
        if kill_target { "0" } else { "23" },
        // `container exists`: 1 = absent after a good removal, 0 = still there.
        if kill_target { "1" } else { "0" },
    );
    std::fs::write(&executable, script).expect("write controlled runtime executable");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
        .expect("make controlled runtime executable");
    ControlledRuntime {
        executable,
        invocations,
        environment,
        target_pid_file,
    }
}

fn adopt_stub_container(
    dir: &Path,
    runtime: &ControlledRuntime,
    cid: &str,
) -> maos_kernel_core::security::sandbox::t3::child::SandboxedContainerChild {
    use maos_domain::sandbox::ContainerRuntimeKind;
    use maos_kernel_core::security::sandbox::t3::child::SandboxedContainerChild;
    use maos_kernel_core::security::sandbox::t3::runtime_detect::ContainerRuntime;

    let state_dir = dir.join(format!("state-{cid}"));
    std::fs::create_dir(&state_dir).expect("create adopted container state dir");
    SandboxedContainerChild::adopt(
        ContainerRuntime {
            kind: ContainerRuntimeKind::Podman,
            path: runtime.executable.clone(),
            version: "stub".to_string(),
        },
        cid.to_string(),
        &state_dir,
    )
}

fn spawn_sleep() -> std::process::Child {
    std::process::Command::new("/bin/sleep")
        .arg("60")
        .spawn()
        .expect("start controlled process")
}

fn wait_child_exit(child: &mut std::process::Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => return false,
        }
    }
}

fn stop_child(child: &mut std::process::Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn assert_one_cid_kill(runtime: &ControlledRuntime, cid: &str) {
    let calls = std::fs::read_to_string(&runtime.invocations).expect("read runtime invocations");
    let kills: Vec<_> = calls
        .lines()
        .filter(|line| line.starts_with("kill "))
        .collect();
    assert_eq!(kills.len(), 1, "the owned CID is killed once");
    assert_eq!(kills[0], format!("kill {cid}"), "kill uses the owned CID");
    let allowed = [
        "PATH",
        "HOME",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ];
    let environment = std::fs::read_to_string(&runtime.environment).expect("read kill environment");
    for entry in environment.lines() {
        let key = entry.split('=').next().unwrap_or_default();
        assert!(
            allowed.contains(&key),
            "runtime kill inherited disallowed environment variable {key}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stopping_a_container_kills_its_cid_and_runtime_client() {
    let dir = scratch_dir("container-stop");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    assert_eq!(supervisor.strategy_name(), "pidfd");
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let cid = "a".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, true);
    let mut container_process = spawn_sleep();
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let mut runtime_client = spawn_sleep();
    supervisor.watch(&core, runtime_client.id());
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);
    core.attach_container(owned_container.terminal().clone());
    supervisor.stop_workers();
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    let container_stopped = wait_child_exit(&mut container_process);
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);
    tokio::task::block_in_place(|| drop(binding));

    assert!(
        runtime_client_stopped,
        "pidfd stop reaches the runtime client"
    );
    assert!(container_stopped, "CID kill reaches the container payload");
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(
        supervisor.counters().signals_sent(),
        1,
        "a successful CID kill and pidfd signal count as one stop"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn direct_worker_unload_stops_its_cid_container() {
    let dir = scratch_dir("container-unload");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let pid = binding.spirit_pid();
    let cid = "b".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, true);
    let mut container_process = spawn_sleep();
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let mut runtime_client = spawn_sleep();
    supervisor.watch(&core, runtime_client.id());
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);
    core.attach_container(owned_container.terminal().clone());

    world
        .scheduler
        .unload(pid)
        .await
        .expect("unload worker SCB");
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    let container_stopped = wait_child_exit(&mut container_process);
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);
    tokio::task::block_in_place(|| drop(binding));

    assert!(
        runtime_client_stopped,
        "unload reaches the runtime-client pidfd"
    );
    assert!(
        container_stopped,
        "on_unload reaches the CID-owned container"
    );
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(supervisor.counters().signals_sent(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn abandoning_a_binding_stops_its_cid_container() {
    let dir = scratch_dir("container-abandon");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let cid = "c".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, true);
    let mut container_process = spawn_sleep();
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let mut runtime_client = spawn_sleep();
    supervisor.watch(&core, runtime_client.id());
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);
    core.attach_container(owned_container.terminal().clone());

    tokio::task::block_in_place(|| drop(binding));
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    let container_stopped = wait_child_exit(&mut container_process);
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);

    assert!(
        runtime_client_stopped,
        "abandon signals the runtime-client pidfd"
    );
    assert!(
        container_stopped,
        "abandon stops the CID-owned container before bridge cleanup"
    );
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(supervisor.counters().signals_sent(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stop_before_container_attachment_still_kills_late_cid() {
    let dir = scratch_dir("late-container-attach");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let mut runtime_client = spawn_sleep();
    supervisor.watch(&core, runtime_client.id());
    let mut container_process = spawn_sleep();
    let cid = "d".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, true);
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);

    core.stop_session();
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    core.attach_container(owned_container.terminal().clone());
    let container_stopped = wait_child_exit(&mut container_process);
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);
    tokio::task::block_in_place(|| drop(binding));

    assert!(runtime_client_stopped, "the pre-attachment stop uses pidfd");
    assert!(
        container_stopped,
        "late attachment immediately stops the CID-owned container"
    );
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(supervisor.counters().signals_sent(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn container_attachment_before_pidfd_stops_cid_then_late_pidfd() {
    let dir = scratch_dir("container-before-watch");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let cid = "e".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, true);
    let mut container_process = spawn_sleep();
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let mut runtime_client = spawn_sleep();
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);
    core.attach_container(owned_container.terminal().clone());
    core.stop_session();
    let container_stopped = wait_child_exit(&mut container_process);
    supervisor.watch(&core, runtime_client.id());
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);
    tokio::task::block_in_place(|| drop(binding));

    assert!(
        container_stopped,
        "stop reaches the CID before a pidfd exists"
    );
    assert!(
        runtime_client_stopped,
        "late pidfd storage signals the runtime client"
    );
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(supervisor.counters().signals_sent(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_cid_kill_is_not_counted_as_a_successful_container_stop() {
    let dir = scratch_dir("container-stop-refused");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let binding = tokio::task::block_in_place(|| {
        supervisor
            .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
            .expect("bind worker SCB")
    });
    let core = Arc::clone(binding.core());
    let cid = "f".repeat(64);
    let runtime = controlled_runtime(&dir, &cid, false);
    let mut container_process = spawn_sleep();
    std::fs::write(&runtime.target_pid_file, container_process.id().to_string())
        .expect("write CID-owned payload process PID");
    let mut runtime_client = spawn_sleep();
    supervisor.watch(&core, runtime_client.id());
    let owned_container = adopt_stub_container(&dir, &runtime, &cid);
    core.attach_container(owned_container.terminal().clone());

    core.stop_session();
    let runtime_client_stopped = wait_child_exit(&mut runtime_client);
    let cid_container_survived = container_process
        .try_wait()
        .expect("poll container process")
        .is_none();
    let successful_stops = supervisor.counters().signals_sent();
    stop_child(&mut runtime_client);
    stop_child(&mut container_process);
    tokio::task::block_in_place(|| drop(binding));

    assert!(
        runtime_client_stopped,
        "pidfd still stops the runtime client"
    );
    assert!(
        cid_container_survived,
        "a non-zero runtime status does not pretend the CID-owned container stopped"
    );
    assert_one_cid_kill(&runtime, &cid);
    assert_eq!(
        successful_stops, 0,
        "a failed CID kill must not count as a successful container stop"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (5) The stop latch and the stop-during-mint window — D-16-3-D.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stop_during_the_mint_never_spawns_the_child() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("stop-in-mint");
    let world = KernelWorld::new(&dir);
    let latch: Arc<Mutex<Option<Arc<WorkerSupervisor>>>> = Arc::new(Mutex::new(None));
    let latch_for_hook = Arc::clone(&latch);
    let supervisor = world.supervisor(maos_bin::supervision::SupervisorSeams {
        after_mint: Some(Arc::new(move || {
            if let Some(supervisor) = latch_for_hook.lock().expect("latch").as_ref() {
                supervisor.stop_workers();
            }
        })),
        ..Default::default()
    });
    *latch.lock().expect("latch") = Some(Arc::clone(&supervisor));
    let manifest = write_worker_manifest(&dir, Some("hang"), None);

    let error = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone)
        .await
        .expect_err("a stop during the mint refuses the spawn");
    assert!(
        error.contains("stopped before spawn"),
        "typed refusal expected, got {error}"
    );

    let view = supervisor
        .bindings()
        .into_iter()
        .next()
        .expect("the binding exists — the stop landed AFTER bind");
    assert_eq!(view.child_pid, None, "no child was ever spawned");
    assert_eq!(view.phase, BindingPhase::PlannedStop);
    assert!(
        world
            .intents(FrameKind::CapabilityInvocation, Some(view.spirit_pid))
            .contains("lifecycle.unload"),
        "the refusal still leaves through the guard"
    );
    assert!(
        world
            .intents(FrameKind::EpistemicHalt, Some(view.spirit_pid))
            .contains("planned_unload"),
        "and still leaves a PlannedUnload receipt"
    );
    assert!(
        world.scheduler.resolve_pid("worker").is_none(),
        "no lock is held across abandon, so the SCB is really gone"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_worker_that_would_bind_after_stop_workers_is_refused() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("stop-latch");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    supervisor.stop_workers();
    let manifest = write_worker_manifest(&dir, Some("hang"), None);

    let error = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone)
        .await
        .expect_err("the latch refuses a late bind");
    assert!(
        error.contains("stopping"),
        "expected the WorkerStopping refusal, got {error}"
    );
    assert!(
        supervisor.bindings().is_empty(),
        "a Worker refused by the latch leaves no binding at all — without it, \
         a standalone run still in its admission probe, host B's next queued \
         frame or a later topology entry would spawn AFTER the SIGTERM"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (6) Two concurrent Workers: ids, and verdicts from their OWN pid's rows.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_workers_get_distinct_ids_and_their_own_verdicts() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("two-workers");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    // One clean, one non-zero. Both share the bridge's `from_spirit_id`
    // ("worker"), which is exactly why the completion read-back had to move to
    // the Worker's pid: the old sender filter mixed their output rows and each
    // verdict was computed partly from the other's.
    let clean = write_worker_manifest(&dir, None, None);
    let broken = write_worker_manifest(&dir, Some("exit-nonzero"), None);
    let (first, second) = tokio::join!(
        run_worker(
            &world,
            &supervisor,
            clean,
            WorkerTask::TopologyEntry { index: 0 }
        ),
        run_worker(
            &world,
            &supervisor,
            broken,
            WorkerTask::TopologyEntry { index: 1 }
        ),
    );

    let ids: BTreeSet<String> = supervisor
        .bindings()
        .into_iter()
        .map(|view| view.spirit_id)
        .collect();
    assert_eq!(
        ids,
        ["worker".to_string(), "worker-2".to_string()]
            .into_iter()
            .collect(),
        "the FIRST Worker of a root is `worker` so `--spirit worker` resolves"
    );
    let labels: BTreeSet<String> = [first, second]
        .into_iter()
        .map(|result| result.unwrap_or_else(|error| format!("err:{error}")))
        .collect();
    assert!(
        labels.contains("completed"),
        "the clean Worker's verdict must not inherit the other's rows: {labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|label| label.starts_with("not_completed")),
        "and the broken Worker's verdict must not inherit the clean one's: {labels:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (7) The reap race — D-16-3-B's `ECHILD` path is the NORMAL path on Linux.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_crash_the_observer_never_saw_is_still_handled_by_finish() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("reap-race");
    let world = KernelWorld::new(&dir);
    let gate = Arc::new(maos_bin::supervision::ObserverGate::default());
    let supervisor = world.supervisor(maos_bin::supervision::SupervisorSeams {
        observer_gate: Some(Arc::clone(&gate)),
        ..Default::default()
    });
    let manifest = write_worker_manifest(&dir, Some("exit-nonzero"), None);

    // The gate is held after the pidfd is stored and BEFORE the observer's
    // `waitid` CALL. A gate placed after the return would not prove the
    // observer looked at all.
    let world_tl = Arc::clone(&world.tl);
    let gate_opener = Arc::clone(&gate);
    let opener = tokio::spawn(async move {
        // Open only once the bridge has reaped — `cli.subprocess.exit` is
        // journaled after `child.wait()` returns.
        let opened = wait_for(Duration::from_secs(10), || {
            world_tl
                .query_frames(FrameFilter {
                    kind: Some(FrameKind::CapabilityInvocation),
                    ..Default::default()
                })
                .map(|rows| rows.iter().any(|row| row.intent == "cli.subprocess.exit"))
                .unwrap_or(false)
        })
        .await;
        gate_opener.open();
        opened
    });

    let _ = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone).await;
    let saw_exit_row = opener.await.expect("gate opener joined");
    assert!(saw_exit_row, "the bridge must journal its exit row");

    let pid = supervisor.bindings()[0].spirit_pid;
    // `finish` handled it: the crash rows are present even though the observer
    // was still parked when the bridge reaped.
    let crashed = wait_for(Duration::from_secs(5), || {
        world
            .intents(FrameKind::CapabilityInvocation, Some(pid))
            .contains("lifecycle.crash")
    })
    .await;
    assert!(crashed, "the reap-race crash must still reach handle_crash");
    assert!(
        world
            .rows(FrameKind::TaskComplete, Some(pid))
            .into_iter()
            .any(|row| row.intent == "task.orphaned"),
        "and must still orphan its task"
    );
    assert!(
        world
            .rows(FrameKind::TaskComplete, None)
            .into_iter()
            .any(|row| row.intent == "task.nacked" && row.payload["task_id"] == "run:worker"),
        "and must still disposition it"
    );
    // The gate opens on the exit row; the parked observer then needs its own
    // scheduling slot to call `waitid`, so wait for it rather than racing it.
    let echild = wait_for(Duration::from_secs(5), || {
        supervisor.counters().observer_echild() >= 1
    })
    .await;
    assert!(
        echild,
        "the observer DID look and DID get ECHILD — that is the whole point of \
         gating before the waitid call; got {}",
        supervisor.counters().observer_echild()
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (8) AC2's EXCLUSIVE-cursor vector — the one place nothing else prints.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn three_workers_that_printed_once_then_hung_all_stall() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("cursor");
    let world = KernelWorld::new(&dir);
    // `RootSupervision::arm` is what spawns BOTH the ProgressWatchdog and the
    // progress stamper. At `af96c907` the watchdog ran only in the serving
    // tail — after every Worker-bearing root had already returned — so nothing
    // could ever emit `task.stalled` for a Worker.
    let root = maos_bin::supervision::RootSupervision::arm(
        Arc::clone(&world.scheduler),
        Arc::clone(&world.crash_detector),
        Arc::clone(&world.tl),
        Arc::clone(&world.journal),
        Arc::clone(&world.halt_registry),
        Arc::clone(&world.iac),
        Arc::clone(&world.telemetry),
        Arc::clone(&world.notification_dispatcher),
        NONCE,
    );
    let supervisor = Arc::clone(root.supervisor());
    // `line-then-hang` prints EXACTLY ONE line and then goes silent, and
    // nothing else in this world prints. With an inclusive `since_ns`
    // high-water the Worker that printed LAST would re-stamp itself every tick
    // and never stall — and with other Workers printing, that Worker is masked.
    // This is the only vector where the defect is visible.
    let manifest = write_worker_manifest(&dir, Some("line-then-hang"), Some(5000));
    let mut runs = Vec::new();
    for index in 0..3 {
        runs.push(tokio::spawn(run_worker_owned(
            Arc::clone(&world.tl),
            Arc::clone(&world.capability),
            Arc::clone(&world.policy),
            Arc::clone(&supervisor),
            manifest.clone(),
            WorkerTask::TopologyEntry { index },
        )));
    }

    let stalled = wait_for(Duration::from_secs(25), || stalled_pids(&world).len() >= 3).await;
    let observed = stalled_pids(&world);
    let bound: BTreeSet<u32> = supervisor
        .bindings()
        .into_iter()
        .map(|view| view.spirit_pid)
        .collect();

    // Tear down BEFORE asserting. A panicking assert here would skip
    // `stop_workers`, and `line-then-hang` sleeps forever: the falsifier run
    // that proves this vector reds would then leave three fixture processes
    // alive and hang the harness instead of failing it.
    supervisor.stop_workers();
    for run in runs {
        let _ = tokio::time::timeout(Duration::from_secs(20), run).await;
    }
    root.stop_and_join().await;

    assert!(
        stalled,
        "all three must stall; stalled pids {observed:?} of bound {bound:?}"
    );
    assert!(
        bound.is_subset(&observed),
        "every bound Worker must be among the stalled: {observed:?} vs {bound:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// (11) BINARY-level vectors: the real `maos run` root.
//
// These drive the built binary with a scratch `HOME`/`MAOS_HOME`/
// `XDG_DATA_HOME` so no developer's `~/.maos/control.json` endpoint can be
// inherited (16-1's decoy-door trap), and read the row set out of the
// Transparency Log the run wrote.
// ────────────────────────────────────────────────────────────────────────────

struct RootRun {
    dir: PathBuf,
    audit_db: PathBuf,
}

impl RootRun {
    fn new(tag: &str) -> Self {
        let dir = scratch_dir(tag);
        std::fs::create_dir_all(dir.join("home")).expect("create scratch home");
        Self {
            // `maos run` opens its Transparency Log under MAOS_HOME, not from
            // MAOS_AUDIT_DB (that variable belongs to the cohort daemon root).
            // Measured by running the binary: `<MAOS_HOME>/audit/transparency.sqlite`.
            audit_db: dir.join("home/audit/transparency.sqlite"),
            dir,
        }
    }

    fn command(&self, manifest: &Path) -> std::process::Command {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_maos"));
        let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let mut paths = vec![root.join("target/debug"), root.join("target/debug/deps")];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        command
            .env_clear()
            .current_dir(&root)
            .arg("run")
            .arg(manifest)
            .arg("--once")
            // Per-command isolation: HOME decides which `control.json` the
            // root would find, and MAOS_HOME/XDG_DATA_HOME decide where it
            // writes.
            .env(
                "HOME",
                t3_substrate::isolated_runtime_home(&self.dir.join("home")),
            )
            .env("MAOS_HOME", self.dir.join("home"))
            .env("XDG_DATA_HOME", self.dir.join("home"))
            .env("MAOS_OLLAMA_URL", "skip")
            .env("PATH", std::env::join_paths(paths).expect("valid PATH"))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for key in [
            "MAOS_T3_RUNTIME",
            "MAOS_T3_IMAGE_LOCK_PATH",
            "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
    }

    /// Every row the run journaled, read back through the real adapter.
    ///
    /// Empty while the root has not opened its log yet — a poller that
    /// panicked on the missing file would be racing boot, not measuring
    /// supervision.
    fn rows(&self, kind: FrameKind) -> Vec<(u32, String)> {
        if !self.audit_db.exists() {
            return Vec::new();
        }
        let log = TransparencyLogAdapter::open(&self.audit_db, NONCE)
            .expect("reopen the run's transparency log");
        log.query_frames(FrameFilter {
            kind: Some(kind),
            ..Default::default()
        })
        .expect("query the run's transparency log")
        .into_iter()
        .map(|row| (row.spirit_pid, row.intent.clone()))
        .collect()
    }
}

#[test]
fn maos_run_refuses_a_one_shot_mode_in_the_same_process() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let run = RootRun::new("one-shot-refusal");
    let manifest = write_worker_manifest_t3(&run.dir, None, None);
    // The one-shot dispatch is gated by the env var ALONE, so without this
    // refusal a non-`--once` `maos run` with it set would fall out of the run
    // block into a one-shot arm while still holding the root's SIGTERM
    // listener — an arm that never tears it down.
    let output = run
        .command(&manifest)
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .output()
        .expect("spawn maos run");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected the typed refusal's exit 2; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("MAOS_ONE_SHOT"),
        "the refusal must name the variable it refuses; stderr:\n{stderr}"
    );
}

#[test]
fn a_hung_worker_under_the_real_root_is_reported_stalled_and_audits_clean() {
    if !t3_substrate::require_t3("binary root stalled Worker") {
        return;
    }
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let run = RootRun::new("binary-hang");
    // 5 s threshold so the stall is observable inside a test budget; the
    // production default is 30 000 ms (the NFR's "> 30s").
    let manifest = write_worker_manifest_t3(&run.dir, Some("hang"), Some(5000));
    let started = Instant::now();
    let mut child = run
        .command(&manifest)
        .spawn()
        .expect("spawn maos run with a hanging Worker");

    // The binding AC is end-to-end: the stall row must land within eight
    // seconds of spawning the binary, not merely before a generous test hang
    // guard. The 5 s manifest threshold leaves 3 s for boot and polling.
    let deadline = started + Duration::from_secs(8);
    let mut stall_pid = None;
    while Instant::now() < deadline {
        if run.audit_db.exists() {
            if let Some((pid, _)) = run
                .rows(FrameKind::TaskStalled)
                .into_iter()
                .find(|(pid, intent)| *pid != 0 && intent == "task.stalled")
            {
                stall_pid = Some(pid);
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    // Terminate the root before asserting: a panicking assert would leave a
    // `hang` fixture and a live daemon behind and hang the harness.
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    let status = wait_with_deadline(&mut child, Duration::from_secs(20));
    let stalled_rows = run.rows(FrameKind::TaskStalled);

    let pid = stall_pid.unwrap_or_else(|| {
        panic!("no `task.stalled` row at a real pid within 8 s; rows: {stalled_rows:?}")
    });
    assert_ne!(pid, 0, "the stall must be reported at the Worker's own pid");
    assert!(
        status.is_some(),
        "the root must exit through its teardown after SIGTERM"
    );
}

/// AC1's crash scene, through the real binary: the whole row set at ONE real
/// pid, and the escalated disposition at pid 0 joined to it by `task_id`.
///
/// At `af96c907` a SIGKILLed standalone Worker's entire Transparency Log was
/// three tokenless pid-0 rows: kind 21, kind 7 `cli.subprocess.exit
/// signaled(sig=9)`, kind 4 verdict. No SCB, no crash, no disposition.
///
/// Under T3 the payload is PID namespace init and cannot receive its own
/// SIGKILL, so the host resolves the PID from the published `cli_wrapper_loaded` CID,
/// verifies `/proc/<pid>/cgroup` names that CID, and kills that exact PID.
/// The attach client reports 137; the payload snapshot independently proves
/// `signaled(sig=9)` through pidfd.
#[test]
fn the_crash_scene_through_the_real_binary_lands_at_one_real_pid() {
    if !t3_substrate::require_t3("binary root crashed Worker") {
        return;
    }
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let run = RootRun::new("binary-crash");
    let base = std::fs::read_to_string(write_worker_manifest_t3(
        &run.dir,
        Some("line-then-hang"),
        None,
    ))
    .expect("read base manifest");
    let manifest = run.dir.join("crash-scene.toml");
    std::fs::write(
        &manifest,
        format!("{base}\n[on_crash]\naction = \"escalate-to-operator\"\n"),
    )
    .expect("write crash-scene manifest");

    let mut root = run
        .command(&manifest)
        .spawn()
        .expect("spawn maos run with a T3 Worker to crash");
    let mut stdout_capture = capture_stdout(&mut root);
    let deadline = Instant::now() + Duration::from_secs(30);
    let output_seen = loop {
        let seen = run.audit_db.exists()
            && TransparencyLogAdapter::open(&run.audit_db, NONCE)
                .expect("reopen the crash-scene transparency log")
                .query_frames(FrameFilter {
                    kind: Some(FrameKind::CliSubprocessOutput),
                    ..Default::default()
                })
                .expect("query Worker output rows")
                .iter()
                .any(|row| row.spirit_pid != 0 && row.intent == "cli.subprocess.output");
        if seen {
            break true;
        }
        assert!(
            Instant::now() < deadline,
            "the T3 Worker never journaled its first output line"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(output_seen);
    let container_id = wait_published_container_id(&stdout_capture, Duration::from_secs(5));
    let payload_pid = t3_payload_host_pid(&container_id);
    let killed = std::process::Command::new("kill")
        .args(["-KILL", &payload_pid.to_string()])
        .status()
        .expect("send SIGKILL to the CID-verified T3 payload");
    assert!(killed.success(), "SIGKILL must reach payload {payload_pid}");
    let status = wait_with_deadline(&mut root, Duration::from_secs(20))
        .expect("root must finish after its T3 Worker crashes");
    stdout_capture.join().expect("stdout reader reaches EOF");
    let stdout = stdout_capture
        .buffer
        .lock()
        .expect("captured root stdout")
        .clone();
    assert!(
        !status.success(),
        "a crashed Worker must fail the run; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("not_completed:process_crash"),
        "the oracle's verdict must be the six-value label, not a literal; \
         stdout:\n{stdout}"
    );
    assert_eq!(
        stdout.matches("\"event\":\"cli_wrapper_loaded\"").count(),
        1,
        "exactly one cli_wrapper_loaded line; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("\"event\":\"spirit_loaded\""),
        "a Worker prints no `spirit_loaded` line — `demo_j1` and `journey_j1` \
         pin those sets exactly; stdout:\n{stdout}"
    );

    let log = TransparencyLogAdapter::open(&run.audit_db, NONCE)
        .expect("reopen the run's transparency log");
    let all = log
        .query_frames(FrameFilter::default())
        .expect("query every row");
    // Find the ONE non-zero pid every Worker row shares.
    let worker_pids: BTreeSet<u32> = all
        .iter()
        .filter(|row| row.intent == "lifecycle.load" || row.intent == "cli.subprocess.exit")
        .map(|row| row.spirit_pid)
        .collect();
    assert_eq!(
        worker_pids.len(),
        1,
        "one Worker, one pid; saw {worker_pids:?}"
    );
    let pid = *worker_pids.iter().next().expect("one pid");
    assert_ne!(pid, 0, "every row AC1 lists must move off pid 0");

    let at_pid: Vec<&str> = all
        .iter()
        .filter(|row| row.spirit_pid == pid)
        .map(|row| row.intent.as_str())
        .collect();
    for intent in [
        "lifecycle.admit",
        "lifecycle.load",
        "lifecycle.start",
        "cli.subprocess.exit",
        "planned_unload",
        "unplanned_crash",
        "task.orphaned",
        "lifecycle.crash",
        "worker.completion-verdict",
    ] {
        assert!(
            at_pid.contains(&intent) || intent == "planned_unload",
            "missing `{intent}` at pid {pid}; rows at that pid: {at_pid:?}"
        );
    }
    let exit_row = all
        .iter()
        .find(|row| row.spirit_pid == pid && row.intent == "cli.subprocess.exit")
        .expect("the bridge's exit row");
    let exit_payload: serde_json::Value =
        serde_json::from_slice(&exit_row.payload_redacted).expect("exit row is JSON");
    let orphan_payload: serde_json::Value = all
        .iter()
        .find(|row| row.spirit_pid == pid && row.intent == "task.orphaned")
        .and_then(|row| serde_json::from_slice(&row.payload_redacted).ok())
        .unwrap_or(serde_json::Value::Null);
    let exit_keys: BTreeSet<&str> = exit_payload
        .as_object()
        .expect("exit record object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        exit_keys,
        [
            "cli_child_pid",
            "container",
            "event",
            "exit_cause",
            "is_crash",
        ]
        .into_iter()
        .collect()
    );
    let container_keys: BTreeSet<&str> = exit_payload["container"]
        .as_object()
        .expect("T3 container record object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        container_keys,
        [
            "runtime_exit_code",
            "oom_marker",
            "payload_exit",
            "payload_exit_source",
            "removed",
            "residue",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(exit_payload["exit_cause"], "exited(code=137)");
    assert_eq!(exit_payload["is_crash"], true);
    assert_eq!(
        exit_payload["container"]["runtime_exit_code"], 137,
        "exit {exit_payload}; orphan {orphan_payload}"
    );
    assert_eq!(exit_payload["container"]["oom_marker"], false);
    assert_eq!(exit_payload["container"]["payload_exit"], "signaled(sig=9)");
    assert_eq!(exit_payload["container"]["payload_exit_source"], "pidfd");
    assert_eq!(exit_payload["container"]["removed"], true);
    assert_eq!(
        exit_payload["container"]["residue"],
        serde_json::Value::Null
    );
    let orphaned = all
        .iter()
        .find(|row| row.spirit_pid == pid && row.intent == "task.orphaned")
        .expect("the crash report");
    let orphan_payload: serde_json::Value =
        serde_json::from_slice(&orphaned.payload_redacted).expect("task.orphaned is JSON");
    assert_eq!(orphan_payload["cause"], "fault.signaled");
    assert_eq!(orphan_payload["exit_signal"], 9);
    assert_eq!(orphan_payload["exit_code"], serde_json::Value::Null);
    assert_eq!(orphan_payload["task_id"], "run:worker");
    assert_eq!(orphan_payload["originator_spirit_id"], "operator");
    assert!(
        orphaned.capability_token.is_none(),
        "host-grant authority must remain tokenless instead of inventing a zero token"
    );
    assert_eq!(
        orphan_payload["in_flight_tokens"],
        serde_json::json!([]),
        "the crash payload must preserve the same missing-token state"
    );
    let crash_receipts: Vec<&_> = all
        .iter()
        .filter(|row| {
            row.spirit_pid == pid
                && row.intent == "unplanned_crash"
                && !row.payload_redacted.is_empty()
        })
        .collect();
    assert_eq!(
        crash_receipts.len(),
        1,
        "AC1 requires exactly one concrete crash receipt at the Worker pid"
    );
    let crash_receipt: serde_json::Value =
        serde_json::from_slice(&crash_receipts[0].payload_redacted)
            .expect("unplanned crash receipt is JSON");
    assert_eq!(crash_receipt["spirit_pid"], pid);
    assert!(
        crash_receipt["halt_id"]
            .as_str()
            .is_some_and(|halt_id| !halt_id.is_empty()),
        "the receipt must carry its durable halt_id: {crash_receipt}"
    );
    // The disposition is at pid 0, sender = originator, joined by `task_id`.
    let escalated = all
        .iter()
        .filter(|row| row.intent == "task.escalated")
        .find(|row| {
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                .is_ok_and(|payload| payload["task_id"] == "run:worker")
        })
        .expect("the run:worker task.escalated disposition");
    assert_eq!(
        escalated.spirit_pid, 0,
        "the originator disposition must remain at pid 0 and join by task_id"
    );
}

fn t3_payload_host_pid(container_id: &str) -> u32 {
    let runtime =
        std::env::var("MAOS_T3_RUNTIME").expect("T3 gate ensures MAOS_T3_RUNTIME is selected");
    let output = std::process::Command::new("timeout")
        .arg("8s")
        .arg(runtime)
        .args(["inspect", "--format", "{{.State.Pid}}", container_id])
        .output()
        .expect("inspect the published T3 Worker CID");
    assert!(
        output.status.success(),
        "inspect T3 CID {container_id} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pid: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or_else(|error| {
            panic!("container {container_id} reported invalid host pid: {error}")
        });
    assert!(
        pid > 1,
        "published T3 CID {container_id} has no live payload"
    );
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .unwrap_or_else(|error| panic!("read payload cgroup for {pid}: {error}"));
    assert!(
        maos_domain::sandbox::cgroup_names_container(&cgroup, container_id),
        "payload PID {pid} is not in the cgroup for published CID {container_id}: {cgroup}"
    );
    pid
}

fn published_container_id(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let event: serde_json::Value = serde_json::from_str(line).ok()?;
        (event["event"] == "cli_wrapper_loaded")
            .then(|| event["container_id"].as_str().map(str::to_owned))
            .flatten()
    })
}
struct StdoutCapture {
    buffer: Arc<Mutex<String>>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl StdoutCapture {
    fn join(&mut self) -> std::thread::Result<()> {
        self.reader
            .take()
            .expect("stdout reader already joined")
            .join()
    }
}

fn capture_stdout(child: &mut std::process::Child) -> StdoutCapture {
    use std::io::BufRead as _;
    let stdout = child.stdout.take().expect("root stdout is piped");
    let captured = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&captured);
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let mut output = sink.lock().expect("root stdout capture lock");
            output.push_str(&line);
            output.push('\n');
        }
    });
    StdoutCapture {
        buffer: captured,
        reader: Some(reader),
    }
}

fn wait_published_container_id(stdout: &StdoutCapture, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(cid) = published_container_id(&stdout.buffer.lock().expect("stdout lock")) {
            return cid;
        }
        assert!(
            Instant::now() < deadline,
            "cli_wrapper_loaded did not publish its CID"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn proc_start_time(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    stat[close + 1..]
        .split_whitespace()
        .nth(19)
        .map(str::to_owned)
}

/// AC5's binary leg: one Worker prints, stalls, and is then crashed in the
/// same root. One audit query must survive and name every omitted non-call
/// shape from that single boot nonce.
#[test]
fn the_fr4_call_feed_survives_every_worker_row_it_must_omit() {
    if !t3_substrate::require_t3("binary root stalled then crashed Worker") {
        return;
    }
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let run = RootRun::new("fr4-stall-then-crash");
    let manifest = write_worker_manifest_t3(&run.dir, Some("line-then-hang"), Some(5000));
    let mut root = run
        .command(&manifest)
        .spawn()
        .expect("spawn the stall-then-crash root");
    let mut stdout_capture = capture_stdout(&mut root);
    let deadline = Instant::now() + Duration::from_secs(20);
    let worker_pid = loop {
        if let Some((pid, _)) = run
            .rows(FrameKind::TaskStalled)
            .into_iter()
            .find(|(pid, intent)| *pid != 0 && intent == "task.stalled")
        {
            break pid;
        }
        assert!(
            Instant::now() < deadline,
            "the Worker never produced task.stalled in the single AC5 root"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let log = TransparencyLogAdapter::open(&run.audit_db, NONCE)
        .expect("reopen the single AC5 root's transparency log");
    let output_row = log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .expect("query Worker output rows")
        .into_iter()
        .find(|row| row.spirit_pid == worker_pid && row.intent == "cli.subprocess.output")
        .expect("line-then-hang must print before it stalls");
    let output_payload: serde_json::Value =
        serde_json::from_slice(&output_row.payload_redacted).expect("output row is JSON");
    assert!(output_payload["child_pid"].as_u64().is_some());
    let container_id = wait_published_container_id(&stdout_capture, Duration::from_secs(5));
    let payload_pid = t3_payload_host_pid(&container_id);
    let payload_start_time = proc_start_time(payload_pid)
        .unwrap_or_else(|| panic!("inspected T3 payload {payload_pid} has no proc identity"));
    assert_eq!(
        proc_start_time(payload_pid).as_deref(),
        Some(payload_start_time.as_str()),
        "the inspected payload PID must remain the same process before signaling"
    );
    let killed = std::process::Command::new("kill")
        .args(["-KILL", &payload_pid.to_string()])
        .status()
        .expect("send SIGKILL to the inspected T3 Worker payload");
    assert!(
        killed.success(),
        "SIGKILL must reach the exact inspected payload pid {payload_pid}"
    );

    let status = wait_with_deadline(&mut root, Duration::from_secs(20))
        .expect("root must finish after the stalled Worker crashes");
    stdout_capture.join().expect("stdout reader reaches EOF");
    let stdout = stdout_capture
        .buffer
        .lock()
        .expect("captured root stdout")
        .clone();
    assert!(
        !status.success(),
        "the crashed root must fail; stdout:\n{stdout}"
    );

    let omissions = assert_fr4_feed_is_clean(&run);
    for named in ["cli.subprocess.output×", "task.complete×", "task.stalled×"] {
        assert!(
            omissions.contains(named),
            "the one-root NDJSON view must name `{named}` among its omissions; \
             stderr:\n{omissions}"
        );
    }
}

/// Run `maos audit query --spirit worker --format ndjson` against one root's
/// log; assert it exits 0 and emits only six-key call rows. Returns its
/// omission line for the caller's own vocabulary assertions.
///
/// The DEFAULT `plain` format renders call and non-call rows with identical
/// columns and prints no count at all, so `--format ndjson` is the only view
/// in which the omission is visible.
fn assert_fr4_feed_is_clean(run: &RootRun) -> String {
    let audit = std::process::Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["audit", "query", "--spirit", "worker", "--format", "ndjson"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("HOME", run.dir.join("home"))
        .env("MAOS_HOME", run.dir.join("home"))
        .env("XDG_DATA_HOME", run.dir.join("home"))
        .output()
        .expect("run maos audit query");
    let omissions = String::from_utf8_lossy(&audit.stderr).to_string();
    assert!(
        audit.status.success(),
        "`maos audit query --spirit worker --format ndjson` must exit 0 once \
         Worker rows carry a real pid — at `af96c907` `classify_fr4_row` \
         returned `Call` for every kind-21 row and the view exited 1 with \
         Fr4SchemaViolation. stderr:\n{omissions}"
    );
    let stdout = String::from_utf8_lossy(&audit.stdout);
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("NDJSON line {line}: {e}"));
        let object = row.as_object().expect("each NDJSON row is an object");
        assert_eq!(
            object.len(),
            6,
            "the FR4 call feed's six keys are unchanged; got {object:?}"
        );
    }
    omissions
}

fn wait_with_deadline(
    child: &mut std::process::Child,
    budget: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait().expect("try_wait on the root") {
            Some(status) => return Some(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn stalled_pids(world: &KernelWorld) -> BTreeSet<u32> {
    world
        .rows(FrameKind::TaskStalled, None)
        .into_iter()
        .map(|row| row.spirit_pid)
        .collect()
}

async fn run_worker_owned(
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    supervisor: Arc<WorkerSupervisor>,
    manifest: PathBuf,
    task: WorkerTask,
) -> Result<String, String> {
    let manifest_root: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).expect("read manifest"))
            .expect("parse manifest");
    tokio::task::spawn_blocking(move || {
        t0_worker::run_t0_worker(
            &manifest_root,
            tl,
            capability,
            policy,
            supervisor.as_ref(),
            task,
        )
        .map(|completion| completion.label().to_string())
    })
    .await
    .expect("Worker blocking thread joined")
}

// ────────────────────────────────────────────────────────────────────────────
// (9) A supervised Worker completes on a ONE-WORKER runtime.
//
// `block_in_place(|| Handle::block_on(..))` is the only legal shape here: a
// direct `Handle::block_on` from a reactor worker panics, and an std `mpsc`
// handoff deadlocks when there is exactly one worker thread.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn a_supervised_worker_completes_on_a_single_worker_runtime() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("one-worker-rt");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let manifest = write_worker_manifest(&dir, None, None);
    let label = run_worker(&world, &supervisor, manifest, WorkerTask::Standalone)
        .await
        .expect("the run completes on a one-worker runtime");
    assert_eq!(label, "completed");
    assert_ne!(supervisor.bindings()[0].spirit_pid, 0);
}

// ────────────────────────────────────────────────────────────────────────────
// (10) `unload_all_loaded` is the ONE unload function.
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unload_all_loaded_closes_a_live_workers_scb_and_reports_it() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("unload-all");
    let world = KernelWorld::new(&dir);
    let supervisor = world.supervisor(Default::default());
    let manifest = write_worker_manifest(&dir, Some("hang"), None);
    let run = tokio::spawn(run_worker_owned(
        Arc::clone(&world.tl),
        Arc::clone(&world.capability),
        Arc::clone(&world.policy),
        Arc::clone(&supervisor),
        manifest,
        WorkerTask::Standalone,
    ));

    let bound = wait_for(Duration::from_secs(10), || {
        supervisor
            .bindings()
            .first()
            .is_some_and(|view| view.child_pid.is_some())
    })
    .await;
    assert!(bound, "the Worker must reach its spawn");
    let pid = supervisor.bindings()[0].spirit_pid;

    // The load-bearing property: collect the pids, DROP the `scbs()` read
    // guard, then unload. Holding the guard across `unload` (which takes the
    // write guard to remove the entry) deadlocks — and a deadlock here would
    // hang this test rather than fail it.
    let report = maos_bin::supervision::unload_all_loaded(
        world.scheduler.as_ref(),
        world.halt_registry.as_ref(),
        None,
    )
    .await;
    assert!(
        !report.had_failures(),
        "unload failures: {:?}",
        report.failed
    );
    assert!(
        world
            .intents(FrameKind::CapabilityInvocation, Some(pid))
            .contains("lifecycle.unload"),
        "the unload row must be at the Worker's own pid"
    );
    assert!(
        world.scheduler.resolve_pid("worker").is_none(),
        "the SCB is removed"
    );
    // `WorkerSpirit::on_unload` signalled the child, so the run ends.
    let _ = tokio::time::timeout(Duration::from_secs(20), run).await;
    assert!(
        supervisor.counters().signals_sent() >= 1,
        "an unload must END the process, not just flip an SCB state"
    );
}
