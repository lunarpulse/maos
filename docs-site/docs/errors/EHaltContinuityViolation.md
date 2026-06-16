---
title: "EHaltContinuityViolation"
sidebar_position: 0
description: "Error reference for EHaltContinuityViolation"
slug: "/errors/EHaltContinuityViolation"
---

# EHaltContinuityViolation

| Field | Value |
|-------|-------|
| **Error Code** | `EHaltContinuityViolation` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Hot-swap halt-protocol schema mismatch — successor does not support predecessor halt version (I14)

## Rust Path

`HaltContinuityError::EHaltContinuityViolation`

## Cause

During a hot-swap, the successor spirit does not support the halt-protocol schema version that the predecessor used to serialize its halt state. This violates invariant I14 which requires halt-protocol continuity across swaps.

## Resolution

Ensure the successor spirit's manifest declares support for the predecessor's halt-protocol version in its `halt_versions_supported` list. If the predecessor used halt protocol v2, the successor must list v2 (or a superset). Update the successor manifest and re-attempt the hot-swap.

## Source Location

```rust
// Emitted from: HaltContinuityError::EHaltContinuityViolation
// Crate: maos-domain
Err(HaltContinuityError::EHaltContinuityViolation)
```
