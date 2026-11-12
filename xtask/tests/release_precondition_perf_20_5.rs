//! Operator ruling 2026-11-12 (rc = b) — `check-release-precondition --perf-verdicts` judges the
//! `check-perf-verdict --json` report for the released SHA: FAIL / INDETERMINATE are always
//! refused, ABSENT only under `--perf-absent-ok`, otherwise every row must be PASS. Every vector
//! drives the real `xtask` binary; a tool crash (no JSON) can never read as ABSENT.

use std::process::{Command, Output};

use serde_json::{json, Value};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn report(sha: &str, verdicts: &[&str]) -> String {
    let rows: Vec<Value> = verdicts
        .iter()
        .enumerate()
        .map(|(i, v)| json!({ "metric": format!("m{i}"), "verdict": v, "reason": "r", "binding": false }))
        .collect();
    json!({ "gate": "check-perf-verdict", "sha": sha, "verdicts": rows }).to_string()
}

fn run(perf: Option<&str>, absent_ok: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let runs = dir.path().join("runs.json");
    let body = json!({ "check_runs": [{ "name": "aggregate", "status": "completed", "conclusion": "success", "head_sha": SHA }] });
    std::fs::write(&runs, body.to_string()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command.args([
        "check-release-precondition",
        "--check-runs",
        runs.to_str().unwrap(),
    ]);
    if let Some(text) = perf {
        let path = dir.path().join("perf.json");
        std::fs::write(&path, text).unwrap();
        command.args(["--perf-verdicts", path.to_str().unwrap()]);
    }
    if absent_ok {
        command.arg("--perf-absent-ok");
    }
    command.output().unwrap()
}

fn refused(output: &Output, needle: &str) {
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "must refuse: {text}");
    assert!(text.contains(needle), "{needle:?} not in: {text}");
}

#[test]
fn all_pass_passes_with_or_without_the_absent_flag() {
    for absent_ok in [false, true] {
        let output = run(Some(&report(SHA, &["PASS", "PASS"])), absent_ok);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("perf verdicts"));
    }
}

#[test]
fn absent_is_accepted_only_under_the_absent_flag_and_never_with_a_fail() {
    let absent = report(SHA, &["PASS", "ABSENT"]);
    refused(&run(Some(&absent), false), "m1=ABSENT");
    assert!(run(Some(&absent), true).status.success());
    for bad in ["FAIL", "INDETERMINATE", "pass", "SKIPPED"] {
        let text = report(SHA, &["ABSENT", bad]);
        for absent_ok in [false, true] {
            refused(&run(Some(&text), absent_ok), &format!("m1={bad}"));
        }
    }
}

#[test]
fn a_crash_a_foreign_sha_or_an_empty_report_is_never_absent() {
    for text in [
        String::new(),
        "{not json".to_string(),
        "error: the perf-ledger is missing".to_string(),
        report(SHA, &[]),
        report("fedcba9876543210fedcba9876543210fedcba98", &["ABSENT"]),
        json!({ "gate": "check-perf-verdict", "sha": SHA }).to_string(),
    ] {
        let output = run(Some(&text), true);
        assert!(
            !output.status.success(),
            "{text:?} must be refused with --perf-absent-ok"
        );
    }
    let missing = run(None, true);
    assert!(
        !missing.status.success(),
        "the absent flag alone is a usage error"
    );
    assert!(
        run(None, false).status.success(),
        "no --perf-verdicts: the leg is not asked for"
    );
}
