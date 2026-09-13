# Sprint Change Proposal — Model-Pin Currency Hotfix Lane

> **Date:** 2026-09-01 · **Workflow:** bmad-correct-course (Batch mode) · **Approved trigger:** `model-pin-currency-correction-input-2026-08-31.md`
> **Operator decisions (this run):** Batch review · OD2 = standalone hotfix lane (J1-lane precedent) · OD4 = error-masking filed as its own story.
> **APPROVED by Lunarpulse 2026-09-01** — CP-1 and CP-2 applied same day; proposal finalized.
> **Change scope classification: Minor** — direct Developer-agent implementation, no backlog reorganization, no PM/Architect replan.

---

## Section 1: Issue Summary

**Problem.** Provider model IDs are hardcoded string literals at three layers of MAOS (provider construction, Spirit capability-scope manifests, templates/fixtures). The Anthropic default `claude-3-haiku-20240307` was **retired by the vendor on 2026-04-20**; every live inference call silently failed while the operator surface rendered the misleading fallback "Inference transport error — the configured provider is unreachable."

**Discovery.** Live incident 2026-08-31: operator ran `maos shell` with a valid `MAOS_ANTHROPIC_API_KEY`. RCA chain: transport probe (fake-key → `HTTP 401 in 0.27s`, network/DNS/TLS healthy) → source trace (`main.rs` model pin; `io/mod.rs:102-108` flattens all HTTP failures to `IoError::Transport`; spirit-hello `lib.rs:101-112` renders the canned line) → 3-file fix (`main.rs:3062`, spirit-hello `lib.rs:118-120`, `spirits/hello-spirit/manifest.toml:12` → `claude-haiku-4-5-20251001`) → rebuilt → **live round-trip verified with operator key**. Fix committed as `53845402` on top of `ee3422d0` (Story 14-2a). Tree is clean.

**Remaining blast radius (live in tree today):**

| # | Finding | Receipt |
|---|---|---|
| E4 | Butler spirit declares retired `claude-3-haiku-20240307` | `spirits/butler/manifest.toml:21` |
| E5 | Researcher spirit declares retired `claude-3-5-sonnet-20241022` (retired 2026-02-19 → `claude-sonnet-4-6`) | `spirits/researcher/manifest.toml:47` |
| E6 | Pricing table keyed to retired `claude-3-5-sonnet` | `xtask/provider-pricing.toml:18` |
| E7 | Templates/examples mint new Spirits pre-pinned to a retired model | `templates/spirit-rust/manifest.toml:12`, `templates/spirit-ts/manifest.toml:12`, `examples/example-spirit/manifest.toml:12`, `examples/example-spirit-ts/manifest.toml:12` |
| E8 | Env contract governs provider URLs but omits model overrides entirely | `crates/maos-bin/src/env_contract.rs` (67-entry registry; `MAOS_OLLAMA_URL` :70 UserFacing; zero `MAOS_*_MODEL`) |
| E9 | Kernel enforces provider prefix only — model part of scope string is decorative | `crates/maos-kernel-core/src/inference/mod.rs:242` |
| E10 | No mechanical gate exists for model currency | `xtask/` (absent), cf. `kernel-core-baseline.toml` pattern |
| E11 | OpenAI/Ollama defaults also hardcoded; `gpt-4o-mini` currency unverified | `main.rs:3072`, `main.rs:3086` |

**Root cause (three layers, distinct verdicts).** Manifest pins = intentional design (explicit, static, attributable capability declaration — I12-consistent). Provider defaults in `main.rs` = governed-surface gap, NOT security (the env already carries the API key; `FIXME(secrets)` at `main.rs:3053` acknowledges deferred config work). Templates/fixtures = accidental copy-paste. The smell is not duplication — it is **absence of a falsifier**: nothing in CI turns red when a pinned model retires.

## Section 2: Impact Analysis

**Corrections to the input package (verified this run).**
- Input §2 said "Epic 19 in flight" — **wrong**. The live epic is **Epic 14** (`in-progress`): 14-0, 14-1, 14-2, 14-2a done; 14-2b, 14-3…14-9 backlog. No Epic-19 exists.
- Input W3 proposed adding an env-contract completeness test — **already exists**: Story 12.6 landed `check-env-contract` BLOCKING for maos-bin with read-shape detection (`env::var`/`var_os` literal + helper-indirected forms). W3's obligation inverts: keep the gate green while registering the four new vars.
- Epic 14 guards its boundary ("closed-enumerated sweep + Zero new PRD FRs"; the J1 lane lives outside it by precedent, lines 240-243 of sprint-status). Hence OD2: **standalone hotfix lane**.

**Epic impact.**
- **Epic 14:** still completable as planned; zero displacement. Two one-line pointers keep the backlog coherent: 14-5 (backends/multi-provider) will consume the new `model-currency.toml` when adding Bedrock/Vertex drivers; 14-8's registration census grows ~43 → ~47 reads (four `MAOS_*_MODEL` registered early as UserFacing). 14.7's registry migration carries the new entries to the shared home with zero behavior change (its own AC).
- **Epics 0–13:** closed; unaffected. No epic is invalidated; no new epic needed; no re-sequencing.
- **PRD:** no conflict — env overrides serve FR47 (provider-agnostic Inference Port) and operator-surface goals; no new FR (Epic-14 boundary respected); MVP scope untouched (v2.2 phase).
- **Architecture:** ZERO kernel-Δ across the lane @ **24472** (derived from `xtask/kernel-core-baseline.toml`, machine-checked — not restated). ADR-005/057 untouched. The 12.6→14.7 env-contract architecture is extended, not contradicted.
- **UX:** no UI/UX surfaces. N/A.
- **Other artifacts:** CI gate enrollment (mirrors the 12.6/14-2a `check-*` pattern); **measured `kloc.toml` grant required for the new xtask gate lines** (a grant lands WITH the lines it authorizes — T0 precedent); sprint-status rows + epic-14 note (this proposal).
- **Concurrency:** shared-write files are `crates/maos-bin/src/main.rs` + `xtask/src/main.rs`. All lanes currently holding writes are closed (14-2a committed `ee3422d0`; J1 lane closed). The hotfix lane names `model-pin-currency-gate-and-retired-pin-sweep` as first holder of both, per the J1 T0 boundary rule.

## Section 3: Recommended Approach

**Option 1 — Direct Adjustment: SELECTED** (effort Low · risk Low).
- Option 2 (Rollback): N/A — nothing to revert; the applied fix is desired and user-verified.
- Option 3 (MVP Review): N/A — MVP unaffected; timeline impact ≈ 1–1.25 dev-days.

**Rationale.** The incident class (retired pin + masked error + no falsifier) is closed by three small workstreams and one filed follow-up, all outside kernel-core, all schedulable immediately as a standalone lane. Folding into backlog Epic-14 stories (14.5) would delay closure behind unrelated provider work; minting inside Epic 14 would require a boundary amendment for no gain. Victor's reservation is baked in: the gate blocks **retired** IDs and must never enforce "latest" — pinning for reproducibility is the design value; the system only needs death detection.

## Section 4: Detailed Change Proposals

### CP-1 — Hotfix lane rows in `sprint-status.yaml` (insert after the 14-9 row)

```yaml
  # === MODEL-PIN CURRENCY HOTFIX LANE (correct-course 2026-09-01, sprint-change-proposal-2026-09-01.md) ===
  # Incident 2026-08-31: retired claude-3-haiku-20240307 broke live inference behind a masked
  # "provider unreachable" fallback; hello-spirit path fixed at 53845402. This lane closes the
  # remaining blast radius + recurrence. Standalone lane (J1-lane precedent — Epic 14's remit is
  # a closed-enumerated sweep). Parallelizable; shared-write order: the gate story holds
  # xtask/src/main.rs + crates/maos-bin/src/main.rs first. ZERO kernel-Δ @24472 all rows.
  model-pin-currency-gate-and-retired-pin-sweep: backlog  # W1+W2. xtask check-model-currency: local-data xtask/model-currency.toml (per-provider allowed + retired lists WITH replacement fields, display-only guidance — OD1 ratified); red on retired/unknown ID naming file:line; offline-safe (no network); scanned surfaces = spirits/*/manifest.toml, templates/*/manifest.toml, examples/*/manifest.toml, xtask/provider-pricing.toml, main.rs provider-construction literals; measured kloc.toml grant lands with the lines; CI enrollment mirrors 12.6/14-2a pattern. Sweep: 7 retired pins → current set (butler+researcher manifests, provider-pricing.toml, 4 template/example manifests; hello-spirit already current — do NOT re-edit). Hermetic test fixtures/fuzz seeds are inert — accepted debt, NOT swept. ACs per input package §W1/§W2 (AC1.1-1.4, AC2.1-2.3).
  provider-model-env-overrides: backlog  # W3. MAOS_ANTHROPIC_MODEL / MAOS_OPENAI_MODEL / MAOS_OLLAMA_MODEL / MAOS_DEFAULT_PROVIDER read at provider construction (main.rs:3059-3086), defaults = current literals; empty-string = unset (MAOS_JOURNAL_PATH empty-guard precedent, maos-audit/src/lib.rs:1423-1428); register 4 UserFacing entries in env_contract.rs — check-env-contract stays BLOCKING-green (12.6 mechanism, NOT a new completeness test); 14.7 migration carries entries to the shared home; gpt-4o-mini currency verified in-story before the OpenAI default is trusted (OD3). Depends: gate story (shared-file ordering only). ACs per input package §W3 (AC3.1-3.4 with AC3.3 corrected to "gate green").
  provider-error-surfacing: backlog  # OD4 SEPARATE STORY (scope text only until authored): unmask real HTTP status (401/404/429) through io/mod.rs:102-108 IoError::Transport flattening + spirit-hello lib.rs:101-112 canned fallback. Touches error taxonomy across providers — own preflight + §A6 net before dev. Filed by this correct-course; deliberately NOT bundled with the model-currency lane.
```

### CP-2 — Coherence note in `epic-14-scale-closers-ecosystem-readiness-v2-2.md` (append after the Sequencing paragraph)

> **Hotfix-lane pointers (2026-09-01, correct-course):** model-currency gate + retired-pin sweep + provider model env overrides landed as the standalone **model-pin currency hotfix lane** (see sprint-status; J1-lane boundary precedent — not Epic-14 scope). **14.5 consumes `xtask/model-currency.toml`** when adding Bedrock/Vertex drivers (new IDs enter the allowed list via the same re-pin ceremony). **14.8's census is ~47 reads, not ~43** — four `MAOS_*_MODEL` vars were registered UserFacing by the hotfix before this story runs.

### CP-3 — Deferred to story authoring (no planning edit now, by lane convention)

- Story files for the three rows above are authored by `bmad-create-story` before leaving backlog (row-without-file = scope text only).
- kloc.toml measured grant, CI enrollment, and the OD3 vendor check are in-story tasks at dev time, not planning edits.
- The input package stays untouched as the dated trigger artifact; this proposal supersedes its §2 "Epic 19" error and its W3 completeness-test AC.

## Section 5: Implementation Handoff

- **Scope: Minor** → route to Developer agent. Sequence: `bmad-create-story model-pin-currency-gate-and-retired-pin-sweep` → dev (quick-dev or dev-story) → review → then `provider-model-env-overrides`; `provider-error-surfacing` authored whenever scheduled, independent.
- **Success criteria:** W1 gate proven red (planted retired ID) and green (post-sweep), offline-safe, ≥2 scanner unit tests; W2 grep-zero on retired IDs across spirits/templates/examples/xtask pricing; W3 live smoke — `MAOS_ANTHROPIC_MODEL=<alt>` surfaces in `Capability scope:` and in the `inference.call` audit row; unset vars reproduce default behavior byte-identically; `check-env-contract` green; `check-kernel-baseline` 24472 zero-Δ; all standing discipline gates green at close.
- **Estimated effort:** W1 ≈ 0.5d · W2 ≈ 0.25d · W3 ≈ 0.5d (incl. verification). No sprint displacement.

---

### Appendix — Checklist status (Batch run)

| Section | Items | Status |
|---|---|---|
| 1 Trigger & context | 1.1–1.3 | [x] Done — incident + receipts E1–E12 (input package) + corrections this run |
| 2 Epic impact | 2.1–2.5 | [x] Done — Epic 14 unaffected; pointers only; no resequencing |
| 3 Artifact conflicts | 3.1–3.4 | [x] Done (3.3 N/A — no UX surface) — PRD/Architecture clean; CI + kloc + tracker edits enumerated |
| 4 Path forward | 4.1–4.4 | [x] Done — Option 1 Direct Adjustment selected; 4.2/4.3 N/A with rationale |
| 5 Proposal components | 5.1–5.5 | [x] Done — Sections 1–5 above |
| 6 Final review & handoff | 6.1–6.5 | [x] Done — 6.3 approved 2026-09-01; 6.4 sprint-status rows applied; 6.5 handoff Minor → Developer agent |
