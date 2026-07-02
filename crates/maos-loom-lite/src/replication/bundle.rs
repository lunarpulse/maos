#![forbid(unsafe_code)]

//! Cross-region replication bundle — the signed mediator envelope for a set of
//! collective-memory KV leaves (Story 11.2a).
//!
//! A region packages its canonical [`CollectiveKvLeaf`] rows into a
//! [`CrossRegionReplicationBundle`], binds the region identity into an Ed25519
//! signature (the "region weld"), and ships the bundle to a peer region. The
//! peer verifies the signature against the *declared* source region's derived
//! public key (so a bundle copied + relabelled as another region fails to
//! verify — R-RG1), confirms the Merkle root matches the carried leaves, and
//! then applies the rows through the CRDT LWW upsert.
//!
//! # Sign-only — no AEAD
//!
//! This module performs Ed25519 **signing** only. There is no encryption /
//! AEAD anywhere (ADR-049). Confidentiality is out of scope for the
//! cross-region convergence path; integrity + region pinning is the contract.
//!
//! # Crypto stack
//!
//! `ed25519-dalek` (sign/verify) + `sha2` (Merkle, reused from `leaf`) + `hkdf`
//! (region-bound key derivation, reused from `maos-audit::sealed_export`). No
//! `ring`, no hand-rolled KDF — consistent with Decision B.

use std::future::Future;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use maos_audit::sealed_export::{derive_region_pubkey, derive_region_signing_seed};
use maos_domain::region::Region;

use super::leaf::{kv_merkle_root, CollectiveKvLeaf};
use crate::schema;
use crate::store::LoomLiteStore;

/// Frozen schema version for the replication-bundle wire shape.
///
/// Bumping this is a wire-format change that MUST be gated by an ADR.
const BUNDLE_SCHEMA_VERSION: u16 = 1;

/// Domain separator bound into every bundle signature. Pinned — never change
/// without an ADR, because doing so re-keys every in-flight bundle.
const SIG_DOMAIN: &[u8] = b"maos.cross-region-replication.v1";

/// Schema-version string + domain separator bound into every re-attestation
/// receipt signature.
const RECEIPT_SIG_DOMAIN: &str = "maos.reattestation-receipt.v1";

// ─── Bundle types ──────────────────────────────────────────────────────────

/// Signed, self-describing envelope for a set of cross-region collective-memory
/// KV leaves.
///
/// `root` is the Merkle root over `leaves` (recomputable — carried for
/// fail-fast verification before touching the store). `region_sig` is the
/// Ed25519 signature over [`build_sign_payload`], produced with the
/// source-region's HKDF-derived signing key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossRegionReplicationBundle {
    pub schema_version: u16,
    pub source_region: String,
    pub root: [u8; 32],
    pub leaves: Vec<CollectiveKvLeaf>,
    pub region_sig: Vec<u8>,
}

/// Outcome of applying a replication bundle to a destination store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub applied_count: usize,
    pub skipped_count: usize,
}

/// Receipt attesting that a source region's bundle landed at a destination
/// region. Signed by the home / control-plane seed over a canonical payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReAttestationReceipt {
    pub schema_version: String,
    pub source_region: String,
    pub dest_region: String,
    pub source_root: [u8; 32],
    pub timestamp_ns: u64,
    pub signature: Vec<u8>,
}

/// Errors raised by the replication-bundle path.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("invalid region: {0}")]
    InvalidRegion(String),
    #[error("signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    #[error(
        "merkle root mismatch: expected {}, actual {}",
        hex::encode(expected),
        hex::encode(actual)
    )]
    RootMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    #[error("store error: {0}")]
    StoreError(String),
    #[error("deserialization error: {0}")]
    DeserializationError(String),
}

// ─── Sign payload construction (pinned — build/verify MUST be symmetric) ────

/// Build the exact byte payload covered by the region signature:
/// `SIG_DOMAIN || schema_version.to_be_bytes() || source_region || root`.
fn build_sign_payload(schema_version: u16, source_region: &[u8], root: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(SIG_DOMAIN.len() + 2 + source_region.len() + 32);
    payload.extend_from_slice(SIG_DOMAIN);
    payload.extend_from_slice(&schema_version.to_be_bytes());
    payload.extend_from_slice(source_region);
    payload.extend_from_slice(root);
    payload
}

/// Build the canonical byte payload covered by a re-attestation receipt
/// signature. Variable-length region strings are length-prefixed (big-endian
/// u32) so a boundary shift cannot collide.
fn build_receipt_sign_payload(
    schema_version: &str,
    source_region: &[u8],
    dest_region: &[u8],
    source_root: &[u8; 32],
    timestamp_ns: u64,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        schema_version.len() + 4 + source_region.len() + 4 + dest_region.len() + 32 + 8,
    );
    payload.extend_from_slice(schema_version.as_bytes());
    payload.extend_from_slice(&(source_region.len() as u32).to_be_bytes());
    payload.extend_from_slice(source_region);
    payload.extend_from_slice(&(dest_region.len() as u32).to_be_bytes());
    payload.extend_from_slice(dest_region);
    payload.extend_from_slice(source_root);
    payload.extend_from_slice(&timestamp_ns.to_be_bytes());
    payload
}

// ─── Bundle build / verify ─────────────────────────────────────────────────

/// Build a signed replication bundle from a set of KV leaves.
///
/// Computes the Merkle root over `leaves`, binds the canonical `source_region`
/// into the signature (the region weld), and signs with the region's
/// HKDF-derived Ed25519 key.
pub fn build_replication_bundle(
    leaves: Vec<CollectiveKvLeaf>,
    source_region: &Region,
    base_seed: &[u8; 32],
) -> CrossRegionReplicationBundle {
    let root = kv_merkle_root(&leaves);
    let sign_payload =
        build_sign_payload(BUNDLE_SCHEMA_VERSION, source_region.as_str().as_bytes(), &root);

    let region_seed = derive_region_signing_seed(base_seed, source_region);
    let signing_key = SigningKey::from_bytes(&region_seed);
    let signature = signing_key.sign(&sign_payload);

    CrossRegionReplicationBundle {
        schema_version: BUNDLE_SCHEMA_VERSION,
        source_region: source_region.as_str().to_string(),
        root,
        leaves,
        region_sig: signature.to_bytes().to_vec(),
    }
}

/// Verify a replication bundle against the source region's derived public key.
///
/// Reconstructs the exact sign payload, verifies the Ed25519 signature under
/// the public key derived from the *bundle-declared* source region (so a
/// relabelled/copied bundle fails — the region weld, R-RG1), and confirms the
/// carried `root` matches the recomputed Merkle root of `leaves`.
pub fn verify_replication_bundle(
    bundle: &CrossRegionReplicationBundle,
    base_seed: &[u8; 32],
) -> Result<(), BundleError> {
    let region = Region::canonicalize(&bundle.source_region)
        .map_err(|e| BundleError::InvalidRegion(e.to_string()))?;

    let sign_payload =
        build_sign_payload(bundle.schema_version, region.as_str().as_bytes(), &bundle.root);

    let pubkey_bytes = derive_region_pubkey(base_seed, &region);
    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| BundleError::SignatureVerificationFailed(format!("invalid pubkey: {e}")))?;

    let sig_bytes: [u8; 64] = bundle
        .region_sig
        .as_slice()
        .try_into()
        .map_err(|_| {
            BundleError::SignatureVerificationFailed(format!(
                "signature must be 64 bytes, got {}",
                bundle.region_sig.len()
            ))
        })?;
    let signature = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(&sign_payload, &signature)
        .map_err(|e| BundleError::SignatureVerificationFailed(format!("{e}")))?;

    let actual_root = kv_merkle_root(&bundle.leaves);
    if actual_root != bundle.root {
        return Err(BundleError::RootMismatch {
            expected: bundle.root,
            actual: actual_root,
        });
    }

    Ok(())
}

// ─── Bundle apply (mediator → destination store) ───────────────────────────

/// Apply a verified bundle to a destination store via the CRDT LWW upsert.
///
/// Each leaf is reconstructed into its typed `MemoryNamespace` / `MemoryValue`
/// (via [`schema::parts_to_namespace`] / [`schema::parts_to_value`]) and written
/// with its ORIGINAL `source_ts` + `source_region` provenance — the CRDT LWW
/// property is preserved across re-attestation apply (the converged value is
/// identical regardless of arrival order).
///
/// `dest_region` is validated against the frozen `ascii-v1` region grammar at
/// the apply boundary (region-identity reflex): a malformed destination tag is
/// rejected fail-closed before any row touches the store.
///
/// Returns an [`ApplyResult`] counting applied vs. skipped rows. A store write
/// error is surfaced fail-closed as [`BundleError::StoreError`]; a leaf that
/// fails namespace/value deserialization is counted as skipped (bad data, not a
/// store fault).
pub fn apply_replication_bundle<'a>(
    bundle: &'a CrossRegionReplicationBundle,
    store: &'a LoomLiteStore,
    dest_region: &'a str,
) -> impl Future<Output = Result<ApplyResult, BundleError>> + 'a {
    // Provenance reference stamped on every applied row: the source region +
    // the bundle Merkle root, matching the kernel `SourceLogRef` shape.
    let source_log_ref = format!(
        "{{\"source_region\":\"{}\",\"merkle_root\":\"{}\"}}",
        bundle.source_region,
        hex::encode(bundle.root)
    );

    async move {
        // Region-identity reflex at the apply boundary.
        let dest = Region::canonicalize(dest_region)
            .map_err(|e| BundleError::InvalidRegion(e.to_string()))?;
        tracing::debug!(
            dest_region = %dest,
            source_region = %bundle.source_region,
            leaf_count = bundle.leaves.len(),
            "applying cross-region replication bundle"
        );

        let mut applied_count = 0usize;
        let mut skipped_count = 0usize;

        for leaf in &bundle.leaves {
            let namespace =
                match schema::parts_to_namespace(&leaf.namespace_kind, &leaf.namespace_detail) {
                    Ok(ns) => ns,
                    Err(e) => {
                        tracing::warn!(
                            source_region = %leaf.source_region,
                            key = %leaf.key,
                            error = %e,
                            "skipping leaf: namespace deserialization failed"
                        );
                        skipped_count += 1;
                        continue;
                    }
                };
            let value = match schema::parts_to_value(&leaf.value_kind, &leaf.value_data) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        source_region = %leaf.source_region,
                        key = %leaf.key,
                        error = %e,
                        "skipping leaf: value deserialization failed"
                    );
                    skipped_count += 1;
                    continue;
                }
            };

            // Validate spirit_pid fits in u32 — the schema stores BIGINT (i64)
            // but the CollectiveMemoryPort trait expects u32.  A negative or
            // >u32::MAX value in a bundle is a data-integrity violation; skip it
            // rather than silently truncating (the UNIQUE key would be corrupted).
            let pid = match u32::try_from(leaf.spirit_pid) {
                Ok(p) => p,
                Err(_) => {
                    tracing::warn!(
                        source_region = %leaf.source_region,
                        spirit_pid = leaf.spirit_pid,
                        key = %leaf.key,
                        "skipping leaf: spirit_pid out of u32 range"
                    );
                    skipped_count += 1;
                    continue;
                }
            };

            store
                .write_with_source(
                    pid,
                    &namespace,
                    &leaf.key,
                    value,
                    leaf.source_ts,
                    &leaf.source_region,
                    &source_log_ref,
                )
                .await
                .map_err(|e| BundleError::StoreError(e.to_string()))?;

            applied_count += 1;
        }

        Ok(ApplyResult {
            applied_count,
            skipped_count,
        })
    }
}

// ─── Re-attestation receipt ────────────────────────────────────────────────

/// Build a signed re-attestation receipt from the home / control-plane seed.
///
/// The receipt binds the source region, destination region, source Merkle root,
/// and a timestamp under a single Ed25519 signature — the control-plane
/// attestation that a cross-region apply occurred.
pub fn build_reattestation_receipt(
    home_seed: &[u8; 32],
    source_region: &str,
    dest_region: &str,
    source_root: [u8; 32],
    timestamp_ns: u64,
) -> ReAttestationReceipt {
    let payload = build_receipt_sign_payload(
        RECEIPT_SIG_DOMAIN,
        source_region.as_bytes(),
        dest_region.as_bytes(),
        &source_root,
        timestamp_ns,
    );

    let signing_key = SigningKey::from_bytes(home_seed);
    let signature = signing_key.sign(&payload);

    ReAttestationReceipt {
        schema_version: RECEIPT_SIG_DOMAIN.to_string(),
        source_region: source_region.to_string(),
        dest_region: dest_region.to_string(),
        source_root,
        timestamp_ns,
        signature: signature.to_bytes().to_vec(),
    }
}

/// Verify a re-attestation receipt against the home / control-plane public key.
pub fn verify_reattestation_receipt(
    receipt: &ReAttestationReceipt,
    home_pubkey: &[u8; 32],
) -> Result<(), BundleError> {
    let payload = build_receipt_sign_payload(
        &receipt.schema_version,
        receipt.source_region.as_bytes(),
        receipt.dest_region.as_bytes(),
        &receipt.source_root,
        receipt.timestamp_ns,
    );

    let verifying_key = VerifyingKey::from_bytes(home_pubkey)
        .map_err(|e| BundleError::SignatureVerificationFailed(format!("invalid pubkey: {e}")))?;

    let sig_bytes: [u8; 64] = receipt
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| {
            BundleError::SignatureVerificationFailed(format!(
                "signature must be 64 bytes, got {}",
                receipt.signature.len()
            ))
        })?;
    let signature = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(&payload, &signature)
        .map_err(|e| BundleError::SignatureVerificationFailed(format!("{e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_leaf(region: &str) -> CollectiveKvLeaf {
        CollectiveKvLeaf {
            source_region: region.to_string(),
            source_ts: 1_700_000_000_000_000_000,
            spirit_pid: 42,
            namespace_kind: "local".to_string(),
            namespace_detail: "detail-x".to_string(),
            key: "memory-key".to_string(),
            value_kind: "text/plain".to_string(),
            value_data: b"hello-world".to_vec(),
        }
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-west-2a").unwrap();
        let leaf_a = sample_leaf("us-west-2a");
        let mut leaf_b = sample_leaf("us-west-2a");
        leaf_b.key = "memory-key-2".to_string();
        let leaves = vec![leaf_a, leaf_b];

        let bundle = build_replication_bundle(leaves, &region, &base_seed);
        assert_eq!(bundle.schema_version, BUNDLE_SCHEMA_VERSION);
        assert_eq!(bundle.source_region, "us-west-2a");
        assert_eq!(bundle.region_sig.len(), 64);

        // Verify against the SAME base seed → derives region A's pubkey → OK.
        verify_replication_bundle(&bundle, &base_seed)
            .expect("bundle signed and verified under the same region must verify");
    }

    #[test]
    fn test_copied_bundle_fails_verification() {
        let base_seed = [0x42u8; 32];
        let region_a = Region::canonicalize("region-a").unwrap();
        let leaves = vec![sample_leaf("region-a")];

        let mut bundle = build_replication_bundle(leaves, &region_a, &base_seed);

        // The copy attack: relabel a region-A bundle as region B. The signature
        // was produced under region A's derived key, but verify now derives
        // region B's pubkey — the weld rejects it.
        bundle.source_region = "region-b".to_string();

        let err = verify_replication_bundle(&bundle, &base_seed).unwrap_err();
        assert!(
            matches!(err, BundleError::SignatureVerificationFailed(_)),
            "a bundle copied from region A and relabelled as region B MUST fail verification (the weld), got {err:?}"
        );
    }

    #[test]
    fn test_tampered_root_fails() {
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-west-2a").unwrap();
        let leaves = vec![sample_leaf("us-west-2a")];

        let mut bundle = build_replication_bundle(leaves, &region, &base_seed);

        // Tamper the root after signing: it is part of the signed payload, so
        // the signature no longer covers it.
        bundle.root[0] ^= 0xff;

        assert!(
            verify_replication_bundle(&bundle, &base_seed).is_err(),
            "a tampered root MUST fail verification"
        );
    }

    #[test]
    fn test_reattestation_receipt_roundtrip() {
        let home_seed = [0x11u8; 32];
        let home_pubkey = maos_audit::sealed_export::derive_pubkey(&home_seed);
        let source_root = [0xabu8; 32];

        let receipt = build_reattestation_receipt(
            &home_seed,
            "us-east-1",
            "eu-west-1",
            source_root,
            1_700_000_000_000_000_000,
        );
        assert_eq!(receipt.schema_version, RECEIPT_SIG_DOMAIN);
        assert_eq!(receipt.signature.len(), 64);

        verify_reattestation_receipt(&receipt, &home_pubkey)
            .expect("receipt signed and verified under the same home key must verify");

        // Tamper → fails.
        let mut tampered = receipt.clone();
        tampered.timestamp_ns += 1;
        assert!(
            verify_reattestation_receipt(&tampered, &home_pubkey).is_err(),
            "a tampered receipt MUST fail verification"
        );

        // Wrong pubkey → fails.
        let other_pubkey = maos_audit::sealed_export::derive_pubkey(&[0xffu8; 32]);
        assert!(
            verify_reattestation_receipt(&receipt, &other_pubkey).is_err(),
            "a receipt verified under the wrong home pubkey MUST fail"
        );
    }

    #[test]
    fn test_empty_bundle_sign_verify() {
        // Edge case: a bundle with zero leaves should still sign/verify.
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-west-2a").unwrap();
        let bundle = build_replication_bundle(vec![], &region, &base_seed);
        assert_eq!(bundle.leaves.len(), 0);
        assert_eq!(bundle.root, [0u8; 32], "empty leaf set → zero Merkle root");
        verify_replication_bundle(&bundle, &base_seed)
            .expect("empty bundle must still verify (it was signed)");
    }

    #[test]
    fn test_wrong_base_seed_fails() {
        // A bundle signed with one seed must NOT verify under a different seed
        // (different derived keys).
        let seed_a = [0x42u8; 32];
        let seed_b = [0x43u8; 32];
        let region = Region::canonicalize("us-west-2a").unwrap();
        let leaves = vec![sample_leaf("us-west-2a")];
        let bundle = build_replication_bundle(leaves, &region, &seed_a);
        let err = verify_replication_bundle(&bundle, &seed_b).unwrap_err();
        assert!(
            matches!(err, BundleError::SignatureVerificationFailed(_)),
            "verification under a different base seed MUST fail"
        );
    }

    #[test]
    fn test_tampered_leaf_in_bundle_fails() {
        // Tampering a leaf after signing should be caught by the Merkle root
        // check (the root no longer matches the leaves).
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("region-a").unwrap();
        let leaves = vec![sample_leaf("region-a")];
        let mut bundle = build_replication_bundle(leaves, &region, &base_seed);
        // Tamper the leaf's value AFTER signing.
        bundle.leaves[0].value_data = b"TAMPERED".to_vec();
        let err = verify_replication_bundle(&bundle, &base_seed).unwrap_err();
        assert!(
            matches!(err, BundleError::RootMismatch { .. }),
            "a tampered leaf must cause a Merkle root mismatch, got {err:?}"
        );
    }

    #[test]
    fn test_build_sign_payload_deterministic() {
        // The sign payload construction must be deterministic.
        let p1 = build_sign_payload(1, b"region-a", &[0xab; 32]);
        let p2 = build_sign_payload(1, b"region-a", &[0xab; 32]);
        assert_eq!(p1, p2, "sign payload must be deterministic");
        // Different region → different payload.
        let p3 = build_sign_payload(1, b"region-b", &[0xab; 32]);
        assert_ne!(p1, p3, "different region must produce different payload");
    }

    #[test]
    fn test_receipt_boundary_shift() {
        // Boundary-shift resistance: "ab"|"c" vs "a"|"bc" in region names.
        let home_seed = [0x11u8; 32];
        let r1 = build_reattestation_receipt(&home_seed, "ab", "c", [0; 32], 0);
        let r2 = build_reattestation_receipt(&home_seed, "a", "bc", [0; 32], 0);
        // They must produce different signatures (different canonical payloads).
        assert_ne!(
            r1.signature, r2.signature,
            "boundary shift in region names must produce distinct receipts"
        );
    }
}
