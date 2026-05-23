use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn clean_report_returns_zero_exit() {
    let output = xtask()
        .args([
            "check-multi-provider-drift",
            "--report",
            "xtask/tests/fixtures/multi-provider-reports/clean.json",
            "--json",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");
    assert!(
        output.status.success(),
        "clean report should exit 0\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn with_outlier_report_returns_zero_exit_annotation_mode() {
    let output = xtask()
        .args([
            "check-multi-provider-drift",
            "--report",
            "xtask/tests/fixtures/multi-provider-reports/with-outlier.json",
            "--json",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");
    assert!(
        output.status.success(),
        "annotation mode should exit 0 on outliers\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let outliers = parsed["outliers"].as_array().unwrap();
    assert!(!outliers.is_empty(), "should have flagged outliers");
}

#[test]
fn with_outlier_strict_mode_returns_nonzero_exit() {
    let output = xtask()
        .args([
            "check-multi-provider-drift",
            "--report",
            "xtask/tests/fixtures/multi-provider-reports/with-outlier.json",
            "--strict",
            "--json",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");
    assert!(
        !output.status.success(),
        "strict mode with outliers should exit non-zero"
    );
}
