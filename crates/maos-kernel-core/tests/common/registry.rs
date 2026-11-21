#![allow(dead_code)]

//! Capability Registry harness shared by the registry integration tests and the I1/I7 ledger proofs.

use std::sync::Arc;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

pub struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(
        &self,
        _sk: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

/// A registry whose policy table declares `manifests` (pid, declared scopes) and whose audit
/// events are drained by a background thread.
pub fn adapter(manifests: &[(u32, Vec<Scope>)]) -> CapabilityRegistryAdapter {
    adapter_with_telemetry(manifests, Arc::new(TelemetryStreamAdapter::default()))
}

/// [`adapter`] publishing its scalar writes to `telemetry`.
pub fn adapter_with_telemetry(
    manifests: &[(u32, Vec<Scope>)],
    telemetry: Arc<TelemetryStreamAdapter>,
) -> CapabilityRegistryAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = cap_tokens::Ed25519SigningKey::new([0u8; 32]);
    let policy = cap_policy::PolicyTable::new();
    {
        let mut inner = cap_policy::PolicyTableInner::default();
        for (pid, scopes) in manifests {
            inner.manifest_scopes.insert(
                *pid,
                cap_policy::ManifestCapabilityScope {
                    scopes: scopes.clone(),
                    declared_tier: SandboxTier(0),
                    trust_tier: cap_policy::decision::TrustTier::Verified,
                },
            );
        }
        policy.update(inner);
    }
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xDEAD_BEEF,
        Arc::new(policy),
        audit_tx,
        cap_quota::CapQuotaTracker::new(),
        Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
        telemetry,
    )
}
