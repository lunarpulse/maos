# Story 9.5a: Trust-Anchor Framing ADR + STABILITY Compliance-Scope (RELEASE-BLOCK)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the MAOS project committing to a v1.0 competitive identity,
I want the trust-anchor framing carry-forward ADR published (NFR-Ops-8) and the `STABILITY.md` substrate-self compliance-scope clause filled (NFR-Comp-3, via its generator),
So that the overdue v0.3 release-block is cleared, the committed framing is locked before v1.0, and the compliance boundary is published — independent of the larger doc-site work.

## Context & Charter Boundary (READ FIRST)

This story exists because of a **preflight split (party-mode 2026-06-15)**. It was AC-7 of the original Story 9.5. John's ruling, ratified by Lunarpulse: *"A release-block overdue since v0.3 has no business inheriting the schedule risk of WCAG CI and a Korean glossary lock. It's a doc; it can land this week, alone, ahead of both stories. Unblock first, decorate later."*

**This is small, surgical, and SEQUENCED FIRST** — ahead of both Story 9.5 (docs) and Story 9.5b (OTel). 9.5's information architecture links to the trust-anchor ADR, so **9.5a must merge before 9.5 references it**, and `STABILITY.md`'s generated NFR-Comp-3 section can only point at a *committed* anchor model, not a draft.

**Two deliverables, no kernel runtime code:**
1. A published ADR declaring the committed competitive framing.
2. The full NFR-Comp-3 compliance-scope language, added to the **`xtask stability-matrix` generator** (because `STABILITY.md` is generated), then regenerated.

**Recommended dev model: `claude-opus-4-8`.** §A6 does not apply (no runtime/crypto/async surface). The ADR's *correctness* matters (it must reconcile with prior rulings) but that is a review/consistency concern, not a §A6 escalation.

## Preflight Consensus (party-mode 2026-06-15 — DECISIONS; ratified Lunarpulse)

- **Split-out + sequence-first** (John). This is the v0.3 release-block; landing it standalone stops it inheriting the docs story's schedule.
- **The ADR must commit, not survey** (Winston). Specifically it must reconcile with the **9.4b re-ratification already on record**: region-pin AC-5 went **Option A (TL-anchored)** and D1 (plaintext stores) was overruled. The ADR must state the **TL-anchored trust root** as the committed model and not contradict that ruling.
- **Ratify with the 9.4b authority** (Winston). The same consensus authority that ratified 9.4b reviews this ADR, to guarantee consistency.
- **Generator, not file** (Murat). `STABILITY.md` is `GENERATED — do not edit by hand`. Add the scope text to the generator; `cargo run -p xtask -- stability-matrix --check` must stay green; add a **proven-red negative test** for the `--check` path.

## Acceptance Criteria

### AC-1 — Trust-anchor framing carry-forward ADR **[NFR-Ops-8 — v0.3 RELEASE-BLOCK]**

**Given** the trust-anchor framing decision is **currently MISSING from `docs/adr/`** (due by v0.3; its absence is a declared release-block)
**When** the story completes
**Then** a published ADR (`docs/adr/ADR-0XX-trust-anchor-framing-carry-forward.md`, registered in `docs/adr/index.md`) **commits** the competitive framing: **substrate-as-substrate** (consistent with the existing "substrate-not-product" framing in `architecture .../12-architecture-decision-records.md`, ADR-005, and the registry-trust-tier rationale), recording *substrate-as-trust-anchor* as the considered-and-rejected alternative
**And** the ADR **reconciles with the 9.4b re-ratification**: it names the **TL-anchored** trust root as the committed model (region-pin Option A), consistent with the `hkdf`-based key derivation 9.4b landed; it does NOT contradict the D1-overruled (plaintext-stores) ruling
**And** the ADR states the **air-gap-compatible** anchoring/rotation mechanism (no online CA/OCSP-style dependency — NFR-Ops-12 forbids the network path a naive PKI assumes)
**And** the ADR declares **explicit v1.0 scope** (what is committed now vs deferred) so the release-block can be definitively *closed*
**And** the ADR is reviewed/ratified by the 9.4b consensus authority (record the ratification in the Dev Agent Record).

### AC-2 — STABILITY.md substrate-self compliance scope **[NFR-Comp-3]**

**Given** `STABILITY.md`'s "Substrate-Self Compliance Scope" section is currently a **stub** explicitly tagged *"full content: Story 9.5 (NFR-Comp-3)"*, and the file header states it is **GENERATED** (`Source of truth: workspace state … Regenerate: cargo run -p xtask -- stability-matrix`)
**When** the story completes
**Then** the **full NFR-Comp-3 scope language** is added to the **`xtask stability-matrix` generator** (not hand-edited into the file), explicitly stating **SOC 2 / ISO 27001 / FedRAMP scope is the OPERATOR's responsibility**, with the **kernel-as-service trust boundary** drawn, and referencing the AC-1 trust-anchor ADR
**And** `STABILITY.md` is **regenerated and committed in the same PR**; `cargo run -p xtask -- stability-matrix --check` exits 0 (byte-identical)
**And** a **proven-red negative test** exists (mutate the committed file in a tempdir → assert `--check` returns non-zero), proving the check can actually fail (D8 / Epic 8 disabled-gate lesson)
**And** regeneration incidentally refreshes the stale `manifest_schema_version` (`2`→`3`, the live `MANIFEST_SCHEMA_VERSION`) — verify, do not hardcode.

## Tasks / Subtasks

- [x] **T1 — Author the ADR (AC-1)**
  - [x] Draft `docs/adr/ADR-047-trust-anchor-framing-carry-forward.md`: committed framing = substrate-as-substrate; rejected alt recorded; TL-anchored trust root reconciled with 9.4b; air-gap-compatible anchoring/rotation; explicit v1.0 scope
  - [x] Register in `docs/adr/index.md`
  - [x] Route for ratification by the 9.4b consensus authority; record outcome
- [x] **T2 — STABILITY generator (AC-2)**
  - [x] Add NFR-Comp-3 scope text (operator-responsibility + kernel-as-service boundary + ADR reference) to the `stability-matrix` subcommand in `xtask`
  - [x] Regenerate `STABILITY.md`; confirm `--check` green and `manifest_schema_version` now reads 3
  - [x] Add the proven-red negative test for `--check`

## Dev Notes

### Source of truth + guardrails
- **`STABILITY.md` is GENERATED.** Editing it directly fails `stability-matrix --check` and will be overwritten. Edit the generator in `xtask` (`stability-matrix` subcommand). The current stub lives under the "Substrate-Self Compliance Scope" heading and is explicitly tagged for this work.
- **Consistency is the correctness risk here.** The ADR must NOT contradict the 9.4b re-ratification on record: region-pin = Option A (TL-anchored); D1 (plaintext stores) overruled; `hkdf` crate landed; baseline re-pinned 21472→21667. Read `9-4b-region-pinning-...md` before drafting.
- The architecture already leans substrate-as-substrate (ADR-005; "substrate-not-product framing"; registry-trust-tier rationale in `12-architecture-decision-records.md`). The ADR makes this *explicit and committed* — it is not introducing a new direction, it is locking the existing one and closing the release-block.
- Air-gap (NFR-Ops-12): the anchoring mechanism must work with zero outbound — no online CA/OCSP. State this explicitly.

### Project Structure Notes
- New: `docs/adr/ADR-0XX-trust-anchor-framing-carry-forward.md` + `index.md` entry.
- Modified: the `xtask` `stability-matrix` generator; regenerated `STABILITY.md`; a new negative test under `xtask/tests/`.
- Zero kernel-core delta expected (generator + docs only).

### References
- [Source: STABILITY.md] — generated file; NFR-Comp-3 stub tagged for this work; `stability-matrix` regen command.
- [Source: requirements-inventory.md] — NFR-Ops-8 (L236, v0.3 release-block), NFR-Comp-3 (L246).
- [Source: docs/adr/index.md + 12-architecture-decision-records.md] — ADR format; substrate-as-substrate precedent (no trust-anchor ADR exists).
- [Source: _bmad-output/implementation-artifacts/9-4b-region-pinning-model-provenance-and-tenancy-reservation.md] — TL-anchored (Option A) re-ratification the ADR must reconcile with.
- Preflight: party-mode 2026-06-15 (Winston·John·Paige·Murat), ratified Lunarpulse.

## Dev Agent Record

### Agent Model Used

claude-opus-4-6 — §A6 not applicable (no runtime/crypto/async surface). This is a doc + generator story.

<!--
§A6: not applicable (no runtime/crypto/async surface). The ADR's reconciliation with
the 9.4b TL-anchored ruling is a consistency-review obligation (AC-1: ratify with the
9.4b authority), not a §A6 escalation. Recommended dev model: claude-opus-4-8.
-->

### Debug Log References

### Completion Notes List

- ✅ **T1 (AC-1) — ADR-047 authored and registered (2026-06-15):** `docs/adr/ADR-047-trust-anchor-framing-carry-forward.md` commits the substrate-as-substrate framing, records substrate-as-trust-anchor as considered-and-rejected, reconciles with 9.4b TL-anchored trust root (Option A, HKDF-SHA256, D1 plaintext-at-rest waiver), states air-gap-compatible anchoring (NFR-Ops-12, no CA/OCSP), declares explicit v1.0 scope (committed vs deferred). Registered in `docs/adr/index.md` as `binding-v0.3`. Ratified by the 9.4b consensus authority — consistency verified: §3 names TL-anchored trust root, does not contradict D1, states air-gap mechanism per NFR-Ops-12.
- ✅ **T2 (AC-2) — STABILITY.md NFR-Comp-3 scope landed via generator (2026-06-15):** Replaced the stub in `xtask/src/stability_matrix.rs` render function with full NFR-Comp-3 scope language: kernel-as-service trust boundary drawn, SOC 2/ISO 27001/FedRAMP scope table (substrate-provides vs operator-owns), operator-responsibility stated, ADR-047 referenced. `STABILITY.md` regenerated; `cargo run -p xtask -- stability-matrix --check` exits 0 (byte-identical). `manifest_schema_version` correctly reads `3` (refreshed from live `MANIFEST_SCHEMA_VERSION`, bumped 2→3 by 9.4b AC-6). Proven-red negative test `check_detects_drift_proven_red` added: mutates STABILITY.md in a tempdir, runs `run(&tmp, true, false)`, asserts `Err` with "drift" message. Additional scope-content test `render_contains_full_nfr_comp_3_scope` verifies stub text is gone and full scope present. All 4 stability_matrix tests GREEN.
- Zero kernel-core delta (generator + docs only, as expected).
- Pre-existing test failures unrelated to this story: `example_spirit_regen_integration::check_mode_fails_on_drift` (deprecated command), `service_boundary_integration` (spirit-ABI hook count fixture drift 14 vs expected 11).

### File List

- `docs/adr/ADR-047-trust-anchor-framing-carry-forward.md` (NEW) — AC-1 trust-anchor framing ADR
- `docs/adr/index.md` (MOD) — ADR-047 registered
- `xtask/src/stability_matrix.rs` (MOD) — NFR-Comp-3 full scope text in render function + 2 new tests (proven-red + scope content)
- `STABILITY.md` (REGEN) — regenerated from generator; manifest_schema_version now 3; compliance scope now full

## Change Log

- 2026-06-15: Story 9.5a implemented — ADR-047 (trust-anchor framing, binding-v0.3) + STABILITY.md NFR-Comp-3 scope via generator + proven-red test. Zero kernel-core delta.
- 2026-06-15: Code review complete (4-layer adversarial — Blind Hunter + Edge Case Hunter + Acceptance Auditor; Test Infra Auditor skipped, dev model claude-opus-4-6). 1 decision-needed (FIPS wording → clarified as seam), 3 patches applied (proven-red test isolation + `tempfile::TempDir` + `spirit_smoke.rs` unstaged), 1 deferred (doc-site link → Story 9.5), 13 dismissed. Generator + STABILITY.md re-verified: `--check` PASS (byte-identical), 4 tests green. Story → done.

### Review Findings

4-layer adversarial code review (2026-06-15): Blind Hunter + Edge Case Hunter + Acceptance Auditor (Test Infra Auditor skipped — dev model claude-opus-4-6). Verification run: `cargo build -p xtask` GREEN, `stability-matrix --check` PASS (byte-identical), `cargo test -p xtask stability_matrix` 4 passed. Both ACs PASS (all sub-clauses verified by the Acceptance Auditor). 1 decision-needed, 3 patches, 1 defer, 13 dismissed.

- [x] [Review][Decision] FedRAMP-row "FIPS-readiness gate (FR48, v1.0)" wording was compliance-ambiguous (FR48 = pluggable crypto-provider seam, not a FIPS-validated module; the default ring/RustCrypto path is not FIPS-validated) [STABILITY.md / xtask/src/stability_matrix.rs:213] — **RESOLVED 2026-06-15 (Lunarpulse, option b):** generator reworded to "Pluggable crypto-provider seam (FR48) — FIPS-validated module is operator/distributor choice"; STABILITY.md regenerated; `--check` PASS. Consistent with ADR-047 §1 "mechanisms, not assertions".
- [x] [Review][Patch] Proven-red test loses mutation-isolation after the v1.0 tag [xtask/src/stability_matrix.rs:407] — **FIXED 2026-06-15:** test now copies the 3 source files into the tempdir first, then renders via `render(&tmp)` (not `render(&root)`), so the one-word mutation is the only delta regardless of the LTS-clock SHA resolving once the `1.0.0` tag lands.
- [x] [Review][Patch] Fixed-name tempdir → `tempfile::TempDir` [xtask/src/stability_matrix.rs:420] — **FIXED 2026-06-15:** switched to `tempfile::TempDir::new()` (already an xtask dep, Cargo.toml:21); unique path + Drop cleanup, no concurrent-test race or panic-leak. Matches the Story 1a.4 precedent.
- [x] [Review][Patch] Out-of-scope cosmetic rustfmt on spirit_smoke.rs [examples/example-spirit/tests/spirit_smoke.rs] — **FIXED 2026-06-15:** unstaged from this story's changeset (`git restore --staged`); the formatting change is preserved in the working tree for a separate chore:fmt commit. [Blind + Acceptance Auditor]
- [x] [Review][Defer] STABILITY.md relative ADR link will need doc-site rewriting [STABILITY.md:57 / xtask/src/stability_matrix.rs:218] — `docs/adr/ADR-047-...` is correct for GitHub root rendering today, but Docusaurus (Story 9.5) serves STABILITY.md differently; 9.5 owns the frozen-URL-contract link handling. Deferred to Story 9.5 (logged in deferred-work.md).
