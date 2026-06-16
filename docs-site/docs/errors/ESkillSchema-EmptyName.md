---
title: "ESkillSchema::EmptyName"
sidebar_position: 0
description: "Error reference for ESkillSchema::EmptyName"
slug: "/errors/ESkillSchema::EmptyName"
---

# ESkillSchema::EmptyName

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::EmptyName` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Skill name must be non-empty

## Rust Path

`ESkillSchema::EmptyName`

## Cause

The skill's TOML frontmatter has a `name` field that is empty or contains only whitespace. Every skill must have a human-readable display name.

## Resolution

Set the `name` field in the TOML frontmatter to a non-empty human-readable string. Example: `name = "My Skill Name"`. The name is used for display purposes in the skill registry.

## Source Location

```rust
// Emitted from: ESkillSchema::EmptyName
// Crate: maos-skill
Err(ESkillSchema::EmptyName)
```
