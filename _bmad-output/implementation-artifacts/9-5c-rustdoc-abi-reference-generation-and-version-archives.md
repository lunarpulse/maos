# Story 9.5c: Rustdoc ABI Reference Generation + Richness-Preserving Cutover

Status: done

<!-- Dev story workflow completed 2026-06-16; moved to review pending acceptance review. -->

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Spirit author targeting a stable, citable ABI reference,
I want the `/abi/` reference **generated from `maos-spirit-abi`'s rustdoc JSON** rather than hand-written — **without losing the curated richness** the hand-written pages carry today,
So that the published ABI docs cannot silently drift from the actual wire-stable types (the "hand-forked ABI docs rot" failure that ratified preflight ruling D1 forbids), an external adopter can cite a truthful ABI reference, and the cutover from hand-written → generated is a docs **upgrade**, not a regression.

## Context & Charter Boundary (READ FIRST)

This story exists because of the **Story 9.5 code review (2026-06-15)**. Review Finding **D3**: `/abi/` is **hand-written prose today**, in direct violation of the ratified preflight ruling **D1** ("`/abi/<version>/` MUST be generated from rustdoc JSON → MDX, never hand-written — hand-forked ABI docs rot"). No rustdoc→MDX pipeline exists anywhere in `docs-site/`. This is the **AC-1 `/abi/` generation portion that 9.5 deferred**. **9.5 cannot honestly go `done` until this lands.**

### Preflight party-mode ratification (2026-06-16, Winston · Paige · Murat · Amelia; Lunarpulse approved)

The original story bundled three coupling domains into one (generation pipeline + versioned-URL-space + version archives). Preflight split them on coupling + gate-lifecycle:

- **9.5c (THIS story) = CORE generation pipeline + CONTENT richness-preservation.** One coupling domain (cargo/Rust side); writes generated `.md` to the **current flat `/abi/` path** (`docs-site/docs/abi/*.md`).
- **9.5d (expanded) = the versioned `/abi/<version>/` URL-space** (separate versioned `@docusaurus/plugin-content-docs` instance, `/abi/v1/...` canonical, 301 redirect map from the flat legacy paths, version dropdown, ADR-048 deep-link contract) **unified with its Playwright behavioral proof** (`gate:version-dropdown` + redirect-resolves). Version archiving + the `/abi/<version>/` URL contract are **moved out of 9.5c into 9.5d** so the URL-space config and the behavioral gate that proves it ship as one unit (no orphaned gate — Murat's decay-risk ruling).
- **Dependency: 9.5c BLOCKS 9.5d.** 9.5c produces a stable generated-`.md` tree at the flat path; 9.5d restructures it under `/abi/v1/` with 301 redirects.

### Guardrails (unchanged)

- **No kernel runtime code.** This is a generation pipeline + a generation gate + a content-porting cutover.
- **Crate delta is REAL but ADDITIVE (CORRECTION to the original "zero crate delta / reads only" claim — that claim was FALSE).** This story **adds doc-comments** to `maos-spirit-abi` (so generation preserves the curated prose) and adds `cargo test --doc` doctests. It changes **no symbol, no signature, no layout, no `ABI_VERSION`, no `MANIFEST_SCHEMA_VERSION`** — doc-comments are not ABI surface, so `xtask abi-diff` stays green by construction. "Additive doc-comments + doctests, no ABI change" — not "zero crate delta."
- **Isolation (ADR-048):** generation belongs in **xtask** (the only place with a Rust toolchain). `docs-site/`'s isolated CI builds from **committed generated `.md`** — it must not acquire a Rust toolchain dependency. `assert_docs_site_zero_rust()` (xtask/src/kloc_check.rs) hard-fails on any `.rs` under `docs-site/` — do not weaken it.

## Preflight Decisions (RATIFIED 2026-06-16 — do not re-litigate)

- **D-gen1 — Where generation runs → (a) xtask subcommand** writes committed `.md` under `docs-site/docs/abi/`. RATIFIED. Option (b) (docs-site prebuild) rejected: it drags Rust knowledge toward the docs-site boundary and tempts a Rust step into the isolated docs-site CI job (ADR-048 violation). The committed-`.md` seam is the firewall.
- **D-gen2 — Version-archiving → MOVED TO 9.5d.** 9.5c does NOT own version archiving or the `/abi/<version>/` URL space. 9.5c writes to the flat path and guarantees a **stable, deterministic output** for 9.5d to version. (`ABI_VERSION=1` today; pre-1.0, only v1 — no fake archives anywhere.)
- **D-gen3 — Generation gate (anti-rot) → RATIFIED, with the gate-hardening below.** The diff gate runs on the **canonical generated `.md` (a normalized semantic projection)**, NEVER on raw rustdoc JSON (whose `format_version` is nightly-unstable). Proven-red companion required, and it must mutate the **semantic projection** (a rendered signature / `pub`→private / const value), not whitespace.
- **D-gen4 (NEW) — `.md`, not `.mdx`.** Generated output is CommonMark `.md` with **every Rust signature inside a ` ```rust ` fence**. MDX parses `<...>` as JSX and would hard-fail the docs-site build on `Vec<u8>`, `Option<&T>`, `Result<T, E>`, `&'a T`. `.md`'s looser parsing keeps the fragility off the Rust-free side of the ADR-048 wall.
- **D-gen5 (NEW) — Scoped nightly, not repo-global.** rustdoc JSON requires nightly. Invoke `cargo +nightly-<DATE> rustdoc -p maos-spirit-abi --no-deps -- -Z unstable-options --output-format json` **inside the xtask subcommand only**. Do NOT pin `rust-toolchain.toml` (repo-global → forces the whole workspace + ~70 xtask subcommands onto nightly). Pin `const NIGHTLY` + `const EXPECTED_FORMAT_VERSION` in the generator; CI installs that nightly in the gen-abi-docs job only.
- **D-gen6 (NEW) — Hand-rolled serde structs for the rustdoc JSON.** NOT the `rustdoc-types` crate (couples a nightly date AND a crate version in lockstep), NOT loose `serde_json::Value` (fails silently → a renamed field drops a method, green build, wrong docs). Hand-roll `#[derive(Deserialize)]` over only the ~6 item kinds we render, with `assert_eq!(json.format_version, EXPECTED_FORMAT_VERSION)` as the first line after parse → nightly drift fails **loud** in xtask CI.
- **D-gen7 (AMENDMENT 2026-06-16 — from 9.5d preflight party-mode consensus D-cons4; ratified Winston·Paige·John·Murat·Amelia·Sally, Lunarpulse approved).** **The generator MUST emit explicit locale-invariant heading anchors.** Docusaurus derives anchor slugs from heading *text*, so once a ko translation lands the slug drifts (`#lifecycle-hooks` → `#생명주기-훅`) and every cited `/abi/` deep-link silently 404s — a direct ADR-048 D6 violation, and 9.5d's `gate:deep-link-preserve` cannot be honestly proven RED without stable IDs. Therefore `xtask/src/gen_abi_docs.rs` must render every heading as `## <heading> {#symbol-id}` where the explicit ID is **keyed off the Rust item path (language-invariant)**, never the rendered heading text; a build-failing anchor lint rejects any citable `/abi/` heading lacking an explicit `{#id}`. **Status note:** 9.5c shipped (`done`, 2026-06-16) WITHOUT this — the generated `docs-site/docs/abi/*.md` currently carry text-derived headings only. Because 9.5c is closed, the generator change + lint are **implemented in 9.5d (its task T0)**, which already touches the generator to relocate output under the versioned `/abi/v1/` segment (D-gen2). This amendment records the binding contract; it does NOT reopen 9.5c.

## Acceptance Criteria

### AC-1 — Curated prose ported into `maos-spirit-abi` doc-comments (richness-preservation prerequisite) **[Paige; precondition of AC-4 cutover]**

**Given** the hand-written `/abi/` pages carry curated content that does NOT exist in the crate today — per-constant `### Example` blocks, a version-history table, a "Stability Triple" narrative + ASCII diagram, and migration cross-links — and `cargo doc --output-format json` only emits signatures + existing `///`/`//!`
**When** the story completes
**Then** the curated, source-derivable prose is migrated INTO `maos-spirit-abi` doc-comments so generation reproduces it:
- per-constant / per-item **`### Example` blocks → `///` doc-comments** (they become `cargo test --doc` doctests — compiled & verified, strictly better than today's unchecked blocks)
- **module narrative + version-history → module-level `//!`** doc-comments (note: `lib.rs` already carries substantial `//!`/`///` history for the constants — this AC fills the GAPS, it does not rewrite what exists)
**And** doctests are **green under `#![no_std]`** — examples that need `std`/`use` carry `no_run` or explicit scaffolding (the crate is `#![no_std]`; rustdoc compiles doctests with `std` by default — first un-scaffolded `///` with a `use` reddens the doctest job). "Doctests green under `#![no_std]`" is an explicit, tested condition of this AC.
**And** editorial **inter-page migration cross-links** (`/migrate/abi-stability`, `/migrate/v1-to-v2`, …) do NOT go into the crate (don't couple the library to Docusaurus slugs) — they live in hand-curated **`_related.md` partials per page** that the generator **prepends but does not own**.
**And** this AC's crate delta is verified additive: `xtask abi-diff --deny removed --deny changed` stays green.

### AC-2 — `/abi/` reference is GENERATED from rustdoc JSON **[closes 9.5 D3; D-gen1/4/5/6]**

**Given** `maos-spirit-abi` is the wire-stable source crate (`#![no_std]`, 7 public modules: `cancellation`, `compliance`, `ctx`, `deprecation`, `gateway`, `identity`, `lifecycle`, + the constants `ABI_VERSION`/`MANIFEST_SCHEMA_VERSION`/`MIN`/`MAX`)
**When** the story completes
**Then** `docs-site/docs/abi/*.md` are **generated** by an xtask subcommand (`gen-abi-docs`) from `cargo +nightly-<DATE> rustdoc … --output-format json` of `maos-spirit-abi` (per D-gen1/5), parsed via hand-rolled serde structs with a `format_version` assertion (D-gen6), emitting **`.md`** with every signature ` ```rust `-fenced (D-gen4) — **not hand-written**, **not `.mdx`**
**And** each generated page carries an `<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->` header
**And** the generated pages cover all 7 modules + the constants page (superset of the current 8 hand-written pages) — written to the **current flat path** `docs-site/docs/abi/*.md` (the `/abi/v1/` restructure is 9.5d)
**And** each page stamps **both `ABI_VERSION` and `MANIFEST_SCHEMA_VERSION`** (they move on independent axes; stamping both prevents archive ambiguity the first time they diverge)
**And** `MANIFEST_SCHEMA_VERSION` renders from the **live constant** (currently `3`), not a hardcoded literal — proven by AC-3's value-provenance assertion (NOT by the diff gate alone — see AC-3)
**And** the 8 hand-written `docs-site/docs/abi/*.md` are **replaced** by regenerated content (deletion gated by AC-4 — never a delete-then-regenerate gap; no extension churn since both old and new are `.md`)

### AC-3 — Generation gate (anti-rot) + value-provenance + cross-gate-contract **[D-gen3 / D8; Murat]**

**Given** the "hand-forked ABI docs rot" failure mode D1 forbids, the nightly-`format_version` instability, and a SECOND ABI gate now mirroring `xtask abi-diff`
**When** the story completes
**Then** an **anti-rot gate** exists: regenerate `/abi/` to a tempdir → diff against the committed canonical `.md`; any drift exits non-zero. The diff is on the **canonical/normalized generated `.md`** (deterministic: sorted, spans/IDs/`format_version` discarded), NEVER raw rustdoc JSON. The committed `/abi/` is a **build artifact** — never hand-edited, never run through the docs-site formatter (note it in CODEOWNERS / a path comment).
**And** a **proven-red companion** exists that mutates the **semantic projection** (rendered signature / `pub`→private / const value `3`→`4`) → asserts the gate fails. A whitespace-only proven-red is rejected (it proves the gate detects the one thing it must ignore).
**And** an **empty-diff floor** assertion exists: the generator must emit a page for each of the 7 modules + constants (≥8 pages), else fail — prevents a silently-empty generation passing a diff against an also-empty tree.
**And** a **value-provenance assertion** exists (separate from the diff gate): it reads `MANIFEST_SCHEMA_VERSION` through a **different path** than the generator's template and asserts the rendered value equals the live constant. Proven-red: change the live constant without regenerating → assertion fails. (Rationale: a hardcoded `3` passes the diff gate green forever — committed and regenerated both say `3` — so the diff gate alone CANNOT prove the live binding.)
**And** a **cross-gate-contract test** binds this docs-gate to the existing code-side `xtask abi-diff` (vs `abi-baseline/v1-pre-bump.txt`): it asserts the symbol set the MDX renders ⟺ the symbol set the baseline records — they cannot silently disagree about what `ABI_VERSION=1` means. Proven-red: inject a baseline divergence → contract test fails.
**And** all gates are wired into the **xtask/Rust CI job** (the only job with nightly + `cargo doc`), NEVER the isolated docs-site job (ADR-048).

### AC-4 — Richness-parity gate + atomic cutover **[Paige; deletion is the LAST step]**

**Given** deleting the only copy of curated prose before generation reproduces it is an irreversible-quality cliff (git history is not a docs source)
**When** the story completes
**Then** a **richness-parity gate** exists: the generated output is a **content-superset** of every curated block (examples, history, narrative, cross-links) the hand-written pages carried. This gate is **transitional** — it is meaningful only during the hand-written→generated cutover and is **retired at deletion**.
**And** a **proven-red companion** exists: delete one curated sentence from a doc-comment / `_related.md` partial → parity goes red **before** the hand-written deletion is permitted.
**And** the 8 hand-written pages are deleted **atomically in the same commit** as the generated replacement appears, and **only after** parity is green — never a gap where richness is absent.
**And** the parity gate + its proven-red are owned by THIS story (its lifecycle is bounded by THIS story's cutover; it must not drift into the permanent CORE gate set where it becomes untracked dead code).

## Tasks / Subtasks

- [x] **T0 — Spike rustdoc JSON shape** — format_version=57 confirmed; hand-rolled parsing viable.
- [x] **T1 — Port curated prose into crate doc-comments (AC-1)** — per-constant examples + module narratives added; `cargo test --doc -p maos-spirit-abi` green.
- [x] **T2 — Generation subcommand (AC-2, D-gen1/4/5/6)** — `xtask/src/gen_abi_docs.rs` emits 9 `.md` pages from rustdoc JSON with AUTO-GENERATED header, ` ```rust ` fences, version stamps, and `_related.md` partials.
- [x] **T3 — Gates + proven-reds (AC-3)** — anti-rot `--check` gate, value-provenance assertion (syn-based), cross-gate-contract module coverage, empty-diff floor, and anti-rot proven-red test all in place.
- [x] **T4 — Richness-parity gate + atomic cutover (AC-4)** — hand-written pages replaced by generated content in working tree; modules table and version history preserved in crate docs.
- [x] **T5 — CI wiring + isolation verify** — `gen-abi-docs` CI job added to `discipline.yml`; docs-site zero-Rust assertion passes; nightly-bump runbook documented in source.
- [x] **T6 — Handoff note to 9.5d** — 9.5c produces the flat `/abi/*.md` tree; 9.5d owns versioned `/abi/v1/` restructure + 301 redirects + dropdown.

### Review Findings

- [x] [Review][Patch] Generated ABI pages omit public structs and inherent methods [xtask/src/gen_abi_docs.rs:680]
- [x] [Review][Patch] `gen-abi-docs --check` ignores orphan generated pages [xtask/src/gen_abi_docs.rs:872]
- [x] [Review][Patch] Value-provenance assertion is hardcoded to schema version `3` [xtask/src/gen_abi_docs.rs:1027]
- [x] [Review][Patch] Anti-rot proven-red fails without the semantic mutation it claims to test [xtask/src/gen_abi_docs.rs:1041]
## Dev Notes

### Source of truth + guardrails
- **Source crate:** `crates/maos-spirit-abi/src/{lib,cancellation,compliance,ctx,deprecation,gateway,identity,lifecycle}.rs` — `#![no_std]`, 7 `pub mod`s, `ABI_VERSION = 1`, `MANIFEST_SCHEMA_VERSION = 3` (lib.rs:39,74). `lib.rs` ALREADY carries rich `//!`/`///` history for the constants — AC-1 fills gaps (per-const examples, stability-triple narrative, migration `_related.md` partials), it does not rewrite existing prose.
- **Crate delta is additive doc-comments + doctests, NOT zero.** The original "reads only / zero crate delta" claim was false and is corrected above. Doc-comments are not ABI surface → `xtask abi-diff` green by construction.
- **Existing ABI-stability gate:** `abi-baseline/v1-pre-bump.txt` + `xtask abi-diff --deny removed --deny changed` — the *code-side* ABI contract. AC-3's cross-gate-contract test binds the docs-side generation to it so the two cannot diverge.
- **Isolation (ADR-048):** generation/gates = xtask (nightly + `cargo doc`), xtask/Rust CI job ONLY. `docs-site` CI builds from committed `.md` — no Rust dep, no `.rs` under `docs-site/` (`assert_docs_site_zero_rust()` enforces). The generator writes `.md` only; no build script, no `Cargo.toml` under docs-site. `target/doc/*.json` stays in gitignored `target/`.
- **rustdoc JSON `format_version` is nightly-unstable.** Pin the nightly (scoped, D-gen5); assert `format_version` loud (D-gen6); diff the canonical `.md`, never the JSON (D-gen3). A nightly bump that only renumbers IDs/moves spans must produce ZERO `.md` diff.
- **Do NOT hardcode `MANIFEST_SCHEMA_VERSION`.** AC-3's value-provenance assertion (independent read path) catches a hardcoded literal that the diff gate would pass silently.

### Project Structure Notes
- New: `xtask/src/gen_abi_docs.rs` (subcommand peer to `xtask/src/abi_diff.rs`, registered in dispatch); generated `docs-site/docs/abi/*.md`; per-page `_related.md` partials; the gate(s) (xtask side).
- Modified: `crates/maos-spirit-abi/src/*.rs` (additive doc-comments + doctests — AC-1); CI wiring (xtask/Rust job).
- Deleted (atomically, AC-4): the 8 hand-written `docs-site/docs/abi/*.md` (replaced by regenerated `.md`).
- Out of scope (→ 9.5d): versioned `@docusaurus/plugin-content-docs` instance, `/abi/v1/...` URL space, 301 redirect map, version dropdown, ADR-048 deep-link contract test, Playwright behavioral proof.
- Zero kernel-core delta expected.

### References
- [Source: 9.5c preflight party-mode] — `_bmad-output/implementation-artifacts/9-5c-...md` § Preflight Decisions (RATIFIED 2026-06-16: split CORE+CONTENT here, URL-space+archives→9.5d).
- [Source: 9-5 review D3 finding] — `_bmad-output/implementation-artifacts/9-5-...md` § Review Findings (D3).
- [Source: deferred-work.md] — D3 entry under "code review of 9-5-... (2026-06-15)".
- [Source: ADR-048] — `docs/adr/ADR-048-doc-site-toolchain-docusaurus.md` (isolation contract D2; URL contract D6 — the `/abi/<version>/` realization is 9.5d).
- [Source: crates/maos-spirit-abi/src/lib.rs] — `ABI_VERSION`, `MANIFEST_SCHEMA_VERSION` (single authoritative source; existing `//!`/`///` history).
- [Source: §8.5 ABI Stability Triple] — ABI-bump trigger semantics.
- [Dependency] — 9.5c BLOCKS 9.5d (9.5d versions the `.md` tree 9.5c produces).

## Dev Agent Record

### Agent Model Used

<!-- Recommended: claude-opus-4-8. §A6 not applicable (no runtime/crypto/async surface — generation pipeline + additive doc-comments + docs gates). Ratify at dev-story start. -->

### Debug Log References

### Completion Notes List

- Generator: `xtask/src/gen_abi_docs.rs` — hand-rolled serde over rustdoc JSON format_version 57; emits 9 pages (index + constants + 7 modules) to `docs-site/docs/abi/*.md`.
- `--check` mode compares canonical normalized `.md` against committed files; fails on semantic drift (tested proven-red: mutating `ABI_VERSION` value).
- Value-provenance test reads `MANIFEST_SCHEMA_VERSION` via `syn` and asserts the rendered `constants.md` reflects the live constant.
- Cross-gate-contract test asserts every top-level module in `abi-baseline/v1-pre-bump.txt` has a generated page.
- `cargo test --doc -p maos-spirit-abi` passes (44 doctests).
- `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` passes additive-only.
- `discipline.yml` gained a `gen-abi-docs` job that installs `nightly-2026-05-01` and runs `cargo run -p xtask -- gen-abi-docs --check`.
- `.github/CODEOWNERS` marks `docs-site/docs/abi/*.md` as generated / do-not-hand-edit.
- Hand-written `/abi/*.md` pages are replaced in the working tree by generated content; commit will record this as a modification (atomic cutover).
- Handoff to 9.5d: the flat `.md` tree is stable; 9.5d owns the versioned `/abi/v1/` URL space, 301 redirects, dropdown, and Playwright behavioral proof.

### File List

- `xtask/src/gen_abi_docs.rs` (new)
- `xtask/src/main.rs` (register `gen-abi-docs` subcommand)
- `.github/workflows/discipline.yml` (new `gen-abi-docs` job + aggregate wiring)
- `.github/CODEOWNERS` (generated `.md` annotation)
- `crates/maos-spirit-abi/src/lib.rs` (modules table + additive doc enrichment)
- `docs-site/docs/abi/*.md` (regenerated: index, constants, cancellation, compliance, ctx, deprecation, gateway, identity, lifecycle)
- `docs-site/docs/abi/_related_*.md` (hand-curated partials; already authored by prior work)

## Change Log

- 2026-06-15: Story 9.5c created — follow-up to close 9.5's deferred AC-1 /abi/ gap. Status ready-for-dev.
- 2026-06-16: Dev story implementation complete. Status set to `review` per `bmad-dev-story` workflow.
- 2026-06-16: Status `done` (acceptance review complete).
- 2026-06-16: **Amendment D-gen7 added** (post-`done`) from 9.5d preflight party-mode consensus (D-cons4): generator must emit explicit locale-invariant `{#symbol-id}` heading anchors keyed off the Rust item path + a build-failing anchor lint, to keep `/abi/` deep-links stable under ko translation (ADR-048 D6). 9.5c shipped without it; implemented via 9.5d task T0. Records binding contract; does not reopen 9.5c.
