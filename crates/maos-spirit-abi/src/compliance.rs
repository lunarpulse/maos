//! Binding-v0.1 ComplianceClaim schema types — **FROZEN**.
//!
//! These types are committed under the joint Mary+Winston adversarial
//! review at `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`.
//! Story 1b.4 froze the schema, added serde derives, resolved the `Uuid`
//! newtype decision, and bumped `ABI_VERSION` to `1`. This freeze is the
//! **one sanctioned ABI break** in Epic 1b.
//!
//! # §8.5 ABI-break rule (review report §5 self-test)
//!
//! The following changes **DO** bump `ABI_VERSION` (mechanical enforcement
//! via the `abi-diff` gate, now baselined at `v1-pre-bump.txt`):
//!
//! | # | Change | ABI Break? |
//! |---|---|---|
//! | 1 | Add required field without `#[serde(default)]` | **YES** |
//! | 2 | Rename any field | **YES** |
//! | 3 | Remove any field | **YES** |
//! | 4 | Change any field's type | **YES** |
//! | 5 | Reorder `Verdict` / `PrincipleRef` / `EvidenceKind` variants without updating explicit `#[repr(u8)]` discriminants | **YES** |
//! | 6 | Remove an enum variant from `Verdict` / `PrincipleRef` / `EvidenceKind` | **YES** |
//!
//! The following changes **DO NOT** bump `ABI_VERSION`:
//!
//! | # | Change | ABI Break? |
//! |---|---|---|
//! | 7 | Add optional field with `#[serde(default, skip_serializing_if = "Option::is_none")]` | **NO** |
//! | 8 | Add enum variant at the end with explicit `#[repr(u8)]` discriminant and `#[serde(other)]` fallback on the enum | **NO** |
//!
//! The abi-diff gate (`--deny removed --deny changed`, baselined at
//! `v1-pre-bump.txt`) is the mechanical half of this rule; this doc-block
//! is the canonical human-readable reference.

extern crate alloc;
use alloc::{collections::BTreeSet, format, string::String, vec::Vec};

/// Ed25519-signed compliance claim envelope.
///
/// Canonical encoding for signature verification:
/// `sign_bytes = sha256(claim_bytes)`. The signature signs the claim
/// payload INDIRECTLY via its SHA-256 hash, keeping the envelope
/// fixed-size and the signature verifiable without CBOR-parsing the
/// claim at the verify step.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
///
/// let envelope = ComplianceClaimEnvelope {
///     signature: [0u8; 64],       // Replace with real Ed25519 signature
///     attester_pubkey: [0u8; 32], // Replace with real public key
///     claim_bytes: vec![],        // CBOR-encoded Claim
///     signing_alg: SigningAlg::Ed25519,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplianceClaimEnvelope {
    /// Ed25519 signature over `sha256(claim_bytes)`. 64 bytes.
    pub signature: [u8; 64],
    /// Ed25519 public key of the attesting party. 32 bytes.
    pub attester_pubkey: [u8; 32],
    /// Canonical CBOR-encoded `Claim` the signature covers (RFC 8949 canonical).
    pub claim_bytes: Vec<u8>,
    /// Signing algorithm identifier.
    pub signing_alg: SigningAlg,
}

impl serde::Serialize for ComplianceClaimEnvelope {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ComplianceClaimEnvelope", 4)?;
        state.serialize_field("signature", &self.signature[..])?;
        state.serialize_field("attester_pubkey", &self.attester_pubkey[..])?;
        state.serialize_field("claim_bytes", &self.claim_bytes)?;
        state.serialize_field("signing_alg", &self.signing_alg)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for ComplianceClaimEnvelope {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            signature: Vec<u8>,
            attester_pubkey: Vec<u8>,
            claim_bytes: Vec<u8>,
            signing_alg: SigningAlg,
        }
        let helper = Helper::deserialize(deserializer)?;
        let signature = helper.signature.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64-byte signature, got {} bytes", v.len()))
        })?;
        let attester_pubkey = helper.attester_pubkey.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 32-byte pubkey, got {} bytes", v.len()))
        })?;
        Ok(ComplianceClaimEnvelope {
            signature,
            attester_pubkey,
            claim_bytes: helper.claim_bytes,
            signing_alg: helper.signing_alg,
        })
    }
}

/// Signing algorithm identifier.
///
/// Additive enum: adding a variant at the end with explicit `#[repr(u8)]`
/// discriminant is NOT an ABI break. Removing or reordering variants IS
/// an ABI break per §8.5.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::SigningAlg;
///
/// let alg = SigningAlg::Ed25519;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SigningAlg {
    /// Ed25519 signature algorithm.
    Ed25519 = 0,
}

/// Execution-context fingerprint bound to every ComplianceClaim.
///
/// The canonical encoding for hash purposes is:
/// `fingerprint_hash = sha256(cbor_canonical(ser(&self)))`
/// using RFC 8949 canonical CBOR.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::{
///     ExecutionContextFingerprint, TrustTier, SandboxTier,
///     CapabilityId, ProviderEndpointPin, CryptoProviderId,
/// };
/// use std::collections::BTreeSet;
///
///
/// let fingerprint = ExecutionContextFingerprint {
///     manifest_hash: [0u8; 32],
///     spirit_version: "1.0.0".into(),
///     trust_tier: TrustTier::PublicVetted,
///     sandbox_tier: SandboxTier::T2,
///     capability_scope: {
///         let mut set = BTreeSet::new();
///         set.insert(CapabilityId("model.invoke".into()));
///         set
///     },
///     provider_endpoint: ProviderEndpointPin {
///         provider_id: "anthropic".into(),
///         endpoint_url: "https://api.anthropic.com".into(),
///         model_id: None,
///     },
///     crypto_provider: CryptoProviderId("ring".into()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionContextFingerprint {
    /// SHA-256 of the Spirit's manifest.toml after canonicalization.
    pub manifest_hash: [u8; 32],
    /// Semantic version of the Spirit whose manifest was hashed.
    pub spirit_version: String,
    /// Trust tier per ADR-009.
    pub trust_tier: TrustTier,
    /// Sandbox tier per ADR-004.
    pub sandbox_tier: SandboxTier,
    /// Capability IDs the Spirit is authorized to hold.
    /// Sorted (BTreeSet) for canonical hash stability.
    pub capability_scope: BTreeSet<CapabilityId>,
    /// Provider + endpoint + optional model-version pin per ADR-005.
    pub provider_endpoint: ProviderEndpointPin,
    /// Pluggable crypto provider identity per §8.6.
    pub crypto_provider: CryptoProviderId,
}

/// Trust tier — operator-visible metadata classification.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::TrustTier;
///
/// let tier = TrustTier::PublicVetted;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    /// Operator-authored or personally vetted.
    Local = 0,
    /// Organization-internal, vouched by operator signing key.
    OrgInternal = 1,
    /// Public, vetted by community or operator.
    PublicVetted = 2,
    /// Public, no organizational vouch.
    PublicUntrusted = 3,
}

/// Sandbox tier — OS-native sandbox primitive classification.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::SandboxTier;
///
/// let tier = SandboxTier::T2;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SandboxTier {
    /// Trusted — no additional sandbox.
    T0 = 0,
    /// UID separation.
    T1 = 1,
    /// Landlock+seccomp narrow / Seatbelt / Windows restricted-token.
    T2 = 2,
    /// T2 + container.
    T3 = 3,
    /// WASM-component sandbox (speculative-vNext).
    T4 = 4,
}

/// Capability identifier — sorted, canonical, hash-stable.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::CapabilityId;
///
/// let id = CapabilityId("model.invoke".into());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct CapabilityId(pub String);

/// Provider endpoint pin per ADR-005 pluggable provider drivers.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::ProviderEndpointPin;
///
/// let pin = ProviderEndpointPin {
///     provider_id: "anthropic".into(),
///     endpoint_url: "https://api.anthropic.com".into(),
///     model_id: Some("claude-3-7-sonnet".into()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderEndpointPin {
    /// Provider identifier (e.g., "anthropic", "openai", "ollama").
    pub provider_id: String,
    /// Base URL of the provider's inference endpoint.
    pub endpoint_url: String,
    /// Optional model identifier pin (model-version pinning ships at v1.0 per NFR-Sec-15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// Crypto provider identifier per §8.6 pluggable crypto trait.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::CryptoProviderId;
///
/// let id = CryptoProviderId("ring".into());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct CryptoProviderId(pub String);

/// The inner claim payload — what `claim_bytes` CBOR-encodes.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::{
///     Claim, Uuid, PrincipleRef, EvidenceKind, Verdict,
/// };
///
/// let claim = Claim {
///     claim_id: Uuid::from_bytes([1u8; 16]),
///     issued_at_unix_ms: 1717200000000,
///     expires_at_unix_ms: Some(1717300000000),
///     principle_refs: vec![PrincipleRef::Soc2TypeIi],
///     evidence: vec![EvidenceKind::ManualReview {
///         reviewer_id: "reviewer-42".into(),
///     }],
///     verdict: Verdict::Admit,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    /// UUID v4 unique claim identifier.
    pub claim_id: Uuid,
    /// Unix timestamp (milliseconds) when the claim was issued.
    pub issued_at_unix_ms: u64,
    /// Optional expiry timestamp. None = no automatic expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix_ms: Option<u64>,
    /// Principles the claim attests compliance with.
    pub principle_refs: Vec<PrincipleRef>,
    /// Evidence supporting the claim.
    pub evidence: Vec<EvidenceKind>,
    /// Verdict rendered by the attesting party.
    pub verdict: Verdict,
}

/// Zero-cost newtype wrapper around `[u8; 16]` — **frozen decision**.
///
/// The review report §1.3 names the type `Uuid`; it does NOT mandate the
/// `uuid` crate. Keeping the newtype: zero new deps, identical 16-byte wire
/// shape, less abi-diff churn. The `uuid` crate's serde impl emits a *string*
/// in human-readable formats and *bytes* in binary formats — using the newtype
/// guarantees `[u8;16]` in all codecs without qualification.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::Uuid;
///
/// let id = Uuid::from_bytes([0xAB; 16]);
/// assert_eq!(id.as_bytes(), &[0xAB; 16]);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Uuid([u8; 16]);

impl Uuid {
    /// Construct a `Uuid` from its raw 16-byte representation.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the underlying 16-byte array.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Principles the claim attests compliance with.
///
/// Additive enum with explicit discriminants. Adding a variant at the
/// end with `#[serde(other)]` fallback is NOT an ABI break; removing or
/// reordering IS.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::PrincipleRef;
///
/// let principle = PrincipleRef::Soc2TypeIi;
/// assert_eq!(principle as u8, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum PrincipleRef {
    /// HIPAA §164.308 compliance.
    Hipaa164308 = 0,
    /// SOC 2 Type II compliance.
    Soc2TypeIi = 1,
    /// ISO 27001 compliance.
    Iso27001 = 2,
    /// EU AI Act Article 14 compliance.
    EuAiActArt14 = 3,
    /// Unknown or future principle — fallback.
    #[serde(other)]
    UnknownPrinciple = 255,
}

/// Evidence supporting a compliance claim.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::EvidenceKind;
///
/// let corpus = EvidenceKind::CorpusReplay {
///     corpus_sha256: [0xAA; 32],
/// };
///
/// let review = EvidenceKind::ManualReview {
///     reviewer_id: "eng-lead-01".into(),
/// };
///
/// assert!(matches!(corpus, EvidenceKind::CorpusReplay { .. }));
/// assert!(matches!(review, EvidenceKind::ManualReview { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Replay of a calibration corpus whose SHA-256 is attested.
    CorpusReplay {
        /// SHA-256 of the calibration corpus JSONL file.
        corpus_sha256: [u8; 32],
    },
    /// Reference to an external penetration test report.
    PenTestReportRef {
        /// URL where the report can be retrieved.
        url: String,
    },
    /// Manual review performed by a named reviewer.
    ManualReview {
        /// Identifier of the human reviewer who performed the review.
        reviewer_id: String,
    },
    /// Cross-Spirit agreement evidence from a multi-Spirit evaluation.
    CrossSpiritAgreement {
        /// Spirit IDs of the participating Spirits.
        participants: Vec<String>,
        /// Agreement rate among participants.
        agreement_rate: f64,
    },
}

/// Verdict rendered by the attesting party.
///
/// Explicit discriminants on every variant — reordering without updating
/// discriminants is an ABI break per §8.5 self-test row #5.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::compliance::Verdict;
///
/// let verdict = Verdict::AdmitWithCaveats {
///     caveats: vec!["Model version not pinned".into()],
/// };
///
/// match verdict {
///     Verdict::Admit => { /* admitted */ }
///     Verdict::AdmitWithCaveats { caveats } => {
///         assert_eq!(caveats.len(), 1);
///     }
///     _ => { /* other verdict */ }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Claim is valid; Spirit may be admitted under attested context.
    Admit = 0,
    /// Admitted but with operator-visible caveats.
    AdmitWithCaveats {
        /// Human-readable caveat strings.
        caveats: Vec<String>,
    } = 1,
    /// Rejected: runtime context has drifted from attested context.
    RejectContextDrift = 2,
    /// Rejected: claim is structurally malformed.
    RejectMalformedClaim = 3,
    /// Rejected: claim has expired.
    RejectExpiredClaim = 4,
    /// Unknown or future verdict — fallback.
    #[serde(other)]
    UnknownVerdict = 255,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn envelope_construction_roundtrip() {
        let env = ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![0xA1, 0x01, 0x02],
            signing_alg: SigningAlg::Ed25519,
        };
        assert_eq!(env.signature.len(), 64);
        assert_eq!(env.attester_pubkey.len(), 32);
        assert_eq!(env.claim_bytes, vec![0xA1, 0x01, 0x02]);
        assert!(matches!(env.signing_alg, SigningAlg::Ed25519));
    }

    #[test]
    fn enum_discriminants_are_stable() {
        assert_eq!(TrustTier::Local as u8, 0);
        assert_eq!(TrustTier::OrgInternal as u8, 1);
        assert_eq!(TrustTier::PublicVetted as u8, 2);
        assert_eq!(TrustTier::PublicUntrusted as u8, 3);

        assert_eq!(SandboxTier::T0 as u8, 0);
        assert_eq!(SandboxTier::T1 as u8, 1);
        assert_eq!(SandboxTier::T2 as u8, 2);
        assert_eq!(SandboxTier::T3 as u8, 3);
        assert_eq!(SandboxTier::T4 as u8, 4);

        assert_eq!(PrincipleRef::EuAiActArt14 as u8, 3);
        assert_eq!(PrincipleRef::UnknownPrinciple as u8, 255);
    }

    #[test]
    fn provider_endpoint_pin_model_id_is_optional() {
        let pin = ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: None,
        };
        assert!(pin.model_id.is_none());
    }

    #[test]
    fn evidence_kind_variants_distinct() {
        let e1 = EvidenceKind::CorpusReplay {
            corpus_sha256: [0u8; 32],
        };
        let e2 = EvidenceKind::PenTestReportRef {
            url: "https://example.com/report".into(),
        };
        let e3 = EvidenceKind::ManualReview {
            reviewer_id: "alice".into(),
        };
        let e4 = EvidenceKind::CrossSpiritAgreement {
            participants: vec!["s1".into(), "s2".into()],
            agreement_rate: 0.95,
        };
        assert!(matches!(e1, EvidenceKind::CorpusReplay { .. }));
        assert!(matches!(e2, EvidenceKind::PenTestReportRef { .. }));
        assert!(matches!(e3, EvidenceKind::ManualReview { .. }));
        assert!(matches!(e4, EvidenceKind::CrossSpiritAgreement { .. }));
    }

    #[test]
    fn uuid_constructor_pair() {
        let bytes = [0xABu8; 16];
        let uuid = Uuid::from_bytes(bytes);
        assert_eq!(uuid.as_bytes(), &bytes);
    }
    /// Story 9.3b AC-Group C #4 — frozen `Claim` byte-unchanged regression.
    ///
    /// A pinned JSON snapshot of a representative `Claim`.  Any change to
    /// field names, field order, or serialization shape breaks this test and
    /// therefore requires an `ABI_VERSION` bump per §8.5.
    #[test]
    fn claim_json_snapshot_is_unchanged() {
        let claim = Claim {
            claim_id: Uuid::from_bytes([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB,
                0xCD, 0xEF,
            ]),
            issued_at_unix_ms: 1_717_000_000_000,
            expires_at_unix_ms: Some(1_717_086_400_000),
            principle_refs: vec![PrincipleRef::EuAiActArt14],
            evidence: vec![EvidenceKind::CorpusReplay {
                corpus_sha256: [0xAB; 32],
            }],
            verdict: Verdict::Admit,
        };

        let json = serde_json::to_string(&claim).expect("Claim serialization must not fail");

        // Golden snapshot — update ONLY alongside an ABI_VERSION bump.
        // The exact shape is derived from the field declaration order and
        // the serde attributes on `Claim`.
        let expected = r#"{"claim_id":[1,35,69,103,137,171,205,239,1,35,69,103,137,171,205,239],"issued_at_unix_ms":1717000000000,"expires_at_unix_ms":1717086400000,"principle_refs":["eu_ai_act_art14"],"evidence":[{"kind":"corpus_replay","corpus_sha256":[171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171,171]}],"verdict":"admit"}"#;

        assert_eq!(
            json, expected,
            "Claim JSON snapshot changed; if intentional, bump ABI_VERSION and update this golden value"
        );
    }

    /// §8.5 additive-tolerance: optional fields with `#[serde(default, skip_serializing_if)]`
    /// and `#[serde(other)]` enum fallback variants deserialize forward-compatibly
    /// WITHOUT requiring an `ABI_VERSION` bump. This is the missing half of the
    /// §8.5 self-test — the breaking-detection half is `claim_json_snapshot_is_unchanged`.
    #[test]
    fn additive_fields_and_unknown_variants_no_bump() {
        // 1. Serialize a Claim WITHOUT the optional `expires_at_unix_ms` field,
        //    then deserialize back — proves #[serde(default, skip_serializing_if)]
        //    round-trips cleanly.
        let claim_no_expiry = Claim {
            claim_id: Uuid::from_bytes([0u8; 16]),
            issued_at_unix_ms: 1_700_000_000_000,
            expires_at_unix_ms: None,
            principle_refs: vec![PrincipleRef::Hipaa164308],
            evidence: vec![],
            verdict: Verdict::Admit,
        };
        let json = serde_json::to_string(&claim_no_expiry).unwrap();
        // The optional field must be ABSENT from the JSON (skip_serializing_if).
        assert!(
            !json.contains("expires_at_unix_ms"),
            "optional None field must be skipped in serialization"
        );
        let roundtrip: Claim = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.expires_at_unix_ms, None);

        // 2. Feed an unknown enum tag to PrincipleRef → must deserialize to
        //    UnknownPrinciple (the #[serde(other)] fallback), proving forward-compat
        //    for additive enum variants.
        let unknown_principle_json = r#""some_future_principle""#;
        let p: PrincipleRef = serde_json::from_str(unknown_principle_json).unwrap();
        assert_eq!(p, PrincipleRef::UnknownPrinciple);

        // 3. Feed an unknown verdict tag → must deserialize to UnknownVerdict.
        let unknown_verdict_json = r#""some_future_verdict""#;
        let v: Verdict = serde_json::from_str(unknown_verdict_json).unwrap();
        assert!(matches!(v, Verdict::UnknownVerdict));

        // 4. ProviderEndpointPin.model_id with #[serde(default, skip_serializing_if)]
        //    round-trips None cleanly.
        let pin = ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: None,
        };
        let pin_json = serde_json::to_string(&pin).unwrap();
        assert!(
            !pin_json.contains("model_id"),
            "optional None model_id must be skipped"
        );
        let pin_rt: ProviderEndpointPin = serde_json::from_str(&pin_json).unwrap();
        assert_eq!(pin_rt.model_id, None);
    }
}
