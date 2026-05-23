#![forbid(unsafe_code)]

//! Exhaustive deterministic truth table for the four ADR-022
//! universal-arithmetic predicates.
//!
//! Exercises ≥100 `(value, threshold_or_bounds, expected)` pairs per
//! predicate, including boundary conditions: `value == threshold`
//! (exclusive for above/below, inclusive for within/outside per
//! `cap/mod.rs:178-184`), `f64::INFINITY`, `f64::NEG_INFINITY`,
//! `±0.0 == 0.0`.
//!
//! NaN inputs are NOT tested here — they are rejected at `set_scalar`
//! (AC1) and never reach the predicate functions.

use maos_kernel_core::api::RingCryptoProvider;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::capability::CapabilityRegistryPort;

use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::{
    cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::sync::Arc;

fn make_adapter() -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
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
fn truth_table_on_value_above() {
    let adapter = make_adapter();

    let cases: &[(f64, f64, bool)] = &[
        // value, threshold, expected
        (5.0, 3.0, true),
        (2.0, 3.0, false),
        (3.0, 3.0, false), // == threshold → exclusive, does NOT fire
        (0.0, 0.0, false),
        (0.0, -0.0, false), // -0.0 == 0.0
        (f64::INFINITY, 1.0, true),
        (1.0, f64::INFINITY, false),
        (1.0, f64::NEG_INFINITY, true),
        (f64::NEG_INFINITY, 0.5, false),
        (0.5, f64::NEG_INFINITY, true),
        (0.0, f64::NEG_INFINITY, true),
        (f64::NEG_INFINITY, f64::NEG_INFINITY, false), // == → false
        (f64::INFINITY, f64::INFINITY, false),         // == → false
        (1e-10, 0.0, true),
        (-1e-10, 0.0, false),
        (-1.0, -2.0, true),
        (-3.0, -2.0, false),
        (f64::MAX, f64::MIN, true),
        (f64::MIN, f64::MAX, false),
    ];

    for (i, (value, threshold, expected)) in cases.iter().enumerate() {
        let result = adapter.on_value_above(*value, *threshold);
        assert_eq!(
            result, *expected,
            "above({value}, {threshold}) = {result}, expected {expected} (case {i})"
        );
    }

    // Additional systematic sweep: value from -2.0 to 2.0, threshold = 0.0
    for value_int in -200..=200 {
        let value = value_int as f64 / 100.0;
        let expected = value > 0.0;
        assert_eq!(
            adapter.on_value_above(value, 0.0),
            expected,
            "above({value}, 0.0) should be {expected}"
        );
    }
}

#[test]
fn truth_table_on_value_below() {
    let adapter = make_adapter();

    let cases: &[(f64, f64, bool)] = &[
        (2.0, 3.0, true),
        (5.0, 3.0, false),
        (3.0, 3.0, false), // == threshold → exclusive, does NOT fire
        (0.0, 0.0, false),
        (0.0, -0.0, false),
        (f64::NEG_INFINITY, 1.0, true),
        (f64::INFINITY, f64::INFINITY, false),
        (1e-10, 0.0, false),
        (-1e-10, 0.0, true),
        (-1.0, -2.0, false),
        (-3.0, -2.0, true),
        (f64::MIN, f64::MAX, true),
        (f64::MAX, f64::MIN, false),
    ];

    for (i, (value, threshold, expected)) in cases.iter().enumerate() {
        let result = adapter.on_value_below(*value, *threshold);
        assert_eq!(
            result, *expected,
            "below({value}, {threshold}) = {result}, expected {expected} (case {i})"
        );
    }

    for value_int in -200..=200 {
        let value = value_int as f64 / 100.0;
        let expected = value < 0.0;
        assert_eq!(
            adapter.on_value_below(value, 0.0),
            expected,
            "below({value}, 0.0) should be {expected}"
        );
    }
}

#[test]
fn truth_table_on_value_within() {
    let adapter = make_adapter();

    let cases: &[(f64, f64, f64, bool)] = &[
        (2.0, 1.0, 3.0, true),
        (1.0, 1.0, 3.0, true), // value == lower → inclusive, fires
        (3.0, 1.0, 3.0, true), // value == upper → inclusive, fires
        (0.5, 1.0, 3.0, false),
        (4.0, 1.0, 3.0, false),
        (0.0, 0.0, 0.0, true),  // degenerate range
        (0.5, 1.0, 0.5, false), // lower > upper but not rejected at runtime
        (f64::INFINITY, 1.0, 2.0, false),
        (1.5, f64::NEG_INFINITY, f64::INFINITY, true),
        (f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, true),
        (0.0, f64::NEG_INFINITY, f64::NEG_INFINITY, false),
    ];

    for (i, (value, lower, upper, expected)) in cases.iter().enumerate() {
        let result = adapter.on_value_within(*value, *lower, *upper);
        assert_eq!(
            result, *expected,
            "within({value}, {lower}, {upper}) = {result}, expected {expected} (case {i})"
        );
    }

    for value_int in -200..=200 {
        let value = value_int as f64 / 100.0;
        let expected = 0.0 <= value && value <= 1.0;
        assert_eq!(
            adapter.on_value_within(value, 0.0, 1.0),
            expected,
            "within({value}, 0.0, 1.0) should be {expected}"
        );
    }
}

#[test]
fn truth_table_on_value_outside() {
    let adapter = make_adapter();

    let cases: &[(f64, f64, f64, bool)] = &[
        (2.0, 1.0, 3.0, false),
        (0.5, 1.0, 3.0, true),  // below lower → fires
        (4.0, 1.0, 3.0, true),  // above upper → fires
        (1.0, 1.0, 3.0, false), // == lower → inclusive, inside
        (3.0, 1.0, 3.0, false), // == upper → inclusive, inside
        (0.0, 0.0, 1.0, false), // == lower → inside (inclusive)
        (1.0, 0.0, 1.0, false), // == upper (inside)
        (f64::INFINITY, 1.0, 2.0, true),
        (f64::NEG_INFINITY, 1.0, 2.0, true),
        (1.5, f64::NEG_INFINITY, f64::INFINITY, false), // everything inside
        (0.0, f64::NEG_INFINITY, f64::NEG_INFINITY, true), // above upper
    ];

    for (i, (value, lower, upper, expected)) in cases.iter().enumerate() {
        let result = adapter.on_value_outside(*value, *lower, *upper);
        assert_eq!(
            result, *expected,
            "outside({value}, {lower}, {upper}) = {result}, expected {expected} (case {i})"
        );
    }

    for value_int in -200..=200 {
        let value = value_int as f64 / 100.0;
        let expected = value < 0.0 || value > 1.0;
        assert_eq!(
            adapter.on_value_outside(value, 0.0, 1.0),
            expected,
            "outside({value}, 0.0, 1.0) should be {expected}"
        );
    }
}
