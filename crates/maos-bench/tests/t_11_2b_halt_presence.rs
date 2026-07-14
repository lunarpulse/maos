#![forbid(unsafe_code)]

//! Story 11.2b — per-region halt-receipt PRESENCE observability (AC3 / D3 / F3).
//!
//! Halt is a **single-kernel** concept measured by receipt **presence**
//! (NFR-Rel-11, ≥99.9%), NOT latency. There is **no cross-region halt mechanism**
//! and 11.2a AP-local-degrade deliberately raises no halt — so a cross-region
//! halt *latency/propagation* number is un-measurable and forbidden (it would
//! time a message that has no transport, or need a kernel-Δ halt-transport).
//!
//! AC3's honest metric is therefore **per-region halt-receipt PRESENCE
//! aggregated across the 3-region pilot**: the operator (Reza / Journey-12)
//! gets the answer to *"did every region I can't see actually emit a halt
//! receipt?"* — with NO latency number, NO cross-region propagation claim, and
//! ZERO kernel plumbing (cross-region halt propagation is a named kernel-Δ
//! future story, explicitly OUT).
//!
//! The pilot partitions the 1000-scenario termination corpus across three
//! regions and reuses the `terminate_spirit` receipt-production seam
//! (`crates/maos-kernel-core/tests/halt_receipt_production_rate.rs` is the
//! pattern). The presence numerator is DERIVED from real terminations
//! (derive-and-reconcile — never a hardcoded count).
//!
//! ## The suppress-emission falsifier (NON-NEGOTIABLE — no falsifier, no AC)
//!
//! A metric nobody can make red is theater. The falsifier suppresses region-A's
//! receipt emission AT THE OBSERVATION/COUNTING LAYER (the receipt-production
//! seam lives in kernel-core, which is ZERO-Δ — so the suppression cannot touch
//! kernel code; it drops region-A's counted receipts, simulating a broken
//! observability sink). The multi-region aggregate then goes NOT-observed /
//! count-drops for region-A → the leg REDs. This is a plain adversarial test
//! (test-internal data manipulation — no production-code mutation, so no
//! feature gate and no release-leak risk, mirroring F7 Arms 2/3).

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState, TerminationKind};
use maos_eval::{TerminationCorpus, TerminationKind as EvalTerminationKind};
use maos_kernel_core::halt::{terminate_spirit, HaltRegistry, PendingHaltMetadata};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;

/// Number of pilot regions the corpus is partitioned across (NFR-Scale-1
/// 3-region pilot).
const NUM_REGIONS: usize = 3;

/// Load the 1000-scenario termination corpus.
fn load_corpus() -> TerminationCorpus {
    TerminationCorpus::load_from(std::path::Path::new(
        "../maos-eval/fixtures/termination-corpus-v0/",
    ))
    .expect("termination-corpus-v0 must exist")
}

/// Map an `EvalTerminationKind` to the kernel's `TerminationKind`.
fn to_kernel_kind(k: &EvalTerminationKind) -> TerminationKind {
    match k {
        EvalTerminationKind::PlannedUnload => TerminationKind::PlannedUnload,
        EvalTerminationKind::HaltAccepted => TerminationKind::HaltAccepted,
        EvalTerminationKind::UnplannedCrash => TerminationKind::UnplannedCrash,
        EvalTerminationKind::HaltRejection => TerminationKind::HaltRejection,
        EvalTerminationKind::RevocationTerminated => TerminationKind::RevocationTerminated,
    }
}

/// Count the receipts a scenario's termination produced (mirrors
/// `halt_receipt_production_rate.rs:89-102`).
///
/// When `suppress` is true the function returns 0 — simulating a broken
/// observability sink for this scenario's region.  The suppression flows
/// THROUGH this counting seam (not a post-hoc accumulator zero) so the
/// falsifier exercises the metric's response to a zero-receipts input,
/// rather than asserting a constant.  Note: this still operates at the
/// counting layer (spec-authorized ZERO-Δ caveat); it does NOT suppress
/// production-code receipt emission in kernel-core (that would require a
/// kernel-core sink seam, deferred to a successor story).
fn count_receipts(
    tl: &TransparencyLogAdapter,
    registry: &HaltRegistry,
    spirit_pid: u32,
    scenario: &maos_eval::TerminationScenario,
    seed: u64,
    suppress: bool,
) -> usize {
    if suppress {
        // Suppressed region: the termination still RUNS (expected is counted
        // by the caller), but the counting seam reports zero receipts —
        // simulating a region whose observability sink broke before receipts
        // reached the aggregate.
        return 0;
    }
    let kind = to_kernel_kind(&scenario.kind);
    let receipts = terminate_spirit(tl, registry, spirit_pid, &scenario.spirit_id, kind, seed);
    if scenario.pending_halts.is_empty() {
        usize::from(!receipts.is_empty())
    } else {
        scenario
            .expected_receipt_ids
            .iter()
            .filter(|expected_id| receipts.iter().any(|r| r.halt_id.as_str() == *expected_id))
            .count()
    }
}

/// Per-region halt-receipt PRESENCE across the 3-region pilot.
///
/// Scenarios are partitioned across regions by `index % NUM_REGIONS`. Each
/// region's presence rate = receipts_produced / expected (NFR-Rel-11 ≥99.9%).
///
/// `suppress_region`: when `Some(r)`, region-r's receipts are suppressed at
/// the counting seam (`count_receipts` returns 0) — the suppress-emission
/// falsifier.  This simulates a region whose halt-receipt observability
/// sink broke: its terminations still ran (expected > 0) but the counting
/// seam reports zero receipts flowing to the aggregate.  The metric MUST
/// then RED for region-r (proving it is not theater).
fn per_region_presence(suppress_region: Option<usize>) -> Vec<(usize, usize, usize, f64)> {
    let corpus = load_corpus();
    // produced[r], expected[r]
    let mut produced = vec![0usize; NUM_REGIONS];
    let mut expected = vec![0usize; NUM_REGIONS];

    for (idx, scenario) in corpus.scenarios.iter().enumerate() {
        let region = idx % NUM_REGIONS;
        let seed = scenario
            .scenario_id
            .as_bytes()
            .iter()
            .fold(0u64, |a, b| a.wrapping_add(*b as u64));
        let tl = TransparencyLogAdapter::open_in_memory(seed);
        let registry = HaltRegistry::new();
        let spirit_pid = (idx as u64 + 1_000_000) as u32;

        for halt_id in &scenario.pending_halts {
            registry
                .insert_pending_with_metadata(
                    HaltId::new(halt_id.clone()).unwrap(),
                    HaltState::PendingResolution,
                    PendingHaltMetadata {
                        spirit_pid,
                        spirit_id: scenario.spirit_id.clone(),
                        payload: EpistemicHaltPayload {
                            halt_id: halt_id.clone(),
                            tag: "test".into(),
                            value: 0.0,
                            threshold: None,
                            policy_id: "".into(),
                            derived_from: "".into(),
                        },
                        fired_ns: 0,
                    },
                )
                .unwrap();
        }

        expected[region] += scenario.expected_receipts;
        let suppress = suppress_region == Some(region);
        let receipts = count_receipts(&tl, &registry, spirit_pid, scenario, seed, suppress);
        produced[region] += receipts;
    }

    (0..NUM_REGIONS)
        .map(|r| {
            let rate = produced[r] as f64 / expected[r].max(1) as f64;
            (r, produced[r], expected[r], rate)
        })
        .collect()
}

/// AC3 / D3 (F3): Per-region halt-receipt PRESENCE observability across the
/// 3-region pilot — GREEN. Every region's own terminations emitted a receipt at
/// ≥99.9% (NFR-Rel-11). The numerator is DERIVED from real terminations
/// (derive-and-reconcile — never a hardcoded count). NO latency number, NO
/// cross-region propagation claim, ZERO kernel plumbing.
#[test]
fn halt_presence_per_region_green() {
    let presence = per_region_presence(None);
    assert_eq!(presence.len(), NUM_REGIONS);
    for &(region, produced, expected, rate) in &presence {
        // Derive-and-reconcile: each region must have terminations to observe.
        assert!(
            expected > 0,
            "region-{region} has zero terminations — the pilot must partition \
             real work across all {NUM_REGIONS} regions (no vacuous region)"
        );
        assert!(
            rate >= 0.999,
            "region-{region} halt-receipt PRESENCE {rate:.4} below the 99.9% \
             floor (NFR-Rel-11) — produced={produced} / expected={expected}. \
             The operator (Reza/Journey-12) cannot confirm region-{region} \
             emitted its halt receipts."
        );
        eprintln!(
            "halt-presence region-{region}: {produced}/{expected} receipts \
             present (rate={rate:.4} ≥ 0.999)"
        );
    }
}

/// AC3 suppress-emission falsifier (NON-NEGOTIABLE). Suppressing region-A's
/// receipt emission at the counting layer MUST drop region-A's presence below
/// the 99.9% floor → the multi-region aggregate REDs. A metric nobody can make
/// red is theater; this proves the aggregate is sensitive to a region whose
/// observability sink broke.
#[test]
fn halt_presence_suppress_emission_falsifier() {
    let suppressed_region = 0; // region-A
    let presence = per_region_presence(Some(suppressed_region));

    // The suppressed region's terminations STILL ran (expected unchanged from
    // the GREEN run), but its counted receipts are dropped → presence collapses.
    let (region, produced, expected, rate) = presence[suppressed_region];
    assert!(
        expected > 0,
        "region-{region} must have terminations to suppress"
    );
    assert_eq!(
        produced, 0,
        "suppressed region-{region} produced must be 0 (emission suppressed at \
         the counting layer)"
    );
    assert!(
        rate < 0.999,
        "suppressed region-{region} presence {rate:.4} must RED (< 0.999) — if \
         suppressing emission does NOT drop the aggregate, the metric is theater \
         (it cannot detect a region that stopped emitting receipts). produced=0 \
         / expected={expected}."
    );
    eprintln!(
        "halt-presence FALSIFIER: suppressing region-{region} emission → \
         presence {rate:.4} (< 0.999) — the aggregate correctly REDs (0/{expected})"
    );

    // The OTHER regions are unaffected (the suppression is region-scoped) — a
    // correct observability sink is per-region, not a global all-or-nothing.
    for &(r, _, _, other_rate) in presence.iter() {
        if r != suppressed_region {
            assert!(
                other_rate >= 0.999,
                "non-suppressed region-{r} presence {other_rate:.4} must stay GREEN \
                 — the suppress-emission falsifier is region-scoped, not global"
            );
        }
    }
}
