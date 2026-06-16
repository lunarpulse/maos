---
title: "ECliWrapperRequiresT3"
sidebar_position: 0
description: "Error reference for ECliWrapperRequiresT3"
slug: "/errors/ECliWrapperRequiresT3"
---

# ECliWrapperRequiresT3

| Field | Value |
|-------|-------|
| **Error Code** | `ECliWrapperRequiresT3` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.5.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

CliWrapperSpirit MUST declare sandbox tier = t3; lower tiers cannot contain subprocess CLI invocation

## Rust Path

`CliWrapperAdmissionError::ECliWrapperRequiresT3`

## Cause

The CLI wrapper spirit manifest declares a sandbox tier lower than t3. Because CLI wrappers spawn external subprocesses, they require t3 (the highest sandbox tier) to contain the subprocess securely.

## Resolution

Set `sandbox_tier = "t3"` in the spirit manifest's `[cli_wrapper]` section. Lower tiers (t1, t2) do not permit subprocess invocation by design.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliWrapperRequiresT3
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliWrapperRequiresT3)
```
