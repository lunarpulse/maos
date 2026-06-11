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

    // P8 — ConsentRupture observability (Murat's J4 seal).
    // The smoke arm internally earns the ConsentRupture on the live wire
    // (consent-denied → rupture frame) and only prints the success marker
    // AFTER confirming the rupture was observed.  This stderr assertion is
    // the genuine oracle — the smoke arm uses an in-memory TL for the
    // consent-rupture portion, so the file-backed TL at
    // transparency_log_path() has boot/config rows but no ConsentRupture rows.
    // Seal 3 (sever rupture sink) makes this test RED by preventing the
    // smoke arm from reaching the success marker.
    assert!(
        stderr.contains("live TCP + real HTTP mobile-push J4 journey complete"),
        "stderr should confirm ConsentRupture observability (J4 seal); got:\n{stderr}"
    );
}
