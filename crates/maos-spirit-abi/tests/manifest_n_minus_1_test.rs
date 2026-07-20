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
    // Story 7.5a's intended posture is: at kernel version N ≥ 3, refuse
    // manifests written for N-2 (lift MIN_SUPPORTED to N-1). Story 9.4b AC-6
    // bumped MANIFEST_SCHEMA_VERSION 2→3 but **deliberately keeps MIN_SUPPORTED
    // at 1** (re-ratification: "Epic 1b baseline manifests load unchanged") —
    // so the N-2 hard-refusal floor-lift is DEFERRED to Story 7.5a and is NOT
    // yet in effect. This test pins that deferral consciously: when 7.5a lifts
    // MIN, flip this back to the strict `MIN > N-2` assertion in the same PR as
    // the STABILITY.md entry.
    if MANIFEST_SCHEMA_VERSION >= 3 {
        let n_minus_2 = MANIFEST_SCHEMA_VERSION - 2;
        assert!(
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION <= n_minus_2,
            "Story 9.4b AC-6 deferred the N-2 floor-lift: MIN_SUPPORTED ({}) is expected to \
             still admit N-2 ({}) at v1.0. If 7.5a has lifted the floor, restore the strict \
             `MIN > N-2` assertion here.",
            MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
            n_minus_2,
        );
    }
}

#[test]
fn manifest_schema_version_pinned_at_epic_6_addition_count() {
    // Schema-bump ledger (this guard fires on every bump to force a conscious
    // update alongside the governing record):
    //   1 → 2  Epic 6 §A4: [[cli_wrapper]] (6.2), [[schedules]] (6.4),
    //          [gateways]/[[gateway]] (6.5), ConsentEnvelope.intent_class +
    //          valid_until_ns (6.4).
    //   2 → 3  Story 9.4b AC-6: additive [model_provenance] section
    //          (covered_model_id, training_data_lineage [reverse-DNS, not
    //          free-text], last_eval_timestamp). Recorded as a ratified
    //          [[ratification]] in xtask/abi-ratifications.toml (ADR-045 §8).
    //   3 → 4  Story 13.5d: [capabilities.required.loom] declaration surface.
    //
    // When the next bump lands, update this assertion in the same PR as the new
    // STABILITY.md / ratification entry.
    assert_eq!(
        MANIFEST_SCHEMA_VERSION, 4,
        "MANIFEST_SCHEMA_VERSION was changed without updating this guard — \
         add the bump to the ledger above + the governing ratification/retro entry",
    );
}
