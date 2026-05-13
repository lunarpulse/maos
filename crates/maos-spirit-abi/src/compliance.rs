//! Binding-v0.1 ComplianceClaim schema types.
//!
//! These types are committed under the joint Mary+Winston adversarial
//! review at `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`.
//! Story 1b.4 freezes the envelope shape and bumps `ABI_VERSION` to 1;
//! serde derives + `Uuid` wiring lands at that freeze, NOT in this story.
//!
//! All field names are stable from v0.1-α onward — renaming any field is
//! an ABI break per §8.5 (review report §5 self-test row #2).

extern crate alloc;
use alloc::{collections::BTreeSet, string::String, vec::Vec};

/// Ed25519-signed compliance claim envelope.
///
/// Canonical encoding for signature verification:
/// `sign_bytes = sha256(claim_bytes)`. The signature signs the claim
/// payload INDIRECTLY via its SHA-256 hash, keeping the envelope
/// fixed-size and the signature verifiable without CBOR-parsing the
/// claim at the verify step.
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

/// Signing algorithm identifier.
///
/// Additive enum: adding a variant at the end with explicit `#[repr(u8)]`
/// discriminant is NOT an ABI break. Removing or reordering variants IS
/// an ABI break per §8.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SigningAlg {
    /// Ed25519 signature algorithm.
    Ed25519 = 0,
}

/// Execution-context fingerprint bound to every ComplianceClaim.
///
/// The canonical encoding for hash purposes is:
/// `fingerprint_hash = sha256(cbor_canonical(ser(&self)))`
/// using RFC 8949 canonical CBOR.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityId(pub String);

/// Provider endpoint pin per ADR-005 pluggable provider drivers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEndpointPin {
    /// Provider identifier (e.g., "anthropic", "openai", "ollama").
    pub provider_id: String,
    /// Base URL of the provider's inference endpoint.
    pub endpoint_url: String,
    /// Optional model identifier pin (model-version pinning ships at v1.0 per NFR-Sec-15).
    pub model_id: Option<String>,
}

/// Crypto provider identifier per §8.6 pluggable crypto trait.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CryptoProviderId(pub String);

/// The inner claim payload — what `claim_bytes` CBOR-encodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    /// UUID v4 unique claim identifier.
    /// At v0.1-α this is a zero-cost newtype around `[u8; 16]`;
    /// Story 1b.4 swaps in the real `uuid::Uuid` when serde derives ship.
    pub claim_id: Uuid,
    /// Unix timestamp (milliseconds) when the claim was issued.
    pub issued_at_unix_ms: u64,
    /// Optional expiry timestamp. None = no automatic expiry.
    pub expires_at_unix_ms: Option<u64>,
    /// Principles the claim attests compliance with.
    pub principle_refs: Vec<PrincipleRef>,
    /// Evidence supporting the claim.
    pub evidence: Vec<EvidenceKind>,
    /// Verdict rendered by the attesting party.
    pub verdict: Verdict,
}

/// Zero-cost newtype wrapper around `[u8; 16]` at v0.1-α.
/// Replaced with `uuid::Uuid` in Story 1b.4.
///
/// The inner array is `pub(crate)` — private constructor is a deliberate
/// ABI stability guarantee. External crates cannot construct a `Uuid`
/// directly; only `maos-spirit-abi` internals and tests may do so.
/// Story 1b.4 swaps this for `uuid::Uuid` when serde derives ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid(pub(crate) [u8; 16]);

/// Principles the claim attests compliance with.
///
/// Additive enum with explicit discriminants. Adding a variant at the
/// end is NOT an ABI break; removing or reordering IS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    UnknownPrinciple = 255,
}

/// Evidence supporting a compliance claim.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
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

        // Verdict carries payload variants, so `as u8` is invalid for the
        // whole enum. The `#[repr(u8)]` attribute with explicit discriminants
        // guarantees the layout; we rely on the compiler for stability.
        //
        // NOTE: `AdmitWithCaveats { caveats: Vec<String> } = 1` cannot be
        // mechanically verified via `as u8` in stable Rust (payload variants
        // don't support discriminant casting — see rust-lang/rust#89520).
        // The explicit discriminant in the enum definition IS the guarantee;
        // a future const-assertion pattern may close this gap.
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
}
