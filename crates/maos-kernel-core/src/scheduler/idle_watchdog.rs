#![forbid(unsafe_code)]

//! Per-Spirit idle watchdog — tracks mailbox quiescence and fires
//! `on_idle` when the wall-clock idle exceeds `idle_window_ms`.
//!
//! Architecture §4.1 + Story 5.1 Task 6.

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use parking_lot::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use super::FiringReservation;
use crate::scheduler::control_block::{ScbLifecycleState, SpiritControlBlock};
use crate::scheduler::hook_dispatch::HookDispatcher;
use crate::scheduler::kernel_ctx::KernelCtx;

/// Story 21-9 AC8 — most `on_idle` firings in flight at once; one per Spirit
/// already, each capped at 30 s.
const MAX_IN_FLIGHT_FIRINGS: usize = 256;

/// Per-Spirit idle watchdog — fires `on_idle` when the Spirit is
/// Running AND (now - last_inbound_frame_ns) > idle_window_ms.
#[maos_attrs::i9_exempt(
    reason = "per-Spirit idle watchdog holding Arc handles to the supervised SCB set + hook dispatcher (already-exempt); supervised transient state per I9, keyed by spirit_pid (Story 7.1.7 baseline-reset)"
)]
pub struct IdleWatchdog {
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    dispatcher: Arc<HookDispatcher>,
    /// Story 21-9 AC8 — Spirits with an `on_idle` reserved or in flight.
    in_flight: Arc<Mutex<HashSet<u32>>>,
}

impl IdleWatchdog {
    pub fn new(
        scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        dispatcher: Arc<HookDispatcher>,
    ) -> Self {
        Self {
            scbs,
            dispatcher,
            in_flight: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Spawn the watchdog background task.
    ///
    /// Polls the SCB map every ~idle_window_ms/10, firing `on_idle`
    /// for Spirits whose mailbox has been quiescent past the threshold.
    /// Uses `last_idle_fire_ns` to avoid multi-fire (prevents
    /// thundering-herd `on_idle` calls when genuinely quiescent).
    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            // MAOS_IDLE_FAST=1 collapses poll interval to 40ms and divides
            // the idle threshold by 100 (test-only convenience).
            let fast_mode = std::env::var_os("MAOS_IDLE_FAST").is_some();
            let poll_ms = if fast_mode { 40u64 } else { 3000u64 };
            let mut interval = tokio::time::interval(Duration::from_millis(poll_ms));
            // Owned in-flight `on_idle` firings, each bounded by the
            // dispatcher's `min(cap, 30 s)` wall.
            let mut fires = JoinSet::new();

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {
                        while let Some(joined) = fires.try_join_next() {
                            Self::note_join(joined);
                        }
                        self.dispatch_due(fast_mode, &mut fires).await;
                    }
                }
            }
            // Shutdown joins every `on_idle` still in flight.
            while let Some(joined) = fires.join_next().await {
                Self::note_join(joined);
            }
        })
    }

    fn note_join(joined: Result<(), tokio::task::JoinError>) {
        if let Err(e) = joined {
            eprintln!("idle-watchdog: in-flight on_idle task ended abnormally: {e}");
        }
    }

    /// Admit every due Spirit and hand its `on_idle` to `fires` without
    /// awaiting it, so a slow hook never holds back a later poll. A Spirit
    /// whose `on_idle` is still in flight is never dispatched twice, and the
    /// total in flight is bounded by `MAX_IN_FLIGHT_FIRINGS`.
    async fn dispatch_due(self: &Arc<Self>, fast_mode: bool, fires: &mut JoinSet<()>) {
        // Collect SCBs to process (clone Arcs outside the lock to avoid
        // holding RwLockReadGuard across await points).
        let candidates: Vec<(Arc<SpiritControlBlock>, FiringReservation<u32>)> = {
            let scbs = self.scbs.read().unwrap();
            let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
            let mut out = Vec::new();
            for (_, scb) in scbs.iter() {
                if scb.current_state() != ScbLifecycleState::Running {
                    continue;
                }
                let last_inbound = scb.last_inbound_frame_ns.load(Ordering::Relaxed);
                let idle_window_ms =
                    scb.runtime_snapshot().manifest.scheduling.idle_window_ms as u64;
                let idle_window_ms = if fast_mode {
                    idle_window_ms / 100
                } else {
                    idle_window_ms
                };
                let idle_window_ns = idle_window_ms * 1_000_000;
                // Story 5.1 review backfill — removed `last_inbound == 0`
                // short-circuit. SCB constructor now seeds last_inbound to
                // creation time so this check is purely temporal. A
                // never-frame-received Spirit fires on_idle one
                // idle_window after load, matching AC4 scenario 1.
                if now_ns.saturating_sub(last_inbound) < idle_window_ns {
                    continue;
                }
                let last_fire = scb.last_idle_fire_ns.load(Ordering::Relaxed);
                if last_fire >= last_inbound && last_fire > 0 {
                    continue;
                }
                let Some(reservation) =
                    FiringReservation::try_reserve(&self.in_flight, scb.pid, MAX_IN_FLIGHT_FIRINGS)
                else {
                    continue;
                };
                out.push((Arc::clone(scb), reservation));
            }
            out
        };

        for (scb, reservation) in candidates {
            let dispatcher = Arc::clone(&self.dispatcher);
            fires.spawn(async move {
                // One handle moves into the native closure and drops only
                // when `on_idle` actually returns (a timed-out call is
                // detached, not preempted); this one lives until the stamp
                // below, so the Spirit is never free before its stamp lands.
                let reservation = Arc::new(reservation);
                // Story 5.1 review backfill — fire FIRST then store
                // last_idle_fire_ns only when the outcome reflects an actual
                // dispatch (Fired / BudgetWarning80 / BudgetExceeded).  Storing
                // unconditionally hid SkippedManifest cases (the previous test
                // `idle_watchdog_skips_manifest_disabled_hook` passed for the
                // wrong reason — see test comment for context).
                let outcome = dispatcher
                    .fire_on_idle_guarded(&scb, Arc::clone(&reservation))
                    .await;
                use crate::scheduler::hook_dispatch::HookOutcome;
                let actually_fired = !matches!(
                    outcome,
                    HookOutcome::SkippedManifest | HookOutcome::SkippedQuarantined
                );
                if actually_fired {
                    let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
                    scb.last_idle_fire_ns.store(now_ns, Ordering::Release);
                }
                drop(reservation);
            });
        }
    }
}

/// Compute the poll interval from idle_window_ms.
/// Bounded: floor 100ms, ceiling 5000ms.
pub fn pick_poll_interval(idle_window_ms: u64) -> Duration {
    let raw_ms = (idle_window_ms / 10).max(100).min(5000);
    Duration::from_millis(raw_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_bounds() {
        assert_eq!(pick_poll_interval(30000), Duration::from_millis(3000));
        assert_eq!(pick_poll_interval(100), Duration::from_millis(100)); // floor
        assert_eq!(pick_poll_interval(100000), Duration::from_millis(5000)); // ceiling
        assert_eq!(pick_poll_interval(10), Duration::from_millis(100)); // below floor → floor
        assert_eq!(pick_poll_interval(3600000), Duration::from_millis(5000)); // 1h → ceiling
    }
}
