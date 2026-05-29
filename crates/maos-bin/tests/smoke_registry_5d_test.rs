//! Smoke test for `MAOS_ONE_SHOT=smoke-registry-5d`.
//!
//! Mirrors `smoke_mcp_acp_test.rs` shape exactly (process-based integration test).
//! Requires `--features fixture_replay` at compile time.

#![cfg(feature = "fixture_replay")]

use std::process::Command;

#[test]
fn smoke_registry_5d_exits_zero_and_outputs_7_json_lines() {
    let bin = env!("CARGO_BIN_EXE_maos-bin");
    let output = Command::new(bin)
        .env("MAOS_ONE_SHOT", "smoke-registry-5d")
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Assert exit code 0
    assert!(
        output.status.success(),
        "smoke-registry-5d exited with non-zero status: {}\nstderr: {}",
        output.status,
        stderr,
    );

    // Collect JSON lines (non-empty, non-comment)
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert!(
        lines.len() >= 7,
        "expected at least 7 JSON lines, got {}: {:?}",
        lines.len(),
        lines,
    );

    // Verify each line is valid JSON with expected keys
    for (i, line) in lines.iter().enumerate() {
        let val: serde_json::Value =
            serde_json::from_str(line).expect(&format!("line {} is not valid JSON: {}", i + 1, line));
        assert!(
            val.get("step").is_some(),
            "line {} missing 'step' key: {}",
            i + 1,
            line
        );
        assert!(
            val.get("surface").is_some(),
            "line {} missing 'surface' key: {}",
            i + 1,
            line
        );
    }

    // Specific assertions
    let step1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(step1["step"], 1);
    assert_eq!(step1["surface"], "registry_init");

    let step7: serde_json::Value = serde_json::from_str(lines[6]).unwrap();
    assert_eq!(step7["step"], 7);
    assert_eq!(step7["surface"], "registry_yank_propagate");
    assert_eq!(step7["outcome"], "ok");
}
