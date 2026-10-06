---
title: "Migrate v4 → v5"
sidebar_position: 5
description: Migrate a Spirit manifest from schema v4 to v5.
---

# Migrate Manifest v4 → v5

Schema v5 introduces the `wasm-component` form and its `artifact` declaration.

## Steps

A v4 `rust-inproc` or `subprocess` Spirit remains valid and may stay at schema v4 while it is within the supported compatibility window. Only a component author needs these changes:

1. Change `[class].manifest_schema_version` from `4` to `5`.
2. Set `forms = ["wasm-component"]`; it cannot be combined with another form.
3. Add an `artifact` path relative to the manifest, ending in `.wasm` and containing no `..` segment.
4. Declare `[sandbox] tier = "T2"`.

```toml
[class]
manifest_schema_version = 5
forms = ["wasm-component"]
artifact = "dist/spirit.wasm"

[sandbox]
tier = "T2"
```

## Compatibility and rollback

A schema v5 kernel continues to load v4 manifests. To roll a component declaration back to an in-process Spirit, remove `artifact`, set `forms = ["rust-inproc"]`, set the sandbox tier to `T0`, and then set the schema version to `4` when targeting a v4 kernel.

The default MAOS binary validates a WASM-component Spirit but reports `wasm_engine_off` while its `wasm-host` engine remains disabled for Export Hold 2. A `wasm-host` build reports `wasm_launch_not_built` until 17-3c implements launch.

## Ratification

The v4→v5 schema change is ratified as `17-3b-AC3-manifest-schema-5` in `xtask/abi-ratifications.toml` and recorded in the ABI stability ledger.
