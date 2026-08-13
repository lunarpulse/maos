#![forbid(unsafe_code)]

//! Revocation applier — the propagation pipeline body.
//!
//! Matches SCBs against CRL entries, revokes capability tokens,
//! emits `SpiritRevoked` frames, applies `[on_revocation].action`,
//! and routes through `terminate_spirit(RevocationTerminated)`.

use std::collections::BTreeMap;
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
    _iac: Arc<crate::iac::IacBusAdapter>,
    halt_registry: Arc<HaltRegistry>,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    telemetry: Arc<IacRtMetrics>,
    /// Validated rules shared with scheduler admission. Its gate atomically
    /// reserves CRLs, installs rules, and snapshots existing SCBs.
    rules: Arc<crate::revocation::rules::ValidatedRevocationRules>,
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
        let rules = Arc::new(crate::revocation::rules::ValidatedRevocationRules::new());
        scheduler.set_revocation_rules(Arc::clone(&rules));
        Self {
            spirits,
            capability,
            scheduler,
            _iac: iac,
            halt_registry,
            tl,
            journal,
            telemetry,
            rules,
            active_drains: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    /// Apply a parsed + signature-verified CRL. The shared asynchronous rule
    /// store reserves the ID and blocks admissions until the existing-SCB
    /// propagation pass commits or rolls back.
    pub async fn apply_crl(
        &self,
        crl: SignedRevocationList,
    ) -> Result<ApplyReport, RevocationError> {
        let start_ns = monotonic_now_ns();
        let _admission_gate = self.rules.admission_guard().await;
        if self.rules.contains_locked(crl.id) {
            return Err(RevocationError::AlreadyApplied {
                id: crl.id.to_string(),
            });
        }
        self.rules.install_locked(crl.id, crl.entries.clone());

        let result = async {
            let mut matched_count = 0usize;
            let mut revoked_count = 0usize;
            let mut halt_receipts_produced = 0usize;
            let mut tokens_revoked_total = 0usize;
            let mut per_spirit = Vec::new();
            let spirits_snapshot = {
                let spirits = self.spirits.read().expect("spirits lock poisoned");
                spirits.values().cloned().collect::<Vec<_>>()
            };

            for scb in spirits_snapshot {
                let runtime = scb.runtime_snapshot();
                let Some(class) = &runtime.manifest.class else {
                    continue;
                };
                let class_name = class.name.clone();
                let version = class.version.clone();
                let entry = crl
                    .entries
                    .iter()
                    .find(|entry| entry.spirit_class == class_name)
                    .cloned();
                let Some(entry) = entry else {
                    continue;
                };
                if !crate::revocation::version_match::semver_range_contains(
                    &version,
                    &entry.version_range,
                )
                .map_err(|error| RevocationError::MalformedVersionRange {
                    range: entry.version_range.clone(),
                    reason: error.to_string(),
                })? {
                    continue;
                }
                matched_count += 1;
                revoked_count += 1;
                let tokens_revoked = self.capability.revoke_all_for_pid(scb.pid).map_err(|_| {
                    RevocationError::Io(format!("capability revocation failed for spirit pid={}", scb.pid))
                })?;
                tokens_revoked_total += tokens_revoked;
                let in_flight_token_count = scb
                    .task_assignments_in_flight
                    .lock()
                    .map(|assignments| assignments.len())
                    .map_err(|_| RevocationError::Io("in-flight task lock poisoned".into()))?;

                let payload_bytes = serde_json::to_vec(&serde_json::json!({
                    "spirit_id": scb.spirit_id,
                    "spirit_pid": scb.pid,
                    "spirit_class": &class_name,
                    "spirit_version": &version,
                    "revocation_origin": crl.origin.as_str(),
                    "revocation_reason": &entry.reason,
                    "applied_at_ns": monotonic_now_ns(),
                    "in_flight_token_count": in_flight_token_count,
                    "action": runtime.on_revocation_action.as_str(),
                }))
                .map_err(|error| RevocationError::Deserialize(format!("payload serialize: {error}")))?;
                self.tl.insert_frame_event(
                    FrameKind::SpiritRevoked, scb.pid, None, "spirit.revoked", &payload_bytes, FrameOrigin::Kernel,
                );

                let receipts = match runtime.on_revocation_action {
                    RevocationAction::TerminateImmediately => {
                        let receipts = terminate_spirit(
                            &self.tl, &self.halt_registry, scb.pid, &scb.spirit_id,
                            maos_domain::halt::TerminationKind::RevocationTerminated, scb.boot_nonce,
                        );
                        self.scheduler.unload(scb.pid).await.map_err(|error| {
                            RevocationError::Io(format!("unload revoked spirit pid={}: {error}", scb.pid))
                        })?;
                        receipts.len()
                    }
                    RevocationAction::DrainThenTerminate | RevocationAction::Quarantine => {
                        if runtime.on_revocation_action == RevocationAction::Quarantine {
                            let marker_bytes = serde_json::to_vec(&serde_json::json!({
                                "spirit_id": scb.spirit_id, "spirit_pid": scb.pid, "quarantine_requested": true,
                            })).map_err(|error| RevocationError::Deserialize(format!("marker serialize: {error}")))?;
                            self.tl.insert_frame_event(
                                FrameKind::CapabilityInvocation, scb.pid, None, "spirit.quarantine_requested",
                                &marker_bytes, FrameOrigin::Kernel,
                            );
                        }
                        let deadline_ms = runtime.manifest.supervision.as_ref()
                            .map(|supervision| u64::from(supervision.progress_threshold_ms).saturating_mul(2))
                            .unwrap_or(60_000);
                        let tl = Arc::clone(&self.tl);
                        let halt_registry = Arc::clone(&self.halt_registry);
                        let scheduler = Arc::clone(&self.scheduler);
                        let active_drains = Arc::clone(&self.active_drains);
                        let spirit_id = scb.spirit_id.clone();
                        let pid = scb.pid;
                        let boot_nonce = scb.boot_nonce;
                        let handle = tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(deadline_ms)).await;
                            let _ = terminate_spirit(
                                &tl, &halt_registry, pid, &spirit_id,
                                maos_domain::halt::TerminationKind::RevocationTerminated, boot_nonce,
                            );
                            if let Err(error) = scheduler.unload(pid).await {
                                eprintln!("revocation: deferred unload failed for pid={pid}: {error}");
                            }
                            if let Ok(mut drains) = active_drains.lock() {
                                drains.remove(&pid);
                            }
                        });
                        self.active_drains
                            .lock()
                            .expect("active revocation drains lock poisoned")
                            .insert(scb.pid, handle);
                        0
                    }
                    _ => {
                        return Err(RevocationError::UnsupportedAction(format!(
                            "{:?}", runtime.on_revocation_action
                        )));
                    }
                };

                halt_receipts_produced += receipts;
                self.journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                    timestamp: monotonic_now_ns() / 1_000_000_000,
                    lifecycle_event: LifecycleEvent::Revoked,
                    spirit_id: scb.spirit_id.clone(),
                    payload: None,
                    effective_sandbox_tier: None,
                }));
                per_spirit.push(ApplyEntry {
                    spirit_id: scb.spirit_id.clone(),
                    spirit_pid: scb.pid,
                    spirit_class: class_name,
                    spirit_version: version,
                    action: runtime.on_revocation_action,
                    tokens_revoked,
                    halt_receipts_produced: receipts,
                    in_flight_token_count,
                });
            }

            let latency_ns = monotonic_now_ns().saturating_sub(start_ns);
            self.telemetry.record_iac_rt(Service::RevocationApplier, Outcome::Ok, latency_ns / 1000);
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
        }.await;

        if result.is_err() {
            self.rules.remove_locked(crl.id);
        }
        result
    }

    /// Remove a CRL ID and its validated admission rule (for `--force` re-apply).
    pub async fn forget(&self, id: CrlId) {
        self.rules.forget(id).await;
    }

    /// List committed or in-flight CRL IDs (for `maosctl revocations list`).
    pub fn list_applied(&self) -> Vec<CrlId> {
        self.rules.list()
    }
}
