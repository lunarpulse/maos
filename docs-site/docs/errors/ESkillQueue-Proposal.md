---
title: "ESkillQueue::Proposal"
sidebar_position: 0
description: "Typed revision proposal validation failure"
slug: "/errors/ESkillQueue::Proposal"
---

# ESkillQueue::Proposal

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::Proposal` |
| **Severity** | user |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A live or persisted revision failed FR57 proposal validation.

## Cause

The wrapped `ESkillProposal` identifies invalid target id/version or an empty diff.

## Resolution

Correct the proposal before resubmitting. Evidence remains root-attached and diffs remain opaque: admission records a decision but does not execute or apply a diff.

## Source Location

crates/maos-skill/src/errors.rs
