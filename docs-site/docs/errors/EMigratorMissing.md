---
title: "EMigratorMissing"
sidebar_position: 0
description: "Error reference for EMigratorMissing"
slug: "/errors/EMigratorMissing"
---

# EMigratorMissing

| Field | Value |
|-------|-------|
| **Error Code** | `EMigratorMissing` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

No state-transfer migrator registered for predecessor→successor class+version pair

## Rust Path

`HotSwapError::EMigratorMissing`

## Cause

A hot-swap was attempted between a predecessor and successor spirit, but no state-transfer migrator is registered for that specific class+version pair. The kernel cannot transfer runtime state without a matching migrator.

## Resolution

Register a migrator for the predecessor→successor class+version pair in the spirit registry. Implement the `StateMigrator` trait for the transition and register it before attempting the hot-swap. Check `maos registry list-migrators` to see what is currently registered.

## Source Location

```rust
// Emitted from: HotSwapError::EMigratorMissing
// Crate: maos-domain
Err(HotSwapError::EMigratorMissing)
```
