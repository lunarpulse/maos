# Preflight — Epics 15–20 (Product-Spine Recovery Lane), 2026-09-04

> **Superseded the same day by Round 2** (`sprint-change-proposal-2026-09-04-round2.md`, input package `correct-course-input-2026-09-04-confidence.md`): Fork C ratified, Epics 15–21 rewritten under the eight confidence rules. Story keys below are the morning's; the mapping is in the R2 proposal §4.2. This file stays as the evidence record.

> **Question (Lunarpulse):** after the course correction, can we confidently and safely achieve what we promised?
> **Method:** six read-only scouts, one per epic, each tasked to *disprove* its epic's AC premises against committed `HEAD d09420e2` (`git show HEAD:`), run one at a time. Their full reports are in the session record; this file keeps the verdicts, the false premises with corrected wording, and the cross-cutting findings that change the plan's shape.
> **Verdict: NOT as written.** The direction is right and every seam is real, but the six epics rest on ~35 premises that are false at HEAD, their exit commands name flags and verbs that do not exist, and the honest duration is **40–58 weeks, not 17–24**. Two findings restructure the plan (§2.1, §2.2). No story should leave `backlog` until §4 is applied.

## 1. Per-epic verdict

| Epic | Confidence the exit command works as written | Duration as written → measured | The premise that breaks it |
|---|---|---|---|
| 15 · W0 | **35** | 1 wk → 1–2 wk (+ governance rounds) | `release.yml` has a first-run defect (`download-artifact@v4` without `merge-multiple` → `sha256sum maos-*` "Is a directory"); two secrets do not exist; aarch64 targets have never compiled in CI; the I9 fix needs kernel lines; 16 (not 9) `other` symbols plus 19 "removed public symbol" rows; **8** `CURRENT_PHASE` constants, not 3 |
| 16 · W1 | **25** | 2–3 wk → 6–9 wk | **Every bundled Spirit is an in-process library call**; there is no T2 subprocess Spirit to sandbox. Operator surface is GET-only and unbound by default. `maosctl pause hello-spirit` stops nothing observable in shell mode. 16-6's audit-drop repair is kernel-core, not out-of-kernel |
| 17 · W2 | **10** | 3–4 wk → 8–14 wk | **No generic subprocess Spirit host exists** (`SpiritForm` = NativeSubprocess for CliWrapper only, WasmComponent; `sdks/spirit-ts` is "a TEST HARNESS, not a kernel runtime"). Butler has no inference seam; `maos eval` verb absent; the "three-week self-tuning" loop and its clock do not exist; MCP client cannot authenticate to Google |
| 18 · W3 | **8** | 3–4 wk → 8–12 wk | `demo-j1 --live` is not a flag; no Orchestrator dispatch loop exists (the composition root says so at `main.rs:3434-3440`); `MAOS_WORKER_TASK` was already removed (the env shortcut is `MAOS_DELEGATED_GOAL`); two of three ABSENT beats need paid, two-host, human-signed runs; real agent CLIs cannot run under T3 (`--network=none`) |
| 19 · W4 | **15** | 4–6 wk → 9–11 wk + human calendar | The release ships no `maosctl` (brew formula post-install calls it); `install <name>` fetch was deliberately removed at v0.5; `maos-spirit publish` refuses every non-stub URI; committing an honest N=3 trial file **reds** a currently-green gate; 23 (not 12) gates pass on absent evidence; the fuzz floor cannot promote inside any epic (90-day ledger span) |
| 20 · W5 | **35** | 4–6 wk → 8–10 wk | `demo-reza --live` will not parse (and red-on-absence is *already* true); the "50-scenario corpus" is the 300-line κ corpus with no reader and no driver into Mira; "19 canonical / 10 SigningKey duplicates" is wrong (17 fns in different domains, 6 keys with distinct semantics); moving `orchestrator/` is a kernel edit; 20-3 and 20-4 contradict on `maos-persistence` |

Sum of measured durations: **40–58 weeks** for 1–2 engineers plus agents, against 17–24 planned.

## 2. Cross-cutting findings (these change the plan's shape)

### 2.1 The v0.1 subprocess Spirit form was never built — and five stories assume it
All ten reference Spirits declare `forms = ["rust-inproc"]` and run as library calls inside the daemon with builder-injected ports. The only subprocess in production is the CliWrapper Worker (T3-forced, `--network=none`), plus the WASM host. There is no Spirit Wire Protocol dispatch (14 hooks, request/response ops such as `log.recall`), no SDK transport, no non-CLI launch path. This invalidates 16-2 (nothing to sandbox), 17-4 (wire recall), 18-2 (sandboxed real Worker), 19-1 (a runnable Python Spirit) and collides with 18-1 (a subprocess Orchestrator cannot take a builder-injected port). It is the largest single item of unbuilt promise in the PRD (v0.1 row: "single-Spirit subprocess form, Spirit Wire Protocol over JSON-RPC").

**Fork A/B for the operator:**
- **A. Build it** — a new epic "Subprocess Spirit host (Spirit Wire Protocol v1)": hook dispatch over NDJSON/CBOR, `log.recall`/`log.fetch` request ops with token scoping, TS SDK transport, launch path, re-host the ten Spirits. 4–8 weeks. Sequenced *before* sandbox, real-Worker and TS/Python work. Total lane ≈ 44–66 weeks.
- **B. Ratify in-process as the v1 Spirit form** — sandbox applies to Workers only (FR5 narrowed by ADR, honestly), TS/Python become conformance harnesses (FR33 narrowed), `rust-inproc` retirement reversed in the roadmap. Total lane ≈ 28–38 weeks. Faster to an operator-visible product, smaller promise.

### 2.2 The kloc instrument now blocks product work
Eight crates sit at exactly zero headroom (`maos-kernel-core` 18933, `maos-bin` 17109, `maos-a2a-core` 4856, `maos-cohort` 5857, `maos-iac` 6960, `maos-audit` 6847, `maos-cli` 5270, `xtask` 41953), `maos-domain` is already red, `check-kernel-baseline` hard-fails on *any* delta from 24472, and `_aggregate_hardfail` is recalculated only at an epic retro (`kloc.toml:501`). Every story in 16–20 adds lines to at least two of these crates. Per-story FLAG-Winston grants would stall every story. **W0 must include a one-time, operator-ratified ceiling re-base** (measured, with the retro-only rule amended), or the lane cannot move.

### 2.3 Sandbox and vendor egress are in structural tension (FR5 vs FR47)
A real `claude`/`codex` Worker needs vendor egress, a writable repo cwd and a credential; T2 seccomp has no `socket/connect`, T3 is `--network=none --read-only`, and `credential_env_var()` has zero consumers (the child inherits the parent env). No AC in 16-2/18-2 can be true until an ADR chooses: kernel-proxied inference for CliWrapper (FR47-pure, big) or an egress allowlist plus kernel-minted scoped credential (pragmatic). This is a design decision, not a story task.

### 2.4 Live model calls have no hermetic seam outside Researcher
`MAOS_REPLAY_CASSETTE` is wired for Researcher only; Butler's cassette is `hand-authored-seed`; CI holds the provider key only in the nightly `tier-2-rerecord` leg; the nightly passes `--live` to nextest, an invalid flag (latent red). 17-1, 17-2, 18-2 and 20-1 all need live completions. A generic record/replay seam at `InferencePortAdapter` (maos-bin) is a prerequisite story, not an assumption.

### 2.5 Exit commands must name what a machine can run
Six commands reference absent flags or verbs (`demo-j1 --live`, `demo-reza --live`, `maos eval halt`, `maos uninstall`, `--epic`, `--deterministic`/`--replay`) or things no single command on a clean machine can do (a paid two-host human-signed run, a three-week self-tuning window, a ≤90-minute incident close, a Homebrew tap that needs an external repo). Rewrite each to an existing or newly defined verb; move paid/external/human legs into labelled operator-lane rows.

### 2.6 External provisioning engineering cannot guarantee (operator checklist, W0)
GitHub secrets `RELEASE_SIGNING_KEY` and `MAOS_RELEASE_PUBKEY` (the guardrail passes vacuously when unset); a macOS/aarch64 build that has never run; `lunarpulse/homebrew-maos` repo + token; AUR account; `fuzz-ledger`/`rto-ledger` branches with write permission; a self-hosted runner or chained jobs for a 24-h fuzz (6-h hosted cap); ≥3 external authors; a pen-test firm with dates; Google Calendar MCP OAuth; Gemini/Bedrock/Vertex credentials.

### 2.7 Gate-honesty premises were undercounted
23 gates pass on absent evidence (not 12); `EvidenceState` exists in 7; `check-third-party-trial` FAILS on `participants_total < 12` so an honest small cohort reds a green gate; the fuzz floor is advisory until the ledger spans 90 days; `sign-and-publish` needs only `build`, so a release can publish over a red aggregate.

### 2.8 Latent defects surfaced by the preflight itself
`release.yml` artifact layout bug (§1); `journey-nightly.yml:94` invalid `--live`; 14-2d's stated blocker is stale (substrate is done; the real blocker is the T_grace raise); 20-3 names `maos-persistence` as the durable pin home while 20-4 deletes it; the `t_14_2a` red's root cause is unmeasured (the test already uses a per-test subscriber; its gate runs it under `--test-threads=1`).

## 3. Safety (can we do this without breaking what is green?)
Green surfaces at risk, by epic: 15 — `check-kernel-baseline`, `check-stability-matrix` (version bump), kloc on maos-bin/kernel-core, the ADR-037 I9 whitelist rule. 16 — 13 test sites that spawn `MAOS_ONE_SHOT`, `cohort_daemon_smoke_13_5c` (`SCANNED_SOURCE_FILES 17==17`), `journey_j0` banner, seccomp unit tests if the allow-list opens; **security:** a mutating operator surface is the "second trust path" `maos-control/src/lib.rs:68-76` refuses by design → needs an ADR and §A6 non-degradable. 17 — Butler/Researcher corpus and journey tests, `distillate_five_metrics_floor`, every gate that keys on `forms`. 18 — `journey_j1` ×3, `worker_completion_2a` (45), `smoke_cli_wrapper_8_12` (inverted by "real by default"), 48 proven-red vectors pinning the topology-loop shape. 19 — `check-third-party-trial`, ledger-fed gates if flipped before ledgers exist, `invariant-lock` + coverage completeness, `check-workspace-count` (a proc-macro crate). 20 — one of the two 14-2a rotation legs reds on the T_grace raise; `check-cohort-mesh` re-runs the pin on any orchestrator move; `journey_butler.rs:285` blocks "zero ignored journey tests".

## 4. What to change before any story leaves `backlog`

1. **Decide fork A/B (§2.1).** This decides whether the lane has six or seven epics and whether FR5/FR33 are narrowed by ADR.
2. **Re-base the kloc ceilings once, in W0**, operator-ratified and measured; amend the retro-only aggregate rule. Without this, 16–20 cannot land a line.
3. **Author three ADRs before 16/18:** Worker egress + credential model (FR47); the mutating operator surface trust path; the Spirit-form decision from step 1. (Fourth, before 20-3: revocation model.)
4. **Rewrite all six exit commands and the ~35 flagged ACs** using the corrected wording in the scout reports; split paid, external and human legs into operator-lane rows with their own keys.
5. **Add to W0:** `release.yml` repair + `maosctl` as a release artifact; the nightly `--live` flag fix; the operator provisioning checklist (§2.6) as a story whose AC is "each item present or explicitly waived"; a generic inference record/replay seam (§2.4) as a story.
6. **Fix the inventory claims** in the epic files: 16 `other` symbols; 8 phase constants; 23 absent-pass gates; 17 canonical fns in distinct domains (no consolidation); 6 signing helpers; the corpus is 300 κ rows, not 50 incidents.
7. **Re-size honestly** after 1–6: under fork A ≈ 44–66 weeks, under fork B ≈ 28–38 weeks. Either is defensible; 17–24 is not.

## 5. What is confirmed true (so the plan is not thrown away)
Every seam S1–S5 holds at HEAD. The registry server, the loom substrate action for three team DBs, `demo-reza`'s red-on-absence, `swap_serving_cert`'s in-place resolver swap, the 30-row Butler corpus, `fire_on_schedule`'s caller, `keyring`'s licensability, hello-spirit's existing `Ambiguous` return, and the sync-`pub` kernel APIs that 16-1 can call without a kernel edit are all real. The plan's *order* survives the preflight; its *premises* and *sizing* do not.
