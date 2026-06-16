---
title: "EPinMismatch::Invalidated"
sidebar_position: 0
description: "Error reference for EPinMismatch::Invalidated"
slug: "/errors/EPinMismatch::Invalidated"
---

# EPinMismatch::Invalidated

| Field | Value |
|-------|-------|
| **Error Code** | `EPinMismatch::Invalidated` |
| **Severity** | user |
| **Recovery Class** | escalate |
| **Owner Crate** | `maos-a2a-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

TOFU pin invalidated for peer (Spirit restart, manual invalidation)

## Rust Path

`EPinMismatch::Invalidated`

## Cause

The TOFU (Trust On First Use) pin for a peer spirit was explicitly invalidated — either because the peer restarted and generated a new identity, or because an operator manually revoked the pin.

## Resolution

Re-establish trust with the peer by initiating a new first-contact handshake. If the peer restarted legitimately, accept the new pin. If the invalidation was unexpected, investigate whether the peer's identity was compromised before re-pinning.

## Source Location

```rust
// Emitted from: EPinMismatch::Invalidated
// Crate: maos-a2a-core
Err(EPinMismatch::Invalidated)
```
