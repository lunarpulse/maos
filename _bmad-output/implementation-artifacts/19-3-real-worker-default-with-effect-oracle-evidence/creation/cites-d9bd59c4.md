# 19-3 creation — cites re-measured at `d9bd59c4` (2026-10-28)

Method: direct `read`/`sed -n` of every range; moved rows located with `git grep -n -F <text> <rev>` across f889b3e4 / f34f6b4e / d9bd59c4; `git diff --stat f34f6b4e..d9bd59c4` (19-1/19-7 moved `worker_spawn.rs` +22 at the spawn, `main.rs` +60…+193, `discipline.yml`; no `maos-kernel-core`, `xtask/src/demo_j1.rs` or `xtask/src/main.rs` change). No cargo.

## Epic §19-3 (`epics/epic-19-founder-loop-w4.md:180-186`) — OLD → NEW

| Where | OLD (@f889b3e4) | NEW @d9bd59c4 | Measured |
|---|---|---|---|
| Closes `:182` | `deferred-work.md:1125` | `deferred-work.md:1044` | `git grep -F 'Admission still executes the manifest-named Worker binary on the host'`: f889b3e4 `:1125`, f34f6b4e `:1044`, d9bd59c4 `:1044` (2026-10-27 archival) |
| AC1 `:184` | `xtask/src/main.rs:1094-1111` | `DemoJ1` `:1100-1117` (doc `:1097-1099`) | read |
| AC1 `:184` | `--live-codex` `main.rs:1097-1098` | `:1102-1104` (doc `:1102`, field `:1104`); `--codex-topology` `:1105-1107`; dispatch `:1517-1523` | read |
| AC1 `:184` | `builtin_fixture_grant` `worker_spawn.rs:207-214` (T3 `:211`), probe `:219` | `:228-235` (T3 `:232`); `builtin_probe_grant` `:240-247` (T3 `:244`); allowlist `:322-323` | grep |
| AC1 `:184` | `live_agent_gate` call `worker_spawn.rs:712-714` | `:734-736` | read |
| AC1 `:184` | doc `smoke_cli_wrapper_8_12.rs:190-194` | `:189-194` (test `:197` holds) | read |
| AC1 `:184` | holds | `worker-cli-fixture.rs:52-117`; `spawn_and_bridge` `runtime.rs:610-621`; `journey_j1.rs:186-195`; `demo_j1.rs:48`, `:544-552`, `:550`; `worker_cli.rs:954`; `manifest.toml:8` | read |
| AC2 `:185` | T3 spawn `worker_spawn.rs:1084-1092` | `BridgeSpawnSpec` literal `:1087-1115` (`T3Launch` `:1106-1114`) | read |
| AC2 `:185` | `run_cli_wrapper_manifest` `worker_spawn.rs:563` | `:585` (returns `WorkerRun` `:1354-1375` since 19-1) | grep |
| AC2 `:185` | residual doc `worker_cli.rs:515-523` | `:516-523` (oracle `:524` holds; codex `:491` holds) | read |
| AC2 `:185` | holds | `manifest.toml:22-23`; `runtime.rs:558-583` / `:590-605` / `:217-251`; grep `current_dir` in `lifecycle/cli_wrapper/` = 0; `t3/argv.rs:70-105`; `t3/spawn.rs:37-54`; `manifest-codex.toml:50-51`; `RELEASE-HOLDS.md:66`; runbook `:619`, B-9 `:633-662` | read |
| AC3 `:186` | `for` `main.rs:4627` | `:4687` | grep |
| AC3 `:186` | `block_in_place` → `run_cli_wrapper_manifest` `main.rs:4893-4894` | `:5028-5029` (standalone arm `:5508-5509`; host B `delegation.rs:900`) | grep |
| AC3 `:186` | `worker_spawn.rs:563` | `:585` | grep |
| AC3 `:186` | job `discipline.yml:3103-3200` | `check-j1-loopback-delegation` `:3334-3440` (19-1 step `:3428`; `worker_completion_2a` step `:3404`) | awk next-job |
| AC3 `:186` | `nfr-perf-8-orchestrator-fanout` `discipline.yml:2683-2698`, owner `:2686` | `:2914-2929`, `continue-on-error` + owner `:2917` (text now "retired by 20-7 (ADR-068)"), bench step `:2929` | grep |
| AC3 `:186` | holds | bench `orchestrator_fanout_nfr_perf_8.rs:22`; `kloc_check.rs:305`; harness `j1.rs:198` | read |

## Outside §19-3 — noted, not edited by this story's correction set

| Where | OLD | NEW @d9bd59c4 |
|---|---|---|
| Operator-lane legs `:30` | `xtask/src/main.rs:1097-1098` | `:1102-1104` (`live_codex_take` `demo_j1.rs:1203`, spawn `:1340-1349` hold) |
| Exit provenance `:27` | `xtask/src/main.rs:1094-1111`, dispatch `:1510`; job `:3103-3200`, `v1-0-ship-gate` `:5743`, `aggregate` `:5985`; T3 action `discipline.yml:3115,3344` | `:1100-1117`, dispatch `:1517`; job `:3334-3440`; `v1-0-ship-gate` job `:6138` (needs entry `:6164`); `aggregate` job `:6285` (needs `v1-0-ship-gate` `:6406`); T3 action `:3346,:3583` (19-1 recorded `:6156`/`:6398` at f34f6b4e for the same two anchors) |
| §R R19-16/17/18, R19-24 `:83-85,:91` | history rows | kept as history (§R is dated @f889b3e4) |

## Stale cites inside files this story edits (AC2/AC4) — `deferred-work.md:778,780` is records-only (`records-owed.md`), not a 19-3 code edit

| File | OLD | NEW |
|---|---|---|
| `RELEASE-HOLDS.md:66` (row 16) | `worker_cli.rs:462-465`, `:498-537`, `:490-497` | `:462-475` (codex `file_change`), `:524-563` (claude oracle), `:516-523` (residual) |
| runbook B-9 `runbook-j1-t8-two-host-paid-run.md:639,642` | same three | same three |
| `deferred-work.md:778,780` (claude false-negative row) | `worker_cli.rs:498-537`, `:490-497`, `:462-465`, `:490-537` | as above |
| `crates/maos-bin/tests/worker_completion_2a.rs:501-505` message | "deferred: it requires a kernel-core `cwd` on BridgeSpawnSpec" | the T3 workspace mount + diff (R19-P3) |
| `spirits/worker/manifest-codex.toml:50-52` | "MAOS does NOT set the child CWD … launch `maos run` FROM $DEMO" | Worker cwd = `/workspace` (K2) |
| `spirits/worker/manifest-claude.toml:54-55,81-83` | "The spawn sets no cwd"; "MAOS's own spawn is a bare `Command::new` with no namespace" | T3 container, `/workspace` cwd (stale since 17-1 and K2) |
| `xtask/src/demo_j1.rs:1331-1334,1345` | "A disposable demo dir the worker may CRUD inside" + `.current_dir(&demo_dir)` | obsolete under K2 — the Worker's cwd is its workspace |
| `crates/maos-bin/src/supervision.rs:1488` | "a 10 s real-CLI liveness probe" | only under D-19-3-07 A |
