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

use decision::{Capability, Intent, PolicyDecision, TrustTier};

/// Operator policy configuration.
#[derive(Debug, Clone, Default)]
pub struct OperatorPolicyConfig {
    /// Per-Spirit sandbox tier floor from operator policy.
    pub spirit_tier_floor: HashMap<u32, SandboxTier>,
    /// Global sandbox floor applied to ALL Spirits regardless of manifest.
    pub global_sandbox_floor: SandboxTier,
    /// Per-capability approval class overrides.
    pub per_capability_approval: HashMap<String, decision::ApprovalClass>,
}

/// Manifest capability scope per Spirit.
#[derive(Debug, Clone, Default)]
pub struct ManifestCapabilityScope {
    pub scopes: Vec<Scope>,
    pub declared_tier: SandboxTier,
    pub trust_tier: decision::TrustTier,
}

/// Inner policy table — the actual data.
#[derive(Debug, Clone, Default)]
pub struct PolicyTableInner {
    pub manifest_scopes: HashMap<u32, ManifestCapabilityScope>,
    pub trust_tier_floor: HashMap<decision::TrustTier, SandboxTier>,
    pub operator_policy: OperatorPolicyConfig,
}

/// Policy table — read-mostly copy-on-write.
///
/// Readers take a single atomic load (free); writers swap the entire
/// `Arc<PolicyTableInner>` at runtime.
#[derive(Debug)]
pub struct PolicyTable {
    inner: Arc<ArcSwap<PolicyTableInner>>,
}

impl PolicyTable {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(PolicyTableInner::default())),
        }
    }

    pub fn evaluate(
        &self,
        spirit_pid: u32,
        capability: &Capability,
        intent: &Intent,
    ) -> PolicyDecision {
        let inner = self.inner.load_full();

        // Fail-closed: unknown Spirits are denied.
        let manifest = match inner.manifest_scopes.get(&spirit_pid) {
            Some(m) => m,
            None => return PolicyDecision::Deny,
        };

        // Scope-match check: the requested capability must be in the
        // Spirit's declared manifest scopes.
        let scope_match = manifest.scopes.iter().any(|s| {
            std::mem::discriminant(s) == std::mem::discriminant(&capability.scope)
        });
        if !scope_match {
            return PolicyDecision::Deny;
        }

        // Effective sandbox tier: strictest of manifest, trust-tier-floor, operator.
        let effective = self.effective_sandbox_tier(spirit_pid, manifest.trust_tier, &inner);
        if effective.0 >= 3 {
            return PolicyDecision::Deny;
        }

        // Approval-class lookup from operator policy (by scope discriminant
        // and by intent discriminant).
        let scope_key = format!("{:?}", std::mem::discriminant(&capability.scope));
        if let Some(class) = inner.operator_policy.per_capability_approval.get(&scope_key) {
            return PolicyDecision::RequireApproval { class: *class };
        }
        let intent_key = format!("{:?}", std::mem::discriminant(intent));
        if let Some(class) = inner.operator_policy.per_capability_approval.get(&intent_key) {
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
            .unwrap_or(SandboxTier(0));
        let trust = inner
            .trust_tier_floor
            .get(&trust_tier)
            .copied()
            .unwrap_or(SandboxTier(0));
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
}

/// Strictest of three sandbox tiers.
pub fn strictest_of(a: SandboxTier, b: SandboxTier, c: SandboxTier) -> SandboxTier {
    SandboxTier(a.0.max(b.0).max(c.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn strictest_of_three() {
        assert_eq!(strictest_of(SandboxTier(0), SandboxTier(1), SandboxTier(2)), SandboxTier(2));
        assert_eq!(strictest_of(SandboxTier(2), SandboxTier(0), SandboxTier(0)), SandboxTier(2));
    }

    #[test]
    fn policy_table_update_is_atomic() {
        let table = PolicyTable::new();
        let mut new_inner = PolicyTableInner::default();
        new_inner.operator_policy.spirit_tier_floor.insert(7, SandboxTier(2));
        table.update(new_inner);
        let loaded = table.inner.load_full();
        assert_eq!(loaded.operator_policy.spirit_tier_floor.get(&7), Some(&SandboxTier(2)));
    }

    #[test]
    fn strictest_of_floor_forces_t0_to_t2_for_public_untrusted() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(42, ManifestCapabilityScope {
            scopes: vec![Scope::FsRead { subtree: "/tmp".into() }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::PublicUntrusted,
        });
        inner.trust_tier_floor.insert(TrustTier::PublicUntrusted, SandboxTier(2));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::PublicUntrusted, &loaded);
        assert_eq!(effective, SandboxTier(2), "trust tier floor must force T0→T2 for PublicUntrusted");
    }

    #[test]
    fn operator_floor_can_force_above_manifest_and_trust_tier() {
        let table = PolicyTable::new();
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(42, ManifestCapabilityScope {
            scopes: vec![Scope::FsRead { subtree: "/tmp".into() }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        });
        inner.trust_tier_floor.insert(TrustTier::Verified, SandboxTier(0));
        inner.operator_policy.spirit_tier_floor.insert(42, SandboxTier(3));
        table.update(inner);

        let loaded = table.inner.load_full();
        let effective = table.effective_sandbox_tier(42, TrustTier::Verified, &loaded);
        assert_eq!(effective, SandboxTier(3), "operator floor must override manifest+trust");
    }

    #[test]
    fn concurrent_readers_never_block_writer() {
        let table = Arc::new(PolicyTable::new());
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(1, ManifestCapabilityScope {
            scopes: vec![Scope::FsRead { subtree: "/tmp".into() }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        });
        table.update(inner);

        let writer_table = table.clone();
        let writer = thread::spawn(move || {
            for i in 0..100 {
                let mut new_inner = PolicyTableInner::default();
                new_inner.manifest_scopes.insert(1, ManifestCapabilityScope {
                    scopes: vec![Scope::FsRead { subtree: format!("/tmp/{i}") }],
                    declared_tier: SandboxTier(0),
                    trust_tier: TrustTier::Verified,
                });
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
}
