//! Story 16-3 — **Worker supervision and root shutdown**.
//!
//! Two capabilities live here, and the module is deliberately UNGATED so the
//! second one reaches every `maos run` root (16-2's ungated `shell_host`
//! delegates to [`unload_all_loaded`]); the Worker half is
//! `#[cfg(feature = "network")]` exactly like [`crate::worker_spawn`], so the
//! no-default-features build stays green.
//!
//! 1. **Root shutdown** ([`unload_all_loaded`], [`UnloadAllReport`]) — one
//!    unload function, one order, every root. NFR-Rel-11's planned half: a
//!    pending halt leaves a receipt, not a gap.
//! 2. **Worker supervision** ([`WorkerSupervision`], [`WorkerSupervisor`]) — a
//!    Worker subprocess gets a real SCB, a real pid on every row, an exit
//!    observation independent of pipe EOF, a progress stamp, and a task record
//!    the kernel can orphan and disposition.
//!
//! # Why an observer, and not a wait after the pump (D-16-3-B)
//!
//! `run_cli_wrapper_manifest` learns a child's exit cause from
//! `SpawnedBridge::wait_and_finalize`, which runs AFTER `pump_to_journal`
//! returns — and the pump returns on EOF of BOTH streams, i.e. when the LAST
//! process holding the pipe closes it, not when the direct child dies. Real
//! agent CLIs fork. Measured at `af96c907`: `sh -c 'sleep 5 & kill -9 $$'`
//! dies at ~0 s and the pump returns 5.0008 s later; through `maos run` with a
//! grandchild holding stdout it was still blocked 10 s later. So "kill →
//! `handle_crash` ≤ 2 s" is structurally false on the pipe path, however fast
//! `handle_crash` itself is.
//!
//! The observer is a `pidfd` + `waitid(EXITED|NOWAIT)` thread opened on the
//! spawning thread immediately after `spawn_and_bridge` returns. `NOWAIT`
//! leaves the zombie for the bridge to reap, so `wait_and_finalize` still
//! classifies and journals `cli.subprocess.exit`. T0 measured the observation
//! at 216 µs after `kill -9` with the pipe still held.
//!
//! # Platform scope, stated (§11 row 6, D-16-3-Q (3))
//!
//! `pidfd` is Linux-only in rustix, and macOS is a shipped release target, so
//! every `rustix::process` call and every `pidfd` reference lives inside
//! [`exit_observation`] under `#[cfg(target_os = "linux")]`. Elsewhere — and on
//! Linux when the root's one-time `pidfd_open` probe fails (`ENOSYS`/`EPERM`) —
//! [`exit_observation::AfterPumpExitObservation`] runs: no observer, no stored
//! child handle, exit handled by [`WorkerBinding::finish`] from the bridge's
//! `ExitCause` after the pump. The cost is real and is NOT hidden: without a
//! pidfd a stop cannot signal the child, so `maosctl unload worker` marks the
//! binding stopped while the process runs to its own exit, and a root SIGTERM
//! with such a Worker still bound exits 1 after the grace, printing
//! `could not be signalled (no pidfd)`, and orphans it.
//!
//! # The patterns, each with a job (D-16-3-Q)
//!
//! * **Ports & Adapters** — [`WorkerSupervision`] is the port
//!   `run_cli_wrapper_manifest` depends on; [`WorkerSupervisor`] is the
//!   kernel-backed adapter; `tests/worker_supervision_16_3.rs` implements the
//!   port as a double so `worker_spawn` vectors run without a kernel.
//! * **Observer** — the child process is the subject; the exit observer
//!   publishes exactly one exit event to its single subscriber, the shared
//!   [`BindingCore`]. The observer knows the platform and nothing about the
//!   kernel: it never calls `handle_crash`, `unload` or the Transparency Log.
//! * **Strategy** — [`exit_observation::ExitObservation`], two
//!   implementations, chosen once per root. The one pidfd-using act (the
//!   `Signal` executor) is a strategy method too.
//! * **State** — [`BindingPhase`] owns its transitions as pure functions.
//! * **Command** — every transition returns a [`BindingAction`] DECIDED under
//!   the binding lock and EXECUTED after it is released. That makes
//!   *lock → decide → release → act* structural instead of a review rule: the
//!   one in-lock act is `SpawnCrashHandler`'s `Handle::spawn`, which does not
//!   block.
//! * **RAII guard** — [`bind`](WorkerSupervision::bind) returns a `#[must_use]`
//!   [`WorkerBinding`]; its `Drop` performs `abandon`, so *every return between
//!   bind and finish goes through `abandon`* holds by construction, including a
//!   `?` a later edit adds. It is held together with the bridge in
//!   [`LiveWorker`] — **guard field first**, because `SpawnedBridge::drop`
//!   kills and reaps the child, and with two separate locals the bridge would
//!   drop first and the guard's signal would prove nothing.

#[cfg(feature = "network")]
use std::future::Future;

use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;

// ────────────────────────────────────────────────────────────────────────────
// Root shutdown (ungated) — D-16-3-M
// ────────────────────────────────────────────────────────────────────────────

/// What [`unload_all_loaded`] did, for the CALLER to print.
///
/// The function itself writes nothing: 16-2's `finish_shell_session` prints
/// through a `&mut impl Write` its own caller supplies, and an `async fn`
/// taking `&mut dyn Write` is not `Send`. So the report is data and the caller
/// renders it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UnloadAllReport {
    /// Halt ids that were pending before the unload and that the registry no
    /// longer knows afterwards — closed by the planned unload, with no
    /// operator resolution.
    pub closed_halts: Vec<String>,
    /// `(spirit_pid, error)` for every pid whose `unload` returned `Err`. The
    /// loop continues past a failure: one stuck Spirit must not strand the
    /// receipts of the others.
    pub failed: Vec<(u32, String)>,
}

impl UnloadAllReport {
    /// `true` when at least one `unload` failed.
    pub fn had_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Render the report's lines. Best-effort writes, like every other line on
    /// a teardown seam: a closed stdout (`maos run | head`) must not panic out
    /// of the caller and skip the audit-writer drain that follows.
    pub fn render(&self, out: &mut impl std::io::Write) {
        for halt_id in &self.closed_halts {
            let _ = writeln!(
                out,
                "halt {halt_id} closed by planned unload (no resolution)"
            );
        }
        for (pid, error) in &self.failed {
            let _ = writeln!(out, "maos: planned unload failed for pid {pid}: {error}");
        }
    }
}

/// Unload every Spirit this root loaded, so a pending halt leaves a RECEIPT
/// (NFR-Rel-11's planned half) instead of a gap.
///
/// Per pid: `drain_for_spirit_dry_run` (non-draining — the registry is the
/// evidence), then `unload(pid).await`, whose `terminate_spirit(PlannedUnload)`
/// drains every pending halt into a receipt row carrying its `halt_id`. Each
/// dry-run id the registry no longer knows afterwards is recorded as closed.
///
/// **The pid list is collected and the `scbs()` read guard is DROPPED before
/// the first `unload`** — `scbs()` is a std `RwLock` and `unload` takes its
/// write guard to remove the entry, so iterating under the read guard while
/// unloading deadlocks (Trap 10).
///
/// **Causal result wins.** An `Err` is recorded and the loop continues; the
/// caller decides what a failure means for the exit code, and only ever lets
/// it matter when the root's own result is `Ok`.
pub async fn unload_all_loaded(
    scheduler: &SpiritSchedulerAdapter,
    halt_registry: &HaltRegistry,
    post_unload: Option<&(dyn Fn(u32) + Send + Sync)>,
) -> UnloadAllReport {
    let mut report = UnloadAllReport::default();
    let pids: Vec<u32> = {
        let scbs = scheduler.scbs();
        let guard = match scbs.read() {
            Ok(guard) => guard,
            // A poisoned SCB map means a panic already happened under it.
            // Unloading nothing is honest; pretending we unloaded is not.
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.keys().copied().collect()
    };
    for pid in pids {
        let pending_before = halt_registry.drain_for_spirit_dry_run(pid);
        let result = scheduler.unload(pid).await;
        if let Some(stop) = post_unload {
            stop(pid);
        }
        if let Err(error) = result {
            report.failed.push((pid, error.to_string()));
            continue;
        }
        for (halt_id, _) in &pending_before {
            if halt_registry.lookup_state(halt_id).is_none() {
                report.closed_halts.push(halt_id.as_str().to_string());
            }
        }
    }
    report
}

/// Finish the serving root after the door and other watchdogs have drained.
///
/// Both hook callers must join before supervision stops and before any Spirit's
/// `on_unload`: dispatch budgets can outlast the other watchdogs' drain timeout.
/// Keep this order shared by the serving root and its real-signal regression.
#[cfg(feature = "network")]
pub async fn finish_serving_shutdown(
    consolidation: &crate::hook_pump::ConsolidationWatchdog,
    hooks: &crate::hook_pump::HookPump,
    stop_supervision: impl Future<Output = ()>,
    scheduler: &SpiritSchedulerAdapter,
    halt_registry: &HaltRegistry,
    post_unload: Option<&(dyn Fn(u32) + Send + Sync)>,
) -> UnloadAllReport {
    consolidation.shutdown().await;
    hooks.shutdown().await;
    stop_supervision.await;
    unload_all_loaded(scheduler, halt_registry, post_unload).await
}

// ────────────────────────────────────────────────────────────────────────────
// Worker supervision (network-gated)
// ────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "network")]
mod worker {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, Weak};
    use std::time::{Duration, Instant};

    use maos_domain::invariants::i1::{IntentClass, TokenId};
    use maos_domain::ports::task::TaskAssignmentRecord;
    use maos_domain::sandbox::{ContainerResidue, ContainerTerminalFacts};
    use maos_domain::supervision::{CrashCause, FaultCause, OnCrashAction, ReplicaResolver};
    use maos_kernel_core::halt::HaltRegistry;
    use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
    use maos_kernel_core::journal::JournalAdapter;
    use maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause;
    use maos_kernel_core::lifecycle::cli_wrapper::{
        ContainerLaunchCustody, ContainerTerminal, TEARDOWN_BUDGET,
    };
    use maos_kernel_core::scheduler::{SpiritManifestBundle, SpiritSchedulerAdapter};
    use maos_kernel_core::supervision::CrashDetector;
    use maos_spirit_abi::ctx::Ctx;
    use maos_spirit_abi::lifecycle::Spirit;

    use super::exit_observation::{
        self, ExitFacts, ExitObservation, ObserveOutcome, ObservedChild, SignalOutcome,
    };

    /// The budget for joining ONE crash handler in `finish`/`abandon`. A join
    /// that exceeds it leaves the handle to [`WorkerSupervisor::join_outstanding`]
    /// — no disposition, no unload by that path; the teardown's
    /// `unload_all_loaded` owns what is left.
    pub const CRASH_HANDLER_JOIN_BUDGET: Duration = Duration::from_secs(5);

    /// The default progress threshold: the NFR's "no progress IAC for > 30 s".
    pub const DEFAULT_PROGRESS_THRESHOLD_MS: u32 = 30_000;

    // ── The task record's source (D-16-3-E) ─────────────────────────────

    /// WHERE a Worker run came from. The caller passes this; `bind` derives the
    /// record from it, because the caller holds no SCB id (ids are assigned
    /// inside `bind`) and must not invent a task id.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum WorkerTask {
        /// `maos run <cli_wrapper manifest>` — the standalone root. The run IS
        /// the operator's task: without a record the watchdog cannot see it and
        /// a crash orphans nothing.
        Standalone,
        /// A `[topology]` member with no `host` — a local Worker entry.
        TopologyEntry { index: usize },
        /// A delegated Worker: a `[topology]` member WITH a `host`, or host B
        /// serving an inbound frame.
        Delegation { frame_id: [u8; 16] },
    }

    impl WorkerTask {
        /// The record's `task_id` and `originator_spirit_id`.
        ///
        /// **No id contains `sk-` and none carries 32 contiguous hex chars**
        /// (Trap 9): the Transparency Log's redaction scrubber replaces any
        /// `sk-` substring and any ≥32-char hex run, and a redacted task id
        /// cannot be joined to its disposition row. That is why a delegation id
        /// is four 8-hex groups separated by `:` rather than a raw 32-hex frame
        /// id.
        pub fn record_ids(&self, spirit_id: &str) -> (String, String) {
            match self {
                WorkerTask::Standalone => (format!("run:{spirit_id}"), "operator".to_string()),
                WorkerTask::TopologyEntry { index } => {
                    (format!("topology:{index}"), "topology".to_string())
                }
                WorkerTask::Delegation { frame_id } => {
                    let hex: String = frame_id.iter().map(|b| format!("{b:02x}")).collect();
                    let grouped = [&hex[0..8], &hex[8..16], &hex[16..24], &hex[24..32]].join(":");
                    (
                        format!("delegation:{grouped}"),
                        crate::delegation::FROM_SPIRIT.to_string(),
                    )
                }
            }
        }
    }

    // ── Errors ──────────────────────────────────────────────────────────

    /// Why a `bind` or a post-bind gate refused.
    #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
    #[non_exhaustive]
    pub enum WorkerSupervisionError {
        /// `stop_workers()` has latched: the root is shutting down, so a Worker
        /// that would bind now never runs. Without this latch a standalone run
        /// still in its admission probe, host B's next queued frame, or a later
        /// topology entry would spawn AFTER the SIGTERM and trip the grace exit
        /// with a false descendant message.
        #[error("maos run: worker supervision is stopping; refusing to bind a new worker")]
        WorkerStopping,
        /// A stop landed between `bind` and the spawn (during the mint), so the
        /// child was never started.
        #[error("maos run: worker stopped before spawn")]
        StoppedBeforeSpawn,
        /// `scheduler.load` refused.
        #[error("maos run: worker scb load failed: {0}")]
        Load(String),
        /// `scheduler.start` refused. The SCB is left `Loaded`; see the
        /// declared residual on the `epic-16-retrospective` row — there is no
        /// `Loaded → Unloaded` transition, so it cannot be unloaded and gets no
        /// receipt.
        #[error("maos run: worker scb start failed: {0}")]
        Start(String),
        /// The kernel could not commit the confirmed T3 spawn report.
        #[error("maos run: worker T3 spawn report commit failed: {0}")]
        SpawnReport(String),
    }

    // ── The state machine (D-16-3-Q (4)) ────────────────────────────────

    /// One Worker binding's phase. Transitions are pure functions of
    /// (phase, input) returning the next phase and the [`BindingAction`] to
    /// execute after the lock is released.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BindingPhase {
        /// Bound, and neither a stop nor an exit has been observed.
        Running,
        /// A crash was observed and `handle_crash` was spawned for it.
        CrashHandled,
        /// A stop was requested (`stop_workers()` or `WorkerSpirit::on_unload`).
        PlannedStop,
        /// The child exited 0 and the clean-exit path ran.
        ExitedClean,
    }

    /// A transition that cannot happen. Kept as a typed refusal rather than a
    /// panic so the unit table can assert it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
    #[non_exhaustive]
    pub enum PhaseRefusal {
        /// The pidfd is stored before the observer starts and before the pump,
        /// so no exit can have been handled yet.
        #[error("pidfd stored in phase {0:?}, after the binding already finished")]
        PidfdStoredAfterExit(BindingPhase),
    }

    /// The command a transition decided on. DECIDED under the binding lock,
    /// EXECUTED after it is released — with exactly one documented exception,
    /// [`BindingAction::SpawnCrashHandler`], whose `Handle::spawn` does not
    /// block.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BindingAction {
        /// Nothing to do.
        Nothing,
        /// Signal the runtime-client pidfd and tear down the attached CID, if any.
        Signal,
        /// The sole in-lock exception: spawn `handle_crash` and retain its
        /// handle, so no reader observes `CrashHandled` without an owner.
        SpawnCrashHandler(CrashCause),
        /// Run `handle_crash` bounded, then disposition only if the SCB was
        /// already unloaded by a concurrent action.
        RunCrashHandlerThenMaybeDisposition(CrashCause),
        /// Join the stored handler bounded, then disposition only if the SCB
        /// was already unloaded by a concurrent action.
        JoinHandlerThenMaybeDisposition,
        /// As above, then unload the SCB.
        JoinHandlerThenMaybeDispositionThenUnload,
        /// Disposition from the kept record, then unload the SCB.
        DispositionAndUnload,
        /// Signal the runtime-client pidfd and tear down the CID while the
        /// bridge remains alive, then disposition and unload.
        SignalThenDispositionAndUnload,
        /// Retire a successful task's ledger record without crash disposition,
        /// then unload its SCB.
        UnloadClean,
    }

    impl BindingPhase {
        /// The pidfd has just been stored under the lock. A stop that landed
        /// during the mint is signalled NOW — this is the window D-16-3-D's
        /// "lock: store the pidfd; if phase is `PlannedStop` ⇒ signal" closes.
        pub fn on_pidfd_stored(self) -> Result<(Self, BindingAction), PhaseRefusal> {
            match self {
                BindingPhase::Running => Ok((BindingPhase::Running, BindingAction::Nothing)),
                BindingPhase::PlannedStop => Ok((BindingPhase::PlannedStop, BindingAction::Signal)),
                other => Err(PhaseRefusal::PidfdStoredAfterExit(other)),
            }
        }

        /// The observer saw the child exit. A clean exit does NOTHING here:
        /// `finish` owns clean exits, because only the call stack knows whether
        /// the completion oracle accepted the run.
        pub fn on_exit(self, cause: &CrashCause) -> (Self, BindingAction) {
            match (self, cause) {
                (BindingPhase::Running, CrashCause::Fault(_)) => (
                    BindingPhase::CrashHandled,
                    BindingAction::SpawnCrashHandler(cause.clone()),
                ),
                (phase, _) => (phase, BindingAction::Nothing),
            }
        }

        /// A stop was requested. `has_stop_target` is a pidfd handle or an
        /// attached container identity, either of which requires the signal
        /// action.
        pub fn begin_stop(self, has_stop_target: bool) -> (Self, BindingAction) {
            match self {
                BindingPhase::Running => (
                    BindingPhase::PlannedStop,
                    if has_stop_target {
                        BindingAction::Signal
                    } else {
                        BindingAction::Nothing
                    },
                ),
                phase => (phase, BindingAction::Nothing),
            }
        }

        /// The Worker's call stack reached the end of the run. `bridge_exit` is
        /// the cause derived from the bridge's own `ExitCause` — `None` for a
        /// clean exit.
        ///
        /// `Running` + a crash is NOT dead code on Linux: it is the path taken
        /// whenever the observer lost the reap race (`ECHILD`), which measured
        /// 106/200 on fast exits.
        pub fn finish(self, bridge_exit: Option<&CrashCause>) -> (Self, BindingAction) {
            match (self, bridge_exit) {
                (BindingPhase::CrashHandled, _) => (
                    BindingPhase::CrashHandled,
                    BindingAction::JoinHandlerThenMaybeDisposition,
                ),
                (BindingPhase::Running, Some(cause)) => (
                    BindingPhase::CrashHandled,
                    BindingAction::RunCrashHandlerThenMaybeDisposition(cause.clone()),
                ),
                (BindingPhase::Running, None) => {
                    (BindingPhase::ExitedClean, BindingAction::UnloadClean)
                }
                // A stopped Worker's task is as dead to its originator as a
                // crashed one's — and the unload is this path's own job,
                // because a topology Worker stopped by SIGTERM exits through a
                // not-completed `return Err` before any teardown runs.
                (BindingPhase::PlannedStop, _) => (
                    BindingPhase::PlannedStop,
                    BindingAction::DispositionAndUnload,
                ),
                (BindingPhase::ExitedClean, _) => {
                    (BindingPhase::ExitedClean, BindingAction::Nothing)
                }
            }
        }

        /// The guard is being dropped without `finish` — an early return, a
        /// `?`, or a panic unwinding between bind and finish.
        pub fn abandon(self) -> (Self, BindingAction) {
            match self {
                // NEVER `PlannedStop` alone: the unload's own `on_unload` →
                // `begin_stop(PlannedStop)` yields `Nothing`, so a child
                // spawned after bind would never be signalled.
                BindingPhase::Running => (
                    BindingPhase::PlannedStop,
                    BindingAction::SignalThenDispositionAndUnload,
                ),
                // `handle_crash` has already drained the ledger and
                // dispositioned, so a plain disposition here would write a
                // second `task.nacked`/`task.escalated`.
                BindingPhase::CrashHandled => (
                    BindingPhase::CrashHandled,
                    BindingAction::JoinHandlerThenMaybeDispositionThenUnload,
                ),
                BindingPhase::PlannedStop => (
                    BindingPhase::PlannedStop,
                    BindingAction::DispositionAndUnload,
                ),
                BindingPhase::ExitedClean => (BindingPhase::ExitedClean, BindingAction::Nothing),
            }
        }
    }

    // ── The kernel executor (Command executors) ─────────────────────────

    /// What a spawned `handle_crash` reported. Deliberately small: the
    /// executor's only caller-visible distinction is "a concurrent unload
    /// removed the SCB first, so the record still needs a disposition".
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CrashOutcome {
        /// `handle_crash` ran: the ledger is drained and dispositioned.
        Handled,
        /// `HandleCrashError::NotLoaded` — a concurrent `unload` (or a second
        /// call) got there first. The kept record still needs a disposition.
        NotLoaded,
    }

    /// A joinable spawned crash handler.
    pub struct CrashHandlerHandle(tokio::task::JoinHandle<CrashOutcome>);

    /// The side-effecting half of the Command pattern: everything a
    /// [`BindingAction`] has to DO. Separated from [`WorkerSupervision`] so a
    /// [`BindingCore`] can hold it without an `Arc` cycle back to the
    /// supervisor, and so a test double can record actions without a kernel.
    pub trait BindingExecutor: Send + Sync {
        /// Classify a native child exit from the runtime-client status.
        fn classify_exit(&self, spirit_pid: u32, facts: ExitFacts) -> CrashCause;
        /// Classify a T3 child exit from its immutable payload/container evidence.
        fn classify_exit_with_container(
            &self,
            spirit_pid: u32,
            facts: ExitFacts,
            container: Option<&ContainerTerminalFacts>,
        ) -> CrashCause {
            let _ = container;
            self.classify_exit(spirit_pid, facts)
        }
        /// Spawn `handle_crash` on the executor's runtime. The returned handle
        /// is owned by the binding; spawning does not block. `record` is the
        /// binding's active record, if any: the orphan delivery's input.
        fn spawn_crash_handler(
            &self,
            spirit_pid: u32,
            cause: CrashCause,
            record: Option<TaskAssignmentRecord>,
        ) -> CrashHandlerHandle;
        /// Join a spawned handler within `budget`. `Err` returns the handle so
        /// the caller can park it for `join_outstanding`.
        fn join_crash_handler(
            &self,
            handle: CrashHandlerHandle,
            budget: Duration,
        ) -> Result<CrashOutcome, CrashHandlerHandle>;
        /// `block_on(handle_crash)` within `budget`. `None` on timeout.
        fn run_crash_handler(
            &self,
            spirit_pid: u32,
            cause: CrashCause,
            record: Option<TaskAssignmentRecord>,
            budget: Duration,
        ) -> Option<CrashOutcome>;
        /// Park a handler whose join timed out.
        fn park_outstanding(&self, handle: CrashHandlerHandle);
        /// Take the record out of the SCB ledger if it is still there, then
        /// apply the FR50 disposition captured at bind. `crashed` is the
        /// crash-phase `NotLoaded` path, the only one that delivers an orphan.
        fn disposition(
            &self,
            spirit_pid: u32,
            action: OnCrashAction,
            record: &TaskAssignmentRecord,
            crashed: bool,
        );
        /// Take the record out of the SCB ledger WITHOUT a disposition.
        fn take_record(&self, spirit_pid: u32, task_id: &str);
        /// `scheduler.unload(pid)` — idempotent for a missing or already
        /// `Unloaded` pid.
        fn unload(&self, spirit_pid: u32);
        /// Journal a per-binding `telemetry.event` row at the Worker's pid.
        fn journal_event(&self, spirit_pid: u32, intent: &str, payload: String);
        /// Patch the minted capability token into the SCB ledger entry.
        fn patch_ledger_token(
            &self,
            spirit_pid: u32,
            task_id: &str,
            token_id: TokenId,
            ttl_deadline_ns: u64,
        );
    }

    // ── Counters and test seams ─────────────────────────────────────────

    /// Supervisor-wide counters. `signals_sent` is the observable AC3 asserts:
    /// the bridge's own `Drop` kills and reaps the child, so with the guard's
    /// signal removed every OTHER observable stays green (validation round 9
    /// probed both mutations 200×) — only this counter reds.
    #[derive(Debug, Default)]
    pub struct SupervisorCounters {
        signals_sent: AtomicU64,
        observer_echild: AtomicU64,
        observer_unavailable: AtomicU64,
    }

    impl SupervisorCounters {
        /// Signals actually delivered (`SignalOutcome::Signalled`).
        pub fn signals_sent(&self) -> u64 {
            self.signals_sent.load(Ordering::Relaxed)
        }
        /// Exits the observer could not see because the bridge reaped first.
        pub fn observer_echild(&self) -> u64 {
            self.observer_echild.load(Ordering::Relaxed)
        }
        /// Bindings that ran without an observer.
        pub fn observer_unavailable(&self) -> u64 {
            self.observer_unavailable.load(Ordering::Relaxed)
        }

        /// The observer lost the reap race. Recorded so the AC3 reap-race
        /// vector can assert the observer DID look and DID get `ECHILD` — a
        /// gate placed after the `waitid` return would not prove that.
        pub(super) fn note_echild(&self) {
            self.observer_echild.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A one-shot gate a test opens. Used ONLY by the AC3 reap-race vector,
    /// which must hold the observer between "pidfd stored" and "`waitid`
    /// called" while the bridge reaps.
    #[derive(Debug, Default)]
    pub struct ObserverGate {
        open: Mutex<bool>,
        cv: std::sync::Condvar,
    }

    impl ObserverGate {
        /// Let the observer proceed to its `waitid` call.
        pub fn open(&self) {
            let mut open = self.open.lock().expect("observer gate poisoned");
            *open = true;
            self.cv.notify_all();
        }

        pub(super) fn wait(&self) {
            let mut open = self.open.lock().expect("observer gate poisoned");
            while !*open {
                open = self.cv.wait(open).expect("observer gate poisoned");
            }
        }
    }

    /// Test seams on [`WorkerSupervisor`], set through
    /// [`WorkerSupervisor::with_seams`].
    ///
    /// `#[doc(hidden)] pub`, never `#[cfg(test)]` and never an env var:
    /// integration tests in `crates/maos-bin/tests/` are separate crates and
    /// cannot see a `cfg(test)` item, and inline test code is kloc-charged.
    #[doc(hidden)]
    #[derive(Clone)]
    pub enum OnUnloadFault {
        /// Panic before the hook's own stop call.
        Panic,
        /// Block the hook for this finite duration before its own stop call.
        Sleep(Duration),
    }

    #[doc(hidden)]
    #[derive(Clone)]
    pub struct SupervisorSeams {
        /// Run after capability minting and before the pre-spawn phase check.
        pub after_mint: Option<Arc<dyn Fn() + Send + Sync>>,
        /// Gate the observer between pidfd storage and its `waitid` call.
        pub observer_gate: Option<Arc<ObserverGate>>,
        /// Observe the runtime-client pid immediately after watch starts.
        pub after_watch: Option<Arc<dyn Fn(u32) + Send + Sync>>,
        /// Inject a post-watch early-return error to exercise RAII cleanup.
        pub error_after_watch: Option<String>,
        /// Inject a panic or finite sleep before `WorkerSpirit::on_unload` stops.
        pub on_unload_fault: Option<OnUnloadFault>,
        /// Override the Worker bundle's hook time cap for deterministic tests.
        pub hook_cap_override: Option<u32>,
        /// Request payload pidfd evidence; false forces the runtime fallback.
        pub payload_pidfd: bool,
    }

    impl Default for SupervisorSeams {
        fn default() -> Self {
            Self {
                after_mint: None,
                observer_gate: None,
                after_watch: None,
                error_after_watch: None,
                on_unload_fault: None,
                hook_cap_override: None,
                payload_pidfd: true,
            }
        }
    }

    struct BindingState {
        /// Monotonic lifecycle phase; planned stop wins later exit evidence.
        phase: BindingPhase,
        /// Runtime-client pidfd, retained only while the binding is active.
        child: Option<ObservedChild>,
        /// Runtime-client OS pid, never the payload's historical pid.
        child_pid: Option<u32>,
        /// Shared CID terminal handle, retained for the binding's full life.
        container_terminal: Option<Arc<ContainerTerminal>>,
        /// Launch coordinator available before the runtime can create a CID.
        container_launch: Option<Arc<ContainerLaunchCustody>>,
        /// Teardown returned success: removal and its existence check completed.
        container_teardown_confirmed: bool,
        /// First typed teardown failure, if any.
        container_residue: Option<ContainerResidue>,
        /// Prevent duplicate stderr reporting across finish/drop/root cleanup.
        container_residue_reported: bool,
        /// Fail-closed bearer revocation for a stop left unconfirmed (AC4.5).
        lease_revoke: Option<Arc<dyn Fn() + Send + Sync>>,
        /// Whether the successful stop signal has already contributed once.
        signal_counted: bool,
        /// At most one crash handler per binding.
        handler: Option<CrashHandlerHandle>,
        /// Kept task record used if the SCB ledger is concurrently removed.
        record: TaskAssignmentRecord,
        /// Disposition selected when this binding was created.
        on_crash_action: OnCrashAction,
        /// Disposition has already been written.
        dispositioned: bool,
        /// A live scheduler ledger entry belongs to this binding.
        record_active: bool,
        /// This binding owns one short-lived task rather than a persistent record.
        turn_scoped: bool,
        /// The runner completed its readiness handshake.
        ready: bool,
    }

    // ── The binding's shared core (Observer's subscriber) ───────────────

    /// The lock-guarded state the observer thread and the Worker's call stack
    /// both reach — the Observer pattern's single subscriber.
    ///
    /// [`WorkerBinding`] is the RAII handle over this core and is held by the
    /// call stack ONLY: the observer can never reach the guard, which
    /// `finish(self)` consumes.
    pub struct BindingCore {
        spirit_id: String,
        spirit_pid: AtomicU32,
        state: Mutex<BindingState>,
        exec: Arc<dyn BindingExecutor>,
        strategy: Arc<dyn ExitObservation>,
        counters: Arc<SupervisorCounters>,
        seams: SupervisorSeams,
    }

    impl BindingCore {
        /// The Worker's SCB id — `worker`, `worker-2`, …
        pub fn spirit_id(&self) -> &str {
            &self.spirit_id
        }

        /// The Worker's SCB pid. 0 only between construction and `load`.
        pub fn spirit_pid(&self) -> u32 {
            self.spirit_pid.load(Ordering::Acquire)
        }

        /// Admission's on-loaded callback binds the only scheduler-owned PID.
        pub fn bind_scheduler_pid(&self, pid: u32) {
            assert_ne!(pid, 0, "scheduler PID must not be a placeholder");
            self.spirit_pid
                .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
                .expect("scheduler PID is already bound");
        }

        /// Register launch custody before entering any runtime call that can create a CID.
        pub fn attach_launch_custody(&self, custody: Arc<ContainerLaunchCustody>) {
            let (late, duplicate) = {
                let mut state = self.lock();
                match &state.container_launch {
                    Some(existing) => (
                        state.phase != BindingPhase::Running,
                        !Arc::ptr_eq(existing, &custody),
                    ),
                    None => {
                        state.container_launch = Some(Arc::clone(&custody));
                        (state.phase != BindingPhase::Running, false)
                    }
                }
            };
            if duplicate || late {
                let _ = custody.cancel_and_teardown(Instant::now() + TEARDOWN_BUDGET);
            }
        }

        /// Retain the T3 terminal handle for the binding's whole life.
        pub fn attach_container(&self, terminal: Arc<ContainerTerminal>) {
            let (late, duplicate, launch_registered) = {
                let mut state = self.lock();
                let launch_registered = state.container_launch.is_some();
                match &state.container_terminal {
                    Some(existing) => (
                        state.phase != BindingPhase::Running,
                        !Arc::ptr_eq(existing, &terminal),
                        launch_registered,
                    ),
                    None => {
                        state.container_terminal = Some(Arc::clone(&terminal));
                        (
                            state.phase != BindingPhase::Running,
                            false,
                            launch_registered,
                        )
                    }
                }
            };
            if duplicate {
                // A second terminal is never adopted; remove it by its own CID.
                let _ = terminal.teardown(Instant::now() + TEARDOWN_BUDGET);
            } else if late && !launch_registered {
                self.signal(Instant::now() + TEARDOWN_BUDGET);
            }
        }

        /// A session timeout/error marks the kill as planned before signalling;
        /// a binding already past `Running` still gets its CID cleaned up.
        pub fn stop_session(&self) {
            let action = self.begin_stop();
            self.execute(action);
            self.cleanup_container(Instant::now() + TEARDOWN_BUDGET);
        }

        /// The current phase.
        pub fn phase(&self) -> BindingPhase {
            self.lock().phase
        }

        /// The kept record's task id.
        pub fn task_id(&self) -> String {
            self.lock().record.task_id.clone()
        }

        /// The turn-scoped session's runner announced `Ready`: from now on every
        /// non-zero exit is a crash.
        pub fn mark_ready(&self) {
            self.lock().ready = true;
        }

        /// A turn-scoped session's runner exit with a typed refusal code
        /// (`maos-wasm-runner`: 2 incompatible world, 3 invalid component,
        /// 5 unrepresentable frame) before `Ready` is the launch refusing the
        /// component, not the Spirit crashing.
        fn is_typed_refusal(&self, facts: &ExitFacts) -> bool {
            let state = self.lock();
            state.turn_scoped
                && !state.ready
                && facts.signal.is_none()
                && matches!(facts.code, Some(2 | 3 | 5))
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, BindingState> {
            match self.state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }
        }

        /// The observer's single publish. Called from the observer thread with
        /// the platform facts and nothing else.
        ///
        /// The classification (a Transparency Log read for the stderr tail)
        /// happens BEFORE the lock is taken; the `Handle::spawn` inside it is
        /// the documented non-blocking exception.
        pub(super) fn on_child_exit(&self, facts: ExitFacts) {
            let pid = self.spirit_pid();
            let terminal = { self.lock().container_terminal.clone() };
            let terminal_facts = terminal.map(|terminal| terminal.facts());
            let cause = if self.is_typed_refusal(&facts) {
                CrashCause::Voluntary
            } else {
                self.exec
                    .classify_exit_with_container(pid, facts, terminal_facts.as_deref())
            };
            let mut state = self.lock();
            let (next, action) = state.phase.on_exit(&cause);
            state.phase = next;
            if let BindingAction::SpawnCrashHandler(cause) = action {
                let record = state.record_active.then(|| state.record.clone());
                state.handler = Some(self.exec.spawn_crash_handler(pid, cause, record));
            }
        }

        /// Store the child handle the strategy opened, and decide whether a
        /// stop that landed during the mint must be signalled now.
        pub(super) fn store_child(
            &self,
            child_pid: u32,
            child: ObservedChild,
        ) -> Result<BindingAction, PhaseRefusal> {
            let mut state = self.lock();
            let (next, action) = state.phase.on_pidfd_stored()?;
            state.phase = next;
            state.child = Some(child);
            state.child_pid = Some(child_pid);
            Ok(action)
        }

        /// Record the child pid even when no handle could be opened, so
        /// `bindings()` and the no-pidfd message can name the process.
        pub(super) fn note_child_pid(&self, child_pid: u32) {
            self.lock().child_pid = Some(child_pid);
        }

        pub(super) fn child_handle(&self) -> Option<ObservedChild> {
            self.lock().child.clone()
        }

        /// The AC3 reap-race vector's gate, if one was installed. The observer
        /// thread holds it after the pidfd is stored and BEFORE its `waitid`
        /// CALL — a gate placed after the return would prove nothing.
        pub(super) fn observer_gate(&self) -> Option<Arc<ObserverGate>> {
            self.seams.observer_gate.clone()
        }

        fn begin_stop(&self) -> BindingAction {
            let mut state = self.lock();
            let has_stop_target = state.child.is_some()
                || state.container_terminal.is_some()
                || state.container_launch.is_some();
            let (next, action) = state.phase.begin_stop(has_stop_target);
            state.phase = next;
            action
        }

        fn view(&self) -> WorkerBindingView {
            let state = self.lock();
            WorkerBindingView {
                spirit_id: self.spirit_id.clone(),
                spirit_pid: self.spirit_pid(),
                child_pid: state.child_pid,
                child: state.child.clone(),
                phase: state.phase,
            }
        }

        /// Snapshot terminal metadata and release the binding-owned child
        /// handle. The pid remains in `child_pid` for audit/corpus lookup, but
        /// no completed binding keeps a pidfd alive.
        fn retire_view(&self) -> WorkerBindingView {
            let mut state = self.lock();
            state.child = None;
            WorkerBindingView {
                spirit_id: self.spirit_id.clone(),
                spirit_pid: self.spirit_pid(),
                child_pid: state.child_pid,
                child: None,
                phase: state.phase,
            }
        }

        /// Execute outside the binding lock. Scheduler mechanism (x) releases
        /// the SCB, receipt, and capabilities before entering the hook.
        fn execute(&self, action: BindingAction) {
            self.execute_until(action, Instant::now() + TEARDOWN_BUDGET);
        }

        fn execute_until(&self, action: BindingAction, deadline: Instant) {
            let pid = self.spirit_pid();
            match action {
                BindingAction::Nothing | BindingAction::SpawnCrashHandler(_) => {}
                BindingAction::Signal => self.signal(deadline),
                BindingAction::SignalThenDispositionAndUnload => {
                    self.signal(deadline);
                    self.disposition();
                    self.exec.unload(pid);
                }
                BindingAction::DispositionAndUnload => {
                    self.disposition();
                    self.exec.unload(pid);
                }
                BindingAction::UnloadClean => {
                    let (turn_scoped, record_active, task_id) = {
                        let state = self.lock();
                        (
                            state.turn_scoped,
                            state.record_active,
                            state.record.task_id.clone(),
                        )
                    };
                    if turn_scoped {
                        self.disposition();
                    } else if record_active {
                        self.exec.take_record(pid, &task_id);
                    }
                    self.exec.unload(pid);
                }
                BindingAction::RunCrashHandlerThenMaybeDisposition(cause) => {
                    let record = {
                        let state = self.lock();
                        state.record_active.then(|| state.record.clone())
                    };
                    match self
                        .exec
                        .run_crash_handler(pid, cause, record, CRASH_HANDLER_JOIN_BUDGET)
                    {
                        Some(CrashOutcome::NotLoaded) => self.disposition(),
                        Some(CrashOutcome::Handled) => self.mark_dispositioned(),
                        None => {}
                    }
                }
                BindingAction::JoinHandlerThenMaybeDisposition => {
                    if self.join_handler() {
                        self.disposition();
                    }
                }
                BindingAction::JoinHandlerThenMaybeDispositionThenUnload => {
                    if self.join_handler() {
                        self.disposition();
                    }
                    self.exec.unload(pid);
                }
            }
        }

        pub fn container_terminal(&self) -> Option<Arc<ContainerTerminal>> {
            self.lock().container_terminal.clone()
        }

        pub fn termination_confirmed(&self) -> bool {
            let (custody, terminal) = {
                let state = self.lock();
                (
                    state.container_launch.clone(),
                    state.container_terminal.clone(),
                )
            };
            custody
                .map(|custody| custody.termination_confirmed())
                .or_else(|| terminal.map(|terminal| terminal.termination_confirmed()))
                .unwrap_or(true)
        }

        /// Record the one CID verdict. The kernel already reported residue on
        /// root stderr, so this only marks it reported and, fail-closed,
        /// revokes the Worker's bearer while termination is unconfirmed.
        fn note_container_teardown(&self, result: Result<(), ContainerResidue>) {
            let revoke = {
                let mut state = self.lock();
                state.container_teardown_confirmed = result.is_ok();
                state.container_residue = result.err();
                state.container_residue_reported |= result.is_err();
                result
                    .is_err()
                    .then(|| state.lease_revoke.clone())
                    .flatten()
            };
            if let Some(revoke) = revoke {
                if !self.termination_confirmed() {
                    revoke();
                }
            }
        }

        /// Mechanical CID cleanup for every stop path, whatever the phase: a
        /// crash-handled binding still owns a container (fault cleanup never
        /// latches `PlannedStop` and never rewrites the frozen snapshot).
        fn cleanup_container(&self, deadline: Instant) {
            let (custody, terminal) = {
                let state = self.lock();
                (
                    state.container_launch.clone(),
                    state.container_terminal.clone(),
                )
            };
            let result = custody
                .map(|custody| custody.cancel_and_teardown(deadline))
                .or_else(|| terminal.map(|terminal| terminal.teardown(deadline)));
            if let Some(result) = result {
                self.note_container_teardown(result);
            }
        }

        fn unresolved_container(&self) -> Option<(String, String, ContainerResidue, bool)> {
            let (terminal, teardown_confirmed, residue, reported) = {
                let state = self.lock();
                (
                    state.container_terminal.clone(),
                    state.container_teardown_confirmed,
                    state.container_residue,
                    state.container_residue_reported,
                )
            };
            let terminal = terminal?;
            if teardown_confirmed && residue.is_none() {
                return None;
            }
            Some((
                self.spirit_id.clone(),
                terminal.container_id().to_owned(),
                residue.unwrap_or(ContainerResidue::TerminationUnconfirmed),
                reported,
            ))
        }

        fn take_unreported_container(&self) -> Option<(String, String, ContainerResidue)> {
            let mut state = self.lock();
            if state.container_residue_reported
                || (state.container_teardown_confirmed && state.container_residue.is_none())
            {
                return None;
            }
            let terminal = state.container_terminal.clone()?;
            state.container_residue_reported = true;
            Some((
                self.spirit_id.clone(),
                terminal.container_id().to_owned(),
                state
                    .container_residue
                    .unwrap_or(ContainerResidue::TerminationUnconfirmed),
            ))
        }

        pub(super) fn signal(&self, deadline: Instant) {
            let (custody, terminal) = {
                let state = self.lock();
                (
                    state.container_launch.clone(),
                    state.container_terminal.clone(),
                )
            };
            let pidfd_outcome = self.strategy.signal_kill(self);
            let teardown = custody
                .as_ref()
                .map(|custody| custody.cancel_and_teardown(deadline))
                .or_else(|| {
                    terminal
                        .as_ref()
                        .map(|terminal| terminal.teardown(deadline))
                });
            if let Some(result) = teardown.as_ref() {
                self.note_container_teardown(*result);
            }
            let delivered = pidfd_outcome == SignalOutcome::Signalled;
            let container_ok = teardown.as_ref().is_none_or(|result| result.is_ok())
                && custody.as_ref().map_or_else(
                    || {
                        terminal
                            .as_ref()
                            .is_none_or(|terminal| terminal.kill_succeeded())
                    },
                    |custody| custody.kill_succeeded(),
                );
            if delivered && container_ok {
                let mut state = self.lock();
                if !state.signal_counted {
                    self.counters.signals_sent.fetch_add(1, Ordering::Relaxed);
                    state.signal_counted = true;
                }
            }
        }

        /// Join the stored handler. Returns `true` when the record still needs
        /// a disposition (the handler reported `NotLoaded`). A join that times
        /// out parks the handle and returns `false`: no disposition, no unload
        /// by this path.
        fn join_handler(&self) -> bool {
            let handle = self.lock().handler.take();
            let Some(handle) = handle else {
                return false;
            };
            match self
                .exec
                .join_crash_handler(handle, CRASH_HANDLER_JOIN_BUDGET)
            {
                Ok(CrashOutcome::NotLoaded) => true,
                Ok(CrashOutcome::Handled) => {
                    self.mark_dispositioned();
                    false
                }
                Err(handle) => {
                    self.exec.park_outstanding(handle);
                    false
                }
            }
        }

        fn mark_dispositioned(&self) {
            self.lock().dispositioned = true;
        }

        /// Disposition from the KEPT record — never from
        /// `scb.runtime_snapshot()`, because a door `unload` removes the SCB
        /// first and a fallback to the default `Nack` would write
        /// `task.nacked` for an `escalate-to-operator` Worker.
        ///
        /// Idempotent: once `dispositioned` is set every `Disposition*`
        /// executor is a no-op.
        fn disposition(&self) {
            let (action, record, crashed) = {
                let mut state = self.lock();
                if state.dispositioned || !state.record_active {
                    return;
                }
                state.dispositioned = true;
                let crashed = state.phase == BindingPhase::CrashHandled;
                (state.on_crash_action, state.record.clone(), crashed)
            };
            self.exec
                .disposition(self.spirit_pid(), action, &record, crashed);
        }
    }

    /// A read-only view of one binding, for the corpora and for operators.
    #[derive(Clone)]
    pub struct WorkerBindingView {
        pub spirit_id: String,
        pub spirit_pid: u32,
        pub child_pid: Option<u32>,
        /// Present only while the binding is active. Terminal history retains
        /// the pid but releases the platform handle.
        pub child: Option<ObservedChild>,
        pub phase: BindingPhase,
    }

    #[derive(Default)]
    struct BindingRegistryState {
        active: Vec<Arc<BindingCore>>,
        completed: Vec<WorkerBindingView>,
        unresolved: Vec<(String, String, ContainerResidue, bool)>,
    }

    #[derive(Default)]
    struct BindingRegistry {
        state: Mutex<BindingRegistryState>,
    }

    impl BindingRegistry {
        fn register(&self, core: Arc<BindingCore>) {
            if let Ok(mut state) = self.state.lock() {
                state.active.push(core);
            }
        }

        fn retire(&self, core: &Arc<BindingCore>) {
            // Registry→binding is the shared lock order used by `views` and
            // unresolved reporting; no caller reaches the registry while
            // holding the binding state lock.
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state
                .active
                .retain(|candidate| !Arc::ptr_eq(candidate, core));
            let view = core.retire_view();
            let unresolved = core.unresolved_container();
            state.completed.push(view);
            if let Some(unresolved) = unresolved {
                state.unresolved.push(unresolved);
            }
        }

        fn views(&self) -> Vec<WorkerBindingView> {
            let Ok(state) = self.state.lock() else {
                return Vec::new();
            };
            let mut views = state.completed.clone();
            views.extend(state.active.iter().map(|core| core.view()));
            views.sort_by_key(|view| {
                view.spirit_id
                    .strip_prefix("worker-")
                    .and_then(|suffix| suffix.parse::<u32>().ok())
                    .unwrap_or(1)
            });
            views
        }

        fn active(&self) -> Vec<Arc<BindingCore>> {
            self.state
                .lock()
                .map(|state| state.active.clone())
                .unwrap_or_default()
        }

        fn by_pid(&self, spirit_pid: u32) -> Option<Arc<BindingCore>> {
            self.state
                .lock()
                .ok()?
                .active
                .iter()
                .find(|core| core.spirit_pid() == spirit_pid)
                .cloned()
        }

        fn take_unreported(&self) -> Vec<(String, String, ContainerResidue)> {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            let mut pending = Vec::new();
            for (spirit_id, cid, residue, reported) in &mut state.unresolved {
                if !*reported {
                    *reported = true;
                    pending.push((spirit_id.clone(), cid.clone(), *residue));
                }
            }
            // Claim active residues while the registry lock still excludes
            // retirement, so a concurrent retire cannot enqueue a stale
            // `reported=false` duplicate for this CID.
            for core in &state.active {
                if let Some(residue) = core.take_unreported_container() {
                    pending.push(residue);
                }
            }
            pending
        }
    }

    // ── The RAII guard (D-16-3-Q (6)) ───────────────────────────────────

    /// The Worker binding's RAII handle.
    ///
    /// Its `Drop` performs `abandon`, which is what makes "every return between
    /// bind and finish is dispositioned and unloaded" true BY CONSTRUCTION
    /// rather than by review: a `?` a later edit adds cannot skip it.
    #[must_use = "a WorkerBinding that is dropped without `finish` is abandoned: \
                  the Worker is signalled, its task dispositioned and its SCB unloaded"]
    pub struct WorkerBinding {
        #[cfg(unix)]
        egress: Option<crate::egress::WorkerEgress>,
        core: Arc<BindingCore>,
        armed: bool,
        active_paths: Arc<ActivePathCount>,
        registry: Arc<BindingRegistry>,
    }

    impl WorkerBinding {
        #[cfg(unix)]
        pub fn attach_egress(&mut self, egress: crate::egress::WorkerEgress) {
            self.core.lock().lease_revoke = Some(egress.revoker());
            self.egress = Some(egress);
        }

        #[cfg(unix)]
        pub fn egress(&self) -> Option<&crate::egress::WorkerEgress> {
            self.egress.as_ref()
        }

        /// The shared core — the handle `watch` and the pre-spawn phase check
        /// need.
        pub fn core(&self) -> &Arc<BindingCore> {
            &self.core
        }

        /// The Worker's SCB pid.
        pub fn spirit_pid(&self) -> u32 {
            self.core.spirit_pid()
        }

        /// The Worker's SCB id.
        pub fn spirit_id(&self) -> &str {
            self.core.spirit_id()
        }

        /// Classify from the runtime-client status and retained T3 facts.
        /// This consumes the guard's responsibility: it disarms before any
        /// fallible work so Drop cannot repeat abandon after this path begins.
        /// Returns whether the run may complete: `false` when an intentional
        /// stop was latched, decided under the same lock as the transition.
        pub fn finish(
            mut self,
            bridge_exit: &maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgeExit,
        ) -> bool {
            self.armed = false;
            if let Some((_, verdict)) = bridge_exit.container.as_ref() {
                self.core.note_container_teardown(*verdict);
            }
            let cause = crash_cause_from_exit(&self.core, bridge_exit);
            #[cfg(unix)]
            if let Some(mut egress) = self.egress.take() {
                if let Err(error) = egress.finish(self.core.termination_confirmed()) {
                    eprintln!("maos: Worker egress cleanup failed: {error}");
                }
            }
            let (action, may_complete) = {
                let mut state = self.core.lock();
                let may_complete = state.phase != BindingPhase::PlannedStop;
                let (next, action) = state.phase.finish(cause.as_ref());
                state.phase = next;
                (action, may_complete)
            };
            self.core.execute(action);
            self.registry.retire(&self.core);
            may_complete
        }
    }

    impl Drop for WorkerBinding {
        fn drop(&mut self) {
            self.active_paths.leave();
            if !self.armed {
                return;
            }
            // Never panic out of a drop — this can run during unwinding.
            let action = {
                let mut state = self.core.lock();
                let (next, action) = state.phase.abandon();
                state.phase = next;
                action
            };
            self.core.execute(action);
            // Abandon from any phase (incl. crash-handled) still removes the CID
            // before the lease decision.
            self.core
                .cleanup_container(Instant::now() + TEARDOWN_BUDGET);
            #[cfg(unix)]
            if let Some(mut egress) = self.egress.take() {
                if let Err(error) = egress.finish(self.core.termination_confirmed()) {
                    use std::io::Write as _;
                    let _ = writeln!(
                        std::io::stderr(),
                        "maos: Worker egress cleanup failed: {error}"
                    );
                }
            }
            self.registry.retire(&self.core);
        }
    }

    /// The Worker's live bridge and its binding, in ONE value with the guard
    /// FIRST — struct fields drop in declaration order, and `SpawnedBridge`'s
    /// own `Drop` kills and reaps the child (`runtime.rs:791-800`). With two
    /// separate locals the bridge (declared later) would drop first, so an
    /// early return would kill the child BEFORE the guard ran: the guard's
    /// signal would prove nothing, and the bridge's SIGKILL landing in
    /// `Running` could race the observer into a spurious crash path (measured
    /// 7/200 and 9/200 by validation round 8).
    pub struct LiveWorker {
        pub binding: WorkerBinding,
        pub bridge: maos_kernel_core::lifecycle::cli_wrapper::runtime::SpawnedBridge,
    }

    /// Classify from the bridge's client status and immutable T3 payload facts.
    ///
    /// ADR-022 keeps Spirit-specific interpretation outside the kernel. Keep
    /// this on the SAME `BindingExecutor` as the pidfd observer so native
    /// mapping cannot drift between feeds; T3 adds only the retained snapshot.
    /// The planned-stop latch and typed launch refusals are resolved first.
    fn crash_cause_from_exit(
        core: &BindingCore,
        exit: &maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgeExit,
    ) -> Option<CrashCause> {
        if core.phase() == BindingPhase::PlannedStop {
            return None;
        }
        let facts = match &exit.cause {
            ExitCause::Exited { code } => ExitFacts {
                signal: None,
                code: Some(*code),
            },
            ExitCause::Signaled { signal } => ExitFacts {
                signal: Some(*signal),
                code: None,
            },
            ExitCause::Unknown => ExitFacts {
                // Unknown client status stays native `Truncated`; never invent
                // an exit code from runtime conventions or stderr.
                signal: None,
                code: None,
            },
        };
        if core.is_typed_refusal(&facts) {
            return None;
        }
        let container_facts = exit.container.as_ref().map(|(facts, _)| facts.as_ref());
        match core
            .exec
            .classify_exit_with_container(core.spirit_pid(), facts, container_facts)
        {
            CrashCause::Voluntary => None,
            cause => Some(cause),
        }
    }

    // ── The port (D-16-3-Q (1)) ─────────────────────────────────────────

    /// The port `run_cli_wrapper_manifest` depends on.
    ///
    /// [`WorkerSupervisor`] is the kernel-backed adapter; a test double
    /// implements this so `worker_spawn` vectors run without a kernel.
    pub trait WorkerSupervision: Send + Sync {
        /// Create a fresh, private state directory for one T3 spawn.
        fn create_t3_state_dir(&self) -> Result<PathBuf, String>;

        /// Whether T3 payload pidfd evidence should be requested.
        fn payload_pidfd(&self) -> bool {
            true
        }
        fn bind(
            &self,
            task: &WorkerTask,
            bundle: SpiritManifestBundle,
        ) -> Result<WorkerBinding, WorkerSupervisionError>;

        /// Open the platform child handle, start the exit observer, and execute
        /// the resulting action. Called on the spawning thread immediately
        /// after `spawn_and_bridge` returns — BEFORE the pump starts.
        fn watch(&self, core: &Arc<BindingCore>, child_pid: u32);

        /// Commit the confirmed T3 spawn identity to its owning SCB and journal.
        fn commit_container_spawn(
            &self,
            core: &Arc<BindingCore>,
            child: &maos_kernel_core::security::sandbox::t3::child::SandboxedContainerChild,
            image: &maos_kernel_core::security::sandbox::t3::image_lock::VerifiedImageAttestation,
        ) -> Result<(), WorkerSupervisionError>;

        /// Patch the minted capability token and its expiry into the kept
        /// record and the SCB ledger entry.
        fn bind_token(&self, core: &Arc<BindingCore>, token_id: TokenId, ttl_deadline_ns: u64);

        /// `true` once `stop_workers()` has latched.
        fn is_stopping(&self) -> bool;

        /// Test seam: called between the mint and the pre-spawn phase check.
        fn after_mint(&self) {}

        /// Test seam: AC3's injected `?`, called immediately after
        /// [`Self::watch`] and before the pump. `Err` proves the RAII guard —
        /// not an explicit `abandon` call — is what unloads a bound Worker.
        fn error_after_watch(&self) -> Result<(), String> {
            Ok(())
        }
    }

    // ── The kernel-backed adapter ───────────────────────────────────────

    /// The active-path count the root's signal listener waits on.
    ///
    /// Incremented by `bind` AFTER the `stopping` latch check and decremented
    /// when the guard is consumed or dropped — never on entry to
    /// `run_cli_wrapper_manifest`, whose pre-bind admission (the fixtures' 2 s
    /// host handshake; 19-3 D-19-3-07 A deleted the real-CLI liveness probe)
    /// would, if counted, hold the count during a SIGTERM and trip a false
    /// descendant exit for a Worker that has no binding at all.
    pub struct ActivePathCount {
        tx: tokio::sync::watch::Sender<usize>,
    }

    impl Default for ActivePathCount {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ActivePathCount {
        /// A fresh counter. `pub` so a test double implementing the port can
        /// hold one without reaching into the supervisor.
        pub fn new() -> Self {
            Self {
                tx: tokio::sync::watch::channel(0usize).0,
            }
        }

        fn enter(&self) {
            self.tx.send_modify(|n| *n += 1);
        }

        fn leave(&self) {
            self.tx.send_modify(|n| *n = n.saturating_sub(1));
        }

        /// A receiver for the listener's check-then-wait. A count already at 0
        /// returns at once — never a bare `Notify::notified()`, which would
        /// never fire for an idle butler root and would send it down the grace
        /// path at 5 s.
        pub fn subscribe(&self) -> tokio::sync::watch::Receiver<usize> {
            self.tx.subscribe()
        }
    }

    /// The kernel-backed [`WorkerSupervision`] adapter: the 16-1
    /// sync-port-over-`Handle` shape, so a SYNC `run_cli_wrapper_manifest`
    /// (19-3 AC3 relies on it staying sync) can drive the async kernel.
    ///
    /// Its `block_on` calls are legal only OFF the async context, which is why
    /// paths (a) and (b) call `run_cli_wrapper_manifest` inside
    /// `tokio::task::block_in_place` and path (c) already runs inside
    /// `spawn_blocking`.
    pub struct WorkerSupervisor {
        exec: Arc<KernelBindingExecutor>,
        strategy: Arc<dyn ExitObservation>,
        counters: Arc<SupervisorCounters>,
        active_paths: Arc<ActivePathCount>,
        registry: Arc<BindingRegistry>,
        #[cfg(unix)]
        state_root: Result<PathBuf, String>,
        next_seq: AtomicU32,
        stopping: AtomicBool,
        boot_nonce: u64,
        root_shutdown: tokio_util::sync::CancellationToken,
        seams: SupervisorSeams,
    }

    impl WorkerSupervisor {
        fn prepare_core(
            &self,
            spirit_id: String,
            task: &WorkerTask,
            bundle: &SpiritManifestBundle,
            record_active: bool,
        ) -> Arc<BindingCore> {
            let (task_id, originator_spirit_id) = task.record_ids(&spirit_id);
            Arc::new(BindingCore {
                spirit_id,
                spirit_pid: AtomicU32::new(0),
                state: Mutex::new(BindingState {
                    phase: BindingPhase::Running,
                    child: None,
                    child_pid: None,
                    container_terminal: None,
                    container_launch: None,
                    container_teardown_confirmed: false,
                    container_residue: None,
                    container_residue_reported: false,
                    lease_revoke: None,
                    signal_counted: false,
                    handler: None,
                    record: TaskAssignmentRecord {
                        task_id,
                        capability_token: None,
                        ttl_deadline_ns: u64::MAX,
                        intent_class: IntentClass::Standard,
                        originator_spirit_id,
                    },
                    on_crash_action: bundle
                        .on_crash
                        .as_ref()
                        .map(|section| section.action)
                        .unwrap_or_default(),
                    dispositioned: false,
                    record_active,
                    turn_scoped: !record_active,
                    ready: false,
                }),
                exec: Arc::clone(&self.exec) as Arc<dyn BindingExecutor>,
                strategy: Arc::clone(&self.strategy),
                counters: Arc::clone(&self.counters),
                seams: self.seams.clone(),
            })
        }

        /// Prepare lifecycle ownership without allocating another SCB or PID.
        pub fn prepare_spawned(
            &self,
            spirit_id: &str,
            bundle: &SpiritManifestBundle,
        ) -> Result<
            (
                Arc<BindingCore>,
                Arc<dyn maos_kernel_core::scheduler::control_block::AnySpiritObj>,
            ),
            WorkerSupervisionError,
        > {
            if self.is_stopping() {
                return Err(WorkerSupervisionError::WorkerStopping);
            }
            let core =
                self.prepare_core(spirit_id.to_owned(), &WorkerTask::Standalone, bundle, false);
            let spirit = maos_kernel_core::scheduler::control_block::make_spirit_obj(
                WorkerSpirit::new(Arc::clone(&core)),
            );
            Ok((core, spirit))
        }

        fn adopt_core(&self, core: Arc<BindingCore>) -> WorkerBinding {
            self.active_paths.enter();
            self.registry.register(Arc::clone(&core));
            if self.is_stopping() {
                core.stop_session();
            }
            WorkerBinding {
                #[cfg(unix)]
                egress: None,
                core,
                armed: true,
                active_paths: Arc::clone(&self.active_paths),
                registry: Arc::clone(&self.registry),
            }
        }

        /// Adopt admission's existing SCB; never call scheduler.load here.
        pub fn adopt_spawned(
            &self,
            core: Arc<BindingCore>,
        ) -> Result<WorkerBinding, WorkerSupervisionError> {
            assert_ne!(core.spirit_pid(), 0, "adopt requires successful admission");
            let binding = self.adopt_core(core);
            if self.is_stopping() {
                return Err(WorkerSupervisionError::WorkerStopping);
            }
            Ok(binding)
        }

        /// Only an active mailbox export belongs in the progress watchdog ledger.
        pub fn begin_turn(
            &self,
            core: &Arc<BindingCore>,
            task_id: String,
        ) -> Result<(), WorkerSupervisionError> {
            let (pid, record) = {
                let mut state = core.lock();
                if self.is_stopping() || state.phase != BindingPhase::Running {
                    return Err(WorkerSupervisionError::WorkerStopping);
                }
                assert!(
                    !state.record_active,
                    "a spawned session already has an active turn"
                );
                state.record.task_id = task_id;
                state.record.capability_token = None;
                state.record.ttl_deadline_ns = u64::MAX;
                state.dispositioned = false;
                state.record_active = true;
                (core.spirit_pid(), state.record.clone())
            };
            // The binding lock is not held across the executor. A stop that
            // lands before the push dispositions this (already active) record
            // from the kept copy; the late ledger entry is then retired here.
            self.exec.push_record(pid, record.clone());
            if core.phase() != BindingPhase::Running {
                self.exec.take_record(pid, &record.task_id);
                return Err(WorkerSupervisionError::WorkerStopping);
            }
            Ok(())
        }

        pub fn complete_turn(&self, core: &Arc<BindingCore>) {
            let (pid, task_id) = {
                let mut state = core.lock();
                assert!(state.record_active, "TurnComplete without an active turn");
                state.record_active = false;
                state.record.capability_token = None;
                state.record.ttl_deadline_ns = u64::MAX;
                (core.spirit_pid(), state.record.task_id.clone())
            };
            self.exec.take_record(pid, &task_id);
        }
        /// Build the root's supervisor. Construct it AFTER
        /// `set_crash_detector` — that call needs `Arc::get_mut(&mut scheduler)`
        /// and therefore strong count 1 (Trap 1).
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            scheduler: Arc<SpiritSchedulerAdapter>,
            crash_detector: Arc<CrashDetector>,
            transparency_log: Arc<TransparencyLogAdapter>,
            lifecycle_journal: Arc<JournalAdapter>,
            halt_registry: Arc<HaltRegistry>,
            iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
            handle: tokio::runtime::Handle,
            boot_nonce: u64,
            root_shutdown: tokio_util::sync::CancellationToken,
        ) -> Self {
            let counters = Arc::new(SupervisorCounters::default());
            let strategy =
                exit_observation::choose_strategy(Arc::clone(&counters), transparency_log.as_ref());
            let registry = Arc::new(BindingRegistry::default());
            #[cfg(unix)]
            let state_root = crate::egress::create_worker_state_root()
                .map_err(|error| format!("create private T3 state root: {error}"));
            Self {
                exec: Arc::new(KernelBindingExecutor {
                    scheduler,
                    crash_detector,
                    transparency_log,
                    lifecycle_journal,
                    halt_registry,
                    iac,
                    handle,
                    registry: Arc::downgrade(&registry),
                    outstanding: Mutex::new(Vec::new()),
                    originator_delivery: std::sync::OnceLock::new(),
                }),
                strategy,
                counters,
                active_paths: Arc::new(ActivePathCount::new()),
                registry,
                #[cfg(unix)]
                state_root,
                next_seq: AtomicU32::new(0),
                stopping: AtomicBool::new(false),
                boot_nonce,
                root_shutdown,
                seams: SupervisorSeams::default(),
            }
        }

        /// Install test seams. `#[doc(hidden)]`: a production caller never
        /// needs it, and the seams must not be `#[cfg(test)]` (integration
        /// tests are separate crates) nor env-var driven.
        #[doc(hidden)]
        pub fn with_seams(mut self, seams: SupervisorSeams) -> Self {
            self.seams = seams;
            self
        }

        /// Story 19-6 (D-19-6-01 A / 05 A) — the root's set-once crash wiring.
        /// Once installed, a crashed delegated Worker's originator receives the
        /// kernel-sent [`crate::delegation::orphan_frame`] — iff it is a Spirit
        /// loaded on THIS root (D-19-6-07 A: `operator`, `topology` and host
        /// B's remote `orchestrator` stay journal-only) — and the executor's
        /// `NotLoaded` disposition asks `resolver`, the same `Arc` the root
        /// gave the `CrashDetector` and the scheduler. Uninstalled, both are
        /// as before (journal-only; no resolver). `Err` returns `resolver`.
        pub fn install_originator_delivery(
            &self,
            resolver: Arc<dyn ReplicaResolver>,
        ) -> Result<(), Arc<dyn ReplicaResolver>> {
            self.exec.originator_delivery.set(resolver)
        }

        /// The root's shutdown token — cancelled by the signal listener, read
        /// by the serving `select!`, the topology loop, the `--once` passes and
        /// the cohort daemon's own `select!`.
        pub fn shutdown_token(&self) -> tokio_util::sync::CancellationToken {
            self.root_shutdown.clone()
        }

        /// Supervisor-wide counters (the AC3 `signals_sent` observable).
        pub fn counters(&self) -> &Arc<SupervisorCounters> {
            &self.counters
        }

        /// The active-path count the listener waits on.
        pub fn active_paths(&self) -> &Arc<ActivePathCount> {
            &self.active_paths
        }

        /// The chosen strategy's name — `pidfd` or `after-pump`.
        pub fn strategy_name(&self) -> &'static str {
            self.strategy.name()
        }

        /// Every binding this root ever made, newest last. Completed bindings
        /// retain metadata only; their platform child handles are released.
        pub fn bindings(&self) -> Vec<WorkerBindingView> {
            self.registry.views()
        }

        /// Stop every live Worker under one absolute T3 teardown deadline.
        ///
        /// All active bindings are latched before any CID teardown is dispatched,
        /// and every action shares one absolute deadline. This does NOT unload
        /// SCBs: scheduler unload owns mechanism (x), the receipt and hook.
        pub fn stop_workers(&self) {
            let deadline = Instant::now() + TEARDOWN_BUDGET;
            self.stopping.store(true, Ordering::SeqCst);
            let jobs: Vec<_> = self
                .registry
                .active()
                .into_iter()
                .map(|core| {
                    let action = core.begin_stop();
                    (core, action)
                })
                .collect();
            std::thread::scope(|scope| {
                let run = |core: Arc<BindingCore>, action| {
                    core.execute_until(action, deadline);
                    core.cleanup_container(deadline);
                };
                let mut handles = Vec::new();
                for (core, action) in jobs {
                    // Concurrent dispatch; a failed thread spawn runs inline.
                    let job = std::thread::Builder::new().spawn_scoped(scope, {
                        let core = Arc::clone(&core);
                        let action = action.clone();
                        move || run(core, action)
                    });
                    match job {
                        Ok(handle) => handles.push(handle),
                        Err(_) => run(core, action),
                    }
                }
                for handle in handles {
                    let _ = handle.join();
                }
            });
        }

        /// Stop a registered binding after any scheduler unload attempt.
        pub fn stop_binding(&self, spirit_pid: u32) {
            if let Some(core) = self.registry.by_pid(spirit_pid) {
                core.stop_session();
            }
        }

        /// Story 19-6 harness plant only (`orchestrator_root::CRASH_PLANT_ENV`):
        /// kill the Running Worker bound to `task_id` through the stop path's
        /// signal/teardown but WITHOUT `begin_stop`, so its exit is a crash the
        /// kernel handles. `false` until that Worker has a child or container.
        pub(crate) fn test_only_crash_task(&self, task_id: &str) -> bool {
            let target = self.registry.active().into_iter().find(|core| {
                core.task_id() == task_id
                    && core.phase() == BindingPhase::Running
                    && (core.child_handle().is_some() || core.container_terminal().is_some())
            });
            target
                .map(|core| core.signal(Instant::now() + TEARDOWN_BUDGET))
                .is_some()
        }

        /// Report unresolved CIDs once, on the root's stderr sink.
        pub fn report_unresolved_containers(&self) {
            for (spirit_id, container_id, residue) in self.registry.take_unreported() {
                eprintln!(
                    "maos: worker {spirit_id} unresolved container {container_id} residue={residue:?}"
                );
            }
        }
        fn cleanup_state_root(&self) {
            #[cfg(unix)]
            if let Ok(root) = &self.state_root {
                if let Err(error) = std::fs::remove_dir(root) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        eprintln!(
                            "maos: preserving non-empty T3 state root for residue ({})",
                            error.kind()
                        );
                    }
                }
            }
        }

        /// Join every crash handler parked by a timed-out join.
        pub async fn join_outstanding(&self, budget: Duration) {
            let parked: Vec<CrashHandlerHandle> = self
                .exec
                .outstanding
                .lock()
                .map(|mut o| std::mem::take(&mut *o))
                .unwrap_or_default();
            for handle in parked {
                let _ = tokio::time::timeout(budget, handle.0).await;
            }
        }

        /// Wait until no Worker call path is in flight, bounded by `grace`.
        /// Returns `true` when every path finished.
        pub async fn await_idle(&self, grace: Duration) -> bool {
            let mut rx = self.active_paths.subscribe();
            if *rx.borrow_and_update() == 0 {
                return true;
            }
            let deadline = tokio::time::Instant::now() + grace;
            loop {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return *rx.borrow() == 0;
                }
                if tokio::time::timeout(remaining, rx.changed()).await.is_err() {
                    return *rx.borrow() == 0;
                }
                if *rx.borrow_and_update() == 0 {
                    return true;
                }
            }
        }

        /// The ids of every binding still short of a terminal phase, for the
        /// grace-exit message.
        pub fn unstopped_workers(&self) -> Vec<(String, bool)> {
            self.bindings()
                .into_iter()
                .filter(|view| {
                    matches!(
                        view.phase,
                        BindingPhase::Running | BindingPhase::PlannedStop
                    )
                })
                .map(|view| (view.spirit_id, view.child.is_some()))
                .collect()
        }
    }
    impl Drop for WorkerSupervisor {
        fn drop(&mut self) {
            self.cleanup_state_root();
        }
    }

    impl WorkerSupervision for WorkerSupervisor {
        fn bind(
            &self,
            task: &WorkerTask,
            mut bundle: SpiritManifestBundle,
        ) -> Result<WorkerBinding, WorkerSupervisionError> {
            if self.stopping.load(Ordering::SeqCst) {
                return Err(WorkerSupervisionError::WorkerStopping);
            }
            if let Some(cap) = self.seams.hook_cap_override {
                let budget = bundle.budget.get_or_insert_with(|| {
                    maos_kernel_core::security::manifest::Budget {
                        context_window_size: 0,
                        time_cap_seconds: cap,
                        context_pressure_permille: 800,
                        context_limit_permille: 950,
                    }
                });
                budget.time_cap_seconds = cap;
            }
            // `worker`, so epic 17's `maos audit query --spirit worker`
            // resolves; later ones are `worker-2`, `worker-3`, …
            let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
            let spirit_id = if seq == 0 {
                "worker".to_string()
            } else {
                format!("worker-{}", seq + 1)
            };
            let core = self.prepare_core(spirit_id.clone(), task, &bundle, true);
            let spirit = WorkerSpirit::new(Arc::clone(&core));
            let pid = self
                .exec
                .handle
                .block_on(
                    self.exec
                        .scheduler
                        .load(&spirit_id, bundle, spirit, self.boot_nonce),
                )
                .map_err(|e| WorkerSupervisionError::Load(e.to_string()))?;
            core.bind_scheduler_pid(pid);

            if let Err(e) = self.exec.handle.block_on(self.exec.scheduler.start(pid)) {
                // Story 16-6 — this used to read "the SCB is `Loaded` and
                // there is no `Loaded → Unloaded` transition, so it cannot
                // be unloaded and gets no receipt". The kernel gained that
                // arm, so the Worker's stranded SCB is now recoverable:
                // `unload_all_loaded` reaps it on the way out and it gets
                // its NFR-Rel-11 receipt like any other Spirit. The declared
                // residual on the `epic-16-retrospective` row is CLOSED.
                return Err(WorkerSupervisionError::Start(e.to_string()));
            }

            // The record is pushed AFTER `start`: the ProgressWatchdog only
            // looks at `Running` SCBs with a non-empty ledger.
            self.exec.push_record(pid, core.lock().record.clone());
            Ok(self.adopt_core(core))
        }

        fn watch(&self, core: &Arc<BindingCore>, child_pid: u32) {
            match self.strategy.watch(child_pid, Arc::clone(core)) {
                ObserveOutcome::Watching => {}
                ObserveOutcome::Unavailable(errno) => {
                    self.counters
                        .observer_unavailable
                        .fetch_add(1, Ordering::Relaxed);
                    core.note_child_pid(child_pid);
                    // The SUPERVISOR journals it, never the strategy: the
                    // strategy makes no Transparency Log call. Kind 4 is
                    // already a non-call kind, so this row cannot red FR4.
                    self.exec.journal_event(
                        core.spirit_pid(),
                        &format!("worker.exit-observer-unavailable:{errno}"),
                        serde_json::json!({
                            "event": "worker.exit-observer-unavailable",
                            "spirit_id": core.spirit_id(),
                            "child_pid": child_pid,
                            "errno": errno,
                            "consequence": "exit handled after the pump from the bridge's ExitCause",
                        })
                        .to_string(),
                    );
                }
            }
            if let Some(after_watch) = &self.seams.after_watch {
                after_watch(child_pid);
            }
        }

        fn commit_container_spawn(
            &self,
            core: &Arc<BindingCore>,
            child: &maos_kernel_core::security::sandbox::t3::child::SandboxedContainerChild,
            image: &maos_kernel_core::security::sandbox::t3::image_lock::VerifiedImageAttestation,
        ) -> Result<(), WorkerSupervisionError> {
            maos_kernel_core::security::sandbox::t3::spawn::commit_spawn_report(
                &self.exec.scheduler,
                self.exec.lifecycle_journal.as_ref(),
                child,
                core.spirit_id().to_owned(),
                image,
            )
            .map_err(|error| WorkerSupervisionError::SpawnReport(error.to_string()))
        }

        fn bind_token(&self, core: &Arc<BindingCore>, token_id: TokenId, ttl_deadline_ns: u64) {
            let task_id = {
                let mut state = core.lock();
                state.record.capability_token = Some(token_id);
                state.record.ttl_deadline_ns = ttl_deadline_ns;
                state.record.task_id.clone()
            };
            self.exec
                .patch_ledger_token(core.spirit_pid(), &task_id, token_id, ttl_deadline_ns);
        }

        fn is_stopping(&self) -> bool {
            self.stopping.load(Ordering::SeqCst)
        }

        fn create_t3_state_dir(&self) -> Result<PathBuf, String> {
            #[cfg(unix)]
            {
                let root = self.state_root.as_ref().map_err(Clone::clone)?;
                crate::egress::create_worker_state_dir(root)
                    .map_err(|error| format!("create private T3 state directory: {error}"))
            }
            #[cfg(not(unix))]
            {
                Err("T3 state directories require Linux".to_string())
            }
        }

        fn payload_pidfd(&self) -> bool {
            self.seams.payload_pidfd
        }

        fn after_mint(&self) {
            if let Some(hook) = &self.seams.after_mint {
                hook();
            }
        }

        fn error_after_watch(&self) -> Result<(), String> {
            match &self.seams.error_after_watch {
                Some(reason) => Err(reason.clone()),
                None => Ok(()),
            }
        }
    }

    /// Kernel-backed implementation of binding actions and lifecycle plumbing.
    /// The lifecycle journal is separate from the scheduler adapter because
    /// spawn-report commits require an adapter that preserves the payload.
    struct KernelBindingExecutor {
        scheduler: Arc<SpiritSchedulerAdapter>,
        crash_detector: Arc<CrashDetector>,
        transparency_log: Arc<TransparencyLogAdapter>,
        lifecycle_journal: Arc<JournalAdapter>,
        #[allow(dead_code)]
        halt_registry: Arc<HaltRegistry>,
        iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
        handle: tokio::runtime::Handle,
        registry: Weak<BindingRegistry>,
        outstanding: Mutex<Vec<CrashHandlerHandle>>,
        /// Story 19-6 — set once by [`WorkerSupervisor::install_originator_delivery`].
        originator_delivery: std::sync::OnceLock<Arc<dyn ReplicaResolver>>,
    }

    impl KernelBindingExecutor {
        fn push_record(&self, spirit_pid: u32, record: TaskAssignmentRecord) {
            let scbs = self.scheduler.scbs();
            let guard = match scbs.read() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(scb) = guard.get(&spirit_pid) {
                if let Ok(mut ledger) = scb.task_assignments_in_flight.lock() {
                    // A new active turn starts its own no-progress window.
                    // Healthy mailbox idle must not age the next export.
                    if ledger.is_empty() {
                        scb.last_progress_iac_ns.store(
                            maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                            Ordering::Relaxed,
                        );
                    }
                    ledger.push(record);
                }
            }
        }

        fn with_scb<T>(
            &self,
            spirit_pid: u32,
            f: impl FnOnce(&Arc<maos_kernel_core::scheduler::control_block::SpiritControlBlock>) -> T,
        ) -> Option<T> {
            let scbs = self.scheduler.scbs();
            let guard = match scbs.read() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.get(&spirit_pid).map(f)
        }

        /// The last ≤ 20 lines / 4 KiB of the Worker's journaled `stderr`
        /// rows at its pid. The query inspects at most 256 recent subprocess
        /// rows, so crash classification cannot scan an unbounded log.
        ///
        /// The observer path reads this EARLY (at detection) and the
        /// after-pump fallback reads it LATE, so the two can legitimately
        /// differ for the same death.
        fn stderr_tail(&self, spirit_pid: u32) -> Option<String> {
            let rows = self
                .transparency_log
                .query_frames(FrameFilter {
                    kind: Some(FrameKind::CliSubprocessOutput),
                    spirit_pid: Some(spirit_pid),
                    limit: Some(256),
                    descending: true,
                    ..Default::default()
                })
                .ok()?;
            let mut lines: Vec<String> = Vec::new();
            for row in rows {
                let Ok(value) = serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                else {
                    continue;
                };
                if value.get("stream").and_then(|s| s.as_str()) != Some("stderr") {
                    continue;
                }
                if let Some(line) = value.get("line").and_then(|l| l.as_str()) {
                    lines.push(line.to_string());
                    if lines.len() == 20 {
                        break;
                    }
                }
            }
            if lines.is_empty() {
                return None;
            }
            lines.reverse();
            let mut tail = lines.join("\n");
            if tail.len() > 4096 {
                let cut = tail
                    .char_indices()
                    .rev()
                    .find(|(idx, _)| *idx <= tail.len() - 4096)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                tail = tail[cut..].to_string();
            }
            Some(tail)
        }
    }

    impl BindingExecutor for KernelBindingExecutor {
        fn classify_exit(&self, spirit_pid: u32, facts: ExitFacts) -> CrashCause {
            let stderr_tail = self.stderr_tail(spirit_pid);
            match facts {
                ExitFacts {
                    signal: Some(signal),
                    ..
                } => CrashCause::Fault(FaultCause::SignaledByKernel {
                    signal,
                    stderr_tail,
                }),
                ExitFacts {
                    code: Some(0),
                    signal: None,
                } => CrashCause::Voluntary,
                ExitFacts {
                    code: Some(code),
                    signal: None,
                } => CrashCause::Fault(FaultCause::NonZeroExit { code, stderr_tail }),
                ExitFacts {
                    code: None,
                    signal: None,
                } => CrashCause::Fault(FaultCause::Truncated {
                    reason: "exit status unrecoverable".to_string(),
                }),
            }
        }
        fn classify_exit_with_container(
            &self,
            spirit_pid: u32,
            facts: ExitFacts,
            container: Option<&ContainerTerminalFacts>,
        ) -> CrashCause {
            match container {
                Some(container) => container.classify(self.stderr_tail(spirit_pid)),
                None => self.classify_exit(spirit_pid, facts),
            }
        }

        fn spawn_crash_handler(
            &self,
            spirit_pid: u32,
            cause: CrashCause,
            record: Option<TaskAssignmentRecord>,
        ) -> CrashHandlerHandle {
            let detector = Arc::clone(&self.crash_detector);
            let orphan = record
                .filter(|_| self.originator_delivery.get().is_some())
                .map(|record| (record, Arc::clone(&self.iac), Arc::clone(&self.scheduler)));
            CrashHandlerHandle(self.handle.spawn(async move {
                match detector.handle_crash(spirit_pid, cause).await {
                    Ok(report) => {
                        // Story 19-6 (D-19-6-01 A, CR-2) — the kernel has drained
                        // the record and journaled `task.orphaned` and the
                        // disposition; only then is the originator told. An
                        // empty outcome drained nothing, so there is no orphan.
                        if let Some((record, iac, scheduler)) =
                            orphan.filter(|_| report.disposition_outcome != Default::default())
                        {
                            deliver_orphan(&iac, &scheduler, &record).await;
                        }
                        CrashOutcome::Handled
                    }
                    Err(_) => CrashOutcome::NotLoaded,
                }
            }))
        }

        fn join_crash_handler(
            &self,
            handle: CrashHandlerHandle,
            budget: Duration,
        ) -> Result<CrashOutcome, CrashHandlerHandle> {
            let CrashHandlerHandle(mut join) = handle;
            // `&mut JoinHandle` is a `Future` (it is `Unpin`), so a timeout
            // that elapses LEAVES THE TASK RUNNING and hands the handle back —
            // which is the point: a handler that overruns its budget is parked
            // for `join_outstanding`, never aborted mid-`handle_crash` and
            // never silently dropped.
            let joined = self
                .handle
                .block_on(async { tokio::time::timeout(budget, &mut join).await });
            match joined {
                Ok(Ok(outcome)) => Ok(outcome),
                // A panicked or aborted handler wrote nothing, so the record
                // still needs a disposition.
                Ok(Err(_)) => Ok(CrashOutcome::NotLoaded),
                Err(_elapsed) => Err(CrashHandlerHandle(join)),
            }
        }

        fn run_crash_handler(
            &self,
            spirit_pid: u32,
            cause: CrashCause,
            record: Option<TaskAssignmentRecord>,
            budget: Duration,
        ) -> Option<CrashOutcome> {
            let handle = self.spawn_crash_handler(spirit_pid, cause, record);
            match self.join_crash_handler(handle, budget) {
                Ok(outcome) => Some(outcome),
                Err(handle) => {
                    self.park_outstanding(handle);
                    None
                }
            }
        }

        fn park_outstanding(&self, handle: CrashHandlerHandle) {
            if let Ok(mut outstanding) = self.outstanding.lock() {
                outstanding.push(handle);
            }
        }

        fn disposition(
            &self,
            spirit_pid: u32,
            action: OnCrashAction,
            record: &TaskAssignmentRecord,
            crashed: bool,
        ) {
            self.take_record(spirit_pid, &record.task_id);
            // Story 19-6 (D-19-6-05 A) — the root's resolver, not `None`: the
            // same `Arc` the `CrashDetector` and the scheduler hold.
            let replica = self.originator_delivery.get();
            maos_kernel_core::supervision::enforce_disposition(
                action,
                std::slice::from_ref(record),
                &self.iac,
                replica.map(|replica| &**replica),
            );
            // Story 19-6 (D-19-6-01 A) — a concurrent unload removed the SCB
            // first, so `handle_crash` journaled no `task.orphaned`; the
            // disposition row above is the record and the originator is
            // still told. Never on a planned stop or a clean exit.
            if crashed && replica.is_some() {
                self.handle
                    .block_on(deliver_orphan(&self.iac, &self.scheduler, record));
            }
        }

        fn take_record(&self, spirit_pid: u32, task_id: &str) {
            self.with_scb(spirit_pid, |scb| {
                if let Ok(mut ledger) = scb.task_assignments_in_flight.lock() {
                    ledger.retain(|entry| entry.task_id != task_id);
                }
            });
        }

        fn unload(&self, spirit_pid: u32) {
            let result = self.handle.block_on(self.scheduler.unload(spirit_pid));
            if let Some(core) = self.registry.upgrade().and_then(|r| r.by_pid(spirit_pid)) {
                core.stop_session();
            }
            if let Err(error) = result {
                eprintln!("maos run: worker unload failed for pid {spirit_pid}: {error}");
            }
        }

        fn journal_event(&self, spirit_pid: u32, intent: &str, payload: String) {
            let _ = self.transparency_log.insert_frame_event(
                FrameKind::TelemetryEvent,
                spirit_pid,
                None,
                intent,
                payload.as_bytes(),
                maos_domain::invariants::i3::FrameOrigin::Kernel,
            );
        }

        fn patch_ledger_token(
            &self,
            spirit_pid: u32,
            task_id: &str,
            token_id: TokenId,
            ttl_deadline_ns: u64,
        ) {
            self.with_scb(spirit_pid, |scb| {
                if let Ok(mut ledger) = scb.task_assignments_in_flight.lock() {
                    for entry in ledger.iter_mut() {
                        if entry.task_id == task_id {
                            entry.capability_token = Some(token_id);
                            entry.ttl_deadline_ns = ttl_deadline_ns;
                        }
                    }
                }
            });
        }
    }

    /// Story 19-6 (D-19-6-01/02/07 A) — FR12's delivery clause. Delivers the
    /// kernel-sent orphan for a `delegation:` record through `deliver_typed`
    /// (journaled before delivery, I2) iff the originator is a Spirit loaded on
    /// this root; a repeat is the typed `Duplicate` and delivers nothing.
    async fn deliver_orphan(
        iac: &maos_kernel_core::iac::IacBusAdapter,
        scheduler: &SpiritSchedulerAdapter,
        record: &TaskAssignmentRecord,
    ) {
        let local = scheduler
            .resolve_pid(&record.originator_spirit_id)
            .is_some();
        let Some(frame_id) = delegation_frame_id(&record.task_id).filter(|_| local) else {
            return;
        };
        let frame = crate::delegation::orphan_frame(frame_id, &record.originator_spirit_id);
        if let Err(error) = iac.deliver_typed(frame, 0, None).await {
            let originator = &record.originator_spirit_id;
            eprintln!("maos run: orphan delivery to {originator} failed: {error}");
        }
    }

    /// The inverse of [`WorkerTask::record_ids`] for a delegation: the frame id
    /// behind `delegation:xxxxxxxx:xxxxxxxx:xxxxxxxx:xxxxxxxx`. `None` for any
    /// other task id (standalone and topology runs have no delegation frame).
    fn delegation_frame_id(task_id: &str) -> Option<[u8; 16]> {
        let hex: String = task_id.strip_prefix("delegation:")?.split(':').collect();
        let mut out = [0u8; 16];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(hex.get(2 * index..2 * index + 2)?, 16).ok()?;
        }
        (hex.len() == 32).then_some(out)
    }

    // ── The Worker's Spirit (the door's reach into a Worker) ────────────

    /// The `Spirit` a Worker's SCB holds.
    ///
    /// Its ONLY job is `on_unload`: `maosctl unload worker` (and every
    /// `unload_all_loaded` at teardown) must actually END the child, not just
    /// flip an SCB state. Every other hook keeps the trait's default no-op — a
    /// subprocess Worker has no in-process behaviour to run.
    pub struct WorkerSpirit {
        core: Arc<BindingCore>,
    }

    impl WorkerSpirit {
        fn new(core: Arc<BindingCore>) -> Self {
            Self { core }
        }
    }

    impl Spirit for WorkerSpirit {
        /// Stop the runtime-client pidfd and owned CID. `on_unload` runs under
        /// a hook budget and never recurses into `unload`.
        fn on_unload(&self, _ctx: &mut Ctx) {
            match &self.core.seams.on_unload_fault {
                Some(OnUnloadFault::Panic) => panic!("injected Worker on_unload panic"),
                Some(OnUnloadFault::Sleep(duration)) => std::thread::sleep(*duration),
                None => {}
            }
            self.core.stop_session();
        }
    }

    // ── The root's supervision (D-16-3-G, D-16-3-H, D-16-3-M) ──────────

    /// How long a root's signal listener waits for in-flight Worker paths
    /// before it gives up, unloads, says exactly what it is losing, and exits.
    ///
    /// The only thing that can hold a path past a stop is a descendant still
    /// holding a stopped Worker's pipe (the bridge's `Drop` joins its reader
    /// threads, and an inherited pipe blocks that join forever). Ending the
    /// process tree is 17-1's `spawn_t3` container; until then this bound is
    /// the honest alternative to hanging.
    pub const ROOT_GRACE: Duration = Duration::from_secs(5);

    /// How often the progress stamper looks for new Worker output.
    pub const STAMPER_TICK: Duration = Duration::from_secs(1);

    /// The stderr handshake a test waits for before sending a signal.
    ///
    /// tokio's signal delivery is a `watch` broadcast: a stream created AFTER
    /// a signal arrives never sees it, and a registration never restores the
    /// default disposition. So the streams are created SYNCHRONOUSLY at the
    /// site and this line is printed only afterwards — a test that signals
    /// before seeing it is racing the registration, not testing the teardown.
    pub const ARMED_MARKER: &str = "maos: root supervision armed";

    /// Everything one `maos run` root's supervision owns.
    ///
    /// Declared as an `Option<RootSupervision>` in `main`'s own scope BEFORE
    /// the run block and filled inside it, for two measured reasons:
    ///
    /// * Spawning it before the run block would give every `MAOS_ONE_SHOT` arm
    ///   a SIGTERM listener that swallows the signal the arm is supposed to die
    ///   from.
    /// * Holding its tasks in run-block LOCALS would detach them when the block
    ///   ends: the serving root would run two ProgressWatchdogs, lose
    ///   `root_shutdown`, and its audit-writer drain would time out on the
    ///   detached owners.
    pub struct RootSupervision {
        root_cancel: tokio_util::sync::CancellationToken,
        root_shutdown: tokio_util::sync::CancellationToken,
        supervisor: Arc<WorkerSupervisor>,
        #[cfg(unix)]
        egress: Option<(
            crate::egress::EgressHandle,
            tokio::task::JoinHandle<std::io::Result<()>>,
        )>,
        watchdog: tokio::task::JoinHandle<()>,
        stamper: tokio::task::JoinHandle<()>,
        listener: tokio::task::JoinHandle<()>,
    }

    impl RootSupervision {
        /// Arm the root: build the supervisor, spawn the ProgressWatchdog, the
        /// progress stamper and the signal listener, and print [`ARMED_MARKER`].
        ///
        /// MUST be called after `set_crash_detector` (`Arc::get_mut` needs the
        /// scheduler's strong count at 1) and inside the run block.
        #[allow(clippy::too_many_arguments)]
        pub fn arm(
            scheduler: Arc<SpiritSchedulerAdapter>,
            crash_detector: Arc<CrashDetector>,
            transparency_log: Arc<TransparencyLogAdapter>,
            lifecycle_journal: Arc<JournalAdapter>,
            halt_registry: Arc<HaltRegistry>,
            iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
            telemetry: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
            notification_dispatcher: Arc<
                maos_director_surface::notification::NotificationDispatcher,
            >,
            boot_nonce: u64,
        ) -> Self {
            Self::arm_with_seams(
                scheduler,
                crash_detector,
                transparency_log,
                lifecycle_journal,
                halt_registry,
                iac,
                telemetry,
                notification_dispatcher,
                boot_nonce,
                SupervisorSeams::default(),
            )
        }

        /// Arm the root with explicit integration-test seams.
        #[doc(hidden)]
        #[allow(clippy::too_many_arguments)]
        pub fn arm_with_seams(
            scheduler: Arc<SpiritSchedulerAdapter>,
            crash_detector: Arc<CrashDetector>,
            transparency_log: Arc<TransparencyLogAdapter>,
            lifecycle_journal: Arc<JournalAdapter>,
            halt_registry: Arc<HaltRegistry>,
            iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
            telemetry: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
            notification_dispatcher: Arc<
                maos_director_surface::notification::NotificationDispatcher,
            >,
            boot_nonce: u64,
            seams: SupervisorSeams,
        ) -> Self {
            let root_cancel = tokio_util::sync::CancellationToken::new();
            let root_shutdown = tokio_util::sync::CancellationToken::new();
            #[cfg(unix)]
            let egress = match maos_domain::operator_door::maos_home() {
                Ok(Some(run_dir)) => match crate::egress::EgressHandle::start(
                    run_dir,
                    Arc::clone(&transparency_log),
                    root_cancel.clone(),
                ) {
                    Ok(egress) => Some(egress),
                    Err(error) => {
                        eprintln!("maos: egress proxy unavailable: {error}");
                        None
                    }
                },
                Ok(None) => None,
                Err(error) => {
                    eprintln!("maos: egress proxy unavailable: {error}");
                    None
                }
            };
            let supervisor = Arc::new(
                WorkerSupervisor::new(
                    Arc::clone(&scheduler),
                    crash_detector,
                    Arc::clone(&transparency_log),
                    lifecycle_journal,
                    Arc::clone(&halt_registry),
                    iac,
                    tokio::runtime::Handle::current(),
                    boot_nonce,
                    root_shutdown.clone(),
                )
                .with_seams(seams),
            );

            // The ProgressWatchdog must be RUNNING WHILE WORKERS LIVE. At
            // `af96c907` it was spawned only in the serving tail, i.e. after
            // every Worker-bearing root had already finished or returned, so
            // nothing could ever emit `task.stalled` for a Worker.
            let watchdog = Arc::new(maos_kernel_core::supervision::ProgressWatchdog::new(
                scheduler.scbs(),
                Arc::clone(&transparency_log),
                telemetry,
                notification_dispatcher,
            ))
            .spawn(root_cancel.clone());

            let stamper = spawn_progress_stamper(
                Arc::clone(&transparency_log),
                scheduler.scbs(),
                root_cancel.clone(),
            );

            let listener = spawn_signal_listener(
                Arc::clone(&supervisor),
                Arc::clone(&scheduler),
                Arc::clone(&halt_registry),
                root_cancel.clone(),
                root_shutdown.clone(),
            );

            Self {
                root_cancel,
                root_shutdown,
                supervisor,
                #[cfg(unix)]
                egress,
                watchdog,
                stamper,
                listener,
            }
        }

        /// The root's Worker supervisor.
        pub fn supervisor(&self) -> &Arc<WorkerSupervisor> {
            &self.supervisor
        }

        #[cfg(unix)]
        pub fn egress_handle(&self) -> Option<&crate::egress::EgressHandle> {
            self.egress.as_ref().map(|(handle, _)| handle)
        }

        /// Cancelled by the signal listener on the first SIGTERM/SIGINT. Every
        /// root's own loop observes THIS, never a second signal — tokio never
        /// restores the default disposition, so a second SIGTERM after the
        /// listener is joined is swallowed.
        pub fn shutdown_token(&self) -> tokio_util::sync::CancellationToken {
            self.root_shutdown.clone()
        }

        /// Stop Workers, cancel and JOIN every task this root spawned, then
        /// join any parked crash handler.
        ///
        /// Called in the ruled teardown order: after the door shutdown and
        /// `drain_started_tasks`, and BEFORE `unload_all_loaded` — an unload
        /// moved earlier would race an in-flight `on_idle` that can still raise
        /// a halt after `terminate_spirit` drained.
        pub async fn stop_and_join(self) {
            self.supervisor.stop_workers();
            let _ = self.supervisor.await_idle(ROOT_GRACE).await;
            self.root_cancel.cancel();
            for (name, handle) in [
                ("progress watchdog", self.watchdog),
                ("worker progress stamper", self.stamper),
                ("root signal listener", self.listener),
            ] {
                if let Err(error) = handle.await {
                    eprintln!("maos: {name} join failed: {error}");
                }
            }
            #[cfg(unix)]
            if let Some((_, task)) = self.egress {
                match task.await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => eprintln!("maos: egress proxy cleanup failed: {error}"),
                    Err(error) => eprintln!("maos: egress proxy task join failed: {error}"),
                }
            }
            self.supervisor.join_outstanding(ROOT_GRACE).await;
            self.supervisor.report_unresolved_containers();
            self.supervisor.cleanup_state_root();
        }
    }

    /// The Worker progress source (D-16-3-G).
    ///
    /// `last_progress_iac_ns` has no production writer: the only writers are
    /// `ScbTracker::update_last_progress_iac` on spirit-origin mailbox delivery
    /// and a smoke arm, and Workers send no IAC. Without this task every
    /// Worker's stamp stays at SCB creation and a Worker that is merely slow
    /// but still printing false-stalls.
    ///
    /// **The cursor is insertion-ordered and exclusive.** Wall-clock
    /// timestamps can move backwards under NTP, so `(timestamp_ns, frame_id)`
    /// can permanently skip a later insert. SQLite `rowid` is monotonic for
    /// this append-only table and returns each output row exactly once.
    fn spawn_progress_stamper(
        transparency_log: Arc<TransparencyLogAdapter>,
        spirits: Arc<
            std::sync::RwLock<
                std::collections::BTreeMap<
                    u32,
                    Arc<maos_kernel_core::scheduler::control_block::SpiritControlBlock>,
                >,
            >,
        >,
        cancel: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut cursor: Option<i64> = None;
            let mut ticker = tokio::time::interval(STAMPER_TICK);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => {}
                }
                let rows = match transparency_log.query_frames(FrameFilter {
                    kind: Some(FrameKind::CliSubprocessOutput),
                    cursor_insertion_id: cursor,
                    order_by_insertion: true,
                    ..Default::default()
                }) {
                    Ok(rows) => rows,
                    Err(error) => {
                        eprintln!("maos: worker progress stamper query failed: {error}");
                        continue;
                    }
                };
                if rows.is_empty() {
                    continue;
                }
                // Both streams count as progress: a Worker printing to stderr
                // is a Worker doing something.
                let mut progressed: std::collections::BTreeSet<u32> =
                    std::collections::BTreeSet::new();
                for row in &rows {
                    progressed.insert(row.spirit_pid);
                    cursor = Some(
                        cursor.map_or(row.insertion_id, |current| current.max(row.insertion_id)),
                    );
                }
                let now_ns = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
                let guard = match spirits.read() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                for pid in progressed {
                    if let Some(scb) = guard.get(&pid) {
                        scb.last_progress_iac_ns.store(now_ns, Ordering::Relaxed);
                    }
                }
            }
        })
    }

    /// The root's signal listener (D-16-3-M).
    ///
    /// On the first SIGTERM/SIGINT it cancels `root_shutdown` and stops
    /// Workers — **on its normal path it never unloads**, because unloading
    /// here would run before the door shutdown and could unload a Spirit in the
    /// middle of a topology load. It then waits, check-then-wait on the
    /// supervisor's active-path count, for [`ROOT_GRACE`]. Only on timeout — a
    /// descendant still holding a stopped Worker's pipe — does it unload
    /// itself, say exactly what the exit is losing, and exit 1.
    fn spawn_signal_listener(
        supervisor: Arc<WorkerSupervisor>,
        scheduler: Arc<SpiritSchedulerAdapter>,
        halt_registry: Arc<HaltRegistry>,
        root_cancel: tokio_util::sync::CancellationToken,
        root_shutdown: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        // Created SYNCHRONOUSLY, before the task is spawned and before
        // `ARMED_MARKER` is printed.
        let mut signals = RootSignals::install();
        eprintln!("{ARMED_MARKER}");
        tokio::spawn(async move {
            tokio::select! {
                _ = root_cancel.cancelled() => return,
                _ = signals.recv() => {}
            }
            root_shutdown.cancel();
            supervisor.stop_workers();
            let idle = tokio::select! {
                // If the teardown cancels during the wait, the teardown owns
                // the exit and this listener simply returns.
                _ = root_cancel.cancelled() => return,
                idle = supervisor.await_idle(ROOT_GRACE) => idle,
            };
            if idle {
                return;
            }
            // A descendant of a stopped Worker still holds its pipe, so the
            // bridge's reader-thread join will never finish and the normal
            // teardown can never run. Write the receipts straight to the
            // Transparency Log and exit, naming the loss.
            let stop_after_unload = |pid| supervisor.stop_binding(pid);
            let report =
                super::unload_all_loaded(&scheduler, &halt_registry, Some(&stop_after_unload))
                    .await;
            report.render(&mut std::io::stderr());
            for (spirit_id, has_child) in supervisor.unstopped_workers() {
                if has_child {
                    eprintln!(
                        "maos: worker {spirit_id} stopped but its output is still held open by a \
                         descendant; exiting without the door shutdown and the audit-channel \
                         drain (process-tree teardown: 17-1)"
                    );
                } else {
                    eprintln!(
                        "maos: worker {spirit_id} could not be signalled (no pidfd); exiting \
                         without the door shutdown and the audit-channel drain"
                    );
                }
            }
            supervisor.report_unresolved_containers();
            // Stated loss: `cap.revoke` channel rows, and the completion rows
            // of door commands still running.
            std::process::exit(1);
        })
    }

    /// The root's signal streams. The ONLY `cfg(unix)`/`cfg(not(unix))` pair in
    /// this story — every pidfd gate lives in the strategy module instead.
    struct RootSignals {
        #[cfg(unix)]
        term: tokio::signal::unix::Signal,
        #[cfg(unix)]
        interrupt: tokio::signal::unix::Signal,
        #[cfg(not(unix))]
        ctrl_c: tokio::signal::windows::CtrlC,
    }

    impl RootSignals {
        #[cfg(unix)]
        fn install() -> Self {
            use tokio::signal::unix::{signal, SignalKind};
            Self {
                term: signal(SignalKind::terminate())
                    .expect("install SIGTERM stream for root supervision"),
                interrupt: signal(SignalKind::interrupt())
                    .expect("install SIGINT stream for root supervision"),
            }
        }

        /// The synchronous constructor matters on Windows too: the async
        /// `tokio::signal::ctrl_c()` registers only when first polled, i.e.
        /// AFTER the armed marker, which would reintroduce the very race the
        /// marker exists to close.
        #[cfg(not(unix))]
        fn install() -> Self {
            Self {
                ctrl_c: tokio::signal::windows::ctrl_c()
                    .expect("install Ctrl-C handler for root supervision"),
            }
        }

        #[cfg(unix)]
        async fn recv(&mut self) {
            tokio::select! {
                _ = self.term.recv() => {}
                _ = self.interrupt.recv() => {}
            }
        }

        #[cfg(not(unix))]
        async fn recv(&mut self) {
            self.ctrl_c.recv().await;
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Exit observation strategy (the ONLY place pidfd and `rustix::process` appear)
// ────────────────────────────────────────────────────────────────────────────

/// The Strategy pattern's module — and the platform firebreak.
///
/// **Every** `rustix::process` call, the pidfd itself, and every
/// `#[cfg(target_os = "linux")]` in this story live here. macOS is a shipped
/// release target (`release.yml` builds `aarch64-apple-darwin`) and rustix has
/// no pidfd there, so a `cfg(unix)` slip anywhere in this module would break
/// the release — and no per-PR job builds macOS, which makes the gate below the
/// only thing standing between that slip and a red tag.
#[cfg(feature = "network")]
pub mod exit_observation {
    use std::sync::Arc;

    use super::worker::{BindingCore, SupervisorCounters};

    /// The platform facts an exit observation can produce — and nothing else.
    /// The observer knows the platform; the core decides what a death means.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ExitFacts {
        /// The terminating signal, if the child died by one.
        pub signal: Option<i32>,
        /// The exit code, if it exited normally.
        pub code: Option<i32>,
    }

    /// Whether an observation was started for this binding.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ObserveOutcome {
        /// An observer thread is running.
        Watching,
        /// No observer for this binding, with the `errno` that explains it.
        /// NOT an error: the exit is handled after the pump from the bridge's
        /// own `ExitCause`.
        Unavailable(i32),
    }

    /// What a stop's signal actually did.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SignalOutcome {
        /// `pidfd_send_signal` returned `Ok` — the child was signalled.
        Signalled,
        /// `ESRCH`: the child was already reaped.
        AlreadyReaped,
        /// No child handle (no pidfd) — the stated fallback cost.
        NotSignalled,
    }

    /// The Strategy: how a Worker's exit is observed, and how a stop reaches
    /// its child. Two implementations, chosen ONCE per root.
    pub trait ExitObservation: Send + Sync {
        /// Open the platform child handle, store it in the core, and start the
        /// observer.
        fn watch(&self, child_pid: u32, core: Arc<BindingCore>) -> ObserveOutcome;
        /// Signal the stored child. The one pidfd-using ACT, kept in the
        /// strategy so no pidfd call escapes this module.
        fn signal_kill(&self, core: &BindingCore) -> SignalOutcome;
        /// `pidfd` or `after-pump`.
        fn name(&self) -> &'static str;
    }

    // ── Linux: the pidfd strategy ───────────────────────────────────────

    #[cfg(target_os = "linux")]
    mod linux {
        use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
        use std::sync::Arc;
        use std::time::Instant;

        use rustix::process::{
            pidfd_open, pidfd_send_signal, waitid, Pid, PidfdFlags, Signal, WaitId, WaitIdOptions,
        };

        use super::super::worker::{BindingCore, SupervisorCounters};
        use super::{ExitFacts, ExitObservation, ObserveOutcome, SignalOutcome};
        use maos_kernel_core::lifecycle::cli_wrapper::TEARDOWN_BUDGET;

        /// A pidfd on one Worker's direct child.
        ///
        /// `Clone` over an `Arc<OwnedFd>` so `bindings()` can hand a view out
        /// to the corpora without duplicating the descriptor and without
        /// leaking an `OwnedFd` past this module.
        #[derive(Debug, Clone)]
        pub struct ObservedChild {
            pidfd: Arc<OwnedFd>,
        }

        impl ObservedChild {
            /// The pidfd, for a corpus that must `poll`/`waitid` the child
            /// itself (the harness's "child gone" oracle).
            pub fn pidfd(&self) -> BorrowedFd<'_> {
                self.pidfd.as_fd()
            }
        }

        /// Opens a pidfd and blocks a dedicated thread in
        /// `waitid(EXITED|NOWAIT)` — the exit observation that does not depend
        /// on pipe EOF.
        pub struct PidfdExitObservation {
            counters: Arc<SupervisorCounters>,
        }

        impl PidfdExitObservation {
            pub(super) fn new(counters: Arc<SupervisorCounters>) -> Self {
                Self { counters }
            }

            /// The root's one-time capability probe: can this process open a
            /// pidfd at all? `ENOSYS` (pre-5.3 kernel) and `EPERM` (seccomp)
            /// both mean the whole root falls back.
            pub(super) fn probe() -> Result<(), i32> {
                let me = Pid::from_raw(std::process::id() as i32).ok_or(libc_errno_inval())?;
                match pidfd_open(me, PidfdFlags::empty()) {
                    Ok(_fd) => Ok(()),
                    Err(errno) => Err(errno.raw_os_error()),
                }
            }
        }

        fn libc_errno_inval() -> i32 {
            rustix::io::Errno::INVAL.raw_os_error()
        }

        impl ExitObservation for PidfdExitObservation {
            fn watch(&self, child_pid: u32, core: Arc<BindingCore>) -> ObserveOutcome {
                let Some(pid) = Pid::from_raw(child_pid as i32) else {
                    return ObserveOutcome::Unavailable(libc_errno_inval());
                };
                // A per-Worker failure AFTER a good root probe (`EMFILE`,
                // `ENOMEM`) is not an error: this binding runs handle-less and
                // its exit is handled after the pump. Nothing unwraps.
                let pidfd = match pidfd_open(pid, PidfdFlags::empty()) {
                    Ok(fd) => Arc::new(fd),
                    Err(errno) => return ObserveOutcome::Unavailable(errno.raw_os_error()),
                };
                let child = ObservedChild {
                    pidfd: Arc::clone(&pidfd),
                };
                // Store the handle BEFORE the observer starts: this is the
                // window a stop that landed during the mint is signalled in.
                match core.store_child(child_pid, child) {
                    Ok(action) => {
                        if matches!(action, super::super::worker::BindingAction::Signal) {
                            core.signal(Instant::now() + TEARDOWN_BUDGET);
                        }
                    }
                    Err(refusal) => {
                        // Unreachable by construction (the pidfd is stored
                        // before the observer and before the pump), and a
                        // refusal is data, never a panic.
                        return ObserveOutcome::Unavailable(refusal_errno(refusal));
                    }
                }
                let counters = Arc::clone(&self.counters);
                let observed = Arc::clone(&core);
                let gate = core.observer_gate();
                let thread = std::thread::Builder::new()
                    .name(format!("maos-worker-exit-{child_pid}"))
                    .spawn(move || {
                        if let Some(gate) = gate {
                            gate.wait();
                        }
                        observe_exit(&pidfd, &observed, &counters);
                    });
                match thread {
                    Ok(_join) => ObserveOutcome::Watching,
                    // The thread would not start, but the pidfd is KEPT: a stop
                    // can still signal the child, and the exit is handled after
                    // the pump.
                    Err(error) => ObserveOutcome::Unavailable(
                        error.raw_os_error().unwrap_or_else(libc_errno_inval),
                    ),
                }
            }

            fn signal_kill(&self, core: &BindingCore) -> SignalOutcome {
                let Some(child) = core.child_handle() else {
                    return SignalOutcome::NotSignalled;
                };
                // Never a raw-pid `kill`: after the reap the pid can be
                // reused, and a reused-pid SIGKILL kills a stranger.
                match pidfd_send_signal(child.pidfd(), Signal::KILL) {
                    Ok(()) => SignalOutcome::Signalled,
                    Err(errno) if errno == rustix::io::Errno::SRCH => SignalOutcome::AlreadyReaped,
                    Err(_) => SignalOutcome::NotSignalled,
                }
            }

            fn name(&self) -> &'static str {
                "pidfd"
            }
        }

        fn refusal_errno(_refusal: super::super::worker::PhaseRefusal) -> i32 {
            libc_errno_inval()
        }

        /// The observer thread body. It never touches the kernel: it produces
        /// [`ExitFacts`] and publishes them to the core.
        fn observe_exit(
            pidfd: &OwnedFd,
            core: &Arc<BindingCore>,
            counters: &Arc<SupervisorCounters>,
        ) {
            loop {
                match waitid(
                    WaitId::PidFd(pidfd.as_fd()),
                    WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
                ) {
                    Ok(Some(status)) => {
                        let facts = ExitFacts {
                            signal: status.terminating_signal().map(|s| s as i32),
                            code: status.exit_status().map(|c| c as i32),
                        };
                        core.on_child_exit(facts);
                        return;
                    }
                    // No status without `NOHANG` — nothing to do but retry.
                    Ok(None) => continue,
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(errno) => {
                        if errno == rustix::io::Errno::CHILD {
                            // The bridge reaped first (measured 106/200 on fast
                            // exits). Publish NOTHING: `finish` handles the exit
                            // from the bridge's own `ExitCause`. This is the
                            // normal path for a fast-exiting crash, not a corner
                            // case.
                            counters.note_echild();
                        }
                        return;
                    }
                }
            }
        }
    }

    // ── Everything else: no observer ────────────────────────────────────

    #[cfg(not(target_os = "linux"))]
    mod fallback {
        /// The non-Linux child handle: no pidfd exists, so there is nothing to
        /// hold. `Clone` and empty so `bindings()` compiles identically on
        /// every platform and no `cfg` leaks out of this module.
        #[derive(Debug, Clone)]
        pub struct ObservedChild;

        impl ObservedChild {
            /// There is no pidfd on this platform.
            pub fn is_pidfd(&self) -> bool {
                false
            }
        }
    }

    /// No observer: the exit is handled after the pump, from the bridge's own
    /// `ExitCause`. Used on every non-Linux platform, and on Linux when the
    /// root's `pidfd_open` probe fails.
    pub struct AfterPumpExitObservation;

    impl ExitObservation for AfterPumpExitObservation {
        fn watch(&self, child_pid: u32, core: Arc<BindingCore>) -> ObserveOutcome {
            core.note_child_pid(child_pid);
            ObserveOutcome::Unavailable(0)
        }

        fn signal_kill(&self, _core: &BindingCore) -> SignalOutcome {
            SignalOutcome::NotSignalled
        }

        fn name(&self) -> &'static str {
            "after-pump"
        }
    }

    #[cfg(target_os = "linux")]
    pub use linux::{ObservedChild, PidfdExitObservation};

    #[cfg(not(target_os = "linux"))]
    pub use fallback::ObservedChild;

    /// Choose the root's strategy ONCE, and journal the choice when it is the
    /// fallback.
    pub(super) fn choose_strategy(
        counters: Arc<SupervisorCounters>,
        transparency_log: &maos_kernel_core::iac::transparency_log::TransparencyLogAdapter,
    ) -> Arc<dyn ExitObservation> {
        #[cfg(target_os = "linux")]
        {
            match PidfdExitObservation::probe() {
                Ok(()) => return Arc::new(PidfdExitObservation::new(counters)),
                Err(errno) => {
                    let _ = transparency_log.insert_frame_event(
                        maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent,
                        0,
                        None,
                        &format!("worker.exit-observer-unavailable:{errno}"),
                        serde_json::json!({
                            "event": "worker.exit-observer-unavailable",
                            "scope": "root",
                            "errno": errno,
                            "consequence": "worker exits are observed after the pump; a stop cannot signal the child",
                        })
                        .to_string()
                        .as_bytes(),
                        maos_domain::invariants::i3::FrameOrigin::Kernel,
                    );
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (counters, transparency_log);
        }
        Arc::new(AfterPumpExitObservation)
    }
}

#[cfg(feature = "network")]
pub use worker::{
    ActivePathCount, BindingAction, BindingCore, BindingExecutor, BindingPhase, CrashHandlerHandle,
    CrashOutcome, LiveWorker, ObserverGate, OnUnloadFault, PhaseRefusal, RootSupervision,
    SupervisorCounters, SupervisorSeams, WorkerBinding, WorkerBindingView, WorkerSpirit,
    WorkerSupervision, WorkerSupervisionError, WorkerSupervisor, WorkerTask, ARMED_MARKER,
    CRASH_HANDLER_JOIN_BUDGET, DEFAULT_PROGRESS_THRESHOLD_MS, ROOT_GRACE, STAMPER_TICK,
};
