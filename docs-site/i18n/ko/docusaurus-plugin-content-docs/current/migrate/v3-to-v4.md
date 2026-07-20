---
title: "마이그레이션 v3 → v4"
sidebar_position: 4
description: Spirit manifest를 스키마 v3에서 v4로 마이그레이션합니다.
review_status: human
---

# Manifest 마이그레이션 v3 → v4

스키마 v4는 `[capabilities.required.loom]` 아래에 선택적 collective 기능 선언을 추가합니다.

## 단계

1. `[class].manifest_schema_version`을 `3`에서 `4`로 변경합니다.
2. Spirit에 필요한 Loom 작업만 추가합니다.

```toml
[capabilities.required.loom]
read = true
write = true
scan = false
```

`true`인 각 값은 어드미션 시 해당 `Loom*` 범위로 변환됩니다. 선언은 capability 중재, enterprise 정책, 토큰 만료 또는 tenant-map 라우팅을 우회하지 않습니다.

## 호환성 및 롤백

이 섹션의 기본값은 모두 `false`이므로 지원 창 안에서 v3 manifest도 계속 허용됩니다. v4 manifest를 되돌리려면 Loom 섹션을 제거하고 class 버전을 `3`으로 설정합니다. 단, 대상 kernel이 v3를 지원해야 합니다.

## 비준

v3→v4 스키마 변경은 `xtask/abi-ratifications.toml`에서 비준되며 ABI 안정성 원장에 기록됩니다.
