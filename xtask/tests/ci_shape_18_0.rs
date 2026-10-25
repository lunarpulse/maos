//! Story 18-0 AC6 (and AC2.3's enrolment) — the live `discipline.yml` shape of
//! the `expensive-tests` job, the two proven-red steps and the blocking
//! `cargo-deny` job. Parses the real workflow as configuration (the
//! `release_workflow_15_4.rs` precedent); reads YAML and TOML only.

use std::fs;
use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

/// The `!cancelled()` condition after normalisation (review DD): `${{ }}`
/// stripped and whitespace removed, so `if: "!cancelled()"` and
/// `if: ${{ !cancelled() }}` are the same condition.
const NOT_CANCELLED: &str = "!cancelled()";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn workflow() -> Value {
    let path = root().join(".github/workflows/discipline.yml");
    serde_yaml::from_str(&fs::read_to_string(path).expect("read discipline workflow"))
        .expect("parse discipline workflow")
}

fn mapping(value: &Value) -> &Mapping {
    value.as_mapping().expect("expected mapping")
}

fn get<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    mapping(value).get(Value::String(name.to_string()))
}

fn key<'a>(value: &'a Value, name: &str) -> &'a Value {
    get(value, name).unwrap_or_else(|| panic!("missing key {name}"))
}

fn str_of<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    get(value, name).and_then(Value::as_str)
}

fn steps(job: &Value) -> &[Value] {
    key(job, "steps").as_sequence().expect("job steps")
}

fn step_index(job: &Value, name: &str) -> usize {
    steps(job)
        .iter()
        .position(|step| str_of(step, "name") == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}

/// `needs` as a list; a scalar `needs: job` is accepted (review DD); a job
/// without `needs` has none.
fn needs(jobs: &Value, job: &str) -> Vec<String> {
    match get(key(jobs, job), "needs") {
        None => Vec::new(),
        Some(Value::String(one)) => vec![one.clone()],
        Some(list) => list
            .as_sequence()
            .expect("needs list or string")
            .iter()
            .map(|need| need.as_str().expect("need string").to_string())
            .collect(),
    }
}

/// An `if:` condition with any `${{ … }}` wrapper and all whitespace removed.
fn condition(step: &Value) -> Option<String> {
    let raw: String = str_of(step, "if")?
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    Some(
        raw.strip_prefix("${{")
            .and_then(|inner| inner.strip_suffix("}}"))
            .unwrap_or(&raw)
            .to_string(),
    )
}

/// True when `run` invokes `--test <name>` as a whole token (review DD: flags
/// such as an added `--locked` do not matter).
fn runs_test(run: &str, name: &str) -> bool {
    let tokens: Vec<&str> = run.split_whitespace().collect();
    tokens.windows(2).any(|w| w == ["--test", name])
}

/// Review 2026-10-25: a step- or job-level `continue-on-error` is the exact
/// masking 18-0 closes; it must be absent (or literally `false`).
fn assert_no_continue_on_error(value: &Value, what: &str) {
    match get(value, "continue-on-error") {
        None | Some(Value::Bool(false)) => {}
        Some(other) => panic!("{what} must not carry continue-on-error (found {other:?})"),
    }
}

#[test]
fn proven_red_steps_run_after_a_red_gate_step() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for (job_name, step, test) in [
        (
            "check-test-cost",
            "Run the 20-12 AC2 proven-red vectors",
            "check_test_cost_proven_red",
        ),
        (
            "check-test-budget",
            "Run the 20-12 AC1 proven-red vectors",
            "check_test_budget_proven_red",
        ),
    ] {
        let job = key(jobs, job_name);
        assert_no_continue_on_error(job, job_name);
        let step = &steps(job)[step_index(job, step)];
        let run = str_of(step, "run").expect("proven-red step run");
        assert!(runs_test(run, test), "'{run}' runs --test {test}");
        assert_eq!(
            condition(step).as_deref(),
            Some(NOT_CANCELLED),
            "'{run}' must run after a red gate step (18-0 AC6.3), not be skipped by it"
        );
        assert_no_continue_on_error(step, &format!("{job_name} step '{run}'"));
    }
}

#[test]
fn expensive_tests_uploads_its_junit_and_builds_no_fixtures() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "expensive-tests");
    assert_no_continue_on_error(job, "expensive-tests");
    let run = step_index(job, "Run the registered expensive tests");
    assert_no_continue_on_error(&steps(job)[run], "the expensive-tests run step");
    let upload_at = step_index(job, "Upload the expensive-tests junit");
    assert!(upload_at > run, "the junit upload follows the run");
    let upload = &steps(job)[upload_at];
    assert!(
        str_of(upload, "uses").is_some_and(|uses| uses.starts_with("actions/upload-artifact@")),
        "the junit is uploaded with actions/upload-artifact"
    );
    assert_eq!(
        condition(upload).as_deref(),
        Some(NOT_CANCELLED),
        "a red expensive run still uploads its junit"
    );
    let with = key(upload, "with");
    assert_eq!(str_of(with, "name"), Some("expensive-tests-junit"));
    assert_eq!(str_of(with, "if-no-files-found"), Some("error"));

    // The uploaded path is where `--profile expensive` writes its junit.
    let nextest: toml::Value = toml::from_str(
        &fs::read_to_string(root().join(".config/nextest.toml")).expect("read nextest config"),
    )
    .expect("parse nextest config");
    let junit = nextest["profile"]["expensive"]["junit"]["path"]
        .as_str()
        .expect("profile.expensive.junit.path");
    assert_eq!(
        str_of(with, "path"),
        Some(format!("target/nextest/expensive/{junit}").as_str())
    );

    // D-18-0-09 A: no fixture build until a registered row needs one.
    assert!(
        steps(job)
            .iter()
            .filter_map(|step| str_of(step, "run"))
            .all(|run| !run.contains("build-test-fixtures")),
        "expensive-tests builds no fixtures (D-18-0-09)"
    );
}

/// D-18-0-10 A / D-15-4-F (and D-18-0-04's enrolment): an unregistered job is
/// enrolled in `aggregate.needs` and in no other job's `needs`.
#[test]
fn unregistered_jobs_are_enrolled_in_aggregate_only() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    for unregistered in ["expensive-tests", "cargo-deny"] {
        assert!(
            needs(jobs, "aggregate").contains(&unregistered.to_string()),
            "{unregistered} is in aggregate.needs"
        );
        for name in mapping(jobs).keys().filter_map(Value::as_str) {
            if name == "aggregate" {
                continue;
            }
            assert!(
                !needs(jobs, name).contains(&unregistered.to_string()),
                "{unregistered} must be enrolled in aggregate.needs only, not in {name}.needs"
            );
        }
    }
}

#[test]
fn cargo_deny_is_its_own_blocking_job_in_aggregate() {
    let workflow = workflow();
    let jobs = key(&workflow, "jobs");
    let job = key(jobs, "cargo-deny");
    assert_no_continue_on_error(job, "the supply-chain gate (D-18-0-04)");
    assert!(key(job, "timeout-minutes").as_u64().is_some());
    let check = &steps(job)[step_index(job, "cargo deny check")];
    assert!(
        str_of(check, "run").is_some_and(|run| run.starts_with("cargo deny check")),
        "the cargo deny check step runs cargo deny check"
    );
    for step in steps(job)
        .iter()
        .filter(|step| str_of(step, "run").is_some_and(|run| run.contains("cargo deny")))
    {
        assert_no_continue_on_error(step, "a cargo deny step");
    }
    // The advisory job no longer hides it.
    assert!(
        steps(key(jobs, "reproducible-build"))
            .iter()
            .filter_map(|step| str_of(step, "run"))
            .all(|run| !run.contains("cargo deny")),
        "cargo deny no longer runs inside the ADVISORY reproducible-build"
    );
}
