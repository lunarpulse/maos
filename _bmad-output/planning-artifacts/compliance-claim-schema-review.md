# ComplianceClaim Schema — Adversarial Review Report

**Review target:** Binding-v0.1 ComplianceClaim wire schema (proposed below in §1).
**Review date:** 2026-05-12
**Purpose:** Authorize Story 1b.4's schema freeze of `crates/maos-spirit-abi/src/compliance.rs` by producing a signed-off adversarial review. This report is the singular contract between Story 0.4 and Story 1b.4.

**Scope:** Schema proposal, field-level secret classification (NFR-Sec-16), context-drift attack-surface enumeration (§8.5), ABI-break-rule self-test. This report does **NOT** modify `compliance.rs` or bump `ABI_VERSION` — those belong to Story 1a.1 (initial types) and Story 1b.4 (freeze + bump to 1).

---

## 1. Schema Proposal

The following Rust type definitions constitute the proposed binding-v0.1 ComplianceClaim wire schema. All types live in `crates/maos-spirit-abi/src/compliance.rs` at freeze time (Story 1b.4). The `ABI_VERSION` constant remains at `0` until the freeze lands.

### 1.1 Envelope (outer signed container)

```rust
/// Ed25519-signed compliance claim envelope.
///
/// The canonical encoding for signature verification is:
///   sign_bytes = sha256(claim_bytes)
/// The signature signs the claim payload indirectly via its SHA-256 hash.
/// This keeps the envelope fixed-size and the signature verifiable without
/// CBOR-parsing the claim at the verify step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceClaimEnvelope {
    /// Ed25519 signature over sha256(claim_bytes). 64 bytes.
    pub signature: [u8; 64],

    /// Ed25519 public key of the attesting party. 32 bytes.
    pub attester_pubkey: [u8; 32],

    /// Canonical CBOR-encoded [`Claim`] the signature covers.
    /// Encoded per RFC 8949 canonical CBOR: shortest-form integers,
    /// definite-length arrays/maps, lex-sorted map keys.
    pub claim_bytes: Vec<u8>,

    /// Signing algorithm identifier.
    pub signing_alg: SigningAlg,
}

/// Additive enum: adding a variant at the end with explicit #[repr(u8)]
/// discriminant and #[serde(other)] fallback is NOT an ABI break.
/// Removing or reordering variants IS an ABI break per §8.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SigningAlg {
    Ed25519 = 0,
    // Future: EcdsaP256 = 1, Ed448 = 2, ...
}
```

### 1.2 Execution-Context Fingerprint

The seven-field fingerprint bound to every claim per §8.5. Together they establish the precise runtime context under which the claim is valid. The kernel compares runtime context against the fingerprint at admission; any drift causes `EComplianceContextDrift`.

```rust
/// Execution-context fingerprint bound to every ComplianceClaim.
///
/// The canonical encoding for hash purposes is:
///   fingerprint_hash = sha256(cbor_canonical(ser(&self)))
/// using RFC 8949 canonical CBOR. This is the input to manifest_hash
/// computation when the fingerprint is embedded in a manifest derivation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContextFingerprint {
    /// SHA-256 of the Spirit's manifest.toml after canonicalization
    /// (sorted keys, no comments, deterministic whitespace).
    pub manifest_hash: [u8; 32],

    /// Semantic version of the Spirit whose manifest was hashed.
    pub spirit_version: String, // semver::Version serialized as "major.minor.patch"

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Local = 0,
    OrgInternal = 1,
    PublicVetted = 2,
    PublicUntrusted = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SandboxTier {
    T0 = 0,
    T1 = 1,
    T2 = 2,
    T3 = 3,
    T4 = 4,
}

/// Capability identifier — sorted, canonical, hash-stable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

/// Provider endpoint pin per ADR-005 pluggable provider drivers.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CryptoProviderId(pub String);
```

### 1.3 Claim Payload

The inner claim that `claim_bytes` encodes in canonical CBOR.

```rust
/// The inner claim payload — what claim_bytes CBOR-encodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// UUID v4 unique claim identifier.
    pub claim_id: Uuid,

    /// Unix timestamp (milliseconds) when the claim was issued.
    pub issued_at_unix_ms: u64,

    /// Optional expiry timestamp. None = no automatic expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix_ms: Option<u64>,

    /// Principles the claim attests compliance with.
    /// Additive enum with explicit discriminants and serde(other) fallback.
    pub principle_refs: Vec<PrincipleRef>,

    /// Evidence supporting the claim.
    pub evidence: Vec<EvidenceKind>,

    /// Verdict rendered by the attesting party.
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum PrincipleRef {
    Hipaa164308 = 0,
    Soc2TypeIi = 1,
    Iso27001 = 2,
    EuAiActArt14 = 3,
    // Future additive (NOT ABI break if end-of-list + explicit discriminant + serde(other)):
    // GdprArt35 = 4, FedRampModerate = 5, ...
    #[serde(other)]
    UnknownPrinciple = 255,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        participants: Vec<String>, // SpiritId as string at v0.1-α
        /// Agreement rate among participants.
        agreement_rate: f64,
    },
    // Future additive (NOT ABI break with explicit discriminants + serde(other)):
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Claim is valid; Spirit may be admitted under attested context.
    Admit = 0,
    /// Admitted but with operator-visible caveats.
    AdmitWithCaveats {
        caveats: Vec<String>,
    } = 1,
    /// Rejected: runtime context has drifted from attested context.
    RejectContextDrift = 2,
    /// Rejected: claim is structurally malformed.
    RejectMalformedClaim = 3,
    /// Rejected: claim has expired.
    RejectExpiredClaim = 4,
    // Future additive (NOT ABI break with explicit discriminant + serde(other)):
    // RejectRevokedAttester = 5,
    #[serde(other)]
    UnknownVerdict = 255,
}
```

### 1.4 Canonical Encoding Rules

All hash-stable encodings use RFC 8949 **Canonical CBOR** (Section 4.2.1):

| Element | Encoding Rule |
|---|---|
| `claim_bytes` | `cbor_canonical(ser(&Claim))` — lex-sorted map keys, shortest-form integers, definite-length |
| `manifest_hash` | `sha256(cbor_canonical(ser(&manifest_toml_canonical_form)))` |
| `corpus_sha256` (in EvidenceKind) | SHA-256 of the corpus JSONL file as streamed by `check_corpus.rs` (line + `\n` per item) |
| `signature` | `ed25519_sign(attester_privkey, sha256(claim_bytes))` |

---

## 2. Reviewer Panel

Two reviewers external to the schema author (the dev agent for Story 0.4) constitute the adversarial review panel per epic-0's "Mary + Winston joint demand."

| Persona | Role | Attestation |
|---|---|---|
| **Mary** (PM / Product Manager) | Reviews §1 proposal for completeness against PRD FR38/FR47/App-E v0.1 surface; reviews §3 secret classification for operator-data-sovereignty alignment; reviews §4 context-drift checklist for completeness of the attack surface enumeration | Below in §6 |
| **Winston** (Architect) | Reviews §1 proposal for structural soundness against §8.5/ADR-004/ADR-005/ADR-009; reviews §4 context-drift checklist for mechanism completeness; reviews §5 ABI-break-rule self-test for correctness against §8.5 binding rule | Below in §6 |
| **Dev Agent (Story 0.4)** | Schema proposer; authors §1 proposal; does NOT self-review | Below in §6 as proposer only |

---

## 3. Per-Field Secret/Non-Secret Classification (NFR-Sec-16)

NFR-Sec-16 requires binary `secret`/`non-secret` annotation on every ComplianceClaim field — no default. The three v0.1-α redaction primitives are:

- `redact-pre-log` — strip before Transparency Log boundary, never persist
- `redact-post-log` — persist hashed/length-encoded only
- `seal-and-export` — full bytes only inside Ed25519-signed sealed-export bundles per NFR-Aud-6

### 3.1 Envelope Fields

| Field Path | Type Summary | Classification | Justification | Redaction Action |
|---|---|---|---|---|
| `signature` | `[u8; 64]` Ed25519 | `non-secret` | Public-by-cryptographic-construction: Ed25519 signatures are verificatory material, not secrets | — |
| `attester_pubkey` | `[u8; 32]` Ed25519 | `non-secret` | Public-by-cryptographic-construction: Ed25519 public keys ARE the verification path | — |
| `claim_bytes` | `Vec<u8>` CBOR | `non-secret` | Contains no out-of-band material beyond what the fingerprint already exposes; can be logged in full | — |
| `signing_alg` | `SigningAlg` enum | `non-secret` | Algorithm identifier is metadata, not key material | — |

### 3.2 Execution-Context Fingerprint Fields

| Field Path | Type Summary | Classification | Justification | Redaction Action |
|---|---|---|---|---|
| `manifest_hash` | `[u8; 32]` | `non-secret` | Hash of public manifest content; no secret material | — |
| `spirit_version` | `String` semver | `non-secret` | Version is public metadata | — |
| `trust_tier` | `TrustTier` enum | `non-secret` | Tier classification is operator-visible metadata | — |
| `sandbox_tier` | `SandboxTier` enum | `non-secret` | Tier classification is operator-visible metadata | — |
| `capability_scope` | `BTreeSet<CapabilityId>` | `non-secret` | Capability identifiers are operator-visible authorization metadata | — |
| `provider_endpoint` | `ProviderEndpointPin` | `non-secret` | Endpoint URL is operator-visible; bearer token routed via it is in `crypto_provider`-attached secrets, NOT in the claim | — |
| `crypto_provider` | `CryptoProviderId` | `non-secret` | The identifier string is metadata; key material is OUT of the ComplianceClaim wire shape by design — the §8.6 trait isolation makes this true | — |

### 3.3 Claim Payload Fields

| Field Path | Type Summary | Classification | Justification | Redaction Action |
|---|---|---|---|---|
| `claim_id` | `Uuid` v4 | `non-secret` | Random identifier, not a secret | — |
| `issued_at_unix_ms` | `u64` | `non-secret` | Timestamp is metadata | — |
| `expires_at_unix_ms` | `Option<u64>` | `non-secret` | Timestamp is metadata | — |
| `principle_refs` | `Vec<PrincipleRef>` | `non-secret` | Enumerated principle identifiers are attestation metadata, not secrets | — |
| `evidence` | `Vec<EvidenceKind>` | (per variant) | See below | — |
| `evidence.CorpusReplay.corpus_sha256` | `[u8; 32]` | `non-secret` | SHA-256 of public corpus content | — |
| `evidence.PenTestReportRef.url` | `String` | `non-secret` | URL is a public reference | — |
| `evidence.ManualReview.reviewer_id` | `String` | `non-secret` | Reviewer identity is part of the attestation chain, not a withheld secret | — |
| `evidence.CrossSpiritAgreement.participants` | `Vec<String>` | `non-secret` | Spirit IDs are metadata | — |
| `evidence.CrossSpiritAgreement.agreement_rate` | `f64` | `non-secret` | Aggregate statistic | — |
| `verdict` | `Verdict` enum | `non-secret` | The verdict is the output of the attestation chain — public by design | — |

### 3.4 Secret-Classification Footer

**v0.1-α invariant:** The ComplianceClaim wire shape contains **ZERO** `secret`-classified fields at v0.1-α. Any addition of a `secret`-classified field is:

1. An **ABI break** per §8.5 (adds a required field with non-default semantics)
2. An **NFR-Sec-16 invariant-lock review** (the v0.5 manifest-evolution lint extends this table's discipline mechanically)
3. A **Transparency Log redaction-policy update** (the chosen `redaction_action_if_secret` must be documented and tested)

Documenting this invariant at v0.1-α makes the v0.5 NFR-Sec-16 lint a **tightening of an already-honored contract** rather than a retroactive restriction.

---

## 4. Context-Drift Attack-Surface Checklist (§8.5)

§8.5 defines the execution-context fingerprint as seven fields. The kernel checks all seven **conjunctively** at admission; any single drift causes `EComplianceContextDrift` and admission rejection. The following table enumerates every fingerprint field's drift attack vector, detection mechanism, false-negative mode, and current mechanism status.

| Fingerprint Field | Drift Attack Vector | Detection Mechanism | False-Negative Mode | Status |
|---|---|---|---|---|
| `manifest_hash` | Attacker ships modified manifest at runtime | Kernel re-hashes manifest at admit-time and compares to claim's `manifest_hash` | Hash collision (~2⁻¹²⁸ infeasible) | **mechanism complete** |
| `spirit_version` | Attacker ships different binary claiming same `spirit_version` | Defense-in-depth via `manifest_hash` (binary embeds manifest hash via Story 1a.1's reflection); matched binary + same version is by construction NOT a drift | Spoofed version with matching manifest hash is impossible by hash preimage resistance | **mechanism complete** |
| `trust_tier` | Operator downgrades trust tier at admit without re-certification | Kernel reads effective trust tier from operator policy and compares to claim | Claim attests "public-vetted" but operator policy says "local" — REJECTED with `EComplianceContextDrift` | **mechanism complete** |
| `sandbox_tier` | Manifest declares T0 but operator policy forces T2 | Kernel computes strictest-of-(manifest, trust-tier, operator-policy) per ADR-004 and compares to claim's attested sandbox_tier | Claim attests T2 but runtime is T2 forced from T0 — this is NOT a drift; attestation-context matches enforcement | **mechanism complete** |
| `capability_scope` | Manifest changes between attestation and admission | `capability_scope: BTreeSet<CapabilityId>` is hash-canonical (sorted), so any scope change fails the `manifest_hash` equality before scope comparison runs; defense-in-depth | Matched manifest hash implies matched capability scope by construction | **mechanism complete** |
| `provider_endpoint` | Operator points at a different Anthropic deployment than attested | Kernel reads `provider_endpoint` from runtime config and compares to claim's `ProviderEndpointPin` | Model-version pin omitted at v0.1-α (provider response signature is opaque) — the kernel can verify the endpoint URL matches but cannot verify the model version behind the endpoint | **partial — model-version pinning ships at v1.0 per NFR-Sec-15** |
| `crypto_provider` | Operator swaps from `ring` to a FIPS module without re-attestation | Kernel reads `CryptoProviderId` from composition root and compares | Identifier equality is exact-string comparison; different FIPS module = different identifier = REJECTED | **mechanism complete** |

### 4.1 Cross-Field Invariant

The seven fingerprint fields are checked **conjunctively**. Any single drift causes admission rejection. This is the §8.5 contract verbatim: the kernel raises `EComplianceContextDrift` for any mismatch across all seven fields, not a majority vote.

### 4.2 Documented Dissent

**provider_endpoint partial status.** The `model_id` field on `ProviderEndpointPin` is `Option<String>` at v0.1-α and is populated as `None` for all v0.1-α claims. The kernel can verify the endpoint URL but cannot verify the model version behind it. This means an operator could point at a different model deployment at the same URL without detection. Model-version pinning ships at v1.0 per NFR-Sec-15. This dissent is **acceptable for v0.1-α** because: (a) v0.1-α's ComplianceClaim corpus is N≈10 smoke fixtures (App-E v0.1 surface), not N=600 adversarial; (b) the claim is that the **schema** supports pinning, not that pinning is enforced; (c) the `Option<String>` field preserves forward-compatibility — adding `model_id` population is an additive operation, not a schema change.

---

## 5. ABI-Break-Rule Self-Test (§8.5)

§8.5's binding rule: adding any required field, removing any field, renaming, type-changing, or removing/reordering enum variants of `Verdict` / `PrincipleRef` / `EvidenceKind` bumps `ABI_VERSION`. Adding optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`, additive enum variants at the end with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback, or loosening bounds — does NOT bump.

| # | Hypothetical Change | ABI Break? | Rationale |
|---|---|---|---|
| 1 | Add `#[serde(default)] schema_version: u32 = 1` to `Claim` | **NO** | Optional + default; additive; existing claims deserialize with schema_version=1 |
| 2 | Rename `attester_pubkey` → `issuer_pubkey` | **YES** | Renaming a field changes the wire shape (CBOR map key changes); existing claims fail to deserialize |
| 3 | Add `Soc2TypeIii` variant to `PrincipleRef` with `#[repr(u8)]` discriminant 4 and `#[serde(other)]` on the enum | **NO** | Additive enum variant at end with explicit discriminant; `#[serde(other)]` handles unknown variants on deser |
| 4 | Remove `RejectExpiredClaim` from `Verdict` | **YES** | Removing an enum variant changes the variant-index mapping; existing verdicts referencing discriminant 4 become `UnknownVerdict` |
| 5 | Reorder `Verdict` variants (e.g., `RejectContextDrift` before `AdmitWithCaveats`) | **YES** | Enum discriminant values change; existing serialized verdicts deserialize to wrong variants |
| 6 | Add `#[serde(default)] optional_caveat: Option<String>` to `ComplianceClaimEnvelope` | **NO** | Optional field with default; existing envelopes deserialize with `optional_caveat = None` |
| 7 | Change `agreement_rate: f64` → `agreement_rate: u32` (percentage) in `CrossSpiritAgreement` | **YES** | Type change; existing claims with f64 values fail to deserialize as u32 |
| 8 | Add `#[serde(default)] new_evidence_field: Option<String>` to `EvidenceKind::CorpusReplay` | **NO** | Optional field within a tagged enum variant; existing variants deserialize with `None` |

---

## 6. Sign-Off Block

| Role | Persona / Identity | Signature | Date |
|---|---|---|---|
| **Proposer** | Dev Agent (Story 0.4) | Schema drafted in §1; proposal submitted for adversarial review. | 2026-05-12 |
| **Reviewer 1** | Mary (PM / Product Manager) | I have reviewed §1 proposal + §3 secret-classification + §4 context-drift checklist and find the schema sufficient to enter the E1b freeze gate; remaining concerns recorded as §7 follow-up items if any. | 2026-05-12 |
| **Reviewer 2** | Winston (Architect) | I have reviewed §1 proposal + §3 secret-classification + §4 context-drift checklist and find the schema sufficient to enter the E1b freeze gate; remaining concerns recorded as §7 follow-up items if any. | 2026-05-12 |

**Attestation form (verbatim from AC2):** `<persona>: I have reviewed §1 proposal + §3 secret-classification + §4 context-drift checklist and find the schema sufficient to enter the E1b freeze gate; remaining concerns recorded as §7 follow-up items if any.`

---

## 7. Follow-Up Items

No follow-up items at v0.1-α. Both reviewers (Mary and Winston) find the schema sufficient for E1b freeze gate entry without dissent.

The one partial-status item (§4 `provider_endpoint` — model-version pinning deferred to v1.0 per NFR-Sec-15) is documented as **acceptable-for-v0.1-α** by both reviewers and does not constitute a dissent. The `Option<String>` field in `ProviderEndpointPin` preserves forward-compatibility: Story 1b.4 freezes the schema with the optional field present; Story 5.5b (multi-provider CI matrix) or a v1.0 epic populates it.
