use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--manifest-path", concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"), "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_staleness_fails() {
    let output = xtask()
        .args([
            "corpus-staleness",
            "--config",
            "xtask/tests/fixtures/violation-staleness/coverage-matrix.yaml",
            "--manifest",
            "xtask/tests/fixtures/violation-staleness/MANIFEST.toml",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("NFR-Meta-2 violation:"),
        "expected NFR-Meta-2 violation in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("2020-01-01"),
        "expected expired date in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_staleness_passes() {
    let output = xtask()
        .args([
            "corpus-staleness",
            "--config",
            "xtask/tests/fixtures/clean-staleness/coverage-matrix.yaml",
            "--manifest",
            "xtask/tests/fixtures/clean-staleness/MANIFEST.toml",
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
