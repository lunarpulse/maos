#![forbid(unsafe_code)]

//! Telemetry Stream — internal module at v0.3 per §4.7.
//!
//! Broadcasts events to subscribed Spirits. Story 4.2 replaces the
//! v0.1-α zero-size placeholder with a `tokio::sync::broadcast`-backed
//! adapter implementing `TelemetryStreamPort`.
//!
//! ## Architecture references
//! - ADR-035 (Observer Scalar Trajectory Channel): `scalar.tap` stream
//! - §4.7 (line 446): "Every `working_memory.set_scalar(tag, value,
//!   derived_from)` write emits a `scalar.tap` event."
//! - I7: Telemetry is broadcast; subscription is per-Spirit.

pub mod iac_rt;

pub use maos_domain::ports::TelemetryStreamPort;

use std::sync::Arc;

use dashmap::DashMap;
use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};

/// Telemetry Stream adapter — broadcasts `ScalarTapEvent`s via
/// `tokio::sync::broadcast` per-topic channels with capacity 2048.
///
/// ### Topic convention
/// `scalar.tap.<tag>` per ADR-035 — one broadcast channel per scalar tag.
///
/// ### Backpressure
/// Broadcast is lossy on slow consumers (Tokio broadcast semantics).
/// Backlog overflow is the consumer's problem — the kernel does NOT
/// block the emission path.
#[maos_attrs::i9_exempt(
    reason = "telemetry stream — per-process broadcast channel state for ADR-035 scalar.tap; parallel to IacRtMetrics, no persistence across restarts"
)]
#[derive(Debug)]
pub struct TelemetryStreamAdapter {
    topics: Arc<DashMap<TelemetryTopic, tokio::sync::broadcast::Sender<ScalarTapEvent>>>,
    subscribers: Arc<DashMap<(String, TelemetryTopic), ()>>,
    capacity: usize,
    drop_count: std::sync::atomic::AtomicUsize,
}

impl Clone for TelemetryStreamAdapter {
    fn clone(&self) -> Self {
        Self {
            topics: Arc::clone(&self.topics),
            subscribers: Arc::clone(&self.subscribers),
            capacity: self.capacity,
            drop_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Default for TelemetryStreamAdapter {
    fn default() -> Self {
        Self::new(2048)
    }
}

impl TelemetryStreamAdapter {
    /// Create an adapter with the given per-topic channel capacity.
    /// Capacity defaults to 2048 — sized for Mira-class diagnostic
    /// Spirit scalar drift fanout.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "broadcast capacity must be > 0");
        Self {
            topics: Arc::new(DashMap::new()),
            subscribers: Arc::new(DashMap::new()),
            capacity,
            drop_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl TelemetryStreamPort for TelemetryStreamAdapter {
    fn publish_event(&self, topic: &TelemetryTopic, event: ScalarTapEvent) {
        if let Some(sender) = self.topics.get(topic) {
            if sender.send(event).is_err() {
                self.drop_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            // No topic channel exists — count as dropped
            self.drop_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn subscribe_topic(&self, spirit_id: &str, topic: &TelemetryTopic) -> bool {
        use dashmap::mapref::entry::Entry;

        // Ensure broadcast channel exists for this topic
        match self.topics.entry(topic.clone()) {
            Entry::Vacant(entry) => {
                let (tx, _rx) = tokio::sync::broadcast::channel(self.capacity);
                entry.insert(tx);
            }
            Entry::Occupied(_) => {}
        }

        // Track per-spirit subscription
        match self
            .subscribers
            .entry((spirit_id.to_string(), topic.clone()))
        {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(());
                true
            }
        }
    }
}

impl TelemetryStreamAdapter {
    /// Get a receiver for a topic. Returns `None` if the topic hasn't
    /// been subscribed to yet.
    pub fn subscribe(
        &self,
        topic: &TelemetryTopic,
    ) -> Option<tokio::sync::broadcast::Receiver<ScalarTapEvent>> {
        self.topics.get(topic).map(|sender| sender.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_new_has_no_topics() {
        let adapter = TelemetryStreamAdapter::new(2048);
        assert!(adapter.topics.is_empty());
    }

    #[test]
    fn subscribe_topic_first_returns_true() {
        let adapter = TelemetryStreamAdapter::new(2048);
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        assert!(adapter.subscribe_topic("spirit-1", &topic));
    }

    #[test]
    fn subscribe_topic_second_returns_false() {
        let adapter = TelemetryStreamAdapter::new(2048);
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        adapter.subscribe_topic("spirit-1", &topic);
        assert!(
            !adapter.subscribe_topic("spirit-1", &topic),
            "re-subscribe by same spirit should return false"
        );
    }

    #[test]
    fn subscribe_topic_different_spirit_returns_true() {
        let adapter = TelemetryStreamAdapter::new(2048);
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        assert!(adapter.subscribe_topic("spirit-1", &topic));
        assert!(
            adapter.subscribe_topic("spirit-2", &topic),
            "different spirit subscribing to same topic should return true"
        );
    }

    #[test]
    fn publish_without_subscriber_silently_dropped() {
        let adapter = TelemetryStreamAdapter::new(2048);
        // No subscribers — send should be silently dropped
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        adapter.publish_event(
            &topic,
            ScalarTapEvent {
                spirit_id: "s1".into(),
                tag: "uncertainty".into(),
                value: 0.75,
                timestamp: 1,
            },
        );
        assert_eq!(
            adapter
                .drop_count
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn publish_with_subscriber_receives_event() {
        let adapter = TelemetryStreamAdapter::new(2048);
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        adapter.subscribe_topic("spirit-1", &topic);
        let mut rx = adapter.subscribe(&topic).unwrap();

        adapter.publish_event(
            &topic,
            ScalarTapEvent {
                spirit_id: "s1".into(),
                tag: "uncertainty".into(),
                value: 0.75,
                timestamp: 1,
            },
        );

        let received = rx.try_recv().expect("subscriber should receive the event");
        assert_eq!(received.spirit_id, "s1");
        assert_eq!(received.tag, "uncertainty");
        assert_eq!(received.value, 0.75);
    }

    #[test]
    fn adapter_default_constructs_with_2048() {
        let adapter = TelemetryStreamAdapter::default();
        assert_eq!(adapter.capacity, 2048);
    }

    #[test]
    fn subscribe_returns_initialized_receiver() {
        let adapter = TelemetryStreamAdapter::new(2048);
        let topic = TelemetryTopic::new("scalar.tap.uncertainty");
        adapter.subscribe_topic("spirit-1", &topic);
        let rx = adapter.subscribe(&topic);
        assert!(rx.is_some());
    }
}
