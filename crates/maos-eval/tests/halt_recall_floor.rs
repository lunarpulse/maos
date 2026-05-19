#![forbid(unsafe_code)]

//! AC6 — halt-recall ≥0.7, halt-precision ≥0.85, predicate-firing recall ≥0.85
//! against the N=62 synthetic-v0 corpus at fixtures/halt-corpus-v0/.
//!
//! Test surface:
//! - `maos_eval::HaltCorpus::load_from`
//! - Pure scoring math (TP/FP/TN/FN counters)

use maos_eval::{HaltCorpus, HaltScenarioOutcome};

fn simulate_predicate(scenario: &maos_eval::HaltScenario) -> bool {
    // For each scalar_write, apply each epistemic_policy_rule
    for write in &scenario.scalar_writes {
        for rule in &scenario.epistemic_policy_rules {
            if write.tag == rule.tag {
                match rule.rule.as_str() {
                    "on_value_above" => {
                        if write.value > rule.threshold {
                            return true;
                        }
                    }
                    "on_value_below" => {
                        if write.value < rule.threshold {
                            return true;
                        }
                    }
                    "on_value_within" => {
                        let lower = rule.lower.unwrap_or(0.0);
                        let upper = rule.upper.unwrap_or(1.0);
                        if lower <= write.value && write.value <= upper {
                            return true;
                        }
                    }
                    "on_value_outside" => {
                        let lower = rule.lower.unwrap_or(0.0);
                        let upper = rule.upper.unwrap_or(1.0);
                        if write.value < lower || write.value > upper {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    false
}

#[test]
fn test_halt_recall_floor() {
    let corpus = HaltCorpus::load_from(
        std::path::Path::new("fixtures/halt-corpus-v0/"),
    ).expect("halt-corpus-v0 must exist");
    assert_eq!(corpus.len(), 62, "corpus size lock — 62 synthetic scenarios authoritative (50 base + 12 Story 4.2 within/outside)");
    assert!(
        corpus.scenarios.iter().all(|s| s.tag == "synthetic-v0"),
        "every scenario MUST carry tag=synthetic-v0 to distinguish from E8 reference"
    );

    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut tn = 0usize;
    let mut fn_count = 0usize;
    let mut predicate_expected = 0usize;
    let mut predicate_fired = 0usize;
    let mut failing_scenarios: Vec<String> = Vec::new();

    for scenario in &corpus.scenarios {
        let predicate_fires = simulate_predicate(scenario);

        match scenario.ground_truth_class {
            HaltScenarioOutcome::TruePositive => {
                if predicate_fires {
                    tp += 1;
                    predicate_fired += 1;
                } else {
                    fn_count += 1;
                    failing_scenarios.push(scenario.scenario_id.clone());
                }
                predicate_expected += 1;
            }
            HaltScenarioOutcome::TrueNegative => {
                if !predicate_fires {
                    tn += 1;
                } else {
                    fp += 1;
                    failing_scenarios.push(scenario.scenario_id.clone());
                }
            }
            HaltScenarioOutcome::FalsePositive => {
                if predicate_fires {
                    fp += 1;
                } else {
                    tn += 1;
                }
            }
            HaltScenarioOutcome::FalseNegative => {
                if !predicate_fires {
                    fn_count += 1;
                } else {
                    tp += 1;
                }
                predicate_expected += 1;
            }
        }
    }

    let recall = if tp + fn_count == 0 { 0.0 } else { tp as f64 / (tp + fn_count) as f64 };
    let precision = if tp + fp == 0 { 0.0 } else { tp as f64 / (tp + fp) as f64 };
    let predicate_recall = predicate_fired as f64 / predicate_expected.max(1) as f64;

    assert!(
        recall >= 0.7,
        "halt-recall {recall:.3} below 0.7 floor; failing scenarios: {failing_scenarios:?}"
    );
    assert!(
        precision >= 0.85,
        "halt-precision {precision:.3} below 0.85 floor; failing scenarios: {failing_scenarios:?}"
    );
    assert!(
        predicate_recall >= 0.85,
        "predicate-firing recall {predicate_recall:.3} below 0.85 floor (FR32)"
    );
}
