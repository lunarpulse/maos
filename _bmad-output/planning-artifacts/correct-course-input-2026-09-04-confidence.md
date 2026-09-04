# Correct-Course Input Package — Raising the Confidence of Epics 15–20

> **Status:** input package for `bmad-correct-course` (Round 2 of 2026-09-04). **Trigger:** `epics/epic-15-20-preflight-2026-09-04.md` — confidence 8–35 per epic, 40–58 measured weeks vs 17–24 planned, ~35 false premises.
> **Author:** Claude (session), for Lunarpulse. **Repo state:** HEAD `d09420e2`; no code touched; Epics 15–20 marked NEEDS-REWORK.

## 1. Why confidence was low — the four failure shapes, named once

The preflight did not find bad engineering. It found four shapes in the *plan*, and every low score traces to one of them:

| Shape | What it looked like | Share of the ~35 false premises |
|---|---|---|
| **Verb without substrate** | exit commands and ACs naming flags/verbs that do not exist (`demo-j1 --live`, `maos eval`, `maos uninstall`, `--epic`, `--replay`) | ~10 |
| **Assumed foundation** | a runtime, seam or mechanism assumed present (subprocess Spirit host, inference cassette seam, Orchestrator dispatch loop, mutating operator surface, credential injection) | ~12 |
| **Human, money or clock inside an engineering exit** | paid two-host runs, external authors, a pen-test firm, a Homebrew tap, "three weeks of declining acceptance", "≤90 minutes" | ~7 |
| **Inventory by recollection** | counts taken from memory instead of the tree (9 vs 16 symbols, 3 vs 8 phase constants, 12 vs 23 absent-pass gates, 19 "duplicate" canonical fns that are different domains, a 50-scenario corpus that is 300 κ rows) | ~6 |

A fifth, structural condition multiplies all four: **eight crates at zero kloc headroom with an equality kernel pin**, so even a correct story cannot land a line without a governance round.

## 2. The confidence rules (what changes the number, not the wording)

Each rule kills one shape. Together they are the design constraint for every rewritten epic.

1. **Verb-first.** An exit command may use only verbs and flags that exist at HEAD, or that are AC1 of a story *in the same epic*. Cross-epic verbs require a dependency edge. Mechanical check: a ≤60-line xtask leg parses the fenced exit block of each `epic-*.md` and resolves every `maos`/`maosctl`/`xtask` token against the live `--help` — one gate in, one absent-pass gate retired.
2. **Hermetic exit, live optional.** Every exit command runs on a GitHub-hosted runner with no secrets. Anything needing money, a human, an external account, a wall-clock window or a second physical host is an **operator-lane row** with its own key, never a clause in an engineering exit. The engineering exit becomes the epic's first CI job on day one (proven-red, then green) — the project's existing convention, applied to the demo itself.
3. **Foundations first, as stories.** The five blockers become the first epic's stories: ceiling re-base; four ADRs; release repair; inference record/replay seam; provisioning checklist. No feature epic opens until they are `done`.
4. **Cite-or-cut.** Every AC names the file:line it changes and its kernel-Δ (from the preflight scouts' corrected wording). An AC with no citation is cut, not deferred.
5. **Spike before build.** Anything unproven at HEAD (WASM recall ops, componentize-js, daemon POST surface, T3 egress allowlist) gets a ≤3-day spike story with a written go/no-go before the build story is sized — the 11-0 precedent.
6. **Size from measurement.** Use the scouts' per-story estimates plus 30% contingency; ≤6 stories per epic; split what exceeds; state the duration range, never a point.
7. **One decision per fork, before the epic opens.** Forks are ADRs, ratified at the foundations epic, not discovered mid-story.
8. **Inventory from the tree.** Every count in an AC is produced by a command written into the AC (`grep -c …`), so the number cannot drift.

## 3. The structural decision — Spirit forms (Fork A / B / C)

The preflight's largest finding: the v0.1 subprocess Spirit form was never built; all ten Spirits are in-process; the only subprocesses are the CliWrapper Worker (T3, `--network=none`) and the WASM host (built at 11.1a, cross-form gate 20/20, off by default only for export counsel).

| Fork | What it means | Cost | Promise kept |
|---|---|---|---|
| **A · Build the subprocess host** | New Spirit Wire Protocol runtime: 14-hook dispatch over NDJSON/CBOR, request ops, SDK transport, launch path, re-host ten Spirits | +4–8 wk, a new unproven runtime | All of FR5/FR10/FR29/FR33 as written |
| **B · Ratify in-process only** | First-party Spirits stay in-proc; sandbox applies to Workers only; TS/Python are conformance harnesses | −4 wk | FR5 narrowed to Workers; FR33 narrowed; the third-party-binary thesis weakens |
| **C · Forms by trust tier (RECOMMENDED)** | `rust-inproc` = first-party, kernel-distribution code (trusted like a kernel module, never admitted from a registry); **WASM component = the third-party and polyglot form** (isolated by construction — the PRD's own v2.0 wording); `cli_wrapper` = agent CLIs under T3 + egress allowlist + kernel-minted credential | +1–2 wk (WIT extension for `log.recall`/`log.fetch`, `frame_bridge` carries intent/consent/lineage/scope, componentize-js spike) | FR5 true for every non-first-party form; FR33 via WASM toolchains (TS now, Python via componentize-py later); FR29 over the WASM wire; no new runtime; ADR-031 already names WASM the second form |

C is recommended because it reuses the one out-of-process form that already exists and is gated, it makes the sandbox claim true *by construction* for exactly the code the PRD worries about (third-party), and it dissolves the 18-1 collision (Orchestrator stays in-proc with a builder-injected port). Its one honest cost: a WASM-inclusive **published** binary waits on export counsel (RELEASE-HOLDS Hold 2); third parties can build with `--features wasm-host` in the meantime, and the v1.0 tag is `v1.0.0-rc` until the GA holds clear — which was already the ledger's rule.

## 4. The re-plan under Fork C (7 epics, 28–40 weeks + operator lane)

| Epic | Wave | Stories (≤6) | Hermetic exit (CI, no secrets) | Operator-lane rows (live/paid/human) | Measured |
|---|---|---|---|---|---|
| **15 Foundations** | W0 | 15-1 green-at-head (I9 attribute move + 2 adds via re-pin, 16 symbols + baseline regen, env vars, trace-race root cause) · 15-2 **ceiling re-base** (one measured, operator-ratified re-base of `kloc.toml`; amend the retro-only aggregate rule) · 15-3 single phase source (8→1) + exit-command verb check · 15-4 release repair (artifact merge, `maosctl` artifact, aarch64 leg, `sign-and-publish` needs aggregate) · 15-5 the four ADRs (forms by trust tier; Worker egress + credential; mutating operator surface; revocation model) + provisioning checklist story · 15-6 inference record/replay seam for every Spirit + nightly `--live` fix | aggregate green; `cargo run -p xtask -- check-exit-commands` green; `git tag v0.1.0-alpha.1` builds signed artifacts incl. `maosctl` | provision `RELEASE_SIGNING_KEY`/`MAOS_RELEASE_PUBKEY`; hotfix lane rows | 2–3 wk |
| **16 One daemon, one door** | W1 | 16-1 daemon POST surface + token minting + 19 verbs re-targeted + 13 tests updated (0 kernel-Δ) · 16-2 shell halt registry + J0 scene (hello `Ambiguous` → `HaltRegistry` + `EpistemicHalt` frame; resolve via 16-1; journey_j0 asserts) · 16-3 subprocess crash → `handle_crash` + 100-kill/50-hang jobs assert numbers · 16-4 `maos uninstall` (new verb) + keyring backend · 16-5 debt (audit drop, legal-hold, deny vocabulary; FLAG-Winston grants from 15-2) | `maos init && MAOS_INFERENCE_MODE=replay maos shell` → halt → `maosctl halt resolve` → `maos run spirits/butler/manifest.toml &` + `maosctl pause butler` observable in `spirit inspect` → `maos uninstall` | live-key J0 leg | 4–5 wk |
| **17 Workers and the third-party form** | W2 | 17-1 Worker egress allowlist + `env_clear` + kernel-minted credential (T3 argv Δ, FLAG-Winston; `/proc/<pid>/environ` test) · 17-2 cgroups applied to Workers (one layout, `pids.max`, delegation precondition) · 17-3a **spike** componentize-js + WIT recall ops (≤3 d, go/no-go) · 17-3b WASM form: `log.recall`/`log.fetch` in `maos:spirit@1.x`, `frame_bridge` carries intent/consent/lineage/scope, one TS Spirit runs and recalls · 17-4 hooks: telemetry pump → `on_telemetry_event`, `on_consolidate` firer, Observer in `LoadedSpiritKind`, 20-scenario activity corpus | fixture Worker probing `~/.ssh` → EACCES row in `maos audit query`; `maos run examples/ts-spirit.wasm --once` recalls its own frames | live `claude`/`codex` under the profile | 4–6 wk |
| **18 Spirits that think** | W3 | 18-1 Butler inference seam + conformant MCP client (initialize/session/bearer) against a fixture MCP calendar server in CI · 18-2 `maos eval halt --class <c>` (new verb) over the 30-row corpus via replay; numbers to registry metadata · 18-3 Researcher `--replay`/`--deterministic`, live-when-configured, judge harness for five-metric via cassettes, `time_cap_seconds=5400` with injected clock · 18-4 self-tuning halt (acceptance ledger in TL + injectable clock) | `MAOS_INFERENCE_MODE=replay maos run spirits/butler/manifest.toml --once` proposes; `maos eval halt --class butler` prints recall/precision | nightly re-record with key; Google OAuth MCP | 4–6 wk |
| **19 Founder loop** | W4 | 19-1 Orchestrator dispatch loop + inference seam + `--epic <file>` · 19-2 FR20 enqueue door via 16-1 + `dequeue_at_safe_point` caller · 19-3 real Worker default under 17-1 + worktree-diff effect oracle (maos-bin, explicit cwd) · 19-4 J1 beats: ambiguity halt emitter (`story.acceptance_criterion.ambiguous`), digest render, `demo-j1 --replay`, journey_j1 asserts | `cargo run -p xtask -- demo-j1 --replay` with ABSENT = {two-host-signed-run} only | `--live-codex`; two-host signed run (2d lane) | 4–6 wk |
| **20 Ship it** | W5 | 20-1 registry client in CLI + `maosctl spirit install <name>@<ver>` (new verb) + vetter `issue`/`revoke` + yank policy · 20-2 deb via `dpkg-deb`, air-gap matrix leg, Docker `ARG FEATURES`, brew formula/AUR files corrected · 20-3 gate honesty: 23 absent-pass → `EvidenceState`, honest-N trial mode, ledger branches, chained fuzz jobs (floor stays advisory until 90 d), coverage matrix generated from `gate-registry.toml` + a `covers` field (no proc-macro), retirements with invariant-lock review · 20-4 Gemini driver + endpoint pin enforced in the adapter · 20-5 `v1.0.0-rc.1` tag + LTS clock rule | clean VM: download signed release → self-hosted registry → `maosctl spirit install researcher@1.0` → `maos shell` | brew tap publish, AUR, external cohort, pen-test scheduling, GA `v1.0.0` after Holds 1–2 | 5–7 wk |
| **21 Multi-host on live substrate** | W6 | 21-1 J4: 50-incident fixture + signal adapter + Mira/Nash inference seams + scenario clock; deny-path intake in the J4 topology (rupture oracle un-ignored) · 21-2 nightly Postgres via `provision-loom-substrate`, the two ignored packages with `--run-ignored`, capture test un-ignored, Reza leg Blocking · 21-3 durable `TofuPinStore` (home: `maos-persistence`, kept), sync read path, 14-2d `T_grace` raise with both legs re-derived, typed post-grace token · 21-4 one instrument (tokei) + valid TOML + author-or-retire the 25K ceiling + env scanner root = workspace + registry relocation with grant + `Secret` class + `MAOS_REGION_HOME` reconciliation · 21-5 (optional, FLAG-Winston) `orchestrator/` port + move, negative re-pin | `cargo run -p xtask -- demo-reza` (already red-on-absence) + nightly with zero ignored journey tests except those with a named owner | live J4 with key (300+ calls) | 5–7 wk |

Sum: **28–40 weeks** engineering; operator lane in parallel. Expected confidence after rewrite (same scale the preflight used): 15 · 75, 16 · 70, 17 · 60, 18 · 65, 19 · 60, 20 · 55, 21 · 55. The residual is honest: kernel-Δ grants (16-5, 17-1, 21-5), the WASM ABI extension, and three spikes.

## 5. What this does NOT change
No FR is added, removed or weakened except by the two ADRs in 15-5 that *narrow honestly* (FR5 applies by form; FR33 is served by WASM toolchains). The phase order stands. Epics 0–14 stay closed. The kernel pin stays 24472 until 15-2's re-base is ratified.

## 6. Decisions requested from the operator (inside the workflow)
1. Fork A / B / **C**.
2. Ratify the eight confidence rules as binding on every recovery story.
3. Ratify the 7-epic shape (renumber 20-*/21-*; Epic 21 = former 20).
4. Ratify a one-time kloc ceiling re-base as 15-2 (measured at landing; amend `kloc.toml:501`).
