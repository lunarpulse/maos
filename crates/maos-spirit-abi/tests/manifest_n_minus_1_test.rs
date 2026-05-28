//! Story 7.5a precursor — manifest schema version N-1 supported floor invariants.
//!
//! Authored under Epic 6 §A4 (retro 2026-05-28) to pin the constant window
//! that Story 7.5a's ABI Stability Triple — `(kernel_version, abi_version,
//! manifest_schema_version)` — will consume verbatim.
//!
//! These tests verify the **kernel-side commitments**:
//!
//! 1. `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` ≤ `MANIFEST_SCHEMA_VERSION`
//!    (the "current" version cannot be below the floor — would deny v_current).
//! 2. `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` ≥ `MANIFEST_SCHEMA_VERSION`
//!    (the kernel must accept what it emits).
//! 3. The N-1 floor is in effect — `MIN_SUPPORTED` ≤ `MANIFEST_SCHEMA_VERSION - 1`
//!    whenever `MANIFEST_SCHEMA_VERSION ≥ 2`. This is the load-bearing
//!    backward-compat commitment Story 7.5a will publish in STABILITY.md.
//! 4. The N-2 hard refusal posture — `MIN_SUPPORTED` > `MANIFEST_SCHEMA_VERSION - 2`
//!    whenever `MANIFEST_SCHEMA_VERSION ≥ 3`. At v0.5-α this assertion is vacuous
//!    (current = 2), but the assertion form is the v1.0 contract shape.
//!
//! The cross-crate "v1 manifest loads on v2 kernel" path is exercised in
//! `crates/maos-manifest/tests/manifest_n_minus_1_compat.rs` — splitting the
//! surfaces avoids a `maos-spirit-abi → maos-manifest` dev-dep cycle.

use maos_spirit_abi::{
    MANIFEST_SCHEMA_VERSION, MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
};

#[test]
fn min_supported_does_not_exceed_current() {
    assert!(
        MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION <= MANIFEST_SCHEMA_VERSION,
        "MIN_SUPPORTED ({}) > MANIFEST_SCHEMA_VERSION ({}) — kernel would refuse the version it emits",
        MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
        MANIFEST_SCHEMA_VERSION,
    );
}

#[test]
fn max_supported_admits_current() {
    assert!(
        MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION >= MANIFEST_SCHEMA_VERSION,
        "MAX_SUPPORTED ({}) < MANIFEST_SCHEMA_VERSION ({}) — kernel would refuse the version it emits",
        MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
        MANIFEST_SCHEMA_VERSION,
    );
}

#[test]
fn n_minus_1_supported_floor_in_effect() {
    // For any kernel at version N ≥ 2, the kernel MUST accept manifests
    // written for N-1. This is the Story 7.5a "N-1 supported" floor.
    if MANIFEST_SCHEMA_VERSION >= 2 {
        let n_minus_1 = MANIFEST_SCHEMA_VERSION - 1;
        assert!(
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION <= n_minus_1,
            "MIN_SUPPORTED ({}) > N-1 ({}) — Story 7.5a N-1 supported floor violated; v_{} manifests would be refused",
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
            n_minus_1,
            n_minus_1,
        );
    }
}

#[test]
fn n_minus_2_hard_refusal_posture() {
    // For any kernel at version N ≥ 3, the kernel MUST refuse manifests
    // written for N-2 (the hard-refusal floor). At v0.5-α (current = 2) this
    // assertion is vacuously true; encoding it now pins the contract shape
    // Story 7.5a will publish in STABILITY.md.
    if MANIFEST_SCHEMA_VERSION >= 3 {
        let n_minus_2 = MANIFEST_SCHEMA_VERSION - 2;
        assert!(
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION > n_minus_2,
            "MIN_SUPPORTED ({}) ≤ N-2 ({}) — Story 7.5a N-2 hard-refusal floor violated; v_{} manifests would be silently admitted",
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
            n_minus_2,
            n_minus_2,
        );
    }
}

#[test]
fn manifest_schema_version_pinned_at_epic_6_addition_count() {
    // Epic 6 added exactly four manifest sections: [[cli_wrapper]] (6.2),
    // [[schedules]] (6.4), [gateways] / [[gateway]] (6.5), and the
    // ConsentEnvelope.intent_class + valid_until_ns additive fields (6.4).
    // The version bump from 1 → 2 corresponds to that single Epic 6 boundary.
    //
    // This test guards against a silent re-bump without an accompanying retro
    // entry — when the next bump lands, this assertion must be updated in
    // the same PR as the new STABILITY.md entry.
    assert_eq!(
        MANIFEST_SCHEMA_VERSION, 2,
        "MANIFEST_SCHEMA_VERSION was changed without updating this guard — \
         confirm Epic 6 §A4 retro is amended OR new retro entry exists",
    );
}
