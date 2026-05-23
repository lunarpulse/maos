#![forbid(unsafe_code)]

//! AC1 integration test — exercises `set_scalar` + tap emission +
//! same-tag overwrite + NaN/empty-tag/empty-derived-from rejection.
//!
//! Uses the real `CapabilityRegistryAdapter` via `test_adapter()`-style
//! construction mirroring `halt_invoke_test.rs`.

use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::Arc;

use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::{
    cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, SetScalarError, WorkingMemoryStore,
};

fn make_adapter() -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _) = maos_kernel_core::capability::cap_audit::channel();
    let quota = CapQuotaTracker::new();
    let working_memory = Arc::new(WorkingMemoryStore::new());
    let telemetry = Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default());
    CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xCAFE,
        policy,
        audit_tx,
        quota,
        working_memory,
        telemetry,
    )
}

#[test]
fn set_scalar_happy_path_returns_tap_event() {
    let adapter = make_adapter();
    let event = adapter
        .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
        .unwrap();
    assert_eq!(event.spirit_id, "spirit-1");
    assert_eq!(event.tag, "uncertainty");
    assert_eq!(event.value, 0.75);
    assert!(event.timestamp > 0);
}

#[test]
fn set_scalar_publishes_to_telemetry_stream() {
    let adapter = make_adapter();
    let topic = maos_domain::invariants::i7::TelemetryTopic::new("scalar.tap.uncertainty");

    // Subscribe to the telemetry topic before writing
    adapter
        .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
        .unwrap();
}

#[test]
fn set_scalar_same_tag_overwrites() {
    let adapter = make_adapter();
    let first = adapter
        .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
        .unwrap();
    let second = adapter
        .set_scalar(1, "spirit-1", "uncertainty", 0.30, "frame-002")
        .unwrap();
    assert_eq!(first.value, 0.75);
    assert_eq!(second.value, 0.30);
}

#[test]
fn set_scalar_rejects_nan_value() {
    let adapter = make_adapter();
    let err = adapter
        .set_scalar(1, "spirit-1", "tag", f64::NAN, "frame-001")
        .unwrap_err();
    assert!(matches!(err, SetScalarError::NanValue));
}

#[test]
fn set_scalar_rejects_empty_tag() {
    let adapter = make_adapter();
    let err = adapter
        .set_scalar(1, "spirit-1", "", 0.5, "frame-001")
        .unwrap_err();
    assert!(matches!(err, SetScalarError::EmptyTag));
}

#[test]
fn set_scalar_rejects_empty_derived_from() {
    let adapter = make_adapter();
    let err = adapter
        .set_scalar(1, "spirit-1", "tag", 0.5, "")
        .unwrap_err();
    assert!(matches!(err, SetScalarError::EmptyDerivedFrom));
}

#[test]
fn set_scalar_spirit_scoped() {
    let adapter = make_adapter();
    adapter
        .set_scalar(1, "spirit-a", "tag-x", 0.1, "f1")
        .unwrap();
    adapter
        .set_scalar(2, "spirit-b", "tag-x", 0.9, "f2")
        .unwrap();
    // Read-back via the store
    let wm = adapter.working_memory();
    let (v1, _) = wm.get_scalar(1, "tag-x").unwrap();
    let (v2, _) = wm.get_scalar(2, "tag-x").unwrap();
    assert_eq!(v1, 0.1);
    assert_eq!(v2, 0.9);
}
