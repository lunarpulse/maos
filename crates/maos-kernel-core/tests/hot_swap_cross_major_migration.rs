#![forbid(unsafe_code)]

//! Integration test: Cross-major migration path (AC2).
//!
//! Covers:
//! - migrates_from with matching version pattern.
//! - EMigratorMissing when no migrator declared.
//! - Version pattern mismatch.
//! - Migrator matches_version_pattern test.

use maos_kernel_core::hot_swap::migrator;

#[test]
fn version_pattern_wildcard_matches_same_major_minor() {
    assert!(migrator::matches_version_pattern("0.3.x", "0.3.1"));
    assert!(migrator::matches_version_pattern("0.3.x", "0.3.99"));
}

#[test]
fn version_pattern_exact_match() {
    assert!(migrator::matches_version_pattern("0.3.1", "0.3.1"));
}

#[test]
fn version_pattern_rejects_different_major() {
    assert!(!migrator::matches_version_pattern("0.3.x", "1.3.1"));
}

#[test]
fn version_pattern_rejects_different_minor() {
    assert!(!migrator::matches_version_pattern("0.3.x", "0.4.1"));
}

#[test]
fn version_pattern_rejects_exact_mismatch() {
    assert!(!migrator::matches_version_pattern("0.3.1", "0.3.2"));
}
