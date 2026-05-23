#![forbid(unsafe_code)]

//! AC1 schema test (Story 1b.5b).
//!
//! Builds a hermetic in-memory SQLite seed mirroring the kernel-side
//! `transparency_log` schema, inserts one `kind = 9` (InferenceCall) row,
//! one `kind = 7` (CapabilityInvocation) row, and one row with
//! `capability_token = NULL` (the negative test), then asserts:
//!
//!   1. `maos_audit::query()` reads back all three rows.
//!   2. `maos_audit::to_fr4_ndjson()` emits exact-schema JSON for the
//!      two well-formed rows and aborts with
//!      `AuditError::Fr4SchemaViolation { line: 3, missing_field: "capability_token" }`
//!      on the NULL-token row.
//!   3. The first two emitted lines parse as valid JSON with exactly the
//!      six FR4 keys (`call_id`, `capability_token`, `spirit_pid`,
//!      `boot_nonce`, `call_type`, `timestamp_ns`).

use std::path::PathBuf;

use maos_audit::{query, to_fr4_ndjson, AuditError, AuditFilter};
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

/// Seed a hermetic on-disk SQLite at `path` with three rows:
///   - `kind = 9` (InferenceCall) with non-null capability_token
///   - `kind = 7` (CapabilityInvocation) with non-null capability_token
///   - `kind = 9` (InferenceCall) with `capability_token = NULL` (FR4 violation seed)
fn seed_db(path: &PathBuf) {
    let conn = Connection::open(path).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");

    // Row 1 — well-formed InferenceCall.
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x11u8; 16] as &[u8],
            1_000i64,
            0i64,
            0xDEAD_BEEFi64,
            &[0xAAu8; 32] as &[u8],
            9i64, // InferenceCall
            "claude-3-haiku",
            b"<redacted>" as &[u8],
            0i64,
        ],
    )
    .expect("insert row 1");

    // Row 2 — well-formed CapabilityInvocation.
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x22u8; 16] as &[u8],
            2_000i64,
            0i64,
            0xDEAD_BEEFi64,
            &[0xBBu8; 32] as &[u8],
            7i64, // CapabilityInvocation
            "provider.infer:anthropic",
            b"<redacted>" as &[u8],
            0i64,
        ],
    )
    .expect("insert row 2");

    // Row 3 — InferenceCall with NULL capability_token (FR4 violation seed).
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x33u8; 16] as &[u8],
            3_000i64,
            0i64,
            0xDEAD_BEEFi64,
            rusqlite::types::Null,
            9i64, // InferenceCall
            "claude-3-haiku",
            b"<redacted>" as &[u8],
            0i64,
        ],
    )
    .expect("insert row 3");
}

#[test]
fn query_returns_all_three_seeded_rows_in_timestamp_order() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("schema-test.sqlite");
    seed_db(&db);

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 3, "expected 3 rows");
    // Ordered by timestamp_ns ASC per the query implementation.
    assert_eq!(entries[0].timestamp_ns, 1_000);
    assert_eq!(entries[1].timestamp_ns, 2_000);
    assert_eq!(entries[2].timestamp_ns, 3_000);
    assert!(entries[0].capability_token_hex.is_some());
    assert!(entries[1].capability_token_hex.is_some());
    assert!(
        entries[2].capability_token_hex.is_none(),
        "row 3 must have NULL token"
    );
}

#[test]
fn fr4_projection_emits_exact_schema_keys_for_well_formed_rows() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("schema-test.sqlite");
    seed_db(&db);

    // Filter to spirit_pid = 0 (matches all three seeded rows).
    let mut filter = AuditFilter::default();
    filter.spirit_pid = Some(0);

    // Take only the first two rows (well-formed) to verify the happy-path
    // schema projection. The third row is the negative test below.
    let mut entries = query(&db, filter).expect("query");
    entries.truncate(2);

    let mut buf = Vec::new();
    to_fr4_ndjson(entries, &mut buf).expect("happy-path emit");

    let written = String::from_utf8(buf).expect("utf8");
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.len(), 2);

    for line in lines {
        let v: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        let obj = v.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "boot_nonce",
                "call_id",
                "call_type",
                "capability_token",
                "spirit_pid",
                "timestamp_ns",
            ],
            "FR4 schema must have exactly these six keys"
        );
        // Mandatory fields are non-null / non-empty.
        assert!(obj["capability_token"].as_str().unwrap().len() == 64);
        assert!(obj["call_id"].as_str().unwrap().len() == 32);
        assert!(obj["spirit_pid"].is_u64());
        assert!(obj["boot_nonce"].is_u64());
        assert!(obj["timestamp_ns"].is_u64());
        let call_type = obj["call_type"].as_str().unwrap();
        assert!(
            call_type == "inference.call" || call_type == "capability.invocation",
            "call_type must be a known kind, got {call_type}"
        );
    }
}

#[test]
fn fr4_projection_rejects_null_token_with_line_and_field() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("schema-test.sqlite");
    seed_db(&db);

    let entries = query(&db, AuditFilter::default()).expect("query");
    assert_eq!(entries.len(), 3);

    let mut buf = Vec::new();
    let err = to_fr4_ndjson(entries, &mut buf).expect_err("must fail on NULL token");
    match err {
        AuditError::Fr4SchemaViolation {
            line,
            missing_field,
        } => {
            assert_eq!(line, 3, "violation must name the offending 1-indexed line");
            assert_eq!(missing_field, "capability_token");
        }
        other => panic!("expected Fr4SchemaViolation, got {other:?}"),
    }

    // Output is buffered — no partial lines reach the writer on violation.
    let written = String::from_utf8(buf).unwrap();
    assert_eq!(
        written.lines().count(),
        0,
        "no partial output should be written on FR4 schema violation"
    );
}

#[test]
fn dispatcher_exit_code_mapping_documented_for_violation() {
    // This test documents the contract that the maos-cli dispatcher MUST
    // map `AuditError::Fr4SchemaViolation` to exit code 2 (see
    // `crates/maos-cli/src/subcommands.rs::audit_query`). The actual exit-code
    // wiring is tested in `crates/maos-cli/tests/audit_no_color_test.rs`; here
    // we just assert the error variant carries the data the dispatcher needs.
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("schema-test.sqlite");
    seed_db(&db);
    let entries = query(&db, AuditFilter::default()).unwrap();
    let mut buf = Vec::new();
    let err = to_fr4_ndjson(entries, &mut buf).unwrap_err();
    let display = format!("{err}");
    assert!(display.contains("FR4 schema violation"));
    assert!(display.contains("line 3"));
    assert!(display.contains("capability_token"));
}
