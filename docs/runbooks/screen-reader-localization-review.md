# Screen-Reader & Localization Manual Review Runbook

**Status:** Active (closes Epic 9 retro carry-forward "Korean SR manual review runbook", elevated to a critical-path item at the Epic 10 retro §A3).
**Scope:** Manual accessibility + language-identity review of localized documentation. Covers `ko` today; extends to `ja` and `zh-Hans` (Story 10.5 AC5).
**Why manual:** The mechanical gates (`gate-ko-coverage.js` / `check-ja-coverage` / `check-zh-coverage`, `gate-glossary-lock.js`) verify **file presence** and **locked-Latin-term preservation** only. They provably **cannot** detect (a) wrong-language content — e.g. Korean shipped under `i18n/ja/` (Story 10.5 R2), or (b) screen-reader UX defects. This runbook is the human complement that catches what the gates structurally can't.

---

## When to run
- Before any release that changes localized content (v1.x ship gate).
- Whenever a new locale is added or a canonical doc is (re)translated.
- Cadence: once per locale per release; re-run the changed-pages subset on any localization PR.

## Inputs
- `LOCALES.md` — supported locales, locked-term registry, per-locale denylists, status table.
- `docs-site/i18n/<locale>/**` — translated content.
- The 5 canonical doc deliverables + the generated ABI reference (the coverage denominator).

## Part A — Language-identity review (the R2 gap)
For each locale, sample ≥5 translated files (include the 5 canonical deliverables):

1. **Native-language confirmation.** A reader of the target language confirms the body is actually in that language — not the source, not another locale's content. Red flag: `i18n/ja` or `i18n/zh-Hans` body text reads as Korean/English.
2. **No byte-identical-to-other-locale bodies.** Spot-check: `diff` the file against the same path under other locales; identical bodies (modulo frontmatter) = untranslated placeholder. (10.5 R2 was 35/40 ja files byte-identical to ko.)
3. **Denylist sanity.** Confirm the locale's `DENYLIST <LOCALE>` terms in `LOCALES.md` are ones that *would actually appear* in that language's machine translation — a denylist of kana/kanji never trips on Korean content, so it cannot backstop a mislabel.
4. **Locked terms in context.** Each locked Latin term (Spirit, kernel, MAOS, Transparency Log, …) appears verbatim and reads naturally inline, not mangled by the translator.

> If any locale fails Part A, the locale is **NOT** release-ready regardless of green coverage/glossary gates. File a finding; do not update `LOCALES.md` status to "Active" until true.

## Part B — Screen-reader review
Tools: NVDA (Windows) or Orca (Linux) or VoiceOver (macOS), latest stable, on the rendered Docusaurus site.

For each sampled page:
1. **Lang attribute.** `<html lang="…">` matches the locale so the SR loads the correct speech engine/pronunciation. Wrong `lang` → garbled TTS even with correct text.
2. **Heading order & landmarks.** Headings are sequential (no skipped levels); main/nav/footer landmarks are announced. (Locale-invariant heading IDs per Story 9.5d AC-5.)
3. **Link text.** Links are meaningful out of context (no bare "여기"/"ここ"/"这里"/"here").
4. **Code blocks & tables.** Code is announced as code; tables have header association; no content trapped in images without alt text.
5. **Navigation.** Version dropdown, sidebar, and locale switcher are keyboard-reachable and announced; focus order is logical.
6. **RTL:** N/A until v2.5 (ar/he deferred per `LOCALES.md`).

## Sign-off
A locale passes when: Part A native-language confirmation ✓ for all sampled canonical deliverables, Part B has zero critical SR defects, and findings (if any) are filed with severity. Record reviewer, locale, date, and sampled file list in the release notes.

**Owner:** Paige (tech writer) + a native-language reviewer per locale. The reviewer is independent of whoever produced the translation.
