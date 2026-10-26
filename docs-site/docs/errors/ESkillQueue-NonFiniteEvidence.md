---
title: "ESkillQueue::NonFiniteEvidence"
sidebar_position: 0
description: "Finite durable revision evidence"
slug: "/errors/ESkillQueue::NonFiniteEvidence"
---

# ESkillQueue::NonFiniteEvidence

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::NonFiniteEvidence` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A revision contains a non-finite halt value or threshold. JSON would serialize it as null and make the persisted evidence unreadable.

## Cause

The revision's telemetry evidence carries an infinite or NaN `f32` halt value or threshold, for example a scalar too large to narrow from `f64`.

## Resolution

Correct the telemetry producer and submit finite measurements. The root refuses before journaling the enqueue or changing the queue; never replace measurements with invented finite values.

## Source Location

crates/maos-skill/src/errors.rs
