# Landing 3 — P1 (cold-swap restore races the old session's teardown) + F2 (shipped TS example cap)

Uncommitted tree on `main` (HEAD faafe427). Logs sit beside this file; every command ran with `env -u CI -u GITHUB_ACTIONS`.

## P1 — root cause (confirmed by reading the code, then reproduced)

Hosted: `spawned_door_20_9::maosctl_cold_swaps_a_component_to_a_new_pid_and_restores_it_when_the_successor_is_refused` failed on ubuntu-26.04-arm (run 43179221668, job 131050832263) and ubuntu-24.04-arm (run 43185836600, job 131072635915): `restore refused: … session mailbox failed: spirit lifecycle-probe is already registered`.

* The door's cold swap (`operator_door.rs`, `cold_swap`: `scheduler.unload(pid)` → `stop_worker_after_unload(pid)` → `admit_and_start`) only *signals* the old session: `stop_binding` → `BindingCore::stop_session` (kill via pidfd). The session is a blocking task (`spirit_session.rs` `run_admitted_session`, `spawn_blocking`) that notices the dead child on its next 20 ms poll, then runs its teardown (grace/kill, `revoke`, `wait_and_finalize`, writer join, `binding.finish`) and only on return drops `Recipient`, whose `Drop` is what calls `mailbox.unregister_spirit` (`spirit_session.rs:258-262`), and `IdentityOwner`.
* Nothing awaited that. A refused successor (artifact gone) fails in `manifest_launch_in` before any session starts, so the restore's `admit_spawned` → `run_admitted_session` → `register_spirit_typed` (`spirit_session.rs:405`) runs at once, and on a slow/loaded core it loses the race to the old session's `Recipient::drop`. (A *successful* successor could lose it the same way.)
* Not an arm-specific bug: it is a scheduling race. The door e2e vector reproduces it locally when the vector and its children are pinned to one CPU with busy-loop jobs (below).

Other callers checked (grep of `register_spirit_typed`/`stop_session`/`run_admitted_session`):
* hot swap / `spirit upgrade --policy hot-swap`: the session and its mailbox registration survive; `retire` only releases the predecessor *binding*. No re-registration, no race.
* `InstalledSpirits::boot` and `maos run`: first session of an id, no predecessor.
* door `unload` verb on a component: a following `load` of a component stays refused (`SpawnedSurfaceUnsupported`); the only path that re-admits a component id is the cold swap, i.e. `SwapHandles::admit_spawned`. That is the one choke point fixed.

## P1 — fix (no kernel change)

* `crates/maos-bin/src/spawned_lifecycle.rs`: `SessionEnds` (latest session of each Spirit id → a `watch::Receiver<()>`), `SessionEnd = watch::Sender<()>` and `SESSION_END_WAIT = 15 s`. `SwapHandles::admit_spawned` awaits `supervisor.session_ends().ended(id)` under `tokio::time::timeout(SESSION_END_WAIT, …)` before preparing the successor; on timeout it refuses typed `session_not_ended: …` instead of racing. No sleep/poll: the wait resolves when the session drops its sender (closed channel).
* `crates/maos-bin/src/spirit_session.rs`: `run_admitted_session` takes `supervisor.session_ends().begin(id)` and the closure binds it first (`let _ended = ended;`), so it is the *last* local dropped: after `Recipient` (mailbox unregistered) and `IdentityOwner`.
* `crates/maos-bin/src/supervision.rs`: `WorkerSupervisor::session_ends` field + accessor; test seam `SupervisorSeams::before_session_release: Option<Arc<dyn Fn(u32)>>` (the existing `#[doc(hidden)]` seam struct; no cfg(test), no env var) called once from the session teardown (`spirit_session.rs`, after `binding.finish`) just before the mailbox is released.
* `crates/maos-bin/tests/support/spawned_world_20_9.rs`: `World::with_seams(seams)`; `build` now arms with `arm_with_seams`.

## P1 — witnesses

Deterministic, in-process, real `maos-wasm-runner` children (`tests/spawned_swap_20_9.rs`):
* `a_cold_swap_admission_waits_for_the_previous_session_to_release_its_mailbox`: the seam parks the unloaded+stopped predecessor's teardown while it still holds its mailbox registration; the restore admission must stay pending for 750 ms (it is decided → panic), then complete once released, at a new pid, with the old session task joined.
* `a_cold_swap_admission_is_refused_typed_when_the_previous_session_never_ends`: bounded wait = typed `session_not_ended` after ≥ `SESSION_END_WAIT`; admission goes through once the session ends.

| Step | Result | Log |
|---|---|---|
| failing-before (seam + witness, no fix) | witness FAILS: `the restore was decided while the predecessor still held its mailbox: Err(ClassBuild { detail: "spawned_start_failed: session mailbox failed: spirit lifecycle-probe is already registered — deregister before re-registering" })` — the hosted error text verbatim | `p1-failing-before-seam-only.log` |
| fix, nextest, spawned_swap_20_9 + spawned_door_20_9 + spawned_lifecycle_20_9 + spawned_installed_20_9 | **48 tests run: 48 passed (3 slow)**, includes both witnesses and the door cold-swap vector | `p1-green-nextest-spawned.log` |
| plant (the `admit_spawned` wait cut out; restored with `cp` + `cmp` + `touch` from `/tmp/p1_fixed_spawned_lifecycle.rs`) | both witnesses FAIL (same text as above; typed-refusal witness fails at 0.8 s); the door vector passes (race not hit on this box without load) | `p1-plant-no-wait-nextest.log` |
| after restore | 3/3 passed (both witnesses + door vector) | `p1-green-after-restore-nextest.log` |

Note: plain `cargo test --test spawned_swap_20_9` (one process, 32 threads) shows two unrelated failures (`a_cross_major_component…archive_current`, `a_prewarm_guest…stderr-flood`): they share the process-global FR11 archive and a `/proc` scan by hostile-twin mode, and pass alone (`--exact`) and under nextest (process per test, which is how CI runs). The same two fail in the single-process run with the two new tests skipped (`p1-single-process-without-new-tests.log`: 21 passed, 2 failed), so they are not caused by this change.

### The existing door vector under CPU stress (local, 20 iterations each, load jobs started and killed by PID)

`scripts` : `p1_stress_loop.sh` (40 busy-loop jobs on 32 CPUs) and `p1_stress_loop_1cpu.sh` (vector + children `taskset -c 0` with 8 busy-loop jobs on that CPU — a starved core like a loaded arm runner). Test exe = `spawned_door_20_9-554fe600a7f943e9`, `--exact` the cold-swap vector.

| Profile | Code | Pass / total | Notes | Log |
|---|---|---|---|---|
| 40 load jobs, 32 CPUs | before (plant) | 20 / 20 | race not hit | `p1-stress-before-plant.log` |
| 40 load jobs, 32 CPUs | after (fix) | 20 / 20 | | `p1-stress-after-fix.log` |
| 1 CPU + 8 load jobs, run 1 | before (plant) | 11 / 20 | 3 × the hosted failure (`spawned_door_20_9.rs:303`, `restore refused … already registered`), 6 × maosctl route-budget timeout (`:285`, starved core, later iterations under other agents' load) | `p1-stress-1cpu-before-plant-run1-15iters.log` (20 iterations ran; the filename's "15" predates the full run) |
| 1 CPU + 8 load jobs, run 2 | before (plant) | 8 / 12 | 3 × `already registered` (`:303`, outputs kept in `…run2.log.fail-{5,10,12}.txt`), 1 × route budget | `p1-stress-1cpu-before-plant-run2.log` |
| 1 CPU + 8 load jobs | after (fix) | 7 / 16 (capped at 16 by the orchestrator) | **0 × `already registered`**; all 9 failures are the first cold swap outliving maosctl's 30 s route budget (`:285`, "running to completion") on the starved core | `p1-stress-1cpu-after-fix.log` + `.fail-N.txt` |

Honest reading: the race signature (`already registered`) is 6 of 32 before and 0 of 16 after. The single-CPU profile also trips an unrelated, load-induced client route-budget timeout (it also occurs before the fix, 7 of 32, and 9 of 16 after); after the fix the first swap additionally waits for the old session's teardown, which on a 1/9-share core adds to that timeout, so the raw pass count under that profile is NOT better after the fix and must not be read as such. It is a starvation artifact of the profile, not of hosted runners (the 32-CPU profile and hosted runs are unaffected).

## F2 — shipped TS example cap

`examples/example-spirit-ts/manifest.toml:43` and `templates/spirit-ts/manifest.toml:43`: `cpu_max_pct = 10` → `100`, with a manifest comment saying why (JS-component start is CPU-bound; this is the example's own cap, not a platform default; still declared and enforced via cgroup `cpu.max`). Evidence the value rests on: hosted run 43193905404 — cap 10, job 131099823857: Ready after ~220 s, hot swap `upgrade_failed` (door `ATTEMPT_WALL = 90 s`); cap 100, job 131099824049: start ~20 s, whole swap 49.5 s, `completed` (margin 90 / 49.5 ≈ 1.8×). No lower cap was measured, so none is claimed.

Every literal for the example updated: both manifests; both READMEs (`examples/example-spirit-ts/README.md:91-92,107`, `templates/spirit-ts/README.md:64-65,80`: governed-launch paragraph now says CPU 100% and that the 360 s budget was measured under 10%, why the shipped cap is 100%); `scripts/prove_17_3c.py:170` (declared resources assert `{"cpu_max_pct": 100, …}`; its `quota * 100 == period * cpu_max_pct` check already derives from the manifest); `.github/workflows/discipline.yml:2969` comment. Not touched: Rust example/template (`cpu_max_pct = 10`), kernel defaults, test fixtures that merely contain `cpu_max_pct = 10` (`wasm_form_admission_17_3b.rs:54` is a synthetic manifest, not the example). No pin or hash of the example manifest is recorded anywhere (registry/install/vet tests build packages from the live file), so nothing was regenerated. `templates-regen --check` passes (`gates/templates-regen.json`).

## Gates and pins (final tree, `gates/*.json`)

`check-serde-error-handling`, `check-env-contract`, `check-pub-field-constructors`, `check-composition-root-completeness`, `check-service-boundary`, `check-test-cost`, `templates-regen --check`, `check-kernel-baseline`, `check-empty-kernel`, `check-unsafe`, `invariant-lock --changed-files <git diff --name-only>`: all exit 0. `cargo clippy -p maos-bin --features maos-bin/wasm-host --all-targets`: exit 0, no warning in the new code (the remaining `supervision.rs`/`f4_pairing.rs` warnings predate the change: 2213 is `6537454e4`; 3472-3473 cast lint) — `p1-clippy.log`. `rustfmt --check` clean on the touched files.

`python3 scripts/repin.py --check` (`gates/repin-check.txt`), exit 2 (human decision pending, nothing applied): `maos-bin` row 38628 → measured 38677 (**+49**, within the pre-adjudicated +60; this landing's part), `maos-wasm-host` row 3149 → 3178 (+29, another agent's), aggregate 203277 → 203354 (+1 → hard-fail 203355). Combined `--apply --story 20-9-spawned-form-lifecycle-hooks --adjudicated maos-bin,maos-wasm-host` is left to the orchestrator. Kernel untouched (`check-kernel-baseline` ok, src_lines 26062).

## Unverified

* No hosted run of the fix (arm images), and no hosted re-measure of the example at `cpu_max_pct = 100` after the manifest change (the 49.5 s figure is run 43193905404, job 131099824049, taken with the cap overridden in the recipe).
* The 17-3c delegated-cgroup proof (`prove_17_3c.py`, `discipline.yml` line 2 / AC4 jobs) was not re-run: it needs the delegated cgroup unit, hosted only.
* `SESSION_END_WAIT = 15 s` is a bound chosen from the teardown path (1 s grace + reap), not measured on a slow runner.
* One `SessionEnds` entry stays per distinct spirit id for the life of the root (a closed receiver, cheap; not pruned).
