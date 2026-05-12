use std::process::Command;

fn fixture(path: &str) -> String {
    format!("xtask/tests/fixtures/{path}")
}

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

#[test]
fn clean_calibration_corpus_smoke() {
    // Smoke test: parse MANIFEST.toml, assert exactly one [corpus.*] row exists,
    // assert item_count == 10 (fixture size), assert valid_until == "2027-05-12".
    let manifest_src = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/clean-calibration/MANIFEST.toml")
    ).expect("read clean-calibration MANIFEST.toml");
    let manifest: toml::Value = toml::from_str(&manifest_src).expect("parse TOML");
    let corpus_table = manifest.get("corpus").expect("corpus key");
    let entries = corpus_table.as_table().expect("corpus is a table");
    assert_eq!(entries.len(), 1, "exactly one corpus entry");
    let entry = entries.values().next().unwrap();
    assert_eq!(entry.get("item_count").unwrap().as_integer().unwrap(), 10);
    assert_eq!(entry.get("valid_until").unwrap().as_str().unwrap(), "2027-05-12");
}
