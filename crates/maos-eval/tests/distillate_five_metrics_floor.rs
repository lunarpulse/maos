#![forbid(unsafe_code)]

//! AC4 — Five-metric distillation gate (NFR-Aud-7), Story 18-3 (D-18-3-06 A).
//!
//! The per-commit gate no longer averages hand-authored labels: it scores
//! what the Researcher EMITTED over the frozen corpus
//! (`spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl`, the frozen
//! survey of each row) with `maos_eval::distill_replay` — recall (source
//! claims present as findings), hedge preservation (each source hedge in the
//! finding's statement TEXT), traceability (every cite is a journaled frame),
//! secret leakage (the real `CorpusBackedRedactionPolicy`) and faithfulness
//! (frozen judge verdicts, provenance-labelled). Every negative control must
//! miss the metric it plants. `maos eval distill --class researcher` replays
//! the same rows through the kernel and must reproduce each frozen survey.
//!
//! The NFR-Aud-8 quarterly N=500 slice below keeps its annotated means
//! (non-goal of 18-3).

use std::collections::BTreeSet;

use maos_eval::distill_replay::{
    frame_id, gate_failures, judge_verdict, keeps_hedge, load_rows, score, stale_judge_prompts,
    Control, FindingFacts, JudgeConfig, ScenarioFacts, JUDGE_PROMPT,
};
use maos_eval::onboarding_gate_corpus::sha256_hex;
use maos_eval::DistillateCorpus;
use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

#[derive(Debug, serde::Deserialize)]
struct Claim {
    claim_id: String,
    hedges: Vec<String>,
}

/// A cite as its 16 bytes (`researcher::encode_frame_id_hex` groups by `:`).
fn cite(hex: &str) -> Option<Vec<u8>> {
    let clean: String = hex.chars().filter(|c| *c != ':').collect();
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(clean.get(i..i + 2)?, 16).ok())
        .collect()
}

#[test]
fn test_distillate_five_metrics_floor() {
    let text =
        std::fs::read_to_string("../../spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl")
            .expect("the frozen Researcher corpus");
    let rows = load_rows::<Claim>(&text).expect("corpus rows parse");
    let policy = CorpusBackedRedactionPolicy::new();
    let facts: Vec<ScenarioFacts> = rows
        .iter()
        .enumerate()
        .map(|(s, row)| {
            let journaled: BTreeSet<Vec<u8>> = row
                .claims
                .iter()
                .enumerate()
                .filter(|(_, c)| !row.unjournaled.contains(&c.claim_id))
                .map(|(c, _)| frame_id(s, c).to_vec())
                .collect();
            let findings = row.survey["findings"]
                .as_array()
                .expect("frozen findings")
                .iter()
                .map(|f| {
                    let statement = f["statement"].as_str().unwrap().to_owned();
                    FindingFacts {
                        claim_id: f["claim_id"].as_str().unwrap().to_owned(),
                        hedges: serde_json::from_value(f["hedges"].clone()).unwrap(),
                        resolves: cite(f["source_log_ref"].as_str().unwrap())
                            .is_some_and(|id| journaled.contains(&id)),
                        leaks: policy.redact(statement.as_bytes()).as_ref() != statement.as_bytes(),
                        statement,
                    }
                })
                .collect();
            ScenarioFacts {
                scenario_id: row.scenario_id.clone(),
                control: row.control,
                source: row
                    .claims
                    .iter()
                    .map(|c| (c.claim_id.clone(), c.hedges.clone()))
                    .collect(),
                findings,
                verdicts: row.judge.verdicts.clone(),
            }
        })
        .collect();

    let score = score(&facts).expect("the corpus scores");
    assert!(score.clean_scenarios >= 12, "{score:?}");
    let planted: BTreeSet<_> = score
        .controls
        .iter()
        .map(|c| format!("{:?}", c.control))
        .collect();
    for control in [
        Control::HedgeDrop,
        Control::FabricatedCite,
        Control::PlantedSecret,
        Control::DroppedClaim,
        Control::UnfaithfulFinding,
    ] {
        assert!(
            planted.contains(&format!("{control:?}")),
            "missing control {control:?}"
        );
    }
    assert_eq!(score.controls.len(), 5, "{score:?}");
    // TIA-4: the faithfulness control is the control rows' frozen `false`
    // verdicts — and control rows never enter a floor.
    let unfaithful = score
        .controls
        .iter()
        .find(|c| c.control == Control::UnfaithfulFinding)
        .expect("the faithfulness control");
    assert_eq!(
        unfaithful.scenario_id,
        "rd13-hedge-drop,rd15-planted-secret"
    );
    assert_eq!(score.metrics.faithfulness, 1.0, "{score:?}");
    let failures = gate_failures(&score);
    assert!(
        failures.is_empty(),
        "NFR-Aud-7 five-metric gate: {failures:?}\n{score:?}"
    );
    assert!(
        rows.iter()
            .all(|r| r.judge.provenance == "seed" || r.judge.provenance == "live-record"),
        "every frozen verdict carries its provenance"
    );
}

/// AA-2 — the judge prompt the scorer sends is the one both pins name.
#[test]
fn the_judge_config_and_the_manifest_pin_the_judge_prompt() {
    let prompt = sha256_hex(JUDGE_PROMPT.as_bytes());
    let config: JudgeConfig =
        toml::from_str(&std::fs::read_to_string("../../tests/judge-config.toml").unwrap())
            .expect("judge config");
    let judge = &config.judge["researcher-faithfulness"];
    assert_eq!(judge.prompt_version_hash, prompt);
    assert_eq!(config.wire_model().unwrap(), "claude-sonnet-5-5");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string("../../tests/corpora/MANIFEST.toml").unwrap())
            .expect("corpora manifest");
    assert_eq!(
        manifest["corpus"]["researcher-distill-corpus-v0"]["prompt_version_hash"].as_str(),
        Some(prompt.as_str())
    );
    // Every frozen row pins this prompt; a row judged under another is refused.
    let text =
        std::fs::read_to_string("../../spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl")
            .expect("the frozen Researcher corpus");
    let mut rows = load_rows::<Claim>(&text).expect("corpus rows parse");
    assert!(stale_judge_prompts(&rows).is_empty());
    rows[0].judge.prompt_sha256 = "ab".repeat(32);
    assert_eq!(
        stale_judge_prompts(&rows),
        [format!(
            "{}: frozen verdicts pin a different judge prompt",
            rows[0].scenario_id
        )]
    );
}

/// EC-6 — hedges match on word boundaries, case-insensitively.
#[test]
fn hedges_match_whole_words_only() {
    assert!(keeps_hedge("Retrieval is Likely to help.", "likely"));
    assert!(keeps_hedge(
        "It may over-weight authors.",
        "may over-weight"
    ));
    assert!(keeps_hedge("most", "most"));
    assert!(!keeps_hedge("That is unlikely to help.", "likely"));
    assert!(!keeps_hedge("It almost always helps.", "most"));
    assert!(!keeps_hedge("It mostly helps.", "most"));
    assert!(!keeps_hedge("It couldn't help.", "could"));
}

/// EC-6 — an empty (or blank) hedge is refused at corpus load.
#[test]
fn a_blank_hedge_is_refused_at_load() {
    for hedge in ["", "  "] {
        let row = serde_json::json!({
            "scenario_id": "x",
            "claims": [{ "claim_id": "x-c1", "hedges": [hedge] }],
            "survey": {},
            "judge": { "prompt_sha256": "", "provenance": "seed", "verdicts": {} },
        });
        let error = load_rows::<Claim>(&row.to_string()).unwrap_err();
        assert!(error.contains("empty hedge"), "{error}");
    }
}

/// BH-4 — a verdict word survives surrounding punctuation and markup.
#[test]
fn judge_verdicts_strip_non_letters_around_the_word() {
    for text in ["faithful", "Faithful.", "**faithful**", "`faithful`\n"] {
        assert_eq!(judge_verdict(text), Ok(true), "{text}");
    }
    for text in ["unfaithful", "UNFAITHFUL!", " *unfaithful* "] {
        assert_eq!(judge_verdict(text), Ok(false), "{text}");
    }
    for text in ["", "faithful enough", "not faithful"] {
        assert!(judge_verdict(text).is_err(), "{text}");
    }
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
        "quarterly digest-recall mean {recall_mean:.3} below 0.90"
    );
    assert!(
        faithfulness_mean >= 0.98,
        "quarterly digest-faithfulness mean {faithfulness_mean:.3} below 0.98"
    );
    assert!(
        hedge_mean >= 0.95,
        "quarterly digest-hedge mean {hedge_mean:.3} below 0.95"
    );

    // Traceability — every scenario carries a non-empty source_log_ref.
    assert!(
        corpus
            .scenarios
            .iter()
            .all(|s| !s.source_log_ref.is_empty()),
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
    let contradiction = corpus
        .scenarios
        .iter()
        .filter(|s| s.expected_faithfulness < 0.99)
        .count();
    let planted = corpus
        .scenarios
        .iter()
        .filter(|s| !s.planted_secrets.is_empty())
        .count();
    let hedge = corpus
        .scenarios
        .iter()
        .filter(|s| s.expected_hedge_preservation < 0.97)
        .count();
    assert!(
        contradiction >= 10,
        "quarterly needs ≥10 contradiction cases, got {contradiction}"
    );
    assert!(
        planted >= 10,
        "quarterly needs ≥10 planted-secret cases, got {planted}"
    );
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
