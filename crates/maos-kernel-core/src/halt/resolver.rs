#![forbid(unsafe_code)]

//! Director-side resolution sink. Story 3.3 defines the trait; Story 4.1
//! adds the production `KernelHaltResolver` that ties resolution into
//! `invoke_halt`'s pending-resolution state + halt-receipt production.
//! Integration with E3 Story 3.3 UX surface wires here — see
//! `crates/maos-director-surface/src/halt_ui.rs`.
//!
//! **Story 4.1 owns the production `KernelHaltResolver`.** This file
//! defines the two test doubles; the kernel-side state machine
//! that holds `(halt_id → HaltState)` lives at Story 4.1
//! (`crates/maos-kernel-core/src/halt/mod.rs::invoke_halt`).
//!
//! The `HaltResolver` trait and `ResolveError` live in
//! `maos-domain::halt` to avoid a circular dependency between
//! `maos-kernel-core` and `maos-director-surface`.

use std::sync::Mutex;
use maos_domain::halt::{HaltId, HaltResolver, Resolution, ResolveError};

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
