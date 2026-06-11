#![forbid(unsafe_code)]

//! J1 — Founder-loop journey (Grade B: orchestrated smoke wrap).
//!
//! The founder-class spirits cannot be loaded via `maos run <spirit>` (the 8.12
//! founder-class gap — `FounderLoopClass` short-circuits with a directional
//! error). J1 wraps the existing `MAOS_ONE_SHOT=smoke-founder-loop-8-4` arm
//! with receiver-side oracles.

use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        return bin.to_string();
    }
    let debug_bin = workspace_root().join("target/debug/maos");
    if debug_bin.exists() {
        return debug_bin.to_string_lossy().into_owned();
    }
    panic!("maos binary not found — run `cargo build -p maos-bin` first");
}

#[test]
fn j1_founder_loop_smoke_wrap() {
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .env("MAOS_ONE_SHOT", "smoke-founder-loop-8-4")
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn founder-loop smoke");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "founder-loop smoke should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );
}

#[test]
fn j1_founder_class_tripwire() {
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/founder/orchestrator/manifest.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn maos run for founder-class");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "maos run of a founder-class spirit should fail (8.12 gap); stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("FounderLoopClass") || stderr.contains("founder"),
        "error should mention founder-class gap; stderr:\n{stderr}"
    );
}
