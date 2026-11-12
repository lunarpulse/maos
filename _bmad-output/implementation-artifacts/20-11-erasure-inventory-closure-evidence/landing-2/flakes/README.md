# Two hosted flakes on main — run 43143326211 @ e1a44cd5 (DevFlakes, 2026-11-12)

Hosted excerpts (ANSI stripped, from `gh api .../jobs/<id>/logs`): `hosted-run-43143326211-excerpts.txt`.
Both are documented families; neither is a product defect. Owner of the rule-10 rows: `20-7`.

## 1. `erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` — operator ruling: WIDEN

Hosted job 130935295525 (workspace-test-suite): `FAIL [44.126s]`, `EC-13 route rows=1000000 wall=10.000145984s response=HandlerStillRunning`, then `pending operation did not finish`
(`hosted-run-43143326211-excerpts.txt`, lines 4-6 of the workspace block). Hosted passes of the same oracle: 40.957 s (run 42533166219), 43.176 s, 43.2 s (two earlier hosted runs, figures
supplied by Main). The handler runs ~15-22 s against the unchanged `route_budget()` 10 s plus a 10 s second wait: the hosted case sat on that edge.

**Operator ruling 2026-11-12 (ask `ac5` = widen):** the SECOND wait becomes 30 s; the total allowance is 10 s route + 30 s = 40 s. The route budget and every product deadline are unchanged; every
assertion is kept (the oracle still proves completion, the receipt's `operation_id`/`outcome`, the tombstone, idempotent repeat, body integrity and the disk envelope).
Change: `crates/maos-bin/tests/erasure_million_20_11.rs:167-178` (`Instant::now() + Duration::from_secs(30)` instead of `forget().route_budget()` for the second wait only).

Room checks (read from the files, not assumed):
- `.config/nextest.toml:11` `slow-timeout = { period = "30s", terminate-after = 4 }` = 120 s per test; the binary is in the `serial` group (`:47`). Worst failing case = setup/seed S + 10 + 30; hosted S
  is 44.126 − 20 ≈ 24.1 s, so ≈ 64 s, and a passing case adds a ≈ 3.6 s tail (43.2 − 24.1 − ~15.5 s handler) ⇒ ≈ 68 s < 120 s. [INFERENCE from hosted figures]
- `xtask/test-budgets.toml:89-93` `[[exception]] binary = "maos-bin::erasure_million_20_11"`, `seconds = 78` (owner `20-11-erasure-inventory-closure`). The binary Σ at the new worst case is
  ≈ 68 s (1M case) + ≈ 5.6 s (the other five, hosted 100k 5.646 s per the exception's own reason) ≈ 73-74 s < 78 s. The cap is NOT below the new worst case; **not widened**. [INFERENCE: ~4 s headroom]

Local observation (ONE run, `million-widened-local.log`, `cargo nextest run -p maos-bin --test erasure_million_20_11 --locked --no-capture`, rc 0): all 6 tests PASS; the 1M case
`EC-13 route wall=10.000120479s response=HandlerStillRunning`, handler completed at door wall `21.833922319s` (max_insert 30.6 ms, ratio 0.0014), case `PASS [38.106s]` (flagged SLOW > 30 s,
not terminated), binary `Summary [43.448s] 6 tests run: 6 passed (1 slow)`. Before the ruling this exact local run failed at ~21 s against the 20 s allowance (20-11 AC5 note).

## 2. `cgroup_ceiling_smoke::killed_leader_cleanup_kills_its_surviving_descendant` — test race, product correct

Hosted job 130952590282 (`cgroup-ceiling-delegated (ubuntu-24.04-arm)`, runner `GitHub Actions 1000033443`): `panicked at crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs:786:23:
read descendant terminal state: No such process (os error 3)` (`hosted-run-43143326211-excerpts.txt`). Known family: the 20-13 story recorded an ARM descendant already dead `X`
(`20-13-pluggable-crypto-provider-selection.md:510-512`, run 42722028970).

Diagnosis. The test (before) did `read_to_string("/proc/<descendant>/stat")` after the leader was killed and treated only `ErrorKind::NotFound` (ENOENT) as "gone". `/proc/<pid>/stat` read on
a task that is released between `open` and `read` fails **ESRCH** (os error 3), and std maps ESRCH to `ErrorKind::Uncategorized`, not `NotFound`. A descendant that `cgroup.kill` killed and its
new parent reaped inside that window is therefore GONE = killed = the success condition; the test panicked on its own observation. The leader was already asserted `Signaled`, and a live
state (`R`/`S`/`D`) is still rejected, as is any other read error. Nothing in the product cleanup is implicated (no kernel/`src` change).

Fix (test-only, no sleeps, no retries): `cgroup_ceiling_smoke.rs:688-714` `descendant_stat_read_is_terminal(&io::Result<String>) -> Result<bool, &io::Error>` classifies a read:
state `Z`/`X`/`x` ⇒ terminal, `R`/`S`/`D` ⇒ alive (the test's assert fails), ENOENT or ESRCH ⇒ gone, any other error ⇒ `Err` (the test panics with it). The ignored hosted test calls it at `:852-856`.

Witnesses (all `cargo nextest run -p maos-kernel-core --test cgroup_ceiling_smoke --locked --no-fail-fast --no-capture`, `env -u CI -u GITHUB_ACTIONS`):

| # | tree | result | log |
|---|---|---|---|
| red-before | helper with the old NotFound-only classification (HEAD semantics) + the two new vectors | rc 100, 2 FAIL: `a_real_stat_read_racing_the_reap_fails_esrch_and_counts_as_gone` and `descendant_stat_read_classifies_gone_dead_live_and_unexplained`, both `Err(Os { code: 3, kind: Uncategorized, message: "No such process" })` | `esrch-red-before.log` |
| plant A | final tree, ESRCH arm disabled (`|| false`) | rc 100, same 2 FAIL | `plant-A-esrch-not-gone.log` |
| plant B | final tree, `S` accepted as a dead state | rc 100, 2 FAIL (`..._classifies_...`, `descendant_terminal_state_accepts_reaped_or_dead_but_not_live`) | `plant-B-live-state-accepted.log` |
| plant C | final tree, unexplained errors swallowed (`Err(_) => Ok(true)`) | rc 100, 1 FAIL (`..._classifies_...`) | `plant-C-unexplained-swallowed.log` |
| final | final tree | rc 0, 6 run / 6 passed / 4 skipped (the 4 are the `#[ignore]` delegated cases) | `esrch-final.log` |

Each plant restored by copy-back from `/tmp/ccs_final.rs` + `cmp` (identical) + `touch`. The real-kernel vector (`a_real_stat_read_racing_the_reap_...`) opens `/proc/<pid>/stat` of a live
`sleep`, kills and reaps it, then reads the open handle: it is deterministic and reproduces the hosted error exactly (`Os { code: 3, kind: Uncategorized, message: "No such process" }`).

Pins: tests are not counted in any kloc pin; `python3 scripts/repin.py --check` ⇒ `PINS IN SYNC` (kernel 27103/21256, aggregate 203243, hardfail 203244, xtask 45461). The suite lists are
unchanged: `ci_shape_18_0.rs:1269` / `discipline.yml:2144` / `cgroup_ceiling_workflow_17_2.rs:166` name the binary and the delegated run's `--expected-count 4` covers only the ignored cases;
the two added vectors are non-ignored (run in workspace-test-suite, like the existing fixture-validation tests).

Not verifiable locally: the real delegated-cgroup run of `killed_leader_cleanup_kills_its_surviving_descendant` (needs the hosted delegated cgroup job). Hosted is the judge.
