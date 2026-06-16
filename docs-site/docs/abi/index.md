---
title: ABI Reference
sidebar_position: 1
description: Overview of the maos-spirit-abi crate — wire-stable types for the MAOS Spirit ABI.
---

# ABI Reference

The `maos-spirit-abi` crate provides the **wire-stable types** that form the contract between the MAOS kernel and Spirits. It is a `#![no_std]` crate with `#![forbid(unsafe_code)]`.

## Design Principles

- **Wire stability**: Types in this crate are the ABI surface. Changes follow strict rules (see [ABI Stability](/migrate/abi-stability)).
- **`#![no_std]`**: The crate depends only on `alloc`, enabling use in constrained environments and subprocess-form Spirits.
- **`#![forbid(unsafe_code)]`**: No unsafe code anywhere in the ABI surface.
- **Serde-first serialization**: All wire types derive `serde::Serialize` and `serde::Deserialize` for codec-agnostic encoding.

## Modules

| Module | Description | Introduced |
|---|---|---|
| [lifecycle](./lifecycle) | `Spirit` trait (14 hooks), `SpiritVtable`, payload types | Story 2.1 (11 hooks), Story 5.2 (+3 hot-swap hooks) |
| [ctx](./ctx) | `Ctx` type — Spirit-author-facing context for hook invocations | Story 2.1 |
| [compliance](./compliance) | `ComplianceClaim` envelope — Ed25519-signed attestation schema | Story 1b.4 (frozen) |
| [identity](./identity) | `SpiritId`, `HostId`, `FrameKind` — wire-stable identity and frame discrimination | Story 1b.1 |
| [cancellation](./cancellation) | `CancellationSignal` trait, `NeverCancel` — runtime-agnostic cancellation | Story 2.1 |
| [gateway](./gateway) | `GatewaySubmodule` trait, `GatewayCtx` — external messaging gateway contract | Story 6.5 (ADR-029) |
| [deprecation](./deprecation) | `DeprecationWarning` — deprecation channel for ABI surface evolution | Story 7.1 |
| [constants](./constants) | `ABI_VERSION`, `MANIFEST_SCHEMA_VERSION`, supported window constants | Story 1b.4, Epic 6, Story 9.4b |

## Re-exports

The crate re-exports `DeprecationWarning` at the root level:

```rust
use maos_spirit_abi::DeprecationWarning;
```

## Version Constants

```rust
use maos_spirit_abi::{
    ABI_VERSION,                         // 1
    MANIFEST_SCHEMA_VERSION,             // 3
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION, // 1
    MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION, // 3
};
```

See [Constants](./constants) for full documentation.
