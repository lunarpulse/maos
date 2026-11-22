#![forbid(unsafe_code)]

//! Spirit Control Block — per-Spirit kernel-side state held by the
//! Spirit Scheduler. Analog to an OS Process Control Block (PCB).
//!
//! Architecture §4.1 + Story 5.1 Task 3.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use parking_lot::RwLock;

use crate::scheduler::kernel_ctx::KernelCtx;
use maos_domain::lifecycle::{LifecycleError, SpiritLifecycleState};
use maos_spirit_abi::lifecycle::Spirit;

use crate::security::manifest::{
    ClassSection, GatewaysSection, LifecycleSection, OnCrashSection, SchedulesSection,
    SchedulingSection, SupervisionSection,
};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::task::TaskAssignmentRecord;
use maos_domain::revocation::RevocationAction;
use maos_domain::sandbox::SandboxInspectReport;
use maos_domain::supervision::OnCrashAction;

/// Kernel-side mirror of `maos_domain::lifecycle::SpiritLifecycleState`
/// encoded as a `repr(u8)` for atomic CAS transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScbLifecycleState {
    Loaded = 0,
    Running = 1,
    Paused = 2,
    Unloaded = 3,
}

impl From<ScbLifecycleState> for SpiritLifecycleState {
    fn from(s: ScbLifecycleState) -> Self {
        match s {
            ScbLifecycleState::Loaded => SpiritLifecycleState::Loaded,
            ScbLifecycleState::Running => SpiritLifecycleState::Running,
            ScbLifecycleState::Paused => SpiritLifecycleState::Paused,
            ScbLifecycleState::Unloaded => SpiritLifecycleState::Unloaded,
        }
    }
}

impl From<SpiritLifecycleState> for ScbLifecycleState {
    fn from(s: SpiritLifecycleState) -> Self {
        match s {
            SpiritLifecycleState::Loaded => ScbLifecycleState::Loaded,
            SpiritLifecycleState::Running => ScbLifecycleState::Running,
            SpiritLifecycleState::Paused => ScbLifecycleState::Paused,
            SpiritLifecycleState::Unloaded => ScbLifecycleState::Unloaded,
        }
    }
}

/// Error returned when an invalid lifecycle state transition is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionError {
    /// The CAS failed because the current state is not the expected one.
    RaceLost {
        expected: ScbLifecycleState,
        actual: ScbLifecycleState,
    },
}

/// Host-only lifecycle finalization event delivered to a spirit object by the
/// kernel AFTER the coordinator has published it as the live runtime.
///
/// This is not a guest hook and has no WIT / guest-ABI representation: it is a
/// type-erased host callback (see [`make_spirit_obj_with_host_hooks`]) letting a
/// host-side owner of a staged candidate (e.g. a spawned-session link) learn that
/// the coordinator's outcome is the single commit authority. A candidate that is
/// refused, stale, or never published never receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLifecycleEvent {
    /// The hot-swap coordinator accepted the outer swap-in outcome and published
    /// this object into the original SCB (same `Arc<SpiritControlBlock>`, nonterminal
    /// state, predecessor object still current). `pid`/`boot_nonce` identify that SCB.
    SwapCommitted { pid: u32, boot_nonce: u64 },
}

/// Why [`SpiritControlBlock::publish_runtime_if_current`] refused to publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimePublishError {
    /// The SCB is `Unloaded`.
    Terminal,
    /// The live runtime's object is no longer the expected one.
    ObjectChanged,
}

/// Object-safe trait bundling all Spirit lifecycle hooks for type-erased dispatch.
///
/// Each method dispatches to the corresponding `Spirit` trait method
/// through the `SpiritVtable<T>` function pointers. The SCB stores this
/// as `Arc<dyn AnySpiritObj>` for heterogeneous Spirit collections.
pub trait AnySpiritObj: Send + Sync {
    fn on_load(&self, ctx: &mut KernelCtx);
    fn on_start(&self, ctx: &mut KernelCtx);
    fn on_frame(&self, ctx: &mut KernelCtx, payload: &[u8]);
    fn on_idle(&self, ctx: &mut KernelCtx);
    fn on_telemetry_event(&self, ctx: &mut KernelCtx, payload: &[u8]);
    fn on_schedule(&self, ctx: &mut KernelCtx, payload: &[u8]);
    fn on_swap_in(&self, ctx: &mut KernelCtx, payload: &[u8]);
    fn on_pause(&self, ctx: &mut KernelCtx);
    fn on_resume(&self, ctx: &mut KernelCtx);
    fn on_unload(&self, ctx: &mut KernelCtx);
    fn on_consolidate(&self, ctx: &mut KernelCtx, payload: &[u8]);
    /// Story 5.2 — swap-out preparation hook.
    fn on_swap_out(&self, ctx: &mut KernelCtx);
    /// Story 5.2 — state snapshot hook (returns CBOR-encoded state blob).
    fn snapshot(&self, ctx: &mut KernelCtx) -> Vec<u8>;
    /// Story 5.2 — cross-major migration hook.
    fn migrate(
        &self,
        ctx: &mut KernelCtx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, maos_spirit_abi::lifecycle::MigratorError>;
    /// Host-only finalization callback (not a guest hook, no ABI). Default: ignored.
    /// Delivered by the hot-swap coordinator only after it published this object.
    fn finalize_lifecycle(&self, _event: HostLifecycleEvent) {}
    /// Host-only (not a guest hook, no ABI): the kernel's `on_idle` firing, with the real
    /// monotonic `deadline` it captured before queuing. Default: the deadline is not
    /// surfaced and this is `on_idle`.
    fn on_idle_until(&self, ctx: &mut KernelCtx, _deadline: std::time::Instant) {
        self.on_idle(ctx);
    }
    /// Host-only counterpart of [`Self::on_idle_until`] for `on_schedule`.
    fn on_schedule_until(
        &self,
        ctx: &mut KernelCtx,
        payload: &[u8],
        _deadline: std::time::Instant,
    ) {
        self.on_schedule(ctx, payload);
    }
}

/// Typed host callbacks of a spawned Spirit for kernel `on_idle` / `on_schedule` firings.
pub struct FiringHooks<T> {
    pub idle: fn(&T, &mut maos_spirit_abi::ctx::Ctx, std::time::Instant),
    pub schedule: fn(
        &T,
        &mut maos_spirit_abi::ctx::Ctx,
        &maos_spirit_abi::lifecycle::SchedulePayload<'_>,
        std::time::Instant,
    ),
}

/// Wraps a concrete `T: Spirit` with its `SpiritVtable` into an `AnySpiritObj`.
struct VtableSpiritObj<T: Spirit + Send + Sync + 'static> {
    spirit: T,
    vtable: maos_spirit_abi::lifecycle::SpiritVtable<T>,
    /// Optional typed host finalizer; see [`make_spirit_obj_with_host_hooks`].
    finalizer: Option<fn(&T, HostLifecycleEvent)>,
    /// Optional typed host callbacks for a kernel firing's absolute deadline.
    firing: Option<FiringHooks<T>>,
}

impl<T: Spirit + Send + Sync + 'static> AnySpiritObj for VtableSpiritObj<T> {
    fn on_load(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_load)(&self.spirit, kctx.ctx);
    }
    fn on_start(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_start)(&self.spirit, kctx.ctx);
    }
    fn on_frame(&self, kctx: &mut KernelCtx, _payload: &[u8]) {
        (self.vtable.on_frame)(
            &self.spirit,
            kctx.ctx,
            &maos_spirit_abi::lifecycle::FramePayload {
                frame_data: _payload,
                frame_len: _payload.len(),
            },
        );
    }
    fn on_idle(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_idle)(&self.spirit, kctx.ctx);
    }
    fn on_telemetry_event(&self, kctx: &mut KernelCtx, _payload: &[u8]) {
        (self.vtable.on_telemetry_event)(
            &self.spirit,
            kctx.ctx,
            &maos_spirit_abi::lifecycle::TelemetryEventPayload {
                event_data: _payload,
                event_len: _payload.len(),
            },
        );
    }
    fn on_schedule(&self, kctx: &mut KernelCtx, _payload: &[u8]) {
        (self.vtable.on_schedule)(
            &self.spirit,
            kctx.ctx,
            &maos_spirit_abi::lifecycle::SchedulePayload {
                schedule_data: _payload,
                schedule_len: _payload.len(),
            },
        );
    }
    fn on_swap_in(&self, kctx: &mut KernelCtx, _payload: &[u8]) {
        (self.vtable.on_swap_in)(
            &self.spirit,
            kctx.ctx,
            &maos_spirit_abi::lifecycle::SwapInPayload {
                predecessor_state: _payload,
                state_len: _payload.len(),
            },
        );
    }
    fn on_pause(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_pause)(&self.spirit, kctx.ctx);
    }
    fn on_resume(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_resume)(&self.spirit, kctx.ctx);
    }
    fn on_unload(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_unload)(&self.spirit, kctx.ctx);
    }
    fn on_consolidate(&self, kctx: &mut KernelCtx, _payload: &[u8]) {
        (self.vtable.on_consolidate)(
            &self.spirit,
            kctx.ctx,
            &maos_spirit_abi::lifecycle::ConsolidatePayload {
                batch_data: _payload,
                batch_len: _payload.len(),
            },
        );
    }
    fn on_swap_out(&self, kctx: &mut KernelCtx) {
        (self.vtable.on_swap_out)(&self.spirit, kctx.ctx);
    }
    fn snapshot(&self, kctx: &mut KernelCtx) -> Vec<u8> {
        (self.vtable.snapshot)(&self.spirit, kctx.ctx)
    }
    fn migrate(
        &self,
        kctx: &mut KernelCtx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, maos_spirit_abi::lifecycle::MigratorError> {
        (self.vtable.migrate)(&self.spirit, kctx.ctx, predecessor_state)
    }
    fn finalize_lifecycle(&self, event: HostLifecycleEvent) {
        if let Some(finalizer) = self.finalizer {
            finalizer(&self.spirit, event);
        }
    }
    fn on_idle_until(&self, kctx: &mut KernelCtx, deadline: std::time::Instant) {
        match &self.firing {
            Some(firing) => (firing.idle)(&self.spirit, kctx.ctx, deadline),
            None => self.on_idle(kctx),
        }
    }
    fn on_schedule_until(
        &self,
        kctx: &mut KernelCtx,
        payload: &[u8],
        deadline: std::time::Instant,
    ) {
        match &self.firing {
            Some(firing) => (firing.schedule)(
                &self.spirit,
                kctx.ctx,
                &maos_spirit_abi::lifecycle::SchedulePayload {
                    schedule_data: payload,
                    schedule_len: payload.len(),
                },
                deadline,
            ),
            None => self.on_schedule(kctx, payload),
        }
    }
}

/// Construct an `Arc<dyn AnySpiritObj>` from a concrete Spirit and its vtable.
pub fn make_spirit_obj<T: Spirit + Send + Sync + 'static>(spirit: T) -> Arc<dyn AnySpiritObj> {
    let vtable = maos_spirit_abi::lifecycle::SpiritVtable::<T>::from_spirit();
    Arc::new(VtableSpiritObj {
        spirit,
        vtable,
        finalizer: None,
        firing: None,
    })
}

/// Like [`make_spirit_obj`], plus the typed host-only callbacks of a spawned Spirit.
///
/// BINDING API for the bin (spawned/staged successor factory): build the staged
/// candidate object with `finalizer = |t: &T, event| ...` where `T` holds the
/// host-side lifecycle link. The hot-swap coordinator invokes it exactly once,
/// synchronously, with [`HostLifecycleEvent::SwapCommitted`] only AFTER it
/// published the candidate into the original SCB; refused/stale/reverted
/// candidates never receive it. The callback must be fast and non-blocking
/// (signal the session, release the guest-`Done` publication gate): it runs on
/// the coordinator's task. The inner guest `SwapIn::Done` must not publish
/// handover or authority itself; it only reports the swap-in result, and the
/// link publishes `Handover` from this callback instead.
///
/// `firing` carries the typed host callbacks for a kernel `on_idle` / `on_schedule`
/// firing; each receives the real monotonic deadline the kernel captured before it
/// queued the firing (see [`AnySpiritObj::on_idle_until`]). No TLS, no ABI change.
pub fn make_spirit_obj_with_host_hooks<T: Spirit + Send + Sync + 'static>(
    spirit: T,
    finalizer: fn(&T, HostLifecycleEvent),
    firing: FiringHooks<T>,
) -> Arc<dyn AnySpiritObj> {
    let vtable = maos_spirit_abi::lifecycle::SpiritVtable::<T>::from_spirit();
    Arc::new(VtableSpiritObj {
        spirit,
        vtable,
        finalizer: Some(finalizer),
        firing: Some(firing),
    })
}

/// Bundled manifest sections held in the SCB's swappable runtime cell.
#[derive(Debug, Clone)]
pub struct SpiritManifestBundle {
    pub scheduling: SchedulingSection,
    pub lifecycle: LifecycleSection,
    pub class: Option<ClassSection>,
    pub hot_swap: Option<crate::security::manifest::HotSwapManifestSection>,
    pub migrates_from: Option<crate::security::manifest::MigratesFromSection>,
    pub halt_protocol_compatibility:
        Option<crate::security::manifest::HaltProtocolCompatibilitySection>,
    pub on_crash: Option<OnCrashSection>,
    pub on_revocation: Option<crate::security::manifest::OnRevocationSection>,
    pub supervision: Option<SupervisionSection>,
    /// Story 6.4 / FR26 — `[[schedule]]` entries declared at admission.
    /// Defaults to the empty section (no scheduled invocations).
    pub schedules: SchedulesSection,
    /// Story 6.5 / FR54 — `[[gateway]]` entries declared at admission.
    /// Defaults to the empty section (no gateway sub-modules).
    pub gateways: GatewaysSection,
    /// Story 8.11 / AC3 — the parsed `[budget]` section. `None` ⇒ the
    /// dispatcher's `DEFAULT_TIME_CAP_SECONDS` governs this Spirit's hooks;
    /// `Some` ⇒ `budget.time_cap_seconds` is the per-Spirit hook cap. The kernel
    /// holds the value but never learns *what* the budget bounds (no LLM types).
    pub budget: Option<crate::security::manifest::Budget>,
    /// Story 20-8 (FR8) — the `[swap_invariants]` a swap reads from the live SCB.
    pub swap_invariants: Option<crate::security::manifest::SwapInvariantsSection>,
}

impl Default for SpiritManifestBundle {
    fn default() -> Self {
        Self {
            scheduling: SchedulingSection::default(),
            lifecycle: LifecycleSection::default(),
            class: None,
            hot_swap: None,
            migrates_from: None,
            halt_protocol_compatibility: None,
            on_crash: None,
            on_revocation: None,
            supervision: None,
            schedules: SchedulesSection::default(),
            gateways: GatewaysSection::default(),
            budget: None,
            swap_invariants: None,
        }
    }
}

/// Mutable execution snapshot held inside the stable SCB allocation.
/// Identity, lifecycle atomics, DRR deficit, watchdog clocks, task ledger and
/// boot nonce deliberately remain outside this cell across hot swaps.
#[maos_attrs::i9_exempt(reason = "SCB runtime snapshot; per-Spirit structural state per I9")]
#[derive(Clone)]
pub struct ScbRuntimeSnapshot {
    pub manifest: SpiritManifestBundle,
    pub spirit_obj: Arc<dyn AnySpiritObj>,
    pub priority_weight: u8,
    pub on_crash_action: OnCrashAction,
    pub on_revocation_action: RevocationAction,
    pub sandbox_tier: SandboxTier,
}

/// Per-Spirit kernel-side control block — the PCB analog.
#[maos_attrs::i9_exempt(
    reason = "per-Spirit kernel control block (PCB analog); supervised single-owner runtime state held under the scheduler supervisor per §4.0.8 — structural-state per I9, bounded by Spirit lifetime, keyed by spirit_id, no parameter drift (Story 7.1.7 baseline-reset)"
)]
pub struct SpiritControlBlock {
    pub pid: u32,
    pub spirit_id: String,
    /// Atomic u8 encoding ScbLifecycleState.
    pub state: AtomicU8,
    /// DRR running deficit counter.
    pub deficit_counter: AtomicU32,
    /// Timestamp (ns) of last inbound frame — mutated by Mailbox::deliver.
    pub last_inbound_frame_ns: AtomicU64,
    /// Timestamp (ns) of last on_idle fire — mutated by IdleWatchdog.
    pub last_idle_fire_ns: AtomicU64,
    /// Story 5.3 — timestamp (ns) of last outbound progress IAC frame.
    pub last_progress_iac_ns: AtomicU64,
    /// Story 5.3 — timestamp (ns) of last heartbeat marker.
    pub last_heartbeat_ns: AtomicU64,
    /// Story 5.3 — timestamp (ns) of last `TaskStalled` emit (multi-fire avoidance).
    pub last_stall_emit_ns: AtomicU64,
    /// Story 5.3 — timestamp (ns) of last `SilentFailureSuspect` emit (multi-fire avoidance).
    pub last_silent_failure_emit_ns: AtomicU64,
    /// Story 5.3 — per-SCB in-flight task ledger.
    pub task_assignments_in_flight: Mutex<Vec<TaskAssignmentRecord>>,
    /// Kernel boot nonce for token validation.
    pub boot_nonce: u64,
    /// Last concrete sandbox application for the active process.  This stays
    /// on the stable SCB, not the swappable runtime snapshot, so a live
    /// operator report cannot regress to fabricated defaults during a swap.
    sandbox_report: RwLock<Option<SandboxInspectReport>>,
    /// The only hot-swappable portion of a control block.
    runtime: Arc<RwLock<ScbRuntimeSnapshot>>,
    /// Publication identity: bumped (under the runtime write lock) by every runtime
    /// replacement, so a record of one publication can never act on a later one even
    /// when both install the same `Arc` object.
    runtime_generation: AtomicU64,
}

impl std::fmt::Debug for SpiritControlBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let runtime = self.runtime_snapshot();
        f.debug_struct("SpiritControlBlock")
            .field("pid", &self.pid)
            .field("spirit_id", &self.spirit_id)
            .field("state", &self.current_state())
            .field("manifest", &runtime.manifest)
            .field("priority_weight", &runtime.priority_weight)
            .field(
                "deficit_counter",
                &self.deficit_counter.load(Ordering::Relaxed),
            )
            .field(
                "last_inbound_frame_ns",
                &self.last_inbound_frame_ns.load(Ordering::Relaxed),
            )
            .field(
                "last_idle_fire_ns",
                &self.last_idle_fire_ns.load(Ordering::Relaxed),
            )
            .field("boot_nonce", &self.boot_nonce)
            .finish_non_exhaustive()
    }
}

impl SpiritControlBlock {
    pub fn new(
        pid: u32,
        spirit_id: String,
        manifest: SpiritManifestBundle,
        spirit_obj: Arc<dyn AnySpiritObj>,
        boot_nonce: u64,
    ) -> Self {
        let priority_weight = manifest.scheduling.priority_weight;
        let on_crash_action = manifest
            .on_crash
            .as_ref()
            .map(|s| s.action.clone())
            .unwrap_or_default();
        let on_revocation_action = manifest
            .on_revocation
            .as_ref()
            .map(|s| s.action)
            .unwrap_or_default();
        Self {
            pid,
            spirit_id,
            state: AtomicU8::new(ScbLifecycleState::Loaded as u8),
            deficit_counter: AtomicU32::new(0),
            // Story 5.1 review backfill — seed `last_inbound_frame_ns` to SCB
            // creation time so the IdleWatchdog can fire on_idle even for
            // Spirits that never received a frame. Previously initialized to
            // 0, which the watchdog filter `last_inbound == 0` rejected,
            // causing the substrate to never fire on_idle in production for
            // freshly-loaded Spirits (the test `idle_watchdog_skips_manifest_
            // disabled_hook` passed for the wrong reason — it relied on this
            // bug). Mailbox::deliver overwrites this on first inbound frame.
            last_inbound_frame_ns: AtomicU64::new(crate::capability::cap_tokens::monotonic_now_ns()),
            last_idle_fire_ns: AtomicU64::new(0),
            last_progress_iac_ns: AtomicU64::new(crate::capability::cap_tokens::monotonic_now_ns()),
            last_heartbeat_ns: AtomicU64::new(crate::capability::cap_tokens::monotonic_now_ns()),
            last_stall_emit_ns: AtomicU64::new(0),
            last_silent_failure_emit_ns: AtomicU64::new(0),
            task_assignments_in_flight: Mutex::new(Vec::new()),
            boot_nonce,
            sandbox_report: RwLock::new(None),
            runtime_generation: AtomicU64::new(0),
            runtime: Arc::new(RwLock::new(ScbRuntimeSnapshot {
                manifest,
                spirit_obj,
                priority_weight,
                on_crash_action,
                on_revocation_action,
                sandbox_tier: SandboxTier::default(),
            })),
        }
    }
    /// Record the report emitted by actual T3 admission/spawn work.
    pub fn record_sandbox_report(&self, report: SandboxInspectReport) {
        debug_assert_eq!(report.spirit_id, self.spirit_id);
        *self.sandbox_report.write() = Some(report);
    }

    /// Return the exact live report retained by this SCB.
    pub fn sandbox_report(&self) -> Option<SandboxInspectReport> {
        self.sandbox_report.read().clone()
    }

    /// Snapshot the current executable runtime without replacing the SCB Arc.
    pub fn runtime_snapshot(&self) -> ScbRuntimeSnapshot {
        self.runtime.read().clone()
    }

    /// The runtime and the publication generation that identifies it, read together
    /// under the runtime lock (the generation only changes under its write lock).
    pub(crate) fn runtime_snapshot_with_generation(&self) -> (ScbRuntimeSnapshot, u64) {
        let runtime = self.runtime.read();
        (runtime.clone(), self.runtime_generation())
    }

    /// Clone the runtime cell for a dispatch task that must resolve the active
    /// object at execution time rather than retaining a predecessor snapshot.
    pub(crate) fn runtime_cell(&self) -> Arc<RwLock<ScbRuntimeSnapshot>> {
        Arc::clone(&self.runtime)
    }

    /// The private staging block relinquishes its runtime after the swap-in hook joins.
    pub(crate) fn into_staged_runtime(self) -> Option<ScbRuntimeSnapshot> {
        Arc::try_unwrap(self.runtime).ok().map(RwLock::into_inner)
    }

    /// Replace only runtime-owned state and return the exact prior snapshot
    pub fn replace_runtime(&self, runtime: ScbRuntimeSnapshot) -> ScbRuntimeSnapshot {
        let mut current = self.runtime.write();
        self.runtime_generation.fetch_add(1, Ordering::AcqRel);
        std::mem::replace(&mut *current, runtime)
    }

    /// The identity of the currently published runtime (see `runtime_generation`).
    pub(crate) fn runtime_generation(&self) -> u64 {
        self.runtime_generation.load(Ordering::Acquire)
    }

    /// Atomically (under the runtime write lock) replace the runtime only while
    /// the SCB is nonterminal, its live object is still `expected` and, when
    /// `generation` is given, it is still that exact publication. Returns the
    /// exact prior snapshot and the new publication's generation. Used for
    /// coordinator commit and matching auto-revert so a stale attempt can never
    /// overwrite a changed runtime.
    pub(crate) fn publish_runtime_if_current(
        &self,
        expected: &Arc<dyn AnySpiritObj>,
        generation: Option<u64>,
        runtime: ScbRuntimeSnapshot,
    ) -> Result<(ScbRuntimeSnapshot, u64), RuntimePublishError> {
        let mut current = self.runtime.write();
        if self.current_state() == ScbLifecycleState::Unloaded {
            return Err(RuntimePublishError::Terminal);
        }
        if !std::ptr::addr_eq(Arc::as_ptr(&current.spirit_obj), Arc::as_ptr(expected))
            || generation.is_some_and(|g| g != self.runtime_generation())
        {
            return Err(RuntimePublishError::ObjectChanged);
        }
        let published = self.runtime_generation.fetch_add(1, Ordering::AcqRel) + 1;
        Ok((std::mem::replace(&mut *current, runtime), published))
    }

    /// Read the current lifecycle state.
    pub fn current_state(&self) -> ScbLifecycleState {
        let raw = self.state.load(Ordering::Acquire);
        match raw {
            0 => ScbLifecycleState::Loaded,
            1 => ScbLifecycleState::Running,
            2 => ScbLifecycleState::Paused,
            3 => ScbLifecycleState::Unloaded,
            _ => ScbLifecycleState::Unloaded,
        }
    }

    /// Attempt a state transition via CAS.
    pub fn try_transition(
        &self,
        expected: ScbLifecycleState,
        next: ScbLifecycleState,
    ) -> Result<ScbLifecycleState, StateTransitionError> {
        // Entering `Unloaded` is serialized with runtime publication: a publish holds
        // the runtime write lock while it checks the state and replaces the runtime,
        // so it either lands before this CAS or sees the terminal state.
        let _publish_fence = (next == ScbLifecycleState::Unloaded).then(|| self.runtime.write());
        let old = self.state.compare_exchange(
            expected as u8,
            next as u8,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        match old {
            Ok(prev) => {
                let prev_state = match prev {
                    0 => ScbLifecycleState::Loaded,
                    1 => ScbLifecycleState::Running,
                    2 => ScbLifecycleState::Paused,
                    _ => ScbLifecycleState::Unloaded,
                };
                Ok(prev_state)
            }
            Err(actual) => {
                let actual_state = match actual {
                    0 => ScbLifecycleState::Loaded,
                    1 => ScbLifecycleState::Running,
                    2 => ScbLifecycleState::Paused,
                    _ => ScbLifecycleState::Unloaded,
                };
                Err(StateTransitionError::RaceLost {
                    expected,
                    actual: actual_state,
                })
            }
        }
    }

    /// Check if a transition is allowed by the state machine.
    ///
    /// Story 16-6 — `(Loaded, Unloaded)` exists because `maosctl load` makes
    /// `Loaded` a durable, operator-visible state for the first time. Before
    /// it, every Spirit was loaded and started in one unbroken `maos run`, so
    /// a `Loaded` SCB was a sub-second transient. The verb that creates a
    /// state owns that state's exit: without this arm a door-loaded Spirit
    /// can never be unloaded, gets no NFR-Rel-11 receipt, keeps its
    /// capability tokens live, and burns its `spirit_id` forever (the SCB
    /// stays in the map, so `resolve_pid`'s linear scan keeps matching it).
    pub fn is_transition_allowed(from: ScbLifecycleState, to: ScbLifecycleState) -> bool {
        use ScbLifecycleState::*;
        matches!(
            (from, to),
            (Loaded, Running)
                | (Loaded, Unloaded)
                | (Running, Paused)
                | (Paused, Running)
                | (Running, Unloaded)
                | (Paused, Unloaded)
                | (Unloaded, Unloaded) // idempotent unload
        )
    }

    /// Validate and attempt a state transition.
    pub fn transition(
        &self,
        next: ScbLifecycleState,
        verb: maos_domain::lifecycle::LifecycleVerb,
    ) -> Result<SpiritLifecycleState, LifecycleError> {
        let current = self.current_state();

        if current == ScbLifecycleState::Unloaded && next == ScbLifecycleState::Unloaded {
            return Ok(SpiritLifecycleState::Unloaded);
        }

        if !Self::is_transition_allowed(current, next) {
            return Err(LifecycleError::InvalidStateTransition {
                spirit_id: self.spirit_id.clone(),
                current: current.into(),
                verb,
            });
        }

        self.try_transition(current, next)
            .map(|_| next.into())
            .map_err(|e| {
                let StateTransitionError::RaceLost {
                    expected: _,
                    actual,
                } = e;
                LifecycleError::InvalidStateTransition {
                    spirit_id: self.spirit_id.clone(),
                    current: actual.into(),
                    verb,
                }
            })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/control_block_state.rs"]
mod tests;
