---
title: "ESkillSchema::MissingFence"
sidebar_position: 0
description: "Error reference for ESkillSchema::MissingFence"
slug: "/errors/ESkillSchema::MissingFence"
---

# ESkillSchema::MissingFence

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillSchema::MissingFence` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

maos.skill.v1 document missing the --- TOML frontmatter fence

## Rust Path

`ESkillSchema::MissingFence`

## Cause

The skill document is missing the `---` TOML frontmatter fence. The `maos.skill.v1` format requires a fenced TOML block at the top of the document.

## Resolution

Add the TOML frontmatter fence at the top of the skill document. The document must start with `---` followed by TOML key-value pairs and end with `---` before the markdown body. Example:
```
---
id = "my-skill"
name = "My Skill"
version = "1.0.0"
---
Skill body here.
```

## Source Location

```rust
// Emitted from: ESkillSchema::MissingFence
// Crate: maos-skill
Err(ESkillSchema::MissingFence)
```
