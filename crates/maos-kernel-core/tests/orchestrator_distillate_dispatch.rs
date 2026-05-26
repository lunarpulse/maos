#![forbid(unsafe_code)]

//! Story 6.2 AC2 — Orchestrator distillate dispatch surface integration tests.
//!
//! Covers the 4-scenario matrix per spec:
//!
//! | # | predecessor | prior_distillate_ref | outcome |
//! |---|---|---|---|
//! | 2.1 | none | `None` | accepted (first dispatch in fan-out) |
//! | 2.2 | `TaskComplete` + `Distillate` row | `Some(distillate_id)` | accepted |
//! | 2.3 | `TaskComplete` | `None` | REJECTED with `EOrchestratorDispatchRawOutput` |
//! | 2.4 | `TaskComplete` | `Some(raw TaskComplete frame_id)` | REJECTED with `EOrchestratorDispatchRawOutput` |

use std::sync::Arc;

use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, PriorDistillateRef,
    TaskAssignPayload, TaskCompletePayload,
};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::FrameKind as TlFrameKind;
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
use smallvec::smallvec;

fn fresh_adapter() -> (Arc<TransparencyLogAdapter>, IacBusAdapter) {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(
        maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new(),
    );
    let mailbox = Arc::new(Mailbox::new(metrics));
    let adapter = IacBusAdapter::new(mailbox, tl.clone());
    (tl, adapter)
}

fn orchestrator_task_assign(prior: Option<PriorDistillateRef>, seq: u64) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: None,
            role: Some(SpiritRole::Orchestrator),
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("worker"),
            host_id: None,
            role: Some(SpiritRole::Worker),
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("task-{seq}"),
            scope: vec![],
            success_criteria: "done".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: prior,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn worker_task_complete(seq: u64) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&(seq | (1u64 << 63)).to_le_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("worker"),
            host_id: None,
            role: Some(SpiritRole::Worker),
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: None,
            role: Some(SpiritRole::Orchestrator),
        }],
        kind: FrameKind::TaskComplete,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskComplete(TaskCompletePayload {
            result: "ok".into(),
        }),
        // Carry an A2A intent so cross-Spirit lineage check passes.
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn write_distillate_row(tl: &Arc<TransparencyLogAdapter>) -> [u8; 16] {
    // Synthetic Distillate row inserted directly. The kernel-side
    // DistillateWriter cycle is exercised in Story 4.4 tests; here we only
    // need the row to exist with FrameKind::Distillate so the AC2 check
    // resolves prior_distillate_ref correctly.
    let _ = tl.insert_frame_event(
        TlFrameKind::Distillate,
        0,
        None,
        "distillate-stub",
        b"{\"digest\":\"synthetic\"}",
        FrameOrigin::Kernel,
    );
    tl.last_frame_id()
}

#[tokio::test(flavor = "multi_thread")]
async fn ac2_scenario_2_1_first_dispatch_no_predecessor_accepted() {
    let (_tl, adapter) = fresh_adapter();
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");

    let frame = orchestrator_task_assign(None, 1);
    let result = adapter.deliver_typed(frame).await;
    assert!(
        result.is_ok(),
        "first dispatch with no predecessor must be accepted: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ac2_scenario_2_2_follow_up_with_distillate_ref_accepted() {
    let (tl, adapter) = fresh_adapter();
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");

    // 1. First dispatch (accepted) — establishes the fan-out.
    let f1 = orchestrator_task_assign(None, 1);
    adapter.deliver_typed(f1).await.expect("first dispatch ok");

    // 2. Worker completes the task.
    let tc = worker_task_complete(2);
    adapter
        .deliver_typed(tc)
        .await
        .expect("task complete ok");

    // 3. Kernel-side distillate row exists for the prior worker output.
    let distillate_id = write_distillate_row(&tl);

    // 4. Orchestrator dispatches next task referencing the distillate.
    let f2 = orchestrator_task_assign(
        Some(PriorDistillateRef {
            digest_frame_id: distillate_id,
            distillation_depth: 1,
            intent_lineage: IntentLineage::default(),
        }),
        3,
    );
    let result = adapter.deliver_typed(f2).await;
    assert!(
        result.is_ok(),
        "follow-up dispatch with distillate_ref must be accepted: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ac2_scenario_2_3_follow_up_none_after_complete_rejected() {
    let (_tl, adapter) = fresh_adapter();
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");

    // First dispatch establishes fan-out.
    adapter
        .deliver_typed(orchestrator_task_assign(None, 1))
        .await
        .expect("first ok");

    // Worker completes.
    adapter
        .deliver_typed(worker_task_complete(2))
        .await
        .expect("complete ok");

    // Follow-up dispatch with no prior_distillate_ref → REJECTED.
    let f2 = orchestrator_task_assign(None, 3);
    let err = adapter.deliver_typed(f2).await.expect_err("must reject");
    match err {
        IacBusError::EOrchestratorDispatchRawOutput { orchestrator, task_id } => {
            assert_eq!(orchestrator, "orchestrator");
            assert_eq!(task_id, "task-3");
        }
        e => panic!("expected EOrchestratorDispatchRawOutput, got {e:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ac2_scenario_2_4_follow_up_pointing_at_raw_task_complete_rejected() {
    let (tl, adapter) = fresh_adapter();
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");

    adapter
        .deliver_typed(orchestrator_task_assign(None, 1))
        .await
        .expect("first ok");

    // Worker completes — captures the raw TaskComplete frame_id.
    adapter
        .deliver_typed(worker_task_complete(2))
        .await
        .expect("complete ok");
    let raw_tc_id = tl.last_frame_id();

    // Follow-up references the RAW TaskComplete row (not a Distillate row) → REJECTED.
    let f2 = orchestrator_task_assign(
        Some(PriorDistillateRef {
            digest_frame_id: raw_tc_id,
            distillation_depth: 0,
            intent_lineage: IntentLineage::default(),
        }),
        3,
    );
    let err = adapter.deliver_typed(f2).await.expect_err("must reject");
    assert!(
        matches!(err, IacBusError::EOrchestratorDispatchRawOutput { .. }),
        "expected EOrchestratorDispatchRawOutput, got {err:?}"
    );
}
