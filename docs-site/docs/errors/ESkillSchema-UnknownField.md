---
title: "ESkillSchema::UnknownField"
sidebar_position: 0
description: "Error reference for ESkillSchema::UnknownField"
slug: "/errors/ESkillSchema::UnknownField"
---

# ESkillSchema::UnknownField

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::UnknownField` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Unknown field in TOML frontmatter — deny_unknown_fields rejects it

## Rust Path

`ESkillSchema::UnknownField`

## Cause

The TOML frontmatter contains a field name that is not recognized by the skill schema. The schema uses `deny_unknown_fields` to reject any extraneous keys, preventing silent misconfiguration.

## Resolution

Remove the unrecognized field from the TOML frontmatter. Check the skill schema documentation for the list of valid fields. Common mistakes: typos in field names (e.g. `verson` instead of `version`), or fields from a newer schema version not yet supported.

## Source Location

```rust
// Emitted from: ESkillSchema::UnknownField
// Crate: maos-skill
Err(ESkillSchema::UnknownField)
```
