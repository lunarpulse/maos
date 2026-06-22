---
title: ABI 안정성 정책
sidebar_position: 4
description: ABI Stability Triple, N-1/N-2 호환성 정책, 그리고 STABILITY.md와 BREAKING.md가 변경을 추적하는 방식.
review_status: machine
---

# ABI 안정성 정책

MAOS는 kernel과 Spirit 사이에 기계적으로 검증되는 호환성 계약을 시행합니다. 이 페이지는 ABI Stability Triple, 호환성 윈도우, 변경 추적 방식을 설명합니다.

## ABI Stability Triple

모든 MAOS kernel은 함께 **ABI Stability Triple**을 구성하는 세 가지 버전 번호를 게시합니다:

```
(kernel_version, abi_version, manifest_schema_version)
```

| 항목 | 현재 값 | 추적 대상 |
|---|---|---|
| `kernel_version` | `0.1.0-alpha` | kernel 바이너리의 의미 버전(`Cargo.toml` 기준) |
| `abi_version` | `1` | Spirit ABI의 와이어 포맷 버전(`ComplianceClaim` 봉투, vtable 레이아웃) |
| `manifest_schema_version` | `3` | Spirit manifest TOML 포맷의 스키마 버전 |

이 상수들은 `maos-spirit-abi/src/lib.rs`에 있으며 어드미션 검사와 CI 게이트가 소비하는 단일 진실 원천입니다.

## 호환성 윈도우

kernel은 Spirit 어드미션 시점에 양 방향으로 **fail-closed** 호환성 윈도우를 시행합니다:

| Manifest 스키마 버전 | kernel 동작 |
|---|---|
| `= MANIFEST_SCHEMA_VERSION` (현재) | ✅ 엄격한 로드(`deny_unknown_fields`) |
| `= N-1` (현재보다 하나 아래) | ✅ 지원 — 생략된 섹션에 대해 WARN 수준 저하 노트와 함께 로드 |
| `< MIN_SUPPORTED` (N-2 이하) | ⛔ 거부 — `SecurityError::EAbiTooOld` |
| `> MAX_SUPPORTED` (미래) | ⛔ 거부 — `SecurityError::EAbiTooNew` |
| `min_substrate_version > kernel_version` | ⛔ 거부 — `SecurityError::ESubstrateTooOld` (FR8) |

조용히 경고하고 무시하는 윈도우는 없습니다. manifest는 보안 아티팩트입니다; fail-open 어드미션은 보안 결함입니다.

## N-1 / N-2 정책

- **N-1 지원**: 현재 kernel보다 한 버전 뒤진 manifest도 여전히 로드됩니다. kernel은 구형 manifest가 생략하는 섹션을 식별하는 WARN 수준 노트를 내보냅니다. Spirit은 작동하지만 새 manifest 기능의 혜택을 받지 못합니다.
- **N-2 거부**: 두 버전 이상 뒤진 manifest는 어드미션 시점에 `SecurityError::EAbiTooOld`로 거부됩니다. 에러 메시지는 선언된 버전과 지원 윈도우를 명시합니다.

이는 Spirit 작성자에게 스키마 범프 이후 마이그레이션할 **한 번의 전체 버전 사이클**을 줍니다.

## ABI 버전 vs. Manifest 스키마 버전

이들은 독립적인 버전 번호입니다:

- **`ABI_VERSION`**은 Spirit ABI의 와이어 포맷을 추적합니다 — `ComplianceClaim` 스키마, vtable 레이아웃, 훅 시그니처. (§8.5 규칙에 따라) 브레이킹 와이어 포맷 변경 시에만 드물게 올라갑니다.
- **`MANIFEST_SCHEMA_VERSION`**은 Spirit manifest TOML 스키마를 추적합니다. 새 manifest 섹션이 추가되면(추가적인 경우라도 추적 목적으로) 올라갑니다.

선택적 manifest 섹션을 추가하면 `MANIFEST_SCHEMA_VERSION`은 올라가지만 `ABI_VERSION`은 올라가지 않습니다. 컴플라이언스 클레임 스키마나 훅 시그니처의 브레이킹 체인지는 `ABI_VERSION`을 올립니다.

## ABI 브레이크의 기준

§8.5에 따라 다음 변경은 `ABI_VERSION`을 **올립니다**:

| 변경 | ABI 브레이크? |
|---|---|
| `#[serde(default)]` 없이 필수 필드 추가 | **YES** |
| 필드 이름 변경 | **YES** |
| 필드 제거 | **YES** |
| 필드 타입 변경 | **YES** |
| `#[repr(u8)]` 판별자 업데이트 없이 enum 배리언트 재정렬 | **YES** |
| enum 배리언트 제거 | **YES** |

다음 변경은 `ABI_VERSION`을 올리지 **않습니다**:

| 변경 | ABI 브레이크? |
|---|---|
| `#[serde(default)]`로 선택적 필드 추가 | No |
| 명시적 판별자와 `#[serde(other)]` 폴백으로 enum 배리언트를 끝에 추가 | No |

## 변경 추적

### `STABILITY.md`

[`STABILITY.md`](https://github.com/maos/maos/blob/main/STABILITY.md) 파일은 워크스페이스 상태에서 **생성**됩니다:

```bash
cargo run -p xtask -- stability-matrix
```

이 파일은 다음을 게시합니다:
- 실시간 ABI Stability Triple 값
- 지원 스키마 윈도우
- 지원 중단(deprecation) 테이블(NFR-Maint-5: 2 마이너 릴리스 경고 후 1 메이저 릴리스 제거)
- LTS 정책(v1.0부터 1년간 보안 전용 패치)
- 서브스트레이트 자기 컴플라이언스 범위

CI는 파일이 워크스페이스 상태와 일치하는지 검증하기 위해 `stability-matrix --check`를 실행합니다.

### `BREAKING.md`

[`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) 파일은 **사람이 작성한** 변경 원장입니다. 모든 브레이킹 체인지는 다음을 동반해야 합니다:

- 날짜가 있는 `## YYYY-MM-DD — <title>` 제목
- 변경을 설명하는 산문
- 구체적인 적응 단계가 있는 `**Migration:**` 라인

`check-breaking-md` CI 게이트는 항목이 마이그레이션 경로를 누락하면 실패합니다.

### `xtask/abi-ratifications.toml`

각 manifest 스키마 버전 범프는 `[[ratification]]` 항목으로 기록되어, 버전이 올라간 시기와 이유의 감사 추적을 제공합니다.

## 지원 중단 타임라인

지원 중단된 공개 표면은 NFR-Maint-5를 따릅니다:

1. `#[maos_attrs::deprecated_since(...)]` 경고와 함께 **2 마이너 릴리스**
2. 제거까지 **1 메이저 릴리스**

지원 중단 경고는 `Ctx::deprecation_warnings()`로 런타임에, 그리고 `spirit-test` SDK로 테스트에서 관찰할 수 있습니다. `stability-matrix --check` 게이트는 모든 지원 중단 표면이 일치하는 `STABILITY.md` 항목을 갖도록 시행합니다.

## 참조

- [ABI 상수](/abi/v1/constants) — 코드 내 실시간 값
- [v1 → v2 마이그레이션](./v1-to-v2)
- [v2 → v3 마이그레이션](./v2-to-v3)

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
