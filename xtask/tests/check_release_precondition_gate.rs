//! Story 15-4 AC3 — the release workflow and these vectors share one predicate.
//! CI release-ready program W4 — the check-run(s) judged are a function of the tag kind: a release
//! candidate needs `release-ready`, GA needs `aggregate` AND `release-ready`, the alpha line keeps
//! `aggregate` alone, and a run under any other name never matches.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

fn run(kind: &str, check_runs: Value) -> Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("check-runs.json");
    std::fs::write(&path, serde_json::to_vec(&check_runs).unwrap()).expect("write fixture");
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "check-release-precondition",
            "--kind",
            kind,
            "--check-runs",
            path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run xtask")
}

/// `release.yml` pipes the `gh api` response(s) on stdin and passes no `--check-runs`.
/// Driving only the flag would leave the transport the release actually uses untested.
fn run_stdin_text(kind: &str, stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-release-precondition", "--kind", kind, "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn xtask");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin fixture");
    child.wait_with_output().expect("run xtask")
}

fn run_stdin(kind: &str, check_runs: Value) -> Output {
    run_stdin_text(kind, &serde_json::to_string(&check_runs).unwrap())
}

fn check_run(name: &str, status: &str, conclusion: Option<&str>) -> Value {
    json!({
        "name": name,
        "status": status,
        "conclusion": conclusion,
        "head_sha": "0123456789abcdef"
    })
}

/// One `check_name=<name>&filter=latest` response, as `gh api` prints it.
fn queried(name: &str, status: &str, conclusion: Option<&str>) -> Value {
    json!({"total_count": 1, "check_runs": [check_run(name, status, conclusion)]})
}

/// Both GA queries concatenated, as `release.yml` pipes them: one JSON document per name.
fn ga_stdin(aggregate: (&str, Option<&str>), ready: (&str, Option<&str>)) -> String {
    format!(
        "{}\n{}\n",
        queried("aggregate", aggregate.0, aggregate.1),
        queried("release-ready", ready.0, ready.1)
    )
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: Output, check: &str, cause: &str) {
    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains(check), "stderr: {stderr}");
    assert!(stderr.contains(cause), "stderr: {stderr}");
}

fn assert_allowed(output: &Output) -> Value {
    assert!(output.status.success(), "stderr: {}", stderr_of(output));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["passed"], true);
    report
}

// ---- alpha: the aggregate-only rule, unchanged ----------------------------------------------

#[test]
fn empty_check_run_set_refuses_and_names_the_main_branch_cause() {
    let output = run("alpha", json!({"total_count": 0, "check_runs": []}));
    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("main"), "stderr: {stderr}");
    assert!(stderr.contains("aggregate"), "stderr: {stderr}");
}

#[test]
fn completed_failure_refuses() {
    let output = run("alpha", queried("aggregate", "completed", Some("failure")));
    assert_refused(output, "aggregate", "failure");
}

#[test]
fn completed_skipped_refuses() {
    let output = run("alpha", queried("aggregate", "completed", Some("skipped")));
    assert_refused(output, "aggregate", "skipped");
}

#[test]
fn in_progress_refuses() {
    let output = run("alpha", queried("aggregate", "in_progress", None));
    assert_refused(output, "aggregate", "in_progress");
}

#[test]
fn completed_success_allows() {
    let output = run("alpha", queried("aggregate", "completed", Some("success")));
    let report = assert_allowed(&output);
    assert_eq!(report["kind"], "alpha");
    assert_eq!(report["check_names"], json!(["aggregate"]));
}

#[test]
fn success_allows_over_the_stdin_transport_the_workflow_uses() {
    let output = run_stdin("alpha", queried("aggregate", "completed", Some("success")));
    let report = assert_allowed(&output);
    assert_eq!(report["head_sha"], "0123456789abcdef");
}

#[test]
fn empty_check_run_set_refuses_over_the_stdin_transport() {
    let output = run_stdin("alpha", json!({"total_count": 0, "check_runs": []}));
    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("main"), "stderr: {stderr}");
}

/// A tagged SHA can carry a run per event. The guard judges the run the API returned first
/// (`filter=latest`); a later stale success must not wave it through.
#[test]
fn a_trailing_success_cannot_mask_the_latest_failure() {
    for (kind, name) in [("alpha", "aggregate"), ("rc", "release-ready")] {
        let output = run(
            kind,
            json!({
                "total_count": 2,
                "check_runs": [
                    check_run(name, "completed", Some("failure")),
                    check_run(name, "completed", Some("success"))
                ]
            }),
        );
        assert_refused(output, name, "failure");
    }
}

#[test]
fn a_similarly_named_check_run_is_not_the_aggregate() {
    for kind in ["alpha", "rc", "ga"] {
        let output = run(
            kind,
            queried("journal-aggregate", "completed", Some("success")),
        );
        assert!(!output.status.success(), "{kind}");
        let stderr = stderr_of(&output);
        assert!(stderr.contains("main"), "{kind}: {stderr}");
    }
}

// ---- rc: judged on release-ready ------------------------------------------------------------

#[test]
fn rc_allows_a_green_release_ready() {
    let output = run("rc", queried("release-ready", "completed", Some("success")));
    let report = assert_allowed(&output);
    assert_eq!(report["kind"], "rc");
    assert_eq!(report["check_names"], json!(["release-ready"]));
    let over_stdin = run_stdin("rc", queried("release-ready", "completed", Some("success")));
    assert_allowed(&over_stdin);
}

/// release-ready red, in progress, skipped, cancelled or absent: an rc is refused, whatever the
/// aggregate says.
#[test]
fn rc_refuses_a_release_ready_that_is_red_in_progress_skipped_or_absent() {
    for (status, conclusion, cause) in [
        ("completed", Some("failure"), "failure"),
        ("completed", Some("cancelled"), "cancelled"),
        ("completed", Some("skipped"), "skipped"),
        ("in_progress", None, "in_progress"),
        ("queued", None, "queued"),
    ] {
        let output = run("rc", queried("release-ready", status, conclusion));
        assert_refused(output, "release-ready", cause);
        let output = run_stdin("rc", queried("release-ready", status, conclusion));
        assert_refused(output, "release-ready", cause);
    }
    let absent = run("rc", json!({"total_count": 0, "check_runs": []}));
    assert_refused(absent, "release-ready", "main");
}

/// The point of the split: a red merge-path `aggregate` does not stop a release candidate whose
/// release-ready is green. The payload carries both names; only release-ready is judged.
#[test]
fn rc_allows_a_green_release_ready_beside_a_red_aggregate() {
    let both = json!({
        "total_count": 2,
        "check_runs": [
            check_run("aggregate", "completed", Some("failure")),
            check_run("release-ready", "completed", Some("success"))
        ]
    });
    let report = assert_allowed(&run("rc", both));
    assert_eq!(report["check_names"], json!(["release-ready"]));
}

/// A green `aggregate` is NOT a release-ready: the wrong name never matches.
#[test]
fn rc_never_accepts_a_green_aggregate_in_place_of_release_ready() {
    let output = run("rc", queried("aggregate", "completed", Some("success")));
    assert_refused(output, "release-ready", "main");
    let output = run_stdin("rc", queried("aggregate", "completed", Some("success")));
    assert_refused(output, "release-ready", "main");
}

// ---- ga: strict, aggregate AND release-ready --------------------------------------------------

#[test]
fn ga_allows_only_when_both_are_green() {
    let output = run_stdin_text(
        "ga",
        &ga_stdin(
            ("completed", Some("success")),
            ("completed", Some("success")),
        ),
    );
    let report = assert_allowed(&output);
    assert_eq!(report["kind"], "ga");
    assert_eq!(report["check_names"], json!(["aggregate", "release-ready"]));
}

/// A red `aggregate` refuses GA although release-ready is green (the case an rc is allowed).
#[test]
fn ga_refuses_a_red_aggregate_even_with_a_green_release_ready() {
    let stdin = ga_stdin(
        ("completed", Some("failure")),
        ("completed", Some("success")),
    );
    let output = run_stdin_text("ga", &stdin);
    assert_refused(output, "aggregate", "failure");
}

#[test]
fn ga_refuses_a_red_release_ready_even_with_a_green_aggregate() {
    let stdin = ga_stdin(
        ("completed", Some("success")),
        ("completed", Some("failure")),
    );
    let output = run_stdin_text("ga", &stdin);
    assert_refused(output, "release-ready", "failure");
}

/// GA with only `aggregate` green: release-ready is absent and the tag is refused.
#[test]
fn ga_refuses_when_only_aggregate_is_green() {
    let output = run_stdin("ga", queried("aggregate", "completed", Some("success")));
    assert_refused(output, "release-ready", "main");
    let green = check_run("aggregate", "completed", Some("success"));
    let output = run("ga", json!({"total_count": 1, "check_runs": [green]}));
    assert_refused(output, "release-ready", "main");
}

/// GA with only `release-ready` green: the strict aggregate is absent and the tag is refused.
#[test]
fn ga_refuses_when_only_release_ready_is_green() {
    let output = run_stdin("ga", queried("release-ready", "completed", Some("success")));
    assert_refused(output, "aggregate", "main");
}

/// Both red: the refusal names both, not only the first.
#[test]
fn ga_names_every_red_check() {
    let stdin = ga_stdin(("completed", Some("failure")), ("in_progress", None));
    let output = run_stdin_text("ga", &stdin);
    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("aggregate check-run conclusion is failure"),
        "{stderr}"
    );
    assert!(
        stderr.contains("release-ready check-run status is in_progress"),
        "{stderr}"
    );
}

// ---- the kind itself ------------------------------------------------------------------------

/// An unclassified kind keeps today's refusal, and nothing is read first.
#[test]
fn an_unclassified_kind_is_refused_before_any_input_is_read() {
    for kind in ["", "beta", "RC", "rc ga", "release-ready", "aggregate"] {
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args([
                "check-release-precondition",
                "--kind",
                kind,
                "--check-runs",
                "/nonexistent/check-runs.json",
            ])
            .output()
            .expect("run xtask");
        assert!(!output.status.success(), "{kind:?}");
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains("unclassified tag kind"),
            "{kind:?}: {stderr}"
        );
        assert!(!stderr.contains("failed to read"), "{kind:?}: {stderr}");
    }
}

/// No `--kind` is no default: the caller must say which check judges the tag.
#[test]
fn a_missing_kind_is_refused() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-release-precondition", "--check-runs", "/nonexistent"])
        .output()
        .expect("run xtask");
    assert!(!output.status.success());
    assert!(
        stderr_of(&output).contains("--kind"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn empty_or_malformed_input_is_refused_for_every_kind() {
    for kind in ["alpha", "rc", "ga"] {
        for text in ["", "   \n", "not json", "{\"check_runs\":[]} trailing"] {
            let output = run_stdin_text(kind, text);
            assert!(!output.status.success(), "{kind}/{text:?}");
            assert!(
                stderr_of(&output).contains("invalid GitHub check-runs response"),
                "{kind}/{text:?}: {}",
                stderr_of(&output)
            );
        }
    }
}
