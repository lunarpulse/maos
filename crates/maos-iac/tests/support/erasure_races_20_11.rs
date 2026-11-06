#![cfg(test)]
//! Test-only interleaving points: actual SQLite appends and a separate-process hold replacement.
use super::*;
use std::cell::RefCell;

type WindowHook = Box<dyn FnMut(&TransparencyLogAdapter, &str, Option<&str>)>;
thread_local! {
    static AFTER_WINDOW: RefCell<Option<WindowHook>> = const { RefCell::new(None) };
    static BEFORE_HOLD_DELETE: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
    static BEFORE_BODY_MUTATION: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

pub(super) fn after_window(log: &TransparencyLogAdapter, sql: &str, principal: Option<&str>) {
    AFTER_WINDOW.with(|slot| {
        if let Some(callback) = slot.borrow_mut().as_mut() {
            callback(log, sql, principal);
        }
    });
}

pub(super) fn before_hold_delete() {
    let callback = BEFORE_HOLD_DELETE.with(|slot| slot.borrow_mut().take());
    if let Some(callback) = callback {
        callback();
    }
}

pub(super) fn before_body_mutation() {
    let callback = BEFORE_BODY_MUTATION.with(|slot| slot.borrow_mut().take());
    if let Some(callback) = callback {
        callback();
    }
}

#[test]
fn review_p1_append_after_the_final_partial_page_is_not_left_unscrubbed() {
    for count in [10, 4106] {
        assert_final_partial_page(count);
    }
}

fn assert_final_partial_page(count: usize) {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    for _ in 0..count {
        log.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            "unrelated",
            b"unrelated",
            FrameOrigin::Kernel,
        );
    }
    let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
    let appended = std::sync::Arc::clone(&captured);
    let mut inserted = false;
    let mut windows = 0;
    AFTER_WINDOW.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |log, sql, principal| {
            if principal == Some("late-page@example.org") && sql.contains("instr(payload_redacted")
            {
                windows += 1;
                if inserted || windows != count.div_ceil(4096) {
                    return;
                }
                inserted = true;
                log.insert_frame_event(
                    FrameKind::McpInvocation,
                    7,
                    None,
                    "late.subject.frame",
                    br#"{"principal_id":"late-page@example.org","canary":"must be scrubbed"}"#,
                    FrameOrigin::SpiritAuto,
                );
                *appended.lock().expect("late id") = Some(log.last_frame_id());
            }
        }))
    });
    let result = log.scrub_forget_frames("late-page@example.org");
    AFTER_WINDOW.with(|slot| *slot.borrow_mut() = None);
    let result = result.expect("a finite single late append must complete");
    let id = captured
        .lock()
        .expect("late id")
        .expect("partial-page hook ran");
    assert_eq!(
        result.1,
        vec![format!("{:032x}", u128::from_be_bytes(id))],
        "exact late id, once only"
    );
    let payload: Vec<u8> = log
        .inner
        .lock()
        .conn
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
            [id.as_slice()],
            |row| row.get(0),
        )
        .expect("late frame");
    let value: serde_json::Value =
        serde_json::from_slice(&payload).unwrap_or_else(|error| panic!("late frame JSON: {error}"));
    assert_eq!(
        value["redacted"], true,
        "matching append before completion survived the final partial page: {value}"
    );
}

#[test]
fn review_f13_scan_snapshot_does_not_chase_a_sustained_writer() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    let append_page = |log: &TransparencyLogAdapter, first: i64| {
        let mut inner = log.inner.lock();
        let transaction = inner.conn.transaction().expect("writer transaction");
        let mut insert = transaction.prepare(
            "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
             VALUES(?1,?2,7,2011,1,'writer',X'78',0)",
        ).expect("writer append");
        for ordinal in first..first + 4096 {
            insert
                .execute(rusqlite::params![ordinal.to_be_bytes().repeat(2), ordinal])
                .expect("append");
        }
        drop(insert);
        transaction.commit().expect("writer commit");
    };
    append_page(&log, 1);
    let windows = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = std::sync::Arc::clone(&windows);
    AFTER_WINDOW.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |log, _sql, _principal| {
            let window = observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if window < 3 {
                append_page(log, 4097 + window as i64 * 4096);
            }
        }))
    });
    let result = log.distillates_written_by(&[(2011, 99)]);
    AFTER_WINDOW.with(|slot| *slot.borrow_mut() = None);
    assert!(result.expect("finite initial scan").is_empty());
    assert_eq!(
        windows.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the initial scan followed appends beyond its one-page entry snapshot"
    );
}

#[test]
fn review_f7_cross_process_replaced_hold_is_not_released_by_a_stale_observation() {
    let directory = tempfile::tempdir().expect("hold fixture");
    let path = directory.path().join("global.sqlite");
    let log =
        TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1).expect("global TL");
    log.place_legal_hold("hold-aba@example.org", "same reason", Some("same case"), 42)
        .expect("old hold");
    assert!(log
        .is_under_legal_hold("hold-aba@example.org")
        .expect("observe old hold"));
    BEFORE_HOLD_DELETE.with(|slot| *slot.borrow_mut() = Some(Box::new(move || {
        let output = std::process::Command::new("python3").args(["-c", r#"
import sqlite3, sys
with sqlite3.connect(sys.argv[1]) as connection:
    connection.execute("DELETE FROM legal_holds WHERE principal_id=?", ("hold-aba@example.org",))
    connection.execute("INSERT INTO legal_holds(principal_id,reason,case_ref,requested_at_ns) VALUES(?,?,?,?)", ("hold-aba@example.org", "same reason", "same case", 42))
"#]).arg(path).output().expect("separate-process SQLite hold writer");
        assert!(output.status.success(), "hold replacement failed: {output:?}");
    })));
    assert!(!log.release_legal_hold("hold-aba@example.org").expect("conditional release"),
        "a release based on the old observation deleted a newly placed hold with identical public fields");
    assert!(log
        .is_under_legal_hold("hold-aba@example.org")
        .expect("new hold survives"));
}

#[test]
fn review_f8_cross_process_hold_and_body_mutation_have_one_sqlite_order() {
    use maos_domain::ports::DistillationPort as _;
    let directory = tempfile::tempdir().expect("mutation fixture");
    let path = directory.path().join("global.sqlite");
    let log = std::sync::Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1).expect("TL"),
    );
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "held.source",
        br#"{"principal_id":"mutation-hold@example.org"}"#,
        FrameOrigin::HumanAuthored,
    );
    let source = log.last_frame_id();
    let digest = crate::adapter::distillate::DistillateWriter::new(
        std::sync::Arc::clone(&log),
        std::sync::Arc::new(0_u64),
    )
    .write_distillate(
        7,
        maos_domain::distillation::DistillationRequest::new(
            vec![source],
            1,
            maos_domain::distillation::DigestPayload::Text("held derivation".into()),
            None,
        )
        .expect("canonical request"),
    )
    .expect("canonical digest")
    .digest_frame_id;
    let original: Vec<u8> = log
        .inner
        .lock()
        .conn
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
            [digest.as_slice()],
            |row| row.get(0),
        )
        .expect("original body");
    let placed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = std::sync::Arc::clone(&placed);
    BEFORE_BODY_MUTATION.with(|slot| *slot.borrow_mut() = Some(Box::new(move || {
        let output = std::process::Command::new("python3").args(["-c", r#"
import sqlite3, sys
try:
    with sqlite3.connect(sys.argv[1], timeout=0) as connection:
        connection.execute("INSERT INTO legal_holds(principal_id,reason,case_ref,requested_at_ns) VALUES(?,?,?,?)", ("mutation-hold@example.org", "litigation", "F8", 42))
except sqlite3.OperationalError as error:
    if "locked" not in str(error):
        raise
    sys.exit(75)
"#]).arg(path).output().expect("independent hold writer");
        assert!(matches!(output.status.code(), Some(0 | 75)), "unexpected hold writer error: {output:?}");
        observed.store(output.status.success(), std::sync::atomic::Ordering::SeqCst);
    })));
    let changed = log
        .scrub_distillate_unless_held(digest, "gdpr-forget")
        .expect("ordered mutation");
    let current: Vec<u8> = log
        .inner
        .lock()
        .conn
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
            [digest.as_slice()],
            |row| row.get(0),
        )
        .expect("current body");
    if placed.load(std::sync::atomic::Ordering::SeqCst) {
        assert!(
            !changed,
            "a hold committed between recheck and mutation, but its derivation was erased"
        );
        assert_eq!(
            current, original,
            "committed hold lost the race without an atomic writer transaction"
        );
    } else {
        assert!(changed, "the erasure writer won the SQLite order");
        assert!(TransparencyLogAdapter::is_kernel_tombstone(&current));
    }
}

#[test]
fn review_f13_sustained_matching_writer_returns_pending_at_the_fixed_pass_cap() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "initial",
        b"unrelated",
        FrameOrigin::Kernel,
    );
    let windows = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = std::sync::Arc::clone(&windows);
    AFTER_WINDOW.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |log, sql, principal| {
            if principal == Some("sustained@example.org") && sql.contains("instr(payload_redacted")
            {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                log.insert_frame_event(
                    FrameKind::McpInvocation,
                    7,
                    None,
                    "writer.matching",
                    b"sustained@example.org",
                    FrameOrigin::SpiritAuto,
                );
            }
        }))
    });
    let result = log.scrub_forget_frames("sustained@example.org");
    AFTER_WINDOW.with(|slot| *slot.borrow_mut() = None);
    assert!(
        matches!(result, Err(AuditError::ErasurePending)),
        "a sustained matching writer was falsely complete: {result:?}"
    );
    assert_eq!(
        windows.load(std::sync::atomic::Ordering::SeqCst),
        8,
        "the fixed pass cap is independent of writer speed"
    );
}

#[test]
fn review_marker_orders_independent_writer_commits_not_handler_return_time() {
    let directory = tempfile::tempdir().expect("writer fixture");
    let path = directory.path().join("global.sqlite");
    let log = TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1).expect("TL");
    log.insert_frame_event(
        FrameKind::McpInvocation,
        7,
        None,
        "first.subject",
        b"marker-subject@example.org",
        FrameOrigin::SpiritAuto,
    );
    let first = log.last_frame_id();
    let mut sweep = ErasureSweep::new(vec!["marker-subject@example.org".into()], None);
    log.sweep_erasure(&mut sweep).expect("finite snapshot");
    let append = |byte: u8| {
        let output=std::process::Command::new("python3").args(["-c",r#"
import sqlite3,sys
with sqlite3.connect(sys.argv[1]) as connection:
    connection.execute("INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin) VALUES(?,1,88,2,?,'independent.writer',?,1)",(bytes([int(sys.argv[2])])*16,int(sys.argv[3]),b'marker-subject@example.org'))
"#]).arg(&path).arg(byte.to_string()).arg((FrameKind::McpInvocation as i64).to_string()).output().expect("independent process");
        assert!(output.status.success(), "{output:?}");
    };
    append(221);
    assert!(
        !log.certify_erasure(&mut sweep)
            .expect("fresh high-water recheck"),
        "a pre-marker writer escaped the tail"
    );
    log.sweep_erasure(&mut sweep).expect("matching tail");
    assert!(log.certify_erasure(&mut sweep).expect("durable marker"));
    let marker = sweep.completion.expect("marker");
    for id in [first, [221; 16]] {
        let inner = log.inner.lock();
        let (rowid, body): (i64, Vec<u8>) = inner
            .conn
            .query_row(
                "SELECT rowid,payload_redacted FROM transparency_log WHERE frame_id=?1",
                [id.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("frame");
        assert!(rowid as u64 <= marker.through_rowid);
        assert!(TransparencyLogAdapter::is_kernel_tombstone(&body));
    }
    append(222);
    let inner = log.inner.lock();
    let (rowid, body): (i64, Vec<u8>) = inner
        .conn
        .query_row(
            "SELECT rowid,payload_redacted FROM transparency_log WHERE frame_id=?1",
            [[222; 16].as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("post-marker commit");
    assert!(rowid as u64 > marker.through_rowid);
    assert_eq!(
        body, b"marker-subject@example.org",
        "post-marker data is not claimed erased"
    );
}

#[test]
fn review_legacy_direct_parent_closure_reads_each_snapshot_window_once() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "root",
        b"source",
        FrameOrigin::HumanAuthored,
    );
    let root = format!("{:032x}", u128::from_be_bytes(log.last_frame_id()));
    {
        let inner = log.inner.lock();
        // Re-imported legacy parents need not be in rowid/topological order.
        for ordinal in (1_u128..=64).rev() {
            let parent = if ordinal == 1 {
                root.clone()
            } else {
                format!("{:032x}", ordinal - 1)
            };
            let body = serde_json::to_vec(
                &serde_json::json!({"source_log_ref":[],"direct_source_log_ref":[parent]}),
            )
            .expect("legacy receipt");
            inner.conn.execute(
                "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
                 VALUES(?1,?2,7,2011,11,'legacy.direct',?3,1)",
                rusqlite::params![ordinal.to_be_bytes().as_slice(),ordinal as i64,body],
            ).expect("legacy import");
        }
    }
    let windows = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = std::sync::Arc::clone(&windows);
    AFTER_WINDOW.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |_log, sql, principal| {
            if principal.is_none() && sql.starts_with("SELECT frame_id,kind,spirit_pid,boot_nonce")
            {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }))
    });
    let closure = log.distillates_citing(&std::collections::HashSet::from([root]));
    AFTER_WINDOW.with(|slot| *slot.borrow_mut() = None);
    assert_eq!(closure.expect("complete legacy closure").len(), 64);
    assert_eq!(
        windows.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "closure repeatedly rescanned the snapshot for every legacy parent"
    );
}

#[test]
fn review_legacy_receipt_order_is_timestamp_then_frame_id_after_closure() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "source",
        b"unrelated source",
        FrameOrigin::HumanAuthored,
    );
    let source = format!("{:032x}", u128::from_be_bytes(log.last_frame_id()));
    {
        let inner = log.inner.lock();
        for (id, time, parent, principal) in [
            (1_u128, 20_i64, source, Some("order@example.org")),
            (2, 10, format!("{:032x}", 1_u128), None),
        ] {
            let body = serde_json::to_vec(
                &serde_json::json!({"source_log_ref":[parent],"principal_id":principal}),
            )
            .expect("receipt");
            inner.conn.execute(
                "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
                 VALUES(?1,?2,7,2011,11,'legacy.order',?3,1)",
                rusqlite::params![id.to_be_bytes().as_slice(),time,body],
            ).expect("imported digest");
        }
    }
    let result = log
        .scrub_forget_frames("order@example.org")
        .expect("complete scrub");
    assert_eq!(
        result.0,
        vec![format!("{:032x}", 2_u128), format!("{:032x}", 1_u128)],
        "receipt reordered direct and derived frames after clock rollback"
    );
}

#[test]
fn review_non_poisoning_schema_panic_rolls_back_the_open_transaction() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "before",
        b"backed row",
        FrameOrigin::Kernel,
    );
    let before = log.last_frame_id();
    log.inner
        .lock()
        .conn
        .execute_batch(
            "CREATE TRIGGER reject_schema BEFORE INSERT ON transparency_log
         WHEN NEW.intent='governance:schema-lifecycle'
         BEGIN SELECT RAISE(ABORT,'real schema frame write fault'); END;",
        )
        .expect("real SQLite write fault");
    let entry = maos_domain::governance::SchemaRegistryEntry {
        schema_id: "review-unwind".into(),
        version: 1,
        effective_at_ns: 1,
        supersedes_hash: None,
        ratified_by: "ADR-044".into(),
        recorded_at_ns: 2,
        schema_content_hash: "review-content-hash".into(),
    };
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = log.register_schema_lifecycle(&entry);
    }));
    assert!(
        failure.is_err(),
        "I2 must still reject the actual failed write"
    );
    {
        let inner = log.inner.lock();
        assert!(
            inner.conn.is_autocommit(),
            "an I2 unwind left a transaction open behind the non-poisoning mutex"
        );
        let count: u64 = inner
            .conn
            .query_row(
                "SELECT count(*) FROM schema_lifecycle_registry WHERE schema_id='review-unwind'",
                [],
                |row| row.get(0),
            )
            .expect("rolled-back registry");
        assert_eq!(
            count, 0,
            "the failed atomic frame left an uncommitted registry row"
        );
        inner
            .conn
            .execute_batch("DROP TRIGGER reject_schema")
            .expect("remove real fault");
    }
    assert_eq!(
        log.last_frame_id(),
        before,
        "last_frame_id must remain backed by a committed row"
    );
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "after",
        b"backed recovery",
        FrameOrigin::Kernel,
    );
    assert_eq!(
        log.query_frames(FrameFilter::default())
            .expect("recovered log")
            .len(),
        2
    );
}

#[test]
fn review_non_poisoning_append_panic_keeps_last_frame_id_backed() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "before",
        b"backed row",
        FrameOrigin::Kernel,
    );
    let before = log.last_frame_id();
    log.inner
        .lock()
        .conn
        .execute_batch(
            "CREATE TRIGGER reject_append BEFORE INSERT ON transparency_log
         WHEN NEW.intent='rejected.write'
         BEGIN SELECT RAISE(ABORT,'real append write fault'); END;",
        )
        .expect("real SQLite write fault");
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        log.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            "rejected.write",
            b"must not deliver",
            FrameOrigin::Kernel,
        );
    }));
    assert!(
        failure.is_err(),
        "I2 must still reject the actual failed write"
    );
    assert_eq!(
        log.last_frame_id(),
        before,
        "a failed append advertised a frame which never committed"
    );
    log.inner
        .lock()
        .conn
        .execute_batch("DROP TRIGGER reject_append")
        .expect("remove real fault");
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "after",
        b"backed recovery",
        FrameOrigin::Kernel,
    );
    assert_eq!(
        log.query_frames(FrameFilter::default())
            .expect("recovered log")
            .len(),
        2
    );
}

#[test]
fn review_non_poisoning_kernel_append_panic_keeps_last_frame_id_backed() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "before",
        b"backed row",
        FrameOrigin::Kernel,
    );
    let before = log.last_frame_id();
    log.inner
        .lock()
        .conn
        .execute_batch(
            "CREATE TRIGGER reject_kernel BEFORE INSERT ON transparency_log
         WHEN NEW.intent='rejected.kernel'
         BEGIN SELECT RAISE(ABORT,'real kernel write fault'); END;",
        )
        .expect("real SQLite write fault");
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        log.insert_kernel_event_returning_id(
            7,
            FrameKind::TaskAssign,
            None,
            "rejected.kernel",
            b"must not deliver",
        );
    }));
    assert!(
        failure.is_err(),
        "I2 must still reject the actual failed write"
    );
    assert_eq!(
        log.last_frame_id(),
        before,
        "a failed kernel append advertised a frame which never committed"
    );
    log.inner
        .lock()
        .conn
        .execute_batch("DROP TRIGGER reject_kernel")
        .expect("remove real fault");
    log.insert_kernel_event_returning_id(
        7,
        FrameKind::TaskAssign,
        None,
        "after",
        b"backed recovery",
    );
    assert_eq!(
        log.query_frames(FrameFilter::default())
            .expect("recovered log")
            .len(),
        2
    );
}

#[test]
fn review_cached_frame_insert_reuses_the_indexed_program() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    for _ in 0..2 {
        log.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            "cached.insert",
            b"ordinary routed-row shape",
            FrameOrigin::Kernel,
        );
    }
    let inner = log.inner.lock();
    let statement = inner
        .conn
        .prepare_cached(FRAME_ROW_INSERT_SQL)
        .expect("the exact production insert program");
    assert_eq!(
        statement.get_status(rusqlite::StatementStatus::Run),
        2,
        "ordinary appends recompiled the indexed INSERT instead of reusing it"
    );
}

#[test]
fn review_descending_correlation_query_selects_the_latest_report() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    let mut ids = [[0_u8; 16]; 2];
    for id in &mut ids {
        log.insert_frame_event_with_correlation(
            FrameKind::TelemetryEvent,
            0,
            None,
            "completed.report",
            "operator.forget.fixture:completed",
            b"receipt metadata",
            FrameOrigin::Kernel,
        );
        *id = log.last_frame_id();
    }
    for (id, timestamp) in ids.iter().zip([10_i64, 20]) {
        log.inner
            .lock()
            .conn
            .execute(
                "UPDATE transparency_log SET timestamp_ns=?2 WHERE frame_id=?1",
                rusqlite::params![id.as_slice(), timestamp],
            )
            .expect("explicitly distinct receipt clocks");
    }
    let latest = log
        .query_frames(FrameFilter {
            correlation_id: Some("completed.report".into()),
            descending: true,
            limit: Some(1),
            ..Default::default()
        })
        .expect("indexed completed-report lookup");
    assert_eq!(
        latest[0].frame_id, ids[1],
        "DESC applied only to the frame-id tie breaker, selecting the oldest report"
    );
}
