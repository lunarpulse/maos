---
title: "ERegionTagInvalid"
sidebar_position: 0
description: "Error reference for ERegionTagInvalid"
slug: "/errors/ERegionTagInvalid"
---

# ERegionTagInvalid

| Field | Value |
|-------|-------|
| **Error Code** | `ERegionTagInvalid` |
| **Severity** | policy |
| **Recovery Class** | fix_config |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Operator-supplied region tag failed the frozen ascii-v1 grammar (AC-12), rejected before any key derivation

## Rust Path

`RegionError::ERegionTagInvalid`

## Cause

An operator-supplied region tag is invalid after trimming whitespace and ASCII lowercasing: its length is outside 2..=32 bytes or it contains characters other than lowercase ASCII letters, digits, and hyphens.

## Triggers

- Region::canonicalize receives a tag such as eu_west, a one-character tag, or a tag longer than 32 bytes after trim and ASCII lowercasing.

## Resolution

Correct the region tag in operator configuration or MAOS_REGION_HOME to 2..=32 ASCII characters from [a-z0-9-], then retry. Region tags are canonicalized by trimming and ASCII lowercasing; underscores, Unicode characters, and embedded spaces remain invalid.

## Source Location

crates/maos-domain/src/region.rs
