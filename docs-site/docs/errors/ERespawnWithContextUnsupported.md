---
title: "ERespawnWithContextUnsupported"
sidebar_position: 0
description: "Error reference for ERespawnWithContextUnsupported"
slug: "/errors/ERespawnWithContextUnsupported"
---

# ERespawnWithContextUnsupported

| Field | Value |
|-------|-------|
| **Error Code** | `ERespawnWithContextUnsupported` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.9.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

recovery_policy=respawn_with_context deferred to Epic 10 / NFR-Rel-3 HSIS — admission fails closed

## Rust Path

`CliWrapperAdmissionError::ERespawnWithContextUnsupported`

## Cause

The spirit manifest sets `recovery_policy = "respawn_with_context"`, but this recovery mode is not yet implemented — it is deferred to Epic 10 / NFR-Rel-3 (HSIS). The kernel fails closed rather than silently ignoring the unsupported policy.

## Resolution

Change the manifest's `recovery_policy` to a supported value (e.g. `"respawn"` or `"halt"`). The `respawn_with_context` mode will be available in a future release. Monitor the MAOS roadmap for Epic 10 delivery.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ERespawnWithContextUnsupported
// Crate: maos-domain
Err(CliWrapperAdmissionError::ERespawnWithContextUnsupported)
```
