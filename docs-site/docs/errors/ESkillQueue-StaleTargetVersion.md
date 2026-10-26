---
title: "ESkillQueue::StaleTargetVersion"
sidebar_position: 0
description: "Current owned version requirement"
slug: "/errors/ESkillQueue::StaleTargetVersion"
---

# ESkillQueue::StaleTargetVersion

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::StaleTargetVersion` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

A proposal targets an obsolete owned version, or a new authored document does not advance the admitted version.

## Cause

The target changed after the author selected it, or the submitted version is not newer.

## Resolution

Read current owned entries and propose against the highest admitted semver. Submit revised documents with an advancing version; revision admission alone never applies a diff.

## Source Location

crates/maos-skill/src/errors.rs
