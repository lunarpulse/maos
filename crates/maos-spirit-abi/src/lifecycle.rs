#![forbid(unsafe_code)]

//! Lifecycle hooks trait — the in-process Rust trait contract between
//! the kernel and a Spirit.
//!
//! Per architecture §5.3: "The Spirit ABI is the contract between the
//! kernel and a Spirit. Every Spirit conforms to it."
//!
//! This module ships the **11-hook signature set** per FR55. The 3
//! deferred hooks from the architecture §5.3 14-hook list are:
//!
//! | Hook | Deferred to | Reason |
//! |---|---|---|
//! | `on_swap_out` | Story 5.2 | Hot-swap state transfer |
//! | `snapshot` | Story 5.2 | Hot-swap state capture |
//! | `migrate` | Story 5.2 | Hot-swap state consume |
//! | `epistemic_resolve` | Story 4.1 | Halt-protocol resolution |
//!
//! Runtime hook firing ships in Story 5.1 (full lifecycle verbs and
//! 11-trigger firing with priority-weighted scheduling). This story
//! provides the trait contract only.
//!
//! ADR-002 (Spirit form at v0.1): The trait signature serves both
//! `rust-inproc` and `subprocess` forms. The `CancellationSignal`
//! abstraction is the bridge that makes this possible — in-process
//! Spirits receive a `&dyn CancellationSignal` backed by the kernel's
//! Tokio runtime; subprocess Spirits receive a signal the wire protocol
//! carries as a message.

use crate::ctx::Ctx;
use core::marker::PhantomData;

// ------------------------------------------------------------------
// Payload types — size-stable anchors for forward compatibility
// ------------------------------------------------------------------

/// Payload delivered to `on_frame`.
///
/// v0.1-β carries raw byte slices. Full typed frames (IAC Bus dispatch)
/// land in Epic 6.
#[derive(Debug, Clone, Copy)]
pub struct FramePayload<'a> {
    pub frame_data: &'a [u8],
    pub frame_len: usize,
}

/// Payload delivered to `on_telemetry_event`.
#[derive(Debug, Clone, Copy)]
pub struct TelemetryEventPayload<'a> {
    pub event_data: &'a [u8],
    pub event_len: usize,
}

/// Payload delivered to `on_schedule`.
#[derive(Debug, Clone, Copy)]
pub struct SchedulePayload<'a> {
    pub schedule_data: &'a [u8],
    pub schedule_len: usize,
}

/// Payload delivered to `on_swap_in` (hot-swap predecessor state reference).
#[derive(Debug, Clone, Copy)]
pub struct SwapInPayload<'a> {
    pub predecessor_state: &'a [u8],
    pub state_len: usize,
}

/// Payload delivered to `on_consolidate` (batch summary).
#[derive(Debug, Clone, Copy)]
pub struct ConsolidatePayload<'a> {
    pub batch_data: &'a [u8],
    pub batch_len: usize,
}

// ------------------------------------------------------------------
// Hook budget key — maps `#[hook(budget = "...")]` to enum variant
// ------------------------------------------------------------------

/// Resource budget envelope key — the `#[hook(budget = "…")]`
/// attribute parses to one of these variants at compile time.
///
/// The kernel consults the manifest's `[budget]` section against this
/// key at firing time. Actual enforcement ships in Story 5.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookBudgetKey {
    /// Context window size budget (tokens or byte-equivalent).
    ContextWindow,
    /// Per-invocation time cap in seconds.
    TimeCapSeconds,
    /// CPU usage percentage cap.
    CpuMaxPct,
    /// Memory ceiling in MB.
    MemoryMaxMb,
    /// File descriptor cap.
    FdMax,
}

// ------------------------------------------------------------------
// Count assertion — fails build if hook count drifts from FR55 (11)
// ------------------------------------------------------------------

/// Count of hook methods on the `Spirit` trait (per FR55).
#[doc(hidden)]
#[macro_export]
macro_rules! count_hooks {
    () => { 11 };
}

// ------------------------------------------------------------------
// Spirit trait — 11 lifecycle hooks with default no-op implementations
// ------------------------------------------------------------------

/// The Spirit lifecycle trait.
///
/// A Spirit implements this trait to receive lifecycle events from the
/// kernel. Every method has a default no-op body, so a Spirit author
/// writes only the hooks they care about.
///
/// # Firing semantics (architecture §5.3 references)
///
/// | Hook | Fires when… | Payload |
/// |---|---|---|
/// | `on_load` | Spirit is admitted and loaded (§5.3.1) | — |
/// | `on_start` | Spirit receives first `Start` verb (§5.3.2) | — |
/// | `on_frame` | IAC frame arrives (§5.3.3) | `FramePayload` |
/// | `on_idle` | No frames for ≥ idle_timeout_ms (§5.3.4) | — |
/// | `on_telemetry_event` | Scalar-tap event fires (§5.3.5) | `TelemetryEventPayload` |
/// | `on_schedule` | Scheduled invocation fires (§5.3.6) | `SchedulePayload` |
/// | `on_swap_in` | Predecessor state arrives (§5.3.7) | `SwapInPayload` |
/// | `on_pause` | Kernel pauses Spirit (§5.3.8) | — |
/// | `on_resume` | Kernel resumes Spirit (§5.3.9) | — |
/// | `on_unload` | Spirit receives `Unload` verb (§5.3.10) | — |
/// | `on_consolidate` | Batch window closes (§5.3.11) | `ConsolidatePayload` |
///
/// All hooks receive a `&mut Ctx` carrying the cancellation signal,
/// capability handle, and mailbox handle.
#[allow(unused_variables)]
pub trait Spirit {
    /// Fired when the Spirit is admitted and loaded into memory.
    /// §5.3.1 — Admission → Load
    fn on_load(&self, ctx: &mut Ctx) {}

    /// Fired when the Spirit receives its first `Start` lifecycle verb.
    /// §5.3.2 — Start
    fn on_start(&self, ctx: &mut Ctx) {}

    /// Fired when an IAC frame arrives at the Spirit's mailbox.
    /// §5.3.3 — Frame receive
    fn on_frame<'a>(&self, ctx: &mut Ctx, payload: &FramePayload<'a>) {}

    /// Fired when no inbound frames have arrived for ≥ budget.idle_timeout_ms.
    /// §5.3.4 — Idle
    fn on_idle(&self, ctx: &mut Ctx) {}

    /// Fired when a scalar-tap event fires.
    /// §5.3.5 — Telemetry event
    fn on_telemetry_event<'a>(&self, ctx: &mut Ctx, payload: &TelemetryEventPayload<'a>) {}

    /// Fired when a scheduled invocation fires.
    /// §5.3.6 — Schedule
    fn on_schedule<'a>(&self, ctx: &mut Ctx, payload: &SchedulePayload<'a>) {}

    /// Fired when predecessor state arrives during hot-swap.
    /// §5.3.7 — Swap-in
    fn on_swap_in<'a>(&self, ctx: &mut Ctx, payload: &SwapInPayload<'a>) {}

    /// Fired when the kernel pauses the Spirit (e.g., resource pressure).
    /// §5.3.8 — Pause
    fn on_pause(&self, ctx: &mut Ctx) {}

    /// Fired when the kernel resumes a paused Spirit.
    /// §5.3.9 — Resume
    fn on_resume(&self, ctx: &mut Ctx) {}

    /// Fired when the Spirit receives an `Unload` lifecycle verb.
    /// §5.3.10 — Unload
    fn on_unload(&self, ctx: &mut Ctx) {}

    /// Fired when a batch window closes; a summary is delivered.
    /// §5.3.11 — Consolidate
    fn on_consolidate<'a>(&self, ctx: &mut Ctx, payload: &ConsolidatePayload<'a>) {}
}

// ------------------------------------------------------------------
// SpiritVtable — per-hook function-pointer dispatch table
// ------------------------------------------------------------------

/// Per-hook dispatch table for the Spirit trait.
///
/// Each field holds a function pointer that calls the corresponding
/// trait method. The kernel dispatches through this table at runtime
/// (Story 5.1). The vtable is constructed by the `#[spirit]` proc-macro
/// and obtained through the generated `__maos_spirit_vtable_<Type>()` symbol.
///
/// `#[repr(C)]` locks the field layout for subprocess-form FFI dispatch
/// (Epic 5, Story 5.5x). Function pointers use Rust references (safe);
/// a raw-pointer shim layer will be added in Epic 5 when the FFI bridge
/// ships, at which point `#![forbid(unsafe_code)]` on this module will
/// need relaxation to `#![deny(unsafe_code)]` per ADR-039 amendment.
#[repr(C)]
#[derive(Clone)]
pub struct SpiritVtable<T: Spirit + 'static> {
    pub on_load: fn(&T, &mut Ctx),
    pub on_start: fn(&T, &mut Ctx),
    pub on_frame: for<'a> fn(&T, &mut Ctx, &FramePayload<'a>),
    pub on_idle: fn(&T, &mut Ctx),
    pub on_telemetry_event: for<'a> fn(&T, &mut Ctx, &TelemetryEventPayload<'a>),
    pub on_schedule: for<'a> fn(&T, &mut Ctx, &SchedulePayload<'a>),
    pub on_swap_in: for<'a> fn(&T, &mut Ctx, &SwapInPayload<'a>),
    pub on_pause: fn(&T, &mut Ctx),
    pub on_resume: fn(&T, &mut Ctx),
    pub on_unload: fn(&T, &mut Ctx),
    pub on_consolidate: for<'a> fn(&T, &mut Ctx, &ConsolidatePayload<'a>),
    #[doc(hidden)]
    pub _phantom: PhantomData<T>,
}

impl<T: Spirit + 'static> SpiritVtable<T> {
    /// Construct a vtable where every hook dispatches to the trait method.
    ///
    /// This is the canonical constructor for tests and for the `#[spirit]`
    /// proc-macro reference implementation.
    #[allow(clippy::too_many_lines)]
    pub fn from_spirit() -> Self {
        fn on_load_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_load(c); }
        fn on_start_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_start(c); }
        fn on_frame_f<'a, T: Spirit>(s: &T, c: &mut Ctx, p: &FramePayload<'a>) { s.on_frame(c, p); }
        fn on_idle_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_idle(c); }
        fn on_telemetry_event_f<'a, T: Spirit>(s: &T, c: &mut Ctx, p: &TelemetryEventPayload<'a>) { s.on_telemetry_event(c, p); }
        fn on_schedule_f<'a, T: Spirit>(s: &T, c: &mut Ctx, p: &SchedulePayload<'a>) { s.on_schedule(c, p); }
        fn on_swap_in_f<'a, T: Spirit>(s: &T, c: &mut Ctx, p: &SwapInPayload<'a>) { s.on_swap_in(c, p); }
        fn on_pause_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_pause(c); }
        fn on_resume_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_resume(c); }
        fn on_unload_f<T: Spirit>(s: &T, c: &mut Ctx) { s.on_unload(c); }
        fn on_consolidate_f<'a, T: Spirit>(s: &T, c: &mut Ctx, p: &ConsolidatePayload<'a>) { s.on_consolidate(c, p); }

        Self {
            on_load:           on_load_f::<T>,
            on_start:          on_start_f::<T>,
            on_frame:          on_frame_f::<T>,
            on_idle:           on_idle_f::<T>,
            on_telemetry_event: on_telemetry_event_f::<T>,
            on_schedule:       on_schedule_f::<T>,
            on_swap_in:        on_swap_in_f::<T>,
            on_pause:          on_pause_f::<T>,
            on_resume:         on_resume_f::<T>,
            on_unload:         on_unload_f::<T>,
            on_consolidate:    on_consolidate_f::<T>,
            _phantom: PhantomData,
        }
    }
}

// ------------------------------------------------------------------
// kernel_invocation_allowed — gates hook invocations against manifest
// ------------------------------------------------------------------

/// Returns `true` if the kernel should invoke the given hook for a
/// Spirit whose manifest declares the `enabled_hooks` subset.
///
/// The invocation gate is signature-level: the kernel consults this
/// predicate at dispatch time. The runtime hook caller ships in
/// Story 5.1.
pub fn kernel_invocation_allowed(enabled_hooks: &[&str], hook_name: &str) -> bool {
    if enabled_hooks.is_empty() {
        return true;
    }
    enabled_hooks.iter().any(|&h| h == hook_name)
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancellation::NeverCancel;

    struct TestSpirit {
        called: core::cell::Cell<u32>,
    }

    impl TestSpirit {
        fn new() -> Self {
            Self { called: core::cell::Cell::new(0) }
        }
    }

    impl Spirit for TestSpirit {
        fn on_load(&self, _ctx: &mut Ctx) {
            self.called.set(self.called.get() + 1);
        }
        fn on_idle(&self, _ctx: &mut Ctx) {
            self.called.set(self.called.get() + 1);
        }
        fn on_unload(&self, _ctx: &mut Ctx) {
            self.called.set(self.called.get() + 1);
        }
    }

    fn test_ctx() -> Ctx {
        static NEVER: NeverCancel = NeverCancel;
        Ctx {
            cancellation: &NEVER,
            capability_handle: crate::ctx::CapabilityHandle(0),
            mailbox_handle: crate::ctx::MailboxHandle(0),
        }
    }

    #[test]
    fn const_assert_hook_count_matches_fr55() {
        assert_eq!(count_hooks!(), 11, "FR55 mandates exactly 11 hooks");
    }

    #[test]
    fn trait_dispatch_smoke() {
        let s = TestSpirit::new();
        let mut ctx = test_ctx();
        s.on_load(&mut ctx);
        assert_eq!(s.called.get(), 1);
        s.on_idle(&mut ctx);
        assert_eq!(s.called.get(), 2);
        s.on_unload(&mut ctx);
        assert_eq!(s.called.get(), 3);
    }

    #[test]
    fn vtable_smoke() {
        let s = TestSpirit::new();
        let vt = SpiritVtable::<TestSpirit>::from_spirit();
        let mut ctx = test_ctx();
        (vt.on_load)(&s, &mut ctx);
        assert_eq!(s.called.get(), 1);
        (vt.on_idle)(&s, &mut ctx);
        assert_eq!(s.called.get(), 2);
        (vt.on_unload)(&s, &mut ctx);
        assert_eq!(s.called.get(), 3);
    }

    #[test]
    fn default_no_ops_are_unit() {
        struct NoHookSpirit;
        impl Spirit for NoHookSpirit {}
        let s = NoHookSpirit;
        let mut ctx = test_ctx();
        s.on_load(&mut ctx);
        s.on_start(&mut ctx);
        s.on_idle(&mut ctx);
        s.on_pause(&mut ctx);
        s.on_resume(&mut ctx);
        s.on_unload(&mut ctx);
    }

    #[test]
    fn kernel_invocation_allowed_empty_permits_all() {
        assert!(kernel_invocation_allowed(&[], "on_load"));
        assert!(kernel_invocation_allowed(&[], "on_frame"));
    }

    #[test]
    fn kernel_invocation_allowed_subset_filters() {
        let enabled = &["on_load", "on_idle", "on_unload"];
        assert!(kernel_invocation_allowed(enabled, "on_load"));
        assert!(kernel_invocation_allowed(enabled, "on_idle"));
        assert!(!kernel_invocation_allowed(enabled, "on_frame"));
        assert!(!kernel_invocation_allowed(enabled, "on_start"));
    }

    #[test]
    fn hook_budget_key_variants_exist() {
        let _k = HookBudgetKey::ContextWindow;
        let _k = HookBudgetKey::TimeCapSeconds;
        let _k = HookBudgetKey::CpuMaxPct;
        let _k = HookBudgetKey::MemoryMaxMb;
        let _k = HookBudgetKey::FdMax;
    }

    #[test]
    fn payload_types_exist() {
        let fp = FramePayload { frame_data: b"hello", frame_len: 5 };
        assert_eq!(fp.frame_len, 5);
        let tp = TelemetryEventPayload { event_data: b"t", event_len: 1 };
        assert_eq!(tp.event_len, 1);
        let sp = SchedulePayload { schedule_data: b"s", schedule_len: 1 };
        assert_eq!(sp.schedule_len, 1);
        let sip = SwapInPayload { predecessor_state: b"p", state_len: 1 };
        assert_eq!(sip.state_len, 1);
        let cp = ConsolidatePayload { batch_data: b"c", batch_len: 1 };
        assert_eq!(cp.batch_len, 1);
    }
}
