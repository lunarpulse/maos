//! Story 20-5 AC4 — `check-release-precondition --fuzz-ledger` refuses a SHA without a
//! complete T3 fuzz chain. The rule is the collector's (`fuzz-prerelease.yml`
//! `t3-ledger-collect`): per target, ONE workflow run's five links, every record a `T3`
//! record at the released commit, libFuzzer-measured seconds summing to >= 86 400.
//! Every vector drives the real `xtask` binary; the aggregate leg is not under test here
//! (`check_release_precondition_gate.rs`).

use std::process::{Command, Output};

use serde_json::{json, Value};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER_SHA: &str = "fedcba9876543210fedcba9876543210fedcba98";
const TARGETS: [&str; 3] = ["manifest_parser", "frame_deser", "fuzz_exec_deps_parse_elf"];

fn record(target: &str, tier: &str, commit: &str, run: &str, link: &str, seconds: u64) -> Value {
    json!({
        "commit": commit,
        "corpus_size": 10,
        "cpu_seconds": seconds,
        "run_id": format!("{run}-{link}"),
        "target": target,
        "tier": tier,
        "timestamp": "2026-11-01T00:00:00Z",
        "workers": 4
    })
}

/// A complete chain for `target`: five links of 17 280 s each (= 86 400 s).
fn chain(target: &str, run: &str) -> Vec<Value> {
    (1..=5)
        .map(|link| record(target, "T3", SHA, run, &link.to_string(), 17_280))
        .collect()
}

fn full_ledger() -> Vec<Value> {
    TARGETS
        .iter()
        .flat_map(|target| chain(target, "1001"))
        .collect()
}

fn ledger(records: Vec<Value>) -> Value {
    json!({ "schema_version": 1, "records": records })
}

fn run_with(ledger_text: Option<String>, head_sha: &str) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let check_runs = dir.path().join("check-runs.json");
    std::fs::write(
        &check_runs,
        serde_json::to_vec(&json!({
            "total_count": 1,
            "check_runs": [{
                "name": "aggregate",
                "status": "completed",
                "conclusion": "success",
                "head_sha": head_sha
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let ledger_path = dir.path().join("fuzz-ledger.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command.args([
        "check-release-precondition",
        "--check-runs",
        check_runs.to_str().unwrap(),
    ]);
    match ledger_text {
        Some(text) => {
            std::fs::write(&ledger_path, text).unwrap();
            command.args(["--fuzz-ledger", ledger_path.to_str().unwrap()]);
        }
        None => {
            command.args(["--fuzz-ledger", ledger_path.to_str().unwrap()]);
        }
    }
    command.output().expect("run xtask")
}

fn run(records: Vec<Value>) -> Output {
    run_with(Some(ledger(records).to_string()), SHA)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused_naming(output: &Output, target: &str) {
    let text = stderr(output);
    assert!(!output.status.success(), "must refuse; stderr: {text}");
    assert!(
        text.contains("T3 fuzz chain") && text.contains(target),
        "refusal must name the chain and the target {target}; stderr: {text}"
    );
}

#[test]
fn a_complete_chain_for_every_target_passes() {
    let output = run(full_ledger());
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("T3 fuzz chain"),
        "the pass line names the leg: {}",
        stderr(&output)
    );
}

#[test]
fn four_links_are_refused() {
    let mut records = full_ledger();
    records.retain(|r| !(r["target"] == "frame_deser" && r["run_id"] == "1001-5"));
    assert_refused_naming(&run(records), "frame_deser");
}

#[test]
fn a_chain_for_another_sha_is_refused() {
    let mut records: Vec<Value> = full_ledger();
    for r in records.iter_mut() {
        if r["target"] == "manifest_parser" {
            r["commit"] = json!(OTHER_SHA);
        }
    }
    assert_refused_naming(&run(records), "manifest_parser");
}

#[test]
fn t2_records_never_count_as_t3() {
    let mut records: Vec<Value> = full_ledger();
    for r in records.iter_mut() {
        if r["target"] == "fuzz_exec_deps_parse_elf" {
            r["tier"] = json!("T2");
        }
    }
    assert_refused_naming(&run(records), "fuzz_exec_deps_parse_elf");
}

#[test]
fn two_runs_each_below_24h_never_sum_across_runs() {
    // Each run has all five links but only 12 h; together they hold 24 h.
    let mut records: Vec<Value> = TARGETS
        .iter()
        .flat_map(|target| chain(target, "1001"))
        .collect();
    for r in records.iter_mut() {
        if r["target"] == "manifest_parser" {
            r["cpu_seconds"] = json!(8_640);
        }
    }
    records.extend((1..=5).map(|link| {
        record(
            "manifest_parser",
            "T3",
            SHA,
            "1002",
            &link.to_string(),
            8_640,
        )
    }));
    assert_refused_naming(&run(records), "manifest_parser");
}

#[test]
fn one_second_short_of_24h_is_refused_and_exactly_24h_passes() {
    let mut records = full_ledger();
    for r in records.iter_mut() {
        if r["target"] == "frame_deser" && r["run_id"] == "1001-3" {
            r["cpu_seconds"] = json!(17_279);
        }
    }
    assert_refused_naming(&run(records), "frame_deser");
    let output = run(full_ledger());
    assert!(output.status.success(), "stderr: {}", stderr(&output));
}

#[test]
fn a_missing_target_is_refused() {
    let records: Vec<Value> = full_ledger()
        .into_iter()
        .filter(|r| r["target"] != "fuzz_exec_deps_parse_elf")
        .collect();
    assert_refused_naming(&run(records), "fuzz_exec_deps_parse_elf");
}

#[test]
fn an_empty_ledger_is_refused() {
    assert_refused_naming(&run(Vec::new()), "manifest_parser");
}

#[test]
fn a_rerun_link_counts_once_and_cannot_stand_in_for_a_missing_link() {
    // Link 3 appended twice (a re-run of the same run id) is still ONE link: with link 5
    // missing the chain has four distinct links.
    let mut records = full_ledger();
    records.retain(|r| !(r["target"] == "manifest_parser" && r["run_id"] == "1001-5"));
    records.push(record("manifest_parser", "T3", SHA, "1001", "3", 17_280));
    assert_refused_naming(&run(records), "manifest_parser");

    // A re-run of a link inside a complete chain does not break it.
    let mut records = full_ledger();
    records.push(record("manifest_parser", "T3", SHA, "1001", "3", 17_280));
    let output = run(records);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
}

#[test]
fn a_link_outside_one_to_five_is_not_a_link() {
    let mut records = full_ledger();
    for r in records.iter_mut() {
        if r["target"] == "frame_deser" && r["run_id"] == "1001-5" {
            r["run_id"] = json!("1001-nightly");
        }
    }
    assert_refused_naming(&run(records), "frame_deser");
}

#[test]
fn a_bad_schema_version_is_a_loud_refusal() {
    let output = run_with(
        Some(json!({ "schema_version": 2, "records": full_ledger() }).to_string()),
        SHA,
    );
    let text = stderr(&output);
    assert!(!output.status.success(), "stderr: {text}");
    assert!(text.contains("schema_version"), "stderr: {text}");
}

#[test]
fn a_malformed_or_missing_ledger_is_a_loud_refusal_never_a_pass() {
    for text in ["", "{", "[]", "{\"schema_version\":1}", "{\"records\":[]}"] {
        let output = run_with(Some(text.to_string()), SHA);
        assert!(
            !output.status.success(),
            "malformed ledger {text:?} must refuse; stderr: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("fuzz ledger"),
            "{}",
            stderr(&output)
        );
    }
    let output = run_with(None, SHA);
    assert!(!output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("fuzz ledger"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_empty_released_sha_is_refused_not_matched_against_empty_commits() {
    let mut records = full_ledger();
    for r in records.iter_mut() {
        r["commit"] = json!("");
    }
    let output = run_with(Some(ledger(records).to_string()), "");
    let text = stderr(&output);
    assert!(!output.status.success(), "stderr: {text}");
    assert!(text.contains("T3 fuzz chain"), "stderr: {text}");
}

#[test]
fn the_aggregate_leg_still_refuses_first() {
    let dir = tempfile::tempdir().unwrap();
    let check_runs = dir.path().join("check-runs.json");
    std::fs::write(
        &check_runs,
        json!({"check_runs": [{"name": "aggregate", "status": "completed",
                               "conclusion": "failure", "head_sha": SHA}]})
        .to_string(),
    )
    .unwrap();
    let ledger_path = dir.path().join("fuzz-ledger.json");
    std::fs::write(&ledger_path, ledger(full_ledger()).to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "check-release-precondition",
            "--check-runs",
            check_runs.to_str().unwrap(),
            "--fuzz-ledger",
            ledger_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("conclusion is failure"),
        "{}",
        stderr(&output)
    );
}

// ---- workflow wiring (AC4.2, AC5.2; D-20-5-07 A) -------------------------------------

fn workflow(file: &str) -> serde_yaml::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../.github/workflows")
        .join(file);
    serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn steps(workflow: &serde_yaml::Value, job: &str) -> Vec<serde_yaml::Value> {
    workflow["jobs"][job]["steps"]
        .as_sequence()
        .unwrap()
        .clone()
}

fn step_named<'a>(steps: &'a [serde_yaml::Value], name: &str) -> (usize, &'a serde_yaml::Value) {
    steps
        .iter()
        .enumerate()
        .find(|(_, step)| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("step {name:?} must exist"))
}

#[test]
fn the_release_job_passes_the_fuzz_ledger_to_the_precondition_for_rc_and_ga_tags() {
    let release = workflow("release.yml");
    let steps = steps(&release, "sign-and-publish");
    let (_, guard) = step_named(&steps, "Require successful main discipline aggregate");
    let run = guard["run"].as_str().unwrap();
    assert!(run.contains("check-release-precondition"), "{run}");
    assert!(
        run.contains("--fuzz-ledger"),
        "the precondition receives the ledger: {run}"
    );
    assert!(
        run.contains("contents/fuzz-ledger.json?ref="),
        "the ledger is fetched read-only from the ledger branch: {run}"
    );
    assert!(
        run.contains("check_name=aggregate&filter=latest"),
        "the aggregate leg keeps its server-side latest filter: {run}"
    );
    assert!(
        run.contains("rc) fuzz_args") && run.contains("ga) fuzz_args") && run.contains("alpha)"),
        "rc and GA kinds require the chain, the alpha kind keeps the aggregate-only precondition: {run}"
    );
}

#[test]
fn the_release_job_runs_rc_readiness_before_signing_for_rc_tags_only() {
    let release = workflow("release.yml");
    let steps = steps(&release, "sign-and-publish");
    let (readiness, step) = step_named(&steps, "Require rc tree readiness (rc tags only)");
    assert_eq!(step["if"].as_str(), Some("steps.tag.outputs.kind == 'rc'"));
    let run = step["run"].as_str().unwrap();
    assert!(
        run.contains("check-rc-readiness") && run.contains("--tag \"$TAG\""),
        "{run}"
    );
    assert_eq!(
        step["env"]["TAG"].as_str(),
        Some("${{ github.ref_name }}"),
        "the tag name reaches the shell as data, never as script text"
    );
    let (sign, _) = step_named(&steps, "Generate SHA256SUMS and sign");
    assert!(readiness < sign, "readiness gates signing");
}

#[test]
fn fuzz_prerelease_is_dispatched_before_the_tag_not_started_by_it() {
    let fuzz = workflow("fuzz-prerelease.yml");
    let on = &fuzz["on"];
    assert!(
        on.get("workflow_dispatch").is_some(),
        "the operator dispatches the chain"
    );
    assert!(
        on.get("push").is_none(),
        "a v*-rc* push trigger would start a duplicate ~26 h chain after the tag it must precede"
    );
}
