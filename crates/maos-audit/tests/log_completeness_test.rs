#![forbid(unsafe_code)]

//! Log-completeness corpus test for `maos-audit`.
//!
//! Validates the deterministic N=100 fixture at
//! `tests/fixtures/log-completeness-v0/events.jsonl`:
//!
//! 1. SHA-256 of the fixture matches the pinned hash.
//! 2. All 100 events round-trip through SQLite → `maos_audit::query()`
//!    with ≥98/100 frame_id recovery.
//! 3. PID-reuse-across-boot events (spirit_pid=42, boot_nonce=1000 vs 2000)
//!    are correctly differentiated.
//! 4. I11 cycle case (distillate referencing another distillate) is present.

use std::collections::HashSet;
use std::path::Path;

use maos_audit::{query, AuditFilter};

/// Schema matching the kernel-side `transparency_log` table.
const TL_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY,
    timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL,
    boot_nonce INTEGER NOT NULL,
    capability_token BLOB,
    kind INTEGER NOT NULL,
    intent TEXT NOT NULL,
    payload_redacted BLOB NOT NULL,
    origin INTEGER NOT NULL
);";

/// Pinned SHA-256 of `events.jsonl`. Regenerate by running the fixture
/// generator and copying the output from `events.jsonl.sha256`.
const PINNED_SHA256: &str = "1e181983068d834f0997e96c90236be04a53fef03275610a651a167161d751f3";

#[derive(serde::Deserialize)]
struct FixtureEvent {
    frame_id: String,
    timestamp_ns: u64,
    spirit_pid: u32,
    boot_nonce: u64,
    capability_token: Option<String>,
    kind: i64,
    intent: String,
    payload_redacted: String,
    origin: i64,
}

/// Read the fixture, verify SHA-256, insert into SQLite, query back, assert
/// recovery ≥98/100 and pid-reuse differentiation.
#[test]
fn log_completeness_corpus_round_trip() {
    // ── 1. Read fixture ─────────────────────────────────────────────────
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let fixture_path =
        Path::new(&manifest_dir).join("tests/fixtures/log-completeness-v0/events.jsonl");
    let sha_path =
        Path::new(&manifest_dir).join("tests/fixtures/log-completeness-v0/events.jsonl.sha256");

    let fixture_bytes = std::fs::read(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", fixture_path, e));

    // ── 2. Verify SHA-256 ───────────────────────────────────────────────
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&fixture_bytes);
    let computed = format!("{:x}", hasher.finalize());

    // Also read the pinned file for a double-check
    let pinned_from_file = std::fs::read_to_string(&sha_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", sha_path, e))
        .trim()
        .to_string();

    assert_eq!(
        computed, PINNED_SHA256,
        "computed SHA-256 does not match compile-time pinned constant"
    );
    assert_eq!(
        computed, pinned_from_file,
        "computed SHA-256 does not match events.jsonl.sha256 file"
    );

    // ── 3. Parse JSONL ──────────────────────────────────────────────────
    let fixture_str = String::from_utf8(fixture_bytes).expect("fixture is not valid UTF-8");
    let events: Vec<FixtureEvent> = fixture_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("invalid JSON line in fixture"))
        .collect();
    assert_eq!(events.len(), 100, "expected exactly 100 events in fixture");

    // ── 4. Create in-memory SQLite and insert ────────────────────────────
    let tmpdir = tempfile::TempDir::new().unwrap();
    let db_path = tmpdir.path().join("test.sqlite");

    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(TL_SCHEMA).unwrap();

        let mut insert_stmt = conn
            .prepare(
                "INSERT INTO transparency_log
                (frame_id, timestamp_ns, spirit_pid, boot_nonce,
                 capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )
            .unwrap();

        for ev in &events {
            let frame_id_bytes = hex::decode(&ev.frame_id).expect("invalid frame_id hex");
            let cap_bytes = ev
                .capability_token
                .as_ref()
                .map(|h| hex::decode(h).expect("invalid capability_token hex"));
            let payload_bytes =
                hex::decode(&ev.payload_redacted).expect("invalid payload_redacted hex");

            insert_stmt
                .execute(rusqlite::params![
                    frame_id_bytes,
                    ev.timestamp_ns as i64,
                    ev.spirit_pid as i64,
                    ev.boot_nonce as i64,
                    cap_bytes,
                    ev.kind,
                    ev.intent,
                    payload_bytes,
                    ev.origin,
                ])
                .unwrap();
        }
    } // drop write connection

    // ── 5. Query back via maos_audit::query() ────────────────────────────
    let recovered = query(&db_path, AuditFilter::default()).expect("query failed");

    // ── 6. Assert ≥98/100 recovery ──────────────────────────────────────
    let recovered_ids: HashSet<String> = recovered.iter().map(|e| e.frame_id_hex.clone()).collect();
    let fixture_ids: HashSet<String> = events.iter().map(|e| e.frame_id.clone()).collect();

    let overlap = recovered_ids.intersection(&fixture_ids).count();
    assert!(
        overlap >= 98,
        "recovery rate {}/100 below 98/100 threshold",
        overlap,
    );

    // ── 7. PID-reuse-across-boot differentiation ────────────────────────
    // Find events with spirit_pid=42 in fixture
    let pid42_boot1000: Vec<_> = events
        .iter()
        .filter(|e| e.spirit_pid == 42 && e.boot_nonce == 1000)
        .collect();
    let pid42_boot2000: Vec<_> = events
        .iter()
        .filter(|e| e.spirit_pid == 42 && e.boot_nonce == 2000)
        .collect();

    assert!(
        !pid42_boot1000.is_empty(),
        "fixture must contain pid-reuse event with boot_nonce=1000"
    );
    assert!(
        !pid42_boot2000.is_empty(),
        "fixture must contain pid-reuse event with boot_nonce=2000"
    );

    // Verify query with boot_nonce filter differentiates correctly
    let filter_boot1000 = AuditFilter {
        spirit_pid: Some(42),
        boot_nonce: Some(1000),
        ..AuditFilter::default()
    };
    let filter_boot2000 = AuditFilter {
        spirit_pid: Some(42),
        boot_nonce: Some(2000),
        ..AuditFilter::default()
    };

    let recovered_boot1000 = query(&db_path, filter_boot1000).unwrap();
    let recovered_boot2000 = query(&db_path, filter_boot2000).unwrap();

    // All recovered entries must have the correct boot_nonce
    for entry in &recovered_boot1000 {
        assert_eq!(
            entry.boot_nonce, 1000,
            "boot_nonce filter returned wrong nonce: got {}, expected 1000",
            entry.boot_nonce,
        );
        assert_eq!(entry.spirit_pid, 42);
    }
    for entry in &recovered_boot2000 {
        assert_eq!(
            entry.boot_nonce, 2000,
            "boot_nonce filter returned wrong nonce: got {}, expected 2000",
            entry.boot_nonce,
        );
        assert_eq!(entry.spirit_pid, 42);
    }

    // The two boot-nonce sets must be disjoint on frame_id
    let ids_1000: HashSet<String> = recovered_boot1000
        .iter()
        .map(|e| e.frame_id_hex.clone())
        .collect();
    let ids_2000: HashSet<String> = recovered_boot2000
        .iter()
        .map(|e| e.frame_id_hex.clone())
        .collect();
    assert!(
        ids_1000.is_disjoint(&ids_2000),
        "pid-reuse events with different boot_nonces must have disjoint frame_ids"
    );

    // Each boot-nonce set must recover at least one fixture entry
    let fid_1000: HashSet<String> = pid42_boot1000.iter().map(|e| e.frame_id.clone()).collect();
    let fid_2000: HashSet<String> = pid42_boot2000.iter().map(|e| e.frame_id.clone()).collect();
    assert!(
        !ids_1000.is_disjoint(&fid_1000),
        "boot_nonce=1000 events not found"
    );
    assert!(
        !ids_2000.is_disjoint(&fid_2000),
        "boot_nonce=2000 events not found"
    );

    // ── 8. I11 cycle case ───────────────────────────────────────────────
    let distillate_events: Vec<_> = events.iter().filter(|e| e.kind == 11).collect();
    assert!(
        distillate_events.len() >= 2,
        "fixture must contain at least 2 Distillate frames for I11 cycle, found {}",
        distillate_events.len(),
    );

    // Verify the referencing distillate payload contains a source log ref
    // to another distillate's frame_id
    let distillate_frame_ids: HashSet<String> = distillate_events
        .iter()
        .map(|e| e.frame_id.clone())
        .collect();

    let mut found_i11_cycle = false;
    for de in &distillate_events {
        let payload_bytes = hex::decode(&de.payload_redacted).unwrap();
        if let Ok(payload_str) = std::str::from_utf8(&payload_bytes) {
            for other_id in &distillate_frame_ids {
                if *other_id != de.frame_id && payload_str.contains(other_id) {
                    found_i11_cycle = true;
                    break;
                }
            }
        }
        if found_i11_cycle {
            break;
        }
    }
    assert!(
        found_i11_cycle,
        "I11 cycle case not found: no distillate payload references another distillate's frame_id"
    );
}
