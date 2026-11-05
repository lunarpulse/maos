//! Story 20-11 hosted finding A: erasure scans must not index every routed append.
#![forbid(unsafe_code)]

use maos_iac::adapter::transparency_log::TransparencyLogAdapter;

#[test]
fn hosted_a_existing_full_indexes_are_replaced_by_predicate_indexes() {
    let dir = tempfile::tempdir().expect("isolated index migration");
    let path = dir.path().join("transparency.sqlite");
    drop(TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1).expect("TL"));
    let connection = rusqlite::Connection::open(&path).expect("legacy database");
    connection
        .execute_batch(
            "DROP INDEX IF EXISTS idx_tlog_distillate_rowid;
         DROP INDEX IF EXISTS idx_tlog_redistill_request_rowid;
         CREATE INDEX IF NOT EXISTS idx_tlog_kind_rowid ON transparency_log(kind);
         CREATE INDEX IF NOT EXISTS idx_tlog_intent_rowid ON transparency_log(intent);",
        )
        .expect("simulate the previously shipped full indexes");
    drop(connection);
    let log = TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 2)
        .expect("migrate existing database");
    let connection = rusqlite::Connection::open(&path).expect("inspect migrated schema");
    let old_count: u64 = connection.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='index' AND name IN ('idx_tlog_kind_rowid','idx_tlog_intent_rowid')",
        [], |row| row.get(0),
    ).expect("old index count");
    assert_eq!(
        old_count, 0,
        "existing databases retain the routed-append regression"
    );
    for (name, predicate) in [
        ("idx_tlog_distillate_rowid", "WHERE kind = 11"),
        (
            "idx_tlog_redistill_request_rowid",
            "WHERE intent = 'distillate.redistill.requested'",
        ),
    ] {
        let sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='index' AND name=?1",
                [name],
                |row| row.get(0),
            )
            .expect("new partial index");
        assert!(
            sql.contains(predicate),
            "{name} must use the reader's exact predicate: {sql}"
        );
    }
    assert!(log
        .distillates_written_by(&[])
        .expect("migrated lineage reader")
        .is_empty());
    assert!(log
        .outstanding_redistill_requests()
        .expect("migrated request reader")
        .is_empty());
    let outcome: u64 = connection.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='idx_tlog_redistill_outcome'",
        [], |row| row.get(0),
    ).expect("outcome index");
    assert_eq!(outcome, 1, "the outcome index is still required");
}

#[test]
fn hosted_a_ordinary_append_index_footprint_proxy() {
    fn measure(legacy: bool) -> (u64, i32, u64) {
        let dir = tempfile::tempdir().expect("bounded proxy fixture");
        let path = dir.path().join("transparency.sqlite");
        drop(TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1).expect("TL"));
        let mut connection = rusqlite::Connection::open(&path).expect("proxy connection");
        if legacy {
            connection
                .execute_batch(
                    "DROP INDEX idx_tlog_distillate_rowid;
                 DROP INDEX idx_tlog_redistill_request_rowid;
                 CREATE INDEX idx_tlog_kind_rowid ON transparency_log(kind);
                 CREATE INDEX idx_tlog_intent_rowid ON transparency_log(intent);",
                )
                .expect("exact pre-correction indexes");
        }
        let transaction = connection.transaction().expect("same append transaction");
        let mut insert = transaction.prepare(
            "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
             VALUES(?1,?2,7,1,?3,'route',X'7b7d',0)",
        ).expect("ordinary routed append shape");
        for ordinal in 0_u64..20_000 {
            let mut id = [0_u8; 16];
            id[8..].copy_from_slice(&ordinal.to_be_bytes());
            insert
                .execute(rusqlite::params![
                    id.as_slice(),
                    ordinal,
                    maos_iac::adapter::transparency_log::FrameKind::TaskAssign as i64,
                ])
                .expect("append");
        }
        let steps = insert.get_status(rusqlite::StatementStatus::VmStep);
        drop(insert);
        transaction.commit().expect("commit proxy");
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .expect("bound WAL footprint");
        let pages = connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .expect("pages");
        let bytes = std::fs::metadata(&path).expect("database footprint").len();
        drop(connection);
        dir.close().expect("remove proxy database promptly");
        (pages, steps, bytes)
    }
    let before = measure(true);
    let after = measure(false);
    eprintln!(
        "20-11 hosted-A cheap proxy, 20000 ordinary appends: full indexes pages={} VM-steps={} bytes={}; partial indexes pages={} VM-steps={} bytes={}; hosted callgrind delta UNVERIFIED",
        before.0, before.1, before.2, after.0, after.1, after.2,
    );
    assert!(
        after.0 < before.0,
        "ordinary appends must not populate the two erasure-only B-trees"
    );
}
