---
title: constants
sidebar_position: 9
description: "ABI_VERSION, MANIFEST_SCHEMA_VERSION, and supported window constants."
---

# Constants

The `maos-spirit-abi` crate root exports four constants that form the mechanical core of the [ABI Stability Triple](/migrate/abi-stability). These are the single source of truth consumed by admission checks, CI gates, and the stability matrix.

## ABI_VERSION

```rust
pub const ABI_VERSION: u32 = 1;
```

Wire-format version of the Spirit ABI. Bumped according to the §8.5 ABI-break rules. Frozen at `1` by Story 1b.4 at the ComplianceClaim envelope freeze.

This constant changes only when a **breaking** wire-format change occurs (field removal, field rename, type change, enum variant removal). Additive changes (optional fields, new enum variants with `#[serde(other)]`) do NOT bump this.

### Example

```rust
use maos_spirit_abi::ABI_VERSION;

assert_eq!(ABI_VERSION, 1);
// Use in version negotiation or logging
println!("Spirit ABI version: {ABI_VERSION}");
```

## MANIFEST_SCHEMA_VERSION

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 3;
```

Schema version currently emitted by the kernel for Spirit manifest TOML files. This is the version a newly-authored manifest should declare in its `[class]` section.

### Version History

| Version | Introduced | Changes |
|---|---|---|
| `1` | Epic 1b | Baseline manifest schema |
| `2` | Epic 6 (2026-05-28) | `[[cli_wrapper]]`, `[[schedules]]`, `[gateways]`/`[[gateway]]`, consent envelope extensions |
| `3` | Story 9.4b (2026-06-15) | `[model_provenance]` section |

### Example

```rust
use maos_spirit_abi::MANIFEST_SCHEMA_VERSION;

assert_eq!(MANIFEST_SCHEMA_VERSION, 3);
// Reference in manifest generation
println!("manifest_schema_version = {MANIFEST_SCHEMA_VERSION}");
```

## MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION

```rust
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;
```

Lowest manifest schema version the kernel accepts at admission. Manifests below this floor are refused with `SecurityError::EAbiTooOld` (N-2 hard refusal).

The floor is lifted on each ABI bump per the N-1 supported / N-2 hard-refusal policy.

### Example

```rust
use maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;

fn check_manifest_version(declared: u32) -> bool {
    declared >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
}

assert!(check_manifest_version(1));
assert!(check_manifest_version(3));
assert!(!check_manifest_version(0));
```

## MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION

```rust
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = MANIFEST_SCHEMA_VERSION; // 3
```

Highest manifest schema version the kernel accepts. Manifests above this ceiling are refused with `SecurityError::EAbiTooNew` (fail-closed; the operator is told a newer kernel is required).

Currently equal to `MANIFEST_SCHEMA_VERSION`. The two constants stay synonymous until an explicit N+1 acceptance window is introduced for forward-compatibility experiments.

### Example

```rust
use maos_spirit_abi::{
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
    MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
};

fn is_version_supported(v: u32) -> bool {
    v >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
        && v <= MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION
}

assert!(is_version_supported(1));  // N-1 — supported
assert!(is_version_supported(2));  // N-1 — supported
assert!(is_version_supported(3));  // Current — supported
assert!(!is_version_supported(4)); // Future — EAbiTooNew
assert!(!is_version_supported(0)); // Below floor — EAbiTooOld
```

## Relationship to the ABI Stability Triple

These constants compose the [ABI Stability Triple](/migrate/abi-stability):

```
(kernel_version, ABI_VERSION, MANIFEST_SCHEMA_VERSION)
= ("0.1.0-alpha", 1, 3)
```

The `kernel_version` comes from the workspace `Cargo.toml`, not from this crate. Together, the triple is the load-time compatibility contract enforced at Spirit admission.

## CI Gates

- **`stability-matrix --check`** verifies that `STABILITY.md` matches these constants.
- **`abi-diff --deny removed --deny changed`** guards `ABI_VERSION` against unintended breaks.
- **`check-manifest-schema-version`** validates that manifest `[class]` sections declare a version within the supported window.

## Reference

- [ABI Stability Policy](/migrate/abi-stability) — full compatibility window documentation
- [v1 → v2 Migration](/migrate/v1-to-v2) — what changed at version 2
- [v2 → v3 Migration](/migrate/v2-to-v3) — what changed at version 3
