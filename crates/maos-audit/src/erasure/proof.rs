//! Signed erasure inventory. Counts bind to real typed component evidence;
//! completion is a durable TL transaction cutoff, never handler return time.
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::merkle::{
    build_tree_from_frame_ids, hash_leaf, prove_exclusion, prove_inclusion, verify_exclusion_proof,
    verify_inclusion_proof, MerkleProof, MerkleTree, NodeHash,
};

pub const MAX_BUNDLE_BYTES: usize = 128 * 1024 * 1024;
pub const SUBJECT_COMMITMENT_SCHEME: &str = "hkdf-sha256-signer-keyed-v1";
pub const COMPLETION_BOUNDARY: &str = "durable_transactional_marker";
pub const REQUIRED_CATEGORIES: [&str; 16] = [
    "memory_namespace",
    "principal_index",
    "capability_tokens",
    "shared",
    "private",
    "principal_frames",
    "intent_lineage",
    "pending_halts",
    "scheduled_invocations",
    "manifest_archive",
    "skill_queue",
    "lifecycle_journal",
    "worker_workspaces",
    "installed_package",
    "legal_hold",
    "region_signing_key",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum CategoryStatus {
    Removed { count: u64 },
    VerifiedEmpty,
    CoverageGap { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErasureCategory {
    pub name: String,
    pub status: CategoryStatus,
}

/// Item ids where the component exposes them; otherwise the actual returned
/// batch receipt/count. No fabricated ids or repeated evidence substitute for n.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemovalEvidence {
    Frame {
        frame_id: String,
    },
    Package {
        spirit_id: String,
        generation: String,
    },
    Schedule {
        schedule_id: String,
    },
    MemoryNamespaceBatch {
        receipt_frame_id: String,
        deleted_entries: u64,
    },
    PrincipalIndexBatch {
        receipt_frame_id: String,
        deleted_index_rows: u64,
    },
    CapabilityBatch {
        boot_nonce: u64,
        spirit_pid: u32,
        revoked_tokens: u64,
    },
}

/// Producer-side observation, coupled before signing rather than parallel lists.
#[derive(Debug)]
pub struct CategoryObservation {
    pub category: ErasureCategory,
    pub frame_id: [u8; 16],
    pub removals: Vec<RemovalEvidence>,
}
impl CategoryObservation {
    pub fn new(
        category: ErasureCategory,
        frame_id: [u8; 16],
        removals: Vec<RemovalEvidence>,
    ) -> Self {
        Self {
            category,
            frame_id,
            removals,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureBlock {
    pub algorithm: String,
    pub attester_pubkey: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErasedFrameProof {
    pub frame_id: String,
    pub pre_inclusion: MerkleProof,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryEvidence {
    pub spirit_id: String,
    pub name: String,
    pub frame_id: String,
    pub post_inclusion: MerkleProof,
    pub removals: Vec<RemovalEvidence>,
}

#[derive(Debug, Clone, Copy)]
pub struct CompletionObservation {
    pub frame_id: [u8; 16],
    pub through_rowid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameSweepCompletion {
    pub frame_id: String,
    pub through_rowid: u64,
    pub post_inclusion: MerkleProof,
}

/// Leaves attest frame identities, not payload bytes. The pre-set is the initial
/// snapshot plus late items observed before their individual redaction. Subject
/// leaves are secret-signer-keyed, bundle-bound commitments, not dictionary hashes.
/// A marker certifies matching pre-marker commits only; subsequent commits are
/// post-completion data. Coverage gaps make this a partial inventory, not a claim
/// that every substrate was erased. Bundle-only verification trusts the signer
/// about observations; against-log verification additionally binds the frame sets.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub redacted_principal_frame_proofs: Vec<ErasedFrameProof>,
    pub subject_exclusion_proofs: Vec<MerkleProof>,
    pub subject_count: u64,
    pub subject_commitment_scheme: String,
    pub completion_boundary: String,
    #[serde(deserialize_with = "super::merkle::required_option")]
    pub frame_sweep_completion: Option<FrameSweepCompletion>,
    pub categories: Vec<ErasureCategory>,
    pub category_evidence: Vec<CategoryEvidence>,
    pub signature_block: SignatureBlock,
}

#[derive(Serialize)]
struct ErasureProofForSigning<'a> {
    schema_version: &'a str,
    spirit_id: &'a str,
    spirit_pid: u32,
    uninstalled_at_ns: u64,
    pre_root: &'a NodeHash,
    post_root: &'a NodeHash,
    pre_leaves: &'a [String],
    post_leaves: &'a [String],
    erased_frame_proofs: &'a [ErasedFrameProof],
    redacted_principal_frame_proofs: &'a [ErasedFrameProof],
    subject_exclusion_proofs: &'a [MerkleProof],
    subject_count: u64,
    subject_commitment_scheme: &'a str,
    completion_boundary: &'a str,
    frame_sweep_completion: &'a Option<FrameSweepCompletion>,
    categories: &'a [ErasureCategory],
    category_evidence: &'a [CategoryEvidence],
}
impl<'a> From<&'a ErasureProof> for ErasureProofForSigning<'a> {
    fn from(proof: &'a ErasureProof) -> Self {
        Self {
            schema_version: &proof.schema_version,
            spirit_id: &proof.spirit_id,
            spirit_pid: proof.spirit_pid,
            uninstalled_at_ns: proof.uninstalled_at_ns,
            pre_root: &proof.pre_root,
            post_root: &proof.post_root,
            pre_leaves: &proof.pre_leaves,
            post_leaves: &proof.post_leaves,
            erased_frame_proofs: &proof.erased_frame_proofs,
            redacted_principal_frame_proofs: &proof.redacted_principal_frame_proofs,
            subject_exclusion_proofs: &proof.subject_exclusion_proofs,
            subject_count: proof.subject_count,
            subject_commitment_scheme: &proof.subject_commitment_scheme,
            completion_boundary: &proof.completion_boundary,
            frame_sweep_completion: &proof.frame_sweep_completion,
            categories: &proof.categories,
            category_evidence: &proof.category_evidence,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ErasureProofError {
    #[error("signing error: {0}")]
    Signing(#[from] ed25519_dalek::SignatureError),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("proof is present at {path}, but directory durability failed: {source}")]
    PublishedButUnsynced {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}
impl ErasureProofError {
    pub fn published_path(&self) -> Option<&Path> {
        match self {
            Self::PublishedButUnsynced { path, .. } => Some(path),
            _ => None,
        }
    }
}
fn invalid(message: &str) -> ErasureProofError {
    ErasureProofError::VerificationFailed(message.into())
}

fn format_frame_id(frame_id: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(39);
    for (index, byte) in frame_id.iter().enumerate() {
        if index > 0 && index % 2 == 0 {
            output.push(':');
        }
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 15)]));
    }
    output
}

fn parse_frame_id_hex(value: &str) -> Result<[u8; 16], ErasureProofError> {
    let bytes = value.as_bytes();
    if bytes.len() != 32 && bytes.len() != 39 {
        return Err(invalid("malformed frame id"));
    }
    let mut output = [0_u8; 16];
    let mut digit = 0;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if bytes.len() == 39 && index % 5 == 4 {
            if byte != b':' {
                return Err(invalid("malformed frame id separator"));
            }
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(invalid("malformed frame id hex")),
        };
        if digit % 2 == 0 {
            output[digit / 2] = nibble << 4;
        } else {
            output[digit / 2] |= nibble;
        }
        digit += 1;
    }
    Ok(output)
}

fn canonicalize(proof: &ErasureProof) -> Result<Vec<u8>, ErasureProofError> {
    fn sort(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.sort_keys();
                for value in map.values_mut() {
                    sort(value);
                }
            }
            serde_json::Value::Array(array) => {
                for value in array {
                    sort(value);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(ErasureProofForSigning::from(proof))
        .map_err(|error| ErasureProofError::Serialization(error.to_string()))?;
    sort(&mut value);
    serde_json::to_vec(&value).map_err(|error| ErasureProofError::Serialization(error.to_string()))
}

fn subject_commitment(
    seed: &[u8; 32],
    spirit: &str,
    subject: &str,
    post_root: &NodeHash,
    at_ns: u64,
) -> Result<NodeHash, ErasureProofError> {
    let mut context = Sha256::new();
    context.update(b"maos.erasure.subject-commitment.v1");
    context.update((spirit.len() as u64).to_le_bytes());
    context.update(spirit.as_bytes());
    context.update((subject.len() as u64).to_le_bytes());
    context.update(subject.as_bytes());
    context.update(at_ns.to_le_bytes());
    let mut leaf = [0; 32];
    hkdf::Hkdf::<Sha256>::new(Some(post_root), seed)
        .expand(&context.finalize(), &mut leaf)
        .map_err(|_| invalid("subject commitment output length"))?;
    Ok(leaf)
}

fn frame_proofs(
    tree: &MerkleTree,
    frames: &[[u8; 16]],
) -> Result<Vec<ErasedFrameProof>, ErasureProofError> {
    let mut seen = BTreeSet::new();
    frames
        .iter()
        .map(|frame| {
            if !seen.insert(frame) {
                return Err(invalid("duplicate erased frame"));
            }
            Ok(ErasedFrameProof {
                frame_id: format_frame_id(frame),
                pre_inclusion: prove_inclusion(tree, hash_leaf(frame))
                    .ok_or_else(|| invalid("erased frame absent from pre-tree"))?,
            })
        })
        .collect()
}

pub fn build_erasure_proof(
    spirit_id: String,
    spirit_pid: u32,
    uninstalled_at_ns: u64,
    pre_frame_ids: &[[u8; 16]],
    post_frame_ids: &[[u8; 16]],
    erased_frame_ids: &[[u8; 16]],
    subject_ids: &[String],
    redacted_principal_frame_ids: &[[u8; 16]],
    observations: Vec<CategoryObservation>,
    completion: Option<CompletionObservation>,
    signing_seed: &[u8; 32],
) -> Result<ErasureProof, ErasureProofError> {
    let pre_tree = build_tree_from_frame_ids(pre_frame_ids);
    let post_tree = build_tree_from_frame_ids(post_frame_ids);
    let mut unique_subjects = BTreeSet::new();
    let subject_exclusion_proofs = subject_ids
        .iter()
        .map(|subject| {
            if !unique_subjects.insert(subject) {
                return Err(invalid("duplicate subject"));
            }
            let leaf = subject_commitment(
                signing_seed,
                &spirit_id,
                subject,
                &post_tree.root,
                uninstalled_at_ns,
            )?;
            prove_exclusion(&post_tree, leaf)
                .ok_or_else(|| invalid("subject commitment has no exclusion proof"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut categories = Vec::with_capacity(observations.len());
    let mut category_evidence = Vec::with_capacity(observations.len());
    for observation in observations {
        let post_inclusion = prove_inclusion(&post_tree, hash_leaf(&observation.frame_id))
            .ok_or_else(|| invalid("category observation absent from post-tree"))?;
        category_evidence.push(CategoryEvidence {
            spirit_id: spirit_id.clone(),
            name: observation.category.name.clone(),
            frame_id: format_frame_id(&observation.frame_id),
            post_inclusion,
            removals: observation.removals,
        });
        categories.push(observation.category);
    }
    let frame_sweep_completion = completion
        .map(|completion| -> Result<_, ErasureProofError> {
            Ok(FrameSweepCompletion {
                frame_id: format_frame_id(&completion.frame_id),
                through_rowid: completion.through_rowid,
                post_inclusion: prove_inclusion(&post_tree, hash_leaf(&completion.frame_id))
                    .ok_or_else(|| invalid("completion marker absent from post-tree"))?,
            })
        })
        .transpose()?;
    let signing_key = SigningKey::from_bytes(signing_seed);
    let mut proof = ErasureProof {
        schema_version: "maos.erasure-proof.v2".into(),
        spirit_id,
        spirit_pid,
        uninstalled_at_ns,
        pre_root: pre_tree.root,
        post_root: post_tree.root,
        pre_leaves: pre_frame_ids.iter().map(format_frame_id).collect(),
        post_leaves: post_frame_ids.iter().map(format_frame_id).collect(),
        erased_frame_proofs: frame_proofs(&pre_tree, erased_frame_ids)?,
        redacted_principal_frame_proofs: frame_proofs(&pre_tree, redacted_principal_frame_ids)?,
        subject_exclusion_proofs,
        subject_count: subject_ids.len() as u64,
        subject_commitment_scheme: SUBJECT_COMMITMENT_SCHEME.into(),
        completion_boundary: COMPLETION_BOUNDARY.into(),
        frame_sweep_completion,
        categories,
        category_evidence,
        signature_block: SignatureBlock {
            algorithm: "Ed25519".into(),
            attester_pubkey: hex::encode(signing_key.verifying_key().to_bytes()),
            signature: String::new(),
        },
    };
    ensure_size(&proof, MAX_BUNDLE_BYTES - 128)?;
    validate_body(&proof, &pre_tree, &post_tree)?;
    proof.signature_block.signature = hex::encode(
        signing_key
            .sign(&Sha256::digest(canonicalize(&proof)?))
            .to_bytes(),
    );
    Ok(proof)
}

fn validate_frame_proofs(
    tree: &MerkleTree,
    proofs: &[ErasedFrameProof],
) -> Result<BTreeSet<[u8; 16]>, ErasureProofError> {
    let mut frames = BTreeSet::new();
    for proof in proofs {
        let frame = parse_frame_id_hex(&proof.frame_id)?;
        if !frames.insert(frame)
            || !verify_inclusion_proof(tree, hash_leaf(&frame), &proof.pre_inclusion)
        {
            return Err(invalid("duplicate or invalid erased-frame pre-inclusion"));
        }
    }
    Ok(frames)
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum RemovalIdentity<'a> {
    Frame([u8; 16]),
    Package(&'a str),
    Schedule(&'a str),
    Batch([u8; 16]),
    Capability(u64, u32),
}

fn removal_count(
    evidence: &CategoryEvidence,
    spirit: &str,
    post: &MerkleTree,
    frames: &BTreeSet<[u8; 16]>,
) -> Result<u64, ErasureProofError> {
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    for item in &evidence.removals {
        let (identity, count) = match (evidence.name.as_str(), item) {
            ("principal_frames" | "intent_lineage", RemovalEvidence::Frame { frame_id }) => {
                let frame = parse_frame_id_hex(frame_id)?;
                if !frames.contains(&frame) {
                    return Err(invalid("removed frame lacks matching pre-inclusion"));
                }
                (RemovalIdentity::Frame(frame), 1)
            }
            (
                "installed_package",
                RemovalEvidence::Package {
                    spirit_id,
                    generation,
                },
            ) if spirit_id == spirit && !generation.is_empty() => {
                (RemovalIdentity::Package(generation), 1)
            }
            ("scheduled_invocations", RemovalEvidence::Schedule { schedule_id })
                if !schedule_id.is_empty() =>
            {
                (RemovalIdentity::Schedule(schedule_id), 1)
            }
            (
                "memory_namespace",
                RemovalEvidence::MemoryNamespaceBatch {
                    receipt_frame_id,
                    deleted_entries,
                },
            ) => {
                let frame = parse_frame_id_hex(receipt_frame_id)?;
                if post.leaves.binary_search(&hash_leaf(&frame)).is_err() {
                    return Err(invalid("memory receipt absent from post-tree"));
                }
                (RemovalIdentity::Batch(frame), *deleted_entries)
            }
            (
                "principal_index",
                RemovalEvidence::PrincipalIndexBatch {
                    receipt_frame_id,
                    deleted_index_rows,
                },
            ) => {
                let frame = parse_frame_id_hex(receipt_frame_id)?;
                if post.leaves.binary_search(&hash_leaf(&frame)).is_err() {
                    return Err(invalid("index receipt absent from post-tree"));
                }
                (RemovalIdentity::Batch(frame), *deleted_index_rows)
            }
            (
                "capability_tokens",
                RemovalEvidence::CapabilityBatch {
                    boot_nonce,
                    spirit_pid,
                    revoked_tokens,
                },
            ) => (
                RemovalIdentity::Capability(*boot_nonce, *spirit_pid),
                *revoked_tokens,
            ),
            _ => return Err(invalid("removal evidence kind/identity mismatch")),
        };
        if !seen.insert(identity) {
            return Err(invalid("repeated removal evidence"));
        }
        total = total
            .checked_add(count)
            .ok_or_else(|| invalid("removal count overflow"))?;
    }
    if matches!(
        evidence.name.as_str(),
        "principal_frames" | "intent_lineage"
    ) && seen.len() != frames.len()
    {
        return Err(invalid("frame removal evidence omits a declared redaction"));
    }
    Ok(total)
}

fn validate_body(
    proof: &ErasureProof,
    pre: &MerkleTree,
    post: &MerkleTree,
) -> Result<(), ErasureProofError> {
    if proof.schema_version != "maos.erasure-proof.v2"
        || proof.subject_commitment_scheme != SUBJECT_COMMITMENT_SCHEME
        || proof.completion_boundary != COMPLETION_BOUNDARY
        || pre.root != proof.pre_root
        || post.root != proof.post_root
        || pre.leaves.len() != proof.pre_leaves.len()
        || post.leaves.len() != proof.post_leaves.len()
    {
        return Err(invalid("schema, roots, or unique leaf-set mismatch"));
    }
    let mut following = post.leaves.iter().peekable();
    for leaf in &pre.leaves {
        while following.peek().is_some_and(|candidate| *candidate < leaf) {
            following.next();
        }
        if following.next() != Some(leaf) {
            return Err(invalid("append-only frame-set violation"));
        }
    }
    let erased = validate_frame_proofs(pre, &proof.erased_frame_proofs)?;
    let principal_frames = validate_frame_proofs(pre, &proof.redacted_principal_frame_proofs)?;
    if !erased.is_disjoint(&principal_frames) {
        return Err(invalid(
            "one frame cannot be counted as two different removal kinds",
        ));
    }
    let mut subjects = BTreeSet::new();
    for subject in &proof.subject_exclusion_proofs {
        if !subjects.insert(subject.leaf) || !verify_exclusion_proof(post, subject) {
            return Err(invalid("duplicate or invalid subject exclusion proof"));
        }
    }
    if proof.subject_count != proof.subject_exclusion_proofs.len() as u64
        || (!principal_frames.is_empty() && proof.subject_count == 0)
    {
        return Err(invalid("subject count/exclusion evidence mismatch"));
    }
    if proof.categories.len() != REQUIRED_CATEGORIES.len()
        || proof.category_evidence.len() != REQUIRED_CATEGORIES.len()
    {
        return Err(invalid("complete mandatory category inventory required"));
    }
    let mut names = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut frame_gap = false;
    for (category, evidence) in proof.categories.iter().zip(&proof.category_evidence) {
        let frame = parse_frame_id_hex(&evidence.frame_id)?;
        if !REQUIRED_CATEGORIES.contains(&category.name.as_str())
            || !names.insert(category.name.as_str())
            || evidence.name != category.name
            || evidence.spirit_id != proof.spirit_id
            || !evidence_ids.insert(frame)
            || !verify_inclusion_proof(post, hash_leaf(&frame), &evidence.post_inclusion)
        {
            return Err(invalid("invalid category observation binding"));
        }
        let frames = if category.name == "intent_lineage" {
            &erased
        } else {
            &principal_frames
        };
        let count = removal_count(evidence, &proof.spirit_id, post, frames)?;
        match &category.status {
            CategoryStatus::Removed { count: claimed } if count != *claimed => {
                return Err(invalid(
                    "Removed count differs from matching typed evidence",
                ))
            }
            CategoryStatus::VerifiedEmpty if count != 0 || !evidence.removals.is_empty() => {
                return Err(invalid("VerifiedEmpty carries removal claims"))
            }
            CategoryStatus::CoverageGap { .. }
                if matches!(
                    category.name.as_str(),
                    "principal_frames" | "intent_lineage"
                ) =>
            {
                frame_gap = true
            }
            _ => {}
        }
    }
    match &proof.frame_sweep_completion {
        None if frame_gap => {}
        Some(completion) => {
            let frame = parse_frame_id_hex(&completion.frame_id)?;
            if frame_gap
                || evidence_ids.contains(&frame)
                || pre.leaves.binary_search(&hash_leaf(&frame)).is_ok()
                || !verify_inclusion_proof(post, hash_leaf(&frame), &completion.post_inclusion)
            {
                return Err(invalid("invalid durable frame-sweep completion marker"));
            }
        }
        None => return Err(invalid("uncertified frame sweep must be a coverage gap")),
    }
    Ok(())
}

/// Size-check raw JSON before deserialization; exact DTOs reject unknown and
/// duplicate fields. Programmatic verification also bounds serialized size.
pub fn decode_proof_bundle(bytes: &[u8]) -> Result<ErasureProof, ErasureProofError> {
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(invalid("bundle exceeds 128 MiB"));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| ErasureProofError::Serialization(error.to_string()))
}

pub fn verify_erasure_proof(
    proof: &ErasureProof,
    pubkey_bytes: &[u8; 32],
) -> Result<(), ErasureProofError> {
    ensure_size(proof, MAX_BUNDLE_BYTES)?;
    if proof.signature_block.algorithm != "Ed25519" {
        return Err(invalid("unsupported signature algorithm"));
    }
    let mut declared_key = [0; 32];
    hex::decode_to_slice(&proof.signature_block.attester_pubkey, &mut declared_key)
        .map_err(|_| invalid("malformed attester key"))?;
    if declared_key != *pubkey_bytes {
        return Err(invalid("pinned attester key mismatch"));
    }
    let mut signature = [0; 64];
    hex::decode_to_slice(&proof.signature_block.signature, &mut signature)
        .map_err(|_| invalid("malformed signature"))?;
    VerifyingKey::from_bytes(pubkey_bytes)?
        .verify(
            &Sha256::digest(canonicalize(proof)?),
            &ed25519_dalek::Signature::from_bytes(&signature),
        )
        .map_err(|_| invalid("signature mismatch"))?;
    let pre = proof
        .pre_leaves
        .iter()
        .map(|value| parse_frame_id_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    let post = proof
        .post_leaves
        .iter()
        .map(|value| parse_frame_id_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    validate_body(
        proof,
        &build_tree_from_frame_ids(&pre),
        &build_tree_from_frame_ids(&post),
    )
}

pub fn verify_erasure_proof_against_log(
    proof: &ErasureProof,
    pubkey_bytes: &[u8; 32],
    expected_pre_frame_ids: &[[u8; 16]],
    expected_post_frame_ids: &[[u8; 16]],
) -> Result<(), ErasureProofError> {
    verify_erasure_proof(proof, pubkey_bytes)?;
    let mut pre = proof
        .pre_leaves
        .iter()
        .map(|value| parse_frame_id_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    let mut post = proof
        .post_leaves
        .iter()
        .map(|value| parse_frame_id_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected_pre = expected_pre_frame_ids.to_vec();
    let mut expected_post = expected_post_frame_ids.to_vec();
    pre.sort_unstable();
    post.sort_unstable();
    expected_pre.sort_unstable();
    expected_post.sort_unstable();
    if pre != expected_pre || post != expected_post {
        return Err(invalid("bundle leaves differ from live TL frame sets"));
    }
    Ok(())
}

struct BoundedWriter<W> {
    inner: W,
    written: usize,
    limit: usize,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit - self.written {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bundle exceeds 128 MiB",
            ));
        }
        let count = self.inner.write(bytes)?;
        self.written += count;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
fn ensure_size(proof: &ErasureProof, limit: usize) -> Result<(), ErasureProofError> {
    serde_json::to_writer(
        BoundedWriter {
            inner: std::io::sink(),
            written: 0,
            limit,
        },
        proof,
    )
    .map_err(|error| ErasureProofError::Serialization(error.to_string()))
}

/// Owner-only random create-new temp, file fsync, no-replace publication, and
/// parent fsync. Existing proof paths and predictable temp symlinks are untouched.
pub fn write_proof_bundle(proof: &ErasureProof, dir: &Path) -> Result<PathBuf, ErasureProofError> {
    std::fs::create_dir_all(dir)?;
    let safe_spirit: String = proof
        .spirit_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!(
        "{}-{}-{}.bundle",
        safe_spirit,
        proof.uninstalled_at_ns,
        hex::encode(&proof.post_root[..8])
    ));
    let mut temporary = tempfile::Builder::new()
        .prefix(".erasure-")
        .tempfile_in(dir)?;
    serde_json::to_writer_pretty(
        BoundedWriter {
            inner: temporary.as_file_mut(),
            written: 0,
            limit: MAX_BUNDLE_BYTES,
        },
        proof,
    )
    .map_err(|error| ErasureProofError::Serialization(error.to_string()))?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(&path)
        .map_err(|error| ErasureProofError::Io(error.error))?;
    if let Err(source) = std::fs::File::open(dir).and_then(|file| file.sync_all()) {
        return Err(ErasureProofError::PublishedButUnsynced { path, source });
    }
    Ok(path)
}

#[cfg(test)]
#[path = "../../tests/support/proof_unit_20_11.rs"]
mod tests;
