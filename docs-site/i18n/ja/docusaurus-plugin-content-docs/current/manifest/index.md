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
| [v3](./v3) | **현재(Current)** | `[model_provenance]` 추가(Story 9.4b). |
| [v2](./v2) | 지원(N-1) | `[[cli_wrapper]]`, `[[schedule]]`, `[[gateway]]` 추가(Epic 6). |
| [v1](./v1) | 거부(N-2) | 기준 스키마(Epic 1b). |

## 버전 정책

kernel은 **N-1 지원 / N-2 거부** 정책을 시행합니다.

- **현재(N):** 전체 기능 세트, 기능 저하 없음.
- **N-1:** 문서화된 저하와 함께 허용 — 새 섹션은 `#[serde(default)]`로 기본값 처리됩니다. kernel은 기본값 처리된 각 섹션에 대해 `WARN` 수준 알림을 내보냅니다.
- **N-2 이하:** 어드미션 시점에 `EAbiTooOld`로 거부됩니다.

권위 있는 버전 상수는 `crates/maos-spirit-abi/src/lib.rs`에 있습니다:

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 3;
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
```

## 최신

최신 스키마 참조는 **[v3](./v3)** 입니다.
