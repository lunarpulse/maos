#![forbid(unsafe_code)]

//! Fixture-replay bench runner — canned-latency fast-mode for smoke arms.
//!
//! Gated behind `#[cfg(any(test, feature = "fixture_replay"))]`.
//! Produces deterministic `JourneyResult` values with canned latencies
//! so the JSON-shape contract can be exercised on every PR without
//! spawning real subprocesses.

use crate::harness::build_journey_result;
use crate::report::JourneyResult;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("fixture replay not supported for journey: {0}")]
    UnknownJourney(String),
}

pub struct FixtureReplayBenchRunner {
    journey: String,
    invocation_count: u64,
    canned_p95_us: u64,
}

impl FixtureReplayBenchRunner {
    pub fn new(journey: &str, invocation_count: u64, canned_p95_us: u64) -> Self {
        Self {
            journey: journey.to_string(),
            invocation_count,
            canned_p95_us,
        }
    }

    pub fn run(&self) -> Result<JourneyResult, BenchError> {
        match self.journey.as_str() {
            "J1" => Ok(self.run_j1()),
            "J4" => Ok(self.run_j4()),
            other => Err(BenchError::UnknownJourney(other.to_string())),
        }
    }

    fn run_j1(&self) -> JourneyResult {
        let budget_us = 25_000;
        let samples: Vec<u64> = (0..self.invocation_count)
            .map(|i| {
                let base = self.canned_p95_us.saturating_sub(5000);
                let offset = (i * 37) % 5000;
                base + offset as u64
            })
            .collect();
        build_journey_result("J1", self.invocation_count, &samples, budget_us)
    }

    fn run_j4(&self) -> JourneyResult {
        let budget_us = 10_000;
        let samples: Vec<u64> = (0..self.invocation_count)
            .map(|i| {
                let base = self.canned_p95_us.saturating_sub(2000);
                let offset = (i * 29) % 2000;
                base + offset as u64
            })
            .collect();
        build_journey_result("J4", self.invocation_count, &samples, budget_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_replay_j1_defer() {
        let runner = FixtureReplayBenchRunner::new("J1", 50, 15_000);
        let result = runner.run().unwrap();
        assert_eq!(result.name, "J1");
        assert_eq!(result.invocation_count, 50);
        assert!(result.budget_met);
    }

    #[test]
    fn fixture_replay_j4_canned() {
        let runner = FixtureReplayBenchRunner::new("J4", 50, 7_000);
        let result = runner.run().unwrap();
        assert_eq!(result.name, "J4");
        assert_eq!(result.invocation_count, 50);
        assert!(result.budget_met);
    }

    #[test]
    fn fixture_replay_j1_budget_breach() {
        let runner = FixtureReplayBenchRunner::new("J1", 50, 30_000);
        let result = runner.run().unwrap();
        assert_eq!(result.name, "J1");
        assert!(!result.budget_met);
    }

    #[test]
    fn fixture_replay_j4_budget_breach() {
        let runner = FixtureReplayBenchRunner::new("J4", 50, 15_000);
        let result = runner.run().unwrap();
        assert_eq!(result.name, "J4");
        assert!(!result.budget_met);
    }

    #[test]
    fn fixture_replay_unknown_journey() {
        let runner = FixtureReplayBenchRunner::new("J99", 10, 5000);
        let err = runner.run().unwrap_err();
        assert!(err.to_string().contains("J99"));
    }
}
