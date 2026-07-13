---
baseline_commit: 48203705ff6f8b9f88d603ae7a68b533581d6f01
---

# Story 12.6: maos-bin env-contract remediation + honest gate enrollment

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Tech-debt remediation. NOT in the original Epic-12 5-story plan. Surfaced by the Story 12.5 code review and RESHAPED by a party-mode preflight (2026-07-13): the draft's "register 11 -> 0 violations" was itself a vacuous green (the gate is blind to 2 in-crate helper-reads) and its blanket "env contract" enrollment overclaimed a one-crate gate as a workspace guarantee. Scope is now honestly bounded to maos-bin; the workspace-wide contract (incl. provider API keys) is split to Story 12.7. Touches only crates/maos-bin/src/env_contract.rs + xtask config/CI + .github/workflows/discipline.yml -- ZERO kernel-core delta. -->

## Story

As the maos release/platform owner,
I want every `MAOS_*` environment variable read within `maos-bin` registered in the env-contract registry, the `check-env-contract` gate hardened so it cannot report a false "0 violations," and that gate wired into the ship-gate aggregate with a scope-honest promise,
so that the `maos-bin` configuration surface is fully and truthfully documented, `check-env-contract` is green **and meaningful** on `epic-12`, and unregistered `maos-bin` env reads become a hard CI failure instead of un-owned debt.

## Context (why this exists, and why it is scoped to maos-bin)

The Story 12.5 code review found `check-env-contract` RED since 2026-07-05 with unregistered `MAOS_*` reads introduced by now-`done` stories (11.4a PDP, 11.4c enterprise identity/at-rest/SIEM, 12.1 cohort daemon). Two facts from the 12.6 preflight bound this story honestly:

1. **The gate walks only `crates/maos-bin/src/**`** (`xtask/src/check_env_contract.rs:35-37`). `MAOS_*` reads in ~15 other crates — including provider **API keys** `MAOS_ANTHROPIC_API_KEY` / `MAOS_OPENAI_API_KEY` — are invisible to it. Enrolling this gate as a blanket "env contract enforced" ship gate would promise workspace coverage it does not deliver. **The workspace-wide contract is Story 12.7, not this story.**
2. **The gate's own green can lie inside the one crate it covers.** It matches a literal `env::var("MAOS_…"` / `env::var_os("MAOS_…"` on a single line, so it is **blind to helper-indirected reads**: `enterprise_pdp_runtime.rs:86,90` read `MAOS_PDP_REFRESH_INTERVAL_MS` / `MAOS_PDP_STALENESS_TTL_MS` via `duration_ms_from_env(name)` → `env::var(name)`, and to the `vars_os()` prefix scan at `enterprise_identity.rs:405`. The real in-`maos-bin` unregistered count is **13, not 11.** Registering only the 11 the gate can see leaves 2 unregistered vars sitting in the "fixed" crate under a green badge.

## Acceptance Criteria

1. **Every `MAOS_*` env read in `maos-bin/src` is registered — all 13, not just the gate-visible 11.** Append 13 `EnvVar` entries to `MAOS_ENV_REGISTRY` (`crates/maos-bin/src/env_contract.rs`) with accurate `purpose` and `stability` (all `UserFacing` — operator-configured enterprise/cohort inputs). The registry's unit tests (`registry_names_are_unique`, `all_entries_start_with_maos_prefix`) still pass.

2. **The gate can no longer report a false "0 violations" — and must not red on non-reads either.** Harden `xtask/src/check_env_contract.rs` to detect env **read shapes** and register-check the `MAOS_*` name each yields: (a) literal `env::var("MAOS_…"` / `env::var_os("MAOS_…"` (existing); (b) helper-indirected reads — a `MAOS_*` string literal passed as the name argument to an env-reading helper, concretely `duration_ms_from_env("MAOS_…"` (`enterprise_pdp_runtime.rs:86,90`); (c) `any_env_with_prefix("MAOS_…_")` prefix scans (`enterprise_identity.rs:76,79,82`) — validated by requiring ≥1 **registered** var to share the prefix, NOT by trying to register the prefix itself. It MUST NOT flag non-read `MAOS_*` literals — see the enumerated exclusion list in Task 2 (writes `set_var`/`remove_var`, child-process `.env("MAOS_…", …)` on a `Command`, error/format strings). **Proven-red (positive):** a deliberately-unregistered helper-indirected read reds the gate; registering/removing it → green — the two real PDP duration vars are the live proof (before AC1 they must red). **Proven-clean (negative):** the 6 known non-read sites (3 prefix scans + `MAOS_SUPERVISION_FAST`/`MAOS_SCHEDULE_FAST` writes + `MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT` child-env ×2) MUST NOT be flagged — a test asserts they stay green. After AC1+AC2, `cargo run -q -p xtask -- check-env-contract` reports **PASS, 0 violations**, honest for `maos-bin/src`.

3. **`check-env-contract` is enrolled as a blocking ship gate, scoped honestly to `maos-bin`.** It currently runs only nested inside the `invariant-lock` job (`discipline.yml:364-370`), which is NOT in the `v1-0-ship-gate` `needs:` list — so its RED never reaches the ship aggregate ("a smoke detector wired to a light nobody looks at"). Wire it in (it hard-blocks by construction — `run()` returns `Err` on violations, no phase disposition of its own):
   - `EXPECTED_GATES` **and** the `is_story10_ship_gate` match arm in `xtask/src/check_ship_gate_completeness.rs` (both — the match arm gap would silently exempt the disposition check);
   - `gate-registry.toml`: add `check-env-contract` to the top `gates = [...]` array AND a `[[ship_gate]]` block with an explicit blocking disposition;
   - `.github/workflows/discipline.yml`: a sibling `check-env-contract` job (copy the `check-service-boundary` shape, `:312-323`) added to the `v1-0-ship-gate` `needs:` array (`:2748-2750`).
   - **Scope-honest promise:** the gate's PASS/FAIL message and its `[[ship_gate]]` comment MUST state its scope is `maos-bin/src` and reference Story 12.7 for workspace coverage — so the ship badge does not imply a workspace-wide contract it does not enforce. (Do NOT rename the gate: Story 12.7 widens it to the workspace, at which point `check-env-contract` becomes the accurate name. A rename now is churn 12.7 would undo, and a gate rename is an invariant-lock breaking change.)
   - `cargo run -q -p xtask -- check-ship-gate-completeness` PASSES.

4. **Zero regression, ZERO kernel delta.** `cargo build --workspace` and `cargo test -p maos-bin` green; the three registry-consuming subsystems are byte-behaviorally unchanged (registration is additive metadata; the detector fix is xtask-only). `cargo run -q -p xtask -- check-kernel-baseline` stays PASSED at **23141** (no file under `crates/maos-kernel-core/src` is touched).

## Out of Scope — promoted to Epic 14 (Stories 14.7–14.9, correct-course 2026-07-13)

The **workspace-wide** env contract is a separate, larger story with its own design. It is written down here with a number so it cannot rot the way this debt did:

- ~30 `MAOS_*` reads across ~15 crates that `check-env-contract` never opens (`maos-audit` 8, `maos-domain/security` incl. 7 registry vars, `maos-registry`, `maos-cli`, `maos-eval`, `maos-siem`, `maos-shell`, `maos-providers`, `maos-kernel-core` test-timing knobs, etc.).
- **Provider API keys** `MAOS_ANTHROPIC_API_KEY` (`maos-providers/src/anthropic.rs:37`) and `MAOS_OPENAI_API_KEY` (`openai.rs:36`) — unregistered secrets — MUST be named and classified in 12.7.
- **Design fork for 12.7:** one workspace-wide scan against a single registry vs. per-crate registries; how to classify secrets vs config; how the gate's name/scope generalizes. Not decided here.

## Tasks / Subtasks

- [x] Task 1 — Register all 13 `maos-bin` `MAOS_*` reads (AC: #1)
  - [x] Append to `MAOS_ENV_REGISTRY` before the closing `];` (currently line 284), matching the existing struct-literal style.
  - [x] **11.4c enterprise identity / at-rest / SIEM (7)** — `UserFacing`:
    - `MAOS_SIEM_FILE` — "Path to the local SIEM file sink; when set and non-empty, enables enterprise SIEM export (Story 11.4c)"
    - `MAOS_KMS_MASTER_KEY` — "Hex-encoded 32-byte org master key for the LocalMasterKeyKms at-rest AEAD envelope; absent keeps byte-identical Option-A plaintext (Story 11.4c)"
    - `MAOS_SSO_JWKS` — "Static JWKS document for the OIDC assertion verifier (Story 11.4c)"
    - `MAOS_SSO_ISSUERS` — "Comma-separated allowlist of accepted OIDC issuers (Story 11.4c)"
    - `MAOS_SSO_AUDIENCE` — "Required OIDC audience claim for SSO assertion verification (Story 11.4c)"
    - `MAOS_SSO_ALGS` — "Optional comma-separated allowed JWS algorithms (default RS256,ES256) (Story 11.4c)"
    - `MAOS_SSO_ASSERTION` — "OIDC assertion (JWT) presented at enterprise-governed capability issuance when SSO is configured (Story 11.4c)"
  - [x] **11.4a enterprise PDP (5 — the 3 gate-visible + the 2 gate-blind)** — `UserFacing`:
    - `MAOS_PDP_POLICY_FILE` — "Explicit path to a Cedar (.cedar) PDP policy file (Story 11.4a)"
    - `MAOS_PDP_POLICY_INLINE` — "Explicit inline Cedar PDP policy text (Story 11.4a)"
    - `MAOS_PDP_POLICY` — "Legacy PDP policy source: inline Cedar text or file path via file:/inline: prefix (Story 11.4a)"
    - `MAOS_PDP_REFRESH_INTERVAL_MS` — "PDP policy refresh interval (ms) for the enterprise PDP runtime reconciler (Story 11.4a; read via duration_ms_from_env at enterprise_pdp_runtime.rs:86)"
    - `MAOS_PDP_STALENESS_TTL_MS` — "PDP staleness TTL (ms); after expiry PDP-granted caps revert to deny (Story 11.4a; enterprise_pdp_runtime.rs:90)"
  - [x] **12.1 cohort daemon (1)** — `UserFacing`:
    - `MAOS_COHORT_DAEMON_CONFIG` — "Path to the cohort A2A daemon TOML config (manifest path + digest summary) (Story 12.1)"
- [x] Task 2 — Harden the detector so the green is honest (AC: #2)
  - [x] In `xtask/src/check_env_contract.rs`, replace the single-line literal match with **read-shape detection**. For each shape, extract the `MAOS_*` literal and require it be in the registry (or, for a prefix scan, require a registered member): (a) `env::var("MAOS_…"` / `env::var_os("MAOS_…"`; (b) `duration_ms_from_env("MAOS_…"` (the only in-crate env-reading helper — the literal sits at the call site, so a literal-argument scan catches it); (c) `any_env_with_prefix("MAOS_…_")` → PASS iff some registered name `starts_with` that prefix (after AC1: `MAOS_SSO_`→5 members, `MAOS_KMS_`→`MAOS_KMS_MASTER_KEY`, `MAOS_SIEM_`→`MAOS_SIEM_FILE`).
  - [x] **Do NOT use a blanket "any `MAOS_` literal" rule** — it false-positives on 6 non-read sites the preflight found: writes `std::env::set_var("MAOS_SUPERVISION_FAST"…)` (`main.rs:5083`), `set_var("MAOS_SCHEDULE_FAST"…)` (`:8878`); child-process env `.env("MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT", …)` on a `Command` (`:9318,:9544`); and the 3 prefix literals (handled by (c), never register-checked as names). These are real `MAOS_*` vars read in OTHER crates → they belong to Story 12.7, and must stay invisible to this gate. The detector matches read call-shapes, not bare string literals.
  - [x] Add BOTH gate tests: **proven-red** — an unregistered helper-indirected read reds it; **proven-clean** — the 6 non-read sites above stay green. Verify: before Task 1 the hardened gate reports **13** violations (11 literal + 2 helper); after Task 1, **0**.
- [x] Task 3 — Enroll as a scope-honest blocking ship gate (AC: #3)
  - [x] `xtask/src/check_ship_gate_completeness.rs`: add `check-env-contract` to `EXPECTED_GATES` AND to the `is_story10_ship_gate` match arm (`:110-142`) — both, or the disposition check silently exempts it.
  - [x] `xtask/gate-registry.toml`: add `"check-env-contract"` to `gates = [...]` (`:5-129`) and a `[[ship_gate]]` block (mirror `check-pentest-gate` `:138-140`) with a blocking disposition; comment names the `maos-bin` scope + Story 12.7.
  - [x] `.github/workflows/discipline.yml`: add a `check-env-contract` job (copy `check-service-boundary` `:312-323`) and list it in the `v1-0-ship-gate` `needs:` (`:2748-2750`). Leave the existing nested `invariant-lock` step as-is or remove it (dev's call; do not double-fail confusingly).
  - [x] Make the gate's PASS/FAIL message state scope: e.g. `check-env-contract: PASS (N maos-bin/src MAOS_* vars registered, 0 violations; workspace coverage tracked in Story 12.7)`.
  - [x] `cargo run -q -p xtask -- check-ship-gate-completeness` PASSES.
- [x] Task 4 — Repair FKCS frozen-snapshot oracle semantics (user-authorized regression remediation)
  - [x] Preserve `xtask/fkcs-baseline.toml` as the historical `frozen-kernel-v2.0` snapshot by correcting its stale count from 23082 to the tagged revision's measured 23081; do not re-pin it to the current 23141 kernel baseline.
  - [x] Make FKCS validate the frozen snapshot against the tagged frozen revision while `check-kernel-baseline` validates current `HEAD`; a legitimate later kernel delta must not make the frozen-tag leg red.
  - [x] Add a regression test proving the frozen snapshot and current approved kernel baseline can diverge while both contracts remain valid.
- [x] Task 5 — Verify (AC: #4)
  - [x] `cargo build --workspace`; `cargo test -p maos-bin`; `cargo run -q -p xtask -- check-env-contract` (PASS, honest 0); `cargo run -q -p xtask -- check-ship-gate-completeness` (PASS); `cargo run -q -p xtask -- check-kernel-baseline` (23147 after user-authorized formatting-only reconciliation); `cargo test --workspace`.

## Dev Notes

### Registry + gate contract (read fully first)

- `crates/maos-bin/src/env_contract.rs`: `EnvVar { name, purpose, stability }` (`:1-5`); `enum EnvStability { HarnessOnly, UserFacing }` (`:7-11`); `MAOS_ENV_REGISTRY` `:13` → `];` `:284` (54 entries today; append before `:284`). Unit tests `:290-322`. All 13 new entries satisfy unique-name + `MAOS_`-prefix.
- `xtask/src/check_env_contract.rs`: `run()->Result<(),String>` (`:4`); walks `<maos_bin_dir>/src` recursively via `fs_walk::collect_rs_files` (`:35-37`, `fs_walk.rs:4-13`); skips only `env_contract.rs` (`:39-42`) and `//` lines (`:44-48`); matches literals `env::var("MAOS_` / `env::var_os("MAOS_` via `line.find` (`:52-58`); PASS/`Ok` when empty else `Err` (`:86-100`). **The literal-match is the blindness AC2 fixes.** `std::env::var(...)` is caught (substring); `env::var(name)` and `vars_os()` are not.

### The 13 (grounded, scout-verified)

| # | Var | Site | Gate sees it? | Story |
|---|---|---|---|---|
| 1 | `MAOS_SIEM_FILE` | `enterprise_identity.rs:179` | yes | 11.4c |
| 2 | `MAOS_KMS_MASTER_KEY` | `:410` | yes | 11.4c |
| 3-6 | `MAOS_SSO_JWKS/ISSUERS/AUDIENCE/ALGS` | `:420-423` | yes | 11.4c |
| 7 | `MAOS_SSO_ASSERTION` | `main.rs:114` | yes | 11.4c |
| 8-10 | `MAOS_PDP_POLICY_FILE/INLINE/POLICY` | `enterprise_pdp_runtime.rs:32-34` | yes (`:32` is `var_os`, still literal → caught) | 11.4a |
| 11-12 | `MAOS_PDP_REFRESH_INTERVAL_MS`, `MAOS_PDP_STALENESS_TTL_MS` | `:86,:90` via `duration_ms_from_env(name)` | **NO — helper-indirected** | 11.4a |
| 13 | `MAOS_COHORT_DAEMON_CONFIG` | `main.rs:7568` | yes | 12.1 |

Do NOT change any call site — they are correct; the fix is registry + detector + enrollment.

### Enrollment mechanics (scout-verified, 3 files)

- `check_ship_gate_completeness.rs`: check #1 (`:89-94`) fails a gate in `EXPECTED_GATES` absent from `discipline.yml`'s `v1-0-ship-gate needs:`; check #2 (`:110-142`) requires a `[[ship_gate]]` disposition only for names in the `is_story10_ship_gate` match — **so add the name to BOTH** or the disposition is silently unenforced.
- `gate-registry.toml`: `gates = [` `:5`→`:129`; `[[ship_gate]]` example `:138-140` (`check-pentest-gate`, `disposition = { v1_0 = "blocking-when-present", v1_5 = "blocking-when-present" }`). Schema: `corpus_types.rs:76-96`.
- coverage-matrix is **one-way** (`coverage_matrix.rs:140-143` checks YAML rows against the registry, never the reverse) — adding to `gates[]` does NOT force a `tests/coverage-matrix.yaml` row. A coverage row is optional policy, not required.
- `discipline.yml`: gate currently a nested step under `invariant-lock` `:364-370`; `v1-0-ship-gate` aggregate `:2748`, `needs:` `:2750`; simple-job template `check-service-boundary` `:312-323`.
- `CURRENT_PHASE = v1_5` everywhere (`check_fkcs.rs:16`); `is_blocking_at` treats `blocking`/`blocking-when-present` as blocking (`:604-608`). **But `check-env-contract` needs no phase trick** — its `run()` returns `Err` unconditionally on violations, so a `needs:` membership hard-blocks the aggregate today (contrast scale-churn's advisory-at-v1_5 — the 12.1 RR8 trap does NOT apply here).

### Testing standards

- AC1/AC4: acceptance is the gate (honest PASS) + existing `env_contract.rs` unit tests. No behavioral test needed for registration.
- AC2: the proven-red gate test IS the deliverable — a helper-indirected unregistered read must red the gate (else the green is vacuous, the exact defect this story exists to prevent).
- AC3: `check-ship-gate-completeness` PASS is the enrollment proof. Keep it hermetic; run only touched checks + `cargo test -p maos-bin`.

### References

- [Source: crates/maos-bin/src/env_contract.rs#1-13,284] — registry contract + bounds.
- [Source: xtask/src/check_env_contract.rs#35-58,86-100] — walk root + literal-match blindness + Err-on-violation.
- [Source: crates/maos-bin/src/enterprise_pdp_runtime.rs#86,90] — the 2 helper-indirected vars via duration_ms_from_env.
- [Source: crates/maos-bin/src/enterprise_identity.rs#405] — vars_os() prefix scan blind spot.
- [Source: xtask/src/check_ship_gate_completeness.rs#16,89-94,110-142] — EXPECTED_GATES + both enrollment checks.
- [Source: xtask/gate-registry.toml#5,129,138-140] — gates[] + [[ship_gate]] schema.
- [Source: .github/workflows/discipline.yml#312-323,364-370,2748-2750] — job template + current nested step + ship aggregate.
- [Source: xtask/src/coverage_matrix.rs#140-143] — one-way coverage cross-ref (no forced row).
- [Source: _bmad-output/party-mode/memories/installed/.memlog.md] — 2026-07-13 12.6 preflight (reshape rationale, ring split).

## Dev Agent Record

### Agent Model Used
- openai-codex/gpt-5.6-terra

### Implementation Plan

- Register the complete `maos-bin` environment surface, prove the detector catches helper-indirected reads without false-positiveing non-reads, then enroll the scoped gate as a blocking aggregate dependency.

### Debug Log References

- `cargo test --workspace` fails in pre-existing `xtask/tests/fkcs_oracle.rs::live_triple_reconciles_real_kernel_core_line_count_not_a_literal`: frozen FKCS baseline pins `23082`, while the live kernel baseline is `23141`. `cargo run -q -p xtask -- check-kernel-baseline` independently passes at `23141`; no kernel files were changed by this story.
- `cargo fmt --check` fails only on pre-existing formatting drift in `crates/maos-a2a-tcp/tests/t_12_5_cohort_hot_swap.rs`, `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs`, and `crates/maos-kernel-core/tests/hot_swap_halt_continuity_corpus_integration.rs`; none are Story 12.6 files.

### Completion Notes List

- Reshaped by party-mode preflight 2026-07-13 (Winston/Murat/John/Amelia/Vex/Grumbal/Boundary/Yui/Mary/Dana): register 13 not 11, harden the detector so "0 violations" is honest, enroll scoped-to-maos-bin; workspace-wide contract split to Story 12.7 (Lunarpulse ratified option 1).
- Task 1: added all 13 `UserFacing` `maos-bin` registry entries. `cargo test -p maos-bin env_contract::tests` passed (3 tests).
- Task 2: replaced literal-only scanning with read-shape detection for direct reads, `duration_ms_from_env`, and prefix scans. The helper-indirected proven-red test failed before the detector change and now passes; the non-read/prefix-clean test passes. Live gate reports 67 registered `maos-bin/src` variables and 0 violations.
- Task 3: enrolled `check-env-contract` as a blocking `v1-0-ship-gate` dependency, with registry disposition and scope-honest CI/output text. `check-ship-gate-completeness` and YAML parsing pass.
- Task 4: separated FKCS historical-snapshot validation from current kernel-baseline validation. The tag-grounded frozen count is 23081; current `HEAD` remains independently pinned at 23141. `cargo test -p xtask --test fkcs_oracle` passed (10 tests); `check-fkcs --json` is green across all 8 legs.
- Task 5: final verification passed — `cargo build --workspace`; `cargo test --workspace` (3233 passed, 77 ignored); `cargo test -p maos-bin` (63 passed); `check-env-contract`; `check-ship-gate-completeness`; `check-kernel-baseline` (23147); `check-fkcs --json` (8/8 green); and `cargo fmt --check`.
- User-authorized formatter normalization re-pinned the current kernel baseline from 23141 to 23147 (+6 physical formatting-only lines). The original Story 12.6 env-contract change remains zero functional kernel delta.

### File List

- `.github/workflows/discipline.yml`
- `_bmad-output/implementation-artifacts/12-6-env-contract-registry-remediation.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `crates/maos-bin/src/env_contract.rs`
- `xtask/gate-registry.toml`
- `xtask/src/check_env_contract.rs`
- `xtask/src/check_ship_gate_completeness.rs`
- `xtask/fkcs-baseline.toml`
- `xtask/src/check_fkcs.rs`
- `xtask/tests/fkcs_oracle.rs`
- `xtask/kernel-core-baseline.toml`
- `crates/maos-a2a-tcp/tests/t_12_5_cohort_hot_swap.rs`
- `crates/maos-kernel-core/src/hot_swap/post_swap_monitor.rs`
- `crates/maos-kernel-core/tests/hot_swap_halt_continuity_corpus_integration.rs`
INS.TAIL:

## Change Log

- 2026-07-13: Registered all `maos-bin` environment reads, hardened and enrolled the scoped env-contract gate, repaired FKCS frozen-snapshot validation, and reconciled user-authorized formatting-only kernel baseline drift.

## Review Findings

_Code review 2026-07-13 (bmad-code-review; Blind Hunter + Edge Case Hunter + Acceptance Auditor). Dev model openai-codex/gpt-5.6-terra. Live gate independently verified GREEN + scope-honest._

### patch

- [x] [Review][Patch][HIGH] Detector defeated by non-single-line / non-canonical read syntax — FIXED 2026-07-13: AST-based read-shape detection recognizes multiline, raw-string, and parenthesized literals while failing closed on unparseable `.rs` sources. Regression tests cover an unregistered multiline raw helper read and registered multiline/raw/parenthesized direct, helper, and prefix reads. [xtask/src/check_env_contract.rs:1-89]
- [x] [Review][Patch][LOW] Naive substring scan flags non-reads as reads — FIXED 2026-07-13: AST call-path matching excludes comments, string contents, setters, `Command::env`, and lookalike paths such as `testenv::var`; regression coverage exercises each. [xtask/src/check_env_contract.rs:42-89,267-294]
- [x] [Review][Patch][LOW] `any_env_with_prefix` accepts a partial-token prefix — FIXED 2026-07-13: prefix scans now require an underscore-terminated prefix plus a registered member; regression test proves `MAOS_SSO_JW` fails despite registered `MAOS_SSO_JWKS`. [xtask/src/check_env_contract.rs:142-163,297-309]
- [x] [Review][Patch][LOW] Unit tests never prove the shape-(b) red→green transition — FIXED 2026-07-13: registered multiline/raw `duration_ms_from_env` is explicitly GREEN; unregistered multiline/raw helper input is explicitly RED. [xtask/src/check_env_contract.rs:213-264]
- [x] [Review][Patch][LOW] FKCS test encodes “must differ” rather than “may differ” — FIXED 2026-07-13: it now validates frozen-tag and current-baseline contracts independently, allowing equality if a legitimate later revision returns to the frozen line count. [xtask/tests/fkcs_oracle.rs:141-155]

### defer

- [x] [Review][Defer][LOW] FKCS diff-oracle leg self-compares live surfaces — an ABI/host regression can derive green [xtask/src/check_fkcs.rs:140-148,334-364] — deferred, pre-existing (untouched by 12.6; leg tests oracle derivation logic, not live ABI drift)
- [x] [Review][Defer][LOW] FKCS frozen-tag leg uses CWD-relative kernel-baseline paths; fails from a workspace subdirectory [xtask/src/check_fkcs.rs:94-106] — deferred, pre-existing (CI runs from workspace root; new `workspace_root()` fixed only `git()`)

### dismissed (2)

- Escaped-quote name truncation in `maos_literal_arguments` — env var names cannot contain `"`; input unreachable.
- AC4 "zero kernel delta" contradiction (`post_swap_monitor.rs` test-only reformat + baseline 23141→23147) — Lunarpulse-authorized, documented FLAG-Winston in `kernel-core-baseline.toml`; byte-behavior-neutral.
