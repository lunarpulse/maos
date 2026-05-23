#![deny(unsafe_code)]

//! Security Manager — supervised service per §4.3.
//!
//! Enforces sandbox tiers, secret isolation, and approval-class
//! mediation. Story 1b.3 lands T0/T1/T2 tier enforcement and
//! per-Spirit resource caps.
//!
//! Story 2.1 adds capability-declaration → policy-table wiring
//! (replacing the hardcoded injection in `maos-bin::main`),
//! `OutputShapePredicate` scaffold, and a `DriftEvent` channel.

pub mod approval;
pub mod crypto;
pub mod drift;
pub mod manifest;
pub mod posture;
pub mod sandbox;

pub use crypto::RingCryptoProvider;
pub use manifest::{resolve_caps, ManifestError, ResolvedCaps, ResourceCaps, SandboxConfig};
pub use maos_domain::ports::SecurityManagerPort;
// Story 1b.5c — appended to preserve original re-export order so the
// signature_hash of each existing symbol remains stable under
// `check-service-boundary`'s use-item hashing (the gate hashes the
// whole `pub use` token-tree per member; reordering would falsely
// flag every existing symbol as removed-and-re-added).
pub use manifest::{
    Author, Budget, CapabilitiesRequired, ClassSection, OutputShape, Posture, PostureSection,
    ProviderCapabilities,
};
// Story 2.1 — appended to preserve re-export order (same discipline).
pub use manifest::{capabilities_required_to_scopes, OutputShapePredicate, OutputShapeViolation};
// Story 2.3 — appended for P2 port-pair completeness (RingCryptoProvider adapter → CryptoProvider Port).
pub use drift::{make_drift_channel, DriftEvent};
pub use maos_domain::ports::CryptoProvider;
pub use sandbox::{
    classify_exit, spawn_sandboxed, SandboxSpec, SandboxViolation, SandboxedChild, SpawnError,
};
// Story 3.2 — appended to preserve re-export order.
pub use manifest::{EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection, ScalarPredicate};
// Story 5.1 — appended to preserve re-export order.
pub use manifest::{LifecycleSection, SchedulingSection};
// Story 5.3 — appended to preserve re-export order.
pub use manifest::{OnCrashSection, SupervisionSection};
pub use posture::{PostureError, PostureState};

use std::sync::Arc;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::scheduler::SpiritSchedulerPort;
use tokio::sync::mpsc;

use crate::capability::cap_audit::{self, CapAuditEvent};
use crate::capability::cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable};

/// Security error raised during admission or enforcement.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("sandbox tier unsupported at this version: requested {0}")]
    SandboxTierUnsupported(SandboxTier),
    #[error("sandbox unavailable on platform: {reason}")]
    SandboxUnavailable { reason: String },
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("policy table lookup failed for spirit {spirit_pid}")]
    PolicyLookup { spirit_pid: u32 },
    #[error("T3 admission failed: {0}")]
    T3AdmissionFailed(String),
}

/// Adapter — implements `SecurityManagerPort` with sandbox tier
/// enforcement and approval mediation.
///
/// Promoted from ZST (v0.1-α) to hold `Arc<PolicyTable>` (Story 1b.3).
/// Story 2.1 adds an optional drift-event sender.
#[maos_attrs::i9_exempt(
    reason = "security manager adapter; holds Arc<PolicyTable> for runtime policy enforcement — structural-state caching per I9"
)]
#[derive(Debug, Clone)]
pub struct SecurityManagerAdapter {
    policy: Arc<PolicyTable>,
    /// Drift-event channel sender (Story 2.1 AC4).
    /// The runtime detector that emits events ships in Story 9.x.
    drift_sender: Option<mpsc::Sender<DriftEvent>>,
}

impl SecurityManagerAdapter {
    pub fn new(policy: Arc<PolicyTable>) -> Self {
        Self {
            policy,
            drift_sender: None,
        }
    }

    /// Set the drift-event sender (Story 2.1 AC4).
    ///
    /// The sender is consumed by the runtime drift detector at Story 9.x.
    pub fn with_drift_sender(mut self, sender: mpsc::Sender<DriftEvent>) -> Self {
        self.drift_sender = Some(sender);
        self
    }

    /// Access the underlying policy table (for tests and composition root).
    pub fn policy(&self) -> &Arc<PolicyTable> {
        &self.policy
    }

    /// Admit a Spirit: compute effective tier, reject T3+, journal Load,
    /// wire manifest-scoped capabilities into the policy table.
    ///
    /// Story 2.1: added `caps_required` parameter for capability-declaration
    /// → policy-table wiring (replaces hardcoded injection in `maos-bin`).
    pub fn admit_spirit(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        _manifest: &SandboxConfig,
        caps: &ResourceCaps,
        caps_required: &CapabilitiesRequired,
        output_shape: Option<&OutputShape>,
        journal: &dyn SpiritSchedulerPort,
        posture_section: &PostureSection,
        epistemic_policy: Option<&EpistemicPolicySection>,
        _scheduling: Option<&SchedulingSection>,
        _lifecycle: Option<&LifecycleSection>,
        _on_crash: Option<&OnCrashSection>,
        _supervision: Option<&SupervisionSection>,
    ) -> Result<SandboxSpec, SecurityError> {
        {
            let declared_scopes = capabilities_required_to_scopes(caps_required);
            let inner = self.policy.inner().load_full();

            let trust_tier = inner
                .manifest_scopes
                .get(&spirit_pid)
                .map(|m| m.trust_tier)
                .unwrap_or(TrustTier::Verified);

            let effective = self
                .policy
                .effective_sandbox_tier(spirit_pid, trust_tier, &inner);

            let mut new_inner = (*inner).clone();
            new_inner.manifest_scopes.insert(
                spirit_pid,
                ManifestCapabilityScope {
                    scopes: declared_scopes,
                    declared_tier: effective,
                    trust_tier,
                },
            );
            new_inner.spirit_postures.insert(
                spirit_pid,
                crate::security::posture::PostureState {
                    current: posture_section.default,
                    allowed_max: posture_section.allowed_max,
                    epistemic_policy: epistemic_policy
                        .cloned()
                        .unwrap_or_else(EpistemicPolicySection::default_open_fail),
                },
            );
            self.policy.update(new_inner);
        }

        let inner = self.policy.inner().load_full();

        let trust_tier = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.trust_tier)
            .unwrap_or(TrustTier::Verified);

        let effective = self
            .policy
            .effective_sandbox_tier(spirit_pid, trust_tier, &inner);

        // Story 5.5a: T3 is now admissible on Linux with container isolation.
        // T4 remains rejected (WASM tier scheduled for v2.0).
        if effective == SandboxTier::T3 {
            let lock = crate::security::sandbox::t3::image_lock::T3ImageLock::load_default()
                .map_err(|e| SecurityError::T3AdmissionFailed(e.to_string()))?;
            if let Some(ref image_pin) = _manifest.image_pin {
                lock.resolve_pin(image_pin)
                    .ok_or_else(|| SecurityError::T3AdmissionFailed(
                        format!("sandbox.image_pin '{}' not present in t3-image.lock", image_pin)
                    ))?;
            } else {
                lock.default_attestation()
                    .map_err(|e| SecurityError::T3AdmissionFailed(e.to_string()))?;
            }
        } else if effective.0 > SandboxTier::T3.0 {
            return Err(SecurityError::SandboxTierUnsupported(effective));
        }

        // Fail-closed: T1 enforcement not yet implemented.
        if effective == SandboxTier::T1 {
            return Err(SecurityError::SandboxTierUnsupported(effective));
        }

        // Resolve resource caps (2-way strictest-of manifest vs operator).
        let operator_caps = inner
            .operator_policy
            .resource_cap_floor
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let resolved_caps = resolve_caps(caps, &operator_caps);

        let declared_scopes = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.scopes.clone())
            .unwrap_or_default();

        // Build OutputShapePredicate from manifest (Story 2.1 AC3).
        let predicate = output_shape.map(OutputShapePredicate::from);

        // Journal the Load transition with effective tier (monotonic clock).
        journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: spirit_id.into(),
            effective_sandbox_tier: Some(effective),
        }));

        if effective == SandboxTier::T3 {
            journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
                lifecycle_event: LifecycleEvent::SandboxApplied,
                spirit_id: spirit_id.into(),
                effective_sandbox_tier: Some(effective),
            }));
        }

        Ok(SandboxSpec {
            tier: effective,
            resolved_caps,
            declared_scopes,
            spirit_id: spirit_id.into(),
            output_shape_predicate: predicate,
        })
    }

    /// Emit a sandbox-block audit event (non-blocking).
    pub fn emit_sandbox_block(
        &self,
        sender: &cap_audit::Sender,
        spirit_pid: u32,
        attempted_syscall: &str,
        sandbox_tier: SandboxTier,
    ) {
        let event = CapAuditEvent::SandboxBlock {
            spirit_pid,
            attempted_syscall: attempted_syscall.into(),
            sandbox_tier,
        };
        // Non-blocking; drop if channel saturated (ADR-030).
        if sender.try_send(event).is_err() {
            cap_audit::record_drop();
        }
    }
}

impl Default for SecurityManagerAdapter {
    fn default() -> Self {
        Self::new(Arc::new(PolicyTable::new()))
    }
}

impl SecurityManagerPort for SecurityManagerAdapter {
    fn sandbox_tier_floor(&self, _spirit_id: &str) -> SandboxTier {
        // v0.1-β: return the global operator floor.
        let inner = self.policy.inner().load_full();
        inner.operator_policy.global_sandbox_floor
    }

    fn effective_sandbox_tier(&self, spirit_pid: u32) -> Option<SandboxTier> {
        let inner = self.policy.inner().load_full();
        let trust_tier = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.trust_tier)
            .unwrap_or(TrustTier::Verified);
        Some(
            self.policy
                .effective_sandbox_tier(spirit_pid, trust_tier, &inner),
        )
    }

    fn approval_class(&self, _capability: &str) -> maos_domain::invariants::i4::ApprovalDecision {
        // v0.1-β placeholder: auto-approve.
        maos_domain::invariants::i4::ApprovalDecision {
            actor: "kernel".into(),
            target: "spirit".into(),
            capability: _capability.into(),
            intent: "default".into(),
            decision: true,
            reasoning: Some("v0.1-β placeholder approval".into()),
        }
    }
}
