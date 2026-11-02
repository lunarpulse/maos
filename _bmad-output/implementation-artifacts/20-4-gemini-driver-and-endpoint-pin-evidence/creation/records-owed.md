# 20-4 creation — records owed

Rulings in force: D01 A / D02 A / D11 A (2026-10-30); preflight D06 D and D-20-4-01…11 RULED exactly as
recommended by Lunarpulse on 2026-11-02 (ask `s204`). Original line numbers below are at `a8af3215` unless
marked "post-edit"; they are provenance, not current offsets. Shared creation/ruling records are now applied
as accounted below. Story and tracker statuses remain `ready-for-dev`; no implementation closure is claimed.

## Applied at creation (Create204 owns these lines this session)

| # | File:line | Edit |
|---|---|---|
| X1 | `_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin.md` | new story, `ready-for-dev` |
| X2 | `_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin-evidence/creation/` | this directory |
| X3 | `_bmad-output/implementation-artifacts/sprint-status.yaml:302` | `20-4-gemini-driver-and-endpoint-pin: backlog` → `ready-for-dev` with the creation note; the prior note kept after `WAS:` |
| X4 | `_bmad-output/implementation-artifacts/deferred-work.md` (new section at the end) | one row: FR38 runtime provider-endpoint drift check is vacuous in production; PROPOSED owner `20-13-pluggable-crypto-provider-selection` (D-20-4-10 B); a different ruling re-points it |

Applied 2026-11-02: `sprint-status.yaml` `last_updated` is `2026-11-02`.

## §A — epic corrections applied to `epic-20-ship-it-w5.md` (2026-11-02, ask s204)

| # | E20 line | Edit |
|---|---|---|
| A1 | `:77` (stories row) | "FR3 (partial) · kernel-Δ 0 · maos-providers +300–400 · maos-bin +40–80" → append "created 2026-11-02 @a8af3215 — ready-for-dev; FR3 incl. Kimi + per-Spirit selection (D01 A); kernel FLAG-Winston R = KA + KIO1 + KIO2 priced +13 / +9 by reverted prototype (D-20-4-06/07 PROPOSED; 0 only if both rule C); measured providers +169, bin +145, domain +8; L1 kernel-free / L2 kernel (D-20-4-08)" (C17) |
| A2 | `:203` (Closes · Δ) | "kernel-Δ 0 · maos-providers +300–400 · maos-bin +40–80" → as A1 (C9, C17) |
| A3 | `:207` (AC1) | `openai_round_trip_test.rs:1-30` → `:15-29` (mock) and note a recording transport is needed (C1); "`anthropic.rs` (307 lines)" → 427 (C2); "`src/`, 1570 lines" → 1425 governed tokei / 1768 physical (C3); `main.rs:3081-3100` → `:3896-3963` (C4); add "replay proves selection and id routing, not the wire (C16)" |
| A4 | `:208` (AC2) | `manifest.rs:2103` / `:2159-2171` → `:2454` / `:2510-2516` (C6); `main.rs:3081-3100`, `:3136` → admission `crates/maos-bin/src/admission.rs:590-723,903-919` (C14); "governed by `xtask/fr63-typed-errors.toml`" → "typed `AdmissionRefusal` code; the FR63 catalog has no consuming gate (C8)"; "kernel-Δ 0" → "pin kernel-Δ 0; the story's kernel row is AC7/AC8 (C9)" |
| A5 | `:209` (AC3) | replace the hotfix-lane dependency text by: "closed by 17-5 (S:223, S:266); the gate must be extended — it cannot see a Gemini/Kimi constructor (`check_model_currency.rs:241-243,287-293`, measured C10)"; `sprint-status.yaml:294` → `:344` (C11) |
| A6 | `:205` (amendment) | `xtask/src/main.rs:446-449` → `:438` (C19); add "received D:1297, D:1298 (kernel IO, owner 20-4) (P3)" |
| A7 | `:380` (dependencies) | "`model-pin-currency-gate-and-retired-pin-sweep` … before 20-4" → "closed by 17-5"; `provider-model-env-overrides` → "per D06" (C22) |
| A8 | `:49` (ops row) | `ops-live-gemini-leg` "records the cassette" → add "after 20-4 L2 (record mode needs the kernel id set, C21)"; Kimi per D-20-4-09 |
| A9 | `:55` (kloc row) | "maos-providers +300–400 … against +400" → C18 figures |

## §B — ledger rows (applied, owners unchanged and rows remain open)

| # | Row | Edit |
|---|---|---|
| B1 | D:1297 | sub-bullet (orchestrator): "Mapped at 20-4 creation (2026-11-02): AC8.1, prototype `kio1-badheader-redaction.diff` (+7 / +5 kernel), D-20-4-07; the provider-side credential check is AC3.4." No owner change |
| B2 | D:1298 | sub-bullet (orchestrator): "Mapped at 20-4 creation (2026-11-02): AC8.2, prototype `kio2-non2xx-truncate-keep-status.diff` (+4 / +2 kernel), D-20-4-07." No owner change |
| B3 | new (applied, X4) | FR38 runtime provider-endpoint drift check vacuous — PROPOSED owner 20-13 (D-20-4-10 B) |

## §O — operator prerequisites (never faked; the story records `ABSENT` until each exists)

| # | Prerequisite | Needed by | Owner | State at creation (2026-11-02) |
|---|---|---|---|---|
| O1 | Paid Gemini API key in the operator's secret store (`MAOS_GEMINI_API_KEY`) | `ops-live-gemini-leg` (record the Gemini cassette, `provenance: live-record`) after L2 | Lunarpulse | absent: `docs/runbooks/provisioning-checklist.md:42` "no credential name exists in the tree" |
| O2 | Kimi (Moonshot, `platform.kimi.ai`) API key (`MAOS_KIMI_API_KEY`); keys are platform-bound (`api.moonshot.ai` vs `.cn`) | Kimi live leg (D-20-4-09) after L2 | Lunarpulse | not inventoried |
| O3 | Model-pin policy edit: `[providers.gemini]` / `[providers.kimi]` `allowed` ids and price-book rows, re-verified on the vendor pages on the day (D-20-4-03) | L1 (AC4) | Winston (pin policy, `xtask/model-currency.toml:7-9`) | candidates read 2026-11-02, not ratified |

## Records applied after the rulings (2026-11-02; original instructions retained below)

- Story: replace "PROPOSED" with the ruled option per row; rewrite only the clauses marked with that row; change log; frontmatter `kernel_grant` / `kloc_grant` to the granted ceilings.
- D06 ruling recorded in E20:7's RULED list; if D06 D: `sprint-status.yaml:224` note re-scoped ("`MAOS_DEFAULT_PROVIDER` moved to 20-4 (D06 D); the three `MAOS_*_MODEL` overrides remain") and `tracker-notes-archive-2026-09-04.md:713-715` cross-referenced; if D06 B: the row retired by explicit migration and this story's AC3 widened; if D06 A: AC1.3/AC2.3's selector waits for that row.
- Under D-20-4-09 A: `sprint-status.yaml:342` note widened (key unchanged); under B: a new `ops-live-kimi-leg` row and E20 operator-lane line.
- Under D-20-4-10 A: the X4 row's owner becomes this key; under C: re-labelled ownerless with its reason.
- FLAG-Winston decision record for the ruled kernel bundle (precedent `20-7-flag-winston-decision.md`), stating ADR-069's remaining room and the 20-1b re-base.

## Records owed at landing (not now)

- `xtask/kloc.toml:427` (maos-providers, pin 2000 kept if the ceiling holds), `:517` (maos-bin), `:425` (maos-domain), `:415` (maos-manifest), `:339` (xtask), `:264` (kernel, L2), `:718` (aggregate, measured + 1) — exact after `cargo fmt --all` / `kloc-check --json`, HISTORY rows citing this key with `->` (D11 A).
- `xtask/kernel-core-baseline.toml:505` + EOF HISTORY, content hashes of `inference/mod.rs`, `io/mod.rs` (L2).
- `deferred-work.md:1297`, `:1298` → `CLOSED <date> by 20-4 @<commit>` with witness.
- `tests/coverage-matrix.yaml:566-570` FR3 row names its gates/tests; `docs/maos.dev/run-maos.md:75` names the new keys and `MAOS_DEFAULT_PROVIDER`; the operator page states the pin's hash definition and its boundary (it locks configuration, not TLS identity); `docs/runbooks/provisioning-checklist.md:42` (operator, when O1 exists).
- E20 `:77` stories row → in-progress/done notes.

## Ruling-record application — 2026-11-02

| Items | Resolution |
|---|---|
| A1–A9 | Applied to epic: created/priced stories row; kernel R; recording transport, builder/manifest/currency/admission corrections; received IO rows; dependencies; L2-only Gemini/Kimi operator leg; measured kloc figures. |
| B1–B2 | Creation mapping sub-bullets added to the existing `deferred-work.md` IO rows: AC8.1/KIO1 +7/+5 and AC8.2/KIO2 +4/+2; provider credential check AC3.4; no owner changes or closures. |
| B3 / X4 | Existing FR38 runtime-context row now RULED to 20-13 under D-20-4-10 B; A/C re-routing branches do not apply. |
| Story | 11 story D-rows plus preflight D06 table row RULED with option/date/ask. Frontmatter grants and changelog retained; recommended content unchanged. |
| D06 D / tracker archive | Tracker lane retains the three `MAOS_*_MODEL` overrides; `MAOS_DEFAULT_PROVIDER` moved to 20-4. Archived four-variable charter has a dated re-scope cross-reference, not a rewritten historical claim. |
| D-20-4-09 A | Tracker and epic operator row widened to Gemini + Kimi live recordings after L2; key unchanged. No credential presence asserted. |
| FLAG-Winston | `20-4-flag-winston-decision.md` records R, grant ceilings, ADR-069 remaining room, cumulative STOP and later-lander re-base against 20-1b/20-11. |
| Preflight / grand plan | Measured pricing, nonzero kernel, D06 D, IO ownership and live-proof boundary reflected in the shared rows. |

**Accounting:** all 12 §A/§B items and all applicable after-ruling records applied; 0 outstanding current recording items. Conditional alternate rulings do not apply. §O inputs and §Records owed at landing remain developer/operator obligations, not completed implementation or pin updates.

