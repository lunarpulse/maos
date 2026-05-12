use super::*;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("maos-cm-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn fr_rows() -> String {
    let mut rows = String::new();
    for n in 1..=65 {
        rows.push_str(&format!(
            r#"  FR{n}:
    gates: []
    corpora: []
    phase: "v0.3"
    valid_until: "2027-05-12"
"#
        ));
    }
    rows
}

fn make_yaml(dir: &Path, mode: &str, coverage: &str) -> PathBuf {
    let path = dir.join("coverage-matrix.yaml");
    std::fs::write(
        &path,
        format!(
            r#"schema_version: 1
current_phase: "v0.1-alpha"
mode: "{}"
phase_order:
  - "v0.1-alpha"
  - "v0.1"
  - "v0.3"
coverage:
{}{}"#,
            mode, coverage, fr_rows()
        ),
    )
    .unwrap();
    path
}

fn make_phase_config(dir: &Path) -> PathBuf {
    let path = dir.join("phase-config.toml");
    std::fs::write(
        &path,
        r#"current_phase = "v0.1-alpha"
phase_order = ["v0.1-alpha", "v0.1", "v0.3"]
"#,
    )
    .unwrap();
    path
}

fn make_manifest(dir: &Path) -> PathBuf {
    let path = dir.join("MANIFEST.toml");
    std::fs::write(&path, "[corpus]\n").unwrap();
    path
}

fn make_registry(dir: &Path) -> PathBuf {
    let path = dir.join("gate-registry.toml");
    std::fs::write(&path, r#"gates = ["check-corpus", "coverage-matrix"]"#).unwrap();
    path
}

#[test]
fn empty_coverage_passes() {
    let dir = tmp_dir("empty");
    let yaml = make_yaml(&dir, "warning", "");
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(report.passed);
    assert_eq!(report.checked, 65); // 65 FR rows from fr_rows()
    cleanup(&dir);
}

#[test]
fn uncovered_delivered_row_violates() {
    let dir = tmp_dir("uncovered");
    let yaml = make_yaml(
        &dir,
        "warning",
        r#"  I9:
    gates: []
    corpora: []
    phase: "v0.1-alpha"
    valid_until: "2027-05-11"
"#,
    );
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.id == "I9"));
    cleanup(&dir);
}

#[test]
fn deferred_row_goes_to_out_of_scope() {
    let dir = tmp_dir("deferred");
    let yaml = make_yaml(
        &dir,
        "warning",
        r#"  Future:
    gates: []
    corpora: []
    phase: "v0.3"
    valid_until: "2027-05-11"
"#,
    );
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(report.passed);
    assert_eq!(report.out_of_scope_deferred.len(), 66); // 65 FR rows + 1 Future row
    assert!(report.out_of_scope_deferred.iter().any(|d| d.id == "Future"));
    cleanup(&dir);
}

#[test]
fn dangling_corpus_ref_fails() {
    let dir = tmp_dir("dangling-corpus");
    let yaml = make_yaml(
        &dir,
        "warning",
        r#"  I9:
    gates: ["check-corpus"]
    corpora: ["missing"]
    phase: "v0.1-alpha"
    valid_until: "2027-05-11"
"#,
    );
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.message.contains("unknown corpus")));
    cleanup(&dir);
}

#[test]
fn dangling_gate_ref_fails() {
    let dir = tmp_dir("dangling-gate");
    let yaml = make_yaml(
        &dir,
        "warning",
        r#"  I9:
    gates: ["unknown-gate"]
    corpora: []
    phase: "v0.1-alpha"
    valid_until: "2027-05-11"
"#,
    );
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.message.contains("unknown gate")));
    cleanup(&dir);
}

#[test]
fn mode_warning_returns_zero_with_violations() {
    let dir = tmp_dir("mode");
    let yaml = make_yaml(
        &dir,
        "warning",
        r#"  I9:
    gates: []
    corpora: []
    phase: "v0.1-alpha"
    valid_until: "2027-05-11"
"#,
    );
    let phase = make_phase_config(&dir);
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(!report.passed);
    assert_eq!(report.mode, "warning");
    cleanup(&dir);
}

#[test]
fn phase_config_mismatch_fails() {
    let dir = tmp_dir("mismatch");
    let yaml = make_yaml(&dir, "warning", "");
    let phase = dir.join("phase-config.toml");
    std::fs::write(&phase, r#"current_phase = "v0.3"
phase_order = ["v0.1-alpha", "v0.1", "v0.3"]
"#).unwrap();
    let manifest = make_manifest(&dir);
    let registry = make_registry(&dir);

    let report = check_coverage_matrix(&yaml, &phase, &manifest, &registry).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.message.contains("phase mismatch")));
    cleanup(&dir);
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        mode: "warning".into(),
        violations: vec![Violation {
            id: "I9".into(),
            message: "test".into(),
        }],
        out_of_scope_deferred: vec![DeferredRow {
            id: "Future".into(),
            phase: "v0.3".into(),
        }],
        checked: 2,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.out_of_scope_deferred.len(), 1);
}
