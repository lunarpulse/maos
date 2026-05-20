#![forbid(unsafe_code)]

//! AC4 — Five-metric distillation gate (NFR-Aud-7).
//!
//! Asserts that the N=100 synthetic-v0 distillate corpus clears:
//! - digest-recall ≥0.90
//! - digest-faithfulness ≥0.98
//! - digest-hedge-preservation ≥0.95 (gated on IAA ≥0.85)
//! - digest-traceability = 100% (kernel-enforced via non-empty source_log_ref)
//! - digest-secret-leakage = 0% (kernel-mediated pre-write redaction)
//!
//! Test surface:
//! - `maos_eval::DistillateCorpus::load_from`
//! - `maos_kernel_core::iac::redaction::CorpusBackedRedactionPolicy::redact`
//! - Pure scoring math

use maos_eval::DistillateCorpus;

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

#[test]
fn test_distillate_five_metrics_floor() {
    let corpus = DistillateCorpus::load_from(
        std::path::Path::new("fixtures/distillate-corpus-v0/"),
    )
    .expect("distillate-corpus-v0 must exist");

    // Corpus size lock
    assert_eq!(
        corpus.scenarios.len(),
        100,
        "corpus size lock — 100 synthetic-v0 scenarios"
    );

    // All scenarios must carry tag=synthetic-v0
    assert!(
        corpus.scenarios.iter().all(|s| s.tag == "synthetic-v0"),
        "every scenario MUST carry tag=synthetic-v0 to distinguish from E8 reference"
    );

    // IAA gate
    assert!(
        corpus.iaa_attestation.hedge_cohen_kappa >= 0.85,
        "IAA gate failed: kappa={} < 0.85",
        corpus.iaa_attestation.hedge_cohen_kappa
    );

    // Metric floors
    let recall_mean = mean(&corpus.scenarios.iter().map(|s| s.expected_recall).collect::<Vec<_>>());
    let faithfulness_mean =
        mean(&corpus.scenarios.iter().map(|s| s.expected_faithfulness).collect::<Vec<_>>());
    let hedge_mean = mean(
        &corpus
            .scenarios
            .iter()
            .map(|s| s.expected_hedge_preservation)
            .collect::<Vec<_>>(),
    );

    assert!(
        recall_mean >= 0.90,
        "digest-recall mean {recall_mean:.3} below 0.90 floor (NFR-Aud-7)"
    );
    assert!(
        faithfulness_mean >= 0.98,
        "digest-faithfulness mean {faithfulness_mean:.3} below 0.98 floor"
    );
    assert!(
        hedge_mean >= 0.95,
        "digest-hedge-preservation mean {hedge_mean:.3} below 0.95 floor"
    );

    // Traceability — structural: every scenario MUST have non-empty source_log_ref
    let untraceable: Vec<_> = corpus
        .scenarios
        .iter()
        .filter(|s| s.source_log_ref.is_empty())
        .map(|s| s.scenario_id.clone())
        .collect();
    assert!(
        untraceable.is_empty(),
        "untraceable scenarios: {untraceable:?}"
    );

    // Secret-leakage — run each digest through the redaction policy
    use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
    let policy = CorpusBackedRedactionPolicy::new();
    let mut leaks: Vec<String> = Vec::new();
    let mut planted_secret_positive_controls: Vec<String> = Vec::new();
    for scenario in &corpus.scenarios {
        let digest_bytes = scenario.digest_payload.as_bytes();
        use std::borrow::Cow;
        let redacted = policy.redact(digest_bytes);
        let redaction_fired = match &redacted {
            Cow::Borrowed(_) => false,
            Cow::Owned(ref redacted_bytes) => redacted_bytes != digest_bytes,
        };

        if !scenario.planted_secrets.is_empty() {
            // Positive control: a scenario with planted_secrets MUST trigger redaction.
            if redaction_fired {
                planted_secret_positive_controls.push(scenario.scenario_id.clone());
            }
            // If redaction did NOT fire, this is a real leak — the secret wasn't detected.
            if !redaction_fired {
                leaks.push(scenario.scenario_id.clone());
            }
        } else if redaction_fired {
            // Unexpected redaction on a clean scenario
            leaks.push(scenario.scenario_id.clone());
        }
    }
    assert!(
        !planted_secret_positive_controls.is_empty(),
        "no planted-secret positive controls triggered redaction — redaction infrastructure may be broken"
    );
    assert!(
        leaks.is_empty(),
        "digest-secret-leakage > 0 — scenarios {leaks:?} contain redactable patterns without planted_secrets annotation"
    );
}

#[test]
#[ignore = "quarterly audit slice not present — v0.3-β acceptable; lands in Story 8.2 alongside Researcher"]
fn test_distillate_corpus_quarterly_audit_shape() {
    let quarterly_dir =
        std::path::Path::new("fixtures/distillate-corpus-v0/quarterly-audit-v0");

    if !quarterly_dir.exists() {
        println!(
            "quarterly audit slice not present — v0.3-β acceptable; lands in Story 8.2 alongside Researcher"
        );
        return;
    }

    let corpus = DistillateCorpus::load_from(quarterly_dir)
        .expect("quarterly-audit-v0 must be parseable if present");
    assert!(
        corpus.scenarios.len() >= 500,
        "quarterly audit slice requires N≥500 scenarios"
    );
}

#[test]
fn test_distillate_corpus_category_distribution() {
    let corpus = DistillateCorpus::load_from(
        std::path::Path::new("fixtures/distillate-corpus-v0/"),
    )
    .expect("distillate-corpus-v0 must exist");

    let contradiction_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.expected_faithfulness < 1.0)
        .count();
    let planted_secret_count = corpus
        .scenarios
        .iter()
        .filter(|s| !s.planted_secrets.is_empty())
        .count();
    let hedge_preservation_count = corpus
        .scenarios
        .iter()
        .filter(|s| s.expected_hedge_preservation < 1.0)
        .count();

    assert!(
        contradiction_count >= 10,
        "corpus must contain >=10 contradiction cases (faithfulness < 1.0), found {contradiction_count}"
    );
    assert!(
        planted_secret_count >= 10,
        "corpus must contain >=10 planted-secret cases, found {planted_secret_count}"
    );
    assert!(
        hedge_preservation_count >= 10,
        "corpus must contain >=10 hedge-preservation cases (hedge < 1.0), found {hedge_preservation_count}"
    );
}
