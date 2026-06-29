#![forbid(unsafe_code)]

//! J4 — Mira/Nash bilateral pair journey (Grade A: real `maos run` topology).
//!
//! The topology path loads Mira and Nash through the production composition root.
//! The oracle is typed topology/drain output; the older smoke success marker is
//! no longer accepted as the sole signal.
//!
//! ## Anti-fake control
//!
//! The `ConsentRupture` oracle (`j4_earned_consent_rupture_typed_oracle_deferred`)
//! must go RED (fail) if production rupture emission is removed. Currently deferred
//! because the J4 topology run does not trigger a real ConsentRupture through the
//! `A2ARouterCore::handle_intake` deny path. When the topology is wired to exercise
//! a deny-path intake, un-ignore this test and assert on the production
//! `ConsentRupturePayload` + `RuptureReason::IntentAllowlistMismatch` types from
//! `maos_domain::frame`. The test must import production types — never local fakes.

use maos_journey_test::AuditDb;
use std::collections::BTreeSet;
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

/// Parse stdout lines as JSON, collecting successfully parsed values.
fn parse_json_lines(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn j4_mira_nash_topology_run_once() {
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j4-mira-nash.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn mira-nash topology");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Story 9.6 — topology Mira now has its EpistemicScalarPort wired via
    // ButlerOrchestratorAdapter, so the J4 topology loads and drains successfully.
    assert!(
        output.status.success(),
        "mira-nash topology should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // Structured JSON assertions — no string scraping.
    let events = parse_json_lines(&stdout);
    assert!(
        !events.is_empty(),
        "topology stdout must contain JSON events; raw stdout:\n{stdout}"
    );

    // Collect spirit_ids from spirit_loaded events.
    let loaded_ids: BTreeSet<&str> = events
        .iter()
        .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .filter_map(|e| e.get("spirit_id").and_then(|v| v.as_str()))
        .collect();
    let expected: BTreeSet<&str> = ["mira", "nash"].iter().copied().collect();
    assert_eq!(
        loaded_ids, expected,
        "topology must load exactly {{mira, nash}}; got {loaded_ids:?}"
    );

    // Assert a drain event with topology: true exists.
    let has_topology_drain = events.iter().any(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("drain")
            && e.get("topology").and_then(|v| v.as_bool()) == Some(true)
    });
    assert!(
        has_topology_drain,
        "J4 topology run should terminate through the drain-complete seam; events:\n{events:?}"
    );

    // The old smoke success-marker oracle must not appear.
    assert!(
        !stderr.contains("live TCP + real HTTP mobile-push J4 journey complete"),
        "success-marker-only smoke oracle must not be used by the Grade-A path; stderr:\n{stderr}"
    );
}

/// ConsentRupture oracle — uses production types from `maos_domain::frame`.
///
/// Runs the J4 topology and queries the transparency log for a
/// `ConsentRupture` row (kind=22). Currently `#[ignore]` because the J4
/// topology does not trigger the `A2ARouterCore::handle_intake` deny path.
/// When the topology is wired to exercise the classified-but-policy-denied
/// (`-32001`) intake leg, un-ignore this test.
///
/// This oracle MUST go RED if production rupture emission is removed — it
/// imports production types, never local fakes, and asserts on transparency-log
/// rows that only the deny path can produce.
#[test]
#[ignore = "ConsentRupture oracle deferred: J4 topology does not yet trigger a deny-path intake; \
            un-ignore when the topology exercises A2ARouterCore::handle_intake rejection"]
fn j4_earned_consent_rupture_typed_oracle_deferred() {
    // Production type imports — compile-error if the types are removed/renamed.
    use maos_domain::frame::RuptureReason;

    // ConsentRupture = kind discriminator 22 in the transparency log.
    const CONSENT_RUPTURE_KIND: i64 = 22;

    let audit = AuditDb::temp();
    let tl_path = audit.transparency_log_path();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j4-mira-nash.toml", "--once"])
        .env("XDG_DATA_HOME", audit.path())
        .env("MAOS_HOME", audit.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn mira-nash topology");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "mira-nash topology should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // Query the transparency log for ConsentRupture rows (kind=22).
    // When the deny-path intake is wired, this count must be >= 1.
    assert!(
        tl_path.exists(),
        "transparency log must be created by the topology run at {tl_path:?}"
    );
    let conn = rusqlite::Connection::open(&tl_path)
        .expect("open transparency log for ConsentRupture query");
    let rupture_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transparency_log WHERE kind = ?1",
            [CONSENT_RUPTURE_KIND],
            |row| row.get(0),
        )
        .expect("query ConsentRupture count");
    assert!(
        rupture_count >= 1,
        "transparency log must contain at least 1 ConsentRupture row (kind={CONSENT_RUPTURE_KIND}) \
         with reason {:?} from production site A2ARouterCore::handle_intake; \
         found {rupture_count} rows",
        RuptureReason::IntentAllowlistMismatch,
    );
}
