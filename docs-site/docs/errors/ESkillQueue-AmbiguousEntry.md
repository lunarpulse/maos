---
title: "ESkillQueue::AmbiguousEntry"
sidebar_position: 0
description: "Entry id required for shared skill id"
slug: "/errors/ESkillQueue::AmbiguousEntry"
---

# ESkillQueue::AmbiguousEntry

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::AmbiguousEntry` |
| **Severity** | user |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Multiple queued entries share the requested skill id.

## Cause

A base document and revisions, or several versions/proposals, coexist. Selecting the first match could decide the wrong payload.

## Resolution

Use the exact entry id printed by the queue list. Revisions have separate approval targets ending in `#<entry_id>`; deciding one never decides its base or sibling.

## Source Location

crates/maos-skill/src/errors.rs
