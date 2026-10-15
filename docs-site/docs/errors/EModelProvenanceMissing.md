---
title: "EModelProvenanceMissing"
sidebar_position: 0
description: "Error reference for EModelProvenanceMissing"
slug: "/errors/EModelProvenanceMissing"
---

# EModelProvenanceMissing

| Field | Value |
|-------|-------|
| **Error Code** | `EModelProvenanceMissing` |
| **Severity** | policy |
| **Recovery Class** | reject |
| **Owner Crate** | `maos-domain` |
| **Since Version** | 1.0.0 |
| **Retryable** | No |
| **Kernel / Spirit** | kernel |

## Summary

Spirit class requires model-provenance but the [model_provenance] manifest section is absent at admission (NFR-Comp-5 / SB-1047)

## Rust Path

`ProvenanceError::EModelProvenanceMissing`

## Cause

The admission policy requires model provenance, but the Spirit manifest has no [model_provenance] section. Absence is allowed only when the policy does not require it.

## Triggers

- validate_model_provenance sees no [model_provenance] section while ModelProvenancePolicy.require is true.

## Resolution

Add a valid [model_provenance] section with covered_model_id, schema-valid training_data_lineage references, and last_eval_timestamp backed by the actual model evaluation, then resubmit through admission. Do not disable the requirement to admit an undocumented model.

## Source Location

crates/maos-registry/src/admission.rs
