#![forbid(unsafe_code)]

//! AC3 — Accessibility cascade (Story 1b.5b).
//!
//! Spawns the `maosctl` binary against a hermetic on-disk SQLite seed
//! (seeded with three rows mirroring the kernel-side schema) and asserts
//! that for each of the four combinations
//!
//!   {`TERM=dumb`, `NO_COLOR=1`} × {`--format ndjson`, `--format plain`}
//!
//! the captured stdout contains exactly zero `0x1b` (ESC) bytes — i.e.,
//! no ANSI escape sequences. Per `Cargo`-test convention we resolve the
//! `maosctl` binary path via the `CARGO_BIN_EXE_maosctl` env var that
//! cargo injects for each `[[bin]]`.
//!
//! Per Decision D1, this test lives in `crates/maos-cli/tests/` (the
//! dispatcher lives in maos-cli — NOT `maos-bin/tests/`).

use std::path::PathBuf;
use std::process::Command;

use rusqlite::Connection;
use tempfile::TempDir;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY,
    timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL,
    boot_nonce INTEGER NOT NULL,
    capability_token BLOB,
    kind INTEGER NOT NULL,
    intent TEXT NOT NULL,
    payload_redacted BLOB NOT NULL,
    origin INTEGER NOT NULL
);
";

/// Seed a SQLite DB at `path` with three well-formed mediated rows
/// (spirit_pid = 0 matches `--spirit hello-spirit`).
fn seed_db(path: &PathBuf) {
    let conn = Connection::open(path).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");
    for (i, kind) in [(1u8, 9i64), (2u8, 7i64), (3u8, 9i64)]
        .into_iter()
        .enumerate()
    {
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[kind.0; 16] as &[u8],
                (1_000i64 + i as i64),
                0i64, // spirit_pid = 0 → hello-spirit
                0xDEAD_BEEFi64,
                &[0xAAu8; 32] as &[u8],
                kind.1,
                "claude-3-haiku",
                b"<redacted>" as &[u8],
                0i64,
            ],
        )
        .expect("insert seed row");
    }
}

/// Resolve maosctl: prefer `CARGO_BIN_EXE_maosctl` (cargo-injected), then
/// fall back to a sibling `maosctl` next to the test binary, then PATH.
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

fn run_maosctl(db_path: &PathBuf, env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(maosctl_path());
    cmd.env_clear();
    // Preserve PATH so the binary can find its dynamic loader.
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    cmd.env("MAOS_AUDIT_DB", db_path);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.args(args);
    cmd.output().expect("spawn maosctl")
}

fn assert_no_ansi(stdout: &[u8], scenario: &str) {
    let esc = stdout.iter().filter(|b| **b == 0x1b).count();
    assert_eq!(
        esc, 0,
        "{scenario}: stdout contains {esc} ANSI escape byte(s) — NFR-Ops-5 violation"
    );
}

#[test]
fn ndjson_with_term_dumb_emits_zero_ansi_bytes() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("seed.sqlite");
    seed_db(&db);
    let out = run_maosctl(
        &db,
        &[("TERM", "dumb")],
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        out.status.success(),
        "TERM=dumb ndjson exit: {:?}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi(&out.stdout, "TERM=dumb --format ndjson");
}

#[test]
fn ndjson_with_no_color_emits_zero_ansi_bytes() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("seed.sqlite");
    seed_db(&db);
    let out = run_maosctl(
        &db,
        &[("NO_COLOR", "1")],
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        out.status.success(),
        "NO_COLOR=1 ndjson exit: {:?}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi(&out.stdout, "NO_COLOR=1 --format ndjson");
}

#[test]
fn plain_with_term_dumb_emits_zero_ansi_bytes() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("seed.sqlite");
    seed_db(&db);
    let out = run_maosctl(
        &db,
        &[("TERM", "dumb")],
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "plain",
        ],
    );
    assert!(
        out.status.success(),
        "TERM=dumb plain exit: {:?}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi(&out.stdout, "TERM=dumb --format plain");
}

#[test]
fn plain_with_no_color_emits_zero_ansi_bytes() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("seed.sqlite");
    seed_db(&db);
    let out = run_maosctl(
        &db,
        &[("NO_COLOR", "1")],
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "plain",
        ],
    );
    assert!(
        out.status.success(),
        "NO_COLOR=1 plain exit: {:?}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_no_ansi(&out.stdout, "NO_COLOR=1 --format plain");
}

#[test]
fn fr4_schema_violation_exits_two_with_diagnostic() {
    // Seed a row with NULL capability_token, then query — exit code must be 2
    // and stderr must name the missing field.
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("violator.sqlite");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(SCHEMA_SQL).unwrap();
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x33u8; 16] as &[u8],
            1_000i64,
            0i64,
            0xDEAD_BEEFi64,
            rusqlite::types::Null,
            9i64,
            "claude-3-haiku",
            b"<redacted>" as &[u8],
            0i64,
        ],
    )
    .unwrap();
    drop(conn);

    let out = run_maosctl(
        &db,
        &[("NO_COLOR", "1")],
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        !out.status.success(),
        "violation row must produce non-zero exit; got {:?}",
        out.status
    );
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(
        code, 2,
        "exit code must be 2 on FR4 schema violation, got {code}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("FR4 schema violation"),
        "stderr must surface the diagnostic; got: {stderr}"
    );
    assert!(
        stderr.contains("capability_token"),
        "stderr must name the missing field; got: {stderr}"
    );
}

#[test]
fn unknown_spirit_exits_two_with_clear_diagnostic() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("seed.sqlite");
    seed_db(&db);
    let out = run_maosctl(
        &db,
        &[("NO_COLOR", "1")],
        &[
            "audit",
            "query",
            "--spirit",
            "orchestrator",
            "--format",
            "ndjson",
        ],
    );
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("only 'hello-spirit'"),
        "diagnostic must name the v0.1-β scope; got: {stderr}"
    );
}
