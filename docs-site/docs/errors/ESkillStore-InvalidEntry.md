---
title: "ESkillStore::InvalidEntry"
sidebar_position: 0
description: "Stored or submitted entry validation refusal"
slug: "/errors/ESkillStore::InvalidEntry"
---

# ESkillStore::InvalidEntry

| Field | Value |
|---|---|
| **Error Code** | `ESkillStore::InvalidEntry` |
| **Severity** | infra |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The durable admission API rejected a submitted or stored entry with a typed `ESkillQueue` cause.

## Cause

Examples include author namespace/ownership failure, principal data, capability escalation, malformed document/proposal or corrupt persisted metadata.

## Resolution

Handle the wrapped queue variant without replacing it with a string fallback. On load failure preserve the original queue; submit, decide and show must not proceed with an invented empty queue. Journal a non-personal refusal receipt at the root-fixed author pid for live submission failures.

## Source Location

crates/maos-skill/src/store.rs
