#![forbid(unsafe_code)]

//! HotSwapCoordinator — kernel-side entry point for Spirit hot-swap.
//!
//! Implements the 12-step hot-swap protocol per ADR-017 binding-v0.3:
//! 1. resolve spirit_id → predecessor_pid
//! 2. snapshot predecessor SCB for saga rollback
//! 3. I14 gate — validate_halt_set over the carried-halt snapshot
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
use std::sync::{Arc, RwLock, Weak};

use parking_lot::Mutex;
use std::time::Instant;

use maos_domain::hot_swap::{HotSwapError, HotSwapResult, PostSwapInvariantViolation};
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i13::AllowedPromotionSet;
use maos_domain::invariants::i3::FrameOrigin;

use crate::capability::cap_policy::{ScopeInstall, ScopeInstallError};
use crate::capability::CapabilityRegistryAdapter;
use crate::halt::{validate_halt_set, HaltRegistry};
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::iac::IacBusAdapter;
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::{
    AnySpiritObj, HostLifecycleEvent, RuntimePublishError, ScbRuntimeSnapshot, SpiritControlBlock,
};
use crate::scheduler::hook_dispatch::HookDispatcher;
use crate::scheduler::HookOutcome;
use crate::telemetry::iac_rt::IacRtMetrics;

use super::migrator::run_migrator;
use super::post_swap_monitor::{PostSwapInvariantSnapshot, PostSwapMonitor};
use super::precheck::{HotSwapPrecheck, PrecheckVerdict as KernelPrecheckVerdict};
use super::saga::{HotSwapSaga, SagaCompensation, SagaPhase};
use super::state_codec::{self, SchemaCompat};

/// Auto-revert state for one committed replacement. Owned solely by that swap's
/// registry window (`MonitorWindow`), so it is freed on clean expiry, cancellation or
/// replacement, or shutdown. The live SCB and committed candidate are scheduler-owned
/// and held only weakly: a revert applies only while that exact runtime is still
/// live, never to an unloaded, reused-PID, or since-changed SCB.
struct PendingRevert {
    scb: Weak<SpiritControlBlock>,
    committed_obj: Weak<dyn AnySpiritObj>,
    /// The committed runtime's publication identity: a revert applies only to this
    /// exact publication, even if a later swap installed the same object `Arc`.
    generation: u64,
    predecessor: ScbRuntimeSnapshot,
    /// What the commit changed in the PID's effective policy scopes; `None` when the
    /// declared set was unchanged or no policy was supplied.
    scope_install: Option<ScopeInstall>,
    /// `Some(displaced)` only when the verified policy replaced the PID's promotion
    /// registration; `displaced` is the exact prior set (`None`: there was none).
    promotion: Option<Option<AllowedPromotionSet>>,
}

/// The one current post-swap window of a PID: its exact deadline, its task, and the
/// revert state it alone owns. A revert in progress holds that state through a
/// [`ClaimedWindow`] while the window itself stays registered, so the monitor task
/// keeps checking and expiring it and shutdown still owns the task.
pub(super) struct MonitorWindow {
    deadline: tokio::time::Instant,
    task: tokio::task::JoinHandle<()>,
    /// `None` while a revert holds the claim.
    pending: Option<PendingRevert>,
}

/// A revert's claim on its window's state across the successor hook. Dropped while
/// still holding the state (the future was cancelled before the revert was decided),
/// it re-arms that same window; dropped after the decision, it closes the window and
/// stops its monitor.
struct ClaimedWindow<'a> {
    owner: &'a HotSwapCoordinator,
    pid: u32,
    task: tokio::task::Id,
    pending: Option<PendingRevert>,
}

impl Drop for ClaimedWindow<'_> {
    fn drop(&mut self) {
        let Some(pending) = self.pending.take() else {
            if let Some(window) = self.owner.take_window(self.pid, self.task) {
                window.task.abort();
            }
            return;
        };
        let mut monitors = self.owner.active_monitors.lock();
        if let Some(window) = monitors
            .get_mut(&self.pid)
            .filter(|w| w.task.id() == self.task)
        {
            window.pending = Some(pending);
        }
    }
}

/// Why the commit section refused to publish.
enum CommitRefusal {
    Unloaded,
    Runtime(RuntimePublishError),
    Scopes(ScopeInstallError),
}

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
    active_monitors: Arc<Mutex<BTreeMap<u32, MonitorWindow>>>,
    /// True only for the composition-root instance: dropping it (shutdown) aborts every
    /// live monitor window, which frees the revert state each one owns. The per-swap
    /// view handed to a monitor never aborts its siblings.
    owns_monitors: bool,
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
            owns_monitors: true,
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
        self.initiate_with_policy(spirit_id, successor_manifest, successor_spirit_obj, None)
            .await
    }

    /// Shared swap path. `policy` is the verified manifest policy: the declared scope
    /// set and the promotion set. `None` leaves scope and promotion authority untouched;
    /// a predecessor's registered promotion set may only be kept or narrowed, never omitted.
    pub(crate) async fn initiate_with_policy(
        &self,
        spirit_id: &str,
        successor_manifest: &crate::scheduler::control_block::SpiritManifestBundle,
        successor_spirit_obj: Arc<dyn AnySpiritObj>,
        policy: Option<(&[Scope], Option<AllowedPromotionSet>)>,
    ) -> Result<HotSwapResult, HotSwapError> {
        let (declared_scopes, promotion) = match policy {
            Some((scopes, set)) => (Some(scopes), Some(set)),
            None => (None, None),
        };
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
        let (pre_swap_runtime, pre_swap_generation) =
            predecessor_scb.runtime_snapshot_with_generation();
        refuse_telemetry_gain(&pre_swap_runtime.manifest, successor_manifest)?;
        self.check_successor_scopes(spirit_id, predecessor_pid, declared_scopes)?;
        if let (Some(pred), Some(succ)) = (self.iac.promotion_set_of(spirit_id), &promotion) {
            if !succ.as_ref().is_some_and(|set| set.is_subset_of(&pred)) {
                return Err(HotSwapError::Internal(
                    "successor intent_promotion_set must keep or narrow its predecessor's".into(),
                ));
            }
        }

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

        // Step 3: I14 gate over the ONE carried-halt snapshot, which the post-swap monitor
        // later inherits. A successor that declares no `halt_protocol_compatibility` is refused
        // while the predecessor has halts to carry.
        saga.advance_to(SagaPhase::HaltContinuityChecked);
        let carried = self.halt_registry.carried_for_spirit(predecessor_pid);
        let successor_accepted_versions: Option<Vec<u32>> = successor_manifest
            .halt_protocol_compatibility
            .as_ref()
            .map(|h| vec![h.version]);
        if let Err(e) = validate_halt_set(
            &carried,
            predecessor_halt_protocol_version,
            successor_accepted_versions.as_deref(),
        ) {
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

        // Story 20-8 (FR8) — the successor keeps every invariant the predecessor
        // declared; refused before any hook fires.
        let dropped = pre_swap_runtime
            .manifest
            .swap_invariants
            .iter()
            .flat_map(|section| &section.preserve)
            .find(|name| {
                !successor_manifest
                    .swap_invariants
                    .iter()
                    .any(|section| section.preserve.contains(name))
            });
        if let Some(invariant) = dropped {
            // The receipt passes the credential redactor, which would erase a
            // valid name shaped like a secret: carry it as reversible hex.
            let hex = (invariant.as_bytes().chunks(2))
                .map(hex::encode)
                .collect::<Vec<_>>()
                .join(":");
            saga.compensate(
                SagaCompensation::RestorePredecessor {
                    reason: format!("swap invariant dropped (utf-8 hex, colon-separated): {hex}"),
                },
                &self.tl,
                &self.journal,
            )
            .await;
            return Err(HotSwapError::SwapInvariantDropped {
                invariant: invariant.clone(),
            });
        }

        // Step 4: Fire on_swap_out hook.
        saga.advance_to(SagaPhase::SwapOutFired);
        let swap_out_result = self.dispatcher.fire_on_swap_out(&predecessor_scb).await;
        match swap_out_result {
            HookOutcome::Fired { .. }
            | HookOutcome::BudgetWarning80 { .. }
            | HookOutcome::SkippedManifest
            | HookOutcome::SkippedQuarantined
            | HookOutcome::DeferredToNextStory
            | HookOutcome::HaltPending => {}
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

        // Refuse an already-unloaded or since-changed PID before staging the candidate
        // or delivering any swap-in; the commit below still rechecks under its lock.
        if !self.is_current(
            predecessor_pid,
            &predecessor_scb,
            &pre_swap_runtime.spirit_obj,
        ) {
            saga.compensate(
                SagaCompensation::DiscardSuccessor {
                    reason: "pid unloaded before swap-in".into(),
                },
                &self.tl,
                &self.journal,
            )
            .await;
            return Err(HotSwapError::NotLoaded {
                spirit_id: spirit_id.to_string(),
            });
        }
        // Step 8: stage the successor as a pinned, UNPUBLISHED candidate. The swap-in
        // runs against this snapshot through a private staging control block (same
        // pid/id/boot nonce, the invocation cap, payload and exception mapping of
        // any hook); the live SCB keeps serving the predecessor and no scheduler,
        // telemetry or archive surface can observe the candidate until commit.
        let candidate_obj = Arc::clone(&successor_spirit_obj);
        let staged_scb = SpiritControlBlock::new(
            predecessor_pid,
            predecessor_scb.spirit_id.clone(),
            successor_manifest.clone(),
            successor_spirit_obj,
            predecessor_scb.boot_nonce,
        );

        // Step 9: fire on_swap_in(payload) on the staged candidate.
        saga.advance_to(SagaPhase::SwapInFired);
        let swap_in_result = self.dispatcher.fire_on_swap_in(&staged_scb, &payload).await;
        match swap_in_result {
            HookOutcome::Fired { .. }
            | HookOutcome::BudgetWarning80 { .. }
            | HookOutcome::SkippedManifest
            | HookOutcome::SkippedQuarantined
            | HookOutcome::DeferredToNextStory
            | HookOutcome::HaltPending => {}
            HookOutcome::BudgetExceeded { .. } | HookOutcome::Panicked { .. } => {
                // Nothing was published, so there is no runtime to restore.
                saga.compensate(
                    SagaCompensation::DiscardSuccessor {
                        reason: format!("swap-in failed: {swap_in_result:?}"),
                    },
                    &self.tl,
                    &self.journal,
                )
                .await;
                return Err(HotSwapError::SwapInFailed {
                    spirit_id: spirit_id.to_string(),
                    error: format!("{swap_in_result:?}"),
                });
            }
        }
        let mut candidate = staged_scb.into_staged_runtime().ok_or_else(|| {
            HotSwapError::Internal("swap-in completed with a shared staging runtime".into())
        })?;
        candidate.sandbox_tier = pre_swap_runtime.sandbox_tier;

        // Commit: the accepted outer outcome is the single authority. Publish only
        // while the original SCB Arc is still the mapped one, is nonterminal, and its
        // live object is still the predecessor we snapshotted. An exclusive map lock
        // serializes this short section with auto-revert and unload; no hook is called
        // and no await occurs while it is held.
        // FR53 (20-3c K1p): the halts that must SURVIVE the post-swap window are exactly the
        // set the I14 gate judged (`carried`, one snapshot). The monitor treats one as lost only
        // if it left the registry; an operator resolving it in the window is not a loss.
        let pre_swap_halt_ids: Vec<String> =
            carried.iter().map(|h| h.as_str().to_string()).collect();

        let published = {
            let map = self.spirits.write().expect("spirits lock poisoned");
            match map.get(&predecessor_pid) {
                Some(live) if Arc::ptr_eq(live, &predecessor_scb) => {
                    // D-21-PF-14: the narrower declared scope set becomes the PID's
                    // effective policy at the same instant the runtime publishes; a
                    // refused publish restores it, so a rejected successor never
                    // leaves policy changed. No policy (synthetic direct caller with no
                    // manifest intake) changes nothing; `Some(&[])` is a real empty
                    // declaration and is installed.
                    let scopes = match declared_scopes {
                        Some(declared) => self
                            .capability
                            .install_successor_scopes(predecessor_pid, declared)
                            .map_err(CommitRefusal::Scopes),
                        None => Ok(None),
                    };
                    scopes.and_then(|scope_install| {
                        match predecessor_scb.publish_runtime_if_current(
                            &pre_swap_runtime.spirit_obj,
                            Some(pre_swap_generation),
                            candidate,
                        ) {
                            Ok((predecessor, generation)) => {
                                // The window's deadline and the halts it must preserve are
                                // fixed at this publication, before any registration or
                                // journal I/O can delay the window's opening.
                                let (monitor, deadline) = self.prepare_monitor(
                                    predecessor_pid,
                                    predecessor_scb.boot_nonce,
                                    pre_swap_halt_ids,
                                );
                                // Authoritative IAC registration follows the exact runtime
                                // publication; the displaced set (matching owner only) is
                                // kept moved, not cloned, for rollback.
                                let promotion = promotion.map(|set| {
                                    self.iac.register_promotion_set_for_pid(
                                        &predecessor_scb.spirit_id,
                                        predecessor_pid,
                                        set,
                                    )
                                });
                                // Step 10: the HotSwap journal entry precedes the
                                // window, so a revert can never be journaled first.
                                self.journal.append_transition(JournalEntry::Lifecycle(
                                    LifecycleEntry {
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_nanos()
                                            as u64,
                                        lifecycle_event: LifecycleEvent::HotSwap,
                                        spirit_id: spirit_id.to_string(),
                                        payload: None,
                                        effective_sandbox_tier: None,
                                    },
                                ));
                                self.open_window(
                                    monitor,
                                    deadline,
                                    PendingRevert {
                                        scb: Arc::downgrade(&predecessor_scb),
                                        committed_obj: Arc::downgrade(&candidate_obj),
                                        generation,
                                        predecessor,
                                        scope_install,
                                        promotion,
                                    },
                                );
                                Ok(())
                            }
                            Err(reason) => {
                                if let Some(install) = &scope_install {
                                    self.capability
                                        .restore_predecessor_scopes(predecessor_pid, install);
                                }
                                Err(CommitRefusal::Runtime(reason))
                            }
                        }
                    })
                }
                _ => Err(CommitRefusal::Unloaded),
            }
        };
        if let Err(reason) = published {
            let (detail, error) = match reason {
                CommitRefusal::Unloaded
                | CommitRefusal::Runtime(RuntimePublishError::Terminal)
                | CommitRefusal::Scopes(ScopeInstallError::UnknownPid) => (
                    "pid unloaded before swap commit",
                    HotSwapError::NotLoaded {
                        spirit_id: spirit_id.to_string(),
                    },
                ),
                CommitRefusal::Runtime(RuntimePublishError::ObjectChanged) => (
                    "runtime changed during swap-in",
                    HotSwapError::SwapInFailed {
                        spirit_id: spirit_id.to_string(),
                        error: "runtime changed during swap-in".into(),
                    },
                ),
                CommitRefusal::Scopes(ScopeInstallError::NotEntitled(scope)) => (
                    "successor scope exceeds the predecessor's at commit",
                    HotSwapError::Internal(format!(
                        "successor declares scope {scope:?} its predecessor does not hold"
                    )),
                ),
            };
            saga.compensate(
                SagaCompensation::DiscardSuccessor {
                    reason: detail.into(),
                },
                &self.tl,
                &self.journal,
            )
            .await;
            return Err(error);
        }

        // Step 11: the journal entry and post-swap monitor window were both
        // registered inside the commit section, in that order.
        saga.advance_to(SagaPhase::Committed);

        // Host-only finalization on the candidate, strictly after publication, the
        // revert record and monitor registration: the coordinator's outcome is the
        // single authority, and a refused/stale candidate never reaches this point.
        candidate_obj.finalize_lifecycle(HostLifecycleEvent::SwapCommitted {
            pid: predecessor_pid,
            boot_nonce: predecessor_scb.boot_nonce,
        });

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

    /// Shared read-only authority check for the operator precheck and actual commit path.
    fn check_successor_scopes(
        &self,
        spirit_id: &str,
        pid: u32,
        declared: Option<&[Scope]>,
    ) -> Result<(), HotSwapError> {
        let Some(declared) = declared else {
            return Ok(());
        };
        self.capability
            .check_successor_scopes(pid, declared)
            .map_err(|error| match error {
                ScopeInstallError::UnknownPid => HotSwapError::NotLoaded {
                    spirit_id: spirit_id.to_string(),
                },
                ScopeInstallError::NotEntitled(scope) => HotSwapError::Internal(format!(
                    "successor declares scope {scope:?} its predecessor does not hold; \
                     a swap may only keep or narrow authority"
                )),
            })
    }

    /// AC7 entry-point: pure precheck (no kernel-state mutation).
    pub fn precheck(
        &self,
        spirit_id: &str,
        successor_manifest_path: &str,
        _successor_version: &str,
    ) -> Result<KernelPrecheckVerdict, HotSwapError> {
        let predecessor_pid = self.resolve_pid(spirit_id)?;
        let (successor_manifest, declared_scopes, _) =
            crate::lifecycle::upgrade::load_bundle_from_file(std::path::Path::new(
                successor_manifest_path,
            ))
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
        refuse_telemetry_gain(&predecessor_runtime.manifest, &successor_manifest)?;
        self.check_successor_scopes(spirit_id, predecessor_pid, Some(declared_scopes.as_slice()))?;
        let predecessor_halt_protocol_version = predecessor_runtime
            .manifest
            .halt_protocol_compatibility
            .as_ref()
            .map(|h| h.version)
            .unwrap_or(1u32);
        // The successor's own declaration, as the swap gate reads it; an omission accepts nothing.
        let successor_accepted_versions: Vec<u32> = successor_manifest
            .halt_protocol_compatibility
            .iter()
            .map(|h| h.version)
            .collect();

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

    /// AC3 entry-point: auto-revert on an invariant violation. Acts only on the PID's
    /// current, unexpired post-swap window (claimed here, so a revert runs at most
    /// once at a time); with no such window it returns `NotLoaded` and changes and
    /// journals nothing.
    pub async fn auto_revert(
        &self,
        spirit_pid: u32,
        invariant_violation: PostSwapInvariantViolation,
    ) -> Result<(), HotSwapError> {
        self.revert_window(spirit_pid, None, invariant_violation)
            .await
    }

    /// Revert the PID's current window. A `scoped` caller is that window's own task and
    /// claims it only while its task identity still matches, so a superseded task can
    /// never consume a newer window. Every path refuses once the window's exact
    /// deadline is reached. The claim stays registered with its window across the
    /// successor hook, so the monitor keeps running and shutdown still owns the task;
    /// cancelling this future re-arms the window and a retry still finds the rollback.
    pub(super) async fn revert_window(
        &self,
        spirit_pid: u32,
        task: Option<tokio::task::Id>,
        invariant_violation: PostSwapInvariantViolation,
    ) -> Result<(), HotSwapError> {
        let mut claim =
            self.claim_window(spirit_pid, task)
                .ok_or_else(|| HotSwapError::NotLoaded {
                    spirit_id: format!("pid-{spirit_pid}"),
                })?;
        self.revert_committed(spirit_pid, &mut claim.pending, invariant_violation)
            .await
    }

    /// Claim the revert state of the PID's unexpired, unclaimed window. With `task`
    /// set, only if that task is still the window's owner.
    fn claim_window(
        &self,
        spirit_pid: u32,
        task: Option<tokio::task::Id>,
    ) -> Option<ClaimedWindow<'_>> {
        let mut monitors = self.active_monitors.lock();
        let window = monitors.get_mut(&spirit_pid)?;
        if task.is_some_and(|id| window.task.id() != id)
            || tokio::time::Instant::now() >= window.deadline
        {
            return None;
        }
        Some(ClaimedWindow {
            owner: self,
            pid: spirit_pid,
            task: window.task.id(),
            pending: Some(window.pending.take()?),
        })
    }

    /// Remove the PID's window (and with it any revert state it still owns), only if
    /// `task` is still its owner.
    pub(super) fn take_window(
        &self,
        spirit_pid: u32,
        task: tokio::task::Id,
    ) -> Option<MonitorWindow> {
        let mut monitors = self.active_monitors.lock();
        if monitors.get(&spirit_pid)?.task.id() != task {
            return None;
        }
        monitors.remove(&spirit_pid)
    }

    /// Whether `task` still owns the PID's registered window.
    pub(super) fn owns_window(&self, spirit_pid: u32, task: tokio::task::Id) -> bool {
        (self.active_monitors.lock().get(&spirit_pid)).is_some_and(|w| w.task.id() == task)
    }

    /// Whether `scb` is still the mapped, nonterminal runtime currently running `obj`.
    fn is_current(
        &self,
        pid: u32,
        scb: &Arc<SpiritControlBlock>,
        obj: &Arc<dyn AnySpiritObj>,
    ) -> bool {
        self.spirits
            .read()
            .expect("spirits lock poisoned")
            .get(&pid)
            .is_some_and(|live| Arc::ptr_eq(live, scb))
            && scb.current_state() != crate::scheduler::control_block::ScbLifecycleState::Unloaded
            && std::ptr::addr_eq(
                Arc::as_ptr(&scb.runtime_cell().read().spirit_obj),
                Arc::as_ptr(obj),
            )
    }

    /// Capture the post-swap window of a just-published swap: the publication instant
    /// fixes its exact deadline, and `pre_swap_halt_ids` (the carried set the I14 gate
    /// judged, one snapshot) are the halts that must SURVIVE it (FR53, 20-3c K1p; one
    /// counts as lost only if it left the registry).
    fn prepare_monitor(
        &self,
        pid: u32,
        boot_nonce: u64,
        pre_swap_halt_ids: Vec<String>,
    ) -> (PostSwapMonitor, tokio::time::Instant) {
        let started = tokio::time::Instant::now();
        let view = Arc::new(Self {
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
            owns_monitors: false,
        });
        // Frame-shape monitoring is opt-in; none is registered here.
        let monitor = PostSwapMonitor::new(
            view,
            pid,
            PostSwapInvariantSnapshot {
                pre_swap_halt_ids,
                pid,
                boot_nonce,
                pre_swap_frame_shapes: Vec::new(),
            },
        );
        let deadline = started + monitor.window;
        (monitor, deadline)
    }

    /// Register the exact post-swap window for a just-published swap, after its
    /// journal entry. Called inside the commit's `spirits` write section, so an older
    /// commit can never replace a newer one's slot; the registry lock is held across
    /// removal of the old window, the spawn and the insertion, and the old window's
    /// task is aborted after.
    fn open_window(
        &self,
        monitor: PostSwapMonitor,
        deadline: tokio::time::Instant,
        pending: PendingRevert,
    ) {
        let pid = monitor.spirit_pid;
        let displaced = {
            let mut monitors = self.active_monitors.lock();
            monitors.retain(|_, window| !window.task.is_finished());
            let task = monitor.spawn_until(deadline, true);
            monitors.insert(
                pid,
                MonitorWindow {
                    deadline,
                    task,
                    pending: Some(pending),
                },
            )
        };
        if let Some(old) = displaced {
            old.task.abort();
        }
    }

    /// Auto-revert for one committed replacement. `claimed` is the revert state taken
    /// from the window by `revert_window`; it is consumed (set to `None`) only once
    /// the revert is decided, so a cancellation during the hook re-arms the window.
    async fn revert_committed(
        &self,
        spirit_pid: u32,
        claimed: &mut Option<PendingRevert>,
        invariant_violation: PostSwapInvariantViolation,
    ) -> Result<(), HotSwapError> {
        // 1. Only a revert whose recorded runtime is still the live one may act: the
        // same SCB Arc is mapped, nonterminal, and still the exact committed
        // publication (object and generation). Never resurrect an unloaded or
        // since-changed runtime. The record holds only Weak refs, so an unloaded
        // SCB/object is already gone.
        let not_loaded = || HotSwapError::NotLoaded {
            spirit_id: format!("pid-{spirit_pid}"),
        };
        let pending = claimed.as_ref().expect("claimed window");
        let generation = pending.generation;
        let (Some(scb), Some(committed_obj)) =
            (pending.scb.upgrade(), pending.committed_obj.upgrade())
        else {
            *claimed = None;
            return Err(not_loaded());
        };
        if !self.is_current(spirit_pid, &scb, &committed_obj)
            || scb.runtime_generation() != generation
        {
            *claimed = None;
            return Err(not_loaded());
        }

        // Fire successor's on_swap_out (graceful shutdown attempt; 2s timeout). The hook
        // runs only on exactly the committed publication, never on a newer runtime.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            self.dispatcher
                .fire_on_swap_out_exact(&scb, &committed_obj, generation),
        )
        .await;
        let PendingRevert {
            predecessor,
            scope_install,
            promotion,
            ..
        } = claimed.take().expect("claimed window");

        // The scope set and the promotion registration are restored in the same
        // locked section that republishes the predecessor, so only under the same live
        // SCB and exact publication. Scopes come back only while the policy still holds
        // exactly what the commit installed; a promotion set is restored only when the
        // verified policy changed it AND this swap's registration is still the live one.
        let restored = {
            let map = self.spirits.write().expect("spirits lock poisoned");
            match map.get(&spirit_pid) {
                Some(live) if Arc::ptr_eq(live, &scb) => scb
                    .publish_runtime_if_current(&committed_obj, Some(generation), predecessor)
                    .map(|_| {
                        if let Some(install) = &scope_install {
                            self.capability
                                .restore_predecessor_scopes(spirit_pid, install);
                        }
                        if let Some(old) = promotion {
                            // One entry-locked step each: a registration another PID
                            // now owns is never overwritten or cleared, and the owned
                            // slot is never observably empty.
                            match old {
                                Some(old) => {
                                    self.iac
                                        .restore_promotion_set(&scb.spirit_id, spirit_pid, old)
                                }
                                None => self
                                    .iac
                                    .unregister_promotion_set_for_pid(&scb.spirit_id, spirit_pid),
                            }
                        }
                    })
                    .is_ok(),
                _ => false,
            }
        };
        if !restored {
            return Err(not_loaded());
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

impl Drop for HotSwapCoordinator {
    /// Shutdown: abort every live monitor window (a claimed one included, so a revert in
    /// its successor hook is cancelled) so the revert state each owns is freed.
    fn drop(&mut self) {
        if self.owns_monitors {
            let windows = std::mem::take(&mut *self.active_monitors.lock());
            for (_, window) in windows {
                window.task.abort();
            }
        }
    }
}

/// An in-place successor may keep or lower its predecessor's `on_telemetry_event` declaration
/// but never gain or enlarge one: the root-wide `scalar.tap.*` read is a host subscription the
/// ordinary `Scope::entitles` check does not cover, and no separately authorized route to widen
/// it exists yet. Refused before the saga starts, so a refused successor is never installed (and
/// so never granted).
fn refuse_telemetry_gain(
    predecessor: &crate::scheduler::control_block::SpiritManifestBundle,
    successor: &crate::scheduler::control_block::SpiritManifestBundle,
) -> Result<(), HotSwapError> {
    let Some(requested) = successor.lifecycle.telemetry else {
        return Ok(());
    };
    match predecessor.lifecycle.telemetry {
        Some(held) if requested.time_cap_seconds() <= held.time_cap_seconds() => Ok(()),
        held => Err(HotSwapError::Internal(format!(
            "successor telemetry subscription ({}s) exceeds its predecessor's ({}); \
             gaining or enlarging telemetry needs a separately authorized route",
            requested.time_cap_seconds(),
            held.map_or("none".to_owned(), |h| format!("{}s", h.time_cap_seconds())),
        ))),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/coordinator_publication.rs"]
mod publication_tests;
