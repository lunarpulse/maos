//! Story 28-1 (AC1, AC6, AC7, AC8) — `xtask eval-report` on the archived live
//! run 43708370483.
//!
//! Provenance of `tests/fixtures/eval-report/` (evidence archive
//! `2026-11-15/live-rerecord-run-43708370483-{cassettes,job.log}`, outside git):
//! - `halt-corpus-v0.3.live.json`, `distill-corpus-v0.live.json`: the live
//!   cassettes, byte for byte (the result lines carry their SHA-256);
//! - `butler.k1.results.jsonl`: job-log line 4466 (the Butler eval line) plus
//!   the two keys the 28-1 `maos eval halt` line adds, `verdicts` (a row halts
//!   iff it is an expected halt or the recorded completion's
//!   `user_preference_drift` is below the manifest's 0.6 — the 15 true and the
//!   8 false positives of the ruling record) and `format_errors: []`;
//! - `butler.k3.results.jsonl`: three lines (cassette names `….live.<i>.json`;
//!   the test copies the one cassette under those names): run 1 as in k1, run 2
//!   with pb02 not halting, run 3 with pb02 and tn01 not halting (synthetic: every
//!   line carries `"fixture": "synthetic-from-live-k1: …"`);
//! - `researcher.k1.results.jsonl`: job-log line 4511 (the distill line) as is.
//!
//! Every case runs the built `xtask` binary from the workspace root.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use maos_eval::halt_replay::{ClassFloors, FloorRecord};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const FALSE_POSITIVES: [&str; 8] = [
    "pb02", "pb03", "pb04", "pb06", "tn01", "tn03", "tn05", "tn09",
];
const CASSETTE: &str = "halt-corpus-v0.3.live.json";

/// The workspace root: only the working directory of the spawned xtask.
fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The `--root` the report resolves its corpus, cassettes and price book against: a FROZEN copy
/// under the fixtures (`root/spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` is the
/// corpus the archived live run scored, sha 7c5bf481...; the live corpus moves whenever a prompt
/// is re-versioned, and so does the price book), so these numbers never depend on later stories.
fn root() -> PathBuf {
    fixtures().join("root")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-report")
}

fn s(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

fn xtask(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(workspace())
        .output()
        .expect("spawn xtask")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn json_of(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {}", stderr(out)))
}

/// `eval-report --results`, the cassettes looked up by name in `cassettes`.
fn aggregate(results: &Path, cassettes: &Path, extra: &[&str]) -> Output {
    let root = root();
    let mut args = vec![
        "eval-report",
        "--results",
        s(results),
        "--root",
        s(&root),
        "--cassettes",
        s(cassettes),
    ];
    args.extend_from_slice(extra);
    xtask(&args)
}

fn r3(v: &Value) -> f64 {
    (v.as_f64().expect("a number") * 1000.0).round() / 1000.0
}

fn lines_of(name: &str) -> Vec<Value> {
    std::fs::read_to_string(fixtures().join(name))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn write_lines(dir: &Path, name: &str, lines: &[Value]) -> PathBuf {
    let path = dir.join(name);
    let text: Vec<String> = lines.iter().map(Value::to_string).collect();
    std::fs::write(&path, text.join("\n") + "\n").unwrap();
    path
}

/// `line` with the given rows' verdicts set, `errors` named as malformed
/// completions (no halt), and the score's counts re-derived from the corpus.
fn edited(line: &Value, halted: &[(&str, bool)], errors: &[&str]) -> Value {
    let mut l = line.clone();
    for (id, h) in halted {
        l["verdicts"][*id] = json!(*h);
    }
    for id in errors {
        l["verdicts"][*id] = json!(false);
    }
    l["format_errors"] = json!(errors);
    let corpus = std::fs::read_to_string(root().join(line["corpus"].as_str().unwrap())).unwrap();
    let labels: BTreeMap<String, bool> = corpus
        .lines()
        .map(|r| serde_json::from_str::<Value>(r).unwrap())
        .map(|r| {
            (
                r["scenario_id"].as_str().unwrap().to_owned(),
                r["expected_halt"].as_bool().unwrap(),
            )
        })
        .collect();
    let (mut tp, mut fp, mut fnn) = (0, 0, 0);
    for (id, v) in l["verdicts"].as_object().unwrap() {
        match (labels[id], v.as_bool().unwrap()) {
            (true, true) => tp += 1,
            (false, true) => fp += 1,
            (true, false) => fnn += 1,
            (false, false) => {}
        }
    }
    l["score"]["true_positives"] = json!(tp);
    l["score"]["false_positives"] = json!(fp);
    l["score"]["false_negatives"] = json!(fnn);
    l
}

/// The k = 3 variant's cassettes: the one live cassette under three names.
fn k3_cassettes(dir: &Path) {
    for i in 1..=3 {
        std::fs::copy(
            fixtures().join(CASSETTE),
            dir.join(format!("halt-corpus-v0.3.live.{i}.json")),
        )
        .unwrap();
    }
}

#[test]
fn the_live_butler_run_reproduces_its_measured_numbers() {
    let out = aggregate(
        &fixtures().join("butler.k1.results.jsonl"),
        &fixtures(),
        &[],
    );
    // The measured precision misses its floor: the report is printed, exit 1.
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("halt_precision_overall mean 0.652 below its floor 0.85"),
        "{}",
        stderr(&out)
    );
    let r = json_of(&out);
    assert_eq!(r["schema"], "maos.eval.live/v1");
    assert_eq!(
        (&r["class"], &r["eval"], &r["k"]),
        (&json!("butler"), &json!("halt"), &json!(1))
    );
    assert_eq!(r["provider"], "anthropic");
    assert_eq!(r["model_id"], "claude-haiku-5-5");
    assert_eq!(r["prompt_version"], "0.3.0");
    assert_eq!(r["pass"], false);

    let precision = &r["metrics"]["halt_precision_overall"];
    assert_eq!(r3(&precision["mean"]), 0.652);
    assert_eq!(precision["n"], 23);
    assert_eq!(
        (r3(&precision["wilson95"][0]), r3(&precision["wilson95"][1])),
        (0.449, 0.812)
    );
    assert_eq!(precision["floor"], 0.85);
    assert_eq!(precision["pass"], false);
    let recall = &r["metrics"]["halt_recall_overall"];
    assert_eq!(
        (recall["mean"].as_f64(), recall["pass"].as_bool()),
        (Some(1.0), Some(true))
    );
    assert_eq!(r["metrics"]["halt_recall_calendar_conflict"]["n"], 12);
    assert_eq!(r["metrics"]["format_error_rate"]["pass"], true);

    let family = |name: &str| {
        let c = &r["confusion_by_family"][name];
        (
            c["tp"].as_u64(),
            c["fp"].as_u64(),
            c["tn"].as_u64(),
            c["fn"].as_u64(),
        )
    };
    assert_eq!(family("cc"), (Some(12), Some(0), Some(0), Some(0)));
    assert_eq!(family("df"), (Some(3), Some(0), Some(0), Some(0)));
    assert_eq!(family("pb"), (Some(0), Some(4), Some(2), Some(0)));
    assert_eq!(family("tn"), (Some(0), Some(4), Some(5), Some(0)));
    assert_eq!(
        r["corpus_families"],
        json!({"cc": 12, "df": 3, "pb": 6, "tn": 9})
    );
    assert_eq!(
        (
            r["flip_rate"].as_f64(),
            r["flip_rows"].as_array().map(Vec::len)
        ),
        (Some(0.0), Some(0))
    );
    assert_eq!(r["format_errors"], json!({"count": 0, "rows": []}));

    // AC8: tokens summed from the cassette, cost from the price book.
    assert_eq!(r["tokens"], json!({"input": 10349, "output": 8157}));
    let expected = (10349.0 * 100.0 + 8157.0 * 500.0) / 1000.0 / 1e6;
    assert!(
        (r["cost_usd"].as_f64().unwrap() - expected).abs() < 1e-12,
        "{}",
        r["cost_usd"]
    );
    assert_eq!(
        r["price_book"]["path"]
            .as_str()
            .map(|p| p.ends_with("xtask/provider-pricing.toml")),
        Some(true)
    );
    assert_eq!(r["runs"][0]["provenance"], "live-record");
    assert_eq!(r["runs"][0]["cassette_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(r["prompt_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn three_repeats_report_the_majority_decision_and_the_flip_rate() {
    let dir = tempfile::tempdir().unwrap();
    k3_cassettes(dir.path());
    let out = aggregate(
        &fixtures().join("butler.k3.results.jsonl"),
        dir.path(),
        &["--repeats", "3"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let r = json_of(&out);
    assert_eq!(r["k"], 3);
    let precision = &r["metrics"]["halt_precision_overall"];
    // Mean of 15/23, 15/22, 15/21; the Wilson interval is over the majority
    // decision (pb02 no halt, tn01 halt): 15 of 22.
    assert_eq!(r3(&precision["mean"]), 0.683);
    assert_eq!(
        (r3(&precision["min"]), r3(&precision["max"])),
        (0.652, 0.714)
    );
    assert_eq!(
        (precision["n"].as_u64(), r3(&precision["majority"])),
        (Some(22), 0.682)
    );
    assert_eq!(r["flip_rows"], json!(["pb02", "tn01"]));
    assert_eq!(r3(&r["flip_rate"]), 0.067);
    assert_eq!(
        r["confusion_by_family"]["pb"],
        json!({"tp": 0, "fp": 3, "tn": 3, "fn": 0})
    );
    assert_eq!(
        r["confusion_by_family"]["tn"],
        json!({"tp": 0, "fp": 4, "tn": 5, "fn": 0})
    );
    assert_eq!(r["tokens"], json!({"input": 31047, "output": 24471}));
    assert_eq!(r["runs"].as_array().map(Vec::len), Some(3));
}

/// Proven red (AC1): one flipped fixture verdict moves the family counts and
/// the interval; a removed repeat moves `k` and the flip rate.
#[test]
fn a_flipped_verdict_and_a_removed_repeat_change_the_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let base = lines_of("butler.k1.results.jsonl").remove(0);
    let flipped = edited(&base, &[("pb02", false)], &[]);
    let results = write_lines(dir.path(), "flipped.jsonl", &[flipped]);
    let out = aggregate(&results, &fixtures(), &[]);
    let r = json_of(&out);
    assert_eq!(
        r["confusion_by_family"]["pb"],
        json!({"tp": 0, "fp": 3, "tn": 3, "fn": 0})
    );
    let precision = &r["metrics"]["halt_precision_overall"];
    assert_eq!(precision["n"], 22);
    assert_ne!(
        (r3(&precision["wilson95"][0]), r3(&precision["wilson95"][1])),
        (0.449, 0.812)
    );
    assert_eq!(
        (r3(&precision["wilson95"][0]), r3(&precision["wilson95"][1])),
        (0.473, 0.836)
    );

    // The same flip without its score counts is a corrupted results file.
    let mut stale = base.clone();
    stale["verdicts"]["pb02"] = json!(false);
    let results = write_lines(dir.path(), "stale.jsonl", &[stale]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("score.false_positives disagrees with its verdicts"),
        "{}",
        stderr(&out)
    );

    k3_cassettes(dir.path());
    let two = write_lines(
        dir.path(),
        "k2.jsonl",
        &lines_of("butler.k3.results.jsonl")[..2],
    );
    let r = json_of(&aggregate(&two, dir.path(), &[]));
    assert_eq!(r["k"], 2);
    assert_eq!(r["flip_rows"], json!(["pb02"]));
    assert_eq!(r3(&r["flip_rate"]), 0.033);
    let out = aggregate(&two, dir.path(), &["--repeats", "3"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("2 result lines for 3 repeats"),
        "{}",
        stderr(&out)
    );
}

/// AC8: an unpriced model is an error, not 0; a cassette or corpus that is
/// not the one the line scored is refused.
#[test]
fn an_unpriced_model_and_a_mismatched_cassette_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut cassette: Value =
        serde_json::from_slice(&std::fs::read(fixtures().join(CASSETTE)).unwrap()).unwrap();
    for e in cassette["entries"].as_array_mut().unwrap() {
        e["response"]["provider_attribution"]["model_id"] = json!("claude-haiku-9-9");
    }
    let bytes = cassette.to_string();
    std::fs::write(dir.path().join(CASSETTE), &bytes).unwrap();
    let mut line = lines_of("butler.k1.results.jsonl").remove(0);

    let results = write_lines(dir.path(), "stale-sha.jsonl", &[line.clone()]);
    let out = aggregate(&results, dir.path(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("cassette_sha256"), "{}", stderr(&out));

    line["cassette_sha256"] = json!(format!("{:x}", Sha256::digest(bytes.as_bytes())));
    let results = write_lines(dir.path(), "unpriced.jsonl", &[line.clone()]);
    let out = aggregate(&results, dir.path(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("claude-haiku-9-9 is not in the price book")
            && stderr(&out).contains("an unpriced model is an error, not 0"),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty(), "an unpriced run prints no report");

    let mut line = lines_of("butler.k1.results.jsonl").remove(0);
    line["corpus_sha256"] = json!("00".repeat(32));
    let results = write_lines(dir.path(), "corpus.jsonl", &[line]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("corpus_sha256"), "{}", stderr(&out));
}

/// A passing candidate's report, with `edits` applied to the live line.
fn report_file(dir: &Path, name: &str, halted: &[(&str, bool)], errors: &[&str]) -> (PathBuf, i32) {
    let base = lines_of("butler.k1.results.jsonl").remove(0);
    let results = write_lines(
        dir,
        &format!("{name}.jsonl"),
        &[edited(&base, halted, errors)],
    );
    let report = dir.join(format!("{name}.live.json"));
    let out = aggregate(&results, &fixtures(), &["--out", s(&report)]);
    (report, out.status.code().expect("exit code"))
}

fn compare(reference: &Path, candidate: &Path) -> Output {
    xtask(&[
        "eval-report",
        "--reference",
        s(reference),
        "--candidate",
        s(candidate),
    ])
}

#[test]
fn a_candidate_is_compared_with_the_reference_by_one_decision() {
    let dir = tempfile::tempdir().unwrap();
    let all_clear: Vec<(&str, bool)> = FALSE_POSITIVES.iter().map(|id| (*id, false)).collect();
    let (perfect, code) = report_file(dir.path(), "perfect", &all_clear, &[]);
    assert_eq!(code, 0, "no false positive: every floor holds");

    // Identical to the reference: pass, no row changed.
    let out = compare(&perfect, &perfect);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let c = json_of(&out);
    assert_eq!(c["schema"], "maos.eval.compare/v1");
    assert_eq!(
        (&c["verdict"], &c["changed_rows"]),
        (&json!("pass"), &json!([]))
    );

    // One more wrong row (15/16 = 0.9375 against 1.0, tolerance 1/15): pass.
    let (one, _) = report_file(dir.path(), "one", &all_clear[..7], &[]);
    let out = compare(&perfect, &one);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(json_of(&out)["changed_rows"], json!(["tn09"]));

    // Two more wrong rows hold the 0.85 floor (15/17 = 0.882) but are worse
    // than the reference by more than one decision: fail.
    let (two, code) = report_file(dir.path(), "two", &all_clear[..6], &[]);
    assert_eq!(code, 0);
    let out = compare(&perfect, &two);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("halt_precision_overall is worse than the reference"),
        "{}",
        stderr(&out)
    );
    assert_eq!(json_of(&out)["verdict"], "fail");

    // The measured 0.652 misses the calibrated floor.
    let (live, code) = report_file(dir.path(), "live", &[], &[]);
    assert_eq!(code, 1);
    let out = compare(&perfect, &live);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains(
            "the candidate fails: halt_precision_overall mean 0.652 below its floor 0.85"
        ),
        "{}",
        stderr(&out)
    );

    // A malformed completion fails the candidate even when every floor holds.
    let (broken, _) = report_file(dir.path(), "broken", &all_clear, &["cc01"]);
    let out = compare(&perfect, &broken);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("format_error_rate"),
        "{}",
        stderr(&out)
    );

    // Reference and candidate must have scored the same corpus.
    let distill = dir.path().join("distill.live.json");
    let out = aggregate(
        &fixtures().join("researcher.k1.results.jsonl"),
        &fixtures(),
        &["--out", s(&distill)],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let out = compare(&perfect, &distill);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("differ in `class`"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_floor_is_calibrated_from_the_wilson_lower_bound_never_below_the_spec_minimum() {
    let dir = tempfile::tempdir().unwrap();
    let all_clear: Vec<(&str, bool)> = FALSE_POSITIVES.iter().map(|id| (*id, false)).collect();
    let (perfect, _) = report_file(dir.path(), "perfect", &all_clear, &[]);
    let calibrate = |spec: &str, metric: &str, result: &Path| {
        xtask(&[
            "eval-report",
            "--calibrate",
            "--spec",
            spec,
            "--metric",
            metric,
            "--result",
            s(result),
        ])
    };

    // 15 of 15: Wilson lower bound 0.796, rounded down 0.79, above the 0.70 spec minimum.
    let out = calibrate("0.70", "halt_recall_overall", &perfect);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let c = json_of(&out);
    assert_eq!(
        (c["floor"].as_f64(), c["spec_change_required"].as_bool()),
        (Some(0.79), Some(false))
    );
    assert_eq!(c["model_id"], "claude-haiku-5-5");
    assert_eq!(c["k"], 1);
    assert!(c["rule"].as_str().unwrap().contains("D-28-1-03 A"));

    // The record format is shared with maos-bench: the calibrated number
    // applies, a hand-lowered one does not.
    let entry = |floor: f64| {
        format!(
            "schema = \"maos.eval.floors/v1\"\n[[floor]]\nmetric = \"halt_recall_overall\"\nfloor = {floor}\n\
             spec_minimum = 0.7\nspec_source = \"NFR-Test-4\"\nmeasured_mean = {}\nwilson95 = [{}, {}]\n\
             k = {}\nmodel_id = \"{}\"\nprompt_sha256 = \"{}\"\nrule = \"D-28-1-03 A\"\nrationale = \"test\"\n\
             date = \"2026-11-15\"\nresult_file = \"perfect.live.json\"\n",
            c["measured_mean"], c["wilson95"][0], c["wilson95"][1], c["k"],
            c["model_id"].as_str().unwrap(), c["prompt_sha256"].as_str().unwrap()
        )
    };
    let orchestrator = ClassFloors {
        halt_recall_calendar_conflict: None,
        halt_recall_overall: 0.70,
        halt_precision_overall: 0.85,
    };
    let record: FloorRecord = toml::from_str(&entry(0.79)).unwrap();
    assert_eq!(
        record.apply(orchestrator).unwrap().halt_recall_overall,
        0.79
    );
    let record: FloorRecord = toml::from_str(&entry(0.60)).unwrap();
    assert!(record
        .apply(orchestrator)
        .unwrap_err()
        .contains("spec minimum"));
    // The spec minimum is not the floor the record's own evidence requires.
    let record: FloorRecord = toml::from_str(&entry(0.70)).unwrap();
    assert!(record
        .apply(orchestrator)
        .unwrap_err()
        .contains("Wilson lower bound 0.79"));

    // The spec minimum wins over a lower interval bound (precision 15 of 15 against 0.85).
    let out = calibrate("0.85", "halt_precision_overall", &perfect);
    assert_eq!(json_of(&out)["floor"], 0.85);

    // A mean below the spec minimum is never lowered to: it asks for an operator D-row.
    let (live, _) = report_file(dir.path(), "live", &[], &[]);
    let out = calibrate("0.85", "halt_precision_overall", &live);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("open an operator spec-change D-row"),
        "{}",
        stderr(&out)
    );
    let c = json_of(&out);
    assert_eq!(
        (c["floor"].as_f64(), c["spec_change_required"].as_bool()),
        (Some(0.85), Some(true))
    );

    // A ceiling is not calibrated.
    let out = calibrate("0.0", "format_error_rate", &perfect);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("ceiling"), "{}", stderr(&out));
}

#[test]
fn the_researcher_distill_line_aggregates_with_the_families_of_its_corpus() {
    let out = aggregate(
        &fixtures().join("researcher.k1.results.jsonl"),
        &fixtures(),
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("hedge_preservation mean 0.944"),
        "{}",
        stderr(&out)
    );
    let r = json_of(&out);
    assert_eq!(
        (&r["eval"], &r["class"]),
        (&json!("distill"), &json!("researcher"))
    );
    assert_eq!(r["model_id"], "claude-sonnet-5-5");
    assert_eq!(r["metrics"]["recall"]["pass"], true);
    assert_eq!(r["metrics"]["secret_leakage"]["pass"], true);
    assert_eq!(r["metrics"]["hedge_preservation"]["pass"], false);
    assert_eq!(r["metrics"]["hedge_preservation"]["wilson95"], Value::Null);
    // The denominators come from the corpus: 25 claims and 18 hedges in the clean rows.
    assert_eq!(r["metrics"]["recall"]["n"], 25);
    assert_eq!(r["metrics"]["hedge_preservation"]["n"], 18);
    assert_eq!(
        r["corpus_families"],
        json!({"clean": 12, "hedge-drop": 1, "fabricated-cite": 1, "planted-secret": 1, "dropped-claim": 1})
    );
    assert_eq!(r["tokens"], json!({"input": 6144, "output": 1662}));
    let expected = (6144.0 * 2000.0 + 1662.0 * 10000.0) / 1000.0 / 1e6;
    assert!(
        (r["cost_usd"].as_f64().unwrap() - expected).abs() < 1e-12,
        "{}",
        r["cost_usd"]
    );
}

/// The archived distill line with every floor met (hedge preservation 1.0) and
/// `edit` applied.
fn distill_line(edit: impl FnOnce(&mut Value)) -> Value {
    let mut l = lines_of("researcher.k1.results.jsonl").remove(0);
    l["score"]["metrics"]["hedge_preservation"] = json!(1.0);
    l["missed_floors"] = json!([]);
    l["pass"] = json!(true);
    edit(&mut l);
    l
}

const SURVEY_DRIFT: &str = "rd05: survey differs from the frozen one (re-pin needed): {}";

/// Proven red: a structural failure of any repeat is a failure of the
/// aggregate; a numeric miss could average out, this never does.
#[test]
fn a_structural_failure_of_one_repeat_makes_the_distill_aggregate_red() {
    let dir = tempfile::tempdir().unwrap();
    let clean = distill_line(|_| {});
    let two = write_lines(dir.path(), "clean.jsonl", &[clean.clone(), clean.clone()]);
    let out = aggregate(&two, &fixtures(), &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(json_of(&out)["pass"], true);

    let bad = distill_line(|l| {
        l["missed_floors"] = json!([SURVEY_DRIFT]);
        l["pass"] = json!(false);
    });
    let mixed = write_lines(dir.path(), "mixed.jsonl", &[clean, bad]);
    let out = aggregate(&mixed, &fixtures(), &[]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "every metric holds: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("survey differs from the frozen one"),
        "{}",
        stderr(&out)
    );
    let r = json_of(&out);
    assert_eq!(
        (&r["pass"], &r["failures"]),
        (&json!(false), &json!([SURVEY_DRIFT]))
    );
    assert_eq!(r["metrics"]["recall"]["pass"], true);
}

/// The survey message embeds model text: a finding that contains the numeric
/// phrase must not hide the structural failure (exact shapes only).
#[test]
fn a_survey_message_embedding_the_numeric_phrase_is_still_structural() {
    let dir = tempfile::tempdir().unwrap();
    let message = "rd05: survey differs from the frozen one (re-pin needed): \
                   {\"findings\":[{\"statement\":\"recall 0.5 misses its floor 0.9\"}]}";
    let line = distill_line(|l| l["missed_floors"] = json!([message]));
    let results = write_lines(dir.path(), "embedded.jsonl", &[line]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "every metric holds: {}",
        stderr(&out)
    );
    assert_eq!(json_of(&out)["failures"], json!([message]));

    // The real numeric and control shapes are still recognised: no structural failure.
    let numeric = distill_line(|l| {
        l["missed_floors"] = json!([
            "recall 0.900 misses its floor 0.9",
            "negative control rd13-hedge-drop (HedgeDrop) not detected"
        ]);
    });
    let results = write_lines(dir.path(), "numeric.jsonl", &[numeric]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert_eq!(json_of(&out)["failures"], json!([]), "{}", stderr(&out));
}

/// One decision is 1 / n: recall 24 of 25 passes against a reference of 1.0,
/// 23 of 25 does not; an undetected control or a structural failure rejects a
/// candidate whose five metrics all hold.
#[test]
fn a_distill_candidate_is_compared_by_one_decision_and_by_its_mandatory_failures() {
    let dir = tempfile::tempdir().unwrap();
    let report = |name: &str, edit: &dyn Fn(&mut Value)| {
        let mut line = distill_line(|_| {});
        edit(&mut line);
        let results = write_lines(dir.path(), &format!("{name}.jsonl"), &[line]);
        let live = dir.path().join(format!("{name}.live.json"));
        aggregate(&results, &fixtures(), &["--out", s(&live)]);
        live
    };
    let reference = report("reference", &|_| {});

    let out = compare(
        &reference,
        &report("one", &|l| l["score"]["metrics"]["recall"] = json!(0.96)),
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "24 of 25 is one decision: {}",
        stderr(&out)
    );

    let out = compare(
        &reference,
        &report("two", &|l| l["score"]["metrics"]["recall"] = json!(0.92)),
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("recall is worse than the reference by 0.080"),
        "{}",
        stderr(&out)
    );

    let blind = report("blind", &|l| {
        l["score"]["controls"][0]["detected"] = json!(false);
        l["missed_floors"] = json!(["negative control rd13-hedge-drop (HedgeDrop) not detected"]);
        l["pass"] = json!(false);
    });
    let out = compare(&reference, &blind);
    assert_eq!(
        out.status.code(),
        Some(1),
        "all five metrics hold: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("the candidate fails: controls not detected"),
        "{}",
        stderr(&out)
    );

    let drifted = report("drifted", &|l| l["missed_floors"] = json!([SURVEY_DRIFT]));
    let out = compare(&reference, &drifted);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("the candidate fails: rd05: survey differs"),
        "{}",
        stderr(&out)
    );
}

// ---------------------------------------------------------------------------
// Story 28-3 review (findings 2 and 4) — the claim-level decisions a distill
// line carries and the interval the report takes over them; a judge-only
// repeat line's two artifacts.
// ---------------------------------------------------------------------------

/// The frozen corpus' clean claims: (scenario, claim, hedge count), corpus order.
fn clean_claims() -> Vec<(String, String, usize)> {
    let corpus = std::fs::read_to_string(
        root().join("spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl"),
    )
    .unwrap();
    corpus
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|row| row["control"].is_null())
        .flat_map(|row| {
            let scenario = row["scenario_id"].as_str().unwrap().to_owned();
            row["claims"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    (
                        scenario.clone(),
                        c["claim_id"].as_str().unwrap().to_owned(),
                        c["hedges"].as_array().unwrap().len(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The metrics `decisions` imply (independent of the xtask's own arithmetic).
fn implied(decisions: &[Value]) -> [f64; 5] {
    let found: Vec<&Value> = decisions
        .iter()
        .map(|d| &d["finding"])
        .filter(|f| !f.is_null())
        .collect();
    let count = |key: &str| found.iter().filter(|f| f[key] == true).count() as f64;
    let hedges: Vec<bool> = found
        .iter()
        .flat_map(|f| f["hedges"].as_array().unwrap().iter().map(|h| h == true))
        .collect();
    let n = found.len() as f64;
    [
        n / decisions.len() as f64,
        count("faithful") / n,
        hedges.iter().filter(|h| **h).count() as f64 / hedges.len() as f64,
        count("resolves") / n,
        count("leaks") / n,
    ]
}

/// A distill line whose decisions are all-true except `flips` (claim index,
/// key) set false (`"found"` drops the claim's finding); its metrics follow.
fn decided_line(flips: &[(usize, &str)]) -> Value {
    let decisions: Vec<Value> = clean_claims()
        .iter()
        .enumerate()
        .map(|(i, (scenario, claim, hedges))| {
            let flipped = |key: &str| flips.contains(&(i, key));
            let finding = if flipped("found") {
                Value::Null
            } else {
                json!({
                    "faithful": !flipped("faithful"),
                    "resolves": !flipped("resolves"),
                    "leaks": flipped("leaks"),
                    "hedges": vec![!flipped("hedge"); *hedges],
                })
            };
            json!({"scenario_id": scenario, "claim_id": claim, "finding": finding})
        })
        .collect();
    let [recall, faithfulness, hedge, traceability, leakage] = implied(&decisions);
    distill_line(|l| {
        l["score"]["metrics"] = json!({
            "recall": recall, "faithfulness": faithfulness, "hedge_preservation": hedge,
            "traceability": traceability, "secret_leakage": leakage,
        });
        l["decisions"] = json!(decisions);
    })
}

fn lower(v: &Value) -> f64 {
    v["wilson95"][0].as_f64().expect("a Wilson interval")
}

/// Before 28-3 a distill line carried only aggregates, so `wilson95` was null
/// for every distill metric and `--calibrate` refused them. With decisions the
/// interval is over the per-claim majority of the k repeats: one flipped
/// repeat of three leaves the majority (25 of 25); two flip it (24 of 25).
#[test]
fn decisions_give_the_distill_metrics_a_wilson_interval_over_the_per_claim_majority() {
    let dir = tempfile::tempdir().unwrap();
    let clean = decided_line(&[]);
    let one = write_lines(
        dir.path(),
        "one.jsonl",
        &[
            clean.clone(),
            decided_line(&[(3, "faithful")]),
            clean.clone(),
        ],
    );
    let r = json_of(&aggregate(&one, &fixtures(), &[]));
    let faith = &r["metrics"]["faithfulness"];
    assert_eq!(
        (faith["n"].as_u64(), faith["majority"].as_f64()),
        (Some(25), Some(1.0))
    );
    assert!((lower(faith) - 0.8668).abs() < 1e-3, "{faith}");
    assert!((faith["mean"].as_f64().unwrap() - (1.0 + 0.96 + 1.0) / 3.0).abs() < 1e-12);
    // 18 hedges in the clean rows; every one kept by the majority.
    let hedge = &r["metrics"]["hedge_preservation"];
    assert_eq!(
        (hedge["n"].as_u64(), hedge["majority"].as_f64()),
        (Some(18), Some(1.0))
    );
    assert!(r["metrics"]["secret_leakage"]["wilson95"].is_array());
    assert_eq!(r["pass"], true, "{}", r["failures"]);

    let two = write_lines(
        dir.path(),
        "two.jsonl",
        &[
            decided_line(&[(3, "faithful")]),
            decided_line(&[(3, "faithful")]),
            clean.clone(),
        ],
    );
    let r = json_of(&aggregate(&two, &fixtures(), &[]));
    let faith = &r["metrics"]["faithfulness"];
    assert_eq!(faith["majority"].as_f64(), Some(0.96));
    assert!((lower(faith) - 0.8046).abs() < 1e-3, "{faith}");

    // A claim the majority did not find leaves every finding denominator.
    let missing = decided_line(&[(0, "found")]);
    let three = write_lines(
        dir.path(),
        "three.jsonl",
        &[missing.clone(), missing, clean],
    );
    let r = json_of(&aggregate(&three, &fixtures(), &[]));
    assert_eq!(r["metrics"]["recall"]["majority"].as_f64(), Some(0.96));
    assert_eq!(r["metrics"]["faithfulness"]["n"], 24);

    // The calibration path now has an interval to take its lower bound from.
    let live = dir.path().join("decided.live.json");
    aggregate(&one, &fixtures(), &["--out", s(&live)]);
    let out = xtask(&[
        "eval-report",
        "--calibrate",
        "--spec",
        "0.98",
        "--metric",
        "faithfulness",
        "--result",
        s(&live),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let c = json_of(&out);
    assert!((c["wilson95"][0].as_f64().unwrap() - 0.8668).abs() < 1e-3);
    assert_eq!(c["floor"], 0.98, "the spec minimum is the larger");
}

#[test]
fn a_line_whose_metrics_disagree_with_its_decisions_or_that_half_carries_them_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut lying = decided_line(&[]);
    lying["score"]["metrics"]["faithfulness"] = json!(0.5);
    let results = write_lines(dir.path(), "lying.jsonl", &[lying]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("score.metrics.faithfulness disagrees with its decisions"),
        "{}",
        stderr(&out)
    );

    let mut wrong_claims = decided_line(&[]);
    wrong_claims["decisions"].as_array_mut().unwrap().pop();
    let results = write_lines(dir.path(), "short.jsonl", &[wrong_claims]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert!(
        stderr(&out).contains("not the corpus' clean claims"),
        "{}",
        stderr(&out)
    );

    let mut bare = decided_line(&[]);
    bare.as_object_mut().unwrap().remove("decisions");
    let results = write_lines(dir.path(), "half.jsonl", &[decided_line(&[]), bare]);
    let out = aggregate(&results, &fixtures(), &[]);
    assert!(
        stderr(&out).contains("only some result lines carry `decisions`"),
        "{}",
        stderr(&out)
    );
}

/// Finding 2 — a judge-only repeat line's `cassette` is the judge OUTPUT and
/// its digest is that file's; the survey it replayed is its own artifact with
/// its own digest, reported and re-hashed.
#[test]
fn a_judge_repeat_line_is_checked_against_both_of_its_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let cassettes = dir.path().join("cassettes");
    std::fs::create_dir(&cassettes).unwrap();
    let output = std::fs::read(fixtures().join("distill-corpus-v0.live.json")).unwrap();
    std::fs::write(cassettes.join("distill-corpus-v0.live.json"), &output).unwrap();
    let survey = b"{\"entries\": [], \"provenance\": \"live-record\"}\n";
    std::fs::write(cassettes.join("survey.json"), survey).unwrap();
    let digest = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));

    let repeat_line = |survey_sha: String| {
        let mut line = decided_line(&[]);
        line["judge_repeat"] = json!({
            "k": 3, "gate_basis": "first-verdict",
            "survey_cassette": "elsewhere/survey.json", "survey_cassette_sha256": survey_sha,
            "agreement_rate": 1.0, "faithfulness_per_pass": [1.0, 1.0, 1.0],
            "faithfulness_mean": 1.0, "faithfulness_worst_pass": 1.0,
        });
        line
    };
    let good = write_lines(dir.path(), "good.jsonl", &[repeat_line(digest(survey))]);
    let out = aggregate(&good, &cassettes, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let run = &json_of(&out)["runs"][0];
    assert_eq!(run["cassette_sha256"], digest(&output).as_str());
    assert_eq!(run["survey_cassette"], "elsewhere/survey.json");
    assert_eq!(run["survey_cassette_sha256"], digest(survey).as_str());
    assert_eq!(run["judge_repeat"]["faithfulness_worst_pass"], 1.0);

    // Each artifact is checked on its own digest.
    let bad_survey = write_lines(
        dir.path(),
        "bad-survey.jsonl",
        &[repeat_line("00".repeat(32))],
    );
    let out = aggregate(&bad_survey, &cassettes, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("judge_repeat.survey_cassette_sha256"),
        "{}",
        stderr(&out)
    );

    // The pre-fix line: `cassette_sha256` over the SURVEY input while
    // `cassette` names the judge output is rejected.
    let mut aliased = repeat_line(digest(survey));
    aliased["cassette_sha256"] = json!(digest(survey));
    let results = write_lines(dir.path(), "aliased.jsonl", &[aliased]);
    let out = aggregate(&results, &cassettes, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("(cassette_sha256)"),
        "{}",
        stderr(&out)
    );
}
