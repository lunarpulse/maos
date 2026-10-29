---
story_id: '19.4'
story_key: 19-4-j1-beats-and-demo-replay
created: 2026-10-28
context_commit: 61af7461
baseline_commit: 61af7461
security_sensitive: true
kernel_grant: "none under the recommended rows — kernel-Δ 0 by construction: the halt is raised by the existing `process_scalar_write` → `invoke_halt` (`crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs:53-95`, `halt/mod.rs:530-592`), resolved by the existing `KernelHaltResolver` (`halt/resolver.rs:116-200`) and read by the existing `drain_for_spirit_dry_run` (`halt/mod.rs:389`). The one kernel-touching option, D-19-4-05 C (cross-boot halt rehydration), was priced by throwaway prototype on a `/tmp` copy of HEAD's kernel src (never the tree; `creation/k1-halt-rehydrate-prototype.diff`): **+44 physical / +40 tokei** (26378 → 26422; 20619 → 20659) — with 19-3's +11/+9 and 19-2's replicated +74/+46 it BREACHES the E19 kernel endpoint (≤ 26483 / ≤ 20686) and would need its own FLAG-Winston grant."
kloc_grant: "RULED 2026-10-28 (ask `w3_kloc`, \"Raise to cover forecasts ×1.3\"; supersedes D-19-4-12): E19 envelope maos-bin 31353 → **31710** (+357), xtask 47624 → **47688** (+64), aggregate hardfail 191248 → **191669** (+421) (`epics/epic-19-founder-loop-w4.md:40,41,48,138`). Basis: measured by scratch prototype (tokei 14.0.0, rustfmt'd, uncompiled; `creation/p1-maos-bin-recommended-bundle.diff`): maos-bin **+184** under the ruled rows (epic forecast 100); xtask ≈ +147…+221 [INFERENCE, analogue table in `creation/measurements.md`]; kernel, maos-control, maos-cli, maos-shell, maos-bench, maos-corpus-gen 0; `spirits/` and `tests/` uncharged (`xtask/src/kloc_check.rs:303,311`). Room at the working tree (61af7461 + uncommitted 19-3/19-6) before the raise: maos-bin 31353 − 30847 = 506 (19-2 196–279 and 19-5 195 still owed); xtask 170; aggregate 892. Exact re-pin at landing; STOP and a new D-row if the cumulative E19 use would exceed the raised endpoints."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`) — concurrency (the `--epic` loop holds on a pending halt while the operator door resolves it on another task; `demo-j1` drives the resolution from a reader thread over the child's stdout) and a control (rule 11): the Orchestrator's ambiguity halt becomes a production halt and `check-j1-loopback-delegation`'s enrolment derivation widens. No kernel grant under the recommended rows (D-19-4-05 C adds FLAG-Winston). Separate Blind (opus-5.5) and Edge (opus-5.5) layers, Acceptance on opus-5.5, Test Infrastructure on glm-5.3; creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol (never reviews its own code)."
---

# Story 19.4: J1 beats and the hermetic `demo-j1` replay

Status: ready-for-dev (RULED 2026-10-28)

<!-- Created 2026-10-28 at 61af7461 (19-1 and 19-7 done, landing 40687349154; 19-3 and 19-6 code written and uncommitted in the working tree; 19-2 ruled ready-for-dev) by bmad-create-story under the lighter creation contract, in parallel with 19-5 (wave 3). D-19-4-01…12 are PROPOSED, not ruled — the operator rules. The tracker row `sprint-status.yaml:290`, the epic and the 15-1 record are NOT edited here: every correction is filed in `19-4-…-evidence/creation/records-owed.md`. No cargo ran (parent rule: a build held the host): cites re-read at HEAD (`git archive` copy in /tmp) with the working-tree shift noted; kloc by the kloc-check instrument replicated with tokei 14.0.0 on HEAD and on the tree; kernel physical by `wc -l` over the git-tracked kernel src; the one kernel-touching option and the recommended maos-bin bundle priced on /tmp copies; gate/cargo confirmations owed (`creation/measurements.md`). -->
<!-- RULED 2026-10-28 (Lunarpulse): D-19-4-01…11 as recommended (ask `w3_19_4`: 01 A, 02 A, 03 B, 04 A, 05 A, 06 A, 07 A, 08 A, 09 A, 10 A, 11 A); D-19-4-12 superseded by ask `w3_kloc` ("Raise to cover forecasts ×1.3": maos-bin 31710, xtask 47688, aggregate 191669). Records owed R1 (epic cites), R2 (envelope rows + D19-1 note), R3 (tracker `:290` → ready-for-dev) applied by the creator the same day; R4 (15-1 annotation) not applied (optional; a done record); R5 is the dev pass's at landing. -->

> **The capability:** *one unattended, secret-free command plays the whole founder loop: the Orchestrator splits a three-story epic, judges each story before handing it to a Worker, halts on the ambiguous one through the kernel — not through a string in the demo — waits while an operator answers over the door, resumes with the answer, finishes the epic, and ends with an FR17 digest whose every cited completion resolves to a Transparency-Log row and which still names the halt it came through. `demo-j1` renders that as executed beats; the beats only an operator, two hosts or a paid key can earn stay visibly unclaimed.*

**Closes:**
- Epic-19 §19-4 AC1–AC5 and the received AC5(b) (`epics/epic-19-founder-loop-w4.md:195-208`; stories row `:154`), renumbered here AC1–AC6 (the epic's second "AC5" is AC6); the epic's hermetic exit command (`:13-22`) — its owner is this story.
- J1's `halt-resume-referential-identity` beat (`xtask/src/demo_j1.rs:902-906`, owner `FOLLOWUP-J1-RESUME-SEAM`; runbook `_bmad-output/test-artifacts/runbook-j1-demo.md:64`) and the deferred oracle `crates/maos-journey-test/tests/journey_j1.rs:363-368`.
- FR17 on the J1 path (the per-run digest with source refs; `_bmad-output/planning-artifacts/prd/functional-requirements.md:51`) — NOT its 100-digest corpus floor.
- The 15-1 cut line "`_2c.rs` J1 blind spot" (`15-1-green-at-head.md:181,722`; Epic-15 retro re-route) — closed, narrowed or delegated per **D-19-4-11**.
- The tracker precondition (`sprint-status.yaml:290`; `deferred-work.md:931`, was `:1010`) is **already satisfied** by `19-7-repin-consent-operator-surface` (done 2026-10-28): f4_pairing V4 `restart_refusal_is_repinned_by_operator_consent_then_delivered` (`crates/maos-bin/tests/f4_pairing.rs:1174`), proven red 40671241869, landing 40687349154 @ `309e277a`; `:931` closed. Recorded, not re-done.

**Does NOT close, and says so:**
- `tier2-live-agent-signed` (operator row `ops-j1-live-agent-signed-take`, `sprint-status.yaml:337`) and `two-host-signed-run` (the closed `j1-crosshost-2d` lane; renders INDETERMINATE) — never demanded by the exit.
- FR17's 100-digest corpus floor; the J3 `maos_digest::derive_team_digest` path (`crates/maos-bin/src/main.rs:500`), a different artifact.
- Cross-boot resume (a halt pending when the process exits is not re-pended by the next boot) under D-19-4-05 A; a writer for `delegation_confidence` under D-19-4-03 B.
- The 19-3 live take and its claude topology (19-3 AC1.2), FR20/FR51 (19-2), crash disposition (19-6).

## Story

As **the founder watching J1 and the operator who answers its halts**,
I want **`demo-j1` to replay the whole epic hermetically — decompose, judge, halt on the ambiguous story, resume with my answer, finish, digest —**
so that **every J1 beat a machine can prove is proven by an executed run, and the digest I read in the morning cites rows that exist, including the halt the run went through.**

**Contract:**
- Epic-19 §19-4 (`epics/epic-19-founder-loop-w4.md:195-208` after records-owed R1); exit block `:13-28`; `:159` — create-story may tighten, not add uncited ACs: AC1–AC5 kept, AC6 is the epic's second AC5.
- §R R19-19…R19-24 (`:86-91`), R19-P1/P2/P7/P8/P9 (`:97-105`), D19-3 A (`:140`), D19-5 A (`:142`), D19-6 B (`:143`), E19 kloc envelope (`:38-48`), Kernel-Δ (`:34`).
- Tracker `sprint-status.yaml:290`; proof contract `docs/testing/proof-contract.md:24-31` (one plant per AC), `:33-75` (review, focused set, tiers).

## Current-tree corrections — read before implementing

Measured at HEAD `61af7461`; the working tree carries uncommitted 19-3/19-6 code in `xtask/src/demo_j1.rs`, `check_j1_loopback_delegation.rs`, `crates/maos-bin/src/main.rs`, `orchestrator_root.rs`, `env_contract.rs`, `spirits/orchestrator/src/lib.rs`, `discipline.yml` — re-measure those cites at T0. Full OLD → NEW table: `creation/cites-61af7461.md`.

| Premise (epic / tracker) | Fact at `61af7461` and consequence |
|---|---|
| AC2: "the Orchestrator is loaded bare today (`main.rs:4997-5001`)" | **False.** 19-1 builds every Orchestrator through `orchestrator_root::construct` (`orchestrator_root.rs:41-59`) / `construct_at_admitted_pid` (`:65-83`) — topology arm `main.rs:5165-5184`, standalone `:6010-6023`, successor factory `:997-1015` — with a context port and (under an explicit mode) the Inference Port, but **no scalar port**. `judge_ambiguity` (`spirits/orchestrator/src/lib.rs:396-445`) has **zero production callers**. → D-19-4-01, D-19-4-02. |
| R19-P9: "the loop itself must consult the pending halt before each `task.assign`" | **Done by 19-1** (D-19-1-09 A): `dispatch_stop` (`orchestrator_root.rs:201-225`) is called before each section's frame (`main.rs:4868-4878`) and stops dispatch (`epic_halted`, `:4876`); a debug plant raises a real halt (`plant_test_halt` `:230-259`, `MAOS_TEST_ONLY_HALT_BEFORE_SECTION`). What is missing is the **judgement** that raises the halt and the **resume** after it. |
| AC2: "journaled under ONE ambiguity tag repo-wide, `task.acceptance_criterion.ambiguous`" | The scalar path journals `payload.tag` = the **scalar** tag (`policy_runtime.rs:134-141`; TL intent `halt/mod.rs:554-561`) — the Orchestrator's halt is `dispatch_ambiguity`, whatever the seeds say. The re-spell also misses the committed corpus: 24 occurrences in `tests/corpora/lcas-v0.3.jsonl` (sha-pinned `tests/corpora/MANIFEST.toml:45-46`; `lcas.rs:25` "bump on any seed change"). → D-19-4-08. |
| AC2: admit `dispatch_ambiguity` **and** `delegation_confidence` | `delegation_confidence` has **no writer** anywhere (`git grep` = manifest `:55-58` + one test comment). After the flip no shipped manifest keeps a Halt rule outside the port set, so the over-fire guard `maos_run_boot_loud_8_11.rs:86-120` (asserts the Orchestrator's `boot_loud_port == false`) loses its subject. → D-19-4-03. |
| AC3: "every `completions[].source_log_ref`" | The digest field is `completed[]` (`MorningDigest`, `spirits/butler/src/lib.rs:706-715`); halts are `open_halts[]` (`:688`). The renderer lists **every** `EpistemicHalt` row in its window as open (`digest_over` `:1740-1751`) — a halt resolved mid-run is still "open" in the post-resume digest (resolution is an approval decision, not a frame: `halt/mod.rs:51-80`). → D-19-4-07. |
| AC3: render at "topology drain" | The topology `--once` drain (`main.rs:5298-5459`) renders no digest; the renderer exists on the root's seam (`main.rs:4015-4032`) and `render_scene_day(pid, boot_nonce)` (`butler/src/lib.rs:438-444`) reads only this boot's frames. → D-19-4-06. |
| AC4: "the CI job sets `MAOS_REPLAY_STRICT=1`" | `journey_j1`'s child is `env_clear`ed (`production_command` `journey_j1.rs:28-42`): a job-level variable never reaches it — the test sets it on the child. It does reach `demo-j1`'s child (no `env_clear`, `demo_j1.rs:544-552`), so the exit job sets it there. |
| AC4: `cassettes/` = `j-butler j-researcher j0` | `cassettes/j1/` exists (19-1 AC5: `orchestrator-halt-corpus-v0.1.json`, schema `maos.journey.cassette/v1`, `provenance: seed`, real prompt hashes); replay is **sequential** (`cassette_replay.rs:135-166`; strict = hash drift is fatal) — entry order = judgement order. |
| AC5: "resume into the same `MAOS_HOME`", `post_resume_digest.cited_refs`, `TrajectoryRef` | `HaltRegistry` is in-process only (`halt/mod.rs:100-112`; no rehydration): a halt pending at exit is gone on the next boot. `TrajectoryRef`/`cited_refs` exist nowhere; the concrete referent is the halt's TL frame id (`HaltReceipt.frame_id`, `halt/mod.rs:562,585-591`). The door is served during a `--once` topology load (bound before it, `main.rs:3530-3596`; `is_door_root` covers `maos run`, `:3412-3414`), so an **in-boot** resolution is reachable; `provided-context` lands in the Spirit's private tier `halt_context::<halt_id>` (`halt/resolver.rs:165-181`). → D-19-4-05. |
| AC1: "`unlanded_beats()` shrinks to one (the tier-2 beat)" | `two-host-signed-run` must stay declared: `apply_capture_judgement` (`demo_j1.rs:958-984`) re-states it in place and `demo_j1_tests.rs:567-626` pin it. `unlanded_beats` keeps **two**. |
| AC1: `demo-j1` beats on the epic topology | Every hermetic beat assumes ONE Worker: P2–P5 read the first event (`demo_j1.rs:591-690`), `lifecycle-stages-in-order` demands each stage **exactly once** (`:729-767`). Re-pointing `TOPOLOGY` (`:48`) to the three-Worker epic topology reds them. → D-19-4-09. |
| Exit: "the epic's first CI job" | **No CI job runs `demo-j1`** (only `cargo test -p xtask demo_j1`, `discipline.yml:3373`). The exit line is `cargo run -p xtask -- demo-j1`, which builds the workspace itself (`demo_j1.rs:414-421`). → D-19-4-10. |
| AC5(b): "the derivation cannot see the 2c lane; files in it are neither enrolled nor named" | **The 2c lane is already enrolment-gated**: `check-j1-two-host-signed-run` leg 10 (`check_j1_two_host_signed_run.rs:1044-1156`) derives `crates/*/tests/*_2c.rs` + `t_2c_*.rs` (`:83-89`) and is Blocking (`gate-registry.toml:481-483`), proven red by `an_unenrolled_story_test_target_reds` (`xtask/tests/j1_crosshost_2c_proven_red.rs:1015`). The real residual: the loopback derivation walks only `crates/maos-bin/tests` (`check_j1_loopback_delegation.rs:113`), so `crates/maos-iac/tests/transparency_duplicate_discrimination_2b.rs` (no J1 job names it) and `crates/maos-cli/tests/cohort_sign_2e.rs` (no gate) are underived. → D-19-4-11. |
| Tracker precondition (`:290`) | Satisfied by 19-7 (above). Nothing of it is in this story. |
| Holds | `SIGNABLE_WORKER_CLIS` `demo_j1.rs:384`; tier-2 owner `:892-896`; README `j1-two-host-evidence/README.md:11-18`; `requires_epistemic_halt_port` `main.rs:513-532` (doc `:506-511`); `ButlerOrchestratorAdapter` `:562`, impl `:585-648`; manifest rule `spirits/orchestrator/manifest.toml:50-53`; `HaltPending` `hook_dispatch.rs:43-44,283-285`; emitter `maos-spirit-hello/src/lib.rs:190`, const `shell_host.rs:35`; `lcas.rs:84,124,164`; `journey_j1.rs:78,233,276`; guard `guards_meta.rs:23-26`; `cli.rs` `Resolve` `:780-802`, kinds `:804-812`; `MorningDigestRenderer` `butler/src/lib.rs:405-491`. |

**Epic cites corrected (OLD → NEW @`61af7461`)** — the creator may not edit the epic; the substitutions are `creation/records-owed.md` R1.1–R1.16: `env_contract.rs:317/322/327` → `:332/337/342`; `main.rs:3793-3814,3805,3806` → `:3833-3840,3833,3834`; `main.rs:4997-5001` (bare) → `:5165-5184` (constructed); Mira `:5012-5058` → `:5195-5241`; tests `:14111,:14404-14424` → `:14322-14356,:14616-14650`; manifest `:40-43` → `:50-53`; `hook_dispatch.rs:281-285` → `:283-285`; renderer `:405-475` → `:405-491`, `main.rs:3987` → `:4015-4032`, `:462-474` → `:462-490`; `completions[]` → `completed[]`; job `:3329-3368` (T3 `:3344`, pre-build `:3358`) → `:3568-3602` (`:3583`, `:3596-3597`); `cli.rs:732-753`/`:756-764` → `:780-802`/`:804-812`; `J1_TEST_SUFFIXES :121` → `:129`; vector `:620-633` → `:632-649`; `deferred-work.md:1010` → `:931` (closed).

## Kernel and kloc pricing — measured at creation

**Kernel:** 0 under every recommended row. Baseline HEAD 26378 physical (= `kernel-core-baseline.toml:505`) / 20619 tokei (= `kloc.toml:264`); working tree 26389 / 20628 (19-3 K2). The only kernel option:

| Shape | D-row | Kernel Δ physical / tokei | Endpoint check | Basis |
|---|---|---:|---|---|
| K1 — `HaltRegistry::rehydrate_pending(tl, boot_nonce, spirit_id, pid, resolved)` re-pends prior boots' unresolved `EpistemicHalt` rows at the new pid | D-19-4-05 C | **+44 / +40** | 26422 + 11 + 74 = 26507 > 26483; 20659 + 9 + 46 = 20714 > 20686 — **breach** (ADR-069 alarm 22000 not reached) | `creation/k1-halt-rehydrate-prototype.diff`, /tmp copy, rustfmt, `wc -l` + tokei |

**kloc** (tokei; HEAD = every pin; tree = HEAD + uncommitted 19-3/19-6; `creation/tokei-61af7461.json`): maos-bin 30380 / **30847**, xtask 47451 / 47454, kernel 20619 / 20628, aggregate 189877 / 190356.

| Crate · component (recommended rows) | Forecast | Basis |
|---|---:|---|
| maos-bin · D-01 A scalar-port factory + `construct` plumbing + 3 construction sites | 71 | prototype `p1-…bundle.diff` |
| maos-bin · D-03 allowlist (A both tags 2 / B 1) | 1–2 | prototype |
| maos-bin · D-02 A `judge_section` + loop call | 23 | prototype |
| maos-bin · D-05 A `hold_for_resolution` + loop resume arm + `MAOS_EPIC_HALT_HOLD_SECS` (read + registry row) | 76 | prototype |
| maos-bin · D-06 A `--epic` drain digest | 12 | prototype |
| **maos-bin Σ** | **+184 measured** (uncompiled); D-01 B ≈ 143; D-05 B ≈ 200–240; D-05 C ≈ 140 + kernel | epic 100 |
| xtask · scene re-point, beats per section, 3 new beats, resume driver, D-11 A derivation | **≈ +147…+221** [INFERENCE] | `evaluate_beats` ≈ 19 lines/beat (`demo_j1.rs:572-802`) |
| maos-journey-test · maos-corpus-gen · spirits · tests | 0 | tests/cassettes/fixtures uncharged; D-08 A no seed edit |
| kernel | 0 (K1 +40 if D-05 C) | above |

Room (endpoint − tree) at creation: maos-bin 506 → after 19-2 (196–279) and 19-5's reservation (195): **32–115** for 19-4's 239 (×1.3). xtask 170 for ≈ 234. Aggregate 892 → 282–382 for ≈ 473. → D-19-4-12, **superseded 2026-10-28 by ask `w3_kloc`**: endpoints maos-bin **31710**, xtask **47688**, aggregate hardfail **191669** (room now maos-bin 863, xtask 234, aggregate 1313 against the tree).

**Landing envelope ruling 2026-10-29 (Lunarpulse, ask `w5_kloc`, "Raise to measured + ~150"):** maos-bin 32086 (`w4_kloc`) → **32860** (+774), aggregate hardfail ~192045 → **~192685**; measured 19-4/19-5 review corrections round 3 maos-bin **32826**, xtask **47682** ≤ 47688, maos-cli **6371** ≤ 6382, maos-control **1496** ≤ 1539, aggregate **192650** (strict pin **192651** = measured + 1); exact pins retain zero headroom, not the envelope.

## Decisions — RULED 2026-10-28 (D-19-4-01…11 as recommended, ask `w3_19_4`; D-19-4-12 superseded by ask `w3_kloc`)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-19-4-01** | How the Orchestrator's judged ambiguity reaches the kernel (AC2) | **A —** the epic's shape: NEW `Orchestrator::with_scalar_port(Arc<dyn EpistemicScalarPort>)` + `Orchestrator::assess_ambiguity(&EpicSection)` (= `judge_ambiguity` then `write_scalar`) in `spirits/orchestrator`; the root mints the port from ONE factory beside `ButlerOrchestratorAdapter` (`main.rs:562-648`, `ButlerPidSource::Resolve` over the Orchestrator's `PidSource`) that `orchestrator_root::construct` takes, so all three construction sites (topology `:5165`, standalone `:6010`, successor `:997`) bind it — maos-bin +71 · **B —** root-only: the loop calls `judge_ambiguity` and writes the triple through a root-held adapter (topology arm only); no `with_scalar_port` — ≈ +30 · **C —** Spirit-side port written from `on_idle` (the Butler/Mira shape) | **A.** The Spirit writes its own scalar in every other class (Butler `main.rs:740`, Researcher `:1072`, Mira `:5239`); 19-1's D-19-1-04 A made `construct` the ONE Orchestrator binding so a successor is faithful (a successor without the port is the forbidden fallback, D-18-1-R1, comment `:993-996`). B leaves a second convention and a successor that cannot halt. C cannot work: dispatch runs inside the topology load (`main.rs:4687`), not in a hook, so an `on_idle` write lands after the frames it should gate (R19-P9). | **RULED A** 2026-10-28 (Lunarpulse, ask `w3_19_4`) |
| **D-19-4-02** | When the judgement runs and what a failure does (AC2) | **A —** before each section's frame, after 19-2's safe-point drain/pause hold and before 19-1's `dispatch_stop`: judge section k → `dispatch_judged` event `{section, tag, value, halt_id}`; the kernel decides the halt; `NotWired` (unset mode, ADR-064) = no judgement; every other `OrchestratorInferenceError` refuses the run typed (D-19-1-12 A: never a default). Ambiguous section 2 ⇒ halt before frame 2 (1 frame) · **B —** after section k completes, judge section k+1 with k's distillate in the prompt (changes `ambiguity_prompt`, re-hashes 19-1's committed corpus `spirits/orchestrator/tests/halt_corpus_19_1.rs`, `cassettes/j1/orchestrator-halt-corpus-v0.1.json`) · **C —** judge all sections once at epic load (halt before frame 1) | **A.** It satisfies "≤ 2 frames" with the ambiguous section never reaching a Worker; reuses 19-1's prompt and cassette hashes unchanged; keeps every hermetic test without `MAOS_INFERENCE_MODE` (`epic_dispatch_19_1`, 19-3's `worker_workspace_19_3`, 19-6's vectors) byte-identical. C defeats FR21's per-section chain and the "within 2 frames" beat. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-03** | The halt-port allowlist flip (AC2, R19-P8) | **A —** as written: admit `dispatch_ambiguity` and `delegation_confidence` (`main.rs:525-529`); flip the doc `:506-511` and the three unit tests `:14322-14356`, `:14616-14633`, `:14635-14650`; re-target the over-fire guard `maos_run_boot_loud_8_11.rs:86-120` to a test-only manifest whose Halt rule is outside the set · **B —** admit `dispatch_ambiguity` only (it gains a writer here); `delegation_confidence` stays outside until a story gives it a writer (rule 10 deferral row); same doc/test flips and guard re-target | **B.** `delegation_confidence` has no writer anywhere (`git grep` = manifest `:55-58` + a test comment): admitting it claims a synchronous transport nothing uses (rule 11(a)). Either way the guard needs a fixture — after the flip no shipped manifest has a Halt rule outside the set. | **RULED B** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-04** | The ambiguous fixture epic (AC2) | **A —** NEW `tests/fixtures/j1/founder-loop-epic.md`, three `### Story` sections, section 2 genuinely under-specified (its acceptance names no measurable bound); `demo-j1` and the journey use it; 19-1's `tests/fixtures/j1/epic.md` unchanged · **B —** edit 19-1's `epic.md` section 2 in place | **A.** `epic.md` is the input of 19-1's, 19-3's and 19-6's vectors; their verdicts do not depend on its wording today, and a shared fixture edited for a fourth story is how that stops being true. The cassette — not the text — decides the replayed scalar, so the text must at least be honest about being ambiguous. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-05** | The resume (AC5) | **A — in-boot hold:** on `dispatch_stop` for a pending halt, the loop holds before the section (poll ≤ 100 ms — 19-2 D-19-2-17 A's pause-hold shape, one helper) bounded by `MAOS_EPIC_HALT_HOLD_SECS` (default **0** = 19-1's stop, so `epic_dispatch_19_1`'s AC4.3(b) is unchanged) and the root's shutdown; resolved over the door (`maosctl halt resolve … --kind provided-context`) → `dispatch_resumed`, the clarification (read from `halt_context::<id>`) appended to the section's goal, the section dispatched without re-judgement; `accepted-halt` → stop as today; `authorized-override` → dispatch unchanged. Kernel 0, maos-bin +76 · **B — cross-boot, maos-bin:** run 1 halts and drains; run 2 in the same `MAOS_HOME` re-pends the unresolved halt from the TL through the public `insert_pending_with_metadata`, is resolved, and re-dispatches from the halted section — ≈ +90–130 [INFERENCE] + resume-from-section · **C — cross-boot, kernel:** B with `HaltRegistry::rehydrate_pending` in the kernel — **+44 / +40 measured**, breaches the E19 kernel endpoint (FLAG-Winston) | **A.** The door is already served during a `--once` load (`main.rs:3530-3596`) and the 16-2 resolver already writes the clarification to the Spirit (`halt/resolver.rs:165-181`): A composes existing pieces at kernel 0. B/C invent cross-boot halt persistence that FR51(b)/16-2 never specified (a pending halt on exit is today terminated with receipts, `supervision::unload_all_loaded`), and C breaks the kernel endpoint. A's residual (a halt pending at exit is not resumable) is filed at landing. | **RULED A** 2026-10-28 (ask `w3_19_4`); kernel-Δ 0 — the K1 prototype is not taken |
| **D-19-4-06** | Where and over what the digest renders (AC3, D19-5 A) | **A —** `--epic` topology runs only, at the `--once` drain after the idle passes (`main.rs:5343`): `seam.digest.render_scene_day(orchestrator_pid, boot_nonce)` — this boot's frames — `{"event":"morning_digest"}` + the TL frame, inference-free · **B —** every topology `--once` drain · **C —** `render_if_new_day` (last-24h window) | **A.** B changes the stdout of every topology run (journey, two-host, cohort fixtures) for no claim; C reads a reused home's earlier boots, so the planted-extra-completion red could not tell a hallucinated ref from a stale real one. A is the 18-4 per-boot window (D-18-4-R5). | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-07** | A resolved halt in the post-resume digest (AC3/AC5) | **A —** Butler's `digest_over` (`spirits/butler/src/lib.rs:1707-1778`) joins `halt.resolve` approval decisions (`halt/mod.rs:51-80`, `reasoning` = `halt=<id>: …`) and moves resolved halts to a NEW `resolved_halts[]` (`source_log_ref` + resolution kind); the composed-view consistency check (`:1757-1760`) counts both; `halt-resume-referential-identity` asserts the pre-halt frame id in `resolved_halts` · **B —** no Butler change; the beat finds the ref in `open_halts` (a resolved halt reported as "requiring resolution", FR17(b)); deferred row to Butler's owner | **A.** FR17(b) says *open* halts; B makes the beat green by reading a mislabel. `spirits/` is kloc-ungoverned; Butler's digest tests (`spirits/butler/tests/{digest,hallucination}.rs`) gain the resolved case. Owed measurement 6 confirms the approval row's query shape. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-08** | The ambiguity tag spelling (AC2) | **A —** no re-spell: the Orchestrator's halt is journaled under its scalar tag `dispatch_ambiguity` (`policy_runtime.rs:134-141`); `task.acceptance_criterion.ambiguous` stays hello-spirit's J0 tag; the LCAS seeds and corpus are untouched; the epic sentence is corrected (records-owed R1.9) · **B —** as written, completed: re-spell `lcas.rs:84,124,164`, bump `RULE_VERSION` (`:26`), regenerate `tests/corpora/lcas-v0.3.jsonl` (24 occurrences) and its sha (`MANIFEST.toml:46`) · **C —** rename the Orchestrator's scalar/rule tag to `task.acceptance_criterion.ambiguous` | **A.** B rewrites a sha-pinned eval corpus and still does not make the J1 halt share a tag (the kernel journals the scalar tag). C breaks 19-1's committed corpus and judgement tests (`spirits/orchestrator/tests/judgement.rs:147`, the prompt's JSON key `lib.rs:164-175`). | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-09** | `demo-j1`'s scene (AC1) | **A —** re-point `TOPOLOGY` (`demo_j1.rs:48`) to `j1-founder-loop-epic.toml` + `--epic` (D-04 fixture); P2–P6 become per-section (each section's `delegation_routed → cli_wrapper_loaded → worker_completion → delegation_completed` once, in section order; `drain` once, last); three new beats + the resume beat; `run_scene` streams stdout (reader thread) to resolve the halt mid-run · **B —** keep the one-Worker scene and add a second `maos run` for the epic (two T3 runs per demo; the new beats from the second) | **A.** The epic re-points it (AC1); B doubles the T3 cost and leaves the narrated loop single-Worker. The live take (19-3's) keeps its own topology. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-10** | The exit's CI job (AC1, D19-3 A) | **A —** NEW own-build job `epic-19-exit` (checkout, toolchain, rust-cache, `./.github/actions/provision-t3-test-substrate`, `MAOS_REPLAY_STRICT=1`, the exit line verbatim, the AC2/AC5 twins), in `aggregate.needs` beside `epic-18-exit` (`discipline.yml:6362`, WT `:6365`) · **B —** a step in `check-j1-loopback-delegation` (own-build, T3 at `:3346`; timeout 30 min) · **C —** a 20-15 consumer (`CARGO=/bin/false`) running a prebuilt `xtask demo-j1 --skip-build` | **A.** C cannot run the line verbatim (`cargo run`); B puts a full `cargo build --workspace` (`demo_j1.rs:414-421`) inside a 30-min job that already builds and runs ~12 targets. Owed measurement 7 sizes A's timeout. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-11** | The `_2c.rs` blind spot (AC6) | **A —** fix at origin (rule 10(a)): the loopback derivation walks `crates/*/tests` for its suffixes (+ `_2e.rs`), enrolment checked crate-qualified on one line (the 2c gate's `has_flag_arg` rule, `check_j1_two_host_signed_run.rs:1126-1132`); `_2c` is excluded and named as leg 10's in the gate doc; the newly derived `maos-iac::transparency_duplicate_discrimination_2b` and `maos-cli::cohort_sign_2e` are enrolled in the J1 job; un-enrolled `crates/maos-iac/tests/xyz_2b.rs` twin vector — xtask +25–40 · **B —** close by record: the gate doc and 15-1 record name leg 10 as the 2c control (proven red `j1_crosshost_2c_proven_red.rs:1015`); residual filed · **C —** the epic's literal: add `_2c.rs` to `J1_TEST_SUFFIXES` and walk `crates/*/tests` — the six 2c targets then need enrolment in BOTH jobs | **A.** The 2c lane already has a derived Blocking control (B's evidence); what is genuinely blind is the two J1-lane files outside `maos-bin/tests`, and that is inside this AC's file. C duplicates a control across two jobs. | **RULED A** 2026-10-28 (ask `w3_19_4`) |
| **D-19-4-12** | Kloc envelope (E19, D19-1 A) | **A —** raise once now, sized at 19-2's upper bound: maos-bin 31353 → **31560** (+207 = 239 − 32); xtask 47624 → **47688** (+64 = central 234 − 170; the upper 287 would need +117); aggregate 191248 → **191519** (+271); STOP and a new D-row above · **B —** no raise now: the dev pass STOPs at the room (maos-bin 32–115, xtask 170) and returns an exact measured request at landing (the per-story ask R1 retired) · **C —** cut to fit: D-01 B, D-11 B, and AC5 split to a follow-up story (maos-bin ≈ 67 ×1.3 = 87 fits; xtask ≈ 67–106 ×1.3 ≤ 138 fits) — an operator-approved scope cut | **A.** E18 retro R1 set one envelope ratified once; the shortfall comes from the tree's measured wave-2 use (maos-bin +467 against 19-3/19-6's reserved 292) and from pricing 19-4 by prototype (184) instead of analogue (100). B guarantees a mid-dev stop on a measured certainty; C drops the AC that closes the epic's only ABSENT owned beat. | **SUPERSEDED** 2026-10-28 by ask `w3_kloc` ("Raise to cover forecasts ×1.3"): maos-bin 31353 → **31710** (+357), xtask 47624 → **47688** (+64), aggregate 191248 → **191669** (+421) — applied in the epic (`epic-19-founder-loop-w4.md:40,41,48,138`) |

## Acceptance Criteria

*(Epic §19-4 as corrected at creation; the epic's second "AC5" is AC6. Clauses marked (D-…) follow the recommended row; a different ruling rewrites only that clause.)*

### AC1 — command and observable

1. **Command (verbatim, the epic's exit):** `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/founder-loop.json cargo run -p xtask -- demo-j1` — no new flag (D-D/ADR-064); the env pair reaches the child (no `env_clear`; `MAOS_LIVE_AGENT` still removed, `demo_j1.rs:550`). `TOPOLOGY` (`:48`) → `spirits/topologies/j1-founder-loop-epic.toml` with `--epic tests/fixtures/j1/founder-loop-epic.md` (D-09 A, D-04 A); the child also gets `MAOS_EPIC_HALT_HOLD_SECS` and a door (`maos init` in the isolated home, the J0 `init_world` shape `journey_j0.rs:96-111`) (D-05 A).
2. **Claim table:** every EXECUTED beat `ok`; P1 `topology-spirits-loaded` unchanged (exact three; D-E holds — no Butler class); P2–P6 per section (D-09 A); NEW executed `epic-decomposed-n-frames` (3 sections ⇒ 3 `delegation_routed`, 3 `delegation_completed`, in section order), `ambiguity-halt-within-2-frames` (`dispatch_judged` for section 2 carries a `halt_id`; `dispatch_halted` follows with ≤ 2 `delegation_routed` before it — 1 under D-02 A), `digest-cites-source-refs` (AC3), `halt-resume-referential-identity` (AC5). `tier2-live-agent-signed` ABSENT by construction (owner row `ops-j1-live-agent-signed-take`), `two-host-signed-run` INDETERMINATE (`apply_capture_judgement` `:958-984` unchanged). `unlanded_beats()` keeps two entries (tier-2, two-host). The "does NOT claim" halt/resume lines (`:307-309`) state the in-boot scope (D-05 A).
3. **CI (D-10 A):** NEW job `epic-19-exit` runs the line verbatim with `MAOS_REPLAY_STRICT=1` after `provision-t3-test-substrate` (D19-3 A), plus the AC2 twin (clear-judgement cassette ⇒ `demo-j1` exits non-zero naming `ambiguity-halt-within-2-frames`); in `aggregate.needs`. The exit block is unchanged.
4. 19-3's live take (`demo_j1.rs:1203-1420`, module doc `:30`) is not touched.

### AC2 — the Orchestrator judges each section and the kernel halts on ambiguity

1. **Spirit (D-01 A):** `Orchestrator::with_scalar_port` and `Orchestrator::assess_ambiguity(&EpicSection) -> Result<(f64, Option<HaltReceipt>), OrchestratorInferenceError>` beside `judge_ambiguity` (`spirits/orchestrator/src/lib.rs:396-445`); no port ⇒ `NotWired`. Dispatch stays deterministic (Decision E, `manifest.toml:41-46` comment).
2. **Root (D-01 A):** one factory beside `ButlerOrchestratorAdapter` (`main.rs:562-648`) bound to the manifest's `[epistemic_policy]` and the Orchestrator's `PidSource`; `orchestrator_root::construct`/`construct_at_admitted_pid` take it; all three construction sites bind it. Shape: `creation/p1-maos-bin-recommended-bundle.diff`.
3. **Loop (D-02 A):** before each section's frame — after 19-2's safe-point drain and pause hold, before 19-1's `dispatch_stop` (`main.rs:4858-4879`) — `orchestrator_root::judge_section` prints `{"event":"dispatch_judged","section":k,"tag":"dispatch_ambiguity","value":v,"halt_id":…}`; the manifest rule (`manifest.toml:50-53`, > 0.5) fires the kernel halt through `process_scalar_write`; `NotWired` = no judgement; any other error refuses the run typed.
4. **Allowlist (D-03 B):** `requires_epistemic_halt_port` admits `dispatch_ambiguity`; doc `:506-511` and tests `:14322-14356`, `:14616-14650` flipped; the over-fire guard `maos_run_boot_loud_8_11.rs:86-120` re-targeted to a test-only manifest with an unlisted Halt tag; `delegation_confidence` deferral row filed.
5. **Tag (D-08 A):** the halt row's intent is `dispatch_ambiguity`; no seed or corpus edit.
6. **Proven red:** the clear-judgement twin cassette (`founder-loop.clear-twin.json`, section 2 judged ≤ 0.5) ⇒ no halt, three frames, `ambiguity-halt-within-2-frames` FAIL — in the exit job (AC1.3) and the journey (AC4).

### AC3 — the FR17 digest at drain cites real rows (D19-5 A)

1. **Render (D-06 A):** an `--epic` `--once` topology run, after its idle passes, renders `seam.digest.render_scene_day(orchestrator_pid, boot_nonce)` — `{"event":"morning_digest"}` + its TL frame (`MORNING_DIGEST_INTENT`), inference-free (`Butler::new()`, `butler/src/lib.rs:473`); non-`--epic` topology runs unchanged.
2. **Resolved halts (D-07 A):** `digest_over` reports a resolved halt in `resolved_halts[]`, never in `open_halts[]`.
3. **Beat `digest-cites-source-refs`:** every `completed[].source_log_ref` (and `resolved_halts[]`/`open_halts[]` ref) resolves to a row in `maosctl audit query --range 1d --format ndjson` (the parity read `demo_j1.rs:494-531` already runs it) of kind `TaskComplete`/`CliSubprocessOutput` (halts: `EpistemicHalt`); ≥ 3 completions. **Proven red:** a synthetic stream with one extra completion ref (`demo_j1_tests`) ⇒ FAIL; Butler's `hallucination.rs` check gains the resolved-halt case.

### AC4 — the journey and the cassette

1. `crates/maos-journey-test/cassettes/j1/founder-loop.json`: schema `maos.journey.cassette/v1`, `provenance: seed`, three entries in judgement order (s1 clear, s2 ambiguous > 0.5, s3 clear; section 2 is not re-judged after resolution, D-05 A), each with the real `prompt_sha256`/`prompt_len` of `ambiguity_prompt(section text)` (the `halt_corpus_19_1.rs:8-11` method; never the all-zero wildcard); the clear twin beside it.
2. `journey_j1.rs`: `j1_resume_continuity_ref_identity_oracle` (`:276-369`) becomes the epic run under replay with `MAOS_REPLAY_STRICT=1` **on the child**, the halt resolved over the door with `maosctl halt resolve <id> --spirit orchestrator --kind provided-context --text …`, and asserts `dispatch_judged`/`dispatch_halted`/`dispatch_resumed`, three `delegation_completed`, the digest refs and the pre-halt frame id in `resolved_halts` (oracle comment `:363-368` replaced). Bounded waits only through the harness (`guards_meta.rs:23-26`). `:78` and `:233` unchanged.
3. Runs in `journey-hermetic-tier-1` (`discipline.yml:3568-3602`, T3 `:3583`); the job pre-builds `maosctl` already (`:3597`).

### AC5 — the resume beat (D-05 A)

1. With `MAOS_EPIC_HALT_HOLD_SECS > 0`, a pending halt on the Orchestrator holds the section (no frame, no Worker) and prints `dispatch_halted` once; a resolution over the door releases it within one poll: `provided-context` ⇒ `{"event":"dispatch_resumed","section":2,"halt_ids":[…],"provided_context":true}` and section 2's goal carries the clarification; `accepted-halt` ⇒ the 19-1 stop; the bound or shutdown ⇒ the 19-1 stop. Default 0 ⇒ 19-1's behaviour byte-for-byte (`epic_dispatch_19_1` AC4.3(b) untouched).
2. `MAOS_EPIC_HALT_HOLD_SECS` registered in `env_contract.rs` (user-facing).
3. `halt-resume-referential-identity` is an EXECUTED beat: the `halt_id`/frame id from `dispatch_judged` is cited in the post-resume digest's `resolved_halts` (D-07 A).
4. **Proven red:** `demo-j1` with the resolve step removed ⇒ the hold expires, `dispatch_halted` stop, the resume beat FAIL (in-tree `demo_j1_tests` over a recorded stream; the plant itself once by the dev pass).

### AC6 — the J1 enrolment blind spot (received from 15-1; D-19-4-11 A)

1. `derive_enrolled_targets` (`check_j1_loopback_delegation.rs:815-860`) walks every `crates/*/tests` for `J1_TEST_SUFFIXES` (+ `_2e.rs`), keeps 15-1's D-2 digit-segment exclusion (`:839-855`) and the `_19_1`/`_19_3` suffixes; `completion-vectors-enrolled` checks `-p <crate> --test <stem>` on one line; `_2c` is named as `check-j1-two-host-signed-run` leg 10's.
2. The J1 job enrols `cargo test -p maos-iac --test transparency_duplicate_discrimination_2b` and `cargo test -p maos-cli --test cohort_sign_2e`.
3. **Proven red (in-tree vector):** an un-enrolled `crates/maos-iac/tests/xyz_2b.rs` reds `completion-vectors-enrolled`; `planting_an_unenrolled_j1_test_file_must_red` (`j1_crosshost_1b_proven_red.rs:632-649`) and the `_19_1`/`_19_3` twins stay green.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-19-4-01…11 RULED 2026-10-28 (ask `w3_19_4`), D-19-4-12 superseded (ask `w3_kloc`); tracker → `in-progress`; 19-2, 19-3, 19-6 landed (shared `main.rs` loop, `orchestrator_root.rs`, `demo_j1.rs`). Run `creation/measurements.md` items 1, 2, 5, 6 (item 3 is moot — D-05 A ruled); re-measure every WT-flagged cite. If the cumulative E19 maos-bin or xtask use would exceed the raised endpoints (31710 / 47688), STOP and return a D-row.
- [ ] **T1 — Spirit + root (AC2):** `with_scalar_port`/`assess_ambiguity`; factory; `construct` signature; three sites; `judge_section`; allowlist + tests + guard fixture.
- [ ] **T2 — Resume (AC5):** `hold_for_resolution` (shared with 19-2's pause-hold helper if it landed as one); loop arm; env row.
- [ ] **T3 — Digest (AC3):** drain render; Butler `resolved_halts`; Butler tests.
- [ ] **T4 — Data (AC2, AC4):** `tests/fixtures/j1/founder-loop-epic.md`; `cassettes/j1/founder-loop.json` + clear twin with real hashes.
- [ ] **T5 — xtask (AC1, AC6):** `demo_j1.rs` scene, beats, streaming + resolve driver; `demo_j1_tests` (the 9-beat fixture `:247-308` becomes per-section); loopback derivation; vectors.
- [ ] **T6 — Tests + CI (AC1.3, AC4, AC6.2):** journey rewrite; `epic-19-exit` job + twins; J1 job enrolments.
- [ ] **T7 — Plants** (one per AC; `proof-contract.md:24-31`): AC2 clear twin (exit job + journey), AC3 extra ref (in-tree), AC5 resolve removed (recorded once), AC6 `xyz_2b.rs` (in-tree); T3-bound ones by a branch-only hosted run (`ci/19-4-verify`, the 19-1 pattern), run id recorded.
- [ ] **T8 — Landing.** `cargo fmt --all`; `kloc-check --json`; exact re-pins inside the ruled envelope with cumulative E19 use; `check-kernel-baseline` unchanged; `check-j1-loopback-delegation`, `check-j1-two-host-signed-run`, `check-exit-commands`; `prepush`; records-owed R5. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Orchestrator Spirit | `spirits/orchestrator/src/lib.rs` (beside `judge_ambiguity` `:396-445`; struct `:290-302`); tests `spirits/orchestrator/tests/judgement.rs` | AC2.1 |
| Root | `crates/maos-bin/src/main.rs` `:506-532`, after `:648`, `:997-1015`, `:4681-4683`, `:4853-4879`, `:5165-5184`, `:5343` (drain), `:6010-6023`, tests `:14322-14356,14616-14650`; `orchestrator_root.rs` `:41-83` + new fns beside `dispatch_stop` `:201-225`; `env_contract.rs` (beside `:332-345`) | AC2.2–2.4, AC3.1, AC5 |
| Butler | `spirits/butler/src/lib.rs` `digest_over` `:1707-1778`, `MorningDigest` `:706-715`; `spirits/butler/tests/{digest,hallucination}.rs` | AC3.2 |
| xtask | `xtask/src/demo_j1.rs` `:48`, `:146-349` (narration/scene), `:533-568` (`run_scene` → streaming), `:572-802`, `:890-908`; `xtask/src/tests/demo_j1_tests.rs`; `xtask/src/check_j1_loopback_delegation.rs` `:113-133`, `:815-860`, leg 5; `xtask/tests/j1_crosshost_1b_proven_red.rs` | AC1, AC3.3, AC5.4, AC6 |
| Tests / data | `crates/maos-journey-test/tests/journey_j1.rs:276-369`; NEW `crates/maos-journey-test/cassettes/j1/founder-loop.json` (+ clear twin); NEW `tests/fixtures/j1/founder-loop-epic.md`; `crates/maos-bin/tests/maos_run_boot_loud_8_11.rs:86-120` (+ fixture manifest) | AC2.4, AC4 |
| CI / records | `.github/workflows/discipline.yml` (NEW `epic-19-exit`; J1 job `:3383-3428`; `aggregate.needs` `:6362` area); `xtask/kloc.toml:517,339,718`; `runbook-j1-demo.md:64,141` | AC1.3, AC6.2, T8 |

### Overlap with 19-2, 19-3, 19-6 (land 19-4 after all three; re-measure)

- **`main.rs` loop `:4853-4879`** — 19-2 adds the safe-point drain and the pause hold (D-19-2-17 A) at the same point (`:4853-4917`) and a drain-event field (`:5352-5360`); 19-6 edits `:4683-4687`, `:4885` and the crash branch `:5027-5116` (WT hunks). 19-4's order at that point: 19-2 drain → 19-2 pause hold → 19-4 judge → 19-1 `dispatch_stop` → 19-4 resolution hold → frame. 19-6's reassign re-dispatch of a section is not a new section: it must not re-judge (keep `epic_next` semantics, 19-6 AC3.2).
- **`orchestrator_root.rs`** — 19-2 adds a helper + hold seam beside `dispatch_stop`/`plant_test_halt`; 19-6 adds `CRASH_PLANT_ENV`, `plant_test_crash`, `declares_reassign`, `dispose_orphan` (WT). 19-4 changes `construct`'s signature — every caller (three in `main.rs`) moves with it.
- **`spirits/orchestrator/src/lib.rs`** — 19-2 adds a fold helper, 19-6 `release_orphaned_delegation` (WT `:478-488`); 19-4 adds the port field + two methods.
- **`env_contract.rs`** — 19-2 and 19-6 add rows (WT `:201-205`).
- **`xtask/src/demo_j1.rs`** — 19-3 edited only the live take (`:1204-1420`), the module doc `:30-33` and two comments (`:436-440`, `:481`); 19-4 edits everything else listed above. **`check_j1_loopback_delegation.rs` / `j1_crosshost_1b_proven_red.rs`** — 19-3 added `_19_3.rs` and its twin; 19-4's derivation keeps them.
- **`discipline.yml`** — 19-3 edited the J1 step `:3427-3428`; 19-2 adds its daemon vectors to `workspace-test-suite`; shared `aggregate.needs`.
- **`xtask/kloc.toml:517,339,718`** — every E19 story re-pins; take the measured figure at landing.

### Current state that must be preserved

- Unset `MAOS_INFERENCE_MODE` ⇒ no judgement, no new event, byte-identical stdout for every existing `--epic` vector (`epic_dispatch_19_1`, `epic_args_19_1`, 19-3/19-6 vectors).
- `MAOS_EPIC_HALT_HOLD_SECS` unset ⇒ 19-1's stop exactly (`dispatch_halted`, drain, exit 0 — AC4.3(b)).
- Cross-host arm: `--epic` refused (`orchestrator_root.rs:122-132`); no judgement there.
- `demo-j1`'s honest-labeling rules (`demo_j1.rs:17-31`), `apply_published_ledgers`' no-promotion rule (`:1048-1061`), the live take's preflights and identity check, `SIGNABLE_WORKER_CLIS`.
- D-E: no Butler class in the topology; the digest renderer builds a bare `Butler::new()` and never infers.

### Guardrails

- Kernel diff: none (any `crates/maos-kernel-core/src` change is outside the recommended rows — STOP).
- Never weaken `dispatch_stop`, the strict-replay refusal, the halt registry or `live_agent_gate` to make a beat pass; a red beat is a finding.
- The hold polls the registry only (no sleep-based ordering in tests: harness bounded waits, `guards_meta.rs:23-26`); `MAOS_HOME` is process-global — `--test-threads=1` for journey/maos-bin vectors.
- One cargo process; `env -u CI -u GITHUB_ACTIONS`; `CARGO_TARGET_DIR=/mnt/build/cargo`; plants byte-restored.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Ambiguity halt (AC2) | clear-judgement twin cassette | `ambiguity-halt-within-2-frames` FAIL; journey asserts no halt + 3 frames |
| Digest citation (AC3) | extra completion ref in a synthetic `morning_digest` | `digest-cites-source-refs` FAIL (in-tree) |
| Resume (AC5) | resolve step removed | hold expires ⇒ stop ⇒ resume beat FAIL |
| Enrolment (AC6) | un-enrolled `crates/maos-iac/tests/xyz_2b.rs` | `completion-vectors-enrolled` FAIL (in-tree) |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo; T3 vectors SKIP without the substrate
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p orchestrator -p butler
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test epic_dispatch_19_1 --test epic_args_19_1 --test maos_run_boot_loud_8_11 -j 1
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --bin maos story_9_6
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-journey-test --test journey_j1 -- --test-threads=1
env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask demo_j1 -- --test-threads=1 && env -u CI -u GITHUB_ACTIONS cargo test --locked -p xtask --test j1_crosshost_1b_proven_red -- --test-threads=1
cargo run -q --locked -p xtask -- check-j1-loopback-delegation --json && cargo run -q --locked -p xtask -- check-j1-two-host-signed-run --json
MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j1/founder-loop.json cargo run -p xtask -- demo-j1   # needs the T3 env (R19-P2)
cargo run -q --locked -p xtask -- kloc-check --json && cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- check-exit-commands
cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-75`)

- DIFFICULT: Blind and Edge (opus-5.5) — Edge walks the hold (resolution racing the poll, shutdown during hold, a second halt during hold, `accepted-halt`/`authorized-override`, the Orchestrator unloaded through the door while held), strict replay with a missing/extra entry, a judgement error per variant, the digest window with a reused home; Acceptance on opus-5.5; Test Infrastructure on glm-5.3 re-reads the new job, the J1 enrolments and the derivation.
- Focused set: `orchestrator`, `butler`, `maos-bin` (targets above + every reader of `spirits/orchestrator/manifest.toml`: `grep -rlE 'orchestrator/manifest.toml|gate_manifest' crates/*/tests spirits/*/tests`), `maos-journey-test --test journey_j1`, `xtask` (`demo_j1`, the four `j1_crosshost_*_proven_red`), `check-corpus` (manifest reader rule).

### Previous-story intelligence

- **19-1:** the loop, the stop, the plant and the judgement this story wires; landed maos-bin ≈ +386 against a 205 forecast — this story is priced by prototype for that reason; T3 vectors were proven only on a branch-only hosted run (`ci/19-1-verify`).
- **19-3 / 19-6 (in the tree):** their maos-bin use is +467 against a 292 reservation (measured here) — the envelope arithmetic above depends on it.
- **19-2 (ruled):** D-19-2-17 A's pause hold is the same poll-and-hold shape AC5 needs; reuse it.
- **16-2:** one resolution mechanism for the REPL and the door (`shell_host_16_2.rs:1030-1032`); `journey_j0.rs` is the door-resolution harness pattern (`init_world`, bounded waits).
- **18-4:** the per-boot digest window (D-18-4-R5) AC3 reuses.

### Git intelligence

| Commit | Content |
|---|---|
| `61af7461` | 19-1, 19-7 done (landing 40687349154 @ 309e277a; `deferred-work.md:931` closed) — this creation's HEAD |
| `a361bd1b` / `c80964d8` | 19-2 / 19-3, 19-6 created and ruled |
| `309e277a` | 19-1/19-7 landing tree |

### References

- Epic `epic-19-founder-loop-w4.md:13-28,34-48,86-91,97-105,140-143,154,159,195-208,223-225`; tracker `sprint-status.yaml:290,337`; `deferred-work.md:931`; `15-1-green-at-head.md:181,722`; `runbook-j1-demo.md:64,141`.
- Code: `xtask/src/demo_j1.rs:17-31,48,139-349,384,393-447,494-568,572-802,890-984,1006-1069`; `xtask/src/check_j1_loopback_delegation.rs:105-133,815-860`; `xtask/src/check_j1_two_host_signed_run.rs:50-52,78-89,1044-1156`; `crates/maos-bin/src/main.rs:506-648,997-1015,3412-3596,3833-3840,4015-4032,4651-4980,5160-5241,5298-5459,6010-6072,14322-14356,14616-14650`; `orchestrator_root.rs:30-259`; `env_contract.rs:332-345`; `spirits/orchestrator/src/lib.rs:60-114,164-226,284-445`, `manifest.toml:33-58`; `spirits/butler/src/lib.rs:396-491,670-715,1677-1870`; `crates/maos-kernel-core/src/halt/mod.rs:51-80,100-112,193-267,389,530-592`, `halt/resolver.rs:116-200`, `capability/working_memory/policy_runtime.rs:120-145`, `scheduler/hook_dispatch.rs:43-44,281-287`; `crates/maos-bin/src/cassette_replay.rs:130-166`; `crates/maos-cli/src/cli.rs:770-812`; `crates/maos-journey-test/tests/journey_j1.rs:1-369`, `journey_j0.rs:76-123`; `crates/maos-bin/tests/maos_run_boot_loud_8_11.rs:86-120`, `epic_args_19_1.rs:403-427`, `f4_pairing.rs:1170-1178`; `crates/maos-corpus-gen/src/lcas.rs:25-26,84,124,164`; `tests/corpora/MANIFEST.toml:45-51`.
- CI: `discipline.yml:3334-3440,3441-3500,3568-3602,6138-6165,6285-6392`; `xtask/gate-registry.toml:464-483`; `docs/testing/proof-contract.md:24-75`.

## Dev Agent Record

### Story context creation

Created 2026-10-28 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `61af7461`, without cargo (parent rule).

**Inputs:** direct reads of every cited range on a `git archive` copy of HEAD (`/tmp/h19x4`) and in the working tree (`creation/cites-61af7461.md`); kloc by tokei replication of the kloc-check instrument on both (`creation/tokei-61af7461.json`; HEAD = every pin); kernel physical by `wc -l` over the 94 git-tracked kernel files (26378 HEAD, 26389 tree); one kernel prototype (K1, +44/+40) and one maos-bin bundle (+184) on `/tmp` copies, rustfmt'd and counted, saved as diffs and never applied to the tree.

**Measurements owed:** `creation/measurements.md` items 1–7.

**Records owed:** `creation/records-owed.md` (R1 epic cites, R2 kloc/kernel per ruling, R3 tracker row incl. the satisfied precondition, R4 15-1 annotation, R5 landing).

**Revert:** nothing to revert — no crate source was modified (prototypes only under `/tmp/proto19x4`); no test ran, so `intent-lineage-coverage-report.md` is untouched.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- `19-4-j1-beats-and-demo-replay-evidence/creation/{measurements.md, cites-61af7461.md, records-owed.md, tokei-61af7461.json, k1-halt-rehydrate-prototype.diff, p1-maos-bin-recommended-bundle.diff}`.

### Completion Notes List

### File List

### Change Log

- 2026-10-28: Story created at `61af7461` (ready-for-dev pending the operator's rulings on D-19-4-01…12). Epic §19-4 cites re-measured (records-owed R1.1–R1.16); tracker precondition recorded as satisfied by 19-7; kernel option D-05 C priced +44/+40 (breaches the E19 endpoint); maos-bin bundle priced +184; envelope shortfall filed as D-19-4-12.
- 2026-10-28: Operator rulings recorded (Lunarpulse): D-19-4-01…11 as recommended (ask `w3_19_4`: 01 A, 02 A, 03 B, 04–11 A); D-19-4-12 superseded by ask `w3_kloc` ("Raise to cover forecasts ×1.3": maos-bin 31353 → 31710, xtask 47624 → 47688, aggregate 191248 → 191669). Status → ready-for-dev (RULED 2026-10-28). Records owed R1 (epic cites), R2 (envelope rows + D19-1 note), R3 (tracker `:290`) applied; R4 not applied; R5 at landing.
