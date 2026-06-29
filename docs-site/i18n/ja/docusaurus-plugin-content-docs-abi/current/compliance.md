---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `compliance` Module {#abi-compliance-module}

## Related {#abi-compliance-related}

- [ABI Stability Policy](/migrate/abi-stability) — `ABI_VERSION=1` 동결 맥락
- [ADR-009](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-009-trust-tier-taxonomy.md) — `TrustTier` 근거
- [ADR-004](https://github.com/lunarpulse/maos/blob/main/docs/adr/ADR-004-sandbox-tier-taxonomy.md) — `SandboxTier` 근거


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

Binding-v0.1 ComplianceClaim 스키마 타입 — **동결(FROZEN)**.

이 타입들은 `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`의 공동 Mary+Winston 적대적 리뷰 하에 확정되었습니다. Story 1b.4가 스키마를 동결하고, serde derive를 추가하고, `Uuid` newtype 결정을 해결하고, `ABI_VERSION`를 `1`로 올렸습니다. 이 동결이 Epic 1b의 **유일한 공인된 ABI 브레이크**입니다.

# §8.5 ABI-break rule (review report §5 self-test) {#compliance-8-5abi-breakrule-reviewreport5self-test}

다음 변경은 `ABI_VERSION`를 **올립니다**(이제 `v1-pre-bump.txt`에 베이스라인된 `abi-diff` 게이트를 통한 기계적 시행):

| # | 변경 | ABI 브레이크? |
|---|---|---|
| 1 | `#[serde(default)]` 없이 필수 필드 추가 | **YES** |
| 2 | 필드 이름 변경 | **YES** |
| 3 | 필드 제거 | **YES** |
| 4 | 필드 타입 변경 | **YES** |
| 5 | 명시적 `#[repr(u8)]` 판별자 업데이트 없이 `Verdict` / `PrincipleRef` / `EvidenceKind` 배리언트 재정렬 | **YES** |
| 6 | `Verdict` / `PrincipleRef` / `EvidenceKind`에서 enum 배리언트 제거 | **YES** |

다음 변경은 `ABI_VERSION`를 올리지 **않습니다**:

| # | 변경 | ABI 브레이크? |
|---|---|---|
| 7 | `#[serde(default, skip_serializing_if = "Option::is_none")]`로 선택적 필드 추가 | **NO** |
| 8 | 명시적 `#[repr(u8)]` 판별자와 enum의 `#[serde(other)]` 폴백으로 enum 배리언트를 끝에 추가 | **NO** |

abi-diff 게이트(`--deny removed --deny changed`, `v1-pre-bump.txt`에 베이스라인)가 이 규칙의 기계적 절반입니다; 이 doc-block이 정준 사람이 읽을 수 있는 참조입니다.


## Enums {#compliance-enums}

### `SigningAlg` {#maos-spirit-abi-compliance-signingalg}

서명 알고리즘 식별자.

추가적 enum: 명시적 `#[repr(u8)]` 판별자로 끝에 배리언트를 추가하는 것은 ABI 브레이크가 아닙니다. 배리언트 제거나 재정렬은 §8.5에 따라 ABI 브레이크입니다.

# Example {#maos-spirit-abi-compliance-signingalg-example}

```rust
use maos_spirit_abi::compliance::SigningAlg;

let alg = SigningAlg::Ed25519;
```


```rust
pub enum SigningAlg {
    Ed25519,
}
```

### `TrustTier` {#maos-spirit-abi-compliance-trusttier}

trust tier — 운영자 가시 메타데이터 분류.

# Example {#maos-spirit-abi-compliance-trusttier-example}

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

### `SandboxTier` {#maos-spirit-abi-compliance-sandboxtier}

sandbox tier — OS 네이티브 샌드박스 프리미티브 분류.

# Example {#maos-spirit-abi-compliance-sandboxtier-example}

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

### `PrincipleRef` {#maos-spirit-abi-compliance-principleref}

클레임이 컴플라이언스를 증명하는 원칙.

명시적 판별자를 가진 추가적 enum. `#[serde(other)]` 폴백으로 끝에 배리언트를 추가하는 것은 ABI 브레이크가 아닙니다; 제거나 재정렬은 ABI 브레이크입니다.

# Example {#maos-spirit-abi-compliance-principleref-example}

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

### `EvidenceKind` {#maos-spirit-abi-compliance-evidencekind}

컴플라이언스 클레임을 뒷받침하는 증거.

# Example {#maos-spirit-abi-compliance-evidencekind-example}

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

### `Verdict` {#maos-spirit-abi-compliance-verdict}

증명 당사자가 내린 판정.

모든 배리언트에 명시적 판별자 — 판별자 업데이트 없이 재정렬하면 §8.5 셀프테스트 행 #5에 따라 ABI 브레이크입니다.

# Example {#maos-spirit-abi-compliance-verdict-example}

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

## 한국 규제 참고사항 {#abi-compliance-korean-regulations}

<!-- TODO: 한국 규제/운영 검토는 v1.0 이후 네이티브 리뷰어와 함께 보강합니다. -->

이 ABI 참조는 `ComplianceClaim` 데이터 구조와 호환성 규칙을 설명합니다. 한국 배포 시 적용되는 법적·규제 해석은 조직의 법무/컴플라이언스 검토를 따르세요.
