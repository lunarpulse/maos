---
title: 컴플라이언스 클레임
sidebar_position: 10
description: 감사 가능성과 신뢰 검증을 위해 Spirit에 ComplianceClaimEnvelope 부착.
review_status: machine
---

# 컴플라이언스 클레임

## Problem

Spirit이 검증 가능한 컴플라이언스 증명이 필요한 환경에서 동작합니다 — 규제 감사, `local`에서 `audited`로의 trust tier 승격, 또는 SB-1047 모델 출처 추적. kernel이 어드미션 시점에 검증할 수 있는 서명된 `ComplianceClaimEnvelope`를 부착해야 합니다.

## Solution

`ComplianceClaimEnvelope`를 빌드하고 서명합니다:

```rust
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, Claim, SigningAlg,
    ExecutionContextFingerprint, TrustTier, SandboxTier,
    PrincipleRef, EvidenceKind, Verdict, Uuid, CapabilityId,
};

/// Build a compliance claim for a Spirit that attests to
/// safety-critical principles with audit evidence.
fn build_compliance_claim() -> ComplianceClaimEnvelope {
    let claim = Claim {
        claim_id: Uuid::from_bytes([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        ]),
        spirit_class: "my-audited-spirit".into(),
        spirit_version: "1.0.0".into(),
        abi_version: 1,
        attester_id: "auditor@example.com".into(),
        issued_at_epoch_secs: 1718400000, // 2024-06-15T00:00:00Z
        expires_at_epoch_secs: Some(1749936000), // 2025-06-15T00:00:00Z
        execution_context: ExecutionContextFingerprint {
            trust_tier: TrustTier::Audited,
            sandbox_tier: SandboxTier::Baseline,
            capabilities: vec![
                CapabilityId("provider.complete".into()),
            ].into_iter().collect(),
            provider_endpoints: vec![],
            crypto_providers: vec![],
        },
        principles: vec![
            PrincipleRef::I1CapabilityMediation,
            PrincipleRef::I2SandboxIsolation,
            PrincipleRef::I3TransparencyLog,
        ],
        evidence: vec![
            EvidenceKind::TestReport {
                suite_name: "admission-smoke".into(),
                pass_count: 42,
                fail_count: 0,
                report_hash_hex: "a1b2c3d4...".into(),
            },
        ],
        verdict: Verdict::Pass,
    };

    // Serialise the claim to CBOR bytes.
    let claim_bytes = serde_cbor::to_vec(&claim).expect("CBOR encode");

    // Sign the SHA-256 hash of the claim bytes with Ed25519.
    // In production, use a hardware-backed signing key.
    let signature = sign_claim_bytes(&claim_bytes);

    ComplianceClaimEnvelope {
        abi_version: 1,
        signing_alg: SigningAlg::Ed25519,
        signature_bytes: signature,
        signer_public_key: get_public_key(),
        claim_bytes,
    }
}

// Stubs — use a real Ed25519 library (e.g., ed25519-dalek).
fn sign_claim_bytes(data: &[u8]) -> Vec<u8> { vec![0u8; 64] }
fn get_public_key() -> Vec<u8> { vec![0u8; 32] }
```

스케줄된 호출이나 모델 출처를 위해 manifest에서 클레임을 참조합니다:

```toml
[class]
name = "my-audited-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "audited"
description = "A Spirit with a compliance claim."

[model_provenance]
covered_model_id = "anthropic.claude-3-opus"
training_data_lineage = ["org.example.dataset-v2"]
last_eval_timestamp = "2026-01-15T00:00:00Z"
```

## Discussion

`ComplianceClaimEnvelope`는 Spirit 클래스 + 버전을 컴플라이언스 판정 집합에 바인딩하는 Ed25519 서명 증명입니다. 스키마는 ABI 버전 1(Story 1b.4)에서 **동결**되었습니다 — 변경은 `§8.5`에 문서화된 엄격한 ABI-브레이크 규칙을 따릅니다.

**봉투 구조:**

| 필드 | 목적 |
|---|---|
| `abi_version` | 봉투 자체의 스키마 버전 |
| `signing_alg` | 사용된 알고리즘(`Ed25519`) |
| `signature_bytes` | `sha256(claim_bytes)`에 대한 Ed25519 서명 |
| `signer_public_key` | 검증용 공개 키 |
| `claim_bytes` | CBOR 인코딩 `Claim` 페이로드 |

**`Claim` 페이로드 포함:**

- `principles` — 클레임이 컴플라이언스를 증명하는 불변량(I1 ~ I14).
- `evidence` — 테스트 보고서, 감사 해시, 기타 지원 자료.
- `verdict` — `Pass`, `Fail`, `Conditional`, 또는 `Withdrawn`.
- `execution_context` — 클레임이 평가된 trust tier, sandbox tier, 역량, 프로바이더 엔드포인트의 핑거프린트.

**컴플라이언스 클레임이 필요한 시점:**

- Spirit을 `local`에서 `audited` trust tier로 승격할 때.
- 규제 요구 사항이 있는 환경에 배포할 때.
- `compliance_claim_ref_hex`를 통해 `[[schedule]]` 항목에서 클레임을 참조할 때.
- SB-1047 모델 출처 요구 사항을 충족할 때(Story 9.4b).

kernel은 `audited`-tier Spirit에 대해 어드미션 시점에 서명을 검증하고 결과를 transparency log에 기록합니다.

## 한국 규제 참고사항

<!-- TODO: 한국 규제/운영 검토는 v1.0 이후 네이티브 리뷰어와 함께 보강합니다. -->

이 페이지의 ComplianceClaim 예시는 MAOS의 감사 가능성·투명성·신뢰 계층 설명을 위한 것입니다. 한국 배포 시 적용되는 법적·규제 해석은 조직의 법무/컴플라이언스 검토를 따르세요.
