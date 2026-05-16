#![forbid(unsafe_code)]

//! Integration test: one-shot hello-Spirit mode via MAOS_ONE_SHOT env var.
//!
//! Starts `maos-bin` with MAOS_ONE_SHOT=hello-spirit and asserts valid JSON on stdout.

use std::process::Command;

#[test]
fn one_shot_hello_spirit_produces_valid_json() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .current_dir(workspace_root)
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should exit successfully
    assert!(
        output.status.success(),
        "maos-bin one-shot failed with status {}.\nstderr: {stderr}",
        output.status
    );

    // Should produce valid JSON on stdout
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert!(
        parsed.get("introduction").is_some(),
        "JSON output missing 'introduction' key: {stdout}"
    );
    assert!(
        parsed.get("capability_scope").is_some(),
        "JSON output missing 'capability_scope' key: {stdout}"
    );
    assert!(
        parsed.get("halt_tags").is_some(),
        "JSON output missing 'halt_tags' key: {stdout}"
    );
    assert!(
        parsed.get("transparency_log").is_some(),
        "JSON output missing 'transparency_log' key: {stdout}"
    );

    // No ANSI escape codes in JSON output (NFR-Ops-5)
    assert!(
        !stdout.contains("\x1b["),
        "JSON output contains ANSI escape codes: {stdout}"
    );
}
