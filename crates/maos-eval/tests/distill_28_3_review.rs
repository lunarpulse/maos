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
        // Round 2 (H): blank and control-character findings are format errors.
        r#"{"finding":""}"#,
        r#"{"finding":"   "}"#,
        r#"{"finding":"\u0000"}"#,
        r#"{"finding":"line one\nline two"}"#,
    ] {
        assert!(!is_finding_completion(bad), "{bad}");
    }
}

/// Round 2 (A) — a judge reply whose attribution is absent (the provider
/// reported no serving model) or another model's is not pinned-judge evidence.
#[test]
fn a_judge_reply_without_or_with_another_attribution_is_not_pinned_judge_evidence() {
    use maos_domain::ports::inference::{
        InferenceResponse, ProviderAttribution, StopReason, TokenUsage,
    };
    let reply = |model: Option<&str>| InferenceResponse {
        text: "faithful".into(),
        stop_reason: StopReason::StopSequence,
        usage: TokenUsage {
            input_tokens: 1,
            output_tokens: 1,
        },
        provider_attribution: ProviderAttribution {
            provider_id: "anthropic".into(),
            endpoint_url: "x".into(),
            model_id: model.map(Into::into),
        },
    };
    use maos_eval::distill_replay::judge_reply_verdict;
    assert_eq!(judge_reply_verdict(&reply(Some(PIN)), PIN), Ok(true));
    let none = judge_reply_verdict(&reply(None), PIN).unwrap_err();
    assert!(none.contains("unverified"), "{none}");
    let other = judge_reply_verdict(&reply(Some("claude-haiku-5-5")), PIN).unwrap_err();
    assert!(
        other.contains("claude-haiku-5-5") && !other.contains("unverified"),
        "{other}"
    );
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
         result_sha256 = \"{ARCHIVE_SHA}\"\n\
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
        text += &format!("operator_ruling = \"{ruling}\"\nruling_sha256 = \"{RULING_SHA}\"\n");
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
    let ruled = apply(&record_text(&keep, 0.80, Some("D-28-3-03 A"))).unwrap();
    assert_eq!(ruled.recall, 0.80);
}

/// Round 3 (6c): the ruling must have the SHAPE of a decision id and the record
/// must carry the SHA-256 of the text it cites; a placeholder is refused.
#[test]
fn a_ruling_must_be_a_decision_id_with_the_hash_of_the_text_it_cites() {
    for ok in ["D-28-3-03", "D-28-3-03 A", "RULING-2026-11-25-hedge-floor"] {
        assert!(maos_eval::distill_replay::ruling_shape_ok(ok), "{ok}");
        assert!(apply(&record_text(&keep, 0.80, Some(ok))).is_ok(), "{ok}");
    }
    for bad in [
        "decisions-2026-11-25 D-x",
        "D-x",
        "D-28-3-3",
        "D-28-3-033",
        "D-28-3-03 a",
        "D-28-3-03 AB",
        "D-28-2-03",
        "RULING-2026-11-25-ab",
        "RULING-26-11-25-hedge",
        "RULING-2026-11-25-bad slug",
        "ok because I said so",
    ] {
        assert!(!maos_eval::distill_replay::ruling_shape_ok(bad), "{bad}");
        let error = apply(&record_text(&keep, 0.80, Some(bad))).unwrap_err();
        assert!(error.contains("is not a decision id"), "{bad}: {error}");
    }
    let no_hash = record_text(&keep, 0.80, Some("D-28-3-03 A"))
        .replace(&format!("ruling_sha256 = \"{RULING_SHA}\"\n"), "");
    let error = apply(&no_hash).unwrap_err();
    assert!(error.contains("needs `ruling_sha256`"), "{error}");
    let bad_hash = record_text(&keep, 0.80, Some("D-28-3-03 A")).replace(RULING_SHA, "nothex");
    assert!(apply(&bad_hash)
        .unwrap_err()
        .contains("needs `ruling_sha256`"));
    // A ruling that is not needed (floor not below its evidence) is not inspected.
    assert!(apply(&record_text(&keep, 0.93, Some("whatever"))).is_ok());
}

fn sha(bytes: &[u8]) -> String {
    maos_eval::onboarding_gate_corpus::sha256_hex(bytes)
}

/// A judge-only result line: 3 findings judged 5 times; finding a flips once,
/// b flips twice, c never (agree 4 + 3 + 5 = 12).
fn judge_line() -> serde_json::Value {
    let verdicts = |v: [bool; 5]| serde_json::json!({"k": 5, "verdicts": v});
    serde_json::json!({
        "event": "eval_distill",
        "judge_prompt_sha256": prompt_sha(),
        "judge_repeat": {
            "k": 5, "judge_calls": 15,
            "findings": {
                "a": verdicts([true, true, false, true, true]),
                "b": verdicts([true, false, false, true, true]),
                "c": verdicts([true, true, true, true, true]),
            }
        }
    })
}

/// A record whose `[judge]` evidence matches `bytes` (the archived line).
fn record_for(bytes: &[u8], agree: usize) -> DistillFloorRecord {
    let text = record_text(&keep, 0.93, None)
        .replace(ARCHIVE_SHA, &sha(bytes))
        .replace("agree = 14", &format!("agree = {agree}"));
    toml::from_str(&text).unwrap()
}

/// Round 3 (6a, 6b): the loader recomputes the evidence from the archived
/// result: hash, counts (from the verdicts, not a typed number) and judge prompt.
#[test]
fn the_loader_recomputes_judge_evidence_from_the_archive() {
    let dir = tempfile::tempdir().unwrap();
    let archived = judge_line();
    let bytes = serde_json::to_vec(&archived).unwrap();
    let rel = "spirits/researcher/eval/judge-repeat.json";
    std::fs::create_dir_all(dir.path().join("spirits/researcher/eval")).unwrap();

    // Absent: an ordinary run passes; the promoting command (`require`) does not.
    let record = record_for(&bytes, 12);
    assert!(record.verify_archive(dir.path(), false).is_ok());
    let error = record.verify_archive(dir.path(), true).unwrap_err();
    assert!(error.contains("--require-archive"), "{error}");

    // Present and true: both modes pass.
    std::fs::write(dir.path().join(rel), &bytes).unwrap();
    assert!(record.verify_archive(dir.path(), true).is_ok());

    // Invented counts (self-consistent: calls = k * findings, agree in range).
    let error = record_for(&bytes, 13)
        .verify_archive(dir.path(), false)
        .unwrap_err();
    assert!(error.contains("is not what its archive proves"), "{error}");
    // An archive edited after the record named it.
    let mut edited = archived.clone();
    edited["judge_repeat"]["findings"]["a"]["verdicts"] =
        serde_json::json!([true, true, true, true, true]);
    std::fs::write(dir.path().join(rel), serde_json::to_vec(&edited).unwrap()).unwrap();
    let error = record.verify_archive(dir.path(), false).unwrap_err();
    assert!(error.contains("result_sha256"), "{error}");
    // A record naming the wrong judge prompt.
    let mut other_prompt = archived.clone();
    other_prompt["judge_prompt_sha256"] = serde_json::json!("0".repeat(64));
    let other = serde_json::to_vec(&other_prompt).unwrap();
    std::fs::write(dir.path().join(rel), &other).unwrap();
    let error = record_for(&other, 12)
        .verify_archive(dir.path(), false)
        .unwrap_err();
    assert!(error.contains("is not what its archive proves"), "{error}");
    // Not a judge-only line / a verdict list of the wrong length / a lying judge_calls.
    let mut short = archived.clone();
    short["judge_repeat"]["findings"]["c"]["verdicts"] = serde_json::json!([true]);
    assert!(maos_eval::distill_replay::judge_repeat_counts(&short)
        .unwrap_err()
        .contains("exactly 5"));
    let mut calls = archived.clone();
    calls["judge_repeat"]["judge_calls"] = serde_json::json!(16);
    assert!(maos_eval::distill_replay::judge_repeat_counts(&calls)
        .unwrap_err()
        .contains("k * findings"));
    assert!(maos_eval::distill_replay::judge_repeat_counts(
        &serde_json::json!({"event": "eval_distill"})
    )
    .is_err());
    let counts = maos_eval::distill_replay::judge_repeat_counts(&archived).unwrap();
    assert_eq!(
        (counts.k, counts.findings, counts.calls, counts.agree),
        (5, 3, 15, 12)
    );
    // A symlink or directory in the archive's place is not an archive.
    std::fs::remove_file(dir.path().join(rel)).unwrap();
    std::fs::create_dir(dir.path().join(rel)).unwrap();
    assert!(record
        .verify_archive(dir.path(), false)
        .unwrap_err()
        .contains("regular file"));
}

/// A bad `result_sha256` is refused at load.
#[test]
fn evidence_without_a_well_formed_archive_hash_is_refused() {
    let text = record_text(&keep, 0.93, None).replace(ARCHIVE_SHA, "ABC");
    let error = apply(&text).unwrap_err();
    assert!(
        error.contains("result_sha256 must be 64 lowercase hex"),
        "{error}"
    );
}

const ARCHIVE_SHA: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const RULING_SHA: &str = "2222222222222222222222222222222222222222222222222222222222222222";

#[test]
fn the_record_carries_the_judge_model_and_agreement_or_it_is_refused() {
    let drop_table = |_: String| String::new();
    let error = apply(&record_text(&drop_table, 0.93, None)).unwrap_err();
    assert!(error.contains("judge"), "{error}");

    let no_agree = |t: String| t.replace("agree = 14\n", "");
    let error = apply(&record_text(&no_agree, 0.93, None)).unwrap_err();
    assert!(error.contains("agree"), "{error}");

    type Edit = Box<dyn Fn(String) -> String>;
    // (case, edit, the message of the check that must fire — not just ANY refusal)
    let cases: Vec<(&str, Edit, &str)> = vec![
        (
            "another model",
            Box::new(|t: String| t.replacen(PIN, "claude-haiku-5-5", 1)),
            "is not the pinned",
        ),
        (
            "another prompt",
            Box::new(|t| t.replace(&prompt_sha(), &"0".repeat(64))),
            "judge prompt_sha256",
        ),
        (
            "k below 2",
            Box::new(|t| t.replace("k = 5\nfindings", "k = 1\nfindings")),
            "one repeat establishes no agreement",
        ),
        (
            "k above the hosted bound (k 11, findings 3, calls 33, agree 33)",
            Box::new(|t| {
                t.replace(
                    "k = 5\nfindings = 3\ncalls = 15\nagree = 14",
                    "k = 11\nfindings = 3\ncalls = 33\nagree = 33",
                )
            }),
            "above the hosted bound 10",
        ),
        (
            "no findings",
            Box::new(|t| t.replace("findings = 3", "findings = 0")),
            "at least one finding",
        ),
        (
            "calls != k * findings",
            Box::new(|t| t.replace("calls = 15", "calls = 16")),
            "calls must equal k * findings",
        ),
        (
            "agree above calls",
            Box::new(|t| t.replace("agree = 14", "agree = 16")),
            "agree must be in",
        ),
        (
            "agree below findings",
            Box::new(|t| t.replace("agree = 14", "agree = 2")),
            "agree must be in",
        ),
        (
            "untraceable result_file",
            Box::new(|t| t.replace("spirits/researcher/eval/judge-repeat.json", "../x")),
            "relative archived path",
        ),
        (
            "blank rule",
            Box::new(|t| {
                t.replace(
                    "rule = \"agree = repeats equal to the finding's first verdict\"",
                    "rule = \" \"",
                )
            }),
            "needs a rule and a rationale",
        ),
    ];
    for (name, edit, fired) in cases {
        let error = apply(&record_text(&*edit, 0.93, None)).unwrap_err();
        assert!(
            error.contains(fired),
            "{name}: wanted `{fired}`, got `{error}`"
        );
    }
}

#[test]
fn no_ruling_lowers_traceability() {
    let text = record_text(&keep, 0.50, Some("D-28-3-03 A"))
        .replace("metric = \"recall\"", "metric = \"traceability\"");
    let error = apply(&text).unwrap_err();
    assert!(
        error.contains("no operator ruling lowers traceability"),
        "{error}"
    );
    // The same ruling still lowers recall (the open decision J-2 of the evidence note).
    let recall = record_text(&keep, 0.50, Some("D-28-3-03 A"));
    assert_eq!(apply(&recall).unwrap().recall, 0.50);
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
        let error = apply(&text).unwrap_err();
        assert!(
            error.contains("must be finite and within [0, 1]"),
            "{bad}: {error}"
        );
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
