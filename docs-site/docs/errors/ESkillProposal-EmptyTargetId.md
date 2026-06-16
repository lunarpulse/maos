---
title: "ESkillProposal::EmptyTargetId"
sidebar_position: 0
description: "Error reference for ESkillProposal::EmptyTargetId"
slug: "/errors/ESkillProposal::EmptyTargetId"
---

# ESkillProposal::EmptyTargetId

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillProposal::EmptyTargetId` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Revision proposal target_skill_id must be non-empty

## Rust Path

`ESkillProposal::EmptyTargetId`

## Cause

A skill revision proposal was submitted with an empty `target_skill_id` field. The proposal must identify which skill it targets.

## Resolution

Set the `target_skill_id` field to the non-empty identifier of the skill being revised. Use `maos skill list` to find valid skill IDs in the registry.

## Source Location

```rust
// Emitted from: ESkillProposal::EmptyTargetId
// Crate: maos-skill
Err(ESkillProposal::EmptyTargetId)
```
