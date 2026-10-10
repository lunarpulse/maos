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

Worker ([cli_wrapper]) manifest requested a sandbox tier other than T3; T3 is the only Worker path (below it no scoped egress exists, T4 has no spawn path)

## Rust Path

`CliWrapperAdmissionError::ECliWrapperRequiresT3`

## Cause

The Worker manifest's `[sandbox]` `tier` resolves to a tier other than `T3` (`T0`, `T1`, `T2` or `T4`). A `[cli_wrapper]` Worker runs only inside a verified T3 rootless container (`--network=none`, `--read-only`, `--cap-drop=ALL`, `--user=65534:65534`), whose only egress is the daemon's proxy socket at `/proxy/egress.sock`. Below `T3` no scoped egress or credential custody exists for a Worker; `T4` has no spawn path. `maos run` refuses the manifest at load, before any probe or spawn. `T3` is not the highest tier, and there is no `hardened` tier: the parser accepts only `T0`–`T4`.

## Resolution

Set `tier = "T3"` in the manifest's `[sandbox]` table, or omit `[sandbox]` (a Worker requests `T3` by default). The operator must also grant `T3` for the manifest's `command` and `[author]` `name` in the `MAOS_HOST_GRANTS` file; otherwise admission fails with `ECliWrapperTierNotGranted`. See [CLI Wrapper Spirit](/cookbook/cli-wrapper-spirit) for the full Worker profile.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliWrapperRequiresT3
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliWrapperRequiresT3)
```
