---
title: "ESkillQueue::Schema"
sidebar_position: 0
description: "Typed skill document validation failure"
slug: "/errors/ESkillQueue::Schema"
---

# ESkillQueue::Schema

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::Schema` |
| **Severity** | user |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A live or persisted authored document failed strict `maos.skill.v1` validation.

## Cause

The wrapped `ESkillSchema` identifies malformed TOML, unknown fields, invalid id/version or a missing body/fence.

## Resolution

Correct the document according to the wrapped schema error before resubmitting. A persisted invalid document must be restored from validated data; never silently discard it or overwrite its store.

## Source Location

crates/maos-skill/src/errors.rs
