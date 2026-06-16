---
title: "EAbiTooNew"
sidebar_position: 0
description: "Error reference for EAbiTooNew"
slug: "/errors/EAbiTooNew"
---

# EAbiTooNew

| Field | Value |
|-------|-------|
| **Error Code** | `EAbiTooNew` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-kernel-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Manifest schema version above maximum supported — kernel upgrade required

## Rust Path

`SecurityError::EAbiTooNew`

## Cause

The spirit manifest declares a schema_version higher than the kernel's maximum supported ABI version. This happens when a spirit built for a newer kernel release is deployed onto an older kernel that has not been upgraded.

## Resolution

Upgrade the MAOS kernel to a version that supports the manifest's declared schema_version. Run `maos version` to check the current kernel ABI ceiling, then consult the release notes for the minimum kernel version that covers the spirit's declared version.

## Source Location

```rust
// Emitted from: SecurityError::EAbiTooNew
// Crate: maos-kernel-core
Err(SecurityError::EAbiTooNew)
```
