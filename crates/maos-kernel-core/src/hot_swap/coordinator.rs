#![forbid(unsafe_code)]

//! HotSwapCoordinator — kernel-side entry point for Spirit hot-swap.
//!
//! Implements the 12-step hot-swap protocol per ADR-017 binding-v0.3:
//! 1. resolve spirit_id → predecessor_pid
//! 2. snapshot predecessor SCB for saga rollback
//! 3. I14 gate — validate_swap_halt_continuity
//! 4. fire on_swap_out hook
//! 5. call snapshot() → CBOR state blob
//! 6. decode + validate envelope (same-major vs cross-major)
//! 7. cross-major? → run_migrator
//! 8. atomic SCB swap under spirits write-lock
//! 9. fire on_swap_in(payload) hook
//! 10. journal LifecycleEvent::Swap
//! 11. spawn PostSwapMonitor (30s window)
//! 12. return HotSwapResult::Completed

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use parking_lot::Mutex;
use std::time::Instant;

use maos_domain::hot_swap::{HotSwapError, HotSwapResult, PostSwapInvariantViolation};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i3::FrameOrigin;

use crate::capability::CapabilityRegistryAdapter;
use crate::halt::{validate_swap_halt_continuity, HaltRegistry, SwapVerdict};
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::iac::IacBusAdapter;
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::{AnySpiritObj, ScbRuntimeSnapshot, SpiritControlBlock};
use crate::scheduler::hook_dispatch::HookDispatcher;
use crate::scheduler::HookOutcome;
use crate::telemetry::iac_rt::IacRtMetrics;

use super::migrator::run_migrator;
use super::post_swap_monitor::{PostSwapInvariantSnapshot, PostSwapMonitor};
use super::precheck::{HotSwapPrecheck, PrecheckVerdict as KernelPrecheckVerdict};
use super::saga::{HotSwapSaga, SagaCompensation, SagaPhase};
use super::state_codec::{self, SchemaCompat};

/// The kernel-side hot-swap coordinator.
///
/// Constructed exactly once at the composition root. Holds Arc handles
/// to all shared adapters — no second instance per §A5 gate.
#[maos_attrs::i9_exempt(
    reason = "composition-root singleton holding Arc handles to already-exempt kernel adapters for hot-swap coordination; supervised transient state per I9, recreated on kernel restart, no cross-key aggregation (Story 7.1.7 baseline-reset)"
)]
pub struct HotSwapCoordinator {
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    journal: Arc<JournalAdapter>,
    tl: Arc<TransparencyLogAdapter>,
    halt_registry: Arc<HaltRegistry>,
    capability: Arc<CapabilityRegistryAdapter>,
    iac: Arc<IacBusAdapter>,
    dispatcher: Arc<HookDispatcher>,
    telemetry: Arc<IacRtMetrics>,
    archive_dir: PathBuf,
    active_monitors: Arc<Mutex<std::collections::BTreeMap<u32, tokio::task::JoinHandle<()>>>>,
    /// Pending runtime snapshots per PID — stored at commit for auto-revert
    /// without replacing the stable control-block Arc.
    pending_reverts: Arc<Mutex<std::collections::BTreeMap<u32, ScbRuntimeSnapshot>>>,
}

impl HotSwapCoordinator {
    /// Construct the coordinator. Called once from the composition root.
    pub fn new(
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        journal: Arc<JournalAdapter>,
        tl: Arc<TransparencyLogAdapter>,
        halt_registry: Arc<HaltRegistry>,
        capability: Arc<CapabilityRegistryAdapter>,
        iac: Arc<IacBusAdapter>,
        dispatcher: Arc<HookDispatcher>,
        telemetry: Arc<IacRtMetrics>,
        archive_dir: PathBuf,
    ) -> Self {
        // Ensure archive directory exists with mode 0700.
        if let Err(e) = std::fs::create_dir_all(&archive_dir) {
            eprintln!(
                "[hot_swap] WARNING: cannot create archive dir {}: {e}",
                archive_dir.display()
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&archive_dir) {
                let mut perms = meta.permissions();
                if perms.mode() & 0o077 != 0 {
                    perms.set_mode(0o700);
                    let _ = std::fs::set_permissions(&archive_dir, perms);
                }
            }
        }

        Self {
            spirits,
            journal,
            tl,
            halt_registry,
            capability,
            iac,
            dispatcher,
            telemetry,
            archive_dir,
            active_monitors: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
            pending_reverts: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
        }
    }

    /// Primary entry-point: initiate a hot-swap from predecessor to successor.
    ///
    /// Returns `Ok(HotSwapResult::Completed { ... })` on success,
    /// `Err(HotSwapError::*)` on failure (with saga compensation applied).
    pub async fn initiate_swap(
        &self,
        spirit_id: &str,
        successor_manifest: &crate::scheduler::control_block::SpiritManifestBundle,
        successor_spirit_obj: Arc<dyn AnySpiritObj>,
    ) -> Result<HotSwapResult, HotSwapError> {
        let start = Instant::now();

        // Step 1: Resolve spirit_id → predecessor_pid.
        let predecessor_pid = self.resolve_pid(spirit_id)?;
        let predecessor_scb = {
            let map = self.spirits.read().expect("spirits lock poisoned");
            Arc::clone(
                map.get(&predecessor_pid)
                    .ok_or_else(|| HotSwapError::NotLoaded {
                        spirit_id: spirit_id.to_string(),
                    })?,
            )
        };
        let pre_swap_runtime = predecessor_scb.runtime_snapshot();

        // Step 2: Snapshot predecessor SCB for saga rollback.
        let mut saga = HotSwapSaga::new().with_pre_swap_snapshot(Arc::clone(&predecessor_scb));
        saga.advance_to(SagaPhase::NotStarted);

        let predecessor_version = pre_swap_runtime
            .manifest
            .class
            .as_ref()
            .map(|c| c.version.clone())
            .unwrap_or_else(|| "unknown".into());
        let predecessor_halt_protocol_version = pre_swap_runtime
            .manifest
            .halt_protocol_compatibility
            .as_ref()
            .map(|h| h.version)
            .unwrap_or(1u32);

        // Step 3: I14 gate — validate_swap_halt_continuity.
        saga.advance_to(SagaPhase::HaltContinuityChecked);
        let successor_accepted_versions: Vec<u32> = successor_manifest
            .halt_protocol_compatibility
            .as_ref()
            .map(|h| vec![h.version])
            .unwrap_or_else(|| vec![1u32]);
        match validate_swap_halt_continuity(
            &self.halt_registry,
            predecessor_pid,
            predecessor_halt_protocol_version,
            Some(&successor_accepted_versions),
        ) {
            Ok(SwapVerdict::SafeDrained { drained_count }) => {
                let _ = drained_count;
            }
            Ok(SwapVerdict::SafeMigrated { migrated_count, .. }) => {
                let _ = migrated_count;
            }
            Err(e) => {
                saga.compensate(
                    SagaCompensation::RestorePredecessor {
                        reason: format!("halt-continuity violation: {e}"),
                    },
                    &self.tl,
                    &self.journal,
                )
                .await;
                return Err(HotSwapError::HaltContinuityViolation(e));
            }
        }

        // Step 4: Fire on_swap_out hook.
        saga.advance_to(SagaPhase::SwapOutFired);
        let swap_out_result = self.dispatcher.fire_on_swap_out(&predecessor_scb).await;
        match swap_out_result {
            HookOutcome::Fired { .. }
            | HookOutcome::BudgetWarning80 { .. }
            | HookOutcome::SkippedManifest
            | HookOutcome::DeferredToNextStory => {}
            HookOutcome::BudgetExceeded { .. } | HookOutcome::Panicked { .. } => {
                saga.compensate(
                    SagaCompensation::RestorePredecessor {
                        reason: format!("swap-out failed: {swap_out_result:?}"),
                    },
                    &self.tl,
                    &self.journal,
                )
                .await;
                return Err(HotSwapError::SwapOutFailed {
                    spirit_id: spirit_id.to_string(),
                    error: format!("{swap_out_result:?}"),
                });
            }
        }

        // Step 5: Call snapshot() → CBOR state blob.
        saga.advance_to(SagaPhase::SnapshotTaken);
        let state_blob = match self.dispatcher.fire_snapshot(&predecessor_scb).await {
            Ok(blob) => blob,
            Err(outcome) => {
                saga.compensate(
                    SagaCompensation::RestorePredecessor {
                        reason: format!("snapshot failed: {outcome:?}"),
                    },
                    &self.tl,
                    &self.journal,
                )
                .await;
                return Err(HotSwapError::SnapshotFailed {
                    spirit_id: spirit_id.to_string(),
                    error: format!("{outcome:?}"),
                });
            }
        };

        let predecessor_state_schema_version = pre_swap_runtime
            .manifest
            .hot_swap
            .as_ref()
            .map(|h| h.state_schema_version)
            .unwrap_or(1u32);
        let successor_state_schema_version = successor_manifest
            .hot_swap
            .as_ref()
            .map(|h| h.state_schema_version)
            .unwrap_or(1u32);

        // Step 6: kernel owns the logical envelope while the Spirit owns the
        // opaque CBOR payload.  Do not serialize then immediately deserialize
        // a value that never crossed a process/archive boundary.
        let envelope = state_codec::validate_logical_envelope(
            state_codec::StateEnvelope {
                schema_version: predecessor_state_schema_version,
                payload: state_blob,
                envelope_version: 1,
            },
            successor_state_schema_version,
        )
        .map_err(|error| match error {
            state_codec::StateCodecError::InvalidExpectedSchemaVersion { expected } => {
                HotSwapError::InvalidExpectedSchemaVersion { expected }
            }
            state_codec::StateCodecError::SchemaVersionMismatch { .. } => {
                HotSwapError::SchemaIncompatible {
                    predecessor_version: predecessor_state_schema_version,
                    successor_version: successor_state_schema_version,
                }
            }
            other => HotSwapError::Internal(format!("state envelope invalid: {other}")),
        })?;

        let payload = envelope.payload;

        // Step 7: Cross-major? → run_migrator.
        let compat = state_codec::detect_compat(
            predecessor_state_schema_version,
            successor_state_schema_version,
        );
        // Story 5.2 review backfill: exhaustive match — Breaking schema must
        // hard-reject, not silently fall through (carry-forward Epic 4 retro #5).
        let payload = match compat {
            SchemaCompat::CrossMajor => {
                run_migrator(
                    &self.dispatcher,
                    &predecessor_scb,
                    &successor_spirit_obj,
                    &payload,
                    successor_manifest,
                    &predecessor_version,
                )
                .await?
            }
            SchemaCompat::SameMajor => payload,
            SchemaCompat::Breaking => {
                return Err(HotSwapError::SchemaIncompatible {
                    predecessor_version: predecessor_state_schema_version,
                    successor_version: successor_state_schema_version,
                });
            }
        };

        // Step 8: replace only the SCB's interior runtime snapshot.  The map
        // continues to point at the original Arc, preserving DRR, watchdog,
        // lifecycle, task-ledger, sandbox and identity state for stale clones.
        let predecessor_runtime = predecessor_scb.replace_runtime(ScbRuntimeSnapshot {
            manifest: successor_manifest.clone(),
            spirit_obj: successor_spirit_obj,
            priority_weight: successor_manifest.scheduling.priority_weight,
            on_crash_action: successor_manifest
                .on_crash
                .as_ref()
                .map(|section| section.action.clone())
                .unwrap_or_default(),
            on_revocation_action: successor_manifest
                .on_revocation
                .as_ref()
                .map(|section| section.action)
                .unwrap_or_default(),
            sandbox_tier: pre_swap_runtime.sandbox_tier,
        });

        // Step 9: Fire on_swap_in(payload) hook on the SUCCESSOR SCB.
        let successor_scb = {
            let map = self.spirits.read().expect("spirits lock poisoned");
            Arc::clone(
                map.get(&predecessor_pid)
                    .expect("successor SCB just inserted"),
            )
        };
        saga.advance_to(SagaPhase::SwapInFired);
        let swap_in_result = self
            .dispatcher
            .fire_on_swap_in(&successor_scb, &payload)
            .await;
        match swap_in_result {
            HookOutcome::Fired { .. }
            | HookOutcome::BudgetWarning80 { .. }
            | HookOutcome::SkippedManifest
            | HookOutcome::DeferredToNextStory => {}
            HookOutcome::BudgetExceeded { .. } | HookOutcome::Panicked { .. } => {
                // Saga: restore predecessor SCB.
                saga.compensate(
                    SagaCompensation::DiscardSuccessor {
                        reason: format!("swap-in failed: {swap_in_result:?}"),
                    },
                    &self.tl,
                    &self.journal,
                )
                .await;
                // Restore the predecessor runtime on the same SCB Arc.
                let map = self.spirits.read().expect("spirits lock poisoned");
                if let Some(scb) = map.get(&predecessor_pid) {
                    scb.replace_runtime(predecessor_runtime);
                }

                return Err(HotSwapError::SwapInFailed {
                    spirit_id: spirit_id.to_string(),
                    error: format!("{swap_in_result:?}"),
                });
            }
        }

        // Step 10: Journal LifecycleEvent::HotSwap.
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.journal
            .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: timestamp_ns,
                lifecycle_event: LifecycleEvent::HotSwap,
                spirit_id: spirit_id.to_string(),
                payload: None,
                effective_sandbox_tier: None,
            }));

        // Step 11: Spawn PostSwapMonitor (30s window).
        saga.advance_to(SagaPhase::Committed);

        // Story 12.5 review fix: snapshot the halts that must SURVIVE the swap
        // — the CURRENT (post-drain) pending set. Step 3's I14 gate legitimately
        // drained the predecessor's halts (resolution), so listing those
        // pre-drain ids made the PostSwapMonitor flag every drained-with-pending
        // swap as HaltSetLoss and auto-revert a valid swap. Only halts still
        // pending after the gate must persist through the post-swap window.
        let pre_swap_halt_ids: Vec<String> = self
            .halt_registry
            .pending_halt_ids()
            .iter()
            .map(|h| h.as_str().to_string())
            .collect();

        // Frame-shape monitoring is opt-in; no pre-swap frame constraints are
        // registered unless the caller supplied them to the monitor.
        let pre_swap_frame_shapes: Vec<String> = Vec::new();
        let invariant_snapshot = PostSwapInvariantSnapshot {
            pre_swap_halt_ids,
            pid: predecessor_pid,
            boot_nonce: predecessor_scb.boot_nonce,
            pre_swap_frame_shapes,
        };

        // Store the exact predecessor runtime for auto-revert.
        {
            let mut reverts = self.pending_reverts.lock();
            reverts.insert(predecessor_pid, predecessor_runtime);
        }

        // Cancel any existing monitor for this PID before spawning a new one.
        {
            let mut monitors = self.active_monitors.lock();
            if let Some(old_handle) = monitors.remove(&predecessor_pid) {
                old_handle.abort();
            }
        }

        let coordinator_arc = Arc::new(Self {
            spirits: Arc::clone(&self.spirits),
            journal: Arc::clone(&self.journal),
            tl: Arc::clone(&self.tl),
            halt_registry: Arc::clone(&self.halt_registry),
            capability: Arc::clone(&self.capability),
            iac: Arc::clone(&self.iac),
            dispatcher: Arc::clone(&self.dispatcher),
            telemetry: Arc::clone(&self.telemetry),
            archive_dir: self.archive_dir.clone(),
            active_monitors: Arc::clone(&self.active_monitors),
            pending_reverts: Arc::clone(&self.pending_reverts),
        });

        let monitor = PostSwapMonitor::new(coordinator_arc, predecessor_pid, invariant_snapshot);
        let monitor_handle = monitor.spawn();
        {
            let mut monitors = self.active_monitors.lock();
            monitors.insert(predecessor_pid, monitor_handle);
        }

        // Step 12: Return HotSwapResult::Completed.
        let latency_ns = start.elapsed().as_nanos() as u64;
        let result = HotSwapResult::new(
            predecessor_pid,
            predecessor_version.clone(),
            successor_manifest
                .class
                .as_ref()
                .map(|c| c.version.clone())
                .unwrap_or_else(|| "unknown".into()),
            0,
            0,
            latency_ns,
            compat,
        )
        .map_err(|e| HotSwapError::Internal(format!("{e:?}")))?;

        Ok(result)
    }

    /// AC7 entry-point: pure precheck (no kernel-state mutation).
    pub fn precheck(
        &self,
        spirit_id: &str,
        successor_manifest_path: &str,
        _successor_version: &str,
    ) -> Result<KernelPrecheckVerdict, HotSwapError> {
        let predecessor_pid = self.resolve_pid(spirit_id)?;
        let successor_manifest = crate::lifecycle::upgrade::load_bundle_from_file(
            std::path::Path::new(successor_manifest_path),
        )
        .map_err(|error| HotSwapError::Internal(format!("successor manifest: {error}")))?;
        let predecessor_scb = {
            let map = self.spirits.read().expect("spirits lock poisoned");
            Arc::clone(
                map.get(&predecessor_pid)
                    .ok_or_else(|| HotSwapError::NotLoaded {
                        spirit_id: spirit_id.to_string(),
                    })?,
            )
        };

        let predecessor_runtime = predecessor_scb.runtime_snapshot();
        let predecessor_halt_protocol_version = predecessor_runtime
            .manifest
            .halt_protocol_compatibility
            .as_ref()
            .map(|h| h.version)
            .unwrap_or(1u32);
        let successor_accepted_versions = vec![predecessor_halt_protocol_version];

        let predecessor_state_schema_version = predecessor_runtime
            .manifest
            .hot_swap
            .as_ref()
            .map(|h| h.state_schema_version)
            .unwrap_or(1u32);
        let successor_state_schema_version = successor_manifest
            .hot_swap
            .as_ref()
            .map(|h| h.state_schema_version)
            .unwrap_or(1u32);

        Ok(HotSwapPrecheck::check(
            &self.halt_registry,
            predecessor_pid,
            predecessor_halt_protocol_version,
            &successor_accepted_versions,
            predecessor_state_schema_version,
            successor_state_schema_version,
        ))
    }

    /// AC3 entry-point: auto-revert called by PostSwapMonitor on invariant violation.
    pub async fn auto_revert(
        &self,
        spirit_pid: u32,
        invariant_violation: PostSwapInvariantViolation,
    ) -> Result<(), HotSwapError> {
        // 1. Fire successor's on_swap_out (graceful shutdown attempt; 2s timeout).
        {
            let scb_opt = {
                let map = self.spirits.read().expect("spirits lock poisoned");
                map.get(&spirit_pid).cloned()
            };
            if let Some(scb) = scb_opt {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    self.dispatcher.fire_on_swap_out(&scb),
                )
                .await;
            }
        }

        // 2. Restore the pre-swap runtime on the same SCB Arc.
        let pre_swap_runtime = {
            let mut reverts = self.pending_reverts.lock();
            reverts.remove(&spirit_pid)
        };
        if let Some(pre_swap_runtime) = pre_swap_runtime {
            let map = self.spirits.read().expect("spirits lock poisoned");
            if let Some(scb) = map.get(&spirit_pid) {
                scb.replace_runtime(pre_swap_runtime);
            }
        }

        // 3. Journal HotSwapAutoReverted.
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.journal
            .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: timestamp_ns,
                lifecycle_event: LifecycleEvent::HotSwapAutoReverted,
                spirit_id: format!("pid-{spirit_pid}"),
                payload: None,
                effective_sandbox_tier: None,
            }));

        // 4. Emit HotSwapAborted IAC frame.
        let payload = serde_json::json!({
            "spirit_pid": spirit_pid,
            "violation": format!("{invariant_violation:?}"),
            "phase": "PostSwapWindow",
            "reason": format!("auto_revert_{invariant_violation:?}"),
        });
        self.tl.insert_frame_event(
            FrameKind::HotSwapAborted,
            spirit_pid,
            None,
            "hot_swap.auto_reverted",
            payload.to_string().as_bytes(),
            FrameOrigin::SpiritAuto,
        );

        Ok(())
    }

    /// Resolve an operator-facing spirit_id to a kernel spirit_pid.
    fn resolve_pid(&self, spirit_id: &str) -> Result<u32, HotSwapError> {
        let map = self.spirits.read().expect("spirits lock poisoned");
        for (pid, scb) in map.iter() {
            if scb.spirit_id == spirit_id {
                return Ok(*pid);
            }
        }
        Err(HotSwapError::NotLoaded {
            spirit_id: spirit_id.to_string(),
        })
    }

    /// Read-only access to the spirits map for monitor invariant checks.
    pub fn spirits_map(&self) -> &Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>> {
        &self.spirits
    }

    /// Read-only access to the halt registry for monitor invariant checks.
    pub fn halt_registry_ref(&self) -> &Arc<HaltRegistry> {
        &self.halt_registry
    }
}
