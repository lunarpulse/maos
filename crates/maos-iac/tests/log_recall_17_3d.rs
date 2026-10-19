#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_domain::invariants::{i1::TokenId, i3::FrameOrigin};
use maos_domain::log_recall::{FrameKindLabel, LogRecallCursor, LogRecallError, LogRecallFilter};
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::{is_spirit_disclosable, BoundedFetch, LogRecallAdapter};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use rusqlite::{params, Connection};

fn insert(
    log: &TransparencyLogAdapter,
    pid: u32,
    kind: FrameKind,
    intent: &str,
    payload: &[u8],
) -> [u8; 16] {
    log.insert_frame_event(kind, pid, None, intent, payload, FrameOrigin::SpiritAuto);
    log.last_frame_id()
}

fn raw_insert(
    conn: &Connection,
    id: u128,
    boot: u64,
    pid: u32,
    kind: i64,
    intent: &str,
) -> [u8; 16] {
    let frame_id = id.to_be_bytes();
    conn.execute(
        "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, X'736563726574', 1)",
        params![&frame_id[..], id as i64, pid, boot as i64, kind, intent],
    ).unwrap();
    frame_id
}

#[test]
fn same_pid_in_different_boots_cannot_recall_or_fetch_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("log.db");
    let first = Arc::new(TransparencyLogAdapter::open(&path, 11).unwrap());
    let old_id = insert(&first, 7, FrameKind::TaskAssign, "old", b"old incarnation");
    let second = Arc::new(TransparencyLogAdapter::open(&path, u64::MAX).unwrap());
    let new_id = insert(
        &second,
        7,
        FrameKind::McpInvocation,
        "new",
        b"new incarnation",
    );
    for (local, own_id, foreign_id) in [(first, old_id, new_id), (second, new_id, old_id)] {
        let adapter = LogRecallAdapter::new(local);
        let page = adapter.recall(7, LogRecallFilter::default()).unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| entry.frame_id)
                .collect::<Vec<_>>(),
            vec![own_id]
        );
        assert!(adapter.fetch(7, own_id).is_ok());
        assert!(matches!(
            adapter.fetch(7, foreign_id),
            Err(LogRecallError::ScopeViolation { .. })
        ));
        assert!(matches!(
            adapter.fetch_attributed_bounded(7, foreign_id, &TokenId([9; 16]), 0),
            Err(LogRecallError::ScopeViolation { .. })
        ));
    }
}

#[test]
fn audit_receipts_identity_and_unknown_kinds_never_starve_pages_or_disclose() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("log.db");
    let log = Arc::new(TransparencyLogAdapter::open(&path, 12).unwrap());
    let mut conn = Connection::open(&path).unwrap();
    let transaction = conn.transaction().unwrap();
    for index in 1..=2000 {
        raw_insert(&transaction, index, 12, 7, 7, "log.recall");
        raw_insert(
            &transaction,
            2000 + index,
            12,
            7,
            28,
            "log.fetch.refused:scope-violation",
        );
    }
    let receipt = raw_insert(
        &transaction,
        4001,
        12,
        7,
        28,
        "log.recall.refused:capability-denied",
    );
    let authorization = raw_insert(&transaction, 4002, 12, 7, 28, "spirit.frame.authorization");
    let identity = raw_insert(&transaction, 4003, 12, 7, 30, "identity.asserted");
    let unknown = raw_insert(&transaction, 4004, 12, 7, 99, "unknown");
    transaction.commit().unwrap();
    let expected = [
        insert(&log, 7, FrameKind::TaskAssign, "one", b"one"),
        insert(&log, 7, FrameKind::TaskComplete, "two", b"two"),
        insert(&log, 7, FrameKind::McpInvocation, "three", b"three"),
    ];
    let adapter = LogRecallAdapter::new(log);
    let page = adapter
        .recall(7, LogRecallFilter::new(None, None, None, 16, None, None))
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.frame_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(page.next_cursor.is_none());
    let mut walked = Vec::new();
    let mut cursor = None;
    loop {
        let page = adapter
            .recall(7, LogRecallFilter::new(None, None, None, 1, cursor, None))
            .unwrap();
        walked.extend(page.entries.into_iter().map(|entry| entry.frame_id));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(walked, expected);
    for id in [receipt, authorization, identity, unknown] {
        assert!(matches!(
            adapter.fetch(7, id),
            Err(LogRecallError::ScopeViolation { .. })
        ));
        assert!(matches!(
            adapter.fetch_attributed_bounded(7, id, &TokenId([8; 16]), 0),
            Err(LogRecallError::ScopeViolation { .. })
        ));
    }
    for kind in [
        FrameKindLabel::CapabilityInvocation,
        FrameKindLabel::GovernanceEvent,
        FrameKindLabel::InferenceCall,
    ] {
        assert!(adapter
            .recall(
                7,
                LogRecallFilter::new(Some(kind), None, None, 16, None, None)
            )
            .unwrap()
            .entries
            .is_empty());
    }
}

#[test]
fn exact_intent_is_applied_before_limit_and_keyset_pagination() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(13));
    for _ in 0..100 {
        insert(&log, 7, FrameKind::TaskAssign, "unwanted", b"noise");
    }
    let first = insert(&log, 7, FrameKind::TaskAssign, "wanted", b"");
    for _ in 0..100 {
        insert(
            &log,
            7,
            FrameKind::TaskAssign,
            "wanted-prefix-only",
            b"noise",
        );
    }
    let last = insert(&log, 7, FrameKind::McpInvocation, "wanted", b"payload");
    let adapter = LogRecallAdapter::new(log);
    let page = adapter
        .recall(
            7,
            LogRecallFilter::new(None, None, None, 1, None, Some("wanted".into())),
        )
        .unwrap();
    assert_eq!(page.entries[0].frame_id, first);
    assert!(!page.entries[0].payload_available);
    let page = adapter
        .recall(
            7,
            LogRecallFilter::new(None, None, None, 1, page.next_cursor, Some("wanted".into())),
        )
        .unwrap();
    assert_eq!(page.entries[0].frame_id, last);
    assert!(page.entries[0].payload_available);
    assert!(page.next_cursor.is_none());
}

#[test]
fn insertion_order_pages_keep_scope_and_disclosure_when_timestamps_move_backward() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("log.db");
    let log = Arc::new(TransparencyLogAdapter::open(&path, 23).unwrap());
    let conn = Connection::open(&path).unwrap();
    let first = raw_insert(&conn, 90, 23, 7, 18, "mcp");
    let second = raw_insert(&conn, 10, 23, 7, 18, "mcp");
    raw_insert(&conn, 200, 23, 8, 18, "foreign pid");
    raw_insert(&conn, 20, 23, 7, 28, "receipt");
    raw_insert(&conn, 30, 24, 7, 18, "foreign boot");
    let third = raw_insert(&conn, 5, 23, 7, 18, "mcp");
    let adapter = LogRecallAdapter::new(log);
    let mut cursor = None;
    for id in [first, second, third] {
        let page = adapter
            .recall(
                7,
                LogRecallFilter {
                    limit: 1,
                    cursor,
                    order_by_insertion: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| entry.frame_id)
                .collect::<Vec<_>>(),
            [id]
        );
        let entry = &page.entries[0];
        cursor = Some(LogRecallCursor::new(entry.timestamp_ns, entry.frame_id));
    }
    let filter = LogRecallFilter {
        cursor,
        order_by_insertion: true,
        ..Default::default()
    };
    assert!(adapter
        .recall(7, filter.clone())
        .unwrap()
        .entries
        .is_empty());
    let fourth = raw_insert(&conn, 2, 23, 7, 18, "mcp");
    let page = adapter.recall(7, filter).unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.frame_id)
            .collect::<Vec<_>>(),
        [fourth]
    );
    let timestamp_page = adapter.recall(7, LogRecallFilter::default()).unwrap();
    assert_eq!(
        timestamp_page
            .entries
            .iter()
            .map(|entry| entry.frame_id)
            .collect::<Vec<_>>(),
        [fourth, third, second, first],
        "the default/WASM timestamp contract is unchanged"
    );
}

#[test]
fn unrepresentable_timestamps_fail_before_any_invocation_is_written() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(14));
    let adapter = LogRecallAdapter::new(Arc::clone(&log));
    let filters = [
        LogRecallFilter::new(None, Some(u64::MAX), None, 1, None, None),
        LogRecallFilter::new(None, None, Some(i64::MAX as u64 + 1), 1, None, None),
        LogRecallFilter::new(
            None,
            None,
            None,
            1,
            Some(LogRecallCursor::new(u64::MAX, [1; 16])),
            None,
        ),
    ];
    for filter in filters {
        assert!(matches!(
            adapter.recall(7, filter.clone()),
            Err(LogRecallError::InvalidCursor(_))
        ));
        assert!(matches!(
            adapter.recall_attributed(7, filter, &TokenId([1; 16])),
            Err(LogRecallError::InvalidCursor(_))
        ));
    }
    assert!(log.query_frames(FrameFilter::default()).unwrap().is_empty());
    let page = adapter
        .recall(
            7,
            LogRecallFilter::new(None, Some(i64::MAX as u64), Some(0), 1, None, None),
        )
        .unwrap();
    assert!(page.entries.is_empty());
    assert!(page.next_cursor.is_none());
}

#[test]
fn attributed_operations_write_one_invocation_each_with_canonical_token_column() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(15));
    let frame = insert(&log, 7, FrameKind::McpInvocation, "mcp", b"payload");
    let adapter = LogRecallAdapter::new(Arc::clone(&log));
    let token = TokenId(std::array::from_fn(|index| index as u8 + 1));
    adapter
        .recall_attributed(7, LogRecallFilter::default(), &token)
        .unwrap();
    assert_eq!(
        adapter
            .fetch_attributed(7, frame, &token)
            .unwrap()
            .payload_redacted,
        b"payload"
    );
    assert_eq!(
        adapter
            .fetch_attributed_bounded(7, frame, &token, 6)
            .unwrap(),
        BoundedFetch::PayloadTooLarge { payload_len: 7 }
    );
    assert!(matches!(
        adapter
            .fetch_attributed_bounded(7, frame, &token, 7)
            .unwrap(),
        BoundedFetch::Fetched(_)
    ));
    assert!(matches!(
        adapter.fetch_attributed_bounded(7, [0; 16], &token, 0),
        Err(LogRecallError::FrameNotFound { .. })
    ));
    let rows = log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter()
            .filter(|entry| entry.intent == "log.recall")
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|entry| entry.intent == "log.fetch")
            .count(),
        4
    );
    for entry in rows {
        let column = entry.capability_token.unwrap();
        assert_eq!(&column[..16], &token.0);
        assert_eq!(&column[16..], &[0; 16]);
    }
}

#[test]
fn maximum_metadata_page_over_megabyte_payloads_has_no_payload_projection() {
    // Virtual blobs exercise a full 1024-entry page without storing a GiB test
    // fixture. The production query's SELECT shape is pinned in the TL unit test.
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE VIEW transparency_log AS
         WITH RECURSIVE ids(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM ids WHERE n < 1024)
         SELECT randomblob(16) AS frame_id, n AS timestamp_ns, 7 AS spirit_pid,
                16 AS boot_nonce, 18 AS kind, 'mcp' AS intent,
                zeroblob(1048576) AS payload_redacted FROM ids;",
    )
    .unwrap();
    let reader = TransparencyLogAdapter::from_read_only_connection(conn);
    let metadata = reader
        .query_frame_metadata(FrameFilter {
            spirit_pid: Some(7),
            disclosable_kinds_only: true,
            limit: Some(1025),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(metadata.len(), 1024);
    assert!(metadata.iter().all(|entry| entry.payload_len == 1048576));
    let page = LogRecallAdapter::query_page(&reader, 7, None, LogRecallFilter::default()).unwrap();
    assert_eq!(page.entries.len(), 1024);
    assert!(page.entries.iter().all(|entry| entry.payload_available));
    assert!(page.next_cursor.is_none());
}

#[test]
fn boot_pid_timestamp_index_is_created_and_reopening_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("log.db");
    for boot in [17, 18] {
        let log = TransparencyLogAdapter::open(&path, boot).unwrap();
        assert_eq!(log.boot_nonce(), boot);
        let conn = Connection::open(&path).unwrap();
        let mut statement = conn
            .prepare("PRAGMA index_info('idx_tlog_boot_pid_ts')")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(2))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            columns,
            ["boot_nonce", "spirit_pid", "timestamp_ns", "frame_id"]
        );
    }
}

#[test]
fn raw_kind_allow_list_has_no_unknown_or_operator_only_discriminants() {
    for raw_kind in -1..=99 {
        assert_eq!(
            is_spirit_disclosable(raw_kind),
            (0..=6).contains(&raw_kind) || raw_kind == 18
        );
    }
}
