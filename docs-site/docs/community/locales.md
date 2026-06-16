---
title: Locales & i18n
sidebar_position: 3
description: Internationalization policy, glossary lock, and locale status.
---

# Locales & Internationalization

> **Full document:** [`LOCALES.md`](https://github.com/lunarpulse/maos/blob/main/LOCALES.md)

## Supported Locales

| Locale | Status | Notes |
|--------|--------|-------|
| `en` (English) | **Default** | Source locale |
| `ko` (Korean) | **v1.0** | Active, glossary-lock-enforced |
| `ja` (Japanese) | Deferred to v1.5 | Not stubbed |
| `zh-CN` (Chinese Simplified) | Deferred to v1.5 | Not stubbed |
| RTL locales | Deferred to v2.5 | Requires layout changes |

## Glossary Lock

Certain terms (Spirit, Worker, kernel, MAOS, etc.) MUST appear verbatim in
all translations. The CI glossary-lock gate checks per translation unit,
positionally. See `LOCALES.md` for the full locked-term registry.

## Known Limitation

A green CI glossary-lock gate proves a term is never *absent*. It does NOT
prove the Korean prose is *correct* or *natural*. The mandatory
`runbook:ko-a11y-manual` requires native Korean review per-release.
