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
| `MANIFEST_SCHEMA_VERSION` (현재) | `3` |
| `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `1` |
| `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `3` |

지원 윈도우는 `1..=3`입니다. `MIN_SUPPORTED` 미만의 manifest는 `SecurityError::EAbiTooOld`로, `MAX_SUPPORTED` 초과는 `SecurityError::EAbiTooNew`로 거부됩니다.

## 마이그레이션 경로

| 시작 | 도착 | 가이드 | v3에서의 kernel 동작 |
|---|---|---|---|
| v1 | v2 | [v1 → v2](./v1-to-v2) | ✅ 로드(N-1 윈도우 내) |
| v2 | v3 | [v2 → v3](./v2-to-v3) | ✅ 현재 버전 |
| v1 | v3 | [v1 → v2](./v1-to-v2) 후 [v2 → v3](./v2-to-v3) 적용 | ✅ 로드(지원 윈도우 내) |

## 호환성 정책

MAOS는 **N-1 지원 / N-2 거부** 정책을 따릅니다:

- **N** (현재): 전체 지원, `deny_unknown_fields`로 엄격한 로드.
- **N-1**: 생략된 섹션에 대해 WARN 수준 저하 노트와 함께 지원.
- **N-2 이하**: 어드미션 시점에 거부(`SecurityError::EAbiTooOld`).

kernel이 스키마 버전 4로 올라가면 버전 1 manifest가 N-2 경계에 도달하여 거부됩니다. 미리 마이그레이션하세요.

전체 안정성 정책은 [ABI Stability](./abi-stability)를 참조하세요.

## 변경 원장(Change Ledger)

모든 브레이킹 체인지는 저장소 루트의 [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md)에 기록됩니다. CI는 모든 브레이킹 체인지 항목이 어떻게 적응해야 하는지 설명하는 `**Migration:**` 라인을 포함하도록 시행합니다.

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
