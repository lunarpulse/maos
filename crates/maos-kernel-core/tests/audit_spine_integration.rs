//! Audit spine integration test — end-to-end verification of the
//! Transparency Log + Approval Decision Log + Lifecycle Journal.
//!
//! Exercises:
//! - 100 sequential IAC frame inserts via TransparencyLogAdapter
//! - 20 approval decision inserts
//! - 50 lifecycle transitions via JournalAdapter
//! - Query-back and ordering verification
//! - Crash-recovery rehydration
//! - Concurrent writes

use maos_kernel_core::iac::{
    TransparencyLogAdapter, FrameKind, FrameFilter,
};
use maos_kernel_core::journal::JournalAdapter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i4::ApprovalDecision;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};

/// Helper: create a tempdir + journal for integration tests.
fn journal_temp() -> (JournalAdapter, tempfile::TempDir) {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");
    let journal = JournalAdapter::open(&path).unwrap();
    (journal, tmpdir)
}

/// End-to-end audit spine smoke test:
/// 100 frames + 20 approvals + 50 transitions, then query back.
#[test]
fn audit_spine_end_to_end() {
    let log = TransparencyLogAdapter::open_in_memory(0xC0FFEE);
    let (journal, _tmpdir) = journal_temp();

    // Insert 100 frame events
    for i in 0..100 {
        let _token = log.insert_frame_event(
            if i % 3 == 0 { FrameKind::TaskAssign } else if i % 3 == 1 { FrameKind::CapabilityInvocation } else { FrameKind::TelemetryEvent },
            (i % 10) as u32,
            if i % 5 == 0 { Some(&[0xAA_u8; 32]) } else { None },
            if i % 2 == 0 { "delegate" } else { "broadcast" },
            format!("payload-{}", i).as_bytes(),
            if i % 2 == 0 { FrameOrigin::HumanAuthored } else { FrameOrigin::SpiritAuto },
        );
    }

    // Insert 20 approval decisions
    for i in 0..20 {
        log.insert_approval_decision(ApprovalDecision {
            actor: format!("user-{}", i % 5),
            target: format!("spirit-{}", i),
            capability: "calendar.read".into(),
            intent: format!("morning-digest-{}", i),
            decision: i % 2 == 0,
            reasoning: if i % 3 == 0 { Some("routine approval".into()) } else { None },
        }).unwrap();
    }

    // Insert 50 lifecycle transitions
    let events = [
        LifecycleEvent::Load, LifecycleEvent::Start, LifecycleEvent::Pause,
        LifecycleEvent::Swap, LifecycleEvent::Unload, LifecycleEvent::Halt,
        LifecycleEvent::Migrate,
    ];
    for i in 0..50 {
        journal.append_transition(JournalEntry {
            timestamp: i as u64,
            lifecycle_event: events[i % events.len()],
            spirit_id: format!("spirit-{}", i % 10),
        });
    }

    // Verify frame count
    let frames = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(frames.len(), 100, "expected 100 frame entries");

    // Verify approval count
    let approvals = log.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 20, "expected 20 approval entries");

    // Verify journal recovery
    let recovered = journal.recover_in_flight();
    assert_eq!(recovered.len(), 10, "expected 10 unique spirits in journal");

    // Verify log-before-deliver ordering
    for window in frames.windows(2) {
        assert!(
            window[0].timestamp_ns <= window[1].timestamp_ns,
            "frames not ordered: {} > {}",
            window[0].timestamp_ns,
            window[1].timestamp_ns,
        );
    }

    // Verify filter works
    let filtered = log.query_frames(FrameFilter {
        spirit_pid: Some(5),
        ..Default::default()
    }).unwrap();
    assert!(filtered.iter().all(|f| f.spirit_pid == 5));
    assert_eq!(filtered.len(), 10, "spirit_pid=5 should have 10 entries");
}

/// Verify the journal's cold-restart rehydration path.
#[test]
fn journal_cold_restart_rehydration() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");

    // Boot 1: write entries
    {
        let journal = JournalAdapter::open(&path).unwrap();
        journal.append_transition(JournalEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
        });
        journal.append_transition(JournalEntry {
            timestamp: 2,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
        });
        journal.append_transition(JournalEntry {
            timestamp: 3,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-beta".into(),
        });
    }

    // Boot 2: rehydrate and verify
    let journal = JournalAdapter::open(&path).unwrap();
    let recovered = journal.recover_in_flight();
    assert_eq!(recovered.len(), 2);
    assert!(recovered
        .iter()
        .any(|(s, e)| s == "spirit-alpha" && *e == LifecycleEvent::Start));
    assert!(recovered
        .iter()
        .any(|(s, e)| s == "spirit-beta" && *e == LifecycleEvent::Load));

    // Can append after rehydration
    journal.append_transition(JournalEntry {
        timestamp: 4,
        lifecycle_event: LifecycleEvent::Unload,
        spirit_id: "spirit-alpha".into(),
    });
    let last = journal.last_event("spirit-alpha").unwrap();
    assert_eq!(last, LifecycleEvent::Unload);
}

/// Verify concurrent writes to the Transparency Log.
#[test]
fn concurrent_transparency_log_writes() {
    use std::sync::Arc;
    use std::thread;

    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x1));
    let mut handles = Vec::new();
    for tid in 0..4 {
        let log_clone = Arc::clone(&log);
        handles.push(thread::spawn(move || {
            for i in 0..25 {
                let _token = log_clone.insert_frame_event(
                    FrameKind::TaskAssign,
                    (tid * 25 + i) as u32,
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
    assert_eq!(entries.len(), 100, "expected 100 rows from 4 threads × 25 inserts");
}
