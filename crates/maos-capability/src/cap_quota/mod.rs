#![forbid(unsafe_code)]

//! Capability **issuance** quota — per admitted scheduler pid, per tumbling window.
//!
//! Story 17-9 (ADR-030 `cap-quota`): every capability token the kernel issues
//! through `issue_with_mediation` occupies one of `budget_per_window` slots of
//! its pid's current window. The order is policy → [`CapQuotaTracker::charge`]
//! → signing → [`CapQuotaTracker::commit`] on success or
//! [`CapQuotaTracker::refund`] on failure, so a policy denial is never charged
//! and an unissued token is never counted.
//!
//! * **Window.** `now_ns / window_ns` of a monotonic clock captured at
//!   construction ([`std::time::Instant`]; never wall time, never
//!   `cap_tokens::monotonic_now_ns`, whose base is rewritten on boot). A slot's
//!   epoch never decreases: the clock is sampled under the pid's entry lock and
//!   a sample older than the slot (or than the highest swept window) is
//!   clamped up to it. Tumbling windows admit at most `2·B` issuances across
//!   one boundary (B at the end of window *w*, B at the start of *w + 1*).
//! * **Capacity is inclusive.** Exactly B charges per window succeed; *exhausted*
//!   means all B slots are reserved, pending signings included, so it may be
//!   transient.
//! * **Signals.** `commit` reports [`QuotaSignal::Pressure`] when `issued` first
//!   reaches ⌈0.80·B⌉ and [`QuotaSignal::Limit`] at ⌈0.95·B⌉ (both on one
//!   commit when they coincide); `charge` reports the first refusal per window
//!   as [`Exhausted::first_in_window`]. Each fires at most once per pid and
//!   window and is re-armed by rollover. The kernel turns them into kind-7
//!   `cap.quota:*` audit rows — operator-facing only. They are the issuance
//!   quota's signals, **not** ADR-016's Spirit-facing context-token
//!   `ContextPressure`/`ContextLimit`/`EContextExhausted`
//!   (`18-5-context-token-budget-accounting`).
//! * **Rolled charges.** A charge is *rolled* once its pid's slot was advanced
//!   by a later `charge` or removed by a sweep; the clock alone rolls nothing.
//!   `commit`/`refund` of a rolled charge change nothing and signal nothing.
//! * **Scope.** One slot per admitted scheduler pid — one incarnation. Hot swap
//!   keeps the pid and its window; cold swap, crash restart, re-admission and
//!   every fresh Worker bind get a new pid and therefore fresh capacity. This
//!   bounds a runaway incarnation, not a Spirit across incarnations, a
//!   principal, or Worker-spawn churn.
//! * **Bounded state.** Lazy and activity-driven: the first charge that observes
//!   a window above the highest swept, once its own reservation is made, removes
//!   every slot of a strictly older window. Idle stale slots persist until
//!   another charge arrives.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

/// An issuance budget per tumbling window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuanceQuota {
    pub budget_per_window: u64,
    pub window: Duration,
}

/// The one named budget source (D-17-9-FW-04 A): 60,000 issuances per admitted
/// pid per 60 s window — an operator-selected throttle (a runaway exhausts in
/// ≈2.3 s at the measured ≈26,000 mediated issuances/s), not a measured bound
/// on legitimate demand.
pub const DEFAULT_ISSUANCE_QUOTA: IssuanceQuota = IssuanceQuota {
    budget_per_window: 60_000,
    window: Duration::from_secs(60),
};

/// Monotonic nanoseconds since an arbitrary origin; injectable for tests.
pub trait QuotaClock: Send + Sync + std::fmt::Debug {
    fn now_ns(&self) -> u64;
}

#[derive(Debug)]
struct ProductionClock {
    origin: Instant,
}
impl QuotaClock for ProductionClock {
    fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }
}

/// An issuance-quota signal; `intent()` is its kind-7 audit-row intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaSignal {
    Pressure,
    Limit,
    Exhausted,
}
impl QuotaSignal {
    pub fn intent(self) -> &'static str {
        match self {
            Self::Pressure => "cap.quota:pressure",
            Self::Limit => "cap.quota:limit",
            Self::Exhausted => "cap.quota:exhausted",
        }
    }
}
const PRESSURE_BIT: u8 = 1;
const LIMIT_BIT: u8 = 2;

/// One admission, consumed by `commit` or `refund`; holds no entry guard.
#[derive(Debug)]
#[must_use]
pub struct Charge {
    spirit_pid: u32,
    window: u64,
    owner: Weak<State>,
}
impl Charge {
    /// The admission window this charge belongs to.
    pub fn window(&self) -> u64 {
        self.window
    }
}

/// The committed count and newly crossed thresholds, pressure before limit.
/// A rolled charge commits nothing: `issued` 0, no signals.
#[derive(Debug)]
pub struct Committed {
    pub window: u64,
    pub issued: u64,
    pub signals: Signals,
}

/// At most two threshold signals, stored inline without allocation.
#[derive(Debug, Default)]
pub struct Signals {
    values: [Option<QuotaSignal>; 2],
}
impl Signals {
    pub fn is_empty(&self) -> bool {
        self.values[0].is_none()
    }
    fn push(&mut self, signal: QuotaSignal) {
        let index = usize::from(self.values[0].is_some());
        self.values[index] = Some(signal);
    }
}
impl IntoIterator for Signals {
    type Item = QuotaSignal;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<QuotaSignal>, 2>>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter().flatten()
    }
}
impl<T: AsRef<[QuotaSignal]>> PartialEq<T> for Signals {
    fn eq(&self, other: &T) -> bool {
        self.values
            .iter()
            .flatten()
            .copied()
            .eq(other.as_ref().iter().copied())
    }
}

/// Every slot of `window` is reserved; `first_in_window` is set on the first
/// refusal per pid and window only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exhausted {
    pub spirit_pid: u32,
    pub window: u64,
    pub first_in_window: bool,
}

#[derive(Debug)]
struct Slot {
    window: u64,
    reserved: u64,
    issued: u64,
    fired: u8,
    exhausted: bool,
}
impl Slot {
    fn new(window: u64) -> Self {
        Self {
            window,
            reserved: 0,
            issued: 0,
            fired: 0,
            exhausted: false,
        }
    }
}

#[derive(Debug)]
#[maos_attrs::i9_exempt(
    reason = "per-pid issuance window slots; strictly older windows swept lazily by the next charge, structural-state caching per I9"
)]
struct State {
    slots: DashMap<u32, Slot>,
    quota: IssuanceQuota,
    clock: Arc<dyn QuotaClock>,
    window_ns: u128,
    pressure_at: u64,
    limit_at: u64,
    swept: AtomicU64,
}

/// Per-pid issuance accounting; clones share one state.
#[derive(Debug, Clone)]
pub struct CapQuotaTracker {
    state: Arc<State>,
}
impl CapQuotaTracker {
    /// [`DEFAULT_ISSUANCE_QUOTA`] on the production monotonic clock.
    pub fn new() -> Self {
        Self::with_quota(DEFAULT_ISSUANCE_QUOTA)
    }
    pub fn with_quota(quota: IssuanceQuota) -> Self {
        Self::with_clock(
            quota,
            Arc::new(ProductionClock {
                origin: Instant::now(),
            }),
        )
    }
    /// # Panics
    /// A zero budget or a zero window is refused.
    pub fn with_clock(quota: IssuanceQuota, clock: Arc<dyn QuotaClock>) -> Self {
        assert!(
            quota.budget_per_window >= 1,
            "IssuanceQuota budget_per_window must be at least 1"
        );
        assert!(
            quota.window > Duration::ZERO,
            "IssuanceQuota window must be at least 1 ns"
        );
        let budget = u128::from(quota.budget_per_window);
        Self {
            state: Arc::new(State {
                slots: DashMap::new(),
                quota,
                clock,
                window_ns: quota.window.as_nanos(),
                pressure_at: (budget * 80).div_ceil(100) as u64,
                limit_at: (budget * 95).div_ceil(100) as u64,
                swept: AtomicU64::new(0),
            }),
        }
    }
    pub fn quota(&self) -> IssuanceQuota {
        self.state.quota
    }
    fn window(&self) -> u64 {
        (u128::from(self.state.clock.now_ns()) / self.state.window_ns) as u64
    }
    /// Reserve one issuance for `spirit_pid` in the current window.
    pub fn charge(&self, spirit_pid: u32) -> Result<Charge, Exhausted> {
        let result = self.reserve(spirit_pid);
        // The entry guard is dropped: `retain` deadlocks against a held reference.
        self.sweep(match &result {
            Ok(charge) => charge.window,
            Err(exhausted) => exhausted.window,
        });
        result
    }
    fn reserve(&self, spirit_pid: u32) -> Result<Charge, Exhausted> {
        let mut slot = self
            .state
            .slots
            .entry(spirit_pid)
            .or_insert_with(|| Slot::new(0));
        // The authoritative (only) clock sample is taken under the entry lock;
        // a slot's epoch never decreases, nor starts below the swept window.
        let window = self
            .window()
            .max(slot.window)
            .max(self.state.swept.load(Ordering::Acquire));
        if window > slot.window {
            *slot = Slot::new(window);
        }
        if slot.reserved == self.state.quota.budget_per_window {
            let first_in_window = !slot.exhausted;
            slot.exhausted = true;
            return Err(Exhausted {
                spirit_pid,
                window,
                first_in_window,
            });
        }
        slot.reserved += 1;
        Ok(Charge {
            spirit_pid,
            window,
            owner: Arc::downgrade(&self.state),
        })
    }
    /// Count a successfully issued token; returns the thresholds it newly reached.
    pub fn commit(&self, charge: Charge) -> Committed {
        let mut committed = Committed {
            window: charge.window,
            issued: 0,
            signals: Signals::default(),
        };
        if !Weak::ptr_eq(&charge.owner, &Arc::downgrade(&self.state)) {
            return committed;
        }
        if let Some(mut slot) = self.state.slots.get_mut(&charge.spirit_pid) {
            if slot.window == charge.window {
                slot.issued += 1;
                committed.issued = slot.issued;
                for (threshold, bit, signal) in [
                    (self.state.pressure_at, PRESSURE_BIT, QuotaSignal::Pressure),
                    (self.state.limit_at, LIMIT_BIT, QuotaSignal::Limit),
                ] {
                    if slot.issued >= threshold && slot.fired & bit == 0 {
                        slot.fired |= bit;
                        committed.signals.push(signal);
                    }
                }
            }
        }
        committed
    }
    /// Release the reservation of a failed issuance; never touches `issued`,
    /// fired signals or the exhaustion latch.
    pub fn refund(&self, charge: Charge) {
        if !Weak::ptr_eq(&charge.owner, &Arc::downgrade(&self.state)) {
            return;
        }
        if let Some(mut slot) = self.state.slots.get_mut(&charge.spirit_pid) {
            if slot.window == charge.window {
                slot.reserved = slot.reserved.saturating_sub(1);
            }
        }
    }
    /// The first caller to observe a window above the highest swept removes
    /// every slot of a strictly older window.
    fn sweep(&self, window: u64) {
        let mut previous = self.state.swept.load(Ordering::Acquire);
        while window > previous {
            match self.state.swept.compare_exchange_weak(
                previous,
                window,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.state.slots.retain(|_, slot| slot.window >= window);
                    break;
                }
                Err(observed) => previous = observed,
            }
        }
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

    #[derive(Debug, Default)]
    struct Clock(AtomicU64);
    impl QuotaClock for Clock {
        fn now_ns(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn tracker(budget: u64) -> (Arc<Clock>, CapQuotaTracker) {
        let clock = Arc::new(Clock::default());
        let quota = IssuanceQuota {
            budget_per_window: budget,
            window: Duration::from_nanos(10),
        };
        (clock.clone(), CapQuotaTracker::with_clock(quota, clock))
    }

    fn windows(t: &CapQuotaTracker) -> Vec<(u32, u64)> {
        let mut slots: Vec<_> = t.state.slots.iter().map(|s| (*s.key(), s.window)).collect();
        slots.sort_unstable();
        slots
    }

    #[test]
    fn overlapping_sweeps_never_remove_a_slot_of_the_current_or_a_later_window() {
        let (_, t) = tracker(4);
        t.state.slots.insert(1, Slot::new(4));
        t.state.slots.insert(2, Slot::new(5));
        t.state.slots.insert(3, Slot::new(6));
        // A sweep for window 5 whose `retain` runs after a later charge already
        // published window 6 (and its slot): only strictly older slots go.
        t.sweep(5);
        assert_eq!(windows(&t), [(2, 5), (3, 6)]);
        // A stale, lower sweep request never runs again.
        t.state.slots.insert(4, Slot::new(4));
        t.sweep(4);
        assert_eq!(windows(&t), [(2, 5), (3, 6), (4, 4)]);
        t.sweep(6);
        assert_eq!(windows(&t), [(3, 6)]);
        assert_eq!(t.state.swept.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn a_refund_or_commit_never_recreates_a_swept_slot() {
        let (clock, t) = tracker(1);
        let held = t.charge(7).unwrap();
        let late = t.charge(9).unwrap();
        clock.0.store(10, Ordering::SeqCst);
        t.commit(t.charge(8).unwrap());
        assert_eq!(windows(&t), [(8, 1)]);
        t.refund(held);
        let c = t.commit(late);
        assert_eq!((c.issued, c.signals.is_empty()), (0, true));
        assert_eq!(windows(&t), [(8, 1)]);
    }

    #[test]
    fn ten_thousand_idle_pids_are_swept_by_one_charge_after_rollover() {
        let (clock, t) = tracker(1);
        for pid in 0..10_000 {
            t.commit(t.charge(pid).unwrap());
        }
        assert_eq!(t.state.slots.len(), 10_000);
        clock.0.store(10, Ordering::SeqCst);
        t.commit(t.charge(10_000).unwrap());
        assert_eq!(windows(&t), [(10_000, 1)]);
    }

    #[test]
    fn a_slot_created_after_a_sweep_never_starts_below_the_swept_window() {
        let (clock, t) = tracker(1);
        clock.0.store(20, Ordering::SeqCst);
        t.commit(t.charge(8).unwrap());
        // An adversarial clock rewound below the swept window cannot open a
        // fresh, older window for a pid whose slot was swept.
        clock.0.store(0, Ordering::SeqCst);
        assert_eq!(t.charge(7).unwrap().window(), 2);
        assert!(t.charge(7).is_err());
    }
}
