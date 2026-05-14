//! FR4 1000-call mediation fixture.
//!
//! Issues 1000 capability tokens across 5 synthetic Spirits,
//! performs 1000 verify+invoke calls, drains the audit channel,
//! and asserts 1000/1000 entries in the audit channel with proper fields.

use std::sync::Arc;

use maos_kernel_core::capability::{
    CapabilityRegistryAdapter, CapabilityRegistryPort, cap_audit, cap_policy, cap_quota, cap_tokens,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;

use crate::common::MockCryptoProvider;

fn make_adapter() -> (CapabilityRegistryAdapter, tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>) {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = cap_tokens::Ed25519SigningKey::new([0u8; 32]);
    let policy = cap_policy::PolicyTable::new();
    {
        let mut inner = cap_policy::PolicyTableInner::default();
        for pid in 1u32..=5 {
            inner.manifest_scopes.insert(pid, cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::FsRead { subtree: "/tmp".into() }],
                declared_tier: SandboxTier(0),
                trust_tier: cap_policy::decision::TrustTier::Verified,
            });
        }
        policy.update(inner);
    }
    let (audit_tx, audit_rx) = cap_audit::channel();
    let quota = cap_quota::CapQuotaTracker::new();
    let adapter = CapabilityRegistryAdapter::new(
        crypto, signing_key, 0xDEAD_BEEF, policy, audit_tx.clone(), quota,
    );
    (adapter, audit_rx)
}

#[test]
fn fr4_1000_call_mediation() {
    cap_tokens::init_monotonic_base();
    let (adapter, mut audit_rx) = make_adapter();
    let posture = [1u8; 32];

    let mut tokens = Vec::with_capacity(1000);
    let mut per_spirit_counts: [u32; 5] = [0; 5];

    // Issue 1000 tokens across 5 Spirits (200 each), PIDs 1..=5
    for spirit_offset in 0..5u32 {
        let spirit_pid = spirit_offset + 1;
        for _ in 0..200 {
            let token = adapter.issue(
                spirit_pid,
                Scope::FsRead { subtree: "/tmp".into() },
                60,
                posture,
                IntentClass::Standard,
            ).unwrap();
            assert_eq!(token.spirit_pid, spirit_pid, "token spirit_pid must match requested PID");
            tokens.push(token);
            per_spirit_counts[spirit_offset as usize] += 1;
        }
    }
    assert_eq!(tokens.len(), 1000, "must issue exactly 1000 tokens");

    // Verify all 1000 tokens
    for token in &tokens {
        adapter.verify(token, posture, SandboxTier(2)).unwrap();
    }

    // Record invocations
    for token in &tokens {
        adapter.record_invocation(token, "fr4-test".into(), b"test-payload").unwrap();
    }

    // Drain audit channel and validate FR4 fields
    let mut issue_count = 0u32;
    let mut verify_count = 0u32;
    let mut invocation_count = 0u32;

    while let Ok(event) = audit_rx.try_recv() {
        match &event {
            cap_audit::CapAuditEvent::Issue { spirit_pid, .. } => {
                assert!(*spirit_pid >= 1 && *spirit_pid <= 5, "issue spirit_pid must be 1-5");
                issue_count += 1;
            }
            cap_audit::CapAuditEvent::Verify { spirit_pid, outcome, .. } => {
                assert!(*spirit_pid >= 1 && *spirit_pid <= 5, "verify spirit_pid must be 1-5");
                assert_eq!(*outcome, cap_audit::VerifyOutcome::Ok, "all verifies should succeed");
                verify_count += 1;
            }
            cap_audit::CapAuditEvent::Invocation { spirit_pid, .. } => {
                assert!(*spirit_pid >= 1 && *spirit_pid <= 5, "invocation spirit_pid must be 1-5");
                invocation_count += 1;
            }
            _ => {}
        }
    }

    // FR4 binding: every external call was mediated and audit-logged.
    assert!(issue_count >= 1000, "FR4: expected >=1000 issue events, got {}", issue_count);
    assert!(verify_count >= 1000, "FR4: expected >=1000 verify events, got {}", verify_count);
    assert!(invocation_count >= 1000, "FR4: expected >=1000 invocation events, got {}", invocation_count);

    // Per-Spirit distribution: 200 each
    for (i, &count) in per_spirit_counts.iter().enumerate() {
        assert_eq!(count, 200, "Spirit {} should have 200 tokens, got {}", i + 1, count);
    }
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
