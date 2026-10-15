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

fn failure_selected(predicate: &str, status_output: &str) -> bool {
    assert_eq!(
        predicate, "${{ steps.status.outputs.overall == 'failure' }}",
        "failure step must consume the executed status step's output"
    );
    status_output.lines().any(|line| line == "overall=failure")
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
fn output(script: &str, needs: &BTreeMap<String, String>) -> String {
    let dir = fresh_dir("maos-cgroup-workflow");
    let file = dir.join("output");
    let needs: BTreeMap<_, _> = needs
        .iter()
        .map(|(name, result)| (name, serde_json::json!({ "result": result })))
        .collect();
    let status = Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script)
        .env("GITHUB_OUTPUT", &file)
        .env(
            "NEEDS_JSON",
            serde_json::to_string(&needs).expect("needs JSON"),
        )
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
            assert!(
                needs.contains_key(failed),
                "{failed} must remain an aggregate need"
            );
            let observed = output(script, &needs);
            println!("{failed}={state}: {}", observed.trim());
            assert_eq!(observed.trim(), "overall=failure", "{failed}={state}");
            let selected = failure_selected(failure_if, &observed);
            assert!(selected, "failure step did not select {failed}={state}");
            assert!(
                !run_successful(failure_run),
                "failure step exited successfully with {failed}={state}"
            );
        }
    }
    let observed = output(script, &successful_dependencies);
    println!("all-success control: {}", observed.trim());
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(failure_if, &observed),
        "failure step selected all-success control"
    );
}
