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
| `MANIFEST_SCHEMA_VERSION` (current) | `5` |
| `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `1` |
| `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` | `5` |

The supported window is `1..=5`. A manifest below `MIN_SUPPORTED` is refused with `SecurityError::EAbiTooOld`; above `MAX_SUPPORTED` with `SecurityError::EAbiTooNew`.

## Migration Paths

| From | To | Guide | Kernel behavior at v5 |
|---|---|---|---|
| v1 | v2 | [v1 → v2](./v1-to-v2) | ✅ Loads (within supported window) |
| v2 | v3 | [v2 → v3](./v2-to-v3) | ✅ Loads (within supported window) |
| v3 | v4 | [v3 → v4](./v3-to-v4) | ✅ Loads (within supported window) |
| v4 | v5 | [v4 → v5](./v4-to-v5) | ✅ Current version |

## Compatibility Policy

MAOS admits the inclusive range configured by its manifest schema bounds:

- `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` is currently `1..=5`.
- A manifest below the minimum is refused at admission with `SecurityError::EAbiTooOld`.
- A manifest above the maximum is refused at admission with `SecurityError::EAbiTooNew`.

Future kernels may change either bound. Migrate proactively before targeting a kernel whose supported window no longer includes your manifest version.

For the full stability policy, see [ABI Stability](./abi-stability).

## Change Ledger

All breaking changes are recorded in [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) at the repository root. CI enforces that every breaking change entry carries a `**Migration:**` line describing how to adapt.
