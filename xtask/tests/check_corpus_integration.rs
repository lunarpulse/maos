use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--manifest-path", concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"), "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_corpus_fails() {
    let output = xtask()
        .args([
            "check-corpus",
            "--manifest",
            "xtask/tests/fixtures/violation-corpus/MANIFEST.toml",
            "--corpora-dir",
            "xtask/tests/fixtures/violation-corpus/corpora",
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
        stderr.contains("NFR-Test-1 violation: corpus integrity broken"),
        "expected 'corpus integrity broken' in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("bad"),
        "expected corpus name 'bad' in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_corpus_passes() {
    let output = xtask()
        .args([
            "check-corpus",
            "--manifest",
            "xtask/tests/fixtures/clean-corpus/MANIFEST.toml",
            "--corpora-dir",
            "xtask/tests/fixtures/clean-corpus/corpora",
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
