# Runbook: Korean Accessibility Manual Review

**Frequency:** per-release (before each version tag)
**Owner:** Native/fluent Korean reviewer
**Scope:** Korean locale pages on the MAOS documentation site

---

## Purpose

Automated accessibility testing (axe-core, WCAG AA) catches approximately
one-third of accessibility issues. The remaining issues — particularly
screen-reader comprehension, translation quality, and semantic correctness
of Korean prose — require human evaluation.

This runbook ensures the Korean documentation is usable by Korean speakers
using assistive technology, not merely passing automated gates.

## Pre-requisites

- MAOS doc-site built and served locally (`npm run build && npx serve build`)
- NVDA (Windows) or VoiceOver (macOS) with a Korean TTS voice installed
- Native or fluent Korean language ability
- Browser with developer tools

## Procedure

### 1. Sample Route Selection

Select a representative sample from the route manifest:

- Landing page (`/ko/`)
- One getting-started page (`/ko/write-a-spirit`)
- One reference page (`/ko/manifest/v3`)
- One error page (`/ko/errors/EAbiTooNew`)
- One community page (`/ko/community/governance`)

### 2. Screen-Reader Walkthrough

For each sampled route:

1. Navigate to the Korean version
2. Enable the screen reader (NVDA: Insert+S / VoiceOver: Cmd+F5)
3. Tab through all interactive elements
4. Verify the screen reader announces:
   - Page title in Korean
   - Navigation landmarks correctly
   - Link text meaningfully (not "click here")
   - Code blocks as code
   - Tables with headers

### 3. Translation Quality Check

For each sampled route:

1. Read the Korean prose aloud
2. Check that locked terms (Spirit, kernel, MAOS, etc.) appear verbatim
3. Verify sentences are grammatically correct Korean
4. Flag any machine-translation artifacts (unnatural phrasing, wrong particles)
5. Verify the content conveys the same meaning as the English source

### 4. Visual Check

1. Toggle between light and dark modes
2. Verify Korean text renders correctly (no missing glyphs, no mojibake)
3. Check that the language switcher preserves the current page and anchor
4. Verify mixed-language blocks (Korean prose + English code) are readable

### 5. Record Results

| Item | Status | Notes |
|------|--------|-------|
| Review date | YYYY-MM-DD | |
| Reviewer | Name (native/fluent Korean) | |
| Routes sampled | N of M | |
| Screen reader used | NVDA/VoiceOver + voice | |
| Translation issues found | N | |
| A11y issues found | N | |
| Overall assessment | Pass / Needs Work | |

## Honest Risk: R1

This runbook exists because Korean screen-reader UX and translation
comprehensibility are **CI-untestable** (Risk R1 in Story 9.5). axe-core
over machine Korean scores 100% while the prose may be unusable. This
manual review is the mitigation — it is mandatory, not optional.

A green automated gate + a logged manual review = the AC-2 conformance
claim: "zero automated-detectable WCAG AA violations + logged manual
screen-reader review."
