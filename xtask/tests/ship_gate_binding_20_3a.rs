//! AC4: run the shipped gate against isolated copies of the real registry and workflows.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tree() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let copy = tempfile::tempdir().unwrap();
    fs::create_dir(copy.path().join("xtask")).unwrap();
    fs::copy(
        root.join("xtask/gate-registry.toml"),
        copy.path().join("xtask/gate-registry.toml"),
    )
    .unwrap();
    let workflows = copy.path().join(".github/workflows");
    fs::create_dir_all(&workflows).unwrap();
    for entry in fs::read_dir(root.join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "yml") {
            fs::copy(&path, workflows.join(path.file_name().unwrap())).unwrap();
        }
    }
    copy
}

fn run(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-ship-gate-completeness", "--json"])
        .current_dir(root)
        .env_remove("MAOS_LEDGER_ARTIFACTS")
        .output()
        .unwrap()
}

fn plant(root: &Path, relative: &str, old: &str, new: &str) {
    let path = root.join(relative);
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(old),
        "real tree no longer contains plant {old:?}"
    );
    fs::write(path, text.replacen(old, new, 1)).unwrap();
}

fn workflow(root: &Path, name: &str) -> PathBuf {
    root.join(".github/workflows").join(name)
}

fn drop_need(root: &Path, job: &str, name: &str) {
    let path = workflow(root, "discipline.yml");
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let needs = yaml["jobs"][job]["needs"].as_sequence_mut().unwrap();
    let before = needs.len();
    needs.retain(|value| value.as_str() != Some(name));
    assert_eq!(needs.len() + 1, before, "plant must drop one real edge");
    fs::write(path, serde_yaml::to_string(&yaml).unwrap()).unwrap();
}

fn red(output: Output, reason: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!(
        "planted RED: rc={:?}; stdout={stdout}; stderr={stderr}",
        output.status.code()
    );
    assert!(!output.status.success(), "plant passed: {stdout} {stderr}");
    assert!(
        format!("{stdout}\n{stderr}").contains(reason),
        "expected {reason:?}: {stdout} {stderr}"
    );
}

#[test]
fn real_tree_has_41_bound_and_enrolled_gates() {
    let copy = tree();
    let output = run(copy.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["expected_count"], 41);
}

#[test]
fn removing_per_commit_ship_edge_reds() {
    let copy = tree();
    drop_need(copy.path(), "v1-0-ship-gate", "check-export-control");
    red(run(copy.path()), "check-export-control");
}

#[test]
fn removing_the_consolidated_job_edge_reds_every_gate_it_carries() {
    let copy = tree();
    drop_need(copy.path(), "v1-0-ship-gate", "xtask-gates");
    let output = run(copy.path());
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for gate in [
        "check-stability-matrix",
        "check-breaking-md",
        "check-pentest-gate",
        "check-third-party-trial",
        "check-red-team-gate",
        "check-env-contract",
        "check-test-cost",
    ] {
        assert!(
            text.contains(&format!(
                "{gate}: expected exactly one ship enrolment, found 0"
            )),
            "{gate}: {text}"
        );
    }
    red(output, "check-breaking-md");
}

#[test]
fn renaming_a_consolidated_gate_step_reds_that_gate() {
    let copy = tree();
    plant(
        copy.path(),
        ".github/workflows/discipline.yml",
        "      - name: check-breaking-md\n",
        "      - name: check-breaking-md-renamed\n",
    );
    red(
        run(copy.path()),
        "check-breaking-md: expected exactly one ship enrolment, found 0",
    );
}

#[test]
fn a_gate_enrolled_directly_and_as_a_consolidated_step_reds() {
    let copy = tree();
    let path = workflow(copy.path(), "discipline.yml");
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    yaml["jobs"]["v1-0-ship-gate"]["needs"]
        .as_sequence_mut()
        .unwrap()
        .push(serde_yaml::Value::String("check-breaking-md".into()));
    fs::write(path, serde_yaml::to_string(&yaml).unwrap()).unwrap();
    red(
        run(copy.path()),
        "check-breaking-md: expected exactly one ship enrolment, found 2",
    );
}

#[test]
fn registry_row_without_enrolment_reds() {
    let copy = tree();
    let path = copy.path().join("xtask/gate-registry.toml");
    let mut text = fs::read_to_string(&path).unwrap();
    text.push_str("\n[[ship_gate]]\nname = \"unregistered-ship-gate\"\ndisposition = { v1_0 = \"blocking\", v1_5 = \"blocking\" }\nbinding = \"blocking\"\n");
    fs::write(path, text).unwrap();
    red(run(copy.path()), "unregistered-ship-gate");
}

#[test]
fn removing_aggregate_ship_edge_reds() {
    let copy = tree();
    drop_need(copy.path(), "aggregate", "v1-0-ship-gate");
    red(run(copy.path()), "aggregate.needs");
}

#[test]
fn misnamed_cadence_jobs_red() {
    for (file, job) in [
        ("rpo-rto-cadence.yml", "rto-drill-t1"),
        ("rpo-rto-cadence.yml", "check-rto-gate"),
        ("fuzz-cadence.yml", "check-fuzz-floor"),
    ] {
        let copy = tree();
        plant(
            copy.path(),
            &format!(".github/workflows/{file}"),
            &format!("  {job}:"),
            &format!("  {job}-misnamed:"),
        );
        red(run(copy.path()), job);
    }
}

#[test]
fn missing_binding_is_refused_by_loader() {
    let copy = tree();
    plant(
        copy.path(),
        "xtask/gate-registry.toml",
        "binding = \"blocking\"\n",
        "",
    );
    red(run(copy.path()), "missing field `binding`");
}

#[test]
fn unknown_binding_is_refused_by_loader() {
    let copy = tree();
    plant(
        copy.path(),
        "xtask/gate-registry.toml",
        "binding = \"blocking\"",
        "binding = \"not-a-class\"",
    );
    red(run(copy.path()), "unknown variant");
}

#[test]
fn cadence_gate_cannot_also_enrol_per_commit() {
    let copy = tree();
    let path = workflow(copy.path(), "discipline.yml");
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    yaml["jobs"]["v1-0-ship-gate"]["needs"]
        .as_sequence_mut()
        .unwrap()
        .push(serde_yaml::Value::String("check-fuzz-floor".into()));
    fs::write(path, serde_yaml::to_string(&yaml).unwrap()).unwrap();
    red(run(copy.path()), "check-fuzz-floor");
}

// ---- CI release-ready program W4: `release-ready` is `aggregate` minus the exclusion table ----------

fn add_need(root: &Path, job: &str, name: &str) {
    let path = workflow(root, "discipline.yml");
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    yaml["jobs"][job]["needs"]
        .as_sequence_mut()
        .unwrap()
        .push(serde_yaml::Value::String(name.into()));
    fs::write(path, serde_yaml::to_string(&yaml).unwrap()).unwrap();
}

#[test]
fn real_tree_release_ready_is_aggregate_minus_twelve_documented_exclusions() {
    let copy = tree();
    let output = run(copy.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(workflow(copy.path(), "discipline.yml")).unwrap())
            .unwrap();
    let listed = yaml["jobs"]["release-ready"]["needs"]
        .as_sequence()
        .unwrap()
        .len();
    assert_eq!(report["release_ready_needs"], listed);
    assert_eq!(
        report["release_ready_excluded"].as_array().unwrap().len(),
        9
    );
}

#[test]
fn dropping_a_required_job_from_release_ready_reds() {
    for job in [
        "workspace-test-suite",
        "check-export-control",
        "epic-20-exit",
        "check-ff-j6",
    ] {
        let copy = tree();
        drop_need(copy.path(), "release-ready", job);
        red(
            run(copy.path()),
            &format!(
                "{job}: aggregate reaches it, but release-ready neither requires nor excludes it"
            ),
        );
    }
}

#[test]
fn keeping_an_excluded_job_in_release_ready_reds() {
    let copy = tree();
    add_need(copy.path(), "release-ready", "check-test-budget");
    red(
        run(copy.path()),
        "check-test-budget: excluded, yet in release-ready.needs",
    );
}

#[test]
fn a_release_ready_job_aggregate_does_not_reach_reds() {
    let copy = tree();
    add_need(copy.path(), "release-ready", "a-nightly-only-job");
    red(
        run(copy.path()),
        "a-nightly-only-job: in release-ready.needs but not reached by aggregate",
    );
    let copy = tree();
    drop_need(copy.path(), "aggregate", "check-kernel-baseline");
    red(
        run(copy.path()),
        "check-kernel-baseline: in release-ready.needs but not reached by aggregate",
    );
}

#[test]
fn the_excluded_xtask_gate_steps_env_must_match_the_table() {
    let pinned =
        "EXCLUDED_XTASK_GATES: \"check-env-contract check-test-cost check-test-cost-vectors\"";
    for changed in [
        "EXCLUDED_XTASK_GATES: \"check-env-contract check-test-cost\"",
        "EXCLUDED_XTASK_GATES: \"check-env-contract check-test-cost check-test-cost-vectors check-breaking-md\"",
    ] {
        let copy = tree();
        plant(copy.path(), ".github/workflows/discipline.yml", pinned, changed);
        red(run(copy.path()), "EXCLUDED_XTASK_GATES");
    }
}

#[test]
fn a_consolidated_gate_without_a_job_output_reds() {
    let copy = tree();
    plant(
        copy.path(),
        ".github/workflows/discipline.yml",
        "      check-stability-matrix: ${{ steps.check-stability-matrix.outcome }}\n",
        "",
    );
    red(
        run(copy.path()),
        "check-stability-matrix: xtask-gates step has no job output",
    );
}

#[test]
fn removing_a_pinned_job_from_both_lists_reds() {
    for job in [
        "nfr-rel-1-crash-detection-2s",
        "workspace-test-suite",
        "epic-20-exit",
        "package-recipes",
    ] {
        let copy = tree();
        drop_need(copy.path(), "aggregate", job);
        drop_need(copy.path(), "release-ready", job);
        red(
            run(copy.path()),
            &format!("{job}: pinned release-critical job missing from release-ready.needs"),
        );
    }
}

#[test]
fn replacing_a_pinned_job_in_both_lists_with_another_reds() {
    let copy = tree();
    for job in ["aggregate", "release-ready"] {
        drop_need(copy.path(), job, "nfr-rel-1-crash-detection-2s");
        add_need(copy.path(), job, "a-replacement-job");
    }
    let output = run(copy.path());
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("a-replacement-job: in release-ready.needs but not pinned"),
        "{text}"
    );
    red(
        output,
        "nfr-rel-1-crash-detection-2s: pinned release-critical job missing",
    );
}

#[test]
fn an_unknown_xtask_gates_step_reds() {
    let copy = tree();
    let path = workflow(copy.path(), "discipline.yml");
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let step: serde_yaml::Value =
        serde_yaml::from_str("{name: extra-gate, id: extra-gate, run: 'true'}").unwrap();
    yaml["jobs"]["xtask-gates"]["steps"]
        .as_sequence_mut()
        .unwrap()
        .push(step);
    fs::write(path, serde_yaml::to_string(&yaml).unwrap()).unwrap();
    let output = run(copy.path());
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("extra-gate: xtask-gates step has no job output"),
        "{text}"
    );
    red(
        output,
        "extra-gate: xtask-gates step is neither excluded nor pinned",
    );
}

#[test]
fn the_required_xtask_steps_env_must_be_every_non_excluded_step() {
    let copy = tree();
    plant(
        copy.path(),
        ".github/workflows/discipline.yml",
        "REQUIRED_XTASK_STEPS: \"build-xtask check-stability-matrix check-serde-error-handling",
        "REQUIRED_XTASK_STEPS: \"build-xtask check-serde-error-handling",
    );
    red(run(copy.path()), "REQUIRED_XTASK_STEPS");
}
