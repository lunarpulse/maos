---
title: Manifest Fields
sidebar_position: 3
description: A structurally valid schema v5 `spirit.toml` reference for a first-party MAOS Spirit.
---

# Manifest Fields

## Problem

You need a reference manifest whose structure passes the daemon's manifest gates. A manifest is a contract between your Spirit and the kernel: unknown fields are rejected at parse time (`deny_unknown_fields`), and required admission sections must be present. Production admission also requires a class that the daemon knows.

## Solution

This schema v5 manifest covers the sections parsed and validated by the daemon admission path for a first-party Rust class. Compile and register that class before using it for production admission:

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

`manifest_schema_version` must be within the kernel's supported range. Schema v5 is the current maximum and supports the `wasm-component` form.

This example's `my-spirit` name is not a built-in daemon class. Its manifest is structurally valid, but a real first-party class must be compiled and registered with the daemon before the daemon can admit it.

`forms` accepts `rust-inproc`, `subprocess`, and `wasm-component`. A `wasm-component` manifest must declare that form alone, use schema v5 or later, declare a relative `.wasm` `artifact`, and set `[sandbox] tier = "T2"`. `artifact` is forbidden for the other forms. For a full third-party TypeScript example, see [WASM Component Spirit](./wasm-component-spirit).

`trust_tier` is one of `local`, `org-internal`, `public-untrusted`, or `public-vetted`; use the hyphenated spellings inside `[class]`. The daemon requires `[sandbox]`, `[resources]`, `[posture]`, and `[output_shape]`; `[budget]` is optional, but if present requires both `context_window_size` and `time_cap_seconds`.
Resource ceilings are independent: `cpu_max_pct` is a positive percentage of one CPU, `memory_max_mb` is MiB, `pids_max` is an optional positive `u32` process/task limit, and `fd_max` is an optional independent file-descriptor limit. Omit a field to leave it unset; schema v4/v5 manifests that omit `pids_max` remain accepted. `pids_max` is enforced only on Linux (native T2 cgroup `pids.max`, T3 `--pids-limit`); the native macOS and Windows sandboxes neither apply nor refuse it. For Worker/T3 behavior and native T2 delegation requirements, see [CLI Wrapper Spirit](./cli-wrapper-spirit).

`[capabilities.required.log]` declares two independent boolean grants: `recall` for `Scope::LogRecall`, and `fetch` for `Scope::LogFetch`. Both default to `false`; unknown keys are rejected. The example opts into both. This table requires schema v5: below schema 5 it is dropped with a degradation warning, just like `[capabilities.required.iac]`, and grants no log scopes. Omission grants neither capability. Calls still undergo per-call policy mediation; a declaration alone does not bypass operator denial or issuance quota.

The issuance budget is not a manifest or operator setting:
`DEFAULT_ISSUANCE_QUOTA` is an inclusive 60,000 issuances per 60 s monotonic
tumbling window per admitted scheduler pid/incarnation, with ≤ 2·B burst across
one boundary. Policy precedes quota and token issuance; failed signing refunds.
Exhausted recall/fetch return `capability-denied` and can resume next window.
See the [issuance-quota reference](/manifest/v5#capability-issuance-quota) for
operator-only `cap.quota:*` signals, qualified delivery, all path-specific
denials and labelled limits. In particular, there is no Spirit-facing issuance
pre-warning or stable-Spirit/principal-wide/Worker-spawn-churn bound. This does
not configure ADR-016 context-token accounting: that is `[budget]`'s optional
`context_pressure_threshold` / `context_limit_threshold` (defaults 0.80 / 0.95;
`18-5-context-token-budget-accounting`).

See [Manifest Reference](/manifest/latest) for the complete field-level specification.
