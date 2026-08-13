#![forbid(unsafe_code)]

//! Story 13.4 (ADR-056) — FR37 vetting machinery.
//!
//! This module builds the `public-vetted` trust tier as a **signed attestation
//! artifact**, never a mutable registry flag. It lives entirely on **Axis A**
//! (the `maos_spirit_abi::compliance::TrustTier` compliance/registry axis, which
//! already carries the `PublicVetted` variant) and out of kernel-core: the
//! kernel runtime sandbox floor uses an unrelated `TrustTier` enum
//! (`maos-kernel-core::capability::cap_policy::decision::TrustTier`) with no
//! public-vetted concept, so nothing here touches kernel-core. ZERO-Δ @23228.
//!
//! # The four verbs (PRD FR37 contract)
//!
//! *issuance* ([`attestation::issue_attestation`]) → *verification*
//! ([`verify_attestation`]) → *journaling* ([`terminal`] observation events) →
//! *revocation* ([`keyring`] revoke events + [`terminal::VettingTerminalCause`]).
//! All with **internal** vetter keys; accredited external vetters (NFR-Comp-2)
//! are a v2.5 slot and explicitly out of scope.
//!
//! # The verify chain (AC5 — Mary's non-splittable floor)
//!
//! [`verify_attestation`] walks **attestation → vetter-key enrollment →
//! operator root**: an attestation is refused unless its vetter key carries a
//! **journaled enrollment, operator-root-signed, predating issuance**. A
//! structurally valid signature from a key nobody vouched for is refused
//! ([`VettingRejection::UnenrolledVetter`]).
//!
//! # Independence of issue and verify (AC2/AC6 leg 1)
//!
//! Issuance encodes the claim ([`attestation::encode_claim`], `serde_cbor::to_vec`)
//! and signs `sha256(claim_bytes)`. Verification **re-derives** `sha256` over the
//! ON-WIRE `claim_bytes` and decodes independently ([`attestation::decode_claim`],
//! `serde_cbor::from_slice`) — it never trusts a value the issuer computed and
//! never re-encodes a struct it was handed. The signature primitive is `ring`
//! Ed25519 (same monoculture as the rest of the workspace).

pub mod attestation;
pub mod keyring;
pub mod terminal;

use maos_spirit_abi::compliance::TrustTier;

pub use attestation::{
    encode_claim, issue_attestation, verify_attestation_signature, RevocationSemantics,
    SuccessorPolicy, VettingAttestation, VettingClaim,
};
pub use keyring::{VetterKeyEvent, VetterKeyEventClaim, VetterKeyEventKind, VetterKeyring};
pub use terminal::{
    classify_terminal_cause, observe_running_spirit, RunningSpiritObservation, TerminalDisposition,
    TerminalInputs, TerminalObservationSink, VettingTerminalCause,
};

use crate::canonical_cbor::sha256;

/// The typed rejection taxonomy for the vetting verify chain. Every refusal
/// names its own distinct cause so a negative test reds on its own defect and
/// never on a neighbor's (the anti-null discipline of AC6).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VettingRejection {
    /// The Ed25519 signature over `sha256(claim_bytes)` did not verify under the
    /// attestation's `vetter_pubkey` (forged / tampered / wrong key).
    #[error("vetting attestation signature invalid (forged or tampered)")]
    ForgedSignature,

    /// `claim_bytes` did not decode as a canonical-CBOR `VettingClaim`.
    #[error("vetting attestation claim malformed: {0}")]
    MalformedClaim(String),

    /// The attestation's bound `manifest_hash` did not equal `sha256(manifest_toml)`
    /// of the package being admitted (exact-hash binding, ADV-056-1).
    #[error("manifest exact-hash mismatch — expected {expected}, attestation bound {bound}")]
    ManifestHashMismatch { expected: String, bound: String },

    /// The attestation's `to_tier` is not `public-vetted`.
    #[error("vetting attestation target tier is not public-vetted (got {0:?})")]
    WrongTargetTier(TrustTier),

    /// `from_tier` must describe the governed public-untrusted promotion.
    #[error("vetting attestation source tier is not public-untrusted (got {0:?})")]
    WrongSourceTier(TrustTier),

    /// The attestation requests a revocation disposition not shipped at v2.2.
    #[error("vetting attestation requests unsupported v2.5 drain-and-refuse semantics")]
    UnsupportedRevocationSemantics,

    /// The signed validity interval is inverted or empty.
    #[error(
        "vetting attestation validity window is invalid: issued {issued_at_unix_ms}, expires {expires_at_unix_ms}"
    )]
    InvalidValidityWindow {
        issued_at_unix_ms: u64,
        expires_at_unix_ms: u64,
    },

    /// The attestation's issuance time is still in the future.
    #[error("vetting attestation is not yet valid: issued {issued_at_unix_ms}, now {now_unix_ms}")]
    NotYetValid {
        issued_at_unix_ms: u64,
        now_unix_ms: u64,
    },

    /// The signed Spirit identity/version does not name the target package.
    #[error(
        "vetting attestation binds {bound_spirit_id}/{bound_version}, expected {expected_spirit_id}/{expected_version}"
    )]
    TargetIdentityMismatch {
        bound_spirit_id: String,
        bound_version: String,
        expected_spirit_id: String,
        expected_version: String,
    },

    /// The claim's stable key id does not match the operator-root enrollment.
    #[error("vetting attestation key id is not enrolled for its signing key")]
    VetterKeyIdMismatch,

    /// `now >= expires_at` — the attestation has lapsed (ExpiryLapse cause).
    #[error("vetting attestation expired at {expires_at_unix_ms} (now {now_unix_ms})")]
    Expired {
        expires_at_unix_ms: u64,
        now_unix_ms: u64,
    },

    /// The `signing_alg` is not one this verifier supports.
    #[error("unsupported vetting signing algorithm")]
    UnsupportedSigningAlg,

    /// No operator-root-signed enrollment exists for this vetter key — the 3am
    /// case (a structurally valid signature from a key nobody vouched for).
    #[error("vetter key is not enrolled by the operator root")]
    UnenrolledVetter,

    /// An enrollment exists but did not predate the attestation's issuance — the
    /// key was vouched for only AFTER it signed, which the chain refuses.
    #[error(
        "vetter-key enrollment (effective {enrolled_at_unix_ms}) does not predate issuance ({issued_at_unix_ms})"
    )]
    EnrollmentNotPredatingIssuance {
        enrolled_at_unix_ms: u64,
        issued_at_unix_ms: u64,
    },

    /// The vetter key was revoked as of `now` (VettingRevocation cause).
    #[error("vetter key was revoked at {revoked_at_unix_ms}")]
    VetterKeyRevoked { revoked_at_unix_ms: u64 },

    /// A vetter-key lifecycle event's operator-root signature did not verify —
    /// the keyring itself is not chained to the operator root.
    #[error("vetter-key lifecycle event operator-root signature invalid")]
    OperatorRootSignatureInvalid,

    /// The keyring's embedded root does not match the configured operator key.
    #[error("vetter keyring is not anchored to the configured operator audit root")]
    OperatorRootMismatch,

    /// Signed journal metadata is duplicated, reordered, or time-regressing.
    #[error("vetter-key journal ordering is invalid")]
    InvalidJournalOrder,

    /// A rotation event omitted its predecessor or attempted a self-rotation.
    #[error("vetter-key rotation event is malformed")]
    InvalidRotation,

    /// An operator-root-signed revocation list targets this Spirit/version.
    #[error("vetting attestation was revoked by signed revocation list")]
    AttestationRevoked,

    /// An attestation revocation list failed schema, origin, root, or signature checks.
    #[error("vetting attestation revocation list is invalid")]
    AttestationRevocationListInvalid,

    /// The upgrade target declares `public-vetted` but presented no attestation
    /// of its own — the exact-hash flap (ADV-056-1): a new manifest version
    /// needs a fresh attestation, an old-version one won't match its hash.
    #[error("upgrade target requires its own current vetting attestation (exact-hash flap)")]
    UpgradeAttestationMissing,
}

/// A verified attestation — the promotion authorization. Only produced by
/// [`verify_attestation`] after the full chain walk succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAttestation {
    /// The signed claim (fields already validated against the package + chain).
    pub claim: VettingClaim,
    /// The vetter public key that signed the attestation (the enrolled key).
    pub vetter_pubkey: [u8; 32],
}

/// Verify a [`VettingAttestation`] end-to-end against the package manifest and
/// the operator vetter keyring, at wall-clock `now_unix_ms`.
///
/// Walks: signature → manifest exact-hash → target tier → expiry →
/// vetter-key enrollment predating issuance (operator-root-signed) → revocation.
/// Returns [`VerifiedAttestation`] on success or the first divergent
/// [`VettingRejection`].
pub fn verify_attestation(
    att: &VettingAttestation,
    manifest_toml: &[u8],
    keyring: &keyring::VetterKeyring,
    expected_operator_root: &[u8; 32],
    expected_spirit_id: &str,
    expected_spirit_version: &str,
    now_unix_ms: u64,
) -> Result<VerifiedAttestation, VettingRejection> {
    let claim = verify_attestation_signature(att)?;

    let expected = sha256(manifest_toml);
    if expected != claim.manifest_hash {
        return Err(VettingRejection::ManifestHashMismatch {
            expected: hex::encode(expected),
            bound: hex::encode(claim.manifest_hash),
        });
    }
    if claim.to_tier != TrustTier::PublicVetted {
        return Err(VettingRejection::WrongTargetTier(claim.to_tier));
    }
    if claim.from_tier != TrustTier::PublicUntrusted {
        return Err(VettingRejection::WrongSourceTier(claim.from_tier));
    }
    if claim.revocation_semantics != RevocationSemantics::RefuseAtNextLoad {
        return Err(VettingRejection::UnsupportedRevocationSemantics);
    }
    if claim.issued_at_unix_ms >= claim.expires_at_unix_ms {
        return Err(VettingRejection::InvalidValidityWindow {
            issued_at_unix_ms: claim.issued_at_unix_ms,
            expires_at_unix_ms: claim.expires_at_unix_ms,
        });
    }
    if claim.issued_at_unix_ms > now_unix_ms {
        return Err(VettingRejection::NotYetValid {
            issued_at_unix_ms: claim.issued_at_unix_ms,
            now_unix_ms,
        });
    }
    if claim.spirit_id != expected_spirit_id || claim.spirit_version != expected_spirit_version {
        return Err(VettingRejection::TargetIdentityMismatch {
            bound_spirit_id: claim.spirit_id,
            bound_version: claim.spirit_version,
            expected_spirit_id: expected_spirit_id.to_owned(),
            expected_version: expected_spirit_version.to_owned(),
        });
    }

    keyring.verify_enrollment(
        expected_operator_root,
        &claim.vetter_key_id,
        &att.vetter_pubkey,
        claim.issued_at_unix_ms,
        now_unix_ms,
    )?;
    keyring.verify_attestation_not_revoked(&claim, expected_operator_root, now_unix_ms)?;

    if now_unix_ms >= claim.expires_at_unix_ms {
        return Err(VettingRejection::Expired {
            expires_at_unix_ms: claim.expires_at_unix_ms,
            now_unix_ms,
        });
    }

    Ok(VerifiedAttestation {
        claim,
        vetter_pubkey: att.vetter_pubkey,
    })
}

/// Story 13.4 (AC3, ADV-056-1) — the upgrade-flap precondition, evaluated
/// **before the migration chain starts** (folded into the existing
/// `spirit upgrade --plan` / `HotSwapPrecheck` seam, NOT a new command).
///
/// If the upgrade target declares `public-vetted`, it MUST carry its own
/// current attestation whose exact-hash binds the TARGET manifest — an
/// old-version attestation fails [`verify_attestation`]'s hash check (the flap
/// is the feature). A non-vetted target needs no attestation and passes.
///
/// `successor_policy` is a hint carried in the (target) attestation; both
/// `ExactOnly` and `ReissueRequiredWithExpeditedReview` still require the target
/// version to present its own exact-hash-bound attestation here — the policy
/// governs the vetter re-issue workflow, not whether the flap fires.
pub fn evaluate_upgrade_precondition(
    target_is_public_vetted: bool,
    target_manifest_toml: &[u8],
    target_attestation: Option<&VettingAttestation>,
    keyring: &keyring::VetterKeyring,
    expected_operator_root: &[u8; 32],
    target_spirit_id: &str,
    target_spirit_version: &str,
    now_unix_ms: u64,
) -> Result<(), VettingRejection> {
    if !target_is_public_vetted {
        return Ok(());
    }
    let attestation = target_attestation.ok_or(VettingRejection::UpgradeAttestationMissing)?;
    verify_attestation(
        attestation,
        target_manifest_toml,
        keyring,
        expected_operator_root,
        target_spirit_id,
        target_spirit_version,
        now_unix_ms,
    )
    .map(|_| ())
}

// ---------------------------------------------------------------------------
// Ed25519 primitives (ring) — shared by attestation + keyring issue/verify.
// ---------------------------------------------------------------------------

/// Sign `message` with a raw 32-byte Ed25519 seed, returning the 64-byte sig.
pub(crate) fn ed25519_sign(seed: &[u8; 32], message: &[u8]) -> [u8; 64] {
    use ring::signature::Ed25519KeyPair;
    let kp = Ed25519KeyPair::from_seed_unchecked(seed).expect("valid 32-byte Ed25519 seed");
    let sig = kp.sign(message);
    let mut out = [0u8; 64];
    out.copy_from_slice(sig.as_ref());
    out
}

/// Derive the 32-byte Ed25519 public key from a raw 32-byte seed.
pub(crate) fn ed25519_pubkey(seed: &[u8; 32]) -> [u8; 32] {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let kp = Ed25519KeyPair::from_seed_unchecked(seed).expect("valid 32-byte Ed25519 seed");
    let mut out = [0u8; 32];
    out.copy_from_slice(kp.public_key().as_ref());
    out
}

/// Verify a 64-byte Ed25519 signature over `message` under a 32-byte pubkey.
/// Deliberately independent of the sign path (verify never re-signs).
pub(crate) fn ed25519_verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    use ring::signature::{UnparsedPublicKey, ED25519};
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(message, signature)
        .is_ok()
}

#[cfg(test)]
mod chain_tests {
    use super::*;
    use crate::canonical_cbor::sha256;
    use attestation::{issue_attestation, RevocationSemantics, SuccessorPolicy, VettingClaim};
    use keyring::{issue_event, VetterKeyEventClaim, VetterKeyEventKind, VetterKeyring};
    use maos_spirit_abi::compliance::TrustTier;

    const MANIFEST: &[u8] = b"[spirit]\nname = \"vetted\"\nversion = \"0.1.0\"\n";

    fn op_seed() -> [u8; 32] {
        [0x11; 32]
    }
    fn vetter_seed() -> [u8; 32] {
        [0x22; 32]
    }

    fn claim(manifest: &[u8], issued: u64, expires: u64) -> VettingClaim {
        VettingClaim {
            manifest_hash: sha256(manifest),
            spirit_id: "vetted".into(),
            spirit_version: "0.1.0".into(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter-01".into(),
            issued_at_unix_ms: issued,
            expires_at_unix_ms: expires,
            revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
            successor_policy: Some(SuccessorPolicy::ExactOnly),
        }
    }

    fn keyring_with_enrollment(enrolled_at: u64) -> VetterKeyring {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(issue_event(
            &op,
            &VetterKeyEventClaim {
                kind: VetterKeyEventKind::Enroll,
                vetter_key_id: "vetter-01".into(),
                vetter_pubkey: v,
                predecessor_pubkey: None,
                effective_at_unix_ms: enrolled_at,
                journal_sequence: 1,
                journaled_at_unix_ms: enrolled_at,
                note: "enrolled".into(),
            },
        ));
        kr
    }

    fn verify_claim_at(
        claim: &VettingClaim,
        now_unix_ms: u64,
    ) -> Result<VerifiedAttestation, VettingRejection> {
        let attestation = issue_attestation(&vetter_seed(), claim);
        let keyring = keyring_with_enrollment(100);
        verify_attestation(
            &attestation,
            MANIFEST,
            &keyring,
            &ed25519_pubkey(&op_seed()),
            &claim.spirit_id,
            &claim.spirit_version,
            now_unix_ms,
        )
    }

    #[test]
    fn full_chain_valid_attestation_verifies() {
        let att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let kr = keyring_with_enrollment(100);
        let verified = verify_attestation(
            &att,
            MANIFEST,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap();
        assert_eq!(verified.claim.to_tier, TrustTier::PublicVetted);
        assert_eq!(verified.vetter_pubkey, ed25519_pubkey(&vetter_seed()));
    }

    #[test]
    fn manifest_hash_mismatch_is_refused() {
        // Attestation binds MANIFEST, but a DIFFERENT manifest is presented.
        let att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let kr = keyring_with_enrollment(100);
        let other = b"[spirit]\nname = \"other\"\n";
        let err = verify_attestation(
            &att,
            other,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap_err();
        assert!(matches!(err, VettingRejection::ManifestHashMismatch { .. }));
    }

    #[test]
    fn expired_attestation_is_refused() {
        let att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 900));
        let kr = keyring_with_enrollment(100);
        let err = verify_attestation(
            &att,
            MANIFEST,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap_err();
        assert!(matches!(err, VettingRejection::Expired { .. }));
    }

    #[test]
    fn unenrolled_vetter_is_refused() {
        let att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let kr = VetterKeyring::new(ed25519_pubkey(&op_seed())); // no enrollment
        let err = verify_attestation(
            &att,
            MANIFEST,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap_err();
        assert_eq!(err, VettingRejection::UnenrolledVetter);
    }

    #[test]
    fn enrollment_after_issuance_is_refused() {
        let att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let kr = keyring_with_enrollment(600); // enrolled AFTER issuance
        let err = verify_attestation(
            &att,
            MANIFEST,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            VettingRejection::EnrollmentNotPredatingIssuance { .. }
        ));
    }

    #[test]
    fn wrong_target_tier_is_refused() {
        let mut c = claim(MANIFEST, 500, 2_000);
        c.to_tier = TrustTier::PublicUntrusted;
        let att = issue_attestation(&vetter_seed(), &c);
        let kr = keyring_with_enrollment(100);
        let err = verify_attestation(
            &att,
            MANIFEST,
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.1.0",
            1_000,
        )
        .unwrap_err();
        assert_eq!(
            err,
            VettingRejection::WrongTargetTier(TrustTier::PublicUntrusted)
        );
    }

    const MANIFEST_V2: &[u8] = b"[spirit]\nname = \"vetted\"\nversion = \"0.2.0\"\n";

    #[test]
    fn upgrade_non_vetted_target_passes_without_attestation() {
        let kr = keyring_with_enrollment(100);
        assert_eq!(
            evaluate_upgrade_precondition(
                false,
                MANIFEST_V2,
                None,
                &kr,
                &ed25519_pubkey(&op_seed()),
                "vetted",
                "0.2.0",
                1_000,
            ),
            Ok(())
        );
    }

    #[test]
    fn upgrade_flap_new_version_without_attestation_is_refused() {
        let kr = keyring_with_enrollment(100);
        assert_eq!(
            evaluate_upgrade_precondition(
                true,
                MANIFEST_V2,
                None,
                &kr,
                &ed25519_pubkey(&op_seed()),
                "vetted",
                "0.2.0",
                1_000,
            ),
            Err(VettingRejection::UpgradeAttestationMissing)
        );
    }

    #[test]
    fn upgrade_flap_old_attestation_does_not_bind_new_version() {
        // Attestation issued for MANIFEST (v0.1.0) does NOT satisfy the v0.2.0
        // target — exact-hash mismatch. The flap.
        let old_att = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let kr = keyring_with_enrollment(100);
        let err = evaluate_upgrade_precondition(
            true,
            MANIFEST_V2,
            Some(&old_att),
            &kr,
            &ed25519_pubkey(&op_seed()),
            "vetted",
            "0.2.0",
            1_000,
        )
        .unwrap_err();
        assert!(matches!(err, VettingRejection::ManifestHashMismatch { .. }));
    }

    #[test]
    fn upgrade_positive_new_version_with_its_own_attestation_passes() {
        let mut new_claim = claim(MANIFEST_V2, 500, 2_000);
        new_claim.spirit_version = "0.2.0".into();
        let new_att = issue_attestation(&vetter_seed(), &new_claim);
        let kr = keyring_with_enrollment(100);
        assert_eq!(
            evaluate_upgrade_precondition(
                true,
                MANIFEST_V2,
                Some(&new_att),
                &kr,
                &ed25519_pubkey(&op_seed()),
                "vetted",
                "0.2.0",
                1_000,
            ),
            Ok(())
        );
    }
    #[test]
    fn future_and_inverted_validity_windows_are_refused() {
        let future = claim(MANIFEST, 1_500, 2_000);
        assert!(matches!(
            verify_claim_at(&future, 1_000),
            Err(VettingRejection::NotYetValid { .. })
        ));

        let inverted = claim(MANIFEST, 2_000, 1_500);
        assert!(matches!(
            verify_claim_at(&inverted, 1_000),
            Err(VettingRejection::InvalidValidityWindow { .. })
        ));
    }

    #[test]
    fn source_tier_and_v2_5_semantics_are_refused() {
        let mut wrong_source = claim(MANIFEST, 500, 2_000);
        wrong_source.from_tier = TrustTier::Local;
        assert_eq!(
            verify_claim_at(&wrong_source, 1_000).unwrap_err(),
            VettingRejection::WrongSourceTier(TrustTier::Local)
        );

        let mut future_semantics = claim(MANIFEST, 500, 2_000);
        future_semantics.revocation_semantics = RevocationSemantics::DrainAndRefuse;
        assert_eq!(
            verify_claim_at(&future_semantics, 1_000).unwrap_err(),
            VettingRejection::UnsupportedRevocationSemantics
        );
    }

    #[test]
    fn identity_and_key_id_are_bound() {
        let attestation = issue_attestation(&vetter_seed(), &claim(MANIFEST, 500, 2_000));
        let keyring = keyring_with_enrollment(100);
        assert!(matches!(
            verify_attestation(
                &attestation,
                MANIFEST,
                &keyring,
                &ed25519_pubkey(&op_seed()),
                "other",
                "0.1.0",
                1_000,
            ),
            Err(VettingRejection::TargetIdentityMismatch { .. })
        ));

        let mut wrong_key_id = claim(MANIFEST, 500, 2_000);
        wrong_key_id.vetter_key_id = "different-id".into();
        assert_eq!(
            verify_claim_at(&wrong_key_id, 1_000).unwrap_err(),
            VettingRejection::VetterKeyIdMismatch
        );
    }

    #[test]
    fn signed_spirit_version_revocation_refuses_attestation() {
        use maos_domain::revocation::{
            CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
        };

        let claim = claim(MANIFEST, 500, 2_000);
        let attestation = issue_attestation(&vetter_seed(), &claim);
        let mut keyring = keyring_with_enrollment(100);
        let entries =
            vec![RevocationEntry::new("vetted", "0.1.0", "vetting withdrawn", None).unwrap()];
        let entries_bytes = maos_domain::revocation::canonical_entries_bytes(&entries).unwrap();
        let signature = ed25519_sign(&op_seed(), &entries_bytes);
        keyring.push_attestation_revocation(
            SignedRevocationList::new(
                CrlId::from_entries(&entries).unwrap(),
                1,
                900_000_000,
                RevocationOrigin::Operator,
                entries,
                signature,
                ed25519_pubkey(&op_seed()),
            )
            .unwrap(),
        );

        assert_eq!(
            verify_attestation(
                &attestation,
                MANIFEST,
                &keyring,
                &ed25519_pubkey(&op_seed()),
                "vetted",
                "0.1.0",
                1_000,
            )
            .unwrap_err(),
            VettingRejection::AttestationRevoked
        );
    }
}
