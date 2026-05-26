#![forbid(unsafe_code)]

//! Story 6.1 — retract corpus test driver (AC2).
//!
//! Walks all 30 scenarios in `fixtures/retract-corpus-v0/` and asserts
//! schema validity + category distribution.

use std::path::Path;

use maos_eval::RetractCorpus;

#[test]
fn retract_corpus_loads_all_30_scenarios() {
    let corpus = RetractCorpus::load_from(Path::new("fixtures/retract-corpus-v0/"))
        .expect("retract-corpus-v0 must exist");
    assert_eq!(corpus.len(), 30, "corpus size lock — 30 scenarios (10 before-delivery / 10 after-delivery / 5 authority-violation / 5 idempotent)");
}

#[test]
fn retract_corpus_categories_are_well_formed() {
    let corpus = RetractCorpus::load_from(Path::new("fixtures/retract-corpus-v0/"))
        .expect("retract-corpus-v0 must exist");

    let valid_categories = [
        "before_delivery",
        "after_delivery",
        "authority_violation",
        "idempotent",
    ];

    for scenario in &corpus.scenarios {
        assert!(
            valid_categories.contains(&scenario.category.as_str()),
            "scenario {} has unknown category: {}",
            scenario.scenario_id,
            scenario.category
        );

        // Validate expected outcome structure
        if scenario.expected_outcome.success {
            assert!(
                scenario.expected_outcome.error_variant.is_none(),
                "successful scenario {} must not have error_variant",
                scenario.scenario_id
            );
        } else {
            assert!(
                scenario.expected_outcome.error_variant.is_some(),
                "failed scenario {} must have error_variant",
                scenario.scenario_id
            );
        }
    }
}

#[test]
fn retract_corpus_category_distribution_is_uniform() {
    let corpus = RetractCorpus::load_from(Path::new("fixtures/retract-corpus-v0/"))
        .expect("retract-corpus-v0 must exist");

    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for s in &corpus.scenarios {
        *counts.entry(s.category.as_str()).or_insert(0) += 1;
    }

    assert_eq!(counts.len(), 4, "exactly 4 categories");
    assert_eq!(counts.get("before_delivery").copied().unwrap_or(0), 10, "before_delivery must have 10 scenarios");
    assert_eq!(counts.get("after_delivery").copied().unwrap_or(0), 10, "after_delivery must have 10 scenarios");
    assert_eq!(counts.get("authority_violation").copied().unwrap_or(0), 5, "authority_violation must have 5 scenarios");
    assert_eq!(counts.get("idempotent").copied().unwrap_or(0), 5, "idempotent must have 5 scenarios");
}
