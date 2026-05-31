#![forbid(unsafe_code)]
#![cfg(feature = "fixture_replay")]

//! Integration test: MAOS_ONE_SHOT=smoke-multi-provider-5 smoke arm.

use std::process::Command;

#[test]
fn smoke_multi_provider_5_exercises_6_surfaces() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .env("MAOS_ONE_SHOT", "smoke-multi-provider-5")
        .env("MAOS_SPIRIT_ABI_MOCK", "1")
        .current_dir(workspace_root)
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "maos-bin smoke-multi-provider-5 failed with status {}.\nstderr: {stderr}",
        output.status
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 6,
        "expected >= 6 JSON lines, got {}: {stdout}",
        lines.len()
    );

    let steps: Vec<serde_json::Value> = lines
        .iter()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    assert!(
        steps.len() >= 6,
        "expected >= 6 parseable JSON steps, got {}",
        steps.len()
    );

    for i in 1..=6u64 {
        let found = steps
            .iter()
            .any(|s| s.get("step").and_then(|v| v.as_u64()) == Some(i));
        assert!(found, "missing step {i} in output: {stdout}");
    }

    assert!(
        stderr.contains("smoke-multi-provider-5 complete"),
        "expected completion message in stderr: {stderr}"
    );
}
