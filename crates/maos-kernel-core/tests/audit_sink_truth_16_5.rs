use std::sync::Arc;

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter,
    CapabilityRegistryPort, WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }

    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(plaintext.to_vec())
    }

    fn sign_capability_token(
        &self,
        _key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut signature = [0u8; 64];
        for (index, byte) in token_bytes.iter().enumerate() {
            signature[index % signature.len()] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

fn adapter_with_audit_sender(audit: cap_audit::Sender) -> CapabilityRegistryAdapter {
    let policy = cap_policy::PolicyTable::new();
    let mut inner = cap_policy::PolicyTableInner::default();
    inner.manifest_scopes.insert(
        7,
        cap_policy::ManifestCapabilityScope {
            scopes: vec![Scope::LoomWrite],
            declared_tier: SandboxTier(0),
            trust_tier: cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(inner);

    CapabilityRegistryAdapter::new(
        Arc::new(MockCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0u8; 32]),
        0x16_05,
        Arc::new(policy),
        audit,
        cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    )
}

#[test]
fn closed_audit_sink_refuses_invocation_and_latches_degraded_health() {
    cap_tokens::init_monotonic_base();
    let (audit, receiver) = cap_audit::channel();
    let adapter = adapter_with_audit_sender(audit);
    let token = adapter
        .issue(
            7,
            Scope::LoomWrite,
            60,
            [1u8; 32],
            IntentClass::HighPrivilege,
        )
        .expect("issue while the audit receiver is healthy");
    let before = cap_audit::audit_health_snapshot();
    drop(receiver);

    let error = adapter
        .record_invocation(&token, "collective.write".into(), br#"{"key":"k"}"#)
        .expect_err("a closed audit sink must refuse a mediated invocation");

    assert_eq!(error, CapError::AuditSinkUnavailable);
    let after = cap_audit::audit_health_snapshot();
    assert_eq!(after.total_drops, before.total_drops + 1);
    assert_eq!(
        after.count(cap_audit::AuditDropSite::Invocation),
        before.count(cap_audit::AuditDropSite::Invocation) + 1
    );
    assert!(
        after.degraded,
        "writer death is process-lifetime degradation"
    );
}

#[test]
fn healthy_audit_sink_accepts_invocation_without_counting_a_drop() {
    cap_tokens::init_monotonic_base();
    let (audit, mut receiver) = cap_audit::channel();
    let adapter = adapter_with_audit_sender(audit);
    let token = adapter
        .issue(
            7,
            Scope::LoomWrite,
            60,
            [2u8; 32],
            IntentClass::HighPrivilege,
        )
        .expect("issue");
    let _ = receiver.try_recv().expect("issue event");
    let before = cap_audit::audit_health_snapshot();

    adapter
        .record_invocation(&token, "collective.write".into(), br#"{"key":"k"}"#)
        .expect("healthy sink");

    assert!(matches!(
        receiver.try_recv(),
        Ok(cap_audit::CapAuditEvent::Invocation { .. })
    ));
    assert_eq!(cap_audit::audit_health_snapshot(), before);
}
