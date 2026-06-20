---
dev_model_used: openai-codex/gpt-5.4
---

# Story 9.5d: Playwright Behavioral Gates + axe-core WCAG AA Scan

Status: done — **9.5d implementation complete; AC-0 through AC-5 verified.** All review findings patched or dismissed; `npm run gate:all` passes.




<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Preflight party-mode consensus folded 2026-06-16 (Winston·Paige·John·Murat·Amelia·Sally; Lunarpulse approved). See "Preflight Consensus" block below. -->

> **Dependency (ratified — now SATISFIED):** 9.5c **BLOCKS** 9.5d; AC-0 + `gate:links` required 9.5c = `done` (consumes 9.5c's generated file/path layout — the literal left-hand side of the redirect map — and the manifest 9.5c finalizes). **9.5c reached `done` 2026-06-16, so 9.5d is unblocked.** Note: 9.5c shipped WITHOUT the D-cons4 explicit-anchor emission (it was surfaced by this preflight after 9.5c closed); per the **9.5c D-gen7 amendment**, the generator change + anchor lint are carried by **9.5d task T0** rather than reopening 9.5c. The stale sprint-status "parallel-safe" note (line ~131, dated 2026-06-15, predates the 2026-06-16 URL-space fold) is corrected to this.

## Story

As the MAOS project shipping a WCAG-AA, Korean-localized doc site,
I want the spec'd **behavioral gates** (deep-link-preserve, fallback, switcher, version-dropdown, links-orphan) implemented via **Playwright against a served build**, AND a **real axe-core WCAG AA scan**,
So that "all gates green" means the site **actually behaves correctly and is actually accessible** — not that config exists or that structural regex checks passed.

## Context & Charter Boundary (READ FIRST)

This story exists because of the **Story 9.5 code review (2026-06-15)**. Two findings:
- **D1** — `@axe-core/cli` + `serve` are **installed devDependencies but `gate:a11y` never invokes them**. The gate runs 3 regex landmark checks on `build/index.html` only. The "zero automated-detectable WCAG AA violations" claim was ungrounded — zero were detected because none were attempted.
- **D2** — **Five Binding Test Gates are entirely unimplemented**; no Playwright capability exists. `gate:links` (orphan-detection), `gate:fallback`, `gate:switcher`, `gate:version-dropdown`, `gate:deep-link-preserve` — all require Playwright against a served build (D7: "config-presence ≠ behavior"). `gate:all` omits them; the 9.5 record's "all gates green" was true only for the implemented subset.

This is the **AC-2 (WCAG) + AC-3 (deep-link) + AC-4 (fallback/switcher/version-dropdown) + AC-1 links-orphan portion that 9.5 deferred**. **9.5 cannot honestly go `done` until this lands.**

### Absorbed scope — versioned `/abi/<version>/` URL-space (folded here at 9.5c preflight, 2026-06-16)

The 9.5c preflight party-mode (Winston·Paige·Murat·Amelia; Lunarpulse approved) ruled that the versioned `/abi/<version>/` URL-space **config** must ship in the SAME story as its behavioral proof — splitting them risks a live `/abi/v1/` with no proven dropdown/redirect (Murat's decay-risk ruling). So 9.5d now ALSO owns the URL-space config (new AC-0 below): the separate versioned `@docusaurus/plugin-content-docs` instance, `/abi/v1/...` canonical, the redirect map from the flat legacy paths, and the ADR-048 D6 deep-link contract. `gate:version-dropdown` / `gate:deep-link-preserve` then prove it. **Dependency: 9.5c BLOCKS 9.5d** — 9.5c generates the `.md` tree at the current flat path (`docs-site/docs/abi/*.md`); 9.5d restructures it under `/abi/v1/` with redirects. The version-dropdown behaviour is **assert-absence pre-1.0** (D-cons3 below), not "graceful presence."

- This story adds a **served-build + headless-browser** capability to the docs-site CI.
- **TOOLING CORRECTION (D-cons-tool, party-mode 2026-06-16):** the prior "`@axe-core/cli` + `serve` are already installed, just wire them" framing was **wrong**. `gate:a11y.js` today is a **static-HTML regex approximation** (it reads `build/*.html`, never a rendered DOM), so this is an **upgrade to a real rendered-DOM axe scan**, not "wiring an installed tool." Concrete dep change to `docs-site/package.json`: **ADD** `@playwright/test` + `@axe-core/playwright`; **KEEP** `serve`; **REMOVE** `@axe-core/cli` (redundant second browser toolchain once Playwright is in — one engine, fewer flake surfaces).
- 9.5's `gate:a11y.js` carries the **honest-scoped docstring** (P-claim patch) and the **coverage hard-fail** — this story **replaces** the static scan with the real `@axe-core/playwright` rendered-DOM scan and re-scopes the claim to the real result. The structural-landmark checks may remain as a fast pre-filter, but axe is the authoritative scan (single source of a11y truth).
- 9.5's `proven-red.js` has the **try/finally safety scaffold** + base tests + the `provenRed()` helper — it **stays for the static gates**. Behavioral gates get a **new Playwright proven-red project** that mutates at *runtime* (no on-disk build mutation — see D-cons-pr below).

## Preflight Decisions

### Original (D-pw1–3, ratified)

- **D-pw1 — Served build in CI.** Use `docusaurus serve` (a dep) on a localhost port in the docs-site workflow; Playwright drives assertions against it. Job stays isolated (no Rust workspace touch) per ADR-048 — the docs-site job is already npm-network-egress by nature (ADR-048 point 3). **Lifecycle owner = Playwright's built-in `webServer` config** (`playwright.config.ts`: `command: 'npx serve build -l <port>'`, `reuseExistingServer: !process.env.CI`, `timeout: 120_000`) — NOT a hand-rolled wrapper or an npm `&&` chain (a synchronous chain cannot manage a backgrounded server + readiness + teardown). `build` runs *before* `playwright test` (first link of the `&&` chain), never inside `webServer.command`.
- **D-pw2 — axe-core integration.** `@axe-core/playwright`, browser-driven, injected into the *same* Playwright run as the behavioral gates (one served instance, one lifecycle). Coverage semantics resolved by **D-cons2** below.
- **D-pw3 — Proven-red for every new gate (D8).** Mechanism resolved by **D-cons-pr** below (runtime mutation, not on-disk build mutation).

### Preflight Consensus — party-mode 2026-06-16 (RATIFIED — Winston·Paige·John·Murat·Amelia·Sally; Lunarpulse approved; per spec ADR-048 D6 + long-term correctness)

- **D-cons1 — Redirect claim is NOT "301" under `serve` (was: D-α).** `@docusaurus/plugin-client-redirects` emits a **client-side `<meta>`/JS redirect** on a static `serve`, not an HTTP 301. Resolution (5(a)+1(b) merged): (1) **drop the false "301" wording** everywhere → "redirect"; (2) the gate asserts the **redirect TARGET resolves 200**, not an HTTP status code we don't control under `serve`; (3) **real HTTP 301 at the production hosting/CDN layer is a binding documented deploy requirement** (Sally·Winston: a client-side meta-refresh is invisible to `curl`/non-JS/link-checkers/citation tooling — the exact toolchain a DPO/CISO uses, so the *spirit* of D6's "frozen URL never 404s" needs a real server redirect in production even though the gate can only prove resolution under `serve`).
- **D-cons2 — axe coverage denominator = DISTINCT rendered pages (was: D-pw2 semantics / D-2).** `scanned == manifest.routes.length × 2` is a tautology once ko-fallback exists (a fallback ko route serves byte-identical en DOM — counts but scans nothing new). **Denominator = distinct rendered pages** (dedupe ko-fallback into its en target by canonical-URL/content fingerprint), hard-gated. Report `ko_translation_coverage` **separately and ungated** (observational; matches the ko-SR-manual-runbook posture). Honest docstring required.
- **D-cons3 — `gate:version-dropdown` asserts ABSENCE pre-1.0 (was: AC-1 "graceful presence").** A single-version dropdown is a non-affordance (WCAG 4.1.2 smell) and Docusaurus may not render one at all → "asserts dropdown renders" is false-red or tautological-green. **Config-gated assertion:** read `version_count` from config — if `<=1`, assert the **negative invariant** (NO dropdown rendered AND the single-version pages resolve at canonical `/abi/v1/`); if `>=2`, assert the dropdown lists all versions and switching changes served content. Branch is config-driven (deterministic), gains teeth automatically when 9.5c archives a second version. The `>=2` proof needs a throwaway second-version fixture (5-line config) — ship it now, do NOT defer the unproven branch.
- **D-cons4 — heading-anchor stability: mandate locale-invariant explicit IDs + build-failing lint (was: D-β). NEW prereq AC-5.** Docusaurus derives anchor slugs from heading *text*, so ko translation drifts the slug (`#lifecycle-hooks` → `#생명주기-훅`) and silently 404s every cited deep-link — a D6 violation, and `gate:deep-link-preserve` cannot be honestly proven RED without stable IDs (Murat: testability prerequisite). **UNANIMOUS:** mandate explicit locale-invariant heading IDs (`## 생명주기 훅 {#lifecycle-hooks}`) on citable docs + a **build-failing content lint**. This constrains **9.5c's generator** (`xtask/src/gen_abi_docs.rs`): it must emit `## <heading> {#symbol-id}` where the explicit ID is **keyed off the Rust item path (language-invariant)**, never the rendered heading. (9.5c reached `done` 2026-06-16 WITHOUT this; recorded as the 9.5c **D-gen7 amendment** and implemented here in **task T0** rather than reopening 9.5c — do NOT spawn a separate story; John: this is the *spine* of the deep-link contract 9.5d already absorbed, not scope creep.)
- **D-cons5 — fallback page is `lang="en"`, chrome is `lang="ko"`, + visible banner (was: D-γ).** A ko route that falls back to en content serves English bytes; `lang="ko"` over English mis-announces to screen readers (WCAG 3.1.1 page language + 3.1.2 language-of-parts) and axe-core will flag it. **UNANIMOUS:** fallback page `<html lang="en">`; localized chrome (navbar/footer/banner) carries `lang="ko"`; render a **visible localized "shown in English because no Korean translation yet" banner**. `gate:fallback` asserts this (supersedes the old `gate:a11y-ko-lang` blanket `lang="ko"` expectation *for fallback routes only* — genuinely-translated ko pages still require `lang="ko"`).
- **D-cons-pr — proven-red mechanism split (was: D-δ + D-pw3).** Existing on-disk `provenRed()` (mutate file + try/finally restore) **stays for the static gates** (`gate:routes`, `gate:troubleshoot`, `gate:glossary-lock`). Behavioral gates get a **new Playwright proven-red project** that mutates at **runtime** (no disk touch — try/finally does not survive SIGKILL/OOM; a poisoned `build/` feeds every downstream gate):
  - `gate:switcher`, `gate:deep-link-preserve` → `page.evaluate()` DOM mutation (restored free by per-test page isolation).
  - `gate:fallback` → `page.route()` interception to force a real 404.
  - `gate:a11y` (axe) → `page.evaluate()` injects a known WCAG violation (strip `lang`, add alt-less `<img>`) → assert axe reports the expected rule id.
  - **`gate:links` → SYNTHETIC FIXTURES, scoped exception (Murat held; Amelia conceded).** `gate:links` is a *bidirectional* manifest⇄page set-comparison (manifest route with no page fails; page absent from manifest fails). `page.route` can fake one direction (a route that 404s) but cannot synthesize the filesystem page-set for the reverse direction → a one-directional, interception-flaky, half-tautological red. So **parameterize `gate:links` to accept a manifest-path + base-URL** and prove RED by feeding it divergent synthetic manifest+page fixtures (both directions). This is a *narrowly scoped* slice of the deferred D5 testability refactor — `gate:links` only, NOT a blanket gate-parameterization (John dissented on scope; mitigated by the narrow scoping).
  - Carry the tautological-green guard (`runCount==0 ⇒ FAIL`) into the Playwright project, plus a **reached-the-network guard** (≥1 200 against the base URL) so a proof can't pass green because the server never came up.
- **D-cons6 — `/abi/latest` is a live alias; FROZEN flat URLs pin to `/abi/v1/` (was: D-ε).** ADR-048 D6 freezes `/abi/<version>/` as a *permanent citable* contract. **UNANIMOUS:** legacy frozen flat URLs (`/abi/ctx`, …) that external adopters cited → redirect to the **pinned `/abi/v1/...`** (the immutable version), NOT to a moving "current." `/abi/latest` stays a **live convenience alias** to current for browsing. The **canonical pinned `/abi/v1/...` URL must be visible in-page and headless-resolvable at v1.0**; a one-click "cite this version" *button* MAY slip to a follow-up if build cost is real (Sally's concession), the visible+copyable pinned URL may not.
- **D-cons-iso — isolation (Winston, ADR-048).** `npx playwright install --with-deps chromium` (chromium ONLY) runs in the isolated, npm-egress `docs-site.yml` job; pin the Playwright version, cache `~/.cache/ms-playwright`. All Playwright config + specs live **under `docs-site/`** so the paths-filter keeps them in the isolated lane; cannot leak to the air-gap job (separate never-npm workflow). `assert_docs_site_zero_rust()` remains in force.
- **D-cons-ci — CI reliability bar (Murat, non-negotiable).** Deterministic readiness polling (poll URL for 200, bounded timeout — **never `sleep`**); ephemeral/configurable port (no hard-coded 3000); guaranteed teardown via process-group kill in finally + signal trap; `retries: 2` **CI-only** (0 locally); chromium-headless only. **Split `gate:static` (fast pure-Node) vs `gate:behavioral` (served-build)** so the inner dev loop need not pay the browser tax.

## Acceptance Criteria

### AC-0 — Versioned `/abi/<version>/` URL-space **[folded from 9.5c preflight; realizes ADR-048 D6 for /abi/]**

**Given** ADR-048 D6 freezes `/abi/<version>/` as a permanent **citable** deep-link contract, today's `/abi/` routes are flat/version-less, and 9.5c produces a generated `.md` tree at the flat path
**When** the story completes
**Then** a **separate versioned `@docusaurus/plugin-content-docs` instance** scoped to ABI is stood up; `/abi/v1/...` is canonical; the **frozen** flat legacy paths (`/abi/ctx`, …) **redirect to the PINNED `/abi/v1/...`** (the immutable version, NOT a moving "current") via `@docusaurus/plugin-client-redirects` (404 on a frozen URL is a regression) **[D-cons6]**
**And** the redirect is a **client-side redirect under static `serve`** (the term "301" is NOT used — `plugin-client-redirects` emits `<meta>`/JS, not HTTP 301); the gate asserts the **redirect target resolves 200**, not an HTTP status; **real HTTP 301 at the production hosting/CDN layer is a binding documented deploy requirement** so the contract survives `curl`/non-JS/citation tooling **[D-cons1]**
**And** `/abi/latest` is a **live convenience alias** to the current ABI version (browsing only); the canonical **pinned `/abi/v1/...` URL is visible in-page and headless-resolvable** so an auditor can cite the immutable form **[D-cons6]**
**And** `gate:version-dropdown` **asserts the negative invariant pre-1.0**: with `version_count <= 1`, NO version dropdown is rendered AND the single-version pages resolve at canonical `/abi/v1/`; the `>=2` branch (lists all versions, switching changes content) is exercised + proven via a throwaway second-version fixture **[D-cons3]** — no fake archived entries
**And** the job stays **isolated** (no Rust workspace touch) per ADR-048 — it consumes 9.5c's committed `.md`, never invokes the Rust toolchain
**And** the ADR-048 D6 deep-link contract is asserted by AC-3's `gate:deep-link-preserve` / `gate:version-dropdown` (config + its behavioral proof ship together — the reason this scope was folded here)

### AC-1 — Playwright capability + served build in CI **[D7]**

**Given** D7 ("config-presence ≠ behavior — verify against the served build")
**When** the story completes
**Then** a served-build step runs in docs-site CI; **Playwright's `webServer` config** (`npx serve build`) owns the served-build lifecycle (D-pw1) and `@playwright/test` drives assertions against it
**And** the job stays **isolated** (no Rust workspace touch) per ADR-048 — verified, not assumed (chromium-only install, pinned + cached; D-cons-iso)
**And** the CI reliability bar holds (D-cons-ci): deterministic readiness polling (never `sleep`), ephemeral port, guaranteed process-group teardown, `retries:2` CI-only, chromium-headless
**And** the gate suite is **split** into `gate:static` (fast pure-Node) and `gate:behavioral` (served-build) so the inner dev loop need not pay the browser tax (D-cons-ci)
**And** `gate:version-dropdown` is **config-gated on `version_count`, not "graceful presence"** (D-cons3): `<=1` ⇒ assert the negative invariant (no dropdown + pages resolve at canonical `/abi/v1/`); `>=2` ⇒ assert the dropdown lists all versions and switching changes served content (proven now via a throwaway second-version fixture)

### AC-2 — Real axe-core WCAG AA scan **[closes 9.5 D1 / AC-2]**

**Given** `gate:a11y` today is a static-HTML regex approximation that never runs axe-core (the prior "already installed, just wire `@axe-core/cli`" framing was wrong — see D-cons-tool)
**When** the story completes
**Then** `gate:a11y` runs **`@axe-core/playwright`** (rendered-DOM scan, injected into the same Playwright run) over the served build; **`@axe-core/cli` is removed** and `@playwright/test` + `@axe-core/playwright` are added (D-cons-tool); any WCAG AA violation fails CI
**And** the coverage hard-gate denominator is **DISTINCT rendered pages**, not `manifest.routes × 2` (D-cons2): ko-fallback routes are deduped into their en target (a fallback serves byte-identical en DOM — counting it is a tautology); `distinct_pages_scanned == distinct_pages_expected` is the hard gate; **`ko_translation_coverage` is reported separately and is NOT gated** (observational)
**And** the conformance claim "zero automated-detectable WCAG AA violations (D9)" is now **grounded** — 9.5's P-claim structural-only scoping is **replaced** with the real claim; the docstring is honest about the distinct-page denominator + ungated translation coverage
**And** a **proven-red** exists (runtime, D-cons-pr): `page.evaluate()` injects a known WCAG AA violation (strip `lang`, add an alt-less `<img>`) into the live page → axe reports the expected rule id → gate fails

### AC-3 — Behavioral gates **[closes 9.5 D2 / AC-1 links · AC-3 deep-link · AC-4 fallback/switcher/version-dropdown]**

**When** the story completes, **all 5** spec'd gates exist and pass against the served build:
- **gate:links** — orphan-detection **seeded from `route-manifest.json`** (a page file not in the manifest fails; a manifest route with no page fails) — *not* crawl-from-root. **Parameterized** to accept a manifest-path + base-URL so its proven-red feeds synthetic fixtures (D-cons-pr). Also asserts every redirect `to` (from the `redirects` schema, below) resolves 200.
- **gate:fallback** — an untranslated ko route **renders the en content, not 404**, AND the fallback page is **`<html lang="en">`** with `lang="ko"` chrome + a **visible localized "shown in English" banner** (D-cons5, WCAG 3.1.1/3.1.2). Genuinely-translated ko pages still require `lang="ko"`.
- **gate:switcher** — the language switcher **preserves the current deep-link path** across en↔ko. Edge cases the gate MUST cover (Sally): anchor-fragment carry, fallback round-trip (ko→en→ko returns to same URL), versioned `/abi/v1/` pages (preserve both locale prefix and version segment, correct ordering), trailing-slash/query normalization.
- **gate:version-dropdown** — config-gated on `version_count` per **D-cons3** (assert ABSENCE pre-1.0; `>=2` branch proven via fixture) — NOT "graceful presence."
- **gate:deep-link-preserve** — a deep-link anchor fragment targets an element that **exists in the rendered ko DOM**. **Depends on AC-5** (explicit locale-invariant heading IDs) — without stable IDs this gate cannot be honestly proven and silently 404s under translation (D-cons4).

**And** each gate has a **proven-red companion** via the mechanism split in **D-cons-pr** (runtime `page.route`/`page.evaluate` for switcher/fallback/deep-link/axe; synthetic fixtures for `gate:links`)

### AC-4 — `gate:all` + proven-red green **[integration]**

**Given** the new gates land
**When** the story completes
**Then** `npm run gate:all` exits 0 **including the new behavioral gates**; `gate:all`'s script list is updated to run `gate:static` (pure-Node) then `gate:behavioral` (served-build) then proven-red. `build` remains the first link; the served-build lifecycle is owned by Playwright `webServer`, not the `&&` chain (D-pw1)
**And** proven-red runs **all** companions — the existing on-disk ones (static gates) AND the new runtime Playwright project (behavioral gates) — and exits 0; the tautological-green guard (`runCount==0 ⇒ FAIL`) + a reached-the-network guard (≥1 200) both hold (D-cons-pr)

### AC-5 — Locale-invariant explicit heading IDs + build-failing anchor lint **[D-cons4; prerequisite for AC-3 `gate:deep-link-preserve` and ADR-048 D6 under i18n]**

**Given** Docusaurus derives anchor slugs from heading *text*, so a ko-translated heading silently changes the slug (`#lifecycle-hooks` → `#생명주기-훅`) and 404s every cited deep-link — a D6 violation, and `gate:deep-link-preserve` cannot be honestly proven RED without stable IDs
**When** the story completes
**Then** citable docs carry **explicit locale-invariant heading IDs** (`## 생명주기 훅 {#lifecycle-hooks}`) and a **build-failing content lint** rejects any citable heading lacking an explicit `{#id}`
**And** the generator (`xtask/src/gen_abi_docs.rs`) emits `## <heading> {#symbol-id}` where the explicit ID is **keyed off the Rust item path (language-invariant)**, never the rendered heading — recorded as the 9.5c **D-gen7 amendment** and implemented here in T0 (9.5c is `done`), not a separate story
**And** the explicit ID resolves identically in both en and ko rendered DOMs (the property `gate:deep-link-preserve` then asserts)

## Tasks / Subtasks

Slice T1→T7 lands green at each internal step in one PR (Amelia). Hard ordering: T2 (URL-space) before T3/T4 (gates assert against it); AC-5/T0 lands in 9.5c at source; proven-red (T6) last.
- [x] **T0 — explicit-ID emission + anchor lint (AC-5, D-cons4, 9.5c D-gen7 amendment)** — `xtask/src/gen_abi_docs.rs` emits `## <heading> {#symbol-id}` keyed off the Rust item path; build-failing lint for any citable heading missing `{#id}`; regenerate the committed `docs-site/abi/v1/*.md`. Blocks 9.5d's `gate:deep-link-preserve`. (Was slated for 9.5c; 9.5c is now `done`, so carried here per the D-gen7 amendment.)
- [x] **T1 — Deps + Playwright setup (AC-1, D-pw1, D-cons-tool, D-cons-iso)** — `package.json`: ADD `@playwright/test` + `@axe-core/playwright`, REMOVE `@axe-core/cli`, KEEP `serve`. Add `playwright.config.ts` (built-in `webServer`, projects: behavioral/a11y/proven-red, chromium-only, `retries:2` CI-only). `npx playwright install --with-deps chromium` + cache in `docs-site.yml`. Confirm isolation.
- [x] **T2 — AC-0 URL-space FIRST (AC-0, D-cons1, D-cons6)** — second `@docusaurus/plugin-content-docs` instance (`id:'abi'`, `routeBasePath:'abi/v1'`); `sidebars-abi.ts`; point 9.5c's generator output dir under the versioned segment; `plugin-client-redirects` map (frozen flat `/abi/*` → pinned `/abi/v1/*`; `/abi/latest` → current); `route-manifest.json` gains a **`redirects: [{from,to}]` schema** + all `/abi/v1/*` routes (else `gate:links`/`gate:deep-link-preserve` are tautological-green). Drop "301" wording → "redirect"; document real HTTP 301 as a hosting/CDN deploy requirement.
- [x] **T3 — axe upgrade (AC-2, D-cons2)** — `@axe-core/playwright` rendered-DOM scan; distinct-rendered-pages denominator (dedupe ko-fallback); separate ungated `ko_translation_coverage`; honest docstring; retire the static scan path.
- [x] **T4 — Behavioral gates (AC-3, D-cons3, D-cons5)** — `gate:links` (parameterized manifest-path+base-URL; asserts redirect targets resolve), `gate:fallback` (en content + `lang="en"` + `lang="ko"` chrome + banner), `gate:switcher` (4 edge cases), `gate:version-dropdown` (config-gated absence + fixture-proven `>=2`), `gate:deep-link-preserve` (explicit-ID, depends on T0).
- [x] **T5 — `gate:all` integration (AC-4)** — split `gate:static` / `gate:behavioral`; wire into `gate:all`; update `docs-site.yml`.
- [x] **T6 — proven-red (D-cons-pr)** — keep on-disk `proven-red.js` for static gates; NEW Playwright proven-red project: runtime `page.route`/`page.evaluate` for switcher/fallback/deep-link/axe; synthetic fixtures for `gate:links`; tautological-green + reached-network guards.
- [x] **T7 — Isolation + zero kernel-core delta verify** (D-cons-iso) — all Playwright config/specs under `docs-site/`; `assert_docs_site_zero_rust()` green; chromium-only, no air-gap leak.

### Review Findings

Code review of the behavioral-gate + URL-space/content diff (groups 1–2). Review mode: full.

**Summary:** 0 `decision-needed`, 16 `patch` (applied), 0 `defer`, 4 `dismissed`.

#### decision-needed

None.

#### patch

- [x] [Review][Patch] `gate:version-dropdown` ≥2 branch is tautological — added throwaway second-version fixture served by a local HTTP server [version-dropdown.spec.ts] — satisfies D-cons3, AC-0, AC-1.
- [x] [Review][Patch] `gate:links` is one-directional — added bidirectional manifest⇄build-page set comparison [links.spec.ts, manifest.ts] — satisfies AC-3.
- [x] [Review][Patch] axe proven-red injects violations but never asserts axe reports the expected rule id — now asserts `html-has-lang` and `image-alt` rule ids [behavioral.proven-red.ts] — satisfies D-cons-pr, AC-2.
- [x] [Review][Patch] axe coverage hard-gate is tautological — `expected_distinct_pages` now lives in `route-manifest.json` and is asserted with a ±1 tolerance against the actual scan [a11y.a11y.ts, route-manifest.json] — satisfies AC-2, D-cons2.
- [x] [Review][Patch] Generated ABI pages carry stuttered heading IDs (`…-related-related`) — fixed `gen_abi_docs.rs` prefix and regenerated pages — satisfies AC-5.
- [x] [Review][Patch] Empty `### Inherent Items` sections immediately followed by item headings in generated pages — added descriptive sentence in generator and regenerated pages [gen_abi_docs.rs] — satisfies AC-5.
- [x] [Review][Patch] architecture §7.1 link points to GitHub `docs/` tree listing instead of the section — updated `_related_identity.md` to the exact architecture doc section anchor — satisfies AC-5.
- [x] [Review][Patch] `postbuild-fallback-lang.js` uses race-prone 10s polling and fragile HTML string matching — replaced with path-based detection and a `MutationObserver` guard script [postbuild-fallback-lang.js] — satisfies D-cons5.
- [x] [Review][Patch] `a11y.a11y.ts` deduplicates pages by `<main>` inner text and aborts on missing `<main>` — missing `<main>` is now collected and reported; distinct-page baseline is external — satisfies AC-2.
- [x] [Review][Patch] Behavioral proven-red only asserts the mutated red state, never the unmutated green baseline — added green-baseline assertions before each mutation [behavioral.proven-red.ts] — satisfies D-cons-pr.
- [x] [Review][Patch] `switcher.spec.ts` `expect(...).first().toHaveCount(1)` is tautological — replaced with exact-count assertion [switcher.spec.ts] — satisfies AC-3.
- [x] [Review][Patch] `links.spec.ts` redirect assertion is fragile on trailing-slash mismatch and JS-disabled clients — normalized trailing slashes in redirect assertion [links.spec.ts] — satisfies AC-3.
- [x] [Review][Patch] Prism token CSS forces monochrome `#1d3c66 !important`, breaking dark-mode readability — scoped the override to light mode only and added a safe catch-all fallback [custom.css] — satisfies AC-2.
- [x] [Review][Patch] `gate:anchor-ids.js` lint mishandles `~~~` fences, nested fences, and has a 2-character minimum ID bug — implemented fence-length aware parser and fixed id regex [gate-anchor-ids.js] — satisfies AC-5.
- [x] [Review][Patch] `playwright.config.ts` lacks validation for `DOCS_SITE_PORT` and `CI` env vars — added `requirePort()` and robust `isCI()` helpers [playwright.config.ts] — satisfies D-cons-ci.
- [x] [Review][Patch] `gate-a11y.js` spawns `npx` without `shell:true` on Unix and duplicates the `gate:a11y` npm script — added `shell: true` and pointed `gate:a11y` npm script at the wrapper [gate-a11y.js, package.json] — satisfies AC-2.

#### defer

None.

#### dismissed

- `/abi/latest` alias is no longer linked from docs content — intentional; the pinned `/abi/v1/` canonical URL is the required visible surface (auditor INFO).
- Docusaurus routeBasePath `/abi/v1` vs client-redirect `/abi` collision concern — no evidence of actual collision; Docusaurus plugin routing handles this (edge-case speculation).
- Dead `_related_*.md` partial files are created but never included — **false**: the generator reads and includes each `_related_<module>.md` partial in its parent page; they are not dead.
- Error-code manifest paths with `::` produce unencoded URL segments — **false**: Docusaurus routes and the static `serve` host these pages at the literal `::` path; encoding/sanitizing to `-` breaks the gate. Accepted as the project's URL convention.


### Review Findings — Groups 3–5

Code review of the remaining diff groups: CI/isolation (group 3), generator/xtask (group 4), and meta files (group 5). Review mode: full.

**Summary:** 0 `decision-needed`, 9 `patch` (applied), 4 `defer`, 6 `dismissed`.


#### decision-needed

None.

#### patch

- [x] [Review][Patch] `docs-site/.gitignore` missing trailing newline [docs-site/.gitignore:26] — added trailing newline.
- [x] [Review][Patch] Story spec T0 still references stale generator output path `docs-site/docs/abi/*.md` [9-5d-playwright-behavioral-gates-and-axe-core-wcag-scan.md:121] — updated to `docs-site/abi/v1/*.md`.
- [x] [Review][Patch] Generator `add_explicit_ids_to_markdown_headings` only handles triple-backtick fences; `~~~` and nested/paired fences desync the parser [xtask/src/gen_abi_docs.rs:580] — replaced naive boolean fence flag with `FenceTracker` that matches fence character and length; parity with `gate-anchor-ids.js`.
- [x] [Review][Patch] Generator test `generated_abi_pages_have_explicit_heading_ids` shares the same fence-detection blind spot [xtask/src/gen_abi_docs.rs:1348] — updated test to use `FenceTracker`; added a new `generated_abi_pages_have_unique_heading_ids` test.
- [x] [Review][Patch] CI `gate:behavioral` step has no explicit `timeout-minutes` [docs-site.yml:63] — added `timeout-minutes: 10`.
- [x] [Review][Patch] Playwright cache step lacks an `id` and `restore-keys` fallback [docs-site.yml:41-45] — added `id: playwright-cache` and `restore-keys` prefix fallback.
- [x] [Review][Patch] `npx playwright install --with-deps chromium` runs unconditionally after a cache hit [docs-site.yml:51-53] — added `if: steps.playwright-cache.outputs.cache-hit != 'true'`.
- [x] [Review][Patch] `anchor_id_from_text` silently drops punctuation (`&`, `#`, `.`, `+`, `=`, `%`, `?`) and has no duplicate-ID guard [xtask/src/gen_abi_docs.rs:43-73] — all ASCII punctuation now maps to `-`; `assemble_page_text` post-processes explicit IDs to deduplicate collisions.
- [x] [Review][Patch] Behavioral proven-red axe `html-has-lang` assertion was flaky under Docusaurus hydration re-adding `lang` — made deterministic by intercepting the home-page response to strip `lang` and inject an alt-less image, then waiting for hydration before running axe [behavioral.proven-red.ts] — satisfies D-cons-pr, AC-2.

#### defer

- [x] [Review][Defer] `anchor_id_from_text` emoji-only fallback `"section"` could collide in `_related_*.md` partials [xtask/src/gen_abi_docs.rs:68-70] — theoretical; current partials are ASCII-curated.
- [x] [Review][Defer] `item_anchor_id` falls back to literal `"item"` for unnamed items [xtask/src/gen_abi_docs.rs:79-83] — theoretical; rendered items carry names.
- [x] [Review][Defer] `module_anchor_id` root fallback collides with the hardcoded `render_constants_page` ID [xtask/src/gen_abi_docs.rs:85-92] — code path not currently reachable.
- [x] [Review][Defer] `cargo run -p xtask -- kloc-check` FAIL recorded as pre-existing workspace KLOC overrun outside `docs-site/` [9-5d-playwright-behavioral-gates-and-axe-core-wcag-scan.md § Dev Notes] — not introduced by this story.

#### dismissed

- CI step comments stripped and stray double-blank line — cosmetic; npm script names are self-documenting.
- Review findings embedded in the story spec — required by `bmad-code-review` step-04; audit trail belongs with the story artifact.
- Sprint-status `last_updated` midnight timestamp is a placeholder — tag-based versioning (`+story-9-5d-review`) is the project convention here.
- Dev model listed as `openai-codex/gpt-5.4` versus earlier `claude-opus-4-8` recommendation — metadata record of the actual agent used.
- Playwright cache key does not include the Playwright version independently — `package-lock.json` pins Playwright; lockfile hash is the standard key.
- CI not triggered by standalone `xtask/src/gen_abi_docs.rs` changes — committed generated pages live under `docs-site/abi/v1`, so output changes already trigger; generator consistency is covered by xtask tests in the Rust CI lane.

## Dev Notes

### Source of truth + guardrails
- **`route-manifest.json`** (`docs-site/route-manifest.json`) is the seed for `gate:links` and the coverage count. **It gains a `redirects: [{from,to}]` schema** (D-cons1/T2) + all `/abi/v1/*` routes — without it `gate:links`/`gate:deep-link-preserve` have nothing to assert against and go tautological-green. The axe coverage denominator is **distinct rendered pages**, NOT `routes.length × locales` (D-cons2). `error_codes` already cross-checked against the error catalog (9.5's gate-troubleshoot-bidi patch).
- **9.5's `gate:a11y.js`** is a static-HTML approximation — **replace** the scan with `@axe-core/playwright` (rendered DOM). Structural-landmark checks may stay as a fast pre-filter but axe is the single source of a11y truth.
- **`proven-red.js`** (on-disk `provenRed()` + try/finally) **stays for static gates**. Behavioral gates use a **new Playwright proven-red project mutating at runtime** (D-cons-pr) — try/finally does NOT survive SIGKILL, so a poisoned `build/` would feed every downstream gate; runtime mutation (per-test page isolation) avoids disk entirely. `gate:links` is the exception: synthetic manifest+page fixtures (bidirectional set-comparison can't be honestly proven by one-directional `page.route`).
- **D7/D8/D9 carry forward**: behavior-against-served-build, proven-red-mandatory, honest-claim-scope. Plus the CI reliability bar (D-cons-ci): deterministic readiness polling (never `sleep`), ephemeral port, process-group teardown, CI-only retries.
- **Isolation (ADR-048, D-cons-iso):** `npx playwright install --with-deps chromium` (chromium only), pinned + cached `~/.cache/ms-playwright`, in the isolated npm-egress `docs-site.yml` job only. All Playwright config/specs under `docs-site/` (paths-filter keeps them in the isolated lane). Cannot leak to the air-gap job (separate never-npm workflow); `assert_docs_site_zero_rust()` stays in force.
- **Dependency (ratified — SATISFIED 2026-06-16):** 9.5c **BLOCKED** 9.5d (AC-0 + `gate:links` consume 9.5c's path layout); 9.5c is now `done`, so 9.5d is unblocked. The AC-5 explicit-ID generator change (9.5c D-gen7 amendment) is carried by T0 here, since 9.5c shipped without it.

### Project Structure Notes
- New: `@playwright/test` + `@axe-core/playwright` devDeps; `playwright.config.ts` (webServer + projects); `sidebars-abi.ts`; behavioral-gate specs + the runtime proven-red project; `route-manifest.json` `redirects` schema.
- Modified: `docs-site/scripts/gate-a11y.js` (→ `@axe-core/playwright`, distinct-page denominator, claim re-scope); `docs-site/package.json` (deps: −`@axe-core/cli`; `gate:static`/`gate:behavioral`/`gate:all` scripts); `docusaurus.config.ts` (second docs instance + client-redirects map); `.github/workflows/docs-site.yml` (chromium install + cache + new gate steps).
- In 9.5c (at source, AC-5/T0): `xtask/src/gen_abi_docs.rs` (explicit item-path `{#id}` emission + anchor lint; generator output under versioned segment).
- Zero kernel-core delta expected (doc-comments/CI/JS only; §A6 N/A).

### References
- [Source: 9-5 review D1 + D2 findings] — `_bmad-output/implementation-artifacts/9-5-...md` § Review Findings (Decision Resolutions D1, D2).
- [Source: deferred-work.md] — D1 + D2 entries under "code review of 9-5-... (2026-06-15)".
- [Source: ADR-048] — isolation contract (docs-site job is its own isolated, npm-egress workflow; air-gap job is separate).
- [Source: docs-site/route-manifest.json] — the seed for `gate:links` + coverage count.
- [Source: 9-5 gate scripts] — `gate-a11y.js` (P-claim scaffold), `proven-red.js` (try/finally `provenRed()` helper) — extend, don't fork conventions.

## Dev Agent Record

### Agent Model Used

- openai-codex/gpt-5.4

### Debug Log References

- `npm run gate:all` — PASS
- `npm run typecheck` — PASS
- `cargo test -p xtask gen_abi_docs` — PASS
- `cargo run -p xtask -- gen-abi-docs --check` — PASS
- `cargo run -p xtask -- kloc-check` — FAIL, pre-existing workspace KLOC budget overruns outside `docs-site/` remain red; docs-site zero-Rust isolation itself stayed intact

### Completion Notes List

- Added real Playwright-based behavioral gates for links, fallback behavior, language switcher deep-link preservation, version-dropdown absence pre-1.0, and locale-invariant deep-link anchors.
- Replaced the structural-only a11y gate with a rendered-DOM `@axe-core/playwright` scan over distinct rendered pages; `ko_translation_coverage` now reports separately and ungated.
- Folded `/abi/v1/` into the live docs configuration: separate ABI docs plugin, pinned redirects from frozen flat `/abi/*`, live `/abi/latest` redirect, and route-manifest redirect metadata.
- Moved generated ABI reference pages under `docs-site/abi/v1`, updated the generator default output path, and added explicit `{#id}` emission plus a build-failing anchor-ID lint.
- Patched Korean ABI fallback pages post-build so untranslated ABI routes present English page language with a visible Korean banner while preserving Korean chrome.
- Added runtime Playwright proven-red coverage and kept the existing on-disk static proven-red checks for the legacy Node gates.

### File List

- Modified: `.github/workflows/docs-site.yml`
- Modified: `docs-site/package.json`
- Modified: `docs-site/package-lock.json`
- Modified: `docs-site/docusaurus.config.ts`
- Modified: `docs-site/route-manifest.json`
- Modified: `docs-site/sidebars.ts`
- Added: `docs-site/sidebars-abi.ts`
- Added: `docs-site/playwright.config.ts`
- Modified: `docs-site/src/css/custom.css`
- Modified: `docs-site/scripts/gate-a11y.js`
- Modified: `docs-site/scripts/proven-red.js`
- Added: `docs-site/scripts/gate-anchor-ids.js`
- Added: `docs-site/scripts/postbuild-fallback-lang.js`
- Modified: `docs-site/docs/index.md`
- Modified: `docs-site/docs/understand-maos.md`
- Modified: `docs-site/docs/write-a-spirit.md`
- Modified: `docs-site/docs/migrate/abi-stability.md`
- Modified: `docs-site/docs/migrate/v1-to-v2.md`
- Modified: `docs-site/docs/migrate/v2-to-v3.md`
- Modified: `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/index.md`
- Modified: `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/understand-maos.md`
- Modified: `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/write-a-spirit.md`
- Added: `docs-site/tests/playwright/manifest.ts`
- Added: `docs-site/tests/playwright/links.spec.ts`
- Added: `docs-site/tests/playwright/fallback.spec.ts`
- Added: `docs-site/tests/playwright/switcher.spec.ts`
- Added: `docs-site/tests/playwright/version-dropdown.spec.ts`
- Added: `docs-site/tests/playwright/deep-link-preserve.spec.ts`
- Added: `docs-site/tests/playwright/a11y.a11y.ts`
- Added: `docs-site/tests/playwright/behavioral.proven-red.ts`
- Added: `docs-site/abi/v1/index.md`
- Added: `docs-site/abi/v1/constants.md`
- Added: `docs-site/abi/v1/lifecycle.md`
- Added: `docs-site/abi/v1/ctx.md`
- Added: `docs-site/abi/v1/compliance.md`
- Added: `docs-site/abi/v1/identity.md`
- Added: `docs-site/abi/v1/cancellation.md`
- Added: `docs-site/abi/v1/gateway.md`
- Added: `docs-site/abi/v1/deprecation.md`
- Added: `docs-site/abi/v1/_related_index.md`
- Added: `docs-site/abi/v1/_related_constants.md`
- Added: `docs-site/abi/v1/_related_lifecycle.md`
- Added: `docs-site/abi/v1/_related_ctx.md`
- Added: `docs-site/abi/v1/_related_compliance.md`
- Added: `docs-site/abi/v1/_related_identity.md`
- Added: `docs-site/abi/v1/_related_cancellation.md`
- Added: `docs-site/abi/v1/_related_gateway.md`
- Added: `docs-site/abi/v1/_related_deprecation.md`
- Deleted: `docs-site/docs/abi/index.md`
- Deleted: `docs-site/docs/abi/constants.md`
- Deleted: `docs-site/docs/abi/lifecycle.md`
- Deleted: `docs-site/docs/abi/ctx.md`
- Deleted: `docs-site/docs/abi/compliance.md`
- Deleted: `docs-site/docs/abi/identity.md`
- Deleted: `docs-site/docs/abi/cancellation.md`
- Deleted: `docs-site/docs/abi/gateway.md`
- Deleted: `docs-site/docs/abi/deprecation.md`
- Deleted: `docs-site/docs/abi/_related_index.md`
- Deleted: `docs-site/docs/abi/_related_constants.md`
- Deleted: `docs-site/docs/abi/_related_lifecycle.md`
- Deleted: `docs-site/docs/abi/_related_ctx.md`
- Deleted: `docs-site/docs/abi/_related_compliance.md`
- Deleted: `docs-site/docs/abi/_related_identity.md`
- Deleted: `docs-site/docs/abi/_related_cancellation.md`
- Deleted: `docs-site/docs/abi/_related_gateway.md`
- Deleted: `docs-site/docs/abi/_related_deprecation.md`
- Modified: `xtask/src/gen_abi_docs.rs`
- Modified: `xtask/src/main.rs`
## Change Log

- 2026-06-15: Story 9.5d created — follow-up to close 9.5's deferred AC-2/AC-3/AC-4 gaps (Playwright behavioral gates + axe-core WCAG AA scan). Spawned from 9.5 code review D1 + D2. Status ready-for-dev.
- 2026-06-16: **Preflight party-mode consensus folded** (Winston·Paige·John·Murat·Amelia·Sally; Lunarpulse approved; decided per spec ADR-048 D6 + long-term correctness). 9 ratified decisions: **D-cons1** "301" is false under `serve` → "redirect" + gate asserts target resolves + real HTTP 301 a documented hosting deploy requirement; **D-cons2** axe denominator = distinct rendered pages (dedupe ko-fallback), `ko_translation_coverage` reported separately/ungated; **D-cons3** `gate:version-dropdown` asserts ABSENCE pre-1.0 (config-gated on `version_count`, `>=2` proven via fixture); **D-cons4** NEW **AC-5** — mandate locale-invariant explicit heading IDs + build-failing lint, constraining 9.5c's `gen_abi_docs.rs` (item-path-keyed `{#id}`); **D-cons5** fallback page `lang="en"` + `lang="ko"` chrome + visible banner (WCAG 3.1.1/3.1.2); **D-cons6** frozen flat URLs pin to `/abi/v1/`, `/abi/latest` = live alias, pinned URL visible+headless-resolvable; **D-cons-pr** proven-red split (on-disk for static gates, runtime `page.route`/`page.evaluate` for behavioral, synthetic fixtures for `gate:links`); **D-cons-tool** drop `@axe-core/cli`, add `@playwright/test` + `@axe-core/playwright`, `gate:a11y` is an UPGRADE not a wiring; **D-cons-iso/-ci** chromium-only isolated install + reliability bar (readiness polling, ephemeral port, teardown, CI-only retries, `gate:static`/`gate:behavioral` split). Dependency corrected: **9.5c BLOCKS 9.5d to `done`** (AC-0/`gate:links` gated); stale sprint-status "parallel-safe" note superseded. Tasks re-sliced T0→T7.
- 2026-06-16: 9.5c reached `done`; **AC-0 dependency satisfied, 9.5d unblocked.** 9.5c shipped without the D-cons4 explicit-anchor emission (surfaced post-close); recorded as 9.5c **D-gen7 amendment** and carried by T0 here rather than reopening 9.5c. Status ready-for-dev.
- 2026-06-16: Implemented 9.5d end-to-end. Added `/abi/v1` docs routing + redirects, explicit anchor IDs, Playwright behavioral gates, rendered-DOM axe scan, runtime proven-red coverage, and CI/browser isolation wiring. Verified with `npm run gate:all`, `npm run typecheck`, `cargo test -p xtask gen_abi_docs`, and `cargo run -p xtask -- gen-abi-docs --check`.
