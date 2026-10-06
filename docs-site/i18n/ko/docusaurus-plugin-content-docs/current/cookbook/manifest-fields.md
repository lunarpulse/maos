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

`trust_tier`는 `local`, `org-internal`, `public-untrusted`, `public-vetted` 중 하나입니다. `[class]` 안에서는 하이픈 표기를 사용하세요. daemon은 `[sandbox]`, `[resources]`, `[posture]`, `[output_shape]`를 요구합니다. `[budget]`은 선택 사항이지만, 존재하면 `context_window_size`와 `time_cap_seconds`를 모두 요구합니다.

전체 필드 수준 명세는 [Manifest 참조](/manifest/latest)를 참조하세요.
