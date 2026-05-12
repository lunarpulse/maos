# Story 0.3: Content-Addressed Corpora Infrastructure + Coverage Matrix CI Gate

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **MAOS test architect**,
I want **the content-addressed corpus harness (SHA-256-pinned JSONL + pinned-judge-LLM contract + quarterly re-baseline workflow + calibration-CI-width math) AND the `tests/coverage-matrix.yaml` CI gate (delivered-phase enforcement + `valid_until` staleness check) to ship together as four xtask subcommands wired into `discipline.yml`, plus a separate `corpus-rebaseline.yml` workflow on `schedule:`**,
so that **every gated NFR has a measurable corpus from day one, every judge call in the test fleet is reproducible by construction, every corpus has a freshness ceiling, and CI fails the moment any delivered FR/NFR has zero corpus rows or any committed corpus's manifest hash drifts — converting NFR-Test-1, NFR-Meta-2, NFR-Meta-3, and NFR-Aud-8 from PRD prose into mechanical gates**.

Story 0.2 committed a forward-compatible 3-row coverage-matrix draft (`I9 / NFR-Test-2 / NFR-Test-9`) with the explicit deferral note **"the file's top-level key is `coverage` … Story 0.3 may rename or restructure"**. This story is where the schema is locked, the gate logic that consumes it lands, and the four founding-sprint corpus-discipline files (`tests/corpora/MANIFEST.toml`, `tests/judge-config.toml`, `tests/phase-config.toml`, `tests/coverage-matrix.yaml`) become the authority every downstream epic (0.4, 0.5, 1a.4, 1b.x, 2.x, 4.x, 5.x, 6.x, 7.x, 9.x, 10.x) reads.

At v0.1-α there are no live judge calls and no Spirit-side corpus content — Story 0.4 commits the first JSONL (`calibration-seed-v0.1.jsonl`, N=100). This story ships the *mechanism*: the verifier, the gate, the schema, the staleness check, the calibration-CI-width math, the rebaseline workflow scaffold, and the structural contract that every future judge call must satisfy. The empty-set must be a valid input — every gate must pass on a workspace with zero corpora.

## Acceptance Criteria

### AC1 — `cargo xtask check-corpus`: SHA-256-pinned JSONL verification (NFR-Test-1)

**Given** a JSONL corpus committed under `tests/corpora/<name>.jsonl` AND its hash registered in `tests/corpora/MANIFEST.toml` under shape `[corpus.<name>] sha256 = "<hex>", schema_version = "<n>", item_count = <n>, valid_until = "<yyyy-mm-dd>", prompt_version_hash = "<hex>", description = "<short prose>"`
**When** CI runs `cargo run -p xtask -- check-corpus --json`
**Then** the xtask iterates every `[corpus.*]` entry in `tests/corpora/MANIFEST.toml`, opens the corresponding `tests/corpora/<name>.jsonl` file, streams it (do **not** load fully into RAM — corpora can reach 10⁵ items per NFR-Sec-4 and 10⁶ rows per NFR-Ops-10), computes a streaming SHA-256, and compares the hex digest to the manifest's `sha256` field
**And** mismatch fails with the literal error message `NFR-Test-1 violation: corpus integrity broken — <name> at <path>: manifest expected <hex_expected>, computed <hex_actual>` (the **exact** prose `corpus integrity broken` is load-bearing — Story 0.3's epic text quotes it verbatim)
**And** a manifest entry whose `<path>` does not exist on disk fails with `NFR-Test-1 violation: corpus missing — <name> at <path>: file does not exist`
**And** a `tests/corpora/*.jsonl` file present on disk but **not** registered in `MANIFEST.toml` fails with `NFR-Test-1 violation: corpus unregistered — <path> has no manifest entry (use 'cargo xtask check-corpus --register <name>' to compute its SHA-256)` — orphan files are not tolerated; the manifest is authoritative
**And** the xtask additionally verifies each line of every JSONL file parses as JSON (per-line `serde_json::from_str` over `Value`); a parse failure fails with `NFR-Test-1 violation: corpus malformed — <name> at <path>:<line>: <serde error>` — JSONL integrity is part of content-addressing because a corrupt line produces a different SHA than a clean one and the actionable error is more useful than "hash mismatch"
**And** at v0.1-α `tests/corpora/MANIFEST.toml` is committed with an empty `[corpus]` table (no rows yet — Story 0.4 commits the first row, `calibration-seed-v0.1`); the xtask **must** exit zero on an empty manifest (the empty-set is a valid input — verified by a `#[test]` named `empty_manifest_passes`)
**And** the xtask exposes a developer-side `--register <name>` mode that reads `tests/corpora/<name>.jsonl`, computes its SHA-256, prints a TOML snippet the developer pastes into `MANIFEST.toml` (the registration step is operator-side ceremony, not auto-write — auto-writing would be a footgun for SHA-256 manifests; the helper is a convenience, not a substitute for review)

### AC2 — `cargo xtask check-judge-config`: pinned-judge-LLM structural contract (NFR-Test-1)

**Given** a judge-LLM contract committed at `tests/judge-config.toml` declaring the four mandatory constraints from epic-0 / Story 0.3 BDD2 — `[judge.<name>] model = "<provider:model_id@version>", temperature = 0.0, top_p = 1.0, seed = <u64>, retry_budget = 1, prompt_version_hash = "<hex>", added_in_story = "<id>"`
**When** CI runs `cargo run -p xtask -- check-judge-config --json`
**Then** the xtask reads `tests/judge-config.toml`, validates every `[judge.<name>]` entry passes **all six** structural checks (parses as `JudgeConfig` shape; `temperature == 0.0` exactly — not 0, not 0.00001; `top_p == 1.0` exactly; `seed` is a `u64` (not absent); `retry_budget == 1` exactly; `prompt_version_hash` matches `^[0-9a-f]{64}$`); and fails with structured per-row error messages
**And** the error for a non-zero temperature is `NFR-Test-1 violation: judge '<name>' has temperature=<value>; pinned-judge contract requires temperature=0.0 (epic-0 / Story 0.3 BDD2)`; analogous messages for the other five constraints
**And** at v0.1-α `tests/judge-config.toml` is committed with an empty `[judge]` table and a documentary header comment (TOML `#`) explaining the contract; the xtask exits zero on an empty table (verified by `#[test] empty_judge_config_passes`)
**And** the xtask **additionally** AST-walks every `*.rs` file under `tests/` and any `crates/*/tests/` directories (use the existing `fs_walk::collect_rs_files`) looking for `syn::Expr::Path` segments whose final path component matches the case-sensitive identifier set `{"reqwest_post_completion", "anthropic_messages", "openai_chat_completions", "openai_chat", "ollama_chat", "completions_create", "messages_create"}` (the canonical "I'm directly calling a provider API" identifiers — the same set that downstream Stories 1b.4 and 5.5b will route through the kernel Inference Port); any match in test code is rejected with `NFR-Test-1 violation: direct judge-LLM call at <file>:<line>: route via JudgeRunner trait + tests/judge-config.toml (epic-0 / Story 0.3 BDD2)`
**And** the identifier set lives in `xtask/judge-direct-call-identifiers.toml` (flat `direct_calls = [...]` list), is empty-tolerant at v0.1-α (no test code yet calls providers), and extension follows the same tightening-requires-invariant-lock convention as Story 0.2's loom blocklist (TOML comment at file top documents this)
**And** **Why a structural lint instead of a runtime check at v0.1-α:** there are no Spirits and therefore no judge calls in v0.1-α test code. The contract has to be enforceable *before* the first judge call lands so Story 0.4's calibration seed corpus, Story 4.4's distillation five-metric gate, and Story 5.5b's multi-provider matrix all encounter the contract as a precondition rather than retrofitting it. The structural lint locks the channel; the runtime check follows when the channel is non-empty.

### AC3 — `corpus-rebaseline.yml` quarterly workflow + `xtask rebaseline-check` ≥98% agreement (NFR-Test-1)

**Given** a separate GitHub Actions workflow `.github/workflows/corpus-rebaseline.yml` triggered by `schedule: cron: '0 14 1 */3 *'` (14:00 UTC on the 1st day of January / April / July / October — the quarterly cadence Murat's amended NFR-Test-1 demands) AND `workflow_dispatch:` (manual trigger for the dev agent to exercise locally)
**When** the workflow runs
**Then** the workflow checks out `main`, installs the Rust toolchain (matching `discipline.yml`'s pin), runs `cargo run -p xtask -- rebaseline-check --json --out /tmp/rebaseline-report.json` against the current `MANIFEST.toml`, and uploads the report as a workflow artifact
**And** the `rebaseline-check` xtask (new module `xtask/src/rebaseline_check.rs`) iterates every `[corpus.<name>]` in `MANIFEST.toml` that carries a `judge_id = "<judge_name>"` field (corpora that don't use a judge — e.g., manifest-fuzz corpora — skip rebaseline by construction), and for each corpus produces a `RebaselineReport { passed: bool, items_total: usize, items_agreed: usize, agreement_ratio: f64 (4-decimal precision), threshold: 0.98, per_corpus: Vec<CorpusAgreement> }`
**And** `agreement_ratio < 0.98` produces a structured violation: `NFR-Test-1 violation: corpus <name> agreement ratio <ratio> below quarterly threshold 0.98 — open re-baseline review issue` (matches the BDD3 prose verbatim including the imperative "open re-baseline review issue")
**And** at v0.1-α the workflow runs against an empty manifest (no `judge_id`-carrying corpora exist yet) and the xtask returns `RebaselineReport { passed: true, items_total: 0, items_agreed: 0, agreement_ratio: 1.0, per_corpus: vec![] }` — the empty-set baseline is `agreement_ratio = 1.0` by mathematical convention (vacuous truth on the empty set; documented as a comment in the xtask)
**And** **`rebaseline-check` does NOT make live LLM calls at v0.1-α.** The judge-call mechanism (the trait `JudgeRunner` that abstracts provider invocations) lands in Story 1b.4 with the Inference Port; this xtask at v0.1-α dispatches via a `JudgeRunner::offline_mode_eq` shim that compares each corpus item's `expected_judgment` field against itself (trivially passes at 100%) — the **plumbing exists end-to-end** so downstream Story 1b.4 plugs in the real `JudgeRunner` without restructuring the gate
**And** the workflow also emits a JSON summary comment on the PR (when triggered by `workflow_dispatch:` with a `pr-number` input) using the same `<!-- discipline-gate-comment -->`-style upsert sentinel — but in a **different comment** keyed by `<!-- rebaseline-comment -->` so the two never collide; cron-triggered runs post the artifact only

### AC4 — `cargo xtask coverage-matrix`: delivered-phase enforcement (NFR-Meta-3)

**Given** the canonical `tests/coverage-matrix.yaml` schema locked by this story (top-level keys: `schema_version: 1`, `current_phase: "v0.1-alpha"`, `mode: "warning"` (Story 0.4 flips to `"hard"` at v0.3 per Story 0.4 AC6), `phase_order: ["v0.1-alpha", "v0.1", "v0.3", "v0.5", "v0.8", "v1.0", "v1.5", "v2.0+"]`, and `coverage: <map>` whose keys are FR/NFR/invariant ids and whose values match the shape `{ gates: [<gate_name>], corpora: [<corpus_name>], phase: "<phase_string>", valid_until: "<yyyy-mm-dd>", notes: "<optional>" }`)
**When** CI runs `cargo run -p xtask -- coverage-matrix --json`
**Then** the xtask parses `tests/coverage-matrix.yaml` (use `serde_yaml` 0.9 — **new dependency**, added to `xtask/Cargo.toml` in this story; documented under "library / framework requirements"), validates the top-level fields are present and well-typed, validates `current_phase ∈ phase_order`, and validates each row's `phase ∈ phase_order`
**And** the xtask reads `tests/phase-config.toml` containing `current_phase = "<phase>"` (this is the **single source of truth** for the current phase; the YAML's `current_phase` field MUST match it, fail with `NFR-Meta-3 violation: phase mismatch — coverage-matrix.yaml says <yaml_phase>, phase-config.toml says <toml_phase>` if they drift — defense-in-depth against accidental phase rollback)
**And** for each row whose `phase` is **at or before** `current_phase` (the **delivered** set, per the BDD4 wording "delivered ≤ current-phase"; comparison uses `phase_order` index, not string compare), the xtask checks that **either** `gates` is non-empty OR `corpora` is non-empty (zero on **both** = uncovered); the rule "gates OR corpora satisfies coverage" is the v0.1-α reading because gates like `check-empty-kernel` are themselves the falsifying surface (no JSONL needed) — the v0.3 hard-mode reading per Story 0.4 will tighten to "every delivered NFR with a numeric floor requires at least one corpus"; document this loosening explicitly in a `// Phase-dependent rule:` comment
**And** **mode = "warning":** at v0.1-α the xtask emits violations to stderr (and to `--json`) but returns exit code zero; **mode = "hard":** the xtask returns exit code non-zero on any violation. The mode switch is the **only** behavioral change between v0.1-α and v0.3; Story 0.4 flips `mode` in the yaml, no xtask change. This split is what makes Story 0.4's "warning-only at v0.1, hard at v0.3" promise testable as a one-line PR diff
**And** the violation message for uncovered delivered row is `NFR-Meta-3 violation: <fr_or_nfr_id> delivered at <phase> has zero corpus and zero gate coverage` (singular form even if both arrays empty — clarity over redundancy)
**And** for rows whose `phase` is **after** `current_phase` (the **deferred** set), the xtask logs them under `out_of_scope_deferred: [...]` in the JSON report (per BDD4's "explicitly labels deferred FR/NFRs … as out-of-scope for the current phase's gate") but does NOT treat them as violations
**And** the xtask validates every `corpora: [<name>]` entry references a `MANIFEST.toml` row whose `<name>` exists (cross-file referential integrity); a dangling reference fails with `NFR-Meta-3 violation: <fr_or_nfr_id> references unknown corpus '<name>' (not in tests/corpora/MANIFEST.toml)` — this catches Story 0.4 / 0.5 / downstream typos at the gate, not at corpus-load time
**And** the xtask validates every `gates: [<name>]` entry against the canonical gate registry committed at `xtask/gate-registry.toml` (root key `gates = [...]`, initial entries: `["reproducible-build", "check-unsafe", "kloc-check", "abi-diff", "invariant-lock", "check-empty-kernel", "check-loom", "check-service-boundary", "check-corpus", "check-judge-config", "coverage-matrix", "corpus-staleness"]`); a dangling gate reference fails with `NFR-Meta-3 violation: <fr_or_nfr_id> references unknown gate '<name>' (not in xtask/gate-registry.toml)`
**And** Story 0.2's three rows (`I9`, `NFR-Test-2`, `NFR-Test-9`) MUST be preserved verbatim under the new schema (their `phase`, `gates`, `corpora` fields exact-match-conform); this PR upgrades the schema **non-destructively** by adding `valid_until: "2027-05-11"` (12 months from this story's creation date 2026-05-12) and `schema_version: 1` etc. — no row is dropped, no field is renamed

### AC5 — `cargo xtask corpus-staleness`: `valid_until` enforcement (NFR-Meta-2)

**Given** a row in `tests/coverage-matrix.yaml` whose `valid_until` date is **strictly before** the system date at gate-run time (UTC; date-only comparison via `chrono::NaiveDate`)
**When** CI runs `cargo run -p xtask -- corpus-staleness --json`
**Then** the xtask iterates every `coverage.<id>` row whose `phase` is `≤ current_phase` (deferred rows are not yet active so their staleness is by definition irrelevant), parses `valid_until` as `NaiveDate`, compares against the current date, and emits violation `NFR-Meta-2 violation: <fr_or_nfr_id> corpus expired <yyyy-mm-dd> (current=<yyyy-mm-dd>); either extend with assessor sign-off PR or rebuild` for every expired row
**And** the xtask **additionally** iterates every `tests/corpora/MANIFEST.toml` `[corpus.<name>]` row whose `valid_until` field is in the past (manifest-side staleness is independent of yaml-side staleness; both must pass), emitting `NFR-Meta-2 violation: corpus <name> expired <yyyy-mm-dd>; either extend with assessor sign-off PR or rebuild`
**And** the gate also **warns** (does NOT fail; emit on stderr only and surface in `--json` under a separate `warnings: [...]` array) on rows whose `valid_until` is within **30 days** of expiry — actionable advance notice prevents Friday-afternoon-firefighting; this is operator-friendly behavior and Murat's discipline ("audit at corpus creation + every 12 months") implies the warning surface
**And** at v0.1-α (no real corpora), the gate exits zero with `warnings: []` and `violations: []`
**And** the gate's clock source is `chrono::Utc::now().date_naive()` (no time-of-day; pure date comparison) — this avoids cross-time-zone flakes between the developer's local run and the CI runner; documented in a `// Clock source:` comment at the top of `xtask/src/corpus_staleness.rs`
**And** **No-update justification PR flow is operator-side ceremony, not xtask logic** (matches Story 0.1's reviewer-pair pattern): a PR that bumps `valid_until` must also touch `docs/corpus-extensions/<id>.md` (a new dir this story creates with `README.md` only); the discipline document explains the contract but the gate does not verify it at v0.1-α — Story 0.5 or a later epic owns the cross-file referential check when the first real corpus actually expires

### AC6 — Calibration-CI-width math + `xtask calibrate` (NFR-Aud-8)

**Given** the NFR-Aud-8 two-tier corpus discipline — N=100 per-commit (CI-width ≈ 0.124 sufficient for trend detection) and N=500 quarterly (CI-width ≤ 0.05 at p=0.90 for digest-recall)
**When** CI runs `cargo run -p xtask -- calibrate --corpus <name> --n <100|500> --p <0.90|0.95> --json`
**Then** the xtask computes the **Wilson-score interval** half-width for a given (n, p) and reports `CalibrationReport { corpus: String, n: usize, pass_rate: f64, ci_lower: f64, ci_upper: f64, ci_width: f64, threshold: Option<f64>, passed: bool }` — Wilson score is used (not normal-approximation Wald) because n=100 with pass rates near 0/1 has well-known boundary failure under Wald; Wilson is the textbook choice (cite: Agresti & Coull 1998) and is what Murat's NFR-Aud-8 amendment implicitly assumes
**And** the xtask exposes a pure-function `wilson_ci(successes: u64, n: u64, z: f64) -> (f64, f64)` with `#[cfg(test)]` unit tests asserting the textbook values: (n=100, p=0.5, z=1.96) → (0.4038, 0.5962) ± 0.001; (n=500, p=0.95, z=1.645) → (0.9335, 0.9650) ± 0.001; (n=0, _, _) → (0.0, 1.0) — empty-set returns full-uncertainty interval per the Wilson definition
**And** for the per-commit pipeline (n=100, p=0.95): the xtask checks `ci_width ≤ 0.20` (Wilson half-width × 2 ≈ 0.085 at the textbook calibration; the epic-0 / Story 0.3 BDD6 prose says "CI-width ≈ 0.124" — that is the looser **Wald** approximation; this story commits to Wilson, **which is tighter than Wald at the same n**, so the "≈ 0.124 sufficient for trend detection" floor remains true and the gate is more honest); document this delta in a code comment so the next test architect doesn't think the math is wrong
**And** for the quarterly pipeline (n=500, p=0.90): the xtask checks `ci_width ≤ 0.05` and fails with `NFR-Aud-8 violation: corpus <name> quarterly CI-width <width> exceeds 0.05 at p=0.90 — increase N or accept wider window with assessor sign-off`
**And** at v0.1-α: no real calibration corpus exists; the xtask runs against synthetic input (a `--synthetic-n 100 --synthetic-pass-rate 0.95` mode used **only** in `#[cfg(test)]` integration tests, not in CI) and `#[test]` cases verify the math against textbook values; the production-CI invocation is wired into `discipline.yml` but **gated** on `[corpus.calibration-seed-v0.1]` existing in `MANIFEST.toml` (no corpus = no calibration run = `CalibrationReport { passed: true, n: 0, ... }`); Story 0.4 lands the corpus and the gate becomes live
**And** the gate's output JSON shape is `serde`-round-trippable per the Story 0.1 convention (round-trip test mandatory)

### AC7 — Adversarial proof: each gate fails independently on a deliberate violation

**Given** the four new xtask subcommands (`check-corpus`, `check-judge-config`, `coverage-matrix`, `corpus-staleness`) plus `rebaseline-check` and `calibrate`, AND the existing fixture-tree pattern from Story 0.1 / 0.2
**When** the dev agent commits five fixture trees: `xtask/tests/fixtures/violation-corpus/` (manifest declares SHA `0xabc...` but JSONL hashes to `0xdef...`), `xtask/tests/fixtures/violation-judge-config/` (judge entry with `temperature = 0.5`), `xtask/tests/fixtures/violation-coverage-matrix/` (delivered row with empty `gates` and `corpora`), `xtask/tests/fixtures/violation-staleness/` (row with `valid_until: "2020-01-01"`), and `xtask/tests/fixtures/violation-judge-direct-call/` (test `*.rs` with `anthropic_messages(...)`)
**Then** `xtask/tests/check_corpus_integration.rs` asserts `cargo run -p xtask -- check-corpus --manifest xtask/tests/fixtures/violation-corpus/MANIFEST.toml --corpora-dir xtask/tests/fixtures/violation-corpus/corpora` exits non-zero AND stderr contains the literal string `NFR-Test-1 violation: corpus integrity broken` AND the offending corpus name AND the manifest-expected and computed hashes
**And** `xtask/tests/check_judge_config_integration.rs` asserts equivalent against `violation-judge-config/` with stderr containing `NFR-Test-1 violation: judge` and the offending parameter (`temperature=0.5`)
**And** the same harness against `violation-judge-direct-call/` asserts stderr contains `NFR-Test-1 violation: direct judge-LLM call` and the offending call identifier
**And** `xtask/tests/coverage_matrix_integration.rs` asserts equivalent against `violation-coverage-matrix/` with stderr containing `NFR-Meta-3 violation:` and the offending FR/NFR id — AND the gate's exit code depends on `mode`: `mode = "hard"` exits non-zero, `mode = "warning"` exits zero but stderr still contains the violation prose (verified by two test cases, one per mode)
**And** `xtask/tests/corpus_staleness_integration.rs` asserts equivalent against `violation-staleness/` with stderr containing `NFR-Meta-2 violation:` and the offending date
**And** each fixture tree has a paired clean tree (`clean-corpus/`, `clean-judge-config/`, `clean-coverage-matrix/`, `clean-staleness/`, `clean-judge-direct-call/`) that asserts the corresponding xtask exits zero (mirrors the `with-unsafe` / `without-unsafe` and `violation-i9` / `clean-i9` pattern)
**And** the four new `discipline.yml` jobs (`check-corpus`, `check-judge-config`, `coverage-matrix`, `corpus-staleness`) wire as siblings of the existing eight jobs (independent `needs:` graph; aggregated by the existing `aggregate` job); the `aggregate` job's PR-comment table extends to include the four new rows preserving the `<!-- discipline-gate-comment -->` upsert sentinel

### AC8 — Coverage-matrix bootstrap rows + schema migration from Story 0.2's draft

**Given** Story 0.2 committed `tests/coverage-matrix.yaml` with three rows (`I9`, `NFR-Test-2`, `NFR-Test-9`) under top-level key `coverage:` and explicitly deferred schema lockdown to this story
**When** this story's PR lands
**Then** the YAML is upgraded **non-destructively** to the schema locked in AC4: top-level `schema_version: 1`, `current_phase: "v0.1-alpha"`, `mode: "warning"`, `phase_order: [...]`, `coverage: { ... }`; the three Story 0.2 rows preserve their `gates` and `phase` exactly, gain a `valid_until: "2027-05-11"` field (12 months from this story's date, per NFR-Meta-2 default validity), and gain `corpora: []` (already there from 0.2; preserved)
**And** four **new** rows are added for the NFRs this story itself mechanically enforces:
  - `NFR-Test-1: { gates: ["check-corpus", "check-judge-config", "corpus-rebaseline"], corpora: [], phase: "v0.1-alpha", valid_until: "2027-05-11", notes: "rebaseline-check is the scheduled-workflow gate; check-corpus + check-judge-config are per-commit" }`
  - `NFR-Meta-2: { gates: ["corpus-staleness"], corpora: [], phase: "v0.1-alpha", valid_until: "2027-05-11" }`
  - `NFR-Meta-3: { gates: ["coverage-matrix"], corpora: [], phase: "v0.1-alpha", valid_until: "2027-05-11", notes: "warning-only at v0.1 per Story 0.4 AC6; hard at v0.3" }`
  - `NFR-Aud-8: { gates: ["calibrate"], corpora: [], phase: "v0.5", valid_until: "2027-05-11", notes: "per-commit calibration N=100 lands with corpus in Story 0.4; quarterly N=500 at v1.0" }`
**And** `xtask/gate-registry.toml` is committed with the canonical 12-gate list from AC4 (the eight existing gates from Story 0.1 / 0.2 plus the four new ones from this story); `rebaseline-check` is **also** included as a 13th entry (the scheduled-workflow gate the YAML row above references)
**And** `tests/phase-config.toml` is committed with `current_phase = "v0.1-alpha"` and a `phase_order` mirror — single-source-of-truth, the YAML must match (verified by AC4's cross-file consistency check)
**And** `tests/corpora/MANIFEST.toml` is committed with **only** a documentary header (TOML comments explaining the schema) and an empty `[corpus]` table — Story 0.4 commits the first row (`calibration-seed-v0.1`)
**And** `tests/judge-config.toml` is committed with a documentary header and an empty `[judge]` table — Story 1b.4 commits the first row when the Inference Port lands
**And** `xtask/judge-direct-call-identifiers.toml` is committed with the seven canonical identifiers from AC2 — extension requires invariant-lock review per the Story 0.2 blocklist convention
**And** Story 0.2's `invariant-lock` xtask's corpus-delta check (`tests/coverage-matrix.yaml` must be touched on invariant-PRs) **continues to pass by construction** since this PR modifies the file; no invariant-lock-relevant `I*.md` file is touched by this story, so the gate fires "no invariants touched → pass" path

## Tasks / Subtasks

- [x] **Task 1: Add `check-corpus` SHA-256 verifier and JSONL parse-check (AC1, AC7)**
  - [x] Add `CheckCorpus { manifest: String, corpora_dir: String, register: Option<String>, json: bool }` variant to the `Commands` enum in `xtask/src/main.rs` with defaults `--manifest=tests/corpora/MANIFEST.toml` and `--corpora-dir=tests/corpora`.
  - [x] Create `xtask/src/check_corpus.rs` implementing the streaming SHA-256 (use `sha2::Sha256` already in `xtask/Cargo.toml` from Story 0.2; **do not** pull in `sha256` or `ring`).
  - [x] Define `CorpusManifest { corpus: BTreeMap<String, CorpusEntry> }` and `CorpusEntry { sha256: String, schema_version: u32, item_count: usize, valid_until: String, prompt_version_hash: String, description: String, judge_id: Option<String> }` (matches AC1 schema; `judge_id` is `Option` for corpora that don't use a judge — manifest-fuzz, wire-fuzz, etc.).
  - [x] Implement streaming hash: open file in `BufReader`, read in 64 KiB chunks, `update()` the hasher per chunk, hex-encode the final digest. Document in a comment that loading 10⁶-row corpora fully into memory (NFR-Ops-10) would OOM the runner.
  - [x] Implement per-line `serde_json::from_str::<serde_json::Value>` parse-check; report (file, line_number, serde error) on failure.
  - [x] Implement orphan-file detection: list every `*.jsonl` under `corpora-dir`, set-difference against `MANIFEST.toml` keys, report orphans.
  - [x] Implement `--register <name>` mode: reads the JSONL, computes SHA-256, prints a ready-to-paste TOML snippet to stdout (do **NOT** modify `MANIFEST.toml` — auto-write is a footgun; the developer reviews before pasting).
  - [x] Unit tests in `check_corpus.rs`: known-hash JSONL round-trip; mismatch detection; missing-file detection; orphan detection; per-line parse-error detection; `empty_manifest_passes`; JSON round-trip of `CorpusManifest`.
  - [x] Add `xtask/tests/check_corpus_integration.rs` plus fixture trees `xtask/tests/fixtures/violation-corpus/` (manifest claims hash X, JSONL hashes to Y) and `xtask/tests/fixtures/clean-corpus/` (manifest hash matches JSONL hash on a 3-line synthetic JSONL).

- [x] **Task 2: Add `check-judge-config` structural-contract validator (AC2, AC7)**
  - [x] Add `CheckJudgeConfig { config: String, identifiers: String, json: bool }` variant; defaults `--config=tests/judge-config.toml`, `--identifiers=xtask/judge-direct-call-identifiers.toml`.
  - [x] Create `xtask/src/check_judge_config.rs`: parse the TOML into `JudgeConfig { judge: BTreeMap<String, JudgeEntry> }`; validate each entry per AC2's six structural checks (`temperature == 0.0`, `top_p == 1.0`, `seed: u64`, `retry_budget == 1`, `prompt_version_hash` matches `^[0-9a-f]{64}$`, `model` matches `^[a-z0-9_-]+:[a-zA-Z0-9._:/@-]+$`).
  - [x] Implement the AST scan: walk `tests/**/*.rs` and `crates/*/tests/**/*.rs` via `fs_walk::collect_rs_files`; use `syn::visit::Visit` with `visit_expr_call` and `visit_expr_path` to collect call-site identifiers; match the rightmost path segment against the identifier set; report `(file, line, identifier)` violations.
  - [x] Skip `#[cfg(test)] mod tests { ... }` inside the xtask crate itself (avoid self-false-positive when the integration test fixtures contain `anthropic_messages(...)` to *trigger* the lint).
  - [x] Commit `xtask/judge-direct-call-identifiers.toml` with the AC2 seven identifiers (`reqwest_post_completion`, `anthropic_messages`, `openai_chat_completions`, `openai_chat`, `ollama_chat`, `completions_create`, `messages_create`) and a TOML-comment header documenting that extending the list is a tightening per the Story 0.2 blocklist convention.
  - [x] Unit tests: empty-config passes; non-zero temperature fails; missing seed fails; bad prompt-hash format fails; direct-call identifier in test code fails; legitimate occurrence inside a `// comment` does NOT fail (AST not grep).
  - [x] Add `xtask/tests/check_judge_config_integration.rs` plus fixture trees `violation-judge-config/` (one entry with `temperature = 0.5`), `clean-judge-config/` (one well-formed entry), `violation-judge-direct-call/` (a test `.rs` with `anthropic_messages(...)`), `clean-judge-direct-call/` (a test `.rs` with no provider call).

- [x] **Task 3: Add `coverage-matrix` gate + lock the YAML schema (AC4, AC7, AC8)**
  - [x] Add `serde_yaml = "0.9"` to `xtask/Cargo.toml` (new dependency; document the choice in the Cargo.toml comment header — `serde_yaml` 0.9 is the most-maintained YAML 1.2 parser in the Rust ecosystem; alternative `yaml-rust2` is also viable but `serde_yaml`'s `serde` integration is tighter for the round-trip-test requirement).
  - [x] Add `CoverageMatrix { config: String, phase_config: String, manifest: String, gate_registry: String, json: bool }` variant; defaults `--config=tests/coverage-matrix.yaml`, `--phase-config=tests/phase-config.toml`, `--manifest=tests/corpora/MANIFEST.toml`, `--gate-registry=xtask/gate-registry.toml`.
  - [x] Create `xtask/src/coverage_matrix.rs`: parse YAML into `CoverageMatrixFile { schema_version: u32, current_phase: String, mode: String, phase_order: Vec<String>, coverage: BTreeMap<String, CoverageRow> }`; parse phase-config TOML; cross-check; iterate `coverage` and apply the AC4 logic.
  - [x] Implement `phase_le(a: &str, b: &str, order: &[String]) -> bool` using `phase_order.iter().position()` — string-compare is wrong (`"v1.5" < "v0.1"` lexicographically); the order list is canonical.
  - [x] Implement mode-dependent exit code: `mode == "hard"` returns non-zero on any violation; `mode == "warning"` returns zero but still emits the violation to stderr and `--json`.
  - [x] Implement cross-file referential checks: every `corpora: [<name>]` must exist in `MANIFEST.toml`; every `gates: [<name>]` must exist in `xtask/gate-registry.toml`.
  - [x] Commit `xtask/gate-registry.toml` with the 13-entry canonical list from AC4/AC8.
  - [x] Commit `tests/phase-config.toml` with `current_phase = "v0.1-alpha"` and the `phase_order` mirror.
  - [x] Upgrade `tests/coverage-matrix.yaml` non-destructively per AC8: add top-level fields, preserve the three Story 0.2 rows, add four new rows.
  - [x] Unit tests: empty `coverage` passes; uncovered delivered row violates; deferred row goes to `out_of_scope_deferred`; dangling corpus ref fails; dangling gate ref fails; mode-warning returns zero exit code with violations in JSON; phase-config mismatch fails; JSON round-trip of `CoverageMatrixFile` and `CoverageReport`.
  - [x] Add `xtask/tests/coverage_matrix_integration.rs` plus fixture trees `violation-coverage-matrix/` (delivered row with empty arrays) and `clean-coverage-matrix/` (the locked-schema yaml itself, copied).

- [x] **Task 4: Add `corpus-staleness` `valid_until` enforcement (AC5, AC7)**
  - [x] Add `chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }` to `xtask/Cargo.toml` — `default-features = false` is intentional because the default features pull in `oldtime`/`serde` integrations we do not need at v0.1-α; the `clock` feature provides `Utc::now()` and `std` is required for `NaiveDate::parse_from_str`.
  - [x] Add `CorpusStaleness { config: String, manifest: String, warn_window_days: i64, json: bool }` variant; defaults `--config=tests/coverage-matrix.yaml`, `--manifest=tests/corpora/MANIFEST.toml`, `--warn-window-days=30`.
  - [x] Create `xtask/src/corpus_staleness.rs`: re-use `CoverageMatrixFile` and `CorpusManifest` types from Tasks 2/3 (factor shared types into `xtask/src/corpus_types.rs` — keep the three xtask modules from over-importing each other).
  - [x] Implement the two-pass scan: yaml rows with `phase ≤ current_phase` AND expired `valid_until` → violation; manifest rows with expired `valid_until` → violation; rows within `warn_window_days` of expiry → warning.
  - [x] Implement clock isolation: `chrono::Utc::now().date_naive()` is the only clock call; pass it through pure helper functions so tests can inject a fixed date.
  - [x] Unit tests: expired row violates; not-yet-due row passes; within-warn-window emits warning; bad date format fails with structured error; JSON round-trip of `StalenessReport`.
  - [x] Add `xtask/tests/corpus_staleness_integration.rs` plus fixture trees `violation-staleness/` (one row with `valid_until: "2020-01-01"`) and `clean-staleness/` (rows dated 2027-05-11).

- [x] **Task 5: Add `calibrate` Wilson-CI math (AC6, AC7)**
  - [x] Add `Calibrate { corpus: String, n: u64, p: f64, synthetic_pass_rate: Option<f64>, json: bool }` variant.
  - [x] Create `xtask/src/calibrate.rs`: pure-function `pub fn wilson_ci(successes: u64, n: u64, z: f64) -> (f64, f64)` (handle `n == 0` → `(0.0, 1.0)`; handle `successes > n` → return `Err` not `panic!`). z-mapping: `z(0.90) = 1.645`, `z(0.95) = 1.96`, `z(0.99) = 2.576`.
  - [x] Implement `calibrate_corpus` orchestration: at v0.1-α with no real judge, the corpus's pass rate comes from the `expected_judgment` field of each JSONL item — if all items carry `expected_judgment` then the synthetic `pass_rate = 1.0` (vacuous truth path); when Story 1b.4 lands the `JudgeRunner`, this orchestration plugs in real judgment.
  - [x] Implement gate thresholds: `(n=100, p=0.95)` → assert `ci_width ≤ 0.20`; `(n=500, p=0.90)` → assert `ci_width ≤ 0.05`. Document that the BDD6 "≈ 0.124" prose is the Wald approximation; Wilson is tighter at the same n, so the contract is honored more strictly than the prose demands.
  - [x] Unit tests: textbook (n=100, p=0.5, z=1.96) → (0.4038, 0.5962) ± 0.001; (n=500, p=0.95, z=1.645) → (0.9335, 0.9650) ± 0.001; (n=0, _, _) → (0.0, 1.0); n=100 with `ci_width > 0.20` fails; n=500 with `ci_width > 0.05` at p=0.90 fails; JSON round-trip of `CalibrationReport`.
  - [x] Wire into `discipline.yml` as job `calibrate-per-commit` that runs `cargo run -p xtask -- calibrate --corpus calibration-seed-v0.1 --n 100 --p 0.95 --json`; **but** gate the job on the corpus existing in the manifest (use a conditional `if: hashFiles('tests/corpora/calibration-seed-v0.1.jsonl') != ''` step) so v0.1-α (no corpus) passes via "step skipped"; Story 0.4 lands the corpus and the gate becomes live.

- [x] **Task 6: Add `rebaseline-check` xtask + `corpus-rebaseline.yml` scheduled workflow (AC3, AC7)**
  - [x] Add `RebaselineCheck { manifest: String, corpora_dir: String, judge_config: String, threshold: f64, out: Option<String>, json: bool }` variant; defaults `--threshold=0.98`.
  - [x] Create `xtask/src/rebaseline_check.rs`: define the `JudgeRunner` trait with method `fn judge(&self, item: &serde_json::Value, expected: &serde_json::Value) -> Result<bool, String>`; ship a v0.1-α `OfflineMode` struct that compares `item == expected` (trivially passes) and document explicitly that Story 1b.4 ships the real `InferencePortJudge` struct that swaps in here.
  - [x] Compute per-corpus agreement ratio (4-decimal precision); aggregate into `RebaselineReport`; report violations for ratios below threshold.
  - [x] Implement vacuous-truth on empty manifest: `agreement_ratio = 1.0` with `items_total = 0`.
  - [x] Create `.github/workflows/corpus-rebaseline.yml` with `schedule: cron: '0 14 1 */3 *'` + `workflow_dispatch:` triggers; checkout + toolchain + `cargo run -p xtask -- rebaseline-check --json --out /tmp/rebaseline-report.json`; upload artifact via `actions/upload-artifact@v4`; on `workflow_dispatch` with `pr-number` input, also post a PR comment keyed by `<!-- rebaseline-comment -->`.
  - [x] Unit tests: empty manifest → vacuous-truth pass; ratio below threshold violates; offline-mode equality check works; JSON round-trip of `RebaselineReport` and `CorpusAgreement`.
  - [x] **Do not** wire `rebaseline-check` into `discipline.yml` per-commit — the entire point of the quarterly cadence is to limit cost/latency exposure of judge-LLM calls; that's a deliberate scope boundary.

- [x] **Task 7: Wire the four per-commit gates into `discipline.yml` (AC4, AC7, AC8)**
  - [x] Add four jobs `check-corpus`, `check-judge-config`, `coverage-matrix`, `corpus-staleness` to `.github/workflows/discipline.yml`, each independent (no `needs:` between them), each modeled on `check-empty-kernel` (checkout, toolchain, rust-cache, `cargo run -p xtask -- <subcommand> --json`).
  - [x] Extend the `aggregate` job's `needs:` array to include the four new jobs (existing 8 → 12).
  - [x] Extend the `aggregate` job's PR-comment table with four new rows (`check-corpus`, `check-judge-config`, `coverage-matrix`, `corpus-staleness`) preserving the `<!-- discipline-gate-comment -->` upsert sentinel.
  - [x] Add the `calibrate-per-commit` job per Task 5 (conditionally enabled).
  - [x] Verify total wall-clock budget (<5 min on the project's runner class) still holds; document in PR description; the four new gates share Cargo cache so cold-start cost is one-time.

- [x] **Task 8: Commit the four founding-sprint discipline files + update docs (AC4, AC5, AC8)**
  - [x] Commit `tests/corpora/MANIFEST.toml` with a documentary TOML-comment header (explains schema, links to Story 0.3, links to NFR-Test-1) and an empty `[corpus]` table.
  - [x] Commit `tests/judge-config.toml` with documentary header (explains the six structural constraints) and empty `[judge]` table.
  - [x] Commit `tests/phase-config.toml` with `current_phase = "v0.1-alpha"` and `phase_order = [...]`.
  - [x] Commit `xtask/gate-registry.toml` with the 13 canonical gate names.
  - [x] Commit `xtask/judge-direct-call-identifiers.toml` with the seven canonical identifiers from AC2.
  - [x] Upgrade `tests/coverage-matrix.yaml` per AC8 (preserves Story 0.2's 3 rows; adds top-level fields; adds 4 new rows).
  - [x] Create `docs/corpus-extensions/README.md` documenting the no-update-justification PR flow (one-paragraph: how to extend `valid_until`, who signs off, what the file under `docs/corpus-extensions/<id>.md` looks like).
  - [x] Update `docs/ci-baselines/README.md`: extend the **Founding-sprint baseline gates** table with five new rows (`check-corpus`, `check-judge-config`, `coverage-matrix`, `corpus-staleness`, `corpus-rebaseline`); the `corpus-rebaseline` row is annotated "scheduled, not per-commit".
  - [x] Update `docs/ci-baselines/v0.1-alpha.json` extending `gate_results` with `check_corpus: pending`, `check_judge_config: pending`, `coverage_matrix: pending`, `corpus_staleness: pending`, `calibrate: pending` slots.

- [x] **Task 9: Verify kloc-check budget headroom after the corpus-discipline additions**
  - [x] Run `cargo xtask kloc-check` locally; verify `xtask` per-crate budget (3000 LOC per `xtask/kloc.toml`) is not breached by the six new modules (estimate ~1000–1300 new LOC across `check_corpus.rs`, `check_judge_config.rs`, `coverage_matrix.rs`, `corpus_staleness.rs`, `calibrate.rs`, `rebaseline_check.rs`, plus shared `corpus_types.rs`).
  - [x] If the xtask crate breaches its budget after this story's additions, **do not** raise the ceiling. Refactor: extract shared TOML / YAML / JSON I/O helpers, fold redundant `Report` shapes, evaluate whether `corpus_types.rs` should live as its own internal crate (`xtask/crates/corpus-types` or similar) — budget increases require invariant-lock review.
   - [x] If the addition prompts a budget review, surface it in the PR description and flag for the Story 0.3 retrospective; do not silently raise the ceiling.

### Review Findings (2026-05-12)

#### Decision Needed

- [x] [Review][Decision] **Phase-order arrays omit `v0.7` and `v0.9` despite architecture document listing them** — AC4 canonical list: `["v0.1-alpha", "v0.1", "v0.3", "v0.5", "v0.8", "v1.0", "v1.5", "v2.0+"]`. Architecture §13 lists `v0.7` (maturity gates) and `v0.9` (Founder Loop). The story doc line 221 says "cross-check the canonical order with the architecture file before committing" — the committed arrays skip them. Must decide: add them now or defer to architecture amendment.
- [x] [Review][Decision] **`calibrate-per-commit` job not wired into `aggregate`'s `needs:` or PR-comment results table** — The job exists in `discipline.yml` but is decoupled from aggregate pass/fail reporting (not in `needs:` array, not in gate_results JSON table). The spec Task 7 says to wire it in. At v0.1-alpha the job skips (no corpus), but when Story 0.4 lands the corpus the gate becomes live and must be visible. Must decide: wire now or defer to Story 0.4.

#### Patch

- [x] [Review][Patch] **`wilson_ci` returns default interval `(0.0, 1.0)` instead of `Err` for `successes > n`** [xtask/src/calibrate.rs] — Spec explicitly says "return `Err` not `panic!`" for `successes > n`. Implementation silently returns a full-uncertainty interval, swallowing impossible-state inputs.
- [x] [Review][Patch] **Production code panics via `unwrap()` on `fs::read_dir` in `check_judge_config.rs`** [xtask/src/check_judge_config.rs] — `fs::read_dir(crates_tests).unwrap_or_else(|_| panic!("cannot read crates dir"))` and `entry.unwrap()`. Should return structured error instead of panicking in CI tooling.
- [x] [Review][Patch] **Multiple `serde_json::to_string_pretty(&report).unwrap()` calls panic on NaN** [xtask/src/calibrate.rs, check_corpus.rs, check_judge_config.rs, coverage_matrix.rs, corpus_staleness.rs, rebaseline_check.rs] — If any report field contains NaN (possible from Wilson CI with degenerate inputs), these panics in production `--json` paths.
- [x] [Review][Patch] **Integration test CWD inconsistency across 3 of 4 integration files** [xtask/tests/check_corpus_integration.rs, xtask/tests/corpus_staleness_integration.rs, xtask/tests/coverage_matrix_integration.rs] — `check_judge_config_integration.rs` uses `--manifest-path` to fix CWD, but other three files use bare `cargo run -p xtask`. Apply the fix uniformly.
- [x] [Review][Patch] **`Violation::Display` uses fragile string-split on `"|"` in `check_corpus.rs`** [xtask/src/check_corpus.rs] — `self.detail.split("|").next().unwrap_or("?")` parses its own detail field back apart. Use structured fields instead.
- [x] [Review][Patch] **`DirectCallVisitor::skip_xtask` uses fragile `starts_with("xtask/")` instead of `Path` API** [xtask/src/check_judge_config.rs] — `file.display().to_string().starts_with("xtask/")` fails for absolute paths or non-workspace-root CWDs.
- [x] [Review][Patch] **`calibrate_corpus` conflicting `ci_width` for empty set: 0.0 vs wilson_ci's 1.0** [xtask/src/calibrate.rs] — When corpus absent from manifest, function returns `ci_width: 0.0` directly, but `wilson_ci(0, 0, z)` returns `(0.0, 1.0)` → width 1.0. Two different empty-set representations.
- [x] [Review][Patch] **`corpus_staleness.rs` treats malformed dates as "Expired" rather than reporting parse error** [xtask/src/corpus_staleness.rs] — `NaiveDate::parse_from_str` failure returns `DateCheck::Expired`. A typo in `valid_until` produces a stale-violation message indistinguishable from genuine expiry. AC5 requires rejecting with format-specific error.
- [x] [Review][Patch] **`coverage_matrix.rs` does not validate `mode` field values** [xtask/src/coverage_matrix.rs] — Only `mode == "hard"` triggers non-zero exit. Any other value (e.g. `"hardd"` typo) silently falls through to exit-zero. Should validate `mode ∈ {"warning", "hard"}`.
- [x] [Review][Patch] **`check_judge_config.rs` `.unwrap()` on `Option` fields in `Display`** [xtask/src/check_judge_config.rs] — `self.file.as_ref().unwrap()` and `self.line.unwrap()` in Display impl for `direct_call` violations. A future refactor with `None` fields would panic during test assertions or CI logging.
- [x] [Review][Patch] **`check_corpus.rs` `item_count` in manifest never validated against actual JSONL line count** [xtask/src/check_corpus.rs] — AC1 defines `item_count = <n>` in the manifest schema. Manifest could claim `item_count = 5` for a 1000-line corpus and gate would pass silently.
- [x] [Review][Patch] **`check_corpus.rs` `--register` does not validate JSON lines** [xtask/src/check_corpus.rs] — Register mode computes SHA-256 but doesn't check per-line JSON. Developer could register malformed JSONL, causing subsequent `check-corpus` to pass hash check but emit `malformed` violations.
- [x] [Review][Patch] **`corpus_staleness.rs` `warn_window_days` can be negative without validation** [xtask/src/corpus_staleness.rs] — Negative `--warn-window-days=-5` silently suppresses all warnings. Should validate CLI argument range.
- [x] [Review][Patch] **`coverage_matrix.rs` `phase_config.phase_order` loaded from TOML but never used** [xtask/src/coverage_matrix.rs] — Only `current_phase` is compared. If TOML's `phase_order` drifts from YAML's, the stale YAML order is used for phase comparison with no detection.
- [x] [Review][Patch] **`rebaseline_check.rs` judge errors silently counted as disagreements** [xtask/src/rebaseline_check.rs] — `judge.judge(&val, &expected).unwrap_or(false)` treats all `Err` returns as disagreements. When Story 1b.4 plugs in real JudgeRunner, transient HTTP failures would inflate disagreement count.
- [x] [Review][Patch] **`rebaseline_check.rs` missing corpus file masked as low agreement ratio** [xtask/src/rebaseline_check.rs] — When a corpus with `judge_id` is in manifest but JSONL file missing, reports `agreement_ratio: 0.0` with no diagnostic about the absent file. Root cause invisible to operator.
- [x] [Review][Patch] **`corpus-rebaseline.yml` uncaught `JSON.parse` / `readFileSync` failures** [.github/workflows/corpus-rebaseline.yml] — If `cargo run` fails, report file may be absent or malformed. `fs.readFileSync` throws on missing files, `JSON.parse` throws on malformed JSON. No try/catch. Job fails with opaque Node.js error.
- [x] [Review][Patch] **`corpus-rebaseline.yml` non-numeric `pr-number` → `NaN`** [.github/workflows/corpus-rebaseline.yml] — `parseInt('${{ github.event.inputs.pr-number }}', 10)` returns `NaN` for non-numeric input. Guard only checks for empty string, not numeric validity.
- [x] [Review][Patch] **Gate name mismatch: `corpus-rebaseline` in coverage-matrix.yaml vs `rebaseline-check` in gate-registry.toml** [tests/coverage-matrix.yaml, xtask/gate-registry.toml] — AC8 row `NFR-Test-1` lists `gates: [..., "corpus-rebaseline"]` but gate-registry.toml lists `"rebaseline-check"`. The coverage-matrix gate would flag its own row as a dangling reference.
- [x] [Review][Patch] **`calibrate.rs` unknown confidence `p` defaults to z=1.96 without warning** [xtask/src/calibrate.rs] — `z_for_confidence` returns 1.96 for any `p` not matching 0.90, 0.95, or 0.99. Should warn or reject unsupported confidence levels.
- [x] [Review][Patch] **`corpus_types.rs` no field-level TOML error reporting on deserialization failure** [xtask/src/corpus_types.rs] — All `CorpusEntry` fields required except `judge_id`. Missing field produces generic serde error, not spec-mandated per-field message.
- [x] [Review][Patch] **`corpus_types.rs:seed` typed as `u64` (non-optional) causes generic TOML error instead of spec-mandated per-row message** [xtask/src/corpus_types.rs, xtask/src/check_judge_config.rs] — Spec requires `NFR-Test-1 violation: judge '<name>' missing seed`. But `seed: u64` causes serde deserialization to fail with generic `"missing field 'seed'"` before the validation loop executes. Make `seed: Option<u64>` or add pre-parse validation.
- [x] [Review][Patch] **Malformed corpus violation stores raw line content, not serde error message (AC1)** [xtask/src/check_corpus.rs] — Spec requires `<serde error>` in output. Implementation captures `line` text instead of serde's parse error string.
- [x] [Review][Patch] **`rebaseline-check` violation message uses dynamic threshold instead of hardcoded `0.98` (AC3)** [xtask/src/rebaseline_check.rs] — Spec requires literal prose `"below quarterly threshold 0.98"`. Implementation interpolates `{}` with threshold parameter, meaning `--threshold 0.95` would produce non-spec-matching message.
- [x] [Review][Patch] **`OfflineMode::judge` compares full item vs `expected_judgment` sub-field — always fails for items with extra fields** [xtask/src/rebaseline_check.rs] — Compares full JSONL item against `expected_judgment` sub-field. For any item with fields beyond `expected_judgment`, these will never be equal. When Story 0.4 lands real corpora with `expected_judgment` embedded in items, OfflineMode returns false for all items.

#### Defer

- [x] [Review][Defer] **`serde_yaml` 0.9.34 is explicitly deprecated** — Spec acknowledges this (line 410) and defers migration to a maintained YAML crate as a future concern. The crate works for the current contract.
- [x] [Review][Defer] **`calibrate.rs` `successes = n` hardcoded placeholder** [xtask/src/calibrate.rs] — At v0.1-alpha with no real corpora this is intentional scaffolding. Story 0.4 lands the first corpus and must replace this with actual pass/fail data from `expected_judgment` comparison.

### Why this story is unusual

This story ships **the meta-test layer**: the gates here are how every future story proves it is actually tested. Story 0.4 ships the first JSONL that this story's verifier hashes. Story 1b.4 ships the first judge call that this story's contract pins. Story 4.4 ships the first distillation-recall corpus that this story's calibration math measures. The dev agent's instinct may be to populate this story with content (write the calibration corpus inline, add five sample JSONLs, write a real judge wrapper) — resist. **This story ships mechanism, not content.** The empty-set must be a valid input across every gate; downstream stories add rows.

This story is also **the schema-lockdown moment** for `tests/coverage-matrix.yaml`. Story 0.2 explicitly committed a forward-compatible draft on the understanding that 0.3 would lock the schema. Every downstream story (0.4 through 10.x) reads this schema; once locked, **schema-breaking changes are an architecture amendment** (invariant-lock review). The dev agent owns getting the schema right the first time: the four top-level fields, the seven row fields, the phase-order array, the mode toggle. There is no v2 schema migration in scope for this story.

### Relevant architecture patterns and constraints

- **PRD § non-functional-requirements / NFR-Test-1 (amended-Murat)** — verbatim source for the SHA-256-of-JSONL / pinned-model / `temperature=0` / `top_p=1.0` / `seed` / retry-budget-1 / prompt-version-hash / quarterly re-baseline ≥98% contract. Every clause in AC1–AC3 traces back to a clause here.
- **PRD § non-functional-requirements / NFR-Meta-2 (corpus-staleness)** — verbatim source for the `valid_until` + 12-month default + "no-update justification" PR + assessor sign-off discipline. AC5 is the mechanical realization.
- **PRD § non-functional-requirements / NFR-Meta-3 (coverage matrix)** — verbatim source for `tests/coverage-matrix.yaml` mapping `{FR, NFR} → {corpora, gates}` + deferred-label semantics + v1.0 100% floor. AC4 + AC8 are the realization.
- **PRD § non-functional-requirements / NFR-Aud-8 (calibration two-tier)** — verbatim source for N=100 per-commit (CI-width ≈ 0.124 sufficient for trend detection) + N=500 quarterly (CI-width ≤ 0.05 at p=0.90 for digest-recall). AC6 implements this honestly — Wilson interval (tighter than Wald) makes the contract more strictly honored than the prose alone.
- **Architecture §appendix-f distillation-pattern-body (corpus size derivation paragraph)** — the only architecture place that explains *why* corpora sizes are what they are (judge-LLM noise floors, IAA convergence). This story's calibration math is the operational complement.
- **Architecture §6 (foundational commitments) commitment #8** — "Constitutional governance is structural, not procedural. Amendments touching invariants I1–I14 require the `invariant-lock` CI gate (machine-checkable diff + corpus delta + phase-commitment update)." The `corpus delta` half is what `tests/coverage-matrix.yaml` measures; this story is what makes that half mechanical.
- **Architecture §12 ADR-037 (constitutional amendment process)** — same three-leg tri-requirement Story 0.1 ships. The coverage-matrix corpus-delta check this story enforces is what Story 0.1's `invariant-lock` xtask reads.
- **Epic 0 "Owns (continuous CI gates)" enumeration** — lines 12–13 of `epic-0-quality-substrate-...md` quote the four discipline lines (NFR-Test-1 / NFR-Meta-3 / NFR-Meta-1 / NFR-Meta-2) verbatim. NFR-Meta-1 (corpus-quality audit ≥8/10) is **operator ceremony, not xtask logic** at v0.1-α — Story 0.5 or a later epic owns mechanizing audit-rubric tracking; AC5's documentary discipline file (`docs/corpus-extensions/README.md`) anchors the ceremony.
- **Story 0.1's `check_unsafe.rs` + `abi_diff.rs` + `kloc_check.rs` are the canonical xtask patterns** — `Visit`-based AST walk (for the AC2 direct-call scan), TOML-driven config (`kloc.toml` → `MANIFEST.toml`, `judge-config.toml`, etc.), `serde`-roundtrip JSON reports, integration-test fixture-tree convention, `ALLOWED: &[&str] = &[]` discipline. Mirror; do not innovate.
- **Story 0.2's `check_empty_kernel.rs` + `check_loom.rs` + `check_service_boundary.rs`** — direct precedents for "lint with config files + denylist + integration test against fixture trees + integration into `discipline.yml`'s aggregate job." This story extends the same shape four more times.
- **Story 0.2's coverage-matrix entries** — must remain bit-exact under the new schema (AC8 verbatim-preservation requirement). The new top-level fields are additive; the row schema gains `valid_until` only.
- **Architecture §13 phased-roadmap matrix** — the `phase_order` array in `tests/phase-config.toml` IS this matrix's left column. `phase_order = ["v0.1-alpha", "v0.1", "v0.3", "v0.5", "v0.8", "v0.7" (?), "v0.9" (?), "v1.0", "v1.5", "v2.0+"]` — note that §13 introduces `v0.7` (maturity gates) and `v0.9` (Founder Loop) as intermediate stops; cross-check the canonical order with the architecture file before committing. Architecture is authoritative; the PRD's `## Phased Development` block may lag behind.
- **NFR-Maint-1 / `xtask/kloc.toml`** — the xtask crate's 3000-LOC ceiling is a real constraint. This story is the densest one yet (six new modules). Task 9 owns budget verification; if a refactor is needed to stay under 3000, prefer factoring shared TOML/YAML/JSON I/O helpers over splitting modules per gate.

### Source tree components to touch

This story adds the following structure (repo-root-relative):

```
maos/
├── .github/workflows/
│   ├── discipline.yml                                       # MODIFIED — adds 4 jobs (+ optional calibrate-per-commit) + aggregate (Task 7)
│   └── corpus-rebaseline.yml                                # NEW — scheduled quarterly + workflow_dispatch (Task 6)
├── docs/
│   ├── ci-baselines/
│   │   ├── README.md                                        # MODIFIED — extends founding-sprint baseline table (Task 8)
│   │   └── v0.1-alpha.json                                  # MODIFIED — extends gate_results (Task 8)
│   └── corpus-extensions/
│       └── README.md                                        # NEW — no-update-justification PR flow doc (Task 8)
├── tests/
│   ├── coverage-matrix.yaml                                 # MODIFIED — schema lockdown + 4 new rows (Task 8)
│   ├── phase-config.toml                                    # NEW — current_phase + phase_order (Task 8)
│   ├── judge-config.toml                                    # NEW — empty [judge] table + schema doc (Task 8)
│   └── corpora/
│       └── MANIFEST.toml                                    # NEW — empty [corpus] table + schema doc (Task 8)
└── xtask/
    ├── Cargo.toml                                           # MODIFIED — adds serde_yaml 0.9, chrono 0.4 (no-default-features)
    ├── gate-registry.toml                                   # NEW — 13 canonical gate names (Task 3 / Task 8)
    ├── judge-direct-call-identifiers.toml                   # NEW — 7 provider-call identifiers (Task 2 / Task 8)
    ├── src/
    │   ├── main.rs                                          # MODIFIED — six new Commands variants
    │   ├── corpus_types.rs                                  # NEW — shared CorpusManifest / CoverageMatrixFile / CoverageRow types
    │   ├── check_corpus.rs                                  # NEW (Task 1)
    │   ├── check_judge_config.rs                            # NEW (Task 2)
    │   ├── coverage_matrix.rs                               # NEW (Task 3)
    │   ├── corpus_staleness.rs                              # NEW (Task 4)
    │   ├── calibrate.rs                                     # NEW (Task 5)
    │   └── rebaseline_check.rs                              # NEW (Task 6)
    └── tests/
        ├── check_corpus_integration.rs                      # NEW (Task 1)
        ├── check_judge_config_integration.rs                # NEW (Task 2)
        ├── coverage_matrix_integration.rs                   # NEW (Task 3)
        ├── corpus_staleness_integration.rs                  # NEW (Task 4)
        └── fixtures/
            ├── violation-corpus/{MANIFEST.toml,corpora/...} # NEW — manifest-hash-mismatch
            ├── clean-corpus/...                             # NEW — manifest-hash-match
            ├── violation-judge-config/judge-config.toml     # NEW — temperature=0.5
            ├── clean-judge-config/judge-config.toml         # NEW — well-formed entry
            ├── violation-judge-direct-call/some_test.rs     # NEW — anthropic_messages(...) inside test code
            ├── clean-judge-direct-call/some_test.rs         # NEW — no direct call
            ├── violation-coverage-matrix/...                # NEW — delivered row with empty gates+corpora
            ├── clean-coverage-matrix/...                    # NEW — schema-conformant minimal yaml
            ├── violation-staleness/coverage-matrix.yaml     # NEW — valid_until: "2020-01-01"
            └── clean-staleness/coverage-matrix.yaml         # NEW — valid_until: "2027-05-11"
```

### Testing standards summary

- **Test approach (mirrors Story 0.2):** the gates *are* the tests. Each xtask subcommand carries Rust-level unit tests under `#[cfg(test)] mod tests` plus a sibling `xtask/tests/<subcommand>_integration.rs` that shells out to the binary against the fixture tree. The Story 0.1 / 0.2 fixture-tree convention is non-negotiable; replicate exactly.
- **Coverage:** ≥80% line coverage in each new `xtask/src/*.rs` module measured locally. Coverage is **still not** a CI gate at v0.1-α; that's Story 2.2 / E2's full classifier territory.
- **Determinism:** every xtask subcommand exposes `--json` mode emitting a `serde`-roundtrippable shape. Round-trip tests are mandatory per the Story 0.1 review-findings JSON-format-stability patch precedent.
- **Empty-set discipline:** every gate **must** pass on a workspace with zero rows (empty `[corpus]`, empty `[judge]`, three-row `coverage` yaml from Story 0.2 + four-row addition from this story). The v0.1-α founding sprint baseline IS the empty-set baseline; mechanical tests `empty_manifest_passes`, `empty_judge_config_passes`, `coverage_matrix_passes_on_minimal_yaml`, `corpus_staleness_passes_on_no_corpora` are mandatory.
- **Wall-clock budget:** the four new per-commit jobs share Cargo cache from Story 0.1 / 0.2 jobs; cold-start cost is one-time, warm cost per job ≤30s. Total `discipline.yml` wall-clock remains <5 min on the project's runner class — verify locally and report any regression.
- **Pinned tool versions:** the only new dependencies are `serde_yaml` 0.9 (YAML parsing for coverage-matrix) and `chrono` 0.4 with `default-features = false, features = ["clock", "std"]` (date math for staleness); document the choice in `xtask/Cargo.toml` comment header. `sha2` 0.10 is already in `xtask/Cargo.toml` from Story 0.2 — re-use, do not duplicate via `sha256`/`ring`/`openssl`.

### Project Structure Notes

- **Alignment with Story 0.1 + 0.2 xtask layout:** the six new modules slot alongside existing `check_unsafe.rs`, `check_empty_kernel.rs`, `check_loom.rs`, `check_service_boundary.rs`, `kloc_check.rs`, `abi_diff.rs`, `invariant_lock.rs`. The shared `corpus_types.rs` module factors `CorpusManifest`, `CoverageMatrixFile`, `CoverageRow`, `JudgeConfig` so the four gate modules don't each `use crate::check_corpus::CorpusManifest` (anti-pattern; modules should not import each other's internal types).
- **Schema-lockdown carries forward to Stories 0.4 / 0.5 / 1b.4 / 4.4:** Story 0.4 commits the first `[corpus.calibration-seed-v0.1]` row + per-row `expected_judgment` field convention; Story 1b.4 commits the first `[judge.<name>]` row + plugs `InferencePortJudge` into the `JudgeRunner` trait; Story 4.4 wires the digest-recall corpus into the calibration N=500 quarterly tier. None of those stories should renegotiate the schema; if they need to, the change is an architecture amendment.
- **No production code touched.** This story does not modify any file under `crates/maos-kernel-core/src/` or `crates/maos-spirit-abi/src/`. The corpus-discipline lives in `tests/`, `docs/`, `.github/workflows/`, and `xtask/`. If the dev agent finds itself editing kernel-core source, stop — the gates are about future commits.
- **Detected conflict carried forward from Story 0.1 / 0.2:** services-as-modules at v0.1-α vs services-as-crates at v0.5+. Not relevant to this story's scope — corpus discipline is workspace-cross-cutting and indifferent to crate layout — but documented in case the dev agent wonders why `xtask/Cargo.toml` is the only `Cargo.toml` modified in this story.
- **`docs/corpus-extensions/` is a NEW directory** with only `README.md` in it. The directory anchors the no-update-justification flow that Story 0.5 / E5 / E7 / E10 will populate when real corpora actually expire (24+ months out from this story). Creating the empty directory now (with the README) gives that future PR somewhere to land; it does not commit the dev agent to anything beyond a one-paragraph runbook.

### References

- [Source: planning-artifacts/epics/epic-0-quality-substrate-cross-cutting-founding-sprint-v01-maintenance-track-thereafter.md#Story-0.3] — full BDD acceptance criteria for the corpus + coverage gates (lines 96–134).
- [Source: planning-artifacts/epics/epic-0-quality-substrate-cross-cutting-founding-sprint-v01-maintenance-track-thereafter.md#Owns-continuous-CI-gates] — lines 12–15 enumerate NFR-Test-1 / NFR-Meta-3 / NFR-Meta-1 / NFR-Meta-2 as continuous CI gates. NFR-Meta-1 (corpus-quality audit ≥8/10) is operator ceremony, not xtask logic at v0.1-α.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Test-1] — content-addressed corpora; pinned model; temperature=0; top_p=1.0; seed; retry budget=1; prompt-version hash; quarterly re-baseline ≥98%.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Meta-2] — valid_until + 12-month default + assessor-signed-off no-update-justification PR.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Meta-3] — coverage matrix mapping {FR, NFR} → {corpora, gates}; deferred labelling; v1.0 100% floor.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Aud-8] — two-tier calibration N=100 per-commit / N=500 quarterly with CI-width thresholds.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Sec-4] — 10⁴ per-commit + 10⁵ quarterly secret-leakage corpora; informs Task 1's "streaming SHA-256, do not load fully into memory" decision.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Ops-10] — 10⁶-row migration corpus; same streaming-hash justification.
- [Source: planning-artifacts/architecture-maos-minimal-opus/06-foundational-commitments.md#Commitment-8] — "Constitutional governance is structural" + invariant-lock tri-requirement (diff + corpus delta + phase-commitment).
- [Source: planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-037] — invariant-lock tri-requirement verbatim.
- [Source: planning-artifacts/architecture-maos-minimal-opus/appendix-f-distillation-pattern-body.md#Corpus-size-derivation] — judge-LLM noise floors + IAA convergence; informs calibration math.
- [Source: planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md] — canonical `phase_order` source.
- [Source: implementation-artifacts/0-1-workspace-ci-pipeline-build-discipline-gates.md] — predecessor story; ships `xtask` skeleton + `discipline.yml` + `invariant-lock` gate.
- [Source: implementation-artifacts/0-2-enforce-empty-kernel-invariants-via-structural-ci-lints.md] — direct predecessor; ships `check_empty_kernel.rs` + `check_loom.rs` + `check_service_boundary.rs` + the three coverage-matrix rows this story preserves; AC6 forward-compatible schema-draft is the schema this story locks.
- [Source: implementation-artifacts/0-2-enforce-empty-kernel-invariants-via-structural-ci-lints.md#tests-coverage-matrix.yaml] — Story 0.2's draft (`coverage:` top-level key + per-row `{ gates, corpora, phase }`); this story extends, does not replace.
- [Source: xtask/src/check_unsafe.rs + xtask/src/check_empty_kernel.rs + xtask/src/abi_diff.rs] — canonical xtask patterns (Visit-based AST, TOML config, `Report` shape, JSON round-trip). Mirror; do not innovate.
- [Source: xtask/tests/check_unsafe_integration.rs + xtask/tests/empty_kernel_integration.rs + xtask/tests/fixtures/...] — canonical fixture-tree integration-test pattern. Mirror.
- [Source: xtask/invariants/lock.toml] — I*.md map; this story does **not** touch any I*.md so the invariant-lock gate takes its "no invariants touched" path on this PR.
- [Source: planning-artifacts/epics/dependency-dag.md] — Story 0.3 downstream consumers: 0.4 (calibration seed corpus), 0.5 (parameterized generators), 1b.4 (ComplianceClaim freeze + Inference Port → first judge call), 4.4 (digest-recall five-metric gate), 5.5b (multi-provider CI matrix), 9.4 (secret-redaction canary review), 10.2 (red-team gate).

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m]

### Debug Log References

### Completion Notes List

- 2026-05-12 — Story 0.3 implementation complete. All 9 tasks finished.
- 82 unit tests pass (including kloc budget verification: xtask at 2522 LOC, under 3000 ceiling).
- All integration tests pass: check_corpus (2), check_judge_config (4), coverage_matrix (3), corpus_staleness (2).
- Task 9 required aggressive code compression: extracted shared `load_toml` and `round_ratio` helpers into `corpus_types.rs`, compressed Display impls, folded duplicate violation-push patterns, and compacted `main.rs` dispatch. Net reduction: 655 LOC.
- Fixed `is_hex_64` bug during integration test validation (digits rejected by `is_ascii_lowercase()` — replaced with `matches!(c, '0'..='9' | 'a'..='f')`).
- Fixed integration test `cargo run` CWD issue by adding `--manifest-path` to the cargo command.

### File List

**New xtask modules (7):**
- `xtask/src/corpus_types.rs` — shared types: CorpusManifest, CoverageMatrixFile, JudgeConfig, GateRegistry, etc.
- `xtask/src/check_corpus.rs` — SHA-256 streaming verifier + JSONL parse-check + orphan detection
- `xtask/src/check_judge_config.rs` — pinned-judge-LLM structural contract validator + AST direct-call scan
- `xtask/src/coverage_matrix.rs` — delivered-phase enforcement gate with mode-dependent exit codes
- `xtask/src/corpus_staleness.rs` — valid_until staleness enforcement with warning window
- `xtask/src/calibrate.rs` — Wilson-score CI-width math
- `xtask/src/rebaseline_check.rs` — quarterly rebaseline check with JudgeRunner trait scaffold

**New test files (extracted inline tests + integration tests):**
- `xtask/src/tests/check_corpus_tests.rs`, `check_judge_config_tests.rs`, `coverage_matrix_tests.rs`, `corpus_staleness_tests.rs`, `calibrate_tests.rs`, `rebaseline_check_tests.rs`
- `xtask/tests/check_corpus_integration.rs`, `check_judge_config_integration.rs`, `coverage_matrix_integration.rs`, `corpus_staleness_integration.rs`
- `xtask/tests/fixtures/violation-corpus/`, `clean-corpus/`, `violation-judge-config/`, `clean-judge-config/`, `violation-judge-direct-call/`, `clean-judge-direct-call/`, `violation-coverage-matrix/`, `clean-coverage-matrix/`, `violation-staleness/`, `clean-staleness/`

**New discipline/config files:**
- `tests/corpora/MANIFEST.toml` — empty [corpus] table with schema documentation
- `tests/judge-config.toml` — empty [judge] table with contract documentation
- `tests/phase-config.toml` — current_phase + phase_order single source of truth
- `xtask/gate-registry.toml` — 13 canonical gate names
- `xtask/judge-direct-call-identifiers.toml` — 7 provider-call identifiers

**Modified files:**
- `xtask/src/main.rs` — 6 new Commands variants + dispatch
- `xtask/Cargo.toml` — added serde_yaml 0.9, chrono 0.4 (no-default-features, clock+std)
- `tests/coverage-matrix.yaml` — schema lockdown: 3 Story 0.2 rows preserved + 4 new rows
- `.github/workflows/discipline.yml` — 4 new per-commit jobs + conditional calibrate-per-commit + aggregate extended
- `.github/workflows/corpus-rebaseline.yml` — new scheduled quarterly workflow
- `docs/ci-baselines/README.md` — extended founding-sprint baseline gates table
- `docs/ci-baselines/v0.1-alpha.json` — extended gate_results
- `docs/corpus-extensions/README.md` — no-update-justification PR flow documentation

---

## Developer Context (LLM optimization — read this first)

### Critical anti-patterns to avoid

1. **Do NOT populate corpus content in this story.** No `tests/corpora/calibration-seed-v0.1.jsonl`, no sample JSONLs, no judge configs with real model strings, no coverage rows beyond the four NFRs this story mechanizes. **Mechanism, not content.** Story 0.4 owns calibration seed corpus content. If the dev agent finds itself writing JSONL, stop and re-read the story.
2. **Do NOT modify the three Story 0.2 rows in `tests/coverage-matrix.yaml`.** Their `gates`, `phase`, and (empty) `corpora` fields must remain bit-exact; you may **only** add `valid_until: "2027-05-11"` to each and the top-level fields. AC8's verbatim-preservation requirement is non-negotiable — Story 0.2's CI baseline depends on it.
3. **Do NOT switch `mode: "warning"` to `"hard"` in `tests/coverage-matrix.yaml`.** Story 0.4 AC6 owns the v0.3 transition explicitly. Flipping it early would fail the founding-sprint CI run (`NFR-Test-1`, `NFR-Meta-2`, `NFR-Meta-3`, `NFR-Aud-8` all have `corpora: []` at v0.1-α — that's by design).
4. **Do NOT make live judge-LLM calls anywhere in xtask code.** The `JudgeRunner` trait has exactly one v0.1-α implementation: `OfflineMode` returning `item == expected`. No `reqwest`, no provider clients, no HTTP. Story 1b.4 lands the real implementation behind the trait; this story locks the seam.
5. **Do NOT use `cargo` or any external CLI to compute SHA-256.** Use `sha2::Sha256` (already in `xtask/Cargo.toml` from Story 0.2). Pipe-shelling `sha256sum` is non-portable (BusyBox vs GNU coreutils) and breaks integration tests on macOS runners.
6. **Do NOT auto-write `tests/corpora/MANIFEST.toml`.** The `--register <name>` mode prints the TOML snippet to stdout. The developer pastes it. Auto-writing a SHA-256 manifest entry is a footgun — a typo or wrong file path silently records the wrong hash and the next CI run "passes" against a now-incorrect baseline.
7. **Do NOT load JSONL fully into memory before hashing.** Stream in 64 KiB chunks. NFR-Sec-4 corpora reach 10⁵ items; NFR-Ops-10 reaches 10⁶ rows. A 10⁶-row corpus at 4 KiB per row is ~4 GiB — that's an OOM on the project's runner class.
8. **Do NOT use string-`grep` for the direct-call scan.** `check_judge_config.rs`'s AST scan re-uses `syn::visit::Visit` from `check_loom.rs` / `check_unsafe.rs`. A string-grep for `anthropic_messages` would false-positive on comments, doctests, string literals (`"call anthropic_messages here later"`), and `#[cfg(test)]`-gated test scaffolding.
9. **Do NOT pull in `chrono` with default features.** Default features pull `oldtime` + `serde` integrations not needed at v0.1-α; specify `default-features = false, features = ["clock", "std"]`. This is the same `default-features` discipline Story 0.1 used for `walkdir` (no-default-features unless needed).
10. **Do NOT add `tests/corpora/MANIFEST.toml` rows or `tests/coverage-matrix.yaml` coverage rows beyond the four enumerated in AC8.** The set of rows committed by this story is **exactly four** new yaml rows (`NFR-Test-1`, `NFR-Meta-2`, `NFR-Meta-3`, `NFR-Aud-8`) + three preserved Story 0.2 rows (`I9`, `NFR-Test-2`, `NFR-Test-9`). No FR rows in this story. Mass-population of `FR1..FR65` is Story 0.4 AC4's job.
11. **Do NOT name the new `corpus-rebaseline.yml` jobs with the same job-name as `discipline.yml` jobs.** Job names share the GitHub Actions namespace per workflow file; cross-workflow they don't collide, but cron-vs-PR-comment patterns differ — use `<!-- rebaseline-comment -->` (not `<!-- discipline-gate-comment -->`) as the upsert sentinel for the scheduled workflow's PR comment.
12. **Do NOT raise the `xtask` kloc budget.** Per `xtask/kloc.toml` the ceiling is 3000 LOC. After Story 0.2 the crate is ~2549 LOC; this story adds ~1000–1300 LOC. If the addition breaches 3000, refactor (factor shared types into `corpus_types.rs`; fold near-identical `Report` shapes) — do not raise the ceiling without invariant-lock review.

### Library / framework requirements

| Concern | Tool | Pin | Why |
|---|---|---|---|
| SHA-256 hashing | `sha2` (already in `xtask/Cargo.toml` from Story 0.2) | `0.10` | AC1 streaming hash |
| YAML parsing | `serde_yaml` (**NEW dep**) | `0.9` | AC4 `coverage-matrix.yaml` parse/serialize |
| Date math | `chrono` (**NEW dep**) | `0.4` with `default-features = false, features = ["clock", "std"]` | AC5 `valid_until` comparison |
| TOML parsing | `toml` (already pinned) | `0.8` | manifest / judge-config / phase-config / gate-registry / identifiers — five new TOML files |
| AST parsing | `syn` (already pinned) | `2.x` + `full` + `visit` | AC2 direct-judge-call structural scan |
| JSON I/O | `serde` + `serde_json` (already pinned) | `1.x` | report shapes + JSONL per-line parse-check |
| Filesystem walk | `xtask/src/fs_walk.rs` (factored in Story 0.2) | n/a | AC2 test-file enumeration |
| HTTP / LLM clients | **forbidden in this story** | n/a | judge calls are routed through `JudgeRunner` trait; only Story 1b.4 introduces a real implementation |

Story 0.1 + 0.2 patterns are otherwise unchanged: `quote 1.0`, `proc-macro2 1.x` with `span-locations`, `walkdir 2.5`. No nightly. Rust stable per `rust-toolchain.toml`.

### File structure requirements (must-follow paths)

- **xtask modules:** `xtask/src/check_corpus.rs`, `check_judge_config.rs`, `coverage_matrix.rs`, `corpus_staleness.rs`, `calibrate.rs`, `rebaseline_check.rs`, and shared `corpus_types.rs` — seven new sibling files. Do **not** create subdirectories like `xtask/src/corpus/` — Story 0.1 / 0.2 keep modules flat under `src/`.
- **xtask config files (flat schema, root-level keys are values or single-level tables):** `xtask/gate-registry.toml`, `xtask/judge-direct-call-identifiers.toml` — follow `xtask/kloc.toml` / `xtask/loom-blocklist.toml` convention.
- **tests-side discipline files:** `tests/coverage-matrix.yaml` (UPGRADED), `tests/phase-config.toml` (NEW), `tests/judge-config.toml` (NEW), `tests/corpora/MANIFEST.toml` (NEW). The `tests/corpora/` directory is created by this story even though it holds no JSONL yet — `MANIFEST.toml`'s parent must exist for AC1's path-existence checks to work coherently.
- **docs:** `docs/corpus-extensions/README.md` (NEW); `docs/ci-baselines/README.md` (MODIFIED — extend Founding-sprint baseline gates table); `docs/ci-baselines/v0.1-alpha.json` (MODIFIED — extend gate_results).
- **workflows:** `.github/workflows/discipline.yml` (MODIFIED — four new sibling jobs + optional `calibrate-per-commit` + extend `aggregate`); `.github/workflows/corpus-rebaseline.yml` (NEW — schedule + workflow_dispatch).
- **integration tests:** four new files under `xtask/tests/` (one per per-commit gate) + ten fixture trees under `xtask/tests/fixtures/`.

### Latest technical information

- **`serde_yaml` 0.9 (May 2026):** The crate is in low-maintenance mode (last release ~2024); the `serde_yml` fork exists but has not stabilized. `serde_yaml` 0.9 remains the de-facto Rust YAML 1.2 parser. Inherit the same posture as `quote!`-based ABI hashing in Story 0.1 (`abi_diff.rs`): document a TODO comment that migration to a maintained YAML crate is a future concern, but the crate works for the current contract. Do not be tempted by `serde_yml` — the fork's API parity is not yet verified across all our serialization paths.
- **`chrono::NaiveDate::parse_from_str` (May 2026):** Stable for `%Y-%m-%d` (ISO 8601 date-only). Use this exact format string in `tests/coverage-matrix.yaml`'s `valid_until` fields and `tests/corpora/MANIFEST.toml`'s `valid_until` fields. Reject any other format with `NFR-Meta-2 violation: <id> valid_until "<value>" not in YYYY-MM-DD format`.
- **`sha2::Sha256` streaming pattern (May 2026):** Use `Sha256::new()` + `update(&chunk)` in a loop + `finalize()` → `[u8; 32]` → hex-encode via `format!("{:x}")` on each byte or via a helper. Story 0.2 added the dependency for `check_service_boundary.rs`'s signature hashing; the same idiom carries over.
- **`syn::visit::Visit` AST traversal pattern (May 2026):** Unchanged from Story 0.2's `check_loom.rs`. `visit_expr_call(&mut self, node: &ExprCall)` is the hook for `func(args...)`; walk `node.func` as `Expr::Path` and inspect `path.segments.last().unwrap().ident` for the call name.
- **GitHub Actions `schedule:` cron syntax (May 2026):** Standard 5-field cron in UTC. `'0 14 1 */3 *'` = 14:00 UTC on the 1st of every third month (January, April, July, October). Document in the workflow file that GitHub may delay scheduled runs under load — the quarterly cadence is "approximately quarterly," not millisecond-precise; this is acceptable per NFR-Test-1 which says "quarterly re-baseline" not "exactly every 90 days."
- **Wilson score interval formula (textbook, Agresti & Coull 1998):** Given `p_hat = successes/n` and z-score `z`, the interval is `(p_hat + z²/(2n) ± z·√(p_hat·(1-p_hat)/n + z²/(4n²))) / (1 + z²/n)`. Implement exactly this; do not paraphrase. Z-table values: `z(0.90) = 1.6449`, `z(0.95) = 1.96`, `z(0.99) = 2.5758`.

### Project-context reference

There is still no `project-context.md` in this repository (verified at story-creation time — `find /home/lunarpulse/dev_ws/maos -name project-context.md` returns no matches). The persistent-facts entry `file:{project-root}/**/project-context.md` resolves to an empty set; this is expected at the founding sprint. Treat the architecture document (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`) and PRD (`_bmad-output/planning-artifacts/prd/`) as the canonical context, exactly as Stories 0.1 and 0.2 did.

---

## Change Log

- 2026-05-12 — Story 0.3 created. Mechanizes NFR-Test-1 (content-addressed corpora + judge contract + quarterly rebaseline), NFR-Meta-2 (valid_until), NFR-Meta-3 (coverage matrix gate), and NFR-Aud-8 (calibration Wilson-CI math). Locks the `tests/coverage-matrix.yaml` schema (extends Story 0.2's forward-compatible draft non-destructively).

## Story Completion Status

Status: **done**.

Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.
