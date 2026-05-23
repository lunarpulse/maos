//! Integration test: UpgradeOrchestrator three-policy dispatch (AC1).
//!
//! v0.3-β stub — full 5-scenario coverage lands with Task 15 completion.

#[test]
fn upgrade_policy_enum_roundtrip() {
    use maos_kernel_core::lifecycle::UpgradePolicy;
    assert_eq!(UpgradePolicy::HotSwap.as_str(), "hot-swap");
    assert_eq!(UpgradePolicy::ColdSwap.as_str(), "cold-swap");
    assert_eq!(UpgradePolicy::Migrator.as_str(), "migrator");
    assert!("hot-swap".parse::<UpgradePolicy>().is_ok());
    assert!("cold-swap".parse::<UpgradePolicy>().is_ok());
    assert!("migrator".parse::<UpgradePolicy>().is_ok());
    assert!("invalid".parse::<UpgradePolicy>().is_err());
}

#[test]
fn upgrade_outcome_serde_roundtrip() {
    use maos_kernel_core::lifecycle::UpgradeOutcome;
    let original = UpgradeOutcome::Completed;
    let json = serde_json::to_string(&original).unwrap();
    let back: UpgradeOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, original);
}
