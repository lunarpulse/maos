# 20-10 creation — records owed

Rulings in force: D01 A, D02 A, D11 A (2026-10-30); D09 A confirmed (ask s202). D-20-10-01…10 RULED exactly as
recommended by Lunarpulse on 2026-11-02 (ask `s2010`), including -09 B starts when picked. Original line numbers
below are at `a8af3215` unless marked "post-edit"; creation/ruling records are applied as accounted below.

## Applied at creation (Create2010)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-10-purge-residue-removal.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-10-purge-residue-removal-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/sprint-status.yaml:308` | `20-10-purge-residue-removal: backlog` → `ready-for-dev` with the creation note; prior note kept after `WAS:` |
| X4 | `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md:276-277` | C1, C2, C3 corrected in place; one provenance line after `:277` |

Applied 2026-11-02: `sprint-status.yaml` `last_updated` is `2026-11-02`.
No new deferral is required by the ruled -06 A / -07 A: docker detection and process-root leakage remain this
story's work, with the existing T0 STOP conditions. No implementation closure is claimed.

## §A — corrections applied to shared planning files (2026-11-02, ask s2010)

| # | File:line | Edit |
|---|---|---|
| A1 | E20:83 (stories row) | "FR2 · kernel-Δ 0 planned" → append "created 2026-11-02 @a8af3215 — ready-for-dev; kernel 0 under every recommendation (options KH/KR/KC/KS priced +10/+8, +2/−1, +5/+1, +3/+2); maos-bin +180…+350 forecast; owned cgroups are `maos.slice/spirit-*` leaves, not `/sys/fs/cgroup/maos` (C6, C8)" |
| A2 | S:308 / RA:24 / RA:162 | add: "the 16-4 report keys on the `maos-` name (never a cleanup authority, `argv.rs:16-21`); `/sys/fs/cgroup/maos` is never created at HEAD (C6, C7)" — S:308 is applied as X3's note; RA rows owed |
| A3 | PF:128, E20:277 | "idmaps" → state the reading used (C4) or the operator's intended meaning if different |
| A4 | PF:84, :247, :293 | as P1, P4, P5 |
| A5 | `20-7-stable-runner-perf-gates.md:281` (Overlap) | append "→ coordinated at 20-10 creation: purge consumes `TerminationUnconfirmed` as 'liveness unknown, CID/state retained' and never emits it (20-10 AC5.1)" |

## Records applied after the rulings (2026-11-02; original conditional instructions retained below)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log;
  frontmatter `kernel_grant`/`kloc_grant` to the granted ceilings.
- If any kernel option (KH, KR, KC, KS) is ruled: FLAG-Winston decision record (precedent
  `20-3c-flag-winston-decision.md`, `20-7-flag-winston-decision.md`) stating ADR-069's remaining room and the re-base
  against 20-1b; landing split L1 kernel-free → L2 kernel last.
- D-20-10-07 B: a `deferred-work.md` row for the process-root leak with the ruled owner (evidence `measurements.md`
  "Host probes"). D-20-10-06 B/C: a row for "T3 runtime detection offers docker; the T3 argv is podman-only" with the
  ruled owner.
- D-20-10-09 A: tracker note "starts after 20-2 done".

## Records owed at landing

- `xtask/kloc.toml:517` (maos-bin), aggregate hardfail (measured + 1) — exact after `cargo fmt --all` /
  `kloc-check --json`, HISTORY first segment `HISTORY: 20-10-purge-residue-removal … <old> -> <new>` (D11 ratchet form).
  Kernel pins (`xtask/kernel-core-baseline.toml` + EOF HISTORY, `kloc.toml:264`) only under a ruled kernel option.
- `docs/adr/ADR-067-maos-data-root-layout-and-purge.md:87-90` (Consequences) and `:92-97` (Verification);
  `docs/maos.dev/run-maos.md:105-121`, `docs-site/docs/run-maos.md:105-121`,
  `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md:102-113`, cookbook
  `docs-site/docs/cookbook/cli-wrapper-spirit.md:63` + ko `:64` (KO coverage 100).
- `crates/maos-bin/src/egress.rs:240-241` doc comment (state root no longer outside purge's inventory, D-03 A).
- `tests/coverage-matrix.yaml:408-415` (FR2) → the new tests; `.github/workflows/discipline.yml:2262`
  `EPIC17_TERMINAL_SUITE_TESTS` count pin and the `required_tests` list if scenes are added to `worker_terminal_facts_17_7`.
- RA:24, :162 → "owned/witnessed by 20-10 @<commit>"; E20:83 → done note.
- `deferred-work.md:13` (owner 20-7) untouched; a recurrence during 20-10's runs is recorded by name and run id.

## Ruling-record application — 2026-11-02

| Items | Resolution |
|---|---|
| A1 | Epic stories row records creation, ready-for-dev, ruled zero-kernel set, priced rejected options, bin forecast and real owned cgroup paths. |
| A2 | Tracker and both routing-audit rows identify the 16-4 name-prefix report as never cleanup authority (`t3/argv.rs:16-21`), and `/sys/fs/cgroup/maos` as never created at HEAD. |
| A3 | Epic and preflight spell out the [INFERENCE] "idmaps" reading: rootless user-namespace mappings, `/etc/subuid`, `/etc/subgid`, podman storage/config, never edited by purge. |
| A4 | Preflight pricing/report/workspace anchors and D09 A corrected. D-09 B supersedes the old after-20-2 sequence in epic, preflight, routing audit, requirements inventory, tracker, story status rationale and grand-plan addendum. |
| A5 | 20-7 overlap note records `TerminationUnconfirmed` = liveness unknown, CID/state retained; purge consumes it and never emits it (20-10 AC5.1). |
| Story / ruling branches | All 10 rows RULED with option/date/ask; grants/changelog retained. No kernel option ruled, so no FLAG-Winston record/landing split is owed; -06/-07 A create no alternate-owner deferral; -09 A wait note does not apply. |
| Received 20-11 R1/R4 | Purge-proof retention question routed openly to this story in `deferred-work.md`; uninstall cite corrected to `main.rs:7360-7399`, without silently deciding the new retention question. |

**Accounting:** all 5 §A items and applicable after-ruling records applied; 0 outstanding current recording items. Story/tracker remain `ready-for-dev`; landing code, docs, exact pins, tests and final ownership witnesses are still future implementation obligations.

