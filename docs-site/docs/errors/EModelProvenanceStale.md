---
title: "EModelProvenanceStale"
sidebar_position: 0
description: "Error reference for EModelProvenanceStale"
slug: "/errors/EModelProvenanceStale"
---

# EModelProvenanceStale

| Field | Value |
|-------|-------|
| **Error Code** | `EModelProvenanceStale` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Declared model-provenance last_eval_timestamp is older than the operator-configured staleness window (D6 — presence necessary but not sufficient)

## Rust Path

`ProvenanceError::EModelProvenanceStale`

## Cause

The declared model-provenance last_eval_timestamp is older than the admission policy's configured max_age_secs relative to now_unix_secs. The staleness check runs only when the operator supplies a maximum age.

## Triggers

- The elapsed age since last_eval_timestamp exceeds an enabled ModelProvenancePolicy.max_age_secs at admission.

## Resolution

Compare the reported evaluation time, admission time, and maximum age with the real evaluation record and clock. Perform a fresh evaluation when required and update last_eval_timestamp to that evaluation's actual time before resubmitting; do not just advance the timestamp to evade the age check.

## Source Location

crates/maos-manifest/src/manifest.rs and crates/maos-registry/src/admission.rs
