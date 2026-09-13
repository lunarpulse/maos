# Preflight R2 — Epic 18 Spirits That Think (W3)

> Scout note, 2026-09-05. Epic file: `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`. Every citation pinned `@9f920180` (HEAD re-verified at start and end; `git status --porcelain` showed only this note's directory as untracked — nothing cited was dirty). Method: read-only. No `maos` binary was executed (Epic-15 finding: `maos --help` boots the daemon); every `maos run` flag was verified in the hand parser `crates/maos-bin/src/worker_spawn.rs:44-72`. `tokei` was run directly for the four crates the Epic-16 §E table does not carry. Builds on the Epic-15/16/17 notes (all `needs-rework`; 15-2/15-3/15-6 all `backlog`).

## Verdict block

| | |
|---|---|
| **Verdict** | **needs-rework** (not blocked: the replay port, the deferred-port builder, the streamable-HTTP transport, the `maos-eval` scorer and the hook budget envelope are all real — but one story (18-4) stands entirely on mechanisms that do not exist and cites the wrong test and the wrong `SystemTime::now()`; 18-2's founding premise ("first real per-class number") is false at HEAD; the exit block carries a wrong path, three undeclared flags and a four-way "replay" fork; and a cap number (5400) with no source in the tree) |
| **Confidence** | **40 / 100** |
| **Measured duration** | **5.0–8.3 weeks** serial for one engineer + agents (epic says 4–6); 4.0–6.5 weeks if 18-3 runs parallel to 18-1 |
| **False / partial premises** | **9 FALSE, 13 PARTIAL** out of 52 audited claims (§A); 30 CONFIRMED TRUE |

**Top-3 blockers**

1. **18-4 is an assumed foundation, end to end.** `notification_acceptance_log` has zero occurrences in the tree (`grep -rn notification_acceptance_log crates spirits xtask` = 0 @9f920180); `Butler::with_clock` does not exist (Butler builders are `with_scenario/with_scalar_port/with_output_channel/with_mcp_port`, `spirits/butler/src/lib.rs:508-539`); the cited `SystemTime::now()` at `lib.rs:833` is line **834** inside `chrono_stub_snooze_time()` (`:831-843`, doc "Stub snooze-time computation … Real scheduling is v0.4") — a display string, not a cadence; Butler has **no** clock consumer at all (`grep -n Clock spirits/butler/src/lib.rs` = 0). `journey_butler.rs:285` is the `#[ignore]` on `jb8_posture_shift_cognition` ("RED: Epic 9 — posture-shift cognition not yet wired", `crates/maos-journey-test/tests/journey_butler.rs:284-289`), not a self-tuning test; `jb3_self_tuning_halt` is a separate file (`tests/jb3_self_tuning_halt.rs:54 jb3_self_tunes_via_belief_variance_halt`) that is **not ignored** and already runs green in `journey-hermetic-tier-1` (`discipline.yml:2128 cargo test -p maos-journey-test`). AC1's `--once` is "a single `on_idle` pass" (`worker_spawn.rs:37-38`, `main.rs:3341`) and cannot show "three simulated weeks"; the on_idle cadence is kernel code (`scheduler/idle_watchdog.rs:44-58`, with its own test knob `MAOS_IDLE_FAST` `:46-49`). The tuning rule itself (what declines, what threshold, where the ledger lives — TL per the input package `:51`, principal namespace per the epic AC2) is undecided. This is a design fork plus three new mechanisms, not a story with two ACs.
2. **18-2's premise is false: the per-class number already exists and is asserted in CI.** `spirits/butler/tests/corpus_halt.rs:296-305` asserts `halt_recall_calendar_conflict >= 0.90` and `halt_precision_overall >= 0.85` on the same 30-row corpus, re-deriving every `observed_halt` through the real kernel `WorkingMemoryOrchestrator::process_scalar_write` (`:105-139`), run by the `butler-tests` job (`discipline.yml:723-730`); the scorer is `maos-eval/src/onboarding_gate_corpus.rs:499-506` (floors `HALT_RECALL_FLOOR = 0.90` `:60`, `HALT_PRECISION_FLOOR = 0.85` `:62`), also driven by `nfr-onb-1-gate` (`discipline.yml:2346-2357`). And the metric measures the **halt arithmetic**, not a model: `assess()` (`lib.rs:623-667`) sets `belief_variance = 0.75 + 0.05·conflicts` from calendar overlaps (`:629-633`); the corpus rows carry no prompt or completion (row `cc01` fields: `calendar_conflict, expected_halt, input{calendar,comms}, observed_halt, scenario_id`). Under replay, "recall from recorded completions" is either today's number again (if the scalar stays arithmetic) or a property of hand-authored cassettes whose prompt hash is the all-zero wildcard (`cassettes/j-butler/on-idle-halt.json:10`, accepted by `cassette_replay.rs:127-133`). The AC must say which scalar the model sets and what reds.
3. **Exit block defects (rule 1/2/7).** Line 3's cassette path `tests/cassettes/j-researcher/survey-distill.json` does not exist — the file is `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json` (consumed at `journey_researcher.rs:69-70`). `--replay`, `--deterministic`, `--clock` are not `maos run` flags (`worker_spawn.rs:53-66` accepts `--live`, `--once`, `--replay-llm`); `--clock` is used by 18-4 AC1 without any AC creating it. Four ways to say "replay" coexist: `--replay-llm` (no-op alias at HEAD `:56-59`), `MAOS_REPLAY_CASSETTE` (`env_contract.rs:255`, read `main.rs:4623`), `MAOS_INFERENCE_MODE=replay` (15-6) and `--replay <path>` (18-3) — an open fork. Line 1 under the amended 15-6 contract ("`replay` requires `MAOS_REPLAY_CASSETTE`", Epic-15 note §K #14) names no cassette. Line 2's `maos eval` cannot be resolved by `check-exit-commands` because `maos` has no `--help` (Epic-15 blocker 1). Line 3 cannot fail on its cassette today: an exhausted or absent cassette errors into the fallback `summarize(statement)` (`spirits/researcher/src/lib.rs:915-932`) and exits 0.

---

## A. Citation audit

| # | Claim | Where | Verdict | Evidence @9f920180 |
|---|---|---|---|---|
| 1 | `MAOS_INFERENCE_MODE=replay` | exit l.1, operator legs | NEITHER at HEAD | `grep -rn MAOS_INFERENCE_MODE crates spirits xtask .github` = 0; created by 15-6 (`sprint-status.yaml:240`, `backlog`) |
| 2 | `maos run spirits/butler/manifest.toml --once` | exit l.1, 18-4 AC1 | TRUE | `worker_spawn.rs:44-72` (`--once` `:55`); manifest exists; `jb3_self_tuning_halt.rs:57` spawns exactly this |
| 3 | "Butler proposes from the cassette; the halt fires on the conflict scenario" | exit l.1 | PARTIAL | halt fires today from the hard-coded seed (`main.rs:4218-4242`, two Confirmed overlaps → 0.80 > 0.7 `manifest.toml:83-86`) — no cassette is involved; "proposes" needs 18-1 |
| 4 | `maos eval halt --class butler` (new verb) | exit l.2, 18-2 AC1 | NEITHER | `grep -n '"eval"' crates/maos-bin/src/main.rs` = 0; `grep -n Eval crates/maos-cli/src/cli.rs` = 0; declared AC1 of 18-2 (rule 1 satisfied) but unresolvable by `check-exit-commands` (no `maos --help`) |
| 5 | "recall ≥0.7 / precision ≥0.85 from recorded completions" | exit l.2 | PARTIAL | floors are PRD (`prd/non-functional-requirements.md:81` NFR-Test-4; `functional-requirements.md:48` FR15; `success-criteria.md:47`); the tree already asserts **0.90**/0.85 (`corpus_halt.rs:298,303`; `onboarding_gate_corpus.rs:60,62`) — the epic's floor is *looser* than the shipped one; "from recorded completions" is not how the number is derived (see blocker 2) |
| 6 | `maos run spirits/researcher/manifest.toml --replay <cassette> --once` | exit l.3 | NEITHER (`--replay`) | `worker_spawn.rs:53-66`: unknown argument → `Err("unknown argument … expected: <manifest> [--live] [--once]")` `:61-66` |
| 7 | `tests/cassettes/j-researcher/survey-distill.json` | exit l.3 | **FALSE** | no `tests/cassettes` dir; file is `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json` |
| 8 | "nightly re-record (`MAOS_INFERENCE_MODE=record`, provider key)" | operator legs | PARTIAL | the leg exists as `tier-2-rerecord` (`journey-nightly.yml:65-99`) using `MAOS_JOURNEY_MODE: record` (`:87`) and the invalid nextest `--live` (`:94`); env var renamed by 15-6; **no `ops-*` tracker row** (`sprint-status.yaml:281-284` has four ops rows, none for re-record) |
| 9 | "Google Calendar MCP with OAuth (the CI server is a conformant fixture)" | operator legs, 18-1 AC2 | PARTIAL | no OAuth code and no `ops-*` row; the "fixture" today is `MockMcp` (`maos-journey-test/src/lib.rs:175-300`), an HTTP responder that reads `Content-Length` and returns the fixture body regardless of method (`:236-295`) — not protocol-conformant |
| 10 | "ZERO kernel-Δ @24472 (clock injection at the adapter)" | Kernel-Δ line | PARTIAL | pin TRUE (`kernel-core-baseline.toml:472 src_lines = 24472`; set = `crates/maos-kernel-core/src`, `check_kernel_baseline.rs:31`). The clocks that matter are **kernel**: `hook_dispatch.rs:381,466,574` (`tokio::time::timeout`), `:592` (80 % `tokio::time::sleep`), `idle_watchdog.rs:50,70,114` (`tokio::time::interval`, `monotonic_now_ns`). "At the adapter" is only zero-Δ if the scene drives `fire_on_idle` itself and the AC3 proof uses tokio paused time (unproven under `spawn_blocking`, `hook_dispatch.rs:576`) |
| 11 | "Closes: S3" | header, 18-1 | PARTIAL | S3 = "One Spirit thinks (Butler never imports InferencePort; **Observer unlaunchable; 2 hook firers have no callers**)" (`sprint-change-proposal-2026-09-04.md:26`); 18-1 closes only the Butler third; Observer/hook callers are 17-4 |
| 12 | "FR17/FR55" closed by 18-1 | 18-1 | PARTIAL | FR17 = digest within 30 s of first session (`functional-requirements.md:50`) needs a schedule (18-4's clock); FR55 = the 11-hook trigger set (`:68`) — 18-1 touches only `on_idle` |
| 13 | "FR30/32 measured, FR56" | header | PARTIAL | FR32 floors are **0.85/0.85** (`functional-requirements.md:77`), not 0.7; FR56 (self-telemetry read) has no AC in this epic |
| 14 | NFR-Test-4 / NFR-Aud-7 / NFR-Perf-6 "closed" | header, 18-2/18-3 | PARTIAL | all three rows are `gates: [] corpora: []` (`tests/coverage-matrix.yaml:1879,973,1359`); closing needs matrix rows + `gate-registry.toml` entries (`check-coverage-matrix-completeness` exists, `gate-registry.toml:58`) — no AC says so |
| 15 | deferred-port builder pattern `main.rs:4599-4622` | 18-1 AC1 | TRUE | `:4590-4622` builds `InferencePortAdapter` + optional `CassetteRecordPort`, `researcher.with_deferred_inference_port(port, binding)` `:4621`; replay arm `:4623-4641`; binding backfilled `:4796` |
| 16 | "Butler has no such field today, `spirits/butler/src/lib.rs:337-470`" | 18-1 AC1 | TRUE | struct `:344-380` (fields `pending, last_assessment, scalar_port, last_halt_receipt, mcp_port, last_notification, output_channel`); `grep -n -i inference spirits/butler/src/lib.rs` = 0 |
| 17 | (implicit) Butler can call `InferencePort::complete` from `on_idle` | 18-1 AC1 | TRUE | `complete` is sync (`cassette_replay.rs:116`; Researcher calls it at `researcher/src/lib.rs:915`); Butler's manifest already declares the scope `provider.complete = ["anthropic.claude-3-haiku-20240307"]` (`manifest.toml:20-21`) |
| 18 | `maos-mcp` "today no `initialize`, no auth header; stdio spawns per call" | 18-1 AC2 | TRUE | `grep -rn '"initialize"\|Mcp-Session-Id\|protocolVersion\|Authorization' crates/maos-mcp` = 0; headers are only `Content-Type`/`Accept` (`transport/streamable_http.rs:32-35`); body is `tools/call` id 1 (`:58-62`); `stdio.rs:24-37` "actual spawn happens per-invocation", `:140` |
| 19 | (implicit) a streamable-HTTP client exists to extend | 18-1 AC2 | TRUE | `StreamableHttpTransport` (`streamable_http.rs:13-51`, 156 lines) over `IoSubsystemPort::http_post`; production impl is `ureq` in **kernel** `io/mod.rs:97-115` (headers passed through `:103-105`) |
| 20 | "protocol-conformant fixture calendar server" exists/proven in CI | 18-1 AC2 | **FALSE** (none exists) | `FixtureReplayMcpServer` is an in-process queue (`maos-mcp/src/fixture_replay.rs:15-53`, feature-gated); `MockMcp` and `spawn_mock_mcp_server` (`butler_8_14b.rs:158`, `researcher_8_14c.rs:68`) answer any request with the next fixture body |
| 21 | `morning_digest` `butler/lib.rs:675-728` "without production caller" | 18-1 AC3 | TRUE | `pub fn morning_digest` `:675-682`; callers: `maos-bench/src/harness/j0.rs:83` (bench), `journey_butler.rs:153`, `butler/tests/{hallucination.rs:152,digest.rs:122}` — none in `maos-bin` |
| 22 | "The retired model pin is gone (depends on the hotfix lane)" | 18-1 AC4 | PARTIAL | pin TRUE (`spirits/butler/manifest.toml:21`; also `researcher/manifest.toml:47 claude-3-5-sonnet-20241022`); the lane row is `model-pin-currency-gate-and-retired-pin-sweep` (`sprint-status.yaml:223`, **backlog**, sweep names "butler+researcher manifests"); the epic's Dependencies line does not name it |
| 23 | 30-row corpus `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` | 18-2 AC1 | TRUE | `wc -l` = 30; size lock `corpus_halt.rs:160`; path constant `BUTLER_CORPUS_REL` (`onboarding_gate_corpus.rs:79`) |
| 24 | "through the 18-1 seam from cassettes" | 18-2 AC1 | PARTIAL | the corpus feeds `ScenarioInput` (calendar/comms); no field carries a prompt/completion; the cassette format is sequence-keyed with a prompt-sha256 check (`cassette_replay.rs:8-18,127-140`) |
| 25 | "numbers written to registry metadata" | 18-2 AC1 | **FALSE** (no such surface) | `grep -rn halt_recall crates/maos-registry/src` = 0; the only schema carrying it is `docs/third-party-trial/results/trial-results-schema.toml:25`; PRD says "published in registry" (`success-criteria.md:47`) but names no field — fork §H |
| 26 | "NFR-Test-4, first real per-class number" | 18-2 AC1 | **FALSE** | see blocker 2 (`corpus_halt.rs:296-305`, `halt_recall_floor.rs:48-90` on the 62-row synthetic corpus) |
| 27 | "corpus gains model I/O fields (prompt, completion reference)" | 18-2 AC2 | TRUE (as a delta) | rows have none today (row `cc01`); `corpus_pin` SHA-256 tests will red on edit (`discipline.yml:727-729`) — the AC must re-pin |
| 28 | "silent fallback at `lib.rs:925` removed" | 18-3 AC1 | PARTIAL | crate is `spirits/researcher`; `:915-932`: on `Err`, `Unconfigured`/`CapabilityDenied` are **logged** (`:921-926`), every other error is silent (`:927 _ => {}`), and all arms fall back to `summarize(statement)` (`:930`); `:925` is inside the eprintln |
| 29 | "live only when a provider is configured, otherwise a typed `Unconfigured` halt" | 18-3 AC1 | PARTIAL | boot-time check already exists: `--live` with no provider → `Err("--live requested but no inference provider is configured")` (`main.rs:4590-4596`); `InferenceError::Unconfigured` exists (`researcher/src/lib.rs:922`); what is missing is the runtime typed exit |
| 30 | "`--deterministic` is explicit" | 18-3 AC1 | NEITHER | today the deterministic path is the absence of `--live` (`main.rs:4642-4645`); `--replay-llm` is an accepted no-op for it (`worker_spawn.rs:56-59`) |
| 31 | "A judge harness in `maos-eval`" | 18-3 AC2 | NEITHER | `maos-eval` exists (`crates/maos-eval`, 14 corpus modules `src/lib.rs:17-31`); `grep -rn judge crates/maos-eval/src` = 0; 8.11 FORK A left the soft metrics as "REPORTED evidence … needs a real judge/replicator LLM" (`researcher/tests/five_metric_live_8_11.rs:20-26`) |
| 32 | `distillate_five_metrics_floor.rs:53-81` "a mean of fixture labels" | 18-3 AC2 | TRUE | `crates/maos-eval/tests/distillate_five_metrics_floor.rs:53-74` means of `expected_recall/faithfulness/hedge_preservation`; asserts `:76-88`; runs at `discipline.yml:1131` |
| 33 | `time_cap_seconds = 5400` | 18-3 AC3 | **FALSE** (no source) | `grep -rn 5400 crates spirits xtask docs _bmad-output/planning-artifacts/prd` = 0; Researcher `[budget].time_cap_seconds = 60` (`spirits/researcher/manifest.toml:90`), Butler 30 (`butler/manifest.toml:65`); the number first appears in `correct-course-input-2026-09-04-confidence.md:51` |
| 34 | `BudgetWarning80` "proven with an injected clock (`hook_dispatch.rs:357-366`)" | 18-3 AC3 | PARTIAL | `:357-367` is `effective_cap_seconds` (the cap lookup); the 80 % sleep is `:592`, the emission `:639-643`; it is **already proven** with real time (`crates/maos-kernel-core/tests/hook_dispatch_budget_envelope.rs:189-225`, 900 ms hook vs 1 s cap); NFR-Perf-6 text is `prd/non-functional-requirements.md:14` |
| 35 | `jb3_self_tuning_halt` "un-ignored" | 18-4 AC1 | **FALSE** | `tests/jb3_self_tuning_halt.rs:53-54` has no `#[ignore]`; runs in `journey-hermetic-tier-1` (`discipline.yml:2095-2128`) |
| 36 | `journey_butler.rs:285` un-ignored | 18-4 AC1 | **FALSE** (wrong test) | `:285` = `#[ignore = "RED: Epic 9 — posture-shift cognition not yet wired"]` on `jb8_posture_shift_cognition` (`:284-289`) |
| 37 | "acceptance ledger declining over three simulated weeks" under `--once` | 18-4 AC1 | **FALSE** (contradiction) | `--once` = "single `on_idle` pass + graceful drain" (`worker_spawn.rs:37-38`; `main.rs:3341-3342`) |
| 38 | `--clock compressed:21d` | 18-4 AC1 | NEITHER | not in `worker_spawn.rs:53-66`; no AC declares it new; kernel already has `MAOS_IDLE_FAST` (÷100 idle window, `idle_watchdog.rs:46-49,80-83`) as a second compression mechanism |
| 39 | `notification_acceptance_log` "exists and persists in the principal namespace (FR31)" | 18-4 AC2 | **FALSE** (does not exist) | 0 hits in tree; FR31 text `functional-requirements.md:76`; the only principal-namespace code is the kernel `PrincipalNamespaceIndex` (`memory/principal.rs:33`); a Spirit-facing principal write port outside the kernel: **not checked** |
| 40 | `Butler::with_clock` replaces `SystemTime::now()` at `lib.rs:833` | 18-4 AC2 | **FALSE** | see blocker 1 (`:834` in `chrono_stub_snooze_time`, `:831-843`) |
| 41 | (implicit) a Clock abstraction exists to inject | 18-4, Kernel-Δ line | PARTIAL | `CohortClock` trait exists **outside** the kernel (`crates/maos-cohort/src/state.rs:25`, test impls in `maos-bin/tests/*_13_*.rs`); nothing in `maos-domain/src/ports/` (27 port files, no clock); nothing Butler-side |
| 42 | "18-4/21-1 compressed clock = one mechanism" | (task premise) | NEITHER | 21-1 AC1 says "under a compressed scenario clock" (`epic-21…w6.md:43`) with no mechanism named; none exists at HEAD — two stories will each invent one unless a shared trait is placed first (§H) |
| 43 | "Dependencies: Epic 16 (a pausable daemon)" | Dependencies | **FALSE** (unused) | no exit line or AC in Epic 18 pauses anything; all three exit lines are `--once`; the real dependencies are 15-6, 15-2, 15-3 and the hotfix row (§I) |
| 44 | "18-1 before 18-2" | Dependencies | TRUE, incomplete | also 18-1 → 18-4 (Butler builder), 18-3 → 18-4 (`--replay` flag), 18-3 independent of 18-1 |
| 45 | "4–6 weeks (measured)" | header | PARTIAL | §G: 5.0–8.3 wk serial |
| 46 | `kloc_check.rs:180,184` excludes `spirits/` | (task premise) | TRUE | `xtask/src/kloc_check.rs:180 "-e","tests"` … `:184 "-e","spirits"` → `spirits/butler`, `spirits/researcher` carry no ceiling |
| 47 | `maos-eval` has a kloc row | (task) | TRUE | `kloc.toml:485 maos-eval = 3696`; tokei 3661 → headroom 35; 15-2 grants +600 (Epic-15 note §E) |
| 48 | `maos-mcp` has a kloc row | (task) | TRUE | `kloc.toml:281 maos-mcp = 1100`; tokei 999 → headroom 101; **no 15-2 row** |
| 49 | `maos-journey-test` headroom | (task) | TRUE | 493/593 → 100 (Epic-16 §E); **no 15-2 row** |
| 50 | 8.11-style record seam exists for Researcher only | (Epic-15 premise) | TRUE | `MAOS_JOURNEY_MODE=record` + `MAOS_REPLAY_CASSETTE` only in the Researcher arm (`main.rs:4607-4641`); `env_contract.rs:255,260` |
| 51 | Butler cassette is a hand-authored seed | (prior preflight) | TRUE | `cassettes/j-butler/on-idle-halt.json:4 "model_id": "hand-authored-seed"`, `:10` all-zero `prompt_sha256`; **no consumer** (`grep -rn on-idle-halt crates` = 0 outside the file) |
| 52 | output_shape violation is observable under `--once` | (exit l.1 falsifier) | PARTIAL | validated (`main.rs:4886-4897`) but only `eprintln!`-ed (`:4895`); exit stays 0 |

## B. Verb-first

| Token | Exists at HEAD? | Created by | Flag |
|---|---|---|---|
| `MAOS_INFERENCE_MODE=replay\|record` | no (0 hits) | 15-6 AC1 (Epic 15, `backlog`) | cross-epic → dependency edge OK; under the amended 15-6 wording `replay` **requires `MAOS_REPLAY_CASSETTE`** — exit line 1 sets none |
| `maos run <manifest> --once` | yes — `worker_spawn.rs:44-72` | — | — |
| `maos eval halt --class butler` | no | 18-2 AC1 | `check-exit-commands` cannot resolve `maos` tokens (no `--help`; Epic-15 blocker 1) → either 15-3 fork (a) (verb table) lands first, or host the verb in `maosctl` (clap, `maos-cli/Cargo.toml:20`) |
| `--replay <path>` | no (`--replay-llm` is a no-op alias, `:56-59`) | 18-3 AC1 (implied, not stated) | collides with `MAOS_REPLAY_CASSETTE` and `MAOS_INFERENCE_MODE=replay` → §H fork 1 |
| `--deterministic` | no | 18-3 AC1 | today = absence of `--live` (`main.rs:4642-4645`) |
| `--clock compressed:21d` | no | **nobody** (18-4 AC1 uses it) | NEITHER — must be an AC1 deliverable |
| `MAOS_REPLAY_STRICT=1` | yes — `main.rs:4624-4626` | — | the only drift falsifier; unused by the exit block |
| `tests/cassettes/j-researcher/survey-distill.json` | **no** | — | path is `crates/maos-journey-test/cassettes/…` |
| `spirits/butler/manifest.toml`, `spirits/researcher/manifest.toml` | yes | — | — |
| `cargo test -p butler` / `-p maos-journey-test` (the CI jobs that already run exit-line-1's substance) | yes — `discipline.yml:730, :2128` | — | "the epic's first CI job, red until green" is **not red** for line 1 minus the env var: `jb1`/`jb3` are green today |

## C. Foundation audit

**18-1**
- `InferencePort::complete` sync, callable from `on_idle` under `spawn_blocking` — TRUE (`hook_dispatch.rs:576`; `researcher/src/lib.rs:915`).
- Deferred-port builder + token/pid backfill — TRUE for Researcher (`researcher/src/lib.rs:502-509`; `main.rs:4178,4598,4633,4796`); Butler needs a copy (~40 lines Butler + ~60–120 lines in the Butler arm `main.rs:4188-4360`).
- Capability scope for `provider.complete` — TRUE, already declared (`butler/manifest.toml:20-21`); whether admission mints it for a Spirit that never calls it today: **not checked**.
- `ButlerMcpPort` / `LiveButlerMcpPort` / `McpClientAdapter` — TRUE (`butler/src/lib.rs:812-825`; `main.rs:4249-4360`, `McpClientImpl` per server from `MAOS_MCP_*_URI` `:4337-4341`).
- Streamable-HTTP transport with header pass-through — TRUE (`streamable_http.rs:27-46`; `io/mod.rs:97-115`). `initialize`/session/bearer are **absent** (A#18) — new protocol state in the transport (session id must persist across calls: the transport is `&self` and stateless `:13-16` → needs interior mutability, ~+80–150 in `maos-mcp`, headroom 101, no 15-2 row).
- Conformant fixture server — **ABSENT** (A#20). Must be a real listener (the `MockMcp` pattern, `maos-journey-test`, headroom 100, no 15-2 row) that rejects `tools/call` before `initialize`, returns `Mcp-Session-Id`, and 401s a missing bearer — otherwise "proven in CI" is vacuous.
- `morning_digest` production caller — needs `audit_db`, `journal_path`, `now_ns`, anomalies, fire-rate (`lib.rs:675-682`); the daemon holds the first two; `now_ns` is the 18-4 clock again.

**18-2**
- Scorer + loader + floors — TRUE (`onboarding_gate_corpus.rs:27-90,431-516`, `HaltCorpus`/`halt_recall_floor.rs`).
- Kernel re-derivation of `observed_halt` — TRUE but lives in a **test** (`corpus_halt.rs:73-139` builds `WorkingMemoryOrchestrator` from `butler` dev-deps on `maos-kernel-core`, `butler/Cargo.toml:33-34`); a production verb needs the same composition → natural host is `maos-bin` (the Butler arm), not `maos-eval` (deps: `maos-domain, maos-spirit-abi`, `maos-eval/Cargo.toml`) nor `maos-cli` (deps: `clap, maos-audit`; no kernel-core edge).
- Registry metadata surface — **ABSENT** (A#25).
- Cassette-per-row — ABSENT: one Butler cassette exists (seed, unconsumed); 30 rows need 30 entries or a prompt-keyed lookup (the port is sequence-keyed, `cassette_replay.rs:117-120`).

**18-3**
- Replay port, strict mode, record port — TRUE (`cassette_replay.rs:8-152,153-220`).
- Typed `Unconfigured` — TRUE at boot (`main.rs:4590-4596`), absent at runtime (`researcher/src/lib.rs:930`).
- Five-metric hermetic test — TRUE (A#32); live evidence pass — TRUE for the two deterministic properties only (`five_metric_live_8_11.rs:9-19`).
- Judge harness / judge cassette — ABSENT; coherence: a judge cassette is keyed by the judge prompt's sha256 (`cassette_replay.rs:127-133`), and the judge prompt embeds the distillate → any regenerated distillate invalidates the cassette (strict) or the wildcard hash makes the verdict independent of the distillate (vacuous). A hermetic judge is coherent only for a **frozen** distillate corpus; "corpus regenerated from real runs" (AC2) and "judged via recorded judge cassettes" pull in opposite directions.
- Budget envelope proof — EXISTS with wall-clock (`hook_dispatch_budget_envelope.rs:189-225`); a paused-time variant is unproven under `spawn_blocking` (`hook_dispatch.rs:576`) → rule 5 spike.

**18-4**
- Acceptance log, tuning rule, clock trait, `--clock` flag, scene loop, ledger persistence — all ABSENT (blocker 1). `handle_option_pick` (`butler/src/lib.rs:551`) is the only "acceptance" event source and is director-driven → under replay the acceptance sequence must be a fixture.

## D. Kernel-Δ audit

| Story | Zero kernel lines possible? | Notes |
|---|---|---|
| 18-1 | **Yes** | Butler crate (no ceiling), `maos-bin` arm, `maos-mcp`, `maos-journey-test`. The `ureq` adapter already passes headers (`io/mod.rs:103-105`) |
| 18-2 | **Yes** | `maos-eval` + verb host; the kernel orchestrator is called, not changed |
| 18-3 | AC1/AC2 yes; **AC3 conditional** | `effective_cap_seconds`, `timeout`, the 80 % sleep and the emission are kernel (`hook_dispatch.rs:360-367,574,592,639`). Paused-time test = 0 Δ if feasible; a `Clock` trait in `HookDispatcher` ≈ +15–30 kernel lines (FLAG-Winston, and `maos-kernel-core` is at 18933/18933 with 15-2's +150 shared by 15-1/16-5/17-1/17-2/21-5) |
| 18-4 | Yes **only** if the scene loop lives in `maos-bin` and calls `fire_on_idle`/the scalar port directly N times | Compressing the kernel's own cadence means `idle_watchdog.rs` (kernel) — `MAOS_IDLE_FAST` exists but is a ÷100 test knob, not a scenario clock |

The epic's "if a kernel hook is needed, FLAG-Winston" hedge sits on 18-3 AC3 and 18-4; the FLAG should be decided before story creation (rule 7).

## E. Kloc headroom (tokei @9f920180; ceilings `kloc.toml`)

| Crate | measured / ceiling / headroom | 15-2 grant (Epic-15 §E) | already claimed by Epics 16/15-6 | Epic 18 ask (estimate) | Verdict |
|---|---|---|---|---|---|
| maos-bin | 17109 / 17109 / **0** (`kloc.toml:343`) | +1200 | 16: +480–820; 15-6: +150–300 | 18-1 arm +80–150 · 18-2 verb +80–200 · 18-3 flags/typed exit +40–80 · 18-4 scene +100–250 → **+300–680** | **exceeds what is left (80–570)** → 15-2 must name Epic 18 or 18-2/18-4 must host code elsewhere |
| maos-mcp | 999 / 1100 / 101 (`:281`) | **none** | — | initialize/session/bearer +80–150 | **needs a 15-2 row (+300)** |
| maos-eval | 3661 / 3696 / 35 (`:485`) | +600 | — | halt eval driver +150–300; judge harness +300–600 | fits after 15-2 |
| maos-journey-test | 493 / 593 / 100 | **none** (Epic-16 §E flagged) | 16-2 +50–120 | conformant fixture server +150–300 | **needs a 15-2 row (+400)** |
| maos-providers | 1252 / 2000 / 748 | +400 | 16-4 +30–60 | 0–50 | fits |
| maos-domain | 8695 / 8644 / **−51** | +300 | 16-5 +16–26; 16-4 +20–40 | a `Clock` port (if placed here) +20–40; ledger types +30–60 | fits only after 15-2 |
| maos-cli | 5270 / 5270 / 0 (`:330`) | +400 | 16-1 +400–700 | 0 unless `maosctl eval` (+60–120) | over-subscribed already |
| maos-kernel-core | 18933 / 18933 / 0 (`:195`) | +150 shared | 15-1, 16-5, 17-1, 17-2, 21-5 | 0 (18-3 AC3 trait would be +15–30) | keep 0 |
| xtask | 41953 / 41953 / 0 (`:212`) | net ≤ 0 | — | 0 — CI job must be a plain `cargo test`, not an xtask gate | OK |
| spirits/butler, spirits/researcher | not governed (`kloc_check.rs:184`) | — | — | most of 18-1/18-4 logic can live here | free |

Aggregate: 155498 measured vs `_aggregate_hardfail = 147057` (`kloc.toml:501`, Epic-16 §E) — red until 15-2.

## F. Hermetic vs operator

| Need | Where | Routed to an `ops-*` row? |
|---|---|---|
| nightly re-record with a provider key (`MAOS_ANTHROPIC_API_KEY`, `journey-nightly.yml:56,88`) | operator legs; 18-2 AC3 | **No** — tracker has only `ops-provisioning-secrets-and-accounts`, `ops-external-cohort-and-pen-test`, `ops-brew-tap-and-aur-publication`, `ops-ga-tag-after-holds` (`sprint-status.yaml:281-284`) |
| Google Calendar OAuth account + consent flow | operator legs; 18-1 AC2 "Google OAuth documented" | **No** |
| `=live` with a key producing a real completion | 18-1 AC1 clause | **No** (clause inside an engineering AC) |
| "three simulated weeks" | 18-4 AC1 | hermetic only if the clock is injected; today it is wall-clock (`idle_watchdog.rs`) |
| a real judge LLM for the soft five metrics | 18-3 AC2 | not routed; 8.11 FORK A already classed it Tier-2/nightly |

Rule 2 requires two new rows: `ops-nightly-cassette-rerecord` (key present, cassettes refreshed, `cassette-age-gate` green — `xtask/src/main.rs:663-671`) and `ops-google-calendar-oauth-mcp` (18-1 AC2 live leg). The engineering ACs should cite those keys instead of "(operator lane)".

## G. Sizing from measurement

| Analogue (landed) | Commit | code LOC (non-md, incl. tests) | calendar |
|---|---|---|---|
| 8-11 daemon + Researcher inference seam + JB-3 | `1f294e08` 2026-06-08 | +1830 (tests +897) | registered 06-06 → 06-08 (2 d) |
| 8-14b Butler MCP driver set (4 servers, mock server) | `e4d9f83b` 2026-06-09 | +1370 | 06-06 → 06-09 (3 d) |
| 8-10 Butler halt seam (A-prime) | `12a86ef1` 2026-06-08 | +1252 | 06-06 → 06-08 (2 d) |
| 8-15 journey harness + cassette replay/record | `454dba44` 2026-06-11 | +2001 (tests +891) | 06-08 → 06-11 (3 d) |
| 7-5b nfr-onb-1 gate + Butler corpus scorer | `e70659c8` 2026-06-01 | +1847 | 1 d |
| 9-1 maosctl audit subcommands (new verbs) | `77a34d0f` 2026-06-12 | +8847 / −2577 (tests +2489) | 06-12 → 06-13 incl. kernel re-pin `2de6b418` (2 d) |
| 5-5c MCP client + 3 transports | `1e3ebc35` 2026-05-24 | +3123 | 1 d |
| 9-5b OTel SLO-class adapter | `fdfa3468` 2026-06-16 | +2481 | preflight 06-15/16 → 06-16 (1–2 d) |
| 10-4c J4/J6 harness rebuild | `c9ce2fd3` 2026-06-24 | +1082 / −220 | 06-23 → 06-24 (1–2 d) |
| 12-4b J3 digest scene | `bec4bb16` 2026-07-12 | +1370 | 1 d |

Scale used = the Epic-17 note's (commit calendar + preflight + §A6 review rounds ≈ 0.4 wk per commit-day), then +30 %:
- **18-1** ≈ 8-14b + half of 8-11 + a 5-5c-class transport change + a new fixture server (no analogue; the MockMcp pattern is +~150): 1.2–2.0 wk → **1.6–2.6 wk**
- **18-2** ≈ 7-5b (+1847, 1 d) + a verb slice of 9-1 + the registry-metadata surface (no analogue): 0.6–1.0 wk → **0.8–1.3 wk**
- **18-3** ≈ 9-5b-class adapter (judge harness, +~1500) + 8-11 FORK-A live pass + flags + AC3 spike: 1.0–1.6 wk → **1.3–2.1 wk**
- **18-4** ≈ design round (tuning rule, ledger schema, clock trait placement) + 12-4b-class scene + 10-4c-class harness: 1.0–1.8 wk → **1.3–2.3 wk**

**Total 5.0–8.3 weeks** serial (epic: 4–6). 18-3 is Researcher-only and can run parallel to 18-1 → 4.0–6.5 wk on two tracks. The epic's floor is reachable only if 18-4's design is settled before it leaves backlog and the fixture server is small.

## H. Fork audit (rule 7)

1. **How replay is selected** — `--replay-llm` (HEAD no-op), `MAOS_REPLAY_CASSETTE` (HEAD), `MAOS_INFERENCE_MODE=replay` (15-6), `--replay <path>` (18-3/18-4). Recommend: **one selector** — `MAOS_INFERENCE_MODE=replay` + `MAOS_REPLAY_CASSETTE=<path>` (already the 15-6 contract); retire `--replay-llm` in 15-6; 18-3 adds only `--deterministic` (explicit) and makes `--live` without a provider a typed non-zero exit. Exit line 3 becomes `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json maos run spirits/researcher/manifest.toml --once`.
2. **Host of `eval halt`** — `maos` (hand parser; needs kernel composition; unresolvable by `check-exit-commands` until 15-3 fork (a)), `maosctl` (clap; no kernel edge today), or a `maos-eval` bin. Recommend: **`maos eval halt` in `maos-bin`** (it must build the Butler arm + `ButlerOrchestratorAdapter` `main.rs:4198-4210`), with 15-2 naming +250 maos-bin for 18-2 and 15-3 choosing fork (a) so the token resolves.
3. **Where the numbers are published** — ComplianceClaim (frozen at E1b), manifest `[benchmarks]` (schema v2, `maos-manifest`), or a signed registry sidecar (`maos-registry/src/storage.rs`). Recommend the **sidecar**: no frozen-schema change, readable by `maosctl`.
4. **Which scalar the model influences** (18-1/18-2) — none (keep arithmetic; then 18-2 is a re-derivation) vs `user_preference_drift` from a comms-triage completion (today an input field, `lib.rs:636-645`). Recommend the latter; it is the only way "from recorded completions" measures anything.
5. **Ledger home** — Transparency Log (`correct-course-input…:51`) vs principal namespace FR31 (epic AC2). Recommend TL frames (I2 log-before-deliver, already queryable via `maos_audit::query`) with FR31 deferred to the Spirit's own store when a Spirit-facing principal write port exists.
6. **Clock placement** — Butler-private trait vs `maos-domain` port vs `maos-spirit-sdk` (shared with 21-1's "scenario clock"). Recommend `maos-spirit-sdk` (no kernel edge, both Spirits consume it, `maos-spirit-sdk = 3000` ceiling `kloc.toml:202`).
7. **18-3 AC3 mechanism** — paused tokio time (0 Δ, unproven under `spawn_blocking`) vs a dispatcher `Clock` trait (kernel Δ). Recommend a ≤1-day spike (rule 5) before 18-3 leaves backlog; if paused time fails, cut AC3 (the envelope is already proven at `hook_dispatch_budget_envelope.rs:189-225`).
8. **Scene driver for 18-4** — `maos-bin` loop over `fire_on_idle` with injected clock (0 Δ) vs kernel cadence compression. Recommend the loop; drop `--once` from the AC1 command or define `--clock` as "drive N on_idle passes at the compressed cadence, then drain".

## I. Dependency audit

- "Epic 15 (15-6 seam)" — TRUE and stronger: also **15-2** (maos-bin 0 headroom; maos-domain red; maos-mcp/maos-journey-test rows missing) and **15-3** (`check-exit-commands` must resolve `maos eval`), plus the hotfix row `model-pin-currency-gate-and-retired-pin-sweep` for 18-1 AC4.
- "Epic 16 (a pausable daemon)" — **unused** by any Epic-18 AC; drop, or state which AC pauses.
- 17-4 — only if "Closes S3" keeps the Observer/hook clause.
- Intra-epic: 18-1 → 18-2 (TRUE); **18-1 → 18-4** (builder), **18-3 → 18-4** (flag) missing; 18-3 ∥ 18-1.
- Nothing in Epic 18 needs a later epic; 21-1 should consume 18-4's clock (edge 18-4 → 21-1 is absent from Epic 21's Dependencies — not checked beyond `epic-21…:43`).

## J. Did R2 fix the prior preflight? (old E17 → new E18)

| Prior false premise (old E17, `epic-15-20-preflight-2026-09-04.md`) | R2 status | Evidence |
|---|---|---|
| "No generic subprocess Spirit host exists" (§1 row 17, §2.1) | **not applicable** — Fork C keeps first-party in-proc; 18 assumes no subprocess host | 18-1 AC1 builder pattern is in-proc (`main.rs:4599-4622`) |
| "Butler has no inference seam" | **fixed** — stated as the premise of 18-1 AC1 | A#16 |
| "`maos eval` verb absent" (§2.5) | **fixed as rule-1 AC1**, but the resolver cannot see `maos` tokens | A#4 |
| "the three-week self-tuning loop and its clock do not exist" | **still present, renamed** — now `--clock compressed:21d`, `Butler::with_clock`, `notification_acceptance_log`; none exists; wrong test and wrong `SystemTime` cited | A#35-40 |
| "MCP client cannot authenticate to Google" | **fixed in wording** (initialize/session/bearer + OAuth operator lane); no ops row; no fixture server | A#9, A#20, §F |
| §2.4 "no hermetic seam outside Researcher" | **fixed by dependency** on 15-6 (`backlog`) | A#1 |
| §2.5 `--deterministic`/`--replay` absent | **partially** — `--deterministic` is 18-3 AC1; `--replay` still appears in exit l.3 and 18-4 with a stale path; `--clock` new and undeclared | §B |
| §2.8 nightly `--live` invalid flag | **routed** to 15-6 | Epic-15 note §K #14-16 |
| §3 "`journey_butler.rs:285` blocks zero-ignored" | **wrong test carried forward** (jb8, not jb3) | A#36 |
| §5 confirmed "30-row Butler corpus" | still TRUE | A#23 |
| (new in R2) `time_cap_seconds = 5400` | **introduced by R2**, unsourced | A#33 |
| (new in R2) "first real per-class number" | **introduced by R2**, false | A#26 |

## K. Verdict, required changes, confirmed-true

**Confidence 40/100.** Every seam the epic touches is real and the two Researcher stories (18-3 AC1/AC2) and 18-1 AC1/AC3 are well-grounded. The number is held down by 18-4 (a story whose every named mechanism is absent and whose two citations point at the wrong code), by 18-2 resting on a premise the CI already contradicts, by three exit-block tokens that no AC creates and a path that does not exist, and by a four-way replay fork that rule 7 says must be closed before any story leaves backlog. None of this is fatal; all of it is rework.

**Verdict: needs-rework. Measured duration: 5.0–8.3 weeks (4.0–6.5 on two tracks).**

### REQUIRED CHANGES (OLD → NEW)

1. **Exit line 1** — OLD: `MAOS_INFERENCE_MODE=replay maos run spirits/butler/manifest.toml --once # Butler proposes from the cassette; the halt fires on the conflict scenario` → NEW: `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json maos run spirits/butler/manifest.toml --once # Butler's proposal is parsed from the recorded completion into the manifest's required output fields (pattern/confidence/evidence/options, manifest.toml:56-59); an output_shape violation or cassette drift exits non-zero (today main.rs:4895 only logs); the belief_variance halt fires as at HEAD (jb3)`.
2. **Exit line 3** — OLD: `maos run spirits/researcher/manifest.toml --replay tests/cassettes/j-researcher/survey-distill.json --once` → NEW: `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json maos run spirits/researcher/manifest.toml --once # an absent or exhausted cassette exits non-zero (the fallback at researcher/src/lib.rs:930 removed by 18-3 AC1)`.
3. **Exit line 2** — OLD: `maos eval halt --class butler # recall ≥0.7 / precision ≥0.85 from recorded completions` → NEW: `maos eval halt --class butler # replays the 30 rows through the 18-1 seam with user_preference_drift taken from the recorded comms-triage completion; prints recall/precision; asserts ≥0.90/≥0.85 (the floors CI already holds at corpus_halt.rs:298,303); writes the signed sidecar the registry serves`. Add to Dependencies: "15-3 fork (a) so `check-exit-commands` can resolve `maos eval`".
4. **18-1 AC2** — OLD: "proven in CI against a protocol-conformant fixture calendar server" → NEW: "proven in CI against a NEW fixture MCP server in `crates/maos-journey-test` (the `MockMcp` listener pattern, `src/lib.rs:175-300`) that (a) rejects `tools/call` before `initialize`, (b) issues and requires `Mcp-Session-Id`, (c) returns 401 without `Authorization: Bearer`; the three negatives are asserted (proven-red) — kernel-Δ 0; `maos-mcp` +80–150 (15-2 row required), `maos-journey-test` +150–300 (15-2 row required)".
5. **18-1 AC4** — add: "depends on tracker row `model-pin-currency-gate-and-retired-pin-sweep` (backlog); the sweep also covers `spirits/researcher/manifest.toml:47`". Add the row to Dependencies.
6. **18-1 AC1 / 18-2 AC3 operator clauses** — replace "(operator lane)" with the row keys `ops-nightly-cassette-rerecord` and `ops-google-calendar-oauth-mcp`; create both rows in `sprint-status.yaml` beside `:281-284`.
7. **18-2 AC1** — OLD: "(NFR-Test-4, first real per-class number)" → NEW: "(NFR-Test-4: the number CI already asserts at `corpus_halt.rs:296-305` becomes model-dependent and published; the `coverage-matrix.yaml:1879` row gains the new gate)". OLD: "numbers written to registry metadata" → NEW: "numbers written to a signed benchmark sidecar in `maos-registry` storage (`src/storage.rs`), readable by `maosctl`" (or whichever §H-3 option is ratified — one, named).
8. **18-2 AC2** — add: "the `corpus_pin` SHA-256 tests (`discipline.yml:727-729`) are re-pinned in the same commit".
9. **18-3 AC1** — OLD: "`--deterministic` is explicit; … (the silent fallback at `lib.rs:925` removed)" → NEW: "`--deterministic` is a new explicit flag (`worker_spawn.rs:53-66`); the runtime fallback `summarize(statement)` at `spirits/researcher/src/lib.rs:930` is removed so that any `InferenceError` under `--live` or replay is a typed non-zero exit (boot-time `Unconfigured` already exists at `main.rs:4590-4596`); the 10 existing `--once` sites stay on the deterministic default".
10. **18-3 AC2** — OLD: "scores real distillates against raw frames via recorded judge cassettes; … the corpus regenerated from real runs" → NEW: "hermetic leg: deterministic string metrics (recall of source-frame claims, traceability, secret-leakage) over a FROZEN distillate corpus replace the label means at `distillate_five_metrics_floor.rs:53-74`; judged faithfulness/hedge run only in the nightly re-record leg (`ops-nightly-cassette-rerecord`) as reported evidence per 8.11 FORK A; `coverage-matrix.yaml:973` row gains the gate". Decide §H fork; do not keep "recorded judge cassettes" and "corpus regenerated" in one AC.
11. **18-3 AC3** — OLD: "`time_cap_seconds = 5400` and `BudgetWarning80` proven with an injected clock (`hook_dispatch.rs:357-366`)" → NEW: "spike (≤1 d, rule 5): can `hook_dispatch.rs:574-643` be driven by tokio paused time under `spawn_blocking` (`:576`)? If yes, add a paused-time variant of `hook_dispatch_budget_envelope.rs:189-225` for the Researcher cap (`manifest.toml:90`, currently 60 s — cite the PRD line if the cap changes); if no, cut this AC (envelope already proven) and file the `Clock` trait as FLAG-Winston". Delete `5400` unless a source is cited.
12. **18-4 (whole story)** — rewrite as two ACs on real mechanisms: AC1 (command): "`MAOS_INFERENCE_MODE=replay … maos run spirits/butler/manifest.toml --clock compressed:21d --acceptance tests/fixtures/j-butler/acceptance-21d.jsonl` (both flags NEW in `worker_spawn.rs:53-66`; `--clock` drives N `fire_on_idle` passes from `maos-bin` at the compressed cadence, then drains — kernel-Δ 0, `idle_watchdog.rs` untouched) prints the ledger per simulated day and the tuning halt on the day the rule (STATED: metric, window, threshold) crosses; a scripted fixture whose acceptance never declines is the proven-red". AC2: "NEW `AcceptanceLedger` written as TL frames (or the ratified §H-5 home) from `handle_option_pick` (`butler/src/lib.rs:551`); NEW `Clock` port in `maos-spirit-sdk` (§H-6) consumed by Butler and re-used by 21-1; `chrono_stub_snooze_time` (`lib.rs:831-843`) reads it". Delete "`jb3_self_tuning_halt` and `journey_butler.rs:285` un-ignored" (jb3 is not ignored; `:285` is jb8 posture-shift, Epic 9 scope).
13. **Kernel-Δ line** — OLD: "clock injection at the adapter; if a kernel hook is needed, FLAG-Winston" → NEW: "18-3 AC3 spike decides paused-time (0) vs dispatcher `Clock` (+15–30, FLAG-Winston) before the story leaves backlog; 18-4 drives on_idle from `maos-bin` (0)".
14. **Dependencies** — OLD: "Epic 15 (15-6 seam) and Epic 16 (a pausable daemon). 18-1 before 18-2." → NEW: "15-2 (maos-bin +250 for 18-2, maos-mcp +300, maos-journey-test +400 named for Epic 18), 15-3 fork (a), 15-6 as amended (`MAOS_INFERENCE_MODE` + `MAOS_REPLAY_CASSETTE`), hotfix row `model-pin-currency-gate-and-retired-pin-sweep`. Order: 18-1 → 18-2; 18-1 → 18-4; 18-3 → 18-4; 18-3 ∥ 18-1. 21-1 consumes 18-4's clock."
15. **Header "Closes"** — OLD: "S3; FR17, FR30/32 measured, FR56; …" → NEW: "S3 (Butler third; Observer/hooks → 17-4); FR17; FR32 (floors 0.85/0.85 per `functional-requirements.md:77`); NFR-Test-4, NFR-Aud-7, NFR-Perf-6 with coverage-matrix rows". Drop FR56 or add an AC.

### CONFIRMED TRUE (do not re-check)

- `maos run <manifest> [--live] [--once] [--replay-llm]` hand parser at `crates/maos-bin/src/worker_spawn.rs:44-72`; `--once` = one on_idle pass.
- Researcher deferred-port builder `spirits/researcher/src/lib.rs:502-509`; daemon arm `main.rs:4590-4641` (live / `MAOS_JOURNEY_MODE=record` / `MAOS_REPLAY_CASSETTE` replay / deterministic); binding backfill `:4796`; `MAOS_REPLAY_STRICT` `:4624-4626`.
- `CassetteReplayPort`/`CassetteRecordPort` in `crates/maos-bin/src/cassette_replay.rs` (schema `maos.journey.cassette/v1`, sequence-keyed, sha256 drift check with all-zero wildcard).
- Butler struct and builders `spirits/butler/src/lib.rs:344-380, 505-539`; `assess()` `:623-667` arithmetic; `morning_digest` `:675`; `chrono_stub_snooze_time` `:831-843`; `handle_option_pick` `:551`; manifest declares `provider.complete` (`manifest.toml:20-21`), `time_cap_seconds = 30` (`:65`), halt rules (`:83-90`).
- Daemon Butler arm `main.rs:4188-4360`: boot-loud scalar port, hard-coded conflict seed `:4218-4242`, `--live` MCP clients from `MAOS_MCP_*_URI` via `StreamableHttpTransport` `:4300-4360`; `--once` output_shape check `:4886-4897` (log-only) and halt render `:4898-4920`.
- `maos-mcp`: `client.rs` (360), `transport/{stdio,sse,streamable_http}.rs`, `fixture_replay.rs`; no initialize/session/auth; prod HTTP = `ureq` in kernel `io/mod.rs:71-115`.
- `maos-eval` exists with `onboarding_gate_corpus::{score_candidate, resolve_corpus, BUTLER_CORPUS_REL}` (floors 0.90/0.85), `halt_corpus` (62 rows), `distillate_corpus` (100 rows); tests `corpus_halt.rs` (butler), `halt_recall_floor.rs`, `distillate_five_metrics_floor.rs`; CI jobs `butler-tests` (`discipline.yml:730`), `nfr-onb-1-gate` (`:2346-2357`), five-metric (`:1131`), `journey-hermetic-tier-1` (`:2128`), `tier-2-rerecord` (`journey-nightly.yml:65-99`).
- Corpora: 30-row `calendar-comms-v0.3.jsonl`, 100-row `digest-corpus-v0.3.jsonl`; cassettes `j0/shell-intro.json`, `j-butler/on-idle-halt.json` (unconsumed seed), `j-researcher/survey-distill.json` (consumed by `journey_researcher.rs:69-70`).
- Kernel time paths: `hook_dispatch.rs:360-367` cap lookup, `:574` timeout, `:576` spawn_blocking, `:592` 80 % sleep, `:639` `BudgetWarning80`; proven at `tests/hook_dispatch_budget_envelope.rs:189-225`. `idle_watchdog.rs:44-58,80-83` cadence + `MAOS_IDLE_FAST`.
- `CohortClock` (`crates/maos-cohort/src/state.rs:25`) is the only clock trait; none in `maos-domain/src/ports/`.
- PRD anchors: NFR-Test-4 `non-functional-requirements.md:81`; NFR-Aud-7 `:62-67`; NFR-Perf-6 `:14`; FR15 `functional-requirements.md:48`; FR17 `:50`; FR31 `:76`; FR32 `:77`; FR55 `:68`; FR56 `:78`; success-criteria `:47`.
- Kloc: `maos-mcp` 999/1100, `maos-eval` 3661/3696, `maos-journey-test` 493/593, `maos-providers` 1252/2000, `maos-bin` 17109/17109, `maos-kernel-core` 18933/18933; spirits excluded (`kloc_check.rs:184`); pin 24472.
- Tracker: Epic 18 rows `sprint-status.yaml:254-259` all `backlog`; hotfix rows `:223-225`; ops rows `:281-284`.

### Epic-specific questions (short answers)

1. `--once` exists; `maos eval halt` is 18-2 AC1 (NEITHER at HEAD); `--replay`/`--clock`/`--deterministic` are NEITHER; `--replay` collides with three existing/planned selectors → fork §H-1.
2. All 18-1 citations hold (A#15-18, #21); streamable-HTTP exists (`streamable_http.rs`) but stateless and unauthenticated; no conformant fixture server exists — `MockMcp` answers anything; retired pin at `butler/manifest.toml:21` (+ researcher `:47`), fixed by `model-pin-currency-gate-and-retired-pin-sweep`.
3. 30 rows TRUE; halt-recall machinery EXISTS and is CI-asserted at 0.90/0.85 through the real kernel (`corpus_halt.rs`); "registry metadata" has no surface; the PRD floor is 0.7/0.85 but the tree holds 0.90/0.85; under replay the metric is honest only about the halt arithmetic unless the AC names a model-set scalar (§H-4).
4. `lib.rs:925` is `spirits/researcher`; fallback real (`:930`), "silent" only for non-Unconfigured errors; `distillate_five_metrics_floor.rs:53-74` is label means; `maos-eval` exists; `hook_dispatch.rs:357-366` is the cap lookup (kernel); 5400 has no source; a judge cassette is coherent only over a frozen distillate.
5. `jb3` not ignored; `:285` is jb8; `notification_acceptance_log` absent; `SystemTime::now()` at `:834` in a snooze stub; no `with_clock`; the only clock trait is `CohortClock` (non-kernel); the cadence/time-cap paths ARE kernel (`idle_watchdog.rs`, `hook_dispatch.rs`) — zero-Δ holds only for a `maos-bin`-driven scene loop.
6. Falsifiers at HEAD: line 1 = exit 0 + halt render (from the hard-coded seed, not the cassette); line 2 = the arithmetic floors; line 3 = exit 0 + "researcher live MCP port wired" — an empty cassette passes because of the fallback. Yes: the halt "fires on the conflict scenario" because the seed and `assess()` say so, not the cassette.
7. 5.0–8.3 wk measured (§G) vs 4–6 stated.
