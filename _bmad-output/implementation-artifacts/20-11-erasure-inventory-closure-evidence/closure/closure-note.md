# 20-11 closure note (record-only, 2026-11-12, `Close2011`)

No code, kernel file, invariant doc or `main.rs` was edited by this closing record. It states what the tree and the hosted run show.

## Hosted evidence

Discipline run `43175946640` on `637721bf`, conclusion `success` (199 jobs; only the advisory `reproducible-build` failed, runner disk exhaustion). Job `workspace-test-suite` id `131040117355` (`gh api repos/lunarpulse/maos/actions/jobs/131040117355/logs`), read at closure:

- `PASS [43.597s] maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (a `SLOW > 30.000s` notice precedes it)
- `PASS [4.664s] maos-bin::erasure_million_20_11 ac5_door_bounded_scan_admits_continuous_writer`
- `PASS [1.521s] maos-bin::erasure_live_20_11 ac2_live_door_erases_principal_free_lineage`
- `PASS [2.461s] maos-bin::erasure_live_20_11 ac6_running_t3_worker_uninstall_proves_live_inventory`
- `Summary [272.041s] 6095 tests run: 6095 passed (3 slow), 137 skipped`

`MAOS_T3_RUNTIME: podman` is set on the job. The Worker variant is not an early-return skip on a declared-CI host: `tests/harness/t3_substrate.rs:9-27` (`require_t3`) routes the missing-substrate case through `ci::skip_unless`, which returns `false` on a declared-CI host (`tests/harness/ci.rs`), and then asserts the runtime and the signed image lock with `panic!` on failure.

## What the 40 s allowance measures

In `erasure_million_20_11.rs` the clock `start` is taken immediately before `submit_and_wait(forget)` and the second wait is `Instant::now() + 30 s` after the route returns `HandlerStillRunning` (route budget 10 s). So the allowance (10 s + 30 s) covers forget submission to the observed completion receipt. The 43.597 s in the PASS line is the whole test wall including the 1M-row/1 KiB fixture build; it is NOT the allowance figure and must not be compared to 40 s. The log excerpt read here does not print the `EC-13 door ... wall=` line, so the actual forget-to-receipt time is not independently read; it is bounded by the test's own `pending operation did not finish` assertion (30 s) and the PASS.

## AC table

| Item | Status | Evidence |
|---|---|---|
| AC1 proof v2, `NotFound` rule, both verifiers | LANDED | `cbed4dad`, `88c7277c`; `erasure_inventory_20_11`, `erasure_review_20_11` (hosted PASS in 43175946640) |
| AC2 intent lineage, stable-owner isolation | LANDED | `88c7277c`; hosted `ac2_live_door_erases_principal_free_lineage` PASS |
| AC3 pending halts, O4, offline orphan closure | LANDED | `689a6a53`, `5ff0ead1`, `c2beedb4`; `landing-2/plants.md` |
| AC4 schedules, package, KS `forget_spirit` | LANDED | same; kernel inside the 26,893/21,078 ceiling per frontmatter |
| AC5.1/.3/.4/.5 windows, serialization, off-worker, plant | LANDED | `063c13cf`, `88c7277c` |
| AC5.2 100k writer-latency bound | LANDED, hosted | `42533166219` (4.698 s); `ac5_door_bounded_scan_admits_continuous_writer` PASS 4.664 s in 43175946640 |
| AC5 "real 1M completion at unchanged deadlines" | AMENDED (`ac5`) | operator ruling widen, `5fe97eda`; hosted PASS above; amendment text in the story AC5 |
| AC6 hosted live T3 leg | LANDED under AC6.3 fallback | Worker SCB has no schedules; Worker PASS [2.461s] on 43175946640/131040117355; researcher variant asserts `Removed{1}` |
| AC7 named stores, typed list, plant | LANDED | T6 |
| AC8.1 coverage FR65 / NFR-Aud-12 | LANDED | `tests/coverage-matrix.yaml` notes updated 2026-11-12 |
| AC8.2 EC-13 / owner rows | LANDED | `deferred-work.md` EC-13 `CLOSED 2026-11-12 by 20-11 @637721bf`; owner rows re-worded |
| AC8.3 ADR-044 addendum | LANDED | `docs/adr/ADR-044-gdpr-article-17-distillate-redaction.md` addendum (not re-read at closure; per the prior truth-table analysis) |
| AC8.4 pins, HISTORY, FLAG-Winston | LANDED | `20-11-flag-winston-decision.md`; kernel-baseline HISTORY rows |
| Private/index hold lease (`kernel_private_index_hold_lease`) | CLAIM-BOUNDARY (named gap, rule 10(b)) | no AC requires it; `decisions-2026-11-12-acceleration.md:61,65` "unblocked for pricing, none granted"; `main.rs:11667` string still says "pending final kernel landing" (stale wording, not edited) |

## Caveats

- Hosted proof of AC5/AC6 is a single run on one tree (`637721bf`); earlier hosted 1M runs passed at 40.957/43.176/43.2 s and failed once (43143326211, 44.126 s) before the widening.
- The final-tree review accounting is the L2 review rounds (`landing-2/review/`), not a fresh review of this closing record.
- Apple and Windows paths were not run for the live legs.
- The 20-7 stable-runner timing/I/O half stays with `ops-stable-perf-runner`.
- `xtask/test-budgets.toml` still names `20-11-erasure-inventory-closure` as owner of the 78 s exception; only a non-empty owner is required by `check_test_budget.rs`.
