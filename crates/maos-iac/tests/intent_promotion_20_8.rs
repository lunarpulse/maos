use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, PriorDistillateRef, TaskAssignPayload,
};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::{
    i1::IntentClass,
    i13::{AllowedPromotionSet, IntentLineage},
    i3::FrameOrigin,
    i8::A2AIntent,
};
use maos_domain::ports::{DistillationPort, IacBusPort};
use maos_iac::adapter::{
    distillate::DistillateWriter, FrameFilter, FrameKind as LogKind, IacBusAdapter, IacRtMetrics,
    Mailbox, TransparencyLogAdapter,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use std::sync::Arc;

fn frame(id: u8, digest: [u8; 16]) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 1,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("sender"),
            host_id: None,
            role: None,
        },
        to: smallvec::smallvec![FrameAddress {
            spirit_id: SpiritId::from("recipient"),
            host_id: None,
            role: None
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "consume".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: Some(PriorDistillateRef {
                digest_frame_id: digest,
                distillation_depth: 1,
                intent_lineage: IntentLineage::default(),
            }),
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

#[tokio::test]
async fn recipient_promotion_set_checks_stored_lineage_before_enqueue() {
    maos_capability::cap_tokens::init_monotonic_base();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(208));
    let bus = IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new()))),
        Arc::clone(&log),
    );
    let _handle = bus.register_spirit(&SpiritId::from("recipient")).unwrap();
    log.insert_frame_event(
        LogKind::TaskAssign,
        1,
        None,
        "consult",
        b"source",
        FrameOrigin::HumanAuthored,
    );
    let source = log.last_frame_id();
    let writer = DistillateWriter::new(Arc::clone(&log), Arc::new(()));
    let request =
        DistillationRequest::new(vec![source], 1, DigestPayload::Text("digest".into()), None)
            .unwrap();
    let digest = writer.write_distillate(1, request).unwrap().digest_frame_id;
    let mut review = AllowedPromotionSet::new();
    review.insert(A2AIntent::new("review"));
    bus.register_promotion_set("recipient", Some(review));
    assert!(matches!(
        bus.deliver_typed(frame(1, digest), 0, None).await,
        Err(IacBusError::IntentPromotionDenied { .. })
    ));
    let rows = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row.intent == "iac:intent-promotion-denied")
            .count(),
        1
    );
    let denial = rows
        .iter()
        .find(|row| row.intent == "iac:intent-promotion-denied")
        .unwrap();
    assert_eq!(denial.kind, LogKind::TelemetryEvent);
    assert!(denial.capability_token.is_none());
    assert!(!rows.iter().any(|row| row.frame_id == [1; 16]));
    let mut consult = AllowedPromotionSet::new();
    consult.insert(A2AIntent::new("consult"));
    bus.register_promotion_set("recipient", Some(consult));
    assert!(bus.deliver_typed(frame(2, digest), 0, None).await.is_ok());
    bus.unregister_promotion_set("recipient");
    assert!(bus.deliver_typed(frame(3, digest), 0, None).await.is_ok());
}
