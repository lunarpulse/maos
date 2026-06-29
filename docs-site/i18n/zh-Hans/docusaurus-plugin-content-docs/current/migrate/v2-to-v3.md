---
title: "마이그레이션 v2 → v3"
sidebar_position: 3
description: Spirit manifest를 스키마 버전 2에서 버전 3으로 마이그레이션하는 단계별 가이드.
review_status: machine
---

# Manifest 마이그레이션 v2 → v3

Manifest 스키마 버전 3은 `[model_provenance]` 섹션을 추가하기 위해 Story 9.4b AC-6(2026-06-15)에서 도입되었습니다.

## 변경된 내용

버전 3은 단일 새 섹션을 추가합니다:

| 추가 | Story | Manifest 섹션 | 목적 |
|---|---|---|---|
| 모델 출처 | 9.4b | `[model_provenance]` | `covered_model_id`, `training_data_lineage`(역방향-DNS 제약), `last_eval_timestamp`로 모델 신원과 훈련 데이터 출처 추적 |

이 섹션은 읽기 시 **선택 사항**입니다 — `from_manifest_toml`은 없으면 `None`을 반환합니다. 즉, v2 manifest는 수정 없이 v3 kernel에서 로드됩니다(N-1 지원 하한선).

## kernel 동작

| kernel 버전 | v2 manifest 동작 |
|---|---|
| 스키마 v3 kernel | ✅ WARN 수준 저하 노트와 함께 로드(N-1 지원) |
| 향후 스키마 v4 kernel | ✅ 윈도우 내 유지 예상(N-1) |
| 향후 스키마 v5 kernel | ⛔ N-2 경계에서 거부 예상 |

## 마이그레이션 단계

### 1단계: 스키마 버전 올리기

manifest의 `[class]` 섹션에서 스키마 버전을 갱신합니다:

```toml
[class]
name = "my-spirit"
manifest_schema_version = 3   # was 2
min_substrate_version = "0.1.0-alpha"
```

### 2단계: (선택) 모델 출처 추가

Spirit이 ML 모델을 감싸거나 호출하면 그 출처를 선언합니다:

```toml
[model_provenance]
covered_model_id = "com.example.my-model-v2"
training_data_lineage = "com.example.dataset.curated-2026q2"
last_eval_timestamp = "2026-06-01T00:00:00Z"
```

**제약:**
- `covered_model_id` — 이 Spirit이 감싸는 모델 식별; 역방향-DNS 포맷 권장.
- `training_data_lineage` — 역방향-DNS 제약 식별자(자유 텍스트 아님). 검증 시 시행됩니다.
- `last_eval_timestamp` — 마지막 평가 실행의 ISO 8601 타임스탬프.

### 3단계: 검증

manifest를 v3 kernel에 대해 로드합니다. kernel은 `deny_unknown_fields`로 검증하므로 스키마 에러가 어드미션 시점에 드러납니다.

## 롤백

`[model_provenance]` 섹션은 선택 사항입니다. 되돌리려면:
1. `[model_provenance]` 섹션을 제거합니다.
2. `[class]`에서 `manifest_schema_version = 2`로 설정합니다.

manifest는 지원 윈도우가 버전 2를 포함하는 어떤 kernel에서도 로드됩니다.

## 비준(Ratification)

버전 3 범프는 Story 7.5a의 ABI Stability Triple 프로세스를 따라 `xtask/abi-ratifications.toml`에 비준 항목으로 기록됩니다.

## 참조

- [ABI 안정성 정책](./abi-stability) — N-1/N-2 규칙과 ABI Stability Triple
- [ABI 상수](/abi/v1/constants) — `MANIFEST_SCHEMA_VERSION`과 지원 윈도우의 실시간 값
- [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) — CI가 시행하는 변경 원장

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
