use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_loom_fails() {
    let output = xtask()
        .args([
            "check-loom",
            "--path",
            "xtask/tests/fixtures/violation-loom",
            "--blocklist",
            "xtask/loom-blocklist.toml",
            "--allowlist",
            "xtask/loom-allowlist.toml",
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
        stderr.contains("NFR-Test-9 violation: Loom-not-in-kernel grep matched 'Planner'"),
        "expected NFR-Test-9 violation for Planner in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_loom_passes() {
    let output = xtask()
        .args([
            "check-loom",
            "--path",
            "xtask/tests/fixtures/clean-loom",
            "--blocklist",
            "xtask/loom-blocklist.toml",
            "--allowlist",
            "xtask/loom-allowlist.toml",
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
