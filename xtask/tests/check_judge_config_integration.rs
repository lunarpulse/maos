use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--manifest-path",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"),
        "-p",
        "xtask",
        "--",
    ]);
    cmd
}

#[test]
fn violation_judge_config_fails() {
    let output = xtask()
        .args([
            "check-judge-config",
            "--config",
            "xtask/tests/fixtures/violation-judge-config/judge-config.toml",
            "--identifiers",
            "xtask/judge-direct-call-identifiers.toml",
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
        stderr.contains("NFR-Test-1 violation: judge"),
        "expected 'NFR-Test-1 violation: judge' in stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("temperature=0.5"),
        "expected temperature=0.5 in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_judge_config_passes() {
    let output = xtask()
        .args([
            "check-judge-config",
            "--config",
            "xtask/tests/fixtures/clean-judge-config/judge-config.toml",
            "--identifiers",
            "xtask/judge-direct-call-identifiers.toml",
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
fn violation_judge_direct_call_fails() {
    // Create a temp directory with a tests/ subdirectory so the AST scan
    // picks up the fixture file.
    let tmp = std::env::temp_dir().join(format!("maos-judge-direct-call-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::create_dir_all(tmp.join("tests")).unwrap();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/violation-judge-direct-call/some_test.rs"
        ),
        tmp.join("tests/some_test.rs"),
    )
    .unwrap();

    let output = xtask()
        .args([
            "check-judge-config",
            "--config",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/clean-judge-config/judge-config.toml"
            ),
            "--identifiers",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/judge-direct-call-identifiers.toml"
            ),
        ])
        .current_dir(&tmp)
        .output()
        .expect("xtask should run");

    let _ = std::fs::remove_dir_all(&tmp);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected failure, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("NFR-Test-1 violation: direct judge-LLM call"),
        "expected direct judge-LLM call violation in stderr, got:\n{stderr}"
    );
}

#[test]
fn clean_judge_direct_call_passes() {
    let tmp = std::env::temp_dir().join(format!(
        "maos-judge-direct-call-clean-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::create_dir_all(tmp.join("tests")).unwrap();
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/clean-judge-direct-call/some_test.rs"
        ),
        tmp.join("tests/some_test.rs"),
    )
    .unwrap();

    let output = xtask()
        .args([
            "check-judge-config",
            "--config",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/clean-judge-config/judge-config.toml"
            ),
            "--identifiers",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/judge-direct-call-identifiers.toml"
            ),
        ])
        .current_dir(&tmp)
        .output()
        .expect("xtask should run");

    let _ = std::fs::remove_dir_all(&tmp);

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
