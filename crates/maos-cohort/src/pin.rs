#![forbid(unsafe_code)]
//! Operator-pinned genesis authority key set — the cohort-level trust root
//! (Story 12.1 / RR4).
//!
//! This is the **out-of-band pinned Ed25519 authority key surface**. It is
//! deliberately a **cohort-level set-valued pin**, NOT a per-peer
//! `A2APeerConfig` field:
//!
//! - `cert_fingerprint` (`maos_a2a_core::PeerCertFingerprint`) is a TLS-cert
//!   SHA-256 — it pins the *member's mTLS identity*, not the cohort authority.
//! - The cohort authority is a directly-pinned Ed25519 pubkey (NOT a
//!   region-HKDF-derived key like `maos-loom-lite`'s
//!   `derive_region_pubkey`). Ed25519 has no key recovery; the verifier holds
//!   the key and checks the signature against it (AC3 / RR3).
//!
//! # Set-valued for rotation overlap (RR5)
//!
//! ADR-054 §2 rotates the authority key via the §7.2.1.a one-generation-overlap
//! idiom (two valid keys: `{current, next}`). A strict single-key equality pin
//! would brick that rotation — so the pin carries a *set*. Steady-state the
//! set is `{current}`; during operator-declared rotation it is
//! `{current, next}`. A re-issue signed by either verifies.
//!
//! # Provisioning (out-of-band)
//!
//! Each member holds the genesis authority pubkey **operator-provisioned out of
//! band** — the same posture as the §7.2 cert-fingerprint pin. TOFU-on-first-
//! manifest is NOT used for the authority key. Custody follows the 9.4b
//! runbook (§15.7): the operator distributes the pubkey to each member host's
//! cohort config before the first manifest is seen. A manifest whose declared
//! authority ≠ the member's pinned set is refused (`ECohortAuthorityUnpinned`)
//! — this closes the genesis circularity (a forged v1 self-declaring +
//! self-signing its own authority).

#![forbid(unsafe_code)]

use ed25519_dalek::VerifyingKey;

use crate::error::CohortError;

/// Operator-pinned genesis authority key set — the cohort trust root.
///
/// Construct via [`PinnedAuthorityKeys::from_hex`] (parses 32-byte Ed25519
/// verifying keys from hex, dedups) or [`PinnedAuthorityKeys::from_keys`].
/// The set MUST be non-empty (a cohort without a pinned authority root cannot
/// verify any manifest).
#[derive(Debug, Clone)]
pub struct PinnedAuthorityKeys {
    keys: Vec<VerifyingKey>,
}

impl PinnedAuthorityKeys {
    /// Construct from already-parsed verifying keys. Dedups by the 32-byte
    /// encoding; rejects an empty set.
    pub fn from_keys(keys: Vec<VerifyingKey>) -> Result<Self, CohortError> {
        if keys.is_empty() {
            return Err(CohortError::EEmptyAuthority);
        }
        let mut seen: Vec<[u8; 32]> = keys.iter().map(|k| k.to_bytes()).collect();
        seen.sort_unstable();
        seen.dedup();
        let unique: Vec<VerifyingKey> = seen
            .into_iter()
            .map(|b| VerifyingKey::from_bytes(&b).expect("round-trip from a valid key"))
            .collect();
        Ok(Self { keys: unique })
    }

    /// Construct from hex-encoded 32-byte Ed25519 verifying keys. Each entry
    /// MUST be 64 lowercase/uppercase hex chars decoding to a valid key.
    /// Dedups; rejects an empty input or any malformed entry.
    pub fn from_hex(hex_keys: &[String]) -> Result<Self, CohortError> {
        if hex_keys.is_empty() {
            return Err(CohortError::EEmptyAuthority);
        }
        let mut keys = Vec::with_capacity(hex_keys.len());
        for h in hex_keys {
            keys.push(parse_verifying_key(h)?);
        }
        Self::from_keys(keys)
    }

    /// The pinned keys, as raw 32-byte arrays.
    pub fn key_bytes(&self) -> Vec<[u8; 32]> {
        self.keys.iter().map(|k| k.to_bytes()).collect()
    }

    /// Hex encoding of the pinned keys (lowercase), for display / logging.
    pub fn hex(&self) -> Vec<String> {
        self.keys.iter().map(|k| hex::encode(k.to_bytes())).collect()
    }

    /// Iterate the underlying verifying keys.
    pub fn iter(&self) -> impl Iterator<Item = &VerifyingKey> {
        self.keys.iter()
    }

    /// Number of distinct pinned keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// `true` iff the set is empty. (Construction forbids empty, but the
    /// method is provided for ergonomic `is_empty()` checks at call sites.)
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

/// Parse a hex string into a 32-byte Ed25519 verifying key.
///
/// Public so manifest validation can reuse the exact grammar on the
/// manifest-declared authority keys (a declared key that is not a valid
/// 32-byte Ed25519 pubkey → `EInvalidAuthorityKey`).
pub(crate) fn parse_verifying_key(hex_str: &str) -> Result<VerifyingKey, CohortError> {
    let bytes = hex::decode(hex_str).map_err(|e| {
        CohortError::EInvalidAuthorityKey(format!("bad hex ({e}): {hex_str}"))
    })?;
    let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        CohortError::EInvalidAuthorityKey(format!(
            "expected 32 bytes (64 hex chars), got {} bytes: {hex_str}",
            bytes.len()
        ))
    })?;
    VerifyingKey::from_bytes(&arr)
        .map_err(|e| CohortError::EInvalidAuthorityKey(format!("{e}: {hex_str}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pk_hex(seed: u8) -> String {
        // Deterministic 32-byte seed → valid Ed25519 key. The all-constant
        // array is a valid seed for ed25519-dalek (no small-subgroup rejection
        // on verification keys derived this way).
        let sk = ed25519_dalek::SigningKey::from_bytes(&{
            let mut s = [0u8; 32];
            s[0] = seed;
            s[31] = 1;
            s
        });
        hex::encode(sk.verifying_key().to_bytes())
    }

    #[test]
    fn from_hex_dedups() {
        let h = pk_hex(7);
        let pinned = PinnedAuthorityKeys::from_hex(&[h.clone(), h.clone()]).unwrap();
        assert_eq!(pinned.len(), 1, "duplicate pinned keys must dedup");
    }

    #[test]
    fn from_hex_rejects_empty() {
        assert!(matches!(
            PinnedAuthorityKeys::from_hex(&[]),
            Err(CohortError::EEmptyAuthority)
        ));
    }

    #[test]
    fn from_hex_rejects_bad_length() {
        let bad = "deadbeef".to_string(); // 4 bytes, not 32
        assert!(matches!(
            PinnedAuthorityKeys::from_hex(&[bad]),
            Err(CohortError::EInvalidAuthorityKey(_))
        ));
    }
}
