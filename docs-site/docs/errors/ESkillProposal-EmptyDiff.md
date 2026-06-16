---
title: "ESkillProposal::EmptyDiff"
sidebar_position: 0
description: "Error reference for ESkillProposal::EmptyDiff"
slug: "/errors/ESkillProposal::EmptyDiff"
---

# ESkillProposal::EmptyDiff

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillProposal::EmptyDiff` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Revision proposal proposed_diff must be non-empty (FR57)

## Rust Path

`ESkillProposal::EmptyDiff`

## Cause

A skill revision proposal was submitted with an empty `proposed_diff` field. Per FR57, every revision proposal must contain a non-empty diff describing what changed.

## Resolution

Populate the `proposed_diff` field with the actual changes before submitting the revision proposal. The diff should describe the content modifications between the current skill version and the proposed revision.

## Source Location

```rust
// Emitted from: ESkillProposal::EmptyDiff
// Crate: maos-skill
Err(ESkillProposal::EmptyDiff)
```
