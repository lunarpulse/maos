#![forbid(unsafe_code)]

//! Story 11.4c Task 2 — the `identity.asserted` audit-kind discriminator pin.
//!
//! # The contract defended
//!
//! The Transparency-Log read side renders FrameKind discriminator 30 as the
//! dot-case string `"identity.asserted"` — the out-of-kernel identity-
//! provenance event added by Task 2. A row written with `kind = 30` MUST
//! surface through the PUBLIC `maos_audit::query` surface as
//! `AuditEntry.kind == "identity.asserted"`, NOT the generic `"unknown"`
//! fallback — otherwise identity-provenance frames are invisible and
//! unfilterable on the read side.
//!
//! Mirrors the i12 `decision.dispatch` (kind = 2) pin (i12_decision_audit_test):
//! seed a row with the raw discriminator, read it back through `query`, and
//! assert the rendered string. The kind↔int maps (`kind_to_string` /
//! `kind_from_string`) are private; `query` is the public black-box path that
//! exercises them.
//!
//! Discriminator 30 is the next free slot after `cost.attribution` (29); it is
//! pinned here so a renumbering on either side (SSO provenance kind string OR
//! audit map) breaks this test, not silent audit corruption.

use std::path::PathBuf;

use maos_audit::{query, AuditFilter};
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

/// Seed a single Transparency-Log row carrying kind = 30 (identity.asserted).
fn seed_identity_asserted_row(path: &PathBuf) {
    let conn = Connection::open(path).expect("open SQLite");
    conn.execute_batch(SCHEMA_SQL).expect("schema init");
    conn.execute(
        "INSERT INTO transparency_log \
         (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &[0x1Du8; 16] as &[u8], // frame_id
            7_000i64,               // timestamp_ns
            4242i64,                // spirit_pid
            0xCAFE_F00Di64,         // boot_nonce
            &[0xEEu8; 32] as &[u8], // capability_token (real blob, like i12)
            30i64,                  // kind — identity.asserted discriminator (Task 2)
            "identity.asserted",
            b"{}" as &[u8],         // payload_redacted
            0i64,                   // origin
        ],
    )
    .expect("insert identity.asserted row");
}

/// Discriminator 30 renders as `"identity.asserted"` on the read side. Today
/// `kind_to_string(30)` falls through to `"unknown"`, so this is RED until the
/// audit map is extended — exactly the missing-audit-kind failure mode.
#[test]
fn identity_asserted_discriminator_30_renders_as_identity_dot_asserted() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("identity-asserted-kind.sqlite");
    seed_identity_asserted_row(&db);

    let entries = query(&db, AuditFilter::default()).expect("query succeeds");
    assert_eq!(entries.len(), 1, "exactly one seeded row");
    assert_eq!(
        entries[0].kind, "identity.asserted",
        "discriminator 30 MUST render as 'identity.asserted' on the read side, \
         not the 'unknown' fallback"
    );
}
