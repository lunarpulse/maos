---
title: 마이그레이션 가이드
sidebar_position: 1
description: MAOS manifest 스키마 버전의 브레이킹 체인지와 마이그레이션 경로 개요.
review_status: machine
---

# 마이그레이션 가이드

MAOS는 Spirit manifest 포맷의 브레이킹 체인지를 추적하기 위해 **manifest 스키마 버전**을 사용합니다. kernel은 어드미션 시점에 호환성 윈도우를 시행합니다 — 윈도우 밖의 Spirit은 타입화된 에러로 거부됩니다.

## 현재 상태

| 상수 | 값 |
|---|---|
| `MANIFEST_SCHEMA_VERSION` (현재) | `5` |
| `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `1` |
| `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `5` |

지원 윈도우는 `1..=5`입니다. `MIN_SUPPORTED` 미만의 manifest는 `SecurityError::EAbiTooOld`로, `MAX_SUPPORTED` 초과는 `SecurityError::EAbiTooNew`로 거부됩니다.

## 마이그레이션 경로

| 시작 | 도착 | 가이드 | v5에서의 kernel 동작 |
|---|---|---|---|
| v1 | v2 | [v1 → v2](./v1-to-v2) | ✅ 로드(지원 윈도우 내) |
| v2 | v3 | [v2 → v3](./v2-to-v3) | ✅ 로드(지원 윈도우 내) |
| v3 | v4 | [v3 → v4](./v3-to-v4) | ✅ 로드(지원 윈도우 내) |
| v4 | v5 | [v4 → v5](./v4-to-v5) | ✅ 현재 버전 |

## 호환성 정책

MAOS는 manifest 스키마 경계로 구성된 포괄 범위를 어드미션합니다:

- `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`은 현재 `1..=5`입니다.
- 최솟값 미만 manifest는 어드미션 시점에 `SecurityError::EAbiTooOld`로 거부됩니다.
- 최댓값 초과 manifest는 어드미션 시점에 `SecurityError::EAbiTooNew`로 거부됩니다.

향후 kernel은 두 경계 중 하나를 변경할 수 있습니다. 대상 kernel의 지원 범위에 manifest 버전이 더 이상 포함되지 않기 전에 마이그레이션하세요.

전체 안정성 정책은 [ABI Stability](./abi-stability)를 참조하세요.

## 변경 원장(Change Ledger)

모든 브레이킹 체인지는 저장소 루트의 [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md)에 기록됩니다. CI는 모든 브레이킹 체인지 항목이 어떻게 적응해야 하는지 설명하는 `**Migration:**` 라인을 포함하도록 시행합니다.

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
