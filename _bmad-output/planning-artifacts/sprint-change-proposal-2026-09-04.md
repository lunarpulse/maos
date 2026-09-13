# Sprint Change Proposal — Product-Spine Recovery Plan as Epics 15–20 (W0–W5)

> **Date:** 2026-09-04 · **Workflow:** bmad-correct-course (Batch mode) · **Operator:** Lunarpulse (present)
> **Trigger:** full fresh promise-vs-reality review at HEAD `4657cace` (*MAOS Promise Ledger*, https://claude.ai/code/artifact/e9b88de1-d01d-4e6b-94c9-932810ca82c5; evidence `review-2026-09-04/`).
> **Operator decisions this run:** Batch review · **each wave becomes a numbered epic (15–20)** so `check-epic-close-coherence`, the epics index, retros and the decision register track it unchanged · AC1 of every story is an operator-runnable command.
> **Prior approvals folded in:** Epic 14 closed by re-scope (Appendix B) and the tracker cleanup (Appendix C) were approved and applied earlier today; this proposal formalizes the six waves those appendices pointed at.
> **Change scope classification: Major** — re-sequences the remaining roadmap into six new epics; PRD phase order unchanged; one authorized kernel delta (Epic 16).
> **APPROVED 2026-09-04 (Lunarpulse) and APPLIED the same day**: Epics 15–20 created under `epics/`, 26 rows renamed, index/DAG/inventory/PRD/architecture updated, register and deferred-work re-pointed; gates re-run (results in Section 5).

---

## Section 1: Issue Summary

**Problem.** The tracker records Epics 0–13 `done` and `RELEASE-HOLDS.md` says every PRD journey is served. Measured at HEAD: 22 of 66 FRs live from a product binary; 0 release tags in 319 commits; 1 of 11 Spirits calls the Inference Port; every reference Spirit implements 1 of 14 lifecycle hooks; the discipline `aggregate` job is red on four gates plus one deterministic test red from a `done` story. "Done" is true at the harness layer and false at the operator layer.

**Discovery.** Eight read-only scouts (one axis each) plus my own `cargo check`, twelve gate runs and a full `cargo test --workspace` on 2026-09-04. Prose in `_bmad-output/` was used only as the statement of the promise; every reality claim cites `file:line`.

**Category (checklist 1.2):** *misunderstanding of original requirements* at the composition layer, compounded by *failed approach* on governance-first sequencing — mechanisms were built and gated story by story without the operator path that composes them.

**Evidence (checklist 1.3) — the five seams, one receipt each:**

| # | Seam | Receipt |
|---|---|---|
| S1 | Control plane never reaches the daemon (every `maosctl` verb but one spawns a fresh `maos` via `MAOS_ONE_SHOT` on an empty registry) | `maos-cli/src/subcommands.rs:1461-1842`; `maos-bin/src/main.rs:2745` |
| S2 | Sandbox and cgroups computed, never applied (`spawn_sandboxed` zero prod callers; bare `Command::spawn` with host creds) | `kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461-473`; `security/mod.rs:39,485`; `scheduler/resource_ceiling.rs:76-138` |
| S3 | One Spirit thinks (Butler never imports InferencePort; Observer unlaunchable; 2 hook firers have no callers) | `spirits/butler/src/lib.rs:390-464`; `main.rs:447-454`; `scheduler/hook_dispatch.rs:285-345` |
| S4 | Gates judge files (12 evidence-file gates pass on absence; Aud-7 gate averages its own fixture labels; 3 `CURRENT_PHASE` sources) | `maos-eval/tests/distillate_five_metrics_floor.rs:53-81`; `gate_common.rs:166` vs `tests/phase-config.toml:5` vs `check_escape_detector.rs:62` |
| S5 | Never shipped (`release.yml` never ran; packaging scaffolds; retired model pins; 401/429 masked as "transport"; secrets env-only) | `release.yml:70-93`; `packaging/homebrew/maos.rb:25`; `spirits/butler/manifest.toml:21`; `io/mod.rs:105-108` |

## Section 2: Impact Analysis

**Epic impact (checklist 2.1–2.5).**
- **Epic 14** (the trigger's epic): could not be completed as planned without 4–6 weeks of governance-only work; **closed by re-scope** with 6 delivered stories (Appendix B). Its 11 remaining rows already sit in the recovery lane.
- **Epics 0–13:** statuses untouched; no story reopened. Their mechanisms are what Epics 15–20 compose.
- **New epics:** six, one per wave — **Epic 15 (W0), 16 (W1), 17 (W2), 18 (W3), 19 (W4), 20 (W5)**. Numbering continues the project's sequence so every existing gate, index and retro convention applies.
- **Resequencing:** the PRD phase order (v0.1 → v0.3/v0.5 → v0.8 → v1.0 → v1.5/v2.0/v2.2) is kept and *executed in that order*; each epic makes one phase's milestone true.
- **Obsolete/parked:** canary auto-rollback + native push and the three `v25-*` rows stay in v2.5 parking (non-gating by operator rule).

**Story impact.** 26 recovery rows are renamed into the six epics (`NN-M-slug` → `(15+N)-M-slug`, W0 renumbered 1–3). The three approved model-pin hotfix rows keep their keys and run inside Epic 15's window. No story file exists yet for any of them; files are authored by `bmad-create-story` with AC1 = command.

**Artifact conflicts (checklist 3.1–3.4).**
- **PRD:** no FR added, removed or weakened. One `[DELTA-2026-09-04]` paragraph in `prd/project-scoping-phased-development.md` records that phase milestones are re-anchored to epic exit commands.
- **Architecture:** no ADR changed. §13 phased roadmap gets a dated note pointing at Epics 15–20. Epic 16 is the **first authorized non-zero kernel delta since 13.5j** (daemon RPC + sandbox application + crash routing); grant measured at landing per T0 precedent. All other epics **ZERO kernel-Δ @24472**.
- **UX:** N/A (kernel/CLI project).
- **Other artifacts:** `epics/index.md` (+6 entries, lane header), `epics/dependency-dag.md` (+recovery DAG), `epics/requirements-inventory.md` (+recovery coverage map), `sprint-status.yaml` (rename + 6 epic keys + 6 retro keys), decision register and `deferred-work.md` owner cells (re-pointed to the new keys — the gates check resolvability), `epic-14-retro-2026-09-04.md` and Appendix C tables (mapping note). CI: no workflow edit in this proposal; gate work is Epic 19-4's content.

**Technical impact.** Epic 16 touches `main.rs` (14,120 lines), `maos-control`, the CliWrapper spawn path and `maos-secrets`; it is the wave most likely to conflict with parallel sessions, so it holds the shared-write lock on `main.rs` first (J1-lane T0 rule).

## Section 3: Recommended Approach

| Option | Verdict | Effort | Risk |
|---|---|---|---|
| **1 · Direct Adjustment — six new epics in the existing structure** | **SELECTED** | High (≈17–24 weeks, 1–2 engineers + agents) | Medium: Epic 16 kernel delta; parallel-session write contention on `main.rs` |
| 2 · Rollback | Not viable — nothing shipped is wrong; the mechanisms are the raw material | — | — |
| 3 · MVP Review | Not needed — MVP (v0.1 J0) is *made true* by Epic 16 rather than reduced; no FR deferred | — | — |

**Rationale.** The waves follow the roadmap's own order; each closes on a command an operator runs on a clean machine, not on a green gate. Numbered epics cost one mechanical rename now and buy mechanical epic-close tracking, index coherence and retro discipline for free. Governance work rides inside epics at ≤20% (CP-2 rules), never as its own epic again.

## Section 4: Detailed Change Proposals

### 4.1 Epics and stories (NEW — six epic files under `epics/`)

Row key convention: `NN-M-slug`. AC1 of every story = the command and what the operator observes. Sizing 3–6 stories per epic (operator preference).

| Epic | Title | Stories (key → one line) | Exit command |
|---|---|---|---|
| **15** · W0 · 1 wk | Green at HEAD, one phase, first signed tag | 15-1 green-at-head (4 red gates + tracing race) · 15-2 single-phase-source · 15-3 first-signed-tag (+ hotfix lane: model-pin gate/sweep, model env overrides, error surfacing — keys unchanged) | `git tag v0.1.0-alpha.1 && gh release view v0.1.0-alpha.1` shows Ed25519-signed SHA256SUMS; aggregate green on tree content |
| **16** · W1 · 2–3 wk | One daemon, one door (J0 for real) | 16-1 daemon-rpc-control-plane · 16-2 apply-sandbox-and-resource-caps · 16-3 subprocess-crash-to-handle-crash · 16-4 kernel-uninstall-and-keyring-secrets · 16-5 j0-halt-on-ambiguity-scene · 16-6 audit-drop-legal-hold-and-a2a-deny-vocabulary (debt slot) | `cargo install maos && maos init && maos shell` → `@hello-spirit refactor …` halts → `maosctl pause hello-spirit` stops the **running** Spirit → `maos uninstall` |
| **17** · W2 · 3–4 wk | Spirits that think (v0.3 / v0.5 for real) | 17-1 butler-through-inference-port · 17-2 researcher-live-by-default-real-five-metric · 17-3 observer-launchable-live-telemetry · 17-4 three-more-hooks-and-wire-log-recall | `maos run spirits/butler/manifest.toml --live` runs Sandra's scene with a real model; `maos eval halt --class butler` prints recall/precision from real completions |
| **18** · W3 · 3–4 wk | The founder loop, end to end (v0.8) | 18-1 orchestrator-decomposes-via-inference-port · 18-2 real-worker-default-with-effect-oracle · 18-3 j1-halt-and-overnight-digest-beats | `cargo run -p xtask -- demo-j1 --live` with zero ABSENT beats |
| **19** · W4 · 4–6 wk | Ship it (v1.0) | 19-1 real-registry-publish-install · 19-2 packaging-from-release-workflow · 19-3 external-author-cohort-and-pen-test-scheduling · 19-4 gate-honesty-pass · 19-5 backends-and-multi-provider | `brew install lunarpulse/maos/maos && maosctl install researcher@1.0 && maos shell`; `v1.0.0` tag starts the LTS clock |
| **20** · W5 · 4–6 wk | Multi-host on live substrate (v1.5 → v2.2 reality) | 20-1 j4-corpus-to-mira-live · 20-2 j3-mesh-and-reza-in-nightly · 20-3 durable-tofu-and-self-leaf-rotation · 20-4 kernel-hygiene-one-instrument · 20-5 env-contract-registry-and-secrets | `cargo run -p xtask -- demo-reza --live` red if any DB missing; zero ignored journey tests in the nightly |

Per-story AC sketches live in the six epic files (created on approval); each story's full ACs are finalized by `bmad-create-story` at preflight.

### 4.2 `sprint-status.yaml`

- OLD `NN-M-slug: backlog` (26 rows) → NEW `(15+N)-M-slug: backlog` (W0 renumbered 1–3); keys for the three hotfix rows unchanged.
- NEW `epic-15` … `epic-20: backlog`; `epic-15-retrospective` … `epic-20-retrospective: optional`.
- Lane header comment rewritten to name Epics 15–20 and the mapping.

### 4.3 `epics/index.md`

- NEW lane header `—— Product-Spine Recovery Lane (ratified 2026-09-04; Epics 15–20, 26 stories) ——` and six `[Epic N: …](./epic-N-….md) — K stories, \`backlog\`` lines (the coherence gate parses this form).

### 4.4 `epics/dependency-dag.md` and `epics/requirements-inventory.md`

- DAG: `15 → 16 → 17 → 18 → 19 → 20` as the spine; intra-epic edges 16-1 → 16-5, 16-2 → 18-2, 17-4 → 20-2, 19-4 → 20-2; the hotfix lane parallel to 15.
- Inventory: NEW "Recovery coverage map (2026-09-04)" — each FR the review classified PARTIAL/CANNED/TEST-ONLY/ABSENT → the epic/story that makes it LIVE; journeys J0/J-Butler/J-Researcher/J1/J3/J4/Reza/J6 → 16/17/17/18/20/20/20/19.

### 4.5 PRD and architecture (notes only)

- `prd/project-scoping-phased-development.md`, after the "Phased delivery" paragraph: `[DELTA-2026-09-04]` — phases stand; each phase milestone is re-anchored to an epic exit command (Epic 16 ⇒ v0.1, 17 ⇒ v0.3/v0.5, 18 ⇒ v0.8, 19 ⇒ v1.0, 20 ⇒ v1.5–v2.2 reality); no FR moved.
- `architecture-maos-minimal-opus/13-phased-roadmap.md`, after the intro: dated note pointing at Epics 15–20 and the ZERO-Δ-except-Epic-16 posture.

### 4.6 Register, deferred-work, retro, appendices (mechanical re-point)

- Every `recovery-*` key in the decision register's Target/Deadline cells, in `deferred-work.md` Owner lines, in `epic-14-retro-2026-09-04.md` §4, in Appendix C and in `tracker-notes-archive-2026-09-04.md` → the new numeric key. `check-decision-register` and `check-dev-record-completeness` are the check.

## Section 5: Implementation Handoff

- **Scope: Major** → PM/Architect-level replan, executed here with the operator present; then **Developer agent** per story.
- **Sequence:** apply 4.2–4.6 mechanically → run `check-epic-close-coherence`, `check-decision-register`, `check-dev-record-completeness`, YAML parse → `bmad-create-story 15-1-green-at-head` first, model-pin hotfix rows in parallel.
- **Success criteria for the lane:** every PRD journey runs live or hermetic-by-choice; ≥50 of 66 FRs LIVE by the end of Epic 19; a `v1.0.0` tag exists; governance:product code ratio below 1:1; each epic closes on its exit command.
- **Rules carried (CP-2, binding):** AC1 = command; gate budget one-in-one-out (`xtask` ceiling not granted upward before Epic 19); a story splits once; story files ≤3,000 words, retros ≤2,000; one kernel-size instrument.
- **Not in this proposal:** re-litigating Epic 0–13 status; new FRs; GA holds (pen-test, export counsel stay on the GA ledger).

---

## Appendix A — Checklist status (Batch run)

| Section | Items | Status |
|---|---|---|
| 1 Trigger & context | 1.1 trigger story: the review itself (no single story); 1.2 problem statement; 1.3 evidence S1–S5 | [x] Done |
| 2 Epic impact | 2.1 Epic 14 not completable as planned → closed by re-scope; 2.2 six new epics; 2.3 Epics 0–13 untouched; 2.4 none obsolete, six needed; 2.5 resequenced by phase order | [x] Done |
| 3 Artifact conflicts | 3.1 PRD: no FR change, delta note; 3.2 architecture: no ADR change, Epic-16 kernel-Δ authorized; 3.3 UX N/A; 3.4 index/DAG/inventory/tracker/register/deferred-work | [x] Done (3.3 N/A) |
| 4 Path forward | 4.1 Direct Adjustment viable → SELECTED; 4.2 rollback not viable; 4.3 MVP review not needed; 4.4 rationale in §3 | [x] Done |
| 5 Proposal components | 5.1–5.5 = Sections 1–5 | [x] Done |
| 6 Final review & handoff | 6.1 complete; 6.2 verified against gate parsers; 6.3 **APPROVED by Lunarpulse 2026-09-04**; 6.4 tracker + epics + index + DAG + inventory + PRD/architecture notes APPLIED same day; 6.5 handoff = Developer agent per story, first `bmad-create-story 15-1-green-at-head` | [x] Done |


## Appendix B (APPROVED + APPLIED 2026-09-04): Epic 14 disposition — finish first, or correct course now?

**Question (Lunarpulse, 2026-09-04):** finish Epic 14 and then commit to the W0–W5 plan, or correct course immediately?

**Recommendation: correct course now.** Land the one story that is mid-flight (14-2c, in `review` by another session), close Epic 14 by re-scope with a short retro, and start W0 the same day. Do not open any further Epic 14 row under its current charter.

### Why finishing first is the expensive path

| Path | Calendar (at measured velocity) | What an operator can do at the end |
|---|---|---|
| **A. Finish Epic 14 as scoped** — 15 remaining rows (9 governance, 4 harness/defect, 2 feature) | 4–6 weeks. Epic 14 closed 5 rows in 9 days (08-26 → 09-03); Epic 13 closed 21 in 25 days. The remaining rows are the heavy ones (14-5 §A6 non-degradable, 14-7/8/9 static-scan, 14-6 ceiling). | Installers stop being scaffolds; maybe two more providers. Still: no verb reaches the daemon, no sandbox applied, no Spirit reasons, aggregate red unless someone fixes it as a side job, no tag. |
| **B. Correct course now** — land 14-2c, close by re-scope, W0 → W1 | W0 1 week; W1 complete around week 4. | Aggregate green and a signed alpha tag in week 1. By week 4: `cargo install maos && maos shell` → halt → `maosctl pause` stops a **running** Spirit → `maos uninstall`, with T2 sandbox actually applied. |

Three structural reasons beyond the calendar:

1. **Rework.** 14-7/14-8/14-9 (env registry over `main.rs`) and 14-6 (ceiling instrument) touch exactly the surfaces W1 restructures (`main.rs` 14,120 lines, one-shot dispatch, `env_contract.rs`). Doing them before W1 means doing a share of them twice.
2. **The remaining rows are the overshoot pattern.** 9 of the 15 add no operator-visible behavior. Continuing them re-commits to the ratio this review measured (governance:product 1.14:1).
3. **Sunk cost is not lost.** 14-1, 14-2, 14-2a delivered real mechanisms (live peer-cert rotation for signed cohort members, 10-host chaos floors). Nothing shipped is discarded; every remaining row keeps a home below.

### Row-by-row disposition (applied only on ratification)

| Epic 14 row | Status today | Disposition | Home |
|---|---|---|---|
| 14-2c plane-C local-leaf declaration | review (in flight, other session) | **Finish** — let it land, do not reopen scope | closes with Epic 14 |
| 14-2d self-identity rotation | blocked | Defer | W5-3 |
| 14-3 ecosystem-readiness ledger | backlog | Absorb | W4-4 |
| 14-4 sweep operational surfaces | backlog | Split: installers → W4-2; canary rollback + native push → post-W3 debt lane | W4-2 / debt |
| 14-5 backends + multi-provider | backlog | Defer; one additional provider may ride W2 if it costs < 1 day | W2 (opt) / W4 |
| 14-6 constitutional ceiling + formal methods | backlog | Absorb | W5-4 |
| 14-7 / 14-8 / 14-9 env-contract registry, census, secrets | backlog | Absorb (W0-1 registers only the 2 vars that red the gate today) | W5-3 |
| model-pin lane (3 rows, approved 2026-09-01) | backlog | **Do now** | W0-3 |
| 14-d3 audit-drop / legal-hold serialization | backlog | Keep as a **defect**, not governance: `record_drop` can lose an audit event while returning `Ok` (I2). Debt lane, first slot | W1 debt lane |
| 14-d4a region-home boot reconciliation | backlog | Defer (multi-region only) | W5-2 |
| 14-e1 erasure-attestation honesty | backlog | Keep as a defect; GDPR-tier | W4 debt lane |
| e12-b1 gate-binding residual | backlog | Absorb | W4-4 |
| v25-* (3 rows) | backlog | Unchanged (v2.5) | v2.5 |
| epic-14-retrospective | optional | **Write it, ≤2,000 words**: what 14-0…14-2c proved, what was re-homed and why | closes Epic 14 |

### Mechanics of the close (so the coherence gate stays green)

- `epic-14` → `done` with the note `closed by re-scope 2026-09-04 (sprint-change-proposal-2026-09-04.md §6)`; each re-homed `14-*` row gets status `deferred` plus a `# → NN-M` pointer rather than being deleted, so `check-epic-close-coherence` and the decision register (D-rows that name these vehicles) keep resolving.
- 14-2c must reach `done` before the epic key flips; if its review stalls, the epic waits on that one row and nothing else.
- Retro is written at the close, per the "mechanical gates compound, promises decay" rule: the re-homing table above is copied into the retro verbatim so it cannot drift.

### Decision requested

Ratify (a) close-by-re-scope of Epic 14 after 14-2c, (b) the disposition table above, (c) W0 start immediately (`15-1-green-at-head` first, model-pin lane in parallel). Nothing in this section changes code.

---


## Appendix C (APPROVED + APPLIED 2026-09-04): Tracker cleanup during the course correction (what is removed, renamed, merged, kept)

**Question (Lunarpulse, 2026-09-04):** on course-correction, do the added Epic 14 rows and everything below line 300 of `sprint-status.yaml` get removed? Can contradictory or useless rows be cleaned up in the same move?

**Answer: yes, and this is the right moment, but "remove" must respect the three machines that read the file.** Measured at HEAD:

| Reader | What it enforces | Consequence for cleanup |
|---|---|---|
| `check-epic-close-coherence` | `epic-N: done` ⇒ every `N-*` row is `done` (`check_epic_close_coherence.rs:138-140`); `epics/index.md` story count and status must agree with the rows | A `14-*` row cannot be `deferred`; to close Epic 14 it is either `done`, **renamed out of the `14-` namespace**, or deleted. `index.md` "17 stories" must be re-derived. |
| `check-decision-register` | every register Target cell resolves to a real `development_status` key, exact or unique-prefix (`check_decision_register.rs:26-32,320-326`) | A row named by a D-row is **renamed, never deleted**, and the register cell is updated in the same edit. The gate catches any miss. |
| dev-record / review-findings / dev-model gates | walk `done` and `review` rows only | `backlog` rows without story files are fine (hotfix-lane precedent). |

Comments are not parsed by any of them. The `development_status` block today carries **316 KB of comments against 13.6 KB of keys and statuses (23 : 1)**; 14-2c alone moved to `done` at 10:18 UTC today with a 2 KB comment.

### 7.1 Rules

1. **Delete** a row only when nothing in the register or another story names it.
2. **Rename + re-home** a row the register names: new key under `NN-M-…`, register cell updated, old story file kept as the design record and pointed to from the new row's one-line comment.
3. **Merge** rows that describe the same seam (a contradiction is two rows for one fix).
4. **Comment diet, file-wide:** every row comment becomes one line ≤160 chars: status word, date, pointer to the story file. Long comments are moved verbatim into the story file's new `## Tracker note` section (or, if no story file exists, into `implementation-artifacts/tracker-notes-archive-2026-09-04.md`). Nothing is lost; the tracker stops being a second copy of the story.
5. **Epic 14 closes as `done` "by re-scope"** with the six delivered stories (14-0, 14-1, 14-2, 14-2a, 14-2b, 14-2c) and a required retro ≤2,000 words; `index.md` says "6 stories (11 re-homed 2026-09-04, §7)".

### 7.2 Row-by-row (lines 299–368 as of 10:18 UTC)

| Row today | Verdict | Why | Becomes |
|---|---|---|---|
| `spec-epic-5-review-finding-closure: blocked` | **Rename + merge** | A spec, not a story; blocked only on gate greenness, which W0-1 / W4-4 own. Named by the register and two J1 stories. | folded into `20-3-gate-honesty-pass`; register cell → `19-4` |
| `14-0`, `14-1`, `14-2`, `14-2a`, `14-2b`, `14-2c` (done) | **Keep** | Delivered. | Epic 14's six stories |
| `14-2d-self-identity-rotation: blocked` | **Merge** | Same seam as `20-3`; two rows for one fix. Story file stays as the design record. | `21-3-durable-tofu-and-self-leaf-rotation` |
| `14-3-…-v2-5-graduation-ledger` | **Merge** | Contradicts the operator rule that v2.5 is non-gating and never scheduled; its only engineering content is ledger honesty. | `19-4` |
| `14-4-…-operational-surfaces` | **Split** | Installers are W4; canary auto-rollback (NFR-Rel-5) and native push have no journey dependency. | installers → `19-2`; canary + push → v2.5 parking row |
| `14-5-…-backends-multi-provider` | **Rename** | Real FR3 gap, but after Spirits reason. | `20-4-gemini-driver-and-endpoint-pin` |
| `14-6-…-constitutional-ceiling` | **Merge, premise corrected** | Cites a ≤25K `kernel-crate-set.toml` / ADR-057 ceiling that exists in no ADR and no file; the row's premise is false. D11/D13 targets updated. | `21-4-one-instrument-and-env-registry` |
| `14-7`, `14-8`, `14-9` env-contract rows | **Merge into one** | Three rows for one registry; W0-1 registers only the two vars that red the gate today. D4b/D4c/D14 targets updated. | `21-4-one-instrument-and-env-registry` |
| `model-pin-…`, `provider-model-env-overrides`, `provider-error-surfacing` | **Keep** | Approved 2026-09-01; they are W0-3. | unchanged (hotfix lane) |
| `14-d3-audit-drop-and-legal-hold-serialization` | **Rename** | Not governance: `record_invocation` can drop an audit event and return `Ok` (I2). Keep as a defect in W1's debt slot. D3/D5 targets updated. | `16-6-audit-drop-and-legal-hold-serialization` |
| `14-d4a-region-home-boot-reconciliation` | **Merge** | Multi-region only; same env/secrets seam. D4a target updated. | `20-5` |
| `14-e1-erasure-attestation-honesty` | **Merge** | Same rule as gate honesty: a control may not assert what it did not observe. D5.2/5.3 targets updated. | `19-4` (named AC) |
| `e12-b1-gate-binding-decay-residual` | **Merge** | Governance residual; D20 target updated. | `19-4` |
| `v25-*` (3 rows) | **Keep, regroup** | v2.5 by operator rule; D5.4 / D6 / 13.5e-D2 targets. | under a `# === v2.5 PARKING ===` header, with the canary/push row |
| `epic-14-retrospective: optional` | **Make required** | D17 anchors a deadline to it; `optional` makes that deadline unqueryable (gate output today). | `backlog` → `done` at close |
| `15-3-model-pin-lane` | **Delete** | My own pointer row duplicating the three hotfix rows. Nothing names it. | — |
| `recovery-w*` (23 others) | **Keep** | The lane. | W0 3 · W1 6 (one debt) · W2 4 · W3 3 · W4 5 · W5 5 |

Net: rows below line 299 go from 49 to 43 (6 done Epic-14 + retro, 3 hotfix, 26 recovery, 4 parking); the file's comment volume falls by roughly twenty times; every register target still resolves; `index.md` re-derived. Nothing that has a D-row loses its key without its D-row being updated in the same commit, and the gate is the check.

### 7.3 What is *not* cleaned up here

- Rows above line 299 (Epics 0–13, J1 lane): statuses untouched; only the comment diet applies to them.
- Story files: none deleted. Re-homed rows keep their files as design records.
- Register rows with UNQUERYABLE deadlines (D3, D17, D18, D19 per today's gate output): re-anchored to recovery keys in the same edit, not closed by fiat.

### 7.4 Execution

Applied 2026-09-04 in one mechanical pass (Python, run once, diff reviewed). Verified: YAML parses (215 rows); `check-epic-close-coherence` PASSED; `check-decision-register` 23 rows / 15 open with only the pre-existing D19 note; `check-review-findings-resolved`, `check-bare-review-findings`, `check-dev-model-tier`, `check-dev-model-used-populated` PASS. A fourth reader not listed above — `check-dev-record-completeness` validates `deferred-work.md` owner keys — required 15 owner lines to be re-pointed to the new keys; its one remaining violation (`deferred-work.md:860`) predates this cleanup and belongs to the 14-2c commit `d09420e2`. Comment text moved to `tracker-notes-archive-2026-09-04.md` (171 entries, verbatim) rather than into individual story files, because a parallel session was writing story files at the time.
