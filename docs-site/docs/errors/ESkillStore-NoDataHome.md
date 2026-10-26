---
title: "ESkillStore::NoDataHome"
sidebar_position: 0
description: "Private skill data home unavailable"
slug: "/errors/ESkillStore::NoDataHome"
---

# ESkillStore::NoDataHome

| Field | Value |
|---|---|
| **Error Code** | `ESkillStore::NoDataHome` |
| **Severity** | infra |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The default durable skill queue cannot resolve a nonempty HOME, or authored admission lacks an explicit materialization directory.

## Cause

The process environment does not identify a private user data home. There is deliberately no `/tmp` fallback for persisted documents and telemetry evidence.

## Resolution

Run with the intended user's HOME configured. The queue lives at `$HOME/.local/share/maos/skills/queue.json`; admitted documents materialize under `$HOME/.maos/skills/`. Explicit test paths remain supported.

## Source Location

crates/maos-skill/src/store.rs
