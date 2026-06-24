#![forbid(unsafe_code)]

//! J4 measurement — Mira-Nash Observer colocation latency.
//!
//! Option B (v0.5-α): one producer subprocess + in-kernel observer subscriber.
//! The producer Spirit calls `kernel.set_scalar(...)` triggering a `scalar.tap`
//! IAC frame; an in-kernel `TelemetryStreamPort` subscriber callback captures
//! the delivery timestamp. Latency = subscriber_callback_time - kernel_emit_time.
//!
//! ## Caveat (documented in ADR-040 rationale)
//!
//! The in-kernel observer is faster-than-real-Observer Spirit (no wire-protocol
//! decode time): budget-met-with-margin is robust; budget-not-met is
//! conservative-favorable to "subprocess works" (unlock rust-inproc).
//!
//! ## Modes
//!
//! - **Smoke mode** (always available): canned-latency `JourneyResult` for
//!   CI-fast JSON-shape verification. Does NOT spawn real processes.
//! - **Real measurement** (requires `kernel_measurement` feature): spawns a
//!   Tokio runtime with kernel adapters and exercises the scalar.tap emission
//!   path end-to-end.

use crate::harness::build_journey_result;
use crate::report::JourneyResult;

/// §13.1 Mira-Nash Observer colocation-latency P95 budget (10ms). Its authority
/// is the §13.1 floor, not a mirror test — so it is not re-asserted by a test.
const J4_P95_BUDGET_US: u64 = 10_000;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("measurement error: {0}")]
    Measurement(String),
}

pub struct J4Config {
    pub invocation_count: u64,
}

impl Default for J4Config {
    fn default() -> Self {
        Self {
            invocation_count: 1000,
        }
    }
}

/// Canonical marker every placeholder (canned/deferred) J4 path emits. Matching
/// this substring proves the run produced NO real numbers, regardless of which
/// fallback ran (feature-off smoke vs. feature-on DEFERRED kernel path).
const J4_PLACEHOLDER_MARKER: &str = "J4 placeholder — NOT real measurements";

/// Emit the placeholder WARNING with a path-specific `detail`. Both fallback
/// paths funnel through here so the NOT-real marker is guaranteed identical,
/// avoiding string drift between the feature-off and feature-on paths.
fn warn_placeholder(detail: &str) {
    eprintln!("WARNING: {J4_PLACEHOLDER_MARKER} ({detail})");
}

/// Run J4 measurement.
///
/// Without the `kernel_measurement` feature, falls back to smoke mode.
/// With `kernel_measurement`, runs the real in-kernel scalar.tap loop.
pub fn run_j4_measurement(config: &J4Config) -> Result<JourneyResult, BenchError> {
    #[cfg(feature = "kernel_measurement")]
    {
        return run_j4_kernel(config);
    }
    #[cfg(not(feature = "kernel_measurement"))]
    {
        let _ = config;
        warn_placeholder("no kernel_measurement feature — smoke-mode canned samples");
        Ok(run_j4_smoke_with_count(config.invocation_count))
    }
}

/// Smoketest-mode J4 measurement with a configurable invocation count.
pub fn run_j4_smoke_with_count(invocation_count: u64) -> JourneyResult {
    let samples: Vec<u64> = (0..invocation_count)
        .map(|i| 1000 + (i * 20) % 5000)
        .collect();
    build_journey_result("J4", invocation_count, &samples, J4_P95_BUDGET_US)
}

/// Smoketest-mode J4 measurement: 50 invocations for CI-fast verification.
pub fn run_j4_smoke() -> JourneyResult {
    run_j4_smoke_with_count(50)
}

/// Real in-kernel J4 measurement.
///
/// Requires `kernel_measurement` feature. Spawns a Tokio runtime, initializes
/// kernel adapters, and exercises the scalar.tap emission path.
#[cfg(feature = "kernel_measurement")]
fn run_j4_kernel(config: &J4Config) -> Result<JourneyResult, BenchError> {
    // DEFERRED (CI remediation 2026-06-12). The real in-kernel scalar.tap wiring
    // authored in Story 8.5 drifted against the current kernel composition and no
    // longer compiles: the `CryptoProvider` trait was reshaped (gained
    // `seal_for_export`/`sign_capability_token`, dropped `sign`/`sign_detached`/
    // `generate_keypair`, changed `verify_signature`), `CapabilityRegistryAdapter::new`
    // grew to 8 args, `TransparencyLogAdapter::new` became `open_in_memory`, and
    // `Ed25519SigningKey::generate` moved. Rebuilding the real path is a tracked
    // story (see deferred-work.md). Until then the `kernel_measurement` build falls
    // back to the SMOKE sample with a loud warning — this keeps the maos-bench lib
    // compiling so the iac_routing_budget / orchestrator_fanout perf benches (which
    // do NOT use J4) can build and run, WITHOUT presenting smoke numbers as real.
    warn_placeholder(
        "real kernel measurement DEFERRED — harness drift since Story 8.5, returning smoke samples",
    );
    Ok(run_j4_smoke_with_count(config.invocation_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j4_config_defaults() {
        let cfg = J4Config::default();
        assert_eq!(cfg.invocation_count, 1000);
    }

    #[test]
    fn j4_smoke_produces_result() {
        let r = run_j4_smoke();
        assert_eq!(r.name, "J4");
        assert_eq!(r.invocation_count, 50);
        assert!(r.p95_us > 0);
        assert!(r.budget_met);
    }
}
