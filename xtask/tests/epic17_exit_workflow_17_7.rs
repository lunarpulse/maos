//! Story 17-7 — pin the T3 terminal-facts workflow route and aggregate gate.
use serde_yaml::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

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
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing step {name}"))
}
// Replant the old skip-ignoring predicate into a copy of the real workflow.
fn workflow_ignoring_skips() -> Value {
    let mut value = workflow();
    let step = value["jobs"]["aggregate"]["steps"]
        .as_sequence_mut()
        .expect("aggregate steps")
        .iter_mut()
        .find(|step| step["name"].as_str() == Some("Check for failures"))
        .expect("status step");
    let script = step["run"].as_str().expect("status script");
    let clause = " or (.key != \"macos-check\" and .value.result == \"skipped\")";
    assert!(
        script.contains(clause),
        "skip clause must exist before mutation"
    );
    step["run"] = Value::String(script.replace(clause, ""));
    value
}

fn dependencies(aggregate: &Value) -> BTreeMap<String, String> {
    get(aggregate, "needs")
        .as_sequence()
        .expect("aggregate needs")
        .iter()
        .map(|name| {
            (
                name.as_str().expect("job name").to_owned(),
                "success".to_owned(),
            )
        })
        .collect()
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
    let dir = fresh_dir("maos-terminal-workflow");
    let file = dir.join("output");
    let needs: BTreeMap<_, _> = needs
        .iter()
        .map(|(name, result)| (name, serde_json::json!({ "result": result })))
        .collect();
    let status = std::process::Command::new("bash")
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
    let dir = fresh_dir("maos-terminal-failure");
    let status = std::process::Command::new("bash")
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
fn aggregate_workflow_executes_non_success_handling_and_failure_step() {
    let value = workflow();
    let aggregate = &value["jobs"]["aggregate"];
    assert!(
        get(aggregate, "needs")
            .as_sequence()
            .expect("aggregate needs")
            .iter()
            .any(|dependency| dependency.as_str() == Some("epic-17-exit")),
        "epic-17-exit must be a required aggregate dependency"
    );
    let dependencies = dependencies(aggregate);
    let script = named(aggregate, "Check for failures")["run"]
        .as_str()
        .expect("status script");
    let failure = named(aggregate, "Fail if any gate failed");
    let failure_if = failure["if"].as_str().expect("failure predicate");
    let failure_run = failure["run"].as_str().expect("failure shell");

    for state in ["skipped", "failure", "cancelled"] {
        let job = "epic-17-exit";
        let mut needs = dependencies.clone();
        needs.insert(job.to_owned(), state.to_owned());
        let observed = output(script, &needs);
        println!("{job}={state}: {}", observed.trim());
        assert_eq!(observed.trim(), "overall=failure", "{job}={state}");
        assert!(
            failure_selected(failure_if, &observed),
            "failure step must run for {job}={state}"
        );
        assert!(
            !run_successful(failure_run),
            "failure step must exit nonzero for {job}={state}"
        );
    }

    let observed = output(script, &dependencies);
    println!("all-success control: {}", observed.trim());
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(failure_if, &observed),
        "failure step selected all-success control"
    );

    // The same executed vector exposes the old skip-ignoring predicate as a false green.
    let without = workflow_ignoring_skips();
    let omitted = &without["jobs"]["aggregate"];
    let omitted_script = named(omitted, "Check for failures")["run"]
        .as_str()
        .expect("status script without target predicate");
    let mut needs = dependencies.clone();
    needs.insert("epic-17-exit".into(), "skipped".into());
    let observed = output(omitted_script, &needs);
    println!(
        "planted skip-ignoring predicate, epic-17-exit=skipped: {}",
        observed.trim()
    );
    assert_eq!(observed.trim(), "overall=success");
    assert!(
        !failure_selected(
            named(omitted, "Fail if any gate failed")["if"]
                .as_str()
                .expect("failure predicate"),
            &observed
        ),
        "predicate removal must expose skipped job as missed failure"
    );
}
