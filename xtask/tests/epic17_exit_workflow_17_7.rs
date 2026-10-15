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
// Remove only the new required-job predicate from a scratch workflow copy.
fn workflow_with_predicate(predicate: bool) -> Value {
    let mut value = workflow();
    if !predicate {
        for step in value["jobs"]["aggregate"]["steps"]
            .as_sequence_mut()
            .expect("aggregate steps")
        {
            if step["name"].as_str() == Some("Check for failures") {
                let script = step["run"]
                    .as_str()
                    .expect("status script")
                    .replace(" || needs['epic-17-exit'].result != 'success'", "");
                step["run"] = Value::String(script);
            }
            if step["name"].as_str() == Some("Fail if any gate failed") {
                let predicate = " || needs['epic-17-exit'].result != 'success'";
                let condition = step["if"].as_str().expect("failure predicate");
                step["if"] = Value::String(condition.replace(predicate, ""));
            }
        }
    }
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
            return needs.values().any(|value| value == state);
        }
        let start = term.find("needs['").expect("needs bracket result") + 7;
        let end = term[start..].find("']").expect("needs closing bracket") + start;
        let name = &term[start..end];
        let state = needs.get(name).map(String::as_str).unwrap_or("");
        assert_eq!(
            term[end + 2..].trim(),
            ".result != 'success'",
            "unsupported required-job predicate: {term}"
        );
        state != "success"
    })
}

fn render_run(script: &str, needs: &BTreeMap<String, String>) -> String {
    let start = script.find("${{").expect("GHA expression in status script");
    let end = script[start..].find("}}").expect("expression end") + start + 2;
    script.replacen(
        &script[start..end],
        if evaluate(&script[start..end], needs) {
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
    let dir = fresh_dir("maos-terminal-workflow");
    let file = dir.join("output");
    let status = std::process::Command::new("bash")
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
        assert_eq!(
            output(&render_run(script, &needs)).trim(),
            "overall=failure",
            "{job}={state}"
        );
        assert!(
            evaluate(failure_if, &needs),
            "failure step must run for {job}={state}"
        );
        assert!(
            !run_successful(failure_run),
            "failure step must exit nonzero for {job}={state}"
        );
    }

    assert_eq!(
        output(&render_run(script, &dependencies)).trim(),
        "overall=success"
    );
    assert!(
        !evaluate(failure_if, &dependencies),
        "failure step selected all-success control"
    );

    // Removing the target predicate makes the status condition false for a
    // skipped dependency. Its shell would not run, leaving the default
    // successful output; the failure-step condition must catch this case.
    let without = workflow_with_predicate(false);
    let omitted = &without["jobs"]["aggregate"];
    let omitted_script = named(omitted, "Check for failures")["run"]
        .as_str()
        .expect("status script without target predicate");
    let mut needs = dependencies.clone();
    needs.insert("epic-17-exit".into(), "skipped".into());
    assert_eq!(
        output(&render_run(omitted_script, &needs)).trim(),
        "overall=success"
    );
    assert!(
        !evaluate(
            named(omitted, "Fail if any gate failed")["if"]
                .as_str()
                .expect("failure predicate"),
            &needs
        ),
        "predicate removal must expose skipped job as missed failure"
    );
}
