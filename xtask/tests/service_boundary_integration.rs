use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

#[test]
fn violation_service_boundary_fails() {
    // Create a temporary baseline that matches the clean fixture.
    let tmpdir = std::env::temp_dir().join("maos-sb-test-baseline");
    let baseline_path = tmpdir.join("baseline.json");
    std::fs::create_dir_all(&tmpdir).unwrap();

    // First, snapshot the clean fixture to establish a baseline.
    let clean_output = xtask()
        .args([
            "check-service-boundary",
            "--path",
            "xtask/tests/fixtures/clean-service-boundary",
            "--baseline",
            "/dev/null",
            "--classes",
            "xtask/kernel-api-classes.toml",
            "--json",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("xtask should run");

    let clean_stdout = String::from_utf8_lossy(&clean_output.stdout);
    let clean_report: serde_json::Value = serde_json::from_str(&clean_stdout)
        .expect("clean fixture should produce valid JSON");
    let baseline_surface = &clean_report["current_surface"];
    std::fs::write(&baseline_path, serde_json::to_string_pretty(baseline_surface).unwrap())
        .unwrap();

    // Now run against the violation fixture with the clean baseline.
    let output = xtask()
        .args([
            "check-service-boundary",
            "--path",
            "xtask/tests/fixtures/violation-service-boundary",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--classes",
            "xtask/kernel-api-classes.toml",
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
        stderr.contains("NFR-Test-2 violation:"),
        "expected NFR-Test-2 violation in stderr, got:\n{stderr}"
    );

    // Clean up temporary baseline directory.
    let _ = std::fs::remove_dir_all(&tmpdir);
}

#[test]
fn clean_service_boundary_passes() {
    let output = xtask()
        .args([
            "check-service-boundary",
            "--path",
            "xtask/tests/fixtures/clean-service-boundary",
            "--baseline",
            "/dev/null",
            "--classes",
            "xtask/kernel-api-classes.toml",
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
