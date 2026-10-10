---
title: "EEgressProxyUnavailable"
sidebar_position: 0
description: "Error reference for EEgressProxyUnavailable"
slug: "/errors/EEgressProxyUnavailable"
---

# EEgressProxyUnavailable

| Field | Value |
|-------|-------|
| **Error Code** | `EEgressProxyUnavailable` |
| **Severity** | security |
| **Recovery Class** | fix_config |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.1.0-alpha.1 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A non-empty Worker egress policy requires an available daemon-owned proxy

## Rust Path

`CliWrapperAdmissionError::EEgressProxyUnavailable`

## Cause

The Worker manifest declares a non-empty `[cli_wrapper]` `egress`, but this `maos` root has no egress proxy: the platform is not Unix, no MAOS home could be resolved (`MAOS_HOME` and `HOME` unset), the proxy failed to start (`maos: egress proxy unavailable: …` on stderr at boot), or the root is shutting down. A Worker's only route to the network is that proxy, so admission refuses rather than spawn a Worker whose declared egress cannot be enforced.

## Resolution

Run the Worker on Linux from a root that can resolve its MAOS home and create its private `egress` directory under it; check boot stderr for the `egress proxy unavailable` line and fix the reported cause. A Worker that needs no network can drop `egress` from its manifest instead.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::EEgressProxyUnavailable
// Crate: maos-domain
Err(CliWrapperAdmissionError::EEgressProxyUnavailable)
```
