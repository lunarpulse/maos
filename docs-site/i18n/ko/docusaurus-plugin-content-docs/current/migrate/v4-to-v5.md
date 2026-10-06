---
title: "마이그레이션 v4 → v5"
sidebar_position: 5
description: Spirit manifest를 스키마 v4에서 v5로 마이그레이션합니다.
review_status: machine
---

# Manifest 마이그레이션 v4 → v5

스키마 v5는 `wasm-component` form과 `artifact` 선언을 도입합니다.

## 단계

v4 `rust-inproc` 또는 `subprocess` Spirit는 지원 호환성 window 안에서 계속 유효하며 schema v4를 유지할 수 있습니다. component 작성자만 다음 변경이 필요합니다.

1. `[class].manifest_schema_version`을 `4`에서 `5`로 변경합니다.
2. `forms = ["wasm-component"]`로 설정합니다. 다른 form과 결합할 수 없습니다.
3. manifest 기준 상대 경로이며 `.wasm`으로 끝나고 `..` segment가 없는 `artifact` 경로를 추가합니다.
4. `[sandbox] tier = "T2"`를 선언합니다.

```toml
[class]
manifest_schema_version = 5
forms = ["wasm-component"]
artifact = "dist/spirit.wasm"

[sandbox]
tier = "T2"
```

## 호환성 및 롤백

schema v5 kernel은 v4 manifest를 계속 로드합니다. component 선언을 in-process Spirit로 되돌리려면 `artifact`를 제거하고 `forms = ["rust-inproc"]`로 설정하며 sandbox tier를 `T0`로 설정한 뒤, v4 kernel을 대상으로 할 때 schema version을 `4`로 설정합니다.

기본 MAOS binary는 WASM-component Spirit를 검증하지만 Export Hold 2 동안 `wasm-host` engine이 비활성화되어 `wasm_engine_off`를 보고합니다. `wasm-host` build는 `maos run`을 통해 격리된 T2 runner에서 이를 실행하며, 지원하지 않는 in-process 기능(operator door, topology, hot-swap 후속 Spirit)은 `spawned_surface_unsupported`로 거부합니다.

## 비준

v4→v5 schema 변경은 `xtask/abi-ratifications.toml`의 `17-3b-AC3-manifest-schema-5`로 비준되었으며 ABI stability ledger에 기록됩니다.
