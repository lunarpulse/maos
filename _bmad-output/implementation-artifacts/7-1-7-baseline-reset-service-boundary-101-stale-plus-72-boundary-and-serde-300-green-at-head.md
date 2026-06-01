---
epic: epic-7
epic_title: "v0.5-β Spirit Ecosystem — Author, Publish, Comply, Stabilize"
dev_model_used: claude-opus-4-8
---

# Story 7.1.7: Baseline Reset — `check-service-boundary` 101-stale + 72 REAL boundary triage + serde-300 green-at-HEAD

**Status:** done

**Type:** Epic 7 → Epic 8 discipline bridge story (§A2 Decision 2 / baseline-reset). Per `[[project_epic_7_retro_outcomes]]` Lunarpulse's retro Decision 2 ("Baseline drift → **dedicated baseline-reset story (7.1.7)** for green-at-HEAD: check-service-boundary 101 stale violations P0, serde-error-handling 300 baseline, coverage-matrix gaps, §A4 hook-count 14"). This is NOT a feature story — its acceptance criteria ARE the output of CI discipline gates that must go from RED-at-HEAD to green-at-HEAD before Epic 8 Story 8.1 (Butler) opens, so 8.1 authors the real `calendar-comms-v0.3.jsonl` fixture on a CLEAN discipline tree rather than inheriting a 328-line `check-service-boundary` failure + a 300-violation soft-fail serde gate. The retro under-scoped one row: `check-service-boundary` is **NOT** all stale-baseline — it mixes 101 mechanically-stale cargo-public-api baseline entries (`removed public`) with **72 REAL boundary violations** (63 P3 + 7 P2 + 2 P1) that each require a TRIAGE DECISION (privatize vs I9-whitelist), NOT a rebaseline. Unlike the pure-rebaseline sibling Story 7.1.6, this story is **NOT zero-crate-code**: ~72 field-visibility changes MAY touch Cargo crate code, and a `pub`→`pub(crate)` privatization IS itself an ABI removal — so the baseline regen (Gate 1a) and the privatization (Gate 1b) must be reconciled together, which is the trickiest interaction in the story. Per `[[feedback_mechanical_gates_compound_promises_decay]]` ("ship the gate-closure in the SAME story that promises it, or it decays"), 7.1.7 carries the ENTIRE green-at-HEAD closure including the baseline regen, the 72 triage decisions, the serde posture decision, and the §A4 hook-count truthfulness verification — no carry-forward marker remains for Epic 8 to inherit.

## Story

As **a discipline-as-code steward who watched the Epic 7 retro flag baseline drift as the LAST blocker before Epic 8 (retro Decision 2; `[[project_epic_7_retro_outcomes]]` line 18) AND as the Story 8.1 (Butler) author who needs `cargo run -p xtask -- check-service-boundary` to exit 0 at HEAD so the Butler fixture work runs on a green discipline baseline rather than triaging an inherited 328-line failure mid-feature**,

I want **(a) the 101 `removed public` violations in `check-service-boundary` — genuinely STALE cargo-public-api baseline entries (symbols listed in the baseline that no longer exist at HEAD) — CLEARED by regenerating the baseline files (purely mechanical: the gate's `NFR-Test-2 violation: removed public kernel symbol '…'` rows disappear once the baseline matches the current monotonically-additive surface); (b) the 72 REAL boundary violations (63 P3 + 7 P2 + 2 P1, each flagged `see check-empty-kernel for full I9 context`) each resolved via an explicit per-field TRIAGE DECISION — for each public field exposing a concrete type, EITHER demote it to `pub(crate)`/private OR add it to the I9 whitelist (`xtask/i9-whitelist.toml`) / pub-field-constructor-allowlist with a one-line written justification — covering at minimum the enumerated set: `HotSwapCoordinator.{journal, tl, halt_registry, capability, iac, dispatcher, telemetry, active_monitors, pending_reverts}` (all `Arc<…>`), `T3ImageLock.attestations: Vec<T3ImageAttestation>`, `CaptureChannel.events: Arc<Mutex<Vec<NotificationEvent>>>`, `GatewayInstance.{submodule: Arc<dyn GatewaySubmodule>, cancel_flag: Arc<AtomicBool>}`, `GatewayCancelHandle.flag: Arc<AtomicBool>`; (c) the cargo-public-api baseline regen (a) and the privatizations (b) RECONCILED — because demoting a `pub` field to `pub(crate)` is itself a public-surface REMOVAL, the dev MUST run the baseline regen AFTER the privatizations land (or in the same pass), and verify the net `cargo public-api --diff` is intentional-removal-only, NOT an accidental Added/Changed surface drift; (d) the `check-serde-error-handling` gate — currently SOFT (`continue-on-error: true` at `.github/workflows/discipline.yml:1008`) with 300 violations across 83 files (`serde_json::from_str(...).unwrap(...)` patterns concentrated in `maos-domain/invariants/*` and `maos-audit/src/lib.rs` e.g. lines 803, 856) — driven to a DOCUMENTED green posture: the dev makes an EXPLICIT, recorded decision between (d-i) REMEDIATE the 300 sites to `.map_err(|e| <CrateError>::Serialize(e.to_string()))?` propagation, OR (d-ii) FREEZE a baseline allowlist (the gate already supports `load_allowlist()` + inline `// xtask-serde-allow` per `xtask/src/check_serde_error_handling.rs`) so the gate fails ONLY on NEW violations beyond the frozen set, then optionally flip the gate to hard-fail; the chosen posture is captured verbatim in an AC and in the dev record; (e) the `coverage-matrix` gate RE-CONFIRMED green — the Epic 7 retro OVERSTATED this as "~19 gaps"; the gate exits 0 at HEAD (the deferred-NFR-Test-N lines are informational/by-design), so this story RE-RUNS `coverage-matrix` AND `coverage-matrix-nfr-test-3` to prove green and documents that a prior probe hit a SHELL error not a gate error — NO coverage work is in scope unless nfr-test-3 genuinely fails; (f) the §A4 hook-count CONFIRMED truthful at 14 (per the prior Epic 7 reconciliation; `xtask/spirit-abi-hook-count.toml` `expected_count = 14`) via a verification task that runs `check-service-boundary`'s hook-count assertion and confirms `14`, NOT a re-litigation of the count; (g) ALL three discipline gates green-at-HEAD before Story 8.1 opens — `check-service-boundary` exits 0, serde gate green-or-frozen-baseline-documented, `coverage-matrix` confirmed exit 0**,

so that **(i) Epic 8 Story 8.1 (Butler) opens on a CLEAN discipline tree per `[[project_epic_7_retro_outcomes]]` critical path ("§A2 baseline-reset (back-to-back bridge, green-at-HEAD) → 8.1 Butler authors real 30-scenario fixture") — the Butler author runs `cargo run -p xtask -- check-service-boundary` and sees exit 0, not a 328-line wall of stale + real violations that would force mid-feature triage; (ii) the 101-vs-72 distinction is RESOLVED CORRECTLY — the retro initially treated all of `check-service-boundary` as stale-baseline, but only 101 are; the 72 REAL boundary violations are genuine I9 encapsulation leaks (concrete `Arc<…>` / `Vec<…>` fields exposed on public structs) whose resolution is a load-bearing encapsulation decision, NOT a mechanical rebaseline; conflating them would either RE-EXPOSE the surface (if all rebaselined away) or FALSE-FAIL forever (if all left as violations); (iii) the baseline-regen + privatization reconciliation per (c) preserves the ABI Stability Triple's monotonic-additive invariant — a privatization is a DELIBERATE removal that the regenerated baseline must reflect, and the `abi-diff` gate must agree the removal is intentional, so the discipline substrate stays internally consistent post-7.1.7; (iv) the serde posture decision per (d) follows `[[feedback_mechanical_gates_compound_promises_decay]]` — the gate has been SOFT (`continue-on-error: true`) since "calibration phase per Epic 5 §A3 — flip to hard-fail post Story 6.3" (`discipline.yml:1008` comment), 2 epics overdue; 7.1.7 makes the EXPLICIT remediate-vs-freeze call so the soft-fail does NOT decay into a 4th epic; (v) the coverage-matrix re-confirmation per (e) prevents over-scoping — the retro's "~19 gaps" framing would have spent the bridge budget on phantom work; the story RE-CONFIRMS green and moves on, honoring `[[feedback_lunarpulse_observability_preference]]` (observe the gate actually exiting 0, not infer it from coverage%); (vi) per `[[feedback_story_sizing]]` the bridge bundles three coherent green-at-HEAD workstreams (service-boundary 101+72, serde posture, coverage re-confirm) under one bridge story the dev completes in one session without crossing into Epic 8 feature territory; (vii) the discipline-gate matrix at HEAD becomes IDEMPOTENT for the baseline-reset gates — re-running `check-service-boundary` / `check-serde-error-handling` / `coverage-matrix` on any later story produces the same exit-0 (or frozen-baseline-clean) result, giving Stories 8.1+ the mechanical regression substrate; (viii) sequencing per the constraints — 7.1.7 runs FIRST or CONCURRENTLY with 7.1.6; a clean `check-service-boundary` reduces noise in 7.1.6's other gate runs so the §A2 full-flip story does not have to read around a 328-line service-boundary failure**.

## What this story is NOT

- **NOT** zero-crate-code. Unlike Story 7.1.6 (pure §A2 backfill + workflow flip, no crate source touched) and unlike Story 7.1.5, this story's Gate-1b privatization of the ~72 REAL boundary fields MAY alter `crates/*/src/**` field visibility (`pub` → `pub(crate)` / private). Each visibility change is a deliberate crate-code edit. The dev MUST flag this divergence in the dev record and treat the crate edits with the same care as a feature change (rebuild, retest the touching crate, re-run the local crate test suite).

- **NOT** an indiscriminate rebaseline of `check-service-boundary`. The retro under-scoped this. ONLY the 101 `removed public` rows are stale-baseline-eligible. The 72 P3/P2/P1 rows are REAL boundary violations and MUST NOT be silenced by regenerating a baseline that "captures" them — that would re-bless the encapsulation leak. Each of the 72 gets an explicit privatize-or-whitelist decision; none is resolved by rebaseline.

- **NOT** a license to whitelist all 72 by default. The I9-whitelist / pub-field-constructor-allowlist path is the ESCAPE HATCH for fields that genuinely must stay public (e.g. a constructor-pattern field consumed across a crate boundary by design); the DEFAULT preferred resolution is privatization (`pub(crate)`). Every whitelist entry carries a one-line written justification; an empty-justification whitelist entry is a review-blocking finding.

- **NOT** a re-litigation of the §A4 hook-count. `xtask/spirit-abi-hook-count.toml` already states `expected_count = 14` (Story 5.2's `on_swap_out` + `snapshot` + `migrate` per ADR-017/020 brought the FR55 Epic-2 baseline of 11 → 14; reconciled in the prior Epic 7 work per `[[project_story_7_5a_landed]]` "§A4 reconciled to truthful 14"). This story VERIFIES the gate reports 14 and the config matches the trait surface at HEAD; it does NOT add/remove hooks and does NOT bump to 15 (the planned `epistemic_resolve` 15th hook stays a forward-shape note unless the HEAD trait surface already carries it — verify, do not assume).

- **NOT** a coverage-matrix expansion. The retro's "~19 gaps" is an OVERSTATEMENT; `coverage-matrix` exits 0 at HEAD (deferred-NFR-Test-N lines are by-design informational). This story RE-CONFIRMS green and runs `coverage-matrix-nfr-test-3` cleanly to verify a prior probe's failure was a SHELL error not a gate error. If `coverage-matrix` or `coverage-matrix-nfr-test-3` genuinely exit non-zero at story open, the dev SURFACES and re-scopes; otherwise NO coverage work lands.

- **NOT** Story 7.1.6 (the §A2 full-flip). 7.1.6 closes the 2 EXISTING §A2 gates (`check-review-findings-resolved` + `check-dev-record-completeness`) by clearing ~42 historical violations + backfilling 41 `dev_model_used`, then hard-failing both gates. That scope is ORTHOGONAL to baseline-reset. 7.1.7 does NOT touch the §A2 gates, does NOT touch story-file Review Findings tables, does NOT backfill `dev_model_used`. The two bridge stories run first/concurrently but stay scope-isolated.

- **NOT** an Epic 8 feature. ZERO Epic 8 surface is pre-staged. No `crates/maos-butler/`, no `calendar-comms-v0.3.jsonl`, no NFR-Onb-1 cohort work. The clean separation preserves the green-at-HEAD diagnostic value — Story 8.1's first commit on a green `check-service-boundary` is the mechanical proof the reset held.

- **NOT** an Epic 7 retrospective. The retro already ran (`epic-7-retro-2026-06-01.md`; `epic-7-retrospective: done` per `[[project_epic_7_retro_outcomes]]`). 7.1.7 is a post-retro bridge executing retro Decision 2, not a closing retro.

- **NOT** a new ADR or a PRD change. Baseline regen + field visibility triage + serde posture decision are below ADR granularity (the encapsulation decisions are applications of the EXISTING I9 invariant + ADR-038 KLOC ceiling + ADR-037 holder-path discipline, not new architecture). If the serde-posture decision (remediate vs freeze) is judged ADR-worthy by the dev, flag for Winston; default is a dev-record + xtask-config-comment record, not a new ADR.

- **NOT** a workspace-member count change. The Cargo crate count stays unchanged (Epic 7 closed at 30 workspace members per `[[project_epic_7_retro_outcomes]]`). 7.1.7 adds ZERO crates; it edits existing crate source (visibility), xtask config (i9-whitelist / serde allowlist), and `discipline.yml` (only if the serde gate flips to hard-fail).

## Bridge Preconditions (Epic 7 retro Decision 2 substrate confirmation + 7.1.7-blocking rows)

| Row | Source | Closure required for 7.1.7? | Status check |
|---|---|---|---|
| **EPIC-7-RETRO-DONE** | Epic 7 retro | **blocking_7_1_7** | Assert `sprint-status.yaml` shows `epic-7-retrospective: done`. If not done, STOP — 7.1.7 is a post-retro bridge executing Decision 2; it must not pre-empt the retro. |
| **SB-EXIT-1-AT-HEAD** | Gate 1 substrate | **blocking_7_1_7** | Run `cargo run -p xtask -- check-service-boundary` (slow; uses cargo-public-api). Assert exit 1 at HEAD with 328 violation lines. Confirm the 3-class split: ~101 `removed public` + ~72 P3/P2/P1 (`see check-empty-kernel for full I9 context`). If the counts have drifted, the dev REPORTS the actual counts and proceeds with the actual list — the 101/72 split is the SCOPE FLOOR, not a literal assertion. |
| **SB-101-STALE-CLASS** | Gate 1a substrate | **VERIFY** | Of the 328 lines, ~101 are `NFR-Test-2 violation: removed public kernel symbol '…'` rows — genuinely stale cargo-public-api baseline (symbols in baseline no longer at HEAD). These are rebaseline-eligible. Confirm by spot-checking 3 symbols against the current surface. |
| **SB-72-REAL-CLASS** | Gate 1b substrate | **VERIFY** | Of the 328 lines, ~72 are `P3 violation: <Struct>.<field>: …; see check-empty-kernel for full I9 context` (63 P3 + 7 P2 + 2 P1). These are REAL boundary violations needing triage. Confirm the enumerated set (HotSwapCoordinator 9 fields, T3ImageLock.attestations, CaptureChannel.events, GatewayInstance 2 fields, GatewayCancelHandle.flag) appears in the gate output. |
| **SERDE-SOFT-AT-HEAD** | Gate 2 substrate | **blocking_7_1_7** | Grep `.github/workflows/discipline.yml` for `continue-on-error: true` in the `check-serde-error-handling:` block (~line 1008). Assert present (the soft-fail substrate). Run `cargo run -p xtask -- check-serde-error-handling`; confirm ~300 violations across ~83 files. If already flipped or already clean, dev SURFACES. |
| **SERDE-ALLOWLIST-CAP** | Gate 2 capability | **VERIFY** | Confirm `xtask/src/check_serde_error_handling.rs` exposes `load_allowlist()` + recognizes inline `// xtask-serde-allow` / `// allow(serde-unwrap)` markers (the freeze-baseline capability path). This is what makes posture (d-ii) feasible without a new gate. |
| **COVERAGE-MATRIX-GREEN** | Gate 3 substrate | **VERIFY — expect exit 0** | Run `cargo run -p xtask -- coverage-matrix`. Assert exit 0 (deferred-NFR-Test-N lines are informational). Run `cargo run -p xtask -- coverage-matrix --measure-nfr-test-3 --dry-run` (the `coverage-matrix-nfr-test-3` job command). If both exit 0, Gate 3 needs NO work. If either fails, dev SURFACES and re-scopes per the NOT clause. |
| **A4-HOOK-COUNT-14** | §A4 substrate | **VERIFY — expect 14** | Read `xtask/spirit-abi-hook-count.toml`; assert `expected_count = 14`. Run `check-service-boundary`'s hook-count assertion; confirm it reports 14 and the 14-entry `[[hooks]]` list matches the `Spirit` trait surface at HEAD. Report; do NOT bump. |
| **7.1.6-CONCURRENCY** | Sequencing | **VERIFY — non-blocking** | Per constraints, 7.1.7 runs FIRST or CONCURRENTLY with 7.1.6. If 7.1.6 is in-flight, confirm 7.1.7 does NOT touch the 2 §A2 gates / story-file RF tables / `dev_model_used` frontmatter (scope-isolation). Report; do NOT block. |
| **ABI-DIFF-RECONCILE-READY** | Gate 1c substrate | **VERIFY** | Confirm the `abi-diff` gate + `cargo public-api --diff` path is runnable locally (per `[[project_story_7_5a_landed]]` LESSON: never `cargo fmt -p crate` here — whole-crate collateral). The privatization-vs-baseline reconciliation depends on this being clean before AND after the 72 triage. |

The AC1 gate classifies all 10 rows. The `blocking_7_1_7` rows must clear before AC2+ implementation opens. Per `[[feedback_mechanical_gates_compound_promises_decay]]` the bridge story IS the green-at-HEAD receipt — the 101-rebaseline + 72-triage + serde-posture + coverage-reconfirm land together; the story file lifecycle is `ready-for-dev → in-progress → in-review (via §A5 gate) → done`.

## Acceptance Criteria

### AC1 — Bridge preconditions classified; 10-row gate exit 0 on all blocking_7_1_7

**Given** the 10 bridge rows in §Bridge-Preconditions above

**When** the dev runs `cargo run -p xtask -- check-service-boundary` + the serde + coverage probes and records the actual at-HEAD counts (101 stale / 72 real / 300 serde / coverage exit code / hook-count)

**Then** each row is classified into `{verify_pass, verify_fail, blocking_7_1_7, scope_drift_surfaced}` and the dev proceeds to AC2+ ONLY if every `blocking_7_1_7` row has cleared

**And** the at-HEAD counts are cited VERBATIM in Completion Notes (the 101/72/300 figures are the SCOPE FLOOR; if drift is found, the dev records the actual numbers and proceeds with the actual list)

**And** the dev MUST NOT begin AC2 baseline regen until AC1's counts are recorded and the 101-vs-72 classification is confirmed

### AC2 — 101 stale `removed public` violations cleared via cargo-public-api baseline regen

**Given** the ~101 `NFR-Test-2 violation: removed public kernel symbol '…'` rows are genuinely stale cargo-public-api baseline entries (symbols listed in the baseline that no longer exist at HEAD)

**When** the dev regenerates the kernel-surface baseline file(s) the `check-service-boundary` gate diffs against (the baseline path(s) the gate loads — verify which baseline drives the `removed public` rows; the gate reads a `KernelSurface` JSON baseline per `check_service_boundary.rs`)

**Then** every `removed public` row disappears (the regenerated baseline reflects the current monotonically-additive surface; no symbol listed in the baseline is absent at HEAD)

**And** the regen is PURELY MECHANICAL — it removes baseline entries for symbols that were legitimately removed/renamed in prior stories without the baseline being updated; it does NOT add new public symbols and does NOT mask the 72 REAL boundary violations

**And** the dev records, in the dev record, the baseline file(s) touched + the before/after `removed public` count (expect 101 → 0)

**And** this AC's baseline regen is SEQUENCED to complete AFTER AC3's privatizations (or in a combined final pass) per the reconciliation in AC4 — because privatizing a `pub` field is itself a surface removal that the regenerated baseline must capture; the dev MUST NOT regen the baseline before the 72 triage lands, else the privatized fields produce NEW `removed public` rows on the next run

### AC3 — 72 REAL boundary violations triaged (privatize OR whitelist with justification)

**Given** the ~72 REAL boundary violations (63 P3 + 7 P2 + 2 P1), each `P3/P2/P1 violation: <Struct>.<field>: <type>; see check-empty-kernel for full I9 context`, including AT MINIMUM the enumerated set:
- `HotSwapCoordinator.{journal, tl, halt_registry, capability, iac, dispatcher, telemetry, active_monitors, pending_reverts}` — all `Arc<…>`
- `T3ImageLock.attestations: Vec<T3ImageAttestation>`
- `CaptureChannel.events: Arc<Mutex<Vec<NotificationEvent>>>`
- `GatewayInstance.{submodule: Arc<dyn GatewaySubmodule>, cancel_flag: Arc<AtomicBool>}`
- `GatewayCancelHandle.flag: Arc<AtomicBool>`

**When** the dev triages EACH of the 72 fields with an explicit, recorded decision

**Then** each field is resolved by EXACTLY ONE of:
- **(a) PRIVATIZE (preferred default)** — demote the field to `pub(crate)` or private in the owning `crates/*/src/**` source; fix any now-broken cross-module access by routing through an accessor/constructor; rebuild + retest the touched crate
- **(b) WHITELIST (escape hatch)** — add the field to `xtask/i9-whitelist.toml` (for I9-holder-path fields) OR `xtask/pub-field-constructor-allowlist.toml` (for constructor-pattern fields consumed by-design across a crate boundary), with a ONE-LINE written justification per entry

**And** the DEFAULT is privatization; a whitelist entry without a written justification is a review-blocking finding (per the NOT clause)

**And** the dev record captures, per field (or per struct for bulk-privatized structs like `HotSwapCoordinator`'s 9 fields): the chosen decision (privatize/whitelist), the rationale, and the touched file path + line

**And** for whitelisted I9-holder-path additions, the dev confirms the addition is consistent with the I9 invariant (per `xtask/i9-whitelist.toml` header: "Adding a fourth entry … requires invariant-lock review per ADR-037") — if the addition touches the holder-path set, flag for invariant-lock review; constructor-pattern allowlist additions do NOT require ADR-037 review

**And** after all 72 are triaged, `cargo run -p xtask -- check-service-boundary` reports ZERO P3/P2/P1 rows (every violation is either privatized-away or whitelisted-with-justification)

### AC4 — Baseline-regen ↔ privatization reconciliation (the trickiest interaction)

**Given** privatizing a `pub` field to `pub(crate)`/private is itself a PUBLIC-SURFACE REMOVAL that the `abi-diff` + cargo-public-api baseline must reflect, and the AC2 baseline regen + AC3 privatizations interact

**When** the dev lands the 72 triage (AC3) and regenerates the baseline (AC2)

**Then** the SEQUENCE is: (1) apply AC3 privatizations to crate source FIRST; (2) THEN regenerate the AC2 baseline so it captures BOTH the 101 already-stale removals AND the new privatization-driven removals as a single coherent surface; (3) run `cargo public-api --diff` and confirm the net delta is **intentional-removal-only** (the privatized fields appear as deliberate Removed items; ZERO accidental Added/Changed rows)

**And** the `abi-diff` discipline gate AGREES the surface change is intentional — privatizations are deliberate encapsulation removals, and the regenerated baseline + abi-diff posture stay internally consistent (per `[[project_story_7_5a_landed]]` the abi-diff posture is Added-only at v1.0; a DELIBERATE removal in a bridge-reset must be explicitly reconciled, NOT auto-blessed — the dev documents WHY each removal is safe: the fields were never a sanctioned consumer-facing surface, they were I9 encapsulation leaks)

**And** the dev record documents this reconciliation explicitly as the trickiest part: the order-of-operations (privatize → regen → diff), the count of privatization-driven removals vs stale-baseline removals, and the confirmation that `cargo public-api --diff` shows no accidental surface drift

**And** if the abi-diff gate's Added-only posture would HARD-FAIL on the deliberate removals, the dev SURFACES the conflict (the removal IS intentional but the gate may treat any removal as a breaking change) and resolves it per the abi-baseline mechanism (regenerate the abi-baseline `txt` under `abi-baseline/` / `xtask/abi-baseline/` to match the post-privatization surface) — NEVER by re-exposing the field

### AC5 — `check-serde-error-handling`: explicit remediate-vs-freeze posture decision; gate green-or-frozen-documented

**Given** the `check-serde-error-handling` gate is SOFT (`continue-on-error: true` at `discipline.yml:~1008`, comment "calibration phase per Epic 5 §A3 — flip to hard-fail post Story 6.3") with ~300 violations across ~83 files — `serde_json::from_str(...).unwrap(...)` (and sibling `.unwrap_or_default()`/`.expect(...)`) patterns concentrated in `maos-domain/invariants/*` and `maos-audit/src/lib.rs` (e.g. lines 803, 856) — and the gate already supports `load_allowlist()` + inline `// xtask-serde-allow` markers

**When** the dev makes an EXPLICIT, RECORDED posture decision between:
- **(d-i) REMEDIATE** — convert the 300 sites to `.map_err(|e| <CrateError>::Serialize(e.to_string()))?` propagation (or the crate-appropriate typed error), driving the violation count to 0, THEN flip `continue-on-error: true` → removed (hard-fail)
- **(d-ii) FREEZE** — capture the current 300 sites into a frozen baseline allowlist (file-based via `load_allowlist()` + inline `// xtask-serde-allow` where appropriate), so the gate fails ONLY on NEW violations beyond the frozen set, THEN optionally flip to hard-fail (hard-failing NEW violations only)

**Then** the chosen posture is documented VERBATIM in this AC's completion note AND in the dev record, with the rationale (e.g. "remediate is ~300 sites across 83 files exceeding the bridge budget → FREEZE a baseline allowlist + hard-fail NEW; full remediation deferred to a tracked follow-up" OR "remediate is bounded → convert all 300 + hard-fail")

**Then** the gate is GREEN-at-HEAD under the chosen posture:
- under (d-i): `cargo run -p xtask -- check-serde-error-handling` exits 0 (zero violations), `continue-on-error: true` removed
- under (d-ii): the frozen allowlist is committed; `cargo run -p xtask -- check-serde-error-handling` exits 0 against the frozen baseline; if the gate is flipped to hard-fail, the removal of `continue-on-error: true` is in the final commit; the frozen-baseline file + its provenance comment ("frozen at Story 7.1.7; N=<count> sites; remediation tracked in <follow-up>") is recorded

**And** if (d-ii) FREEZE is chosen, the dev records the FOLLOW-UP closure target for the deferred full remediation (so the frozen baseline does not silently become permanent debt per `[[feedback_mechanical_gates_compound_promises_decay]]`)

**And** the RECOMMENDED posture (non-binding guidance for the dev): if the 300-site remediation is too large for the bridge budget, FREEZE the baseline allowlist and gate only NEW violations, then flip to hard-fail — this locks the discipline forward without blocking the bridge on a 300-site refactor

### AC6 — `coverage-matrix` re-confirmed green + `coverage-matrix-nfr-test-3` clean + §A4 hook-count truthful at 14

**Given** the Epic 7 retro OVERSTATED coverage-matrix as "~19 gaps"; the gate exits 0 at HEAD (deferred-NFR-Test-N lines are informational/by-design), and a prior `coverage-matrix-nfr-test-3` probe hit a SHELL error not a gate error

**When** the dev re-runs the coverage gates and the hook-count assertion

**Then**:
- `cargo run -p xtask -- coverage-matrix` exits 0 — RE-CONFIRMED green; the deferred-NFR-Test-N lines are documented as informational, NOT gaps requiring work
- `cargo run -p xtask -- coverage-matrix --measure-nfr-test-3 --dry-run` (the `coverage-matrix-nfr-test-3` job command) exits 0 cleanly — confirming the prior probe's failure was a shell-invocation error, not a gate error
- IF either coverage gate genuinely fails at HEAD, the dev SURFACES and re-scopes (per the NOT clause); otherwise ZERO coverage work lands

**And** the §A4 hook-count is CONFIRMED truthful at 14:
- `xtask/spirit-abi-hook-count.toml` `expected_count = 14`
- `check-service-boundary`'s hook-count assertion reports 14
- the 14-entry `[[hooks]]` list matches the `Spirit` trait surface at HEAD (no `epistemic_resolve` 15th hook on the trait unless HEAD already carries it — verify, do not bump)
- this is a VERIFICATION, not a re-litigation; the dev records "hook-count confirmed 14 (truthful per prior Epic 7 reconciliation)" in the dev record

**And** the green-at-HEAD success criteria are met as a single coherent close:
- `cargo run -p xtask -- check-service-boundary` exits 0 (101 stale cleared + 72 real triaged + baseline reconciled)
- `cargo run -p xtask -- check-serde-error-handling` exits 0 (remediated OR frozen-baseline-clean, posture documented)
- `cargo run -p xtask -- coverage-matrix` exits 0 (re-confirmed) AND `coverage-matrix-nfr-test-3` clean
- `cargo public-api --diff` shows intentional-removal-only (no accidental drift)

**And** local verification before `done` (all exit 0 / clean):
- `cargo run -p xtask -- check-service-boundary`
- `cargo run -p xtask -- check-serde-error-handling`
- `cargo run -p xtask -- coverage-matrix`
- `cargo run -p xtask -- coverage-matrix --measure-nfr-test-3 --dry-run`
- `cargo public-api --diff` (intentional-removal-only)
- `cargo test -p <each touched crate>` for every crate whose source was edited by AC3 privatizations

**And** the Story 7.1.7 PR is committed in logical commits preserving the bisect surface:
- **Commit 1**: AC1 classification (record-only / no source change)
- **Commit 2**: AC3 72-field triage (privatizations to crate source + whitelist additions) — the crate-code commit; run touched-crate tests
- **Commit 3**: AC2 baseline regen reconciled with AC4 — regenerate kernel-surface + abi-baseline AFTER privatizations; confirm `cargo public-api --diff` intentional-removal-only
- **Commit 4**: AC5 serde posture (remediation OR frozen allowlist + optional `continue-on-error` removal) + AC6 coverage re-confirm record + §A4 hook-count verification note (the closure commit)

## Tasks / Subtasks

- [x] **Task 0 (AC1)** — Classify bridge preconditions; record at-HEAD counts; confirm 101-vs-72 split
  - [x] Subtask 0.1 — Ran `check-service-boundary`; captured 328-line output; recorded counts (101 removed / 155 class-other / 2 P1 / 7 P2 / 63 P3)
  - [x] Subtask 0.2 — Confirmed 101 `removed public` are stale-baseline + 72 P-class carry the I9 cross-ref; enumerated set present
  - [x] Subtask 0.3 — Ran serde + coverage + hook-count probes; recorded soft-fail present, 300 serde, coverage-matrix exit 0, **nfr-test-3 exit 1 (drift, D6)**, hook-count 14
  - [x] Subtask 0.4 — Drift found (structural: D1-D6). STOPPED and reported actual counts; surfaced scope correction before AC2+
  - [x] Subtask 0.5 — Commit 1: AC1 classification record (Completion Notes citation) — recorded in Dev Agent Record (commits left to Lunarpulse; see Change Log)

- [x] **Task 1 (AC3)** — Triage the 72 REAL boundary violations (mechanism corrected: `#[i9_exempt]` + gate-correctness, NOT privatization — see AC1-D2)
  - [x] Subtask 1.1 — `HotSwapCoordinator` (all 10 flagged fields): `#[maos_attrs::i9_exempt(reason=…)]` (one attr covers all fields) + i9-exemptions.md entry; rebuilt + retested maos-kernel-core
  - [x] Subtask 1.2 — `T3ImageLock.attestations` — `#[i9_exempt]` (bounded config state); recorded
  - [x] Subtask 1.3 — `CaptureChannel.events` — RECLASSIFIED: it is a `#[cfg(test)]` test double in `security/approval.rs` (spec enumerated it as production in error); cleared at root by the I9-walker `#[cfg(test)]`-skip, NOT exempted
  - [x] Subtask 1.4 — `GatewayInstance.{submodule, cancel_flag}` + `GatewayCancelHandle.flag` — `#[i9_exempt]` (per-process transient gateway state); recorded
  - [x] Subtask 1.5 — Swept the full P-class set: 13 production state-holders → `#[i9_exempt]`; 19 test-double false-positives (8 in tests/ + benches/, 5 in src `#[cfg(test)]` mods, +`MockLifecycleResolver`) → gate-correctness fix; P1 (2) false-positives + P2 (7) per Lunarpulse's decision
  - [x] Subtask 1.6 — Exemptions recorded in `docs/invariants/i9-exemptions.md` (15 new entries incl. the previously-undocumented `GatewayDispatcher`); P2 (7) added to `ADAPTER_PORT_EXEMPTIONS` with written justifications. No `i9-whitelist.toml` holder-path additions (no ADR-037 review needed)
  - [x] Subtask 1.7 — Re-ran `check-service-boundary`; ZERO P1/P2/P3 rows remain
  - [x] Subtask 1.8 — `cargo test -p maos-kernel-core` / `-p maos-bin` / `-p maos-spirit-hello` / `-p xtask --bins` all green (202 gate unit tests pass)
  - [x] Subtask 1.9 — Commit 2 (AC3): changes staged as a logical unit (see File List); git commit left to Lunarpulse

- [x] **Task 2 (AC2 + AC4)** — Baseline regen reconciled (AC4 interaction MOOT — no privatization, see AC1-D3)
  - [x] Subtask 2.1 — Baseline = `docs/ci-baselines/kernel-surface-v0.1-beta.json` (per `check_service_boundary.rs` default + main.rs `--baseline`); classes = `xtask/kernel-api-classes.toml`
  - [x] Subtask 2.2 — Regenerated AFTER all `#[i9_exempt]` attrs landed (the attr changes each struct's `signature_hash` since `canonicalize_signature` quotes attrs) — captures the 101 stale removals AND the 155 re-classified additions in one coherent surface (154 → 332 items)
  - [x] Subtask 2.3 — `abi-diff --base abi-baseline/v1-pre-bump.txt` (the CI form): PASSED with AND without my changes — zero public-API delta (the `#[i9_exempt]` attr is pass-through; no field privatized → no surface change at all, the cleanest-possible AC4 outcome)
  - [x] Subtask 2.4 — N/A: no `abi-baseline` regen needed — no deliberate removals occurred (AC4-D3). The local `abi-diff` default (`HEAD~1`) fails on a PRE-EXISTING `maos-domain/iac_bus.rs` async-fn rendering diff, unrelated to this story and not the CI gate form
  - [x] Subtask 2.5 — Re-ran `check-service-boundary`: 256 NFR-Test-2 rows (101 removed + 155 class-other) → 0, P-class 72 → 0 → gate exits 0
  - [x] Subtask 2.6 — Reconciliation documented in Completion Notes (attr-first → regen-last ordering; signature_hash mechanism; no accidental drift; abi-diff unchanged)
  - [x] Subtask 2.7 — Commit 3 (AC2+AC4): staged

- [x] **Task 3 (AC5)** — `check-serde-error-handling`: FREEZE posture (Lunarpulse's decision)
  - [x] Subtask 3.1 — Re-ran gate: 300 violations across 83 files; concentration confirmed (maos-domain/invariants/*, maos-audit/src/lib.rs:803,856, maos-eval/isolation_corpus.rs, + many test files)
  - [x] Subtask 3.2 — DECIDED: **FREEZE** baseline allowlist + gate-NEW (Lunarpulse-selected; rationale: 300 sites across 83 files, a large share in test/corpus code, exceeds the bridge budget)
  - [x] Subtask 3.3a — N/A (remediate not chosen)
  - [x] Subtask 3.3b — FREEZE: generated `xtask/serde-error-allowlist.toml` (300 `location = "file:line"` entries via the gate's `load_allowlist()`) with provenance + follow-up comment; gate exits 0 against the frozen set
  - [x] Subtask 3.4 — Flipped to HARD-FAIL: removed `continue-on-error: true` from `discipline.yml` `check-serde-error-handling` (deletion; no `continue-on-error: false` added) — gate now hard-fails on NEW sites only
  - [x] Subtask 3.5 — Commit 4 part 1 (AC5): staged

- [x] **Task 4 (AC6)** — Coverage re-confirm + §A4 hook-count + final green-at-HEAD
  - [x] Subtask 4.1 — `coverage-matrix` exit 0 (deferred-NFR-Test-N informational); `coverage-matrix --measure-nfr-test-3 --dry-run` exit 0 — but CORRECTION (AC1-D6): nfr-test-3 was GENUINELY red (exit 1, missing `crates/maos-spirit-hello/manifest.toml`), NOT a shell error. Fixed in-scope per Lunarpulse by authoring the manifest
  - [x] Subtask 4.2 — `xtask/spirit-abi-hook-count.toml` `expected_count = 14` confirmed; `check-service-boundary` (which runs the hook-count assertion) passes → 14 truthful; NOT bumped
  - [x] Subtask 4.3 — Full green-at-HEAD suite re-run: all 6 commands exit 0 (+ check-empty-kernel bonus); idempotent on re-run
  - [x] Subtask 4.4 — `sprint-status.yaml`: `7-1-7-…: ready-for-dev → in-progress → review` (dev-story workflow sets `review`; the §A5 review gate flips to `done`); no other row touched
  - [x] Subtask 4.5 — Commit 4 part 2 (AC6): staged
  - [x] Subtask 4.6 — Story 7.1.7 dev record + Review Findings table left in valid state for the §A5/§A6 gates

## Dev Notes

### Relevant patterns and constraints

- **This story is NOT zero-crate-code — the load-bearing divergence from 7.1.5/7.1.6.** The 72-field triage's preferred resolution (privatization) edits `crates/*/src/**` field visibility. A `pub` → `pub(crate)` demotion IS a public-surface removal. Treat each privatization as a real crate edit: rebuild the owning crate, fix now-broken cross-module access via accessors/constructors, and re-run that crate's test suite. Flag this divergence explicitly in the dev record.

- **The 101-vs-72 distinction is the retro's under-scope correction.** The Epic 7 retro (`[[project_epic_7_retro_outcomes]]` Decision 2) treated `check-service-boundary` as "101 stale violations". The gate output at HEAD is 328 lines = ~101 stale `removed public` (rebaseline-eligible) + ~72 REAL P3/P2/P1 boundary violations (triage-required). Conflating them is the trap: rebaselining all 328 would RE-BLESS the 72 encapsulation leaks; leaving all 328 as violations would FALSE-FAIL forever. The fix is class-aware: regen the 101, triage the 72.

- **The baseline-regen ↔ privatization reconciliation is the trickiest interaction (AC4).** Order matters: privatize the 72 fields FIRST (those become deliberate surface removals), THEN regenerate the cargo-public-api baseline so it captures both the 101 already-stale removals AND the privatization-driven removals in one coherent surface. Running the regen BEFORE the privatizations would produce NEW `removed public` rows on the next gate run (the now-private fields). Confirm `cargo public-api --diff` is intentional-removal-only after both land.

- **The abi-diff Added-only posture (v1.0) must be reconciled, not bypassed.** Per `[[project_story_7_5a_landed]]` the abi-diff posture is Added-only at v1.0. A bridge-reset privatization is a DELIBERATE removal — document WHY each removal is safe (the fields were I9 encapsulation leaks, never a sanctioned consumer-facing surface) and regenerate the abi-baseline `txt` if the gate would otherwise hard-fail. NEVER re-expose a field to satisfy Added-only; the removal IS the intended encapsulation fix.

- **The serde gate has been soft for 2 epics — decide, don't decay.** `discipline.yml:~1008` comment: "calibration phase per Epic 5 §A3 — flip to hard-fail post Story 6.3". It's now 2 epics overdue. Per `[[feedback_mechanical_gates_compound_promises_decay]]`, the bridge must make the EXPLICIT remediate-vs-freeze call. The gate already supports `load_allowlist()` + inline `// xtask-serde-allow` markers, so the freeze path (d-ii) needs NO new gate code — just a frozen baseline file + a `continue-on-error` deletion. Recommended (non-binding): if 300 sites across 83 files exceed the bridge budget, FREEZE + gate-new + flip-hard, with a tracked follow-up for full remediation.

- **DO NOT over-scope coverage-matrix.** The retro's "~19 gaps" is wrong; `coverage-matrix` exits 0 at HEAD (deferred-NFR-Test-N lines are by-design informational). RE-CONFIRM green + run `coverage-matrix-nfr-test-3` cleanly (a prior probe hit a SHELL error, not a gate error). If both exit 0, this gate needs no work. Honor `[[feedback_lunarpulse_observability_preference]]` — observe the gate exit 0, don't infer from coverage%.

- **§A4 hook-count is a VERIFICATION, not a decision.** `xtask/spirit-abi-hook-count.toml` already states `expected_count = 14` (FR55 Epic-2 baseline of 11 + Story 5.2's `on_swap_out`/`snapshot`/`migrate` per ADR-017/020; reconciled to truthful 14 in prior Epic 7 work per `[[project_story_7_5a_landed]]`). Confirm the gate reports 14 and the trait surface matches; do NOT bump to 15 (`epistemic_resolve` stays a forward-shape note).

- **LESSON from `[[project_story_7_5a_landed]]`: never `cargo fmt -p crate` here** — it triggers whole-crate collateral. When privatizing fields, edit surgically; do not run crate-wide formatters as part of the visibility change.

- **Sequencing: 7.1.7 runs FIRST or CONCURRENTLY with 7.1.6.** A clean `check-service-boundary` reduces noise in 7.1.6's other gate runs so the §A2 full-flip story does not read around a 328-line service-boundary failure. The two bridges are scope-isolated: 7.1.7 touches service-boundary/serde/coverage/visibility; 7.1.6 touches the 2 §A2 gates + story-file RF/`dev_model_used`. Neither touches the other's surface.

- **Recommended dev model: `claude-opus-4-8`** (per the Epic 7 cadence and the crate-code triage judgment this story requires).

### Source tree components to touch

| Path | Disposition | Why |
|---|---|---|
| `crates/*/src/**` (HotSwapCoordinator, T3ImageLock, CaptureChannel, GatewayInstance, GatewayCancelHandle + remaining P-class owners) | UPDATE (visibility) | AC3 — privatize the 72 REAL boundary fields to `pub(crate)`/private (the non-zero-crate-code part) |
| `xtask/i9-whitelist.toml` | UPDATE (conditional) | AC3 — whitelist any field that must stay public (holder-path; ADR-037 review if applicable) with justification |
| `xtask/pub-field-constructor-allowlist.toml` | UPDATE (conditional) | AC3 — whitelist constructor-pattern fields consumed by-design across a crate boundary, with justification |
| kernel-surface baseline file(s) the `check-service-boundary` gate diffs (per `check_service_boundary.rs` load path) | REGEN | AC2 — clear the 101 stale `removed public` rows + capture privatization-driven removals |
| `abi-baseline/*.txt` / `xtask/abi-baseline/*.txt` | REGEN (conditional) | AC4 — reconcile the abi-diff Added-only posture with the deliberate privatization removals |
| serde frozen-baseline allowlist file (if freeze posture) + inline `// xtask-serde-allow` markers | NEW/UPDATE (conditional) | AC5 — freeze the 300 sites if remediation exceeds bridge budget |
| `crates/maos-domain/src/invariants/**`, `crates/maos-audit/src/lib.rs` (e.g. :803,:856) | UPDATE (conditional) | AC5 — if remediate posture: convert serde sites to `.map_err(…Serialize…)?` |
| `.github/workflows/discipline.yml` (~line 1008) | UPDATE (conditional) | AC5 — remove `continue-on-error: true` from `check-serde-error-handling` IF flipping to hard-fail |
| `xtask/spirit-abi-hook-count.toml` | READ-ONLY (verify) | AC6 — confirm `expected_count = 14`; do NOT edit |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | UPDATE (at done) | sprint-status `7-1-7-…: done`; preserve `epic-7: in-progress`; do NOT touch other rows |

### Testing standards summary

- **Local verification before `done` (all exit 0 / clean):** `check-service-boundary`, `check-serde-error-handling`, `coverage-matrix`, `coverage-matrix --measure-nfr-test-3 --dry-run`, `cargo public-api --diff` (intentional-removal-only), and `cargo test -p <each touched crate>` for every crate whose source was edited by AC3.
- **CI verification post-PR:** the discipline.yml pipeline runs the baseline-reset gates; `check-service-boundary` (already hard-fail, no `continue-on-error`) must now PASS at HEAD; the serde gate is green under the chosen posture; coverage gates confirmed green.
- **Re-runnability / idempotence:** after the reset, re-running `check-service-boundary` / `check-serde-error-handling` / `coverage-matrix` on an unchanged workspace produces identical exit-0 output — the mechanical regression substrate Stories 8.1+ inherit.
- **Reconciliation proof:** `cargo public-api --diff` after the privatize → regen → diff sequence shows ONLY intentional removals (no accidental Added/Changed); this is the AC4 evidence.

### Project Structure Notes

- **Alignment with unified project structure.** Visibility edits stay within the existing owning crates; no new workspace member (count stays at Epic-7's 30). Whitelist additions follow the established `xtask/*-allowlist.toml` / `xtask/i9-whitelist.toml` schema. The serde freeze path reuses the gate's existing `load_allowlist()` mechanism — no new xtask gate.
- **Detected conflicts or variances (with rationale).**
  - Workspace-count gate (`check-workspace-count`) stays unchanged — 7.1.7 adds ZERO crates.
  - `check-service-boundary` is ALREADY hard-fail at HEAD (no `continue-on-error`); the 328-line exit-1 is a real CI blocker, which is why this baseline-reset gates Epic 8 (per retro Decision 2).
  - The serde `continue-on-error: true` removal (if flipping) is a DELETION of the soft-fail line; YAML semantics make missing-field == fail-fast default. Do NOT add `continue-on-error: false`.
  - The abi-diff Added-only posture (v1.0, per `[[project_story_7_5a_landed]]`) conflicts with deliberate privatization removals — resolved by regenerating the abi-baseline `txt` to match the post-privatization surface, documenting each removal as a safe I9-leak fix. This is the one place the bridge intentionally removes public surface; it is reconciled, not bypassed.
  - §A4 hook-count is verify-only; `xtask/spirit-abi-hook-count.toml` is read-only in this story.

### References

- [Source: Memory `[[project_epic_7_retro_outcomes]]` — retro Decision 2: dedicated baseline-reset story 7.1.7 (check-service-boundary 101 stale P0, serde 300 baseline, coverage-matrix gaps, §A4 hook-count 14); Epic 8 critical path = §A1 7.1.6 → §A2 baseline-reset → 8.1 Butler]
- [Source: Memory `[[feedback_mechanical_gates_compound_promises_decay]]` — soft-fail gates decay across epics; ship the closure in the SAME story; serde gate soft since Epic 5 §A3]
- [Source: Memory `[[project_story_7_5a_landed]]` — abi-diff Added-only v1.0 posture; §A4 reconciled to truthful 14; check-service-boundary 101 stale-baseline pre-existing P0; LESSON never `cargo fmt -p crate`]
- [Source: Memory `[[feedback_lunarpulse_observability_preference]]` — observe the gate exit 0, do not infer from coverage%; coverage-matrix re-confirm not re-author]
- [Source: Memory `[[feedback_story_sizing]]` — bridge bundles three green-at-HEAD workstreams under one story]
- [Source: .github/workflows/discipline.yml:251 — `check-service-boundary` (hard-fail, cargo-public-api, exit 1 at HEAD)]
- [Source: .github/workflows/discipline.yml:1008 — `check-serde-error-handling` `continue-on-error: true` "calibration phase per Epic 5 §A3 — flip to hard-fail post Story 6.3"]
- [Source: .github/workflows/discipline.yml:654,564 — `coverage-matrix` + `coverage-matrix-nfr-test-3` jobs]
- [Source: xtask/src/check_service_boundary.rs — KernelSurface baseline diff (`removed public`), P3/I9 violations (`see check-empty-kernel for full I9 context`), i9-whitelist/i9-denylist load, hook-count assertion]
- [Source: xtask/src/check_serde_error_handling.rs — `load_allowlist()` + inline `// xtask-serde-allow` / `// allow(serde-unwrap)` freeze-baseline capability]
- [Source: xtask/spirit-abi-hook-count.toml — `expected_count = 14`; 14-entry hook attribution; `epistemic_resolve` forward-shape note]
- [Source: xtask/i9-whitelist.toml — holder-path schema; ADR-037 invariant-lock for new holder-path entries]
- [Source: xtask/pub-field-constructor-allowlist.toml — constructor-pattern public-field escape hatch]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml:80 — `7-1-7-…: backlog` (entry pre-exists; do NOT add)]

## Dev Agent Record

### Agent Model Used

claude-opus-4-8[1m] (Opus 4.8, 1M context) — per story recommendation.

### Debug Log References

- `check-service-boundary` at HEAD: `/tmp/sb_out.txt` (447 lines, EXIT=1, 328 violations).
- `check-serde-error-handling` at HEAD: `/tmp/serde_out.txt` (EXIT=1, 300 violations / 83 files).
- `coverage-matrix` at HEAD: `/tmp/cov_out.txt` (EXIT=0 ✓).
- `coverage-matrix --measure-nfr-test-3 --dry-run` at HEAD: `/tmp/cov3_out.txt` (EXIT=1 ✗ — see AC1-D6).

### Completion Notes List

#### AC1 — at-HEAD counts recorded VERBATIM (the scope floor + the actual drift)

`cargo run -p xtask -- check-service-boundary` → **EXIT 1, 328 violation lines**, split into THREE classes (the spec named only two):

| Class | Spec floor | **Actual at HEAD** | Clearing mechanism (verified against gate source) |
|---|---|---|---|
| `removed public kernel symbol` (stale `KernelSurface` JSON baseline) | 101 | **101** | Regenerate `docs/ci-baselines/kernel-surface-v0.1-beta.json` |
| `new public … has class 'other'` (added-since-baseline, unclassified) | **(omitted by spec)** | **155** | **Same baseline regen** — once the baseline captures the item it is no longer "added", so no classification is required (`check_service_boundary.rs:196-216`) |
| P-class boundary (P1+P2+P3) | 72 | **72** (2 P1 + 7 P2 + 63 P3) | see below — **NOT** privatization (see AC1-D2) |

101 + 155 + 72 = **328** ✓ (the spec's "101 + 72" sums to 173; the 155 `class 'other'` rows were unaccounted).

Other probes at HEAD:
- `check-serde-error-handling` → **EXIT 1, 300 violations / 83 files** (`serde_json::{from_str,to_string,…}` + `.unwrap/.expect/.unwrap_or_*`); a large share are in **test files** (`crates/maos-audit/tests/*.rs`, `crates/maos-eval/src/isolation_corpus.rs`). `continue-on-error: true` confirmed present at `discipline.yml` `check-serde-error-handling` block.
- `coverage-matrix` → **EXIT 0 ✓** (deferred-NFR-Test-N lines are informational/by-design, exactly as the spec predicted).
- `coverage-matrix --measure-nfr-test-3 --dry-run` → **EXIT 1 ✗** (see AC1-D6).
- `xtask/spirit-abi-hook-count.toml` `expected_count = 14` ✓; 14-entry `[[hooks]]` list present (verify-only, not bumped).

#### AC1 — SCOPE-DRIFT findings (`scope_drift_surfaced`) — the spec materially mismodels this gate

- **D1 — 155 unanticipated `class 'other'` rows.** The spec's "328 lines = 101 + 72" omitted the 155 `new public … has class 'other'` classification rows. They share a root cause with the 101 (a stale `KernelSurface` baseline) and clear in the **same** mechanical regen — so AC2's scope is "256 NFR-Test-2 rows → 0", not "101 → 0".
- **D2 — P3 clears via `#[i9_exempt]`, NOT privatization.** The I9 walker (`check_empty_kernel.rs:224-238`) flags a denylisted-type field **regardless of its visibility** — there is no `is_pub` check. Demoting `pub`→`pub(crate)` (AC3's "preferred default") therefore **does not clear a P3 violation**. The only mechanisms are (a) `#[i9_exempt(reason="…")]` on the struct + a matching entry in `docs/invariants/i9-exemptions.md`, or (b) relocating the struct into an `i9-whitelist.toml` holder-path dir (ADR-037-gated). The enumerated set (`HotSwapCoordinator.{…}`, `T3ImageLock.attestations`, `CaptureChannel.events`, `GatewayInstance.{…}`, `GatewayCancelHandle.flag`) is confirmed present in the output.
- **D3 — AC4 "the trickiest interaction" is MOOT.** Because the P3 fix is an attribute (`#[i9_exempt]`), no field becomes private, so the public surface is unchanged → `cargo public-api`/`abi-diff` see **zero** deliberate removals. The privatization↔baseline reconciliation the story centers AC4 on does not arise. (The `KernelSurface` snapshot tracks only pub `fn/struct/enum/trait/type/const/static/use` — never fields — so even a hypothetical privatization would be invisible to *this* baseline.)
- **D4 — P1 (2) are FALSE POSITIVES; P1/P2 escape hatches are gate SOURCE constants, not config.** `check_p1_single_owner` counts every `<Adapter>::new` in `crates/maos-bin/src/main.rs` with no `#[cfg(test)]`/boot-path scoping. The flagged sites — `SecurityManagerAdapter::new` ×3 (lines 814, 3283, 5056) and `IacBusAdapter::new` ×2 (284, 3577) — are mostly inside standalone **smoke-subcommand fns** (`smoke_abi_7_5a` @5032, `smoke_orchestrator_fanout_6_2`, the inline admit tests), each of which legitimately builds a throwaway adapter; only line 284 is the production boot path. P2 (7) is `api::*` adapter exports lacking a `maos_domain::ports::<Port>` re-export. P1's escape hatch is the `SERVICE_ADAPTERS` const and P2's is the `ADAPTER_PORT_EXEMPTIONS` const — both in `check_service_boundary.rs`, i.e. clearing them edits the **gate itself**, not a config file.
- **D5 — serde-300 spreads into test code** (above) → the spec-recommended **FREEZE** posture is the pragmatic call.
- **D6 — `coverage-matrix-nfr-test-3` GENUINELY FAILS (EXIT 1), contradicting the spec.** The spec (AC6 / precondition COVERAGE-MATRIX-GREEN) asserts a prior probe's failure "was a SHELL error not a gate error" and expects exit 0. At HEAD the gate exits **1** with `spirit hello-spirit: manifest not found at crates/maos-spirit-hello/manifest.toml` — a real gate failure, and `coverage-matrix-nfr-test-3` **is** in the CI hard-gate `needs:` list (`discipline.yml:1536`). This is a real blocker the spec dismissed; per the AC6 NOT-clause the dev SURFACES and re-scopes.

**AC1 row classification:** `EPIC-7-RETRO-DONE`=verify_pass · `SB-EXIT-1-AT-HEAD`=verify_pass (328 confirmed) · `SB-101-STALE-CLASS`=verify_pass · `SB-72-REAL-CLASS`=verify_pass · `SERDE-SOFT-AT-HEAD`=verify_pass · `SERDE-ALLOWLIST-CAP`=verify_pass · `COVERAGE-MATRIX-GREEN`=**scope_drift_surfaced (nfr-test-3 exit 1, D6)** · `A4-HOOK-COUNT-14`=verify_pass · `7.1.6-CONCURRENCY`=verify_pass (scope-isolated) · `ABI-DIFF-RECONCILE-READY`=verify_pass (but D3 makes it a no-op). Plus structural drift D1-D4 above.

> **AC1 STOP-and-report gate (Subtask 0.4) reached.** The drift is structural, not numeric: the spec's prescribed remediations (AC3 privatize; AC4 reconciliation) do not match the gate, 155 rows are unaccounted, and a gate the spec called green (nfr-test-3) is red. Surfaced to Lunarpulse for a scope-correction decision before AC2+ implementation opens.

**Lunarpulse's 3 scope-correction decisions (2026-06-01):** (1) P1/P2 → **fix gate correctness + exempt P2**; (2) nfr-test-3 (D6) → **investigate then fix in-scope**; (3) serde → **freeze baseline + hard-fail NEW**.

#### AC3 — 72 P-class triaged via the ACTUAL gate mechanism (not privatization)

The I9 walker (`check_empty_kernel`) ignores field visibility, so the 63 P3 rows were triaged by class:
- **13 production kernel state-holders → `#[maos_attrs::i9_exempt(reason=…)]` + `docs/invariants/i9-exemptions.md` entry** (one attr per struct covers all its fields): `SpiritControlBlock`, `SpiritSchedulerAdapter`, `HotSwapCoordinator`, `PostSwapMonitor`, `PostSwapInvariantSnapshot`, `KernelLifecycleResolver`, `IdleWatchdog`, `HookDispatcher`, `WorkingMemoryOrchestrator`, `GatewayInstance`, `GatewayCancelHandle`, `T3ImageLock`, `ScbTracker`. Each is a genuine supervision-tree state owner (the scheduler is the supervisor per §4.0.8; the INFO payload already reports `spirit-scheduler.p3 = supervisor-exception`). Also documented the previously-undocumented `GatewayDispatcher` exempt (carried since Story 6.5) — that was the 64th `check-empty-kernel` violation.
- **19 test-double false-positives → gate-correctness fix at root** (Lunarpulse-endorsed philosophy): I9 governs production kernel state only, so the walker is now scoped to `src/` (excludes the 8 structs in `tests/` + `benches/`) and skips `#[cfg(test)]` modules (excludes 5 in-src test doubles incl. `CaptureChannel`, which the spec's enumerated set wrongly called production). `MockLifecycleResolver` (a `pub mod test_double` deliberately NOT `#[cfg(test)]`-gated, shipped for external test consumers) got an explicit `#[i9_exempt]` since the walker correctly still sees it.
- **P1 (2) false-positives → gate-correctness fix**: `check_p1_single_owner` counted every `<Adapter>::new` in `maos-bin/main.rs`, flagging standalone `smoke_*` CLI-subcommand handlers and a one-shot admission probe as "double construction." The P1 visitor now skips `smoke_*` fns and honors an inline `// p1-allow:` marker (added to the transient admission probe at main.rs); SecurityManagerAdapter and IacBusAdapter now count exactly 1 (the production boot path).
- **P2 (7) → `ADAPTER_PORT_EXEMPTIONS` const additions with written justifications** (Lunarpulse: exempt P2): `IacBusAdapter`, `take_io_journal`, `SetScalarError`, `WorkingMemorySlot`, `WorkingMemoryStore`, `HotSwapCoordinator`, `McpClientAdapter` — none is an adapter↔port pair requiring a Port trait at the v0.1-β services-as-modules layout.

Result: `check-service-boundary` P1=0 P2=0 P3=0; `check-empty-kernel` 0 violations (was 64, a third red gate this story also greened).

#### AC2 + AC4 — baseline regen reconciled; AC4 interaction proved moot

Regenerated `docs/ci-baselines/kernel-surface-v0.1-beta.json` (154 → 332 items) from `check-service-boundary --json .current_surface`, **after** all `#[i9_exempt]` attrs landed — because `canonicalize_signature` quotes the full item including attributes, so the attr changes each annotated struct's `signature_hash`; regenerating last captures the final surface. This single regen cleared BOTH the 101 `removed public` AND the 155 `class 'other'` rows (an item the baseline now contains is no longer "added," so needs no classification). **AC4's privatization↔baseline reconciliation never arose**: no field was privatized (the `#[i9_exempt]` attr is pass-through), so the public API is unchanged — `abi-diff --base abi-baseline/v1-pre-bump.txt` (the CI form) PASSES identically with and without my changes (verified by stash-test). No `abi-baseline` regen needed. (The local `abi-diff` default `--base HEAD~1` fails on a PRE-EXISTING `maos-domain/iac_bus.rs` async-fn rendering diff between HEAD~1 and HEAD — unrelated to this story and not the CI gate invocation.)

#### AC5 — serde FREEZE posture (verbatim)

> **Posture: FREEZE.** Remediating all 300 `serde_json::{from_str,to_string,to_vec,from_slice}` + `.unwrap()/.expect()/.unwrap_or_*()` sites across 83 files — a large share in test/corpus code — exceeds the bridge budget. Froze the current 300 sites into `xtask/serde-error-allowlist.toml` (file-based `location = "file:line"` via the gate's existing `load_allowlist()`), so the gate hard-fails ONLY on NEW sites beyond the frozen set, and removed `continue-on-error: true` from `discipline.yml` `check-serde-error-handling`. **Follow-up closure target:** Story 8.x serde-error-handling remediation — convert the frozen sites to `.map_err(|e| <CrateError>::Serialize(e.to_string()))?` crate-by-crate, shrinking the allowlist to empty (recorded in the allowlist provenance header so the freeze does not silently become permanent debt per `[[feedback_mechanical_gates_compound_promises_decay]]`).

Result: `check-serde-error-handling` exits 0 against the frozen baseline; `continue-on-error` removed → hard-fail on NEW.

#### AC6 — coverage re-confirm, D6 fix, §A4 hook-count, final green-at-HEAD

- `coverage-matrix` exit 0 (re-confirmed; deferred-NFR-Test-N lines informational/by-design — NO coverage work landed).
- **D6 corrected**: `coverage-matrix-nfr-test-3 --dry-run` was GENUINELY red (exit 1) because `crates/maos-spirit-hello/manifest.toml` had never been committed (no git history) and the gate reads it for NFR-Test-3 capability reachability. Authored a truthful manifest: `hello-spirit-bench` is a pure echo-acknowledgement loop (reads `task.assign`, replies `task.complete`/`ok`; calls NO provider) → empty `[capabilities.required]` → the gate reports 100% coverage (nothing to exercise), matching the recorded `coverage_pct: 100`. Gate now exits 0.
- §A4 hook-count: `xtask/spirit-abi-hook-count.toml expected_count = 14` confirmed truthful; `check-service-boundary` (which runs the hook-count assertion) passes → 14. NOT bumped (`epistemic_resolve` stays a forward-shape note; HEAD `Spirit` trait carries 14).
- **Green-at-HEAD final suite (all exit 0):** `check-service-boundary`=0, `check-serde-error-handling`=0, `coverage-matrix`=0, `coverage-matrix-nfr-test-3`=0, `abi-diff --base abi-baseline/v1-pre-bump.txt`=0, `check-empty-kernel`=0 (bonus). Idempotent on re-run. Touched-crate tests green: `maos-kernel-core`, `maos-bin`, `maos-spirit-hello`, `xtask --bins` (202 gate unit tests pass). Pre-existing-unrelated: `xtask` `example_spirit_regen_integration` template-drift test fails identically with my changes stashed (NOT caused by 7.1.7).

**Scope discipline:** ZERO Epic 8 surface pre-staged; workspace stays 30 crates; no new ADR (encapsulation decisions apply the existing I9 invariant); §A2 story-file gates untouched (scope-isolated from concurrent 7.1.6). Crate-source edits (the `#[i9_exempt]` attrs + the main.rs `// p1-allow:` comment) flagged per the "NOT zero-crate-code" clause — all pass-through / comment-only, no runtime behavior change, confirmed by the full touched-crate test pass.

### File List

**Crate source (maos-kernel-core — `#[maos_attrs::i9_exempt]` additions, pass-through attr, no behavior change):**
- `crates/maos-kernel-core/src/scheduler/control_block.rs` — i9_exempt `SpiritControlBlock`
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs` — i9_exempt `SpiritSchedulerAdapter`
- `crates/maos-kernel-core/src/scheduler/verb_resolver.rs` — i9_exempt `KernelLifecycleResolver` + `MockLifecycleResolver`
- `crates/maos-kernel-core/src/scheduler/idle_watchdog.rs` — i9_exempt `IdleWatchdog`
- `crates/maos-kernel-core/src/scheduler/hook_dispatch.rs` — i9_exempt `HookDispatcher`
- `crates/maos-kernel-core/src/hot_swap/coordinator.rs` — i9_exempt `HotSwapCoordinator`
- `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs` — i9_exempt `PostSwapMonitor` + `PostSwapInvariantSnapshot`
- `crates/maos-kernel-core/src/capability/working_memory/orchestrator.rs` — i9_exempt `WorkingMemoryOrchestrator`
- `crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs` — i9_exempt `GatewayInstance` + `GatewayCancelHandle`
- `crates/maos-kernel-core/src/security/sandbox/t3/image_lock.rs` — i9_exempt `T3ImageLock`
- `crates/maos-kernel-core/src/iac.rs` — i9_exempt `ScbTracker`

**Crate source (maos-bin — comment only):**
- `crates/maos-bin/src/main.rs` — `// p1-allow:` marker on the transient admission-probe construction

**Gate logic (xtask):**
- `xtask/src/check_empty_kernel.rs` — scope I9 walk to `src/` + skip `#[cfg(test)]` modules (gate-correctness fix)
- `xtask/src/check_service_boundary.rs` — P1 visitor skips `smoke_*` fns + honors `// p1-allow:`; `ADAPTER_PORT_EXEMPTIONS` += 7 (P2)

**Config / baselines:**
- `xtask/serde-error-allowlist.toml` — **NEW**: frozen 300-site serde baseline (provenance + follow-up)
- `docs/invariants/i9-exemptions.md` — 15 new exemption entries (13 production holders + `GatewayDispatcher` + `MockLifecycleResolver`)
- `docs/ci-baselines/kernel-surface-v0.1-beta.json` — **REGEN** (154 → 332 items; captures post-`#[i9_exempt]` surface)
- `crates/maos-spirit-hello/manifest.toml` — **NEW**: truthful FR58 echo-acknowledgement manifest (D6 fix)
- `.github/workflows/discipline.yml` — removed `continue-on-error: true` from `check-serde-error-handling` (hard-fail flip)

**Tracking:**
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `7-1-7-…: ready-for-dev → in-progress → review`
- `_bmad-output/implementation-artifacts/7-1-7-…-green-at-head.md` — this story file (Dev Agent Record, tasks, status)

### Change Log

| Date | Change |
|---|---|
| 2026-06-01 | AC1: classified the 10 bridge rows; recorded at-HEAD counts (101 removed / 155 class-other / 2 P1 / 7 P2 / 63 P3 / 300 serde / coverage-matrix exit 0 / **nfr-test-3 exit 1** / hook-count 14); surfaced 6 structural spec divergences (D1-D6) and obtained Lunarpulse's 3 scope-correction decisions. |
| 2026-06-01 | AC3: triaged the 72 P-class — 14 `#[i9_exempt]` additions + 15 i9-exemptions.md entries; I9-walker gate-correctness fix (src-scope + cfg(test)-skip) for 19 test-double false-positives; P1 gate-correctness fix (smoke-skip + `// p1-allow:`); 7 P2 `ADAPTER_PORT_EXEMPTIONS` entries. `check-service-boundary` P-class → 0; `check-empty-kernel` → green (bonus). |
| 2026-06-01 | AC2+AC4: regenerated kernel-surface baseline (154→332) after the attrs landed → 256 NFR-Test-2 rows → 0; abi-diff (CI form) confirmed zero public-API delta (AC4 reconciliation moot — no privatization). |
| 2026-06-01 | AC5: FROZE 300 serde sites into `xtask/serde-error-allowlist.toml` + removed `continue-on-error` (hard-fail NEW); follow-up = Story 8.x serde remediation. |
| 2026-06-01 | AC6: re-confirmed coverage-matrix green; fixed D6 by authoring `crates/maos-spirit-hello/manifest.toml`; verified hook-count 14; full green-at-HEAD suite exits 0 + idempotent; touched-crate tests green. Status → review. |

### Review Findings

#### decision-needed (0 — resolved)

- [x] [Review][Defer] **Baseline JSON hash drift on ~20 un-exempted structs** — Root cause: stale baseline. The old baseline was committed at Story 2.2 (`9624dbe`). `manifest.rs` alone accumulated 1064 lines of change across Epics 3-7 but the baseline was never regenerated. The ~20 hash deltas are legitimate accumulated changes from prior epics, NOT unreported 7.1.7 code drift. syn/quote/proc-macro2 versions identical at old baseline and HEAD (syn 2.0.117). Resolution: stale-baseline correction is the purpose of this bridge story; no further action. [docs/ci-baselines/kernel-surface-v0.1-beta.json]

#### patch (7)

- [x] [Review][Patch] **`has_cfg_test()` uses blind substring match** — Replaced with `syn::Meta`-based cfg-token parsing: `cfg_meta_has_test_ident()` + `cfg_tokens_have_unnegated_test()`. Correctly matches `test` Ident; skips `cfg(not(test))` (production) and `cfg(feature="test")` (string literal). Applied. [xtask/src/check_empty_kernel.rs:269-272]
- [x] [Review][Patch] **`has_cfg_test()` only checks `ItemMod`** — Added `has_cfg_test()` guard to `visit_item_struct` before the exemption/violation check. `#[cfg(test)]` structs inside non-test modules now correctly skipped. Applied. [xtask/src/check_empty_kernel.rs:255]
- [x] [Review][Patch] **`quote::quote!().to_string()` used as cfg parser** — Replaced with token-level iteration of `proc_macro2::TokenStream` matching on `Ident("test")` instead of stringifying. `cfg_tokens_have_unnegated_test()` handles `not(…)` groups. Applied. [xtask/src/check_empty_kernel.rs:271]
- [x] [Review][Patch] **`has_i9_exempt` requires exact `reason` token** — Extended match to accept `rationale` and `justification` in addition to `reason`. Applied. [xtask/src/check_empty_kernel.rs:284-287]
- [x] [Review][Patch] **`is_p1_allowed` line-1-above check breaks on blank lines** — Replaced fixed line-N-2 check with backward-scan loop through blank/whitespace lines until a non-empty line is found, then checks for `// p1-allow:`. Resilient to blank-line insertion. Applied. [xtask/src/check_service_boundary.rs:591-601]
- [x] [Review][Patch] **`check_empty_kernel` fallback to `kernel_path` silently re-introduces test scanning** — Added `eprintln!` warning when falling back to crate root (no `src/`), alerting operators to potential false-positive I9 violations. Applied. [xtask/src/check_empty_kernel.rs:114-118]
- [x] [Review][Patch] **`smoke_*` skip is total AST subtree skip** — Added architectural comment: "This skip affects ONLY the P1OwnerVisitor. Future visitor extensions should be added in separate visitor structs or impl blocks." Intent correct (smoke functions are not composition root). Applied. [xtask/src/check_service_boundary.rs:642-643]

#### defer (9)

- [x] [Review][Defer] **`McpClientAdapter` exemption documents admitted architectural gap** — exemption itself says "deferred tidy-up" with no automated escalation. Deferred: pre-existing belt-and-suspenders pattern; the exemption is honestly documented and tracked. [xtask/src/check_service_boundary.rs:78-81]
- [x] [Review][Defer] **`// p1-allow:` magic-string comment convention** — no compiler enforcement; a typo silently disables the exemption. Deferred: spec-chosen mechanism for P1 false-positive resolution; the gate-correctness fix was Lunarpulse's decision. [xtask/src/check_service_boundary.rs:594,599]
- [x] [Review][Defer] **`ADAPTER_PORT_EXEMPTIONS` third field is free-form text** — mixes `"N/A"` sentinels with narratives in a `(&str, &str, &str)` tuple with no enum. Deferred: pre-existing pattern established in prior stories. [xtask/src/check_service_boundary.rs:40-83]
- [x] [Review][Defer] **`serde-error-allowlist.toml` line-number entries have zero staleness detection** — any line insertion above a frozen site shifts its number, silently dropping the allowlist entry and causing CI hard-fail. Deferred: known FREEZE posture tradeoff; follow-up Story 8.x full remediation will empty the allowlist. [xtask/serde-error-allowlist.toml]
- [x] [Review][Defer] **Frozen allowlist ratchet can only grow** — the gate hard-fails on NEW violations but has no content-digest for existing entries; stale entries produce false-positive CI failures. Deferred: Story 8.x serde-remediation follow-up tracked in allowlist header. [xtask/serde-error-allowlist.toml]
- [x] [Review][Defer] **`// p1-allow:` marker context-free** — the bare substring match on the constructor line or the line above can match an unrelated `// p1-allow:` comment, silently exempting a different construction. Deferred: accepted risk in Lunarpulse's gating-correctness decision. [xtask/src/check_service_boundary.rs:593-600]
- [x] [Review][Defer] **`infer_module_path` hardcodes crate name `"maos_kernel_core"`** — if the check is re-used for another kernel crate, exemption documentation cross-check would fail. Deferred: pre-existing; single-kernel-crate workspace layout makes this benign today. [xtask/src/check_empty_kernel.rs:222]
- [x] [Review][Defer] **`MockLifecycleResolver` is pub, not `#[cfg(test)]`-gated** — `pub mod test_double` compiles into production binaries; `#[i9_exempt]` masks it from I9. Deferred: protected by separate `check-mock-not-in-release` gate (discipline.yml:208-223); `#[i9_exempt]` just prevents double-flagging. [crates/maos-kernel-core/src/scheduler/verb_resolver.rs:131-142]
- [x] [Review][Defer] **Serde hard-fail flip unconditional on line-number-based allowlist** — removing `continue-on-error` makes all serde violations blocking; false positives from line drift can block CI. Deferred: accepted FREEZE posture tradeoff; mitigation = Story 8.x follow-up. [.github/workflows/discipline.yml:1008]
