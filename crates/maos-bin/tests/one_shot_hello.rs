#![forbid(unsafe_code)]

//! Integration test: one-shot hello-Spirit mode via MAOS_ONE_SHOT env var.
//!
//! Starts `maos-bin` with MAOS_ONE_SHOT=hello-spirit and asserts valid JSON on stdout.

use std::process::Command;
use std::time::{Duration, Instant};

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;

#[test]
fn one_shot_hello_spirit_produces_valid_json() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let start = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .env("MAOS_OLLAMA_URL", "skip")
        // Story 16-1 / D-16-1-Q: empty scratch "HOME" — the floor is file-granular,
        // and a one-shot that later becomes a root must not regain the hazard.
        .env("HOME", doorless_home::doorless_home())
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

/// S1 review P2 — the one-shot reads `spirits/hello-spirit/manifest.toml` from its
/// CWD and admits it through `SecurityManagerAdapter::admit_spirit` directly, not
/// through `load_admit_start`: a retired model pin must be refused there too
/// (`model_not_admitted`), under explicit replay, before the Spirit runs. An
/// admitted pin (Sonnet 5.5) is not refused for its model.
///
/// Proven red: delete the one-shot's `check_gated_pins` call ⇒ the retired pin
/// is admitted and the refusal assertion fails.
fn one_shot_with_hello_pin(model: &str) -> std::process::Output {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let source = std::fs::read_to_string(root.join("spirits/hello-spirit/manifest.toml"))
        .expect("read the hello manifest");
    let pinned = source.replace("anthropic.claude-haiku-5-5", &format!("anthropic.{model}"));
    assert_ne!(pinned, source, "the fixture must really change the pin");
    let world = tempfile::tempdir().expect("scratch world");
    let cwd = world.path().join("cwd");
    std::fs::create_dir_all(cwd.join("spirits/hello-spirit")).expect("cwd");
    std::fs::write(cwd.join("spirits/hello-spirit/manifest.toml"), pinned).expect("manifest");
    Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", world.path())
        .env("MAOS_HOME", world.path().join("maos"))
        .env("XDG_DATA_HOME", world.path().join("xdg"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root.join("crates/maos-journey-test/cassettes/j0/shell-intro.json"),
        )
        .current_dir(&cwd)
        .output()
        .expect("failed to execute maos-bin")
}

#[test]
fn one_shot_hello_refuses_a_retired_model_pin_under_explicit_replay() {
    let refused = one_shot_with_hello_pin("claude-3-haiku-20240307");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        !refused.status.success()
            && stderr.contains("model_not_admitted")
            && stderr.contains("claude-3-haiku-20240307"),
        "a retired pin must refuse the one-shot admission: {} {stderr}",
        refused.status
    );
    assert!(
        !String::from_utf8_lossy(&refused.stdout).contains("introduction"),
        "the Spirit must not have run"
    );

    let admitted = one_shot_with_hello_pin("claude-sonnet-5-5");
    assert!(
        !String::from_utf8_lossy(&admitted.stderr).contains("model_not_admitted"),
        "an allowed pin is not refused for its model: {}",
        String::from_utf8_lossy(&admitted.stderr)
    );
}
