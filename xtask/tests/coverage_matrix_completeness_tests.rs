//! Story 10.1b AC3, Task 3.4 — proven-red test for check-coverage-matrix-completeness.
//! Empty one v1.0 NFR's gates array → xtask must fail; restore → must pass.

use std::io::Write;

fn workspace_path(rel: &str) -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().join(rel)
}

/// Proven-red: create a coverage matrix with a v1.0 NFR that has empty gates → must fail.
#[test]
fn fails_on_empty_v1_0_gates() {
    let dir = tempfile::tempdir().unwrap();
    let matrix_path = dir.path().join("tests/coverage-matrix.yaml");
    std::fs::create_dir_all(matrix_path.parent().unwrap()).unwrap();

    let yaml = r#"
schema_version: 1
current_phase: v1.0
coverage:
  NFR-Test-Empty:
    gates: []
    corpora: []
    phase: v1.0
    valid_until: '2027-01-01'
"#;
    let mut f = std::fs::File::create(&matrix_path).unwrap();
    f.write_all(yaml.as_bytes()).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-coverage-matrix-completeness", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");

    assert!(
        !output.status.success(),
        "Must fail when v1.0 NFR has empty gates. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("NFR-Test-Empty"),
        "Output must name the failing NFR. stdout: {stdout}"
    );
}

/// Restore: v1.0 NFR with non-empty gates → must pass.
#[test]
fn passes_on_populated_v1_0_gates() {
    let dir = tempfile::tempdir().unwrap();
    let matrix_path = dir.path().join("tests/coverage-matrix.yaml");
    std::fs::create_dir_all(matrix_path.parent().unwrap()).unwrap();

    let yaml = r#"
schema_version: 1
current_phase: v1.0
coverage:
  NFR-Test-Populated:
    gates:
      - some-gate
    corpora: []
    phase: v1.0
    valid_until: '2027-01-01'
"#;
    let mut f = std::fs::File::create(&matrix_path).unwrap();
    f.write_all(yaml.as_bytes()).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-coverage-matrix-completeness", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");

    assert!(
        output.status.success(),
        "Must pass when v1.0 NFR has non-empty gates. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Advisory-until-engagement with empty gates counts as non-empty.
#[test]
fn advisory_until_engagement_counts_as_non_empty() {
    let dir = tempfile::tempdir().unwrap();
    let matrix_path = dir.path().join("tests/coverage-matrix.yaml");
    std::fs::create_dir_all(matrix_path.parent().unwrap()).unwrap();

    let yaml = r#"
schema_version: 1
current_phase: v1.0
coverage:
  NFR-Sec-7-Like:
    gates: []
    corpora: []
    enforcement: advisory-until-engagement
    phase: v1.0
    valid_until: '2027-01-01'
"#;
    let mut f = std::fs::File::create(&matrix_path).unwrap();
    f.write_all(yaml.as_bytes()).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-coverage-matrix-completeness", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run xtask");

    assert!(
        output.status.success(),
        "advisory-until-engagement must count as non-empty. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
