#![forbid(unsafe_code)]

//! Capability policy — read-mostly copy-on-write policy table.
//!
//! Per architecture §4.6: "Read-mostly; copy-on-write for policy updates."

pub mod decision;

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use decision::{ApprovalClass, Capability, Intent, PolicyDecision, TrustTier};

/// Operator policy configuration.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(reason = "operator policy config; part of PolicyTableInner CoW snapshot")]
pub struct OperatorPolicyConfig {
    /// Per-Spirit sandbox tier floor from operator policy.
    pub spirit_tier_floor: HashMap<u32, SandboxTier>,
    /// Global sandbox floor applied to ALL Spirits regardless of manifest.
    pub global_sandbox_floor: SandboxTier,
    /// Per-capability approval class overrides.
    pub per_capability_approval: HashMap<String, decision::ApprovalClass>,
    /// Resource cap floor from operator policy (Story 1b.3).
    pub resource_cap_floor: Option<crate::security::manifest::ResourceCaps>,
}

/// Manifest capability scope per Spirit.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(reason = "manifest scope; part of PolicyTableInner CoW snapshot")]
pub struct ManifestCapabilityScope {
    pub scopes: Vec<Scope>,
    pub declared_tier: SandboxTier,
    pub trust_tier: decision::TrustTier,
}

/// Inner policy table — the actual data.
#[derive(Debug, Clone, Default)]
#[maos_attrs::i9_exempt(
    reason = "inner policy data behind ArcSwap; structural-state caching per I9"
)]
pub struct PolicyTableInner {
    pub manifest_scopes: HashMap<u32, ManifestCapabilityScope>,
    pub trust_tier_floor: HashMap<decision::TrustTier, SandboxTier>,
    pub operator_policy: OperatorPolicyConfig,
    /// Story 3.2 — per-Spirit runtime posture state. Updated atomically
    /// via CoW swap (same shape as manifest_scopes).
    pub spirit_postures: HashMap<u32, crate::security::posture::PostureState>,
}

/// Policy table — read-mostly copy-on-write.
///
/// Readers take a single atomic load (free); writers swap the entire
/// `Arc<PolicyTableInner>` at runtime.
#[derive(Debug)]
#[maos_attrs::i9_exempt(reason = "operator policy table; structural-state caching per I9")]
pub struct PolicyTable {
    inner: Arc<ArcSwap<PolicyTableInner>>,
}

impl PolicyTable {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(PolicyTableInner::default())),
        }
    }

    /// Access the inner ArcSwap (for SecurityManagerAdapter admission).
    pub fn inner(&self) -> &Arc<ArcSwap<PolicyTableInner>> {
        &self.inner
    }

    pub fn evaluate(
        &self,
        spirit_pid: u32,
        capability: &Capability,
        intent: &Intent,
    ) -> PolicyDecision {
        // Story 4.3 — FR56 self-telemetry always-allow rule.
        // Self-telemetry is a positive always-allow — the Spirit reads
        // its own data without per-read operator admission.  The rule is
        // enumerable so operators can audit the policy table and see the
        // self-telemetry cap-class explicitly (NFR-Aud-1).
        if matches!(
            capability.scope,
            maos_domain::invariants::i1::Scope::SelfTelemetryRead
        ) || matches!(intent, Intent::SelfTelemetryRead)
        {
            return PolicyDecision::Allow;
        }

        let inner = self.inner.load_full();

        // Fail-closed: unknown Spirits are denied.
        let manifest = match inner.manifest_scopes.get(&spirit_pid) {
            Some(m) => m,
            None => return PolicyDecision::Deny,
        };

        // Scope-match check: the requested capability must be in the
        // Spirit's declared manifest scopes.
        let scope_match = manifest
            .scopes
            .iter()
            .any(|s| std::mem::discriminant(s) == std::mem::discriminant(&capability.scope));
        if !scope_match {
            return PolicyDecision::Deny;
        }

        // Effective sandbox tier: strictest of manifest, trust-tier-floor, operator.
        let effective = self.effective_sandbox_tier(spirit_pid, manifest.trust_tier, &inner);
        if effective.0 > 3 {
            return PolicyDecision::Deny;
        }

        // Story 4.4 — default approval-class rules for new capabilities.
        // LogRecall / LogFetch → AutonomousWithHalt (normal admission under halt protocol).
        // DistillateWrite → Assistive (prompt-on-write per architecture §F.4).
        use maos_domain::invariants::i1::Scope;
        match &capability.scope {
            Scope::DistillateWrite => {
                return PolicyDecision::RequireApproval {
                    class: ApprovalClass::Interactive,
                };
            }
            Scope::LogRecall | Scope::LogFetch => {
                // Normal admission — halt protocol is enforced at the caller layer.
                // Falls through to Allow below.
            }
            Scope::SkillAuthorSelf => {
                // Story 7.4 — FR39/FR57 `skill.author.self`. This arm is reached
                // ONLY after the scope-match above confirmed the Spirit DECLARES
                // `skill.author.self` in its manifest scopes — i.e. UNLIKE
                // `SelfTelemetryRead` (the top-of-function always-allow), this
                // capability is NOT always-allow: a Spirit that does not declare
                // it is denied at the scope-match gate. The capability grants ONLY
                // the write-to-queue (a Spirit may author a skill into the PENDING
                // operator-admission queue); it does NOT auto-admit the skill.
                // Activation still requires the FR39 operator-admission path
                // (`SkillAdmissionQueue::approve`). Falls through to Allow below
                // (the write is permitted; the queue gates activation).
            }
            _ => {}
        }

        // Approval-class lookup from operator policy (by scope discriminant
        // and by intent discriminant).
        let scope_key = format!("{:?}", std::mem::discriminant(&capability.scope));
        if let Some(class) = inner
            .operator_policy
            .per_capability_approval
            .get(&scope_key)
        {
            return PolicyDecision::RequireApproval { class: *class };
        }
        let intent_key = format!("{:?}", std::mem::discriminant(intent));
        if let Some(class) = inner
            .operator_policy
            .per_capability_approval
            .get(&intent_key)
        {
            return PolicyDecision::RequireApproval { class: *class };
        }

        PolicyDecision::Allow
    }

    /// Compute the effective sandbox tier: strictest of (manifest,
    /// trust-tier-floor for the Spirit's trust tier, operator-policy).
    pub fn effective_sandbox_tier(
        &self,
        spirit_pid: u32,
        trust_tier: decision::TrustTier,
        inner: &PolicyTableInner,
    ) -> SandboxTier {
        let manifest = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.declared_tier)
            .unwrap_or(SandboxTier::DEFAULT_FLOOR);
        let trust = inner
            .trust_tier_floor
            .get(&trust_tier)
            .copied()
            .unwrap_or(SandboxTier::DEFAULT_FLOOR);
        let operator = inner
            .operator_policy
            .spirit_tier_floor
            .get(&spirit_pid)
            .copied()
            .unwrap_or(inner.operator_policy.global_sandbox_floor);
        strictest_of(manifest, trust, operator)
    }

    /// Update the policy table atomically (CoW swap).
    pub fn update(&self, new_policy: PolicyTableInner) {
        self.inner.store(Arc::new(new_policy));
    }

    /// Atomically shift a Spirit's posture under the ceiling constraint
    /// (Story 3.2, AC4).
    pub fn shift_posture(
        &self,
        spirit_pid: u32,
        new_posture: crate::security::manifest::Posture,
    ) -> Result<[u8; 32], crate::security::posture::PostureError> {
        use crate::security::manifest::Posture;
        use crate::security::posture::PostureError;

        let inner = self.inner.load_full();

        // Reject non-runtime postures
        if new_posture == Posture::Autonomous {
            return Err(PostureError::NonRuntimePosture(Posture::Autonomous));
        }

        // Validate spirit exists
        let state = inner
            .spirit_postures
            .get(&spirit_pid)
            .ok_or(PostureError::UnknownSpirit(spirit_pid))?;

        // Validate ceiling
        if new_posture > state.allowed_max {
            return Err(PostureError::AboveCeiling {
                requested: new_posture,
                allowed: state.allowed_max,
            });
        }

        // CoW swap
        let mut new_inner = (*inner).clone();
        if let Some(s) = new_inner.spirit_postures.get_mut(&spirit_pid) {
            s.current = new_posture;
        }
        let new_hash = new_inner
            .spirit_postures
            .get(&spirit_pid)
            .map(|s| s.posture_hash())
            .unwrap_or([0u8; 32]);
        self.update(new_inner);

        Ok(new_hash)
    }

    /// Posture-aware evaluation (Story 3.2, AC5).
    pub fn evaluate_with_posture(
        &self,
        spirit_pid: u32,
        base_class: maos_domain::notification::ApprovalClass,
    ) -> PolicyDecision {
        let inner = self.inner.load_full();

        // Step 1: Fail-closed — unknown spirit
        let state = match inner.spirit_postures.get(&spirit_pid) {
            Some(s) => s,
            None => return PolicyDecision::Deny,
        };

        // Step 2: ControlPlane always requires approval
        if base_class == maos_domain::notification::ApprovalClass::ControlPlane {
            return PolicyDecision::RequireApproval {
                class: domain_class_to_decision(base_class),
            };
        }

        // Step 3: Posture × class matrix lookup
        let requires =
            crate::security::posture::posture_requires_approval(state.current, base_class);
        let matrix_decision = if requires {
            PolicyDecision::RequireApproval {
                class: domain_class_to_decision(base_class),
            }
        } else {
            PolicyDecision::Allow
        };

        // Step 4: Operator policy overrides take precedence (existing evaluate semantics)
        let scope_key = format!("{:?}", std::mem::discriminant(&base_class));
        if let Some(class) = inner
            .operator_policy
            .per_capability_approval
            .get(&scope_key)
        {
            return PolicyDecision::RequireApproval { class: *class };
        }

        matrix_decision
    }
}

/// Strictest of three sandbox tiers.
pub fn strictest_of(a: SandboxTier, b: SandboxTier, c: SandboxTier) -> SandboxTier {
    SandboxTier(a.0.max(b.0).max(c.0))
}

/// Map domain ApprovalClass → kernel decision::ApprovalClass.
fn domain_class_to_decision(
    c: maos_domain::notification::ApprovalClass,
) -> decision::ApprovalClass {
    match c {
        maos_domain::notification::ApprovalClass::ReadonlyScoped => {
            decision::ApprovalClass::ReadonlyScoped
        }
        maos_domain::notification::ApprovalClass::ReadonlySearch => {
            decision::ApprovalClass::ReadonlySearch
        }
        maos_domain::notification::ApprovalClass::Mutating => decision::ApprovalClass::Mutating,
        maos_domain::notification::ApprovalClass::ExecCapable => {
            decision::ApprovalClass::ExecCapable
        }
        maos_domain::notification::ApprovalClass::ControlPlane => {
            decision::ApprovalClass::ControlPlane
        }
        maos_domain::notification::ApprovalClass::Interactive => {
            decision::ApprovalClass::Interactive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn strictest_of_three() {
        assert_eq!(
            strictest_of(SandboxTier(0), SandboxTier(1), SandboxTier(2)),
            SandboxTier(2)
        );
        assert_eq!(
            strictest_of(SandboxTier(2), SandboxTier(0), SandboxTier(0)),
            SandboxTier(2)
        );
    }

    #[test]
    fn policy_table_update_is_atomic() {
        let table = PolicyTable::new();
        let mut new_inner = PolicyTableInner::default();
        new_inner
            .operator_policy
            .spirit_tier_floor
            .insert(7, SandboxTier(2));
        table.update(new_inner);
        let loaded = table.inner.load_full();
        assert_eq!(
            loaded.operator_policy.spirit_tier_floor.get(&7),
            Some(&SandboxTier(2))
        );
    }

    #[test]
    fn strictest_of_floor_forces_t0_to_t2_for_public_untrusted() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            42,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::PublicUntrusted,
            },
        );
        inner
            .trust_tier_floor
            .insert(TrustTier::PublicUntrusted, SandboxTier(2));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::PublicUntrusted, &loaded);
        assert_eq!(
            effective,
            SandboxTier(2),
            "trust tier floor must force T0→T2 for PublicUntrusted"
        );
    }

    #[test]
    fn operator_floor_can_force_above_manifest_and_trust_tier() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            42,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        inner
            .trust_tier_floor
            .insert(TrustTier::Verified, SandboxTier(0));
        inner
            .operator_policy
            .spirit_tier_floor
            .insert(42, SandboxTier(3));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::Verified, &loaded);
        assert_eq!(
            effective,
            SandboxTier(3),
            "operator floor must override manifest+trust"
        );
    }

    #[test]
    fn concurrent_readers_never_block_writer() {
        let table = Arc::new(PolicyTable::new());
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            1,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        table.update(inner);

        let writer_table = table.clone();
        let writer = thread::spawn(move || {
            for i in 0..100 {
                let mut new_inner = PolicyTableInner::default();
                new_inner.manifest_scopes.insert(
                    1,
                    ManifestCapabilityScope {
                        scopes: vec![Scope::FsRead {
                            subtree: format!("/tmp/{i}"),
                        }],
                        declared_tier: SandboxTier(0),
                        trust_tier: TrustTier::Verified,
                    },
                );
                writer_table.update(new_inner);
            }
        });

        let reader_table = table.clone();
        let reader = thread::spawn(move || {
            for _ in 0..10_000 {
                let inner = reader_table.inner.load_full();
                assert!(inner.manifest_scopes.contains_key(&1));
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn skill_author_self_requires_manifest_declaration_not_always_allow() {
        use maos_domain::invariants::i1::Scope;
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        // Spirit 7 DECLARES skill.author.self; Spirit 8 declares only FsRead.
        inner.manifest_scopes.insert(
            7,
            ManifestCapabilityScope {
                scopes: vec![Scope::SkillAuthorSelf],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        inner.manifest_scopes.insert(
            8,
            ManifestCapabilityScope {
                scopes: vec![Scope::FsRead {
                    subtree: "/tmp".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        table.update(inner);

        let cap = Capability {
            scope: Scope::SkillAuthorSelf,
        };
        let intent = Intent::FsRead {
            subtree: "n/a".into(),
        };

        // Declaring Spirit → Allow: the write-to-queue is authorized.
        assert_eq!(table.evaluate(7, &cap, &intent), PolicyDecision::Allow);
        // Non-declaring Spirit → Deny: scope-match fails.
        assert_eq!(table.evaluate(8, &cap, &intent), PolicyDecision::Deny);
        // Unknown Spirit → Deny. CRITICAL contrast with SelfTelemetryRead, which
        // is always-allow EVEN for an unknown Spirit: SkillAuthorSelf is NOT
        // always-allow — it is fail-closed and requires manifest declaration.
        assert_eq!(table.evaluate(999, &cap, &intent), PolicyDecision::Deny);
        let self_tel = Capability {
            scope: Scope::SelfTelemetryRead,
        };
        assert_eq!(
            table.evaluate(999, &self_tel, &intent),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn evaluate_with_posture_returns_deny_for_unknown_spirit() {
        use crate::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
        use crate::security::posture::PostureState;

        let table = PolicyTable::new();
        let decision =
            table.evaluate_with_posture(999, maos_domain::notification::ApprovalClass::Mutating);
        assert!(matches!(decision, PolicyDecision::Deny));
    }

    #[test]
    fn evaluate_with_posture_operator_override_takes_precedence() {
        use crate::security::manifest::{EpistemicAction, EpistemicPolicySection, Posture};
        use crate::security::posture::PostureState;

        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.spirit_postures.insert(
            0,
            PostureState {
                current: Posture::AutonomousWithHalt,
                allowed_max: Posture::AutonomousWithHalt,
                epistemic_policy: EpistemicPolicySection {
                    rules: vec![],
                    default_action: EpistemicAction::VerbalizeOnly,
                },
            },
        );
        // AutonomousWithHalt + Mutating = Allow per matrix, but operator
        // override forces RequireApproval
        inner.operator_policy.per_capability_approval.insert(
            format!(
                "{:?}",
                std::mem::discriminant(&maos_domain::notification::ApprovalClass::Mutating)
            ),
            decision::ApprovalClass::Mutating,
        );
        table.update(inner);

        let decision =
            table.evaluate_with_posture(0, maos_domain::notification::ApprovalClass::Mutating);
        assert!(
            matches!(decision, PolicyDecision::RequireApproval { .. }),
            "operator override must take precedence over matrix Allow"
        );
    }
}
