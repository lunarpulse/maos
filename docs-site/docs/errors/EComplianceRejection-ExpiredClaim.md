---
title: "EComplianceRejection::ExpiredClaim"
sidebar_position: 0
description: "Error reference for EComplianceRejection::ExpiredClaim"
slug: "/errors/EComplianceRejection::ExpiredClaim"
---

# EComplianceRejection::ExpiredClaim

| Field | Value |
|-------|-------|
| **Error Code** | `EComplianceRejection::ExpiredClaim` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-compliance` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

ComplianceClaim carried an expires_at_unix_ms in the past

## Rust Path

`EComplianceRejection::ExpiredClaim`

## Cause

The ComplianceClaim's `expires_at_unix_ms` timestamp is in the past relative to the kernel's clock. Claims are time-bounded to prevent stale attestations from being replayed.

## Resolution

Re-issue the ComplianceClaim with a fresh `expires_at_unix_ms` value. Ensure the claim-issuing service and the MAOS kernel have synchronized clocks (NTP). If clock skew is suspected, check `timedatectl` or equivalent on both the issuer and the kernel host.

## Source Location

```rust
// Emitted from: EComplianceRejection::ExpiredClaim
// Crate: maos-compliance
Err(EComplianceRejection::ExpiredClaim)
```
