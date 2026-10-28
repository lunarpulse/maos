# 19-1 creation measurements (2026-10-28, HEAD `f34f6b4e`)

One cargo process on the host; `CARGO_TARGET_DIR=/mnt/build/cargo`; every command prefixed `env -u CI -u GITHUB_ACTIONS`; gates run as `cargo run -q --locked -p xtask -- <gate>`. Tree clean before and after (`git status --short`: only a sibling's untracked `19-7-…-evidence/`). No workspace test run, so `intent-lineage-coverage-report.md` was not touched.

## Baseline at HEAD

| Command | Result |
|---|---|
| `check-kernel-baseline` | `PASSED (maos-kernel-core/src = 26378 lines, 95 files, pinned 26378)` |
| `kloc-check --json` | exit 0, `passed: true`, aggregate **188504**, `over_budget: []`; maos-kernel-core 20619, maos-bin 29497, xtask 47422, maos-bench 2359, maos-eval 4675, maos-control 1344, maos-cli 6155, maos-shell 613, maos-journey-test 946 (`kloc-check-runs.json` → `head_f34f6b4e`) |
| `xtask/kloc.toml` pins | kernel `:264` 20619 · xtask `:339` 47422 · maos-bin `:517` 29497 · maos-bench `:520` 2359 · maos-eval `:674` 4675 · `_aggregate_hardfail` `:718` 188505 |
| `xtask/kernel-core-baseline.toml:505` | `src_lines = 26378` |

## Prototype 1 — K2 T3 workspace mount (19-3's D19-2 A shape), for D-19-1-01

Diff: `k2-prototype.diff` (`T3Launch.workspace` → `T3SpawnContext.workspace` → `build_runtime_argv`: `--volume=<dir>:/workspace:rw,U` + `--workdir=/workspace`; maos-bin literal sites `worker_spawn.rs:1084`, `main.rs:7629` set `workspace: None`).

| Step | Result |
|---|---|
| `cargo check --locked -p maos-kernel-core --lib` | exit 0 |
| `check-kernel-baseline` | exit 1 (expected): `maos-kernel-core/src is 26389 lines … pins 26378. changed (3): lifecycle/cli_wrapper/runtime.rs, security/sandbox/t3/argv.rs, security/sandbox/t3/spawn.rs` → **+11 physical** |
| `kloc-check --json` | exit 1 (expected): kernel 20619 → **20628 (+9 tokei)**, maos-bin 29497 → 29499 (+2), aggregate 188504 → 188515 (+11) |
| Revert | `git checkout --` the five files; `check-kernel-baseline` PASSED 26378; `kloc-check --json` exit 0, aggregate 188504 (`after_revert`) |

The 20-15 conmon change in `t3/spawn.rs` (`git diff f889b3e4..HEAD`: `invoke_bounded` → `invoke_bounded_command`, `launch_t3` init cwd) does not touch K2's sites, so the tokei delta is the §R figure (+9); the physical +11 vs §R's +12 is one doc line of prototype shape.

## Prototype 2 — `--epic` in `parse_run_args` (AC1), for the maos-bin forecast

Diff: `epic-parse-prototype.diff` (`RunArgs.epic`, `RUN_USAGE`, the `--epic <file>` arm, the scene-combination refusal, `main.rs:10790` literal). `kloc-check --json`: maos-bin 29497 → **29506 (+9)**; `cargo fmt --check` reshapes the refusal by one line (≈ +10 formatted). Reverted; gates re-PASSED as above.

## CI per-test cost (for the test budget)

`ci-timings-40609213235.tsv` — the `workspace-test-suite-junit` artifact of landing run 40609213235 (`gh run download`). A one-Worker T3 founder-loop run costs 1.16–1.18 s (`smoke_cli_wrapper_8_12`), planted T3 refusals 1.11–1.21 s (`worker_completion_2a`); `eval_halt_18_2` Σ 1.37 s; suite Σ 564.1 s of the 800 s total (`xtask/test-budgets.toml:26-27`); `check-test-budget` passed (job 122564100523 log: `binaries 549, total_sum_seconds 564.1`).
