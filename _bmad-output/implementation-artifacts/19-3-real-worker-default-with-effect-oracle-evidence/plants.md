# 19-3 proven-red plants

## Plant set (prepared 2026-10-29)

Prepared against the working tree `75e1e493` plus the uncommitted correction round 2 (to be committed as-is). No plant was applied and no cargo command was run while preparing them. All three diffs pass a forward `git apply --check` against that tree and fail `git apply --check -R`. The new-file plant fails the reverse check because `crates/maos-bin/tests/xyz_19_3.rs` does not exist. The sha256 values below are those of the diff files in this directory.

Each plant is applied alone, in place, and restored byte-identically (`docs/testing/proof-contract.md:24-31`). Local plants use the 19-1 procedure (`19-1-…-evidence/plants.md`):

1. Copy the file aside and `git apply` the diff.
2. Run the target; it must go red.
3. Copy the file back, check it with `cmp`, `touch` it, and run the target again; it must go green.

A new-file plant is restored by deleting the file and checking that it is absent. Local runs use `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo` and one cargo process.

Plant count: AC2.4 ×1 (`proof-contract.md:28`, "one plant"), AC3.2 ×1, AC4.2 ×1. AC2.1 (kernel argv) has no plant: it is covered by pure-function vectors in `t3_argv_custody_17_1.rs`, and the AC2.4 plant covers the path (§Proven-red controls).

| Plant (diff · sha256) | Planted site (current tree) | Asserting test → panic site | Expected red | Tier | Command | Status |
|---|---|---|---|---|---|---|
| AC2.4 effect gate: `ac2-effect-gate-removed-plant.diff` · `efefe5069bc736791747861b1e85885c51bab107fd9871d067f81fd408ee2a67` | `crates/maos-bin/src/worker_spawn.rs:1309-1311`. The `worker_cli::gate_workspace_effect(…)` call after `parse_completion` is deleted, so the claude oracle's `Completed` stands over an empty diff (`workspace_effect` is still computed and reported) | `t3_runs::claude_claiming_a_write_over_an_untouched_workspace_fails_the_run` (`crates/maos-bin/tests/worker_workspace_19_3.rs:736`; vector (a), the RELEASE-HOLDS row-16 vector) → `:745` | `a claimed write over an empty diff must FAIL the run.`: vector (a) exits 0. Vector (c) `codex_file_change_without_a_workspace_diff_fails_the_run` (`:814`) would red the same way; the plant run selects (a) only | hosted (T3) | `cargo test --locked --color never -p maos-bin --test worker_workspace_19_3 -- --exact t3_runs::claude_claiming_a_write_over_an_untouched_workspace_fails_the_run --test-threads=1` | PENDING (hosted `ci/w2-verify`) |
| AC3.2 gate enrolment: `ac3-unenrolled-plant.diff` · `14fecf26b8a20ee689e3ed85cf42f7294fd25d2ef96f0af4381028ff0211204d` | NEW `crates/maos-bin/tests/xyz_19_3.rs` (`#[test] fn never_run_in_ci() {}`), not enrolled in the `check-j1-loopback-delegation` job step (`.github/workflows/discipline.yml:3431`). The gate derives its required set from `J1_TEST_SUFFIXES` (`xtask/src/check_j1_loopback_delegation.rs:133-135`, `_19_3.rs`) | gate leg `completion-vectors-enrolled` (`xtask/src/check_j1_loopback_delegation.rs:877`). The committed in-tree twin is `planting_an_unenrolled_19_3_test_file_must_red` (`xtask/tests/j1_crosshost_1b_proven_red.rs:676`) | exit 1, `"passed":false`, exactly one finding `{"check":"completion-vectors-enrolled", …}` whose detail names `` `cargo test -p maos-bin --test xyz_19_3` `` as not invoked by the job. The leg's check count rises by one, which shows the file was derived, not listed | local (T0) | Confirm `cargo run --locked -q -p xtask -- check-j1-loopback-delegation --json` exits 0 unplanted, then run the same command planted. Restore by deleting `xyz_19_3.rs`. The twin runs green with `cargo test --locked -p xtask --test j1_crosshost_1b_proven_red -- --test-threads=1` | RED observed 2026-10-29: gate exit 1, 0.29 s command wall-clock; `"passed":false`, exactly one `completion-vectors-enrolled` finding: ``.github/workflows/discipline.yml's `check-j1-loopback-delegation` JOB does not invoke `cargo test -p maos-bin --test xyz_19_3` `` (detail excerpt); leg checks rose 11 → 12. New file removed; `test ! -e` exit 0 and tracked-source diff stat empty (new-file restoration; `cmp`/`touch` not applicable). Restored gate GREEN: exit 0, 0.09 s. Twin target GREEN: exit 0, 24.81 s, all 36 tests passed, including `planting_an_unenrolled_19_3_test_file_must_red`. |
| AC4.2 no host execution: `ac4-host-probe-reinserted-plant.diff` · `a305fb805dd84d31712485f026914c0287926fb2c8a759b89a24edddd15a7fac` | `crates/maos-bin/src/worker_spawn.rs:841`, in the `ProbeStrategy::SpawnIsProbe` arm, after `check_egress`. The deleted host-side liveness probe is re-inserted with the pre-`c535e434` `run_liveness_probe` shape: `<resolved cli> --version` on the host, `env_clear` plus `PATH`/`HOME`/`XDG_RUNTIME_DIR`/`DBUS_SESSION_BUS_ADDRESS`, null stdio, and a non-zero exit refuses admission | `t3_runs::admission_never_executes_the_real_cli_on_the_host` (`worker_workspace_19_3.rs:833`) → `:857` | `the fake claude ran outside /workspace (marker …)`, followed by the marker's content `ran in <repo root>: --version`. The admission assert at `:852` passes first (one `cli_wrapper_loaded`) | hosted (T3) | `cargo test --locked --color never -p maos-bin --test worker_workspace_19_3 -- --exact t3_runs::admission_never_executes_the_real_cli_on_the_host --test-threads=1` | PENDING (hosted `ci/w2-verify`) |

### Local execution 2026-10-29

Executed in `/home/lunarpulse/dev_ws` at `f535db955da1b7919905f7ae62f38c5948beccce`. Per the execution assignment, both baseline and planted gate used exactly `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask /tmp/xtask-maos-9fea84c0 check-j1-loopback-delegation --json`, rather than rebuilding through the table's cargo wrapper. Unplanted baseline: exit 0, 0.23 s, `"passed":true`, 11 enrolment checks. The twin command used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo-maos nice -n 5 cargo test --locked -p xtask --test j1_crosshost_1b_proven_red -- --test-threads=1`. Timings include command compilation/execution where applicable and use a monotonic clock. The diff applied unchanged; `/tmp/plant-orig-19-3-ac3-unenrolled/` records the new-file plant's backup location (there was no original file); logs are `/tmp/plant-19-3-ac3-unenrolled-{red,green}.log` and `/tmp/plant-19-3-twin-green.log`. Hosted AC2.4 and AC4.2 plants were SKIPPED locally: no rootless Podman; hosted proof remains pending on `ci/w2-verify`.

## T0 `--volume …:U` confirmation (story T0)

The unplanted `worker_workspace_19_3` control in `ci/ci-w2-verify.yml` is the first hosted execution of the `rw,U` + `--user=65534` mount on the CI rootless image. It runs all six `t3_runs::` vectors, including `claude_that_writes_into_its_workspace_completes` (`h.txt` lands in the reported workspace) and `each_epic_worker_gets_its_own_workspace`. It must exit rc 0 with no `SKIP:` line.

Status: PENDING (hosted `ci/w2-verify`).

## Hosted plants (T3; `ci/w2-verify`)

The 19-3 rows run in the shared wave-2 workflow `19-2-fr20-enqueue-door-and-safe-point-evidence/ci/ci-w2-verify.yml` (branch `ci/w2-verify` only, never merged). The pattern and the per-plant checks are described in `19-2-…-evidence/plants.md` § Hosted plants.

The 19-3 control is `worker_workspace_19_3`, whose six named `t3_runs::` vectors must all run. After it come the AC2.4 and AC4.2 plants, each restored with `git checkout -- crates/maos-bin/src/worker_spawn.rs` and `git diff --exit-code`. The story's T6 named a separate `ci/19-3-verify` branch; that branch is folded into `ci/w2-verify`.

Run id: PENDING.
