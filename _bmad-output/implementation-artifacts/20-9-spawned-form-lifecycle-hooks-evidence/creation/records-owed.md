# 20-9 creation — records owed

Rulings in force: D01 A / D02 A / D11 A (2026-10-30); D07 A (2026-11-01, `rows203c`); D-19-5-10 C; R5; D-20-3c-01 A.
At creation D-20-9-01…16 were proposed. Line numbers are working-tree lines at creation (the same as `8c809f9c` for every row touched)
unless marked post-edit. They record provenance; they are not current offsets. Story and tracker status are `ready-for-dev`, and
no implementation closure is claimed.

**Rulings recorded 2026-11-03:** D-20-9-01…16 as recommended (ask `d_batch4`), including D-15 A STOP 21230 for 20-9 L2. D-20-14-12 C (ask `d2014_12`) sets joint STOP 21250 for the combined 20-9 L2 + 20-14 L2 cumulative; shared STOP 21209 otherwise, ADR-069 alarm 22000 unchanged. Kernel request binds at dev start; no landing claimed.

## Applied at creation (Create209, 2026-11-03)

| # | File | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks-evidence/creation/` | this directory: cites, measurements, raw JSON, ten prototype diffs |
| X3 | `_bmad-output/implementation-artifacts/20-9-flag-winston-decision.md` | FLAG-Winston request, `PROPOSED` (R / KD07A-lean / KD07B-lean / KD07A-inline / KO-poll / KO-count / KO-row / KW priced) |
| X4 | `sprint-status.yaml:309` | `backlog` → `ready-for-dev` with the CREATED 2026-11-03 note; the prior note is kept after `WAS: backlog`. `last_updated` was already 2026-11-03 |
| X5 | `deferred-work.md:1260`, `:1275` (post-edit sub-bullets `:1261`, `:1277`) | a "Mapped at 20-9 creation" sub-bullet under each owned row; no owner or status change |

## §A — epic corrections applied in place to `epic-20-ship-it-w5.md` (OLD → NEW; C-numbers in `cites-8c809f9c.md`)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:82` (stories row) | "kernel-Δ measured FLAG-Winston" → the created/priced row: R +16/+21, cumulative 21203 ≤ 21209 (×2 at 21224 → D-20-9-15), measured iac +39 and bin +14, PROPOSED ceilings, L1/L2, after 20-8 L1 |
| A2 | `:266` (amendment) | appended the creation pointer and the measured facts F1, F4, F9 |
| A3 | `:268` (AC1) | `main.rs:3388-3402` struck → `:3423-3432` (C1) |
| A4 | `:270` (AC3) | `deferred-work.md:1193,1204` struck → `:1260,1275` (C5); D07 "OPEN" read with the 2026-11-01 ruling (shape D-20-9-14) |
| A5 | `:367` | `schedule_watchdog_fr26.rs:505-542` → `:537-578`; `deferred-work.md:1193` → `:1260` (C5, C6) |
| A6 | `:368` | `deferred-work.md:1204` → `:1275` (C5) |
| A7 | `:369` | `first_id` `:326-342` → `:313-345` (C7) |

## §P — preflight, audit, inventory (applied)

| # | File:line | Edit |
|---|---|---|
| P1 | `epic-20-preflight-2026-10-30.md:83` (kloc table) | "No numeric forecast" → measured/forecast figures + PROPOSED ceilings (D-20-9-16 A); `S:306; E20:271-276` → `S:309; E20:264-270,362-370` (C9) |
| P2 | `:127` (kernel/hosted table) | "Yes; unmeasured" → R measured, ceiling, cumulative and D-20-9-15; T2 only (no T3 form); `S:306` → `S:309` (C10) |
| P3 | `:244` | appended "Re-measured 2026-11-03": `main.rs:3423-3432` (C1) |
| P4 | `:291` (D07) | appended the 2026-11-01 ruling's shape pricing (D-20-9-14) |
| P5 | `routing-audit-2026-10-22.md:23` | `17-4…md:51,135` → `:53,71,135,341` (C8); created + FLAG figure + kernel-free hooks (F1) |
| P6 | `routing-audit-2026-10-22.md:182` | FR55 evidence cite corrected (C8); created pointer |
| P7 | `epics/requirements-inventory.md:594` | created note, R, PROPOSED envelope |

## Records owed after the operator rules (not now)

- Story: replace "PROPOSED" with the ruled option for each row, and rewrite only the clauses marked with that row. Frontmatter `kernel_grant` / `kloc_grant` become RULED text with the granted ceilings; add a change-log line.
- `20-9-flag-winston-decision.md`: status `GRANTED`/`REFUSED` with the ask id. Under D-13 B/C/D or D-14 B/C, rewrite AC7/AC8 and the ceilings to that row's priced figure. Under D-13 C, the kernel-surface baseline re-issue (`docs/ci-baselines/kernel-surface-v0.1-beta.json`) and a classes row join AC7. Under D-13 B, the `kernel-api-classes.toml` row joins AC7.
- D-20-9-15: record the ruled STOP (joint 21250 with D-20-14-12 / 21230 / 22000 / split / none) in E20 `:82`, preflight `:127`, the grand plan's cumulative sentence (`maos-grand-plan-2026-10-22.md:147`) and the tracker. Under C, add the split key to the tracker and epic, then run `check-exit-commands` and `check-decision-register`.
- Tracker `:309` and E20 `:82` add a `# RULED …` line (the 20-4/20-10/20-11 pattern). Preflight `:83`, `:127`, RA `:23` and RI `:594` replace PROPOSED with the ruled ceilings.
- Under D-20-9-11 B, KW enters AC4 and the FLAG record (+9 / +6; `hot_swap/coordinator.rs` hash `:958`).
- Under D-20-9-12 B, AC5 becomes a typed refusal, and a deferred row "spawned cold swap" is filed (owner none, trigger: the first story that adds a door spawned surface).
- Under D-20-9-04 B, a deferred row "pause/resume not delivered to spawned forms" is filed, and FR55 stays PARTIAL in RA `:182`.
- Under D-20-9-07 C, "live telemetry" in E20 `:268` is labelled not delivered for spawned forms (rule 11).

## Records owed at landing (not now)

- `xtask/kloc.toml` rows: `:517` (maos-bin), `:703` (maos-wasm-host), `:425` (maos-domain, only if changed), `:339` (xtask), `:406` (maos-iac, L2), `:264` (kernel, L2), `:718` (aggregate, measured + 1). Each is exact after `cargo fmt --all` / `kloc-check --json`, with a HISTORY row whose first segment cites `20-9-spawned-form-lifecycle-hooks` with `->` (D11 A).
- `xtask/kernel-core-baseline.toml` `src_lines` `:505`, `scheduler/schedule_watchdog.rs` hash `:1005`, set hash `:939`, EOF HISTORY (L2).
- `xtask/abi-ratifications.toml`: `20-9-wit-spirit-2.4.0`. Fixtures re-issued; `gen-abi-docs`.
- `tests/coverage-matrix.yaml:351` (FR10), `:811` (FR55), the FR26 row; `docs/invariants/I14.md` routed-witness paragraph.
- `deferred-work.md:1260`, `:1275` → `CLOSED <date> by 20-9 @<commit>` with witness.
- RA `:173` (FR10 subprocess clause) and `:182` (FR55) close for spawned forms; RI `:566`/`:594`; E20 `:82` and tracker `:309` get in-progress/done notes.
