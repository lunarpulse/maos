# Sprint Change Proposal — Product-Spine Recovery Lane (W0–W5)

> **Date:** 2026-09-04 · **Trigger:** full fresh promise-vs-reality review at HEAD `4657cace` (report: *MAOS Promise Ledger*, https://claude.ai/code/artifact/e9b88de1-d01d-4e6b-94c9-932810ca82c5; evidence: `review-2026-09-04/`).
> **Status:** PENDING Lunarpulse ratification. Rows are registered in `sprint-status.yaml` as `backlog` scope text; no story file is authored and no code is touched until ratified.
> **Change scope classification: Major** — re-sequences the remaining roadmap around an operator-runnable product path; Epic 14's governance rows continue only inside a capped debt lane.

---

## Section 1: Issue Summary

**Problem.** The tracker records Epics 0–13 `done` and RELEASE-HOLDS.md says "All PRD journeys served." Measured at HEAD: 22 of 66 FRs are live from a product binary; 0 release tags in 319 commits; 1 of 11 Spirits calls the Inference Port (hello-spirit); every reference Spirit implements 1 of 14 hooks; the discipline `aggregate` job is red on four gates plus one deterministic test red from a `done` story. "Done" is true at the harness layer and false at the operator layer.

**Five seams (each closes a column of the FR table):**

| # | Seam | Receipt |
|---|---|---|
| S1 | Control plane never reaches the daemon — every `maosctl` verb but one spawns a fresh `maos` via `MAOS_ONE_SHOT` on an empty registry | `maos-cli/src/subcommands.rs:1461-1842`; `maos-bin/src/main.rs:2740-2749` |
| S2 | Sandbox/cgroups computed, never applied — `spawn_sandboxed` zero prod callers; bare `Command::spawn` with host creds | `kernel-core/src/lifecycle/cli_wrapper/runtime.rs:461-473`; `security/mod.rs:39,485`; `scheduler/resource_ceiling.rs:76-138` |
| S3 | One Spirit thinks — Butler never imports InferencePort; Observer unlaunchable; `fire_on_consolidate`/`fire_on_telemetry_event` no callers | `spirits/butler/src/lib.rs:390-464`; `main.rs:447-454`; `scheduler/hook_dispatch.rs:285-345` |
| S4 | Gates judge files — 12 evidence-file gates pass on absence; Aud-7 gate averages its own fixture labels; 3 `CURRENT_PHASE` sources disagree | `maos-eval/tests/distillate_five_metrics_floor.rs:53-81`; `gate_common.rs:166` vs `tests/phase-config.toml:5` vs `check_escape_detector.rs:62` |
| S5 | Never shipped — `release.yml` never ran; packaging scaffolds; retired model pins; 401/429 masked as "transport"; secrets env-only | `release.yml:70-93`; `packaging/homebrew/maos.rb:25`; `spirits/butler/manifest.toml:21`; `io/mod.rs:105-108` |

**Overshoot (numbers, no editorial):** governance:product code 1.14:1 (2.85:1 with tests); `xtask` 42K LOC = 2.3× kernel, 72 gate modules, 160 CI jobs; prose:Rust 2.15:1 by words; 148 story files vs 36–60 planned (66 splits); Epic 14 = 13 governance/harness rows of 24; kernel pin +58% since 8.4 across 31 re-pins with two instruments 5.5K lines apart.

## Section 2: Impact Analysis

- **Epic 14:** not invalidated; its FEATURE rows (14-2a done, 14-2d, 14-4, 14-5, 14-d3, 14-d4a) map onto W1/W4/W5 below. Its GOVERNANCE rows (14-3, 14-6, 14-7, 14-8, 14-9, e12-b1, 14-e1) move into the capped debt lane (≤20% of any wave). No Epic-14 row is deleted.
- **Model-pin hotfix lane (2026-09-01, approved):** unchanged; it IS W0-3.
- **PRD/architecture:** zero new FRs; the phase order (v0.1→v0.3→v0.5→v0.8→v1.0→…) is kept and *executed* rather than re-planned. NFR floors are not relaxed; two already-relaxed floors (Rel-7, Cost-1) stay as amended.
- **Kernel-Δ:** W1 (S1/S2/crash routing) is the first authorized non-zero kernel delta since 13.5j; grant is measured at landing per T0 precedent. All other waves default ZERO-Δ.
- **Release holds:** unaffected; GA still gates on pen-test + export counsel. Alpha tags do not need them.

## Section 3: Recommended Approach

**Option: waves closed by a demo command (SELECTED).** Each wave's exit is one command an operator runs on a clean machine, not a green gate. Rejected: (a) continue Epic 14 as ordered — 13 of its 24 rows add no operator-visible behavior; (b) big-bang "v1.0 hardening" — repeats the pattern that produced 18 claim boundaries.

## Section 4: Detailed Change Proposals

### CP-1 — Lane rows in `sprint-status.yaml` (applied, backlog, scope text only)

Waves and demo commands:

| Wave | Exit command | Closes |
|---|---|---|
| **W0** (1 wk) | `git tag v0.1.0-alpha.1 && gh release view v0.1.0-alpha.1` shows signed SHA256SUMS; aggregate green on tree content | 4 red gates + 1 test red; one phase source; model-pin lane; first tag |
| **W1** (2–3 wk) | `cargo install maos && maos init && maos shell` → `@hello-spirit refactor …` halts → `maosctl pause hello-spirit` stops the *running* Spirit → `maos uninstall` | S1, S2, S5 (J0) |
| **W2** (3–4 wk) | `maos run spirits/butler/manifest.toml --live` runs Sandra's scene with a real model; `maos eval halt --class butler` prints recall/precision from real completions | S3 (v0.3/v0.5) |
| **W3** (3–4 wk) | `cargo run -p xtask -- demo-j1 --live` with zero ABSENT beats | J1 (v0.8) |
| **W4** (4–6 wk) | `brew install … && maosctl install researcher@1.0 && maos shell` | S4, shipping, ecosystem (v1.0 tag, LTS clock) |
| **W5** (4–6 wk) | `cargo run -p xtask -- demo-reza --live` red if any DB missing; zero ignored journey tests in nightly | J3/J4/Reza live; TOFU durability; kernel hygiene |

### CP-2 — Governance diet (binding on every wave; mechanical where a gate exists)

1. AC1 of every story is a command plus what the operator observes. Stories with no operator-visible effect go to the debt lane, capped at 20% of a wave.
2. Gate budget: `xtask` kloc ceiling is NOT granted upward until W4 (it is at 7 lines headroom — the freeze is already mechanical). New gate ⇒ retire one.
3. Story splits once; a second split is a scope cut.
4. Story files ≤3,000 words; retros ≤2,000; decision-register rows with no code owner after two waves are deleted.
5. One kernel-size instrument (tokei code), one ceiling in one ADR; grants measured at landing.

### CP-3 — Deferred to story authoring

Story files via `bmad-create-story` per lane convention. Kernel grant for W1 measured in-story. The `≤25K kernel-crate-set` ceiling cited in `sprint-change-proposal-2026-07-13.md:35` has no ADR and no file; W5-4 either authors it or retires the citation.

## Section 5: Implementation Handoff

- **Scope: Major → operator ratification required** before any W0 code. Suggested first move after ratification: `bmad-create-story recovery-w0-1-green-at-head` (mechanical, ~1–2 days).
- **Success criteria for the lane:** every PRD journey runs live or hermetic-by-choice; ≥50 of 66 FRs LIVE by W4; a `v1.0.0` tag exists; governance:product code ratio falls below 1:1.
- **Not in this proposal:** re-litigating Epic 0–13 status; new FRs; GA holds.

### Appendix — Checklist status

| Section | Status |
|---|---|
| 1 Trigger & context | [x] review report + evidence dir |
| 2 Epic impact | [x] Epic 14 re-homed, not invalidated |
| 3 Artifact conflicts | [x] PRD/architecture unchanged; kernel-Δ authorized only at W1 |
| 4 Path forward | [x] waves-by-demo selected |
| 5 Proposal components | [x] CP-1 rows applied as backlog; CP-2 rules; CP-3 deferrals |
| 6 Final review & handoff | [ ] **awaiting Lunarpulse** |
