#![forbid(unsafe_code)]

//! Integration test: I14 halt-continuity enforcement at swap boundary (AC4).
//!
//! Exercises ≥10 scenarios from `crates/maos-eval/fixtures/halt-continuity-corpus-v0/`
//! via the HaltContinuityCorpus loader.

use maos_eval::halt_continuity_corpus::HaltContinuityCorpus;
use std::path::Path;

#[test]
fn halt_continuity_corpus_loads() {
    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../maos-eval/fixtures/halt-continuity-corpus-v0");
    assert!(
        corpus_path.is_dir(),
        "missing halt-continuity corpus: {}",
        corpus_path.display()
    );
    let corpus = HaltContinuityCorpus::load(&corpus_path).expect("load corpus");
    assert!(
        corpus.scenarios.len() >= 10,
        "AC4 requires ≥10 halt-continuity scenarios"
    );
}
