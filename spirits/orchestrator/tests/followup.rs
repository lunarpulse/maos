#![forbid(unsafe_code)]

//! Story 19-1 (AC4, D-19-1-06 A) — the typed REMOTE follow-up. Section k+1's
//! `task.assign` carries section k's `PriorDistillateRef` (FR21: never raw
//! output) and is built under `assign_frame_remote`'s lineage, host and
//! consent rules; the local `followup_dispatch` keeps its same-Host shape.

use maos_domain::frame::{FramePayload, IacFrame, PriorDistillateRef};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::SpiritRole;
use orchestrator::{Orchestrator, RemoteFrameError, DELEGATION_CONSENT_INTENT};

const TO_HOST: &str = "developer-remote-host";
const FROM_HOST: &str = "founder-loop-host";

fn lineage() -> IntentLineage {
    IntentLineage::new(vec![A2AIntent::new(DELEGATION_CONSENT_INTENT)])
}

fn prior() -> PriorDistillateRef {
    PriorDistillateRef {
        digest_frame_id: [0xD1; 16],
        distillation_depth: 1,
        intent_lineage: lineage(),
    }
}

fn remote_followup(o: &Orchestrator, lineage: IntentLineage) -> Result<IacFrame, RemoteFrameError> {
    o.followup_dispatch_remote(
        2,
        0xBEEF,
        "worker-2",
        SpiritRole::Worker,
        "### Story 2: validate the schema",
        "exit 0",
        prior(),
        lineage,
        TO_HOST,
        FROM_HOST,
        A2AIntent::new(DELEGATION_CONSENT_INTENT),
    )
}

#[test]
fn remote_followup_carries_the_prior_distillate_hosts_and_a_granter_bound_envelope() {
    let o = Orchestrator::new("orchestrator");
    let frame = remote_followup(&o, lineage()).expect("a canonical lineage builds the frame");

    match &frame.payload {
        FramePayload::TaskAssign(payload) => {
            assert_eq!(payload.goal, "### Story 2: validate the schema");
            assert_eq!(payload.prior_distillate_ref, Some(prior()));
        }
        other => panic!("expected TaskAssign, got {other:?}"),
    }
    assert_eq!(&frame.frame_id[..8], &2u64.to_le_bytes());
    assert_eq!(&frame.frame_id[8..], &0xBEEFu64.to_le_bytes());
    assert!(!frame.to.is_empty());
    for addr in frame.to.iter() {
        assert_eq!(addr.spirit_id.as_str(), "worker-2");
        assert_eq!(addr.host_id.as_ref().map(|h| h.as_str()), Some(TO_HOST));
    }
    assert_eq!(
        frame.from.host_id.as_ref().map(|h| h.as_str()),
        Some(FROM_HOST)
    );
    let envelope = frame
        .consent_envelope
        .as_ref()
        .expect("a remote follow-up carries a consent envelope");
    assert_eq!(
        envelope.granter, frame.from,
        "ADR-012: granter == frame.from"
    );
    assert_eq!(
        envelope.intent_class.as_ref().map(|i| i.as_str()),
        Some(DELEGATION_CONSENT_INTENT)
    );

    let decision = o.last_dispatch().expect("the follow-up is recorded");
    assert!(decision.has_distillate_ref);
    assert_eq!(decision.distillation_depth, 1);
    assert_eq!(o.dispatched_count(), 1);
}

/// The typed builder is exactly `build_task_assign(Some(prior))` promoted by
/// `assign_frame_remote` — no field differs but the per-envelope consent id.
#[test]
fn remote_followup_equals_the_composed_remote_frame() {
    let typed = remote_followup(&Orchestrator::new("orchestrator"), lineage()).unwrap();
    let o = Orchestrator::new("orchestrator");
    let payload = o.build_task_assign("### Story 2: validate the schema", "exit 0", Some(prior()));
    let composed = o
        .assign_frame_remote(
            2,
            0xBEEF,
            "worker-2",
            SpiritRole::Worker,
            payload,
            lineage(),
            TO_HOST,
            FROM_HOST,
            A2AIntent::new(DELEGATION_CONSENT_INTENT),
        )
        .unwrap();
    let without_consent_id = |mut frame: IacFrame| {
        if let Some(envelope) = frame.consent_envelope.as_mut() {
            envelope.consent_id = [0; 16];
        }
        frame
    };
    assert_eq!(without_consent_id(typed), without_consent_id(composed));
}

#[test]
fn remote_followup_refuses_an_empty_lineage_before_recording_a_dispatch() {
    let o = Orchestrator::new("orchestrator");
    assert_eq!(
        remote_followup(&o, IntentLineage::default()),
        Err(RemoteFrameError::EmptyIntentLineage)
    );
    assert_eq!(o.dispatched_count(), 0);
    assert!(o.last_dispatch().is_none());
}

#[test]
fn remote_followup_refuses_a_non_canonical_lineage_before_recording_a_dispatch() {
    let o = Orchestrator::new("orchestrator");
    assert_eq!(
        remote_followup(&o, IntentLineage::new(vec![A2AIntent::new("task.assign")])),
        Err(RemoteFrameError::NonCanonicalIntent("task.assign".into()))
    );
    assert_eq!(o.dispatched_count(), 0);
}

/// D-19-1-04 A — the root's loop drives a clone; the clone's dispatch is the
/// loaded instance's bookkeeping.
#[test]
fn a_clone_shares_the_dispatch_bookkeeping() {
    let loaded = Orchestrator::new("orchestrator");
    let emitter = loaded.clone();
    emitter.begin_delegation();
    assert!(
        !loaded.is_safe_point(),
        "the loaded instance sees the in-flight delegation"
    );
    remote_followup(&emitter, lineage()).unwrap();
    assert_eq!(loaded.dispatched_count(), 1);
    emitter.complete_delegation();
    assert!(loaded.is_safe_point());
}

#[test]
fn the_local_followup_keeps_its_same_host_shape() {
    let o = Orchestrator::new("orchestrator");
    let local = o.followup_dispatch(
        1,
        "worker",
        SpiritRole::Worker,
        "review",
        "ok",
        prior(),
        lineage(),
    );
    assert!(local.to.iter().all(|a| a.host_id.is_none()));
    assert!(local.from.host_id.is_none());
    assert!(local.consent_envelope.is_none());
}
