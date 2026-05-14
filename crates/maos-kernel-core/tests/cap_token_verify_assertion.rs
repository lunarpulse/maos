//! Strict P99 latency assertions for cap-token verify.
//!
//! NFR-Perf-3: P99 < 100µs overall (verify + audit-enqueue).
//! ADR-030: P99 < 5µs hot-path shard read only.

use std::sync::Arc;
use std::time::Instant;

use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, CapTokensShardRing, Ed25519SigningKey};
use maos_kernel_core::capability::cap_audit;
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};

struct MockCryptoProvider;

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

fn make_test_ring() -> CapTokensShardRing {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let (audit_tx, _audit_rx) = cap_audit::channel();
    CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, audit_tx)
}

fn make_test_token(ring: &CapTokensShardRing) -> CapabilityToken {
    let posture = [1u8; 32];
    ring.issue(7, Scope::FsRead { subtree: "/tmp".into() }, 60, posture, IntentClass::Standard).unwrap()
}

#[test]
fn cap_token_verify_p99_under_5us_hot_path() {
    init_monotonic_base();
    let ring = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    let mut samples = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let start = Instant::now();
        ring.verify(&token, posture, SandboxTier(2)).unwrap();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    let p99_ns = samples[9_899];
    eprintln!("cap_token_verify P99 = {}ns (ADR-030 budget: 5000ns)", p99_ns);
    assert!(p99_ns < 5_000, "ADR-030 binding broken: cap_token_verify P99 = {}ns, budget = 5000ns", p99_ns);
}

#[test]
fn cap_token_verify_p99_under_100us_overall() {
    init_monotonic_base();
    let ring = make_test_ring();
    let token = make_test_token(&ring);
    let posture = [1u8; 32];
    let mut samples = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let start = Instant::now();
        ring.verify(&token, posture, SandboxTier(2)).unwrap();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort_unstable();
    let p99_us = samples[9_899] / 1_000;
    assert!(p99_us < 100, "NFR-Perf-3 binding broken: P99 = {}µs, budget = 100µs", p99_us);
}
