# Run MAOS

This is the **"run MAOS"** door of [the three-door landing page](./index.md)
(NFR-Onb-3) — for operators standing up and running the MAOS kernel.

## Build the kernel

```sh
cargo build -p maos-bin --release
```

## Operate

The kernel composition root lives in `crates/maos-bin`. Operational surfaces —
the transparency log, capability mediation, sandbox tiers, ComplianceClaim
admission, the registry, and yank propagation — are documented under
[`docs/`](../) and the published [`STABILITY.md`](../../STABILITY.md) ABI
compatibility matrix.

For air-gapped install and registry import, see the `maosctl import --offline`
path shipped in Story 7.2.

---

> **Status (v0.3):** functional entry point; the polished operator portal +
> WCAG-AA conformance are deferred to **Story 9.5** (NFR-Onb-3).
