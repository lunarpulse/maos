#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 17-8 AC6.2: independent readers ignore populated consent side tables.

use maos_audit::AuditFilter;
use maos_domain::log_recall::{EmitterScopeReason, LogRecallFilter, RecallScope};
use maos_iac::adapter::log_recall::{LogRecallAdapter, PageScope};
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use rusqlite::{params, Connection};
use std::path::Path;

#[path = "../../../tests/harness/legacy_tl_schema.rs"]
mod legacy_tl_schema;
use legacy_tl_schema::LEGACY_SCHEMA_SQL;

fn fixture(path: &Path, current: bool) {
    if current {
        drop(TransparencyLogAdapter::open(path, 3).unwrap());
    }
    let conn = Connection::open(path).unwrap();
    if !current {
        conn.execute_batch(LEGACY_SCHEMA_SQL).unwrap();
    }
    for (id, pid, kind) in [(1u8, 42, 0), (2, 42, 1), (3, 43, 0), (4, 0, 22)] {
        conn.execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
              boot_nonce, capability_token, kind, intent, correlation_id, payload_redacted, origin)
             VALUES (?1, ?2, ?3, 'sender', 'recipient', 3, ?4, ?5, 'remote', 'correlation', ?6, 0)",
            params![
                vec![id; 16],
                i64::from(id),
                pid,
                vec![9u8; 32],
                kind,
                b"{\"value\":\"fixture\"}".as_slice()
            ],
        )
        .unwrap();
    }
    let tables = [
        "transparency_log_recipients",
        "transparency_log_consent",
        "transparency_log_consent_ruptures",
    ];
    for table in tables {
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, i64::from(current), "{table}");
    }
    if current {
        // Explicit side rows keep every base byte (including timestamps) identical.
        conn.execute(
            "INSERT INTO transparency_log_recipients VALUES (?1, 43, 'recipient', NULL)",
            [vec![1u8; 16]],
        )
        .unwrap();
        conn.execute("INSERT INTO transparency_log_consent VALUES (?1, ?2, 'sender', NULL, NULL, ?3, 'remote', NULL, 2)", params![vec![1u8; 16], vec![7u8; 16], i64::MIN + 1]).unwrap();
        conn.execute("INSERT INTO transparency_log_consent_ruptures VALUES (?1, 43, 'sender', NULL, 'remote')", [vec![4u8; 16]]).unwrap();
        for table in tables {
            let count: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 1, "fixture must exercise populated {table}");
        }
    }
    // Freeze these offline artifacts in rollback-journal mode: read-only WAL
    // connections can otherwise recreate a WAL rejected by deterministic export.
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
        .unwrap();
}

#[test]
fn independent_readers_return_identical_base_rows_with_populated_side_tables() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("legacy.sqlite");
    let current = dir.path().join("current.sqlite");
    fixture(&legacy, false);
    fixture(&current, true);

    for query in [maos_audit::query, maos_audit::query_with_redaction] {
        let before = query(&legacy, AuditFilter::default()).unwrap();
        let after = query(&current, AuditFilter::default()).unwrap();
        assert_eq!(before.len(), 4);
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(after).unwrap()
        );
    }
    let before = maos_siem::export_from_tl(&legacy, AuditFilter::default()).unwrap();
    assert_eq!(before.len(), 4);
    assert_eq!(
        before,
        maos_siem::export_from_tl(&current, AuditFilter::default()).unwrap()
    );
    assert_eq!(
        maos_audit::backup::compute_merkle_root(&legacy).unwrap(),
        maos_audit::backup::compute_merkle_root(&current).unwrap()
    );
    let canonical = |path: &Path| {
        let rows = maos_loom_lite::canonical::read_sqlite_frames(path).unwrap();
        assert_eq!(rows.len(), 4);
        rows.iter()
            .map(|row| row.to_canonical_bytes())
            .collect::<Vec<_>>()
    };
    assert_eq!(canonical(&legacy), canonical(&current));

    let readers =
        [&legacy, &current].map(|path| TransparencyLogAdapter::open_read_only(path).unwrap());
    let mut cursor = None;
    let mut ids = Vec::new();
    loop {
        let pages = readers.each_ref().map(|reader| {
            LogRecallAdapter::query_page(
                reader,
                42,
                PageScope::Emitter(EmitterScopeReason::CrossWall),
                LogRecallFilter {
                    limit: 1,
                    cursor: cursor.clone(),
                    ..Default::default()
                },
            )
            .unwrap()
        });
        assert_eq!(pages[0], pages[1]);
        assert_eq!(
            pages[0].scope,
            RecallScope::Emitter {
                reason: EmitterScopeReason::CrossWall
            }
        );
        ids.extend(pages[0].entries.iter().map(|row| row.frame_id));
        cursor = pages[0].next_cursor.clone();
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(ids, vec![[1; 16], [2; 16]]);
}
