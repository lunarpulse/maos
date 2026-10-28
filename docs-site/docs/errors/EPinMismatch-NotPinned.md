---
title: "EPinMismatch::NotPinned"
sidebar_position: 0
description: "Error reference for EPinMismatch::NotPinned"
slug: "/errors/EPinMismatch::NotPinned"
---

# EPinMismatch::NotPinned

| Field | Value |
|-------|-------|
| **Error Code** | `EPinMismatch::NotPinned` |
| **Severity** | user |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-a2a-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

No TOFU pin recorded for peer — first-contact not yet attempted

## Rust Path

`EPinMismatch::NotPinned`

## Cause

No TOFU pin exists for the target peer. Pins are configured, not learned: each one is loaded at transport bind from a `[[tcp.peer_pins]]` row in this host's configuration, and an unpinned peer is refused rather than trusted on first contact.

## Resolution

Verify the peer's leaf-certificate SHA-256 fingerprint and current boot nonce out of band, add a `[[tcp.peer_pins]]` row for it (`peer_id`, `fingerprint`, decimal `boot_nonce`) to this host's configuration, and restart this process so the pin store is rebuilt from config. No runtime command creates a first pin; `maosctl a2a repin` only re-arms an existing pin.

## Source Location

```rust
// Emitted from: EPinMismatch::NotPinned
// Crate: maos-a2a-core
Err(EPinMismatch::NotPinned)
```
