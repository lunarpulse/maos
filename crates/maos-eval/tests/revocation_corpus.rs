#![forbid(unsafe_code)]

//! Story 5.4 — revocation corpus test driver (AC5).
//!
//! Walks all 30 scenarios in `fixtures/revocation-corpus-v0/` and asserts
//! schema validity. Full end-to-end propagation verification requires a
//! running kernel (see `crates/maos-kernel-core/tests/revocation_applier_pipeline.rs`).

use std::path::Path;

use maos_eval::RevocationCorpus;

#[test]
fn revocation_corpus_loads_all_30_scenarios() {
    let corpus = RevocationCorpus::load_from(Path::new("fixtures/revocation-corpus-v0/"))
        .expect("revocation-corpus-v0 must exist");
    assert_eq!(corpus.len(), 30, "corpus size lock — 30 scenarios (6 categories × 5 each)");
}

#[test]
fn revocation_corpus_categories_are_well_formed() {
    let corpus = RevocationCorpus::load_from(Path::new("fixtures/revocation-corpus-v0/"))
        .expect("revocation-corpus-v0 must exist");

    let valid_categories = [
        "valid_signature_immediate_terminate",
        "valid_signature_drain_then_terminate",
        "valid_signature_quarantine",
        "invalid_signature_rejected",
        "malformed_version_range_rejected",
        "trust_anchor_mismatch_rejected",
    ];

    for scenario in &corpus.scenarios {
        assert!(
            valid_categories.contains(&scenario.category.as_str()),
            "scenario {} has unknown category: {}",
            scenario.scenario_id,
            scenario.category
        );

        // Accepted scenarios must have propagation latency
        if scenario.expected_outcome.accepted {
            assert!(
                scenario.expected_outcome.propagation_latency_ms.is_some(),
                "accepted scenario {} must have propagation_latency_ms",
                scenario.scenario_id
            );
            assert_eq!(
                scenario.expected_outcome.error_variant, None,
                "accepted scenario {} must not have error_variant",
                scenario.scenario_id
            );
        } else {
            // Rejected scenarios must have error_variant
            assert!(
                scenario.expected_outcome.error_variant.is_some(),
                "rejected scenario {} must have error_variant",
                scenario.scenario_id
            );
        }
    }
}

#[test]
fn revocation_corpus_category_distribution_is_uniform() {
    let corpus = RevocationCorpus::load_from(Path::new("fixtures/revocation-corpus-v0/"))
        .expect("revocation-corpus-v0 must exist");

    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for s in &corpus.scenarios {
        *counts.entry(s.category.as_str()).or_insert(0) += 1;
    }

    assert_eq!(counts.len(), 6, "exactly 6 categories");
    for (_, count) in counts {
        assert_eq!(count, 5, "each category must have exactly 5 scenarios");
    }
}
