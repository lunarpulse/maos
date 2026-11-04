# 20-5 creation — records owed

## Applied at creation (Create205; minimal targeted edits, each record re-read immediately before its edit)

| Record | Edit |
|---|---|
| `epic-20-ship-it-w5.md` | §A below (row `:78`, §20-5 `:213-218`) |
| `sprint-status.yaml` | `:1` `last_updated: 2026-11-04`; `:305` `20-5-v1-0-rc-tag-and-lts-rule: backlog` → `ready-for-dev` with a `📝 CREATED 2026-11-04 @73e7afbc` comment; prior comment kept after `| WAS:` |
| `epic-20-preflight-2026-10-30.md` | `:80` (§1.3 20-5 row) and `:126` (§2 readiness row): creation figures appended |
| `routing-audit-2026-10-22.md` | `:131` NFR-Sec-5: evidence column names this story's AC4 |
| `epics/requirements-inventory.md` | `:217` NFR-Maint-6: owner pointer to 20-5 AC2 |
| `deferred-work.md` | own rows `:107`, `:1175`, `:1186`, `:1209`, `:1298`: one "20-5 created 2026-11-04" pointer each (no closure, no owner change); a sibling (20-11) had an uncommitted edit at `:1396`, untouched |

## §A — E20 §20-5 OLD → NEW (applied in place; full evidence `cites-73e7afbc.md`)

| Where | OLD | NEW |
|---|---|---|
| Table row `:78` | "NFR-Maint-6 · kernel-Δ 0 · xtask +20–40" | + creation figures, D-rows PROPOSED, L1/L2 |
| *Closes · Δ* `:213` | "xtask +20–40" | kept as historical; creation measurement note added below it |
| Amendment `:215` | Stability CLI `main.rs:674-679` | `:619-625` (dispatch `:1406-1409`) |
| Amendment `:215` | stale wording `:221`, `STABILITY.md:59-60` | + `:350,:367`; the LTS sentence `stability_matrix.rs:215-219` / `STABILITY.md:53-57` contradicts NFR-Maint-6 (D-20-5-05) |
| Amendment `:215` | `release.yml:142-158` | `:152-174` (`prerelease` `:174`) |
| Amendment `:215` | `check_release_precondition.rs:48-78` | `:48-79` |
| AC1 `:217` | `xtask/src/main.rs:595` | `:619` |
| AC1 `:217` | `Cargo.toml:64`, `0.1.0-alpha` today | `:67`, `0.1.0-alpha.1` |
| AC1 `:217` | `release.yml` `:88-94` gets `prerelease … '-rc'` (an rc publishes as a full release today) | **closed:** `:174` already `contains(github.ref_name, '-')`; retained |
| AC1 `:217` | `container.yml` `:12-14` | `:11-14` |
| AC1 `:217` | `release.yml:75-77` | `:75,133-134` |
| AC2 `:218` | `docs/release/` today three files; `tag-procedure.md` is 15-4 AC5 | four files; `tag-procedure.md` exists, alpha-only |

## §B — planning-record observations (not edited; for the orchestrator / operator)

- Tracker `:305` cites `security/mod.rs:314-333` → `:279-289,525-573` (kept inside the `| WAS:` history; the new
  comment carries the correct cite).
- `ops-stable-perf-runner` / 20-7 note (`sprint-status.yaml:337`) says "O1–O5"; 20-7's own record now has O1–O7
  (`20-7-…-evidence/creation/records-owed.md:49-60`). 20-7's row, not edited here.
- `docs/release/tag-procedure.md:34` cites `release.yml:79-84` → `:117-122`; step 5 (`recovery-lane` merge) is
  history. Fixed by AC4.3 / D-20-5-15, not at creation.
- `deny.toml:21-32` say RUSTSEC-2021-0127 / -2025-0134 are "tracked as deferred work"; no row names either → AC7.
- `docs/runbooks/provisioning-checklist.md:38` says "rc precondition belongs to 20-5" — true; updated at AC4 landing.

## §C — routed observations (rule 10; destinations named, none silently dropped)

| Observation | Destination |
|---|---|
| The architecture's T3 tier is "24 h wall-clock on N=8 (≈ 192 CPU-hours)" (`5-spirit-abi.md:191`) while the collector requires Σ ≥ 86 400 libFuzzer seconds per target (`fuzz-prerelease.yml:323-324`) — the Q3 ruling (link count yields to 24 h) made the collector's rule; AC4 mirrors the collector and does not re-litigate it | operator (20-3a Q3 ruling stands); noted in D-20-5-07 evidence |
| After `v1.0.0`, `stability-matrix --check` diverges between clones and `git archive` copies (`stability_matrix.rs:352-368,508-509`) | this story, D-20-5-06 |
| `Family::Variant` collapse grew from 24 to 42 routes | this story, AC6 / D-20-5-11 |
| The 72 CPU-hour per-target pre-GA fuzz floor (`5-spirit-abi.md:193`) | GA lane (`ops-ga-tag-after-holds`); not an rc gate (story "Does NOT close") |

## Ruling records — updated 2026-11-04 (operator, ask d205)

- D-20-5-01…16 ruled as recommended: D-01 B, D-06 B, every other row A. Story D-row statuses, frontmatter `kloc_grant`, tracker, epic, preflight and deferred-work references updated. Grants bind at dev start with exact-pin re-basing; kernel-Δ 0.
- ABI ratification row (D-20-5-04 A) — the operator ratifies; the dev only proposes.

## Records owed at landing (not now)

- L1: `xtask/kloc.toml` rows + `_aggregate_hardfail`; `tests/coverage-matrix.yaml:1189-1195`;
  `docs/runbooks/provisioning-checklist.md:38`; `deferred-work.md:1186,1209` per D-11; `:1298` re-check line; new rows
  for RUSTSEC-2021-0127 / -2025-0134.
- L2: `deferred-work.md:107,1175` closures; `sprint-status.yaml:349` (`ops-rc-signed-tag`) prerequisites from
  `rc-contract.md` §2–§3; routing audit `:131`.
