# ⚠️ UNTRANSLATED SCAFFOLD — NOT A REAL CHINESE TRANSLATION

This `i18n/zh-Hans/` tree is **untranslated work-in-progress scaffold**, not a real
Chinese (Simplified) localization. Its pages are **Korean placeholder content cloned from
`i18n/ko/`** — they contain Hangul, not Chinese.

**Status:** real Chinese translation is **DEFERRED to v2.0 (Epic 11)** per the Epic-10
retrospective §A2 re-review finding **R2**. The original Story 10.5 AC5 "zh-Hans active"
claim was a fabricated deliverable (≈37/40 files were byte-identical to `ko`).

**Do NOT:**

- advertise or ship this as a Chinese translation;
- read the `check-zh-coverage` gate's "100% file-presence" as translation coverage — it
  is deliberately **report-only** at v1.5 because these files are placeholders;
- delete this scaffold — it is intentionally retained as the v2.0 starting point per the
  descope decision (2026-06-27).

**v2.0 (Epic 11) will:** replace this with real machine translation under the locked-term
model, add a **language-identity gate** (the coverage + glossary-lock gates provably
cannot detect wrong-language content — that is how this scaffold passed tautologically),
and re-instate `ZHHANS_COVERAGE_MIN=100`.

See `LOCALES.md` and
`_bmad-output/planning-artifacts/epics/epic-11-v20-technical-phase-DRAFT.md`.
