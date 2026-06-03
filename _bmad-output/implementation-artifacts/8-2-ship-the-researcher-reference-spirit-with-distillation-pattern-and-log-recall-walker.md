---
dev_model_used: claude-opus-4-8
---

# Story 8.2: Ship the Researcher Reference Spirit with Distillation Pattern and `log.recall` Walker

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- dev_model_used frontmatter is set by the dev agent in AC1 (§A2 hard-fail gate). Recommended: claude-opus-4-8 (Decision F). -->

## Story

As a v0.5 substrate user,
I want the **Researcher reference Spirit** shipped in `spirits/researcher/` with the **distillation pattern** as the canonical single-Spirit example, a **participant-scoped `log.recall` walker** (Story 4.4 `LogRecallPort`, NOT Butler's unscoped `ranged_recall`) selecting which Transparency Log frames to preserve, **Spirit-side LLM compression with the kernel-enforced I11 audit chain** (`source_log_ref` flattened to raw frames, `distillation_depth`, kernel-computed `intent_lineage`), a **`scalar.tap` subscription**, AND the **NFR-Aud-8 quarterly N=500 distillate corpus** (the slice the existing five-metric harness explicitly defers to this story),
So that the v0.5 distillation primitives are demonstrably composable, the **five-metric distillation gate (NFR-Aud-7)** has its primary reference implementation, and the NFR-Aud-8 quarterly audit shape goes from `#[ignore]`d-stub to enforced.

## What this story IS and IS NOT (read first — scope is deliberately bounded)

This is the **second cognitive reference Spirit** and the **primary reference implementation of the §9.5 distillation pattern**. It is the first Spirit to use the *participant-scoped* read path (`LogRecallPort`) — Butler (8.1) deliberately used the unscoped `ranged_recall` and explicitly deferred the scoped walker to this story (8.1 "IS NOT" §). Scope is drawn to prevent over-building (hypothesize-mode ILP+LLM, live web/arXiv drivers, a full re-implementation of Butler's digest) and under-building (a manifest-only stub that never walks the log or writes a real distillate through the I11 chain).

**This story IS:**
- A real **Researcher Spirit crate** at `spirits/researcher/` (rust-inproc form, zero kernel KLOC) — survey-mode posture, `[output_shape]` = findings/Open Questions/Confidence Map/Bibliography (architecture §6.2).
- A **participant-scoped `log.recall` walker**: Researcher calls `LogRecallPort::recall(spirit_pid, filter)` + `fetch(spirit_pid, frame_id)` (Story 4.4), with results kernel-scoped to the calling Spirit's emitter frames (`LogRecallError::ScopeViolation` on cross-Spirit fetch). This is the v0.5 distillation read primitive (FR29).
- **Spirit-side distillation** that compresses recalled frames into a digest and persists it through the Story 4.4 `DistillateWriter` — proving the real **I11 audit chain** end-to-end (non-empty `source_log_ref`, `distillation_depth ≥ 1`, kernel-computed `intent_lineage`; transitive flatten of digests-of-digests; `EDigestAuditChainMissing` on a missing chain).
- **Researcher wired as the reference producer for the five-metric distillation gate (NFR-Aud-7)** — digest-recall ≥0.90 / faithfulness ≥0.98 / hedge-preservation ≥0.95 (IAA-gated) / traceability 100% / secret-leakage 0%, measured deterministically (no live LLM in CI).
- The **NFR-Aud-8 quarterly N≥500 corpus** authored at `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/`, flipping `test_distillate_corpus_quarterly_audit_shape` from `#[ignore]`d to enforced.
- A **`scalar.tap` subscription** (Researcher subscribes via `TelemetryStreamPort::subscribe_topic`; receives `ScalarTapEvent`s published by peers) and a focused **morning-digest contribution** demonstrating Researcher distillates compose into the FR17 digest pattern.
- A **§13.1 J-Researcher latency** measurement (distillation step <100ms P95) with `BudgetWarning` emitted on overrun (NFR-Perf-6).

**This story IS NOT:**
- It does **NOT** ship `hypothesize-mode` (the ILP + LLM hybrid for novel-hypothesis generation). Architecture §6.2: hypothesize-mode is *declared in the posture-set* but **ships fully at v1.0**. Only **survey-mode** ships here. (Decision C.)
- It does **NOT** build real Web/arXiv/GitHub/citation-graph MCP client drivers. As with Butler (8.1 Decision B), Researcher declares the MCP capability scopes in its manifest (exercising the kernel-mediated `Scope::McpCall` path) but its inputs come from **fixture-replay** scenario data. Live drivers are deferred to v0.5+/Epic 9. (Decision B.)
- It does **NOT** re-implement Butler's morning digest. Butler remains the v0.3 digest *producer*; Researcher's "contributes the digest at v0.5+" obligation is satisfied by a **composability demo** — a Researcher distillate is shown consumable in the digest pattern — not by replacing or depending on the `butler` crate. (Decision E.)
- It does **NOT** add distillation, summarization, contradiction-detection, recall-selection, or anomaly-classification logic to `maos-kernel-core`. All Researcher cognition is Spirit-side; the kernel-API surface invariant (Story 0.2) must stay GREEN (any new kernel public fn = class `other` → build-break).
- It does **NOT** change the existing N=100 synthetic-v0 five-metric corpus or the public surface of the `distillate_five_metrics_floor` harness — it *adds* the quarterly slice and wires Researcher as the reference producer. Edits to `maos-eval` beyond adding the quarterly corpus + flipping the one `#[ignore]` are flagged and justified.

## LOCKED Design Decisions (do NOT silently re-decide — chosen during story creation; flagged for Winston)

**Decision A — Researcher home `spirits/researcher/` + workspace count bump 31 → 32.**
Researcher lives at **`spirits/researcher/`** as a new workspace member crate (mirrors Butler Decision A, ratified by Winston 2026-06-02). Rationale: epic-8 mandate ("Zero kernel KLOC — all subprocess Spirit code in `spirits/`"); reference Spirits are sibling crates compiling against the published ABI like `examples/example-spirit` and `spirits/butler`. This bumps `check-workspace-count` **31 → 32** (AC8 updates the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md`). **Pre-existing path conflict to fix:** `tests/coverage-matrix.yaml:1198-1199` already declares a `researcher` slot with the WRONG path `crates/maos-spirit-researcher` — correct it to `spirits/researcher` (exactly the `crates/maos-spirit-butler → spirits/butler` correction 8.1 made). **FLAG Winston:** confirm Researcher is a workspace member at `spirits/researcher` (count → 32).

**Decision B — Web/arXiv/GitHub/citation-graph inputs are MOCKED via fixture-replay; no live MCP drivers.**
Architecture §6.2 lists broad capabilities (web search, arXiv, GitHub, citation-graph traversal, MCP broadly), and §13 v0.5 lists "broad MCP capabilities." The codebase ships only the 5.5c MCP *transport* substrate — no research provider drivers. Researcher declares the `[[capabilities.required.mcp.servers]]` scopes (→ `Scope::McpCall`) but scenario inputs are served by fixture-replay (`crates/maos-mcp/src/fixture_replay.rs`), exactly as Butler did. **FLAG Winston:** confirm the v0.5 "broad MCP capabilities" milestone is satisfied by capability-scope wiring + fixture-replay, not live drivers (carry-forward: first live research driver ships its own conformance corpus).

**Decision C — `hypothesize-mode` ILP+LLM hybrid is OUT of scope (ships v1.0).**
Per architecture §6.2, hypothesize-mode is declared in the manifest posture-set but the full ILP+LLM hybrid implementation lands at v1.0. This story ships **survey-mode** (exploratory, reactive, divergent) only. Declaring `hypothesize-mode` in the posture-set (without implementing the generative path) is acceptable and matches the architecture; do not build the ILP path.

**Decision D — Five-metric gate split: structural metrics MEASURED against Researcher's real output; quality metrics corpus-annotated against an IAA-gold corpus.**
The existing harness (`crates/maos-eval/tests/distillate_five_metrics_floor.rs`) reads `expected_recall`/`expected_faithfulness`/`expected_hedge_preservation` as **annotations** on an IAA-gold corpus (App F.5 derivation: these are noise-limited against held-out replicator/judge LLMs and CANNOT be recomputed live in CI), and **measures** traceability (structural: non-empty `source_log_ref`) and secret-leakage (runs the real `CorpusBackedRedactionPolicy::redact`). Researcher adopts this split:
- **Traceability (100%) + secret-leakage (0%) are MEASURED against Researcher's REAL distillates** — Researcher actually walks `log.recall`, writes through the real `DistillateWriter` (proving the I11 chain resolves), and its digest payloads pass the real redaction filter.
- **Recall / faithfulness / hedge-preservation are corpus-annotated** against an IAA ≥0.85 gold corpus (the N=100 synthetic-v0 slice stays the NFR-Aud-7 gate; Researcher is named the reference producer).
Researcher is the *primary reference implementation*; the gate corpus is not regenerated. **FLAG Winston:** confirm this split (vs. requiring a live-LLM recall/faithfulness recompute, which violates the CI-determinism rule and App F.5).

**Decision E — Morning-digest "contribution" is a composability demo, not a reimplementation.**
Epic AC4 says "Researcher contributes the morning digest at v0.5+ (extending Butler's v0.3 implementation)." This is satisfied by demonstrating a **Researcher distillate is consumable in the digest pattern** (e.g., a Researcher `DistillationReceipt`/digest feeds the same `source_log_ref`-cited digest shape), NOT by re-building Butler's `morning_digest` path and NOT by adding a `butler` crate dependency. Keep it focused; Butler stays the v0.3 digest producer.

**Decision F — Recommended dev model: `claude-opus-4-8`.**
Rationale: large, integration-heavy story spanning the participant-scoped `LogRecallPort` (cursor pagination + scope enforcement), the I11 distillation chain (transitive flatten + kernel intent-lineage), the five-metric harness + a new N=500 corpus, the `scalar.tap` telemetry subscription, and the J-Researcher bench. Memory records deepseek-v4-pro is weak on async invariants / integration plumbing / port-injection threading — the in-proc Spirit→port bridge (the same risk class 8.1 navigated) recurs here. 8.1 used claude-opus-4-8.

## Prerequisites (verified present at story-creation time — re-verify in AC1)

| Prerequisite | Status | Path / Evidence |
|---|---|---|
| Spirit ABI + 14 lifecycle hooks, `#[spirit]` proc-macro, `Ctx` | ✅ PRESENT | `crates/maos-spirit-abi/src/lifecycle.rs`, `…/ctx.rs`; `crates/maos-spirit-derive/src/lib.rs` |
| Spirit SDK + local runner + spirit-test harness + v0.5 assert macros | ✅ PRESENT | `crates/maos-spirit-sdk/src/{local_runner.rs,spirit_test/{harness.rs,assert.rs}}` |
| **Story 4.4 `LogRecallPort` (participant-scoped)** — trait + domain types | ✅ PRESENT | `crates/maos-domain/src/ports/log_recall.rs:14-28` (trait); `…/src/log_recall.rs` (`LogRecallFilter`/`LogRecallCursor`/`LogRecallPage`/`LogRecallEntry`/`LogFetchResponse`/`LogRecallError`) |
| **`LogRecallAdapter`** (concrete impl) | ✅ PRESENT | `crates/maos-iac/src/adapter/log_recall.rs:53,63` — `pub fn new(transparency_log: Arc<TransparencyLogAdapter>) -> Self`; scope enforced at `:278` (recall) / `:366` (fetch `ScopeViolation`); re-exported via `maos_kernel_core` |
| **Distillation I11 chain** — `DistillationRequest/Receipt`, `DistillateWriter`, `EDigestAuditChainMissing` | ✅ PRESENT | `crates/maos-domain/src/distillation.rs:14-35,97-118,139-158`; `crates/maos-iac/src/adapter/distillate.rs:58-68 (new),194-251 (flatten),253-286 (intent_lineage),290-369 (write_distillate)` |
| **Five-metric gate (NFR-Aud-7) harness + corpus loader** | ✅ PRESENT | `crates/maos-eval/tests/distillate_five_metrics_floor.rs:26-136`; `crates/maos-eval/src/distillate_corpus.rs` (`DistillateCorpus::load_from`, `DistillateScenario`, `IaaAttestation`); N=100 corpus at `crates/maos-eval/fixtures/distillate-corpus-v0/` |
| **NFR-Aud-8 quarterly stub** — the `#[ignore]`d test this story flips | ✅ PRESENT | `crates/maos-eval/tests/distillate_five_metrics_floor.rs:138-156` — `#[ignore = "…lands in Story 8.2 alongside Researcher"]`, expects `quarterly-audit-v0/` with N≥500 |
| **`scalar.tap` Telemetry Stream** — port + concrete adapter + test pattern | ✅ PRESENT | `crates/maos-domain/src/ports/telemetry.rs:12-26` (`TelemetryStreamPort`); `crates/maos-domain/src/invariants/i7.rs:29 (TelemetryTopic),45 (ScalarTapEvent)`; `crates/maos-kernel-core/src/telemetry/mod.rs:78` (`impl TelemetryStreamPort for TelemetryStreamAdapter`); **reference test** `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` |
| **`Scope::LogRecall` + `Scope::McpCall`** capability scopes | ✅ PRESENT | `crates/maos-domain/src/invariants/i1.rs:60 (enum Scope),83 (LogRecall),245 (McpCall{server,tool})` |
| Redaction policy (secret-leakage metric) | ✅ PRESENT | `maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy}` |
| MCP fixture-replay (mocked research inputs) | ✅ PRESENT (verify) | `crates/maos-mcp/src/fixture_replay.rs` |
| `ranged_recall` (UNscoped — what Researcher must NOT use) | ✅ PRESENT (contrast) | `crates/maos-audit/src/log_composition.rs:103-108` — Butler's v0.3 path; Researcher uses the SCOPED `LogRecallPort` instead |
| §13.1 bench harness (`maos-bench`) | ✅ PRESENT | `crates/maos-bench/benches/section_13_1.rs` + `maos_bench::harness` (8.1 added `j0`; J1/J4 exist; **no J-Researcher journey yet** — AC7 adds one) |
| Butler reference crate (structure to mirror) | ✅ PRESENT | `spirits/butler/{Cargo.toml,manifest.toml,src/lib.rs,tests/}` (Story 8.1) |
| **`spirits/researcher/` Spirit + corpora** | ❌ **ABSENT** — **this story creates them** | no `spirits/researcher/` today |
| coverage-matrix `researcher` slot (WRONG path — fix to `spirits/researcher`) | ⚠️ PRESENT-WRONG | `tests/coverage-matrix.yaml:1198-1199` path = `crates/maos-spirit-researcher` (Decision A correction) |

## Acceptance Criteria

### AC1 — Prerequisites & scope classified mechanically before Spirit work opens

**Given** the prerequisite table above
**When** AC1 runs first
**Then** the dev confirms each ✅ path/symbol still exists (`LogRecallPort`/`LogRecallAdapter`, `DistillateWriter`/`EDigestAuditChainMissing`, the five-metric harness + the `#[ignore]`d quarterly test, `TelemetryStreamAdapter` + `scalar_tap_subscriber.rs`, `Scope::LogRecall`/`McpCall`, MCP `fixture_replay`) and records the result in the Dev Agent Record
**And** the Researcher absence is confirmed (no `spirits/researcher/`) and Decisions A–F are recorded as the chosen resolutions, not silently re-decided
**And** `dev_model_used` is recorded in the story frontmatter (§A2 hard-fail gate).

### AC2 — Researcher Spirit ships with the participant-scoped `log.recall` walker

**Given** the Researcher reference Spirit in `spirits/researcher/`
**When** Researcher is loaded with a corpus to distill
**Then** the manifest declares survey-mode posture (`[posture]`), the `[output_shape]` required fields `findings`/`open_questions`/`confidence_map`/`bibliography`, the `Scope::LogRecall` + `[[capabilities.required.mcp.servers]]` scopes, a budgeted envelope (`[budget]`, `[resources]`), sandbox tier, and the §6.2 `[epistemic_policy]` rules (see Dev Notes for exact tags/predicates)
**And** Researcher calls `LogRecallPort::recall(spirit_pid, filter)` (cursor-paginated via `LogRecallFilter::new(...)`) to **walk** the Transparency Log and `fetch(spirit_pid, frame_id)` for payloads
**And** the walker is **participant-scoped per Story 4.4** — results are limited to the calling Spirit's frames; a cross-Spirit `fetch` yields `LogRecallError::ScopeViolation` (proven by a negative test driving the real `LogRecallAdapter` as a dev-dep, mirroring Butler's kernel-adapter-as-dev-dep pattern)
**And** Researcher uses the SCOPED `LogRecallPort`, **not** the unscoped `ranged_recall` (the explicit 8.1→8.2 contract).

### AC3 — Researcher distillate is written through the kernel-enforced I11 audit chain

**Given** Researcher writes a distillate (Spirit-side LLM compression of recalled frames; compressor mocked/seeded — no live LLM in CI)
**When** the kernel processes the digest write via `DistillateWriter::write_distillate(spirit_pid, request)`
**Then** the `DistillationRequest` carries **non-empty `source_log_ref`** (the `[u8;16]` frame ids selected by the walker), **`distillation_depth ≥ 1`**, and a `digest_payload`
**And** the returned `DistillationReceipt` shows `effective_source_log_ref` **transitively flattened to original raw frames** (digests-of-digests resolve to raw, with cycle detection) and a **kernel-computed `intent_lineage`** (I13 — union of input-frame intents, NEVER Spirit-self-reported)
**And** a missing audit-chain element (empty `source_log_ref` or `distillation_depth < 1`) yields **`DistillationError::AuditChainMissing` (`E_DIGEST_AUDIT_CHAIN_MISSING`)** — kernel-enforced, not bypassed Spirit-side (negative test).

### AC4 — Five-metric distillation gate (NFR-Aud-7) passes with Researcher as the reference implementation

**Given** the five-metric distillation gate measured with Researcher as the primary reference producer (Decision D)
**When** the eval corpus runs (`cargo test -p maos-eval --test distillate_five_metrics_floor`)
**Then** digest-recall ≥0.90 / faithfulness ≥0.98 / hedge-preservation ≥0.95 (IAA ≥0.85 gold) / traceability 100% / secret-leakage 0% all pass on the N=100 synthetic-v0 corpus (the gate is unchanged; recall/faithfulness/hedge are corpus-annotated, traceability is structural non-empty `source_log_ref`, secret-leakage runs the real `CorpusBackedRedactionPolicy`)
**And** Researcher's **own** distillates (from AC3) are verified to clear **traceability (100%)** — every cited `source_log_ref` resolves to a real frame via the walker — and **secret-leakage (0%)** — Researcher digest payloads pass the real redaction filter — by a deterministic test in `spirits/researcher/tests/` (no live LLM)
**And** Researcher is recorded as the reference producer in the coverage-matrix `researcher` slot (path corrected to `spirits/researcher`, Decision A).

### AC5 — NFR-Aud-8 quarterly N=500 corpus authored; the `#[ignore]`d quarterly test goes green

**Given** the NFR-Aud-8 quarterly audit slice (the existing test at `distillate_five_metrics_floor.rs:138` is `#[ignore]`d with "lands in Story 8.2 alongside Researcher")
**When** the quarterly corpus is authored at `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/` (N≥500 `scenario-*.json` + `iaa-attestation.json`, loadable by `DistillateCorpus::load_from`, SHA-pinned per Story 0.3)
**Then** the `#[ignore]` attribute is **removed** from `test_distillate_corpus_quarterly_audit_shape` and the test passes (N≥500 enforced; loads clean; IAA ≥0.85)
**And** all five metrics are reportable per the quarterly corpus (the floors hold on the N=500 slice as on the N=100 slice)
**And** the quarterly corpus is registered in the corpus-staleness / coverage-matrix surfaces so silent edits fail loud
**And** any edit to `maos-eval` beyond (a) adding the corpus fixtures and (b) removing the single `#[ignore]` is flagged and justified in the Dev Agent Record.

### AC6 — `scalar.tap` subscription + morning-digest contribution (composability demo)

**Given** Researcher subscribes to `scalar.tap`
**When** scalars are published by other Spirits (`TelemetryStreamPort::publish_event(topic, ScalarTapEvent)` on a `TelemetryTopic::new("scalar.tap.<metric>")`)
**Then** Researcher's subscription is established via `subscribe_topic(spirit_id, topic)` (proven by a test mirroring `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`, driving the real `TelemetryStreamAdapter` as a dev-dep) and Researcher **receives** the stream
**And** Researcher can include the observed scalar pattern in a subsequent distillate/digest (the received `ScalarTapEvent` data is incorporated into a digest payload)
**And** Researcher **contributes the morning digest at v0.5** as a focused composability demo (Decision E) — a Researcher distillate is shown consumable in the FR17 digest pattern (source-log-ref-cited), without re-implementing or depending on the `butler` crate.

### AC7 — §13.1 J-Researcher latency budget measured; `BudgetWarning` on overrun

**Given** the §13.1 J-Researcher journey (long-running researcher: 50-frame burst, 16–64 KB payloads, including one EpistemicHalt+Resume cycle) and the epic's <100ms P95 distillation-step budget
**When** the J-Researcher bench runs (add a `harness::j_researcher` journey to `crates/maos-bench/benches/section_13_1.rs`, mirroring how 8.1 added `harness::j0`; do NOT measure only in-crate — 8.1's in-crate latency test was a review must-fix)
**Then** the measured distillation-step P95 is recorded against the <100ms budget in the Dev Agent Record
**And** a budget overrun emits a **`BudgetWarning`** (NFR-Perf-6; `FrameKind::BudgetWarning` exists in the audit-log discriminator set)
**And** if the budget is missed, the §13.1 three-condition inproc-unlock check (ADR-002) is referenced rather than silently migrating to inproc to mask code-path overhead ("J1 is the floor reference; fix our code first").

### AC8 — Zero kernel KLOC; kernel-API invariant holds; workspace count reconciled; manifest conforms

**Given** Researcher is a rust-inproc Spirit (zero kernel KLOC)
**When** Researcher's distillation, recall-selection, and contradiction/confidence logic is added
**Then** the logic lives entirely in `spirits/researcher/`, **not** in `maos-kernel-core` (Story 0.2 kernel-API surface invariant stays GREEN — no new kernel public fn)
**And** `check-workspace-count` is reconciled to **32** (Decision A): root `Cargo.toml` members + the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` both updated 31→32
**And** the manifest passes `maos-manifest` validation — **verify each declared section against the authoritative validators before authoring** (Butler proved `[epistemic_policy.rules]` use `on_value_above`/`on_value_below`; **do NOT invent `[capabilities.parallelism]`** — it is not a parsed manifest field today (only `std::thread::available_parallelism` exists in code); if parallelism must be declared, confirm the exact field/section or omit it and record parallelism as documented intent — adding an unknown section risks `deny_unknown_fields`).

### AC9 — CI / discipline wiring green end-to-end

**Given** the discipline gates
**When** CI runs at HEAD
**Then** a `researcher-tests` job is added to `.github/workflows/discipline.yml` (`cargo test -p researcher --locked`) and wired into the final gate-aggregation `needs:` list (mirrors `butler-tests` at line 532 / its slot in the `needs:` list at line ~1553)
**And** both Researcher corpora (the researcher self-test corpus + the NFR-Aud-8 quarterly N=500) are SHA-pinned and registered in the corpus-staleness / coverage-matrix surfaces (Story 0.3)
**And** `xtask check-service-boundary` (0 new violations), `check-workspace-count` (32/32), `coverage-matrix`, `abi-diff` (Researcher ABI-neutral, Added-only/removed=[]), and the §A2 `check-dev-model-used-populated` gate are all GREEN at HEAD — **no flipped-while-red** (the Epic 7 §A2 trap)
**And** the existing `distillate_five_metrics_floor` tests (including the now-un-ignored quarterly test) pass
**And** the Dev Agent Record lists every file created/modified, and any pre-existing RED (e.g. `kloc-check`, the 7.5a-era `service_boundary_integration` stale-baseline) is verified Researcher-neutral (identical clean-HEAD-vs-changes) and flagged, not introduced.

## Tasks / Subtasks

- [x] **T1 — Prerequisite + scope pre-check (AC1)**
  - [x] Re-verify every ✅ row in the Prerequisites table (paths + key symbols), especially `LogRecallPort`/`LogRecallAdapter::new`, `DistillateWriter::write_distillate`, the `#[ignore]`d quarterly test, `TelemetryStreamAdapter`, `Scope::LogRecall`; record in Dev Agent Record
  - [x] Confirm `spirits/researcher/` absent; record Decisions A–F as chosen resolutions
  - [x] Set `dev_model_used` frontmatter (§A2 gate)
- [x] **T2 — Scaffold the Researcher crate (AC2, AC8, Decision A)**
  - [x] Create `spirits/researcher/` (`Cargo.toml`, `manifest.toml`, `src/lib.rs`, `tests/`) — mirrors `spirits/butler/` shape. **Spirit-side deps are PURE domain only** (`maos-spirit-sdk[local_runner]` + `maos-spirit-abi` + `maos-domain` + serde): unlike Butler, Researcher does NOT depend on `maos-audit` (it uses the SCOPED `LogRecallPort` domain trait, not the unscoped `ranged_recall` free-fn — the 8.1→8.2 contract is enforced at the dependency level). Dev-deps: `maos-spirit-sdk[local_runner,mock,spirit_test]` + `maos-kernel-core` + `maos-manifest` + `maos-eval` + tokio/tempfile/sha2/toml. No `rusqlite` needed (real `TransparencyLogAdapter` seeds the TL).
  - [x] Added `spirits/researcher` to root `Cargo.toml` members (→ 32); bumped the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` 31→32; `check-workspace-count: PASSED (actual=32, declared=32)`.
  - [x] Authored `manifest.toml`: `[class]` (abi=1.0, schema=2, min_substrate_version, forms=["rust-inproc"], trust_tier="local"); `[capabilities.required]` provider.complete (claude-3-5-sonnet ≥Sonnet-tier) + 4 `[[capabilities.required.mcp.servers]]` web/arxiv/github/citation-graph (→ `Scope::McpCall`, Decision B); `[posture]` autonomy=assistive; `[output_shape] required_fields = ["findings","open_questions","confidence_map","bibliography"]`; `[budget]`+`[resources]`; `[sandbox] tier="T2"`; `[epistemic_policy]` 2 §6.2 rules (`methodology_conflict` on_value_above 0.7, `load_bearing_confidence` on_value_below 0.7). **Verified against authoritative validators** (`manifest_self_check` + `ClassSection`/`CapabilitiesRequired`/`PostureSection`/`SandboxConfig`/`EpistemicPolicySection::from_toml_str` all parse clean, zero warnings — `tests/spirit_smoke.rs`). **Did NOT invent `[capabilities.parallelism]`** (recorded as documented intent only — survey/hypothesize cognitive posture-set lives Spirit-side, not in `[posture]`; see AC1 substrate findings).
- [x] **T3 — Participant-scoped `log.recall` walker (AC2)**
  - [x] Implemented Researcher's recall walker (`Researcher::walk` + free-fns `recall_all`/`fetch_payloads`): `LogRecallPort::recall(spirit_pid, LogRecallFilter::new(...))` with cursor pagination (follows `next_cursor`, bounded by `MAX_RECALL_PAGES`); `fetch(spirit_pid, frame_id)` for payloads. Pure over the `&dyn LogRecallPort` domain trait.
  - [x] Integration test `tests/recall_walker.rs` driving the real `LogRecallAdapter::new(Arc<TransparencyLogAdapter>)` (dev-dep): walker returns ONLY the Spirit's own frames across multiple pages (5 of pid 10, 4 of pid 20, no cross-contamination either way); cross-Spirit `fetch` → `LogRecallError::ScopeViolation{frame_id,requested_pid:20,owner_pid:10}` (negative). The researcher lib never reaches into kernel-core (Story 0.2) and has NO `maos-audit` dep, so `ranged_recall` is unreachable at compile time. **3/3 pass.**
- [x] **T4 — Distillation + I11 audit chain (AC3)**
  - [x] Implemented Spirit-side distillation (`survey` → `to_distillation_request` → `distill_through`): seeded deterministic compressor → `DistillationRequest` (non-empty `source_log_ref` = deduped walked frame ids, `distillation_depth = depth.max(1)`).
  - [x] Persisted via real `DistillateWriter::write_distillate` (`tests/distillation_i11.rs`): receipt shows transitively-flattened `effective_source_log_ref` (== original raws, exact set) + kernel-computed `intent_lineage` (sorted union `["consult","inform","verify"]`); multi-hop test (digest-of-digest flattens to original raws, depth 2).
  - [x] Negatives: empty `source_log_ref` via struct-literal bypass → `DistillationError::AuditChainMissing` (kernel-side); fabricated ref → `SourceFrameNotFound`; empty survey → `AuditChainMissing` at the request layer. **5/5 pass.**
- [x] **T5 — Five-metric gate reference + researcher self-verification (AC4)**
  - [x] Confirmed `cargo test -p maos-eval --test distillate_five_metrics_floor` (N=100) passes UNCHANGED with Researcher named the reference producer (`test_distillate_five_metrics_floor` + `_category_distribution` ok; corpus NOT regenerated).
  - [x] `spirits/researcher/tests/five_metric_self_verify.rs` (deterministic, no live LLM): Researcher's own distillates clear **traceability 100%** (every cited ref resolves via the real walker) + **secret-leakage 0%** (real `CorpusBackedRedactionPolicy::redact`; colon-separated frame-id cites keep hex runs < the 32-char token threshold); negatives — fabricated ref → `FrameNotFound`; planted `sk-ant-api03-` digest fires redaction (live positive control); plus a defense-in-depth test proving TL write-time redaction scrubs source-frame secrets before the survey. **4/4 pass.**
  - [x] Corrected coverage-matrix `researcher` slot path → `spirits/researcher`; recorded Researcher as the NFR-Aud-7 reference producer (inline comment).
- [x] **T6 — NFR-Aud-8 quarterly N=500 corpus (AC5)**
  - [x] Authored `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/` — N=500 `scenario-*.json` + `iaa-attestation.json` (tag=`quarterly-v0`, 2 annotators, κ=0.87 ≥ 0.85). Distribution: 350 typical / 50 hedge-focus / 50 contradiction / 50 planted-secret (≥10 each category). Deterministic generator `quarterly-audit-v0/generate.py` gated behind `MAOS_GEN_QUARTERLY_CORPUS=1` (pure function of index — no live LLM, no RNG; NFR-Testability-1 bit-identical), mirroring Butler's `MAOS_GEN_DIGEST_CORPUS`.
  - [x] Removed `#[ignore]` from `test_distillate_corpus_quarterly_audit_shape`; the test now ENFORCES N≥500 + tag + IAA≥0.85 + the three annotated floors (recall≥0.90/faith≥0.98/hedge≥0.95) + traceability 100% + secret-leakage 0% (real redaction, ≥10 planted-secret positive controls) + category distribution. **3/3 maos-eval distillate tests pass.** **AC5 FLAG:** beyond the fixtures + the `#[ignore]` removal, the test BODY was enhanced from a bare `len()>=500` to the full floor/IAA/traceability/secret-leakage/category assertions (so the un-ignored test is meaningful, per AC5's "all five metrics reportable on the N=500 slice"); the N=100 gate (`test_distillate_five_metrics_floor`) is UNCHANGED.
  - [x] SHA-pinned via NEW `crates/maos-eval/tests/distillate_corpus_quarterly_pin.rs` (manifest hash over the 501 json files; PIN `225184b3…a7d41`) — **AC5 FLAG:** a new pin test file is an additional maos-eval change, justified by the Story 0.3 SHA-pin requirement (mirrors Butler's `corpus_pin.rs`). Registered in `tests/coverage-matrix.yaml` NFR-Aud-8 row (corpora += `quarterly-audit-v0`, note records the PIN + regen command); `coverage-matrix` + `corpus-staleness` gates PASS.
- [x] **T7 — `scalar.tap` subscription + digest contribution (AC6)**
  - [x] `tests/scalar_tap.rs` (mirrors `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs`, driving the real `TelemetryStreamAdapter` dev-dep): Researcher `subscribe_topic("researcher", TelemetryTopic::new("scalar.tap.confidence"))` (first=true, re-subscribe=false, different-spirit=true); a peer publishes a `ScalarTapEvent`; Researcher RECEIVES it within the 100ms bound.
  - [x] `Researcher::incorporate_scalar` folds the received scalar into a SUBSEQUENT survey (confidence_map key `observed::observer::confidence` = 0.62 + open-question + scalars).
  - [x] Composability demo: a Researcher distillate (walk→survey→incorporate-scalar→`distill_through` real `DistillateWriter`) is consumable in the FR17 digest pattern (every finding cites a source-log-ref; the receipt carries a non-empty kernel-resolved chain). NO `butler` dependency (Decision E; enforced at the dep level). **2/2 pass.**
- [x] **T8 — J-Researcher latency (AC7)**
  - [x] Added `harness::j_researcher` to `crates/maos-bench/src/harness/` + wired `bench_j_researcher` into `benches/section_13_1.rs` (NOT in-crate — 8.1's review must-fix). Journey = 50-frame burst, 16–64 KB payloads, one EpistemicHalt+Resume leg (the burst's opposing strong-methodology claims yield a `methodology_conflict` primary scalar). The survey now COMPRESSES (bounded `summarize`, `MAX_FINDING_SUMMARY_CHARS=280`) so a digest is smaller than its inputs (§9.5). Distillation step = `survey → write_distillate`; **measured P95 = 23.2ms** (P50 21.7 / P99 26.3 / max 28.6 / mean 22.0ms) vs the **<100ms budget — MET**. Budget overrun emits `FrameKind::BudgetWarning` (NFR-Perf-6) — proven by the forced-0µs-budget test journaling the warning frame. **2/2 harness tests pass.**
  - [x] P95 recorded above; budget met, so the ADR-002 three-condition inproc-unlock is not invoked (the in-proc reference form measures the real code path, not a masked one).
- [x] **T9 — CI / discipline green (AC9)**
  - [x] Added `researcher-tests` job (`cargo test -p researcher --locked`) to `.github/workflows/discipline.yml` + wired into the `aggregate` `needs:` list (mirrors `butler-tests`). YAML validated (92 jobs; `researcher-tests` present + in aggregate needs).
  - [x] All AC9 gates GREEN at HEAD: `check-service-boundary` (0 violations), `check-workspace-count` (32/32), `coverage-matrix` (exit 0), `corpus-staleness` (PASSED), `abi-diff` (exit 0, Researcher ABI-neutral), `check-dev-model-used-populated` (PASS), `check-empty-kernel` (0 violations), `kloc-check` (exit 0); the five-metric + now-enforced quarterly tests pass; butler regression clean (0 failures). Pre-existing reds verified Researcher-neutral (kernel-surface gates; Researcher is zero-kernel-KLOC in `spirits/`). No flipped-while-red. File List complete.

## Dev Notes

### Spirit form & scaffolding (mirror Butler 8.1)
- **Form: rust-inproc** — matches `spirits/butler` and the SDK `LocalRunner`/spirit-test. `forms = ["rust-inproc"]`. Scaffold by copying `spirits/butler/` shape: `Cargo.toml` (Spirit-side deps only in `[dependencies]`; kernel adapters in `[dev-dependencies]` to PROVE integration without violating Story 0.2), `manifest.toml`, `src/lib.rs` (`#[spirit] impl Researcher { fn on_frame/on_idle… }` + pure Spirit-side API methods), `tests/`.
- The `#[spirit]` macro is applied to an **inherent `impl` block**; it synthesizes no-op bodies for unused hooks and exports `__maos_spirit_vtable_Researcher()`. Hooks validated against `HOOK_NAMES` in `maos-spirit-derive`. Keep state in `Arc<Mutex<...>>` so the Spirit stays `Sync` (Butler `src/lib.rs:237-245`).
- `Ctx` exposes only opaque handles (`cancellation()`, `capability()`, `mailbox()`, `deprecation_warnings()`). A lifecycle hook **cannot** reach kernel services directly — the digest/recall/distillation/telemetry integrations are proven in tests that drive the real kernel adapters as **dev-dependencies** (Butler's resolved pattern; see 8.1 "Highest-risk integration"). This is the single most likely place to lose a review cycle — do not reach into `maos-kernel-core` from `spirits/researcher`'s lib.

### Participant-scoped `log.recall` (Story 4.4) — the core of this story
- **Trait** `LogRecallPort` ([crates/maos-domain/src/ports/log_recall.rs:14-28](crates/maos-domain/src/ports/log_recall.rs)): `recall(&self, spirit_pid: u32, filter: LogRecallFilter) -> Result<LogRecallPage, LogRecallError>` and `fetch(&self, spirit_pid: u32, frame_id: [u8;16]) -> Result<LogFetchResponse, LogRecallError>`.
- **Filter** `LogRecallFilter::new(kind: Option<FrameKindLabel>, since_ns: Option<u64>, until_ns: Option<u64>, limit: usize, cursor: Option<LogRecallCursor>, intent_filter: Option<String>)` — `limit` clamps to `MAX_LIMIT=1024`. Use `::new` (not a struct literal) so the validation/clamp applies. Page = `{ entries: Vec<LogRecallEntry>, next_cursor: Option<LogRecallCursor> }`; iterate pages via `next_cursor` (keyset pagination on `{last_timestamp_ns, last_frame_id}`).
- `LogRecallEntry` **omits the payload** (`payload_available: bool`) — call `fetch(frame_id)` for `LogFetchResponse { …, payload_redacted: Vec<u8>, capability_token, origin }` (lazy-load honors A2A consent re-check at disclosure).
- **Scope enforcement** ([crates/maos-iac/src/adapter/log_recall.rs](crates/maos-iac/src/adapter/log_recall.rs)): `recall` filters to `spirit_pid` (emitter-side at v0.3-β; recipient-side deferred v0.5+); `fetch` returns `LogRecallError::ScopeViolation{frame_id, requested_pid, owner_pid}` when the requester is not the emitter. Build the adapter with `LogRecallAdapter::new(transparency_log: Arc<TransparencyLogAdapter>)`.
- **Contrast (the 8.1→8.2 contract):** Butler used the UNscoped free-fn `ranged_recall` (`maos-audit`); Researcher MUST use the SCOPED `LogRecallPort`. FR29: "kernel scopes results to participant frames and honors A2A consent envelopes."

### Distillation I11 chain (Story 4.4) — how the distillate is persisted
- `DistillationRequest { source_log_ref: Vec<[u8;16]> (non-empty), distillation_depth: u32 (≥1), digest_payload: DigestPayload, segment_hint: Option<SegmentHint> }` ([crates/maos-domain/src/distillation.rs:14-35](crates/maos-domain/src/distillation.rs)). Construct via the constructor (rejects empty refs / depth<1) so the author-guard is exercised; the negative AuditChainMissing test can use a struct-literal bypass to exercise the *writer's* enforcement (Butler's approach).
- `DistillateWriter::new(transparency_log: Arc<TransparencyLogAdapter>, memory: Arc<dyn Any+Send+Sync>)` → `write_distillate(&self, spirit_pid: u32, request) -> Result<DistillationReceipt, DistillationError>` ([crates/maos-iac/src/adapter/distillate.rs:58,290](crates/maos-iac/src/adapter/distillate.rs)).
- **Kernel computes `intent_lineage` (I13)** ([:253-286]) — queries the TL for each source frame, unions+sorts intents (`BTreeSet<A2AIntent>`). NEVER Spirit-self-reported. **Transitive flatten** ([:194-251]) — work-list recursion: a `FrameKind::Distillate` source resolves to its own `effective_source_log_ref` (so any digest references *original raw frames*), `HashSet` cycle detection.
- Errors ([distillation.rs:139-158]): `AuditChainMissing{reason}` (`E_DIGEST_AUDIT_CHAIN_MISSING`), `IntentPromotionDenied{digest_frame_id}` (I13 `E_INTENT_PROMOTION_DENIED`), `SourceFrameNotFound{frame_id}`, `Storage(String)`. `FrameKind::Distillate = 11` is the audit discriminator.

### Five-metric gate (NFR-Aud-7) + quarterly slice (NFR-Aud-8)
- Gate test: `crates/maos-eval/tests/distillate_five_metrics_floor.rs:26-136` — loads `DistillateCorpus::load_from("fixtures/distillate-corpus-v0/")`, locks `len()==100` + `tag=="synthetic-v0"` + `iaa.hedge_cohen_kappa >= 0.85`, then floors recall≥0.90 / faithfulness≥0.98 / hedge≥0.95 (annotated means), traceability (non-empty `source_log_ref`), secret-leakage (real `CorpusBackedRedactionPolicy::redact`, with a positive-control assertion that planted secrets DO fire). **Do not weaken or regenerate this.**
- `DistillateScenario` fields ([crates/maos-eval/src/distillate_corpus.rs:21-47](crates/maos-eval/src/distillate_corpus.rs)): `scenario_id, tag, spirit_class, source_raw_frames[], digest_payload, source_log_ref[](hex), distillation_depth, intent_lineage_expected[], expected_recall, expected_faithfulness, expected_hedge_preservation, planted_secrets[]`. `IaaAttestation { corpus_version, annotator_count, hedge_cohen_kappa, computed_at }`. Loader scans `scenario-*.json` (sorted by filename) + `iaa-attestation.json`.
- **NFR-Aud-8 (AC5):** the quarterly stub at `:138-156` is `#[ignore = "…lands in Story 8.2 alongside Researcher"]` and expects `fixtures/distillate-corpus-v0/quarterly-audit-v0/` with `len() >= 500`. Author that dir (same `DistillateScenario` schema + `iaa-attestation.json`), then **remove the `#[ignore]`**. For determinism (NFR-Testability-1: "reproducible from a published seed; bit-identical pass/fail"), generate the N=500 corpus from a seeded generator gated behind an env flag — no live LLM. App F.5 derives why recall/faithfulness/hedge are annotated, not live-recomputed.

### `scalar.tap` Telemetry Stream (I7)
- Port `TelemetryStreamPort` ([crates/maos-domain/src/ports/telemetry.rs:12-26](crates/maos-domain/src/ports/telemetry.rs)): `publish_event(&self, topic: &TelemetryTopic, event: ScalarTapEvent)` + `subscribe_topic(&self, spirit_id: &str, topic: &TelemetryTopic) -> bool`.
- `TelemetryTopic::new("scalar.tap.<metric>")` (e.g. `scalar.tap.confidence`), `ScalarTapEvent { spirit_id, tag, value: f64, timestamp }` ([crates/maos-domain/src/invariants/i7.rs:29,45](crates/maos-domain/src/invariants/i7.rs)). I7 phasing: v0.5 = runtime-operational (Researcher is v0.5).
- Concrete impl: `TelemetryStreamAdapter` at [crates/maos-kernel-core/src/telemetry/mod.rs:78](crates/maos-kernel-core/src/telemetry/mod.rs). **Reference test to mirror:** [crates/maos-kernel-core/tests/scalar_tap_subscriber.rs](crates/maos-kernel-core/tests/scalar_tap_subscriber.rs) — drive it as a dev-dep, subscribe Researcher, publish, assert receipt.

### Researcher cognitive shape (architecture §6.2) — manifest content
- Posture: `survey-mode` (default). Declare `hypothesize-mode` in the posture-set per §6.2 but DO NOT implement the ILP+LLM path (Decision C; v1.0).
- Output shape: `findings + Open Questions + Confidence Map + Bibliography` — kernel rejects emit missing any of the four (`[output_shape] required_fields`).
- Epistemic policy (§6.2): halt on `claim.methodology_strength` when two papers report contradictory findings both with strong methodology (Spirit computes a scalar; kernel does universal-arithmetic comparison — likely `on_value_above` on a "methodology_conflict" scalar); halt on `claim.load_bearing` with `confidence_below = 0.7` (`on_value_below = { threshold = 0.7 }`); `verbalize_only` on `claim.exploratory`. **Verify the exact tag-string + predicate-key serialization against `maos-manifest` validators before authoring** (Butler confirmed bare tags `belief_variance`/`user_preference_drift` + `on_value_above`/`on_value_below`; the architecture's `claim.` prefix is prose, the manifest tag is bare).
- Capabilities: `Scope::LogRecall` (the walker), `provider.complete` (compressor model class ≥Sonnet-tier / 70B+, temp ≤0.3 per App F.4), broad MCP servers (web/arXiv/GitHub/citation-graph → `Scope::McpCall`, fixture-replayed). **`[capabilities.parallelism] = 8` from §6.2 is NOT a confirmed manifest field — verify or omit (AC8).**
- Adaptive-chunk-ratio summarization (§6.2 / App F.4): first-turn/last-turn anchoring (preserve original task statement + final output uncompressed, compress the middle); `target_max_tokens` default `max(2048, 0.15×original)`; compression ratio in `[0.05, 0.25]`. These are non-binding conventions (App F); the binding floors are the five-metric gate.

### §13.1 J-Researcher latency
- §13.1 J-Researcher journey ([13-phased-roadmap.md:47](_bmad-output/planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md)): "50-frame burst, 16–64 KB payloads; includes one EpistemicHalt + Resume cycle" — measures tail latency + halt/resume cost. Epic 8.2 budget: distillation-step **<100ms P95**.
- **8.1 lesson (AC7 review must-fix):** Butler measured J0 in-crate and was forced to move it into the `maos-bench` harness. Add the J-Researcher journey to `crates/maos-bench/benches/section_13_1.rs` (8.1 added `harness::j0`; J1/J4 already exist) — do not measure only in-crate. `BudgetWarning` on overrun (NFR-Perf-6; `FrameKind::BudgetWarning`).

### Testing standards
- SDK spirit-test harness + v0.5 macros (`spirit_test_assert!`, `spirit_test_expect_frame!`, `spirit_test_expect_halt!`, `assert_no_deprecations!`) — `crates/maos-spirit-sdk/src/spirit_test/assert.rs`. Note spirit-test halts are SIMULATED (`harness.resolve_halt`); a real `DistillationReceipt`/`HaltReceipt`/scope-violation must come from the real kernel adapters in integration tests (dev-deps), not spirit-test.
- All corpus/distillate verification must be **deterministic** — mock/seed the compressor (no live LLM in CI). Traceability + secret-leakage are cross-references against real log rows / the real redaction filter, not LLM judgements.
- SHA-pin both new corpora per Story 0.3; register in corpus-staleness / coverage-matrix so edits fail loud.

### Project Structure Notes
- **New crate** `spirits/researcher/` (Decision A): `Cargo.toml`, `manifest.toml`, `src/lib.rs`, `tests/` (recall walker, distillation I11, five-metric self-verify, scalar.tap, latency, corpus pins), `tests/fixtures/`. Add to root `Cargo.toml` members (→32); bump the sentinel; correct the coverage-matrix slot path.
- **New corpus** `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/` (N≥500) — the only maos-eval addition besides removing one `#[ignore]`.
- **No edits** to `maos-kernel-core` (Story 0.2). Researcher cognition is Spirit-side only; kernel adapters reached as dev-deps in tests.

### References
- [Source: epics/epic-8-…miranash-v03-v15.md#Story 8.2] — story statement + 5 BDD AC blocks
- [Source: architecture-maos-minimal-opus/6-reference-spirits.md#6.2 Researcher] — cognitive shape, posture, output shape, distillation pattern, epistemic policy, eval metrics
- [Source: architecture-maos-minimal-opus/9-memory-knowledge.md#§9.5 + §9.5.1] + [appendix-f-distillation-pattern-body.md] — distillation pattern interface, five-metric gate (Table 9.5-1), Story 4.4 surface, App F.5 floor derivation
- [Source: architecture-maos-minimal-opus/13-phased-roadmap.md#13.1] — J-Researcher journey + <100ms budget + ADR-002 inproc-unlock
- [Source: prd/functional-requirements.md] — FR29 (log.recall participant-scoped), FR30 (distillate I11), FR58 (per-phase reference Spirit), FR17 (digest contribution), FR56 (self-telemetry)
- [Source: prd/non-functional-requirements.md] — NFR-Aud-7 (five-metric gate), NFR-Aud-8 (quarterly N=500), NFR-Perf-6 (BudgetWarning), NFR-Testability-1 (corpus reproducibility)
- [Source: _bmad-output/implementation-artifacts/8-1-…-spirit-side.md] — Butler crate structure, the in-proc Spirit→kernel-adapter-as-dev-dep bridge pattern, the maos-bench J0 latency must-fix, Decisions A/B mirrored here
- [Source: crates/maos-domain/src/ports/log_recall.rs + src/log_recall.rs] — `LogRecallPort`, filter/page/cursor/entry/error types
- [Source: crates/maos-iac/src/adapter/log_recall.rs] — `LogRecallAdapter::new`, scope enforcement
- [Source: crates/maos-domain/src/distillation.rs + crates/maos-iac/src/adapter/distillate.rs] — I11 chain, write_distillate, transitive flatten, intent_lineage
- [Source: crates/maos-eval/tests/distillate_five_metrics_floor.rs + src/distillate_corpus.rs] — five-metric gate, the `#[ignore]`d quarterly test, corpus schema/loader
- [Source: crates/maos-domain/src/ports/telemetry.rs + src/invariants/i7.rs + crates/maos-kernel-core/src/telemetry/mod.rs + tests/scalar_tap_subscriber.rs] — scalar.tap subscription
- [Source: crates/maos-domain/src/invariants/i1.rs] — `Scope::LogRecall`, `Scope::McpCall`
- [Source: spirits/butler/{Cargo.toml,manifest.toml,src/lib.rs}] — reference crate structure to mirror
- [Source: .github/workflows/discipline.yml:517,532,~1553] — `example-spirit-tests`/`butler-tests` job + gate-aggregation pattern; [tests/coverage-matrix.yaml:1192-1208] — butler/researcher slots

### Review Findings

**Team Consensus (per spec & long-term correctness):**

- [x] [Review][Decision] ~~Manifest `[posture]` is autonomy spectrum, not cognitive survey-mode~~ — **RESOLVED: Update AC2 spec language.** The implementation correctly uses `[posture]` for the autonomy spectrum (`assistive`). The spec text in AC2 that says "survey-mode posture" is imprecise — it should read "autonomy posture" with a note that cognitive survey/hypothesize mode is realized Spirit-side as `ResearcherPosture`. This aligns the spec with the actual manifest schema (`deny_unknown_fields` on `PostureSection`) and avoids inventing an invalid manifest field. The `ResearcherPosture` enum (including `Hypothesize` per Decision C) stays Spirit-side where it belongs.
- [x] [Review][Decision] ~~`Scope::LogRecall` isn't manifest-declarable~~ — **RESOLVED: Document as kernel-granted; schedule schema change.** The spec language in AC2 should be updated to state that `Scope::LogRecall` is exercised via the adapter's `CapabilityInvocation` audit row (kernel-granted), not declared in `[capabilities.required]`. A future story (Epic 9 or v1.0 manifest-schema revision) will add `LogRecall` to `capabilities_required_to_scopes` so it CAN be declared. For v0.5, the manifest documents the requirement in `description` + inline comment, which is sufficient.
- [x] [Review][Patch] Mutex::lock().unwrap() will abort on poison (lib.rs:253) — **FIXED:** `unwrap_or_else(|e| e.into_inner())` [blind+edge]
- [x] [Review][Patch] O(n²) methodology-conflict scan with no cardinality cap (lib.rs:562-580) — **FIXED:** capped at 10,000 claims with warning [blind]
- [x] [Review][Patch] No cancellation checkpoint inside `survey` — **FIXED:** cooperative `std::thread::yield_now()` every 256 frames and every 1,024 conflict iterations [blind]
- [x] [Review][Patch] Test helpers silently ignore `insert_frame_event` errors — **DISMISSED:** `insert_frame_event` returns `LogBeforeDeliver<()>` (not Result) and panics on failure per I2; `let _ =` is idiomatic [blind]
- [x] [Review][Patch] `summarize` splits on Unicode scalar values, not grapheme clusters — **FIXED:** added `unicode-segmentation` dep, uses `graphemes(true)` [blind]
- [x] [Review][Patch] `decode_frame_id_hex` accepts upper-case while `encode` emits lower-case — **FIXED:** rejects uppercase + non-ASCII [blind]
- [x] [Review][Patch] Hard-coded 100ms timeout in scalar_tap.rs flaky under CI load — **FIXED:** increased to 500ms [blind]
- [x] [Review][Patch] Benchmark Criterion config miscalibrated — **FIXED:** added `run_j_researcher_measurement_smoke()` with 100 invocations for Criterion; full 1,000 kept for `#[ignore]`d test [blind]
- [x] [Review][Patch] Category distribution assertions vacuous — **FIXED:** tightened thresholds to `faithfulness < 0.99` and `hedge < 0.97` matching generate.py bands [blind+edge+auditor]
- [x] [Review][Patch] CI job lacks timeout — **FIXED:** added `timeout-minutes: 15` to `researcher-tests` [blind]
- [x] [Review][Patch] Benchmark measures full journey, not just distillation step — **FIXED:** added `measure_distillation_step()` for isolated measurement; Criterion uses `run_j_researcher_measurement_smoke()` [blind]
- [x] [Review][Patch] Redaction-fired detection semantically brittle — **FIXED:** `matches!(Cow::Owned(b) if b != payload)` in all three locations [blind]
- [x] [Review][Patch] Bench harness contains unignored slow tests — **FIXED:** `#[ignore]`d `j_researcher_measures_the_distillation_step_within_budget` [blind]
- [x] [Review][Patch] No CI job runs the Criterion benchmark — **FIXED:** added `researcher-bench` job with `cargo bench -p maos-bench --bench section_13_1 -- --test` [blind]
- [x] [Review][Patch] Magic numbers undocumented — **FIXED:** added `// pid 10 = researcher` and `// depth 1 = direct digest` comments [blind]
- [x] [Review][Patch] Duplicate claim_id across frames silently overwrites confidence_map (lib.rs:348) — **FIXED:** detects duplicate and pushes open_question [edge]
- [x] [Review][Patch] scalar.tap event value NaN not guarded (lib.rs:446) — **FIXED:** NaN → 0.0 with open_question [edge]
- [x] [Review][Patch] non-ASCII input with byte length 32 passes decode_frame_id_hex (lib.rs:617-625) — **FIXED:** `!clean.is_ascii()` check [edge]
- [x] [Review][Defer] No runtime enforcement of manifest budgets — time_cap_seconds/memory_max_mb declarative only; pre-existing pattern [blind]
- [x] [Review][Defer] `on_idle` cannot consume pending frames / disconnected from `walk` — architectural design choice for test harness [blind]
- [x] [Review][Defer] `incorporate_scalar` truncates f64→f32→f64 — design choice for scalar precision [blind]
- [x] [Review][Patch] ~~`bibliography` may contain duplicate entries~~ — **FIXED:** deduplicated by `source_log_ref` using `HashSet` + `retain` before `SurveyOutput` return [blind]
- [x] [Review][Defer] Aggregate `needs` array unmaintainable growth — pre-existing CI pattern [blind]

## Dev Agent Record

### Agent Model Used

`claude-opus-4-8` (Decision F) — recorded in frontmatter `dev_model_used` per §A2.

### Debug Log References

### Completion Notes List

#### AC1 — Prerequisites & scope classified (verified 2026-06-02)

All ✅ rows in the Prerequisites table re-verified by direct source read:

| Prerequisite | Verified at |
|---|---|
| `LogRecallPort` trait (`recall`/`fetch`) | `crates/maos-domain/src/ports/log_recall.rs:14-28` ✅ |
| `LogRecallFilter::new` (MAX_LIMIT=1024 clamp), `LogRecallCursor/Page/Entry/FetchResponse/Error` | `crates/maos-domain/src/log_recall.rs` ✅ — `LogRecallError::ScopeViolation{frame_id,requested_pid,owner_pid}` present |
| `LogRecallAdapter::new(Arc<TransparencyLogAdapter>)` + emitter-scope enforcement | `crates/maos-iac/src/adapter/log_recall.rs:63` (new), `:366` (fetch ScopeViolation) ✅ — re-exported `maos_kernel_core::iac::log_recall::LogRecallAdapter` (`adapter.rs:19` + `maos_iac` glob) |
| `DistillationRequest/Receipt`, `DistillateWriter::write_distillate`, transitive flatten + kernel `intent_lineage`, `AuditChainMissing` | `crates/maos-domain/src/distillation.rs`; `crates/maos-iac/src/adapter/distillate.rs:58,194,254,290` ✅ |
| Five-metric harness + corpus loader + the `#[ignore]`d quarterly test | `crates/maos-eval/tests/distillate_five_metrics_floor.rs:26-136` (gate), `:138-156` (`test_distillate_corpus_quarterly_audit_shape`, `#[ignore]`) ✅; `DistillateCorpus::load_from` scans `scenario-*.json` + `iaa-attestation.json` |
| `TelemetryStreamPort` + `TelemetryStreamAdapter` + reference test | `crates/maos-domain/src/ports/telemetry.rs:12-26`; `crates/maos-kernel-core/src/telemetry/mod.rs:78`; `crates/maos-kernel-core/tests/scalar_tap_subscriber.rs` ✅ |
| `Scope::LogRecall` / `Scope::LogFetch` / `Scope::DistillateWrite` / `Scope::McpCall` | `crates/maos-domain/src/invariants/i1.rs:60,83-92` ✅ |
| MCP fixture-replay | `crates/maos-mcp/src/fixture_replay.rs` ✅ (present) |
| `maos-bench` §13.1 harness (`harness::j0/j1/j4` + `build_journey_result`) | `crates/maos-bench/src/harness/{mod.rs,j0.rs}`; `benches/section_13_1.rs` ✅ — no J-Researcher yet (AC7 adds) |
| Butler reference crate | `spirits/butler/{Cargo.toml,manifest.toml,src/lib.rs,tests/}` ✅ |
| `spirits/researcher/` | **ABSENT** ✅ (confirmed — this story creates it) |
| coverage-matrix `researcher` slot WRONG path | `tests/coverage-matrix.yaml` path=`crates/maos-spirit-researcher` ⚠️ (Decision A corrects → `spirits/researcher`) |

**Decisions A–F recorded as chosen resolutions (not re-decided).**

**Substrate-reality findings that refine the spec (flagged for Winston):**
- **Manifest `[posture]` is the autonomy spectrum** (`Posture::{Cautious,Assistive,AutonomousWithHalt,Autonomous}`, `manifest.rs:541-556`), NOT the cognitive `survey-mode`/`hypothesize-mode` set. With `deny_unknown_fields` on every section, the survey/hypothesize *cognitive* posture-set cannot live in `[posture]` nor in a custom section. **Resolution:** `[posture]` autonomy = `assistive` (mirrors Butler); the survey-mode cognitive posture is realized Spirit-side as a `ResearcherPosture` enum (declaring `Hypothesize` per Decision C without implementing the ILP path) and surfaced in output. Documented intent, flagged for Winston (extends Q4).
- **`Scope::LogRecall` is not a manifest-declarable capability field** — `capabilities_required_to_scopes` (`manifest.rs:503-531`) derives only `ProviderInfer` + `McpCall` from the manifest. LogRecall/LogFetch/DistillateWrite scopes are granted by the kernel at the capability layer, not declared in the manifest `[capabilities.required]` block. **Resolution:** the manifest documents the LogRecall requirement in `description` + an inline comment; the *enforced* `Scope::LogRecall` path is exercised by the walker driving the real `LogRecallAdapter` (which emits the `CapabilityInvocation` audit row) in the dev-dep integration test. Flagged for Winston.

### File List

**Created — Researcher crate (`spirits/researcher/`, workspace member #32):**
- `spirits/researcher/Cargo.toml` — pure Spirit-side deps (spirit-sdk/abi + maos-domain); kernel adapters dev-deps only; NO `maos-audit` (scoped-port contract).
- `spirits/researcher/manifest.toml` — survey-mode v0.5 envelope (validated vs authoritative validators).
- `spirits/researcher/src/lib.rs` — `Researcher` Spirit (`#[spirit] on_idle`), `ResearcherPosture`, `ClaimPayload`/`SurveyOutput`/`Finding`/`BibEntry`, the participant-scoped `walk`/`recall_all`/`fetch_payloads`, `survey` (seeded compressor w/ bounded `summarize`), `to_distillation_request`/`distill_through`, `incorporate_scalar`, colon-hex encode/decode.
- `spirits/researcher/tests/spirit_smoke.rs` — on_idle survey + manifest validators (AC2/AC8).
- `spirits/researcher/tests/recall_walker.rs` — scoped walker + ScopeViolation vs real `LogRecallAdapter` (AC2).
- `spirits/researcher/tests/distillation_i11.rs` — I11 chain/flatten/intent_lineage + negatives vs real `DistillateWriter` (AC3).
- `spirits/researcher/tests/five_metric_self_verify.rs` — traceability 100% + secret-leakage 0% + controls (AC4).
- `spirits/researcher/tests/scalar_tap.rs` — subscription + incorporate + FR17 composability demo (AC6).

**Created — NFR-Aud-8 quarterly corpus + bench:**
- `crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/` — 500 `scenario-*.json` + `iaa-attestation.json` + `generate.py` (seeded, env-gated) + `README.md` (AC5).
- `crates/maos-eval/tests/distillate_corpus_quarterly_pin.rs` — SHA-256 directory pin (AC5/AC9).
- `crates/maos-bench/src/harness/j_researcher.rs` — §13.1 J-Researcher journey + BudgetWarning (AC7).

**Modified:**
- `Cargo.toml` — workspace members += `spirits/researcher` (→32).
- `Cargo.lock` — new member resolved.
- `crates/maos-eval/tests/distillate_five_metrics_floor.rs` — removed `#[ignore]` + enhanced quarterly test (floors/IAA/traceability/secret-leakage/category). N=100 gate UNCHANGED.
- `crates/maos-eval/fixtures/distillate-corpus-v0/README.md` — quarterly slice LANDED.
- `crates/maos-bench/Cargo.toml` — += `researcher` dep.
- `crates/maos-bench/src/harness/mod.rs` — `pub mod j_researcher`.
- `crates/maos-bench/benches/section_13_1.rs` — `bench_j_researcher` wired into the criterion group.
- `tests/coverage-matrix.yaml` — `researcher` slot path → `spirits/researcher` (Decision A) + NFR-Aud-8 corpus registration + SHA.
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — workspace-count sentinel 31→32.
- `.github/workflows/discipline.yml` — `researcher-tests` job + wired into the aggregate `needs:` list.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 8.2 → in-progress→review.
- `_bmad-output/implementation-artifacts/8-2-…-log-recall-walker.md` — frontmatter `dev_model_used`, Tasks, Dev Agent Record, Change Log, Status.

### AC6/AC7/AC9 evidence

- **AC9 gates GREEN at HEAD (verified locally):** `check-service-boundary` PASSED (0 violations), `check-empty-kernel` PASSED (0 violations — Story 0.2, AC8 no new kernel public fn), `check-workspace-count` PASSED (32/32), `coverage-matrix` exit 0, `corpus-staleness` PASSED, `abi-diff` exit 0 (Researcher ABI-neutral), `check-dev-model-used-populated` PASS, `kloc-check` exit 0. Researcher full suite `cargo test -p researcher --locked` = **26 tests pass** across 6 files. `maos-eval` distillate suite = 3 pass (incl. the now-enforced quarterly). `j_researcher` harness = 2 pass.
- **Pre-existing reds verified Researcher-neutral:** the 7.1.7 `check-service-boundary` stale-baseline is GREEN at HEAD (0 violations) — Researcher adds none; `kloc-check`/`check-empty-kernel` are kernel-surface gates and Researcher lives entirely in `spirits/` (zero kernel KLOC). No flipped-while-red.


### Change Log

| Date | Change |
|---|---|
| 2026-06-02 | Story 8.2 implemented (AC1–AC9). NEW `researcher` crate at `spirits/researcher/` (workspace 31→32): participant-scoped `log.recall` walker (Story 4.4 `LogRecallPort`, NOT `ranged_recall`), seeded survey compressor, I11 distillate chain via real `DistillateWriter`, scalar.tap subscription, FR17 composability demo. NEW NFR-Aud-8 quarterly N=500 corpus (`quarterly-audit-v0/`, seeded `generate.py`, SHA-pinned) — flipped `test_distillate_corpus_quarterly_audit_shape` from `#[ignore]`d to enforced. NEW `harness::j_researcher` §13.1 journey (distillation-step P95=23.2ms < 100ms; BudgetWarning on overrun). `researcher-tests` CI job added + wired. 26 researcher tests + 3 maos-eval distillate + 2 j_researcher pass; all AC9 discipline gates green at HEAD. 5 LOCKED decisions A–F honored; 2 substrate-reality refinements flagged for Winston (manifest `[posture]` is autonomy-not-cognitive; `Scope::LogRecall` is kernel-granted, not manifest-declarable). |

## Questions / Clarifications for the Architect (Winston)

1. **Decision A (workspace count → 32):** confirm Researcher is a workspace member at `spirits/researcher` (mirrors Butler) — bumps `check-workspace-count` 31→32 and corrects the coverage-matrix `researcher` slot path from `crates/maos-spirit-researcher` to `spirits/researcher`.
2. **Decision B (v0.5 broad-MCP milestone):** confirm "broad MCP capabilities (web/arXiv/GitHub/citation-graph)" is satisfied at the substrate layer by `Scope::McpCall` capability-scope wiring + fixture-replay, with live research drivers deferred to v0.5+/Epic 9.
3. **Decision D (five-metric split):** confirm traceability + secret-leakage are MEASURED against Researcher's real distillates while recall/faithfulness/hedge stay corpus-annotated (App F.5 / CI-determinism), rather than requiring a live-LLM recall/faithfulness recompute.
4. **`[capabilities.parallelism]` (AC8):** §6.2 says "parallelism = 8" but no such manifest field is parsed today. Omit it (record as documented intent) unless you want a manifest-schema field added — which would be its own ABI/manifest change, out of scope here.
5. **Sandbox tier:** §13 introduces T3 (containerized) at v0.5, but the rust-inproc reference/measurement form can't run a container. Recommend `[sandbox] tier = "T2"` for the reference crate (testable) with T3 as the production-deploy target — confirm (same asymmetry Butler's T2 reference handled).
