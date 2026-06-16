<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `compliance` Module

## Related

- [ABI Stability Policy](/migrate/abi-stability) — `ABI_VERSION=1` freeze context
- [ADR-009](/adr/ADR-009-trust-tier-taxonomy) — `TrustTier` rationale
- [ADR-004](/adr/ADR-004-sandbox-tier-taxonomy) — `SandboxTier` rationale


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

Binding-v0.1 ComplianceClaim schema types — **FROZEN**.

These types are committed under the joint Mary+Winston adversarial
review at `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`.
Story 1b.4 froze the schema, added serde derives, resolved the `Uuid`
newtype decision, and bumped `ABI_VERSION` to `1`. This freeze is the
**one sanctioned ABI break** in Epic 1b.

# §8.5 ABI-break rule (review report §5 self-test)

The following changes **DO** bump `ABI_VERSION` (mechanical enforcement
via the `abi-diff` gate, now baselined at `v1-pre-bump.txt`):

| # | Change | ABI Break? |
|---|---|---|
| 1 | Add required field without `#[serde(default)]` | **YES** |
| 2 | Rename any field | **YES** |
| 3 | Remove any field | **YES** |
| 4 | Change any field's type | **YES** |
| 5 | Reorder `Verdict` / `PrincipleRef` / `EvidenceKind` variants without updating explicit `#[repr(u8)]` discriminants | **YES** |
| 6 | Remove an enum variant from `Verdict` / `PrincipleRef` / `EvidenceKind` | **YES** |

The following changes **DO NOT** bump `ABI_VERSION`:

| # | Change | ABI Break? |
|---|---|---|
| 7 | Add optional field with `#[serde(default, skip_serializing_if = "Option::is_none")]` | **NO** |
| 8 | Add enum variant at the end with explicit `#[repr(u8)]` discriminant and `#[serde(other)]` fallback on the enum | **NO** |

The abi-diff gate (`--deny removed --deny changed`, baselined at
`v1-pre-bump.txt`) is the mechanical half of this rule; this doc-block
is the canonical human-readable reference.

## Enums

Signing algorithm identifier.

Additive enum: adding a variant at the end with explicit `#[repr(u8)]`
discriminant is NOT an ABI break. Removing or reordering variants IS
an ABI break per §8.5.

# Example

```rust
use maos_spirit_abi::compliance::SigningAlg;

let alg = SigningAlg::Ed25519;
```

```rust
pub enum SigningAlg {
    Ed25519,
}
```

Trust tier — operator-visible metadata classification.

# Example

```rust
use maos_spirit_abi::compliance::TrustTier;

let tier = TrustTier::PublicVetted;
```

```rust
pub enum TrustTier {
    Local,
    OrgInternal,
    PublicVetted,
    PublicUntrusted,
}
```

Sandbox tier — OS-native sandbox primitive classification.

# Example

```rust
use maos_spirit_abi::compliance::SandboxTier;

let tier = SandboxTier::T2;
```

```rust
pub enum SandboxTier {
    T0,
    T1,
    T2,
    T3,
    T4,
}
```

Principles the claim attests compliance with.

Additive enum with explicit discriminants. Adding a variant at the
end with `#[serde(other)]` fallback is NOT an ABI break; removing or
reordering IS.

# Example

```rust
use maos_spirit_abi::compliance::PrincipleRef;

let principle = PrincipleRef::Soc2TypeIi;
assert_eq!(principle as u8, 1);
```

```rust
pub enum PrincipleRef {
    Hipaa164308,
    Soc2TypeIi,
    Iso27001,
    EuAiActArt14,
    UnknownPrinciple,
}
```

Evidence supporting a compliance claim.

# Example

```rust
use maos_spirit_abi::compliance::EvidenceKind;

let corpus = EvidenceKind::CorpusReplay {
    corpus_sha256: [0xAA; 32],
};

let review = EvidenceKind::ManualReview {
    reviewer_id: "eng-lead-01".into(),
};

assert!(matches!(corpus, EvidenceKind::CorpusReplay { .. }));
assert!(matches!(review, EvidenceKind::ManualReview { .. }));
```

```rust
pub enum EvidenceKind {
    CorpusReplay,
    PenTestReportRef,
    ManualReview,
    CrossSpiritAgreement,
}
```

Verdict rendered by the attesting party.

Explicit discriminants on every variant — reordering without updating
discriminants is an ABI break per §8.5 self-test row #5.

# Example

```rust
use maos_spirit_abi::compliance::Verdict;

let verdict = Verdict::AdmitWithCaveats {
    caveats: vec!["Model version not pinned".into()],
};

match verdict {
    Verdict::Admit => { /* admitted */ }
    Verdict::AdmitWithCaveats { caveats } => {
        assert_eq!(caveats.len(), 1);
    }
    _ => { /* other verdict */ }
}
```

```rust
pub enum Verdict {
    Admit,
    AdmitWithCaveats,
    RejectContextDrift,
    RejectMalformedClaim,
    RejectExpiredClaim,
    UnknownVerdict,
}
```
