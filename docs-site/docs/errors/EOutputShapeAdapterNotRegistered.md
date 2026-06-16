---
title: "EOutputShapeAdapterNotRegistered"
sidebar_position: 0
description: "Error reference for EOutputShapeAdapterNotRegistered"
slug: "/errors/EOutputShapeAdapterNotRegistered"
---

# EOutputShapeAdapterNotRegistered

| Field | Value |
|-------|-------|
| **Error Code** | `EOutputShapeAdapterNotRegistered` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Output-shape adapter not registered in the Spirit registry

## Rust Path

`CliWrapperAdmissionError::EOutputShapeAdapterNotRegistered`

## Cause

The spirit manifest references an `output_shape_version` whose adapter is not registered in the Spirit registry. The kernel cannot parse the CLI output without a matching adapter.

## Resolution

Register the output-shape adapter in the Spirit registry before submitting the manifest. Implement the adapter for the declared `output_shape_version` and register it. Alternatively, change the manifest to reference an already-registered shape version.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::EOutputShapeAdapterNotRegistered
// Crate: maos-domain
Err(CliWrapperAdmissionError::EOutputShapeAdapterNotRegistered)
```
