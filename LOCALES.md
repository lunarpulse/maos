# Locales & Internationalization Policy

This document is the **single source of truth** for MAOS documentation
localization policy, locked-term registries, known-bad-translation denylists,
and per-locale status. CI glossary-lock gates read their configuration
FROM this file (D5, Story 9.5).

## Supported Locales

| Locale | Status | Notes |
|--------|--------|-------|
| `en` (English) | **Default** — source locale | All content authored in English |
| `ko` (Korean) | **active — partial coverage** | Machine-translated with locked-term enforcement. Story 9.5 shipped a representative sample (journey-layer pages) + English fallback; **full canonical coverage is owned by Story 10.3** (see Page Coverage below) |
| `ja` (Japanese) | **Deferred to v1.5** | Not stubbed; explicit deferral |
| `zh-CN` (Chinese Simplified) | **Deferred to v1.5** | Not stubbed; explicit deferral |
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

Adding a locale (above) is distinct from *fully translating* it. Korean is **active
but partial**: Story 9.5 deliberately shipped the toolchain, the glossary lock, and a
representative sample (the journey-layer pages), with **English fallback** for the rest
(`gate:fallback`, AC-4). The glossary-lock gate enforces locked terms only on pages that
*are* translated — it does **not** require full coverage — so coverage is a permitted
floor today, not the target.

- **Visibility now:** `npm run gate:ko-coverage` (in `docs-site/`) reports
  translated/total per canonical doc deliverable. Report-only by default.
- **Teeth at v1.0:** **Story 10.3** (Epic 10 v1.0 ship gate, NFR-Doc-6) requires Korean
  present for **all 5 canonical doc deliverables**. That story wires
  `KO_COVERAGE_MIN=100` into the `docs-site` CI to turn the promise into a mechanical
  gate (a promise without a gate decays — Epic 8 lesson). Open scoping question for 10.3:
  whether the 37 generated `/errors/` pages count toward the troubleshoot deliverable.

## Deferral Policy

- **v1.0:** English (source) + Korean (locked-term-enforced, CI-gated; **full canonical
  page coverage at Story 10.3** — see Page Coverage above)
- **v1.5:** Japanese + Chinese Simplified (same locked-term model)
- **v2.5:** RTL locale support (requires Docusaurus RTL theme + layout audit)
- New locales follow the same pattern: add to `docs-site/docusaurus.config.ts`
  `i18n.locales`, populate `i18n/<locale>/`, extend locked-term checks

## CI Integration

The glossary-lock gate (`gate:glossary-lock`) reads this file's
`LOCKED_TERMS` and `DENYLIST` sections, then:

1. For each English doc file and its Korean counterpart:
   - Asserts `count(locked_term, ko_file) >= count(locked_term, en_file)`
   - Fails if any denylist entry appears in the Korean file
2. Checks canonical casing (case-sensitive match for code identifiers)
3. Reports per-file violations with line numbers

The gate runs as part of the `docs-site` CI workflow.
