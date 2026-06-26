---
title: Manifest 필드
sidebar_position: 3
description: 스키마 v3의 모든 섹션을 설명하는 전체 `spirit.toml` manifest.
review_status: machine
---

# Manifest 필드

## Problem

`spirit.toml`(manifest 스키마 버전 3)에서 사용 가능한 모든 섹션과 필드에 대한 참조가 필요합니다. manifest는 Spirit과 kernel 사이의 계약입니다 — 알 수 없는 필드는 파스 타임 거부(`deny_unknown_fields`)를, 필수 필드 누락은 어드미션 실패를 일으킵니다.

## Solution

스키마 v3의 모든 섹션을 다루는 전체 manifest:

```toml
# ── Identity ──────────────────────────────────────────────
[class]
name = "my-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"              # local | community | audited
description = "A fully-specified Spirit manifest."

[author]
name = "Ada Lovelace"
url = "https://example.com"

# ── Sandbox & Resources ──────────────────────────────────
[sandbox]
tier = "baseline"                 # baseline | hardened | paranoid

[resources]
max_memory_mb = 256
max_cpu_ms = 5000

# ── Autonomy & Output ────────────────────────────────────
[posture]
default = "supervised"            # inert | supervised | autonomous
allowed_max = "autonomous"

[output_shape]
required_fields = ["response", "confidence"]

# ── Budget ────────────────────────────────────────────────
[budget]
max_inference_calls = 100
time_cap_seconds = 300

# ── Capabilities ──────────────────────────────────────────
[capabilities.required]
[capabilities.required.provider]
complete = ["anthropic/claude-3"]

[capabilities.required.mcp]
[[capabilities.required.mcp.servers]]
name = "search-server"
tools = ["web_search"]

# ── Scheduling ────────────────────────────────────────────
[scheduling]
priority_weight = 100             # 1-255; default 100
yield_every_polls = 64
idle_window_ms = 30000

# ── Lifecycle ─────────────────────────────────────────────
[lifecycle]
enabled_hooks = [
  "on_load", "on_start", "on_idle",
  "on_frame", "on_schedule", "on_unload",
]

# ── Supervision ───────────────────────────────────────────
[supervision]
heartbeat_interval_ms = 5000
progress_threshold_ms = 30000
silent_failure_threshold_ms = 30000

[on_crash]
action = "restart"                # restart | stop | notify_operator

[on_revocation]
action = "graceful_shutdown"      # graceful_shutdown | immediate_stop | notify_only

# ── Hot-Swap ──────────────────────────────────────────────
[hot_swap]
state_schema_version = 1

[migrates_from]
versions = ["0.9.0"]

[halt_protocol_compatibility]
version = 1

# ── Epistemic Policy (§4.6.1) ─────────────────────────────
[epistemic_policy]
default_action = "verbalize_only" # verbalize_only | halt | mute

[[epistemic_policy.rules]]
tag = "uncertainty"
action = "halt"
kappa_floor = 0.6

# ── Providers (Story 5.5b) ────────────────────────────────
[[providers]]
id = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
endpoint = "https://api.anthropic.com"
model = "claude-3"
config_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

# ── MCP (Story 5.5c) ─────────────────────────────────────
[mcp]
[[mcp.servers]]
name = "local-search"
command = "npx"
args = ["-y", "@anthropic/mcp-search"]
trust_tier = "local"
allowed_tools = ["web_search"]

# ── Scheduled Invocations (Story 6.4 / FR26) ──────────────
[[schedule]]
id = "daily-digest"
cadence = "0 9 * * *"
rate_limit_per_hour = 2

# ── Gateway (Story 6.5 / FR54 / ADR-029) ─────────────────
[[gateway]]
id = "slack-gw"
type = "slack"
auth_secret_ref = "vault://slack-bot-token"
on_inbound = "on_frame"
reconnect_backoff_secs = 5
max_message_bytes = 4096

# ── CLI Wrapper (Story 6.2 / ADR-021) ────────────────────
[cli_wrapper]
command = "/usr/local/bin/mytool"
output_shape_version = 1
recovery_policy = "restart"

[cli_wrapper.posture]
default = "supervised"
allowed_max = "supervised"

# ── Model Provenance (Story 9.4b / SB-1047) ──────────────
[model_provenance]
covered_model_id = "anthropic.claude-3-opus"
training_data_lineage = ["org.example.dataset-v2"]
last_eval_timestamp = "2026-01-15T00:00:00Z"
```

## Discussion

kernel은 모든 섹션을 `#[serde(deny_unknown_fields)]`로 파스합니다 — 오타는 조용한 기본값이 아니라 파스 타임 에러가 됩니다. 생략한 섹션은 안전한 기본값으로 돌아갑니다(예: `[scheduling]`은 `priority_weight = 100`이 기본값).

기억할 핵심 규칙:

- **`manifest_schema_version`**은 `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`(현재 1)과 `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`(현재 3) 사이여야 합니다. kernel은 최신 섹션에 대해 문서화된 저하 경고와 함께 N-1 manifest을 허용합니다.
- **`trust_tier`**는 어드미션 검사에 영향을 줍니다 — `local` Spirit은 서명 검증을 건너뛰고; `audited` Spirit은 유효한 `ComplianceClaimEnvelope`가 필요합니다.
- **`[lifecycle].enabled_hooks`**은 kernel이 발화하는 훅을 제한합니다. 빈 목록은 "모든 훅 허용"을 의미합니다. Spirit ABI의 14-훅 세트에 나열된 훅만 유효한 이름입니다.
- **`[cli_wrapper]`**와 네이티브 Spirit 훅은 상호 배타적입니다 — kernel은 둘 다 선언한 manifest을 거부합니다.

전체 필드 수준 명세는 [Manifest 참조](/manifest/latest)를 참조하세요.
