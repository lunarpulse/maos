use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
use maos_registry::admission::{verify_publisher_sig, verify_publisher_sig_with};
use maos_spirit_abi::compliance::{
    CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

struct RejectingProvider;
impl CryptoProvider for RejectingProvider {
    fn verify_signature(&self, _: &[u8], _: &[u8], _: &[u8]) -> Result<(), CryptoError> {
        Err(CryptoError::SignatureInvalid)
    }
    fn seal_for_export(
        &self,
        _: &[u8],
        _: &[u8],
        _: &[u8],
        _: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::OperationFailed("not used"))
    }
    fn sign_capability_token(&self, _: &[u8], _: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::OperationFailed("not used"))
    }
}

#[test]
fn selected_publisher_verification_matches_ring_and_rejects_tampering() {
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let manifest = b"[spirit]\nname = \"publisher-test\"\nversion = \"1.0.0\"\n".to_vec();
    let artifact = b"signed bytes".to_vec();
    let mut hash = Sha256::new();
    hash.update((manifest.len() as u64).to_le_bytes());
    hash.update(&manifest);
    hash.update((artifact.len() as u64).to_le_bytes());
    hash.update(&artifact);
    let signature: [u8; 64] = key.sign(&hash.finalize()).as_ref().try_into().unwrap();
    let fp = ExecutionContextFingerprint {
        manifest_hash: Sha256::digest(&manifest).into(),
        spirit_version: "1.0.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    let envelope = maos_compliance::builder::build_signed_envelope(
        maos_compliance::builder::encode_claim_bytes(&fp, None),
        &key,
        public,
    );
    let pkg = SignedPackage::new(
        SpiritId::from("publisher-test"),
        "1.0.0".into(),
        manifest,
        artifact,
        signature,
        public,
        envelope,
    );
    let ring = maos_kernel_core::security::crypto::RingCryptoProvider;
    assert!(verify_publisher_sig(&pkg));
    assert!(verify_publisher_sig_with(&pkg, &ring));
    assert!(!verify_publisher_sig_with(&pkg, &RejectingProvider));
    for mutation in 0..3 {
        let mut altered = pkg.clone();
        match mutation {
            0 => altered.manifest_toml[0] ^= 1,
            1 => altered.artifact_bytes[0] ^= 1,
            _ => altered.signature[0] ^= 1,
        }
        assert!(!verify_publisher_sig(&altered));
        assert!(!verify_publisher_sig_with(&altered, &ring));
    }
}
