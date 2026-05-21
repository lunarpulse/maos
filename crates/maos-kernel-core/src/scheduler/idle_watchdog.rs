#![forbid(unsafe_code)]

//! Per-Spirit idle watchdog — tracks mailbox quiescence and fires
//! `on_idle` when the wall-clock idle exceeds `idle_window_ms`.
//!
//! Architecture §4.1 + Story 5.1 Task 6.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::scheduler::control_block::{ScbLifecycleState, SpiritControlBlock};
use crate::scheduler::hook_dispatch::HookDispatcher;
use crate::scheduler::kernel_ctx::KernelCtx;

/// Per-Spirit idle watchdog — fires `on_idle` when the Spirit is
/// Running AND (now - last_inbound_frame_ns) > idle_window_ms.
pub struct IdleWatchdog {
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    dispatcher: Arc<HookDispatcher>,
}

impl IdleWatchdog {
    pub fn new(
        scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        dispatcher: Arc<HookDispatcher>,
    ) -> Self {
        Self { scbs, dispatcher }
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

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {
                        self.check_and_fire(fast_mode).await;
                    }
                }
            }
        })
    }

    async fn check_and_fire(&self, fast_mode: bool) {
        // Collect SCBs to process (clone Arcs outside the lock to avoid
        // holding RwLockReadGuard across await points).
        let candidates: Vec<Arc<SpiritControlBlock>> = {
            let scbs = self.scbs.read().unwrap();
            let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
            scbs
                .iter()
                .filter(|(_, scb)| {
                    if scb.current_state() != ScbLifecycleState::Running {
                        return false;
                    }
                    let last_inbound = scb.last_inbound_frame_ns.load(Ordering::Relaxed);
                    let idle_window_ms = if fast_mode {
                        scb.manifest.scheduling.idle_window_ms as u64 / 100
                    } else {
                        scb.manifest.scheduling.idle_window_ms as u64
                    };
                    let idle_window_ns = idle_window_ms * 1_000_000;
                    if last_inbound == 0 || now_ns.saturating_sub(last_inbound) < idle_window_ns {
                        return false;
                    }
                    let last_fire = scb.last_idle_fire_ns.load(Ordering::Relaxed);
                    if last_fire >= last_inbound && last_fire > 0 {
                        return false;
                    }
                    true
                })
                .map(|(_, scb)| Arc::clone(scb))
                .collect()
        };

        for scb in &candidates {
            let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
            scb.last_idle_fire_ns
                .store(now_ns, Ordering::Release);
            let _outcome = self.dispatcher.fire_on_idle(scb).await;
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
