---
title: Migration Guides
sidebar_position: 1
description: Overview of breaking changes and migration paths for MAOS manifest schema versions.
---

# Migration Guides

MAOS uses a **manifest schema version** to track breaking changes to the Spirit manifest format. The kernel enforces a compatibility window at admission time — Spirits outside the window are refused with a typed error.

## Current State

| Constant | Value |
|---|---|
| `MANIFEST_SCHEMA_VERSION` (current) | `3` |
| `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `1` |
| `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `3` |

The supported window is `1..=3`. A manifest below `MIN_SUPPORTED` is refused with `SecurityError::EAbiTooOld`; above `MAX_SUPPORTED` with `SecurityError::EAbiTooNew`.

## Migration Paths

| From | To | Guide | Kernel behavior at v3 |
|---|---|---|---|
| v1 | v2 | [v1 → v2](./v1-to-v2) | ✅ Loads (within N-1 window) |
| v2 | v3 | [v2 → v3](./v2-to-v3) | ✅ Current version |
| v1 | v3 | Apply [v1 → v2](./v1-to-v2) then [v2 → v3](./v2-to-v3) | ✅ Loads (within supported window) |

## Compatibility Policy

MAOS follows an **N-1 supported / N-2 hard-refusal** policy:

- **N** (current): Full support, strict load with `deny_unknown_fields`.
- **N-1**: Supported with WARN-level degradation notes for omitted sections.
- **N-2 and below**: Hard refusal at admission (`SecurityError::EAbiTooOld`).

When the kernel bumps to schema version 4, version 1 manifests will hit the N-2 boundary and be refused. Migrate proactively.

For the full stability policy, see [ABI Stability](./abi-stability).

## Change Ledger

All breaking changes are recorded in [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) at the repository root. CI enforces that every breaking change entry carries a `**Migration:**` line describing how to adapt.
