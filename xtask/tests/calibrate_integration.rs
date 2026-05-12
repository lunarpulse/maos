use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--manifest-path", concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"), "-p", "xtask", "--"]);
    cmd
}

fn fixture(path: &str) -> String {
    format!("xtask/tests/fixtures/{path}")
}

#[test]
fn calibrate_passes_on_clean_corpus() {
    let output = xtask()
        .args([
            "calibrate",
            "--corpus", "calibration-seed-v0.1",
            "--n", "10",
            "--p", "0.95",
            "--manifest", &fixture("clean-calibration/MANIFEST.toml"),
            "--corpora-dir", &fixture("clean-calibration/corpora"),
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
fn calibrate_passes_on_mismatch_but_within_ci() {
    // 100 items, 30 mismatched → pass_rate=0.70, ci_width≈0.18 < 0.20 → PASSED
    let output = xtask()
        .args([
            "calibrate",
            "--corpus", "calibration-seed-v0.1-mismatched",
            "--n", "100",
            "--p", "0.95",
            "--manifest", &fixture("violation-calibration-mismatch/MANIFEST.toml"),
            "--corpora-dir", &fixture("violation-calibration-mismatch/corpora"),
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success (pass_rate=0.70 gives ci_width≈0.18 < 0.20), got failure. stderr:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PASSED"),
        "expected PASSED, got:\n{stdout}"
    );
}

#[test]
fn calibrate_fails_on_mismatch_at_p_0_99() {
    // At p=0.99 (z=2.5758), pass_rate=0.70 produces ci_width > 0.20,
    // so the gate should exit NON-ZERO with an NFR-Aud-8 violation message.
    let output = xtask()
        .args([
            "calibrate",
            "--corpus", "calibration-seed-v0.1-mismatched",
            "--n", "100",
            "--p", "0.99",
            "--manifest", &fixture("violation-calibration-mismatch/MANIFEST.toml"),
            "--corpora-dir", &fixture("violation-calibration-mismatch/corpora"),
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure at p=0.99 with pass_rate=0.70, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("NFR-Aud-8 violation"),
        "expected 'NFR-Aud-8 violation' in stderr, got:\n{stderr}"
    );
}

#[test]
fn calibrate_surfaces_malformed_items() {
    let output = xtask()
        .args([
            "calibrate",
            "--corpus", "calibration-seed-v0.1-malformed",
            "--n", "10",
            "--p", "0.95",
            "--manifest", &fixture("violation-calibration-malformed/MANIFEST.toml"),
            "--corpora-dir", &fixture("violation-calibration-malformed/corpora"),
            "--json",
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
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON report");
    assert_eq!(report["malformed_items"], serde_json::json!(5));
    assert_eq!(report["n"], serde_json::json!(10));
}
