---
title: "ESkillSchema::InvalidSemver"
sidebar_position: 0
description: "Error reference for ESkillSchema::InvalidSemver"
slug: "/errors/ESkillSchema::InvalidSemver"
---

# ESkillSchema::InvalidSemver

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::InvalidSemver` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Skill version is not a valid semver string

## Rust Path

`ESkillSchema::InvalidSemver`

## Cause

The skill's `version` field in the TOML frontmatter is not a valid semver string. The kernel strictly validates version strings at parse time.

## Resolution

Set the `version` field to a valid semver string: `MAJOR.MINOR.PATCH` (e.g. `version = "1.0.0"`). Partial versions like `1.0` or non-numeric versions like `v1` are not accepted.

## Source Location

```rust
// Emitted from: ESkillSchema::InvalidSemver
// Crate: maos-skill
Err(ESkillSchema::InvalidSemver)
```
