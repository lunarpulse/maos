#![forbid(unsafe_code)]

//! Spirit Control Block — per-Spirit kernel-side state held by the
//! Spirit Scheduler. Analog to an OS Process Control Block (PCB).
//!
//! Architecture §4.1 + Story 5.1 Task 3.

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use maos_domain::lifecycle::{LifecycleError, SpiritLifecycleState};
use maos_spirit_abi::lifecycle::Spirit;
use crate::scheduler::kernel_ctx::KernelCtx;

use crate::security::manifest::{LifecycleSection, SchedulingSection};

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
}

/// Wraps a concrete `T: Spirit` with its `SpiritVtable` into an `AnySpiritObj`.
struct VtableSpiritObj<T: Spirit + Send + Sync + 'static> {
    spirit: T,
    vtable: maos_spirit_abi::lifecycle::SpiritVtable<T>,
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
}

/// Construct an `Arc<dyn AnySpiritObj>` from a concrete Spirit and its vtable.
pub fn make_spirit_obj<T: Spirit + Send + Sync + 'static>(
    spirit: T,
) -> Arc<dyn AnySpiritObj> {
    let vtable = maos_spirit_abi::lifecycle::SpiritVtable::<T>::from_spirit();
    Arc::new(VtableSpiritObj { spirit, vtable })
}

/// Bundled manifest sections held on the SCB.
#[derive(Debug, Clone)]
pub struct SpiritManifestBundle {
    pub scheduling: SchedulingSection,
    pub lifecycle: LifecycleSection,
}

impl Default for SpiritManifestBundle {
    fn default() -> Self {
        Self {
            scheduling: SchedulingSection::default(),
            lifecycle: LifecycleSection::default(),
        }
    }
}

/// Per-Spirit kernel-side control block — the PCB analog.
pub struct SpiritControlBlock {
    pub pid: u32,
    pub spirit_id: String,
    /// Atomic u8 encoding ScbLifecycleState.
    pub state: AtomicU8,
    /// Manifest sections (scheduling, lifecycle).
    pub manifest: SpiritManifestBundle,
    /// Type-erased Spirit object for hook dispatch.
    pub spirit_obj: Arc<dyn AnySpiritObj>,
    /// Priority weight for DRR scheduling.
    pub priority_weight: u8,
    /// DRR running deficit counter.
    pub deficit_counter: AtomicU32,
    /// Timestamp (ns) of last inbound frame — mutated by Mailbox::deliver.
    pub last_inbound_frame_ns: AtomicU64,
    /// Timestamp (ns) of last on_idle fire — mutated by IdleWatchdog.
    pub last_idle_fire_ns: AtomicU64,
    /// Kernel boot nonce for token validation.
    pub boot_nonce: u64,
}

impl std::fmt::Debug for SpiritControlBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpiritControlBlock")
            .field("pid", &self.pid)
            .field("spirit_id", &self.spirit_id)
            .field("state", &self.current_state())
            .field("manifest", &self.manifest)
            .field("priority_weight", &self.priority_weight)
            .field("deficit_counter", &self.deficit_counter.load(Ordering::Relaxed))
            .field("last_inbound_frame_ns", &self.last_inbound_frame_ns.load(Ordering::Relaxed))
            .field("last_idle_fire_ns", &self.last_idle_fire_ns.load(Ordering::Relaxed))
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
        Self {
            pid,
            spirit_id,
            state: AtomicU8::new(ScbLifecycleState::Loaded as u8),
            manifest,
            spirit_obj,
            priority_weight,
            deficit_counter: AtomicU32::new(0),
            last_inbound_frame_ns: AtomicU64::new(0),
            last_idle_fire_ns: AtomicU64::new(0),
            boot_nonce,
        }
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
        let old =
            self.state
                .compare_exchange(expected as u8, next as u8, Ordering::AcqRel, Ordering::Acquire);
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
    pub fn is_transition_allowed(
        from: ScbLifecycleState,
        to: ScbLifecycleState,
    ) -> bool {
        use ScbLifecycleState::*;
        matches!(
            (from, to),
            (Loaded, Running)
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
                let StateTransitionError::RaceLost { expected: _, actual } = e;
                LifecycleError::InvalidStateTransition {
                    spirit_id: self.spirit_id.clone(),
                    current: actual.into(),
                    verb,
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::lifecycle::SpiritVtable;

    struct TestSpirit;
    impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}
    // TestSpirit is a unit struct — automatically Send + Sync + 'static.

    fn make_scb(state: ScbLifecycleState) -> SpiritControlBlock {
        let spirit_obj = make_spirit_obj(TestSpirit);
        let scb = SpiritControlBlock::new(
            42,
            "test-spirit".into(),
            SpiritManifestBundle::default(),
            spirit_obj,
            0xCAFE,
        );
        scb.state.store(state as u8, Ordering::Release);
        scb
    }

    #[test]
    fn loaded_to_running_allowed() {
        let scb = make_scb(ScbLifecycleState::Loaded);
        assert!(SpiritControlBlock::is_transition_allowed(
            scb.current_state(),
            ScbLifecycleState::Running
        ));
        let result = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
        assert!(result.is_ok());
        assert_eq!(scb.current_state(), ScbLifecycleState::Running);
    }

    #[test]
    fn running_to_paused_allowed() {
        let scb = make_scb(ScbLifecycleState::Running);
        let result = scb.try_transition(ScbLifecycleState::Running, ScbLifecycleState::Paused);
        assert!(result.is_ok());
        assert_eq!(scb.current_state(), ScbLifecycleState::Paused);
    }

    #[test]
    fn paused_to_running_allowed() {
        let scb = make_scb(ScbLifecycleState::Paused);
        let result = scb.try_transition(ScbLifecycleState::Paused, ScbLifecycleState::Running);
        assert!(result.is_ok());
        assert_eq!(scb.current_state(), ScbLifecycleState::Running);
    }

    #[test]
    fn running_to_unloaded_allowed() {
        let scb = make_scb(ScbLifecycleState::Running);
        let result = scb.try_transition(ScbLifecycleState::Running, ScbLifecycleState::Unloaded);
        assert!(result.is_ok());
        assert_eq!(scb.current_state(), ScbLifecycleState::Unloaded);
    }

    #[test]
    fn paused_to_unloaded_allowed() {
        let scb = make_scb(ScbLifecycleState::Paused);
        let result = scb.try_transition(ScbLifecycleState::Paused, ScbLifecycleState::Unloaded);
        assert!(result.is_ok());
        assert_eq!(scb.current_state(), ScbLifecycleState::Unloaded);
    }

    #[test]
    fn loaded_to_paused_rejected() {
        assert!(!SpiritControlBlock::is_transition_allowed(
            ScbLifecycleState::Loaded,
            ScbLifecycleState::Paused
        ));
    }

    #[test]
    fn loaded_to_unloaded_rejected() {
        assert!(!SpiritControlBlock::is_transition_allowed(
            ScbLifecycleState::Loaded,
            ScbLifecycleState::Unloaded
        ));
    }

    #[test]
    fn unload_idempotent() {
        let scb = make_scb(ScbLifecycleState::Unloaded);
        let result = scb.transition(ScbLifecycleState::Unloaded, maos_domain::lifecycle::LifecycleVerb::Unload);
        assert!(result.is_ok());
    }

    #[test]
    fn cas_race_tolerance() {
        let scb = make_scb(ScbLifecycleState::Loaded);
        let r1 = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
        assert!(r1.is_ok());
        let r2 = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
        assert!(r2.is_err());
        assert!(matches!(r2.unwrap_err(), StateTransitionError::RaceLost { .. }));
    }
}
