//! AC3 — FR21: the Orchestrator dispatches **distillate-fed**, never raw. Proven
//! against the **real** kernel FR21 gate (`IacBusAdapter::deliver_typed` →
//! `EOrchestratorDispatchRawOutput`) as a dev-dep, reproducing the 4-scenario
//! matrix of `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs`
//! but driven from the Orchestrator reference Spirit:
//!
//! - 2.1: first dispatch (`None`) ⇒ accepted (no predecessor).
//! - 2.2: follow-up with a producer-authored `Distillate` ref ⇒ accepted.
//! - 2.3: follow-up with `None` after a completion ⇒ REJECTED.
//! - 2.4: follow-up pointing at a raw `TaskComplete` row ⇒ REJECTED.
//!
//! **Decision K** — the distillate is authored by the **producer** (the Worker)
//! over its OWN emitter frames via the real `DistillateWriter::write_distillate`
//! (no `ScopeViolation`); the Orchestrator only **references** the resulting
//! `Distillate`, and the kernel runs `admit_for_consumer` (I13). The distillate
//! row is written BEFORE the Orchestrator references it (mirrors the 6.2
//! reference ordering).

use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskCompletePayload};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::{AllowedPromotionSet, IntentLineage};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameKind as TlFrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox};
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
use orchestrator::Orchestrator;

const WORKER_PID: u32 = 20;

fn fresh_adapter() -> (Arc<TransparencyLogAdapter>, IacBusAdapter) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let mailbox = Arc::new(Mailbox::new(metrics));
    let adapter = IacBusAdapter::new(mailbox, tl.clone());
    (tl, adapter)
}

fn lineage() -> IntentLineage {
    IntentLineage::new(vec![A2AIntent::new("founder-loop-wedge")])
}

fn memory() -> Arc<dyn std::any::Any + Send + Sync> {
    Arc::new(0u8)
}

/// A worker `TaskComplete` frame delivered through the real bus (establishes a
/// prior completion in the FR21 window).
fn worker_task_complete(seq: u64) -> IacFrame {
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&(seq | (1u64 << 62)).to_le_bytes());
    let mut to = smallvec::SmallVec::<[FrameAddress; 1]>::new();
    to.push(FrameAddress {
        spirit_id: SpiritId::from("orchestrator"),
        host_id: None,
        role: Some(SpiritRole::Orchestrator),
    });
    IacFrame {
        frame_id,
        timestamp_ns: seq,
        logical_clock: seq,
        from: FrameAddress {
            spirit_id: SpiritId::from("worker"),
            host_id: None,
            role: Some(SpiritRole::Worker),
        },
        to,
        kind: FrameKind::TaskComplete,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskComplete(TaskCompletePayload {
            result: "worker done".into(),
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: lineage(),
    }
}

/// Producer-side distillation (Decision K): the Worker seeds its OWN emitter
/// frames and distills them via the real `DistillateWriter`. Returns the
/// `digest_frame_id` of the minted `Distillate` row + the kernel-computed lineage.
fn producer_distills_own_output(
    tl: &Arc<TransparencyLogAdapter>,
    intents: &[&str],
) -> ([u8; 16], IntentLineage) {
    let mut source = Vec::new();
    for intent in intents {
        let _ = tl.insert_frame_event(
            TlFrameKind::InferenceCall,
            WORKER_PID,
            None,
            intent,
            br#"{"claim":"worker output"}"#,
            FrameOrigin::SpiritAuto,
        );
        source.push(tl.last_frame_id());
    }
    let writer = DistillateWriter::new(Arc::clone(tl), memory());
    let req = DistillationRequest::new(
        source,
        1,
        DigestPayload::Text("worker output, distilled".into()),
        None,
    )
    .unwrap();
    let receipt = writer
        .write_distillate(WORKER_PID, req)
        .expect("producer distills its own frames — no ScopeViolation");
    (receipt.digest_frame_id, receipt.intent_lineage)
}

#[tokio::test(flavor = "multi_thread")]
async fn scenario_2_1_first_dispatch_none_accepted() {
    let (_tl, adapter) = fresh_adapter();
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _reviewer = adapter
        .register_spirit_typed(&SpiritId::from("reviewer"))
        .expect("register reviewer");

    let orch = Orchestrator::new("orchestrator");
    let frame = orch.first_dispatch(1, "worker", SpiritRole::Worker, "design", "done", lineage());
    let result = adapter.deliver_typed(frame).await;
    assert!(
        result.is_ok(),
        "first dispatch with no predecessor must be accepted: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn scenario_2_2_followup_with_producer_distillate_accepted() {
    let (tl, adapter) = fresh_adapter();
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _reviewer = adapter
        .register_spirit_typed(&SpiritId::from("reviewer"))
        .expect("register reviewer");

    let orch = Orchestrator::new("orchestrator");

    // First dispatch establishes the fan-out; worker completes.
    adapter
        .deliver_typed(orch.first_dispatch(
            1,
            "worker",
            SpiritRole::Worker,
            "design",
            "done",
            lineage(),
        ))
        .await
        .expect("first dispatch ok");
    adapter
        .deliver_typed(worker_task_complete(2))
        .await
        .expect("task complete ok");

    // Decision K — producer distills its OWN output BEFORE the Orchestrator
    // references it.
    let (digest_id, kernel_lineage) = producer_distills_own_output(&tl, &["consult", "verify"]);

    // I13 — the Orchestrator's allowed-promotion set admits the digest's lineage.
    let mut allowed = AllowedPromotionSet::new();
    allowed.insert(A2AIntent::new("consult"));
    allowed.insert(A2AIntent::new("verify"));
    let writer = DistillateWriter::new(Arc::clone(&tl), memory());
    writer
        .admit_for_consumer(digest_id, &allowed)
        .expect("I13: orchestrator may promote the producer's distillate");
    assert!(
        !kernel_lineage.as_slice().is_empty(),
        "kernel computed a non-empty intent_lineage"
    );

    // Orchestrator references the distillate (never the raw TaskComplete).
    let prior = maos_domain::frame::PriorDistillateRef {
        digest_frame_id: digest_id,
        distillation_depth: 1,
        intent_lineage: kernel_lineage,
    };
    let frame = orch.followup_dispatch(
        3,
        "reviewer",
        SpiritRole::Worker,
        "review",
        "ok",
        prior,
        lineage(),
    );
    let result = adapter.deliver_typed(frame).await;
    assert!(
        result.is_ok(),
        "follow-up with a producer-authored distillate ref must be accepted: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn scenario_2_3_followup_none_after_completion_rejected() {
    let (_tl, adapter) = fresh_adapter();
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _reviewer = adapter
        .register_spirit_typed(&SpiritId::from("reviewer"))
        .expect("register reviewer");

    let orch = Orchestrator::new("orchestrator");
    adapter
        .deliver_typed(orch.first_dispatch(
            1,
            "worker",
            SpiritRole::Worker,
            "design",
            "done",
            lineage(),
        ))
        .await
        .expect("first ok");
    adapter
        .deliver_typed(worker_task_complete(2))
        .await
        .expect("complete ok");

    // A raw follow-up (prior_distillate_ref = None) after a completion — the
    // exact loophole FR21 closes. Built via the low-level builder to simulate a
    // mis-behaving Orchestrator; the KERNEL gate rejects it.
    let payload = orch.build_task_assign("review", "ok", None);
    let frame = orch.assign_frame(3, "reviewer", SpiritRole::Worker, payload, lineage());
    let err = adapter
        .deliver_typed(frame)
        .await
        .expect_err("raw follow-up must be rejected");
    match err {
        IacBusError::EOrchestratorDispatchRawOutput { orchestrator, .. } => {
            assert_eq!(orchestrator, "orchestrator");
        }
        e => panic!("expected EOrchestratorDispatchRawOutput, got {e:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn scenario_2_4_followup_pointing_at_raw_task_complete_rejected() {
    let (tl, adapter) = fresh_adapter();
    let _orchestrator = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");
    let _worker = adapter
        .register_spirit_typed(&SpiritId::from("worker"))
        .expect("register worker");
    let _reviewer = adapter
        .register_spirit_typed(&SpiritId::from("reviewer"))
        .expect("register reviewer");

    let orch = Orchestrator::new("orchestrator");
    adapter
        .deliver_typed(orch.first_dispatch(
            1,
            "worker",
            SpiritRole::Worker,
            "design",
            "done",
            lineage(),
        ))
        .await
        .expect("first ok");
    adapter
        .deliver_typed(worker_task_complete(2))
        .await
        .expect("complete ok");
    // The raw TaskComplete row id — NOT a Distillate row.
    let raw_tc_id = tl.last_frame_id();

    let prior = maos_domain::frame::PriorDistillateRef {
        digest_frame_id: raw_tc_id,
        distillation_depth: 1,
        intent_lineage: IntentLineage::default(),
    };
    let frame = orch.followup_dispatch(
        3,
        "reviewer",
        SpiritRole::Worker,
        "review",
        "ok",
        prior,
        lineage(),
    );
    let err = adapter
        .deliver_typed(frame)
        .await
        .expect_err("a ref to a raw TaskComplete must be rejected");
    assert!(
        matches!(err, IacBusError::EOrchestratorDispatchRawOutput { .. }),
        "expected EOrchestratorDispatchRawOutput, got {err:?}"
    );
}
