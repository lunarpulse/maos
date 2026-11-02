# 20-7 landing 1 — local plants and failing-before witnesses (2026-11-02)

Method (`proof-contract.md:24-31`): copy the file aside, plant in the real tree, run the named vector, copy
the original back, `cmp` by sha256 against the pre-plant bytes, `touch`. Every plant below reports
`red = true` and `restored byte-identical = true` (the harness compared sha256 before and after). Commands
run with `env -u CI -u GITHUB_ACTIONS`, inherited `CARGO_TARGET_DIR`.

## Local plants

| Plant | File | Change | Vector that reds | Result |
|---|---|---|---|---|
| AC1.3 verdict | `xtask/src/check_perf_verdict.rs` | `representative` = best of five (`min`) instead of the median | `perf_gates_20_7::the_median_decides_a_budget_breach_not_the_best_of_five` (best 4000 < budget 5000 < median 6000) | RED, restored identical |
| AC2 ×4 | `.github/workflows/perf.yml` / `container.yml` | (a) fork guard removed from both host jobs; (b) `pull_request_target` added; (c) workflow `permissions: contents: write`; (d) a second workflow job `runs-on: [self-hosted, maos-perf]` | `xtask check-perf-runner-policy` on the REAL tree, rc = 1 each (clean tree rc = 0) | 4/4 RED, each restored identical |
| AC3.4 literal | `crates/maos-bench/src/decision.rs` | re-add `const J4_P95_BUDGET_US: u64 = 10_000;` | `perf_gates_20_7::the_budget_file_is_the_only_home_of_budget_literals` | RED, restored identical |
| AC7.6 K1w | `crates/maos-kernel-core/src/security/sandbox/t3/child.rs` | restore `if facts.is_none() \|\| self.lock().snapshot_inflight { return Err(..) }` | `t3_terminal_helpers_17_7::teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` | RED: 3 / 3 / 2 of 20 instances skipped `rm` in three runs; restored identical |
| AC9.5 drop | `crates/maos-bin/src/supervision.rs` | `unload` back to bare `self.handle.block_on(..)` | `worker_terminal_facts_17_7::supervisor_scenes::barrier_failure_unwinds_with_exactly_one_panic_20_7` | RED, restored identical |
| AC10 honesty | `crates/maos-kernel-core/tests/cap_token_verify_assertion.rs` | a 400 µs sleep in the audit enqueue of the "overall" test | `cap_token_verify_plus_audit_enqueue_p99_under_nfr_perf_3` reds while the verify-only catastrophe guard stays green (proves the overall test times the enqueue) | RED, restored identical |
| AC4.4 completeness | (in-vector, tempdir) | delete the NFR-Perf-4 bench | `perf_gates_20_7::deleting_the_nfr_perf_4_bench_reds_the_completeness_vector` | RED by construction (vector is its own plant; green on the real tree) |
| AC1 noisy / best-of-five, AC2 ×4 (synthetic trees), Tier C count over tolerance | tempdir | see the vectors | `perf_gates_20_7` (24 vectors) | green on the real tree |

## Failing-before at `a4c00897` (`git archive a4c00897` → `/tmp/head-tree`, `MAOS_20_7_TREE`)

| AC | Witness |
|---|---|
| AC1 / AC2 / AC5 | The subcommands do not exist at `a4c00897`: `git show a4c00897:xtask/src/main.rs \| grep -c check-perf` = 0; there is no `perf.yml`, no budget file, no Tier C harness. |
| AC3.3 | `the_budget_file_is_the_only_home_of_budget_literals` against the `a4c00897` tree reds on all eight sites: `j1.rs:25`, `j1.rs:170`, `j4.rs:36`, `decision.rs:14`, `decision.rs:15`, `j4_latency_regression.rs:25`, `iac_routing_budget.rs:37`, `cap_token_verify_assertion.rs:86`. |
| AC4.4 | `every_nfr_perf_and_hot_path_budget_has_a_metric_with_an_existing_instrument` against the `a4c00897` tree reds: `xtask/perf-budgets.toml: cannot read`. |
| AC7.2 | The K1w plant above IS the failing-before for the stub vector (the HEAD code before K1w is exactly the restored early return): 3/20, 3/20, 2/20 instances return `TerminationUnconfirmed` with no `rm`. |
| AC9.2 | The AC9 plant above (bare `block_on` ≡ HEAD `unload`) reds the single-panic vector (observed red; the stderr text was not captured in the harness log). The hosted 41013625894 excerpt is the original evidence of the second panic. |
| AC10 | n/a (shared-runner wall-clock; recorded hosted samples 5670 / 5480 ns). |

## Hosted-only legs

The operator executed `../ci/ci-20-7-verify.yml` on `ci/20-7-verify`, run
[`41372839255`](https://github.com/lunarpulse/maos/actions/runs/41372839255),
tree `7cae45bcfe23739cb7c8350d2d3b5e23b86aa65d` (branch-only workflow on
`9f64dd02`). Nothing in this section was executed on the dev host.

### Hosted diagnoses and workflow corrections

Line references below are the downloaded individual job logs (timestamps retained).
The corrections are edit-only; they are not claims of a successful replacement run.

| Leg / job | Evidence and cause | Correction |
|---|---|---|
| `real-spawn` / `125046786371` | Lines 2678–2679: native runner build finishes, then fixture builder requests `--target wasm32-wasip2`. Lines 2701–2712: E0463 for `core`, explicitly missing `wasm32-wasip2`, fixture exit 101. | Install `wasm32-wasip2` in this job's toolchain action. |
| `real-spawn-plant` / `125046786610` | Lines 2678–2679, 2702–2711: same missing WASI target, before the plant test ran. | Install that target; precompile the test and require the distinct-container assertion in the captured failure output, not arbitrary cargo failure. |
| `k2r-witness (base)` / `125046786577` | Lines 97–104: checkout treats abbreviated `a4c00897` as branch/tag patterns (`refs/heads/a4c00897*`, `refs/tags/a4c00897*`), all fetch attempts exit 1. This is ref classification, not evidence of a failing-before test. | Checkout the tested ref with full history, then resolve the base commit locally and detach; default is full SHA `a4c0089785bfc037ff211bb2cb946c0a3d51e666`. |
| `k2r-plant` / `125046786727` | Lines 544–570: patch/hash step succeeds (`k2r-plant.diff: OK`), followed by successful builds. Lines 3161–3165: `ac4_v4` PASS in 14.925 s, then `PLANT NOT DETECTED`. The shim at 530–533 intercepts only `kill`; `child.rs:464–470` can still issue `rm --force`, which can kill the payload and mask the removed pidfd signal. This masking route is a source-derived explanation, not a traced runtime invocation. | Both K2r legs still stall `kill`, but refuse `rm` while real `inspect` reports a running payload. Removing an already stopped container remains real. Require nextest test-failure exit 100 and the `/proc` stop-oracle failure, not a build/setup error. |
| `tier-c-plant` / `125046786529` | Lines 2563–2571: planted token path measures 77980.0 instructions, allocation row PASS at 1.0, but instruction row is ABSENT (no committed baseline). All three instruction rows remain ABSENT; expected `FAIL cap-token-verify-instructions` grep fails, exit 1 at 2572. The mutation ran; the expected binding baseline was missing. | Commit the clean hosted instruction baselines below, retaining 2% tolerance; replace the ad-hoc replacement script with the hashed, context-bound diff below. Instruction FAIL and allocation PASS remain required. |

Successful comparison: `k2r-witness (head)` / `125046786624` uses the same
stable rustc 1.99.1 (log lines 130, 216, 314) with no extra target, but only
builds the native binaries (lines 544–548); `ac4_v4` passes in 6.429 s at
lines 3081–3083. It never invokes the all-fixtures WASI build. The passed
`diag-ac4`, both `s3-both` legs, and `tier-c` are not replacement evidence
for the failed legs.

The fan-out stand-in previously emitted no `cli_wrapper_loaded` events and
therefore would encounter the loaded-population check before the intended
distinct-CID assertion (source inspection, not an observed hosted test result).
With explicit integration-owner authorization, the identical distinct-CID
assertion now runs immediately after successful-child/nonzero-work checks,
before population/accounting checks. All genuine-path population, rate,
pairing, dropped-task and event assertions remain unchanged and strict.
The workflow requires `AC6.2: every Worker is a distinct real container`
in the plant's failure output.

### Hosted Tier C instruction baselines (D-20-7-09 B, AC5)

Measurement job:
[`125046786606`](https://github.com/lunarpulse/maos/actions/runs/41372839255/job/125046786606).
Ubuntu 24.04.5, image `ubuntu-24.04` / `20261025.353.1`; debug harness built
with `kernel_measurement`; stable `rustc 1.99.1 (58cf8f9c3 2026-10-13)`,
host `x86_64-unknown-linux-gnu`, full compiler commit
`58cf8f9c35b355199727627d07ae34a4e2846d67` (log lines 314–320).
Valgrind **3.22.0**, Ubuntu package `1:3.22.0-0ubuntu3` (log lines 476,
503–507; artifact `empty.valgrind.log:3` confirms runtime version).
Command: `valgrind --tool=callgrind --toggle-collect=maos_tier_c_measured`.
Empty path: zero iterations, **36 Ir**.

The clean job prints 12944.0, 997449.7 and 188885.9 instructions at log lines
2534, 2537 and 2540. The committed values preserve the exact per-iteration
figures from the raw `totals:` lines, subtracting the empty path once:

| Metric | Iterations | Raw Ir total | `(Ir - 36) / iterations` committed baseline |
|---|---:|---:|---:|
| `cap-token-verify-instructions` | 10000 | 129440192 | 12944.0156 |
| `iac-routing-instructions` | 2000 | 1994899360 | 997449.662 |
| `frame-lowering-instructions` | 10000 | 1888859418 | 188885.9382 |

Raw receipts:
[`tier-c-counts`, artifact `13110004159`](https://github.com/lunarpulse/maos/actions/runs/41372839255/artifacts/13110004159),
ZIP SHA256 `050e06d612d23bd3edffbd702301fc4bf5369d9d4ffb93c68bb6cee1570f2027`.
`totals:` lines: `empty.callgrind:149`, `cap_token_verify.callgrind:3301`,
`iac_routing.callgrind:39505`, `frame_lowering.callgrind:6228`.
Harness JSON reports 10001/10000, 42128/2000 and 30000/10000 allocations.
The already committed allocation baselines (1.0, 21.24, 3.0) are unchanged;
all allocation rows PASS in the hosted measurement.

Only these three measured instruction rows bind at current phase `v1_5`.
Their existing opening regression tolerance **0.02** (2%) and zero noise
tolerance are unchanged; this is first hosted baseline establishment, not
shadow-derived tolerance calibration. Unmeasured wall-clock rows stay advisory.
The real-budget count fixtures now include nonzero instruction totals at these
baselines, require all six Tier C rows PASS/binding with no ABSENT, and exercise
an instruction-only regression with allocation PASS. The synthetic
missing-baseline test and advisory Tier W ABSENT expectations remain intentional.

### Reproducible plant patches

- K2r inline diff SHA256:
  `fb16d366e841fd6031b208b48437b15b79d2e14d82eba024fff1ac22a4c0aa8d`.
  Its seven-line hunk still matches current `child.rs:445–451`; only
  `let signalled = self.kill_payload();` becomes `let signalled = false;`.
  No recut or hash change was needed.
- Tier C diff `../ci/tier-c-plant.diff` SHA256:
  `f23de4c85b151d97593af49e6c07bbcfd1cbdaf4eac52fbea9b56753b7fb1ebe`.
  Its old-context hunk matches current `tier_c_harness.rs:154–161`; it adds
  the same allocation-free 1000-iteration black-boxed xorshift after verify.
  The workflow checks this exact hash before `git apply`.

Patch applicability above was checked by reading the current context, not by
applying a mutation in the shared checkout. No cargo, gates, tests or formatters
were run during these corrections.

Integration verification required: `cargo test --locked -p xtask --test
perf_gates_20_7`, `--test perf_budgets_20_7`, and `cargo run --locked -p xtask --
check-perf-counts --dir <downloaded tier-c-counts> --json` against the real budget
file (six binding PASS verdicts); rerun the corrected hosted legs, requiring
base/K2r plant stop-oracle reds, HEAD green, fan-out smoke green, distinct-CID
plant red, clean Tier C green and instruction-only plant red/allocation PASS.
The first run does not close these pending hosted proofs.

## Landing-1 review correction re-plants (2026-11-01)

The five existing local plants were re-executed together with nextest, serial,
no fail-fast, against their separate named witnesses (no plant relies on another
plant's red). The first drop mutation lacked the `Option` wrapper and failed to
compile (exit 101); corrected to `Some(handle.block_on(..))` before the witness
run. The compiling plant run exited **100: five tests, five failures**:

| Plant / vector | Observed result |
|---|---|
| Best-of-five / `the_median_decides_a_budget_breach_not_the_best_of_five` | FAIL: INDETERMINATE rather than expected FAIL (best-based spread 0.525 > 0.4); no false-green attribution |
| J4 literal / `the_budget_file_is_the_only_home_of_budget_literals` | FAIL: names `decision.rs:13` |
| K1w early return / `teardown_waits_for_a_stalled_snapshot_owner_then_removes_the_container_20_7` | FAIL: 10/20 instances skipped removal |
| Bare block_on / `barrier_failure_unwinds_with_exactly_one_panic_20_7` | FAIL: runtime nesting panic and destructor abort observed |
| Slow audit enqueue / `cap_token_verify_plus_audit_enqueue_p99_under_nfr_perf_3` | FAIL: P99 569767 ns > 100000 ns |

Every source restored byte-identically; full SHA256 receipts are in
`review-corrections/plant-restore.json`. No permanent kernel/bench/bin source
change is hidden in these plants.

**R3 instruction-only plant:** replaced the second allocating `verify` call
with a black-boxed 1000-iteration xorshift loop in the measured path. Real
10000-iteration harness observations before/under the plant both reported
**10000 allocations** (exit 0). Source restore SHA256:
`3d12396dca30774b2248db910151f4484aaa29dfbad77bf8a728a736a0727bda`,
byte-identical true. Instruction-red proof was **UNVERIFIED** locally. The first
hosted run measured the instruction increase but exposed absent baselines;
the hosted measurements/corrections above bind the clean baselines under the
operator's D-20-7-09 B ruling. A replacement hosted plant run is still required.

The expanded `perf_gates_20_7` negative fixtures exercise structural selector/
trigger/guard/secrets bypasses, malformed count/budget input, missing/duplicate
process observations, calibration identity, temporal rebaselines, duplicate SHA,
provenance and timestamp attacks. These are persistent in-vector refusal plants,
not mutations left in the working tree.

## AC9.4 whole-package loaded runs (review correction)

Executed **50 whole `maos-bin` package runs**, not a focused supervisor filter:
`env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --no-fail-fast`.
Each run exercised 765 tests across 98 binaries (15 skipped). All runs had 32
live `sha256sum /dev/zero` CPU-load processes on the 32-CPU host; all load
processes were terminated and reaped afterward (`loaded-package/load.json`).

**48/50 package runs passed.** Two unrelated named assertions remain open;
neither was retried to manufacture a green:

- Run 22, exit 100: `maos_uninstall_16_4::purge_refuses_live_store_without_changing_filesystem_identity`,
  `maos_uninstall_16_4.rs:1240`, refusal-created filesystem identity mismatch.
- Run 50, exit 100: `one_daemon_one_door_16_1::ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid`,
  `one_daemon_one_door_16_1.rs:982`, lifecycle/revocation ordering assertion.

Full per-run outputs and exit codes are in `review-corrections/loaded-package/`.
The original supervisor no-kill-record hang was not reproduced; that does not
prove its absence or close its existing open record. The separate complete
workspace serial and default runs passed 5517/5517 each.
