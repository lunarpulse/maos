---
title: "ESkillQueue::PrincipalDataRefused"
sidebar_position: 0
description: "Indexed principal payload refusal"
slug: "/errors/ESkillQueue::PrincipalDataRefused"
---

# ESkillQueue::PrincipalDataRefused

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::PrincipalDataRefused` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The document or opaque diff contains a token-bounded principal id indexed to the root-fixed author pid.

## Cause

The skill queue is outside forget's erasure reach, so known indexed principals must not enter its payload store.

## Resolution

Remove personal identifiers before resubmitting. The refusal and its receipt must never echo the matched principal. This check is not content-blind privacy detection; unindexed personal data remains a stated limitation.

## Source Location

crates/maos-skill/src/errors.rs
