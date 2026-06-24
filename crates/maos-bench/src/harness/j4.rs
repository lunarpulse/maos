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

#![forbid(unsafe_code)]

// Story 10.4c: bench-fault-inject MUST NOT exist in release binaries (D3).
#[cfg(all(feature = "bench-fault-inject", not(debug_assertions)))]
compile_error!(
    "bench-fault-inject MUST NOT be enabled in release builds — \
     it is a test-only fault injection feature (Story 10.4c D3)"
);

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

/// Configuration for the J4 measurement loop.
pub struct J4Config {
    /// Total number of scalar.tap emit→subscribe iterations.
    pub invocation_count: u64,
    /// Warmup iterations discarded before recording latency samples.
    /// Default: 50 (D2 de-flaking; must leave ≥200 post-warmup samples).
    pub warmup_count: u64,
}

impl Default for J4Config {
    fn default() -> Self {
        Self {
            invocation_count: 1000,
            warmup_count: 50,
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
/// Requires `kernel_measurement` feature. Spawns a Tokio runtime with pinned
/// worker threads, initializes the kernel substrate from the verified template
/// (`scalar_tap_subscriber.rs:24-77`), and exercises the `scalar.tap` emission
/// path via the serial loop pattern (D1 — one scalar in flight, monotonic
/// `Instant`, no correlation map needed).
///
/// ## Measurement
///
/// Gate scalar: **cross-task delivery latency** `recv_instant − emit_instant`
/// (`std::time::Instant`, monotonic, same-process → no clock skew).
///
/// Secondary diagnostic: emitter-side **emit-cost** `emit_done − emit_instant`
/// (attribute a RED to emit vs. delivery).
///
/// ## Fault injection (AC2 / D3)
///
/// With `bench-fault-inject`: injects a ≥15ms delay INSIDE the measured
/// `[emit_instant, recv_instant]` span — between `emit_instant` capture and
/// `set_scalar` call. Harness-side only; zero kernel-core delta.
#[cfg(feature = "kernel_measurement")]
fn run_j4_kernel(config: &J4Config) -> Result<JourneyResult, BenchError> {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use maos_domain::invariants::i7::TelemetryTopic;
    use maos_domain::ports::crypto::CryptoProvider;
    use maos_kernel_core::capability::{
        cap_audit, cap_policy::PolicyTable, cap_quota::CapQuotaTracker,
        cap_tokens::Ed25519SigningKey, CapabilityRegistryAdapter, WorkingMemoryStore,
    };
    use maos_kernel_core::telemetry::TelemetryStreamAdapter;
    use maos_domain::ports::TelemetryStreamPort;

    // D2: pinned tokio worker count for reproducibility.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .map_err(|e| BenchError::Measurement(format!("tokio runtime: {e}")))?;

    rt.block_on(async {
        // ── Kernel substrate (verified template: scalar_tap_subscriber.rs:24-77) ──
        let telemetry = Arc::new(TelemetryStreamAdapter::new(2048));
        let crypto: Arc<dyn CryptoProvider> =
            Arc::new(maos_kernel_core::api::RingCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        let (audit_tx, _audit_rx) = cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let working_memory = Arc::new(WorkingMemoryStore::new());

        // 8-arg CapabilityRegistryAdapter ctor — SAME telemetry instance for
        // both the ctor and the subscription (AC1 shared-instance requirement).
        let adapter = CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xCAFE,
            policy,
            audit_tx,
            quota,
            working_memory,
            Arc::clone(&telemetry),
        );

        // ── Subscribe BEFORE the first set_scalar (broadcast has no replay) ──
        let tag = "bench";
        let topic = TelemetryTopic::new(&format!("scalar.tap.{tag}"));
        let is_new = telemetry.subscribe_topic("j4-observer", &topic);
        assert!(is_new, "subscribe_topic must return true on first subscribe");

        let mut rx = telemetry
            .subscribe(&topic)
            .expect("subscribe must return Some after subscribe_topic");

        // ── Serial measurement loop ──
        let total = config.invocation_count + config.warmup_count;
        let mut latency_samples =
            Vec::with_capacity(config.invocation_count as usize);
        let mut emit_cost_samples =
            Vec::with_capacity(config.invocation_count as usize);
        let mut samples_received: u64 = 0;

        for i in 0..total {
            let emit_instant = Instant::now();

            // AC2 / D3: fault injection INSIDE the measured span.
            #[cfg(feature = "bench-fault-inject")]
            {
                tokio::time::sleep(Duration::from_millis(15)).await;
            }

            adapter
                .set_scalar(1, "bench-spirit", tag, i as f64, "bench-frame")
                .map_err(|e| BenchError::Measurement(format!("set_scalar: {e}")))?;

            let emit_done = Instant::now();

            let recv_result = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
            let recv_instant = Instant::now();

            match recv_result {
                Ok(Ok(_event)) => {
                    samples_received += 1;
                    // Discard warmup prefix (D2).
                    if i >= config.warmup_count {
                        let latency_us = (recv_instant - emit_instant).as_micros() as u64;
                        let emit_cost_us = (emit_done - emit_instant).as_micros() as u64;
                        latency_samples.push(latency_us);
                        emit_cost_samples.push(emit_cost_us);
                    }
                }
                Ok(Err(e)) => {
                    return Err(BenchError::Measurement(format!(
                        "broadcast recv error at sample {i}: {e}"
                    )));
                }
                Err(_timeout) => {
                    return Err(BenchError::Measurement(format!(
                        "subscriber timeout at sample {i} (500ms)"
                    )));
                }
            }
        }

        // AC1: assert samples_received == total (warmup + measurement).
        assert_eq!(
            samples_received, total,
            "samples_received ({samples_received}) must equal total iterations ({total})"
        );

        // D2: N ≥ 200 post-warmup floor.
        assert!(
            latency_samples.len() >= 200,
            "post-warmup samples ({}) below N=200 floor — \
             increase invocation_count (currently {})",
            latency_samples.len(),
            config.invocation_count
        );

        // ── Emit-cost secondary diagnostic (AC1 D1) ──
        if !emit_cost_samples.is_empty() {
            let mut sorted_emit = emit_cost_samples;
            sorted_emit.sort_unstable();
            let n = sorted_emit.len();
            let emit_p95_idx = ((n as f64 * 0.95).ceil() as usize).saturating_sub(1).min(n - 1);
            let emit_p95 = sorted_emit[emit_p95_idx];
            let emit_mean: u64 = sorted_emit.iter().sum::<u64>() / n as u64;
            eprintln!(
                "J4 emit-cost diagnostic: P95={emit_p95}µs mean={emit_mean}µs \
                 (secondary — NOT the gate scalar; for emit-vs-delivery attribution)"
            );
        }

        Ok(build_journey_result(
            "J4",
            config.invocation_count,
            &latency_samples,
            J4_P95_BUDGET_US,
        ))
    })
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
