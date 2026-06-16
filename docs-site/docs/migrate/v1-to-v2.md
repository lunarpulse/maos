---
title: "Migrate v1 → v2"
sidebar_position: 2
description: Step-by-step guide to migrate a Spirit manifest from schema version 1 to version 2.
---

# Migrate Manifest v1 → v2

Manifest schema version 2 was introduced in Epic 6 (retro 2026-05-28) to track four additive sections landed across Stories 6.2, 6.4, and 6.5.

## What Changed

All four additions use `#[serde(default)]`, so they are **wire-compatible** — a v1 manifest loads under a v2+ kernel without modification. However, you should bump the version to declare intent and access the new features.

| Addition | Story | Manifest Section | Purpose |
|---|---|---|---|
| CLI wrappers | 6.2 | `[[cli_wrapper]]` | Declare CLI subprocess invocations with `command`, `output_shape_version`, `recovery_policy`, `posture`, `shutdown_signal` |
| Schedules | 6.4 | `[[schedules]]` | Declare scheduled invocations with `id`, `cadence`, `rate_limit_per_hour`, `compliance_claim_ref_hex`, `side_effect_scopes`, `payload_b64` |
| Gateways | 6.5 | `[gateways]` / `[[gateway]]` | Declare external gateway integrations (Telegram, Slack, Discord, etc.) with `id`, `type`, `auth_secret_ref`, `inbound_routing` |
| Consent envelope extensions | 6.4 | `ConsentEnvelope` fields | Added `intent_class` and `valid_until_ns` to the consent envelope shape |

## Kernel Behavior

| Kernel version | v1 manifest behavior |
|---|---|
| Schema v2 kernel | ✅ Loads with WARN-level degradation notes |
| Schema v3 kernel | ✅ Loads (still within the supported window `1..=3`) |
| Future schema v4 kernel | ⛔ **Hard refusal** — `SecurityError::EAbiTooOld` (N-2 boundary) |

When the kernel reaches schema version 4, **v1 manifests will be refused**. Migrate now to avoid future breakage.

## Migration Steps

### Step 1: Bump the Schema Version

In your manifest's `[class]` section, change (or add) the schema version:

```toml
[class]
name = "my-spirit"
manifest_schema_version = 2   # was 1
min_substrate_version = "0.1.0-alpha"
```

### Step 2: (Optional) Add CLI Wrapper Declarations

If your Spirit invokes CLI subprocesses, declare them:

```toml
[[cli_wrapper]]
command = "/usr/bin/my-tool"
output_shape_version = 1
recovery_policy = "restart"
posture = "restricted"
shutdown_signal = "SIGTERM"
```

### Step 3: (Optional) Add Schedule Declarations

If your Spirit needs scheduled invocations:

```toml
[[schedules]]
id = "daily-check"
cadence = "0 0 * * *"
rate_limit_per_hour = 1
compliance_claim_ref_hex = "abcdef0123456789"
side_effect_scopes = ["network", "filesystem"]
payload_b64 = ""
```

### Step 4: (Optional) Add Gateway Declarations

If your Spirit integrates with external messaging platforms:

```toml
[gateways]

[[gateway]]
id = "telegram-bot"
type = "telegram"
auth_secret_ref = "tg-bot-token"
inbound_routing = "on_frame"
```

### Step 5: Validate

Load your updated manifest against a v2+ kernel. The kernel performs strict validation with `deny_unknown_fields` — any typos or unrecognized fields are caught at admission.

## Rollback

Because all v2 additions use `#[serde(default)]`, you can revert `manifest_schema_version` back to `1` and remove the new sections. The manifest will load on any kernel within the supported window.

## Reference

- [ABI Stability Policy](./abi-stability) — N-1/N-2 rules and the ABI Stability Triple
- [ABI Constants](/abi/constants) — live values for `MANIFEST_SCHEMA_VERSION` and supported window
- [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) — CI-enforced change ledger
