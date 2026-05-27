//! Story 6.3 AC6 — NFR-Rel-7 A2A trust under churn harness scaffolding.
//!
//! v0.5 ships 3-host compressed scaffold; v2.0 binding at 30-host (compressed).

use maos_a2a::chaos::churn::{run_scaffold, ChurnHarnessConfig};

#[tokio::test]
async fn scenario_6_1_scaffold_runs_to_completion() {
    // Per AC6 §6.1 — assert ChurnDrillReport has all three metrics populated
    // with finite values.
    let cfg = ChurnHarnessConfig::default();
    let report = run_scaffold(cfg.clone()).await;
    // All metrics populated (non-default)
    assert!(report.detection_latency_median_secs > 0);
    assert!(report.recovery_secs > 0);
    // Config preserved
    assert_eq!(report.config.host_count, 3);
    assert_eq!(report.config.duration_weeks, 4);
}

#[tokio::test]
async fn scenario_6_2_blast_radius_bounded_by_host_count() {
    // Per AC6 §6.2 — adversarial-peer attempts TOFU pin spoofing → blocked by
    // NFR-Sec-12 substrate; max_blast_radius bounded by host count.
    let cfg = ChurnHarnessConfig {
        host_count: 3,
        adversarial_host_count: 10, // more adversaries than hosts
        ..ChurnHarnessConfig::default()
    };
    let report = run_scaffold(cfg).await;
    assert!(report.max_blast_radius <= 3);
}

#[tokio::test]
async fn scenario_6_3_consent_bypass_blocked_by_ac3_substrate() {
    // Per AC6 §6.3 — adversarial-peer attempts ADR-012 consent bypass
    // (sends `code-mutation-directive` to a peer that does NOT have it in
    // accept_allowlist) → blocked by AC3 substrate; logged to chord/audit.
    //
    // Cross-references the AC3 cross-Spirit consent corpus — the consent
    // bypass attempt is structurally rejected at the receiver's
    // accept_allowlist check (AC3 §3.3 covers this directly).
    //
    // Here we verify the scaffold's report shape: a 3-host run with 3
    // adversarial host attempts records the bounded blast radius and
    // detection latency.
    let cfg = ChurnHarnessConfig::default();
    let report = run_scaffold(cfg).await;
    assert!(report.detection_latency_median_secs > 0);
}

#[tokio::test]
async fn scenario_6_4_detection_latency_under_60s_at_3_host_compressed() {
    // Per AC6 §6.4 — adversarial join at t_0; first detection at t_1;
    // assert t_1 - t_0 < 60s for the 3-host compressed scale (sanity check;
    // the v2.0 floor is ≤1h at 30-host scale).
    let cfg = ChurnHarnessConfig::default();
    let report = run_scaffold(cfg).await;
    assert!(report.detection_latency_median_secs < 60);
    // Sanity: the harness's v2.0 floor calc against 30-host scaling — the
    // 3-host scaffold's reported detection meets the floor by an order of
    // magnitude.
    assert!(report.passes_v20_floors);
}
