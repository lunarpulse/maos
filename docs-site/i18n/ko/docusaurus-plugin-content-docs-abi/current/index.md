---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `index` Module {#abi-index-module}

## Related {#abi-index-related}

- [ABI Stability Policy](/migrate/abi-stability) — 호환성 윈도우와 범프 규칙
- [Constants](./constants) — `ABI_VERSION`과 `MANIFEST_SCHEMA_VERSION` 참조


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*

`maos-spirit-abi` — 와이어 안정 타입 전용(`#![no_std]`).

`ABI_VERSION`는 ComplianceClaim 스키마가 공동 Mary+Winston 적대적 리뷰 하에 동결될 때(Story 1b.4, `_bmad-output/planning-artifacts/compliance-claim-schema-review.md` 참조) `1`로 올려졌습니다.

여기서 `ABI_VERSION`를 올리는 것이 §8.5에 따른 **ABI-범프 트리거**입니다. abi-diff 게이트(`--deny removed --deny changed`)는 1b.4 동결 후 `abi-baseline/v1-pre-bump.txt`에 베이스라인됩니다.

## Story 2.1 additive surface (does NOT bump `ABI_VERSION`) {#maos-spirit-abi-story2-1additivesurface-doesnotbump-abi-version}

Story 2.1이 추가합니다:
- `pub mod cancellation` — `CancellationSignal` 트레이트 + `NeverCancel`
- `pub mod lifecycle` — `Spirit` 트레이트(11 훅) + `SpiritVtable<T>` + 페이로드 타입
- `pub mod ctx` — `Ctx` Spirit 작성자용 컨텍스트 타입

모든 추가는 §8.5 행 7+8에 따라 ABI-추가적입니다. `ABI_VERSION`는 `1`로 유지됩니다.

# Version history {#maos-spirit-abi-versionhistory}

| 모듈 / 상수 | 도입 | 비고 |
|---|---|---|
| `ABI_VERSION` | Story 1b.4 | ComplianceClaim 봉투 동결 시 `1`로 고정 |
| `cancellation`, `lifecycle`, `ctx` | Story 2.1 | 추가적 ABI 표면, 버전 범프 없음 |
| `identity` | Story 1b.1 | v0.1-β 이후 와이어 안정 |
| `compliance` | Story 1b.4 | 동결된 스키마, ABI_VERSION 범프 트리거 |
| `gateway` | Story 6.5 | ADR-029 binding-v1.0 |
| `deprecation` | Story 7.1 | empty-present 지원 중단 채널 |
| `MANIFEST_SCHEMA_VERSION = 3` | Story 9.4b | `[model_provenance]` 섹션 |

## Modules {#maos-spirit-abi-modules}
| 모듈 | 설명 | 도입 |
|---|---|---|
| [`cancellation`] | `CancellationSignal` 트레이트 + `NeverCancel` — 런타임 무관 취소 | Story 2.1 |
| [`compliance`] | `ComplianceClaim` 봉투 — Ed25519 서명 증명 스키마 | Story 1b.4 (동결) |
| [`ctx`] | `Ctx` 타입 — 훅 호출을 위한 Spirit 작성자용 컨텍스트 | Story 2.1 |
| [`deprecation`] | `DeprecationWarning` — ABI 표면 진화를 위한 지원 중단 채널 | Story 7.1 |
| [`gateway`] | `GatewaySubmodule` 트레이트 + `GatewayCtx` — 외부 메시징 게이트웨이 계약 | Story 6.5 (ADR-029) |
| [`identity`] | `SpiritId`, `HostId`, `FrameKind` — 와이어 안정 신원과 프레임 판별 | Story 1b.1 |
| [`lifecycle`] | `Spirit` 트레이트 + `SpiritVtable` + 페이로드 타입 | Story 2.1 |
