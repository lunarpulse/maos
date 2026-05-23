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

/// Loaded T3 image lock file.
#[derive(Debug, Clone)]
pub struct T3ImageLock {
    attestations: Vec<T3ImageAttestation>,
}

impl T3ImageLock {
    /// Load the pin file from `path`.
    pub fn load(path: &Path) -> Result<Self, T3Error> {
        let bytes = std::fs::read(path)
            .map_err(|e| T3Error::Io(format!("read t3-image.lock: {e}")))?;
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
        self.default_attestation().and_then(|a| {
            a.entries
                .first()
                .ok_or(T3Error::NoDefaultImage)
        })
    }
}

/// Get a fallback default image URI for argv building.
/// This is used when no lock file is available.
pub fn get_default_image() -> Option<String> {
    Some("gcr.io/distroless/cc-debian12".to_string())
}

/// Load the lock file and verify it against the trust anchor.
pub fn load_and_verify_lock(
    trust_anchor_pub: &[u8],
    crypto: &dyn maos_domain::ports::crypto::CryptoProvider,
) -> Result<T3ImageLock, T3Error> {
    let lock = T3ImageLock::load_default()?;
    for attestation in &lock.attestations {
        image_verify::verify_image_attestation(attestation, trust_anchor_pub, crypto)?;
    }
    Ok(lock)
}
