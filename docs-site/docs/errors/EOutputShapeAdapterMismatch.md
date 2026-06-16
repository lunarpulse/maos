---
title: "EOutputShapeAdapterMismatch"
sidebar_position: 0
description: "Error reference for EOutputShapeAdapterMismatch"
slug: "/errors/EOutputShapeAdapterMismatch"
---

# EOutputShapeAdapterMismatch

| Field | Value |
|-------|-------|
| **Error Code** | `EOutputShapeAdapterMismatch` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

ADR-021: observed CLI output shape does not match declared output_shape_version — admission refused

## Rust Path

`CliWrapperAdmissionError::EOutputShapeAdapterMismatch`

## Cause

The CLI wrapper's actual output (observed during the probe or first invocation) does not match the `output_shape_version` declared in the manifest. Per ADR-021 the kernel enforces shape consistency at admission time to prevent downstream parsing failures.

## Resolution

Update the manifest's `output_shape_version` to match the CLI binary's actual output format, or update the CLI binary to produce output matching the declared shape. Run the CLI manually and compare its output structure against the declared shape adapter.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::EOutputShapeAdapterMismatch
// Crate: maos-domain
Err(CliWrapperAdmissionError::EOutputShapeAdapterMismatch)
```
