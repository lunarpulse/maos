---
title: "ESkillQueue::DuplicateSkillId"
sidebar_position: 0
description: "Error reference for ESkillQueue::DuplicateSkillId"
slug: "/errors/ESkillQueue::DuplicateSkillId"
---

# ESkillQueue::DuplicateSkillId

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillQueue::DuplicateSkillId` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A Pending entry with this SkillId already exists in the admission queue

## Rust Path

`ESkillQueue::DuplicateSkillId`

## Cause

A skill with the same `SkillId` already exists in the Pending state within the admission queue. The queue enforces uniqueness to prevent duplicate processing.

## Resolution

Wait for the existing pending entry to be processed (admitted or rejected) before re-submitting. If the pending entry is stale, cancel it first with `maos skill cancel <skill_id>`, then re-submit.

## Source Location

```rust
// Emitted from: ESkillQueue::DuplicateSkillId
// Crate: maos-skill
Err(ESkillQueue::DuplicateSkillId)
```
