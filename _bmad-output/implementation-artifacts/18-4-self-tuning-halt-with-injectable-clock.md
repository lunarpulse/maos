---
story_id: '18.4'
story_key: 18-4-self-tuning-halt-with-injectable-clock
created: 2026-10-25
context_commit: 419f051f
baseline_commit: 419f051f
security_sensitive: true
kernel_grant: "none — kernel-Δ **0** under the recommended rows, measured at creation (rule 5 / E16-A7). Method: every AC was mapped to its code path (no AC needs `crates/maos-kernel-core/src`: the scene drives the public `HookDispatcher::fire_on_idle`, `hook_dispatch.rs:279-282`; the principal port composes the already-`pub` `MemoryManagerAdapter::for_spirit`, `memory/mod.rs:414`). The one kernel alternative (D-18-4-02 C, Principal-namespace values always durable) was priced by a throwaway prototype in `memory/private.rs:644` (one condition): `cargo check -p maos-kernel-core` clean, `cargo fmt --check` clean, `check-kernel-baseline` FAILED as expected at 26378 lines (pin 26377, changed: memory/private.rs), `kloc-check --json` maos-kernel-core 20609 (+1); reverted with `git checkout`, `check-kernel-baseline` PASSED 26377 / 95 files again. If landing measures any kernel line, the story stops for a FLAG-Winston row."
kloc_grant: "RULED 2026-10-25 (operator, Lunarpulse): D-18-4-08 A — **Epic 18's named maos-bin budget raised 28444 → 28510**; 18-4 lands at its exact measured figure (re-pin after `cargo fmt --all` via `kloc-check --json`); above 28510 → re-ask before landing; maos-spirit-sdk lands exact inside its ceiling; `_aggregate_hardfail` = measured + 1; kernel 0. Measured at creation, `kloc-check --json` (tokei 14.0.0) at 419f051f: maos-bin 28328 / 28328 (`xtask/kloc.toml:517`) against Epic 18's named budget 28444 (D-18-3-11 B) → 116 left; forecast +80–140 [INFERENCE] → ruled raise to 28510. maos-spirit-sdk 846 / 3000 (`:271`), maos-journey-test 915 / 1104 (`:681`), maos-domain 9805 / 9805 (`:425`, untouched), maos-kernel-core 20608 / 20608 (`:264`, untouched), aggregate 184002 / `_aggregate_hardfail` 184003 (`:718`)."
review: "bmad-code-review under `docs/testing/proof-contract.md`. Dev and reviewer model `anthropic/claude-opus-5-5` ONLY (operator ruling 2026-10-25; fable only if the operator is asked later, `proof-contract.md:57`). D-18-4-02 and D-18-4-03 get a security-focused opus-5-5 reading. **Security-critical: AC3** (a Spirit-facing write port into the FR31 principal namespace — pid fusion, authorization, RTBF, the cross-boot pid-reuse read finding) and **AC4** (an option pick executes the action the model declared — what the director sees must be what runs). The review record names a security-focused layer for both, on opus-5-5."
---

# Story 18.4: Self-tuning halt with an injectable clock

Status: ready-for-dev

<!-- Created 2026-10-25 at 419f051f by bmad-create-story under the lighter creation contract (no party-mode preflight); 18-0/18-1/18-8/18-2 done, 18-3 under review and treated as landing. The host's only cargo process was used (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`): `kloc-check --json`, `check-kernel-baseline`, one `cargo build -p maos-bin --bin maos` of HEAD, one throwaway kernel prototype (priced, reverted). RULED 2026-10-25 by the operator (Lunarpulse): D-18-4-01…08 all A as recommended; review opus-5-5 only (security reading on D-02/D-03) → `ready-for-dev`. Rulings applied docs-only (no cargo). -->

> **The capability:** *J-Butler's resolution beat runs on a keyless box. Twenty-one simulated days of the director's responses replay through the real Butler on a scenario clock; the Butler's own uncertainty about the director — not a falling acceptance rate — crosses its threshold and the existing `belief_variance` halt fires, upstream of outcome degradation. What it learned survives a restart in the director's principal namespace, the option the director picks runs the action the model declared, the morning digest rolls over on the director's local day under the same clock, and its anomaly line finally reads a real feed.*

**Closes:** exit line 4 (epic `:17`, red at HEAD — measured). J-Butler's machine-checked leg and the routing-audit widening (`routing-audit-2026-10-22.md:38,110,113`: FR31 principal-namespace persistence of the acceptance ledger, persistent learning, actual option effects, variance-led halt). FR31 for Butler's acceptance record (`prd/functional-requirements.md:77`). The three 18-1 review deferrals owned here: `deferred-work.md:1232` (local-day rollover + rollover test via the injectable `Clock`), `:1233` (option-pick mapping), `:1234` (FR17 (c) production anomaly feed; D-18-4-07). The `Clock` port 21-1 re-uses (epic-21 `:21,:42,:53,:55,:119`).

**Does NOT close, and says so:**
- **A production director pick surface.** `handle_option_pick` has no production caller and `maos shell`'s Butler pick is a canned string that never reaches Butler (`crates/maos-shell/src/lib.rs:509-526`). Picks enter through the scene fixture and the library API only; the surface is routed by D-18-4-05.
- **Restoring the record after a door `unload`/`load`.** Private-tier storage is keyed by the boot-local pid (D-18-4-02); the root `maos run` Butler keeps pid 1 across process restarts (measured), a door reload does not. Labelled; finding routed by D-18-4-02.
- **The halt's resolution beat** (`prd/user-journeys.md:39`: the director confirms and the kernel updates `[epistemic_policy]`) — the scene ends at the halt and drains, as `--once` does; resolution is the 16-2 / 3-3 `maosctl halt resolve` path.
- **TL timestamps on scenario time.** The kernel stamps TL frames with the wall clock (kernel-Δ 0), so the scene's per-day digests are rollover witnesses, not content witnesses (content stays proven at real time by `butler_digest_18_1`).
- **DST** (D-18-4-06 A is a fixed offset), the Observer's structural-anomaly producer (D-18-4-07), and `jb8_posture_shift_cognition` (`journey_butler.rs:283-284`, Epic 9; 21-2 AC4 names its owner per the epic).

## Story

As **the director whose Butler learns from what I accept**,
I want **the Butler to notice when it no longer knows what I want, halt and ask — replayable over three simulated weeks on a keyless box — with its learning kept in my principal namespace and the option I pick doing what it said**,
so that **self-tuning is supervised upstream of degraded suggestions, my preference data stays forgettable, and no option runs an action I was not shown.**

**Contract:** epic-18 §18-4 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:167-173`, creation corrections `:170`, cites corrected in place `:168-173`), exit line 4 and its provenance (`:17`, `:25`), Kernel-Δ (`:32`), kloc asks and the 18-3 ruling (`:36`, `:38`), §R R18-16, R18-17 (`:67-68`), R18-22 (`:73`, AC3 widening), stories row (`:95`), Dependencies (`:232`: 18-1 → 18-4, 18-3 → 18-4, 21-1 consumes the `Clock`); `routing-audit-2026-10-22.md:38,110,113`; `deferred-work.md:1232-1234`; PRD Journey B `prd/user-journeys.md:9-55` (climax `:32`, resolution `:34`, halt `:36-37`, `:39`), FR17 `prd/functional-requirements.md:51`, FR31 `:77`; ADR-026 (`docs/adr/ADR-026-principal-memory-namespace.md:12-18`); ADR-028 row d (`docs/adr/ADR-028-replay-determinism-trace-shape.md:112`); ADR-064; `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `419f051f` by direct reads, the HEAD `maos` binary (rebuilt at creation) run under a scratch `HOME`, `kloc-check --json`, `check-kernel-baseline` and one reverted kernel prototype.

| Premise (epic / §R / deferral) | Fact at `419f051f` and consequence |
|---|---|
| Exit line 4 is red: unknown `--clock` | **Holds, measured:** `maos run: unknown argument '--clock' — expected: <manifest> [--live] [--once]`, exit 1 (`crates/maos-bin/src/worker_spawn.rs:82-87`; parser `:67-101`, `RunArgs` `:55-61`). |
| AC1: "a fixture whose acceptance never declines must not halt" while the rolling acceptance rate feeds `preference_alignment` → `user_preference_drift < 0.6` | **Infeasible on the seed.** The root seeds a calendar conflict (`main.rs:695-713`), so `belief_variance` = 0.8 (`spirits/butler/src/lib.rs:1117-1120`) > 0.7 (`manifest.toml:84-88`) and the halt fires on pass 1 regardless of any fixture — measured: two `--once` runs both print `{"event":"halt","render":"halted on belief_variance",…}`. A conflict-free scenario produces no notification to pick (D-18-1-R6; `lib.rs:853-860`). → D-18-4-01. |
| AC1 (rate drives the drift halt) vs AC3 (routing audit: variance-led, "rather than substituting the declining acceptance rate") | The two ACs contradict. PRD `:34-37`: the Butler's **unconditioned** policy is uncertain about a bimodal preference; the halt is `belief_variance`; the per-mode acceptance rates are its *diagnostic*; `:39` "upstream of outcome degradation". → D-18-4-01 A. |
| FR31: "persists across a daemon restart in the principal namespace" | Private-tier values that are not `Markdown` and ≤ 4 KiB live **only in the in-memory map** (`crates/maos-kernel-core/src/memory/private.rs:618-626`, `:644-675`; threshold `4 * 1024`, `crates/maos-bin/src/main.rs:2219-2222`) — a small JSON principal record is lost at process exit. `Markdown` always spills to `<memory_root>/<pid>/<ns_hex>/<key>.md` (`private.rs:3-8`, `:264-279`). → D-18-4-02. |
| (unstated) the record's owner is stable across restarts | Spill paths are keyed by the **boot-local pid**: `allocate_pid` is a process counter from 1 (`scheduler/scheduler_loop.rs:29-33`). Root Butler measured at pid 1 in two process runs; a door reload allocates a new pid. The private read path has no authorization check (`memory/mod.rs:835-860`), so a later boot's pid-1 Spirit can read Butler's spill [code-read; not executed]. → D-18-4-02 (finding routed). |
| AC3: "a Spirit-facing principal write port this story delivers" | None exists: Butler's ports are scalar / inference / MCP / output (`lib.rs:707-742`); `SpiritMemoryView` (`memory/for_spirit.rs:18-60`) is kernel glue. Everything a root-side port needs is `pub`: `MemoryManagerAdapter::for_spirit` (`memory/mod.rs:414`), Principal write enforcement (`:771-782`) with every admitted pid authorized (`crates/maos-bin/src/admission.rs:829`), `reject_principal_outside_private` (`:188-199`), index recording (`:789-798`). Zero kernel-Δ. |
| AC2: ledger "written as Transparency-Log frames" (§H-5) | TL frames are append-only; `maosctl forget` erases the private/shared stores and the principal index (`private.rs:811-826`; ADR-026 `:12`), not TL rows. A ledger **restored** from the TL would resurrect a forgotten director's preference history. → D-18-4-02 A: personal record in the principal namespace (restore source); the TL carries a non-personal attestation (I2). |
| AC2: `handle_option_pick` (`lib.rs:551`) | `spirits/butler/src/lib.rs:1039-1087`; no caller outside its unit tests (`:1628-1745`); `'a'`/`'b'`/`'c'` run fixed actions whatever the model wrote (`deferred-work.md:1233`, whose `:1022` cite is now `:1039`). The prompt (`:170-193`) says nothing about option semantics. → D-18-4-03. |
| AC3: "option picks produce their declared effects" in the scene | Butler's MCP ports are wired only on the `--live` arm (`main.rs:5571-5685`); ADR-064 forbids `replay` with `--live` (`crates/maos-bin/src/inference_mode.rs:55`). The hermetic scene has no MCP port. → D-18-4-04. |
| FR17 (c) feed (`deferred-work.md:1234`) | `ANOMALY_FEED_UNKNOWN` (`lib.rs:583-584`) overwrites the field (`:1281`). The only production anomaly source is 17-10's escape tail, which writes stderr lines only (`crates/maos-bin/src/escape_detector_consumer.rs:1-41`; spawn `:188-238`; armed `main.rs:4552-4562`). `ObserverSurface::to_notification` (`spirits/observer/src/lib.rs:401-411`) has no caller. → D-18-4-07. |
| AC2: `Clock` port in `maos-spirit-sdk`; Butler reads it | No clock in the SDK (`crates/maos-spirit-sdk/src/lib.rs:15-33`; deps `Cargo.toml:10-15`, features `:17-22`) or ABI. Workspace clock traits: `CohortClock` (`crates/maos-cohort/src/state.rs:25-27`), `QuotaClock` (`crates/maos-capability/src/cap_quota/mod.rs:65-68`). Butler's wall-clock reads: `MorningDigestRenderer::render_if_new_day` (`lib.rs:320-325`) and `chrono_stub_snooze_time` (`:1358-1371`, `SystemTime::now` `:1362`); UTC day at `:339`, `:569`, `:618-624`. `grep -c -i clock spirits/butler/src/lib.rs` = 5 (comments/strings: `:323,:336,:337,:568,:584`), not 0. |
| Exit line 4 replays `on-idle-halt.json` for 21 passes | The cassette has **1 entry** (seq 0, real `prompt_sha256 70f7ae50…`, `prompt_len` 697); replay has no leftover check (`crates/maos-bin/src/cassette_replay.rs:133-143`), so line 1 is unaffected by more entries. 21 passes need 21. AC1's command omits `MAOS_REPLAY_STRICT=1` that exit line 4 carries — corrected in place. |
| Daemon cadence | `IdleWatchdog` fires `on_idle` once per quiet period (`scheduler/idle_watchdog.rs:93-96`; `MAOS_IDLE_FAST` `:46-50`, ÷100 `:79-83`). The scene must call `fire_on_idle` itself (as `--once` does, `main.rs:6202`), never wait on the watchdog. |
| Halt line under `--once` | The Butler arm always renders `belief_variance` (`main.rs:6255-6261`, `:6306-6307`), whichever tag fired. The scene renders the fired tag (fix at origin for the scene path). |
| Sprint row "journey_butler.rs:285 un-ignored" | Stale: `:283-284` is `jb8_posture_shift_cognition` (Epic 9); `jb3_self_tunes_via_belief_variance_halt` (`jb3_self_tuning_halt.rs:54-55`) is un-ignored, single-pass, and asserts the **conflict** halt. Row comment refreshed at creation. |
| Holds | `routing-audit-2026-10-22.md:38`; FR31 `prd/functional-requirements.md:77`; FR17 `:51`; `CohortClock` `state.rs:25`; SDK ceiling `kloc.toml:271`; jb3 `:55`; `journey_butler.rs:283-284`; `ANOMALY_FEED_UNKNOWN` `:583-584`; `to_notification` `:401`; `AnomalyFlagged` `crates/maos-domain/src/notification.rs:50-65`. |

**Epic cites corrected in place (OLD → NEW @419f051f):** `worker_spawn.rs:67-99` → `:67-101`; unknown-flag error `:82-86` → `:82-87`; `--once` field `:59-60` → `:60` (struct `:55-61`); once firing `main.rs:5806` / `:5832` → `main.rs:6202` (tail `:6171`); `idle_watchdog.rs:44-58` → `:44-63`; `MAOS_IDLE_FAST` `:46-49` → `:46-50`; `handle_option_pick` `lib.rs:551` → `:1039-1087`; `preference_alignment` `lib.rs:636-645` → field `:448`, read in `assess` `:1126-1135`; drift rule `manifest.toml:92-94` → `:90-94`; `memory/principal.rs:33` → struct `:38-41` (attribute `:30-37`); SDK tokei 839 @9f920180 → 846; SDK `Cargo.toml:10-22` → deps `:10-15`, features `:17-22`; `chrono_stub_snooze_time` `lib.rs:831-843`, `SystemTime` `:834` → `:1358-1371`, `:1362`; `grep -c -i clock` = 0 → 5 (comments only); jb3 job `discipline.yml:2968` → `journey-hermetic-tier-1` `:3267`; `butler-tests` `discipline.yml:730` → `:974-991`; `journey-hermetic-tier-1` `:2128` → `:3267`.

## Kernel and kloc pricing (E16-A7) — measured at creation

**Kernel-Δ 0 (recommended rows).** No AC needs kernel source: the scene calls `HookDispatcher::fire_on_idle` (`hook_dispatch.rs:279-282`), the principal port composes `MemoryManagerAdapter::for_spirit` (`memory/mod.rs:414`) in the root, the anomaly frames use the root's TL adapter. **D-18-4-02 C priced by prototype:** `matches!(namespace, MemoryNamespace::Principal { .. }) ||` before `should_spill_to_disk` at `private.rs:644` → +1 physical (26377 → 26378, `check-kernel-baseline` FAILED, changed `memory/private.rs`) / +1 tokei (20608 → 20609); compiles, fmt-clean; reverted, `check-kernel-baseline` PASSED 26377 / 95. Identity-keyed principal storage (C's second half) was not prototyped [INFERENCE +20–50, changes `authorize_principal_writes`' signature].

**kloc at `419f051f`** (`kloc-check --json`, tokei 14.0.0, `passed: true`, `over_budget: []`):

| Crate | Measured / ceiling | 18-4 forecast [INFERENCE] | Consequence |
|---|---|---|---|
| maos-bin | 28328 / 28328 (`xtask/kloc.toml:517`); Epic 18 named budget 28444 (116 left; D-18-3-11 B) | +80–140: `--clock`/`--acceptance` parse + typed refusals (+12–20), scene loop and its events (+30–50), seam fields + restore event (+8–15), pid-fused principal port (+20–30), escape-tail anomaly frame (+10–20), `MAOS_UTC_OFFSET` env row (+5) | **D-18-4-08** |
| maos-spirit-sdk | 846 / 3000 (`:271`) | +70–130 (`clock` module, `ClockSpec` parser, `PrincipalMemoryPort` trait) | fits |
| maos-journey-test | 915 / 1104 (`:681`) | 0 (tests and data only) | fits |
| maos-domain | 9805 / 9805 (`:425`) | 0 (`NotificationEvent::AnomalyFlagged` reused) | — |
| maos-kernel-core | 20608 / 20608 (`:264`); pin 26377 (`xtask/kernel-core-baseline.toml:505`) | 0 (measured) | — |
| aggregate | 184002; `_aggregate_hardfail` 184003 (`:718`) | +150–270 | re-pin = measured + 1 at landing |

`spirits/*` is ungoverned (`xtask/src/kloc_check.rs:310-311`): the ledger, tuning rule, fixture parser, option vocabulary, anomaly reader and restore logic go there.

## Decisions — RULED 2026-10-25 by the operator (Lunarpulse)

**Ruling record.** D-18-4-01 **A**, 02 **A**, 03 **A**, 04 **A**, 05 **A**, 06 **A**, 07 **A**, 08 **A** — every row as recommended. Review on `anthropic/claude-opus-5-5` only; D-18-4-02 and D-18-4-03 get a security-focused opus reading. CR-n are creation defaults carried into the rulings.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-4-01** | Which signal is the tuning halt, and how does the scene avoid the seed's day-1 conflict halt? | **A — ledger-informed `belief_variance`:** once the 7-simulated-day window holds ≥ 7 decided offers, `belief_variance` = binary entropy H₂(p) of the window's **pooled** acceptance rate p (the unconditioned policy PRD `:37` names); before that the conflict proxy (`lib.rs:1117-1120`) applies unchanged. The halt is the existing `belief_variance on_value_above 0.7` (`manifest.toml:84-88`), no new rule; the per-work-mode rates are the halt's diagnostic; the acceptance rate never feeds `preference_alignment`. The fixture carries ≥ 7 history days (day ≤ 0) so the window is full on day 1. Accepted = a pick whose action is not `snooze`; `snooze` and `ignored` decline · B — epic AC1 verbatim (rate < 0.6 → `preference_alignment` → drift halt) · C — conditional entropy H(accept \| work_mode) · D — new tag `acceptance_entropy` + new manifest rule | **A.** H₂(p) > 0.7 ⇔ p ∈ (0.189, 0.811): with 7 offers, 2–5 accepts halt while the pooled rate can still be ≥ 0.6 (5/7 = 0.714) — the halt is *upstream* (`:39`). B halts on day 1 on the seed (measured) and contradicts AC3/R18-22. C is 0 for a perfectly bimodal director (shallow always declines, deep always accepts) — it never halts in exactly the PRD's case. D still halts on day 1 and adds a rule AC1 forbids. Cold start keeps every no-ledger path byte-identical: exit line 1, jb3, `butler_inference_18_1`, the 18-2 corpus. | RULED 2026-10-25 — A: ledger-informed `belief_variance` (entropy of the pooled 7-day rate once ≥ 7 decisions; conflict proxy until then; fixture ≥ 7 history days) — the halt leads the rate, no-ledger paths unchanged |
| **D-18-4-02** | Where the acceptance record lives, what a restart restores from, and how it is durable (FR31, ADR-026) | **A — personal record in the principal namespace, restored from it:** each decision (sim time, day, work mode, response, action, accepted) is written by Butler through a NEW pid-fused `PrincipalMemoryPort` (trait in `maos-spirit-sdk`, string values; impl in the root over `MemoryManagerAdapter::for_spirit(pid)`) as `MemoryValue::Markdown` (always spilled, `private.rs:618-626`) under `principal:<principal_id>:butler.acceptance/v1`; restore at attach reads it; the TL gets a non-personal `butler:acceptance` attestation (decision id, notification frame ref, action kind, sim time) written before the effect (I2). Labels: door reload (new pid) restores 0 records — typed `acceptance_ledger_restored {records: 0}`; the cross-boot pid-reuse read is filed to `20-3b-gate-retirement-and-coverage-generator`'s FR31 isolation witness (routing audit `:45,:155`) with a FLAG-Winston note for the kernel fix · B — TL is the ledger of record; restore from the TL by Spirit name across boots (`maos_audit::resolve_spirit_name(.., true)`, `crates/maos-audit/src/lib.rs:1663-1751`) · C — ◆ kernel: Principal values always durable (+1 / +1 measured) and keyed by a stable Spirit identity (+20–50 [INFERENCE]) | **A.** ADR-026 `:12-14` puts exactly this data (Butler watching its user) in the principal namespace so forget/SAR/redaction apply; B resurrects a forgotten director's history from the TL (rejected on RTBF). `Markdown` is the documented durable kind (`private.rs:6-8`); root pid 1 measured stable across process restarts. C is the right long-term kernel shape but buys a FLAG-Winston grant in a kernel-Δ-0 story; the finding goes to the isolation witness that already owns FR31 isolation. **Security-critical.** | RULED 2026-10-25 — A: principal namespace via a pid-bound port, `Markdown`, restored from there; non-personal TL attestation; pid-reuse finding filed `deferred-work.md:1252` → `20-3b` (received item in epic-20 before its Dependencies), security-relevant |
| **D-18-4-03** | How a pick maps to the option the model wrote (`deferred-work.md:1233`) | **A — closed action vocabulary:** the prompt declares `linear_note`, `reminder`, `snooze`; each option string is `"<action>: <text>"`; `parse_proposal` refuses an unknown, duplicate or missing action prefix (typed); `handle_option_pick(index)` runs the action the picked option names; the notification renders the action beside the text; `DETERMINISTIC_OPTIONS` gain the prefixes · B — positional mapping, labelled in the render · C — options become `{action, text}` objects | **A.** Closes the defect; keeps `options: [string]` (manifest `output_shape` `:57-60` untouched). Cost: the prompt changes, so every Butler prompt hash moves — regenerate the 18-2 halt corpus with its generator (`spirits/butler/tests/halt_eval_corpus_18_2.rs:185-188`, `#[ignore]`), re-pin `spirits/butler/tests/corpus_pin.rs:25` and `tests/corpora/MANIFEST.toml:133-139` (`[corpus."butler-calendar-comms-v0.3"]`), re-author `on-idle-halt.json`. B leaves the defect. C adds a schema change for no gain. **Security-relevant (AC4).** | RULED 2026-10-25 — A: closed action list `linear_note`/`reminder`/`snooze` in the prompt, unknown refused; the 18-2 corpus regeneration + re-pin is in T4 |
| **D-18-4-04** | What an option's effect is in the hermetic scene (no MCP port) | **A — effects through Butler's MCP port when wired; in the scene a `linear_note` pick is recorded as accepted with a typed `effect: refused (mcp unconfigured)` — never "written"; `reminder` stays the v0.4 stub, labelled; `snooze` computes its time from the `Clock`. A library witness with a scripted `ButlerMcpPort` proves each named action's effect** · B — wire MCP ports under explicit replay when `MAOS_MCP_*_URI` is set and run a fixture MCP server in `epic-18-exit` | **A.** The scene's subject is tuning; the effect contract is proven at library level as 18-8 AC5 did (scripted MCP port). B re-opens the `--live`-only MCP wiring (`main.rs:5571-5685`) and CI shape for a cosmetic exit-line effect. | RULED 2026-10-25 — A: scene effects typed `refused (mcp unconfigured)`; scripted-MCP library witness |
| **D-18-4-05** | Production pick surface and the principal id | **A — picks enter only via the scene fixture and the library API; the principal id comes from the fixture header; a new `deferred-work.md` row routes the director pick surface (and the canned `maos-shell` stub, `crates/maos-shell/src/lib.rs:509-526`) to `19-2-fr20-enqueue-door-and-safe-point` (director → running-Spirit input over the door)** · B — add `maosctl spirit pick <spirit> <n>` + a door route now (maos-control, maos-cli, maos-bin; new authenticated write route) · C — A, plus replace the shell stub with a typed "no pick surface" refusal now | **A.** B is a new door write route with its own security review and +60–120 [INFERENCE] beyond D-18-4-08. A production daemon therefore has no ledger input and stays on the conflict proxy — labelled. 19-2's ratification of the routed row is the operator's. | RULED 2026-10-25 — A: fixture + library only; pick surface and canned shell stub → `19-2` AC4 (received), `deferred-work.md:1253` |
| **D-18-4-06** | Local day (`deferred-work.md:1232`) | **A — `Clock::utc_offset_secs()`; `SystemClock` reads `MAOS_UTC_OFFSET` (`±HH:MM`, unset = `+00:00`, stated as UTC in the digest) once at boot, malformed → typed boot refusal; registered in `env_contract.rs`; the scenario clock takes the fixture's offset; the renderer's day, its clock-step guard (`lib.rs:338-346`) and the yesterday window (`:614-628`) use the local day; the digest carries `utc_offset_secs`** · B — host time zone via `chrono` / `iana-time-zone` (both already in `Cargo.lock`, 0.4.44 / 0.1.65, transitive) as direct deps · C — keep UTC (D-18-1-R5) and close the local half by ruling | **A.** Deterministic under replay (ADR-028 row d), no new dependency, testable through the same `Clock`. DST is not modelled (labelled). B adds a dependency and host-tz nondeterminism to replay output. | RULED 2026-10-25 — A: `MAOS_UTC_OFFSET`, default UTC, typed refusal; DST not modelled, labelled |
| **D-18-4-07** | FR17 (c) production anomaly feed (`deferred-work.md:1234`) | **A — the 17-10 escape tail journals each anomaly it surfaces as a TL `telemetry.event` frame `anomaly.flagged:escape-detector` (payload: the `AnomalyFlagged` fields, `confidence` 1.0) BEFORE its stderr line (I2); the production digest reads `anomaly.flagged:` frames in its clock-driven window into `NotificationEvent::AnomalyFlagged` through the existing ≥ 0.6 filter (`lib.rs:1231-1249`); `anomalies_basis` names the wired source and the unwired one (Observer structural anomalies); `ANOMALY_FEED_UNKNOWN` retires** · B — the digest re-derives escape anomalies from kind-8 rows with `maos_escape_detector` (needs per-pid declarations the TL does not carry) · C — keep the explicit unknown and re-route the feed | **A.** One producer, one durable record, one reader; the empty list is honest because the basis names what is and is not wired. B duplicates 17-10's correlation without its inputs. C leaves FR17 (c) open with no natural owner. maos-bin +10–20. | RULED 2026-10-25 — A: `anomaly.flagged:` TL frames from the 17-10 escape consumer; digest names wired sources; the unknown string retires |
| **D-18-4-08** | kloc governance | **A — raise Epic 18's named maos-bin budget 28444 → 28510 (+66; the forecast's high end ×1.3, rule 6, over 28328); exact re-pin at landing (after `cargo fmt --all`, `kloc-check --json`); above 28510 → re-ask; maos-spirit-sdk lands exact inside its ceiling; `_aggregate_hardfail` = measured + 1; kernel 0** · B — hold 28444; T0 prototype-measures maos-bin and returns a measured request before dev proceeds · C — cut scope to fit 116 (D-18-4-07 → C, the principal port impl into `spirits/butler` over a raw `MemoryManagerPort`) | **A.** Logic already sits in `spirits/butler` and the SDK; what remains in maos-bin is composition the root alone can do (pid fusion over the kernel adapter, TL writes, the scene loop). C hands a Spirit an un-fused `MemoryManagerPort` (it could write as any pid) — rejected on the 18-1 lesson. | RULED 2026-10-25 — A: named maos-bin budget 28444 → 28510 (epic `:38`, `:232`); exact re-pin at landing; above → re-ask |
| CR-1 | `Clock` shape (§H-6 resolved: SDK) | `maos-spirit-sdk::clock` (std feature): `trait Clock: Send + Sync { fn now_ns(&self) -> u64; fn utc_offset_secs(&self) -> i32; }`; `SystemClock`; `ScenarioClock` (shared, settable by the driver); `ClockSpec::parse("compressed:<n>(m\|h\|d)")` (21-1's `compressed:90m` parses too). `Butler::with_clock`; renderer and snooze read it. No SDK dependency added (`Cargo.toml:10-15`). | — |
| CR-2 | Scene driver and refusals | `--clock compressed:<n>d` (Butler: whole days; one `fire_on_idle` per simulated day after `ScenarioClock` is set to the day's start; scenario base from the fixture header, never wall time) requires `--acceptance <path>`; typed refusals: either flag without the other, with `--once`, with `--live`, on a non-Butler manifest, a fixture missing a reached day, a malformed line. Exit 0 when the scene ran (halt or not); `pass_verdict` (`crates/maos-bin/src/butler_root.rs:13-33`) applies every pass. | — |
| CR-3 | Fixture and cassette | `tests/fixtures/j-butler/acceptance-21d.jsonl` (header: `principal`, `start`, `utc_offset`; then one line per day `{day, work_mode, response}`, history days ≤ 0) and `acceptance-21d-steady.jsonl`. `on-idle-halt.json` re-authored (D-18-4-03 prompt) to 21 entries, real hashes, `provenance: seed`, no wildcard; a test pins the count. | — |
| CR-4 | Construction paths (18-1 / 18-3 lesson) | `ButlerSeam` (`lib.rs:233-245`, filled once at `main.rs:4073-4119`) gains `clock`, the principal port factory and the attestation sink; `attach` (`:253-287`) wires them and restores, so the boot arm, the door load and the hot-swap successor (`build_spirit_obj` Butler arm `main.rs:1034-1091`, callers `:3457-3470`, `:9440-9444`) are bound alike. | — |

## Acceptance Criteria

*(Epic §18-4 as corrected at creation and ruled 2026-10-25 (D-18-4-01…08 all A). The epic's AC3 is split: persistence stays AC3, option effects become AC4; the variance-led clause moves into AC1's tuning rule. AC5 is the inherited FR17 (c) deferral. One plant per control AC, `docs/testing/proof-contract.md:26`.)*

**Inherited deferred rows owned by this story (all mapped; none re-owned):**

| Row | Item | AC | Tasks | Ruling |
|---|---|---|---|---|
| `deferred-work.md:1232` | local-day rollover + a rollover test through the injectable `Clock` | AC2 (2.1–2.3, plant 2.5) | T1, T2, T3, T5 | D-18-4-06 A |
| `deferred-work.md:1233` | option pick runs the action the model wrote | AC4 (4.1–4.3, plant 4.4) | T2, T4, T5 | D-18-4-03 A, 04 A |
| `deferred-work.md:1234` | FR17 (c) production anomaly feed | AC5 (5.1–5.2, plant 5.3) | T2, T3, T5 | D-18-4-07 A |

Filed at the rulings (owned elsewhere): `deferred-work.md:1252` (pid-keyed private memory, ⚠ security → `20-3b-gate-retirement-and-coverage-generator`) and `:1253` (director pick surface → `19-2-fr20-enqueue-door-and-safe-point`). Each of `:1232-1234` closes on 18-4's landing run.

### AC1 — command: exit line 4 runs 21 simulated days and the variance-led halt fires upstream (D-18-4-01, CR-2, CR-3)

1. `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json maos run spirits/butler/manifest.toml --clock compressed:21d --acceptance tests/fixtures/j-butler/acceptance-21d.jsonl` on a keyless runner (scratch `HOME`, `MAOS_OLLAMA_URL` unset) exits 0. Per simulated day it prints `{"event":"acceptance_day", "day", "work_mode", "response", "action", "accepted", "window": {"offers", "accepted", "rate", "by_mode"}, "belief_variance"}`; the day `belief_variance` crosses 0.7 it prints `{"event":"halt", "render": "halted on belief_variance", "day", "diagnostic": {"rate", "by_mode"}}` and drains (`scene_complete` names the halt day).
2. The halt day is ≤ 21 and on it the pooled rolling rate is ≥ 0.6 (upstream, PRD `:39`); the steady fixture runs 21 days with no halt; no stdout line carries a wall-clock value (ADR-028 row d).
3. Every no-ledger path is unchanged: exit line 1, `jb3`, `butler_inference_18_1`, the 18-2 eval numbers after the D-18-4-03 regeneration.
4. CI: `epic-18-exit` (`discipline.yml:2279-2486`) runs line 4 verbatim and its steady twin; `xtask/tests/ci_shape_18_0.rs` gains the line-4 shape test beside `:255`, `:320`, `:412`. `check-exit-commands` resolves `--clock`/`--acceptance`.
5. Process witness `crates/maos-bin/tests/butler_scene_18_4.rs` (`env!("CARGO_BIN_EXE_maos")`, fresh `HOME`, no nested cargo): both fixtures; each typed refusal of CR-2.
6. **Proven red (1 plant):** in ledger-informed mode compute `belief_variance` with the conflict proxy (ignore the window) → the steady twin halts on day 1 → red.

### AC2 — the `Clock` port, local-day rollover and the acceptance attestation (D-18-4-06, CR-1; `deferred-work.md:1232`)

1. CR-1's `Clock` in `maos-spirit-sdk`; `Butler::with_clock`; `MorningDigestRenderer` takes the clock (no `SystemTime` left in `spirits/butler/src`), as does the snooze time. The serving loop's 60 s check (`main.rs:8944-8960`) and the start/`--once` renders read the same clock.
2. Local day per D-18-4-06 A: `render_at`'s day, its clock-step guard and `yesterday_predicate_fire_rate` use `(now_ns + offset) / NANOS_PER_DAY`; the digest carries `utc_offset_secs`; `MAOS_UTC_OFFSET` registered (`crates/maos-bin/src/env_contract.rs`) and refused typed when malformed.
3. Rollover tests: library (`spirits/butler/tests/`) — a `ScenarioClock` stepped across local midnight at a non-zero offset renders once per local day, never twice, and never after a step back; process — the AC1 scene prints exactly one `morning_digest` per simulated day, days strictly increasing. `digest_production_18_1.rs:9-11`'s "waits for 18-4" note is removed.
4. Each decision on days ≥ 1 goes through `handle_option_pick` and writes its `butler:acceptance` attestation frame before the effect (I2); the scene's frame count equals its decided offers; queryable with `maos audit query`.
5. **Proven red (1 plant):** make `MorningDigestRenderer` read `SystemTime` again → the process scene prints one digest, not 21 → red.

### AC3 — acceptance learning survives a restart in the principal namespace (FR31; D-18-4-02, D-18-4-05; security-critical)

1. NEW `PrincipalMemoryPort` (SDK trait) with a root impl fused to the Spirit's resolved pid (it cannot name another pid), writing `principal:<principal_id>:butler.acceptance/v1` in the Private tier as `Markdown`; Butler writes each decision through it and restores from it in `ButlerSeam::attach`, printing `{"event":"acceptance_ledger_restored","records":N,"pid":p}`.
2. Restart witness: the AC1 fixture split at day 10 across two `maos run --clock … --acceptance …` processes on one `MAOS_HOME` halts on the same day as the uninterrupted run; `maos_audit::subject_access_query` (`crates/maos-audit/src/lib.rs:1863`) lists the principal rows; `maosctl forget --principal <id>` then a restart restores 0 records (RTBF holds; the TL attestations name no work mode or response).
3. Isolation: an unauthorized pid's write is refused `PrincipalWriteUnauthorized` (`memory/mod.rs:779`); a door-loaded Butler is bound (CR-4) and reports `records: 0` for the labelled pid change; the cross-boot pid-reuse read is filed per D-18-4-02.
4. **Proven red (1 plant):** skip the restore in `attach` → the resumed half cold-starts on the conflict proxy and halts on day 11 → red.

### AC4 — a pick runs the action the model declared (D-18-4-03, D-18-4-04; `deferred-work.md:1233`; security-relevant)

1. The prompt declares the closed vocabulary; `parse_proposal` refuses an unknown, duplicate or missing action prefix (typed `ButlerError::Proposal`); `handle_option_pick` dispatches the picked option's named action; `butler_notification` shows action and text.
2. Library witness (scripted `ButlerMcpPort`, precedent 18-8 AC5): a completion listing `snooze` first and `linear_note` second — pick 2 writes the Linear note with the model's text, pick 1 snoozes at `Clock` + 15 min; an out-of-range pick consumes nothing. Scene: a `linear_note` pick records `effect: refused (mcp unconfigured)`.
3. 18-2 corpus regenerated by its generator; corpus pins re-pinned in the same commit; `maos eval halt --class butler` still meets its floors.
4. **Proven red (1 plant):** dispatch by position again → the reordered-options witness writes no Linear note on pick 2 → red. `deferred-work.md:1233` closes on the landing run.

### AC5 — FR17 (c) reads a production anomaly feed (D-18-4-07; `deferred-work.md:1234`)

1. The escape tail writes `anomaly.flagged:escape-detector` before its stderr line; the production digest lists those ≥ 0.6 in its window with `source_log_ref`s, plus `anomalies_basis` (`wired: ["escape-detector"]`, `unwired: ["observer"]`); `ANOMALY_FEED_UNKNOWN` is deleted and `butler_digest_18_1.rs:244-247` asserts the basis instead.
2. Witnesses: `escape_anomaly_tail_17_10.rs` asserts the frame precedes the line; a seeded-TL digest test lists a seeded anomaly and omits one below 0.6.
3. **Proven red (1 plant):** the digest stops reading `anomaly.flagged:` frames → the seeded anomaly is missing → red. `deferred-work.md:1234` closes on the landing run.

## Tasks / Subtasks

- [ ] **T0 — Preflight at the dev HEAD.** Every D-18-4 row RULED 2026-10-25; tracker `ready-for-dev` → `in-progress`. `git diff --stat 419f051f -- spirits/butler crates/maos-spirit-sdk crates/maos-bin/src/{main.rs,worker_spawn.rs,butler_root.rs,escape_detector_consumer.rs,env_contract.rs} crates/maos-journey-test/cassettes/j-butler .github/workflows/discipline.yml xtask/tests/ci_shape_18_0.rs tests/corpora/MANIFEST.toml`; re-anchor drift. Re-measure `kloc-check --json` / `check-kernel-baseline` (expect 28328 / 20608 / 26377 if 18-3 landed unchanged).
- [ ] **T1 — SDK** (CR-1, AC3.1): `clock` module + `ClockSpec`; `PrincipalMemoryPort` trait; unit tests.
- [ ] **T2 — Butler** (AC1–AC5): `with_clock`; acceptance record + window + D-18-4-01 rule; restore; attestation sink; option vocabulary (D-18-4-03); effects (D-18-4-04); local day; anomaly reader; `ButlerSeam` fields + `attach`.
- [ ] **T3 — Root** (AC1–AC3, AC5): parser + refusals (`worker_spawn.rs:55-101`); scene loop beside the `--once` tail (`main.rs:6171-6325`); seam fill (`:4073-4119`); pid-fused principal port; `MAOS_UTC_OFFSET`; escape-tail frame write.
- [ ] **T4 — Fixtures + cassettes** (CR-3, AC4.3): both acceptance fixtures; `on-idle-halt.json` (21 entries, the D-18-4-03 prompt); **D-18-4-03 cost:** run the 18-2 generator (`cargo test -p butler --test halt_eval_corpus_18_2 -- --ignored`, `halt_eval_corpus_18_2.rs:185-188`) to rewrite `calendar-comms-v0.3.jsonl` and the three `halt-corpus-v0.3*.json` cassettes, then re-pin `corpus_pin.rs:25` and `tests/corpora/MANIFEST.toml:133-139` in the same commit; `maos eval halt --class butler` must still meet its floors.
- [ ] **T5 — Witnesses**: `butler_scene_18_4.rs` (AC1, AC2.3 process, AC3.2–3.3); Butler library tests (AC2.3, AC4.2); 17-10 tail + digest tests (AC5.2); `butler_digest_18_1` basis assertion.
- [ ] **T6 — CI** (AC1.4): `epic-18-exit` line 4 + steady twin; `ci_shape_18_0.rs`.
- [ ] **T7 — Plants and records:** AC1–AC5 once each in `18-4-evidence/plants.md` (command, output, wall-clock); a CI plant for the line-4 twin; epic §18-4 if anything moved (the D-18-4-02 / D-18-4-05 rows are already filed, `deferred-work.md:1252-1253`).
- [ ] **T8 — Governance** (D-18-4-08 A): `cargo fmt --all`; `kloc-check --json`; maos-bin re-pinned at its exact figure ≤ 28510 (above → stop and re-ask), exact re-pins with HISTORY notes; `check-kernel-baseline` (unchanged), `check-exit-commands`, `check-env-contract`, `check-corpus`, `cassette-age-gate`, `check-test-cost`, `check-dev-record-completeness`; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule --test ci_shape_18_0`.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| SDK | `crates/maos-spirit-sdk/src/` (new `clock`, `principal` modules; `lib.rs:15-33` re-exports) | no clock, no memory port → both |
| Butler | `spirits/butler/src/lib.rs`: seam `:228-288`, renderer `:298-361`, `NotificationPayload` `:97-107`, `DETERMINISTIC_OPTIONS` `:112-116`, `parse_proposal` `:133-165`, `proposal_prompt` `:170-193`, `NANOS_PER_DAY`/UTC `:567-569`, `ANOMALY_FEED_UNKNOWN` `:583-584`, yesterday window `:614-643`, struct `:707-742`, `idle_pass` `:797-919`, `handle_option_pick` `:1039-1087`, `assess` `:1111-1154`, digest `:1163-1285`, snooze `:1358-1371` | wall clock, UTC, fixed actions, conflict-only variance → `Clock`, local day, declared actions, ledger-informed variance |
| Root | `crates/maos-bin/src/worker_spawn.rs:55-101`; `main.rs:4073-4119` (seam), `:5519-5703` (Butler arm), `:1034-1091` (`build_spirit_obj`), `:6171-6325` (`--once` tail; scene beside it), `:8944-8960` (rollover timer), `:2694-2715` (memory adapter), `:4552-4562` + `escape_detector_consumer.rs:188-238` (tail); `env_contract.rs` | `--live`/`--once` only → scene flags |
| Data | `crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json`, `halt-corpus-v0.3*.json`; `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl`; `tests/fixtures/j-butler/` (new) | 1-entry seed → 21 entries; prompt hashes regenerated |
| CI | `.github/workflows/discipline.yml:2279-2486`; `xtask/tests/ci_shape_18_0.rs` | lines 1–3 → + line 4 and twin |

### Current state that must be preserved

- **No-ledger Butler is byte-identical** in its halt behaviour: the conflict proxy, exit line 1, `jb3` (`jb3_self_tuning_halt.rs:54-97`), `butler_inference_18_1`, the 18-2 corpus semantics (only hashes and option prefixes move).
- **18-1 seam semantics** (`lib.rs:253-287`): boot refusal for an unregistered provider, per-pass mint, digest hook; refused passes stay typed (`pass_verdict`).
- **18-8 AC5**: both-fetches-failed refusal (`lib.rs:816-818`).
- **ADR-064 lattice** (`inference_mode.rs:28-67`); unset mode stays deterministic.
- **Kernel**: no file under `crates/maos-kernel-core/src` changes; `scheduler/idle_watchdog.rs` untouched (21-1 relies on it, epic-21 `:53`).
- **17-10 tail**: exactly-once cursor semantics and its stop report (`escape_detector_consumer.rs:15-41`).

### Guardrails

- Never a silent fallback: a fixture gap, a malformed option, a failed restore, a refused effect are typed and visible.
- Spirit-side seams bound on every construction path (CR-4); daemon behaviour witnessed at process level.
- The port is pid-fused at the root — a Spirit never receives a raw `MemoryManagerPort`.
- No personal field (work mode, response, option text) in a TL frame; the principal namespace holds them.
- Scene stdout is clock-deterministic (ADR-028 row d): scenario time only.
- One plant per control AC, restored by `git checkout -- <file>`; one target dir; `env -u CI -u GITHUB_ACTIONS`; no cargo inside a `#[test]`. Revert `intent-lineage-coverage-report.md` if a run modifies it.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Ledger-informed variance (AC1) | conflict proxy in ledger mode | steady twin halts day 1 |
| Clock-driven rollover (AC2) | renderer reads `SystemTime` | 1 digest, not 21 |
| Principal restore (AC3) | skip restore in `attach` | resumed half halts day 11 |
| Declared-action dispatch (AC4) | dispatch by position | no Linear note on pick 2 |
| Anomaly feed (AC5) | digest ignores `anomaly.flagged:` | seeded anomaly missing |

In CI the steady twin of line 4 is the standing red for exit line 4.

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-spirit-sdk -p butler
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test butler_scene_18_4 --test butler_inference_18_1 --test butler_digest_18_1 --test butler_daemon_18_1 --test escape_anomaly_tail_17_10 --test eval_halt_18_2
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test --test jb3_self_tuning_halt --test journey_butler
cargo build --locked -p maos-bin --bin maos && MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json target/debug/maos run spirits/butler/manifest.toml --clock compressed:21d --acceptance tests/fixtures/j-butler/acceptance-21d.jsonl
cargo run -p xtask -- check-exit-commands && cargo run -p xtask -- check-env-contract && cargo run -p xtask -- check-corpus
cargo run -p xtask -- kloc-check --json && cargo run -p xtask -- check-kernel-baseline && cargo run -p xtask -- check-test-cost
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-57`)

- **Model:** `anthropic/claude-opus-5-5` (fable only with operator permission). **Security layer (AC3, AC4):** pid fusion and authorization of the principal port; no personal data in TL frames; RTBF after restart; the pid-reuse finding's routing; the action shown equals the action run; parse refusals on hostile option text.
- **Focused suites:** `-p maos-spirit-sdk`, `-p butler`, `-p maos-bin` (the tests above), `-p maos-journey-test` (`jb3`, `journey_butler`). No kernel source change → no kernel `--lib` run unless landing measures one.
- **Gates:** `check-exit-commands`, `check-env-contract`, `check-corpus`, `cassette-age-gate`, `kloc-check`, `check-kernel-baseline`, `check-test-cost`, `check-dev-record-completeness`.

### Previous-story intelligence

- **18-1 (done):** the seam (`ButlerSeam`, `lib.rs:228-288`; root fill `main.rs:4073-4119`), `pass_verdict`, the digest renderer and its R4/R5 rulings (60 s check, UTC + `day <= d`), and the measured once-per-quiet-period daemon finding (`18-1-…md:345`) that forced `unload`/`load`/`start` witnesses. Its review deferred the three rows this story closes.
- **18-8 (done):** the scripted-MCP-port + counting-inference witness pattern (AC5) reused by AC4.
- **18-2 (done):** the halt corpus generator and pins D-18-4-03 regenerates; corpora in `tests/corpora/MANIFEST.toml`.
- **18-3 (review):** "one binding on every construction path" (D-18-3-04), typed refusals (D-18-3-05), the named maos-bin budget 28444 shared with this story, and the measured fact that no dispatcher `Clock` is needed (epic `:232`).
- **17-10:** the escape tail and its exactly-once cursor; **17-11:** declared-provider mint (unchanged here).

### Git intelligence

`419f051f` 18-1/18-8/18-2 done + 18-3 dev pass; `ee395748` 18-2; `140b906b` 18-8; `a85673f0` 18-1 (seam, `epic-18-exit`). None touched `handle_option_pick`, the private memory store or the escape tail since 17-10.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-4 `:167-173` (creation corrections `:170`), exit block `:11-26` (`:17`, `:25`, `:26`), Kernel-Δ `:32`, kloc `:36`, `:38`, rules 9–11 `:40-42`, §R R18-16/17 `:67-68`, R18-22 `:73`, stories row `:95`, Dependencies `:232`.
- `routing-audit-2026-10-22.md:38,45,110,113,155`; `prd/user-journeys.md:9-55`; `prd/functional-requirements.md:51,77`; `deferred-work.md:1232-1234`; `docs/adr/ADR-026-principal-memory-namespace.md:12-18`; `docs/adr/ADR-028-replay-determinism-trace-shape.md:112`; `docs/testing/proof-contract.md:22-27,31-57,59-81`; epic-21 `:21,:42,:53,:55,:119`; §H forks `review-2026-09-04/preflight-r2/epic-18.md:184`.

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `419f051f`, clean tree. Inputs: four read-only scouts (Butler crate, root, kernel ports/SDK, docs/deferrals) plus direct re-reads of every decision-bearing range; HEAD `maos` rebuilt (`cargo build -p maos-bin --bin maos`) and run: exit line 4 (rc 1, unknown `--clock`) and `--once` twice on one scratch home (both pid 1, both `halted on belief_variance`, anomalies = the unknown string); `kloc-check --json` and `check-kernel-baseline` at HEAD; the D-18-4-02 C kernel prototype (priced, reverted, baseline PASSED). `git status` after the revert showed only this story's doc edits.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25)

### Debug Log References

### Completion Notes List

### File List

- `_bmad-output/implementation-artifacts/18-4-self-tuning-halt-with-injectable-clock.md` (A)
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md` (M — exit-line-4 provenance `:25`, shape `:26`, 18-4 kloc ruling `:38`, §R R18-16/17 `:67-68`, stories row `:95`, §18-4 `:168-173` incl. creation corrections + rulings `:170`, Dependencies `:232`)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (M — 18-4 row only: comment at creation; `backlog` → `ready-for-dev` at the rulings)
- `_bmad-output/implementation-artifacts/deferred-work.md` (M — rows `:1232-1234` mapped; `:1252-1253` filed at the rulings)
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md` (M — §19-2 Closes `:67`, received AC4 `:72`)
- `_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md` (M — 20-3b received item before Dependencies `:248-251`)

### Change Log

- 2026-10-25 — Story created (backlog; D-18-4-01…08 PROPOSED) at `419f051f`. Exit line 4 measured red. Epic AC1's tuning rule is infeasible on the seed (conflict halts pass 1) and contradicts AC3 → D-18-4-01 (ledger-informed `belief_variance`, upstream of the rate). Small Private-tier values are not durable and spills are keyed by the boot-local pid → D-18-4-02 (principal namespace as restore source, `Markdown`, finding routed; kernel alternative priced +1/+1 and reverted). Option mapping, scene effects, pick surface, local day, anomaly feed, kloc → D-18-4-03…08. Kernel-Δ 0. Epic §18-4, exit-line-4 provenance, R18-16/17 and the stories row re-anchored in place; tracker comment refreshed.
- 2026-10-25 — RULED (operator, Lunarpulse): D-18-4-01…08 all A as recommended (08: Epic 18 named maos-bin budget 28444 → 28510, exact re-pin at landing, above → re-ask). Review `anthropic/claude-opus-5-5` only; security-focused reading on D-02/D-03. Filed `deferred-work.md:1252` (⚠ pid-keyed private memory → 20-3b; received item in epic-20) and `:1253` (director pick surface → 19-2 AC4, received in epic-19); `:1232-1234` mapped to AC2/AC4/AC5. Status → `ready-for-dev`.
