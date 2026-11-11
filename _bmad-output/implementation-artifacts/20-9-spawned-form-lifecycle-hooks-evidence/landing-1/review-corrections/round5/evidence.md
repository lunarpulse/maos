# 20-9 L1 correction round 5 (the last) — evidence (Dev209L1d)

Reviews of the round-4 corrections (`fba6af2a`): `../../review/gpt-review-r4.md` (3 P2 + closure notes) and `../../review/security-r4.md` (GLM-5.3: 1 High, 1 Medium, 1 Low + an adjacent note), persisted verbatim.

## Method

- **Red-before:** a scratch `git archive HEAD` (`e322c7ad`) in `/tmp/base209r5` with ONLY the changed test files, `support/spawned_world_20_9.rs` (a stub artifact name) and the `hostile-twin` fixture source (a new `slow-ready` mode) overlaid, and two scratch shims so the vectors compile at the base: `HookPump::parked()` (the accessor the vector reads) and `LifecycleLink::begin_attempt`/`end_attempt` holding the base's attempt-blind attach and release statements verbatim. Own target dir `/tmp/base209r5-target` (passed per command, never exported), own fixtures; deleted afterwards. Raw log `red/red-e322c7ad.log`: **7 selected vectors, 7 failed**.
- **Plants:** in the final tree, one mutation per fix (`../plants.sh`, exactly-once replace, `PLANT_DIR=round5/plants`, `@@` separator), the proving test, then copy-back + `cmp` + `touch`. Six plant runs, six replacements, raw logs `plants/r5-*.log`, each ending `restored … cmp clean, touched`, none `DID NOT APPLY`. The first `r5-6` run failed in 0.6 s for the wrong reason (a stale `hostile-twin` fixture after a comment edit); it was re-run after rebuilding the fixtures and the log retained is the valid one (red at 100.7 s).
- **Final tree:** `final-focused.log`, `workspace-nextest.log`, `gate-*.log`, `clippy-ratchet.log`, `verb-table-no-default.log`, `check-no-default.log`, `fmt-check.log` (this directory).

## Witness table

| Finding | Witness (final tree) | Red at `e322c7ad` | Plant → result |
|---|---|---|---|
| GPT r4 #1, GLM r4 F3 identical batches | `spawned_lifecycle_20_9::two_identical_forget_batches_are_each_reported_as_they_actually_ran` (budget 6 s, first notice 4 s, second would give up after 3 s) | `purge_undelivered` non-empty (the notice that ran read as undelivered) | `r5-1-identical-batches-share-a-record` red |
| GPT r4 #2 full queue slot | `spawned_lifecycle_20_9::a_telemetry_event_that_finds_the_queue_slot_taken_is_dropped_not_delivered` (frame turn holds the session, consolidation holds the slot) | `an event that never entered the queue is not a delivery: left 1 right 0` | `r5-2-full-slot-not-a-drop` red |
| GPT r4 #3 parked entries | `hook_pump_17_4::finished_wall_overruns_are_not_kept_for_the_life_of_the_pump` (four notices, budget 2 s, hook 3 s, no shutdown) | `4 overruns are still held after four finished notices` | `r5-3-parked-never-reaped` red |
| GLM r4 F1 attempt-blind release | `spawned_swap_20_9::a_late_end_of_an_older_attempt_never_unfences_a_newer_one` (link attempt API: begin, fence, begin again, stale end, own end) | `the retry ends the attempt it replaces` (the base never ended it; the stale-end assertion is not reached there) | `r5-4-stale-end-takes-the-fence` red; `r5-5-begin-keeps-the-old-fence` red |
| GLM r4 F2 one attempt deadline | `spawned_swap_20_9::a_successor_slow_in_each_stage_is_refused_at_one_attempt_wall_not_one_per_stage` (`Ready` after 50 s, then silent: refused at about 90 s) and the two single-stage vectors (never `Ready`, never answers its probe) | slow-ready: `held the swap past its 90 s attempt wall` (guard at 100 s; the base needs about 110 s); the two single-stage vectors refused at 60 s, before the 90 s wall | `r5-6-wall-per-stage` red (100.7 s) |
| GLM r4 F1 pin vehicle | the pump no longer caches a dispatch SCB: removed, no vector (a pin cannot be arranged deterministically) | — | — |
| GLM r4 F1 guest refusal path | `declined`/`failed` now releases the attempt like a transport error: no vector (the probe guest cannot decline a `migrate`/`swap-in`) | — | — |
| closure notes, idle watchdog, lock wait, backoff | dispositioned, no code (kernel source, door contract, session policy) | — | `deferred-work.md` §20-9 L1 correction round 5 |

Not witnessed: (1) the late-drop theft through a pinned successor object (the link API vector and the two plants stand in); (2) the prompt release on a guest refusal; (3) the per-dispatch SCB's cost (one small allocation per event per subscriber that the cache saved; unmeasured); (4) the reviewer's "second identical batch dropped in the queue" shape, which cannot arise now by construction (the second waits for the first).

## Pins (exact, after `cargo fmt --all`, `kloc-check --json`)

| Item | Before (HEAD) | Now | Ceiling |
|---|---:|---:|---:|
| maos-bin | 38019 | 38061 (+42) | 38422 |
| maos-wasm-host | 3137 | 3143 (+6, the `hostile-twin` `slow-ready` mode) | 3348 |
| xtask | 45333 | 45333 (not touched by this story) | — |
| aggregate (clean) | 202018 | 202066 (hardfail 202067) | 202125 (20-5's, the binding one) |

The tree also holds the operator's untracked `spikes/gpui-operator-shell/` and two `*gpui*` planning files (not mine, not touched): the unmodified aggregate reads 202288, and `(unknown:spikes)` is over its own budget, so `gate-kloc-check.log` is red for that reason alone. Kernel 26841 / 21042 unchanged.

## Observed verification (final tree, exit codes)

- `cargo fmt --all --check` 0; `cargo check -p maos-bin --no-default-features --lib` 0; `cargo test -p maos-bin --no-default-features --test verb_table_15_3` 0.
- clippy (`--workspace --all-targets`) 0; ratchet 0 at 1328 occurrences / 868 distinct (unchanged from rounds 3 and 4).
- Workspace nextest (one run, `--profile ci`, real `cargo build -p maos-bin -p maos-cli` first, fixtures rebuilt first): 5968 run, 5966 passed, 2 failed, 137 skipped. The two: `kloc_check::tests::kloc_check_runs_on_workspace` (the operator's untracked `spikes/`, as for `gate-kloc-check.log`) and `erasure_million_20_11::ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (the known flake).
- Explicit feature-gated focused run (`MAOS_17_2_CGROUP_MODE=undelegated`, `--features maos-bin/network,maos-bin/wasm-host`; all 20-9 suites including the three 90 s attempt-wall vectors, `spawned_scope_17_3c`, the 17-x suites, `erasure_live_20_11`, `cohort_daemon_smoke_13_5c`, `one_daemon_one_door_16_1`, `package(maos-wasm-host)`): 337 run, 337 passed, 3 skipped, rc 0.
- Gates at rc 0: kernel-baseline, dev-record-completeness, epic-close-coherence, exit-commands, decision-register, env-contract, manifest-schema-version, equiv-fixture-provenance, wasm-form-equiv, fkcs, `gen-abi-docs --check`, `stability-matrix --check`, invariant-lock; `kloc-check` rc 1 for the foreign `spikes/` reason only (`aggregate 202288 >= 202067`, `(unknown:spikes) 366 > 244`).
