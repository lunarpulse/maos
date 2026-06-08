//! AC3 / AC5 — the pre-halt scalar-drift watchdog (the v0.5 headline demo).
//!
//! A peer's scalar runup is published over the REAL `TelemetryStreamAdapter`;
//! Observer detects the trajectory entering the watch-band approaching the halt
//! threshold and surfaces a drift early-warning **before** the threshold crosses,
//! dispatched through the REAL `maos-director-surface` `NotificationDispatcher`
//! into a capturing channel (Decision B — the operator-actionable surface §6.5
//! grants Observer). Fixture-driven from `tests/fixtures/drift-scenarios.json`.

use std::sync::{Arc, Mutex};

use serde::Deserialize;

use maos_director_surface::notification::{
    NotificationChannel, NotificationDispatcher, NotificationError, NotificationLevel,
    NotificationSurface,
};
use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::notification::{NotificationEvent, NotificationEventError};
use maos_domain::ports::TelemetryStreamPort;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

use observer::{DriftDirection, Observer, PrincipalScope, WatchThreshold};

// ── a capturing operator channel (the test's NotificationChannel) ────────────

struct CapturingChannel {
    sink: Arc<Mutex<Vec<NotificationEvent>>>,
}

impl NotificationChannel for CapturingChannel {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::Terminal
    }
    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        self.sink.lock().unwrap().push(event.clone());
        Ok(())
    }
}

// ── fixture shapes ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DriftScenario {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    note: String,
    tag: String,
    threshold: f64,
    direction: DriftDirection,
    warn_margin: f64,
    namespace: Vec<String>,
    events: Vec<EventFixture>,
    expect_warnings: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventFixture {
    spirit_id: String,
    tag: String,
    value: f64,
}

fn load_scenarios() -> Vec<DriftScenario> {
    let raw = include_str!("fixtures/drift-scenarios.json");
    serde_json::from_str(raw).expect("drift-scenarios.json parses")
}

/// Whether `value` has NOT yet crossed the halt threshold (early-warning window).
fn is_pre_threshold(direction: DriftDirection, value: f64, threshold: f64) -> bool {
    match direction {
        DriftDirection::Above => value < threshold,
        DriftDirection::Below => value > threshold,
    }
}

#[tokio::test]
async fn drift_scenarios_warn_before_halt_via_the_real_dispatcher() {
    let scenarios = load_scenarios();
    assert!(scenarios.len() >= 4, "fixtures present");

    let mut saw_a_real_warning = false;

    for sc in &scenarios {
        let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
        let topic = TelemetryTopic::new(format!("scalar.tap.{}", sc.tag));

        let observer = Observer::watching(
            PrincipalScope::from_patterns(sc.namespace.clone()),
            vec![WatchThreshold::new(
                sc.tag.clone(),
                sc.threshold,
                sc.direction,
                sc.warn_margin,
            )
            .unwrap()],
        );
        telemetry.subscribe_topic(observer.observer_id(), &topic);
        let mut rx = telemetry.subscribe(&topic).expect("receiver");

        // Capturing operator surface behind the REAL dispatcher.
        let captured = Arc::new(Mutex::new(Vec::new()));
        let mut dispatcher = NotificationDispatcher::new();
        dispatcher.register(Box::new(CapturingChannel {
            sink: Arc::clone(&captured),
        }));

        // The peer publishes the runup (broadcast buffers it for the receiver).
        for (i, ev) in sc.events.iter().enumerate() {
            telemetry.publish_event(
                &topic,
                ScalarTapEvent {
                    spirit_id: ev.spirit_id.clone(),
                    tag: ev.tag.clone(),
                    value: ev.value,
                    timestamp: (i as u64) + 1,
                },
            );
        }

        // Observer drains the stream, classifies drift, and dispatches surfaces.
        let mut warned_values = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            let value = ev.value;
            if let Some(surface) = observer.observe_scalar(&ev) {
                let note = surface
                    .to_notification(observer.observer_id())
                    .expect("valid anomaly notification");
                let report = dispatcher
                    .dispatch(note, NotificationLevel::Immediate)
                    .expect("dispatch");
                assert_eq!(report.delivered, 1, "{}: surfaced to the operator", sc.name);
                warned_values.push(value);
            }
        }

        // The number of operator-surfaced early-warnings matches the fixture.
        let delivered = captured.lock().unwrap().len();
        assert_eq!(
            delivered, sc.expect_warnings,
            "{}: expected {} early-warning(s), got {}",
            sc.name, sc.expect_warnings, delivered
        );

        // Every early-warning fired BEFORE the threshold crossed (the operator
        // can intervene before the halt) — the I7 "witness the runup" obligation.
        for v in &warned_values {
            assert!(
                is_pre_threshold(sc.direction, *v, sc.threshold),
                "{}: warning at {v} must be pre-threshold {} (dir {:?})",
                sc.name,
                sc.threshold,
                sc.direction
            );
        }
        if !warned_values.is_empty() {
            saw_a_real_warning = true;
        }

        // Every dispatched event is the AnomalyFlagged variant (Decision B).
        for ev in captured.lock().unwrap().iter() {
            assert!(
                matches!(ev, NotificationEvent::AnomalyFlagged { .. }),
                "{}: surfaced via AnomalyFlagged (no new FrameKind)",
                sc.name
            );
        }
    }

    assert!(
        saw_a_real_warning,
        "at least one scenario must surface a pre-halt drift early-warning"
    );
}

#[test]
fn anomaly_constructor_rejects_invalid_surfaces() {
    // AC5 — the validated constructor is the only construction path; struct
    // literals (which bypass these checks) are never used in production code.
    assert!(matches!(
        NotificationEvent::anomaly_flagged("observer", "mira", "", 0.5),
        Err(NotificationEventError::EmptySummary)
    ));
    assert!(matches!(
        NotificationEvent::anomaly_flagged("observer", "mira", "drift", f32::NAN),
        Err(NotificationEventError::NanConfidence)
    ));
    assert!(matches!(
        NotificationEvent::anomaly_flagged("observer", "mira", "drift", 1.5),
        Err(NotificationEventError::ConfidenceOutOfRange)
    ));
}
