use super::*;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("maos-cs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn make_yaml(dir: &Path, valid_until: &str) -> PathBuf {
    let path = dir.join("coverage-matrix.yaml");
    std::fs::write(
        &path,
        format!(
            r#"schema_version: 1
current_phase: "v0.1-alpha"
mode: "warning"
phase_order:
  - "v0.1-alpha"
  - "v0.1"
coverage:
  Test:
    gates: ["check-corpus"]
    corpora: []
    phase: "v0.1-alpha"
    valid_until: "{}"
"#,
            valid_until
        ),
    )
    .unwrap();
    path
}

fn make_manifest(dir: &Path, valid_until: &str) -> PathBuf {
    let path = dir.join("MANIFEST.toml");
    std::fs::write(
        &path,
        format!(
            r#"[corpus.test]
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
schema_version = 1
item_count = 0
valid_until = "{}"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "test"
"#,
            valid_until
        ),
    )
    .unwrap();
    path
}

#[test]
fn expired_row_violates() {
    let dir = tmp_dir("expired");
    let yaml = make_yaml(&dir, "2020-01-01");
    let manifest = dir.join("MANIFEST.toml");
    std::fs::write(&manifest, "[corpus]\n").unwrap();

    let today = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
    let report = check_staleness(&yaml, &manifest, today, 30).unwrap();
    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    assert!(report.violations[0].message.contains("NFR-Meta-2 violation"));
    cleanup(&dir);
}

#[test]
fn not_yet_due_row_passes() {
    let dir = tmp_dir("due");
    let yaml = make_yaml(&dir, "2027-05-11");
    let manifest = dir.join("MANIFEST.toml");
    std::fs::write(&manifest, "[corpus]\n").unwrap();

    let today = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
    let report = check_staleness(&yaml, &manifest, today, 30).unwrap();
    assert!(report.passed);
    assert!(report.violations.is_empty());
    cleanup(&dir);
}

#[test]
fn within_warn_window_emits_warning() {
    let dir = tmp_dir("warn");
    let yaml = make_yaml(&dir, "2026-05-20");
    let manifest = dir.join("MANIFEST.toml");
    std::fs::write(&manifest, "[corpus]\n").unwrap();

    let today = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
    let report = check_staleness(&yaml, &manifest, today, 30).unwrap();
    assert!(report.passed);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].days_remaining, 8);
    cleanup(&dir);
}

#[test]
fn manifest_expired_violates() {
    let dir = tmp_dir("manifest");
    let yaml = dir.join("coverage-matrix.yaml");
    std::fs::write(
        &yaml,
        r#"schema_version: 1
current_phase: "v0.1-alpha"
mode: "warning"
phase_order:
  - "v0.1-alpha"
  - "v0.1"
coverage:
  Future:
    gates: []
    corpora: []
    phase: "v0.1"
    valid_until: "2027-05-11"
"#,
    )
    .unwrap();
    let manifest = make_manifest(&dir, "2020-01-01");

    let today = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
    let report = check_staleness(&yaml, &manifest, today, 30).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.id == "test"));
    cleanup(&dir);
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            kind: "expired".into(),
            id: "Test".into(),
            valid_until: "2020-01-01".into(),
            current_date: "2026-05-12".into(),
            message: "test message".into(),
        }],
        warnings: vec![Warning {
            id: "Near".into(),
            valid_until: "2026-06-01".into(),
            current_date: "2026-05-12".into(),
            days_remaining: 20,
            message: "near expiry".into(),
        }],
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
    assert_eq!(parsed.warnings.len(), 1);
}
