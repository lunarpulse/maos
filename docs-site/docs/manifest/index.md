---
title: Manifest Schema Reference
sidebar_position: 0
description: Overview of the MAOS Spirit manifest schema and links to versioned references.
---

# Manifest Schema Reference

Every Spirit ships a `manifest.toml` describing its identity, resource budget, capabilities, and operational surface. The kernel validates this manifest at admission against the current `MANIFEST_SCHEMA_VERSION` constant defined in `maos-spirit-abi`.

## Schema versions

| Version | Status | Description |
|---------|--------|-------------|
| [v3](./v3) | **Current** | Adds `[model_provenance]` (Story 9.4b). |
| [v2](./v2) | Supported (N-1) | Adds `[[cli_wrapper]]`, `[[schedule]]`, `[[gateway]]` (Epic 6). |
| [v1](./v1) | Hard-refused (N-2) | Baseline schema (Epic 1b). |

## Version policy

The kernel enforces an **N-1 supported / N-2 hard-refusal** policy:

- **Current (N):** full feature set, no degradation.
- **N-1:** admitted with documented degradation — newer sections default via `#[serde(default)]`. The kernel emits `WARN`-level notices for each defaulted section.
- **N-2 and older:** rejected at admission with `EAbiTooOld`.

The authoritative version constant lives at `crates/maos-spirit-abi/src/lib.rs`:

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 3;
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
```

## Latest

The latest schema reference is **[v3](./v3)**.
