---
title: "ESkillQueue::ForeignNamespace"
sidebar_position: 0
description: "Author namespace refusal"
slug: "/errors/ESkillQueue::ForeignNamespace"
---

# ESkillQueue::ForeignNamespace

| Field | Value |
|---|---|
| **Error Code** | `ESkillQueue::ForeignNamespace` |
| **Severity** | security |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-skill` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

The authored id does not begin with `<class>.`, where the root resolves the author class from the live Spirit.

## Cause

A Spirit attempted to author outside its own namespace.

## Resolution

Use an id in the author's namespace. Do not change provenance or bypass the root-bound port. Nothing is enqueued.

## Source Location

crates/maos-skill/src/errors.rs
