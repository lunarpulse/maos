---
title: "ECliWrapperTierNotGranted"
sidebar_position: 0
description: "Error reference for ECliWrapperTierNotGranted"
slug: "/errors/ECliWrapperTierNotGranted"
---

# ECliWrapperTierNotGranted

| Field | Value |
|-------|-------|
| **Error Code** | `ECliWrapperTierNotGranted` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.9.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Manifest requested sandbox tier not granted by host-side grant allowlist — fail-closed, no silent downgrade

## Rust Path

`CliWrapperAdmissionError::ECliWrapperTierNotGranted`

## Cause

The spirit manifest requests a sandbox tier (e.g. t3) that the host-side grant allowlist does not include. The kernel enforces fail-closed behavior — it will not silently downgrade to a lower tier.

## Resolution

Add the requested sandbox tier to the host's grant allowlist configuration. Locate the grant allowlist (typically in the MAOS host config or deployment policy) and add the spirit's identity or class to the t3 grant list. Re-deploy after updating the allowlist.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliWrapperTierNotGranted
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliWrapperTierNotGranted)
```
