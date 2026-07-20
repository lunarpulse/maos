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
use maos_audit::erasure::merkle::{build_tree, prove_inclusion};
use maos_audit::sealed_export::{
    derive_region_pubkey, derive_region_signing_seed, derive_team_pubkey, derive_team_signing_seed,
};
use maos_domain::region::Region;
use maos_domain::team::TeamId;

use super::leaf::{kv_merkle_root, CollectiveKvLeaf};
use crate::schema;
use crate::store::{LoomLiteStore, RowAttestation};

/// Frozen schema version for the replication-bundle wire shape.
///
/// Bumping this is a wire-format change that MUST be gated by an ADR.
const BUNDLE_SCHEMA_VERSION: u16 = 1;
/// Story 13.2 (AC3/G5): schema version for a v2 (cross-team) bundle. The team
/// dimension is an INDEPENDENT axis from the leaf `KV_LEAF_DOMAIN_V2` tag —
/// bumping one never implies the other (Yui). Bumping this is an ADR-gated
/// wire-format change.
const BUNDLE_SCHEMA_VERSION_V2: u16 = 2;

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
    /// Story 13.2 (AC3): source-team identity for a v2 (cross-team) bundle.
    /// `None` = v1 region-only bundle (byte-exact 11.2a path). `Some` = v2:
    /// verify derives the pubkey from the CLAIMED `(region, team)`.
    /// `serde(default)` is REQUIRED — without it a v1 bundle (no `source_team`
    /// on the wire) stops deserializing (Round-2 wire-compat fix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_team: Option<TeamId>,
    pub region_sig: Vec<u8>,
}

/// Outcome of applying a replication bundle to a destination store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyResult {
    pub applied_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CrossTeamApplyContext<'a> {
    pub to_team: &'a TeamId,
    pub intent: &'a str,
}

impl<'a> CrossTeamApplyContext<'a> {
    pub fn new(to_team: &'a TeamId, intent: &'a str) -> Self {
        Self { to_team, intent }
    }
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
    #[error("cross-team bundle requires an explicit destination team and intent")]
    CrossTeamContextRequired,
    #[error("region-only bundle must not carry cross-team apply context")]
    UnexpectedCrossTeamContext,
    #[error("destination store home team is invalid: {reason}")]
    DestinationTeamInvalid { reason: String },
    #[error("source team {team} equals destination home team")]
    SelfCrossing { team: TeamId },
    #[error("destination team mismatch: store={configured}, requested={requested}")]
    DestinationTeamMismatch {
        configured: TeamId,
        requested: TeamId,
    },
    #[error("destination region mismatch: store={configured}, requested={requested}")]
    DestinationRegionMismatch {
        configured: String,
        requested: String,
    },
    #[error(
        "destination store has no configured home region for a cross-team apply (requested {requested})"
    )]
    DestinationRegionUnconfigured { requested: String },
    #[error("cross-team leaf identity mismatch at key {key}: {reason}")]
    LeafIdentityMismatch { key: String, reason: String },
    #[error("no verifying key configured for the claimed (region {region}, team {team})")]
    TeamVerifyingKeyUnavailable { region: String, team: TeamId },
    #[error("cross-team consent denied: {from_team}->{to_team}, intent={intent}")]
    ConsentDenied {
        from_team: TeamId,
        to_team: TeamId,
        intent: String,
    },
    #[error("cross-team consent state stale: {reason}")]
    ConsentStateStale { reason: String },
    #[error("cross-team consent state unavailable: {reason}")]
    ConsentStateUnavailable { reason: String },
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

/// Build the v2 (cross-team) sign payload:
/// `SIG_DOMAIN || schema_version || LP(source_region) || LP(source_team) || root`.
///
/// Unlike the v1 payload, both `source_region` and `source_team` are
/// length-prefixed so a `region|team` boundary shift cannot collide. The
/// distinct `schema_version` (2) also keeps this grammar disjoint from the v1
/// payload under the same `SIG_DOMAIN` — a v1 and v2 payload can never coincide.
fn build_team_sign_payload(
    schema_version: u16,
    source_region: &[u8],
    source_team: &[u8],
    root: &[u8; 32],
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(
        SIG_DOMAIN.len() + 2 + 4 + source_region.len() + 4 + source_team.len() + 32,
    );
    payload.extend_from_slice(SIG_DOMAIN);
    payload.extend_from_slice(&schema_version.to_be_bytes());
    payload.extend_from_slice(&(source_region.len() as u32).to_be_bytes());
    payload.extend_from_slice(source_region);
    payload.extend_from_slice(&(source_team.len() as u32).to_be_bytes());
    payload.extend_from_slice(source_team);
    payload.extend_from_slice(root);
    payload
}

pub(crate) fn verify_team_root_signature(
    schema_version: u16,
    source_region: &Region,
    source_team: &TeamId,
    root: &[u8; 32],
    region_sig: &[u8],
    public_key: &[u8; 32],
) -> bool {
    if schema_version != BUNDLE_SCHEMA_VERSION_V2 {
        return false;
    }
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(region_sig) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    let payload = build_team_sign_payload(
        schema_version,
        source_region.as_str().as_bytes(),
        source_team.as_str().as_bytes(),
        root,
    );
    verifying_key.verify(&payload, &signature).is_ok()
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
    let sign_payload = build_sign_payload(
        BUNDLE_SCHEMA_VERSION,
        source_region.as_str().as_bytes(),
        &root,
    );

    let region_seed = derive_region_signing_seed(base_seed, source_region);
    let signing_key = SigningKey::from_bytes(&region_seed);
    let signature = signing_key.sign(&sign_payload);

    CrossRegionReplicationBundle {
        schema_version: BUNDLE_SCHEMA_VERSION,
        source_region: source_region.as_str().to_string(),
        root,
        leaves,
        source_team: None,
        region_sig: signature.to_bytes().to_vec(),
    }
}

/// Build a signed v2 (cross-team) replication bundle (Story 13.2 / Fork-4).
///
/// Signs with the per-team HKDF-derived key (the SECOND-stage weld over the
/// region seed). Verification re-derives the key from the bundle-DECLARED
/// `(region, team)`, so a forged same-region cross-team bundle — one that
/// *claims* a team it cannot sign for — cannot verify (ADV-055-1, the Fork-4
/// payoff). Build/verify are symmetric on [`build_team_sign_payload`].
pub fn build_replication_bundle_v2(
    mut leaves: Vec<CollectiveKvLeaf>,
    source_region: &Region,
    source_team: &TeamId,
    base_seed: &[u8; 32],
) -> CrossRegionReplicationBundle {
    // Stamp the envelope identity onto every leaf BEFORE hashing. The v2
    // canonical leaf carries `source_region`/`source_team`, and the
    // destination read path reconstructs each leaf with the envelope team —
    // an unstamped (first-party) leaf would hash differently at read than at
    // apply, landing a permanently unreadable row (13.3 review). Stamping is
    // idempotent for leaves already carrying this exact identity.
    for leaf in &mut leaves {
        leaf.source_region = source_region.as_str().to_string();
        leaf.source_team = Some(source_team.clone());
    }
    let root = kv_merkle_root(&leaves);
    let sign_payload = build_team_sign_payload(
        BUNDLE_SCHEMA_VERSION_V2,
        source_region.as_str().as_bytes(),
        source_team.as_str().as_bytes(),
        &root,
    );
    let team_seed = derive_team_signing_seed(base_seed, source_region, source_team);
    let signing_key = SigningKey::from_bytes(&team_seed);
    let signature = signing_key.sign(&sign_payload);

    CrossRegionReplicationBundle {
        schema_version: BUNDLE_SCHEMA_VERSION_V2,
        source_region: source_region.as_str().to_string(),
        root,
        leaves,
        source_team: Some(source_team.clone()),
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

    // Dispatch on schema_version (G5). v1 = byte-exact region path
    // (derive_region_pubkey). v2 = per-team path: derive the pubkey from the
    // bundle-DECLARED (region, team) — NEVER a key the bundle carries (R-RG1
    // team analogue). A forger who stamps source_team = A but cannot sign under
    // team-A's derived key fails here.
    let (sign_payload, pubkey_bytes) = match bundle.schema_version {
        BUNDLE_SCHEMA_VERSION => {
            if bundle.source_team.is_some() {
                return Err(BundleError::SignatureVerificationFailed(
                    "v1 bundle must not carry source_team".to_string(),
                ));
            }
            (
                build_sign_payload(
                    bundle.schema_version,
                    region.as_str().as_bytes(),
                    &bundle.root,
                ),
                derive_region_pubkey(base_seed, &region),
            )
        }
        BUNDLE_SCHEMA_VERSION_V2 => {
            let team = bundle.source_team.as_ref().ok_or_else(|| {
                BundleError::SignatureVerificationFailed(
                    "v2 bundle must carry source_team".to_string(),
                )
            })?;
            (
                build_team_sign_payload(
                    bundle.schema_version,
                    region.as_str().as_bytes(),
                    team.as_str().as_bytes(),
                    &bundle.root,
                ),
                derive_team_pubkey(base_seed, &region, team),
            )
        }
        other => {
            return Err(BundleError::SignatureVerificationFailed(format!(
                "unsupported bundle schema_version {other}"
            )));
        }
    };

    let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| BundleError::SignatureVerificationFailed(format!("invalid pubkey: {e}")))?;

    let sig_bytes: [u8; 64] = bundle.region_sig.as_slice().try_into().map_err(|_| {
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

/// Apply an already-verified bundle to a destination store via the CRDT LWW upsert.
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
fn apply_verified_replication_bundle<'a>(
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
        let merkle_tree = bundle.source_team.as_ref().map(|_| {
            let leaf_hashes: Vec<[u8; 32]> = bundle
                .leaves
                .iter()
                .map(CollectiveKvLeaf::canonical_hash)
                .collect();
            build_tree(&leaf_hashes)
        });

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

            // The persisted bytes must equal the signed bytes: the write path
            // re-serializes through `value_to_parts`/`namespace_to_parts`, so
            // a leaf whose bytes do not round-trip (e.g. non-canonical JSON)
            // would land a row whose read-path reconstructed hash never
            // matches the persisted one (13.3 review).
            match schema::value_to_parts(&value) {
                Ok((kind, data)) if kind == leaf.value_kind && data == leaf.value_data => {}
                Ok(_) => {
                    tracing::warn!(
                        source_region = %leaf.source_region,
                        key = %leaf.key,
                        "skipping leaf: value bytes do not round-trip through the store codec"
                    );
                    skipped_count += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        source_region = %leaf.source_region,
                        key = %leaf.key,
                        error = %e,
                        "skipping leaf: value re-serialization failed"
                    );
                    skipped_count += 1;
                    continue;
                }
            }
            {
                let (ns_kind, ns_detail) = schema::namespace_to_parts(&namespace);
                if ns_kind != leaf.namespace_kind.as_str() || ns_detail != leaf.namespace_detail {
                    tracing::warn!(
                        source_region = %leaf.source_region,
                        key = %leaf.key,
                        "skipping leaf: namespace does not round-trip through the store codec"
                    );
                    skipped_count += 1;
                    continue;
                }
            }

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

            let attestation_parts = merkle_tree.as_ref().and_then(|tree| {
                let leaf_hash = leaf.canonical_hash();
                prove_inclusion(tree, leaf_hash).map(|proof| (leaf_hash, proof))
            });
            let attestation = attestation_parts
                .as_ref()
                .map(|(leaf_hash, inclusion_proof)| RowAttestation {
                    leaf_canonical_hash: leaf_hash,
                    merkle_root: &bundle.root,
                    region_sig: &bundle.region_sig,
                    bundle_schema_version: bundle.schema_version,
                    inclusion_proof,
                });

            store
                .write_with_source_attested(
                    pid,
                    &namespace,
                    &leaf.key,
                    value,
                    crate::store::WriteSource {
                        ts: leaf.source_ts,
                        region: &leaf.source_region,
                        log_ref: &source_log_ref,
                        team: bundle.source_team.as_ref(),
                    },
                    attestation.as_ref(),
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

/// Verify a replication bundle, then apply it to a destination store.
///
/// This is the only public apply entry point: signature and Merkle-root
/// verification complete before any store access. A forged same-region
/// cross-team bundle therefore returns [`BundleError::SignatureVerificationFailed`]
/// before a row can land.
pub async fn apply_replication_bundle(
    bundle: &CrossRegionReplicationBundle,
    store: &LoomLiteStore,
    dest_region: &str,
    cross_team: Option<CrossTeamApplyContext<'_>>,
    base_seed: &[u8; 32],
) -> Result<ApplyResult, BundleError> {
    verify_replication_bundle(bundle, base_seed)?;
    let dest = Region::canonicalize(dest_region)
        .map_err(|error| BundleError::InvalidRegion(error.to_string()))?;
    // Region binding: v1 (region-only) bundles keep the legacy leniency — an
    // unconfigured destination accepts any region. A cross-team bundle binds
    // to a CONFIGURED destination region: empty is a typed refusal, not a
    // skipped check (13.3 review).
    if store.config().home_region.is_empty() {
        if bundle.source_team.is_some() {
            return Err(BundleError::DestinationRegionUnconfigured {
                requested: dest.as_str().to_string(),
            });
        }
    } else if store.config().home_region != dest.as_str() {
        return Err(BundleError::DestinationRegionMismatch {
            configured: store.config().home_region.clone(),
            requested: dest.as_str().to_string(),
        });
    }
    match (bundle.source_team.as_ref(), cross_team) {
        (None, None) => {}
        (None, Some(_)) => return Err(BundleError::UnexpectedCrossTeamContext),
        (Some(_), None) => return Err(BundleError::CrossTeamContextRequired),
        (Some(from_team), Some(context)) => {
            let home_team = TeamId::new(&store.config().home_team).map_err(|error| {
                BundleError::DestinationTeamInvalid {
                    reason: error.to_string(),
                }
            })?;
            if from_team == &home_team {
                return Err(BundleError::SelfCrossing { team: home_team });
            }
            if context.to_team != &home_team {
                return Err(BundleError::DestinationTeamMismatch {
                    configured: home_team,
                    requested: context.to_team.clone(),
                });
            }
            // Leaf/envelope identity: every leaf must carry the verified
            // envelope's (region, team). Apply persists the envelope team
            // alongside a hash of the leaf itself; a mismatch lands rows
            // that permanently fail the read-path hash check (13.3 review).
            for leaf in &bundle.leaves {
                if leaf.source_team.as_ref() != Some(from_team)
                    || leaf.source_region != bundle.source_region
                {
                    return Err(BundleError::LeafIdentityMismatch {
                        key: leaf.key.clone(),
                        reason: "leaf (source_region, source_team) differs from the signed \
                                 bundle envelope"
                            .to_string(),
                    });
                }
            }
            let consent =
                store
                    .cross_team_consent()
                    .ok_or_else(|| BundleError::ConsentStateUnavailable {
                        reason: "no cross-team consent port is configured".to_string(),
                    })?;
            let granted = consent
                .is_granted(from_team, context.to_team, context.intent)
                .map_err(|error| match error {
                    crate::cross_team_consent::CrossTeamConsentError::Stale { reason } => {
                        BundleError::ConsentStateStale { reason }
                    }
                    crate::cross_team_consent::CrossTeamConsentError::StateUnavailable {
                        reason,
                    } => BundleError::ConsentStateUnavailable { reason },
                })?;
            if !granted {
                return Err(BundleError::ConsentDenied {
                    from_team: from_team.clone(),
                    to_team: context.to_team.clone(),
                    intent: context.intent.to_string(),
                });
            }
            // Write/read coherence: the destination must hold the verifying
            // key for the claimed (region, team) — the same manifest-declared
            // pair set the read path enforces. A crossing the destination
            // could never serve is refused here instead of landing rows that
            // read back as AttestationInvalid (13.3 review, party-mode D1).
            let claimed_region = Region::canonicalize(&bundle.source_region)
                .map_err(|error| BundleError::InvalidRegion(error.to_string()))?;
            if store
                .team_verifying_key(&claimed_region, from_team)
                .is_none()
            {
                return Err(BundleError::TeamVerifyingKeyUnavailable {
                    region: claimed_region.as_str().to_string(),
                    team: from_team.clone(),
                });
            }
        }
    }
    apply_verified_replication_bundle(bundle, store, dest.as_str()).await
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

    let sig_bytes: [u8; 64] = receipt.signature.as_slice().try_into().map_err(|_| {
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
            source_team: None,
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
    fn test_v1_wire_without_source_team_deserializes() {
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-west-2a").unwrap();
        let bundle = build_replication_bundle(vec![sample_leaf("us-west-2a")], &region, &base_seed);
        let wire = serde_json::to_value(&bundle).expect("serialize v1 bundle");

        let object = wire.as_object().expect("bundle serializes as an object");
        assert!(
            !object.contains_key("source_team"),
            "v1 bundle wire payload must omit source_team"
        );
        let leaf = object["leaves"]
            .as_array()
            .and_then(|leaves| leaves.first())
            .and_then(serde_json::Value::as_object)
            .expect("v1 bundle carries one object leaf");
        assert!(
            !leaf.contains_key("source_team"),
            "v1 leaf wire payload must omit source_team"
        );

        let decoded: CrossRegionReplicationBundle =
            serde_json::from_value(wire).expect("deserialize pre-13.2 v1 wire payload");
        assert!(decoded.source_team.is_none());
        assert!(decoded.leaves[0].source_team.is_none());
        verify_replication_bundle(&decoded, &base_seed)
            .expect("deserialized v1 bundle remains verifiable");
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

    // ─── Story 13.2 AC3/AC5 — cross-team entry closure (Option A) ────────────

    fn team(id: &str) -> TeamId {
        TeamId::new(id).unwrap()
    }

    #[test]
    fn test_v2_team_bundle_roundtrip() {
        // Positive control: a v2 bundle built + verified for the same (region, team).
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let t = team("security");
        let bundle =
            build_replication_bundle_v2(vec![sample_leaf("us-east-1")], &region, &t, &base_seed);
        assert_eq!(bundle.schema_version, BUNDLE_SCHEMA_VERSION_V2);
        assert_eq!(bundle.source_team.as_ref(), Some(&t));
        verify_replication_bundle(&bundle, &base_seed).expect("genuine v2 bundle must verify");
    }

    #[test]
    fn test_v1_bundle_still_verifies_and_rejects_stray_team() {
        // v1 byte-exact path is untouched; a v1 bundle carrying a stray
        // source_team (wire tampering) is refused.
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let bundle = build_replication_bundle(vec![sample_leaf("us-east-1")], &region, &base_seed);
        assert_eq!(bundle.schema_version, BUNDLE_SCHEMA_VERSION);
        assert!(bundle.source_team.is_none());
        verify_replication_bundle(&bundle, &base_seed).expect("v1 bundle must still verify");
        let mut tampered = bundle.clone();
        tampered.source_team = Some(team("security"));
        assert!(
            matches!(
                verify_replication_bundle(&tampered, &base_seed),
                Err(BundleError::SignatureVerificationFailed(_))
            ),
            "a v1 bundle must not carry source_team"
        );
    }

    #[test]
    fn test_forged_team_stamp_refused_at_verify_same_region() {
        // THE Fork-4 payoff (AC5a). SAME region, cross-team forgery. team-B
        // builds a v2 bundle for itself, then relabels source_team = team-A (a
        // stamp it cannot sign for — it lacks team-A's derived key). verify
        // derives team-A's key from the CLAIMED (region, team-A) and the team-B
        // signature fails. NOT cross-region: region is identical throughout, so
        // this is exactly where the team weld earns its keep (region_guard
        // already refuses cross-region — F9 anti-tautology).
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let team_a = team("security");
        let team_b = team("support");

        // team-B's legitimate bundle (signed with team-B's derived key).
        let mut forged = build_replication_bundle_v2(
            vec![sample_leaf("us-east-1")],
            &region,
            &team_b,
            &base_seed,
        );
        // Forge the stamp: claim team-A, SAME region.
        forged.source_team = Some(team_a.clone());

        let err = verify_replication_bundle(&forged, &base_seed).unwrap_err();
        assert!(
            matches!(err, BundleError::SignatureVerificationFailed(_)),
            "a same-region cross-team forged stamp MUST be refused at verify, got {err:?}"
        );

        // INDEPENDENT verifier (anti-tautology, R-RG1 shape): derive team-A's
        // expected pubkey straight from the write codec (sealed_export), NOT a
        // re-call of verify_replication_bundle. The forged signature must NOT
        // verify under team-A's key; a positive control (team-A signs) MUST.
        let payload = build_team_sign_payload(
            BUNDLE_SCHEMA_VERSION_V2,
            region.as_str().as_bytes(),
            team_a.as_str().as_bytes(),
            &forged.root,
        );
        let pk_a =
            VerifyingKey::from_bytes(&derive_team_pubkey(&base_seed, &region, &team_a)).unwrap();
        let forged_sig_arr: [u8; 64] = forged.region_sig.as_slice().try_into().unwrap();
        assert!(
            pk_a.verify(&payload, &Signature::from_bytes(&forged_sig_arr))
                .is_err(),
            "forged team-B signature must not verify under independently-derived team-A key"
        );
        let genuine = build_replication_bundle_v2(
            vec![sample_leaf("us-east-1")],
            &region,
            &team_a,
            &base_seed,
        );
        let genuine_sig_arr: [u8; 64] = genuine.region_sig.as_slice().try_into().unwrap();
        // The genuine bundle's leaves are stamped with team-A's identity at
        // build (13.3 review), so its root differs from the forged bundle's —
        // the positive control signs the GENUINE root.
        let genuine_payload = build_team_sign_payload(
            BUNDLE_SCHEMA_VERSION_V2,
            region.as_str().as_bytes(),
            team_a.as_str().as_bytes(),
            &genuine.root,
        );
        assert!(
            pk_a.verify(&genuine_payload, &Signature::from_bytes(&genuine_sig_arr))
                .is_ok(),
            "genuine team-A signature MUST verify under the independently-derived team-A key \
             (proves verify is not a reject-everything stub)"
        );
    }

    #[test]
    fn test_team_identity_source_reflex() {
        // AC5d: the accepted bundle's identity is what the DERIVED key proves at
        // verify, not the label it carries. A genuine team-A bundle relabeled to
        // ANY other team fails (the key no longer matches the claim).
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let team_a = team("security");
        let bundle = build_replication_bundle_v2(
            vec![sample_leaf("us-east-1")],
            &region,
            &team_a,
            &base_seed,
        );
        verify_replication_bundle(&bundle, &base_seed).expect("genuine verifies");
        let mut relabeled = bundle.clone();
        relabeled.source_team = Some(team("support"));
        let err = verify_replication_bundle(&relabeled, &base_seed).unwrap_err();
        assert!(
            matches!(err, BundleError::SignatureVerificationFailed(_)),
            "relabeling source_team MUST break verification (identity is key-proven), got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_apply_refuses_forged_bundle_writes_zero_rows() {
        // AC5b / Option A entry closure: verify_and_apply on a forged cross-team
        // bundle refuses at verify BEFORE any store access — the forged row never
        // lands. Hermetic: the store is a lazily-constructed pool at an
        // unreachable address (never connects). If apply were (wrongly) reached,
        // the pool would yield a Pool/Timeout StoreError; the
        // SignatureVerificationFailed proves the entry gate short-circuited and
        // the store was never touched (zero rows).
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let team_a = team("security");
        let team_b = team("support");
        let mut forged = build_replication_bundle_v2(
            vec![sample_leaf("us-east-1")],
            &region,
            &team_b,
            &base_seed,
        );
        forged.source_team = Some(team_a);

        let store = LoomLiteStore::new(crate::store::StoreConfig {
            connection_string: "host=127.0.0.1 port=1 user=maos dbname=maos sslmode=disable"
                .to_string(),
            ..crate::store::StoreConfig::default()
        })
        .await
        .expect("lazy store construction must not connect");

        let err = apply_replication_bundle(&forged, &store, "us-east-1", None, &base_seed)
            .await
            .unwrap_err();
        assert!(
            matches!(err, BundleError::SignatureVerificationFailed(_)),
            "forged bundle must be refused at verify before any store write, got {err:?}"
        );
    }

    /// 13.3 review: a verify-passing bundle whose leaves disagree with the
    /// signed envelope (region, team) must be refused at apply — before
    /// consent and before any store access — or apply would persist the
    /// envelope team alongside a hash of the foreign leaf, landing rows that
    /// permanently fail the read-path hash check.
    #[tokio::test]
    async fn apply_refuses_leaf_identity_mismatch() {
        let base_seed = [0x42u8; 32];
        let region = Region::canonicalize("us-east-1").unwrap();
        let team_a = team("security");
        let team_b = team("support");

        // Forge: leaves stamped for team-B, envelope claims team-A, signed
        // with team-A's derived key — verify passes, identity does not.
        let mut forged_leaves = vec![sample_leaf("us-east-1")];
        forged_leaves[0].source_team = Some(team_b.clone());
        let root = kv_merkle_root(&forged_leaves);
        let payload = build_team_sign_payload(
            BUNDLE_SCHEMA_VERSION_V2,
            region.as_str().as_bytes(),
            team_a.as_str().as_bytes(),
            &root,
        );
        let signing_key =
            SigningKey::from_bytes(&derive_team_signing_seed(&base_seed, &region, &team_a));
        let forged = CrossRegionReplicationBundle {
            schema_version: BUNDLE_SCHEMA_VERSION_V2,
            source_region: region.as_str().to_string(),
            root,
            leaves: forged_leaves,
            source_team: Some(team_a.clone()),
            region_sig: signing_key.sign(&payload).to_bytes().to_vec(),
        };
        verify_replication_bundle(&forged, &base_seed)
            .expect("control: the forged envelope verifies under team-A's key");

        let store = LoomLiteStore::new(crate::store::StoreConfig {
            connection_string: "host=127.0.0.1 port=1 user=maos dbname=maos sslmode=disable"
                .to_string(),
            home_region: "us-east-1".to_string(),
            home_team: "support".to_string(),
            ..crate::store::StoreConfig::default()
        })
        .await
        .expect("lazy store construction must not connect");

        let err = apply_replication_bundle(
            &forged,
            &store,
            "us-east-1",
            Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
            &base_seed,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, BundleError::LeafIdentityMismatch { .. }),
            "leaf/envelope identity mismatch must refuse at apply, got {err:?}"
        );
    }
}
