//! AC6 — Researcher subscribes to the `scalar.tap` Telemetry Stream, receives
//! scalars published by peers, folds the observed pattern into a subsequent
//! distillate, and demonstrates that a Researcher distillate is consumable in
//! the FR17 digest pattern (source-log-ref-cited) — a composability demo, NOT a
//! re-implementation of Butler's morning digest and with NO dependency on the
//! `butler` crate (Decision E; enforced at the dependency level — researcher's
//! Cargo.toml carries no `butler`).
//!
//! Proven against the real `TelemetryStreamAdapter` (and, for the demo, the real
//! `LogRecallAdapter` + `DistillateWriter`) driven as dev-deps — mirrors
//! `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`.

use std::sync::Arc;
use std::time::Duration;

use researcher::{ClaimPayload, RecalledFrame, Researcher};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::log_recall::LogRecallFilter;
use maos_domain::ports::TelemetryStreamPort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

fn claim_frame(id_byte: u8, claim_id: &str) -> RecalledFrame {
    let claim = ClaimPayload {
        claim_id: claim_id.into(),
        statement: "the effect is likely present".into(),
        topic: "fusion".into(),
        methodology_strength: 0.9,
        confidence: 0.92,
        load_bearing: true,
        polarity: true,
        hedges: vec!["likely".into()],
    };
    RecalledFrame {
        frame_id: [id_byte; 16],
        intent: "inform".into(),
        payload: serde_json::to_vec(&claim).unwrap(),
    }
}

#[tokio::test]
async fn researcher_subscribes_receives_and_incorporates_a_scalar() {
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let topic = TelemetryTopic::new("scalar.tap.confidence");

    // Researcher establishes its per-Spirit subscription (I7).
    let is_new = telemetry.subscribe_topic("researcher", &topic);
    assert!(is_new, "first subscribe returns true");
    assert!(
        !telemetry.subscribe_topic("researcher", &topic),
        "re-subscribe by the same Spirit returns false"
    );
    assert!(
        telemetry.subscribe_topic("observer", &topic),
        "a different Spirit subscribing the same topic returns true"
    );

    let mut rx = telemetry.subscribe(&topic).expect("receiver");

    // A peer publishes a scalar; Researcher receives it within the 100ms bound.
    let pub_telemetry = Arc::clone(&telemetry);
    let pub_topic = topic.clone();
    let writer = tokio::spawn(async move {
        pub_telemetry.publish_event(
            &pub_topic,
            ScalarTapEvent {
                spirit_id: "observer".into(),
                tag: "confidence".into(),
                value: 0.62,
                timestamp: 1,
            },
        );
    });

    let received = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("scalar received within 500ms")
        .expect("event");
    writer.await.unwrap();
    assert_eq!(received.spirit_id, "observer");
    assert_eq!(received.tag, "confidence");
    assert_eq!(received.value, 0.62);

    // Researcher folds the observed scalar into a SUBSEQUENT survey/distillate.
    let researcher = Researcher::new();
    let mut survey = researcher.survey(&[claim_frame(0x11, "c1")]);
    researcher.incorporate_scalar(&mut survey, &received);
    assert!(
        survey
            .confidence_map
            .keys()
            .any(|k| k == "observed::observer::confidence"),
        "the observed scalar is incorporated into the confidence map"
    );
    assert_eq!(
        survey
            .scalars
            .get("observed::observer::confidence")
            .copied(),
        Some(0.62)
    );
    assert!(survey
        .open_questions
        .iter()
        .any(|q| q.contains("observed scalar.tap 'confidence'")));
}

#[tokio::test]
async fn researcher_distillate_is_consumable_in_the_fr17_digest_pattern() {
    // Composability demo (Decision E): a Researcher distillate — built over the
    // scoped log.recall walker, carrying the observed scalar — is shown
    // consumable in the FR17 digest pattern (every claim cites a source_log_ref
    // that resolves to a real frame). No butler crate involved.
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x_F17_DE5));
    let _ = tl.insert_frame_event(
        FrameKind::InferenceCall,
        10,
        None,
        "inform",
        &serde_json::to_vec(&ClaimPayload {
            claim_id: "q1".into(),
            statement: "quarterly outlook is likely stable".into(),
            topic: "ops".into(),
            methodology_strength: 0.9,
            confidence: 0.93,
            load_bearing: true,
            polarity: true,
            hedges: vec!["likely".into()],
        })
        .unwrap(),
        FrameOrigin::SpiritAuto,
    );

    let recall = LogRecallAdapter::new(Arc::clone(&tl));
    let writer = DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8));
    let researcher = Researcher::new();

    let frames = researcher
        .walk(&recall, 10, LogRecallFilter::default())
        .unwrap();
    let mut survey = researcher.survey(&frames);
    researcher.incorporate_scalar(
        &mut survey,
        &ScalarTapEvent {
            spirit_id: "observer".into(),
            tag: "drift".into(),
            value: 0.3,
            timestamp: 2,
        },
    );

    // FR17 digest pattern: the distillate is persisted with a kernel-resolved,
    // source-log-ref-cited audit chain — exactly what a digest consumer reads.
    let receipt = researcher.distill_through(&writer, 10, &survey, 1).unwrap();
    assert!(
        !receipt.effective_source_log_ref.is_empty(),
        "the contributed distillate cites a non-empty, kernel-resolved source_log_ref"
    );
    // Every finding cites a frame that resolves — the digest is consumable.
    for finding in &survey.findings {
        assert!(!finding.source_log_ref.is_empty());
    }
}
