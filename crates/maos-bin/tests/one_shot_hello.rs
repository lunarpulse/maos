#![forbid(unsafe_code)]

//! Integration test: one-shot hello-Spirit mode via MAOS_ONE_SHOT env var.
//!
//! Starts `maos-bin` with MAOS_ONE_SHOT=hello-spirit and asserts valid JSON on stdout.

use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn one_shot_hello_spirit_produces_valid_json() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let start = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .env("MAOS_OLLAMA_URL", "skip")
        .current_dir(workspace_root)
        .output()
        .expect("failed to execute maos-bin");
    let elapsed = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The hello-spirit one-shot should complete within 10 seconds
    // (fast path uses UnconfiguredProvider when no API key is set).
    if elapsed > Duration::from_secs(10) {
        // If it takes longer, something is stuck (e.g., Ollama connection)
        eprintln!("WARNING: one-shot took {elapsed:?}, possible connection hang");
    }

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
