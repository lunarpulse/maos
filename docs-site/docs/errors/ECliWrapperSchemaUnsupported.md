---
title: "ECliWrapperSchemaUnsupported"
sidebar_position: 0
description: "Error reference for ECliWrapperSchemaUnsupported"
slug: "/errors/ECliWrapperSchemaUnsupported"
---

# ECliWrapperSchemaUnsupported

| Field | Value |
|-------|-------|
| **Error Code** | `ECliWrapperSchemaUnsupported` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Worker manifest [cli_wrapper] manifest_schema_version is outside the supported window or not an integer — refused before the strict parse

## Rust Path

`CliWrapperAdmissionError::ECliWrapperSchemaUnsupported`

## Cause

The Worker manifest's `[cli_wrapper]` table declares `manifest_schema_version` with a value outside the window this kernel accepts (`1..=1`), or with a value that is not an integer (a string, a float, a boolean, an array, zero or a negative number). The check runs before the strict parse of the table, so a manifest written for a newer schema is refused as too new rather than as an unknown key. An absent key means `1`; the check runs before any probe, process or token exists.

## Resolution

Set `manifest_schema_version` in `[cli_wrapper]` to an integer inside the window the error message names (`1` today), or remove the key (absent means `1`). A manifest written for a newer schema needs a newer MAOS kernel: upgrade the kernel, do not edit the manifest down.

## Source Location

```rust
// Emitted from: CliWrapperAdmissionError::ECliWrapperSchemaUnsupported
// Crate: maos-domain
Err(CliWrapperAdmissionError::ECliWrapperSchemaUnsupported)
```
