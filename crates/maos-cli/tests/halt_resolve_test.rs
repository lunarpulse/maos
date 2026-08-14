#![forbid(unsafe_code)]

//! CLI integration tests for `maosctl halt` (Story 3.3, AC6).
//!
//! Each of the three resolution kinds exits 0 AND lands one Approval
//! Decision Log row whose `capability` / `intent` / `reasoning` columns match
//! the FR15 contract; the required-arg validation rejects provided-context
//! without `--text`; `NO_COLOR` suppresses ANSI on `halt list`; and an
//! unknown Spirit exits with code 2.

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

/// Spawn `maosctl` against a throwaway audit DB. The DB is discarded with the
/// tempdir — use [`run_maosctl_and_inspect_db`] when the row must be read back.
fn run_maosctl(extra_env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    let tmp = TempDir::new().expect("tempdir");
    let out = spawn_maosctl(tmp.path(), extra_env, args);
    drop(tmp);
    out
}

/// Spawn `maosctl` and hand back a read-only connection to the audit DB it
/// wrote, so the Approval Decision Log row can be asserted on. Mirrors
/// `orchestrator_queue_test.rs::run_queue_and_inspect_db` — same capture
/// pattern, same deliberate tempdir leak to keep the DB alive for the caller.
fn run_maosctl_and_inspect_db(
    extra_env: &[(&str, &str)],
    args: &[&str],
) -> (std::process::Output, rusqlite::Connection) {
    let tmp = TempDir::new().expect("tempdir");
    let db_path = tmp.path().join("transparency.sqlite");
    let out = spawn_maosctl(tmp.path(), extra_env, args);

    let conn =
        rusqlite::Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open audit db for inspection");

    std::mem::forget(tmp);
    (out, conn)
}

fn spawn_maosctl(
    tmp: &std::path::Path,
    extra_env: &[(&str, &str)],
    args: &[&str],
) -> std::process::Output {
    let db_path = tmp.join("transparency.sqlite");
    let journal_path = tmp.join("journal.ndjson");
    let xdg = tmp.join("xdg");
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
    cmd.output().expect("spawn maosctl")
}

/// The single Approval Decision Log row written by a `halt resolve` run.
fn only_halt_resolve_row(conn: &rusqlite::Connection) -> (String, String, Option<String>) {
    let mut stmt = conn
        .prepare(
            "SELECT capability, intent, reasoning FROM approval_decision_log \
             WHERE capability = 'halt.resolve'",
        )
        .expect("prepare ADL query");
    let rows: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("query ADL")
        .map(|r| r.expect("ADL row"))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "expected exactly one halt.resolve row, got {rows:?}"
    );
    rows.into_iter().next().expect("one row")
}

#[test]
fn halt_resolve_accepted_halt_writes_adl_row() {
    let (out, conn) = run_maosctl_and_inspect_db(
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

    let (capability, intent, reasoning) = only_halt_resolve_row(&conn);
    assert_eq!(capability, "halt.resolve");
    assert_eq!(intent, "accepted_halt");
    let reasoning = reasoning.expect("reasoning column must be populated");
    assert!(
        reasoning.contains("halt=halt-001") && reasoning.contains("accepted_halt"),
        "reasoning must carry halt_id + kind, got: {reasoning}"
    );
}

#[test]
fn halt_resolve_provided_context_writes_adl_row_with_supplied_text() {
    let (out, conn) = run_maosctl_and_inspect_db(
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
            "the issue is X",
        ],
    );
    assert!(
        out.status.success(),
        "provided-context with --text should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let (capability, intent, reasoning) = only_halt_resolve_row(&conn);
    assert_eq!(capability, "halt.resolve");
    assert_eq!(intent, "provided_context");
    let reasoning = reasoning.expect("reasoning column must be populated");
    assert!(
        reasoning.contains("provided_context: the issue is X"),
        "reasoning must carry the director-supplied context, got: {reasoning}"
    );
}

#[test]
fn halt_resolve_authorized_override_writes_adl_row_with_operator_policy_ref() {
    let (out, conn) = run_maosctl_and_inspect_db(
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
            "policy://override/2026-05",
        ],
    );
    assert!(
        out.status.success(),
        "authorized-override with --operator-policy should exit 0; got {:?}\nstdout: {}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let (capability, intent, reasoning) = only_halt_resolve_row(&conn);
    assert_eq!(capability, "halt.resolve");
    assert_eq!(intent, "authorized_override");
    let reasoning = reasoning.expect("reasoning column must be populated");
    assert!(
        reasoning.contains("authorized_override: operator_policy_ref=policy://override/2026-05"),
        "reasoning must carry the operator policy reference, got: {reasoning}"
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
    // clap's `required_if_eq` fires before `dispatch_halt` shells out, so the
    // usage diagnostic — not a maos-bin error — is what the operator sees.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--text"),
        "expected a clap usage error naming --text, got: {stderr}"
    );
}

#[test]
fn halt_resolve_empty_text_rejected() {
    let out = run_maosctl(
        &[],
        &[
            "halt",
            "resolve",
            "halt-007",
            "--spirit",
            "hello-spirit",
            "--kind",
            "provided-context",
            "--text",
            "   ",
        ],
    );
    assert!(
        !out.status.success(),
        "a blank --text must not resolve a halt; got {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("provided_context text must be non-empty"),
        "expected the domain non-empty diagnostic, got: {stderr}"
    );
}

#[test]
fn halt_list_no_color_emits_zero_ansi() {
    let out = run_maosctl(
        &[("NO_COLOR", "1")],
        &["halt", "list", "--spirit", "hello-spirit"],
    );
    assert!(
        out.status.success(),
        "halt list should exit 0; got {:?}\nstderr: {}",
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
fn halt_resolve_unknown_spirit_exits_two() {
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
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown spirit must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn halt_list_unknown_spirit_exits_two() {
    let out = run_maosctl(&[], &["halt", "list", "--spirit", "unknown-spirit"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "unknown spirit must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
}
