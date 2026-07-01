# Export Counsel Precondition — Story 11.1a

**Date:** 2026-06-30
**Status:** OPEN — merge precondition, not a note
**Blocks:** Merge of Story 11.1a to the shippable line (any release branch / `main` post-freeze)

## The precondition

Merging Story 11.1a to the **shippable line** is **GATED** on export-compliance
counsel clearing the **5D002.c.1** classification question raised by the WASM
hosting surface. This is a **named merge precondition**, not an advisory note:
until counsel signs off (and records the disposition in
`docs/compliance/eccn-classification.md`), 11.1a stays on the development line
behind the `wasm-host` feature flag and MUST NOT be promoted to a distributable
build.

## The open question

The classification trigger is undecided. Two candidate triggers are on the
table, and they have very different blast radii:

1. **The vendored `wasmtime` engine.** If 5D002.c.1 is triggered by the
   in-repo, compiler-bearing `wasmtime` dependency itself, then the
   classification event is the moment the dependency enters `Cargo.toml` /
   `Cargo.lock` of a crate on the shippable line — *regardless of whether the
   feature is enabled in any shipped build*. Under this reading the dependency
   add is itself the controlled event and counsel must clear it before the dep
   touches a distributable manifest.

2. **The user-shipped WASM Spirit.** If 5D002.c.1 is triggered only by the
   runtime execution of user-supplied WASM code (the Spirit payload), then the
   vendored engine source behind a dev-only feature is not the trigger, and the
   classification event is the first *distributable runtime* that actually
   invokes `wasmtime`.

Counsel must determine which reading applies. The answer determines whether the
`maos-bin` `wasm-host` dependency wiring can land on `main` at all before
clearance, or only on a feature-gated development line.

## Preflight position (MAOS engineering, pending counsel)

While the question is open, MAOS engineering holds the following preflight
position, which is deliberately conservative and is *not* a legal determination:

- **In-tree source behind a dev feature ≠ a distributable runtime.** The
  `wasm-host` feature is **OFF by default** in `crates/maos-bin/Cargo.toml`.
  The vendored `wasmtime` engine, `maos-wasm-host`, and `maos-wasm-runner` are
  gated behind that feature and do NOT appear in the default `maos` build
  artifact. The `xtask` gate
  `check_export_control::check_wasm_host_absent_from_default` (Story 11.1a
  negative AC) asserts this absence and goes RED if the feature leaks into the
  default build.
- This keeps the WASM surface on the **development line** only. It does **not**
  constitute a distributable runtime and is asserted to be non-distributable
  precisely *because* the classification question is open.
- This position is **provisional and reversible**: if counsel rules that the
  trigger is reading (1) above, the dependency add itself must be withdrawn from
  any shippable manifest until cleared, regardless of feature-gating.

## Resolution criteria

This precondition is satisfied when **all** of the following hold:

1. Export-compliance counsel has issued a written determination on whether the
   5D002.c.1 trigger is the vendored engine (reading 1) or the user-shipped
   Spirit runtime (reading 2).
2. The resulting ECCN classification for the WASM hosting surface is recorded
   in `docs/compliance/eccn-classification.md` (the doc enumerated by the
   `check-export-control` ship-gate).
3. STABILITY.md `<!-- PRESERVED:export -->` fence reflects the determination.

Until then, the `wasm-host` feature remains OFF by default and the
`check_wasm_host_absent_from_default` gate guards the default build.
