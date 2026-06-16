---
title: ABI Stability Policy
sidebar_position: 4
description: The ABI Stability Triple, N-1/N-2 compatibility policy, and how STABILITY.md and BREAKING.md track changes.
---

# ABI Stability Policy

MAOS enforces a mechanically-checked compatibility contract between the kernel and Spirits. This page explains the ABI Stability Triple, the compatibility window, and how changes are tracked.

## The ABI Stability Triple

Every MAOS kernel publishes three version numbers that together form the **ABI Stability Triple**:

```
(kernel_version, abi_version, manifest_schema_version)
```

| Leg | Current Value | What It Tracks |
|---|---|---|
| `kernel_version` | `0.1.0-alpha` | Semantic version of the kernel binary (from `Cargo.toml`) |
| `abi_version` | `1` | Wire-format version of the Spirit ABI (`ComplianceClaim` envelope, vtable layout) |
| `manifest_schema_version` | `3` | Schema version of the Spirit manifest TOML format |

These constants live in `maos-spirit-abi/src/lib.rs` and are the single source of truth consumed by admission checks and CI gates.

## Compatibility Window

The kernel enforces a **fail-closed** compatibility window in both directions at Spirit admission:

| Manifest Schema Version | Kernel Behavior |
|---|---|
| `= MANIFEST_SCHEMA_VERSION` (current) | ✅ Strict load (`deny_unknown_fields`) |
| `= N-1` (one below current) | ✅ Supported — loads with WARN-level degradation notes for omitted sections |
| `< MIN_SUPPORTED` (N-2 and below) | ⛔ Hard refusal — `SecurityError::EAbiTooOld` |
| `> MAX_SUPPORTED` (future) | ⛔ Hard refusal — `SecurityError::EAbiTooNew` |
| `min_substrate_version > kernel_version` | ⛔ Hard refusal — `SecurityError::ESubstrateTooOld` (FR8) |

There is no silent warn-and-ignore window. A manifest is a security artifact; fail-open admission would be a security defect.

## N-1 / N-2 Policy

- **N-1 supported**: A manifest one version behind the current kernel still loads. The kernel emits WARN-level notes identifying sections the older manifest omits. The Spirit functions but does not benefit from new manifest features.
- **N-2 hard refusal**: A manifest two or more versions behind is refused at admission with `SecurityError::EAbiTooOld`. The error message names the declared version and the supported window.

This gives Spirit authors **one full version cycle** to migrate their manifests after a schema bump.

## ABI Version vs. Manifest Schema Version

These are independent version numbers:

- **`ABI_VERSION`** tracks the wire format of the Spirit ABI — the `ComplianceClaim` schema, vtable layout, and hook signatures. It bumps rarely (only on breaking wire-format changes per the §8.5 rules).
- **`MANIFEST_SCHEMA_VERSION`** tracks the Spirit manifest TOML schema. It bumps when new manifest sections are added (even additive ones, for tracking purposes).

Adding an optional manifest section bumps `MANIFEST_SCHEMA_VERSION` but does NOT bump `ABI_VERSION`. Breaking changes to the compliance claim schema or hook signatures bump `ABI_VERSION`.

## What Constitutes an ABI Break

Per §8.5, the following changes **bump** `ABI_VERSION`:

| Change | ABI Break? |
|---|---|
| Add required field without `#[serde(default)]` | **YES** |
| Rename any field | **YES** |
| Remove any field | **YES** |
| Change any field's type | **YES** |
| Reorder enum variants without updating `#[repr(u8)]` discriminants | **YES** |
| Remove an enum variant | **YES** |

The following changes do **NOT** bump `ABI_VERSION`:

| Change | ABI Break? |
|---|---|
| Add optional field with `#[serde(default)]` | No |
| Add enum variant at end with explicit discriminant and `#[serde(other)]` fallback | No |

## Tracking Changes

### `STABILITY.md`

The file [`STABILITY.md`](https://github.com/maos/maos/blob/main/STABILITY.md) is **generated** from workspace state:

```bash
cargo run -p xtask -- stability-matrix
```

It publishes:
- The live ABI Stability Triple values
- The supported schema window
- A deprecation table (NFR-Maint-5: 2 minor releases of warning, then 1 major release to remove)
- The LTS policy (1-year security-only patches from v1.0)
- Substrate-self compliance scope

CI runs `stability-matrix --check` to verify the file matches workspace state.

### `BREAKING.md`

The file [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) is the **human-authored** change ledger. Every breaking change must land with:

- A dated `## YYYY-MM-DD — <title>` heading
- Prose describing the change
- A `**Migration:**` line with concrete adaptation steps

The `check-breaking-md` CI gate fails when an entry is missing its migration path.

### `xtask/abi-ratifications.toml`

Each manifest schema version bump is recorded as a `[[ratification]]` entry, providing an audit trail of when and why the version was bumped.

## Deprecation Timeline

Deprecated public surfaces follow NFR-Maint-5:

1. **2 minor releases** with `#[maos_attrs::deprecated_since(...)]` warnings
2. **1 major release** to remove

Deprecation warnings are observable at runtime via `Ctx::deprecation_warnings()` and in tests via the `spirit-test` SDK. The `stability-matrix --check` gate enforces that every deprecated surface has a matching `STABILITY.md` entry.

## Reference

- [ABI Constants](/abi/constants) — live values in code
- [v1 → v2 Migration](./v1-to-v2)
- [v2 → v3 Migration](./v2-to-v3)
