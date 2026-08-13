//! T3 image lock — loads and resolves the `t3-image.lock` JSON pin file.
//!
//! The pin file maps an operator-chosen `image_pin` name (from the manifest's
//! `[sandbox].image_pin` field) to a `T3ImageAttestation` entry. At v0.5-α
//! the lock file contains a single test-only attestation entry; the
//! production trust anchor is operator-supplied at deploy time via env-var
//! `MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX`.

use std::path::{Path, PathBuf};

use maos_domain::sandbox::{T3Error, T3ImageAttestation, T3ImageEntry};

use crate::security::sandbox::t3::image_verify;

/// Default path to the T3 image lock file.
const DEFAULT_LOCK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/security/sandbox/t3-image.lock"
);

/// A lock that has passed trust-anchor, signature, and placeholder validation.
///
/// The inner attestations deliberately cannot be constructed by callers.  T3
/// spawning accepts only an image selected from this type, preventing a caller
/// from bypassing admission with deserialized lock material.
#[derive(Debug, Clone)]
pub struct VerifiedImageLock {
    attestations: Vec<T3ImageAttestation>,
}

/// A selected image whose containing lock was cryptographically verified.
#[derive(Debug, Clone)]
pub struct VerifiedImageAttestation {
    attestation: T3ImageAttestation,
    entry: T3ImageEntry,
}

impl VerifiedImageAttestation {
    pub(crate) fn entry(&self) -> &T3ImageEntry {
        &self.entry
    }

    #[cfg(test)]
    pub(crate) fn for_test(attestation: T3ImageAttestation, entry: T3ImageEntry) -> Self {
        Self { attestation, entry }
    }
}

/// Loaded T3 image lock file. This is parse-only and must not reach spawning.
#[derive(Debug, Clone)]
#[maos_attrs::i9_exempt(
    reason = "loaded T3 image lock config; Vec<T3ImageAttestation> is bounded structural config state per I9, keyed by image digest, no parameter drift (Story 7.1.7 baseline-reset)"
)]
pub struct T3ImageLock {
    attestations: Vec<T3ImageAttestation>,
}

impl T3ImageLock {
    /// Load the pin file from `path`.
    pub fn load(path: &Path) -> Result<Self, T3Error> {
        let bytes =
            std::fs::read(path).map_err(|e| T3Error::Io(format!("read t3-image.lock: {e}")))?;
        let attestations: Vec<T3ImageAttestation> =
            serde_json::from_slice(&bytes).map_err(|e| T3Error::Io(e.to_string()))?;
        Ok(Self { attestations })
    }

    /// Load from the default path, or from `MAOS_T3_IMAGE_LOCK_PATH` env-var.
    pub fn load_default() -> Result<Self, T3Error> {
        let path = std::env::var("MAOS_T3_IMAGE_LOCK_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_LOCK_PATH));
        Self::load(&path)
    }

    /// Resolve a pin name to the matching entry.
    /// Returns `None` if no entry's `image_uri` matches `name`.
    pub fn resolve_pin(&self, name: &str) -> Option<&T3ImageEntry> {
        for attestation in &self.attestations {
            for entry in &attestation.entries {
                if entry.image_uri == name {
                    return Some(entry);
                }
            }
        }
        None
    }

    /// Find the default attestation (exactly one entry with `default_for_v05 = true`).
    pub fn default_attestation(&self) -> Result<&T3ImageAttestation, T3Error> {
        for attestation in &self.attestations {
            if attestation.entries.iter().any(|e| e.default_for_v05) {
                return Ok(attestation);
            }
        }
        Err(T3Error::NoDefaultImage)
    }

    /// Get the first entry from the default attestation.
    pub fn default_entry(&self) -> Result<&T3ImageEntry, T3Error> {
        self.default_attestation()
            .and_then(|a| a.entries.first().ok_or(T3Error::NoDefaultImage))
    }
}

impl VerifiedImageLock {
    /// Select a named image from the verified lock.
    pub fn resolve_pin(&self, name: &str) -> Result<VerifiedImageAttestation, T3Error> {
        self.attestations
            .iter()
            .flat_map(|attestation| {
                attestation
                    .entries
                    .iter()
                    .map(move |entry| (attestation, entry))
            })
            .find(|(_, entry)| entry.image_uri == name)
            .map(|(attestation, entry)| VerifiedImageAttestation {
                attestation: attestation.clone(),
                entry: entry.clone(),
            })
            .ok_or_else(|| T3Error::ImagePinMissing {
                name: name.to_owned(),
            })
    }

    /// Select the single v0.5 default image from the verified lock.
    pub fn default_entry(&self) -> Result<VerifiedImageAttestation, T3Error> {
        let mut defaults = self.attestations.iter().flat_map(|attestation| {
            attestation
                .entries
                .iter()
                .filter(move |entry| entry.default_for_v05)
                .map(move |entry| (attestation, entry))
        });
        let (attestation, entry) = defaults.next().ok_or(T3Error::NoDefaultImage)?;
        if defaults.next().is_some() {
            return Err(T3Error::SignatureInvalid);
        }
        Ok(VerifiedImageAttestation {
            attestation: attestation.clone(),
            entry: entry.clone(),
        })
    }
}

/// Load the lock file and verify it against the configured trust anchor.
///
/// A parsed [`T3ImageLock`] is intentionally not sufficient for admission or
/// spawn: only this function constructs the capability-like verified wrapper.
pub fn load_and_verify_lock(
    trust_anchor_pub: &[u8],
    crypto: &dyn maos_domain::ports::crypto::CryptoProvider,
) -> Result<VerifiedImageLock, T3Error> {
    verify_loaded_lock(T3ImageLock::load_default()?, trust_anchor_pub, crypto)
}

/// Load and verify an explicitly selected lock file.
///
/// Composition roots and deterministic tests use this path rather than
/// mutating process-global environment variables.
pub fn load_and_verify_lock_at(
    path: &Path,
    trust_anchor_pub: &[u8],
    crypto: &dyn maos_domain::ports::crypto::CryptoProvider,
) -> Result<VerifiedImageLock, T3Error> {
    verify_loaded_lock(T3ImageLock::load(path)?, trust_anchor_pub, crypto)
}

fn verify_loaded_lock(
    lock: T3ImageLock,
    trust_anchor_pub: &[u8],
    crypto: &dyn maos_domain::ports::crypto::CryptoProvider,
) -> Result<VerifiedImageLock, T3Error> {
    for attestation in &lock.attestations {
        if is_shipped_placeholder(attestation) {
            return Err(T3Error::PlaceholderImageLock);
        }
        image_verify::verify_image_attestation(attestation, trust_anchor_pub, crypto)?;
    }
    Ok(VerifiedImageLock {
        attestations: lock.attestations,
    })
}

fn is_shipped_placeholder(attestation: &T3ImageAttestation) -> bool {
    attestation.signed_at_ns == 0
        && attestation.signature.iter().all(|byte| *byte == 1)
        && attestation.signer_pub_key.iter().all(|byte| *byte == 2)
        && attestation.entries.iter().any(|entry| {
            entry.image_sha256.iter().all(|byte| *byte == 0xaa)
                && entry.description.contains("placeholder")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::sandbox::ImageAttestationId;

    #[test]
    fn shipped_placeholder_shape_is_never_verifiable() {
        let placeholder = T3ImageAttestation {
            id: ImageAttestationId([0xaa; 32]),
            schema_version: 1,
            signed_at_ns: 0,
            entries: vec![T3ImageEntry {
                image_uri: "registry.invalid/placeholder".into(),
                image_sha256: [0xaa; 32],
                description: "placeholder".into(),
                default_for_v05: true,
            }],
            signature: [1; 64],
            signer_pub_key: [2; 32],
        };
        assert!(is_shipped_placeholder(&placeholder));
    }

    #[test]
    fn shipped_default_lock_is_rejected_before_signature_use() {
        assert!(matches!(
            load_and_verify_lock_at(
                Path::new(DEFAULT_LOCK_PATH),
                &[2; 32],
                &crate::security::RingCryptoProvider,
            ),
            Err(T3Error::PlaceholderImageLock)
        ));
    }
}
