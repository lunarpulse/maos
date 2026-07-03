//! Pre-staged-overlap rotation procedure + `T_grace` computation per
//! architecture §7.2.1.a.

use serde::{Deserialize, Serialize};

/// `T_grace = max(2 × p99_handshake_rtt, 5 s)` per architecture §7.2.1.a.
///
/// `p99_handshake_rtt_ms` is the trailing 30-day p99 of `iac_handshake_duration_us`
/// (TLS 1.3 handshake duration). If `days_of_history < 30` (cold deployment),
/// use the max observed handshake duration floored at 500 ms.
pub fn compute_t_grace(p99_handshake_rtt_ms: u64, days_of_history: u32) -> std::time::Duration {
    let baseline_ms = if days_of_history < 30 {
        std::cmp::max(p99_handshake_rtt_ms, 500)
    } else {
        p99_handshake_rtt_ms
    };
    let t_grace_ms = std::cmp::max(2 * baseline_ms, 5_000);
    std::time::Duration::from_millis(t_grace_ms)
}

/// Per-agent timestamps per architecture §7.2.1.b:
///
/// - `t_0` — `revoke()` API call returns success at CA
/// - `t_1` — agent's OCSP/CRL check first returns `revoked` for old cert
/// - `t_2` — agent completes successful TLS handshake with replacement cert
///   AND first data-plane request succeeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRotationTimestamps {
    pub agent_id: String,
    pub t_0_ns: u64,
    /// `None` if the agent never observed the revocation.
    pub t_1_ns: Option<u64>,
    /// `None` if the agent never re-handshook successfully.
    pub t_2_ns: Option<u64>,
}

impl AgentRotationTimestamps {
    /// `t_1 − t_0` in milliseconds; `None` if `t_1` is `None`.
    pub fn revocation_propagation_ms(&self) -> Option<u64> {
        self.t_1_ns
            .map(|t1| (t1.saturating_sub(self.t_0_ns)) / 1_000_000)
    }

    /// `t_2 − t_1` in milliseconds; `None` if either is `None`.
    pub fn re_handshake_ms(&self) -> Option<u64> {
        match (self.t_1_ns, self.t_2_ns) {
            (Some(t1), Some(t2)) => Some(t2.saturating_sub(t1) / 1_000_000),
            _ => None,
        }
    }

    /// `t_2 − t_0` in milliseconds; `None` if `t_2` is `None`.
    pub fn end_to_end_ms(&self) -> Option<u64> {
        self.t_2_ns
            .map(|t2| t2.saturating_sub(self.t_0_ns) / 1_000_000)
    }
}

/// Aggregate timing-floor distributions per §7.2.1.b.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationDrillReport {
    pub drill_id: String,
    pub host_count: u32,
    pub p99_handshake_rtt_ms: u64,
    pub t_grace_ms: u64,
    pub per_agent: Vec<AgentRotationTimestamps>,
    /// `t_1 − t_0` distribution across the fleet.
    pub revocation_propagation_p50_ms: u64,
    pub revocation_propagation_p99_ms: u64,
    /// `t_2 − t_1` distribution across the fleet.
    pub re_handshake_p50_ms: u64,
    pub re_handshake_p99_ms: u64,
    /// `t_2 − t_0` distribution across the fleet.
    pub end_to_end_p50_ms: u64,
    pub end_to_end_p99_ms: u64,
    /// `cert_post_grace_reject` rate as a fraction in `[0.0, 1.0]`.
    pub post_grace_reject_rate: f64,
    /// Pass/fail per §7.2.1.b floors.
    pub passes_v07_floors: bool,
    pub passes_v10_floors: bool,
    /// v1.5 NFR-Sec-13 floors: ≥ as strict as existing v0.7/v1.0 floors.
    /// The literal NFR-Sec-13 numbers (median ≤60s / p99 ≤5min) are LOOSER
    /// than the ratified v0.7/v1.0 floors; passes_v15_floors inherits all
    /// existing strictness (passes_v10_floors implies passes_v07_floors).
    pub passes_v15_floors: bool,
}

impl RotationDrillReport {
    /// Compute the aggregate distributions from per-agent timestamps.
    pub fn from_per_agent(
        drill_id: impl Into<String>,
        host_count: u32,
        p99_handshake_rtt_ms: u64,
        t_grace_ms: u64,
        per_agent: Vec<AgentRotationTimestamps>,
        post_grace_reject_count: u64,
        post_grace_total_count: u64,
    ) -> Self {
        let prop_samples: Vec<u64> = per_agent
            .iter()
            .filter_map(|a| a.revocation_propagation_ms())
            .collect();
        let rh_samples: Vec<u64> = per_agent
            .iter()
            .filter_map(|a| a.re_handshake_ms())
            .collect();
        let e2e_samples: Vec<u64> = per_agent.iter().filter_map(|a| a.end_to_end_ms()).collect();

        let (prop_p50, prop_p99) = percentiles(&prop_samples);
        let (rh_p50, rh_p99) = percentiles(&rh_samples);
        let (e2e_p50, e2e_p99) = percentiles(&e2e_samples);

        let post_grace_reject_rate = if post_grace_total_count == 0 {
            0.0
        } else {
            post_grace_reject_count as f64 / post_grace_total_count as f64
        };

        // §7.2.1.b floors:
        // v0.7: revocation propagation p50 ≤ 30s, p99 ≤ 90s; re-handshake p50 ≤ 30s, p99 ≤ 60s.
        // v1.0: + end-to-end p50 ≤ 60s, p99 ≤ 150s; cert_post_grace_reject ≤ 0.1%.
        let passes_v07_floors =
            prop_p50 <= 30_000 && prop_p99 <= 90_000 && rh_p50 <= 30_000 && rh_p99 <= 60_000;

        let passes_v10_floors = passes_v07_floors
            && e2e_p50 <= 60_000
            && e2e_p99 <= 150_000
            && post_grace_reject_rate <= 0.001;

        // v1.5: NFR-Sec-13 by name — MUST be ≥ as strict as existing v0.7/v1.0
        // floors. The literal NFR-Sec-13 numbers (median ≤60s / p99 ≤5min) are
        // LOOSER than the existing floors; adopting those alone would REGRESS
        // the gate. passes_v15_floors inherits passes_v10_floors strictness.
        let passes_v15_floors = passes_v10_floors;

        Self {
            drill_id: drill_id.into(),
            host_count,
            p99_handshake_rtt_ms,
            t_grace_ms,
            per_agent,
            revocation_propagation_p50_ms: prop_p50,
            revocation_propagation_p99_ms: prop_p99,
            re_handshake_p50_ms: rh_p50,
            re_handshake_p99_ms: rh_p99,
            end_to_end_p50_ms: e2e_p50,
            end_to_end_p99_ms: e2e_p99,
            post_grace_reject_rate,
            passes_v07_floors,
            passes_v10_floors,
            passes_v15_floors,
        }
    }
}

pub(crate) fn percentiles(samples: &[u64]) -> (u64, u64) {
    if samples.is_empty() {
        return (0, 0);
    }
    let mut s = samples.to_vec();
    s.sort_unstable();
    let p50_idx = (s.len() as f64 * 0.50).floor() as usize;
    let p99_idx = ((s.len() as f64 * 0.99).floor() as usize).min(s.len() - 1);
    (s[p50_idx.min(s.len() - 1)], s[p99_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_grace_cold_deployment_floor_500ms() {
        let t = compute_t_grace(100, 7);
        assert_eq!(t, std::time::Duration::from_millis(5_000));
    }

    #[test]
    fn t_grace_cold_deployment_observed_higher_than_500() {
        let t = compute_t_grace(3000, 7);
        // 2 × max(3000, 500) = 6000; max(6000, 5000) = 6000
        assert_eq!(t, std::time::Duration::from_millis(6_000));
    }

    #[test]
    fn t_grace_steady_state() {
        let t = compute_t_grace(2000, 30);
        // 2 × 2000 = 4000; max(4000, 5000) = 5000
        assert_eq!(t, std::time::Duration::from_millis(5_000));
    }

    #[test]
    fn t_grace_steady_state_observed_higher_than_2500() {
        let t = compute_t_grace(3000, 30);
        // 2 × 3000 = 6000; max(6000, 5000) = 6000
        assert_eq!(t, std::time::Duration::from_millis(6_000));
    }

    #[test]
    fn agent_revocation_propagation_calc() {
        let a = AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: 0,
            t_1_ns: Some(15_000_000_000), // 15s in ns
            t_2_ns: Some(25_000_000_000), // 25s in ns
        };
        assert_eq!(a.revocation_propagation_ms(), Some(15_000));
        assert_eq!(a.re_handshake_ms(), Some(10_000));
        assert_eq!(a.end_to_end_ms(), Some(25_000));
    }

    #[test]
    fn report_passes_v07_under_floors() {
        let agents = vec![AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: 0,
            t_1_ns: Some(10_000_000_000), // 10s
            t_2_ns: Some(20_000_000_000), // 20s
        }];
        let r = RotationDrillReport::from_per_agent("test", 1, 500, 5_000, agents, 0, 100);
        assert!(r.passes_v07_floors);
        assert!(r.passes_v10_floors); // 10/10/20 all under
    }

    #[test]
    fn report_fails_v07_when_prop_p99_above_90s() {
        // 3 agents; one with t_1 at 100s
        let agents = vec![
            AgentRotationTimestamps {
                agent_id: "a".into(),
                t_0_ns: 0,
                t_1_ns: Some(10_000_000_000),
                t_2_ns: Some(20_000_000_000),
            },
            AgentRotationTimestamps {
                agent_id: "b".into(),
                t_0_ns: 0,
                t_1_ns: Some(20_000_000_000),
                t_2_ns: Some(30_000_000_000),
            },
            AgentRotationTimestamps {
                agent_id: "c".into(),
                t_0_ns: 0,
                t_1_ns: Some(100_000_000_000), // 100s — outside p99 floor 90s
                t_2_ns: Some(140_000_000_000),
            },
        ];
        let r = RotationDrillReport::from_per_agent("test", 3, 500, 5_000, agents, 0, 100);
        assert!(!r.passes_v07_floors);
    }

    #[test]
    fn report_fails_v10_when_post_grace_reject_rate_above_01pct() {
        let agents = vec![AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: 0,
            t_1_ns: Some(10_000_000_000),
            t_2_ns: Some(20_000_000_000),
        }];
        let r = RotationDrillReport::from_per_agent(
            "test", 1, 500, 5_000, agents, 10, 1_000, // 1% — above v1.0 floor
        );
        assert!(r.passes_v07_floors);
        assert!(!r.passes_v10_floors);
    }
}
