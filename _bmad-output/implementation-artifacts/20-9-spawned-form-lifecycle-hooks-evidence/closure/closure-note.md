# 20-9 closure note (2026-11-12)

Story `20-9-spawned-form-lifecycle-hooks` -> `done` (records only; orchestrator under delegated authority, same precedent as the 20-13 AC7.2 amendment). Written by `Close209Records`. Every id below was read from a file in this evidence tree or from `gh run view`; nothing is quoted from memory.

Evidence root: `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks-evidence/` (`E/` below).

## 1. Commits and runs

| What | Ref |
|---|---|
| L1 (kernel-free, AC1-AC5) | `9f03064b`, `c8e5d035`, `f56959c2`, `fba6af2a`, `459a606a` |
| L2 (kernel, AC7, AC8, AC9 minus AC9.1), reviews | `6bb4f339`, reviews `49945857`; hosted discipline run 43149216081 on `6bb4f339`, 199/199 green (confirmed with `gh run view` by the earlier truth table) |
| Landing 3 (kernel-free product-defect fixes) | `fe44017a` |
| Hosted AC6 proof | run 43217945522, branch `ci/20-9-verify-2`, commit `18af9303` = main `fe44017a` + only `.github/workflows/ci-20-9-verify.yml`; run conclusion `failure` as intended (only the AC6.2 plant job is red); 9 of 10 jobs green |
| Superseded recipe runs | 43179221668, 43182348879, 43185836600, 43188123642, 43193905404 (`E/ci/run-*.md`), 43212140215 (recipe bugs, `E/ci/run-43212140215.md`) |

## 2. AC by AC

| AC | Verdict | Evidence |
|---|---|---|
| AC1 `maos:spirit@2.4.0` lifecycle interface, runner, probe guest | LANDED, hosted | `9f03064b`; `E/landing-1/plant-ac1.log`; four-image `maos-wasm-host` 32/32 in run 43217945522 (jobs 131181865881, 131181866431, 131181865938, 131181867170), incl. `lifecycle_runner_20_9` 4 PASS, `guest_stdio_t2_20_9` 3 PASS |
| AC1.5 TS template/example, docs EN/KO | LANDED, hosted | `9f03064b`, `fe44017a` (`cpu_max_pct` 100); componentize-js node 22 job 131181865987 and node 24 job 131181865280, vitest 14/14 plus build and componentize |
| AC2 budgeted triggers reach a spawned Spirit | LANDED, hosted | `9f03064b`..`459a606a`; `E/landing-1/plant-ac2.log`; `spawned_lifecycle_20_9` 15 PASS on four images |
| AC3 `[[schedule]]` at admission | LANDED | `9f03064b`; `E/landing-1/plant-ac3.log`; `erasure_live_20_11` updated for the new truth |
| AC4 spawned hot swap, upgrade, revert | LANDED, hosted | `9f03064b`, `c8e5d035`; `E/landing-1/plant-ac4.log`; `spawned_swap_20_9` 27 PASS and `spawned_door_20_9` 6 PASS on four images |
| AC5 cold swap, Worker refusal | LANDED, hosted | `9f03064b`; `E/landing-1/plant-ac5.log`; the cold-swap door vector green on both arm images (7.16 s / 6.98 s) after the `fe44017a` race fix |
| AC6.1 hosted T2 swap, same pid, one declaration row, counter | **SATISFIED AS AMENDED (D1)** | run 43217945522: shipped example job 131181865598 (attempt 5 completed, `latency_ns` 29,503,939,836, pid 1 -> 1, one successor declaration row, one `lifecycle.hot_swap` row); counter on the recipe-local variant job 131181865721 (TS rows 1..96 strict, last N=80 before the swap row, first tick after = 81, 0 dropped rows; Spirit pid 1, runner child 16274 -> 16326); `E/ci/run-43217945522.md`, `E/ci/hosted209b-report.md`, `E/ci/logs/run7-43217945522/` |
| AC6.1 `wasm-host-tests` runs AC1-AC5; `check-wasm-form-equiv` green | MET | four images 32/32 and 132/132 (1 skipped) incl. both new cold-swap witnesses by name; `check-wasm-form-equiv` job success in run 43149216081 |
| AC6.2 hosted plant | MET (D2) | job 131181865685: swap step red with `spawned_swap_unsupported: the running Spirit exports no lifecycle`, judge step green; run `failure` is the intended plant |
| AC7 `poll_once`, `PollOutcome`, FR26 vectors, plant | LANDED | `6bb4f339`; `E/landing-2/failing-before-poll-once-compile.log`, `failing-before-fr26-vectors.log`, `plants/` |
| AC8 D07 A suppression, revoke, `schedule.suppressed` row | LANDED | `6bb4f339`; kernel 27,103 / 21,256, STOP 22,000; plants `E/landing-2/plants/plant-ac8-*.log` |
| AC8.4/AC8.5 pins, FLAG record, docs | LANDED | `6bb4f339`; `20-9-flag-winston-decision.md`; ADR-013, ADR-030, cookbook |
| AC9.1 `docs/invariants/I14.md` | SKIPPED by operator ruling 2026-11-12 (D4) | row re-owned to `21-10-invariant-ledger-proofs` in `deferred-work.md`; witnesses named in `tests/coverage-matrix.yaml` FR10; I14 claim stays `asserted` |
| AC9.1 coverage matrix FR10/FR26/FR55 | LANDED | `tests/coverage-matrix.yaml` |
| AC9.2 `deferred-work.md` PF-04 and D07 A rows | CLOSED | stamped `@6bb4f339` (this closure) |
| AC9.2 RA `:173`/`:182`, RI `:566`/`:602` | CLOSED | `routing-audit-2026-10-22.md` FR10 and FR55 rows and `epics/requirements-inventory.md` FR55 rows, closed for spawned forms with this evidence; the F3 limit is cited in them |

## 3. Orchestrator decisions (2026-11-12, delegated authority)

- **D1.** AC6.1 is satisfied by the branch-only recipe job `epic-20-exit-swap` in run 43217945522, not by main's `discipline.yml` `epic-20-exit`; folding the swap leg into main is a deferred row owned by `21-9-wasm-host-default-on` (it changes what the default artifact proves). The "idle, v+1 hot swap, same pid, counter continues" text is amended to what is provable: hot swap on the unmodified shipped example, counter continuity on a recipe-local variant that ticks in `consolidate`. "idle N+1 after the swap" is not shown on the shipped example (F3). The original AC6.1 text is kept in the story under a dated amendment note.
- **D2.** AC6.2 plant: job 131181865685 swap step red, judge green. Four images 32/32 `maos-wasm-host` and 132/132 `maos-bin` (1 skipped), both new cold-swap witnesses by name; componentize-js node 22/24 green.
- **D3.** Product findings F1-F5 are not fixed in 20-9. Each is an open rule-10 row owned by `21-9-wasm-host-default-on` with a trigger and is listed in `docs/release/v1.0.0-rc.1.md` known limitations. F1 a denied `author()` ends the session `OutOfFuel` (exit 4; job 131181865883). F2 the successor's `swap-in` `console.log` is dropped (`Prewarm::next_stdout`, `spawned_lifecycle.rs:160-162`). F3 `idle` fires once per quiescence (`idle_watchdog.rs:93-95`). F4 shutdown `lifecycle_session_ended` panic caught by the pump (`supervision.rs:2749:30`). F5 guest `setTimeout` residual (expected panic, not exercised hosted). Severity Low/P3 for the rc: no security or data-loss effect.
- **D4.** AC9.1 stays skipped by operator ruling.
- Recipe note: `E/ci/ci-20-9-verify.yml` changed in runs 6 and 7; the earlier run files carry a dated note and the previous version is in git history at `fe44017a`.

## 4. Reviews

| Review | Reviewer | Result | Where |
|---|---|---|---|
| L1 rounds 1-4 (GPT and GLM security) | GPT / GLM | all findings fixed or dispositioned in `deferred-work.md` | `E/landing-1/review/` |
| L2 | GPT review, GLM security review | GPT 2 P3, GLM 2 Low + 2 Info, 0 P0-P2; every finding a rule-10 row | `E/landing-2/review/gpt-review.md`, `security.md` (`49945857`) |
| Landing 3 console fix | GPT review and GPT security reviewer | 0 P0-P3 findings beyond the repin P2 (applied) | `E/landing-3/review/console-fix-gpt-review.md`, `console-fix-security-gpt-reviewer.md` |
| Landing 3 cold-swap race fix | GPT review | P0 0, P1 0, P2 0, P3 0 | `E/landing-3/review/race-fix-gpt-review.md` |

Security-review substitution disclosure: GLM quota is exhausted until 2026-11-15; the landing-3 security reading ran on GPT (a different family from the Anthropic implementer). Landing-3 gate results: `E/landing-3/gates/`.

## 5. Deferred rows

Every row owned by or triggered on 20-9 was closed with a citation or re-owned to an existing live key (`21-9-wasm-host-default-on`; the I14 row to `21-10-invariant-ledger-proofs`). Kernel follow-ups need no operator grant inside the ADR-069 envelope (ruling `kfree`, `decisions-2026-11-12-dedrift.md` row 6). F1-F5 were added under "Observed during: 20-9 hosted proof". Lines are `deferred-work.md` as of this closure (rows after line 1732 shift by +10 for the new F rows).

| Line | Row | Before | After |
|---|---|---|---|
| 1359 | PF-04 refused-poll observer | CLOSED text, stamp owed | stamped `@6bb4f339`; owner cue removed (routed-to wording) |
| 1375 | D07 A untokened fire | CLOSED text, stamp owed | stamped `@6bb4f339`; owner cue removed |
| 1505 | `[[schedule]]` admission (20-11 T0 note) | owner 20-9 AC3 | CLOSED by AC3 `@9f03064b` |
| 1630, 1675 | Promotion-set swap and crash cleanup (kernel-side) | owner 20-9 | `21-9-wasm-host-default-on`; trigger next kernel touch of `[intent_promotion_set]`; `kfree` |
| 1659 | AC9.1 `I14.md` | owner 20-9, dispositioned | `21-10-invariant-ledger-proofs` (I14 ledger owner); D4 |
| 1660 | L2 not-taken umbrella | owner 20-9 until hosted run | `21-9-wasm-host-default-on` (index; sub-rows carry the triggers) |
| 1661 | Suppressed-fire `reason` untyped (Info) | owner 20-9 | `21-9-wasm-host-default-on` |
| 1662 | Provisional FR11 record newest during cold-swap start | owner 20-9 | `21-9-wasm-host-default-on` |
| 1663 | kloc overdrafts, record only | owner 20-9 | CLOSED (record complete); owner cue removed |
| 1664 | L2 hosted legs stay hosted | owner 20-9 | CLOSED by run 43217945522 |
| 1665 | `DiscardSuccessor` phase (GPT P3-1) | owner 20-9 | `21-9-wasm-host-default-on`; next kernel touch of `hot_swap/`; `kfree` |
| 1666, 1667 | `archive.rs` temp-file leak; predictable symlink-following temp path | owner 20-9 | `21-9-wasm-host-default-on`; next kernel touch of `hot_swap/archive.rs`; `kfree` |
| 1668 | 86,400 s guest starves watchdogs | owner 20-9 | `21-9-wasm-host-default-on`; before third-party manifests are default-admitted; `kfree` |
| 1669 | no `[[schedule]]` cap | owner 20-9 | `21-9-wasm-host-default-on`; next manifest-schema revision |
| 1670 | revoke-failure `eprintln!` | owner 20-9 | `21-9-wasm-host-default-on`; next touch of `schedule_watchdog.rs`; `kfree` |
| 1674 | L1 hosted-only legs unverified | owner 20-9 | NARROWED: AC6, node 22/24, four images CLOSED by run 43217945522; macOS/Windows compile half re-owned to `21-9-wasm-host-default-on` |
| 1676 | Post-swap revert cannot restore the retired child | owner 20-9 | `21-9-wasm-host-default-on` (D-20-9-11 A by design; B is a kernel line, `kfree`) |
| 1677 | Swap waits for a component still starting | owner 20-9 | `21-9-wasm-host-default-on`, with the hosted observation (4 refusals, ~20 s, job 131181865598) |
| 1678 | Bin test worlds leave a temp dir | owner 20-9 | `21-9-wasm-host-default-on` (opportunistic) |
| 1679 | Interim state between cold-swap steps | CLOSED, stamp owed | stamped `@6bb4f339` |
| 1684, 1685, 1686 | Telemetry scope; no FR11 record for hot-swapped successor; successor scope check | owner 20-9 L2 | `21-9-wasm-host-default-on` |
| 1695 | `IdleWatchdog` serial fire | in-poll closed, residual owner 20-9 | stamped `@6bb4f339`; next-poll residual `21-9-wasm-host-default-on`; `kfree` |
| 1696, 1697, 1698, 1699 | Hook cancel cancels the wait; `write_frame` backpressure; unbounded `upgrade_lock`; round-5 witnesses | owner 20-9 L2 | `21-9-wasm-host-default-on` |
| 1706, 1707 | Teardown race (GLM r3 F2); archive atomicity (GLM r3 F3) | closed, stamp owed | stamped `@6bb4f339`; owner cue removed |
| 1708, 1709, 1710 | Global `upgrade_lock`; `redistill_results` slots; round-4 witnesses | owner 20-9 L2 | `21-9-wasm-host-default-on` |
| 1717-1721 | Artifact TOCTOU; `authorize_principal_writes`; TL write under the pump mutex; slow subscriber; `LifecycleCallers` unwind | owner 20-9 L2 | `21-9-wasm-host-default-on` (1718 kernel, `kfree`) |
| 1733-1742 (new) | F1-F5 hosted-proof findings | absent | OPEN, `21-9-wasm-host-default-on`, triggers, Low/P3 (D3) |
| 1760 | Swap-invariant refusal has no WASM witness (was 1750) | owner 20-9, rule 10(b) | `21-9-wasm-host-default-on` |
| 1763 | Coordinator not self-serializing (was 1753) | owner 20-9 | `21-9-wasm-host-default-on`; next kernel touch of `hot_swap/coordinator.rs`; `kfree` |

## 6. Not proven and known limits

- The release-daemon swap legs (jobs 131181865598, 131181865721) ran on ubuntu-24.04 x86_64 only; the arm and 26.04 images ran the `maos-bin`/`maos-wasm-host` vectors.
- The swap legs are a branch-only job, not main's `epic-20-exit`; main never ran them.
- "idle N+1 after the swap" is not shown on the shipped example (F3).
- Counter continuity is shown on a recipe-local variant, not on the shipped example; the successor's own `swap-in` log line was never observed (F2).
- No macOS or Windows compile leg for `wasm-host` ran (row re-owned to `21-9-wasm-host-default-on`, trigger: the default build turns `wasm-host` on).
- `SESSION_END_WAIT` = 15 s was chosen from the teardown path, not measured on a slow runner (`E/landing-3/fixes-p1-f2.md`).
- **Hosted discipline for `fe44017a` (run 43211291217) completed with conclusion `failure` at 13:28:33Z, 194 of 199 jobs success. It is NOT a green run and no green is claimed.** Non-green jobs: `workspace-test-suite` (6103 tests run, 6102 passed, 1 failed: `maos-bin::worker_terminal_facts_17_7 s3_external_sigkill_requires_pidfd_fact`, 10.98 s, the known podman 4.9.3 pidfd-fact flake; rows in `deferred-work.md` at the `s3_external_sigkill` entries near line 1308), `check-multi-tenant-loom` (`root_shutdown_unload_16_3::ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker` panicked at `root_shutdown_unload_16_3.rs:1436`; the known row owned by `ops-stable-perf-runner`, `deferred-work.md` line 7), and the three aggregating jobs `v1-0-ship-gate`, `aggregate`, `v1-5-ship-gate` (they fail because of the two above). I read the failing test names from `gh run view --log-failed`; I did not diff either failure against earlier occurrences beyond matching the rows by name.
- The 20-8 plant leg (job 131181865883) shows the 101 panic gone but the session still ends `OutOfFuel` (F1).

## Correction 2026-11-12: `done` was recorded over a red hosted run

20-9 was recorded `done` while the hosted discipline run on its product tree (`fe44017a`, run 43211291217, conclusion `failure`) was red. The non-green jobs, checked with `gh run view 43211291217 --repo lunarpulse/maos --json jobs`, are `workspace-test-suite` (`maos-bin::worker_terminal_facts_17_7 s3_external_sigkill_requires_pidfd_fact`), `check-multi-tenant-loom` (`root_shutdown_unload_16_3::ac4_v3b_sigterm_mid_topology_unloads_what_loaded_and_nacks_the_worker`, panic at `crates/maos-bin/tests/root_shutdown_unload_16_3.rs:1436`) and the fan-in jobs `v1-0-ship-gate`, `aggregate` and `v1-5-ship-gate`. Section 6 above already lists this run as not green; this section states the consequence for the status explicitly.

- No acceptance criterion of 20-9 is changed by this correction.
- The `ac4_v3b` fix is in progress before the rc candidate (agent FixAc4v3b); its result is to be recorded in the post-tag record commit.
- The `done` status stands per the reviewer's instruction. Re-open 20-9 only if the `ac4_v3b` fix changes an acceptance criterion.
