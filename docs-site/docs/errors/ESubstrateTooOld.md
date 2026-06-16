---
title: "ESubstrateTooOld"
sidebar_position: 0
description: "Error reference for ESubstrateTooOld"
slug: "/errors/ESubstrateTooOld"
---

# ESubstrateTooOld

| Field | Value |
|-------|-------|
| **Error Code** | `ESubstrateTooOld` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-kernel-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

FR8: running kernel older than Spirit's declared min_substrate_version

## Rust Path

`SecurityError::ESubstrateTooOld`

## Cause

The spirit manifest declares a `min_substrate_version` (per FR8) that is higher than the running kernel's version. The spirit requires a newer kernel than what is currently deployed.

## Resolution

Upgrade the MAOS kernel to at least the version specified in the spirit's `min_substrate_version` field. Run `maos version` to check the current kernel version. Alternatively, if the spirit's requirement is overly strict, lower the `min_substrate_version` in the manifest.

## Source Location

```rust
// Emitted from: SecurityError::ESubstrateTooOld
// Crate: maos-kernel-core
Err(SecurityError::ESubstrateTooOld)
```
