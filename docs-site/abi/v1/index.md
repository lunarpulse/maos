<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `index` Module {#abi-index-module}

## Related {#abi-index-related}

- [ABI Stability Policy](/migrate/abi-stability) — compatibility window and bump rules
- [Constants](./constants) — `ABI_VERSION` and `MANIFEST_SCHEMA_VERSION` reference


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

`maos-spirit-abi` — wire-stable types ONLY (`#![no_std]`).

`ABI_VERSION` was bumped to `1` in Story 1b.4 when the ComplianceClaim
schema was frozen under the joint Mary+Winston adversarial review (see
`_bmad-output/planning-artifacts/compliance-claim-schema-review.md`).

Bumping `ABI_VERSION` here is the **ABI-bump trigger** per §8.5.
The abi-diff gate (`--deny removed --deny changed`) is baselined at
`abi-baseline/v1-pre-bump.txt` after the 1b.4 freeze.

## Story 2.1 additive surface (does NOT bump `ABI_VERSION`) {#maos-spirit-abi-story2-1additivesurface-doesnotbump-abi-version}

Story 2.1 adds:
- `pub mod cancellation` — `CancellationSignal` trait + `NeverCancel`
- `pub mod lifecycle` — `Spirit` trait (11 hooks) + `SpiritVtable<T>` + payload types
- `pub mod ctx` — `Ctx` Spirit-author-facing context type

All additions are ABI-additive per §8.5 rows 7+8. `ABI_VERSION` remains `1`.

# Version history {#maos-spirit-abi-versionhistory}

| Module / constant | Introduced | Notes |
|---|---|---|
| `ABI_VERSION` | Story 1b.4 | Frozen at `1` at ComplianceClaim envelope freeze |
| `cancellation`, `lifecycle`, `ctx` | Story 2.1 | Additive ABI surface, no version bump |
| `identity` | Story 1b.1 | Wire-stable since v0.1-β |
| `compliance` | Story 1b.4 | Frozen schema, ABI_VERSION bump trigger |
| `gateway` | Story 6.5 | ADR-029 binding-v1.0 |
| `deprecation` | Story 7.1 | Empty-present deprecation channel |
| `MANIFEST_SCHEMA_VERSION = 4` | Story 13.5d | `[capabilities.required.loom]` section |

## Modules {#maos-spirit-abi-modules}
| Module | Description | Introduced |
|---|---|---|
| [`cancellation`] | `CancellationSignal` trait + `NeverCancel` — runtime-agnostic cancellation | Story 2.1 |
| [`compliance`] | `ComplianceClaim` envelope — Ed25519-signed attestation schema | Story 1b.4 (frozen) |
| [`ctx`] | `Ctx` type — Spirit-author-facing context for hook invocations | Story 2.1 |
| [`deprecation`] | `DeprecationWarning` — deprecation channel for ABI surface evolution | Story 7.1 |
| [`gateway`] | `GatewaySubmodule` trait + `GatewayCtx` — external messaging gateway contract | Story 6.5 (ADR-029) |
| [`identity`] | `SpiritId`, `HostId`, `FrameKind` — wire-stable identity and frame discrimination | Story 1b.1 |
| [`lifecycle`] | `Spirit` trait + `SpiritVtable` + payload types | Story 2.1 |
