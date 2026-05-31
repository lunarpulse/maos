//! Byte-stable CBOR encoding for the execution-context fingerprint hash.
//!
//! # Why this module is load-bearing
//!
//! The CCAC ±2% cross-validation (AC5) and the producer→evaluator round-trip
//! (AC6 smoke step 1) only hold if the corpus GENERATOR and the admission-time
//! EVALUATOR hash the [`ExecutionContextFingerprint`] with the *same* byte
//! sequence. This module is that single shared encoder — both
//! `maos-corpus-gen::ccac` and [`crate::evaluator`] route through
//! [`fingerprint_hash`].
//!
//! # The byte-stable encoding (documented per AC2 item 4)
//!
//! We use `serde_cbor`'s deterministic struct serialization:
//!
//! * Struct fields are emitted in **declaration order** (`manifest_hash`,
//!   `spirit_version`, `trust_tier`, `sandbox_tier`, `capability_scope`,
//!   `provider_endpoint`, `crypto_provider`) — `serde_cbor` does not reorder
//!   struct keys, so the map key order is fixed by the frozen ABI struct shape.
//! * `capability_scope` is a `BTreeSet<CapabilityId>` and `provider_endpoint`
//!   is a fixed struct, so every collection is emitted in a canonical, sorted,
//!   definite-length form.
//! * Integers use `serde_cbor`'s shortest-form encoding; all fields are
//!   fixed-width or length-prefixed.
//!
//! This is the EXACT encoding the v0.5-α producer
//! (`maos-spirit-cli::compliance_claim::auto_populate`, via the lifted
//! [`compute_fingerprint_hash`]) already pins against, so lifting the logic
//! here keeps every existing fixture and the producer round-trip byte-identical.
//! Because the key order is dictated by the frozen struct (not an arbitrary
//! map), the output is RFC-8949-canonical for this fixed shape and is stable
//! across hosts and `serde_cbor` patch releases.

use maos_spirit_abi::compliance::ExecutionContextFingerprint;
use sha2::Digest;

/// Canonical CBOR bytes of an [`ExecutionContextFingerprint`].
pub fn encode_fingerprint(fp: &ExecutionContextFingerprint) -> Vec<u8> {
    // `ExecutionContextFingerprint` is plain data with derived `Serialize`;
    // serialization is infallible for the frozen shape.
    serde_cbor::to_vec(fp).expect("ExecutionContextFingerprint is always CBOR-serializable")
}

/// `fingerprint_hash = sha256(canonical_cbor(fp))`.
///
/// This is the lifted, renamed home of Story 5.5d's
/// `compliance_verify::compute_fingerprint_hash`. The evaluator and the CCAC
/// generator MUST both call this so the hashes agree byte-for-byte.
pub fn fingerprint_hash(fp: &ExecutionContextFingerprint) -> [u8; 32] {
    sha256(&encode_fingerprint(fp))
}

/// Story 5.5d compatibility alias — the name `maos-spirit-cli` and the
/// admission tests import. Delegates to [`fingerprint_hash`].
pub fn compute_fingerprint_hash(fp: &ExecutionContextFingerprint) -> [u8; 32] {
    fingerprint_hash(fp)
}

/// SHA-256 of arbitrary bytes.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{
        CapabilityId, CryptoProviderId, ProviderEndpointPin, SandboxTier, TrustTier,
    };
    use std::collections::BTreeSet;

    fn sample_fp() -> ExecutionContextFingerprint {
        let mut caps = BTreeSet::new();
        caps.insert(CapabilityId("fs.read".into()));
        caps.insert(CapabilityId("net.connect".into()));
        ExecutionContextFingerprint {
            manifest_hash: [7u8; 32],
            spirit_version: "1.2.3".into(),
            trust_tier: TrustTier::PublicUntrusted,
            sandbox_tier: SandboxTier::T3,
            capability_scope: caps,
            provider_endpoint: ProviderEndpointPin {
                provider_id: "anthropic".into(),
                endpoint_url: "https://api.anthropic.com".into(),
                model_id: Some("claude".into()),
            },
            crypto_provider: CryptoProviderId("ring".into()),
        }
    }

    #[test]
    fn fingerprint_hash_is_deterministic() {
        let fp = sample_fp();
        assert_eq!(fingerprint_hash(&fp), fingerprint_hash(&fp));
    }

    #[test]
    fn capability_scope_order_is_canonical() {
        // BTreeSet sorts, so inserting in different orders yields the same hash.
        let mut a = sample_fp();
        let mut caps_b = BTreeSet::new();
        caps_b.insert(CapabilityId("net.connect".into()));
        caps_b.insert(CapabilityId("fs.read".into()));
        a.capability_scope = caps_b;
        assert_eq!(fingerprint_hash(&a), fingerprint_hash(&sample_fp()));
    }

    #[test]
    fn distinct_tiers_hash_differently() {
        let mut a = sample_fp();
        a.trust_tier = TrustTier::Local;
        assert_ne!(fingerprint_hash(&a), fingerprint_hash(&sample_fp()));
    }

    #[test]
    fn compute_alias_matches() {
        let fp = sample_fp();
        assert_eq!(compute_fingerprint_hash(&fp), fingerprint_hash(&fp));
    }
}
