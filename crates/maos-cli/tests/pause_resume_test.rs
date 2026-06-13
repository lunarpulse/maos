#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl pause/resume` (Story 3.4, AC3).
//!
//! Verifies pause/resume exit cleanly, unknown spirits are rejected,
//! NO_COLOR suppresses ANSI, and the Lifecycle Journal gets Pause+Resume.

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
fn pause_hello_spirit_exits_zero() {
    let out = run_maosctl(&[], &["pause", "hello-spirit"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn resume_hello_spirit_exits_zero() {
    let out = run_maosctl(&[], &["resume", "hello-spirit"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn pause_rejects_unknown_spirit() {
    let out = run_maosctl(&[], &["pause", "unknown-spirit"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only 'hello-spirit'"),
        "expected rejection, got: {stderr}"
    );
}

#[test]
fn resume_rejects_unknown_spirit() {
    let out = run_maosctl(&[], &["resume", "unknown-spirit"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only 'hello-spirit'"),
        "expected rejection, got: {stderr}"
    );
}

#[test]
fn pause_no_color_zero_ansi() {
    let out = run_maosctl(&[("NO_COLOR", "1")], &["pause", "hello-spirit"]);
    let stderr = out.stderr;
    let esc_count = stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(esc_count, 0, "NO_COLOR stderr contained ANSI escapes");
}

#[test]
fn lifecycle_journal_has_pause_and_resume() {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let journal_path = tmp.path().join("journal.ndjson");
    let xdg = tmp.path().join("xdg");
    std::fs::create_dir_all(&xdg).expect("xdg mkdir");

    let workspace_root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));

    // Pause
    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root);
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    cmd.args(["pause", "hello-spirit"]);
    let out1 = cmd.output().expect("spawn pause");
    assert!(
        out1.status.success(),
        "pause failed: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    // Resume
    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    if let Ok(p) = std::env::var("PATH") {
        cmd.env("PATH", p);
    }
    cmd.current_dir(&workspace_root);
    cmd.env("MAOS_AUDIT_DB", &db_path);
    cmd.env("MAOS_JOURNAL_PATH", &journal_path);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.env("MAOS_BIN_PATH", maos_bin_path());
    cmd.args(["resume", "hello-spirit"]);
    let out2 = cmd.output().expect("spawn resume");
    assert!(
        out2.status.success(),
        "resume failed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    // Verify Lifecycle Journal has Pause and Resume
    let journal_content = std::fs::read_to_string(&journal_path).expect("read journal");
    assert!(
        journal_content.contains("\"Pause\""),
        "Journal missing Pause: {journal_content}"
    );
    assert!(
        journal_content.contains("\"Resume\""),
        "Journal missing Resume: {journal_content}"
    );

    drop(tmp);
}
