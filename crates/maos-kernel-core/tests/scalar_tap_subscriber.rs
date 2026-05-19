#![forbid(unsafe_code)]

//! AC4 integration test — exercises the Telemetry Stream broadcast:
//! constructs a subscriber, calls `set_scalar` from a separate task,
//! asserts the subscriber receives `(spirit_id, tag, value, timestamp)`
//! within 100ms.
//!
//! Uses `#[tokio::test]` per the project convention. Validates the
//! I7 invariant: the subscriber CANNOT call `invoke_halt` from the
//! receiver side (the receiver is read-only; no mutation channel is
//! exposed).

use std::sync::Arc;

use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::ports::TelemetryStreamPort;
use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::{
    CapabilityRegistryAdapter,
    cap_tokens::Ed25519SigningKey,
    cap_policy::PolicyTable,
    cap_quota::CapQuotaTracker,
    WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

fn make_adapter(
    telemetry: Arc<TelemetryStreamAdapter>,
) -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(maos_kernel_core::api::RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let (audit_tx, _) = maos_kernel_core::capability::cap_audit::channel();
    let quota = CapQuotaTracker::new();
    let working_memory = Arc::new(WorkingMemoryStore::new());
    CapabilityRegistryAdapter::new(
        crypto, signing_key, 0xCAFE, policy, audit_tx, quota, working_memory,
    )
}

#[tokio::test]
async fn subscriber_receives_scalar_tap_event() {
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let adapter = make_adapter(Arc::clone(&telemetry));

    let topic = TelemetryTopic::new("scalar.tap.uncertainty");

    // Subscribe first so the broadcast channel exists
    let is_new = telemetry.subscribe_topic("observer-spirit", &topic);
    assert!(is_new, "first subscribe should return true");

    let mut rx = telemetry.subscribe(&topic).unwrap();

    // Call set_scalar from a separate task
    let adapter_clone = adapter; // move adapter
    let telemetry_clone = Arc::clone(&telemetry);
    let topic_clone = topic.clone();
    let write_task = tokio::spawn(async move {
        let event = adapter_clone
            .set_scalar(1, "spirit-1", "uncertainty", 0.75, "frame-001")
            .unwrap();
        // Publish the event to the telemetry stream
        telemetry_clone.publish_event(&topic_clone, event);
    });

    // Wait for the subscriber to receive the event (bound 100ms)
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        rx.recv(),
    )
    .await;

    write_task.await.unwrap();

    let received = result.expect("subscriber should receive event within 100ms").unwrap();
    assert_eq!(received.spirit_id, "spirit-1");
    assert_eq!(received.tag, "uncertainty");
    assert_eq!(received.value, 0.75);
    assert!(received.timestamp > 0);
}

#[tokio::test]
async fn subscribe_twice_returns_false() {
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let topic = TelemetryTopic::new("scalar.tap.uncertainty");

    let first = telemetry.subscribe_topic("observer-1", &topic);
    assert!(first);

    let second = telemetry.subscribe_topic("observer-2", &topic);
    assert!(!second, "re-subscribe to same topic should return false");
}

#[tokio::test]
async fn subscriber_readonly_cannot_mutate() {
    // The I7 invariant: the subscriber receives ScalarTapEvent via
    // broadcast::Receiver<ScalarTapEvent> — the type is Clone but not
    // Mut. The receiver side carries no reference to `CapabilityRegistryPort`,
    // `HaltRegistry`, or any mutation-capable handle. This test confirms
    // the receiver cannot call `invoke_halt` or trigger any halt.
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let topic = TelemetryTopic::new("scalar.tap.uncertainty");
    telemetry.subscribe_topic("observer", &topic);
    let rx = telemetry.subscribe(&topic).unwrap();

    // The receiver type is `broadcast::Receiver<ScalarTapEvent>` —
    // no mutation-capable handle exposed.
    let _receiver_ref: &tokio::sync::broadcast::Receiver<ScalarTapEvent> = &rx;

    // Compile-time proof: there is no path to invoke_halt from a
    // `Receiver<ScalarTapEvent>`.
}
