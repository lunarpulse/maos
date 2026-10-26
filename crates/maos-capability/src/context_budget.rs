#![forbid(unsafe_code)]

//! ADR-016 context-token ledger — per admitted scheduler pid (Story 18-5).
//!
//! Only Spirit reports move it; the kernel never estimates tokens. Occupancy
//! is absolute (D-18-5-08 A): the last report wins, a threshold re-arms when a
//! report falls back below it, and exhaustion clears on a report within the
//! window, so a distilling Spirit recovers without a restart. Window 0 turns
//! accounting off. A hot-swap successor keeps the pid's slot until its first
//! report; unload, crash and revocation drop it (`revoke_all_for_pid`).
//!
//! Thresholds are `⌈window · permille / 1000⌉`: Butler 8192 → 6554 / 7783,
//! Researcher 16384 → 13108 / 15565.

use dashmap::DashMap;
use maos_domain::invariants::i1::Scope;
use maos_domain::ports::context_budget::{ContextLevel, ContextStatus};

/// A Spirit's manifest window and thresholds, fixed by the root.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContextWindow {
    pub size: u32,
    pub pressure_permille: u16,
    pub limit_permille: u16,
}

impl ContextWindow {
    fn level(self, used: u32) -> ContextLevel {
        let at = |permille: u16| (u64::from(self.size) * u64::from(permille)).div_ceil(1000);
        if self.size == 0 {
            ContextLevel::Within
        } else if used > self.size {
            ContextLevel::Exhausted
        } else if u64::from(used) >= at(self.limit_permille) {
            ContextLevel::Limit
        } else if u64::from(used) >= at(self.pressure_permille) {
            ContextLevel::Pressure
        } else {
            ContextLevel::Within
        }
    }
}

/// One upward threshold crossing; the adapter turns it into one typed frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crossing {
    pub level: ContextLevel,
    pub threshold_permille: u16,
}

/// `true` for ADR-016's exits: distil (`ProviderInfer`, `MemWrite`) and hand
/// off (`IacSend`). Every other scope, including any added later, is refused
/// to an exhausted pid.
pub fn is_exit(scope: &Scope) -> bool {
    matches!(
        scope,
        Scope::ProviderInfer { .. } | Scope::MemWrite { .. } | Scope::IacSend { .. }
    )
}

/// The per-pid ledger: the level of each pid's last report.
#[derive(Debug, Default)]
pub struct ContextBudget {
    slots: DashMap<u32, ContextLevel>,
}

impl ContextBudget {
    /// Record `used` for `pid` against `window`; returns the typed status and
    /// the upward crossings (pressure before limit; exhaustion emits none —
    /// its signal is the refusal).
    pub fn report(
        &self,
        pid: u32,
        window: ContextWindow,
        used: u32,
    ) -> (ContextStatus, Vec<Crossing>) {
        let level = window.level(used);
        let previous = self
            .slots
            .insert(pid, level)
            .unwrap_or(ContextLevel::Within);
        let crossings = [
            (ContextLevel::Pressure, window.pressure_permille),
            (ContextLevel::Limit, window.limit_permille),
        ]
        .into_iter()
        .filter(|&(threshold, _)| previous < threshold && level >= threshold)
        .map(|(level, threshold_permille)| Crossing {
            level,
            threshold_permille,
        })
        .collect();
        let status = ContextStatus {
            used,
            window: window.size,
            level,
            crossed: (level > previous).then_some(level),
        };
        (status, crossings)
    }

    /// `true` when `pid`'s last report exceeded its window.
    pub fn exhausted(&self, pid: u32) -> bool {
        self.slots
            .get(&pid)
            .is_some_and(|level| *level == ContextLevel::Exhausted)
    }

    /// The issuance gate: an exhausted pid is admitted only for the exits.
    pub fn admits(&self, pid: u32, scope: &Scope) -> bool {
        is_exit(scope) || !self.exhausted(pid)
    }

    /// Drop `pid`'s slot (unload, crash, revocation).
    pub fn remove(&self, pid: u32) {
        self.slots.remove(&pid);
    }
}
