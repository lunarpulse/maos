---
title: "ESkillQueue::UnknownEntryPath"
sidebar_position: 0
description: "Unknown persisted entry label refusal"
slug: "/errors/ESkillQueue::UnknownEntryPath"
---

# ESkillQueue::UnknownEntryPath

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::UnknownEntryPath` |
| **Severity** | infra |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A stored entry path is not `package_shipped`, `author_self` or `revision_proposal`.

## Cause

The queue is corrupt or was produced by an incompatible implementation. V1 migration does not replace unknown labels with a package label.

## Resolution

Preserve the original queue for diagnosis and restore a valid backup or repair its provenance deliberately. Submit, decide and show fail closed; never replace the unreadable queue with an empty one.

## Source Location

crates/maos-skill/src/errors.rs
