//! Rotation drill markdown surface — Story 14-2 / AC6.1.a: **WIRED, not
//! deleted.** This module sat dead for eleven stories (zero callers since
//! 6-3's calibration phase); it is now the rendered-markdown publication
//! surface for the rotation report, with the sample-count disclosure AC4.2
//! requires.
//!
//! Every percentile axis publishes its sample count **computed from the
//! same filter the percentile engine uses** (`rotation.rs:
//! per_agent.iter().filter_map(...)`, never `.len()`): at n ≤ 100 the p99
//! IS the worst of n (first real percentile at n=101), and a reader must
//! not mistake a max-of-ten for a characterised tail.

use crate::chaos::rotation::RotationDrillReport;

/// Serialize the report to a markdown block, headed by the per-axis sample
/// counts. The counts derive from `per_agent` exactly as
/// `RotationDrillReport::from_per_agent` derives its samples.
pub fn report_to_markdown(report: &RotationDrillReport) -> Result<String, serde_json::Error> {
    let n_prop = report
        .per_agent
        .iter()
        .filter(|a| a.t_1_ns.is_some())
        .count();
    let n_rh = report
        .per_agent
        .iter()
        .filter(|a| a.t_1_ns.is_some() && a.t_2_ns.is_some())
        .count();
    let n_e2e = report
        .per_agent
        .iter()
        .filter(|a| a.t_2_ns.is_some())
        .count();
    let json = serde_json::to_string_pretty(report)?;
    // §A6 close-pass (CloseEdge-4): a zero-sample axis renders as UNMEASURED,
    // never as "the worst of n" — there is no worst element at n=0, and the
    // percentile engine's zero defaults must not read as an observed tail.
    let axis = |n: usize| {
        if n == 0 {
            "unmeasured".to_string()
        } else {
            format!("{n} (at n <= 100 the p99 IS the worst of n)")
        }
    };
    Ok(format!(
        "\n## Rotation drill {} \
         (n: propagation={}, re-handshake={}, end-to-end={})\n\n```json\n{json}\n```\n",
        report.drill_id,
        axis(n_prop),
        axis(n_rh),
        axis(n_e2e),
    ))
}
