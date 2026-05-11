# Epic 4: Halt Protocol + Memory Substrate + Cognition Primitives (v0.3 → v1.0) — **SINGLE HALT OWNER**

**Goal:** The kernel performs ONLY universal-arithmetic comparisons. Spirit author declares cognitive policies via four predicates over tagged scalars; kernel triggers halts when predicates fire; every distillation is auditable end-to-end via I11+I12+I13. Cross-Spirit memory isolation is mechanically provable. **Halt protocol — schema types in E1a, mechanism + I14 invariant + halt-receipt + recall/precision floors OWNED HERE.**

**Owns:**
- Halt protocol mechanism (ADR-019 + ADR-022): three resolution kinds (`provided_context`, `accepted_halt`, `authorized_override`); `epistemic.halt(payload)` invocation; halt-receipt production rate ≥99.9% on every Spirit termination planned or unplanned.
- Tagged-scalar slot in Capability Registry (ADR-022): `working_memory.set_scalar(tag, value, derived_from)`.
- Four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`) — kernel performs NO Spirit-specific cognitive computation (no variance, entropy, EFE, KL, ensemble disagreement, derivatives, statistical tests — Spirit computes those itself per §4.0.7).
- Three memory tiers (Memory Manager, I5-enforced): `private` (per-Spirit `Arc<RwLock<HashMap>>` + per-Spirit-namespaced filesystem); `shared` (Host-wide SQLite-backed kv with namespace prefix per writer); `collective` scaffold (full Loom-lite Postgres+pgvector implementation in E10 at v1.5).
- Principal Memory Namespace ADR-026 full implementation: `principal:<principal_id>:<schema>` typed namespace within private tier with subject-access query, right-to-be-forgotten, redaction-on-export.
- `log.recall(filter, limit, cursor)` + `log.fetch(frame_id)` (ADR-013) — participant-scoped with A2A consent envelope honoring.
- Distillates with kernel-enforced I11 audit chain: mandatory `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage`. Distillation work itself (selection, summarization) is Spirit-authored.
- Per-tag `[epistemic_policy]` parser; kernel triggers halts when predicates fire and journals halt reason with structured payload (`tag`, `value`, `threshold`, `policy_id`, `derived_from`).
- Spirit self-telemetry within principal namespace (FR56): success/failure counts, latency distributions, halt-recall events, distillation outcomes — read without per-read operator admission.
- `scalar.tap` channel (ADR-035, binding-v0.5): dedicated read-only stream from Capability Registry's tagged-scalar slot; every `set_scalar` write emits `(spirit_id, tag, value, timestamp)`.
- Hot-Swap Coordinator I14 enforcement check (`halt_set` validation before swap; `EHaltContinuityViolation` if Spirit-author hasn't declared `halt_protocol_compatibility = N`) — **halt-continuity runtime path in E5; halt schema verified here**.
- Cross-Spirit memory isolation 200-corpus authoring + execution (NFR-Sec-14: 8 categories — namespace enumeration / working-memory read-across / decision-frame observation / halt-signal observation / transparency-log cross-read / working-memory-digest cross-read / capability-token forgery cross-Spirit / sandbox-escape lateral).

**FRs covered:** FR15 (halt resolution mechanism — E3 owns UX surface), FR27, FR28, FR29, FR30, FR31, FR32, FR56.

**Key NFRs:** NFR-Test-4 (halt-recall ≥0.7, halt-precision ≥0.85 per Spirit class on bmad-eval — full gate against E8 Spirits), NFR-Test-6 LCAS framework completion (full corpus across E2 + E7), NFR-Sec-14 (cross-Spirit memory iso 200-corpus, P0 ship-block), NFR-Aud-7 (5-metric distillation gate: digest-recall ≥0.90 / digest-faithfulness ≥0.98 unflagged contradictions / digest-hedge-preservation ≥0.95 / digest-traceability = 100% kernel-enforced via I11 / digest-secret-leakage = 0%), NFR-Aud-14 (intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain).

**Corpora authored in E4:**
- HSIS partial (Researcher + Observer Spirit class corpora — 50+50 = 100 of 300 total; remaining 200 in E5).
- Cross-Spirit memory isolation 200-corpus.
- Five-metric distillation eval corpus ~200 annotated digests.

**Acceptance demo:** Spirit declares `[epistemic_policy]` with predicate `on_value_above(tag="uncertainty", threshold=0.8)`; Spirit writes scalar above threshold; kernel emits halt with structured reason; Spirit-A cannot enumerate or read Spirit-B's principal namespace under any of 200 adversarial scenarios.

### Stories

## Story 4.1: Halt Protocol Mechanism — Three Resolution Kinds + Halt-Receipt 99.9% (SINGLE HALT OWNER)

As the substrate's halt-protocol owner,
I want the halt mechanism (ADR-019 + ADR-022) to be the SINGLE owner of: halt invocation primitive, the three resolution kinds, the I14 halt-continuity invariant, halt-receipt production, and the halt-recall/precision floor measurement — while E1a holds halt schema types only, E3 holds halt resolution UX only, and E5 holds halt-continuity-across-hot-swap only,
So that halt logic never fragments into multiple owners and every Spirit termination (planned or unplanned) produces an audit-grade receipt.

**Acceptance Criteria:**

**Given** `crates/maos-kernel-core/src/halt/mod.rs::invoke_halt(payload: HaltPayload) -> HaltReceipt`
**When** a Spirit calls `epistemic.halt(payload)` from its `[epistemic_policy]` rules
**Then** `maos-kernel-core` journals a `HaltEntry` to `crates/maos-audit/src/journal.rs::write_halt_entry()` with fields `{ tag, value, threshold, policy_id, derived_from, spirit_pid, boot_nonce, timestamp_ns }`
**And** the kernel suspends the Spirit thread and enters `HaltState::PendingResolution`
**And** this is unit-tested in `crates/maos-kernel-core/tests/halt_invoke_test.rs` against `MockHaltResolver` (no integration dependency on E3 Story 3.3 at this AC's gate)

**Given** the `HaltResolver` trait defined in `crates/maos-kernel-core/src/halt/resolver.rs` with `MockHaltResolver` for unit isolation
**When** unit tests exercise the three resolution kinds (`provided_context`, `accepted_halt`, `authorized_override`)
**Then** `authorized_override` appends `OutputMarker::Override` to the Spirit's output queue (consumed by `output_shape` predicates from Story 4.2)
**And** `accepted_halt` transitions the Spirit to `HaltState::Terminated` and emits `task.orphaned` per FR12
**And** `provided_context` resumes with the supplied context appended to working memory
**And** all three paths produce a `HaltReceipt` with resolution fields populated
**And** a comment block in `resolver.rs` states: "Integration with E3 Story 3.3 UX surface wires here — see `crates/maos-director-surface/src/halt_ui.rs`." (the actual UX integration test is owned by Story 3.3, not this story)

**Given** any termination path in `crates/maos-kernel-core/src/lifecycle/` (planned unload, unplanned crash, or halt-rejection)
**When** `terminate_spirit()` is called
**Then** a `HaltReceipt` is written to `crates/maos-audit/src/journal.rs` before the OS process exits
**And** the receipt production rate is ≥99.9% measured against the 1000-termination corpus at `crates/maos-eval/fixtures/termination-corpus-v0/`
**And** `cargo test -p maos-kernel-core -- test_halt_receipt_production_rate` asserts ≥999/1000 receipts present

**Given** the v0.3 provisional halt corpus at `crates/maos-eval/fixtures/halt-corpus-v0/` (N=50 hand-authored synthetic scenarios — round-3 fix per Amelia's defect finding; the E8 reference-Spirit corpus replaces this at v1.0)
**When** `cargo test -p maos-eval -- test_halt_recall_floor` runs against the synthetic corpus
**Then** halt-recall is ≥0.7 across the 50 scenarios
**And** halt-precision is ≥0.85
**And** the predicate-firing recall floor is ≥0.85 (FR32)
**And** the test output names any failing scenario by file path for triage
**And** the corpus is tagged `synthetic-v0` to distinguish from E8 reference corpora at v1.0
**And** **intra-E4 ordering: Story 4.5 (HSIS corpus 100 scenarios) MUST close before Story 4.1 AC closes at v1.0** to provide the production-grade corpus replacing `synthetic-v0`

**Given** the halt-continuity-across-hot-swap I14 invariant
**When** Hot-Swap Coordinator (E5 Story 5.2) calls `validate_halt_set(spirit_manifest)` in `crates/maos-kernel-core/src/halt/mod.rs`
**Then** the function returns `Err(EHaltContinuityViolation { schema_mismatch: ... })` if the incoming Spirit hasn't declared `halt_protocol_compatibility = N` matching the predecessor's halt schema version
**And** the integration test that exercises this end-to-end lives in `crates/maos-lifecycle/tests/hot_swap_halt_continuity_test.rs` and is owned by Story 5.2 (not this story)
**And** the unit test for `validate_halt_set` returning the typed error lives in `crates/maos-kernel-core/tests/halt_continuity_test.rs` and is owned here

## Story 4.2: Implement the Tagged-Scalar Slot with Four Universal-Arithmetic Predicates

As a Spirit author,
I want to write tagged scalars via `working_memory.set_scalar(tag, value, derived_from)` AND declare per-tag `[epistemic_policy]` rules using the four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`), AND have those writes streamed to subscribers via `scalar.tap`,
So that the kernel performs ONLY universal-arithmetic comparison — never variance, entropy, EFE, KL, or any Spirit-specific cognitive computation (§4.0.7).

**Acceptance Criteria:**

**Given** a Spirit calls `working_memory.set_scalar("uncertainty", 0.83, "derived_from_observation_42")`
**When** the kernel persists the scalar
**Then** the kernel records `(spirit_id, tag, value, derived_from, timestamp)` to the working-memory store
**And** the kernel does NOT interpret tag-specific semantics — only routes by tag identity
**And** the write emits to the `scalar.tap` channel (ADR-035, binding-v0.5) as `(spirit_id, tag, value, timestamp)`

**Given** a Spirit declares `[epistemic_policy] on_value_above(tag="uncertainty", threshold=0.8)`
**When** a `set_scalar` for `tag="uncertainty"` writes a value > 0.8
**Then** the kernel triggers `epistemic.halt(payload)` per Story 4.1
**And** the predicate evaluation involves only the four universal-arithmetic predicates (no statistical tests, no derivatives, no Spirit-specific math)

**Given** the kernel-API surface invariant test (Story 0.2)
**When** any kernel function involving Spirit-specific cognitive computation is added
**Then** the function is classified as `other` and the build hard-fails
**And** the test enforces §4.0.7's non-interpretability principle structurally

**Given** an Observer Spirit subscribed to `scalar.tap`
**When** any other Spirit writes a scalar
**Then** the Observer receives `(spirit_id, tag, value, timestamp)` in real time
**And** the Observer can detect pre-halt drift before the predicate fires (consumed by E8 Story 8.3)

**Given** the predicate-firing recall and precision floors
**When** measured against the bmad-eval corpus per Spirit class
**Then** predicate-firing recall is ≥0.85 per Spirit class (FR32)
**And** precision is ≥0.85 per Spirit class

## Story 4.3: Provide Three Memory Tiers with Principal Namespace and Spirit Self-Telemetry

As a Spirit author,
I want three memory tiers — `private` (per-Spirit), `shared` (Host-wide), and `collective` (scaffold; full Postgres+pgvector Loom-lite at v1.5) — AND a typed Principal Memory Namespace `principal:<principal_id>:<schema>` under the private tier with subject-access / right-to-be-forgotten / redaction-on-export contracts, AND the ability to read my own performance telemetry within that namespace without per-read operator admission,
So that I can build cognitive Spirits with proper memory hygiene and the substrate enforces I5 namespace isolation mechanically.

**Acceptance Criteria:**

**Given** the Memory Manager three tiers
**When** a Spirit calls `memory.write(tier, key, value)`
**Then** `private` writes go to per-Spirit `Arc<RwLock<HashMap>>` + per-Spirit-namespaced filesystem
**And** `shared` writes go to Host-wide SQLite-backed kv with namespace prefix per writer
**And** `collective` writes are rejected at v0.5 with a clear error (full Loom-lite at v1.5 via E10 Story 10.4)
**And** every write is namespace-enforced per I5 — Spirit-A cannot write outside its own namespace

**Given** a Spirit writes to `principal:alice@example.org:calendar`
**When** the kernel persists the entry
**Then** the entry lives in the Spirit's private tier under the `principal:` typed namespace (ADR-026)
**And** the entry is automatically eligible for subject-access query (E9 Story 9.1)
**And** the entry is eligible for GDPR Art. 17 forget cascade (E9 Story 9.2)
**And** the entry is eligible for redaction-on-export

**Given** a Spirit opts into the `memory.md` convention
**When** the Spirit writes `memory.md` to its private namespace
**Then** the kernel persists it like any other private-tier write
**And** the kernel does NOT interpret the contents (universal cohort convention)

**Given** a Spirit reads its own performance telemetry within its principal namespace (FR56)
**When** the Spirit calls `telemetry.self()`
**Then** the kernel returns success/failure counts, latency distributions, halt-recall events, distillation outcomes
**And** the call does NOT require per-read operator admission (Spirit's own data, Spirit reads it)
**And** the data is scoped to the Spirit's principal namespace per FR31

## Story 4.4: Enforce the I11 Audit Chain on Distillates with `log.recall` and the Five-Metric Gate

As a Spirit author building a Researcher-class Spirit,
I want `log.recall(filter, limit, cursor)` + `log.fetch(frame_id)` participant-scoped with A2A consent honoring, AND the ability to produce distillates with a kernel-enforced I11 audit chain (mandatory `source_log_ref`, `distillation_depth`, `intent_lineage`), AND a measurement harness for the five-metric distillation gate (NFR-Aud-7),
So that I can build memory-distilling Spirits whose every digest is provably traceable back to raw frames and measurable against the five quality metrics.

**Acceptance Criteria:**

**Given** a Spirit calls `log.recall(filter, limit, cursor)`
**When** the kernel processes the recall
**Then** the kernel scopes results to participant frames (Spirit was sender or receiver)
**And** the kernel honors A2A consent envelopes — frames marked private to a peer are excluded
**And** payloads fetch on-demand via `log.fetch(frame_id)` with the same scoping

**Given** a Spirit produces a distillate via Spirit-side LLM compression
**When** the Spirit writes the digest
**Then** the kernel enforces the I11 audit chain — the digest MUST include `source_log_ref` flattened to original raw frames, `distillation_depth`, and `intent_lineage`
**And** the kernel rejects digest writes missing any of the three with `EDigestAuditChainMissing`

**Given** the five-metric distillation gate harness
**When** a distillation-shipping Spirit's digests are measured against the eval corpus
**Then** digest-recall is ≥0.90 (NFR-Aud-7)
**And** digest-faithfulness is ≥0.98 unflagged contradictions
**And** digest-hedge-preservation is ≥0.95
**And** digest-traceability is 100% (kernel-enforced via I11)
**And** digest-secret-leakage is 0% (zero-tolerance)

**Given** the corpus tiers (NFR-Aud-8)
**When** the harness runs the N=100 per-commit slice
**Then** CI-width ≈ 0.124 is observed (sufficient for trend detection)
**And** the quarterly N=500 audit gives CI-width ≤ 0.05 at p=0.90 for digest-recall

## Story 4.5: Author the Cross-Spirit Isolation 200-Corpus and Enforce I14 Halt-Continuity in Hot-Swap

As the substrate's cross-Spirit-isolation guarantor,
I want a 200-scenario adversarial corpus (NFR-Sec-14) where Spirit-A actively attempts to enumerate / read / side-channel / timing-attack Spirit-B's substrate state, AND the Hot-Swap Coordinator's I14 enforcement check (validate `halt_set` before swap; reject with `EHaltContinuityViolation`), AND 100% intent-lineage propagation across re-emission (NFR-Aud-14),
So that the v1.0 hermes-tenant positioning sentence is defended by mechanical evidence, not asserted.

**Acceptance Criteria:**

**Given** the cross-Spirit memory isolation 200-corpus
**When** the corpus is authored and committed
**Then** the corpus covers 8 categories (≥25 scenarios per category): namespace enumeration / working-memory read-across / decision-frame observation / halt-signal observation / transparency-log cross-read / working-memory-digest cross-read / capability-token forgery cross-Spirit / sandbox-escape lateral
**And** each scenario has Spirit-A actively attacking and Spirit-B's expected state un-leaked

**Given** the corpus runs as a CI gate
**When** all 200 scenarios execute
**Then** isolation is maintained in 200/200 (NFR-Sec-14 floor)
**And** any leak is a P0 ship-block

**Given** a Hot-Swap operation (E5 Story 5.2)
**When** the Hot-Swap Coordinator validates `halt_set` against the manifest's `halt_protocol_compatibility = N` declaration
**Then** the swap proceeds if the schemas are compatible
**And** the swap is rejected with `EHaltContinuityViolation` if active halts would be orphaned by the schema change

**Given** any cross-Spirit IAC frame
**When** the frame is emitted or re-emitted
**Then** the frame carries unbroken `intent_lineage` chain back to the originating principal intent (NFR-Aud-14, I13)
**And** 100% of cross-Spirit frames carry the lineage
**And** missing lineage is rejected at the IAC bus with `EIntentLineageBroken`

---
