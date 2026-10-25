---
story_id: '18.3'
story_key: 18-3-researcher-replay-live-and-judged-five-metric
created: 2026-10-25
context_commit: ee395748
baseline_commit: ee395748
security_sensitive: true
kernel_grant: "none — kernel-Δ **0**, measured at creation (rule 5 / E16-A7) by the AC3 spike: one throwaway test file (`crates/maos-kernel-core/tests/zz_spike_paused_time_18_3.rs`, 160 code lines) drove the UNCHANGED dispatcher under tokio paused time, ran 3/3 green, and was deleted. With the spike present `check-kernel-baseline` PASSED 26377 / 95 files and `kloc-check --json` kept `maos-kernel-core` at 20608 (test files are excluded, `xtask/src/kloc_check.rs:302-303`); after the revert `git status` was clean and `check-kernel-baseline` PASSED 26377 / 95 again. The epic's conditional ◆ (+15–30, dispatcher `Clock` trait) is discharged; no FLAG-Winston row. AC3's plant is a temporary kernel edit, restored. If the kernel figure moves at landing, the story stops and returns a FLAG-Winston D-row."
kloc_grant: "RULED 2026-10-25 (operator, Lunarpulse): D-18-3-11 B — **Epic 18's named maos-bin budget raised +385 → +635 (27809 → 28444)**; 18-3 lands at its exact measured figure (re-pin after `cargo fmt --all` via `kloc-check --json`); a landing figure above 28444 → re-ask before landing. 18-4 draws on the same remainder. maos-eval and maos-bench (zero headroom) are exact measured requests at landing; `_aggregate_hardfail` = measured + 1; kernel 0 (any kernel line stops the story for a FLAG-Winston row). Measured at creation, `kloc-check --json` (tokei 14.0.0) at ee395748: maos-bin 28094 / 28094; maos-eval 4271 / 4271 (zero headroom; 15-2 ledger `xtask/kloc.toml:672-673` books 'Epic 18 halt-eval driver and judge harness +450–900', 18-2 used +230); maos-bench 2032 / 2032 (zero headroom, its 15-2 reservation consumed by 18-2, `xtask/kloc.toml:519`); maos-journey-test 915 / 1104; maos-cli 6104 / 6856; maos-registry 3452 / 4282; maos-providers 1392 / 2000; maos-kernel-core 20608 / 20608; aggregate 183247 (`_aggregate_hardfail` 183248, `xtask/kloc.toml:718`). Forecasts [INFERENCE, no maos-bin prototype]: maos-bin +90–165, maos-eval +150–300, maos-bench +100–200, maos-journey-test +0–10, the rest 0. `spirits/*` is ungoverned (`xtask/src/kloc_check.rs:310-311`)."
review: "bmad-code-review under `docs/testing/proof-contract.md` (focused suites + enrolled gates; workspace once in CI). Dev and reviewer model `anthropic/claude-opus-5-5` ONLY: operator ruling 2026-10-25, fable declined (including for AC5's security layer). **Security-critical: AC5** (which provider a Researcher token names, keyless explicit replay without rerouting, boot refusal for an unregistered declared provider, per-pass mint, door/successor binding) — the review record names a security-focused Security layer for it, run on opus-5-5. No kernel diff, so no FLAG-Winston sign-off unless landing measures one."
---

# Story 18.3: Researcher typed replay/live exits, hermetic five-metric, nightly judge and the AC3 spike

Status: review

<!-- Created 2026-10-25 at ee395748 by bmad-create-story under the lighter creation contract (no party-mode preflight); 18-1, 18-8 and 18-2 were treated as landed (their combined landing CI was running). The host's only cargo process was used for measurement (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`). The HEAD `maos`/`maosctl` binaries (built 13:56, HEAD committed 13:57) were run for the exit-line-3 and TIA-6 probes. One throwaway kernel test (the AC3 spike) was written, run, measured and deleted. RULED 2026-10-25 by the operator (Lunarpulse): D-18-3-01 A, 02 B, 03 A, 04 A, 05 A, 06 A, 07 A, 08 B, 09 A, 10 A, 11 B (Epic 18 named maos-bin budget → 28444); review opus-5-5 only → `ready-for-dev`. -->

> **The capability:** *the Researcher's model path is real or it refuses. Exit line 3 replays its recorded completions through the kernel Inference Port on a keyless box, naming a provider the Researcher declares; an absent, exhausted or drifted cassette — or any provider error — is a typed non-zero exit, never a silent `summarize` fallback. Its distillates are scored on all five NFR-Aud-7 metrics over a frozen corpus in CI, the live judge refreshes that corpus nightly, and the Researcher's 60 s time cap is proven under paused time without touching the kernel.*

**Closes:** NFR-Aud-7 for the distillation-shipping classes (`prd/non-functional-requirements.md:62-67`; coverage row `tests/coverage-matrix.yaml:981-985`, `gates: [] corpora: []` today) — scope per D-18-3-08. NFR-Perf-6 (`prd/non-functional-requirements.md:14`; row `:1378-1382`, empty today). Exit line 3 (epic `:16`; red at HEAD, measured). J-Researcher's routing-audit widening (`routing-audit-2026-10-22.md:37,111`). `deferred-work.md:1212` (keyless explicit replay refused; AC5) and `:1247` (TIA-6: no CI step reads the 18-2 sidecar through `maosctl`; AC1). The 18-1 review lesson (a Spirit bound to the Inference Port is bound on every construction path; refused passes are typed; an unregistered declared provider refuses at boot) applied to the Researcher (AC5).

**Does NOT close, and says so:**
- **Live model evidence in CI.** CI replays committed completions and frozen judge verdicts; while their provenance is `seed`, every published number says so (rule 11). Live recording and the live judge are the operator lane (`ops-nightly-cassette-rerecord`, `sprint-status.yaml:332`).
- **The NFR-Aud-8 quarterly N=500 slice** (`crates/maos-eval/tests/distillate_five_metrics_floor.rs:142-272`) keeps its annotated means; only the N=100 per-commit floor (`:26-136`) is replaced.
- **The `--live` Researcher's MCP ports on a door-loaded or hot-swapped successor** (collective, MCP and log-recall ports are attached only by `maos run`'s boot arm, `main.rs:883-891`). D-18-3-04 binds the successor's *inference*; its MCP fan-out stays boot-arm-only and is labelled.
- 18-4's `Clock` port (the spike shows none is needed in the dispatcher), 18-6's re-distillation trigger, the Orchestrator halt class (`19-1` AC5).

## Story

As **the director deciding whether a Researcher survey can be trusted**,
I want **the Researcher to reason through the Inference Port it declares — replayable on a keyless box, refusing loudly when the model path fails — and its distillates scored on all five NFR-Aud-7 metrics in CI**,
so that **a survey is never a silent truncation of its sources dressed as a model summary, the five floors are measured on what the Researcher actually emits, and the 60 s budget envelope is proven deterministically.**

**Contract:** epic-18 §18-3 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:153-165`, corrected in place at creation `:157`, rulings `:159`), exit line 3 and its provenance (`:16`, `:24`), Kernel-Δ (`:32`, 18-3 spike sentence), kloc asks (`:36`, 18-3 ruling `:38`), §R R18-13, R18-14, R18-15 (`:64-66`), R18-22 (`:73`, AC4 widening), R18-23 (`:74`, AC5 carry), R18-P2 (`:81`), confidence rules 9–11 (`:40-42`), Dependencies (`:230-232`); `routing-audit-2026-10-22.md:37,109,111`; `deferred-work.md:1212,1247`; ADR-064 (`docs/adr/ADR-064-one-inference-mode-selector.md:67-84`); `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `ee395748` by direct reads of every cited range, runs of the HEAD binaries under a scratch `HOME` with no provider key, `kloc-check --json`, `check-kernel-baseline`, and the AC3 spike.

| Premise (epic / §R) | Fact at `ee395748` and consequence |
|---|---|
| Exit line 3 "HEAD: exit 0 … the empty-cassette twin exits 0 because the error falls into `summarize(statement)`" (epic `:24`) | **Measured: exit 1** on the CI-shaped runner (`MAOS_OLLAMA_URL` unset, as `epic-18-exit` does at `discipline.yml:2255`): `Error: "maos run: token issue failed: kernel policy denied capability issuance"`. The default-URL Ollama registers and becomes the router default (`main.rs:3780-3798`); the explicit arm names it (`:5774-5777`); the binding mints it (`:5972-5987`); 17-11 refuses it because the Researcher declares `anthropic`+`openai` only (`spirits/researcher/manifest.toml:49`). **With `MAOS_OLLAMA_URL=skip`** both the committed cassette and an empty one (`"entries":[]`) exit **0**, as does the legacy (mode-unset) arm: `on_idle_fired` in 24–28 µs, because `maos run` builds `Researcher::new()` (`main.rs:5603`) with no frames and no MCP port off `--live`, so `on_idle` (`spirits/researcher/src/lib.rs:422-490`) never calls `synthesize`. The fallback is never reached; **no completion is consumed**. → AC1 needs a survey input for the bound non-`--live` pass (D-18-3-01) and a consumed-count assertion, not only the fallback removal. Corrected in place. |
| The committed cassette is consumed by `journey_researcher.rs:69-70` | `jr2_live_mcp_fan_out_via_pty` (`crates/maos-journey-test/tests/journey_researcher.rs:49-100`, cassette `:65-66,73`) runs `--live`; with the mode unset that resolves `Live { explicit: false }` (`crates/maos-bin/src/inference_mode.rs:35-46`), which ignores `MAOS_REPLAY_CASSETTE`. **Nothing consumes `survey-distill.json` at HEAD.** Its single entry has the wildcard hash (`survey-distill.json:11`), which skips the drift check even when strict (`crates/maos-bin/src/cassette_replay.rs:151-152`). |
| AC1: "`Unconfigured`/`CapabilityDenied` logged `:948-953`, every other error silent `:954`" | Holds, and every **replay** error is in the silent arm: `CassetteReplayPort` maps exhaustion/drift to `InferenceError::ProviderTransport` (`cassette_replay.rs:171-174`). The shared-router replay provider goes through the kernel adapter (`:192-210`). |
| AC1 regression surface (not cited by the epic) | `jr2` asserts exit 0 (`journey_researcher.rs:89-93`) and `researcher_8_14c_mcp_fanout` asserts `status.success()` (`crates/maos-bin/tests/researcher_8_14c.rs:279-281`). Both run `--live` on a zero-config box whose only provider is the `anthropic` `UnconfiguredProvider` (`main.rs:3800-3804`): MCP fan-out produces claims, `complete` returns `Unconfigured`, and they pass **only through the fallback**. They must flip to asserting the typed refusal after the fan-out, as 18-1 did for `butler_8_14b` (`crates/maos-bin/tests/butler_8_14b.rs:584-609`). ADR-064 forbids `replay` with `--live` (`inference_mode.rs:55`; ADR `:73-74`), and the Anthropic base URL is hard-coded (`main.rs:3742-3746`), so no hermetic live completion exists for a process-level `--live` Researcher. → CR-4. |
| AC5: "the explicit arm takes the router default `main.rs:5510-5514`; binding `:5729-5747`; adapter `inference/mod.rs:256-262`" | Explicit arm `main.rs:5770-5786` (default `:5774-5777`); **the `--live` arm also mints the router default** (`:5757-5762`); legacy arm mints the first declared provider (`:5807-5809`, 17-11 review). One token minted at admission with a 60 s TTL (`:5985`, mint `:3964-3981`), so a daemon Researcher's passes after 60 s are `CapabilityDenied` (D-18-3-03). The request sends `provider_id: None` (`spirits/researcher/src/lib.rs:938`), and the adapter resolves `req.provider_id.or(default)` (`crates/maos-kernel-core/src/inference/mod.rs:256-261`, check `:263`). 18-1 already registered the replay driver under every interned id (`main.rs:3857-3865`), so a request that names `anthropic` replays keylessly. Corrected in place. |
| 18-1 review lesson: every construction path binds the seam (D-18-1-R1) | `build_spirit_obj` (`main.rs:898-979`; callers: the hot-swap successor factory `:3346` and the door `load` `:9302`) builds `::researcher::Researcher::new()` (`:930`) — **unbound**. Under `--live`/explicit mode a door-loaded or hot-swapped Researcher silently runs the deterministic path. → D-18-3-04. |
| AC2: replace the label means over the N=100 corpus with deterministic string metrics | Job `discipline.yml:1487-1500` (was `:1449-1461`); floor asserts `distillate_five_metrics_floor.rs:75-86` (was `:76-88`). The N=100 corpus is hand-authored stubs: `source_raw_frames[].payload_summary` = `"Raw frame 1"` (`crates/maos-eval/fixtures/distillate-corpus-v0/scenario-001.json`), so recall of source claims cannot be computed over it. A **new** frozen Researcher corpus is required. `ClaimPayload` already carries `hedges` (`spirits/researcher/src/lib.rs:190-210`), copied verbatim into `Finding.hedges` (`:804-816`), so hedge preservation in the model's statement text is a deterministic string metric over frozen completions. Corrected in place. |
| AC2: "the judge harness belongs to the nightly lane" (`grep -rn judge crates/maos-eval/src` = 0) | Holds. But a judge contract exists since Story 0.3: `tests/judge-config.toml` (`[judge]` empty; six constraints: pinned model, temperature 0.0, top_p 1.0, seed, retry_budget 1, `prompt_version_hash`), enforced by `check-judge-config` (`xtask/src/check_judge_config.rs:67`; job `discipline.yml:739-751`), which also rejects direct provider-call identifiers in tests (`xtask/judge-direct-call-identifiers.toml`). The judge must route through the Inference Port and commit its row (CR-6; NFR-Test-1, `prd/non-functional-requirements.md:78`). |
| AC4: "every distillation-shipping class" | Production distillate writers at HEAD: **only the Digest class** (`maos_digest::persist_team_digest` via `render_j3_digest_scene`, `main.rs:501`, `--once` arm `:6098-6123`). The Researcher's `distill_through` (`spirits/researcher/src/lib.rs:1015-1025`) has no production caller; the other `write_distillate` calls are smoke one-shots (`main.rs:6751`, `:10039`). → D-18-3-08. |
| AC4: "contradiction halt/resume" | The Researcher's two halt rules (`spirits/researcher/manifest.toml:120-131`) are **not in the halt-port set** (`requires_epistemic_halt_port`, `main.rs:512-527`, tags `:523-525`), so the boot arm attaches no scalar port and production never writes `methodology_conflict`/`load_bearing_confidence` (18-2 recorded this as the Researcher class's `model_driven: false` reason, `crates/maos-bench/src/halt_eval.rs:108-111`). A declared halt that production never evaluates is a rule-11 (a) gap. → D-18-3-09. |
| AC4: "eight-way dispatch" | `RESEARCHER_PARALLELISM = 8` (`spirits/researcher/src/lib.rs:81`), enforced by `Semaphore::new(RESEARCHER_PARALLELISM)` in `LiveResearcherMcpPort` (`main.rs:1200-1240`). A recorded 14-exchange MCP fixture exists and is unused (`spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl`; `grep -rn researcher-mcp-corpus-v0.5` over `*.rs/*.toml/*.yaml` = 0). |
| AC3: spike — "if no, the dispatcher `Clock` trait is FLAG-Winston +15–30" | **Measured yes** (§Kernel and kloc pricing). Cap lookup `effective_cap_seconds` `hook_dispatch.rs:350-357` (was `:350-355`); envelope test `hook_crosses_eighty_percent_before_timeout` `crates/maos-kernel-core/tests/hook_dispatch_budget_envelope.rs:191-238` (was `:189-225`); coverage `:1378` (was `:1376`). The `5400` lives only in the tracker comment (`sprint-status.yaml:279`, refreshed at creation). Corrected in place. |
| Holds | `spirits/researcher/src/lib.rs:914-958`, `:924`, `:941`, `:948-953`, `:954`, `:956`; `worker_spawn.rs:77-78` (parser now `:67-101`); `cassette_replay.rs:146-152`; `five_metric_live_8_11.rs:20-26`; `prd/non-functional-requirements.md:14,62-67`; `prd/user-journeys.md:67-104` (journey `:57-106`), citation floor `:104`; Researcher cap `manifest.toml:96`, Butler `:65`; R18-P2 `manifest.toml:49`; `deferred-work.md:1212` (its `main.rs:5510-5514` is stale → `:5770-5786`). `--once` sites: 36 @9f920180 → 61 @`ee395748`. |

## Kernel and kloc pricing (E16-A7) — measured at creation

**AC3 spike (rule 5, ≤ 1 d; actual ≈ 25 min).** One throwaway test file in `crates/maos-kernel-core/tests/`, run with `cargo test -p maos-kernel-core --test zz_spike_paused_time_18_3 -- --nocapture` (11.4 s wall including the incremental build; tests 0.20 s), against the unchanged dispatcher (`hook_dispatch.rs:556-679`, tokio 1.53.3, `#[tokio::test(start_paused = true)]`; `tokio` `test-util` is already a kernel dev-dep, `crates/maos-kernel-core/Cargo.toml:104`). The Spirit's hook signals "entered" and blocks on a std channel; the SCB carries `budget = Some(Budget { time_cap_seconds: 60, .. })`, so `effective_cap_seconds` returns the Researcher cap.

| Case | Driver | Outcome | Real time |
|---|---|---|---|
| (1) 80 % warning | `advance(47.9 s)` → mailbox empty; `advance(+0.2 s)` → `BudgetWarning` frame; release the hook | `BudgetWarning80 { fired: true, cap_seconds: 60 }` | 4.4 ms |
| (2) 100 % exceed | `advance(60.1 s)` while blocked | `BudgetExceeded { cap_seconds: 60 }`; frames `BudgetWarning` then `BudgetExceeded` | 3.6 ms |
| (3) auto-advance hazard | 200 ms real-time hook, no manual advance | `Fired` — tokio inhibits auto-advance while a `spawn_blocking` task runs, so no spurious 48 s / 60 s firing | 202.6 ms |

**Labelled limit (D-18-3-10):** `wall_ns` in the outcome and the envelope's elapsed come from `cap_tokens::monotonic_now_ns()` (`hook_dispatch.rs:557,598,608`, std clock), so under paused time they report real nanoseconds (0.86 ms and 1.2 ms above), not 48 s. The variant asserts the outcome kind, `cap_seconds: 60` and the frame order, never `wall_ns ≥ 48 s`.

**Kernel figure:** with the spike present `check-kernel-baseline` PASSED (26377 lines, 95 files, pin 26377) and `kloc-check --json` reported `maos-kernel-core` 20608 (tests are excluded, `xtask/src/kloc_check.rs:302-303`). Deleted by `rm`; `git status` clean; `check-kernel-baseline` PASSED 26377 / 95 again. **Kernel-Δ 0; no FLAG-Winston row.** Nothing else in the story needs the kernel: AC5 is solved by the request naming the minted provider (the adapter already honours `req.provider_id`, `inference/mod.rs:256-261`).

**kloc at `ee395748`** (`kloc-check --json`, tokei 14.0.0, `passed: true`, `over_budget: []`):

| Crate | Measured / ceiling | 18-3 forecast [INFERENCE] | Consequence |
|---|---|---|---|
| maos-bin | 28094 / 28094 (`xtask/kloc.toml:517`); Epic 18 named budget 28194 at creation (100 left) → **28444 by ruling D-18-3-11 B** (350 left, shared with 18-4's +100–250) | +90–165: provider rule + per-pass mint (+15–25), seed journaling (+15–30), `--once` verdict + survey event + `output_shape` + distillate write (+25–40), successor/door seam (+10–20), scalar port (+10–20), `eval distill` arm (+15–30) | **D-18-3-11 B**: exact re-pin at landing; above 28444 → re-ask |
| maos-eval | 4271 / 4271 (`:674`); 15-2 envelope 4831 (`:672-673`, "+450–900", 18-2 used +230) | +150–300 (string metrics, frozen-verdict scorer, corpus loader) | measured request at landing, inside the 15-2 envelope |
| maos-bench | 2032 / 2032 (`:520`); its 15-2 reservation consumed by 18-2 (`:519`) | +100–200 (five-metric replay composition beside `halt_eval.rs`) | measured request at landing (no ledger) |
| maos-journey-test | 915 / 1104 (`:681`) | +0–10 (jr2 assertion) | fits |
| maos-cli / maos-registry / maos-providers | 6104 / 6856 · 3452 / 4282 · 1392 / 2000 | 0 | — |
| maos-kernel-core | 20608 / 20608 (`:264`) | 0 (measured) | — |
| aggregate | 183247; `_aggregate_hardfail` 183248 (`:718`) | +240–675 | re-pin = measured + 1 at landing |

`spirits/researcher` (seed, binding type, refusal state, scalar builder) is ungoverned (`xtask/src/kloc_check.rs:310-311`); CR-1 keeps logic there.

## Decisions — RULED 2026-10-25 by the operator (Lunarpulse)

**Ruling record.** D-18-3-01 **A**, 02 **B**, 03 **A**, 04 **A**, 05 **A**, 06 **A**, 07 **A**, 08 **B**, 09 **A**, 10 **A**, 11 **B** (the operator deviated from the creation recommendation A: Epic 18's named maos-bin budget is raised to 28444). Review on `anthropic/claude-opus-5-5` only; fable declined, including for AC5's security layer. CR-n are creation defaults carried into the rulings.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-3-01** | What does the bound non-`--live` Researcher pass survey? (It has no input at HEAD and consumes zero completions — measured.) | **A — an embedded seed claim set in `spirits/researcher` (e.g. 3 `ClaimPayload`s, one hedged), journaled by the root as TL frames at boot so findings cite resolvable refs; used when the inference port is bound and no MCP port is wired; one cassette entry per claim** · B — under explicit replay/record the root wires a fixture MCP client over `researcher-mcp-corpus-v0.5.jsonl` so `on_idle` runs the production fan-out → walk → join → survey · C — a new `maos run --frames <path>` flag | **A.** Butler-seed parity (`construct_butler_core` seeds its scenario, `main.rs:689-708`); hermetic; no new flag (C changes the parser `worker_spawn.rs:67-101`, rule 1). B couples the inference selector to MCP replay, which ADR-064 does not cover (`:62-84`); AC4's fan-out proof is a library witness instead. | RULED 2026-10-25 — A: seed claims written to the TL; hermetic and resolvable, no new flag |
| **D-18-3-02** | Which provider does the Researcher's token and request name? (R18-P2: it declares `anthropic`+`openai`.) | A — its first declared provider only (the `ButlerSeam::attach` rule, `spirits/butler/src/lib.rs:259-266`) · **B — its first declared provider the router registers (`anthropic`, else `openai`); none registered under `--live`/explicit → typed boot refusal naming both** · C — the router default when declared, else refuse | **B.** Keyless explicit replay registers every interned id (`main.rs:3857-3865`), so B picks `anthropic` there, like A; B also serves an OpenAI-only host (why 17-11 declared `openai`, `manifest.toml:46-49`). Both `--live` (`:5757-5762`) and explicit (`:5774-5777`) take the router default today and are refused on any Ollama-default host (measured). Zero-config nuance as D-18-1-R2: the `anthropic` `UnconfiguredProvider` placeholder (`:3800-3804`) counts as registered, the root boots, and each pass refuses typed `Unconfigured`. | RULED 2026-10-25 — B: first declared provider that is registered, else typed boot refusal |
| **D-18-3-03** | Token lifetime for a daemon Researcher | **A — mint per pass via the root's `ButlerSeam.mint` (`main.rs:3960-3982`), 60 s TTL kept** · B — one admission mint, labelled · C — a longer TTL | **A.** Today one token is minted at admission (`:5985`); with the fallback removed, every daemon pass after 60 s would be refused `CapabilityDenied`. A mirrors D-18-1-05 A: every mint audited, policy-checked and charged to the 17-9 window. | RULED 2026-10-25 — A: mint per pass, as D-18-1-05 A |
| **D-18-3-04** | Door-loaded and hot-swapped Researchers | **A — one Researcher binding attached on the boot arm, the door `load` and the successor (`build_spirit_obj` `main.rs:930`), each reading its declared providers from its own manifest (as the Butler arm `:938-977`)** · B — refuse door-load/hot-swap of a Researcher while the root is bound, typed · C — leave unbound, labelled | **A.** The 18-1 review measured the same gap for Butler (EC-3/TIA-2) and ruled one construction path (D-18-1-R1). C contradicts D-18-1-04 A ("never the arithmetic fallback"). The successor's MCP/collective ports stay boot-arm-only (labelled non-goal). | RULED 2026-10-25 — A: one binding on boot, door load and hot-swap successor, reusing 18-1's seam pattern |
| **D-18-3-05** | What a model-path failure does | **A — `--once`: typed non-zero (`Researcher::take_pass_error` + a root verdict like `maos_bin::butler_root::pass_verdict`, `crates/maos-bin/src/butler_root.rs:11`); daemon: that pass is refused typed (stderr, no `last_output`, no distillate, no scalar) and the daemon keeps serving; a failed MCP survey or walk on a bound `--live` pass (today `eprintln!` + return, `spirits/researcher/src/lib.rs:444-483`) is the same typed refusal; unset mode stays deterministic (ADR-064)** · B — daemon exits on the first refusal · C — keep a fallback for transient errors | **A.** Mirrors D-18-1-04 A and 18-8 AC5 (both-fetches-failed refusal). C is the silent path AC1 removes. `summarize` stays only as the deterministic compressor and as the bound on model text (`lib.rs:942`, §9.5). | RULED 2026-10-25 — A: `--once` non-zero; daemon refuses the pass and keeps serving; MCP failures count |
| **D-18-3-06** | AC2 vs AC4: are faithfulness and hedge CI floors? (8.11 FORK A says reported-only; R18-22 / AC4 / NFR-Aud-7 say every class gates all five.) | **A — all five are CI floors over the NEW frozen Researcher corpus: recall (source `claim_id`s present as findings), hedge (each source hedge present in the finding's *statement text*), traceability (every `source_log_ref` resolves), secret-leakage (real `CorpusBackedRedactionPolicy`) as deterministic string metrics; faithfulness from **frozen judge verdicts** committed with the corpus (judge-prompt SHA pinned, `provenance: seed` until recorded); the live judge runs only nightly** · B — four deterministic floors in CI; faithfulness reported nightly only (contradicts AC4 and needs a PRD-level operator ruling) · C — status quo label means | **A.** The epic itself notes a judge cassette "is coherent only over a frozen distillate" (epic AC2, `:162`): over a frozen corpus, replaying recorded verdicts is coherent and keyless. FORK A's real constraint — no live judge in CI — is kept. Rule 11: every number carries its provenance. | RULED 2026-10-25 — A: all five floors over a new frozen corpus; faithfulness from frozen judge verdicts |
| **D-18-3-07** | How the frozen corpus and verdicts are produced and regenerated | **A — `maos eval distill --class researcher` beside 18-2's `eval halt` (composition in `maos-bench`, scorer in `maos-eval`, thin arm in maos-bin): replay = the CI floor; `MAOS_INFERENCE_MODE=record` = the nightly regeneration of completions and judge verdicts into `.live` siblings, promoted by the operator procedure of D-18-2-R3 (`journey-nightly.yml:102-119`)** · B — an `#[ignore]` live test in `spirits/researcher` run nightly · C — an xtask command | **A.** One record path (15-6's `CassetteRecordProvider`, `cassette_replay.rs:362-379`), one verb family, the 18-2 crate split (D-18-2-01 B: `maos-eval` cannot depend on the kernel). B bypasses the shared router; C puts product logic in xtask (zero headroom, `kloc.toml:339`). | RULED 2026-10-25 — A: `maos eval distill`, replay in CI / record nightly into the `.live.json` sibling, mirroring 18-2's split and D-18-2-R3 |
| **D-18-3-08** | Which classes are "distillation-shipping" for AC4 | A — the Researcher (its distillate written on the production path by this story) and the Digest class, all five metrics each · **B — the Researcher wired and gated on all five; the Digest class gated on traceability and secret-leakage, with recall/faithfulness/hedge recorded N/A-by-construction (its distillate is a roll-up of TL fact refs, `spirits/digest/src/lib.rs:437-456`, with no source claims or hedges) in the NFR-Aud-7 row** · C — the Researcher only, its production distillate deferred to 18-6 | **B.** Today only the Digest class writes a production distillate (`main.rs:501`); the Researcher's `distill_through` has no caller. 18-6 depends on the Researcher's replay/live distillation (epic Dependencies `:232`), so the write belongs here (rule 10 (a)). A would invent claims and hedges for a class that has none. | RULED 2026-10-25 — B: Researcher all five; Digest traceability + leakage, the rest N/A |
| **D-18-3-09** | Does production evaluate the Researcher's declared halts (J-Researcher's contradiction halt)? | **A — yes: the boot arm and the successor attach an `EpistemicScalarPort` for the Researcher (reusing `ButlerOrchestratorAdapter`, `main.rs:557-643`, which already takes policy + spirit id); `requires_epistemic_halt_port` (`:512-527`) gains `methodology_conflict`/`load_bearing_confidence`, which turns the Researcher arm's existing boot-loud refusal (`:5593-5602`, "declares a self-halting posture but has no EpistemicScalarPort wiring") into the wiring point; the surveyed pass writes `primary_scalar`; 18-2's Researcher reason string (`crates/maos-bench/src/halt_eval.rs:108-111`) is updated** · B — a hermetic library witness only; the production gap labelled and routed · C — remove the halt rules from the manifest | **A.** The manifest declares two halts production never evaluates (rule 11 (a)); J-Researcher's halt (`prd/user-journeys.md:73-80`) is a control. Kernel-Δ 0 (the adapter is maos-bin). B leaves a declared control unwired; C deletes PRD behaviour. | RULED 2026-10-25 — A: wire the Researcher's halts through the existing scalar adapter |
| **D-18-3-10** | Paused-time `wall_ns` reports real time (spike) | **A — accept; the variant asserts outcome kind, `cap_seconds` and frame order** · B — switch the dispatcher to `tokio::time::Instant` (kernel edit, FLAG-Winston, ≈ +3 lines [INFERENCE]) | **A.** The envelope's correctness (warn at 80 %, exceed at 100 %) is what NFR-Perf-6 asks; the elapsed field is telemetry. B buys a kernel grant for a test convenience. | RULED 2026-10-25 — A: accept real-time `wall_ns`; kernel-Δ stays 0 |
| **D-18-3-11** | kloc governance | A — pre-ratify maos-bin ≤ 28194 for 18-3 (the rest of Epic 18's named budget); 18-4 returns its own raise request · **B — raise the named budget now (+385 → +635, 27809 → 28444)** · C — no pre-ratification; T0 prototype-measures maos-bin and returns a measured request before dev proceeds | Creation recommended A with C's stop rule. **Ruled B:** logic still goes to `spirits/researcher` (CR-1); maos-bin is re-pinned at its exact measured figure at landing (after `cargo fmt --all`, `kloc-check --json`); above 28444 → re-ask before landing. maos-eval (inside the 15-2 envelope 4831) and maos-bench (no ledger) are exact measured requests at landing; `_aggregate_hardfail` = measured + 1; kernel 0, any kernel line stops the story for a FLAG-Winston row. | RULED 2026-10-25 — B (operator; deviates from the creation recommendation A): Epic 18 named maos-bin budget 28444; exact re-pin at landing; above 28444 → re-ask **Addendum 2026-10-25 (operator, Lunarpulse): maos-bench 2325 RATIFIED** (+293 over 2032, measured request at landing, above the 2232 stop line). |
| CR-1 | Binding shape, no second seam | `researcher::ResearcherInferenceBinding { provider_id, mint }` mirrors `butler::ButlerInferenceBinding` (`spirits/butler/src/lib.rs:200-207`); the request names `provider_id` (as Butler, `:945`). The root reuses `ButlerSeam.inference` (port builder + registered ids) and `ButlerSeam.mint` (`main.rs:3955-4001`; the seam already says it mints the Researcher's token, `spirits/butler/src/lib.rs:237-238`). No second mint, port builder or seam type. `spirits/researcher/Cargo.toml` gains no dependency on `butler`. | — |
| CR-2 | Cassette | `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json` is re-authored: one entry per seed claim, the real `prompt_sha256`/`prompt_len` of `synthesize`'s prompt (`spirits/researcher/src/lib.rs:929-932`), `provenance: seed`, no wildcard (a test asserts it). Strict replay. It stays under the journey cassette tree (cassette-age gate, nightly upload `journey-nightly.yml:121-127`). | — |
| CR-3 | CI | `epic-18-exit` (`discipline.yml:2226-2370`): build `maosctl` too (`cargo build --locked -p maos-cli --bin maosctl`); the line-2 step runs `maosctl spirit benchmark butler` under its scratch `HOME` (TIA-6, `deferred-work.md:1247`; probed at HEAD: rc 0, "signature verified against the development seed's key"); add exit line 3 verbatim (keyless, `MAOS_OLLAMA_URL` unset) asserting `researcher inference seam wired`, a `researcher_survey` event with the four sections and `completions_consumed` = entries; and its twin over an empty cassette that must exit non-zero naming `exhausted`. `xtask/tests/ci_shape_18_0.rs` gains the line-3 shape test (precedents `:255`, `:320`). The five-metric gate runs in `nfr-aud-7-distillate-five-metrics-floor` (`:1487-1500`). | — |
| CR-4 | Regressions | `jr2` and `researcher_8_14c_mcp_fanout` keep asserting the fan-out, then assert the zero-config typed refusal (`Unconfigured`, non-zero) instead of exit 0. `maos_run_researcher_live_once_wires_mcp_port` (no status assert), `jr1`, `jr_zero_side_effect_deterministic_floor` and every unset-mode `--once` site stay unchanged. | — |
| CR-5 | Coverage and registries | NFR-Aud-7 row (`tests/coverage-matrix.yaml:981-985`) gains the five-metric gate and the corpus; NFR-Perf-6 (`:1378-1382`) gains the envelope gate; both names go into `xtask/gate-registry.toml` (unknown names are refused, `xtask/src/coverage_matrix.rs:142`); the corpus is registered in `tests/corpora/MANIFEST.toml` (precedent `:141-147`). | — |
| CR-6 | Judge contract (NFR-Test-1) | `[judge.researcher-faithfulness]` in `tests/judge-config.toml` with all six constraints (hedge preservation is a deterministic string metric under D-18-3-06 A, so it has no judge row); the nightly judge calls the Inference Port (no identifier from `xtask/judge-direct-call-identifiers.toml`); its model pin passes `check-model-currency`; `check-judge-config` green. | — |

## Acceptance Criteria

*(Epic §18-3 as corrected at creation and ruled 2026-10-25 (D-18-3-01 A, 02 B, 03 A, 04 A, 05 A, 06 A, 07 A, 08 B, 09 A, 10 A, 11 B). One plant per control AC, `docs/testing/proof-contract.md:26`.)*

### AC1 — command: exit line 3 replays through the Inference Port, and every model-path failure is typed (D-18-3-01, D-18-3-05; CR-2, CR-3, CR-4)

1. `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json maos run spirits/researcher/manifest.toml --once` on a keyless runner (scratch `HOME`, `MAOS_OLLAMA_URL` unset) exits 0. The bound pass surveys the seed claims (D-18-3-01 A) and consumes exactly one completion per claim through the kernel `InferencePortAdapter` (explicit replay, `main.rs:3839-3866`). Under `--once` the root prints `{"event":"researcher_survey", …}` carrying `findings`, `open_questions`, `confidence_map`, `bibliography` and `completions_consumed`, checked against the manifest `output_shape` (`spirits/researcher/manifest.toml:87-90`) with `OutputShapePredicate` as the Digest arm does (`main.rs:6110-6114`).
2. The fallback is removed (`spirits/researcher/src/lib.rs:914-958`): `synthesize` returns a typed error. A bound pass that hits an unbound binding, any `InferenceError`, or (bound `--live`) a failed MCP survey/walk records a typed refusal readable via `Researcher::take_pass_error`. Under `--once` the root exits non-zero with it; in a daemon the pass is refused typed and the daemon keeps serving (D-18-3-05 A). Unset mode keeps the deterministic survey byte-for-byte.
3. Reds:
   - an empty cassette (`exhausted`);
   - a drifted prompt (strict, real hashes);
   - consumed completions ≠ surveyed claims;
   - the legacy (mode-unset) cassette arm (`main.rs:5787-5817`) with an exhausted cassette — the same typed exit.
4. CI (CR-3): `epic-18-exit` runs line 3 verbatim and its empty-cassette twin, and the TIA-6 `maosctl spirit benchmark butler` step; `ci_shape_18_0.rs` pins the shape.
5. Regressions (CR-4): `jr2` and `researcher_8_14c_mcp_fanout` assert the typed refusal after the fan-out.
6. Process-level witness `crates/maos-bin/tests/researcher_inference_18_3.rs` (`env!("CARGO_BIN_EXE_maos")`, fresh `HOME`, no key, no nested cargo): good cassette exit 0 with the survey event; empty, drifted and short cassettes exit 1 naming the cause; mode unset stays deterministic.
7. **Proven red (1 plant):** restore `summarize(statement)` on `Err` in `synthesize` → the empty-cassette case exits 0 → red.

### AC2 — the five metrics are computed from what the Researcher emits, over a frozen corpus (D-18-3-06, D-18-3-07; CR-5, CR-6)

1. NEW frozen corpus (location set at dev; registered in `tests/corpora/MANIFEST.toml`, SHA-pinned): N scenarios of source claims (as TL-frame payloads), their recorded completions (real prompt hashes), the frozen survey output, and frozen judge verdicts carrying the judge-prompt SHA and `provenance`. Includes negative controls: a hedge-dropping completion set, a fabricated cite, a planted secret, a dropped claim.
2. `maos eval distill --class researcher` (D-18-3-07 A) replays the completions through `Researcher::survey` → the distillate (kernel `DistillateWriter`, I11) and prints recall, faithfulness (frozen verdicts), hedge preservation, traceability and secret leakage against NFR-Aud-7's floors, with provenance; exit 1 on any missed floor or a recomputed survey that differs from the frozen one (re-pin needed).
3. `crates/maos-eval/tests/distillate_five_metrics_floor.rs:26-136` stops asserting label means; its N=100 job (`discipline.yml:1487-1500`) runs the new gate. The quarterly slice (`:142-272`) is unchanged (non-goal).
4. `MAOS_INFERENCE_MODE=record maos eval distill --class researcher` (nightly, `tier-2-rerecord`, after `:114-119`) writes the `.live.json` sibling (D-18-2-R3 pattern) — completions and live judge verdicts through the Inference Port — never the committed corpus; promotion is the D-18-2-R3 operator procedure. Hermetic witness: record against a stub router writes real hashes and `provenance: live-record`, then replays green.
5. CR-6 judge row committed; `check-judge-config` green.
6. **Proven red (1 plant):** make the hedge metric read the structural `Finding.hedges` list (always preserved) instead of the statement text → the hedge-dropping negative control passes → the gate test reds.

### AC3 — the Researcher's 60 s envelope under paused time (spike = yes; D-18-3-10)

1. NEW `crates/maos-kernel-core/tests/hook_dispatch_paused_time_18_3.rs` (or a paused-time block in `hook_dispatch_budget_envelope.rs` beside `:191-238`): the cap is read from `spirits/researcher/manifest.toml:96` (60, via `maos_manifest::Budget`), not re-typed; `advance` just under 80 % → no frame; just over → `BudgetWarning` frame, release → `BudgetWarning80 { fired: true, cap_seconds: 60 }`; `advance` past 100 % → `BudgetExceeded` with both frames; a real-time hook with no advance → `Fired`. Asserts kinds, `cap_seconds` and order, not `wall_ns` (D-18-3-10 A). NFR-Perf-6 is `prd/non-functional-requirements.md:14`; if the cap changes, cite it.
2. `tests/coverage-matrix.yaml:1378-1382` gains the gate (CR-5). The `5400` is gone (tracker comment refreshed at creation).
3. **Proven red (1 plant, temporary kernel edit, restored; kernel-Δ stays 0):** `cap_seconds.saturating_mul(800)` → `saturating_mul(1000)` (`hook_dispatch.rs:582`) → the 80 % case reds.

### AC4 — NFR-Aud-7 per shipping class, and the J-Researcher survey (routing audit `:37`; D-18-3-06, D-18-3-08, D-18-3-09)

1. **Researcher (shipping, D-18-3-08 B):** the surveyed pass writes its distillate through `distill_through` (`spirits/researcher/src/lib.rs:1015-1025`) into the root's `DistillateWriter` (`main.rs:2687`) — traceability kernel-enforced by I11 — and AC2's five floors gate it.
2. **Digest class:** traceability and secret-leakage are gated on its production distillate (`main.rs:501`); recall/faithfulness/hedge are recorded N/A-by-construction in the NFR-Aud-7 row with the reason.
3. **Production halt (D-18-3-09 A):** the Researcher's boot arm and successor attach the scalar port; `requires_epistemic_halt_port` covers its two tags; a surveyed pass writes `primary_scalar` (`spirits/researcher/src/lib.rs:265`); 18-2's Researcher reason string is updated.
4. **J-Researcher hermetic survey** (library/bench witness over the real kernel world; `spirits/researcher/tests/` or `maos-bench`): MCP fan-out over the recorded `researcher-mcp-corpus-v0.5.jsonl` exchanges through the kernel `McpClientAdapter` with observed peak concurrency = `RESEARCHER_PARALLELISM` (8); a contradictory strong-methodology pair → `methodology_conflict` > 0.7 → kernel halt → resolved once (`accepted-halt`) via `HaltRegistry::resolve` (`crates/maos-kernel-core/src/halt/mod.rs:243`), journaled → the next pass resumes and surveys; the output passes the four-section `OutputShapePredicate`; ≥ 95 % of `source_log_ref`s resolve via `LogRecallAdapter` (`prd/user-journeys.md:104`); the survey pass finishes inside the 60 s cap (wall-clock recorded).
5. **Proven red (1 plant):** drop the Researcher's scalar write in the root wiring → the production-halt witness (a `--once` replay over a contradiction seed) shows no `epistemic.halt` → red.

### AC5 — keyless explicit replay names a declared provider; every construction path is bound (D-18-3-02, D-18-3-03, D-18-3-04; security-critical)

1. The Researcher's token and request name its first declared provider the router registers (D-18-3-02 B). Keyless explicit replay names `anthropic` and replays through the interned driver (`main.rs:3857-3865`); the TL `infer:` frame names `anthropic`; no token is minted for an undeclared provider.
2. Under `--live` or an explicit mode, when neither declared provider is registered (e.g. `MAOS_OLLAMA_URL` set to a real endpoint, no keys), boot refuses typed: "researcher inference: no inference provider is configured for its declared providers `anthropic`, `openai`". Zero-config boots and refuses each pass typed `Unconfigured` (D-18-1-R2 nuance).
3. Per-pass mint (D-18-3-03 A): a daemon Researcher's pass more than 60 s after admission still infers.
4. Door-loaded and hot-swapped Researchers are bound like the boot arm (D-18-3-04 A): witness in `crates/maos-bin/tests/researcher_inference_18_3.rs` — a door-loaded Researcher's pass adds an `infer:` frame (precedent `butler_daemon_18_1`).
5. **Proven red (1 plant):** send `provider_id: None` again → the keyless line-3 case is refused (`CapabilityDenied` against the router default) → red. `deferred-work.md:1212` closes on the landing run.

## Tasks / Subtasks

- [x] **T0 — Preflight at the dev HEAD.**
  - Every D-18-3 row RULED 2026-10-25; story and tracker `ready-for-dev` → `in-progress`.
  - `git diff --stat ee395748 -- crates/maos-bin/src/{main.rs,butler_root.rs,cassette_replay.rs,inference_mode.rs} spirits/researcher spirits/butler/src/lib.rs crates/maos-eval crates/maos-bench crates/maos-kernel-core/src/scheduler crates/maos-journey-test .github/workflows/{discipline,journey-nightly}.yml tests/coverage-matrix.yaml tests/judge-config.toml xtask/gate-registry.toml tests/corpora/MANIFEST.toml`; re-anchor drift in the Debug Log.
  - Re-measure `kloc-check --json` and `check-kernel-baseline` (expect 20608 / 26377). Per D-18-3-11 B, maos-bin may grow to Epic 18's named 28444 (shared with 18-4); a figure above 28444 at any point → stop and re-ask.
- [x] **T1 — Researcher crate** (AC1.2, AC5, AC4.3; CR-1) — `ResearcherInferenceBinding`; `synthesize` → `Result`; `pass_error` + `take_pass_error`; MCP survey/walk refusal under a bound pass; seed claim set (D-18-3-01 A); scalar-port builder; distillate hook.
- [x] **T2 — Root wiring** (AC1, AC4, AC5) — provider rule + boot refusal (`main.rs:5757-5786`); per-pass mint via `ButlerSeam.mint`; seed journaling; `--once` verdict + `researcher_survey` + `output_shape`; distillate write; scalar port + `requires_epistemic_halt_port`; `build_spirit_obj` Researcher arm (`:930`).
- [x] **T3 — Cassette + regressions** (AC1; CR-2, CR-4) — re-author `survey-distill.json`; no-wildcard test; `jr2`, `researcher_8_14c_mcp_fanout`; `researcher_inference_18_3.rs`.
- [x] **T4 — Frozen corpus + scorer + composition + verb** (AC2; D-18-3-06/07) — corpus + negative controls + pin + MANIFEST row; `maos-eval` scorer; `maos-bench` composition; `eval distill` arm; replace the N=100 floor test body.
- [x] **T5 — Judge + nightly** (AC2.4–2.5; CR-6) — judge row; record path; `tier-2-rerecord` step; hermetic record witness.
- [x] **T6 — Paused-time envelope** (AC3).
- [x] **T7 — J-Researcher witness** (AC4.4) and the Digest-class traceability/leakage gate (AC4.2).
- [x] **T8 — CI + coverage** (CR-3, CR-5) — `epic-18-exit` line 3 + twin + `maosctl` step; `ci_shape_18_0.rs`; coverage rows; gate registry; 18-2's reason string.
- [x] **T9 — Plants and records** — AC1–AC5, once each, in `18-3-evidence/plants.md` (command, output, wall-clock); a CI plant for the line-3 twin; epic §18-3 if anything moved.
- [x] **T10 — Governance** (D-18-3-11 B) — `cargo fmt --all`; `kloc-check --json`; maos-bin re-pinned at its exact measured figure (≤ 28444, else re-ask); maos-eval / maos-bench exact measured requests returned **before** landing; `check-kernel-baseline` (unchanged), `check-exit-commands`, `coverage-matrix`, `check-judge-config`, `check-model-currency`, `check-env-contract`, `check-test-cost`, `check-service-boundary`, `cassette-age-gate`, `check-dev-record-completeness`; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule --test ci_shape_18_0` before the closing commit.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Researcher | `spirits/researcher/src/lib.rs:367-401` (struct), `:422-490` (`on_idle`), `:527-544` (builders), `:914-958` (`synthesize`), `:1015-1025` (`distill_through`) | silent fallback → typed refusal; seed; binding with per-pass mint; scalar port |
| Root | `crates/maos-bin/src/main.rs:5592-5855` (Researcher arm; `--live` `:5620-5769`, explicit `:5770-5786`, legacy `:5787-5817`, deterministic `:5818-5822`), `:5972-5987` (binding), `:6049-6187` (`--once` tail), `:898-979` (`build_spirit_obj`), `:512-527` (halt-port set), `:557-643` (scalar adapter) | router-default mint → declared-provider binding on every construction path |
| Cassette | `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json` | 1 wildcard entry, consumed by nothing → one hashed entry per seed claim |
| Five-metric | new corpus; `crates/maos-eval/src/` scorer; `crates/maos-bench/src/` composition beside `halt_eval.rs`; `eval distill` arm; `crates/maos-eval/tests/distillate_five_metrics_floor.rs:26-136` | label means → measured metrics |
| Judge | `tests/judge-config.toml`; `.github/workflows/journey-nightly.yml:67-139` | empty `[judge]` → one pinned row; nightly record step |
| Kernel tests | new paused-time test in `crates/maos-kernel-core/tests/` | wall-clock 1 s cap only → Researcher 60 s cap, deterministic |
| CI / coverage | `.github/workflows/discipline.yml:2226-2370`, `:1487-1500`; `xtask/tests/ci_shape_18_0.rs`; `tests/coverage-matrix.yaml:981-985,1378-1382`; `xtask/gate-registry.toml`; `tests/corpora/MANIFEST.toml` | — |
| Regressions | `crates/maos-journey-test/tests/journey_researcher.rs:49-100`; `crates/maos-bin/tests/researcher_8_14c.rs:244-300` | exit 0 via fallback → typed refusal asserted |

### Current state that must be preserved

- **Unset mode = deterministic** (ADR-064 `:67-71`): `maos run researcher --once` with no mode, no cassette and no `--live` prints `researcher deterministic survey` and touches no router (`main.rs:5818-5822`; `jr_zero_side_effect_deterministic_floor`, `journey_researcher.rs:103-154`).
- **ADR-064 lattice** (`inference_mode.rs:28-67`): `replay` + `--live` stays a typed conflict; record/replay need a cassette.
- **18-1's Butler seam** (`main.rs:3955-4001`, `spirits/butler/src/lib.rs:228-288`): unchanged in behaviour; the Researcher only reuses its mint and port builder.
- **18-2's `eval halt`** and its Researcher class corpus (`crates/maos-bench/src/halt_eval.rs:84-116`): only the reason string changes (D-18-3-09).
- **8.11 hard gates** (`spirits/researcher/tests/five_metric_live_8_11.rs`, `five_metric_self_verify.rs`): stay green; they become a subset of AC2.
- **The kernel**: no file under `crates/maos-kernel-core/src` changes (AC3's plant is temporary).

### Guardrails

- Never fall back silently: a bound pass either infers or refuses typed. Never re-type a floor (NFR-Aud-7 floors from one source; the cap from the manifest).
- One record/replay path (`cassette_replay.rs`), one mint (`ButlerSeam.mint`), one port builder; no second seam type.
- No wildcard prompt hash in any 18-3 cassette (strict mode does not catch it, `cassette_replay.rs:151-152`).
- No token for an undeclared provider; the request names the minted provider.
- Every published number carries its provenance (`seed` vs `live-record`); frozen judge verdicts say so.
- One plant per control AC, in place, restored by `git checkout -- <file>`. One target dir. Local runs `env -u CI -u GITHUB_ACTIONS`. No `cargo` inside a `#[test]` (`proof-contract.md:22-27,59-81`).
- If `intent-lineage-coverage-report.md` is modified by a kernel run, revert it.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| No silent fallback (AC1) | restore `summarize(statement)` on `Err` | empty-cassette case exits 0 |
| Hedge metric reads emitted text (AC2) | score structural `Finding.hedges` | hedge-dropping negative control passes |
| 80 % envelope (AC3) | warn at 100 % (`hook_dispatch.rs:582`, temporary) | 80 % paused-time case |
| Production halt (AC4) | drop the Researcher scalar write | no `epistemic.halt` on the contradiction seed |
| Declared-provider request (AC5) | `provider_id: None` | keyless line 3 refused |

In CI the `epic-18-exit` empty-cassette twin is the standing red for exit line 3.

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p researcher -p maos-eval -p maos-bench -p maos-digest
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test researcher_inference_18_3 --test researcher_8_14c --test inference_mode_15_6 --test butler_daemon_18_1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test --test journey_researcher
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test hook_dispatch_budget_envelope --test hook_dispatch_paused_time_18_3
cargo build --locked -p maos-bin --bin maos && MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json target/debug/maos run spirits/researcher/manifest.toml --once
cargo run -p xtask -- check-judge-config && cargo run -p xtask -- coverage-matrix && cargo run -p xtask -- check-kernel-baseline
cargo run -p xtask -- kloc-check --json && cargo run -p xtask -- check-test-cost
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-57`)

- **Reviewer model:** `anthropic/claude-opus-5-5` ONLY (operator ruling 2026-10-25; fable declined, including for AC5). The AC5 security-focused reading is an opus-5-5 layer.
- **Security layer (AC5):** provider selection and boot refusal; no undeclared mint; request/token agreement; per-pass mint audit and quota; door/successor binding; no keyless reroute.
- **Focused suites:** `-p researcher`, `-p maos-eval`, `-p maos-bench`, `-p maos-digest`, `-p maos-bin` (`researcher_inference_18_3`, `researcher_8_14c`, `inference_mode_15_6`, the eval-distill witness), `-p maos-journey-test` (`journey_researcher`), `-p maos-kernel-core` (the two envelope tests). No kernel source change, so no `cargo test -p maos-kernel-core --lib` unless landing measures one.
- **Gates:** `check-exit-commands`, `coverage-matrix`, `check-judge-config`, `check-model-currency`, `kloc-check`, `check-kernel-baseline` (unchanged), `check-test-cost`, `cassette-age-gate`, `check-service-boundary`.

### Previous-story intelligence

- **18-1 (review, landing):** the seam this story reuses — `ButlerSeam` (`spirits/butler/src/lib.rs:228-288`), root mint/port builder (`main.rs:3955-4001`), replay under interned ids (`:3857-3865`), `pass_verdict` (`crates/maos-bin/src/butler_root.rs:11`). Review rulings applied by analogy: D-18-1-R1 (every construction path bound → D-18-3-04), D-18-1-R2 (unregistered declared provider refuses at boot → D-18-3-02), D-18-1-04/05 A (typed refusal, per-pass mint → D-18-3-05/03). 18-1 left the Researcher arm's behaviour unchanged on purpose (`18-1-…md:26,168`).
- **18-2 (review, landing):** the `eval` verb, `maos-bench` composition and `maos-eval` scorer split (D-18-2-01 B), record → `.live.json` + operator promotion (D-18-2-R3), corpora in `tests/corpora/MANIFEST.toml` (D-18-2-R5), TIA-6 deferred here. Its Researcher class reason string changes under D-18-3-09.
- **18-8 (review, landing):** AC5's both-fetches-failed refusal is the Butler precedent for D-18-3-05's MCP clause.
- **17-11 (done):** the declared-provider entitlement and the legacy-arm first-declared mint (`main.rs:5802-5809`); its review deferred `deferred-work.md:1212` here.
- **8.11 / 8.14c:** FORK A, the hard gates, the MCP fan-out and parallelism 8.
- **15-6:** ADR-064, the shared-router replay/record wrappers.

### Git intelligence

`ee395748` 18-2 dev + review (+ 18-8 landing fix); `140b906b` 18-8 dev + review (+ 18-1 landing fix); `08ae68ad` 18-1 CI proven-red; `a85673f0` 18-1 dev + review (Butler seam, `epic-18-exit`); `7b0b59e1` DECIDE-2/3 + 18-1 created. None changed the Researcher's arm behaviour, `synthesize`, the N=100 floor test or the dispatcher.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-3 `:153-165` (creation corrections `:157`, rulings `:159`), exit block `:11-26` (`:16`, `:24`), operator lane `:28-29`, Kernel-Δ `:32`, kloc asks `:36` (18-3 ruling `:38`), rules 9–11 `:40-42`, §R `:46-83` (R18-13…15 `:64-66`, R18-22 `:73`, R18-23 `:74`, R18-P2 `:81`), stories table `:94`, Dependencies `:230-232`.
- `routing-audit-2026-10-22.md:37,109,111`; `prd/non-functional-requirements.md:14,62-67,78`; `prd/user-journeys.md:57-106`; `deferred-work.md:1212,1247`; `docs/adr/ADR-064-one-inference-mode-selector.md:25-35,67-84,129-139`; `docs/testing/proof-contract.md:22-27,31-57,59-81`; `tests/judge-config.toml`; `xtask/src/check_judge_config.rs`.

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract) at `ee395748`, clean tree. Inputs: direct reads of every cited range; the HEAD `maos` binary run four ways for exit line 3 (keyless CI-shaped: exit 1, token refused; `MAOS_OLLAMA_URL=skip` good / empty cassette: exit 0, zero completions; legacy arm empty cassette: exit 0); `maos eval halt --class butler` then `maosctl spirit benchmark butler` under one scratch `HOME` (rc 0 / rc 0, dev-seed signature verified — the TIA-6 chain works locally); `kloc-check --json` and `check-kernel-baseline` at HEAD; the AC3 spike (3/3 green, measured, deleted; `check-kernel-baseline` PASSED before, during and after). `git status` after the revert showed only this story's doc edits. `intent-lineage-coverage-report.md` unmodified.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25; dev passes Dev183 + Dev183b, 2026-10-25)

### Debug Log References

- One cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`, `env -u CI -u GITHUB_ACTIONS`.
- Plants: `18-3-evidence/plants.md` (AC1–AC5 red → copy-back + `cmp` + `touch` → green); CI plant `18-3-evidence/ci/plant.diff` (= AC1 diff), sha256 `c59ac2191e4d889ae5483e8c1f947ecb27095d7401aed07d59ab35fb536213f2`; all six diffs `git apply --check` OK.
- `kloc-check --json` after `cargo fmt --all` (tokei 14.0.0): maos-bin 28094 → 28328 (≤ 28444), maos-eval 4271 → 4499, maos-bench 2032 → 2325 (operator-ratified D-18-3-11 addendum), `_aggregate_hardfail` 183248 → 184003 (measured 184002 + 1); all pinned with HISTORY notes. `kloc-check: PASSED (aggregate=184002 LOC)`.

### Completion Notes List

- AC1–AC5 implemented with witnesses: `researcher_inference_18_3` (7/7), `inference_seam_18_3`, `inference_seam_8_11`, `eval_distill_18_3`, `distillate_five_metrics_floor`, `hook_dispatch_paused_time_18_3` (3/3), `j_researcher_survey_18_3`, `five_metric_digest_18_3`; `ci_shape_18_0` (38/38 with the xtask ratchet tests) covers the `epic-18-exit` line-3 step + twin and the NFR-Aud-7 job.
- `maos_run_boot_loud_8_11::deterministic_posture_spirit_without_port_boots_clean` retargeted from the Researcher to the Observer: D-18-3-09 A makes the Researcher require an `EpistemicScalarPort`, so it no longer exercises the over-fire guard.
- Gates PASS: check-kernel-baseline (26377 / 95, Kernel-Δ 0), check-env-contract, check-exit-commands, check-service-boundary, check-test-cost, check-serde-error-handling, coverage-matrix, check-model-currency, cassette-age-gate (WARN only, 2 unrelated stale cassettes), check-corpus (13), check-judge-config (1), check-dev-record-completeness; `cargo fmt --all -- --check` clean. kloc-check FAILS (maos-bench STOP, above).
- maos-bench 2325 exceeded the 2232 stop line; the operator (Lunarpulse) RATIFIED 2325 on 2026-10-25 (D-18-3-11 addendum). Re-pinned; `kloc-check` PASSED. Post-fix whole-batch re-run (Researcher/inference-mode targets): 178 run, 178 passed, 7 skipped; xtask ratchet + `ci_shape_18_0`: 38/38.

### File List

- `_bmad-output/implementation-artifacts/18-3-researcher-replay-live-and-judged-five-metric.md` (A)
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md` (M — header `:9`, operator lane `:28-29`, exit line 3 provenance `:24`, Kernel-Δ `:32`, kloc asks `:36` + 18-3 ruling `:38`, §R R18-13…15 `:64-66`, R18-23 `:74`, stories row `:94`, §18-3 `:155-165` incl. rulings `:159`, Dependencies `:232`)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (M — 18-3 row only: comment at creation; `backlog` → `ready-for-dev` at the rulings)
- `_bmad-output/implementation-artifacts/18-3-evidence/` (A — `plants.md`, `ac1…ac5-plant.diff`, `ci/plant.diff`, `ci/plant.diff.sha256`)
- `.github/workflows/discipline.yml`, `.github/workflows/journey-nightly.yml` (M)
- `crates/maos-bench/src/{distill_eval.rs (A), halt_eval.rs, lib.rs}` (M)
- `crates/maos-bin/src/{researcher_root.rs (A), lib.rs, main.rs}` (M)
- `crates/maos-bin/tests/{researcher_inference_18_3.rs, eval_distill_18_3.rs, j_researcher_survey_18_3.rs}` (A); `cohort_daemon_smoke_13_5c.rs`, `researcher_8_14c.rs`, `maos_run_boot_loud_8_11.rs` (M)
- `crates/maos-eval/src/{distill_replay.rs (A), lib.rs}`, `crates/maos-eval/tests/distillate_five_metrics_floor.rs` (M)
- `crates/maos-journey-test/cassettes/j-researcher/{distill-corpus-v0.json (A), survey-distill.json (M)}`, `crates/maos-journey-test/tests/journey_researcher.rs` (M)
- `crates/maos-kernel-core/tests/hook_dispatch_paused_time_18_3.rs` (A; no kernel src change)
- `spirits/researcher/src/lib.rs` (M); `spirits/researcher/tests/{inference_seam_18_3.rs, fixtures/distill-corpus-v0.jsonl}` (A); `five_metric_live_8_11.rs`, `inference_seam_8_11.rs` (M)
- `spirits/digest/tests/five_metric_digest_18_3.rs` (A)
- `tests/corpora/{researcher-distill-corpus-v0.jsonl (A), MANIFEST.toml (M)}`, `tests/coverage-matrix.yaml`, `tests/judge-config.toml`, `xtask/gate-registry.toml`, `xtask/kloc.toml`, `xtask/tests/ci_shape_18_0.rs` (M)

### Change Log

- 2026-10-25 — Story created (backlog; D-18-3-01…11 PROPOSED) at `ee395748`. Exit line 3 measured red at HEAD (keyless router-default token refused by 17-11; with `MAOS_OLLAMA_URL=skip` the pass consumes zero completions, so the `summarize` fallback is never reached); the successor/door Researcher is unbound; the `--live` arm also mints the router default; the daemon token expires after 60 s; `jr2` and `researcher_8_14c_mcp_fanout` pass only through the fallback; the N=100 five-metric corpus is stubs; only the Digest class ships a production distillate; the Researcher's declared halts are never evaluated in production; the Story 0.3 judge contract applies. AC3 spike run: tokio paused time drives the dispatcher → kernel-Δ 0, ◆ discharged. Epic §18-3, exit-line-3 provenance, Kernel-Δ, kloc asks, §R rows and Dependencies corrected in place; the stale tracker comment (`--replay/--deterministic`, `5400`) refreshed.
- 2026-10-25 — RULED (operator, Lunarpulse): D-18-3-01 A, 02 B, 03 A, 04 A, 05 A, 06 A, 07 A (`.live.json` sibling per D-18-2-R3), 08 B, 09 A, 10 A, 11 B (deviates from the creation recommendation A: Epic 18 named maos-bin budget +385 → +635, 27809 → 28444; exact re-pin at landing; above 28444 → re-ask). Review `anthropic/claude-opus-5-5` only (fable declined, including for AC5's security layer). Status → `ready-for-dev`.
- 2026-10-25 — Dev (Dev183, Dev183b; `anthropic/claude-opus-5-5`): T0–T9 done; AC1–AC5 green and proven red; gates green except kloc-check. maos-bin re-pinned 28094 → 28328, maos-eval 4271 → 4499; maos-bench measured 2325 > 2232 → STOP (re-ask).
- 2026-10-25 — Operator RATIFIED maos-bench 2325 (D-18-3-11 addendum). maos-bench re-pinned 2032 → 2325, `_aggregate_hardfail` 183248 → 184003; `kloc-check` PASSED; T10 done. Status → `review`.
