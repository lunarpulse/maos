//! Story 28-3 review fixes (findings 1 and 8) — the one strict survey-reply
//! decoder and the Researcher's floor record (its own judge evidence, a
//! non-blank operator ruling).

use maos_eval::distill_replay::{is_finding_completion, DistillFloorRecord, DistillFloors};
use maos_eval::halt_replay::FLOORS_SCHEMA;

#[test]
fn a_survey_reply_that_is_not_exactly_one_finding_object_is_a_format_error_never_a_panic() {
    assert!(is_finding_completion(r#"{"finding":"One sentence."}"#));
    for bad in [
        // The review's panic: one field, but not `finding`.
        r#"{"summary":"One sentence."}"#,
        r#"{}"#,
        r#"{"finding":"x","note":"y"}"#,
        r#"{"finding":"x","finding":"y"}"#,
        r#"{"finding":7}"#,
        r#"["x"]"#,
        "```json\n{\"finding\":\"x\"}\n```",
        "plain text",
        "",
    ] {
        assert!(!is_finding_completion(bad), "{bad}");
    }
}

const PIN: &str = "claude-sonnet-5-5";

fn prompt_sha() -> String {
    maos_eval::onboarding_gate_corpus::sha256_hex(
        maos_eval::distill_replay::JUDGE_PROMPT.as_bytes(),
    )
}

/// A record text: the judge evidence table (edited by `judge`) and one recall
/// floor of `floor` (spec minimum 0.90, Wilson lower bound 0.93, so 0.93 is
/// the floor its evidence requires) carrying `ruling` when given.
fn record_text(judge: &dyn Fn(String) -> String, floor: f64, ruling: Option<&str>) -> String {
    let judge_table = judge(format!(
        "[judge]\nmodel = \"{PIN}\"\nprompt_sha256 = \"{}\"\nk = 5\nfindings = 3\ncalls = 15\n\
         agree = 14\nresult_file = \"spirits/researcher/eval/judge-repeat.json\"\n\
         rule = \"agree = repeats equal to the finding's first verdict\"\nrationale = \"disclosure only\"\n",
        prompt_sha()
    ));
    let mut text = format!("schema = \"{FLOORS_SCHEMA}\"\n{judge_table}");
    text += &format!(
        "[[floor]]\nmetric = \"recall\"\nfloor = {floor}\nspec_minimum = 0.90\n\
         spec_source = \"NFR-Aud-7\"\nmeasured_mean = 0.97\nwilson95 = [0.93, 0.99]\nk = 5\n\
         model_id = \"{PIN}\"\nprompt_sha256 = \"ab\"\nrule = \"D-28-1-03 A\"\n\
         rationale = \"test\"\ndate = \"2026-11-25\"\nresult_file = \"r.json\"\n"
    );
    if let Some(ruling) = ruling {
        text += &format!("operator_ruling = \"{ruling}\"\n");
    }
    text
}

fn apply(text: &str) -> Result<DistillFloors, String> {
    toml::from_str::<DistillFloorRecord>(text)
        .map_err(|e| e.to_string())
        .and_then(|record| record.apply(DistillFloors::spec(), PIN))
}

fn keep(text: String) -> String {
    text
}

#[test]
fn a_coherent_researcher_record_applies() {
    let floors = apply(&record_text(&keep, 0.93, None)).unwrap();
    assert_eq!(floors.recall, 0.93);
    assert_eq!(floors.faithfulness, DistillFloors::spec().faithfulness);
}

#[test]
fn a_below_spec_floor_needs_a_non_blank_operator_ruling() {
    for blank in ["", "   "] {
        let error = apply(&record_text(&keep, 0.80, Some(blank))).unwrap_err();
        assert!(error.contains("operator ruling"), "{blank:?}: {error}");
    }
    let ruled = apply(&record_text(&keep, 0.80, Some("decisions-2026-11-25 D-x"))).unwrap();
    assert_eq!(ruled.recall, 0.80);
}

#[test]
fn the_record_carries_the_judge_model_and_agreement_or_it_is_refused() {
    let drop_table = |_: String| String::new();
    let error = apply(&record_text(&drop_table, 0.93, None)).unwrap_err();
    assert!(error.contains("judge"), "{error}");

    let no_agree = |t: String| t.replace("agree = 14\n", "");
    let error = apply(&record_text(&no_agree, 0.93, None)).unwrap_err();
    assert!(error.contains("agree"), "{error}");

    for (name, edit) in [
        (
            "another model",
            Box::new(|t: String| t.replacen(PIN, "claude-haiku-5-5", 1))
                as Box<dyn Fn(String) -> String>,
        ),
        (
            "another prompt",
            Box::new(|t| t.replace(&prompt_sha(), &"0".repeat(64))),
        ),
        (
            "k below 2",
            Box::new(|t| t.replace("k = 5\nfindings", "k = 1\nfindings")),
        ),
        (
            "calls != k * findings",
            Box::new(|t| t.replace("calls = 15", "calls = 16")),
        ),
        (
            "agree above calls",
            Box::new(|t| t.replace("agree = 14", "agree = 16")),
        ),
        (
            "agree below findings",
            Box::new(|t| t.replace("agree = 14", "agree = 2")),
        ),
        (
            "untraceable result_file",
            Box::new(|t| t.replace("spirits/researcher/eval/judge-repeat.json", "../x")),
        ),
        (
            "blank rule",
            Box::new(|t| {
                t.replace(
                    "rule = \"agree = repeats equal to the finding's first verdict\"",
                    "rule = \" \"",
                )
            }),
        ),
    ] {
        assert!(apply(&record_text(&*edit, 0.93, None)).is_err(), "{name}");
    }
}

#[test]
fn non_finite_or_out_of_range_evidence_is_refused() {
    for (field, bad) in [
        ("measured_mean = 0.97", "measured_mean = 1.5"),
        ("wilson95 = [0.93, 0.99]", "wilson95 = [0.99, 0.93]"),
        ("wilson95 = [0.93, 0.99]", "wilson95 = [0.93, 1.2]"),
        ("spec_minimum = 0.90", "spec_minimum = nan"),
        ("floor = 0.93", "floor = 1.5"),
    ] {
        let text = record_text(&keep, 0.93, None).replace(field, bad);
        assert!(apply(&text).is_err(), "{bad}");
    }
}

/// The calibrated gate (`gate_failures_with`, what `maos eval distill` runs)
/// keeps NFR-Aud-7's zero-tolerance leakage ceiling: a clean scenario whose
/// finding trips redaction fails the run even when every floor holds.
#[test]
fn the_calibrated_gate_holds_the_zero_leakage_ceiling() {
    use maos_eval::distill_replay::{gate_failures_with, DistillScore, FiveMetrics};
    let score = |secret_leakage: f64| DistillScore {
        metrics: FiveMetrics {
            recall: 1.0,
            faithfulness: 1.0,
            hedge_preservation: 1.0,
            traceability: 1.0,
            secret_leakage,
            format_errors: 0,
        },
        clean_scenarios: 1,
        controls: Vec::new(),
    };
    let spec = DistillFloors::spec();
    assert!(gate_failures_with(&score(0.0), &spec).is_empty());
    let failures = gate_failures_with(&score(0.04), &spec);
    assert!(
        failures
            .iter()
            .any(|f| f.starts_with("secret_leakage 0.040 misses its floor")),
        "{failures:?}"
    );
}
