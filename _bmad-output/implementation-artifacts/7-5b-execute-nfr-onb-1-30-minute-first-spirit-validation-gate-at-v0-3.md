---
dev_model_used: claude-opus-4-8
---

# Story 7.5: Execute NFR-Onb-1 30-Minute First Spirit Validation Gate at v0.3 (7.5b)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the human-research lead validating MAOS's onboarding floor,
I want the **NFR-Onb-1 30-Minute First Spirit Validation Gate execution infrastructure** built and CI-wired at the v0.3 release — the recruitment/screener/support-log protocol artifacts, the three-door page at `docs.maos.dev`, a mechanical outcome-scoring harness + cohort gate evaluator (≥10/12 succeed, median ≤45 min, p95 ≤90 min, halt-recall ≥0.90 on the calendar-conflict subset, halt-precision ≥0.85 overall), the NFR-Onb-4 iteration-cadence machinery, and one end-to-end dry-run self-trial proving the harness is correctly wired,
So that the v0.3 release criterion is met via **reproducible human-trial evidence — not a vibe** — and the N=12 stratified human trial can run out-of-band against artifacts whose results are scored deterministically by CI.

## What this story IS and IS NOT (read first — scope is deliberately bounded)

This story was scoped by two **locked decisions** (see next section). It is the single most "this is not pure code" story in the project, so the boundary is drawn explicitly to prevent the dev agent from either over-building (recruiting humans) or under-building (skipping the harness).

**This story IS:**
- The complete, reproducible **gate-execution infrastructure**: protocol doc, screener, three-door page, stratification validator, outcome-scoring harness, cohort gate evaluator, NFR-Onb-4 cadence machinery, xtask gate + CI arm + gate-registry registration.
- A **Butler-corpus seam**: the scoring harness reads a *configurable corpus path* that defaults to a 7.5b-owned, SHA-pinned **fixture** corpus and auto-prefers the real `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` the instant Story 8.1 lands it. (Decision 1.)
- **One end-to-end dry-run self-trial**: the dev agent scaffolds ONE Spirit via the Story 2.3 `cargo generate` template, scores it against the fixture corpus through the harness, emits one `outcomes.jsonl` row, and runs the gate evaluator on that N=1 sample to prove the wiring. (Decision 2 — this is a smoke proof, NOT the N=12 gate.)

**This story IS NOT:**
- It does **NOT** recruit 12 real human participants or run a 14-day trial. That is an out-of-band human-research activity that *consumes* this story's artifacts. The ACs are written as **"the harness enforces / produces X"**, never "humans did X."
- It does **NOT** author the real Butler-class regression corpus or the Butler Spirit. Story 8.1 owns `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` exclusively ("no other story authors this corpus" — epic-8 Story 8.1 AC). 7.5b ships a clearly-labelled **fixture stand-in** at a 7.5b-owned path and a seam, never the canonical corpus.
- It does **NOT** ship the WCAG-AA-polished public site. The three-door page ships **functional** at v0.3; WCAG AA + full canonical-docs publication is **deferred to Story 9.5** (per NFR-Onb-3 note in the epic AC).
- It does **NOT** add a new workspace crate. Workspace count **stays at 30** (reuse `maos-eval` for the scoring corpus, `xtask` for the gate). Adding a crate trips `check-workspace-count` — do not do it without flagging Winston.

## LOCKED Design Decisions (do NOT silently re-decide — these were chosen by the research/architecture lead during story creation)

**Decision 1 — Butler forward-dependency → SEAM + SHA-pinned fixture corpus (NOT pull-Butler-forward, NOT synthetic-placeholder).**
Story 8.1 (Butler v0.3 + `calendar-comms-v0.3.jsonl`) is `epic-8: backlog` and not implemented. Rather than block 7.5b or reorder epics, build the full harness now against a 7.5b-owned SHA-pinned **fixture** corpus, with a resolver that auto-swaps in the real Butler corpus when it appears. This matches the project's documented forward-dependency policy: *"slice minimum prerequisites forward, not reorder"* (`dependency-verification-12-epic-ordering.md`). The fixture is explicitly marked as a stand-in; the gate result is labelled `provisional` until the real corpus is wired.

**Decision 2 — Human-trial boundary → INFRA + PROTOCOL ARTIFACTS + ONE dry-run self-trial (NOT full literal execution).**
The N=12 / 14-day trial is inherently un-runnable by a dev agent. 7.5b delivers everything mechanically buildable + a single end-to-end self-trial that proves the harness scores correctly. The live trial is a human-research activity run later against these artifacts.

## Prerequisites (verified present at story-creation time — re-verify in AC1)

| Prerequisite | Status | Path / Evidence |
|---|---|---|
| Story 2.3 cargo-generate template | ✅ PRESENT | `templates/spirit-rust/` (Cargo.toml, manifest.toml, README.md, src/lib.rs, tests/spirit_smoke.rs) |
| Story 2.3 local runner | ✅ PRESENT | `crates/maos-spirit-sdk/src/local_runner.rs` (`LocalRunner::run`, `LocalRunnerFixture`, `RunReport`) |
| Story 2.3 baked example Spirit | ✅ PRESENT | `examples/example-spirit/` (+ `example-spirit-ts/`) |
| Story 0.3 corpus harness | ✅ PRESENT | `maos-corpus-gen`, `maos-eval/src/*_corpus.rs`, `xtask check-corpus` / `corpus-staleness` |
| **Story 8.1 Butler + calendar-comms corpus** | ❌ **ABSENT** (`epic-8: backlog`) | `spirits/` has only `hello-spirit`; no `spirits/butler/`, no `calendar-comms-v0.3.jsonl` → **handled by Decision 1 seam** |
| Three-door page / `docs.maos.dev` source | ❌ ABSENT | no `docs/maos.dev/` → this story creates it (functional; WCAG-AA → 9.5) |

## Acceptance Criteria

### AC1 — Prerequisites & seam classified mechanically before harness work opens

**Given** the prerequisite table above
**When** AC1 runs first
**Then** a check confirms each ✅ prerequisite path still exists (template, local_runner symbol, example-spirit, corpus harness) and the dev records the result in the Dev Agent Record
**And** the Butler-corpus absence is confirmed (no `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`) and the seam (Decision 1) is the chosen resolution — recorded, not silently worked around
**And** no new workspace crate is introduced (`check-workspace-count` stays GREEN at 30).

### AC2 — Recruitment protocol + stratified screener committed and machine-validatable

**Given** the recruitment process must be documented and reproducible (not a vibe)
**When** the protocol artifacts are authored
**Then** `docs/research/nfr-onb-1-protocol.md` documents: cohort target N=12, the stratification floor (≥4 no prior MAOS contribution / ≥3 never written a Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only), the 14-day zero-DM-support window, the public-issue-tracker support-routing rule, the outcome-tracking procedure, and the NFR-Onb-4 cadence
**And** `docs/research/nfr-onb-1-screener.md` is the participant screener form whose questions map 1:1 to the stratification strata
**And** a machine-readable cohort schema (`docs/research/nfr-onb-1-cohort.schema.json`) defines a participant record (stratum flags + offline-only + native-language) so a cohort manifest can be validated mechanically
**And** the private artifact locations are documented (`_research/nfr-onb-1/v0.3/recruitment-log.jsonl`, `_research/nfr-onb-1/v0.3/outcomes.jsonl`) and `_research/` is gitignored (private, NOT in main repo) — the repo ships only the schema + a committed redacted example.

### AC3 — Stratification validator: a cohort manifest is PASS only if it meets the N=12 strata floor

**Given** a cohort manifest conforming to the cohort schema
**When** the stratification validator runs
**Then** it asserts N=12 AND every stratum floor (≥4 / ≥3 / ≥2 / ≥2 / ≥1) is met, emitting a typed PASS/FAIL with the specific failing stratum named on FAIL
**And** a unit test proves a deficient cohort (e.g. only 1 non-English-native) FAILS with the correct stratum identified
**And** a unit test proves the committed redacted example cohort PASSES.

### AC4 — Outcome-scoring harness: a candidate Spirit is scored deterministically against the Butler-class corpus (via the seam)

**Given** a participant's Spirit and the Butler-class regression corpus (real if present, else the SHA-pinned fixture — Decision 1)
**When** the scoring harness runs one candidate
**Then** it resolves the corpus path via a documented resolver that PREFERS `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` and FALLS BACK to the 7.5b fixture, logging which corpus + its SHA-256 it used and stamping the outcome `corpus_source: butler|fixture`
**And** it computes, per the NFR-Onb-1 floor: (a) compiles-against-published-ABI boolean, (b) corpus pass over the 30 scenarios, (c) **halt-recall ≥0.90 on the calendar-conflict subset**, (d) **halt-precision ≥0.85 overall**, (e) time-to-success minutes
**And** "succeed" is defined exactly as NFR-Onb-1: (a) ∧ (b) ∧ (c) ∧ (d) within the time/window budget
**And** it emits one `outcomes.jsonl` row per candidate conforming to `docs/research/nfr-onb-1-outcomes.schema.json`
**And** the fixture corpus is committed at a **7.5b-owned** path (e.g. `crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl`), SHA-256-pinned, with a header line/manifest marking it a **STAND-IN for Story 8.1** — it is NOT written to the `spirits/butler/...` path (ownership boundary).

### AC5 — Cohort gate evaluator: PASS only when the full NFR-Onb-1 floor is met; result is `provisional` on fixture corpus

**Given** an `outcomes.jsonl` for a cohort
**When** the gate evaluator runs
**Then** it computes the cohort floor — **≥10 of 12 succeed**, **median time-to-success ≤45 min**, **p95 time-to-success ≤90 min** — and emits PASS/FAIL with every failing sub-criterion named
**And** when scored against the fixture corpus the verdict is stamped `provisional: true` (real-corpus verdicts are `provisional: false`) so a fixture-only PASS can never be mistaken for the live v0.3 gate
**And** unit tests prove: a 10/12 cohort within budget PASSES; a 9/12 cohort FAILS on success-count; a cohort with median 50 min FAILS on median; a cohort with p95 95 min FAILS on p95 — each naming the correct sub-criterion.

### AC6 — NFR-Onb-4 iteration-cadence machinery: misses are tracked and 3 consecutive misses escalate

**Given** the NFR-Onb-4 operational commitment (not a one-shot gate)
**When** a gate run misses the floor
**Then** the machinery records the miss in a run-ledger (`_research/nfr-onb-1/v0.3/run-ledger.jsonl`, private; schema committed) and surfaces the directive "run a fresh 6-author cohort within 2 weeks"
**And** when 3 consecutive misses are recorded it raises an `EscalateReleaseReview` signal (PRD-author + architecture lead + research lead) — proven by a unit test that feeds 3 sequential misses and asserts escalation, and a test that a PASS resets the consecutive-miss counter.

### AC7 — Three-door page live at `docs.maos.dev` (NFR-Onb-3), Spirit-author door wired to the real template

**Given** NFR-Onb-3
**When** the three-door page is authored
**Then** `docs/maos.dev/` hosts the landing page with exactly three doors — **"write a Spirit" / "run MAOS" / "understand MAOS"**
**And** the "write a Spirit" door links to the Story 2.3 cargo-generate template with the **verbatim working command** (`cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit`) and the 30-minute first-Spirit path
**And** a link-integrity check confirms the Spirit-author door's referenced template path (`templates/spirit-rust/`) actually exists in the repo (no dead link)
**And** WCAG-AA conformance + canonical publication is explicitly **deferred to Story 9.5** (noted in-page and in Dev Notes) — do NOT build the polished public HTML site here.

### AC8 — One end-to-end dry-run self-trial proves the harness is wired (Decision 2)

**Given** the full harness (AC3–AC6) and the fixture corpus (AC4)
**When** the dev agent runs ONE self-trial end-to-end
**Then** it scaffolds a single Spirit via the Story 2.3 template (or reuses `examples/example-spirit/`), runs it through `LocalRunner` against the fixture corpus, scores it via the harness, and emits ONE `outcomes.jsonl` row
**And** it runs the gate evaluator on that N=1 sample and the run completes without panics, producing a `provisional` verdict
**And** the self-trial is captured as a smoke test (e.g. `smoke-onb-1-7-5b`) so CI re-runs it — this is a **wiring proof, explicitly NOT the N=12 gate** (assert N=1 and `provisional: true` in the test so it can never be mistaken for the real gate).

### AC9 — Gate registered as a discipline rail + gate-registry + coverage-matrix; ships GREEN in this story

**Given** the project's gate-discipline pattern (mirror Story 7.3 CCAC ship-gate, Story 7.5a stability gate)
**When** the gate is wired
**Then** a new xtask subcommand (e.g. `nfr-onb-1-gate`) runs the stratification validator + gate evaluator in `--check` mode against the committed example cohort + self-trial outcome, failing loudly on drift
**And** it is registered in `xtask/gate-registry.toml`, added as a `discipline.yml` job arm, and reflected in the `coverage-matrix` walker so the gate cannot silently disappear
**And** all new tests + the new discipline arm + the smoke arm ship **GREEN in this same story** (no promised-but-deferred gate — see the "mechanical gates compound; promises decay" project lesson).

## Tasks / Subtasks

- [x] **Task 1 — AC1: Prerequisite + seam classification gate**
  - [x] Add a small check (xtask or test) asserting template/local_runner/example-spirit/corpus-harness paths exist and Butler corpus is absent; record in Dev Agent Record
  - [x] Confirm `check-workspace-count` GREEN (no new crate)
- [x] **Task 2 — AC2: Protocol + screener + schemas**
  - [x] Write `docs/research/nfr-onb-1-protocol.md` (recruitment, stratification, 14-day zero-DM, public-tracker routing, outcome tracking, NFR-Onb-4 cadence)
  - [x] Write `docs/research/nfr-onb-1-screener.md` (questions ↔ strata 1:1)
  - [x] Add `docs/research/nfr-onb-1-cohort.schema.json` + `nfr-onb-1-outcomes.schema.json` + `nfr-onb-1-run-ledger.schema.json`
  - [x] Commit a redacted example cohort + example outcomes under `docs/research/examples/`
  - [x] Add `_research/` to `.gitignore`; document private artifact paths
- [x] **Task 3 — AC3: Stratification validator** (in `maos-eval`, e.g. `onboarding_gate_corpus.rs`)
  - [x] Parse cohort manifest; assert N=12 + strata floors; typed FAIL names failing stratum
  - [x] Unit tests: deficient cohort FAILS (correct stratum); example cohort PASSES
- [x] **Task 4 — AC4: Outcome-scoring harness + corpus seam**
  - [x] Corpus-path resolver: prefer `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`, else 7.5b fixture; log path + SHA-256; stamp `corpus_source`
  - [x] Commit SHA-pinned fixture corpus at `crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl` with STAND-IN header (30 scenarios incl. a calendar-conflict subset)
  - [x] Score one candidate: compiles-against-ABI, corpus pass, halt-recall (calendar-conflict subset), halt-precision (overall), time-to-success; "succeed" = NFR-Onb-1 conjunction
  - [x] Emit `outcomes.jsonl` row conforming to schema; unit tests for recall/precision math at the ≥0.90 / ≥0.85 boundaries
- [x] **Task 5 — AC5: Cohort gate evaluator**
  - [x] ≥10/12 + median ≤45 + p95 ≤90; name each failing sub-criterion; `provisional` flag from `corpus_source`
  - [x] Unit tests: 10/12 PASS; 9/12 FAIL; median 50 FAIL; p95 95 FAIL
- [x] **Task 6 — AC6: NFR-Onb-4 cadence machinery**
  - [x] Run-ledger append + consecutive-miss counter + `EscalateReleaseReview` at 3
  - [x] Unit tests: 3 misses → escalate; PASS resets counter
- [x] **Task 7 — AC7: Three-door page**
  - [x] `docs/maos.dev/` index + three door pages; "write a Spirit" → verbatim cargo-generate command + 30-min path
  - [x] Link-integrity check (template path exists); note WCAG-AA deferral to 9.5
- [x] **Task 8 — AC8: Dry-run self-trial**
  - [x] Scaffold/reuse one Spirit; run via `LocalRunner` against fixture corpus; score; emit one outcome; run evaluator (N=1, `provisional`)
  - [x] Capture as `smoke-onb-1-7-5b`; assert N=1 ∧ provisional so it can't pose as the real gate
- [x] **Task 9 — AC9: Gate wiring (GREEN in-story)**
  - [x] xtask `nfr-onb-1-gate` (`--check`) over example cohort + self-trial outcome
  - [x] Register in `gate-registry.toml`; add `discipline.yml` arm + smoke arm; update `coverage-matrix`
  - [x] Run the full discipline-relevant subset locally; record evidence in Dev Agent Record

## Dev Notes

### Architecture compliance (binding rules — cite in PRs)
- **Workspace stays 30** — reuse `maos-eval` (scoring corpus, mirroring `hsis_corpus.rs` / `halt_corpus.rs` / `distillate_corpus.rs` which already carries `expected_recall`) and `xtask` (gate). NO new crate. `check-workspace-count` is a hard rail.
- **Ownership boundary** — do NOT write to `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`; Story 8.1 owns it ("no other story authors this corpus"). 7.5b ships a stand-in at a 7.5b-owned path + a resolver seam.
- **Fail-loud, never silent** — corpus resolver logs which corpus + SHA it used; fixture verdicts are `provisional`; gate evaluator names every failing sub-criterion. Mirror the typed-error + `--check`-drift pattern of `stability_matrix.rs` and the CCAC ship-gate (Story 7.3).
- **Private research artifacts** — `_research/**` is gitignored; only schemas + redacted examples are committed. A DM-support breach invalidates a trial (protocol doc), but that is a human-process rule, not code.

### Source tree — exact insertion points (do NOT reinvent)
- Scoring corpus + validator + evaluator + cadence: `crates/maos-eval/src/onboarding_gate_corpus.rs` (new module; register in `crates/maos-eval/src/lib.rs` alongside the other `*_corpus` modules).
- Fixture corpus: `crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl`.
- Gate command: `xtask/src/nfr_onb_1_gate.rs` (mirror `stability_matrix.rs` dual-mode `run(workspace_root, check, json)`; wire into `xtask/src/main.rs` + `xtask/src/lib.rs`).
- Registry/CI: `xtask/gate-registry.toml` (append `nfr-onb-1-gate`), `.github/workflows/discipline.yml` (new job arm + `smoke-onb-1-7-5b` arm), `xtask/src/coverage_matrix.rs`.
- Docs: `docs/research/nfr-onb-1-*.md` + `docs/research/*.schema.json` + `docs/research/examples/`; `docs/maos.dev/` for the three-door page; `.gitignore` (`_research/`).
- Self-trial smoke: a `tests/` integration test (gate behind the project's smoke/fixture-replay feature convention used by other smoke arms).

### Local runner / scoring reality (read `crates/maos-spirit-sdk/src/local_runner.rs` before coding)
- `LocalRunner::run(spirit, vtable, fixture) -> RunReport` is stateless; `RunReport` carries `hooks_fired`, `elapsed_per_hook`, `mock_bus_frames` (empty at v0.3 prerequisite — populated by Story 2.4), `deprecation_warnings_surfaced`.
- `mock_bus_frames` being empty at v0.3 means halt/notification signals for recall/precision scoring must come from the **scenario expectations in the corpus** compared against the Spirit's observable outputs/hooks — design the fixture corpus scenario schema to carry expected halt-tags (calendar-conflict subset flagged) and the harness to derive recall/precision from expected-vs-observed. Document this scoring contract in the module so Story 8.1's real corpus drops in cleanly.

### Testing standards (mirror Stories 7.1–7.5a)
- Tests live in the same crate; xtask gates run in `--check` mode in CI. **Never run `cargo fmt -p <crate>`** here — whole-crate collateral (Story 7.5a lesson). Use `cargo fmt -- <files>` or rustfmt on specific files.
- Many kernel tests need `init_monotonic_base()` before any `monotonic_now_ns()` path (recurring fix across Epics 4–7); if the scoring/time path touches monotonic time, seed it in tests.
- Every new gate + smoke arm must ship GREEN in THIS story — promised gates decay (project lesson `mechanical_gates_compound_promises_decay`).

### Previous-story intelligence (Story 7.5a, just shipped to review)
- 7.5a added `stability_matrix.rs` as a dual-mode generate/`--check` xtask gate sourcing every value from live state — **this is the exact pattern to clone** for `nfr-onb-1-gate`.
- 7.5a ended at ~89 discipline jobs and surfaced a pre-existing `check-service-boundary` 101 stale-baseline P0 and a still-degraded §A2 hard-fail flip. Those are NOT this story's to fix, but expect a noisy baseline — record any pre-existing RED you encounter rather than absorbing it into 7.5b's scope.
- 7.5a reconciled the §A4 hook-count to a truthful 14; keep dev-record completeness (`check-dev-record-completeness`) and Review-Findings rows honest — those gates block sprint-status `done`.

### Project structure notes
- NFR-Onb-3 is listed v0.5 in `prd/non-functional-requirements.md:124` but the epic-7 Story 7.5b AC ties the functional three-door page to v0.3 with WCAG-AA polish at 9.5 — follow the **epic AC** (functional now, polish in 9.5). Note the version-label discrepancy for Winston in the question list.
- The gate is the **v0.3 release criterion** (`architecture .../13-phased-roadmap.md`, `glossary.md`). 7.5b ships the machinery that lets that criterion be evaluated; the human cohort run is out-of-band.

### References
- [Source: _bmad-output/planning-artifacts/epics/epic-7-...-v25.md#story-75b] — full Story 7.5b ACs
- [Source: _bmad-output/planning-artifacts/prd/non-functional-requirements.md#122-125] — NFR-Onb-1 (tightened), NFR-Onb-3, NFR-Onb-4 verbatim floors
- [Source: _bmad-output/planning-artifacts/epics/epic-2-...-v03.md#story-23] — cargo-generate template + local runner prerequisite (shipped)
- [Source: _bmad-output/planning-artifacts/epics/epic-8-...-v15.md#story-81] — Butler corpus ownership ("no other story authors this corpus"); `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`
- [Source: _bmad-output/planning-artifacts/epics/dependency-verification-12-epic-ordering.md] — forward-dependency policy ("slice prerequisites forward, not reorder")
- [Source: _bmad-output/planning-artifacts/spirit-development-and-sharing.md#v03-prerequisite] — template/runner/example paths; 7.5b is the gate CONSUMER, 2.3 ships substrate
- [Source: crates/maos-spirit-sdk/src/local_runner.rs] — `LocalRunner` / `RunReport` scoring substrate
- [Source: xtask/src/stability_matrix.rs] — dual-mode `--check` gate pattern to clone
- [Source: xtask/gate-registry.toml] — gate registration surface

## Dev Agent Record

### Agent Model Used

claude-opus-4-8

### Debug Log References

- `cargo test -p maos-eval --lib onboarding_gate_corpus` → 18 passed (AC1/AC3/AC4/AC5/AC6 boundary tests).
- `cargo test -p maos-eval --test onb_1_self_trial -- --nocapture` → 1 passed; emits one `outcomes.jsonl` row (`corpus_source=fixture`, `provisional=true`), N=1 verdict `passed:false` (success-count 1 < 10 — guards against masquerading as the live gate).
- `cargo run -p xtask -- nfr-onb-1-gate --check` → PASS (prereqs GREEN, example cohort PASSES stratification, example outcomes PASS the cohort gate provisional, self-trial provisional, three-door link integrity OK).
- `cargo run -p xtask -- check-workspace-count --json` → `passed:true`, actual-count 30 (no new crate).
- `cargo run -p xtask -- coverage-matrix` → exit 0 (warning mode); NFR-Onb-1 row now references `nfr-onb-1-gate`; my change adds **zero** new violations (NFR-Onb-1 appears only as an out-of-scope-deferred row, phase v0.3 > current v0.1-alpha).
- `cargo build --workspace` → Finished, no errors.

### Completion Notes List

All 9 ACs implemented and GREEN in-story. Workspace stays at **30** crates (no new crate — reused `maos-eval` + `xtask`).

- **AC1** — `classify_prerequisites(workspace_root)` in `onboarding_gate_corpus.rs` asserts template / `LocalRunner` symbol / example-spirit / corpus-harness present and Butler corpus absent (seam active, `corpus_source=fixture`); surfaced by `nfr-onb-1-gate` and proven by unit test `ac1_prerequisites_classified`. `check-workspace-count` GREEN at 30.
- **AC2** — `docs/research/nfr-onb-1-protocol.md` (N=12, stratification floor, 14-day zero-DM window, public-tracker support routing, outcome tracking, NFR-Onb-4 cadence, private-vs-committed artifact table), `nfr-onb-1-screener.md` (Q1–Q5 ↔ strata 1:1), three JSON schemas (cohort/outcomes/run-ledger), redacted examples under `docs/research/examples/`, and `_research/` added to `.gitignore`.
- **AC3** — `validate_stratification` asserts N=12 + every stratum floor (≥4/≥3/≥2/≥2/≥1) and names the deficient stratum on FAIL. Tests: deficient non-English cohort FAILs naming `non_english_native`; example cohort PASSES; wrong-N FAILs on `cohort_size`.
- **AC4** — `resolve_corpus` PREFERS `spirits/butler/.../calendar-comms-v0.3.jsonl`, falls back to the 7.5b fixture, logs path + SHA-256, stamps `corpus_source`. `score_candidate` computes compiles-against-ABI ∧ corpus-pass ∧ halt-recall (calendar-conflict subset) ∧ halt-precision (overall) ∧ within-window; SHA-pinned 30-scenario fixture committed at `crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl` (STAND-IN header line; SHA `1a5b0738…d03d9110`). The `observations` map is the **seam**: `None` → fixture's baked `observed_halt` (v0.3); `Some` → Story 8.1's real bus-observed halts — documented in the module scoring contract so the real corpus + Story 2.4 bus capture drop in without touching the public surface. Boundary tests: recall exactly 0.90 PASSES, 0.80 FAILS; precision exactly 0.85 PASSES.
- **AC5** — `evaluate_cohort` PASSES only on ≥10/12 succeed ∧ median ≤45 ∧ p95 ≤90, naming each failing sub-criterion, and stamps `provisional` from `corpus_source`. Tests: 10/12 PASS; 9/12 FAIL on success-count; median-50 FAIL on median; p95-95 FAIL on p95; real-corpus is non-provisional.
- **AC6** — `CadenceMachine` ledgers runs, surfaces the directive "run a fresh 6-author cohort within 2 weeks" on a miss, raises `EscalateReleaseReview` (PRD-author + architecture lead + research lead) at 3 consecutive misses, and resets on a PASS. Tests: 3 misses → escalate; PASS resets; `from_ledger` reconstructs the counter.
- **AC7** — `docs/maos.dev/` ships a functional three-door page (Write a Spirit / Run MAOS / Understand MAOS); the write-a-Spirit door carries the verbatim `cargo generate … templates/spirit-rust …` command + the 30-min path; the gate's link-integrity check confirms `templates/spirit-rust/` exists (no dead link); WCAG-AA + canonical publication explicitly deferred to Story 9.5 (noted in-page).
- **AC8** — `crates/maos-eval/tests/onb_1_self_trial.rs` (`smoke-onb-1-7-5b`) scaffolds ONE Spirit via the `#[spirit]` macro, runs it through `LocalRunner`, scores it against the fixture, emits one `outcomes.jsonl` row, and evaluates the N=1 sample → provisional. Asserts **N=1 ∧ provisional ∧ !passed** so it can never pose as the N=12 gate.
- **AC9** — `xtask nfr-onb-1-gate --check` runs the prereq classification + stratification (example cohort) + cohort evaluator (example outcomes) + self-trial-provisional + three-door link integrity, failing loudly on drift. Registered in `gate-registry.toml`; `discipline.yml` gains a `nfr-onb-1-gate` arm + a `smoke-onb-1-7-5b` arm, both added to `aggregate.needs`; `coverage-matrix.yaml` NFR-Onb-1 row now lists `nfr-onb-1-gate`. Gate + smoke ship GREEN in this story.

**Pre-existing RED observed (NOT in 7.5b scope — recorded per the 7.5a guidance):**
- `check-serde-error-handling` is at a **pre-existing failing baseline of 300 violations at HEAD** (concentrated in `maos-domain/invariants/*` and test code; CI scans `crates` by default). My new code adds **zero** new violations (the one `.expect()` after `serde_json::to_string` in the self-trial was rewritten to an explicit `match`); the count returns to 300.
- `coverage-matrix` runs in `mode: warning` and already reports `passed:false` from pre-existing unregistered-gate references (e.g. `smoke-skill-7-4`, `audit_spine_integration`); it exits 0. My addition introduces no new violation.

**Flag for Winston:** NFR-Onb-3 is labelled **v0.5** in `prd/non-functional-requirements.md` but the epic-7 Story 7.5b AC ties the **functional** three-door page to **v0.3** (WCAG-AA polish → 9.5). I followed the epic AC. Please reconcile the PRD version label.

### File List

**New — Rust:**
- `crates/maos-eval/src/onboarding_gate_corpus.rs` — stratification validator + scoring harness + corpus resolver seam + cohort evaluator + NFR-Onb-4 cadence + AC1 prereq classification (+ unit tests).
- `crates/maos-eval/tests/onb_1_self_trial.rs` — `smoke-onb-1-7-5b` dry-run self-trial.
- `xtask/src/nfr_onb_1_gate.rs` — the `nfr-onb-1-gate` discipline rail (+ unit test).

**New — fixtures / docs / schemas:**
- `crates/maos-eval/fixtures/nfr-onb-1/calendar-comms-v0.3.fixture.jsonl` — SHA-pinned 30-scenario STAND-IN corpus.
- `crates/maos-eval/fixtures/nfr-onb-1/README.md` — STAND-IN labelling + SHA-256 pin + distribution.
- `docs/research/nfr-onb-1-protocol.md`, `docs/research/nfr-onb-1-screener.md`.
- `docs/research/nfr-onb-1-cohort.schema.json`, `docs/research/nfr-onb-1-outcomes.schema.json`, `docs/research/nfr-onb-1-run-ledger.schema.json`.
- `docs/research/examples/cohort.example.json`, `outcomes.example.jsonl`, `run-ledger.example.jsonl`, `self-trial-outcome.example.jsonl`.
- `docs/maos.dev/index.md`, `write-a-spirit.md`, `run-maos.md`, `understand-maos.md`.

**Modified:**
- `crates/maos-eval/src/lib.rs` — register `onboarding_gate_corpus` module.
- `crates/maos-eval/Cargo.toml` — add `sha2` dep + `maos-spirit-sdk` (local_runner) dev-dep.
- `xtask/Cargo.toml` — add `maos-eval` dep.
- `xtask/src/main.rs` — `mod nfr_onb_1_gate;` + `NfrOnb1Gate` command + match arm.
- `xtask/gate-registry.toml` — register `nfr-onb-1-gate`.
- `tests/coverage-matrix.yaml` — NFR-Onb-1 row gains `nfr-onb-1-gate` + updated notes.
- `.github/workflows/discipline.yml` — `nfr-onb-1-gate` + `smoke-onb-1-7-5b` arms + `aggregate.needs`.
- `.gitignore` — add `_research/`.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 7-5b → review.

### Change Log

- 2026-06-01 — Story 7.5b implemented: NFR-Onb-1 30-Minute First Spirit Validation Gate **execution infrastructure** at v0.3. New `maos-eval::onboarding_gate_corpus` module (stratification validator, outcome-scoring harness, Butler-corpus resolver seam, cohort gate evaluator, NFR-Onb-4 cadence machinery, AC1 prereq classification); SHA-pinned 30-scenario STAND-IN fixture corpus; protocol/screener/3 schemas + redacted examples; functional three-door `docs.maos.dev` page; `smoke-onb-1-7-5b` dry-run self-trial; `xtask nfr-onb-1-gate` discipline rail registered in gate-registry + coverage-matrix + 2 new discipline arms. Workspace stays 30. Locked Decisions 1 (Butler seam + SHA-pinned fixture, verdicts provisional) and 2 (infra + protocol + ONE self-trial, no live N=12 trial) honored. All new tests + gate + smoke GREEN in-story.

### Review Findings

- [x] [Review][Patch] CadenceMachine re-escalation: keep `>=`, add test for 4th+ miss — Team consensus (Winston/Amelia): `>=` semantics are correct — the alarm condition persists until resolved. Test `ac6_fourth_miss_re_escalates` added. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Patch] Recall denominator: keep `calendar_conflict && expected_halt`, add doc comment — Team consensus: the stricter denominator is statistically correct for halt-recall. Code comment added documenting the semantic for Story 8.1. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Patch] Add optional `native_language` string field to cohort schema + `ParticipantRecord` — Team consensus: AC2 explicitly names "native-language"; optional string added for spec alignment and future bias-auditing. `docs/research/nfr-onb-1-cohort.schema.json` + `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Patch] No runtime SHA-256 pin validation against committed fixture — `FIXTURE_CORPUS_SHA256` constant added; `resolve_corpus` asserts the computed digest matches on fixture path. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Patch] Dead variable `_observations` in self-trial — removed. `crates/maos-eval/tests/onb_1_self_trial.rs`
- [x] [Review][Patch] Empty/zero-scenario corpus vacuously passes + `CORPUS_SCENARIO_COUNT` never enforced — empty-corpus assertion in `score_candidate`; `validate_corpus_size` called by self-trial and xtask gate to enforce exact 30-scenario count on file-loaded corpora. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Defer] Fragile `LocalRunner` string-contains heuristic in `classify_prerequisites` — greps source code for `"impl LocalRunner"` (matches comments, doc comments, string literals). Test-only prerequisite check; works for current codebase. `crates/maos-eval/src/onboarding_gate_corpus.rs:1211-1214` — deferred, pre-existing testing pattern limitation
- [x] [Review][Defer] `participant_id` schema pattern `^P[0-9]{2,}$` not enforced by Rust code — `ParticipantRecord` accepts any `String`. Schema is for human validation; self-trial uses `"self-trial-7-5b"` which targets the outcomes schema (minLength only). `crates/maos-eval/src/onboarding_gate_corpus.rs:611` — deferred, schema is validation boundary
- [x] [Review][Defer] ~~`CorpusLine` uses `#[serde(untagged)]` producing poor error messages~~ — FIXED: replaced untagged with manual dispatch on `stand_in_for` key. `crates/maos-eval/src/onboarding_gate_corpus.rs`
- [x] [Review][Defer] `workspace_root()` in test helpers relies on CWD being crate directory — `.parent().parent().unwrap()` from CWD. Pre-existing project convention repeated here; breaks only with non-standard test runners. `crates/maos-eval/src/onboarding_gate_corpus.rs:1247-1256` — deferred, pre-existing pattern
