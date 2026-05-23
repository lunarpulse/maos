//! Criterion bench for cap-token verify hot path.
//!
//! ADR-030 ship gate: P99 < 5µs.

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

fn make_test_token(ring: &CapTokensShardRing) -> CapabilityToken {
    let posture = [1u8; 32];
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
}

fn bench_cap_token_verify(c: &mut Criterion) {
    init_monotonic_base();
    let ring = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    let mut group = c.benchmark_group("cap_token_verify");
    group.sample_size(10_000);
    group.bench_function("verify_hot_path", |b| {
        b.iter(|| {
            ring.verify(&token, posture, SandboxTier(2)).unwrap();
        });
    });
    group.finish();
}

criterion_group!(benches, bench_cap_token_verify);
criterion_main!(benches);
