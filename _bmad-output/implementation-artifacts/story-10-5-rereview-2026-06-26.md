# Story 10.5 — Clean Full-Layer Re-Review (§A2) — 2026-06-26

**Trigger:** Epic 10 retro action item §A2. The original 10.5 review was degraded — the multi-layer adversarial subagents hit Anthropic rate-limits and were replaced by main-session self-review on a 7-AC story carrying **+355 LOC of unsafe Windows FFI** in the kernel.
**Method:** 4 independent adversarial layers per §A6 — Blind Hunter, Edge Case Hunter, Acceptance Auditor, Test-Infra Auditor — over the uncommitted `epic10` working tree (209 files, +12.3k LOC).
**Verdict:** 🔴 **NO-GO for v1.5 release. Reopen 10.5 for rework.** Two ACs (AC3, AC5) are **not actually met**. The degraded original review missed a non-compiling kernel deliverable and a fabricated i18n deliverable.

> The §A6 safety net is now empirically justified: the layers that the original review skipped are exactly the ones that caught the release-blockers. Two of the blockers were found independently by **multiple** layers.

---

## Release-blockers (must clear before any v1.5 release claim)

| ID | Finding | Found by | Fix |
|----|---------|----------|-----|
| **R1** | **Windows sandbox does not compile.** `windows.rs:21-25` imports `Win32::System::JobObjects::{...}` but `crates/maos-kernel-core/Cargo.toml:75` doesn't enable the `Win32_System_JobObjects` feature; not rescued by unification (win32job uses windows 0.61, kernel uses 0.58 — two separate lockfile entries). Plus `windows.rs:29` imports non-existent `SidAndAttributesHash`. **AC3's "ships" claim is false.** | Blind + Edge (independent) | Add `"Win32_System_JobObjects"` to the feature list (consider bumping windows 0.58→0.61 to match win32job and dedupe); delete the `SidAndAttributesHash` import. **Verify with `cargo build --target x86_64-pc-windows-msvc -p maos-kernel-core`.** |
| **R2** | **AC5 ja/zh-Hans trees contain Korean text, not Japanese/Chinese.** Quantified: 35/40 ja files byte-identical to ko; 35/40 zh byte-identical to ko; the rest differ only by a `high_risk:` frontmatter line. Coverage gate (file-presence) + glossary-lock (Latin locked-term preservation; JA/ZH denylists list kana/kanji that can't appear in Korean) are **tautologically green**. `LOCALES.md` independently asserts ja/zh "Active — Story 10.5 AC5" — corroborating the mislabel. | Acceptance (quantified) | Produce real ja/zh translations. **Add a language-identity gate dimension** — the existing coverage/glossary gates provably cannot detect wrong-language content. |
| **R3** | **AC3 tripwire violation.** Kernel-core diff touches 6 files outside the authorized `windows.rs`/sandbox surface — `memory/{mod,principal,read_entry_point,write_entry_point}.rs`, `security/{mod,operator_config}.rs` — pure `cargo fmt` churn, behavior-preserving, but **undisclosed** in the FLAG-Winston re-pin HISTORY (which attributes the whole 22574→22929 delta to windows.rs). Silent budget draw the D7 tripwire exists to catch. | Acceptance | Revert the fmt churn in those 6 files (caution: workspace-wide `cargo fmt --check`), **or** disclose them in `kernel-core-baseline.toml` HISTORY with Winston sign-off. |
| **R4** | **Unsafe Windows FFI is never runtime-executed in CI.** `windows-check` runs only `cargo check -p maos-kernel-core` (compile-only) + an unrelated signature test. Sandbox spawn/token/job code is `#[cfg(windows)]` and only type-checked. A handle leak, wrong SID buffer, an integrity label that doesn't apply, or a Job cap that doesn't cap would all pass green. | Test-Infra + Acceptance | Add `crates/maos-kernel-core/tests/sandbox_enforcement_windows.rs` (`#![cfg(windows)]`): positive spawn (`cmd /C exit 0`) + **negative control** (child exceeding `memory_max_mb` is Job-killed, or integrity reads Low). Run it in `windows-check` (not just `cargo check`). |
| **R5** | **JetBrains AC2 tests wired into no CI job at all.** No `cargo test -p maos-acp` or `-p maos-bin` in any of the 11 workflow files. The ACP bridge has zero CI enforcement. | Test-Infra | Add `cargo test -p maos-acp` and `cargo test -p maos-bin --test jetbrains_acp_server`; add both to the ship-gate `needs:`. |

## High

| ID | Finding | Fix |
|----|---------|-----|
| **H1** | Windows memory cap is **working-set, not a hard commit ceiling** (`limit_working_memory(0, bytes)` = `JOB_OBJECT_LIMIT_WORKINGSET`). Spec 1b.3 wanted `JOB_OBJECT_LIMIT_PROCESS_MEMORY`. Worse, `min=0` likely makes `SetInformationJobObject` return `ERROR_INVALID_PARAMETER`, so **every memory-capped spawn errors out**. Found by Blind + Edge + Acceptance. | Set `ProcessMemoryLimit`/`JobMemoryLimit` via raw `SetInformationJobObject(JobObjectExtendedLimitInformation, …)`; if keeping a working-set limit, use a nonzero min. |
| **H2** | `check-skill-conformance` has **no external proven-red vector** — the proven-red test files got rustfmt-only changes; all journaled-results failure branches untested. Inverting the pass condition would stay green. | Add `xtask/tests/story_10_5_proven_red.rs` (false booleans → fail, malformed TOML → fail, corrupt fixture → fail, absent results → advisory pass; mirror `story_10_2_proven_red.rs`). |

## Medium

- **M1** Windows spawn drops env vars + stdio (`bInheritHandles=FALSE`, no `STARTF_USESTDHANDLES`, `get_envs()` unused) → divergent cross-platform contract; latent until Windows spawn is wired into the runtime.
- **M2** CJK/non-ASCII skill names rejected (`anthropic_adapter.rs:89-93` keeps `is_alphanumeric()` CJK; `schema.rs` requires ASCII id) — ironic for the i18n story. Transliterate/slugify before id derivation.
- **M3** No `SKILL.md` size limit → untrusted third-party DoS (multi-GB / YAML-anchor bomb). Cap file size before read in both discovery arms.
- **M4** id-collision between `dir/SKILL.md` bundle and flat `*.md` (last-wins in `state_by_id`, no warn) → admission shadowing. Dedup + warn on collision.
- **M5** Binary JetBrains test can't distinguish real vs mock resolver (asserts only `kind` + a never-printed `StubLifecycleResolver` string). Assert the `NotLoaded` error-receipt payload for an unloaded spirit.
- **M6** Conformance gate trusts self-reported booleans (`executed_without_kernel_modification`, `abi_unchanged`) in the results TOML; cross-check against the real `abi-diff`/`check-kernel-baseline` artifacts or drop them.

## Low
`STILL_ACTIVE`(259) exit-code ambiguity; SID buffer alignment (ARM64 latent); `cpu_max_pct==0` → unlimited CPU; masked skip-reason on adapter fallback; symlink-following in bundle discovery; `maos-skill`/`maos-acp`/`maos-bin` have no per-crate CI test jobs.

## Process finding (the §A6 proof)
The mandatory multi-layer review **did not actually run** on 10.5 (rate-limited → main-session self-review), and that self-review shipped a **fabricated AC5 deliverable** plus a non-compiling AC3. This is the concrete evidence behind retro §A6: *multi-layer review (incl. Test-Infra) is non-negotiable below opus-4-8 or whenever review subagents fail.*

## Secondary (not blockers)
- New gates labeled `disposition.v1_0 = "advisory"` but the `v1-0-ship-gate` aggregate fails on any `needs.*` failure/skip → **de-facto blocking** (stricter than labeled — acceptable, but document it).
- The human-readable "Ship gate results summary" omits rows for the 4 new gates (ja/zh/windows/skill-conformance) though the fail-check includes them — a reviewer scanning the table won't see their status.

## Verified clean (no action)
Handle lifecycle (no double-close; `into_handle()` `ManuallyDrop`); spawn ordering fail-closed (`CREATE_SUSPENDED` → assign-to-job → `ResumeThread`, no TOCTOU); CPU-rate math; Windows arg quoting; `mod.rs` `classify_exit`; **AC2** real-resolver wiring in `main.rs` (Stub resolvers removed); **AC4** 2-year LTS render; **AC6** rotation floors (`passes_v15_floors = passes_v10_floors`, non-regressing, asserted by name); **AC7** ComplianceClaim additive-serde round-trip; `check_migration_merkle`/`check_red_team_gate`/`check_rto_gate` all genuinely derive-and-reconcile.

---

## Rework checklist (10.5 → reopen)

- [ ] **R1** Fix Windows build; prove with `cargo build --target x86_64-pc-windows-msvc -p maos-kernel-core`.
- [ ] **R2** Real ja/zh-Hans translations + language-identity gate dimension; update `LOCALES.md` only when true.
- [ ] **R3** Resolve kernel-core fmt churn (revert or disclose in HISTORY w/ Winston).
- [ ] **R4** Add `sandbox_enforcement_windows.rs` + run it in `windows-check`.
- [ ] **R5** Wire `maos-acp` / `maos-bin` JetBrains tests into CI + ship-gate `needs`.
- [ ] **H1** Switch to process/job memory limit (hard cap); fix `min=0`.
- [ ] **H2** Add `story_10_5_proven_red.rs`.
- [ ] Triage M1–M6 + Low (M2/M3 recommended before third-party skill exposure).

**On completion:** re-run this 4-layer review clean, then 10.5 returns to `done` and release-hold §A2 clears.

---

## Update 2026-06-26 — R1 isolated, deepened, and import-layer fix applied + proven

Triggered by Lunarpulse running `cargo build --target x86_64-pc-windows-msvc` (failed "at the maos directory").

- **Why the full kernel cross-build fails on this host (not yet an R1 signal):** the Manjaro host has no Windows target std (now installed) and, more fundamentally, maos-kernel-core's dep tree pulls C-FFI crates (`ring`, `libsqlite3-sys`) whose build scripts need a Windows C cross-compiler. `cargo check --target x86_64-pc-windows-msvc -p maos-kernel-core` dies on those build scripts **before** reaching `windows.rs`. **Full AC3 compile verification therefore requires a real Windows runner — which is finding R4.**
- **R1 isolated via a minimal repro** (`windows`/`win32job` only — pure-Rust, no C). The repro reproduced R1 with a real `E0432` and found a **third** import fault the review missed:
  1. `Win32_System_JobObjects` feature missing (module gated).
  2. `SidAndAttributesHash` — nonexistent symbol, **unused** → delete.
  3. **`OpenProcessToken` imported from the wrong module** — it lives in `Win32::System::Threading` (gated by `Win32_Security`, which is on), not `Win32::Security`; it **is** used at `windows.rs:195`, so it must move, not delete.
- **Fix applied + proven** (this session): `Cargo.toml` adds `Win32_System_JobObjects`; `windows.rs` Security block drops `OpenProcessToken`+`SidAndAttributesHash`; Threading block gains `OpenProcessToken`. Isolated repro `cargo check --target x86_64-pc-windows-msvc` → **Finished, 0 errors**. Real-file net line delta **6/6 = 0** (tripwire safe, no re-pin); Linux build still green.
- **R1 remaining (NOT closed):** the repro proves only that the **import layer resolves**. The 337-line unsafe body (incl. **H1** memory-cap and the win32job-0.61 ⟷ windows-0.58 HANDLE interop) is **unverified** — it can only be compile-proven on a Windows runner. So R1 is "import-layer fixed+proven; full-file compile + runtime test (R4) still open." Do the rest as 10.5 rework on Windows CI.

## Update 2026-06-26 (b) — Windows CI stood up (R1-build / R4 / R5 now ENFORCED, not yet green)

Upgraded the `windows-check` job in `.github/workflows/discipline.yml` (already in the `v1-0-ship-gate` aggregate) from compile-only to real build + run:
- **R1/AC3:** `cargo build -p maos-kernel-core -p maos-cli` (real compile on `windows-latest` — the only place the unsafe sandbox FFI compiles for its target; the dev host can't cross-build it past ring/libsqlite3-sys C build scripts).
- **R4:** new test `crates/maos-kernel-core/tests/sandbox_enforcement_windows.rs` (`#![cfg(windows)]`) — positive control (`cmd /C exit 0` spawns + exits 0 through restricted-token→job→resume), exit-code preservation, and a memory-capped smoke that doubles as the **H1 falsifier** (working-set min=0 → `ERROR_INVALID_PARAMETER` would turn it RED). Run via `cargo test -p maos-kernel-core --test sandbox_enforcement_windows`.
- **R5:** `cargo test -p maos-acp --test jetbrains_bridge_test` + `cargo test -p maos-bin --test jetbrains_acp_server` added to the job.

**Expectation:** this gate will go **RED** on the first Actions run until the 10.5 rework lands (it will surface remaining R1-body / H1, and any R5 cross-platform gaps). That red is **correct and intended** — the gate now enforces AC3 instead of asserting it. Verification requires a push to trigger GitHub Actions (cannot be observed from the Linux dev host). Locally validated: YAML well-formed, job wired into aggregate, Linux build unaffected; the test mirrors the Linux sandbox tests' exact API.
