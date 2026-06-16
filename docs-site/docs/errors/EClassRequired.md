---
title: "EClassRequired"
sidebar_position: 0
description: "Error reference for EClassRequired"
slug: "/errors/EClassRequired"
---

# EClassRequired

| Field | Value |
|-------|-------|
| **Error Code** | `EClassRequired` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-kernel-core` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Spirit admitted without required [class] section — manifest malformed or parser bypassed

## Rust Path

`SecurityError::EClassRequired`

## Cause

The spirit manifest is missing the mandatory `[class]` section. Every spirit must declare its class for the kernel to apply the correct sandbox policy and lifecycle rules. This can also occur if a manifest parser bypass allowed a classless manifest to reach admission.

## Resolution

Add a valid `[class]` section to the spirit manifest TOML. Example: `[class]
kind = "worker"`. Then re-submit the manifest through the standard admission path — never bypass the manifest parser.

## Source Location

```rust
// Emitted from: SecurityError::EClassRequired
// Crate: maos-kernel-core
Err(SecurityError::EClassRequired)
```
