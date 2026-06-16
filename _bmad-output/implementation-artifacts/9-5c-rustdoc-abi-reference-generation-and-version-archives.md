# Story 9.5c: Rustdoc ABI Reference Generation + Version Archives

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Spirit author targeting a stable, citable ABI reference,
I want the `/abi/<version>/` reference **generated from `maos-spirit-abi`'s rustdoc JSON** rather than hand-written,
So that the published ABI docs cannot silently drift from the actual wire-stable types (the "hand-forked ABI docs rot" failure that ratified preflight ruling D1 forbids), and an external adopter can cite a specific, truthful ABI version.

## Context & Charter Boundary (READ FIRST)

This story exists because of the **Story 9.5 code review (2026-06-15)**. Review Finding **D3**: `/abi/` is **hand-written prose today**, in direct violation of the ratified preflight ruling **D1** ("`/abi/<version>/` MUST be generated from rustdoc JSON → MDX, never hand-written — hand-forked ABI docs rot"). No rustdoc→MDX pipeline exists anywhere in `docs-site/`. Additionally AC-1's "archived ≥2 minor versions back" is unmet (only a single `current` version exists).

This is the **AC-1 `/abi/` portion that 9.5 deferred**. **9.5 cannot honestly go `done` until this lands.**

- **No kernel runtime code.** This is a generation pipeline + a generation gate + a version-archiving mechanism.
- **Zero kernel-core delta.** `maos-spirit-abi` is `#![no_std]`/`#![forbid(unsafe_code)]`, already wire-stable; this story **reads** its rustdoc, it does not change the crate.
- **Isolation (ADR-048):** generation belongs in **xtask** (the only place with a Rust toolchain). `docs-site/`'s isolated CI builds from **committed generated MDX** — it must not acquire a Rust toolchain dependency.

## Preflight Decisions (ratify at story start — or via party-mode)

- **D-gen1 — Where does generation run?** Options: (a) an **xtask subcommand** writes committed MDX under `docs-site/docs/abi/` (Rust knowledge stays in xtask; docs-site CI stays pure — **recommended**, honors ADR-048 isolation); (b) a docs-site prebuild script reads a committed rustdoc-JSON artifact (docs-site gains a JSON-transform dep, stays Rust-free). **Recommend (a).**
- **D-gen2 — Version-archiving scope.** `ABI_VERSION` is currently `1` (single version). Establish the **freeze mechanism now** (on each ABI bump per §8.5, snapshot generated `/abi/v<N>` into `docs-site/versioned_docs`), but **do not claim ≥2 archived versions exist** — pre-1.0, only v1. Honest scoping; the "≥2" fills as ABI bumps accumulate.
- **D-gen3 — Generation gate (anti-rot).** A "regenerate to tempdir → `git diff --no-index` against committed `/abi/` → exit 0" gate proving committed pages match current rustdoc (prevents hand-edits re-creeping). **Proven-red companion required (D8)** — mutate a committed page → gate fails.

## Acceptance Criteria

### AC-1 — `/abi/` reference is GENERATED from rustdoc JSON **[closes 9.5 D3 / AC-1]**

**Given** `maos-spirit-abi` is the wire-stable source crate (`#![no_std]`, 7 public modules: `cancellation`, `compliance`, `ctx`, `deprecation`, `gateway`, `identity`, `lifecycle`, + the constants `ABI_VERSION`/`MANIFEST_SCHEMA_VERSION`/`MIN`/`MAX`)
**When** the story completes
**Then** `docs-site/docs/abi/*.mdx` are **generated** from `cargo doc --output-format json` (nightly `RUSTDOCFLAGS='-Z unstable-options --output-format json'`) of `maos-spirit-abi` via an xtask subcommand (per D-gen1), **not hand-written**
**And** each generated page carries an `<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->` header
**And** the generated pages cover all 7 modules + the constants page (superset of the current 8 hand-written pages)
**And** `MANIFEST_SCHEMA_VERSION` renders from the **live constant** (currently `3`), not a hardcoded literal — verified by the generation diff gate
**And** the 8 hand-written `docs-site/docs/abi/*.md` are **deleted** (clean cutover — no stale hand-written copies left beside the generated output)

### AC-2 — Generation gate (anti-rot) **[D8]**

**Given** the "hand-forked ABI docs rot" failure mode D1 forbids
**When** the story completes
**Then** a generation gate exists: regenerate `/abi/` to a tempdir, diff against committed `/abi/`; any drift exits non-zero
**And** a **proven-red companion** exists (mutate a committed `/abi/` page → assert the gate fails)
**And** the gate is wired into CI (the xtask gate set or a docs-site step — placement must respect the ADR-048 isolation contract per D-gen1)

### AC-3 — Version-archive mechanism **[AC-1 archive clause]**

**Given** the URL contract `/abi/<version>/` and AC-1's "archived ≥2 minor versions back" (qualified by D-gen2: `ABI_VERSION=1` today)
**When** the story completes
**Then** a **documented freeze procedure** exists: on each `ABI_VERSION` bump (per §8.5), snapshot current generated `/abi/v<N>` into `docs-site/versioned_docs` and surface it in the version dropdown
**And** the version dropdown (`docusaurus.config.ts`) is wired to render archived ABI versions **when they exist** (currently renders only "v1 (latest)" — honest; no fake archived entries)
**And** `/abi/latest` always redirects to the current ABI version

## Tasks / Subtasks

- [ ] **T1 — Generation script (AC-1, D-gen1)** — xtask subcommand `gen-abi-docs`: build rustdoc JSON for `maos-spirit-abi`, transform per-module to MDX, write under `docs-site/docs/abi/` with the AUTO-GENERATED header. Pin/document the nightly toolchain requirement.
- [ ] **T2 — Clean cutover (AC-1)** — delete the 8 hand-written `docs-site/docs/abi/*.md`; regenerate; verify the route-manifest `/abi/*` routes still resolve with their content floors.
- [ ] **T3 — Generation gate + proven-red (AC-2, D-gen3)** — `gate:abi-generated` (regenerate→diff); proven-red companion.
- [ ] **T4 — Version-archive mechanism (AC-3, D-gen2)** — freeze procedure doc; version-dropdown wiring; `/abi/latest` redirect.
- [ ] **T5 — CI wiring + isolation verify** — wire the generation gate; confirm zero kernel-core delta; confirm docs-site CI still has no Rust toolchain dependency (it consumes committed MDX only).

## Dev Notes

### Source of truth + guardrails
- **Source crate:** `crates/maos-spirit-abi/src/{lib,cancellation,compliance,ctx,deprecation,gateway,identity,lifecycle}.rs` — `#![no_std]`, 7 `pub mod`s, `ABI_VERSION = 1`, `MANIFEST_SCHEMA_VERSION = 3` (in `lib.rs:39,74`).
- **Existing ABI-stability gate:** `abi-baseline/v1-pre-bump.txt` + `xtask abi-diff --deny removed --deny changed` — the *code-side* ABI-stability contract. This story's generation is the **docs-side mirror** of that same contract; the two must agree on what `ABI_VERSION=1` means.
- **Isolation (ADR-048):** generation = xtask (Rust toolchain). `docs-site` CI builds from **committed** generated MDX — no Rust dep in the isolated job. The `_docs_site_isolation` contract is now ENFORCED by `assert_docs_site_zero_rust()` (landed in 9.5's P-iso patch) — do not weaken it.
- **rustdoc JSON requires nightly.** Document the toolchain pin explicitly; the gate fails loud if the pinned nightly is absent.
- **Do NOT hardcode `MANIFEST_SCHEMA_VERSION`.** The generated constants page reads the live constant; the generation-diff gate enforces this.

### Project Structure Notes
- New: xtask generation subcommand (e.g., `xtask/src/gen_abi_docs.rs`); generated `docs-site/docs/abi/*.mdx`; a generation-gate script.
- Deleted: the 8 hand-written `docs-site/docs/abi/*.md`.
- Modified: `docs-site/docusaurus.config.ts` (version dropdown), `docs-site/sidebars.ts` (if needed), CI wiring.
- Zero kernel-core delta expected.

### References
- [Source: 9-5 review D3 finding] — `_bmad-output/implementation-artifacts/9-5-...md` § Review Findings (Decision Resolutions D3).
- [Source: deferred-work.md] — D3 entry under "code review of 9-5-... (2026-06-15)".
- [Source: ADR-048] — docs-site isolation contract (D2 / Hard Guardrails).
- [Source: crates/maos-spirit-abi/src/lib.rs] — `ABI_VERSION`, `MANIFEST_SCHEMA_VERSION` constants (the single authoritative source).
- [Source: §8.5 ABI Stability Triple] — ABI-bump trigger semantics for the version-archive freeze.

## Dev Agent Record

### Agent Model Used

<!-- Recommended: claude-opus-4-8. §A6 not applicable (no runtime/crypto/async surface — pure generation + docs). Ratify at dev-story start. -->

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-06-15: Story 9.5c created — follow-up to close 9.5's deferred AC-1 /abi/ gap (rustdoc-JSON→MDX generation + version archives). Spawned from 9.5 code review D3. Status ready-for-dev.
