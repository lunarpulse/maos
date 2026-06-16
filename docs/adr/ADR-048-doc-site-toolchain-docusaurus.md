# ADR-048: Doc-Site Toolchain — Docusaurus

## Status

**Accepted** — ratified at Story 9.5 preflight party-mode (2026-06-15, Winston·John·Paige·Murat, Lunarpulse approved). binding-v1.0 (NFR-Doc-7). Supersedes no prior ADR (NFR-Doc-7 was previously unresolved).

## Context

NFR-Doc-7 requires a decision on documentation toolchain by v0.5 and production deployment by v1.0. The audience split drives the choice:

- **Kernel authors** prefer in-tree `mdBook` (Rust-native, no Node dependency, familiar to Rustaceans).
- **DPO/CISO/operators** need a searchable, versioned, accessible site they can cite in an audit — with i18n (Korean at v1.0), WCAG AA conformance, version dropdowns, and deep-link stability.

Two finalists were evaluated:

| Criterion | mdBook | Docusaurus |
|---|---|---|
| i18n (Korean + future locales) | Manual / plugin | First-class (`i18n/ko/`) |
| Doc versioning + archive | Not built-in | First-class (version dropdown, ≥2 archived) |
| WCAG AA conformance | Manual theming | React component model + Infima CSS + skip-links |
| Search | mdBook-search | Algolia / local search plugin |
| Citable deep-links | ✅ | ✅ (plus `@docusaurus/plugin-client-redirects` for URL stability) |
| ABI ref from rustdoc JSON | Custom pipeline either way | rustdoc JSON → MDX generator |
| Node dependency | None | Requires Node ≥18 |

## Decision

**Docusaurus** (React/TypeScript, `docs-site/` at repo root).

Paige's argument prevailed: the operator/auditor audience values citability, versioning, i18n, and accessibility — all first-class in Docusaurus — more than zero-Node purity. The rustdoc ABI reference is generated from `rustdoc --output-format json` → MDX converter; ABI pages are never hand-written (they rot vs the code).

### Isolation contract (D2, Winston — MANDATORY)

The doc-site is structurally isolated from the Rust workspace:

1. **Never a Cargo workspace member** — no entry in root `Cargo.toml [workspace.members]`.
2. **KLOC gate carries an explicit, enforced `docs-site` zero-Rust assertion** — `assert_docs_site_zero_rust()` in `xtask/src/kloc_check.rs` walks `docs-site/` and hard-fails on any `.rs` file; the `_docs_site_isolation` key in `xtask/kloc.toml` documents the contract. Enforced by the kloc gate, not relied on by incidence. (Story 9.5 code review correction: the assertion is in the xtask gate logic, not a TOML string value.)
3. **Air-gap structural test runs on a separate CI job** that never invokes the doc toolchain; `npm` is network egress by nature and must never be reachable from the air-gap job.
4. **Kernel-core baseline delta from doc-site stories = 0** — a gate assertion. Docs touch zero kernel crates.
5. **The doc-site CI job (`docs-site.yml`) is its own isolated workflow** — does not touch the Rust workspace, the air-gap test, or any kernel-core/service-boundary/KLOC gate.

### URL contract (D6 — frozen)

The following routes are frozen as deep-link targets:

```
/manifest/<version>/   /cookbook/          /migrate/
/troubleshoot/         /deploy/           /errors/<ERR_NAME>
/abi/<version>/
```

Any reorg that moves these routes must leave a 301 redirect via `@docusaurus/plugin-client-redirects`. 404s on frozen URLs are a regression.

## Consequences

- Node 18+ is required to build docs; CI installs it in the `docs-site` job only.
- The Rust workspace and the doc-site share zero build surface. Cargo never sees `docs-site/`.
- Korean i18n is structurally supported from day one; Japanese + Chinese-simplified are deferred to v1.5 (documented in `LOCALES.md`).
- The rustdoc JSON → MDX pipeline is a build step, not a manual process; stale ABI docs are a build failure.

## Cross-references

- NFR-Doc-7 (toolchain decision by v0.5; production by v1.0)
- Story 9.5 preflight party-mode (2026-06-15)
- `xtask/kloc.toml` `_docs_site_isolation` key
- `.github/workflows/docs-site.yml` (isolated CI job)
