# 20-14 creation — records owed

At creation: D01 A / D02 A / D11 A (2026-10-30); D-20-14-01…12 proposed. Line numbers are at
`8c809f9c`; every shared-file edit below is an in-line substring edit (no line added or removed), applied by an
exactly-once scripted replace after a fresh read, so sibling creators' concurrent rows (20-8, 20-13) are untouched.
No implementation closure is claimed.

**Rulings recorded 2026-11-03:** D-20-14-01…11 as recommended (ask `d_batch4`), including -02 B and -03 B; D-20-14-12 C (ask `d2014_12`), joint STOP 21250 for the combined 20-9 L2 + 20-14 L2 cumulative. KT and ceilings remain measured requests, binding at dev start; no implementation or landing claimed.

## Applied at creation (Create2014 owns these files this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/20-14-flag-winston-decision.md` | new, `PROPOSED` (nothing granted) |
| X4 | `_bmad-output/implementation-artifacts/sprint-status.yaml:316` | `backlog` → `ready-for-dev` with the CREATED 2026-11-03 note; the prior note kept after `WAS: backlog  #`; `last_updated` already `2026-11-03` |

## §A — epic corrections applied to `epic-20-ship-it-w5.md` (OLD → NEW in `cites-8c809f9c.md`)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:87` (stories row) | appended the creation note: KT +35/+25 recommended, KM/KP/K0 priced, cumulative 21150 ≤ 21209, measured crates, envelope asked, L1/L2 |
| A2 | `:321` (Closes · Δ) | appended "Created 2026-11-03 @8c809f9c …", the HEAD failing-before observation and the pointer to the cites file |
| A3 | `:323` (amendment) | proxy `:383` → `:404-407`; emission `:424-427` → `:437-461` (`:448`); `cost.rs:57-61` → `:60-61`; `cli.rs:729` → `:746-763`; row `:4722` → `:4772-4782`; builder `:4790` → `:4842-4983`; pin at creation 26841/21042 (C1–C7) |
| A4 | `:325` (Why) | `cost.rs:248-266` → `:247-268`; `inference/mod.rs:375` → `:404-407`; `:406-426` → `:437-461`; `cost.rs:57-60` → `:60-61`; `cli.rs:667` → `:746-763`; `subcommands.rs:4614`, `:4622-4645` → `:4842-4983`, `:4850-4873` (C2–C8) |
| A5 | `:328` (AC2) | "Added-only, `abi-diff`" annotated (abi-diff reads only `maos-spirit-abi`, D-20-14-03); "set from the dispatched `task.assign`" annotated (no port caller is dispatched one, D-20-14-01/02); `inference/mod.rs:406-426` → `:398-461`; `cost.rs:248-266` → `:247-268` (C4, C8, C12, C13) |
| A6 | `:329` (AC3) | `cost.rs:57-60` → `:60-61`; `inference/mod.rs:375` → `:404-407`; `cost.rs:239-245` → `:238-245` (C2, C7, C9) |
| A7 | `:330` (AC4) | "the operator cost page" annotated: none exists, placement D-20-14-10 (C18) |

The historical filing pin at `:321` ("26361 physical / 20,594 tokei") is kept as provenance; `:323` carries the HEAD pin.

## §B — other planning records (applied)

| # | File:line | Edit |
|---|---|---|
| B1 | `planning-artifacts/epic-20-preflight-2026-10-30.md:87` (D02 envelope table) | "All figures by prototype …; no numeric grant" → creation figures and the PROPOSED envelope |
| B2 | `…preflight…:131` (kernel column) | "**Yes; unmeasured**, SCB marker + emission path" → measured KT +35/+25 recommended, KM/KP/K0 priced, PROPOSED ceiling, L2 last |
| B3 | `…preflight…:258` | `cost.rs:248-266` → `:247-268`; task-source and abi-diff findings (C8, C12, C13) |
| B4 | `…preflight…:259` | proxy `:383` → `:404-407`; emission `:424-427` → `:437-461` (`:448`); `cost.rs:57-61` → `:60-61` |
| B5 | `…preflight…:260` | `cli.rs:729` → `:746-763`; row `:4722` → `:4772-4782`; builder `:4790` → `:4842-4983` |
| B6 | `…preflight…:261` | appended: pin at creation 26841/21042, KT measured (PROPOSED) |
| B7 | `planning-artifacts/routing-audit-2026-10-22.md:69` (DECIDE-2) | appended the creation note and envelope |
| B8 | `…routing-audit…:130` (FR64 row) | appended the creation note and envelope |
| B9 | `planning-artifacts/epics/requirements-inventory.md:570` | appended the creation note and envelope |
| B10 | `…requirements-inventory.md:619` | appended the creation note, envelope and the task-source finding |

## §C — routed observations (owed when the operator rules; nothing invented here)

| # | Observation | Proposed destination | Why not now |
|---|---|---|---|
| R1 | Worker CLI spend through the egress proxy is never a cost frame (no usage parser, K10) | under D-20-14-06 A: a new `deferred-work.md` row naming the operator-assigned destination (proposed v1.1 cost-dimensions work, ADR-046 R6); under B this story | PROPOSED row; no E20 story touches the egress proxy (rule 10 has no (a)/(b) destination) |
| R2 | `deferred-work.md:91` NFR-Cost-1 "Ownerless and open" covers both halves | at `done` (AC6.2): the per-Spirit/task/principal clause closed by 20-14; the ≥ 98 % half per D-20-14-11 | closure follows landing |
| R3 | PRD DELTA-2026-10-25 text says the task reference is "sourced from an `InferenceRequest` correlation id and an SCB current-task marker" (`prd/project-scoping-phased-development.md:412`; NFR `non-functional-requirements.md:179`) | amend in place if D-20-14-02 B/C/D or D-20-14-03 B is ruled (PRD owner; the operator's ruling record) | the wording follows the ruling |
| R4 | ADR-046 `:15`, `:38-40`, `:79-81`, `:88` | AC5.2 at landing | implementation record |
| R5 | Sibling story `20-11-erasure-inventory-closure.md` cites the pid-reuse row as `deferred-work.md:1320`; at `8c809f9c` that row is `:1329` (`:1320` is an 18-4 closure note) | 20-11's record (orchestrator) | not this story's file |
| R6 | `tests/coverage-matrix.yaml:899-904`, `:1103-1108` | AC6.1 at landing | witnesses do not exist yet |

**Accounting:** X1–X4, A1–A7 and B1–B10 applied; R1–R6 recorded with destinations, none closed by this pass.
