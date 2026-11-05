use super::*;

#[test]
fn recall_metadata_query_projects_only_payload_length_before_limit() {
    let (sql, params) = TransparencyLogAdapter::frame_query(
        FrameFilter {
            boot_nonce: Some(12),
            spirit_pid: Some(7),
            disclosable_kinds_only: true,
            intent: Some("wanted".into()),
            limit: Some(1025),
            ..Default::default()
        },
        FRAME_METADATA_PROJECTION,
    );
    let (projection, predicates) = sql.split_once(" FROM ").unwrap();
    assert_eq!(projection.matches("payload_redacted").count(), 1);
    assert!(projection.ends_with("length(payload_redacted)"));
    assert!(!projection.contains(", payload_redacted"));
    assert!(predicates.contains("boot_nonce = ?"));
    assert!(predicates.contains("spirit_pid = ?"));
    assert!(predicates.contains("kind IN (0,1,2,3,4,5,6,18)"));
    assert!(predicates.contains("intent = ?"));
    assert!(predicates.ends_with("ORDER BY timestamp_ns, frame_id ASC LIMIT ?"));
    assert_eq!(params.len(), 4);
}

#[test]
fn open_in_memory_succeeds() {
    let log = TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF);
    assert!(log.inner.try_lock().is_some());
}

#[test]
fn insert_frame_event_creates_one_row() {
    let log = TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF);
    let _token = log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "delegate",
        b"test payload",
        FrameOrigin::HumanAuthored,
    );
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].spirit_pid, 7);
    assert_eq!(entries[0].kind, FrameKind::TaskAssign);
    assert_eq!(entries[0].intent, "delegate");
}

#[test]
fn cross_team_correlation_requires_both_physical_logs() {
    let security = maos_domain::team::TeamId::new("security").unwrap();
    let support = maos_domain::team::TeamId::new("support").unwrap();
    let security_log = TransparencyLogAdapter::open_in_memory(1);
    let support_log = TransparencyLogAdapter::open_in_memory(2);
    let correlation_id = "cross-team-action-01";

    for (log, pid, intent) in [
        (&security_log, 7, "cross-team-send"),
        (&support_log, 8, "cross-team-receive"),
    ] {
        let _ = log.insert_frame_event_with_correlation(
            FrameKind::CapabilityInvocation,
            pid,
            None,
            correlation_id,
            intent,
            b"redacted",
            FrameOrigin::Kernel,
        );
    }

    let one_path = security_log
        .query_frames(FrameFilter {
            correlation_id: Some(correlation_id.to_owned()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        one_path.len(),
        1,
        "a one-path reader cannot reconcile both team records"
    );

    let reconciled = reconcile_correlated_frames(
        &[(&security, &security_log), (&support, &support_log)],
        correlation_id,
    )
    .unwrap();
    assert_eq!(reconciled.len(), 2);
    assert_eq!(
        reconciled[0].entry.correlation_id.as_deref(),
        Some(correlation_id)
    );
    assert_eq!(
        reconciled[1].entry.correlation_id.as_deref(),
        Some(correlation_id)
    );
    assert_ne!(reconciled[0].team_id, reconciled[1].team_id);
}

#[test]
fn correlation_column_migrates_an_existing_transparency_log() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                from_spirit_id TEXT NOT NULL DEFAULT '',
                to_spirit_id TEXT NOT NULL DEFAULT '',
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );",
    )
    .unwrap();
    drop(conn);

    let log = TransparencyLogAdapter::open(&path, 3).unwrap();
    let _ = log.insert_frame_event_with_correlation(
        FrameKind::CapabilityInvocation,
        9,
        None,
        "migrated-correlation",
        "migrated-action",
        b"redacted",
        FrameOrigin::Kernel,
    );
    let entries = log
        .query_frames(FrameFilter {
            correlation_id: Some("migrated-correlation".to_owned()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].correlation_id.as_deref(),
        Some("migrated-correlation")
    );
}

#[test]
fn team_shard_keeps_legal_holds_in_global_database() {
    let dir = tempfile::TempDir::new().unwrap();
    let team_path = dir.path().join("teams/security/transparency.sqlite");
    let global_path = dir.path().join("transparency.sqlite");
    std::fs::create_dir_all(team_path.parent().unwrap()).unwrap();

    let log = TransparencyLogAdapter::open(&team_path, 4).unwrap();
    log.attach_global_legal_holds(&global_path).unwrap();
    log.place_legal_hold("principal-1", "legal-hold", Some("case-1"), 10)
        .unwrap();

    let team = rusqlite::Connection::open(&team_path).unwrap();
    let team_tables: i64 = team
        .query_row(
            "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'legal_holds'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        team_tables, 0,
        "legal_holds must not move into the team shard"
    );

    let global = rusqlite::Connection::open(&global_path).unwrap();
    let global_holds: i64 = global
        .query_row("SELECT count(*) FROM legal_holds", [], |row| row.get(0))
        .unwrap();
    assert_eq!(global_holds, 1);
}

#[test]
fn insert_frame_with_capability_token() {
    let log = TransparencyLogAdapter::open_in_memory(0xAAAA_BBBB);
    let token_bytes: [u8; 32] = [0xAA; 32];
    let _token = log.insert_frame_event(
        FrameKind::CapabilityInvocation,
        42,
        Some(&token_bytes),
        "file.read",
        b"/tmp/data",
        FrameOrigin::SpiritAuto,
    );
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].capability_token, Some([0xAA; 32]));
}

#[test]
fn hundred_sequential_inserts_are_ordered() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    for i in 0..100 {
        let _token = log.insert_frame_event(
            FrameKind::TaskAssign,
            i as u32,
            None,
            "seq-test",
            b"payload",
            FrameOrigin::HumanAuthored,
        );
    }
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(entries.len(), 100);
    // Verify timestamps are non-decreasing
    for window in entries.windows(2) {
        assert!(
            window[0].timestamp_ns <= window[1].timestamp_ns,
            "timestamps not ordered: {} > {}",
            window[0].timestamp_ns,
            window[1].timestamp_ns,
        );
    }
}

#[test]
fn insertion_cursor_does_not_skip_a_later_row_after_clock_rollback() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    let _ = log.insert_frame_event(
        FrameKind::CliSubprocessOutput,
        1,
        None,
        "first",
        b"{}",
        FrameOrigin::Kernel,
    );
    let first = log.query_frames(FrameFilter::default()).unwrap().remove(0);
    let _ = log.insert_frame_event(
        FrameKind::CliSubprocessOutput,
        2,
        None,
        "second",
        b"{}",
        FrameOrigin::Kernel,
    );
    log.inner
        .lock()
        .conn
        .execute(
            "UPDATE transparency_log SET timestamp_ns = 0 WHERE spirit_pid = 2",
            [],
        )
        .unwrap();

    let rows = log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            cursor_insertion_id: Some(first.insertion_id),
            order_by_insertion: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.spirit_pid).collect::<Vec<_>>(),
        vec![2],
        "a later insert remains visible even when its wall clock moved backward"
    );
}

#[test]
fn query_filter_by_spirit_pid() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    for pid in [7u32, 7, 7, 42, 99] {
        let _token = log.insert_frame_event(
            FrameKind::TaskAssign,
            pid,
            None,
            "filter-test",
            b"payload",
            FrameOrigin::HumanAuthored,
        );
    }
    let entries = log
        .query_frames(FrameFilter {
            spirit_pid: Some(7),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.spirit_pid == 7));
}

#[test]
fn query_filter_by_since_ns_excludes_older_frames() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    let _t1 = log.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "old",
        b"old-payload",
        FrameOrigin::SpiritAuto,
    );
    // Small sleep to ensure timestamps differ (wall-clock granularity).
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _t2 = log.insert_frame_event(
        FrameKind::TaskComplete,
        1,
        None,
        "new",
        b"new-payload",
        FrameOrigin::SpiritAuto,
    );

    let all = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(all.len(), 2);

    // Filter with since_ns after the first frame's timestamp — only the second should match.
    let since = all[0].timestamp_ns + 1;
    let filtered = log
        .query_frames(FrameFilter {
            since_ns: Some(since),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].intent, "new");
}

#[test]
fn concurrent_inserts_from_threads() {
    use std::sync::Arc;
    use std::thread;

    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x1));
    let mut handles = Vec::new();
    for tid in 0..4 {
        let log_clone = Arc::clone(&log);
        handles.push(thread::spawn(move || {
            for i in 0..10 {
                let _token = log_clone.insert_frame_event(
                    FrameKind::TaskAssign,
                    (tid * 10 + i) as u32,
                    None,
                    "concurrent-test",
                    b"payload",
                    FrameOrigin::SpiritAuto,
                );
            }
        }));
    }
    for h in handles {
        h.join().expect("thread panicked");
    }
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(
        entries.len(),
        40,
        "expected 40 rows from 4 threads × 10 inserts"
    );
}

#[test]
fn insert_approval_decision_creates_row() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    log.insert_approval_decision(ApprovalDecision {
        actor: "user-1".into(),
        target: "spirit-butler".into(),
        capability: "calendar.read".into(),
        intent: "morning-digest".into(),
        decision: true,
        reasoning: Some("user grants calendar read for digest spirit".into()),
    })
    .unwrap();

    let approvals = log.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].actor, "user-1");
    assert_eq!(approvals[0].decision, true);
    assert_eq!(
        approvals[0].reasoning.as_deref(),
        Some("user grants calendar read for digest spirit")
    );
}

#[test]
fn approval_log_is_distinct_table() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);

    // Insert one row into each table
    let _token = log.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        None,
        "schema-test",
        b"payload",
        FrameOrigin::HumanAuthored,
    );
    log.insert_approval_decision(ApprovalDecision {
        actor: "actor".into(),
        target: "target".into(),
        capability: "cap".into(),
        intent: "intent".into(),
        decision: true,
        reasoning: None,
    })
    .unwrap();

    let inner = log.inner.lock();
    let conn = &inner.conn;

    // 1. Verify two audit tables exist (sqlite_sequence is an internal table from AUTOINCREMENT)
    let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(
        tables,
        vec![
            "approval_decision_log",
            "legal_holds",
            // Story 9.3b (commit 3041fec) added the schema-lifecycle registry
            // table at open; this exhaustive list was left stale until the
            // Story 9.4b AC-6 regression caught it.
            "schema_lifecycle_registry",
            "transparency_log",
            // Story 17-8 — participant recall side tables.
            "transparency_log_consent",
            "transparency_log_consent_ruptures",
            "transparency_log_recipients",
            "transparency_log_retractions"
        ],
        "expected exactly eight tables"
    );

    // 2. No foreign keys on transparency_log
    let mut stmt = conn
        .prepare("PRAGMA foreign_key_list(transparency_log)")
        .unwrap();
    let fk_tlog: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(fk_tlog.is_empty(), "transparency_log has foreign keys");

    // 3. No foreign keys on approval_decision_log
    let mut stmt = conn
        .prepare("PRAGMA foreign_key_list(approval_decision_log)")
        .unwrap();
    let fk_adl: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(fk_adl.is_empty(), "approval_decision_log has foreign keys");

    // 4. Each table has exactly 1 row
    let tlog_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transparency_log", [], |row| {
            row.get(0)
        })
        .unwrap();
    let adl_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(tlog_count, 1);
    assert_eq!(adl_count, 1);

    // 5. Deleting from one table does not affect the other (test-only DELETE)
    conn.execute("DELETE FROM transparency_log", []).unwrap();
    let adl_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM approval_decision_log", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        adl_after, 1,
        "approval row affected by transparency_log DELETE"
    );
}

#[test]
#[should_panic(expected = "MAOS kernel panic — Transparency Log write failed")]
fn insert_frame_panics_on_write_failure() {
    // Create an in-memory adapter, then corrupt the connection
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    // Drop the inner connection by replacing it with a broken one
    {
        let mut inner = log.inner.lock();
        // Close the connection to force failure on next write
        let old_conn = std::mem::replace(&mut inner.conn, Connection::open_in_memory().unwrap());
        drop(old_conn);
        // The new connection doesn't have the schema, so INSERT will fail
        // Actually let's just force a failure by dropping and not recreating
    }
    // Try to insert — should panic because the new connection has no table
    let _ = log.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        None,
        "panic-test",
        b"payload",
        FrameOrigin::HumanAuthored,
    );
}

#[test]
fn iac_bus_port_enqueue_writes_and_routes() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    let _token = log.enqueue_frame(b"test-frame-data", FrameOrigin::HumanAuthored);
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(entries.len(), 1, "enqueue should write one row");
    let mailbox_frames = log.mailbox().drain_pending();
    assert_eq!(mailbox_frames.len(), 1, "enqueue should route to mailbox");
}

#[test]
fn iac_bus_port_broadcast_writes_and_routes() {
    let log = TransparencyLogAdapter::open_in_memory(0x1);
    let _token = log.broadcast_frame(b"broadcast-data", FrameOrigin::SpiritAuto);
    let entries = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, FrameKind::TelemetryEvent);
}

// ---- Story 5.3 — FrameKind variant tests ----

#[test]
fn frame_kind_task_stalled_from_i64() {
    assert_eq!(FrameKind::from_i64(15), Some(FrameKind::TaskStalled));
}

#[test]
fn frame_kind_silent_failure_suspect_from_i64() {
    assert_eq!(
        FrameKind::from_i64(16),
        Some(FrameKind::SilentFailureSuspect)
    );
}

#[test]
fn frame_kind_non_exhaustive_match() {
    let kind = FrameKind::TaskStalled;
    let _name = match kind {
        FrameKind::TaskStalled => "stalled",
        FrameKind::SilentFailureSuspect => "silent_failure",
        // All other variants — the #[non_exhaustive] on the enum means
        // downstream crates need a wildcard, but within the defining crate
        // we can match exhaustively.
        _ => "other",
    };
}

#[test]
fn frame_kind_spirit_revoked_from_i64() {
    assert_eq!(FrameKind::from_i64(17), Some(FrameKind::SpiritRevoked));
}

#[test]
fn frame_kind_spirit_revoked_discriminant() {
    assert_eq!(FrameKind::SpiritRevoked as i64, 17);
}
