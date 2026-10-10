#![forbid(unsafe_code)]

//! Regression: topology `--once` must close every capability-audit sender before
//! awaiting the audit writer, so the writer flushes instead of timing out at exit.

use std::process::Command;

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

fn add_t3_environment(command: &mut Command) {
    for key in [
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}
fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn founder_loop_once_drains_capability_audit_writer() {
    if !t3_substrate::require_t3("founder-loop audit-drain Worker") {
        return;
    }
    let home = tempfile::TempDir::new().expect("create isolated MAOS home");
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command.env_clear();
    add_t3_environment(&mut command);
    let output = command
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(home.path()))
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .env("PATH", {
            let profile = std::env::current_exe()
                .expect("test executable path")
                .parent()
                .and_then(std::path::Path::parent)
                .expect("test profile directory")
                .to_path_buf();
            let mut paths = vec![profile.clone(), profile.join("deps")];
            if let Some(existing) = std::env::var_os("PATH") {
                paths.extend(std::env::split_paths(&existing));
            }
            std::env::join_paths(paths).expect("test PATH")
        })
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
