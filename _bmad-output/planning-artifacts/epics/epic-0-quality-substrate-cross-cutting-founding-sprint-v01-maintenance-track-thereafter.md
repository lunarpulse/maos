# Epic 0: Quality Substrate (cross-cutting; founding sprint v0.1; maintenance track thereafter)

**Goal:** Every CI gate green from day one. Without E0, every subsequent epic's gated NFR is a check against a bank that doesn't exist. This is the substrate-of-the-substrate.

**Owns (continuous CI gates):**
- `cargo xtask check-service-boundary` P1–P4 stub (full implementation in E2) — kernel-API surface invariant (NFR-Test-2): build-time reflection classifies every kernel API as universal-arithmetic / data-movement / supervision / **other**; new function in "other" class is build-break.
- Empty-kernel invariant I9 (ADR-006) — structural lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}`.
- Loom-not-in-kernel grep (NFR-Test-9) — `grep` of kernel crate for orchestration/planning symbols returns ∅.
- KLOC budget enforcement (`tokei`, `xtask/kloc.toml`, aggregate ≤20 KLOC, alarm at 16) — NFR-Maint-1.
- Reproducible build gate (`cargo build --locked` on Rust stable; no nightly).
- Zero-`unsafe` in kernel capability-validation path (NFR-Sec-9 — gate from day one).
- Content-addressed corpora infrastructure (NFR-Test-1: SHA-256 of JSONL, pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash, retry budget=1, quarterly re-baseline ≥98% on golden snapshot).
- Coverage matrix CI gate (NFR-Meta-3: `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}; CI fails if any delivered FR/NFR has zero corpus).
- Corpus-quality audit rubric (NFR-Meta-1: ≥8/10 per corpus, 12-month re-audit).
- Corpus-staleness `valid_until` enforcement (NFR-Meta-2: CI fails if any active gate references an expired corpus; default validity 12 months).
- Invariant-lock CI gate (ADR-037) on every PR touching I1–I14.
- ABI-diff lint (per architecture-minimal-opus §CI-gates).
- Calibration harness infrastructure (NFR-Aud-8: N=100 per-commit pipeline + N=500 quarterly audit runner — corpus content authored per-epic).
- **ComplianceClaim schema adversarial review** before E1b freezes (Mary + Winston joint demand).

**Corpora authored in E0:**
- Calibration seed corpus N=100 (NFR-Aud-8 per-commit slice).
- Coverage matrix skeleton: 0-item rows for every FR + NFR, populated by owning epics.

**v0.1 founding-sprint acceptance:** CI pipeline green on empty workspace; coverage matrix template populated; corpus harness operational; calibration seed corpus committed; ComplianceClaim schema adversarial review report signed off; PR adding a persistent field outside I9 whitelist is rejected by CI.

**No FRs.** Cross-cutting infrastructure that gates every subsequent epic.

### Stories

## Story 0.1: Workspace CI Pipeline + Build Discipline Gates

As a maintainer of MAOS,
I want every PR to be gated by build-discipline checks (reproducible build, zero-`unsafe` in capability-validation path, KLOC budget alarm, ABI-diff lint, invariant-lock CI gate),
So that architectural commitments cannot erode silently between v0.1 and v2.0.

**Acceptance Criteria:**

**Given** a fresh checkout of the MAOS workspace
**When** `cargo build --locked` runs on Rust stable
**Then** the build produces a reproducible artifact
**And** the build fails if any nightly feature is referenced

**Given** a PR that introduces `unsafe { … }` anywhere in `maos-kernel-core/capability/`
**When** CI runs `cargo xtask check-unsafe`
**Then** the PR is rejected with `NFR-Sec-9 violation: zero-unsafe gate failed in capability-validation path`

**Given** the workspace exceeds 16 KLOC of kernel trusted core measured by `tokei` per `xtask/kloc.toml`
**When** CI runs the KLOC budget check
**Then** a warning alarm fires labelled `NFR-Maint-1 alarm — 16 KLOC threshold reached`
**And** the build hard-fails at 20 KLOC aggregate

**Given** a PR that changes the public ABI surface of `maos-spirit-abi`
**When** CI runs the ABI-diff lint
**Then** the diff is annotated against the previous tagged ABI version
**And** the lint enforces the ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` rules

**Given** a PR that touches any of the 14 invariants I1–I14 in `maos-domain`
**When** CI runs the `invariant-lock` job (ADR-037)
**Then** the PR is blocked unless ≥2 maintainer sign-offs are present on the lock-edit commit
**And** the journal records the invariant-lock decision

**Given** the founding-sprint acceptance for E0
**When** CI runs on an empty workspace (no production code yet)
**Then** every build-discipline gate is green
**And** the green run is committed as the v0.1-α CI baseline

## Story 0.2: Enforce Empty-Kernel Invariants via Structural CI Lints

As a MAOS architect,
I want structural lints that block kernel growth in ways that would violate the empty-kernel invariant (I9, ADR-006), smuggle orchestration logic into the kernel (NFR-Test-9), or add functions outside the permitted computational classes (NFR-Test-2),
So that the kernel-as-substrate commitment is mechanically enforced at PR-merge time, not merely a code-review aspiration.

**Acceptance Criteria:**

**Given** a PR that adds a persistent struct field outside the three sanctioned locations (`Journal`, `TransparencyLog`, `CapabilityRegistry::tokens`)
**When** CI runs the I9 structural lint
**Then** the PR is rejected with `I9 violation: persistent field <field_name> not in I9 whitelist`

**Given** the MAOS kernel crate (`maos-kernel-core/`)
**When** CI runs `grep` for orchestration/planning symbols (`Loom`, `Planner`, `Goal`, `Orchestrator` types in the kernel crate)
**Then** the grep result returns ∅
**And** the PR is rejected with `NFR-Test-9 violation: Loom-not-in-kernel grep matched <symbol>` if any match is found

**Given** a PR that adds a new public kernel API function exported via `kernel::api::*`
**When** the `cargo xtask check-service-boundary` job classifies the function via Rust `syn` static analysis
**Then** the function MUST be classified as one of: `universal-arithmetic`, `data-movement`, `supervision`
**And** the build hard-fails if the function falls into class `other`
**And** the violation surfaces with NFR-Test-2 reference

**Given** the I9 / NFR-Test-2 / NFR-Test-9 gates are wired
**When** an attempt PR is opened that deliberately violates each gate
**Then** all three gates fail independently
**And** the failure messages are actionable (include the offending file, line, and rule citation)

## Story 0.3: Content-Addressed Corpora Infrastructure + Coverage Matrix CI Gate

As a test architect,
I want the content-addressed corpus harness (SHA-256-pinned JSONL, pinned model versions, temperature=0 judge calls, deterministic retry budget, quarterly re-baseline pipeline) AND the `tests/coverage-matrix.yaml` CI gate to ship together,
So that every gated NFR has a measurable corpus and CI fails the moment any delivered FR/NFR has zero corpus coverage.

**Acceptance Criteria:**

**Given** a corpus JSONL file committed to `tests/corpora/`
**When** the corpus is loaded by any CI gate
**Then** the load verifies the corpus's SHA-256 against the committed manifest
**And** mismatches fail the build with `NFR-Test-1 violation: corpus integrity broken`

**Given** a judge-LLM call inside any test gate
**When** the call is dispatched
**Then** the model version is pinned to a fixed identifier
**And** `temperature=0, top_p=1.0, seed` are set where the provider supports them
**And** the retry budget is exactly 1
**And** the prompt-version hash is committed alongside the corpus

**Given** the quarterly re-baseline pipeline
**When** the runner re-executes all gated corpora against pinned models
**Then** agreement with the golden snapshot is ≥98%
**And** any deviation triggers a re-baseline review issue (NFR-Test-1)

**Given** `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}
**When** CI runs the coverage-matrix gate (NFR-Meta-3)
**Then** the build fails if any FR or NFR with phase-status `delivered ≤ current-phase` has zero corpus rows

**Given** a corpus row in `coverage-matrix.yaml`
**When** the corpus's `valid_until` date is in the past
**Then** CI fails with `NFR-Meta-2 violation: corpus expired <date>; either extend or rebuild`
**And** an explicit no-update justification PR with assessor sign-off is required to extend

**Given** the calibration harness
**When** the N=100 per-commit pipeline runs
**Then** the pipeline emits CI-width ≈ 0.124 (sufficient for trend detection per NFR-Aud-8)
**And** the quarterly N=500 audit pipeline emits CI-width ≤ 0.05 at p=0.90 for digest-recall

## Story 0.4: ComplianceClaim Schema Adversarial Review + Calibration Seed Corpus

As a substrate-of-the-substrate maintainer,
I want the ComplianceClaim schema adversarially reviewed before E1b freezes it, AND the v0.1 calibration seed corpus N=100 committed alongside the coverage-matrix template,
So that the schema's binding-v0.1 ABI commitment is not built on shaky ground and the corpus discipline runs from day one.

**Acceptance Criteria:**

**Given** the ComplianceClaim schema draft from `maos-spirit-abi/src/compliance.rs`
**When** the adversarial review panel (≥2 reviewers external to the schema author) examines the schema
**Then** the panel produces a signed-off review report in `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`
**And** the report enumerates each field's `secret`/`non-secret` classification (NFR-Sec-16)
**And** the report explicitly checks context-drift attack surfaces (manifest hash, version, trust tier, sandbox tier, capability scope, provider-endpoint, crypto-provider)

**Given** the review report is signed off
**When** E1b moves to freeze the schema
**Then** the schema's `ABI_VERSION` is committed and the freeze event is journaled

**Given** the calibration seed corpus N=100 (clearly-decidable bucket, distributed across categories per NFR-Aud-8)
**When** the corpus is committed to `tests/corpora/calibration-seed-v0.1.jsonl`
**Then** the corpus is SHA-256-pinned per Story 0.3
**And** the corpus is registered in `tests/coverage-matrix.yaml`
**And** the corpus carries a `valid_until` date 12 months out

**Given** the coverage-matrix template
**When** initial population occurs
**Then** every FR (FR1–FR65) and every NFR has at least a 0-item row in `coverage-matrix.yaml`
**And** the gate runs in warning-only mode for v0.1 founding sprint before becoming a hard gate at v0.3

## Story 0.5: Parameterized Corpus Generators — Secret-Redaction + Red-Team Frameworks

As the test-architecture lead facing ~2,249 hand-authored corpus items across the v1.0 + v1.5 ship gates,
I want two parameterized generator frameworks committed early: `crates/maos-corpus-gen/src/secret_redaction/` (produces the 10⁴ per-commit + 10⁵ quarterly secret-leakage corpora from ~200 seed patterns) AND `crates/maos-corpus-gen/src/red_team/` (produces the 640-item adversarial-Spirit red-team corpus from 80 canonical scenarios across 8 attack classes),
So that scheduling fictions (hand-authoring 10,000+ items) collapse to engineering artifacts — generator + seed + expansion rules — and downstream Stories 6.x / 10.2 / 10.3 can consume large corpora without inventing them at gate time.

**Acceptance Criteria:**

**Given** the `crates/maos-corpus-gen/` workspace crate
**When** the crate is compiled
**Then** the crate exposes a `CorpusGenerator` trait declared in `crates/maos-corpus-gen/src/lib.rs` with methods: `seed_corpus()`, `expand(n: usize)`, `validate(item: &Item) -> ValidationOutcome`, `coverage_report() -> CoverageReport`
**And** generator output is deterministic given a seed file SHA and an expansion-rule version
**And** generator output is SHA-256-pinned per Story 0.3's corpus discipline

**Given** the secret-redaction generator (`crates/maos-corpus-gen/src/secret_redaction/`)
**When** the generator runs with ~200 seed patterns covering all secret classes (API keys / OAuth tokens / private keys / database URLs / JWT / AWS / GCP / Azure / SSH / GPG)
**Then** the per-commit run produces 10⁴ deduplicated items in `tests/corpora/secret-redaction-1e4-<sha>.jsonl` (NFR-Sec-4)
**And** the quarterly run produces 10⁵ items via wider parameter sweep
**And** the 1000-canary-per-month production canary corpus is produced independently with cryptographic markers (NFR-Sec-4 floor)
**And** any expansion rule that produces a false negative (i.e., a real secret missed by the redactor) is a P0 ship-block

**Given** the red-team generator (`crates/maos-corpus-gen/src/red_team/`)
**When** the generator runs with 80 canonical seed scenarios across 8 attack classes (capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse — N=10 per class)
**Then** the expansion produces ≥640 deduplicated items in `tests/corpora/red-team-640-<sha>.jsonl` (NFR-Sec-10)
**And** the per-class floor is ≥80 items after expansion (8× from N=10 seed)
**And** deduplication preserves coverage: every seed scenario appears in expanded form

**Given** the generator coverage report
**When** `cargo run -p maos-corpus-gen -- coverage --corpus <name>` runs
**Then** the report shows attack-class coverage, parameter-space coverage, and any unexpanded seed slots
**And** the report is consumed by Story 10.2 (red-team gate) and Story 9.4 (secret-redaction operational canary review)

**Given** the v0.5 readiness handoff
**When** Story 5.5b (multi-provider CI) needs secret-redaction tests in CI
**Then** the 10⁴ per-commit corpus is already available
**And** Story 6.x ConsentRupture testing has the red-team generator available for adversarial fixtures

---
