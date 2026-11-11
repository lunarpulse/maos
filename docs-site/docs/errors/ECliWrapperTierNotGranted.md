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

Add a `[[grant]]` entry for the manifest's `command` (`attested_image`) and `[author]` `name` (`signing_key_id`) with `permitted_tier = "T3"` to the operator-owned grant file that the `MAOS_HOST_GRANTS` environment variable names, then re-run `maos run`. The manifest cannot grant itself a tier; only the host-managed allowlist (the built-in hermetic fixture grants plus that file) can.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliWrapperTierNotGranted
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliWrapperTierNotGranted)
```
