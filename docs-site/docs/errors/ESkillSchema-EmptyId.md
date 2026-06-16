---
title: "ESkillSchema::EmptyId"
sidebar_position: 0
description: "Error reference for ESkillSchema::EmptyId"
slug: "/errors/ESkillSchema::EmptyId"
---

# ESkillSchema::EmptyId

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::EmptyId` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Skill id must be non-empty

## Rust Path

`ESkillSchema::EmptyId`

## Cause

The skill's TOML frontmatter has an `id` field that is empty or contains only whitespace. Every skill must have a non-empty identifier.

## Resolution

Set the `id` field in the TOML frontmatter to a non-empty value. Example: `id = "my-skill-name"`. The ID must also conform to charset rules (lowercase alphanumeric, hyphens, dots).

## Source Location

```rust
// Emitted from: ESkillSchema::EmptyId
// Crate: maos-skill
Err(ESkillSchema::EmptyId)
```
