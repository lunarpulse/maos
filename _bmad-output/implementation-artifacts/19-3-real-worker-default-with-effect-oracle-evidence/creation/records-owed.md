# 19-3 creation — records owed (not applied by the creator)

The creator wrote only this story file and `creation/`. Every edit below is for the parent / tracker owner to apply. Substrings are exact (verified with `grep -o` on the epic at d9bd59c4).

## R1 — Epic §19-3 cite corrections (`_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md`)

| Line | OLD substring | NEW substring |
|---|---|---|
| `:182` | `` owns `deferred-work.md:1125` `` | `` owns `deferred-work.md:1044` (was `:1125` @f889b3e4; moved by the 2026-10-27 archival, corrected at 19-3 creation @d9bd59c4) `` |
| `:184` | `` `xtask/src/main.rs:1094-1111` `` | `` `xtask/src/main.rs:1100-1117` `` |
| `:184` | `` `worker_spawn.rs:207-214` (T3, `:211`), probe grant `:219` `` | `` `worker_spawn.rs:228-235` (T3, `:232`), probe grant `:240-247` `` |
| `:184` | `` (exists, `main.rs:1097-1098`) `` | `` (exists, `main.rs:1102-1104`) `` |
| `:184` | `` called `worker_spawn.rs:712-714` `` | `` called `worker_spawn.rs:734-736` `` |
| `:184` | `` doc `:190-194` `` | `` doc `:189-194` `` |
| `:185` | `` `worker_spawn.rs:1084-1092` `` | `` `worker_spawn.rs:1087-1115` `` |
| `:185` | `` `run_cli_wrapper_manifest` (`worker_spawn.rs:563`) `` | `` `run_cli_wrapper_manifest` (`worker_spawn.rs:585`) `` |
| `:185` | `` residual doc `:515-523` `` | `` residual doc `:516-523` `` |
| `:186` | `` (`main.rs:4627`) `` | `` (`main.rs:4687`) `` |
| `:186` | `` (`worker_spawn.rs:563`; `` | `` (`worker_spawn.rs:585`; `` |
| `:186` | `` `main.rs:4893-4894` `` | `` `main.rs:5028-5029` `` |
| `:186` | `` (`discipline.yml:3103-3200`) `` | `` (`discipline.yml:3334-3440`) `` |
| `:186` | `` (`discipline.yml:2683-2698`; ADVISORY with owner and reason `:2686` `` | `` (`discipline.yml:2914-2929`; ADVISORY with owner and reason `:2917` `` |

Noted, not for §19-3 (outside it; report to the epic owner): `:30` `xtask/src/main.rs:1097-1098` → `:1102-1104`; `:27` `xtask/src/main.rs:1094-1111` → `:1100-1117`, dispatch `:1510` → `:1517`, job `:3103-3200` → `:3334-3440`, `v1-0-ship-gate` `:5743` → needs entry `:6164` (job `:6138`), `aggregate` `:5985` → needs `v1-0-ship-gate` `:6406` (job `:6285`), T3 action `discipline.yml:3115,3344` → `:3346,3583`. §R rows R19-16/17/18/24 stay as dated history.

## R2 — Epic §19-3 premise notes (append after the rulings, citing the ruled D-row)

- AC1 "a NEW `--live-claude`": `demo-j1 --live-codex` already takes its adapter from the topology (`xtask/src/demo_j1.rs:895`, `resolve_topology_worker` `:1087`, `SIGNABLE_WORKER_CLIS = ["codex","claude"]` `:384`) → D-19-3-08.
- AC1 "`demo-j1` … runs `j1-founder-loop-epic.toml`": `TOPOLOGY` is `j1-founder-loop.toml` (`demo_j1.rs:48`); the re-point is 19-4 AC1's.
- AC2 "the fake-`claude` … vector is the proven-red": the vector exists only as a manual runbook measurement (`runbook-j1-t8-two-host-paid-run.md:647-652`); no committed test → 19-3 commits it.
- AC3 "NEW `worker_sequence_19_3.rs` asserts three distinct `child_pid`s …": already asserted by 19-1 (`crates/maos-bin/tests/epic_dispatch_19_1.rs:199-318`: per-section order `:229-237`, three distinct `child_pid` `:250-255`, three distinct SCB pids `:256-267`) → D-19-3-10.
- AC3 bench: every Worker is T3 (R19-P2); the bench job has no T3 provisioning (`discipline.yml:2914-2929`) → D-19-3-11.

## R3 — Tracker (`_bmad-output/implementation-artifacts/sprint-status.yaml:289`)

- OLD: `  19-3-real-worker-default-with-effect-oracle: backlog  # worktree-diff effect oracle (maos-bin, explicit cwd); real CLI default under 17-1 profile (live = operator lane); RELEASE-HOLDS row 16`
- NEW (after the operator rules D-19-3-01…11): `  19-3-real-worker-default-with-effect-oracle: ready-for-dev  # CREATED 2026-10-28 at d9bd59c4; D-19-3-01…11 <RULING> — story: 19-3-real-worker-default-with-effect-oracle.md; K2 T3 workspace mount (D19-2 A; measured +11 physical / +9 tokei at d9bd59c4 by scratch replication, cargo confirmation owed) | WAS: worktree-diff effect oracle (maos-bin, explicit cwd); real CLI default under 17-1 profile (live = operator lane); RELEASE-HOLDS row 16 — "explicit cwd" retired by R19-P3 (T3 workspace mount), "real CLI default" retired by epic AC1 (fixture hermetic, real CLI operator lane)`
- Also bump `last_updated` per the file's convention.

## R4 — At landing (owned by the dev pass, listed here so nothing is lost)

- `deferred-work.md:1044` — disposition per D-19-3-07 (A: narrow to the fixture handshake, owner named by the operator; B: same; C: CLOSED by 19-3; D: owner re-pointed).
- `deferred-work.md:778,780` — append a cross-reference under D-19-3-06 A: "19-3 ships the workspace diff (`worker.completion-verdict` row carries it); the false-negative stays open in its lane". Under D-19-3-06 B: CLOSED by 19-3. Cites `worker_cli.rs:498-537`/`:490-497`/`:462-465` → `:524-563`/`:516-523`/`:462-475`.
- `RELEASE-HOLDS.md:66` row 16 rewritten in place (no renumbering; "row 16" is cited across the tree, e.g. `_bmad-output/test-artifacts/j1-t8-step0-claude-probe.sh:12,102`).
- `runbook-j1-t8-two-host-paid-run.md:619-662` B-9 demoted to advisory.
- `xtask/kernel-core-baseline.toml:505` + HISTORY, `xtask/kloc.toml:264` (kernel), `:517`, `:339`/`:520` if moved, `:718`; surface pin `docs/ci-baselines/kernel-surface-v0.1-beta.json` (`T3Launch` `:757,:1497`, `T3SpawnContext` `:992`).
