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
    let corpus =
        DistillateCorpus::load_from(std::path::Path::new("fixtures/distillate-corpus-v0/"))
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
    let recall_mean = mean(
        &corpus
            .scenarios
            .iter()
            .map(|s| s.expected_recall)
            .collect::<Vec<_>>(),
    );
    let faithfulness_mean = mean(
        &corpus
            .scenarios
            .iter()
            .map(|s| s.expected_faithfulness)
            .collect::<Vec<_>>(),
    );
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

/// NFR-Aud-8 — the quarterly N≥500 audit slice (Story 8.2, AC5). The `#[ignore]`
/// is REMOVED now that `quarterly-audit-v0/` is authored (generated
/// deterministically by `quarterly-audit-v0/generate.py` — NFR-Testability-1).
/// The same five floors that gate the N=100 slice hold on the N=500 slice.
#[test]
fn test_distillate_corpus_quarterly_audit_shape() {
    let quarterly_dir = std::path::Path::new("fixtures/distillate-corpus-v0/quarterly-audit-v0");

    let corpus = DistillateCorpus::load_from(quarterly_dir)
        .expect("quarterly-audit-v0 must exist and be parseable (Story 8.2)");

    // N≥500 lock + tag.
    assert!(
        corpus.scenarios.len() >= 500,
        "quarterly audit slice requires N≥500 scenarios, found {}",
        corpus.scenarios.len()
    );
    assert!(
        corpus.scenarios.iter().all(|s| s.tag == "quarterly-v0"),
        "every quarterly scenario MUST carry tag=quarterly-v0"
    );

    // IAA gate.
    assert!(
        corpus.iaa_attestation.hedge_cohen_kappa >= 0.85,
        "quarterly IAA gate failed: kappa={} < 0.85",
        corpus.iaa_attestation.hedge_cohen_kappa
    );

    // The three annotated metric floors hold on the N=500 slice (as on N=100).
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
    assert!(recall_mean >= 0.90, "quarterly digest-recall mean {recall_mean:.3} below 0.90");
    assert!(
        faithfulness_mean >= 0.98,
        "quarterly digest-faithfulness mean {faithfulness_mean:.3} below 0.98"
    );
    assert!(hedge_mean >= 0.95, "quarterly digest-hedge mean {hedge_mean:.3} below 0.95");

    // Traceability — every scenario carries a non-empty source_log_ref.
    assert!(
        corpus.scenarios.iter().all(|s| !s.source_log_ref.is_empty()),
        "quarterly traceability: every scenario MUST have a non-empty source_log_ref"
    );

    // Secret-leakage — planted-secret digests MUST fire redaction (positive
    // control); clean digests MUST NOT.
    use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
    use std::borrow::Cow;
    let policy = CorpusBackedRedactionPolicy::new();
    let mut leaks: Vec<String> = Vec::new();
    let mut positive_controls = 0usize;
    for scenario in &corpus.scenarios {
        let payload = scenario.digest_payload.as_bytes();
        let redacted = policy.redact(payload);
        // Defensive: require both allocation AND content change to count as fired.
        let fired = matches!(&redacted, Cow::Owned(b) if b != payload);
        if scenario.planted_secrets.is_empty() {
            if fired {
                leaks.push(scenario.scenario_id.clone());
            }
        } else if fired {
            positive_controls += 1;
        } else {
            leaks.push(scenario.scenario_id.clone());
        }
    }
    assert!(
        positive_controls >= 10,
        "quarterly corpus needs ≥10 planted-secret positive controls that fire redaction, got {positive_controls}"
    );
    assert!(
        leaks.is_empty(),
        "quarterly digest-secret-leakage > 0 — scenarios {leaks:?}"
    );

    // Category distribution — ≥10 each of contradiction / planted-secret / hedge.
    // Thresholds must isolate the actual category bands (see generate.py):
    //   typical: faith=0.995, hedge=0.97
    //   hedge-focus: faith=0.995, hedge=0.950..0.960
    //   contradiction: faith=0.980..0.985, hedge=0.97
    //   planted-secret: faith=0.995, hedge=0.97
    let contradiction = corpus.scenarios.iter().filter(|s| s.expected_faithfulness < 0.99).count();
    let planted = corpus.scenarios.iter().filter(|s| !s.planted_secrets.is_empty()).count();
    let hedge = corpus.scenarios.iter().filter(|s| s.expected_hedge_preservation < 0.97).count();
    assert!(contradiction >= 10, "quarterly needs ≥10 contradiction cases, got {contradiction}");
    assert!(planted >= 10, "quarterly needs ≥10 planted-secret cases, got {planted}");
    assert!(hedge >= 10, "quarterly needs ≥10 hedge cases, got {hedge}");
}

#[test]
fn test_distillate_corpus_category_distribution() {
    let corpus =
        DistillateCorpus::load_from(std::path::Path::new("fixtures/distillate-corpus-v0/"))
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
