#![forbid(unsafe_code)]

//! Story 20-7 AC3 — the one performance budget source, `xtask/perf-budgets.toml`
//! (ADR-068 §5): per metric the requirement id, quantile, budget, unit, noise
//! and regression tolerance, per-phase disposition, tier (C counts / W wall
//! clock) and instrument. No budget constant lives anywhere else; the
//! `maos-bench` harnesses and benches and the kernel's cap-token guard read
//! this file, and `budget_literals` is the scan that proves they still do.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

pub const PATH: &str = "xtask/perf-budgets.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Protocol {
    /// The GA phase the dispositions are read at; equals `gate_common::CURRENT_PHASE`.
    pub phase: String,
    /// ADR-068 §5 calibration: coefficient of variation above this ⇒ `INDETERMINATE`.
    pub calibration_cv_max_pct: f64,
    /// ADR-068 §5 calibration: median drift above this ⇒ `INDETERMINATE`.
    pub calibration_drift_max_pct: f64,
    /// ADR-068 §5 regression: the change must also exceed this many MADs.
    pub mad_multiplier: f64,
    /// ADR-068 §6: the baseline is the median of this many latest `PASS` runs.
    pub baseline_runs: usize,
    /// ADR-068 §6: a newest ledger entry older than this is red.
    pub stale_hours: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// The measured value must stay at or under the budget (latency).
    Max,
    /// The measured value must stay at or over the budget (throughput).
    Min,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub id: String,
    /// NFR / §13.1 / success-criteria / ADR id the budget comes from.
    pub requirement: String,
    pub quantile: String,
    /// Absent only for Tier C rows, whose figure is the committed `baseline`.
    pub budget: Option<f64>,
    pub unit: String,
    pub direction: Direction,
    /// Five-run spread / median above this ⇒ `INDETERMINATE`.
    pub noise_tolerance: f64,
    /// Relative change over the baseline that may `FAIL` (with the MAD test).
    pub regression_tolerance: f64,
    /// `"C"` (counts, every PR) or `"W"` (wall clock, the stable runner).
    pub tier: String,
    /// The file that measures it; must exist (the completeness vector).
    pub instrument: String,
    pub disposition: HashMap<String, String>,
    /// Tier C: the harness path name measured.
    pub path: Option<String>,
    /// Tier C: the committed baseline count per iteration (absent until measured).
    pub baseline: Option<f64>,
}

impl Metric {
    /// Does a breach block at `phase`? Only an explicit `blocking` does.
    pub fn binding_at(&self, phase: &str) -> bool {
        crate::gate_common::phase_disposition(&self.disposition, phase) == Some("blocking")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub protocol: Protocol,
    pub metric: Vec<Metric>,
}

pub fn parse(text: &str) -> Result<Budgets, String> {
    let budgets: Budgets = toml::from_str(text).map_err(|error| format!("{PATH}: {error}"))?;
    let phases = ["v1_0", "v1_5", "v2_0", "v2_2"];
    let p = &budgets.protocol;
    if !phases.contains(&p.phase.as_str())
        || p.baseline_runs == 0
        || p.stale_hours <= 0
        || [
            p.calibration_cv_max_pct,
            p.calibration_drift_max_pct,
            p.mad_multiplier,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err(format!("{PATH}: invalid protocol"));
    }
    let mut ids = std::collections::HashSet::new();
    for m in &budgets.metric {
        let wall = m.tier == "W";
        if !ids.insert(&m.id)
            || m.id.is_empty()
            || m.requirement.is_empty()
            || m.instrument.is_empty()
            || !["W", "C"].contains(&m.tier.as_str())
            || !["p50", "p95", "p99", "p99.9", "sustained", "count"].contains(&m.quantile.as_str())
            || [m.noise_tolerance, m.regression_tolerance]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || m.budget
                .into_iter()
                .chain(m.baseline)
                .any(|v| !v.is_finite() || v < 0.0)
            || m.disposition.is_empty()
            || m.disposition.iter().any(|(p, d)| {
                !phases.contains(&p.as_str()) || !["advisory", "blocking"].contains(&d.as_str())
            })
            || (wall
                && (m.budget.is_none()
                    || !["us", "frames_per_s"].contains(&m.unit.as_str())
                    || m.path.is_some()
                    || m.baseline.is_some()))
            || (!wall
                && (m.path.as_ref().is_none_or(String::is_empty)
                    || m.quantile != "count"
                    || !["allocations", "instructions"].contains(&m.unit.as_str())
                    || m.budget.is_some()
                    || m.direction != Direction::Max
                    || (m.binding_at(&p.phase) && m.baseline.is_none())))
        {
            return Err(format!("{PATH}: invalid metric {}", m.id));
        }
    }
    Ok(budgets)
}

pub fn load(root: &Path) -> Result<Budgets, String> {
    let text = std::fs::read_to_string(root.join(PATH))
        .map_err(|error| format!("{PATH}: cannot read: {error}"))?;
    parse(&text)
}

/// Budget literals the story moved into the file, by the exact site that held
/// them (AC3.2). A site is a finding when its pattern still matches.
const LITERAL_SITES: &[(&str, &str)] = &[
    (
        "crates/maos-bench/src/harness/j1.rs",
        "J1_P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-bench/src/harness/j1.rs",
        "J1_CI_CEILING_US: u64 = ",
    ),
    (
        "crates/maos-bench/src/harness/j4.rs",
        "J4_P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-bench/src/decision.rs",
        "J1_P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-bench/src/decision.rs",
        "J4_P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-bench/benches/j4_latency_regression.rs",
        "J4_P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-bench/benches/iac_routing_budget.rs",
        "P95_BUDGET_US: u64 = ",
    ),
    (
        "crates/maos-kernel-core/tests/cap_token_verify_assertion.rs",
        "p99_ns < 5_000",
    ),
];

/// `file:line — text` for every budget literal still outside the budget file.
pub fn budget_literals(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for (file, pattern) in LITERAL_SITES {
        let Ok(text) = std::fs::read_to_string(root.join(file)) else {
            findings.push(format!("{file}: unreadable (a budget site was moved)"));
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if !code.starts_with("//") && code.contains(pattern) {
                findings.push(format!("{file}:{} — {}", index + 1, code));
            }
        }
    }
    findings
}
