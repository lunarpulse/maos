---
title: "ESkillStore::Io"
sidebar_position: 0
description: "Error reference for ESkillStore::Io"
slug: "/errors/ESkillStore::Io"
---

# ESkillStore::Io

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillStore::Io` |
| **Severity** | infra |
| **Recovery Class** | retry |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | Yes |
| **Kernel / Spirit** | kernel |

## Summary

Filesystem I/O failure reading or writing the durable skill-queue cache (queue.json)

## Rust Path

`ESkillStore::Io`

## Cause

The skill-queue store encountered a filesystem error reading queue.json or creating, writing, syncing, or renaming its atomic replacement. A nonexistent queue file alone is treated as an empty queue, not this error.

## Triggers

- Reading queue.json fails, or the atomic write fails while creating, writing, syncing, renaming, or syncing the parent directory.

## Resolution

Inspect the underlying I/O error and `<maos_home>/skills/queue.json` (`MAOS_HOME`, otherwise `$HOME/.maos`). Restore directory access, writable storage, and available space as appropriate; preserve existing queues and journal the one-time validated migration from the former data-home path.

## Source Location

crates/maos-skill/src/store.rs
