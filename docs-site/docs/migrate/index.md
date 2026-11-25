---
title: Migration Guides
sidebar_position: 1
description: Overview of breaking changes and migration paths for MAOS manifest schema versions.
---

# Migration Guides

MAOS uses a **manifest schema version** to track changes to the Spirit manifest format. The kernel enforces a compatibility window at admission time — Spirits outside the window are refused with a typed error.

## Current State

| Constant | Value |
|---|---|
| `MANIFEST_SCHEMA_VERSION` (ratified) | `8` |
| `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `1` |
| `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `9` |

Schema 8 is the latest ratified schema; [schema 9](../manifest/v9) is proposed (Story 22-9) and awaits operator ratification. The support window is `1..=9`.
A manifest below `MIN_SUPPORTED` is refused with `SecurityError::EAbiTooOld`; above `MAX_SUPPORTED` with `SecurityError::EAbiTooNew`.

## Migration Paths

| From | To | Guide | Kernel behavior |
|---|---|---|---|
| v1 | v2 | [v1 → v2](./v1-to-v2) | ✅ Loads (within supported window) |
| v2 | v3 | [v2 → v3](./v2-to-v3) | ✅ Loads (within supported window) |
| v3 | v4 | [v3 → v4](./v3-to-v4) | ✅ Loads (within supported window) |
| v4 | v5 | [v4 → v5](./v4-to-v5) | ✅ Loads (within supported window) |
| v5 | v6 | [Native skill declaration](../manifest/v6) | ✅ Loads (within supported window) |
| v6 | v7 | [FR8 declarations](../manifest/v7) | ✅ Loads for non-listeners (within supported window) |
| v7 | v8 | [Explicit telemetry subscription](../manifest/v8) | Required for telemetry listeners |
| v8 | v9 | [Consent schema floor](../manifest/v9) | Required for `[consent]` declarations (proposed) |

## Compatibility Policy

MAOS admits the inclusive range configured by its manifest schema bounds:

- `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` is currently `1..=9` in the candidate.
- A manifest below the minimum is refused at admission with `SecurityError::EAbiTooOld`.
- A manifest above the maximum is refused at admission with `SecurityError::EAbiTooNew`.
- Schemas 1–7 remain supported for non-listeners. Any `on_telemetry_event` listing or `[lifecycle.telemetry]` table below schema 8 is refused, not degraded away.
- Schemas 1–8 remain supported for manifests that declare no `[consent]` table. A `[consent]` table below schema 9 is refused, not degraded away.

Future kernels may change either bound. Migrate proactively before targeting a kernel whose supported window no longer includes your manifest version.

For the full stability policy, see [ABI Stability](./abi-stability).

## Change Ledger

All breaking changes are recorded in [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) at the repository root. CI enforces that every breaking change entry carries a `**Migration:**` line describing how to adapt.
