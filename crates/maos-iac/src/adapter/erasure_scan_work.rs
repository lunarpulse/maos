#![cfg(test)]

//! Test-only SQLite VM accounting: wall scheduling and fsync are not scan work.
use super::*;
use std::cell::RefCell;

thread_local! {
    static WINDOWS: RefCell<Option<Vec<(String, i32)>>> = const { RefCell::new(None) };
}

pub(super) fn record(statement: &rusqlite::Statement<'_>, sql: &str, boundary_steps: i32) {
    WINDOWS.with(|slot| {
        if let Some(windows) = slot.borrow_mut().as_mut() {
            let steps = statement
                .reset_status(rusqlite::StatementStatus::VmStep)
                .saturating_add(boundary_steps);
            windows.push((sql.to_owned(), steps));
        }
    });
}

#[test]
fn ac5_every_scan_has_bounded_sqlite_work_at_100k() {
    assert_bounded_scan_work(100_000, 1);
}

#[test]
fn ac5_every_scan_has_bounded_sqlite_work_at_1m_rowid_with_100k_rows() {
    assert_bounded_scan_work(100_000, 10);
}

fn assert_bounded_scan_work(count: i64, stride: i64) {
    // This companion isolates SQLite work, not disk/fsync scheduling. The
    // real door vectors separately retain their 1 KiB disk/WAL fixtures.
    let log = TransparencyLogAdapter::open_in_memory(2011);
    {
        let mut inner = log.inner.lock();
        let transaction = inner.conn.transaction().expect("seed transaction");
        {
            let mut insert = transaction.prepare("INSERT INTO transparency_log (rowid,frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin) VALUES (?1,?2,?1,7,2011,11,'scan.filler',?3,0)").expect("seed statement");
            let body = b"x".as_slice();
            for row in 1..=count {
                insert
                    .execute(rusqlite::params![
                        row * stride,
                        (row * stride).to_be_bytes().repeat(2),
                        body
                    ])
                    .expect("seed row");
            }
        }
        transaction.commit().expect("seed commit");
    }
    log.insert_frame_event(
        FrameKind::TaskAssign,
        7,
        None,
        "erased-source",
        b"seed",
        FrameOrigin::Kernel,
    );
    log.scrub_frame_body(
        log.last_frame_id(),
        FrameKind::TaskAssign as i64,
        "gdpr-forget",
    )
    .expect("real prior erasure");
    WINDOWS.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
    // All kind-11 rows expose a planner regression that an all-kind-1
    // latency fixture misses: idx_tlog_kind can rescan N rows per window.
    assert!(log
        .distillates_written_by(&[(2011, 99)])
        .expect("writer scan")
        .is_empty());
    let (owned, unattributed) = log
        .distillates_written_by_identity("no-such-owner")
        .expect("admission-qualified reader");
    assert!(owned.is_empty());
    assert_eq!(unattributed.len(), count as usize);
    assert!(log
        .principal_bearing_frames("scan-target@example.org")
        .expect("principal scan")
        .is_empty());
    assert!(log
        .distillates_citing(&std::collections::HashSet::new())
        .expect("lineage scan")
        .is_empty());
    let (_, _, _, _, malformed) = log
        .scrub_forget_frames("scan-target@example.org")
        .expect("late sweep");
    assert_eq!(malformed.len(), count as usize);
    assert!(log
        .outstanding_redistill_requests()
        .expect("recovery scan")
        .is_empty());
    let windows = WINDOWS.with(|slot| slot.borrow_mut().take().expect("enabled accounting"));
    let mut queries = std::collections::BTreeMap::<String, Vec<i32>>::new();
    for (sql, steps) in windows {
        queries.entry(sql).or_default().push(steps);
    }
    assert_eq!(
        queries.len(),
        8,
        "all old and admission-qualified forget predicates must be exercised"
    );
    let high_water = log.frame_high_water().expect("measured high-water");
    for (sql, windows) in queries {
        let max = *windows.iter().max().expect("nonempty scan");
        let total: i64 = windows.iter().map(|steps| i64::from(*steps)).sum();
        let ratio = f64::from(max) / total as f64;
        eprintln!("EC-13 work seeded_kind11_rows={count} high_water={high_water} windows={} max_steps={max} total_steps={total} ratio={ratio} sql={sql}",windows.len());
        assert!(
            ratio < 0.25,
            "one lock window consumed >=25% of scan work: {ratio}"
        );
        // Independent of the production page constant: a plant must not
        // enlarge its own acceptance budget. JSON predicates have ample room.
        assert!(
            max <= 4096 * 128,
            "unbounded scan window: {max} SQLite VM steps; {sql}"
        );
    }
}

#[test]
fn ac5_rowid_scans_restore_timestamp_and_frame_id_order() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    {
        let inner = log.inner.lock();
        for (id, timestamp) in [(3u128, 7), (1, 7), (2, 2)] {
            inner.conn.execute(
                "INSERT INTO transparency_log (frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin) VALUES (?1,?2,7,2011,11,'ordering',?3,0)",
                rusqlite::params![&id.to_be_bytes()[..], timestamp, br#"{"source_log_ref":[],"principal_id":"order@example.org"}"#.as_slice()],
            ).expect("ordered fixture");
            inner.conn.execute(
                "INSERT INTO transparency_log (frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin) VALUES (?1,?2,8,2011,11,'ordering',?3,0)",
                rusqlite::params![&(id + 3).to_be_bytes()[..], timestamp, b"x".as_slice()],
            ).expect("malformed fixture");
        }
    }
    for frames in [
        log.distillates_written_by(&[(2011, 7)])
            .expect("writer scan"),
        log.principal_bearing_frames("order@example.org")
            .expect("principal scan"),
    ] {
        assert_eq!(
            frames
                .iter()
                .map(|frame| u128::from_be_bytes(frame.frame_id))
                .collect::<Vec<_>>(),
            [2, 1, 3]
        );
    }
    let (_, _, _, _, malformed) = log
        .scrub_forget_frames("missing@example.org")
        .expect("malformed scan");
    assert_eq!(malformed, [5u128, 4, 6].map(|id| format!("{id:032x}")));
}

#[test]
fn ac5_opaque_outcome_payload_is_storable_but_bad_recovery_is_reported() {
    let log = TransparencyLogAdapter::open_in_memory(2011);
    log.insert_frame_event(
        FrameKind::TaskComplete,
        0,
        None,
        "distillate.redistill.pending",
        b"not JSON",
        FrameOrigin::Kernel,
    );
    let request = RedistillRequest {
        redacted_distillate_frame_id: format!("{:032x}", 1),
        writer_spirit_pid: 7,
        writer_boot_nonce: 2011u128.to_le_bytes(),
        surviving_source_log_ref: Vec::new(),
    };
    log.insert_redistill_request(&request).expect("request");
    let error = log
        .outstanding_redistill_requests()
        .expect_err("malformed outcome must remain an explicit read error");
    assert!(error.to_string().contains("malformed JSON"), "{error}");
}
