#![forbid(unsafe_code)]

//! CrashDetector — handles Spirit crashes (panic, signal, OOM, timeout).
//!
//! Story 5.3 — 7-step protocol per AC1.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use tokio::task::JoinHandle;

use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::supervision::{
    CrashCause, DispositionOutcome, HandleCrashError, HandleCrashReport, ReplicaResolver,
};

use crate::halt::{terminate_spirit, HaltRegistry};
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::SpiritControlBlock;
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};
use maos_domain::halt::TerminationKind;

/// The kernel-side crash detector.
#[maos_attrs::i9_exempt(
    reason = "supervision surface — holds only Arc references to existing kernel state (SCB map, TransparencyLog, HaltRegistry, CapabilityRegistry, IAC Bus, telemetry); no independently-mutable persistent state"
)]
pub struct CrashDetector {
    /// Same Arc the Scheduler holds — composition-root gate enforces single instance.
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    /// Same Arc Story 4.1's terminate_spirit consumes.
    tl: Arc<TransparencyLogAdapter>,
    /// Same Arc the Scheduler holds — per-PID drain after Story 5.3's drain_for_spirit refinement.
    halt_registry: Arc<HaltRegistry>,
    /// Capability revocation on crash per ADR-033.
    capability: Arc<crate::capability::CapabilityRegistryAdapter>,
    /// FR50 disposition routing.
    iac: Arc<crate::iac::IacBusAdapter>,
    /// Telemetry — iac_rt_duration_us with service=spirit_scheduler, outcome=crash_handled.
    telemetry: Arc<IacRtMetrics>,
    /// Lifecycle journal — crash events per I10.
    journal: Arc<JournalAdapter>,
    /// v0.3-β: NullReplicaResolver; v0.5+ multi-instance hosting swaps this.
    replica: Option<Arc<dyn ReplicaResolver>>,
    /// Active per-PID crash handlers — abort previous on re-crash (extremely rare; safety net).
    active_handlers: Arc<Mutex<BTreeMap<u32, JoinHandle<()>>>>,
}

impl CrashDetector {
    pub fn new(
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        tl: Arc<TransparencyLogAdapter>,
        halt_registry: Arc<HaltRegistry>,
        capability: Arc<crate::capability::CapabilityRegistryAdapter>,
        iac: Arc<crate::iac::IacBusAdapter>,
        telemetry: Arc<IacRtMetrics>,
        journal: Arc<JournalAdapter>,
    ) -> Self {
        Self {
            spirits,
            tl,
            halt_registry,
            capability,
            iac,
            telemetry,
            journal,
            replica: None,
            active_handlers: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Story 5.3 — inject a ReplicaResolver (v0.5+ multi-instance hosting).
    pub fn with_replica_resolver(mut self, replica: Arc<dyn ReplicaResolver>) -> Self {
        self.replica = Some(replica);
        self
    }

    /// Entry-point called from the scheduler's hook-dispatch path when a hook
    /// returns `HookOutcome::Panicked` (rust-inproc form) OR from the
    /// `SubprocessSupervisor` trait's `on_child_exit` callback when a
    /// subprocess Child returns a non-zero/signaled exit status (subprocess form).
    ///
    /// Latency budget: handler MUST complete within 2s (NFR-Rel-1).
    /// `task.orphaned` IAC frame MUST be emitted within 5s (FR12).
    pub async fn handle_crash(
        &self,
        spirit_pid: u32,
        cause: CrashCause,
    ) -> Result<HandleCrashReport, HandleCrashError> {
        let start = Instant::now();

        // Step 1: Acquire SCB
        let scb = {
            let map = self.spirits.read().expect("spirits lock poisoned");
            map.get(&spirit_pid)
                .cloned()
                .ok_or(HandleCrashError::NotLoaded(spirit_pid))?
        };

        // Active-handler dedup / abort
        {
            let mut handlers = self
                .active_handlers
                .lock()
                .expect("active_handlers lock poisoned");
            // Abort previous handler for this PID if any
            if let Some(prev) = handlers.remove(&spirit_pid) {
                prev.abort();
            }
            // Check-and-return-early if a handler is already active.
            if handlers.contains_key(&spirit_pid) {
                return Err(HandleCrashError::NotLoaded(spirit_pid)); // Crash already being handled
            }
            // Insert a placeholder handle (JoinHandle from a no-op spawn) to mark active
            handlers.insert(spirit_pid, tokio::spawn(std::future::ready(())));
        }

        // Step 2: Mark SCB state atomically to Unloaded (idempotent via CAS)
        let _ = scb.transition(
            crate::scheduler::control_block::ScbLifecycleState::Unloaded,
            maos_domain::lifecycle::LifecycleVerb::Unload,
        );

        // Step 3: Revoke all capability tokens for the PID
        let actual_tokens_revoked =
            self.capability.revoke_all_for_pid(spirit_pid).unwrap_or(0) as usize;

        // Step 4: Produce halt-receipts via terminate_spirit
        let receipts = terminate_spirit(
            &self.tl,
            &self.halt_registry,
            spirit_pid,
            &scb.spirit_id,
            TerminationKind::UnplannedCrash,
            scb.boot_nonce,
        );
        let halt_receipts_produced = receipts.len();

        // Step 5: Emit task.orphaned for each in-flight task
        let drained_tasks: Vec<_> = {
            let mut tasks = scb
                .task_assignments_in_flight
                .lock()
                .expect("task ledger lock poisoned");
            std::mem::take(&mut *tasks)
        };
        let tokens_revoked = actual_tokens_revoked;
        let task_orphaned_emitted_at_ns = crate::capability::cap_tokens::monotonic_now_ns();
        for task in &drained_tasks {
            let payload = serde_json::json!({
                "task_id": task.task_id,
                "originator_spirit_id": task.originator_spirit_id,
                "exit_signal": cause.exit_signal(),
                "exit_code": cause.exit_code(),
                "stderr_tail": cause.stderr_tail(),
                "cause": cause.as_str(),
                "in_flight_tokens": [task.capability_token],
            });
            let mut padded_token = [0u8; 32];
            padded_token[..16].copy_from_slice(&task.capability_token.0);
            self.tl.insert_frame_event(
                FrameKind::TaskComplete,
                spirit_pid,
                Some(&padded_token),
                "task.orphaned",
                &serde_json::to_vec(&payload).unwrap_or_default(),
                FrameOrigin::Kernel,
            );
        }

        // Step 6: Apply FR50 disposition
        let runtime = scb.runtime_snapshot();
        let disposition_outcome = crate::supervision::disposition::enforce_disposition(
            runtime.on_crash_action.clone(),
            &drained_tasks,
            &self.iac,
            self.replica.as_deref(),
        );

        // Step 7: Journal LifecycleEvent::Crash, THEN remove SCB from the map
        let _ = self
            .journal
            .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
                lifecycle_event: LifecycleEvent::Crash,
                spirit_id: scb.spirit_id.clone(),
                payload: None,
                effective_sandbox_tier: Some(runtime.sandbox_tier),
            }));

        {
            let mut map = self.spirits.write().expect("spirits lock poisoned");
            map.remove(&spirit_pid);
        }
        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
        let crash_journal_payload = serde_json::json!({
            "lifecycle_event": "Crash",
            "spirit_id": scb.spirit_id,
            "spirit_pid": spirit_pid,
            "cause": cause.as_str(),
        });
        self.tl.insert_frame_event(
            FrameKind::CapabilityInvocation,
            spirit_pid,
            None,
            "lifecycle.crash",
            crash_journal_payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );

        let detection_latency_ns = start.elapsed().as_nanos() as u64;
        self.telemetry.record_iac_rt(
            Service::SpiritScheduler,
            Outcome::CrashHandled,
            detection_latency_ns / 1000,
        );

        // Clean up active_handlers
        {
            let mut handlers = self
                .active_handlers
                .lock()
                .expect("active_handlers lock poisoned");
            handlers.remove(&spirit_pid);
        }

        Ok(HandleCrashReport {
            spirit_pid,
            spirit_id: scb.spirit_id.clone(),
            cause,
            detection_latency_ns,
            task_orphaned_emitted_at_ns,
            halt_receipts_produced,
            tokens_revoked,
            disposition_outcome,
        })
    }
}
