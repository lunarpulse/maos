---
title: Manifest Fields
sidebar_position: 3
description: Complete spirit.toml manifest with all schema v3 sections explained.
---

# Manifest Fields

## Problem

You need a reference for every section and field available in `spirit.toml` (manifest schema version 3). The manifest is the contract between your Spirit and the kernel — an unknown field causes a parse-time rejection (`deny_unknown_fields`), and a missing required field causes an admission failure.

## Solution

A complete manifest covering all schema v3 sections:

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

The kernel parses every section with `#[serde(deny_unknown_fields)]` — a typo becomes a parse-time error, not a silent default. Sections you omit fall back to safe defaults (e.g., `[scheduling]` defaults to `priority_weight = 100`).

Key rules to remember:

- **`manifest_schema_version`** must be between `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` (currently 1) and `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` (currently 3). The kernel accepts N-1 manifests with documented degradation warnings for newer sections.
- **`trust_tier`** affects admission checks — `local` Spirits skip signature verification; `audited` Spirits require a valid `ComplianceClaimEnvelope`.
- **`[lifecycle].enabled_hooks`** limits which hooks the kernel fires. An empty list means "all hooks allowed". Only hooks listed in the Spirit ABI's 14-hook set are valid names.
- **`[cli_wrapper]`** and native Spirit hooks are mutually exclusive — the kernel rejects a manifest that declares both.

See [Manifest Reference](/manifest/latest) for the full field-level specification.
