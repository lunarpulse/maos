# 20-3c creation — records owed

Rulings: none yet for this story's rows. D01 A / D02 A / D11 A (operator, 2026-10-30) govern; D05, D07, D10, D13 and
D-20-3c-01…11 are PROPOSED. Line numbers are at `3341f9e4` unless marked "post-edit".

## Applied at creation (Create203c owns these files this session)

| # | File:line (post-edit) | Edit |
|---|---|---|
| A1 | `_bmad-output/implementation-artifacts/sprint-status.yaml:300` | `20-3c-audit-truth-and-halt-receipt-residuals: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |
| A2 | E20 `epic-20-ship-it-w5.md:180` | member 11 cite `main.rs:4489-4494,5859-5872,6025-6040` → the one-shot arms `:7486-7490,7644-7648,7861-7865,9459-9463` (shell/run drains named as such); "per-arm unmeasured" → measured 4/4; E19 recurrence record named (`19-1-…md:329`, `19-7-…md:297`); `operator_door.rs:362-372` mechanism added (C11, C13, T1, T3) |
| A3 | E20 `:182` | `deferred-work.md:1130` → `:1194`; `:3,1202` → `:64,1268`; `:933` → `:996` (D1–D4) |
| A4 | E20 `:192` (AC7) | `delegation.rs:273,384,484,553` → `:455,584,713,782`; `:820` → `:1133-1159`; `adapter.rs:324-327` → `:324-328`; `transparency_log.rs:350-351` → `:351-352`; `deferred-work.md:1070` → `:1062`; `:1171` → `:1166` (C19, C20, D5, D6) |
| A5 | E20 `:194` | `xtask/kernel-core-baseline.toml:27-60` → `:27-28` + EOF (`:505`, `:915`) and `xtask/kloc.toml:264`; "figure comes from a measured prototype" → the measured figures (C21) |
| A6 | E20 `:75` (stories row) | appended creation note: created 2026-10-31 @3341f9e4, kernel priced R +12/+26, live members 11/12/16/17 + received items |

`deferred-work.md` is **not** edited at creation: the member-5 reconciliation (D:932) and every closure are D05-dependent
(owed below), and the gate counts live owner assertions.

## Records owed after the rulings (not now)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log.
- FLAG-Winston decision record for the ruled kernel bundle (precedent `17-4-flag-winston-decision.md`,
  `18-6-flag-winston-decision.md`), stating ADR-069's remaining room (`ADR-069…md:35-46`; R reaches 21018, under 22000).
- Under D05 A: `deferred-work.md:932` → `CLOSED by 16-6, measured at 20-3c creation` (cites in the story's member
  inventory row 5). Under D05 B/C: the story's AC1 changes, not the ledger.
- Under D07 A/B: hand the partial-mint observation (`cites-3341f9e4.md` T8; `schedule_watchdog.rs:216-232,326-342`) to
  20-9's creation, not to this story.
- Under D10 A/D: the 20-3a evidence row (`deferred-work.md:1022`) gains a "re-reproduced at 20-3c creation" sub-bullet
  citing `runtime/endpoint-reproduction-3341f9e4.json`.
- Under D-20-3c-01 A/B: `docs/invariants/I14.md` owner — the operator names the receiving open key (20-9 recommended).
- Under D-20-3c-06 B/C or D-20-3c-11 C: PRD DELTA rows (`functional-requirements.md:98`, `non-functional-requirements.md:164`).
- Under D-20-3c-11 B: a tracker lane for the production RPO/RTO cadence proof (operator-backed, `ops-*` naming).
- Tracker `sprint-status.yaml:300`: append the ruling ask id.

## Records owed at landing (not now)

- `xtask/kloc.toml:264` (kernel, L2), `:265` (maos-capability, L2), `:517` (maos-bin), `:425` (maos-domain), `:489`
  (maos-cli), `:409` (maos-audit), `:691` (maos-shell), `:406` (maos-iac), `:339` (xtask), `:718` (aggregate, measured
  + 1) — exact after `cargo fmt --all` / `kloc-check --json`, HISTORY rows citing this key.
- `xtask/kernel-core-baseline.toml:505` (+ HISTORY, content hashes `halt/mod.rs`, `hot_swap/precheck.rs`,
  `inference/mod.rs`); `docs/ci-baselines/kernel-surface-v0.1-beta.json` only if `check-service-boundary` reports the
  precheck parameter rename.
- `deferred-work.md:10,26,64,954,955,996,1022,1062,1194,1268` → `CLOSED <date> by 20-3c @<commit>` with the witness,
  or rule-10 re-owned; `routing-audit-2026-10-22.md:46` → owned/witnessed by 20-3c @<commit>.
- `tests/coverage-matrix.yaml:862-867` (FR62 witness); ADR-045 §4 (`:36-40`) per D-20-3c-05;
  `tests/integration/audit_9_1_headline_smoke.sh:181-194` per D-20-3c-06; `docs/invariants/I14.md:4-10`.
- E20 `:75` stories row → done note; `epics/index.md` only if the owner mirrors story status there.

## Applied 2026-11-01 (rows203c, grant203c)

- Story `20-3c-audit-truth-and-halt-receipt-residuals.md`: status lines 8, 9, 15, 131, 148, 150; decision rows 156–170; AC2.6 I14 owner line 190; change-log entry after line 392.
- `deferred-work.md:932` (member 5): closed by `16-6-maosctl-load-over-the-door`; the `Owner:` line removed.
- `deferred-work.md:1023`: member 16 sub-bullet (re-reproduction evidence and D10 A); no owner change.
- `sprint-status.yaml:300`: 20-3c `in-progress` with DEV-START note; `:341` `ops-rpo-rto-cadence-proof` operator lane (D-20-3c-11 B).
- `docs/invariants/I14.md:5,10`: owner `20-9-spawned-form-lifecycle-hooks`; claim `asserted` unchanged.
- `epic-20-ship-it-w5.md`: `:7` ruling record; `:75` row in-progress; `:196` ruled paragraph; `:360` D07 status; `:365-366` two bullets.
- `20-3c-flag-winston-decision.md` (new): R = K1p + E3; grants; ADR-069 room; landing protocol.

Not applied (landing items, open until L1 and L2): `xtask/kloc.toml` pins; `xtask/kernel-core-baseline.toml:505`; the deferred-work landing lines (10, 26, 64, 954, 955, 996, 1022, 1062, 1194, 1268); `routing-audit-2026-10-22.md:46`; `tests/coverage-matrix.yaml:862-867`; `tests/integration/audit_9_1_headline_smoke.sh:181-194`; ADR-045 §4; `docs/invariants/I14.md:4-10` claim text; epic `:75` kernel-Δ column.
