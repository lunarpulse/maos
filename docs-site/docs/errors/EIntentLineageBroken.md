---
title: "EIntentLineageBroken"
sidebar_position: 0
description: "Error reference for EIntentLineageBroken"
slug: "/errors/EIntentLineageBroken"
---

# EIntentLineageBroken

| Field | Value |
|-------|-------|
| **Error Code** | `EIntentLineageBroken` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Cross-Spirit frame has empty intent_lineage on non-human origin (I14 consent-laundering guard)

## Rust Path

`IacBusError::EIntentLineageBroken`

## Cause

A cross-spirit IAC (inter-agent communication) frame arrived with an empty `intent_lineage` field, but its origin is non-human. Invariant I14 requires that all machine-originated frames carry a non-empty intent lineage to prevent consent-laundering — one spirit impersonating a human origin to bypass approval gates.

## Resolution

Ensure the sending spirit populates the `intent_lineage` field on every outbound IAC frame. The lineage must trace back to the original human-approved intent. Review the sending spirit's IAC dispatch logic to confirm it propagates lineage from its own inbound frames.

## Source Location

```rust
// Emitted from: IacBusError::EIntentLineageBroken
// Crate: maos-domain
Err(IacBusError::EIntentLineageBroken)
```
