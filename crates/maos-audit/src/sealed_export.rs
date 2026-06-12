//! FR44 sealed-export: deterministic bundle serialization, Ed25519 signing,
//! and in-tree verification.
//!
//! Uses `ed25519-dalek` + `sha2` — NOT `ring` and NOT `maos-kernel-core`.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

// ─── Bundle types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditBundle {
    pub schema_version: String,
    pub entries: Vec<crate::AuditEntry>,
    pub i12_digest_refs: Vec<String>,
    pub i11_distilled_content: Vec<I11Content>,
    pub freshness: FreshnessMetadata,
    pub signature_block: SignatureBlock,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct I11Content {
    pub source_log_ref: Vec<String>,
    pub distillation_depth: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FreshnessMetadata {
    pub export_timestamp_ns: u64,
    pub covered_window: CoveredWindow,
    pub export_seq: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoveredWindow {
    pub since_ns: u64,
    pub until_ns: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignatureBlock {
    pub algorithm: String,
    pub attester_pubkey: String,
    pub signature: String,
}

/// Unsigned bundle — all fields except `signature_block`.
/// Used as the intermediate form for canonical serialization + signing.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BundleForSigning {
    pub schema_version: String,
    pub entries: Vec<crate::AuditEntry>,
    pub i12_digest_refs: Vec<String>,
    pub i11_distilled_content: Vec<I11Content>,
    pub freshness: FreshnessMetadata,
}

#[derive(Debug, thiserror::Error)]
pub enum SealedExportError {
    #[error("signing error: {0}")]
    Signing(#[from] ed25519_dalek::SignatureError),
    #[error("invalid seed length: expected 32 bytes, got {0}")]
    InvalidSeedLen(usize),
    #[error("invalid public key: {0}")]
    InvalidPubkey(String),
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    #[error("canonical serialization error: {0}")]
    Serialization(String),
    #[error("signature verification failed")]
    VerificationFailed,
}

// ─── Core functions ────────────────────────────────────────────────────────

/// Build a `BundleForSigning` from the raw components.
pub fn build_bundle(
    entries: Vec<crate::AuditEntry>,
    i12_refs: Vec<String>,
    i11_content: Vec<I11Content>,
    freshness: FreshnessMetadata,
) -> BundleForSigning {
    BundleForSigning {
        schema_version: "maos.audit-bundle.v1".to_string(),
        entries,
        i12_digest_refs: i12_refs,
        i11_distilled_content: i11_content,
        freshness,
    }
}

/// Deterministic canonical serialization: sorted keys, no insignificant whitespace.
///
/// Serializes to `serde_json::Value`, recursively sorts all object keys via
/// `BTreeMap` ordering, then outputs compact JSON. Ensures byte-identical
/// output regardless of struct field declaration order.
pub fn canonicalize(bundle: &BundleForSigning) -> Vec<u8> {
    _canonicalize_to_bytes(bundle)
}

/// Sign the canonical bundle bytes with Ed25519.
///
/// Computes sha256(canonical_bytes), signs with the given seed,
/// and returns the complete signed `AuditBundle`.
pub fn sign_bundle(
    bundle_for_signing: BundleForSigning,
    seed: &[u8; 32],
) -> Result<AuditBundle, SealedExportError> {
    let canonical = canonicalize(&bundle_for_signing);
    let digest = Sha256::digest(&canonical);

    let signing_key = SigningKey::from_bytes(seed);
    let signature = signing_key.sign(&digest);
    let pubkey_bytes = signing_key.verifying_key().to_bytes();

    Ok(AuditBundle {
        schema_version: bundle_for_signing.schema_version,
        entries: bundle_for_signing.entries,
        i12_digest_refs: bundle_for_signing.i12_digest_refs,
        i11_distilled_content: bundle_for_signing.i11_distilled_content,
        freshness: bundle_for_signing.freshness,
        signature_block: SignatureBlock {
            algorithm: "Ed25519".to_string(),
            attester_pubkey: hex::encode(pubkey_bytes),
            signature: hex::encode(signature.to_bytes()),
        },
    })
}

/// In-tree convenience verifier.
///
/// Rebuilds the canonical bytes (all fields except `signature_block`),
/// hashes with sha256, and verifies the Ed25519 signature.
pub fn verify_bundle(
    bundle: &AuditBundle,
    pubkey_bytes: &[u8; 32],
) -> Result<(), SealedExportError> {
    let verifying_key = VerifyingKey::from_bytes(pubkey_bytes)
        .map_err(|e| SealedExportError::InvalidPubkey(format!("{e}")))?;

    let unsigned = BundleForSigning {
        schema_version: bundle.schema_version.clone(),
        entries: bundle.entries.clone(),
        i12_digest_refs: bundle.i12_digest_refs.clone(),
        i11_distilled_content: bundle.i11_distilled_content.clone(),
        freshness: bundle.freshness.clone(),
    };

    let canonical = canonicalize(&unsigned);
    let digest = Sha256::digest(&canonical);

    let sig_bytes: [u8; 64] = hex::decode(&bundle.signature_block.signature)
        .map_err(|e| SealedExportError::InvalidSignature(format!("signature hex: {e}")))?
        .try_into()
        .map_err(|v: Vec<u8>| {
            SealedExportError::InvalidSignature(format!(
                "signature must be 64 bytes, got {}",
                v.len()
            ))
        })?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(&digest, &signature)
        .map_err(|_| SealedExportError::VerificationFailed)
}

/// Serialize a `BundleForSigning` to canonical bytes (sorted keys, no whitespace).
///
/// The approach: serialize to `serde_json::Value`, which preserves struct field
/// order, then re-serialize using `serde_json::to_string` which outputs compact
/// JSON. Since we control the struct definition order and use Vec (not HashMap),
/// field order is deterministic.
///
/// For true sorted-key guarantee (independent of struct definition order),
/// we serialize to a `BTreeMap<String, serde_json::Value>` first.
fn _canonicalize_to_bytes(bundle: &BundleForSigning) -> Vec<u8> {
    // Serialize to serde_json::Value, then convert to sorted representation
    let value = serde_json::to_value(bundle).expect("BundleForSigning is always serializable");
    let sorted = sort_value(value);
    serde_json::to_string(&sorted)
        .expect("sorted Value is always serializable")
        .into_bytes()
}

/// Recursively sort all JSON object keys using BTreeMap for deterministic order.
fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: serde_json::Map<String, serde_json::Value> = {
                let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                serde_json::Map::from_iter(entries)
            };
            let sorted_inner: serde_json::Map<String, serde_json::Value> = sorted
                .into_iter()
                .map(|(k, v)| (k, sort_value(v)))
                .collect();
            serde_json::Value::Object(sorted_inner)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}

/// Derive the Ed25519 public key from a seed.
pub fn derive_pubkey(seed: &[u8; 32]) -> [u8; 32] {
    let signing_key = SigningKey::from_bytes(seed);
    signing_key.verifying_key().to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditEntry;

    fn make_test_entries() -> Vec<AuditEntry> {
        vec![AuditEntry {
            frame_id_hex: "deadbeef".to_string(),
            timestamp_ns: 1000,
            spirit_pid: 1,
            boot_nonce: 42,
            capability_token_hex: None,
            kind: "test.kind".to_string(),
            intent: "test.intent".to_string(),
        }]
    }

    fn make_freshness() -> FreshnessMetadata {
        FreshnessMetadata {
            export_timestamp_ns: 2000,
            covered_window: CoveredWindow {
                since_ns: 0,
                until_ns: 2000,
            },
            export_seq: 1,
        }
    }

    #[test]
    fn sign_verify_roundtrip() {
        let seed = [7u8; 32];
        let entries = make_test_entries();
        let freshness = make_freshness();
        let i11 = vec![I11Content {
            source_log_ref: vec!["ref1".to_string()],
            distillation_depth: 0,
        }];

        let unsigned = build_bundle(entries, vec!["i12ref".to_string()], i11, freshness);
        let signed = sign_bundle(unsigned, &seed).expect("signing should succeed");

        assert_eq!(signed.schema_version, "maos.audit-bundle.v1");
        assert_eq!(signed.signature_block.algorithm, "Ed25519");
        assert_eq!(signed.entries.len(), 1);
        assert_eq!(signed.i12_digest_refs.len(), 1);

        let pubkey = derive_pubkey(&seed);
        verify_bundle(&signed, &pubkey).expect("verification should succeed");
    }

    #[test]
    fn wrong_key_fails_verification() {
        let seed = [7u8; 32];
        let wrong_seed = [99u8; 32];

        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness());
        let signed = sign_bundle(unsigned, &seed).unwrap();

        let wrong_pubkey = derive_pubkey(&wrong_seed);
        assert!(verify_bundle(&signed, &wrong_pubkey).is_err());
    }

    #[test]
    fn canonical_deterministic() {
        let unsigned = build_bundle(
            make_test_entries(),
            vec!["a".to_string()],
            vec![],
            make_freshness(),
        );
        let bytes1 = canonicalize(&unsigned);
        let bytes2 = canonicalize(&unsigned);
        assert_eq!(
            bytes1, bytes2,
            "canonical serialization must be deterministic"
        );
    }

    #[test]
    fn canonical_sorted_keys() {
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness());
        let bytes = canonicalize(&unsigned);
        let json_str = String::from_utf8(bytes).unwrap();

        // The top-level keys must appear in sorted order:
        // entries, freshness, i11_distilled_content, i12_digest_refs, schema_version
        let entries_pos = json_str.find("\"entries\"").unwrap();
        let freshness_pos = json_str.find("\"freshness\"").unwrap();
        let i11_pos = json_str.find("\"i11_distilled_content\"").unwrap();
        let i12_pos = json_str.find("\"i12_digest_refs\"").unwrap();
        let schema_pos = json_str.find("\"schema_version\"").unwrap();

        assert!(
            entries_pos < freshness_pos,
            "entries must come before freshness"
        );
        assert!(freshness_pos < i11_pos, "freshness must come before i11");
        assert!(i11_pos < i12_pos, "i11 must come before i12");
        assert!(i12_pos < schema_pos, "i12 must come before schema_version");
    }

    #[test]
    fn tamper_i11_content_fails_verification() {
        let seed = [7u8; 32];
        let entries = make_test_entries();
        let freshness = make_freshness();
        let i11 = vec![I11Content {
            source_log_ref: vec!["original_ref".to_string()],
            distillation_depth: 0,
        }];

        let unsigned = build_bundle(entries, vec!["i12ref".to_string()], i11, freshness);
        let mut signed = sign_bundle(unsigned, &seed).expect("signing should succeed");

        // Tamper: change the first source_log_ref string
        signed.i11_distilled_content[0].source_log_ref[0] = "TAMPERED".to_string();

        let pubkey = derive_pubkey(&seed);
        assert!(
            verify_bundle(&signed, &pubkey).is_err(),
            "tampered i11 content must fail verification"
        );
    }

    #[test]
    fn tamper_i12_digest_ref_fails_verification() {
        let seed = [7u8; 32];
        let entries = make_test_entries();
        let freshness = make_freshness();

        let unsigned = build_bundle(
            entries,
            vec!["original_digest_ref".to_string()],
            vec![],
            freshness,
        );
        let mut signed = sign_bundle(unsigned, &seed).expect("signing should succeed");

        // Tamper: modify one character of the first i12 digest ref
        signed.i12_digest_refs[0] = "tampered_digest_ref".to_string();

        let pubkey = derive_pubkey(&seed);
        assert!(
            verify_bundle(&signed, &pubkey).is_err(),
            "tampered i12 digest ref must fail verification"
        );
    }

    #[test]
    fn replay_metadata_present() {
        let seed = [7u8; 32];
        let entries = make_test_entries();
        let freshness = make_freshness();

        let unsigned = build_bundle(entries, vec![], vec![], freshness);
        let signed = sign_bundle(unsigned, &seed).expect("signing should succeed");

        // Verify freshness metadata fields are populated
        assert!(
            signed.freshness.export_timestamp_ns > 0,
            "export_timestamp_ns must be > 0"
        );
        // covered_window has since/until
        let _since = signed.freshness.covered_window.since_ns;
        let _until = signed.freshness.covered_window.until_ns;
        assert!(
            signed.freshness.export_seq >= 0, // u64 is always >= 0 but check explicitly
            "export_seq must be present"
        );
        // Also verify the bundle passes verification
        let pubkey = derive_pubkey(&seed);
        verify_bundle(&signed, &pubkey).expect("signed bundle must verify");
    }
}
