---
title: Manifest 스키마 참조
sidebar_position: 0
description: MAOS Spirit manifest 스키마 개요와 버전별 참조 링크.
review_status: machine
---

# Manifest 스키마 참조

모든 Spirit는 자신의 신원, 리소스 예산, 역량(capability), 운영 표면을 기술하는 `manifest.toml`을 배포합니다. kernel은 어드미션(admission) 시점에 이 manifest를 `maos-spirit-abi`에 정의된 현재 `MANIFEST_SCHEMA_VERSION` 상수와 대조하여 검증합니다.

## 스키마 버전

| 버전 | 상태 | 설명 |
|---------|--------|-------------|
| [v8](./v8) | **최신 비준 버전** | 명시적 텔레메트리 구독(Story 21-9). |
| [v7](./v7) | 지원(N-1) | FR8 manifest 선언(Story 20-8). |
| [v6](./v6) | 지원 | Native `skill.author.self` 선언(Story 18-7). |
| [v5](./v5) | 지원 | `wasm-component`와 `[class].artifact` 추가(Story 17-3b). |
| [v4](./v4) | 지원 | `[capabilities.required.loom]` 추가(Story 13.5d). |
| [v3](./v3) | 지원 | `[model_provenance]` 추가(Story 9.4b). |
| [v2](./v2) | 지원 | `[cli_wrapper]`, `[[schedule]]`, `[[gateway]]` 추가(Epic 6). |
| [v1](./v1) | 지원 | 기준 스키마(Epic 1b). |

## 버전 정책

스키마 8 kernel은 설정된 지원 범위 안에서 스키마 버전을 검사합니다.

- **지원 범위:** `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`, 현재 `1..=8`.
- **`MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` 미만:** 어드미션 시점에 `EAbiTooOld`로 거부됩니다.
- **`MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` 초과:** 어드미션 시점에 `EAbiTooNew`로 거부됩니다.

범위 내에 있어도 모든 선언이 유효한 것은 아닙니다. 구독하지 않는 스키마 1–7 manifest는 계속 지원합니다.
스키마 8 미만에서 `on_telemetry_event`를 나열하거나 `[lifecycle.telemetry]`를 선언하면
경고만 내고 로드하지 않고 거부합니다. `ABI_VERSION`과 manifest 하한은 `1`로 유지됩니다.

권위 있는 버전 상수는 `crates/maos-spirit-abi/src/lib.rs`에 있습니다:

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 8;
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = MANIFEST_SCHEMA_VERSION;
```

향후 kernel은 두 경계 중 하나를 변경할 수 있습니다. 대상 kernel의 지원 범위에 manifest 버전이 더 이상 포함되지 않기 전에 마이그레이션하세요.

## 최신

최신 비준 스키마는 **[v8](./v8)**입니다. 적용하기 전에 대상 kernel 버전을 확인하세요.
