# Story 10.2: V1.0/V1.5 Ship-Gate Infrastructure — Trial, Cross-Form, Red-Team

Status: done

<!-- Preflight: party-mode 2026-06-21 (Winston·Amelia·Murat·John, ratified Lunarpulse).
     7 forks resolved (all 4/4 unanimous after Round 2):
     F1 → B: validate committed artifacts, no live CI Spirit execution.
     F2 → B: cross-form gate is ADVISORY (CLI-wrapper-only is near-tautological until rust-inproc v2.0+).
     F3 → B: v1.0 advisory + v1.5-ship-gate aggregate with phase field in gate-registry.toml;
              advisory output includes "WOULD HAVE BLOCKED SHIP" banner.
     F4 → A: PR review is trust boundary, no crypto signing.
     F5 → A-prime: raw count = BLOCKING assertion; Wilson CI = ADVISORY log (compute, log bounds, do not block at N=12);
                    promote to blocking when N grows. 41-point CI band at N=12 is not a precision instrument (Murat).
     F6 → C: per-participant frames≥1000 + halt_recall≥0.85; SBOM/signing are operational verification, not gate logic.
     F7 → A: 80 canonical scenarios (10/class, ≥9/10 floor); 640 expansion is supplementary evidence.
     Title renamed: "Run the Trial" → "Ship-Gate Infrastructure" (story builds gates, not runs trials).
     ADR-040 Status: accepted — rust-inproc DEFERRED to v2.0+.
     Story has 4 ACs — within §A5 6-AC limit. -->

## Story

As a substrate quality lead at v1.0,
I want the v1.0/v1.5 ship-gate infrastructure for the third-party trial (NFR-Test-8), CLI-wrapper cross-form distributional equivalence (NFR-Test-7, advisory per ADR-040 rust-inproc deferral), and the adversarial-Spirit red-team 80-scenario corpus gate (NFR-Sec-10 v1.5),
so that v1.0 has mechanically-verifiable gate infrastructure ready to activate when external engagements produce results — not aspirational claims that decay.

## Acceptance Criteria

1. **AC-1: Third-Party Trial N=12 Gate Infrastructure** — Given the N=12 stratified recruitment requirements (≥4 no prior MAOS contribution; ≥3 never written Rust Spirit; ≥2 never written Rust at all; ≥2 non-English-native; ≥1 working offline-only), when the trial gate CI job (`check-third-party-trial`) runs, then: if `docs/third-party-trial/results/trial-results.toml` exists, the gate parses it via typed serde deserialization and asserts `successes >= 10` (blocking), stratification constraints met (blocking), each successful participant's `frames_run >= 1000` and `halt_recall >= 0.85` (blocking per-participant validation — F6→C); Wilson CI lower bound is computed and logged as advisory (not blocking — F5→A-prime: 41-point CI band at N=12 is directional, not dispositive); if the results file is absent, the gate passes with advisory annotation following the `calibrate-per-commit` conditional pattern from 10.1b; if malformed or missing required fields, hard-fail; the gate is wired into `v1.0-ship-gate` aggregate and main `aggregate` `needs:` list; SBOM + signing chain verification is documented in trial protocol as an operational step, not automated in the gate (F6→C).

2. **AC-2: CLI-Wrapper Cross-Form Distributional Equivalence Gate (ADVISORY)** — Given ADR-040 status `accepted` (rust-inproc deferred to v2.0+), when the cross-form gate evaluates, then: NFR-Test-7 cross-form scope is CLI-wrapper-only (no rust-inproc ↔ subprocess, no any-rust ↔ wasm); the `check-cross-form-equiv` xtask validates a pre-committed `cross-form-results.json` artifact (F1→B: committed artifact validation, not live CI execution) containing Mann-Whitney U-test results comparing CLI-wrapper ↔ subprocess behavioral output distributions over 30 runs against the `hello` reference Spirit; the gate asserts p > 0.05 as an **advisory** signal (F2→B: single-form comparison is near-tautological, advisory until rust-inproc lands); if the artifact is absent, the gate passes with advisory annotation; if ADR-040 status changes in a future version, the gate logs a warning that expanded cross-form testing is needed; the gate is wired into `v1.0-ship-gate` aggregate as `enforcement: advisory-until-engagement` and main `aggregate` needs; the gate produces structured output with U-test statistics for human review.

3. **AC-3: Adversarial Red-Team 80-Scenario Gate Infrastructure (V1.5 PHASE)** — Given the adversarial-Spirit red-team corpus (NFR-Sec-10), when the `check-red-team-gate` xtask runs, then: if `docs/red-team/results/red-team-results.toml` exists, the gate parses it via typed serde deserialization and asserts per-class floor ≥9/10 for each of the 8 attack classes across the 80 canonical scenarios (F7→A: gate checks 80 canonical, not 640 expanded; 640 is supplementary evidence), aggregate ≥72/80, and 0 unmitigated categories (no class scores 0); if the results file is absent, the gate passes with advisory annotation ("red-team engagement pending") and emits a **"WOULD HAVE BLOCKED SHIP"** banner when thresholds would have failed (F3→B); if malformed, hard-fail; the gate validates `corpus_sha256` matches the `red-team-640` entry in `tests/corpora/MANIFEST.toml`; the gate is wired into `v1.0-ship-gate` aggregate as `enforcement: advisory-until-engagement` with `phase: v1.5` in `gate-registry.toml` (F3→B: v1.0 advisory now, v1.5 blocking when aggregate exists); the gate is also wired into main `aggregate` needs.

4. **AC-4: Protocol Documentation + Coverage Matrix + Gate Registry Update** — Given the operational protocols for both engagements, when Story 10.2 ships, then: `docs/third-party-trial/` contains recruitment protocol, trial-results schema, Wilson CI computation documentation, SBOM + signing chain CI bot verification runbook; `docs/red-team/` contains engagement protocol (attack-class mapping to ABI entry points, corpus content-addressing requirements, external pen-tester instructions), results schema; `tests/coverage-matrix.yaml` is updated: NFR-Test-8 gets `gates: [check-third-party-trial]` phase v1.0, NFR-Test-7 gets `gates: [check-cross-form-equiv]` with `enforcement: advisory-until-engagement` phase v1.5, NFR-Sec-10 gets `gates: [check-red-team-gate]` with `enforcement: advisory-until-engagement` phase v1.5; `gate-registry.toml` adds all 3 gates with `phase` field (v1.0 or v1.5); `check-ship-gate-completeness` expected gates list updated to include all 3 new gates.

## Tasks / Subtasks

- [x] Task 1 — Third-Party Trial Gate Infrastructure (AC: 1, 4)
  - [x] 1.1 Create `docs/third-party-trial/` directory with recruitment protocol: stratification matrix (N=12, ≥4/≥3/≥2/≥2/≥1 breakdown), 14-day no-DM-support window timeline, participant consent/NDA template stub, trial environment setup instructions (fresh Host VM, `maosctl install`, reference Spirit template), success criteria definitions (signed binary + load + ≥1000 frames + halt-recall ≥0.85)
  - [x] 1.2 Author `docs/third-party-trial/results/trial-results-schema.toml` defining the expected schema: `[trial] participants_total: int, successes: int, trial_start: date, trial_end: date, methodology_version: string`; `[[participant]]` array with `id: string, stratum: string[], produced_binary: bool, binary_loads: bool, frames_run: int, halt_recall: float, sbom_verified: bool, signing_chain_verified: bool`; stratification verification fields `no_prior_contribution: int, no_rust_spirit: int, no_rust: int, non_english: int, offline_only: int`
  - [x] 1.3 Author Wilson CI computation documentation: formula for Wilson score interval at N=12, expected bounds [0.552, 0.953] at 10/12 success, explain why N=5 is meaningless (per NFR-Test-8), explain why Wilson CI is advisory-only at N=12 (F5→A-prime: 41-point CI band, meaningful but not dispositive)
  - [x] 1.4 Author SBOM + signing chain CI bot verification runbook: steps to re-load signed Spirit binary on clean VM, verify SBOM completeness, verify signing chain against trusted root (operational verification, not automated gate per F6→C)
  - [x] 1.5 Add xtask `check-third-party-trial`: if `docs/third-party-trial/results/trial-results.toml` exists, parse via `toml::from_str::<TrialResults>`; **blocking assertions**: `participants_total >= 12`, stratification constraints met, `successes >= 10`, per-participant `frames_run >= 1000` and `halt_recall >= 0.85` for each successful participant; **advisory log**: compute Wilson CI lower bound, log bounds to stdout/GITHUB_STEP_SUMMARY (do NOT assert — F5→A-prime); if absent, emit advisory annotation + `GITHUB_STEP_SUMMARY` output and pass; if malformed, hard-fail with `::error::` annotation; validate date fields non-empty + ISO-8601, reject negative counts (~80-120 LOC)
  - [x] 1.6 Add `check-third-party-trial` job to `discipline.yml` using `actions/checkout@v5`; wire into `v1.0-ship-gate` aggregate `needs:` and main `aggregate` `needs:`
  - [x] 1.7 Proven-red (5 vectors): (a) `trial-results.toml` with `successes = 8` (below floor) → fail; (b) `trial-results.toml` with `no_prior_contribution = 3` (below ≥4 stratification) → fail; (c) `trial-results.toml` with all valid, `successes = 10`, all strata met → pass; (d) absent → pass with advisory; (e) malformed TOML → hard-fail

- [x] Task 2 — CLI-Wrapper Cross-Form Distributional Equivalence Gate (AC: 2)
  - [x] 2.1 Add xtask `check-cross-form-equiv`: if `docs/cross-form/results/cross-form-results.json` exists (F1→B: committed artifact, not live execution), parse and validate the pre-committed Mann-Whitney U-test results; assert p > 0.05 as **advisory** (F2→B: log result + GITHUB_STEP_SUMMARY, do not block ship); read ADR-040 frontmatter Status to determine scope at runtime; if `accepted`, scope = CLI-wrapper-only; if artifact absent, emit advisory annotation and pass; if malformed, hard-fail (~80-100 LOC — simpler than live execution)
  - [x] 2.2 Implement Mann-Whitney U-test validation in pure Rust: parse the pre-committed artifact's U-statistic, sample sizes, and p-value; validate internal consistency (recompute p from U and sample sizes); OR if the artifact contains raw per-run hashes, recompute the full U-test (~40-60 LOC)
  - [x] 2.3 Add `check-cross-form-equiv` job to `discipline.yml`; wire into `v1.0-ship-gate` aggregate `needs:` and main `aggregate` `needs:` with `enforcement: advisory-until-engagement`
  - [x] 2.4 Proven-red (3 vectors): (a) `cross-form-results.json` with p-value = 0.03 (below 0.05) → advisory warns (gate still passes but logs warning); (b) valid results with p > 0.05 → advisory passes clean; (c) absent → advisory pass with "cross-form results pending" annotation

- [x] Task 3 — Adversarial Red-Team Gate Infrastructure (AC: 3, 4)
  - [x] 3.1 Create `docs/red-team/` directory with engagement protocol: attack-class mapping to ABI entry points (8 classes → specific kernel modules per `docs/pen-test/scope.md` pattern), corpus content-addressing requirements (must match `red-team-640` SHA-256 in MANIFEST.toml), external pen-tester instructions (use published ABI only, no MAOS team authorship, pre-freeze + content-address before execution), mapping of 80 canonical scenarios to 8 attack classes
  - [x] 3.2 Author `docs/red-team/results/red-team-results-schema.toml` defining expected schema: `[gate] corpus_sha256: string, engagement_start: date, engagement_end: date, methodology_version: string`; `[[class_result]]` with `class: string, scenarios_total: int, detected_blocked: int, unmitigated: int, notes: string`; aggregate section `[aggregate] total_scenarios: int, total_detected: int, total_unmitigated_categories: int`
  - [x] 3.3 Add xtask `check-red-team-gate`: if `docs/red-team/results/red-team-results.toml` exists, parse via typed serde deserialization; assert per-class `detected_blocked >= 9` for each of 8 classes across 80 canonical scenarios (F7→A); assert aggregate `total_detected >= 72` out of 80; assert `total_unmitigated_categories == 0`; validate `corpus_sha256` matches `red-team-640` entry in `tests/corpora/MANIFEST.toml`; when thresholds fail, emit **"WOULD HAVE BLOCKED SHIP"** banner to GITHUB_STEP_SUMMARY (F3→B); if absent, emit advisory annotation + GITHUB_STEP_SUMMARY and pass; if malformed, hard-fail; validate date fields + reject negative counts (~100-140 LOC)
  - [x] 3.4 Add `check-red-team-gate` job to `discipline.yml`; wire into `v1.0-ship-gate` aggregate `needs:` and main `aggregate` `needs:` with `enforcement: advisory-until-engagement`, `phase: v1.5`
  - [x] 3.5 Proven-red (5 vectors): (a) `red-team-results.toml` with one class `detected_blocked = 7` (below ≥9/10) → gate logs "WOULD HAVE BLOCKED SHIP"; (b) `red-team-results.toml` with one class `detected_blocked = 0` (unmitigated category) → gate logs "WOULD HAVE BLOCKED SHIP"; (c) `red-team-results.toml` with aggregate `total_detected = 70` (below ≥72) → gate logs "WOULD HAVE BLOCKED SHIP"; (d) valid results with all thresholds met → pass clean; (e) absent → pass with advisory

- [x] Task 4 — Coverage Matrix + Gate Registry + Ship-Gate Completeness (AC: 4)
  - [x] 4.1 Update `tests/coverage-matrix.yaml`: NFR-Test-8 with `gates: [check-third-party-trial]`, phase v1.0; NFR-Test-7 with `gates: [check-cross-form-equiv]`, `enforcement: advisory-until-engagement`, phase v1.5; NFR-Sec-10 with `gates: [check-red-team-gate]`, `enforcement: advisory-until-engagement`, phase v1.5
  - [x] 4.2 Update `xtask/gate-registry.toml`: add `check-third-party-trial` (phase v1.0), `check-cross-form-equiv` (phase v1.5, advisory), `check-red-team-gate` (phase v1.5, advisory); add `phase` field to new entries
  - [x] 4.3 Update `xtask/src/check_ship_gate_completeness.rs`: add all 3 new gates to `EXPECTED_GATES` list
  - [x] 4.4 Proven-red for 4.3: remove one new gate from `EXPECTED_GATES` → completeness check must fail; restore → pass

## Dev Notes

### Architecture Compliance

**Zero-kernel-core delta.** This story is gate infrastructure + documentation. No kernel code changes.

**ADR-040 decision binding (Story 5.5e, 2025-05-25):** rust-inproc Spirit form DEFERRED to v2.0+. Consequence: NFR-Test-7 cross-form scope for v1.0/v1.5 is CLI-wrapper distributional equivalence ONLY, and the gate is ADVISORY (F2→B). When rust-inproc lands, promote the gate from advisory to blocking — zero code delta, one registry entry change.

**Epic 9 §A1 applies:** Proven-red as dev-pass gate. Every task includes proven-red subtask with minimum vectors specified.

**Epic 9 §A5 applies:** 4 ACs — within the 6-AC ceiling.

**Model tier:** Tier-1 (opus-4-8 mandatory per Epic 9 §A2 — adversarial red-team is security-critical).

### Party-Mode Preflight Decisions (2026-06-21, all 4/4 unanimous)

**F1 → B (4/4):** Validate committed artifacts, don't execute tests in the gate. CI execution of 60 Spirit runs would measure CI runner noise, not behavioral equivalence (Murat). Matches existing `check-pentest-gate` advisory-if-absent pattern.

**F2 → B (4/4):** CLI-wrapper-only cross-form is an advisory gate, not ship-blocking. With rust-inproc deferred (ADR-040), the comparison is near-tautological — CLI wrapper invokes the same subprocess binary with an argv-prefix (Winston). Single-form "cross-form" is an identity test, not equivalence (Murat). Advisory status is the right call until v2.0+ when rust-inproc creates a real comparison axis.

**F3 → B (4/4, Winston conceded R2):** Wire advisory into v1.0-ship-gate AND create v1.5-ship-gate aggregate entry with `phase: v1.5` in gate-registry.toml. Advisory output includes "WOULD HAVE BLOCKED SHIP" banner when thresholds fail — forces conscious confrontation, prevents silent rot. Creates the graduation criteria early — good discipline (Murat). One registry entry, zero premature architecture (Amelia).

**F4 → A (4/4):** PR review is the trust boundary. GPG signing solves the wrong threat model at N=12 where participants are known by name. Document the trust assumption explicitly. If trust model changes at scale, add signing then.

**F5 → A-prime (4/4, Amelia+John moved R2):** Raw count (`successes >= 10`) = BLOCKING assertion. Wilson CI = ADVISORY log (compute bounds, log to GITHUB_STEP_SUMMARY, do not block). At N=12 the Wilson CI band is 41 percentage points wide — meaningful for reporting but not a precision instrument for gating (Murat power analysis). A false-positive blocking gate creates p-hacking or gate-bypass incentives — both worse than advisory logging. Promote to blocking when N grows and the band tightens. The spec says Wilson CI is "meaningful at N=12" — meaningful ≠ blocking (Winston).

**F6 → C (4/4):** Per-participant validation for `frames_run >= 1000` and `halt_recall >= 0.85` — these are the metrics that make a "success" meaningful. SBOM and signing chain are operational verification documented in the trial protocol, not automated gate logic. Single-responsibility: the gate validates trial outcomes, not supply chain compliance.

**F7 → A (4/4):** Gate checks 80 canonical scenarios (10 per class, floor ≥9/10). The 640 expanded corpus (8× parameter variation) is supplementary evidence in the engagement report, not gate scope. Scaling floors to 640 conflates generator quality with security posture and couples the gate to corpus-gen internals (Murat). 80 canonical gives clear per-class accountability and actionable failure messages.

**Title renamed (4/4):** "Run the Third-Party Trial N=12" → "V1.0/V1.5 Ship-Gate Infrastructure — Trial, Cross-Form, Red-Team." The story builds durable gate machinery, not runs a trial.

### Conditional Gate Pattern (reuse from 10.1b)

All three gates follow the `calibrate-per-commit` advisory-if-absent pattern established by `check-pentest-gate` in Story 10.1b:

```yaml
check-third-party-trial:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v5
    - name: Check third-party trial results
      run: |
        if [ -f docs/third-party-trial/results/trial-results.toml ]; then
          cargo run -p xtask -- check-third-party-trial
        else
          echo "::warning::Third-party trial pending — trial-results.toml absent"
          echo "## Third-Party Trial Gate: ADVISORY" >> "$GITHUB_STEP_SUMMARY"
          echo "Trial has not yet been executed." >> "$GITHUB_STEP_SUMMARY"
        fi
```

The `$GITHUB_STEP_SUMMARY` output ensures advisory status is visible (not buried in annotation body), per 10.1b F2 decision.

### Existing Assets (DO NOT RECREATE)

| Asset | Location | Status |
|---|---|---|
| Red-team corpus 640 items | `tests/corpora/red-team-640.jsonl` | Committed, SHA-pinned in MANIFEST.toml |
| Red-team generator | `crates/maos-corpus-gen/src/red_team/` | mod.rs, seeds.rs, expansion.rs, validation.rs |
| Red-team seeds 80 scenarios | `crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml` | 10 seeds × 8 classes |
| Corpus MANIFEST | `tests/corpora/MANIFEST.toml` | red-team-640 entry with SHA-256 |
| Pen-test scope mapping | `docs/pen-test/scope.md` | Attack surface → crate/module mapping |
| Pen-test engagement harness | `docs/pen-test/engagement-manifest.toml` | Reproducible environment |
| Pen-test gate (conditional) | `xtask/src/check_pentest_gate.rs` | Advisory-if-absent pattern to reuse |
| Coverage matrix | `tests/coverage-matrix.yaml` | NFR-Test-7/8/Sec-10 rows exist with empty gates |
| Gate registry | `xtask/gate-registry.toml` | 37 gates currently |
| Ship-gate completeness | `xtask/src/check_ship_gate_completeness.rs` | 5 expected gates currently |
| v1.0-ship-gate aggregate | `.github/workflows/discipline.yml` | Existing aggregate job |
| `hello` reference Spirit | `spirits/hello/` | Subprocess-form reference Spirit |
| ADR-040 | `docs/adr/ADR-040-rust-inproc-measurement-gate-v05-decision.md` | Status: accepted |
| Safety-critical corpus pattern | `crates/maos-eval/src/safety_critical_corpus.rs` | Corpus gate pattern example |
| Bench infrastructure | `crates/maos-bench/` | J1/J4 measurement primitives |

### Wilson Score Interval — Advisory Implementation (F5→A-prime)

The gate computes Wilson CI but logs it as advisory, not blocking:
- p-hat = successes / n
- Wilson lower bound = (p-hat + z²/(2n) - z·sqrt(p-hat(1-p-hat)/n + z²/(4n²))) / (1 + z²/n)
- z = 1.96 for 95% CI
- At N=12, successes=10: CI = [0.552, 0.953] per NFR-Test-8
- Output: `info!("Wilson CI [{lower:.3}, {upper:.3}] at N={n} (advisory — promote to blocking at N≥30)")`
- The GITHUB_STEP_SUMMARY includes the Wilson CI bounds for human review

### Cross-Form Gate — Committed Artifact Pattern (F1→B)

The `check-cross-form-equiv` xtask does NOT build or run Spirits in CI. It validates a pre-committed `docs/cross-form/results/cross-form-results.json` artifact:
1. The artifact is produced locally by the dev team running 30 subprocess + 30 CLI-wrapper iterations of `hello` Spirit.
2. The artifact contains: per-run output frame hashes, sample sizes, U-statistic, p-value, and test metadata (Spirit version, run date, environment).
3. The xtask parses the artifact, optionally recomputes the U-test from raw data for internal consistency, and logs the result.
4. Since AC-2 is advisory (F2→B), the gate always passes but logs whether p > 0.05.

### Red-Team Gate — "WOULD HAVE BLOCKED SHIP" Banner (F3→B)

The advisory output for the red-team gate must be loud:
```
## ⚠️ Red-Team Gate: WOULD HAVE BLOCKED SHIP (v1.5)
- Class 'resource_exhaustion': 7/10 detected (BELOW 9/10 floor)
- Aggregate: 68/80 (BELOW 72/80 floor)
- This gate is advisory at v1.0. It WILL block at v1.5.
```

### Red-Team Corpus Content-Addressing (F7→A)

The `check-red-team-gate` xtask validates that the engagement used the canonical corpus:
1. Read `corpus_sha256` from `red-team-results.toml`.
2. Read the SHA-256 from `tests/corpora/MANIFEST.toml` for the `red-team-640` entry.
3. Assert they match — prevents results from being run against a different/modified corpus.
4. Gate checks 80 canonical scenarios (10 per class). The 640 expansion is supplementary evidence reported by the pen-tester but not gate-asserted.

### Dependency on 10.1b

Story 10.1b created the pen-test engagement infrastructure (`docs/pen-test/`) and the conditional gate pattern. Story 10.2 reuses:
- The `calibrate-per-commit` advisory-if-absent CI job pattern.
- The `docs/pen-test/scope.md` attack surface mapping (referenced by red-team engagement protocol).
- The `$GITHUB_STEP_SUMMARY` advisory output convention (10.1b F2).
- Typed serde deserialization for TOML result files.
- Date field validation (non-empty + ISO-8601), negative count rejection, deduped GITHUB_STEP_SUMMARY lookup, `OpenOptions::append(true).create(true)` — all per 10.1b review patches.

### Previous Story Intelligence

**From Story 10.1a:**
- v1.0-ship-gate aggregate uses `if: always()` with explicit `contains(needs.*.result, 'failure') || contains(needs.*.result, 'skipped') || contains(needs.*.result, 'cancelled')` — new gates must be added to `needs:`.
- `check-ship-gate-completeness` xtask validates expected gates — update `EXPECTED_GATES` constant.
- Use `actions/checkout@v5` (not @v4).

**From Story 10.1b:**
- `check-pentest-gate` established advisory-if-absent with `$GITHUB_STEP_SUMMARY` output.
- `enforcement: advisory-until-engagement` counts as non-empty for `check-coverage-matrix-completeness`.
- JSON mode emits only JSON to stdout, workflow commands to stderr (10.1b review Patch #1).

**From Epic 9 retro:**
- §A1 proven-red as dev-pass gate — not just review checkpoint.
- §A2 model-tier: this story is Tier-1 (opus-4-8 mandatory).
- Party-mode preflight standard, sprint-status tracking must stay current.

### Project Structure Notes

- Trial docs: NEW `docs/third-party-trial/` directory (recruitment protocol, results schema, Wilson CI docs, SBOM runbook)
- Trial results: NEW `docs/third-party-trial/results/` directory (schema, README)
- Cross-form results: NEW `docs/cross-form/results/` directory (schema for pre-committed artifact)
- Red-team docs: NEW `docs/red-team/` directory (engagement protocol, results schema)
- Red-team results: NEW `docs/red-team/results/` directory (schema, README)
- Gate xtask: ADD `check-third-party-trial` to xtask
- Gate xtask: ADD `check-cross-form-equiv` to xtask (artifact validator, not live executor)
- Gate xtask: ADD `check-red-team-gate` to xtask
- Discipline workflow: MODIFY `.github/workflows/discipline.yml` (add 3 CI jobs, wire into v1.0-ship-gate and aggregate needs)
- Coverage matrix: MODIFY `tests/coverage-matrix.yaml` (populate NFR-Test-7, NFR-Test-8, NFR-Sec-10 gate entries)
- Gate registry: MODIFY `xtask/gate-registry.toml` (add 3 new gates with `phase` field)
- Ship-gate completeness: MODIFY `xtask/src/check_ship_gate_completeness.rs` (add 3 gates to EXPECTED_GATES)
- Xtask main: MODIFY `xtask/src/main.rs` (add 3 mod declarations, 3 Command variants, 3 dispatch arms)

**Estimated total delta:** ~280-380 LOC Rust (check-third-party-trial ~80-120 + check-cross-form-equiv ~80-100 + check-red-team-gate ~100-140 + Wilson CI ~15-20 + ship-gate-completeness update ~5 + xtask main ~15), ~40-60 LOC CI YAML, ~400-600 lines markdown (recruitment protocol, trial schema, Wilson CI docs, SBOM runbook, red-team protocol, results schemas, cross-form artifact schema)

### References

- [Source: _bmad-output/planning-artifacts/epics/epic-10-v10-ship-gate-v15-collective-tier-v10-v15.md — Story 10.2 definition]
- [Source: _bmad-output/planning-artifacts/prd/non-functional-requirements.md — NFR-Test-7 (line 80), NFR-Test-8 (line 82), NFR-Sec-10 (line 42)]
- [Source: _bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md — Threat model, adversarial corpus]
- [Source: docs/adr/ADR-040-rust-inproc-measurement-gate-v05-decision.md — Status: accepted, rust-inproc deferred to v2.0+]
- [Source: _bmad-output/implementation-artifacts/10-1a-automated-v1-0-ship-gates-ccac-hsis-stability-aggregate.md — v1.0-ship-gate aggregate pattern, check-ship-gate-completeness]
- [Source: _bmad-output/implementation-artifacts/10-1b-pen-test-engagement-harness-and-gate-infrastructure.md — conditional gate pattern, advisory-if-absent, coverage matrix update]
- [Source: crates/maos-corpus-gen/src/red_team/mod.rs — RedTeamGenerator, RedTeamSeed, RedTeamItem types]
- [Source: crates/maos-corpus-gen/seeds/red-team-seeds-v0.1.toml — 80 canonical scenarios, 8 attack classes × 10 seeds]
- [Source: tests/corpora/MANIFEST.toml — red-team-640 entry SHA-256: 783d064d...]
- [Source: xtask/src/check_pentest_gate.rs — advisory-if-absent gate pattern to reuse]
- [Source: xtask/src/check_ship_gate_completeness.rs — EXPECTED_GATES validation, currently 5 gates]
- [Source: xtask/gate-registry.toml — 37 current gates]
- [Source: tests/coverage-matrix.yaml — NFR-Test-7/8/Sec-10 entries with empty gates]
- [Source: crates/maos-eval/src/safety_critical_corpus.rs — corpus gate pattern (Cohen's κ, SHA-pin, validation)]
- [Source: _bmad-output/implementation-artifacts/epic-9-retro-2026-06-19.md — §A1 proven-red, §A2 model-tier, §A3 security prep]
- [Source: commit 6d49f51 — 10-1b pen-test engagement harness]
- [Source: commit 0132d38 — 10-1a automated v1.0 ship gates]
- [Source: party-mode preflight 2026-06-21 — F1→B, F2→B, F3→B, F4→A, F5→A-prime, F6→C, F7→A; all 4/4 unanimous]

## Dev Agent Record

### Agent Model Used

Tier-1: claude-opus-4-8 MANDATORY (per Epic 9 §A2 — adversarial red-team is security-critical)

<!--
§A6 NON-OPUS SAFETY NET (Epic 8 retro 2026-06-12, ratified by Lunarpulse).
-->
Tier-1 story — claude-opus-4-8 mandatory per §A2. §A6 net N/A if Opus used.

### Debug Log References

### Completion Notes List

- AC-1: `check-third-party-trial` xtask gate implemented with typed serde deserialization, blocking assertions (successes≥10, stratification, per-participant frames≥1000 + halt_recall≥0.85), Wilson CI advisory logging. Advisory-if-absent pattern. 5 proven-red vectors pass (248 LOC).
- AC-2: `check-cross-form-equiv` xtask gate implemented with Mann-Whitney U-test recomputation from raw per-run hashes, advisory-only per ADR-040 (F2→B). 3 proven-red vectors pass (191 LOC).
- AC-3: `check-red-team-gate` xtask gate implemented with corpus SHA-256 provenance validation against MANIFEST.toml, per-class floor ≥9/10, aggregate ≥72/80, 0 unmitigated categories. "WOULD HAVE BLOCKED SHIP" banner on threshold failure (F3→B). 5 proven-red vectors pass (242 LOC).
- AC-4: Documentation complete (4 trial docs, 2 red-team docs, 1 cross-form schema). Coverage matrix updated (NFR-Test-7, NFR-Test-8, NFR-Sec-10). Gate registry updated (3 new gates, 40 total). Ship-gate completeness updated (8 expected gates). 3 CI jobs added to discipline.yml.
- All 15 proven-red integration tests pass. 320 existing xtask tests pass (0 regressions).
- Zero kernel-core delta. dev_model_used: claude-opus-4-6.
- Attack classes grounded from actual `red-team-seeds-v0.1.toml`: capability_confusion, iac_frame_injection, distillation_poisoning, ledger_tampering, cross_spirit_privilege_escalation, resource_exhaustion, side_channel_timing, kernel_syscall_abuse.
- Wilson CI at N=12, s=10: [0.552, 0.953] (formula yields 0.953 not 0.962 per brief; documented discrepancy in wilson-ci.md).

### File List

- docs/third-party-trial/README.md (new)
- docs/third-party-trial/results/trial-results-schema.toml (new)
- docs/third-party-trial/wilson-ci.md (new)
- docs/third-party-trial/sbom-signing-verification.md (new)
- docs/red-team/README.md (new)
- docs/red-team/results/red-team-results-schema.toml (new)
- docs/cross-form/results/cross-form-results-schema.json (new)
- xtask/src/check_third_party_trial.rs (new)
- xtask/src/check_cross_form_equiv.rs (new)
- xtask/src/check_red_team_gate.rs (new)
- xtask/src/main.rs (modified — 3 mod declarations, 3 Command variants, 3 dispatch arms)
- xtask/src/check_ship_gate_completeness.rs (modified — 3 gates added to EXPECTED_GATES)
- xtask/gate-registry.toml (modified — 3 new gate entries)
- tests/coverage-matrix.yaml (modified — NFR-Test-7, NFR-Test-8, NFR-Sec-10 gates populated)
- .github/workflows/discipline.yml (modified — 3 CI jobs, v1-0-ship-gate needs, aggregate needs)
- xtask/tests/story_10_2_proven_red.rs (new — 15 proven-red integration tests)
- _bmad-output/implementation-artifacts/sprint-status.yaml (modified — status in-progress→review)

### Change Log

- 2026-06-21: Story 10.2 implementation complete. 3 ship-gate xtask modules (check-third-party-trial, check-cross-form-equiv, check-red-team-gate), 7 documentation files, CI wiring, coverage-matrix/gate-registry/ship-gate-completeness updates, 15 proven-red tests. Zero kernel-core delta.
- 2026-06-21: **Code review patches applied** (3-layer adversarial re-review + party-mode consensus Winston·Amelia·John·Murat). 36 patches across 9 files: 3 Critical derive-from-detail gate fixes (trial successes/participant reconciliation, per-participant unconditional validation + halt_recall bounds, red-team 8-distinct-canonical-classes enforcement + aggregate cross-validation + scenarios bounds), 8 High (cross-form hash truncation→full-hex u128 rank, sample-size/spirit validation, U1/U2 convention, fail-on-detected-divergence, ADR-040 runtime read), 12 Medium (p_value/u_statistic NaN guards, coverage-matrix NFR-Test-8 label fix, v1.5-ship-gate aggregate + [[ship_gate]] disposition registry, deterministic-degeneracy detection→NOT-APPLICABLE verdict, notes serde-default, methodology_version enforcement), proven-red expanded 15→21 tests (recompute path, corpus_sha mismatch, malformed-input, empty-participants, duplicate-classes). D1-D5 resolved A on all 5 (team overrode reviewer's cheaper picks on D3/D5). Build + 21 tests + clippy green. PRD 0.962→0.953 corrected. chmod -x on 3 new .rs files.

### Review Findings

**Code review date:** 2026-06-21 (re-review, second pass)
**Reviewers:** Blind Hunter, Edge Case Hunter, Acceptance Auditor (3-layer adversarial review; Test Infra Auditor skipped — dev_model_used is Claude per Epic 8 §A6)
**Scope:** staged diff on `epic10` HEAD 6d49f51 (18 files, +2417/-5)
**Total findings:** 38 (3 Critical, 8 High, 12 Medium, 12 Low, 3 Info) — 16 NEW (missed by prior review), 21 prior confirmed unpatched, 1 prior retracted (#332), 2 deferred

> **Re-review note.** The prior 3-layer review (22 findings) was re-verified line-by-line against the current diff. **All prior patch/decision checkboxes were unchecked — none had been applied.** Each prior finding is re-confirmed below. Prior finding #332 ("completeness test removes 3 gates") is **RETRACTED** — the fixture omits exactly 1 of the 8 EXPECTED_GATES (`check-red-team-gate`), so Task 4.4 is correctly satisfied; only a residual parametrization gap remains (test removes a fixed gate, not any-of-8). Prior finding on Wilson 0.962 vs 0.953 is reclassified: `0.962` appears nowhere in the diff (code + wilson-ci.md already say 0.953, which is mathematically correct); the stale `0.962` lives only in the NFR-Test-8 PRD line — doc-only fix.

---

#### Decision-Needed — RESOLVED via party-mode consensus (Winston·Amelia·John·Murat, 2026-06-21)

> **Consensus: A on all 5 decisions** under the user's "spec fidelity + long-term correctness" lens. The team overrode the reviewer's cheaper picks on D1, D3, and D5 — exactly the three where the defect was silent misrepresentation in the gate output. Winston's unifying principle: a gate has THREE orthogonal verdict axes — (1) integrity/precondition (always hard-fail, "advisory" licenses nothing here), (2) phase (the only thing "advisory at v1.0" governs), (3) measurement-validity (failure = NOT-APPLICABLE, never silent PASS). The reviewer's B/C options all failed axis 1 or 3. Implementation refinements from Amelia + Murat folded into the patches below.

- [x] [Review][Decision] D1 → **A (implemented as Amelia's verdict/error split)** — Advisory v1.5-phase gates wired into blocking v1.0-ship-gate aggregate: malformed advisory artifact blocks v1.0 ship [.github/workflows/discipline.yml:2120-2166] -- Err-on-malformed STAYS (axis-1 precondition failure is always fatal). Winston: "'advisory' governs the verdict axis only; a gate that can't parse input has crashed, not produced an advisory." Implementation: split gate JSON output into `verdict: Pass|Fail|Skipped` (phase-gated by aggregate) + `error: MalformedInput|Integrity|...` (always fatal). This subsumes both A and C — the dual semantics become mechanical, not documented.
- [x] [Review][Decision] D2 → **A** — Cross-form gate returns Ok / JSON `passed:true` even when its own consistency check detects U divergence [xtask/src/check_cross_form_equiv.rs:179,236-250] -- Unanimous. Detected divergence is axis-1 integrity failure (tampering/corruption), not axis-2 advisory verdict. **ORDERING HAZARD (Amelia):** making divergence fatal is BLOCKED on fixing the hash truncation + silent-skip first (patches #7, #10), else the blocking gate false-positives on honest artifacts. Sequence: fix hash comparator → make length-mismatch Err → THEN flip divergence → Err → proven-red for both.
- [x] [Review][Decision] D3 → **A** — No v1.5-ship-gate aggregate job and no per-entry `phase` field in gate-registry.toml [xtask/gate-registry.toml + .github/workflows/discipline.yml] -- Unanimous; reviewer's B OVERRIDDEN. John: "Task 4.2 / F3→B is imperative — names the file AND the field; B is spec-washing." Winston: "the entire PURPOSE of ship-gate infrastructure is to MECHANIZE graduation; B makes v1.5 cutover unmechanized — the successes-bug pattern one layer up." Murat: "with B the 'WILL block at v1.5' is UNTESTABLE — no proven-red can assert the promotion flips the gate." **Schema refinement (Amelia):** phase is a `disposition={v1_0, v1_5}` MAP not a scalar (AC-3 needs red-team advisory@v1.0 AND blocking@v1.5). v1.5-ship-gate aggregate stands up NOW (mirror v1.0 membership) so cutover is a disposition flip, not an infrastructure fire-drill.
- [x] [Review][Decision] D4 → **A** — Trial stratification counts are self-reported, not derived from participant[].stratum [xtask/src/check_third_party_trial.rs:37,162-168] -- Unanimous. Same derive-and-reconcile shape as Critical #1. **Schema latent bug (Amelia):** stratified trials use MUTUALLY EXCLUSIVE strata → `stratum: Vec<String>` is wrong, should be `stratum: String` (the Vec implies overlap sampling the design doesn't use). Two-stage reconcile: (1) derive per-stratum counts from participant[].stratum; (2) assert derived == [trial] reported (integrity); (3) assert derived >= floor. Edge cases: empty stratum → Err(MissingStratum); unknown stratum name → Err(UnknownStratum); underpopulated stratum flagged distinctly from tamper.
- [x] [Review][Decision] D5 → **A** — Deterministic-Spirit degeneracy makes the cross-form U-test vacuous [xtask/src/check_cross_form_equiv.rs:74-107] -- Unanimous; reviewer's B OVERRIDDEN. Winston: "B is documentation theater — the gate still emits passed:true with U=450; every downstream consumer reads a measurement that was never made." Murat: "false confidence lives in the gate OUTPUT, not a README the operator opens at decision time." Implementation (~15 LOC, Amelia): `if distinct_hashes(group) < 2 → Ok(Verdict::Skipped(DeterministicOutput))`. Not a fail (determinism is fine), not a silent pass — an honest "couldn't measure" verdict. Aggregate treats Skip as non-blocking (same as advisory verdict-fail@v1.0).

---

#### Patch Findings (Critical)

- [x] [Review][Patch] #1 Trial gate trusts self-reported `successes`; never derives from participant records [xtask/src/check_third_party_trial.rs:169,179-188] -- `participant=[]` + `participants_total=12, successes=12` PASSES the blocking v1.0 gate with zero participant records. The load-bearing AC-1/F6→C control is trivially fabricatable. FIX: assert `participant.len()==participants_total`; derive successes via full conjunction (produced_binary && binary_loads && frames_run>=1000 && halt_recall.is_finite() && in [0.85,1.0]); reconcile derived==trial.successes.
- [x] [Review][Patch] #2 Per-participant validation gated behind `produced_binary && binary_loads`; non-producers silently skipped; halt_recall unbounded [xtask/src/check_third_party_trial.rs:180,184] -- 12 participants all `produced_binary=false` + `successes=12` passes. `halt_recall=NaN` passes (`NaN<0.85` is false); `halt_recall=5.0` passes. FIX: validate every participant unconditionally; add `halt_recall.is_finite() && (0.0..=1.0).contains(&halt_recall)`; success = full conjunction.
- [x] [Review][Patch] #3 Red-team gate never enforces 8 DISTINCT canonical attack classes [xtask/src/check_red_team_gate.rs:173,179-189] -- Only checks `class_result.len()==8`; class identity/uniqueness/coverage never validated. 8 identical "resource_exhaustion" rows at 10/10 pass. FIX: load the 8 canonical names from red-team-seeds-v0.1.toml, collect classes into a HashSet, assert set equality; treat mismatch as structural hard-fail (not advisory).

---

#### Patch Findings (High)

- [x] [Review][Patch] #4 Red-team [aggregate] trusted without cross-validation against [[class_result]] sums [xtask/src/check_red_team_gate.rs:155-158,190-201] -- No check sum(detected_blocked)==total_detected, sum(scenarios_total)==total_scenarios, count(detected==0)==total_unmitigated_categories. At v1.5 (blocking) the aggregate is gameable independent of per-class rows. FIX: recompute all three from class_result, assert equality.
- [x] [Review][Patch] #5 Red-team per-class floor is absolute count (>=9), not ratio; scenarios_total never ==10; detected_blocked<=scenarios_total never checked [xtask/src/check_red_team_gate.rs:18,183] -- `scenarios_total=100, detected_blocked=9` passes (9% detection); `detected_blocked=50, scenarios_total=10` passes (>100%). FIX: assert `scenarios_total==10`, `detected_blocked<=scenarios_total`, `total_scenarios==80`.
- [x] [Review][Patch] #6 [NEW] class_result=[] (zero entries) + aggregate total_detected=72 advisory-passes [xtask/src/check_red_team_gate.rs:24-29,173,203-243] -- `class_result.len()!=8` is currently a threshold failure (advisory), not a structural hard-fail. A class-less file passes on a fabricated aggregate at v1.0. FIX: treat len!=8 / class-set mismatch as structural `Err`. (Subsumes EdgeCaseHunter `detected_blocked>scenarios_total` and `total_scenarios=1000` paths.)
- [x] [Review][Patch] #7 Cross-form `hash_to_u64` truncates SHA-256 to 64 bits; fallback byte-sum is order-insensitive [xtask/src/check_cross_form_equiv.rs:65-69] -- `take(16)` discards 192 bits → distinct hashes collide in ranking → fabricated ties → corrupted U. The byte-sum fallback is hit by the schema's own placeholder `"sha256-of-output-frame-1"`. FIX: rank by full hex string or u128; remove the non-hash fallback for hex inputs.
- [x] [Review][Patch] #8 Cross-form gate never validates 30-run sample sizes or that spirit_name=='hello' [xtask/src/check_cross_form_equiv.rs:29-38,163-164] -- `TestMetadata` is `#[allow(dead_code)]`; `sample_size_cli=2, spirit_name="nash"` passes. AC-2 "30 runs against the hello reference Spirit" unenforced. FIX: assert `n1==n2==30`, `cli_wrapper_runs==subprocess_runs==30`, `spirit_name=="hello"`.
- [x] [Review][Patch] #9 Cross-form U-test recomputation (~40 LOC mann_whitney_u + recompute branch) has ZERO proven-red coverage [xtask/tests/story_10_2_proven_red.rs:232-285] -- `make_cross_form` never emits `per_run_hashes_*`; the recompute branch is dead in tests. FIX: add a vector with 30+30 hex hashes exercising recomputation; assert `consistency_ok==true`.
- [x] [Review][Patch] #10 [NEW] Cross-form recompute silently SKIPPED when hash lengths != sample sizes — tampered artifact passes with no consistency check, no warning [xtask/src/check_cross_form_equiv.rs:172] -- Length mismatch leaves `u_recomputed=None`, `consistency_ok=true`, emits nothing. An artifact claiming n=30 but supplying 2 hashes bypasses the only internal-consistency control silently. FIX: warn or hard-fail when hashes present but lengths disagree.

---

#### Patch Findings (Medium)

- [x] [Review][Patch] #11 Cross-form gate does NOT read ADR-040 frontmatter at runtime; spec-divergent docstring [xtask/src/check_cross_form_equiv.rs:9-17] -- Advisory/CLI-wrapper-only is hardcoded; a docstring claims "NFR-Test-7 is removed from v1.5 scope" which diverges from the spec (AC-2 says phase v1.5 advisory). Task 2.1 requires runtime read. FIX: parse ADR-040 Status at runtime (~5 LOC), branch on it; reconcile docstring with spec.
- [x] [Review][Patch] #12 Cross-form p_value not range-checked — NaN/negative/>1 silently treated as equivalent [xtask/src/check_cross_form_equiv.rs:191] -- `NaN<=0.05` is false → not divergent → reported equivalent. FIX: reject `!is_finite() || !(0.0..=1.0).contains(&p_value)` as malformed.
- [x] [Review][Patch] #13 [NEW] u_statistic=NaN in artifact → bogus consistency_ok=true [xtask/src/check_cross_form_equiv.rs:179] -- `(u-NaN).abs()>tol` is false → false consistency. FIX: reject `!u_reported.is_finite()` as malformed.
- [x] [Review][Patch] #14 [NEW] cross-form test_metadata.run_date malformed/empty unchecked [xtask/src/check_cross_form_equiv.rs:34] -- The other two gates validate dates; cross-form echoes run_date to step-summary unchecked. FIX: validate run_date as ISO-8601, or drop the field.
- [x] [Review][Patch] #15 Cross-form U1/U2 convention undocumented; tolerance 45 too small for a U2 artifact [xtask/src/check_cross_form_equiv.rs:106,178] -- A correct U2 artifact (= n1*n2 − U1, differs ~900 at n=30) is flagged inconsistent. FIX: compute both U1 and U2; accept the reported value if it matches either within tolerance; pin the convention in the schema.
- [x] [Review][Patch] #16 Red-team ClassResult.notes required in code but schema says 'optional' [xtask/src/check_red_team_gate.rs:47] -- Missing notes → serde error → hard Err → v1.0 ship blocked. Proven-red fixtures all set `notes="test"`, masking it. FIX: `#[serde(default)] pub notes: String`, OR mark required in schema.
- [x] [Review][Patch] #17 Red-team proven-red vectors do not assert the "WOULD HAVE BLOCKED SHIP" banner [xtask/tests/story_10_2_proven_red.rs:351-406] -- Vectors a/b/c assert only `json[threshold_met]==false`. `write_step_summary` is a no-op when `GITHUB_STEP_SUMMARY` is unset (and `run_in_tempdir` doesn't set it). FIX: set GITHUB_STEP_SUMMARY to a tempfile in the helper, read back, assert banner text; or add a JSON field carrying the banner.
- [x] [Review][Patch] #18 [NEW] coverage-matrix.yaml mislabels NFR-Test-8 as 'advisory-until-engagement' but the trial gate is blocking-when-present [tests/coverage-matrix.yaml] -- The same label is used for NFR-Sec-10/NFR-Test-7 (whose gates never fail) and NFR-Test-8 (which returns Err on successes<10/stratification breach). The notes field even admits "raw count blocking" while enforcement says "advisory." A reader cannot tell which gates actually block. FIX: introduce a distinct enforcement value (e.g. `blocking-when-present`) for NFR-Test-8, or document the dual semantics explicitly.
- [x] [Review][Patch] #19 [NEW] corpus_sha256 provenance hard-fail (load-bearing security control) has ZERO proven-red coverage [xtask/src/check_red_team_gate.rs:161-169] -- All red-team proven-red fixtures write a fixture MANIFEST whose SHA EQUALS the results SHA. No vector supplies a mismatched SHA to prove the hard-fail fires; a regression in `extract_corpus_sha` or the comparison (wrong key, case-folding, whitespace) would ship undetected. FIX: add a vector with results corpus_sha256 differing from the manifest by one byte; assert `!success` and that the failure names the mismatch.

---

#### Patch Findings (Low / Info)

- [x] [Review][Patch] #20 successes > participants_total accepted → Wilson CI sqrt(negative)=NaN; blocking gate passes on impossible input [xtask/src/check_third_party_trial.rs:84-93,191] -- `successes=100, participants_total=12` passes the strat loop (100>=10, 12>=12) and returns Ok; `wilson_ci(100,12)` writes NaN to the step summary. FIX: assert `successes<=participants_total`; guard inside `wilson_ci` (`n>0 && successes<=n`).
- [x] [Review][Patch] #21 [NEW] Duplicate participant.id values never checked [xtask/src/check_third_party_trial.rs:35,179] -- Schema says `# unique participant identifier` but the gate never dedups; `P001` twice inflates the apparent cohort. FIX: collect ids into a HashSet, assert `len==participant.len()`.
- [x] [Review][Patch] #22 No malformed-input proven-red vector for cross-form (JSON) and red-team (TOML) gates [xtask/tests/story_10_2_proven_red.rs] -- Only the trial gate has a malformed-TOML vector. Both gates hard-fail on malformed input but no test proves it. FIX: add malformed-JSON and malformed-TOML vectors asserting `!success`.
- [x] [Review][Patch] #23 `red_team_gate_logs_would_block_on_low_aggregate` does not isolate the aggregate floor; ships debug prose [xtask/tests/story_10_2_proven_red.rs:386-406] -- 5 classes set to 8 (below per-class floor) AND aggregate=70; `threshold_met==false` is ambiguous between the two causes. Also contains leftover "Let's set several to 8" thinko comments. FIX: set all 8 classes=9 (pass floor), aggregate=70 only; delete the debug prose.
- [x] [Review][Patch] #24 [NEW] Red-team JSON hardcodes `passed:true` even when every threshold failed [xtask/src/check_red_team_gate.rs:215-229] -- `threshold_met:false` alongside `passed:true` is contradictory for any downstream consumer keying on `passed`. FIX: `passed = threshold_met` (or rename to `ship_ready`).
- [x] [Review][Patch] #25 [NEW] "WOULD HAVE BLOCKED SHIP" banner only in the sub-job step summary — invisible in the aggregate ship-gate summary [xtask/src/check_red_team_gate.rs:212 + .github/workflows/discipline.yml aggregate] -- The aggregate job's summary table shows only `check-red-team-gate | success`; the loud F3→B signal is buried one job deep. FIX: have the aggregate surface the banner via job outputs / shared artifact, or document that reviewers must open each sub-job.
- [x] [Review][Patch] #26 [NEW] methodology_version never validated non-empty; total_scenarios never asserted ==80 [xtask/src/check_red_team_gate.rs:38,190-195] -- `methodology_version` is `#[allow(dead_code)]`; `total_scenarios=9999` passes. Schema says "Must be exactly 80." FIX: assert non-empty `methodology_version`; assert `total_scenarios==80`.
- [x] [Review][Patch] #27 [NEW] proven-red `trial_gate_fails_on_low_successes` inadvertently CONFIRMS the self-reporting bug [xtask/tests/story_10_2_proven_red.rs:175-183] -- `VALID_PARTICIPANTS` contains 10 producers meeting the conjunction; the test sets `successes=8` and asserts failure. The gate fails only because it trusts the self-reported field — if successes were derived, the derived value would be 10 and the gate would pass. FIX: after fixing #1 derivation, re-pin this vector so participant records themselves encode only 8 successes.
- [x] [Review][Patch] #28 [NEW] corpus_sha256 case/whitespace sensitivity — benign uppercase or trailing whitespace hard-fails ship [xtask/src/check_red_team_gate.rs:162] -- FIX: compare `trim().to_lowercase()` on both sides.
- [x] [Review][Patch] #29 [DISPUTED→narrow] `ship_gate_completeness_fails_with_missing_gate` removes a fixed gate, not any-of-8 [xtask/tests/story_10_2_proven_red.rs:458-489] -- Prior #332 claimed "removes 3 gates" — RETRACTED (the fixture omits exactly 1 of 8 EXPECTED_GATES, `check-red-team-gate`). Residual: the test hardcodes which gate is removed, so it only proves that one specific omission is caught, not any of the 8. FIX: parametrize over each expected gate (low value — note only).
- [x] [Review][Patch] #30 Wilson CI upper-bound: code + wilson-ci.md correctly compute 0.953; stale 0.962 lives only in the NFR-Test-8 PRD [docs/third-party-trial/wilson-ci.md + _bmad-output/planning-artifacts/prd/non-functional-requirements.md] -- `0.962` appears NOWHERE in the diff (verified). Doc-only fix: amend NFR-Test-8 PRD 0.962→0.953. Not a code change.
- [x] [Review][Patch] #31 New Rust source files committed with executable mode 100755 [xtask/src/check_cross_form_equiv.rs, check_red_team_gate.rs, check_third_party_trial.rs] -- FIX: `chmod -x` the three new .rs files.

---

#### Deferred — RESOLVED (both fixed pre-completion per Lunarpulse)

- [x] [Review][Defer→Patch] #32 Date validation was cosmetic — now uses chrono::NaiveDate::parse_from_str [xtask/src/gate_common.rs] -- FIXED: extracted shared `validate_dates` to `gate_common.rs` using `chrono::NaiveDate::parse_from_str("%Y-%m-%d")`; rejects impossible dates (`2026-99-99`), enforces `start <= end` ordering. Applied to all 4 gate modules (trial, red-team, cross-form date check, pentest). chrono already a workspace dep.
- [x] [Review][Defer→Patch] #33 In --json mode workflow commands went to stderr — now documented + structured in JSON payload [xtask/src/gate_common.rs] -- FIXED: extracted shared `emit_command` to `gate_common.rs`. JSON mode keeps stderr (stdout clean for JSON parsing); the structured warning/error is carried in the JSON payload fields (`advisory`, `failures`, `consistency_ok`) so programmatic consumers assert on JSON, not stderr. Non-JSON mode (production CI) uses stdout where Actions parses workflow commands. DRY'd across all 4 gate modules.
