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

The queue declares neither `maos.skill-queue.v2` nor the supported v1 migration format, or has no version. The store refuses it instead of interpreting its entries as the current schema.

## Triggers

- `schema_version` is missing or does not identify v1 or v2.

## Resolution

Check which MAOS version wrote queue.json and use compatible software. Preserve the original payload file and restore a valid backup if necessary. Do not relabel an unknown schema as v2, discard payloads or rebuild an empty queue over an unreadable file. V1 files migrate explicitly and save as v2.

## Source Location

crates/maos-skill/src/store.rs
