---
title: "Migrate v3 → v4"
sidebar_position: 4
description: Migrate a Spirit manifest from schema v3 to v4.
---

# Migrate Manifest v3 → v4

Schema v4 adds optional declared collective capabilities under `[capabilities.required.loom]`.

## Steps

1. Change `[class].manifest_schema_version` from `3` to `4`.
2. Add only the Loom operations the Spirit needs:

```toml
[capabilities.required.loom]
read = true
write = true
scan = false
```

Each `true` value is converted at admission to its corresponding `Loom*` scope. A declaration does not bypass capability mediation, enterprise policy, token expiry, or tenant-map routing.

## Compatibility and rollback

The section defaults to all `false`; v3 manifests remain accepted in the supported window. To roll back a v4 manifest, remove the Loom section and set the class version to `3`, provided the destination kernel supports v3.

## Ratification

The v3→v4 schema change is ratified in `xtask/abi-ratifications.toml` and recorded in the ABI stability ledger.
