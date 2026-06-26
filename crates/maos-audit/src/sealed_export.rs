//! FR44 sealed-export: deterministic bundle serialization, Ed25519 signing,
//! and in-tree verification.
//!
//! Uses `ed25519-dalek` + `sha2` — NOT `ring` and NOT `maos-kernel-core`.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use maos_domain::region::Region;
use sha2::{Digest, Sha256};

// ─── Story 9.4b AC-5 — region-bound TL signing-key derivation ────────────────

/// HKDF-SHA256 salt domain-separator for region-bound TL signing-key derivation.
const REGION_TL_SIGNING_SALT: &[u8] = b"maos.region.tl-signing.v1";
/// HKDF `info` prefix binding the frozen region encoding id (AC-12 `ascii-v1`)
/// into the derivation context.  Two spellings of one region canonicalize to the
/// same bytes (see [`Region`]) and therefore derive the same key — by design.
const REGION_INFO_PREFIX: &[u8] = b"maos.region.ascii-v1:";

/// Derive a region-bound Ed25519 signing seed from a base seed (AC-5 / AC-12).
///
/// `region` is already-canonical (`ascii-v1`) by virtue of the [`Region`] type,
/// so this derivation is stable and unambiguous.  A bundle signed under one
/// region cannot be verified under another region's derived key (R-RG1), which
/// is the cryptographic root of region pinning at the export/TL layer.
pub fn derive_region_signing_seed(base_seed: &[u8; 32], region: &Region) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(REGION_TL_SIGNING_SALT), base_seed);
    let mut info = Vec::with_capacity(REGION_INFO_PREFIX.len() + region.as_bytes().len());
    info.extend_from_slice(REGION_INFO_PREFIX);
    info.extend_from_slice(region.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(&info, &mut okm)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    okm
}

/// Derive the region-bound Ed25519 *public* key — the expected attester key for
/// verifying a bundle pinned to `region` (the verify-side companion of
/// [`derive_region_signing_seed`]).
pub fn derive_region_pubkey(base_seed: &[u8; 32], region: &Region) -> [u8; 32] {
    derive_pubkey(&derive_region_signing_seed(base_seed, region))
}

// ─── Bundle types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditBundle {
    pub schema_version: String,
    pub entries: Vec<crate::AuditEntry>,
    pub i12_digest_refs: Vec<String>,
    pub i11_distilled_content: Vec<I11Content>,
    pub freshness: FreshnessMetadata,
    #[serde(default, skip_serializing_if = "is_false")]
    pub applied_redaction: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub redaction_policy: String,
    /// Story 9.4b AC-5 — canonical (`ascii-v1`) jurisdiction tag this bundle is
    /// region-pinned to.  `None` for pre-region (v2) bundles — omitted from the
    /// canonical bytes so region-less exports stay byte-identical (AC-11 /
    /// R-SCH compat).  When `Some`, the signing key is HKDF-derived from this
    /// tag, so tampering it breaks verification (R-RG4′).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub applied_redaction: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub redaction_policy: String,
    /// Story 9.4b AC-5 — region tag covered by the signature.  `None` is omitted
    /// from canonical bytes (byte-identity preserved); `Some` is signed, so a
    /// post-sign tamper of the region field fails verification (R-RG4′).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl BundleForSigning {
    /// Region-pin this bundle to `region` (Story 9.4b AC-5).  The region is
    /// covered by the canonical bytes AND drives HKDF derivation of the signing
    /// key in [`sign_bundle`].
    pub fn with_region(mut self, region: &Region) -> Self {
        self.region = Some(region.as_str().to_string());
        self
    }
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
///
/// Used for the 9.1 `maos.audit-bundle.v1` surface. Redaction fields are left
/// at their default values so they are omitted from the canonical bytes,
/// preserving the 9.1 byte-identity contract.
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
        applied_redaction: false,
        redaction_policy: String::new(),
        region: None,
    }
}

/// Build a `BundleForSigning` for a `maos.trajectory.v1` export.
///
/// Unlike [`build_bundle`], this populates `applied_redaction` and
/// `redaction_policy` so they are covered by the Ed25519 signature.
pub fn build_trajectory_bundle(
    entries: Vec<crate::AuditEntry>,
    i12_refs: Vec<String>,
    i11_content: Vec<I11Content>,
    freshness: FreshnessMetadata,
    applied_redaction: bool,
    redaction_policy: String,
) -> BundleForSigning {
    BundleForSigning {
        schema_version: "maos.trajectory.v1".to_string(),
        entries,
        i12_digest_refs: i12_refs,
        i11_distilled_content: i11_content,
        freshness,
        applied_redaction,
        redaction_policy,
        region: None,
    }
}

/// Sign the canonical bundle bytes with Ed25519.
///
/// Computes sha256(canonical_bytes), signs with the given seed,
/// and returns the complete signed `AuditBundle`.
pub fn sign_bundle(
    bundle_for_signing: BundleForSigning,
    seed: &[u8; 32],
) -> Result<AuditBundle, SealedExportError> {
    let canonical = canonicalize(&bundle_for_signing)?;
    let digest = Sha256::digest(&canonical);

    // AC-5: when region-pinned, sign with the HKDF-derived region key so the
    // bundle only verifies under that region's derived attester key (R-RG1).
    let effective_seed = match &bundle_for_signing.region {
        Some(tag) => {
            let region = Region::canonicalize(tag).map_err(|e| {
                SealedExportError::Serialization(format!("invalid region tag: {e}"))
            })?;
            derive_region_signing_seed(seed, &region)
        }
        None => *seed,
    };
    let signing_key = SigningKey::from_bytes(&effective_seed);
    let signature = signing_key.sign(&digest);
    let pubkey_bytes = signing_key.verifying_key().to_bytes();

    Ok(AuditBundle {
        schema_version: bundle_for_signing.schema_version,
        entries: bundle_for_signing.entries,
        i12_digest_refs: bundle_for_signing.i12_digest_refs,
        i11_distilled_content: bundle_for_signing.i11_distilled_content,
        freshness: bundle_for_signing.freshness,
        applied_redaction: bundle_for_signing.applied_redaction,
        redaction_policy: bundle_for_signing.redaction_policy,
        region: bundle_for_signing.region,
        signature_block: SignatureBlock {
            algorithm: "Ed25519".to_string(),
            attester_pubkey: hex::encode(pubkey_bytes),
            signature: hex::encode(signature.to_bytes()),
        },
    })
}

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
        applied_redaction: bundle.applied_redaction,
        redaction_policy: bundle.redaction_policy.clone(),
        // AC-5/R-RG4′: region is covered by the signature — a post-sign tamper
        // changes the recomputed digest and verification fails.
        region: bundle.region.clone(),
    };

    let canonical = canonicalize(&unsigned)?;
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

/// Serialize any serializable value to canonical bytes (sorted keys, no whitespace).
///
/// Public so that `replay::runner` and `maosctl audit replay` can reuse the same
/// canonicalizer — ADR-028 D5b (one canonicalizer, not three).
pub fn canonicalize_value<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, SealedExportError> {
    let value =
        serde_json::to_value(value).map_err(|e| SealedExportError::Serialization(e.to_string()))?;
    let sorted = sort_value(value);
    serde_json::to_string(&sorted)
        .map_err(|e| SealedExportError::Serialization(e.to_string()))
        .map(|s| s.into_bytes())
}

/// Deterministic canonical serialization: sorted keys, no insignificant whitespace.
///
/// Serializes to `serde_json::Value`, recursively sorts all object keys via
/// `BTreeMap` ordering, then outputs compact JSON. Ensures byte-identical
/// output regardless of struct field declaration order.
pub fn canonicalize(bundle: &BundleForSigning) -> Result<Vec<u8>, SealedExportError> {
    canonicalize_value(bundle)
}

/// Recursively sort all JSON object keys using BTreeMap for deterministic order.
///
/// Public so that callers can canonicalize arbitrary `serde_json::Value` shapes
/// with the same ordering rules (e.g., `maosctl audit replay` over an untrusted
/// bundle read from disk).
pub fn sort_value(value: serde_json::Value) -> serde_json::Value {
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
            payload: String::new(),
            redaction: None,
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

    // ─── Story 9.4b AC-5 region-binding gates ──────────────────────────────

    fn region(tag: &str) -> Region {
        Region::canonicalize(tag).unwrap()
    }

    /// R-RG1 (MERGE-BLOCKING) — same-input-opposite-verdict: a bundle pinned to
    /// the home region verifies under the home-derived attester key; the SAME
    /// bundle is rejected under a foreign region's derived key.
    #[test]
    fn r_rg1_home_allow_foreign_region_violation() {
        let seed = [7u8; 32];
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness())
            .with_region(&region("eu"));
        let signed = sign_bundle(unsigned, &seed).unwrap();

        // home (eu) attester key -> ALLOW
        let eu_pub = derive_region_pubkey(&seed, &region("eu"));
        assert!(
            verify_bundle(&signed, &eu_pub).is_ok(),
            "home region must verify"
        );

        // foreign (us) attester key -> region violation (verification fails)
        let us_pub = derive_region_pubkey(&seed, &region("us"));
        assert!(
            matches!(
                verify_bundle(&signed, &us_pub),
                Err(SealedExportError::VerificationFailed)
            ),
            "foreign-region key must fail verification (R-RG1)"
        );
    }

    /// R-RG4′ (MERGE-BLOCKING) — cryptographic-binding bite: tampering the
    /// region tag in a signed bundle breaks verification (the region is covered
    /// by the signed digest), NOT merely a region-field string check.
    #[test]
    fn r_rg4_prime_region_tamper_breaks_verification() {
        let seed = [7u8; 32];
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness())
            .with_region(&region("eu"));
        let mut signed = sign_bundle(unsigned, &seed).unwrap();
        let eu_pub = derive_region_pubkey(&seed, &region("eu"));
        assert!(verify_bundle(&signed, &eu_pub).is_ok());

        // Attacker rewrites the region field post-seal.
        signed.region = Some("us".to_string());
        assert!(
            verify_bundle(&signed, &eu_pub).is_err(),
            "region tamper must fail verification (R-RG4′)"
        );
        // ...and it does not verify under the tampered region's key either.
        let us_pub = derive_region_pubkey(&seed, &region("us"));
        assert!(verify_bundle(&signed, &us_pub).is_err());
    }

    /// AC-11 / R-SCH — byte-identity preserved: a region-less (`None`) bundle
    /// serializes WITHOUT a `region` key, so pre-region exports stay byte-for-
    /// byte identical (9.2b HARD byte-identity replay not disturbed).
    #[test]
    fn r_sch_region_none_omitted_from_canonical_bytes() {
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness());
        assert!(unsigned.region.is_none());
        let canonical = String::from_utf8(canonicalize(&unsigned).unwrap()).unwrap();
        assert!(
            !canonical.contains("region"),
            "region-less bundle must not emit a region key: {canonical}"
        );
    }

    /// AC-11 / R-SCH3 — backward compat: a v2 bundle JSON with NO region field
    /// deserializes (region = None) and verifies under the raw (non-derived)
    /// seed, exactly as before this story.
    #[test]
    fn r_sch3_v2_no_region_field_verifies_with_raw_seed() {
        let seed = [3u8; 32];
        // Sign with NO region -> raw seed path (the pre-9.4b behavior).
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness());
        let signed = sign_bundle(unsigned, &seed).unwrap();
        // Round-trip through JSON (a "v2 on disk" bundle has no region key).
        let json = serde_json::to_string(&signed).unwrap();
        assert!(!json.contains("\"region\""), "v2 bundle must omit region");
        let reparsed: AuditBundle = serde_json::from_str(&json).unwrap();
        assert!(reparsed.region.is_none());
        let raw_pub = derive_pubkey(&seed);
        assert!(verify_bundle(&reparsed, &raw_pub).is_ok());
    }

    /// AC-12 — two spellings of one region derive the IDENTICAL signing key
    /// (the irreversible failure class this story guards against).
    #[test]
    fn ac12_two_spellings_derive_identical_key() {
        let seed = [9u8; 32];
        let a = derive_region_signing_seed(&seed, &region("US-EAST-1"));
        let b = derive_region_signing_seed(&seed, &region("us-east-1"));
        assert_eq!(a, b);
        // Different regions derive different keys.
        let c = derive_region_signing_seed(&seed, &region("eu-west-1"));
        assert_ne!(a, c);
    }

    /// R-SCH2 — round-trip: a region-pinned bundle survives serialize/
    /// deserialize WITHOUT dropping the region tag, and still verifies.
    #[test]
    fn r_sch2_region_bundle_roundtrip_no_field_drop() {
        let seed = [5u8; 32];
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness())
            .with_region(&region("ap-northeast-1"));
        let signed = sign_bundle(unsigned, &seed).unwrap();
        let json = serde_json::to_string(&signed).unwrap();
        let reparsed: AuditBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed.region.as_deref(), Some("ap-northeast-1"));
        let pubkey = derive_region_pubkey(&seed, &region("ap-northeast-1"));
        assert!(verify_bundle(&reparsed, &pubkey).is_ok());
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
        let bytes1 = canonicalize(&unsigned).unwrap();
        let bytes2 = canonicalize(&unsigned).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "canonical serialization must be deterministic"
        );
    }

    #[test]
    fn canonical_sorted_keys() {
        let unsigned = build_bundle(make_test_entries(), vec![], vec![], make_freshness());
        let bytes = canonicalize(&unsigned).unwrap();
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
