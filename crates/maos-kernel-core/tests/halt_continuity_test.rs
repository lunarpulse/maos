#![forbid(unsafe_code)]

//! AC5 — I14 halt-continuity unit test for `validate_halt_set`.
//!
//! Test surface:
//! - `maos_kernel_core::halt::validate_halt_set`
//! - `maos_domain::halt::HaltContinuityError::EHaltContinuityViolation`
//! - `maos_domain::halt::HaltContinuityError::MissingHaltProtocolCompatibility`

use maos_domain::halt::{HaltContinuityError, HaltId};
use maos_kernel_core::halt::validate_halt_set;

#[test]
fn validate_halt_set_empty_predecessor_succeeds_regardless_of_successor() {
    let result = validate_halt_set(&[], 1, None);
    assert!(result.is_ok(), "empty halt_set is always safe to swap");
    let result = validate_halt_set(&[], 1, Some(&[]));
    assert!(result.is_ok());
}

#[test]
fn validate_halt_set_matching_version_succeeds() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let result = validate_halt_set(&halts, 1, Some(&[1, 2]));
    assert!(result.is_ok());
}

#[test]
fn validate_halt_set_mismatched_version_returns_typed_error() {
    let halts = vec![HaltId::new("halt-1").unwrap(), HaltId::new("halt-2").unwrap()];
    let err = validate_halt_set(&halts, 1, Some(&[2, 3])).unwrap_err();
    match err {
        HaltContinuityError::EHaltContinuityViolation { predecessor, successor, orphan_count } => {
            assert_eq!(predecessor, 1);
            assert_eq!(successor, 3, "successor field is max of accepted_versions");
            assert_eq!(orphan_count, 2);
        }
        other => panic!("expected EHaltContinuityViolation, got {other:?}"),
    }
}

#[test]
fn validate_halt_set_missing_compatibility_returns_typed_error() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let err = validate_halt_set(&halts, 1, None).unwrap_err();
    assert!(matches!(err, HaltContinuityError::MissingHaltProtocolCompatibility));
}

#[test]
fn validate_halt_set_empty_accepted_versions_returns_violation_not_compatibility_missing() {
    let halts = vec![HaltId::new("halt-1").unwrap()];
    let err = validate_halt_set(&halts, 1, Some(&[])).unwrap_err();
    // accepted_versions is Some(&[]) — present but empty; this is a
    // schema mismatch (orphan_count > 0, no matching version), NOT a
    // missing-field error.
    assert!(matches!(err, HaltContinuityError::EHaltContinuityViolation { .. }));
}
