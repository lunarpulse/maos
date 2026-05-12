use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--manifest-path", concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"), "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_coverage_matrix_hard_fails() {
    let output = xtask()
        .args([
            "coverage-matrix",
            "--config",
            "xtask/tests/fixtures/violation-coverage-matrix/coverage-matrix.yaml",
            "--phase-config",
            "xtask/tests/fixtures/violation-coverage-matrix/phase-config.toml",
            "--manifest",
            "xtask/tests/fixtures/violation-coverage-matrix/MANIFEST.toml",
            "--gate-registry",
            "xtask/tests/fixtures/violation-coverage-matrix/gate-registry.toml",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure for hard mode, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("NFR-Meta-3 violation:"),
        "expected NFR-Meta-3 violation in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Uncovered"),
        "expected uncovered id in stderr, got:\n{stderr}"
    );
}

#[test]
fn violation_coverage_matrix_warning_passes() {
    // Warning mode: violation is emitted to stderr but exit code is zero.
    let output = xtask()
        .args([
            "coverage-matrix",
            "--config",
            "xtask/tests/fixtures/violation-coverage-matrix-warning/coverage-matrix.yaml",
            "--phase-config",
            "xtask/tests/fixtures/violation-coverage-matrix-warning/phase-config.toml",
            "--manifest",
            "xtask/tests/fixtures/violation-coverage-matrix-warning/MANIFEST.toml",
            "--gate-registry",
            "xtask/tests/fixtures/violation-coverage-matrix-warning/gate-registry.toml",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success for warning mode, got failure. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("NFR-Meta-3 violation:"),
        "expected NFR-Meta-3 violation in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_coverage_matrix_passes() {
    let output = xtask()
        .args([
            "coverage-matrix",
            "--config",
            "xtask/tests/fixtures/clean-coverage-matrix/coverage-matrix.yaml",
            "--phase-config",
            "xtask/tests/fixtures/clean-coverage-matrix/phase-config.toml",
            "--manifest",
            "xtask/tests/fixtures/clean-coverage-matrix/MANIFEST.toml",
            "--gate-registry",
            "xtask/tests/fixtures/clean-coverage-matrix/gate-registry.toml",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success, got failure. stderr:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASSED"),
        "expected PASSED in stdout, got:\n{stdout}"
    );
}
