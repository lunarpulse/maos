#![forbid(unsafe_code)]

//! Story 20-3c AC4 (member 11) — a `MAOS_ONE_SHOT` arm closes EVERY `audit_tx`
//! holder before it awaits the audit writer, so the drain completes instead of
//! waiting out its window and the arm exiting 0 on a log line.
//!
//! Measured at `3341f9e4` on the prebuilt binary: each of these five arms
//! printed `audit writer drain timed out after 5s`, exited 0, and took
//! 5.2–6.3 s. The failing-before witness is the `drain timed out` assertion and
//! the wall bound; a leaked sender now fails the arm with a typed non-zero exit.

use std::process::Command;
use std::time::{Duration, Instant};

fn run_arm(mode: &str) {
    let home = tempfile::TempDir::new().expect("isolated MAOS home");
    let root = home.path();
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("MAOS_HOME", root.join("maos"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_ONE_SHOT", mode)
        .current_dir(workspace)
        .output()
        .unwrap_or_else(|error| panic!("spawn maos MAOS_ONE_SHOT={mode}: {error}"));
    let elapsed = started.elapsed();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("drain timed out"),
        "{mode}: the audit writer must drain, not time out; stderr:\n{stderr}"
    );
    assert!(
        output.status.success(),
        "{mode}: exit {:?}; stderr:\n{stderr}",
        output.status.code()
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "{mode}: a completed drain returns well inside the 5 s window, took {elapsed:?}"
    );
}

#[test]
fn smoke_epic_4_drains_its_audit_writer() {
    run_arm("smoke-epic-4");
}

#[test]
fn smoke_spirit_5_drains_its_audit_writer() {
    run_arm("smoke-spirit-5");
}

#[test]
fn smoke_supervision_5_drains_its_audit_writer() {
    run_arm("smoke-supervision-5");
}

#[test]
fn smoke_upgrade_revoke_5_drains_its_audit_writer() {
    run_arm("smoke-upgrade-revoke-5");
}

#[test]
fn hello_spirit_drains_its_audit_writer() {
    run_arm("hello-spirit");
}
