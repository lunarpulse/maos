---
title: "EEgressNotGranted"
sidebar_position: 0
description: "Error reference for EEgressNotGranted"
slug: "/errors/EEgressNotGranted"
---

# EEgressNotGranted

| Field | Value |
|-------|-------|
| **Error Code** | `EEgressNotGranted` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 0.1.0-alpha.1 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The Worker requests a host:port destination outside its host grant

## Rust Path

`CliWrapperAdmissionError::EEgressNotGranted`

## Cause

The Worker manifest's `[cli_wrapper]` `egress` lists a `host:port` destination that the operator's host grant for this `command` and `[author]` `name` does not permit. Manifest egress must be a subset of the grant's `permitted_egress_destinations`, where a bare host grants only `host:443`. `maos run` refuses the Worker at admission, before any probe or spawn; no destination is ever added from the manifest.

## Resolution

Remove the destination from the manifest's `egress`, or have the operator add it to the matching `[[grant]]` entry's `permitted_egress_destinations` in the `MAOS_HOST_GRANTS` file (write `host:port` for any port other than 443), then rerun `maos run`.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::EEgressNotGranted
// Crate: maos-domain
Err(CliWrapperAdmissionError::EEgressNotGranted { destination })
```
