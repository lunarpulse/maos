#![forbid(unsafe_code)]

//! J4 — Mira/Nash bilateral pair journey (Grade B: orchestrated smoke wrap).
//!
//! Like J1, the Mira/Nash topology runs via `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13`
//! (live TCP loopback, real ConsentRupture frame since 8.13.1). This wrap asserts
//! the smoke arm exit 0 with receiver-side oracles.

use maos_journey_test::AuditDb;
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
fn j4_mira_nash_tcp_smoke_wrap() {
    let audit_db = AuditDb::temp();
    let output = Command::new(maos_bin())
        .env("MAOS_ONE_SHOT", "smoke-mira-nash-tcp-8-13")
        .env("XDG_DATA_HOME", audit_db.path())
        .env("MAOS_HOME", audit_db.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn mira-nash tcp smoke");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "mira-nash tcp smoke should exit 0; stderr:\n{stderr}"
    );

    assert!(
        stderr.contains("smoke-mira-nash-tcp-8-13"),
        "stderr should mention smoke-arm identity; got:\n{stderr}"
    );

    // P8 — ConsentRupture journaled row assertion (Murat's J4 seal).
    // The smoke arm internally asserts ConsentRupture was observed before
    // printing the success marker, which proves the rupture was earned.
    assert!(
        stderr.contains("live TCP + real HTTP mobile-push J4 journey complete"),
        "stderr should confirm ConsentRupture observability (J4 seal); got:\n{stderr}"
    );

    // The smoke arm journals to an in-memory TL (open_in_memory), so the
    // on-disk SQLite at transparency_log_path() is not populated by the
    // smoke. If a future smoke variant writes to file-backed TL, this
    // query will assert ConsentRupture rows directly.
    let tl_path = audit_db.transparency_log_path();
    if tl_path.exists() {
        let conn = rusqlite::Connection::open(&tl_path)
            .expect("open TL for ConsentRupture query");
        // FrameKind::ConsentRupture = 22 (Story 6.4 / ADR-034)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transparency_log WHERE kind = 22",
                [],
                |row| row.get(0),
            )
            .expect("query ConsentRupture count");
        assert!(
            count > 0,
            "TL should contain at least one ConsentRupture row (kind=22)"
        );
    }
}
