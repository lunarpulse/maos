# ADR-046: Cost Attribution and Reconciliation (FR64 / NFR-Cost-1)

## Status

**Accepted** — ratified at Story 9.3b Task 0 (2026-06-14). binding-v0.5. Supersedes the 2026-06-13 preflight stub; folds in Round-2 rulings (R2–R6, R8), the **sec-redteam sign-off** (SR-1…SR-5), the **operator decision** to ship the observability-not-invoice posture (2026-06-14), and pre-Task-0 closeouts (C0, C1, C2). Implementation pending (Story 9.3b). Determinism discipline inherited from [ADR-028](ADR-028-replay-determinism-trace-shape.md); GDPR machinery from [ADR-026](ADR-026-principal-memory-namespace.md). The kernel re-pin is dogfooded through [ADR-045](ADR-045-governance-audit-artifacts.md) §F6.

## Context

FR64 asks for per-Spirit/per-task/per-principal cost attribution in the Transparency Log; NFR-Cost-1 asks for ≥98% reconciliation against provider billing. Today `TokenUsage` + `ProviderAttribution` are returned by providers then **discarded** at the inference emission site (`crates/maos-kernel-core/src/inference/mod.rs:350`); there is **no pricing model** anywhere; `task` and `principal` are not in the inference ABI; reconciliation "against provider billing" is untestable in CI as written (the untestable-non-oracle failure mode killed in ADR-028 / Story 9.2b F3).

A sec-redteam review (3 adversarial lenses, 2026-06-14) examined the proposed lineage-derived attribution against the code and **falsified two load-bearing claims**, forcing a posture change:

- **The attribution source is spoofable by the attributed party.** Principal is derived from `principal_ids_for_spirit_pid` over the `principal_index`, which is written whenever a Spirit writes a `principal:<id>:<schema>` namespace. **`validate_namespace_write()` (`crates/maos-kernel-core/src/memory/mod.rs:327`) is a stub returning `true`** — no per-principal write authorization. A Spirit can write to *any* principal namespace, injecting arbitrary principals into its own attribution set. "Never caller-supplied" is true of the inference request but false of the write path.
- **The source is lifetime-cumulative, not per-call.** `principal_ids_for_spirit_pid` = `SELECT DISTINCT principal_id … WHERE writer_spirit_pid = ?`, **no time/session bound**. `Resolved(single)` is the transient case; cumulative `Ambiguous` is the steady state for a multi-tenant Spirit; an inference-only Spirit returns `[]`.
- **No session-principal concept exists** anywhere in the kernel (verified). The proxy is the only available source — so the fix is not "use the real principal" (there is none) but to **restrict and label** what the proxy may drive.
- The originally-named compensating control (reconcile per-principal vs provider invoices) is **structurally blind**: the provider bills a single host API key with no principal breakdown, so the only reconcilable quantity is the aggregate host total, which nets out all cross-principal mis-attribution.

**Operator decision (2026-06-14):** ship the **observability-not-invoice** posture. With no per-principal invoice emitted, the spoofable/smeared/blind-control risks cannot cause financial mis-billing — they degrade only a clearly-labeled, coverage-reported signal.

## Decision

### 0. POSTURE — cost attribution is a provenance-tagged OBSERVABILITY signal, NOT a per-principal invoice

This is the governing decision. Everything below serves it. FR64 v1.0 produces a cost *observability report* with honest coverage reporting and a `host-unallocated` pool; it does **not** emit a per-principal charge.

### 1. R4 — kernel emits RAW dimensional facts; money is computed read-time

Kernel-core **never multiplies tokens by price** (pricing is policy; policy mutates; a price baked into an append-only journal is wrong the moment the price-book changes). The journaled cost frame carries quantities + identity only — **no `cost_micro`/`usd_micros` field**. `ProviderPricingConfig` (static `(provider, model) → {input_per_1k, output_per_1k}` in micro-USD, loaded at init, **no live fetch**) is a *type* in `maos-domain`, *consumed* at reconcile-time in `maos-audit`, **never imported by kernel-core**.

### 2. R6 — extensible cost-frame shape; CPU-time / storage-I/O deferred to v1.1

Cost-dimension quantities ride `dimensions: BTreeMap<CostDimension, i64>` (integer units) on a `schema_version: u16`-stamped payload; identity (`provider`, `model`, `principal`, `spirit_pid`, `ts`) are explicit struct fields.

- **`BTreeMap`, never `HashMap`** — nondeterministic serialization order would break ADR-028 byte-identity replay on day one.
- v1.0 dimensions = `TokensIn`, `TokensOut`. **`UsdMicros` is NOT a journaled dimension** (money is read-time, §1).
- **CPU-time and storage-I/O are DEFERRED to v1.1** (neither exists today). The extensible shape makes the addition purely additive (new enum variant + new map key; absent keys were never serialized, so existing journaled frames stay byte-identical). A **forward-read-tolerance test** (a synthetic frame with an unmodeled dimension key the reader must tolerate, not panic) ships in v1.0 as the insurance on the deferral.

### 3. R3 — `task_ref` is DROPPED from v1.0

Verified: `InferenceRequest` (`crates/maos-domain/src/ports/inference.rs:30`) carries no request/correlation/span id, and the SCB (`control_block.rs:273`) has no current-task marker. So a `task_ref` would be **structurally always-None** at emission — dead weight on a billing surface. Because `abi-diff` is Added-only, a future story can add it **for free, already carrying a value**. No synthesized weak key (a `spirit_pid + timestamp` key fabricates an attribution signal and lies about its resolution). The dimension's intent is documented here, not in the ABI. **Named successor:** the SCB current-task marker (its own story) unblocks per-task cost.

### 4. R2 / SR-4 — principal attribution: tagged, non-fabricating, count-not-members

Principal is resolved at emission via `principal_ids_for_spirit_pid(req.spirit_pid)` as `PrincipalRef::{ Unattributed | Resolved(id) | Ambiguous }`, carrying `attribution_source: write-target-proxy` + `attribution_confidence: {exact|ambiguous|unknown}`:

- empty ⇒ `Unattributed` / `unknown` (the frame still emits), single ⇒ `Resolved` / `exact`, N ⇒ **`Ambiguous` carrying a COUNT + `ambiguous` marker — never the member identifiers** (recording `{alice,bob,carol}` against one event is a cross-tenant linkage leak and multiplies the un-erasable channel).
- The kernel **never splits / pick-firsts / fabricates**.
- The field is **never named or typed "the authorizing principal"** — it is a write-target proxy, and the governance/audit record must not assert it as authority (it would be a plausible-looking falsehood with full append-only weight).

### 5. SR-1 / SR-2 — billing posture: observability, not invoice

- **Only high-confidence `Resolved(single)` is attributed to a principal.** `Ambiguous` + `Unattributed` cost rolls into an explicit **`host-unallocated`** line. **No N-way split, no per-principal charge.**
- The report surfaces an explicit **`attributable_fraction`** coverage metric (per-spirit + host-wide), so operators see the limitation instead of it being laundered into confident per-principal numbers.
- The proxy's trust caveat (`validate_namespace_write` is an unguarded stub) is acceptable **only because** no invoice is emitted; gating that write path is a tracked follow-up (§10), not a 9.3b blocker.

### 6. R5 — integer micro-units; accumulate then round once

Cost is computed read-time in **integer micro-USD — no `f64` anywhere in the accumulation path**. Tokens are `u64`; `price_micro_per_1k` is `u64` ($0.003/1k = 3000). **Accumulate `tokens × price_micro_per_1k` as `u128`, divide `/1000` ONCE at the window boundary** — round-per-call-then-sum systematically under-bills (e.g. price 1 micro/1k, 600 tokens ×3 → per-call floor `0+0+0=0` vs sum-then-round `1800/1000=1`), a monotonic revenue leak. `u128` avoids overflow at scale. This is why §1's raw-facts split is *required*, not merely cleaner: per-call money in the journal would bake the leak in permanently.

### 7. F9 — independent oracle; the CI gate is exact, not fuzzy

- **Independent golden cost vectors** (~15–20 scenarios), `expected_micro_usd` computed by a route that does **NOT import the pricing function** (anti-tautology), asserting **sum-then-round** (replicating the §6 accumulation order — this is what catches a reconcile that regresses to round-per-call). Coverage: single-call, multi-call, multi-provider, multi-model, zero-token, cache-hit.
- **Property tests** on aggregation + rounding (round-per-call vs sum-then-round drift, monotonicity, `.5`-boundary determinism, no `f64`).
- **CI gate = 100% / deterministic** against a committed synthetic price-book fixture (the arithmetic is exact-to-rounding; "≥98%" is NOT a CI threshold).
- **Operational ≥98% reconciliation = a weekly sampling runbook, explicitly NOT a CI gate, and re-scoped per SR-2 to an AGGREGATE price-book/token-count control** (`Σ(attributed) vs host-invoice-total`). It does NOT and cannot validate per-principal correctness (no external per-principal ground truth exists). The runbook text must state this limitation; no gate is labeled "best-effort". This layer catches a provider changing rates under you — which no synthetic fixture can.

### 8. SR-3 (CRITICAL) — extend the forget cascade to every new principal-bearing frame kind, in this story

Because `principal_id` is journaled (the point of the signal), it must stay erasable. The cascade (`MemoryManagerAdapter::forget_with_reason`, `crates/maos-kernel-core/src/memory/mod.rs:113`) is currently frame-kind-specific (Distillate only). It **must be extended** to cost-attribution and every principal-bearing governance frame, **landed in this story**:

- Insertion point (closeout C0): replicate the distillate-scrub step (`:208-227`) for cost/governance frames. Discovery is a **clean indexed query** (principal_id is a structured field, not a body scan). New `*_frames_for_principal` + `scrub_*` methods live in **maos-iac** (off-baseline); the kernel-core touch is the orchestration call only. Wire the redacted frame-ids into the cascade payload, `ForgetReceipt`, and the **erasure proof pre-tree** (`crates/maos-audit/src/erasure/proof.rs`).
- **Test:** emit a principal-bearing 9.3b frame for P → `forget(P)` → assert P absent from BOTH `principal_index` AND every 9.3b frame kind in the journal AND **byte-identity replay still passes post-scrub**.
- **If this cannot land in 9.3b, emit NO principal-bearing frames in 9.3b** — append-only + byte-identity replay makes any gap permanent and irreversible (no follow-up can fix it).

### 9. AC6 — erasure↔lifecycle join (shared `schema_id`)

The erasure record stamps `schema_id` + `schema_version` (the schema in force at erasure-execution time, read from the [ADR-045](ADR-045-governance-audit-artifacts.md) §R10 registry — **closeout C2: AC6 depends on R10, sequence R10 → AC6**), via additive fields on `ForgetReceipt` + the cascade payload (both already extensible), bytes-identical for existing entries (mirrors 9.2b F1). The lifecycle stream and the erasure stream key off **one canonical `schema_id`** covering the set of erasure-class lineage ids (Art.17 / legal-hold / retention), with a test that fails on a divergent `schema_id` for the same claim.

### 10. SR-5 — proxy debt recorded; the real fix is per-call principal binding

v1.0 attribution is a **write-target reverse-lookup proxy, not an authority/session principal**. The durable fix is **per-call principal binding** (tie attribution to the live session/task lineage at the emission site) — the same missing primitive as the §3 SCB-current-task-marker. Tracked as a follow-up; not a 9.3b blocker under the observability posture.

## Consequences

- **Kernel-core delta ~60–100 LOC** (closeout C1 — supersedes the stub's ~70–120 *and* Round-2's ~46–64): FR64 emission ~28–40 + FrameKind 28 ~18–24 + **SR-3 forget-orchestration call + AC6 erasure-payload stamp ~15–35** (both touch `forget_with_reason`). Bulk of cost-compute/pricing/scrub work stays off-baseline (maos-domain/maos-audit/maos-iac). **Re-pin target ~21400–21440** (from 21336), jointly with FR62 (FLAG-Winston), recorded as the first ratified `AbiExtensionProposal`.
- NFR-Cost-1 splits into a deterministic CI claim (100% vs synthetic price-book) and an operational claim (aggregate ≥98% vs real invoices, runbook) — errata applied to `requirements-inventory.md`.
- The cost report is **coverage-honest, not an invoice**: a real `attributable_fraction` < 1 is expected and surfaced, not hidden.
- Two items are explicitly **out of 9.3b, tracked**: the **SR-1 write-path authorization** fix (`validate_namespace_write` stub) and the **SR-5 / R3 per-call principal binding** (SCB current-task marker + an `InferenceRequest` correlation id).

## Gate

- **CI cost-reconcile oracle**: integer micro-units (no `f64`), independent golden vectors (no import of the pricing fn, assert sum-then-round), rounding/aggregation property tests, 100% vs synthetic price-book fixture.
- **Billing-posture assertions**: no billable/attributed record for cardinality ≠ `Resolved(single)`; `Ambiguous` journals a count not members; `attributable_fraction` surfaced.
- **SR-3 forget-cascade coverage test**: `forget(P)` clears P from `principal_index` + all 9.3b principal-bearing frames + byte-identity replay passes.
- **Forward-read-tolerance test** for an unmodeled cost dimension (R6 deferral insurance).
- `cargo xtask check-kernel-baseline` green at the jointly re-pinned figure (~21400–21440).

## Sign-off required

sec-redteam on **F8 (principal semantics)** and **F9 (cost oracle)**. **Recorded 2026-06-14: F8 = SIGNED OFF under the observability-not-invoice posture** (SR-1, SR-2, SR-4, SR-5 satisfied by scope; **SR-3 is the one Critical blocking gate** dev must clear). F9 oracle design signed off (R5/R6/§7).
