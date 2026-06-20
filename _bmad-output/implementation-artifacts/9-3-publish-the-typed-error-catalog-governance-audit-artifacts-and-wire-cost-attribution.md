---
dev_model_used: claude-opus-4-6
recommended_dev_model: claude-opus-4-8
---

# Story 9.3: Publish the Typed Error Catalog (FR63)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- ⚑ SPLIT at preflight (party-mode 2026-06-13, Winston·Murat·John·Amelia). The original 9.3 bundled FR63+FR62+FR64.
     Split on the kernel-impact seam (mirrors 9.2→9.2b): THIS story = FR63 (typed error catalog — kernel-neutral,
     unblocked, lands first, feeds Story 9.5's troubleshooting-guide 100%-coverage gate). Story 9.3b = FR62 + FR64
     (governance + cost — the kernel-touching half carrying both blockers). See 9-3b-*.md. -->

> **⚑ ORIGIN.** Split from the original 9.3 by unanimous party-mode preflight (2026-06-13). FR63 is **kernel-neutral, unblocked, and broad but mechanical**; it was chained in the epic to two blocked concerns (frozen-ComplianceClaim, cost-reconciliation oracle) that now live in **Story 9.3b**. Keeping FR63 standalone protects Story 9.5's critical path (its troubleshooting guide must cover 100% of this catalog).

## Story

As an enterprise operator,
I want every kernel-emitted error to carry a stable typed code from a published catalog with CI-enforced metadata (FR63 / NFR-Doc-2),
so that errors are diagnosable, version-stable across an LTS cycle, and the catalog cannot silently drift from the code.

---

## Context & Charter Boundary (READ FIRST)

- **Zero kernel-core KLOC.** This is purely additive metadata + an `xtask` check + a generator. `git diff -- crates/maos-kernel-core/src/ --stat` must be **empty**; `check-kernel-baseline` stays green at **21336**. If you find yourself editing kernel src, STOP — you are in 9.3b's lane.
- **Workspace stays 44 crates.** The catalog source-of-truth is a **TOML registry** + an `xtask` check, NOT a new `maos-errors` crate (preflight F1 rejected the new-crate option).
- **`maos-audit` read-only, `maos-cli` kernel-core-free, `abi-diff` Added-only** — all stay green (FR63 touches none of them).
- CI checks are **`xtask` subcommands** (`cargo xtask <check>`, module under `xtask/src/`), NOT standalone `[[bin]]`s. The epic's literal `cargo run --bin error-metadata-check` is **errata'd** to `cargo xtask error-catalog-check` (preflight F3).

**§A6 note.** FR63 is kernel-neutral and mechanical — NOT a correctness-critical category by itself. Standard review suffices. (The §A6 mandate attaches to 9.3b, not here.) Recommended dev model: `claude-opus-4-8`, but a competent non-Opus dev is acceptable for FR63 with normal review.

---

## Preflight Consensus (party-mode 2026-06-13 — DECISIONS, not options)

These were ratified 4/4 (Winston · Murat · John · Amelia). Implement them; do not re-litigate.

- **F1 — Source-of-truth = `xtask/error-catalog.toml` registry, cross-checked against the live enums.** Rejected: (a) doc-comment annotations (parsing `syn` across 80+ enums is format-drift bait, not PR-reviewable), (c) a new `maos-errors` crate (forces migrating 80+ enums for a metadata benefit; → 45 crates). The TOML registry is version-controlled data; the enums are code; **CI asserts the bijection.**
- **F2 — Lock the SET, not the count.** There is no canonical "14+" list anywhere — the number was never the point. v1.0 scope = the **kernel-emitted `E*` invariant error set** (the named-`E…` family). Cardinality is an **output**, reported by the check; "14+" becomes a lower-bound regression guard against accidental mass-deletion, nothing more.
- **F3 — `cargo xtask error-catalog-check`, kill the bin alias.** Matches the repo convention; one canonical invocation (a second entry point is a second thing that drifts). John files the epic errata (done).
- **F4 — Data/presentation seam with Story 9.5.** THIS story ships the **TOML registry + the CI gate + a deterministic generator** emitting one machine-readable per-error artifact (`<ERR_NAME>` → retryability + cause-chain + version-stability). **Story 9.5 owns rendering / WCAG / i18n / the live `docs.maos.dev/errors/` site and the 100%-coverage gate** (which validates against this catalog). The boundary object is the generated artifact format.
- **Murat's anti-tautology mandate (binding):** the check must be a real oracle — (i) the enum-enumeration must derive from **AST/macro, not a hand-maintained list** (else the whole gate is theater); (ii) a **negative meta-test** must prove the checker FAILS on a deliberately-removed field AND on a deliberately-un-catalogued error. If we can't write a test that makes the checker fail on purpose, it's decoration, not a gate.

---

## Acceptance Criteria

### AC1 — Stable typed codes + 6-field metadata, bidirectionally CI-enforced

**Given** the registered kernel-emitted error set (F2 canonical `E*` set) and `xtask/error-catalog.toml`
**When** `cargo xtask error-catalog-check` runs
**Then** every registered variant carries all **6 metadata fields**: `code` / `severity` / `recovery-class` / `owner` / `kernel-or-spirit` / `since-version` (NFR-Doc-2)
**And** the check **exits non-zero** if any registered variant is missing any field
**And** the check **exits non-zero** if a kernel-emitted `E*` error exists in source but is **absent from the registry** (un-catalogued → fail)
**And** the check **exits non-zero** if the registry lists an entry whose error no longer exists in source (stale → fail)
**And** the enum enumeration derives from **AST/macro inventory**, not a hand-maintained list (Murat's mandate)
**And** `recovery-class` reuses the existing retryability semantics where present (`is_retriable`, `crates/maos-kernel-core/src/inference/router.rs:76-84`) rather than inventing a parallel taxonomy

### AC2 — Anti-tautology negative meta-test

**Given** the catalog check
**When** the test suite runs a fixture that (a) removes a required field from one registry entry and (b) un-catalogues one emitted error
**Then** the checker reports a non-zero/failing result for BOTH mutations (the gate is provably falsifiable, not decoration)

### AC3 — Deterministic catalog generator + Story-9.5 boundary

**Given** the catalog generator
**When** it runs
**Then** it emits a **byte-deterministic** machine-readable artifact (one fragment per `<ERR_NAME>`) carrying **retryability + cause-chain semantics + version-stability guarantee** consistent with the LTS policy (no breaking error-code changes within an LTS cycle)
**And** determinism uses the established discipline (`BTreeMap` ordering, no freshly-read clocks, fixed field order — same family as ADR-028 / 9.2b); two runs produce identical bytes (golden-file test)
**And** a **link-shape test** asserts each registered error maps to a `docs.maos.dev/errors/<ERR_NAME>` path
**And** live-site rendering, WCAG, i18n, and the 100%-coverage gate are **explicitly out of scope** (Story 9.5 consumes this artifact)

### AC4 — Discipline / regression floors

1. **Zero kernel-core KLOC** — `git diff -- crates/maos-kernel-core/src/ --stat` empty; `check-kernel-baseline` green at 21336.
2. **Workspace = 44 crates** (no new crate); **`maos-audit` read-only**, **`maos-cli` kernel-core-free**, **`abi-diff` Added-only** all green (FR63 should not perturb them).
3. **Hard-fail gates green**: `check-review-findings-resolved`, `check-dev-record-completeness`, `check-dev-model-used-populated`, `check-epic-close-green`, `check-service-boundary`. `### Review Findings` a real table or explicit green.
4. **`cargo xtask error-catalog-check` wired into the CI gate harness** (the same place `check_serde_error_handling` and friends are dispatched).
5. **Smoke arm**: `error-catalog-check` passes on the full registered set; flips to non-zero when a field is removed or an error is un-catalogued (AC2 in CI).

---

## Tasks / Subtasks

- [x] **Task 1 — Registry + schema** (AC1)
  - [x] `xtask/error-catalog.toml` with the F2 kernel-emitted `E*` set seeded; 6-field schema per entry (code / severity / recovery-class / owner / kernel-or-spirit / since-version)
  - [x] Define `recovery-class` taxonomy reusing `is_retriable` semantics (`crates/maos-kernel-core/src/inference/router.rs:76-84`)
- [x] **Task 2 — `cargo xtask error-catalog-check`** (AC1, AC2)
  - [x] New `xtask/src/check_error_catalog.rs` modeled on `xtask/src/check_serde_error_handling.rs` (regex/AST scan, TOML config, `--json`, non-zero exit, clear locus); `Commands` variant + dispatch arm in `xtask/src/main.rs` (~:719 neighborhood)
  - [x] **AST/macro** enum inventory (NOT hand-maintained) — bidirectional bijection: missing-field FAIL, un-catalogued FAIL, stale-entry FAIL
  - [x] Negative meta-test fixture proving the checker fails on a removed field AND an un-catalogued error
- [x] **Task 3 — Generator + 9.5 boundary** (AC3)
  - [x] Deterministic per-`<ERR_NAME>` artifact (retryability + cause-chain + version-stability); `BTreeMap`/fixed-order/no-clocks; golden-file determinism test
  - [x] Link-shape test (`docs.maos.dev/errors/<ERR_NAME>`); document the 9.3/9.5 boundary in the artifact README
- [x] **Task 4 — Discipline + smoke** (AC4)
  - [x] Kernel-neutral verified; workspace 44; gates green; check wired into CI harness; smoke arm (pass + provable-fail)

---

## Dev Notes

### What EXISTS and you MUST reuse

| Capability | Location | Reuse for |
|---|---|---|
| CI-check convention (xtask subcommand) | `xtask/src/check_serde_error_handling.rs`; dispatch `xtask/src/main.rs:719` | `error-catalog-check` structure |
| Config-as-TOML-baseline precedent | `xtask/kernel-core-baseline.toml`, `xtask/unsafe-allowlist.toml` | `xtask/error-catalog.toml` registry |
| Retryability semantics | `is_retriable(&ProviderError)` `crates/maos-kernel-core/src/inference/router.rs:76-84` | `recovery-class` field |
| Determinism discipline | ADR-028 / 9.2b (`BTreeMap`, no clocks, fixed field order) | generator byte-determinism |
| Error enum landscape (80+ enums) | workspace-wide; kernel `E*` family in `crates/maos-domain/` + `crates/maos-kernel-core/` | F2 set seed |

### Candidate kernel-emitted `E*` set (F2 — LOCK at Task 1; this is the starting inventory, not the final list)

`EIntentLineageBroken`, `EIntentPromotionDenied`, `EDigestAuditChainMissing`, `EHaltContinuityViolation`, `EMigratorMissing`, `EOrchestratorDispatchRawOutput`, `EContextExhausted`, `ERegionViolation` (9.4), `EComplianceContextDrift`, `ERateLimited`, plus the named variants on `IacBusError` (`crates/maos-domain/src/iac_bus_types.rs:14`). The AST inventory (AC1) is authoritative — this list is the seed the bijection check is built against; the final N is whatever the kernel actually emits.

### What is MISSING and you MUST build

1. Entire catalog infra: `xtask/error-catalog.toml` + `cargo xtask error-catalog-check` + generator + per-error artifact. **No catalog, no `docs/errors/` dir, no check exists today.**

### Project Structure Notes

- No new crate (workspace stays 44). New schemas/artifacts are not crates.
- The check is an `xtask` subcommand (module + `Commands` variant + dispatch arm), NOT a `[[bin]]`.
- Live-site rendering is Story 9.5 — do not stand up a doc site here.

### Previous-work intelligence

- Reuse 9.2b's determinism discipline for the generator (it is the freshest in-tree precedent for byte-identical output).
- The check mirrors `check_serde_error_handling` structurally — that is the lowest-risk path; don't invent a new check shape.
- This story is deliberately the *easy, unblocked* half of the original 9.3. If you hit a frozen-schema or cost-oracle question, you are in **9.3b** — stop and check the seam.

### References

- [Source: requirements-inventory.md] FR63 (:90), NFR-Doc-2 (:193) — with 2026-06-13 preflight errata
- [Source: epics/epic-9-...md] Story 9.3 split note + FR63 ACs (errata'd)
- [Source: 9-3b-...md] the FR62+FR64 half (governance + cost)

---

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

<!--
§A6 NON-OPUS SAFETY NET: FR63 (this story) is kernel-neutral and mechanical — NOT a §A6 correctness-critical
category. Standard review suffices. (§A6 applies to Story 9.3b — kernel delta + cost-vs-billing + governance.)
Record "Opus (net N/A)" or "non-Opus, standard review (FR63 not §A6-critical)".
-->
Opus (net N/A) — FR63 is kernel-neutral, mechanical catalog infra. Not §A6-critical.

### Debug Log References

### Completion Notes List

- **37 E* items catalogued** across 5 scan directories (maos-domain, maos-kernel-core, maos-compliance, maos-a2a-core, maos-skill)
- AST scanner uses `syn` to parse all `.rs` files, discovers E*-prefixed enum variants + variants inside E*-prefixed error enums
- Bidirectional bijection enforced: un-catalogued → FAIL, stale → FAIL, missing field → FAIL, invalid value → FAIL
- 6-field metadata per entry: code / severity / recovery_class / owner / kernel_or_spirit / since_version
- `recovery_class` taxonomy: retry, retry_with_correction, reject, fix_config, escalate — aligned with `is_retriable` semantics
- Generator produces byte-deterministic JSON artifact (BTreeMap ordering, no clocks, fixed field order)
- 16 unit tests including 3 negative meta-tests (AC2), golden-file determinism, link-shape, and live catalog integration tests
- Min entry count regression guard (F2): currently 14, actual count 37
- `docs/errors/error-catalog.json` generated as the 9.3/9.5 boundary artifact
- 2026-06-14 (dev-story closeout verification): `cargo xtask error-catalog-check` PASS (37 registered / 37 discovered, 0 violations); 16 xtask unit tests PASS including 3 negative meta-tests; generator re-run produced zero diff against `docs/errors/error-catalog.json`; kernel-core diff empty; workspace count 44; kernel baseline 21336; hard-fail gates green. Note: `xtask/tests/service_boundary_integration.rs` clean-fixture tests fail pre-existing because `--path` fixture causes workspace_root to resolve to the fixture directory and `spirit-abi-hook-count.toml` is not found there (fallback 11 vs actual 14); the production `cargo xtask check-service-boundary` gate passes.

### File List

- xtask/error-catalog.toml (new)
- xtask/src/check_error_catalog.rs (new)
- xtask/src/main.rs (modified — added mod, Commands variants, dispatch arms)
- docs/errors/error-catalog.json (new — generated artifact)

### Change Log

- 2026-06-13: Story 9.3 FR63 — typed error catalog, CI gate, deterministic generator (claude-opus-4-6)

### Review Findings

All findings resolved by team consensus; `docs_url` keeps `::` per spec and long-term correctness.

#### resolved

- [x] [Review][Decision→Dismiss] `docs_url` embeds Rust `::` path separators in URL slugs — Team consensus: keep `::` per spec; the registered `code` IS `<ERR_NAME>` and Story 9.5 owns routing if it needs normalization. [check_error_catalog.rs:424]
- [x] [Review][Patch] `derives_thiserror` is over-broad — Now matches exact `Error` or `thiserror::Error` path tokens instead of substring `Error`. [check_error_catalog.rs:106-145]
- [x] [Review][Patch] `is_cfg_test` is over-broad — Now parses `cfg` predicate as `syn::Meta` and checks for exact `test` path. [check_error_catalog.rs:148-177]
- [x] [Review][Patch] Dead conditional in `scan_items` — Removed redundant if/else; both discovery rules intentionally produce `EnumName::VariantName`. [check_error_catalog.rs:201-233]
- [x] [Review][Patch] `scan_file` silently swallows read/parse failures — Now returns `Result<…, String>` and propagates I/O / `syn` errors. [check_error_catalog.rs:186-198]
- [x] [Review][Patch] Generated `docs/errors/error-catalog.json` lacks trailing newline — Generator now appends `\n` before writing. [check_error_catalog.rs:492-506]
- [x] [Review][Patch] Duplicate `rust_path` entries in TOML silently last-win — Registry build now detects duplicates and reports an `InvalidValue` violation. [check_error_catalog.rs:325-336]
- [x] [Review][Patch] No `source_file` consistency check — Validation now cross-checks registry `source_file` against discovered source file and reports drift as `Stale`. [check_error_catalog.rs:339-383]
- [x] [Review][Patch] `min_entry_count = 14` is far below actual 37 — Raised to 35. [error-catalog.toml:35]
- [x] [Review][Patch] `run()` resolves workspace root via `std::env::current_dir()` — Now uses `CARGO_MANIFEST_DIR`/parent like the test suite. [check_error_catalog.rs:514-570,610-624]
- [x] [Review][Patch] Validation double-checks `code` and checks `description` outside `REQUIRED_FIELDS` — Consolidated all required fields (including `description` and `source_file`) into a single `REQUIRED_FIELDS` loop; `rust_path` checked separately. [check_error_catalog.rs:385-418]
- [x] [Review][Patch] Generator falls back to `"unknown"` cause-chain semantics — `run_generate` now validates the catalog before generation; `generate_catalog_artifact` panics on unrecognized severity (fail-loud). [check_error_catalog.rs:452-508,572-608]
- [x] [Review][Patch] `mod check_error_catalog` breaks alphabetical ordering in `main.rs` — Moved after `check_env_contract`. [main.rs:18-19]

#### dismissed

- [x] [Review][Dismiss] Hardcoded `version_stability` string — Meets AC3 as written; not required to be TOML-configurable.
- [x] [Review][Dismiss] `retry_with_correction` maps to `retryable: false` — Semantically correct: caller must correct before retrying.
- [x] [Review][Dismiss] Mixed stdout/stderr in `--json` failure mode — Matches existing xtask convention.
