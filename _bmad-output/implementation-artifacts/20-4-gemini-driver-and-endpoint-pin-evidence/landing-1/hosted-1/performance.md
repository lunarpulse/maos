# Hosted-1 model-currency performance evidence

Hosted diagnosis: discipline run 41815429165, check-test-budget job126540412403 measured `xtask::model_currency_gate` at **30.601 s / 25 tests**, above the existing **30.000 s** per-binary sum budget. This supplied CI observation is ground truth; no hosted rerun is claimed.

## Controlled local audit profile

Five independent `xtask check-model-currency --json` child processes were measured before and after with Python `resource.getrusage(RUSAGE_CHILDREN)` deltas and `perf_counter`. Every sample exited 0. `/usr/bin/time -v` was unavailable (exit127), so no output from it is treated as a profile.

| Mean per audit | Before full-file AST | Final root-only scan |
|---|---:|---:|
| User CPU seconds | 0.5305912 | 0.4310398 |
| Wall seconds | 0.5437248 | 0.4401365 |

Controlled user CPU fell **18.762%**. This supports a real cost from full-file AST work; it does **not** establish that AST construction was the majority of total audit cost, nor predict an observed hosted result. The implementation lexes once, discards root fn/impl/mod brace groups before syn can see their contents, and checks constant prefixes before constructing only `ItemConst` ASTs. It has no cross-audit cache or invalidation shortcut.

## Fresh serial JUnit, unchanged policy

| Run | Cases | Sum of testcase times | Existing budget |
|---|---:|---:|---:|
| Before | 25 | 13.640 s | 30.000 s |
| Final | 26 | 15.879 s | 30.000 s |

Both are fresh serial profile-ci runs. The final case-name set contains **every original case**, plus the new opaque-body scope control. Final sum is below the unchanged budget. Local JUnit wall sums did not improve; they are reported as measured, not relabelled as a speedup. Controlled CPU profiling is the improvement evidence. No budget row, exception or vector was removed/relaxed.

`currency-before-junit.xml`, `currency-after-junit.xml`, both timing JSON files, CPU sample JSON files and `performance-comparison.json` retain the raw measurements. Per-test timing is the assignment's supported alternative to applying the whole-workspace budget gate to a partial JUnit (which would also diagnose intentionally absent exception binaries).

## Rejected and intermediate observations

An early asynchronous-build draft copied the still-old 25-test JUnit and profiled the old executable; `rejected-measurement-draft.json` explicitly disqualifies it. It is not final-after evidence. Token-only chunking without an early constant-prefix check was also measured and retained in `intermediate-token-only-measurements.json`; it is superseded. The final files named `after` contain only the completed 26-test run and rebuilt final audit executable.
