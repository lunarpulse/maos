#![forbid(unsafe_code)]

use std::path::Path;

use maos_eval::T3EscapeCorpus;

fn skip_if_no_container_runtime() -> bool {
    for bin in ["/usr/bin/podman", "/usr/bin/docker"] {
        if std::process::Command::new(bin)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return false;
        }
    }
    true
}

#[test]
fn t3_escape_corpus_loads_all_25_scenarios() {
    let corpus = T3EscapeCorpus::load_from(Path::new(
        "fixtures/t3-escape-corpus-v0/",
    ))
    .expect("t3-escape-corpus-v0 must exist");
    assert_eq!(
        corpus.scenarios.len(),
        25,
        "corpus size lock — 25 scenarios (5 categories × 5 each)"
    );
}

#[test]
fn t3_escape_corpus_categories_are_well_formed() {
    let corpus = T3EscapeCorpus::load_from(Path::new(
        "fixtures/t3-escape-corpus-v0/",
    ))
    .expect("t3-escape-corpus-v0 must exist");

    let valid_categories = [
        "filesystem_escape",
        "network_escape",
        "process_escape",
        "capability_escape",
        "runtime_escape",
    ];

    for scenario in &corpus.scenarios {
        assert!(
            valid_categories.contains(&scenario.category.as_str()),
            "scenario {} has unknown category: {}",
            scenario.scenario_id,
            scenario.category
        );
        assert!(
            !scenario.attack_payload.command.is_empty(),
            "scenario {} must have non-empty attack command",
            scenario.scenario_id
        );
        assert!(
            scenario.expected_outcome.block_observed,
            "scenario {} must expect block_observed=true",
            scenario.scenario_id
        );
        assert_eq!(
            scenario.tier_target, "T3",
            "scenario {} must target T3",
            scenario.scenario_id
        );
        assert_eq!(
            scenario.split, "sec-14a",
            "scenario {} must be in sec-14a split",
            scenario.scenario_id
        );
    }
}

#[test]
fn t3_escape_corpus_each_category_has_five_scenarios() {
    let corpus = T3EscapeCorpus::load_from(Path::new(
        "fixtures/t3-escape-corpus-v0/",
    ))
    .expect("t3-escape-corpus-v0 must exist");

    let categories = [
        "filesystem_escape",
        "network_escape",
        "process_escape",
        "capability_escape",
        "runtime_escape",
    ];

    for cat in &categories {
        let count = corpus
            .scenarios
            .iter()
            .filter(|s| s.category == *cat)
            .count();
        assert_eq!(
            count, 5,
            "category {cat} must have exactly 5 scenarios, got {count}"
        );
    }
}

#[test]
fn t3_escape_corpus_all_scenarios_unique_ids() {
    let corpus = T3EscapeCorpus::load_from(Path::new(
        "fixtures/t3-escape-corpus-v0/",
    ))
    .expect("t3-escape-corpus-v0 must exist");

    let mut ids: Vec<&str> = corpus.scenarios.iter().map(|s| s.scenario_id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        corpus.scenarios.len(),
        "all scenario IDs must be unique"
    );
}
