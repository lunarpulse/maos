#![forbid(unsafe_code)]
//! Story 17-8 AC3.5 (D-17-8-03 A) — the production rupture producer: an A2A
//! accept-policy denial journaled by `journal_rupture_frame` withholds, from the
//! rejected local incarnation, the earlier frames its peer granter sent it under
//! the same intent class — and nothing else.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use maos_cohort::journal_rupture_frame;
use maos_domain::frame::{
    ConsentEnvelope, ConsentRecall, ConsentRupturePayload, FrameAddress, FramePayload, IacFrame,
    RuptureReason, RuptureRejection, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::log_recall::LogRecallError;
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{FrameKind, FrameRecipient, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind as WireKind, HostId, SpiritId};

const C: u32 = 7;
const INTENT: &str = "diagnosis-handoff:read-only-evidence";

fn address(spirit: &str, host: Option<&str>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(spirit),
        host_id: host.map(|host| HostId(host.into())),
        role: None,
    }
}

fn granted(granter: FrameAddress, intent: &str) -> ConsentEnvelope {
    ConsentEnvelope {
        consent_id: [0xC0; 16],
        granter,
        timestamp_ns: 1,
        intent_class: Some(A2AIntent::new(intent)),
        valid_until_ns: None,
        recall: ConsentRecall::Full,
    }
}

fn frame(id: u8, from: FrameAddress, to: FrameAddress, kind: WireKind) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from,
        to: std::iter::once(to).collect(),
        kind,
        intent: IntentClass::Readonly,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "peer delegation".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

/// The router's rupture frame for a denied delivery (`router.rs:446-474`): from
/// the denied first addressee (this host's id as the peer wrote it), to the
/// denied frame's sender, with the denied frame's envelope cloned.
fn router_rupture(id: u8, envelope: ConsentEnvelope) -> IacFrame {
    let denier = address("c", Some("host-a"));
    let mut rupture = frame(
        id,
        denier.clone(),
        address("peer", Some("host-b")),
        WireKind::ConsentRupture,
    );
    rupture.payload = FramePayload::ConsentRupture(ConsentRupturePayload {
        rupture_id: [id; 16],
        original_frame_id: [0xEE; 16],
        original_kind: WireKind::TaskAssign,
        accepted: vec![],
        rejected: vec![RuptureRejection {
            address: denier,
            reason: RuptureReason::IntentAllowlistMismatch,
        }],
        ruptured_at_ns: 0,
    });
    rupture.consent_envelope = Some(envelope);
    rupture
}

fn received(log: &TransparencyLogAdapter, id: u8, envelope: ConsentEnvelope) {
    let mut delivered = frame(
        id,
        address("peer", Some("host-b")),
        address("c", None),
        WireKind::TaskAssign,
    );
    delivered.consent_envelope = Some(envelope);
    let recipients = [FrameRecipient {
        pid: C,
        spirit_id: "c".into(),
        role: None,
    }];
    log.insert_routed_frame(
        &delivered,
        FrameKind::TaskAssign,
        0,
        None,
        "readonly",
        b"x",
        &recipients,
    );
}

#[test]
fn router_rupture_withholds_only_its_own_granters_same_intent_frames() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(9));
    let admitted = Arc::new(RwLock::new(BTreeMap::from([("c".to_string(), C)])));
    log.install_recipient_resolver(admitted as _).unwrap();
    let peer = address("peer", Some("host-b"));
    received(&log, 1, granted(peer.clone(), INTENT));
    received(&log, 2, granted(peer.clone(), "other:intent"));
    let recall = LogRecallAdapter::new(Arc::clone(&log));
    assert!(recall.fetch(C, [1; 16]).is_ok());

    // An envelope NOT granted by the denied frame's sender records no grant.
    let stolen = granted(address("someone-else", Some("host-b")), INTENT);
    journal_rupture_frame(&log, &router_rupture(3, stolen)).unwrap();
    assert!(recall.fetch(C, [1; 16]).is_ok());

    journal_rupture_frame(&log, &router_rupture(4, granted(peer.clone(), INTENT))).unwrap();
    assert_eq!(
        recall.fetch(C, [1; 16]),
        Err(LogRecallError::ConsentWithheld { frame_id: [1; 16] })
    );
    // Exact intent class: the other-intent frame from the same peer is untouched,
    // and a frame received after the rupture is not withheld by it.
    assert!(recall.fetch(C, [2; 16]).is_ok());
    received(&log, 5, granted(peer, INTENT));
    assert!(recall.fetch(C, [5; 16]).is_ok());
}
