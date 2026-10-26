---
title: "ESkillQueue::InvalidStoredEntry"
sidebar_position: 0
description: "Corrupt persisted skill identity or payload"
slug: "/errors/ESkillQueue::InvalidStoredEntry"
---

# ESkillQueue::InvalidStoredEntry

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::InvalidStoredEntry` |
| **Severity** | infra |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A stored entry violates identity, payload, provenance or schema invariants.

## Cause

Examples include a SHA-256 mismatch, missing v2 author payload, mismatched document id/version, duplicate live entry id or invalid provenance.

## Resolution

Preserve the original file and restore validated data. Do not recompute hashes to legitimize altered content. Load and save refuse malformed entries; v1 label-only migration remains explicitly marked legacy.

## Source Location

crates/maos-skill/src/errors.rs
