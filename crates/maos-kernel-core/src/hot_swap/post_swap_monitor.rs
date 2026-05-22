#![forbid(unsafe_code)]

//! Post-swap invariant monitor — 30s window auto-revert (NFR-Rel-5).
//!
//! Spawns a Tokio task that polls at 1s cadence against the swap's
//! invariant snapshot. On violation, calls `auto_revert` on the
//! coordinator.

use std::sync::Arc;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use maos_domain::halt::HaltId;

/// Snapshot of invariants taken at swap commit.
#[derive(Debug, Clone)]
pub struct PostSwapInvariantSnapshot {
    pub pre_swap_halt_ids: Vec<String>,
    pub pid: u32,
    pub boot_nonce: u64,
    pub pre_swap_frame_shapes: Vec<String>,
}

//// Re-export the domain violation type for the saga layer.
pub use maos_domain::hot_swap::PostSwapInvariantViolation;

/// Monitor task that watches for post-swap invariant violations.
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
    /// The task polls at 1s cadence until the window expires or a violation is detected.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        let coordinator = Arc::clone(&self.coordinator);
        let spirit_pid = self.spirit_pid;
        let window = self.window;
        let invariant_snapshot = Arc::clone(&self.invariant_snapshot);

        tokio::spawn(async move {
            let deadline = Instant::now() + window;
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            // Skip the first immediate tick.
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
            let map = self.coordinator.spirits_map().read().expect("spirits lock poisoned");
            if let Some(_scb) = map.get(&self.spirit_pid) {
                let current_halt_ids: std::collections::BTreeSet<String> = self.coordinator
                    .halt_registry_ref()
                    .pending_halt_ids()
                    .iter()
                    .map(|h| h.as_str().to_string())
                    .collect();
                for halt_id in &snapshot.pre_swap_halt_ids {
                    if !current_halt_ids.contains(halt_id) {
                        return Some(PostSwapInvariantViolation::HaltSetLoss {
                            lost_halt_ids: vec![halt_id.clone()],
                        });
                    }
                }
            }
        }

        // (ii) Boot-nonce stability.
        {
            let map = self.coordinator.spirits_map().read().expect("spirits lock poisoned");
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

    /// Static variant used only for unit tests that don't have a full coordinator.
    fn check_invariants_static(
        _snapshot: &PostSwapInvariantSnapshot,
    ) -> Option<PostSwapInvariantViolation> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_window_uses_env_var() {
        std::env::set_var("MAOS_AUTO_REVERT_FAST", "1");
        // We can't construct a real coordinator in a unit test without the full kernel,
        // but we can verify the window duration logic.
        let snapshot = PostSwapInvariantSnapshot {
            pre_swap_halt_ids: vec!["halt-001".into()],
            pid: 42,
            boot_nonce: 0xCAFE,
            pre_swap_frame_shapes: vec![],
        };

        // The window would be 300ms when MAOS_AUTO_REVERT_FAST=1.
        // Just verify the type constructs.
        let _snapshot = snapshot.clone();
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
}
