---
review_status: machine
---

<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `constants` Module {#abi-constants-module}

## Related {#abi-constants-related}

- [ABI Stability Policy](/migrate/abi-stability) — 전체 호환성 윈도우 문서
- [v1 → v2 Migration](/migrate/v1-to-v2) — manifest 스키마 버전 2에서 변경된 점
- [v2 → v3 Migration](/migrate/v2-to-v3) — manifest 스키마 버전 3에서 변경된 점


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*


## Constants {#maos-spirit-abi-constants}

### `ABI_VERSION` {#maos-spirit-abi-abi-version}

MAOS Spirit ABI의 ABI 버전 상수.

ABI Stability Triple 규칙(§8.5)에 따라 올라갑니다.

**Story 1b.4가 ComplianceClaim 봉투 동결 시 `1`에 고정했습니다.**

# Example {#maos-spirit-abi-abi-version-example}

```rust
use maos_spirit_abi::ABI_VERSION;

assert_eq!(ABI_VERSION, 1);
```


```rust
pub const ABI_VERSION: u32 = 1u32;
```

### `MANIFEST_SCHEMA_VERSION` {#maos-spirit-abi-manifest-schema-version}

kernel이 현재 내보내는 manifest 스키마 버전.

Epic 6 §A4(소급 2026-05-28)에서 Epic 6 스토리 6.2 / 6.4 / 6.5에 걸쳐 들어온 네 가지 추가 섹션을 추적하기 위해 `2`로 올려졌습니다:

- `[[cli_wrapper]]` (Story 6.2 — `command`, `output_shape_version`,
  `recovery_policy`, `posture`, `shutdown_signal`).
- `[[schedules]]` (Story 6.4 — `id`, `cadence`, `rate_limit_per_hour`,
  `compliance_claim_ref_hex`, `side_effect_scopes`, `payload_b64`).
- `[gateways]` / `[[gateway]]` (Story 6.5 — `id`, `type`, `auth_secret_ref`,
  `inbound_routing`, 게이트웨이별 설정 블록).
- `ConsentEnvelope.intent_class` + `ConsentEnvelope.valid_until_ns`
  (Story 6.4 — 동의 봉투 형태에 추가).

네 가지 추가는 모두 TOML/serde 계층에서 와이어 호환입니다
(`#[serde(default)]` + `#[serde(deny_unknown_fields)]`), 따라서
`MANIFEST_SCHEMA_VERSION = 2`의 kernel은 `= 1` 기준으로 작성된 manifest를 허용합니다
(`MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`이 시행하는 N-1 지원 하한선).

추가적 `[model_provenance]` 섹션을 추적하기 위해 Story 9.4b AC-6(2026-06-15)에서 `3`으로 올려졌습니다(`covered_model_id`, `training_data_lineage`
— 역방향-DNS 제약, 자유 텍스트 아님 — `last_eval_timestamp`). 이
섹션은 TOML/serde 계층에서 와이어 호환입니다: 읽기 시 선택 사항입니다
(`from_manifest_toml`은 없으면 `None`을 반환), 따라서 `MANIFEST_SCHEMA_VERSION = 3`의 kernel은 여전히 `= 2` 기준으로 작성된 manifest를 허용합니다
(N-1 지원 하한선) — AC-11 append-only 호환. `xtask/abi-ratifications.toml`에 하나의 비준된 `[[ratification]]` 항목으로 기록됩니다.

이 상수는 `maos-manifest::ClassSection` 검증과 `xtask
check-manifest-schema-version` 게이트가 소비하는 단일 권위 원천입니다. Story 7.5a의 ABI Stability Triple
`(kernel_version, abi_version, manifest_schema_version)`이 이 상수를 직접 소비합니다.

# Example {#maos-spirit-abi-manifest-schema-version-example}

```rust
use maos_spirit_abi::MANIFEST_SCHEMA_VERSION;

assert_eq!(MANIFEST_SCHEMA_VERSION, 4);
```


```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 4u32;
```

### `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` {#maos-spirit-abi-min-supported-manifest-schema-version}

이 kernel이 어드미션 시점에 수락하는 가장 낮은 manifest 스키마 버전.

Story 7.5a가 N-1 지원 / N-2 거부 정책에 따라 각 ABI 범프 시 이 하한선을 올립니다. v0.5-α에서 하한선은 `1`로 유지됩니다 — Epic 1b 기준 manifest가 변경 없이 로드됩니다.

# Example {#maos-spirit-abi-min-supported-manifest-schema-version-example}

```rust
use maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;

fn check_manifest_version(declared: u32) -> bool {
    declared >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
}

assert!(check_manifest_version(1));
assert!(check_manifest_version(3));
assert!(!check_manifest_version(0));
```


```rust
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1u32;
```

### `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` {#maos-spirit-abi-max-supported-manifest-schema-version}

이 kernel이 내보내거나 수락하는 가장 높은 manifest 스키마 버전.

현재 `MANIFEST_SCHEMA_VERSION`과 같습니다. Story 7.5a가 순방향 호환 실험을 위한 명시적 N+1 수락 윈도우를 도입할 때까지 두 상수는 동의어로 유지됩니다.

# Example {#maos-spirit-abi-max-supported-manifest-schema-version-example}

```rust
use maos_spirit_abi::{
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
    MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
};

fn is_version_supported(v: u32) -> bool {
    v >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
        && v <= MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION
}

assert!(is_version_supported(1));  // N-1 — supported
assert!(is_version_supported(2));  // N-1 — supported
assert!(is_version_supported(3));  // Current — supported
assert!(is_version_supported(4)); // 현재 — 지원됨
assert!(!is_version_supported(0)); // Below floor — EAbiTooOld
```


```rust
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 4u32;
```
