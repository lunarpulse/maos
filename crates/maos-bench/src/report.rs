#![forbid(unsafe_code)]

//! Wire-stable JSON report schemas for §13.1 measurement gate.
//!
//! ## Schema stability contract
//!
//! Field additions allowed with `#[serde(default)]`; field removals bump
//! the schema version. v0.5-α is the baseline.

use serde::{Deserialize, Serialize};

/// Top-level bench report — committed to `tests/reports/section-13-1-<sha>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchReport {
    #[doc = "Construct via [`BenchReport::new`] to enforce validation; struct literals bypass checks."]
    pub run_id: String,
    #[doc = "Construct via [`BenchReport::new`] — monotonic_now_ns() at run start."]
    pub started_at_ns: u64,
    #[doc = "Construct via [`BenchReport::new`] — short git SHA or 'untracked'."]
    pub git_sha: String,
    #[doc = "Construct via [`BenchReport::new`] — per-journey results."]
    pub journeys: Vec<JourneyResult>,
    #[doc = "Construct via [`BenchReport::new`] — derived from journey budgets."]
    pub decision: DecisionRecord,
}

impl BenchReport {
    pub fn new(
        run_id: String,
        started_at_ns: u64,
        git_sha: String,
        journeys: Vec<JourneyResult>,
        decision: DecisionRecord,
    ) -> Self {
        Self {
            run_id,
            started_at_ns,
            git_sha,
            journeys,
            decision,
        }
    }
}

/// Per-journey latency result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JourneyResult {
    #[doc = "Construct via [`JourneyResult::new`] to enforce validation; struct literals bypass checks."]
    pub name: String,
    #[doc = "Construct via [`JourneyResult::new`] — must be ≥1000 for production; ≥50 for smoke/fast-mode."]
    pub invocation_count: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds."]
    pub p50_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds; THIS IS THE BUDGET-GATED METRIC."]
    pub p95_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds."]
    pub p99_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds."]
    pub max_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds."]
    pub mean_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — microseconds."]
    pub std_dev_us: u64,
    #[doc = "Construct via [`JourneyResult::new`] — placeholder 0 if RSS/CPU sampling not wired at v0.5-α."]
    pub cpu_user_pct: u32,
    #[doc = "Construct via [`JourneyResult::new`] — placeholder 0 if RSS/CPU sampling not wired at v0.5-α."]
    pub cpu_sys_pct: u32,
    #[doc = "Construct via [`JourneyResult::new`] — placeholder 0 if RSS/CPU sampling not wired at v0.5-α."]
    pub rss_max_mb: u64,
    #[doc = "Construct via [`JourneyResult::new`] — true iff p95_us ≤ journey-specific budget."]
    pub budget_met: bool,
}

impl JourneyResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        invocation_count: u64,
        p50_us: u64,
        p95_us: u64,
        p99_us: u64,
        max_us: u64,
        mean_us: u64,
        std_dev_us: u64,
        budget_met: bool,
    ) -> Self {
        Self {
            name,
            invocation_count,
            p50_us,
            p95_us,
            p99_us,
            max_us,
            mean_us,
            std_dev_us,
            cpu_user_pct: 0,
            cpu_sys_pct: 0,
            rss_max_mb: 0,
            budget_met,
        }
    }
}

/// Decision record produced by `decide()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    #[doc = "Construct via [`DecisionRecord::new`] to enforce validation; struct literals bypass checks."]
    pub outcome: String,
    #[doc = "Construct via [`DecisionRecord::new`]."]
    pub j1_p95_met: bool,
    #[doc = "Construct via [`DecisionRecord::new`]."]
    pub j4_p95_met: bool,
    #[doc = "Construct via [`DecisionRecord::new`] — human-readable explanation linking numbers to decision."]
    pub rationale: String,
    #[doc = "Construct via [`DecisionRecord::new`] — 'ADR-040'."]
    pub adr_id: String,
}

impl DecisionRecord {
    pub fn new(
        outcome: String,
        j1_p95_met: bool,
        j4_p95_met: bool,
        rationale: String,
        adr_id: String,
    ) -> Self {
        Self {
            outcome,
            j1_p95_met,
            j4_p95_met,
            rationale,
            adr_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_report_serde_roundtrip() {
        let j1 = JourneyResult::new("J1".into(), 1000, 5000, 15000, 30000, 50000, 12000, 5000, true);
        let j4 = JourneyResult::new("J4".into(), 1000, 2000, 7000, 12000, 20000, 6000, 2000, true);
        let dr = DecisionRecord::new(
            "defer-rust-inproc-to-v2.0+".into(),
            true,
            true,
            "both budgets met".into(),
            "ADR-040".into(),
        );
        let report = BenchReport::new(
            "run-001".into(),
            123456789,
            "abc1234".into(),
            vec![j1, j4],
            dr,
        );

        let json = serde_json::to_string_pretty(&report).unwrap();
        let roundtripped: BenchReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, roundtripped);
    }
}
