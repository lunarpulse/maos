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
| [v5](./v5) | **Current** | Adds `wasm-component` and `[class].artifact` (Story 17-3b). |
| [v4](./v4) | Supported (N-1) | Adds `[capabilities.required.loom]` (Story 13.5d). |
| [v3](./v3) | Supported | Adds `[model_provenance]` (Story 9.4b). |
| [v2](./v2) | Supported | Adds `[[cli_wrapper]]`, `[[schedule]]`, `[[gateway]]` (Epic 6). |
| [v1](./v1) | Supported | Baseline schema (Epic 1b). |

## Version policy

The kernel admits exactly the schema versions in its configured inclusive range:

- **Current configured range:** `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`, currently `1..=5`.
- **Below `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`:** rejected at admission with `EAbiTooOld`.
- **Above `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`:** rejected at admission with `EAbiTooNew`.

The authoritative version constant lives at `crates/maos-spirit-abi/src/lib.rs`:

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 5;
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 5;
```

Future kernels may change either bound. Migrate before targeting a kernel whose supported range no longer contains your manifest version.

## Latest

The latest schema reference is **[v5](./v5)**.
