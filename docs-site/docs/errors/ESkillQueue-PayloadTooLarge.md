---
title: "ESkillQueue::PayloadTooLarge"
sidebar_position: 0
description: "Skill submission payload limit"
slug: "/errors/ESkillQueue--PayloadTooLarge"
---

# ESkillQueue::PayloadTooLarge

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::PayloadTooLarge` |
| **Severity** | policy |
| **Recovery Class** | retry_with_correction |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | Yes, with correction |
| **Kernel / Spirit** | kernel |

## Summary

The submitted skill document or revision diff exceeds its 256 KiB limit.

## Cause

Spirit-authored payloads are bounded before they enter the pending queue.

## Resolution

Reduce the document or diff to at most 256 KiB before submitting it.

## Source Location

crates/maos-skill/src/errors.rs
