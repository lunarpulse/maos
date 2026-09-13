# Sprint Change Proposal — Round 2 (2026-09-04): Raising the Confidence of the Recovery Lane

> **Workflow:** bmad-correct-course (Batch) · **Operator:** Lunarpulse (present) · **Trigger:** `epics/epic-15-20-preflight-2026-09-04.md` (confidence 8–35 per epic; ~35 false premises; 40–58 measured weeks vs 17–24 planned).
> **Input package:** `correct-course-input-2026-09-04-confidence.md` (four failure shapes, eight confidence rules, Spirit-form fork A/B/C, 7-epic re-plan).
> **Operator decisions this round:** Batch · **Fork C — forms by trust tier** · eight confidence rules binding · 7-epic shape (15 Foundations, 16–21) · one-time kloc ceiling re-base as 15-2 · `v1.0.0` stays `-rc` until GA Holds 1–2 clear.
> **APPROVED and APPLIED 2026-09-04.**
> **Change scope classification: Major** — restructures the recovery lane from 6 to 7 epics, adds a foundations epic, narrows two FRs by ADR, and re-points the decision register.

---

## Section 1: Issue Summary

**Problem.** The morning's ratified Epics 15–20 cannot be delivered as written. Six read-only preflight scouts, run one per epic against committed `HEAD d09420e2`, found that the exit commands name absent verbs (`demo-j1 --live`, `demo-reza --live`, `maos eval`, `maos uninstall`, `--epic`, `--replay`), that five stories assume a subprocess Spirit host that was never built, that four ACs put paid, human or wall-clock work inside an engineering exit, that six inventory counts were wrong, and that eight crates at zero kloc headroom with an equality kernel pin block every story from landing a line.

**Discovery.** Preflight 2026-09-04 (scouts sequential after a rate-limit stall; reports in the session record; verdicts in the preflight file).

**Category (checklist 1.2):** *misunderstanding of original requirements* — the plan restated PRD promises as if their substrate existed — compounded by *technical limitation discovered*: the v0.1 subprocess Spirit form is absent from the tree.

**Evidence (checklist 1.3):** the preflight file §1 (per-epic table with `file:line`), §2.1 (`SpiritForm = {NativeSubprocess (CliWrapper only), WasmComponent}`, all ten manifests `forms=["rust-inproc"]`, `sdks/spirit-ts` self-described test harness), §2.2 (`kloc.toml` zero-headroom rows, `check_kernel_baseline.rs:7`, `kloc.toml:501`), §2.5–§2.8.

## Section 2: Impact Analysis

**Epic impact.** Epics 15–20 (all `backlog`, NEEDS-REWORK) are **replaced** by Epics 15–21: a new foundations epic absorbs the five blockers; former 16-2 splits into Worker-only sandboxing under Epic 17; former 17-4's "wire recall" is re-based on the existing WASM host (Fork C); former 20 becomes 21. Epics 0–14: untouched. The hotfix lane and `v25-*` parking: untouched.

**Story impact.** 26 rows → 34 engineering rows + 4 operator-lane rows + 1 parked row (mapping in §4.2). Every AC is rewritten from the scouts' corrected wording with a `file:line` citation and a kernel-Δ statement; ACs without a citation are cut.

**Artifact conflicts (checklist 3.1–3.4).**
- **PRD:** no FR added or removed. Two FRs are **narrowed by ADR, honestly**: FR5 (sandbox enforcement) applies *by form* — WASM by construction, CliWrapper by profile, first-party in-process code trusted as kernel distribution; FR33 (per-language templates) is served by WASM toolchains (TS via componentize-js now; Python via componentize-py later). A `[DELTA-2026-09-04 R2]` paragraph records both.
- **Architecture:** four ADRs authored in 15-5 (forms by trust tier; Worker egress + credential model; mutating operator surface trust path; revocation model). ADR-031's "WASM is the second Spirit form" is affirmed; the roadmap's rust-inproc retirement note is amended (first-party stays in-proc). Kernel pin stays 24472 until 15-2's re-base is ratified at landing; Epics 16-5, 17-1, 21-5 are the named FLAG-Winston deltas.
- **UX:** N/A.
- **Other artifacts:** seven epic files (six replaced, one new), `epics/index.md`, `dependency-dag.md`, `requirements-inventory.md` (coverage map re-pointed), `sprint-status.yaml` (rename + new epic/retro keys + operator-lane section), decision register and `deferred-work.md` owner cells, `epic-14-retro-2026-09-04.md` §4, morning proposal Appendix C, tracker archive, preflight file (pointer). New xtask leg `check-exit-commands` (≤60 lines) with one absent-pass gate retired (15-3).

**Technical impact.** Epic 15 touches `xtask/kloc.toml` (re-base), `release.yml`, `main.rs` (replay seam), gate TOMLs; Epic 16 touches `maos-control` (POST parser), `maos-cli` (19 verbs), `maos-shell` (halt registry), 13 test sites; Epic 17 touches kernel `t3/argv.rs` (egress allowlist, FLAG-Winston), `maos-wasm-host` + WIT (ABI extension, `abi-diff` + ratification), `resource_ceiling.rs`. The shared-write lock on `main.rs` belongs to Epic 16 first.

## Section 3: Recommended Approach

| Option | Verdict | Effort | Risk |
|---|---|---|---|
| **1 · Direct Adjustment under the eight rules and Fork C** | **SELECTED** | 28–40 weeks engineering (measured + 30%), operator lane parallel | Medium: three FLAG-Winston deltas, one WIT ABI extension, three spikes |
| 2 · Rollback | Not viable — nothing shipped is wrong | — | — |
| 3 · MVP Review | Partially adopted: FR5/FR33 narrowed by ADR rather than by silence; MVP (J0) made true in Epic 16 | — | — |

**Rationale.** Confidence moves when the four failure shapes are made structurally impossible: verbs must exist (rule 1, mechanical), exits must be hermetic (rule 2, a CI job from day one), foundations land first (rule 3, an epic), every AC cites the code it changes (rule 4). Fork C reuses the one out-of-process form that already exists and is gated, so the sandbox claim becomes true by construction for exactly the code the PRD worries about, without a new runtime.

## Section 4: Detailed Change Proposals

### 4.1 Epics (seven files under `epics/`; full ACs in the files)

| Epic | Title · wave | Stories | Hermetic exit (CI, no secrets) | Operator-lane rows | Measured |
|---|---|---|---|---|---|
| **15** | Foundations · W0 | 15-1 green-at-head · 15-2 kloc-ceiling-rebase · 15-3 single-phase-source-and-exit-command-check · 15-4 release-repair-and-first-signed-tag · 15-5 decision-adrs-and-provisioning-checklist · 15-6 inference-replay-seam | aggregate green; `check-exit-commands` green; `v0.1.0-alpha.1` builds signed artifacts incl. `maosctl` | `ops-provisioning-secrets-and-accounts` | 2–3 wk |
| **16** | One daemon, one door (J0) · W1 | 16-1 daemon-post-surface-and-verb-retarget · 16-2 shell-halt-registry-and-j0-scene · 16-3 subprocess-crash-to-handle-crash · 16-4 maos-uninstall-and-keyring · 16-5 audit-drop-legal-hold-and-a2a-deny-vocabulary | `maos init && MAOS_INFERENCE_MODE=replay maos shell` → halt → `maosctl halt resolve` → `maos run butler &` + `maosctl pause butler` visible in `spirit inspect` → `maos uninstall` | live-key J0 | 4–5 wk |
| **17** | Workers and the third-party form · W2 | 17-1 worker-egress-allowlist-and-scoped-credential · 17-2 worker-cgroups-applied · 17-3a wasm-recall-and-componentize-spike · 17-3b wasm-third-party-form-with-log-recall · 17-4 hooks-observer-and-activity-corpus | fixture Worker probing `~/.ssh` → EACCES row in `maos audit query`; `maos run examples/ts-spirit.wasm --once` recalls its own frames | live claude/codex under the profile | 4–6 wk |
| **18** | Spirits that think · W3 | 18-1 butler-inference-seam-and-conformant-mcp · 18-2 eval-halt-verb-and-corpus-numbers · 18-3 researcher-replay-live-and-judged-five-metric · 18-4 self-tuning-halt-with-injectable-clock | `MAOS_INFERENCE_MODE=replay maos run spirits/butler/manifest.toml --once` proposes; `maos eval halt --class butler` prints recall/precision | nightly re-record; Google OAuth MCP | 4–6 wk |
| **19** | Founder loop · W4 | 19-1 orchestrator-dispatch-loop-and-epic-input · 19-2 fr20-enqueue-door-and-safe-point · 19-3 real-worker-default-with-effect-oracle · 19-4 j1-beats-and-demo-replay | `cargo run -p xtask -- demo-j1 --replay`, ABSENT = {two-host-signed-run} | `--live-codex`; two-host signed run (2d lane) | 4–6 wk |
| **20** | Ship it · W5 | 20-1 registry-client-install-verb-vetter-and-yank · 20-2 deb-airgap-docker-and-formula-corrections · 20-3 gate-honesty-pass · 20-4 gemini-driver-and-endpoint-pin · 20-5 v1-0-rc-tag-and-lts-rule | clean VM: signed release → self-hosted registry → `maosctl spirit install researcher@1.0` → `maos shell`; `v1.0.0-rc.1` | `ops-brew-tap-and-aur-publication`, `ops-external-cohort-and-pen-test`, `ops-ga-tag-after-holds` | 5–7 wk |
| **21** | Multi-host on live substrate · W6 | 21-1 j4-incident-fixture-and-live-mira-nash · 21-2 nightly-postgres-and-ignored-journeys · 21-3 durable-tofu-and-self-leaf-rotation · 21-4 one-instrument-and-env-registry · 21-5 orchestrator-port-and-move (optional, FLAG-Winston) | `cargo run -p xtask -- demo-reza` + nightly with zero ignored journey tests except those with a named owner | live J4 (300+ calls) | 5–7 wk |

### 4.2 Row mapping (old → new; register and deferred-work cells follow)

| Old (morning) | New | Note |
|---|---|---|
| 15-1 green-at-head | 15-1 green-at-head | ACs corrected (16 symbols, baseline regen, I9 attribute move + re-pin) |
| 15-2 single-phase-source | 15-3 single-phase-source-and-exit-command-check | 8 constants → 1; adds `check-exit-commands` |
| 15-3 first-signed-tag | 15-4 release-repair-and-first-signed-tag | artifact merge, `maosctl` artifact, aarch64 leg, publish needs aggregate |
| — | 15-2 kloc-ceiling-rebase · 15-5 decision-adrs-and-provisioning-checklist · 15-6 inference-replay-seam | new foundations |
| 16-1 daemon-rpc-control-plane | 16-1 daemon-post-surface-and-verb-retarget | 19 verbs, token minting, 0 kernel-Δ |
| 16-5 j0-halt-on-ambiguity-scene | 16-2 shell-halt-registry-and-j0-scene | halt registry + `EpistemicHalt` frame |
| 16-3, 16-4 | 16-3, 16-4 | keys unchanged (16-4 renamed slug: `maos-uninstall-and-keyring`) |
| 16-6 audit-drop… | 16-5 audit-drop-legal-hold-and-a2a-deny-vocabulary | kernel lines, FLAG-Winston (D3/D5/D7/D18) |
| 16-2 apply-sandbox-and-resource-caps | 17-1 worker-egress-allowlist-and-scoped-credential + 17-2 worker-cgroups-applied | Workers only (Fork C) |
| 17-4 three-more-hooks-and-wire-log-recall | 17-3a spike + 17-3b wasm-third-party-form-with-log-recall; hooks → 17-4 | WASM, not a new host |
| 17-3 observer-launchable-live-telemetry | 17-4 hooks-observer-and-activity-corpus | merged |
| 17-1 butler-through-inference-port | 18-1 butler-inference-seam-and-conformant-mcp + 18-4 self-tuning-halt-with-injectable-clock | split |
| 17-2 researcher-live-by-default… | 18-3 researcher-replay-live-and-judged-five-metric | flags exist first |
| — | 18-2 eval-halt-verb-and-corpus-numbers | new verb story |
| 18-1, 18-2, 18-3 | 19-1, 19-3, 19-4 | `MAOS_DELEGATED_GOAL`, not `MAOS_WORKER_TASK`; `demo-j1 --replay` |
| — | 19-2 fr20-enqueue-door-and-safe-point | new |
| 19-1, 19-2, 19-4, 19-5 | 20-1, 20-2, 20-3, 20-4 | 20-4 = Gemini + pin only; Bedrock/Vertex/Vault → `later-bedrock-vertex-and-kms-backends` |
| 19-3 external-author-cohort-and-pen-test-scheduling | `ops-external-cohort-and-pen-test` (operator lane) | not an engineering exit |
| — | 20-5 v1-0-rc-tag-and-lts-rule | new |
| 20-1, 20-2, 20-3 | 21-1, 21-2, 21-3 | 21-3 keeps `maos-persistence` as the durable pin home (resolves the 20-3/20-4 conflict) |
| 20-4 + 20-5 | 21-4 one-instrument-and-env-registry | "consolidate canonical/SigningKey" dropped (false premise) |
| — | 21-5 orchestrator-port-and-move | optional, FLAG-Winston, negative re-pin |

Decision-register re-points: D3/D5/D7/D12/D18 → 16-5; D1/D8/D9/D10/D20 (and D5's gate half) → 20-3; D4a/D4b/D4c/D14/D11/D13 → 21-4; D17 stays 15-1.

### 4.3 PRD and architecture

- `prd/project-scoping-phased-development.md`: `[DELTA-2026-09-04 R2]` — Fork C recorded; FR5 by form; FR33 via WASM toolchains; 7 epics; 28–40 weeks.
- `architecture-maos-minimal-opus/13-phased-roadmap.md` §13.1 disposition note: rust-inproc is **not** retired for first-party Spirits; WASM is the third-party form (ADR-031 affirmed); four ADRs named for 15-5.

### 4.4 Tracker and governance

- `sprint-status.yaml`: replace the 15–20 block with 15–21 (31 rows), epic + retro keys, a `# === OPERATOR LANE ===` section (4 rows) and one `later-*` row; the eight confidence rules and Fork C in the lane header.
- `check-exit-commands` (15-3): parses each `epic-*.md` fenced exit block and resolves `maos`/`maosctl`/`xtask` tokens against `--help`; enrolled Blocking; `check-cna-registration` (file-presence only) retired in exchange.

## Section 5: Implementation Handoff

- **Scope: Major** → applied here with the operator present; then Developer agent per story.
- **Sequence:** apply 4.1–4.4 → `check-epic-close-coherence`, `check-decision-register`, `check-dev-record-completeness`, YAML → `bmad-create-story 15-1-green-at-head`; 15-2 and 15-5 need operator ratification *at landing* (measured re-base; four ADRs).
- **Success criteria:** each epic's hermetic exit is a green CI job before the epic closes; expected confidence after rewrite 15·75 / 16·70 / 17·60 / 18·65 / 19·60 / 20·55 / 21·55; lane 28–40 weeks.
- **Rules carried:** the eight confidence rules (input package §2) supersede CP-2 of the morning proposal where they overlap.

---

## Appendix A — Checklist status (Batch)

| Section | Status |
|---|---|
| 1 Trigger & context | [x] preflight file + input package |
| 2 Epic impact | [x] 15–20 replaced by 15–21; 0–14 untouched; resequenced by foundations-first |
| 3 Artifact conflicts | [x] PRD delta (FR5/FR33 narrowed by ADR); architecture (4 ADRs, ADR-031 affirmed); UX N/A; tracker/index/DAG/inventory/register |
| 4 Path forward | [x] Direct Adjustment under rules + Fork C |
| 5 Proposal components | [x] Sections 1–5 |
| 6 Final review & handoff | [x] 6.3 APPROVED by Lunarpulse 2026-09-04; 6.4 applied the same day (7 epic files, tracker 15–21 + operator lane, index/DAG/inventory/PRD/architecture, register + deferred-work re-pointed) |
