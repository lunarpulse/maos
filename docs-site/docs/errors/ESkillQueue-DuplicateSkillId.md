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

A conflicting pending submission or an already-recorded canonical entry identity exists.

## Rust Path

`ESkillQueue::DuplicateSkillId`

## Cause

New documents are bounded to one Pending entry per skill id. Revision proposals are bounded to one Pending entry per target and full author provenance (class, pid, boot nonce), so different live author instances may submit distinct revisions. Exact canonical entry ids cannot be replayed as new entries, even after a terminal decision.

## Resolution

Wait for the pending entry's operator decision before submitting a distinct payload. Use a new advancing document version or a new revision payload rather than replaying the same entry. Select exact entry ids when showing or deciding coexisting revisions; no cancel command is required or implied.

## Source Location

```rust
// Emitted from: ESkillQueue::DuplicateSkillId
// Crate: maos-skill
Err(ESkillQueue::DuplicateSkillId)
```
