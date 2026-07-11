//! Story 11.5 FKCS contract tests — literal AC3 admission + itemized scoring.
//!
//! Literal AC3 (user-selected): the negative-control fourth Spirit references
//! an off-frozen-surface / `pub(crate)`-style internal and is rejected by the
//! REAL `maos_registry::admission::admit_spirit` with a journaled/falsifiable
//! `OffFrozenSurface` verdict — NOT only by the out-of-band `FrozenSymbolGate`.
//! These tests assert that admission behavior and the itemized 27/30 + 85/90
//! scoring boundaries; they deliberately do NOT assert `FrozenSymbolGate` as the
//! AC3 mechanism.

use maos_fkcs::{
    AdmissionHarness, ChecklistCategory, KernelFreezeProvenance, ProxyCohort, ProxySpirit,
    SpiritChecklistReport, FKCS_AGGREGATE_FLOOR, FKCS_PER_SPIRIT_FLOOR,
};

/// The off-frozen-surface `pub(crate)`-style internal the negative control
/// "requires". This is a real kernel-core symbol that is NOT on the frozen ABI
/// or host public surface, so a conformance check must reject it.
const OFF_SURFACE_INTERNAL: &str = "maos_kernel_core::scheduler::pick_next_spirit_from_slice";

// ---------------------------------------------------------------------------
// Literal AC3 — admission rejection at the real `admit_spirit`.
// ---------------------------------------------------------------------------

#[test]
fn negative_control_rejects_at_real_admit_spirit_while_conformance_proxy_admits() {
    let harness = AdmissionHarness::default();
    let proxy = ProxySpirit::conformance("proxy-echo-a");
    let negative = ProxySpirit::negative_control("negative-internal-ref", OFF_SURFACE_INTERNAL);

    // Conformance proxy admits on the REAL admit_spirit path.
    let admitted = harness.admit(&proxy);
    assert!(
        admitted.admitted,
        "conformance proxy must admit on the real admit_spirit path"
    );
    assert!(
        admitted.journaled,
        "an admission decision must be journaled/auditable, not merely returned inline"
    );

    // Literal AC3: the negative control is rejected by the REAL admit_spirit —
    // not by FrozenSymbolGate, and not by an always-admit stub.
    let rejected = harness.admit(&negative);
    assert!(
        !rejected.admitted,
        "negative control must be rejected at real admit_spirit (literal AC3)"
    );
    assert!(
        rejected.journaled,
        "the rejection must be journaled/auditable so it is falsifiable from the audit trail"
    );
    assert!(
        rejected.reason.contains("off-frozen-surface"),
        "rejection reason must name the off-frozen-surface violation; got: {}",
        rejected.reason
    );
    assert!(
        rejected.reason.contains("FKCS"),
        "rejection reason must carry the FKCS tag; got: {}",
        rejected.reason
    );
}

/// §A7.1 anti-canned reflex: an always-admit (blind) harness ADMITS the
/// negative control, proving the rejection above is bound to the genuine
/// `OffFrozenSurface` arm of `admit_spirit` rather than a canned assertion. A
/// stubbed admission that bypassed the surface check would admit here and red
/// the negative-control leg — this is the admission-falsifier under the new
/// design.
#[test]
fn always_admit_blind_harness_proves_the_rejection_is_not_canned() {
    let negative = ProxySpirit::negative_control("negative-internal-ref", OFF_SURFACE_INTERNAL);

    // The blind twin bypasses admit_spirit entirely and always admits.
    let blind = AdmissionHarness::always_admit_for_test();
    let blind_result = blind.admit(&negative);
    assert!(
        blind_result.admitted,
        "the blind always-admit harness must admit the negative control (falsifier baseline)"
    );

    // The real harness rejects exactly where the blind one admits.
    let real = AdmissionHarness::default();
    let real_result = real.admit(&negative);
    assert!(
        !real_result.admitted,
        "the real harness must reject where the blind harness admits — a stub would red the leg"
    );
    assert_ne!(
        blind_result.admitted, real_result.admitted,
        "blind and real paths must disagree on the negative control"
    );
}

// ---------------------------------------------------------------------------
// Itemized scoring — green cohort, per-spirit / aggregate floor boundaries.
// ---------------------------------------------------------------------------

#[test]
fn cohort_green_reconciles_every_spirit_and_clears_the_aggregate_floor() {
    let harness = AdmissionHarness::default();
    let cohort = ProxyCohort::new(vec![
        ProxySpirit::conformance("proxy-echo-a"),
        ProxySpirit::conformance("proxy-identity-b"),
        ProxySpirit::conformance("proxy-wasm-c"),
    ]);
    let stable = KernelFreezeProvenance::stable_at(23_081);

    let report = cohort.evaluate(&harness, &stable);

    assert_eq!(report.cohort_label, "in-house Chinese-wall proxy");
    assert_eq!(report.total_spirits, 3);
    assert_eq!(report.admitted_count, 3);
    assert_eq!(
        report.reconciled_count, 3,
        "green freeze: every admitted conformance spirit fully reconciles"
    );
    assert_eq!(
        report.max_aggregate_score, 90,
        "max aggregate = total_spirits * 30"
    );
    // A fully-green conformance proxy scores EXACTLY 30 (itemized, not flat).
    assert!(
        report.per_spirit_scores.iter().all(|s| *s == 30),
        "green conformance scores 30/30 per spirit; got {:?}",
        report.per_spirit_scores
    );
    assert_eq!(report.aggregate_score, 90);
    assert!(report.clears_aggregate_floor());
    assert!(report.floor_is_advisory_for_proxy_cohort);
    assert!(!report.is_na);
}

#[test]
fn cohort_checklist_is_itemized_across_categories_not_a_flat_bucket() {
    let harness = AdmissionHarness::default();
    let cohort = ProxyCohort::new(vec![ProxySpirit::conformance("proxy-echo-a")]);
    let stable = KernelFreezeProvenance::stable_at(23_081);

    let report = cohort.evaluate(&harness, &stable);
    let spirit = &report.per_spirit[0];

    assert_eq!(spirit.score, SpiritChecklistReport::MAX_SCORE);
    assert_eq!(
        spirit.items.len(),
        SpiritChecklistReport::MAX_SCORE as usize,
        "checklist is exactly 30 named, independently-falsifiable items"
    );
    assert!(spirit.admitted);
    assert!(!spirit.negative_control);
    assert!(spirit.reconciled(), "a 30/30 spirit is reconciled");

    // The checklist spans multiple distinct categories — not a flat single
    // bucket. (ChecklistCategory is Eq, not Ord, so assert named-category
    // presence rather than collecting into an ordered set.)
    let has_category = |cat| spirit.items.iter().any(|item| item.category == cat);
    assert!(has_category(ChecklistCategory::AbiSymbolCoverage));
    assert!(has_category(ChecklistCategory::AuditInvariant));
    assert!(has_category(ChecklistCategory::ComplianceClaimVerify));
    assert!(
        spirit
            .items
            .iter()
            .any(|item| item.category != spirit.items[0].category),
        "an itemized checklist must span multiple categories, not a flat bucket"
    );
    assert!(
        spirit.items.iter().all(|item| item.passed),
        "a green conformance proxy passes every checklist item"
    );

    // The AuditInvariant category tracks the three freeze sub-derivations and is
    // fully green under a stable freeze.
    let audit = spirit.items_in(ChecklistCategory::AuditInvariant);
    assert!(!audit.is_empty(), "AuditInvariant category must be present");
    assert_eq!(
        spirit.passed_in(ChecklistCategory::AuditInvariant),
        audit.len() as u32,
        "stable freeze passes every audit item"
    );
}

/// Oracle RED path: a freeze with a single drifted axis drops `reconciled_count`
/// to zero while `admitted_count` holds at 3 — the two axes are independent.
/// Each spirit loses exactly one audit point (30 -> 29); the aggregate (87)
/// still clears the 85 floor.
#[test]
fn cohort_one_freeze_axis_red_drops_reconciled_count_while_admission_holds() {
    let harness = AdmissionHarness::default();
    let cohort = ProxyCohort::new(vec![
        ProxySpirit::conformance("proxy-echo-a"),
        ProxySpirit::conformance("proxy-identity-b"),
        ProxySpirit::conformance("proxy-wasm-c"),
    ]);
    // Lines drift 23081 -> 23082; abi additive + host allowlist still hold.
    let one_axis_red = KernelFreezeProvenance::from_measure(23_081, 23_082, true, true);
    assert!(
        !one_axis_red.frozen(),
        "fixture: a line drift makes the freeze RED"
    );
    assert!(!one_axis_red.line_stable());

    let report = cohort.evaluate(&harness, &one_axis_red);

    assert_eq!(
        report.admitted_count, 3,
        "admission count must hold across a freeze RED (the axes are independent)"
    );
    assert_eq!(
        report.reconciled_count, 0,
        "a RED freeze drops reconciled_count even though all spirits still admit"
    );
    assert!(
        report.per_spirit_scores.iter().all(|s| *s == 29),
        "one drifted freeze axis costs exactly one audit item (30 -> 29); got {:?}",
        report.per_spirit_scores
    );
    assert_eq!(report.aggregate_score, 87);
    assert!(
        report.clears_aggregate_floor(),
        "87 >= 85: a single-axis-red cohort still clears the aggregate floor"
    );
}

/// Per-spirit floor vs aggregate floor: when ALL three freeze axes are RED each
/// spirit bottoms out at the 27 floor, yet the 3-spirit aggregate (81) FAILS the
/// 85 aggregate floor. This makes 27 and 85 two distinct, meaningful thresholds
/// (3 x 27 = 81 < 85) rather than trivially-coupled bounds.
#[test]
fn cohort_all_freeze_axes_red_hits_per_spirit_floor_but_fails_aggregate_floor() {
    let harness = AdmissionHarness::default();
    let cohort = ProxyCohort::new(vec![
        ProxySpirit::conformance("proxy-echo-a"),
        ProxySpirit::conformance("proxy-identity-b"),
        ProxySpirit::conformance("proxy-wasm-c"),
    ]);
    let all_axes_red = KernelFreezeProvenance::from_measure(23_081, 23_082, false, false);
    assert!(!all_axes_red.frozen());

    let report = cohort.evaluate(&harness, &all_axes_red);

    assert!(
        report
            .per_spirit_scores
            .iter()
            .all(|s| *s == FKCS_PER_SPIRIT_FLOOR),
        "each spirit bottoms out at the per-spirit floor {} when every freeze axis is red; got {:?}",
        FKCS_PER_SPIRIT_FLOOR,
        report.per_spirit_scores
    );
    assert_eq!(report.aggregate_score, 81, "3 x 27 = 81");
    assert!(
        !report.clears_aggregate_floor(),
        "81 < {}: a cohort can hit every per-spirit floor yet fail the aggregate floor",
        FKCS_AGGREGATE_FLOOR
    );
    assert_eq!(
        report.reconciled_count, 0,
        "no spirit reconciles under a fully-red freeze"
    );
}

/// A negative-control spirit in the cohort scores well below the per-spirit
/// floor and drags the aggregate under the floor — proving the floor is
/// enforced and a stubbed/cooked cohort cannot vacuously pass.
#[test]
fn cohort_with_negative_control_scores_below_floor_and_fails_aggregate() {
    let harness = AdmissionHarness::default();
    let cohort = ProxyCohort::new(vec![
        ProxySpirit::conformance("proxy-echo-a"),
        ProxySpirit::negative_control("negative-internal-ref", OFF_SURFACE_INTERNAL),
    ]);
    let stable = KernelFreezeProvenance::stable_at(23_081);

    let report = cohort.evaluate(&harness, &stable);

    assert_eq!(
        report.admitted_count, 1,
        "only the conformance proxy admits"
    );
    assert_eq!(
        report.reconciled_count, 1,
        "only the conformance proxy reconciles"
    );

    let negative_score = report
        .per_spirit
        .iter()
        .find(|s| s.negative_control)
        .expect("negative control is present in the cohort")
        .score;
    assert!(
        negative_score < FKCS_PER_SPIRIT_FLOOR,
        "the rejected negative control must score below the per-spirit floor; got {negative_score}"
    );
    assert!(
        report
            .per_spirit_scores
            .iter()
            .any(|s| *s < FKCS_PER_SPIRIT_FLOOR),
        "at least one spirit must fall below the floor"
    );
    assert!(
        !report.clears_aggregate_floor(),
        "a cohort containing a sub-floor spirit must fail the aggregate floor"
    );
}

/// Empty cohort is N/A — never a vacuous pass.
#[test]
fn empty_cohort_is_na_never_a_vacuous_pass() {
    let harness = AdmissionHarness::default();
    let stable = KernelFreezeProvenance::stable_at(23_081);
    let empty = ProxyCohort::new(Vec::new()).evaluate(&harness, &stable);

    assert!(empty.is_na, "an empty cohort is N/A, never a vacuous pass");
    assert_eq!(empty.admitted_count, 0);
    assert_eq!(empty.reconciled_count, 0);
    assert_eq!(empty.aggregate_score, 0);
}
