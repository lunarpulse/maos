#![forbid(unsafe_code)]

//! Story 13.5g — Phase A in-artifact tenant binding wiring (AC2/AC3) and the
//! Phase B live-Postgres substrate (AC4).
//!
//! The pure verdict logic ([`maos_audit::decide_phase_a`] /
//! [`maos_audit::verify_datname_binding`]) is unit-tested in `maos-audit`
//! (hermetic `Blocking` legs). These integration tests cover the WIRING that
//! lives in `maos-bin::tenant_map::phase_a_preflight` — the read-only artifact
//! read + sidecar read + decide orchestration — the **composition-root
//! ordering** in `main.rs` exercised through the real binary, and the live
//! [`maos_loom_lite::store::LoomLiteStore::current_database`] operand that only
//! the Postgres substrate can provide.
//!
//! The boot legs matter because the helper legs cannot see `main.rs`: deleting
//! the Phase A block outright leaves every direct-call leg green. They drive
//! `MAOS_ONE_SHOT=cohort-a2a-daemon` with **no** `MAOS_COHORT_DAEMON_CONFIG`,
//! which is a precise ordering probe — that mode fails at
//! `cohort_daemon_bootstrap`, *after* the Transparency Log is opened, so the
//! error the binary actually prints says whether Phase A ran and whether it ran
//! before the open (D-4).
//!
//! Foreign-shard fixtures carry **≥ 1 `transparency_log` row** (Trap 1): an
//! empty copied artifact is genuinely indistinguishable from a fresh one and is
//! correctly treated as fresh (Trap 2).

use std::path::{Path, PathBuf};
use std::process::Command;

use maos_audit::{
    decide_phase_a, read_tenant_artifact, transparency_log_team_binding_path,
    verify_datname_binding, write_tenant_binding, DatnameBindingDecision,
    TenantBindingPhaseADecision, TenantBindingPhaseARefusal,
};
use maos_bin::tenant_map::phase_a_preflight;
use maos_domain::team::TeamId;

/// Build a TL artifact with `rows` transparency_log rows and an optional
/// in-artifact `tenant_binding` row + `.team` sidecar. Forces a WAL checkpoint
/// + drop so the rows are durable before the read-only open (Trap 6: WAL).
fn make_artifact(
    path: &Path,
    rows: usize,
    binding_team: Option<&str>,
    binding_datname: Option<&str>,
    sidecar_team: Option<&str>,
) {
    // AC1: every open this story adds carries NOFOLLOW, fixtures included.
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::default() | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open fixture");
    conn.execute_batch("PRAGMA journal_mode=WAL;").expect("wal");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transparency_log (
            frame_id BLOB NOT NULL PRIMARY KEY,
            timestamp_ns INTEGER NOT NULL,
            spirit_pid INTEGER NOT NULL,
            from_spirit_id TEXT NOT NULL DEFAULT '',
            to_spirit_id TEXT NOT NULL DEFAULT '',
            boot_nonce INTEGER NOT NULL,
            capability_token BLOB,
            kind INTEGER NOT NULL,
            intent TEXT NOT NULL,
            correlation_id TEXT,
            payload_redacted BLOB NOT NULL,
            origin INTEGER NOT NULL
        );",
    )
    .expect("create transparency_log");
    for i in 0..rows {
        let mut fid = [0u8; 16];
        fid[0..8].copy_from_slice(&(i as u64).to_be_bytes());
        conn.execute(
            "INSERT INTO transparency_log \
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, 1, 1, 1, 'seed', X'00', 0)",
            rusqlite::params![&fid[..], (i as i64) + 1],
        )
        .expect("insert seed row");
    }
    if let Some(team) = binding_team {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tenant_binding (\
                id INTEGER PRIMARY KEY CHECK (id = 1),\
                team_id TEXT NOT NULL,\
                datname TEXT,\
                bound_at_ns INTEGER NOT NULL\
            );",
        )
        .expect("create tenant_binding");
        conn.execute(
            "INSERT INTO tenant_binding (id, team_id, datname, bound_at_ns) VALUES (1, ?1, ?2, 1)",
            rusqlite::params![team, binding_datname],
        )
        .expect("insert binding");
    }
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
    drop(conn);
    if let Some(team) = sidecar_team {
        std::fs::write(transparency_log_team_binding_path(path), team).expect("write sidecar");
    }
}

/// `tenant_binding` table exists iff it was planted (read-only open must not
/// create it).
fn tenant_binding_table_exists(path: &Path) -> bool {
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open probe");
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master \
         WHERE type = 'table' AND name = 'tenant_binding')",
        [],
        |row| row.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn count_tl_rows(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open count");
    conn.query_row("SELECT COUNT(*) FROM transparency_log", [], |row| {
        row.get(0)
    })
    .unwrap_or(-1)
}

// ── Hermetic Blocking legs (Phase A wiring) ────────────────────────────────

/// AC3 row 5 / D-3 / D-4: a file-copied foreign shard carrying history (≥ 1 row)
/// and no matching `.team` sidecar is REFUSED before the TL is opened, and the
/// read-only preflight does NOT mutate the other team's artifact.
#[test]
fn phase_a_refuses_foreign_shard_with_history_before_append() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("transparency.sqlite");
    // Foreign shard: team-b's history, no in-artifact binding, no sidecar.
    make_artifact(&path, 3, None, None, None);
    let env = TeamId::new("team-a").unwrap();

    let decision = phase_a_preflight(&path, &env).expect("preflight reads");
    assert_eq!(
        decision,
        TenantBindingPhaseADecision::Refuse(
            TenantBindingPhaseARefusal::UnboundHistoryWithoutSidecar { env: env.clone() }
        ),
        "a foreign shard with history and no sidecar must be refused (D-3)"
    );

    // D-4 / AC2: the refused, read-only preflight must not have mutated the
    // artifact — no tenant_binding table was created, and the foreign rows are
    // intact.
    assert!(
        !tenant_binding_table_exists(&path),
        "Phase A preflight must be read-only (D-4): no tenant_binding created"
    );
    assert_eq!(count_tl_rows(&path), 3, "foreign history must be untouched");
}

/// AC3 row 2: an artifact whose in-artifact binding names a different team is
/// refused (this is the wiring twin of the pure verdict leg).
#[test]
fn phase_a_refuses_artifact_bound_to_foreign_team() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("transparency.sqlite");
    let foreign = TeamId::new("team-b").unwrap();
    // Plant a real binding for team-b through the production writer.
    make_artifact(&path, 0, None, None, None);
    write_tenant_binding(&path, &foreign, None).unwrap();
    let env = TeamId::new("team-a").unwrap();

    let decision = phase_a_preflight(&path, &env).expect("preflight reads");
    assert_eq!(
        decision,
        TenantBindingPhaseADecision::Refuse(TenantBindingPhaseARefusal::BoundToForeignTeam {
            bound: "team-b".to_string(),
            env: env.clone(),
        }),
        "an artifact bound to another team must be refused"
    );
}

/// AC3 rows 3 & 4 wiring: a fresh artifact (no binding, 0 rows) returns
/// NeedsWrite, and the binding written after a successful open is read back so
/// the next boot Proceeds. Exercises the NeedsWrite → write → Proceed cycle.
#[test]
fn phase_a_needs_write_then_proceeds_after_binding_written() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("transparency.sqlite");
    let env = TeamId::new("team-a").unwrap();

    // Fresh: no file yet → NeedsWrite.
    let decision = phase_a_preflight(&path, &env).expect("preflight reads");
    assert_eq!(decision, TenantBindingPhaseADecision::NeedsWrite);

    // Composition root writes the binding after a successful open.
    write_tenant_binding(&path, &env, None).unwrap();

    // Next boot: binding present and == env → Proceed.
    let decision = phase_a_preflight(&path, &env).expect("preflight reads");
    assert_eq!(decision, TenantBindingPhaseADecision::Proceed);
}

/// AC3 row 4 wiring: a legacy team log (history + matching `.team` sidecar, no
/// in-artifact binding) migrates silently — NeedsWrite, not Refuse.
#[test]
fn phase_a_legacy_artifact_with_matching_sidecar_migrates() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("transparency.sqlite");
    let env = TeamId::new("team-a").unwrap();
    make_artifact(&path, 2, None, None, Some("team-a"));

    let decision = phase_a_preflight(&path, &env).expect("preflight reads");
    assert_eq!(
        decision,
        TenantBindingPhaseADecision::NeedsWrite,
        "a legacy log with a matching sidecar migrates, not refuses"
    );
}

// ── Live AdvisorySubstrate leg (Phase B, AC4) ──────────────────────────────
//
// The pure `verify_datname_binding` is a hermetic Blocking leg in `maos-audit`.
// This `#[ignore]`d leg proves the live substrate half of AC4: a real
// `current_database()` value flows into the persisted-vs-live comparison, a
// matching persisted datname Proceeds, and a drifted persisted datname is
// Refused. Run only against a live Postgres (`--ignored` + MAOS_LOOM_POSTGRES).

/// Live Phase B check. Panics — never skips — when the substrate env the gate
/// keys on is absent: `check-reza-production-path` declares this leg's substrate
/// present from `MAOS_TEST_POSTGRES_TEAM_A`/`_B`, and its only anti-vacuity
/// oracle is `running 1 test`/`1 passed`, so an early `return` here would score
/// green having connected to nothing. Same `.expect` idiom as every other live
/// leg in the workspace (`cohort_daemon_smoke_13_5c.rs`, `tenant_wall_live.rs`).
#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A (live Postgres)"]
async fn phase_b_persisted_datname_vs_live_current_database() {
    let conn_str = std::env::var("MAOS_TEST_POSTGRES_TEAM_A")
        .expect("MAOS_TEST_POSTGRES_TEAM_A must be set for the live Phase B leg");
    let store = maos_loom_lite::store::LoomLiteStore::new(maos_loom_lite::store::StoreConfig {
        connection_string: conn_str,
        ..Default::default()
    })
    .await
    .expect("connect live Postgres");
    let live = store
        .current_database()
        .await
        .expect("current_database() against live Postgres");

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("transparency.sqlite");
    let team = TeamId::new("team-a").unwrap();

    // Persisted datname != live → drift refused.
    write_tenant_binding(&path, &team, Some("maos_definitely_not_live")).unwrap();
    let read = read_tenant_artifact(&path).unwrap();
    assert!(
        matches!(
            verify_datname_binding(read.binding_datname.as_deref(), &live),
            DatnameBindingDecision::RefuseDatnameDrift { .. }
        ),
        "a persisted datname drifting from the live database must be refused"
    );

    // Persisted datname == live → proceed (the steady-state second boot).
    write_tenant_binding(&path, &team, Some(&live)).unwrap();
    let read = read_tenant_artifact(&path).unwrap();
    assert_eq!(
        verify_datname_binding(read.binding_datname.as_deref(), &live),
        DatnameBindingDecision::Proceed,
        "a persisted datname matching the live database proceeds"
    );

    // Sanity: the pure decision agrees with the live-derived operands.
    assert_eq!(
        decide_phase_a(read.binding_team.as_deref(), &team, 0, None),
        TenantBindingPhaseADecision::Proceed
    );
}

// ── Composition-root ordering legs (AC2/AC3/D-4, through the real binary) ──
//
// Everything above calls `phase_a_preflight` directly and therefore cannot see
// `main.rs`: delete the Phase A block from the composition root and every leg
// above stays green. These two drive the shipped binary instead.
//
// The probe is `MAOS_ONE_SHOT=cohort-a2a-daemon` with NO
// `MAOS_COHORT_DAEMON_CONFIG`. That mode fails inside `cohort_daemon_bootstrap`,
// which runs AFTER `open_with_global_legal_holds`, so the transcript is an
// ordering oracle rather than a bare exit code:
//   * refused at Phase A  → the refusal text, and NO "opened on-disk" line;
//   * proceeded past it   → "opened on-disk", then the daemon-config error.
// `MAOS_LOOM_POSTGRES` is only probed with `is_some()` at Phase A, so a bogus
// value arms tenant mode while keeping the leg hermetic — the collective store
// is not constructed until long after both assertions have been decided.

/// `"maos: Transparency Log opened on-disk at"` — printed immediately after
/// `open_with_global_legal_holds` returns.
const TL_OPENED_MARKER: &str = "Transparency Log opened on-disk at";
/// The tenant-map failure raised in the `MAOS_LOOM_POSTGRES` arm, which sits
/// after BOTH the Transparency Log open and the deferred binding write. Seeing
/// it proves the boot travelled past both; not seeing it proves it did not.
const POST_OPEN_MARKER: &str = "tenant map construction failed";

/// Where `resolved_transparency_log_path()` lands in tenant mode under
/// `MAOS_HOME`, which takes precedence over `MAOS_AUDIT_DB`.
fn tenant_shard(home: &Path, team: &str) -> PathBuf {
    home.join("audit")
        .join("teams")
        .join(team)
        .join("transparency.sqlite")
}

/// Run the shipped binary through the composition root with tenant mode armed
/// and no live Postgres, returning the merged stdout+stderr transcript. Both
/// callers expect a non-zero exit, so `.output()` cannot deadlock on a listener.
fn boot_tenant(home: &Path, team: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env("MAOS_HOME", home)
        .env_remove("MAOS_AUDIT_DB")
        .env("MAOS_LOOM_POSTGRES", "postgres://127.0.0.1:1/hermetic")
        .env("MAOS_LOOM_HOME_TEAM", team)
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_OLLAMA_URL", "skip")
        .output()
        .expect("spawn maos");
    let transcript = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "this probe boots without a daemon config and must always exit non-zero; \
         transcript:\n{transcript}"
    );
    transcript
}

/// AC2 + D-4 at the composition root: a foreign shard carrying history is
/// refused by the real binary, and the refusal lands BEFORE the Transparency
/// Log is opened. Deleting or reordering the Phase A block in `main.rs` reds
/// this leg and no other.
#[test]
fn boot_refuses_foreign_shard_before_opening_the_transparency_log() {
    let dir = tempfile::TempDir::new().unwrap();
    let shard = tenant_shard(dir.path(), "team-a");
    std::fs::create_dir_all(shard.parent().unwrap()).unwrap();
    // Trap 1: foreign history, no sidecar. An EMPTY shard would be the fresh
    // row of the AC3 table and would prove nothing.
    make_artifact(&shard, 3, None, None, None);

    let transcript = boot_tenant(dir.path(), "team-a");

    assert!(
        transcript.contains("refused before open (Phase A)"),
        "the boot must refuse at Phase A; transcript:\n{transcript}"
    );
    assert!(
        !transcript.contains(TL_OPENED_MARKER),
        "D-4: the refusal must precede the Transparency Log open; transcript:\n{transcript}"
    );
    assert!(
        !transcript.contains(POST_OPEN_MARKER),
        "the boot must stop at Phase A, never reaching the collective-store arm; \
         transcript:\n{transcript}"
    );
    assert!(
        !tenant_binding_table_exists(&shard),
        "a refused boot must not mutate the other team's artifact (read-only preflight)"
    );
    assert_eq!(
        count_tl_rows(&shard),
        3,
        "the foreign history must be untouched by the refused boot"
    );
}

/// AC3 row 4 + the deferred binding write at the composition root: a legacy
/// shard (history + matching `.team` sidecar, no in-artifact binding) proceeds
/// past Phase A, and the binding row is written after the open. Deleting the
/// `pending_tenant_binding_write` consumption in `main.rs` reds this leg.
#[test]
fn boot_writes_binding_after_open_for_a_legacy_shard() {
    let dir = tempfile::TempDir::new().unwrap();
    let shard = tenant_shard(dir.path(), "team-a");
    std::fs::create_dir_all(shard.parent().unwrap()).unwrap();
    // The sidecar is written as `team + "\n"` by `bind_tenant_audit_artifact`.
    make_artifact(&shard, 2, None, None, Some("team-a\n"));

    let transcript = boot_tenant(dir.path(), "team-a");

    assert!(
        transcript.contains(TL_OPENED_MARKER),
        "a legacy shard must migrate, not refuse; transcript:\n{transcript}"
    );
    assert!(
        transcript.contains(POST_OPEN_MARKER),
        "the boot must reach the store arm, proving it passed Phase A and the write; \
         transcript:\n{transcript}"
    );

    let read = read_tenant_artifact(&shard).expect("read migrated artifact");
    assert_eq!(
        read.binding_team.as_deref(),
        Some("team-a"),
        "the in-artifact binding must be written after a successful open"
    );
    assert_eq!(
        read.binding_datname, None,
        "datname belongs to Phase B and is recorded on the first tenant boot"
    );
    assert!(
        read.transparency_log_rows >= 2,
        "the legacy history must survive the migration"
    );
}
