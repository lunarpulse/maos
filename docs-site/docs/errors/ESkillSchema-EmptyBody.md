---
title: "ESkillSchema::EmptyBody"
sidebar_position: 0
description: "Error reference for ESkillSchema::EmptyBody"
slug: "/errors/ESkillSchema::EmptyBody"
---

# ESkillSchema::EmptyBody

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::EmptyBody` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Skill body must be present (non-empty markdown after frontmatter fence)

## Rust Path

`ESkillSchema::EmptyBody`

## Cause

The skill document has no content after the TOML frontmatter fence. A skill must contain non-empty markdown body text that describes its behavior.

## Resolution

Add markdown content after the closing `---` fence of the TOML frontmatter. The body should describe the skill's purpose, triggers, and behavior. Even a minimal description is required.

## Source Location

```rust
// Emitted from: ESkillSchema::EmptyBody
// Crate: maos-skill
Err(ESkillSchema::EmptyBody)
```
