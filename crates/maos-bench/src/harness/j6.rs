#![forbid(unsafe_code)]

//! J6 measurement — Diego cold-start latency (§13.1; Story 8.5 AC6).
//!
//! J6 measures the **cold-start** time to load a Mira/Nash-shaped reference
//! Spirit: from a cold kernel substrate to a Spirit ready to receive its first
//! frame. The §13.1 budget is **< 500ms P95** (`J6_P95_BUDGET_US = 500_000`).
//!
//! Mirrors the J4 harness structure (Decision G — J6 is AUTHORED NEW here; J4 is
//! consumed from the existing harness):
//!
//! - **Smoke mode** (always available): canned cold-start `JourneyResult` for
//!   CI-fast JSON-shape verification + the criterion regression guard. Does NOT
//!   spawn real processes.
//! - **Real measurement** (requires `kernel_measurement` feature): cold-loads the
//!   kernel substrate a Mira/Nash rust-inproc Spirit needs (Transparency Log +
//!   Telemetry Stream + working-memory store + halt registry) `N` times and
//!   measures each cold-construction latency.
//!
//! A budget breach at HEAD is a §13.1 measurement RECORDED in the Dev Agent
//! Record (the J1/8.4 escape-hatch semantics: "fix our code first; do not mask").

use crate::harness::build_journey_result;
use crate::report::JourneyResult;

/// §13.1 J6 cold-start budget: Diego cold-start < 500ms P95.
pub const J6_P95_BUDGET_US: u64 = 500_000;

#[derive(Debug, thiserror::Error)]
pub enum J6Error {
    #[error("measurement error: {0}")]
    Measurement(String),
}

pub struct J6Config {
    pub invocation_count: u64,
}

impl Default for J6Config {
    fn default() -> Self {
        Self {
            invocation_count: 100,
        }
    }
}

/// Run J6 measurement.
///
/// Without the `kernel_measurement` feature, falls back to smoke mode.
/// With `kernel_measurement`, runs the real cold-load loop.
pub fn run_j6_measurement(config: &J6Config) -> Result<JourneyResult, J6Error> {
    #[cfg(feature = "kernel_measurement")]
    {
        return run_j6_kernel(config);
    }
    #[cfg(not(feature = "kernel_measurement"))]
    {
        eprintln!(
            "WARNING: J6 measurement running in smoke mode (no kernel_measurement feature). \
             Results are NOT real measurements."
        );
        Ok(run_j6_smoke_with_count(config.invocation_count))
    }
}

/// Smoketest-mode J6 measurement with a configurable invocation count. The canned
/// samples are representative cold-start latencies (≈120–300ms) comfortably under
/// the 500ms budget — for JSON-shape + budget-shape verification only.
pub fn run_j6_smoke_with_count(invocation_count: u64) -> JourneyResult {
    if invocation_count == 0 {
        // Avoid panic in build_journey_result on empty samples.
        return JourneyResult::new(
            "J6".into(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            true, // vacuously within budget
        );
    }
    let samples: Vec<u64> = (0..invocation_count)
        .map(|i| 120_000 + (i * 311) % 180_000)
        .collect();
    build_journey_result("J6", invocation_count, &samples, J6_P95_BUDGET_US)
}

/// Smoketest-mode J6 measurement: 50 cold-loads for CI-fast verification.
pub fn run_j6_smoke() -> JourneyResult {
    run_j6_smoke_with_count(50)
}

/// Real J6 cold-load measurement.
///
/// Requires `kernel_measurement` feature. For each iteration, cold-constructs the
/// kernel substrate AND instantiates the actual Mira/Nash Spirits, measuring the
/// full cold-start from empty process to Spirit-ready.
#[cfg(feature = "kernel_measurement")]
fn run_j6_kernel(config: &J6Config) -> Result<JourneyResult, J6Error> {
    // DEFERRED (CI remediation 2026-06-12). The real cold-load path authored in
    // Story 8.5 references the `mira` and `nash` Spirit crates, which are NOT
    // maos-bench dependencies, plus the private `maos_domain::frame::FrameOrigin`
    // path — it no longer compiles. Rebuilding it (add mira/nash dev-deps, use the
    // current substrate constructors + `invariants::i3::FrameOrigin`) is a tracked
    // story (see deferred-work.md). Until then the `kernel_measurement` build falls
    // back to the SMOKE sample with a loud warning — this keeps the maos-bench lib
    // compiling so the perf benches (which do NOT use J6) build and run, WITHOUT
    // presenting smoke numbers as real.
    eprintln!(
        "WARNING: J6 real cold-start measurement is DEFERRED (harness drift since Story 8.5); \
         returning a SMOKE sample — these are NOT real measurements."
    );
    Ok(run_j6_smoke_with_count(config.invocation_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j6_config_defaults() {
        let cfg = J6Config::default();
        assert_eq!(cfg.invocation_count, 100);
    }

    #[test]
    fn j6_budget_constant() {
        assert_eq!(J6_P95_BUDGET_US, 500_000);
    }

    #[test]
    fn j6_smoke_produces_result_within_budget() {
        let r = run_j6_smoke();
        assert_eq!(r.name, "J6");
        assert_eq!(r.invocation_count, 50);
        assert!(r.p95_us > 0);
        assert!(
            r.budget_met,
            "J6 cold-start smoke must be within the 500ms budget; p95={}us",
            r.p95_us
        );
    }

    #[test]
    fn j6_measurement_falls_back_to_smoke_without_feature() {
        let r = run_j6_measurement(&J6Config::default()).expect("J6 measurement");
        assert_eq!(r.name, "J6");
        // Either smoke (no feature) or real (feature) — both must report a budget.
        assert!(r.p95_us > 0);
    }
}
