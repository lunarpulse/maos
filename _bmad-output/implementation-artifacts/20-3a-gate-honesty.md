---
story_id: '20.3a'
story_key: 20-3a-gate-honesty
created: 2026-10-15
context_commit: 6537454e
baseline_commit: 6537454e4e2f5d21361b48c95638876051529e2f
non_degradable: true   # ◆ — the non-author §A6 review net (Blind + Edge + Acceptance + Test-Infra + runtime execution) is mandatory
kernel_grant: "NONE. kernel-Δ 0 — no file under `crates/maos-kernel-core/src` changes; `check-kernel-baseline` and `check-service-boundary` stay green."
kloc_grant: "RULED Q1 (2026-10-15, consensus) + S3 SIGNED OFF by the operator (Lunarpulse, 2026-10-15). `xtask` 44370/44370, `maos-audit` 7267/7267, `maos-bin` 25939/25939, `_aggregate_hardfail` 175373 vs measured 175372 — ZERO headroom on every row this story touches. Per-crate rows: exact measured raises under the eased RECOVERY-LANE CEILING RULE (`xtask/kloc.toml:60-69`: code exists, `cargo fmt --all`-measured, cites open `20-3a-gate-honesty`, figure + driver in the same commit) — forecast xtask +490…+1070 (rule-6 envelope ≤ +1390), maos-audit +20…+40, maos-bin +20…+40, maos-eval 0…+300 (AC11 probe code, if any). `_aggregate_hardfail` is NOT under the eased rule (`:106-115`); the operator has authorized in advance the strict measured+1 figure at landing (S3) — the landing commit records the exact figure, its per-crate drivers and this sign-off. No reservation, no slack."
depends_on: "none blocking start. `17-3a` (done) — the pull-forward trigger; `17-6` (done) — the three T2 suites; `17-1` (done) — the T3 masks; `15-3` (done) — single phase source, D20."
blocks: "`20-3b-gate-retirement-and-coverage-generator` (epic: a gate must be honest about its evidence state before a retirement decision is read off it); `20-3c-audit-truth-and-halt-receipt-residuals`; `20-7-stable-runner-perf-gates` consumes this story's PASS/FAIL/INDETERMINATE/ABSENT vocabulary (ADR-068 `:114-116`)."
---

# Story 20.3a: Gate honesty

Status: review

## Story

As **the operator who reads a green `aggregate` as "the controls ran and held"**,
I want **every gate and CI job to say ABSENT when its evidence is missing, INDETERMINATE when it could not decide, and to fail when a capable runner skipped it**,
so that **no green run, ledger row or signed attestation asserts something that was never measured.**

**Contract:** epic-20 §"20-3a-gate-honesty / 20-3b…" (split table: former AC1, AC2, AC3, AC6 + 15-5's `check-adr040-accepted` two defects) + the 2026-09-25 pull-forward paragraph (confidence rule 11's class; `check-j4-latency`; the bench "regression vs. the baseline" wording) + the 19 `deferred-work.md` rows owned by this key (§Owned residuals) + decision-register rows **D1** and **D9** + retro action **E16-A2**. The pre-split AC text (epic §20-3-gate-honesty-pass) is the source; where it disagrees with the tree at `6537454e`, §Current-tree corrections governs and the epic was amended in the same change (rule 9). **Q1–Q5 were ruled 2026-10-15 by consensus under the operator's decision rule, and S1–S3 were signed off by the operator the same day** (§Operator rulings) — nothing in this story waits on a decision. ◆ non-degradable. Owners: Murat + Winston.

**Does NOT close, and says so:**
- 20-3b's work: the generator spike, generated coverage matrix, the TEN retirements (incl. `check-cross-form-equiv`, `check-adr040-accepted`). Do not convert, repair or enrol a gate 20-3b deletes.
- 20-7's work: stable-runner timing, `perf-ledger`, and retiring the three `continue-on-error` perf jobs (`discipline.yml:2039`, `:2091`, `:2109`) — they stay advisory here and gain only an owner in their label (AC5.5; ADR-068 `:144-174`).
- Runtime-record truth (20-3c): receipts/journal rows. The one erasure attestation here (AC9) is 14-e1 / D5.3, filed to this key by `14-0` AC4.2.
- Making absent evidence present: no pen-test, cohort or RTO drill is produced (operator-lane rows). Fuzz CPU-hours accrue only through the restored cadence (AC3.4); this story does not wait for the 90-day window to fill.

## Current-tree corrections — read before implementing

Every epic and deferred-work cite was re-measured at `6537454e` by five scouts plus direct runs (creation log: `20-3a-evidence/creation-measurements-6537454e.log`). OLD → NEW:

| Epic / ledger premise | Fact at `6537454e` and consequence |
|---|---|
| **19-gate inventory**, 17 converted; `check-cna-registration` in it | `check-cna-registration` is **gone** (15-3 AC6, `gate-registry.toml:231-235`). Gates that still report **`"passed": true` over absent evidence (observed, exit 0)**: `check-pentest-gate` (`check_pentest_gate.rs:51-58,79-87`), `check-red-team-gate` (`check_red_team_gate.rs:109-135`), `check-third-party-trial` (`check_third_party_trial.rs:202-237`), `check-fuzz-floor` (`check_fuzz_floor.rs:71-85`; observed `0 s / 259200 s` → `passed:true`). Same code shape, not exercised at HEAD (fixture present): `check-skill-conformance` (`check_skill_conformance.rs:141-156`). Already honest prior art (`passed:false`, `skipped:true`, exit 0): `check-migration-merkle` (`:169-194`), `check-rto-gate` (`:57-84`) — D1's ruling names them the pattern. |
| Phase-advisory (6): RED → `Ok(())` at `v1_5` | `check-enterprise-pdp` (`:535-545`), `check-enterprise-identity` (`:622-625`), `check-fkcs` (`:341-345`), `check-wasm-form-equiv` (`:278-287`) **already exit 1 on RED** through `dev_enforced_red_blocks(Blocking)`. **One real half-migration:** `check-trial-attestation` computes `dev_blocks` (`:73-77`) and uses it for JSON `passed` (`:94`) but exits on `blocking_now` (`:135`) — at `v1_5` a RED oracle prints *PASS (advisory — oracle RED)* and exits 0 while JSON says `passed:false`. **`check-escape-detector`** emits `"passed": true` with `substrate_absent:true` on a RED (`:803-842`, literal at `:826`). `check-fkcs`'s D8 hold is deleted (D8 CLOSED 2026-09-29) — do not touch. |
| Ledger gates (4) map Absent → `Ok(())` off-lane; convert | **Not converted.** D2 (CLOSED by decision, register `:65`, ruling `:398-409`) ratified `passed` + `product_claim: NOT_PROVEN` + per-leg `ABSENT` + WOULD-HAVE-BLOCKED banner as the explicit claim boundary, and `ledger_enforced()` defaults to `GITHUB_ACTIONS` (`evidence_ledger.rs:131-143`) so CI enforces. 20-3a only adds the gate-level `evidence_state` field in `finish_ledger_gate` (`:1519-1605`) so every gate shares one top-level vocabulary. Re-opening D2 is out of scope. |
| `EvidenceState` lives in 7 files / 4 gates | Unchanged (`gate_common.rs:328-430`; four states; `EvidenceVerdict::project` `:405-419` is the only constructor). No fifth state is added — honest-N `preliminary` is a trial status, not an evidence state. |
| AC6: 40 `EXPECTED_GATES` ∪ 38 `[[ship_gate]]` = 42; 18 modules gain `BindingClass` | **42 ∪ 40 = 44** (`check_ship_gate_completeness.rs:20-120`; `gate-registry.toml:187-486`); symmetric difference unchanged (const-only `ccac-n600-ship-gate`, `nfr-rel-3-hsis-95pct`, `check-stability-matrix`, `check-breaking-md`; registry-only `fuzz-build`, `check-fuzz-floor`). `BindingClass` is in **18** `check_*.rs` (17 ship-gate). Workflow-only names are **9** (`ccac-n600-ship-gate`, `nfr-rel-3-hsis-95pct`, `check-ko-coverage`, `windows-check`, `check-live-bilateral-consent`, `check-mobile-push-on-halt`, `check-j4-latency`, `workspace-test-suite`, `fuzz-build`), structural 2, retiring 1 (`check-cross-form-equiv`) → **15 modules** without a class: pentest, third-party-trial, red-team, export-control, fuzz-targets, migration-merkle, `rto-drill` (**real module** `check_rto.rs:70-121`, not workflow-only), rto-gate, skill-conformance, loom-substrate-drift, env-contract, decision-register, exit-commands, model-currency, fuzz-floor. `ShipGateEntry` has no class field (`corpus_types.rs:84-87`). |
| "`EXPECTED_GATES` never cross-checked against `needs`" (E16 round 3) | **Half false.** Forward v1.0 check exists (`check_ship_gate_completeness.rs:212-223`, weekly exemptions `:130`, legacy `:143-148`, parser `:380-433`). Missing: registry → const reverse check, cadence-job reconciliation (`rto-drill` runs as `rto-drill-t1`, `rpo-rto-cadence.yml:39,77-80`), and aggregate → `v1-0-ship-gate` reachability (aggregate needs it directly; `v1-5-ship-gate` is in no aggregate). |
| D20 / E12-B1 close here | **Both already closed** (D20 by 15-3, register `:86`; E12-B1 `done`, sprint `:347-350`). Do not re-claim. |
| AC3: neither ledger branch exists; collectors warn + no-op; jq bug at `fuzz-cadence.yml:146` | **`rto-ledger` exists** on origin (`2ea6fcde`), **`fuzz-ledger` does not**. Collectors **create/seed** a missing branch (`fuzz-cadence.yml:135-136`, `rpo-rto-cadence.yml:118-119`) and turn a failed push into a warning (`:158-162`, `:130-134`); the downstream gate jobs skip on a missing branch (`fuzz-cadence.yml:181-198`, `rpo-rto-cadence.yml:153-170`). jq defect moved to **`:150`**: `.records += $rec[0]` is array + object (**reproduced: jq exit 5, `cannot calculate [] + {…}`**), and because it is the left side of `&&` under `set -e` the loop continues and the ledger silently gains nothing. Fuzz is 3 targets × 600 s × 4 workers weekly (`:58-96`), no chaining exists. |
| *(not in the epic)* fuzz evidence = what libFuzzer measured | **Drift from architecture §5.2** (`5-spirit-abi.md:185-197`: tiered T1/T2/T3 ladder, floor "non-negotiable", unit = "CPU-hours by libFuzzer counter, not wall-clock"). The record step writes the constant `cpu_seconds: 2400` and `corpus_size: 0` (`fuzz-cadence.yml:87-96`), the run is weekly 600 s × 4 (Epic-12 retro downgrade, `:12-17`, which itself proposes re-ratifying a *lower* floor), and no 24 h pre-release run exists although PRD NFR-Sec-5/6 (`non-functional-requirements.md:38-39`) make "24h `cargo-fuzz`, zero crashes" a v1.0 ship gate. Corrected by AC3.4 (Q3 ruling). |
| *(not in the epic)* T3 escape corpus | Each of the 25 scenarios declares a payload and an expected containment outcome (`t3-escape-corpus-v0/methodology-attestation.json`); 5 execute, 20 are labelled with owner `20-3a`. At least one declared expectation is not a host escape (`filesystem_escape_etc_passwd_001` reads the container's own file). AC11 (Q4 ruling). |
| `check-adr040-accepted`: registry name ≠ CLI name; in no workflow | **Name defect disproved:** CLI is `check-adr040-accepted` (observed in `xtask --help`), equal to `gate-registry.toml:34`; only the human banner says `check-adr-040-accepted` (`check_adr_040_accepted.rs:43`). No-workflow defect true. 20-3b AC7 **retires** the module — 20-3a repairs neither (§D-20-3a-G). |
| Rule 11 inventory @92911f59: 7 `continue-on-error`, 6 `\|\| echo` | **5 `continue-on-error`** (T3 pair removed by 17-1): `discipline.yml:74` (`reproducible-build`, job-level), `:1686` (`determinism-tests`), `:2039`/`:2091`/`:2109` (perf → 20-7). **4 `\|\| echo`**: `journal-append.yml:79` (**masks** PR-body fetch → revert pairing, `invariant_lock.rs:191-196,369-385,421-427`), `journal-append.yml:110` (benign display), `fuzz-cadence.yml:138` / `rpo-rto-cadence.yml:120` (file seeds; the RTO seed is an explicit `drill_success = false`). New masks found: `aggregate`'s failure predicate ignores `skipped` (`discipline.yml:5412`), while `v1-0-ship-gate` (`:4976`) and `v1-5-ship-gate` (`:5459`) fail on it; only `macos-check` has a job-level `if:` (`:4607`) and already has its own disposition; aggregate's advisory row reads `needs.determinism-tests.result` (`:5314`) — always `success` under `continue-on-error` — instead of the job's `test_result` output (`:1674-1675`); `set +e` grep scans at `:120` and `:592` treat a grep error (exit 2) as "no violation". 22 `set +e` / 16 `always()` / 6 `\|\| true` are classified benign (preserve rc or publish evidence) in §Rule-11 inventory. |
| "Suites that turn an environment refusal into `ok`" | Three T2 suites: **closed by 17-6** (panic when `CI` or `GITHUB_ACTIONS` is set). Still open: `maos-telemetry/tests/otel_gates.rs:493-535` (three air-gap tests turn *any* `network_syscalls` error — missing strace **or a failing target** — into `SKIP` + pass; helper `:104-140`); **`maos-kernel-core/tests/hot_swap_halt_continuity_test.rs:13-17` is a null control** — the path `crates/maos-eval/fixtures/…` is relative to the workspace root but cargo runs integration tests from the crate dir, so it **always** returns (the corpus exists at `crates/maos-eval/fixtures/halt-continuity-corpus-v0/scenarios/`); `maos-a2a-tcp/tests/support/mod.rs:817-819` (unreadable `/proc/self/limits` → silent return); root-capable runner returns: `maos-bin/tests/maos_uninstall_16_4.rs:1413-1421,1654-1663`, `maos-kernel-core/tests/private_forget_restart_13_5i.rs:298-305`. Two CI conventions disagree: T2 `CI.is_some() \|\| GITHUB_ACTIONS.is_some()`; T3 `tests/harness/t3_substrate.rs:6-29` `CI == "true"`; `t3_terminal_helpers_17_7.rs:266-277,461-464` `CI` only. |
| `check-j4-latency` blocking-in-name-only; `cargo tree … --release` [INFERENCE] | **Confirmed both.** `gate-registry.toml:348-349` `v1_5 = "blocking"` at `CURRENT_PHASE = "v1_5"`; the test prints *WOULD BLOCK* and passes (`maos-bench/tests/t_10_4c_j4_latency_gate.rs:76-102`). **`cargo tree -p maos-bench -e features --release` exits 1 (`unexpected argument '--release'`)**, so `discipline.yml:3669-3673` greps nothing and prints *D3 verified* — a null control. |
| Bench wording `discipline.yml:1024,1045` | Moved to **`:1047`** (`founder-loop-bench`) and **`:1068`** (`mira-nash-bench`); each runs `cargo bench … -- --test` once, no baseline exists. |
| Deferred `:548` revocation `unwrap_or(0)` signed as `Removed` | **Already fixed** — `revoke_all_for_pid` returns `usize` (`maos-kernel-core/src/capability/mod.rs:360-362`, infallible shard walk `maos-capability/src/cap_tokens/mod.rs:306-318`); caller `main.rs:9062`, signed at `:9158-9162`. Close on measurement; do not invent a `CoverageGap` for an unreachable error. |
| Deferred `:549` / 14-e1 `decommission_region_key` `completed: true` — "unreachable" | **Live and reachable.** `regional_teardown.rs:151-159` derives a pubkey and asserts `completed: true`; production calls it at `main.rs:9274` and signs a regional receipt (`:9275-9293`) on the region-pinned, no-hold uninstall path; `erasure_uninstall_13_5b.rs:311-340` asserts that receipt. The "shared coverage fails first" reason is obsolete (`regional_teardown.rs:50-74`, `UNCOVERED_STORES = []`). |
| Deferred `:1011` Windows ACP HOME | **Closed in code**: `jetbrains_acp_server.rs:33-51` records HOME was not the cause; `:75-136` assert the intended non-Unix refusal. Close with evidence. |
| Deferred `:1037` three T2 suites | **Closed by 17-6** (sprint `:263,457`; run `36425906424`). Close with evidence. |
| Deferred `:922` "three `?` callers" | Cohort mesh (`check_cohort_mesh.rs:702-727`) and multi-tenant loom (`:1896-1910`) now emit their own red report; **`check_epic_close_coherence.rs:80-85,124` still `?`-propagates before any JSON** (a pin read error leaves `--json` empty). |
| Deferred `:923` kernel-pin CI wiring `discipline.yml:2089,2106` | Moved to **`:2691`** (`fetch-depth: 0`), **`:2698`** (`components: rustfmt`), **`:2707-2708`** (named step). Wiring correct; no local pin. |
| Deferred `:1126` catalog drift: ten codes | Still exactly ten: TOML 49 codes (`xtask/error-catalog.toml:42-630`), JSON 39 keys (`docs/errors/error-catalog.json`); generator drops `cause`/`remediation` (`check_error_catalog.rs:484-499`, overwrite `:590-600`); no check compares TOML with committed JSON (`:514-570` is TOML↔AST; golden test `:1059-1060` compares two generated strings). |
| `D1`, `D9` deadline *before 20-3a leaves `backlog`* | Flipping this row to `ready-for-dev` reds `check-decision-register` (`check_decision_register.rs:31-34,419-421`) unless the clause moves. **D1** is ruled (register `:411-429`: out-of-ledger evidence contract, no ledger expansion) and implemented by AC1; **D9** is unruled (Q2). Both deadlines were **re-anchored at creation** to *before `20-3a-gate-honesty` reaches `done`* — clause REPLACED, not appended (parser splits on `;`, `:396-409`; precedent D3/D7, D-16-5-H). |
| Epic table: xtask +0–150 | Pre-split arithmetic for 20-3a's half alone is +750…+1220; re-measured forecast §Funding. Every touched row has ZERO headroom. |
| Local observation (not a defect of this story) | `check-multi-tenant-loom` exits 1 on the dev host: leg `maos-bin-whole-package-parallel-isolation` INDETERMINATE because `founder_loop_once_drains_capability_audit_writer` FAILED in a whole-package run. The gate is honest; the failure is unattributed — T0 re-runs it and, if it reproduces, files it with an owner (rule 10). |

## Acceptance Criteria

*(Epic former AC1/AC2/AC3/AC6 tightened against measurement, plus the pull-forward charter and owned residuals; each cites what it changes. "Planted" = a committed vector or a recorded command+output per rule 11(c).)*

### AC1 — command: absent is never a pass (former AC1, D1's out-of-ledger evidence contract)

**Given** HEAD with no `docs/pen-test/findings/summary.toml` (`check_pentest_gate.rs:46`), no `docs/red-team/results/red-team-results.toml` (`check_red_team_gate.rs:15`), no `docs/third-party-trial/results/trial-results.toml`, no `fuzz-ledger.json` at the workspace root (`check_fuzz_floor.rs:33,72`), no migration corpus and no RTO evidence — the state measured at `6537454e`,
**when** the operator runs
```
cargo run -q -p xtask -- check-pentest-gate --json; echo rc=$?
```
(and the same for `check-red-team-gate`, `check-third-party-trial`, `check-skill-conformance` with `docs/skill-conformance/results/skill-conformance-results.toml` (`:39`) removed in an isolated copy, `check-fuzz-floor`, `check-migration-merkle`, `check-rto-gate`),
**then** each prints one JSON object carrying `"gate"`, `"evidence_state":"ABSENT"`, `"passed":false`, `"advisory":true` and a `reason`, emits `::warning::<gate>: ABSENT — <reason> (not a pass)` on stderr, and exits **0** because its registry disposition at `CURRENT_PHASE` is `advisory`/`blocking-when-present` — never `"passed": true`.

1. **One contract, one home.** `gate_common.rs` gains a small out-of-ledger report helper that derives `evidence_state` **only** through `EvidenceVerdict::project` (`:405-419`); `passed == evidence_state.is_proven() && no blocking finding`. Exit rule: a RED (attempted, not green) leg blocks iff `dev_enforced_red_blocks(class, substrate_present)` (`:229-234`); ABSENT exits 0 + warning unless the disposition at `CURRENT_PHASE` is plain `blocking`, which exits 1. No fifth state; no gate-local copy of the rule.
2. **Converted call sites (exactly these):** `check_pentest_gate.rs:51-87`, `check_red_team_gate.rs:109-135`, `check_third_party_trial.rs:150-154,202-237`, `check_skill_conformance.rs:141-156`, `check_fuzz_floor.rs:71-85` (missing/unreadable ledger → ABSENT; a readable ledger younger than 90 days → INDETERMINATE advisory with its accumulated seconds, never `passed:true`), `check_migration_merkle.rs:169-194` and `check_rto_gate.rs:57-84` (their `skipped` field is **replaced** by `evidence_state`; every consumer migrated in the same change — grep workflows, `xtask/tests`, docs), `check_escape_detector.rs:803-842` (substrate-absent RED → `passed:false`, `INDETERMINATE`, advisory), `check_trial_attestation.rs:135` (exit follows `dev_blocks`, not `blocking_now` — a RED oracle exits 1 at `v1_5`), and `evidence_ledger.rs::finish_ledger_gate` (adds the gate-level `evidence_state` = worst required leg; exit semantics unchanged per D2).
3. **Workflows stop pre-empting the gate.** Jobs that branch on file presence and skip the CLI (`discipline.yml:3121-3126` third-party trial; `:3175-3192` migration merkle; any other found) always invoke the gate, which reports ABSENT itself — the YAML no longer decides evidence state.
4. **Proven red** (rule 11(c)): per converted gate, one vector in `xtask/tests/` that runs the real binary on an isolated tree: absent input → `passed:false` + `ABSENT` + rc 0; for trial-attestation a planted RED leg → rc 1 at `v1_5`; for escape-detector a substrate-absent RED → `passed:false`. Planting the old `"passed": true` literal back reds the vector.

### AC2 — honest-N: a preliminary cohort can be committed (former AC2)

`check-third-party-trial` derives `trial_status: "preliminary"` from a **validated** `participants_total < 12` (today an immediate `Err`, `check_third_party_trial.rs:325-330`): all integrity checks still run first and stay fatal at any N (dates/counts `:274-288`, declared ≤ N `:290-295`, enumerated count + unique IDs `:297-321`, derived == declared `:434-441`, canonical strata `:448-477`); only the sampling minima (N ≥ 12, successes ≥ 10 `:442-446`, strata floors `:478-499`) become advisory shortfalls. Report: `passed:false`, `advisory:true`, `trial_status`, N, derived successes, per-stratum counts vs floors, `evidence_state:"INDETERMINATE"`; exit 0 under `blocking-when-present` at `v1_0`/`v1_5`, exit 1 at `v2_0` (`gate-registry.toml:191-193` comment gains the `preliminary` case; v2.0 producer-key requirement `:253-271` unchanged). The floor policy carries a date in `docs/third-party-trial/README.md` and the schema (`results/trial-results-schema.toml`); no automatic date-based promotion is invented. Input path stays `docs/third-party-trial/results/trial-results.toml` (`:81`) — `ops-external-cohort-and-pen-test` commits there. **Proven red** (extend `xtask/tests/story_10_2_proven_red.rs`, keep the `trial_gate` filter that `discipline.yml:4374` runs): consistent N=3 → preliminary rc 0 at `v1_5`, rc 1 at `v2_0`; N=3 with a duplicate ID / inconsistent N / inflated successes → rc 1; N=12 with a strata shortfall → rc 1; an early advisory `return` planted before integrity checks reds the inconsistent-N vector.

### AC3 — ledger collectors fail loudly (former AC3 + 15-5's jq fix)

1. `fuzz-cadence.yml:150`: `.records += [$rec[0]]` (or `+= $rec`), as its own command — append failure fails the step; a test drives the collector shell body in a throwaway git repo (20-6's "execute the real script" convention, `xtask/tests/macos_compile_guard_20_6.rs:195-207`) and asserts one appended element.
2. Collectors **never create** a ledger branch (`fuzz-cadence.yml:135-136`, `rpo-rto-cadence.yml:118-119` removed): a missing branch fails the collector with `::error::` naming `ops-provisioning-secrets-and-accounts`; a rejected push fails (`:158-162`, `:130-134`); branch creation stays operator-lane.
3. Downstream gate jobs (`fuzz-cadence.yml:168-198`, `rpo-rto-cadence.yml:139-170`) always run the gate; a missing branch reaches the gate as ABSENT (AC1), not a job-level skip.
4. **Fuzz evidence is measured, not asserted (Q3 ruling — architecture §5.2 `5-spirit-abi.md:185-197`, PRD NFR-Sec-5/6 `non-functional-requirements.md:38-39`).** (a) Each record's `cpu_seconds` is the **libFuzzer-reported run time summed across workers** (each worker's `Done <N> runs in <S> second(s)` line, or `-print_final_stats=1`, on the pinned nightly), and `corpus_size` is the real corpus count — the hard-coded `2400` / `0` (`fuzz-cadence.yml:87-96`) go; a record with any worker missing its counter is refused, never defaulted. (b) The cadence is sized from the floor, never the floor from the cadence: the 72 CPU-h/target and 1,000 CPU-h aggregate 90-day floor stays (`check_fuzz_floor.rs:34-37`; lowering it is a major-version conversation, `5-spirit-abi.md:197`), the "re-ratify a lower floor" option is deleted from `fuzz-cadence.yml:12-17`, and the run job is restored to the architecture's **T2 nightly** (1×/day, 4 h wall-clock per target, workers = runner cores) — reverting the Epic-12 retro's weekly 600 s downgrade (**S1, signed off 2026-10-15**; the header comment records the reversal and its grounds). Arithmetic recorded in the registry comment (`gate-registry.toml:254-264`): with 4 workers, 90 nightly runs ≈ 1,440 CPU-h/target ≥ 72 and ≈ 4,320 aggregate ≥ 1,000; the bootstrap-advisory window and auto-promotion (`check_fuzz_floor.rs:10-16`) are unchanged. (c) **T3 pre-release, 24 h per target** (PRD NFR-Sec-5/6 "24h, zero crashes"; the epic's "chained ≤ 6 h jobs"): a `workflow_dispatch` + `v*-rc*` tag-triggered chain of four ≤ 6 h links per target, each link resuming the previous link's corpus artifact, zero crashes or the chain fails, one ledger record per link tagged `tier:"T3"` and the tagged SHA. The release precondition that refuses an rc without a green T3 chain for its SHA is **routed to `20-5-v1-0-rc-tag-and-lts-rule`** (rule 10(b): it owns rc readiness; annotated on its tracker row). T1 per-commit stays build-only (`discipline.yml:3545-3565`, preflight F3 — a ratified developer-experience reduction, `5-spirit-abi.md:197`).
5. Proven red: missing branch, rejected push (bare remote with a pre-receive refusal), malformed record, a record whose worker log lacks the libFuzzer counter, and a T3 link that hits a planted crash input — each reds its script or job (the crash plant on a throwaway branch, run id recorded).

### AC4 — `BindingClass` on all 44 ship-gate names; list drift closed (former AC6)

1. `[[ship_gate]]` rows gain a required `binding = "blocking" | "advisory-substrate" | "mixed"` (`mixed` = per-leg classes: cross-region-consensus, multi-region-slo, multi-tenant-loom, escape-detector), loaded by `ShipGateEntry` (`corpus_types.rs:84-87`) with unknown/missing values refused, exposed beside `read_disposition` (`gate_common.rs:193-205`). It is the only class home for the 9 workflow-only and 2 structural names; no wrapper module is invented for them.
2. The 15 modules without a class: each red → exit decision goes through `dev_enforced_red_blocks(<registry class>, substrate_present)`. **Measure first** which of the 15 consult phase/disposition for a RED today; those change behaviour and each gets a planted-red vector at `v1_5`. Modules already phase-independent (exit 1 on any finding) are recorded with their exit line in the Dev Agent Record — no ceremony code, no tautological test.
3. Drift: `EXPECTED_GATES` (`check_ship_gate_completeness.rs:20-120`) and the registry become one set of 44 names (the 4 const-only names gain registry rows; `fuzz-build`, `check-fuzz-floor` enter the const with their cadence exemption); the legacy exemptions `:143-148` go.
4. `check-ship-gate-completeness` adds: registry → const reverse check; each name resolved to exactly one of {`v1-0-ship-gate.needs`, declared cadence job (`rto-drill`→`rto-drill-t1`, `check-rto-gate`, `check-fuzz-floor` in their cadence workflows)}; `aggregate.needs` contains `v1-0-ship-gate`. It does **not** enrol aggregate's 119 other prerequisites as ship gates. Proven red: remove a per-commit name from v1.0 needs; add a registry row with no enrolment; drop aggregate's `v1-0-ship-gate` edge; misname the cadence job; delete a `binding` field.

### AC5 — rule 11 (b), workflows: a skip is never a silent green

1. `aggregate` "Fail if any gate failed" (`discipline.yml:5411-5412`) also fails on `skipped`, exactly as `v1-0-ship-gate` does (`:4976`); `macos-check` stays governed by its 20-6 disposition step (`:5418-5422`) and is the only exemption, named in the predicate. Proven red: a planted `if: false` on a required job reds aggregate with **NOT RUN** in the summary (CI run id recorded).
2. `determinism-tests` (`:1665-1697`) and `reproducible-build` (`:63-74`) keep `continue-on-error` only with an explicit **`ADVISORY — owner: <name>; reason: <why>`** comment in the job; aggregate's table reads `needs.determinism-tests.outputs.test_result` (not `.result`, `:5314`) so a failed advisory run shows ⚠️, never ✅. Neither is cited as proof anywhere (grep docs).
3. `set +e` grep scans (`:120`, `:592`): grep exit ≥ 2 fails the step; no-match stays clean. Proven red: unreadable scan input.
4. `journal-append.yml:79`: when a PR number exists, a failed body fetch fails the job (revert pairing depends on it, `invariant_lock.rs:191-196,369-385`); otherwise labelled ADVISORY with owner + reason and the entry marks revert detection `unverified`. `journal-append.yml:110`, `fuzz-cadence.yml:138`, `rpo-rto-cadence.yml:120` get a one-line classification comment (benign seed/display).
5. A committed workflow test (pattern `xtask/tests/cgroup_ceiling_workflow_17_2.rs` / `macos_compile_guard_20_6.rs`) inventories every `continue-on-error` and `|| echo` under `.github/workflows/` and fails if one lacks an `ADVISORY — owner:` label or a classification comment — the inventory cannot silently regrow. The three perf jobs (`:2039`, `:2091`, `:2109`) already say `ADVISORY` + reason but name **no owner**: add `owner: Murat — retired by 20-7 (ADR-068)`; nothing else in those jobs changes.

### AC6 — rule 11 (b), tests: an environment refusal is named locally and fails on a capable runner

1. One cross-crate helper, `tests/harness/ci.rs` (beside `t3_substrate.rs`, included with `#[path]`), `declared_ci() -> bool` = `CI` or `GITHUB_ACTIONS` set (the T2 convention). `t3_substrate.rs:6-29` and `t3_terminal_helpers_17_7.rs:266-277,461-464` move to it (rule 11(d): touched here). The three 17-6 T2 helpers are not edited.
2. `maos-telemetry/tests/otel_gates.rs`: the helper (`:104-140`) returns a typed cause (tool unavailable vs target failed vs log unreadable); only *tool unavailable* may skip, with a named reason, and only when `!declared_ci()`; a failing traced target is a test failure everywhere. `workspace-test-suite` installs `strace` (or the three tests move to a job that does) — **observe** in CI that all three ran (`--nocapture` marker count), run id recorded.
3. `hot_swap_halt_continuity_test.rs:13-17`: path anchored on `env!("CARGO_MANIFEST_DIR")`, missing corpus panics; the test must then really load ≥ 10 scenarios (if the loader is broken, that is the finding — fix at origin, rule 10(a)).
4. `maos-a2a-tcp/tests/support/mod.rs:817-819`: unreadable limits panics under `declared_ci()` (Linux); `unlimited` stays a pass.
5. Root-capable returns (`maos_uninstall_16_4.rs:1413-1421,1654-1663`, `private_forget_restart_13_5i.rs:298-305`): print `SKIP (<test>): running with CAP_DAC_OVERRIDE — permission control unobservable`, and panic under `declared_ci()`.
6. Proven red: each converted site, with `CI=1`, under its refusal → panic (recorded command + output; the telemetry one also in CI).

### AC7 — `check-j4-latency` and the bench jobs say what they can do

1. Registry `gate-registry.toml:348-349`: the J4 **budget** is `advisory` at `v1_5` with a comment *binding gate = 20-7 / ADR-068 stable runner*; the job's sample-count, integrity and mutation steps (`discipline.yml:3650-3665`) stay blocking. The over-budget print in `t_10_4c_j4_latency_gate.rs:76-102` reads `ADVISORY over budget (binding: 20-7)` — no "WOULD BLOCK at v1.5" while the phase is v1.5.
2. `discipline.yml:3669`: drop `--release` (invalid flag); the step fails if `cargo tree` itself fails, then greps. Proven red: a non-zero `cargo tree` and a planted `bench-fault-inject` default feature each red the step.
3. `discipline.yml:1047`, `:1068`: wording becomes *correctness smoke (`--test`, one iteration); no timing baseline exists — 20-7 owns regression*. ADR-068 `:156-158,172-174` satisfied.

### AC8 — diagnostics and generated artifacts tell the truth (owned residuals)

1. **`abi-diff`** (`abi_diff.rs:88-98`): report keeps stdout, stderr and exit status; a tool failure says *could not run: <stderr>*, never *breaking change detected*. **`check-wasm-form-equiv`** (`:75-82,219-227`): a leg that exited non-zero before a test summary reports *harness failed to build/run*; *vacuous* is reserved for a zero-test run. Both stay fail-closed. Proven red for each cause. (deferred `:1048`, E16-A2 item)
2. **Error catalog** (`:1126`): `xtask/error-catalog.toml` becomes the single source — `cause`/`remediation` move into it for the 39 curated codes and are authored for the 10 missing (`ECrossWallRecallDenied`, `EPinMismatch::FingerprintCollision`, `ESkillStore::{Io,Json,UnknownSchemaVersion}`, `ERegionViolation`, `ERegionTagInvalid`, `ETeamIdInvalid`, `EModelProvenanceMissing`, `EModelProvenanceStale`); `error-catalog-generate` emits all 49 deterministically; `check-error-catalog` fails when the committed JSON ≠ generated; the 10 troubleshoot pages exist so `docs-site/scripts/gate-troubleshoot-bidi.js` stays green; `docs-site.yml:9-10,20-21` paths include the TOML. Proven red: delete one JSON key; blank one TOML `cause`; two consecutive generations byte-identical.
3. **Composite red reports** (`:922`): `check-epic-close-coherence` emits its JSON report on the `check(...)?` / `read_pinned(...)?` error paths (`:80-85,124`); committed vectors (a serialized `CARGO_BIN_EXE_xtask` seam on an isolated baseline copy — never mutate the tracked pin) assert each of cohort-mesh, multi-tenant-loom, epic-close-coherence emits parseable, gate-named JSON on a planted mismatch **and** on an infrastructure error. Deleting cohort-mesh's emit block (`:720-723`) reds a vector.
4. **Kernel-pin CI wiring pin** (`:923`): an `xtask/tests/` YAML test pins `jobs.check-kernel-baseline` `fetch-depth: 0` (`:2691`), `components: rustfmt` (`:2698`) and the named step running `--test kernel_pin_content_hash_16_0` (`:2707-2708`); each deletion reds it.
5. **D11 ratchet** (`:916`): the real-tree assertion in `d11_xtask_ceiling_ratchet.rs:195-213` evaluates the *current* `xtask = N` row's own grant comment (parse the full key, `:164-179` tokenizer replaced) through `authorize_xtask_raise` with `DoneStory` live; historical grants are not re-judged. Proven red: raise the row with no grant / with an unrelated valid citation elsewhere in the file / citing a `done` key.
6. **Journey binaries** (`:950`): one shared resolver for `maos`/`maosctl` used by `journey_j0.rs:23-40`, `journey_butler.rs:21-29`, `journey_j3.rs:11-19`, `journey_j4.rs:29-37`, `journey_researcher.rs:18-26`, `jb3_self_tuning_halt.rs:40-48` (J1's target-dir-aware resolver `journey_j1.rs:50+` is the model) that refuses a binary older than any `crates/maos-bin/src` / `crates/maos-cli/src` file, naming `cargo build -p maos-bin -p maos-cli`. Proven red: touch a source file without rebuilding → named refusal.

### AC9 — no signed erasure attestation asserts a decommission it did not perform (14-e1, D5.3, deferred `:549`)

`decommission_region_key` (`regional_teardown.rs:151-159`) no longer returns `completed: true`. With no real region-key revocation mechanism wired, the region-pinned uninstall path computes a `CategoryStatus::CoverageGap { reason: "region signing key not decommissioned — derivable from the retained base seed" }` (`proof.rs:22-26`) **before** `build_erasure_proof` (`main.rs:9220-9236`), and does **not** emit a successful two-phase regional receipt (`:9260-9293`); the builder/verifier stay fail-closed (`:227-243`, `:286-299`). The base seed is not destroyed (it signs HOME receipts and would regenerate the key anyway). `erasure_uninstall_13_5b.rs:288-359` and the receipt unit tests are migrated to the honest outcome (clean cutover). Proven red: re-plant `completed: true` → the new assertion reds; a region-pinned uninstall yields the signed gap and no successful regional receipt. maos-audit + maos-bin only; kernel-Δ 0.

### AC10 — ledgers, registers and the epic are reconciled

1. `deferred-work.md`: every row owned by `20-3a-gate-honesty` is closed with evidence or re-routed by rule 10 to a **not-done** named key (§Owned residuals gives each row's disposition) — `check-dev-record-completeness` would otherwise red when this key reaches `done`.
2. Decision register: **D1** → CLOSED with the AC1 evidence (row text corrected to the ruling's subject set); **D9** — **CLOSED at creation 2026-10-15** by the Q2 ruling (register row + ruling section); the dev only verifies the cited boundary still holds at landing; **D5** gains a dated evidence line for D5.2 (closed on measurement) and D5.3 (AC9) — D5 stays OPEN for `v25-erasure-crash-reconciliation`.
3. `sprint-status.yaml` E16-A2 → `done` with the AC1/AC5/AC6/AC8.1 evidence.
4. `check-adr040-accepted`: the 15-5 filing is annotated *name defect disproved (banner only); no-workflow defect resolved by 20-3b AC7 retirement* in 20-3b's epic row — nothing is repaired in a gate scheduled for deletion.
5. `check-decision-register`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-ship-gate-completeness`, `kloc-check` all green at the landing commit.

### AC11 — the T3 escape corpus executes what it claims (Q4 ruling; deferred `:1129`)

The corpus is an existing contract: every scenario declares a payload and an expected containment outcome (`crates/maos-eval/fixtures/t3-escape-corpus-v0/methodology-attestation.json`), and 17-1's Q7 routed the twenty non-executed ones to this key. Label-only is not a terminal state.

1. `t3_network_escape_executor.rs` (or a sibling under `crates/maos-eval/tests/`) executes **all 25** scenarios under verified T3 through the existing `require_t3` path (`tests/harness/t3_substrate.rs`) and the same spawn route as the five (`:154-176`); each asserts the scenario's declared outcome (attack command fails / blocked effect observed). Kind-8 rows are asserted **only** through 17-1's D-10 confirmed-write marker path; a scenario that cannot produce one asserts `block_observed` and labels its kind-8 leg with owner and reason (17-1 D-12 precedent) — never a fabricated row.
2. A scenario whose declared expectation is not a host escape (e.g. reading the container's own `/etc/passwd`, `filesystem_escape_etc_passwd_001`) is **corrected, not made to pass**: its expectation is rewritten to the real boundary (host file unreachable) or the scenario is withdrawn with a recorded reason, as a new corpus version under the corpus's own methodology (invariant-lock applies if it is a locked corpus — check before editing).
3. `discipline.yml:1704-1751` (`t3-escape-corpus`): the 5-executed / 20-labelled accounting becomes 25 executed (or 25 minus withdrawn, each named); SKIP still fails; the job comment states exactly what T3 is claimed against.
4. Proven red: drop `--cap-drop=ALL` from the T3 argv on a throwaway branch → at least one capability scenario reds in CI (run id recorded); remove one scenario's executor → the count check reds.


## Tasks / Subtasks

- [x] **T0 — Baseline** (all). At `6537454e`+: run the creation gate batch again (`20-3a-evidence/creation-measurements-6537454e.log` lists the commands); confirm every §Current-tree row; re-run `cargo test -p maos-bin --test drain_once_audit_writer` (the local multi-tenant-loom red) and, if it reproduces, file it in `deferred-work.md` with a not-done owner. `cargo run -p xtask -- kloc-check --json` → record the starting per-crate figures.
- [x] **T1 — Out-of-ledger evidence helper** (AC1.1). `gate_common.rs`: helper + unit tests of the truth table (ABSENT/INDETERMINATE never `passed`; exit by class/disposition). No new `EvidenceState` variant.
- [x] **T2 — Gate conversions** (AC1.2–1.4). The nine call sites + `finish_ledger_gate` field; migrate `skipped` consumers; workflow branches removed; one proven-red vector file `xtask/tests/gate_honesty_20_3a.rs` (real binary, isolated tree, serialized where it needs cwd).
- [x] **T3 — Honest-N** (AC2). `check_third_party_trial.rs` reorder (integrity first), `preliminary` status, registry comment, README + schema date; vectors in `story_10_2_proven_red.rs` (`trial_gate` prefix).
- [x] **T4 — Ledger collectors + fuzz evidence** (AC3). Both cadence workflows; collector-script test in a throwaway repo; libFuzzer-counter records; T2 nightly cadence (S1 signed off); T3 24 h chain; registry arithmetic comment.
- [x] **T5 — BindingClass + drift** (AC4). `binding` field (44 rows), loader + refusal, `check_ship_gate_completeness.rs` reverse/cadence/reachability checks; measure the 15 modules' red→exit and convert only phase-consulting ones; vectors.
- [x] **T6 — Workflow honesty** (AC5). aggregate skip predicate, determinism output, ADVISORY labels, grep exit codes, journal-append; inventory test.
- [x] **T7 — Test-suite honesty** (AC6). `tests/harness/ci.rs`; telemetry typed causes + strace in CI; hot-swap corpus anchor; fd limits; root-capable sites; migrate T3 helpers' CI check.
- [x] **T8 — J4 + bench wording** (AC7).
- [x] **T9 — Diagnostics + artifacts** (AC8.1–8.6).
- [x] **T10 — Erasure attestation** (AC9). maos-audit + maos-bin; migrate `erasure_uninstall_13_5b.rs` and receipt unit tests.
- [x] **T10b — T3 escape corpus executors** (AC11). Run the 25 locally under the T3 substrate first, classify each scenario's declared expectation (true boundary / mis-stated), correct mis-stated ones as a new corpus version, then wire CI.
- [x] **T11 — Ledger/register/epic reconciliation** (AC10) incl. §Owned residuals dispositions; deferred `:1019` attribution (3 whole-package `cargo test -p maos-bin` debug runs, then `ps -eo pid,ppid,etimes,args | grep worker-cli-fixture`; read the `GrandchildGuard` arming window `root_shutdown_unload_16_3.rs:403-431`) and `:1029` reproduction (hold the minted endpoint between `maos init` and boot; log the holder pid on `EndpointInUse`, as the row asks) — dispose per the Q5 ruling.
- [x] **T12 — Measure and grant** (frontmatter `kloc_grant`). `cargo fmt --all`; `cargo run -p xtask -- kloc-check --json`; exact per-crate raises (`xtask`, `maos-audit`, `maos-bin`, `maos-eval` if charged) with driver per file under the eased rule; `_aggregate_hardfail` strict measured + 1 under the S3 sign-off (cite it in the `kloc.toml` comment); all in the landing commit.
- [x] **T13 — CI proof** (rule 11(c)). Push; record run ids for: aggregate red on a planted skipped job (throwaway branch, never merged), telemetry tests executed with strace, collector red on a missing branch (workflow_dispatch on a fork-free throwaway branch), J4 `cargo tree` step red on a planted flag. Local-only proofs: command + output under `20-3a-evidence/`.
- [x] **T14 — `bmad-code-review` §A6** (◆ non-degradable): Blind Hunter + Edge Case Hunter + Acceptance Auditor + Test-Infrastructure + runtime execution layer, non-author; every finding dispositioned in §Review Findings; named question per E15-A6: *does this test read the tree, or only what it set itself?*

### Review Findings

T14 §A6, 2026-10-15, on `20-3a/integration`. Nine non-author layers ran as fresh-context reviewer subagents (ruling R4): Blind Hunter ×3 (xtask / crates / workflows+docs), Edge Case Hunter ×3 (same split), Acceptance Auditor, Test-Infrastructure (two sub-reviewers) and a runtime execution layer. **E15-A6 answer** (Test-Infrastructure + runtime layer): the workflow, registry, catalog, ratchet, kernel-pin, corpus and executor-accounting checks read the tree or copies derived from it. Gate vectors run the compiled binary on constructed evidence. The runtime layer reproduced every AC1 output, and all 10 of its mutation plants went red.

- [x] [Review][Patch] Reza exact-test leg named a renamed test (P1; Edge-crates, Acceptance F1) [`xtask/src/check_reza_production_path.rs`] — leg `gdpr-regional-shared-verified-empty` now runs `regional_uninstall_signs_key_coverage_gap`; ADR-059 amendment and `docs/runbooks/dr-1-restore-drill.md` follow. `check-reza-production-path --json` rc 0; the leg reports `running 1 test; 1 passed`.
- [x] [Review][Patch] Fuzz floor could never report PROVEN under `advisory-substrate` (Edge-xtask F1) [`xtask/gate-registry.toml` `check-fuzz-floor`] — binding `blocking`; vector `mature_fuzz_ledger_meeting_the_floor_passes`.
- [x] [Review][Patch] Malformed fuzz ledger returned before the JSON report (Blind-xtask) [`xtask/src/check_fuzz_floor.rs`] — now RED with its report; vector `malformed_fuzz_ledger_is_red_with_a_report`.
- [x] [Review][Patch] A young fuzz ledger was modelled as RED without substrate (Edge-xtask F2, Acceptance F2, Test-infra F1) [`xtask/src/gate_common.rs`, `check_fuzz_floor.rs`] — new `Observation::Accumulating`: it projects INDETERMINATE, never passes and never blocks; the truth table now covers 5 observations. The finding's demand for exit 1 is **dismissed**: AC1.2 reads "a readable ledger younger than 90 days → INDETERMINATE advisory with its accumulated seconds, never `passed:true`". R1 rules on ABSENT evidence, not on AC3.4(b)'s bootstrap window.
- [x] [Review][Patch] Gate registry read relative to cwd (Edge-xtask F4) [`xtask/src/gate_common.rs` `registry_path`] — resolves to the workspace root when cwd lacks the registry.
- [x] [Review][Patch] D11 ratchet broke once 20-3a reached `done` and re-judged historical grants (Blind-xtask, Edge-xtask F5, Test-infra F2/F3) [`xtask/tests/d11_xtask_ceiling_ratchet.rs`] — `authorize_row(kloc, statuses, stories_dir)` accepts a done citation only when that story file records the figure on an `xtask` line. The positive vector picks a live open story.
- [x] [Review][Patch] `OwnedRoot` could SIGKILL a recycled process-group id after reaping (Blind-crates, Edge-crates) [`crates/maos-bin/tests/root_shutdown_unload_16_3.rs`] — tracks `reaped` through shadowing `try_wait`/`wait`. Drop skips the group kill when the root is reaped and `/proc/<pid>` exists: a number still in use as a PGID cannot be reissued, so a live `/proc/<pid>` after reaping means the group is gone.
- [x] [Review][Patch] `runtime_escape_runtime_version_disclosure_005` counted image composition as containment, and its ENOENT would have reached the blocked-event marker (a fabricated kind-8 row) (Blind-crates) [`crates/maos-eval/fixtures/t3-escape-corpus-v1/methodology-attestation.json`] — **withdrawn by name with its reason** (AC11.2: "or the scenario is withdrawn with a recorded reason"). Its executor and probe arm are removed. The accounting asserts executable ∪ withdrawn = the 25 v0 ids, and the CI job requires 24/24 and names the withdrawal (AC11.3: "25 minus withdrawn, each named"). The host-runtime boundary stays executed by `_podman_sock_002`/`_docker_sock_001`. Plants: dropping an executor reds; un-withdrawing without an executor reds (`20-3a-evidence/t14-t3-accounting-plants.log`).
- [x] [Review][Patch] `network_escape_raw_socket_004` was classified "true boundary", though protocol 0 fails with EPROTONOSUPPORT regardless of CAP_NET_RAW (Blind-crates) — reclassified `corrected`. The same correction was found for `capability_escape_cap_net_raw_002`: ping may use unprivileged ICMP datagram sockets. Reasons are recorded in both the scenario JSON and the methodology.
- [x] [Review][Patch] The nightly fuzz runner tripped reproducible-build's stable-only scan (Edge-workflows) [`scripts/run-fuzz-link.sh`] — `RUSTUP_TOOLCHAIN=nightly`, not `cargo +nightly`.
- [x] [Review][Patch] A multi-value record file bypassed validation (Edge-workflows) [`scripts/collect-ledger.sh`] — exactly one JSON value is now required before append.
- [x] [Review][Patch] Shared workflow-level concurrency could drop a pending fuzz run behind the 25 h T3 chain (Blind-workflows) [`fuzz-cadence.yml`, `fuzz-prerelease.yml`] — removed. The collector retries ≤ 5 on a non-fast-forward push; a pre-receive refusal stays fatal.
- [x] [Review][Patch] The Unix-only collector test compiled on Windows (Blind-xtask) [`xtask/tests/ledger_collectors_20_3a.rs`] — `#![cfg(unix)]`.
- [x] [Review][Patch] Landing grant and landing-gate evidence missing (Acceptance F3) — T12 measured and granted; the AC10.5 gates ran green at the landing tree (Completion Notes).
- [x] [Review][Patch] E16-A2 not reconciled (Acceptance F4) [`sprint-status.yaml`] — annotated IMPLEMENTED with the AC1/AC5/AC6/AC8.1 evidence; it flips to `done` with T13's run ids (R2).
- [x] [Review][Patch] AC4.2 measurement inventory not recorded (Acceptance F5) — now in the Debug Log.
- [x] [Review][Patch] No executable recipes for AC3.5's crash plant or AC11.4's cap-drop plant (Acceptance F6) — added to Completion Notes §T13 recipes.
- [x] [Review][Patch] AC6.6 root-site refusal proofs absent (Acceptance F7) — these need root; this host has no non-interactive sudo and refuses `unshare -r`. Exact commands are in §T13 recipes. The `CI=1` telemetry refusal did run (rc 101, runtime layer).
- [x] [Review][Patch] Found at landing, not by a layer: AC7.2's defect class in two sibling steps [`.github/workflows/discipline.yml` D7 `check-wasm-form-equiv`, F7 `check-multi-region-slo`]. Both piped `cargo tree --release` (an invalid flag) into `grep -q`, so the guards were vacuous: the old step passed with the fault feature planted as a root default. Fixed with the same form as J4. The J4 vector now executes all three real steps (`20-3a-evidence/t14-release-graph-steps.log`).
- [x] [Review][Dismiss] escape-detector's green path prints `passed:true` beside `evidence_state: INDETERMINATE` (Edge-xtask F3) — D2 (register, CLOSED by decision) ratified `passed` plus per-leg states as the ledger-gate claim boundary. AC1.2 converts only the substrate-absent RED (vector `escape_substrate_refusal_never_passes`), and the roll-up already discloses unsigned live legs.
- [x] [Review][Dismiss] The journey resolver joins a relative `CARGO_TARGET_DIR` to the workspace root (Blind-crates) — this is J1's ratified resolver, and AC8.6 migrates consumers onto it.
- [x] [Review][Dismiss] Freshness checks only the two binaries' `src` trees (Blind-crates) — AC8.6 scopes the check to source mtimes plus the rebuild command.
- [x] [Review][Dismiss] Successful-receipt test coverage was removed (Test-infra) — AC9 deletes the success path; deliberate cutover.
- [x] [Review][Dismiss] The top-level `evidence_state` from `finish_ledger_gate` has no end-to-end assertion (Test-infra) — accepted coverage limitation; the roll-up is unit-tested (`worst_state_names_the_gate`).

## Dev Notes

### Design decisions

- **D-20-3a-A — no fifth evidence state.** ABSENT/INDETERMINATE already cover "not run"/"could not decide"; `preliminary` and `accumulating` are gate-specific statuses carried beside `evidence_state`. ADR-055 `:155` and ADR-068 `:114-116` depend on the four-state vocabulary.
- **D-20-3a-B — `passed` means proven.** The lie being removed is `"passed": true` on absent/indeterminate evidence. Exit code stays governed by class + disposition, so an advisory gate still lets CI proceed — visibly, with `advisory:true` and a warning.
- **D-20-3a-C — D2 is not re-opened.** Ledger gates keep their ratified boundary; they gain only the shared top-level field.
- **D-20-3a-D — registry is the class home.** Workflow-only and structural names have no module; a per-row `binding` field is the only place their class can live, and one loader refuses missing/unknown values. Modules whose behaviour would not change get no edit (rule: never test wiring).
- **D-20-3a-E — no bespoke CI detection.** One helper (`tests/harness/ci.rs`), the T2 two-variable convention (stricter than T3's `CI == "true"`).
- **D-20-3a-F — ADVISORY needs a person and a reason in the YAML** (rule 11(b)); a story key alone is not an owner once it closes.
- **D-20-3a-G — gates 20-3b deletes are not repaired here** (`check-cross-form-equiv`, `check-adr040-accepted`).
- **D-20-3a-H — error catalog single source.** Preserving JSON fields across generations would keep two sources; moving curated text into the TOML makes the JSON a pure artifact a gate can compare.

### Rule-11 inventory (benign rows — no change, recorded so the inventory test has a baseline)

`set +e` that preserve rc / validate evidence: `discipline.yml:1121,1164,1176,1188,1200,1212` (example-spirit-ts), `:1440,1477` (NFR-Rel with one visible retry), `:1730,1790` (T3, 17-1), `:1882,1908,1926,1945` (epic-17-exit), `:3385,3403,3417,3442,3494` (wasm-host-tests), `:4774` (workspace-test-suite). `always()` evidence uploads/summaries: `:1897,1961,2981,3514,3962,4022,4133,4209,4929,5441`, `fuzz-cadence.yml:111`, `rpo-rto-cadence.yml:97`, `journal-append.yml:129`. `|| true` predicate/no-match: `journal-aggregate.yml:102,104`, `discipline.yml:4513,4564,4591,4598`. Masks fixed by this story: `discipline.yml:74,1686,120,592,5412,5314`, `journal-append.yml:79`, `fuzz-cadence.yml:171-198`, `rpo-rto-cadence.yml:142-170`.

### Owned residuals (`deferred-work.md`, all `**Owner: 20-3a-gate-honesty**`)

| Row | Disposition |
|---|---|
| `:548` revocation `unwrap_or(0)` | Close — fixed on measurement (§Current-tree). |
| `:549` decommission `completed: true` | AC9. |
| `:648` FKCS hold | Already in a CLOSED section (D8); make the owner line unmistakably historical so no gate counts it. |
| `:760` vetting-lapse crossing | **CLOSED by decision at creation (Q2 / D9).** Boundary = upgrade/promotion (`main.rs:130-204`, from `operator_door.rs:877-884,967-969`); no crossing check is built (PRD has none; none exists in `maos-loom-lite`/`maos-a2a-core`/`maos-cohort`). The post-promotion lapse consequence is FR37's revocation semantics + FR59's vetter yank → `20-1` (tracker row annotated). Dev marks the row dispositioned with this pointer. |
| `:916` D11 ratchet | AC8.5. |
| `:922` composite reports | AC8.3. |
| `:923` kernel-pin wiring | AC8.4. |
| `:950` stale journey binary | AC8.6. |
| `:1009` aggregate red on main (standing rule) | AC5.1 is the standing rule; close the row as historical with that pointer (no claim about today's `main` without check-run evidence). |
| `:1011` Windows ACP HOME | Close with code evidence. |
| `:1019` leaked grandchildren | T11 attribution. **Q5 ruling:** it is test-harness infrastructure → fix at origin (rule 10(a), kloc-free) — if the leak is the guard-arming window (guard armed only after discovery, `root_shutdown_unload_16_3.rs:403-431`), arm ownership from spawn and prove it with a planted panic; if not reproduced in 3 runs and no window is found, close as not-reproduced with the measurement. |
| `:1021` butler hang | Already struck; close the residual timeout note with the current `workspace-test-suite` ceiling (`discipline.yml:4657-4671`) re-derived from a completed run. |
| `:1029` EndpointInUse | T11 reproduction; mechanism [INFERENCE]: `maos-domain/src/operator_door.rs:369-371` binds port 0 and drops the listener before `:423-428` records it. **Q5 ruling:** 20-3a attributes (reproduction + holder pid); the production fix is **routed by charter to `20-3c-audit-truth-and-halt-receipt-residuals`** as a new member — `control.json` is a record asserting an endpoint the root does not hold (rule 10(c)); the owner line moves to 20-3c with the reproduction attached. Not fixed here (production `maos-domain`, outside this story's blast radius). |
| `:1037` three T2 suites | Close — 17-6. |
| `:1042` J4 | AC7.1–7.2. |
| `:1043` bench wording | AC7.3. |
| `:1048` wrong-cause diagnostics | AC8.1. |
| `:1126` catalog drift | AC8.2. |
| `:1129` twenty unexecuted T3 scenarios | **AC11 (Q4 ruling).** Honesty is already enforced (`t3_network_escape_executor.rs:140-152`, `discipline.yml:1733-1751`: 5 executed / 20 labelled, SKIP fails); execution is what remains. The M3 0600/0666 socket legs are already executed at `:180-218`. Selection today is by blocker string, not category (`capability_escape_cap_net_raw_002` executes; `network_escape_raw_socket_004` does not). |

### Funding (rule 6; ruled Q1)

| Item | xtask | maos-audit | maos-bin |
|---|---:|---:|---:|
| AC1 helper + 9 sites + ledger field | +140…+270 | 0 | 0 |
| AC2 honest-N | +50…+100 | 0 | 0 |
| AC3 collectors (YAML mostly) | 0…+20 | 0 | 0 |
| AC4 binding field, loader, reconciliation, phase-consulting modules | +150…+340 | 0 | 0 |
| AC7 | 0…+10 | 0 | 0 |
| AC8.1/.2/.3/.5 | +145…+330 | 0 | 0 |
| AC9 | 0 | +20…+40 | +20…+40 |
| AC11 (maos-eval: probe code only; tests are kloc-free) | 0 | 0 | 0 (maos-eval 0…+300) |
| **Total** | **+485…+1070** (≤ +1390 with +30%) | **+20…+40** | **+20…+40** |

Tests under `crates/*/tests` and `xtask/tests` are kloc-free (`xtask/kloc.toml:1-3`); inline `#[cfg(test)]` is charged. 20-3b nets xtask −4.5k…−5.2k and repays this; the epic orders 20-3a first.

### Guardrails

- Kernel-Δ 0: no edit under `crates/maos-kernel-core/src` (tests under `crates/maos-kernel-core/tests` are fine).
- `gate-registry.toml:1-3`: renaming/removing a gate is breaking — this story renames nothing; the `binding` field is additive.
- Clean cutover: `skipped` → `evidence_state`, old tokenizer, old regional-success path — every caller migrated, no aliases.
- Do not edit `ADR-040`'s `Status:` line (15-5's warning).
- Do not "fix" `multi_provider_matrix.rs:73-75` (its caller asserts non-empty) or `check_mock_not_in_release_smoke.rs` (platform applicability) — measured false positives.
- Every new test reads the tree or runs the real binary; a vector that passes against its own fixture only is a null control (E15-A6).

### Testing standards

`cargo test -p xtask --test <new files>`; `cargo test -p xtask` full (968+ at 20-6); affected crates' integration tests (`maos-telemetry`, `maos-kernel-core --test hot_swap_halt_continuity_test`, `maos-a2a-tcp`, `maos-bin --test erasure_uninstall_13_5b --test maos_uninstall_16_4`, `maos-journey-test`); the gates of AC10.5; `cargo fmt --all --check`; `cargo clippy -p xtask`. Workflow scripts are tested by executing them (20-6), never by substring presence alone.

### Previous-story intelligence (20-6, 17-7, 17-1)

- 20-6: the review found a TDZ `ReferenceError` in aggregate's embedded JS that would have redded every PR — any edit to aggregate's `github-script` step must be executed under node in a test (20-6 did); `aggregate` is `if: always()` and only `failure`/`cancelled` red it today (its P8).
- 17-7: preflight tables + enumerated callers before code; CI-only evidence is recorded with run ids; "a skip on a declared-capable runner is a failure, not a fallback".
- 17-1: removed the T3 `continue-on-error` pair (`b19556ab`, run `37988000230`); its Q7 ruling is the source of `:1129`'s label-only state.
- Shared target dir on the dev host (`CARGO_TARGET_DIR=/mnt/build/cargo`, 17-3a F9): rebuild `maos`/`maosctl` before any journey proof. **Observed 2026-10-15:** `target/debug/xtask` was overwritten by another workspace's `xtask` binary (it answered `usage: cargo xtask <verify | regen-golden | check-license-headers>`). Always invoke gates as `cargo run -q -p xtask -- <gate>`, never the bare binary, and treat any gate evidence produced by a bare path as void.

### Git intelligence

Last commits: `6537454e` feat(worker) T3 lifecycle (17-7), `37c34be4` 17-2, `b19556ab` 17-1, `f88a1968` 20-6 done, `d371b5a4` 20-6 T8 record. Titles are `<story-key>: …` or conventional `feat(scope)`/`ci(scope)`; kloc re-pins ride in the landing commit with the driver in the `kloc.toml` comment.

### Latest technical facts (checked 2026-10-15)

- GitHub Actions: a step with `continue-on-error: true` has `outcome: failure` but `conclusion: success`, and the job's `result` is `success` — aggregate must read an explicit output to see the failure ([contexts reference](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts)).
- jq `--slurpfile name file` binds `$name` to an **array** of the file's JSON texts; `$rec[0]` is the object, so `.records += $rec[0]` is array + object (reproduced locally, jq exit 5).
- cargo 1.99 `cargo tree` has no `--release` (exit 1, observed).
- Toolchain `stable` (`rust-toolchain.toml`), xtask: clap 4.5 (`clap_derive` 4.6.1 locked), serde_json 1, toml 0.8, serde_yaml 0.9.

### References

- Epic: `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` §20-3a/20-3b split, §20-3-gate-honesty-pass, confidence rules 10–11, §Re-preflight 2026-10-15.
- Register: `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md` rows D1, D2, D5, D8, D9, D20; rulings `:398-429`.
- ADR-055 `:155`; ADR-068 `:27-30,114-116,144-174`.
- `_bmad-output/implementation-artifacts/deferred-work.md` rows listed in §Owned residuals; `sprint-status.yaml:282,455-457`.
- `15-5-decision-adrs-and-provisioning-checklist.md:906-910` (adr040 filing); `17-1-…md:324` (Q7).

## Operator rulings (2026-10-15 — Q1–Q5, by consensus under the operator's decision rule)

**Rule applied (operator, 2026-10-15):** *long-term correctness; follow the PRD strictly; actively correct architectural drift; honour existing contracts; fill requirement gaps in line with the roadmap.*

| Q | Ruling | Grounds under the rule |
|---|---|---|
| **Q1 funding** | **20-3a lands before 20-3b** (the epic's contract: a gate must be honest before a retirement is read off it). Per-crate rows: exact measured raises under the eased RECOVERY-LANE CEILING RULE. `_aggregate_hardfail`: strict measured+1, authorized in advance by the operator (**S3 ✅**). | Honour contracts: `kloc.toml:60-69` eases per-crate raises only; `:106-115` keeps the aggregate behind an explicit grant. Reversing the order would delete gates on evidence they misreport. |
| **Q2 D9** | **Accept the upgrade/promotion boundary; build no crossing check.** D9 CLOSED by decision at creation. The PRD gap that does exist — what happens to an already-promoted Spirit when its vetting attestation is revoked or lapses — is FR37's revocation semantics and FR59's vetter yank (warn / quarantine / auto-revoke by operator policy), routed to `20-1-registry-client-install-verb-vetter-and-yank` (it owns `vet revoke` and the yank policy; tracker row annotated). | PRD strictly: no FR requires a vetting refusal at a crossing (FR37 `functional-requirements.md:87`, FR59 `:93`, vetter trust model `domain-specific-requirements.md:25`); "no invented mechanism" is D9's own constraint. Fill the gap: post-promotion revocation is required and gets a named vehicle. |
| **Q3 fuzz** | **In scope, and wider than "chaining": correct the drift.** libFuzzer-counter records; floor unchanged; T2 nightly 4 h restored (**S1 ✅**); T3 24 h pre-release chain (≤ 6 h links); rc-tag precondition routed to `20-5` (AC3.4). | Architecture §5.2 makes the floor non-negotiable and the unit "CPU-hours by libFuzzer counter, not wall-clock" (`5-spirit-abi.md:193-197`); the workflow asserts a constant `2400`, runs weekly (≈ 8× under the floor by its own comment) and has no 24 h run although PRD NFR-Sec-5/6 make 24 h a v1.0 ship gate. |
| **Q4 T3 corpus** | **Execute all 25 (AC11); correct mis-stated scenarios rather than pass them.** No claim-boundary shortcut (**S2 ✅**). | Honour contracts: the corpus declares a containment outcome per scenario; 17-1 Q7 routed the twenty here, it did not retire them. Long-term correctness: a declared control with no execution is rule 11(a)'s case. |
| **Q5 `:1019` / `:1029`** | `:1019` → fix at origin in the test harness once attributed (or close not-reproduced with the measurement). `:1029` → attribute here; production fix routed to `20-3c` by charter. | Rule 10: (a) inside this story's blast radius for test infrastructure; (c) charter match for a record (`control.json`) asserting a held endpoint the root does not hold. |

**Operator sign-off — Lunarpulse, 2026-10-15 (all three given; binding):**
- **S1** ✅ — Q3(b): the Epic-12 retro's weekly fuzz downgrade is reverted to the architecture's T2 nightly (≈ 12 runner-hours/day across 3 targets; GitHub-hosted minutes are free on a public repository [INFERENCE: the repo's refs are readable unauthenticated]). T4 proceeds in full.
- **S2** ✅ — Q4: the twenty scenarios 17-1 Q7 left labelled are executed here (AC11). T10b proceeds.
- **S3** ✅ — Q1: the `_aggregate_hardfail` raise at landing is authorized in advance at strict measured + 1; the landing commit records the exact figure and drivers. T12 needs no further round-trip.

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5 is the dev of record and integrator. Slices T2–T10b and the T11 investigations were built by eleven fresh-context subagents on **openai-codex/gpt-6.1-sol**, each in an isolated git worktree (`20-3a/<slug>` branches off `20-3a/base`) with its own target dir. The T14 review layers ran on the same subagent model, and T3-corpus follow-ups were done by the integrator after the provider policy filter stopped two subagents. The integrator merged and re-verified everything, and owns every claim below.

### Debug Log References

- `20-3a-evidence/t0-baseline-6537454e.log` — T0 gate batch, kloc start figures (xtask 44370, maos-audit 7267, maos-bin 25939, maos-eval 3802, aggregate 175372), and the `drain_once_audit_writer` attribution.
- **Operator rulings taken during dev (2026-10-15, Lunarpulse, via the dev session):** (R1) AC1 internal conflict — the headline says all seven out-of-ledger gates exit 0 on ABSENT "because" their v1_5 disposition is advisory/blocking-when-present, but `gate-registry.toml` has plain `blocking` at v1_5 for `check-migration-merkle`, `check-rto-gate`, `check-fuzz-floor`, `check-skill-conformance`; ruled **AC1.1 literal — disposition decides**: ABSENT + plain `blocking` exits 1 (accepted consequence: `check-migration-merkle` reds `aggregate`/`v1-0-ship-gate` until a corpus exists in CI; the two cadence gates red until evidence exists). (R2) CI proofs are delivered as recipes; the landing commit stays local, nothing is pushed — T13 stays open until the operator runs them. (R3) The T3 escape corpus is verified in CI only (no podman/docker on the dev host). (R4) T14 by fresh-context reviewer subagents.
- T0 attribution: the dev-host shell exports `CI=true`; `tests/harness/t3_substrate.rs` treats that as a declared-capable runner and panics without `MAOS_T3_RUNTIME` (rule 11(b), correct). With `env -u CI -u GITHUB_ACTIONS` the same test SKIPs named and passes. Not a defect — nothing filed in `deferred-work.md`.
- T1 red→green: planted `starts_with("blocking")` (blocking-when-present also blocking ABSENT) → `out_of_ledger_truth_table_is_exhaustive` and `absent_evidence_follows_the_disposition_at_the_requested_phase` FAILED; restored → 11/11 (lib) and 11/11 (bin) green.
- **AC4.2 measurement (T5).** Of the 15 modules without a class, 8 are phase-independent: they exit 1 on any finding and are unchanged. Exit lines: `check_export_control.rs:234`, `check_fuzz_targets.rs:208`, `check_rto.rs:115`, `check_loom_substrate_drift.rs:915`, `check_env_contract.rs:185`, `check_decision_register.rs:722`, `check_exit_commands.rs:1714`, `check_model_currency.rs:388`. The other 7, the out-of-ledger gates (pentest, red-team, third-party-trial, skill-conformance, fuzz-floor, migration-merkle, rto-gate), now decide through `OutOfLedgerVerdict::judge_registered`, i.e. registry class plus disposition. No module consulted the phase for a RED outside those 7 and trial-attestation (`:135` → `dev_blocks`).
- **Integration test run** (`20-3a/integration`, `env -u CI -u GITHUB_ACTIONS cargo test --workspace`): 4799 passed, 19 failed, every failure attributed:
  - the 2 aggregate-workflow tests: fixed;
  - the kloc ceiling test: expected until T12, green after;
  - 6 `init_test`: no `target` symlink in the worktree;
  - 4 `cgroup_ceiling_smoke`: no delegated cgroup on this host;
  - 2 escape-detector: probe fixture not built;
  - 2 python twin: no Ed25519 library;
  - 2 `worker_resource_limits_17_2`: pre-existing `assert!(require_t3)`, filed in deferred-work's new ownerless-dispositioned section.
- **Landing tree runs** (main working tree; code identical to `20-3a/integration` tip `b4f124fc`, plus T12's `kloc.toml` and the D7/F7 step fix):
  - `cargo test -p xtask --no-fail-fast`: 1058 passed, 0 failed;
  - `d11_xtask_ceiling_ratchet` 8/8 and `recovery_lane_ceiling_rule` 10/10;
  - `workflow_honesty_20_3a` 8/8 and `ship_gate_binding_20_3a` 8/8;
  - `cargo fmt --all --check` clean;
  - AC10.5 gates all rc 0: `check-decision-register`, `check-dev-record-completeness` (169 done stories), `check-epic-close-coherence` (21 epics), `check-exit-commands`, `check-ship-gate-completeness` (44 gates), `kloc-check` (aggregate 176000).
- **Integration-only runs after the T14 patches:**
  - `cargo test -p maos-eval --test t3_network_escape_executor --test t3_escape_workflow --test t3_escape_corpus`: 4 + 2 + 3 passed, `T3-ESCAPE-ACCOUNTED executors=24 classified=25 withdrawn=1`;
  - `check-reza-production-path --json`: rc 0;
  - gate batch: pentest/red-team/third-party-trial ABSENT rc 0; skill-conformance PROVEN; fuzz-floor/migration-merkle/rto-gate ABSENT rc 1 (R1).
- T11 `:1019` harness fix: planted-panic red → green and 3 whole-package runs with zero orphans (`20-3a-evidence/t11-harness-residuals/`). `:1029` reproduced: holder pid 396702, exit 78 → 0 (`endpoint-reproduction.*`); the production fix is routed to 20-3c (Q5).
- **Operator rulings 2026-10-16 (Lunarpulse, via the dev session):** (R5) the dev may use `gh` and push throwaway `ci/20-3a-*` branches for T13 plants and PR runs; `main` is still pushed by the operator only. (R1-A) R1 stands, but the permanent red it caused is removed at the source: the `check-migration-merkle` job generates the pinned corpus in CI (option A). (R6) Pre-existing reds fixed in this story's follow-up: the 17-7 BREAKING.md Migration block, the `gpt-6-astra` allowlist entry, the rustls/wasmtime patch bumps, and the unsafe-libyaml documented ignore.

### Completion Notes List

- Story created 2026-10-15 by `bmad-create-story` (five scouts + direct gate runs at `6537454e`). Ultimate context engine analysis completed - comprehensive developer guide created.
- **DEV COMPLETE 2026-10-17 — Status `review`.** Every task is checked.
  - **T13:** each listed CI proof has a run id (§T13 recipes and CI evidence below).
  - **T11 / AC10.1:** every row this key owns is closed or re-routed; the last two, deferred-work line 1036 (§Owned residuals `:1037`) and line 1128 (`:1129`), closed with CI evidence.
  - **T11 / AC10.3:** E16-A2 is `done`.
  - **Final verify:** run 38988216311 (PR #9, `84944db4`) is 184/184 green, aggregate included. It needed one re-run of the failed jobs, for the 16-3 `ac4_v3b` 5 s orphan-bound flake that is routed to `20-7`.
  - **Landing:** `main` holds the landing commit `389a07e9`. The verified follow-ups `ef3cb4f7..84944db4`, plus record-only commits, are local; the operator pushes `main` (R5).
- **T12 kloc** (after `cargo fmt --all`, `kloc-check --json`, tokei 14.0.0; per-file drivers in each `kloc.toml` row):

  | Row | Before | After | Δ | Note |
  |---|---|---|---|---|
  | xtask | 44370 | 44812 | +442 | raise (+433 landing, +9 CI follow-up) |
  | maos-eval | 3802 | 4038 | +236 | test-only T3 probe |
  | maos-audit | 7267 | 7248 | −19 | exact lower re-pin |
  | maos-bin | 25939 | 25917 | −22 | exact lower re-pin |
  | maos-kernel-core | 20545 | 20545 | 0 | kernel-Δ 0 |
  | `_aggregate_hardfail` | 175373 | 176010 | | measured 176009 + 1 (S3) |

  Grant record for the D11 ratchet: xtask ceiling 44812 granted by 20-3a-gate-honesty (landing 44803, CI follow-up 44812).
- **AC1** — `gate_common` gained `Observation`, `OutOfLedgerVerdict` (judge, judge_registered, finish), `GateBinding`/`read_binding` and `EvidenceState::worst`. Converted: the 7 out-of-ledger gates, trial-attestation's exit, escape-detector's substrate-absent RED, and `finish_ledger_gate`'s `evidence_state`. `skipped` is gone from migration-merkle and rto-gate, with every consumer migrated. Workflows always invoke the gates.
  - R1: skill-conformance, fuzz-floor, migration-merkle and rto-gate exit 1 on ABSENT at v1_5. The cadence gates red until evidence exists. Migration-merkle no longer waits for a corpus: under R1 option A (2026-10-16) the CI job generates the pinned corpus and proves it.
  - Deviation: `Observation::Accumulating` covers the < 90 d fuzz window (AC1.2 text; see Review Findings).
- **AC2** — honest-N: integrity runs first; a small consistent cohort is `preliminary` (never PROVEN); v2.0 refusal and the N=12 stratum shortfall have vectors.
- **AC3** — collector scripts fail on a missing branch, a rejected push or a malformed record, with no branch seeding. Records carry libFuzzer counters, corpus counts, SHA and tier. T2 is restored to nightly 4 h (S1). Downstream gates use `always()`.
  - Deviation: the T3 24 h pre-release chain is **five 5 h links**, not ≤ 6 h × 4, because the 355-min job cap leaves no room for 6 h of fuzzing after build. 5 × 5 h = 25 h ≥ 24 h (runbook `fuzz-cadence.md`).
  - `fuzz-build` stays per-commit. The rc-tag precondition is routed to 20-5.
- **AC4** — all 44 rows carry `binding`; the loader refuses missing or unknown values; registry and const are one set of 44; reverse, cadence and reachability checks have proven-red vectors.
  - Deviation: reza is `mixed`, because it has both Blocking and AdvisorySubstrate legs.
- **AC5** — aggregate fails on a skipped or cancelled need and prints NOT RUN (only macOS exempt); advisory jobs carry `ADVISORY — owner:`; grep rc ≥ 2 propagates; journal-append fetch failures are fatal; the escape-hatch inventory has regrowth plants. `workspace-test-suite` has `timeout-minutes: 65`, measured from runs 37644181760/37658103096.
- **AC6** — `tests/harness/ci.rs` provides `declared_ci()` (CI or GITHUB_ACTIONS). Also: typed telemetry causes, with strace installed in CI and the 3 markers asserted; the hot-swap corpus is manifest-anchored; fd-limit and the three root-capable sites name their refusal locally and panic under declared CI.
- **AC7** — the J4 budget is advisory and its integrity, sample and mutation steps blocking. The invalid `cargo tree --release` is gone, failure propagates, and the inverted graph is used — now also in D7 and F7, which had the same defect (Review Findings). The two bench jobs use the correctness-smoke wording.
- **AC8** — abi-diff and wasm-form-equiv report the observed cause. `xtask/error-catalog.toml` is the single source for 49 codes, and the committed JSON is drift-checked. The CLI name is the existing `error-catalog-check`. Ten troubleshoot pages were added, and `route-manifest.json` was hand-edited because no generator exists. epic-close-coherence emits JSON on every error path. The kernel-pin CI wiring is tested, the D11 ratchet hardened, and the journey binary resolver checks freshness.
- **AC9** — regional uninstall no longer builds a success receipt. It signs the key-coverage gap and refuses to attest pre-partition shared residue. The ADR-059 amendment is recorded.
- **AC10** — rows and registers updated:
  - Register: D1 CLOSED; D9 boundary verified at landing; D5 gains D5.2/D5.3 evidence and stays OPEN.
  - The 20-3b epic row is annotated for `check-adr040-accepted`.
  - deferred-work rows closed (§Owned residuals numbering): `:548`, `:549`, `:648`, `:760`, `:916`, `:922`, `:923`, `:950`, `:1009`, `:1011`, `:1019`, `:1021`, `:1042`, `:1043`, `:1048`, `:1126`.
  - Two ownerless-dispositioned rows were added. `:1029` is routed to 20-3c.
- **AC11** — `t3-escape-corpus-v1` (v0 unchanged) classifies all 25: 11 true boundary, 13 corrected, 1 withdrawn by name. The 24 executable scenarios have executors on the `require_t3` route, with kind-8 assertions only via D-10 markers. The runtime-free accounting and the real job shell are tested. Execution is CI-only (R3).
- **Kernel-Δ 0**: nothing under `crates/maos-kernel-core/src` changed. `check-cross-form-equiv`, `check-adr040-accepted` and the ADR-040 Status line are untouched.
- **§T13 recipes** (operator-run; throwaway branches `ci/20-3a-*`, never merged; record each run id here):
  1. *Landing push* — `workspace-test-suite` must log `OTEL_AIRGAP_RAN` for all 3 air-gap tests (AC6.2). `t3-escape-corpus` must print `T3-ESCAPE-ACCOUNTING executed=24/24 withdrawn=1 ['runtime_escape_runtime_version_disclosure_005'] unexecuted=0` on both `ubuntu-24.04` and `ubuntu-26.04` (AC11.1/11.3). `aggregate` must be green apart from the R1 ABSENT reds (migration-merkle). **Observed in run 38807569588: AC6.2 and AC11.1/11.3 green; `aggregate` red (§T13 CI evidence).**
  2. *AC5.1* — add `if: false` to job `check-unsafe` (`discipline.yml:168`) → `aggregate` red, summary row `check-unsafe … NOT RUN`. **DONE 2026-10-17, isolated:** run 38988216897 attempt 3 (PR #12, `a2fd627e` = `84944db4` plus the plant only). 182 jobs green, `check-unsafe` skipped, and `aggregate` (job 117318317677) is the one failure, exit 1, with no other non-success need. The PR summary it writes reads `| check-unsafe | ❌ NOT RUN — skipped |` (comment 6227142213, updated 05:21:08Z). Attempts 1–2 were confounded: attempt 1 by the T3 bound flakes, attempt 2 by the ledger re-run binding (deferred-work).
  3. *AC6.2 plants* — (a) delete the `Install strace` step → the air-gap step red with `tool unavailable on declared CI`; (b) delete one `OTEL_AIRGAP_RAN` print in `crates/maos-telemetry/tests/otel_gates.rs` → the marker assertion red. **DONE 2026-10-16.**
     - (a): strace turned out to be preinstalled at `/usr/bin/strace`, so the plant moved it away instead of deleting the install step. All three air-gap tests then panicked with `tool unavailable on declared CI: strace unavailable` (run 38926602107, job 117059641919, PR #10).
     - (b): `AssertionError: telemetry air-gap controls NOT RUN … observed=[2 of 3]` (run 38926608301, job 117059661990, PR #11).
  4. *AC3.2/3.5 collectors* — in `scripts/collect-ledger.sh`, set `fuzz) branch=fuzz-ledger-20-3a-proof-absent`, then `gh workflow run fuzz-cadence.yml --ref ci/20-3a-collector-absent`. For speed, cut the link duration `14400` in `fuzz-cadence.yml:44` to `30`. Expect `fuzz-ledger-collect` red with the operator-lane diagnostic and `check-fuzz-floor` ABSENT. Then the same with `rto) branch=rto-ledger-20-3a-proof-absent` and `gh workflow run rpo-rto-cadence.yml`. **DONE 2026-10-16:**
     - fuzz-cadence run 38926617566: `fuzz-ledger-collect` red with `fatal: couldn't find remote ref …` and `fuzz-ledger-20-3a-proof-absent unavailable — ops-provisioning-secrets-and-accounts must provision the ledger branch and write access`; `check-fuzz-floor` ABSENT rc 1.
     - rpo-rto-cadence run 38926622364: `rto-ledger-collect` red with the same diagnostic; `check-rto-gate` read the real `rto-ledger` and returned PROVEN_BLOCKING (RTO 2 s, drill 2026-10-11).
  5. *AC3.5 crash* — in `crates/maos-manifest/fuzz/fuzz_targets/manifest_parser.rs`, add `assert!(data != b"20-3a-planted-crash");` at the top of the fuzz body. Commit the seed `crates/maos-manifest/fuzz/corpus/manifest_parser/planted` containing exactly `20-3a-planted-crash` (no newline). Set the five `18000` link durations in `fuzz-prerelease.yml` to `30`, and redirect `fuzz)` in `collect-ledger.sh` as in step 4 so no planted record reaches the real ledger. Then `gh workflow run fuzz-prerelease.yml --ref ci/20-3a-fuzz-crash`. Expect `fuzz-link-1 (manifest_parser)` red with `T3 manifest_parser link 1 crashed or a worker failed; refusing evidence`. **DONE 2026-10-16** (fuzz-prerelease run 38926626626):
     - `fuzz-link-1 (manifest_parser)` (job 117059709128) red with exactly that error.
     - The other two targets' link 1 stayed green, links 2–5 were skipped, and no record reached a ledger.
     - Control: the same 30 s `manifest_parser` link without the plant was green (fuzz-cadence job 117059684811).
  6. *AC11.4* — delete `"--cap-drop=ALL".to_string(),` (`crates/maos-kernel-core/src/security/sandbox/t3/argv.rs:77`); this branch only, so kernel-Δ stays 0 at landing. Expect `t3-escape-corpus` red on ≥ 1 `capability_escape_*` bounding-bit assertion. **DONE 2026-10-16** (run 38926602107, PR #10): both images fail at `capability_escape_cap_dac_override_003: the attempt escaped or failed unexpectedly` (probe exit 1), and the accounting reports `executed=0/24` (jobs 117059631587 / 117059631227).
  7. *AC7.2* — re-add `--release` to the J4 `cargo tree` (the step "Verify bench-fault-inject absent from release graph (D3)") → step red with cargo's `unexpected argument` error. Or plant `default = ["bench-fault-inject"]` in `crates/maos-bench/Cargo.toml` → `FAIL: bench-fault-inject present`. **DONE 2026-10-16:** `error: unexpected argument '--release' found` reds the step (run 38926602107, job 117059642044, PR #10).
  8. *AC6.6 root sites* (local, root) — for each test: build its binary with `cargo test -p maos-bin --test maos_uninstall_16_4 --no-run` or `cargo test -p maos-kernel-core --test private_forget_restart_13_5i --no-run`, then run `sudo env CI=1 <binary> --exact <test>`. Tests: `purge_refuses_an_unopenable_store_directory_as_lock_unavailable`, `purge_failure_mid_deletion_leaves_an_in_progress_receipt`, `forget_fails_closed_when_pid_directory_is_unreadable`. Expect exit 101 with the named-capability panic each time. Save the output under `20-3a-evidence/`. **DONE 2026-10-16** (the dev container has passwordless sudo): each test exits 0 as uid 1000, named SKIP and exit 0 as root without CI, and exit 101 as root with `CI=1` — `20-3a-evidence/t13-ac6-6-root-sites.log`.
  9. *docs-site* — a change to `xtask/error-catalog.toml` alone must trigger `docs-site.yml`. **DONE 2026-10-16:** branch `ci/20-3a-plant-docs-filter` was pushed at `875831a4` (no docs-site run), then commit `37c03bfb` touching only `xtask/error-catalog.toml` was pushed and triggered docs-site run 38923215324. That run is green: `gate:behavioral` 3m18s, `gate:proven-red` green, so the `expected_distinct_pages` 133 fix also holds in CI.

- **§T13 CI evidence — discipline run 38807569588 and docs-site run 38807569253** (push of `389a07e9` to `main`, 2026-10-15). At first these were read through the public Actions REST API. From 2026-10-16, `gh` (installed to `~/.local/bin`, authenticated by the operator; R5) supplied the job logs. The previous full discipline run on `main` was green run 37658103096 @ `8f6869b8` (2026-10-07). The 17-1, 17-2 and 17-7 landings never ran full discipline on `main`, so some reds below predate 20-3a.

  | Job | Result | Cause | Action |
  |---|---|---|---|
  | `t3-escape-corpus` (24.04 / 26.04) | ✅ jobs 116672290227 / 116672290266 | — | AC11.1/11.3 proof: accounting step green on both images |
  | `workspace-test-suite` step 8 (air-gap) | ✅ job 116672297542 | — | AC6.2 proof; closes deferred-work line 1036 |
  | `docs-site` `gate:behavioral` | ❌ 10-min timeout | **20-3a**: `expected_distinct_pages` 136 assumed +10 pages. Every `Family::Variant` route renders its family's first page (React Router reads `:Variant` as a param), so the new pages add only +7. The a11y test failed and CI's 2 retries ran past 10 min. Reproduced locally with the CI Playwright suite: baseline 126 = 126, landing 133 | → 133. Local behavioral 131/132 in 2.1 min; the one fail (`fallback.spec`) fails identically on the baseline build (host fonts). **CI green: docs-site run 38923215324** (behavioral 3m18s). Route defect filed to `20-5` |
  | `nfr-rel-11-halt-receipt-999pct` | ❌ "Assert the two halves still sum" | **20-3a**: the harness-ownership control made `root_shutdown_unload_16_3` 9 + 3 = 12 tests; the guard hard-codes 11 | guard and step name → 12 / 9 |
  | `check-trial-attestation` | ❌ | predates 20-3a (since 17-1): the job runs all of `maos-eval`, including the T3 executor, which panics by design on declared CI without T3. Reproduced with `CI=true GITHUB_ACTIONS=true` | scoped to `--lib --test trial_attestation` (50 + 11 pass under CI env) |
  | `check-breaking-md` | ❌ | predates 20-3a (17-7): the entry used `- **Migration:**` bullets; the gate needs a `**Migration:**` line | operator ruling 2026-10-16: one `**Migration:**` line over the same bullets |
  | `check-dev-model-tier` | ❌ | predates 20-3a (17-2): the record names `openai-codex/gpt-6-astra`, absent from the frontier allowlist | operator ruling 2026-10-16: `gpt-6-astra` allowlisted (xtask +9 from rustfmt layout, re-pinned) |
  | `reproducible-build` `cargo deny check` | ❌ | advisory DB, not code: rustls 0.23.45 (RUSTSEC-2026-0346…0349), wasmtime/wasmtime-wasi 48.0.5 (0352…0364), unsafe-libyaml unmaintained (2024-0449) | operator ruling 2026-10-16: rustls → 0.23.46 and wasmtime → 48.0.6 (patch bumps); unsafe-libyaml documented ignore + deferred migration row. `cargo deny check` now reports advisories/bans/licenses/sources all `ok`; `cargo check --workspace --all-targets` ok; maos-wasm-host 92 and maos-a2a-tcp 64 tests pass |
  | `check-migration-merkle` | ❌ ABSENT rc 1 | R1 consequence (no 1e6 corpus in CI) | **R1 option A (operator ruling 2026-10-16):** the job regenerates the pinned corpus with its deterministic generator (1m30s in CI) and runs the gate. In verify run 38926594989 (PR #9, job 117059607470) with a Postgres 16 service: `PROVEN_BLOCKING`, `partial: false`, live cross-check true. That service then red `check-loom-substrate-drift`: a `services.postgres` gate job must be a registered substrate contract, and those contracts are the evidence-ledger set. The service is therefore dropped (`d237782a`); the gate stays PROVEN on the four re-derived oracles with `partial: true`, and the live run is filed (deferred-work). |
  | `workspace-test-suite` step 14 (full suite) | ❌ exit 101 after 16 min | 17-7 `s3_external_sigkill_requires_pidfd_fact`: 10.03 s against a 5 s bound on podman 4.9.3, residue `TerminationUnconfirmed` (log via `gh`, which is now authenticated). It took 10.03 s again in run 38987395150, i.e. the full `SNAPSHOT_BUDGET` | Bound calibration routed to `20-7` (deferred-work); intermittent, not a 20-3a regression |
  | `check-multi-tenant-loom` | ❌ | Red on every run, each time on a different live-T3 test. **Measured cause:** the gate's whole-package `cargo test -p maos-bin` leg ran outside the 17-2 delegated systemd cgroup that `workspace-test-suite` provides. In throwaway runs on `ci/20-3a-loom-diag`, the plain leg red 6/6 (38985809156) and the delegated leg green 3/4 (38986836808, 38987395150). A first hypothesis (podman-lock contention; per-binary lease `d237782a`) was falsified in CI and reverted (`0bdaffb1`) | **Operator ruling 2026-10-16:** the gate step runs inside `scripts/run_cgroup_ceiling_17_2.py --mode delegated`, and the wrapper passes the loom substrate env and the GitHub run binding (`84944db4`). Green in verify run 38988216311 and in plant run 38988216897 attempt 3 |
  | `v1-0-ship-gate`, `v1-5-ship-gate`, `aggregate` | ❌ | downstream of the above | — |

  All other 173 discipline jobs passed, including `check-reza-production-path`, `check-escape-detector`, `check-j4-latency`, `check-wasm-form-equiv`, `check-multi-region-slo` (the D7/F7 steps), `check-fuzz-targets` and `check-unsafe`. `journal-append` and `multi-provider` passed.

  Verify run 38926594989 (PR #9, `875831a4`) was 179/184 green: docs-site, nfr-rel-11, trial-attestation, breaking-md, dev-model-tier, reproducible-build, migration-merkle and workspace-test-suite all passed. It was red only on `check-loom-substrate-drift` (the Postgres service, fixed above) and `check-multi-tenant-loom` (fixed above), plus the three aggregates. Plant runs: PR #10 (38926602107), PR #11 (38926608301), and the dispatches 38926617566 / 38926622364 / 38926626626. Extracted lines are in `20-3a-evidence/t13-ci-plants.log`.

  **Final verify:** run 38988216311 (PR #9 @ `84944db4`) is 184/184 green, `aggregate` included.
  - Attempt 1 failed only `workspace-test-suite`, on the 16-3 `ac4_v3b` 5 s orphan-bound flake (routed to `20-7`). Re-running the failed jobs gave attempt 2 green.
  - `check-migration-merkle` regenerated the corpus and was PROVEN; `check-multi-tenant-loom` was green inside the delegated envelope.
  - The `t3-escape-corpus` jobs were green, and so were `docs-site`, `nfr-rel-11` (12 = 9 + 3), `check-trial-attestation`, `check-breaking-md`, `check-dev-model-tier` and `reproducible-build` (`cargo deny`).

### File List

- `.github/workflows/discipline.yml`
- `.github/workflows/docs-site.yml`
- `.github/workflows/fuzz-cadence.yml`
- `.github/workflows/fuzz-prerelease.yml` (new)
- `.github/workflows/journal-append.yml`
- `.github/workflows/rpo-rto-cadence.yml`
- `BREAKING.md`
- `Cargo.lock`
- `_bmad-output/implementation-artifacts/20-3a-evidence/creation-measurements-6537454e.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t0-baseline-6537454e.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/commands.json` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/endpoint-reproduction.json` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/endpoint-reproduction.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/fixture-build.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/package-run-1-orphans.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/package-run-1.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/planted-panic-red-replant.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/planted-panic-red.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-1-orphans.txt` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-1.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-2-orphans.txt` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-2.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-3-orphans.txt` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t11-harness-residuals/whole-package-run-3.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t14-release-graph-steps.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t14-t3-accounting-plants.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-gate-honesty.md` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t13-ac6-6-root-sites.log` (new)
- `_bmad-output/implementation-artifacts/20-3a-evidence/t13-ci-plants.log` (new)
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`
- `crates/maos-a2a-tcp/tests/h_guards.rs`
- `crates/maos-a2a-tcp/tests/support/mod.rs`
- `crates/maos-audit/src/erasure/regional_teardown.rs`
- `crates/maos-bench/tests/t_10_4c_j4_latency_gate.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/tests/erasure_uninstall_13_5b.rs`
- `crates/maos-bin/tests/maos_uninstall_16_4.rs`
- `crates/maos-bin/tests/root_shutdown_unload_16_3.rs`
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/methodology-attestation.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/capability_escape/capability_escape_cap_dac_override_003.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/capability_escape/capability_escape_cap_net_raw_002.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/capability_escape/capability_escape_cap_setuid_005.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/capability_escape/capability_escape_cap_sys_admin_001.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/capability_escape/capability_escape_cap_sys_ptrace_004.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/filesystem_escape/filesystem_escape_etc_passwd_001.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/filesystem_escape/filesystem_escape_home_004.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/filesystem_escape/filesystem_escape_host_root_005.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/filesystem_escape/filesystem_escape_proc_1_root_002.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/filesystem_escape/filesystem_escape_var_log_maos_003.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/network_escape/network_escape_dns_resolution_003.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/network_escape/network_escape_ipv6_outbound_005.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/network_escape/network_escape_raw_socket_004.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/network_escape/network_escape_tcp_outbound_001.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/network_escape/network_escape_udp_outbound_002.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/process_escape/process_escape_fork_bomb_004.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/process_escape/process_escape_kill_pid_1_003.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/process_escape/process_escape_mount_005.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/process_escape/process_escape_proc_1_exe_002.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/process_escape/process_escape_ptrace_host_pid_001.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/runtime_escape/runtime_escape_docker_sock_001.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/runtime_escape/runtime_escape_image_pull_via_sock_003.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/runtime_escape/runtime_escape_podman_sock_002.json` (new)
- `crates/maos-eval/fixtures/t3-escape-corpus-v1/runtime_escape/runtime_escape_runtime_config_rewrite_004.json` (new)
- `crates/maos-eval/test-fixtures/t3-network-escape-probe/Cargo.toml`
- `crates/maos-eval/test-fixtures/t3-network-escape-probe/src/main.rs`
- `crates/maos-eval/tests/t3_escape_workflow.rs` (new)
- `crates/maos-eval/tests/t3_network_escape_executor.rs`
- `crates/maos-journey-test/tests/jb3_self_tuning_halt.rs`
- `crates/maos-journey-test/tests/journey_butler.rs`
- `crates/maos-journey-test/tests/journey_j0.rs`
- `crates/maos-journey-test/tests/journey_j3.rs`
- `crates/maos-journey-test/tests/journey_j4.rs`
- `crates/maos-journey-test/tests/journey_researcher.rs`
- `crates/maos-journey-test/tests/support/binaries.rs` (new)
- `crates/maos-kernel-core/tests/hot_swap_halt_continuity_test.rs`
- `crates/maos-kernel-core/tests/private_forget_restart_13_5i.rs`
- `crates/maos-kernel-core/tests/t3_terminal_helpers_17_7.rs`
- `crates/maos-telemetry/tests/otel_gates.rs`
- `deny.toml`
- `docs-site/docs/errors/ECrossWallRecallDenied.md` (new)
- `docs-site/docs/errors/EModelProvenanceMissing.md` (new)
- `docs-site/docs/errors/EModelProvenanceStale.md` (new)
- `docs-site/docs/errors/EPinMismatch-FingerprintCollision.md` (new)
- `docs-site/docs/errors/ERegionTagInvalid.md` (new)
- `docs-site/docs/errors/ERegionViolation.md` (new)
- `docs-site/docs/errors/ESkillStore-Io.md` (new)
- `docs-site/docs/errors/ESkillStore-Json.md` (new)
- `docs-site/docs/errors/ESkillStore-UnknownSchemaVersion.md` (new)
- `docs-site/docs/errors/ETeamIdInvalid.md` (new)
- `docs-site/route-manifest.json`
- `docs/adr/ADR-059-operator-authority-collective-erasure.md`
- `docs/ci-baselines/README.md`
- `docs/errors/error-catalog.json`
- `docs/pen-test/findings/README.md`
- `docs/red-team/README.md`
- `docs/red-team/results/red-team-results-schema.toml`
- `docs/runbooks/dr-1-restore-drill.md`
- `docs/runbooks/fuzz-cadence.md`
- `docs/runbooks/provisioning-checklist.md`
- `docs/third-party-trial/README.md`
- `docs/third-party-trial/results/trial-results-schema.toml`
- `scripts/collect-ledger.sh` (new)
- `scripts/record-fuzz.py` (new)
- `scripts/run_cgroup_ceiling_17_2.py`
- `scripts/run-fuzz-link.sh` (new)
- `tests/harness/ci.rs` (new)
- `tests/harness/t3_substrate.rs`
- `xtask/error-catalog.toml`
- `xtask/gate-registry.toml`
- `xtask/kloc.toml`
- `xtask/src/abi_diff.rs`
- `xtask/src/check_cohort_mesh.rs`
- `xtask/src/check_dev_model_tier.rs`
- `xtask/src/check_epic_close_coherence.rs`
- `xtask/src/check_error_catalog.rs`
- `xtask/src/check_escape_detector.rs`
- `xtask/src/check_fuzz_floor.rs`
- `xtask/src/check_migration_merkle.rs`
- `xtask/src/check_pentest_gate.rs`
- `xtask/src/check_red_team_gate.rs`
- `xtask/src/check_reza_production_path.rs`
- `xtask/src/check_rto_gate.rs`
- `xtask/src/check_ship_gate_completeness.rs`
- `xtask/src/check_skill_conformance.rs`
- `xtask/src/check_third_party_trial.rs`
- `xtask/src/check_trial_attestation.rs`
- `xtask/src/check_wasm_form_equiv.rs`
- `xtask/src/corpus_types.rs`
- `xtask/src/evidence_ledger.rs`
- `xtask/src/gate_common.rs`
- `xtask/tests/cgroup_ceiling_workflow_17_2.rs`
- `xtask/tests/d11_xtask_ceiling_ratchet.rs`
- `xtask/tests/diagnostics_20_3a.rs` (new)
- `xtask/tests/epic17_exit_workflow_17_7.rs`
- `xtask/tests/error_catalog_20_3a.rs` (new)
- `xtask/tests/gate_honesty_20_3a.rs` (new)
- `xtask/tests/kernel_pin_ci_wiring_20_3a.rs` (new)
- `xtask/tests/ledger_collectors_20_3a.rs` (new)
- `xtask/tests/pentest_gate_tests.rs`
- `xtask/tests/ship_gate_binding_20_3a.rs` (new)
- `xtask/tests/story_10_2_proven_red.rs`
- `xtask/tests/story_10_4a_ac1_proven_red.rs`
- `xtask/tests/story_10_5_proven_red.rs`
- `xtask/tests/workflow_honesty_20_3a.rs` (new)

### Change Log

- 2026-10-15 — created `ready-for-dev`; epic §20-3a OLD→NEW block filed; register D1/D9 deadlines re-anchored to *reaches `done`*; sprint row flipped.
- 2026-10-15 — Q1–Q5 ruled by consensus under the operator's decision rule (long-term correctness, PRD-strict, correct drift, honour contracts, fill roadmap gaps): AC3.4–3.5 widened (libFuzzer-counter records, T2 nightly, T3 24 h chain), AC11 added (all 25 T3 scenarios executed), D9 CLOSED by decision, `:1029` production fix routed to 20-3c, rc-tag T3 precondition routed to 20-5, D9's post-promotion lapse consequence routed to 20-1.
- 2026-10-15 — operator sign-off S1–S3 given (Lunarpulse). No open decision remains. Status: `ready-for-dev`.
- 2026-10-15 — dev (`bmad-dev-story`): T0–T10b, T12 and T14 done, with operator rulings R1–R4 taken during dev (Debug Log). Slices were built in worktrees and merged on `20-3a/integration`. T14 ran nine review layers; every finding is patched or dismissed with grounds (§Review Findings). At landing, AC7.2's defect class was also fixed in D7/F7. kloc granted at the exact measured figures, aggregate measured + 1 (S3). One local landing commit, nothing pushed (R2). **HALTED**: T13 (CI proofs, recipes in Completion Notes) and T11's three CI-dependent closures remain. Status: `in-progress`.
- 2026-10-16 — T13 CI reading of the operator's push (discipline 38807569588, docs-site 38807569253). Fixed the two reds 20-3a caused: the docs-site `expected_distinct_pages` (136 → 133, measured) and the nfr-rel-11 split guard (11 → 12). Scoped `check-trial-attestation` to its own targets (red since 17-1). Per operator rulings: reformatted the 17-7 BREAKING.md Migration block, allowlisted `gpt-6-astra`, bumped rustls 0.23.46 and wasmtime 48.0.6, and documented the unsafe-libyaml ignore. Deferred-work line 1036 CLOSED with CI evidence; line 1128 records 24/24 on both images; two rows filed (colon-slug routes → 20-5; serde_yaml migration, ownerless-dispositioned). kloc: xtask 44812, aggregate 176010. Still HALTED: `workspace-test-suite` and `check-multi-tenant-loom` need their job logs, plus the remaining T13 plants.
- 2026-10-16/17 — T13 completed in CI with `gh` (R5).
  - **R1 option A:** `check-migration-merkle` generates the pinned corpus in CI. The Postgres service was dropped after it red the loom drift gate; the live cross-backend run is filed as deferred work.
  - **Loom leg:** `check-multi-tenant-loom` runs inside the 17-2 delegated envelope (cause measured on `ci/20-3a-loom-diag`). A falsified T3-lease hypothesis was tried and reverted.
  - **Plants:** all are red in CI — AC3.2 ×2, AC3.5, AC6.2 ×2, AC7.2, AC11.4 ×2 images, and AC5.1 isolated. AC6.6 root sites and the AC8.2 path filter were recorded.
  - **Ledgers:** deferred-work lines 1036 and 1128 CLOSED; E16-A2 `done`. Rows filed: T3 wall-clock bounds → `20-7`; ledger re-run binding and the migration live substrate, both ownerless-dispositioned.
  - **Verify:** run 38988216311 is 184/184 green.
  - Status → `review`.
