//! Integration test: LogRecallAdapter scope, pagination, and fetch.

use std::sync::Arc;

use maos_domain::log_recall::{LogRecallError, LogRecallFilter};
use maos_domain::ports::LogRecallPort;

use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

fn seed_frames(tl: &Arc<TransparencyLogAdapter>, pid: u32, count: usize) {
    for i in 0..count {
        let payload = format!("payload-{pid}-{i}");
        let _token = tl.insert_frame_event(
            FrameKind::TaskAssign,
            pid,
            None,
            "delegate",
            payload.as_bytes(),
            FrameOrigin::HumanAuthored,
        );
    }
}

#[test]
fn recall_emitter_scope_only_returns_own_frames() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE401));
    seed_frames(&tl, 10, 5);
    seed_frames(&tl, 20, 5);
    seed_frames(&tl, 30, 3);

    let adapter = LogRecallAdapter::new(tl);
    let page = adapter.recall(10, LogRecallFilter::default()).unwrap();

    assert_eq!(page.entries.len(), 5);
    assert!(page.entries.iter().all(|e| e.peer_spirit_pid == 10));
}

#[test]
fn recall_cursor_pagination() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE402));
    seed_frames(&tl, 10, 5);

    let adapter = LogRecallAdapter::new(tl);

    // Page 1: limit = 2
    let page1 = adapter
        .recall(10, LogRecallFilter::new(None, None, None, 2, None, None))
        .unwrap();
    assert_eq!(page1.entries.len(), 2);
    assert!(page1.next_cursor.is_some());

    // Page 2: continue with cursor
    let page2 = adapter
        .recall(
            10,
            LogRecallFilter::new(None, None, None, 2, page1.next_cursor, None),
        )
        .unwrap();
    assert_eq!(page2.entries.len(), 2);
    assert!(page2.next_cursor.is_some());

    // Page 3: final page (1 entry + no cursor)
    let page3 = adapter
        .recall(
            10,
            LogRecallFilter::new(None, None, None, 2, page2.next_cursor, None),
        )
        .unwrap();
    assert_eq!(page3.entries.len(), 1);
    assert!(page3.next_cursor.is_none());
}

#[test]
fn fetch_happy_path() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE403));
    seed_frames(&tl, 10, 1);

    let page = LogRecallAdapter::new(Arc::clone(&tl))
        .recall(10, LogRecallFilter::default())
        .unwrap();
    let frame_id = page.entries[0].frame_id;

    let adapter = LogRecallAdapter::new(tl);
    let resp = adapter.fetch(10, frame_id).unwrap();
    assert_eq!(resp.frame_id, frame_id);
    assert!(!resp.payload_redacted.is_empty());
}

#[test]
fn fetch_scope_violation() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE404));
    seed_frames(&tl, 10, 1);

    let page = LogRecallAdapter::new(Arc::clone(&tl))
        .recall(10, LogRecallFilter::default())
        .unwrap();
    let frame_id = page.entries[0].frame_id;

    let adapter = LogRecallAdapter::new(tl);
    let err = adapter.fetch(20, frame_id).unwrap_err();
    match err {
        LogRecallError::ScopeViolation {
            frame_id: fid,
            requested_pid,
            owner_pid,
        } => {
            assert_eq!(fid, frame_id);
            assert_eq!(requested_pid, 20);
            assert_eq!(owner_pid, 10);
        }
        _ => panic!("expected ScopeViolation, got: {err:?}"),
    }
}

#[test]
fn fetch_emits_capability_invocation_audit_row() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xE405));
    seed_frames(&tl, 10, 1);

    let page = LogRecallAdapter::new(Arc::clone(&tl))
        .recall(10, LogRecallFilter::default())
        .unwrap();
    let frame_id = page.entries[0].frame_id;

    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    adapter.fetch(10, frame_id).unwrap();

    let audit_filter = FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    };
    let audit_rows = tl.query_frames(audit_filter).unwrap();
    assert!(
        audit_rows.iter().any(|r| r.intent == "log.fetch"),
        "expected CapabilityInvocation row with intent log.fetch"
    );
}
