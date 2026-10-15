---
title: "ETeamIdInvalid"
sidebar_position: 0
description: "Error reference for ETeamIdInvalid"
slug: "/errors/ETeamIdInvalid"
---

# ETeamIdInvalid

| Field | Value |
|-------|-------|
| **Error Code** | `ETeamIdInvalid` |
| **Severity** | policy |
| **Recovery Class** | fix_config |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 2.2.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Operator-supplied team id failed the frozen lowercase ASCII tenant-identity grammar (ADR-055)

## Rust Path

`TeamIdError::ETeamIdInvalid`

## Cause

A team identifier is not already canonical lowercase ASCII [a-z0-9-] with a length of 2..=32 bytes. TeamId rejects uppercase letters, whitespace, Unicode, and other characters without normalizing signed identity material.

## Triggers

- TeamId::new receives a team id such as Team-A, a value with surrounding whitespace, or a value outside the 2..=32 byte range.

## Resolution

Correct the team identifier at its authoritative configuration or manifest source to 2..=32 lowercase ASCII letters, digits, or hyphens, and regenerate any signatures over changed material. Do not silently trim or lowercase an existing signed team identifier.

## Source Location

crates/maos-domain/src/team.rs
