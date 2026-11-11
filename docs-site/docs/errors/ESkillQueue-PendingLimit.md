---
title: "ESkillQueue::PendingLimit"
sidebar_position: 0
description: "Skill author pending submission limit"
slug: "/errors/ESkillQueue--PendingLimit"
---

# ESkillQueue::PendingLimit

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::PendingLimit` |
| **Severity** | policy |
| **Recovery Class** | retry |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | Yes, after pending decisions |
| **Kernel / Spirit** | kernel |

## Summary

The author class already has 64 pending skill submissions.

## Cause

The pending queue bounds outstanding submissions per author class.

## Resolution

Have an operator decide pending submissions before submitting another.

## Source Location

crates/maos-skill/src/errors.rs
