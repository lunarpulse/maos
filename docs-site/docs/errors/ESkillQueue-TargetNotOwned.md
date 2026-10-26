---
title: "ESkillQueue::TargetNotOwned"
sidebar_position: 0
description: "Revision ownership refusal"
slug: "/errors/ESkillQueue::TargetNotOwned"
---

# ESkillQueue::TargetNotOwned

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::TargetNotOwned` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A revision requires an admitted AuthorSelf entry whose recorded class matches the root-resolved author class.

## Cause

The target is missing, belongs to another class, is package-shipped or has not been admitted.

## Resolution

Select an admitted owned skill from `own_entries`. An operator must admit the original authored document first. Nothing is enqueued.

## Source Location

crates/maos-skill/src/errors.rs
