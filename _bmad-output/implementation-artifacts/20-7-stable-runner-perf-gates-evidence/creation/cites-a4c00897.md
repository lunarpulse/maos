# 20-7 creation — cites re-measured at `a4c00897` (rule 9)

Every range below was re-read at HEAD `a4c00897` (`git log -1 -- crates xtask` = `a4c00897`). "E20" =
`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`; "S" = `sprint-status.yaml`; "PF" =
`epic-20-preflight-2026-10-30.md`; "D" = `deferred-work.md`. The shared files (E20, D, PF) are NOT edited by this
creation: the epic corrections are owed through `records-owed.md` §A (orchestrator applies). The story file cites
the NEW values.

## C — epic §20-7 (E20:236-253) and its stories row (E20:79)

| # | Where | OLD | NEW at `a4c00897` |
|---|---|---|---|
| C1 | E20:240 (amendment) | advisory jobs `discipline.yml:2940-3013` | `nfr-perf-1-iac-routing-budget` `:2970-2983`; `nfr-perf-j4-latency` `:3022-3035`; `nfr-perf-8-orchestrator-fanout` `:3040-3055` (each `continue-on-error: true  # ADVISORY — owner: Murat — retired by 20-7 (ADR-068)`, `:2973,3025,3043`) |
| C2 | E20:244, :253 (AC7) | `discipline.yml:1693-1778` | as C1 |
| C3 | E20:240 | J4 budget advisory `:4891-4931` | `check-j4-latency` job `:4856-4896`; advisory comment `:4870`; mutation step `:4877-4887` |
| C4 | E20:238 | `tests/coverage-matrix.yaml:1326-1374` | NFR-Perf rows `:1380-1436` (Perf-1 `:1380` → `check-j4-latency`; Perf-2 `:1389`, -4 `:1405`, -5 `:1410`, -7 `:1427`, -8 `:1432` all `gates: []`; Perf-3 `:1394` → `cap_token_verify_assertion`) |
| C5 | E20:240 | coverage `:1364-1419` | as C4 |
| C6 | E20:238 | J1 budgets `crates/maos-bench/src/harness/j1.rs:183-190` | `j1.rs:25` (`J1_P95_BUDGET_US = 25_000`), `:162-170` (`J1_CI_CEILING_US = 50_000` `:170`) — PF:234 already corrected this; E20:238 still carries the old range |
| C7 | E20:248 (AC3) | J4 10 ms copies `j4.rs:36`, `decision.rs:15`, `j4_latency_regression.rs:25` | same three, plus the **J1 25 ms copy at `decision.rs:14`** (a fourth budget literal outside `j1.rs:25`) |
| C8 | E20:249 (AC4) | `iac_routing_budget.rs:37,153-172` | `:36-37` (`BUDGET_INVOCATIONS`, `P95_BUDGET_US = 1000`), `:122` (`budget_met = p95 <= …`), soft-fail `:157-170` |
| C9 | E20:249 (AC4) | "the existing `hot_swap_latency` bench" (implied maos-bench) | `crates/maos-kernel-core/benches/hot_swap_latency.rs` (`crates/maos-kernel-core/Cargo.toml:122-124`); benches are outside the kernel pin (`src/` only) and outside kloc (`xtask/src/kloc_check.rs:305`) |
| C10 | E20:244 | `check-j4-latency` "registered blocking at the current phase" | `xtask/gate-registry.toml:457-459`: `disposition = { v1_0 = "advisory", v1_5 = "advisory" }`, `binding = "advisory-substrate"` (20-3a AC7, D:1073 CLOSED) |
| C11 | E20:244 | `t_10_4c_j4_latency_gate.rs:76-102` | `:70-92` (advisory print `:86`) |
| C12 | E20:252 (AC6) | `check_release_precondition.rs` "refuses a tag SHA without PASS" | file is 96 lines and reads only the GitHub check-runs `aggregate` (`:33-60`); no ledger read exists — AC11 adds it |
| C13 | E20:242 | `deferred-work.md:1084-1094,1152-1154,1192` | `:1075` (NFR-Perf gates), `:1181-1191` (S3), `:1192-1198` (root-shutdown family; supervisor recurrence `:1197`), `:1256-1259` (`ac4_v4` / 18-0 allowance), `:1297-1298` (`cap_token_verify_p99`), `:96` (ac3 door-unload route budget) |
| C14 | E20:79, :238 | "kernel-Δ 0" | kernel-Δ **measured FLAG-Winston** if D-20-7-01/02 rule A (R = K1w + K2r: +40 physical / +31 tokei; R′ = K1 + K2r +26/+17; `kloc-kernel-a4c00897-and-prototypes.json`); 0 only if both rows rule a no-kernel option (D-01 C/D, D-02 C) |
| C15 | E20:79, :238 | xtask +250–450; maos-bench +150–300 | creation forecast: xtask +560…+990 [INFERENCE]; maos-bench `src/` +60…+200 [INFERENCE] (bench files uncharged); maos-bin 0…+40 [INFERENCE] (story §Kernel and kloc) |
| C16 | E20:242 | "Received from E19 … diagnose and fix … without weakening guarantees"; D:1198 "both remaining items are wall-clock calibrations of T3 teardown claims" | measured at creation: bimodal one-budget stalls with degraded evidence (S3 residue + client 125; ac4 root exit ≈ 10 s), not calibration (`flake-diagnosis.md`) |

## T — tracker S:304 / S:327

| # | OLD | NEW |
|---|---|---|
| T1 | S:304 `coverage-matrix.yaml:1335-1374` | `:1389-1436` (C4) |
| T2 | S:304 `decision.rs:15` | `decision.rs:14-15` (C7) |
| T3 | S:327 deliverable "fingerprint + calibration report committed with `docs/runbooks/perf-runner.md`" | the runbook does not exist (`docs/runbooks/` has 8 files, none `perf-*`); the provisioning row is `docs/runbooks/provisioning-checklist.md:44` (`absent`); `gh api …/actions/runners` total_count **0** (2026-11-01); branches `fuzz-ledger`, `rto-ledger` exist, **no `perf-ledger`** (`git ls-remote origin`) |

## P — preflight §3.4 / §6

| # | OLD | NEW |
|---|---|---|
| P1 | PF:232 `:2940-3013`; J4 `:4891-4931` | C1, C3 |
| P2 | PF:233 coverage `:1364-1419` | C4 |
| P3 | PF:306 S3 `D:1084-1086` | `D:1187-1191` |
| P4 | PF:307 ac4 `D:1087-1093,1152-1154` | `D:1192-1198,1256-1259` |
| P5 | PF:308 supervisor `D:1093,1140-1141` | `D:1197` |
| P6 | PF:297 (D13) | RULED 2026-11-01 (rows203c): A (`20-3c-…md:159`) |

## L — ledger rows (D) and brief

| # | OLD | NEW |
|---|---|---|
| L1 | D:1181 `worker_terminal_facts_17_7.rs:419-434` | bound `:415-425`, assertion `:426-430`, facts `:431-455` |
| L2 | D:1256 "the podman-4 5 s orphan bound already recorded at `:1158`" | `:1192` |
| L3 | D:1190 "41243340929 … The failure text is not recorded" | `10.039739216s (bound 5s, runtime podman version 4.9.3)`; `TerminationUnconfirmed`; client 125 (`ci-excerpts/run-41243340929-job-124661589790.txt`) |
| L4 | D:1191 "41281999213 … The failure text is not recorded" | `10.052044568s …`; `TerminationUnconfirmed`; client 125 (`ci-excerpts/run-41281999213-job-124775636112.txt`) |
| L5 | D:1259 "The failure text is not recorded in this row" | `the standalone Worker (pid 32438|61396) is still in /proc after 5s`; nextest `FAIL [16.772s]` (`ci-excerpts/run-41281999213-job-124775636052.txt`, `run-41289787129-job-124798452868.txt`) |
| L6 | D:1298 "The failure text is not recorded in this row" | `P99 = 5480ns, budget = 5000ns` (`ci-excerpts/run-41243340929-job-124661589929.txt`) |
| L7 | brief: S3 "again in 41233186774" | **wrong**: S3 passed (1.447 s) in job 124631445292; that run's red was `worker_freeze_19_2.rs:274` (TL-before-table, fixed `714a1428`) |
| L8 | brief: ac4 recurrence "41275…" | unresolvable: no run id with that prefix (`gh run list`, 2026-11-01) |
| L9 | D:1193 "job logs need a token this host does not have" | the operator's `gh` token on this host now reads job logs (all 14 excerpts fetched 2026-11-01) |

## H — anchors this story's ACs rely on (verified, unchanged)

`crates/maos-bench/src/harness/j4.rs:34-36,196`; `crates/maos-bench/benches/j4_latency_regression.rs:23-25`;
`crates/maos-bench/benches/orchestrator_fanout_nfr_perf_8.rs:22,57-66`; `crates/maos-kernel-core/tests/cap_token_verify_assertion.rs:1-4,67-111`;
`crates/maos-kernel-core/src/security/sandbox/t3/child.rs:23-30,102-148,196-269,273-280,325-375,401-466`;
`crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs:310-320,519-523`; `crates/maos-kernel-core/src/security/sandbox/t3/runtime_detect.rs:38-41`;
`crates/maos-bin/src/supervision.rs:451-463,849-853,1183-1221,1453-1470,1911-1945,2503-2504,2619,2934-2970,3201-3210`;
`crates/maos-bin/tests/root_shutdown_unload_16_3.rs:342-391,1349-1371,1427-1470,1475-1529,2101-2147`;
`crates/maos-bin/tests/worker_terminal_facts_17_7.rs:195-313,397-456`; `…/supervisor_scenes.rs:646-806`; `…/stub_scenes.rs:25-41,91-121`;
`crates/maos-bin/tests/worker_crash_corpus_16_3.rs:1-40,88-102` (16-3 corpus harness, 100 Workers in batches of 20);
`docs/adr/ADR-068-stable-performance-runner.md:51-170`; `docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`;
PRD `functional-requirements.md:41` (FR12), `non-functional-requirements.md:9-16` (NFR-Perf-1…8), `success-criteria.md:40-46` (six hot-path budgets);
`routing-audit-2026-10-22.md:48`; `epic-19-founder-loop-w4.md:187,194` + `19-3-…md:108,132,293` (D-19-3-11 C).
