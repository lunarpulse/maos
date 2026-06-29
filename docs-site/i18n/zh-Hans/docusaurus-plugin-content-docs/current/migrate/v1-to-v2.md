---
title: "마이그레이션 v1 → v2"
sidebar_position: 2
description: Spirit manifest를 스키마 버전 1에서 버전 2로 마이그레이션하는 단계별 가이드.
review_status: machine
---

# Manifest 마이그레이션 v1 → v2

Manifest 스키마 버전 2는 Stories 6.2, 6.4, 6.5에 걸쳐 들어온 네 가지 추가 섹션을 추적하기 위해 Epic 6(소급 2026-05-28)에서 도입되었습니다.

## 변경된 내용

네 가지 추가는 모두 `#[serde(default)]`를 사용하므로 **와이어 호환**입니다 — v1 manifest는 수정 없이 v2+ kernel에서 로드됩니다. 단, 의도를 선언하고 새 기능에 접근하려면 버전을 올려야 합니다.

| 추가 | Story | Manifest 섹션 | 목적 |
|---|---|---|---|
| CLI 래퍼 | 6.2 | `[[cli_wrapper]]` | `command`, `output_shape_version`, `recovery_policy`, `posture`, `shutdown_signal`로 CLI 서브프로세스 호출 선언 |
| 스케줄 | 6.4 | `[[schedules]]` | `id`, `cadence`, `rate_limit_per_hour`, `compliance_claim_ref_hex`, `side_effect_scopes`, `payload_b64`로 스케줄된 호출 선언 |
| 게이트웨이 | 6.5 | `[gateways]` / `[[gateway]]` | `id`, `type`, `auth_secret_ref`, `inbound_routing`으로 외부 게이트웨이 통합(Telegram, Slack, Discord 등) 선언 |
| 동의 봉투 확장 | 6.4 | `ConsentEnvelope` 필드 | 동의 봉투 형태에 `intent_class`와 `valid_until_ns` 추가 |

## kernel 동작

| kernel 버전 | v1 manifest 동작 |
|---|---|
| 스키마 v2 kernel | ✅ WARN 수준 저하 노트와 함께 로드 |
| 스키마 v3 kernel | ✅ 로드(지원 윈도우 `1..=3` 내) |
| 향후 스키마 v4 kernel | ⛔ **거부** — `SecurityError::EAbiTooOld`(N-2 경계) |

kernel이 스키마 버전 4에 도달하면 **v1 manifest는 거부됩니다**. 향후 브레이킹을 피하려면 지금 마이그레이션하세요.

## 마이그레이션 단계

### 1단계: 스키마 버전 올리기

manifest의 `[class]` 섹션에서 스키마 버전을 변경(또는 추가)합니다:

```toml
[class]
name = "my-spirit"
manifest_schema_version = 2   # was 1
min_substrate_version = "0.1.0-alpha"
```

### 2단계: (선택) CLI 래퍼 선언 추가

Spirit이 CLI 서브프로세스를 호출하면 다음과 같이 선언합니다:

```toml
[[cli_wrapper]]
command = "/usr/bin/my-tool"
output_shape_version = 1
recovery_policy = "restart"
posture = "restricted"
shutdown_signal = "SIGTERM"
```

### 3단계: (선택) 스케줄 선언 추가

Spirit에 스케줄된 호출이 필요하면:

```toml
[[schedules]]
id = "daily-check"
cadence = "0 0 * * *"
rate_limit_per_hour = 1
compliance_claim_ref_hex = "abcdef0123456789"
side_effect_scopes = ["network", "filesystem"]
payload_b64 = ""
```

### 4단계: (선택) 게이트웨이 선언 추가

Spirit이 외부 메시징 플랫폼과 통합되면:

```toml
[gateways]

[[gateway]]
id = "telegram-bot"
type = "telegram"
auth_secret_ref = "tg-bot-token"
inbound_routing = "on_frame"
```

### 5단계: 검증

갱신된 manifest를 v2+ kernel에 대해 로드합니다. kernel은 `deny_unknown_fields`로 엄격한 검증을 수행합니다 — 오타나 인식할 수 없는 필드는 어드미션 시점에 잡힙니다.

## 롤백

모든 v2 추가는 `#[serde(default)]`를 사용하므로, `manifest_schema_version`을 `1`로 되돌리고 새 섹션을 제거할 수 있습니다. manifest는 지원 윈도우 내의 어떤 kernel에서도 로드됩니다.

## 참조

- [ABI 안정성 정책](./abi-stability) — N-1/N-2 규칙과 ABI Stability Triple
- [ABI 상수](/abi/v1/constants) — `MANIFEST_SCHEMA_VERSION`과 지원 윈도우의 실시간 값
- [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) — CI가 시행하는 변경 원장

## 한국 규제 참고사항

<!-- TODO: Korean regulatory addendum, content deferred post-v1.0 -->
