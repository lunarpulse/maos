---
story_id: '19.1'
story_key: 19-1-orchestrator-dispatch-loop-and-epic-input
created: 2026-10-28
context_commit: f34f6b4e
baseline_commit: f34f6b4e
security_sensitive: true
kernel_grant: "none — kernel-Δ 0 by construction, measured at creation: no AC and no option of D-19-1-02…15 touches `crates/maos-kernel-core/src` (every API the story uses is public outside it: `HaltRegistry::drain_for_spirit_dry_run` `halt/mod.rs:389`, the `DistillationPort` impl in maos-iac `adapter/distillate.rs:309-428`, the FR21 gate in maos-iac `adapter/orchestrator_dispatch.rs:63-146`). `check-kernel-baseline` PASSED 26378 / 95 files and tokei 20619 (`xtask/kloc.toml:264`) at `f34f6b4e`, and re-PASSED after both throwaway prototypes were reverted. No FLAG-Winston row. If the kernel figure moves at landing, the story stops and returns a FLAG-Winston D-row (E16-A7)."
kloc_grant: "Inside the D19-1 A envelope (RATIFIED 2026-10-27; xtask rebased 2026-10-28 by D-20-16-07 A): maos-bin 29497 → ≤ 30563, maos-bench 2359 → ≤ 2476, xtask 47422 → ≤ 47624, `_aggregate_hardfail` 188505 → ≤ 190055 (endpoint per D-19-1-01). Forecast [INFERENCE except the measured parse leg] maos-bin +145–230 (epic 205; `--epic` parse measured +9 by prototype), maos-bench +55–75 under D-19-1-13 A (+90–110 under B), xtask +20–35; maos-eval, maos-control, maos-cli, maos-shell, maos-journey-test, kernel 0; `spirits/`, `tests/`, `benches/` uncharged (`xtask/src/kloc_check.rs:303,305,311`). Exact re-pins at landing after `cargo fmt --all` / `kloc-check --json`; STOP only if the landing exceeds an envelope ceiling. Cumulative E19 use after 19-1 (first story; central forecast): maos-bin ≈ 190 of +1066, maos-bench ≈ 65 of +117, xtask ≈ 28 of +202."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier — no kernel grant, but security-critical ACs (`proof-contract.md:67-71`): AC1/AC4 build consent-enveloped A2A `task.assign` frames per section (ADR-012: granter == `frame.from` on spirit and host), AC2 changes the paid-path goal rule (cross-host REQUIRED, loopback refusal), AC3 gives the Orchestrator a `provider.complete` capability and an audited per-pass mint, AC5 publishes through 18-2's signed sidecar. Separate Blind (opus-5.5) and Edge (opus-5.5) layers, Acceptance on opus-5.5, Test Infrastructure on glm-5.3. Creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol. The Orchestrator manifest change widens the focused set (`proof-contract.md:48-54`)."
---

# Story 19.1: Orchestrator dispatch loop and `--epic` input

Status: in-progress

<!-- Created 2026-10-28 at f34f6b4e by bmad-create-story under the lighter creation contract (no party-mode preflight), after the ratified Epic-19 §R preflight (D19-1 A … D19-6 B, 2026-10-27) and the 20-15/20-16 landing (main 271e2a33..b35c9284, run 40609213235; f34f6b4e is records-only). D-19-1-01…15 PROPOSED — the parent flips this file and `sprint-status.yaml:287` to ready-for-dev after the operator rules. Measured, not read: every 19-1 cite re-read at HEAD (four corrected in the epic, below); `check-kernel-baseline` + `kloc-check --json` at HEAD; a throwaway K2 prototype (19-3's D19-2 A shape) to confirm or re-derive the E19 kernel/aggregate endpoints; a throwaway `--epic` parse prototype for the maos-bin forecast; both reverted, gates re-PASSED; per-test CI cost from the landing run's workspace junit. One cargo process; CARGO_TARGET_DIR=/mnt/build/cargo; `env -u CI -u GITHUB_ACTIONS`. No crate source changed; no workspace test run, so `intent-lineage-coverage-report.md` was not touched. Evidence: `19-1-orchestrator-dispatch-loop-and-epic-input-evidence/creation/`. -->

> **The capability:** *the founder hands MAOS an epic file; the Orchestrator splits it deterministically into its `### Story` sections and dispatches them one at a time to three real (fixture) Workers over the loopback A2A leg. After each Worker completes, the root distils that Worker's journaled output into a `Distillate` row at the Worker's own pid, and the next section's `task.assign` carries a reference to it — FR21 in production, checked by the kernel-side IAC gate on real rows. The Orchestrator also gets its inference port (the 18-1 seam shape), and its halt behaviour is scored on a completion-consuming corpus like the Butler's.*

**Closes:**
- Epic-19 §19-1 AC1–AC5 (`epics/epic-19-founder-loop-w4.md:160-168` after the corrections below; stories row `:150`), FR21 in production (the first production callers of `DistillateWriter::write_distillate` and of an FR21 follow-up), NFR-Test-4 / FR15 for the Orchestrator class (AC5, received 2026-10-25 from 18-2 by D-18-2-05 A; routing audit `routing-audit-2026-10-22.md:36`).
- `deferred-work.md:1178` (`maos eval halt` works only from the workspace root; owner this story).
- The pending-halt check before each `task.assign` (R19-P9) **if D-19-1-09 A is ruled**; otherwise 19-4 keeps it.

**Does NOT close, and says so:**
- The live ambiguity halt: the scalar port (`with_scalar_port`), the allowlist flip (`main.rs:506-532`, tests `:14111`, `:14404-14424`), the halt journaled under `task.acceptance_criterion.ambiguous` — 19-4 AC2. This story only gives the Orchestrator the judgement and the port (D-19-1-12).
- FR20 enqueue/recall over the door (19-2), the effect oracle and the real Worker (19-3), the `demo-j1` re-point and the three new beats (19-4), the J1 cassette `cassettes/j1/founder-loop.json` (19-4 AC4).
- Epic dispatch on the cross-host arm (refused under D-19-1-02 A: the far Host runs the Worker and nothing returns, `main.rs:4844-4871`).
- FR21's 60 s wall-clock proxy (`delegation.rs:274-295`): the FIRST dispatch of a new run within 60 s of an earlier run's `TaskComplete` on the same data home is still refused. Unchanged; tests use a fresh `MAOS_HOME`.
- A nightly re-record leg for the Orchestrator cassette (operator lane, as 18-2's was); `maos eval halt` from an installed binary outside a source checkout (D-19-1-15 A keeps today's limit).

## Story

As **the founder running the J1 loop**,
I want **`maos run … --epic <file>` to dispatch each story of my epic to a Worker in turn, each follow-up built on the previous Worker's distillate**,
so that **the Orchestrator runs a real multi-step plan under FR21 instead of one hard-wired delegation — and its halts are measured like every other class's.**

**Contract:**
- Epic-19 §19-1 (`epics/epic-19-founder-loop-w4.md:160-168`; `:158` — create-story may tighten, not add uncited ACs; the five ACs kept, the R19-P9 halt check placed by D-19-1-09); §R R19-P4, P5, P6, P9, P12 (`:99-107`); D19-1 A, D19-4 A (`:137,140`); xtask rebase (`:49`).
- Tracker `sprint-status.yaml:287`; proof contract `docs/testing/proof-contract.md:24-31` (plants), `:33-75` (review, focused set, tiers), `:76-97` (test cost, fixtures), `:99-142` (build once, 20-15), `:144-180` (pre-push, 20-16), `:182-187` (local profile).
- Confidence rules 9 (refine at HEAD), 10 (route by charter), 11 (a control is real or labelled) (`epic-19:53-55`).

## Current-tree corrections — read before implementing

Measured at `f34f6b4e`. `git diff --stat f889b3e4..HEAD` touches no file under `crates/maos-bin/src`, `spirits/`, `crates/maos-bench`, `crates/maos-iac`; the §R cites into them hold except the two below. What moved is `.github/workflows/discipline.yml` (20-15), `xtask/src/main.rs` (+7), the kloc/kernel pins and `t3/spawn.rs` (conmon).

| Premise (epic / tracker) | Fact at `f34f6b4e` and consequence |
|---|---|
| AC1: three `host`-bearing Worker entries in a new topology | **Holds.** `topology.rs` has no host-count cap (`validate_remote_topology_target` `:31`); Workers get SCB ids `worker`, `worker-2`, `worker-3` (`supervision.rs:717,1926`). `j1-founder-loop.toml:28-30` stays pinned to one `host` by the gate (`check_j1_loopback_delegation.rs:249-263`) and `topology_delegation_1a.rs:228-238`. Every host-bearing topology lists the orchestrator first (`j1-founder-loop.toml:9`, `-crosshost.toml:44`, `-codex.toml:28`). |
| AC1: N frames via `assign_frame_remote`, "one per section" | Mapping section → Worker entry and the count-mismatch behaviour are unstated → **D-19-1-03**. |
| AC4: "dispatches section k+1 with `followup_dispatch`" | **Cannot as written.** `followup_dispatch` (`spirits/orchestrator/src/lib.rs:389-401`) builds through `assign_frame` (`:252-300`: `host_id: None`, `consent_envelope: None`). The loopback leg routes only host-bearing recipients (`delegation.rs:248-251`; no router ⇒ `CrossHostNotConfigured`), and the gate demands `assign_frame_remote` (`check_j1_loopback_delegation.rs:271`). A remote, prior-carrying builder is needed → **D-19-1-06**. |
| AC4 / R19-P4: "the producer pid is fixed at story creation" | **Measured: only the Worker's pid works.** `write_distillate` refuses a citer whose pid differs from any flattened source row's pid (`CiterUnauthorized`, I11, Story 8.10 AC2(b): `crates/maos-iac/src/adapter/distillate.rs:348-377`). The Worker's `CliSubprocessOutput` rows are journaled at its real pid (pump `worker_spawn.rs:1161-1167`, finalize `:1173-1175`, read back by pid `:1229-1233`, intent `cli.subprocess.output` — non-empty lineage, `runtime.rs:942-951`). `run_cli_wrapper_manifest` returns only `WorkerCompletion` (`worker_spawn.rs:563-577`) → **D-19-1-07**. |
| AC4: "the FR21 60 s-window check passes on the real rows" | The gate is `check_orchestrator_distillate_required` (`maos-iac/src/adapter/orchestrator_dispatch.rs:63-146`, called from `deliver_typed`): once ANY `TaskComplete` sits in the TL within 60 s, every Orchestrator `task.assign` must carry `prior_distillate_ref` resolving to a `Distillate` row with depth ≥ 1, else `EOrchestratorDispatchRawOutput` and the row is not written. `journal_completion` writes exactly such a `TaskComplete` after Worker k (`delegation.rs:374-392`), so frames 2…N need the distillate and frame 1 needs none. The comment at `delegation.rs:274-295` names the cross-run false positive (unchanged here). |
| AC2: "the loopback fallback string is replaced by the `--epic` sections" | Every no-`--epic` loopback consumer relies on that string: `demo-j1` (`demo_j1.rs:48,544-545`), `journey_j1.rs:85,284,310`, `smoke_cli_wrapper_8_12.rs:164`, `drain_once_audit_writer.rs:40`. Only the sentinel test sets `MAOS_DELEGATED_GOAL` on loopback (`smoke_cli_wrapper_8_12.rs:281`); `f4_pairing.rs:483,526`, `two_host_delegation_2b.rs:497`, `root_shutdown_unload_16_3.rs:2007` use the cross-host topology. Tracker `:287` says "MAOS_DELEGATED_GOAL removed on loopback arm" → **D-19-1-02**. |
| AC2: "enrolls its tests in that derivation" | The derivation is suffix-based (`J1_TEST_SUFFIXES` `:121`, `derive_enrolled_targets` `:772-812`), and 15-1's D-2 rule EXCLUDES a stem with a decimal segment before the suffix (`:795-807`) — a story-numbered `*_19_1a.rs` would be dropped, not enrolled → **D-19-1-10**. A new gate leg would ripple into `ledger_leg_names()` (`:145`), its pinned list (`j1_crosshost_1b_proven_red.rs:793-800`) and demo-j1's beats (`demo_j1.rs:232,805-813,843`) → **D-19-1-11**. |
| AC3: "Orchestrator gains `with_deferred_inference_port`" — which instance? | Three construction sites build a bare Orchestrator: hot-swap successor factory (`main.rs:993-995`), topology arm (`:4997-5001`), standalone arm (`:5826-5830`); plus the root's own `delegation_emitter` (`:4611-4612`), which has no pid and its own in-flight state. `Orchestrator` is `Clone` over `Arc<Mutex<DispatchState>>` (`lib.rs:133-139`) → **D-19-1-04**. `spirits/orchestrator/tests/spirit_smoke.rs:73-74,104-108` assert "no `[capabilities.required]`" (flip). |
| AC3: the provider list for `[capabilities.required] provider.complete` | Unstated → **D-19-1-05**. |
| AC5: "AC3 gives the Orchestrator its inference port, so the corpus is built where its scalar first comes from a completion" | No completion→scalar judgement exists (`grep dispatch_ambiguity` outside tests/manifests = 0; `InferencePort` in `spirits/orchestrator/src/lib.rs` = 0); 19-4 AC2 wires the live write → ownership **D-19-1-12**. A Butler-like model-driven arm is ≈ 2× the epic's maos-bench 60 if copied (`butler_verdicts` 97 code lines, `halt_eval.rs:383-481`) → **D-19-1-13**. |
| AC5: owns `deferred-work.md:1260` | **Moved to `:1178`** (2026-10-27 archive split). Its suggested fix ("the workspace root `main.rs` already computes") exists only inside `mod tests` (`main.rs:13992`, `workspace_root()` `:14638-14639`) → **D-19-1-15**. Where the class gate runs in CI → **D-19-1-14**. |
| R19-P9: the loop must consult the pending halt | `HaltRegistry::drain_for_spirit_dry_run` is `pub` (`halt/mod.rs:389`) — kernel-Δ 0 for either owner; `HaltPending` gates only `on_idle` (`hook_dispatch.rs:43-44`, `:281-285`) → **D-19-1-09**. |
| R19-P12: 18-6 forget / 18-7 skill queue | `forget_with_reason` `memory/mod.rs:437-606` (§R cited `:435-602`; doc from `:429`) scrubs distillate bodies naming the forgotten principal via `scrub_forget_frames` (`:546-555`); a 19-1 distillate enters the cascade through its body (→ D-19-1-08) or its sources (the Worker's rows). The skill queue is boot-non-fatal (`main.rs:2583-2586`): no interaction. |
| R19-P2: the hermetic run needs T3 | The new J1 tests call `require_t3` and SKIP locally without the substrate (`proof-contract.md:182-187`); `check-j1-loopback-delegation` (`discipline.yml:3346`) and `workspace-test-suite` (`:5906`) provision it. [INFERENCE] this host's rootless podman still fails as §R measured @f889b3e4 (not re-run). |
| E19 kernel/aggregate endpoints "derived, confirm at the 19-1 preflight" (`epic-19:49`) | K2 re-measured by prototype at HEAD: +11 physical / +9 tokei → **D-19-1-01**. |
| Tracker `sprint-status.yaml:287`: "reuse 18-2's scorer if present, else own harness" | Stale (R19-P6: present; fallback retired). Not edited here — flagged for the tracker owner. |
| Holds | `parse_run_args` and its error `worker_spawn.rs:84-147`, `:109-113`; `RunArgs` `:55-64`; `RUN_USAGE` `:75-76`; `tests/fixtures/` has no `j1/`; emitter `main.rs:4611-4612`, `for` `:4627`, no-loop comment `:4670-4677`, `build_task_assign` `:4764`, `assign_frame_remote` `:4770`; goal read `:4739-4741`, REQUIRED `:4744-4758`, fallback `:4759-4762`; `env_contract.rs:32` (doc `:33-35`); writer `main.rs:2658-2661`, callers `:6805` (smoke), `:10126` (harness); `first_dispatch` `lib.rs:372`; `port_builder` `main.rs:3912-3937`, resolver `:3807`; `researcher_root.rs:179-212` (`attach`, `bind` `:195`); manifest `:7,18-20,33-36,37-48`; `halt_eval.rs` `Composition` `:60-63`, `CLASSES` `:84-116` (doc `:82-83`), `USAGE` `:118`; floors `onboarding_gate_corpus.rs:62,65`; FR15 `functional-requirements.md:49`; NFR-Test-4 `non-functional-requirements.md:81`; `docs/adr/` tops at ADR-069; gate registry `xtask/gate-registry.toml:465-467` (list `:112`). |

**Epic cites corrected in place (OLD → NEW @`f34f6b4e`; `epics/epic-19-founder-loop-w4.md`, §19-1 only):**
- AC1 `:164`: `worker_spawn.rs:84-150` @f889b3e4 → `:84-147` (the fn ends at `:147`).
- AC2 `:165`: job `discipline.yml:3103-3200` → `:3334-3420`; `v1-0-ship-gate` `:5743` → `:6156`; `aggregate` `:5985` → `:6398` (20-15 workflow reshape).
- AC3 `:166`: `butler_root.rs:182-200` → `context_binder` `:165-192`, `context_window` `:196-204`.
- AC5 `:168`: `deferred-work.md:1260` → `:1178`.
- Noted, not edited (outside §19-1): exit provenance `:27` cites `xtask/src/main.rs:1094-1111` (now `DemoJ1` `:1100-1117`, doc `:1097-1099`), the same three workflow lines and the T3 action at `discipline.yml:3115,3344` (now `:3346,3575`); §19-2 AC4 `deferred-work.md:1277` is now `:1195`; §19-4 `:189` / §R R19-24 `:90` / D19-6 `:142` carry `deferred-work.md:1010` (→ `:931`, per the 19-7 creator).

## Kernel and kloc pricing — measured at creation

**Kernel:** Δ 0 by construction (`kernel_grant` above). `check-kernel-baseline` PASSED 26378 / 95 files; tokei 20619.

**E19 endpoint re-derivation (for D-19-1-01) — K2 prototype at `f34f6b4e`, reverted** (`creation/k2-prototype.diff`, `measurements.md`):

| Figure | Landed base @f34f6b4e | K2 prototype | Δ | §R / derived endpoint |
|---|---:|---:|---:|---:|
| kernel physical (`check-kernel-baseline`) | 26378 | 26389 | +11 | +12 → **26390** |
| kernel tokei (`kloc-check`) | 20619 | 20628 | +9 | +9 → **20628** |
| maos-bin (K2 literal sites) | 29497 | 29499 | +2 | in 19-3's 125 |
| aggregate measured / hardfail | 188504 / 188505 | 188515 | +11 | 188505 + 1541 + 9 = **190055** |

The conmon change in `t3/spawn.rs` (`invoke_bounded_command`, `launch_t3` init cwd) does not touch K2's sites (`T3SpawnContext` `:37-54`, `build_runtime_argv` call `:409-420`, `argv.rs:25-37,70-105`, `runtime.rs:257-268,570-581`). ADR-069: 20628 is 1,372 under the 22,000 alarm.

**kloc at `f34f6b4e`** (`kloc-check --json` exit 0, aggregate 188504, `over_budget` []). Forecast by component (tokei code; analogues measured at HEAD):

| Crate · component | Forecast | Basis |
|---|---:|---|
| maos-bin · `--epic` parse (`RunArgs.epic`, arm, `RUN_USAGE`, scene refusal) | **+9 measured** (≈ +10 formatted) | prototype 2, `creation/epic-parse-prototype.diff` |
| maos-bin · epic load, section ↔ entry binding and refusals (D-02, D-03) | 25–40 | `parse_run_args` 64 code lines for 4 flags |
| maos-bin · loop rework: goal per section, distillate write, remote follow-up, events | 40–60 | topology `[cli_wrapper]` arm `main.rs:4661-4961` = 190 code lines |
| maos-bin · `run_cli_wrapper_manifest` returns the Worker pid + stdout frame ids (D-07 A) | 10–20 | `:1211-1250` read-back already holds both |
| maos-bin · one Orchestrator construction: port, per-pass mint, context port, pid fixup, clone retention, three sites (D-04 A) | 50–80 | `researcher_root.rs` `attach`+`bind` 32 code lines; Mira pid fixup `main.rs:5036-5052` |
| maos-bin · pending-halt check before each assign (D-09 A) | 8–12 | one `drain_for_spirit_dry_run` call + event |
| **maos-bin Σ** | **+145–230** (central ≈ 190; epic 205) | envelope 29497 → 30563 |
| maos-bench · `Composition::Orchestrator`, class row, row type, `USAGE` | 25–35 | class rows 14–17 lines, `ButlerRow` 12 (`halt_eval.rs:85-115,356-368`) |
| maos-bench · per-row pass through a shared model-driven runner (D-13 A) / separate copy (B) | 25–35 / 60–75 | `butler_verdicts` 97 code lines (`:383-481`) |
| maos-bench · `deferred-work.md:1178` fix (D-15 A) | 3–8 | one root-relative join, two call sites |
| **maos-bench Σ** | **+55–75 (A) / +90–110 (B)** (epic 60) | envelope 2359 → 2476 (+117, shared with 19-3's 30) |
| xtask · `_19_1.rs` suffix (D-10 A), leg-1 epic sub-checks (D-11 A) | **+20–35** (epic 25) | leg 1's host check `:249-263` ≈ 13 code lines |
| maos-eval · maos-control · maos-cli · maos-shell · maos-journey-test · kernel | 0 | scorer generic (`maos_eval::halt_replay`); cassettes/corpora are data |

**Cumulative E19 envelope use after 19-1 (central):** maos-bin 29497 + 190 = 29687 of 30563 (190 of +1066); maos-bench 2359 + 65 = 2424 of 2476 (65 of +117 — leaves 52 for 19-3's 30); xtask 47422 + 28 = 47450 of 47624 (28 of +202); aggregate hardfail 188505 + ≈ 283 of 190055.

## Decisions — RULED A (all fifteen, operator Lunarpulse, 2026-10-28, via ask; recommendations adopted)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-19-1-01** | E19 kernel and aggregate endpoints (`epic-19:49`: derived, confirm at this preflight) | **A — confirm** 26390 physical / 20628 tokei / `_aggregate_hardfail` 190055: D19-2 A's ratified +12/+9 on the landed 26378/20619, D19-1's +1550 on the landed 188505; 19-3 re-pins its exact measured figure at or under them · **B — re-derive** to the HEAD prototype: 26389 / 20628 / 190055 · **C — defer** the kernel endpoints to 19-3's creation; fix only 190055 now | **A.** The K2 prototype at HEAD measures +11 physical / +9 tokei (table above); the tokei delta, which is what the aggregate counts, equals §R's, so 190055 = 188505 + 1541 + 9 holds exactly; the one-line physical gap is prototype shape (a doc line), so the ratified +12 is a ceiling 19-3 lands under. B tightens a ratified grant by one line for no gain; C leaves the aggregate arithmetic and the grant on different bases. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-02** | Goal source per arm (AC2) | **A —** loopback: the `--epic` sections are the goal; without `--epic` the rehearsal string stays (no consumer migrates); `MAOS_DELEGATED_GOAL` on the loopback arm refuses typed, naming `--epic` (registry doc `env_contract.rs:33-35`: cross-host only); `--epic` on the cross-host arm refuses typed before any emit · **B —** as A, but `MAOS_DELEGATED_GOAL` on loopback is ignored with one stderr warning · **C —** as the epic reads literally: delete the rehearsal fallback, so every host-bearing loopback run needs `--epic` (migrate `demo_j1.rs:544-545`, `journey_j1.rs:85,284,310`, `smoke_cli_wrapper_8_12.rs:164`, `drain_once_audit_writer.rs:40` now) | **A.** Five loopback consumers and demo-j1 need no goal and assert mechanism (`main.rs:4735-4738`); C front-runs 19-4's `TOPOLOGY` re-point and grows xtask outside this story's 25. The tracker's "removed on loopback arm" (`:287`) and the re-keyed sentinel mean the env stops being a loopback goal source; refusing (not ignoring, B) keeps the F7 class visible — a goal the operator set and the run silently dropped. Cross-host: the far Host runs the Worker and nothing returns (`main.rs:4844-4871`), so no local `TaskComplete` → no distillate → section k+1 cannot carry section k's ref. T0 confirms each `f4_pairing` / `root_shutdown_unload_16_3` vector runs the cross-host arm. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-03** | Section ↔ Worker entry (AC1) | **A —** strict 1:1 in file order: section k → the k-th host-bearing entry; a section count ≠ host-bearing entry count refuses typed before any emit · **B —** every section to the first host-bearing entry, admitted once per section · **C —** round-robin sections over the host-bearing entries | **A.** The epic fixes three sections and three entries; 19-3 AC3 asserts three distinct SCB ids (`supervision.rs:717,1926`), each pair closed before the next `task.assign`; Decision E gives the count to the split, never the cassette — B and C absorb the mismatch the AC1 proven-red needs to see. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-04** | Which Orchestrator drives the loop and carries the port (AC1, AC3; R19-P5 / D19-4 A) | **A —** one Orchestrator per load: the `orchestrator` entry's instance is built by ONE `orchestrator_root::construct` (port from `port_builder`, per-pass mint, `ContextBudgetPort` via the seam's binder, pid by fixup as Mira's `main.rs:5036-5052`); the root keeps a clone (shared `Arc<Mutex<DispatchState>>`) as the loop's emitter; `delegation_emitter` (`:4611-4612`) is deleted; the same `construct` serves the standalone arm (`:5826-5830`) and the successor factory (`:993-995`); a host-bearing entry before the `orchestrator` entry refuses typed · **B —** keep the root's separate emitter (no pid, no port, own in-flight state); bind the port on the scheduler-loaded instance only · **C —** keep the separate emitter and bind it to the admitted pid's port too (two instances, one identity) | **A.** 19-2 drains through `Orchestrator::drain_next` (`lib.rs:219-227`), whose safe-point gate reads `in_flight`; under B the loop's `begin/complete_delegation` (`main.rs:4784,4947`) never reach the instance 19-2 drains, so "never preempt an in-flight delegation" reads 0 in flight mid-delegation. The mint, the context port and the halt check (D-09) need the admitted pid, which only the loaded instance has. 18-1's one-construction rule (`butler/src/lib.rs:295-300`, D-18-1-R1): a successor without the port is the forbidden arithmetic fallback. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-05** | The Orchestrator's `provider.complete` list (AC3) | **A —** `["anthropic.claude-haiku-4-5-20251001", "openai.gpt-4o-mini"]` · **B —** the Researcher's pair `["anthropic.claude-sonnet-5-5", "openai.gpt-4o-mini"]` (`researcher/manifest.toml:49`) · **C —** the Butler's single id (`butler/manifest.toml:21`) | **A.** The judgement is one scalar against `dispatch_ambiguity > 0.5` (`orchestrator/manifest.toml:40-43`) — the Butler's tier; 17-11's ruling (`researcher/manifest.toml:46-48`) is why a second vendor is declared (an OpenAI-only box is not refused at issuance); the first declared provider keys the class cassette (`halt_eval.rs:392-398`). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-06** | The FR21 follow-up on the remote path (AC4) | **A —** NEW `Orchestrator::followup_dispatch_remote(…, prior: PriorDistillateRef, …, to_host, from_host, intent)` in `spirits/orchestrator` (ungoverned): non-optional `prior` + `assign_frame_remote`'s lineage and consent rules · **B —** the root composes `build_task_assign(…, Some(prior))` + `assign_frame_remote` (no new API) · **C —** widen `followup_dispatch` with an optional remote spec | **A.** `followup_dispatch` cannot reach a Worker (corrections table). A keeps "a well-behaved Orchestrator cannot emit a raw follow-up by construction" (`lib.rs:385-388`) on the path production uses, and gives the gate (D-11) a name to needle; B drops the typed split on exactly that path; C re-opens a signature with 12 local call sites (`lib.rs:305-307`). The epic's proven-red ("a follow-up built with `first_dispatch`", a local frame that would not route) becomes: the remote follow-up planted with `prior_distillate_ref: None`. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-07** | Distillate producer pid and write site (AC4; R19-P4) | **A —** the Worker's real pid: `run_cli_wrapper_manifest` returns the Worker's `spirit_pid` and its stdout `CliSubprocessOutput` frame ids with the verdict; the loop calls `write_distillate(worker_pid, …)` on the `--epic` path only, after a `Completed` verdict · **B —** the Worker's pid, written inside `run_cli_wrapper_manifest` for every completed delegated Worker · **C —** the root's Orchestrator pid (the struck §R rationale) | **A.** Measured: I11 citer authorization refuses any pid but the source rows' (`distillate.rs:348-377`) — C is refused unless an I11 control is weakened. B adds `Distillate` rows to the standalone arm (`main.rs:5326`) and host B (`delegation.rs:703`) that no AC asks for. A changes one return type: callers `main.rs:4894,5326`, `delegation.rs:703`, tests `two_host_delegation_2b.rs:801`, `worker_terminal_facts_17_7/unload_scenes.rs:205`, `cli_wrapper_exhaustion_17_9.rs:283` (the last two are archived — §CI below). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-08** | Distillate digest payload (AC4; R19-P12) | **A —** `DigestPayload::Json` of non-raw facts: `{section, heading_sha256, verdict, stdout_lines, stderr_lines, last_stdout_tl_ref}` · **B —** `DigestPayload::Text` of the Worker's last stdout line(s) · **C —** an LLM summary through the Orchestrator's port | **A.** FR21 is "always a distillate ref, never raw output" (`lib.rs:16-24`) — B is raw output under a `Distillate` kind; C breaks AC3 ("never for dispatch") and the deterministic unset path (D-D). A carries no free text, so 18-6's cascade reaches a 19-1 distillate only through its sources (`memory/mod.rs:546-555`). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-09** | Who owns "the loop consults the pending halt before each `task.assign`" (R19-P9) | **A — 19-1:** before each `task.assign` the loop calls `HaltRegistry::drain_for_spirit_dry_run(orchestrator_pid)` (`halt/mod.rs:389`); a pending halt stops dispatch — no further `task.assign`, a `dispatch_halted` event naming the halt id and the undispatched sections, normal drain; proven red with a test-planted pending halt; 19-4 adds the producer and the resume · **B — 19-4** (as the epic reads at 19-4 AC2); 19-1's loop has no check · **C —** 19-1 adds the pre-assign point only, no check | **A.** The loop body is this story's; rule 11(a) lets a control land before its producer when the producer's story is a named, not-done key (19-4); under B 19-4 re-opens this loop. Kernel-Δ 0 either way (public API). Exit code on a halted `--once` run is 19-4's to tighten with its beat; A prints the event and exits 0 [INFERENCE: a halt is a designed stop, as 18-4's scene halt]. If A is ruled, 19-4 AC2's sentence cites this check at its creation. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-10** | How 19-1's J1 tests enter the gate's derivation (AC2) | **A —** add `"_19_1.rs"` to `J1_TEST_SUFFIXES` (`check_j1_loopback_delegation.rs:121`): every `crates/maos-bin/tests/*_19_1.rs` must be enrolled in the job, by construction; D-2 unchanged (the stem before `_19_1.rs` has no decimal segment) · **B —** a named const like `CONSENT_REFUSAL_RS` (`:126`) — enrolled by memory · **C —** a digit-free lane suffix (e.g. `_j1e.rs`) | **A.** A story-numbered name on an existing suffix (`*_19_1a.rs`) is excluded by D-2 (`:795-807`; vector `planting_an_epic_numbered_foreign_lane_test_file_stays_green`, `j1_crosshost_1b_proven_red.rs:661-680`). A keeps the derivation's falsifier (`planting_an_unenrolled_j1_test_file_must_red`, `:620-634`; add a `_19_1` twin); 19-3 (`worker_sequence_19_3.rs`) and 19-4 reuse it. Consequence: name 19-1's non-J1 tests outside the suffix (D-14). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-11** | The gate leg for the epic topology (AC2) | **A —** extend leg 1 `frame-borne-route-intact` (`:242`) with two sub-checks: `j1-founder-loop-epic.toml` declares exactly three `host = "developer-remote-host"` entries; `main.rs` emits follow-ups through the typed remote builder (D-06); no new leg name; the four proven-red fixture trees (`j1_crosshost_{1a,1b,2a,2b}_proven_red.rs`, e.g. `lay_green` `1b:172-202`) lay a GOOD epic topology and the needle · **B —** a new leg `epic-dispatch-loop-intact` (+ `ledger_leg_names()`, pinned list `1b:793-800`, demo-j1 beat narration `demo_j1.rs:805-813`) · **C —** no gate leg; the maos-bin tests only | **A.** Beats are derived from `ledger_leg_names()` (`demo_j1.rs:232,843`): B adds a demo-j1 beat now, ahead of 19-4's beat plan, and costs more of the xtask +25; C drops AC2's gate clause (a Blocking static oracle over the topology the exit will run). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-12** | Who builds the Orchestrator's completion → scalar judgement (AC5 prerequisite) | **A — 19-1:** `Orchestrator::judge_ambiguity(section) → ("dispatch_ambiguity", value, derived_from)` over the deferred port, with a public prompt builder (hashed by the corpus, as 18-2 AC2); AC5 scores it; 19-4 calls it from the loop and writes it through `with_scalar_port` · **B — 19-4** builds it; AC5 moves to 19-4 (re-route under rule 10) | **A.** AC5 cannot score a class with no producer; the harness writes the scalar itself, as the Researcher arm does (`halt_eval.rs:279-283`), so 19-1 needs no scalar port. B re-routes a received AC a second time. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-13** | AC5 composition in `maos-bench` (kloc) | **A —** one model-driven runner shared by Butler and Orchestrator: router, `Counted`, consumed check and cassette facts factored out of `butler_verdicts` (`:383-481`), per-class pass as a closure · **B —** a separate `orchestrator_verdicts` copy | **A.** B ≈ +90–110 against an envelope of +117 that 19-3 also draws (+30); A ≈ +55–75 and one place where "consumed ≠ rows is refused" lives. | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-14** | Where the Orchestrator class gate runs in CI (AC5) | **A —** a workspace-suite test `crates/maos-bin/tests/eval_halt_orchestrator.rs` (own-build `workspace-test-suite`, Blocking via `aggregate`; no archive change) driving `maos eval halt --class orchestrator` through `CARGO_BIN_EXE_maos`: committed replay green, the miss twin exit 1, sidecar written · **B —** a step in `epic-18-exit` (debug-`maos` consumer, `discipline.yml:2494`; precedent `:2589-2640`) · **C —** defer the CI line to 19-4's epic job | **A.** Zero workflow edits; `eval_halt_18_2` costs Σ 1.37 s in CI (`creation/ci-timings-40609213235.tsv`) against the 30 s per-binary default (`test-budgets.toml:26`). B puts an Epic-19 claim in the Epic-18 exit job; C leaves the class gate un-run until 19-4 (rule 11(b)). | RULED A (2026-10-28, Lunarpulse, ask) |
| **D-19-1-15** | The `deferred-work.md:1178` fix | **A —** resolve the corpus and the committed cassette against the compile-time workspace root in `maos-bench` (`concat!(env!("CARGO_MANIFEST_DIR"), "/../..")`, the idiom of `main.rs:14638-14639`) · **B —** embed the corpus (`include_str!`) and the default cassette (`include_bytes!`), as the manifests are (`halt_eval.rs:96,113`); replay needs a bytes constructor beside `CassetteReplayProvider::from_file` (`main.rs:760`) · **C —** search upward from the cwd for the workspace `Cargo.toml` | **A.** The row's own fix ("resolve both against the workspace root"), ≈ 3–8 tokei, any cwd inside a checkout (CI restores binaries at identical paths, `proof-contract.md:110-112`). B grows maos-bin and the binary (corpora 16–18 KB, cassette 25 KB per class, `ls -la`); C guesses. Under A an installed binary outside a checkout still cannot find the corpus (Does NOT close). | RULED A (2026-10-28, Lunarpulse, ask) |

## Acceptance Criteria

*(Epic §19-1 as corrected at creation; AC1–AC5 kept (epic `:158`). Clauses marked (D-…) follow the recommended row; a different ruling rewrites only that clause.)*

### AC1 — command

1. `maos run spirits/topologies/j1-founder-loop-epic.toml --epic tests/fixtures/j1/epic.md --once` exits 0. For each section k = 1..3, in order: `delegation_routed` (`transport: loopback-rehearsal`, `goal` = section k's text), `topology_worker_admit`, `cli_wrapper_loaded` (three distinct `child_pid`), `cli_wrapper_exit`, `distillate_written` (`section`, `digest_frame_id`, `producer_pid` = that Worker's `spirit_pid`) and `delegation_completed`; three `task.assign` rows in the TL, frames 2 and 3 carrying `prior_distillate_ref`.
2. NEW `--epic <file>` in `parse_run_args` (`worker_spawn.rs:84-147`; `RunArgs` `:55-64`; `RUN_USAGE` `:75-76`), refused typed: without a value; with `--clock/--acceptance`; on a manifest with no host-bearing entry; on the cross-host arm (D-02). Parse vectors live in `crates/maos-bin/tests/` (in-`src` tests are kloc-charged).
3. NEW `tests/fixtures/j1/epic.md` (three `### Story` sections) and `spirits/topologies/j1-founder-loop-epic.toml` (orchestrator, architect, reviewer, then three `../worker/manifest.toml` entries with `host = "developer-remote-host"`). `j1-founder-loop.toml` is unchanged.
4. The split is deterministic, in `spirits/orchestrator` (Decision E: the count is the split's, never the cassette's); section k → the k-th host-bearing entry, a count mismatch refuses typed before any emit (D-03). Frames go out through `assign_frame_remote` (`main.rs:4770`; needle `check_j1_loopback_delegation.rs:271`) — the first via today's `build_task_assign`, the follow-ups via the typed remote builder (D-06). Workers run in sequence, each closed at a safe point.
5. Unset `MAOS_INFERENCE_MODE` is today's deterministic path (D-D); `demo-j1` gets no flag (`demo_j1.rs:544-552`).
6. **Proven red:** an epic whose second heading is not `### Story` → typed refusal naming 2 sections vs 3 entries, zero `task.assign` rows, exit non-zero.

### AC2 — goal source and the gate

1. Goal per D-02: on loopback the sections are the goal; without `--epic` the rehearsal string stays; `MAOS_DELEGATED_GOAL` on loopback refuses typed naming `--epic`; the cross-host REQUIRED arm (`main.rs:4744-4758`) is unchanged. The `env_contract.rs:32` row stays, its doc naming `--epic` as the loopback source.
2. `smoke_cli_wrapper_8_12.rs:261-299` is re-keyed to a one-section epic (`tests/fixtures/j1/epic-one-section.md`, the same non-ASCII sentinel): the `delegation_routed` goal equals the section byte for byte; the rehearsal constant is absent. Its test name and the file's other three tests are unchanged (the rootless canary selects `maos_run_cli_wrapper_worker_spawns_real_subprocess` from this binary, `scripts/run_cgroup_ceiling_17_2.py:288-289`).
3. `check-j1-loopback-delegation` (Blocking, `xtask/gate-registry.toml:465-467`; job `discipline.yml:3334-3420`; `v1-0-ship-gate` `:6156`; `aggregate` `:6398`) gains the epic sub-checks in leg 1 (D-11) and the `_19_1.rs` suffix (D-10); the job enrolls `cargo test -p maos-bin --test epic_dispatch_19_1 -- --test-threads=1`; the gate stays green.
4. **Proven red (gate):** a fourth `host` entry in `j1-founder-loop-epic.toml` reds `frame-borne-route-intact` (a proven-red vector in `j1_crosshost_1b_proven_red.rs`, run by the job at `:3354`); the un-enrolled `xyz_19_1.rs` twin of `:620-634` reds `completion-vectors-enrolled`.

### AC3 — the Orchestrator's inference port (D19-4 A)

1. `Orchestrator::with_deferred_inference_port(port, binding)` (the Researcher shape, `researcher/src/lib.rs:938`) and `with_context_port`; manifest `[capabilities.required] provider.complete` per D-05; the Decision-E comment (`manifest.toml:33-36`) amended: dispatch deterministic, ambiguity judged. Stays `rust-inproc` (`:7`).
2. Bound by one construction (D-04) from 18-1's `port_builder` (`main.rs:3912-3937`) with a per-pass mint (`seam.mint`) and an 18-5 `ContextBudgetPort` from the seam's binder (`butler_root.rs:165-192`; window from `[budget] context_window_size = 8192`, `manifest.toml:18-20`), at the topology arm, the standalone arm and the successor factory. No new `InferencePortAdapter` (`check-composition-root-completeness` reds a second `Arc::new(<Adapter>::new(…))`).
3. Observable: unset mode leaves it unbound (ADR-064); an explicit mode binds it and prints `orchestrator inference seam wired (provider <id>)`. The port serves only the judgement (D-12), never dispatch. `spirit_smoke.rs:73-74,104-108` assert the declared capability instead of its absence.

### AC4 — FR21 in production

1. After each `Completed` Worker on the `--epic` path the loop writes a distillate at the Worker's pid over its stdout `CliSubprocessOutput` rows (D-07), payload per D-08, through `DistillateWriter` (the only sanctioned inserter, `distillate.rs:404-416`); section k+1 goes out with the typed remote follow-up carrying that `PriorDistillateRef` (D-06); the FR21 gate (`orchestrator_dispatch.rs:63-146`) passes on the real rows.
2. Pending halt (D-09 A): before each `task.assign` the loop checks the Orchestrator's pid; a pending halt stops dispatch with `dispatch_halted`.
3. **Proven red (two controls, named under `proof-contract.md:28`):** (a) the follow-up planted with `prior_distillate_ref: None` → `EOrchestratorDispatchRawOutput`, no `task.assign` row for it, exit non-zero; (b) a pending halt planted for the Orchestrator's pid before section 2 → one `task.assign`, `dispatch_halted` naming sections 2–3.

### AC5 — Orchestrator class halt corpus and gate (received from 18-2, D-18-2-05 A)

1. `halt_eval.rs` gains `Composition::Orchestrator` and an `orchestrator` row (D-13), `USAGE` (`:118`) widened; floors `halt_recall_overall` ≥ `NFR_TEST_4_RECALL_FLOOR` 0.70 and precision ≥ `HALT_PRECISION_FLOOR` 0.85 (`onboarding_gate_corpus.rs:62,65`), `model_driven: true`. Each row replays one completion through the kernel `InferencePortAdapter` with a token minted for the first declared provider, the judgement of D-12, and `process_scalar_write` under the Orchestrator's `[epistemic_policy]`; consumed ≠ rows is refused.
2. NEW corpus `spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.jsonl` (30 rows, as both shipped classes) with a SHA pin test, registered in `tests/corpora/MANIFEST.toml` (+ symlink, the `:136-150` pattern) and in the `NFR-Test-4` row of `tests/coverage-matrix.yaml:1926-1945`; NEW cassette `crates/maos-journey-test/cassettes/j1/orchestrator-halt-corpus-v0.1.json` (`provenance: seed`, real prompt hashes, no wildcard — 18-2 AC2's tests).
3. FR15 on an Orchestrator halt: a witness in `spirits/orchestrator/tests/` resolves one halt once per kind (`provided-context`, `accepted-halt`, `authorized-override`) through `tests/harness/fr15_resolution.rs` (`resolve_each_kind` `:65`), each journaled with its reasoning chain.
4. `maos eval halt --class orchestrator` runs from any cwd inside the checkout (D-15); `deferred-work.md:1178` is closed with this story's key. CI per D-14.
5. **Proven red:** the miss twin (planted rows' completions judged clear) exits 1 naming the missed floor — the standing negative control, as 18-2's drift-blind twin. Kernel-Δ 0.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-19-1-01…15 RULED; tracker → `in-progress`. Re-measure any cite a ruling moves. Confirm each `f4_pairing.rs` / `root_shutdown_unload_16_3.rs` vector's arm (D-02) and that topology admission takes a `provider.complete` Orchestrator (the class refusal at `main.rs:5068-5073` is by kind).
- [ ] **T1 — `spirits/orchestrator`** (AC1, AC3, AC4, AC5; ungoverned): `split_epic`; `with_deferred_inference_port`, `with_context_port`, binding type; `followup_dispatch_remote` (D-06); `judge_ambiguity` + prompt builder (D-12); manifest (D-05) and comment; `spirit_smoke.rs` flips; unit tests for split, follow-up and judgement.
- [ ] **T2 — maos-bin** (AC1–AC4): `--epic` parse; epic load and binding (D-02, D-03); `orchestrator_root.rs` construction at the three sites (D-04); loop rework (goal, distillate D-07/D-08, follow-up, halt check D-09); `run_cli_wrapper_manifest` return; `env_contract.rs` doc. Tests: NEW `crates/maos-bin/tests/epic_dispatch_19_1.rs` (T3, `require_t3`, fresh `MAOS_HOME`); parse vectors in `crates/maos-bin/tests/`; the sentinel re-key.
- [ ] **T3 — xtask gate** (AC2): suffix (D-10), leg-1 sub-checks (D-11); the four proven-red fixture trees; the two new vectors; the job step in `discipline.yml:3334-3420`.
- [ ] **T4 — AC5:** `halt_eval.rs` (D-13, D-15); corpus, pin test, MANIFEST + symlink, coverage-matrix; seed cassette; FR15 witness; `eval_halt_orchestrator.rs` (D-14); `deferred-work.md:1178` closed.
- [ ] **T5 — Plants** (rule 11(c); `proof-contract.md:24-31`): AC1 heading, AC2 gate (×2 vectors in-tree), AC4 (a)+(b), AC5 miss twin; in place, restored byte-identical, per-plant wall in the record. T3-bound plants run in CI once (`ci/19-1-verify`) when this host lacks the substrate; the run id is recorded.
- [ ] **T6 — Landing.** `cargo fmt --all`; `kloc-check --json`; exact re-pins (maos-bin `:517`, maos-bench `:520`, xtask `:339`, `_aggregate_hardfail` `:718`) inside the envelope, cumulative use in the record; `check-kernel-baseline` PASSED 26378; `cargo run -q --locked -p xtask -- prepush` exit 0 with wall before the push (20-16); `d11_xtask_ceiling_ratchet` + `recovery_lane_ceiling_rule` before the closing commit (its xtask line cites this full key). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Orchestrator | `spirits/orchestrator/src/lib.rs`, `manifest.toml`, `tests/spirit_smoke.rs`, new tests + `tests/fixtures/halt-corpus-v0.1.jsonl` | bare, no capability → split, port, remote follow-up, judgement |
| Root | `crates/maos-bin/src/worker_spawn.rs:55-147,563-577`, `main.rs:993-995,4603-4961,5826-5830`, new `crates/maos-bin/src/orchestrator_root.rs` (+ `lib.rs` module), `env_contract.rs:33-35` | one delegation → epic loop with FR21 distillates |
| Topology / fixtures | new `spirits/topologies/j1-founder-loop-epic.toml`, `tests/fixtures/j1/epic.md`, `epic-one-section.md`, the planted-heading fixture | — |
| Gate | `xtask/src/check_j1_loopback_delegation.rs:121,242-…`; `xtask/tests/j1_crosshost_{1a,1b,2a,2b}_proven_red.rs`; `.github/workflows/discipline.yml:3334-3420` | one host → plus the epic topology |
| Eval | `crates/maos-bench/src/halt_eval.rs`; new `crates/maos-bin/tests/eval_halt_orchestrator.rs`; `tests/corpora/MANIFEST.toml`; `tests/coverage-matrix.yaml:1926-1945`; new cassette under `crates/maos-journey-test/cassettes/j1/` | two classes → three |
| Kloc | `xtask/kloc.toml:339,517,520,718` | exact re-pins inside the envelope |
| Records | `deferred-work.md:1178`; this file; `sprint-status.yaml:287` | — |

### Current state that must be preserved

- `j1-founder-loop.toml` and every no-`--epic` run, byte for byte (demo-j1, `journey_j1`, `smoke_cli_wrapper_8_12`, `drain_once_audit_writer`) — under D-02 A.
- The cross-host arm: REQUIRED goal, no local Worker, `topology_worker_delegated_offhost` (`main.rs:4858-4871`).
- `run_cli_wrapper_manifest` stays synchronous under `block_in_place` (`main.rs:4893-4913`); the verdict is enforced for every `[cli_wrapper]` entry (`:4936-4942`); teardown on signal (`:4919-4925`).
- The FR21 gate, the I11 citer check and the 60 s proxy are consumed, not modified (maos-iac).
- `live_agent_gate` and `ci_local_split_refuses_a_granted_real_agent_without_the_live_flag` (`smoke_cli_wrapper_8_12.rs:197`) unchanged; CI never sets `MAOS_LIVE_AGENT`.
- The J1 gate's leg names and demo-j1 beat set (D-11 A); `unlanded_beats()` (`demo_j1.rs:890-908`) unchanged — 19-4 changes it.
- 18-2's Butler and Researcher numbers and sidecars unchanged; `maos eval halt --class butler` stays exit line 2 of `epic-18-exit`.

### Guardrails

- Never weaken I11 (citer authorization) or FR21 to make the loop pass; a refusal is a finding.
- No new `InferencePortAdapter`; one Orchestrator construction for all three sites (D-04); the port never decides dispatch.
- Distillates only through `DistillateWriter`; no raw output in the payload (D-08).
- `--test-threads=1` for every `-p maos-bin` J1 step (`MAOS_HOME` is process-global, `discipline.yml:3390-3391`); fresh `MAOS_HOME` per test (the 60 s proxy).
- No test spawns cargo (`proof-contract.md:85-90`); fixtures come from `cargo run -p xtask -- build-test-fixtures` (the job builds `worker-cli-fixture`, `discipline.yml:3355-3356`).
- One cargo process; `env -u CI -u GITHUB_ACTIONS`; `CARGO_TARGET_DIR=/mnt/build/cargo`; plants byte-restored.

### CI and the 20-15 build-once contract

- **No new archive family or consumer pin.** The new targets run only in own-build jobs: `epic_dispatch_19_1` in `check-j1-loopback-delegation` (compiles itself, `discipline.yml:3338-3348`; T3 `:3346`) and in `workspace-test-suite` (`:5863`, T3 `:5906`); `eval_halt_orchestrator` in `workspace-test-suite`; the orchestrator tests in `orchestrator-tests` (`:1097-1116`, `cargo test -p orchestrator`) and the workspace suite; the xtask vectors in `check-j1-loopback-delegation` (`:3349-3354`). The 20-15 consumers are `wasm-host-tests`, `cgroup-ceiling-*`, `epic-17-exit` (archives) and `epic-18-exit` (debug binaries) (`proof-contract.md:126-127`); none gains a test.
- **Archived sources this story edits, pins unchanged.** `smoke_cli_wrapper_8_12` is in family `maos-bin-network-wasm` (`ci-test-producer` `:2019-2023`) and its `maos_run_cli_wrapper_worker_spawns_real_subprocess` is the rootless canary (`run_cgroup_ceiling_17_2.py:288-289`; bound by `ci_shape_18_0.rs:1395`) — AC2 re-keys a different test in that binary; no name or count moves. Under D-07 A `worker_terminal_facts_17_7` (`unload_scenes.rs:205`) and `cli_wrapper_exhaustion_17_9` (`:283`) — family `maos-bin-network` (`:2024-2027`), count-pinned by `--expected-count "${EPIC17_TERMINAL_SUITE_TESTS}"` (`:2257`) — change their call shape only; test counts and names stay. 20-16's pin reader (`prepush`) checks those count pins locally.
- Test cost: a one-Worker T3 founder-loop run costs ≈ 1.2 s in CI; `epic_dispatch_19_1` (one three-Worker run + three planted runs) ≈ 5–8 s [INFERENCE from `creation/ci-timings-40609213235.tsv`], under the 30 s default — no `[[exception]]`; suite Σ 564.1 s of 800 s.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Deterministic split (AC1) | second heading `### Notes` | typed refusal, 0 `task.assign`, exit ≠ 0 |
| J1 gate, epic topology (AC2) | a fourth `host` entry | `FAIL frame-borne-route-intact` |
| J1 gate, enrolment (AC2) | un-enrolled `xyz_19_1.rs` | `FAIL completion-vectors-enrolled` |
| FR21 gate (AC4a) | remote follow-up with `prior_distillate_ref: None` | `EOrchestratorDispatchRawOutput`, exit ≠ 0 |
| Pending-halt check (AC4b, D-09 A) | pending halt for the Orchestrator pid before section 2 | one `task.assign`, `dispatch_halted` |
| Class gate (AC5) | miss-twin cassette | exit 1 naming the missed floor |

### Downstream — demo-j1, journeys and siblings

- **demo-j1:** unchanged by this story under D-02 A / D-11 A (`TOPOLOGY` `demo_j1.rs:48`, beats from `ledger_leg_names()`); 19-4 re-points it to `j1-founder-loop-epic.toml` and adds `epic-decomposed-n-frames` (fed by AC1's events). Under explicit `MAOS_INFERENCE_MODE` the Orchestrator now binds a port; nothing consumes it until 19-4, so the exit's cassette (19-4 AC4) needs Orchestrator turns only from 19-4 on.
- **Journeys:** `journey_j1.rs` (job `journey-hermetic-tier-1`) runs `j1-founder-loop.toml` and is unchanged; `maos_run_boot_loud_8_11.rs:93` runs the Orchestrator standalone (focused set: the manifest change).
- **19-2** drains the same Orchestrator instance the loop drives (D-04 A); **19-3** adds `_19_3.rs` under the same suffix rule and runs the three Workers in this loop; **19-4** writes `dispatch_ambiguity` from `judge_ambiguity` through `with_scalar_port` and resumes after the halt this loop stops on (D-09 A); **19-6** delivers a Worker crash to this loop's Orchestrator.

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo build -p maos-bin --bin maos && cargo run -p xtask -- build-test-fixtures --only worker-cli-fixture
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test epic_dispatch_19_1 --test smoke_cli_wrapper_8_12 --test eval_halt_orchestrator --test topology_delegation_1a --test maos_run_boot_loud_8_11 -j 1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p orchestrator
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bench
env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask --test j1_crosshost_1a_proven_red --test j1_crosshost_1b_proven_red --test j1_crosshost_2a_proven_red --test j1_crosshost_2b_proven_red -- --test-threads=1
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants   # manifest change widens the focused set
cargo run -q --locked -p xtask -- check-j1-loopback-delegation --json
cargo run -q --locked -p xtask -- check-corpus && cargo run -q --locked -p xtask -- check-composition-root-completeness
cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-test-cost
cargo run -q --locked -p xtask -- prepush   # before every push (20-16)
```

### Review plan (`proof-contract.md:33-75`)

- DIFFICULT: separate Blind and Edge (opus-5.5) — Edge walks the arm matrix of D-02 (loopback ± `--epic` ± env; cross-host ± `--epic`), the count-mismatch and signal-between-sections paths, and the FR21 window across sections; Acceptance on opus-5.5; Test Infrastructure on glm-5.3 re-reads the job enrolment and the archived sources' pins.
- Focused set: `orchestrator`, `maos-bin` (the targets above + every reader of the Orchestrator manifest: `topology_delegation_1a`, `maos_run_boot_loud_8_11`, `journey_j1`, `demo_j1_tests`, the four proven-red files), `maos-bench`, `spirit_boundary_invariants`, `check-corpus`.

### Previous-story intelligence

- **20-16:** `prepush` is required before every push; its pin reader covers the count pins this story's archived edits sit under.
- **20-15:** archive families and consumer pins; a moved test name or count is a red consumer.
- **18-2:** the class table, the model-driven arm, seed cassettes with real prompt hashes, the drift-blind twin as the standing negative control, `fr15_resolution.rs`, the sidecar; BH-6 → `deferred-work.md:1178`.
- **18-3 / 18-1:** one construction path for every instance (D-18-3-04 A, D-18-1-R1); `port_builder`; per-pass mint.
- **18-5:** the context binder resolves the window from the admitted pid's bundle.
- **18-6:** forget scrubs distillate bodies and sources; orphan re-distillation requests are recovered.
- **j1-crosshost-1a/1b/2a/2b, 15-1:** the gate's static legs, derivation by suffix, D-2 digit exclusion, `--test-threads=1`.

### Git intelligence

| Commit | Content |
|---|---|
| `f34f6b4e` | 20-15, 20-16 done (records only; code tree = `b35c9284`) |
| `b35c9284` | landing green 40609213235 (191/191) |
| `ed2118a0` | conmon FLAG-Winston: kernel 26367 → 26378 / 20610 → 20619 (`t3/spawn.rs`) |
| `84e0e659` | Epic 18 closed; Epic-19 §R ratified (D19-1…D19-6) |
| `f889b3e4` | the §R preflight's measured tree |

### References

- Epic `epic-19-founder-loop-w4.md:34-55,99-107,137-142,150,158,160-168,195,211`; tracker `sprint-status.yaml:287-293`; `deferred-work.md:1178`.
- Code: `crates/maos-bin/src/worker_spawn.rs:55-147,563-577,1084-1092,1161-1250`; `main.rs:993-995,2583-2586,2658-2661,3912-3937,4603-5114,5826-5830,13992,14638-14639`; `delegation.rs:248-296,374-392`; `env_contract.rs:32-35`; `researcher_root.rs:179-212`; `butler_root.rs:165-204`; `spirits/orchestrator/src/lib.rs:16-24,133-139,219-227,252-401`; `manifest.toml:7,18-20,33-48`; `maos-iac/src/adapter/distillate.rs:309-428`; `orchestrator_dispatch.rs:63-146`; `halt/mod.rs:389`; `hook_dispatch.rs:43-44,281-285`; `memory/mod.rs:437-606`; `halt_eval.rs:60-118,224-300,383-481`; `check_j1_loopback_delegation.rs:121,145,242-293,772-907`; `demo_j1.rs:48,232,544-552,805-843,890-908`.
- CI: `discipline.yml:1097-1116,1988-2039,2245-2260,2494,3334-3420,5863,5906,6156,6398`; `scripts/run_cgroup_ceiling_17_2.py:283-301`; `xtask/test-budgets.toml:26-27`; `docs/testing/proof-contract.md:24-31,33-75,76-97,99-142,144-180,182-187`.

## Dev Agent Record

### Story context creation

Created 2026-10-28 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `f34f6b4e`, clean tree.

**Inputs:** direct reads of every cited range; `check-kernel-baseline` PASSED 26378 / 95; `kloc-check --json` exit 0 (aggregate 188504); K2 prototype (+11 / +9; maos-bin +2) and `--epic` parse prototype (maos-bin +9), both reverted with gates re-PASSED (`creation/measurements.md`, `k2-prototype.diff`, `epic-parse-prototype.diff`, `kloc-check-runs.json`); per-test CI cost from the landing run's workspace junit (`creation/ci-timings-40609213235.tsv`).

**Revert:** five files per prototype restored with `git checkout --`; `git status --short` shows only a sibling's untracked `19-7-…-evidence/`; no crate source changed; no workspace test run, so `intent-lineage-coverage-report.md` untouched.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

### Completion Notes List

### File List

- `_bmad-output/implementation-artifacts/19-1-orchestrator-dispatch-loop-and-epic-input.md`
- `_bmad-output/implementation-artifacts/19-1-orchestrator-dispatch-loop-and-epic-input-evidence/creation/{measurements.md, k2-prototype.diff, epic-parse-prototype.diff, kloc-check-runs.json, ci-timings-40609213235.tsv}`
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (§19-1 `:164-166,168` cite corrections)

### Change Log

- 2026-10-28: Story created at `f34f6b4e` (backlog; ready-for-dev after the operator rules D-19-1-01…15). Four §19-1 epic cites corrected in place; K2 re-measured for the E19 endpoints (D-19-1-01); `--epic` parse priced by prototype; CI per-test cost read from run 40609213235.
- 2026-10-28: Operator ruled D-19-1-01 … D-19-1-15 all **A** (as recommended; Lunarpulse, via ask). D-19-1-01 A confirms the E19 endpoints: kernel 26390 physical / 20628 tokei (D19-2 grant +12/+9 over the landed 26378/20619; the 19-3 prototype re-measured +11/+9 at f34f6b4e), `_aggregate_hardfail` 190055 (188505 + 1550). Status backlog → ready-for-dev; tracker `sprint-status.yaml:287` flipped and epic-19 → in-progress.
- 2026-10-28: Development started (status ready-for-dev → in-progress; tracker `sprint-status.yaml:287` flipped; parallel units, candidate pins applied; reviews pending).
