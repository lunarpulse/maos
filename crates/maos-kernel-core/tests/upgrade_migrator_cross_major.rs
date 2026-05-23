#![forbid(unsafe_code)]

//! Integration test: migrator policy cross-major upgrade (AC4).
//!
//! v0.3-β: Full end-to-end migrator test requires the composition root.
//! These tests verify the `MigratorNotDeclared` pre-check and policy enum
//! contracts.

#[test]
fn migrator_policy_exists_in_upgrade_policy_enum() {
    use maos_kernel_core::lifecycle::UpgradePolicy;
    let policy = UpgradePolicy::Migrator;
    assert_eq!(policy.as_str(), "migrator");
    assert!("migrator".parse::<UpgradePolicy>().is_ok());
}

#[test]
fn migrator_not_declared_error_display() {
    use maos_kernel_core::lifecycle::UpgradeError;
    let err = UpgradeError::MigratorNotDeclared;
    let msg = format!("{err}");
    assert!(msg.contains("migrator"));
    assert!(msg.contains("migrates_from"));
}

#[test]
fn migrator_not_declared_is_distinct_from_not_loaded() {
    use maos_kernel_core::lifecycle::UpgradeError;
    let migrator_err = UpgradeError::MigratorNotDeclared;
    let not_loaded_err = UpgradeError::NotLoaded {
        spirit_id: "x".into(),
    };
    assert_ne!(
        format!("{migrator_err}"),
        format!("{not_loaded_err}"),
        "MigratorNotDeclared must have distinct display from NotLoaded"
    );
}

#[test]
fn upgrade_outcome_reverted_serde_roundtrip() {
    use maos_kernel_core::lifecycle::UpgradeOutcome;
    let original = UpgradeOutcome::Reverted;
    let json = serde_json::to_string(&original).unwrap();
    let back: UpgradeOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, original);
    assert_eq!(json, "\"reverted\"");
}

#[test]
fn upgrade_outcome_failed_serde_roundtrip() {
    use maos_kernel_core::lifecycle::UpgradeOutcome;
    let original = UpgradeOutcome::Failed;
    let json = serde_json::to_string(&original).unwrap();
    let back: UpgradeOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, original);
    assert_eq!(json, "\"failed\"");
}
