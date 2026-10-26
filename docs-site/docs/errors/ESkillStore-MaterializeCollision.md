---
title: "ESkillStore::MaterializeCollision"
sidebar_position: 0
description: "Foreign skill file collision"
slug: "/errors/ESkillStore::MaterializeCollision"
---

# ESkillStore::MaterializeCollision

| Field | Value |
|---|---|
| **Error Code** | `ESkillStore::MaterializeCollision` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Admitting an AuthorSelf document would overwrite a file not recognized as an admitted document owned by the same author class.

## Cause

`$HOME/.maos/skills/<id>.md` already exists with foreign or modified content. Byte equality to an unadmitted draft alone does not establish ownership.

## Resolution

Preserve and inspect the existing file. Resolve the collision explicitly before retrying reconciliation; never overwrite it merely because the operator admitted a queued document. Known admitted owned bytes may be replaced atomically by a newer admitted document. Revision approvals never materialize content.

## Source Location

crates/maos-skill/src/store.rs
