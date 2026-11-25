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
    let mut handle = bus.register_spirit(&SpiritId::from("recipient")).unwrap();
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
    bus.register_promotion_set_for_pid("recipient", 2, Some(review));
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
    let record: serde_json::Value = serde_json::from_slice(&denial.payload_redacted).unwrap();
    assert_eq!(record["recipient"], "recipient");
    assert_eq!(record["sender"], "sender");
    assert_eq!(
        record["frame_id"],
        ulid::Ulid::from_bytes([1; 16]).to_string()
    );
    assert_eq!(
        record["digest_frame_id"],
        ulid::Ulid::from_bytes(digest).to_string()
    );
    assert_eq!(record["cause"], "lineage-denied");
    assert!(!rows.iter().any(|row| row.frame_id == [1; 16]));
    assert!(matches!(handle.try_recv(), Ok(None)));
    let mut consult = AllowedPromotionSet::new();
    consult.insert(A2AIntent::new("consult"));
    bus.register_promotion_set_for_pid("recipient", 2, Some(consult));
    assert!(bus.deliver_typed(frame(2, digest), 0, None).await.is_ok());
    assert!(matches!(handle.try_recv(), Ok(Some(_))));
    bus.unregister_promotion_set_for_pid("recipient", 2);
    assert!(bus.deliver_typed(frame(3, digest), 0, None).await.is_ok());
    assert!(matches!(handle.try_recv(), Ok(Some(_))));
    let mut source_frame = frame(4, digest);
    if let FramePayload::TaskAssign(task) = &mut source_frame.payload {
        task.prior_distillate_ref = None;
    }
    bus.deliver_typed(source_frame, 0, None).await.unwrap();
    assert!(matches!(handle.try_recv(), Ok(Some(_))));
    let request = DistillationRequest::new(
        vec![[4; 16]],
        1,
        DigestPayload::Text("routed digest".into()),
        None,
    )
    .unwrap();
    let routed_digest = writer.write_distillate(0, request).unwrap().digest_frame_id;
    let mut standard = AllowedPromotionSet::new();
    standard.insert(A2AIntent::new("standard"));
    bus.register_promotion_set_for_pid("recipient", 2, Some(standard));
    assert!(bus
        .deliver_typed(frame(5, routed_digest), 0, None)
        .await
        .is_ok());
    assert!(matches!(handle.try_recv(), Ok(Some(_))));
    assert!(matches!(
        bus.deliver_typed(frame(6, [42; 16]), 0, None).await,
        Err(IacBusError::PromotionDistillateNotFound)
    ));
    assert!(matches!(handle.try_recv(), Ok(None)));
}

#[test]
fn promotion_registration_returns_old_set_only_to_same_owner() {
    let bus = IacBusAdapter::default();
    let mut old = AllowedPromotionSet::new();
    old.insert(A2AIntent::new("review"));
    assert!(bus
        .register_promotion_set_for_pid("r", 1, Some(old.clone()))
        .is_none());
    let replaced = bus.register_promotion_set_for_pid("r", 1, Some(AllowedPromotionSet::new()));
    assert_eq!(replaced, Some(old));
    // A different PID replacing the entry never receives the old owner's set.
    assert!(bus
        .register_promotion_set_for_pid("r", 2, Some(AllowedPromotionSet::new()))
        .is_none());
    assert!(bus.register_promotion_set_for_pid("r", 1, None).is_none());
    assert!(!bus.has_promotion_set("r"));
}

#[test]
fn restore_promotion_set_replaces_only_an_empty_or_owned_slot() {
    let bus = IacBusAdapter::default();
    let set = |intent: &str| {
        let mut set = AllowedPromotionSet::new();
        set.insert(A2AIntent::new(intent));
        set
    };
    // An empty slot takes the set, owned by the restoring PID.
    bus.restore_promotion_set("r", 1, set("review"));
    assert!(bus.has_promotion_set("r"));
    // An owned slot is replaced in place.
    bus.restore_promotion_set("r", 1, set("consult"));
    assert_eq!(
        bus.register_promotion_set_for_pid("r", 1, Some(set("standard"))),
        Some(set("consult"))
    );
    // A slot another PID owns is left untouched.
    bus.register_promotion_set_for_pid("r", 2, Some(set("delegate")));
    bus.restore_promotion_set("r", 1, set("review"));
    assert_eq!(
        bus.register_promotion_set_for_pid("r", 2, None),
        Some(set("delegate"))
    );
}
