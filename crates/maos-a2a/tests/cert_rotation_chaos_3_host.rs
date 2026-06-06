//! Story 6.3 AC5 — NFR-Sec-13 mTLS cert rotation chaos test harness.
//!
//! v0.5 calibration phase per architecture §7.2.1.b — metrics MEASURED and
//! REPORTED but NOT enforced.

use maos_a2a::chaos::{
    harness_3_host::{run_drill, DrillConfig},
    rotation::compute_t_grace,
};

#[tokio::test]
async fn scenario_5_1_3_host_happy_path_calibration_baseline() {
    // 3-host synthetic chaos drill on a happy path; assert all three timing
    // distributions pass v0.7 floors AND v1.0 cert_post_grace_reject floor;
    // passing in calibration mode generates baseline data.
    let cfg = DrillConfig::default();
    let report = run_drill(cfg).await;
    assert!(report.passes_v07_floors);
    assert!(report.passes_v10_floors);
    assert_eq!(report.per_agent.len(), 3);
    // All three p99 distributions populated.
    assert!(report.revocation_propagation_p99_ms > 0);
    assert!(report.re_handshake_p99_ms > 0);
    assert!(report.end_to_end_p99_ms > 0);
}

#[tokio::test]
async fn scenario_5_2_one_agent_lagged_60s_breach_in_calibration() {
    // 3-host drill with one agent's OCSP poll lagged 60s; revocation
    // propagation p50 still ≤30s but p99 floats above floor; in calibration
    // mode the test reports the breach without failing.
    let cfg = DrillConfig {
        drill_id: "lag-60s".into(),
        target_propagation_ms_per_agent: vec![10_000, 15_000, 100_000],
        target_re_handshake_ms_per_agent: vec![5_000, 8_000, 12_000],
        ..DrillConfig::default()
    };
    let report = run_drill(cfg).await;
    assert!(report.revocation_propagation_p50_ms <= 30_000);
    assert!(report.revocation_propagation_p99_ms >= 90_000);
    // Calibration mode: failing the v0.7 floor reports without panic.
    assert!(!report.passes_v07_floors);
}

#[tokio::test]
async fn scenario_5_3_post_grace_reject_increments() {
    // Per AC5 §5.3 — agent presents old cert AFTER t_revoke + T_grace.
    // cert_post_grace_reject increments; v1.0 floor breach is reported.
    let cfg = DrillConfig {
        drill_id: "post-grace-1".into(),
        post_grace_reject_count: 1,
        post_grace_total_count: 1_000,
        ..DrillConfig::default()
    };
    let report = run_drill(cfg).await;
    // 1/1000 = 0.1% — exactly at the boundary
    assert!(report.post_grace_reject_rate <= 0.001);
    // v1.0 floor: ≤0.1% — at the boundary, still passes
    assert!(report.passes_v10_floors);

    // Above the boundary
    let cfg_above = DrillConfig {
        drill_id: "post-grace-2".into(),
        post_grace_reject_count: 2,
        post_grace_total_count: 1_000,
        ..DrillConfig::default()
    };
    let report_above = run_drill(cfg_above).await;
    assert!(report_above.post_grace_reject_rate > 0.001);
    assert!(!report_above.passes_v10_floors);
}

#[test]
fn scenario_5_4_t_grace_boundary_semantics() {
    // Per AC5 §5.4 — T_grace boundary test.
    //   compute_t_grace(p99_handshake_rtt_ms, days_of_history) = max(2 × p99, 5s)
    //   Cold deployment (<30 days) floors at 500ms.

    // Cold deployment, observed p99 = 100ms (below 500ms floor)
    let t = compute_t_grace(100, 7);
    assert_eq!(t, std::time::Duration::from_millis(5_000));

    // Cold deployment, observed p99 = 3000ms → 2×3000 = 6000ms > 5000ms
    let t = compute_t_grace(3000, 7);
    assert_eq!(t, std::time::Duration::from_millis(6_000));

    // Steady state, observed p99 = 2000ms → 2×2000 = 4000ms; floor 5000ms
    let t = compute_t_grace(2000, 30);
    assert_eq!(t, std::time::Duration::from_millis(5_000));
}

#[test]
fn scenario_5_5_retry_policy_correctness() {
    // Per AC5 §5.5 — handshake retry per §7.2.1.a.
    use maos_a2a::HandshakeRetryPolicy;
    let p = HandshakeRetryPolicy::default();
    assert_eq!(p.backoff_ms, vec![100, 300, 1000]);
    assert_eq!(p.max_attempts, 4); // 1 original + 3 retries
    assert_eq!(p.jitter_pct, 20);
    // attempt 1 = original (no delay)
    assert_eq!(p.delay_for_attempt(1, Some(0)), 0);
    // attempt 2: 100ms ± 20% = [80, 120]
    let d = p.delay_for_attempt(2, Some(0));
    assert!((80..=120).contains(&d));
    // attempt 3: 300ms ± 20% = [240, 360]
    let d = p.delay_for_attempt(3, Some(0));
    assert!((240..=360).contains(&d));
    // attempt 4: 1000ms ± 20% = [800, 1200]
    let d = p.delay_for_attempt(4, Some(0));
    assert!((800..=1200).contains(&d));
}

#[test]
fn retry_policy_only_retries_bad_cert_or_expired_per_arch_7_2_1_a() {
    use maos_a2a::error::{A2AError, HandshakeFailureClass};
    use maos_a2a::HandshakeRetryPolicy;
    let p = HandshakeRetryPolicy::default();
    assert!(p.is_retryable(&A2AError::HandshakeFailed {
        class: HandshakeFailureClass::BadCertificate,
        message: "leaf malformed".into(),
    }));
    assert!(p.is_retryable(&A2AError::HandshakeFailed {
        class: HandshakeFailureClass::CertExpired,
        message: "leaf expired".into(),
    }));
    // Other handshake failures bubble up immediately per §7.2.1.a.
    assert!(!p.is_retryable(&A2AError::HandshakeFailed {
        class: HandshakeFailureClass::Other,
        message: "DECRYPT_ERROR: alert".into(),
    }));
    assert!(!p.is_retryable(&A2AError::TransportFailed("connection reset".into())));
}
