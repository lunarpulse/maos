---
title: "ESkillStore::Json"
sidebar_position: 0
description: "Error reference for ESkillStore::Json"
slug: "/errors/ESkillStore::Json"
---

# ESkillStore::Json

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillStore::Json` |
| **Severity** | infra |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The durable skill-queue cache (queue.json) is not valid JSON / failed to (de)serialize

## Rust Path

`ESkillStore::Json`

## Cause

The skill-queue store could not deserialize queue.json into QueueFile or serialize the queue entries. Invalid JSON, missing required fields, or values incompatible with the persisted queue schema cause deserialization to fail.

## Triggers

- queue.json is malformed, omits required QueueFile fields, or contains data with an incompatible type.

## Resolution

Inspect the underlying JSON error and preserve the failing queue.json for diagnosis. Use maosctl skills list to rebuild the admission view from discovery and the transparency log instead of inventing queue entries, and repair or restore a valid cache before persisting changes.

## Source Location

crates/maos-skill/src/store.rs
