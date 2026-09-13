//! Story 15-4 AC3 — the release workflow and these vectors share one predicate.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

fn run(check_runs: Value) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("check-runs.json");
    std::fs::write(&path, serde_json::to_vec(&check_runs).unwrap()).expect("write fixture");
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "check-release-precondition",
            "--check-runs",
            path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run xtask")
}

/// `release.yml` pipes the `gh api` response on stdin and passes no `--check-runs`.
/// Driving only the flag would leave the transport the release actually uses untested.
fn run_stdin(check_runs: Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-release-precondition", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn xtask");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&serde_json::to_vec(&check_runs).unwrap())
        .expect("write stdin fixture");
    child.wait_with_output().expect("run xtask")
}

fn response(status: &str, conclusion: Option<&str>) -> Value {
    json!({
        "total_count": 1,
        "check_runs": [{
            "name": "aggregate",
            "status": status,
            "conclusion": conclusion,
            "head_sha": "0123456789abcdef"
        }]
    })
}

fn assert_refused(output: Output, cause: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("aggregate"), "stderr: {stderr}");
    assert!(stderr.contains(cause), "stderr: {stderr}");
}

#[test]
fn empty_check_run_set_refuses_and_names_the_main_branch_cause() {
    let output = run(json!({"total_count": 0, "check_runs": []}));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main"), "stderr: {stderr}");
    assert!(stderr.contains("aggregate"), "stderr: {stderr}");
}

#[test]
fn completed_failure_refuses() {
    assert_refused(run(response("completed", Some("failure"))), "failure");
}

#[test]
fn completed_skipped_refuses() {
    assert_refused(run(response("completed", Some("skipped"))), "skipped");
}

#[test]
fn in_progress_refuses() {
    assert_refused(run(response("in_progress", None)), "in_progress");
}

#[test]
fn completed_success_allows() {
    let output = run(response("completed", Some("success")));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], true);
    assert_eq!(report["check_name"], "aggregate");
}

#[test]
fn success_allows_over_the_stdin_transport_the_workflow_uses() {
    let output = run_stdin(response("completed", Some("success")));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], true);
    assert_eq!(report["head_sha"], "0123456789abcdef");
}

#[test]
fn empty_check_run_set_refuses_over_the_stdin_transport() {
    let output = run_stdin(json!({"total_count": 0, "check_runs": []}));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main"), "stderr: {stderr}");
}

/// A tagged SHA can carry an `aggregate` run per event. The guard judges the run the
/// API returned first (`filter=latest`); a later stale success must not wave it through.
#[test]
fn a_trailing_success_cannot_mask_the_latest_failure() {
    let output = run(json!({
        "total_count": 2,
        "check_runs": [
            {"name": "aggregate", "status": "completed", "conclusion": "failure", "head_sha": "abc"},
            {"name": "aggregate", "status": "completed", "conclusion": "success", "head_sha": "abc"}
        ]
    }));
    assert_refused(output, "failure");
}

#[test]
fn a_similarly_named_check_run_is_not_the_aggregate() {
    let output = run(json!({
        "total_count": 1,
        "check_runs": [{
            "name": "journal-aggregate",
            "status": "completed",
            "conclusion": "success",
            "head_sha": "abc"
        }]
    }));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main"), "stderr: {stderr}");
}
