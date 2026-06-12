#![forbid(unsafe_code)]

//! Integration test: cold-swap upgrade with in-flight task semantics (AC3).
//!
//! v0.3-β: Full end-to-end cold-swap with scheduler requires the composition
//! root (see `maos-bin/src/main.rs::smoke-upgrade-revoke-5`). These tests
//! verify the type-level and policy-level contracts that the cold-swap arm
//! depends on.

#[test]
fn cold_swap_policy_exists_in_upgrade_policy_enum() {
    use maos_kernel_core::lifecycle::UpgradePolicy;
    // ColdSwap must be a valid policy variant
    let policy = UpgradePolicy::ColdSwap;
    assert_eq!(policy.as_str(), "cold-swap");
    assert!("cold-swap".parse::<UpgradePolicy>().is_ok());
}

#[test]
fn upgrade_report_captures_halt_receipts_field() {
    use maos_kernel_core::lifecycle::{UpgradeOutcome, UpgradePolicy, UpgradeReport};
    let report = UpgradeReport {
        spirit_id: "test-spirit".into(),
        predecessor_version: "0.1.0".into(),
        successor_version: "0.1.1".into(),
        policy: UpgradePolicy::ColdSwap,
        outcome: UpgradeOutcome::Completed,
        latency_ns: 1_000_000,
        halt_receipts_produced: 2,
    };
    assert_eq!(report.halt_receipts_produced, 2);
    assert_eq!(report.policy, UpgradePolicy::ColdSwap);
    assert_eq!(report.outcome, UpgradeOutcome::Completed);
}

#[test]
fn upgrade_error_lifecycle_variant_carries_message() {
    use maos_kernel_core::lifecycle::UpgradeError;
    let err = UpgradeError::Lifecycle(maos_domain::lifecycle::LifecycleError::NotLoaded {
        spirit_id: "test".into(),
    });
    let msg = format!("{err}");
    assert!(
        msg.contains("test"),
        "error message should contain spirit_id: {msg}"
    );
}

#[test]
fn upgrade_error_not_loaded_display() {
    use maos_kernel_core::lifecycle::UpgradeError;
    let err = UpgradeError::NotLoaded {
        spirit_id: "missing-spirit".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("missing-spirit"));
    assert!(msg.contains("not loaded"));
}
