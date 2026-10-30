# 20-3b creation — records owed

Rulings: none yet for this story's rows. D01 A / D02 A / D11 A (operator, 2026-10-30) govern; D03, D04, D08 and
D-20-3b-01…16 are PROPOSED. Line numbers are at `23f58da4` unless marked "post-edit".

## Applied at creation (Create203b owns these files this session)

| # | File:line (post-edit) | Edit |
|---|---|---|
| A1 | `_bmad-output/implementation-artifacts/sprint-status.yaml:299` | `20-3b-gate-retirement-and-coverage-generator: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |
| A2 | `deferred-work.md:903` (new continuation of `:902`) | deny half closed by 18-0 (`discipline.yml:157-189`, `aggregate.needs` `:6397`); clippy half open → AC6 / D-20-3b-11 |
| A3 | `deferred-work.md:911` (new continuation of `:910`, was `:909`) | CLOSED by measurement: lockfile rustls 0.23.46 / rustls-webpki 0.103.16 / chacha20 0.10.2 + blocking deny; advisory DB not queried |
| A4 | `deferred-work.md:1200` (new sub-bullet under `:1198-1199`, was `:1196-1197`) | cites re-measured (F1–F9) + measured kernel pricing; status unchanged (open) |
| A5 | `deferred-work.md:1224` (new continuation of `:1223`, was `:1220`) | cites re-measured; AC7 / D-20-3b-12 |

## Epic edits owed to the epic owner (EpicAmend this session; not applied here)

E20 = `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`. The D01 A amendment (2026-10-30) already carries
preflight §3.2; the rows below are the measurements the preflight did not have.

| # | Where | Edit |
|---|---|---|
| E1 | E20 §20-3b amendment (the "AMENDED 2026-10-30 … §3.2: For 20-3b AC4/5/7" paragraph) | append: "Measured at 20-3b creation @23f58da4 (story `20-3b-gate-retirement-and-coverage-generator.md`): enrolment closure is wider than six sites — also `prepush.rs:25-28,42`, `smoke-discipline-7-1-5` (`discipline.yml:3715-3720`) + maos-bin arm (`crates/maos-bin/src/main.rs:9214-9216,13836-13887`), aggregate script outputs, v1-0/v1-5 ship-gate needs, and two live gates importing four helpers of `check_dev_model_used_populated.rs` (`check_dev_model_tier.rs:19-21`, `check_dev_record_completeness.rs:476-478`); `aggregate.needs` has 133 entries (`:6394-6535`); measured gross retirement **xtask −5409 / aggregate −5411** (GO), −5316 (NO-GO), maos-bin −2; 33 coverage violations reproduced by replica; the cross-form job never runs its module (artifact `docs/cross-form/results/cross-form-results.json` absent). D-20-3b-01…16 PROPOSED." |
| E2 | E20 received §20-3b (the paragraph at E20@HEAD:280) | append: "Re-measured at 20-3b creation @23f58da4: `deferred-work.md:1198-1200` (was `:1252`/`:1196-1197`); `private.rs:263-278` (pid `:273`), read `mod.rs:779-830`, scan `:832-860`; the principal index and export are pid-keyed (`memory/principal.rs:16-26`, `mod.rs:921`) and forget skips non-numeric dirs (`private.rs:849`). ◆ FLAG-Winston row named: D03 + D-20-3b-05 — measured by reverted prototype K1 +1/+1, K2a +41/+33, K2b +90/+79 (+ maos-domain +1); recommended K1 + K2b **+91 physical / +80 tokei**; maos-bin +9. PROPOSED." |
| E3 | E20 stories row `20-3b-…` (E20@HEAD:64) | append " · created 2026-10-30 @23f58da4 (ready-for-dev; D03/D04/D08 + D-20-3b-01…16 PROPOSED); kernel-Δ 0 for deletion/generation, FLAG-Winston R′ +91/+80 requested for FR31; xtask measured gross −5409" |
| E4 | `epics/index.md` (epic 20 line) | none required (epic already `in-progress`); only if the owner mirrors story status there |

## Records owed after the rulings (not now)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log.
- FLAG-Winston decision record for the ruled kernel shape (precedent `17-4-flag-winston-decision.md`,
  `18-6-flag-winston-decision.md`), stating the remaining ADR-069 envelope (`ADR-069…md:35-46` item 3 only if the row
  crosses 22000 — it does not at 20698).
- Tracker `sprint-status.yaml:299`: append the ruling ask id.

## Records owed at landing (not now)

- `xtask/kloc.toml:339` (xtask, lowered), `:517` (maos-bin), `:425` (maos-domain, if D-05 B), `:264` (kernel),
  `:718` (aggregate, measured + 1) — exact after `cargo fmt --all` / `kloc-check --json`, HISTORY rows citing this key.
- `xtask/kernel-core-baseline.toml:505` (+ HISTORY, content hashes for `memory/{private,mod,principal}.rs`),
  `docs/ci-baselines/kernel-surface-v0.1-beta.json`, `xtask/kernel-api-classes.toml` (two new pub fns).
- `deferred-work.md:902-903` (clippy half), `:1198-1200`, `:1223-1224` → CLOSED by 20-3b @<commit>; 16-3 §11 row 15,
  15-1's group-hash filing, 15-5's `check-adr040-accepted` filing, D-17-4-PF-R2 debt → closed notes.
- `routing-audit-2026-10-22.md:45,113,133,144,145,155,157,158,161` → OWNED/witnessed by 20-3b @<commit> (or the
  labelled owner where a witness lands red, AC5.5).
- ADR-031 `:75`, ADR-040 `:5`, ADR-043 `:3`, ADR-045 `:60`, ADR-068 `:6`; `docs/testing/proof-contract.md:156,159,167`
  (with the code, not after).
