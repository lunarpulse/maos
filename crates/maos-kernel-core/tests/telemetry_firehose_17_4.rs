#![forbid(unsafe_code)]

//! Story 17-4 (D-17-4-PF-R1) — the broad `scalar.tap` channel.
//!
//! Every assertion runs through the production write path
//! (`CapabilityRegistryAdapter::set_scalar` → store `seq` → `publish_event`)
//! and reads the one receiver `subscribe_all()` returns. No exact-topic
//! subscription is declared anywhere: the broad channel must not depend on
//! one.

use std::sync::Arc;

use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::TelemetryStreamPort;
use maos_kernel_core::capability::{
    cap_policy::PolicyTable, cap_quota::CapQuotaTracker, cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

fn registry(telemetry: Arc<TelemetryStreamAdapter>) -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let (audit_tx, _) = maos_kernel_core::capability::cap_audit::channel();
    CapabilityRegistryAdapter::new(
        crypto,
        Ed25519SigningKey::new([0u8; 32]),
        0x17_04,
        Arc::new(PolicyTable::new()),
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        telemetry,
    )
}

fn event(seq: u64, tag: &str) -> ScalarTapEvent {
    ScalarTapEvent {
        seq,
        spirit_id: "emitter".into(),
        tag: tag.into(),
        value: seq as f64,
        timestamp: 0,
    }
}

#[test]
fn one_emitter_alternating_tags_arrive_in_program_order_on_subscribe_all() {
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let registry = registry(Arc::clone(&telemetry));
    let mut rx = telemetry.subscribe_all();

    // 1 500 writes, alternating two tags no Spirit ever subscribed to, below
    // the 2 048 capacity so nothing may be lagged.
    let mut written = Vec::with_capacity(1_500);
    for i in 0..1_500_u32 {
        let tag = if i % 2 == 0 { "tag_a" } else { "tag_b" };
        let value = f64::from(i) * 0.1 + f64::from(0.8_f32);
        written.push(
            registry
                .set_scalar(9, "emitter", tag, value, "test")
                .unwrap(),
        );
    }

    for (index, expected) in written.iter().enumerate() {
        let got = rx
            .try_recv()
            .unwrap_or_else(|error| panic!("event {index} missing: {error:?}"));
        assert_eq!(got.seq, expected.seq, "event {index} out of order");
        assert_eq!(got.tag, expected.tag, "event {index} tag");
        assert_eq!(got.spirit_id, "emitter");
        assert_eq!(
            got.value.to_bits(),
            expected.value.to_bits(),
            "event {index} value"
        );
    }
    assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);

    // The store labels writes 0..n in write order.
    let seqs: Vec<u64> = written.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (0..1_500_u64).collect::<Vec<_>>());
}

#[tokio::test]
async fn an_overflowing_broad_receiver_reports_the_exact_lag_then_resumes_in_order() {
    let telemetry = TelemetryStreamAdapter::new(2);
    let mut rx = telemetry.subscribe_all();
    let topic = TelemetryTopic::new("scalar.tap.undeclared");
    for seq in 0..5 {
        let tag = if seq % 2 == 0 { "tag_a" } else { "tag_b" };
        telemetry.publish_event(&topic, event(seq, tag));
    }
    assert_eq!(rx.recv().await.unwrap_err(), RecvError::Lagged(3));
    assert_eq!(rx.recv().await.unwrap().seq, 3);
    assert_eq!(rx.recv().await.unwrap().seq, 4);
    assert_eq!(rx.try_recv().unwrap_err(), TryRecvError::Empty);
}

#[tokio::test]
async fn the_exact_topic_path_is_unchanged_beside_the_broad_channel() {
    let telemetry = TelemetryStreamAdapter::default();
    let mut all = telemetry.subscribe_all();
    let declared = TelemetryTopic::new("scalar.tap.declared");
    assert!(telemetry.subscribe_topic("watcher", &declared));
    let mut narrow = telemetry
        .subscribe(&declared)
        .expect("declared topic channel");

    telemetry.publish_event(&declared, event(0, "declared"));
    telemetry.publish_event(&TelemetryTopic::new("scalar.tap.other"), event(1, "other"));

    assert_eq!(narrow.recv().await.unwrap().seq, 0);
    assert_eq!(narrow.try_recv().unwrap_err(), TryRecvError::Empty);
    assert_eq!(all.recv().await.unwrap().seq, 0);
    assert_eq!(all.recv().await.unwrap().seq, 1);
}

#[test]
fn clones_share_one_broad_channel() {
    let telemetry = TelemetryStreamAdapter::default();
    let mut rx = telemetry.subscribe_all();
    let clone = telemetry.clone();
    clone.publish_event(&TelemetryTopic::new("scalar.tap.x"), event(7, "x"));
    assert_eq!(rx.try_recv().unwrap().seq, 7);
}
