---
title: "EManifestSchemaConflict"
sidebar_position: 0
description: "Error reference for EManifestSchemaConflict"
slug: "/errors/EManifestSchemaConflict"
---

# EManifestSchemaConflict

| Field | Value |
|-------|-------|
| **Error Code** | `EManifestSchemaConflict` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Manifest declares both [class] and [cli_wrapper] — mutually exclusive per architecture §6.7

## Rust Path

`CliWrapperAdmissionError::EManifestSchemaConflict`

## Cause

The spirit manifest contains both a `[class]` section and a `[cli_wrapper]` section. Per architecture §6.7 these are mutually exclusive — a spirit is either a native class-based spirit or a CLI wrapper, never both.

## Resolution

Remove one of the conflicting sections from the manifest. If the spirit wraps an external CLI binary, keep `[cli_wrapper]` and remove `[class]`. If it is a native spirit, keep `[class]` and remove `[cli_wrapper]`.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::EManifestSchemaConflict
// Crate: maos-domain
Err(CliWrapperAdmissionError::EManifestSchemaConflict)
```
