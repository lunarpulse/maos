//! I7: Telemetry is broadcast; subscription is per-Spirit.
//!
//! Pre-halt scalar trajectory observable via the `scalar.tap` stream so
//! Observer Spirits witness the runup, not just the alarm.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3**: `—` (unchanged).
//! - **v0.5**: `runtime` — Telemetry Stream + `scalar.tap` operational.
//! - **v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i7::{InvariantI7, TelemetryTopic, ScalarTapEvent};
//!
//! let _marker: InvariantI7 = InvariantI7;
//! let topic = TelemetryTopic::new("scalar.tap.confidence");
//! assert_eq!(topic.as_str(), "scalar.tap.confidence");
//! ```

/// I7 marker type — Telemetry is broadcast; subscription is per-Spirit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI7;

/// Telemetry topic identifier — namespaced broadcast channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TelemetryTopic(String);

impl TelemetryTopic {
    /// Create a new telemetry topic.
    pub fn new(topic: impl Into<String>) -> Self {
        Self(topic.into())
    }

    /// Return the topic string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A scalar-tap event — the pre-halt observable trajectory.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScalarTapEvent {
    /// Spirit that emitted the scalar.
    pub spirit_id: String,
    /// Tag identifying the scalar metric.
    pub tag: String,
    /// Scalar value.
    pub value: f64,
    /// Unix timestamp (ms) of the observation.
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_topic_creation() {
        let t = TelemetryTopic::new("scalar.tap.entropy");
        assert_eq!(t.as_str(), "scalar.tap.entropy");
    }

    #[test]
    fn scalar_tap_event_construction() {
        let e = ScalarTapEvent {
            spirit_id: "s1".into(),
            tag: "entropy".into(),
            value: 0.75,
            timestamp: 1_700_000_000_000,
        };
        assert_eq!(e.value, 0.75);
    }
}
