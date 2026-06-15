//! Story 9.4b AC-13/AC-14/AC-15 — two-phase regional teardown + region-neutral
//! erasure receipt.
//!
//! ## Honest crypto framing (Option A, re-ratification R1) — FLAG: Mary/John
//!
//! Under the ratified **Option A**, sealed TL/export artifacts are *signed*, not
//! encrypted (the working-memory rows are plaintext, region-bound by audit
//! governance — see [`crate::sealed_export`]). So Mary's D4 wording
//! "region-key destruction = crypto-shredding" does **not** literally hold for
//! these artifacts: destroying a *signing* key provides no confidentiality.
//!
//! This module implements the behaviourally-correct model:
//!   - **Phase (a)** — the forget cascade over the region-scoped PLAINTEXT rows
//!     IS the actual GDPR erasure (rows are deleted/redacted at rest). Reuses
//!     the Story 9.2 cascade via a jurisdiction-label filter (NOT re-authored).
//!   - **Phase (b)** — region **decommission**: revoke the region's signing
//!     capability so no further verifiable sealed artifacts can be produced
//!     under it. (Not "crypto-shredding" — honest naming under Option A.)
//!
//! Both phases are required; if EITHER did not complete the receipt build
//! **fails closed** and never reports success (AC-14).
//!
//! The receipt is signed with the **HOME / control-plane key** and is therefore
//! **region-NEUTRAL** (AC-10/AC-15): decommissioning the region does NOT destroy
//! the compliance receipt, which a regulator can still verify afterwards.
//!
//! Ratification note: this reframes D4's "crypto-shredding" to "signing-key
//! decommission". The behaviour (phase (a) erases, home-key receipt survives
//! phase (b)) is unambiguous; only the *wording* deviates. Ratify at review.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use maos_domain::region::Region;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::erasure::proof::SignatureBlock;
use crate::sealed_export::derive_region_pubkey;

/// Schema id for the region-neutral teardown receipt.
pub const REGIONAL_TEARDOWN_SCHEMA_VERSION: &str = "maos.regional-teardown.v1";

/// Honest phase-(b) method label under Option A (signed, not encrypted).
pub const KEY_DECOMMISSION_METHOD: &str = "signing-key-decommission";

/// The three plaintext working-memory backends the forget cascade must cover.
pub const REQUIRED_STORES: &[&str] = &["private", "principal_index", "shared"];

/// All known store names. Used to reject typos / unknown stores in attestation
/// construction. Currently identical to [`REQUIRED_STORES`]; kept separate so
/// future optional stores can be added without weakening the required set.
pub const KNOWN_STORES: &[&str] = &["private", "principal_index", "shared"];

/// Phase (a) — forget-cascade completion attestation (the REAL erasure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetCascadeAttestation {
    /// The stores the cascade covered — must include all of [`REQUIRED_STORES`].
    pub stores_covered: Vec<String>,
    /// Count of region-scoped principals whose rows were erased/redacted.
    pub erased_principal_count: u64,
    /// Whether the cascade completed across ALL required stores.
    pub completed: bool,
}

impl ForgetCascadeAttestation {
    /// Construct from a cascade outcome, computing `completed` structurally:
    /// the cascade is complete iff every [`REQUIRED_STORES`] entry was covered.
    ///
    /// Returns an error if any entry in `stores_covered` is not a
    /// [`KNOWN_STORES`] member — this rejects typos and fabricated store names.
    pub fn from_outcome(
        stores_covered: Vec<String>,
        erased_principal_count: u64,
    ) -> Result<Self, RegionalTeardownError> {
        let unknown: Vec<&str> = stores_covered
            .iter()
            .filter(|s| !KNOWN_STORES.contains(&s.as_str()))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            return Err(RegionalTeardownError::UnknownStoreName {
                names: unknown.into_iter().map(String::from).collect(),
            });
        }
        let completed = REQUIRED_STORES
            .iter()
            .all(|req| stores_covered.iter().any(|s| s == req));
        Ok(Self {
            stores_covered,
            erased_principal_count,
            completed,
        })
    }
}

/// Phase (b) — region signing-key **decommission** attestation.
///
/// NOT crypto-shredding: under Option A the artifacts are signed, so this
/// revokes the region's ability to PRODUCE verifiable sealed artifacts rather
/// than rendering existing data unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyDecommissionAttestation {
    /// The region-derived public key that is now revoked (proves WHICH key).
    pub decommissioned_region_pubkey: String,
    /// Decommission method (honest naming under Option A).
    pub method: String,
    /// Whether the decommission completed.
    pub completed: bool,
}

/// Derive the phase-(b) attestation: which region signing key is being revoked.
/// Pure — names the region-derived pubkey (from [`derive_region_pubkey`]) so a
/// verifier can bind the receipt to the exact decommissioned key.
pub fn decommission_region_key(base_seed: &[u8; 32], region: &Region) -> KeyDecommissionAttestation {
    KeyDecommissionAttestation {
        decommissioned_region_pubkey: hex::encode(derive_region_pubkey(base_seed, region)),
        method: KEY_DECOMMISSION_METHOD.to_string(),
        completed: true,
    }
}

/// The two-part, region-neutral, home-key-signed teardown receipt (AC-15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionalTeardownReceipt {
    pub schema_version: String,
    /// The region torn down — NAMED in the (signed) payload, but the signing key
    /// is the HOME key, not this region's key (region-NEUTRAL).
    pub region: String,
    pub torn_down_at_ns: u64,
    /// Phase (a) attestation.
    pub forget_cascade: ForgetCascadeAttestation,
    /// Phase (b) attestation.
    pub key_decommission: KeyDecommissionAttestation,
    /// HOME / control-plane signature (region-NEUTRAL — AC-10/AC-15).
    pub signature_block: SignatureBlock,
}

#[derive(Debug, Serialize)]
struct ReceiptForSigning<'a> {
    schema_version: &'a str,
    region: &'a str,
    torn_down_at_ns: u64,
    forget_cascade: &'a ForgetCascadeAttestation,
    key_decommission: &'a KeyDecommissionAttestation,
}

#[derive(Debug, thiserror::Error)]
pub enum RegionalTeardownError {
    /// AC-14 fail-closed: a phase did not complete, so no success is reported.
    #[error("regional teardown incomplete — phase '{phase}' did not complete; fail-closed (AC-14)")]
    IncompletePhase { phase: String },
    #[error("unknown store name(s) in forget-cascade attestation: {names:?}")]
    UnknownStoreName { names: Vec<String> },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("verification failed: {0}")]
    VerificationFailed(String),
}

fn canonicalize(unsigned: &ReceiptForSigning) -> Result<Vec<u8>, RegionalTeardownError> {
    let value = serde_json::to_value(unsigned)
        .map_err(|e| RegionalTeardownError::Serialization(e.to_string()))?;
    let sorted = sort_value(value);
    serde_json::to_vec(&sorted).map_err(|e| RegionalTeardownError::Serialization(e.to_string()))
}

fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, sort_value(v))).collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}

/// AC-14/AC-15 — build the region-neutral teardown receipt. **Fail-closed**:
/// returns [`RegionalTeardownError::IncompletePhase`] if EITHER phase did not
/// complete. Signed with `home_signing_seed` (region-NEUTRAL), so the receipt
/// survives region decommission (AC-10).
pub fn build_regional_teardown_receipt(
    home_signing_seed: &[u8; 32],
    region: &Region,
    torn_down_at_ns: u64,
    forget_cascade: ForgetCascadeAttestation,
    key_decommission: KeyDecommissionAttestation,
) -> Result<RegionalTeardownReceipt, RegionalTeardownError> {
    if !forget_cascade.completed {
        return Err(RegionalTeardownError::IncompletePhase {
            phase: "forget_cascade".to_string(),
        });
    }
    if !key_decommission.completed {
        return Err(RegionalTeardownError::IncompletePhase {
            phase: "key_decommission".to_string(),
        });
    }

    let region_str = region.as_str().to_string();
    let unsigned = ReceiptForSigning {
        schema_version: REGIONAL_TEARDOWN_SCHEMA_VERSION,
        region: &region_str,
        torn_down_at_ns,
        forget_cascade: &forget_cascade,
        key_decommission: &key_decommission,
    };
    let canonical = canonicalize(&unsigned)?;
    let digest = Sha256::digest(&canonical);
    let signing_key = SigningKey::from_bytes(home_signing_seed);
    let signature = signing_key.sign(&digest);
    let pubkey_bytes = signing_key.verifying_key().to_bytes();

    Ok(RegionalTeardownReceipt {
        schema_version: REGIONAL_TEARDOWN_SCHEMA_VERSION.to_string(),
        region: region_str,
        torn_down_at_ns,
        forget_cascade,
        key_decommission,
        signature_block: SignatureBlock {
            algorithm: "Ed25519".to_string(),
            attester_pubkey: hex::encode(pubkey_bytes),
            signature: hex::encode(signature.to_bytes()),
        },
    })
}

/// Verify a teardown receipt against the HOME / control-plane public key.
/// Re-checks both phases completed and recomputes + verifies the home-key
/// signature over the canonical payload (so tampering `region` or either
/// attestation breaks verification).
pub fn verify_regional_teardown_receipt(
    receipt: &RegionalTeardownReceipt,
    home_pubkey: &[u8; 32],
) -> Result<(), RegionalTeardownError> {
    if !receipt.forget_cascade.completed {
        return Err(RegionalTeardownError::IncompletePhase {
            phase: "forget_cascade".to_string(),
        });
    }
    if !receipt.key_decommission.completed {
        return Err(RegionalTeardownError::IncompletePhase {
            phase: "key_decommission".to_string(),
        });
    }
    let unsigned = ReceiptForSigning {
        schema_version: &receipt.schema_version,
        region: &receipt.region,
        torn_down_at_ns: receipt.torn_down_at_ns,
        forget_cascade: &receipt.forget_cascade,
        key_decommission: &receipt.key_decommission,
    };
    let canonical = canonicalize(&unsigned)?;
    let digest = Sha256::digest(&canonical);
    let vk = VerifyingKey::from_bytes(home_pubkey)
        .map_err(|e| RegionalTeardownError::VerificationFailed(e.to_string()))?;
    let sig_bytes = hex::decode(&receipt.signature_block.signature)
        .map_err(|e| RegionalTeardownError::VerificationFailed(e.to_string()))?;
    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|e| RegionalTeardownError::VerificationFailed(e.to_string()))?;
    vk.verify(&digest, &sig)
        .map_err(|e| RegionalTeardownError::VerificationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed_export::derive_pubkey;

    fn region(s: &str) -> Region {
        Region::canonicalize(s).unwrap()
    }

    fn complete_cascade() -> ForgetCascadeAttestation {
        ForgetCascadeAttestation::from_outcome(
            REQUIRED_STORES.iter().map(|s| s.to_string()).collect(),
            3,
        )
        .unwrap()
    }

    #[test]
    fn cascade_completed_only_when_all_required_stores_covered() {
        assert!(complete_cascade().completed);
        // Missing `shared` → not complete.
        let partial = ForgetCascadeAttestation::from_outcome(
            vec!["private".into(), "principal_index".into()],
            3,
        )
        .unwrap();
        assert!(!partial.completed);
    }

    #[test]
    fn from_outcome_rejects_unknown_store_names() {
        let err = ForgetCascadeAttestation::from_outcome(
            vec!["private".into(), "principal_index".into(), "shared".into(), "bogus".into()],
            3,
        )
        .unwrap_err();
        assert!(
            matches!(err, RegionalTeardownError::UnknownStoreName { ref names } if names == &["bogus"]),
            "expected UnknownStoreName, got: {err:?}"
        );
    }

    #[test]
    fn ac14_fail_closed_when_cascade_incomplete() {
        let home = [7u8; 32];
        let partial = ForgetCascadeAttestation::from_outcome(vec!["private".into()], 1).unwrap();
        let key = decommission_region_key(&[9u8; 32], &region("eu"));
        let err =
            build_regional_teardown_receipt(&home, &region("eu"), 100, partial, key).unwrap_err();
        assert!(matches!(
            err,
            RegionalTeardownError::IncompletePhase { ref phase } if phase == "forget_cascade"
        ));
    }

    #[test]
    fn ac14_fail_closed_when_key_decommission_incomplete() {
        let home = [7u8; 32];
        let mut key = decommission_region_key(&[9u8; 32], &region("eu"));
        key.completed = false; // phase (b) did not complete
        let err = build_regional_teardown_receipt(&home, &region("eu"), 100, complete_cascade(), key)
            .unwrap_err();
        assert!(matches!(
            err,
            RegionalTeardownError::IncompletePhase { ref phase } if phase == "key_decommission"
        ));
    }

    #[test]
    fn ac14_both_phases_complete_builds_verifiable_receipt() {
        let home = [7u8; 32];
        let key = decommission_region_key(&[9u8; 32], &region("eu"));
        let receipt =
            build_regional_teardown_receipt(&home, &region("eu"), 100, complete_cascade(), key)
                .unwrap();
        // AC-15: two-part receipt.
        assert!(receipt.forget_cascade.completed);
        assert!(receipt.key_decommission.completed);
        assert_eq!(receipt.key_decommission.method, KEY_DECOMMISSION_METHOD);
        // Verifies under the HOME pubkey.
        let home_pub = derive_pubkey(&home);
        assert!(verify_regional_teardown_receipt(&receipt, &home_pub).is_ok());
    }

    #[test]
    fn ac15_receipt_is_home_key_bound_region_neutral() {
        // The receipt is signed by the HOME key, NOT the region key.
        let home = [7u8; 32];
        let base = [9u8; 32];
        let key = decommission_region_key(&base, &region("eu"));
        let receipt =
            build_regional_teardown_receipt(&home, &region("eu"), 100, complete_cascade(), key)
                .unwrap();
        let home_pub = derive_pubkey(&home);
        let region_pub = derive_region_pubkey(&base, &region("eu"));
        assert!(verify_regional_teardown_receipt(&receipt, &home_pub).is_ok());
        // Region-NEUTRAL: it does NOT verify under the region-derived key.
        assert!(verify_regional_teardown_receipt(&receipt, &region_pub).is_err());
    }

    #[test]
    fn ac10_region_key_destruction_then_receipt_still_verifies() {
        // Build the receipt while the region key still exists...
        let home = [7u8; 32];
        let mut base = [9u8; 32];
        let key = decommission_region_key(&base, &region("eu"));
        let receipt =
            build_regional_teardown_receipt(&home, &region("eu"), 100, complete_cascade(), key)
                .unwrap();
        let home_pub = derive_pubkey(&home);
        // ...then "destroy" the region key material (zeroize the base seed)...
        base = [0u8; 32];
        let _ = base; // region key is now unrecoverable
                      // ...the home-key-bound receipt STILL verifies (AC-10).
        assert!(verify_regional_teardown_receipt(&receipt, &home_pub).is_ok());
    }

    #[test]
    fn tampering_region_breaks_verification() {
        let home = [7u8; 32];
        let key = decommission_region_key(&[9u8; 32], &region("eu"));
        let mut receipt =
            build_regional_teardown_receipt(&home, &region("eu"), 100, complete_cascade(), key)
                .unwrap();
        let home_pub = derive_pubkey(&home);
        assert!(verify_regional_teardown_receipt(&receipt, &home_pub).is_ok());
        // Mutate the signed region field post-signing → verification fails.
        receipt.region = "us".to_string();
        assert!(verify_regional_teardown_receipt(&receipt, &home_pub).is_err());
    }

    #[test]
    fn ac13_placement_is_region_specific_not_assumed() {
        // The decommissioned key is region-SPECIFIC: a teardown targeting `eu`
        // names a different revoked key than one targeting `us`, so placement is
        // explicit and verifiable (tested, not assumed).
        let base = [9u8; 32];
        let eu = decommission_region_key(&base, &region("eu"));
        let us = decommission_region_key(&base, &region("us"));
        assert_ne!(
            eu.decommissioned_region_pubkey, us.decommissioned_region_pubkey,
            "AC-13: each region's decommissioned key must be distinct"
        );
    }

    #[test]
    fn receipt_round_trips_through_json() {
        let home = [7u8; 32];
        let key = decommission_region_key(&[9u8; 32], &region("ap-northeast-1"));
        let receipt = build_regional_teardown_receipt(
            &home,
            &region("ap-northeast-1"),
            42,
            complete_cascade(),
            key,
        )
        .unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        let back: RegionalTeardownReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, back);
        let home_pub = derive_pubkey(&home);
        assert!(verify_regional_teardown_receipt(&back, &home_pub).is_ok());
    }
}
