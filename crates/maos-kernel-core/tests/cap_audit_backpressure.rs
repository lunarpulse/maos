//! Audit backpressure load test.
//!
//! Asserts the hot path uses `try_send` + `AuditDrop` counter increment,
//! never `send().await`, under channel saturation. Spawns a real writer
//! task with an in-memory TransparencyLog to exercise the full audit path.

use std::sync::Arc;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use std::time::Instant;

use maos_kernel_core::capability::{
    CapabilityRegistryAdapter, CapabilityRegistryPort, WorkingMemoryStore, cap_audit, cap_policy, cap_quota, cap_tokens,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(
        &self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self, _k: &[u8], _n: &[u8], _a: &[u8], p: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(
        &self, _sk: &[u8], token_bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn make_adapter_with_writer() -> (CapabilityRegistryAdapter, tokio::task::JoinHandle<()>) {
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
    let (audit_tx, audit_rx) = cap_audit::channel();
    let quota = cap_quota::CapQuotaTracker::new();

    let tlog = Arc::new(maos_kernel_core::iac::TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
    let writer = cap_audit::CapAuditWriter::spawn(audit_rx, tlog);

    let telemetry = Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default());
    let adapter = CapabilityRegistryAdapter::new(crypto, signing_key, 0xDEAD_BEEF, Arc::new(policy), audit_tx, quota, Arc::new(WorkingMemoryStore::new()), telemetry);
    (adapter, writer)
}

#[tokio::test]
async fn hot_path_never_blocks_under_audit_saturation() {
    cap_tokens::init_monotonic_base();
    let (adapter, _writer) = make_adapter_with_writer();
    let posture = [1u8; 32];

    let start = Instant::now();
    for i in 0..100_000 {
        let token = adapter.issue(
            7,
            Scope::FsRead { subtree: "/tmp".into() },
            60,
            posture,
            IntentClass::Standard,
        ).unwrap();
        adapter.verify(&token, posture, SandboxTier(2)).unwrap();
        if i % 1000 == 0 {
            adapter.revoke(token.token_id).unwrap();
        }
    }
    let elapsed = start.elapsed();

    // Hot path must stay under 5s wall-clock for 100K ops
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "hot path too slow: {:?}",
        elapsed
    );

    // Audit drop counter must be > 0 — the bounded channel (8192 depth)
    // cannot absorb 200K+ events without drops under this load.
    let drops = cap_audit::audit_drop_count();
    assert!(
        drops > 0,
        "expected audit drops under 100K load (channel depth = 8192), got 0 drops — hot path may be blocking"
    );
}
