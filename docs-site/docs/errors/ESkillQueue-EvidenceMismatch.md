---
title: "ESkillQueue::EvidenceMismatch"
sidebar_position: 0
description: "Root-fixed telemetry pid requirement"
slug: "/errors/ESkillQueue::EvidenceMismatch"
---

# ESkillQueue::EvidenceMismatch

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::EvidenceMismatch` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The telemetry report's pid differs from the author pid fixed by the root.

## Cause

The root assembled mismatched provenance and telemetry. The Spirit-facing port accepts no evidence or pid argument.

## Resolution

Obtain evidence from the root aggregator for the bound pid since its bind instant. Do not coerce the report pid or accept caller-supplied telemetry. Nothing is enqueued.

## Source Location

crates/maos-skill/src/errors.rs
