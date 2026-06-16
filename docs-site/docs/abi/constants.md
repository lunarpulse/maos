<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `constants` Module

## Related

- [ABI Stability Policy](/migrate/abi-stability) — full compatibility window documentation
- [v1 → v2 Migration](/migrate/v1-to-v2) — what changed at manifest schema version 2
- [v2 → v3 Migration](/migrate/v2-to-v3) — what changed at manifest schema version 3


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 3*


## Constants

ABI version constant for the MAOS Spirit ABI.

Bumped according to the ABI Stability Triple rules (§8.5).

**Story 1b.4 froze this at `1`** at the ComplianceClaim envelope freeze.

# Example

```rust
use maos_spirit_abi::ABI_VERSION;

assert_eq!(ABI_VERSION, 1);
```

```rust
pub const ABI_VERSION: u32 = 1u32;
```

Manifest schema version currently emitted by the kernel.

Bumped to `2` in Epic 6 §A4 (retro 2026-05-28) to track the four additive
sections landed across Epic 6 stories 6.2 / 6.4 / 6.5:

- `[[cli_wrapper]]` (Story 6.2 — `command`, `output_shape_version`,
  `recovery_policy`, `posture`, `shutdown_signal`).
- `[[schedules]]` (Story 6.4 — `id`, `cadence`, `rate_limit_per_hour`,
  `compliance_claim_ref_hex`, `side_effect_scopes`, `payload_b64`).
- `[gateways]` / `[[gateway]]` (Story 6.5 — `id`, `type`, `auth_secret_ref`,
  `inbound_routing`, gateway-specific config blocks).
- `ConsentEnvelope.intent_class` + `ConsentEnvelope.valid_until_ns`
  (Story 6.4 — additive on the consent envelope shape).

All four additions are wire-compatible at the TOML/serde layer
(`#[serde(default)]` + `#[serde(deny_unknown_fields)]`), so kernels at
`MANIFEST_SCHEMA_VERSION = 2` accept manifests authored against `= 1`
(the N-1 supported floor enforced by `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`).

Bumped to `3` in Story 9.4b AC-6 (2026-06-15) to track the additive
`[model_provenance]` section (`covered_model_id`, `training_data_lineage`
— reverse-DNS-constrained, NOT free-text — `last_eval_timestamp`). The
section is wire-compatible at the TOML/serde layer: it is OPTIONAL on read
(`from_manifest_toml` returns `None` when absent), so kernels at
`MANIFEST_SCHEMA_VERSION = 3` still admit manifests authored at `= 2`
(the N-1 supported floor) — AC-11 append-only compat. Recorded as one
ratified `[[ratification]]` entry in `xtask/abi-ratifications.toml`.

This constant is the single authoritative source consumed by
`maos-manifest::ClassSection` validation and by the `xtask
check-manifest-schema-version` gate. Story 7.5a's ABI Stability Triple
`(kernel_version, abi_version, manifest_schema_version)` consumes this
constant directly.

# Example

```rust
use maos_spirit_abi::MANIFEST_SCHEMA_VERSION;

assert_eq!(MANIFEST_SCHEMA_VERSION, 3);
```

```rust
pub const MANIFEST_SCHEMA_VERSION: u32 = 3u32;
```

Lowest manifest schema version this kernel accepts at admission.

Story 7.5a will lift this floor on each ABI bump per the N-1 supported /
N-2 hard-refusal policy. At v0.5-α the floor remains at `1` — Epic 1b
baseline manifests load unchanged.

# Example

```rust
use maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;

fn check_manifest_version(declared: u32) -> bool {
    declared >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
}

assert!(check_manifest_version(1));
assert!(check_manifest_version(3));
assert!(!check_manifest_version(0));
```

```rust
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1u32;
```

Highest manifest schema version this kernel emits or accepts.

Currently equal to `MANIFEST_SCHEMA_VERSION`. The two constants stay
synonymous until Story 7.5a introduces an explicit N+1 acceptance window
for forward-compatibility experiments.

# Example

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

```rust
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 3u32;
```
