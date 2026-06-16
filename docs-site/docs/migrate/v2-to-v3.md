---
title: "Migrate v2 → v3"
sidebar_position: 3
description: Step-by-step guide to migrate a Spirit manifest from schema version 2 to version 3.
---

# Migrate Manifest v2 → v3

Manifest schema version 3 was introduced in Story 9.4b AC-6 (2026-06-15) to add the `[model_provenance]` section.

## What Changed

Version 3 adds a single new section:

| Addition | Story | Manifest Section | Purpose |
|---|---|---|---|
| Model provenance | 9.4b | `[model_provenance]` | Track model identity and training data lineage with `covered_model_id`, `training_data_lineage` (reverse-DNS-constrained), and `last_eval_timestamp` |

The section is **OPTIONAL on read** — `from_manifest_toml` returns `None` when absent. This means v2 manifests load under v3 kernels without modification (N-1 supported floor).

## Kernel Behavior

| Kernel version | v2 manifest behavior |
|---|---|
| Schema v3 kernel | ✅ Loads with WARN-level degradation notes (N-1 supported) |
| Future schema v4 kernel | ✅ Expected to remain in window (N-1) |
| Future schema v5 kernel | ⛔ Hard refusal expected at N-2 boundary |

## Migration Steps

### Step 1: Bump the Schema Version

In your manifest's `[class]` section, update the schema version:

```toml
[class]
name = "my-spirit"
manifest_schema_version = 3   # was 2
min_substrate_version = "0.1.0-alpha"
```

### Step 2: (Optional) Add Model Provenance

If your Spirit wraps or invokes an ML model, declare its provenance:

```toml
[model_provenance]
covered_model_id = "com.example.my-model-v2"
training_data_lineage = "com.example.dataset.curated-2026q2"
last_eval_timestamp = "2026-06-01T00:00:00Z"
```

**Constraints:**
- `covered_model_id` — identifies the model this Spirit wraps; reverse-DNS format recommended.
- `training_data_lineage` — reverse-DNS-constrained identifier (NOT free-text). This is enforced at validation.
- `last_eval_timestamp` — ISO 8601 timestamp of the last evaluation run.

### Step 3: Validate

Load the manifest against a v3 kernel. The kernel validates with `deny_unknown_fields`, so any schema errors surface at admission time.

## Rollback

The `[model_provenance]` section is optional. To revert:
1. Remove the `[model_provenance]` section.
2. Set `manifest_schema_version = 2` in `[class]`.

The manifest will load on any kernel whose supported window includes version 2.

## Ratification

The version 3 bump is recorded as a ratified entry in `xtask/abi-ratifications.toml`, following the ABI Stability Triple process from Story 7.5a.

## Reference

- [ABI Stability Policy](./abi-stability) — N-1/N-2 rules and the ABI Stability Triple
- [ABI Constants](/abi/constants) — live values for `MANIFEST_SCHEMA_VERSION` and supported window
- [`BREAKING.md`](https://github.com/maos/maos/blob/main/BREAKING.md) — CI-enforced change ledger
