# 20-7-stable-runner-perf-gates: closure under operator ruling r1 (2026-11-12)

Ruling r1: `_bmad-output/planning-artifacts/decisions-2026-11-12-acceleration.md` item 8. The story closes at L1 plus the rc waiver. L2 (T12 shadow, T13 bind) and the runner-only ACs (AC1.4, AC6.1, AC11.1/11.2/11.3, AC12.1) move to the UNCLAIMED backlog row `ops-stable-perf-runner` (`sprint-status.yaml`). This is not a widening of any time budget and not retry-to-green.

## Claim boundary (the honest statement)

**"perf verdicts ABSENT; no stable-runner timing claim."** Nothing here claims an NFR-Perf timing figure measured on a stable runner. No hosted-runner timing red was made green by widening a bound or by retrying. The 5 s root-exit bound (AC8.4) remains the product bound for the rc; the hosted ac4/supervisor reds are *dispositioned* as hosted timing noise, a disclosed disposition and not a proof: the cause of the historical ac4 orphan is still unidentified (D-20-7-11 C).

Landed commits: `e3231e88` (story), `1fd4de31` (landing 1), `9f64dd02` (review corrections), `850e4c00` (hosted-leg fixes, callgrind baselines), `a8af3215` (D-20-7-09/10/11), `ac1fc12a` (cold-swap oracle), `5fe97eda` (two flake repairs), `d00802aa` (20-5 perf-verdict release leg, `--perf-absent-ok` for the rc).

## Acceptance criteria

| AC | Disposition | Evidence |
|---|---|---|
| AC1.1-1.3 verdict command, fixture ledger, best-of-five plant | LANDED (`1fd4de31`, `9f64dd02`) | `xtask/tests/perf_gates_20_7.rs`; `landing-1/plants.md` |
| AC1.4 runner reds (J4 15 ms inject, noisy calibration) | MOVED to `ops-stable-perf-runner` | needs the runner |
| AC2 runner policy | LANDED (`1fd4de31`, `9f64dd02`) | hosted `check-perf-runner-policy` green in run 43150757804 |
| AC3 budget file | LANDED (`1fd4de31`) | `xtask/perf-budgets.toml`, literal-scan vector and plant |
| AC4 benches, six hot-path rows, completeness vector | LANDED (`1fd4de31`) except the production WASM `recall` leg | the leg is AMENDED out by D-20-7-10 A and is a rule-10 `ownerless and open` row (needs a guest harness; not runner-only) |
| AC5 Tier C | LANDED (`850e4c00`) | baselines 12944.0156, 997449.662, 188885.9382; plant run 41423701978 |
| AC6.2/6.3 hosted smoke and plant | LANDED (`850e4c00`) | run 41423701978 |
| AC6.1 1 h soak, 50 Workers | MOVED to `ops-stable-perf-runner` | runner-only |
| AC7 S3 / K1w | LANDED (`1fd4de31`) | stub vector plants 3/20, 3/20, 2/20; hosted `s3-both` green on 24.04 and 26.04 in 41423701978 |
| AC8 K2r defense-in-depth | LANDED, AMENDED | AC8.3 retracted by D-20-7-11 C; AC8.4 amended for the rc (5 s product bound unchanged; hosted timing-noise reds dispositioned under r1; cause unidentified) |
| AC9 supervisor barrier | LANDED (`1fd4de31`, `9f64dd02`) | AC9.4 recurrence stays an open rule-10 item; its stable-runner half is on the ops row |
| AC10 cap-token guard | LANDED (`1fd4de31`) | the 5 µs Tier W measurement is not made (claim boundary) |
| AC11.1 shadow, AC11.2 binding, AC11.3 registry row | MOVED to `ops-stable-perf-runner` | the release-precondition perf leg of AC11.2 landed separately in `d00802aa` (20-5 L2) |
| AC12.1 retire the three advisory jobs | MOVED to `ops-stable-perf-runner` | retiring before binding would remove the only NFR-Perf-1/8 smoke |
| AC12.2/12.3 runbook, checklist, coverage rows | LANDED | `docs/runbooks/perf-runner.md`, `provisioning-checklist.md`, `tests/coverage-matrix.yaml` |
| AC13 ledger stops naming the key | LANDED by this closure | row table below; `check-dev-record-completeness` 0 violations |

## Tasks

| Task | Disposition |
|---|---|
| T0-T10 | LANDED (`e3231e88`, `1fd4de31`, `9f64dd02`, `850e4c00`, `a8af3215`, `ac1fc12a`) |
| T11 | LANDED (`1fd4de31`, `9f64dd02`, `850e4c00`, `ac1fc12a`, `5fe97eda`, `d00802aa`); main discipline run 43150757804 @ `d00802aa` green |
| T12 L2 shadow | MOVED to `ops-stable-perf-runner`, not ticked |
| T13 L2 bind and retire | MOVED to `ops-stable-perf-runner`, not ticked |

## `deferred-work.md` rows (111 rows touched, one pass)

| Class | Rows | Count | Treatment |
|---|---|---|---|
| CLOSED by landed work | cgroup read race (`:6`), S3 recurrences (`:1310`, `:1311`), cap-token 5 µs (`:1424`, `:1425`), stalled-runtime S3 decision (`:1478`), F11 perf fingerprint (`:1546`) | 7 | `CLOSED 2026-11-12 by 20-7-stable-runner-perf-gates @<sha>` plus witness; the cgroup fix is hosted-unverified (stated) |
| 1M erasure pending oracle, hosted timing noise | `:3-5`, `:30`, `:35`, `:47`, `:50`, `:55`, `:56`, `:61-64`, `:67`, `:68`, `:1555`, `:1557`, `:1591`, `:1596`, `:1609`, `:1613`, `:1620`, `:1623` | 23 | dated r1 disposition; owner cue `ops-stable-perf-runner`; `20-11` functional owner kept; no bound changed |
| Per-binary and suite sum advisories, hosted timing noise | `:8`, `:31-41`, `:57`, `:69-74`, `:80-83`, `:87`, `:90`, `:94`, `:1484-1487`, `:1558`, `:1594`, `:1597`, `:1615`, `:1622`, `:1624`, `:1629`, `:1677`, `:1689`, `:1700`, `:1711` | 40 | same; `xtask/test-budgets.toml` untouched |
| Default-concurrency, cause-unknown timing | `:9-11`, `:29`, `:48`, `:51-54`, `:78`, `:79`, `:88`, `:89`, `:111-117`, `:215`, `:1483` | 22 | same; no cause claimed |
| Snapshot-owner teardown timing | `:96`, `:1621`, `:1732` | 3 | same; K1w repairs the deterministic defect only |
| ac4/supervisor orphan bound | `:7` (new ac4_v3b row), `:1318`, `:1381`, `:1382`, `:1383`, `:1384` | 6 | disclosed disposition, not a proof; AC8.4 treats a live payload after root exit as a defect; cause unidentified (D-20-7-11 C) |
| Stable-runner-only | `:66`, `:1320`, `:1480` (soak), `:1482` (historical ac4 cause, if it recurs on the runner) | 4 | re-owned to `ops-stable-perf-runner` with the claim boundary |
| Split row | `:1195` (coverage and bench half CLOSED, soak and binding half re-owned) | 1 | both halves recorded on the row |
| Rule-10 `ownerless and open` with `Dispositioned by` | `:18` (W2 observer bound: no vetting/yank AC owns it; trigger: next change to `yank_policy.rs` observer or final-join code), `:95` (historical T3 roots; trigger: first story authorizing historical cleanup or finding a live producer), `:1479` (WASM recall; trigger: first guest benchmark harness), `:1592` (total_seconds re-tighten; trigger: operator rules on a budget change) | 4 | `:1592`: green run 43150757804 sum 1195.692 s over 6066 tests passes 1440; the row's rule yields 1495, a widening, NOT applied |
| Functional owner kept | `:1727` (`20-11` with `ops-stable-perf-runner`) | 1 | text re-pointed only |

Residual stale or ownerless-without-disposition owner rows for `20-7-stable-runner-perf-gates`: 0 (gate output: `check-dev-record-completeness` zero violations with 20-7 `done` in a scratch copy).

## Not verified

- Hosted discipline on the final tree after this closure; the cgroup-test fix on a hosted ARM runner; the F11 `perf.yml` change on a hosted runner.
- No stable-runner measurement of any kind; every perf verdict is ABSENT by design.
