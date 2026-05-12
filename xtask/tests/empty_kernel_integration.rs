use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_i9_fails() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/violation-i9",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
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
        stderr.contains("I9 violation: persistent struct"),
        "expected I9 violation in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("HungryCache"),
        "expected 'HungryCache' in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("violation-i9"),
        "expected fixture path in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_i9_passes() {
    let output = xtask()
        .args([
            "check-empty-kernel",
            "--path",
            "xtask/tests/fixtures/clean-i9",
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
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
