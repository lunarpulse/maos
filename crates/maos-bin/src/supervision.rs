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
        if let Err(error) = scheduler.unload(pid).await {
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

// ────────────────────────────────────────────────────────────────────────────
// Worker supervision (network-gated)
// ────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "network")]
mod worker {
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use maos_domain::invariants::i1::{IntentClass, TokenId};
    use maos_domain::ports::task::TaskAssignmentRecord;
    use maos_domain::supervision::{CrashCause, FaultCause, OnCrashAction};
    use maos_kernel_core::halt::HaltRegistry;
    use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
    use maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause;
    use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
    use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
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
        /// Signal the child through the strategy (`pidfd_send_signal` on Linux).
        Signal,
        /// In-lock exception: `Handle::spawn(handle_crash)`, storing the handle
        /// so no reader ever sees `CrashHandled` without one.
        SpawnCrashHandler(CrashCause),
        /// `block_on(handle_crash)` bounded, then disposition only if the
        /// handler reported `NotLoaded`.
        RunCrashHandlerThenMaybeDisposition(CrashCause),
        /// Join the stored handler bounded, then disposition only if it
        /// reported `NotLoaded`.
        JoinHandlerThenMaybeDisposition,
        /// As above, then `unload`.
        JoinHandlerThenMaybeDispositionThenUnload,
        /// Disposition from the kept record, then `unload`.
        DispositionAndUnload,
        /// Signal through the strategy WHILE THE BRIDGE IS STILL ALIVE, then
        /// disposition and unload. `abandon(Running)` uses this: the bridge's
        /// own `Drop` kills and reaps the child, so a guard that only
        /// dispositioned would prove nothing about the stop.
        SignalThenDispositionAndUnload,
        /// Take the record out of the ledger WITHOUT disposition (the verdict
        /// row is the record), then `unload`.
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

        /// A stop was requested. `has_child` is whether a child handle is
        /// stored — without one (no pidfd) there is nothing to signal, which is
        /// the stated fallback cost.
        pub fn begin_stop(self, has_child: bool) -> (Self, BindingAction) {
            match self {
                BindingPhase::Running => (
                    BindingPhase::PlannedStop,
                    if has_child {
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
        /// Classify a child exit into a [`CrashCause`], reading the Worker's
        /// journaled stderr tail. Called OUTSIDE the binding lock.
        fn classify_exit(&self, spirit_pid: u32, facts: ExitFacts) -> CrashCause;
        /// `Handle::spawn(handle_crash)`. The one act performed under the lock:
        /// it does not block.
        fn spawn_crash_handler(&self, spirit_pid: u32, cause: CrashCause) -> CrashHandlerHandle;
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
            budget: Duration,
        ) -> Option<CrashOutcome>;
        /// Park a handler whose join timed out.
        fn park_outstanding(&self, handle: CrashHandlerHandle);
        /// Take the record out of the SCB ledger if it is still there, then
        /// apply the FR50 disposition captured at bind.
        fn disposition(
            &self,
            spirit_pid: u32,
            action: OnCrashAction,
            record: &TaskAssignmentRecord,
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
    #[derive(Default, Clone)]
    pub struct SupervisorSeams {
        /// Called between the mint and the pre-spawn phase check.
        pub after_mint: Option<Arc<dyn Fn() + Send + Sync>>,
        /// Held by the observer thread after the pidfd is stored and BEFORE its
        /// `waitid` CALL.
        pub observer_gate: Option<Arc<ObserverGate>>,
        /// AC3's injected-`?` point, PINNED after
        /// [`ExitObservation::watch`](exit_observation::ExitObservation::watch)
        /// has stored the child handle and started the observer, and BEFORE
        /// `pump_to_journal`.
        ///
        /// The position is load-bearing: injected between `spawn_and_bridge`
        /// and `watch`, even a correct build has no child handle to signal, so
        /// the vector would pass for the wrong reason.
        pub error_after_watch: Option<String>,
    }

    // ── The binding's shared core (Observer's subscriber) ───────────────

    struct BindingState {
        phase: BindingPhase,
        child: Option<ObservedChild>,
        child_pid: Option<u32>,
        handler: Option<CrashHandlerHandle>,
        record: TaskAssignmentRecord,
        on_crash_action: OnCrashAction,
        dispositioned: bool,
    }

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

        /// The current phase.
        pub fn phase(&self) -> BindingPhase {
            self.lock().phase
        }

        /// The kept record's task id.
        pub fn task_id(&self) -> String {
            self.lock().record.task_id.clone()
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
            let cause = self.exec.classify_exit(pid, facts);
            let mut state = self.lock();
            let (next, action) = state.phase.on_exit(&cause);
            state.phase = next;
            if let BindingAction::SpawnCrashHandler(cause) = action {
                state.handler = Some(self.exec.spawn_crash_handler(pid, cause));
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
            let has_child = state.child.is_some();
            let (next, action) = state.phase.begin_stop(has_child);
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

        /// Execute a decided action. The lock is NOT held here: the executor
        /// calls `unload`, which fires `WorkerSpirit::on_unload`, which takes
        /// this very lock. Holding it across `unload` blocks to the 30 s hook
        /// cap, after which `check_hook_outcome?` returns before the receipt,
        /// the revoke and the map removal — leaving an `Unloaded` SCB every
        /// later `unload` skips (Trap 6b).
        fn execute(&self, action: BindingAction) {
            let pid = self.spirit_pid();
            match action {
                BindingAction::Nothing => {}
                BindingAction::SpawnCrashHandler(_) => {
                    // Already executed inside `on_child_exit`'s lock: it is the
                    // documented non-blocking exception, and re-running it here
                    // would spawn a second handler.
                }
                BindingAction::Signal => {
                    self.signal();
                }
                BindingAction::SignalThenDispositionAndUnload => {
                    self.signal();
                    self.disposition();
                    self.exec.unload(pid);
                }
                BindingAction::DispositionAndUnload => {
                    self.disposition();
                    self.exec.unload(pid);
                }
                BindingAction::UnloadClean => {
                    let task_id = self.lock().record.task_id.clone();
                    self.exec.take_record(pid, &task_id);
                    self.exec.unload(pid);
                }
                BindingAction::RunCrashHandlerThenMaybeDisposition(cause) => {
                    match self
                        .exec
                        .run_crash_handler(pid, cause, CRASH_HANDLER_JOIN_BUDGET)
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

        fn signal(&self) {
            match self.strategy.signal_kill(self) {
                SignalOutcome::Signalled => {
                    self.counters.signals_sent.fetch_add(1, Ordering::Relaxed);
                }
                SignalOutcome::AlreadyReaped | SignalOutcome::NotSignalled => {}
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
            let (action, record) = {
                let mut state = self.lock();
                if state.dispositioned {
                    return;
                }
                state.dispositioned = true;
                (state.on_crash_action, state.record.clone())
            };
            self.exec.disposition(self.spirit_pid(), action, &record);
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
            let view = core.retire_view();
            if let Ok(mut state) = self.state.lock() {
                state
                    .active
                    .retain(|candidate| !Arc::ptr_eq(candidate, core));
                state.completed.push(view);
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
        core: Arc<BindingCore>,
        armed: bool,
        active_paths: Arc<ActivePathCount>,
        registry: Arc<BindingRegistry>,
    }

    impl WorkerBinding {
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

        /// The run reached its end. Consumes the guard and DISARMS it before
        /// executing, so `Drop` never runs `abandon` after `finish`.
        pub fn finish(mut self, bridge_exit: &ExitCause) {
            self.armed = false;
            let cause = crash_cause_from_exit(&self.core, bridge_exit);
            let action = {
                let mut state = self.core.lock();
                let (next, action) = state.phase.finish(cause.as_ref());
                state.phase = next;
                action
            };
            self.core.execute(action);
            self.registry.retire(&self.core);
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

    /// Map the bridge's own `ExitCause` to a [`CrashCause`] (D-16-3-F).
    ///
    /// `code == 0` is NOT a crash (ADR-022's false-positive rule): a clean exit
    /// whose completion oracle says `not_completed` still leaves the run
    /// failing, but it is not a fault. A signal the supervisor itself sent
    /// (phase `PlannedStop`) is never a crash either.
    ///
    /// The classification — including the journaled `stderr_tail` — goes
    /// through the SAME executor the observer uses, so the two paths cannot
    /// disagree about what a status MEANS. They can still legitimately carry
    /// DIFFERENT stderr tails: the observer reads at detection (early), this
    /// path reads after EOF (late).
    fn crash_cause_from_exit(core: &BindingCore, exit: &ExitCause) -> Option<CrashCause> {
        if core.phase() == BindingPhase::PlannedStop {
            return None;
        }
        let facts = match exit {
            ExitCause::Exited { code } => ExitFacts {
                signal: None,
                code: Some(*code),
            },
            ExitCause::Signaled { signal } => ExitFacts {
                signal: Some(*signal),
                code: None,
            },
            // Both sources lost — the executor maps this to `Truncated`, the
            // only free-text variant, REUSED rather than adding a `FaultCause`
            // variant: the epic ratified no kernel delta.
            ExitCause::Unknown => ExitFacts {
                signal: None,
                code: None,
            },
        };
        match core.exec.classify_exit(core.spirit_pid(), facts) {
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
        /// Load + start the Worker's SCB and push its in-flight task record.
        fn bind(
            &self,
            task: &WorkerTask,
            bundle: SpiritManifestBundle,
        ) -> Result<WorkerBinding, WorkerSupervisionError>;

        /// Open the platform child handle, start the exit observer, and execute
        /// the resulting action. Called on the spawning thread immediately
        /// after `spawn_and_bridge` returns — BEFORE the pump starts.
        fn watch(&self, core: &Arc<BindingCore>, child_pid: u32);

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
    /// `run_cli_wrapper_manifest`, where a 10 s real-CLI liveness probe running
    /// during a SIGTERM would hold the count past the grace and trip a false
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
        next_seq: AtomicU32,
        stopping: AtomicBool,
        boot_nonce: u64,
        root_shutdown: tokio_util::sync::CancellationToken,
        seams: SupervisorSeams,
    }

    impl WorkerSupervisor {
        /// Build the root's supervisor. Construct it AFTER
        /// `set_crash_detector` — that call needs `Arc::get_mut(&mut scheduler)`
        /// and therefore strong count 1 (Trap 1).
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            scheduler: Arc<SpiritSchedulerAdapter>,
            crash_detector: Arc<CrashDetector>,
            transparency_log: Arc<TransparencyLogAdapter>,
            halt_registry: Arc<HaltRegistry>,
            iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
            handle: tokio::runtime::Handle,
            boot_nonce: u64,
            root_shutdown: tokio_util::sync::CancellationToken,
        ) -> Self {
            let counters = Arc::new(SupervisorCounters::default());
            let strategy =
                exit_observation::choose_strategy(Arc::clone(&counters), transparency_log.as_ref());
            Self {
                exec: Arc::new(KernelBindingExecutor {
                    scheduler,
                    crash_detector,
                    transparency_log,
                    halt_registry,
                    iac,
                    handle,
                    outstanding: Mutex::new(Vec::new()),
                }),
                strategy,
                counters,
                active_paths: Arc::new(ActivePathCount::new()),
                registry: Arc::new(BindingRegistry::default()),
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

        /// Stop every live Worker and latch the supervisor: a Worker that would
        /// bind after this point is refused [`WorkerSupervisionError::WorkerStopping`].
        ///
        /// It does NOT unload: the listener's normal path stops Workers and
        /// nothing else, because unloading here would run before the door
        /// shutdown and could unload a Spirit mid-topology-load.
        pub fn stop_workers(&self) {
            self.stopping.store(true, Ordering::SeqCst);
            let cores = self.registry.active();
            for core in cores {
                let action = core.begin_stop();
                core.execute(action);
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

    impl WorkerSupervision for WorkerSupervisor {
        fn bind(
            &self,
            task: &WorkerTask,
            bundle: SpiritManifestBundle,
        ) -> Result<WorkerBinding, WorkerSupervisionError> {
            if self.stopping.load(Ordering::SeqCst) {
                return Err(WorkerSupervisionError::WorkerStopping);
            }
            // Ids are per-root and never reused: the FIRST Worker of a root is
            // `worker`, so epic 17's `maos audit query --spirit worker`
            // resolves; later ones are `worker-2`, `worker-3`, …
            let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
            let spirit_id = if seq == 0 {
                "worker".to_string()
            } else {
                format!("worker-{}", seq + 1)
            };
            let (task_id, originator_spirit_id) = task.record_ids(&spirit_id);
            let on_crash_action = bundle
                .on_crash
                .as_ref()
                .map(|section| section.action)
                .unwrap_or_default();
            let record = TaskAssignmentRecord {
                task_id,
                // Patched to `Some` after a `CliSubprocessSpawn` capability is
                // minted; remains `None` under host-grant authority.
                capability_token: None,
                ttl_deadline_ns: u64::MAX,
                intent_class: IntentClass::Standard,
                originator_spirit_id,
            };
            let core = Arc::new(BindingCore {
                spirit_id: spirit_id.clone(),
                spirit_pid: AtomicU32::new(0),
                state: Mutex::new(BindingState {
                    phase: BindingPhase::Running,
                    child: None,
                    child_pid: None,
                    handler: None,
                    record: record.clone(),
                    on_crash_action,
                    dispositioned: false,
                }),
                exec: Arc::clone(&self.exec) as Arc<dyn BindingExecutor>,
                strategy: Arc::clone(&self.strategy),
                counters: Arc::clone(&self.counters),
                seams: self.seams.clone(),
            });

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
            core.spirit_pid.store(pid, Ordering::Release);

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
            self.exec.push_record(pid, record);
            self.active_paths.enter();
            self.registry.register(Arc::clone(&core));
            Ok(WorkerBinding {
                core,
                armed: true,
                active_paths: Arc::clone(&self.active_paths),
                registry: Arc::clone(&self.registry),
            })
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

    /// The kernel side of every [`BindingAction`].
    struct KernelBindingExecutor {
        scheduler: Arc<SpiritSchedulerAdapter>,
        crash_detector: Arc<CrashDetector>,
        transparency_log: Arc<TransparencyLogAdapter>,
        #[allow(dead_code)]
        halt_registry: Arc<HaltRegistry>,
        iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
        handle: tokio::runtime::Handle,
        outstanding: Mutex<Vec<CrashHandlerHandle>>,
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

        fn spawn_crash_handler(&self, spirit_pid: u32, cause: CrashCause) -> CrashHandlerHandle {
            let detector = Arc::clone(&self.crash_detector);
            CrashHandlerHandle(self.handle.spawn(async move {
                match detector.handle_crash(spirit_pid, cause).await {
                    Ok(_) => CrashOutcome::Handled,
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
            budget: Duration,
        ) -> Option<CrashOutcome> {
            let handle = self.spawn_crash_handler(spirit_pid, cause);
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
        ) {
            self.take_record(spirit_pid, &record.task_id);
            maos_kernel_core::supervision::enforce_disposition(
                action,
                std::slice::from_ref(record),
                &self.iac,
                None,
            );
        }

        fn take_record(&self, spirit_pid: u32, task_id: &str) {
            self.with_scb(spirit_pid, |scb| {
                if let Ok(mut ledger) = scb.task_assignments_in_flight.lock() {
                    ledger.retain(|entry| entry.task_id != task_id);
                }
            });
        }

        fn unload(&self, spirit_pid: u32) {
            if let Err(error) = self.handle.block_on(self.scheduler.unload(spirit_pid)) {
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
        /// Signal the child. `on_unload` runs INSIDE `scheduler.unload`, in
        /// `spawn_blocking` under a hook budget, so it must never reach back
        /// into `unload`: `begin_stop` decides under the binding lock, releases
        /// it, and the only act is the strategy's signal.
        fn on_unload(&self, _ctx: &mut Ctx) {
            let action = self.core.begin_stop();
            self.core.execute(action);
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
            halt_registry: Arc<HaltRegistry>,
            iac: Arc<maos_kernel_core::iac::IacBusAdapter>,
            telemetry: Arc<maos_kernel_core::telemetry::iac_rt::IacRtMetrics>,
            notification_dispatcher: Arc<
                maos_director_surface::notification::NotificationDispatcher,
            >,
            boot_nonce: u64,
        ) -> Self {
            let root_cancel = tokio_util::sync::CancellationToken::new();
            let root_shutdown = tokio_util::sync::CancellationToken::new();
            let supervisor = Arc::new(WorkerSupervisor::new(
                Arc::clone(&scheduler),
                crash_detector,
                Arc::clone(&transparency_log),
                Arc::clone(&halt_registry),
                iac,
                tokio::runtime::Handle::current(),
                boot_nonce,
                root_shutdown.clone(),
            ));

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
                watchdog,
                stamper,
                listener,
            }
        }

        /// The root's Worker supervisor.
        pub fn supervisor(&self) -> &Arc<WorkerSupervisor> {
            &self.supervisor
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
            self.supervisor.join_outstanding(ROOT_GRACE).await;
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
            let report = super::unload_all_loaded(&scheduler, &halt_registry).await;
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

        use rustix::process::{
            pidfd_open, pidfd_send_signal, waitid, Pid, PidfdFlags, Signal, WaitId, WaitIdOptions,
        };

        use super::super::worker::{BindingCore, SupervisorCounters};
        use super::{ExitFacts, ExitObservation, ObserveOutcome, SignalOutcome};

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
                            self.signal_kill(&core);
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
    CrashOutcome, LiveWorker, ObserverGate, PhaseRefusal, RootSupervision, SupervisorCounters,
    SupervisorSeams, WorkerBinding, WorkerBindingView, WorkerSpirit, WorkerSupervision,
    WorkerSupervisionError, WorkerSupervisor, WorkerTask, ARMED_MARKER, CRASH_HANDLER_JOIN_BUDGET,
    DEFAULT_PROGRESS_THRESHOLD_MS, ROOT_GRACE, STAMPER_TICK,
};
