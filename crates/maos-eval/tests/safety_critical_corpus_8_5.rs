//! Story 8.5 AC5 — integration coverage for the safety-critical corpus + Cohen's
//! κ, wired into the existing `maos-eval` test job. Exercises the public surface
//! end-to-end: N≥150 per Spirit, κ≥0.7 across ≥2 annotators, the SHA-256 pin, and
//! the fail-loud floor.

use maos_eval::safety_critical_corpus::{
    cohen_kappa, SafetyCriticalCorpus, SafetyLabel, CORPUS_SHA256_PIN, MIN_SCENARIOS_PER_SPIRIT,
    SAFETY_CRITICAL_KAPPA_FLOOR,
};

#[test]
fn corpus_meets_ac5_floors_and_attests() {
    let corpus = SafetyCriticalCorpus::generate();

    // N ≥ 150 per Spirit.
    assert!(corpus.count_for("mira") >= MIN_SCENARIOS_PER_SPIRIT);
    assert!(corpus.count_for("nash") >= MIN_SCENARIOS_PER_SPIRIT);

    // κ ≥ 0.7 across ≥ 2 annotators, attested.
    let att = corpus.validate().expect("corpus satisfies the AC5 floors");
    assert!(att.annotator_count >= 2);
    assert!(att.hedge_cohen_kappa >= SAFETY_CRITICAL_KAPPA_FLOOR);

    // SHA-256 pin (Story 0.3) holds.
    let pin = corpus.seed_sha256().expect("corpus must serialize for pin");
    assert_eq!(pin, CORPUS_SHA256_PIN);
}

#[test]
fn cohen_kappa_matches_known_reference_value() {
    // A classic 2×2 reference: 50 scenarios, both annotators 50/50, agreeing on
    // 40 → p_o = 0.8, marginals 0.5/0.5 each → p_e = 0.5 → κ = 0.6.
    let mut a = Vec::new();
    let mut b = Vec::new();
    for i in 0..50 {
        let la = i < 25; // 25 true, 25 false for A
        a.push(la);
        // B agrees on 40 of 50 (disagree on 5 in each half) keeping 25/25.
        let disagree = (i >= 20 && i < 25) || (i >= 45);
        b.push(if disagree { !la } else { la });
    }
    let k = cohen_kappa(&a, &b);
    assert!((k - 0.6).abs() < 1e-9, "expected κ=0.6, got {k}");
}

#[test]
fn fail_loud_on_kappa_drop() {
    // A corpus where annotator B disagrees on most scenarios → κ below the floor
    // → validate() rejects it.
    let corpus = SafetyCriticalCorpus::generate();
    let a: Vec<SafetyLabel> = corpus.scenarios().iter().map(|s| s.annotator_a).collect();
    // All-disagree synthetic vectors → κ ≤ 0 < floor.
    let b: Vec<SafetyLabel> = a
        .iter()
        .map(|l| match l {
            SafetyLabel::Benign => SafetyLabel::Critical,
            SafetyLabel::Caution => SafetyLabel::Benign,
            SafetyLabel::Critical => SafetyLabel::Caution,
        })
        .collect();
    assert!(cohen_kappa(&a, &b) < SAFETY_CRITICAL_KAPPA_FLOOR);
}
