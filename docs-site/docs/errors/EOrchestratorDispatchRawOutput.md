---
title: "EOrchestratorDispatchRawOutput"
sidebar_position: 0
description: "Error reference for EOrchestratorDispatchRawOutput"
slug: "/errors/EOrchestratorDispatchRawOutput"
---

# EOrchestratorDispatchRawOutput

| Field | Value |
|-------|-------|
| **Error Code** | `EOrchestratorDispatchRawOutput` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Orchestrator dispatch references raw worker output instead of a distillate (FR21)

## Rust Path

`IacBusError::EOrchestratorDispatchRawOutput`

## Cause

An orchestrator spirit's dispatch references raw worker output directly instead of referencing a distillate (per FR21). Raw output must be distilled before it can be dispatched to downstream consumers, to ensure safety and compliance review.

## Resolution

Update the orchestrator's dispatch logic to reference a distillate ID rather than a raw output ID. Ensure the worker's output has been processed through the distillation pipeline before the orchestrator dispatches it. Check the orchestrator's IAC frame construction code.

## Source Location

```rust
// Emitted from: IacBusError::EOrchestratorDispatchRawOutput
// Crate: maos-domain
Err(IacBusError::EOrchestratorDispatchRawOutput)
```
