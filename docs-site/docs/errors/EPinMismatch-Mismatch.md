---
title: "EPinMismatch::Mismatch"
sidebar_position: 0
description: "Error reference for EPinMismatch::Mismatch"
slug: "/errors/EPinMismatch::Mismatch"
---

# EPinMismatch::Mismatch

| Field | Value |
|-------|-------|
| **Error Code** | `EPinMismatch::Mismatch` |
| **Severity** | security |
| **Recovery Class** | escalate |
| **Owner Crate** | `maos-a2a-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

TOFU pin mismatch for peer — pinned fingerprint does not match observed

## Rust Path

`EPinMismatch::Mismatch`

## Cause

The observed cryptographic fingerprint of a peer spirit does not match the previously pinned fingerprint (TOFU). This is a potential man-in-the-middle indicator — something is presenting a different identity than what was trusted on first contact.

## Resolution

Do NOT blindly accept the new fingerprint. Investigate why the peer's fingerprint changed. If the peer was legitimately re-keyed, verify through an out-of-band channel and update the pin. If the change is unexplained, treat it as a potential security incident and escalate to the security team.

## Source Location

```rust
// Emitted from: EPinMismatch::Mismatch
// Crate: maos-a2a-core
Err(EPinMismatch::Mismatch)
```
