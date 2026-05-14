#![forbid(unsafe_code)]

//! Capability quota — per-Spirit budget tracker.
//!
//! Per architecture §4.6: tracks `tokens_consumed_this_window` against a
//! per-Spirit `budget_limit`, emitting pressure/limit events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;

use maos_domain::ports::capability::CapError;

/// Pressure threshold — 80% utilization.
pub const PRESSURE_THRESHOLD: f64 = 0.80;
/// Limit threshold — 95% utilization.
pub const LIMIT_THRESHOLD: f64 = 0.95;
/// Exhausted threshold — 100% utilization.
pub const EXHAUSTED_THRESHOLD: f64 = 1.00;

/// Quota state per Spirit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuotaState {
    /// Healthy — under pressure threshold.
    Healthy(f64),
    /// Pressure — over 80%.
    Pressure(f64),
    /// Limit — over 95%.
    Limit(f64),
    /// Exhausted — over 100%.
    Exhausted(f64),
}

/// Per-Spirit budget tracker.
#[derive(Debug)]
pub struct CapQuotaTracker {
    inner: Arc<DashMap<u32, AtomicU64>>,
    /// Per-Spirit budgets.
    limits: Arc<DashMap<u32, u64>>,
    /// Tracks which pressure thresholds have already fired per Spirit per window.
    pressure_fired: Arc<DashMap<u32, AtomicU64>>,
}

impl CapQuotaTracker {
    /// Create a new quota tracker.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            limits: Arc::new(DashMap::new()),
            pressure_fired: Arc::new(DashMap::new()),
        }
    }

    /// Set the budget for a Spirit.
    pub fn set_budget(&self, spirit_id: u32, budget: u64) {
        self.limits.insert(spirit_id, budget);
    }

    /// Check and increment the quota for a Spirit.
    /// Only increments the counter if the check passes (no budget corruption).
    pub fn check_and_increment(
        &self,
        spirit_id: u32,
        cost: u64,
        budget: u64,
    ) -> Result<QuotaState, CapError> {
        if budget == 0 {
            return Err(CapError::ContextExhausted { spirit_id });
        }
        let entry = self.inner.entry(spirit_id).or_insert_with(|| AtomicU64::new(0));
        let prev = entry.load(Ordering::Relaxed);
        let projected = prev + cost;
        let ratio = projected as f64 / budget as f64;

        if ratio >= EXHAUSTED_THRESHOLD {
            return Err(CapError::ContextExhausted { spirit_id });
        }

        // Only increment after confirming we're under budget
        entry.fetch_add(cost, Ordering::Relaxed);

        if ratio >= LIMIT_THRESHOLD {
            Ok(QuotaState::Limit(ratio))
        } else if ratio >= PRESSURE_THRESHOLD {
            Ok(QuotaState::Pressure(ratio))
        } else {
            Ok(QuotaState::Healthy(ratio))
        }
    }

    /// One-shot pressure event: returns true the FIRST time the Spirit
    /// crosses the pressure threshold this window.
    pub fn try_fire_pressure(&self, spirit_id: u32) -> bool {
        let entry = self.pressure_fired.entry(spirit_id).or_insert_with(|| AtomicU64::new(0));
        let prev = entry.fetch_or(0b01, Ordering::Relaxed);
        (prev & 0b01) == 0
    }

    /// One-shot limit event: returns true the FIRST time the Spirit
    /// crosses the limit threshold this window.
    pub fn try_fire_limit(&self, spirit_id: u32) -> bool {
        let entry = self.pressure_fired.entry(spirit_id).or_insert_with(|| AtomicU64::new(0));
        let prev = entry.fetch_or(0b10, Ordering::Relaxed);
        (prev & 0b10) == 0
    }

    /// Reset the window counter for a Spirit.
    pub fn reset_window(&self, spirit_id: u32) {
        if let Some(entry) = self.inner.get(&spirit_id) {
            entry.store(0, Ordering::Relaxed);
        }
    }

    /// Get the current consumed count for a Spirit.
    pub fn consumed(&self, spirit_id: u32) -> u64 {
        self.inner
            .get(&spirit_id)
            .map(|e| e.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for CapQuotaTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_and_increment_healthy() {
        let tracker = CapQuotaTracker::new();
        let state = tracker.check_and_increment(7, 10, 100).unwrap();
        assert!(matches!(state, QuotaState::Healthy(_)));
    }

    #[test]
    fn check_and_increment_pressure() {
        let tracker = CapQuotaTracker::new();
        let state = tracker.check_and_increment(7, 85, 100).unwrap();
        assert!(matches!(state, QuotaState::Pressure(_)));
    }

    #[test]
    fn check_and_increment_limit() {
        let tracker = CapQuotaTracker::new();
        let state = tracker.check_and_increment(7, 96, 100).unwrap();
        assert!(matches!(state, QuotaState::Limit(_)));
    }

    #[test]
    fn check_and_increment_exhausted() {
        let tracker = CapQuotaTracker::new();
        let err = tracker.check_and_increment(7, 101, 100);
        assert!(matches!(err, Err(CapError::ContextExhausted { spirit_id: 7 })));
    }

    #[test]
    fn reset_window_clears_counter() {
        let tracker = CapQuotaTracker::new();
        tracker.check_and_increment(7, 50, 100).unwrap();
        assert_eq!(tracker.consumed(7), 50);
        tracker.reset_window(7);
        assert_eq!(tracker.consumed(7), 0);
    }
}
