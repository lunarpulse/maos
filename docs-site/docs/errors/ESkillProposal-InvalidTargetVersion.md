---
title: "ESkillProposal::InvalidTargetVersion"
sidebar_position: 0
description: "Error reference for ESkillProposal::InvalidTargetVersion"
slug: "/errors/ESkillProposal::InvalidTargetVersion"
---

# ESkillProposal::InvalidTargetVersion

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillProposal::InvalidTargetVersion` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Revision proposal target_version is not valid semver

## Rust Path

`ESkillProposal::InvalidTargetVersion`

## Cause

The `target_version` field in the revision proposal is not a valid semver string (e.g. `1.2.3`). The kernel requires strict semver for all version references.

## Resolution

Fix the `target_version` to be a valid semver string (MAJOR.MINOR.PATCH, e.g. `1.0.0`). Pre-release and build metadata are allowed per semver spec (e.g. `1.0.0-beta.1`). Do not use partial versions like `1.0` or non-numeric segments.

## Source Location

```rust
// Emitted from: ESkillProposal::InvalidTargetVersion
// Crate: maos-skill
Err(ESkillProposal::InvalidTargetVersion)
```
