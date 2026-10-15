---
title: "ESkillStore::UnknownSchemaVersion"
sidebar_position: 0
description: "Error reference for ESkillStore::UnknownSchemaVersion"
slug: "/errors/ESkillStore::UnknownSchemaVersion"
---

# ESkillStore::UnknownSchemaVersion

| Field | Value |
|-------|-------|
| **Error Code** | `ESkillStore::UnknownSchemaVersion` |
| **Severity** | user |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

queue.json carries an unknown/absent schema_version (hard error — no best-effort parse; the version stamp is a tripwire)

## Rust Path

`ESkillStore::UnknownSchemaVersion`

## Cause

queue.json parsed as a QueueFile, but its schema_version differs from maos.skill-queue.v1. The store refuses the unknown version instead of interpreting its entries as the current schema.

## Triggers

- A structurally valid QueueFile declares a schema_version other than maos.skill-queue.v1. A missing version instead fails JSON deserialization.

## Resolution

Check which MAOS version wrote queue.json and use compatible software. Preserve the old file for diagnosis; maosctl skills list can rebuild its view from discovery and the transparency log. Do not merely relabel an unknown schema as maos.skill-queue.v1.

## Source Location

crates/maos-skill/src/store.rs
