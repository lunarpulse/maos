#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl revoke-token` (Story 3.4, AC4).
//!
//! Verifies invalid hex rejection, unknown token rejection, NO_COLOR.
//!
//! Note at v0.3-β: capability tokens are per-process in-memory (CapTokensShardRing
//! is NOT persisted). A valid-token revoke across separate processes is not
//! possible until Story 5.4 adds persistent token storage. The surface validation
//! and not-found path are the v0.3-β contract.

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

fn workspace_root() -> PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn revoke_token_rejects_invalid_hex_short() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root());
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    cmd.args(["revoke-token", "abc"]);
    let out = cmd.output().expect("spawn maosctl");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid token_id"),
        "expected hex-rejection diagnostic, got: {stderr}"
    );
    drop(tmp);
}

#[test]
fn revoke_token_rejects_invalid_hex_nonhex() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root());
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    cmd.args(["revoke-token", "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"]);
    let out = cmd.output().expect("spawn maosctl");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid token_id"),
        "expected hex-rejection diagnostic, got: {stderr}"
    );
    drop(tmp);
}

#[test]
fn revoke_token_unknown_token_exits_nonzero() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root());
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    // 32 zeros — no such token ever issued
    cmd.args(["revoke-token", "00000000000000000000000000000000"]);
    let out = cmd.output().expect("spawn maosctl");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not found"),
        "expected not-found diagnostic, got: {stderr}"
    );
    drop(tmp);
}

#[test]
fn revoke_token_no_color_zero_ansi() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root());
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    cmd.env("NO_COLOR", "1");
    cmd.args(["revoke-token", "00000000000000000000000000000000"]);
    let out = cmd.output().expect("spawn maosctl");

    let stderr = out.stderr;
    let esc_count = stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(esc_count, 0, "NO_COLOR stderr contained ANSI escapes");
    drop(tmp);
}
