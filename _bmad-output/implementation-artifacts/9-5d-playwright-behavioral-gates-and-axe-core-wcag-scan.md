# Story 9.5d: Playwright Behavioral Gates + axe-core WCAG AA Scan

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the MAOS project shipping a WCAG-AA, Korean-localized doc site,
I want the spec'd **behavioral gates** (deep-link-preserve, fallback, switcher, version-dropdown, links-orphan) implemented via **Playwright against a served build**, AND a **real axe-core WCAG AA scan**,
So that "all gates green" means the site **actually behaves correctly and is actually accessible** — not that config exists or that structural regex checks passed.

## Context & Charter Boundary (READ FIRST)

This story exists because of the **Story 9.5 code review (2026-06-15)**. Two findings:
- **D1** — `@axe-core/cli` + `serve` are **installed devDependencies but `gate:a11y` never invokes them**. The gate runs 3 regex landmark checks on `build/index.html` only. The "zero automated-detectable WCAG AA violations" claim was ungrounded — zero were detected because none were attempted.
- **D2** — **Five Binding Test Gates are entirely unimplemented**; no Playwright capability exists. `gate:links` (orphan-detection), `gate:fallback`, `gate:switcher`, `gate:version-dropdown`, `gate:deep-link-preserve` — all require Playwright against a served build (D7: "config-presence ≠ behavior"). `gate:all` omits them; the 9.5 record's "all gates green" was true only for the implemented subset.

This is the **AC-2 (WCAG) + AC-3 (deep-link) + AC-4 (fallback/switcher/version-dropdown) + AC-1 links-orphan portion that 9.5 deferred**. **9.5 cannot honestly go `done` until this lands.**

- This story adds a **served-build + headless-browser** capability to the docs-site CI.
- `@axe-core/cli` + `serve` are **already installed** (9.5 added them but didn't wire them). This story **wires** them.
- 9.5's `gate:a11y.js` already carries the **honest-scoped docstring** (P-claim patch) and the **coverage hard-fail** — this story **upgrades** it to invoke axe-core and re-scopes the claim to the real result.
- 9.5's `proven-red.js` already has the **try/finally safety scaffold** + 3 base tests + the `provenRed()` helper — add the new behavioral-gate proven-reds following that pattern.

## Preflight Decisions (ratify at story start — or via party-mode)

- **D-pw1 — Served build in CI.** Use `docusaurus serve` (already a dep) on a localhost port in the docs-site workflow; Playwright drives assertions against it. **Confirm this keeps the job isolated** (no Rust workspace touch) per ADR-048 — it should, since the docs-site job is already npm-network-egress by nature (ADR-048 point 3).
- **D-pw2 — axe-core integration.** Use `@axe-core/playwright` (browser-driven, matches the served build) over manifest×{en,ko}; assert zero WCAG AA violations. **Resolve the AC-2 coverage-hard-gate vs AC-4 fallback-is-OK semantics** (D7): count a fallback-served ko route as "scanned" only if the en page was actually scanned, then hard-fail on any shortfall.
- **D-pw3 — Proven-red for every new gate (D8).** Each behavioral gate gets a proven-red companion (mutate the served build/page → assert the gate fails). No tautological-green recurrence.

## Acceptance Criteria

### AC-1 — Playwright capability + served build in CI **[D7]**

**Given** D7 ("config-presence ≠ behavior — verify against the served build")
**When** the story completes
**Then** a served-build step runs in docs-site CI (`docusaurus serve` on localhost) and `@playwright/test` drives assertions against it
**And** the job stays **isolated** (no Rust workspace touch) per ADR-048 — verified, not assumed
**And** a `gate:version-dropdown` dependency on 9-5c's archives is **graceful**: if no ABI versions are archived yet (pre-9-5c or `ABI_VERSION=1`), the gate asserts the dropdown renders at least "latest"; once 9-5c archives versions, it asserts switching between them

### AC-2 — Real axe-core WCAG AA scan **[closes 9.5 D1 / AC-2]**

**Given** `@axe-core/cli` + `serve` are installed but `gate:a11y` never invoked them
**When** the story completes
**Then** `gate:a11y` runs **axe-core** over **manifest×{en,ko}** against the served build; `scanned == expected` is a **hard gate** (D7); any WCAG AA violation fails CI
**And** the conformance claim "zero automated-detectable WCAG AA violations (D9)" is now **grounded** — and 9.5's P-claim honest-scoping (which downscoped the claim to "structural landmarks only") is **replaced** with the real claim
**And** a **proven-red** exists: inject a known WCAG AA violation (e.g., a `<button>` without an accessible name) into the served build → gate fails

### AC-3 — Behavioral gates **[closes 9.5 D2 / AC-1 links · AC-3 deep-link · AC-4 fallback/switcher/version-dropdown]**

**When** the story completes, **all 5** spec'd gates exist and pass against the served build:
- **gate:links** — orphan-detection **seeded from `route-manifest.json`** (a page file not in the manifest fails; a manifest route with no page fails) — *not* crawl-from-root
- **gate:fallback** — an untranslated ko route **renders the en content, not 404**
- **gate:switcher** — the language switcher **preserves the current deep-link path** across en↔ko
- **gate:version-dropdown** — the dropdown switches between ABI versions (graceful per AC-1; depends on 9-5c archives when they exist)
- **gate:deep-link-preserve** — a deep-link anchor fragment targets an element that **exists in the rendered ko DOM**

**And** each gate has a **proven-red companion (D8 / D-pw3)**

### AC-4 — `gate:all` + proven-red green **[integration]**

**Given** the new gates land
**When** the story completes
**Then** `npm run gate:all` exits 0 **including the new behavioral gates**; `gate:all`'s script list is updated to run them
**And** proven-red runs **all** companions (the 3 from 9.5 + the new behavioral-gate ones) and exits 0

## Tasks / Subtasks

- [ ] **T1 — Served-build CI + Playwright setup (AC-1, D-pw1)** — add `@playwright/test`; `docusaurus serve` step in `docs-site.yml`; confirm isolation.
- [ ] **T2 — axe-core wiring (AC-2, D-pw2)** — drive `@axe-core/playwright` over manifest×{en,ko}; hard coverage gate; replace 9.5's P-claim honest-scoping with the real claim; proven-red.
- [ ] **T3 — Behavioral gates (AC-3, D-pw3)** — implement `gate:links`, `gate:fallback`, `gate:switcher`, `gate:version-dropdown`, `gate:deep-link-preserve` + proven-red each.
- [ ] **T4 — `gate:all` integration (AC-4)** — add new gates to `gate:all`; verify full proven-red suite green.
- [ ] **T5 — Isolation + zero kernel-core delta verify.**

## Dev Notes

### Source of truth + guardrails
- **`route-manifest.json`** (`docs-site/route-manifest.json`) is the seed for `gate:links` (orphan-detection) and the coverage count (`routes.length × locales`). `error_codes` there is already cross-checked against the error catalog (landed in 9.5's gate-troubleshoot-bidi patch).
- **9.5's `gate:a11y.js`** carries the P-claim honest docstring + coverage hard-fail — **upgrade, don't rewrite from scratch**. The structural-landmark checks can remain as a fast pre-check; axe-core is the authoritative scan.
- **9.5's `proven-red.js`** has the `provenRed(file, mutate, gateCmd)` helper with try/finally restore. Behavioral gates test a *served* build, so their proven-red mutates the served page/build then restores — extend the helper or add a sibling for served-build mutations.
- **D7/D8/D9 carry forward** from 9.5's preflight: behavior-against-served-build, proven-red-mandatory, honest-claim-scope.
- **Isolation (ADR-048):** Playwright browsers are a CI download. Confirm this does **not** violate the *air-gap* job's network boundary — it must not, because this is the **docs-site job** (already npm-network-egress by nature, ADR-048 point 3). The air-gap job is a separate, never-npm workflow.

### Project Structure Notes
- New: `@playwright/test` devDep; `playwright.config.ts`; served-build CI step; 5 behavioral-gate scripts (or an integrated `gate-behavioral.js`); their proven-reds.
- Modified: `docs-site/scripts/gate-a11y.js` (axe-core wiring + claim re-scope); `docs-site/package.json` `gate:all` script; `.github/workflows/docs-site.yml`.
- Zero kernel-core delta expected.

### References
- [Source: 9-5 review D1 + D2 findings] — `_bmad-output/implementation-artifacts/9-5-...md` § Review Findings (Decision Resolutions D1, D2).
- [Source: deferred-work.md] — D1 + D2 entries under "code review of 9-5-... (2026-06-15)".
- [Source: ADR-048] — isolation contract (docs-site job is its own isolated, npm-egress workflow; air-gap job is separate).
- [Source: docs-site/route-manifest.json] — the seed for `gate:links` + coverage count.
- [Source: 9-5 gate scripts] — `gate-a11y.js` (P-claim scaffold), `proven-red.js` (try/finally `provenRed()` helper) — extend, don't fork conventions.

## Dev Agent Record

### Agent Model Used

<!-- Recommended: claude-opus-4-8. §A6 not applicable (CI/Playwright/JS — no Rust runtime). Ratify at dev-story start. -->

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-06-15: Story 9.5d created — follow-up to close 9.5's deferred AC-2/AC-3/AC-4 gaps (Playwright behavioral gates + axe-core WCAG AA scan). Spawned from 9.5 code review D1 + D2. Status ready-for-dev.
