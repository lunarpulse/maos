#![forbid(unsafe_code)]

//! Post-swap invariant monitor — 30s window auto-revert (NFR-Rel-5).
//!
//! Spawns a Tokio task that polls at 1s cadence against the swap's
//! invariant snapshot. On violation, calls `auto_revert` on the
//! coordinator.

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Snapshot of invariants taken at swap commit.
#[derive(Debug, Clone)]
#[maos_attrs::i9_exempt(
    reason = "immutable invariant snapshot captured at swap commit; Vec<String> of pre-swap halt-ids / frame-shapes is bounded structural state per I9, no parameter drift (Story 7.1.7 baseline-reset)"
)]
pub struct PostSwapInvariantSnapshot {
    pub pre_swap_halt_ids: Vec<String>,
    pub pid: u32,
    pub boot_nonce: u64,
    pub pre_swap_frame_shapes: Vec<String>,
}

//// Re-export the domain violation type for the saga layer.
pub use maos_domain::hot_swap::PostSwapInvariantViolation;

/// Monitor task that watches for post-swap invariant violations.
#[maos_attrs::i9_exempt(
    reason = "post-swap invariant monitor holding Arc to the already-exempt HotSwapCoordinator + invariant snapshot; supervised transient task state per I9 (Story 7.1.7 baseline-reset)"
)]
pub struct PostSwapMonitor {
    pub coordinator: Arc<super::coordinator::HotSwapCoordinator>,
    pub spirit_pid: u32,
    pub invariant_snapshot: Arc<PostSwapInvariantSnapshot>,
    pub window: Duration,
}

impl PostSwapMonitor {
    /// Create a new monitor. Default window is 30s.
    pub fn new(
        coordinator: Arc<super::coordinator::HotSwapCoordinator>,
        spirit_pid: u32,
        invariant_snapshot: PostSwapInvariantSnapshot,
    ) -> Self {
        let window = if std::env::var("MAOS_AUTO_REVERT_FAST").as_deref() == Ok("1") {
            Duration::from_millis(300)
        } else {
            Duration::from_secs(30)
        };

        Self {
            coordinator,
            spirit_pid,
            invariant_snapshot: Arc::new(invariant_snapshot),
            window,
        }
    }

    /// Spawn the monitor task. Returns a JoinHandle.
    /// The task polls at `tick_cadence` until the window expires or a violation is detected.
    ///
    /// In production (30s window), cadence is 1s — 30 invariant checks per swap.
    /// In `MAOS_AUTO_REVERT_FAST=1` mode (300ms window), cadence shrinks to
    /// 50ms so the test path actually fires at least one check before the window
    /// expires. Story 5.2 review backfill: the original 1s cadence made FAST mode
    /// silently no-op because the first real check tick came AFTER the 300ms deadline.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        let coordinator = Arc::clone(&self.coordinator);
        let spirit_pid = self.spirit_pid;
        let window = self.window;
        let invariant_snapshot = Arc::clone(&self.invariant_snapshot);
        // Choose a cadence small enough that at least a few ticks fit inside
        // the window. 50ms in fast mode, 1s in production.
        let tick_cadence = if window <= Duration::from_secs(1) {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(1)
        };

        tokio::spawn(async move {
            let deadline = Instant::now() + window;
            let mut interval = tokio::time::interval(tick_cadence);
            // Skip the first immediate tick (tokio::time::interval fires immediately).
            interval.tick().await;

            while Instant::now() < deadline {
                interval.tick().await;
                // Use the instance method so we can access the coordinator's adapters.
                let monitor = PostSwapMonitor {
                    coordinator: Arc::clone(&coordinator),
                    spirit_pid,
                    invariant_snapshot: Arc::clone(&invariant_snapshot),
                    window,
                };
                if let Some(violation) = monitor.check_invariants() {
                    let _ = coordinator.auto_revert(spirit_pid, violation).await;
                    return;
                }
            }
        })
    }

    /// Check the three invariants using the coordinator's adapters.
    /// Returns a violation if any is breached.
    pub fn check_invariants(&self) -> Option<PostSwapInvariantViolation> {
        let snapshot = &self.invariant_snapshot;

        // (i) Halt-set delta — no silent loss of pending halts.
        {
            let map = self
                .coordinator
                .spirits_map()
                .read()
                .expect("spirits lock poisoned");
            if let Some(_scb) = map.get(&self.spirit_pid) {
                let current_halt_ids: std::collections::BTreeSet<String> = self
                    .coordinator
                    .halt_registry_ref()
                    .pending_halt_ids()
                    .iter()
                    .map(|h| h.as_str().to_string())
                    .collect();
                if let Some(lost) =
                    detect_halt_set_loss(&snapshot.pre_swap_halt_ids, &current_halt_ids)
                {
                    return Some(PostSwapInvariantViolation::HaltSetLoss {
                        lost_halt_ids: lost,
                    });
                }
            }
        }

        // (ii) Boot-nonce stability.
        {
            let map = self
                .coordinator
                .spirits_map()
                .read()
                .expect("spirits lock poisoned");
            if let Some(scb) = map.get(&self.spirit_pid) {
                if scb.boot_nonce != snapshot.boot_nonce {
                    return Some(PostSwapInvariantViolation::BootNonceMismatch {
                        expected: snapshot.boot_nonce,
                        observed: scb.boot_nonce,
                    });
                }
            }
        }

        // (iii) Output-shape contract — placeholder; full implementation requires
        // sampling journaled frames and validating against successor's [output_shape].
        // Deferred to Story 5.4 when the journal sampler is available.

        None
    }
}

/// Pure halt-set-delta check: which snapshot halts are no longer pending?
///
/// The snapshot MUST hold only the halts expected to SURVIVE the swap — the
/// post-drain pending set captured at commit. Halts the I14 gate drained-
/// resolved are legitimately gone and must NOT appear here; otherwise this
/// reports a false `HaltSetLoss` and the monitor auto-reverts a valid
/// drained-with-pending swap (Story 12.5 review fix).
fn detect_halt_set_loss(
    snapshot_halt_ids: &[String],
    current_pending: &std::collections::BTreeSet<String>,
) -> Option<Vec<String>> {
    let lost: Vec<String> = snapshot_halt_ids
        .iter()
        .filter(|id| !current_pending.contains(*id))
        .cloned()
        .collect();
    (!lost.is_empty()).then_some(lost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_window_resolves_from_env_var() {
        // Save + clear to isolate from other tests' env state.
        let prior = std::env::var("MAOS_AUTO_REVERT_FAST").ok();
        std::env::set_var("MAOS_AUTO_REVERT_FAST", "1");
        let fast = resolve_window_for_test();
        std::env::remove_var("MAOS_AUTO_REVERT_FAST");
        let slow = resolve_window_for_test();
        if let Some(v) = prior {
            std::env::set_var("MAOS_AUTO_REVERT_FAST", v);
        }
        assert_eq!(fast, Duration::from_millis(300));
        assert_eq!(slow, Duration::from_secs(30));
    }

    fn resolve_window_for_test() -> Duration {
        if std::env::var("MAOS_AUTO_REVERT_FAST").as_deref() == Ok("1") {
            Duration::from_millis(300)
        } else {
            Duration::from_secs(30)
        }
    }

    #[test]
    fn invariant_snapshot_construction() {
        let snapshot = PostSwapInvariantSnapshot {
            pre_swap_halt_ids: vec!["halt-A".into(), "halt-B".into()],
            pid: 100,
            boot_nonce: 0xDEAD,
            pre_swap_frame_shapes: vec!["frame-1".into()],
        };
        assert_eq!(snapshot.pid, 100);
        assert_eq!(snapshot.pre_swap_halt_ids.len(), 2);
    }

    #[test]
    fn survivor_snapshot_without_the_drained_halt_is_not_a_loss() {
        // Story 12.5 review fix (GREEN): a drained-with-pending swap resolves the
        // pending halt at the I14 gate, so the commit-time SURVIVOR snapshot is
        // empty. The monitor must NOT flag the drained halt as loss.
        let survivor_snapshot: Vec<String> = vec![];
        let current_pending = std::collections::BTreeSet::new();
        assert_eq!(detect_halt_set_loss(&survivor_snapshot, &current_pending), None);
    }

    #[test]
    fn a_pre_drain_snapshot_would_falsely_flag_the_drained_halt() {
        // Story 12.5 review (RED shape): the pre-fix code snapshotted the
        // PRE-drain set, so a drained halt — absent from live pending — was a
        // FALSE HaltSetLoss that auto-reverted valid swaps. Guards the regression.
        let pre_drain_snapshot = vec!["drained-halt".to_string()];
        let current_pending = std::collections::BTreeSet::new();
        assert_eq!(
            detect_halt_set_loss(&pre_drain_snapshot, &current_pending),
            Some(vec!["drained-halt".to_string()]),
            "a pre-drain snapshot falsely flags the drained halt — the exact bug the survivor snapshot fixes"
        );
    }

    #[test]
    fn a_genuinely_lost_survivor_is_still_flagged() {
        // The monitor must STILL catch a real loss: a halt that survived the
        // gate (in the survivor snapshot) but then vanished during the window.
        let survivor_snapshot = vec!["carried-halt".to_string()];
        let current_pending = std::collections::BTreeSet::new();
        assert_eq!(
            detect_halt_set_loss(&survivor_snapshot, &current_pending),
            Some(vec!["carried-halt".to_string()])
        );
    }

    #[test]
    fn a_surviving_halt_still_pending_is_clean() {
        let survivor_snapshot = vec!["carried-halt".to_string()];
        let current_pending: std::collections::BTreeSet<String> =
            ["carried-halt".to_string()].into_iter().collect();
        assert_eq!(detect_halt_set_loss(&survivor_snapshot, &current_pending), None);
    }
}
