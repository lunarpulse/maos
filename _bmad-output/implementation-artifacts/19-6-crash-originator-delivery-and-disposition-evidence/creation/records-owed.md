# 19-6 creation — records owed (not applied here; the parent applies them)

This creation pass wrote only the story record and this `creation/` directory. Every edit below is owed to a
tracked file the creator may not touch. Line numbers are at `d9bd59c4`.

## Cite corrections (OLD → NEW, rule 9)

| # | File:line | Old text | New text |
|---|---|---|---|
| R1 | `_bmad-output/implementation-artifacts/sprint-status.yaml:292` | `` installs `NullReplicaResolver` (main.rs:2795-2796) `` | `` installs `NullReplicaResolver` (main.rs:2997-2998, was :2795-2796 @2026-10-22 and :2970 @f889b3e4/f34f6b4e — moved by 19-1/19-7; corrected at 19-6 creation @d9bd59c4) `` |
| R2 | `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md:91` (§R R19-24) | `` `NullReplicaResolver` `main.rs:2970`, was `:2795-2796` `` | `` `NullReplicaResolver` `main.rs:2997-2998` (was `:2970` @f889b3e4, `:2795-2796` @2026-10-22; corrected at 19-6 creation @d9bd59c4) `` |
| R3 | `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md:131` (§R kloc row 19-6) | `` replacing `NullReplicaResolver` (`main.rs:2970`) `` | `` replacing `NullReplicaResolver` (`main.rs:2997-2998`, was `:2970`) `` |
| R4 | `_bmad-output/planning-artifacts/routing-audit-2026-10-22.md:21` | `` `main.rs:2795-2796` `` | `` `main.rs:2795-2796` (→ `crates/maos-bin/src/main.rs:2997-2998` @d9bd59c4, 19-6 creation) `` |
| R5 | `_bmad-output/planning-artifacts/routing-audit-2026-10-22.md:177` | `` `main.rs:2795-2796` `NullReplicaResolver` `` | `` `main.rs:2795-2796` `NullReplicaResolver` (→ `:2997-2998` @d9bd59c4) `` |

Held (re-read, no edit): `16-3…md:35-37` (audit `:21,:176,:186`; tracker `:292`), `16-3…md:297` (audit `:186`), `requirements-inventory.md:592`, PRD `functional-requirements.md:41,44`, `non-functional-requirements.md:20`.

## Epic additions owed

| # | File:line | Edit |
|---|---|---|
| E1 | `epic-19-founder-loop-w4.md`, insert a new section before `:204` (`### 19-7-…`), i.e. after §19-4's last line `:202` | `### 19-6-crash-originator-delivery-and-disposition — Crash delivery to the task originator and declared disposition` · `*Closes · Δ:* FR12 (delivery to in-flight task originators), FR50 (all three declared dispositions applied in the J1 loop), NFR-Rel-1 (the ≤5 s floor measured on the delivered frame) — routed by the 2026-10-22 audit (`routing-audit-2026-10-22.md:21,176-177,186`; `sprint-status.yaml:292`) · kernel-Δ 0 under the recommended rows (D-19-6-01/05 B/C price +27/+24, ±0 byte-break, −1/−1 + maos-domain +11 — story record) · forecast maos-bin 100 (D19-1) re-priced at creation to 152–244 (D-19-6-10)` · `- **AC1 (command).** <the story's AC1, verbatim after the ruling>`. The epic currently has NO `### 19-6` heading (nor `### 19-5`); the stories row `:156`, order `:159` and Dependencies `:212` are the only anchors (`grep -n '^### '` → `:161,:171,:180,:188,:204`). |
| E2 | `epic-19-founder-loop-w4.md:156` (stories row) | append ` · created 2026-10-28 @d9bd59c4 (D-19-6-01…11 PROPOSED); kernel-Δ 0 under the recommended rows; forecast re-priced 152–244 (D-19-6-10)` — only after the ruling; if D-19-6-10 B is ruled, the envelope row `:40` and aggregate row `:48` take the raise the ruling names. |
| E3 | `epic-19-founder-loop-w4.md:34` (Kernel-Δ paragraph) | none if D-19-6-01 A and D-19-6-05 A are ruled ("19-6 ZERO" holds). If D-19-6-01 B is ruled: append "19-6 measured by reverted prototype @d9bd59c4: +27 physical / +24 tokei (`supervision/crash_detector.rs`), FLAG-Winston". If D-19-6-05 B/C: append the ±0 byte-break / −1/−1 + maos-domain figure. |

## Tracker edits owed (`sprint-status.yaml`)

| # | Line | Edit |
|---|---|---|
| T1 | `:292` | after the ruling: `19-6-crash-originator-delivery-and-disposition: ready-for-dev  # CREATED 2026-10-28 at d9bd59c4; D-19-6-01…11 <ruling> — story: 19-6-crash-originator-delivery-and-disposition.md | WAS: <current comment with R1 applied>` |
| T2 | `last_updated` | today's date, per the skill's step 6 |

## Records owed at landing (not now; listed so the dev pass does not forget them)

- `routing-audit-2026-10-22.md:176,177,186` — FR12 / FR50 / NFR-Rel-1 rows PARTIAL → OWNED by `19-6` with the landing commit; the 16-3 cut-line row (`16-3-…md:429`) is history and is not edited.
- `requirements-inventory.md:592` — status note "closed by 19-6 @<commit>".
- `xtask/kloc.toml:517,718` — exact re-pins (maos-bin, aggregate) with the cumulative E19 line; `:264`/`kernel-core-baseline.toml:505,971-972` only if a kernel option is ruled.
- `deferred-work.md` — no row is owned by 19-6 (`grep 19-6` = 0); rows the story files for its declared cuts (cross-host originator return; non-Spirit originators) per rule 10 at landing.

## Cross-story note (from Create19x2, 2026-10-28)

19-2's FR20 safe-point drain (D-19-2-02) runs only before a NEW section's first frame (gated on `epic_next` advancing); a 19-6 reassign re-dispatch of the same section is not a task completion and must not open it. 19-6 keeps `epic_next` unchanged on reassign (story AC3.2 guardrail). Both stories edit `main.rs:4853-5128` and `orchestrator_root.rs`: serialize the merges.
