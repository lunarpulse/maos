#![forbid(unsafe_code)]

//! Vetter-key lifecycle (ADV-056-3, AC5) — a **distinct signed type**, NOT the
//! unsigned `maos_domain::governance::VetterKeyPayload` (that is per-Spirit
//! admission-decision telemetry from ADR-045; this is the vetter-**key**
//! enrollment/rotation/revocation lifecycle, Ed25519-signed by the operator
//! §7.3 audit root).
//!
//! Each lifecycle event is an envelope signed by the **operator root** over
//! `sha256(canonical_cbor(event_claim))`. The [`VetterKeyring`] is the ordered,
//! append-only journal of these events; [`VetterKeyring::verify_enrollment`]
//! walks it to prove a vetter key was vouched for (enrolled, operator-root
//! signed, predating issuance) and not revoked.

use super::{ed25519_sign, ed25519_verify, VettingRejection};
use crate::canonical_cbor::sha256;
use maos_domain::revocation::{semver_range_contains, RevocationOrigin, SignedRevocationList};
use maos_spirit_abi::compliance::SigningAlg;

/// The lifecycle verb for a vetter key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum VetterKeyEventKind {
    /// Enroll a new vetter key (the operator vouches for it).
    Enroll = 0,
    /// Rotate: enroll a successor key, recording the predecessor.
    Rotate = 1,
    /// Revoke a vetter key (invalidates it going forward).
    Revoke = 2,
}

/// The signed inner claim of a vetter-key lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VetterKeyEventClaim {
    /// Which lifecycle verb.
    pub kind: VetterKeyEventKind,
    /// Stable human id of the vetter key.
    pub vetter_key_id: String,
    /// The vetter public key being enrolled / rotated-to / revoked.
    pub vetter_pubkey: [u8; 32],
    /// For `Rotate`: the predecessor key being retired (else `None`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_pubkey: Option<[u8; 32]>,
    /// Wall-clock the event takes effect (ms).
    pub effective_at_unix_ms: u64,
    /// Monotonic sequence assigned by the append-only operator journal.
    pub journal_sequence: u64,
    /// Wall-clock when the signed event was appended to that journal.
    pub journaled_at_unix_ms: u64,
    /// Free-text operator note.
    pub note: String,
}

/// A vetter-key lifecycle event: the [`VetterKeyEventClaim`] signed by the
/// operator root. Mirrors the attestation envelope shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VetterKeyEvent {
    /// Ed25519 signature over `sha256(event_bytes)` by the operator root.
    pub signature: [u8; 64],
    /// The operator-root public key that signed this event.
    pub operator_pubkey: [u8; 32],
    /// Canonical CBOR-encoded [`VetterKeyEventClaim`].
    pub event_bytes: Vec<u8>,
    /// Signing algorithm identifier.
    pub signing_alg: SigningAlg,
}

impl serde::Serialize for VetterKeyEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VetterKeyEvent", 4)?;
        state.serialize_field("signature", &self.signature[..])?;
        state.serialize_field("operator_pubkey", &self.operator_pubkey[..])?;
        state.serialize_field("event_bytes", &self.event_bytes)?;
        state.serialize_field("signing_alg", &self.signing_alg)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for VetterKeyEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            signature: Vec<u8>,
            operator_pubkey: Vec<u8>,
            event_bytes: Vec<u8>,
            signing_alg: SigningAlg,
        }
        let helper = Helper::deserialize(deserializer)?;
        let signature = helper.signature.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64-byte signature, got {} bytes", v.len()))
        })?;
        let operator_pubkey = helper.operator_pubkey.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 32-byte pubkey, got {} bytes", v.len()))
        })?;
        Ok(VetterKeyEvent {
            signature,
            operator_pubkey,
            event_bytes: helper.event_bytes,
            signing_alg: helper.signing_alg,
        })
    }
}

/// Encode a [`VetterKeyEventClaim`] to canonical CBOR.
pub fn encode_event(claim: &VetterKeyEventClaim) -> Vec<u8> {
    match serde_cbor::to_vec(claim) {
        Ok(bytes) => bytes,
        Err(error) => {
            panic!("VetterKeyEventClaim CBOR encoding for keyring signing failed: {error}")
        }
    }
}

/// Issue (sign) a vetter-key lifecycle event with the operator root seed.
pub fn issue_event(operator_seed: &[u8; 32], claim: &VetterKeyEventClaim) -> VetterKeyEvent {
    use super::ed25519_pubkey;
    let event_bytes = encode_event(claim);
    let sign_bytes = sha256(&event_bytes);
    let signature = ed25519_sign(operator_seed, &sign_bytes);
    VetterKeyEvent {
        signature,
        operator_pubkey: ed25519_pubkey(operator_seed),
        event_bytes,
        signing_alg: SigningAlg::Ed25519,
    }
}

/// The operator's vetter-key journal — append-only, in event order. Anchored to
/// a single operator root public key (the §7.3 audit key).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VetterKeyring {
    /// The operator root public key every lifecycle event must be signed by.
    pub operator_root_pubkey: [u8; 32],
    /// The ordered lifecycle events.
    pub events: Vec<VetterKeyEvent>,
    /// Operator-root-signed, Spirit/version-scoped attestation revocations.
    #[serde(default)]
    pub attestation_revocations: Vec<SignedRevocationList>,
}

impl VetterKeyring {
    /// A keyring anchored to `operator_root_pubkey` with no events.
    pub fn new(operator_root_pubkey: [u8; 32]) -> Self {
        Self {
            operator_root_pubkey,
            events: Vec::new(),
            attestation_revocations: Vec::new(),
        }
    }

    pub fn push(&mut self, event: VetterKeyEvent) {
        self.events.push(event);
    }

    pub fn push_attestation_revocation(&mut self, revocation: SignedRevocationList) {
        self.attestation_revocations.push(revocation);
    }

    pub fn ensure_operator_root(
        &self,
        expected_operator_root: &[u8; 32],
    ) -> Result<(), VettingRejection> {
        if &self.operator_root_pubkey != expected_operator_root {
            return Err(VettingRejection::OperatorRootMismatch);
        }
        Ok(())
    }

    fn verified_claim(
        &self,
        event: &VetterKeyEvent,
    ) -> Result<VetterKeyEventClaim, VettingRejection> {
        if event.signing_alg != SigningAlg::Ed25519
            || event.operator_pubkey != self.operator_root_pubkey
        {
            return Err(VettingRejection::OperatorRootSignatureInvalid);
        }
        let sign_bytes = sha256(&event.event_bytes);
        if !ed25519_verify(&self.operator_root_pubkey, &sign_bytes, &event.signature) {
            return Err(VettingRejection::OperatorRootSignatureInvalid);
        }
        serde_cbor::from_slice(&event.event_bytes)
            .map_err(|error| VettingRejection::MalformedClaim(error.to_string()))
    }

    /// Prove that the signing key and stable key id were operator-root enrolled
    /// in the append-only journal before issuance and remained active at `now`.
    pub fn verify_enrollment(
        &self,
        expected_operator_root: &[u8; 32],
        vetter_key_id: &str,
        vetter_pubkey: &[u8; 32],
        issued_at_unix_ms: u64,
        now_unix_ms: u64,
    ) -> Result<(), VettingRejection> {
        self.ensure_operator_root(expected_operator_root)?;

        let mut enrollment: Option<(u64, u64)> = None;
        let mut earliest_revocation: Option<u64> = None;
        let mut last_sequence: Option<u64> = None;
        let mut last_journaled_at: Option<u64> = None;
        let mut saw_pubkey_with_other_id = false;

        for event in &self.events {
            let claim = self.verified_claim(event)?;
            if last_sequence.is_some_and(|previous| claim.journal_sequence <= previous)
                || last_journaled_at.is_some_and(|previous| claim.journaled_at_unix_ms < previous)
                || claim.journaled_at_unix_ms > now_unix_ms
            {
                return Err(VettingRejection::InvalidJournalOrder);
            }
            last_sequence = Some(claim.journal_sequence);
            last_journaled_at = Some(claim.journaled_at_unix_ms);

            if claim.kind == VetterKeyEventKind::Rotate {
                let predecessor = claim
                    .predecessor_pubkey
                    .ok_or(VettingRejection::InvalidRotation)?;
                if predecessor == claim.vetter_pubkey {
                    return Err(VettingRejection::InvalidRotation);
                }
                if &predecessor == vetter_pubkey {
                    earliest_revocation = Some(
                        earliest_revocation.map_or(claim.effective_at_unix_ms, |previous| {
                            previous.min(claim.effective_at_unix_ms)
                        }),
                    );
                }
            }

            match claim.kind {
                VetterKeyEventKind::Enroll | VetterKeyEventKind::Rotate => {
                    if &claim.vetter_pubkey == vetter_pubkey {
                        if claim.vetter_key_id != vetter_key_id {
                            saw_pubkey_with_other_id = true;
                        } else {
                            let candidate =
                                (claim.effective_at_unix_ms, claim.journaled_at_unix_ms);
                            enrollment = Some(enrollment.map_or(candidate, |previous| {
                                if candidate.1 < previous.1 {
                                    candidate
                                } else {
                                    previous
                                }
                            }));
                        }
                    }
                }
                VetterKeyEventKind::Revoke => {
                    if &claim.vetter_pubkey == vetter_pubkey && claim.vetter_key_id == vetter_key_id
                    {
                        earliest_revocation = Some(
                            earliest_revocation.map_or(claim.effective_at_unix_ms, |previous| {
                                previous.min(claim.effective_at_unix_ms)
                            }),
                        );
                    }
                }
            }
        }

        let Some((effective_at, journaled_at)) = enrollment else {
            return Err(if saw_pubkey_with_other_id {
                VettingRejection::VetterKeyIdMismatch
            } else {
                VettingRejection::UnenrolledVetter
            });
        };
        let enrolled_at = effective_at.max(journaled_at);
        if enrolled_at >= issued_at_unix_ms {
            return Err(VettingRejection::EnrollmentNotPredatingIssuance {
                enrolled_at_unix_ms: enrolled_at,
                issued_at_unix_ms,
            });
        }
        if let Some(revoked_at) = earliest_revocation {
            if revoked_at <= now_unix_ms {
                return Err(VettingRejection::VetterKeyRevoked {
                    revoked_at_unix_ms: revoked_at,
                });
            }
        }
        Ok(())
    }

    pub fn verify_attestation_not_revoked(
        &self,
        claim: &super::VettingClaim,
        expected_operator_root: &[u8; 32],
        now_unix_ms: u64,
    ) -> Result<(), VettingRejection> {
        self.ensure_operator_root(expected_operator_root)?;
        for revocation in &self.attestation_revocations {
            if revocation.schema_version != 1
                || revocation.origin != RevocationOrigin::Operator
                || revocation.signer_pub_key != *expected_operator_root
                || revocation.entries.is_empty()
            {
                return Err(VettingRejection::AttestationRevocationListInvalid);
            }
            let entries_bytes = serde_json::to_vec(&revocation.entries)
                .map_err(|_| VettingRejection::AttestationRevocationListInvalid)?;
            if !ed25519_verify(
                expected_operator_root,
                &entries_bytes,
                &revocation.signature,
            ) {
                return Err(VettingRejection::AttestationRevocationListInvalid);
            }
            if revocation.issued_at_ns / 1_000_000 > now_unix_ms {
                continue;
            }
            for entry in &revocation.entries {
                let version_matches =
                    semver_range_contains(&claim.spirit_version, &entry.version_range)
                        .map_err(|_| VettingRejection::AttestationRevocationListInvalid)?;
                if entry.spirit_class == claim.spirit_id && version_matches {
                    return Err(VettingRejection::AttestationRevoked);
                }
            }
        }
        Ok(())
    }

    pub fn is_revoked(&self, vetter_pubkey: &[u8; 32], now_unix_ms: u64) -> bool {
        self.events.iter().any(|event| {
            self.verified_claim(event)
                .map(|claim| {
                    (claim.kind == VetterKeyEventKind::Revoke
                        && &claim.vetter_pubkey == vetter_pubkey
                        || claim.kind == VetterKeyEventKind::Rotate
                            && claim.predecessor_pubkey.as_ref() == Some(vetter_pubkey))
                        && claim.effective_at_unix_ms <= now_unix_ms
                })
                .unwrap_or(false)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vetting::ed25519_pubkey;

    fn op_seed() -> [u8; 32] {
        [0x11; 32]
    }

    fn vetter_seed() -> [u8; 32] {
        [0x22; 32]
    }

    fn enroll_event(
        operator_seed: &[u8; 32],
        vetter_pk: [u8; 32],
        effective: u64,
    ) -> VetterKeyEvent {
        issue_event(
            operator_seed,
            &VetterKeyEventClaim {
                kind: VetterKeyEventKind::Enroll,
                vetter_key_id: "vetter-01".into(),
                vetter_pubkey: vetter_pk,
                predecessor_pubkey: None,
                effective_at_unix_ms: effective,
                journal_sequence: 1,
                journaled_at_unix_ms: effective,
                note: "enrolled".into(),
            },
        )
    }

    fn revoke_event(
        operator_seed: &[u8; 32],
        vetter_pk: [u8; 32],
        effective: u64,
    ) -> VetterKeyEvent {
        issue_event(
            operator_seed,
            &VetterKeyEventClaim {
                kind: VetterKeyEventKind::Revoke,
                vetter_key_id: "vetter-01".into(),
                vetter_pubkey: vetter_pk,
                predecessor_pubkey: None,
                effective_at_unix_ms: effective,
                journal_sequence: 2,
                journaled_at_unix_ms: effective,
                note: "revoked".into(),
            },
        )
    }

    #[test]
    fn enrolled_predating_issuance_verifies() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, v, 100));
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Ok(())
        );
    }

    #[test]
    fn unenrolled_vetter_is_refused() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let kr = VetterKeyring::new(ed25519_pubkey(&op));
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::UnenrolledVetter)
        );
    }

    #[test]
    fn enrollment_not_predating_issuance_is_refused() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, v, 600)); // enrolled AFTER issuance @500
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::EnrollmentNotPredatingIssuance {
                enrolled_at_unix_ms: 600,
                issued_at_unix_ms: 500,
            })
        );
    }

    #[test]
    fn revoked_vetter_is_refused() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, v, 100));
        kr.push(revoke_event(&op, v, 800));
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::VetterKeyRevoked {
                revoked_at_unix_ms: 800,
            })
        );
    }

    #[test]
    fn wrong_operator_root_breaks_chain() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        // Keyring anchored to a DIFFERENT root than the event signer.
        let mut kr = VetterKeyring::new([0x99; 32]);
        kr.push(enroll_event(&op, v, 100));
        assert_eq!(
            kr.verify_enrollment(&[0x99; 32], "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::OperatorRootSignatureInvalid)
        );
    }

    #[test]
    fn tampered_event_breaks_operator_root_signature() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        let mut ev = enroll_event(&op, v, 100);
        ev.event_bytes[0] ^= 0xFF;
        kr.push(ev);
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::OperatorRootSignatureInvalid)
        );
    }
    #[test]
    fn embedded_root_cannot_replace_configured_operator_root() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, v, 100));
        assert_eq!(
            kr.verify_enrollment(&[0x99; 32], "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::OperatorRootMismatch)
        );
    }

    #[test]
    fn rotation_retires_predecessor_and_enrolls_successor() {
        let op = op_seed();
        let predecessor = ed25519_pubkey(&vetter_seed());
        let successor = ed25519_pubkey(&[0x33; 32]);
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, predecessor, 100));
        kr.push(issue_event(
            &op,
            &VetterKeyEventClaim {
                kind: VetterKeyEventKind::Rotate,
                vetter_key_id: "vetter-02".into(),
                vetter_pubkey: successor,
                predecessor_pubkey: Some(predecessor),
                effective_at_unix_ms: 200,
                journal_sequence: 2,
                journaled_at_unix_ms: 200,
                note: "rotate".into(),
            },
        ));

        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &predecessor, 300, 300,),
            Err(VettingRejection::VetterKeyRevoked {
                revoked_at_unix_ms: 200,
            })
        );
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-02", &successor, 300, 300,),
            Ok(())
        );
    }

    #[test]
    fn journal_sequence_reordering_is_refused() {
        let op = op_seed();
        let v = ed25519_pubkey(&vetter_seed());
        let mut kr = VetterKeyring::new(ed25519_pubkey(&op));
        kr.push(enroll_event(&op, v, 100));
        kr.push(issue_event(
            &op,
            &VetterKeyEventClaim {
                kind: VetterKeyEventKind::Revoke,
                vetter_key_id: "vetter-01".into(),
                vetter_pubkey: v,
                predecessor_pubkey: None,
                effective_at_unix_ms: 200,
                journal_sequence: 1,
                journaled_at_unix_ms: 200,
                note: "duplicate sequence".into(),
            },
        ));
        assert_eq!(
            kr.verify_enrollment(&ed25519_pubkey(&op), "vetter-01", &v, 500, 1_000),
            Err(VettingRejection::InvalidJournalOrder)
        );
    }
}
