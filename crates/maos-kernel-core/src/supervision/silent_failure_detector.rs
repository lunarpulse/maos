#![forbid(unsafe_code)]

//! SilentFailureDetector — detects Spirits that heartbeat but make no progress.
//!
//! Story 5.3 — AC3.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::watchdog_common::pick_poll_cadence;
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::scheduler::control_block::ScbLifecycleState;
use crate::scheduler::control_block::SpiritControlBlock;
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};

#[maos_attrs::i9_exempt(reason = "supervision surface — holds only Arc references to existing kernel state (SCB map, TransparencyLog, telemetry); no independently-mutable persistent state")]
pub struct SilentFailureDetector {
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    tl: Arc<TransparencyLogAdapter>,
    telemetry: Arc<IacRtMetrics>,
        notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
}

impl SilentFailureDetector {
    pub fn new(
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        tl: Arc<TransparencyLogAdapter>,
        telemetry: Arc<IacRtMetrics>,
    notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
    ) -> Self {
        Self { spirits, tl, telemetry, notification_dispatcher }
    }

    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            let cadence = pick_poll_cadence();
            let mut interval = tokio::time::interval(cadence);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = interval.tick() => self.check_all_spirits().await,
                }
            }
        })
    }

    async fn check_all_spirits(&self) {
        let snapshot: Vec<Arc<SpiritControlBlock>> = {
            let map = self.spirits.read().expect("spirits lock poisoned");
            map.values().cloned().collect()
        };

        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();

        for scb in snapshot {
            if scb.current_state() != ScbLifecycleState::Running {
                continue;
            }

            let in_flight_count = {
                let tasks = scb.task_assignments_in_flight.lock().expect("task ledger lock poisoned");
                tasks.len()
            };
            if in_flight_count == 0 {
                continue;
            }

            let silent_failure_threshold_ms = scb
                .manifest
                .supervision
                .as_ref()
                .map(|s| s.silent_failure_threshold_ms)
                .unwrap_or(30000);
            let last_heartbeat_ns = scb.last_heartbeat_ns.load(Ordering::Relaxed);
            let last_progress_iac_ns = scb.last_progress_iac_ns.load(Ordering::Relaxed);
            let last_silent_failure_emit_ns = scb.last_silent_failure_emit_ns.load(Ordering::Relaxed);

            // Silent-failure condition: heartbeat is newer than progress, AND
            // the gap between heartbeat and progress exceeds threshold.
            let heartbeat_progress_gap = last_heartbeat_ns.saturating_sub(last_progress_iac_ns);
            let gap_exceeds_threshold = heartbeat_progress_gap
                > (silent_failure_threshold_ms as u64 * 1_000_000);
            // Derive refire cooldown from 2x the progress threshold
            let refire_cooldown_ns = (silent_failure_threshold_ms as u64 * 2) * 1_000_000;
            let can_refire = (now_ns.saturating_sub(last_silent_failure_emit_ns)) > refire_cooldown_ns;

            if gap_exceeds_threshold && can_refire && last_heartbeat_ns > 0 {
                let cas = scb.last_silent_failure_emit_ns.compare_exchange(
                    last_silent_failure_emit_ns,
                    now_ns,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                );
                if cas.is_ok() {
                    let payload = serde_json::json!({
                        "spirit_pid": scb.pid,
                        "spirit_id": scb.spirit_id,
                        "last_heartbeat_age_ms": now_ns.saturating_sub(last_heartbeat_ns) / 1_000_000,
                        "last_progress_iac_age_ms": now_ns.saturating_sub(last_progress_iac_ns) / 1_000_000,
                        "heartbeat_progress_gap_ms": heartbeat_progress_gap / 1_000_000,
                        "in_flight_task_count": in_flight_count,
                    });

                    self.tl.insert_frame_event(
                        FrameKind::SilentFailureSuspect,
                        scb.pid,
                        None,
                        "silent_failure.suspect",
                        &serde_json::to_vec(&payload).unwrap_or_default(),
                        maos_domain::invariants::i3::FrameOrigin::Kernel,
                    );

                    self.telemetry.record_iac_rt(
                        Service::SpiritScheduler,
                        Outcome::Ok,
                        0,
                    );
                }
            }
        }
    }
}
