#![forbid(unsafe_code)]

//! Default crypto provider adapter — `ring`/`rustls`-backed.
//!
//! Implements `maos_domain::ports::crypto::CryptoProvider` for the
//! default `RingCryptoProvider` unit struct. Per §8.6 + FR48 + NFR-Sec-15:
//! "the seam exists; specific FIPS modules are downstream distributor
//! concern." This file IS that seam.
//!
//! # Why the adapter is a unit struct
//!
//! Per the I9 structural-state lint, persistent key material cannot
//! live in struct fields outside the three sanctioned holders
//! (journal/, iac/transparency_log.rs, capability/cap_tokens/). At
//! v0.1-α key material is passed through method-call arguments by the
//! caller (Story 1b.2's `cap_tokens::issue` will load the signing key
//! from `cap_tokens`-local state and pass `&[u8]` slices into
//! `sign_capability_token`). The adapter holds NO state.
//!
//! # FR48 swap-pattern verification
//!
//! `maos-bin/src/main.rs` constructs `Arc::new(RingCryptoProvider)` and
//! binds it to a local `Arc<dyn CryptoProvider>`. Swapping to a v1.0+
//! FIPS-validated provider is one line in `main.rs`:
//! `Arc::new(FipsCryptoProvider::from_module_id("…"))`. No Spirit
//! binary, no `cap_tokens`-side code, no audit-spine code recompiles.
//! Verified at v0.1-α by the `swap_pattern_compiles` test below.

use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use ring::{aead, signature};

/// Default crypto provider — `ring`-backed Ed25519 + AES-256-GCM.
///
/// Zero-size; key material is caller-supplied per the I9 discipline.
/// Implements `CryptoProvider` from `maos_domain::ports::crypto`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RingCryptoProvider;

impl CryptoProvider for RingCryptoProvider {
    fn verify_signature(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature_bytes: &[u8],
    ) -> Result<(), CryptoError> {
        let pk = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
        pk.verify(message, signature_bytes)
            .map_err(|_| CryptoError::SignatureInvalid)
    }

    fn seal_for_export(
        &self,
        sealing_key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if sealing_key.len() != 32 {
            return Err(CryptoError::MalformedKey("AES-256-GCM key must be 32 bytes"));
        }
        if nonce.len() != 12 {
            return Err(CryptoError::OperationFailed("AES-GCM nonce must be 12 bytes"));
        }
        let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, sealing_key)
            .map_err(|_| CryptoError::MalformedKey("AES-256-GCM key rejected by ring"))?;
        let key = aead::LessSafeKey::new(unbound);
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
            .map_err(|_| CryptoError::OperationFailed("nonce construction failed"))?;
        let aad = aead::Aad::from(aad);
        let mut in_out = plaintext.to_vec();
        if key.seal_in_place_append_tag(nonce, aad, &mut in_out).is_err() {
            // Zeroize buffer on failure — plaintext may still be present.
            in_out.iter_mut().for_each(|b| *b = 0);
            return Err(CryptoError::OperationFailed("AES-GCM seal failed"));
        }
        Ok(in_out)
    }

    fn sign_capability_token(
        &self,
        signing_key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // Ed25519 keypair seed is 32 bytes (raw); ring derives the public
        // key from the seed. Story 1b.2 owns the key-generation/loading
        // path; at v0.1-α this method is exercised only by tests below.
        let keypair = signature::Ed25519KeyPair::from_seed_unchecked(signing_key)
            .map_err(|_| CryptoError::MalformedKey("Ed25519 seed must be 32 bytes"))?;
        Ok(keypair.sign(token_bytes).as_ref().to_vec())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use ring::signature::KeyPair;

    /// Mock crypto provider for hexagonal port-test patterns.
    ///
    /// Lives behind `#[cfg(test)]` so it never reaches a release build.
    /// Story 1b.2's `cap_tokens` unit tests will use this to verify the
    /// trait-object substitution pattern without a real `ring` keypair.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MockCryptoProvider;

    impl CryptoProvider for MockCryptoProvider {
        fn verify_signature(
            &self,
            _public_key: &[u8],
            _message: &[u8],
            signature: &[u8],
        ) -> Result<(), CryptoError> {
            // Mock policy: signature of all-zero bytes verifies; everything else fails.
            if signature.iter().all(|&b| b == 0) {
                Ok(())
            } else {
                Err(CryptoError::SignatureInvalid)
            }
        }
        fn seal_for_export(
            &self,
            _key: &[u8],
            _nonce: &[u8],
            _aad: &[u8],
            plaintext: &[u8],
        ) -> Result<Vec<u8>, CryptoError> {
            // Mock policy: pass-through (NOT a real seal — for trait-shape
            // verification only).
            Ok(plaintext.to_vec())
        }
        fn sign_capability_token(
            &self,
            _signing_key: &[u8],
            token_bytes: &[u8],
        ) -> Result<Vec<u8>, CryptoError> {
            // Mock policy: deterministic 64-byte signature.
            let mut sig = [0u8; 64];
            for (i, b) in token_bytes.iter().enumerate() {
                sig[i % 64] ^= *b;
            }
            Ok(sig.to_vec())
        }
    }

    fn known_ed25519_keypair() -> (Vec<u8>, Vec<u8>) {
        // 32-byte all-zero seed → ring derives a deterministic keypair.
        // Used for repeatable signature tests; NOT a real production key.
        let seed = vec![0u8; 32];
        let keypair = signature::Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let public = keypair.public_key().as_ref().to_vec();
        (seed, public)
    }

    #[test]
    fn ring_sign_verify_round_trip() {
        let provider = RingCryptoProvider;
        let (seed, public) = known_ed25519_keypair();
        let message = b"v0.1-alpha test message";
        let sig = provider.sign_capability_token(&seed, message).unwrap();
        assert_eq!(sig.len(), 64, "Ed25519 signature must be 64 bytes");
        assert!(provider.verify_signature(&public, message, &sig).is_ok());
    }

    #[test]
    fn ring_verify_rejects_tampered_message() {
        let provider = RingCryptoProvider;
        let (seed, public) = known_ed25519_keypair();
        let message = b"original message";
        let sig = provider.sign_capability_token(&seed, message).unwrap();
        let tampered = b"tampered message";
        assert_eq!(
            provider.verify_signature(&public, tampered, &sig),
            Err(CryptoError::SignatureInvalid)
        );
    }

    #[test]
    fn ring_verify_rejects_malformed_public_key() {
        let provider = RingCryptoProvider;
        let bad_pk = vec![0u8; 16]; // wrong length — Ed25519 PKs are 32 bytes
        let result = provider.verify_signature(&bad_pk, b"msg", &vec![0u8; 64]);
        // ring returns Unspecified for both bad-key and bad-sig; we map
        // to SignatureInvalid (coarse-grained at v0.1-α per CryptoError taxonomy).
        assert_eq!(result, Err(CryptoError::SignatureInvalid));
    }

    #[test]
    fn ring_seal_produces_gcm_tag_appended_ciphertext() {
        let provider = RingCryptoProvider;
        let key = [1u8; 32];
        let nonce = [2u8; 12];
        let aad = b"compliance-claim-header";
        let plaintext = b"sealed audit bundle bytes";
        let ciphertext = provider.seal_for_export(&key, &nonce, aad, plaintext).unwrap();
        assert_eq!(
            ciphertext.len(),
            plaintext.len() + 16,
            "AES-256-GCM appends a 16-byte tag"
        );
        // We do not unseal in this test — the `unseal_for_import`
        // operation is a Story 7.3 ComplianceClaim verify concern.
        // This test confirms the seal primitive runs and produces a
        // tag-appended ciphertext; round-trip verification is covered
        // by the symmetric `ring::aead::open_in_place` invariant ring
        // already tests upstream.
    }

    #[test]
    fn ring_seal_rejects_wrong_key_length() {
        let provider = RingCryptoProvider;
        let short_key = [1u8; 16];
        let nonce = [2u8; 12];
        assert!(matches!(
            provider.seal_for_export(&short_key, &nonce, b"", b"data"),
            Err(CryptoError::MalformedKey(_))
        ));
    }

    #[test]
    fn mock_provider_satisfies_trait_for_swap_pattern() {
        // FR48 swap-pattern verification: a non-default provider can be
        // substituted at the trait-object level without changing any
        // call-site code.
        fn accepts_any_provider(p: &dyn CryptoProvider) -> Result<(), CryptoError> {
            p.verify_signature(b"", b"any", &vec![0u8; 64])
        }
        let default = RingCryptoProvider;
        let mock = MockCryptoProvider;
        // Both compile against the same function signature — that IS the proof.
        let _ = accepts_any_provider(&default);
        let _ = accepts_any_provider(&mock);
    }
}
