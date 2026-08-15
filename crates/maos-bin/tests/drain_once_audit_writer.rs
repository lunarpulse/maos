#![forbid(unsafe_code)]

//! Regression: topology `--once` must close every capability-audit sender before
//! awaiting the audit writer, so the writer flushes instead of timing out at exit.

use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn founder_loop_once_drains_capability_audit_writer() {
    let home = tempfile::TempDir::new().expect("create isolated MAOS home");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("spawn founder-loop topology");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "founder-loop topology must exit successfully; stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("audit writer topology drain timed out"),
        "topology --once must drain the audit writer before exit; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("topology --once complete"),
        "topology --once must reach its clean completion seam; stderr:\n{stderr}"
    );

    let audit_db = home.path().join("audit").join("transparency.sqlite");
    let connection = rusqlite::Connection::open_with_flags(
        audit_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open founder-loop transparency log");
    let capability_invocations: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM transparency_log WHERE kind = 7",
            [],
            |row| row.get(0),
        )
        .expect("count CapabilityInvocation audit rows");
    assert!(
        capability_invocations > 0,
        "founder-loop must persist its CapabilityInvocation audit rows before exit"
    );
}
