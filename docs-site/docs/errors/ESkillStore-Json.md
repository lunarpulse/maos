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

The durable payload queue could not be strictly serialized or deserialized.

## Rust Path

`ESkillStore::Json`

## Cause

Invalid JSON, missing required v2 identity/provenance/payload fields, unknown fields or incompatible types prevent deserialization. V1 label-only entries are accepted only through the explicit versioned migration.

## Triggers

- queue.json is malformed, omits required QueueFile fields, or contains data with an incompatible type.

## Resolution

Preserve the failing queue.json and inspect the underlying JSON error. Restore validated payload data or repair it deliberately before submitting or deciding. Discovery and the transparency log cannot reconstruct lost skill documents, diffs or telemetry evidence, so never rebuild an empty queue over a corrupt payload store.

## Source Location

crates/maos-skill/src/store.rs
