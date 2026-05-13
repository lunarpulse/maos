//! CryptoProvider port trait per architecture §8.6 + FR48 + NFR-Sec-15.
//!
//! The kernel's cryptographic operations — signature verification,
//! sealed-export encryption, capability-token signing — route through
//! this trait so FIPS 140-3-validated modules (NFR-Sec-15 v1.0),
//! hardware-backed crypto (HSM / TPM / TEE), or post-quantum
//! implementations can be substituted at the `maos-bin` composition
//! root without recompiling any Spirit binary.
//!
//! # v0.1-α scope
//!
//! Trait shape only. The default `RingCryptoProvider` adapter lives in
//! `maos-kernel-core::security::crypto`. v0.1-α has zero kernel call
//! sites that invoke these methods (Story 1b.1 lands audit-spine
//! `verify_signature` on journal entries; Story 1b.2 lands `cap_tokens`
//! `sign_capability_token`; Story 7.3 lands ComplianceClaim envelope
//! `verify_signature` at admission time).
//!
//! # Sync trait method signatures
//!
//! Per ADR-010's binding-v0.1 gate "domain core compiles without async
//! runtime", every method below is `fn` (not `async fn`). Crypto
//! primitives in `ring`/`rustls` are sync-by-construction (CPU-bound,
//! no I/O). Adapter implementations that need async wrappers (e.g.,
//! HSM RPC) wrap the sync trait method behind a `spawn_blocking` at
//! the call site — but that is a future-story concern, NOT a v0.1-α
//! port-trait commitment.
//!
//! # Operations and their FR48 mapping
//!
//! | Operation | FR48 surface | Default impl primitive |
//! |---|---|---|
//! | `verify_signature` | "kernel signature verification" | `ring::signature::UnparsedPublicKey::verify` (Ed25519) |
//! | `seal_for_export` | "sealed-export encryption" | `ring::aead::SealingKey::seal_in_place_append_tag` (AES-256-GCM) |
//! | `sign_capability_token` | "capability-token signing" | `ring::signature::Ed25519KeyPair::sign` |

use thiserror::Error;

/// Crypto-provider port — pluggable kernel cryptographic operations.
///
/// Implemented by `maos_kernel_core::security::crypto::RingCryptoProvider`
/// at v0.1-α; v1.0+ swaps in FIPS-validated, hardware-backed, or
/// post-quantum providers per NFR-Sec-15.
///
/// **Trait-object safety:** all methods take `&self`, return `Result`
/// or owned `Vec<u8>`, and use no generics — the trait IS object-safe.
/// Composition root holds `Arc<dyn CryptoProvider>` (verified by
/// `let _: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);`
/// at `maos-bin/src/main.rs`).
pub trait CryptoProvider: Send + Sync {
    /// Class: data-movement
    ///
    /// Verify an Ed25519 signature over `message` using `public_key`.
    /// Returns `Ok(())` iff the signature is valid; `Err(CryptoError::SignatureInvalid)`
    /// on any failure (bad signature, wrong public key length, malformed key bytes).
    /// At v0.1-α the underlying `ring` primitive returns a single `Unspecified`
    /// error for all failure modes, so the adapter maps everything to
    /// `SignatureInvalid` (coarse-grained per the v0.1-α error taxonomy).
    /// Per-key-length pre-validation and finer-grained `MalformedKey` discrimination
    /// are deferred to Story 7.3's refined error taxonomy.
    /// The default `RingCryptoProvider` accepts raw 32-byte Ed25519 public keys
    /// per `ring::signature::ED25519`.
    fn verify_signature(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), CryptoError>;

    /// Class: data-movement
    ///
    /// Encrypt `plaintext` under `sealing_key` using AES-256-GCM,
    /// returning the ciphertext-with-tag in a new `Vec<u8>`. The
    /// `nonce` MUST be 12 bytes and MUST be unique per (key, message)
    /// pair (per AES-GCM contract — reuse is a confidentiality break).
    /// `aad` is additional authenticated data — bound into the tag
    /// but not encrypted (e.g., the ComplianceClaim envelope header).
    fn seal_for_export(
        &self,
        sealing_key: &[u8],
        nonce: &[u8],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;

    /// Class: data-movement
    ///
    /// Sign `token_bytes` with `signing_key` (an Ed25519 keypair seed,
    /// 32 bytes) producing a 64-byte Ed25519 signature. Used by
    /// `cap_tokens::issue` (Story 1b.2) to sign the (Spirit-PID +
    /// boot-nonce + expiry) tuple per ADR-023.
    fn sign_capability_token(
        &self,
        signing_key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;
}

/// Crypto-provider error taxonomy.
///
/// Adapter implementations map their primitive errors (e.g.,
/// `ring::error::Unspecified`) into one of these variants. The
/// taxonomy is deliberately coarse at v0.1-α; refinements per
/// distributor (FIPS module error codes, HSM hardware faults)
/// land in Story 7.3's ComplianceClaim verify path.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    /// Signature did not match the message under the public key.
    #[error("signature verification failed")]
    SignatureInvalid,

    /// Key bytes did not parse as a valid key of the expected algorithm.
    #[error("malformed key: {0}")]
    MalformedKey(&'static str),

    /// Nonce length, AEAD tag length, or input-length mismatch.
    #[error("crypto operation failed: {0}")]
    OperationFailed(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    // The maos-domain crate carries only TRAIT-SHAPE tests; the real
    // round-trip tests for the default adapter live in
    // `crates/maos-kernel-core/src/security/crypto.rs#tests`.

    #[test]
    fn crypto_error_distinguishes_variants() {
        assert_ne!(
            CryptoError::SignatureInvalid,
            CryptoError::MalformedKey("bad-len")
        );
        assert_ne!(
            CryptoError::MalformedKey("a"),
            CryptoError::MalformedKey("b")
        );
    }

    #[test]
    fn crypto_provider_is_object_safe() {
        // If this compiles, the trait is dyn-compatible per
        // RFC 2027 object safety rules — required for `Arc<dyn CryptoProvider>`
        // in `maos-bin/src/main.rs`.
        fn _accepts_dyn(_: &dyn CryptoProvider) {}
    }
}
