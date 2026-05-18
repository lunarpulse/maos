#![forbid(unsafe_code)]

//! NFR-Rel-9 v0.3-beta scaffold — revoke propagation latency <=5s p99
//! under 1000 token revocations (Story 3.4, AC5).
//!
//! This test measures the full revoke → verify → Err(Revoked) path through
//! CapTokensShardRing. Story 5.4 extends to 10^4 concurrent validations
//! for the production gate.
//!
//! v0.3-beta scaffold note: the full NFR-Rel-9 10^4 corpus lands at Story 5.4.

use std::time::Instant;

use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoError;
use maos_domain::ports::CryptoProvider;
use maos_kernel_core::capability::cap_tokens::{CapTokensShardRing, Ed25519SigningKey, RevokeReason};
use maos_kernel_core::capability::cap_audit;

use std::sync::Arc;

const REVOKE_P99_BUDGET_US: u64 = 5_000_000;

struct TestCrypto;
impl CryptoProvider for TestCrypto {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> { Ok(()) }
    fn seal_for_export(&self, _key: &[u8], _nonce: &[u8], _aad: &[u8], _pt: &[u8]) -> Result<Vec<u8>, CryptoError> { Ok(vec![]) }
    fn sign_capability_token(&self, _key: &[u8], _msg: &[u8]) -> Result<Vec<u8>, CryptoError> { Ok(vec![0u8; 64]) }
}

fn setup_ring() -> CapTokensShardRing {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(TestCrypto);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let (audit_tx, _audit_rx) = cap_audit::channel();
    CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, audit_tx)
}

#[test]
fn nfr_rel_9_1000_token_revoke_latency_v03_scaffold() {
    const N: usize = 1000;
    let ring = setup_ring();

    let tokens: Vec<_> = (0..N)
        .map(|i| {
            ring.issue(
                i as u32,
                Scope::FsRead { subtree: format!("/tmp/{i}").into() },
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
            .unwrap()
        })
        .collect();

    let mut revoke_latencies_us = Vec::with_capacity(N);

    for token in &tokens {
        let t0 = Instant::now();
        ring.revoke(token.token_id, RevokeReason::Operator).unwrap();
        let result = ring.verify(token, [0u8; 32], SandboxTier(2));
        assert_eq!(result, Err(maos_domain::ports::capability::CapError::Revoked));
        revoke_latencies_us.push(t0.elapsed().as_micros() as u64);
    }

    revoke_latencies_us.sort();
    let p99 = revoke_latencies_us[(N * 99) / 100];

    assert!(
        p99 < REVOKE_P99_BUDGET_US,
        "revoke P99 = {p99}us exceeds 5s budget ({REVOKE_P99_BUDGET_US}us)"
    );
}
