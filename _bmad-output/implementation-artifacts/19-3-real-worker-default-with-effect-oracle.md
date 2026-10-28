---
story_id: '19.3'
story_key: 19-3-real-worker-default-with-effect-oracle
created: 2026-10-28
context_commit: d9bd59c4
baseline_commit: d9bd59c4
security_sensitive: true
kernel_grant: "◆ FLAG-Winston — D19-2 A RATIFIED 2026-10-27 (Lunarpulse): K2 T3 workspace mount, `T3Launch.workspace` → `T3SpawnContext.workspace` → `build_runtime_argv` (`--volume=<dir>:/workspace:rw,U` + `--workdir=/workspace`), grant +12 physical / +9 tokei; endpoints moved by the 20-15 conmon grant and CONFIRMED by D-19-1-01 A: **26390 physical / 20628 tokei** (`epics/epic-19-founder-loop-w4.md:50` (a)). Measured basis: §R reverted prototype @f889b3e4 +12/+9 (26367 → 26379; 20610 → 20619, `epic-19:112-120`); 19-1 creation re-measure @f34f6b4e +11/+9 (`19-1-…-evidence/creation/measurements.md`); this creation @d9bd59c4 by scratch replication without cargo **+11 physical / +9 tokei** (26378 → 26389; 20619 → 20628 — zero tokei slack; `creation/measurements.md` Prototype 1). The cargo re-measure is OWED (`creation/measurements.md` items 1–4). Kernel files unchanged since f34f6b4e. ADR-069: 20628 is 1,372 under the 22,000 alarm. The landed kernel diff must stay the prototype's shape: any extra kernel tokei line (validation, D-19-3-01 B's `--userns` line) breaches 20628 and returns a FLAG-Winston D-row before landing (E16-A7). Re-pin `xtask/kernel-core-baseline.toml:505` (+HISTORY, procedure `:1000-1008`), `xtask/kloc.toml:264`, surface pin `docs/ci-baselines/kernel-surface-v0.1-beta.json` in the same commit."
kloc_grant: "Inside the D19-1 A envelope as extended by the E19 kloc ruling 2026-10-28 (`epics/epic-19-founder-loop-w4.md:40-48`). Measured at d9bd59c4 (tokei replication = the pins): maos-bin 30380 / 31166, xtask 47451 / 47624, maos-bench 2451 / 2490, kernel 20619 / 20628, `_aggregate_hardfail` 189878 / 191061. Forecast under the recommended rows [INFERENCE except the measured legs]: maos-bin **+17…+51** (central ≈ +35: workspace module 45–65, seam 20–30, adapter flag 6–10, smoke literal +1 measured, D-19-3-07 A deletion **−55 measured**); without D-19-3-07 A **+72…+106**; epic 125 (19-3's share × 1.3 = 162). xtask **−5…+1** (D-19-3-08 A; +15–30 under B; epic 30). maos-bench **0** (D-19-3-11 C; +25–35 under A, room 39). Kernel **+9 tokei measured**. maos-control, maos-cli, maos-shell, maos-a2a-core, maos-journey-test, maos-eval 0; `tests/`, `benches/`, `spirits/` uncharged (`xtask/src/kloc_check.rs:303,305,311`). Exact re-pins at landing after `cargo fmt --all` / `kloc-check --json`."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston grant and a sandbox-boundary change (a read-write host bind mount into every T3 Worker; D-19-3-01 decides its ownership/userns posture), plus a completion-oracle change that gates signing (`demo_j1.rs:1392`). Separate Blind (opus-5.5) and Edge (opus-5.5) layers, Acceptance on opus-5.5, Test Infrastructure on glm-5.3. Creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol. The live-manifest edits (D-19-3-09) widen the focused set to every manifest reader (`proof-contract.md:48-54`)."
---

# Story 19.3: Fixture Worker hermetically, real Worker on the operator lane, with an effect oracle ◆ FLAG-Winston

Status: ready-for-dev (RULED 2026-10-28)

<!-- Created 2026-10-28 at d9bd59c4 (19-1 + 19-7 implemented and reviewed, not yet on origin/main b35c9284) by bmad-create-story under the lighter creation contract, in parallel with 19-2 and 19-6 (wave 2; each depends only on 19-1, whose code is in the tree). D-19-3-01…11 are PROPOSED, not ruled — the operator rules; the parent flips `sprint-status.yaml:289` (records-owed R3). NO cargo was run (another cargo process held the host): every cite re-read at HEAD; per-crate kloc by the kloc-check instrument replicated (tokei 14.0.0, argv `xtask/src/kloc_check.rs:294-318`); kernel physical lines by `git ls-files | xargs cat | wc -l`; the K2 prototype, its keep-id variant and the D-19-3-07 A deletion measured on scratch copies under /tmp built from `git show d9bd59c4:`. No crate source changed, even temporarily. Cargo confirmations are listed as "Measurements owed". Evidence: `19-3-real-worker-default-with-effect-oracle-evidence/creation/`. -->

> **The capability:** *a Worker's "done" means something happened. Every T3 Worker gets its own workspace, mounted read-write at `/workspace` and set as its working directory; MAOS snapshots it before the spawn and diffs it after the exit. A real `claude` that answers "I have written the file" over an untouched workspace is no longer `completed` — it is `not_completed:no_effect_evidence`, the codex shape — so a signed run attests remote work, not remote non-refusal. The hermetic path keeps the fixture Worker; the real Worker runs only on the operator lane, and no vendor binary runs on the host outside T3.*

**Closes:**
- Epic-19 §19-3 AC1–AC3 (`epics/epic-19-founder-loop-w4.md:180-186`; stories row `:153`), FR25 (wrapped agent CLI Worker, `_bmad-output/planning-artifacts/prd/functional-requirements.md:64`) and FR52 (external CLI subprocess under capability authority, Tier-3 profile, `:66`) for the founder loop: fixture hermetic, real CLI on the operator lane, an effect-bearing completion verdict.
- `RELEASE-HOLDS.md:66` row 16 ("`claude`'s completion verdict is NOT an effect oracle") — its named successor, "a MAOS-side worktree diff at the spawn seam"; runbook `runbook-j1-t8-two-host-paid-run.md` 부록 B-9 (`:633-662`; abort list `:619`) demoted from blocking to advisory.
- `deferred-work.md:1044` (was `:1125`; admission probes run the Worker binary on the host, outside T3; owner 19-3, operator leg, ruled 2026-10-06 Q5) — closed, narrowed or re-routed per **D-19-3-07**.

**Does NOT close, and says so:**
- The `claude` false NEGATIVE (`deferred-work.md:778`: an incidental denied tool call scores a performed task `permission_denied`) — open in its lane under D-19-3-06 A; the diff this story ships is its enabling signal.
- The `demo-j1` re-point to `j1-founder-loop-epic.toml`, the three new beats and `unlanded_beats()` (19-4); the paid live take itself (operator rows `ops-j1-live-agent-signed-take`, `ops-live-worker-under-17-1-profile`; `epic-19:30-31`); a vendor-CLI T3 image (`docs-site/docs/cookbook/cli-wrapper-spirit.md:79`: "MAOS ships no vendor-CLI image").
- Concurrent Workers (AC3: sequence is the contract); a goal-named-artifact oracle (the diff proves *an* effect, not *the requested* effect).
- Cleanup of retained workspaces (under D-19-3-01 A the Worker's files are owned by the container's mapped uid; the operator removes them with `podman unshare rm -rf`, documented).

## Story

As **the founder running J1 and the operator who signs its live take**,
I want **every Worker to write into a workspace MAOS diffs, and a completion verdict that needs a non-empty diff from a real agent CLI**,
so that **`completed=true` on a signed artifact means the Worker produced something — not merely that nothing was denied — and the hermetic loop stays fixture-only while the paid run stays operator-only.**

**Contract:**
- Epic-19 §19-3 (`epics/epic-19-founder-loop-w4.md:180-186` after records-owed R1); `:159` — create-story may tighten, not add uncited ACs: AC1–AC3 kept; AC4 tightens the Closes line's owned deferral (`:182`).
- §R R19-P2 (`:98`, T3 substrate), R19-P3 (`:99`, a `cwd` never reaches a T3 Worker), R19-P13 (`:109`, bench kloc), kernel table (`:112-120`), D19-2 A (`:139`), D19-3 A (`:140`), E19 kloc ruling (`:40-48`), derived endpoints (`:50` (a)).
- Tracker `sprint-status.yaml:289`; proof contract `docs/testing/proof-contract.md:24-31` (one plant per AC, `:28`), `:33-75` (review, focused set, tiers), `:76-97` (test cost), `:99-142` (build once), `:144-180` (pre-push), `:182-187` (local profile).

## Current-tree corrections — read before implementing

Measured at `d9bd59c4`. `git diff --stat f34f6b4e..d9bd59c4` touches no `maos-kernel-core`, `xtask/src/demo_j1.rs` or `xtask/src/main.rs` path; 19-1/19-7 moved `worker_spawn.rs`, `main.rs`, `delegation.rs`, `discipline.yml`. Full OLD → NEW table: `creation/cites-d9bd59c4.md`.

| Premise (epic / tracker / deferred row) | Fact at `d9bd59c4` and consequence |
|---|---|
| AC2: the workspace reaches a Worker through `T3Launch.workspace` → `T3SpawnContext.workspace` → `build_runtime_argv` | **Holds.** Every Worker is T3 (`spirits/worker/manifest.toml:22-23`; `T3Launch` literal `worker_spawn.rs:1106-1114`); the T3 arm returns before the plain `Command` (`runtime.rs:558-583` vs `:590-605`); argv `t3/argv.rs:70-105` has `--read-only`, `--tmpfs=/tmp`, `--user=65534:65534`, the binary bind `/maos/spirit:ro` (`:86`), no workspace. K2 re-measured: **+11 physical / +9 tokei** (frontmatter). |
| AC2 (K2 shape `rw,U`) — ownership left implicit | podman `:U` recursively chowns the **host** source to the container user's mapped uid ([podman volume docs](https://docs.podman.io/en/v4.4/markdown/options/volume.html)); with `--user=65534:65534` that is a subuid: the daemon cannot read a 0700 workspace afterwards nor delete the Worker's files. `--userns=keep-id:uid=65534,gid=65534` ([userns docs](https://docs.podman.io/en/v4.6.1/markdown/options/userns.container.html), podman ≥ 4.3) keeps host ownership but runs the payload as the operator's real uid → **D-19-3-01**. |
| AC2: "a per-run workspace" — lifetime, location, retention unstated | → **D-19-3-02**. State dirs live under `XDG_RUNTIME_DIR/maos-worker-state` (0700, `egress.rs:242-260`) and are removed at shutdown (`supervision.rs:1835-1847`); a work product needs a retained home. |
| AC2: "snapshots it before spawn and diffs it after exit" | Fingerprint unstated → **D-19-3-03**. |
| AC2: "`claude_result_object_oracle` becomes `Completed` only when the diff is non-empty" | Which adapters (codex is native-effect, `worker_cli.rs:441-500`; fixture is marker-only, writes nothing — `worker-cli-fixture.rs:24`) → **D-19-3-04**; where the gate lives (`parse_completion` is a pure fn of journaled rows, `worker_cli.rs:170`; called `worker_spawn.rs:1286`) → **D-19-3-05**; how it composes with `permission_denials` (`:545-555`; the false negative `deferred-work.md:778`) → **D-19-3-06**. |
| AC2: "the fake-`claude` 'I have written the file' vector … is the proven-red" | **No committed vector exists** — a manual measurement (`runbook-j1-t8-two-host-paid-run.md:647-652`). The T3 fake-CLI pattern exists (`worker_completion_2a.rs:832-835` `plant_fake_cli`, `run_live` `:898-923`, `claude_permission_denial_fails_the_run` `:1041`); this story commits the vector. The parse-level asymmetry test `the_two_oracles_are_not_equivalent_and_the_asymmetry_is_asserted` (`:476-508`) names the old remedy ("a kernel-core `cwd` on BridgeSpawnSpec", `:505`). |
| AC1: "a NEW `--live-claude`" | **The adapter already comes from the topology:** `unlanded_beats` says so (`demo_j1.rs:895`); `live_codex_take` resolves the worker (`resolve_topology_worker` `:1087`) and allows `SIGNABLE_WORKER_CLIS = ["codex","claude"]` (`:384`, checked `:1234`, `:1392`). `--live-codex --codex-topology <claude twin>` runs claude today → **D-19-3-08**. |
| AC1: "`demo-j1` … runs `j1-founder-loop-epic.toml` with three fixture Workers" | `TOPOLOGY` = `j1-founder-loop.toml` (`demo_j1.rs:48`, one Worker); the re-point is 19-4 AC1. The three-Worker hermetic run is `maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md --once` (19-1). |
| AC1: live topologies "under `MAOS_LIVE_AGENT` + `MAOS_HOST_GRANTS`" | Both committed live manifests declare **no `egress`** (`manifest-codex.toml:16-21`, `manifest-claude.toml:85-104`) while T3 runs `--network=none` with egress only through the proxy socket (`argv.rs:79,95-100`; lease `worker_spawn.rs:935-965`) [INFERENCE: an empty request reaches no vendor API] → **D-19-3-09**. The live take still `current_dir`s the daemon into a "disposable demo dir the worker may CRUD inside" (`demo_j1.rs:1331-1334,1345`) — obsolete under K2. |
| AC3: "A NEW `worker_sequence_19_3.rs` asserts three distinct `child_pid`s …, each pair closed before the next `task.assign`" | **Already asserted by 19-1**: `epic_dispatch_19_1.rs:199-318` — per-section order `delegation_routed → … → cli_wrapper_loaded → cli_wrapper_exit → … → delegation_completed` ×3 (`:229-237`), three distinct `child_pid` (`:250-255`) and SCB pids (`:256-267`), safe-point completions (`:316`); enrolled at `discipline.yml:3428` → **D-19-3-10**. The suffix list is `J1_TEST_SUFFIXES` `check_j1_loopback_delegation.rs:129` (`_19_1.rs` added by D-19-1-10 A). |
| AC3: `MAOS_BENCH_REAL_SPAWN=1` "spawning fixture subprocesses" | No T3 provisioning in `nfr-perf-8-orchestrator-fanout` (`discipline.yml:2914-2929`; owner comment `:2917` "retired by 20-7 (ADR-068)"); the only bridge harness spawns at T0 (`maos-bench/src/harness/j1.rs:189-215`); no production Worker path is T0 → **D-19-3-11**. |
| `deferred-work.md:1125` | **Moved to `:1044`.** Two host probes: the kernel handshake for the fixtures (`admission.rs:48-170`, `Command::new` `:74-94`, called via `admit_cli_wrapper_journaled` `worker_spawn.rs:823-831`) and the real-CLI `--version` liveness probe (`worker_cli.rs:259-306`, called `worker_spawn.rs:848-851`); published as "admission probe runs on the host" (`docs-site/docs/cookbook/cli-wrapper-spirit.md:57,79`; `docs-site/docs/deploy/topology.md:144`; ko twin `:58,:80`) → **D-19-3-07**. |
| Holds | `live_agent_gate` `worker_cli.rs:954` (call `worker_spawn.rs:734-736`), `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` `smoke_cli_wrapper_8_12.rs:197` (doc `:189-194`; job step `discipline.yml:3419-3420`); `spirits/worker/manifest.toml:8`; `builtin_fixture_grant` `worker_spawn.rs:228-235`; topology `for` `main.rs:4687`; `block_in_place` → `run_cli_wrapper_manifest` `main.rs:5028-5029` (standalone `:5508-5509`, host B `delegation.rs:900`); `run_cli_wrapper_manifest` `worker_spawn.rs:585` (sync, doc `:574-577`); verdict TL row `:1331-1348`; `RELEASE-HOLDS.md:66`; class rows `xtask/kernel-api-classes.toml:673,686,688`; every shipped topology is routability-checked (`worker_manifests_2a.rs:315-340`). |

**Epic cites corrected (OLD → NEW @`d9bd59c4`)** — the creator may not edit the epic; the exact substitutions are `creation/records-owed.md` R1: `deferred-work.md:1125` → `:1044`; `xtask/src/main.rs:1094-1111` → `:1100-1117`; `main.rs:1097-1098` → `:1102-1104`; `worker_spawn.rs:207-214` (T3 `:211`), probe `:219` → `:228-235` (T3 `:232`), probe `:240-247`; `worker_spawn.rs:712-714` → `:734-736`; doc `:190-194` → `:189-194`; `worker_spawn.rs:1084-1092` → `:1087-1115`; `worker_spawn.rs:563` → `:585` (×2); residual `:515-523` → `:516-523`; `main.rs:4627` → `:4687`; `main.rs:4893-4894` → `:5028-5029`; `discipline.yml:3103-3200` → `:3334-3440`; `:2683-2698` → `:2914-2929`; `:2686` → `:2917`.

## Kernel and kloc pricing — measured at creation

**Kernel (D19-2 A, FLAG-Winston):** measured at d9bd59c4 without cargo (`creation/measurements.md`). Baseline 26378 physical (= pin `kernel-core-baseline.toml:505`) / 20619 tokei (= `kloc.toml:264`).

| Shape | Kernel Δ physical / tokei | Endpoint | maos-bin literal Δ | Basis |
|---|---:|---|---:|---|
| K2 `rw,U` (D-19-3-01 A / C) | **+11 / +9** | 26389 ≤ 26390 · 20628 = 20628 | +2 | `creation/k2-prototype-d9bd59c4.diff` (19-1's diff re-applied; kernel hunks at original lines) |
| K2 + `--userns=keep-id` (D-19-3-01 B) | +12 / +10 | 26390 · **20629 > 20628** | +2 | `creation/k2-keepid-variant-d9bd59c4.diff` |
| Fixture handshake into T3 (D-19-3-07 C) | owed, [INFERENCE] +15–30 tokei | beyond the D19-2 A grant | — | `measurements.md` item 6 |

**kloc at d9bd59c4** (tokei replication = every pin): maos-bin 30380 · xtask 47451 · maos-bench 2451 · kernel 20619 · maos-control 1442 · maos-cli 6317 · maos-shell 613 · maos-a2a-core 5037 · aggregate 189877 (hardfail 189878). `creation/tokei-d9bd59c4.json`.

| Crate · component (recommended rows) | Forecast | Basis |
|---|---:|---|
| maos-bin · NEW `worker_workspace.rs`: create under the data home, bounded metadata walk, diff + summary JSON (D-02 A, D-03 A) | 45–65 | `egress.rs:242-271` private-dir helpers ≈ 30; walk + compare ≈ 25 |
| maos-bin · seam in `run_cli_wrapper_manifest`: create, `workspace: Some(..)`, diff after `wait_and_finalize`, gate, three event/TL fields (D-04 A, D-05 A) | 20–30 | the verdict block `:1286-1348` |
| maos-bin · `WorkerCli::requires_workspace_effect` (default false; codex, claude true) | 6–10 | trait default + two overrides |
| maos-bin · T3 smoke literal `main.rs:7822-7836` | +1 measured | K2 prototype |
| maos-bin · D-19-3-07 A: host liveness probe deleted | **−55 measured** | `creation/d1044-a-liveness-removal-sketch.diff` |
| **maos-bin Σ** | **+17…+51** (central ≈ +35); without D-07 A +72…+106; D-07 B +102…+151 | epic 125; E19 room 31166 − 30380 = 786 for 19-2…19-6 |
| xtask · `"_19_3.rs"` in `J1_TEST_SUFFIXES` (D-10 A), demo-dir removal from the live take | **−5…+1** (D-08 A); +15–30 (D-08 B) | `demo_j1.rs:1331-1334,1345` |
| maos-bench | **0** (RULED 2026-10-28 D-19-3-11 C; +25–35 was A's, room 39) | `harness/j1.rs:189-215` analogue |
| kernel | **+9 tokei** measured | above |
| aggregate | ≈ +45 central (+20…+95) → ≤ 189973 of 191061 | Σ |

Test literal sites (uncharged): `T3SpawnContext` in `worker_terminal_facts_17_7/{runtime,startup,stub}_scenes.rs:56,95,106`, `maos-kernel-core/tests/sandbox_t3_spawn_linux.rs:71`, `t3_terminal_helpers_17_7.rs:361`; `T3Launch` `maos-eval/tests/t3_network_escape_executor.rs:302`; `build_runtime_argv` `worker_resource_limits_17_2.rs:597`, `t3_argv_custody_17_1.rs:70`.

## Decisions — RULED 2026-10-28 (operator rulings via ask; letters and ask ids in the Status column)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-19-3-01** | Workspace mount ownership (the K2 argv; kernel) | **A —** `--volume=<dir>:/workspace:rw,U` + `--workdir=/workspace` (the ratified prototype): the daemon creates the workspace **0755** under a **0700** parent it owns; podman chowns it to the Worker's subuid; the daemon reads it by metadata; the Worker's files stay subuid-owned (cleanup `podman unshare rm -rf`, documented) · **B —** `rw` (no `U`) + `--userns=keep-id:uid=65534,gid=65534`: host ownership kept, but the payload runs as the operator's real uid · **C —** `rw` (no `U`), workspace pre-created **0777** under the 0700 parent: Worker writes as its subuid into an operator-owned dir; nested Worker dirs not removable by the daemon | **A.** It is the ratified shape and measures +11 / +9 (zero tokei slack, exact endpoint). B weakens isolation — today the payload is an unprivileged subuid that owns nothing on the host (`argv.rs:83`); keep-id makes an escape land as the operator — and costs +10 tokei, one over the D19-2 A endpoint (20629 > 20628). C keeps the subuid but leaves a world-writable directory and mixed ownership for the same cleanup gap. A's 0700 parent keeps other host users out (the bind mount needs no parent traversal by the container). | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-02** | Workspace lifetime and location | **A —** one fresh workspace per Worker run, created by `run_cli_wrapper_manifest` at `<data home>/workspaces/<container_name>/` (`container_name` `worker_spawn.rs:1069`), retained after exit, its path in `cli_wrapper_loaded` and the verdict row; no signature change · **B —** one workspace per `maos run`, shared by the epic's Workers in sequence (section k+1 sees section k's files); `run_cli_wrapper_manifest` gains a parameter (3 production callers `main.rs:5029,5509`, `delegation.rs:900`; 3 test callers) and a per-Worker pre-spawn snapshot · **C —** an operator-named `--workspace <dir>` (default A) | **A.** The epic's "per-run … passes … snapshots … diffs" is function-scoped; FR21 hands section k+1 a **distillate**, "always a distillate ref, never raw output" (`spirits/orchestrator/src/lib.rs:16-24`) — B adds a second, untracked cross-Worker channel through files; C under D-01 A would chown the operator's own tree (`:U`), and under B needs keep-id. Retention is required: the work product is the point (B-9 reads it today, `runbook-…-paid-run.md:654-658`). | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-03** | Diff fingerprint | **A —** bounded metadata walk: relative path, file type, size, `mtime_ns` (no content read), entry cap (e.g. 10 000; over the cap = a typed refusal, never a silent truncation) · **B —** content SHA-256 per regular file · **C —** top-level entry count only | **A.** Under D-01 A the Worker's files are subuid-owned and may be 0600 — `stat` needs only directory search permission, a read does not; B fails exactly there and costs a full read of a paid Worker's output; C misses a nested write. Under D-02 A the pre-spawn snapshot is empty by construction (assert it); the fingerprint matters for "modified/removed" only if D-02 B is ruled. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-04** | Which adapters the effect gate applies to | **A —** `claude` and `codex` (codex conjunctive with its native `file_change` evidence, `worker_cli.rs:462-475`); `worker-cli-fixture` and `worker-probe-fixture` exempt (marker oracle; the diff is still reported) · **B —** `claude` only (the epic text) · **C —** every adapter, the fixture writing one deterministic file into `/workspace` so the hermetic run exercises the gate | **A.** Under T3 only `/workspace` and the 64 MiB `/tmp` tmpfs are writable (`argv.rs:81-82`); a codex `file_change` into `/tmp` is native evidence of an effect that is discarded with the container — conjunction refuses it. The fixture's contract is "no filesystem writes, deterministic stdout" (`worker-cli-fixture.rs:24-26`) and 11 consumers ride it (`epic-19:184`); C changes it for a check the T3 fake-CLI vectors already prove hermetically in CI (`discipline.yml:3404`, `require_worker_t3`). | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-05** | Where the gate lives | **A —** the composition-root seam: after `parse_completion` (`worker_spawn.rs:1286`), a `Completed` verdict from an adapter with `requires_workspace_effect()` and an empty diff becomes `NotCompleted(NoEffectEvidence)`; `parse_completion` stays a pure function of journaled rows · **B —** widen `WorkerCli::parse_completion(stdout, stderr, exit, &WorkspaceEffect)` — 4 impls, ≈ 25 unit call sites in `worker_completion_2a.rs` · **C —** a new per-adapter `effect_verdict(&diff)` hook | **A.** One place, the same seam where 2a put every other run-level control (`refuse_missing_argv_flags` `:881`, `refuse_unsafe_argv` `:889`); the six-value label set is unchanged (`worker_cli.rs:87-106`); the asymmetry unit test (`worker_completion_2a.rs:476-508`) stays true at parse level and is re-worded to point at the run-level gate. B re-opens a signature with ≈ 25 call sites for no extra fact; C duplicates A's one-line rule per adapter. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-06** | `permission_denials` vs the diff (`deferred-work.md:778`, the false negative) | **A —** conjunctive: claude completes only on success + `!is_error` + empty `permission_denials` + non-empty diff; `:778` stays open in its lane, cross-referenced · **B —** the diff supersedes the denial check: success + `!is_error` + non-empty diff → `Completed`, denials journaled as a fact in the verdict row; closes `:778` · **C —** a seventh label `completed_with_denials` | **A.** `:780` itself forbids dropping `permission_denials` ("the ONLY thing separating a refusal from a success"); with a diff it is no longer the only thing, but a partial run (one file written, the essential one denied) would score `Completed` under B. `:778` is "LANE: worker-adapter completion semantics" with no owner story (rule 10: not this story's charter). C widens the vocabulary every capture reader and `demo-j1` (`:1392`) consumes. B is the right follow-up once a goal-named artifact exists. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-07** | `deferred-work.md:1044` — admission probes run on the host | **A —** delete the host-side real-CLI liveness probe (`worker_cli.rs:259-306`; arm `worker_spawn.rs:833-858` keeps `check_egress` + the T3 floor): the T3 spawn is the invokability check; the fixtures' kernel handshake stays on the host (repo-built binaries, built-in grants `:228-247`) — the row is narrowed to that half and kept open, owner named by the operator; kernel 0; maos-bin **−55 measured** · **B —** run the liveness probe as a one-shot T3 container through the public `spawn_t3` (the T3 smoke precedent `main.rs:7822-7845`); kernel 0; maos-bin +30–45; ≈ +1 s per live Worker [INFERENCE from 1.16–1.18 s per T3 run, `19-1-…/creation/measurements.md`]; fixture half as A · **C —** A or B, plus the fixture handshake into T3 (`admission.rs:74-94` through `spawn_t3`): closes the row; kernel-touching beyond D19-2 A, priced by prototype (owed, `measurements.md` item 6) · **D —** re-route the row unchanged | **A.** The row's risk is a vendor binary executing unsandboxed on the host (Q5, operator leg); A removes it with negative kloc and no kernel line. The probe proves nothing the T3 spawn does not: it discards output (`worker_cli.rs:259-263`), the adapter verifies shape at completion, and 16-3's guard disposes a failed spawn after bind (`worker_spawn.rs:579-582`). The residual (a PATH-resolved binary named `worker-cli-fixture` runs on the host at admission) is real but bounded by the built-in grant's fixed (image, author). C spends a new FLAG-Winston grant on a hermetic fixture. Under A: `worker_completion_2a.rs:150-192` rewritten, docs narrowed (`cli-wrapper-spirit.md:57,79`, `topology.md:144`, ko `:58,:80`), `supervision.rs:1488` comment. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-08** | Operator-lane selector for the claude Worker (AC1) | **A —** no new flag: ship `spirits/topologies/j1-founder-loop-claude.toml` (the `-codex.toml` twin over `manifest-claude.toml`); `demo-j1 --live-codex --codex-topology <it>` already runs it; correct the codex-only doc strings (`xtask/src/main.rs:1102,1105`, `demo_j1.rs:30`) · **B —** NEW `--live-claude` (+ `--claude-topology`), mutually exclusive with `--live-codex`, both through one renamed live take · **C —** rename `--live-codex`/`--codex-topology` → `--live`/`--live-topology` everywhere (exit-block comment `epic-19:17`, `xtask/tests/check_exit_commands_gate.rs:471`, `runbook-j1-demo.md`, `demo_j1.rs:895`) | **A.** The flag selects *live*, the topology selects the adapter — measured, not assumed (`demo_j1.rs:895,1087,1234,1392`). B adds a second spelling of one switch (+15–30 xtask) and a `run()` signature change in a file 19-4 edits; C is the clean name but rewrites the exit block's comment line and 15-3's exit-parser fixture for a cosmetic gain. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-09** | Live manifests' egress under the 17-1 profile | **A —** add `egress = ["api.anthropic.com:443"]` to `manifest-claude.toml` and `["api.openai.com:443"]` to `manifest-codex.toml` (the cookbook shape, `cli-wrapper-spirit.md:20-25`; `argv_prefix` and its hash unchanged) and correct both manifests' stale cwd/spawn comments · **B —** leave egress to the operator row `ops-live-worker-under-17-1-profile` (committed live manifests stay non-runnable under T3) | **A.** A committed live topology that cannot reach its vendor is decoration (the reason `worker_manifests_2a.rs:10-11` exists); the host grant still bounds every destination (`worker_spawn.rs:691-694`), so declaring the request grants nothing. [INFERENCE: the proxy admits only requested ∩ granted destinations — confirm on the operator leg.] | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-10** | AC3's test file (sequencing is already proven by 19-1) | **A —** cite `epic_dispatch_19_1.rs:199-318` for AC3's sequence/distinct-pid clauses; NEW `crates/maos-bin/tests/worker_workspace_19_3.rs` carries what is new (the effect vectors, three distinct workspaces over the epic run); `"_19_3.rs"` added to `J1_TEST_SUFFIXES` (`check_j1_loopback_delegation.rs:129`) and the job step (`discipline.yml:3428`) · **B —** the epic's `worker_sequence_19_3.rs` as written (a second three-Worker T3 run re-asserting 19-1's clauses) + A's file · **C —** no new file: vectors in `worker_completion_2a.rs` (enrolled `:3404`), the workspace clause added to `epic_dispatch_19_1.rs` | **A.** B duplicates a T3 run (≈ 3.5 s CI, `19-1-…/creation/ci-timings-40609213235.tsv` basis) for assertions that exist; A follows D-19-1-10 A, which named `_19_3.rs` as the next suffix, and keeps the gate's enrolment by construction (`derive_enrolled_targets`, `check_j1_loopback_delegation.rs:772-812`). C is cheaper but mixes 19-3's oracle into a 2a file whose header scopes it to 2a's defect. | RULED 2026-10-28: **A** (ask `w2_19_3`) |
| **D-19-3-11** | AC3's real-spawn bench | **A —** `MAOS_BENCH_REAL_SPAWN=1` mode spawning N `worker-cli-fixture` subprocesses through `spawn_and_bridge` at **T0** (helper under `crates/maos-bench/src/harness/`, the `j1.rs:189-215` shape; ≈ 25–35, room 39); job unchanged (mode off in CI) · **B —** the same at **T3** (`T3Launch`; job gains `provision-t3-test-substrate`; ≈ 40–60, over the room) · **C —** drop the clause from 19-3; NFR-Perf-8's subprocess fan-out goes to the binding perf track `20-7-stable-runner-perf-gates` (`sprint-status.yaml:304`, ADR-068) | **C.** No production Worker spawns at T0 (R19-P2), so A measures a path nothing runs and reports it under an NFR whose subject is Workers; B measures the real path but puts 50 container launches into a 15-minute advisory job its owner marks "retired by 20-7 (ADR-068)" (`discipline.yml:2917`). FR25/FR52 need no bench. A is the cheap way to keep the clause literally. | RULED 2026-10-28: **C** (ask `w2_19_3_d11`) |

## Acceptance Criteria

*(Epic §19-3 as corrected at creation; AC1–AC3 kept, AC4 from the Closes line. Clauses marked (D-…) follow the recommended row; a different ruling rewrites only that clause.)*

### AC1 — command

1. **Hermetic:** `cargo run -p xtask -- demo-j1` (no flag; `--live-codex` is the only live switch, `xtask/src/main.rs:1100-1117`) runs fixture Workers only — `TOPOLOGY` unchanged here (`demo_j1.rs:48`; 19-4 re-points it); `MAOS_LIVE_AGENT` removed on the child (`:550`). And `maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md --once` exits 0 with, per Worker, `cli_wrapper_loaded` carrying a NEW `workspace` field (three distinct existing directories, D-02 A) and `worker_completion` carrying a NEW `workspace_effect` `{created, modified, removed}`; for the fixture the effect is reported, not gated (D-04 A).
2. **Operator lane:** `cargo run -p xtask -- demo-j1 --live-codex --codex-topology spirits/topologies/j1-founder-loop-claude.toml` (D-08 A; NEW topology over `spirits/worker/manifest-claude.toml`, with `egress` per D-09 A) runs one real `claude` Worker under T3 with `MAOS_LIVE_AGENT=1` + `MAOS_HOST_GRANTS` (rows `ops-j1-live-agent-signed-take`, `ops-live-worker-under-17-1-profile`; operator-only, never CI). The live take no longer creates a demo dir or sets the daemon's cwd for the Worker (`demo_j1.rs:1331-1334,1345`); it prints the Worker's workspace path.
3. **The control, unchanged:** `live_agent_gate` (`worker_cli.rs:954`, call `worker_spawn.rs:734-736`) and `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` (`smoke_cli_wrapper_8_12.rs:197`; doc `:189-194`; job step `discipline.yml:3419-3420`). `spirits/worker/manifest.toml:8` stays the hermetic baseline. "Real Worker by default" stays retired.

### AC2 — the T3 workspace and the effect oracle (FLAG-Winston, D19-2 A)

1. **Kernel (K2):** `T3Launch.workspace: Option<PathBuf>` (`runtime.rs:257-268`) → `T3SpawnContext.workspace: Option<&Path>` (`t3/spawn.rs:37-54`; filled at `runtime.rs:570-579`) → `build_runtime_argv(…, workspace, …)` (`t3/argv.rs:25-37`) pushes the mount and `--workdir=/workspace` after the egress socket (`:95-100`) — argv per D-01 A (`rw,U`). Shape = `creation/k2-prototype-d9bd59c4.diff`; validation lives in maos-bin, never in the kernel. Re-pin in the same commit: `kernel-core-baseline.toml:505` (+HISTORY), `kloc.toml:264`, surface pin (`T3Launch`, `T3SpawnContext`, `build_runtime_argv`; class rows `kernel-api-classes.toml:673,686,688` exist). `None` keeps today's argv byte for byte. Argv vectors in `crates/maos-kernel-core/tests/t3_argv_custody_17_1.rs`: mount + workdir present iff `Some`, absent for `None`.
2. **Workspace (maos-bin):** NEW `crates/maos-bin/src/worker_workspace.rs` — create per D-02 A (0755 under a daemon-owned 0700 parent, D-01 A), fingerprint per D-03 A, diff summary. `run_cli_wrapper_manifest` (`worker_spawn.rs:585`) creates it before the spawn, sets `workspace: Some(..)` (`:1106-1114`), diffs after `wait_and_finalize` (`:1195-1204`). No `set_current_dir` (REFUSED: it moves the daemon's own relative paths); no `BridgeSpawnSpec.cwd` (R19-P3).
3. **Gate (D-04 A, D-05 A, D-06 A):** after `parse_completion` (`:1286`), a `Completed` verdict from `claude` or `codex` with an empty diff becomes `NotCompleted(NoEffectEvidence)` (label `not_completed:no_effect_evidence`, `worker_cli.rs:99-101`); `permission_denials` still refuses. The `worker.completion-verdict` TL row (`:1331-1348`) gains `workspace` and `workspace_effect` — the effect is journaled at the Worker's pid beside the verdict, so a capture reads it instead of an operator's `ls` (B-9).
4. **Proven red (committed, T3):** in `worker_workspace_19_3.rs` (D-10 A), `require_worker_t3`, the `plant_fake_cli`/`run_live` pattern (`worker_completion_2a.rs:832-835,898-923`): (a) fake `claude` printing a clean result object "I have written the file", writing nothing → exit ≠ 0, stderr names `no_effect_evidence`, `worker_completion` `completed:false` — the row-16 vector; (b) the same object + `echo ok > h.txt` → exit 0, `completed:true`, `created: 1`, `h.txt` in the reported workspace; (c) fake `codex` emitting an applied `file_change` but writing nothing → `no_effect_evidence` (D-04 A). **Plant (one, `proof-contract.md:28`):** delete the seam gate → (a) exits 0 → red.
5. **Records and docs:** `RELEASE-HOLDS.md:66` row 16 rewritten in place as closed by 19-3, naming the residual (the diff proves *an* effect, not the requested one; D-06 A keeps `permission_denials` conjunctive) and its stale cites (`worker_cli.rs:462-465,:498-537,:490-497` → `:462-475,:524-563,:516-523`); runbook B-9 (`:633-662`, abort list `:619` row 9) demoted to advisory with the same cite fixes; `worker_completion_2a.rs:501-505` message re-pointed; manifest comments (`manifest-codex.toml:50-52`, `manifest-claude.toml:54-55,81-83`) corrected; `docs-site/docs/deploy/topology.md:138` and `docs-site/docs/cookbook/cli-wrapper-spirit.md` (+ ko twin) state the `/workspace` mount, its ownership (D-01) and retention (D-02).

### AC3 — sequence, not concurrency

1. The epic's Workers run in sequence through the 19-1 loop: sequential `for` (`main.rs:4687`), synchronous `run_cli_wrapper_manifest` (`worker_spawn.rs:585`, doc `:574-577`) under `block_in_place` (`main.rs:5028-5029`), the FR21 chain (section k+1 carries section k's distillate) — proven by `epic_dispatch_19_1.rs:199-318` (order `:229-237`, distinct `child_pid` `:250-255`, distinct SCB pids `:256-267`), cited, not duplicated (D-10 A).
2. NEW in `worker_workspace_19_3.rs`: over one epic run, three distinct workspace paths, each created before its Worker's `cli_wrapper_loaded` and none shared (D-02 A). `"_19_3.rs"` joins `J1_TEST_SUFFIXES` (`check_j1_loopback_delegation.rs:129`) and the job step at `discipline.yml:3428` gains `--test worker_workspace_19_3` (`--test-threads=1`); the gate stays green, and its un-enrolled twin (`xyz_19_3.rs`, the `j1_crosshost_1b_proven_red.rs` vector pattern) reds `completion-vectors-enrolled`.
3. ~~Bench: a T0 `MAOS_BENCH_REAL_SPAWN=1` helper in `crates/maos-bench/src/harness/` spawning N `worker-cli-fixture` subprocesses through `spawn_and_bridge` (`orchestrator_fanout_nfr_perf_8.rs:22`; job `discipline.yml:2914-2929` unchanged and advisory).~~ — routed to `20-7-stable-runner-perf-gates` by D-19-3-11 C (operator-approved scope cut); per the ruling, no maos-bench change in 19-3 (kloc forecast: maos-bench **0**).

### AC4 — `deferred-work.md:1044` (owned deferral)

1. Per D-07 A: no real agent CLI binary executes on the host during admission — `run_liveness_probe` and `ProbeStrategy::Liveness` are deleted; the T3 floor and `check_egress` stay (`worker_spawn.rs:834-847`). The fixtures' kernel handshake (`admission.rs:48-170`) is unchanged and the row is narrowed to it.
2. **Proven red (T3):** a fake `claude` whose `--version` branch writes a marker to a host path outside its workspace → after a run the marker is absent. **Plant:** re-insert the `run_liveness_probe` call → marker present → red.
3. `deferred-work.md:1044` updated (narrowed, owner per the operator); published lines narrowed (`cli-wrapper-spirit.md:57,79`, `topology.md:144`, ko `:58,:80`); `supervision.rs:1488` comment.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-19-3-01…11 RULED; tracker → `in-progress`. Run `creation/measurements.md` items 1–4 (and 5/6 if the ruling needs them); if K2 measures above +11 / +9 at the dev HEAD, STOP and return a FLAG-Winston D-row. Confirm `--volume …:U` on the CI rootless images with one hosted run before writing the vectors (this host's rootless podman fails, R19-P2).
- [ ] **T1 — Kernel K2** (AC2.1): three files per the prototype; argv vectors; surface pin; FLAG-Winston obligations a–d (`kernel-core-baseline.toml:1000-1008`).
- [ ] **T2 — maos-bin** (AC2.2–2.3, AC4.1): `worker_workspace.rs` + `lib.rs` module; seam in `run_cli_wrapper_manifest`; `requires_workspace_effect`; event/TL fields; T3 smoke literal; D-07 A deletion; update every test literal site (list above).
- [ ] **T3 — Tests** (AC2.4, AC3.2, AC4.2): `worker_workspace_19_3.rs`; asymmetry test re-worded; `worker_completion_2a.rs:150-192` per D-07.
- [ ] **T4 — xtask / CI** (AC1.2, AC3.2): `_19_3.rs` suffix + un-enrolled twin vector; J1 job step; live-take demo-dir removal and workspace print; doc strings per D-08.
- [ ] **T5 — Data and docs** (AC1.2, AC2.5, AC4.3): `j1-founder-loop-claude.toml`; manifest `egress` (D-09) and comment fixes; RELEASE-HOLDS row 16; runbook B-9; docs-site (en + ko); `deferred-work.md:1044`.
- [ ] **T6 — Plants** (rule 11(c); `proof-contract.md:24-31`): AC2.4 (gate deleted) and AC4.2 (probe re-inserted) — both T3-bound: a branch-only hosted run (`ci/19-3-verify`, the `19-1-…-evidence/ci/ci-19-1-verify.yml` pattern), run id recorded; AC3.2's un-enrolled twin is an in-tree xtask vector.
- [ ] **T7 — Landing.** `cargo fmt --all`; `kloc-check --json`; exact re-pins (kernel `:264`, maos-bin `:517`, xtask `:339` if moved, `:718`) inside the envelope, cumulative use in the record; `check-kernel-baseline` PASSED at the new pin; `check-service-boundary`; `cargo run -q --locked -p xtask -- prepush` exit 0 with wall (20-16); `d11_xtask_ceiling_ratchet` if xtask moves. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Kernel | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:257-268,570-579`, `security/sandbox/t3/spawn.rs:37-54,409-420`, `security/sandbox/t3/argv.rs:25-37,95-100`; `xtask/kernel-core-baseline.toml:505`; `docs/ci-baselines/kernel-surface-v0.1-beta.json` | AC2.1 |
| Worker seam | `crates/maos-bin/src/worker_spawn.rs:585-1361` (create `:1068-1115`, diff after `:1204`, gate after `:1286`, rows `:1146-1170,1287-1348`; D-07 arm `:833-858`), `worker_cli.rs` (trait `:111-245`, probe `:247-306`), NEW `worker_workspace.rs`, `lib.rs`, `main.rs:7822-7836` (literal only), `supervision.rs:1488` (comment) | AC2.2–2.3, AC4.1 |
| Tests | NEW `crates/maos-bin/tests/worker_workspace_19_3.rs`; `worker_completion_2a.rs:150-192,476-508`; `maos-kernel-core/tests/t3_argv_custody_17_1.rs`; literal sites (above) | AC2.4, AC3.2, AC4.2 |
| xtask / CI | `xtask/src/check_j1_loopback_delegation.rs:129`; `xtask/tests/j1_crosshost_1b_proven_red.rs` (twin); `xtask/src/demo_j1.rs:1331-1345` (+ `:30` doc), `xtask/src/main.rs:1102,1105` (doc strings); `.github/workflows/discipline.yml:3428` | AC1.2, AC3.2 |
| Data / docs | NEW `spirits/topologies/j1-founder-loop-claude.toml`; `spirits/worker/manifest-claude.toml`, `manifest-codex.toml`; `RELEASE-HOLDS.md:66`; `_bmad-output/test-artifacts/runbook-j1-t8-two-host-paid-run.md:619-662`; `docs-site/docs/deploy/topology.md:138,144`; `docs-site/docs/cookbook/cli-wrapper-spirit.md:57,79` + ko `:58,:80`; `deferred-work.md:1044` | AC1.2, AC2.5, AC4.3 |
| Kloc | `xtask/kloc.toml:264,517,(339),718` | T7 |

### Overlap with wave-2 siblings and 19-4

- **19-4 (`xtask/src/demo_j1.rs`)**: 19-3 touches only the live take (`:1331-1334,1345`, a printed workspace line near `:1349-1360`) and the module doc `:30` (D-08 A); 19-4 edits `TOPOLOGY` `:48`, beats `:578-589`, `unlanded_beats` `:890-908` (incl. the tier-2 remediation string `:895`, which D-08 B/C would also rewrite), resume beat `:902-906`. Under D-08 B the `run()` signature (`:139-145`, `:257-270`) and `xtask/src/main.rs:1101-1117,1517-1523` are shared too. AC1.2 and AC1-doc are the only 19-3 ACs in that file; AC2/AC3/AC4 do not touch it.
- **19-2** (per its creator): `main.rs` regions `:4853-5360` and `:9419+`, `orchestrator_root.rs`, door/CLI/shell — no overlap except `xtask/kloc.toml:517,718` re-pins and the J1 job step list.
- **19-6** [INFERENCE — not yet written at this creation]: crash delivery likely touches `supervision.rs` and the Worker crash path in `worker_spawn.rs`; 19-3 edits `supervision.rs:1488` (comment, D-07 A) and `run_cli_wrapper_manifest`. Land in sequence; re-measure cites after whichever lands first.
- Shared by all: `xtask/kloc.toml:517,718`, `discipline.yml` J1 job (`:3334-3440`).

### Current state that must be preserved

- `None` workspace ⇒ byte-identical argv (every non-Worker `spawn_t3` caller: T3 smoke `main.rs:7822`, kernel/eval tests).
- The fixture's 3-line SHA-pinned output and marker oracle (`worker-cli-fixture.rs:102-116`; `worker_cli.rs:569-577`); `live_agent_gate` and the CI split test; `refuse_ambient_auth` (`:313-321`), `refuse_missing_argv_flags` / `refuse_unsafe_argv` (`worker_spawn.rs:881-890`); the six-value label set.
- `run_cli_wrapper_manifest` stays sync and guard-covered (`:574-582`); `WorkerRun` (`:1369-1375`) and 19-1's distillate inputs unchanged.
- The live take's preflights (`demo_j1.rs:1203-1300`), the post-run identity check (`:1392`), single-Worker refusal (`:1103-1108`).

### Guardrails

- Kernel diff = the prototype's shape; nothing else in `crates/maos-kernel-core/src` (any extra tokei line breaches 20628).
- Never weaken `permission_denials`, `live_agent_gate`, the T3 floor or the host grant to make a vector pass; a refusal is a finding.
- No process-global `set_current_dir`; no host-side execution of a real agent CLI (D-07 A); workspace parent 0700 and daemon-owned.
- T3 tests: `require_worker_t3`, fresh `MAOS_HOME`, `--test-threads=1` (`MAOS_HOME` is process-global); no test spawns cargo (`proof-contract.md:85-90`); fixtures from `build-test-fixtures` (`discipline.yml:3356`).
- One cargo process; `env -u CI -u GITHUB_ACTIONS`; `CARGO_TARGET_DIR=/mnt/build/cargo`; plants byte-restored.

### CI and the 20-15 build-once contract

- `worker_workspace_19_3` runs own-build in `check-j1-loopback-delegation` (T3 `discipline.yml:3346`) and `workspace-test-suite` (`:5871`, T3 `:5914`); no new archive family.
- Archived sources edited for call shape only (names and counts unchanged): `worker_terminal_facts_17_7` (family `maos-bin-network`, `:2025`, count-pinned `--expected-count "${EPIC17_TERMINAL_SUITE_TESTS}"` `:2257`), kernel T3 tests; `prepush`'s pin reader (20-16) checks them. Adding argv vectors to `t3_argv_custody_17_1.rs` moves that binary's test count — check its consumer pin before landing.
- Cost: three fake-CLI T3 runs + one marker run ≈ 4–5 s [INFERENCE, 1.16–1.18 s per T3 run]; under the 30 s per-binary default.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Effect gate (AC2.4) | delete the seam gate | vector (a) exits 0 |
| Gate enrolment (AC3.2) | un-enrolled `xyz_19_3.rs` | `FAIL completion-vectors-enrolled` (in-tree vector) |
| No host execution (AC4.2) | re-insert the liveness probe | host marker present |
| Kernel argv (AC2.1) | — (pure-fn vectors in `t3_argv_custody_17_1.rs`; the AC2.4 plant covers the path) | — |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo; T3 vectors SKIP without the substrate
env -u CI -u GITHUB_ACTIONS cargo build -p maos-bin --bin maos && cargo run -p xtask -- build-test-fixtures --only worker-cli-fixture
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test worker_workspace_19_3 --test worker_completion_2a --test worker_manifests_2a --test epic_dispatch_19_1 --test smoke_cli_wrapper_8_12 -j 1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core && env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask --test j1_crosshost_1b_proven_red -- --test-threads=1 && env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask demo_j1 -- --test-threads=1
cargo run -q --locked -p xtask -- check-j1-loopback-delegation --json
cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- check-service-boundary && cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-test-cost
cargo run -q --locked -p xtask -- prepush   # before every push (20-16)
```

### Review plan (`proof-contract.md:33-75`)

- DIFFICULT: Blind and Edge (opus-5.5) — Edge walks ownership (subuid files, 0600 files, nested dirs, symlinks inside `/workspace`, the over-cap walk), a Worker that deletes its own workspace, a signal between spawn and diff, and the D-04 matrix (fixture/probe/codex/claude × empty/non-empty diff × denials); Acceptance on opus-5.5; Test Infrastructure on glm-5.3 re-reads the J1 job enrolment and the archived literal sites.
- Focused set: `maos-kernel-core` (+ `--lib`), `maos-bin` (targets above + every manifest reader: `worker_manifests_2a`, `topology_delegation_1a`, `journey_j1`, `demo_j1_tests`, the four `j1_crosshost_*_proven_red`), `maos-eval` (`t3_network_escape_executor`), `check-service-boundary`, `check-kernel-baseline`.

### Previous-story intelligence

- **19-1:** landed maos-bin ≈ +386 against a 205 forecast and maos-bench +92 against 55–75 — the E19 envelope was raised once for it (`epic-19:40-48`); this story's forecast is component-priced with two measured legs. T3 vectors SKIP on this host; plants and the three-Worker run were proven only by a branch-only hosted run (`ci/19-1-verify`); the `_19_1.rs` suffix rule and the un-enrolled-twin vector are the pattern AC3.2 reuses; `run_cli_wrapper_manifest` returns `WorkerRun` (D-19-1-07 A).
- **19-7:** creation without cargo (tokei replication) — the method used here; kloc and kernel claims re-confirmed by the parent.
- **17-1 / 17-7:** every Worker is a T3 container (`podman create`, no `--rm`; PID 1 `/maos/spirit`); state dirs are private and custody-owned (`egress.rs:242-271`, `child.rs:633`).
- **j1-crosshost-2a/2d:** the claude oracle's two defects (false positive B-9; false negative `deferred-work.md:778`); fake-CLI T3 vectors (`worker_completion_2a.rs:751-1302`).

### Git intelligence

| Commit | Content |
|---|---|
| `d9bd59c4` | 19-1, 19-7 (in-progress): halt-row assertion; dev records — this creation's tree |
| `e49b83c4` / `6f872383` | 19-1/19-7 review corrections and candidate (`WorkerRun`, epic loop, `_19_1.rs`) |
| `f34f6b4e` | 20-15/20-16 done (records); 19-1's K2 re-measure (+11 / +9) |
| `b35c9284` | origin/main; landing green 40609213235 |
| `f889b3e4` | §R preflight tree; K2 first priced (+12 / +9) |

### References

- Epic `epic-19-founder-loop-w4.md:30-31,40-50,98-99,109,112-120,139-140,153,159,180-186,212`; tracker `sprint-status.yaml:289,304`; `deferred-work.md:778-780,1044`; `RELEASE-HOLDS.md:66`; runbook `runbook-j1-t8-two-host-paid-run.md:619-662`.
- Code: `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:217-268,552-621`; `security/sandbox/t3/argv.rs:25-105`; `t3/spawn.rs:37-54,341,392-420`; `lifecycle/cli_wrapper/admission.rs:48-170,220`; `crates/maos-bin/src/worker_spawn.rs:228-247,574-1375`; `worker_cli.rs:43-306,441-563,569-662,949-960`; `egress.rs:242-271`; `supervision.rs:1488,1835-1847,2024-2035`; `main.rs:4687,5028-5029,5508-5509,7822-7836`; `delegation.rs:900`; `xtask/src/demo_j1.rs:30,48,139-145,257-270,384,544-552,890-908,1087-1108,1203-1400`; `xtask/src/main.rs:1100-1117,1517-1523`; `check_j1_loopback_delegation.rs:113-129,772-812`; `spirits/worker/src/bin/worker-cli-fixture.rs:1-117`; manifests `manifest.toml`, `manifest-codex.toml:40-68`, `manifest-claude.toml:1-107`; `spirits/topologies/j1-founder-loop-codex.toml`.
- CI: `discipline.yml:2914-2929,3334-3440,3568-3583,5871-5914`; `docs/testing/proof-contract.md:24-31,33-75,76-97,99-142,144-180,182-187`; ADR-069 `:35-46`; podman `--volume`/`--userns` docs (linked above).

## Dev Agent Record

### Story context creation

Created 2026-10-28 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `d9bd59c4`, without cargo (parent rule: another cargo process on the host).

**Inputs:** direct reads of every cited range; `git grep` across f889b3e4 / f34f6b4e / d9bd59c4 for moved rows (`creation/cites-d9bd59c4.md`); kloc by tokei replication (= every pin; `creation/tokei-d9bd59c4.json`); kernel physical lines by `git ls-files … | xargs cat | wc -l` = 26378; three scratch prototypes under `/tmp` (K2 `rw,U` +11 / +9, keep-id +12 / +10, D-07 A −55 maos-bin; `creation/*.diff`, each `git apply --check`-clean against HEAD and never applied); podman `:U` / `keep-id` semantics from the podman docs (linked).

**Measurements owed:** `creation/measurements.md` items 1–6 (cargo confirmation of the baseline, the K2 grant basis, the keep-id variant, the surface-pin delta, the D-07 A deletion, and — only if C is considered — the fixture-handshake-in-T3 prototype).

**Records owed:** `creation/records-owed.md` (R1 epic cite substitutions, R2 premise notes, R3 tracker row, R4 landing records).

**Revert:** nothing to revert — no crate source was modified; `git status --short` shows only this story's files and siblings' untracked evidence dirs; no test run, so `intent-lineage-coverage-report.md` untouched.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- `19-3-real-worker-default-with-effect-oracle-evidence/creation/{measurements.md, cites-d9bd59c4.md, records-owed.md, tokei-d9bd59c4.json, k2-prototype-d9bd59c4.diff, k2-keepid-variant-d9bd59c4.diff, d1044-a-liveness-removal-sketch.diff}`.

### Completion Notes List

### File List

### Change Log

- 2026-10-28: Story created at `d9bd59c4` (ready-for-dev pending the operator's rulings on D-19-3-01…11). Epic §19-3 cites re-measured (14 substitutions owed, records-owed R1); `deferred-work.md:1125` → `:1044`; K2 re-measured by scratch replication (+11 / +9; endpoints 26389 / 20628 under the confirmed 26390 / 20628); cargo confirmations owed.
- 2026-10-28: Operator rulings recorded (ask `w2_19_3`): D-19-3-01…10 RULED **A**; D-19-3-11 RULED **C** (ask `w2_19_3_d11`) — an operator-approved scope cut: AC3's real-spawn bench clause struck from AC3.3 and routed to `20-7-stable-runner-perf-gates`; kloc forecast line pinned to maos-bench **0**; Status → ready-for-dev (RULED 2026-10-28).
