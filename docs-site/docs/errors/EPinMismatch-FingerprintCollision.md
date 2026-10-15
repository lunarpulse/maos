---
title: "EPinMismatch::FingerprintCollision"
sidebar_position: 0
description: "Error reference for EPinMismatch::FingerprintCollision"
slug: "/errors/EPinMismatch::FingerprintCollision"
---

# EPinMismatch::FingerprintCollision

| Field | Value |
|-------|-------|
| **Error Code** | `EPinMismatch::FingerprintCollision` |
| **Severity** | security |
| **Recovery Class** | escalate |
| **Owner Crate** | `maos-a2a-core` |
| **Since Version** | 2.2.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Fingerprint insertion refused — another peer already holds the fingerprint as a current pin or open rotation generation

## Rust Path

`EPinMismatch::FingerprintCollision`

## Cause

A proposed first-contact pin, operator-approved re-pin, or next rotation fingerprint is already held by another peer as a current pin or open rotation generation. Opening a rotation window with the peer's own current fingerprint is also refused.

## Triggers

- First contact or an approved re-pin reuses another peer's held fingerprint; a rotation next fingerprint collides with another peer or equals its own current pin.

## Resolution

Compare the reported peer, colliding_peer, and fingerprint against the trusted peer configuration. Give each peer a distinct verified identity; for rotation, use a new fingerprint different from the current pin. Investigate unexpected key reuse before retrying the trust or rotation operation.

## Source Location

crates/maos-a2a-core/src/tofu.rs
