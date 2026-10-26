---
title: "ESkillQueue::AuthoredIdTooLong"
sidebar_position: 0
description: "Authored identifier length limit"
slug: "/errors/ESkillQueue::AuthoredIdTooLong"
---

# ESkillQueue::AuthoredIdTooLong

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::AuthoredIdTooLong` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

An authored skill id exceeds 128 ASCII bytes, so its approval identity cannot safely traverse the operator door and materialization path.

## Cause

The Spirit-authored document declares an id longer than the operator door's 128-byte identifier boundary; the materialized `<id>.md` name would also approach the filesystem name limit.

## Resolution

Choose an id in the root-assigned namespace with at most 128 bytes. Do not truncate a previously submitted identity.

## Source Location

crates/maos-skill/src/errors.rs
