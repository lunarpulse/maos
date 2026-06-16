---
title: compliance
sidebar_position: 4
description: "ComplianceClaim envelope — Ed25519-signed attestation schema, Verdict, PrincipleRef, and EvidenceKind types."
---

# `compliance` Module

The compliance module defines the **ComplianceClaim** schema — Ed25519-signed envelopes that attest a Spirit's compliance with regulatory principles. The schema was **frozen** at Story 1b.4 under joint adversarial review.

## ComplianceClaimEnvelope

The top-level signed envelope. The signature covers `sha256(claim_bytes)`, keeping the envelope fixed-size and the signature verifiable without CBOR-parsing the inner claim.

```rust
pub struct ComplianceClaimEnvelope {
    /// Ed25519 signature over sha256(claim_bytes). 64 bytes.
    pub signature: [u8; 64],
    /// Ed25519 public key of the attesting party. 32 bytes.
    pub attester_pubkey: [u8; 32],
    /// Canonical CBOR-encoded Claim (RFC 8949 canonical).
    pub claim_bytes: Vec<u8>,
    /// Signing algorithm identifier.
    pub signing_alg: SigningAlg,
}
```

### Example: Constructing an Envelope

```rust
use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

let envelope = ComplianceClaimEnvelope {
    signature: [0u8; 64],       // Replace with real Ed25519 signature
    attester_pubkey: [0u8; 32], // Replace with real public key
    claim_bytes: vec![],        // CBOR-encoded Claim
    signing_alg: SigningAlg::Ed25519,
};
```

## SigningAlg

Signing algorithm identifier. Additive enum — new variants can be appended without an ABI break.

```rust
#[repr(u8)]
pub enum SigningAlg {
    Ed25519 = 0,
}
```

## Claim

The inner claim payload — what `claim_bytes` CBOR-encodes.

```rust
pub struct Claim {
    pub claim_id: Uuid,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: Option<u64>,  // None = no automatic expiry
    pub principle_refs: Vec<PrincipleRef>,
    pub evidence: Vec<EvidenceKind>,
    pub verdict: Verdict,
}
```

### Example: Building a Claim

```rust
use maos_spirit_abi::compliance::{Claim, Uuid, PrincipleRef, EvidenceKind, Verdict};

let claim = Claim {
    claim_id: Uuid::from_bytes([1u8; 16]),
    issued_at_unix_ms: 1717200000000,
    expires_at_unix_ms: Some(1717300000000),
    principle_refs: vec![PrincipleRef::Soc2TypeIi],
    evidence: vec![EvidenceKind::ManualReview {
        reviewer_id: "reviewer-42".into(),
    }],
    verdict: Verdict::Admit,
};
```

## Uuid

Zero-cost newtype wrapper around `[u8; 16]`. Uses a fixed 16-byte wire shape in all codecs (unlike the `uuid` crate which emits strings in human-readable formats).

```rust
pub struct Uuid([u8; 16]);
```

### Example

```rust
use maos_spirit_abi::compliance::Uuid;

let id = Uuid::from_bytes([0xAB; 16]);
assert_eq!(id.as_bytes(), &[0xAB; 16]);
```

## ExecutionContextFingerprint

Execution-context fingerprint bound to every ComplianceClaim. Canonical encoding: `sha256(cbor_canonical(ser(&self)))` using RFC 8949.

```rust
pub struct ExecutionContextFingerprint {
    pub manifest_hash: [u8; 32],
    pub spirit_version: String,
    pub trust_tier: TrustTier,
    pub sandbox_tier: SandboxTier,
    pub capability_scope: BTreeSet<CapabilityId>,
    pub provider_endpoint: ProviderEndpointPin,
    pub crypto_provider: CryptoProviderId,
}
```

## TrustTier

Operator-visible metadata classification per ADR-009.

```rust
#[repr(u8)]
pub enum TrustTier {
    Local = 0,           // Operator-authored or personally vetted
    OrgInternal = 1,     // Organization-internal, vouched by operator signing key
    PublicVetted = 2,    // Public, vetted by community or operator
    PublicUntrusted = 3, // Public, no organizational vouch
}
```

## SandboxTier

OS-native sandbox primitive classification per ADR-004.

```rust
#[repr(u8)]
pub enum SandboxTier {
    T0 = 0,  // Trusted — no additional sandbox
    T1 = 1,  // UID separation
    T2 = 2,  // Landlock+seccomp narrow / Seatbelt / Windows restricted-token
    T3 = 3,  // T2 + container
    T4 = 4,  // WASM-component sandbox (speculative-vNext)
}
```

## PrincipleRef

Regulatory principles the claim attests compliance with. Additive enum with explicit discriminants.

```rust
#[repr(u8)]
pub enum PrincipleRef {
    Hipaa164308 = 0,      // HIPAA §164.308
    Soc2TypeIi = 1,       // SOC 2 Type II
    Iso27001 = 2,         // ISO 27001
    EuAiActArt14 = 3,     // EU AI Act Article 14
    UnknownPrinciple = 255, // Fallback for future principles (#[serde(other)])
}
```

## EvidenceKind

Evidence supporting a compliance claim. Tagged enum (`#[serde(tag = "kind")]`).

```rust
pub enum EvidenceKind {
    CorpusReplay { corpus_sha256: [u8; 32] },
    PenTestReportRef { url: String },
    ManualReview { reviewer_id: String },
    CrossSpiritAgreement { participants: Vec<String>, agreement_rate: f64 },
}
```

### Example: Evidence Variants

```rust
use maos_spirit_abi::compliance::EvidenceKind;

let corpus = EvidenceKind::CorpusReplay {
    corpus_sha256: [0xAA; 32],
};

let pen_test = EvidenceKind::PenTestReportRef {
    url: "https://example.com/report.pdf".into(),
};

let review = EvidenceKind::ManualReview {
    reviewer_id: "eng-lead-01".into(),
};

let agreement = EvidenceKind::CrossSpiritAgreement {
    participants: vec!["spirit-a".into(), "spirit-b".into()],
    agreement_rate: 0.95,
};
```

## Verdict

Verdict rendered by the attesting party. Explicit `#[repr(u8)]` discriminants on every variant.

```rust
#[repr(u8)]
pub enum Verdict {
    Admit = 0,
    AdmitWithCaveats { caveats: Vec<String> } = 1,
    RejectContextDrift = 2,
    RejectMalformedClaim = 3,
    RejectExpiredClaim = 4,
    UnknownVerdict = 255,  // Fallback (#[serde(other)])
}
```

### Example: Verdict Usage

```rust
use maos_spirit_abi::compliance::Verdict;

let verdict = Verdict::AdmitWithCaveats {
    caveats: vec!["Model version not pinned".into()],
};

match verdict {
    Verdict::Admit => println!("Admitted"),
    Verdict::AdmitWithCaveats { caveats } => {
        for caveat in &caveats {
            println!("Caveat: {caveat}");
        }
    }
    Verdict::RejectContextDrift => println!("Context drifted"),
    _ => println!("Other verdict"),
}
```

## Supporting Newtypes

### CapabilityId

Sorted, canonical, hash-stable capability identifier.

```rust
pub struct CapabilityId(pub String);
```

### CryptoProviderId

Pluggable crypto provider identity per §8.6.

```rust
pub struct CryptoProviderId(pub String);
```

### ProviderEndpointPin

Provider endpoint pin per ADR-005 pluggable provider drivers.

```rust
pub struct ProviderEndpointPin {
    pub provider_id: String,
    pub endpoint_url: String,
    pub model_id: Option<String>,  // Model-version pinning ships at v1.0
}
```
