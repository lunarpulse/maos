#![forbid(unsafe_code)]

//! The [`VettingAttestation`] signed-envelope type + [`VettingClaim`] payload.
//!
//! The envelope mirrors `maos_spirit_abi::compliance::ComplianceClaimEnvelope`
//! exactly (hand-rolled fixed-array serde over `[u8;64]` / `[u8;32]`) so it
//! round-trips byte-stably, and — like that envelope — the signature covers
//! `sha256(claim_bytes)` rather than the CBOR directly, keeping verification
//! independent of claim parsing.

use maos_spirit_abi::compliance::{SigningAlg, TrustTier};

use super::{ed25519_pubkey, ed25519_sign, ed25519_verify, VettingRejection};
use crate::canonical_cbor::sha256;

/// An Ed25519-signed vetting attestation — the `public-vetted` promotion
/// **artifact** (ADR-056, AC1). Presence of a valid attestation is what
/// promotes a Spirit; there is no mutable registry flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VettingAttestation {
    /// Ed25519 signature over `sha256(claim_bytes)`. 64 bytes.
    pub signature: [u8; 64],
    /// Ed25519 public key of the vetter that signed. 32 bytes. This is the
    /// cryptographic identity the verify chain resolves against the keyring.
    pub vetter_pubkey: [u8; 32],
    /// Canonical CBOR-encoded [`VettingClaim`] the signature covers.
    pub claim_bytes: Vec<u8>,
    /// Signing algorithm identifier.
    pub signing_alg: SigningAlg,
}

impl serde::Serialize for VettingAttestation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("VettingAttestation", 4)?;
        state.serialize_field("signature", &self.signature[..])?;
        state.serialize_field("vetter_pubkey", &self.vetter_pubkey[..])?;
        state.serialize_field("claim_bytes", &self.claim_bytes)?;
        state.serialize_field("signing_alg", &self.signing_alg)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for VettingAttestation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            signature: Vec<u8>,
            vetter_pubkey: Vec<u8>,
            claim_bytes: Vec<u8>,
            signing_alg: SigningAlg,
        }
        let helper = Helper::deserialize(deserializer)?;
        let signature = helper.signature.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64-byte signature, got {} bytes", v.len()))
        })?;
        let vetter_pubkey = helper.vetter_pubkey.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 32-byte pubkey, got {} bytes", v.len()))
        })?;
        Ok(VettingAttestation {
            signature,
            vetter_pubkey,
            claim_bytes: helper.claim_bytes,
            signing_alg: helper.signing_alg,
        })
    }
}

/// What happens to the promotion when the attestation is revoked.
///
/// v2.2 ships `RefuseAtNextLoad` only. `DrainAndRefuse` is the reserved v2.5
/// slot (draining a running Spirit is kernel work, out of this story's zero-Δ
/// scope). At the **load** boundary both behave identically — a load is
/// refused; the drain difference only concerns already-running Spirits, which
/// v2.2 does not act on beyond a journaled observation event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum RevocationSemantics {
    /// A revoked/expired attestation is refused at the next admission (v2.2).
    RefuseAtNextLoad = 0,
    /// v2.5 reserved: drain-and-refuse a running Spirit. Treated as
    /// refuse-at-next-load at the load boundary in v2.2.
    DrainAndRefuse = 1,
}

/// Upgrade semantics for a new manifest version (ADV-056-1, AC3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SuccessorPolicy {
    /// The attestation binds exactly one manifest hash; a new version needs its
    /// own fresh attestation (the exact-hash flap is the feature).
    ExactOnly = 0,
    /// A new version requires a re-issued attestation via an expedited review
    /// path. Still exact-hash-bound; this is a policy hint for the vetter flow.
    ReissueRequiredWithExpeditedReview = 1,
}

/// The inner claim `claim_bytes` canonical-CBOR-encodes.
///
/// Binds the manifest exact-hash, the tier transition, the vetter key id, the
/// validity window, revocation semantics, and the optional successor policy.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VettingClaim {
    /// `sha256(manifest_toml)` raw bytes — the exact-hash the attestation binds
    /// (NOT the canonical manifest form).
    pub manifest_hash: [u8; 32],
    /// Spirit identity the attestation vets.
    pub spirit_id: String,
    /// Spirit version the attestation vets.
    pub spirit_version: String,
    /// Tier the Spirit is promoted FROM (provenance).
    pub from_tier: TrustTier,
    /// Tier the Spirit is promoted TO — must be `public-vetted`.
    pub to_tier: TrustTier,
    /// Stable identifier of the vetter key (hex of the vetter pubkey by
    /// convention; the crypto anchor is the pubkey that signed the envelope).
    pub vetter_key_id: String,
    /// Issuance wall-clock (ms). The enrollment MUST predate this.
    pub issued_at_unix_ms: u64,
    /// Expiry wall-clock (ms). `now >= expires_at` ⇒ ExpiryLapse.
    pub expires_at_unix_ms: u64,
    /// What a revocation does to the promotion.
    pub revocation_semantics: RevocationSemantics,
    /// Optional upgrade successor policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successor_policy: Option<SuccessorPolicy>,
}

/// Encode a [`VettingClaim`] to canonical CBOR (the ISSUE codec). Struct fields
/// serialize in declaration order with definite lengths (see `canonical_cbor`
/// rationale) so the encoding is byte-stable across hosts.
pub fn encode_claim(claim: &VettingClaim) -> Vec<u8> {
    match serde_cbor::to_vec(claim) {
        Ok(bytes) => bytes,
        Err(error) => panic!("VettingClaim CBOR encoding for attestation signing failed: {error}"),
    }
}

/// Decode `claim_bytes` back to a [`VettingClaim`] (the VERIFY codec — the
/// independent inverse of [`encode_claim`], never a re-encode).
pub fn decode_claim(bytes: &[u8]) -> Result<VettingClaim, String> {
    serde_cbor::from_slice(bytes).map_err(|e| e.to_string())
}

/// Issue (sign) a vetting attestation with a raw 32-byte vetter Ed25519 seed.
///
/// Sets `vetter_pubkey` from the seed and signs `sha256(canonical_cbor(claim))`.
pub fn issue_attestation(vetter_seed: &[u8; 32], claim: &VettingClaim) -> VettingAttestation {
    let claim_bytes = encode_claim(claim);
    let sign_bytes = sha256(&claim_bytes);
    let signature = ed25519_sign(vetter_seed, &sign_bytes);
    let vetter_pubkey = ed25519_pubkey(vetter_seed);
    VettingAttestation {
        signature,
        vetter_pubkey,
        claim_bytes,
        signing_alg: SigningAlg::Ed25519,
    }
}

/// Verify ONLY the attestation's own signature and decode its claim — the
/// codec-independent step. Re-derives `sha256` over the on-wire `claim_bytes`
/// and checks the Ed25519 signature under `vetter_pubkey`; then decodes the
/// claim. Does NOT walk the enrollment chain (that is [`super::verify_attestation`]).
pub fn verify_attestation_signature(
    att: &VettingAttestation,
) -> Result<VettingClaim, VettingRejection> {
    if att.signing_alg != SigningAlg::Ed25519 {
        return Err(VettingRejection::UnsupportedSigningAlg);
    }
    let sign_bytes = sha256(&att.claim_bytes);
    if !ed25519_verify(&att.vetter_pubkey, &sign_bytes, &att.signature) {
        return Err(VettingRejection::ForgedSignature);
    }
    decode_claim(&att.claim_bytes).map_err(VettingRejection::MalformedClaim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn sample_claim() -> VettingClaim {
        VettingClaim {
            manifest_hash: [0xAB; 32],
            spirit_id: "vetted-spirit".into(),
            spirit_version: "0.1.0".into(),
            from_tier: TrustTier::PublicUntrusted,
            to_tier: TrustTier::PublicVetted,
            vetter_key_id: "vetter-01".into(),
            issued_at_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
            revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
            successor_policy: Some(SuccessorPolicy::ExactOnly),
        }
    }

    #[test]
    fn claim_cbor_round_trips() {
        let claim = sample_claim();
        let bytes = encode_claim(&claim);
        let back = decode_claim(&bytes).unwrap();
        assert_eq!(claim, back);
    }

    #[test]
    fn canonical_encoding_is_deterministic() {
        let claim = sample_claim();
        assert_eq!(encode_claim(&claim), encode_claim(&claim));
    }

    /// Golden byte-pin: the canonical claim encoding is frozen. Any accidental
    /// schema/field-order/encoding change reds here (AC1 golden byte-pin).
    #[test]
    fn claim_encoding_golden_byte_pin() {
        let claim = sample_claim();
        let hex_bytes = hex::encode(encode_claim(&claim));
        assert_eq!(
            hex_bytes, GOLDEN_CLAIM_HEX,
            "canonical claim encoding drifted"
        );
    }

    #[test]
    fn issue_then_verify_signature_round_trip() {
        let s = seed(7);
        let claim = sample_claim();
        let att = issue_attestation(&s, &claim);
        assert_eq!(att.vetter_pubkey, ed25519_pubkey(&s));
        let decoded = verify_attestation_signature(&att).unwrap();
        assert_eq!(decoded, claim);
    }

    #[test]
    fn forged_signature_is_rejected() {
        let s = seed(7);
        let claim = sample_claim();
        let mut att = issue_attestation(&s, &claim);
        att.signature[0] ^= 0xFF;
        assert_eq!(
            verify_attestation_signature(&att),
            Err(VettingRejection::ForgedSignature)
        );
    }

    #[test]
    fn tampered_claim_bytes_break_signature() {
        let s = seed(7);
        let claim = sample_claim();
        let mut att = issue_attestation(&s, &claim);
        // Flip a claim byte without re-signing — signature (over sha256) fails.
        att.claim_bytes[0] ^= 0xFF;
        assert_eq!(
            verify_attestation_signature(&att),
            Err(VettingRejection::ForgedSignature)
        );
    }

    #[test]
    fn envelope_serde_round_trips() {
        let s = seed(3);
        let att = issue_attestation(&s, &sample_claim());
        let bytes = serde_cbor::to_vec(&att).unwrap();
        let back: VettingAttestation = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(att, back);
    }

    // Pinned during first green run (see verification notes).
    const GOLDEN_CLAIM_HEX: &str = "aa6d6d616e69666573745f68617368982018ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab18ab697370697269745f69646d7665747465642d7370697269746e7370697269745f76657273696f6e65302e312e306966726f6d5f74696572707075626c69635f756e7472757374656467746f5f746965726d7075626c69635f7665747465646d7665747465725f6b65795f6964697665747465722d3031716973737565645f61745f756e69785f6d731903e872657870697265735f61745f756e69785f6d731907d0747265766f636174696f6e5f73656d616e74696373737265667573655f61745f6e6578745f6c6f616470737563636573736f725f706f6c6963796a65786163745f6f6e6c79";
}
