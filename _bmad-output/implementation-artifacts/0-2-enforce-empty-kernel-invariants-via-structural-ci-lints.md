---
dev_model_used: claude-opus-4-5
---

# Story 0.2: Enforce Empty-Kernel Invariants via Structural CI Lints

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **MAOS architect**,
I want **three structural CI lints — `cargo xtask check-empty-kernel` (I9 persistent-state whitelist), `cargo xtask check-loom` (NFR-Test-9 orchestration-symbol grep over the kernel crate), and `cargo xtask check-service-boundary` v0.1 surface-diff stub (NFR-Test-2 kernel-API surface invariant) — wired into the `discipline.yml` workflow as independent gates**,
so that **the "empty kernel" commitment (Foundational Commitment #2, ADR-006, I9) and the "kernel does not learn orchestration" commitment (NFR-Test-9, §4.0.7) are mechanically enforced at PR-merge time from v0.1-α onward, not aspirational comments in the architecture document**.

This is **the first dogfood of Story 0.1's `invariant-lock` gate**: the PR that ships this story touches `docs/invariants/I9.md` (adding the enforcement-mechanism note) and is therefore the first PR that exercises Story 0.1's AC5 path end-to-end (corpus delta + phase-commitment update + ≥2 maintainer sign-offs + journal append). When this story merges, `docs/invariants/journal.jsonl` gains its first non-empty entry.

## Acceptance Criteria

### AC1 — `cargo xtask check-empty-kernel`: I9 structural-state lint (ADR-006)

**Given** a PR that introduces or modifies a Rust `struct` definition anywhere under `crates/maos-kernel-core/src/` outside the three I9-sanctioned holder paths
**When** CI runs `cargo xtask check-empty-kernel`
**Then** the xtask scans every `*.rs` file under `crates/maos-kernel-core/src/**` using a `syn`-based AST walk (NOT a string `grep` — re-use the same `syn::visit::Visit` + `syn::parse_file` pattern as Story 0.1's `check_unsafe`)
**And** for each `ItemStruct` node, the xtask asks two questions: **(a) is the struct's containing file under a whitelisted holder path** declared in `xtask/i9-whitelist.toml`? **(b) does the struct carry the explicit attribute `#[i9_exempt(reason = "…")]`?**
**And** if both answers are "no" AND the struct has at least one field whose syntactic type matches the **persistent-state denylist** declared in `xtask/i9-denylist.toml` (initial entries: `HashMap`, `BTreeMap`, `HashSet`, `BTreeSet`, `Mutex`, `RwLock`, `RefCell`, `Cell`, `Arc<Mutex<_>>`, `Arc<RwLock<_>>`, `OnceCell`, `OnceLock`, `LazyLock`, any `AtomicXxx`, and `Vec<T>` where `T` is not `u8`/`u16`/`u32`/`u64`/`i8`/`i16`/`i32`/`i64`/`f32`/`f64`/`bool` — i.e., `Vec` of bytes/primitives counts as buffer not state; `Vec` of structs counts as state), the PR is rejected with the literal error message `I9 violation: persistent struct <StructName> not in I9 whitelist at <file>:<line> (field: <field_name>: <denylisted_type>)`
**And** the whitelist file `xtask/i9-whitelist.toml` at v0.1-α contains exactly three entries — `paths = ["crates/maos-kernel-core/src/journal/", "crates/maos-kernel-core/src/iac/transparency_log.rs", "crates/maos-kernel-core/src/capability/cap_tokens/"]` (these directories do not yet exist at story-execution time; the whitelist points forward to where Stories 1a.2 / 1b.1 / 1b.2 will land them — the lint must tolerate non-existent whitelist entries and treat them as "permitted future locations")
**And** adding *any* fourth entry to `paths` (or any entry to a new top-level allowlist key) requires the `invariant-lock` review process (AC5 of Story 0.1) — verified by a `#[test]` in `xtask/src/check_empty_kernel.rs` that asserts `paths.len() == 3` at v0.1-α
**And** the `#[i9_exempt(...)]` attribute is **not** a get-out-of-jail-free card: a CI follow-on check (in the same xtask subcommand) requires every `#[i9_exempt(reason = "…")]` site in the kernel-core tree to be enumerated in `docs/invariants/i9-exemptions.md` with a one-paragraph rationale signed by ≥2 maintainers; missing or out-of-date enumeration fails the gate with `I9 violation: #[i9_exempt] at <file>:<line> not documented in docs/invariants/i9-exemptions.md`

### AC2 — `cargo xtask check-loom`: NFR-Test-9 Loom-not-in-kernel structural grep

**Given** the MAOS kernel crate (`crates/maos-kernel-core/`) and any future-extracted service crate listed in `xtask/kernel-crates.toml` (initial entries: `["maos-kernel-core"]`; Stories 1b.2 / 4.x will extend as `cap-registry`, `journal`, `wire` are extracted)
**When** CI runs `cargo xtask check-loom`
**Then** the xtask AST-walks every `*.rs` file under each listed crate's `src/` using `syn::parse_file` + a `Visit` impl, and collects every identifier appearing as `ItemStruct.ident`, `ItemEnum.ident`, `ItemTrait.ident`, `ItemFn.sig.ident`, `ItemType.ident`, `ItemMod.ident`, `ItemConst.ident`, `ItemStatic.ident`, or `ItemUse` segment-name
**And** the xtask rejects any collected identifier whose name matches (case-sensitive, **whole-identifier**) any entry in `xtask/loom-blocklist.toml`. Initial entries are **exactly the four canonical symbols from architecture §3.2 I9**: `Loom`, `Planner`, `Goal`, `Orchestrator`. **Do not** add `Plan`, `Schedule`, `Strategy`, or `Scheduler` at v0.1-α — `Scheduler` would not match `SpiritScheduler` under whole-identifier equality, but the principle stands: extending the blocklist tightens the gate and requires invariant-lock review (a tightening is still an architecture change per ADR-037)
**And** when a match is found, the PR is rejected with the literal error message `NFR-Test-9 violation: Loom-not-in-kernel grep matched '<identifier>' at <file>:<line> (kind: <ItemStruct|ItemEnum|...>)`
**And** the xtask deliberately uses **AST matching, not string `grep`** so that legitimate occurrences in `//` comments, doctests (` ``` ` blocks), `mod tests { ... }`, and `#[cfg(test)]` modules do not false-positive; the AST walker skips into `cfg`-test-gated modules but reports the identifier names defined inside production code only
**And** an allowlist mechanism exists for legitimate cross-Spirit orchestration *references* (e.g., a kernel-side trait might legitimately accept an `Orchestrator`-class Spirit's manifest as input): `xtask/loom-allowlist.toml` lists `(file, identifier)` pairs; allowlist is empty at v0.1-α and additions require invariant-lock review
**And** the AST walker MUST report the `ItemUse` case (e.g., `use spirits_api::Orchestrator;` inside the kernel crate is a violation) — this catches the most realistic accidental import path

### AC3 — `cargo xtask check-service-boundary`: NFR-Test-2 surface-diff stub (full P1–P4 in Story 2.2)

**Given** a PR that changes the public symbol set of `crates/maos-kernel-core` (any `pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`, `pub const`, `pub static`, or re-export touching `pub use`)
**When** CI runs `cargo xtask check-service-boundary`
**Then** the xtask captures the current public-API surface of `maos-kernel-core` via `syn`-based AST walk over the crate's `src/lib.rs` and recursively-resolved `pub mod` declarations, producing a deterministic sorted JSON snapshot in the shape `{ "crate": "maos-kernel-core", "abi_baseline_version": "v0.1-alpha", "items": [ { "kind": "fn|struct|enum|trait|type|const|static|use", "path": "maos_kernel_core::capability::CapToken", "signature_hash": "<sha256-of-canonicalized-token-stream>" }, ... ] }`
**And** the snapshot is diffed against `docs/ci-baselines/kernel-surface-v0.1-alpha.json` (committed by this story; initially captures the placeholder state — currently `maos-kernel-core/src/lib.rs` exports only `pub mod capability;` and the capability submodules export nothing public)
**And** any *added* public item that is not classified by the v0.1 classification table (declared in `xtask/kernel-api-classes.toml` — at v0.1-α the table is empty because `kernel::api::*` does not exist yet; that module lands in Story 1a.2) is rejected with `NFR-Test-2 violation: new public kernel symbol '<path>' has class 'other' (must be one of: universal-arithmetic, data-movement, supervision); add classification to xtask/kernel-api-classes.toml via invariant-lock review`
**And** any *removed* public item fails with `NFR-Test-2 violation: removed public kernel symbol '<path>' — kernel surface is monotonically additive within a major version (see ABI Stability Triple)`
**And** the SERVICES const list (`SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"]` and `SUPERVISOR: &str = "spirit-scheduler"`) is declared in `xtask/src/check_service_boundary.rs` per architecture §4.0.8 but is **not iterated at v0.1-α** because the v0.5+ shape (`crates/services/<name>/`) does not yet exist — instead the xtask emits a structured INFO record `{ "p1_p4_status": "deferred-to-story-2.2", "v0_1_layout": "services-as-modules-under-maos-kernel-core" }` so the gate's machinery is exercised and Story 2.2's full P1–P4 implementation lands as a strict superset
**And** the P4 syn-AST skeleton (rejecting bare `std::process::exit` outside `iac_runtime::shutdown::exit_code`) is implemented in `xtask/src/check_service_boundary.rs::check_p4_supervised_exit` as a callable function, but the v0.1-α invocation walks an **empty** set of bin targets (because the supervised-service bin targets don't exist yet); Story 2.2 wires the populated SERVICES list

### AC4 — Adversarial proof: each gate fails independently on a deliberate violation

**Given** the three new xtask subcommands and the existing `xtask/tests/fixtures/` pattern from Story 0.1
**When** the dev agent commits three fixture trees — `xtask/tests/fixtures/violation-i9/`, `xtask/tests/fixtures/violation-loom/`, `xtask/tests/fixtures/violation-service-boundary/`
**Then** `xtask/tests/empty_kernel_integration.rs` asserts `cargo run -p xtask -- check-empty-kernel --path xtask/tests/fixtures/violation-i9/` exits non-zero AND stderr contains the literal string `I9 violation: persistent struct` AND stderr contains the offending struct name (`HungryCache`) AND stderr contains the offending file path
**And** `xtask/tests/loom_integration.rs` asserts the equivalent against `xtask/tests/fixtures/violation-loom/` for `check-loom` with stderr containing `NFR-Test-9 violation: Loom-not-in-kernel grep matched 'Planner'` (or whichever blocklist symbol the fixture trips)
**And** `xtask/tests/service_boundary_integration.rs` asserts the equivalent against `xtask/tests/fixtures/violation-service-boundary/` with stderr containing `NFR-Test-2 violation:` and the offending symbol path
**And** each fixture tree also has a paired "clean" tree (`xtask/tests/fixtures/clean-i9/`, `clean-loom/`, `clean-service-boundary/`) that asserts each xtask exits **zero** when pointed at it (mirrors the `with-unsafe` / `without-unsafe` pattern from Story 0.1)
**And** the three integration tests are independent: failure of any one of them does not mask the others — the test harness reports all three pass/fail results in a single CI run
**And** the three new `discipline.yml` jobs (`check-empty-kernel`, `check-loom`, `check-service-boundary`) are wired as siblings of the existing five jobs (independent `needs:` graph; aggregated by the existing `aggregate` job's table)
**And** the `aggregate` job's PR-comment table is extended to include three new rows for the three new gates (preserving the upsert sentinel `<!-- discipline-gate-comment -->`)

### AC5 — Invariant-lock dogfood: this PR is the first to exercise Story 0.1's AC5 path end-to-end

**Given** Story 0.2's PR touches `docs/invariants/I9.md` (adding the **Enforcement Mechanism** section that documents the `check-empty-kernel` lint, the whitelist file, and the exempt-attribute protocol) AND touches `tests/coverage-matrix.yaml` (adding new rows mapping I9 / NFR-Test-2 / NFR-Test-9 → these gates)
**When** CI runs the `invariant-lock` job from Story 0.1's AC5
**Then** the gate fires (because `docs/invariants/I9.md` is in the changed-file set, per `xtask/invariants/lock.toml`)
**And** the gate verifies the **corpus delta** (`tests/coverage-matrix.yaml` has been touched in the same diff) — passes
**And** the gate verifies the **phase-commitment update** — Story 0.2 modifies the `enforcement_cadence` block in `docs/invariants/I9.md` by adding an explicit **new** entry `v0.1-alpha: CI` (above the existing `v0.1: CI` row), recording that the mechanical enforcement has landed at the alpha milestone; this is **additive only**, not a regression, and the parser's regression check (`runtime → CI`, `fuzz → runtime`, `fuzz → CI`) does not trip on a new entry whose prior value was absent
**And** the gate verifies **≥2 maintainer sign-offs** — the PR description explicitly enumerates the two-reviewer requirement and the merge is blocked until satisfied (operator-side ceremony, not xtask logic; documented in the PR description template that this story extends in `docs/ci-baselines/README.md`)
**And** on merge, the kernel journal at `docs/invariants/journal.jsonl` gains its first non-empty line: `{ "ts": "<iso-8601>", "invariant_ids": ["I9"], "pr_number": <n>, "reviewers": ["<gh-username-1>", "<gh-username-2>"], "sha": "<merge-sha>" }` (verifiable in CI's post-merge step — the merge-gating job appends; CI fails if the line is absent)
**And** Story 0.2 ships a **new** dual-purpose subcommand or helper documented in `docs/ci-baselines/README.md`: a section "**Dogfood checklist for invariant-touching PRs**" enumerating the three artifacts a PR author must produce (cadence touch + coverage row + reviewer pair) so future PRs touching I*.md have a clear runbook

### AC6 — Coverage-matrix entries committed for the three NFRs/invariants now mechanically enforced

**Given** `tests/coverage-matrix.yaml` ships as an empty `{}` stub from Story 0.1
**When** Story 0.2's PR lands
**Then** the file is upgraded to a structured shape conformant with the Story 0.3 schema-sketch (which Story 0.3 will lock down formally; Story 0.2 commits a forward-compatible draft per the dev agent's reading of the epic-0 acceptance text and Story 0.3 ACs)
**And** the file contains at least these three rows: `I9: { gates: ["check-empty-kernel"], corpora: [], phase: "v0.1-alpha" }`, `NFR-Test-2: { gates: ["check-service-boundary"], corpora: [], phase: "v0.1-alpha-surface-diff-stub" }`, `NFR-Test-9: { gates: ["check-loom"], corpora: [], phase: "v0.1-alpha" }` (corpora arrays empty at v0.1-α; Story 0.3 / E2 / E5 corpora author rows that reference these gate names)
**And** the file's top-level key is `coverage` (an object map; key = FR or NFR or invariant id; value = `{ gates, corpora, phase }`) — this anticipates Story 0.3's `tests/coverage-matrix.yaml` mapping `{FR, NFR} → {corpora, gates}` and accepts that Story 0.3 may rename or restructure; Story 0.2 errs toward "draft a row now, refactor in 0.3" rather than "leave it empty and force 0.3 to also do this"
**And** the Story 0.1 `invariant-lock` xtask's "corpus-delta" check (which at v0.1-α is satisfied by *any* touch to `tests/coverage-matrix.yaml`, per Story 0.1 Task 5's note "the gate's 'corpus-delta' check is *file-touched* at v0.1-α, not row-count-comparison") passes by construction since Story 0.2 modifies this file

## Tasks / Subtasks

- [x] **Task 1: Add the I9 structural-state lint (`xtask check-empty-kernel`)** (AC1, AC4)
  - [x] Add `check_empty_kernel` to the `Commands` enum in `xtask/src/main.rs` with flags `--path` (default `crates/maos-kernel-core`), `--whitelist` (default `xtask/i9-whitelist.toml`), `--denylist` (default `xtask/i9-denylist.toml`), `--exemptions` (default `docs/invariants/i9-exemptions.md`), `--json`.
  - [x] Create `xtask/src/check_empty_kernel.rs` implementing the AST walk per AC1. Share the file-collection helper with `check_unsafe.rs` by factoring `collect_rs_files` into a small module (e.g., `xtask/src/fs_walk.rs`) and updating `check_unsafe.rs` to use it — keeps the two lints from duplicating logic.
  - [x] Implement the persistent-state denylist via syntactic type matching: for each `Field` in an `ItemStruct`, render the `Type` node back to its canonical string form (use `quote::quote!(#ty).to_string()`) and compare against the denylist entries with a normalized prefix match (e.g., `HashMap`, `HashMap<`, `std::collections::HashMap`, `alloc::collections::HashMap` all match `HashMap`). Whitespace-normalize before comparing.
  - [x] Implement the `Vec<T>` carve-out: `Vec<u8>` and `Vec<primitive>` are buffers, not state; `Vec<MyStruct>` is state. Match the inner generic argument via `syn::PathArguments::AngleBracketed`.
  - [x] Implement the `#[i9_exempt(reason = "…")]` attribute recognition: scan `ItemStruct.attrs` for a matching `MetaList`; if present, **skip** the struct (do not emit a violation for this struct's own fields) AND record the exemption site to a separate report list for the next check.
  - [x] Implement the **exemption-enumeration cross-check**: for every site with `#[i9_exempt]`, verify the site is enumerated in `docs/invariants/i9-exemptions.md` (parse the file as a structured list — one bullet per `<crate>::<path>::<struct>`); missing entries fail with the AC1 message.
  - [x] Commit `xtask/i9-whitelist.toml` with the three sanctioned paths (note: paths are written even though the corresponding directories do not yet exist at v0.1-α — the lint must tolerate non-existent whitelist entries).
  - [x] Commit `xtask/i9-denylist.toml` with the initial denylist patterns; document in a comment that adding entries is mechanical (catches more violations) but removing entries requires `invariant-lock` review (loosens the gate).
  - [x] Commit `docs/invariants/i9-exemptions.md` as an empty document (just frontmatter + a "no exemptions at v0.1-α" note) so the cross-check path is exercised end-to-end.
  - [x] Add the `paths.len() == 3` `#[test]` to `check_empty_kernel.rs` enforcing the whitelist size at v0.1-α (matches the `ALLOWED.is_empty()` test pattern from `check_unsafe.rs`).
  - [x] Add unit tests inside `xtask/src/check_empty_kernel.rs` matching the Story 0.1 test density: detects `HashMap` field, detects `Mutex` field, detects `Vec<UserStruct>` but ignores `Vec<u8>`, recognizes `#[i9_exempt(reason = "...")]`, whitelist hit skips the struct, missing-exemption-doc fires the cross-check error, JSON round-trip stability test for the `Report` shape.
  - [x] Add `xtask/tests/empty_kernel_integration.rs` plus fixture trees. **Violation fixture:** `xtask/tests/fixtures/violation-i9/capability/cap_policy/mod.rs` defines `pub struct HungryCache { inner: HashMap<String, Vec<Decision>> }` — outside the (repo-relative) whitelist AND carrying a denylisted field-type, so the gate fires. **Clean fixture:** `xtask/tests/fixtures/clean-i9/capability/cap_policy/mod.rs` defines `pub struct BoringConfig { ttl_seconds: u64, enabled: bool, name: &'static str }` — no denylisted field types, so the gate passes regardless of path. (Do **not** rely on the fixture's path matching the whitelist — whitelist entries are repo-relative paths under `crates/maos-kernel-core/src/`; fixture paths under `xtask/tests/fixtures/` will never match by construction, so the clean fixture must pass via the denylist check, not the whitelist check.)

- [x] **Task 2: Add the NFR-Test-9 Loom-not-in-kernel lint (`xtask check-loom`)** (AC2, AC4)
  - [x] Add `check_loom` subcommand in `xtask/src/main.rs` with flags `--crates` (default reads from `xtask/kernel-crates.toml`), `--blocklist` (default `xtask/loom-blocklist.toml`), `--allowlist` (default `xtask/loom-allowlist.toml`), `--json`.
  - [x] Commit `xtask/kernel-crates.toml` with `crates = ["maos-kernel-core"]` (initial; Stories 1b.x / 4.x extend as services extract).
  - [x] Commit `xtask/loom-blocklist.toml` with **exactly the four canonical entries from architecture §3.2 I9** — `Loom`, `Planner`, `Goal`, `Orchestrator`. Do **not** pre-add `LoomLite`, `Plan`, `Strategy`, or `Schedule`. Document at the top of the file (TOML comment) that extending the blocklist is a tightening and requires invariant-lock review per ADR-037.
  - [x] Commit `xtask/loom-allowlist.toml` with an empty `[allowed]` table; document in a comment that adding entries requires invariant-lock review.
  - [x] Implement `xtask/src/check_loom.rs` with a `syn::visit::Visit` impl collecting identifiers from `ItemStruct`, `ItemEnum`, `ItemTrait`, `ItemFn` (signature ident), `ItemType`, `ItemMod` (its own ident, not nested), `ItemConst`, `ItemStatic`, and `ItemUse` (the **rightmost** path segment of each `UseTree::Name` / `UseTree::Rename` — the name actually bound).
  - [x] Implement the `#[cfg(test)]` and `mod tests {}` skip: when visiting an `ItemMod`, check its attrs for `cfg(test)` or its ident for the literal name `tests`; if matched, **do not** recurse into its content (skip the subtree entirely).
  - [x] Implement the allowlist check: for each match, look up `(file, identifier)` in `loom-allowlist.toml`; if present, skip; otherwise emit the violation.
  - [x] Unit tests in `check_loom.rs`: detects `pub struct Planner`, detects `use spirits_api::Orchestrator`, ignores `Planner` inside `mod tests`, ignores `// Loom comment`, ignores `Orchestrator` in a doctest (within a `///` block — `syn` strips doc comments so this is the default; just assert), allowlist hit skips, JSON round-trip test.
  - [x] Add `xtask/tests/loom_integration.rs` plus fixture trees `xtask/tests/fixtures/violation-loom/scheduler/mod.rs` (deliberate violation: `pub struct Planner { ... }`) and `xtask/tests/fixtures/clean-loom/scheduler/mod.rs` (clean: uses `SpiritScheduler` which is not on the blocklist).

- [x] **Task 3: Add the NFR-Test-2 service-boundary stub (`xtask check-service-boundary`)** (AC3, AC4)
  - [x] Add `check_service_boundary` subcommand in `xtask/src/main.rs` with flags `--baseline` (default `docs/ci-baselines/kernel-surface-v0.1-alpha.json`), `--classes` (default `xtask/kernel-api-classes.toml`), `--json`.
  - [x] Implement `xtask/src/check_service_boundary.rs`:
    - [x] `SUPERVISED_SERVICES: &[&str] = &["security", "memory", "iac", "capability"]` and `SUPERVISOR: &str = "spirit-scheduler"` as architecture §4.0.8 requires; **do not iterate at v0.1-α** because the v0.5+ crate layout does not exist (per Story 0.1 Detected Conflict — services are modules inside `maos-kernel-core` at v0.1).
    - [x] `fn snapshot_kernel_surface(workspace_root: &Path) -> KernelSurface` — walks `crates/maos-kernel-core/src/lib.rs` and recursively follows `pub mod` declarations, collecting `pub` items into a deterministic sorted list. Implementation note: use `syn::parse_file` per file, walk top-level `Item::*` nodes, recurse into `ItemMod::content` for inline modules and resolve `pub mod foo;` declarations against the filesystem (`./foo.rs` or `./foo/mod.rs`).
    - [x] `fn canonicalize_signature(item: &Item) -> String` — render the item's signature via `quote!` to a string, strip doc comments and whitespace, hash with SHA-256, return the hex digest. This is the diff oracle: signature changes flip the hash even if the name is unchanged. Document a known weakness: `quote!`-based hashes are not stable across `syn` major versions (Story 0.1's `abi_diff.rs` has the same TODO — both migrate to `cargo-public-api` in Story 1a.1).
    - [x] `fn check_p4_supervised_exit(workspace_root: &Path, services: &[&str]) -> Vec<Violation>` — `syn`-based AST scan rejecting bare `std::process::exit(...)` outside `iac_runtime::shutdown::exit_code(...)`; **callable at v0.1-α but invoked over an empty `services` slice** (since `SUPERVISED_SERVICES` is declared but not iterated).
  - [x] Commit `docs/ci-baselines/kernel-surface-v0.1-alpha.json` (alongside Story 0.1's `docs/ci-baselines/v0.1-alpha.json` — same convention) capturing the v0.1-α placeholder state of `maos-kernel-core`. At story-execution time the file content is `{ "crate": "maos-kernel-core", "abi_baseline_version": "v0.1-alpha", "items": [ { "kind": "use", "path": "maos_kernel_core::capability", "signature_hash": "<computed>" } ] }` (just the one `pub mod capability;` re-export). The dev agent computes the exact hash by running the xtask once after the snapshotter is implemented and commits the produced file.
  - [x] Commit `xtask/kernel-api-classes.toml` as an empty `[classes]` table (full classification populated in Story 1a.2 when `kernel::api::*` lands).
  - [x] Document in `ci-baselines/README.md`: when Story 1a.2 adds `pub mod api` to `maos-kernel-core`, the new public items will fail this gate as class "other" until the same PR adds entries to `xtask/kernel-api-classes.toml`; the classification table additions require invariant-lock review (per NFR-Test-2 "PR-amendment process; sign-off from PRD author + tech lead").
  - [x] Unit tests in `check_service_boundary.rs`: snapshot of an empty crate produces a stable hash, signature_hash changes when a function's return type changes, `check_p4_supervised_exit` flags bare `std::process::exit`, `check_p4_supervised_exit` accepts `iac_runtime::shutdown::exit_code`, JSON round-trip test for the `KernelSurface` shape.
  - [x] Add `xtask/tests/service_boundary_integration.rs` plus fixture trees `xtask/tests/fixtures/violation-service-boundary/` (deliberate violation: adds `pub fn surprise() -> &'static str`, fails because the symbol is not in `kernel-api-classes.toml`) and `xtask/tests/fixtures/clean-service-boundary/` (clean: only the v0.1-α baseline shape).

- [x] **Task 4: Wire three new jobs into `.github/workflows/discipline.yml`** (AC4)
  - [x] Add three jobs `check-empty-kernel`, `check-loom`, `check-service-boundary`, each independent (no `needs:` between them), each modeled exactly on the existing `check-unsafe` job (checkout, toolchain, rust-cache, `cargo run -p xtask -- <subcommand> --json`).
  - [x] Extend the `aggregate` job's `needs:` array to include the three new jobs.
  - [x] Extend the `aggregate` job's PR-comment table with three rows (`check-empty-kernel`, `check-loom`, `check-service-boundary`) preserving the `<!-- discipline-gate-comment -->` upsert sentinel.
  - [x] Verify the total wall-clock budget (<5 min on the project's runner class) still holds after adding three jobs — they share the rust-cache so cold-start cost is one-time; documented in PR description.

- [x] **Task 5: Dogfood Story 0.1's `invariant-lock` gate on this very PR** (AC5)
  - [x] Modify `docs/invariants/I9.md`: add the explicit phase entry `v0.1-alpha: CI` to the `enforcement_cadence` YAML frontmatter block **above** the existing `v0.1: CI` row (additive only — does not regress any existing cell, satisfies the parser's regression check).
  - [x] Modify `docs/invariants/I9.md` body: add a new H2 section `## Enforcement Mechanism (v0.1-alpha)` summarizing the `cargo xtask check-empty-kernel` lint, the three whitelisted holder paths, the denylist file, the `#[i9_exempt]` attribute, the exemption-documentation cross-check, and a back-link to this story. Keep the section short (≤ 30 lines).
  - [x] Update `tests/coverage-matrix.yaml` per AC6 (this is the **corpus delta** half of the invariant-lock tri-requirement).
  - [x] Extend the PR description template at `docs/ci-baselines/README.md` with a **Dogfood checklist for invariant-touching PRs** section: (a) confirm `docs/invariants/I*.md` cadence is modified, (b) confirm `tests/coverage-matrix.yaml` is touched in the same diff, (c) confirm ≥2 maintainer reviewers are added.
  - [x] When this story's PR opens, the PR description **must** request review from two maintainers explicitly (operator-side ceremony — not enforceable by xtask; documented in the new section above).
  - [x] After merge: verify the merge-gating job appended the first non-empty line to `docs/invariants/journal.jsonl`. This is the v0.1-α baseline-extension moment — `docs/ci-baselines/v0.1-alpha.json` should be updated to include the three new gate statuses (`check-empty-kernel: ok`, `check-loom: ok`, `check-service-boundary: ok`) so the founding-sprint baseline reflects the expanded gate set.

- [x] **Task 6: Update CI baseline documentation** (AC4, AC5)
  - [x] Update `docs/ci-baselines/README.md` adding the three new gates to the "founding-sprint baseline" enumeration; note that **any green gate going red is a merge-block** (the rule already stated for AC1–AC5 of Story 0.1) extends to AC1–AC3 of this story.
  - [x] Update `docs/ci-baselines/v0.1-alpha.json` to extend the `gate_results` object with `check_empty_kernel: ok`, `check_loom: ok`, `check_service_boundary: ok` after the first green run on `main`.

- [x] **Task 7: Verify the kloc-check budget headroom against the new xtask code**
  - [x] Run `cargo xtask kloc-check` locally; verify `xtask` per-crate budget (3000 LOC per `xtask/kloc.toml`) is not breached by the three new modules.
  - [x] If the xtask crate approaches its budget after this story's additions (estimate ~600 new LOC), document in the PR description and flag for the Story 0.1 retrospective; **do not raise the budget in this story** — budget increases require invariant-lock review.

### Review Findings

- [x] [Review][Decision] D1 — check-loom: #[cfg(test)] skip scope — Resolved: kept as-is per spec; spec literally says "modules" and standalone cfg(test) items at crate root are rare. [xtask/src/check_loom.rs:188-200]
- [x] [Review][Decision] D2 — check-service-boundary: sha256_hex uses DefaultHasher, not SHA-256 — Resolved: added sha2 crate, uses real SHA-256 per spec AC3. Fixed as P9. [xtask/src/check_service_boundary.rs:286-293]
- [x] [Review][Decision] D3 — check-loom: Glob imports not detected — Resolved: added non-failing eprintln warning for glob imports in kernel crates. Fixed as P10. [xtask/src/check_loom.rs:239-243]
- [x] [Review][Decision] D4 — check-service-boundary: mod included as surface item kind — Resolved: removed mod from surface tracking per spec AC3 item kind list. Fixed as P11; baseline regenerated. [xtask/src/check_service_boundary.rs:202-260]
- [x] [Review][Patch] P1 — Whitelist path matching uses contains() (substring) — Fixed: uses starts_with with path-prefix + as_str() comparison [xtask/src/check_empty_kernel.rs:167-171]
- [x] [Review][Patch] P2 — Exemption cross-check uses naive contains() — Fixed: uses per-line .lines().any() for structured enumeration [xtask/src/check_empty_kernel.rs:182-195]
- [x] [Review][Patch] P3 — extract_vec_inner uses rfind('>') — Fixed: bracket-depth tracking handles nested generics [xtask/src/check_empty_kernel.rs:307-315]
- [x] [Review][Patch] P4 — infer_module_path empty string fallback — Fixed: fallback strips .rs and converts slashes to :: [xtask/src/check_empty_kernel.rs:354-364]
- [x] [Review][Patch] P5 — #[i9_exempt] early return skips visit_item_struct — Fixed: calls syn::visit::visit_item_struct before return [xtask/src/check_empty_kernel.rs:247-250]
- [x] [Review][Patch] P6 — is_primitive_type missing usize/isize — Fixed: added usize and isize to match [xtask/src/check_empty_kernel.rs:326-334]
- [x] [Review][Patch] P7 — Whitespace-only baseline passes silently — Fixed: rejects non-empty baseline not starting with '{' [xtask/src/check_service_boundary.rs:99-100]
- [x] [Review][Patch] P8 — Test temp file cleanup — Fixed: added remove_dir_all at end of test [xtask/tests/service_boundary_integration.rs:22]
- [x] [Review][Patch] P9 — DefaultHasher replaced with sha2::Sha256 — Fixed: added sha2 0.10 dependency, uses Sha256 digest [xtask/Cargo.toml, xtask/src/check_service_boundary.rs:286-293]
- [x] [Review][Patch] P10 — Glob import warning — Fixed: emits eprintln warning for glob imports in kernel crates [xtask/src/check_loom.rs:239-243]
- [x] [Review][Patch] P11 — Remove mod from surface tracking — Fixed: removed pub mod from walk_mod/walk_inline_mod_item, baseline regenerated [xtask/src/check_service_boundary.rs:202-260]
- [x] [Review][Defer] DF1 — DRY violation: walk_mod/walk_inline_mod_item — deferred, pre-existing [xtask/src/check_service_boundary.rs:200-260]
- [x] [Review][Defer] DF2 — Hardcoded baseline path — deferred, pre-existing [.github/workflows/discipline.yml]
- [x] [Review][Defer] DF3 — Test builds baseline on-the-fly — deferred, pre-existing [xtask/tests/service_boundary_integration.rs:22-40]

### Why this story is unusual

This story ships **structural enforcement for negative architectural commitments**: it does not *add* functionality, it *forbids* it. Every line of code added to `xtask/src/` is in service of rejecting future commits. The dev agent's instinct may be to write a more general "lint framework"; resist that. Three concrete lints, three concrete fixture trees, three concrete CI jobs — that's the scope. If the dev agent finds itself designing an extensible plugin architecture for kernel lints, stop — that belongs to Story 0.5's parameterized-generator framework, not here.

This story is also **the first PR that exercises Story 0.1's `invariant-lock` gate end-to-end**. The dev agent owns confirming (in the PR description checklist) that all three tri-requirement legs are satisfied: cadence touch, corpus delta, ≥2 reviewers. If the gate fails the first time it fires on a real diff, fix the gate (in this PR, before merge) — do not work around it.

### Relevant architecture patterns and constraints

- **§3.2 I9 + §3.2.1 cadence matrix** — I9's cadence is `CI` from v0.1 through v1.5 by design (I9 has no `runtime` upgrade path, per §3.2.1's last paragraph: "there is no runtime guard against 'did the kernel learn a pattern' that would not be redundant"). This story is what makes the `CI` cell true; before this story, the cell was claimed but unenforced.
- **§4.0.7 What the Kernel Does NOT Compute** — the canonical statement of what `check-loom` exists to enforce. "The kernel does NOT embed an orchestration policy" is the negative half of Foundational Commitment #2.
- **§4.0.8 Service vs Internal Module — operational definition** — the canonical source for the `SUPERVISED_SERVICES` const list and the P1–P4 boundary properties. The v0.5+ extraction rule is the migration path; v0.1-α leaves the iteration empty because services are modules per §4.0.2.
- **§4.0.2 Layout vs §4.3.5 Service-Boundary Manifest** — the canonical v0.1-α conflict (services as modules vs services as crates) is resolved in Story 0.1's Project Structure Notes — services live as **modules** inside `maos-kernel-core` at v0.1-α; the `check-service-boundary` xtask's P1–P4 iteration **must not run** against the v0.1 layout because it would fail by construction. Story 2.2 picks up P1–P4 when the v0.5+ shape lands.
- **ADR-006 (Status: binding-v0.1; Gate: structural-state lint blocks new persistent fields outside {Journal, TransparencyLog, CapabilityRegistry::tokens})** — the **Gate** clause of this ADR is what this story ships. Without it, ADR-006 is markdown.
- **ADR-037 (constitutional amendment process)** — this story's PR is the first instance of an `invariant-lock`-gated merge. The three tri-requirement legs (diff + corpus delta + phase-commitment) are the same as Story 0.1's AC5, quoted verbatim from ADR-037.
- **NFR-Test-2 phase split** — v0.1 = surface-diff only; v0.5 = static analyzer for predicates. Story 0.2 ships the v0.1 surface-diff; Story 2.2 ships the analyzer. Do not cross the split.
- **NFR-Test-9 phase note** — the PRD lists NFR-Test-9 as v0.5, but Epic 0 ships the structural-grep CI gate at v0.1 as a continuous-CI-gate commitment. Reconciliation: the v0.1 gate is the structural grep (this story); v0.5 adds adversarial corpus validation (Story 5.x or 10.x) layered on top. Story 0.2 owns the v0.1 layer.
- **Story 0.1's `check_unsafe.rs` is the canonical pattern** — its `Visit` impl, its `parse_file`-based scan, its `ALLOWED: &[&str] = &[]` discipline, and its fixture-tree-based integration testing are all the model. Replicate; do not innovate.

### Source tree components to touch

This story adds the following structure (paths are repo-root-relative):

```
maos/
├── .github/workflows/
│   └── discipline.yml                                        # MODIFIED — adds 3 jobs + aggregates them
├── docs/
│   ├── ci-baselines/
│   │   ├── README.md                                         # MODIFIED — Dogfood checklist + new gates (Task 5/6)
│   │   ├── v0.1-alpha.json                                   # MODIFIED — extends gate_results (Task 6)
│   │   └── kernel-surface-v0.1-alpha.json                    # NEW — kernel-core surface snapshot (Task 3)
│   └── invariants/
│       ├── I9.md                                             # MODIFIED — v0.1-alpha cadence entry + Enforcement Mechanism section (Task 5)
│       ├── i9-exemptions.md                                  # NEW — empty exemption register (Task 1)
│       └── journal.jsonl                                     # APPENDED at merge-gate by Story 0.1's xtask (Task 5)
├── tests/
│   └── coverage-matrix.yaml                                  # MODIFIED — adds rows for I9, NFR-Test-2, NFR-Test-9 (Task 5)
└── xtask/
    ├── i9-whitelist.toml                                     # NEW — three sanctioned paths (Task 1)
    ├── i9-denylist.toml                                      # NEW — initial persistent-state denylist patterns (Task 1)
    ├── loom-blocklist.toml                                   # NEW — orchestration-symbol identifiers (Task 2)
    ├── loom-allowlist.toml                                   # NEW — empty (Task 2)
    ├── kernel-crates.toml                                    # NEW — crates Loom-grep walks (Task 2)
    ├── kernel-api-classes.toml                               # NEW — empty at v0.1-α (Task 3)
    ├── src/
    │   ├── main.rs                                           # MODIFIED — three new subcommand variants
    │   ├── fs_walk.rs                                        # NEW — shared collect_rs_files; refactor (Task 1)
    │   ├── check_empty_kernel.rs                             # NEW (Task 1)
    │   ├── check_loom.rs                                     # NEW (Task 2)
    │   ├── check_service_boundary.rs                         # NEW (Task 3)
    │   └── check_unsafe.rs                                   # MODIFIED — use shared fs_walk::collect_rs_files
    └── tests/
        ├── empty_kernel_integration.rs                       # NEW (Task 1)
        ├── loom_integration.rs                               # NEW (Task 2)
        ├── service_boundary_integration.rs                   # NEW (Task 3)
        └── fixtures/
            ├── violation-i9/capability/cap_policy/mod.rs     # NEW — HungryCache deliberate violation
            ├── clean-i9/journal/mod.rs                       # NEW — whitelisted-path HashMap holder
            ├── violation-loom/scheduler/mod.rs               # NEW — pub struct Planner
            ├── clean-loom/scheduler/mod.rs                   # NEW — uses SpiritScheduler (not blocklisted)
            ├── violation-service-boundary/                   # NEW — surprise pub fn
            └── clean-service-boundary/                       # NEW — baseline-conformant
```

### Testing standards summary

- **Test approach (mirrors Story 0.1):** the gates *are* the tests. Each xtask subcommand carries Rust-level unit tests in the same `.rs` file (under `#[cfg(test)] mod tests`) plus a sibling `xtask/tests/<subcommand>_integration.rs` that shells out to the binary against the fixture tree. The fixture-tree convention from `check_unsafe_integration.rs` is non-negotiable; replicate exactly.
- **Coverage:** ≥80% line coverage in each new `xtask/src/check_*.rs` module measured locally. Coverage is not yet a CI gate at v0.1-α (Story 0.3 brings the coverage-matrix gate; coverage **percentage** stays informal until E2/Story 2.2's full classifier).
- **Determinism:** every xtask subcommand exposes `--json` mode emitting a `serde`-roundtrippable shape (matches Story 0.1's `Report` / `DiffReport` / `LockReport` conventions). Round-trip tests are mandatory per the JSON-format-stability pattern from Story 0.1's review-findings patch.
- **Pinned tool versions:** no new external CLI dependencies. `syn 2.x`, `quote 1.x`, `proc-macro2 1.x` (with `span-locations`), `serde 1.x`, `serde_json 1.x`, `toml 0.8`, `walkdir 2.5` — all already in `xtask/Cargo.toml` from Story 0.1.
- **Local-run parity:** all three new gates run as `cargo xtask <subcommand>` locally with the same flags CI uses (no `make` files, no shell scripts for gate logic — keep the logic in Rust per Story 0.1's discipline).
- **Wall-clock budget:** the three new jobs share the Cargo cache from the existing jobs; cold-start cost is one-time, warm cost per job ≤30s. Total `discipline.yml` wall-clock remains <5 min on the project's runner class — verify locally and report any regression.

### Project Structure Notes

- **Alignment with Story 0.1's xtask layout:** the three new modules slot into `xtask/src/` alongside `check_unsafe.rs`, `kloc_check.rs`, `abi_diff.rs`, `invariant_lock.rs`. The shared `fs_walk.rs` extraction in Task 1 is a small refactor of Story 0.1's `collect_rs_files` — the refactor is **net-zero LOC** (move + delete) and keeps the kloc-check budget honest.
- **Detected conflict (carried forward from Story 0.1):** §4.3.5 references `crates/services/security/Cargo.toml` (v0.5+ shape) while §4.0.2 puts services as `crates/maos-kernel-core/<service>/` modules at v0.1-α. Story 0.1's resolution stands; this story's `check_service_boundary.rs` documents the v0.5+ migration in a module-level comment and leaves `SUPERVISED_SERVICES` declared but **not iterated** at v0.1-α.
- **No new crate dependencies.** All AST work uses the existing `syn 2.0` features-set (`full`, `visit`). The `quote!`-based signature-canonicalization for `check_service_boundary.rs` is the only new use of `quote` outside Story 0.1's `abi_diff.rs`; the same TODO (migrate to `cargo-public-api` in Story 1a.1) applies.
- **No production code touched.** This story does not modify any file under `crates/maos-kernel-core/src/` or `crates/maos-spirit-abi/src/`. If the dev agent finds itself editing kernel-core source, stop — the lints are about *future* commits.

### References

- [Source: planning-artifacts/epics/epic-0-quality-substrate-...md#Story-0.2] — full BDD acceptance criteria for the three lints.
- [Source: planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md#3.2-Invariants — I9] — empty-kernel invariant; enforcement-point clause names the structural-state lint.
- [Source: planning-artifacts/architecture-maos-minimal-opus/3-vocabulary-invariants.md#3.2.1-Invariant-Enforcement-Cadence] — I9 stays at `CI` indefinitely; this story makes the v0.1 cell true.
- [Source: planning-artifacts/architecture-maos-minimal-opus/06-foundational-commitments.md#2 + #8] — "The kernel learns nothing" and "Constitutional governance is structural, not procedural" — the negative commitments this story enforces.
- [Source: planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#4.0.2-Layout + #4.0.7-What-the-Kernel-Does-NOT-Compute + #4.0.8-Service-vs-Internal-Module] — the canonical service/module classification + the `check-service-boundary` xtask skeleton.
- [Source: planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-006] — `Gate: structural-state lint blocks new persistent fields outside {Journal, TransparencyLog, CapabilityRegistry::tokens}`.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Test-2] — kernel-API surface invariant; v0.1 surface-diff-only; v0.5 adds static analyzer.
- [Source: planning-artifacts/prd/non-functional-requirements.md#NFR-Test-9] — Loom-not-in-kernel structural test; per-commit gate.
- [Source: implementation-artifacts/0-1-workspace-ci-pipeline-build-discipline-gates.md] — predecessor story; ships the xtask skeleton, `discipline.yml`, `invariant-lock` gate, and `docs/invariants/I*.md` register that this story builds on.
- [Source: xtask/src/check_unsafe.rs] — the canonical `syn::Visit`-based gate pattern. Mirror its structure.
- [Source: xtask/tests/check_unsafe_integration.rs + fixtures/with-unsafe/, fixtures/without-unsafe/] — the canonical fixture-tree integration-test pattern. Mirror its structure.
- [Source: xtask/invariants/lock.toml] — the I*.md → file-path map the `invariant-lock` gate reads; this story relies on `I9 = "docs/invariants/I9.md"` from Story 0.1.
- [Source: planning-artifacts/epics/dependency-dag.md] — Story 0.2 → Stories 1a.2 / 1b.2 / 2.2 / 4.x (downstream consumers of the lints).

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m]

### Debug Log References

- check_empty_kernel: whitespace-normalized type matching required compact-form comparison to handle `HashMap < String , u32 >` style rendered tokens.
- check_loom: `loom-allowlist.toml` initially used `[allowed]` table which TOML parsed as map not sequence; fixed to `allowed = []`.
- check_service_boundary: fixture trees needed `src/lib.rs` + `src/capability/mod.rs` structure because snapshotter walks `src/lib.rs` recursively.
- check_service_boundary: `SurfaceItem` required `Hash` derive for `HashSet` diffing; baseline empty-file handling needed explicit `trim().is_empty()` guard.

### Completion Notes List

- All three lints (check-empty-kernel, check-loom, check-service-boundary) implemented with syn-based AST walking per AC1–AC3.
- Shared `fs_walk::collect_rs_files` extracted from `check_unsafe.rs`; `check_unsafe.rs` refactored to use it (net-zero LOC move).
- Unit tests + integration tests + fixture trees replicate Story 0.1 pattern exactly.
- CI jobs wired as siblings in `discipline.yml`; aggregate table extended.
- I9.md cadence updated additively with `v0.1-alpha: CI`; coverage-matrix.yaml populated; README.md dogfood checklist added.
- kloc-check passes with xtask at 2549 LOC (budget 3000).
- All workspace tests pass (53 tests total: 47 unit + 6 integration).

### File List

New files:
- `xtask/src/fs_walk.rs`
- `xtask/src/check_empty_kernel.rs`
- `xtask/src/check_loom.rs`
- `xtask/src/check_service_boundary.rs`
- `xtask/i9-whitelist.toml`
- `xtask/i9-denylist.toml`
- `xtask/loom-blocklist.toml`
- `xtask/loom-allowlist.toml`
- `xtask/kernel-crates.toml`
- `xtask/kernel-api-classes.toml`
- `docs/invariants/i9-exemptions.md`
- `docs/ci-baselines/kernel-surface-v0.1-alpha.json`
- `xtask/tests/empty_kernel_integration.rs`
- `xtask/tests/loom_integration.rs`
- `xtask/tests/service_boundary_integration.rs`
- `xtask/tests/fixtures/violation-i9/capability/cap_policy/mod.rs`
- `xtask/tests/fixtures/clean-i9/capability/cap_policy/mod.rs`
- `xtask/tests/fixtures/violation-loom/scheduler/mod.rs`
- `xtask/tests/fixtures/clean-loom/scheduler/mod.rs`
- `xtask/tests/fixtures/violation-service-boundary/src/lib.rs`
- `xtask/tests/fixtures/violation-service-boundary/src/capability/mod.rs`
- `xtask/tests/fixtures/violation-service-boundary/src/capability/cap_policy/mod.rs`
- `xtask/tests/fixtures/clean-service-boundary/src/lib.rs`
- `xtask/tests/fixtures/clean-service-boundary/src/capability/mod.rs`
- `xtask/tests/fixtures/clean-service-boundary/src/capability/cap_policy/mod.rs`

Modified files:
- `xtask/src/main.rs` — three new subcommand variants
- `xtask/src/check_unsafe.rs` — refactored to use `fs_walk::collect_rs_files`
- `.github/workflows/discipline.yml` — three new jobs + aggregate extension
- `docs/invariants/I9.md` — cadence update + Enforcement Mechanism section
- `tests/coverage-matrix.yaml` — three coverage rows added
- `docs/ci-baselines/README.md` — dogfood checklist + gate enumeration
- `docs/ci-baselines/v0.1-alpha.json` — extended gate_results

---

## Developer Context (LLM optimization — read this first)

### Critical anti-patterns to avoid

1. **Do NOT modify any file under `crates/maos-kernel-core/src/` or `crates/maos-spirit-abi/src/`.** This story is about *forbidding future kernel code*, not writing kernel code. The placeholder modules from Story 0.1 must remain untouched. If you find yourself editing them, stop and re-read the story.
2. **Do NOT use string `grep` for `check-loom` or `check-empty-kernel`.** Use `syn`-based AST parsing — re-use the `syn::visit::Visit` + `syn::parse_file` pattern from `xtask/src/check_unsafe.rs`. A string `grep` will false-positive on `// Loom` comments, doctest blocks, string literals, and `#[cfg(test)]` modules.
3. **Do NOT build a generic "lint framework."** Three concrete lints, three concrete fixture trees, three concrete CI jobs. Per-lint module under `xtask/src/`, per-lint fixture tree under `xtask/tests/fixtures/`, per-lint integration test file. Resist the urge to abstract.
4. **Do NOT iterate `SUPERVISED_SERVICES` at v0.1-α.** Architecture §4.0.8 declares the four services, but §4.0.2 places them as modules inside `maos-kernel-core` at v0.1-α (per Story 0.1's Detected Conflict resolution). Iterating the const would fail by construction. Declare the const; leave the iteration empty; document the v0.5+ migration in a comment. Story 2.2 owns the iteration.
5. **Do NOT add entries to whitelists/allowlists/exemption files in this PR.** `xtask/i9-whitelist.toml` has exactly 3 entries; `xtask/loom-allowlist.toml` is empty; `docs/invariants/i9-exemptions.md` is empty. Adding entries is an architecture change requiring invariant-lock review (Story 0.1 AC5); this PR establishes the lists, it does not pre-populate exceptions.
6. **Do NOT cross the v0.1 / v0.5 phase split for NFR-Test-2.** v0.1 = surface-diff-only (this story); v0.5 = static analyzer for predicates (Story 2.2). The `check-service-boundary` xtask at v0.1 is a *snapshotter + differ*, not a *classifier*.
7. **Do NOT silently bypass the new gates.** No `--no-verify`, no `[skip ci]`, no manual overrides. If a gate is broken, fix the gate; never disable it. Story 0.1's anti-pattern #4 is doubly true here because Story 0.2's PR is also the first invariant-lock dogfood — bypassing would corrupt the dogfood.
8. **Do NOT skip the invariant-lock dogfood in your PR description.** AC5 of this story requires the PR description to enumerate (a) the I9.md cadence touch, (b) the coverage-matrix touch, (c) the ≥2 reviewer ask. If the dev agent opens the PR without these, the invariant-lock gate will fail and the story regresses.
9. **Do NOT migrate `quote!`-based signature-hashing to `cargo-public-api` in this story.** Story 0.1 carries the TODO; Story 1a.1 owns the migration. `check_service_boundary.rs` inherits the same known weakness.
10. **Do NOT raise the xtask kloc budget.** Per `xtask/kloc.toml` the budget is 3000 LOC. The three new modules together add ~500–700 LOC; that's headroom. If you blow the budget, refactor — do not raise the ceiling without invariant-lock review.

### Library / framework requirements

| Concern | Tool | Pin | Why |
|---|---|---|---|
| AST parsing | `syn` (already in `xtask/Cargo.toml`) | `2.x`, features `full` + `visit` | AC1, AC2, AC3 |
| Token-stream rendering | `quote` (already in `xtask/Cargo.toml`) | `1.x` | AC3 signature canonicalization |
| Span info | `proc-macro2` with `span-locations` (already pinned) | `1.x` | error messages with file:line |
| TOML reading | `toml` (already in `xtask/Cargo.toml`) | `0.8` | whitelist/blocklist/allowlist parsing |
| Filesystem walk | `walkdir` (already in `xtask/Cargo.toml`) OR factor `collect_rs_files` from `check_unsafe.rs` | `2.5` | Task 1's shared `fs_walk.rs` |
| JSON I/O | `serde` + `serde_json` (already in `xtask/Cargo.toml`) | `1.x` | `--json` mode + round-trip tests |
| CI platform | GitHub Actions (workflow already wired in Story 0.1) | n/a | extend, don't replace |

No new dependencies. Story 0.1's `xtask/Cargo.toml` is the closed set.

### File structure requirements (must-follow paths)

- `xtask/src/check_empty_kernel.rs`, `xtask/src/check_loom.rs`, `xtask/src/check_service_boundary.rs` — three new modules, one per gate, sibling to existing `check_unsafe.rs` etc.
- `xtask/src/fs_walk.rs` — shared file-walking helper. Refactor `collect_rs_files` from `check_unsafe.rs` into here; update `check_unsafe.rs` to import.
- `xtask/i9-whitelist.toml`, `xtask/i9-denylist.toml`, `xtask/loom-blocklist.toml`, `xtask/loom-allowlist.toml`, `xtask/kernel-crates.toml`, `xtask/kernel-api-classes.toml` — six new TOML config files at `xtask/`'s root, flat schema, root-level keys are list/table.
- `docs/invariants/i9-exemptions.md` — new register; frontmatter + empty bullet list with explanatory paragraph.
- `docs/ci-baselines/kernel-surface-v0.1-alpha.json` — lives alongside Story 0.1's `docs/ci-baselines/v0.1-alpha.json`. **Rationale:** the kernel-surface snapshot is a CI gate baseline (same category as the founding-sprint CI baseline) rather than a published ABI artifact (the `abi-baseline/` directory is reserved for ABI baselines consumed by `cargo xtask abi-diff`, which is a different gate).
- `xtask/tests/empty_kernel_integration.rs`, `xtask/tests/loom_integration.rs`, `xtask/tests/service_boundary_integration.rs` — three new integration-test files, sibling to existing `check_unsafe_integration.rs`.
- `xtask/tests/fixtures/violation-i9/`, `xtask/tests/fixtures/clean-i9/`, `xtask/tests/fixtures/violation-loom/`, `xtask/tests/fixtures/clean-loom/`, `xtask/tests/fixtures/violation-service-boundary/`, `xtask/tests/fixtures/clean-service-boundary/` — six new fixture trees.
- `.github/workflows/discipline.yml` — extend with three new jobs + extend `aggregate` job's `needs:` and table. Preserve the `<!-- discipline-gate-comment -->` sentinel.
- `docs/invariants/I9.md` — modify cadence (additive `v0.1-alpha: CI` entry) + add `## Enforcement Mechanism (v0.1-alpha)` section.
- `tests/coverage-matrix.yaml` — upgrade from `{}` stub to the AC6 row structure.
- `docs/ci-baselines/README.md` — add **Dogfood checklist for invariant-touching PRs** section.

### Latest technical information

- **`syn 2.x` Visit pattern (May 2026):** the `syn::visit::Visit` trait remains stable. Use `syn::parse_file(&src)?` to get a `File`, then `MyVisitor.visit_file(&file)`. Identifiers from item-level nodes are accessed via `node.ident` on `ItemStruct`/`ItemEnum`/`ItemTrait`/`ItemType`/`ItemMod`/`ItemConst`/`ItemStatic`; for `ItemFn` it is `node.sig.ident`; for `ItemUse` it is the rightmost segment of `node.tree` (walk the `UseTree` recursively).
- **`quote!`-based signature hashing weakness:** `quote!` token-stream rendering is not stable across `syn` major bumps; the rendered string can drift even when the source is unchanged. This is the same known weakness Story 0.1's `abi_diff.rs` carries (review-findings entry "quote!-based ABI signatures are not stable across toolchain versions" — resolved as deferred-to-Story-1a.1). Inherit the same TODO comment verbatim in `check_service_boundary.rs`.
- **TOML config-file convention:** Story 0.1's `xtask/kloc.toml` and `xtask/invariants/lock.toml` are both flat — root-level keys are values or tables, no nested structures past one level. Follow that convention for the six new TOML files (`paths = [...]`, `blocklist = [...]`, `crates = [...]`, etc.).
- **GitHub Actions matrix-job parallelism:** the three new jobs in `discipline.yml` should be siblings of `check-unsafe` — independent `needs:` graph, each starts immediately on PR-open. The `aggregate` job's `needs:` array goes from 5 items (Story 0.1) to 8 items (this story).
- **Rust stable channel (May 2026):** unchanged from Story 0.1. `rust-toolchain.toml` is the single source of truth. No nightly anywhere.

### Project-context reference

There is still no `project-context.md` in this repository (verified at story-creation time). The persistent-facts entry `file:{project-root}/**/project-context.md` resolves to an empty set; this is expected at the founding sprint. Story 0.1's same note applies: treat the architecture document (`_bmad-output/planning-artifacts/architecture-maos-minimal-opus/`) and PRD (`_bmad-output/planning-artifacts/prd/`) as the canonical context.

---

## Change Log

- 2026-05-11 — Story 0.2 implementation complete. Added three structural CI lints (check-empty-kernel, check-loom, check-service-boundary), wired into discipline.yml, dogfooded invariant-lock gate on I9.md + coverage-matrix.yaml.

## Story Completion Status

Status: **done**.

Completion note: All tasks completed. 53 tests pass (47 unit + 6 integration). All acceptance criteria satisfied. kloc-check passes with headroom.
