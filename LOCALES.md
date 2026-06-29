# Locales & Internationalization Policy

This document is the **single source of truth** for MAOS documentation
localization policy, locked-term registries, known-bad-translation denylists,
and per-locale status. CI glossary-lock gates read their configuration
FROM this file (D5, Story 9.5).

## Supported Locales

| Locale | Status | Notes |
|--------|--------|-------|
| `en` (English) | **Default** — source locale | All content authored in English |
| `ko` (Korean) | **active — full canonical coverage** | Machine-translated with locked-term enforcement. Full canonical coverage delivered by Story 10.3 (all 5 canonical doc deliverables + the generated ABI reference). Story 9.5 shipped the toolchain, the glossary lock, and a representative sample (journey-layer pages) with English fallback (see Page Coverage below) |
| `ja` (Japanese) | **Deferred to v2.0 (Epic 11)** | NOT yet translated. `i18n/ja/` currently holds untranslated WIP scaffold (Korean placeholder content cloned from `ko`), so the coverage gate is report-only at v1.5. Real Japanese translation + 100% gate enforcement is Epic 11 (Story 10.5 AC5 descoped per Epic-10 retro §A2 finding R2) |
| `zh-Hans` (Chinese Simplified) | **Deferred to v2.0 (Epic 11)** | NOT yet translated. `i18n/zh-Hans/` currently holds untranslated WIP scaffold (Korean placeholder content cloned from `ko`), so the coverage gate is report-only at v1.5. Real Chinese translation + 100% gate enforcement is Epic 11 (Story 10.5 AC5 descoped per Epic-10 retro §A2 finding R2) |
| RTL locales (ar, he, etc.) | **Deferred to v2.5** | Requires layout and component changes |

## Locked-Term Registry

These terms MUST appear **verbatim** (exact casing) in all translations.
They are proper nouns, technical identifiers, or domain-specific terms
where localization would break machine-readability or cause confusion.

The CI glossary-lock gate checks these **per translation unit, positionally**:
`count(term, ko_unit) >= count(term, en_unit)` for each document. A term
present in the English source but absent in the Korean translation fails CI.

<!-- BEGIN LOCKED_TERMS -->
Spirit
Worker
kernel
MAOS
Transparency Log
ComplianceClaim
Capability Registry
CapabilityHandle
MailboxHandle
SpiritVtable
SpiritId
HostId
FrameKind
GatewaySubmodule
CancellationSignal
ABI_VERSION
MANIFEST_SCHEMA_VERSION
sandbox tier
trust tier
ADR-001
ADR-002
ADR-004
ADR-006
ADR-010
ADR-011
ADR-012
ADR-014
ADR-022
ADR-023
ADR-026
ADR-028
ADR-030
ADR-032
ADR-037
ADR-038
ADR-039
ADR-040
ADR-041
ADR-045
ADR-046
ADR-047
ADR-048
<!-- END LOCKED_TERMS -->

## Known-Bad-Translation Denylist

These Korean translations MUST NEVER appear in `i18n/ko/` strings.
They are common mistranslations that would confuse Korean readers or
break the domain model.

<!-- BEGIN DENYLIST -->
| English Term | Bad Korean | Why |
|---|---|---|
| Spirit | 정신 (精神) | "Spirit" is a proper noun in MAOS (an agent), not the concept of "mind/spirit" |
| Worker | 노동자 (勞動者) | "Worker" is a Spirit role class, not a "laborer" |
| kernel | 커널 | While 커널 is the standard Korean for "kernel", in MAOS docs it MUST remain as the English "kernel" to match code identifiers and error messages |
<!-- END DENYLIST -->

### Japanese (日本語) denylist

These Japanese translations MUST NEVER appear in `i18n/ja/` strings.

<!-- BEGIN DENYLIST JA -->
| English Term | Bad Japanese | Why |
|---|---|---|
| Spirit | 精霊 / スピリット | "Spirit" is a proper noun in MAOS (an agent), not a supernatural entity |
| Worker | ワーカー | "Worker" is a Spirit role class; must remain English to match code identifiers |
| kernel | カーネル | "kernel" must remain English in MAOS docs to match code and error messages |
<!-- END DENYLIST JA -->

### Chinese Simplified (简体中文) denylist

These Chinese translations MUST NEVER appear in `i18n/zh-Hans/` strings.

<!-- BEGIN DENYLIST ZH-HANS -->
| English Term | Bad Chinese | Why |
|---|---|---|
| Spirit | 精神 / 灵魂 | "Spirit" is a proper noun in MAOS (an agent), not a conceptual term |
| Worker | 工人 / 工作者 | "Worker" is a Spirit role class; must remain English |
| kernel | 内核 | "kernel" must remain English in MAOS docs to match code and error messages |
<!-- END DENYLIST ZH-HANS -->

## Green ≠ Correct Korean

> **Caveat:** A green CI glossary-lock gate proves that a locked term is
> never *absent* from the Korean translation. It does NOT prove the Korean
> prose is *correct*, *natural*, or *comprehensible*. Machine-translated
> Korean that passes CI may still be unusable for a native Korean reader.
>
> The mandatory `runbook:ko-a11y-manual` (see Risk Register in Story 9.5)
> requires a native/fluent Korean reviewer to evaluate translation quality
> per-release. CI is a floor, not a ceiling.

## Page Coverage (per-locale completeness)

Adding a locale (above) is distinct from *fully translating* it. Korean is now
**active — full canonical coverage** for v1.0: Story 9.5 shipped the toolchain,
glossary lock, and journey-layer sample; Story 10.3 completed the canonical
coverage set and turned coverage into a mechanical ship gate.

- **Mechanical gate:** `npm run gate:ko-coverage` (in `docs-site/`) reports
  translated/total per canonical doc deliverable and honors
  `KO_COVERAGE_MIN=100` in CI.
- **Canonical denominator:** all 5 canonical doc deliverables plus the generated
  ABI reference are counted. Generated `/errors/` pages are **excluded** from
  the denominator for v1.0 (resolved in Story 10.3; they remain English until a
  future localization pass).

## Deferral Policy

- **v1.0:** English (source) + Korean (locked-term-enforced, CI-gated; **full canonical
  page coverage at Story 10.3** — see Page Coverage above)
- **v1.5:** _(Japanese + Chinese Simplified were planned here but DESCOPED to v2.0
  per Epic-10 retro §A2 finding R2 — `i18n/ja` & `i18n/zh-Hans` are untranslated WIP
  scaffold, coverage gates report-only; see the status table above)_
- **v2.0 (Epic 11):** Japanese + Chinese Simplified — real machine translation with the
  same locked-term model, **plus a new language-identity gate dimension** (the existing
  file-presence coverage + glossary-lock gates provably cannot detect wrong-language
  content, which is how the Korean-placeholder scaffold passed tautologically)
- **v2.5:** RTL locale support (requires Docusaurus RTL theme + layout audit)
- New locales follow the same pattern: add to `docs-site/docusaurus.config.ts`
  `i18n.locales`, populate `i18n/<locale>/`, extend locked-term checks

## Translation review status

Every Korean translation file (`i18n/ko/**/*.md`) carries a `review_status`
front-matter field that tracks where it sits in the three-layer quality
model (machine translation → glossary-lock CI gate → native/fluent reviewer):

| `review_status` | Meaning |
|---|---|
| `machine` | Machine-translated; the glossary-lock CI gate enforces locked-term correctness, but the unit has NOT yet been reviewed by a native/fluent Korean reviewer. This is the CI floor. |
| `human-reviewed` | Passed an initial native/fluent Korean review (`runbook:ko-a11y-manual`). |
| `approved` | Signed off for the release; no further changes expected. |

The `high_risk: true` front-matter flag marks translation units prioritized
for native review. At v1.0 this is the two high-stakes deployment guides
(`deploy/air-gap-deployment.md` and `deploy/release-signing.md`), where a
translation error could mislead an operator during a safety-critical
procedure. CI does not gate on these flags — they drive reviewer
prioritization, since the CI gate is a floor, not a ceiling.

## CI Integration

The glossary-lock gate (`gate:glossary-lock`) reads this file's
`LOCKED_TERMS` and `DENYLIST` sections, then:

1. For each English doc file and its Korean counterpart:
   - Asserts `count(locked_term, ko_file) >= count(locked_term, en_file)`
   - Fails if any denylist entry appears in the Korean file
2. Checks canonical casing (case-sensitive match for code identifiers)
3. Reports per-file violations with line numbers
4. Asserts canonical Korean coverage (`gate:ko-coverage`, enforced at `KO_COVERAGE_MIN=100` from Story 10.3 / v1.0)

### Source-quality rule (preflight Paige P3)

All configuration key names, CLI flags, and file paths MUST be wrapped in
inline code spans in the English source **before** translation. Bare TOML
keys, flags, and paths in prose get mangled by the machine-translation
engine; wrapping them in inline code spans prevents that. Audit the English
source pages and wrap obvious bare references conservatively — do not reflow
other text.

The gate runs as part of the `docs-site` CI workflow.
