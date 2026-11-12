I wrote both files, and both gates still pass when run with the prebuilt binary. `check-decision-register` exits 0 (passed, 23 rows, 6 open, no findings). `check-epic-close-coherence` exits 0 (passed, 21 epics, pin 26062, no violations). Nothing else was touched: no cargo, no git changes, no workflow dispatch, and `intent-lineage-coverage-report.md` is unchanged.

**Files:**
- `_bmad-output/planning-artifacts/epic-21-preflight-2026-11-12.md` (new), measured at HEAD `582f9509`.
- `epics/epic-21-multi-host-on-live-substrate-w6.md`: three new story sections, a dated re-preflight paragraph, a dated "re-measured" note under 21-1, 21-2, 21-3, 21-4, 21-6 and 21-7, and the 21-9/10/11 table rows updated. The long acceptance-criteria lines are left word for word; corrections sit in the notes beside them.

**New section headings**
- `### 21-9-wasm-host-default-on — WASM host on by default`
- `### 21-10-invariant-ledger-proofs — I1–I14 invariant ledger proofs`
- `### 21-11-agent-authoring-kit-and-agent-author-trial — Agent authoring kit and agent-author trial`

21-9 has a table of all 38 `deferred-work.md` rows it owns (39 mentions of the key, counting the section header at `:1736`). 25 rows become acceptance criteria, including every security row (`:1667` the `write_atomic` temp-file issue, `:1668` the hostile `time_cap_seconds` guest). 13 rows ship without a fix, each with a stated reason; for example `:1763` cannot be reached from the default artifact.

**Order:** 21-9 first, 21-10 alongside it, 21-11 after 21-9, 21-1 last; the dependency graph is in the preflight as mermaid. One hidden GA prerequisite: the tracker orders 21-7 and 21-8 "after 21-2", but 21-2 is not in the ga3 GA set (D-21-PF-02).

**Open D-rows (operator decides)**

| ID | Choice | Rec. |
|---|---|---|
| PF-01 | Keep 21-9 in GA? It is the recorder's placement, not the operator's words | A: keep |
| PF-02 | 21-2 is a hidden GA prerequisite through 21-7/21-8: add it to GA / split it / drop the ordering edges | C: drop the edges |
| PF-03 | `release.yml:165-171` requires every perf metric to PASS for a GA tag, but the perf runner was dropped: accept ABSENT for GA / revive the runner / accept hosted-runner results | A: accept ABSENT |
| PF-04 | Is 21-11 a GA precondition? | A: no |
| PF-05 | 21-1 cassette source: record from local Ollama / agent-authored (J4 closure NOT MEASURED) / Ollama when available, else agent-authored | C |
| PF-06 | Provenance label for an agent-authored cassette: existing `seed` / add `synthetic` | A: `seed` |
| PF-07 | What replaces the removed live re-record as proof that record-and-flush works | A: a per-commit record-then-replay test |
| PF-08 | `check-export-control`: invert it / retire the leak check | A: invert |
| PF-09 | Epic 20 exit line 9 asserts `wasm_engine_off` | A: re-rule it to the engine answering |
| PF-10 | Release size bound with the engine in | A: re-measure at landing |
| PF-11 | Scope of 21-9's inherited rows | A: the table (25 criteria / 13 justified) |
| PF-12…17 | One ruling per inherited row: telemetry listener scope, hook overrun cost, successor scope check, not-ready swap, FR11 archive record, post-swap revert | B, A, A, A, A, B |
| PF-18 | Who may edit the `docs/invariants` ledger (20-9 skipped it as "never edited by a story") | A: 21-10 edits it under `invariant-lock` |
| PF-19 | How to write the property tests: `proptest` / hand-rolled / hand-rolled plus fuzz targets for I1/I8/I11 | C |
| PF-20 | GA bar for 21-10 | A: all 14 invariants proven |
| PF-21 | Shape of the re-ruled author gate | A: one gate with an `author_kind` field |
| PF-22 | Agent trial size | A: N=12 across ≥3 agent harnesses or models |
| PF-23 | `maos-spirit validate`/`inspect` are stubs | A: implement `validate` |

**Corrected cites (OLD → NEW)**
- **21-11:** `check_third_party_trial.rs:325` → `:538-543` (`participants_total < 12`); `:325` was never the N=12 line. Related floors: ≥10 successes `:453-456`, human strata `:463-499`.
- **21-1:**
  - `env_contract.rs:255,265` → `:382,387,392`.
  - `worker_spawn.rs:53-66` → `:88-139`. `--clock` now requires `--acceptance` (`:138`), so AC1's command as written would be refused.
  - Topology arms `main.rs:3797-3847` → `:5911-5960`; consent smoke `:10969-11040` → `:14213-14405`.
  - `journey_j4.rs:117` → `:116`; `frame.rs:386` → `:399`; PRD `:226` → `:227`.
  - Tracker's `synthetic` provenance → refused by the reader (`cassette_replay.rs:45-53` accepts only `seed`/`live-record`).
- **21-2:**
  - `discipline.yml:2930/2995/3077/3130` → `:5350/5415/5499/5577`; `:3054` → `:5476`.
  - `gate_common.rs:216-233` → `:239-250`; `check_reza_production_path.rs:456-457` → `:400-407`.
  - There are now three `#[ignore]`s, not two: the new one is `recording_source_15_6.rs:38`.
- **21-3:**
  - Test callers `:211` → `:210` and `:550,1252` → `:539,1241`.
  - `RuptureReason` `frame.rs:383` → `:397`; `RuptureRejection` `:359-377` → `:387`.
  - `router.rs:502` → `:496`; `build_server_config` `:1322` → `:1325`.
  - Cohort bootstrap `main.rs:9765` → `:12340`; `region.rs:97` → `:89`; `operator_config.rs:203` → `RegionSection` at `:167`.
- **21-4:**
  - `kloc_check.rs:163-190` → `:294-317`; `check_kernel_baseline.rs:99-110` → `:470-480`.
  - `env_contract.rs` is now 671 tokei / 709 physical / 125 entries (was 479/507/87).
  - `subcommands.rs:1938` → `maos-cli/src/door_client.rs:32`.
- **21-6 / 21-7:**
  - `memory/mod.rs:363,894` → `:367,853`; `adapter.rs:641` → `:750`; `mailbox.rs:792` → `:798`.
  - Tracker row `:316` → `:419`; deferred rows `1169-1171` → `1328-1330`.
- **Epic header:**
  - RELEASE-HOLDS `:59` → `:86`; 14-2 clauses `:76-110` → `:123-243`.
  - `demo-reza` `main.rs:1012` → `:1155`; operator row `:291` → `:438`.
  - Kernel pin 24472 → 26062 physical / 20429 tokei.

The 21-9 anchors in the tracker (`Cargo.toml:15,33`, `check_export_control.rs:194,310`, `prepush.rs:509`) are still correct. The two test clauses are (g) `:921-941` and (h) `:943-948`.

**Ollama recording:** record mode works with Ollama at HEAD. It only needs a cassette path (`inference_mode.rs:52-54`). It wraps every registered driver, including Ollama (`main.rs:4043-4055`, `:3962-3972`), and `maos run` flushes the recording (`:6078-6084`). Two limits: the model is fixed at `llama3.1:8b` (`main.rs:17735`), and recording only becomes possible once 21-1's own AC2 adds inference to Mira and Nash.

**Kernel forecasts** (method: read the touched files; tokei; the landing measures with `check-kernel-baseline` / `kloc-check --json`, re-pins with `scripts/repin.py` and writes the HISTORY row in the same commit)

| Item | Forecast |
|---|---|
| 21-1, 21-2, 21-3, 21-4, 21-8, 21-11 | 0 |
| 21-9 default-on core | 0 |
| 21-9 inherited kernel rows | +49…+143 |
| 21-10 | 0 expected; tests sit outside the pinned `src` set |
| 21-6 | +40…+120 |
| 21-7 | 0…+20 |
| quarantine closure | +30…+80 |

Worst case cumulative is about 20,792, below the 22,000 alarm, so no alarm or envelope D-row is needed.

**Kloc forecasts**
- **21-9:** maos-bin +0…+120 net; xtask −40…+30; maos-wasm-host +5…+40; maos-manifest +10…+25.
- **21-10:** 0, or xtask 0…+30 only if `invariant_lock.rs` learns the fuzz cadence.
- **21-11:** xtask +30…+90; maos-spirit-cli +60…+200 if `validate` is implemented.

Today almost every crate Epic 21 touches is at zero headroom; aggregate is 203354 against a hard-fail of 203355.

**Not measurable without cargo:**
- The stripped binary size with the WASM engine in, against the 35,651,584 B bound.
- Whether macOS and Windows compile `wasm-host`.
- Exact kernel and kloc deltas.
- Whether an Ollama recording completes all 50 incidents.
- Whether `cargo deny` passes with wasmtime in the default build.
- Whether any 21-10 property test finds an invariant broken.