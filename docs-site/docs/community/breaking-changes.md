---
title: Breaking Changes
sidebar_position: 5
description: Ledger of breaking changes to the MAOS ABI and manifest surface.
---

# Breaking Changes

> **Full document:** [`BREAKING.md`](https://github.com/lunarpulse/maos/blob/main/BREAKING.md)

The breaking changes ledger is CI-enforced (NFR-Maint-7). Every breaking
change lands with a dated entry describing what changed and how to migrate.

For the ABI stability commitments and compatibility window, see
[`STABILITY.md`](https://github.com/lunarpulse/maos/blob/main/STABILITY.md)
and the [ABI Stability guide](/migrate/abi-stability).

## Entry Format

Each entry contains:
- Date and short title heading
- Description of what changed and why
- `**Migration:**` line with concrete steps to adapt

## Current Entries

### 2026-05-31 — v0.x → v1.0 ABI stability commitments activated

Story 7.5a activated the ABI Stability Triple enforcement. The kernel now
rejects Spirits with out-of-window schema versions (`EAbiTooOld`, `EAbiTooNew`)
and incompatible substrate versions (`ESubstrateTooOld`).

**Migration:** No action required for Spirits declaring truthful
`min_substrate_version` and `manifest_schema_version` within the supported
window.
