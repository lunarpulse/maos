# 20-8 creation — records owed

At creation: D01 A / D02 A / D11 A (2026-10-30); D-18-7-11 A (2026-10-26); D-20-8-01…12 proposed. Line numbers are at
`8c809f9c` unless marked post-edit. They record provenance and are not current offsets. Story and tracker status are `ready-for-dev`;
no implementation closure is claimed.

**Rulings recorded 2026-11-03 (ask `d_batch4`):** D-20-8-01…12 as recommended. KS and the ceilings remain measured requests, binding at dev start; no implementation or landing claimed.

## Applied at creation (Create208, 2026-11-03)

| # | File | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-8-manifest-contract-completion.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-8-manifest-contract-completion-evidence/creation/` | this directory (cites, measurements, raw JSON, nine prototype diffs) |
| X3 | `_bmad-output/implementation-artifacts/20-8-flag-winston-decision.md` | FLAG-Winston request, `PROPOSED` (KS / KB / K0 / KB4 priced) |
| X4 | `sprint-status.yaml:308` | `20-8-manifest-contract-completion: backlog` → `ready-for-dev` with the CREATED 2026-11-03 note; the prior note kept after `WAS: backlog`; `last_updated` → 2026-11-03 |
| X5 | `deferred-work.md:1350` (post-edit `:1351`) | sub-bullet "Mapped at 20-8 creation" under the received D-18-7-11 A row; no owner change |
| X6 | `deferred-work.md` (new section at the end) | `[explanation_shape]` has no carrier frame. Ownerless and trigger-owned under D-20-8-05 A; a B/C ruling re-points it to this key |

## §A — epic corrections applied in place to `epic-20-ship-it-w5.md` (OLD → NEW; C-numbers in `cites-8c809f9c.md`)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:81` (stories row) | "FR8 · kernel-Δ measured FLAG-Winston" → created/priced row: KS +38/+33 (KB, KB4, K0 priced), cumulative 21158 ≤ 21209, measured crates, PROPOSED ceilings, L1/L2, after 20-1a (C7) |
| A2 | `:258` (amendment) | `manifest.rs:228-237,2423-2454` struck → fields absent everywhere; ranges are `ClassSection` `:223-238` and `[providers]` `:2423-2455`; new sections after `:4101`, `POST_V1_SCHEMA_SECTIONS` `:336-337`; "After 20-1" = 20-1a (`install.rs:109-150`); signed install path already verified; missing = journaled record (`admission.rs:63-117`); story pointer (C1, C3, C4) |
| A3 | `:261` (AC2) | `deferred-work.md:1210-1216` struck → `:1342-1350` (C2) |

## §P — preflight, audit, inventory (applied)

| # | File:line | Edit |
|---|---|---|
| P1 | `epic-20-preflight-2026-10-30.md:82` (kloc table) | "No numeric forecast" → measured/forecast figures + PROPOSED ceilings (D-20-8-12 A) |
| P2 | `:126` (kernel/hosted table) | "After 1 … Yes; unmeasured" → after 20-1a (D-20-8-02 A), KS measured, ceiling, cumulative, hosted jobs, D:1342-1350, D-20-8-05 A label, 20-9 contract pointer |
| P3 | `:242` | appended "Re-measured 2026-11-03" (C1, C4) |
| P4 | `:243` | D:1210-1216 → D:1342-1350 (C2) |
| P5 | `routing-audit-2026-10-22.md:22` | dependency → 20-1a; FLAG figure; "unsigned admission" qualified (C3, C4) |
| P6 | `routing-audit-2026-10-22.md:153` | FR8 evidence re-measured; created + PROPOSED envelope |
| P7 | `epics/requirements-inventory.md:593` | created note, consumers, label, WIT 2.3.0, KS, PROPOSED envelope |

## Records owed after the operator rules (not now)

- Story: replace "PROPOSED" with the ruled option per row and rewrite only the clauses marked with that row. Frontmatter `kernel_grant` / `kloc_grant` → RULED text with granted ceilings; change log.
- `20-8-flag-winston-decision.md`: status `GRANTED`/`REFUSED` with ask id. Under D-20-8-07 B/C/D, rewrite AC8 and the ceilings to that row's priced figure (KB +10/+8; K0 0, with the 20-9 contract point 4 owner changed; KB4 +34/+32).
- Tracker `:308` and E20 `:81` add a `# RULED …` line (the 20-4/20-10/20-11 pattern). Preflight `:82`, `:126`, RA `:22`, RI `:593` replace PROPOSED with the ruled ceilings.
- Under D-20-8-02 B, the start gate also waits for `20-1b`. Under C, the tracker note states the parallel integration owner.
- Under D-20-8-05 B/C, the X6 row's owner becomes this key and AC5 becomes a control with a plant.
- Under D-20-8-06 B, I13's "v0.5 runtime" sentence is labelled false in `docs/invariants/I13.md` and `crates/maos-domain/src/invariants/i13.rs:9-12` (rule 11) instead of becoming true.
- Under D-20-8-10 C, the received row stays open and AC6/AC7 move to 20-9 (its tracker note and §Contract for 20-9 point 1 change).

## Records owed at landing (not now)

- `xtask/kloc.toml` rows: `:415` (maos-manifest), `:517` (maos-bin), `:703` (maos-wasm-host), `:425` (maos-domain), `:406` (maos-iac), `:693` (maos-skill), `:339` (xtask), `:264` (kernel, L2), `:718` (aggregate, measured + 1). Exact after `cargo fmt --all` / `kloc-check --json`, with HISTORY rows whose first segment cites `20-8-manifest-contract-completion` with `->` (D11 A).
- `xtask/kernel-core-baseline.toml` src_lines + content hashes of `scheduler/control_block.rs`, `lifecycle/upgrade.rs`, `hot_swap/coordinator.rs` + set hash (L2).
- `xtask/abi-ratifications.toml`: `20-8-manifest-schema-7`, `20-8-wit-spirit-2.3.0`. `STABILITY.md` regenerated; `docs-site/abi/v1` via `gen-abi-docs`.
- `tests/coverage-matrix.yaml:922-928` (FR8), `:656-666` (FR39), `:834` (FR57).
- `deferred-work.md:1344-1350` → `CLOSED <date> by 20-8 @<commit>` with witness.
- E20 `:81` and tracker `:308` → in-progress/done notes; `docs/invariants/I13.md` enforcement text (AC4).
