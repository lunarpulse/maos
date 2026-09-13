# Model-Pin Currency Correction — Course-Correction Input Package

> **Status:** Input package for the `bmad-correct-course` workflow (Step 1 change trigger).
> **Date:** 2026-08-31 · **Author:** Amelia (dev) with party-mode review (Winston, Murat, Victor, Mary, John).
> **Repo:** `maos/` (v0.1-alpha). Kernel-Δ for all proposed work: **ZERO** (nothing touches `crates/maos-kernel-core/src/**`).

---

## 1. Issue Summary (change trigger)

**Problem statement.** Provider model IDs are hardcoded as string literals across 3 distinct layers of MAOS. The Anthropic default (`claude-3-haiku-20240307`) was **retired by Anthropic on 2026-04-20**, which silently broke every live inference call while the error surface reported the *misleading* fallback "Inference transport error — the configured provider is unreachable." Three further retired pins remain live in the tree today.

**How discovered.** 2026-08-31 live incident: operator ran `maos shell` with a valid `MAOS_ANTHROPIC_API_KEY`, received the fallback line. Full RCA (probe → source trace → 3-file fix → live re-test) confirmed root cause; fix for the `hello-spirit` path is already applied and user-verified. This proposal covers the **remaining blast radius and recurrence prevention**.

**Evidence table (receipts).**

| # | Finding | Receipt |
|---|---|---|
| E1 | Anthropic default pinned to retired model in provider construction | `crates/maos-bin/src/main.rs:3062` (fixed 2026-08-31 → `claude-haiku-4-5-20251001`) |
| E2 | Retirement is vendor-documented, replacement named | Anthropic model-deprecations page: `claude-3-haiku-20240307` retired 2026-04-20 → `claude-haiku-4-5-20251001` |
| E3 | All HTTP failures flattened to one opaque fallback line | `crates/maos-kernel-core/src/io/mod.rs:102-108` (`IoError::Transport`) + `crates/maos-spirit-hello/src/lib.rs:101-112` (canned message) |
| E4 | Butler spirit declares retired model | `spirits/butler/manifest.toml:21` (`claude-3-haiku-20240307`) |
| E5 | Researcher spirit declares retired model | `spirits/researcher/manifest.toml:47` (`claude-3-5-sonnet-20241022`, retired 2026-02-19 → `claude-sonnet-4-6`) |
| E6 | Pricing table keyed to retired model | `xtask/provider-pricing.toml:18` (`claude-3-5-sonnet`) |
| E7 | Templates mint new Spirits pre-pinned to a retired model | `templates/spirit-rust/manifest.toml:12`, `templates/spirit-ts/manifest.toml:12`, `examples/example-spirit/manifest.toml:12`, `examples/example-spirit-ts/manifest.toml:12` |
| E8 | Env contract governs URLs but omits model overrides entirely | `crates/maos-bin/src/env_contract.rs` (registry; `MAOS_OLLAMA_URL` at :70 is `UserFacing`; zero `MAOS_*_MODEL` entries) |
| E9 | Kernel enforces provider prefix only — model part of scope string is decorative | `crates/maos-kernel-core/src/inference/mod.rs:242` (`provider == provider_id`) |
| E10 | No mechanical gate exists for model currency (repo gates KLOC, corpora, ABI baseline — not model IDs) | `xtask/` (absent), cf. `xtask/kernel-core-baseline.toml` pattern |
| E11 | OpenAI/Ollama defaults also hardcoded, currency of `gpt-4o-mini` unverified | `main.rs:3072` (`gpt-4o-mini`), `main.rs:3086` (`llama3.1:8b`) |
| E12 | Prior incident record: model-ID change reverted as "unrelated" during a security story | `_bmad-output/implementation-artifacts/14-2a-production-mtls-rotation-trigger.md:1214-1218` |

**Root cause (three layers, distinct verdicts).**

| Layer | Verdict | Note |
|---|---|---|
| Spirit-manifest pins | **Intentional design — keep.** Explicit capability declaration; static, inspectable, attributable (I12-consistent). | Story 2.1 artifact documents v0.1 scope-derivation simplification (full ID declared, provider prefix enforced). |
| Provider defaults in `main.rs` | **Governed-surface gap, not security.** The env contract deliberately governs provider URLs but omits models. | `FIXME(secrets)` at `main.rs:3053` acknowledges deferred config work. |
| Templates/fixtures duplication | **Accidental copy-paste.** | Hermetic test fixtures are inert; templates are not (they generate new Spirits). |

The smell is not duplication per se — it is **absence of a falsifier**: nothing in CI can turn red when a pinned model retires (E9+E10). Security framing for the `main.rs` pins does not hold: the env already carries the API key, so an env override adds no new trust domain. Manifest pins stay static by design and are untouched by the proposed overrides.

---

## 2. Impact Analysis

**Epic impact.**
- **Epic 9 (operator/scheduler surfaces, incl. Butler live dispatch)** — directly exposed: Butler goes live with a dead pin (E4). Highest urgency.
- **Epic 5.5b (multi-provider matrix)** — story artifact itself codifies the hardcoded model (`implementation-artifacts/5-5b-...md:282`); gate will keep this class of artifact honest going forward.
- **Story 9.5 (operator portal, deferred)** — unaffected mechanically; W3's env vars reduce future portal scope.
- **Epic 19 (in flight)** — no direct collision; all proposed changes are outside kernel-core and outside Epic-19 files. Insert as hotfix story, no replan.

**Story/artifact conflicts.** None open: the three-file hello-spirit fix (main.rs model, spirit-hello scope label, hello-spirit manifest) is already applied and live-verified 2026-08-31; remaining edits are additive or in untouched files.

**Technical impact.**
- New xtask gate + one data TOML (W1) — CI wiring follows the existing `check-*` pattern.
- 7 file edits under `spirits/`, `xtask/`, `templates/`, `examples/` (W2).
- `main.rs` provider construction reads 4 new env vars with current literals as defaults; `env_contract.rs` gains `UserFacing` entries (W3). Kernel-Δ ZERO holds for all three workstreams.

**Constraint adopted (Victor, room reservation).** The gate blocks **retired** IDs; it must never enforce "latest." Pinning for reproducibility is the design value; the system only needs death detection.

---

## 3. Recommended Approach

**Classification: Direct Adjustment (Minor scope).** Three workstreams, sequenced W1 → W2 → W3. Total estimate ~1–1.25 dev-days incl. verification.

### W1 — `check-model-currency` xtask gate (prevention, ~0.5d)

- New data file `xtask/model-currency.toml`: per-provider `allowed` list + `retired` list (with `replacement` field for operator guidance). Local data only — **the gate must not require network access** (offline/air-gap CI parity; the repo already ships `--features air-gap` builds).
- New xtask command `check-model-currency` scanning: `spirits/*/manifest.toml`, `templates/*/manifest.toml`, `examples/*/manifest.toml`, `xtask/provider-pricing.toml`, and the model literals in `crates/maos-bin/src/main.rs` provider construction.
- Red on any retired/unknown ID; green on allowed. Re-pin ceremony mirrors `kernel-core-baseline.toml` (explicit edit + disclosure, never silent).
- CI: add to the gate suite where `check-kernel-baseline` runs.

**Acceptance criteria (falsifiable).**
- AC1.1 With a planted retired ID in a copy of `spirits/hello-spirit/manifest.toml`, `cargo run -p xtask -- check-model-currency` exits non-zero naming file+line. (proven-red)
- AC1.2 On the post-W2 tree, the same command exits zero. (green)
- AC1.3 Gate runs with networking disabled (e.g. `unshare -n` smoke) — proves local-data-only.
- AC1.4 `cargo test -p xtask` passes incl. ≥2 unit tests for the scanner (retired-hit, allowed-pass, unknown-id).

### W2 — Retired-pin retrofit (repair, ~0.25d)

Edits, each grounded in vendor replacement guidance (E2):

| File:line | OLD | NEW |
|---|---|---|
| `spirits/butler/manifest.toml:21` | `anthropic.claude-3-haiku-20240307` | `anthropic.claude-haiku-4-5-20251001` |
| `spirits/researcher/manifest.toml:47` | `anthropic.claude-3-5-sonnet-20241022` | `anthropic.claude-sonnet-4-6` |
| `xtask/provider-pricing.toml:18` | `claude-3-5-sonnet` | `claude-sonnet-4-6` (+ verify price rows current) |
| `templates/spirit-rust/manifest.toml:12` | haiku-3 pin | `claude-haiku-4-5-20251001` |
| `templates/spirit-ts/manifest.toml:12` | haiku-3 pin | `claude-haiku-4-5-20251001` |
| `examples/example-spirit/manifest.toml:12` | haiku-3 pin | `claude-haiku-4-5-20251001` |
| `examples/example-spirit-ts/manifest.toml:12` | haiku-3 pin | `claude-haiku-4-5-20251001` |

**Acceptance criteria.**
- AC2.1 `grep -rn 'claude-3-haiku-20240307\|claude-3-5-sonnet-20240219\|claude-3-5-sonnet-20241022' spirits templates examples xtask/provider-pricing.toml` returns zero hits.
- AC2.2 Manifest-parsing tests still pass (`cargo test -p maos-manifest`, plus template smoke if the cargo-generate template test exists — verify, don't assume).
- AC2.3 W1 gate green on the result (ties W2 to W1).
- Explicit non-goal: sweeping hermetic test fixtures/fuzz seeds (`maos-providers` test constants, `maos-manifest` seeds) — inert by construction; changing them risks snapshot churn for zero runtime value. Recorded as accepted debt.

### W3 — Model env overrides + env-contract registration (closure of the governed-surface gap, ~0.5d)

- Read at provider construction (`main.rs:3059-3086`), defaults = current literals:
  - `MAOS_ANTHROPIC_MODEL` (default `claude-haiku-4-5-20251001`)
  - `MAOS_OPENAI_MODEL` (default `gpt-4o-mini` — currency to verify in-story via vendor deprecation page before merge; see Open decision OD3)
  - `MAOS_OLLAMA_MODEL` (default `llama3.1:8b`)
  - `MAOS_DEFAULT_PROVIDER` (default: anthropic when key present, current behavior preserved)
- Register all four in `env_contract.rs` as `UserFacing` with purposes.
- Empty-string handling: treat as unset (match `MAOS_JOURNAL_PATH` empty-guard precedent at `maos-audit/src/lib.rs:1423-1428`) — NOT as tonight's bug class where empty key reached transport.

**Acceptance criteria.**
- AC3.1 `MAOS_ANTHROPIC_MODEL=<alt> ./target/debug/maos shell` live smoke: the alt model appears in `Capability scope:` rendering and the provider call targets it (audit `inference.call` intent row shows the model id).
- AC3.2 Unset vars reproduce byte-identical default behavior (shell smoke diff).
- AC3.3 `env_contract` completeness test (if none exists, add): every `MAOS_*` var read via `std::env::var` in `maos-bin/src/main.rs` has a registry entry — this makes E8's gap class mechanically unrepeatable.
- AC3.4 Kernel-Δ zero: `check-kernel-baseline` green at current pin.

---

## 4. Verification plan (consolidated)

1. Red→green proof for W1 (AC1.1/1.2), run live, output captured.
2. `cargo build -p maos-bin` + `cargo test -p xtask -p maos-manifest`.
3. Live smoke with real operator key: `maos shell` → `@hello-spirit say hi` (path proven 2026-08-31; repeat to confirm no regression from W3).
4. `maos audit query --format plain` shows the new `inference.call` row (model attribution) — ties AC3.1 to the transparency log.
5. All existing discipline gates green (`check-kernel-baseline`, `check-empty-kernel`, workspace-count).

## 5. Risks / tradeoffs

| Risk | Mitigation |
|---|---|
| Gate false-positives blocking CI on vendor churn | `retired` list is hand-curated local data; adding an ID is an explicit, reviewable diff (same ceremony as baseline re-pin) |
| Config sprawl (every machine a different model) | Overrides are opt-in; defaults stay pinned; audit attribution (`provider_attribution.model_id`) keeps runs attributable |
| `gpt-4o-mini` currency unverified (E11) | OD3: verify in-story before W3 merge; W2/W3-Anthropic must not block on it |
| Operator-visible env surface grows → stability expectations | `UserFacing` registration is the mechanism the repo already uses for exactly this contract |
| Pricing table beyond the model key may be stale (rates) | Out of scope; flagged as follow-up data task |

## 6. Implementation handoff

- **Scope classification: Minor** — direct implementation by Developer agent (bmad-dev-story / quick-dev), no backlog reorganization, no PM/Architect replan required.
- **Suggested story split:** one hotfix story per workstream (W1 gate, W2 sweep, W3 env contract) — or a single combined story; either fits current sprint without displacing Epic-19 in-flight work.
- **Success criteria:** all ACs above green with captured receipts; live smoke re-verified; incident class (retired pin + masked error) mechanically impossible to reintroduce silently.

## 7. Open decisions for the course-correction run

- **OD1** — Does `model-currency.toml` carry `replacement` mappings (operator guidance on red) or allowed/retired sets only? (Recommend: carry replacements; display-only, never auto-rewrite.)
- **OD2** — Insert as hotfix story now vs. fold into Epic-9 scheduler story. (Recommend: hotfix now; Butler's live dispatch is the next consumer.)
- **OD3** — `gpt-4o-mini` verification (vendor deprecation check) — in-story task gate for W3's OpenAI default.
- **OD4** — Whether the E3 error-masking fix (surface real HTTP status from `io/mod.rs`) rides along or stays a separate story. (Recommend: separate story — touches error taxonomy across providers; this package stays model-currency-only. The room flagged it earlier as option (b); don't bundle.)

---

### Appendix — already-applied fix (2026-08-31, user-verified live)

`main.rs:3062` model swap, `maos-spirit-hello/src/lib.rs:118-120` scope label, `spirits/hello-spirit/manifest.toml:12` manifest pin → `claude-haiku-4-5-20251001`. Rebuilt, `maos shell` live round-trip with operator key confirmed streaming response + transparency-log rows. These files must NOT be re-edited by W2 (already current).
