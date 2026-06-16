---
title: "EComplianceRejection::SignatureInvalid"
sidebar_position: 0
description: "Error reference for EComplianceRejection::SignatureInvalid"
slug: "/errors/EComplianceRejection::SignatureInvalid"
---

# EComplianceRejection::SignatureInvalid

| Field | Value |
|-------|-------|
| **Error Code** | `EComplianceRejection::SignatureInvalid` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-compliance` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

ComplianceClaim Ed25519 signature verification failed (or signing_alg unsupported)

## Rust Path

`EComplianceRejection::SignatureInvalid`

## Cause

The Ed25519 signature on the ComplianceClaim failed cryptographic verification. This can occur if the claim was tampered with in transit, signed with a wrong or rotated key, or uses an unsupported signing algorithm.

## Resolution

Verify that the signing key used to produce the ComplianceClaim matches the public key the kernel trusts. Check the key fingerprint with the compliance authority. If keys were rotated, update the kernel's trusted-key store and re-sign the claim.

## Source Location

```rust
// Emitted from: EComplianceRejection::SignatureInvalid
// Crate: maos-compliance
Err(EComplianceRejection::SignatureInvalid)
```
