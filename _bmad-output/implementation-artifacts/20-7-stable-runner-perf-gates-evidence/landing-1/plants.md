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

## Hosted-only legs (never run here)

The hosted-only witnesses remain in `../ci/ci-20-7-verify.yml`: `k2r-witness`
(podman kill shim), `k2r-plant` (diff sha256 `fb16d366…aa8d`), the allocation-neutral
instruction plant, `real-spawn-plant`, `s3-both`, `diag-ac4`, `tier-c`. None has
been executed here; the operator pushes the branch.

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
byte-identical true. Instruction-red proof is **UNVERIFIED**, not replaced by
an allocation-red: the hosted plant now demands instruction FAIL and allocation
PASS, and requires the R3 hosted baselines/operator decision.

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
