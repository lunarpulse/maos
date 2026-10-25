# 18-0 plants (D-18-0-13 A) — one per control AC, in place, local, recorded once

Dev HEAD `c8613658` + the uncommitted 18-0 tree, 2026-10-24, host kernel 7.2.9-1-cachyos, `CARGO_TARGET_DIR=/mnt/build/cargo`,
`env -u CI -u GITHUB_ACTIONS`. Every planted file had uncommitted 18-0 edits, so each was restored from a pre-plant copy
(`cp <copy> <file>`), not `git checkout`.

## AC5 — budget rows (`xtask/test-budgets.toml`)

- Plant: `maos-bin::worker_terminal_facts_17_7` exception `seconds = 153` → `100` (pre-plant copy `/tmp/cdeny/test-budgets.preplant.toml`).
- Command: `/mnt/build/cargo/debug/xtask check-test-budget --junit /tmp/cdeny/junit-40080259088/junit.xml --json`
  (run 40080259088's `workspace-test-suite-junit` artifact).
- Output (exit 1):

```
{"gate":"check-test-budget","passed":false,"junit":"/tmp/cdeny/junit-40080259088/junit.xml","binaries":504,"total_span_seconds":335.40701031684875,"total_sum_seconds":579.8809999999992,"findings":["maos-bin::worker_terminal_facts_17_7 took 117.273 s (Σ per-test time) over 31 test(s), budget 100.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml"]}
::error::check-test-budget: FAIL — 1 finding(s):
- maos-bin::worker_terminal_facts_17_7 took 117.273 s (Σ per-test time) over 31 test(s), budget 100.000 s — make it cheaper, or add an [[exception]] with reason + owner to xtask/test-budgets.toml
```

- Exactly one finding, exit 1. Restored (153); the same command → `passed: true`, `findings: []`, exit 0.
- Wall-clock (2026-10-24 run): the planted run's own timing was lost (the shell aborted after the red exit under a stale
  `set -e`); the 0.015 s figure is the identical command on the **restored** file (prebuilt xtask, no compile).
- **Re-proved 2026-10-25 (review patch, this plant only; the tree now also carries the review patches, `xtask/src`
  unchanged so the prebuilt `/mnt/build/cargo/debug/xtask` is current):** same plant (153 → 100, pre-plant copy
  `/tmp/cdeny/test-budgets.preplant-review.toml`), same command → exit 1, the same single finding (117.273 s over 31
  tests, budget 100.000 s), **planted-run wall 0.032 s**; restored by copy-back, `cmp` of the restored
  `xtask/test-budgets.toml` against the pre-plant copy → identical (exit 0); restored run exit 0, `findings: []`, wall
  0.018 s. Copy-back rather than `git checkout`: the file carries uncommitted 18-0 rows that a checkout would discard.

## AC6 — `expensive-tests` shape / proven-red `if:` (`.github/workflows/discipline.yml`)

- Plant: removed `if: ${{ !cancelled() }}` from `check-test-budget`'s step "Run the 20-12 AC1 proven-red vectors"
  (pre-plant copy `/tmp/cdeny/discipline.preplant.yml`).
- Command: `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test ci_shape_18_0`
- Output (exit 100), wall 0.887 s (incremental, test binary already built):

```
        PASS [   0.024s] (1/3) xtask::ci_shape_18_0 cargo_deny_is_its_own_blocking_job_in_aggregate
        PASS [   0.026s] (2/3) xtask::ci_shape_18_0 expensive_tests_uploads_its_junit_and_is_enrolled_in_aggregate_only
        FAIL [   0.041s] (3/3) xtask::ci_shape_18_0 proven_red_steps_run_after_a_red_gate_step
    thread 'proven_red_steps_run_after_a_red_gate_step' panicked at xtask/tests/ci_shape_18_0.rs:81:9:
    assertion `left == right` failed: 'cargo test -p xtask --test check_test_budget_proven_red' must run after a red gate step (18-0 AC6.3), not be skipped by it
      left: None
     right: Some("${{ !cancelled() }}")
     Summary [   0.042s] 3 tests run: 2 passed, 1 failed, 0 skipped
```

- Restored (`cmp` identical to the pre-plant copy); the same command → 3 passed (see Dev Agent Record).

## AC7 — rupture id unique across restarts (`crates/maos-a2a-core/src/router.rs`)

- Plant: the mint restored to the pre-18-0 `frame_id[0..8] ‖ alloc_id()` block (pre-plant copy `/tmp/cdeny/router.preplant.rs`).
- Command: `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-cohort --test rupture_restart_18_0`
- Output (exit 100), wall 3.756 s (incl. the incremental rebuild of maos-a2a-core and dependants):

```
        FAIL [   0.200s] (1/1) maos-cohort::rupture_restart_18_0 the_same_denied_frame_after_a_restart_journals_a_second_rupture
    thread 'the_same_denied_frame_after_a_restart_journals_a_second_rupture' panicked at crates/maos-cohort/tests/rupture_restart_18_0.rs:152:43:
    assertion `left == right` failed: boot 2: the denial must be journaled and answered as a policy deny, never CODE_INTERNAL: consent denied but rupture persistence failed: rupture row f0f0f0f0f0f0f0f00100000000000000 was already present — this rupture event was NOT journaled (id collision across restarts?); audit before trusting the TL's rupture set
      left: -32099
     right: -32001
     Summary [   0.201s] 1 test run: 0 passed, 1 failed, 0 skipped
```

- The second denial reds with `CODE_INTERNAL` (-32099) on the colliding id `f0…f0 ‖ 01 00…` (counter restarted at 1); the
  boot-1 row is the only rupture row. Restored (`cmp` identical to the pre-plant copy); the same command → 1 passed.

## AC8 — `maos init` creates the CRL directory (`crates/maos-shell/src/lib.rs`)

- Plant: removed `std::fs::create_dir_all(maos_domain::revocation::default_crl_dir())?;` from `run_init` (pre-plant copy
  `/tmp/cdeny/shell-lib.preplant.rs`).
- Command: `env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test shell_8_14a`
- Output (exit 100), wall 8.719 s (incl. relinking `maos`):

```
        PASS [   0.038s] (1/6) maos-bin::shell_8_14a init_creates_dot_maos_and_is_idempotent
        FAIL [   0.043s] (2/6) maos-bin::shell_8_14a init_creates_the_crl_directory_and_honours_maos_crl_path
    thread 'init_creates_the_crl_directory_and_honours_maos_crl_path' panicked at crates/maos-bin/tests/shell_8_14a.rs:84:9:
    first init: /tmp/maos-8-14a-crl-2332193-1792883298075633271/xdg-data/maos/crl is a directory
     Summary [   0.526s] 6 tests run: 5 passed, 1 failed, 0 skipped
```

- Restored (`cmp` identical to the pre-plant copy); the same command → 6 passed.

## Count

Four plants (AC5, AC6, AC7, AC8), one each; AC2/AC3 are existing gates (HEAD reds recorded in `dev/t0-cargo-deny-head.txt`,
`creation/governance-1fec4882.txt`); AC4 changes tests, not controls.
