---
title: "ESkillQueue::CapabilityEscalation"
sidebar_position: 0
description: "Skill capability subset requirement"
slug: "/errors/ESkillQueue::CapabilityEscalation"
---

# ESkillQueue::CapabilityEscalation

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::CapabilityEscalation` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A document requires an action absent from the author's declared scope action keys.

## Cause

At least one `required_capabilities` entry is not an exact member of the root-supplied declared action set.

## Resolution

Remove undeclared requirements or have the operator change and re-admit the Spirit manifest through normal admission. A skill cannot grant authority. Nothing is enqueued.

## Source Location

crates/maos-skill/src/errors.rs
