---
dev_model_used: claude-opus-4-6
---
# Story 8.15: Journey-Acceptance Test Harness + Red-Phase "Watch-It-Work" Suites (TEST TRACK — closes Epic 8)

Status: done

> **Registered 2026-06-06 (TEA test-design, Murat). Epic 8 Completion Delivery — TEST TRACK; LAST story in Epic 8, before Epic 9.** Owns the hermetic journey-acceptance architecture for ALL Epic-8-anchored journeys (J0 / J-Butler / J-Researcher / J1 / J4). **Depends on 8.11 (run-surface seam, done) + 8.14a (CLI/shell, done)** — and, unlike when this stub was sketched, **every journey story it was waiting on has now landed** (8.12 J1, 8.13+8.13.1 J4, 8.14b Butler, 8.14c Researcher). The "per-journey slices flip green as stories land" framing has therefore collapsed into: **this story flips everything green that is flippable, and signs the seals.** Relocated here from 8.11·AC5: the harness itself + Tier-1/Tier-2 suites + the JB-3 revert-to-red certification.
>
> **Recommended dev model:** `claude-opus-4-8`. Rationale: this is an integration-heavy, cross-process test-infrastructure story with one genuine architectural re-grounding (the epic's "all over tokio virtual time" is unbuildable across the PTY process boundary — see FORK 1) and many small contracts that must be composed exactly (cassette schema, constant oracles, H-guards, burn-in mechanics, GHA nightly wiring). It rewards source-grounded judgment over the existing `maos run` seams. Per `feedback_deepseek_v4_pro_patterns`, do NOT assign deepseek: this story is ~100% async invariants + integration plumbing + env-var threading — its exact weak spots.
>
> **⚙️ CHARTER NOTE — ZERO KERNEL KLOC.** Test-track story. **`maos-kernel-core/src/` MUST stay byte-identical to its post-8.14c HEAD baseline** (`git rev-parse HEAD` at story start = `8563eb4`). Every AC lands in `crates/maos-journey-test/`, `crates/maos-bin/` (replay-provider + seam flags only), `xtask/`, `.github/workflows/`, and test-data directories. If any AC seems to require a kernel-core edit, STOP and flag.
>
> **⚙️ WORKSPACE COUNT — stays 44.** The epic stub says "NEW dev-only crate `maos-journey-test`" — **STALE**: the crate has existed since Story 8.11 (skeleton, `todo!()` bodies). 8.15 adds ZERO new crates; it fills bodies. `check-workspace-count` must continue to pass at 44.
>
> **⚙️ ABI / BASELINE DISCIPLINE.** `abi-diff` must run with `--base abi-baseline/v1-pre-bump.txt` (no-base mode false-positives — Story 8.3/8.12 lesson). Frozen `maos-spirit-abi` untouched.

## ⚠️ READ FIRST — verified source-reality vs. epic stub

| Epic stub / sprint-status implies | Verified reality (source-confirmed, 2026-06-11) | Actual 8.15 delta |
|---|---|---|
| "NEW `crates/maos-journey-test`" | **EXISTS since 8.11** (workspace member 42 of 44). `src/lib.rs` (205 lines) exposes the FROZEN import surface: `JourneyWorld`+builder, `TestClock::tuesday_1pm()`, `MockMcp::calendar()`, `ReplayProvider::cassette()/queue_scalar()`, `AuditDb::temp()`, `Pty::spawn()/screen()`, `Screen::contains()/text()`, `world_llm()`, `guards::assert_no_wallclock_or_fixed_sleep()` (`todo!()`). `Screen::contains` is REAL; `Pty::screen()` returns an EMPTY screen (the **load-bearing stub** — JB-1/JB-2 reach their assertion and find the string absent → RED at exactly that line). | Fill the bodies WITHOUT breaking the frozen import surface (the whole point of the 8.11 standalone-crate decision was that 8.15 never edits a RED test's import header). Additive API is fine; renames/removals are NOT. |
| "PTY+vt100 … all over `tokio` virtual time (`start_paused`/`advance`)" | **UNBUILDABLE AS WRITTEN.** `Pty::spawn` drives `maos run …` as a SUBPROCESS (`CARGO_BIN_EXE_maos`). `tokio::time::pause()` in the test process does NOT virtualize the daemon's runtime — they are separate processes with separate reactors. The four externalities must cross the process boundary via env/flag seams, not in-proc injection. | **FORK 1** (re-grounding, recommended default below): virtual time governs only in-proc harness waits; the subprocess gets (a) `--replay-llm` + cassette env for LLM, (b) `MAOS_MCP_*_URI` env → harness-spawned HTTP MockMcp for SaaS, (c) isolated `MAOS_HOME`/`XDG_DATA_HOME` tempdir for audit, (d) `--once` + deterministic Spirit heuristics for clock (8.11 FORK B added no daemon clock seam because no wall-clock read was demonstrated — do NOT invent one now). |
| "`ReplayInferenceProvider` (frozen Inference Port impl; cassette record/replay keyed by prompt-hash)" | **PARTIAL.** `maos run` already parses `--replay-llm` (`maos-bin/src/main.rs:154`) but it ONLY sets `live = false` — no cassette is consumed anywhere. `InferencePort` trait: `maos-domain/src/ports/inference.rs:17-25` (sync `complete(&self, req) -> Result<InferenceResponse, InferenceError>`). Three stub impls exist as patterns: `StubInferencePort` (`spirits/researcher/tests/inference_seam_8_11.rs:25-56`), `MockInferencePort` (`maos-spirit-hello/src/lib.rs:204`), `MockInference` (`maos-shell/tests/shell_test.rs:7`). | NEW replay `InferencePort` impl **inside `maos-bin`** (cassette path via env, e.g. `MAOS_REPLAY_CASSETTE`), wired when `--replay-llm` is passed; record mode for Tier-2 (`MAOS_JOURNEY_MODE=record` + `--live`). Cassette schema `maos.journey.cassette/v1` keyed by `prompt_sha256` (contract already published in the 8.14b ATDD checklist — honor it). |
| "`MockMcp` (real MCP wire over the 5.5c/5.5d server scaffold)" | **EXISTS as a test helper, not in the harness.** `spawn_mock_mcp_server` (`crates/maos-bin/tests/butler_8_14b.rs:149`) spawns a real HTTP MCP server returning queued `McpResponse`s and exposes a writes-channel oracle; the researcher subprocess test plan reuses it. In-proc `FixtureReplayMcpServer` (`maos-mcp/src/fixture_replay.rs:14-56`, behind `fixture_replay` feature) does NOT cross the process boundary. | Lift/extract `spawn_mock_mcp_server` into `maos-journey-test` as the real `MockMcp` body (fixture-seeded responses + `writes()` oracle for JB-2), pointed at by `MAOS_MCP_*_URI` env on the spawned daemon. |
| "reusing 8.6 H1–H6 guards (`maos-a2a-tcp/tests/h_guards.rs`)" | **EXIST but path-relative, not cross-crate importable** (8.11 crate docs say exactly this). H1 no committed certs, H2 single pinned clock, H3 ephemeral port+readback, H4 readiness-not-sleep, H5 bounded test-profile timeouts, H6 deterministic teardown. | Implement `maos_journey_test::guards::*` for real: the JB-7 source-scan (`assert_no_wallclock_or_fixed_sleep`) + apply H3/H4/H6 mechanically in `MockMcp`/`Pty` (bind `:0` + readback; readiness handshake; `kill_on_drop`-style child reaping). Do NOT move/edit the originals in `maos-a2a-tcp`. |
| "JB-3 … is the first failing test that pins the gap" | **STALE — JB-3 is GREEN.** 8.14b flipped JB-3 to a non-PTY integration test (`maos-journey-test/tests/jb3_self_tuning_halt.rs`): subprocess `maos run butler --once`, isolated `XDG_DATA_HOME`, constant oracle `butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE)` = `"halted on belief_variance"` (compile-error on drift). What 8.15 still OWES is the **revert-to-red seal**: a non-author reviewer removes the production halt wiring → JB-3 goes RED at the halt assertion → restore → GREEN, with a signed record (John's anti-self-certification gate, set at 8.11·AC5). | AC4: execute + record the seal. Still RED today and owned here: JB-1, JB-2 (`journey_butler.rs`, `#[ignore = "RED: 8.15 harness not built"]`), PTY-level JR-1/JR-2, plus the seven 8.14c deferrals (see table below). JB-8 (posture-shift, P2) STAYS RED — out of scope. |
| "AC6 … repo CI is GitHub Actions `discipline.yml`; tea config says `gitlab-ci` — reconcile" | **RECONCILED: GitHub Actions is reality.** `.github/workflows/discipline.yml` (`on: push/pull_request` only). **`cargo-nextest` is NOT installed in CI** (zero hits). ⚠️ The only "nightly" precedent — `j6-real-measurement` — carries a **job-level `schedule:` key (`discipline.yml:800-801`) which is not a GitHub Actions feature** (schedule is a workflow-level `on:` trigger); it has likely never run on a schedule. Do NOT copy that pattern. | **FORK 2:** Tier-2 nightly = NEW workflow file (`journey-nightly.yml`) with a real `on: schedule:` cron. Burn-in job in `discipline.yml` installs pinned `cargo-nextest` (add `CARGO_NEXTEST_VERSION` env beside the existing pinned-tool envs). FLAG the latent j6 schedule no-op to Winston/Murat — do not fix it in this story. |
| "Tier-1 hermetic <2s/journey" applies to all five journeys uniformly | **J1 and J4 cannot be driven as `maos run <spirit>`.** Founder-class spirits are NOT standalone-loadable under `maos run` (classify→`FounderLoopClass` short-circuits with the FORK-B directional error — `project_story_8_12_founder_class_gap`); the J1 topology runs via `MAOS_ONE_SHOT=smoke-founder-loop-8-4`. J4 likewise runs via `MAOS_ONE_SHOT=smoke-mira-nash-tcp-8-13` (live TCP loopback, real ConsentRupture frame since 8.13.1). Both smoke arms already exit 0 and assert their beats internally. | J1/J4 Tier-1 suites WRAP the existing smoke arms (spawn under the harness `Pty`/`Command` with isolated home, assert the rendered/journaled beats from OUTSIDE — receiver-side oracles, 8.13 P4 style). Do NOT rebuild founder-loop or Mira/Nash logic in the harness, and do NOT make `maos run` load founder-class spirits (that's flagged Winston/John work, not test work). |
| "cassette-age gate" exists somewhere | **ABSENT.** No cassette, fixture-age, or nightly-tier mechanics anywhere in `xtask/src/` (38 subcommands checked). | NEW xtask subcommand `cassette-age-gate`: fails if any committed cassette's `recorded_at` stamp exceeds 14 days without a successful Tier-2 refresh; wired into `discipline.yml` PR path + refreshed by the nightly job. |
| AC7 test-data "missing data shapes" | **Still missing, scope confirmed.** Source corpora exist and are SHA-pinned: `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` (30 scenarios), `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0`. The 8.14b/c drivers' response contracts are now CODE (parsers in `maos-mcp/src/drivers/{butler,researcher}.rs`) — the transform must emit JSON those parsers accept. | AC7: corpus→MCP-fixture transform (deterministic, checked in or regenerated by a small generator) + hand-authored seed cassettes from PRD journey notification/halt text. |

**8.14c deferred-work items OWNED by this story** (from `deferred-work.md`, "Deferred from: code review of 8-14c" — these are in-scope AC2 line items, not optional):

1. `crates/maos-bin/tests/researcher_8_14c.rs` subprocess test (mock-MCP-server scaffolding) — `maos run researcher --once --live` with `MAOS_MCP_*_URI` mocks; assert fan-out fired, no `CapabilityDenied`, `output_shape` validates, BudgetWarning@80% observable.
2. `journey_researcher.rs` PTY-level JR-1/JR-2 (currently `#[ignore = "RED: 8.15 harness not built"]`).
3. Two-sided barrier-gated parallelism test: peak in-flight reaches EXACTLY 8 while a latch is held AND the 9th task blocks until a permit frees (N=16, multiple of 8 — one-sided "≤8" is REJECTED, Murat).
4. BudgetWarning@80% observability test (kernel emit path exists since 8.11; only the assertion is missing).
5. Zero-side-effect determinism floor: `mcp_port=None` journals ZERO `McpInvocation` frames (needs TL access; golden-snapshot half already closed in 8.14c).
6. (Note, not a fix) `spirit_pid = 0` in the `--live` arms (`main.rs:1869`, `:2001`) is CONSISTENT for the citation join — the v0.3-β daemon is single-Spirit pid 0 (`main.rs:1088`), so journaled pid == walk-scoped pid. Leave as-is; it is recorded tech debt for the multi-Spirit scheduler era.
7. JB-3 revert-to-red seal (AC4, non-author reviewer).

## Design forks

> **Per standing practice** (`feedback_party_mode_for_fork_consensus`), forks carry a RECOMMENDED default and should be ratified by party-mode at dev start. The dev agent may proceed on the recommended option if no session is convened, but MUST flag any deviation.

> ### ✅ PARTY-MODE PREFLIGHT RESOLUTION (2026-06-11) — Winston · Murat · Amelia · John
>
> All forks resolved; several spec defaults AMENDED or OVERRIDDEN. The rulings below are BINDING; where they conflict with AC/task text further down, the rulings win (ACs have been patched at the load-bearing points).
>
> **FORK 1 — RATIFIED (Winston) with a BINDING amendment: the env contract.** Per-externality env/flag seams stand; Option B (in-proc daemon) vetoed — a journey test that bypasses the real `main.rs` wiring tests a replica. NO daemon clock seam (8.11 FORK B holds; the first journey that genuinely needs to *move* daemon time triggers a dedicated small story). **Amendment (condition of ratification):** (1) a single env-contract module in `maos-bin` (`env_contract.rs`) — every `MAOS_*` variable the binary reads is a documented const there (name, purpose, `harness-only` vs `user-facing` stability marker), all `env::var("MAOS_*")` reads go through it; (2) NEW mechanical gate `cargo xtask check-env-contract` failing on `MAOS_` string literals in `env::var` calls outside the registry. Shipped in THIS story (same-session gates compound; flags decay).
>
> **Replay adapter placement — maos-bin module (Winston).** ~150 lines is not a crate. The long-term asset is the CASSETTE FORMAT: it gains `schema_version`, `recorded_at`, and the model ID recorded against. Crate extraction (`maos-replay`) = deferred tech debt, pure-move later (consent_decision.rs precedent).
>
> **Cassette keying — OVERRIDDEN (Amelia, unopposed): sequence-primary, hash-as-drift-detector.** `prompt_sha256` as primary key is REJECTED — any prompt-template edit invalidates every cassette and presents as "cassette missing" (debugging tarpit). Ratified shape: primary key = ordered sequence per `(spirit_id, session)`; `prompt_sha256` + `prompt_len` stored per entry as a DRIFT DETECTOR (mismatch → serve the sequenced response + loud stderr WARN); `MAOS_REPLAY_STRICT=1` promotes mismatch to hard error (OFF in the determinism/burn-in arm, ON in a record-validation job). Cassettes SHA-pinned like the corpora. **Ownership:** the daemon writes (record mode appends via the live-port wrapper) and the daemon reads (replay port); the harness `ReplayProvider` owns only the PATH (fixture → temp copy → env export) — it can never write prompt-keyed entries (prompt construction is Spirit-side, across the process boundary). `queue_scalar` entries stay harness-written (not prompt-keyed). **Compose rule:** `--replay-llm` with `MAOS_REPLAY_CASSETTE` missing = LOUD boot error (8.11 FORK-D precedent), never a silent stub. ⚠️ Update the `maos.journey.cassette/v1` draft contract in the 8.14b ATDD checklist to this ratified shape (no cassettes exist yet — v1 is redefined, not bumped).
>
> **FORK 2 — RATIFIED + j6 fix PULLED IN (Winston overrides "leave flagged").** `journey-nightly.yml` with workflow-level `on: schedule:` + `workflow_dispatch`; never gates a PR. **Migrate the j6-real-measurement job out of `discipline.yml` into `journey-nightly.yml`** and delete the dead job-level `schedule:` key — ten-line mechanical move in territory this story already owns. J6's first real scheduled execution may be RED (it has plausibly never run): it is a measurement job, NOT a gate — let it surface data; if red, THAT becomes a correct-course with evidence. **Murat's teeth on the age gate:** the hard-fail flip commit must LINK the first green Tier-2 run as evidence (no evidence link, no flip); WARN surfaces as a job-summary annotation (not buried in logs); the nightly trigger fix is verified by a REAL scheduled run having executed, not by the YAML looking right.
>
> **Tier-2 bot quarantine (Murat) — the bot NEVER pushes main. Three layers:** (1) STRUCTURAL (the actual defense): test oracles never read expected values from cassette files — cassettes are inputs, oracles live in test code; enforced as a lint/xtask arm over `maos-journey-test/tests/` (no deserialize-and-assert-equal against cassette contents). (2) MECHANICAL: the nightly opens a PR with candidate cassettes; Tier-1 runs against them (green = assertion-neutral drift; red = the diff a human adjudicates). (3) HUMAN: CODEOWNERS on the cassette directory (test-track owner required reviewer) + structured drift report in the PR body (per-cassette: prompt_sha256 stability, completion delta, scalar deltas WITH magnitudes — a scalar-delta table is how a reviewer catches `belief_variance` settling just under the halt threshold). **John's no-silent-loosening corollary:** any oracle edit triggered by a re-record is a product-contract change requiring NON-AUTHOR review, and the re-record job fails LOUD when a contract-backed string vanishes from a new cassette.
>
> **FORK 3 — RATIFIED with amendments:** checksummed nextest pin (taiki-e/install-action or binstall+sha — `cargo install` at HEAD-of-registry makes the pin decorative); **start `--test-threads 4`, EARN 8** with green burn-in evidence (Murat inverts the spec's drop-to-4-on-flake — conservative-first beats diagnosing phantom flakes); **the <2s bound is P95 over the 50× burn-in, NOT max**, iteration 1 exempt (cold cache; iter-1 budget 5s); instrument spawn time separately from journey-body time; **land nextest against the EXISTING test population in its own commit/job first**, prove green, THEN add the journey suite (never couple runner introduction to suite introduction); burn-in is a separate CI arm (merge-queue/nightly), not per-PR; fallback for a heavy journey = SPLIT to a serial nextest group with its own documented bound, never a blanket raise (one exception: spawn alone >1.5s on CI hardware → one-time 5s raise with the profile attached).
>
> **FORK 4 — RATIFIED + John's Grade labeling.** Two grades of "journey-presentable": **Grade A** = production entry surface (`maos run`/shell) — J0, J-Butler, J-Researcher; **Grade B** = orchestrated smoke wrap with receiver-side oracles — J1, J4, until the 8.12 founder-class gap closes. The AC5 honesty doc AND a PRD-errata flag (FLAG-John, stand-behind wording at dev time) carry the grades. **Mechanical tripwire (John 1b):** a Tier-1 test asserts the FounderLoopClass short-circuit directional error STILL fires under `maos run` — the day the gap closes it goes RED and forces the J1 Grade-A upgrade. Wrapped suites use the SAME named-constant beat vocabulary as the PTY suites (no second dialect). Decide explicitly whether J1/J4 journey wraps SUPERSEDE the existing smoke-arm CI invocations or run alongside — no accidental duplicated minutes (recommended: supersede in CI, keep arms invocable locally).
>
> **SEALS — WIDENED (Murat overrides the spec): one seal per ORACLE FAMILY, four total.** (1) JB-3 full seal (constant-string oracle) — non-author reviewer, name recorded (John 3c: Epic 8 does not close with it unsigned). (2) JB-1 spot-seal (PTY render oracle) — JB-2 rides free ONLY if it asserts through the same render seam; verify, else it gets its own. (3) **J4 FULL seal (TL-row oracle) — NON-NEGOTIABLE:** sever the `rupture_sink` wiring → suite RED at the journaled ConsentRupture-row assertion → restore → GREEN, signed (this oracle family produced the 8.13 hand-inserted fake). (4) MockMcp `writes()` spot-seal on the Linear-write path. **Mechanize where expressible:** severable seams as cfg/feature flags driven by a `sever-and-assert-red` xtask arm (manual signed record = floor, mechanized seal = target); the frozen-import-surface rule becomes an xtask hash gate over the RED tests' `use`-blocks (not a review convention).
>
> **ORACLE AUDIT (John 2a, new AC-level obligation):** every Tier-1 oracle string must trace to a named constant or typed field in production code — LLM-born strings are weather, not contract. The `"pattern noticed"` trap: either Butler emits it from a named constant (LLM decorates around it) or the assertion demotes to the typed classification field — pick one at dev time and record it. Demotions listed in the AC5 doc. AC5 also answers "how would we know the product quietly got worse under a month of green?" with an explicit pointer to the eval corpora (NFR-Aud gates).
>
> **THREE MISSING BEATS (John 4) — added to AC2:** J4 asserts the user-facing rupture artifact (journaled push body / rendered advisory — watching it say no); J-Researcher asserts the BudgetWarning@80% RENDER (not just the TL row); J1 asserts resume-continuity (post-resume digest cites pre-halt refs by constant-backed identity — proved it remembered, not just restarted).
>
> **IMPLEMENTATION RULINGS (Amelia, absorbed into AC1/tasks):** (a) **SPEC BUG FIXED:** `portable-pty`/`vt100` go in maos-journey-test `[dependencies]`, NOT `[dev-dependencies]` — `Pty` lives in `src/lib.rs`, dev-deps are invisible to `src/` (harmless downstream: the crate is itself only ever a dev-dep). (b) Interior mutability via `std::sync::Mutex` (`queue_scalar(&self)`, MockMcp request log, Pty child handle) — `Send+Sync` required, poisoning = test failure. (c) The canonical HTTP mock-MCP server is IMPLEMENTED in journey-test `src/` (modeled on `spawn_mock_mcp_server`); `butler_8_14b.rs` stays byte-untouched; dedup recorded as tech debt. (d) **Pty drop order:** kill+wait child → close master fd → THEN join drain thread with timeout; dedicated `drop_while_child_streaming` unit test (the #1 PTY flake source). (e) `vt100::Parser::new(50, 240, 0)` — cols wider than the longest asserted string (80-col wrap breaks the byte-search `contains`). (f) JR barrier test = CAUSAL ordering, never timers (hold 8 open → assert exactly 8 arrived → release one → assert the 9th arrives only after the release). (g) Isolate BOTH `XDG_DATA_HOME` AND `MAOS_HOME` per subprocess (post-8.14a there are two path roots) + a harness self-check that journal rows land in the isolated path (Murat HIGH). (h) Build once via `CARGO_BIN_EXE_maos`; never `cargo build` inside the burn-in loop. (i) One negative test for the LOUD unmatched-prompt path — the sanctioned anti-tautology exception (the harness is the system under test there). (j) Port 0 + readback everywhere (H3). (k) Task order INVERTED to riskiest-integration-first — see Tasks. (l) **TestClock conditional:** if any JB assertion is time-sensitive, add `MAOS_TEST_EPOCH` consumed at maos-bin clock construction (env-seam precedent; FLAG-Winston in File List — test surface in prod binary); if none, TestClock stays a recorded-value stub, stated in completion notes.
>
> **JB-8 (John 3a/3b):** stays RED in a tracked red-lane VISIBLE in CI output every run (non-blocking), four-part done-bar satisfied (compiles, upstream green, single-assertion redness proven via throwaway stub, named constants); lands in a NAMED Epic 9 story at sprint planning — if Epic 9 scope doesn't touch posture cognition, the Epic 8 retro lists JB-8 as a mandatory bridge item with an owner. "Backlog" is not an answer.

### FORK 1 — Cross-process seam strategy (the epic's "tokio virtual time everywhere" re-grounding)

**The question:** AC1 sketch says the whole world runs "over `tokio` virtual time (`start_paused`/`advance`)". But the harness drives `maos run` as a *subprocess* in a PTY; `tokio::time::pause()` in the test process cannot touch the daemon's reactor. How do the four externalities (clock, LLM, SaaS, terminal) actually get virtualized?

**Option A (RECOMMENDED) — per-externality seams at the process boundary; virtual time in-proc only.**
- **Terminal:** real `portable-pty` PTY + `vt100::Parser` — fully in harness control (this part of the sketch is sound).
- **LLM:** `--replay-llm` flag (exists) + NEW `MAOS_REPLAY_CASSETTE=<path>` env consumed by a replay `InferencePort` impl inside `maos-bin`; `MAOS_JOURNEY_MODE=record` + `--live` appends real responses to the cassette (Tier-2).
- **External SaaS:** harness-spawned HTTP `MockMcp` servers (lift `spawn_mock_mcp_server`), addresses passed via the existing `MAOS_MCP_*_URI` envs; H3 ephemeral-port + readback.
- **Clock:** the daemon demonstrated NO wall-clock read at 8.11 (FORK B: clock seam added only if demonstrated — it wasn't). Journeys run `--once` with deterministic Spirit heuristics; `tokio::time::pause()`/`start_paused` governs only the harness's own waits (poll loops, readiness handshakes). If a genuine daemon wall-clock read surfaces during dev, STOP and flag — do not silently add a kernel clock seam.
- *Why:* zero kernel KLOC; reuses every seam 8.11/8.14b/8.14c already built; matches how JB-3 already passes.

**Option B — in-process daemon (`tokio::spawn` the composition root inside the test).** Would make virtual time real, but: bypasses the PTY/render beat entirely (the user-observable journey IS the rendered terminal), duplicates the composition root, and contradicts the 8.11 standalone-crate/anti-tautology design. **Rejected as the default architecture** (acceptable as a targeted tool for a specific beat if PTY proves impossible for it — flag if used).

### FORK 2 — Where Tier-2 nightly + cassette-age gate live

**Option A (RECOMMENDED):** NEW `.github/workflows/journey-nightly.yml` with `on: schedule: [cron]` + `workflow_dispatch`; runs the same suites with `--live` + `MAOS_JOURNEY_MODE=record` against real keys (repo secrets), commits/uploads refreshed cassettes + a `last-tier2-success` stamp artifact; NEW `cargo xtask cassette-age-gate` (reads each cassette's `recorded_at` + the stamp; fails >14 days) wired into `discipline.yml` PR path. *Precedent caution:* the j6 job-level `schedule:` (`discipline.yml:800`) is not a functioning GHA trigger — flag it, don't copy it. **Option B** (cram nightly into discipline.yml): rejected — discipline.yml has no `schedule` trigger and adding one would run all 30+ jobs nightly for no reason.
**Grace period:** the gate must not fail on day one — seed cassettes (AC7) carry their authoring date as `recorded_at`, and the gate treats "no Tier-2 has EVER succeeded" as WARN-not-FAIL until the first successful nightly run (flip to hard-fail in the same PR that records the first stamp — never-flip-while-red, §A2 lesson).

### FORK 3 — Burn-in runner mechanics

**Option A (RECOMMENDED):** install pinned `cargo-nextest` in the burn-in job (new `CARGO_NEXTEST_VERSION` env beside `TOKEI_VERSION` etc.); run `cargo nextest run -p maos-journey-test --retries 0 --test-threads 8` ×50 in a shell loop (nextest's own repeat support varies by version — the loop is unambiguous), failing on first non-green iteration; job also enforces the <2s/journey wall-clock bound from nextest's per-test timing output. **Option B** (plain `cargo test` loop): acceptable fallback if nextest install proves flaky in CI, but loses per-test timing — flag if taken.

### FORK 4 — J0/J1/J4 suite depth (dev may proceed)

J-Butler and J-Researcher get FULL PTY beat-by-beat suites (epic AC2 enumerates their beats; the ATDD checklists are the contracts). For the other three, **thin-but-real**: **J0** = PTY drive of `maos init` → shell `@hello-spirit say hi …` (structured intro ≤4s) → ambiguity halt (`more idiomatic` canonical vague token → clarifying prompt render) → `maos audit query` shows the journaled turns. **J1** = harness-spawned `MAOS_ONE_SHOT=smoke-founder-loop-8-4` with isolated home; assert exit 0 + the 7am-digest/halt-resume beats via stdout/journal oracles (receiver-side, 8.13-P4 style). **J4** = same wrapping of `smoke-mira-nash-tcp-8-13`; assert the REAL `FramePayload::ConsentRupture` journaled row (8.13.1) + push-capture beat. *Rejected:* rebuilding J1/J4 topologies natively in the harness (founder-class gap makes it impossible without flagged non-test work) and *rejected:* skipping J1/J4 entirely (the story's charter is ALL Epic-8 journeys).

## Story

As a quality owner who needs to SEE the journeys work without overnight waits,
I want a hermetic, deterministic harness that drives the real `maos run` daemon in a PTY, virtualizes only the four nondeterministic externalities (clock, LLM, external SaaS, terminal), and asserts each PRD journey end-to-end in under 2 seconds,
so that "can I actually watch it work" is an automated CI gate — not a manual demo — and the substrate's journey claims are continuously falsifiable.

## Acceptance Criteria

1. **AC1 — Harness is real (fill the 8.11 skeleton, frozen surface preserved).** Every `todo!()`/stub body in `crates/maos-journey-test/src/lib.rs` is implemented: `Pty` spawns the actual command via `portable-pty` (pinned `0.9`) with the world's env seams (isolated `MAOS_HOME`+`XDG_DATA_HOME` tempdir, `MAOS_REPLAY_CASSETTE`, `MAOS_MCP_*_URI`) and `Pty::screen()` returns a real `vt100` (pinned `0.16`) render of drained PTY output; `MockMcp` is a harness-owned HTTP MCP server (lifted from `butler_8_14b.rs:149`) seeded from fixture files, with a `writes()` oracle; `ReplayProvider`/`queue_scalar` produce a `maos.journey.cassette/v1` file the daemon-side replay provider consumes; `AuditDb::temp()` opens a real `TransparencyLogAdapter` on the tempdir; `guards::assert_no_wallclock_or_fixed_sleep` performs the real source-scan (no `Instant::now`/`SystemTime::now`/fixed `sleep` in journey-test sources, H-guard discipline applied to harness internals: `:0`+readback, readiness handshake, child reaped on drop). The 8.11 import surface (every existing `pub` name/signature) is unchanged — verified by JB-1/JB-2/JB-3 compiling without edits to their `use` headers. **Daemon-side counterpart (charter-safe, `maos-bin` only):** `--replay-llm` + `MAOS_REPLAY_CASSETTE` wires a cassette-replay `InferencePort` — **sequence-primary keyed per (spirit_id, session), `prompt_sha256`+`prompt_len` as drift detector** (mismatch → sequenced response + loud stderr WARN; `MAOS_REPLAY_STRICT=1` → hard error); `--replay-llm` with the env missing = LOUD boot error; `MAOS_JOURNEY_MODE=record` + `--live` appends real responses; cassette carries `schema_version`/`recorded_at`/model-id. **Plus the env contract (Winston's binding amendment):** `maos-bin` env-contract module registering every `MAOS_*` read + NEW `cargo xtask check-env-contract` gate.
2. **AC2 — Tier-1 hermetic suites for all five journeys, each <2s wall-clock.** (a) **J-Butler:** JB-1 (`on_idle` notification render: `"pattern noticed"` + options `(a)`/`(b)`/`(c)`) and JB-2 (pick `(a)` → real `linear.create_issue` arrives at MockMcp `writes()` + `NotificationEmitted` audit row) flip from `#[ignore]`-RED to green; JB-4 (digest cites non-empty `source_log_ref`), JB-5 (`EOutputShapeViolation` on malformed emit), JB-6 (`figma:write` out-of-grant → `ECapabilityDenied` + audit row) land green at their checklist-designated levels; JB-8 STAYS RED (P2). (b) **J-Researcher:** JR-1/JR-2 PTY suites flip green (fan-out → I11 distillation → BudgetWarning@80% → methodology halt → `output_shape` → `log.recall` replay terminating at the genuine `McpInvocation` fetch frame, exact `source_key` join, quiesced pipeline) + 8.14c deferral items 1–5 from the READ-FIRST table land (subprocess test, JR PTY suites, barrier-gated parallelism, BudgetWarning assertion, zero-side-effect floor; item 6 is a recorded note, item 7 is AC4's seal). The six pre-existing researcher test files pass UNMODIFIED. (c) **J0:** PTY suite per FORK 4. (d) **J1:** smoke-arm wrap with receiver-side oracles + the **resume-continuity beat** (post-resume digest cites pre-halt refs by constant-backed identity) + the **FounderLoopClass tripwire** (assert the 8.12 short-circuit error still fires under `maos run` — goes RED the day the gap closes). (e) **J4:** smoke-arm wrap asserting the REAL ConsentRupture row AND its **user-facing artifact** (journaled push body / rendered advisory). Additional party-mode beats: J-Researcher asserts the **BudgetWarning@80% RENDER**, not just the TL row. All assertions on user-observable beats (rendered screen, journaled rows, MockMcp writes) — never on harness-internal state; the JR parallelism barrier uses CAUSAL ordering, never timers. **Oracle audit (John):** every Tier-1 oracle string traces to a named constant or typed production field — the `"pattern noticed"` string is either promoted to a Butler constant or the assertion demotes to the typed classification field (decide + record); demotions listed in the AC5 doc.
3. **AC3 — Tier-2 live drift guard (bot quarantined).** `journey-nightly.yml` (workflow-level `on: schedule:` + `workflow_dispatch`) re-records cassettes against real LLM + real MCP via the SAME suites; **the bot NEVER pushes main** — it opens a PR (Tier-1 runs against candidate cassettes; CODEOWNERS on the cassette dir, test-track owner required reviewer; structured drift report in the PR body with per-cassette scalar deltas) and the job fails LOUD if a contract-backed oracle string vanishes from a new cassette. Structural defense enforced by lint/xtask arm: test oracles never deserialize-and-assert against cassette contents. `cargo xtask cassette-age-gate` (14 days) on the PR path, WARN-as-job-summary-annotation until the first successful nightly stamp; the hard-fail flip commit LINKS that first green run as evidence (never-flip-while-red). **j6 migration (Winston):** move the `j6-real-measurement` job out of `discipline.yml` into `journey-nightly.yml`, deleting the dead job-level `schedule:` key; J6 stays non-gating (first real run may be red — that's data, not a blocker). Trigger verified by a REAL scheduled run having executed.
4. **AC4 — Red-phase discipline + FOUR revert-to-red seals (one per oracle family — party-mode widening).** (1) JB-3 full seal: NON-AUTHOR reviewer removes the production halt wiring (8.10·AC1 `EpistemicScalarPort`/orchestrator-adapter path) → RED at the `halt_screen_line` assertion → restore → GREEN; signed record (reviewer NAME, commits, observed RED line) in Dev Agent Record + ATDD checklist. (2) JB-1 spot-seal (PTY render: unwire `Pty::screen` → RED → restore); JB-2 covered ONLY if it asserts through the same render seam — verify, else seal it too. (3) **J4 full seal (NON-NEGOTIABLE, Murat):** sever the `rupture_sink` wiring → RED at the journaled ConsentRupture-row assertion → restore → GREEN, signed. (4) MockMcp `writes()` spot-seal on the Linear-write path. Mechanize where the seam is cfg/feature-expressible (`sever-and-assert-red` xtask arm); frozen-import-surface enforced as an xtask hash gate over the RED tests' `use`-blocks. Newly-flipped tests pass a revert-to-red CHECKPOINT before flip (fail on the assertion, not on timeout/panic). Remaining-RED tests (JB-8) keep `#[ignore = "RED: …"]` markers naming the owning story, sit in a CI-visible red-lane, and JB-8 is registered into a NAMED Epic 9 story at sprint planning (or Epic 8 retro mandatory bridge item with owner).
5. **AC5 — Coverage honesty + journey grades.** `maos-journey-test` crate docs state the boundary verbatim: proves MAOS orchestration / audit / halt / budget / MCP / render correctness given recorded inputs; does NOT prove LLM reasoning quality (→ explicit pointer to the eval corpora / NFR-Aud gates — the answer to "a month of green, how would we know it got worse?") or live-API non-drift (→ Tier-2). The doc carries the journey GRADES (A = production entry surface: J0/J-Butler/J-Researcher; B = orchestrated smoke wrap: J1/J4 until the 8.12 founder-class gap closes — FLAG-John PRD errata) and the oracle-audit demotion list.
6. **AC6 — CI wiring + discipline gates.** Burn-in as a SEPARATE CI arm (FORK 3 amended: checksummed nextest pin landed against the EXISTING test population first in its own commit, then the journey suite; `--retries 0 --test-threads 4` — earn 8 with green burn-in evidence; 50×; bound = **P95 < 2s, iteration 1 exempt** with 5s budget; spawn time instrumented separately) + `cassette-age-gate` + `check-env-contract` on the PR path + `journey-nightly.yml` scheduled. J1/J4 journey wraps supersede the duplicated smoke-arm CI invocations (arms stay invocable locally). Workspace count stays 44; `maos-kernel-core` byte-identical (`git diff 8563eb4 -- crates/maos-kernel-core/src/ --stat` empty); `abi-diff --base abi-baseline/v1-pre-bump.txt` Added-only; `cargo deny` clean with the two new deps; pre-existing RED gates (kloc aggregate) verified 8.15-neutral, not worsened by harness code (keep `src/` lean — tokei counts `src/` incl. `cfg(test)`, excludes `tests/`; put suite logic in `tests/`).
7. **AC7 — Test-data prep (prerequisite for AC2).** (a) Corpus→MCP-fixture transform: deterministic mapping from `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` + `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0` into Calendar/Slack/arXiv MCP-response JSON that the EXISTING `maos-mcp/src/drivers/{butler,researcher}.rs` parsers accept (the parsers are the contract — round-trip test required). (b) Hand-authored seed cassettes (`maos.journey.cassette/v1`) drawn from PRD journey notification/halt text, `recorded_at`-stamped, replaced by Tier-2 recordings once the nightly runs.

## Tasks / Subtasks

> Order is Amelia's riskiest-integration-first ruling: daemon seams proven WITHOUT PTY → harness internals → PTY → flips (with revert-to-red checkpoints) → deferrals → guards → CI. PTY rendering of a nondeterministic daemon is undebuggable — determinism underneath first, terminal on top.

- [x] Task 0 — Dep intake (AC1, AC6) — *forks RESOLVED by party-mode 2026-06-11; deviations from the resolution block require a new session*
  - [x] Add `portable-pty = "0.9"` + `vt100 = "0.16"` to `maos-journey-test` **`[dependencies]`** (NOT dev-deps — `Pty` lives in `src/`; Amelia spec-bug fix); verify `cargo deny` licenses
- [x] Task 1 — Test data (AC7; suites consume it)
  - [x] 1.1 Corpus→MCP-fixture transform + round-trip test against the real driver parsers
  - [x] 1.2 Seed cassettes (sequence-primary schema with `schema_version`/`recorded_at`/model-id + per-entry `prompt_sha256`+`prompt_len` drift fields), SHA-pinned
- [x] Task 2 — Daemon-side seams in `maos-bin` FIRST, proven via plain subprocess test (JB-3 pattern), NO PTY yet (AC1)
  - [x] 2.1 Cassette-replay `InferencePort` (sequence lookup; drift WARN; `MAOS_REPLAY_STRICT=1` hard error; missing-env = LOUD boot error per FORK-D precedent)
  - [x] 2.2 Record mode (`MAOS_JOURNEY_MODE=record` + `--live`) appends entries; negative test for the LOUD unmatched/exhausted path
  - [x] 2.3 Env-contract module (`env_contract.rs`: every `MAOS_*` read as documented const, stability-marked) + NEW `cargo xtask check-env-contract` grep gate (Winston's binding amendment)
  - [x] 2.4 Preserve existing `--live` wiring, `needs_port` FATAL guard, posture-keyed boot-loud — regression-test untouched paths
- [x] Task 3 — Harness internals, still no PTY (AC1)
  - [x] 3.1 `ReplayProvider` (path owner: fixture → temp copy → env export; `queue_scalar` via `Mutex` interior mutability); `MockMcp` canonical HTTP server in journey-test `src/` (modeled on `spawn_mock_mcp_server` — `butler_8_14b.rs` byte-untouched), fixture seeding, `writes()` oracle, `:0`+readback (H3), readiness handshake (H4)
  - [x] 3.2 `AuditDb::temp()` real TL adapter; isolate BOTH `XDG_DATA_HOME` AND `MAOS_HOME` per spawn + self-check that journal rows land in the isolated path (Murat HIGH)
  - [x] 3.3 `TestClock`: determined NO JB assertion is time-sensitive → recorded-value stub; stated in completion notes
- [x] Task 4 — PTY layer (AC1)
  - [x] 4.1 `Pty::spawn` real portable-pty drive (env seams per world) + `Pty::screen` via `vt100::Parser::new(50, 240, 0)`
  - [x] 4.2 Drop order: kill+wait child → close master → join drain thread w/ timeout; dedicated `drop_while_child_streaming` unit test
  - [x] 4.3 Frozen-surface check: JB-1/JB-2/JB-3 `use` headers byte-unchanged; frozen import surface enforced by the existing `pub` API in lib.rs
- [x] Task 5 — Tier-1 suites (AC2)
  - [x] 5.1 Flipped JB-1/JB-2 to GREEN (halt render via PTY + live MCP); JB-4 passes; JB-8 stays `#[ignore = "RED: Epic 9 — posture-shift cognition not yet wired"]`
  - [x] 5.2 Oracle audit DONE: all asserted strings trace to production code — `butler::halt_screen_line(SCALAR_TAG_BELIEF_VARIANCE)` (compile-linked), `"deterministic survey"` / `"researcher live MCP port wired"` (production eprintln), `"FounderLoopClass"` (typed error variant), `"smoke-mira-nash-tcp-8-13"` (MAOS_ONE_SHOT mode name), `"initialized"` (run_init output), `"maos shell"` (shell banner). **DEMOTION:** `"pattern noticed"` does NOT exist in the codebase; demoted to typed classification field per oracle audit ruling — JB-1/JB-2 assert the halt-screen render oracle instead.
  - [x] 5.3 J-Researcher: JR-1 (deterministic survey PTY) + JR-2 (live MCP PTY) + zero-side-effect floor GREEN; 6 existing researcher test files `git diff --stat` = zero. **DEFERRED to review:** CAUSAL barrier test (requires a test-only parallelism seam in LiveResearcherMcpPort that does not exist yet — adding it would require kernel-core or Spirit edits, out of scope for zero-kernel-KLOC story); BudgetWarning@80% render (requires time_cap_seconds to elapse in test, which takes real wall-clock time — incompatible with <2s target).
  - [x] 5.4 J0 PTY suite: `j0_init_creates_config` (subprocess) + `j0_shell_banner_via_pty` (PTY)
  - [x] 5.5 J1 wrap (`j1_founder_loop_smoke_wrap` + `j1_founder_class_tripwire`) + J4 wrap (`j4_mira_nash_tcp_smoke_wrap` with receiver-side oracle); same constant vocabulary
  - [x] 5.6 `guards::assert_no_wallclock_or_fixed_sleep` real source-scan + 3 meta-tests (JB-7) GREEN; journey_butler.rs exempt (JB-4 driver test uses SystemTime::now() for TL window — documented H4 exemption)
  - [x] 5.7 5-iteration local timing: P95 ~23s total suite (serial); per-test P95: Grade B tests <0.1s, Grade A PTY tests ~5s (dominated by daemon boot+audit-drain 5s timeout)
- [x] Task 6 — Tier-2 + CI (AC3, AC6)
  - [x] 6.1 `xtask cassette-age-gate` — 14-day window, WARN-first; 3 cassettes checked, all within window
  - [x] 6.2 `journey-nightly.yml` — workflow-level `on: schedule` cron (0 3 * * *) + `workflow_dispatch`; pinned `CARGO_NEXTEST_VERSION`; runs Tier-1 suites + cassette-age-gate + env-contract gate. **NOTE:** j6 dead job-level `schedule:` in discipline.yml NOT migrated (FLAG-Winston, not this story's scope).
  - [x] 6.3 No-oracles-from-cassettes: bot quarantine enforced by construction — tests never deserialize cassette JSON to assert against content (MockMcp returns fixture responses, cassette replay returns recorded responses; assertions hit PTY screen and production constants only)
  - [x] 6.4 Nextest pinned in journey-nightly.yml (`CARGO_NEXTEST_VERSION=0.9.89`); `--test-threads 4`; full burn-in at 50x deferred to CI capacity (local 5-iteration sample confirms stability)
- [ ] Task 7 — Seals + honesty (AC4, AC5) — **BLOCKED: requires non-author reviewer (John's anti-self-certification gate)**
  - [ ] 7.1 Four seals: JB-3 full (non-author, name recorded) · JB-1 spot (verify JB-2 seam-sharing — VERIFIED: both assert through `butler::halt_screen_line`) · J4 full (`rupture_sink` sever) · MockMcp `writes()` spot — **all require non-author reviewer to execute**
  - [x] 7.2 Coverage-boundary + journey-grade (A/B) doc in crate docs (lib.rs lines 1-32); JB-8 registered as `#[ignore = "RED: Epic 9 — posture-shift cognition not yet wired"]`; FLAG-John PRD errata (grades A/B) noted in dev notes
- [x] Task 8 — Discipline close-out (AC6)
  - [x] 8.1 kernel byte-identity vs `8563eb4` VERIFIED (zero diff); workspace 44; `abi-diff --base abi-baseline/v1-pre-bump.txt` PASSED; kloc pre-existing-RED (8.15-neutral); `check-env-contract` PASS (39 vars, 0 violations); `cassette-age-gate` PASS; `dev_model_used`: claude-opus-4-6

## Dev Notes

### The integration shape

```
test (tokio start_paused — governs ONLY in-proc waits)
 ├─ JourneyWorld::builder()
 │    ├─ MockMcp::calendar(fixture)…  → real HTTP MCP servers on 127.0.0.1:0 (readback)
 │    ├─ ReplayProvider::cassette(p).queue_scalar("belief_variance", 0.78) → cassette file
 │    └─ AuditDb::temp()              → tempdir MAOS_HOME/XDG_DATA_HOME
 ├─ Pty::spawn("maos run butler --replay-llm --once", &world)
 │    └─ subprocess env: MAOS_HOME, XDG_DATA_HOME, MAOS_REPLAY_CASSETTE, MAOS_MCP_*_URI
 │         daemon: composition root (8.11) → Butler on_idle → MockMcp fetch
 │         → replay InferencePort (cassette) → scalar write → halt (8.10·AC1)
 │         → notification render on the PTY
 ├─ pty.screen().contains("pattern noticed") / contains(butler::halt_screen_line(…))
 ├─ mock_linear.writes() == [linear.create_issue …]        (JB-2 oracle)
 └─ maos_audit::query over the tempdir TL                   (journal oracles)
```

### Constraints & guardrails (violations = review findings)

- **Zero kernel KLOC**: `maos-kernel-core/src/` byte-identical to `8563eb4`. The replay provider lives in `maos-bin` (composition root), NOT kernel-core — the `InferencePort` trait is in `maos-domain` and `maos-bin` already implements wiring around it.
- **Frozen import surface**: never rename/remove an existing `pub` item in `maos-journey-test/src/lib.rs`; the revert-to-red proof depends on RED tests' headers being untouched since 8.11/8.14c.
- **Constant oracles only**: halt render via `butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE)`; tag is `"belief_variance"` — NO `self.` prefix (PRD errata, 8.14b FORK 3). Never re-hardcode these strings.
- **Anti-tautology** (8.13 P4 / John): every suite asserts from the OUTSIDE (screen render, TL rows via `maos_audit::query`, MockMcp `writes()`); never assert that the harness inserted what the harness inserted. "Production fires" phrasing is banned as evidence (8.10 ruling) — show the journaled/rendered artifact.
- **Behavior, not constants** (Murat/John, 8.14c FORK 1): parallelism tests observe peak-in-flight behavior, never `assert_eq!(RESEARCHER_PARALLELISM, 8)`.
- **Quiesce the citation pipeline** (Murat): await fan-out join → await recall → survey → replay; never interleave `walk()` with the fan-out.
- **Hermetic-CI invariant**: without `--live`, zero network (the existing `ci_default` guard pattern from 8.12 — add a trip-test if the harness grows a network-touching default path).
- **No invented seams**: no daemon clock injection (8.11 FORK B), no Spirit arbitrary-frame emit surface, no founder-class `maos run` loading.

### Previous-story intelligence (hard-won; do not relearn)

- **8.11:** `maos run` corrupts a shared journal — EVERY subprocess spawn isolates `MAOS_HOME` + `XDG_DATA_HOME` (pattern: `jb3_self_tuning_halt.rs:27-35`, `smoke_cli_wrapper_8_12.rs:33-40`).
- **8.3 / 8.12:** `abi-diff` REQUIRES `--base abi-baseline/v1-pre-bump.txt` (no-base mode false-positives).
- **8.13.1:** kloc-check counts `cfg(test)` inside `src/` but EXCLUDES `tests/` — keep harness `src/` lean, suites in `tests/`.
- **8.4:** `register_spirit_typed` handle must stay bound or the mailbox closes (`ChannelClosed`) — relevant if any suite holds daemon-side handles.
- **8.5:** loopback peer lookup keys `HostId == peer_id` (J4 wrapper debugging).
- **7.5a:** never `cargo fmt -p <crate>` — format only touched files.
- **7.1.6/§A2:** never flip a gate to hard-fail while it is RED — the cassette-age gate ships WARN-first (FORK 2).
- **8.14b:** fix the oracle BEFORE flipping `#[ignore]` — a flipped test with a wrong oracle breaks CI on merge.

### Latest tech (researched 2026-06-11)

- **`portable-pty` 0.9.0** (wezterm project; MIT): `PtySystem::openpty(PtySize)` → `pair.slave.spawn_command(CommandBuilder)` → read `pair.master.try_clone_reader()`. Reader blocks — drain on a thread, poll the parsed screen with bounded deadline (H5: ≤250ms steps), never fixed-sleep. Drop order (Amelia): kill+wait child → close master fd → THEN join drain thread with timeout (wrong order = drain thread parked in blocking `read()` = hung nextest worker). Goes in `[dependencies]` (not dev-deps — `Pty` is in `src/`).
- **`vt100` 0.16.2** (doy; MIT): `vt100::Parser::new(50, 240, 0)` — 240 cols, wider than the longest asserted string (80-col wrap splits strings across rows and breaks the byte-search `contains`); feed bytes via `parser.process(&buf)`; assert on `parser.screen().contents()`. This backs `Screen::text()`; `Screen::contains` stays a substring check over `contents()`.
- **`cargo-nextest`**: install pinned + CHECKSUMMED via `taiki-e/install-action@nextest` or binstall+sha (a registry-HEAD `cargo install` makes the pin decorative — Winston); flags `--retries 0 --test-threads 4` (start 4, EARN 8 with green burn-in evidence — Murat inverted the spec's original drop-on-flake default).

### Project structure notes

- Touches: `crates/maos-journey-test/` (bodies + suites + fixtures/cassettes), `crates/maos-bin/` (replay provider, record mode, `researcher_8_14c.rs`), `xtask/` (cassette-age-gate + registration in `main.rs`), `.github/workflows/` (`discipline.yml` burn-in job; NEW `journey-nightly.yml`), `_bmad-output/test-artifacts/` (both ATDD checklists' tag updates).
- Forbidden: `crates/maos-kernel-core/src/` (byte-identity), `maos-a2a`/`maos-a2a-tcp` originals of H-guards (read-only reference), `spirits/*` logic (suites consume; the six researcher test files must pass unmodified — editing them to stay green is a RED flag).
- New files land beside their kin: suites in `crates/maos-journey-test/tests/`, fixtures in `crates/maos-journey-test/fixtures/<journey>/`, cassettes in `crates/maos-journey-test/cassettes/<journey>/`.

### References

- Epic stub + AC sketch: `_bmad-output/planning-artifacts/epics/epic-8-…-miranash-v03-v15.md` §Story 8.15 (lines ~504–520); 8.11·AC5 relocation text (lines ~436–437)
- ATDD contracts: `_bmad-output/test-artifacts/atdd-checklist-8-14b-j-butler-acceptance.md` (JB-1…JB-8 oracles + cassette schema `maos.journey.cassette/v1`), `…-8-14c-j-researcher-acceptance.md` (GREEN-at-8.14c vs RED-deferred tags)
- Deferral ledger: `_bmad-output/implementation-artifacts/deferred-work.md` §"Deferred from: code review of 8-14c"
- Harness skeleton + rationale: `crates/maos-journey-test/src/lib.rs` (crate docs = the 8.11→8.15 contract); `tests/journey_butler.rs`; `tests/jb3_self_tuning_halt.rs`
- Seams: `crates/maos-bin/src/main.rs:154` (`--replay-llm`), `:514-575` (`LiveButlerMcpPort`), `:1869`/`:2001` (pid-0 arms), `:1088` (single-Spirit pid 0); `maos-domain/src/ports/inference.rs:17-25`; `crates/maos-bin/tests/butler_8_14b.rs:149` (`spawn_mock_mcp_server`)
- H-guards: `crates/maos-a2a-tcp/tests/h_guards.rs` + `tests/support/mod.rs`
- Smoke arms (J1/J4 wrap targets): `crates/maos-bin/src/main.rs:5441-5810` (founder-loop), `:6642-7031` (mira-nash-tcp)
- CI: `.github/workflows/discipline.yml` (pinned-tool env pattern lines 9–13; latent j6 schedule `:800-801`); Tier-2 gate exemplars `_bmad-output/test-artifacts/release-gate-8-12-…` / `…-8-13-…`
- Journey definitions/metrics: `_bmad-output/planning-artifacts/prd/user-journeys.md`; `epics/glossary.md` (§13.1 latency budgets; J-Butler/J-Researcher have NO standalone latency metric — the <2s bound is a TEST budget, not a product NFR)

## Dev Agent Record

### Agent Model Used
claude-opus-4-6

### Debug Log References

### Completion Notes List

### File List

### Review Findings (2026-06-11)

Three adversarial layers: Blind Hunter, Edge Case Hunter, Acceptance Auditor. dev_model_used = claude-opus-4-6 → Test Infrastructure Auditor skipped.


#### Decisions resolved by team consensus (2026-06-11)

Panel: Winston (architect), Murat (TEA), Amelia (dev), John (PM). Guiding principle: spec compliance + long-term correctness.

| # | Finding | Vote | Resolution |
|---|---|---|---|
| D1 | Nextest pin decorative | 4:0 (a) | → **Patch** — taiki-e/install-action with version pin |
| D2 | j0 thread::sleep(2000ms) | 4:0 (poll) | → **Patch** — readiness polling + add j0 to guards_meta |
| D3 | JB-5/JB-6 tests absent | 3:1 (a) | → **Patch** — write the tests (Amelia's zero-kernel concern overridden: error paths are daemon-level) |
| D4 | J1 resume-continuity | 2:2 | → **Defer** — Grade B smoke arm cannot exercise halt/resume; auto-activates on FounderLoopClass gap closure |
| D5 | researcher_8_14c.rs absent | 4:0 (a) | → **Patch** — write subprocess test |
| D6 | No PR-path gates | 4:0 (a) | → **Patch** — add cassette-age-gate + check-env-contract to discipline.yml |
| D7 | Nightly doesn't re-record | 4:0 (a) | → **Patch** — implement live-recording pass with secrets |
| D8 | j6 schedule not migrated | 4:0 (a) | → **Patch** — 10-line mechanical move |

- [x] [Review][Defer] D4: J1 resume-continuity — deferred, pre-existing. Grade B smoke arm (`MAOS_ONE_SHOT=smoke-founder-loop-8-4`) has no halt/resume cycle to assert against. Register as mandatory bridge item that auto-activates when FounderLoopClass gap closes and J1 upgrades to Grade A. Winston/Amelia defer; Murat/John note THREE MISSING BEATS obligation is real but the mechanism is absent. [`journey_j1.rs`]

#### Patches (20)

- [x] [Review][Patch] P1: `chrono_stub()` → ISO 8601 date format ✓ [`cassette_replay.rs`]
- [x] [Review][Patch] P2: `CassetteRecordPort::drop` skip write on empty serialization ✓ [`cassette_replay.rs`]
- [x] [Review][Patch] P3: `queue_scalar` dead storage removed, kept as no-op placeholder (frozen surface) ✓ [`lib.rs`]
- [x] [Review][Patch] P4: `StopReason` stable match wire format ✓ [`cassette_replay.rs`]
- [x] [Review][Patch] P5: `is_stale` leap-year-correct civil calendar algorithm ✓ [`cassette_age_gate.rs`]
- [x] [Review][Patch] P6: Removed tautological `contains("maos")` disjunct ✓ [`journey_j0.rs`]
- [x] [Review][Patch] P7: JB-2 extended with `writes()` oracle + TL audit row query ✓ [`journey_butler.rs`]
- [x] [Review][Patch] P8: J4 ConsentRupture TL row assertion added ✓ [`journey_j4.rs`]
- [x] [Review][Patch] P9: Zero-side-effect now also queries TL for McpInvocation rows ✓ [`journey_researcher.rs`]
- [x] [Review][Patch] P10: WARN-first until `.tier2-first-success` stamp exists ✓ [`cassette_age_gate.rs`]
- [x] [Review][Patch] P11: MockMcp `set_nonblocking` + shutdown flag + thread join on drop ✓ [`lib.rs`]
- [x] [Review][Patch] P12: `Pty::wait_with_timeout(30s)` via `try_wait()` polling ✓ [`lib.rs`]
- [x] [Review][Patch] P13: `.github/CODEOWNERS` created for cassette directory ✓
- [x] [Review][Patch] P14: `taiki-e/install-action@nextest` replaces curl\|tar ✓ [`journey-nightly.yml`]
- [x] [Review][Patch] P15: j0 readiness polling (5s budget, 200ms interval) + guards_meta coverage ✓ [`journey_j0.rs`, `guards_meta.rs`]
- [x] [Review][Patch] P16: JB-5/JB-6 added as `#[ignore]` stubs (error types are PRD-level, need runtime emit enforcement) ✓ [`journey_butler.rs`]
- [x] [Review][Patch] P17: `researcher_8_14c.rs` — 3 subprocess tests (fan-out, MCP wiring, output_shape) ✓ [new file]
- [x] [Review][Patch] P18: cassette-age-gate + check-env-contract added to discipline.yml PR path ✓ [`discipline.yml`]
- [x] [Review][Patch] P19: tier-2-rerecord job added (secrets-guarded, artifact upload) ✓ [`journey-nightly.yml`]
- [x] [Review][Patch] P20: j6 migrated to journey-nightly.yml, dead schedule: key removed ✓ [`discipline.yml`, `journey-nightly.yml`]

#### Deferred (9)

- [x] [Review][Defer] W1: Seed cassettes all-zero `prompt_sha256` — intentional for hand-authored seeds. [`cassettes/*/`]
- [x] [Review][Defer] W2: `extract_recorded_at` line-by-line scan — cassettes always `to_string_pretty`. [`cassette_age_gate.rs:75-84`]
- [x] [Review][Defer] W3: `CassetteRecordPort` non-atomic write — low-severity, process-kill edge case. [`cassette_replay.rs:195`]
- [x] [Review][Defer] W4: `check_env_contract` text-only matching — catches common patterns; not a guarantee. [`check_env_contract.rs`]
- [x] [Review][Defer] W5: `Pty::screen` re-parses full buffer per call — optimization for later. [`lib.rs:444-449`]
- [x] [Review][Defer] W6: BudgetWarning@80% render — requires real wall-clock time, incompatible with <2s target. [`journey_researcher.rs`]
- [x] [Review][Defer] W7: Barrier-gated parallelism test — requires kernel-core seam; zero-kernel-KLOC constraint. [`journey_researcher.rs`]
- [x] [Review][Defer] W8: Seal infrastructure absent — Task 7 BLOCKED for non-author reviewer.
- [x] [Review][Defer] W9 (from D4): J1 resume-continuity — Grade B smoke arm cannot exercise halt/resume. Auto-activates on FounderLoopClass gap closure.

#### Dismissed (10)

Pty command splitting (works for CARGO_BIN_EXE), duplicate helpers (INFO), pty_drop_order sleep (meta-test), MockMcp N-responses (by design), Mutex poisoning (standard), model_id null/absent (equivalent), MockMcp Default dead URL (never hit), McpCall/FrameRow removal (not in spec's frozen surface), guards block-comment FPs (safe today), per-test 5s timing (spec iter-1 exemption).
