//! CI release-ready program W4 — the `release-ready` fan-in in `discipline.yml` is run as the script it
//! is: its `run:` text is extracted from the workflow YAML and executed under bash with a synthetic
//! `needs` context (the same shape `toJSON(needs)` gives the hosted job), so a green YAML whose rule
//! does not decide cannot pass. Proven red: a failed, cancelled or skipped required job, a failed
//! required `xtask-gates` step, an `xtask-gates` failure no step explains, and an applicable macOS guard
//! that did not run all red it; a failed EXCLUDED `xtask-gates` step does not, and is recorded.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{json, Map, Value};

const JUDGE_STEP: &str = "Judge the release-critical set";

fn workflow() -> serde_yaml::Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/discipline.yml");
    serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// What the script reads from the workflow.
struct FanIn {
    script: String,
    excluded: String,
    required: String,
    needs: Vec<String>,
    xtask_gate_outputs: Vec<String>,
}

fn fan_in() -> FanIn {
    let workflow = workflow();
    let job = &workflow["jobs"]["release-ready"];
    let step = job["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|step| step["name"].as_str() == Some(JUDGE_STEP))
        .unwrap_or_else(|| panic!("step {JUDGE_STEP:?} must exist"));
    let names = |value: &serde_yaml::Value| -> Vec<String> {
        value
            .as_sequence()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_string())
            .collect()
    };
    FanIn {
        script: step["run"].as_str().unwrap().to_string(),
        excluded: step["env"]["EXCLUDED_XTASK_GATES"]
            .as_str()
            .unwrap()
            .to_string(),
        required: step["env"]["REQUIRED_XTASK_STEPS"]
            .as_str()
            .unwrap()
            .to_string(),
        needs: names(&job["needs"]),
        xtask_gate_outputs: workflow["jobs"]["xtask-gates"]["outputs"]
            .as_mapping()
            .unwrap()
            .keys()
            .map(|key| key.as_str().unwrap().to_string())
            .collect(),
    }
}

/// What the script produced: the process output and the job summary file.
struct Judged {
    output: Output,
    summary: String,
}

impl Judged {
    fn text(&self) -> String {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&self.output.stdout),
            String::from_utf8_lossy(&self.output.stderr)
        )
    }
}

/// Run the fan-in script over an all-green `needs` context after `edit` has changed it.
fn judge(edit: impl FnOnce(&mut Map<String, Value>)) -> Judged {
    let fan_in = fan_in();
    let mut context = Map::new();
    for need in &fan_in.needs {
        context.insert(need.clone(), json!({ "result": "success", "outputs": {} }));
    }
    let gates: Map<String, Value> = fan_in
        .xtask_gate_outputs
        .iter()
        .map(|key| (key.clone(), json!("success")))
        .collect();
    context.insert(
        "xtask-gates".into(),
        json!({ "result": "success", "outputs": gates }),
    );
    context.insert(
        "macos-scope".into(),
        json!({ "result": "success", "outputs": { "applicable": "true" } }),
    );
    edit(&mut context);
    let dir = tempfile::tempdir().unwrap();
    let summary_path = dir.path().join("summary.md");
    std::fs::write(&summary_path, "").unwrap();
    let output = Command::new("bash")
        .arg("-c")
        .arg(&fan_in.script)
        .current_dir(dir.path())
        .env("NEEDS_JSON", Value::Object(context).to_string())
        .env("EXCLUDED_XTASK_GATES", &fan_in.excluded)
        .env("REQUIRED_XTASK_STEPS", &fan_in.required)
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .output()
        .expect("bash");
    Judged {
        output,
        summary: std::fs::read_to_string(&summary_path).unwrap(),
    }
}

fn set_job(context: &mut Map<String, Value>, job: &str, result: &str) {
    context
        .get_mut(job)
        .unwrap_or_else(|| panic!("{job} is a need"))["result"] = json!(result);
}

fn set_gates(context: &mut Map<String, Value>, result: &str, outcomes: &[(&str, &str)]) {
    let gates = context.get_mut("xtask-gates").unwrap();
    gates["result"] = json!(result);
    for (step, outcome) in outcomes {
        assert!(
            gates["outputs"].get(*step).is_some(),
            "{step} is an xtask-gates output"
        );
        gates["outputs"][*step] = json!(outcome);
    }
}

fn assert_red(judged: &Judged, needle: &str) {
    let text = judged.text();
    assert!(!judged.output.status.success(), "must be red: {text}");
    assert!(text.contains(needle), "{needle:?} not in: {text}");
}

fn assert_green(judged: &Judged) {
    assert!(judged.output.status.success(), "{}", judged.text());
}

#[test]
fn an_all_green_context_is_green() {
    let judged = judge(|_| {});
    assert_green(&judged);
    assert!(judged.text().contains("every required gate passed"));
    assert!(judged
        .summary
        .contains("| workspace-test-suite | success |"));
    assert!(!judged.text().contains("::warning"), "{}", judged.text());
}

#[test]
fn a_failed_cancelled_or_skipped_required_job_is_red() {
    for job in [
        "workspace-test-suite",
        "epic-20-exit",
        "check-export-control",
        "nfr-rel-9-revocation-5s-p99",
        "air-gap-netns-witness",
        "cgroup-ceiling-delegated",
    ] {
        for result in ["failure", "cancelled", "skipped"] {
            let judged = judge(|context| set_job(context, job, result));
            assert_red(&judged, &format!("{job}={result}"));
        }
    }
}

/// The aggregate rule: only `macos-check` may be skipped, and only when macos-scope said not applicable.
#[test]
fn the_macos_guard_follows_the_aggregate_rule() {
    let not_applicable = judge(|context| {
        set_job(context, "macos-check", "skipped");
        context.get_mut("macos-scope").unwrap()["outputs"]["applicable"] = json!("false");
    });
    assert_green(&not_applicable);
    assert!(not_applicable
        .text()
        .contains("NOT APPLICABLE (not a pass)"));
    let unran = judge(|context| set_job(context, "macos-check", "skipped"));
    assert_red(&unran, "macos-check applicable but did not run");
    let scope_failed = judge(|context| {
        set_job(context, "macos-check", "skipped");
        set_job(context, "macos-scope", "failure");
    });
    assert_red(&scope_failed, "macos-scope=failure");
    let check_failed = judge(|context| set_job(context, "macos-check", "failure"));
    assert_red(&check_failed, "macos-check=failure");
}

/// A required `xtask-gates` step (an R-set gate) reds the fan-in, alone or beside an excluded failure.
#[test]
fn a_failed_required_xtask_gate_step_is_red() {
    for step in [
        "check-breaking-md",
        "check-stability-matrix",
        "check-pentest-gate",
        "build-xtask",
    ] {
        let judged = judge(|context| set_gates(context, "failure", &[(step, "failure")]));
        assert_red(&judged, &format!("{step}=failure"));
        let beside_excluded = judge(|context| {
            let outcomes = [(step, "failure"), ("check-env-contract", "failure")];
            set_gates(context, "failure", &outcomes);
        });
        assert_red(&beside_excluded, &format!("{step}=failure"));
    }
    // A required step that never ran (skipped behind a failed one) is not success.
    let skipped = judge(|context| {
        set_gates(context, "failure", &[("check-breaking-md", "skipped")]);
    });
    assert_red(&skipped, "check-breaking-md=skipped");
}

/// The point of the excluded steps: red there does NOT red the fan-in, and is never hidden.
#[test]
fn a_failed_excluded_xtask_gate_step_does_not_red_but_is_recorded() {
    let pinned = fan_in().excluded;
    let excluded: Vec<&str> = pinned.split_whitespace().collect();
    assert_eq!(
        excluded,
        [
            "check-env-contract",
            "check-test-cost",
            "check-test-cost-vectors"
        ]
    );
    for step in &excluded {
        let step = *step;
        let judged = judge(|context| set_gates(context, "failure", &[(step, "failure")]));
        assert_green(&judged);
        let warning = format!(
            "::warning title=release-ready records an excluded gate failure::EXCLUDED merge-path xtask-gates step(s) not green: {step}=failure"
        );
        assert!(
            judged.text().contains(&warning),
            "{step}: {}",
            judged.text()
        );
        let row = format!("| xtask-gates / {step} | failure (EXCLUDED: not required here) |");
        assert!(judged.summary.contains(&row), "{step}: {}", judged.summary);
        assert!(judged
            .summary
            .contains("EXCLUDED merge-path xtask-gates step(s) not green"));
    }
    let every = judge(|context| {
        let outcomes: Vec<(&str, &str)> = excluded.iter().map(|step| (*step, "failure")).collect();
        set_gates(context, "failure", &outcomes);
    });
    assert_green(&every);
}

/// An `xtask-gates` failure that no step output explains is red: it must not hide behind the exclusion.
#[test]
fn an_unexplained_xtask_gates_failure_is_red() {
    let judged = judge(|context| set_gates(context, "failure", &[]));
    assert_red(&judged, "failed but no gate step output explains it");
    for result in ["cancelled", "skipped"] {
        let judged = judge(|context| {
            set_gates(context, result, &[]);
            context.get_mut("xtask-gates").unwrap()["outputs"] = json!({});
        });
        assert_red(&judged, "xtask-gates published no build-xtask outcome");
    }
}

/// Nothing outside the pinned list is excusable: every other gate step reds the fan-in.
#[test]
fn only_the_pinned_excluded_steps_are_excused() {
    let fan_in = fan_in();
    let excluded: Vec<&str> = fan_in.excluded.split_whitespace().collect();
    for step in fan_in
        .xtask_gate_outputs
        .iter()
        .filter(|step| !excluded.contains(&step.as_str()))
    {
        let step = step.as_str();
        let judged = judge(|context| set_gates(context, "failure", &[(step, "failure")]));
        assert_red(&judged, &format!("{step}=failure"));
    }
}

#[test]
fn the_fan_in_runs_always_and_carries_a_ceiling() {
    let workflow = workflow();
    let job = &workflow["jobs"]["release-ready"];
    assert_eq!(
        job["if"].as_str(),
        Some("always()"),
        "a skipped fan-in must not read as a pass"
    );
    assert!(job["timeout-minutes"].as_u64().is_some());
    assert!(job["continue-on-error"].is_null());
    for step in job["steps"].as_sequence().unwrap() {
        assert!(
            step["continue-on-error"].is_null(),
            "no step of the release-critical fan-in may continue on error"
        );
    }
}

fn remove_output(context: &mut Map<String, Value>, step: &str) {
    context.get_mut("xtask-gates").unwrap()["outputs"]
        .as_object_mut()
        .unwrap()
        .remove(step)
        .unwrap_or_else(|| panic!("{step} is an xtask-gates output"));
}

/// Fail closed on a dropped output: a required step whose output key is ABSENT is red, with the job
/// failed (its failed step invisible) or reported green, and never excused as an excluded failure.
#[test]
fn a_missing_required_xtask_output_is_red() {
    let fan_in = fan_in();
    for step in fan_in.required.split_whitespace() {
        for result in ["failure", "success"] {
            let judged = judge(|context| {
                set_gates(context, result, &[]);
                remove_output(context, step);
            });
            let needle = if step == "build-xtask" {
                "xtask-gates published no build-xtask outcome".to_string()
            } else {
                format!("{step}=missing")
            };
            assert_red(&judged, &needle);
        }
    }
    // The reported case: `check-unsafe`'s output dropped, `xtask-gates` failed beside an excluded failure.
    let judged = judge(|context| {
        set_gates(context, "failure", &[("check-env-contract", "failure")]);
        remove_output(context, "check-unsafe");
    });
    assert_red(&judged, "check-unsafe=missing");
}

/// The required list is every `xtask-gates` output except the excluded ones, so no step is unlisted.
#[test]
fn the_required_list_is_every_xtask_output_but_the_excluded() {
    let fan_in = fan_in();
    let excluded: Vec<&str> = fan_in.excluded.split_whitespace().collect();
    let mut expected: Vec<&str> = fan_in
        .xtask_gate_outputs
        .iter()
        .map(String::as_str)
        .filter(|step| !excluded.contains(step))
        .collect();
    let mut required: Vec<&str> = fan_in.required.split_whitespace().collect();
    expected.sort_unstable();
    required.sort_unstable();
    assert_eq!(required, expected);
}
