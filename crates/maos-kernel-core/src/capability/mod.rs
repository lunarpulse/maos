#![forbid(unsafe_code)]

//! Capability Registry — supervised service per §4.6.
//!
//! Decomposed per ADR-030 into four sub-modules (hot path / policy /
//! audit / quota). At v0.1-α the four sub-module shells exist from
//! Story 1a.1; this story adds the port-trait re-export and the
//! `CapabilityRegistryAdapter` composite with runtime bodies.

pub mod cap_tokens;
pub mod cap_policy;
pub mod cap_audit;
pub mod cap_quota;
pub mod working_memory;

pub use maos_domain::ports::CapabilityRegistryPort;
pub use maos_domain::ports::capability::TokenIssuer;
pub use working_memory::{WorkingMemorySlot, SetScalarError};
pub use working_memory::store::WorkingMemoryStore;

use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::CryptoProvider;

use cap_audit::Sender;
use cap_policy::PolicyTable;
use cap_quota::CapQuotaTracker;
use cap_tokens::CapTokensShardRing;
use crate::telemetry::TelemetryStreamAdapter;
use maos_domain::ports::TelemetryStreamPort;

fn scope_to_intent(scope: &Scope) -> cap_policy::decision::Intent {
    match scope {
        Scope::FsRead { subtree } => cap_policy::decision::Intent::FsRead { subtree: subtree.clone() },
        Scope::FsWrite { subtree } => cap_policy::decision::Intent::FsWrite { subtree: subtree.clone() },
        Scope::NetHttps { domain } => cap_policy::decision::Intent::NetHttps { domain: domain.clone() },
        Scope::ProcExec { binary } => cap_policy::decision::Intent::ProcExec { binary: binary.clone() },
        Scope::SubSpiritSpawn { class } => cap_policy::decision::Intent::SubSpiritSpawn { class: class.clone() },
        Scope::ProviderInfer { provider } => cap_policy::decision::Intent::ProviderInfer { provider: provider.clone() },
        Scope::IacSend { peer_class } => cap_policy::decision::Intent::IacSend { peer_class: peer_class.clone() },
        Scope::MemRead { scope: s } => cap_policy::decision::Intent::MemRead { scope: s.clone() },
        Scope::MemWrite { scope: s } => cap_policy::decision::Intent::MemWrite { scope: s.clone() },
        Scope::SelfTelemetryRead => cap_policy::decision::Intent::SelfTelemetryRead,
        Scope::LogRecall => cap_policy::decision::Intent::LogRecall,
        Scope::LogFetch => cap_policy::decision::Intent::LogFetch,
        Scope::DistillateWrite => cap_policy::decision::Intent::DistillateWrite,
        _ => {
            panic!(
                "scope_to_intent: unmapped Scope variant {:?} — add an explicit arm before calling this function",
                scope
            )
        }
    }
}

/// Composite adapter — holds the four ADR-030 sub-modules and the
/// `CryptoProvider` trait object. Story 4.2 adds the `working_memory`
/// sub-module as a 5th field (ADR-022 tagged-scalar slot).
#[maos_attrs::i9_exempt(reason = "capability registry composite; holds exempt sub-module Arcs")]
pub struct CapabilityRegistryAdapter {
    tokens: Arc<CapTokensShardRing>,
    policy: Arc<PolicyTable>,
    audit: Sender,
    quota: Arc<CapQuotaTracker>,
    /// Story 4.2 — per-Spirit tagged-scalar slot store.
    working_memory: Arc<WorkingMemoryStore>,
    /// Story 4.2 — telemetry stream for scalar.tap broadcast.
    telemetry: Arc<TelemetryStreamAdapter>,
}

impl std::fmt::Debug for CapabilityRegistryAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapabilityRegistryAdapter")
            .field("tokens", &"Arc<CapTokensShardRing>")
            .field("policy", &"Arc<PolicyTable>")
            .field("audit", &"Sender")
            .field("quota", &"Arc<CapQuotaTracker>")
            .field("working_memory", &"Arc<WorkingMemoryStore>")
            .finish()
    }
}

impl CapabilityRegistryAdapter {
    /// Construct the composite adapter. Called from the composition root.
    /// Story 4.2 adds the `working_memory` parameter as a 5th field.
    pub fn new(
        crypto: Arc<dyn CryptoProvider>,
        signing_key: cap_tokens::Ed25519SigningKey,
        boot_nonce: u64,
        policy: Arc<PolicyTable>,
        audit: Sender,
        quota: CapQuotaTracker,
        working_memory: Arc<WorkingMemoryStore>,
        telemetry: Arc<TelemetryStreamAdapter>,
    ) -> Self {
        let tokens = Arc::new(CapTokensShardRing::new(
            crypto,
            signing_key,
            boot_nonce,
            audit.clone(),
        ));
        Self {
            tokens,
            policy,
            audit,
            quota: Arc::new(quota),
            working_memory,
            telemetry,
        }
    }

    /// Issue a capability token with full mediation: quota → policy → tokens.
    pub fn issue_with_mediation(
        &self,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
        posture_hash: [u8; 32],
        intent_class: IntentClass,
    ) -> Result<CapabilityToken, CapError> {
        // 1. Quota check
        let budget = 1_000_000u64;
        let _quota_state = self
            .quota
            .check_and_increment(spirit_pid, 1, budget)?;

        // 2. Policy check: derive Intent from the actual scope
        let cap = cap_policy::decision::Capability {
            scope: scope.clone(),
        };
        let intent = scope_to_intent(&scope);
        let decision = self.policy.evaluate(spirit_pid, &cap, &intent);
        match decision {
            cap_policy::decision::PolicyDecision::Allow => {}
            cap_policy::decision::PolicyDecision::Deny => {
                return Err(CapError::PolicyDenied);
            }
            cap_policy::decision::PolicyDecision::RequireApproval { .. } => {
                return Err(CapError::PolicyDenied);
            }
        }

        // 3. Token issue
        self.tokens
            .issue(spirit_pid, scope, ttl_secs, posture_hash, intent_class)
    }

    /// Look up the scope for a token ID without full verification.
    pub fn get_token_scope(&self, token_id: &TokenId) -> Option<Scope> {
        self.tokens.get_scope(token_id)
    }

    /// Story 4.2 — write a tagged scalar to the per-Spirit slot store
    /// and publish a `ScalarTapEvent` to the telemetry stream.
    pub fn set_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<maos_domain::invariants::i7::ScalarTapEvent, SetScalarError> {
        let event = self.working_memory.set_scalar(spirit_pid, spirit_id, tag, value, derived_from)?;
        let topic = maos_domain::invariants::i7::TelemetryTopic::new(&format!("scalar.tap.{}", tag));
        self.telemetry.publish_event(&topic, event.clone());
        Ok(event)
    }

    /// Expose the working-memory store for read-back in integration tests.
    pub fn working_memory(&self) -> &WorkingMemoryStore {
        &self.working_memory
    }

    /// Verify and audit a capability token.
    pub fn verify_and_audit(
        &self,
        token: &CapabilityToken,
        posture_hash: [u8; 32],
        sandbox: SandboxTier,
    ) -> Result<(), CapError> {
        let result = self.tokens.verify(token, posture_hash, sandbox);
        let outcome = cap_audit::VerifyOutcome::from_result(&result);

        if self.audit.try_send(cap_audit::CapAuditEvent::Verify {
            token_id: token.token_id,
            spirit_pid: token.spirit_pid,
            outcome,
        }).is_err() {
            cap_audit::record_drop();
        }

        result
    }
}

impl TokenIssuer for CapabilityRegistryAdapter {
    fn issue_with_mediation(
        &self,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
        posture_hash: [u8; 32],
        intent_class: IntentClass,
    ) -> Result<CapabilityToken, CapError> {
        CapabilityRegistryAdapter::issue_with_mediation(self, spirit_pid, scope, ttl_secs, posture_hash, intent_class)
    }

    fn get_token_scope(&self, token_id: &TokenId) -> Option<Scope> {
        CapabilityRegistryAdapter::get_token_scope(self, token_id)
    }
}

impl CapabilityRegistryPort for CapabilityRegistryAdapter {
    fn on_value_above(&self, value: f64, threshold: f64) -> bool {
        value > threshold
    }

    fn on_value_below(&self, value: f64, threshold: f64) -> bool {
        value < threshold
    }

    fn on_value_within(&self, value: f64, lower: f64, upper: f64) -> bool {
        lower <= value && value <= upper
    }

    fn on_value_outside(&self, value: f64, lower: f64, upper: f64) -> bool {
        value < lower || value > upper
    }

    fn issue(
        &self,
        spirit_pid: u32,
        scope: Scope,
        ttl_secs: u32,
        posture_snapshot_hash: [u8; 32],
        intent_class: IntentClass,
    ) -> Result<CapabilityToken, CapError> {
        self.issue_with_mediation(
            spirit_pid,
            scope,
            ttl_secs,
            posture_snapshot_hash,
            intent_class,
        )
    }

    fn verify(
        &self,
        token: &CapabilityToken,
        current_posture_hash: [u8; 32],
        current_sandbox: SandboxTier,
    ) -> Result<(), CapError> {
        self.verify_and_audit(token, current_posture_hash, current_sandbox)
    }

    fn revoke(&self, token_id: TokenId) -> Result<(), CapError> {
        self.tokens.revoke(token_id, cap_tokens::RevokeReason::Operator)
    }

    fn record_invocation(
        &self,
        token: &CapabilityToken,
        intent: String,
        payload: &[u8],
    ) -> Result<(), CapError> {
        if self.audit.try_send(cap_audit::CapAuditEvent::Invocation {
            token_id: token.token_id,
            spirit_pid: token.spirit_pid,
            capability_token_bytes: token.token_id.0.to_vec(),
            intent,
            payload: payload.to_vec(),
        }).is_err() {
            cap_audit::record_drop();
        }
        Ok(())
    }
}

impl CapabilityRegistryAdapter {
    /// Story 5.1 — revoke all capability tokens for a given spirit_pid.
    /// Called by `SpiritSchedulerAdapter::unload` during graceful teardown.
    pub fn revoke_all_for_pid(&self, spirit_pid: u32) -> Result<usize, CapError> {
        Ok(self.tokens.revoke_all(spirit_pid))
    }

    #[cfg(any(test, feature = "test-introspection"))]
    pub fn list_active_tokens(&self) -> Vec<TokenId> {
        self.tokens.list_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::crypto::tests::MockCryptoProvider;

    fn test_adapter() -> CapabilityRegistryAdapter {
        let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
        let signing_key = cap_tokens::Ed25519SigningKey::new([0u8; 32]);
        let policy = PolicyTable::new();
        // Register Spirit 7 in the policy table so evaluate() doesn't deny it
        {
            let mut inner = cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(7, cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::FsRead { subtree: "/tmp".into() }],
                declared_tier: SandboxTier(0),
                trust_tier: cap_policy::decision::TrustTier::Verified,
            });
            policy.update(inner);
        }
        let (audit_tx, _audit_rx) = cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let working_memory = Arc::new(WorkingMemoryStore::new());
        let telemetry = Arc::new(TelemetryStreamAdapter::default());
        CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xDEAD_BEEF,
            Arc::new(policy),
            audit_tx,
            quota,
            working_memory,
            telemetry,
        )
    }

    #[test]
    fn adapter_issue_and_verify() {
        cap_tokens::init_monotonic_base();
        let adapter = test_adapter();
        let posture = [1u8; 32];
        let token = adapter
            .issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard)
            .unwrap();
        assert_eq!(token.spirit_pid, 7);
        assert!(adapter.verify(&token, posture, SandboxTier(2)).is_ok());
    }

    #[test]
    fn adapter_on_value_predicates() {
        let adapter = test_adapter();
        assert!(adapter.on_value_above(5.0, 3.0));
        assert!(!adapter.on_value_above(2.0, 3.0));
        assert!(adapter.on_value_below(2.0, 3.0));
        assert!(adapter.on_value_within(2.0, 1.0, 3.0));
        assert!(!adapter.on_value_outside(2.0, 1.0, 3.0));
    }
}
