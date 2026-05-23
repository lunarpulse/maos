#![forbid(unsafe_code)]

//! ProgressWatchdog — detects hung Spirits (no progress IAC for >threshold).
//!
//! Story 5.3 — AC2.

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

#[maos_attrs::i9_exempt(
    reason = "supervision surface — holds only Arc references to existing kernel state (SCB map, TransparencyLog, telemetry); no independently-mutable persistent state"
)]
pub struct ProgressWatchdog {
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    tl: Arc<TransparencyLogAdapter>,
    telemetry: Arc<IacRtMetrics>,
    notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
}

impl ProgressWatchdog {
    pub fn new(
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        tl: Arc<TransparencyLogAdapter>,
        telemetry: Arc<IacRtMetrics>,
        notification_dispatcher: Arc<maos_director_surface::notification::NotificationDispatcher>,
    ) -> Self {
        Self {
            spirits,
            tl,
            telemetry,
            notification_dispatcher,
        }
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
                let tasks = scb
                    .task_assignments_in_flight
                    .lock()
                    .expect("task ledger lock poisoned");
                tasks.len()
            };
            if in_flight_count == 0 {
                continue;
            }

            let progress_threshold_ms = scb
                .manifest
                .supervision
                .as_ref()
                .map(|s| s.progress_threshold_ms)
                .unwrap_or(30000);
            let last_progress_iac_ns = scb.last_progress_iac_ns.load(Ordering::Relaxed);
            let last_stall_emit_ns = scb.last_stall_emit_ns.load(Ordering::Relaxed);

            let stalled = (now_ns.saturating_sub(last_progress_iac_ns))
                > (progress_threshold_ms as u64 * 1_000_000);
            // Derive refire cooldown from 2x the progress threshold
            let refire_cooldown_ns = (progress_threshold_ms as u64 * 2) * 1_000_000;
            let can_refire = (now_ns.saturating_sub(last_stall_emit_ns)) > refire_cooldown_ns;

            if stalled && can_refire {
                // CAS to avoid multi-fire
                let cas = scb.last_stall_emit_ns.compare_exchange(
                    last_stall_emit_ns,
                    now_ns,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                );
                if cas.is_ok() {
                    let first_task = {
                        let tasks = scb
                            .task_assignments_in_flight
                            .lock()
                            .expect("task ledger lock poisoned");
                        tasks.first().cloned()
                    };

                    let payload = serde_json::json!({
                        "spirit_pid": scb.pid,
                        "spirit_id": scb.spirit_id,
                        "in_flight_task_count": in_flight_count,
                        "no_progress_duration_ms": now_ns.saturating_sub(last_progress_iac_ns) / 1_000_000,
                        "first_in_flight_task_id": first_task.as_ref().map(|t| t.task_id.clone()).unwrap_or_default(),
                        "originator_spirit_id": first_task.as_ref().map(|t| t.originator_spirit_id.clone()).unwrap_or_default(),
                    });

                    self.tl.insert_frame_event(
                        FrameKind::TaskStalled,
                        scb.pid,
                        None,
                        "task.stalled",
                        &serde_json::to_vec(&payload).unwrap_or_default(),
                        maos_domain::invariants::i3::FrameOrigin::Kernel,
                    );

                    self.telemetry
                        .record_iac_rt(Service::SpiritScheduler, Outcome::Ok, 0);
                }
            }
        }
    }
}
