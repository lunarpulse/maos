---
title: "ESkillProposal::InvalidTargetIdCharset"
sidebar_position: 0
description: "Error reference for ESkillProposal::InvalidTargetIdCharset"
slug: "/errors/ESkillProposal::InvalidTargetIdCharset"
---

# ESkillProposal::InvalidTargetIdCharset

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillProposal::InvalidTargetIdCharset` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Revision proposal target_skill_id contains invalid characters

## Rust Path

`ESkillProposal::InvalidTargetIdCharset`

## Cause

The `target_skill_id` in the revision proposal contains characters outside the allowed set. Skill IDs must conform to the charset rules (typically lowercase alphanumeric, hyphens, and dots).

## Resolution

Correct the `target_skill_id` to use only allowed characters: lowercase letters (a-z), digits (0-9), hyphens (-), and dots (.). Remove or replace any uppercase letters, spaces, underscores, or special characters.

## Source Location

```rust
// Emitted from: ESkillProposal::InvalidTargetIdCharset
// Crate: maos-skill
Err(ESkillProposal::InvalidTargetIdCharset)
```
