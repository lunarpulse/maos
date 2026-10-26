---
title: "ESkillQueue::AlreadyDecided"
sidebar_position: 0
description: "Terminal skill admission decision"
slug: "/errors/ESkillQueue::AlreadyDecided"
---

# ESkillQueue::AlreadyDecided

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::AlreadyDecided` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The selected entry is no longer Pending.

## Cause

An operator already admitted or rejected it, possibly in a different invocation.

## Resolution

Show the selected entry and its journal decision. Author a new version or submit a new revision rather than rewriting a terminal decision. The caller must reconcile approval rows before preparing decisions.

## Source Location

crates/maos-skill/src/errors.rs
