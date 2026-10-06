//! Story 17-3c — trusted-caller `spirit_pid` journaling and the
//! unregister/reload lifecycle of a Spirit's mailbox, including an
//! unregister that races a delivery blocked on a full recipient channel.

use std::sync::Arc;
use std::time::Duration;

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::mailbox::Mailbox;
use maos_iac::adapter::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

const STEP: Duration = Duration::from_secs(5);

fn addr(id: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    }
}

fn task_frame(id: u8, to: &[&str]) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: addr("director"),
        to: to.iter().map(|t| addr(t)).collect(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("goal-{id}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn adapter() -> (IacBusAdapter, Arc<Mailbox>, Arc<TransparencyLogAdapter>) {
    init_monotonic_base();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let bus = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&log));
    (bus, mailbox, log)
}

fn frame_ids_for_pid(log: &TransparencyLogAdapter, pid: u32) -> Vec<[u8; 16]> {
    let rows = log
        .query_frames(FrameFilter {
            spirit_pid: Some(pid),
            ..FrameFilter::default()
        })
        .expect("query_frames");
    rows.into_iter().map(|row| row.frame_id).collect()
}

#[tokio::test]
async fn deliver_typed_journals_the_trusted_caller_pid() {
    let (bus, mailbox, log) = adapter();
    let mut handle = mailbox.register_spirit("worker").unwrap();

    bus.deliver_typed(task_frame(1, &["worker"]), 7, None)
        .await
        .unwrap();
    bus.deliver_typed(task_frame(2, &["worker"]), 0, None)
        .await
        .unwrap();

    assert_eq!(frame_ids_for_pid(&log, 7), vec![[1u8; 16]]);
    assert!(!frame_ids_for_pid(&log, 0).contains(&[1u8; 16]));
    assert!(frame_ids_for_pid(&log, 0).contains(&[2u8; 16]));
    assert!(handle.try_recv().unwrap().is_some());
    assert!(handle.try_recv().unwrap().is_some());
}

#[tokio::test]
async fn unregistered_spirit_can_be_registered_again_and_receives_new_frames() {
    let (bus, mailbox, _log) = adapter();
    let mut stale = mailbox.register_spirit("worker").unwrap();
    mailbox.unregister_spirit("worker");

    let mut fresh = mailbox.register_spirit("worker").unwrap();
    bus.deliver_typed(task_frame(3, &["worker"]), 7, None)
        .await
        .unwrap();

    let (kind, frame) = fresh
        .try_recv()
        .unwrap()
        .expect("reloaded Spirit receives frame");
    assert_eq!(kind, FrameKind::TaskAssign);
    assert_eq!(frame.frame_id, [3u8; 16]);
    assert!(
        !matches!(stale.try_recv(), Ok(Some(_))),
        "the stale handle must not receive frames addressed to the reloaded Spirit"
    );
}

#[tokio::test]
async fn delivery_to_unregistered_spirit_is_a_typed_error() {
    let (bus, mailbox, _log) = adapter();
    let _handle = mailbox.register_spirit("worker").unwrap();
    mailbox.unregister_spirit("worker");

    let result = bus.deliver_typed(task_frame(4, &["worker"]), 7, None).await;
    assert!(
        matches!(&result, Err(IacBusError::UnknownSpirit(id)) if id == "worker"),
        "expected UnknownSpirit(worker), got {result:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unregister_racing_a_blocked_multi_recipient_delivery_neither_panics_nor_deadlocks() {
    init_monotonic_base();
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut full = mailbox.register_spirit("full").unwrap();
    let _leaving = mailbox.register_spirit("leaving").unwrap();

    // Fill `full`'s TaskAssign channel (capacity 64) so the next send awaits.
    for id in 0..64u8 {
        mailbox.deliver(task_frame(id, &["full"])).await.unwrap();
    }

    let delivering = Arc::clone(&mailbox);
    let delivery = tokio::spawn(async move {
        delivering
            .deliver(task_frame(200, &["full", "leaving"]))
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !delivery.is_finished(),
        "delivery must be awaiting the full channel"
    );

    // Unregister while the delivery awaits; a map guard held across that await
    // would block this forever.
    let unregistering = Arc::clone(&mailbox);
    let unregister =
        tokio::task::spawn_blocking(move || unregistering.unregister_spirit("leaving"));
    tokio::time::timeout(STEP, unregister)
        .await
        .expect("unregister_spirit deadlocked behind a blocked delivery")
        .unwrap();

    // Free one slot so the blocked send completes and delivery reaches `leaving`.
    assert!(full.try_recv().unwrap().is_some());
    let result = tokio::time::timeout(STEP, delivery)
        .await
        .expect("delivery hung after the channel drained")
        .expect("delivery task must not panic");
    assert!(
        matches!(&result, Err(IacBusError::UnknownSpirit(id)) if id == "leaving"),
        "expected UnknownSpirit(leaving), got {result:?}"
    );
}
