#![forbid(unsafe_code)]

//! Director-side resolution sink. Story 3.3 defines the trait; Story 4.1
//! adds the production `KernelHaltResolver` that ties resolution into
//! `invoke_halt`'s pending-resolution state + halt-receipt production.
//! Integration with E3 Story 3.3 UX surface wires here — see
//! `crates/maos-director-surface/src/halt_ui.rs`.
//!
//! **Story 4.1 owns the production `KernelHaltResolver`.** This file
//! defines the two test doubles + the production resolver; the kernel-side
//! state machine that holds `(halt_id → HaltState)` lives at
//! `crates/maos-kernel-core/src/halt/mod.rs::invoke_halt`.
//!
//! The `HaltResolver` trait and `ResolveError` live in
//! `maos-domain::halt` to avoid a circular dependency between
//! `maos-kernel-core` and `maos-director-surface`.

use std::sync::{Arc, Mutex};
use maos_domain::halt::{
    HaltId, HaltResolver, HaltState, OutputMarker, Resolution, ResolveError,
};
use maos_domain::invariants::i3::FrameOrigin;
use crate::halt::HaltRegistry;
use crate::halt::output_markers::OutputMarkerRegistry;
use crate::iac::transparency_log::{TransparencyLogAdapter, FrameKind};
use crate::iac::Mailbox;

/// Captures every `resolve` call for unit-test assertion. Story 3.3
/// uses this from `halt_ui` tests to verify the submission path
/// without depending on Story 4.1's kernel-side mechanism.
#[maos_attrs::i9_exempt(reason = "test double — captures resolve() calls; production wiring is v0.3-β bootstrap; Story 4.1 swaps for KernelHaltResolver")]
#[derive(Debug, Default)]
pub struct MockHaltResolver {
    calls: Mutex<Vec<(HaltId, Resolution)>>,
}

impl MockHaltResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calls(&self) -> Vec<(HaltId, Resolution)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl HaltResolver for MockHaltResolver {
    fn resolve(&self, halt_id: &HaltId, resolution: Resolution) -> Result<(), ResolveError> {
        self.calls.lock().unwrap().push((halt_id.clone(), resolution));
        Ok(())
    }
}

/// Capture a SECOND mode that returns `UnknownHalt` for every input —
/// used by `halt_ui` tests to prove the submission path surfaces
/// `ResolveError` to the caller (vs. silently dropping it).
pub struct FailingHaltResolver;

impl HaltResolver for FailingHaltResolver {
    fn resolve(&self, halt_id: &HaltId, _: Resolution) -> Result<(), ResolveError> {
        Err(ResolveError::UnknownHalt(halt_id.as_str().into()))
    }
}

// ----- Story 4.1 — production KernelHaltResolver (additive) -----

/// **Integration with E3 Story 3.3 UX surface wires here** — see
/// `crates/maos-director-surface/src/halt_ui.rs::HaltFlow::submit_resolution`.
/// `HaltFlow` accepts `Arc<R: HaltResolver>` and calls
/// `resolver.resolve(...)`; Story 4.1 substitutes `KernelHaltResolver`
/// for `MockHaltResolver` at the composition root
/// (`crates/maos-bin/src/main.rs:528` — see AC3).
///
/// **The UX integration test (three-tap flow + dispatcher fanout) is
/// owned by Story 3.3**, not this story. Story 4.1's unit tests use the
/// kernel-side machinery (TL, Journal, Registry) directly.
///
/// ## Architecture references
/// - §4.6.1 (Epistemic Halt mechanism — three resolution kinds)
/// - Epic 3 retro A1 (`HaltResolver` at `maos-domain`)
#[maos_attrs::i9_exempt(reason = "production halt resolver — ties resolution into HaltRegistry + TransparencyLog + OutputMarkerRegistry; kernel-side state machine for three resolution kinds")]
pub struct KernelHaltResolver {
    registry: Arc<HaltRegistry>,
    tl: Arc<TransparencyLogAdapter>,
    output_markers: Arc<OutputMarkerRegistry>,
    #[allow(dead_code)]
    mailbox: Arc<Mailbox>,
    boot_nonce: u64,
}

impl KernelHaltResolver {
    pub fn new(
        registry: Arc<HaltRegistry>,
        tl: Arc<TransparencyLogAdapter>,
        output_markers: Arc<OutputMarkerRegistry>,
        mailbox: Arc<Mailbox>,
        boot_nonce: u64,
    ) -> Self {
        Self {
            registry,
            tl,
            output_markers,
            mailbox,
            boot_nonce,
        }
    }
}

impl HaltResolver for KernelHaltResolver {
    fn resolve(&self, halt_id: &HaltId, resolution: Resolution) -> Result<(), ResolveError> {
        // 1. Confirm halt is pending; transition state atomically
        let terminal = match &resolution {
            Resolution::ProvidedContext { .. } => HaltState::Resumed,
            Resolution::AcceptedHalt => HaltState::Terminated,
            Resolution::AuthorizedOverride { .. } => HaltState::Overridden,
        };
        let pre = self.registry.resolve(halt_id, terminal).map_err(|e| match e {
            crate::halt::ResolveStateError::NotPending(s) => ResolveError::UnknownHalt(s),
            crate::halt::ResolveStateError::AlreadyTerminal(s) => ResolveError::AlreadyResolved(s),
        })?;
        assert_eq!(
            pre, HaltState::PendingResolution,
            "registry must only transition from PendingResolution"
        );

        // 2. Per-variant side-effects (kernel-side, not Spirit-side)
        match &resolution {
            Resolution::ProvidedContext { text } => {
                // Story 4.3 (Principal Memory Namespace) wires the actual
                // working-memory write at memory.write. v0.3-β here only
                // marks the resolution kind; the supplied context is
                // already in the Approval Decision Log row written by
                // HaltFlow's journal call (3.3 AC4).
                let _ = text;
            }
            Resolution::AcceptedHalt => {
                // FR12 — emit task.orphaned via Transparency Log
                // v0.3-β shape: FrameKind::TaskComplete carrying
                // "orphaned: accepted_halt halt_id=..."
                self.emit_task_orphaned(halt_id);
            }
            Resolution::AuthorizedOverride { operator_policy_ref } => {
                // Append OutputMarker::Override to the Spirit's output queue.
                // Story 4.2's output_shape predicates consume this marker
                // to gate subsequent output frames.
                // Use OutputMarker::override_for to enforce non-empty validation.
                match OutputMarker::override_for(halt_id.clone(), operator_policy_ref.clone()) {
                    Ok(marker) => self.output_markers.append_for_halt(halt_id, marker),
                    Err(_) => {} // Empty policy_ref rejected at director surface; skip marker
                }
            }
        }

        Ok(())
    }
}

impl KernelHaltResolver {
    fn emit_task_orphaned(&self, halt_id: &HaltId) {
        // Construct a FrameKind::TaskComplete frame with the orphan payload.
        // v0.3-β writes directly to the Transparency Log.
        let payload = format!("orphaned: accepted_halt halt_id={}", halt_id.as_str());
        self.tl.insert_frame_event(
            FrameKind::TaskComplete,
            0, // spirit_pid — v0.3-β uses 0 for kernel-side orphan events
            None,
            "task.orphaned",
            payload.as_bytes(),
            FrameOrigin::Kernel,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_resolve_calls() {
        let mock = MockHaltResolver::new();
        let hid = HaltId::new("halt-1").unwrap();
        let res = Resolution::AcceptedHalt;
        let result = mock.resolve(&hid, res);
        assert!(result.is_ok());
        let calls = mock.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0.as_str(), "halt-1");
        assert!(matches!(calls[0].1, Resolution::AcceptedHalt));
    }

    #[test]
    fn mock_call_count_reflects_multiple_calls() {
        let mock = MockHaltResolver::new();
        mock.resolve(&HaltId::new("a").unwrap(), Resolution::AcceptedHalt).unwrap();
        mock.resolve(&HaltId::new("b").unwrap(), Resolution::AcceptedHalt).unwrap();
        mock.resolve(&HaltId::new("c").unwrap(), Resolution::AcceptedHalt).unwrap();
        assert_eq!(mock.call_count(), 3);
    }

    #[test]
    fn failing_resolver_returns_unknown_halt() {
        let fail = FailingHaltResolver;
        let hid = HaltId::new("halt-1").unwrap();
        let result = fail.resolve(&hid, Resolution::AcceptedHalt);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ResolveError::UnknownHalt(s) => assert_eq!(s, "halt-1"),
            _ => panic!("expected UnknownHalt, got {:?}", err),
        }
    }

    #[test]
    fn mock_is_send_and_sync() {
        fn _assert_send_sync<T: Send + Sync>(_: T) {}
        _assert_send_sync(MockHaltResolver::new());
        _assert_send_sync(FailingHaltResolver);
    }
}
