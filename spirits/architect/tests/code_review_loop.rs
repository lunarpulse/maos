//! AC5 — the Architect→Reviewer code-review loop, FR21-clean end-to-end, proven
//! against the **real** kernel FR21 gate (`IacBusAdapter::deliver_typed`) +
//! `DistillateWriter` as dev-deps:
//!
//! Orchestrator dispatches to Architect → Architect proposes (deterministic) →
//! Architect (producer) distills its OWN proposal (Decision K) → Orchestrator
//! references the distillate dispatching to Reviewer → Reviewer critiques
//! (deterministic) → Reviewer (producer) distills its OWN critique → the
//! critique distillate flows back through the Orchestrator.
//!
//! Every hop's `task.assign` after the first carries a `PriorDistillateRef`
//! (never raw — FR21-clean), and neither Spirit invokes a live LLM (Decision E).
//! Architect and Reviewer carry `SpiritRole::Worker` (Decision C).

use std::sync::Arc;

use architect::Architect;
use reviewer::{DesignUnderReview, Reviewer};

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PriorDistillateRef, TaskCompletePayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameKind as TlFrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox};
use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};

const ARCHITECT_PID: u32 = 30;
const REVIEWER_PID: u32 = 31;

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

/// A `SpiritRole::Worker` `task.complete` frame from `worker_id` (Architect and
/// Reviewer are specialized Workers — Decision C).
fn worker_complete(worker_id: &str, seq: u64, result: &str) -> IacFrame {
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
            spirit_id: SpiritId::from(worker_id),
            host_id: None,
            role: Some(SpiritRole::Worker), // Decision C
        },
        to,
        kind: FrameKind::TaskComplete,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskComplete(TaskCompletePayload {
            result: result.into(),
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: lineage(),
    }
}

/// Producer-side distillation (Decision K): the producing Spirit seeds its OWN
/// output frame and distills it via the real `DistillateWriter`. Returns the
/// `Distillate` row id the Orchestrator will reference.
fn producer_distills(
    tl: &Arc<TransparencyLogAdapter>,
    producer_pid: u32,
    intent: &str,
    digest: &str,
) -> PriorDistillateRef {
    let _ = tl.insert_frame_event(
        TlFrameKind::InferenceCall,
        producer_pid,
        None,
        intent,
        digest.as_bytes(),
        FrameOrigin::SpiritAuto,
    );
    let source = vec![tl.last_frame_id()];
    let writer = DistillateWriter::new(Arc::clone(tl), memory());
    let req =
        DistillationRequest::new(source, 1, DigestPayload::Text(digest.to_string()), None).unwrap();
    let receipt = writer
        .write_distillate(producer_pid, req)
        .expect("producer distills its own output — no ScopeViolation");
    PriorDistillateRef {
        digest_frame_id: receipt.digest_frame_id,
        distillation_depth: receipt.effective_distillation_depth,
        intent_lineage: receipt.intent_lineage,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn architect_to_reviewer_loop_is_fr21_clean_end_to_end() {
    let (tl, adapter) = fresh_adapter();
    let _orch = adapter
        .register_spirit_typed(&SpiritId::from("orchestrator"))
        .expect("register orchestrator");
    let _arch = adapter
        .register_spirit_typed(&SpiritId::from("architect"))
        .expect("register architect");
    let _rev = adapter
        .register_spirit_typed(&SpiritId::from("reviewer"))
        .expect("register reviewer");

    let orch = orchestrator::Orchestrator::new("orchestrator");
    let architect = Architect::new("architect");
    let reviewer = Reviewer::new("reviewer");

    // 1. Orchestrator dispatches the design task to the Architect (first
    //    dispatch — no predecessor, None accepted). Architect carries Worker role.
    let f1 = orch.first_dispatch(
        1,
        "architect",
        SpiritRole::Worker,
        "design the overnight founder-loop task",
        "a sound component decomposition",
        lineage(),
    );
    match &f1.payload {
        FramePayload::TaskAssign(p) => assert!(p.prior_distillate_ref.is_none()),
        _ => panic!("expected TaskAssign"),
    }
    adapter
        .deliver_typed(f1)
        .await
        .expect("dispatch to architect accepted");

    // 2. Architect proposes (deterministic) and completes.
    let spec = "parse director instruction; build task assignment; attach distillate ref";
    let proposal = architect.propose(spec);
    assert_eq!(
        architect.propose(spec),
        proposal,
        "bit-identical (no live LLM)"
    );
    adapter
        .deliver_typed(worker_complete("architect", 2, &proposal.digest_text()))
        .await
        .expect("architect task.complete ok");

    // 3. Architect (producer) distills its OWN proposal (Decision K).
    let design_ref = producer_distills(
        &tl,
        ARCHITECT_PID,
        "design-proposal",
        &proposal.digest_text(),
    );

    // 4. Orchestrator references the distillate dispatching to the Reviewer
    //    (follow-up — ALWAYS a distillate ref → FR21-clean). Accepted by the
    //    REAL gate.
    let f2 = orch.followup_dispatch(
        3,
        "reviewer",
        SpiritRole::Worker,
        "review the proposed design",
        "actionable critique",
        design_ref,
        lineage(),
    );
    match &f2.payload {
        FramePayload::TaskAssign(p) => assert!(
            p.prior_distillate_ref.is_some(),
            "FR21: the Architect→Reviewer hop carries a distillate ref"
        ),
        _ => panic!("expected TaskAssign"),
    }
    adapter
        .deliver_typed(f2)
        .await
        .expect("FR21-clean dispatch to reviewer accepted");

    // 5. Reviewer critiques (deterministic) — mapping the proposal at the seam.
    let under_review = DesignUnderReview {
        components: proposal.components.clone(),
        interfaces: proposal.interfaces.clone(),
        risks: proposal.risks.clone(),
    };
    let critique = reviewer.review(&under_review);
    assert_eq!(
        reviewer.review(&under_review),
        critique,
        "bit-identical critique"
    );
    adapter
        .deliver_typed(worker_complete("reviewer", 4, &critique.digest_text()))
        .await
        .expect("reviewer task.complete ok");

    // 6. Reviewer (producer) distills its OWN critique — flows back through the
    //    Orchestrator (FR21-clean).
    let critique_ref = producer_distills(
        &tl,
        REVIEWER_PID,
        "design-critique",
        &critique.digest_text(),
    );
    let f3 = orch.followup_dispatch(
        5,
        "architect",
        SpiritRole::Worker,
        "incorporate the review feedback",
        "revised design",
        critique_ref,
        lineage(),
    );
    adapter
        .deliver_typed(f3)
        .await
        .expect("critique distillate flows back, FR21-clean");

    // The loop ran end-to-end with zero raw-output dispatches.
    assert_eq!(
        orch.dispatched_count(),
        3,
        "3 dispatches: architect, reviewer, back to architect"
    );
}
