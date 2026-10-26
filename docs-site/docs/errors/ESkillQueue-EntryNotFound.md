---
title: "ESkillQueue::EntryNotFound"
sidebar_position: 0
description: "Unknown queue selector"
slug: "/errors/ESkillQueue::EntryNotFound"
---

# ESkillQueue::EntryNotFound

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::EntryNotFound` |
| **Severity** | user |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

No queued entry matches the requested entry id or skill id.

## Cause

The selector is mistyped or names an entry absent from this data home.

## Resolution

List the queue and use a returned entry id. No decision or payload mutation occurs.

## Source Location

crates/maos-skill/src/errors.rs
