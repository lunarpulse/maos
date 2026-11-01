//! Criterion bench for cap-token verify hot path.
//!
//! ADR-030 hot-path claim (P99 < 5µs) and NFR-Perf-3 (P99 < 100µs): rows of
//! `xtask/perf-budgets.toml`, judged on the stable runner (Tier W) and, as
//! instruction/allocation counts, on every pull request (Tier C).

#[path = "../../maos-bench/src/budgets.rs"]
mod budgets;

use criterion::{criterion_group, Criterion};
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

/// Story 20-7 D-20-7-04 A: the stable runner's own P99 of `verify` carries the
/// ADR-030 5 µs hot-path claim and NFR-Perf-3's 100 µs, both rows of
/// `xtask/perf-budgets.toml`.
fn budget_leg() -> bool {
    init_monotonic_base();
    let ring = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    for _ in 0..5_000 {
        ring.verify(&token, posture, SandboxTier(2)).unwrap();
    }
    let mut samples: Vec<u64> = (0..100_000)
        .map(|_| {
            let start = std::time::Instant::now();
            ring.verify(&token, posture, SandboxTier(2)).unwrap();
            start.elapsed().as_nanos() as u64
        })
        .collect();
    samples.sort_unstable();
    let p99_us = samples[samples.len() * 99 / 100 - 1] as f64 / 1_000.0;
    let mut within = budgets::report("adr-030-cap-token-verify-p99", p99_us);
    within &= budgets::report("nfr-perf-3-cap-token-verify-p99", p99_us);
    within
}

fn main() -> std::process::ExitCode {
    benches();
    Criterion::default().configure_from_args().final_summary();
    budgets::finish(budget_leg())
}
