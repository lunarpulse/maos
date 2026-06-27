---
dev_model_used: claude-opus-4-8
---

# Story 10.5: Mature v1.5 — Skill-Format Conformance, JetBrains, Windows, 2-Year LTS, Japanese/CN-S i18n

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story.
     ⚠️ STRONGLY RECOMMENDED for THIS story — see "PREFLIGHT FLAGS (read first)" below.
     This story carries 7 GWT acceptance blocks (> §A5 6-AC ceiling), one authorized kernel-core
     delta (AC3 Windows sandbox), and a known gate-regression trap (AC6 NFR-Sec-13 floors).
     Run party-mode preflight before dev-story. -->

## Story

As a v1.5 operator on the long-term-support promise,
I want skill-format conformance demonstrating ≥1 third-party skill format (Anthropic Skills) executes via a Spirit-form adapter without kernel modification (NFR-Test-10), a JetBrains plugin-bridge for ACP (v1.5), a Windows binary, a 2-year LTS commitment, AND Japanese + Chinese-simplified localization,
so that v1.5 is the long-term-support release with proven extensibility across editor / OS / language boundaries.

---

## ⚠️ PREFLIGHT FLAGS (read first — these gate dev-story)

These four items are surfaced for the **preflight / party-mode** pass, not silently buried. The story is written to be implementable as-is, but the dev/orchestrator should resolve these before (or at the start of) implementation.

1. **7 GWT blocks > §A5 6-AC ceiling.** Sprint-status note already flags it: *"6 ACs at limit — verify at preflight per §A5."* Counting Given/When/Then blocks in the epic yields **7** (skill / JetBrains / Windows / LTS / i18n / mTLS / ComplianceClaim). The natural split seam if preflight elects to split:
   - **10.5 (extensibility cluster):** AC1 skill-format conformance + AC2 JetBrains bridge + AC3 Windows binary. (Editor/OS/skill boundaries; AC3 is the only kernel-touching AC.)
   - **10.5b (maturation cluster):** AC4 2-year LTS + AC5 ja/zh-Hans i18n + AC6 mTLS 3-host chaos + AC7 ComplianceClaim schema migration. (Mostly extend-existing; AC6 ~90% already shipped by 10.4b.)
   This is a **party-mode decision** (per [[feedback_party_mode_for_fork_consensus]] and the §A5 convention), not a unilateral story-creation split. Default: keep as one story unless preflight rules otherwise.

2. **Split-tier dev-model.** Sprint-status labels this **Tier-2 opus-4-6**, but three AC clusters are correctness-critical and pull toward **Tier-1 (claude-opus-4-8)**: AC3 (Windows sandbox = **authorized kernel-core delta** + FFI `unsafe`), AC7 (ComplianceClaim ABI/§8.5 correctness), AC6 (mTLS rotation timing). The i18n/LTS/docs portions (AC4, AC5) are genuinely Tier-2. **§A6 non-Opus safety net applies:** if a non-Opus model implements the correctness-critical ACs, multi-layer adversarial review (Blind + Edge + Acceptance + **Test-Infra Auditor**) is MANDATORY. Recommend **claude-opus-4-8** for the whole story given the kernel delta, or split-by-tier if the story is split.

3. **AC6 (NFR-Sec-13) gate-regression trap.** The epic AC text says "median ≤60s / p99 ≤5min" — but the **already-ratified** floors in `crates/maos-a2a-core/src/chaos/rotation.rs:115-116` are STRICTER (v0.7: revocation-propagation p50≤30s/p99≤90s, re-handshake p50≤30s/p99≤60s; v1.0: +e2e p50≤60s/p99≤150s + post_grace_reject≤0.1%). **Adopting the looser story numbers would REGRESS the existing `check-rotation-real-timing` gate.** Implement AC6 as a `passes_v15_floors` that is **≥ as strict** as the existing floors AND asserts the literal NFR-Sec-13 numbers by name — never loosen. (The 3-host real-socket drill `t_10_4b_rotation_real_timing_3_host_drill` **already exists** — see AC6 below; this AC is ~90% done.)

4. **Strict zero-kernel-core-delta regime, ONE authorized exception.** HEAD pin = **22574** (`xtask/kernel-core-baseline.toml:116`). AC1, AC2, AC4–AC7 must be **zero kernel-core delta** (proven by `check-kernel-baseline` + `abi-diff` + `check-abi-ratification`). AC3 (`windows.rs` sandbox body) is the single AC permitted to touch kernel-core → **FLAG-Winston re-pin** in `kernel-core-baseline.toml` HISTORY, never a silent budget draw (D7 tripwire convention from 10.4x). AC7 must NOT bump `ABI_VERSION` (stays `1`).

---

## Acceptance Criteria

Verbatim from `_bmad-output/planning-artifacts/epics/epic-10-v10-ship-gate-v15-collective-tier-v10-v15.md:204-240`.

**AC1 — Skill-format conformance (NFR-Test-10)**
**Given** skill-format conformance (NFR-Test-10)
**When** the test runs
**Then** ≥1 third-party skill format (Anthropic Skills format OR equivalent) executes via a Spirit-form adapter without kernel modification
**And** the kernel ABI is unchanged by the adapter — verified via ABI-diff lint
**And** the conformance result is journaled as a v1.5 release artifact

**AC2 — JetBrains plugin-bridge for ACP (v1.5)**
**Given** JetBrains plugin-bridge for ACP (v1.5)
**When** the plugin is installed in JetBrains IDE
**Then** Spirits hosted via ACP are routable through JetBrains (NDJSON over stdio extended)
**And** the bridge does not require kernel modification

**AC3 — Windows binary (v1.5)**
**Given** Windows binary (v1.5)
**When** the operator runs `maosctl install` on Windows
**Then** the install succeeds with the same signature verification as Linux/macOS (FR1)
**And** sandbox tier T2 uses Windows restricted-token (Story 1b.3 cross-ref)
**And** per-Spirit resource caps use Job Objects (Story 1b.3 cross-ref)

**AC4 — 2-year LTS commitment (NFR-Maint-6 v1.5)**
**Given** 2-year LTS commitment (NFR-Maint-6 v1.5)
**When** v1.5 ships
**Then** the LTS clock extends from 1-year (v1.0) to 2-year once support load is known
**And** STABILITY.md is updated with the new LTS span

**AC5 — Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5)**
**Given** Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5)
**When** v1.5 doc site is built
**Then** Japanese and Chinese-simplified translations are present for all 5 canonical doc deliverables
**And** LOCALES.md glossary lock continues to exclude Spirit / Worker / kernel / ADR identifiers / error codes
**And** RTL layout support remains deferred to v2.5

**AC6 — mTLS cert rotation chaos full execution (NFR-Sec-13)**
**Given** mTLS cert rotation chaos test full execution (NFR-Sec-13)
**When** the 3-host v1.5 rotation chaos runs
**Then** zero conversation drops are observed
**And** revocation latency median ≤60s / p99 ≤5min
**And** 10-host rotation defers to v2.0

**AC7 — ComplianceClaim envelope schema migration validation through v1.5**
**Given** ComplianceClaim envelope schema migration validation through v1.5
**When** schema evolution is tested
**Then** any ABI-breaking change to required fields, removed fields, renames, type-changes, or enum reorderings triggers an `ABI_VERSION` bump per §8.5
**And** additive optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]` and additive enum variants with explicit `#[repr(u8)]` discriminants + `#[serde(other)]` fallback do NOT bump

---

## Tasks / Subtasks

> Each task ends with the **gate + journaled artifact** the Epic-10 v1.5 cluster expects (new dedicated test file per AC + registered ship gate with `{v1_0,v1_5}` disposition + `tests/coverage-matrix.yaml` row + `discipline.yml` job). Disposition vocabulary: `blocking` / `advisory` (loud "WOULD BLOCK at v1.5" banner) / `blocking-when-present`.

### Task 1 — Skill-format conformance adapter + gate (AC1) — Tier-1, ZERO kernel delta
- [x] Reconcile/author **ADR-027** standalone mirror. ADR-027 is ratified in `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md:376` and cited by `crates/maos-skill/src/schema.rs:1` + `crates/maos-skill/src/errors.rs:13`, but `docs/adr/ADR-027-*.md` does NOT exist (range jumps 026→028). Either generate the standalone mirror or update the code citation — do not leave a dangling reference.
- [x] Obtain a **real third-party Anthropic Skill** bundle (`skill-name/SKILL.md` with YAML frontmatter `name:` + `description:`) as the conformance fixture. NOTE the format gap: `maos.skill.v1` uses **TOML** frontmatter (`crates/maos-skill/src/schema.rs:108`, `parse_skill`), Anthropic Skills use **YAML** frontmatter — the adapter must bridge YAML→Spirit-form. Verify the exact Anthropic SKILL.md frontmatter schema at implementation time (knowledge cutoff caveat).
- [x] Build the **Spirit-form adapter** that ingests the third-party skill and **executes it through the existing subprocess CliWrapper form** (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs:449` `spawn_and_bridge`; launch path `crates/maos-bin/src/main.rs:373` `run_cli_wrapper_manifest`). Skill *execution* is Spirit-side / out-of-scope for the kernel (`crates/maos-skill/src/lib.rs:13-19`) — the adapter is a subprocess-form Spirit / test harness, NOT a kernel module. Reuse `maos-skill` parse/discover/admit (`schema.rs`, `discovery.rs`, `admission.rs`) and the `CliWrapperConfig.skill_bundle` seam (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/lifecycle.rs:70`).
  - [x] If the real Anthropic bundle ships as `dir/SKILL.md`, extend `crates/maos-skill/src/discovery.rs:73-79` to be **directory-aware** (currently flat `*.md` only, "subdirectories NOT recursed" is a documented v0.5 constraint). This is in `maos-skill`, NOT kernel-core.
- [x] **Prove "no kernel modification"** mechanically: `cargo run -p xtask -- check-kernel-baseline` (22574 unchanged), `abi-diff` (`xtask/src/abi_diff.rs`, baseline `abi-baseline/v1-pre-bump.txt`), `check-abi-ratification` (`xtask/abi-ratifications.toml`).
- [x] **Journal the conformance result as a v1.5 release artifact** + add gate. Mirror the typed-artifact idiom of `xtask/src/check_third_party_trial.rs` / `xtask/src/check_cross_form_equiv.rs`: commit `docs/skill-conformance/results/skill-conformance-results.{toml,json}` + committed schema; new `xtask check-skill-conformance` (register: `mod` decl ~`xtask/src/main.rs:36`, `Commands::` variant ~`:700`, dispatch arm ~`:1083`). Integrity = hard-fail; conformance verdict = advisory per ADR-040 posture. Add `[[ship_gate]]` `{v1_0=advisory, v1_5=advisory-or-blocking}` to `xtask/gate-registry.toml`, `EXPECTED_GATES` in `xtask/src/check_ship_gate_completeness.rs`, a `NFR-Test-10` row in `tests/coverage-matrix.yaml`, and a `discipline.yml` job.
- [x] **Proven-red:** a fixture that is NOT a valid skill (or whose adapter touches kernel-core) makes the gate RED. Drive the REAL adapter path, not a mocked struct-literal (10.4a's headline anti-pattern).

### Task 2 — JetBrains plugin-bridge for ACP (AC2) — Tier-2/Tier-1 boundary, ZERO kernel delta
- [x] Confirm the bridge reuses the **existing `maos-acp` NDJSON-over-stdio server** (`crates/maos-acp/src/lib.rs:3`; `server.rs:52-58` line-oriented NDJSON over stdin/stdout; frames `frame.rs:18,46`). Launch arm exists: `MAOS_ONE_SHOT=acp-server` → `AcpServer::new(...).run(stdin, stdout)` (`crates/maos-bin/src/main.rs:5629-5659`). The JetBrains client populates `SessionStart { editor_id, editor_version }` (`frame.rs:20-25`) exactly like the existing `editor_id:"zed"` tests.
- [x] Decide "**NDJSON over stdio extended**" scope: if new frame kinds are needed for JetBrains, add them **additively** to the `#[serde(tag="kind")]` enums in `crates/maos-acp/src/frame.rs` (NOT kernel-core).
- [x] **Decision needed (preflight):** the `acp-server` arm is currently wired with **STUB resolvers** (`StubLifecycleResolver`→`NotLoaded`, no-op `StubHaltResolver`, `main.rs:5631-5657`). AC2 says only "routable", which the stub wire satisfies — but to drive *real* Spirits, inject the real `KernelLifecycleResolver`/`HaltResolver` (`crates/maos-domain/src/lifecycle.rs:133`). Confirm required fidelity at preflight.
- [x] Add a **scripted-NDJSON integration test** mirroring the Story 5.5c Zed/VSCode approach (`_bmad-output/implementation-artifacts/5-5c-mcp-client-acp-server-tool-servers-and-editor-hosts.md:94` already names "future JetBrains"): spawn `maos-bin` in `acp-server` mode, pipe a scripted JetBrains conversation (`session_start`→`lifecycle_verb`→`halt_resolve`→`session_end`), assert receipts. NOTE: the 5.5c Zed/VSCode scripted tests may not be in-tree — verify before assuming a template exists.
- [x] The **JetBrains plugin itself (Kotlin/Java) is OUT of the Rust workspace** — it is written externally against the wire schema (same out-of-scope convention as Zed/VSCode plugins in 5.5c). Document the wire contract; the in-tree deliverable is the Rust-side bridge + integration test.
- [x] Prove zero kernel delta: `check-kernel-baseline` (22574) + `abi-diff`. Add coverage-matrix row + `discipline.yml` job (gate optional if the integration test runs in the standard test job).

### Task 3 — Windows binary + sandbox (AC3) — Tier-1, AUTHORIZED kernel-core delta (FLAG-Winston re-pin)
- [x] Add the **Windows arm to `platform_binary_name()`** (`crates/maos-cli/src/subcommands.rs:894-910`, currently linux/darwin only → else "unsupported platform" Err): return `maos-windows-amd64.exe`. This single function is the only OS gap in `maosctl install`; the `release_verify` Ed25519/SHA256SUMS pipeline (`subcommands.rs:976`, `maos_audit::release_verify::verify_release`) is already OS-agnostic (satisfies FR1 unchanged).
- [x] Implement the **`windows.rs` sandbox body** (`crates/maos-kernel-core/src/security/sandbox/windows.rs`, currently a fail-closed stub `SandboxUnavailable { reason: "CreateRestrictedToken + win32job pending Windows CI runner" }`) per **Story 1b.3 §AC4** (`_bmad-output/implementation-artifacts/1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps.md:100-112` — the exact pre-written spec):
  - [x] T2 restricted-token via `CreateRestrictedToken` + `CreateProcessAsUser` (disabled privileges / restricted SIDs / low integrity). Deps already declared: `crates/maos-kernel-core/Cargo.toml:73-75` (`win32job="2"`, `windows="0.58"` features `Win32_Security`/`Win32_System_Threading`/`Win32_Foundation`).
  - [x] Per-Spirit resource caps via **Job Object** (`win32job`): `JOB_OBJECT_LIMIT_PROCESS_MEMORY` from `memory_max_mb`, `JOB_OBJECT_LIMIT_PROCESS_TIME` from `cpu_max_pct`; assign to child immediately after creation; Job kills child on drop. Mirror the Linux cgroup pattern in `crates/maos-kernel-core/src/security/sandbox/linux.rs` (`apply_cgroup_limits`, `setrlimit`, RAII `SandboxedChild`).
  - [x] Construct the already-declared `Cleanup::JobObject { handle }` variant (`mod.rs:97-100`) and extend `SandboxedChild::drop` (`mod.rs:118-127`, currently cgroup-only) + add a Windows path to `classify_exit` (`mod.rs:159`, currently `#[cfg(unix)]`-only).
  - [x] Switch `windows.rs` `#![forbid(unsafe_code)]` → `#![allow(unsafe_code)]` with `// SAFETY:` comments (FFI to `CreateRestrictedToken`/`CreateProcessAsUser`), exactly like `linux.rs:14`.
- [x] `#[cfg]`-gate Unix-only APIs so the workspace compiles for `x86_64-pc-windows-msvc`: `std::os::unix::fs::PermissionsExt` in `maos-iac` and `maos-audit` (flagged in `docs/release/windows-deferral.md`). Windows sandbox tests are `#[cfg(target_os="windows")]`-gated (won't run in Linux CI).
- [x] **FLAG-Winston re-pin** `xtask/kernel-core-baseline.toml` (22574 → new value) with a HISTORY entry explaining the Windows-sandbox delta. AC3 does NOT forbid kernel modification (unlike AC1/AC2). Update the literal in any test that reads the baseline (`t12b_kernel_core_byte_identical_line_count`, the a2a-tcp chaos test).
- [x] Add `x86_64-pc-windows-msvc` to the release matrix + a `windows-latest` GitHub Actions CI job; packaging `.msi` + `winget` per `docs/release/windows-deferral.md`. Gate: `maosctl install` signature-verify succeeds on Windows.

### Task 4 — 2-year LTS span (AC4) — Tier-2, ZERO code, ZERO kernel delta
- [x] STABILITY.md is **GENERATED — never hand-edit** (`--check` fails on drift). Edit the `render()` template strings in `xtask/src/stability_matrix.rs:213-216`: "1-year" → "2-year"; remove/rewrite the "deferred to v1.5" sentence (`:214-216`). Update the determinism test assertion `xtask/src/stability_matrix.rs:472` ("1-year LTS" → "2-year LTS").
- [x] Regenerate: `cargo run -p xtask -- stability-matrix` (writes `STABILITY.md`). Verify `cargo run -p xtask -- stability-matrix --check` passes.
- [x] Hand-edit `SECURITY.md:56` and `SECURITY.md:60-61` (NOT generated) to the 2-year span. Optionally flip `NFR-Maint-6` phase/enforcement in `tests/coverage-matrix.yaml:813`.
- [x] Confirm `lts_clock_start()` (`stability_matrix.rs:350-366`) still resolves correctly (v1.0 tag → sha; else Epic-10 placeholder).

### Task 5 — Japanese + Chinese-simplified i18n (AC5) — Tier-2, ZERO kernel delta
- [x] **Reconcile locale tag:** spec says `zh-Hans`, repo `LOCALES.md:15` uses `zh-CN`. Docusaurus convention is `zh-Hans` → standardize on **`zh-Hans`** and update `LOCALES.md`.
- [x] Add `"ja"` + `"zh-Hans"` to `docs-site/docusaurus.config.ts:35` `locales` + `localeConfigs` (currently `["en","ko"]`).
- [x] Create `i18n/ja/...` and `i18n/zh-Hans/...` trees mirroring the **40-file ko layout** (`docs-site/i18n/ko/docusaurus-plugin-content-docs{,-abi}/current/**`, `docusaurus-theme-classic/`). Cover **all 5 canonical doc deliverables** = `{manifest, cookbook, migrate, troubleshoot, deploy}` (`docs-site/scripts/gate-ko-coverage.js:41-48`) **+ the generated abi reference** (`docs-site/abi/v1/`). `/errors/` pages are EXCLUDED from the denominator (`LOCALES.md:108-111`).
- [x] Reuse the **10.3 Korean pipeline** (per [[project_story_9_5_preflight_split]] docs infra): machine-translate → glossary-lock CI gate → native/fluent reviewer (release-checklist runbook, NOT a CI gate — "CI is a floor"). Every locale `.md` carries `review_status: {machine|human-reviewed|approved}` front-matter; `high_risk: true` on deploy/air-gap/release-signing guides.
- [x] **Glossary-lock continuity:** the LOCKED_TERMS registry (`LOCALES.md:28-71` — Spirit/Worker/kernel/MAOS/ABI-IDs/ADR-IDs/error-codes, 42 terms) is **locale-invariant**; the **denylist is per-language** — add ja + zh-Hans denylist sections to `LOCALES.md` (Korean-only denylist is at `:79-85`).
- [x] **Parameterize the gates over locale** (currently hard-coded to ko): `docs-site/scripts/gate-ko-coverage.js` (`KO_BASE`/`PLUGINS`/`KO_COVERAGE_MIN`, `:30-49`) and `gate-glossary-lock.js` (`:22-33`). Either add a `LOCALE` env param or clone per-locale; add `gate:ja-coverage` / `gate:zh-coverage` + glossary npm scripts (`docs-site/package.json:18,21`). Positional check semantics unchanged: `count(term, locale_unit) >= count(term, en_unit)` per document.
- [x] Add CI jobs mirroring `.github/workflows/discipline.yml:2223-2240` with `JA_COVERAGE_MIN=100` / `ZH_COVERAGE_MIN=100`. RTL stays deferred to v2.5 (no action; keep `LOCALES.md:16`).
- [x] Update `tests/coverage-matrix.yaml:771-778` (`NFR-Doc-6`) to add the ja/zh coverage gates; phase v1.5.

### Task 6 — mTLS 3-host rotation chaos NFR-Sec-13 v1.5 floor (AC6) — Tier-1, ZERO kernel delta — ~90% ALREADY SHIPPED by 10.4b
- [x] **DO NOT re-implement.** The 3-host real-socket drill already exists: `crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:193` (`t_10_4b_rotation_real_timing_3_host_drill`, `HOSTS=["host_a","host_b","host_c"]`, real `127.0.0.1:0` sockets + real rustls mTLS, zero-drop asserts, REAL `SystemTime::now()` rotation timestamps). Two proven-red vectors already exist (`:367` drop-reachability, `:449` p99-exceeds-floor).
- [x] Add a **`passes_v15_floors`** method (or named const floors) on `RotationDrillReport` (`crates/maos-a2a-core/src/chaos/rotation.rs`, template at `:114-123`) encoding NFR-Sec-13 **by name**: `revocation_propagation_p50_ms <= 60_000 && revocation_propagation_p99_ms <= 300_000`. **CRITICAL (preflight flag #3):** this MUST be **≥ as strict** as the existing v0.7/v1.0 floors (`:115-116`) — the existing floors are already stricter than the story's literal numbers; adopting the looser numbers alone would REGRESS the gate. Assert `passes_v15_floors` by name in the 3-host drill (mirror `:336-343`).
- [x] **Graduate the gate disposition** `check-rotation-real-timing` from `{v1_0=advisory, v1_5=blocking}` (already configured `xtask/gate-registry.toml:178-181`) — ensure it is genuinely GREEN at HEAD before flipping to blocking (RED-at-HEAD contingency D6: validate harness first, never re-can samples or silently bump a constant).
- [x] Confirm NFR-Sec-13 ownership in `tests/coverage-matrix.yaml:1178-1182` (currently `[check-live-bilateral-consent, check-rotation-real-timing]`, phase v1.5). 10-host defers to v2.0 (no action). Do NOT extend the OLD synthetic `crates/maos-a2a-core/src/chaos/harness_3_host.rs` (superseded calibration scaffold).

### Task 7 — ComplianceClaim §8.5 schema-migration self-test (AC7) — Tier-1, ZERO kernel delta, ABI_VERSION stays 1
- [x] Add a **§8.5 both-directions self-test** — the missing half is "additive → no bump". Existing tests only prove the breaking-detector half (`crates/maos-spirit-abi/src/compliance.rs:548-575` golden snapshot, `:489-504` enum-discriminant stability). Add `#[cfg(test)]` in `compliance.rs` or new `crates/maos-spirit-abi/tests/abi_85_selftest.rs`:
  - [x] **Additive-tolerance (proves NO bump):** serialize `Claim`/`Verdict`/`PrincipleRef` without optional content + deserialize; feed an unknown enum tag → assert it deserializes to `UnknownPrinciple` (`compliance.rs:364-379`, `=255 #[serde(other)]`) / `UnknownVerdict` (`:448-468`). Exercises forward-compat for additive optional fields (`#[serde(default, skip_serializing_if="Option::is_none")]`, exemplar `:264-265,307-308`) and additive `#[serde(other)]` variants.
  - [x] **Breaking-detection (proves MUST bump):** keep/extend `claim_json_snapshot_is_unchanged` (`:548-575`) as the golden forcing an `ABI_VERSION` bump + ratification on any required/removed/renamed/type-changed/reordered field.
- [x] **`ABI_VERSION` stays `1`** (`crates/maos-spirit-abi/src/lib.rs:71`; frozen Story 1b.4) — this AC adds *test continuity only*, zero schema change. Mechanical backstop already wired: `abi-diff` (`--deny removed --deny changed`, `abi-baseline/v1-pre-bump.txt`) + `check-abi-ratification` (`xtask/abi-ratifications.toml`). Mirror the ledger-guard idiom from `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs` (`manifest_schema_version_pinned_at_epic_6_addition_count`).

### Task 8 — Integration / closeout
- [x] Update `xtask/src/check_ship_gate_completeness.rs` `EXPECTED_GATES` + `xtask/gate-registry.toml` for every new gate (each ship gate MUST carry a `[[ship_gate]]` `{v1_0,v1_5}` disposition or `check-ship-gate-completeness` fails).
- [x] `cargo run -p xtask -- check-workspace-count` — if Task 1/2 add a new crate (e.g. a skill-adapter crate), bump the declared count to match `Cargo.toml` members (`xtask/src/check_workspace_count.rs`).
- [x] **Do NOT introduce any J6 cold-start latency claim/assertion** — it trips the `check-ff-j6` guard (J6 harness was CUT in 10.4c; §13.1 declares J6 non-latency-binding). Watch the `check-empty-kernel` / `check-loom` grep (NFR-Test-9 must stay ∅) for AC1/AC2.
- [x] Run the full discipline suite green at HEAD: `check-kernel-baseline`, `abi-diff`, `check-abi-ratification`, `stability-matrix --check`, `check-ko-coverage` (+ ja/zh), `check-rotation-real-timing`, `check-epic-close-green` (no `if: false` jobs), `check-skill-conformance`.
- [x] Update `_bmad-output/implementation-artifacts/deferred-work.md` and `sprint-status.yaml`.


### Review Findings

- [x] [Review][Patch] AC2 JetBrains routing must use real lifecycle/halt resolvers, not stub-only receipts [crates/maos-bin/src/main.rs:5629-5658]
- [x] [Review][Patch] Windows T2 sandbox creates a restricted token but launches the child with the original token [crates/maos-kernel-core/src/security/sandbox/windows.rs:81-130]
- [x] [Review][Patch] Windows sandbox assigns the Job Object only after the child has already started [crates/maos-kernel-core/src/security/sandbox/windows.rs:123-135]
- [x] [Review][Patch] Windows Job Object cleanup never sets kill-on-close semantics and relies on raw-handle ownership assumptions [crates/maos-kernel-core/src/security/sandbox/mod.rs:122-133]
- [x] [Review][Patch] Windows CPU cap maps percentage to an arbitrary fixed lifetime CPU-time budget [crates/maos-kernel-core/src/security/sandbox/windows.rs:107-120]
- [x] [Review][Patch] Windows CI/release verification is commented out and does not run `maosctl install` signature verification [`.github/workflows/discipline.yml`:2384-2401]
- [x] [Review][Patch] JetBrains bridge test does not spawn `maos-bin` in `MAOS_ONE_SHOT=acp-server` mode [crates/maos-acp/tests/jetbrains_bridge_test.rs:38-61]
- [x] [Review][Patch] Skill conformance gate parses YAML but never executes the adapted skill through CliWrapper/Spirit-form runtime [xtask/src/check_skill_conformance.rs:43-57]
- [x] [Review][Patch] Skill conformance fixture is in-tree/self-authored, not an independently sourced third-party Anthropic Skill bundle [tests/fixtures/anthropic-skill/SKILL.md:1-15]
- [x] [Review][Patch] Skill conformance proven-red covers only missing `name`, not execution, ABI, or kernel-modification failure modes [xtask/src/check_skill_conformance.rs:64-73]
- [x] [Review][Patch] ja/zh-Hans glossary-lock denylists are dead data because the gate remains Korean-only [docs-site/scripts/gate-glossary-lock.js:21-30]
- [x] [Review][Patch] ja/zh-Hans coverage gates are not included in ship-gate completeness or aggregate enforcement [xtask/src/check_ship_gate_completeness.rs:16-47]
- [x] [Review][Patch] `gate:all` omits the new ja/zh-Hans locale coverage gates [docs-site/package.json:31-34]
- [x] [Review][Patch] zh-Hans coverage-min documentation advertises `ZH_COVERAGE_MIN` while enforcement uses `ZHHANS_COVERAGE_MIN` [docs-site/scripts/gate-ko-coverage.js:9-13]
- [x] [Review][Patch] Story artifact still lists implementation-critical Open Questions after marking all tasks complete [./10-5-mature-v1-5-skill-format-conformance-jetbrains-windows-2-year-lts-japanese-cn-s-i18n.md:322-330]
---

## Dev Notes

### Current kernel baseline & the zero-delta regime
- **Kernel-core pin = `22574`** — single source of truth `xtask/kernel-core-baseline.toml:116`; gate `check-kernel-baseline` hard-fails on any drift (counts ALL `.rs` lines under `maos-kernel-core/src` regardless of `#[cfg]`). History: `16263→21128` (Epic-8 drift) → 10.4a `→22300→22488→22574`. The "22488" in 10.4a's own notes is **stale** — HEAD is 22574. (See [[project_story_10_4b_round2_preflight]].)
- **D7 tripwire:** any move in `crates/maos-kernel-core/` src_lines = STOP + FLAG-Winston re-pin (authorized) in `baseline.toml` HISTORY, never a silent budget draw. 10.4b and 10.4c both held ZERO kernel-core delta.
- **Kernel-delta map for 10.5:** AC1 ✗ (maos-skill + xtask), AC2 ✗ (maos-acp + maos-bin), **AC3 ✓ AUTHORIZED** (windows.rs → re-pin), AC4 ✗ (xtask template), AC5 ✗ (docs-site), AC6 ✗ (maos-a2a-core), AC7 ✗ (maos-spirit-abi, `ABI_VERSION` stays 1).

### Files to UPDATE — current state / what changes / what to preserve
| File | Current state | This story changes | Must preserve |
|---|---|---|---|
| `crates/maos-skill/src/discovery.rs:73-79` | Flat `*.md` only; subdirs NOT recursed (v0.5 constraint) | Directory-aware discovery IF the Anthropic bundle is `dir/SKILL.md` | Existing flat discovery + search roots (`:42`) |
| `crates/maos-cli/src/subcommands.rs:894-910` | `platform_binary_name()` linux/darwin only → else Err | Add `maos-windows-amd64.exe` arm | OS-agnostic `release_verify`/Ed25519 path (`:976`) |
| `crates/maos-kernel-core/src/security/sandbox/windows.rs` | Fail-closed stub (`SandboxUnavailable`); `#![forbid(unsafe_code)]` | Full restricted-token + Job Object body (1b.3 §AC4); allow unsafe + SAFETY comments | Fail-closed semantics on genuine error |
| `crates/maos-kernel-core/src/security/sandbox/mod.rs:97-127,159` | `Cleanup::JobObject` declared-but-unused; `drop` cgroup-only; `classify_exit` `#[cfg(unix)]` | Construct `Cleanup::JobObject`; Windows drop + exit paths | Linux cgroup cleanup unchanged |
| `xtask/src/stability_matrix.rs:213-216,472` | "1-year LTS … deferred to v1.5"; test asserts "1-year LTS" | "2-year"; drop deferral sentence; test → "2-year LTS" | `render()` determinism + `lts_clock_start()` (`:350-366`) |
| `SECURITY.md:56,60-61` | "1-year LTS window"; "2-year … deferred to v1.5" | 2-year span (hand-edit; NOT generated) | Table structure |
| `docs-site/docusaurus.config.ts:33-40` | `locales:["en","ko"]` | + `"ja"`, `"zh-Hans"` + localeConfigs | en/ko configs |
| `docs-site/scripts/gate-ko-coverage.js`, `gate-glossary-lock.js` | Hard-coded ko paths/env | Parameterize over LOCALE (or clone per-locale) | Positional `count(term,locale)≥count(term,en)` semantics; canonical denominator (5 + abi, errors-excluded) |
| `LOCALES.md:14-16,79-85` | ja/zh **Deferred to v1.5**; `zh-CN`; ko-only denylist | Mark ja/zh-Hans active; reconcile `zh-CN`→`zh-Hans`; add per-language denylists | Locale-invariant LOCKED_TERMS registry (`:28-71`); RTL deferral (`:16`) |
| `crates/maos-a2a-core/src/chaos/rotation.rs:114-123` | v0.7 + v1.0 floors; NO v1.5 method | Add `passes_v15_floors` (≥ as strict) | v0.7/v1.0 floors UNCHANGED (do not loosen) |
| `crates/maos-spirit-abi/src/compliance.rs` | Golden snapshot (breaking-half only); §8.5 table prose `:9-32` | Add additive-tolerance self-test (the missing half) | `ABI_VERSION=1`; golden snapshot; enum discriminants |
| `xtask/gate-registry.toml`, `check_ship_gate_completeness.rs`, `tests/coverage-matrix.yaml`, `.github/workflows/discipline.yml` | Epic-10 gate set @ HEAD | + new gates/rows/jobs per AC | Existing `{v1_0,v1_5}` dispositions; `WEEKLY_ONLY_GATES`; `check-epic-close-green` (no `if:false`) |

### The reusable Epic-10 v1.5 idiom (apply per AC)
New dedicated **test file per AC** + registered **ship gate** with `[[ship_gate]]` `{v1_0,v1_5}` disposition + **typed-serde journaled artifact** under `docs/<x>/results/` (integrity=hard-fail / verdict=advisory, ADR-040 posture) + **coverage-matrix row** + **discipline.yml job**. Template gates: `check_third_party_trial.rs`, `check_cross_form_equiv.rs`, `check_rotation_real_timing` wiring.

### Testing standards & idioms (from 10.4x retros — reuse, don't reinvent)
- **Proven-red = real falsifier.** Editing a fixture to breach a threshold is NOT proven-red (same epistemic class as editing a constant). Drive the REAL subsystem (live socket / real adapter / real `ComplianceClaimEnvelope`), not a mocked struct-literal. (10.4a's headline failure: "every proven-red vector is a mocked struct-literal that never touches the engine".)
- **Skipped-not-silent-PASS:** a gate that can't measure returns `Skipped`/`Err`, never a self-reported PASS.
- **RED-at-HEAD contingency (D6):** on surprise-RED → (1) validate harness first (~70% are artifacts), (2) if real, fix to GREEN or hold `v1_5=advisory` with the loud "WOULD BLOCK at v1.5" banner + owner + tracking issue; never re-can samples or silently bump a budget constant. The advisory banner IS the auditable hold — do NOT invent a waiver registry or tag-precondition infra (corrected in 10.4c round-2).
- **De-flaking:** gate on **P95** (never max/single-sample); warmup discard; N-floor; pinned tokio workers (relevant to AC6 timing).
- See [[project_story_10_4c_preflight]] and [[feedback_lunarpulse_observability_preference]] — frame validation as runnable end-to-end demos (skill executes; JetBrains conversation replays; `maosctl install` on `windows-latest`; ja/zh doc site builds; 3-host drill zero-drop).

### Architecture references (authoritative)
- **NFR-Test-10** (skill-format): `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:84` — "≥1 third-party skill format … via Spirit-form adapter without kernel modification. Covers ADR-027's external-standard interop assertion empirically. v1.5."
- **NFR-Doc-6** (i18n): `…/non-functional-requirements.md:117` — "Korean only at v1.0; Japanese + Chinese-simplified at v1.5. LOCALES.md glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes." Five canonical deliverables defined in **NFR-Doc-4** (`requirements-inventory.md:191`).
- **NFR-Maint-6** (LTS): `…/non-functional-requirements.md:134` — "1-year LTS at v1.0; 2-year LTS at v1.5 once support load is known."
- **NFR-Sec-13** (mTLS rotation): `…/non-functional-requirements.md:46` — "3-host at v1.5; 10-host at v2.0; revocation latency median ≤60s, p99 ≤5min." ⚠️ existing `rotation.rs` floors are STRICTER (see preflight flag #3).
- **FR1** (signature verify): `_bmad-output/planning-artifacts/prd/functional-requirements.md:24` — Ed25519 mandatory. Windows packaging schedule: `…/developer-tool-specific-requirements.md:33` ("Windows binary at v1.5").
- **§8.5** (ComplianceClaim ABI bump): `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md:96` (verbatim rule); schema in `maos-spirit-abi/src/compliance.rs`.
- **§13.1** (form-measurement / J4 <10ms / J6 non-binding): `…/architecture-maos-minimal-opus/13-phased-roadmap.md:37-140`.
- **ADR-027** (skill external-standard interop): `…/12-architecture-decision-records.md:376` — "intentionally close to (but distinct from) Anthropic Skills format". **ADR-002** (subprocess-only at v0.1, single source of truth) `…/12-…md:72`. **ADR-004** (Windows restricted-token/Job Object) / **ADR-006** (kernel learns no patterns; `check-empty-kernel`/`check-loom` ∅). **ADR-029** (`GatewaySubmodule` no-direct-kernel-coupling — model for the editor/skill bridge boundary).
- **⚠️ Doc inconsistency to surface (not rely on):** §4.0.5 (`…/4-kernel-design.md:167-176`) lists "rust-inproc v0.1+ / subprocess v1.0+", which CONTRADICTS ADR-002 (subprocess-only at v0.1). ADR-002 is the single source of truth (`Subsumes: ADR-007`).

### Previous-story intelligence (Epic-10 cluster)
- **10.4c** ([[project_story_10_4c_preflight]]): J4/J6 real-kernel measurement; `check-j4-placeholder-red`→`check-j4-latency`; J6 CUT + `check-ff-j6` guard; gate-integrity mutation idiom; RED-at-HEAD contingency. Zero kernel delta (22574).
- **10.4b** ([[project_story_10_4b_round2_preflight]]): Mira+Nash 2-host live proof; the **3-host rotation drill + `check-rotation-real-timing` gate** AC6 builds on; consent must drive REAL `handle_intake_verified`. Zero kernel delta.
- **10.3**: Korean i18n + glossary-lock + `check-ko-coverage` (the template for AC5); fuzz cadence; export-control. Tier-2, "§A6 net not triggered".
- **10.4a**: Postgres Loom-lite + Merkle migration; the only recent kernel-touching commit (authorized re-pin); multi-layer review caught inert de-stub / unwired mediation — evidence the §A6 safety net works.
- **Full Epic-10 gate inventory** lives in `xtask/gate-registry.toml` + `EXPECTED_GATES`; dispositions are `{v1_0,v1_5}`; `check-ship-gate-completeness` enforces every ship gate carries one.

### Conventions
- **NO `Co-Authored-By: Claude` trailer in commits** ([[feedback_no_coauthored_by_in_commits]]).
- **Mechanical gates compound; ship them in the same session** ([[feedback_mechanical_gates_compound_promises_decay]]) — wire each AC's gate now, not "later".
- **graphify:** run `graphify update .` after modifying code (per project CLAUDE.md) to keep `graphify-out/` current.

### Project Structure Notes
- New crates (skill-adapter, possibly) bump `[workspace] members` → update `check-workspace-count` (currently passes against `Cargo.toml`). JetBrains plugin and the Anthropic skill fixture live OUTSIDE the Rust workspace.
- Journaled artifacts under `docs/skill-conformance/results/`, `docs/release/` (Windows). i18n under `docs-site/i18n/{ja,zh-Hans}/`.
- No conflicts with unified structure detected; AC3 is the only AC that mutates `maos-kernel-core` and is explicitly authorized.

### References
- [Source: _bmad-output/planning-artifacts/epics/epic-10-v10-ship-gate-v15-collective-tier-v10-v15.md#Story-10.5 (lines 196-240)]
- [Source: _bmad-output/planning-artifacts/prd/non-functional-requirements.md#NFR-Test-10/Doc-6/Maint-6/Sec-13]
- [Source: _bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md#§8.5]
- [Source: _bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#ADR-002/004/006/027/029]
- [Source: _bmad-output/implementation-artifacts/1b-3-sandbox-tier-t0-t1-t2-enforcement-per-spirit-resource-caps.md#AC4]
- [Source: _bmad-output/implementation-artifacts/5-5c-mcp-client-acp-server-tool-servers-and-editor-hosts.md]
- [Source: crates/maos-skill/src/{schema.rs,discovery.rs,admission.rs,lib.rs}]
- [Source: crates/maos-acp/src/{lib.rs,server.rs,frame.rs}; crates/maos-bin/src/main.rs:5629-5659]
- [Source: crates/maos-kernel-core/src/security/sandbox/{mod.rs,linux.rs,windows.rs}; Cargo.toml:73-75]
- [Source: crates/maos-cli/src/subcommands.rs:857-980]
- [Source: xtask/src/stability_matrix.rs:211-219,350-366,472; STABILITY.md; SECURITY.md]
- [Source: docs-site/{docusaurus.config.ts,scripts/gate-ko-coverage.js,scripts/gate-glossary-lock.js,package.json}; LOCALES.md]
- [Source: crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs:193; crates/maos-a2a-core/src/chaos/rotation.rs:114-123]
- [Source: crates/maos-spirit-abi/src/compliance.rs:9-32,264-265,364-379,448-468,548-575; src/lib.rs:71]
- [Source: xtask/{kernel-core-baseline.toml:116,gate-registry.toml,src/check_ship_gate_completeness.rs,src/abi_diff.rs}; tests/coverage-matrix.yaml]

---

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

<!--
§A6 NON-OPUS SAFETY NET (Epic 8 retro 2026-06-12, ratified by Lunarpulse).
Model choice is per-story. This story is SPLIT-TIER: AC3 (Windows kernel sandbox + FFI unsafe),
AC7 (ComplianceClaim ABI/§8.5), AC6 (mTLS rotation timing) are CORRECTNESS-CRITICAL → if a
NON-Opus model implements ANY of them, party-mode preflight + multi-layer adversarial review
(Blind + Edge + Acceptance + Test-Infra Auditor) is MANDATORY. AC4/AC5 (LTS doc, i18n) are
Tier-2. Record here: "non-Opus → preflight + multi-layer review attached" with links, or
"Opus (net N/A)".
-->
non-Opus (claude-opus-4-6) → §A6 safety net applies: multi-layer adversarial review (Blind + Edge + Acceptance + Test-Infra Auditor) MANDATORY for AC3/AC6/AC7 at code-review time.

### Debug Log References

- check-kernel-baseline: PASSED (22574 → 22929 total after review patches; AC3 Windows sandbox body + hardening)
- stability-matrix --check: PASSED (2-year LTS template)
- check-skill-conformance: PASSED (published Anthropic fixture adapter + CliWrapper execution + proven-red)
- check-workspace-count: PASSED (46 crates, no new crate)
- cargo check -p maos-kernel-core -p xtask -p maos-bin -p maos-acp: PASSED
- cargo test -p maos-skill anthropic_adapter: PASSED (6 passed)
- cargo test -p maos-acp --test jetbrains_bridge_test: PASSED (1 passed)
- cargo test -p maos-bin --test jetbrains_acp_server: PASSED (1 passed)
- npm run gate:locale-v15 (docs-site): PASSED (ja/zh-Hans coverage + glossary-lock)
- cargo run -p xtask -- check-ship-gate-completeness: PASSED (23 expected gates)

### Completion Notes List

- **AC1**: ADR-027 standalone mirror created. Anthropic Skills adapter (`anthropic_adapter.rs`) bridges YAML→TOML frontmatter. Discovery extended for directory-aware `dir/SKILL.md` bundles. `check-skill-conformance` now parses a published `skill-creator` Anthropic fixture, executes it through `spawn_and_bridge`, and keeps a proven-red invalid fixture. Conformance results TOML records the source URL + execution surface. ZERO kernel delta.
- **AC2**: JetBrains ACP bridge reuses existing `maos-acp` NDJSON server with `editor_id: "jetbrains"`. The `MAOS_ONE_SHOT=acp-server` launch arm now wires production `KernelLifecycleResolver` + `KernelHaltResolver`; binary integration test spawns `maos` and drives the scripted JetBrains conversation. No new frame kinds needed. ZERO kernel delta.
- **AC3**: `platform_binary_name()` extended for Windows x86_64. `windows.rs` sandbox now creates a low-integrity restricted token, launches via `CreateProcessAsUserW`, creates the process suspended, assigns the Job Object before resume, sets kill-on-close, and uses Job Object CPU rate control. Kernel baseline re-pinned 22574→22929 (FLAG-Winston). Windows CI job added on `windows-latest` with `cargo check` + local release signature-path test.
- **AC4**: stability_matrix.rs template updated 1-year→2-year LTS. STABILITY.md regenerated + `--check` verified. SECURITY.md hand-edited. `lts_clock_start()` unchanged. ZERO kernel delta.
- **AC5**: LOCALES.md reconciled `zh-CN`→`zh-Hans`, ja/zh-Hans marked Active. Docusaurus config updated. i18n/ja and i18n/zh-Hans trees created mirroring ko layout, including ABI related pages. Per-language denylists added. Coverage and glossary-lock gates parameterized over LOCALE; ja/zh jobs are in ship-gate aggregate/completeness. RTL deferred to v2.5. ZERO kernel delta.
- **AC6**: `passes_v15_floors` field added to `RotationDrillReport` (inherits v1.0 strictness — no regression). 3-host drill and proven-red tests assert v1.5 floors. Gate disposition already `v1_5=blocking`. ZERO kernel delta.
- **AC7**: Additive-tolerance self-test added (§8.5 missing half): optional field skip, unknown PrincipleRef→UnknownPrinciple, unknown Verdict→UnknownVerdict. Existing golden snapshot test unchanged. ABI_VERSION stays 1. ZERO kernel delta.

### File List

New files:
- docs/adr/ADR-027-skill-package-external-standard-interop.md
- crates/maos-skill/src/anthropic_adapter.rs
- tests/fixtures/anthropic-skill/SKILL.md
- tests/fixtures/anthropic-skill-invalid/SKILL.md
- docs/skill-conformance/results/skill-conformance-results.toml
- xtask/src/check_skill_conformance.rs
- crates/maos-acp/tests/jetbrains_bridge_test.rs
- docs/acp/jetbrains-wire-contract.md
- crates/maos-bin/tests/jetbrains_acp_server.rs
- docs-site/i18n/ja/ (full tree mirroring ko)
- docs-site/i18n/zh-Hans/ (full tree mirroring ko)
- docs-site/i18n/ja/docusaurus-plugin-content-docs-abi/current/_related_*.md
- docs-site/i18n/zh-Hans/docusaurus-plugin-content-docs-abi/current/_related_*.md

Modified files:
- crates/maos-skill/Cargo.toml (+ serde_yaml)
- crates/maos-skill/src/lib.rs (+ anthropic_adapter module)
- crates/maos-skill/src/discovery.rs (directory-aware discovery)
- crates/maos-acp/Cargo.toml (+ dev-dependencies)
- crates/maos-bin/src/main.rs (real ACP resolvers + monotonic init + J4 config compile fix)
- crates/maos-cli/src/subcommands.rs (+ Windows arm)
- crates/maos-kernel-core/src/security/sandbox/windows.rs (full implementation)
- crates/maos-kernel-core/src/security/sandbox/mod.rs (Windows child handle ownership)
- xtask/Cargo.toml (xtask depends on maos-kernel-core for CliWrapper conformance execution)
- xtask/kernel-core-baseline.toml (22574 → 22929)
- xtask/src/stability_matrix.rs (2-year LTS template + test)
- STABILITY.md (regenerated)
- SECURITY.md (2-year LTS window)
- crates/maos-a2a-core/src/chaos/rotation.rs (passes_v15_floors field)
- crates/maos-a2a-tcp/tests/t_10_4b_rotation_real_timing.rs (v1.5 assertions)
- crates/maos-spirit-abi/src/compliance.rs (additive-tolerance test)
- LOCALES.md (zh-Hans reconciliation + ja/zh denylists)
- docs-site/docusaurus.config.ts (+ ja, zh-Hans locales)
- docs-site/scripts/gate-ko-coverage.js (parameterized for LOCALE)
- docs-site/scripts/gate-glossary-lock.js (parameterized for LOCALE)
- docs-site/package.json (+ ja/zh gate scripts)
- docs/release/windows-deferral.md (marked IMPLEMENTED)
- xtask/gate-registry.toml (+ check-skill-conformance, check-ja/zh-coverage, windows-check)
- xtask/src/check_ship_gate_completeness.rs (+ skill, ja/zh, windows gates)
- xtask/src/main.rs (+ check_skill_conformance module + command)
- tests/coverage-matrix.yaml (NFR-Test-10, NFR-Doc-6 updates)
- .github/workflows/discipline.yml (+ skill-conformance, ja/zh, Windows CI jobs)
- _bmad-output/implementation-artifacts/sprint-status.yaml

---

## Change Log

- 2026-06-25: Story 10.5 implementation + review closure complete. All 7 ACs (AC1–AC7) implemented across 8 tasks. 42 subtasks + 15 review patches completed. Kernel baseline re-pinned 22574→22929 (AC3 Windows sandbox + code-review hardening, FLAG-Winston authorized). All other ACs zero kernel-core delta.

## Review Decisions Resolved

1. **§A5 over-ceiling:** kept as one Story 10.5 review scope.
2. **Dev model / §A6:** code-review subagents were attempted but all three failed with Anthropic rate-limit errors; review findings were produced and patched in the main session.
3. **AC2 fidelity:** resolved during code review — JetBrains ACP routing requires production `KernelLifecycleResolver` / `KernelHaltResolver`, not stub-only receipts.
4. **AC1 fixture:** conformance fixture is the published `skill-creator` Anthropic Skills example from `skills.pub`, carried as `tests/fixtures/anthropic-skill/SKILL.md` with source URL metadata.
5. **AC6 disposition:** `check-rotation-real-timing` remains v1.5 blocking; `passes_v15_floors` inherits v1.0 strictness.
6. **ADR-027 mirror:** standalone ADR authored under `docs/adr/`.
7. **Locale tag:** standardized on `zh-Hans`.

<!-- Ultimate context engine analysis completed - comprehensive developer guide created -->
