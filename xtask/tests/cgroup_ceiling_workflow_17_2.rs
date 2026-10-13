//! Story 17-2 — execute aggregate status handling against required-job results.
use serde_yaml::Value;
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

fn workflow() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/discipline.yml");
    serde_yaml::from_str(&fs::read_to_string(path).expect("discipline workflow"))
        .expect("workflow YAML")
}
fn get<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .as_mapping()
        .expect("mapping")
        .get(&Value::String(name.into()))
        .unwrap_or_else(|| panic!("missing {name}"))
}
fn steps(job: &Value) -> &[Value] {
    get(job, "steps").as_sequence().expect("steps")
}
fn named<'a>(job: &'a Value, name: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|s| get(s, "name").as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}

// Evaluate only the predicates accepted in the aggregate result-disposition
// condition. This intentionally is not a general GitHub Actions expression engine.
fn evaluate(expr: &str, needs: &BTreeMap<String, String>) -> bool {
    let e = expr
        .trim()
        .trim_start_matches("${{")
        .trim_end_matches("}}")
        .trim();
    e.split("||").any(|term| {
        let term = term.trim().trim_matches(|c| c == '(' || c == ')').trim();
        if term.starts_with("contains(needs.*.result,") {
            let state = term.split('\'').nth(1).expect("contains state");
            return needs.values().any(|v| v == state);
        }
        let start = term.find("needs['").expect("needs bracket result") + 7;
        let end = term[start..].find("']").expect("needs closing bracket") + start;
        let name = &term[start..end];
        let state = needs.get(name).map(String::as_str).unwrap_or("");
        assert!(
            term[end + 2..].trim().starts_with(".result != 'success'"),
            "unsupported required-job result predicate: {term}"
        );
        state != "success"
    })
}
fn render_run(script: &str, needs: &BTreeMap<String, String>) -> String {
    let start = script.find("${{").expect("GHA expression in status script");
    let end = script[start..].find("}}").expect("expression end") + start + 2;
    let condition = &script[start..end];
    script.replacen(
        condition,
        if evaluate(condition, needs) {
            "true"
        } else {
            "false"
        },
        1,
    )
}
fn fresh_dir(prefix: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).expect("fresh temp dir");
    path
}
fn output(script: &str) -> String {
    let dir = fresh_dir("maos-cgroup-workflow");
    let file = dir.join("output");
    let status = Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("GITHUB_OUTPUT", &file)
        .status()
        .expect("bash status script");
    let value = fs::read_to_string(&file).expect("GITHUB_OUTPUT");
    let _ = fs::remove_dir_all(dir);
    assert!(status.success(), "aggregate status shell failed");
    value
}
fn run_successful(script: &str) -> bool {
    let dir = fresh_dir("maos-cgroup-failure");
    let status = Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("GITHUB_OUTPUT", dir.join("output"))
        .status()
        .expect("failure step shell");
    let _ = fs::remove_dir_all(dir);
    status.success()
}
#[test]
fn aggregate_reports_every_required_job_non_success_as_failure() {
    let w = workflow();
    let aggregate = get(get(&w, "jobs"), "aggregate");
    let required = ["cgroup-ceiling-delegated", "cgroup-ceiling-undelegated"];
    let successful_dependencies: BTreeMap<String, String> = get(aggregate, "needs")
        .as_sequence()
        .expect("aggregate needs")
        .iter()
        .map(|value| {
            (
                value.as_str().expect("job name").to_owned(),
                "success".to_owned(),
            )
        })
        .collect();
    let status = named(aggregate, "Check for failures");
    let script = get(status, "run").as_str().expect("status shell script");
    let failure = named(aggregate, "Fail if any gate failed");
    let failure_if = get(failure, "if").as_str().expect("failure step predicate");
    let failure_run = get(failure, "run").as_str().expect("failure step shell");
    for failed in required {
        for state in ["skipped", "failure", "cancelled"] {
            let mut needs = successful_dependencies.clone();
            if let Some(result) = needs.get_mut(failed) {
                *result = state.to_owned();
            }
            assert_eq!(
                output(&render_run(script, &needs)).trim(),
                "overall=failure",
                "{failed}={state}"
            );
            assert!(
                evaluate(failure_if, &needs),
                "failure step did not select {failed}={state}"
            );
            assert!(
                !run_successful(failure_run),
                "failure step exited successfully with {failed}={state}"
            );
        }
    }
    let all = successful_dependencies;
    assert_eq!(output(&render_run(script, &all)).trim(), "overall=success");
    assert!(
        !evaluate(failure_if, &all),
        "failure step selected all-success control"
    );
}
