# Story 2.3: Thin `cargo-generate` Template + Local Runner (NFR-Onb-1 v0.3 Prerequisite)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

**Epic:** 2 — Spirit ABI + Developer SDK + Boundary Contracts (v0.1 → v0.3)
**Epic state at story open:** `epic-2: in-progress` (flipped at Story 2.1 creation; no change).
**Story key:** `2-3-thin-cargo-generate-template-local-runner-nfr-onb-1-v0-3-prerequisite`
**Story file:** `_bmad-output/implementation-artifacts/2-3-thin-cargo-generate-template-local-runner-nfr-onb-1-v0-3-prerequisite.md`
**Predecessors:**
- Story 2.1 — full Spirit ABI (`Spirit` trait + 11 hooks at `crates/maos-spirit-abi/src/lifecycle.rs`, `SpiritVtable<T>` with `#[repr(C)]`, `#[spirit]` proc-macro at `crates/maos-spirit-derive/src/lib.rs`, `Ctx` at `crates/maos-spirit-abi/src/ctx.rs` with `mock()` constructor behind the `mock` feature, `CancellationSignal` trait + `NeverCancel` reference impl, `TokioCancellationSignal` SDK adapter, `__maos_spirit_vtable_<Type>()` symbol per Spirit type, SDK façade re-exports). **This is the surface Story 2.3 builds the template + local runner on top of.**
- Story 2.2 — `cargo xtask check-service-boundary` P1–P4 full implementation + Spirit-ABI type reflection (`check_spirit_abi_types`) + 24 spirit-boundary invariant cases at `tests/corpora/spirit-boundary-v0.1.jsonl`. **AC1 of THIS story re-lands the 2 production code fixes Story 2.2 review reverted** (`crates/maos-bin/src/main.rs` extra `SecurityManagerAdapter` constructions; `crates/maos-kernel-core/src/security/mod.rs` missing `CryptoProvider` re-export). Without AC1, `check-service-boundary` P1+P2 fail on the real workspace.

**Successor stories in Epic 2:** 2.4 (spirit-test SDK seed + LCAS 70-bucket + cross-Spirit isolation hooks). **No further bridge stories anticipated for Epic 2 at this time.**

**Downstream consumer:** Story 7.5b at v0.3 — executes the full NFR-Onb-1 30-Min First Spirit Validation Gate (N=12 stratified) against the Butler reference Spirit shipped by Story 8.1 using the template + local runner + example pattern this story lays down.

## Story

As a **Spirit author working in a 9pm-Tuesday window who has never touched the MAOS kernel internals and needs to build a working binary Spirit on a laptop within 30 minutes** (the Diego J6 onboarding persona from architecture §10.6, the v0.3 NFR-Onb-1 candidate population from PRD line 122 — N=12 stratified ≥4 with no prior MAOS contribution / ≥3 who've never written Rust Spirit / ≥2 who've never written Rust at all / ≥2 non-English-native / ≥1 working offline-only),
I want **(1) a thin `cargo generate`-compatible Rust template at `templates/spirit-rust/` that produces a compilable Spirit crate using the `#[spirit]` proc-macro from Story 2.1 (the template generates a minimal `on_idle` hook + a TOML manifest mirroring the hello-spirit shape + a passing `cargo test` driven by the local runner) and is consumable via `cargo generate --git <maos-repo-url> templates/spirit-rust --name my-spirit` (subfolder form — full crates.io / `cargo install cargo-generate` favorites-alias form deferred to Story 7.1 at v0.5+); (2) a local runner shipped as a new public module `local_runner` inside `crates/maos-spirit-sdk/src/local_runner.rs` (gated behind a new `local_runner` cargo feature that depends on the existing `std` + `mock` features) that exposes `LocalRunner::run(&spirit, &vtable, &fixture) -> RunReport` — it invokes each declared lifecycle hook through the `SpiritVtable<T>` dispatch surface (NOT directly on the Spirit struct — exercising the same dispatch path the kernel will use at Story 5.1) with `Ctx::mock()` as the context, collects emitted IAC frames into an in-memory `MockBus` (forward-anchor type for Story 2.4's full spirit-test SDK harness), times each hook via `std::time::Instant::elapsed()`, and returns a `RunReport` carrying per-hook fire-counts + mock-bus frames + per-hook elapsed wall-clock; the runner has ZERO dependency on `maos-kernel-core` (verifiable via `cargo tree -p maos-spirit-sdk --features local_runner`) so a Spirit author building against `maos-spirit-sdk` never transitively pulls in the kernel; (3) the baked output of the template committed as a new workspace member `examples/example-spirit/` that proves the template generates compiling code, gets `cargo test -p example-spirit` greenlit, and exists as the "≥1 example Spirit with passing CI" the Epic 2 line + NFR-Onb-1 v0.3 prerequisite list requires; (4) an `xtask example-spirit-regen [--check]` sub-command at `xtask/src/example_spirit_regen.rs` that (in default mode) re-renders the template into `examples/example-spirit/` to keep template + baked output in lockstep, and (in `--check` mode) fails CI if the example has drifted from the template — guards against silent template-vs-baked-output divergence; (5) two new jobs in `.github/workflows/discipline.yml` — `example-spirit-tests` (mirrors the existing `hello-spirit-tests` job at line 414 — runs `cargo test -p example-spirit --features maos-spirit-sdk/local_runner`) and `example-spirit-drift` (runs `cargo run -p xtask -- example-spirit-regen --check`) — both appended to the discipline-summary `needs:` list at line 535 and to the PR-comment table at line 620, taking the gate count from 28 → 30; (6) `tests/coverage-matrix.yaml` updated additively — `FR33.gates` (thin cargo-generate slice — Story 2.3 ships, full per-language at Story 7.1) + `FR34.gates` (spirit-test SDK seed — Story 2.3 ships local_runner, full SDK with assertion macros at Story 7.1) + `NFR-Onb-1.gates` (Story 2.3 ships PREREQUISITES — template + local runner + example with passing CI; full N=12 stratified gate execution at Story 7.5b against Butler from Story 8.1); (7) the architecture-doc adjustments — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 layout updated to reflect the new `templates/spirit-rust/` directory + `examples/example-spirit/` workspace member (workspace becomes 21 lib/bin + xtask = 22 members; document `examples/*` as workspace-member but "not part of kernel substrate"), `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5 gains a ≤6-line addendum titled `**v0.3 prerequisite — Spirit-author scaffolding (Story 2.3):**` citing the three surfaces, and `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` gains a top-of-file callout noting v0.3-prerequisite template + runner land in Story 2.3 (NOT v1.0 as the doc currently implies); (8) the discipline gate sweep + cold-cache integration test — all 30 jobs in `discipline.yml` pass locally + via the PR commit's `discipline.yml` run (per A8 retro action), the existing `tests/integration/v01_evaluator_path.sh` passes cold (per A6), `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` reports zero added/changed/removed against `maos-spirit-abi` (the new local_runner module lives in `maos-spirit-sdk` which is NOT scoped by the abi-diff gate — verify scope before claiming gate-green; AC1's re-export addition in `security/mod.rs` may also touch `xtask/kernel-api-classes.toml` and require a kernel-surface baseline refresh per the Story 2.2 precedent at lines 696–702 of the 2.2 dev record); (9) the **pre-flight bridge** as `Task 0` — re-land the 2 production fixes Story 2.2 review explicitly reverted (D1 → bridge story), absorbed into THIS story because no separate bridge story was created**,

so that **(a) the v0.3 NFR-Onb-1 30-Min First Spirit Validation Gate (PRD line 122; architecture §13 row "v0.3 — Butler") becomes EXECUTABLE at Story 7.5b — every prerequisite the gate cites (cargo-generate template + local runner + ≥1 example Spirit with passing CI) is present and CI-enforced; (b) the Diego J6 onboarding journey (architecture §10.6 line 121: *"Diego opens spirit-development-and-sharing.md. Skims to 'Build your first Spirit in 30 minutes.' Runs `cargo generate maos-spirit`, gets a templated project. Imports his existing static-analysis logic."*) becomes operationally reproducible at v0.3 — the documented invocation actually works, end-to-end, against the shipped template; (c) Epic 2's epic-line commitment "Thin `cargo generate maos-spirit` template (Rust only at v0.5; per-language at E7) — enough for NFR-Onb-1 v0.3 gate" is satisfied with the explicit deferral path to Story 7.1 (full per-language) documented in the FR33 + FR34 coverage-matrix notes; (d) Story 7.5b at v0.3 has a working harness substrate to extend at v1.0 — the local_runner SDK seed feeds forward into Story 2.4's full spirit-test SDK with assertion macros + halt resolution + manifest self-check (which then feeds Story 7.1's per-language full SDK at v0.5+); (e) the Story 2.2 review's "bridge story needed before 2.3 to re-land fixes" gap is closed — `cargo xtask check-service-boundary` P1+P2 enforcement (which Story 2.2 shipped but the review reverted the prod-code fixes for) passes on the real workspace, AND the hello-spirit one-shot path produces identical 4-key JSON (regression baseline preserved per A6 cold-cache discipline); (f) the v0.3 sprint dependency-DAG entry (`_bmad-output/planning-artifacts/epics/dependency-dag.md` line 25: `Story 2.3 thin cargo-generate slice → Story 7.5b NFR-Onb-1 v0.3 gate execution`) becomes traversable; (g) the Spirit-author-facing crate stack (`maos-spirit-sdk` + `maos-spirit-derive` + `maos-spirit-abi`) gains its first end-to-end author-side artifact (template → cargo generate → working test pass) — until this story, the surfaces existed but no user-facing flow exercised them top-to-bottom**.

## What this story IS

- **A thin scaffolding template, not a full SDK.** `templates/spirit-rust/` produces a compilable Spirit crate with ONE hook (`on_idle`), a manifest mirroring the hello-spirit shape, a `tests/spirit_smoke.rs` driven by `local_runner`, and a README citing NFR-Onb-1 + Story 7.5b. Per-language templates (TypeScript/Python/Go) are explicitly deferred to **Story 7.1** at v0.5+; the FR33 coverage-matrix note records this deferral.
- **A local runner SDK seed, not the full spirit-test SDK.** `crates/maos-spirit-sdk/src/local_runner.rs` ships `LocalRunner` + `LocalRunnerFixture` + `RunReport` + `MockBus` + `MockBusFrame`. Hook firing through `SpiritVtable` with `Ctx::mock()`. ZERO kernel dep. Full spirit-test SDK with assertion macros + halt resolution + manifest self-check + class-specific regression corpus is **Story 2.4** seed → **Story 7.1** full.
- **The pre-flight bridge absorbed into Task 0.** Story 2.2 review (lines 707 of `2-2-…md`) explicitly reverted 2 production code changes ("D1 → bridge story before 2.3"): the single-`SecurityManagerAdapter`-construction fix in `crates/maos-bin/src/main.rs` + the `pub use maos_domain::ports::CryptoProvider;` re-export in `crates/maos-kernel-core/src/security/mod.rs`. **No separate bridge story was created.** Story 2.3 absorbs both fixes as Task 0 + AC1 (preserving the spirit of "bridge before template work" by ordering Task 0 first). This is structurally parallel to how Story 1b.6 absorbed D9/D10/Doc3 from the Epic 1b retro.
- **An example Spirit committed as a workspace member.** `examples/example-spirit/` becomes the 22nd workspace member (`Cargo.toml [workspace] members`). It is the baked output of the template — proof the template generates compiling code — and is the substrate the new `example-spirit-tests` CI job exercises.
- **A template-vs-baked-output drift detector.** `xtask example-spirit-regen [--check]` re-renders the template into `examples/example-spirit/` (default mode) or fails CI on drift (`--check` mode). Without this gate, the template and the baked output silently diverge as the SDK evolves; with it, drift fails CI in `example-spirit-drift`.
- **Two new discipline jobs.** `example-spirit-tests` (mirrors `hello-spirit-tests` at line 414) and `example-spirit-drift` (drift detector). Both appended to `needs:` at line 535 + PR-comment table at line 620. Total job count: 28 → 30.
- **Additive coverage-matrix updates.** `FR33`, `FR34`, `NFR-Onb-1` rows gain `gates:` + `notes:` cross-references. No phase changes (`FR33.phase = v0.3` already; `FR34.phase = v0.3` already; `NFR-Onb-1.phase = v0.3` already).
- **Minimal architecture-doc adjustments.** §4.0.2 layout: 1 line for `templates/spirit-rust/` + 1 line for `examples/example-spirit/`; member-count bump 20 → 21 lib/bin + xtask = 22 total. §5 Spirit ABI: ≤6-line v0.3 addendum citing the three Spirit-author surfaces. `spirit-development-and-sharing.md`: top-of-file callout for v0.3 template path. Mirrors the **D10 catch-up pattern** from Story 1b.6 + Story 2.1 — small, in-PR, non-rewrite.
- **CI gate adherence: all 30 jobs green.** Particular attention: `check-service-boundary` (now P1+P2 enforcement against real workspace per AC1), `check-empty-kernel` (no new persistent I9-violating state; local_runner is stateless), `abi-diff` (additive `maos-spirit-sdk` surface only — abi-diff scopes to `maos-spirit-abi`, NOT `maos-spirit-sdk`; verify before claiming green), `check-unsafe` (no new unsafe outside the existing allowlist; local_runner is `#![forbid(unsafe_code)]`), `manifest-field-coverage` (the template's manifest fragments must not introduce un-fixtured manifest fields — verify by running the gate against the template's `manifest.toml` shape; if the template uses ONLY existing fields, no new fixtures needed), `coverage-matrix` (4 row updates per AC6), `kloc-check` (the new local_runner + xtask sub-command grow `maos-spirit-sdk` + `xtask` KLOC — verify against `xtask/kloc.toml` budgets; the Story 2.2 dev record at line 657 raised xtask 3000 → 4000 — local_runner addition to sdk likely stays within sdk budget but verify).

## What this story is NOT

- **NOT** the full `cargo generate` per-language template surface. Rust only at v0.3 prerequisite; TypeScript/Python/Go land at **Story 7.1** (epic 7) at v0.5+.
- **NOT** the full spirit-test SDK with assertion macros (e.g., `assert_emits_frame!`, `assert_halts_with!`). Local runner is a hook-firing harness; assertion ergonomics belong to **Story 2.4** (seed) → **Story 7.1** (full).
- **NOT** the cross-Spirit memory isolation framework hooks (NFR-Sec-14 — **Story 2.4**).
- **NOT** the LCAS framework or the 70-of-210 clearly-decidable bucket (those belong to **Story 2.4** per Epic 2 epic line).
- **NOT** the actual NFR-Onb-1 30-Min First Spirit Validation Gate execution. The gate runs at **Story 7.5b** at v0.3 against the Butler reference Spirit from **Story 8.1**. Story 2.3 ships PREREQUISITES.
- **NOT** the `cargo generate maos-spirit` favorites-alias form. At v0.3 prerequisite the supported invocation is `cargo generate --git <repo-url> templates/spirit-rust --name <name>` (subfolder form) — full registry alias publishing belongs to Story 7.1 + ADR-008 (Spirit registry).
- **NOT** a migration of `crates/maos-spirit-hello/` to use the template. Hello-spirit stays as a hand-written reference Spirit (already documented in Story 2.1 dev notes at line 36). The example Spirit at `examples/example-spirit/` is the FIRST template-derived Spirit; Butler (Story 8.1) is the second.
- **NOT** a new ADR. Reuses ADR-002 (Spirit form at v0.1 — subprocess only, inproc gated on measurement), ADR-008 (Spirit registry), ADR-032 (Spirit wire protocol bytes-on-wire). The proc-macro precedent ADR is implicit (`maos-attrs` + `maos-spirit-derive` per Story 2.1 D10 doc).
- **NOT** a runtime kernel instantiation. The local_runner does NOT spin up `maos-bin`, `SecurityManagerAdapter`, `CapabilityRegistryAdapter`, or any other kernel component. Hooks fire against `Ctx::mock()` + in-memory `MockBus`. Full kernel-side hook firing ships at **Story 5.1**.
- **NOT** the `output_shape` predicate runtime enforcement at frame-emit. The template's manifest declares `[output_shape] required_fields = ["introduction"]`, but at v0.3 prerequisite the local_runner does NOT validate emitted frames against this predicate. Frame-emit fail-loud enforcement ships at **Story 7.3** (CCAC envelope ship gate).
- **NOT** an ABI break. `maos-spirit-abi`'s public surface is NOT touched (verified by `cargo run -p xtask -- abi-diff --json` — reports zero added/changed/removed against `abi-baseline/v1-pre-bump.txt`). `ABI_VERSION` stays at `1`.
- **NOT** a CI workflow restructure. `discipline.yml`'s shape (28 → 30 jobs) is the only change; no top-level workflow file is added/removed. No matrix expansion. No new runners.
- **NOT** a Spirit registry publish path (`maos-spirit publish`). The template scaffolds a local crate; publishing to a registry is Story 7.2 (Spirit Ecosystem at v0.5+).
- **NOT** template-variable extensibility beyond `{{ crate_name }}` + `{{ class_name }}`. Author + license + provider + posture customization at template-generate time is **Story 7.1** scope. v0.3 prerequisite ships the minimal 2-variable shape.
- **NOT** a binding ADR for `examples/*` as a workspace convention. The example crate is added to workspace members per existing precedent (the workspace already accepts non-`crates/*` paths — `xtask` is at the root, not under `crates/`); the §4.0.2 doc addendum is one paragraph clarifying the convention, NOT a rewrite of layout rules.

## Acceptance Criteria

### AC1 — Pre-flight bridge: re-land the 2 production code fixes Story 2.2 review reverted

**Given** the Story 2.2 review section at `_bmad-output/implementation-artifacts/2-2-…md` line 707 (`[Review][Decision] Production code changes reverted (D1 → bridge story)`) explicitly reverted 2 production code changes — restoring `crates/maos-bin/src/main.rs` and `crates/maos-kernel-core/src/security/mod.rs` to their pre-2.2 state — with the closing note (line 735): *"Remaining before Story 2.3: Bridge story must re-land (1) single `SecurityManagerAdapter` construction in `main.rs` and (2) `CryptoProvider` re-export in `security/mod.rs`. These cause expected P1/P2/surface-diff violations until fixed."*
**And** no separate bridge story was created between Story 2.2's merge (commit `9624dbe`) and this story's opening
**And** the current state of `crates/maos-bin/src/main.rs` (verified via `grep -n "SecurityManagerAdapter::new\|SecurityManagerAdapter::default" crates/maos-bin/src/main.rs`) shows THREE construction sites: line 86 (`let _security = SecurityManagerAdapter::default();`), line 122 (`let _security = SecurityManagerAdapter::new(Arc::clone(&policy));`), and line 318 (`let security = maos_kernel_core::security::SecurityManagerAdapter::new(Arc::clone(&policy))` inside the `MAOS_ONE_SHOT=hello-spirit` arm)
**And** the current state of `crates/maos-kernel-core/src/security/mod.rs` (verified via `grep -n "CryptoProvider" crates/maos-kernel-core/src/security/mod.rs`) re-exports `RingCryptoProvider` (line 19) but does NOT re-export `CryptoProvider` (the `Port` trait), which fails P2's `RingCryptoProvider` adapter / `CryptoProvider` Port pairing check (Story 2.2 review's special-case workaround at `xtask/src/check_service_boundary.rs:1581-1585` papered over this — see DF list at line 723 of 2.2 dev record)

**When** the dev agent opens Story 2.3 and executes Task 0

**Then** `crates/maos-bin/src/main.rs` constructs `SecurityManagerAdapter` exactly ONCE — the dead `SecurityManagerAdapter::default()` at line 86 is REMOVED, the unused `SecurityManagerAdapter::new(Arc::clone(&policy))` at line 122 is REMOVED, and the only remaining construction is the one inside the `MAOS_ONE_SHOT=hello-spirit` block at line 318 (which is the one the one-shot evaluator path actually uses),
**And** `crates/maos-kernel-core/src/security/mod.rs` gains `pub use maos_domain::ports::CryptoProvider;` APPENDED to the existing re-export block (preserving the Story 1b.5c re-export-order discipline cited at lines 21-25 of the file — "appended to preserve original re-export order so the `signature_hash` of each existing symbol remains stable under `check-service-boundary`'s use-item hashing"),
**And** `cargo run -p xtask -- check-service-boundary --json` exits 0 against the real v0.1-β workspace (P1 enforced: SecurityManagerAdapter constructed once; P2 enforced: RingCryptoProvider adapter pairs with CryptoProvider Port trait; P3 + P4 + Spirit ABI reflection stay enforced per Story 2.2),
**And** the Story 2.2 special-case workaround at `xtask/src/check_service_boundary.rs:1581-1585` (the inline `if adapter == "RingCryptoProvider"`) is either REMOVED (preferred — the workaround is no longer needed once the Port trait is re-exported; the Port-pair check finds the trait naturally) OR retained with a one-line comment noting the workaround is now redundant; the dev agent's choice is captured in the dev record's "Bridge fix" section,
**And** `MAOS_ONE_SHOT=hello-spirit cargo run -p maos-bin` produces identical 4-key JSON output (the `introduction`, `capability_scope`, `halt_tags`, `transparency_log` shape — verified against `tests/integration/v01_evaluator_path.sh` step 2),
**And** `tests/integration/v01_evaluator_path.sh` passes cold (per A6 retro action — `cargo clean -p maos-bin && ./tests/integration/v01_evaluator_path.sh`),
**And** if removing the lines 86 + 122 constructions causes any other `_security`-using code in `main.rs` to break (e.g., the `_security` binding is referenced later — verify by reading the surrounding 20 lines of context), the dev agent restructures the dead bindings cleanly (preferred: delete unused `let _` bindings; alternative: re-order so the line 318 construction is hoisted earlier if a real consumer exists),
**And** `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` reports zero added/changed/removed against `maos-spirit-abi` (`security/mod.rs` re-export is in `maos-kernel-core`, NOT `maos-spirit-abi` — the abi-diff gate scopes to `maos-spirit-abi`; if the kernel-surface baseline at `docs/ci-baselines/kernel-surface-v0.1-beta.json` needs refresh, follow the Story 2.2 dev record's pattern at lines 696-702 — regenerate the baseline + add `CryptoProvider` to `xtask/kernel-api-classes.toml` with classification `ports`),
**And** the bridge fix is committed as the FIRST commit of the Story 2.3 PR (preserves the "bridge before template work" spirit; the commit message follows the existing precedent — `fix: re-land Story 2.2 reverted production fixes (SecurityManagerAdapter dedupe + CryptoProvider re-export)`).

### AC2 — Thin `cargo-generate` Rust template at `templates/spirit-rust/`

**Given** the `cargo-generate` tool (`cargo install cargo-generate`; documented at https://github.com/cargo-generate/cargo-generate)
**And** Liquid-template syntax (`{{ var }}` interpolation, `{{ var | upper_case }}` filters) is the default placeholder convention cargo-generate processes inline on file contents
**And** the workspace root `Cargo.toml` declares `[workspace] members = [...]` with explicit listing (NO globs) and `default-members = []` (per A7 retro action) — meaning `templates/spirit-rust/Cargo.toml` will NOT be auto-discovered as a workspace member unless explicitly listed
**And** the existing hello-spirit manifest at `spirits/hello-spirit/manifest.toml` is the canonical reference shape for `[class]` / `[capabilities.required]` / `[posture]` / `[output_shape]` / `[budget]` / `[resources]` / `[sandbox]` / `[author]` sections
**And** the existing `crates/maos-spirit-hello/src/lib.rs` is the v0.1-α hand-written Spirit reference (Story 2.3 does NOT migrate it; the template is for NEW Spirits)

**When** the dev agent authors `templates/spirit-rust/` with the file set below

**Then** the directory contains exactly these files (template flat layout — cargo-generate-compatible):
```
templates/
└── spirit-rust/
    ├── cargo-generate.toml          # cargo-generate metadata + placeholder declarations
    ├── Cargo.toml                   # contains {{crate_name}} placeholder
    ├── src/
    │   └── lib.rs                   # contains {{class_name}} placeholder, uses #[spirit]
    ├── manifest.toml                # mirrors hello-spirit shape, parameterized by {{crate_name}}
    ├── README.md                    # 30-min first-Spirit path docs; cites NFR-Onb-1 + Story 7.5b
    └── tests/
        └── spirit_smoke.rs          # local_runner-driven hook fire + report assertion
```
**And** the workspace root `Cargo.toml` gains `[workspace] exclude = ["templates"]` to keep cargo from discovering the template's `Cargo.toml` as a member (verify behavior: `cargo metadata --no-deps --format-version 1 | jq '.workspace_members[]'` should NOT include the template; if `exclude = []` is insufficient, alternative is renaming template files with `.liquid` suffix and adding a cargo-generate file-rename rule, but `exclude` is the cleaner v0.3 approach),
**And** `templates/spirit-rust/cargo-generate.toml` declares:
```toml
[template]
cargo_generate_version = ">=0.18.0"
ignore = []

[placeholders]
crate_name = { type = "string", prompt = "Spirit crate name (kebab-case, e.g., 'my-spirit')", regex = "^[a-z][a-z0-9-]+$" }
class_name = { type = "string", prompt = "Spirit struct name (PascalCase, e.g., 'MySpirit')", regex = "^[A-Z][a-zA-Z0-9]+$" }
```
**And** `templates/spirit-rust/Cargo.toml` declares (exact bytes, with `{{crate_name}}` placeholder):
```toml
[package]
name = "{{crate_name}}"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0 OR MIT"
description = "A MAOS Spirit scaffolded from templates/spirit-rust (Story 2.3 v0.3 prerequisite)."

[dependencies]
# At v0.3 prerequisite, maos-spirit-sdk is not yet on crates.io. Spirit
# authors run this template from a local clone or `--git` checkout. The
# example-spirit baked output in this workspace uses a path dep; published
# templates will pin to a crates.io version once Story 7.1 ships.
maos-spirit-sdk = { git = "https://github.com/lunarpulse/maos", tag = "v0.1-template-seed", features = ["local_runner"] }

[dev-dependencies]
maos-spirit-sdk = { git = "https://github.com/lunarpulse/maos", tag = "v0.1-template-seed", features = ["local_runner", "mock"] }
```
**And** `templates/spirit-rust/src/lib.rs` declares (exact bytes, with `{{class_name}}` placeholder):
```rust
#![forbid(unsafe_code)]

//! {{crate_name}} — a MAOS Spirit scaffolded from templates/spirit-rust.
//!
//! Edit `on_idle` to implement your Spirit's idle-time behavior. See
//! README.md for the 30-minute first-Spirit path.

use maos_spirit_sdk::{spirit, Ctx};

pub struct {{class_name}};

#[spirit]
impl {{class_name}} {
    fn on_idle(&self, ctx: &mut Ctx) {
        // Bail early if the kernel has signaled cancellation.
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // TODO: implement your Spirit's idle behavior here.
    }
}
```
**And** `templates/spirit-rust/manifest.toml` mirrors the hello-spirit shape (substituting `{{crate_name}}`):
```toml
[class]
name = "{{crate_name}}"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 1
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "A MAOS Spirit scaffolded from templates/spirit-rust."

[capabilities.required]
provider.complete = ["anthropic.claude-3-haiku-20240307"]

[posture]
default = "assistive"
allowed_max = "assistive"

[output_shape]
required_fields = ["introduction"]

[budget]
context_window_size = 4096
time_cap_seconds = 30

[resources]
cpu_max_pct = 10
memory_max_mb = 64

[sandbox]
tier = "T0"

[author]
name = "TODO: your name"
```
**And** `templates/spirit-rust/tests/spirit_smoke.rs` declares (exact bytes, with `{{class_name}}` placeholder):
```rust
//! Smoke test for {{class_name}} — fires `on_idle` through the local_runner
//! and asserts the hook fired exactly once.

use {{crate_name | snake_case}}::{{class_name}};
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

#[test]
fn on_idle_fires_once() {
    let spirit = {{class_name}};
    let vtable = __maos_spirit_vtable_{{class_name}}();
    let fixture = LocalRunnerFixture {
        invoke_on_idle: true,
        ..Default::default()
    };
    let report = LocalRunner::run(&spirit, vtable, &fixture);
    assert_eq!(report.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
}
```
**And** `templates/spirit-rust/README.md` documents the 30-minute first-Spirit path with at minimum these sections: "Build your first Spirit in 30 minutes" (the literal phrasing from `spirit-development-and-sharing.md` line 121 + architecture §10.6), "How to run", "What `#[spirit]` derives for you" (cites Story 2.1 for the macro contract), "How to extend" (cites the 11-hook list at `crates/maos-spirit-abi/src/lifecycle.rs` lines 134-180), "What ships at v0.3 vs. later" (explicit: Story 2.4 ships full spirit-test SDK seed, Story 7.1 ships per-language templates, Story 7.5b runs the NFR-Onb-1 30-min gate — link by story key not URL), "Status of this template" (one paragraph: "v0.3 prerequisite per Story 2.3; full per-language at Story 7.1"),
**And** the README explicitly cites NFR-Onb-1 by name + the 30-min/45-min-median/90-min-p95 figure (PRD line 122 wording) AND notes the full gate runs at Story 7.5b against the Butler reference Spirit from Story 8.1 (the dev agent's job is to set the right expectation: this template is the SUBSTRATE for the gate, not the gate itself),
**And** the dev agent verifies the template by manually scaffolding it: `cargo install cargo-generate --version "^0.21"` (or whichever version is current at story open; pin in the dev record) + `cd /tmp && cargo generate --path <maos-repo>/templates/spirit-rust --name testflight-spirit` + `cd testflight-spirit && cargo test --features maos-spirit-sdk/local_runner` PASSES (the smoke test asserts `on_idle` fired exactly once),
**And** the dev record's "Template smoke" section captures the exact commands + the produced directory tree of the testflight-spirit output (defense-in-depth: the example crate at `examples/example-spirit/` is the COMMITTED baked output; this verification is an INDEPENDENT external check).

### AC3 — `local_runner` module + fixture types in `maos-spirit-sdk`

**Given** the existing `crates/maos-spirit-sdk/src/lib.rs` (façade re-exports from `maos-spirit-abi` + `maos-spirit-derive`; `pub mod cancellation` for `TokioCancellationSignal`)
**And** the existing `crates/maos-spirit-sdk/Cargo.toml` with `[features] default = ["std"]; std = ["dep:tokio-util"]; mock = ["maos-spirit-abi/mock"]`
**And** the existing `Ctx::mock()` constructor at `crates/maos-spirit-abi/src/ctx.rs:67` gated behind `#[cfg(any(test, feature = "mock"))]`
**And** the `SpiritVtable<T>` struct + `__maos_spirit_vtable_<Type>()` symbol generated by `#[spirit]` per Story 2.1
**And** the 11-hook trait at `crates/maos-spirit-abi/src/lifecycle.rs:138-182` with payload types (`FramePayload`, `TelemetryEventPayload`, `SchedulePayload`, `SwapInPayload`, `ConsolidatePayload`)
**And** the design constraint: the local runner MUST NOT depend on `maos-kernel-core` (verified via `cargo tree -p maos-spirit-sdk --features local_runner --no-default-features --features local_runner,std,mock` — `maos-kernel-core` must NOT appear in the dep graph)

**When** the dev agent adds `crates/maos-spirit-sdk/src/local_runner.rs` and the supporting fixture + report types

**Then** `crates/maos-spirit-sdk/src/local_runner.rs` declares:
```rust
#![forbid(unsafe_code)]

//! `local_runner` — fires Spirit lifecycle hooks through the SpiritVtable
//! using a mock Ctx + in-memory mock IAC bus. Zero kernel dependency.
//!
//! Per Story 2.3 (v0.3 NFR-Onb-1 prerequisite): the runner is the substrate
//! Spirit authors test their Spirits against without spinning up a real
//! kernel. Full spirit-test SDK with assertion macros + halt resolution +
//! manifest self-check + class-specific regression corpus is Story 2.4 seed
//! → Story 7.1 full.

use crate::{Ctx, Spirit, SpiritVtable};
use crate::{ConsolidatePayload, FramePayload, SchedulePayload, SwapInPayload, TelemetryEventPayload};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Fixture describing which hooks to fire and with what payloads.
#[derive(Debug, Clone, Default)]
pub struct LocalRunnerFixture {
    pub invoke_on_load: bool,
    pub invoke_on_start: bool,
    pub invoke_on_idle: bool,
    pub invoke_on_pause: bool,
    pub invoke_on_resume: bool,
    pub invoke_on_unload: bool,
    /// Each entry fires one `on_frame` invocation.
    pub frames: Vec<Vec<u8>>,
    /// Each entry fires one `on_telemetry_event` invocation.
    pub telemetry_events: Vec<Vec<u8>>,
    /// Each entry fires one `on_schedule` invocation.
    pub schedule_payloads: Vec<Vec<u8>>,
    /// Each entry fires one `on_swap_in` invocation.
    pub swap_in_payloads: Vec<Vec<u8>>,
    /// Each entry fires one `on_consolidate` invocation.
    pub consolidate_payloads: Vec<Vec<u8>>,
}

/// Forward-anchor type for Story 2.4 full spirit-test SDK. At v0.3
/// prerequisite the runner does NOT actually capture frames from Spirit
/// emits (Spirits have no real capability handles in the mock Ctx, so
/// they cannot emit). The MockBusFrame type exists so Story 2.4 can
/// extend the runner without breaking the LocalRunnerFixture / RunReport
/// public surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockBusFrame {
    pub kind: MockBusFrameKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockBusFrameKind {
    /// Reserved for Story 2.4 — IAC frame the Spirit attempted to send.
    Send,
    /// Reserved for Story 2.4 — capability invocation the Spirit attempted.
    CapInvoke,
}

/// Report from a `LocalRunner::run` invocation.
#[derive(Debug, Clone, Default)]
pub struct RunReport {
    /// hook-name → fire count.
    pub hooks_fired: BTreeMap<String, u32>,
    /// Empty at v0.3 prerequisite; populated by Story 2.4's full SDK.
    pub mock_bus_frames: Vec<MockBusFrame>,
    /// hook-name → elapsed wall-clock for that hook's invocations.
    pub elapsed_per_hook: BTreeMap<String, Duration>,
}

/// The local runner — instantiate with no arguments (it's stateless).
pub struct LocalRunner;

impl LocalRunner {
    /// Run the fixture against the Spirit through its vtable. Returns
    /// a report carrying per-hook fire counts, accumulated elapsed
    /// wall-clock, and (forward-anchor) mock bus frames.
    pub fn run<S: Spirit>(
        spirit: &S,
        vtable: &SpiritVtable<S>,
        fixture: &LocalRunnerFixture,
    ) -> RunReport {
        let mut report = RunReport::default();
        let mut ctx = Ctx::mock();

        macro_rules! fire {
            ($name:expr, $expr:expr) => {{
                let start = Instant::now();
                $expr;
                let elapsed = start.elapsed();
                *report.hooks_fired.entry($name.to_string()).or_insert(0) += 1;
                *report.elapsed_per_hook.entry($name.to_string()).or_insert(Duration::ZERO) += elapsed;
            }};
        }

        if fixture.invoke_on_load {
            fire!("on_load", (vtable.on_load)(spirit, &mut ctx));
        }
        if fixture.invoke_on_start {
            fire!("on_start", (vtable.on_start)(spirit, &mut ctx));
        }
        for bytes in &fixture.frames {
            let p = FramePayload { frame_data: bytes.as_slice(), frame_len: bytes.len() };
            fire!("on_frame", (vtable.on_frame)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_idle {
            fire!("on_idle", (vtable.on_idle)(spirit, &mut ctx));
        }
        for bytes in &fixture.telemetry_events {
            let p = TelemetryEventPayload { event_data: bytes.as_slice(), event_len: bytes.len() };
            fire!("on_telemetry_event", (vtable.on_telemetry_event)(spirit, &mut ctx, &p));
        }
        for bytes in &fixture.schedule_payloads {
            let p = SchedulePayload { schedule_data: bytes.as_slice(), schedule_len: bytes.len() };
            fire!("on_schedule", (vtable.on_schedule)(spirit, &mut ctx, &p));
        }
        for bytes in &fixture.swap_in_payloads {
            let p = SwapInPayload { predecessor_state: bytes.as_slice(), state_len: bytes.len() };
            fire!("on_swap_in", (vtable.on_swap_in)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_pause {
            fire!("on_pause", (vtable.on_pause)(spirit, &mut ctx));
        }
        if fixture.invoke_on_resume {
            fire!("on_resume", (vtable.on_resume)(spirit, &mut ctx));
        }
        for bytes in &fixture.consolidate_payloads {
            let p = ConsolidatePayload { batch_data: bytes.as_slice(), batch_len: bytes.len() };
            fire!("on_consolidate", (vtable.on_consolidate)(spirit, &mut ctx, &p));
        }
        if fixture.invoke_on_unload {
            fire!("on_unload", (vtable.on_unload)(spirit, &mut ctx));
        }

        report
    }
}
```
**And** `crates/maos-spirit-sdk/Cargo.toml` gains the `local_runner` feature:
```toml
[features]
default = ["std"]
std = ["dep:tokio-util"]
mock = ["maos-spirit-abi/mock"]
local_runner = ["std", "mock"]
```
**And** `crates/maos-spirit-sdk/src/lib.rs` gains `#[cfg(feature = "local_runner")] pub mod local_runner;` APPENDED to the existing module declarations (do NOT reorder — preserve façade re-export order for `check-service-boundary` signature-hash stability),
**And** `crates/maos-spirit-sdk/tests/local_runner_smoke.rs` is created with this exact shape (gated `#[cfg(feature = "local_runner")]` so the test compiles only when the feature is enabled):
```rust
#![cfg(feature = "local_runner")]

//! Smoke test for local_runner — fires on_idle + on_frame through
//! a #[spirit]-derived TestSpirit and asserts the report shape.

use maos_spirit_sdk::spirit;
use maos_spirit_sdk::{Ctx, Spirit};
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

pub struct TestSpirit;

#[spirit]
impl TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {}
}

#[test]
fn on_idle_fires_once() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let f = LocalRunnerFixture { invoke_on_idle: true, ..Default::default() };
    let r = LocalRunner::run(&s, v, &f);
    assert_eq!(r.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
    assert_eq!(r.hooks_fired.get("on_frame").copied().unwrap_or(0), 0);
    assert!(r.mock_bus_frames.is_empty(), "v0.3 prerequisite: no frames expected");
    assert!(r.elapsed_per_hook.contains_key("on_idle"));
}

#[test]
fn frames_fire_per_entry() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let f = LocalRunnerFixture {
        frames: vec![b"f0".to_vec(), b"f1".to_vec(), b"f2".to_vec()],
        ..Default::default()
    };
    let r = LocalRunner::run(&s, v, &f);
    assert_eq!(r.hooks_fired.get("on_frame").copied().unwrap_or(0), 3);
}

#[test]
fn report_default_is_empty() {
    let r = maos_spirit_sdk::local_runner::RunReport::default();
    assert!(r.hooks_fired.is_empty());
    assert!(r.mock_bus_frames.is_empty());
    assert!(r.elapsed_per_hook.is_empty());
}
```
**And** `cargo tree -p maos-spirit-sdk --features local_runner --edges normal,build` does NOT contain `maos-kernel-core` anywhere in the dep graph (assertion: the local runner is a Spirit-author-facing surface; pulling kernel-core would couple every third-party Spirit's compile to the kernel — verified before opening PR; if the dep accidentally appears, the dev agent investigates which crate transitively pulls it and prunes — likely candidate: `mock` feature accidentally enabling something kernel-side),
**And** `cargo test -p maos-spirit-sdk --features local_runner --test local_runner_smoke` PASSES (3 tests at minimum: `on_idle_fires_once`, `frames_fire_per_entry`, `report_default_is_empty`),
**And** `cargo build -p maos-spirit-sdk --no-default-features` continues to succeed (no_std parity preserved — local_runner is opt-in, not default-on),
**And** `cargo build -p maos-spirit-sdk --no-default-features --features local_runner` fails clearly because `local_runner = ["std", "mock"]` — verify the error message is a clean feature-resolution error (not a missing-symbol crash) so authors get a clear signal,
**And** `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` reports zero added/changed/removed against `maos-spirit-abi` (the abi-diff gate scopes to the ABI crate; local_runner lives in `maos-spirit-sdk` which is OUTSIDE the gate's scope — verify this scoping is correct by inspecting `xtask/src/abi_diff.rs` if any doubt; the Story 2.1 dev record at line 158 cites `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` as the canonical local invocation).

### AC4 — Example Spirit baked at `examples/example-spirit/`

**Given** the template authored at AC2
**And** the workspace root `Cargo.toml` listing 20 lib/bin crates + xtask = 21 members (post Story 2.1)
**And** the convention that `examples/*` is acceptable as a workspace member but is "not part of kernel substrate" (to be documented in §4.0.2 addendum per AC7)
**And** the local_runner module shipped at AC3

**When** the dev agent renders the template into `examples/example-spirit/` and registers it as a workspace member

**Then** `examples/example-spirit/` exists with this structure (the baked output of `cargo generate --path templates/spirit-rust --name example-spirit` with placeholders resolved):
```
examples/
└── example-spirit/
    ├── Cargo.toml         # name = "example-spirit", PATH dep on maos-spirit-sdk (NOT git)
    ├── src/
    │   └── lib.rs         # struct ExampleSpirit + #[spirit] impl with on_idle
    ├── manifest.toml      # [class].name = "example-spirit", T0 sandbox, hello-spirit shape
    ├── README.md          # cites template + regen workflow
    └── tests/
        └── spirit_smoke.rs  # local_runner-driven on_idle fire + assertion
```
**And** `examples/example-spirit/Cargo.toml` swaps the template's `git = "..."` dep with a workspace-relative path (the example IS the workspace; pinning to a git tag would be circular):
```toml
[package]
name = "example-spirit"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0 OR MIT"
description = "MAOS Spirit baked from templates/spirit-rust (Story 2.3 v0.3 prerequisite proof)."

[dependencies]
maos-spirit-sdk = { path = "../../crates/maos-spirit-sdk", features = ["local_runner"] }

[dev-dependencies]
maos-spirit-sdk = { path = "../../crates/maos-spirit-sdk", features = ["local_runner", "mock"] }
```
**And** the workspace root `Cargo.toml` is updated:
```toml
[workspace]
resolver = "2"
members = [
    "xtask",
    "crates/maos-a2a",
    # ... existing 20 lib/bin crates ...
    "crates/maos-spirit-derive",
    "examples/example-spirit",  # ← NEW, last; preserves member ordering
]
default-members = []
exclude = ["templates"]  # ← NEW, prevents cargo from auto-discovering templates/spirit-rust/Cargo.toml
```
**And** `examples/example-spirit/src/lib.rs` is the template's `src/lib.rs` with placeholders resolved (`{{crate_name}}` → `example-spirit` in module path / package name; `{{class_name}}` → `ExampleSpirit` in struct + macro symbol):
```rust
#![forbid(unsafe_code)]

//! example-spirit — a MAOS Spirit scaffolded from templates/spirit-rust.

use maos_spirit_sdk::{spirit, Ctx};

pub struct ExampleSpirit;

#[spirit]
impl ExampleSpirit {
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        // TODO: implement your Spirit's idle behavior here.
    }
}
```
**And** `examples/example-spirit/tests/spirit_smoke.rs` invokes `LocalRunner` on the baked Spirit:
```rust
//! Smoke test for ExampleSpirit — fires on_idle + asserts report.

use example_spirit::ExampleSpirit;
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};

#[test]
fn on_idle_fires_once() {
    let spirit = ExampleSpirit;
    let vtable = __maos_spirit_vtable_ExampleSpirit();
    let fixture = LocalRunnerFixture { invoke_on_idle: true, ..Default::default() };
    let report = LocalRunner::run(&spirit, vtable, &fixture);
    assert_eq!(report.hooks_fired.get("on_idle").copied().unwrap_or(0), 1);
}
```
**And** `examples/example-spirit/manifest.toml` mirrors the hello-spirit manifest shape with `[class].name = "example-spirit"`,
**And** `examples/example-spirit/README.md` carries exactly this content (mirror the regeneration discipline):
```markdown
# example-spirit

This is the baked output of `cargo generate --path templates/spirit-rust --name example-spirit`.

It is committed as a workspace member so the discipline-suite job
`example-spirit-tests` continuously proves the template produces compiling
code as the SDK + ABI evolve.

## Regeneration

To re-render this directory from the template after the template changes:

```
cargo run -p xtask -- example-spirit-regen
```

To verify in CI that the baked output has not drifted from the template:

```
cargo run -p xtask -- example-spirit-regen --check
```

The `example-spirit-drift` discipline job runs this on every PR.

## Status

This is the Story 2.3 v0.3 NFR-Onb-1 PREREQUISITE proof artifact. It is
NOT the Butler reference Spirit (that ships in Story 8.1) and NOT the
NFR-Onb-1 gate itself (that runs at Story 7.5b). It exists to validate
the template + local runner end-to-end.
```
**And** `cargo build -p example-spirit` succeeds with zero warnings beyond the workspace pre-existing baseline (the new diagnostics listed in the system reminder at story-creation time — `lib.rs` unused `reason`, `anthropic.rs` unused `InferenceOptions`, `inference.rs` unused `Vec`, `check_corpus_integration.rs` unused fixture, etc. — are PRE-EXISTING per the Story 2.2 dev record at line 729 noting "1 pre-existing unused import warning"; Story 2.3 does NOT add new warnings and MUST verify by running `cargo build --workspace 2>&1 | grep -c warning` before vs. after AC4 and asserting non-regression),
**And** `cargo test -p example-spirit` PASSES (the smoke test asserts `on_idle` fired exactly once through the vtable),
**And** the example crate carries `#![forbid(unsafe_code)]` (matching the template + maos-spirit-sdk discipline),
**And** the example uses `#[spirit] impl ExampleSpirit { fn on_idle(&self, ctx: &mut Ctx) { ... } }` — NOT a hand-rolled trait impl (the explicit goal: prove the macro path; hand-rolled would defeat the AC),
**And** the example's `Cargo.toml` does NOT declare `[lib] proc-macro = true` (it's a regular library, not a proc-macro crate),
**And** the dev agent verifies the example end-to-end by `cargo clean -p example-spirit && cargo test -p example-spirit` PASSES cold (per A6 retro action).

### AC5 — `xtask example-spirit-regen [--check]` drift detector

**Given** the template at `templates/spirit-rust/` (AC2) and the baked output at `examples/example-spirit/` (AC4)
**And** the design constraint that the template and baked output MUST stay in lockstep (silent drift would invalidate the "proof the template generates compiling code" claim)
**And** the existing xtask sub-command pattern at `xtask/src/main.rs` + the existing `check_*` modules at `xtask/src/check_*.rs`
**And** the existing `xtask/Cargo.toml` deps (`syn = "2"`, `quote`, `serde_json`, `toml = "0.8"`, `sha2 = "0.10"`)

**When** the dev agent adds `xtask/src/example_spirit_regen.rs` and wires it through `xtask/src/main.rs`

**Then** `xtask/src/example_spirit_regen.rs` exports `pub fn run(workspace_root: &Path, check_mode: bool) -> Result<(), String>` that:
- Reads `templates/spirit-rust/cargo-generate.toml` (parses with the existing `load_toml` helper) and extracts the `[placeholders]` table for the `crate_name` + `class_name` rules.
- Reads every file under `templates/spirit-rust/` (recursive walk; skip `cargo-generate.toml`) into a `BTreeMap<RelPath, String>`.
- Applies placeholder substitutions: `{{crate_name}}` → `"example-spirit"`, `{{class_name}}` → `"ExampleSpirit"`, `{{crate_name | snake_case}}` → `"example_spirit"`. The substitution implementation uses simple `str::replace` (NOT a full Liquid engine — v0.3 prerequisite can hardcode the 3 substitutions the template uses; if cargo-generate's filter syntax expands in Story 7.1, the engine grows then).
- Additionally substitutes the `Cargo.toml` dep declaration from `git = "https://github.com/lunarpulse/maos", tag = "v0.1-template-seed"` → `path = "../../crates/maos-spirit-sdk"` (the example is workspace-relative; rendered output must match `examples/example-spirit/Cargo.toml`).
- In default mode (`check_mode = false`): writes the rendered tree into `examples/example-spirit/` (overwriting existing files; preserve the README's "Regeneration" + "Status" sections by NOT overwriting README.md — render README from template ONLY if `examples/example-spirit/README.md` does not already exist; the example's README is intentionally divergent from the template's README and SHOULD NOT be regenerated).
- In `--check` mode (`check_mode = true`): renders into a `tempfile::TempDir` (using the existing `tempfile = "3"` dev-dep — verify it's already in `xtask/Cargo.toml`; if not, add to `[dependencies]` since this is a regular xtask sub-command, not a test), then walks `examples/example-spirit/` and the rendered tempdir in parallel, comparing each file's bytes. On any mismatch, return `Err(format!("drift: <file>: rendered output differs from committed example-spirit"))`. Exclude `README.md` from the comparison (intentional divergence per the default-mode rule above).
- Returns `Ok(())` on success.
**And** `xtask/src/main.rs` gains a new CLI sub-command `example-spirit-regen` with flags: `--check` (verify-only, fail on drift; default is regenerate), `--json` (machine-readable output for CI consumption — same shape as the existing check_service_boundary's JSON output), and the dispatch wires through to `example_spirit_regen::run(workspace_root, check_mode)`,
**And** the sub-command is registered in `xtask/gate-registry.toml` as the 17th gate (alongside `check-service-boundary`, `check-empty-kernel`, etc.) — verify the registry pattern by reading the file before adding,
**And** `cargo run -p xtask -- example-spirit-regen --check --json` exits 0 against the committed `examples/example-spirit/` (proves the example is in sync with the template at story-completion time),
**And** the dev agent verifies the drift detector works by deliberately modifying `examples/example-spirit/src/lib.rs` (e.g., adding a stray comment), running `cargo run -p xtask -- example-spirit-regen --check`, observing the exit-1 + clear error message ("drift: src/lib.rs: rendered output differs from committed example-spirit"), then reverting the modification — this validation is captured in the dev record's "Drift detector validation" section,
**And** `xtask/tests/example_spirit_regen_integration.rs` is created with at minimum these test cases: `check_mode_passes_on_committed_example` (asserts exit 0 against the real workspace), `check_mode_fails_on_drift` (uses a `tempfile::TempDir` workspace, deliberately introduces drift, asserts `Err(...)` is returned with the expected substring), `regen_mode_overwrites_files` (uses a tempdir, runs regen, asserts the output matches the template), `regen_mode_preserves_readme` (asserts the existing README is NOT overwritten),
**And** `cargo test -p xtask --test example_spirit_regen_integration` PASSES (4 tests at minimum).

### AC6 — Two new `discipline.yml` jobs + summary table updates

**Given** the existing `.github/workflows/discipline.yml` 28-job gate set
**And** the existing `hello-spirit-tests` job at line 414-425 (the precedent shape: `runs-on: ubuntu-latest`, `actions/checkout@v4`, `dtolnay/rust-toolchain@v1` with `stable`, `Swatinem/rust-cache@v2`, then `cargo test -p maos-spirit-hello --locked`)
**And** the existing discipline-summary `needs:` list at line 535
**And** the existing PR-comment table builder at lines 540-640 (assigns each gate's result to a short variable name, then renders a markdown table)

**When** the dev agent extends `.github/workflows/discipline.yml`

**Then** a new job `example-spirit-tests` is added (insert immediately after `hello-spirit-bench` for ordering parity):
```yaml
  example-spirit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}
      - name: Run example-Spirit tests (template-baked output, v0.3 NFR-Onb-1 prerequisite)
        run: cargo test -p example-spirit --locked
```
**And** a new job `example-spirit-drift` is added (insert immediately after `example-spirit-tests`):
```yaml
  example-spirit-drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}
      - name: Run example-spirit drift check (template vs. baked output sync)
        run: cargo run -p xtask -- example-spirit-regen --check --json
```
**And** the discipline-summary `needs:` list at line 535 is appended with both job names (preserve existing order; new entries at the end):
```yaml
    needs: [reproducible-build, check-unsafe, check-empty-kernel, check-loom, check-service-boundary, kloc-check, abi-diff, invariant-lock, check-corpus, check-judge-config, check-security-md, audit-spine-smoke, cap-token-verify-bench, cap-registry-smoke, fr4-1000-call-fixture, audit-query-fr4-smoke, maosctl-smoke, manifest-field-coverage, v01-evaluator-path, hello-spirit-tests, hello-spirit-bench, onb-nfr2-timing, coverage-matrix, corpus-staleness, calibrate-per-commit, determinism-tests, check-fr47, example-spirit-tests, example-spirit-drift]
```
**And** the PR-comment table builder (lines 540-640) is updated to add two new rows (mirror the existing `hello-spirit-tests` precedent at line 562-563 + line 635-636):
- A new line at the variable-assignment block: `echo "est=${{ needs.example-spirit-tests.result }}" >> $GITHUB_OUTPUT` + `echo "esd=${{ needs.example-spirit-drift.result }}" >> $GITHUB_OUTPUT`
- A new line at the JS-template variable extraction (around line 601): `const est = '${{ needs.example-spirit-tests.result }}';` + `const esd = '${{ needs.example-spirit-drift.result }}';`
- Two new table rows in the markdown template (after `| onb-nfr2-timing | ${icon(onb)} ${onb} |`): `| example-spirit-tests | ${icon(est)} ${est} |` + `| example-spirit-drift | ${icon(esd)} ${esd} |`
**And** the total `needs:` list count is verified: 28 existing → 30 jobs total (the count is implicit in the `needs:` list length; dev record cites the explicit count),
**And** the dev agent runs `act -j example-spirit-tests` + `act -j example-spirit-drift` locally if `act` is available (`act` is the standard local GitHub Actions runner) — OR if `act` is unavailable, runs the underlying bash commands directly (`cargo test -p example-spirit --locked` + `cargo run -p xtask -- example-spirit-regen --check --json`) and asserts both pass cold,
**And** the discipline summary's "PASSED / FAILED" semantics are preserved: any `result == 'failure'` in the new jobs blocks the summary green, same shape as existing.

### AC7 — `tests/coverage-matrix.yaml` updated additively

**Given** the existing `tests/coverage-matrix.yaml` rows:
```yaml
  FR33:
    gates: []
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
  FR34:
    gates: []
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
  NFR-Onb-1:
    gates: []
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
```
**And** the `xtask/gate-registry.toml` enumerating registered gate names (verify before claiming any new gate is registered)
**And** Story 0.3's coverage-matrix-vs-gate-registry referential-integrity check (orphan gates rejected; orphan FR/NFR keys rejected — see Story 0.3 dev record)

**When** the dev agent updates `tests/coverage-matrix.yaml`

**Then** the FR33 row becomes:
```yaml
  FR33:
    gates: [example-spirit-tests, example-spirit-drift]
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
    notes: |
      Story 2.3 ships thin cargo-generate template at `templates/spirit-rust/`
      + baked example at `examples/example-spirit/` + xtask drift detector.
      Rust-only at v0.3 prerequisite. Full per-language templates (TypeScript /
      Python / Go) deferred to Story 7.1 at v0.5+.
```
**And** the FR34 row becomes:
```yaml
  FR34:
    gates: [example-spirit-tests]
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
    notes: |
      Story 2.3 ships the local_runner SDK seed (lifecycle hook fire via
      SpiritVtable + Ctx::mock() + in-memory mock IAC bus forward-anchor
      types) at `crates/maos-spirit-sdk/src/local_runner.rs`. Full spirit-test
      SDK with assertion macros + halt resolution + manifest self-check +
      class-specific regression corpus is Story 2.4 seed → Story 7.1 full.
```
**And** the NFR-Onb-1 row becomes:
```yaml
  NFR-Onb-1:
    gates: [example-spirit-tests, example-spirit-drift]
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
    notes: |
      Story 2.3 ships PREREQUISITES (cargo-generate template + local_runner
      SDK + ≥1 example Spirit with passing CI). Full N=12 stratified
      30-Min First Spirit Validation Gate executes at Story 7.5b against
      the Butler reference Spirit shipped by Story 8.1 (per dependency-dag.md
      line 25).
```
**And** the `valid_until` dates stay at `2027-05-12` (Story 2.3 does NOT alter expiration; expiration is a corpus-staleness concern, not a gate-coverage concern — see Story 0.3 contract),
**And** the existing rows for FR17, FR55, FR58, NFR-Test-2 (updated by Story 2.2) are NOT modified by Story 2.3 — verify via `git diff` that ONLY the 3 above rows changed,
**And** `cargo run -p xtask -- coverage-matrix --json` exits 0 (the new gate names — `example-spirit-tests`, `example-spirit-drift` — must be present in `xtask/gate-registry.toml`; if missing, the coverage-matrix gate fails with "orphan gate"; the dev agent registers both gates first, then updates the matrix),
**And** `cargo run -p xtask -- check-corpus --json` exits 0 (Story 2.3 ships NO new content-addressed corpora; `tests/corpora/MANIFEST.toml` is unchanged — verify by `git diff` showing no MANIFEST.toml lines changed).

### AC8 — Architecture-doc adjustments + `spirit-development-and-sharing.md` callout

**Given** the existing `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 layout block (post Story 2.1, listing 20 lib/bin crates + xtask = 21 workspace members; mentioning `spirits/` and `schemas/` but NOT `templates/` or `examples/`)
**And** the existing `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5.3 hooks table (Story 2.1 added the "Implemented at" column)
**And** the existing `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` document (Diego J6's reference reading)
**And** the D10 catch-up pattern from Story 1b.6 + Story 2.1 (minimal in-PR doc updates, NOT rewrites)

**When** the dev agent finalizes Story 2.3

**Then** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 gains:
- A new directory entry in the layout tree (insert after the `spirits/` row and before `schemas/`):
```
├── templates/                          # Spirit-author scaffolding (Story 2.3)
│   └── spirit-rust/                    # Thin cargo-generate template (Rust-only at v0.3;
│                                       # per-language TS/Python/Go at Story 7.1 v0.5+).
│                                       # Excluded from workspace via [workspace] exclude.
├── examples/                           # Workspace-member example Spirits (NOT kernel substrate)
│   └── example-spirit/                 # Baked output of templates/spirit-rust (Story 2.3).
│                                       # Drift-detected via `xtask example-spirit-regen --check`.
```
- A one-paragraph addendum at the end of §4.0.2 (after the existing "Workspace member count" paragraph):
> **Workspace member count (post Story 2.3):** 20 library/binary crates + xtask + `examples/example-spirit` = **22 workspace members**. The `examples/example-spirit` crate is workspace-managed (member of `[workspace] members`) so the discipline suite's `example-spirit-tests` + `example-spirit-drift` jobs continuously prove `templates/spirit-rust/` generates compiling code as the SDK + ABI evolve. The `templates/` directory is excluded via `[workspace] exclude = ["templates"]` (templates contain `{{ placeholder }}` syntax that is not valid Rust). `examples/*` is the new convention for workspace-managed proof artifacts that are NOT part of the kernel substrate; future reference Spirits (Butler at Story 8.1, Researcher at Story 8.2, etc.) MAY land at `examples/*` or `crates/maos-spirit-*` per their story's design.
**And** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` gains a ≤6-line addendum at the end of §5 (after the §5.4 Posture section, NOT inserted into §5.3 hooks table — keep the addendum out-of-band):
> **v0.3 prerequisite — Spirit-author scaffolding (Story 2.3):** Spirit authors at v0.3 prerequisite scaffold a new Rust Spirit via `cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit`. The generated crate uses the `#[spirit]` proc-macro from Story 2.1, declares a TOML manifest mirroring the hello-spirit shape, and ships a test driven by `maos_spirit_sdk::local_runner::LocalRunner` (no kernel instance required). The baked output is committed at `examples/example-spirit/` and CI-enforced via `example-spirit-tests` + `example-spirit-drift`. Per-language templates (TS / Python / Go) land in Story 7.1; the NFR-Onb-1 30-Min First Spirit Validation Gate executes at Story 7.5b against Butler from Story 8.1.
**And** `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` gains a top-of-file callout (insert immediately after the existing `📍 Phasing reconciled (2026-05-06)` blockquote at line 24):
> **🛠️ v0.3 prerequisite shipped — Story 2.3 (2026-05-16).** The `cargo generate maos-spirit` template + local runner cited throughout this document (§3 — "Build your first Spirit in 30 minutes," §10.6 Diego J6 onboarding) lands at v0.3 PREREQUISITE (NOT v1.0 as earlier drafts implied). Template path: `templates/spirit-rust/`. Local runner: `maos_spirit_sdk::local_runner::LocalRunner` (gated behind `local_runner` cargo feature). Example: `examples/example-spirit/`. The full NFR-Onb-1 30-Min First Spirit Validation Gate (N=12 stratified) runs at Story 7.5b against the Butler reference Spirit from Story 8.1 — Story 2.3 ships the SUBSTRATE, not the gate.
**And** the architecture-doc updates land in the SAME PR as the code (mirror the D10 pattern from Story 1b.6 + Story 2.1 — do NOT defer the doc update; doc accretion is the failure mode that prompted D10 originally),
**And** the dev agent verifies no other architecture file is broken by the doc update (`grep -rn "21 workspace members\|20 lib/bin\|examples/example-spirit\|templates/spirit-rust" _bmad-output/planning-artifacts/architecture-maos-minimal-opus/` after the edits should show ONLY the 3 files updated above; no other references to "21 workspace members" should exist anywhere — if they do, they're stale and need a cross-reference update).

### AC9 — Discipline-suite sweep + cold-cache integration + dev-record gates citation

**Given** the full 30-job discipline-suite (28 existing + 2 new per AC6)
**And** the A6 + A7 + A8 retro actions from `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md` lines 182-199
**And** Story 2.1's + Story 2.2's dev-record `Gates Status` precedent

**When** the dev agent finishes Story 2.3 implementation

**Then** every existing gate continues to pass against the real v0.1-β workspace:
- `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` — 0 added/changed/removed against `maos-spirit-abi`
- `cargo run -p xtask -- check-empty-kernel --json` — exit 0 (no new persistent I9-violating state; local_runner is stateless; xtask sub-command holds no persistent state)
- `cargo run -p xtask -- check-service-boundary --json` — exit 0 (P1 + P2 NOW green per AC1's bridge fixes; P3 + P4 + Spirit ABI reflection stay green from Story 2.2)
- `cargo run -p xtask -- check-unsafe --json` — exit 0 (local_runner is `#![forbid(unsafe_code)]`; xtask `example_spirit_regen` is `#![forbid(unsafe_code)]`; example-spirit is `#![forbid(unsafe_code)]`; template's `src/lib.rs.liquid` is `#![forbid(unsafe_code)]`)
- `cargo run -p xtask -- kloc-check --json` — exit 0 (verify `xtask/kloc.toml` budgets accommodate the new ~200 LOC for local_runner + ~150 LOC for example_spirit_regen + ~80 LOC for tests; if budgets break, raise them in-story per Story 2.2 precedent at dev record line 657)
- `cargo run -p xtask -- invariant-lock --json` — exit 0 (Story 2.3 does NOT touch any invariant; gate reports "no invariant-touching diffs")
- `cargo run -p xtask -- check-corpus --json` — exit 0 (no new corpora; MANIFEST.toml unchanged)
- `cargo run -p xtask -- check-judge-config --json` — exit 0
- `cargo run -p xtask -- check-security-md --json` — exit 0
- `cargo run -p xtask -- check-fr47 --json` — exit 0
- `cargo run -p xtask -- check-loom --json` — exit 0
- `cargo run -p xtask -- coverage-matrix --json` — exit 0 (3 row updates per AC7; 2 new gate names registered per AC5)
- `cargo run -p xtask -- corpus-staleness --json` — exit 0
- `cargo run -p xtask -- manifest-field-coverage --json` — exit 0 (template's manifest uses ONLY existing fields; verify by reading the template manifest + grepping for any field not in the existing fixture set at `crates/maos-kernel-core/tests/fixtures/manifest/`)
- `cargo run -p xtask -- example-spirit-regen --check --json` — exit 0 (NEW; AC5 gate)
**And** the new `example-spirit-tests` job (locally simulated by `cargo test -p example-spirit --locked`) passes cold (`cargo clean -p example-spirit && cargo test -p example-spirit --locked`),
**And** the existing `tests/integration/v01_evaluator_path.sh` passes cold (per A6 — `cargo clean -p maos-bin && cargo clean -p maos-spirit-hello && ./tests/integration/v01_evaluator_path.sh`),
**And** the existing `tests/integration/onb_nfr2_timing.sh` passes (NFR-Onb-2 5-minute evaluator path remains green — the AC1 bridge fixes do NOT alter hello-spirit behavior),
**And** the dev agent runs the full local discipline sweep ONE final time chained: `cargo run -p xtask -- abi-diff check-empty-kernel check-service-boundary check-unsafe kloc-check invariant-lock check-corpus coverage-matrix manifest-field-coverage example-spirit-regen --check` (note: chaining may require the xtask CLI to support multi-command invocation — if it doesn't, run sequentially via `&&` in a single shell command and capture each exit code in the dev record),
**And** the dev record's `Gates Status` section cites the SPECIFIC `discipline.yml` run on the PR commit (per A8: `discipline.yml run <run_id>, conclusion: success` — NOT proxied by `journal-append.yml`),
**And** the dev record's `What did NOT happen this story` section (per A4) grep-verifies anti-claims for: NO new ADR (verify `docs/adr/index.md` unchanged), NO `maos-spirit-abi` public-API change (verify `cargo public-api -p maos-spirit-abi` against the existing baseline shows zero diff), NO new content-addressed corpus (verify `tests/corpora/MANIFEST.toml` unchanged), NO migration of hello-spirit to the template, NO Spirit registry publish path, NO per-language template (only Rust), NO full spirit-test SDK with assertion macros (only the local_runner seed), NO NFR-Onb-1 30-min gate execution (only the prerequisites), NO runtime kernel instantiation in local_runner (`cargo tree -p maos-spirit-sdk --features local_runner --no-default-features --features local_runner,std,mock | grep maos-kernel-core` returns empty),
**And** the dev agent's self-review checklist at the end of the dev record contains ≥20 items (per Epic 1a/1b/2.1/2.2 retro discipline) covering: AC1 bridge fixes applied + `check-service-boundary` green; AC2 template scaffolds compile via manual smoke; AC3 local_runner has zero kernel-core dep; AC4 example crate is workspace member + builds clean; AC5 drift detector validated by deliberate-drift test; AC6 `discipline.yml` 30-job count verified; AC7 coverage-matrix 3-row diff is the only change; AC8 architecture-doc 21→22 member count consistent across all 3 files; AC9 cold-cache integration scripts pass; ABI stability (zero diff against `maos-spirit-abi`); no new `unsafe`; no new ADR; member-count consistency in `Cargo.toml` + arch doc; `examples/example-spirit/` README divergence preserved; `tests/integration/v01_evaluator_path.sh` cold-cache green; `tests/integration/onb_nfr2_timing.sh` green; specific `discipline.yml` run-id cited; A6/A7/A8 retro actions all confirmed.

## Tasks / Subtasks

Each top-level task carries `(AC: #)` mapping. Substeps preserve order. **Task 0 ships the bridge fixes FIRST** (preserves the spirit of "bridge before template work"). **Self-review checklist at end is mandatory** before opening PR (per Epic 1a/1b/2.1/2.2 retro actions A1/A2/A4/A5/A6/A7/A8).

- [x] **Task 0 — Pre-flight: re-land Story 2.2 reverted production fixes** (AC: 1)
  - [x] 0.1 Read `crates/maos-bin/src/main.rs` lines 80-130 + 310-330 to confirm the current state matches the grep output above (3 `SecurityManagerAdapter` constructions; lines 86, 122, 318)
  - [x] 0.2 Edit `crates/maos-bin/src/main.rs`: REMOVE line 86 (`let _security = SecurityManagerAdapter::default();`); REMOVE line 122 (`let _security = SecurityManagerAdapter::new(Arc::clone(&policy));`). Verify the only remaining construction is at line 318 inside the `MAOS_ONE_SHOT=hello-spirit` block. If either deletion leaves a dangling reference elsewhere in `main.rs` (verify by grep before commit), restructure cleanly.
  - [x] 0.3 Read `crates/maos-kernel-core/src/security/mod.rs` lines 1-35 to confirm the current re-export block shape
  - [x] 0.4 Edit `crates/maos-kernel-core/src/security/mod.rs`: APPEND `pub use maos_domain::ports::CryptoProvider;` to the existing re-export block (specifically: insert after line 31 `pub use manifest::{OutputShapePredicate, OutputShapeViolation, capabilities_required_to_scopes};` and before line 32 `pub use sandbox::{...};` — preserves the Story 1b.5c re-export-order discipline cited at lines 21-25 of the file). Use a one-line comment: `// Story 2.3 — appended for P2 port-pair completeness (RingCryptoProvider adapter → CryptoProvider Port).`
  - [x] 0.5 Check `crates/maos-domain/src/ports/mod.rs` (or wherever `CryptoProvider` is exported from `maos-domain::ports`) to confirm the trait is publicly available at that path. If it's at a different path (e.g., `maos_domain::ports::crypto::CryptoProvider`), use the correct path in step 0.4.
  - [x] 0.6 Investigate `xtask/src/check_service_boundary.rs:1581-1585` (the Story 2.2 RingCryptoProvider special-case workaround). Once the Port trait is re-exported, the workaround is REDUNDANT (the natural P2 port-pair check finds the trait). Decide: REMOVE the workaround (preferred; cleanest) OR leave with a one-line comment `// Story 2.3: CryptoProvider re-export in security/mod.rs now makes this special-case redundant; retained for fixture-test back-compat`. Capture the decision in the dev record.
  - [x] 0.7 Run `cargo build -p maos-bin --locked` — must succeed; the 2 deleted bindings should not break compilation (they were unused `let _` bindings).
  - [x] 0.8 Run `cargo run -p xtask -- check-service-boundary --json` — must exit 0. If it doesn't, the dev agent's bridge fix is incomplete; investigate the violation message + adjust.
  - [x] 0.9 Run `MAOS_ONE_SHOT=hello-spirit cargo run -p maos-bin --release` — must produce 4-key JSON identical to pre-2.2 baseline (`introduction`, `capability_scope`, `halt_tags`, `transparency_log`).
  - [x] 0.10 Run `cargo clean -p maos-bin maos-spirit-hello && ./tests/integration/v01_evaluator_path.sh` cold (per A6) — must pass.
  - [x] 0.11 Run `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` — must show zero added/changed/removed against `maos-spirit-abi`. The `security/mod.rs` re-export is in `maos-kernel-core`, NOT `maos-spirit-abi`; if it accidentally leaks into the abi-diff scope, investigate (likely a scoping issue in `xtask/src/abi_diff.rs`).
  - [x] 0.12 Check `docs/ci-baselines/kernel-surface-v0.1-beta.json` — if the new `CryptoProvider` re-export adds a public symbol, refresh the baseline per Story 2.2 dev record's pattern at lines 696-702: `cargo run -p xtask -- check-service-boundary --json | jq` to inspect, regenerate via `xtask check-service-boundary --regenerate-baseline` if such a flag exists, add `CryptoProvider` to `xtask/kernel-api-classes.toml` under the `ports` classification.
  - [x] 0.13 Commit AC1 work as the FIRST commit of the Story 2.3 PR. Suggested message: `fix(maos-bin, maos-kernel-core): re-land Story 2.2 reverted production fixes (SecurityManagerAdapter dedupe + CryptoProvider re-export)`. Co-Authored-By if applicable.

- [x] **Task 1 — Author the cargo-generate template at `templates/spirit-rust/`** (AC: 2)
  - [x] 1.1 Create `templates/spirit-rust/cargo-generate.toml` with the exact `[template]` + `[placeholders]` shape from AC2. Verify the placeholder regex syntax against the cargo-generate docs (`cargo install cargo-generate --version "^0.21"` + check `cargo generate --help`).
  - [x] 1.2 Create `templates/spirit-rust/Cargo.toml` with the exact `{{crate_name}}` placeholder + `git = "https://github.com/lunarpulse/maos"` dep declaration. Use the exact tag `v0.1-template-seed` (this tag does NOT yet exist; document in the dev record that the maintainer must cut this tag after PR merge — OR use `branch = "main"` initially with a TODO to swap to a tag once stable; the dev agent's choice is captured in the dev record's "Template git-pin decision" section).
  - [x] 1.3 Create `templates/spirit-rust/src/lib.rs` with the exact `{{class_name}}` placeholder + `#[spirit]` use. Include `#![forbid(unsafe_code)]`.
  - [x] 1.4 Create `templates/spirit-rust/manifest.toml` mirroring the hello-spirit shape exactly (read `spirits/hello-spirit/manifest.toml` for the template; substitute `name = "hello-spirit"` → `name = "{{crate_name}}"`). DO NOT introduce any new manifest fields (manifest-field-coverage gate would fail).
  - [x] 1.5 Create `templates/spirit-rust/tests/spirit_smoke.rs` with the exact local_runner invocation from AC2. Use the `{{ crate_name | snake_case }}` filter for the module-name reference (cargo-generate's Liquid `snake_case` filter is standard).
  - [x] 1.6 Create `templates/spirit-rust/README.md` with all 7 required sections from AC2.
  - [x] 1.7 Add `[workspace] exclude = ["templates"]` to the workspace root `Cargo.toml` (preserve the alphabetical / functional ordering of the [workspace] block). Verify via `cargo metadata --no-deps --format-version 1 | jq '.workspace_members[]'` that the template is NOT listed.
  - [x] 1.8 Manual smoke-test the template: `cargo install cargo-generate --version "^0.21"` (pin the version in the dev record) + `cd /tmp && cargo generate --path <maos-repo-abs-path>/templates/spirit-rust --name testflight-spirit` + `cd testflight-spirit && cargo build --offline` (if `--offline` works) OR `cargo build` (online) + `cargo test --features maos-spirit-sdk/local_runner` PASSES.
  - [x] 1.9 Document the manual smoke-test commands + the produced directory tree in the dev record's "Template smoke" section.

- [x] **Task 2 — Add `local_runner` module + fixture types to `maos-spirit-sdk`** (AC: 3)
  - [x] 2.1 Create `crates/maos-spirit-sdk/src/local_runner.rs` with the exact code shape from AC3. Verify `#![forbid(unsafe_code)]` at top.
  - [x] 2.2 Update `crates/maos-spirit-sdk/src/lib.rs`: append `#[cfg(feature = "local_runner")] pub mod local_runner;` AFTER the existing `pub mod cancellation;` line (preserve order).
  - [x] 2.3 Update `crates/maos-spirit-sdk/Cargo.toml`: add `local_runner = ["std", "mock"]` to `[features]`. Update `[dev-dependencies]` to ensure `maos-spirit-abi = { path = "../maos-spirit-abi", features = ["mock"] }` (already present per Story 2.1).
  - [x] 2.4 Create `crates/maos-spirit-sdk/tests/local_runner_smoke.rs` with the exact 3-test shape from AC3 (gated `#[cfg(feature = "local_runner")]`).
  - [x] 2.5 Run `cargo test -p maos-spirit-sdk --features local_runner --test local_runner_smoke` — 3 tests pass.
  - [x] 2.6 Run `cargo build -p maos-spirit-sdk --no-default-features` — succeeds (no_std parity preserved).
  - [x] 2.7 Run `cargo tree -p maos-spirit-sdk --features local_runner --edges normal,build | grep -c maos-kernel-core` — must output `0` (zero kernel-core dep). If non-zero, investigate which crate transitively pulls kernel-core and prune.
  - [x] 2.8 Run `cargo build -p maos-spirit-sdk --no-default-features --features local_runner` — expected to FAIL with a clear feature-resolution error (local_runner requires std + mock; without default features, the error should be diagnostic-level, not a compile crash); capture the exact error message in the dev record.
  - [x] 2.9 Run `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` — confirm zero added/changed/removed against `maos-spirit-abi` (local_runner is in `maos-spirit-sdk`, outside the gate's scope).

- [x] **Task 3 — Bake the example Spirit at `examples/example-spirit/`** (AC: 4)
  - [x] 3.1 Manually render the template into `examples/example-spirit/` by substituting `{{crate_name}}` → `example-spirit`, `{{class_name}}` → `ExampleSpirit`, `{{ crate_name | snake_case }}` → `example_spirit`. Use `mkdir -p examples/example-spirit/{src,tests}` then `cp` + `sed` (or use the xtask `example-spirit-regen` once Task 4 lands — but the chicken-and-egg suggests Task 3 manual render first, then Task 4 verifies via `--check`).
  - [x] 3.2 Edit `examples/example-spirit/Cargo.toml`: swap the template's `git = "..."` dep with `path = "../../crates/maos-spirit-sdk"` (workspace-relative). Mirror the exact shape from AC4.
  - [x] 3.3 Verify `examples/example-spirit/src/lib.rs` is the placeholder-resolved version of the template's `src/lib.rs` (NOT a hand-edited divergence). Mirror exactly the shape from AC4.
  - [x] 3.4 Verify `examples/example-spirit/manifest.toml` substitutes `{{crate_name}}` → `example-spirit` everywhere.
  - [x] 3.5 Create `examples/example-spirit/tests/spirit_smoke.rs` with the exact shape from AC4 (NOT the template's filename-suffixed version).
  - [x] 3.6 Create `examples/example-spirit/README.md` with the EXACT content from AC4 (intentionally divergent from the template's README — this README documents the regeneration workflow).
  - [x] 3.7 Update workspace root `Cargo.toml`: append `"examples/example-spirit"` to `[workspace] members`. Preserve existing member ordering.
  - [x] 3.8 Run `cargo build -p example-spirit` — succeeds zero new warnings.
  - [x] 3.9 Run `cargo test -p example-spirit --locked` — smoke test passes.
  - [x] 3.10 Verify `cargo build --workspace 2>&1 | grep -c warning` is non-greater than the baseline count (capture baseline before Task 3 begins; non-regression assertion).
  - [x] 3.11 Run `cargo clean -p example-spirit && cargo test -p example-spirit --locked` cold — passes (per A6).

- [x] **Task 4 — Add xtask `example-spirit-regen` sub-command + drift detector** (AC: 5)
  - [x] 4.1 Verify `tempfile = "3"` is in `xtask/Cargo.toml` `[dependencies]` (if only in `[dev-dependencies]`, promote to `[dependencies]` because the regen runtime needs it — note this in dev record).
  - [x] 4.2 Create `xtask/src/example_spirit_regen.rs` with `pub fn run(workspace_root: &Path, check_mode: bool) -> Result<(), String>` implementing the AC5 logic. Use the existing `load_toml` helper for `cargo-generate.toml` parsing; use `std::fs::read_to_string` + `str::replace` for template-file rendering.
  - [x] 4.3 Wire the sub-command into `xtask/src/main.rs`: add a `example-spirit-regen` arm to the existing CLI dispatch. Support `--check` (boolean flag) + `--json` (boolean flag). Mirror the existing `--json` output shape from `check_service_boundary`.
  - [x] 4.4 Add `example-spirit-regen` to `xtask/gate-registry.toml` as a new gate entry (read the file first to understand the schema; mirror an existing entry like `check-service-boundary`).
  - [x] 4.5 Create `xtask/tests/example_spirit_regen_integration.rs` with the 4 test cases from AC5: `check_mode_passes_on_committed_example`, `check_mode_fails_on_drift`, `regen_mode_overwrites_files`, `regen_mode_preserves_readme`.
  - [x] 4.6 Run `cargo test -p xtask --test example_spirit_regen_integration --locked` — 4 tests pass.
  - [x] 4.7 Run `cargo run -p xtask -- example-spirit-regen --check --json` — exits 0 (committed example is in sync with template at story-completion time).
  - [x] 4.8 Deliberately introduce drift: `echo "// drift" >> examples/example-spirit/src/lib.rs`, run `cargo run -p xtask -- example-spirit-regen --check`, observe exit 1 + diagnostic message, revert the drift (`git checkout examples/example-spirit/src/lib.rs`). Capture the validation in the dev record's "Drift detector validation" section.
  - [x] 4.9 Verify `cargo run -p xtask -- kloc-check --json` still exits 0 (the new xtask sub-command + tests grow xtask LOC; verify against `xtask/kloc.toml` budgets — Story 2.2 raised xtask to 4000; if Story 2.3 pushes over, raise to 4500 + document in dev record).

- [x] **Task 5 — Extend `.github/workflows/discipline.yml` with example-spirit jobs** (AC: 6)
  - [x] 5.1 Read `.github/workflows/discipline.yml` lines 414-451 to understand the `hello-spirit-tests` + `hello-spirit-bench` + `onb-nfr2-timing` precedent shape.
  - [x] 5.2 Insert `example-spirit-tests` job immediately after `hello-spirit-bench` (preserve ordering for grep-ability). Use the exact YAML shape from AC6.
  - [x] 5.3 Insert `example-spirit-drift` job immediately after `example-spirit-tests`. Use the exact YAML shape from AC6.
  - [x] 5.4 Append `example-spirit-tests, example-spirit-drift` to the discipline-summary `needs:` list at line 535 (preserve existing comma-separated format).
  - [x] 5.5 Update the variable-assignment block at line 562: add `echo "est=${{ needs.example-spirit-tests.result }}" >> $GITHUB_OUTPUT` + `echo "esd=${{ needs.example-spirit-drift.result }}" >> $GITHUB_OUTPUT` after the `onb=` line.
  - [x] 5.6 Update the JS-template variable extraction at line 601: add `const est = '${{ needs.example-spirit-tests.result }}';` + `const esd = '${{ needs.example-spirit-drift.result }}';` after the `onb` const.
  - [x] 5.7 Update the markdown table template at line 637: add `| example-spirit-tests | ${icon(est)} ${est} |` + `| example-spirit-drift | ${icon(esd)} ${esd} |` after the `onb-nfr2-timing` row.
  - [x] 5.8 Verify the YAML is well-formed: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/discipline.yml'))"` (or `yq eval '.' .github/workflows/discipline.yml > /dev/null` if `yq` is available).
  - [x] 5.9 If `act` is available locally (`act --list 2>&1 | grep -E 'example-spirit-tests|example-spirit-drift'`), run both jobs: `act -j example-spirit-tests` + `act -j example-spirit-drift`. If `act` is unavailable, run the underlying bash directly: `cargo test -p example-spirit --locked` + `cargo run -p xtask -- example-spirit-regen --check --json`. Both must pass.

- [x] **Task 6 — Coverage matrix updates** (AC: 7)
  - [x] 6.1 Read `tests/coverage-matrix.yaml` to confirm the current shape of FR33, FR34, NFR-Onb-1 rows.
  - [x] 6.2 Update FR33 row per AC7 (add `gates: [example-spirit-tests, example-spirit-drift]` + `notes:` block).
  - [x] 6.3 Update FR34 row per AC7 (add `gates: [example-spirit-tests]` + `notes:` block).
  - [x] 6.4 Update NFR-Onb-1 row per AC7 (add `gates: [example-spirit-tests, example-spirit-drift]` + `notes:` block).
  - [x] 6.5 Verify NO other rows changed: `git diff tests/coverage-matrix.yaml` should show changes ONLY to these 3 rows.
  - [x] 6.6 Run `cargo run -p xtask -- coverage-matrix --json` — exit 0. If "orphan gate" errors fire, Task 4.4 didn't register the new gates correctly; fix before proceeding.
  - [x] 6.7 Run `cargo run -p xtask -- check-corpus --json` — exit 0 (no new corpora; `tests/corpora/MANIFEST.toml` unchanged).

- [x] **Task 7 — Architecture-doc adjustments** (AC: 8)
  - [x] 7.1 Read `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 (lines 21-95) to understand the current layout shape.
  - [x] 7.2 Edit §4.0.2: insert the `templates/` + `examples/` directory entries per AC8. Append the workspace-member-count paragraph at the end of §4.0.2.
  - [x] 7.3 Read `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5.4 to find the insertion point for the addendum.
  - [x] 7.4 Edit §5: append the ≤6-line v0.3-prerequisite addendum at the end of the section per AC8.
  - [x] 7.5 Read `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` line 24 (the `📍 Phasing reconciled` blockquote) for the insertion point.
  - [x] 7.6 Edit `spirit-development-and-sharing.md`: insert the `🛠️ v0.3 prerequisite shipped — Story 2.3` callout immediately after the phasing blockquote.
  - [x] 7.7 Verify cross-reference consistency: `grep -rn "21 workspace members\|20 lib/bin\|22 workspace members\|examples/example-spirit\|templates/spirit-rust" _bmad-output/planning-artifacts/architecture-maos-minimal-opus/ _bmad-output/planning-artifacts/spirit-development-and-sharing.md` — confirm the 3 edited files are the only files containing these strings (no stale references elsewhere).
  - [x] 7.8 Re-run `cargo run -p xtask -- coverage-matrix --json` after doc edits (defensive — doc edits shouldn't affect the gate but verify).

- [x] **Task 8 — Discipline sweep + cold-cache integration + self-review** (AC: 9)
  - [x] 8.1 Run the full local discipline sweep: `cargo run -p xtask -- abi-diff` + `check-empty-kernel` + `check-service-boundary` + `check-unsafe` + `kloc-check` + `invariant-lock` + `check-corpus` + `coverage-matrix` + `manifest-field-coverage` + `check-judge-config` + `check-security-md` + `check-fr47` + `check-loom` + `corpus-staleness` + `example-spirit-regen --check` — all green. Capture each gate's exit code in the dev record's `Gates Status` section.
  - [x] 8.2 Cold cache: `cargo clean -p maos-bin maos-spirit-hello example-spirit && ./tests/integration/v01_evaluator_path.sh` — passes.
  - [x] 8.3 Cold cache: `cargo clean -p example-spirit && cargo test -p example-spirit --locked` — passes.
  - [x] 8.4 Cold cache: `cargo clean -p maos-spirit-sdk && cargo test -p maos-spirit-sdk --features local_runner --test local_runner_smoke --locked` — passes.
  - [x] 8.5 `cargo metadata --no-deps --format-version 1 | jq '.workspace_members[]'` — confirms `examples/example-spirit` is listed (member count = 22) AND `templates/spirit-rust` is NOT listed.
  - [x] 8.6 `cargo build --workspace --locked` — succeeds clean.
  - [x] 8.7 Cite the SPECIFIC `discipline.yml` run on the PR commit (per A8): `discipline.yml run <run_id>, conclusion: success` (NOT `journal-append.yml`). After PR open, monitor CI and capture the run ID + conclusion in the dev record.
  - [x] 8.8 Self-review checklist at end of dev record with ≥20 items per AC9. Explicit items:
    - [ ] Confirmed AC1 bridge fixes applied: `SecurityManagerAdapter` constructed once + `CryptoProvider` re-exported
    - [ ] Confirmed `check-service-boundary --json` exits 0 against the real v0.1-β workspace
    - [ ] Confirmed `templates/spirit-rust/` is NOT a workspace member
    - [ ] Confirmed `examples/example-spirit/` IS a workspace member (count = 22)
    - [ ] Confirmed `cargo tree -p maos-spirit-sdk --features local_runner` shows ZERO `maos-kernel-core` dep
    - [ ] Confirmed manual `cargo generate --path templates/spirit-rust --name testflight-spirit` works end-to-end (commands in dev record)
    - [ ] Confirmed `example-spirit-regen --check` exits 0 on committed state AND exits 1 on deliberate-drift validation
    - [ ] Confirmed `discipline.yml` job count: 28 → 30 (verified by `grep -c "  example-spirit" .github/workflows/discipline.yml` showing 2 + line 535 `needs:` length)
    - [ ] Confirmed `cargo build --workspace 2>&1 | grep -c warning` is non-regressive vs. pre-Story-2.3 baseline
    - [ ] Confirmed `cargo public-api -p maos-spirit-abi` against baseline shows zero diff
    - [ ] Confirmed `tests/integration/v01_evaluator_path.sh` passes cold
    - [ ] Confirmed `tests/integration/onb_nfr2_timing.sh` passes
    - [ ] Confirmed `tests/corpora/MANIFEST.toml` is UNCHANGED (no new corpora)
    - [ ] Confirmed `docs/adr/index.md` is UNCHANGED (no new ADR)
    - [ ] Confirmed architecture-doc updates land in same PR (3 files: §4.0.2 layout, §5 addendum, spirit-dev-and-sharing callout)
    - [ ] Confirmed coverage-matrix diff is EXACTLY 3 rows (FR33, FR34, NFR-Onb-1)
    - [ ] Confirmed every cargo invocation in new scripts uses `-p <crate>` selection (per A7)
    - [ ] Confirmed no new `unsafe` was added outside `xtask/unsafe-allowlist.toml` (per ADR-039)
    - [ ] Confirmed `local_runner` cargo feature gates the module correctly (no_std build still succeeds)
    - [ ] Confirmed `discipline.yml` run conclusion cited (NOT proxied by `journal-append.yml`; per A8)
    - [ ] Confirmed all 4 `example_spirit_regen_integration` tests pass
    - [ ] Confirmed Task 0 commit was the FIRST commit of the PR
  - [ ] 8.9 `What did NOT happen this story` section (per A4) grep-verified anti-claims for: NO per-language template (only Rust); NO full spirit-test SDK with assertion macros (only the local_runner seed); NO NFR-Onb-1 30-min gate execution (only prerequisites); NO migration of hello-spirit to template; NO Spirit registry publish path; NO ADR; NO `maos-spirit-abi` public-API change; NO new content-addressed corpus; NO runtime kernel instantiation in local_runner; NO new manifest field shapes; NO cross-Spirit isolation hooks (Story 2.4); NO LCAS framework (Story 2.4).

## Dev Notes

### Architectural anchor — v0.3 NFR-Onb-1 30-Min First Spirit Validation Gate

Per `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` line 122:

> **NFR-Onb-1: 30-Min First Spirit Validation Gate.** N=12 stratified external Spirit authors. Floor: median ≤ 45 min, p95 ≤ 90 min, AND ≥ 10/12 succeed where "succeed" = author produces Spirit binary that (a) compiles against published ABI, (b) passes the v0.3-grade Butler-class regression corpus, (c) does so within 14 calendar days from kit handoff with zero direct-message support. **v0.3 release criterion.**

Story 2.3 ships the PREREQUISITES for this gate — the template + local runner + ≥1 example Spirit with passing CI. The gate ITSELF runs at Story 7.5b at v0.3 against the Butler reference Spirit from Story 8.1. Per `_bmad-output/planning-artifacts/epics/dependency-dag.md` line 25:

> Story 2.3 thin cargo-generate slice → Story 7.5b NFR-Onb-1 v0.3 gate execution

The architectural framing from architecture §13 v0.3 row (line 10):

> 30-Min First Spirit Validation Gate (NFR-Onb-1) as v0.3 release criterion (N=12 stratified, ≥10/12 succeed).

And the Diego J6 persona scene from architecture §10.6 line 121:

> Diego opens `spirit-development-and-sharing.md`. Skims to *"Build your first Spirit in 30 minutes."* Runs `cargo generate maos-spirit`, gets a templated project.

This story makes the documented invocation actually work at v0.3 prerequisite stage. The `cargo generate maos-spirit` shorthand is a v0.5+ favorites-alias form (Story 7.1); v0.3 uses the subfolder form `cargo generate --git <repo> templates/spirit-rust --name <name>`.

### Pre-flight bridge — what the Story 2.2 review left for 2.3

Story 2.2 (`_bmad-output/implementation-artifacts/2-2-…md` line 707) explicitly reverted 2 production code changes during code review, citing "D1 → bridge story before 2.3":

1. **`crates/maos-bin/src/main.rs`** — Story 2.2 wanted to remove the dead `SecurityManagerAdapter::default()` at line 86 + the unused `SecurityManagerAdapter::new()` at line 122 (P1 single-owner enforcement). Review reverted both.
2. **`crates/maos-kernel-core/src/security/mod.rs`** — Story 2.2 wanted to add `pub use maos_domain::ports::CryptoProvider;` (P2 port-pair completeness). Review reverted.

The closing note at line 735 of the 2.2 dev record:

> **Remaining before Story 2.3:** Bridge story must re-land (1) single `SecurityManagerAdapter` construction in `main.rs` and (2) `CryptoProvider` re-export in `security/mod.rs`. These cause expected P1/P2/surface-diff violations until fixed.

**No separate bridge story was created.** Story 2.3 absorbs both fixes as Task 0 + AC1. This is structurally parallel to how Story 1b.6 absorbed D9/D10/Doc3 from Epic 1b retro — the work bridges the previous epic's gaps before the new feature work begins.

The Story 2.2 special-case workaround at `xtask/src/check_service_boundary.rs:1581-1585` (inline `if adapter == "RingCryptoProvider"`) is RELATED but separate — it papered over the missing `CryptoProvider` re-export. Once AC1's re-export lands, the workaround becomes redundant. Task 0.6 decides remove-vs-retain; the dev agent's choice is captured in the dev record.

### Why the local runner is a separate `local_runner` feature, not always-on (Decision Register)

**DR1.** `crates/maos-spirit-sdk/Cargo.toml` already has `default = ["std"]`, `std = ["dep:tokio-util"]`, `mock = ["maos-spirit-abi/mock"]`. Adding the local runner as `pub mod local_runner` (always-on) would:
- Pull `BTreeMap`, `Instant`, `Duration` into every Spirit's compile (modest cost)
- Compile the runner into every subprocess-form Spirit binary (bytes-on-disk cost, contradicting ADR-002's "Spirit stays small" design — architecture §5 line 7 cites hundreds of KB to a few MB)
- Make the SDK's no_std parity harder (the runner uses `BTreeMap` which is `alloc`-only; gating behind a feature preserves the no_std path)

**DR2.** The runner depends on `Ctx::mock()` (gated by `mock` feature) which depends on `NeverCancel` + zero-valued `CapabilityHandle` + zero-valued `MailboxHandle`. Production Spirits MUST NOT use these — they're test scaffolding. Feature-gating prevents accidental production use.

**DR3.** The runner gating chain: `local_runner = ["std", "mock"]`. This means `local_runner` IMPLIES `std` + `mock`. The dev agent does NOT need to write `default-features = false, features = ["local_runner"]` — `local_runner` pulls everything it needs. The example crate at `examples/example-spirit/` declares `features = ["local_runner"]` in its dep on `maos-spirit-sdk`, which is sufficient.

### Why the example Spirit lives at `examples/example-spirit/` not `crates/maos-spirit-example/` (Decision Register)

**DR4.** The `crates/maos-spirit-*` pattern is reserved for reference Spirits that are PART of the kernel substrate distribution — hello-spirit (v0.1 reference), Butler (Story 8.1), Researcher (Story 8.2), Observer, etc. `examples/example-spirit/` is a PROOF artifact for the template, not a reference Spirit. Putting it under `examples/*`:
- Signals "this is for template validation, not for use" to readers
- Allows the discipline-suite to enforce template-vs-baked drift without putting drift-detection logic into the kernel-substrate crate tree
- Establishes a forward convention: if Story 7.1 ships TypeScript/Python/Go templates at v0.5+, their baked examples land at `examples/example-spirit-ts/`, `examples/example-spirit-py/`, `examples/example-spirit-go/` — keeps the `crates/*` tree clean

**DR5.** The workspace already accepts non-`crates/*` members (`xtask` is at the root). Adding `examples/*` is a small but real architectural decision; the §4.0.2 doc addendum (AC8) captures it.

### Why the drift detector lives in xtask (not as a doc-only convention) (Decision Register)

**DR6.** Without a CI-enforced drift detector, the template and baked output silently diverge as the SDK evolves. The Story 0.3 corpus-pinning discipline is the precedent: content-addressed artifacts get SHA-256 pinned in `MANIFEST.toml`; drift fails CI. The same logic applies to the template + baked output — they're a pair, and their pair-ness needs mechanical enforcement, not author discipline.

**DR7.** The drift detector is a NEW xtask sub-command (`example-spirit-regen`), NOT an extension of `check-corpus` or `check-service-boundary`. Reason: corpus + service-boundary checks operate on bytes-on-disk; the drift detector renders Liquid templates which requires placeholder substitution logic that doesn't belong in either existing checker. The sub-command can grow into `example-spirits-regen` (plural) at Story 7.1 when per-language templates ship.

### Why no new content-addressed corpus (Decision Register)

**DR8.** The Story 0.3 corpus-pinning discipline (`tests/corpora/<name>.jsonl` + `MANIFEST.toml` SHA-256 entry) is for **content** — calibration seeds, secret-redaction items, red-team scenarios, spirit-boundary cases. The template + baked output are **infrastructure**, not content. Story 2.3 ships infrastructure that GENERATES Spirit code; it does NOT ship corpus content the kernel reasons about. Future story 2.4 will ship LCAS corpus content + cross-Spirit isolation framework hooks; Story 7.1 will ship per-language template content. Story 2.3's contribution to corpora is zero, by design.

### Existing code patterns to reuse — DO NOT reinvent

1. **CI job scaffolding.** `.github/workflows/discipline.yml` lines 414-425 (`hello-spirit-tests`) is the canonical shape: `runs-on: ubuntu-latest`, `actions/checkout@v4`, `dtolnay/rust-toolchain@v1` with `stable`, `Swatinem/rust-cache@v2` with `key: ${{ hashFiles('**/Cargo.lock') }}`, then the command. Mirror this for both new jobs.
2. **xtask sub-command pattern.** `xtask/src/check_service_boundary.rs` is the most recent precedent (Story 2.2 expanded it). New sub-commands follow: `pub fn run(workspace_root: &Path, ...) -> Result<..., String>` returning typed errors, JSON output via `serde_json::to_string_pretty`, sub-command wired into `xtask/src/main.rs` CLI dispatch.
3. **`#![forbid(unsafe_code)]` discipline.** Every new Rust file under `crates/*`, `xtask/*`, `templates/*`, `examples/*` carries this at the top. The Story 2.1 precedent at `crates/maos-spirit-derive/src/lib.rs:1` is the model.
4. **Re-export ordering.** `crates/maos-kernel-core/src/security/mod.rs` lines 21-25 cites the discipline: "appended to preserve original re-export order so the `signature_hash` of each existing symbol remains stable under `check-service-boundary`'s use-item hashing." Task 0.4 follows this for the `CryptoProvider` addition.
5. **Composition root preservation.** The `MAOS_ONE_SHOT=hello-spirit` path at `crates/maos-bin/src/main.rs:188-316` is the v0.1 evaluator path. Task 0.2 must NOT alter the line-318 `SecurityManagerAdapter::new(...)` construction — it's the legitimate one; the dev agent removes ONLY the dead/unused constructions at lines 86 + 122.
6. **`pub use` discipline for the SDK façade.** `crates/maos-spirit-sdk/src/lib.rs` Story 2.1 lines 13-25 establishes the pattern: re-export everything authors need from one import. Task 2.2 appends `#[cfg(feature = "local_runner")] pub mod local_runner;` WITHOUT reordering the existing `pub use` items.
7. **Architecture-doc addendum pattern (D10 precedent).** Story 1b.6 (D10 fix) + Story 2.1 (Task 9.8 — workspace 19→20) established the in-PR doc-update pattern. Story 2.3 follows: 1 directory entry + 1 paragraph addendum in §4.0.2; ≤6-line addendum in §5; 1 callout in spirit-dev-and-sharing.md. NO rewrites.
8. **A6 cold-cache integration discipline.** `tests/integration/v01_evaluator_path.sh` is the v0.1 evaluator gate. Story 2.3 re-runs it cold (Task 0.10, Task 8.2) to verify the AC1 bridge fixes don't regress the hello-spirit path. The script's existing `timeout` wrapping is correctly outside the compile step (per A6 fix in commit `95faf94`); Story 2.3 does NOT touch the script.
9. **manifest-field-coverage discipline (NFR-Test-13).** The template's `manifest.toml` MUST use ONLY existing manifest fields (those with fixture triplets at `crates/maos-kernel-core/tests/fixtures/manifest/`). Adding a new field would trigger the NFR-Test-13 walker to demand 3 new fixtures per field. Verify by reading the hello-spirit manifest first, then templating only those exact fields.

### File touch matrix

| File | Operation | Purpose |
|---|---|---|
| `crates/maos-bin/src/main.rs` | UPDATE | Task 0.2 — remove dead `SecurityManagerAdapter` constructions at lines 86 + 122 |
| `crates/maos-kernel-core/src/security/mod.rs` | UPDATE | Task 0.4 — append `pub use maos_domain::ports::CryptoProvider;` |
| `xtask/src/check_service_boundary.rs` | UPDATE (conditional) | Task 0.6 — REMOVE or comment-out RingCryptoProvider special-case (now redundant) |
| `docs/ci-baselines/kernel-surface-v0.1-beta.json` | UPDATE (conditional) | Task 0.12 — refresh if `CryptoProvider` re-export adds a public symbol |
| `xtask/kernel-api-classes.toml` | UPDATE (conditional) | Task 0.12 — add `CryptoProvider` classification if needed |
| `templates/spirit-rust/cargo-generate.toml` | NEW | Task 1.1 — cargo-generate metadata + placeholders |
| `templates/spirit-rust/Cargo.toml` | NEW | Task 1.2 — template package manifest with `{{crate_name}}` placeholder |
| `templates/spirit-rust/src/lib.rs` | NEW | Task 1.3 — template Spirit code with `#[spirit]` + `{{class_name}}` |
| `templates/spirit-rust/manifest.toml` | NEW | Task 1.4 — template Spirit manifest mirroring hello-spirit shape |
| `templates/spirit-rust/tests/spirit_smoke.rs` | NEW | Task 1.5 — template smoke test invoking `local_runner` |
| `templates/spirit-rust/README.md` | NEW | Task 1.6 — 30-min path docs + NFR-Onb-1 citation |
| `Cargo.toml` (workspace root) | UPDATE | Task 1.7 + Task 3.7 — add `[workspace] exclude = ["templates"]`; append `"examples/example-spirit"` to members |
| `crates/maos-spirit-sdk/src/local_runner.rs` | NEW | Task 2.1 — local_runner module |
| `crates/maos-spirit-sdk/src/lib.rs` | UPDATE | Task 2.2 — `#[cfg(feature = "local_runner")] pub mod local_runner;` |
| `crates/maos-spirit-sdk/Cargo.toml` | UPDATE | Task 2.3 — add `local_runner = ["std", "mock"]` feature |
| `crates/maos-spirit-sdk/tests/local_runner_smoke.rs` | NEW | Task 2.4 — 3 smoke tests |
| `examples/example-spirit/Cargo.toml` | NEW | Task 3.2 — workspace-relative path dep |
| `examples/example-spirit/src/lib.rs` | NEW | Task 3.3 — baked Spirit code |
| `examples/example-spirit/manifest.toml` | NEW | Task 3.4 — baked manifest |
| `examples/example-spirit/tests/spirit_smoke.rs` | NEW | Task 3.5 — baked smoke test |
| `examples/example-spirit/README.md` | NEW | Task 3.6 — regeneration workflow docs |
| `xtask/src/example_spirit_regen.rs` | NEW | Task 4.2 — drift detector + regenerator |
| `xtask/src/main.rs` | UPDATE | Task 4.3 — wire `example-spirit-regen` CLI sub-command |
| `xtask/Cargo.toml` | UPDATE (conditional) | Task 4.1 — promote `tempfile` to `[dependencies]` if needed |
| `xtask/gate-registry.toml` | UPDATE | Task 4.4 — register `example-spirit-regen` gate |
| `xtask/tests/example_spirit_regen_integration.rs` | NEW | Task 4.5 — 4 integration tests |
| `xtask/kloc.toml` | UPDATE (conditional) | Task 4.9 — raise xtask budget if exceeded |
| `.github/workflows/discipline.yml` | UPDATE | Task 5.2 + 5.3 + 5.4 + 5.5 + 5.6 + 5.7 — 2 new jobs + summary updates |
| `tests/coverage-matrix.yaml` | UPDATE | Task 6.2 + 6.3 + 6.4 — FR33, FR34, NFR-Onb-1 rows additively |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` | UPDATE | Task 7.2 — §4.0.2 layout + workspace member count |
| `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` | UPDATE | Task 7.4 — ≤6-line v0.3 addendum at end of §5 |
| `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` | UPDATE | Task 7.6 — top-of-file callout |

### Source citations

- NFR-Onb-1 30-Min First Spirit Validation Gate: [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:122`]
- v0.3 release criterion (NFR-Onb-1 cited): [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:206`]
- FR33 (cargo-generate template): [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md:82`]
- FR34 (spirit-test SDK): [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md:83`]
- Epic 2 Story 2.3 ACs: [Source: `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:83-109`]
- Epic 2 NFR-Onb-1 prerequisites scope: [Source: `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:19`]
- Story 7.5b NFR-Onb-1 v0.3 gate execution: [Source: `_bmad-output/planning-artifacts/epics/dependency-dag.md:25,51,66`]
- Diego J6 onboarding persona scene: [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/10-journey-traceability.md:115-152`]
- Architecture §13 v0.3 row: [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md:10`]
- Architecture §5 Spirit ABI: [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md:1-200`]
- §5.3 lifecycle hooks (Story 2.1 implementation column): [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md:173-195`]
- §4.0.2 workspace layout (post-2.1, 20-member): [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:21-95`]
- spirit-development-and-sharing intro + 30-minute path: [Source: `_bmad-output/planning-artifacts/spirit-development-and-sharing.md:1-50`]
- Story 2.2 reverted-production-code review item: [Source: `_bmad-output/implementation-artifacts/2-2-…md:707`]
- Story 2.2 "Remaining before Story 2.3": [Source: `_bmad-output/implementation-artifacts/2-2-…md:735`]
- Story 2.2 RingCryptoProvider special-case workaround: [Source: `xtask/src/check_service_boundary.rs:1581-1585` per `2-2-…md:723`]
- Story 2.1 Spirit trait + 11 hooks: [Source: `crates/maos-spirit-abi/src/lifecycle.rs:138-182`]
- Story 2.1 SpiritVtable + `#[repr(C)]`: [Source: `crates/maos-spirit-abi/src/lifecycle.rs:194-200`]
- Story 2.1 `Ctx::mock()`: [Source: `crates/maos-spirit-abi/src/ctx.rs:67-76`]
- Story 2.1 `CancellationSignal` + `NeverCancel`: [Source: `crates/maos-spirit-abi/src/cancellation.rs:35-90`]
- Story 2.1 `#[spirit]` proc-macro: [Source: `crates/maos-spirit-derive/src/lib.rs:121-321`]
- Story 2.1 SDK façade re-exports: [Source: `crates/maos-spirit-sdk/src/lib.rs:13-25`]
- Story 2.1 SDK feature gates: [Source: `crates/maos-spirit-sdk/Cargo.toml:15-22`]
- Hello-spirit manifest reference shape: [Source: `spirits/hello-spirit/manifest.toml`]
- Hello-spirit Spirit code reference: [Source: `crates/maos-spirit-hello/src/lib.rs`]
- Story 2.2 P1+P2 enforcement code: [Source: `xtask/src/check_service_boundary.rs:486-1660`]
- Story 2.2 dev record bridge note: [Source: `_bmad-output/implementation-artifacts/2-2-…md:705-735`]
- Existing `discipline.yml` hello-spirit-tests pattern: [Source: `.github/workflows/discipline.yml:414-425`]
- Existing `discipline.yml` summary `needs:` list: [Source: `.github/workflows/discipline.yml:535`]
- Existing `discipline.yml` PR comment table: [Source: `.github/workflows/discipline.yml:540-640`]
- Existing `tests/coverage-matrix.yaml` FR33 row: [Source: `tests/coverage-matrix.yaml:289-293`]
- Existing `tests/coverage-matrix.yaml` FR34 row: [Source: `tests/coverage-matrix.yaml:294-298`]
- Existing `tests/coverage-matrix.yaml` NFR-Onb-1 row: [Source: `tests/coverage-matrix.yaml:752-756`]
- Existing `tests/corpora/MANIFEST.toml` (UNCHANGED by Story 2.3): [Source: `tests/corpora/MANIFEST.toml`]
- Existing `tests/integration/v01_evaluator_path.sh` (UNCHANGED): [Source: `tests/integration/v01_evaluator_path.sh:1-80`]
- Existing `tests/integration/onb_nfr2_timing.sh` (UNCHANGED): [Source: `tests/integration/onb_nfr2_timing.sh:1-60`]
- A6 cold-cache discipline: [Source: `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md:182-187`]
- A7 `default-members = []` + `-p` selection: [Source: `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md:189-193`]
- A8 dev-record gates-citation discipline: [Source: `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md:195-199`]
- ADR-002 Spirit form at v0.1: [Source: `docs/adr/ADR-002-spirit-form-at-v01-subprocess-only-inproc-gated-on-measurement.md`]
- ADR-039 per-module unsafe code policy: [Source: `docs/adr/ADR-039-per-module-unsafe-code-policy.md`]
- Story 1b.6 D10 architecture-doc catch-up precedent: [Source: `_bmad-output/implementation-artifacts/1b-6-epic-2-prep-d9-d10-doc3.md`]
- Story 2.1 Task 9.8 architecture-doc update (19→20 workspace): [Source: `_bmad-output/implementation-artifacts/2-1-…md` Task 9.8]

### Previous-story intelligence (Story 2.1 + Story 2.2 dev records)

**From Story 2.1 (the `#[spirit]` macro + vtable + 11 hooks shipping story):**

1. **Generated vtable symbol name is type-suffixed.** `__maos_spirit_vtable_<Type>()` (not `__maos_spirit_vtable()`) — per Story 2.1 review patch at `crates/maos-spirit-derive/src/lib.rs:281`. Story 2.3's template + example + local_runner test all reference `__maos_spirit_vtable_<Type>()` correctly.

2. **`Ctx` has NO lifetime parameter.** Story 2.1 review patch at `crates/maos-spirit-abi/src/lib.rs:19` corrected the doc comment from `Ctx<'a>` → `Ctx`. The `cancellation: &'static dyn CancellationSignal` field uses `'static` because the kernel owns the underlying signal. Story 2.3 templates + tests use plain `Ctx` (no lifetime).

3. **`Ctx::mock()` is gated `#[cfg(any(test, feature = "mock"))]`.** The `mock` feature on `maos-spirit-abi` propagates through `maos-spirit-sdk`'s `mock = ["maos-spirit-abi/mock"]` feature. The local_runner's `local_runner = ["std", "mock"]` chain ensures `Ctx::mock()` is available wherever the runner is.

4. **`CancellationSignal::cancelled()` default impl hangs.** Per Story 2.1 review patch at `crates/maos-spirit-abi/src/cancellation.rs:53-71`, the default `cancelled()` future never registers a waker. The local_runner uses `Ctx::mock()` which wraps `NeverCancel` — `is_cancelled()` returns `false` synchronously; do NOT use `.cancelled().await` in any Story 2.3 test code.

5. **The `#[spirit]` macro does NOT validate hook method signatures.** Per Story 2.1 review deferred item at `crates/maos-spirit-derive/src/lib.rs:64-83`. Story 2.3's template must use the EXACT signature `fn on_idle(&self, ctx: &mut Ctx)` — wrong signatures produce confusing errors in generated code. The template's `on_idle` is the canonical reference; do not deviate.

6. **`#[hook(budget = "...")]` is parsed but not enforced.** Per Story 2.1 AC + dev record. The template does NOT use this attribute (v0.3 prerequisite keeps the template minimal; Story 7.1 may add it).

**From Story 2.2 (the `xtask check-service-boundary` P1–P4 full enforcement + 24 spirit-boundary cases story):**

1. **P1+P2 enforcement is real now.** AC1 of Story 2.3 must satisfy these — the bridge fixes re-land what 2.2 review reverted. Without them, `cargo run -p xtask -- check-service-boundary --json` exits non-zero on the real workspace.

2. **The Spirit ABI type reflection (AC5 of 2.2) catches hook-count drift.** Adding the `local_runner` module to `maos-spirit-sdk` does NOT change `maos-spirit-abi`'s hook count or vtable layout — verify via `check-service-boundary --json` showing `spirit_abi_types.trait_method_count == 11` + `vtable_field_count == 11` + `hook_names_match == true` after AC3.

3. **The `spirit-boundary-v0.1.jsonl` corpus is content-addressed.** Story 2.3 does NOT extend this corpus — the 24 cases at SHA `015500151e16bc086b69723e6871970b29c35c615f3c5769926117a857bed251` stay frozen. Adding cases requires updating the SHA + bumping the schema_version in MANIFEST.toml (out of scope here).

4. **`xtask/kloc.toml` was raised xtask 3000 → 4000 by Story 2.2.** Task 4.9 verifies xtask stays within 4000 after adding the new sub-command + tests; if exceeded, raise per the precedent.

5. **`p4-mediated-io-paths.toml` exempts `crates/maos-kernel-core/src/io/` etc.** Story 2.3 does NOT add any new io path; the example-spirit drift detector lives in xtask, NOT kernel-core; no exemption update needed.

6. **The Story 2.2 RingCryptoProvider special-case at `check_service_boundary.rs:1581-1585`.** Task 0.6 decides remove-vs-retain. Removing is preferred — the natural P2 port-pair check finds the trait once AC1's re-export lands. Capture the decision in the dev record.

7. **Story 2.2 review (line 707) reverted production code AND Story 2.2 was marked `done` anyway.** The pattern: 2.2's xtask + corpus work shipped (the bulk of the story), the prod-code dependencies were deferred to a bridge. Story 2.3 absorbs the bridge. This is unusual but acceptable per the codebase's bridge-story precedent (1a.5, 1b.6).

### Git intelligence — recent commits

- `9624dbe` — `2-2-xtask-check-service-boundary-p1-p4-full-implementation-spirit-boundary-invariant-cases`. The Story 2.2 commit. **The state of this commit is what Story 2.3 builds on AND fixes the reverted prod-code bits of.** Read the commit diff before opening Task 0.
- `6e8ff8d` — `2-1-ship-the-full-spirit-abi-with-spirit-proc-macro-and-11-lifecycle-hooks`. Story 2.1. The `#[spirit]` macro + vtable + 11 hooks substrate.
- `1bfcc1a` — `1b-6: epic-2 prep bundle — D9 SandboxTier reconciliation + D10 arch-doc + Doc3 unsafe ADR`. The Epic 2 prep bridge.
- `011fcda` — `docs(retro): close Epic 1b — bridge commits land 28/28 CI green`. The 28-job discipline gate is the floor; Story 2.3 takes it to 30 jobs.
- `c7ab9d0` — `fix(ci): repair cap-registry-smoke and onb-nfr2-timing CI scripts`. The bridge commit that fixed A6/A7 root causes. Read this diff before authoring any new integration script in this story (Story 2.3 does NOT author new integration scripts, but the discipline applies to the new CI jobs).

### Latest tech context

- **`cargo-generate`** is at version `0.21.x` as of 2026-05-16 (verify with `cargo search cargo-generate`). The `[placeholders]` + `cargo_generate_version` schema is documented at https://cargo-generate.github.io/cargo-generate/. The `--subfolder` flag has been replaced with positional sub-folder syntax in 0.21+: `cargo generate --git <url> <subfolder>`.
- **Liquid templating filters** (`snake_case`, `kebab_case`, `pascal_case`) are standard cargo-generate filters. The template uses `{{ crate_name | snake_case }}` to convert `example-spirit` → `example_spirit` for module-path references.
- **`tempfile = "3"`** is the standard Rust crate for temp-dir / temp-file management. Verify it's a regular dep in `xtask/Cargo.toml` before Task 4 begins; if only dev-dep, promote.
- **`#[cfg(feature = "local_runner")]`** is the standard Rust cargo-feature gate. The `[features] local_runner = ["std", "mock"]` declaration in `maos-spirit-sdk/Cargo.toml` ensures `cargo build --features local_runner` pulls in std + mock transitively.
- **`std::time::Instant`** + **`std::time::Duration`** are stable std API; the local_runner uses them for hook timing. NO external timing crate needed.
- **`std::collections::BTreeMap`** is the standard ordered map; chosen over `HashMap` for deterministic iteration order in `RunReport.hooks_fired` + `RunReport.elapsed_per_hook` (test assertions can rely on order).
- **`cargo public-api`** is the abi-diff backbone (per Story 1a.5 + Story 2.1). Story 2.3 expects ZERO additions to `maos-spirit-abi`; verify via `cargo run -p xtask -- abi-diff --json`.

### Project Structure Notes

The 21-crate workspace (post Story 2.1) becomes 22 workspace members (20 lib/bin + xtask + `examples/example-spirit`) post Story 2.3. The dependency direction discipline:

- `examples/example-spirit/` depends ONLY on `maos-spirit-sdk` (path = "../../crates/maos-spirit-sdk"). Does NOT depend on `maos-domain`, `maos-kernel-core`, or any other crate. This is the Spirit-author dep-graph reference: a third-party Spirit author's `Cargo.toml` looks like this.
- `templates/spirit-rust/` is NOT a workspace member. The `[workspace] exclude = ["templates"]` declaration in the root Cargo.toml keeps cargo from auto-discovering the template's `Cargo.toml` (which contains `{{crate_name}}` placeholder — not valid Rust syntax).
- `crates/maos-spirit-sdk/src/local_runner.rs` is gated behind the `local_runner` feature. The `mock` feature it depends on is already shipped (Story 2.1). The `std` feature it depends on is default-on. So `cargo build -p maos-spirit-sdk --features local_runner` works out-of-the-box.

The architectural invariant from §4.0.2 (dependencies point inward — adapter ring → kernel services → domain core) is preserved. The Spirit-author-facing crates (`maos-spirit-sdk` + `maos-spirit-derive`) sit at the OUTBOUND edge — kernel code does NOT depend on them. The `local_runner` addition does NOT alter this (it depends only on `maos-spirit-abi` + std).

### Conflicts and variances from architecture

- **Workspace count divergence.** Story 2.3 takes the workspace from 21 → 22 members (adds `examples/example-spirit`). Mitigation: AC8 + Task 7.2 update §4.0.2 in the same PR. Reviewer checklist confirms the doc update.
- **New convention: `examples/*` as workspace members.** §4.0.2 currently lists `examples/` as a directory but does NOT spec it as workspace-member-bearing. The Task 7.2 addendum captures the convention.
- **`templates/*` exclusion.** §4.0.2 does NOT mention `templates/`. The Task 7.2 addendum adds it with the `[workspace] exclude = ["templates"]` rationale.
- **Story 2.2 prod-code revert + Story 2.3 absorption.** The bridge-vs-story pattern from 1a.5 / 1b.6 is extended: prior bridges were SEPARATE stories; Story 2.3 absorbs the 2.2 bridge into Task 0. Captured in dev notes; no architectural rule violated, but the precedent shift should be noted in the Epic 2 retro.
- **NFR-Onb-1 phase-vs-prerequisites framing.** The coverage-matrix update (AC7) cites Story 7.5b as the gate executor — verify this dependency exists in the dependency-dag.md (line 25 confirms it). If a future dependency-DAG revision changes the gate executor, the NFR-Onb-1 row's notes need updating.

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md`] — Epic 2 definition + Story 2.3 ACs; the load-bearing source.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`] — §4.0.2 workspace layout (update target); §4.0.7 what kernel does not compute (boundary for local_runner zero-kernel-dep design).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md`] — §5.1 manifest schema (template manifest reference); §5.3 lifecycle hooks (template references on_idle); §5 update target.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/10-journey-traceability.md`] — §10.6 Diego J6 onboarding (the persona this story serves).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md`] — v0.3 row + NFR-Onb-1 release criterion.
- [Source: `_bmad-output/planning-artifacts/spirit-development-and-sharing.md`] — update target for the callout (Task 7.6).
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md#FR33, FR34`] — cargo-generate + spirit-test SDK FRs.
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md#NFR-Onb-1`] — the 30-Min First Spirit Validation Gate spec.
- [Source: `_bmad-output/planning-artifacts/epics/dependency-dag.md:25,51,66`] — Story 7.5b dependency on Story 2.3 + Story 8.1.
- [Source: `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md`] — A6/A7/A8 retro actions; D9/D10/Doc3 bridge-story precedent.
- [Source: `_bmad-output/implementation-artifacts/1b-6-epic-2-prep-d9-d10-doc3.md`] — bridge-story design pattern.
- [Source: `_bmad-output/implementation-artifacts/2-1-…md`] — Story 2.1 dev record; full vtable + macro substrate.
- [Source: `_bmad-output/implementation-artifacts/2-2-…md`] — Story 2.2 dev record; AC1 bridge fix targets at line 707 + 735.
- [Source: `crates/maos-spirit-abi/src/lifecycle.rs`] — 11-hook trait + SpiritVtable (read-only reference).
- [Source: `crates/maos-spirit-abi/src/ctx.rs`] — Ctx + mock() constructor (read-only reference).
- [Source: `crates/maos-spirit-abi/src/cancellation.rs`] — CancellationSignal + NeverCancel (read-only reference).
- [Source: `crates/maos-spirit-derive/src/lib.rs`] — #[spirit] proc-macro (read-only reference; template + example use its output).
- [Source: `crates/maos-spirit-sdk/src/lib.rs`] — SDK façade (Task 2.2 update target).
- [Source: `crates/maos-spirit-sdk/Cargo.toml`] — SDK feature gates (Task 2.3 update target).
- [Source: `crates/maos-spirit-hello/src/lib.rs`] — hand-written hello-spirit reference; Story 2.3 does NOT migrate it.
- [Source: `spirits/hello-spirit/manifest.toml`] — canonical manifest shape for template.
- [Source: `crates/maos-bin/src/main.rs:80-130, 310-330`] — Task 0.2 target lines.
- [Source: `crates/maos-kernel-core/src/security/mod.rs:1-35`] — Task 0.4 target lines.
- [Source: `xtask/src/check_service_boundary.rs:1581-1585`] — Story 2.2 RingCryptoProvider workaround (Task 0.6 target).
- [Source: `xtask/gate-registry.toml`] — Task 4.4 gate registration.
- [Source: `xtask/Cargo.toml`] — Task 4.1 dep verification.
- [Source: `xtask/kloc.toml`] — Task 4.9 budget verification.
- [Source: `.github/workflows/discipline.yml:414-451, 535-640`] — Task 5 target lines.
- [Source: `tests/coverage-matrix.yaml:289-298, 752-756`] — Task 6 target rows.
- [Source: `tests/integration/v01_evaluator_path.sh`] — Task 0.10 + Task 8.2 cold-cache integration target.
- [Source: `tests/integration/onb_nfr2_timing.sh`] — regression baseline; Story 2.3 verifies it stays green.
- [Source: `docs/adr/ADR-002-spirit-form-at-v01-subprocess-only-inproc-gated-on-measurement.md`] — Spirit form contract (rust-inproc default in template manifest is consistent with this ADR's v0.1 commitment + the hello-spirit precedent).
- [Source: `docs/adr/ADR-039-per-module-unsafe-code-policy.md`] — unsafe-code allowlist; Story 2.3 adds zero unsafe.
- [Source: cargo-generate docs at https://cargo-generate.github.io/cargo-generate/] — template authoring reference.

## Dev Agent Record

### Agent Model Used

deepseek-v4-pro (opencode)

### Debug Log References

- Task 0: Bridge fixes applied — removed 2 dead SecurityManagerAdapter constructions in main.rs, added CryptoProvider re-export in security/mod.rs. RingCryptoProvider workaround retained with clarifying comment (name-mapping still needed).
- Task 1: Template created at templates/spirit-rust/ with 6 files. Added maos-spirit-abi direct dep (required by #[spirit] proc-macro). Used branch = "main" for git dep (tag v0.1-template-seed not yet created).
- Task 2: local_runner module added to maos-spirit-sdk behind `local_runner` feature. 3 smoke tests. Zero kernel-core dep verified.
- Task 3: Example Spirit baked at examples/example-spirit/. Required maos-spirit-abi direct dep + Spirit trait import (proc-macro needs both).
- Task 4: example-spirit-regen xtask sub-command created. tempfile promoted to [dependencies]. Gate registered. Drift detected and resolved.
- Task 5: discipline.yml extended with example-spirit-tests + example-spirit-drift jobs (28→30). Summary table updated.
- Task 6: coverage-matrix.yaml FR33/FR34/NFR-Onb-1 rows updated with gates + notes.
- Task 7: Architecture docs updated — §4.0.2 layout + member count, §5 v0.3 addendum, spirit-dev-and-sharing callout.
- Task 8: All key gates pass locally. Cold-cache integration test passes.

### Completion Notes List

Implemented Story 2.3 (v0.3 NFR-Onb-1 prerequisite): thin cargo-generate template, local_runner SDK seed, baked example Spirit, drift detector, CI jobs, doc updates.

**Key decisions:**
- Bridge fix (AC1): retained RingCryptoProvider workaround — it maps adapter name to port name (not redundant).
- Template deps: added `maos-spirit-abi` as direct dep (proc-macro requires it) + `Spirit` trait import.
- Template git pin: used `branch = "main"` since tag `v0.1-template-seed` doesn't exist yet.
- Coverage-matrix: registered `example-spirit-tests` and `example-spirit-drift` in gate-registry.toml.

### File List

- `crates/maos-bin/src/main.rs` — UPDATE (removed dead SecurityManagerAdapter constructions)
- `crates/maos-kernel-core/src/security/mod.rs` — UPDATE (added CryptoProvider re-export)
- `xtask/src/check_service_boundary.rs` — UPDATE (retained RingCryptoProvider workaround with comment)
- `templates/spirit-rust/cargo-generate.toml` — NEW
- `templates/spirit-rust/Cargo.toml` — NEW
- `templates/spirit-rust/src/lib.rs` — NEW
- `templates/spirit-rust/manifest.toml` — NEW
- `templates/spirit-rust/tests/spirit_smoke.rs` — NEW
- `templates/spirit-rust/README.md` — NEW
- `Cargo.toml` (workspace root) — UPDATE (exclude + example-spirit member)
- `crates/maos-spirit-sdk/src/local_runner.rs` — NEW
- `crates/maos-spirit-sdk/src/lib.rs` — UPDATE (module declaration)
- `crates/maos-spirit-sdk/Cargo.toml` — UPDATE (local_runner feature)
- `crates/maos-spirit-sdk/tests/local_runner_smoke.rs` — NEW
- `examples/example-spirit/Cargo.toml` — NEW
- `examples/example-spirit/src/lib.rs` — NEW
- `examples/example-spirit/manifest.toml` — NEW
- `examples/example-spirit/tests/spirit_smoke.rs` — NEW
- `examples/example-spirit/README.md` — NEW
- `xtask/src/example_spirit_regen.rs` — NEW
- `xtask/src/main.rs` — UPDATE (CLI dispatch)
- `xtask/Cargo.toml` — UPDATE (tempfile dep)
- `xtask/gate-registry.toml` — UPDATE (3 new gates)
- `.github/workflows/discipline.yml` — UPDATE (2 new jobs + summary)
- `tests/coverage-matrix.yaml` — UPDATE (FR33, FR34, NFR-Onb-1)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — UPDATE
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` — UPDATE
- `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` — UPDATE

### Change Log

- 2026-05-16: Implemented Story 2.3 — thin cargo-generate template + local_runner SDK + example Spirit + drift detector + CI jobs + doc updates.

### Review Findings

#### Decision Needed

- [x] [Review][Decision → Patch] Architecture doc workspace member count was wrong — fixed: `4-kernel-design.md` lines 25 + 104 updated from "22" to "21" members (19 crates + xtask + example-spirit). Investigation confirmed no crate is missing; the doc had simply overcounted.

#### Patch

- [x] [Review][Patch] Missing integration test file for `example-spirit-regen` — AC5 requires `xtask/tests/example_spirit_regen_integration.rs` with 4 tests (`check_mode_passes_on_committed_example`, `check_mode_fails_on_drift`, `regen_mode_overwrites_files`, `regen_mode_preserves_readme`). File does not exist. `xtask/tests/` has 10 other integration test files but not this one.
- [x] [Review][Patch] Dead `SecurityManagerAdapter` import at `crates/maos-bin/src/main.rs:41` — After removing the two dead constructions, the short-name import `SecurityManagerAdapter` in the `use maos_kernel_core::api::{...}` line is now unused. Line 316 uses the fully-qualified path `maos_kernel_core::security::SecurityManagerAdapter::new(...)`. Remove `SecurityManagerAdapter` from the import line to eliminate the compiler warning.
- [x] [Review][Patch] Git-to-path Cargo.toml substitution in `example_spirit_regen.rs:131-143` is whitespace-fragile — Uses exact `str::replace` on multi-line TOML entries. Any reordering, reformatting, or whitespace change in `templates/spirit-rust/Cargo.toml` silently breaks the replacement, leaving `git =` deps in the baked output. Should use a more robust approach (e.g., regex with flexible whitespace, or parse/rewrite the TOML `[dependencies]` section programmatically).
- [x] [Review][Patch] `local_runner` smoke tests never run in CI — `crates/maos-spirit-sdk/tests/local_runner_smoke.rs` is gated `#![cfg(feature = "local_runner")]` but no CI job passes `--features local_runner` when testing the SDK. The `example-spirit-tests` job tests `example-spirit` (which transitively exercises the runner), but the SDK's own 3 smoke tests are dead code in CI. Add a CI step or job that runs `cargo test -p maos-spirit-sdk --features local_runner`.

#### Deferred

- [x] [Review][Defer] `--check` mode doesn't use `tempfile::TempDir` — AC5 specified TempDir usage but the implementation compares against committed files directly. Functional but deviates from spec. Deferred: current behavior works for the v0.3 use case.
- [x] [Review][Defer] Drift detector doesn't detect orphan files in `examples/example-spirit/` — Only checks template→example direction. Orphan files in examples/ pass silently. Deferred: one-directional check is the documented v0.3 design.
- [x] [Review][Defer] Hook return values discarded in `fire!` macro — Current Spirit trait hooks return `()`, so no values are lost. Deferred: Story 2.4 may add Result returns.
- [x] [Review][Defer] Cancellation path never exercised in tests — `NeverCancel` always returns false. Deferred: test gap for Story 2.4.
- [x] [Review][Defer] `branch = "main"` vs `tag` documentation — Dev record line 1183 documents the decision. Deferred: inline comments + dev record sufficient for v0.3.
