//! Story 21-7 — the production consent policy (granter + gate), the retract audience
//! (D-21-7-03 A) and the unbound-port refusal (D-21-7-04 C), on the real adapter, mailbox and
//! Transparency Log. "Consent" here is a technical recall/delivery restriction.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::TelemetryEventPayload;
use maos_domain::frame::{
    ConsentEnvelope, ConsentRecall, FrameAddress, FramePayload, IacFrame, RuptureReason,
    TaskAssignPayload,
};
use maos_domain::iac_bus_types::{IacBusError, RetractOutcome};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::IacBusPort;
use maos_iac::adapter::mailbox::{Mailbox, SpiritMailboxHandle, KERNEL_SENDER_SPIRIT_ID};
use maos_iac::adapter::ConsentPolicy;
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, SpiritId};

fn addr(id: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    }
}

fn task(id: u8, from: &str, to: &[&str]) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: addr(from),
        to: to.iter().map(|id| addr(id)).collect(),
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

struct World {
    bus: IacBusAdapter,
    tl: Arc<TransparencyLogAdapter>,
    pids: Arc<RwLock<BTreeMap<String, u32>>>,
    handles: BTreeMap<&'static str, SpiritMailboxHandle>,
}

const SPIRITS: [(&str, u32); 4] = [("a", 1), ("b", 2), ("c", 3), ("d", 4)];

fn world() -> World {
    init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let pids = Arc::new(RwLock::new(
        SPIRITS
            .iter()
            .map(|(id, pid)| (id.to_string(), *pid))
            .collect(),
    ));
    tl.install_recipient_resolver(Arc::clone(&pids) as _)
        .unwrap();
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    mailbox.install_transparency_log(Arc::clone(&tl)).unwrap();
    let bus = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&tl));
    mailbox
        .install_consent_gate(Arc::clone(bus.consent_policy()) as _)
        .unwrap();
    let handles = SPIRITS
        .iter()
        .map(|(id, _)| (*id, mailbox.register_spirit(id).unwrap()))
        .collect();
    World {
        bus,
        tl,
        pids,
        handles,
    }
}

impl World {
    fn pid(&self, id: &str) -> u32 {
        SPIRITS.iter().find(|(name, _)| *name == id).unwrap().1
    }

    /// Stamp as the session does, then deliver as the admitted sender.
    async fn send(&self, mut frame: IacFrame) {
        let pid = self.pid(frame.from.spirit_id.as_str());
        let released = frame.frame_id;
        self.bus.consent_policy().stamp(&mut frame, pid);
        self.bus.deliver_typed(frame, pid, None).await.unwrap();
        self.bus.consent_policy().release(released);
    }

    fn declare(&self, sender: &str, grants: &[(&str, ConsentRecall)]) {
        let grants = grants.iter().map(|(id, r)| (id.to_string(), *r)).collect();
        self.bus
            .consent_policy()
            .register(sender, self.pid(sender), Some(grants));
    }

    fn drain(&mut self, id: &str) -> Vec<(FrameKind, IacFrame)> {
        let handle = self.handles.get_mut(id).unwrap();
        std::iter::from_fn(|| handle.try_recv().unwrap()).collect()
    }

    fn recipients(&self, frame_id: [u8; 16]) -> Vec<String> {
        let recorded = self.tl.recorded_recipients(frame_id).unwrap();
        recorded.into_iter().map(|r| r.spirit_id).collect()
    }
}

#[test]
fn stamp_mints_the_senders_envelope_and_discards_a_guest_supplied_one() {
    let w = world();
    w.declare(
        "a",
        &[
            ("b", ConsentRecall::HeaderOnly),
            ("c", ConsentRecall::Withheld),
        ],
    );
    let forged = ConsentEnvelope {
        consent_id: [9; 16],
        granter: addr("b"),
        timestamp_ns: 1,
        intent_class: None,
        valid_until_ns: Some(2),
        recall: ConsentRecall::Full,
    };
    let policy = w.bus.consent_policy();

    let mut one = task(1, "a", &["b"]);
    one.consent_envelope = Some(forged);
    policy.stamp(&mut one, 1);
    let envelope = one.consent_envelope.clone().unwrap();
    assert!(envelope.granted_by(&one.from), "granter is the sender");
    assert_eq!(envelope.recall, ConsentRecall::HeaderOnly);
    assert_eq!(
        (envelope.intent_class.clone(), envelope.valid_until_ns),
        (None, None)
    );
    assert_ne!(envelope.timestamp_ns, 1);

    let mut again = task(2, "a", &["b"]);
    policy.stamp(&mut again, 1);
    assert_ne!(
        again.consent_envelope.unwrap().consent_id,
        envelope.consent_id,
        "every envelope carries its own id"
    );

    let mut both = task(3, "a", &["b", "c"]);
    policy.stamp(&mut both, 1);
    assert_eq!(
        both.consent_envelope.unwrap().recall,
        ConsentRecall::Withheld
    );

    let mut ungranted = task(4, "a", &["d"]);
    policy.stamp(&mut ungranted, 1);
    assert_eq!(
        ungranted.consent_envelope.unwrap().recall,
        ConsentRecall::Withheld
    );

    let mut undeclared = task(5, "d", &["b"]);
    undeclared.consent_envelope = Some(envelope);
    policy.stamp(&mut undeclared, 4);
    assert!(
        undeclared.consent_envelope.is_none(),
        "no declaration, no envelope"
    );
}

#[tokio::test]
async fn an_uncovered_addressee_is_refused_and_never_recorded() {
    let mut w = world();
    w.declare("a", &[("b", ConsentRecall::HeaderOnly)]);
    w.send(task(1, "a", &["b", "c"])).await;

    assert_eq!(w.recipients([1; 16]), vec!["b".to_string()]);
    assert_eq!(
        w.drain("b").len(),
        1,
        "the granted addressee receives the frame"
    );
    assert!(
        w.drain("c").is_empty(),
        "the uncovered addressee receives nothing"
    );
    let sender: Vec<_> = w.drain("a");
    let [(FrameKind::ConsentRupture, rupture)] = sender.as_slice() else {
        panic!("the sender receives exactly one typed rupture, got {sender:?}");
    };
    let FramePayload::ConsentRupture(payload) = &rupture.payload else {
        panic!("rupture payload");
    };
    assert_eq!(payload.original_frame_id, [1; 16]);
    assert_eq!(payload.rejected.len(), 1);
    assert_eq!(payload.rejected[0].address.spirit_id.as_str(), "c");
    assert_eq!(payload.rejected[0].reason, RuptureReason::ConsentNotGranted);
}

#[tokio::test]
async fn a_sender_without_a_declaration_is_unrestricted_and_withdrawal_lifts_the_gate() {
    let mut w = world();
    w.send(task(1, "d", &["b", "c"])).await;
    assert_eq!(
        w.recipients([1; 16]),
        vec!["b".to_string(), "c".to_string()]
    );
    assert!(w.drain("d").is_empty(), "no rupture without a declaration");

    w.declare("d", &[("b", ConsentRecall::Full)]);
    w.send(task(2, "d", &["c"])).await;
    assert!(w.recipients([2; 16]).is_empty());
    w.bus.withdraw_spirit("d", 4);
    w.send(task(3, "d", &["c"])).await;
    assert_eq!(w.recipients([3; 16]), vec!["c".to_string()]);
}

#[tokio::test]
async fn retract_reaches_every_recorded_recipient_regardless_of_later_policy() {
    let mut w = world();
    w.declare(
        "a",
        &[("b", ConsentRecall::Full), ("c", ConsentRecall::Full)],
    );
    w.send(task(1, "a", &["b", "c", "d"])).await;
    assert_eq!(
        w.recipients([1; 16]),
        vec!["b".to_string(), "c".to_string()]
    );
    for id in ["a", "b", "c", "d"] {
        w.drain(id);
    }
    // The policy narrows between send and retract: the retraction must still arrive.
    w.declare("a", &[("d", ConsentRecall::Full)]);

    let outcome = w
        .bus
        .retract([1; 16], "superseded".into(), &SpiritId::from("a"), None)
        .await
        .unwrap();
    let RetractOutcome::Retracted { retract_frame_id } = outcome else {
        panic!("expected a retraction, got {outcome:?}");
    };
    for id in ["b", "c"] {
        let got = w.drain(id);
        let [(FrameKind::Retract, frame)] = got.as_slice() else {
            panic!("{id} must receive the retract, got {got:?}");
        };
        assert_eq!(frame.frame_id, retract_frame_id);
    }
    assert!(
        w.drain("d").is_empty(),
        "a never-recorded addressee is not notified"
    );
    assert!(w.drain("a").is_empty(), "the gate never ruptures a retract");
    let journaled = w.tl.query_frame_by_id(retract_frame_id).unwrap().unwrap();
    assert_eq!(
        journaled.spirit_pid,
        w.pid("a"),
        "journaled at the original sender's pid"
    );
    assert_eq!(w.recipients(retract_frame_id).len(), 2);
    assert!(matches!(
        w.bus
            .retract([1; 16], "again".into(), &SpiritId::from("a"), None)
            .await
            .unwrap(),
        RetractOutcome::Already { existing_retract_frame_id } if existing_retract_frame_id == retract_frame_id
    ));
    assert!(matches!(
        w.bus
            .retract([1; 16], "not mine".into(), &SpiritId::from("b"), None)
            .await,
        Err(IacBusError::RetractAuthorityViolation { .. })
    ));
}

#[tokio::test]
async fn retract_skips_a_recipient_that_is_no_longer_the_admitted_incarnation() {
    let mut w = world();
    w.send(task(1, "a", &["b", "c"])).await;
    for id in ["b", "c"] {
        w.drain(id);
    }
    // `c` was replaced by a new incarnation (new pid) after receiving the frame.
    w.pids.write().unwrap().insert("c".into(), 33);

    w.bus
        .retract([1; 16], "superseded".into(), &SpiritId::from("a"), None)
        .await
        .unwrap();
    assert_eq!(w.drain("b").len(), 1);
    assert!(
        w.drain("c").is_empty(),
        "the successor never received the original"
    );
}

#[tokio::test]
async fn the_unbound_port_refuses_a_spirit_sender_and_keeps_the_kernel_at_pid_zero() {
    let w = world();
    let spirit = task(1, "a", &["b"]);
    let refused = w.bus.deliver(spirit).await;
    assert!(
        matches!(&refused, Err(IacBusError::UnattributedSender(sender)) if sender == "a"),
        "{refused:?}"
    );
    assert!(
        w.tl.query_frame_by_id([1; 16]).unwrap().is_none(),
        "nothing journaled as the root"
    );

    let mut kernel = task(2, KERNEL_SENDER_SPIRIT_ID, &["b"]);
    kernel.auto_marker = FrameOrigin::Kernel;
    w.bus.deliver(kernel).await.unwrap();
    let row = w.tl.query_frame_by_id([2; 16]).unwrap().unwrap();
    assert_eq!(
        (row.spirit_pid, row.from_spirit_id.as_str()),
        (0, KERNEL_SENDER_SPIRIT_ID)
    );
}

#[test]
fn stamping_as_another_pid_refuses_everything_and_a_stale_cleanup_cannot_erase_a_successor() {
    let w = world();
    w.declare("a", &[("b", ConsentRecall::Full)]);
    let policy = w.bus.consent_policy();
    let mut frame = task(1, "a", &["b"]);
    assert!(
        policy.stamp(&mut frame, 9),
        "an unbound incarnation refuses every addressee"
    );
    assert_eq!(
        frame.consent_envelope.unwrap().recall,
        ConsentRecall::Withheld
    );
    policy.release([1; 16]);

    // The replacement registered; the old incarnation's cleanup arrives late.
    policy.register(
        "a",
        9,
        Some([("c".to_string(), ConsentRecall::Full)].into()),
    );
    policy.unregister("a", 1);
    let mut kept = task(2, "a", &["c"]);
    assert!(
        !policy.stamp(&mut kept, 9),
        "the replacement's declaration survived"
    );
    policy.release([2; 16]);

    policy.unregister("a", 9);
    let mut gone = task(3, "a", &["c"]);
    policy.stamp(&mut gone, 9);
    assert!(gone.consent_envelope.is_none());
    policy.release([3; 16]);
}

#[tokio::test]
async fn a_stamped_frame_is_gated_by_its_own_snapshot_after_its_sender_is_withdrawn() {
    let mut w = world();
    w.declare("a", &[("b", ConsentRecall::Full)]);
    let mut frame = task(1, "a", &["b", "c"]);
    w.bus.consent_policy().stamp(&mut frame, 1);
    // The sender unloads between the stamp and the gate.
    w.bus.withdraw_spirit("a", 1);
    w.bus.deliver_typed(frame, 1, None).await.unwrap();
    w.bus.consent_policy().release([1; 16]);

    assert_eq!(w.recipients([1; 16]), vec!["b".to_string()]);
    assert!(
        w.drain("c").is_empty(),
        "the retired sender was still gated by its envelope"
    );
}

#[tokio::test]
async fn telemetry_never_creates_directed_participant_records() {
    let w = world();
    w.declare("a", &[("b", ConsentRecall::Full)]);
    let mut telemetry = task(1, "a", &["b", "c"]);
    telemetry.kind = FrameKind::TelemetryEvent;
    telemetry.payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
        event_type: "t".into(),
        data: "d".into(),
    });
    w.send(telemetry).await;
    assert!(
        w.recipients([1; 16]).is_empty(),
        "a gate-exempt broadcast must not make any addressee a recipient"
    );
}

#[tokio::test]
async fn a_retired_sender_incarnation_cannot_retract() {
    let w = world();
    w.send(task(1, "a", &["b"])).await;
    w.pids.write().unwrap().insert("a".into(), 77);
    let refused = w
        .bus
        .retract([1; 16], "superseded".into(), &SpiritId::from("a"), None)
        .await;
    assert!(
        matches!(refused, Err(IacBusError::RetractAuthorityViolation { .. })),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_reload_between_the_verdict_and_the_enqueue_never_reaches_the_replacement() {
    let mut w = world();
    let frame = task(1, "a", &["b"]);
    let prepared = w.bus.mailbox().prepare(&frame, Some(&*w.pids));
    w.pids.write().unwrap().insert("b".into(), 22);
    w.bus
        .mailbox()
        .deliver_prepared(frame, prepared)
        .await
        .unwrap();
    assert!(w.drain("b").is_empty());
}

#[tokio::test]
async fn an_unloaded_addressee_suppresses_neither_the_rupture_nor_the_valid_delivery() {
    let mut w = world();
    w.declare("a", &[("b", ConsentRecall::Full)]);
    w.bus.mailbox().unregister_spirit("c");
    w.send(task(1, "a", &["c", "b"])).await;

    assert_eq!(w.drain("b").len(), 1);
    let sender = w.drain("a");
    assert!(
        matches!(sender.as_slice(), [(FrameKind::ConsentRupture, _)]),
        "the sender is told about the refused addressee: {sender:?}"
    );
}

#[test]
fn a_swap_may_only_keep_or_narrow_consent() {
    let rows = |rows: &[(&str, ConsentRecall)]| -> BTreeMap<String, ConsentRecall> {
        rows.iter().map(|(id, r)| (id.to_string(), *r)).collect()
    };
    let old = rows(&[
        ("b", ConsentRecall::HeaderOnly),
        ("c", ConsentRecall::Withheld),
    ]);
    let widening = ConsentPolicy::widening;
    assert!(widening(None, None).is_none() && widening(None, Some(&old)).is_none());
    assert!(widening(Some(&old), Some(&old)).is_none());
    assert!(widening(Some(&old), Some(&rows(&[("b", ConsentRecall::Withheld)]))).is_none());
    assert!(
        widening(Some(&old), None).is_some(),
        "dropping the declaration is accept-all"
    );
    assert!(widening(Some(&old), Some(&rows(&[("b", ConsentRecall::Full)]))).is_some());
    assert!(widening(Some(&old), Some(&rows(&[("c", ConsentRecall::HeaderOnly)]))).is_some());
    assert!(widening(Some(&old), Some(&rows(&[("d", ConsentRecall::Withheld)]))).is_some());

    let w = world();
    w.declare("a", &[("b", ConsentRecall::HeaderOnly)]);
    let wider = rows(&[("b", ConsentRecall::Full)]);
    assert!(w
        .bus
        .consent_policy()
        .widened_by("a", Some(&wider))
        .is_some());
    assert!(w
        .bus
        .consent_policy()
        .widened_by("d", Some(&wider))
        .is_none());
}
