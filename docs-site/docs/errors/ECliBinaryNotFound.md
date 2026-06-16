---
title: "ECliBinaryNotFound"
sidebar_position: 0
description: "Error reference for ECliBinaryNotFound"
slug: "/errors/ECliBinaryNotFound"
---

# ECliBinaryNotFound

| Field | Value |
|-------|-------|
| **Error Code** | `ECliBinaryNotFound` |
| **Severity** | infra |
| **Recovery Class** | fix_config |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

CLI binary not found on PATH or at the declared path

## Rust Path

`CliWrapperAdmissionError::ECliBinaryNotFound`

## Cause

The CLI wrapper spirit references an external binary (via the manifest's `binary` or `path` field) that does not exist at the declared filesystem path and is not discoverable on the system PATH.

## Resolution

Install the required CLI binary and ensure it is either on PATH or referenced by absolute path in the manifest. Verify with `which <binary>` or `ls <declared_path>`. If running in a container, ensure the binary is included in the image.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliBinaryNotFound
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliBinaryNotFound)
```
