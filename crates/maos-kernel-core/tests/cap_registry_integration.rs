//! Integration test for the Capability Registry.
//!
//! Covers issue/verify/revoke/expire/TOCTOU/cross-Spirit isolation.

use std::sync::Arc;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

use maos_kernel_core::capability::{
    CapabilityRegistryAdapter, CapabilityRegistryPort, cap_audit, cap_policy, cap_quota, cap_tokens,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::CryptoProvider;

use crate::common::MockCryptoProvider;

fn make_adapter() -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = cap_tokens::Ed25519SigningKey::new([0u8; 32]);
    let policy = cap_policy::PolicyTable::new();
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
    let quota = cap_quota::CapQuotaTracker::new();
    let working_memory = Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new());
    let telemetry = Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default());
    CapabilityRegistryAdapter::new(crypto, signing_key, 0xDEAD_BEEF, Arc::new(policy), audit_tx, quota, working_memory, telemetry)
}

#[test]
fn integration_issue_verify_revoke() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap();
    assert_eq!(token.spirit_pid, 7);
    assert!(adapter.verify(&token, posture, SandboxTier(2)).is_ok());

    adapter.revoke(token.token_id).unwrap();
    assert!(matches!(adapter.verify(&token, posture, SandboxTier(2)), Err(CapError::Revoked)));
}

#[test]
fn integration_toctu_rejects_stale_posture() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture_v1 = [1u8; 32];
    let posture_v2 = [2u8; 32];

    let token = adapter.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture_v1, IntentClass::Standard).unwrap();
    assert!(adapter.verify(&token, posture_v1, SandboxTier(2)).is_ok());
    assert!(matches!(adapter.verify(&token, posture_v2, SandboxTier(2)), Err(CapError::PostureMismatch)));
}

#[test]
fn integration_cross_spirit_replay_rejected() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap();
    let mut tampered = token.clone();
    tampered.spirit_pid = 8;
    assert!(matches!(adapter.verify(&tampered, posture, SandboxTier(2)), Err(CapError::SpiritIdMismatch)));
}

#[test]
fn integration_high_privilege_ttl_capped() {
    cap_tokens::init_monotonic_base();
    let adapter = make_adapter();
    let posture = [1u8; 32];

    let token = adapter.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 3600, posture, IntentClass::HighPrivilege).unwrap();
    assert!(token.expiry_ns <= cap_tokens::monotonic_now_ns() + 60 * 1_000_000_000 + 1_000_000);
}

mod common {
    use maos_domain::ports::crypto::{CryptoError, CryptoProvider};

    pub struct MockCryptoProvider;

    impl CryptoProvider for MockCryptoProvider {
        fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
            Ok(())
        }
        fn seal_for_export(&self, _k: &[u8], _n: &[u8], _a: &[u8], p: &[u8]) -> Result<Vec<u8>, CryptoError> {
            Ok(p.to_vec())
        }
        fn sign_capability_token(&self, _sk: &[u8], token_bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
            let mut sig = [0u8; 64];
            for (i, b) in token_bytes.iter().enumerate() {
                sig[i % 64] ^= *b;
            }
            Ok(sig.to_vec())
        }
    }
}
