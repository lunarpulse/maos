#![forbid(unsafe_code)]

//! The observable token contract used by CRL propagation: issue succeeds
//! before revocation and verification returns the typed `Revoked` error after.

use std::sync::Arc;

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::CryptoProvider;
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_tokens::{
    CapTokensShardRing, Ed25519SigningKey, RevokeReason,
};
use maos_kernel_core::security::RingCryptoProvider;

#[test]
fn issued_token_is_denied_by_the_real_post_revocation_verify_path() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    let (audit, _receiver) = cap_audit::channel();
    let ring = CapTokensShardRing::new(crypto, Ed25519SigningKey::new([0u8; 32]), 17, audit);
    let token = ring
        .issue(
            41,
            Scope::FsRead {
                subtree: "/tmp/revocation-denial".into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .unwrap();
    assert!(ring.verify(&token, [0u8; 32], SandboxTier::T2).is_ok());
    ring.revoke(token.token_id, RevokeReason::Operator).unwrap();
    assert!(matches!(
        ring.verify(&token, [0u8; 32], SandboxTier::T2),
        Err(maos_domain::ports::capability::CapError::Revoked)
    ));
}
