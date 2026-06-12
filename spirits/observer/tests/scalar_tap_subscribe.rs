//! AC2 — Observer subscribes BROADLY to the `scalar.tap` Telemetry Stream and
//! receives peer scalars, with a CLIENT-SIDE principal-namespace filter (FR31,
//! Decision C — the `TelemetryStreamPort` has only exact-topic subscription, so
//! the namespace scoping lives Spirit-side).
//!
//! Proven against the REAL `TelemetryStreamAdapter` driven as a dev-dep —
//! mirrors `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`.

use std::sync::Arc;
use std::time::Duration;

use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::ports::TelemetryStreamPort;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

use observer::{DriftDirection, Observer, PrincipalScope, WatchThreshold};

#[tokio::test]
async fn observer_subscribes_and_receives_peer_scalars() {
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let topic = TelemetryTopic::new("scalar.tap.belief_variance");

    let observer = Observer::watching(
        PrincipalScope::from_patterns(["mira"]),
        vec![WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, 0.15).unwrap()],
    );

    // Observer establishes its per-Spirit subscription (I7).
    assert!(
        telemetry.subscribe_topic(observer.observer_id(), &topic),
        "first subscribe returns true"
    );
    assert!(
        !telemetry.subscribe_topic(observer.observer_id(), &topic),
        "re-subscribe by the same Spirit returns false"
    );
    assert!(
        telemetry.subscribe_topic("some-other-spirit", &topic),
        "a different Spirit subscribing the same topic returns true"
    );

    let mut rx = telemetry.subscribe(&topic).expect("receiver");

    // A peer publishes a scalar; Observer receives it within the 500ms bound
    // (≥500ms per the 8.2 CI-flake fix).
    let pub_telemetry = Arc::clone(&telemetry);
    let pub_topic = topic.clone();
    let writer = tokio::spawn(async move {
        pub_telemetry.publish_event(
            &pub_topic,
            ScalarTapEvent {
                spirit_id: "mira".into(),
                tag: "belief_variance".into(),
                value: 0.42,
                timestamp: 1,
            },
        );
    });

    let received = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("scalar received within 500ms")
        .expect("event");
    writer.await.unwrap();
    assert_eq!(received.spirit_id, "mira");
    assert_eq!(received.tag, "belief_variance");
    assert_eq!(received.value, 0.42);
    assert!(received.timestamp > 0);
}

#[tokio::test]
async fn fr31_client_side_filter_drops_out_of_namespace_emitters() {
    let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
    let topic = TelemetryTopic::new("scalar.tap.belief_variance");

    // Observer's principal scope admits only `mira`.
    let observer = Observer::watching(
        PrincipalScope::from_patterns(["mira"]),
        vec![WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, 0.15).unwrap()],
    );
    telemetry.subscribe_topic(observer.observer_id(), &topic);
    let mut rx = telemetry.subscribe(&topic).expect("receiver");

    // Two peers publish the SAME in-band value; only `mira` is in-namespace.
    for spirit in ["mira", "stranger"] {
        telemetry.publish_event(
            &topic,
            ScalarTapEvent {
                spirit_id: spirit.into(),
                tag: "belief_variance".into(),
                value: 0.66,
                timestamp: 1,
            },
        );
    }

    // The transport delivers BOTH events (broadcast is not namespace-aware);
    // Observer's CLIENT-SIDE FR31 filter is what scopes them.
    let mut surfaced_subjects = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        // in_namespace is the FR31 gate; observe_scalar applies it internally too.
        if observer.in_namespace(&ev.spirit_id) {
            if let Some(surface) = observer.observe_scalar(&ev) {
                surfaced_subjects.push(surface.subject);
            }
        }
    }
    assert_eq!(
        surfaced_subjects,
        vec!["mira".to_string()],
        "only the in-namespace emitter is surfaced; 'stranger' is dropped (FR31)"
    );
}
