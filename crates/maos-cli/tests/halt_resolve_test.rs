#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl halt resolve` (Story 3.3, AC6).
//!
//! Verifies the three resolution kinds exit cleanly, the required-arg
//! validation rejects provided-context without --text, NO_COLOR suppresses
//! ANSI, and unknown spirits are rejected.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maosctl");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maosctl")
}

fn maos_bin_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maos") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maos");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maos")
}

fn run_maosctl(extra_env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    let workspace_root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    cmd.current_dir(&workspace_root);
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.args(args);
    let out = cmd.output().expect("spawn maosctl");
    drop(tmp);
    out
}

#[test]
fn halt_resolve_accepted_halt_exits_zero() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-001",
            "--spirit",
            "hello-spirit",
            "--kind",
            "accepted-halt",
        ],
    );
    assert!(
        out.status.success(),
        "accepted-halt should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn halt_resolve_provided_context_exits_zero_with_reasoning() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-002",
            "--spirit",
            "hello-spirit",
            "--kind",
            "provided-context",
            "--text",
            "missing context for the halt",
        ],
    );
    assert!(
        out.status.success(),
        "provided-context with --text should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn halt_resolve_authorized_override_exits_zero() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-003",
            "--spirit",
            "hello-spirit",
            "--kind",
            "authorized-override",
            "--operator-policy",
            "policy://ops-override-001",
        ],
    );
    assert!(
        out.status.success(),
        "authorized-override with --operator-policy should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn halt_resolve_missing_text_rejected_by_clap() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-004",
            "--spirit",
            "hello-spirit",
            "--kind",
            "provided-context",
        ],
    );
    assert!(
        !out.status.success(),
        "provided-context without --text must be rejected; got {:?}",
        out.status.code(),
    );
}

#[test]
fn halt_resolve_no_color_emits_zero_ansi() {
    let out = run_maosctl(
        &[("NO_COLOR", "1")],
        &[
            "halt",
            "resolve",
            "halt-005",
            "--spirit",
            "hello-spirit",
            "--kind",
            "accepted-halt",
        ],
    );
    assert!(
        out.status.success(),
        "expected exit 0; got {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let esc_stdout = out.stdout.iter().filter(|b| **b == 0x1b).count();
    let esc_stderr = out.stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(
        esc_stdout, 0,
        "NO_COLOR=1: stdout contains {esc_stdout} ANSI escape byte(s)"
    );
    assert_eq!(
        esc_stderr, 0,
        "NO_COLOR=1: stderr contains {esc_stderr} ANSI escape byte(s)"
    );
}

#[test]
fn halt_resolve_unknown_spirit_exits_nonzero() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-006",
            "--spirit",
            "unknown-spirit",
            "--kind",
            "accepted-halt",
        ],
    );
    assert!(
        !out.status.success(),
        "unknown spirit must be rejected; got {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}
