---
title: "ERegionViolation"
sidebar_position: 0
description: "Error reference for ERegionViolation"
slug: "/errors/ERegionViolation"
---

# ERegionViolation

| Field | Value |
|-------|-------|
| **Error Code** | `ERegionViolation` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Foreign-region data detected on a read/verify/enforcement path — region pinning fail-closed (NFR-Comp-4 / PIPL §40)

## Rust Path

`RegionError::ERegionViolation`

## Cause

A region-pinned read or write received data tagged for a different region, or foreign-origin data without a verifiable region tag. The enforcement path rejects it instead of trusting data outside the configured home region.

## Triggers

- A read or write under region pinning carries a foreign region tag or no verifiable region tag.

## Resolution

Inspect the expected and found regions and the operation detail. Route the operation to data bound to the configured home region, or restore its verifiable region metadata from a trusted source. Do not relabel foreign data or disable pinning to force the operation through.

## Source Location

crates/maos-kernel-core/src/memory/read_entry_point.rs and crates/maos-kernel-core/src/memory/write_entry_point.rs
