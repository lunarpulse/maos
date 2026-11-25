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
| [v9](./v9) | Proposed (Story 22-9) | `[consent]` schema floor and recipient roster check. |
| [v8](./v8) | **Latest ratified** | Explicit telemetry subscription (Story 21-9). |
| [v7](./v7) | Supported (N-1) | FR8 manifest declarations (Story 20-8). |
| [v6](./v6) | Supported | Native `skill.author.self` declaration (Story 18-7). |
| [v5](./v5) | Supported | Adds `wasm-component` and `[class].artifact` (Story 17-3b). |
| [v4](./v4) | Supported | Adds `[capabilities.required.loom]` (Story 13.5d). |
| [v3](./v3) | Supported | Adds `[model_provenance]` (Story 9.4b). |
| [v2](./v2) | Supported | Adds `[cli_wrapper]`, `[[schedule]]`, `[[gateway]]` (Epic 6). |
| [v1](./v1) | Supported | Baseline schema (Epic 1b). |

## Version policy

The schema 9 kernel checks versions against its inclusive range:

- **Supported range:** `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION..=MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`, currently `1..=9`.
- **Below `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`:** rejected at admission with `EAbiTooOld`.
- **Above `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION`:** rejected at admission with `EAbiTooNew`.

Being in range does not make every declaration valid. Schemas 1–7 still support
non-listeners, but a manifest below schema 8 that lists `on_telemetry_event` or
contains `[lifecycle.telemetry]` is refused, not loaded with a warning; likewise a
`[consent]` table below schema 9. `ABI_VERSION` and the manifest floor remain `1`.

The authoritative version constant lives at `crates/maos-spirit-abi/src/lib.rs`:

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 9;
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = MANIFEST_SCHEMA_VERSION;
```

Future kernels may change either bound. Migrate before targeting a kernel whose supported range no longer contains your manifest version.

## Latest

The latest ratified schema is **[v8](./v8)**. Schema [v9](./v9) is proposed (Story 22-9) and awaits
operator ratification. Check the kernel version before targeting either.
