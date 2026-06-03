#![forbid(unsafe_code)]

//! Measurement primitives for §13.1 J1 + J4 journeys.
//!
//! - Quantile computation: nearest-rank P50/P95/P99 + max/mean/std_dev.
//! - Timer: `monotonic_now_ns()` via `std::time::Instant` + `OnceLock` base.
//! - `BenchHarness`: owns run metadata + journey results.

pub mod j0;
pub mod j1;
pub mod j4;
pub mod j_researcher;

use crate::report::JourneyResult;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Compute P50/P95/P99/max/mean/std_dev from a sorted slice of microsecond samples.
///
/// Uses the **nearest-rank** method:
/// `P_k = sorted[ceil(k/100 * n) - 1]`.
///
/// # Panics
///
/// Panics if `samples` is empty.
pub fn compute_quantiles(samples: &[u64]) -> (u64, u64, u64, u64, u64, u64) {
    assert!(!samples.is_empty(), "samples must not be empty");
    let n = samples.len();
    let p50 = percentile_by_nearest_rank(samples, 50.0);
    let p95 = percentile_by_nearest_rank(samples, 95.0);
    let p99 = percentile_by_nearest_rank(samples, 99.0);
    let max = *samples.last().unwrap();

    let sum: u64 = samples.iter().sum();
    let mean = sum / n as u64;

    let variance: f64 = samples
        .iter()
        .map(|&s| {
            let diff = s as f64 - mean as f64;
            diff * diff
        })
        .sum::<f64>()
        / n as f64;
    let std_dev = variance.sqrt() as u64;

    (p50, p95, p99, max, mean, std_dev)
}

fn percentile_by_nearest_rank(sorted: &[u64], percentile: f64) -> u64 {
    let n = sorted.len();
    let rank = (percentile / 100.0 * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    sorted[idx]
}

static MONOTONIC_BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub fn init_monotonic_base() {
    let _ = MONOTONIC_BASE.set(Instant::now());
}

pub fn monotonic_now_ns() -> u64 {
    let base = MONOTONIC_BASE.get().copied().unwrap_or_else(|| {
        let now = Instant::now();
        let _ = MONOTONIC_BASE.set(now);
        now
    });
    base.elapsed().as_nanos() as u64
}

pub fn system_time_now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

pub fn run_id() -> String {
    format!("maos-bench-{}", system_time_now_ns())
}

pub fn git_sha() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "untracked".to_string())
}

pub struct BenchHarness {
    pub run_id: String,
    pub started_at_ns: u64,
    pub git_sha: String,
    pub journey_results: Vec<JourneyResult>,
}

impl BenchHarness {
    pub fn new() -> Self {
        Self {
            run_id: run_id(),
            started_at_ns: monotonic_now_ns(),
            git_sha: git_sha(),
            journey_results: Vec::new(),
        }
    }

    pub fn add_journey(&mut self, result: JourneyResult) {
        self.journey_results.push(result);
    }
}

/// Build a `LatencyHistogram` from raw microsecond samples and compute
/// a `JourneyResult` from it.
pub fn build_journey_result(
    name: &str,
    invocation_count: u64,
    samples_us: &[u64],
    budget_us: u64,
) -> JourneyResult {
    assert!(invocation_count > 0, "invocation_count must be > 0");
    assert!(!samples_us.is_empty(), "samples must not be empty");
    let mut sorted = samples_us.to_vec();
    sorted.sort_unstable();
    let (p50, p95, p99, max, mean, std_dev) = compute_quantiles(&sorted);
    let budget_met = p95 <= budget_us;
    JourneyResult::new(name.into(), invocation_count, p50, p95, p99, max, mean, std_dev, budget_met)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_quantiles_known_distribution() {
        let samples: Vec<u64> = (1..=100).collect();
        let (p50, p95, p99, max, mean, std_dev) = compute_quantiles(&samples);
        assert_eq!(p50, 50);
        assert_eq!(p95, 95);
        assert_eq!(p99, 99);
        assert_eq!(max, 100);
        assert_eq!(mean, 50);
        assert!(std_dev > 0);
    }

    #[test]
    fn compute_quantiles_uniform_distribution() {
        let samples: Vec<u64> = (0..1000).map(|x| x * 10).collect();
        let (p50, p95, _p99, _max, mean, _std_dev) = compute_quantiles(&samples);
        assert_eq!(p50, 4990);
        assert_eq!(p95, 9490);
        assert!(mean > 0);
    }

    #[test]
    fn build_journey_result_budget_check() {
        let samples: Vec<u64> = vec![1000, 2000, 3000, 4000, 5000];
        let r = build_journey_result("test", 5, &samples, 5000);
        assert_eq!(r.name, "test");
        assert_eq!(r.invocation_count, 5);
        assert!(r.budget_met);
        assert_eq!(r.p95_us, 5000);
    }

    #[test]
    fn build_journey_result_budget_breached() {
        let samples: Vec<u64> = vec![1000, 2000, 3000, 4000, 10000];
        let r = build_journey_result("test", 5, &samples, 5000);
        assert!(!r.budget_met);
    }

    #[test]
    fn git_sha_returns_string() {
        let sha = git_sha();
        assert!(!sha.is_empty());
    }

    #[test]
    fn bench_harness_new() {
        let h = BenchHarness::new();
        assert!(!h.run_id.is_empty());
        assert!(h.started_at_ns > 0);
        assert!(!h.git_sha.is_empty());
    }
}
