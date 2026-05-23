//! NFR-Sec-14 cross-Spirit isolation 200-corpus integration test (AC2).
//!
//! Loads the full 200-scenario corpus, runs all scenarios through
//! `IsolationCorpusRunner`, and asserts 200/200 isolation maintained.
//! ANY leak is a P0 ship-block.
//!
//! Category-level coverage: aggregate ≥25 scenarios per category
//! across the Sec-14a/Sec-14b split.

use std::path::Path;

use maos_eval::isolation_corpus::serde_variant::to_snake_case;
use maos_eval::isolation_corpus::{IsolationAttackCategory, IsolationCorpus};
use maos_kernel_core::isolation::IsolationCorpusRunner;

#[test]
fn nfr_sec_14_200_scenarios_zero_leaks() {
    let corpus_path = Path::new("../maos-eval/fixtures/isolation-corpus-v0/");
    assert!(
        corpus_path.exists(),
        "NFR-Sec-14 P0 ship-block: isolation corpus fixture not found at {} — CI must fail-loud",
        corpus_path.display()
    );

    let corpus = IsolationCorpus::load_from(corpus_path)
        .expect("isolation-corpus-v0 must exist and be valid");

    assert_eq!(corpus.total(), 200, "NFR-Sec-14 floor: 200 scenarios exact");
    assert_eq!(corpus.count_split("sec-14a"), 100);
    assert_eq!(corpus.count_split("sec-14b"), 100);

    let runner = IsolationCorpusRunner::new(corpus);
    let report = runner
        .run_all()
        .expect("NFR-Sec-14 floor: 200/200 isolation maintained — ANY leak is a P0 ship-block");

    assert_eq!(
        report.scenarios_passed + report.scenarios_deferred,
        200,
        "all 200 scenarios must pass or be explicitly deferred"
    );
    assert_eq!(
        report.scenarios_with_breach, 0,
        "P0 ship-block: any breach fails CI"
    );

    // Category-level coverage assertion: aggregate ≥25 per category
    for category in IsolationAttackCategory::all() {
        let cat_str = to_snake_case(category);
        let count = report.per_category.get(cat_str).copied().unwrap_or(0);
        assert!(
            count >= 25,
            "category {:?} below 25-scenario aggregate floor: got {}",
            category,
            count
        );
    }
}
