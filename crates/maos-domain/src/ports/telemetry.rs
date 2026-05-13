//! Telemetry Stream port trait per architecture §4.7.
//!
//! Broadcasts events to subscribed Spirits. At v0.1-α this is an
//! internal module; Story 4.4 lands the `scalar.tap` stream and
//! pre-halt scalar drift watchdog.

use crate::invariants::i7::{ScalarTapEvent, TelemetryTopic};

/// Telemetry Stream — broadcast events, per-Spirit subscription.
///
/// Per §4.7: "Telemetry is broadcast; subscription is per-Spirit (I7)."
pub trait TelemetryStreamPort {
    /// Class: data-movement
    ///
    /// Publish a scalar-tap event to the given telemetry topic.
    /// All subscribed Spirits receive the event.
    fn publish_event(&self, topic: &TelemetryTopic, event: ScalarTapEvent);

    /// Class: data-movement
    ///
    /// Subscribe a Spirit to a telemetry topic. Returns `true` if
    /// the subscription was newly created, `false` if it already
    /// existed.
    fn subscribe_topic(&self, spirit_id: &str, topic: &TelemetryTopic) -> bool;
}
