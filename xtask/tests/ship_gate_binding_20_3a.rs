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
fn real_tree_has_44_bound_and_enrolled_gates() {
    let copy = tree();
    let output = run(copy.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["expected_count"], 44);
}

#[test]
fn removing_per_commit_ship_edge_reds() {
    let copy = tree();
    drop_need(copy.path(), "v1-0-ship-gate", "check-breaking-md");
    red(run(copy.path()), "check-breaking-md");
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
