#![forbid(unsafe_code)]

//! Revocation applier — the propagation pipeline body.
//!
//! Matches SCBs against CRL entries, revokes capability tokens,
//! emits `SpiritRevoked` frames, applies `[on_revocation].action`,
//! and routes through `terminate_spirit(RevocationTerminated)`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock};

use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::revocation::{
    ApplyEntry, ApplyReport, CrlId, RevocationAction, RevocationError, SignedRevocationList,
};

use crate::capability::cap_tokens::monotonic_now_ns;
use crate::halt::{terminate_spirit, HaltRegistry};
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::SpiritControlBlock;
use crate::scheduler::SpiritSchedulerAdapter;
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};
use maos_domain::invariants::i3::FrameOrigin;

/// Aggregator — holds all adapters needed to propagate a CRL.
#[maos_attrs::i9_exempt(reason = "revocation applier composite; holds exempt adapter Arcs")]
pub struct RevocationApplier {
    spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    capability: Arc<crate::capability::CapabilityRegistryAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    iac: Arc<crate::iac::IacBusAdapter>,
    halt_registry: Arc<HaltRegistry>,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    telemetry: Arc<IacRtMetrics>,
    /// Idempotency: track applied CRL IDs to reject re-imports.
    applied_crls: Arc<RwLock<BTreeSet<CrlId>>>,
    /// Story 5.4 — active drain handles for `DrainThenTerminate` policy.
    active_drains: Arc<Mutex<BTreeMap<u32, tokio::task::JoinHandle<()>>>>,
}

impl RevocationApplier {
    pub fn new(
        spirits: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        capability: Arc<crate::capability::CapabilityRegistryAdapter>,
        scheduler: Arc<SpiritSchedulerAdapter>,
        iac: Arc<crate::iac::IacBusAdapter>,
        halt_registry: Arc<HaltRegistry>,
        tl: Arc<TransparencyLogAdapter>,
        journal: Arc<JournalAdapter>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            spirits,
            capability,
            scheduler,
            iac,
            halt_registry,
            tl,
            journal,
            telemetry,
            applied_crls: Arc::new(RwLock::new(BTreeSet::new())),
            active_drains: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Apply a parsed + signature-verified CRL. Returns one `ApplyEntry` per
    /// matched Spirit. Idempotent: re-applying the same CRL returns
    /// `Err(RevocationError::AlreadyApplied { id })`.
    pub async fn apply_crl(
        &self,
        crl: SignedRevocationList,
    ) -> Result<ApplyReport, RevocationError> {
        let start_ns = monotonic_now_ns();

        // 1. Idempotency check (read-only)
        {
            let read_set = self
                .applied_crls
                .read()
                .expect("applied_crls lock poisoned");
            if read_set.contains(&crl.id) {
                return Err(RevocationError::AlreadyApplied {
                    id: crl.id.to_string(),
                });
            }
        }

        let mut matched_count = 0usize;
        let mut revoked_count = 0usize;
        let mut halt_receipts_produced = 0usize;
        let mut tokens_revoked_total = 0usize;
        let mut per_spirit = Vec::new();

        // 2. Iterate spirits and match against CRL entries
        let spirits_snapshot = {
            let spirits = self.spirits.read().expect("spirits lock poisoned");
            spirits.values().cloned().collect::<Vec<_>>()
        };

        for scb in spirits_snapshot {
            let class_name = scb
                .manifest
                .class
                .as_ref()
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let version = scb
                .manifest
                .class
                .as_ref()
                .map(|c| c.version.clone())
                .unwrap_or_default();

            for entry in &crl.entries {
                if class_name != entry.spirit_class {
                    continue;
                }
                let matches = crate::revocation::version_match::semver_range_contains(
                    &version,
                    &entry.version_range,
                )
                .unwrap_or(false);
                if !matches {
                    continue;
                }

                matched_count += 1;
                revoked_count += 1;

                // Revoke capability tokens
                let tokens_revoked = self
                    .capability
                    .revoke_all_for_pid(scb.pid)
                    .map_err(|_e| RevocationError::Io(format!(
                        "capability revocation failed for spirit pid={}",
                        scb.pid
                    )))?;
                tokens_revoked_total += tokens_revoked;

                let in_flight_token_count = scb
                    .task_assignments_in_flight
                    .lock()
                    .map(|v| v.len())
                    .unwrap_or(0);

                // Emit SpiritRevoked frame
                let payload = serde_json::json!({
                    "spirit_id": scb.spirit_id,
                    "spirit_pid": scb.pid,
                    "spirit_class": class_name,
                    "spirit_version": version,
                    "revocation_origin": crl.origin.as_str(),
                    "revocation_reason": entry.reason,
                    "applied_at_ns": monotonic_now_ns(),
                    "in_flight_token_count": in_flight_token_count,
                    "action": scb.on_revocation_action.as_str(),
                });
                let payload_bytes = serde_json::to_vec(&payload)
                    .map_err(|e| RevocationError::Deserialize(format!("payload serialize: {e}")))?;
                self.tl.insert_frame_event(
                    FrameKind::SpiritRevoked,
                    scb.pid,
                    None,
                    "spirit.revoked",
                    &payload_bytes,
                    FrameOrigin::Kernel,
                );

                // Apply declared action
                let receipts = match scb.on_revocation_action {
                    RevocationAction::TerminateImmediately => {
                        let receipts = terminate_spirit(
                            &self.tl,
                            &self.halt_registry,
                            scb.pid,
                            &scb.spirit_id,
                            maos_domain::halt::TerminationKind::RevocationTerminated,
                            scb.boot_nonce,
                        );
                        let receipt_count = receipts.len();
                        // Fire-and-forget unload
                        let scheduler = Arc::clone(&self.scheduler);
                        let pid = scb.pid;
                        tokio::spawn(async move {
                            if let Err(e) = scheduler.unload(pid).await {
                                eprintln!(
                                    "revocation: unload failed for pid={pid} after RevocationTerminated: {e}"
                                );
                            }
                        });
                        receipt_count
                    }
                    RevocationAction::DrainThenTerminate | RevocationAction::Quarantine => {
                        let deadline_ms = scb
                            .manifest
                            .supervision
                            .as_ref()
                            .map(|s| s.progress_threshold_ms * 2)
                            .unwrap_or(60_000);
                        let tl = Arc::clone(&self.tl);
                        let halt = Arc::clone(&self.halt_registry);
                        let scheduler = Arc::clone(&self.scheduler);
                        let pid = scb.pid;
                        let spirit_id = scb.spirit_id.clone();
                        let boot_nonce = scb.boot_nonce;
                        let drains = Arc::clone(&self.active_drains);
                        let handle = tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                deadline_ms as u64,
                            ))
                            .await;
                            let _receipts = terminate_spirit(
                                &tl,
                                &halt,
                                pid,
                                &spirit_id,
                                maos_domain::halt::TerminationKind::RevocationTerminated,
                                boot_nonce,
                            );
                            if let Err(e) = scheduler.unload(pid).await {
                                eprintln!(
                                    "revocation: drain-then-terminate unload failed for pid={pid}: {e}"
                                );
                            }
                            // Prune self from active_drains on completion
                            if let Ok(mut m) = drains.lock() {
                                m.remove(&pid);
                            }
                        });
                        {
                            let mut drains = self.active_drains.lock().expect("active_drains poisoned");
                            drains.insert(scb.pid, handle);
                        }
                        0 // receipts produced asynchronously
                    }
                    _ => {
                        eprintln!(
                            "revocation: unknown on_revocation_action={:?} for spirit pid={}, skipping termination",
                            scb.on_revocation_action, scb.pid
                        );
                        0
                    },
                };

                if scb.on_revocation_action == RevocationAction::Quarantine {
                    let marker = serde_json::json!({
                        "spirit_id": scb.spirit_id,
                        "spirit_pid": scb.pid,
                        "quarantine_requested": true,
                    });
                    let marker_bytes = serde_json::to_vec(&marker).map_err(|e| {
                        RevocationError::Deserialize(format!("marker serialize: {e}"))
                    })?;
                    self.tl.insert_frame_event(
                        FrameKind::CapabilityInvocation,
                        scb.pid,
                        None,
                        "spirit.quarantine_requested",
                        &marker_bytes,
                        FrameOrigin::Kernel,
                    );
                }

                halt_receipts_produced += receipts;

                // Journal LifecycleEvent::Revoked
                let now_ns = monotonic_now_ns();
                self.journal
                    .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                        timestamp: now_ns / 1_000_000_000,
                        lifecycle_event: LifecycleEvent::Revoked,
                        spirit_id: scb.spirit_id.clone(),
                        effective_sandbox_tier: None,
                    }));

                per_spirit.push(ApplyEntry {
                    spirit_id: scb.spirit_id.clone(),
                    spirit_pid: scb.pid,
                    spirit_class: class_name.clone(),
                    spirit_version: version.clone(),
                    action: scb.on_revocation_action,
                    tokens_revoked,
                    halt_receipts_produced: receipts,
                    in_flight_token_count,
                });

                // Break after first matching entry for this SCB — avoid duplicate
                // journal entries, frames, and terminate calls.
                break;
            }
        }

        let latency_ns = monotonic_now_ns().saturating_sub(start_ns);
        self.telemetry
            .record_iac_rt(Service::RevocationApplier, Outcome::Ok, latency_ns / 1000);

        // Insert applied CRL id only after successful processing.
        // If two concurrent callers race, the second will see AlreadyApplied
        // on its next attempt.
        {
            let mut write_set = self
                .applied_crls
                .write()
                .expect("applied_crls lock poisoned");
            write_set.insert(crl.id);
        }

        Ok(ApplyReport {
            crl_id: crl.id,
            origin: crl.origin,
            matched_count,
            revoked_count,
            halt_receipts_produced,
            tokens_revoked_total,
            apply_latency_ns: latency_ns,
            per_spirit,
        })
    }

    /// Remove a CRL ID from the applied set (for `--force` re-apply).
    pub fn forget(&self, id: CrlId) {
        let mut set = self
            .applied_crls
            .write()
            .expect("applied_crls lock poisoned");
        set.remove(&id);
    }

    /// List already-applied CRLs (for `maosctl revocations list`).
    pub fn list_applied(&self) -> Vec<CrlId> {
        let set = self
            .applied_crls
            .read()
            .expect("applied_crls lock poisoned");
        set.iter().cloned().collect()
    }
}
