//! Self-Telemetry port trait per architecture FR56.
//!
//! Spirits read their own performance telemetry (success/failure counts,
//! latency distributions, halt events, distillation outcomes) scoped to
//! their principal namespace per FR31, without requiring per-read operator
//! admission.  The adapter lives at `maos-kernel-core::memory::self_telemetry`.

use crate::self_telemetry::{SelfTelemetryError, SelfTelemetryReport};

/// Self-telemetry port — read-only composer over existing kernel state.
///
/// Per FR56: "Spirit's own data; Spirit reads it" — the calling
/// `spirit_pid` is kernel-set from the wire-protocol context, not
/// Spirit-supplied.
pub trait SelfTelemetryPort: Send + Sync + 'static {
    /// Class: data-movement
    ///
    /// Return per-Spirit performance telemetry for the time window
    /// `[since_ns.unwrap_or(0), now_ns())`.  The data IS scoped to the
    /// Spirit's principal namespace per FR31 (best-effort at v0.3-β;
    /// precise filtering lands with Story 4.4's `intent_lineage`).
    fn self_telemetry(
        &self,
        spirit_pid: u32,
        since_ns: Option<u64>,
    ) -> Result<SelfTelemetryReport, SelfTelemetryError>;
}
