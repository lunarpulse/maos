---
title: "EComplianceRejection::MalformedClaim"
sidebar_position: 0
description: "Error reference for EComplianceRejection::MalformedClaim"
slug: "/errors/EComplianceRejection::MalformedClaim"
---

# EComplianceRejection::MalformedClaim

| Field | Value |
|-------|-------|
| **Error Code** | `EComplianceRejection::MalformedClaim` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-compliance` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

ComplianceClaim could not be decoded, was missing a required field, or carried an unknown enum value

## Rust Path

`EComplianceRejection::MalformedClaim`

## Cause

The ComplianceClaim payload could not be decoded (invalid encoding), was missing a required field (e.g. `issuer`, `subject`, `expires_at_unix_ms`), or contained an unrecognized enum variant that the kernel cannot interpret.

## Resolution

Validate the ComplianceClaim against the expected schema before submission. Ensure all required fields are present and that enum values match the kernel's known set. Use a JSON/CBOR validator to check encoding integrity.

## Source Location

```rust
// Emitted from: EComplianceRejection::MalformedClaim
// Crate: maos-compliance
Err(EComplianceRejection::MalformedClaim)
```
