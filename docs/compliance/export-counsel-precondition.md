# Export Counsel — Release Claim Boundary (formerly a Story 11.1a merge precondition)

**Date:** 2026-06-30 (original); superseded 2026-11-12
**Status:** SUPERSEDED as a merge precondition — now a **release claim boundary**.
No export-compliance counsel review was performed; no ECCN determination was made.
**Blocks:** nothing. Merge and the GA tag are not gated on counsel
(operator ruling 2026-11-12; `RELEASE-HOLDS.md` Boundary 2).

## Current position

The operator ruled on 2026-11-12 that NFR-Comp-1 (ECCN classification) is not
being pursued: there is no counsel, no firm and no budget; only AI agents.
Consequences:

- Counsel clearance is **not** a merge precondition for Story 11.1a or any
  other change to the shippable line, and not a GA-tag precondition.
- The release **may not claim** NFR-Comp-1 is met, may not claim any ECCN or
  EAR99 classification, and may not claim counsel cleared the WASM hosting
  surface. The release notes say so (`RELEASE-HOLDS.md` GA-tag procedure,
  item 2).
- The operator ruled the WASM host **ON** in the default build (story
  `21-9-wasm-host-default-on`). The default `maos` build includes the Wasmtime
  engine; an explicit engine-free build
  (`--no-default-features --features network`) remains supported. Neither build
  is classified, and nothing here is permission to distribute in any
  jurisdiction. Operators and distributors must obtain their own advice.
- `STABILITY.md` §Export states exactly: "No ECCN determination was made; the
  surface is not classified."

## Historical rationale (retained; not a current gate)

Originally (2026-06-30) merging Story 11.1a to the shippable line was gated on
counsel clearing the **5D002.c.1** question raised by the WASM hosting surface,
with the feature held OFF by default so the development line carried it only.
The open question was never answered. Two candidate triggers were identified:

1. **The vendored `wasmtime` engine.** If 5D002.c.1 is triggered by the
   in-repo, compiler-bearing `wasmtime` dependency itself, then the
   classification event is the moment the dependency enters a shippable
   crate's `Cargo.toml` / `Cargo.lock`, regardless of feature state.
2. **The user-shipped WASM Spirit.** If 5D002.c.1 is triggered only by runtime
   execution of user-supplied WASM code, the engine source is not the trigger
   and the event is the first distributable runtime that invokes `wasmtime`.

Which reading applies remains undecided. The former engineering preflight
(off-by-default feature plus the `check_export_control` absence gate) was a
conservative engineering position, never a legal determination, and the
default-on ruling retires it. If anyone later obtains a counsel determination,
it is recorded in `docs/compliance/eccn-classification.md` and reflected in the
`STABILITY.md` `<!-- PRESERVED:export -->` fence; until then the open question
stands as a claim boundary.
