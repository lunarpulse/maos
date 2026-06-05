//! Calibration-mode report writer.
//!
//! At v0.5 the rotation drill report is appended to
//! `_bmad-output/implementation-artifacts/mtls-rotation-chaos-report.md` as a
//! fenced JSON block. Per `[[feedback_lunarpulse_observability_preference]]`
//! the report IS the observable evidence.

use crate::chaos::rotation::RotationDrillReport;

/// Serialize the report to a markdown-fenced JSON block.
pub fn report_to_markdown(report: &RotationDrillReport) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string_pretty(report)?;
    Ok(format!(
        "\n## Rotation drill {}\n\n_v0.5 calibration mode — metrics measured, gates not enforced._\n\n```json\n{json}\n```\n",
        report.drill_id
    ))
}

/// Calibration-mode breach reporter — logs but does NOT panic.
pub fn report_breach(metric: &str, observed: u64, floor: u64) -> String {
    format!(
        "[v0.5 calibration] {metric} = {observed}ms breached floor {floor}ms (NOT enforced)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos::rotation::AgentRotationTimestamps;

    #[test]
    fn report_markdown_contains_drill_id_and_json() {
        let r = RotationDrillReport::from_per_agent(
            "drill-001",
            3,
            500,
            5_000,
            vec![AgentRotationTimestamps {
                agent_id: "a".into(),
                t_0_ns: 0,
                t_1_ns: Some(1_000_000_000),
                t_2_ns: Some(2_000_000_000),
            }],
            0,
            100,
        );
        let md = report_to_markdown(&r).expect("md");
        assert!(md.contains("drill-001"));
        assert!(md.contains("```json"));
        assert!(md.contains("v0.5 calibration mode"));
    }

    #[test]
    fn report_breach_logs_metric() {
        let s = report_breach("revocation_propagation_p99", 100_000, 90_000);
        assert!(s.contains("breached floor 90000"));
        assert!(s.contains("calibration"));
    }
}
