#![forbid(unsafe_code)]

//! I12 decision-audit test (Story 3.3).
//!
//! Verifies that `decision.*` frames written with `working_memory_digest_refs`
//! survive the maos-audit read path. Because `maos-audit` depends only on
//! `maos-domain` (not `maos-kernel-core`), this test inserts rows directly
//! via rusqlite and reads them back through `maos_audit::query` plus a
//! direct rusqlite payload read to verify the I12 field round-trips
//! through serde.

use std::path::PathBuf;

use maos_audit::{query, AuditFilter};
use maos_domain::frame::DecisionDispatchPayload;
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use rusqlite::Connection;

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

fn seed_db(path: &PathBuf) {
    let conn = Connection::open(path).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");

    let payload = DecisionDispatchPayload {
        decision_id: 42,
        approved: true,
        working_memory_digest_refs: WorkingMemoryDigestRefs::new(vec![
            "frame-aaa".into(),
            "frame-bbb".into(),
        ]),
    };
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");

    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0xDDu8; 16] as &[u8],
            5_000i64,
            0i64,
            0xCAFE_F00Di64,
            &[0xEEu8; 32] as &[u8],
            2i64, // DecisionDispatch
            "decide",
            &payload_bytes as &[u8],
            0i64,
        ],
    )
    .expect("insert DecisionDispatch row");
}

#[test]
fn decision_dispatch_kind_maps_to_decision_dot_dispatch() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("i12-test.sqlite");
    seed_db(&db);

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, "decision.dispatch");
    assert_eq!(entries[0].intent, "decide");
    assert_eq!(entries[0].spirit_pid, 0);
}

#[test]
fn decision_dispatch_payload_preserves_working_memory_digest_refs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("i12-test.sqlite");
    seed_db(&db);

    let conn = Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");

    let payload_bytes: Vec<u8> = conn
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE kind = 2",
            [],
            |row| row.get(0),
        )
        .expect("read payload_redacted");

    let payload: DecisionDispatchPayload =
        serde_json::from_slice(&payload_bytes).expect("deserialize DecisionDispatchPayload");

    assert_eq!(payload.decision_id, 42);
    assert!(payload.approved);
    assert_eq!(
        payload.working_memory_digest_refs.as_slice(),
        &["frame-aaa", "frame-bbb"],
    );
}

#[test]
fn decision_dispatch_with_empty_digest_refs_round_trips() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("i12-test.sqlite");

    let conn = Connection::open(&db).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");

    let payload = DecisionDispatchPayload {
        decision_id: 7,
        approved: false,
        working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
    };
    let payload_bytes = serde_json::to_vec(&payload).expect("serialize payload");

    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0xCCu8; 16] as &[u8],
            6_000i64,
            0i64,
            0xCAFE_F00Di64,
            &[0xFFu8; 32] as &[u8],
            2i64,
            "decide",
            &payload_bytes as &[u8],
            0i64,
        ],
    )
    .expect("insert row");

    drop(conn);

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, "decision.dispatch");

    let conn2 = Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");
    let bytes: Vec<u8> = conn2
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE kind = 2",
            [],
            |row| row.get(0),
        )
        .expect("read payload");
    let back: DecisionDispatchPayload = serde_json::from_slice(&bytes).expect("deserialize");
    assert!(back.working_memory_digest_refs.as_slice().is_empty());
}
