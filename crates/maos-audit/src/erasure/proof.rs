//! Story 9.2 — signed proof-of-erasure bundle.
//!
//! Builds a Merkle inclusion + exclusion proof over TL frame_ids, attaches
//! per-category erasure status, signs the canonical bundle, and writes it to
//! the retention directory.  Verification is also provided for in-tree tests.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::merkle::{hash_leaf, build_tree_from_frame_ids, prove_exclusion, prove_inclusion, MerkleProof, NodeHash};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status")]
pub enum CategoryStatus {
    Removed { count: u64 },
    VerifiedEmpty,
    CoverageGap { reason: String },
}

/// One substrate category in the proof envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErasureCategory {
    pub name: String,
    pub status: CategoryStatus,
}

/// Signature block — mirrors the sealed-export convention from Story 9.1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignatureBlock {
    pub algorithm: String,
    pub attester_pubkey: String,
    pub signature: String,
}

/// Per-erased-frame proof: attests this distillate frame existed in the
/// pre-erasure Transparency-Log tree.  Combined with the appended
/// `distillate.redacted` marker (present in the post-tree), this proves the
/// frame was a genuine target of the body-scrub.  (AC3 inclusion: "erased
/// items WERE in the pre-erasure tree".)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErasedFrameProof {
    pub frame_id: String,
    pub pre_inclusion: MerkleProof,
}

/// Signed proof-of-erasure bundle.
///
/// # Security model (AC3, Decision D2)
///
/// The verifier **recomputes** `pre_root` and `post_root` from the leaf sets
/// and asserts equality with the claimed roots, so a tampered leaf list breaks
/// verification.  `erased_frame_proofs` prove each scrubbed distillate was
/// present in the pre-tree; `subject_exclusion_proofs` prove each erased
/// principal's canonical leaf is absent from the post-tree.  An empty proof
/// set on a bundle that claims erasure is rejected.
///
/// The residual bundle-only limitation is **completeness**: a third-party
/// verifier cannot know the leaf list is exhaustive without TL access.  The
/// in-process variant [`verify_erasure_proof_against_log`] closes that gap by
/// re-reading the live TL frame set.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErasureProof {
    pub schema_version: String,
    pub spirit_id: String,
    pub spirit_pid: u32,
    pub uninstalled_at_ns: u64,
    pub pre_root: NodeHash,
    pub post_root: NodeHash,
    pub pre_leaves: Vec<String>,
    pub post_leaves: Vec<String>,
    pub erased_frame_proofs: Vec<ErasedFrameProof>,
    pub subject_exclusion_proofs: Vec<MerkleProof>,
    pub categories: Vec<ErasureCategory>,
    pub signature_block: SignatureBlock,
}

/// Unsigned intermediate used for canonical serialization + signing.
#[derive(Debug, Clone, Serialize)]
struct ErasureProofForSigning {
    pub schema_version: String,
    pub spirit_id: String,
    pub spirit_pid: u32,
    pub uninstalled_at_ns: u64,
    pub pre_root: NodeHash,
    pub post_root: NodeHash,
    pub pre_leaves: Vec<String>,
    pub post_leaves: Vec<String>,
    pub erased_frame_proofs: Vec<ErasedFrameProof>,
    pub subject_exclusion_proofs: Vec<MerkleProof>,
    pub categories: Vec<ErasureCategory>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErasureProofError {
    #[error("signing error: {0}")]
    Signing(#[from] ed25519_dalek::SignatureError),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}

/// Hex-encode a 16-byte frame_id as eight colon-separated byte pairs — the
/// canonical format shared with `maos-iac::transparency_log::format_frame_id_hex`
/// and the standalone verifier.  (P26: a single canonical format across modules.)
fn format_frame_id(frame_id: &[u8; 16]) -> String {
    frame_id
        .chunks(2)
        .map(|chunk| format!("{:02x}{:02x}", chunk[0], chunk[1]))
        .collect::<Vec<_>>()
        .join(":")
}

fn parse_frame_id_hex(s: &str) -> Option<[u8; 16]> {
    let clean: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let bytes = hex::decode(clean).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

/// Canonicalize the unsigned proof for signing: sorted JSON keys, compact.
fn canonicalize(unsigned: &ErasureProofForSigning) -> Result<Vec<u8>, ErasureProofError> {
    let value = serde_json::to_value(unsigned).map_err(|e| ErasureProofError::Serialization(e.to_string()))?;
    let sorted = sort_value(value);
    serde_json::to_vec(&sorted).map_err(|e| ErasureProofError::Serialization(e.to_string()))
}

fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_value(v)))
                .collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(arr.into_iter().map(sort_value).collect()),
        other => other,
    }
}

/// Build a signed proof-of-erasure bundle.
///
/// * `pre_frame_ids` — all TL frame_ids present before the uninstall cascade.
/// * `post_frame_ids` — all TL frame_ids present after the cascade.
/// * `erased_frame_ids` — the distillate frames whose bodies were scrubbed;
///   each is proven present in the pre-tree (AC3 inclusion).  May be empty if
///   no distillates were touched.
/// * `erased_principal_ids` — the principals erased; each yields a canonical
///   subject-leaf exclusion proof against the post-tree (AC3 exclusion).
pub fn build_erasure_proof(
    spirit_id: String,
    spirit_pid: u32,
    uninstalled_at_ns: u64,
    pre_frame_ids: &[[u8; 16]],
    post_frame_ids: &[[u8; 16]],
    erased_frame_ids: &[[u8; 16]],
    erased_principal_ids: &[String],
    categories: Vec<ErasureCategory>,
    signing_seed: &[u8; 32],
) -> Result<ErasureProof, ErasureProofError> {
    let pre_tree = build_tree_from_frame_ids(pre_frame_ids);
    let post_tree = build_tree_from_frame_ids(post_frame_ids);

    // Inclusion: each erased distillate frame was present in the pre-tree.
    let mut erased_frame_proofs = Vec::with_capacity(erased_frame_ids.len());
    for fid in erased_frame_ids {
        let leaf = hash_leaf(fid);
        let proof = prove_inclusion(&pre_tree, leaf).ok_or_else(|| {
            ErasureProofError::VerificationFailed(
                "erased frame is absent from the pre-tree; cannot prove inclusion".to_string(),
            )
        })?;
        erased_frame_proofs.push(ErasedFrameProof {
            frame_id: format_frame_id(fid),
            pre_inclusion: proof,
        });
    }

    // Exclusion: each erased principal's canonical subject leaf is absent from
    // the post-tree.
    let mut subject_exclusion_proofs = Vec::new();
    for pid in erased_principal_ids {
        let leaf = canonical_exclusion_leaf(&spirit_id, std::slice::from_ref(pid));
        if post_tree.leaves.binary_search(&leaf).is_ok() {
            return Err(ErasureProofError::VerificationFailed(
                "canonical subject leaf collides with a post-tree leaf; cannot prove exclusion"
                    .to_string(),
            ));
        }
        if let Some(proof) = prove_exclusion(&post_tree, leaf) {
            subject_exclusion_proofs.push(proof);
        }
    }

    let pre_leaves = pre_frame_ids.iter().map(format_frame_id).collect();
    let post_leaves = post_frame_ids.iter().map(format_frame_id).collect();

    let unsigned = ErasureProofForSigning {
        schema_version: "maos.erasure-proof.v1".to_string(),
        spirit_id: spirit_id.clone(),
        spirit_pid,
        uninstalled_at_ns,
        pre_root: pre_tree.root,
        post_root: post_tree.root,
        pre_leaves,
        post_leaves,
        erased_frame_proofs,
        subject_exclusion_proofs,
        categories,
    };

    let canonical = canonicalize(&unsigned)?;
    let digest = Sha256::digest(&canonical);
    let signing_key = SigningKey::from_bytes(signing_seed);
    let signature = signing_key.sign(&digest);
    let pubkey_bytes = signing_key.verifying_key().to_bytes();

    Ok(ErasureProof {
        schema_version: unsigned.schema_version,
        spirit_id,
        spirit_pid,
        uninstalled_at_ns,
        pre_root: unsigned.pre_root,
        post_root: unsigned.post_root,
        pre_leaves: unsigned.pre_leaves,
        post_leaves: unsigned.post_leaves,
        erased_frame_proofs: unsigned.erased_frame_proofs,
        subject_exclusion_proofs: unsigned.subject_exclusion_proofs,
        categories: unsigned.categories,
        signature_block: SignatureBlock {
            algorithm: "Ed25519".to_string(),
            attester_pubkey: hex::encode(pubkey_bytes),
            signature: hex::encode(signature.to_bytes()),
        },
    })
}

/// Verify a proof bundle given the attester public key bytes (bundle-only).
///
/// Recomputes both Merkle roots from the leaf sets, re-verifies every
/// inclusion/exclusion proof, and rejects empty proof sets on bundles that
/// claim erasure.  Does NOT re-read the live TL — use
/// [`verify_erasure_proof_against_log`] for the completeness check that
/// catches a signer withholding frames.
pub fn verify_erasure_proof(
    proof: &ErasureProof,
    pubkey_bytes: &[u8; 32],
) -> Result<(), ErasureProofError> {
    let unsigned = ErasureProofForSigning {
        schema_version: proof.schema_version.clone(),
        spirit_id: proof.spirit_id.clone(),
        spirit_pid: proof.spirit_pid,
        uninstalled_at_ns: proof.uninstalled_at_ns,
        pre_root: proof.pre_root,
        post_root: proof.post_root,
        pre_leaves: proof.pre_leaves.clone(),
        post_leaves: proof.post_leaves.clone(),
        erased_frame_proofs: proof.erased_frame_proofs.clone(),
        subject_exclusion_proofs: proof.subject_exclusion_proofs.clone(),
        categories: proof.categories.clone(),
    };

    let canonical = canonicalize(&unsigned)?;
    let digest = Sha256::digest(&canonical);
    let pubkey = VerifyingKey::from_bytes(pubkey_bytes).map_err(|e| {
        ErasureProofError::VerificationFailed(format!("invalid pubkey: {e}"))
    })?;
    let signature_bytes = hex::decode(&proof.signature_block.signature)
        .map_err(|e| ErasureProofError::VerificationFailed(format!("bad signature hex: {e}")))?;
    let signature = ed25519_dalek::Signature::from_slice(&signature_bytes)
        .map_err(|e| ErasureProofError::VerificationFailed(format!("bad signature: {e}")))?;

    pubkey
        .verify(&digest, &signature)
        .map_err(|_| ErasureProofError::VerificationFailed("signature mismatch".to_string()))?;

    // Recompute both roots from the leaf sets — closes the root↔leaves tamper
    // gap (P9): a signer cannot pair an arbitrary root with a mismatched list.
    let pre_fids: Vec<[u8; 16]> = proof
        .pre_leaves
        .iter()
        .filter_map(|s| parse_frame_id_hex(s))
        .collect();
    let post_fids: Vec<[u8; 16]> = proof
        .post_leaves
        .iter()
        .filter_map(|s| parse_frame_id_hex(s))
        .collect();
    let pre_tree = build_tree_from_frame_ids(&pre_fids);
    let post_tree = build_tree_from_frame_ids(&post_fids);
    if pre_tree.root != proof.pre_root {
        return Err(ErasureProofError::VerificationFailed(
            "pre_root does not match the recomputed root of pre_leaves".to_string(),
        ));
    }
    if post_tree.root != proof.post_root {
        return Err(ErasureProofError::VerificationFailed(
            "post_root does not match the recomputed root of post_leaves".to_string(),
        ));
    }

    // Inclusion: each erased frame was in the pre-tree.
    for efp in &proof.erased_frame_proofs {
        let fid = parse_frame_id_hex(&efp.frame_id).ok_or_else(|| {
            ErasureProofError::VerificationFailed("malformed erased frame_id".to_string())
        })?;
        let leaf = hash_leaf(&fid);
        if !super::merkle::verify_proof(proof.pre_root, leaf, &efp.pre_inclusion) {
            return Err(ErasureProofError::VerificationFailed(
                "erased-frame pre-inclusion proof invalid".to_string(),
            ));
        }
    }

    // Exclusion: each canonical subject leaf is absent from the post-tree.
    for sp in &proof.subject_exclusion_proofs {
        if !super::merkle::verify_proof(proof.post_root, sp.leaf, sp) {
            return Err(ErasureProofError::VerificationFailed(
                "subject exclusion proof invalid".to_string(),
            ));
        }
        if post_fids.iter().any(|fid| hash_leaf(fid) == sp.leaf) {
            return Err(ErasureProofError::VerificationFailed(
                "subject exclusion leaf is present in the post-tree".to_string(),
            ));
        }
    }

    // Reject empty proof sets on a bundle that claims erasure (P18).
    let claims_removal = proof
        .categories
        .iter()
        .any(|c| matches!(c.status, CategoryStatus::Removed { count } if count > 0));
    if claims_removal
        && proof.erased_frame_proofs.is_empty()
        && proof.subject_exclusion_proofs.is_empty()
    {
        return Err(ErasureProofError::VerificationFailed(
            "bundle claims erasure but carries no inclusion or exclusion proofs".to_string(),
        ));
    }

    Ok(())
}

/// In-process verification that additionally re-reads the live TL frame sets
/// and asserts they equal the bundle's leaves — the AC3 completeness check
/// that catches a signer withholding frames (true exclusion-forgery
/// resistance, only available where the verifier has TL access).
pub fn verify_erasure_proof_against_log(
    proof: &ErasureProof,
    pubkey_bytes: &[u8; 32],
    expected_pre_frame_ids: &[[u8; 16]],
    expected_post_frame_ids: &[[u8; 16]],
) -> Result<(), ErasureProofError> {
    verify_erasure_proof(proof, pubkey_bytes)?;
    let mut bundle_pre: Vec<[u8; 16]> = proof
        .pre_leaves
        .iter()
        .filter_map(|s| parse_frame_id_hex(s))
        .collect();
    let mut bundle_post: Vec<[u8; 16]> = proof
        .post_leaves
        .iter()
        .filter_map(|s| parse_frame_id_hex(s))
        .collect();
    let mut exp_pre = expected_pre_frame_ids.to_vec();
    let mut exp_post = expected_post_frame_ids.to_vec();
    bundle_pre.sort();
    bundle_post.sort();
    exp_pre.sort();
    exp_post.sort();
    if bundle_pre != exp_pre {
        return Err(ErasureProofError::VerificationFailed(
            "bundle pre_leaves do not match the live pre-erasure TL frame set".to_string(),
        ));
    }
    if bundle_post != exp_post {
        return Err(ErasureProofError::VerificationFailed(
            "bundle post_leaves do not match the live post-erasure TL frame set".to_string(),
        ));
    }
    Ok(())
}

/// Write a signed proof bundle to the retention directory using an atomic
/// temp-file + rename so a crash mid-write cannot leave a truncated,
/// unverifiable bundle (P17).  The filename carries a short post-root suffix
/// so re-installs / colliding monotonic ns cannot overwrite a prior proof, and
/// the spirit_id is sanitized against path separators (P16).  Returns the path.
pub fn write_proof_bundle(
    proof: &ErasureProof,
    dir: &Path,
) -> Result<std::path::PathBuf, ErasureProofError> {
    std::fs::create_dir_all(dir)?;
    let safe_spirit: String = proof
        .spirit_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let root_suffix = hex::encode(&proof.post_root[0..8]);
    let filename = format!("{}-{}-{}.bundle", safe_spirit, proof.uninstalled_at_ns, root_suffix);
    let path = dir.join(&filename);
    let bytes = serde_json::to_vec_pretty(proof)
        .map_err(|e| ErasureProofError::Serialization(e.to_string()))?;
    let temp = dir.join(format!(".{}.tmp", filename));
    std::fs::write(&temp, &bytes)?;
    std::fs::rename(&temp, &path)?;
    Ok(path)
}

/// Derive the canonical exclusion leaf for an erased principal under a spirit.
/// The leaf is a domain-separated hash over the spirit + principal set; it is
/// never a real TL frame, so its exclusion proves the subject has no
/// self-tagged residual frame in the post-tree (not that any particular data
/// frame is deleted — the TL is append-only).
pub fn canonical_exclusion_leaf(spirit_id: &str, principal_ids: &[String]) -> NodeHash {
    let mut hasher = Sha256::new();
    hasher.update(b"maos.erasure.exclusion");
    hasher.update(spirit_id.as_bytes());
    for pid in principal_ids {
        hasher.update(b"\x00");
        hasher.update(pid.as_bytes());
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_round_trip() {
        let seed = [7u8; 32];
        // Pre-tree has 4 frames; the cascade adds one (the forget record) post.
        let pre: Vec<[u8; 16]> = (0u8..4).map(|i| [i; 16]).collect();
        let mut post = pre.clone();
        post.push([42; 16]);
        // Two of the pre-frames are distillates that got scrubbed.
        let erased: Vec<[u8; 16]> = vec![pre[0], pre[2]];
        let principals = vec!["alice@example.org".to_string()];
        let proof = build_erasure_proof(
            "spirit-1".into(),
            1,
            1234,
            &pre,
            &post,
            &erased,
            &principals,
            vec![
                ErasureCategory {
                    name: "memory_namespace".into(),
                    status: CategoryStatus::Removed { count: 3 },
                },
                ErasureCategory {
                    name: "scheduled_invocations".into(),
                    status: CategoryStatus::CoverageGap {
                        reason: "no per-spirit enumeration API".into(),
                    },
                },
            ],
            &seed,
        )
        .unwrap();

        let pubkey = ed25519_dalek::SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes();
        assert!(verify_erasure_proof(&proof, &pubkey).is_ok());
        // Against-log variant must also pass with the real frame sets.
        assert!(verify_erasure_proof_against_log(&proof, &pubkey, &pre, &post).is_ok());
    }

    #[test]
    fn erased_frame_absent_from_pre_tree_is_rejected() {
        // Builder-side forgery: claim a frame was erased that never existed.
        let seed = [9u8; 32];
        let pre: Vec<[u8; 16]> = (0u8..4).map(|i| [i; 16]).collect();
        let post = pre.clone();
        let never_existed = [250u8; 16];
        let result = build_erasure_proof(
            "spirit-2".into(),
            2,
            1,
            &pre,
            &post,
            &[never_existed],
            &[],
            vec![],
            &seed,
        );
        assert!(result.is_err(), "cannot prove inclusion of a frame absent from pre-tree");
    }

    #[test]
    fn tampered_post_root_is_rejected() {
        let seed = [3u8; 32];
        let pre: Vec<[u8; 16]> = (0u8..4).map(|i| [i; 16]).collect();
        let post = pre.clone();
        let mut proof = build_erasure_proof(
            "spirit-3".into(),
            3,
            9,
            &pre,
            &post,
            &[pre[0]],
            &["bob@example.org".to_string()],
            vec![ErasureCategory {
                name: "memory_namespace".into(),
                status: CategoryStatus::Removed { count: 1 },
            }],
            &seed,
        )
        .unwrap();
        // Flip a byte of post_root — recomputed root must no longer match.
        proof.post_root[0] ^= 0xff;
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes();
        assert!(
            verify_erasure_proof(&proof, &pubkey).is_err(),
            "a tampered post_root must fail recompute-equality"
        );
    }

    #[test]
    fn against_log_catches_withheld_frames() {
        let seed = [4u8; 32];
        let pre: Vec<[u8; 16]> = (0u8..4).map(|i| [i; 16]).collect();
        let post = pre.clone();
        let proof = build_erasure_proof(
            "spirit-4".into(),
            4,
            9,
            &pre,
            &post,
            &[pre[0]],
            &["carol@example.org".to_string()],
            vec![ErasureCategory {
                name: "memory_namespace".into(),
                status: CategoryStatus::Removed { count: 1 },
            }],
            &seed,
        )
        .unwrap();
        let pubkey = ed25519_dalek::SigningKey::from_bytes(&seed)
            .verifying_key()
            .to_bytes();
        // Bundle-only verification passes...
        assert!(verify_erasure_proof(&proof, &pubkey).is_ok());
        // ...but against-log with the WRONG post set must fail (withheld-frame forgery).
        let mut lying_post = post.clone();
        lying_post.push([99; 16]);
        assert!(
            verify_erasure_proof_against_log(&proof, &pubkey, &pre, &lying_post).is_err(),
            "against-log verification must catch a signer withholding frames"
        );
    }
}
