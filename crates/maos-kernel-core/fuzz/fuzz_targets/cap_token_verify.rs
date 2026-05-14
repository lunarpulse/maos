#![no_main]

use libfuzzer_sys::fuzz_target;
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_tokens::{CapTokensShardRing, Ed25519SigningKey};
use std::sync::Arc;

struct FuzzCryptoProvider;

impl CryptoProvider for FuzzCryptoProvider {
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

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    token_id_bytes: [u8; 16],
    spirit_pid: u32,
    expiry_ns: u64,
    signature_bytes: [u8; 64],
    posture_hash: [u8; 32],
}

fuzz_target!(|input: FuzzInput| {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let crypto: Arc<dyn CryptoProvider> = Arc::new(FuzzCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let (tx, _rx) = cap_audit::channel();
    let ring = CapTokensShardRing::new(crypto, signing_key, 0xDEAD_BEEF, tx);

    let token = CapabilityToken::new(
        TokenId(input.token_id_bytes),
        input.spirit_pid,
        input.expiry_ns,
        input.signature_bytes,
    );

    let _ = ring.verify(&token, input.posture_hash, SandboxTier(2));
});
