---
title: "EAbiTooOld"
sidebar_position: 0
description: "Error reference for EAbiTooOld"
slug: "/errors/EAbiTooOld"
---

# EAbiTooOld

| Field | Value |
|-------|-------|
| **Error Code** | `EAbiTooOld` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-kernel-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Manifest schema version below N-2 minimum supported — hard refusal

## Rust Path

`SecurityError::EAbiTooOld`

## Cause

The spirit manifest declares a schema_version that falls below the kernel's N-2 minimum support window. The kernel enforces a hard floor to prevent running dangerously outdated manifests that may lack required security fields.

## Resolution

Re-author the spirit manifest to target a schema_version within the kernel's supported range. Check `maos manifest validate <manifest>` for the current minimum, then bump the manifest's `schema_version` field and verify all required fields for that version are present.

## Source Location

```rust
// Emitted from: SecurityError::EAbiTooOld
// Crate: maos-kernel-core
Err(SecurityError::EAbiTooOld)
```
