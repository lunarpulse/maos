#![forbid(unsafe_code)]

//! Story 5.4 — upgrade-policy corpus test driver (AC1).
//!
//! Walks all 20 scenarios in `fixtures/upgrade-policy-corpus-v0/` and asserts
//! schema validity. Full end-to-end upgrade verification requires a running
//! kernel (see `crates/maos-kernel-core/tests/upgrade_orchestrator_three_policies.rs`).

use std::path::Path;

use maos_eval::UpgradePolicyCorpus;

#[test]
fn upgrade_policy_corpus_loads_all_20_scenarios() {
    let corpus =
        UpgradePolicyCorpus::load_from(Path::new("fixtures/upgrade-policy-corpus-v0/"))
            .expect("upgrade-policy-corpus-v0 must exist");
    assert_eq!(
        corpus.len(),
        20,
        "corpus size lock — 20 scenarios (4 categories × 5 each)"
    );
}

#[test]
fn upgrade_policy_corpus_categories_are_well_formed() {
    let corpus =
        UpgradePolicyCorpus::load_from(Path::new("fixtures/upgrade-policy-corpus-v0/"))
            .expect("upgrade-policy-corpus-v0 must exist");

    let valid_policies = ["hot_swap", "cold_swap", "migrator", "migrator"];
    let mut policy_idx = 0usize;

    for scenario in &corpus.scenarios {
        assert!(
            ["hot_swap_success", "cold_swap_success", "migrator_success", "policy_mismatch"]
                .contains(&scenario.category.as_str()),
            "scenario {} has unknown category: {}",
            scenario.scenario_id,
            scenario.category
        );

        let expected_policy = match scenario.category.as_str() {
            "hot_swap_success" => "hot-swap",
            "cold_swap_success" => "cold-swap",
            "migrator_success" | "policy_mismatch" => "migrator",
            other => panic!("unexpected category: {other}"),
        };
        assert_eq!(
            scenario.policy, expected_policy,
            "scenario {} policy mismatch",
            scenario.scenario_id
        );

        let expected_outcome = match scenario.category.as_str() {
            "hot_swap_success" | "cold_swap_success" | "migrator_success" => "completed",
            "policy_mismatch" => "failed",
            other => panic!("unexpected category: {other}"),
        };
        assert_eq!(
            scenario.expected_outcome.report_outcome, expected_outcome,
            "scenario {} outcome mismatch",
            scenario.scenario_id
        );
    }
}

#[test]
fn upgrade_policy_corpus_category_distribution_is_uniform() {
    let corpus =
        UpgradePolicyCorpus::load_from(Path::new("fixtures/upgrade-policy-corpus-v0/"))
            .expect("upgrade-policy-corpus-v0 must exist");

    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for s in &corpus.scenarios {
        *counts.entry(s.category.as_str()).or_insert(0) += 1;
    }

    assert_eq!(counts.len(), 4, "exactly 4 categories");
    for (_, count) in counts {
        assert_eq!(count, 5, "each category must have exactly 5 scenarios");
    }
}
