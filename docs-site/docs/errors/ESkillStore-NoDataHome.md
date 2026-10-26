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

The durable queue cannot resolve a private `MAOS_HOME` or `HOME`, or authored admission lacks an explicit materialization directory.

## Cause

The process environment does not identify a private user data home. There is deliberately no `/tmp` fallback for persisted documents and telemetry evidence.

## Resolution

Configure an absolute `MAOS_HOME`, otherwise the intended user's `HOME`. The queue lives at `<maos_home>/skills/queue.json`; validated legacy `$HOME/.local/share/maos/skills/queue.json` is migrated once with a journal receipt. Admitted documents still materialize under `$HOME/.maos/skills/`. Boot warns and leaves authoring unavailable instead of blocking the daemon or offline erasure children. Explicit test paths remain supported.

## Source Location

crates/maos-skill/src/store.rs
