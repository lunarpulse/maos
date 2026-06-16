---
title: "EComplianceRejection::ContextDrift"
sidebar_position: 0
description: "Error reference for EComplianceRejection::ContextDrift"
slug: "/errors/EComplianceRejection::ContextDrift"
---

# EComplianceRejection::ContextDrift

| Field | Value |
|-------|-------|
| **Error Code** | `EComplianceRejection::ContextDrift` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-compliance` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Runtime execution-context drifted from the attested ComplianceClaim fingerprint

## Rust Path

`EComplianceRejection::ContextDrift`

## Cause

During runtime, the execution context (environment fingerprint, resource bindings, or host state) changed in a way that no longer matches the fingerprint attested in the ComplianceClaim. This indicates the compliance invariant was violated after initial admission.

## Resolution

Investigate what changed in the execution environment since the ComplianceClaim was issued. Common triggers include host reconfiguration, resource re-binding, or container migration. Re-attest the ComplianceClaim against the current context fingerprint and re-submit the spirit.

## Source Location

```rust
// Emitted from: EComplianceRejection::ContextDrift
// Crate: maos-compliance
Err(EComplianceRejection::ContextDrift)
```
