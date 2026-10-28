# 19-5 creation — records owed (not applied here; the parent applies them)

**Rulings 2026-10-28 (Lunarpulse; asks `w3_19_5`, `w3_19_5_d10`, `w3_kloc`):** D-19-5-01…09, 11, 13, 14 as recommended (07 B, the rest A); D-19-5-10 **C** (operator-approved scope cut → T2 applies); D-19-5-12 superseded (maos-bin 31353 → 31710, xtask 47624 → 47688, aggregate hardfail 191248 → 191669). Every conditional below resolves to its ruled branch.

**Applied 2026-10-28 (Create19x5, at the parent's instruction, after Create19x4's epic/tracker edits; no landing items):** R1 + T1 → `sprint-status.yaml:291`; T2 → `:306`; T4 no-op (`last_updated: 2026-10-28` already, `:1`); R2–R4 → `routing-audit-2026-10-22.md:20,173,175`; R5 → `deferred-work.md:97`; E3 → epic `:34`; E4 → `:130`; E2 → `:155`; E5 → `:159` and Dependencies `:232`; E1 + P1 → new epic §19-5 `:211-216` (before §19-6, now `:218`). X1 NOT filed — handed to the parent as an open routing item. Landing rows below remain owed.

This creation pass wrote only the story record and this `creation/` directory. Every edit below is owed to a
tracked file the creator may not touch. Line numbers are at `61af7461` (the tracker, epic, audit, inventory and
deferred-work files carry no uncommitted edit).

## Cite corrections (OLD → NEW, rule 9)

| # | File:line | Old text | New text |
|---|---|---|---|
| R1 | `_bmad-output/implementation-artifacts/sprint-status.yaml:291` | `` (16-1…md:478-486) `` | `` (16-1…md:303-309, was :478-486 @3a7c4260 — moved by the 2026-10-27 archive split d614eb9b; corrected at 19-5 creation @61af7461) `` |
| R2 | `_bmad-output/planning-artifacts/routing-audit-2026-10-22.md:20` | `` `16-1…md:43-46,478-486` `` | `` `16-1…md:43-46,478-486` (→ `:43-46,303-309` @61af7461, 19-5 creation) `` |
| R3 | `routing-audit-2026-10-22.md:173` | `` `16-1…md:478-486` `` | `` `16-1…md:478-486` (→ `:303-309` @61af7461) `` |
| R4 | `routing-audit-2026-10-22.md:175` | `` `16-1…md:43-46,484-486` `` | `` `16-1…md:43-46,484-486` (→ `:43-46,307-309` @61af7461) `` |
| R5 | `_bmad-output/implementation-artifacts/deferred-work.md:97` | `` Story 5.5x tracks the fix. `crates/maos-kernel-core/src/lifecycle/upgrade.rs:131-171` `` | `` Owner: `19-5-cross-form-upgrade-and-cold-swap` (rule 10(a); "Story 5.5x" never existed) — cold-swap half by D-19-5-01, hot-swap ABI half by D-19-5-09. `crates/maos-kernel-core/src/lifecycle/upgrade.rs:139-233` @61af7461 `` — after the ruling; if D-19-5-09 B is ruled, the hot-swap half's owner is `20-8-manifest-contract-completion` instead |

Held (re-read, no edit): `16-1…md:43-46` (tracker `:291`, audit `:20`, `:175`); `05-fr1-20-scout.md:13` (tracker `:291`, audit `:174`); `requirements-inventory.md:591`; PRD `functional-requirements.md:39,40,43`; epic `:91` (R19-24, `sprint-status.yaml:291`).

## Premise correction filed for the operator (not applied)

| # | Where | Premise | Fact @61af7461 | Routed to |
|---|---|---|---|---|
| P1 | tracker `:291` "After 19-3 (the subprocess Worker is E19's form)"; routing audit `:20` "parity for the subprocess form" | the `[cli_wrapper]` Worker is FR10's subprocess Spirit form | ADR-060 `docs/adr/ADR-060-spirit-forms-by-trust-tier.md:27-31`: `[cli_wrapper]` is "a third manifest shape, not a form string"; the forms are `rust-inproc`, `subprocess` (compatibility token only, `:78-79`; the native subprocess host was never built, `:54-57`) and `wasm-component` (T2 runner). A Worker's SCB Spirit has no in-process behaviour (`maos-bin/src/supervision.rs:2324-2345` HEAD) and a vendor agent CLI exposes no snapshot/swap-in | **D-19-5-10** (the operator rules the destination) |

## Epic edits owed

| # | File:line | Edit |
|---|---|---|
| E1 | `epics/epic-19-founder-loop-w4.md`, insert a new section before `:211` (`### 19-6-…`), i.e. after §19-4's last line `:209` | `### 19-5-cross-form-upgrade-and-cold-swap — Cross-form upgrade and cold swap ◆ FLAG-Winston (scheduler)` · `*Closes · Δ:*` per the ruled rows (recommended: FR49 cold-swap + migrator policies over the door; FR11 archived-predecessor gate at admission with an admission-time archive record; FR10's in-process clause witnessed (tokens + working memory) and the ADR-020 decision tree reachable for admitted predecessors; FR10's spawned-form clause routed per D-19-5-10) · kernel-Δ **−41 physical / −43 tokei** under the recommended rows (K1 + K6 + K8, reverted prototypes @61af7461) · forecast maos-bin re-priced 186–201 (D-19-5-12) · `- **AC1 (command).**` = story AC1 verbatim after the ruling. The epic has no `### 19-5` section today (`grep -n '^### '`). |
| E2 | `epic-19-founder-loop-w4.md:155` (stories row) | append ` · created 2026-10-28 @61af7461 (D-19-5-01…14 <ruling>); kernel-Δ −41/−43 under the recommended rows; forecast re-priced 186–201 (D-19-5-12)` — after the ruling |
| E3 | `epic-19-founder-loop-w4.md:34` (Kernel-Δ paragraph) | replace "19-5 ◆ FLAG-Winston (scheduler) is priced by prototype at its own creation (E16-A7)." with "19-5 measured by reverted prototype @61af7461 (E16-A7): recommended K1 + K6 + K8 **−41 physical / −43 tokei** (delete the kernel cold-swap arm; `check_abi_triple`; upgrade-target section defaults) — FLAG-Winston re-pin LOWER; kernel-positive alternatives K2 +21/+19, K3 +53/+48, K4 +73/+66, K5 +16/+13 each need their own grant (story record)." — after the ruling, with the ruled figure |
| E4 | `epic-19-founder-loop-w4.md:130` (§R kloc row 19-5) | `150 \| … \| analogue only` → `150 (re-priced at creation 186–201, D-19-5-12) \| …` |
| E5 | `epic-19-founder-loop-w4.md:159` and Dependencies `:225` | if D-19-5-10 C or D is ruled: "19-3 → 19-5" becomes "19-3 → 19-5 (file overlap only — `main.rs`; the Worker is not FR10's form, ADR-060)"; if A: unchanged |

## Tracker edits owed (`sprint-status.yaml`)

| # | Line | Edit |
|---|---|---|
| T1 | `:291` | after the ruling: `19-5-cross-form-upgrade-and-cold-swap: ready-for-dev  # CREATED 2026-10-28 at 61af7461; D-19-5-01…14 <ruling> — story: 19-5-cross-form-upgrade-and-cold-swap.md \| WAS: <current comment with R1 applied>` |
| T2 | `:306` (`20-9-spawned-form-lifecycle-hooks`) | only if D-19-5-10 C/D is ruled: prepend `⚠ RECEIVED 2026-10-28 from 19-5 (D-19-5-10 <C\|D>): FR10's spawned-form clause — an in-flight version swap of a wasm-component Spirit preserving state and tokens (WIT snapshot/swap-in exports; the successor factory refuses spawned forms today, maos-bin/src/main.rs:3331-3340 @61af7461) — charter match: lifecycle hooks reaching spawned forms. \|` |
| T3 | `:254` (`epic-16-retrospective`, done) | at landing only: annotate the cold-swap item ("cold-swap upgrade starts the successor under a new pid without admit_spirit … name an owner here" and the raw-map rollback census) `→ CLOSED by 19-5 @<commit>` |
| T4 | `last_updated` | today's date |

## Cross-story note (finding outside 19-5's charter — not decided here)

- **X1 — admission drops more than the swap sections.** `GatedManifest::bundle()` (`crates/maos-bin/src/admission.rs:404-412`) builds the SCB bundle from scheduling, lifecycle, class and budget only, so a class Spirit admitted by `maos run` / `maosctl load` also loses `[on_crash]` (FR50's declared disposition — the kernel reads `on_crash_action` from the SCB runtime, `crates/maos-kernel-core/src/hot_swap/coordinator.rs:308-317` shows the field set), `[on_revocation]` (FR13) and `[supervision]`. 19-5's D-19-5-06 A carries only the three ADR-020 swap sections. Owner by charter: FR50 → `19-6-crash-originator-delivery-and-disposition` (Workers already parse `[on_crash]` separately, `worker_spawn.rs:620-639` per its record) or a new row; FR13 → `20-1` (FR13 quarantine widening). Filed for the parent to route (rule 10).

## Records owed at landing (not now)

- `routing-audit-2026-10-22.md:174` FR11, `:175` FR49 → OWNED by `19-5` with the landing commit; `:173` FR10 → in-process half OWNED, spawned-form half per D-19-5-10.
- `requirements-inventory.md:591` — status note "closed by 19-5 @<commit>" (+ the routed half if D-19-5-10 C/D).
- `deferred-work.md:97` — CLOSED by 19-5 (or the hot-swap half re-homed per D-19-5-09 B).
- `xtask/kernel-core-baseline.toml:505` (+HISTORY) lower re-pin, content pins `:920`, `:954` (and `:899`/`:948` only if D-05 B/C), `xtask/kloc.toml:264` kernel lower re-pin, `:517` maos-bin, `:484` maos-control, `:489` maos-cli, `:718` aggregate — exact after `cargo fmt --all` / `kloc-check --json`, with the cumulative E19 line; `docs/ci-baselines/kernel-surface-v0.1-beta.json` + `xtask/kernel-api-classes.toml` (new `check_abi_triple` row).
- `tests/integration/smoke_upgrade_revoke_5.sh:16-23` — JSON-line count 4 → 3, and the step-2 cold-swap assertion `:31-35` removed (D-19-5-01 A removes the smoke arm's cold-swap step; the script is not in CI).
