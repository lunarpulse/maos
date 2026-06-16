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

No TOFU pin exists for the target peer because first-contact has never been completed. The spirit attempted to communicate with a peer it has never established trust with.

## Resolution

Initiate a first-contact handshake with the peer to establish and record the TOFU pin. Use `maos a2a trust <peer_id>` or the equivalent API call to perform initial trust establishment before attempting communication.

## Source Location

```rust
// Emitted from: EPinMismatch::NotPinned
// Crate: maos-a2a-core
Err(EPinMismatch::NotPinned)
```
