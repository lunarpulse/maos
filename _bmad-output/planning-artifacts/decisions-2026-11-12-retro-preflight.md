# Operator decisions — 2026-11-12 Epic 20 retrospective and Epic 21 preflight D-rows

Recorded 2026-11-12 by the orchestrator's records session (`RecordRulings14`), not by the session that wrote the
retrospective or the preflight. Sources of the rows: `../implementation-artifacts/epic-20-retro-2026-11-12.md`
(§17 and Amendment A4, `D-E20-*`) and `epic-21-preflight-2026-11-12.md` (§7, `D-21-PF-*`). Earlier same-day
records stand unless named: `decisions-2026-11-12-dedrift.md` (DD), `decisions-2026-11-12-acceleration.md`,
`docs/release/decisions-2026-11-12.md`. Documents only: no source, workflow, pin, story file or git state is
changed by this record.

## How it was asked

The orchestrator showed the operator an ask form listing every row below as ID, short question and the
recommendation from the source document (the options texts stay in the two source documents). Form fields:
`ruling` (accept-all or per-row) and `overrides` (free text).

## Operator words

> ruling = accept-all
> overrides = (empty)

Meaning: every row on the form is ruled as the recommendation in the source document. Rows on the form:
D-E20-1, 2, 5, 6, 7, 8, 9, 10, 11 and D-21-PF-01 … PF-23 (PF-02 and PF-03 are the same questions as D-E20-11 and
D-E20-10). D-E20-3 was already resolved by the orchestrator (retro Amendment A4) and is not re-ruled here.
D-E20-4 was not put to the operator; see "Orchestrator decision" below.

## Ruled rows

Option letters and texts are those of the source documents. "Later change" says what must still be done; this
record does none of it.

| ID | Question (short) | Ruled option | Follow-up owner | Later code / workflow change |
|---|---|---|---|---|
| D-E20-1 | Who owns a production caller for the kernel T3 admission arm? | **A** — assign to `21-10-invariant-ledger-proofs` (I9 tier floors are its subject; kernel growth under `kfree` needs no ask) | `21-10-invariant-ledger-proofs` (closes E20-A5) | Yes: kernel/bin code in 21-10 |
| D-E20-2 | Owner of the 95 rows naming `ops-stable-perf-runner` | **B** — re-own only the two recurring flakes (`s3_external_sigkill…`, `ac4_v3b`/`nfr-rel-11` family) to a live hardening story or to 21-4, leave the rest | `orchestrator` (E20-A4); target story chosen when 21-4 is created | Yes: `deferred-work.md` Owner tokens of the two flakes change when the target is named; the other rows keep their owner |
| D-E20-5 | Ratify ABI row `20-5-cli-wrapper-schema-version` now or later | **B** — ratify before `v1.0.0` (the LTS clock starts there) | `orchestrator`; precondition of `ops-ga-tag-after-holds` | Yes: `xtask/abi-ratifications.toml:320-328` `proposed` → ratified before the GA tag |
| D-E20-6 | Review-model rule while GLM quota is out | **A** — GPT-family security review is standing, disclosed on every record; **B** (hold the landing until GLM returns) for any story that adds a new trust boundary | `orchestrator` (E20-A10) | No |
| D-E20-7 | Success-criteria definitions and thresholds | **B** — restate the FR threshold for GA after the first measurement (first measurement: 43/66 strict, 53/66 inclusive, retro A1); ratio reported under all three definitions | `orchestrator` (E20-A8). The restated number is not given in the ruling; the orchestrator files it in the E20-A8 result | No code; the lane's threshold text changes in a later record |
| D-E20-8 | What counts as "FR LIVE" | **A** — only built-binary tests count | `orchestrator` (E20-A8); `21-10-invariant-ledger-proofs` uses the same definition | No |
| D-E20-9 | Epic 21 re-preflight scope | **B** — GA set first (21-3, 21-6, 21-7, 21-8, 21-9, 21-10), the rest when picked | `orchestrator` | No |
| D-E20-10 = D-21-PF-03 | GA tag vs perf verdicts | **A** — extend ruling `rc` = b to GA: `release.yml`'s `ga` case also accepts ABSENT perf verdicts with the notes label (perf rows stay labelled, not measured) | `ops-ga-tag-after-holds` (post-rc; 21-9 also touches `release.yml`) | **Yes: `.github/workflows/release.yml:165-171` change, post-rc, not made by this record** |
| D-E20-11 = D-21-PF-02 | Hidden GA prerequisite 21-2 ("After 21-2" on 21-7/21-8) | **C** — drop both ordering edges; decide per story | `21-7-production-consent-granter-and-gate`, `21-8-three-protocol-session-witness` (each decides at creation) | No; the tracker clause is removed by this record |
| D-21-PF-01 | 21-9 in the GA set | **A** — confirm (21-9 stays in the GA set) | `21-9-wasm-host-default-on` | No |
| D-21-PF-04 | 21-11 in the GA set | **A** — not GA; release notes label NFR-Test-8/NFR-Onb-1 as not measured for human authors and report the agent trial under its own label | `21-11-agent-authoring-kit-and-agent-author-trial` | No |
| D-21-PF-05 | 21-1 cassette source | **C** — Ollama recording (provenance `live-record`, J4 closure MEASURED for that model) when a host with Ollama + `llama3.1:8b` is available at story time, else agent-authored with J4 closure labelled NOT MEASURED | `21-1-j4-incident-fixture-and-live-mira-nash` | Yes: 21-1 |
| D-21-PF-06 | Agent-authored provenance spelling | **A** — use the existing `seed` with `model_id = "agent-authored"` | `21-1-j4-incident-fixture-and-live-mira-nash` | Yes: 21-1 (the tracker's "`synthetic`" wording is superseded) |
| D-21-PF-07 | Witness of the record-flush success path after 21-2 removes the live re-record | **A** — a hermetic per-commit witness: record mode over the fixture driver (`fixture_replay` feature), flush, then strict replay of the written file | `21-2-nightly-postgres-and-ignored-journeys` (and 21-1 where it adds the record test) | Yes: new test; `journey-nightly.yml` live-leg removal stays with 21-2 (DD "Open work" 4) |
| D-21-PF-08 | `check-export-control` after default-on | **A** — invert: the default closure must contain the engine and the runner; the STABILITY export fence states ON and no ECCN determination | `21-9-wasm-host-default-on` | Yes: `xtask/src/check_export_control.rs` and its test clauses |
| D-21-PF-09 | Epic 20 exit line 9 asserts `wasm_engine_off` | **A** — 21-9 re-rules line 9 to the engine-on observation (the installed TS Spirit answers) and keeps the typed refusal as a witness on a `--no-default-features --features network` build | `21-9-wasm-host-default-on` | Yes: epic file, `discipline.yml:2962`, `registry_exit_20_1.rs:36` and the 26 `wasm_engine_off` sites |
| D-21-PF-10 | Release size bound 35,651,584 B with the engine in | **A** — re-measure the stripped default binary at landing; bound = measured value plus a stated margin, same commit | `21-9-wasm-host-default-on` | Yes: `xtask/src/prepush.rs:509` |
| D-21-PF-11 | Inherited-row scope for 21-9 | **A** — the epic §21-9 table: 25 ACs, 13 justified | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-12 | Telemetry listening (D:1684) | **B** — refuse a telemetry-listing manifest without an explicit scope | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-13 | Hook overrun cost (D:1696) | **A** — the link deadline ends the session (kill the running export) | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-14 | Successor scope check (D:1686) | **A** — hierarchical containment per `Scope` kind and re-derive `declared_sends` at handover | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-15 | Not-ready spawned swap (D:1677) | **A** — the door waits for readiness up to the pre-warm wall | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-16 | FR11 archive on hot swap (D:1685) | **A** — write a version record from the committed-outcome closure | `21-9-wasm-host-default-on` | Yes: 21-9 |
| D-21-PF-17 | Post-swap revert of a spawned Spirit (D:1676) | **B** — keep the typed end `swap_reverted_after_retire` (ruled design, fail-closed, witnessed) | `21-9-wasm-host-default-on` | No (existing behaviour stays) |
| D-21-PF-18 | Ledger write authority for 21-10 | **A** — 21-10 edits `docs/invariants/I*.md` (+ README routed-witness table) under `invariant-lock` (coverage-matrix touch + cadence hunk) | `21-10-invariant-ledger-proofs` | Yes: supersedes for 21-10 the standing "never edited by a story" rule (D:1659) |
| D-21-PF-19 | Property mechanism | **C** — hand-rolled seeded generators plus cargo-fuzz targets for the three `v1.0: fuzz` invariants (I1, I8, I11) on the existing fuzz workflows; no new property crate | `21-10-invariant-ledger-proofs` | Yes: tests and fuzz targets |
| D-21-PF-20 | GA bar for 21-10 | **A** — 14/14 invariants `guaranteed` | `21-10-invariant-ledger-proofs` | Yes: ledger flips on evidence only |
| D-21-PF-21 | Gate re-rule shape for 21-11 | **A** — one gate: `author_kind = "agent" \| "human"` per participant; agent trials judged under their own floors and label; human floors optional and never blocking | `21-11-agent-authoring-kit-and-agent-author-trial` | Yes: `xtask/src/check_third_party_trial.rs` |
| D-21-PF-22 | Agent-trial denominator and harness | **A** — N=12 blank-context runs across ≥3 harness/model combinations, transcripts committed, each Spirit re-verified by `check-trial-attestation` on a clean VM, floor ≥10/12 | `21-11-agent-authoring-kit-and-agent-author-trial` | Yes: 21-11 |
| D-21-PF-23 | `maos-spirit validate`/`inspect` stubs | **A** — implement `validate` (reuse the admission gate) as the kit's self-check; retire or implement `inspect` | `21-11-agent-authoring-kit-and-agent-author-trial` | Yes: `crates/maos-spirit-cli/src/bin/maos-spirit.rs:107-114` |

## Orchestrator decision (not an operator ruling)

**D-E20-4 revised from B to A.** The retro Amendment A4 resolved D-E20-4 as option B (flip `epic-20` with the
post-tag record commit). `check-epic-close-coherence` reds `stale-epic-key` once all 19 stories and the
retrospective are `done` while `epic-20` stays `in-progress`, so `epic-20` flips to `done` now (option A). The
`v1.0.0-rc.1` tag stays tracked by `ops-rc-signed-tag` under ruling 13 of `decisions-2026-11-12-acceleration.md`.
D-E20-3 (retro Amendment A4) is unchanged.

## Not changed by the rulings

- `check-decision-register` parses only `epics/epic-14-preflight-decisions.md` (`xtask/src/check_decision_register.rs:62`);
  these rulings are not rows of that register, so it is not edited.
- GA set (`ga3`) is unchanged: 21-3, 21-6, 21-7, 21-8, 21-9, 21-10 and `later-registry-quarantine-closure`.

## Applied by this record (documents only)

- `epic-20-retro-2026-11-12.md` §17 and Amendment A4, and `epic-21-preflight-2026-11-12.md` §7: each D-row marked
  RULED with date, option and a pointer here; option texts kept.
- `sprint-status.yaml`: `epic-20` `in-progress` → `done` (dated comment); the "After 21-2" ordering clause removed from
  `21-7` and `21-8`; ruling pointers appended to `21-1`, `21-9`, `21-10`, `21-11`.
- No workflow, crate, story file, gate, pin or `abi-ratifications.toml` change.
