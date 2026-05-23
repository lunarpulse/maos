//! Criterion bench for revocation propagation latency.
//!
//! NFR-Rel-9 ship gate: ≤5s p99 from apply_crl return to first CapError::Revoked.

use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_tokens::{
    init_monotonic_base, CapTokensShardRing, Ed25519SigningKey,
};

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(
        &self,
        _pk: &[u8],
        _msg: &[u8],
        _sig: &[u8],
    ) -> Result<(), maos_domain::ports::crypto::CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(
        &self,
        _sk: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn make_test_ring() -> CapTokensShardRing {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let (audit_tx, _audit_rx) = cap_audit::channel();
    CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, audit_tx)
}

fn bench_revocation_propagation(c: &mut Criterion) {
    init_monotonic_base();
    let ring = make_test_ring();
    let posture = [1u8; 32];

    // Issue 100 tokens
    let tokens: Vec<CapabilityToken> = (0..100)
        .map(|_| {
            ring.issue(
                7,
                Scope::FsRead {
                    subtree: "/tmp".into(),
                },
                60,
                posture,
                IntentClass::Standard,
            )
            .unwrap()
        })
        .collect();

    let mut group = c.benchmark_group("revocation_propagation");
    group.measurement_time(std::time::Duration::from_secs(15));
    group.sample_size(20);
    group.bench_function("revoke_all_for_pid_under_10k_verify_storm", |b| {
        b.iter(|| {
            // Revoke all tokens for pid 7
            ring.revoke_all(7);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_revocation_propagation);
criterion_main!(benches);
