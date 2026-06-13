//! Story 9.2 (AC2 Engineering AC3 / NFR-Aud-13) — erasure SLA integration test.
//!
//! Exercises the logical-tick SLA primitive against boundary conditions and a
//! planted miss, asserting both the NFR-Aud-13 24h hard floor and the
//! configurable `erasure_sla_days` window (30 default / 7 enterprise).

#![forbid(unsafe_code)]

use maos_audit::erasure::sla::{ErasureSlaConfig, TICKS_PER_24H, TICKS_PER_DAY};

#[test]
fn nfr_aud_13_24h_hard_floor_boundary() {
    // An audit-log entry completed within the 24h window is in SLA.
    assert!(
        ErasureSlaConfig::within_24h(0, TICKS_PER_24H),
        "completion at exactly 24h must be within the floor"
    );
    // One tick past 24h is a violation.
    assert!(
        !ErasureSlaConfig::within_24h(0, TICKS_PER_24H + 1),
        "completion one tick past 24h must violate the floor"
    );
    // The 24h floor is independent of the configured window.
    assert!(
        !ErasureSlaConfig::within_24h(0, TICKS_PER_DAY * 2),
        "a 2-day completion violates the 24h floor regardless of config"
    );
}

#[test]
fn default_30_day_window_boundary() {
    let cfg = ErasureSlaConfig::default();
    assert_eq!(cfg.days, 30, "default erasure_sla_days is 30");
    assert!(
        cfg.within_sla(0, 30 * TICKS_PER_DAY),
        "completion at exactly 30 days is within the configured window"
    );
    assert!(
        !cfg.within_sla(0, 30 * TICKS_PER_DAY + 1),
        "completion one tick past 30 days violates the configured window"
    );
}

#[test]
fn enterprise_7_day_knob() {
    let cfg = ErasureSlaConfig::new(7);
    assert_eq!(cfg.days, 7);
    assert!(
        cfg.within_sla(0, 7 * TICKS_PER_DAY),
        "enterprise 7-day window admits a 7-day completion"
    );
    assert!(
        !cfg.within_sla(0, 8 * TICKS_PER_DAY),
        "enterprise 7-day window rejects an 8-day completion"
    );
}

#[test]
fn planted_miss_scenario_is_detected() {
    // Planted miss: a forget cascade whose audit-log entry lands PAST the SLA
    // window.  The SLA primitive must flag it so a watchdog can alert.
    let cfg = ErasureSlaConfig::default();
    let accepted_tick = 1_000;
    let miss_tick = accepted_tick + (cfg.ticks_allowed()) + 1;
    assert!(
        !cfg.within_sla(accepted_tick, miss_tick),
        "a completion past the configured window is a planted miss"
    );
    assert!(
        !ErasureSlaConfig::within_24h(accepted_tick, miss_tick),
        "the planted miss also violates the 24h floor"
    );
    // A well-behaved completion inside both windows is clean.
    let good_tick = accepted_tick + 10;
    assert!(cfg.within_sla(accepted_tick, good_tick));
    assert!(ErasureSlaConfig::within_24h(accepted_tick, good_tick));
}

#[test]
fn backwards_clock_is_clamped_not_wrapped() {
    // A monotonic-clock glitch (completed before accepted) must not wrap to a
    // huge delta and false-flag a violation.
    let cfg = ErasureSlaConfig::new(1);
    assert!(
        cfg.within_sla(100, 50),
        "backwards clock is clamped to a zero delta, not flagged"
    );
}
