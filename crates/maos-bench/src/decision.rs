#![forbid(unsafe_code)]

//! §13.1 decision rule — pure function that gates rust-inproc unlock.
//!
//! ## Rule (per arch §13.1 per-journey latency budgets table)
//!
//! - J1 budget: `j1-bridge-p95` in `xtask/perf-budgets.toml` (25 ms).
//! - J4 budget: `j4-scalar-tap-p95` in `xtask/perf-budgets.toml` (10 ms).
//! - BOTH budgets met → `defer-rust-inproc-to-v2.0+`
//! - EITHER budget breached → `unlock-rust-inproc-in-v0.5`

use crate::report::{DecisionRecord, JourneyResult};

fn j1_p95_budget_us() -> u64 {
    crate::budgets::budget_us("j1-bridge-p95")
}

fn j4_p95_budget_us() -> u64 {
    crate::budgets::budget_us("j4-scalar-tap-p95")
}

pub fn decide(
    j1: &JourneyResult,
    j4: &JourneyResult,
    j6: Option<&JourneyResult>,
) -> DecisionRecord {
    let (j1_budget, j4_budget) = (j1_p95_budget_us(), j4_p95_budget_us());
    let j1_met = j1.p95_us <= j1_budget;
    let j4_met = j4.p95_us <= j4_budget;
    let j6_met = j6.map_or(true, |j| j.not_measured || j.budget_met);
    let outcome = if j1_met && j4_met {
        "defer-rust-inproc-to-v2.0+".to_string()
    } else {
        "unlock-rust-inproc-in-v0.5".to_string()
    };
    let rationale = format!(
        "J1 P95 = {}us (budget {}us, met={}); J4 P95 = {}us (budget {}us, met={}); J6 met={}; all-met={} → {}",
        j1.p95_us,
        j1_budget,
        j1_met,
        j4.p95_us,
        j4_budget,
        j4_met,
        j6_met,
        j1_met && j4_met && j6_met,
        outcome,
    );
    DecisionRecord::new(outcome, j1_met, j4_met, j6_met, rationale, "ADR-040".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j1_with_p95(p95_us: u64) -> JourneyResult {
        JourneyResult::new(
            "J1".into(),
            1000,
            p95_us / 2,
            p95_us,
            p95_us * 2,
            p95_us * 3,
            p95_us,
            p95_us / 10,
            p95_us <= j1_p95_budget_us(),
        )
    }

    fn j4_with_p95(p95_us: u64) -> JourneyResult {
        JourneyResult::new(
            "J4".into(),
            1000,
            p95_us / 2,
            p95_us,
            p95_us * 2,
            p95_us * 3,
            p95_us,
            p95_us / 10,
            p95_us <= j4_p95_budget_us(),
        )
    }

    #[test]
    fn both_budgets_met_defers_rust_inproc() {
        let j1 = j1_with_p95(18_500);
        let j4 = j4_with_p95(8_200);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "defer-rust-inproc-to-v2.0+");
        assert!(d.j1_p95_met);
        assert!(d.j4_p95_met);
        assert!(d.j6_p95_met);
        assert!(d.rationale.contains("defer"));
    }

    #[test]
    fn j1_breach_unlocks_rust_inproc() {
        let j1 = j1_with_p95(32_000);
        let j4 = j4_with_p95(8_200);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "unlock-rust-inproc-in-v0.5");
        assert!(!d.j1_p95_met);
        assert!(d.j4_p95_met);
        assert!(d.rationale.contains("unlock"));
    }

    #[test]
    fn j4_breach_unlocks_rust_inproc() {
        let j1 = j1_with_p95(18_500);
        let j4 = j4_with_p95(12_000);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "unlock-rust-inproc-in-v0.5");
        assert!(d.j1_p95_met);
        assert!(!d.j4_p95_met);
    }

    #[test]
    fn neither_met_unlocks_rust_inproc() {
        let j1 = j1_with_p95(32_000);
        let j4 = j4_with_p95(12_000);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "unlock-rust-inproc-in-v0.5");
        assert!(!d.j1_p95_met);
        assert!(!d.j4_p95_met);
    }

    #[test]
    fn j1_at_budget_boundary_met() {
        let j1 = j1_with_p95(25_000);
        let j4 = j4_with_p95(10_000);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "defer-rust-inproc-to-v2.0+");
        assert!(d.j1_p95_met);
        assert!(d.j4_p95_met);
    }

    #[test]
    fn j1_one_us_over_budget_not_met() {
        let j1 = j1_with_p95(25_001);
        let j4 = j4_with_p95(10_000);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.outcome, "unlock-rust-inproc-in-v0.5");
        assert!(!d.j1_p95_met);
        assert!(d.j4_p95_met);
    }

    #[test]
    fn adr_id_is_always_adr_040() {
        let j1 = j1_with_p95(18_500);
        let j4 = j4_with_p95(8_200);
        let d = decide(&j1, &j4, None);
        assert_eq!(d.adr_id, "ADR-040");
    }
    #[test]
    fn j6_not_measured_is_not_a_false_red() {
        // Story 10.4c review P2: a CUT (not_measured) J6 must NOT record
        // j6_p95_met=false — budget_met is false only because it was never
        // measured, not because it breached.
        let j1 = j1_with_p95(18_500);
        let j4 = j4_with_p95(8_200);
        let j6 = JourneyResult::not_measured("J6".into());
        let d = decide(&j1, &j4, Some(&j6));
        assert_eq!(d.outcome, "defer-rust-inproc-to-v2.0+");
        assert!(d.j6_p95_met, "a not_measured J6 must read j6_p95_met=true");
    }

    #[test]
    fn j6_measured_but_breached_is_a_real_red() {
        // The guard must not over-broaden: a MEASURED J6 that genuinely
        // breached (not_measured=false, budget_met=false) still reads false.
        let j1 = j1_with_p95(18_500);
        let j4 = j4_with_p95(8_200);
        let mut j6 = JourneyResult::not_measured("J6".into());
        j6.not_measured = false; // now a real (breached) measurement
        j6.budget_met = false;
        let d = decide(&j1, &j4, Some(&j6));
        assert!(
            !d.j6_p95_met,
            "a measured-but-breached J6 must read j6_p95_met=false"
        );
    }
}
