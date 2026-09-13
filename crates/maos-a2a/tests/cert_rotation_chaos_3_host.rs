//! Story 6.3 AC5 → **retired to its surviving halves by Story 14-2 (AC6.1)**:
//! the synthetic `harness_3_host` drill and `MetricsCollector` are DELETED —
//! they seeded `t_0/t_1/t_2` from `DrillConfig` constants and asserted their
//! own floors green. What remains here are the tests that exercise REAL
//! production logic only: `compute_t_grace` boundary semantics (§7.2.1.a)
//! and the `HandshakeRetryPolicy` retry-class filter. The live rotation
//! drill + the `check-rotation-real-timing` gate own NFR-Sec-13 enforcement;
//! `scenario_5_3`'s post-grace falsifier was ported into the real drill
//! (`t_10_4b_rotation_proven_red_post_grace_boundary`) BEFORE this deletion.

use maos_a2a::chaos::rotation::compute_t_grace;

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
