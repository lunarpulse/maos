---
title: Understand MAOS
sidebar_position: 3
description: Architecture, invariants, and trust model.
---

# Understand MAOS

For readers building a mental model of the architecture, constitutional invariants, and trust boundary.

## The big idea

MAOS is a **kernel that hosts LLM-backed Spirits** the way an OS hosts processes: the Spirit declares what it needs in a manifest, the kernel mediates every capability, and every consequential action is logged before it is delivered.

## Key concepts

| Concept | What it means |
|---|---|
| **Spirit** | An LLM-backed agent, loaded from a manifest, isolated by sandbox tiers |
| **Manifest** | The Spirit's declaration of identity, capabilities, and constraints ([schema reference](/manifest/latest)) |
| **Kernel** | The MAOS runtime: mediates capabilities, enforces invariants, manages lifecycle |
| **Transparency Log** | Append-only audit trail — every consequential action is recorded before delivery |
| **ComplianceClaim** | Cryptographic envelope binding a Spirit's identity to its compliance posture |
| **Capability Token** | Time-bounded, scope-limited, PID-bound access credential |

## Architecture

- **Hexagonal design** — the kernel defines port traits; adapters (inference, MCP, A2A, persistence) are pluggable
- **14 constitutional invariants** — mechanically enforced by CI gates (see [`docs/invariants/`](https://github.com/lunarpulse/maos/tree/main/docs/invariants))
- **ABI Stability Triple** — `(kernel_version, abi_version, manifest_schema_version)` with N-1 supported / N-2 hard-refusal policy

## Trust model

- Capability mediation: every Spirit operation requires a scoped, time-bounded token
- Sandbox tiers: T0 (no isolation) → T3 (container-isolated)
- The transparency log records before delivery — if it's not in the log, it didn't happen
- ComplianceClaim envelopes are verified at admission

See [`SECURITY.md`](https://github.com/lunarpulse/maos/blob/main/SECURITY.md) for the full trust model.

## Next steps

- [Manifest Schema](/manifest/latest) — every field your Spirit can declare
- [Migration Guides](/migrate/) — breaking-change migration paths
- [ABI Reference](/abi/v1/) — generated API docs for `maos-spirit-abi`
