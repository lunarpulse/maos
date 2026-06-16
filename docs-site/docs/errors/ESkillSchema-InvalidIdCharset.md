---
title: "ESkillSchema::InvalidIdCharset"
sidebar_position: 0
description: "Error reference for ESkillSchema::InvalidIdCharset"
slug: "/errors/ESkillSchema::InvalidIdCharset"
---

# ESkillSchema::InvalidIdCharset

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::InvalidIdCharset` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Skill id contains invalid characters (allowed: a-z, 0-9, '-', '.')

## Rust Path

`ESkillSchema::InvalidIdCharset`

## Cause

The skill's `id` field contains characters outside the allowed set. Only lowercase letters (a-z), digits (0-9), hyphens (-), and dots (.) are permitted.

## Resolution

Rename the skill ID to use only allowed characters. Replace uppercase letters with lowercase, spaces/underscores with hyphens. Example: `My_Skill` → `my-skill`.

## Source Location

```rust
// Emitted from: ESkillSchema::InvalidIdCharset
// Crate: maos-skill
Err(ESkillSchema::InvalidIdCharset)
```
