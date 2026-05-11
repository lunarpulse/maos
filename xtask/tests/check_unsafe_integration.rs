use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn with_unsafe_fails() {
    let output = xtask()
        .args([
            "check-unsafe",
            "--path",
            "xtask/tests/fixtures/with-unsafe/capability",
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
        stderr.contains("NFR-Sec-9 violation"),
        "expected NFR-Sec-9 violation in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("unsafe fn"),
        "expected 'unsafe fn' in stderr, got:\n{stderr}"
    );
}

#[test]
fn without_unsafe_passes() {
    let output = xtask()
        .args([
            "check-unsafe",
            "--path",
            "xtask/tests/fixtures/without-unsafe/capability",
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
