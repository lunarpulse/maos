# Understand MAOS

This is the **"understand MAOS"** door of [the three-door landing page](./index.md)
(NFR-Onb-3) — for readers building a mental model of the architecture, the
constitutional invariants, and the trust boundary.

## Start here

- **Architecture** — the kernel-as-service design and the Spirit ABI:
  [`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`](../../_bmad-output/planning-artifacts/architecture-maos-minimal-opus/).
- **Invariants** — the constitutional invariants the substrate enforces and how
  they're mechanically gated: [`docs/invariants/`](../invariants/).
- **ABI stability** — the v1.0 ABI Stability Triple and compatibility guarantees:
  [`STABILITY.md`](../../STABILITY.md).
- **Trust model** — capability mediation, sandbox tiers, the transparency log,
  and ComplianceClaim envelopes, summarized in [`SECURITY.md`](../../SECURITY.md).

## The big idea

MAOS is a **kernel that hosts LLM-backed Spirits** the way an OS hosts processes:
the Spirit declares what it needs in a manifest, the kernel mediates every
capability, and every consequential action is logged before it is delivered.

---

> **Status (v0.3):** functional index; the polished docs site + WCAG-AA
> conformance are deferred to **Story 9.5** (NFR-Onb-3).
