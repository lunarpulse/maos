//! Story 17-9 AC2.5 — issuance-quota tracker behavior through the public
//! charge/commit/refund API only (default features; CR-8).
use maos_capability::cap_quota::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

const WINDOW_NS: u64 = 10;

#[derive(Debug, Default)]
struct ManualClock(AtomicU64);
impl ManualClock {
    fn set(&self, ns: u64) {
        self.0.store(ns, Ordering::SeqCst);
    }
}
impl QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn tracker(budget: u64) -> (Arc<ManualClock>, CapQuotaTracker) {
    let clock = Arc::new(ManualClock::default());
    let tracker = CapQuotaTracker::with_clock(
        IssuanceQuota {
            budget_per_window: budget,
            window: Duration::from_nanos(WINDOW_NS),
        },
        clock.clone(),
    );
    (clock, tracker)
}

fn fill(t: &CapQuotaTracker, pid: u32, count: u64) {
    for _ in 0..count {
        t.commit(t.charge(pid).unwrap());
    }
}

#[test]
fn the_default_is_the_one_named_budget_source() {
    assert_eq!(
        DEFAULT_ISSUANCE_QUOTA,
        IssuanceQuota {
            budget_per_window: 60_000,
            window: Duration::from_secs(60)
        }
    );
    assert_eq!(CapQuotaTracker::new().quota(), DEFAULT_ISSUANCE_QUOTA);
    assert_eq!(CapQuotaTracker::default().quota(), DEFAULT_ISSUANCE_QUOTA);
    let quota = IssuanceQuota {
        budget_per_window: 7,
        window: Duration::from_secs(3),
    };
    assert_eq!(CapQuotaTracker::with_quota(quota).quota(), quota);
}

#[test]
#[should_panic(expected = "IssuanceQuota budget_per_window must be at least 1")]
fn a_zero_budget_is_refused() {
    let _ = tracker(0);
}

#[test]
#[should_panic(expected = "IssuanceQuota window must be at least 1 ns")]
fn a_zero_window_is_refused() {
    let _ = CapQuotaTracker::with_quota(IssuanceQuota {
        budget_per_window: 1,
        window: Duration::ZERO,
    });
}

#[test]
fn capacity_is_inclusive_at_small_and_medium_budgets() {
    for budget in [1, 2, 3, 20] {
        let (_, t) = tracker(budget);
        for issued in 1..=budget {
            let c = t.commit(t.charge(7).unwrap());
            assert_eq!((c.window, c.issued), (0, issued), "B = {budget}");
        }
        assert_eq!(
            t.charge(7).unwrap_err(),
            Exhausted {
                spirit_pid: 7,
                window: 0,
                first_in_window: true
            },
            "B = {budget}: charge B + 1 is refused"
        );
        assert!(!t.charge(7).unwrap_err().first_in_window, "B = {budget}");
        // Another pid has independent threshold latches, too.
        for issued in 1..=budget {
            let c = t.commit(t.charge(8).unwrap());
            assert_eq!(c.issued, issued, "B = {budget}");
            assert_eq!(
                c.signals
                    .into_iter()
                    .filter(|s| *s == QuotaSignal::Pressure)
                    .count(),
                usize::from(issued == (budget * 80).div_ceil(100)),
                "B = {budget}, Q commit {issued}"
            );
        }
    }
}

#[test]
fn charges_are_bound_to_their_own_tracker() {
    let (_, a) = tracker(1);
    let (_, b) = tracker(1);
    let pending = b.charge(7).unwrap();
    let refund_pending = b.charge(8).unwrap();
    b.refund(a.charge(8).unwrap());
    assert!(
        b.charge(8).is_err(),
        "foreign refund must not restore capacity"
    );
    let foreign = b.commit(a.charge(7).unwrap());
    assert_eq!(foreign.issued, 0);
    assert!(foreign.signals.is_empty());
    let local = b.commit(pending);
    assert_eq!(local.issued, 1);
    assert_eq!(local.signals, [QuotaSignal::Pressure, QuotaSignal::Limit]);
    b.refund(refund_pending);
}

#[test]
fn thresholds_are_the_ceilings_of_eighty_and_ninety_five_percent() {
    // PF-01: (B, commit that fires Pressure, commit that fires Limit).
    for (budget, pressure_at, limit_at) in
        [(1, 1, 1), (2, 2, 2), (3, 3, 3), (7, 6, 7), (20, 16, 19)]
    {
        let (_, t) = tracker(budget);
        for issued in 1..=budget {
            let expected: Vec<QuotaSignal> = [
                (issued == pressure_at).then_some(QuotaSignal::Pressure),
                (issued == limit_at).then_some(QuotaSignal::Limit),
            ]
            .into_iter()
            .flatten()
            .collect();
            assert_eq!(
                t.commit(t.charge(7).unwrap()).signals,
                expected,
                "B = {budget}, commit {issued}"
            );
        }
    }
}

#[test]
fn the_widest_budget_and_windows_beyond_u64_nanoseconds_never_overflow() {
    let (_, t) = tracker(u64::MAX);
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!((c.issued, c.signals.is_empty()), (1, true));
    // Exactly 2^64 ns, and the widest Duration: the clock's whole range is window 0.
    for window in [Duration::new(18_446_744_073, 709_551_616), Duration::MAX] {
        assert!(window.as_nanos() >= 1u128 << 64);
        let clock = Arc::new(ManualClock::default());
        let t = CapQuotaTracker::with_clock(
            IssuanceQuota {
                budget_per_window: u64::MAX,
                window,
            },
            clock.clone(),
        );
        clock.set(u64::MAX);
        let c = t.commit(t.charge(7).unwrap());
        assert_eq!((c.window, c.issued), (0, 1));
    }
}

#[test]
fn the_window_rolls_at_exactly_window_ns_and_rearms_everything() {
    let (clock, t) = tracker(1);
    for window in 0..3 {
        clock.set(window * WINDOW_NS);
        let c = t.commit(t.charge(7).unwrap());
        assert_eq!((c.window, c.issued), (window, 1));
        assert_eq!(c.signals, [QuotaSignal::Pressure, QuotaSignal::Limit]);
        clock.set((window + 1) * WINDOW_NS - 1);
        assert_eq!(
            t.charge(7).unwrap_err(),
            Exhausted {
                spirit_pid: 7,
                window,
                first_in_window: true
            },
            "one ns before the boundary the window has not rolled"
        );
        assert!(!t.charge(7).unwrap_err().first_in_window);
    }
    // Skipped windows reset like adjacent ones.
    clock.set(100 * WINDOW_NS);
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!((c.window, c.issued), (100, 1));
}

#[test]
fn a_refund_within_the_window_frees_the_slot_without_counting_or_signalling() {
    let (_, t) = tracker(20);
    fill(&t, 7, 15);
    t.refund(t.charge(7).unwrap());
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!(c.issued, 16);
    assert_eq!(c.signals, [QuotaSignal::Pressure]);
    fill(&t, 7, 2);
    t.refund(t.charge(7).unwrap());
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!(c.issued, 19);
    assert_eq!(c.signals, [QuotaSignal::Limit]);
    // The 20th slot is still free after the refunds.
    assert!(t.commit(t.charge(7).unwrap()).signals.is_empty());
    assert!(t.charge(7).unwrap_err().first_in_window);
}

#[test]
fn a_refund_never_touches_issued_fired_signals_or_the_exhaustion_latch() {
    // Fired bits survive an out-of-order refund.
    let (_, t) = tracker(20);
    fill(&t, 7, 15);
    let failed = t.charge(7).unwrap();
    assert_eq!(
        t.commit(t.charge(7).unwrap()).signals,
        [QuotaSignal::Pressure]
    );
    t.refund(failed);
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!(c.issued, 17, "the refund did not decrement issued");
    assert!(c.signals.is_empty(), "the refund did not re-arm Pressure");

    // The latch survives a refund that frees capacity.
    let (_, t) = tracker(1);
    let failed = t.charge(7).unwrap();
    assert!(t.charge(7).unwrap_err().first_in_window);
    t.refund(failed);
    assert_eq!(
        t.commit(t.charge(7).unwrap()).signals,
        [QuotaSignal::Pressure, QuotaSignal::Limit]
    );
    assert!(!t.charge(7).unwrap_err().first_in_window);
}

#[test]
fn a_rolled_charge_neither_commits_nor_refunds_into_the_new_window() {
    let (clock, t) = tracker(2);
    let refunded = t.charge(7).unwrap();
    let committed = t.charge(7).unwrap();
    clock.set(WINDOW_NS);
    // A later charge advances the slot: both held charges are rolled.
    let current = t.charge(7).unwrap();
    t.refund(refunded);
    let c = t.commit(committed);
    assert_eq!((c.window, c.issued, c.signals.is_empty()), (0, 0, true));
    assert_eq!(t.commit(current).issued, 1);
    fill(&t, 7, 1);
    assert!(
        t.charge(7).unwrap_err().first_in_window,
        "the stale refund restored no capacity in the new window"
    );
}

#[test]
fn without_a_later_charge_a_late_finalization_belongs_to_its_admission_window() {
    let (clock, t) = tracker(2);
    let first = t.charge(7).unwrap();
    let second = t.charge(7).unwrap();
    clock.set(WINDOW_NS + 3);
    // The clock alone rolls nothing (PF-02).
    let c = t.commit(first);
    assert_eq!((c.window, c.issued, c.signals.is_empty()), (0, 1, true));
    t.refund(second);
    // The first charge after the crossing opens window 1 with full capacity.
    let c = t.commit(t.charge(7).unwrap());
    assert_eq!((c.window, c.issued), (1, 1));
}

#[test]
fn concurrent_charges_cannot_oversubscribe_or_duplicate_signals() {
    let (_, t) = tracker(1_000);
    let barrier = Barrier::new(12);
    let totals = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..12)
            .map(|_| {
                let (t, barrier) = (t.clone(), &barrier);
                scope.spawn(move || {
                    let mut counts = [0u64; 4];
                    barrier.wait();
                    for _ in 0..200 {
                        match t.charge(7) {
                            Ok(charge) => {
                                counts[0] += 1;
                                for signal in t.commit(charge).signals {
                                    match signal {
                                        QuotaSignal::Pressure => counts[1] += 1,
                                        QuotaSignal::Limit => counts[2] += 1,
                                        QuotaSignal::Exhausted => {
                                            panic!("commit emitted exhaustion")
                                        }
                                    }
                                }
                            }
                            Err(e) => counts[3] += u64::from(e.first_in_window),
                        }
                    }
                    counts
                })
            })
            .collect();
        workers.into_iter().fold([0u64; 4], |mut total, worker| {
            for (sum, count) in total.iter_mut().zip(worker.join().unwrap()) {
                *sum += count;
            }
            total
        })
    });
    assert_eq!(totals, [1_000, 1, 1, 1]);
}
