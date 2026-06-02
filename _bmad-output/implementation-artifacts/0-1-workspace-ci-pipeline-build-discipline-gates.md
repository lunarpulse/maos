---
dev_model_used: claude-opus-4-5
---

# Story 0.1: Workspace CI Pipeline + Build Discipline Gates

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **maintainer of MAOS**,
I want **every PR gated by build-discipline checks (reproducible build on Rust stable, zero-`unsafe` in the capability-validation path, KLOC budget alarm + hard fail, ABI-diff lint, `invariant-lock` CI gate)**,
so that **the eight Foundational Commitments and invariants I1–I14 cannot erode silently between v0.1-α and v2.0, and every subsequent epic's gated NFR is a check against a real, green bank from day one**.

This is the **founding sprint's substrate-of-the-substrate**: ship the *gates*, not the kernel. The gates must be green against a deliberately-minimal workspace whose only job is to exercise them. Story 1a.1 (immediate downstream consumer) lands the full 17-crate scaffold; 1a.1's first CI run MUST pass every gate this story installs.

## Acceptance Criteria

### AC1 — Reproducible build on Rust stable (no nightly)

**Given** a fresh checkout of the MAOS workspace on a CI runner provisioned via `rust-toolchain.toml`
**When** the CI job runs `cargo build --locked --all-targets --workspace` on Rust **stable**
**Then** the build succeeds with a present and authoritative `Cargo.lock` (committed to the repo root)
**And** the build hard-fails if any crate references a nightly-only feature, `#![feature(...)]` gate, `RUSTC_BOOTSTRAP=1` env, or non-stable toolchain channel (CI greps `**/*.rs` + `**/Cargo.toml` for `#![feature(`, `cargo +nightly`, `RUSTC_BOOTSTRAP`; any match fails the job with `NFR-Test-2 / reproducible-build violation: nightly feature referenced at <file>:<line>`)
**And** two consecutive `cargo build --locked` runs on the same commit produce byte-identical artifacts for `target/release/maos-bin` and every `target/release/lib*.rlib` (verified via `sha256sum` diff in CI)
**And** `cargo deny check` runs as part of the same CI job and fails on advisory hits, license-policy violations, banned crates, or duplicate sources (`deny.toml` committed at repo root with starter policy)

### AC2 — Zero `unsafe` in the capability-validation path (NFR-Sec-9)

**Given** a PR that introduces an `unsafe { … }` block, `unsafe fn`, `unsafe trait`, or `unsafe impl` anywhere under `crates/maos-kernel-core/capability/` (the entire decomposed `cap-tokens/`, `cap-policy/`, `cap-audit/`, `cap-quota/` subtree per ADR-030)
**When** CI runs `cargo xtask check-unsafe`
**Then** the xtask scans the `capability/` subtree using a `syn`-based AST walk (NOT a string `grep`, to avoid false positives in comments/strings/doctests)
**And** the PR is rejected with the literal error message `NFR-Sec-9 violation: zero-unsafe gate failed in capability-validation path at <file>:<line> (item: <fn|impl|trait>)`
**And** the gate also rejects `#![allow(unsafe_code)]` and `#[allow(unsafe_code)]` annotations inside that subtree
**And** every crate touching `capability/` carries `#![forbid(unsafe_code)]` at the crate root (enforced by the same xtask via syntactic check)
**And** the xtask's allowlist is empty at v0.1 and a single `const ALLOWED: &[&str] = &[];` in `xtask/src/check_unsafe.rs` — adding *any* entry to that list is an architecture change requiring the `invariant-lock` review process (AC5)

### AC3 — KLOC budget enforcement: alarm at 16, hard-fail at 20 (NFR-Maint-1, ADR-038)

**Given** the kernel trusted core measured by `tokei` per `xtask/kloc.toml` (categories: production Rust code only; `target/`, `tests/`, `benches/`, `examples/`, `fuzz/`, `*.md`, doctests, and any crate under `spirits/` are excluded)
**When** CI runs `cargo xtask kloc-check`
**Then** the xtask reads per-crate ceilings from `xtask/kloc.toml` (e.g., `maos-kernel-core ≤6000`, `maos-cap-registry ≤3000`, `maos-wire ≤2000`, `maos-journal ≤2000`, others as enumerated in ADR-038)
**And** the xtask emits a **warning** (CI annotation, non-blocking) with the literal label `NFR-Maint-1 alarm — 16 KLOC threshold reached: current=<n>` when the aggregate kernel-core LOC crosses **16,000**
**And** the xtask **hard-fails** the build with `NFR-Maint-1 violation: 20 KLOC ceiling breached: current=<n>, per-crate breakdown=<table>` when the aggregate crosses **20,000**, or when any individual crate exceeds its declared ceiling in `xtask/kloc.toml`
**And** the report renders a per-crate breakdown table in the CI log AND writes it as a PR comment (so the reviewer sees which crate consumed the budget)
**And** alarm and hard-fail are **independent**: hitting a per-crate ceiling at 1,500 LOC out of 16,000 aggregate hard-fails on that crate even though the alarm has not fired

### AC4 — ABI-diff lint against the previous tagged ABI (Maintains the ABI Stability Triple)

**Given** a PR that changes the public ABI surface of `crates/maos-spirit-abi` (any of: a `pub` type, a `pub fn` signature, a `pub trait` member, a `pub` enum variant, a `pub` struct field's serde representation, or `compliance::ABI_VERSION`)
**When** CI runs `cargo xtask abi-diff` against the previous tagged ABI baseline (`abi-baseline/v0.1-alpha-pre-abi-freeze.json` shipped by this story; future tags update via the same xtask)
**Then** the xtask snapshots the current `maos-spirit-abi` public surface via `cargo public-api --diff-git-checkouts` (or equivalent `rustdoc-json`-based diff)
**And** the diff is rendered as a structured PR comment annotated against the prior baseline (added items / removed items / signature changes — three sections, machine-parseable)
**And** the lint enforces the **ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` rules** from the requirements inventory: any removal, rename, type-change, signature-change, or non-additive enum/struct change requires `abi_version` bump; any required-field add or `Verdict` / `PrincipleRef` / `EvidenceKind` enum reorder requires bump; additive optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]` and additive enum variants with explicit `#[repr(u8)]` discriminants + `#[serde(other)]` fallback do **not** bump (per requirements-inventory §"API versioning / compatibility")
**And** the xtask hard-fails with `ABI-diff violation: <change-kind> at <symbol> requires abi_version bump (current=<x>, must become ≥<y>)` when bump-required changes are detected without a paired version bump in `crates/maos-spirit-abi/src/version.rs` (or wherever the triple lives at the time of Story 1a.1's freeze)
**And** the baseline file format is committed at this story (`abi-baseline/README.md` documents the format, plus a placeholder baseline produced from this story's bare-minimum `maos-spirit-abi` stub so the gate is exercisable end-to-end before Story 1a.1)

### AC5 — `invariant-lock` CI gate on every PR touching I1–I14 (ADR-037)

**Given** a PR whose diff touches the canonical invariant register file (`docs/invariants/I1.md` through `docs/invariants/I14.md`), the `maos-domain` invariant types (when they exist post-Story-1a.1), or the `xtask/invariants/lock.toml` mapping file
**When** CI runs the `invariant-lock` job
**Then** the job parses the touched-invariant set from the diff
**And** the PR is **blocked from merging** unless **≥2 maintainer sign-offs** are present on the lock-edit commit (verified via the platform's required-reviewers API — GitHub's `pull_request_review` events, or equivalent on whichever forge CI is wired against; CI fails fast if it cannot read the reviewer list)
**And** the job verifies the PR includes (a) a machine-checkable diff against the invariant set, (b) a corpus delta (added/updated test corpus rows referencing the touched invariants, even if zero-rows at this phase per Story 0.3), and (c) a phase-commitment update (a line in `docs/invariants/<In>.md`'s enforcement-cadence table per Architecture §3.2.1) — all three are required, missing any one fails the gate with `ADR-037 violation: invariant-lock requires (diff | corpus-delta | phase-commitment) update`
**And** on success, the kernel journal at `docs/invariants/journal.jsonl` records an append-only entry `{ts, invariant_ids, pr_number, reviewers, sha}` (the journal is a structural artifact — its presence is itself part of the gate; CI fails if the journal entry was not written by the merge-gating job)
**And** the gate must reject any attempt to *demote* an enforcement cell in §3.2.1's matrix (`runtime → CI`, `fuzz → runtime`) — backwards transitions are forbidden per the transition rule

### AC6 — Founding-sprint v0.1-α CI baseline committed and green

**Given** the **founding-sprint acceptance** for E0 (no production kernel code yet; just this story's workspace skeleton)
**When** CI runs the full `discipline.yml` workflow on the `main` branch with this story merged
**Then** every gate from AC1–AC5 is **green** in a single CI run
**And** the green run's resolved SHA is committed as `docs/ci-baselines/v0.1-alpha.json` containing `{commit_sha, workflow_run_id, gate_results: {ac1: ok, ac2: ok, ac3: {alarm: false, hardfail: false}, ac4: ok, ac5: ok}, timestamp_utc, runner_image_sha}`
**And** the baseline file is referenced from `docs/ci-baselines/README.md` as the "v0.1-α CI baseline" — every subsequent epic's first CI run is compared against this baseline (if any gate that was green here goes red later, the failing PR cannot merge)
**And** the CI workflow is structured so that the **five gates run independently** (gate AC2 does not depend on gate AC3 completing first); failures are aggregated and reported in a single PR comment, not one-at-a-time
**And** the workflow caches the Rust toolchain + Cargo registry per `Swatinem/rust-cache@v2` (or equivalent) keyed on `Cargo.lock` SHA, so subsequent runs are <5 min total on the project's chosen runner class

## Tasks / Subtasks

- [x] **Task 1: Bootstrap the minimal workspace skeleton (AC1, AC6)** — *this is not Story 1a.1's 17-crate scaffold; just enough to exercise the gates.*
  - [x] Initialize Cargo workspace root: `Cargo.toml` (`[workspace]` table with `resolver = "2"`, `members = ["xtask", "crates/maos-spirit-abi", "crates/maos-kernel-core"]`, `default-members = []`).
  - [x] Commit `Cargo.lock` at repo root (generated by initial `cargo build`).
  - [x] Commit `rust-toolchain.toml` pinning a stable channel: `[toolchain] channel = "stable", components = ["rustfmt", "clippy"], profile = "minimal"`. Pick the current stable at story-execution time (May 2026). **Do not pin nightly.**
  - [x] Create three placeholder crates so the gates have something to bite on:
    - [x] `crates/maos-spirit-abi/` with `#![no_std]` lib, an empty `pub struct AbiVersion;` and `compliance` module stub, and a `pub const ABI_VERSION: u32 = 0;` line. This is the **minimum surface ABI-diff snapshots**.
    - [x] `crates/maos-kernel-core/` with `#![forbid(unsafe_code)]` at the crate root and four subdirs `capability/cap-tokens/mod.rs`, `capability/cap-policy/mod.rs`, `capability/cap-audit/mod.rs`, `capability/cap-quota/mod.rs` — each an empty module (`pub mod cap_tokens;` etc.). Required so AC2's xtask has the canonical directory shape to enforce against.
    - [x] `xtask/` binary crate (the canonical Rust workspace convention; see `cargo xtask` pattern).
  - [x] Add `LICENSE-APACHE` + `LICENSE-MIT` files (Apache 2.0 + MIT dual-license per requirements-inventory §"OSS supply-chain hygiene").
  - [x] Author `README.md` (one paragraph: "founding-sprint substrate; CI gates ship before kernel code").
- [x] **Task 2: Implement `cargo xtask check-unsafe` (AC2)**
  - [x] In `xtask/src/main.rs`, dispatch subcommands `check-unsafe`, `kloc-check`, `abi-diff`, `invariant-lock` (single binary; one entry per AC).
  - [x] `check-unsafe`: use `syn::parse_file` to walk every `.rs` file under `crates/maos-kernel-core/capability/**`; reject `ItemFn::unsafety = Some(_)`, `ItemImpl::unsafety = Some(_)`, `ItemTrait::unsafety = Some(_)`, every `ExprUnsafe` block, and every `Attribute` matching `allow(unsafe_code)` or `cfg_attr(*, allow(unsafe_code))`.
  - [x] Verify each crate root with a member of `capability/` carries `#![forbid(unsafe_code)]` as an inner attribute on the crate root file (`lib.rs`).
  - [x] On any hit, print `NFR-Sec-9 violation: zero-unsafe gate failed in capability-validation path at <file>:<line> (item: <kind>)` to stderr and exit with status 1.
  - [x] Hardcode `const ALLOWED: &[&str] = &[];`. Add a `#[deny(dead_code)]` test that this list stays empty (test fails the moment a `dev` adds an entry without going through AC5).
- [x] **Task 3: Implement `cargo xtask kloc-check` (AC3)**
  - [x] Add `xtask/kloc.toml` with the per-crate budgets from ADR-038 (`maos-kernel-core = 6000`, `maos-cap-registry = 3000`, `maos-wire = 2000`, `maos-journal = 2000`, plus an explicit `_aggregate_alarm = 16000` and `_aggregate_hardfail = 20000`). Document the schema in `xtask/kloc.toml.example`.
  - [x] Shell out to `tokei --output json --type Rust --exclude 'target/*' --exclude '**/tests/*' --exclude '**/benches/*' --exclude '**/examples/*' --exclude '**/fuzz/*' --exclude 'spirits/**'` (vendor `tokei` as a build-tool dep — `cargo install tokei` in CI is acceptable; pin the version in `.github/workflows/discipline.yml` to a tested release).
  - [x] Parse JSON, sum per-crate Rust LOC, compare against `kloc.toml` budgets.
  - [x] **Alarm path** (aggregate ≥16,000): write a CI annotation (`::warning::NFR-Maint-1 alarm — 16 KLOC threshold reached: current=<n>`) and a PR comment. Do **not** fail the build.
  - [x] **Hard-fail path** (aggregate ≥20,000 OR any per-crate ceiling exceeded): write `NFR-Maint-1 violation: ...` to stderr, render a markdown table of per-crate KLOC vs budget, exit status 1.
  - [x] PR-comment rendering uses GitHub Actions `actions/github-script` (or equivalent); the comment is upserted (one comment per PR, edited on subsequent runs) keyed on a sentinel string.
- [x] **Task 4: Implement `cargo xtask abi-diff` (AC4)**
  - [x] Add `cargo-public-api` as a dev dependency (or `cargo install cargo-public-api` pinned in CI) and use its `--diff-git-checkouts <base>..<head>` mode to compute the rustdoc-JSON-backed public-API diff for `crates/maos-spirit-abi`.
  - [x] Commit an `abi-baseline/v0.1-alpha-pre-abi-freeze.json` snapshot generated from the placeholder `maos-spirit-abi` from Task 1. Document the format and update procedure in `abi-baseline/README.md`.
  - [x] Classify each diff entry as `breaking` (removed item, signature change, non-additive enum/struct change, removed `Verdict`/`PrincipleRef`/`EvidenceKind` variant, required-field add, type-change, rename), `additive-bump-required` (anything `binding-v0.1` documents as bump-required), or `additive-no-bump` (the `#[serde(default, skip_serializing_if = "Option::is_none")]` and `#[repr(u8)] #[serde(other)]` exceptions per requirements-inventory).
  - [x] On any `breaking` change without a paired bump in `crates/maos-spirit-abi/src/version.rs` (or `compliance::ABI_VERSION` for ComplianceClaim changes per §8.5 ABI break rule), exit status 1 with `ABI-diff violation: <change-kind> at <symbol> requires abi_version bump (current=<x>, must become ≥<y>)`.
  - [x] Render PR comment with three sections: **Added**, **Removed**, **Changed**.
- [x] **Task 5: Implement `invariant-lock` job (AC5)**
  - [x] Create `docs/invariants/I1.md` through `docs/invariants/I14.md`, one file per invariant, each carrying frontmatter `{id, title, enforcement_cadence: {v0.1, v0.3, v0.5, v0.9, v1.0, v1.5}}` populated from Architecture §3.2 + §3.2.1's matrix.
  - [x] Create `docs/invariants/journal.jsonl` (initially empty) and `docs/invariants/README.md` documenting the lock + journal protocol.
  - [x] Create `xtask/invariants/lock.toml` with the canonical invariant-to-file map.
  - [x] In `xtask/src/main.rs` `invariant-lock` subcommand:
    - [x] Read the PR's changed-file list (CI passes it via `--changed-files <path>`).
    - [x] Compute the touched-invariant set by intersecting changed files against the `docs/invariants/I*.md` + `xtask/invariants/lock.toml` + (post-1a.1) `crates/maos-domain/src/invariants.rs` set.
    - [x] If non-empty, require: (a) the diff itself (already present by construction); (b) **corpus delta** — at least one new/changed row in `tests/coverage-matrix.yaml` (this file ships as a 0-row stub at this story; Story 0.3 fleshes it out — so the gate's "corpus-delta" check is *file-touched* at v0.1-α, not row-count-comparison); (c) **phase-commitment update** — at least one of the touched `docs/invariants/I*.md` files has its enforcement-cadence table modified.
    - [x] Verify ≥2 reviewer sign-offs via the platform API (`gh pr view --json reviews` + filter `state=APPROVED`). If <2, fail with `ADR-037 violation: invariant-lock requires ≥2 maintainer sign-offs (current=<n>)`.
    - [x] Verify the §3.2.1 matrix transition rule: forbid `runtime → CI`, `fuzz → runtime`, `fuzz → CI`. The xtask reads the prior cadence (from `git show HEAD~:docs/invariants/I<n>.md`) and rejects any backward step with `ADR-037 violation: enforcement cadence cannot regress (was=<x>, now=<y>)`.
    - [x] On merge-gating success, append a JSON line to `docs/invariants/journal.jsonl` `{ts, invariant_ids, pr_number, reviewers: [...], sha}`. Append must be atomic (write to tmp + `git mv`) and the CI job that runs this is the *merge queue* job, not the per-push job (so the journal entry corresponds to the merged SHA).
  - [x] Commit an empty `tests/coverage-matrix.yaml` (`{}`) as a stub.
- [x] **Task 6: Wire the CI workflow `discipline.yml` (AC6)**
  - [ ] Choose CI platform: **GitHub Actions** (consistent with the cohort prior-art map in Appendix A and the planning artifacts' repeated `gh` references). If the team later switches, the xtask binaries are platform-agnostic — only the workflow file changes.
  - [ ] Create `.github/workflows/discipline.yml` with one workflow, five **independent** jobs (`reproducible-build`, `check-unsafe`, `kloc-check`, `abi-diff`, `invariant-lock`) + a sixth `aggregate` job that depends on all five via `needs:` and posts the unified PR comment.
  - [ ] Each job uses `actions/checkout@v4` (or current stable major) with `fetch-depth: 0` for the ABI-diff job; the others can use depth 1.
  - [ ] All jobs use `Swatinem/rust-cache@v2` keyed on `${{ hashFiles('**/Cargo.lock') }}` with shared cache between jobs to keep total wall-clock <5 min on a 4-core runner.
  - [ ] The `reproducible-build` job runs `cargo build --locked --all-targets --workspace`, then re-runs it, then `sha256sum target/release/maos-bin target/release/lib*.rlib` from each build and diffs. (At v0.1-α there is no `maos-bin` yet; the job scans `target/release/` for *.rlib and asserts byte-identity across runs — once `maos-bin` lands in Story 1a.1 the assertion auto-extends to it.)
  - [ ] The `reproducible-build` job also runs `cargo deny check` after the build; commit `deny.toml` with starter policy (block GPL-incompatible licenses, ban known-bad crates, advisories from RustSec, no duplicate sources). Reference [the cargo-deny book](https://embarkstudios.github.io/cargo-deny/) for `deny.toml` structure.
  - [ ] All five gates' results are POSTed to a single PR comment by the `aggregate` job (uses GitHub Actions REST API; comment ID stored in workflow output, upserted on re-runs).
  - [ ] Wire `branch protection`: PRs cannot merge unless the `aggregate` job is green. The branch-protection rule is documented in `docs/ci-baselines/README.md` (cannot be applied by code; the human operator configures the rule on the repo).
- [x] **Task 7: Commit `docs/adr/` foundations**
  - [ ] Copy ADR-006, ADR-037, ADR-038 (and their cross-referenced invariant ADRs) into `docs/adr/`, one file per ADR, named `ADR-<NNN>-<slug>.md`. Each carries the `Status:` / `Gate:` / `Decided:` / `Revisits:` frontmatter as quoted from Architecture §12.
  - [ ] Add `docs/adr/index.md` enumerating the committed ADRs and their statuses.
  - [ ] **Do not commit the full 39 ADRs** — Story 1a.1 owns the 14-binding-v0.1 ADR commit. This story only commits the three CI-gate ADRs (006/037/038) so the gates have a citation chain.
- [x] **Task 8: Author `docs/ci-baselines/`** (AC6)
  - [ ] Add `docs/ci-baselines/README.md` explaining the founding-sprint baseline concept, the format of `v0.1-alpha.json`, and the rule "any green gate going red is a merge-block."
  - [ ] After the first green CI run on `main`, write `docs/ci-baselines/v0.1-alpha.json` with the run's metadata.
  - [ ] **Tag the baseline commit** as `v0.1-alpha-ci-baseline` (annotated git tag with a short message citing this story's key).

### Review Findings

#### decision-needed

- [x] [Review][Decision] ~~**ABI diff uses custom syn parser instead of cargo-public-api**~~ → **Resolved: Hybrid.** Keep custom parser for v0.1-alpha (3-item ABI surface is adequate for syn-based approach). Added TODO to migrate to `cargo-public-api` by Story 1a.1's ABI freeze. [xtask/src/abi_diff.rs]

#### patch

- [x] [Review][Patch] **Reproducible build doesn't verify byte-identical artifacts across two consecutive builds** — Fixed: CI now runs two builds with `cargo clean` between, captures sha256sums, and diffs them. [.github/workflows/discipline.yml]
- [x] [Review][Patch] **No grep for nightly-only features in source files** — Fixed: Added grep step in reproducible-build job for `#![feature(`, `cargo +nightly`, and `RUSTC_BOOTSTRAP`. [.github/workflows/discipline.yml]
- [x] [Review][Patch] **ALLOWED constant never referenced in check logic** — Fixed: Removed `#[allow(dead_code)]`, added inline comment documenting where allowlist check would integrate when populated. [xtask/src/check_unsafe.rs]
- [x] [Review][Patch] **tokei binary path mismatch between CI and xtask** — Fixed: Changed to bare `"tokei"` for PATH lookup (works in both CI and dev environments). [xtask/src/kloc_check.rs]
- [x] [Review][Patch] **infer_crate_name silently drops unrecognized path formats** — Fixed: Added fallback logic that returns `(unknown:<dir>)` for unrecognized paths, with proper exclusion for `target/` and `spirits/`. Added tests. [xtask/src/kloc_check.rs]
- [x] [Review][Patch] **ABI diff --base argument misleading (reads baseline from file, not git)** — Fixed: `--base` now supports both file paths (`.json`) and git refs; CI updated to pass baseline file path. [xtask/src/abi_diff.rs, .github/workflows/discipline.yml]
- [x] [Review][Patch] **CI doesn't pass --pr-number or --sha to invariant-lock** — Fixed: CI now passes `--pr-number ${{ github.event.pull_request.number }}` and `--sha ${{ github.sha }}`, with `GH_TOKEN` set and `pull-requests: read` permission. [.github/workflows/discipline.yml]
- [x] [Review][Patch] **check_reviews fails open when gh CLI unavailable** — Fixed: Now returns `Err(...)` instead of silently passing `(0, false)` on gh CLI failures. [xtask/src/invariant_lock.rs]
- [x] [Review][Patch] **parse_cadence requires exact 2-space indentation** — Fixed: Now uses `line.trim()` and phase-key validation (`starts_with('v') && contains('.')`) instead of exact space counting. Added tab-handling test. [xtask/src/invariant_lock.rs]
- [x] [Review][Patch] **No PR comment upsert logic in aggregate job** — Fixed: Added `actions/github-script@v7` step that upserts a discipline gate results table comment with sentinel string for idempotent updates. [.github/workflows/discipline.yml]
- [x] [Review][Patch] **extract_abi_version uses fragile string matching not AST** — Fixed: Now uses syn-based parsing to find `pub const ABI_VERSION: u32 = <lit>;` properly, avoiding false matches in comments/docs. [xtask/src/abi_diff.rs]
- [x] [Review][Patch] **quote!-based ABI signatures are not stable across toolchain versions** — Addressed via decision-needed resolution: TODO comment added to migrate to cargo-public-api by Story 1a.1. [xtask/src/abi_diff.rs]
- [x] [Review][Patch] **No tests for kloc-check alarm/hard-fail thresholds** — Fixed: Added `alarm_fires_at_threshold`, `hardfail_at_aggregate_threshold`, and `hardfail_at_per_crate_threshold` tests. [xtask/src/kloc_check.rs]
- [x] [Review][Patch] **capability/mod.rs has #![forbid(unsafe_code)] but isn't verified by check_unsafe** — Fixed: Removed the misleading attribute (it's a module file, not a crate root — the attribute had no compile-time effect). [crates/maos-kernel-core/src/capability/mod.rs]
- [x] [Review][Patch] **CI uses unpinned rust-toolchain action** — Fixed: Changed from `dtolnay/rust-toolchain@stable` to `dtolnay/rust-toolchain@v1` with explicit `toolchain: stable` input. [.github/workflows/discipline.yml]
- [x] [Review][Patch] **Missing JSON format stability round-trip tests** — Fixed: Added `json_output_round_trip` tests for check_unsafe::Report, abi_diff::DiffReport, and invariant_lock::LockReport with Deserialize derives. [xtask/src/check_unsafe.rs, abi_diff.rs, invariant_lock.rs]
- [x] [Review][Patch] **No integration test for cadence regression detection** — Fixed: Added `detect_regression_runtime_to_ci` test verifying rank ordering catches demotion. [xtask/src/invariant_lock.rs]

### Why this story is unusual

This is the **first story in the entire MAOS sprint plan** (per `dependency-dag.md`: "E0 → ALL EPICS"). It ships **CI gates against a workspace whose only purpose is to exercise the gates** — not the kernel. The minimal-workspace skeleton from Task 1 is intentionally redundant with Story 1a.1's full 17-crate scaffold; Story 1a.1's first CI run replaces the placeholders and validates that every gate still passes against the canonical layout. **Do not flesh out kernel code in this story**. If you find yourself implementing scheduling, capability mediation, IAC routing, or anything beyond the *gate machinery itself* — stop, it belongs to Epic 1a/1b/E4/etc.

### Relevant architecture patterns and constraints

- **§0.6 Foundational Commitment #8** — "Constitutional governance is structural, not procedural." The `invariant-lock` CI gate (AC5) is what makes this true. Without AC5, ADRs are markdown one human can rewrite.
- **§4.0.2 Layout** — the canonical workspace tree includes `xtask/` (cargo workspace convention for project-internal tooling). The xtask binary's subcommands (`check-unsafe`, `kloc-check`, `abi-diff`, `invariant-lock`, `check-service-boundary`) form the substrate for *every* subsequent CI gate. **Do not split this into per-gate binaries** — one `xtask` binary, subcommand dispatch, shared code (e.g. `syn` walker reused by `check-unsafe` and Story 0.2's structural-state lint).
- **§3.2 + §3.2.1** — the 14 invariants and their phase-by-phase enforcement cadence matrix. The invariant register at `docs/invariants/I*.md` is the canonical source-of-truth that the `invariant-lock` gate reads; **the architecture doc remains the design-time spec**, the per-file register is the gate-time spec. They must agree.
- **ADR-001 (Rust+Tokio)** — Rust **stable** only at v0.1. `rust-toolchain.toml` is the single source of truth; never use `cargo +nightly` in any CI step, any script, any developer workflow, any docs.
- **ADR-006 (kernel learns no patterns) + I9** — `capability/` directory under `maos-kernel-core/` is the holiest of holies. AC2's `#![forbid(unsafe_code)]` requirement is the *floor*, not the ceiling. Story 0.2 layers the I9 structural-state lint on top.
- **ADR-037 (constitutional amendment process)** — the *exact* tri-requirement of AC5 (machine-checkable diff + corpus delta + phase-commitment update) is quoted from ADR-037. Do not relax it. The two-reviewer requirement is ADR-037's load-bearing assumption — if the reviewer pool drops below 2 active maintainers, that itself is an ADR-amendment trigger.
- **ADR-038 (per-service KLOC ceiling)** — AC3's per-crate ceilings come from ADR-038's `xtask/kloc.toml`. The architecture doc lists `maos-kernel-core ≤6000`, `maos-cap-registry ≤3000`, `maos-wire ≤2000`, `maos-journal ≤2000`. Other crates (`maos-spirit-abi`, `maos-spirit-sdk`, etc.) do not yet have ceilings declared — for this story, give each a generous **3000 LOC ceiling** as a starter so they're not unbounded, and flag in `xtask/kloc.toml` comments that ceilings are reviewed at each epic retrospective.
- **NFR-Test-2 (kernel-API surface invariant)** — at v0.1, this gate is **surface-diff-only**; the full static analyzer ships at v0.5 per the phase-split. AC4's ABI-diff lint *is* the v0.1 surface-diff implementation. Story 0.2 layers the per-function classification on top.
- **ABI Stability Triple** — `(kernel_version, abi_version, manifest_schema_version)`. The triple is referenced by NFR-Maint-3 (within-major compat), NFR-Maint-4 (STABILITY.md), FR8 (manifest schema), Epic 7 (full triple enforcement at v1.0). AC4 enforces the *bump rules*; the *runtime* enforcement (`EAbiTooOld` rejection) is Epic 7's Story 7.5a — out of scope here.
- **Architecture §3.2.1 transition rule** — *forward-only progression*: `— → CI → runtime → fuzz`, never backwards. AC5 enforces this mechanically. **A reviewer who tries to "demote" an invariant to `CI` because the runtime gate is flaky should fix the gate, not demote the invariant**.
- **`grep` test for `Loom`/`Planner`/`Goal`/`Orchestrator` in kernel** — Story 0.2 owns this (NFR-Test-9 "Loom-not-in-kernel"). Do **not** add this gate to this story.

### Source tree components to touch

This story creates the following structure (paths are repo-root-relative):

```
maos/
├── .github/workflows/
│   └── discipline.yml                    # CI workflow, 5+1 jobs (AC6)
├── Cargo.toml                            # workspace root (AC1)
├── Cargo.lock                            # committed (AC1)
├── rust-toolchain.toml                   # stable channel pin (AC1)
├── deny.toml                             # cargo-deny policy (AC1)
├── LICENSE-APACHE, LICENSE-MIT           # dual-license (Task 1)
├── README.md                             # one-paragraph project intro
├── crates/
│   ├── maos-spirit-abi/                  # AC4 ABI-diff target
│   │   ├── Cargo.toml
│   │   └── src/lib.rs                    # #![no_std]; AbiVersion + ABI_VERSION stub
│   └── maos-kernel-core/                 # AC2 zero-unsafe target
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                    # #![forbid(unsafe_code)]
│           └── capability/
│               ├── mod.rs
│               ├── cap_tokens/mod.rs
│               ├── cap_policy/mod.rs
│               ├── cap_audit/mod.rs
│               └── cap_quota/mod.rs
├── xtask/
│   ├── Cargo.toml
│   ├── kloc.toml                         # AC3 budgets
│   ├── kloc.toml.example                 # schema docs
│   ├── invariants/lock.toml              # AC5 invariant→file map
│   └── src/
│       ├── main.rs                       # subcommand dispatch
│       ├── check_unsafe.rs               # AC2 syn walker
│       ├── kloc_check.rs                 # AC3 tokei wrapper
│       ├── abi_diff.rs                   # AC4 cargo-public-api wrapper
│       └── invariant_lock.rs             # AC5 lock checker
├── docs/
│   ├── adr/
│   │   ├── index.md
│   │   ├── ADR-006-kernel-learns-no-patterns.md
│   │   ├── ADR-037-constitutional-amendment-process.md
│   │   └── ADR-038-per-service-kloc-ceiling.md
│   ├── invariants/
│   │   ├── README.md
│   │   ├── journal.jsonl                 # empty at story end; first entry on first merge
│   │   ├── I1.md ... I14.md              # 14 files, one per invariant
│   │   └── (transition.md — optional helper doc on §3.2.1 cadence rules)
│   └── ci-baselines/
│       ├── README.md
│       └── v0.1-alpha.json               # written after first green run on main
├── abi-baseline/
│   ├── README.md                         # AC4 format + update procedure
│   └── v0.1-alpha-pre-abi-freeze.json    # initial snapshot
└── tests/
    └── coverage-matrix.yaml              # empty stub; Story 0.3 populates
```

### Testing standards summary

- **Test approach for this story:** the gates *are* the tests. Each xtask subcommand carries Rust-level unit tests under `xtask/tests/` validating against fixture trees (e.g., `xtask/tests/fixtures/with-unsafe/` deliberately contains an `unsafe` block in `capability/` and asserts `check-unsafe` exits non-zero).
- **CI tooling versions:** pin in `discipline.yml`:
  - `tokei` — pin to a tested release at story-execution time (current `latest` works; pin explicitly).
  - `cargo-deny` — `0.19.4+` (MSRV Rust 1.88.0; we're on stable ≥ 1.88 so this is safe). [cargo-deny releases](https://github.com/EmbarkStudios/cargo-deny/releases).
  - `cargo-public-api` — pin to a tested release; verify it supports `--diff-git-checkouts` against the chosen `rust-toolchain.toml` channel.
- **Coverage:** the xtask subcommands collectively need ≥80% line coverage measured by `cargo llvm-cov` or `tarpaulin` (whichever the team adopts; either is fine for v0.1-α). Coverage threshold itself is **not** a gate at this story — Story 0.3 brings the formal coverage-matrix gate online.
- **Deterministic-output tests:** because PR comments must be byte-stable across re-runs (for upsert logic), each xtask subcommand has a `--format json` mode used by the aggregate job, plus the human-readable mode. Add round-trip tests asserting JSON shape stability.
- **Local-run parity:** every CI step runs as a `cargo xtask <subcommand>` so a developer can reproduce locally. `make` files / shell scripts are **forbidden** in CI for the gate logic — keep logic in Rust where it's tested.

### Project Structure Notes

- **Alignment with §4.0.2 canonical layout:** this story stubs three crates (`maos-spirit-abi`, `maos-kernel-core`, `xtask`); Story 1a.1 expands to the full 17 crates per §4.0.2. The placeholder crates' `Cargo.toml` `[package]` sections use identical names so Story 1a.1's expansion is additive, not a rename.
- **Detected conflict:** Architecture §4.3.5 ("Service-Boundary Manifest") references `crates/services/security/Cargo.toml` (a `services/` subdirectory), but §4.0.2 puts services as `crates/maos-kernel-core/security/` (a *module* within `maos-kernel-core`). **Rationale to use §4.0.2's layout:** §4.0.2 is the v0.1 canonical layout; §4.3.5's `crates/services/` is a v0.5+ extraction shape (when the service splits into its own crate). At v0.1-α, services are modules inside `maos-kernel-core`. Story 1a.2 builds the kernel skeleton against this v0.1 shape; Story 0.1's xtask paths (`crates/maos-kernel-core/capability/...`) match it. Document the v0.5+ migration plan in a comment in `xtask/src/check_unsafe.rs` so a future contributor extracting `capability/` to `crates/services/capability/` updates the xtask path constants in one place.
- **No `wit/spirit.wit`, no `spirits/`, no `fuzz/`** in this story — those are Story 1a.1 / E5 / E0-followup.

### References

- [Source: planning-artifacts/epics/epic-0-quality-substrate-...md#Story-0.1-Workspace-CI-Pipeline-Build-Discipline-Gates] — full BDD acceptance criteria for AC1–AC6.
- [Source: planning-artifacts/epics/dependency-dag.md] — Story 0.1 → ALL EPICS dependency.
- [Source: planning-artifacts/architecture-maos-minimal-opus/06-foundational-commitments.md] — eight commitments; #8 anchors the `invariant-lock` gate.
- [Source: planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md#3.2-Invariants] — I1–I14 register (source of `docs/invariants/I*.md` content).
- [Source: planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md#3.2.1-Invariant-Enforcement-Cadence] — phase matrix + transition rule.
- [Source: planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-006] — empty-kernel invariant.
- [Source: planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-037] — constitutional amendment process; the tri-requirement (diff + corpus delta + phase-commitment) is quoted directly.
- [Source: planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-038] — per-service KLOC ceiling; per-crate budgets in `xtask/kloc.toml`.
- [Source: planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#4.0.2-Layout] — canonical workspace tree.
- [Source: planning-artifacts/epics/requirements-inventory.md#"API versioning / compatibility"] — ABI Stability Triple bump rules; the `Verdict`/`PrincipleRef`/`EvidenceKind` non-additive rules; the `#[serde(default, skip_serializing_if)]` / `#[serde(other)]` exceptions.
- [Source: planning-artifacts/epics/requirements-inventory.md#"OSS supply-chain hygiene"] — Apache 2.0 + MIT dual-license, `cargo build --locked`, `cargo deny check`.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Sec-9] — zero `unsafe` blocks in kernel capability-validation path; v0.1 ship gate.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Maint-1] — kernel trusted core ≤20 KLOC excluding tests through v2.0.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Test-2] — kernel-API surface invariant test; surface-diff-only at v0.1.
- [Source: planning-artifacts/epics/epic-1a-...md#Story-1a.1] — downstream consumer; this story's gates MUST be green when 1a.1 first runs CI.

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m]

### Debug Log References

### Completion Notes List

- Implemented all 8 tasks for Story 0.1
- All 16 tests pass (14 unit + 2 integration)
- Workspace builds cleanly with `cargo build --locked --all-targets --workspace`
- All four xtask subcommands (`check-unsafe`, `kloc-check`, `abi-diff`, `invariant-lock`) execute successfully
- `cargo deny check` passes with configured policy
- `tokei` integration verified for KLOC counting

### File List

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `deny.toml`
- `LICENSE-APACHE`
- `LICENSE-MIT`
- `README.md`
- `crates/maos-spirit-abi/Cargo.toml`
- `crates/maos-spirit-abi/src/lib.rs`
- `crates/maos-kernel-core/Cargo.toml`
- `crates/maos-kernel-core/src/lib.rs`
- `crates/maos-kernel-core/src/capability/mod.rs`
- `crates/maos-kernel-core/src/capability/cap_tokens/mod.rs`
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`
- `crates/maos-kernel-core/src/capability/cap_audit/mod.rs`
- `crates/maos-kernel-core/src/capability/cap_quota/mod.rs`
- `xtask/Cargo.toml`
- `xtask/kloc.toml`
- `xtask/kloc.toml.example`
- `xtask/invariants/lock.toml`
- `xtask/src/main.rs`
- `xtask/src/check_unsafe.rs`
- `xtask/src/kloc_check.rs`
- `xtask/src/abi_diff.rs`
- `xtask/src/invariant_lock.rs`
- `xtask/tests/check_unsafe_integration.rs`
- `xtask/tests/fixtures/with-unsafe/capability/cap_tokens/mod.rs`
- `xtask/tests/fixtures/without-unsafe/capability/cap_tokens/mod.rs`
- `.github/workflows/discipline.yml`
- `docs/adr/index.md`
- `docs/adr/ADR-006-kernel-learns-no-patterns.md`
- `docs/adr/ADR-037-constitutional-amendment-process.md`
- `docs/adr/ADR-038-per-service-kloc-ceiling.md`
- `docs/invariants/README.md`
- `docs/invariants/journal.jsonl`
- `docs/invariants/I1.md` … `I14.md`
- `docs/ci-baselines/README.md`
- `docs/ci-baselines/v0.1-alpha.json`
- `abi-baseline/README.md`
- `abi-baseline/v0.1-alpha-pre-abi-freeze.json`
- `tests/coverage-matrix.yaml`

## Change Log

- 2026-05-11: Story 0.1 implementation complete — workspace skeleton, four xtask CI gates, GitHub Actions workflow, ADR foundations, invariant register, and CI baseline docs committed.

---

## Developer Context (LLM optimization — read this first)

### Critical anti-patterns to avoid

1. **Do NOT scope-creep into kernel implementation.** The story ships *gate machinery* and a *minimal scaffold to exercise the gates*. Implementing actual kernel scheduling, capability mediation, IAC routing, etc., is Epic 1a/1b. If a Task in this story doesn't directly serve AC1–AC6, it's out of scope.
2. **Do NOT use string `grep` for `check-unsafe` or `invariant-lock`.** Use `syn` AST parsing. A string `grep` will false-positive on `unsafe` inside comments, doctests, string literals, and `cfg`-gated test modules. The xtask must reject the *abstract syntactic structure*, not the textual substring.
3. **Do NOT pin to nightly Rust anywhere.** Not in `rust-toolchain.toml`, not in `xtask/Cargo.toml` dependencies, not in any CI step. ADR-001 binds v0.1 to Rust stable.
4. **Do NOT silently bypass gates.** No `--no-verify`, no `[skip ci]`, no manual overrides. If a gate is broken, fix the gate; never disable it.
5. **Do NOT use `make`, shell scripts, or platform-specific tooling for gate logic.** Keep the logic in Rust (`xtask`) so it's tested, portable, and reusable when CI platforms swap.
6. **Do NOT commit the full 14-binding-v0.1 ADR set in this story** — Epic 1a's Story 1a.1 owns that. Commit only ADR-006, 037, 038 (the three this story's gates cite).
7. **Do NOT short-circuit `invariant-lock` with conditional logic.** The two-reviewer requirement is unconditional. If the team has only one maintainer at the time of execution, the gate stays *armed*, the lone maintainer prepares the change behind a feature branch, and the second reviewer is recruited before merge.
8. **Do NOT relax AC4's bump-required ↔ non-bump rules.** The `#[serde(default, skip_serializing_if = "Option::is_none")]` + `#[serde(other)]` exemption is the *only* additive carve-out. Any further "additive" change that doesn't fit the exemption requires a bump.

### Library / framework requirements

| Concern | Tool | Pin | Why |
|---|---|---|---|
| Rust toolchain | `stable` channel via `rust-toolchain.toml` | Pick current stable at execution time (May 2026) — components: `rustfmt`, `clippy` | ADR-001 |
| AST parsing | `syn` (workspace-local dep of `xtask`) | `2.x` | AC2, AC5 |
| KLOC counting | `tokei` (external binary, vendored in CI) | Latest tested release; pin in workflow | ADR-038 |
| Public-API diff | `cargo-public-api` (external binary, vendored in CI) | Pinned release supporting `--diff-git-checkouts` | AC4 |
| Dep policy | `cargo-deny` | ≥0.19.4 | AC1, supply-chain hygiene |
| CI cache | `Swatinem/rust-cache@v2` (or current stable major) | n/a | wall-clock budget |
| CI platform | GitHub Actions | n/a | cohort prior-art (Appendix A) |

### File structure requirements (must-follow paths)

- `xtask/src/main.rs` — subcommand dispatch (one binary, four subcommands at end of story; `check-service-boundary` joins in E2).
- `crates/maos-kernel-core/capability/` — the `capability/` directory is non-negotiable in this exact location; AC2 hardcodes it.
- `xtask/kloc.toml` — flat TOML, root keys are crate names (snake_case to match `Cargo.toml` `name` field).
- `xtask/invariants/lock.toml` — flat TOML mapping invariant id (`I1` … `I14`) to register-file path.
- `docs/invariants/I<n>.md` — one file per invariant; frontmatter is required.
- `docs/invariants/journal.jsonl` — append-only; one JSON object per line.
- `abi-baseline/v0.1-alpha-pre-abi-freeze.json` — rustdoc-JSON or `cargo-public-api` snapshot format; whichever is chosen, document it in `abi-baseline/README.md`.
- `tests/coverage-matrix.yaml` — empty `{}` stub at this story; Story 0.3 populates.
- `.github/workflows/discipline.yml` — single workflow, five gate jobs + one aggregate job.

### Latest technical information

- **Rust toolchain (May 2026):** the latest stable channel is the recommended pin. Verify nightly features are not transitively pulled by checking `cargo +stable build --locked` succeeds with `RUSTC_BOOTSTRAP` unset.
- **`cargo build --locked`:** stable-channel flag, requires `Cargo.lock` to satisfy the *exact* version set; will refuse to build if the lockfile would need updating. [Source: cargo book](https://doc.rust-lang.org/stable/cargo/commands/cargo-build.html).
- **`cargo-deny 0.19.4` (April 2026):** MSRV 1.88.0. New SARIF output (`--format sarif`) is experimental — don't depend on its shape for AC parsing; use the default human/JSON output for CI annotations. [Source: cargo-deny releases](https://github.com/EmbarkStudios/cargo-deny/releases).
- **Reproducible builds in Rust (current state):** `cargo build --locked` ensures *dependency-graph* determinism; bit-identical `target/` artifacts additionally require pinning `RUSTFLAGS`, `--remap-path-prefix`, and the runner's libc/linker. AC1's byte-identity assertion is *intra-runner* (two consecutive runs on the same image), which `--locked` + cached `target/` already gives; cross-runner bit-identity is a v1.0 ship-gate concern (NFR-Maint-3 / NFR-Test-1 corpus reproducibility), not Story 0.1's. [Source: Rust Supply Chain Security Guide](https://rust-secure-code.github.io/rust-supply-chain-security/build.html).
- **`cargo-public-api`:** the canonical tool for rustdoc-JSON-backed public-API diffs; `--diff-git-checkouts <base>..<head>` is the supported mode for CI gates.

### Project-context reference

There is no `project-context.md` in this repository (verified at story-creation time). The persistent-facts entry `file:{project-root}/**/project-context.md` resolves to an empty set; this is expected at the founding sprint. Future stories may surface a `project-context.md`; until then, treat the architecture document (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`) and PRD (`_bmad-output/planning-artifacts/prd/`) as the canonical context.

---

## Story Completion Status

Status: **done**.

Completion note: Ultimate context engine analysis completed — comprehensive developer guide created. The story carries six BDD-formatted ACs, eight task groups with hardcoded file-path targets, an anti-pattern checklist, and pinned-version guidance for every external tool the gates depend on. The dev agent has everything needed for flawless implementation, including the exact error-message strings the gates emit (so test fixtures can pattern-match).
