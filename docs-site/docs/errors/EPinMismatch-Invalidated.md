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

The TOFU (Trust On First Use) pin for a peer is invalidated. Usually the peer Spirit restarted: its frame carried a boot nonce different from the pinned one (NFR-Rel-6 restart detection), the certificate identity is unchanged, and re-pin consent is required. The pin can also be manually invalidated, or a rotation-window or first-contact operation can be refused against an existing or invalidated pin.

## Resolution

After a peer restart, confirm the restart with the peer's operator and obtain its new boot nonce (decimal) out of band. On the receiving host's `cohort-a2a-daemon` door run `maosctl a2a repin list`, then `maosctl a2a repin approve <peer> --boot-nonce <N>` within 300 s of the detection: it re-pins the SAME pinned fingerprint with nonce N and writes an Approval Decision Log row first. The re-pin is held in memory only; a restart of the receiving daemon rebuilds pins from `[[tcp.peer_pins]]`. The alternative recovery is to correct that row's `boot_nonce` and restart the receiving host. If the restart was unexpected, investigate a possible compromise before approving; refuse with `maosctl a2a repin reject <peer> --boot-nonce <N>`.

## Source Location

```rust
// Emitted from: EPinMismatch::Invalidated
// Crate: maos-a2a-core
Err(EPinMismatch::Invalidated)
```
