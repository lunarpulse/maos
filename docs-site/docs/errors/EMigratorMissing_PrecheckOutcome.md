---
title: "EMigratorMissing_PrecheckOutcome"
sidebar_position: 0
description: "Error reference for EMigratorMissing_PrecheckOutcome"
slug: "/errors/EMigratorMissing_PrecheckOutcome"
---

# EMigratorMissing_PrecheckOutcome

| Field | Value |
|-------|-------|
| **Error Code** | `EMigratorMissing_PrecheckOutcome` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Precheck verdict: migrator missing for the hot-swap candidate

## Rust Path

`PrecheckOutcome::EMigratorMissing`

## Cause

The hot-swap precheck phase detected that no migrator is registered for the candidate swap. This is the precheck-phase equivalent of EMigratorMissing — it fires before the actual swap is attempted to provide an early failure signal.

## Resolution

Register the missing migrator for the predecessor→successor transition before re-running the precheck. This is identical to the EMigratorMissing remediation — implement and register a `StateMigrator` for the specific class+version pair.

## Source Location

```rust
// Emitted from: PrecheckOutcome::EMigratorMissing
// Crate: maos-domain
Err(PrecheckOutcome::EMigratorMissing)
```
