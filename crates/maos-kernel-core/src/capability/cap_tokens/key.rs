#![forbid(unsafe_code)]

//! Ed25519 signing key newtype — resolves the deferred-work item from
//! Story 1a.3: `sign_capability_token` `&[u8]` seed with no compile-time
//! size hint.
//!
//! The kernel-side caller passes `&signing_key.0[..]` into the trait;
//! the trait surface stays `&[u8]` (per the 1a.3 freeze; changing it is
//! an ABI break we will not make at v0.1-β).

/// Ed25519 signing key — 32-byte seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ed25519SigningKey(pub [u8; 32]);

impl Ed25519SigningKey {
    /// Construct from a 32-byte seed.
    pub fn new(seed: [u8; 32]) -> Self {
        Self(seed)
    }

    /// Return the seed bytes as a slice for `CryptoProvider::sign_capability_token`.
    pub fn as_seed_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}
