//! IAC Bus intent-lineage integration test (AC4).
//!
//! Verifies the NFR-Aud-14 intent-lineage propagation at the IAC bus:
//! - Human-authored cross-Spirit frame with empty lineage → kernel auto-populates
//! - Spirit-auto cross-Spirit frame with empty lineage → `EIntentLineageBroken`
//! - Same-Spirit and broadcast frames bypass the check per ADR-018.

use std::sync::Arc;

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload, PosturePreferences};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::ports::IacBusPort;
use maos_kernel_core::iac::{
    IacBusAdapter, Mailbox, TransparencyLogAdapter,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

fn make_frame(from: &str, to: &[&str], origin: FrameOrigin) -> IacFrame {
    let addresses: smallvec::SmallVec<[FrameAddress; 1]> = to
        .iter()
        .map(|s| FrameAddress {
            spirit_id: SpiritId::from(*s),
            host_id: None,
            role: None,
        })
        .collect();
    IacFrame {
        frame_id: [0xAB; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(from),
            host_id: None,
            role: None,
        },
        to: addresses,
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "lineage test".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
        }),
        auto_marker: origin,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

#[tokio::test]
async fn human_authored_cross_spirit_auto_populates_lineage() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);
    let _handle = adapter.register_spirit(&SpiritId::from("spirit-b")).unwrap();

    let frame = make_frame("spirit-a", &["spirit-b"], FrameOrigin::HumanAuthored);
    let result = adapter.deliver(frame).await;
    assert!(result.is_ok(), "human-authored cross-spirit should succeed");
}

#[tokio::test]
async fn spirit_auto_cross_spirit_empty_lineage_rejected() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);
    let _handle = adapter.register_spirit(&SpiritId::from("spirit-b")).unwrap();

    let frame = make_frame("spirit-a", &["spirit-b"], FrameOrigin::SpiritAuto);
    let result = adapter.deliver(frame).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        IacBusError::EIntentLineageBroken { from, to, origin } => {
            assert_eq!(from, "spirit-a");
            assert_eq!(to, "spirit-b");
            assert_eq!(origin, FrameOrigin::SpiritAuto);
        }
        e => panic!("expected EIntentLineageBroken, got {e:?}"),
    }
}

#[tokio::test]
async fn spirit_auto_cross_spirit_with_lineage_succeeds() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);
    let _handle = adapter.register_spirit(&SpiritId::from("spirit-b")).unwrap();

    let mut frame = make_frame("spirit-a", &["spirit-b"], FrameOrigin::SpiritAuto);
    frame.intent_lineage = IntentLineage::new(vec![A2AIntent::new("standard")]);
    let result = adapter.deliver(frame).await;
    assert!(result.is_ok(), "spirit-auto with lineage should succeed");
}

#[tokio::test]
async fn same_spirit_empty_lineage_succeeds() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);
    let _handle = adapter.register_spirit(&SpiritId::from("spirit-a")).unwrap();

    let frame = make_frame("spirit-a", &["spirit-a"], FrameOrigin::SpiritAuto);
    let result = adapter.deliver(frame).await;
    assert!(result.is_ok(), "same-spirit should bypass lineage check");
}

#[tokio::test]
async fn broadcast_empty_to_succeeds() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);

    let frame = make_frame("spirit-a", &[], FrameOrigin::SpiritAuto);
    let result = adapter.deliver(frame).await;
    assert!(result.is_ok(), "broadcast should bypass lineage check");
}

#[tokio::test]
async fn re_emission_preserves_lineage() {
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let adapter = IacBusAdapter::new(mailbox.clone(), log);
    let _handle = adapter.register_spirit(&SpiritId::from("spirit-b")).unwrap();

    let mut frame = make_frame("spirit-a", &["spirit-b"], FrameOrigin::SpiritAuto);
    frame.intent_lineage = IntentLineage::new(vec![
        A2AIntent::new("standard"),
        A2AIntent::new("consult"),
    ]);
    let result = adapter.deliver(frame).await;
    assert!(result.is_ok(), "spirit-auto with lineage should succeed (re-emission case)");
}
