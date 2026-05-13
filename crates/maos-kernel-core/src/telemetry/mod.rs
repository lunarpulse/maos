#![forbid(unsafe_code)]

//! Telemetry Stream — internal module at v0.1 per §4.7.
//!
//! Broadcasts events to subscribed Spirits. At v0.1-α this is an
//! empty hexagonal adapter shell; Story 4.4 lands the `scalar.tap`
//! stream and pre-halt scalar drift watchdog.

pub use maos_domain::ports::TelemetryStreamPort;

/// Adapter shell — Story 4.4 implements `TelemetryStreamPort` for this
/// type with broadcast event routing and per-Spirit subscription.
/// At v0.1-α this is a zero-size placeholder; no fields, no methods.
#[derive(Debug, Clone, Copy, Default)]
pub struct TelemetryStreamAdapter;
