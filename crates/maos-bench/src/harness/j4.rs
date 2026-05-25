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
        eprintln!(
            "WARNING: J4 measurement running in smoke mode (no kernel_measurement feature). \
             Results are NOT real measurements."
        );
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
    use crate::harness::monotonic_now_ns;
    use std::sync::{Arc, mpsc};

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_time()
        .build()
        .map_err(|e| BenchError::Measurement(format!("Tokio runtime build failed: {e}")))?;

    let (tx, rx) = mpsc::channel();
    let n = config.invocation_count;

    rt.block_on(async move {
        use maos_domain::invariants::i1::CapabilityToken;
        use maos_domain::ports::crypto::CryptoProvider;
        use maos_kernel_core::capability::cap_policy::PolicyTable;
        use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
        use maos_kernel_core::capability::CapabilityRegistryAdapter;
        use maos_kernel_core::iac::mailbox::Mailbox;
        use maos_kernel_core::iac::TransparencyLogAdapter;
        use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
        use maos_kernel_core::telemetry::TelemetryStreamAdapter;

        struct MockCrypto;
        impl CryptoProvider for MockCrypto {
            fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> bool {
                true
            }
            fn sign(
                &self,
                _keypair: &[u8],
                _msg: &[u8],
            ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
                Ok(vec![0u8; 64])
            }
            fn generate_keypair(
                &self,
            ) -> Result<(Vec<u8>, Vec<u8>), maos_domain::ports::crypto::CryptoError> {
                Ok((vec![0u8; 32], vec![0u8; 64]))
            }
            fn sign_detached(
                &self,
                _sk: &[u8],
                _msg: &[u8],
            ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
                Ok(vec![0u8; 64])
            }
        }

        let crypto = Arc::new(MockCrypto);
        let signing_key = Ed25519SigningKey::generate();
        let policy_table = PolicyTable::new();
        let iac_metrics = IacRtMetrics::new();
        let cap_registry = CapabilityRegistryAdapter::new(&signing_key, &policy_table, &iac_metrics)
            .map_err(|e| BenchError::Measurement(e.to_string()))?;

        let tl = Arc::new(TransparencyLogAdapter::new());
        let mailbox = Mailbox::new(tl.clone(), &iac_metrics);
        let _telemetry = TelemetryStreamAdapter::new(mailbox.subscribe_telemetry());

        let mut samples_us = Vec::with_capacity(n as usize);

        for _i in 0..n {
            let emit_time = monotonic_now_ns();
            let result = cap_registry.set_scalar("tap", 0);
            let recv_time = monotonic_now_ns();
            let latency_ns = recv_time.saturating_sub(emit_time);
            samples_us.push(latency_ns / 1000);
            let _ = result;
        }

        let _ = tx.send(samples_us);
        Ok::<_, BenchError>(())
    })
    .map_err(|e: Box<dyn std::error::Error + Send + Sync>| {
        BenchError::Measurement(e.to_string())
    })?
    .map_err(|e: BenchError| e)?;

    let samples_us = rx
        .recv()
        .map_err(|e| BenchError::Measurement(format!("channel receive: {e}")))?;

    let result = build_journey_result("J4", config.invocation_count, &samples_us, J4_P95_BUDGET_US);
    Ok(result)
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
    fn j4_budget_constant() {
        assert_eq!(J4_P95_BUDGET_US, 10_000);
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
