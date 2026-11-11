# AC5/EC-13 1M pending-forget oracle — local failure investigation (2026-11-11, Dev2011c)

Oracle: `crates/maos-bin/tests/erasure_million_20_11.rs::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption`
(panic `:186 "pending operation did not finish"`). Code under test at HEAD `459a606a` (no source edit made by this investigation).

## Verdict

**Not a regression introduced after 20-11 L1, and not caused by 20-14 round 3 (kinds 28/29 prefilter bypass).**
The handler's total wall time is ~21 s on this host's `/tmp` from the first review-correction commit `88c7277c` through HEAD (flat, ±1 s), while the
oracle allows `route_budget()` (10 s) + one further `route_budget()` (10 s) = **20 s** from submit. The oracle is a hard threshold sitting ~5-10 % below
this host's handler time and ~29 % below the hosted handler time (hosted first completion 15.54 s per the story record; local btrfs run 15.53 s). So it fails on every run here
and passes only when the host is faster than ~20 s (hosted nextest serial; one local whole-binary parallel run at 19.89 s).

## Method and one invalidated first attempt

* Scratch copies via `git archive <commit>` under `/tmp/ac5-inv/`, own `CARGO_TARGET_DIR=/tmp/ac5-inv/target`, never the repo target. All deleted afterwards.
* **First bisect attempt invalid, retained** under `invalid-stale-fingerprint-first-attempt/`: `git archive` stamps commit-time mtimes, so cargo reused another
  scratch tree's (instrumented HEAD) workspace-crate artifacts (the `a13140b8` log contains `SCAN` lines that tree did not have; `063c13cf`/`88c7277c`/`cbed4dad` also
  failed to compile against a mixed tree). Fix: `find $tree -type f -exec touch {} +` after extraction. All tables below are from the second, clean attempt.
* Two plain runs per commit (`run-<commit>-{1,2}.log`, the shipped oracle, `--nocapture --test-threads=1`) plus two diagnostic runs (`diag-run-tmp-<commit>-{1,2}.log`)
  that differ from the oracle only in two scratch-test edits: observation deadline 120 s instead of `route_budget()`, and a print of the completion instant. The diagnostic
  therefore measures the handler's real completion time. Commits before `063c13cf` keep the test in `shell_host_16_2.rs` (same body, no `ac5` file split).
* Profile: scratch copy of HEAD with a `Drop` guard in `transparency_log.rs::scan_rowid_range` printing per-scan windows / total / lock-wait / lock-hold
  (`head-instrumented-scan.log`; diff `scratch-scan-instrumentation-and-page16384.diff.txt`).

## Per-commit timing (host `/tmp` = overlay, fsync median 4.9 ms idle; `/mnt/build` btrfs 2.7 ms; load avg ~5)

| commit | what | route wall | plain oracle run 1 / run 2 (test wall) | handler completion (diag, s) |
|---|---|---|---|---|
| `cbed4dad` | 20-11 L1 candidate | 8.5 / 8.3 s (completes inside route in the diag runs; writer starved: 22-25 inserts, max insert 1.5-1.9 s; the one run that went pending had 537 inserts, max 2.16 s) | FAIL repeat returned `HandlerStillRunning` (49.96 s) / ok (33.78 s) | n/a, 8.3-8.5 |
| `88c7277c` | L1 review corrections (marker sweeps, windows, FairMutex hand-off) | 10.0 | FAIL 43.17 / FAIL 41.36 | 24.11 / 21.59 |
| `063c13cf` | L1 hosted follow-up (landing) | 10.0 | FAIL 36.24 / FAIL 36.25 | 20.65 / 21.03 |
| `a13140b8` | hosted run 42533166219 commit (hosted PASS) | 10.0 | FAIL 44.38 / FAIL 43.34 | 20.85 / 21.88 |
| `a44c76d9` | pre 20-14 review | 10.0 | FAIL 36.74 / FAIL 38.07 | 22.01 / 21.47 |
| `29f9a3ea` | 20-14 round 2 (before the round-3 SQL change) | 10.0 | FAIL 48.62 / FAIL 42.45 | 20.92 / 21.28 |
| `fc7373f0` | 20-14 round 3 (`kind IN (28,29) OR instr`) | 10.0 | FAIL 43.39 / FAIL 39.58 | 21.63 / 21.58 |
| `459a606a` | HEAD | 10.0 | FAIL (`head-459a606a-run1.log` 36.39 s; `run-459a606a-1/2.log` 41.33 / 44.22 s) | 20.75 / 21.59 |
| HEAD, `TMPDIR=/mnt/build` (btrfs) | fsync 2.7 ms | 10.0 | not run plain | **15.53** / 22.09 |
| HEAD, whole binary, default parallelism | all 6 tests concurrent (loom-leg shape, local) | 10.0 | PASS (36.21 s; 1M completion 19.885 s, 100k 2.63 s) | 19.89 |

Reading: `063c13cf` → `459a606a` is flat (20.65-22.01 s vs 20.75-21.59 s); `29f9a3ea` → `fc7373f0` (the 20-14 round-3 SQL change) is flat (20.92/21.28 vs 21.63/21.58, inside run noise of
~±1 s). The only step change is `cbed4dad` → `88c7277c` (8.3-8.5 s → 21.6-24.1 s), which is 20-11's own AC5.1 rewrite, whose purpose was the AC5.2 failing-before
(writer starved for 1.5-2.2 s in `cbed4dad`, 25 inserts; now 2.2k inserts, max insert 24-88 ms, wall ratio 0.001-0.004).

## Where the wall time goes (HEAD, instrumented, 4096-row pages; `head-instrumented-scan.log`)

The forget handler = door `forget_with_reason` → `scrub_forget_frames` (first sweep) + `HookPump::complete_forget` late sweep (`outstanding_redistill_requests` + a second
`sweep_erasure` loop from rowid 0 with a fresh `ErasureSweep`). That is **9 full-log scans × 245 four-thousand-row windows = 2205 lock turns** per 1M-row forget:
prior-tombstone `json_valid` scan, the principal `instr` scan, the kind-11 owned/closure scan, the malformed-kind-11 scan, the redistill-request scan, then the same four again.

* lock hold (real SQL work, debug test profile): 8.62 s total (per scan 0.5-1.9 s; the `instr` scan 1.9 s, `json_valid` scan 1.1 s, index-driven kind-11 scans 0.5 s).
* lock wait on the writer: 11.75 s total, 5.33 ms per window ≈ one fsynced writer insert on this host's `/tmp` (idle fsync 4.9 ms). `FairMutex` hands every released lock
  to the queued writer, so scan and writer strictly alternate: 2213 writer inserts in 20.75 s, i.e. 2205 turns.
* The writer's fsync and the window SQL are therefore the whole cost. The JSON decode added by 20-14 round 3 is never reached: kinds 28/29 are absent from this fixture and all
  1M filler rows are kind 1.

Completion ≈ windows × (window work + writer turn), i.e. 2205 × ~9.4 ms = 20.7 s here, 2205 × ~7 ms = 15.5 s on hosted/btrfs. That is the AC5.1 design cost, not a defect.

## Scratch experiment (not applied; page size is a ruled AC5 parameter)

`FORGET_SCAN_PAGE` 4096 → 16384 in the scratch tree only: 9 scans × 62 windows = 558 turns; lock wait 11.75 s → 3.24 s; handler completion 11.4-11.7 s; max writer insert
43.5 ms; wall ratio 0.0037-0.0038; oracle PASS twice (`page16384-instrumented-run-{1,2}.log`). The AC5 work-bound tests/plants calibrate against 4096 (`erasure_scan_work.rs:119`
`4096 * 128` VM-step cap; story AC5.5/`i64::MAX` plant calibration), so applying it needs a ruling and re-calibration of those controls; it is a one-constant production change.

## Hosted loom leg (c)

No hosted loom log for this test exists in the repo, so the loom failure is **inferred, not observed**: `check-multi-tenant-loom` runs `cargo test -p maos-bin` whole-package with default
parallelism, so the 1M vector competes with the 100k and replay vectors and with whatever else the runner is doing for the same 20 s. Locally the same binary at default parallelism passed at 19.885 s
(a coin-flip on the 20 s threshold, `head-459a606a-whole-binary-default-parallel.log`).

Exact proposal (**not applied**): in `xtask/src/check_multi_tenant_loom.rs` (~:1859-1863) change the leg to skip only that one by name — the leg exists to catch the two-tests-one-process `set_var`
race, which a parallelism-preserving `--skip` keeps; the heavy vector keeps its own hosted coverage in the nextest serial group (hosted pass 40.957 s, run 42533166219):

```rust
const HEAVY_1M_FORGET: &str = "ac5_million_row_pending_forget_finishes_and_repeats_without_corruption";
...
TestLeg {
    name: "maos-bin-whole-package-parallel-isolation",
    class: BindingClass::Blocking,
    args: &["test", "-p", "maos-bin", "--", "--skip", HEAVY_1M_FORGET],
},
```

`run_exact_test_leg` already takes `suite_tests = Some(..)` for this leg, so the non-vacuity check (`PARALLEL_ISOLATION_TESTS` must pass) is unaffected; the "NO test-name filter" comment (`:1855`) must be
amended to say the one skip does not change parallelism. Cost: a few xtask lines [INFERENCE: const + multi-line array, +4..+9] against the xtask pin; `.github/workflows/discipline.yml` needs no change
(it only invokes the xtask). Caveat: `--skip` is a substring filter across all binaries; the name is unique in the workspace.

## Recommendation

1. Do **not** treat this as a code regression; no code was changed, no budget/deadline widened, no retry.
2. AC5/EC-13: the 1M completion oracle is hosted-serial evidence only (already the story's AC5 shared-host disposition; the hosted pass exists). It **cannot** be claimed from this host. Whether AC5 can close is a
   review ruling on the already-recorded hosted pass, not something local runs can demonstrate.
3. If the operator wants the oracle to be local-reproducible/robust (a 15.5 s hosted vs 20 s hard threshold is a thin margin), the principled lever is fewer lock turns, in priority order:
   (i) page 4096 → 16384 (one constant; halves+ the time, recalibrate the quarter-scan/VM-step controls — needs a ruling because AC5 pins the window size);
   (ii) seed the late sweep from the first sweep's high-water/held/malformed state instead of re-scanning from rowid 0 (removes 4-5 of 9 scans; needs a carrier from `scrub_forget_frames`
   — kernel-core `ForgetOutcome` is kernel-frozen, an adapter-side hand-off costs maos-iac lines at ceiling 8770 / zero headroom: STOP, priced need required [INFERENCE ~+25..+40]).
4. Independent of 3, apply the loom-leg `--skip` above if the hosted loom leg is red on this test.
