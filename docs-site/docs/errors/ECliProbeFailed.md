---
title: "ECliProbeFailed"
sidebar_position: 0
description: "Error reference for ECliProbeFailed"
slug: "/errors/ECliProbeFailed"
---

# ECliProbeFailed

| Field | Value |
|-------|-------|
| **Error Code** | `ECliProbeFailed` |
| **Severity** | infra |
| **Recovery Class** | retry |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | Yes |
| **Kernel / Spirit** | kernel |

## Summary

CLI probe subprocess exited non-zero, stdio I/O failure, or adapter-declared probe protocol violated

## Rust Path

`CliWrapperAdmissionError::ECliProbeFailed`

## Cause

The CLI wrapper ran its probe subprocess (typically `<binary> --version` or a custom probe command) and the subprocess exited with a non-zero code, produced unreadable output, or violated the adapter's declared probe protocol.

## Resolution

Run the probe command manually outside MAOS to diagnose the failure: e.g. `<binary> --version`. Check that the binary starts correctly, has the right permissions, and that its output matches the adapter's expected probe format. This error is retryable — transient failures (e.g. a locked resource) may resolve on retry.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliProbeFailed
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliProbeFailed)
```
