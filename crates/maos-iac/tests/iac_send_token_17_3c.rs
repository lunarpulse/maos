//! Story 17-3c — `iac.send` capability mediation: the kernel-minted token
//! passed to `deliver_typed` / `retract` is journaled in the Transparency Log
//! row's `capability_token` column (direct writer and DRR scheduler paths);
//! `None` keeps the column NULL.

use std::sync::Arc;

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::iac_bus_types::RetractOutcome;
use maos_domain::invariants::i1::{IntentClass, TokenId};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::mailbox::Mailbox;
use maos_iac::adapter::transparency_log::{
    capability_token_column, FrameFilter, FrameKind as TlKind, TransparencyLogAdapter,
    TransparencyLogEntry,
};
use maos_iac::adapter::{DrrScheduler, IacBusAdapter, IacRtMetrics};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

fn addr(id: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    }
}

fn task_frame(id: u8) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: addr("guest"),
        to: [addr("worker")].into_iter().collect(),
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

fn rows_for_pid(log: &TransparencyLogAdapter, pid: u32) -> Vec<TransparencyLogEntry> {
    log.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        ..FrameFilter::default()
    })
    .expect("query_frames")
}

/// Deliver one tokened and one tokenless frame, retract with and without a
/// token, and assert every row's `capability_token` column. Under DRR only the
/// tokened retract runs: DRR journals every Retract row under the placeholder
/// frame id `[0; 16]`, so a second DRR retract is a duplicate row (pre-existing).
async fn exercise(drr: bool) {
    init_monotonic_base();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut bus = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&log));
    if drr {
        let (warning_tx, _warning_rx) = tokio::sync::mpsc::unbounded_channel();
        bus = bus.with_drr_scheduler(DrrScheduler::new(Arc::clone(&log), warning_tx));
    }
    let _worker = mailbox.register_spirit("worker").unwrap();
    let guest = SpiritId::from("guest");
    let send_token = TokenId([0x5A; 16]);
    let retract_token = TokenId([0xC3; 16]);

    bus.deliver_typed(task_frame(1), 41, Some(send_token))
        .await
        .unwrap();
    bus.deliver_typed(task_frame(2), 42, None).await.unwrap();

    let tokened = rows_for_pid(&log, 41);
    assert_eq!(tokened.len(), 1);
    assert_eq!(
        tokened[0].capability_token,
        Some(capability_token_column(&send_token))
    );
    let tokenless = rows_for_pid(&log, 42);
    assert_eq!(tokenless.len(), 1);
    assert_eq!(tokenless[0].capability_token, None);

    let outcome = bus
        .retract([1; 16], "superseded".into(), &guest, Some(retract_token))
        .await
        .unwrap();
    assert!(matches!(outcome, RetractOutcome::Retracted { .. }));
    if !drr {
        let outcome = bus
            .retract([2; 16], "superseded".into(), &guest, None)
            .await
            .unwrap();
        assert!(matches!(outcome, RetractOutcome::Retracted { .. }));
    }

    let retract_row = |pid| {
        rows_for_pid(&log, pid)
            .into_iter()
            .find(|row| row.kind == TlKind::Retract)
            .expect("retract row journaled under the original sender's pid")
    };
    assert_eq!(
        retract_row(41).capability_token,
        Some(capability_token_column(&retract_token))
    );
    if !drr {
        assert_eq!(retract_row(42).capability_token, None);
    }
}

#[test]
fn capability_token_column_is_token_then_sixteen_zero_bytes() {
    let column = capability_token_column(&TokenId([0xAB; 16]));
    assert_eq!(column[..16], [0xAB; 16]);
    assert_eq!(column[16..], [0; 16]);
}

#[tokio::test]
async fn direct_writer_journals_the_iac_send_token() {
    exercise(false).await;
}

// `flush_batch` uses `block_in_place`, which needs the multi-thread runtime.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drr_scheduler_journals_the_iac_send_token() {
    exercise(true).await;
}
