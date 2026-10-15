---
title: "ECrossWallRecallDenied"
sidebar_position: 0
description: "Error reference for ECrossWallRecallDenied"
slug: "/errors/ECrossWallRecallDenied"
---

# ECrossWallRecallDenied

| Field | Value |
|-------|-------|
| **Error Code** | `ECrossWallRecallDenied` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Cross-wall log recall lacks a fresh directional consent grant

## Rust Path

`LogRecallError::ECrossWallRecallDenied`

## Cause

Cross-wall log recall could not prove fresh directional consent: the grant is absent or reversed, the consent provider or state is unavailable or stale, or the remote read port is not configured.

## Triggers

- A cross-wall recall has no grant, a grant in the wrong direction, stale or unavailable consent state, no consent provider, or no remote read port.

## Resolution

Inspect the refusal reason. Obtain a fresh grant for the requested team and log-recall direction, restore the consent provider or state if unavailable, and configure the remote read port before retrying. Do not bypass the consent check.

## Source Location

crates/maos-iac/src/adapter/log_recall.rs
