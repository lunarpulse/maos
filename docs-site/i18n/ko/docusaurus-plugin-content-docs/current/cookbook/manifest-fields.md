---
title: Manifest 필드
sidebar_position: 3
description: 퍼스트파티 MAOS Spirit을 위한 구조적으로 유효한 스키마 v5 `spirit.toml` 참조.
review_status: machine
---

# Manifest 필드

## Problem

daemon의 manifest 게이트를 통과하는 구조를 가진 참조 manifest가 필요합니다. manifest는 Spirit과 kernel 사이의 계약입니다. 알 수 없는 필드는 파스 타임에 거부되고(`deny_unknown_fields`), 필수 어드미션 섹션은 반드시 있어야 합니다. 프로덕션 어드미션에는 daemon이 아는 class도 필요합니다.

## Solution

이 스키마 v5 manifest는 퍼스트파티 Rust class에 대해 daemon 어드미션 경로가 파스하고 검증하는 섹션을 다룹니다. 프로덕션 어드미션에 사용하기 전에 해당 class를 컴파일하고 daemon에 등록하세요:

```toml
# ── Identity ──────────────────────────────────────────────
[class]
name = "my-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 5
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"              # local | org-internal | public-untrusted | public-vetted
description = "A structurally valid Spirit manifest."

[author]
name = "Ada Lovelace"
url = "https://example.com"

# ── Sandbox & Resources ──────────────────────────────────
[sandbox]
tier = "T0"                      # An in-process Spirit declares T0.

[resources]
cpu_max_pct = 100
memory_max_mb = 256
fd_max = 256
pids_max = 64

# ── Autonomy & Output ────────────────────────────────────
[posture]
default = "assistive"
allowed_max = "autonomous"

[output_shape]
required_fields = ["response", "confidence"]

# ── Budget ────────────────────────────────────────────────
[budget]
context_window_size = 32768
time_cap_seconds = 300

# ── Capabilities ──────────────────────────────────────────
[capabilities.required]
[capabilities.required.provider]
complete = ["anthropic/claude-3"]

[capabilities.required.log]
recall = true
fetch = true

# ── Scheduling & Lifecycle ────────────────────────────────
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[lifecycle]
enabled_hooks = ["on_load", "on_start", "on_idle", "on_frame", "on_schedule", "on_unload"]

# ── Epistemic Policy ──────────────────────────────────────
[epistemic_policy]
default_action = "verbalize_only"

[[epistemic_policy.rules]]
tag = "uncertainty"
action = "halt"
on_confidence_below = 0.6
```

## Discussion

`manifest_schema_version`은 kernel이 지원하는 범위 안에 있어야 합니다. 스키마 v5는 현재 최대 버전이며 `wasm-component` 형식을 지원합니다.

이 예시의 `my-spirit` 이름은 daemon의 내장 class가 아닙니다. manifest는 구조적으로 유효하지만, daemon이 어드미션하려면 실제 퍼스트파티 class를 컴파일하고 등록해야 합니다.

`forms`는 `rust-inproc`, `subprocess`, `wasm-component`를 받습니다. `wasm-component` manifest는 그 형식만 단독으로 선언하고, 스키마 v5 이상을 사용하며, 상대 `.wasm` `artifact`를 선언하고, `[sandbox] tier = "T2"`를 설정해야 합니다. 다른 형식에서는 `artifact`가 금지됩니다. 완전한 서드파티 TypeScript 예시는 [WASM Component Spirit](./wasm-component-spirit)를 참조하세요.

리소스 상한은 서로 독립적입니다. `cpu_max_pct`는 CPU 한 개 대비 양의 백분율이고, `memory_max_mb`는 MiB이며, `pids_max`는 선택적 양의 `u32` 프로세스/태스크 제한이고, `fd_max`는 선택적인 별도 파일 디스크립터 제한입니다. 필드를 생략하면 설정되지 않습니다. `pids_max`를 생략한 schema v4/v5 manifest는 계속 허용됩니다. `pids_max`는 Linux에서만 적용됩니다(native T2 cgroup `pids.max`, T3 `--pids-limit`). native macOS 및 Windows sandbox는 이를 적용하지도 거부하지도 않습니다. Worker/T3 동작과 native T2 위임 요구 사항은 [CLI Wrapper Spirit](./cli-wrapper-spirit)을 참조하세요.

`[capabilities.required.log]`는 서로 독립적인 두 boolean 권한을 선언합니다. `recall`은 `Scope::LogRecall`, `fetch`는 `Scope::LogFetch`에 해당합니다. 두 값의 기본값은 모두 `false`이며 알 수 없는 key는 거부됩니다. 예제는 두 권한을 모두 요청합니다. 이 table은 스키마 v5가 필요합니다. 스키마 5 미만에서는 `[capabilities.required.iac]`와 마찬가지로 기능 저하 경고와 함께 제거되며 log scope를 부여하지 않습니다. 생략하면 어느 권한도 부여되지 않습니다. 호출마다 정책 중재를 거치므로 선언만으로 운영자 거부나 발급 할당량을 우회할 수 없습니다.

발급 예산은 manifest나 운영자 설정이 아닙니다. `DEFAULT_ISSUANCE_QUOTA`는
승인된 scheduler pid/incarnation마다 경계를 포함해 60 s monotonic tumbling
window당 60,000회 발급을 허용하며, 한 경계를 가로지르는 burst는 ≤ 2·B입니다.
정책을 먼저 평가하고 할당량을 차감한 뒤 token을 발급하며, 서명 실패는 환불합니다.
소진된 recall/fetch는 `capability-denied`를 반환하고 다음 window에 재개할 수 있습니다.
운영자 전용 `cap.quota:*` 신호, 전달 한계, 모든 경로별 거부 및 명시된 한계는
[발급 할당량 참조](/manifest/v5#capability-issuance-quota)를 확인하세요.
특히 Spirit 대상 발급 사전 경고가 없고 stable-Spirit, principal 전체 또는
Worker spawn churn을 제한하지 않습니다. 이는 ADR-016의 context-token 회계를
설정하거나 구현하지 않습니다. 해당 회계는 17-9가 아닌
`18-5-context-token-budget-accounting`이 담당합니다.

전체 필드 수준 명세는 [Manifest 참조](/manifest/latest)를 참조하세요.
