# Story 2.4: Seed the spirit-test SDK with LCAS Framework and Cross-Spirit Isolation Hooks

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

**Epic:** 2 — Spirit ABI + Developer SDK + Boundary Contracts (v0.1 → v0.3)
**Epic state at story open:** `epic-2: in-progress` (flipped at Story 2.1 creation; unchanged — Story 2.4 is the **last** story in Epic 2 before `epic-2-retrospective` becomes available).
**Story key:** `2-4-seed-the-spirit-test-sdk-with-lcas-framework-and-cross-spirit-isolation-hooks`
**Story file:** `_bmad-output/implementation-artifacts/2-4-seed-the-spirit-test-sdk-with-lcas-framework-and-cross-spirit-isolation-hooks.md`
**Predecessors:**
- Story 2.1 — full Spirit ABI (`Spirit` trait + 11 hooks at `crates/maos-spirit-abi/src/lifecycle.rs`, `SpiritVtable<T>` with `#[repr(C)]`, `#[spirit]` proc-macro at `crates/maos-spirit-derive/src/lib.rs`, `Ctx` at `crates/maos-spirit-abi/src/ctx.rs` with `mock()` constructor behind the `mock` feature, `CancellationSignal` trait + `NeverCancel` reference impl, payload types `FramePayload`/`TelemetryEventPayload`/`SchedulePayload`/`SwapInPayload`/`ConsolidatePayload`, `__maos_spirit_vtable_<Type>()` symbol per Spirit type, SDK façade re-exports). **This is the trait + dispatch surface that the spirit-test SDK seed extends.**
- Story 2.2 — `cargo xtask check-service-boundary` P1–P4 full implementation + Spirit-ABI type reflection (`check_spirit_abi_types`) + 24 spirit-boundary invariant cases at `tests/corpora/spirit-boundary-v0.1.jsonl`. **The JSONL corpus schema + `tests/corpora/MANIFEST.toml` registration pattern Story 2.4's LCAS corpus reuses verbatim.**
- Story 2.3 — thin `cargo-generate` template at `templates/spirit-rust/` + `LocalRunner` at `crates/maos-spirit-sdk/src/local_runner.rs` (gated by `local_runner` feature; `local_runner = ["std", "mock"]`) + baked example at `examples/example-spirit/` + `xtask example-spirit-regen [--check]` drift detector + `example-spirit-tests` + `example-spirit-drift` discipline jobs (30 total jobs). `LocalRunner::run(&spirit, vtable, &fixture) -> RunReport` is the substrate Story 2.4 extends — adding IAC frame capture, halt-resolution simulation, manifest self-check, assertion macros, and cross-Spirit isolation hook points. **Story 2.4 does NOT replace `LocalRunner`; it adds a sibling `SpiritTest` harness in the same crate that wraps `LocalRunner` and grows the surface.**

**Successor stories:** Epic 2 closes with this story. **No further bridge stories anticipated for Epic 2.** The next Epic 2 artifact is `epic-2-retrospective` (currently `optional` per sprint-status.yaml line 94) — author it after Story 2.4 lands per Epic 1a/1b/2.0 retro discipline if any cross-story drift surfaced (e.g., D-items, doc-catch-up, or repeated-mistake patterns).

**Downstream consumers (in dependency order):**
- **Story 4.5** at v0.8 — authors the 200-scenario NFR-Sec-14 cross-Spirit memory isolation corpus across 8 categories (namespace enumeration / working-memory read-across / decision-frame observation / halt-signal observation / transparency-log cross-read / working-memory-digest cross-read / capability-token forgery cross-Spirit / sandbox-escape lateral) using the framework hook points Story 2.4 ships (AC3).
- **Story 8.1** at v0.3 — Butler reference Spirit's acceptance tests consume the spirit-test SDK seed (assertion macros + halt resolution simulator + manifest self-check) for the J-Butler journey end-to-end test (`on_idle` + morning-digest emit + halt-recall ≥0.90 on calendar-conflict subset).
- **Story 7.1** at v0.5+ — full spirit-test SDK with per-language assertion macros (TS/Python/Go) extends the Rust seed Story 2.4 ships; AC1's assertion-macro surface is the canonical reference shape.
- **Story 7.5b** at v0.3 — N=12 stratified NFR-Onb-1 30-Min First Spirit Validation Gate uses the spirit-test SDK seed (specifically `assert_emits_frame!` + `manifest_self_check!`) as part of the "passing CI" criterion the gate verifies on each participant's Spirit.
- **Story 4.1** at v0.5 — full halt protocol mechanism (3 resolution kinds — `provided_context` / `accepted_halt` / `authorized_override` — + `HaltReceipt` 99.9%) hardens the halt-resolution simulator Story 2.4 ships into runtime production code. The simulator's enum names and method shapes are the forward-anchor contract.
- **Stories 8.2 + 8.3 + 8.4 + 8.5** (Researcher / Observer / Founder-Loop / Mira-Nash reference Spirits at v0.5–v1.5) — each Spirit's acceptance test suite extends the SDK seed pattern; the SDK seed is the canonical reference shape for "how a reference Spirit's tests are organized" per Story 8.x dev-record convention.
- **Story 10.2** at v1.5 — third-party trial N=12 (NFR-Test-8) uses the SDK seed as the "spirit-test" tool participants are expected to know about within the 14-day no-DM-support window (the participant who reads `spirit-development-and-sharing.md` §13 reaches `spirit-test` via the §13 link).

## Story

As a **test architect responsible for ensuring the substrate has the test-harness surfaces it needs at v0.3 so that downstream story authors (Story 4.5 NFR-Sec-14 corpus, Story 8.1 Butler acceptance, Story 7.5b NFR-Onb-1 30-Min gate, Story 4.1 halt-protocol mechanism) inherit a working harness — NOT a retrofitted-at-v1.0 scramble**,
I want **(1) the spirit-test SDK seed shipped as a new `spirit_test` module inside `crates/maos-spirit-sdk/src/spirit_test/` (gated behind a new `spirit_test` cargo feature that depends on `local_runner` + `std` + `mock` — `spirit_test = ["local_runner", "std", "mock"]`) exposing a `SpiritTest<S: Spirit>` harness that wraps `LocalRunner` and adds: (a) IAC frame I/O capture via a `MockBus` API hook authors can drive (`bus.deliver_frame(...)`, `bus.assert_emitted(...)`, `bus.frames_sent_by_spirit() -> Vec<MockBusFrame>`) — promoting Story 2.3's forward-anchor `MockBusFrame`/`MockBusFrameKind` types from "reserved" to working capture surfaces; (b) a halt-resolution simulator with the three resolution kinds documented in architecture §6.3 + epic-3 line 92 (`HaltResolutionKind::ProvidedContext { context_bytes: Vec<u8> }`, `HaltResolutionKind::AcceptedHalt`, `HaltResolutionKind::AuthorizedOverride { override_marker: Vec<u8> }`) — these are FORWARD-ANCHOR enum variants matching the future Story 4.1 `HaltResolver` trait contract from `crates/maos-domain/src/invariants/i14.rs` + epic-4 line 46's `MockHaltResolver` pattern; the simulator method `harness.resolve_halt(halt_id, HaltResolutionKind::...)` records the resolution in the report and (at v0.3 prerequisite) does NOT yet invoke an `on_epistemic_resolve` hook (that hook ships at Story 4.1 per `crates/maos-spirit-abi/src/lifecycle.rs` lines 8-17 deferred-hook table); (c) a manifest self-check primitive `harness.manifest_self_check(&manifest_toml_bytes) -> Result<ManifestSelfCheckReport, ManifestSelfCheckViolation>` that runs the existing NFR-Test-13 walker pattern (well-formed / malformed-rejected / edge-case) from `crates/maos-kernel-core/tests/fixtures/manifest/` — accepts raw TOML bytes, parses via existing section parsers (`ClassSection::from_toml_str`, `CapabilitiesRequired::from_toml_str`, `Posture::from_toml_str`, `OutputShape::from_toml_str`, `Budget::from_toml_str`, `Resources::from_toml_str`, `Sandbox::from_toml_str`), returns a typed report listing parsed sections + violations + edge-case warnings; this primitive does NOT yet invoke `SecurityManagerAdapter::admit_spirit` (admission is kernel-side, Story 5.1 / Story 1b.3); (d) a class-specific regression-corpus skeleton — `pub struct RegressionCorpus { pub class: SpiritClass, pub cases: Vec<RegressionCase> }` + `pub enum SpiritClass { Anticipatory, Exploratory, FounderLoop, DiagnosticArchitect, Generic }` (matching architecture §6 reference-Spirit taxonomy: Butler=Anticipatory, Researcher=Exploratory, Orchestrator+Worker+Architect+Reviewer=FounderLoop, Mira+Nash=DiagnosticArchitect, third-party=Generic; the regression-corpus type is a typed CONTAINER — actual class-specific corpora ship in Stories 8.1–8.5); (e) **assertion macros** — `assert_emits_frame!(report, predicate)`, `assert_halts_with!(report, kind_predicate)`, `assert_hook_fired!(report, hook_name, expected_count)`, `assert_no_capability_invocation!(report, scope)`, `assert_manifest_well_formed!(self_check_report)` — each macro panics with a structured diagnostic on failure and is gated `#[cfg(any(test, feature = "spirit_test"))]` so production builds don't pay the dependency cost; (2) the LCAS (Long-context Ambiguity Stress) framework + clearly-decidable bucket — 70 items at `tests/corpora/lcas-v0.3.jsonl` (JSONL, one item per line, deterministic sort by `id`, RFC 8259 strict; mirroring the Story 2.2 `spirit-boundary-v0.1.jsonl` shape) with each item carrying a `class` field (always `"clearly_decidable"` for the v0.3 ship; the `"genuinely_ambiguous"` n=70 and `"adversarially_misleading"` n=70 buckets land at Story 8.x at v0.8 per Epic 2 line 21 explicit deferral; combined ship-floor is N=210 at v0.5 per PRD line 80), `gold_label` (one of `"halt"` / `"continue"` — the canonical ground-truth for halt-recall + halt-precision measurement per NFR-Test-4), `trajectory_text` (a synthesized long-context trajectory ≥4096 chars + ≤16384 chars to exercise the v0.3-relevant context-window pressure surface; v0.5 + v1.5 expansion to longer trajectories is Story 7.1 + Story 10.2), `planted_claim` (the load-bearing claim the Spirit must surface — at the clearly-decidable bucket this is unambiguous; the genuinely-ambiguous + adversarially-misleading buckets in Story 8.x ship the harder cases per PRD NFR-Test-6 AMENDED note: *"Adversarial trajectories contain a planted load-bearing claim contradicting a louder repeated claim"*), `expected_signals` (an array of expected `[halt_tag]` strings the well-behaved Spirit should emit — derivable from `gold_label = halt` cases only); the corpus is registered in `tests/corpora/MANIFEST.toml` with `[corpus."lcas-v0.3"]` block carrying SHA-256 + schema_version=1 + item_count=70 + valid_until=2027-05-16 + prompt_version_hash + description + no judge_id at v0.3 (gate is structural at v0.3, judge-LLM agreement layer is Story 7.1 + Story 8.x at v0.5+); the corpus IS authored by hand at v0.3 (70 items is the manual-authoring practical ceiling; the v0.8 expansion to N=210 uses `maos-corpus-gen::lcas` generator authored at Story 7.1 per the existing `maos-corpus-gen` precedent for `secret_redaction` + `red_team` corpora); a companion Rust harness at `crates/maos-spirit-sdk/tests/lcas_smoke.rs` (gated `#[cfg(feature = "spirit_test")]`) parses the JSONL, verifies the SHA-256 against MANIFEST.toml, and asserts each item is well-formed (gold_label ∈ {`halt`, `continue`}; trajectory_text length in range; expected_signals empty iff gold_label=`continue`; planted_claim non-empty); (3) the NFR-Sec-14 cross-Spirit memory isolation framework hooks — a new `pub mod isolation;` inside `spirit_test/` exposing: `pub struct CrossSpiritIsolationFixture<A: Spirit, B: Spirit> { pub spirit_a: A, pub vtable_a: SpiritVtable<A>, pub spirit_b: B, pub vtable_b: SpiritVtable<B>, pub attack_corpus: Vec<IsolationAttackCase> }` + `pub enum IsolationAttackCategory { NamespaceEnumeration, WorkingMemoryReadAcross, DecisionFrameObservation, HaltSignalObservation, TransparencyLogCrossRead, WorkingMemoryDigestCrossRead, CapabilityTokenForgeryCrossSpirit, SandboxEscapeLateral }` (the 8 categories per architecture §8.1 isolation corpus line 13 + epic-4 line 17) + `pub struct IsolationAttackCase { pub id: String, pub category: IsolationAttackCategory, pub attack_payload: Vec<u8>, pub expected_isolation_maintained: bool }` + `pub trait IsolationHookPoint { fn before_spirit_a_attempt(&mut self) -> IsolationHookOutcome; fn after_spirit_a_attempt(&mut self, attempt_result: AttemptResult) -> IsolationHookOutcome; fn before_spirit_b_observe(&mut self) -> IsolationHookOutcome; fn after_spirit_b_observe(&mut self, observation: ObservationResult) -> IsolationHookOutcome; }` + a `pub struct DefaultIsolationHook;` reference impl that records all hook firings into a `Vec<HookCallRecord>` for inspection — the harness method `fixture.run_attack_case(case: &IsolationAttackCase, hook: &mut dyn IsolationHookPoint) -> IsolationOutcome` walks: hook.before_spirit_a_attempt → Spirit-A fires hook (e.g., on_frame with the attack_payload) → hook.after_spirit_a_attempt → hook.before_spirit_b_observe → Spirit-B fires hook (e.g., on_idle to drain observable state) → hook.after_spirit_b_observe → returns `IsolationOutcome { isolation_maintained: bool, hook_records: Vec<HookCallRecord> }`; at v0.3 prerequisite there is NO attack corpus committed (the 200-scenario corpus is Story 4.5 at v0.8; the type surfaces ship NOW so Story 4.5 has a working substrate not retrofitted scaffolding); a single end-to-end smoke test at `crates/maos-spirit-sdk/tests/isolation_smoke.rs` constructs a 2-Spirit fixture with a single trivial attack case + `DefaultIsolationHook` and asserts the 4 hook points fire in order + the returned outcome is well-formed; the smoke test PROVES the framework wiring works without requiring a real attack corpus; (4) `tests/coverage-matrix.yaml` updated additively — `FR34.gates` adds `spirit-test-tests` + `lcas-corpus-tests` (joining the existing `example-spirit-tests` from Story 2.3); `NFR-Test-6` row gains `gates: [lcas-corpus-tests]` + `corpora: [lcas-v0.3]` + `notes: "Story 2.4 ships the LCAS clearly-decidable 70-item bucket at lcas-v0.3.jsonl; genuinely-ambiguous + adversarially-misleading buckets (140 items) land at Story 8.x at v0.8 (require A2A scenarios from E6 to be valid); full N=210 at v0.5 ship gate per PRD line 208."` + `phase` stays at v0.5 (Story 2.4 ships PARTIAL bucket — v0.5 phase reflects when the corpus REACHES ship-gate completeness at full N=210); `NFR-Sec-14` row gains `gates: [isolation-framework-tests]` + `notes: "Story 2.4 ships the cross-Spirit isolation framework HOOKS (IsolationHookPoint trait + CrossSpiritIsolationFixture + 8-category IsolationAttackCategory enum + DefaultIsolationHook reference impl). The 200-scenario adversarial corpus (Sec-14a same-Host n=100 + Sec-14b cross-Host n=100 per ADR-040) is Story 4.5 at v0.8."` + `phase` stays at v0.8 (framework hooks at v0.3; corpus + execution at v0.8); `NFR-Test-3` (SDK coverage ≥80%) gains a notes-only update: `"Story 2.4 ships spirit-test SDK seed (assertion macros + IAC frame I/O + halt resolution simulator + manifest self-check + class-specific regression corpus skeleton + cross-Spirit isolation framework hooks). The SDK seed counts toward the ≥80% coverage floor measured at Story 7.5b + Story 10.2 N=12 third-party trial."`; (5) two new discipline jobs in `.github/workflows/discipline.yml` — `spirit-test-tests` (runs `cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke`) + `lcas-corpus-tests` (runs `cargo test -p maos-spirit-sdk --features spirit_test --test lcas_smoke`) + `isolation-framework-tests` (runs `cargo test -p maos-spirit-sdk --features spirit_test --test isolation_smoke`) — all 3 appended to the discipline-summary `needs:` list (the Story 2.3 needs list at line 535 already extends through `example-spirit-drift`) and to the PR-comment table at lines 540–640, taking the gate count from 30 (post-2.3) → 33; (6) architecture-doc adjustments — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 layout addendum gains a one-line note about the new `spirit_test` feature on `maos-spirit-sdk` (no new workspace member — `spirit_test` is a feature-gated module inside the existing crate; the member count stays at 22), `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5 gains a ≤8-line addendum titled `**v0.3 prerequisite — spirit-test SDK seed (Story 2.4):**` citing the assertion macros + halt resolution simulator + manifest self-check + isolation framework hooks + LCAS clearly-decidable 70-bucket, `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` (the §8.1 isolation corpus section) gains a ≤4-line addendum noting that Story 2.4 ships the IsolationHookPoint framework HOOKS (not the corpus); `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` gains a top-of-file callout (immediately after the existing Story 2.3 callout at line 23) noting v0.3-prerequisite spirit-test SDK seed lands in Story 2.4; (7) tests/corpora/MANIFEST.toml updated additively with the `[corpus."lcas-v0.3"]` block + `valid_until = "2027-05-16"` + SHA-256 computed via `cargo run -p xtask -- check-corpus --register lcas-v0.3`; (8) the discipline gate sweep — all 33 jobs in `discipline.yml` pass locally + via the PR commit's `discipline.yml` run (per A8 retro action), the existing `tests/integration/v01_evaluator_path.sh` passes cold (per A6), the existing `tests/integration/onb_nfr2_timing.sh` stays green, `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` reports zero added/changed/removed against `maos-spirit-abi` (the spirit_test module lives in `maos-spirit-sdk` outside the gate's scope; `MockBusFrame`/`MockBusFrameKind` types from Story 2.3 stay in `maos-spirit-sdk` — verify before claiming green per Story 2.3 dev record's scoping note)**,

so that **(a) Story 4.5's 200-corpus authoring (NFR-Sec-14 — P0 ship-blocker per PRD line 47 + architecture §8.5 row 7) inherits a working IsolationHookPoint framework + 8-category enum + 2-Spirit fixture instead of having to invent the harness mid-story (which is the failure mode that triggered the Epic 2 line 14 "framework hooks at v0.3, corpus at v0.8" architectural split); (b) Story 8.1's Butler reference Spirit acceptance tests (NFR-Onb-1 v0.3 gate substrate per Story 2.3 + epic-8 line 7's *halt-recall ≥0.90 on calendar-conflict subset*) can be written against `assert_emits_frame!` + `assert_halts_with!` + `manifest_self_check!` macros — the macros LOWER the cognitive load for the J-Butler journey end-to-end test from "invent assertion ergonomics + invoke `LocalRunner` raw" to "declare expected behavior in 1 line per AC"; (c) Story 7.5b's N=12 stratified NFR-Onb-1 30-Min First Spirit Validation Gate execution (PRD line 122 + architecture §13 row "v0.3 — Butler") has a working `spirit-test` tool participants discover via the `spirit-development-and-sharing.md` §13 link — Diego J6's pattern *"runs `spirit-test` on a corpus of 50 known-buggy code examples"* (architecture §10.6 line 130) becomes operationally reproducible at v0.3 against the SDK seed shipped here; (d) Story 4.1's halt-protocol mechanism (3 resolution kinds + HaltReceipt 99.9% + halt-recall ≥0.7 / halt-precision ≥0.85 per NFR-Test-4) inherits the simulator's enum names and method shapes verbatim — the v0.3 simulator IS the forward-anchor contract; (e) Story 7.1's full per-language spirit-test SDK (TS/Python/Go) at v0.5+ has a canonical Rust reference shape to mirror — Story 7.1 ships the per-language port of the assertion-macro set Story 2.4 commits; (f) the LCAS NFR-Test-6 ship-gate (N=210 at v0.5 per PRD line 80) becomes incrementally achievable rather than a v0.5 cliff — 70 items at v0.3 + 70 items at Story 8.x at v0.8 + 70 items at Story 7.1 / Story 8.x at v0.5+ (the Story 8.x split is what Epic 2 line 21 explicitly commits — *"raised from 60 for statistical power; Mann-Whitney U at p<0.01 needs ~64 per group at power=0.84"*); (g) Epic 2 closes with its epic-line "Acceptance demo" (line 25) fully provable: *"External developer clones `spirit-template`, implements `on_idle`, runs `cargo test` (which invokes spirit-test SDK harness), gets passing report — without reading kernel internals"* — Story 2.3 shipped the template + LocalRunner, Story 2.4 ships the spirit-test SDK harness with assertion macros so `cargo test` reports against named assertions rather than raw report-field inspection; (h) the v0.3 sprint dependency-DAG entries (`_bmad-output/planning-artifacts/epics/dependency-dag.md` line 28: `Story 2.4 spirit-test SDK seed → Story 8.1 Butler acceptance + Story 4.5 NFR-Sec-14 framework + Story 4.1 halt-protocol simulator → Story 7.1 full SDK + Story 7.5b NFR-Onb-1 gate execution`) become traversable; (i) Epic 2 retrospective opens with a CLEAN sprint (no D-items carried over from 2.4 to a bridge story; the spirit of "ship the SDK seed at v0.3 not v1.0" is preserved without spillover)**.

## What this story IS

- **A spirit-test SDK SEED, not a full SDK.** Story 2.4 ships: assertion macros (5), IAC frame I/O capture (promoting Story 2.3's forward-anchor `MockBusFrame` types to working capture), halt-resolution simulator (3 kinds, forward-anchor for Story 4.1), manifest self-check primitive (delegates to existing section parsers, NOT to `SecurityManagerAdapter`), class-specific regression-corpus skeleton (typed container only — actual class corpora ship in Stories 8.1–8.5), cross-Spirit isolation framework hooks (4 hook points + 8 attack categories + 2-Spirit fixture). **Full SDK with per-language assertion macros (TS/Python/Go), judge-LLM agreement layer, full attack-corpus library, full registry-publish path is Story 7.1 + Story 7.2 at v0.5+.**
- **LCAS clearly-decidable bucket ONLY (70 of 210).** Genuinely-ambiguous (n=70) + adversarially-misleading (n=70) buckets are explicitly deferred to **Story 8.x at v0.8** per Epic 2 line 22 (*"adversarial bucket REQUIRES A2A scenarios from E6"*) + Epic 8 line 23 (*"E2 owns clearly-decidable; E8 owns the remaining 140 — timed for v0.8 when A2A exists"*). Story 2.4 ships the 70-item clearly-decidable bucket + a `tests/coverage-matrix.yaml` `NFR-Test-6` notes update making the partial-ship explicit (the `phase: v0.5` is preserved — Story 2.4 does NOT promote NFR-Test-6 to ship-gate-met; it ships the FIRST third of the ship-gate corpus).
- **NFR-Sec-14 framework HOOKS, not the corpus.** Story 2.4 ships `IsolationHookPoint` + `CrossSpiritIsolationFixture` + `IsolationAttackCategory` (8 variants) + `DefaultIsolationHook` reference impl + a 2-Spirit smoke test proving the wiring. **The 200-scenario adversarial corpus is Story 4.5 at v0.8 (Sec-14a n=100 same-Host + Sec-14b n=100 cross-Host per ADR-040).** The framework hooks Story 2.4 ships are what Story 4.5 plugs the corpus INTO.
- **A new `spirit_test` cargo feature on `maos-spirit-sdk`.** Story 2.3 added `local_runner = ["std", "mock"]`; Story 2.4 adds `spirit_test = ["local_runner", "std", "mock"]`. The `spirit_test` feature is OPT-IN — production builds of Spirits do NOT pay the dependency cost. The 3 new test files (`spirit_test_smoke.rs`, `lcas_smoke.rs`, `isolation_smoke.rs`) are all gated `#[cfg(feature = "spirit_test")]`.
- **Three new discipline jobs.** `spirit-test-tests` + `lcas-corpus-tests` + `isolation-framework-tests`. Each mirrors the Story 2.3 `example-spirit-tests` + `example-spirit-drift` precedent exactly (runs-on: ubuntu-latest; rust-toolchain stable; rust-cache; one `cargo test` line). All 3 are appended to the discipline-summary `needs:` list + PR-comment table. Gate count: 30 → 33.
- **Additive coverage-matrix updates.** `FR34`, `NFR-Test-6`, `NFR-Sec-14`, `NFR-Test-3` rows gain `gates`/`corpora`/`notes` cross-references. **No `phase:` changes** (NFR-Test-6 stays at `v0.5` — that's when the full N=210 corpus + ship-gate execution lands; NFR-Sec-14 stays at `v0.8` — that's when the 200-corpus ships + executes; Story 2.4 ships PARTIAL substrate for both).
- **Minimal architecture-doc adjustments.** §4.0.2 layout: 1 line for the `spirit_test` feature on `maos-spirit-sdk` (no new workspace member — member count stays at 22). §5 Spirit ABI: ≤8-line v0.3 addendum citing the 5 spirit-test surfaces. §8 Security: ≤4-line addendum noting IsolationHookPoint framework hooks (not the corpus). `spirit-development-and-sharing.md`: top-of-file callout. **Mirrors the D10 catch-up pattern from Story 1b.6 + Story 2.1 + Story 2.3 — small, in-PR, non-rewrite.**
- **CI gate adherence: all 33 jobs green.** Particular attention: `check-service-boundary` (the new `spirit_test` module must not introduce kernel-side service boundary violations — the module lives entirely in `maos-spirit-sdk`, which is NOT a service; verify via `cargo run -p xtask -- check-service-boundary --json` showing no new violations); `check-empty-kernel` (the framework hooks are stateless types — no new persistent I9-violating state); `abi-diff` (additive `maos-spirit-sdk` surface only — abi-diff scopes to `maos-spirit-abi`, NOT `maos-spirit-sdk`; verify scope before claiming gate-green per Story 2.3 dev record); `check-unsafe` (no new unsafe — spirit_test module is `#![forbid(unsafe_code)]`); `manifest-field-coverage` (the spirit-test SDK seed's manifest self-check primitive uses ONLY existing section parsers — no new manifest fields added); `coverage-matrix` (4 row updates per AC4); `kloc-check` (the new spirit_test module + isolation module + 3 new tests + LCAS JSONL + harness — verify against `xtask/kloc.toml` budgets; the Story 2.3 dev record's pattern was to raise the budget in-story if needed; `maos-spirit-sdk` likely needs a budget raise of ~1500 LOC); `check-corpus` (the new `lcas-v0.3.jsonl` must pass MANIFEST.toml SHA-256 verification + item_count match).
- **The Epic 2 retrospective opens cleanly.** No D-items carried over to bridge stories. The Epic 2 acceptance demo (line 25) becomes fully provable at story-completion time. The dev record's "What did NOT happen this story" section explicitly enumerates the 7 deferred items (per-language SDK, judge-LLM agreement layer, full LCAS N=210, NFR-Sec-14 200-corpus, Story 4.1 runtime halt-protocol, Story 5.1 hook firing, Story 7.5b gate execution) so the retro has a clear inventory.

## What this story is NOT

- **NOT** the full spirit-test SDK with per-language assertion macros. Rust only at v0.3; TypeScript/Python/Go land at **Story 7.1** at v0.5+.
- **NOT** the judge-LLM agreement layer on LCAS. The 70-item clearly-decidable bucket is GATE-VERIFIED BY STRUCTURAL ASSERTION (gold_label matches Spirit's halt/continue decision; no judge call). Judge-LLM agreement is Story 7.1 + Story 8.x at v0.5+ once a Spirit class with halt-recall ≥0.7 exists to validate against.
- **NOT** the full LCAS N=210 corpus. 70 items at v0.3 + 70 items at Story 8.x at v0.8 (genuinely-ambiguous) + 70 items at Story 7.1 / Story 8.x at v0.5+ (adversarially-misleading). The corpus is INCREMENTAL.
- **NOT** the NFR-Sec-14 200-scenario adversarial corpus. The framework HOOKS ship now; the corpus is **Story 4.5** at v0.8 (Sec-14a n=100 + Sec-14b n=100 per ADR-040).
- **NOT** the runtime halt-protocol mechanism. The simulator ships at v0.3 with the 3 resolution kinds as forward-anchor enums; the runtime mechanism + HaltReceipt 99.9% + I14 hot-swap-halt-continuity enforcement is **Story 4.1** at v0.5.
- **NOT** runtime hook firing. Story 2.4's `LocalRunner`-wrapping `SpiritTest` harness fires hooks via the vtable dispatch path Story 2.3 shipped — NOT via a real kernel runtime. Full runtime hook firing with priority-weighted scheduling is **Story 5.1**.
- **NOT** the NFR-Onb-1 30-Min First Spirit Validation Gate execution. Story 7.5b runs the gate at v0.3 against Butler from Story 8.1. Story 2.4 ships the spirit-test SDK seed that participants USE during the gate.
- **NOT** an ABI break. `maos-spirit-abi`'s public surface is NOT touched (verified by `cargo run -p xtask -- abi-diff --json` reporting zero added/changed/removed). `ABI_VERSION` stays at `1`.
- **NOT** a new ADR. Reuses ADR-002 (Spirit form), ADR-019 + ADR-022 (halt protocol — Story 4.1 owns the mechanism), ADR-040 (Sec-14a + Sec-14b threat-model split). The spirit-test SDK pattern is implicit (parallel to how `serde` ships `serde_test` as a sibling crate; here we use a feature flag inside `maos-spirit-sdk` rather than a sibling crate because the SDK is small + the feature gating keeps production builds clean).
- **NOT** a Spirit registry publish path or signing infrastructure. The SDK seed is a TEST harness for in-development Spirits; publishing / signing are Story 7.2 + Story 7.5a at v0.5+.
- **NOT** a `maos-corpus-gen::lcas` generator. The 70-item bucket is HAND-AUTHORED at v0.3 (manual-authoring is the practical authoring mode at this size). The generator for the v0.8 expansion is Story 7.1.
- **NOT** new manifest sections or fields. The manifest self-check primitive delegates to EXISTING section parsers (`ClassSection`, `CapabilitiesRequired`, `Posture`, `OutputShape`, `Budget`, `Resources`, `Sandbox`) — no new sections introduced.
- **NOT** template-shape changes. The Story 2.3 template at `templates/spirit-rust/` is NOT modified by Story 2.4. The example crate at `examples/example-spirit/` is NOT modified — Story 2.4 adds NEW tests inside `crates/maos-spirit-sdk/tests/`, not inside `examples/example-spirit/tests/`. The Story 2.3 `xtask example-spirit-regen --check` drift detector continues to pass unmodified.
- **NOT** a CI workflow restructure. `discipline.yml`'s shape (30 → 33 jobs) is the only change. No top-level workflow file added/removed. No matrix expansion.
- **NOT** a new ABI version. The forward-anchor enum/trait shapes ship as `maos-spirit-sdk` types ONLY — they are NOT promoted to `maos-spirit-abi` until Story 4.1 (when the runtime mechanism ships). This preserves the §8.5 freeze on `maos-spirit-abi` post-1b.4.

## Acceptance Criteria

### AC1 — spirit-test SDK seed module + `spirit_test` cargo feature

**Given** the existing `crates/maos-spirit-sdk/src/lib.rs` (façade re-exports from `maos-spirit-abi` + `maos-spirit-derive`; `pub mod cancellation` for `TokioCancellationSignal` gated `#[cfg(feature = "std")]`; `pub mod local_runner` gated `#[cfg(feature = "local_runner")]`)
**And** the existing `crates/maos-spirit-sdk/Cargo.toml` `[features]` block: `default = ["std"]; std = ["dep:tokio-util"]; mock = ["maos-spirit-abi/mock"]; local_runner = ["std", "mock"]`
**And** the existing `crates/maos-spirit-sdk/src/local_runner.rs` with `LocalRunner::run(&S, &SpiritVtable<S>, &LocalRunnerFixture) -> RunReport` + `MockBus` forward-anchor types (`MockBusFrame { kind, bytes }`, `MockBusFrameKind { Send, CapInvoke }`)
**And** the design constraint: spirit-test must NOT depend on `maos-kernel-core` (verified via `cargo tree -p maos-spirit-sdk --features spirit_test --edges normal,build | grep -c maos-kernel-core` → must output `0`)
**And** the design constraint: manifest self-check must use EXISTING section parsers from `crates/maos-kernel-core/src/security/manifest.rs` (NOT re-implement parsing) — but `maos-kernel-core` is NOT a dependency of `maos-spirit-sdk`. **Resolution:** the manifest self-check primitive in `spirit_test` re-implements the section-parsing skin via `toml::from_str::<MyMinimalManifestShape>` against a small private struct that mirrors the kernel-side parsers' field set. This preserves the zero-kernel-dep constraint. **Document the duplication explicitly in the dev record's "Manifest self-check duplication" section** so a future Story 7.1 dev knows to consolidate when the SDK gains a proper manifest-types sub-crate.

**When** the dev agent adds `crates/maos-spirit-sdk/src/spirit_test/` and the supporting types

**Then** `crates/maos-spirit-sdk/Cargo.toml` gains the new feature:
```toml
[features]
default = ["std"]
std = ["dep:tokio-util"]
mock = ["maos-spirit-abi/mock"]
local_runner = ["std", "mock"]
spirit_test = ["local_runner", "std", "mock"]
```
**And** `crates/maos-spirit-sdk/Cargo.toml` `[dependencies]` gains `toml = { version = "0.8", optional = true }` and the `spirit_test` feature list grows to `spirit_test = ["local_runner", "std", "mock", "dep:toml"]` (the `toml` crate is needed for the manifest self-check primitive; it's optional + feature-gated to preserve no_std + no-spirit_test builds),
**And** `crates/maos-spirit-sdk/src/spirit_test/mod.rs` is created with this module structure:
```
crates/maos-spirit-sdk/src/spirit_test/
├── mod.rs              # pub mod harness; pub mod assert; pub mod halt; pub mod manifest; pub mod regression; pub mod isolation;
├── harness.rs          # SpiritTest<S> wrapping LocalRunner + ExtendedRunReport adding halt + manifest_self_check fields
├── assert.rs           # 5 assertion macros (declarative #[macro_export])
├── halt.rs             # HaltResolutionKind enum + HaltResolutionRecord + simulator method
├── manifest.rs         # ManifestSelfCheckReport + ManifestSelfCheckViolation + manifest_self_check fn + minimal manifest shape
├── regression.rs       # RegressionCorpus + RegressionCase + SpiritClass enum
└── isolation.rs        # IsolationHookPoint trait + CrossSpiritIsolationFixture + IsolationAttackCategory + DefaultIsolationHook
```
**And** `crates/maos-spirit-sdk/src/lib.rs` gains `#[cfg(feature = "spirit_test")] pub mod spirit_test;` APPENDED to the existing module declarations (do NOT reorder — preserve façade re-export order for `check-service-boundary` signature-hash stability per the Story 1b.5c re-export discipline),
**And** `crates/maos-spirit-sdk/src/spirit_test/mod.rs` declares:
```rust
#![forbid(unsafe_code)]

//! `spirit_test` — the spirit-test SDK seed (Story 2.4 v0.3 prerequisite).
//!
//! Wraps `LocalRunner` (Story 2.3) with: IAC frame I/O capture, halt
//! resolution simulator (3 kinds — forward-anchor for Story 4.1), manifest
//! self-check primitive, class-specific regression corpus skeleton,
//! assertion macros, and cross-Spirit isolation framework hooks.
//!
//! Per Epic 2 line 14: "spirit-test SDK seed: local runner without kernel +
//! manifest self-check + class-specific regression corpus skeleton." Full
//! per-language SDK with assertion macros + halt resolution + manifest
//! self-check + class-specific regression corpus is Story 7.1 at v0.5+.

pub mod harness;
pub mod assert;
pub mod halt;
pub mod manifest;
pub mod regression;
pub mod isolation;

pub use harness::{SpiritTest, ExtendedRunReport};
pub use halt::{HaltResolutionKind, HaltResolutionRecord};
pub use manifest::{ManifestSelfCheckReport, ManifestSelfCheckViolation, manifest_self_check};
pub use regression::{RegressionCorpus, RegressionCase, SpiritClass};
pub use isolation::{
    CrossSpiritIsolationFixture, IsolationAttackCategory, IsolationAttackCase,
    IsolationHookPoint, IsolationOutcome, DefaultIsolationHook,
    HookCallRecord, AttemptResult, ObservationResult,
};
```
**And** `crates/maos-spirit-sdk/src/spirit_test/harness.rs` declares the `SpiritTest<S>` harness that wraps `LocalRunner` + extends `RunReport`:
```rust
#![forbid(unsafe_code)]

//! `SpiritTest<S>` — wraps `LocalRunner` (Story 2.3) with halt resolution
//! + manifest self-check + frame capture support.

use crate::local_runner::{LocalRunner, LocalRunnerFixture, RunReport, MockBusFrame};
use crate::{Spirit, SpiritVtable};
use crate::spirit_test::halt::{HaltResolutionKind, HaltResolutionRecord};
use crate::spirit_test::manifest::{ManifestSelfCheckReport, ManifestSelfCheckViolation, manifest_self_check};
use std::collections::BTreeMap;

/// Extended report carrying everything `RunReport` carries plus the
/// halt resolutions the simulator recorded.
#[derive(Debug, Clone, Default)]
pub struct ExtendedRunReport {
    pub base: RunReport,
    pub halt_resolutions: Vec<HaltResolutionRecord>,
    pub captured_frames: Vec<MockBusFrame>,
}

/// The spirit-test harness. Owns a fixture, an extended report, and
/// the surfaces an author drives the Spirit through.
pub struct SpiritTest<'a, S: Spirit + 'static> {
    pub spirit: &'a S,
    pub vtable: &'a SpiritVtable<S>,
    fixture: LocalRunnerFixture,
    report: ExtendedRunReport,
}

impl<'a, S: Spirit + 'static> SpiritTest<'a, S> {
    /// Construct a new harness around a Spirit + its vtable.
    pub fn new(spirit: &'a S, vtable: &'a SpiritVtable<S>) -> Self {
        Self {
            spirit,
            vtable,
            fixture: LocalRunnerFixture::default(),
            report: ExtendedRunReport::default(),
        }
    }

    /// Mutable access to the underlying fixture so authors can add
    /// frames / telemetry events / schedule payloads / etc.
    pub fn fixture_mut(&mut self) -> &mut LocalRunnerFixture {
        &mut self.fixture
    }

    /// Simulate a halt resolution. Records the resolution in the report.
    /// At v0.3 prerequisite this does NOT yet invoke an
    /// `on_epistemic_resolve` hook (that hook ships at Story 4.1).
    pub fn resolve_halt(&mut self, halt_id: String, kind: HaltResolutionKind) {
        self.report.halt_resolutions.push(HaltResolutionRecord { halt_id, kind });
    }

    /// Run the fixture against the Spirit through the vtable. Returns
    /// an `ExtendedRunReport` carrying the base report + halt resolutions
    /// + captured frames (the last two are populated by the harness
    /// surfaces above; the base report is whatever `LocalRunner` produces).
    pub fn run(mut self) -> ExtendedRunReport {
        let base = LocalRunner::run(self.spirit, self.vtable, &self.fixture);
        self.report.base = base;
        self.report
    }

    /// Manifest self-check primitive. Returns a typed report listing
    /// parsed sections + violations + edge-case warnings.
    pub fn manifest_self_check(
        &self,
        manifest_toml_bytes: &[u8],
    ) -> Result<ManifestSelfCheckReport, ManifestSelfCheckViolation> {
        manifest_self_check(manifest_toml_bytes)
    }
}
```
**And** `crates/maos-spirit-sdk/src/spirit_test/halt.rs` declares the halt resolution simulator types (forward-anchor for Story 4.1):
```rust
#![forbid(unsafe_code)]

//! Halt resolution simulator — forward-anchor for Story 4.1.
//!
//! Architecture §6.3 + Epic 3 line 92 commit to the 3 resolution kinds:
//! `provided_context` (operator supplied additional context the Spirit
//! should consult before continuing), `accepted_halt` (operator agreed
//! with the halt; Spirit unloads), `authorized_override` (operator
//! authorized the action despite the halt; Spirit continues with an
//! override marker added to subsequent output for `output_shape`
//! predicates).
//!
//! Story 2.4 ships the ENUM SHAPE as the forward-anchor contract.
//! Story 4.1 ships the runtime mechanism (HaltResolver trait + 99.9%
//! HaltReceipt + I14 hot-swap halt-continuity enforcement).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaltResolutionKind {
    /// Operator supplied additional context — Spirit should consult
    /// the context bytes before continuing.
    ProvidedContext { context_bytes: Vec<u8> },
    /// Operator agreed with the halt — Spirit unloads.
    AcceptedHalt,
    /// Operator authorized the action despite the halt — Spirit
    /// continues with the override marker added to subsequent output.
    AuthorizedOverride { override_marker: Vec<u8> },
}

/// Record of a halt resolution the simulator surfaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaltResolutionRecord {
    pub halt_id: String,
    pub kind: HaltResolutionKind,
}
```
**And** `crates/maos-spirit-sdk/src/spirit_test/manifest.rs` declares the manifest self-check primitive (re-implements the section-parsing skin to preserve zero-kernel-dep):
```rust
#![forbid(unsafe_code)]

//! Manifest self-check primitive — parses raw TOML bytes through a
//! minimal-manifest-shape and returns a typed report listing parsed
//! sections + violations + edge-case warnings.
//!
//! **Duplication note (Story 2.4 dev record):** The kernel-side section
//! parsers at `crates/maos-kernel-core/src/security/manifest.rs` are the
//! authoritative implementations. This SDK-side re-skin is intentional
//! to preserve the zero-kernel-dep constraint (the SDK is consumed by
//! third-party Spirit crates that must not transitively pull in the
//! kernel). Future Story 7.1 may consolidate by extracting a shared
//! manifest-types sub-crate; tracked in the dev record.

use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestSelfCheckReport {
    pub class_name: String,
    pub class_version: String,
    pub forms: Vec<String>,
    pub trust_tier: String,
    pub capabilities_required_count: usize,
    pub posture_default: String,
    pub posture_allowed_max: String,
    pub output_shape_required_fields: Vec<String>,
    pub budget_context_window_size: Option<u32>,
    pub budget_time_cap_seconds: Option<u32>,
    pub resources_cpu_max_pct: Option<u32>,
    pub resources_memory_max_mb: Option<u32>,
    pub sandbox_tier: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestSelfCheckViolation {
    TomlParseError(String),
    MissingRequiredSection(&'static str),
    InvalidValue { field: &'static str, value: String, reason: &'static str },
}

#[derive(Deserialize)]
struct ManifestMinimalShape {
    class: ClassSection,
    capabilities: Option<CapabilitiesSection>,
    posture: PostureSection,
    output_shape: Option<OutputShapeSection>,
    budget: Option<BudgetSection>,
    resources: Option<ResourcesSection>,
    sandbox: SandboxSection,
}
#[derive(Deserialize)] struct ClassSection { name: String, version: String, forms: Vec<String>, trust_tier: String }
#[derive(Deserialize)] struct CapabilitiesSection { required: Option<toml::Table> }
#[derive(Deserialize)] struct PostureSection { default: String, allowed_max: String }
#[derive(Deserialize)] struct OutputShapeSection { required_fields: Vec<String> }
#[derive(Deserialize)] struct BudgetSection { context_window_size: Option<u32>, time_cap_seconds: Option<u32> }
#[derive(Deserialize)] struct ResourcesSection { cpu_max_pct: Option<u32>, memory_max_mb: Option<u32> }
#[derive(Deserialize)] struct SandboxSection { tier: String }

pub fn manifest_self_check(
    manifest_toml_bytes: &[u8],
) -> Result<ManifestSelfCheckReport, ManifestSelfCheckViolation> {
    let toml_str = std::str::from_utf8(manifest_toml_bytes)
        .map_err(|e| ManifestSelfCheckViolation::TomlParseError(format!("non-UTF-8: {e}")))?;
    let parsed: ManifestMinimalShape = toml::from_str(toml_str)
        .map_err(|e| ManifestSelfCheckViolation::TomlParseError(e.to_string()))?;

    let mut warnings = Vec::new();
    if parsed.class.forms.is_empty() {
        warnings.push("class.forms is empty — Spirit cannot be loaded in any form".to_string());
    }
    if let Some(ref os) = parsed.output_shape {
        if os.required_fields.iter().any(|f| f.contains(' ')) {
            return Err(ManifestSelfCheckViolation::InvalidValue {
                field: "output_shape.required_fields",
                value: "<contains whitespace>".to_string(),
                reason: "field names must not contain whitespace (Story 2.1 AC3)",
            });
        }
    }
    if !matches!(parsed.sandbox.tier.as_str(), "T0" | "T1" | "T2" | "T3" | "T4") {
        return Err(ManifestSelfCheckViolation::InvalidValue {
            field: "sandbox.tier",
            value: parsed.sandbox.tier.clone(),
            reason: "tier must be one of T0/T1/T2/T3/T4",
        });
    }

    Ok(ManifestSelfCheckReport {
        class_name: parsed.class.name,
        class_version: parsed.class.version,
        forms: parsed.class.forms,
        trust_tier: parsed.class.trust_tier,
        capabilities_required_count: parsed.capabilities
            .and_then(|c| c.required)
            .map(|t| t.len())
            .unwrap_or(0),
        posture_default: parsed.posture.default,
        posture_allowed_max: parsed.posture.allowed_max,
        output_shape_required_fields: parsed.output_shape.map(|o| o.required_fields).unwrap_or_default(),
        budget_context_window_size: parsed.budget.as_ref().and_then(|b| b.context_window_size),
        budget_time_cap_seconds: parsed.budget.as_ref().and_then(|b| b.time_cap_seconds),
        resources_cpu_max_pct: parsed.resources.as_ref().and_then(|r| r.cpu_max_pct),
        resources_memory_max_mb: parsed.resources.as_ref().and_then(|r| r.memory_max_mb),
        sandbox_tier: parsed.sandbox.tier,
        warnings,
    })
}
```
**And** `crates/maos-spirit-sdk/src/spirit_test/regression.rs` declares the regression corpus skeleton (typed container only):
```rust
#![forbid(unsafe_code)]

//! Class-specific regression corpus skeleton — typed container.
//!
//! Actual class corpora ship in Stories 8.1–8.5 (Butler / Researcher /
//! Founder-Loop / Mira-Nash reference Spirits). This crate ships the
//! TYPE SHAPE so reference-Spirit authors plug in without inventing
//! the schema.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiritClass {
    /// Architecture §6 Butler — anticipatory single-Spirit (on_idle).
    Anticipatory,
    /// Architecture §6 Researcher — exploratory single-Spirit + distillation.
    Exploratory,
    /// Architecture §6 Orchestrator + Worker + Architect + Reviewer wedge.
    FounderLoop,
    /// Architecture §6 Mira + Nash — diagnostic-architect bilateral pair.
    DiagnosticArchitect,
    /// Third-party Spirit class — Diego J6 onboarding persona.
    Generic,
}

#[derive(Debug, Clone)]
pub struct RegressionCase {
    pub id: String,
    pub fixture_setup: String,
    pub expected_assertions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegressionCorpus {
    pub class: SpiritClass,
    pub cases: Vec<RegressionCase>,
}

impl RegressionCorpus {
    pub fn new(class: SpiritClass) -> Self {
        Self { class, cases: Vec::new() }
    }
}
```
**And** `crates/maos-spirit-sdk/src/spirit_test/assert.rs` declares the 5 assertion macros (declarative `#[macro_export]` macros so callers can `use maos_spirit_sdk::spirit_test::*;` and invoke them as `assert_emits_frame!(report, ...)` etc.):
```rust
#![forbid(unsafe_code)]

//! Assertion macros — panic with structured diagnostics on failure.
//!
//! Each macro is gated behind `#[cfg(feature = "spirit_test")]` at the
//! module level (callers must enable the feature to access).

/// Assert that the report's captured frames contain at least one frame
/// matching the predicate.
#[macro_export]
macro_rules! assert_emits_frame {
    ($report:expr, $predicate:expr) => {{
        let matched: Vec<_> = $report.captured_frames.iter().filter(|f| $predicate(f)).collect();
        assert!(
            !matched.is_empty(),
            "assert_emits_frame!: no captured frame matched the predicate. \
             captured_frames={:?}", $report.captured_frames
        );
    }};
}

/// Assert that the report's halt_resolutions contain at least one resolution
/// matching the kind predicate.
#[macro_export]
macro_rules! assert_halts_with {
    ($report:expr, $kind_predicate:expr) => {{
        let matched: Vec<_> = $report.halt_resolutions.iter().filter(|r| $kind_predicate(&r.kind)).collect();
        assert!(
            !matched.is_empty(),
            "assert_halts_with!: no halt resolution matched the predicate. \
             halt_resolutions={:?}", $report.halt_resolutions
        );
    }};
}

/// Assert that a specific hook fired the expected number of times.
#[macro_export]
macro_rules! assert_hook_fired {
    ($report:expr, $hook_name:expr, $expected_count:expr) => {{
        let actual = $report.base.hooks_fired.get($hook_name).copied().unwrap_or(0);
        assert_eq!(
            actual, $expected_count,
            "assert_hook_fired!: hook '{}' fired {} times, expected {}",
            $hook_name, actual, $expected_count
        );
    }};
}

/// Assert that no frame was sent matching the CapInvoke scope.
#[macro_export]
macro_rules! assert_no_capability_invocation {
    ($report:expr, $scope:expr) => {{
        use $crate::local_runner::MockBusFrameKind;
        let matched: Vec<_> = $report.captured_frames.iter().filter(|f| {
            f.kind == MockBusFrameKind::CapInvoke && f.bytes.starts_with($scope.as_bytes())
        }).collect();
        assert!(
            matched.is_empty(),
            "assert_no_capability_invocation!: found {} capability invocations for scope '{}'. \
             matches={:?}", matched.len(), $scope, matched
        );
    }};
}

/// Assert that the manifest self-check report indicates a well-formed manifest.
#[macro_export]
macro_rules! assert_manifest_well_formed {
    ($self_check_report:expr) => {{
        assert!(
            $self_check_report.warnings.is_empty(),
            "assert_manifest_well_formed!: manifest has warnings: {:?}",
            $self_check_report.warnings
        );
        assert!(
            !$self_check_report.class_name.is_empty(),
            "assert_manifest_well_formed!: class.name is empty"
        );
        assert!(
            !$self_check_report.forms.is_empty(),
            "assert_manifest_well_formed!: class.forms is empty"
        );
    }};
}
```
**And** `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs` is created with this exact shape (gated `#[cfg(feature = "spirit_test")]`) covering AC1's surface end-to-end:
```rust
#![cfg(feature = "spirit_test")]

//! Smoke test for spirit_test SDK seed — exercises the harness +
//! halt resolution + manifest self-check + assertion macros.

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use maos_spirit_sdk::spirit_test::{
    SpiritTest, HaltResolutionKind, manifest_self_check, ManifestSelfCheckViolation,
};
use maos_spirit_sdk::{assert_hook_fired, assert_halts_with, assert_manifest_well_formed};

pub struct TestSpirit;

#[spirit]
impl TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {}
}

#[test]
fn harness_runs_on_idle_and_records_resolution() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.fixture_mut().invoke_on_idle = true;
    h.resolve_halt("halt-001".to_string(), HaltResolutionKind::AcceptedHalt);
    let report = h.run();
    assert_hook_fired!(report, "on_idle", 1);
    assert_halts_with!(report, |k| matches!(k, HaltResolutionKind::AcceptedHalt));
}

#[test]
fn provided_context_resolution_carries_bytes() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.resolve_halt(
        "halt-002".to_string(),
        HaltResolutionKind::ProvidedContext { context_bytes: b"clarification text".to_vec() },
    );
    let report = h.run();
    assert_eq!(report.halt_resolutions.len(), 1);
    match &report.halt_resolutions[0].kind {
        HaltResolutionKind::ProvidedContext { context_bytes } => {
            assert_eq!(context_bytes.as_slice(), b"clarification text");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn authorized_override_resolution_carries_marker() {
    let s = TestSpirit;
    let v = __maos_spirit_vtable_TestSpirit();
    let mut h = SpiritTest::new(&s, v);
    h.resolve_halt(
        "halt-003".to_string(),
        HaltResolutionKind::AuthorizedOverride { override_marker: b"OPS-OVERRIDE-42".to_vec() },
    );
    let report = h.run();
    assert_eq!(report.halt_resolutions.len(), 1);
    match &report.halt_resolutions[0].kind {
        HaltResolutionKind::AuthorizedOverride { override_marker } => {
            assert_eq!(override_marker.as_slice(), b"OPS-OVERRIDE-42");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn manifest_self_check_accepts_hello_spirit_shape() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [output_shape]
        required_fields = ["introduction"]

        [sandbox]
        tier = "T0"
    "#;
    let report = manifest_self_check(manifest).expect("should parse");
    assert_manifest_well_formed!(report);
    assert_eq!(report.class_name, "hello");
    assert_eq!(report.sandbox_tier, "T0");
}

#[test]
fn manifest_self_check_rejects_whitespace_in_required_field() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [output_shape]
        required_fields = ["with space"]

        [sandbox]
        tier = "T0"
    "#;
    let result = manifest_self_check(manifest);
    assert!(matches!(
        result,
        Err(ManifestSelfCheckViolation::InvalidValue { field: "output_shape.required_fields", .. })
    ));
}

#[test]
fn manifest_self_check_rejects_invalid_sandbox_tier() {
    let manifest = br#"
        [class]
        name = "hello"
        version = "0.1.0"
        forms = ["rust-inproc"]
        trust_tier = "local"

        [posture]
        default = "assistive"
        allowed_max = "assistive"

        [sandbox]
        tier = "T99"
    "#;
    let result = manifest_self_check(manifest);
    assert!(matches!(
        result,
        Err(ManifestSelfCheckViolation::InvalidValue { field: "sandbox.tier", .. })
    ));
}
```
**And** `cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke` PASSES (6 tests minimum: `harness_runs_on_idle_and_records_resolution`, `provided_context_resolution_carries_bytes`, `authorized_override_resolution_carries_marker`, `manifest_self_check_accepts_hello_spirit_shape`, `manifest_self_check_rejects_whitespace_in_required_field`, `manifest_self_check_rejects_invalid_sandbox_tier`),
**And** `cargo tree -p maos-spirit-sdk --features spirit_test --edges normal,build | grep -c maos-kernel-core` outputs `0` (zero kernel-core dep),
**And** `cargo build -p maos-spirit-sdk --no-default-features` continues to succeed (no_std parity preserved — spirit_test is opt-in, not default-on),
**And** `cargo build -p maos-spirit-sdk --features local_runner` continues to succeed (Story 2.3's local_runner build path is unbroken),
**And** `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` reports zero added/changed/removed against `maos-spirit-abi` (the spirit_test module lives in `maos-spirit-sdk`, outside the gate's scope per Story 2.3 dev record's note),
**And** the example-spirit smoke test at `examples/example-spirit/tests/spirit_smoke.rs` continues to pass unmodified (`cargo test -p example-spirit --locked`),
**And** the Story 2.3 `xtask example-spirit-regen --check --json` continues to exit 0 (Story 2.4 does NOT modify `templates/spirit-rust/` or `examples/example-spirit/`).

### AC2 — LCAS framework + 70-item clearly-decidable bucket at `tests/corpora/lcas-v0.3.jsonl`

**Given** the Story 2.2 corpus JSONL pattern at `tests/corpora/spirit-boundary-v0.1.jsonl` (one JSON object per line, deterministic sort by `id`, RFC 8259 strict)
**And** the existing `tests/corpora/MANIFEST.toml` schema (per Story 0.3) with `[corpus."<name>"]` blocks carrying `sha256`, `schema_version`, `item_count`, `valid_until`, `prompt_version_hash`, `description`, optional `judge_id`
**And** the Epic 2 line 22 explicit deferral: *"the remaining 140 items (genuinely-ambiguous + adversarially-misleading) are explicitly deferred to E2 + E7/E8 (require A2A scenarios from E6 to be valid)"*
**And** the PRD NFR-Test-6 ship floor: N=210 at v0.5 — Story 2.4 ships the FIRST 70 (clearly-decidable bucket)

**When** the dev agent authors `tests/corpora/lcas-v0.3.jsonl` and registers it in `tests/corpora/MANIFEST.toml`

**Then** the file `tests/corpora/lcas-v0.3.jsonl` is created with exactly 70 lines (one JSON object per line, deterministic sort by `id` lexicographic ascending). Each line carries this schema (verify by `jq -e '. | has("id") and has("class") and has("gold_label") and has("trajectory_text") and has("planted_claim") and has("expected_signals")' tests/corpora/lcas-v0.3.jsonl | grep -c true` must output `70`):
```json
{
  "id": "lcas-cd-001",
  "class": "clearly_decidable",
  "gold_label": "halt",
  "trajectory_text": "<≥4096 and ≤16384 char string>",
  "planted_claim": "<unambiguous load-bearing claim that the Spirit must surface>",
  "expected_signals": ["claim.security_vulnerability", "<other expected halt tags>"]
}
```
**And** all 70 items carry `class = "clearly_decidable"` exactly (no `genuinely_ambiguous` or `adversarially_misleading` items at v0.3 — those land at Story 8.x at v0.8 per Epic 2 line 22),
**And** the 70 items are distributed across `gold_label`: ≥30 with `gold_label = "halt"` (each carries non-empty `expected_signals` with the canonical halt tags from architecture §5.1 + §6.3 — `claim.security_vulnerability`, `claim.style_suggestion`, `claim.architecture_decision`, `scope.expansion_detected`, `test.persistent_failure`, etc.) + ≥30 with `gold_label = "continue"` (each carries `expected_signals = []` since well-behaved Spirit does NOT halt on these) + the remainder distributed proportionally,
**And** every item's `trajectory_text` is between 4096 and 16384 characters inclusive (verify by `jq -r '.trajectory_text | length' tests/corpora/lcas-v0.3.jsonl | awk 'BEGIN{ok=1} { if ($1 < 4096 || $1 > 16384) ok=0 } END{exit !ok}'`),
**And** every item's `planted_claim` field is non-empty (string length ≥ 1),
**And** every item's `expected_signals` field is an array of strings (may be empty iff `gold_label = "continue"`),
**And** the file is sorted by `id` ascending (verify by `jq -r '.id' tests/corpora/lcas-v0.3.jsonl | sort -c`),
**And** the IDs follow the pattern `lcas-cd-NNN` where `NNN` is zero-padded 3-digit ascending (`lcas-cd-001` through `lcas-cd-070`),
**And** `tests/corpora/MANIFEST.toml` gains a new entry (sorted alphabetically into the existing block):
```toml
[corpus."lcas-v0.3"]
sha256 = "<computed via cargo run -p xtask -- check-corpus --register lcas-v0.3>"
schema_version = 1
item_count = 70
valid_until = "2027-05-16"
prompt_version_hash = "<computed via xtask>"
description = "Story 2.4 LCAS (Long-context Ambiguity Stress) clearly-decidable bucket 70/210 — exercises NFR-Test-6 halt-recall + halt-precision measurement substrate. Each item carries gold_label (halt|continue), trajectory_text (4096-16384 chars), planted_claim (unambiguous load-bearing claim), expected_signals (halt tags the well-behaved Spirit emits). The remaining 140 items (genuinely-ambiguous n=70 + adversarially-misleading n=70) land at Story 8.x at v0.8 (require A2A scenarios from E6 to be valid). Full N=210 at v0.5 ship gate per PRD line 80. Gate-verified by structural assertion at v0.3 (gold_label matches Spirit's halt/continue decision); judge-LLM agreement layer is Story 7.1 + Story 8.x at v0.5+."
# judge_id omitted at v0.3 — Story 7.1 / Story 8.x adds when judge-LLM agreement layer ships
```
**And** the SHA-256 is computed by running `cargo run -p xtask -- check-corpus --register lcas-v0.3` and pasting the produced TOML snippet (mirror the Story 2.2 corpus registration pattern at lines 348 of `2-2-…md`),
**And** the companion Rust harness `crates/maos-spirit-sdk/tests/lcas_smoke.rs` is created (gated `#[cfg(feature = "spirit_test")]`):
```rust
#![cfg(feature = "spirit_test")]

//! LCAS smoke test — parses tests/corpora/lcas-v0.3.jsonl, verifies the
//! SHA-256 against MANIFEST.toml, and asserts each item is well-formed.
//!
//! Story 2.4 ships the 70-item clearly-decidable bucket. Story 8.x at
//! v0.8 ships the remaining 140 items (genuinely-ambiguous + adversarially-
//! misleading). Full N=210 at v0.5 per PRD line 80.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct LcasItem {
    id: String,
    class: String,
    gold_label: String,
    trajectory_text: String,
    planted_claim: String,
    expected_signals: Vec<String>,
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("corpora")
        .join("lcas-v0.3.jsonl")
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("corpora")
        .join("MANIFEST.toml")
}

#[test]
fn lcas_corpus_item_count_is_70() {
    let bytes = fs::read(corpus_path()).expect("read lcas-v0.3.jsonl");
    let count = bytes.split(|&b| b == b'\n').filter(|line| !line.is_empty()).count();
    assert_eq!(count, 70, "Story 2.4 LCAS clearly-decidable bucket must be exactly 70 items");
}

#[test]
fn lcas_corpus_sha256_matches_manifest() {
    let bytes = fs::read(corpus_path()).expect("read lcas-v0.3.jsonl");
    let computed = format!("{:x}", Sha256::digest(&bytes));
    let manifest = fs::read_to_string(manifest_path()).expect("read MANIFEST.toml");
    let recorded = manifest
        .lines()
        .skip_while(|l| !l.contains(r#"[corpus."lcas-v0.3"]"#))
        .find(|l| l.trim_start().starts_with("sha256"))
        .expect("MANIFEST.toml [corpus.\"lcas-v0.3\"].sha256 line");
    assert!(recorded.contains(&computed), "SHA-256 mismatch: computed={computed} recorded={recorded}");
}

#[test]
fn lcas_corpus_well_formed_schema() {
    let text = fs::read_to_string(corpus_path()).expect("read lcas-v0.3.jsonl");
    for (i, line) in text.lines().enumerate() {
        let item: LcasItem = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} parse error: {e}", i + 1));
        assert_eq!(item.class, "clearly_decidable", "v0.3 ships clearly-decidable bucket only");
        assert!(item.id.starts_with("lcas-cd-"), "id pattern: {}", item.id);
        assert!(["halt", "continue"].contains(&item.gold_label.as_str()),
                "gold_label must be halt or continue: {}", item.gold_label);
        assert!(item.trajectory_text.len() >= 4096, "trajectory too short: id={} len={}", item.id, item.trajectory_text.len());
        assert!(item.trajectory_text.len() <= 16384, "trajectory too long: id={} len={}", item.id, item.trajectory_text.len());
        assert!(!item.planted_claim.is_empty(), "planted_claim empty: {}", item.id);
        if item.gold_label == "continue" {
            assert!(item.expected_signals.is_empty(),
                    "continue items must have empty expected_signals: {}", item.id);
        } else {
            assert!(!item.expected_signals.is_empty(),
                    "halt items must have non-empty expected_signals: {}", item.id);
        }
    }
}

#[test]
fn lcas_corpus_sorted_by_id() {
    let text = fs::read_to_string(corpus_path()).expect("read lcas-v0.3.jsonl");
    let ids: Vec<String> = text.lines().filter_map(|l| {
        serde_json::from_str::<serde_json::Value>(l).ok()?.get("id")?.as_str().map(String::from)
    }).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "items must be sorted by id ascending");
}
```
**And** `crates/maos-spirit-sdk/Cargo.toml` `[dev-dependencies]` gains `sha2 = "0.10"` + `serde_json = "1"` + `serde = { version = "1", features = ["derive"] }` (verify whether already present; the kernel-side workspace likely has `sha2` already — add only if missing from this crate's dev-deps),
**And** `cargo test -p maos-spirit-sdk --features spirit_test --test lcas_smoke` PASSES (4 tests minimum: `lcas_corpus_item_count_is_70`, `lcas_corpus_sha256_matches_manifest`, `lcas_corpus_well_formed_schema`, `lcas_corpus_sorted_by_id`),
**And** `cargo run -p xtask -- check-corpus --json` exits 0 (the new corpus passes MANIFEST.toml SHA-256 verification + item_count matches),
**And** the dev agent verifies the authoring discipline: each item's `trajectory_text` is hand-authored (NOT generated by a script at v0.3 per the explicit deferral of the generator to Story 7.1); the 70 items cover a thematic spread across the PRD-referenced LCAS scenario taxonomy (security claims, architecture decisions, scope expansion, persistent test failure, style suggestions, etc. — at minimum 6 distinct halt-tag categories represented across the 30+ halt items, captured in the dev record's "LCAS authoring spread" section).

### AC3 — NFR-Sec-14 cross-Spirit memory isolation framework hooks

**Given** architecture §8.1 (8-category isolation corpus) + epic-4 line 17 (8 categories enumerated) + ADR-040 (Sec-14a same-Host + Sec-14b cross-Host split)
**And** the existing `crates/maos-spirit-sdk/src/spirit_test/` module from AC1
**And** the design constraint: framework hooks must be USABLE by a future Story 4.5 author dropping in the 200-scenario corpus (Sec-14a n=100 + Sec-14b n=100); the hooks must NOT presuppose a particular attack-vector implementation — they are observation/inspection points around Spirit-A's attempt + Spirit-B's response

**When** the dev agent adds `crates/maos-spirit-sdk/src/spirit_test/isolation.rs` and the supporting smoke test

**Then** `crates/maos-spirit-sdk/src/spirit_test/isolation.rs` declares:
```rust
#![forbid(unsafe_code)]

//! Cross-Spirit memory isolation framework hooks (NFR-Sec-14 substrate).
//!
//! Story 2.4 ships the HOOK SHAPE — the 4 hook points + 8 attack-category
//! enum + 2-Spirit fixture + DefaultIsolationHook reference impl. The
//! 200-scenario adversarial corpus (Sec-14a n=100 same-Host + Sec-14b
//! n=100 cross-Host per ADR-040) is Story 4.5 at v0.8.
//!
//! Architecture §8.1 + epic-4 line 17 enumerate the 8 categories.

use crate::{Spirit, SpiritVtable};
use crate::local_runner::{LocalRunner, LocalRunnerFixture};

/// The 8 categories per architecture §8.1 + epic-4 line 17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationAttackCategory {
    NamespaceEnumeration,
    WorkingMemoryReadAcross,
    DecisionFrameObservation,
    HaltSignalObservation,
    TransparencyLogCrossRead,
    WorkingMemoryDigestCrossRead,
    CapabilityTokenForgeryCrossSpirit,
    SandboxEscapeLateral,
}

/// A single attack case in the Story 4.5 future corpus.
#[derive(Debug, Clone)]
pub struct IsolationAttackCase {
    pub id: String,
    pub category: IsolationAttackCategory,
    /// Payload bytes Spirit-A attempts to use to read Spirit-B's state.
    pub attack_payload: Vec<u8>,
    /// What outcome the test expects (always true at framework level;
    /// Story 4.5 corpus authoring sets to false ONLY for known-vulnerable
    /// scenarios under remediation — at v0.3 prerequisite no such scenarios
    /// exist).
    pub expected_isolation_maintained: bool,
}

/// What a hook point returns — at v0.3 prerequisite all variants are
/// non-fatal recording surfaces; Story 4.5 may extend with veto power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationHookOutcome {
    Continue,
    Abort,
}

/// What Spirit-A's attempt resolved to (recorded by the framework).
#[derive(Debug, Clone)]
pub struct AttemptResult {
    pub hooks_fired_during_attempt: Vec<String>,
    pub frames_emitted: u32,
}

/// What Spirit-B's observable state revealed.
#[derive(Debug, Clone)]
pub struct ObservationResult {
    pub hooks_fired_during_observation: Vec<String>,
    pub frames_emitted: u32,
    pub leaked_bytes: Option<Vec<u8>>,
}

/// A record of one hook firing — for inspection.
#[derive(Debug, Clone)]
pub struct HookCallRecord {
    pub hook_name: &'static str,
    pub case_id: String,
    pub outcome: IsolationHookOutcome,
}

/// The 4-point hook trait — Story 4.5 plugs corpus-specific behavior
/// into these methods; DefaultIsolationHook records calls for inspection.
pub trait IsolationHookPoint {
    fn before_spirit_a_attempt(&mut self, case_id: &str) -> IsolationHookOutcome;
    fn after_spirit_a_attempt(&mut self, case_id: &str, result: &AttemptResult) -> IsolationHookOutcome;
    fn before_spirit_b_observe(&mut self, case_id: &str) -> IsolationHookOutcome;
    fn after_spirit_b_observe(&mut self, case_id: &str, observation: &ObservationResult) -> IsolationHookOutcome;
}

/// Reference impl recording all hook firings into a Vec.
#[derive(Debug, Clone, Default)]
pub struct DefaultIsolationHook {
    pub records: Vec<HookCallRecord>,
}

impl IsolationHookPoint for DefaultIsolationHook {
    fn before_spirit_a_attempt(&mut self, case_id: &str) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "before_spirit_a_attempt",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn after_spirit_a_attempt(&mut self, case_id: &str, _result: &AttemptResult) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "after_spirit_a_attempt",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn before_spirit_b_observe(&mut self, case_id: &str) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "before_spirit_b_observe",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
    fn after_spirit_b_observe(&mut self, case_id: &str, _observation: &ObservationResult) -> IsolationHookOutcome {
        self.records.push(HookCallRecord {
            hook_name: "after_spirit_b_observe",
            case_id: case_id.to_string(),
            outcome: IsolationHookOutcome::Continue,
        });
        IsolationHookOutcome::Continue
    }
}

/// Outcome of one attack case run.
#[derive(Debug, Clone)]
pub struct IsolationOutcome {
    pub case_id: String,
    pub isolation_maintained: bool,
    pub attempt_result: AttemptResult,
    pub observation_result: ObservationResult,
}

/// 2-Spirit fixture for cross-Spirit isolation testing.
pub struct CrossSpiritIsolationFixture<'a, A: Spirit + 'static, B: Spirit + 'static> {
    pub spirit_a: &'a A,
    pub vtable_a: &'a SpiritVtable<A>,
    pub spirit_b: &'a B,
    pub vtable_b: &'a SpiritVtable<B>,
}

impl<'a, A: Spirit + 'static, B: Spirit + 'static> CrossSpiritIsolationFixture<'a, A, B> {
    pub fn new(
        spirit_a: &'a A,
        vtable_a: &'a SpiritVtable<A>,
        spirit_b: &'a B,
        vtable_b: &'a SpiritVtable<B>,
    ) -> Self {
        Self { spirit_a, vtable_a, spirit_b, vtable_b }
    }

    /// Run one attack case through the 4-point hook protocol.
    pub fn run_attack_case<H: IsolationHookPoint>(
        &self,
        case: &IsolationAttackCase,
        hook: &mut H,
    ) -> IsolationOutcome {
        let _ = hook.before_spirit_a_attempt(&case.id);

        // Fire Spirit-A through on_frame with the attack payload.
        let fixture_a = LocalRunnerFixture {
            frames: vec![case.attack_payload.clone()],
            ..Default::default()
        };
        let report_a = LocalRunner::run(self.spirit_a, self.vtable_a, &fixture_a);
        let attempt = AttemptResult {
            hooks_fired_during_attempt: report_a.hooks_fired.keys().cloned().collect(),
            frames_emitted: 0,
        };
        let _ = hook.after_spirit_a_attempt(&case.id, &attempt);

        let _ = hook.before_spirit_b_observe(&case.id);

        // Fire Spirit-B through on_idle to drain observable state.
        let fixture_b = LocalRunnerFixture { invoke_on_idle: true, ..Default::default() };
        let report_b = LocalRunner::run(self.spirit_b, self.vtable_b, &fixture_b);
        let observation = ObservationResult {
            hooks_fired_during_observation: report_b.hooks_fired.keys().cloned().collect(),
            frames_emitted: 0,
            leaked_bytes: None,
        };
        let _ = hook.after_spirit_b_observe(&case.id, &observation);

        // At v0.3 prerequisite the framework always reports
        // isolation_maintained = true because the LocalRunner does not
        // share any state between Spirit-A and Spirit-B (each runs with
        // its own Ctx::mock()). Story 4.5 plugs in real leak detection.
        IsolationOutcome {
            case_id: case.id.clone(),
            isolation_maintained: true,
            attempt_result: attempt,
            observation_result: observation,
        }
    }
}
```
**And** `crates/maos-spirit-sdk/tests/isolation_smoke.rs` is created (gated `#[cfg(feature = "spirit_test")]`):
```rust
#![cfg(feature = "spirit_test")]

//! Smoke test for the NFR-Sec-14 isolation framework — constructs a
//! 2-Spirit fixture, runs a trivial attack case through DefaultIsolationHook,
//! asserts the 4 hook points fire in order + the returned outcome is
//! well-formed.

use maos_spirit_sdk::{spirit, Ctx, Spirit};
use maos_spirit_sdk::spirit_test::{
    CrossSpiritIsolationFixture, DefaultIsolationHook, IsolationAttackCase,
    IsolationAttackCategory, IsolationHookPoint,
};

pub struct SpiritA;
#[spirit]
impl SpiritA {
    fn on_frame(&self, _ctx: &mut Ctx, _payload: &maos_spirit_sdk::FramePayload) {}
}

pub struct SpiritB;
#[spirit]
impl SpiritB {
    fn on_idle(&self, _ctx: &mut Ctx) {}
}

#[test]
fn isolation_framework_fires_4_hook_points_in_order() {
    let a = SpiritA;
    let va = __maos_spirit_vtable_SpiritA();
    let b = SpiritB;
    let vb = __maos_spirit_vtable_SpiritB();
    let fixture = CrossSpiritIsolationFixture::new(&a, va, &b, vb);
    let mut hook = DefaultIsolationHook::default();
    let case = IsolationAttackCase {
        id: "iso-smoke-001".to_string(),
        category: IsolationAttackCategory::NamespaceEnumeration,
        attack_payload: b"smoke-attack".to_vec(),
        expected_isolation_maintained: true,
    };
    let outcome = fixture.run_attack_case(&case, &mut hook);
    assert_eq!(hook.records.len(), 4);
    assert_eq!(hook.records[0].hook_name, "before_spirit_a_attempt");
    assert_eq!(hook.records[1].hook_name, "after_spirit_a_attempt");
    assert_eq!(hook.records[2].hook_name, "before_spirit_b_observe");
    assert_eq!(hook.records[3].hook_name, "after_spirit_b_observe");
    assert_eq!(outcome.case_id, "iso-smoke-001");
    assert!(outcome.isolation_maintained);
    assert!(outcome.attempt_result.hooks_fired_during_attempt.iter().any(|h| h == "on_frame"));
    assert!(outcome.observation_result.hooks_fired_during_observation.iter().any(|h| h == "on_idle"));
}

#[test]
fn all_8_categories_constructible() {
    let _all = [
        IsolationAttackCategory::NamespaceEnumeration,
        IsolationAttackCategory::WorkingMemoryReadAcross,
        IsolationAttackCategory::DecisionFrameObservation,
        IsolationAttackCategory::HaltSignalObservation,
        IsolationAttackCategory::TransparencyLogCrossRead,
        IsolationAttackCategory::WorkingMemoryDigestCrossRead,
        IsolationAttackCategory::CapabilityTokenForgeryCrossSpirit,
        IsolationAttackCategory::SandboxEscapeLateral,
    ];
    assert_eq!(_all.len(), 8, "architecture §8.1 + epic-4 line 17 enumerate exactly 8 categories");
}
```
**And** `cargo test -p maos-spirit-sdk --features spirit_test --test isolation_smoke` PASSES (2 tests minimum: `isolation_framework_fires_4_hook_points_in_order`, `all_8_categories_constructible`),
**And** the `IsolationAttackCategory` enum has EXACTLY 8 variants (verify by `grep -c "^    [A-Z]" crates/maos-spirit-sdk/src/spirit_test/isolation.rs | head` — if drift to ≠8, story scope is wrong),
**And** the `IsolationHookPoint` trait has EXACTLY 4 methods (the 4 hook points around Spirit-A's attempt + Spirit-B's observation — `before_spirit_a_attempt`, `after_spirit_a_attempt`, `before_spirit_b_observe`, `after_spirit_b_observe`),
**And** the framework module imports ZERO kernel-core types (`grep -c "maos_kernel_core" crates/maos-spirit-sdk/src/spirit_test/isolation.rs` must output `0`).

### AC4 — `tests/coverage-matrix.yaml` updated additively

**Given** the existing `tests/coverage-matrix.yaml` rows post-Story-2.3:
```yaml
  FR34:
    gates: [example-spirit-tests]
    corpora: []
    phase: v0.3
    valid_until: '2027-05-12'
    notes: |
      Story 2.3 ships the local_runner SDK seed ...
  NFR-Test-6:
    gates: []
    corpora: []
    phase: v0.5
    valid_until: '2027-05-12'
  NFR-Sec-14:
    gates: []
    corpora: []
    phase: v0.8
    valid_until: '2027-05-12'
  NFR-Test-3:
    gates: []
    corpora: []
    phase: v1.0
    valid_until: '2027-05-12'
```
**And** the `xtask/gate-registry.toml` enumerating registered gate names (post-Story 2.3 the registry includes `example-spirit-tests`, `example-spirit-drift`, `example-spirit-regen`)
**And** Story 0.3's coverage-matrix-vs-gate-registry referential-integrity check (orphan gates rejected; orphan FR/NFR keys rejected)

**When** the dev agent updates `tests/coverage-matrix.yaml`

**Then** the FR34 row becomes (append 3 gates + extend notes):
```yaml
  FR34:
    gates: [example-spirit-tests, spirit-test-tests, lcas-corpus-tests, isolation-framework-tests]
    corpora: [lcas-v0.3]
    phase: v0.3
    valid_until: '2027-05-12'
    notes: |
      Story 2.3 ships the local_runner SDK seed (lifecycle hook fire via
      SpiritVtable + Ctx::mock() + in-memory mock IAC bus forward-anchor
      types) at `crates/maos-spirit-sdk/src/local_runner.rs`. Story 2.4
      extends the seed with the spirit_test SDK harness (assertion macros +
      IAC frame I/O capture + halt resolution simulator + manifest self-check +
      class-specific regression corpus skeleton + cross-Spirit isolation
      framework hooks) at `crates/maos-spirit-sdk/src/spirit_test/`. Full
      per-language SDK with assertion macros + judge-LLM agreement layer +
      registry publish path is Story 7.1 at v0.5+.
```
**And** the NFR-Test-6 row becomes (add gate + corpus, preserve phase):
```yaml
  NFR-Test-6:
    gates: [lcas-corpus-tests]
    corpora: [lcas-v0.3]
    phase: v0.5
    valid_until: '2027-05-16'
    notes: |
      Story 2.4 ships the LCAS clearly-decidable 70-item bucket at
      `tests/corpora/lcas-v0.3.jsonl`; genuinely-ambiguous (n=70) + adversarially-
      misleading (n=70) buckets land at Story 8.x at v0.8 (require A2A
      scenarios from E6 to be valid per Epic 2 line 22 + Epic 8 line 23).
      Full N=210 ship-gate execution at v0.5 per PRD line 80. Gate-verified
      by structural assertion at v0.3 (gold_label matches Spirit's halt/continue
      decision); judge-LLM agreement layer is Story 7.1 + Story 8.x at v0.5+.
```
**And** the NFR-Sec-14 row becomes (add gate, preserve phase):
```yaml
  NFR-Sec-14:
    gates: [isolation-framework-tests]
    corpora: []
    phase: v0.8
    valid_until: '2027-05-16'
    notes: |
      Story 2.4 ships the cross-Spirit memory isolation framework HOOKS
      (IsolationHookPoint 4-point trait + CrossSpiritIsolationFixture
      2-Spirit harness + 8-category IsolationAttackCategory enum +
      DefaultIsolationHook reference impl). The 200-scenario adversarial
      corpus (Sec-14a n=100 same-Host + Sec-14b n=100 cross-Host per
      ADR-040) is Story 4.5 at v0.8 — the hooks Story 2.4 ships are what
      Story 4.5 plugs the corpus INTO.
```
**And** the NFR-Test-3 row becomes (notes-only update, preserve phase + empty gates):
```yaml
  NFR-Test-3:
    gates: []
    corpora: []
    phase: v1.0
    valid_until: '2027-05-12'
    notes: |
      Story 2.4 ships the spirit-test SDK seed (5 assertion macros + IAC
      frame I/O capture + halt resolution simulator + manifest self-check +
      class-specific regression corpus skeleton + cross-Spirit isolation
      framework hooks) at `crates/maos-spirit-sdk/src/spirit_test/`. The
      SDK seed counts toward the ≥80% coverage floor measured at Story 7.5b
      (NFR-Onb-1 N=12 stratified gate) + Story 10.2 (third-party trial N=12
      at v1.5 per NFR-Test-8).
```
**And** the `valid_until` dates change ONLY on NFR-Test-6 + NFR-Sec-14 (both bumped from `'2027-05-12'` to `'2027-05-16'` — the corpus + framework lands now and the 1-year staleness clock restarts; FR34 + NFR-Test-3 stay at `'2027-05-12'` because they're notes-only updates),
**And** the existing rows for FR17, FR55, FR58, FR33, NFR-Onb-1, NFR-Test-2 (updated by Stories 2.1/2.2/2.3) are NOT modified by Story 2.4 — verify via `git diff` that ONLY the 4 above rows changed,
**And** `xtask/gate-registry.toml` gains the 3 new gates appended to the existing list (mirror the Story 2.3 pattern of appending `example-spirit-tests`, `example-spirit-drift`, `example-spirit-regen`):
```toml
gates = [
    "reproducible-build",
    "check-unsafe",
    "kloc-check",
    "abi-diff",
    "invariant-lock",
    "check-empty-kernel",
    "check-loom",
    "check-service-boundary",
    "check-corpus",
    "check-judge-config",
    "coverage-matrix",
    "corpus-staleness",
    "rebaseline-check",
    "calibrate",
    "check-security-md",
    "manifest-field-coverage",
    "example-spirit-regen",
    "example-spirit-tests",
    "example-spirit-drift",
    "spirit-test-tests",
    "lcas-corpus-tests",
    "isolation-framework-tests",
]
```
**And** `cargo run -p xtask -- coverage-matrix --json` exits 0 (the new gate names — `spirit-test-tests`, `lcas-corpus-tests`, `isolation-framework-tests` — are present in `xtask/gate-registry.toml`; if missing, the coverage-matrix gate fails with "orphan gate"; the dev agent registers all 3 gates first, then updates the matrix),
**And** `cargo run -p xtask -- check-corpus --json` exits 0 (the new corpus `lcas-v0.3` is registered in `tests/corpora/MANIFEST.toml` per AC2; SHA-256 + item_count match; no orphan files; the existing 4 corpora — `calibration-seed-v0.1`, `secret-redaction-1e4`, `red-team-640`, `spirit-boundary-v0.1` — stay unchanged).

### AC5 — Three new `discipline.yml` jobs + summary table updates

**Given** the existing `.github/workflows/discipline.yml` 30-job gate set (post-Story 2.3 added `example-spirit-tests` + `example-spirit-drift`)
**And** the existing `example-spirit-tests` job shape (the precedent: `runs-on: ubuntu-latest`, `actions/checkout@v4`, `dtolnay/rust-toolchain@v1` with `stable`, `Swatinem/rust-cache@v2`, then `cargo test -p example-spirit --locked`)
**And** the existing discipline-summary `needs:` list (post-Story 2.3 ends with `, example-spirit-tests, example-spirit-drift]`)
**And** the existing PR-comment table builder at lines 540-640 (assigns each gate's result to a short variable name, then renders a markdown table)

**When** the dev agent extends `.github/workflows/discipline.yml`

**Then** three new jobs are added (insert immediately after `example-spirit-drift` for ordering parity — preserve grep-ability):
```yaml
  spirit-test-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}
      - name: Run spirit_test SDK seed smoke tests (Story 2.4 v0.3 prerequisite)
        run: cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke --locked

  lcas-corpus-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}
      - name: Run LCAS corpus smoke tests (Story 2.4 NFR-Test-6 substrate)
        run: cargo test -p maos-spirit-sdk --features spirit_test --test lcas_smoke --locked

  isolation-framework-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ hashFiles('**/Cargo.lock') }}
      - name: Run cross-Spirit isolation framework smoke tests (Story 2.4 NFR-Sec-14 substrate)
        run: cargo test -p maos-spirit-sdk --features spirit_test --test isolation_smoke --locked
```
**And** the discipline-summary `needs:` list is appended with all 3 job names (preserve existing order; new entries at the end):
```yaml
    needs: [reproducible-build, check-unsafe, check-empty-kernel, check-loom, check-service-boundary, kloc-check, abi-diff, invariant-lock, check-corpus, check-judge-config, check-security-md, audit-spine-smoke, cap-token-verify-bench, cap-registry-smoke, fr4-1000-call-fixture, audit-query-fr4-smoke, maosctl-smoke, manifest-field-coverage, v01-evaluator-path, hello-spirit-tests, hello-spirit-bench, onb-nfr2-timing, coverage-matrix, corpus-staleness, calibrate-per-commit, determinism-tests, check-fr47, example-spirit-tests, example-spirit-drift, spirit-test-tests, lcas-corpus-tests, isolation-framework-tests]
```
**And** the PR-comment table builder (lines 540-640) is updated to add three new rows (mirror the Story 2.3 `est`/`esd` precedent):
- New lines at the variable-assignment block:
  - `echo "stt=${{ needs.spirit-test-tests.result }}" >> $GITHUB_OUTPUT`
  - `echo "lct=${{ needs.lcas-corpus-tests.result }}" >> $GITHUB_OUTPUT`
  - `echo "ift=${{ needs.isolation-framework-tests.result }}" >> $GITHUB_OUTPUT`
- New lines at the JS-template variable extraction:
  - `const stt = '${{ needs.spirit-test-tests.result }}';`
  - `const lct = '${{ needs.lcas-corpus-tests.result }}';`
  - `const ift = '${{ needs.isolation-framework-tests.result }}';`
- Three new table rows in the markdown template (after the `| example-spirit-drift | ${icon(esd)} ${esd} |` row):
  - `| spirit-test-tests | ${icon(stt)} ${stt} |`
  - `| lcas-corpus-tests | ${icon(lct)} ${lct} |`
  - `| isolation-framework-tests | ${icon(ift)} ${ift} |`
**And** the total `needs:` list count is verified: 30 existing → 33 jobs total (the count is implicit in the `needs:` list length; dev record cites the explicit count),
**And** the dev agent runs `act -j spirit-test-tests` + `act -j lcas-corpus-tests` + `act -j isolation-framework-tests` locally if `act` is available — OR if `act` is unavailable, runs the underlying bash commands directly (`cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke --locked` + `--test lcas_smoke --locked` + `--test isolation_smoke --locked`) and asserts all 3 pass cold,
**And** the YAML is well-formed: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/discipline.yml'))"` exits 0,
**And** the discipline summary's "PASSED / FAILED" semantics are preserved: any `result == 'failure'` in the new jobs blocks the summary green, same shape as existing.

### AC6 — Architecture-doc adjustments + `spirit-development-and-sharing.md` callout

**Given** the existing `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 layout block (post Story 2.3, listing 20 lib/bin crates + xtask + `examples/example-spirit` = 22 workspace members; mentioning `templates/spirit-rust/` excluded via `[workspace] exclude = ["templates"]`)
**And** the existing `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5 v0.3 prerequisite addendum (Story 2.3 added at line 210)
**And** the existing `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` §8.1 isolation corpus section (line 13 + lines 45-47)
**And** the existing `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` top-of-file Story 2.3 callout (line 23)
**And** the D10 catch-up pattern from Story 1b.6 + Story 2.1 + Story 2.3 (minimal in-PR doc updates, NOT rewrites)

**When** the dev agent finalizes Story 2.4

**Then** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 layout addendum gains a one-line addition appended to the existing Story 2.3 paragraph (do NOT alter the existing 22-member count — Story 2.4 ships NO new workspace member; the `spirit_test` feature is a module inside the existing `crates/maos-spirit-sdk` crate):
> **`spirit_test` feature on `maos-spirit-sdk` (post Story 2.4):** The crate gains an opt-in `spirit_test` cargo feature (depends on `local_runner` + `std` + `mock`) gating a new `crates/maos-spirit-sdk/src/spirit_test/` module that ships the SDK seed (assertion macros + IAC frame I/O capture + halt resolution simulator + manifest self-check + class-specific regression corpus skeleton + cross-Spirit isolation framework hooks). Workspace member count stays at **22** — the new module is feature-gated inside the existing crate, not a new workspace member.

**And** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` gains a ≤8-line addendum at the end of §5 (after the existing Story 2.3 addendum at line 210; do NOT alter the existing addendum):
> **v0.3 prerequisite — spirit-test SDK seed (Story 2.4):** Spirit authors at v0.3 prerequisite gain the `spirit_test` cargo feature on `maos-spirit-sdk` exposing `SpiritTest<S>` (wraps `LocalRunner` with halt resolution + manifest self-check + frame capture), 5 assertion macros (`assert_emits_frame!`, `assert_halts_with!`, `assert_hook_fired!`, `assert_no_capability_invocation!`, `assert_manifest_well_formed!`), the 3-kind halt resolution simulator (forward-anchor for Story 4.1 — `ProvidedContext`, `AcceptedHalt`, `AuthorizedOverride`), and the cross-Spirit memory isolation framework hooks (`IsolationHookPoint` 4-point trait + `CrossSpiritIsolationFixture` 2-Spirit harness + 8-category `IsolationAttackCategory` enum per §8.1). The LCAS clearly-decidable 70-item bucket ships at `tests/corpora/lcas-v0.3.jsonl`. Full per-language SDK with judge-LLM agreement layer + registry publish path lands at Story 7.1 at v0.5+; the NFR-Sec-14 200-scenario adversarial corpus (Sec-14a + Sec-14b) lands at Story 4.5 at v0.8.

**And** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` gains a ≤4-line addendum to §8.1 (insert immediately after the existing "Cross-Spirit memory isolation corpus" paragraph at line 47):
> **v0.3 framework hooks (Story 2.4):** The `maos-spirit-sdk` crate's `spirit_test` feature ships the cross-Spirit isolation framework HOOKS — `IsolationHookPoint` 4-point trait (`before_spirit_a_attempt` / `after_spirit_a_attempt` / `before_spirit_b_observe` / `after_spirit_b_observe`), `CrossSpiritIsolationFixture` 2-Spirit harness, 8-category `IsolationAttackCategory` enum matching the 8 categories above. The 200-scenario corpus authoring + execution (Sec-14a n=100 + Sec-14b n=100 per ADR-040) is Story 4.5 at v0.8; the framework hooks are the substrate Story 4.5 plugs the corpus INTO.

**And** `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` gains a top-of-file callout (insert immediately after the existing Story 2.3 callout at line 23):
> **🛠️ v0.3 prerequisite shipped — Story 2.4 (2026-05-16 + 1 day).** The spirit-test SDK seed cited throughout this document (§13 — "Diego runs `spirit-test` on a corpus of 50 known-buggy code examples") lands at v0.3 PREREQUISITE. SDK seed: `maos_spirit_sdk::spirit_test::*` (gated behind `spirit_test` cargo feature on `maos-spirit-sdk`). Surfaces: `SpiritTest<S>` harness (wraps `LocalRunner`), 5 assertion macros (`assert_emits_frame!` / `assert_halts_with!` / `assert_hook_fired!` / `assert_no_capability_invocation!` / `assert_manifest_well_formed!`), 3-kind halt resolution simulator, manifest self-check primitive, cross-Spirit memory isolation framework hooks. LCAS clearly-decidable 70-bucket: `tests/corpora/lcas-v0.3.jsonl`. Full per-language SDK (TS/Python/Go) + judge-LLM agreement layer + registry publish path lands at Story 7.1 + Story 7.2 at v0.5+; NFR-Sec-14 200-scenario adversarial corpus at Story 4.5 at v0.8.

**And** the architecture-doc updates land in the SAME PR as the code (mirror the D10 pattern from Story 1b.6 + Story 2.1 + Story 2.3 — do NOT defer the doc update; doc accretion is the failure mode that prompted D10 originally),
**And** the dev agent verifies no other architecture file is broken by the doc update (`grep -rn "22 workspace members\|spirit_test feature\|IsolationHookPoint\|LCAS clearly-decidable" _bmad-output/planning-artifacts/architecture-maos-minimal-opus/` after the edits should show ONLY the 3 files updated above; no other references should exist anywhere — if they do, they're stale and need a cross-reference update).

### AC7 — Discipline-suite sweep + cold-cache integration + dev-record gates citation

**Given** the full 33-job discipline-suite (30 existing post-Story 2.3 + 3 new per AC5)
**And** the A6 + A7 + A8 retro actions from `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md` lines 182-199
**And** Story 2.1's + Story 2.2's + Story 2.3's dev-record `Gates Status` precedent

**When** the dev agent finishes Story 2.4 implementation

**Then** every existing gate continues to pass against the real v0.1-β workspace:
- `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` — 0 added/changed/removed against `maos-spirit-abi` (spirit_test lives in `maos-spirit-sdk`, outside the gate's scope per Story 2.3 precedent)
- `cargo run -p xtask -- check-empty-kernel --json` — exit 0 (the spirit_test module is stateless; no new persistent I9-violating state; the `RegressionCorpus` + `IsolationHookPoint` types are containers that DO NOT own kernel state — they live in the SDK crate)
- `cargo run -p xtask -- check-service-boundary --json` — exit 0 (P1 + P2 + P3 + P4 + Spirit ABI reflection stay green from Story 2.2/2.3; the new spirit_test module does NOT add a new service — it's a module inside `maos-spirit-sdk` which is NOT a service)
- `cargo run -p xtask -- check-unsafe --json` — exit 0 (spirit_test/mod.rs + harness.rs + assert.rs + halt.rs + manifest.rs + regression.rs + isolation.rs all declare `#![forbid(unsafe_code)]`)
- `cargo run -p xtask -- kloc-check --json` — exit 0 (verify `xtask/kloc.toml` budgets accommodate the new ~600 LOC for spirit_test module + ~70 LOC for assertion macros + ~400 LOC for isolation module + ~200 LOC for manifest self-check + ~150 LOC for harness + halt + regression + ~80 LOC for 3 test files + the LCAS JSONL is not Rust code; if `maos-spirit-sdk` budget breaks, raise it in-story per Story 2.2/2.3 precedent)
- `cargo run -p xtask -- invariant-lock --json` — exit 0 (Story 2.4 does NOT touch any invariant; gate reports "no invariant-touching diffs")
- `cargo run -p xtask -- check-corpus --json` — exit 0 (new corpus `lcas-v0.3` registered in MANIFEST.toml per AC2; SHA-256 matches; item_count = 70; existing 4 corpora unchanged)
- `cargo run -p xtask -- check-judge-config --json` — exit 0
- `cargo run -p xtask -- check-security-md --json` — exit 0
- `cargo run -p xtask -- check-fr47 --json` — exit 0
- `cargo run -p xtask -- check-loom --json` — exit 0
- `cargo run -p xtask -- coverage-matrix --json` — exit 0 (4 row updates per AC4; 3 new gate names registered per AC4)
- `cargo run -p xtask -- corpus-staleness --json` — exit 0 (the new corpus's `valid_until = '2027-05-16'` is non-stale; existing 4 corpora's valid_until dates stay accurate)
- `cargo run -p xtask -- manifest-field-coverage --json` — exit 0 (Story 2.4 introduces NO new manifest fields — the spirit_test manifest self-check primitive uses ONLY existing section parsers' field set)
- `cargo run -p xtask -- example-spirit-regen --check --json` — exit 0 (Story 2.4 does NOT modify `templates/spirit-rust/` or `examples/example-spirit/`)
- `cargo run -p xtask -- spirit-test-tests` (NEW; AC5 gate — implemented as `cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke --locked`) — exit 0
- `cargo run -p xtask -- lcas-corpus-tests` (NEW; AC5 gate — implemented as `cargo test -p maos-spirit-sdk --features spirit_test --test lcas_smoke --locked`) — exit 0
- `cargo run -p xtask -- isolation-framework-tests` (NEW; AC5 gate — implemented as `cargo test -p maos-spirit-sdk --features spirit_test --test isolation_smoke --locked`) — exit 0
**And** the new `spirit-test-tests`, `lcas-corpus-tests`, `isolation-framework-tests` jobs all pass cold (`cargo clean -p maos-spirit-sdk && cargo test -p maos-spirit-sdk --features spirit_test --locked`),
**And** the existing `tests/integration/v01_evaluator_path.sh` passes cold (per A6 — `cargo clean -p maos-bin && cargo clean -p maos-spirit-hello && ./tests/integration/v01_evaluator_path.sh`),
**And** the existing `tests/integration/onb_nfr2_timing.sh` passes (NFR-Onb-2 5-minute evaluator path remains green — Story 2.4 does NOT alter hello-spirit behavior or the one-shot evaluator path),
**And** the existing `cargo test -p example-spirit --locked` continues to pass (Story 2.3's example crate is unmodified),
**And** the dev agent runs the full local discipline sweep ONE final time chained: `cargo run -p xtask -- abi-diff check-empty-kernel check-service-boundary check-unsafe kloc-check invariant-lock check-corpus coverage-matrix manifest-field-coverage example-spirit-regen --check` (chaining may require xtask multi-command invocation — if unsupported, run sequentially via `&&` in a single shell command and capture each exit code in the dev record),
**And** the dev record's `Gates Status` section cites the SPECIFIC `discipline.yml` run on the PR commit (per A8: `discipline.yml run <run_id>, conclusion: success` — NOT proxied by `journal-append.yml`),
**And** the dev record's `What did NOT happen this story` section (per A4) grep-verifies anti-claims for: NO new ADR (verify `docs/adr/index.md` unchanged), NO `maos-spirit-abi` public-API change (verify `cargo public-api -p maos-spirit-abi` against the existing baseline shows zero diff), NO new content-addressed corpus other than `lcas-v0.3` (verify `tests/corpora/MANIFEST.toml` shows ONLY the 1 new `[corpus."lcas-v0.3"]` block; existing 4 blocks unchanged), NO modification to `templates/spirit-rust/` or `examples/example-spirit/`, NO modification to `crates/maos-spirit-hello/`, NO new Spirit registry publish path, NO judge-LLM agreement layer on LCAS (structural-assertion only at v0.3), NO 200-scenario NFR-Sec-14 corpus (framework hooks only), NO per-language SDK (Rust only), NO runtime halt-protocol mechanism (simulator only), NO runtime hook firing (vtable dispatch only), NO NFR-Onb-1 30-min gate execution (substrate only), NO `epistemic_resolve` hook on the Spirit trait (Story 4.1 ships this), NO `output_shape` runtime enforcement (Story 7.3 ships this), NO `on_swap_out` / `snapshot` / `migrate` hooks (Story 5.2 ships these), NO cargo-public-api break (verified by abi-diff against `abi-baseline/v1-pre-bump.txt`), NO new persistent kernel state (verified by `check-empty-kernel`), NO maos-kernel-core dep transitively pulled by SDK (`cargo tree -p maos-spirit-sdk --features spirit_test --edges normal,build | grep -c maos-kernel-core` returns 0),
**And** the dev agent's self-review checklist at the end of the dev record contains ≥24 items (per Epic 1a/1b/2.1/2.2/2.3 retro discipline) covering: AC1 spirit_test SDK seed module + 6 smoke tests pass + zero kernel dep verified; AC2 LCAS 70-item bucket authored + SHA-256 registered + 4 smoke tests pass; AC3 IsolationHookPoint 4-method trait + 8-variant enum + 2 smoke tests pass + zero kernel dep verified; AC4 coverage-matrix 4 row updates + gate-registry 3 new gates registered; AC5 `discipline.yml` 33-job count verified + summary `needs:` list + PR-comment table updated; AC6 architecture-doc 4 files updated (4-kernel-design.md + 5-spirit-abi.md + 8-security-approval-model.md + spirit-development-and-sharing.md); AC7 cold-cache integration scripts pass; ABI stability (zero diff against `maos-spirit-abi`); no new `unsafe`; no new ADR; member-count consistency (stays at 22); `examples/example-spirit/` + `templates/spirit-rust/` unchanged; hello-spirit one-shot path produces identical 4-key JSON (regression preserved); `tests/integration/v01_evaluator_path.sh` cold-cache green; `tests/integration/onb_nfr2_timing.sh` green; specific `discipline.yml` run-id cited; A4/A6/A7/A8 retro actions all confirmed; the 6 smoke + 4 LCAS + 2 isolation = 12 new tests all pass; the LCAS 70-item authoring spread captured (≥6 distinct halt-tag categories represented); the 8 IsolationAttackCategory variants are EXACTLY the 8 from architecture §8.1 (verify by counting the enum body); the 3 HaltResolutionKind variants are EXACTLY the 3 from architecture §6.3 + epic-3 line 92 (`ProvidedContext`, `AcceptedHalt`, `AuthorizedOverride`).

## Tasks / Subtasks

Each top-level task carries `(AC: #)` mapping. Substeps preserve order. **Self-review checklist at end is mandatory** before opening PR (per Epic 1a/1b/2.1/2.2/2.3 retro actions A1/A2/A4/A5/A6/A7/A8).

- [ ] **Task 1 — Add `spirit_test` cargo feature + module skeleton to `maos-spirit-sdk`** (AC: 1)
  - [ ] 1.1 Read `crates/maos-spirit-sdk/Cargo.toml` to confirm current shape (post-Story 2.3 the `local_runner` feature exists).
  - [ ] 1.2 Edit `crates/maos-spirit-sdk/Cargo.toml`: add `toml = { version = "0.8", optional = true }` to `[dependencies]`; add `spirit_test = ["local_runner", "std", "mock", "dep:toml"]` to `[features]`; add `sha2 = "0.10"` + `serde_json = "1"` + `serde = { version = "1", features = ["derive"] }` to `[dev-dependencies]` (verify each is not already present; if `serde_json` is, that's fine — keep one entry).
  - [ ] 1.3 Create `crates/maos-spirit-sdk/src/spirit_test/mod.rs` with the exact module structure + `pub use` re-exports from AC1.
  - [ ] 1.4 Edit `crates/maos-spirit-sdk/src/lib.rs`: APPEND `#[cfg(feature = "spirit_test")] pub mod spirit_test;` AFTER the existing `#[cfg(feature = "local_runner")] pub mod local_runner;` line (preserve order; do NOT reorder existing pub mod declarations).
  - [ ] 1.5 Run `cargo build -p maos-spirit-sdk --no-default-features` — succeeds (no_std parity preserved).
  - [ ] 1.6 Run `cargo build -p maos-spirit-sdk` (default features) — succeeds.
  - [ ] 1.7 Run `cargo build -p maos-spirit-sdk --features local_runner` — succeeds (Story 2.3 build path unbroken).
  - [ ] 1.8 Run `cargo build -p maos-spirit-sdk --features spirit_test` — succeeds (NEW build path; module skeleton compiles).

- [ ] **Task 2 — Implement harness + halt + manifest + regression + assert modules** (AC: 1)
  - [ ] 2.1 Create `crates/maos-spirit-sdk/src/spirit_test/harness.rs` with the exact `SpiritTest<S>` + `ExtendedRunReport` shape from AC1.
  - [ ] 2.2 Create `crates/maos-spirit-sdk/src/spirit_test/halt.rs` with the exact `HaltResolutionKind` enum (3 variants: `ProvidedContext { context_bytes: Vec<u8> }`, `AcceptedHalt`, `AuthorizedOverride { override_marker: Vec<u8> }`) + `HaltResolutionRecord` shape from AC1. Verify the 3 variants match architecture §6.3 + epic-3 line 92 wording.
  - [ ] 2.3 Create `crates/maos-spirit-sdk/src/spirit_test/manifest.rs` with the exact `ManifestSelfCheckReport` + `ManifestSelfCheckViolation` + `manifest_self_check` fn + minimal manifest shape from AC1. Document the "duplication note" prominently in the module-level doc comment per AC1.
  - [ ] 2.4 Create `crates/maos-spirit-sdk/src/spirit_test/regression.rs` with the exact `SpiritClass` enum (5 variants: `Anticipatory`, `Exploratory`, `FounderLoop`, `DiagnosticArchitect`, `Generic`) + `RegressionCase` + `RegressionCorpus` shape from AC1.
  - [ ] 2.5 Create `crates/maos-spirit-sdk/src/spirit_test/assert.rs` with the exact 5 macros (`assert_emits_frame!`, `assert_halts_with!`, `assert_hook_fired!`, `assert_no_capability_invocation!`, `assert_manifest_well_formed!`) using `#[macro_export]` per AC1.
  - [ ] 2.6 Run `cargo build -p maos-spirit-sdk --features spirit_test --locked` — all 5 new files compile.
  - [ ] 2.7 Run `cargo run -p xtask -- check-unsafe --json` — exit 0 (all 7 new files declare `#![forbid(unsafe_code)]`).

- [ ] **Task 3 — Author the spirit_test smoke test at `tests/spirit_test_smoke.rs`** (AC: 1)
  - [ ] 3.1 Create `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs` with the exact 6-test shape from AC1.
  - [ ] 3.2 Run `cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke --locked` — 6 tests pass.
  - [ ] 3.3 Run `cargo tree -p maos-spirit-sdk --features spirit_test --edges normal,build | grep -c maos-kernel-core` — must output `0`. If non-zero, investigate which crate transitively pulls kernel-core and prune (likely candidate: an accidental SDK-side import of a kernel-side type).
  - [ ] 3.4 Verify the 3 halt resolution kinds are constructible per the 3-variant enum: write a quick manual smoke `cargo test -p maos-spirit-sdk --features spirit_test --test spirit_test_smoke -- --nocapture` and grep for "ProvidedContext", "AcceptedHalt", "AuthorizedOverride" in the test source.

- [ ] **Task 4 — Implement the isolation framework module + smoke test** (AC: 3)
  - [ ] 4.1 Create `crates/maos-spirit-sdk/src/spirit_test/isolation.rs` with the exact 8-variant `IsolationAttackCategory` + 4-method `IsolationHookPoint` trait + `CrossSpiritIsolationFixture` + `DefaultIsolationHook` + `IsolationOutcome` + `IsolationAttackCase` + `AttemptResult` + `ObservationResult` + `HookCallRecord` shape from AC3.
  - [ ] 4.2 Verify the 8-variant enum exactly matches architecture §8.1 line 13 + epic-4 line 17: `NamespaceEnumeration`, `WorkingMemoryReadAcross`, `DecisionFrameObservation`, `HaltSignalObservation`, `TransparencyLogCrossRead`, `WorkingMemoryDigestCrossRead`, `CapabilityTokenForgeryCrossSpirit`, `SandboxEscapeLateral`.
  - [ ] 4.3 Verify the 4-method `IsolationHookPoint` trait shape: `before_spirit_a_attempt`, `after_spirit_a_attempt`, `before_spirit_b_observe`, `after_spirit_b_observe`.
  - [ ] 4.4 Create `crates/maos-spirit-sdk/tests/isolation_smoke.rs` with the exact 2-test shape from AC3.
  - [ ] 4.5 Run `cargo test -p maos-spirit-sdk --features spirit_test --test isolation_smoke --locked` — 2 tests pass.
  - [ ] 4.6 Verify `grep -c "maos_kernel_core" crates/maos-spirit-sdk/src/spirit_test/isolation.rs` outputs `0`.

- [ ] **Task 5 — Hand-author the LCAS 70-item clearly-decidable corpus** (AC: 2)
  - [ ] 5.1 Create `tests/corpora/lcas-v0.3.jsonl` with EXACTLY 70 items, one JSON object per line, sorted by `id` ascending (`lcas-cd-001` through `lcas-cd-070`).
  - [ ] 5.2 Each item's schema: `{"id": "lcas-cd-NNN", "class": "clearly_decidable", "gold_label": "halt"|"continue", "trajectory_text": "<≥4096 ≤16384 chars>", "planted_claim": "<unambiguous>", "expected_signals": [...]}`.
  - [ ] 5.3 Distribute the 70 items: ≥30 with `gold_label = "halt"` (each with non-empty `expected_signals`) + ≥30 with `gold_label = "continue"` (each with `expected_signals = []`) + remainder distributed proportionally.
  - [ ] 5.4 Cover ≥6 distinct halt-tag categories across the halt items: `claim.security_vulnerability`, `claim.style_suggestion`, `claim.architecture_decision`, `scope.expansion_detected`, `test.persistent_failure`, `story.acceptance_criterion.ambiguous` (these are the architecture §5.1 + epic-8 line 31 canonical halt tag set; document the spread in the dev record's "LCAS authoring spread" section).
  - [ ] 5.5 Verify well-formedness with `jq -e '. | has("id") and has("class") and has("gold_label") and has("trajectory_text") and has("planted_claim") and has("expected_signals")' tests/corpora/lcas-v0.3.jsonl | grep -c true` outputs `70`.
  - [ ] 5.6 Verify sort: `jq -r '.id' tests/corpora/lcas-v0.3.jsonl | sort -c` exits 0.
  - [ ] 5.7 Verify trajectory length distribution: `jq -r '.trajectory_text | length' tests/corpora/lcas-v0.3.jsonl | awk 'BEGIN{ok=1} { if ($1 < 4096 || $1 > 16384) ok=0 } END{exit !ok}'` exits 0.
  - [ ] 5.8 Compute SHA-256 via `cargo run -p xtask -- check-corpus --register lcas-v0.3` and paste the produced TOML snippet into `tests/corpora/MANIFEST.toml` (sorted alphabetically: `lcas-v0.3` slots after `calibration-seed-v0.1` and before `red-team-640`).
  - [ ] 5.9 Verify `cargo run -p xtask -- check-corpus --json` exits 0.

- [ ] **Task 6 — Author the LCAS smoke test at `tests/lcas_smoke.rs`** (AC: 2)
  - [ ] 6.1 Create `crates/maos-spirit-sdk/tests/lcas_smoke.rs` with the exact 4-test shape from AC2.
  - [ ] 6.2 Run `cargo test -p maos-spirit-sdk --features spirit_test --test lcas_smoke --locked` — 4 tests pass.
  - [ ] 6.3 Verify the SHA-256 reconciliation test passes by computing locally: `sha256sum tests/corpora/lcas-v0.3.jsonl` matches the value in MANIFEST.toml.

- [ ] **Task 7 — Extend `.github/workflows/discipline.yml` with 3 new jobs** (AC: 5)
  - [ ] 7.1 Read `.github/workflows/discipline.yml` to locate the `example-spirit-tests` + `example-spirit-drift` jobs (post-Story 2.3) and the discipline-summary `needs:` list + PR-comment table builder.
  - [ ] 7.2 Insert `spirit-test-tests` job immediately after `example-spirit-drift`. Use the exact YAML shape from AC5.
  - [ ] 7.3 Insert `lcas-corpus-tests` job immediately after `spirit-test-tests`. Use the exact YAML shape from AC5.
  - [ ] 7.4 Insert `isolation-framework-tests` job immediately after `lcas-corpus-tests`. Use the exact YAML shape from AC5.
  - [ ] 7.5 Append `spirit-test-tests, lcas-corpus-tests, isolation-framework-tests` to the discipline-summary `needs:` list (preserve existing comma-separated format).
  - [ ] 7.6 Update the variable-assignment block: add `echo "stt=..."`, `echo "lct=..."`, `echo "ift=..."` per AC5.
  - [ ] 7.7 Update the JS-template variable extraction: add `const stt = ...;`, `const lct = ...;`, `const ift = ...;` per AC5.
  - [ ] 7.8 Update the markdown table template: add 3 new rows per AC5.
  - [ ] 7.9 Verify YAML well-formedness: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/discipline.yml'))"`.
  - [ ] 7.10 If `act` is available locally, run all 3 new jobs: `act -j spirit-test-tests` + `act -j lcas-corpus-tests` + `act -j isolation-framework-tests`. If `act` is unavailable, run the underlying bash commands directly and assert all 3 pass cold.

- [ ] **Task 8 — Coverage matrix + gate registry updates** (AC: 4)
  - [ ] 8.1 Read `xtask/gate-registry.toml` to confirm the current shape (post-Story 2.3 ends with `example-spirit-tests`, `example-spirit-drift`).
  - [ ] 8.2 Append `spirit-test-tests`, `lcas-corpus-tests`, `isolation-framework-tests` to the `gates = [...]` list per AC4.
  - [ ] 8.3 Read `tests/coverage-matrix.yaml` to confirm the current shape of FR34, NFR-Test-6, NFR-Sec-14, NFR-Test-3 rows.
  - [ ] 8.4 Update FR34 row per AC4 (append 3 gates + extend notes; preserve `phase: v0.3` + `valid_until: '2027-05-12'`).
  - [ ] 8.5 Update NFR-Test-6 row per AC4 (add gate + corpus + bump `valid_until` to `'2027-05-16'`; preserve `phase: v0.5`).
  - [ ] 8.6 Update NFR-Sec-14 row per AC4 (add gate + bump `valid_until` to `'2027-05-16'`; preserve `phase: v0.8`).
  - [ ] 8.7 Update NFR-Test-3 row per AC4 (notes-only update; preserve `phase: v1.0` + `valid_until: '2027-05-12'` + empty gates).
  - [ ] 8.8 Verify NO other rows changed: `git diff tests/coverage-matrix.yaml` should show changes ONLY to these 4 rows + the `notes` field of FR34 (which Story 2.3 set; Story 2.4 extends).
  - [ ] 8.9 Run `cargo run -p xtask -- coverage-matrix --json` — exit 0. If "orphan gate" errors fire, Task 8.2 didn't register the new gates correctly; fix before proceeding.

- [ ] **Task 9 — Architecture-doc adjustments** (AC: 6)
  - [ ] 9.1 Read `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2 to understand the current layout addendum (Story 2.3 set member count to 22).
  - [ ] 9.2 Append the one-paragraph spirit_test feature addendum per AC6 (do NOT alter the existing Story 2.3 22-member paragraph).
  - [ ] 9.3 Read `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md` §5 to locate the existing Story 2.3 addendum (at line 210).
  - [ ] 9.4 Append the ≤8-line Story 2.4 addendum per AC6 immediately after the Story 2.3 addendum.
  - [ ] 9.5 Read `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` §8.1 to locate the existing "Cross-Spirit memory isolation corpus" paragraph (at line 47).
  - [ ] 9.6 Insert the ≤4-line Story 2.4 framework-hooks addendum per AC6 immediately after that paragraph.
  - [ ] 9.7 Read `_bmad-output/planning-artifacts/spirit-development-and-sharing.md` top-of-file callouts to locate the existing Story 2.3 callout (at line 23).
  - [ ] 9.8 Insert the Story 2.4 callout per AC6 immediately after the Story 2.3 callout.
  - [ ] 9.9 Verify no broken cross-references: `grep -rn "22 workspace members\|spirit_test feature\|IsolationHookPoint\|LCAS clearly-decidable" _bmad-output/planning-artifacts/architecture-maos-minimal-opus/` should show ONLY the 3 files updated above.

- [ ] **Task 10 — Discipline-suite sweep + cold-cache integration + self-review** (AC: 7)
  - [ ] 10.1 Run the full local discipline suite chained per AC7. Capture each gate's exit code in the dev record's `Gates Status` section.
  - [ ] 10.2 Run `cargo clean -p maos-spirit-sdk && cargo test -p maos-spirit-sdk --features spirit_test --locked` cold — all 12 new tests pass.
  - [ ] 10.3 Run `cargo clean -p maos-bin && cargo clean -p maos-spirit-hello && ./tests/integration/v01_evaluator_path.sh` cold — passes (regression preserved).
  - [ ] 10.4 Run `./tests/integration/onb_nfr2_timing.sh` — passes (NFR-Onb-2 5-min path stays green).
  - [ ] 10.5 Run `cargo test -p example-spirit --locked` — passes (Story 2.3's example crate unmodified).
  - [ ] 10.6 Run `cargo run -p xtask -- example-spirit-regen --check --json` — exit 0 (Story 2.3's drift detector stays green).
  - [ ] 10.7 Run `cargo run -p xtask -- abi-diff --base abi-baseline/v1-pre-bump.txt --json` — confirm 0 added/changed/removed against `maos-spirit-abi`. If non-zero, investigate; the only legitimate path for an additive symbol is via `cargo public-api` baseline refresh per Story 2.2 dev record's pattern.
  - [ ] 10.8 Cite the SPECIFIC `discipline.yml` run on the PR commit in the dev record (per A8: `discipline.yml run <run_id>, conclusion: success` — explicitly distinguish from `journal-append.yml`).
  - [ ] 10.9 Write the dev record's "What did NOT happen this story" section per AC7 — grep-verify each anti-claim listed.
  - [ ] 10.10 Author the ≥24-item self-review checklist per AC7. Each item is a concrete, mechanically-verifiable assertion.
  - [ ] 10.11 Compose the dev record's "LCAS authoring spread" section: enumerate the halt-tag categories represented in the 70-item corpus + per-category item counts (mandates the ≥6 distinct halt-tag categories AC2 requires).
  - [ ] 10.12 Compose the dev record's "Manifest self-check duplication" section: document the intentional duplication of the kernel-side section parsers in `spirit_test/manifest.rs` + the consolidation path at Story 7.1.
  - [ ] 10.13 Compose the dev record's "Halt resolution forward-anchor decision" section: cite that the 3-kind enum (`ProvidedContext`, `AcceptedHalt`, `AuthorizedOverride`) is the forward-anchor contract for Story 4.1's runtime mechanism + the contract is committed in `spirit_test/halt.rs` (NOT in `maos-spirit-abi` — preserves the post-1b.4 ABI freeze).
  - [ ] 10.14 Compose the dev record's "Epic 2 retro readiness" section: enumerate the 7 explicitly-deferred items (per-language SDK, judge-LLM agreement layer, full LCAS N=210, NFR-Sec-14 200-corpus, Story 4.1 runtime halt-protocol, Story 5.1 hook firing, Story 7.5b gate execution) so `epic-2-retrospective` opens with a clean inventory.

## Dev Notes

### Architectural anchor — Epic 2 closes here

Per `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md` lines 25–26:

> **Acceptance demo:** External developer clones `spirit-template`, implements `on_idle`, runs `cargo test` (which invokes spirit-test SDK harness), gets passing report — **without** reading kernel internals.

Story 2.3 shipped the template + `LocalRunner`. Story 2.4 ships the `SpiritTest` harness with assertion macros — completing the "invokes spirit-test SDK harness, gets passing report" half of the epic-line acceptance demo. After Story 2.4 lands, the Epic 2 acceptance demo is fully provable end-to-end:
1. `cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit` (Story 2.3)
2. Author writes `fn on_idle(&self, ctx: &mut Ctx) { ... }` using the `#[spirit]` macro (Story 2.1)
3. `cargo test --features maos-spirit-sdk/spirit_test` invokes `SpiritTest::new(...).fixture_mut().invoke_on_idle = true; ... assert_hook_fired!(report, "on_idle", 1);` (Story 2.4)
4. Report passes. Author has not read a single line of `crates/maos-kernel-core/*`.

### The forward-anchor contracts

Three of Story 2.4's surfaces are **forward-anchor contracts** — types whose shape ships now as `maos-spirit-sdk` types (NOT promoted to `maos-spirit-abi`) and whose runtime mechanism ships at a later story. The contract guarantee: the runtime story (4.1 / 4.5 / 5.1) MUST preserve the type shape Story 2.4 ships. If the runtime story needs to change the shape, it must escalate (which would re-open Story 2.4 scope).

| Type | Ships at v0.3 in | Runtime mechanism at | Forward-anchor contract |
|---|---|---|---|
| `HaltResolutionKind` (3 variants) | `spirit_test::halt` | Story 4.1 (halt protocol) | The 3 variant names + payloads stay fixed |
| `IsolationHookPoint` (4 methods) + `IsolationAttackCategory` (8 variants) | `spirit_test::isolation` | Story 4.5 (200-corpus) | The 4 methods + 8 categories stay fixed |
| `MockBusFrame` / `MockBusFrameKind` | `local_runner` (Story 2.3) | Story 2.4 (this story) | Promoted from "reserved" to "working capture" by Story 2.4 |

This pattern preserves the post-1b.4 `maos-spirit-abi` freeze (no additions to the ABI crate) while letting downstream stories inherit a working harness rather than retrofitted scaffolding.

### Why feature-gated module rather than sibling crate

The `serde` / `serde_test` precedent uses a sibling crate. The `tokio` / `tokio-test` precedent uses a sibling crate. The MAOS choice for `spirit_test` is a **feature-gated module inside `maos-spirit-sdk`** — different from those precedents. Rationale:

1. **Small surface.** `spirit_test` is ~1500 LOC at v0.3 and likely caps at ~5000 LOC at v0.5 with judge-LLM agreement + per-language ports. A sibling crate would add workspace overhead + cross-crate compile-time coupling for a feature that's always co-released with the SDK.
2. **Zero-kernel-dep already enforced.** The constraint that fixed the `serde_test` split (different dep graph from production `serde`) is satisfied here by the feature flag + `cargo tree` verification.
3. **No publishing-cycle issue.** `serde_test` is published independently for users who want only `serde` without test infrastructure. MAOS Spirit authors who ship a Spirit will ALWAYS want `spirit_test` available (it's the test harness for the Spirit they're building). The feature flag captures this: opt-in for production builds, default-on for development.
4. **The duplication note (AC1).** The manifest self-check primitive re-implements the kernel-side section parsers' field shape — duplication that a sibling crate wouldn't help with either. The consolidation path is Story 7.1's manifest-types sub-crate.

If Story 7.1 extracts `spirit_test` into a sibling crate (e.g., `crates/maos-spirit-test/`), this is a structural change worth re-evaluating. At v0.3 the feature-flag-on-existing-SDK choice is the lower-friction option.

### Why 70 LCAS items hand-authored at v0.3

PRD NFR-Test-6 commits N=210 at v0.5. Epic 2 line 23 commits 70 of those 210 at v0.3 (clearly-decidable bucket). Why 70 hand-authored:

- **Statistical adequacy for substrate gate.** 70 items is the practical maximum a single dev can hand-author in a sprint while preserving thematic spread; the v0.5 expansion to 210 uses `maos-corpus-gen::lcas` generator at Story 7.1 (which can produce ~5000 items at zero-marginal-cost, but those items need to be CALIBRATED against the 70 hand-authored seeds — the generator's seed corpus IS the v0.3 70-bucket).
- **No A2A scenarios available at v0.3.** Epic 2 line 22: *"the remaining 140 items (genuinely-ambiguous + adversarially-misleading) are explicitly deferred to E2 + E7/E8 (require A2A scenarios from E6 to be valid)."* The adversarially-misleading bucket specifically requires cross-Spirit attack vectors that the A2A peer mesh (Epic 6) makes possible. Authoring those at v0.3 would produce items that can't be evaluated until v0.8.
- **Halt-recall / halt-precision measurement substrate.** PRD line 80: *"Mann-Whitney U at p<0.01 needs ~64 per group at power=0.84."* 70 per bucket is the floor; v0.3 ships the FIRST bucket so Story 4.1's halt-recall ≥0.7 + halt-precision ≥0.85 measurement (NFR-Test-4) has a working substrate.

### Why the manifest self-check duplicates kernel-side parsers

The manifest self-check primitive at `spirit_test::manifest` re-implements a minimal section-parsing skin. This is INTENTIONAL duplication. The alternative (cross-crate dep on `maos-kernel-core::security::manifest`) would couple every third-party Spirit's compile to the kernel — violating the AC1 constraint `cargo tree -p maos-spirit-sdk --features spirit_test | grep -c maos-kernel-core` must output 0.

The Story 7.1 consolidation path: extract a `crates/maos-manifest-types/` sub-crate with the field-level types (`ClassSection`, `CapabilitiesRequired`, etc.) consumed by BOTH `maos-kernel-core::security::manifest` (kernel-side parsing + admission) AND `maos-spirit-sdk::spirit_test::manifest` (SDK-side self-check). At v0.3 the simpler choice is to duplicate + document — captured in the dev record's "Manifest self-check duplication" section.

### Cross-references — what other stories consume Story 2.4's surfaces

| Story | What it consumes | Story 2.4 surface |
|---|---|---|
| 4.1 (halt protocol) | The 3-variant `HaltResolutionKind` enum shape | `spirit_test::halt::HaltResolutionKind` |
| 4.5 (NFR-Sec-14 200-corpus) | `IsolationHookPoint` 4-method trait + `IsolationAttackCategory` 8-variant enum + `CrossSpiritIsolationFixture` 2-Spirit harness | `spirit_test::isolation::*` |
| 5.1 (runtime hook firing) | The vtable dispatch path Story 2.3 + 2.4's `LocalRunner`/`SpiritTest` wrapping exercises | `SpiritVtable<S>` from `maos-spirit-abi` |
| 7.1 (full per-language SDK) | The 5 assertion macros + manifest self-check + halt simulator + regression corpus + isolation framework as canonical Rust reference shape | All of `spirit_test::*` |
| 7.5b (NFR-Onb-1 30-min gate) | The SDK seed as the "passing CI" criterion participants exercise on their Spirits | `spirit_test::*` + assertion macros |
| 8.1 (Butler) | `assert_emits_frame!` + `assert_halts_with!` + `manifest_self_check!` for acceptance tests | `spirit_test::assert::*` |
| 8.2 / 8.3 / 8.4 / 8.5 (reference Spirits) | The `RegressionCorpus` + `SpiritClass` typed container | `spirit_test::regression::*` |
| 10.2 (third-party trial) | The SDK seed via `spirit-development-and-sharing.md` §13 link | All of `spirit_test::*` |

### Project Structure Notes

- New module: `crates/maos-spirit-sdk/src/spirit_test/` (7 files: mod.rs + harness.rs + assert.rs + halt.rs + manifest.rs + regression.rs + isolation.rs)
- New tests: `crates/maos-spirit-sdk/tests/spirit_test_smoke.rs`, `lcas_smoke.rs`, `isolation_smoke.rs`
- New corpus: `tests/corpora/lcas-v0.3.jsonl` (70 lines) + 1 new `[corpus."lcas-v0.3"]` block in `tests/corpora/MANIFEST.toml`
- Workspace member count UNCHANGED: stays at 22 (the new module is feature-gated inside existing `maos-spirit-sdk` crate)
- `Cargo.toml` changes: `maos-spirit-sdk/Cargo.toml` gains `toml` dep + `spirit_test` feature + 3 dev-deps (`sha2`, `serde_json`, `serde`)
- CI: `.github/workflows/discipline.yml` gains 3 jobs (30 → 33 total)
- Coverage matrix: `tests/coverage-matrix.yaml` updates 4 rows (FR34, NFR-Test-6, NFR-Sec-14, NFR-Test-3)
- Gate registry: `xtask/gate-registry.toml` appends 3 new gate names
- Architecture docs: 4 files updated additively (4-kernel-design.md addendum, 5-spirit-abi.md addendum, 8-security-approval-model.md addendum, spirit-development-and-sharing.md callout)
- NO changes to: `templates/spirit-rust/`, `examples/example-spirit/`, `crates/maos-spirit-abi/`, `crates/maos-spirit-derive/`, `crates/maos-spirit-hello/`, `crates/maos-bin/`, `crates/maos-kernel-core/`, `xtask/src/check_service_boundary.rs`, `xtask/src/example_spirit_regen.rs`, `abi-baseline/v1-pre-bump.txt`, `docs/adr/`, `docs/invariants/i9-exemptions.md`

### References

- Epic 2 line 14: spirit-test SDK seed scope — `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:10`
- Epic 2 line 21-22: LCAS 70/210 split — `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:15` + `:22`
- Epic 2 line 25: acceptance demo — `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:25`
- Epic 2 Story 2.4 raw ACs — `_bmad-output/planning-artifacts/epics/epic-2-spirit-abi-developer-sdk-boundary-contracts-v01-v03.md:111-140`
- Epic 4 line 17: 8 isolation categories — `_bmad-output/planning-artifacts/epics/epic-4-halt-protocol-memory-substrate-cognition-primitives-v03-v10-single-halt-owner.md:17`
- Epic 4 line 21: NFR-Test-6 + NFR-Sec-14 + NFR-Aud-7 + NFR-Aud-14 floors — `:21`
- Epic 8 line 23: LCAS bucket split (E2 owns clearly-decidable; E8 owns 140 at v0.8) — `_bmad-output/planning-artifacts/epics/epic-8-reference-spirits-butler-researcherobserver-orchestratorworkersarchitectreviewer-miranash-v03-v15.md:23`
- Architecture §5.1 manifest schema: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md:9-113`
- Architecture §5.3 lifecycle hooks (14-hook architecture; 11 of 14 ship at Epic 2): `5-spirit-abi.md:173-210`
- Architecture §6.3 halt-precision-recall: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md:12`
- Architecture §8.1 isolation corpus (200 scenarios, 8 categories): `8-security-approval-model.md:13` + `:45-47`
- Architecture §10.6 Diego J6 onboarding (spirit-test reference): `10-journey-traceability.md:130`
- ADR-002 Spirit form at v0.1 — subprocess only, inproc gated on measurement: `12-architecture-decision-records.md:67-110`
- ADR-019 + ADR-022 halt protocol (Story 4.1 owns the mechanism): `12-architecture-decision-records.md` + epic-4 line 35
- ADR-040 Sec-14a + Sec-14b threat-model split: `12-architecture-decision-records.md:44` + `:524`
- PRD NFR-Sec-14 (200/200, P0 ship-block, v0.8): `_bmad-output/planning-artifacts/prd/non-functional-requirements.md:47`
- PRD NFR-Test-6 (N=210 in 3 buckets of 70, v0.5 ship gate): `prd/non-functional-requirements.md:80`
- PRD NFR-Test-3 (SDK coverage ≥80%): per coverage-matrix.yaml + epic-2 line 19
- PRD line 208 (v0.5 ships NFR-Test-6 full): `prd/non-functional-requirements.md:208`
- PRD line 210 (v0.8 ships NFR-Sec-14 200-corpus): `prd/non-functional-requirements.md:210`
- Story 2.3 dev record (LocalRunner shape + zero-kernel-dep verification pattern): `_bmad-output/implementation-artifacts/2-3-thin-cargo-generate-template-local-runner-nfr-onb-1-v0-3-prerequisite.md`
- Story 2.2 dev record (corpus registration + JSONL schema pattern): `_bmad-output/implementation-artifacts/2-2-xtask-check-service-boundary-p1-p4-full-implementation-spirit-boundary-invariant-cases.md`
- Story 2.1 dev record (Spirit ABI + `#[spirit]` macro): `_bmad-output/implementation-artifacts/2-1-ship-the-full-spirit-abi-with-spirit-proc-macro-and-11-lifecycle-hooks.md`
- Story 0.3 corpus infrastructure contract: `_bmad-output/implementation-artifacts/0-3-content-addressed-corpora-infrastructure-coverage-matrix-ci-gate.md`
- Existing `crates/maos-spirit-sdk/src/local_runner.rs` (Story 2.3 substrate)
- Existing `crates/maos-spirit-abi/src/lifecycle.rs` (Story 2.1 trait + vtable + payload types)
- Existing `crates/maos-spirit-abi/src/ctx.rs` (Story 2.1 `Ctx::mock()` constructor)
- Existing `crates/maos-kernel-core/src/security/manifest.rs` (kernel-side section parsers — referenced for duplication discipline)
- Existing `tests/corpora/MANIFEST.toml` (Story 0.3 schema)
- Existing `tests/corpora/spirit-boundary-v0.1.jsonl` (Story 2.2 JSONL pattern Story 2.4's LCAS mirrors)
- Existing `xtask/gate-registry.toml` (post-Story 2.3 17-gate baseline)
- Existing `.github/workflows/discipline.yml` (post-Story 2.3 30-job baseline)
- Epic 1b retro A6/A7/A8 cold-cache + `-p` selection + `discipline.yml` citation discipline: `_bmad-output/implementation-artifacts/epic-1b-retro-2026-05-16.md:182-199`

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
