#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl orchestrator queue/status` (Story 3.4, AC2).
//!
//! Verifies enqueue exits cleanly, Approval Decision Log rows are written,
//! unknown spirits are rejected, empty instructions are rejected, and NO_COLOR
//! suppresses ANSI.

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
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maos-bin") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maos-bin");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maos-bin")
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

fn run_queue_and_inspect_db(args: &[&str]) -> (std::process::Output, rusqlite::Connection) {
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
    cmd.args(args);
    let out = cmd.output().expect("spawn maosctl");

    let conn =
        rusqlite::Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open db for inspection");

    std::mem::forget(tmp);
    (out, conn)
}

#[test]
fn orchestrator_queue_exits_zero() {
    let out = run_maosctl(
        &[],
        &[
            "orchestrator",
            "queue",
            "--spirit",
            "hello-spirit",
            "draft the PR",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn orchestrator_queue_rejects_unknown_spirit() {
    let out = run_maosctl(
        &[],
        &["orchestrator", "queue", "--spirit", "unknown-spirit", "x"],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only 'hello-spirit'"),
        "expected spirit-rejection diagnostic, got: {stderr}"
    );
}

#[test]
fn orchestrator_queue_rejects_empty_instruction() {
    let out = run_maosctl(
        &[],
        &["orchestrator", "queue", "--spirit", "hello-spirit", ""],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("must be non-empty"),
        "expected empty-instruction diagnostic, got: {stderr}"
    );
}

#[test]
fn orchestrator_status_exits_zero() {
    let out = run_maosctl(&[], &["orchestrator", "status", "--spirit", "hello-spirit"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn orchestrator_status_no_color_zero_ansi() {
    let out = run_maosctl(
        &[("NO_COLOR", "1")],
        &["orchestrator", "status", "--spirit", "hello-spirit"],
    );
    let stderr = out.stderr;
    let esc_count = stderr.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(esc_count, 0, "NO_COLOR stderr contained ANSI escapes");
}

#[test]
fn orchestrator_queue_writes_adl_row_with_correct_content() {
    let (out, conn) = run_queue_and_inspect_db(&[
        "orchestrator",
        "queue",
        "--spirit",
        "hello-spirit",
        "draft the PR",
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut stmt = conn.prepare(
        "SELECT capability, intent, reasoning FROM approval_decision_log ORDER BY id DESC LIMIT 1"
    ).unwrap();
    let row: (String, String, Option<String>) = stmt
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap();

    assert_eq!(row.0, "orchestrator.queue", "capability mismatch");
    assert_eq!(row.1, "queue", "intent mismatch");
    assert!(
        row.2.as_ref().map_or(false, |r| r.contains("draft the PR")),
        "reasoning should contain goal text, got: {:?}",
        row.2
    );
}

#[test]
fn hex_validation_rejects_uppercase() {
    let out = run_maosctl(&[], &["revoke-token", "AABBCCDDEEFF00112233445566778899"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lowercase hex"),
        "expected lowercase rejection, got: {stderr}"
    );
}
