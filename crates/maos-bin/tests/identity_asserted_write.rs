#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 11.4c — composition-root write helper for the out-of-kernel
//! `identity.asserted` audit row (kind = 30).
//!
//! `maos_bin::enterprise_identity::append_identity_asserted` is the sanctioned
//! production write path for this one non-kernel provenance row. It lives in
//! `maos-bin` (the writer / composition-root side), NOT in `maos-audit`, which
//! is read-only by design (Story 9.2 Decision A.2, enforced by the
//! `maos-audit-read-only` gate). Every OTHER Transparency-Log row is written by
//! the kernel's `TransparencyLogAdapter::insert_frame_event`, which cannot
//! express kind 30 without an L1 kernel-core delta. This file pins the helper's
//! public contract:
//!
//! 1. It inserts exactly one row that surfaces via `maos_audit::query` with
//!    `kind == "identity.asserted"`.
//! 2. The four payload fields (`subject`, `issuer`, `capability_key`,
//!    `decision_time_ns`) round-trip through `payload_redacted`.
//! 3. `capability_token` is `NULL`, `origin == 0` (non-kernel), and
//!    `timestamp_ns == decision_time_ns` (the IdP assertion instant).
//! 4. It fails closed (returns `Err`) when the `transparency_log` table is
//!    absent — it does NOT create the schema.
//! 5. Two writes produce two distinct `frame_id`s (random, no hard-coded PK).

use maos_audit::{query, AuditFilter};
use maos_bin::enterprise_identity::append_identity_asserted;
use rusqlite::Connection;

/// Exact `transparency_log` DDL — mirrors the schema-creation helper in the
/// kind-map test (`identity_asserted_kind_test.rs`) and the kernel's own DDL.
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

/// Create a fresh Transparency-Log DB holding ONLY the schema (no rows).
fn fresh_db(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let db = dir.path().join(name);
    {
        let conn = Connection::open(&db).expect("open SQLite for schema init");
        conn.execute_batch(SCHEMA_SQL).expect("schema init");
        conn.close().expect("close schema-creation conn");
    }
    db
}

const SUBJECT: &str = "alice@example.com";
const ISSUER: &str = "https://idp.example.com";
const CAPABILITY_KEY: &str = "cap-key-7f3a";
const DECISION_TIME_NS: u64 = 1_700_000_000_000_000_000;
const SPIRIT_PID: u32 = 4242;
const BOOT_NONCE: u64 = 0xCAFE_F00D;

/// AC: one row written, read back as `kind == "identity.asserted"`, with the
/// four payload fields round-tripping and the non-kernel frame metadata set.
#[test]
fn append_identity_asserted_writes_one_kind_30_row_round_tripping_payload() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = fresh_db(&tmp, "identity-asserted-write.sqlite");

    append_identity_asserted(
        &db,
        SPIRIT_PID,
        BOOT_NONCE,
        SUBJECT,
        ISSUER,
        CAPABILITY_KEY,
        DECISION_TIME_NS,
    )
    .expect("append succeeds");

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 1, "exactly one row written");

    let row = &entries[0];
    assert_eq!(row.kind, "identity.asserted");
    assert_eq!(row.intent, "identity.asserted");
    assert_eq!(row.spirit_pid, SPIRIT_PID);
    assert_eq!(row.boot_nonce, BOOT_NONCE);
    assert_eq!(row.timestamp_ns, DECISION_TIME_NS);
    assert_eq!(
        row.capability_token_hex, None,
        "identity.asserted carries NO capability token"
    );

    // Payload round-trips as valid JSON with exactly the four fields.
    let payload: serde_json::Value =
        serde_json::from_str(&row.payload).expect("payload is valid JSON");
    let obj = payload.as_object().expect("payload is a JSON object");
    assert_eq!(obj.len(), 4, "exactly the four payload fields");
    assert_eq!(payload["subject"].as_str(), Some(SUBJECT));
    assert_eq!(payload["issuer"].as_str(), Some(ISSUER));
    assert_eq!(payload["capability_key"].as_str(), Some(CAPABILITY_KEY));
    assert_eq!(payload["decision_time_ns"].as_u64(), Some(DECISION_TIME_NS));
}

/// The row is filterable on the read side by the kind string — the SSO
/// adapter's downstream consumers query `kind = "identity.asserted"` directly.
#[test]
fn append_identity_asserted_row_is_filterable_by_kind_string() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = fresh_db(&tmp, "identity-asserted-filter.sqlite");

    append_identity_asserted(
        &db,
        SPIRIT_PID,
        BOOT_NONCE,
        SUBJECT,
        ISSUER,
        CAPABILITY_KEY,
        DECISION_TIME_NS,
    )
    .expect("append succeeds");

    let mut filter = AuditFilter::default();
    filter.kind = Some("identity.asserted".to_string());
    let matched = query(&db, filter).expect("filtered query succeeds");
    assert_eq!(matched.len(), 1, "kind filter matches the written row");

    // Negative control: an unrelated kind matches nothing.
    let mut other = AuditFilter::default();
    other.kind = Some("task.assign".to_string());
    let missed = query(&db, other).expect("other-kind query succeeds");
    assert!(
        missed.is_empty(),
        "task.assign filter must not match identity.asserted"
    );
}

/// Fail-closed: a missing `transparency_log` table is an error, NOT a silent
/// no-op (the helper does not create the schema — single-sourced in the kernel).
#[test]
fn append_identity_asserted_fails_closed_when_table_missing() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("no-schema.sqlite");
    // Create the FILE with no table (so the failure is the missing table, not
    // a missing file).
    {
        let conn = Connection::open(&db).expect("open SQLite");
        conn.close().expect("close");
    }

    let res = append_identity_asserted(&db, 1, 1, "s", "i", "k", 1);
    assert!(
        res.is_err(),
        "must fail closed when the transparency_log table is absent"
    );
}

/// Two writes produce two DISTINCT frame_ids — guards against an accidental
/// hard-coded frame_id that would collide on the second insert (which, with a
/// plain INSERT, must surface as an error rather than `OR IGNORE` swallowing).
#[test]
fn append_identity_asserted_two_writes_have_distinct_frame_ids() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = fresh_db(&tmp, "identity-asserted-distinct.sqlite");

    append_identity_asserted(&db, 1, 1, "s1", "i", "k", 10).expect("first append");
    append_identity_asserted(&db, 1, 1, "s2", "i", "k", 20)
        .expect("second append — distinct random frame_id, no PK collision");

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 2, "two distinct rows");
    assert_ne!(
        entries[0].frame_id_hex, entries[1].frame_id_hex,
        "frame_ids must be distinct (random, not a constant)"
    );
}
