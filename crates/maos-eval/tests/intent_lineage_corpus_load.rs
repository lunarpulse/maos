//! Story 6.3 Task 6 — confirm the lineage corpus loads with the
//! 20 NEW cross-Host scenarios added in Story 6.3 (51..70).
//!
//! The Story 6.2 NFR-Aud-14 100% coverage gate at
//! `crates/maos-kernel-core/tests/intent_lineage_corpus_v0.rs` exercises
//! the runtime path (which currently has a pre-existing
//! `init_monotonic_base()` panic — Epic 5/6 carry-forward unrelated to
//! Story 6.3); this companion test runs the LOADER-ONLY path so the new
//! 20 scenarios are mechanically validated as additive.

use maos_eval::intent_lineage_corpus::{IntentLineageClass, IntentLineageCorpus};
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("fixtures/intent-lineage-corpus-v0")
}

#[test]
fn story_6_3_lineage_corpus_loads_70_scenarios_including_20_a2a() {
    let dir = corpus_dir();
    let corpus = IntentLineageCorpus::load_from(&dir).expect("corpus load");
    assert!(
        corpus.len() >= 70,
        "expected at least 70 scenarios (50 Story 6.2 + 20 Story 6.3 A2A); got {}",
        corpus.len()
    );

    // Verify the new 20 scenarios are in the corpus by scenario_id prefix.
    let a2a_loopback_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.scenario_id.contains("lineage_via_a2a_loopback"))
        .count();
    let a2a_cross_host_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.scenario_id.contains("lineage_via_a2a_cross_host"))
        .count();
    assert_eq!(a2a_loopback_count, 10, "expected 10 lineage_via_a2a_loopback scenarios");
    assert_eq!(a2a_cross_host_count, 10, "expected 10 lineage_via_a2a_cross_host scenarios");
}

#[test]
fn story_6_3_a2a_scenarios_use_existing_class_for_additive_compat() {
    // Per Story 6.3 Task 6 §dev-notes — the new 20 scenarios use the existing
    // `LineageChainUninterrupted` class so the enum doesn't need new variants
    // (additive-only ABI; no `#[non_exhaustive]` required).
    let corpus = IntentLineageCorpus::load_from(&corpus_dir()).expect("load");
    for s in corpus.scenarios.iter().filter(|s| {
        s.scenario_id.contains("lineage_via_a2a")
    }) {
        assert_eq!(
            s.class,
            IntentLineageClass::LineageChainUninterrupted,
            "A2A scenario {} must use LineageChainUninterrupted class",
            s.scenario_id
        );
    }
}

#[test]
fn story_6_3_a2a_scenarios_assert_accept_with_non_empty_lineage() {
    let corpus = IntentLineageCorpus::load_from(&corpus_dir()).expect("load");
    for s in corpus.scenarios.iter().filter(|s| {
        s.scenario_id.contains("lineage_via_a2a")
    }) {
        assert!(s.expected_outcome.accepted, "{}: must be accept-path", s.scenario_id);
        assert!(
            !s.expected_outcome.expected_lineage_intents.is_empty(),
            "{}: must have non-empty expected lineage",
            s.scenario_id
        );
    }
}

/// Story 6.4 Task 6 — confirm the lineage corpus extends to 100 scenarios
/// with three new lineage-class buckets (consent_rupture / rate_limited /
/// on_schedule).
#[test]
fn story_6_4_lineage_corpus_extends_to_100_scenarios() {
    let dir = corpus_dir();
    let corpus = IntentLineageCorpus::load_from(&dir).expect("corpus load");
    assert!(
        corpus.len() >= 100,
        "expected at least 100 scenarios; got {}",
        corpus.len()
    );

    let consent_rupture_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.scenario_id.contains("lineage_via_consent_rupture"))
        .count();
    let rate_limited_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.scenario_id.contains("lineage_via_rate_limited"))
        .count();
    let on_schedule_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.scenario_id.contains("lineage_via_on_schedule"))
        .count();
    assert_eq!(
        consent_rupture_count, 10,
        "expected 10 lineage_via_consent_rupture scenarios"
    );
    assert_eq!(
        rate_limited_count, 10,
        "expected 10 lineage_via_rate_limited scenarios"
    );
    assert_eq!(
        on_schedule_count, 10,
        "expected 10 lineage_via_on_schedule scenarios"
    );
}

#[test]
fn story_6_4_new_scenarios_use_existing_class() {
    let corpus = IntentLineageCorpus::load_from(&corpus_dir()).expect("load");
    for s in corpus.scenarios.iter().filter(|s| {
        s.scenario_id.contains("lineage_via_consent_rupture")
            || s.scenario_id.contains("lineage_via_rate_limited")
            || s.scenario_id.contains("lineage_via_on_schedule")
    }) {
        assert_eq!(
            s.class,
            IntentLineageClass::LineageChainUninterrupted,
            "{}: must use LineageChainUninterrupted class",
            s.scenario_id
        );
    }
}
