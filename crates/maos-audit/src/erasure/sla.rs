//! Story 9.2 — erasure SLA logical-clock primitive.
//!
//! NFR-Aud-13 requires an audit-log entry within 24h of acceptance.  This
//! module provides a pure logical-tick formulation so the test can prove the
//! SLA boundary without real sleeps.  Two windows are modeled:
//!
//! * `within_24h` — the NFR-Aud-13 hard floor (one day).  Every forget cascade
//!   MUST journal its `principal.forget` frame within this window.
//! * `within_sla` — the configured `erasure_sla_days` window (30 default /
//!   7 enterprise).  This is the operator-tunable maximum, always >= 24h.

#![forbid(unsafe_code)]

/// Logical ticks per simulated day.  A day is a coarse-grained unit; the test
/// uses ticks directly so the assertion is deterministic.
pub const TICKS_PER_DAY: u64 = 1_000;

/// Number of logical ticks in the 24h acceptance-to-completion window
/// (NFR-Aud-13).  Equal to `TICKS_PER_DAY` — a day IS the 24h floor.
pub const TICKS_PER_24H: u64 = TICKS_PER_DAY;

/// Configurable SLA knob.  Default = 30 days; enterprise = 7 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErasureSlaConfig {
    pub days: u32,
}

impl Default for ErasureSlaConfig {
    fn default() -> Self {
        Self { days: 30 }
    }
}

impl ErasureSlaConfig {
    pub fn new(days: u32) -> Self {
        Self { days }
    }

    pub fn ticks_allowed(&self) -> u64 {
        self.days as u64 * TICKS_PER_DAY
    }

    /// Configured-window check (`erasure_sla_days`).
    pub fn within_sla(&self, accepted_tick: u64, completed_tick: u64) -> bool {
        completed_tick.saturating_sub(accepted_tick) <= self.ticks_allowed()
    }

    /// NFR-Aud-13 24h hard-floor check.  Independent of the configured window
    /// — every cascade must satisfy this regardless of `erasure_sla_days`.
    pub fn within_24h(accepted_tick: u64, completed_tick: u64) -> bool {
        completed_tick.saturating_sub(accepted_tick) <= TICKS_PER_24H
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_30_day_sla_passes_within_window() {
        let cfg = ErasureSlaConfig::default();
        assert!(cfg.within_sla(0, 29 * TICKS_PER_DAY));
        assert!(cfg.within_sla(0, 30 * TICKS_PER_DAY));
    }

    #[test]
    fn default_30_day_sla_misses_outside_window() {
        let cfg = ErasureSlaConfig::default();
        assert!(!cfg.within_sla(0, 31 * TICKS_PER_DAY));
    }

    #[test]
    fn enterprise_7_day_sla_has_tighter_boundary() {
        let cfg = ErasureSlaConfig::new(7);
        assert!(cfg.within_sla(0, 7 * TICKS_PER_DAY));
        assert!(!cfg.within_sla(0, 8 * TICKS_PER_DAY));
    }

    #[test]
    fn same_tick_is_always_within_sla() {
        let cfg = ErasureSlaConfig::new(1);
        assert!(cfg.within_sla(42, 42));
    }

    #[test]
    fn backwards_clock_is_treated_as_zero_delta() {
        let cfg = ErasureSlaConfig::new(1);
        // saturating_sub protects against negative wrap.
        assert!(cfg.within_sla(100, 50));
    }
}
