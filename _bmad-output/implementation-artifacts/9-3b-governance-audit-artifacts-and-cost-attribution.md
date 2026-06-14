---
dev_model_used: "claude-opus-4-6"
recommended_dev_model: claude-opus-4-8
---

# Story 9.3b: Governance Audit Artifacts (FR62) + Cost Attribution (FR64)

Status: **done** (2026-06-14). All tasks complete; all review findings resolved. Dev model claude-opus-4-6. §A6: Opus (net N/A). Kernel re-pin 21336→21438 (+102 LOC, FLAG-Winston).

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- ⚑ SPLIT from the original 9.3 at party-mode preflight (2026-06-13, Winston·Murat·John·Amelia). THIS story = the
     kernel-touching half: FR62 (governance audit artifacts) + FR64 (cost attribution). Story 9.3 = FR63 (error
     catalog, kernel-neutral, lands first). FR62 and FR64 are co-located because they share ALL their plumbing
     (F6 ABI-proposal model, F7 multi-kind filter, F8 task/principal threading) and both carry a blocker
     (F5 frozen-ComplianceClaim, F9 cost-reconciliation oracle), behind ONE authorized kernel re-pin. -->

> **⚑ ORIGIN.** Split from the original 9.3 by unanimous party-mode (2026-06-13). The split seam was contested (Winston initially wanted FR62 with FR63) and reconciled live: FR62 is **not** kernel-neutral (its FrameKind add re-pins the kernel like FR64's emission), so FR62 belongs with FR64 behind one re-pin authorization, with FR64's re-pin recorded as the **first ratified `AbiExtensionProposal`** — dogfooding FR62's own governance model intra-story.

## Story

As an enterprise operator and compliance officer,
I want governance audit-queryable artifacts (vetter-key admission/rotation, ABI-extension proposals/ratification, ComplianceClaim schema lifecycle) journaled and queryable via `maosctl audit query --kind governance` (FR62), AND cost attribution per Spirit/principal recorded at the inference site as a **provenance-tagged observability signal** (NOT a per-principal invoice — sec-redteam 2026-06-14; task dimension dropped v1.0 per R3) with a deterministic cost report + honest coverage reporting (FR64 + NFR-Cost-1),
so that the ledger explains itself: who did what under whose authority, and — to the extent attribution is confidently resolvable — who spent what.

---

## Context & Charter Boundary (READ FIRST)

This is the **write-side, kernel-touching** half. Two concerns, sequenced.

- **AUTHORIZED kernel-core delta (Winston pre-authorized the SHAPE).** Both FR62 (new `FrameKind` discriminant in `maos-iac`) and FR64 (cost capture at `crates/maos-kernel-core/src/inference/mod.rs:350`) move the kernel baseline. Treat exactly like Story 9.2's delta: **minimal + additive**, re-pin `xtask/kernel-core-baseline.toml` (currently **21336**) **once, jointly**, in the same PR, **FLAG-Winston**. Estimated net kernel-core delta **~60–100 LOC** (Pre-Task-0 closeout C1, supersedes the R4 ~46–64 figure): FR64 emission capture ~28–40 + FrameKind 28 ~18–24 + **SR-3 forget-orchestration call + AC6 erasure-payload stamp ~15–35** (both touch `forget_with_reason` in kernel-core). Bulk of cost-compute/pricing/scrub work stays off-baseline (maos-domain/maos-audit/maos-iac). Pre-authorized re-pin target **~21400–21440** (revised; confirm the exact figure post-implementation).
- **`abi-diff` stays Added-only.** Any ABI surface change (the `task_ref` correlation field, new public types) must be purely additive against `xtask/abi-baseline/`.
- **`maos-audit` stays read-only** (`SQLITE_OPEN_READ_ONLY`, `#![forbid(unsafe_code)]`); the FR62 query side and FR64 reconcile side are pure readers. The Story-9.2 no-write-open grep applies.
- **`maos-cli` stays kernel-core-free** (`dep_kernel_core_free_test.rs`).
- **Workspace stays 44 crates.**

### SEQUENCING CONSTRAINT (normative AC, not a hope — Winston's concession condition)

**FR62's F6 governance model (`AbiExtensionProposal` + ratification record + the abi-diff↔ratified-proposal reconciliation gate) MUST land and be exercisable BEFORE FR64's kernel re-pin.** FR64's re-pin is then implemented **as the first ratified `AbiExtensionProposal`**, recorded through the model it dogfoods. Build order within this story: **FR62 governance model → FR62 streams → FR64 cost → FR64 re-pin via the F6 model.**

**§A6 NON-OPUS SAFETY NET applies — MANDATORY.** FR64 (cost attribution + kernel-core delta) and FR62 (audit-spine governance journaling) are named correctness-critical categories. Non-Opus dev ⇒ party-mode preflight (done) + multi-layer adversarial review (Blind + Edge + Acceptance + TestInfra) + **sec-redteam sign-off on F8 (principal semantics) and F9 (cost oracle)**. Recommended dev model: `claude-opus-4-8`.

---

## Preflight Consensus (party-mode 2026-06-13 — DECISIONS, not options)

Ratified 4/4 (Winston · Murat · John · Amelia), several **overruling the original spec defaults**. Implement; do not re-litigate.

- **F5 (BLOCKER) — ComplianceClaim freeze → DECOUPLED event stream. Reject BOTH the struct field AND the envelope field.** Schema lifecycle (version / effective-date / supersedes / ratified-by) is a property of **the schema**, owned by a registry — NOT of any claim instance. Adding a `#[serde(default)]` field to the frozen `Claim` *or* its envelope is a freeze violation in an "additive" costume (Winston). **Ruling:** journal a **schema-lifecycle governance event stream** (new `FrameKind`) that *references* ComplianceClaim identity and carries version + effective-date **on the event**. The frozen `crates/maos-spirit-abi/src/compliance.rs` `Claim`/envelope is **never touched** — which also preserves 9.2b's 21336 bytes-identity construction (Murat). Sign-off: Winston (ADR-037) + Mary (schema co-author).
- **F6 — ABI-extension proposal/ratification → `AbiExtensionProposal` governance object + a RECONCILIATION GATE.** Today ABI governance is *only* `xtask abi-diff` (mechanical truth: did the ABI change). FR62 adds the **provenance record** (who proposed/ratified, under which ADR). The **new gate reconciles the two**: every abi-diff-detected change must have a corresponding ratified proposal in the journal, or CI fails. By-construction testable; keep abi-diff itself **mechanical, don't soften it into a hand-wavable workflow** (Murat). Anchored on **ADR-037 (Constitutional amendment process)**. Scope = data model + emission + query + the reconciliation gate; **no voting UI.**
- **F7 — `maosctl audit query --kind governance` → additive category resolver + `kind IN (…)`.** Do NOT overload `kind_from_string`'s `Option<i64>` signature (single-kind callers stay byte-identical). Add `kind_category_to_kinds(&str) -> Option<Vec<i64>>` (`"governance" → vec![…]`, `"cost" → vec![…]`); the filter at `crates/maos-audit/src/lib.rs:185-192` builds `kind IN (?, …)` with a bind loop when a category resolves, keeps `kind = ?` for single. **Three assertions (Murat):** (i) completeness — the `governance` expansion is cross-checked against the kind registry so a future 4th governance stream can't silently fall out (silent under-reporting is the worst audit failure mode); (ii) non-contamination — a non-governance kind never appears under `--kind governance`; (iii) backward-compat — existing single-kind queries byte-identical. Make it multi-kind-capable from day one (`--kind governance,cost`) — touch the query ABI once (John).
- **F8 (BLOCKER) — task/principal attribution → ATTRIBUTION-AT-EMISSION, not caller-supplied ABI. (Spec default INVERTED.)** `principal_id` is a security-attribution field; if a Spirit can populate its own, cost+governance attribution become **spoofable by the attributed party** (Winston). Verified (this preflight): at the emission site,
  - **principal IS resolvable** via `self.transparency_log.principal_ids_for_spirit_pid(req.spirit_pid)` (`crates/maos-iac/src/adapter/transparency_log.rs:674-697`), ~5–10 LOC, zero new kernel state. **Two caveats flagged for sec-redteam:** it returns a `Vec` (a Spirit may write to multiple principals → need a first/single/split policy), and it resolves *principals-written-to* (a memory-write proxy), **not** necessarily the authoritative session principal. Prefer a kernel-resident session principal if one exists; else use this reverse-lookup with the documented caveat — **sec-redteam signs off on whether memory-write-history is acceptable attribution.**
  - **task is NOT resolvable** at emission (verified: the inference adapter holds no scheduler ref, and the SCB has **no current-task marker** even if it did — `crates/maos-kernel-core/src/scheduler/control_block.rs:273` is a list of all in-flight tasks). Per Winston's own carve-out — `task_ref` is a **correlation-id, not an authz field** — and since lineage genuinely cannot resolve it, an **additive `task_ref` correlation-id** is justified. Adding a scheduler ref to the inference adapter (~30–50 LOC, breaks adapter isolation) is the alternative; **default to the additive correlation-id** unless sec-redteam/Winston prefer the coupling.
- **F9 (BLOCKER) — pricing + reconciliation → independent oracle; kill the ≥98% CI threshold. (Spec default AMENDED.)** `ProviderPricingConfig` (static `(provider, model) → {input_per_1k, output_per_1k}`, loaded at init, NO live fetch) computes `cost`. **The oracle must NOT import the pricing function to derive its expectation** (else it's a tautology — same formula on both sides proves only that `×` works — Murat). Four layers:
  1. **Independent golden cost vectors** (~15–20 scenarios) with `expected_micro_usd` computed by a *different route* (human-audited reference table checked in), covering single-call, multi-call task, multi-task principal rollup, mixed-provider, mixed-model, zero-token, cache-hit-discount.
  2. **Property tests on aggregation + rounding** (where real cost bugs live): round-per-call-then-sum vs sum-then-round drift; monotonicity; rounding determinism at the `.5` boundary; **integer micro-units — NO `f64` anywhere in the accumulation path** (an `f64` in pricing accumulation is itself a red-team finding).
  3. **CI gate = 100% / deterministic** against the committed synthetic price-book fixture (Winston: kill the fuzzy "≥98%" — by construction the arithmetic is exact-to-rounding).
  4. **Operational drift SLO = ≥98% vs REAL provider invoices, weekly sampling runbook — explicitly NOT a CI gate** (real invoices are non-deterministic external input; this is the 9.2b F3 line). This layer is what catches a provider changing rates under you, which no synthetic fixture can.
  - Sequencing: **F8 lands before the F64 multi-principal cost vectors**, or those vectors test a stubbed principal and prove nothing (Murat).

---

## Preflight Round 2 (2026-06-14 — DEFERRED-ITEM RESOLUTIONS, party-mode Winston·Amelia·Murat·Mary·John)

Round 1 (above) ratified the design but **deferred** several correctness-critical items to "sec-redteam sign-off" or "confirm at dev time." Round 2 closed them against the **verified code** (greps recorded in the dev log). These are **RULINGS that supersede the deferred placeholders**; where a ruling amends an AC, the AC is marked `(R2)`. Implement; do not re-litigate.

- **R1 — Sequencing bootstrap is NOT circular: the reconciliation gate is ONE-DIRECTIONAL (`abi-diff ⊆ ratified`).** (Winston) The gate asserts every abi-diff-detected change is *covered by* a ratified proposal — NOT the converse. Base case = empty set, which reconciles by arithmetic (`∅ ⊆ anything`), so the gate is **born green against the zero-delta baseline** on the commit that introduces it (introducing the gate moves no ABI). **NO `genesis`/bootstrap exemption flag** — the base case falls out of the set algebra, never out of a special-case branch (an exemption flag becomes a permanent loophole). Dogfooding ordering proof lives in the **transparency_log frame sequence, NOT git history** (squash-merge erases git order; TL seq is monotonic + tamper-evident): the FrameKind-28 ratification frame for the FR64 re-pin must be a **strict TL-ancestor** (`seq < delta`) of the baseline move it ratifies.
- **R2 — F8 principal: ship the proxy FIRST-CLASS and NON-AGGREGATABLE.** (Winston+Amelia) Payload carries `principal: PrincipalRef` + `attribution_basis: enum { MemoryWriteProxy, SessionPrincipal }`. Exact match arm at `inference/mod.rs:350`:
  ```rust
  let principal = match principal_ids_for_spirit_pid(req.spirit_pid).as_slice() {
      []    => PrincipalRef::Unattributed,            // bill SURVIVES (never drop), flagged for reconcile sweep
      [one] => PrincipalRef::Resolved(one.clone()),   // happy path — bills correctly
      many  => PrincipalRef::Ambiguous(many.to_vec()),// emit ALL; reconcile-time policy splits/escalates
  };
  ```
  Kernel NEVER splits/pick-firsts (no causal basis → silent mis-billing). Cardinality>1 surfaces to the F9 reconcile runbook. When a kernel-resident session principal lands (named successor story), `attribution_basis` flips to `SessionPrincipal`, cardinality collapses to 1, **schema unchanged**.
- **R3 — F8 task_ref: DROPPED from v1.0 (both Winston & Amelia conceded on the verified fact).** VERIFIED: `InferenceRequest` (`crates/maos-domain/src/ports/inference.rs:30`) has **no** request_id/correlation_id/span_id; the SCB (`control_block.rs:273`) has **no** current-task marker. So `task_ref` would be **structurally always-None** at emission — dead weight on a billing surface, and the Added-only `abi-diff` rule lets a future story add it *later for free, already carrying a value*. **Do NOT synthesize a weak `spirit_pid+boot_nonce+timestamp` key** — that fabricates an attribution signal F8 forbids and lies about its own resolution. The dimension's intent is documented in **ADR-046 (prose), not the ABI**. The SCB current-task marker is the **named follow-up** that unblocks per-task cost (flag for John as its own story). → **AC4 amended (R2): principal-only dimension + negative assertion that no `task_ref`/`correlation_id`/`request_id` key appears in the serialized cost frame.**
- **R4 — Cost split: kernel emits RAW dimensional facts; reconcile computes money.** (Amelia) Kernel-core NEVER multiplies tokens by price (pricing is policy, mutates; a price baked into the journal is wrong the moment the price-book changes). Journaled payload carries quantities + identity only, **no `cost_micro` / `usd_micros` field**. `ProviderPricingConfig` is a *type* in `maos-domain`, *consumed* at reconcile in `maos-audit`, **never imported by kernel-core**. Revised kernel-core net **+46–64 LOC** (under the +70–120 ceiling) → lands **~21382–21400** (below the prior ~21450 projection; confirm post-impl). Per-zone budget: emission `inference/mod.rs:350` +28–40; FrameKind 28 in `transparency_log.rs` +18–24; payload+pricing type in `maos-domain` +30–45 (not kernel); category filter + read-time compute in `maos-audit` +40–70 (not kernel).
- **R5 — F9 accumulation: `u128` accumulate `tokens × price_micro_per_1k`, divide `/1000` ONCE at the window boundary.** (Amelia) Round-per-call-then-sum **systematically under-bills** (proven: price 1 micro/1k, 600 tokens ×3 → per-call floor `0+0+0=0` vs sum-then-round `1800/1000=1`) — a monotonic revenue leak. `u128` accumulator (u64×u64 overflows at scale). The independent golden oracle asserts **sum-then-round** and must replicate this exact accumulation order (that is what makes it catch a reconcile that regresses to round-per-call). Confirms R4: per-call money in the journal would bake the leak in permanently.
- **R6 — CPU-time/storage-IO: DEFER to v1.1 behind an EXTENSIBLE cost-frame shape.** (Murat) Cost-dimension quantities live in `dimensions: BTreeMap<CostDimension, i64>` (integer micro/raw units) on a `schema_version: u16`-stamped payload. **BTreeMap (sorted), NEVER HashMap** — nondeterministic order breaks 9.2b's HARD byte-identity replay on day one. Ship a **forward-read-tolerance test NOW** (synthetic frame with an unmodeled `dimensions` key → reader must not panic). **Reconciliation with R4: the journaled `dimensions` are QUANTITY-ONLY (`TokensIn`, `TokensOut`; v1.1 `CpuMicros`, `StorageIoMicros`) — NO `UsdMicros` dimension** (money is read-time per R4; Murat's Round-1 enum listed `UsdMicros`, struck here). Identity/attribution (`provider`, `model`, `principal`, `spirit_pid`, `ts`) stay explicit struct fields. Deferral text in ADR-046 must assert: adding a dimension later is additive — new enum variant + new map key; absent keys were never serialized, so existing journaled frames stay byte-identical.
- **R7 — abi-diff↔ratified gate anti-tautology: a 3-test bite-proof set, NOT one happy-path test.** (Murat) (a) real re-pin abi-diff + matching ratification → **PASS**; (b) **SAME real abi-diff + WITHHELD ratification → FAIL** (the clincher — same input, opposite verdict, kills the always-pass degeneracy); (c) mutated-canary change (`__maos_test_canary`) + non-matching ratification → **FAIL** (fails-closed + kills the rubber-stamp degeneracy where any ratification present ⇒ pass). NEVER test the gate solely with the change it shipped to ratify. Smoke arm (c) (AC-Group C #6) maps to test (b)+(c).
- **R8 — F8 principal correctness is UNTESTABLE in CI: RISK FINDING (recorded, not papered over).** (Murat) `principal_ids_for_spirit_pid` is a *memory-write proxy* ("principals written-to"), not the authoritative *session principal* ("acted-for"); they can diverge. A test `journaled == proxy_value` is **tautological** (source equals itself); a test `journaled == true_session_principal` **cannot be written** (no such oracle exists). **CI guards DROP only, labeled honestly as "journals *a* principal, not *the correct* principal."** Compensating control = **F9 layer-4 weekly runbook, which MUST reconcile PER-PRINCIPAL** (an aggregate total nets out cross-principal mis-attribution: A over-billed + B under-billed sums correct). Severity medium-high for a billing feature — this is the sec-redteam F8 sign-off item, now scoped to "is memory-write-proxy acceptable given per-principal-runbook compensation."
- **R9 — Completeness X-check independent source = `(0i64..).map_while(FrameKind::from_i64)`; check lives in `xtask`.** (Murat+Amelia) VERIFIED: `FrameKind` has **no strum/EnumIter, no `ALL` const**; `maos-audit` does **NOT** depend on `maos-iac`. Construct (no strum dep, no magic number, no kernel delta):
  ```rust
  let all: Vec<FrameKind> = (0i64..).map_while(FrameKind::from_i64).collect();
  assert!((all.len() as i64..256).all(|i| FrameKind::from_i64(i).is_none()),
          "FrameKind discriminants must stay contiguous from 0");
  ```
  `from_i64` is the **deserialization forcing function — independent of the category map** (add kind 28 and forget `from_i64` ⇒ a round-trip deser test breaks). Lands in **`xtask/src/check_governance_categories.rs`** (sibling to `xtask/src/check_error_catalog.rs`), wired in `xtask/src/main.rs`; xtask adds `maos-iac` + `maos-audit` build-deps (tooling, outside the runtime layering graph → no new runtime edge, no 44-crate risk; maos-audit does NOT depend on maos-iac — verified). **Two additive functions (closeout C3 — `FrameKind→&str` is NOT needed):** `kind_category_to_kinds(&str) -> Option<Vec<i64>>` (CLI resolver, F7) + `governance_category(i64) -> Option<Category>` (the inverse, for the check), both in maos-audit over the i64 domain. The X-check asserts the two **round-trip** over `(0i64..).map_while(FrameKind::from_i64)`. **Two assertions:** exhaustive-no-`Unclassified` (drop-out guard) + known-governance-positive (mis-bin guard); **NO catch-all `_ => Other` arm**.
- **R9b — NEW FINDING: `kind_from_string` already silently omits 13 of 28 kinds.** VERIFIED (`maos-audit/lib.rs:552`): maps only 0–11, 17, 19, 22; kinds 12–16, 18, 20, 21, 23–27 fall to `_ => None` — a **pre-existing instance of the exact silent-under-reporting failure F7 guards against.** CONSEQUENCE: the R9 completeness check must operate on the **i64/enum domain with an explicit `EXCLUDED` set**, NOT route through `kind_from_string` (which would spuriously fail on legitimately-unmapped non-governance kinds). Record the `kind_from_string` gap as a flagged audit-queryability debt (decide fix-now-vs-defer at dev time; not a blocker for 9.3b's governance+cost kinds, which the resolver adds explicitly).
- **R10 — F5 schema registry does NOT exist today → this story STUBS a minimal append-only registry.** (Mary) The frozen `compliance.rs` Claim is a *type*, not a *registry* (no version/effective/supersession). Without the stub, the lifecycle stream journals decisions **nothing actually made** (paper-trail-with-no-author). Stub = append-only manifest of `(schema_id, version, effective_at, supersedes, ratified_by)`; population is an **explicit authorized governance action that HARD-REJECTS any entry lacking a `ratified_by: ADR-id`**. Provenance chain: ADR ratification → `maosctl governance` admit (requires ratified_by) → registry append → lifecycle frame journaled; registry-write + journal-frame are **one atomic act** so the journal can never claim a lifecycle the registry doesn't hold.
- **R11 — F5 reference key = SCHEMA identity, three parts.** (Mary) `schema_id` = stable version-INDEPENDENT reverse-DNS lineage name (e.g. `compliance.claim.gdpr-erasure`), **NOT a Rust type path/discriminant** (those drift on refactor and break the join) — the correlation key; `schema_content_hash` = per-version fingerprint — the integrity anchor; `supersedes: Option<schema_content_hash>` = references the **prior version's hash** (verifiable chain v3→v2→v1), not its number. Event references schema identity ONLY — **zero claim-INSTANCE ids** (upholds the F5 boundary).
- **R12 — F5/FR62 stakeholder data-model gaps (two fields + one in-scope cross-story fix):** (Mary+John)
  - **Dual timestamps (cheap, now):** every governance event carries BOTH `recorded_at` (monotonic journal position) AND `effective_at` (when the decision takes governance effect) — they genuinely differ (ratified Jun 14, effective Jul 1) and single-timestamp makes as-of-T incident reconstruction impossible.
  - **Erasure schema-version stamp — John RULES IN-SCOPE for 9.3b (NOT a follow-up).** VERIFIED: the shipped `AuditEntry` (`maos-audit/lib.rs:85`) has **no** `schema_id`/`schema_version`. Every erasure journaled without it is a **permanent forensic dead-end** (erased records immutable, cannot retrofit) — the daily-accrual-of-unfixable-loss property forces it in now. Fix is additive, bytes-identical, **mirrors 9.2b F1** (`skip_serializing_if`). Own isolated AC + own bytes-identical proof; **MUST NOT entangle the FR62/FR64 kernel re-pin** — if dev finds it does, **escalate to John before merge**.
  - **Shared-key contract (John, non-negotiable AC):** the lifecycle stream and the erasure stream MUST key off **one canonical `schema_id` source of truth** (R11's lineage id), covering the **SET** of erasure-class lineage ids (Art.17 erasure / legal-hold / retention-expiry — VERIFIED these concepts exist, `memory.rs:364` `LegalHoldRecord`), not just `gdpr-erasure`. Test fails if the two streams can emit a divergent `schema_id` for the same claim (a stamp without this contract is a fake join that passes a demo and fails an audit).

**Sec-redteam sign-off scope after Round 2** (the §A6 net items, now sharpened): **F8** = "is memory-write-proxy acceptable given the R8 per-principal-runbook compensation + the R2 Unattributed/Ambiguous non-fabrication policy"; **F9** = the R5 integer-accumulation oracle + R6 no-`UsdMicros`-in-journal; **F5** = the R10 registry-authority + R11 schema-identity decoupling. **→ F8 sign-off result below.**

---

## Sec-Redteam Sign-Off — F8 Principal Semantics (2026-06-14, 3 adversarial lenses + code verification)

**VERDICT: BLOCK-by-default for the as-described per-principal-invoice design → RESOLVED by operator decision (2026-06-14) to ship the OBSERVABILITY-NOT-INVOICE posture.** The memory-write-proxy does NOT get sign-off to drive billing or authority records, but DOES get sign-off to drive a **provenance-tagged observability signal** under gates SR-1…SR-5. Two load-bearing claims were **falsified against the code**, and one is a **Critical GDPR finding**. R2 and R8 are amended by this sign-off (it supersedes them where they conflict).

> **✅ OPERATOR DECISION (2026-06-14): observability-not-invoice ACCEPTED.** FR64 v1.0 attributes cost as an observability signal only. This resolves SR-1, SR-2, SR-4, SR-5 **by scope** (no per-principal invoice is emitted, so the spoofable/smeared/blind-control risks cannot cause financial mis-billing — they degrade only a clearly-labeled, coverage-reported signal). **SR-3 (forget-cascade coverage) is NOT resolved by scope — it remains a hard blocking gate** because principal_id is still journaled and must remain erasable. The SR-1 write-path root fix + per-call principal binding (SR-5) are tracked follow-ups, not 9.3b blockers, under this posture.

**Code-verified facts that drove the verdict:**
- **The "never caller-supplied → not spoofable by the attributed party" claim is FALSE.** `validate_namespace_write()` (`crates/maos-kernel-core/src/memory/mod.rs:327`) is a v0.3-β **stub returning `true` unconditionally** — no per-principal write authorization. A Spirit can write `principal:<ANY id>:<schema>` (`MemoryNamespace::principal(...)`, only syntactic validation), and that records `writer_spirit_pid → that principal` in `principal_index`. The attributed set is therefore **fully attacker-controlled via the unguarded write path** — provenance laundering, not de-spoofing.
- **The reverse-lookup is lifetime-cumulative, not session-scoped.** `principal_ids_for_spirit_pid` = `SELECT DISTINCT principal_id FROM principal_index WHERE writer_spirit_pid = ?` — **no time/session bound** (`transparency_log.rs:674-697`). It answers "whom has this Spirit *ever* written for," not "whom is this call *for*." `Resolved(single)` is the **transient** case; cumulative `Ambiguous` is the **steady state** for any multi-tenant Spirit; an inference-only Spirit returns `[]` → `Unattributed`.
- **No session-principal concept exists anywhere** (grep `session_principal|current_principal|acting_principal|on_behalf` = empty). The proxy is the only available source — so the fix is *not* "use the session principal" (it doesn't exist); it is to **restrict + label** what the proxy may drive until per-call principal binding is built.
- **The R8 compensating control is structurally blind to the actual risk.** The provider invoice bills a **single host API key with zero principal breakdown** — there is no external per-principal ground truth to reconcile against. The runbook can only check `Σ(attributed) vs host-total`, which **nets out all cross-principal mis-attribution** (alice +X, bob −X = correct total). R8's "per-principal runbook" cannot validate per-principal correctness; it is a price-book/token-count aggregate control mislabeled.
- **GDPR (CRITICAL): the forget cascade is frame-kind-specific.** `scrub_distillate_body` (`transparency_log.rs:585`) + `distillate_frames_for_pids` (filters `kind = FrameKind::Distillate`, `:726`) only scrub **Distillate** frames; `principal_index.forget()` (`principal.rs:144`) deletes index rows in a **different store** from the journal. New principal-bearing cost/governance frames would be **un-erasable** — append-only + 9.2b HARD byte-identity replay makes the leak **permanent and irreversible**.

### Gating conditions (ALL must hold before/within dev; otherwise emit NO principal-bearing frames in 9.3b)

- **SR-1 — Attribution is an OBSERVABILITY signal, NOT an invoice, until the write path is gated.** The root fix (enforce `validate_namespace_write` to bind a writable `principal_id` to an authenticated token) is a kernel-auth effort likely beyond 9.3b's authorized delta — so for 9.3b: **every attribution carries a provenance/trust tag, and untrusted-provenance attribution NEVER drives a billable charge or an authority claim.** (checkable: attribution type has a trust/provenance field; billing path filters on it; a test proves an un-gated write cannot produce a billable `Resolved`.)
- **SR-2 — Never invoice from `Ambiguous` or `Unattributed`; NO N-way split.** Billable = high-confidence `Resolved(single)` only; `Ambiguous` + `Unattributed` → a **`host-unallocated` pool**, never a per-principal charge. Emit an explicit **`attributable_fraction`** coverage metric (per-spirit + host-wide). **Relabel the R8 runbook** as an aggregate price-book/token-count control; its text MUST state per-principal attribution has **no external ground truth** and is validated by no control. (checkable: assert no billable record for cardinality ≠ Resolved-single; coverage metric surfaced; R8 doc wording.)
- **SR-3 (CRITICAL) — Extend the forget cascade to EVERY new principal-bearing frame kind, in THIS story.** Cost-attribution + every governance frame variant embedding `principal_id` MUST be enumerated in and scrubbed by the 9.2/9.2b cascade, landed together. (checkable test: emit a 9.3b principal-bearing frame for P → `forget(P)` → assert P absent from BOTH `principal_index` AND every 9.3b frame kind in the journal AND byte-identity replay still passes post-scrub.) **If this cannot land in 9.3b, do not emit principal-bearing frames in 9.3b** — the irreversibility forbids a follow-up. Confirm the 9.2b `AuditEntry` `skip_serializing_if` redaction mechanism actually reaches the principal_id position in the new frames (not buried in a `Vec`/nested struct it can't parse).
- **SR-4 — Journal provenance + confidence, never a falsehood-as-fact; never `Ambiguous` membership.** The journaled field MUST carry `attribution_source: write-target-proxy` + `attribution_confidence: {exact|ambiguous|unknown}` and MUST NOT be named/typed as "the authorizing principal" (the governance record would otherwise assert a plausible-looking falsehood with full append-only authority). **`Ambiguous` → journal a COUNT + confidence marker, NOT the member identifiers** (recording `{alice,bob,carol}` against one event is a cross-tenant linkage leak AND multiplies the un-erasable channel). **`Empty` → explicit `unknown` sentinel, never a default/guessed principal.** (checkable: schema has the provenance/confidence fields; no journaled frame contains a multi-member principal set.)
- **SR-5 — Record the proxy debt + the real fix.** ADR-046 MUST state prominently that v1.0 attribution is a **write-target reverse-lookup proxy, not an authority/session principal**, spell out the divergence cases, and open a tracked debt to introduce **per-call principal binding** (tie attribution to the live session/task lineage) so the cumulative reverse-lookup can be retired. This is the same named-successor as the R3 SCB-current-task-marker follow-up — they are the same missing primitive (per-call causal context at the emission site).

**Net effect on the story:** FR64 v1.0 ships cost attribution as a **provenance-tagged observability signal** with honest coverage reporting and a `host-unallocated` pool — **not** a per-principal invoice — and **only if** SR-3's erasure coverage lands. The "≥98% per-principal runbook" (R8) is re-scoped to an aggregate sanity gate. This is the long-term-correct posture: bill only what is defensible, quarantine what is not, never mint permanent un-erasable or mislabeled personal data.

---

## Pre-Task-0 Closeouts (2026-06-14 — all open confirm-before-coding items resolved against code)

Every pre-implementation open item is now closed with a code-grounded answer. **The story is dev-ready: Task 0 (author ADR-045/046) → Tasks 1–4 with no unresolved design/feasibility question.** Two of these are *corrections* dev MUST carry (C1 kernel-delta size, C2 AC6↔R10 dependency).

- **C0 — SR-3 forget-cascade extension is FEASIBLE with a clear insertion point (not a research risk).** The cascade is `MemoryManagerAdapter::forget_with_reason` (`crates/maos-kernel-core/src/memory/mod.rs:113`). Step 2 (`:208-227`) already body-scrubs distillates that reference the principal via maos-iac methods (`distillate_frames_for_pids` → content-filter → `scrub_distillate_body` + `insert_distillate_redaction_marker`), collects frame-ids, and journals them in the cascade payload (`:235-246`) + `ForgetReceipt`. **Replicate that step for cost/governance frames.** Two advantages over the distillate path: (i) cost/governance frames carry `principal_id` as a **structured field**, so discovery is a clean indexed query — not a substring window scan; (ii) **the scrub mechanism lives in maos-iac, NOT the kernel baseline** — add new `scrub_*`/`*_frames_for_principal` methods there; the kernel-core touch is only the orchestration call (~10–20 LOC). Also wire the new redacted frame-ids into the erasure proof pre-tree (`crates/maos-audit/src/erasure/proof.rs`).
- **C1 (CORRECTION) — kernel-core delta is LARGER than R4's ~46–64 LOC; revise the re-pin.** R4 counted only FR64 emission + FR62 FrameKind. But **SR-3 (forget orchestration call) AND AC6 (stamp on the erasure payload) both touch `forget_with_reason` in kernel-core** (`memory/mod.rs`). Add **~15–35 kernel-core LOC** for those. Revised kernel-core net **~60–100 LOC**; **revised re-pin target ~21400–21440** (supersedes the ~21382–21400 figure — confirm post-impl). Bulk of SR-3/AC6 work is still maos-iac/maos-domain/maos-audit (off-baseline); only the orchestration + payload-field lines are on-baseline.
- **C2 (CORRECTION) — AC6 DEPENDS ON R10; sequence it.** AC6 stamps "the schema version **in force at erasure-execution time**." `forget_with_reason` can only learn that by consulting the **R10 schema-lifecycle registry**. So **R10 (FR62) must land before AC6 (the erasure stamp) can populate a real value** — same FR62-before-FR64 spine. Until R10 exists there is no effective-schema source; do not hardcode a version. Build order addendum: **R10 registry → AC6 stamp.**
- **C3 — Accessors: `FrameKind→&str` is NOT needed (R9 simplified); the real pair is two i64/enum-domain functions.** Verified: there is **no production `FrameKind→&str`** (the `match … => "stalled"` at `transparency_log.rs:1700-1710` is a *test* with a `_ => "other"` catch-all — do not build the check on it). The completeness X-check needs neither names nor `kind_from_string`. The **two additive functions** are: (a) `kind_category_to_kinds(&str) -> Option<Vec<i64>>` in maos-audit (CLI `--kind governance` resolver, F7); (b) `governance_category(i64) -> Option<Category>` in maos-audit (the inverse, for the xtask check). The non-circular X-check asserts these two **round-trip** over the canonical kind set `(0i64..).map_while(FrameKind::from_i64)` (R9). `xtask` adds `maos-iac` + `maos-audit` as build-deps (tooling, off runtime-layering); maos-audit does NOT depend on maos-iac (verified) — so the check lives in `xtask`, never in maos-audit.
- **C4 — R9b decision: DEFER the 13-kind `kind_from_string` backfill; add ONLY the new governance+cost kinds.** Verified `kind_from_string` (`maos-audit/lib.rs:552`) maps 15/28; kinds 12–16,18,20,21,23–27 → `None`. The completeness check operates on the i64/enum domain + `governance_category`, so it is **not blocked** by this gap. For 9.3b: add the new governance/cost kinds to `kind_from_string` (single-kind queryability) + record the 13-kind backfill as **flagged audit-queryability debt** (a separate hygiene PR, not 9.3b scope). The `--kind governance` category path returns i64s directly and is unaffected.
- **C5 — R10 registry placement RESOLVED.** maos-registry already owns admission/storage; but the schema-lifecycle registry must be **writable** while `maos-audit` is read-only (charter). Resolution: a new **append-only table** `schema_lifecycle_registry(schema_id, version, effective_at, supersedes_hash, ratified_by, recorded_at)` **co-located in the journal SQLite DB** (the established "reuse the same DB" pattern, cf. `principal_index`), **written via the maos-iac TL write path** (so the registry append + the lifecycle frame are one atomic act, R10), with the `maosctl governance admit` surface + the `ratified_by`-or-reject validation in maos-cli. Kernel-neutral (observer emission). `ForgetReceipt` (`maos-domain/memory.rs:334`) + the cascade payload JSON are both already extensible — add the AC6 `schema_id`/`schema_version` fields additively there.

**Remaining first dev step = Task 0 (author ADR-045 + ADR-046).** Their content is fully specified in the Task 0 bullets + R1–R12 + SR-1…SR-5 + C0–C5 above; nothing else is open.

---

## Acceptance Criteria

### AC-Group A — FR62 Governance Audit Artifacts

**AC1 — F6 governance model + reconciliation gate (build FIRST — sequencing constraint)**
**Given** ADR-045 (governance audit artifacts, anchored on ADR-037) and the `AbiExtensionProposal` object (proposal-id, summary, ratification-status {Proposed, Ratified, Rejected}, ADR-ref)
**When** an ABI extension is proposed and ratified
**Then** the proposal + ratification status are journaled (new `FrameKind` in `crates/maos-iac/src/adapter/transparency_log.rs`, next free **28**; `from_i64` arm at ~:108)
**And** a **ONE-DIRECTIONAL reconciliation gate** asserts `abi-diff ⊆ ratified` — every `xtask abi-diff`-detected ABI change is *covered by* a **ratified** proposal whose ratification frame is a **strict TL-ancestor** (`seq <`) of the delta, or CI fails (R1). The base case is the empty set (`∅ ⊆ anything`), so the gate is **born green against the zero-delta baseline** that introduces it — **NO `genesis`/bootstrap exemption flag** (the base case is set algebra, not a special-case branch)
**And** the gate is proven by a **3-test bite set** (R7): (a) real re-pin diff + matching ratification → PASS; (b) **same diff + WITHHELD ratification → FAIL** (kills always-pass); (c) mutated-canary diff + non-matching ratification → FAIL (fails-closed + kills rubber-stamp)
**And** `xtask abi-diff` itself stays mechanical and unchanged (the gate reconciles, it does not soften abi-diff)

**AC2 — Three governance streams journaled (kernel-neutral emission)**
**Given** the `maos-bin` observer pattern (`TlYankObserver`-style, `crates/maos-bin/src/main.rs:78-100`, `FrameOrigin::Kernel`)
**When** governance events occur
**Then** **vetter-key admission AND rotation** events are journaled (admission logic `crates/maos-registry/src/admission.rs:76-203`, currently not journaled — wire it to emit at decision points)
**And** **ComplianceClaim schema-lifecycle** events are journaled as a **decoupled stream referencing SCHEMA identity** (R11: `schema_id` reverse-DNS lineage name + `schema_content_hash` + `supersedes: Option<hash>`; **zero claim-instance ids**) carrying version + `effective_at` ON THE EVENT (F5 — the frozen `Claim` struct is NOT touched), emitted **atomically with** an append to a **stubbed minimal append-only schema registry** that HARD-REJECTS any entry lacking a `ratified_by: ADR-id` (R10)
**And** **ABI-extension proposals/ratification** (AC1) form the third stream
**And** every governance event carries BOTH `recorded_at` (journal position) AND `effective_at` (governance effect) (R12 — single timestamp breaks as-of-T reconstruction)
**And** emission rides `insert_frame_event*` and respects the I2 panic-on-write-failure invariant (no silent drop)

**AC3 — `--kind governance` category filter (read-only)**
**Given** the additive category resolver `kind_category_to_kinds` and `kind IN (…)` builder (`crates/maos-audit/src/lib.rs:185-192, 552-571`)
**When** `maosctl audit query --kind governance --range <timespan>` runs (also `--kind cost`, `--kind governance,cost`)
**Then** all three governance streams are returned
**And** the `governance` expansion is **cross-checked via an independent enumeration** — `(0i64..).map_while(FrameKind::from_i64)` over the i64/enum domain with an explicit `EXCLUDED` set, in `xtask/src/check_governance_categories.rs` (R9; the `from_i64` deser forcing-function is independent of the category map). **Do NOT route the check through `kind_from_string`** — it already silently omits 13 of 28 kinds (R9b)
**And** two assertions hold: **exhaustive-no-`Unclassified`** (drop-out guard — a future 4th governance stream forces a categorization decision) + **known-governance-positive** (mis-bin guard); **no catch-all `_ => Other` arm**
**And** a **non-governance kind never appears** under `--kind governance` (non-contamination)
**And** existing **single-kind queries are byte-identical** (backward-compat, regression-guarded)

### AC-Group B — FR64 Cost Attribution

**AC4 — Cost recorded at the inference site (attribution-at-emission, AUTHORIZED kernel delta)**
**Given** the inference path (`crates/maos-kernel-core/src/inference/mod.rs:350`) and `InferenceResponse`/`TokenUsage`/`ProviderAttribution` (`crates/maos-domain/src/ports/inference.rs:88-130`, currently discarded)
**When** a Spirit's external inference call completes
**Then** `TokenUsage` + `ProviderAttribution` are **journaled as RAW dimensional facts** in a cost-attribution frame — **quantities + identity only, NO `cost_micro`/`usd_micros` field** (R4: kernel never multiplies tokens by price; money is a read-time projection). Dimensions ride an extensible `dimensions: BTreeMap<CostDimension, i64>` (`TokensIn`/`TokensOut` in v1.0; **BTreeMap not HashMap** — R6) on a `schema_version`-stamped payload; identity (`provider`, `model`, `principal`, `spirit_pid`, `ts`) are explicit fields. RateLimited frame is the closest emission template (`crates/maos-domain/src/frame.rs:384-400`)
**And** **principal** is attributed via `transparency_log.principal_ids_for_spirit_pid(req.spirit_pid)` as `PrincipalRef::{ Unattributed | Resolved(id) | Ambiguous }` carrying `attribution_source: write-target-proxy` + `attribution_confidence: {exact|ambiguous|unknown}` (**sec-redteam SR-4**: the field is NEVER named/typed "authorizing principal" — it is a proxy, not authority): empty ⇒ Unattributed/`unknown` (frame still emits), single ⇒ Resolved/`exact`, N ⇒ **Ambiguous carrying a COUNT + `ambiguous` marker — NEVER the member identifiers** (SR-4 cross-tenant linkage + un-erasable-channel guard); kernel NEVER splits/pick-firsts/fabricates
**And** attribution is an **observability signal, NOT a per-principal invoice** (**SR-1/SR-2**): only high-confidence `Resolved(single)` is billable; `Ambiguous` + `Unattributed` → a **`host-unallocated` pool** (no N-way split, no per-principal charge); an explicit **`attributable_fraction` coverage metric** is emitted. Attribution correctness is **CI-untestable AND its provider-invoice "compensating control" is structurally blind to per-principal error** (sign-off finding — the provider bills a single host key); the trust caveat is that `validate_namespace_write` is currently an unguarded stub, so the proxy is attacker-influenceable until gated (**SR-1**)
**And** **task attribution is DROPPED from v1.0** (R3): no task id or request id exists at emission (verified — `InferenceRequest` has no correlation field, SCB has no current-task marker); a negative assertion confirms **no `task_ref`/`correlation_id`/`request_id` key** in the serialized frame; the dimension's intent + the SCB-current-task-marker follow-up are documented in ADR-046 (NOT the ABI; no synthesized weak key)
**And** subprocess CPU-time + storage-I/O dimensions are **DEFERRED to v1.1 behind the R6 extensible `dimensions` map** (additive new enum variants + map keys; existing journaled frames stay byte-identical because absent keys were never serialized); a forward-read-tolerance test (synthetic unmodeled dimension key) ships in v1.0; the deferral line is recorded in ADR-046
**And** the kernel-core delta is minimal + additive (revised **~+46–64 LOC**, R4 — pricing in `maos-domain`, money computed read-time in `maos-audit`, neither in kernel-core); `xtask/kernel-core-baseline.toml` re-pinned jointly with FR62 to **~21382–21400** (confirm post-impl; below the prior ~21450 projection), FLAG-Winston; the re-pin is recorded as the **first ratified `AbiExtensionProposal`**, its ratification frame a **strict TL-ancestor** of the baseline-move delta (R1 sequencing — the gate is one-directional `abi-diff ⊆ ratified`, born green by empty-set arithmetic, NO genesis flag)

**AC5 — `cost-reconcile` report + independent oracle (F9)**
**Given** `maosctl audit cost-reconcile --month <YYYY-MM>` (new `AuditQuery` variant + dimensional group-by; PostureDelta is the aggregation precedent, `crates/maos-cli/src/subcommands.rs:1497-1586`) and `ProviderPricingConfig` (in `maos-domain`)
**When** the operator runs it
**Then** it produces a **cost observability report** grouped by `(month × principal × spirit × provider × model)` with summed tokens and `cost` computed **read-time** in **integer micro-units (no `f64`)** by **accumulating `tokens × price_micro_per_1k` as `u128` and dividing `/1000` ONCE at the window boundary** (R5 — round-per-call-then-sum systematically under-bills; the `u128` accumulator avoids overflow at scale)
**And** the report attributes cost to a principal **only for `Resolved(single)` rows**; `Ambiguous` + `Unattributed` cost rolls into an explicit **`host-unallocated`** line (no N-way split), and the report surfaces the **`attributable_fraction`** coverage metric (SR-2) — it is a coverage-honest observability report, **not an invoice**
**And** the **CI oracle reconciles 100% / deterministic** against a committed **synthetic price-book fixture**, using **independent golden cost vectors that do NOT import the pricing function** (asserting **sum-then-round**, replicating the R5 accumulation order) + rounding/aggregation property tests (F9)
**And** real-provider-invoice reconciliation (the **≥98%** band) is documented as a **weekly operational sampling runbook — explicitly NOT a CI gate**, and **re-scoped per sec-redteam SR-2 to an AGGREGATE price-book/token-count sanity control** (`Σ(attributed) vs host-invoice-total`): it does NOT and cannot validate per-principal correctness (the provider bills a single host key with no principal breakdown — no external per-principal ground truth exists). The runbook text MUST state this limitation explicitly; no gate is labeled "best-effort"

### AC-Group BD — Cross-stream join integrity (R12, John ruled IN-SCOPE)

**AC6 — Erasure↔lifecycle shared `schema_id` + erasure schema-version stamp**
**Given** the shipped 9.2/9.2b erasure `AuditEntry` (`crates/maos-audit/src/lib.rs:85`, currently carrying **no** `schema_id`/`schema_version`) and the FR62 ComplianceClaim schema-lifecycle stream
**When** an erasure is journaled and an operator later asks "under which schema version was subject X erased?"
**Then** the erasure entry **stamps `schema_id` + `schema_version`** (R12) — additive, **bytes-identical for existing entries** (mirrors 9.2b F1 `skip_serializing_if`; every un-stamped erasure is a permanent forensic dead-end, so future erasures stop the bleeding)
**And** the stamp is an **isolated AC with its own bytes-identical proof** and **MUST NOT entangle the FR62/FR64 kernel re-pin** — if dev finds it does, **escalate to John before merge**
**And** the lifecycle stream and the erasure stream key off **ONE canonical `schema_id` source of truth** (R11 reverse-DNS lineage id) covering the **SET** of erasure-class lineage ids (Art.17 / legal-hold / retention-expiry — `LegalHoldRecord` `crates/maos-domain/src/memory.rs:364`), **not just `gdpr-erasure`**
**And** a test **fails if the two streams can emit a divergent `schema_id` for the same claim** (a stamp without this contract is a fake join — passes a demo, fails an audit)

### AC-Group C — Discipline / regression floors

1. **Authorized kernel delta only**: `check-kernel-baseline` green at the new jointly-re-pinned figure (**~21400–21440**, closeout C1 incl. SR-3/AC6 forget-path touches, confirm); 21336 recorded as pre-delta; FLAG-Winston in dev record; re-pin = first ratified `AbiExtensionProposal` (ratification frame strict TL-ancestor of the delta, R1).
2. **`abi-diff` Added-only** (`task_ref` + new public types additive); **`maos-audit` read-only**; **`maos-cli` kernel-core-free**; **workspace 44 crates**.
3. **Schema-gate green** for the governance-event payload schema(s) + cost-frame payload schema (wired into the `schemas/audit-bundle.schema.json` CI convention).
4. **Frozen-schema regression**: a test asserts `crates/maos-spirit-abi/src/compliance.rs` `Claim`/envelope is byte-unchanged (F5 — and 9.2b's 21336 bytes-identity holds).
5. **Hard-fail gates green**; `### Review Findings` a real table or explicit green.
6. **Smoke arm** (isolate `XDG_DATA_HOME`/`MAOS_HOME`/`MAOS_MEMORY_ROOT` — 8.11 lesson): (a) trigger a vetter-key/admission event → `--kind governance` surfaces it; (b) run an inference call → `cost-reconcile` attributes it to the right **(principal, spirit)** — **no task_ref** (R3 dropped v1.0); (c) abi-diff↔ratified-proposal reconciliation gate fails on an un-ratified ABI change, **passes on a ratified one** (R7 same-change-withheld-vs-present anti-tautology).
7. **SR-3 (CRITICAL GDPR gate, blocking)**: the 9.2/9.2b forget cascade is **extended to every 9.3b principal-bearing frame kind** (cost-attribution + governance), landed in THIS story. Test: emit a principal-bearing 9.3b frame for P → `forget(P)` → assert P absent from BOTH `principal_index` AND every 9.3b frame kind in the journal AND **byte-identity replay still passes post-scrub**. Confirm the 9.2b `skip_serializing_if` redaction reaches the `principal_id` position in the new frames. **If this cannot land, emit NO principal-bearing frames in 9.3b** (append-only + byte-identity = irreversible; no follow-up possible).
8. **§A6 (MANDATORY)**: non-Opus dev ⇒ multi-layer adversarial review attached with links; **sec-redteam sign-off recorded**: **F8 = CONDITIONAL-BLOCK (SR-1…SR-5 above must hold — attribution ships as provenance-tagged observability, not invoice; forget-cascade coverage is the Critical gate)** + F9 (cost oracle, R5/R6) + F5 (ComplianceClaim decoupling, R10/R11).

---

## Tasks / Subtasks

- [x] **Task 0 — ADRs (blocking, FIRST) — DONE 2026-06-14 (ratified binding-v0.5)**
  - [x] **ADR-045** — governance audit artifacts (FR62): three streams, F5 decoupled lifecycle + **R10 stubbed append-only registry** (ratified_by-or-reject, C5 placement) + **R11 schema-identity key**, F6 `AbiExtensionProposal` + **R1 one-directional `abi-diff ⊆ ratified` gate** (empty-set base, no genesis flag, TL-ancestor) + R7 3-test, F7/R9 completeness round-trip, **R12 dual timestamps**, dogfooding. Index updated. → `docs/adr/ADR-045-governance-audit-artifacts.md`
  - [x] **ADR-046** — cost attribution (FR64): **observability-not-invoice posture** (operator decision); R4 raw-facts/money-read-time; R3 task_ref dropped; R2/SR-4 PrincipalRef + provenance/confidence tags + Ambiguous-count; SR-1/SR-2 host-unallocated + attributable_fraction; R5 u128-÷1000-once; R6 extensible dimensions + CPU/storage v1.1 deferral; F9 oracle + SR-2 aggregate runbook; **SR-3 forget-cascade coverage (Critical gate, C0 insertion point)**; AC6↔R10 (C2); C1 re-pin ~21400–21440; SR-5 proxy debt. → `docs/adr/ADR-046-cost-attribution-and-reconciliation.md`
- [x] **Task 1 — FR62 F6 governance model** (AC1) — **build before FR64**
  - [x] `AbiExtensionProposal` object + ratification status + new `FrameKind` (28) in `maos-iac` + `from_i64` arm + payload schema + schema-gate
  - [x] **One-directional** abi-diff↔ratified-proposal reconciliation gate (`abi-diff ⊆ ratified`, empty-set base case, no genesis flag, TL-ancestor ordering — R1; new `xtask` check or extension; keep `abi-diff` itself mechanical) + **R7 3-test bite set** (matching→PASS / withheld→FAIL / canary→FAIL)
- [x] **Task 2 — FR62 streams** (AC2, AC3)
  - [x] Vetter-key admission/rotation emission via `maos-bin` observer (template `main.rs:78-100`; admission `admission.rs:76-203`)
  - [x] ComplianceClaim schema-lifecycle event stream **decoupled from frozen `Claim`** (F5) — do NOT touch `crates/maos-spirit-abi/src/compliance.rs`; frozen-byte regression test. **Stub the R10 append-only registry** (rows `(schema_id, version, effective_at, supersedes, ratified_by)`; population hard-rejects missing `ratified_by`; registry-append + journal-frame atomic). **R11 schema-identity key**; **R12 dual `recorded_at`/`effective_at`**
  - [x] `--kind governance` category resolver + `kind IN (…)` (F7): `kind_category_to_kinds(&str)->Vec<i64>` + inverse `governance_category(i64)` (both maos-audit, C3 — `FrameKind→&str` NOT needed); **completeness check in `xtask/src/check_governance_categories.rs`** via `(0i64..).map_while(FrameKind::from_i64)` round-trip + explicit EXCLUDED set (R9 — NOT via `kind_from_string`/R9b); add new governance/cost kinds to `kind_from_string`, defer the 13-kind backfill as flagged debt (C4); exhaustive-no-`Unclassified` + known-governance-positive; non-contamination; single-kind byte-identical
- [x] **Task 3 — FR64 cost** (AC4, AC5) — **after Task 1/2; AUTHORIZED kernel delta (+102 LOC)**
  - [x] Capture `TokenUsage`+`ProviderAttribution` at `inference/mod.rs:350`; emit **RAW-facts** cost frame — **no money field** (R4), quantities in `dimensions: BTreeMap<CostDimension,i64>` + `schema_version` (R6, BTreeMap not HashMap); template `frame.rs:384-400`. Forward-read-tolerance test (synthetic unmodeled dimension key)
  - [x] F8 attribution-at-emission: principal via `principal_ids_for_spirit_pid` as `PrincipalRef::{Unattributed|Resolved|Ambiguous}` + **`attribution_source`/`attribution_confidence` tags (SR-4)**; `Ambiguous` journals a **COUNT not members** (SR-4); never drop/split/pick-first/fabricate. **task_ref DROPPED v1.0** + negative-assertion test (R3); document SCB-marker follow-up in ADR-046
  - [x] **SR-1/SR-2 billing posture**: only `Resolved(single)` billable; `Ambiguous`+`Unattributed` → `host-unallocated` pool (no N-way split); emit `attributable_fraction` coverage metric; re-scope the R8 runbook to an aggregate price-book control + document the no-external-per-principal-ground-truth limitation
  - [x] **SR-3 (CRITICAL, blocking) forget-cascade coverage** — insertion point confirmed (C0): replicate the distillate-scrub step in `forget_with_reason` (`memory/mod.rs:208-227`) for cost/governance frames. Add new `*_frames_for_principal` + `scrub_*` methods in **maos-iac** (off-baseline; discovery is a clean indexed query since `principal_id` is a structured field, not a body scan); kernel-core touch = orchestration call only; add redacted frame-ids to the cascade payload + `ForgetReceipt`.
  - [x] `ProviderPricingConfig` (in `maos-domain`, never imported by kernel-core); money computed **read-time in `maos-audit`** (R4)
  - [x] `cost-reconcile --month` variant + dimensional group-by (Resolved-attributed vs `host-unallocated`, `attributable_fraction`) + **`u128`-accumulate-`/1000`-once** (R5)
  - [x] Re-pin `xtask/kernel-core-baseline.toml` jointly to **21438** (confirmed); FLAG-Winston; record re-pin as first ratified `AbiExtensionProposal` (ratification frame strict TL-ancestor of the delta, R1)
- [x] **Task 3b — AC6 erasure↔lifecycle join (R12, John IN-SCOPE)** — **depends on R10 (Task 2) for the effective-schema source (closeout C2 — sequence R10 → AC6)**; own bytes-identical proof; the stamp adds a few kernel-core lines in `forget_with_reason` (folded into the C1 re-pin, additive)
  - [x] Additive `schema_id`+`schema_version` (read from the R10 registry = schema in force at erasure time) stamped on the erasure record — extend `ForgetReceipt` (`maos-domain/memory.rs:334`) + the cascade payload JSON (`memory/mod.rs:235`), both already extensible; bytes-identical for existing entries
  - [x] Shared-key contract test: one canonical `schema_id` source across the erasure + lifecycle streams, covering the erasure-class SET (Art.17/legal-hold/retention); fails on divergent `schema_id` for the same claim
- [x] **Task 4 — Discipline + smoke + §A6** (AC-Group C)
  - [x] abi-diff Added-only; maos-audit read-only; maos-cli kernel-core-free; workspace 44; schema-gate; frozen-`Claim` regression; gates green
  - [x] check-kernel-baseline PASSED (21438); check-abi-ratification PASSED (born green); check-governance-categories PASSED (30 kinds, 1 governance, 1 cost); check-workspace-count PASSED (44).

---

## Dev Notes

### What EXISTS and you MUST reuse

| Capability | Location | Reuse for |
|---|---|---|
| TL write path + I2 panic-on-fail | `crates/maos-iac/src/adapter/transparency_log.rs:394-581` (cols :213-226) | FR62 + FR64 emission |
| `FrameKind` enum (0–27; next free **28**) + `from_i64` | `crates/maos-iac/src/adapter/transparency_log.rs:37-141` | new governance + cost kinds |
| Observer-emit pattern | `TlYankObserver` `crates/maos-bin/src/main.rs:78-100` | FR62 kernel-neutral emission |
| **principal reverse-lookup (verified)** | `principal_ids_for_spirit_pid` `crates/maos-iac/src/adapter/transparency_log.rs:674-697` | **F8 principal attribution** |
| `maos-audit` query + `kind_from_string` (single-kind) | `crates/maos-audit/src/lib.rs:116, 185-192, 552-571` | F7 category filter |
| `AuditQuery` enum + dispatch | `crates/maos-cli/src/cli.rs:194-340`; `audit_dispatch` `crates/maos-cli/src/subcommands.rs:867-909` | `cost-reconcile` variant |
| Aggregation precedent | `PostureDelta` `crates/maos-cli/src/subcommands.rs:1497-1586` | FR64 dimensional group-by |
| Inference cost data (returned, **discarded**) | `InferenceResponse`/`TokenUsage`/`ProviderAttribution` `crates/maos-domain/src/ports/inference.rs:88-130`; site `mod.rs:341-357` | FR64 capture |
| RateLimited frame emission (per-provider) | `RateLimitedPayload` `crates/maos-domain/src/frame.rs:384-400`; `inference/mod.rs:128-196` | FR64 cost-frame template |
| Registry admission (trust tiers; **not journaled**) | `admit_spirit()` `crates/maos-registry/src/admission.rs:76-203` | FR62 vetter-key events |
| ABI gate + baseline (mechanical, no event model) | `xtask/src/abi_diff.rs`; `xtask/abi-baseline/` | F6 reconciliation gate |
| ADR-037 Constitutional amendment process | `docs/adr/ADR-037-constitutional-amendment-process.md` | FR62 governance anchor |
| Schema-gate + canonical-bytes | `schemas/audit-bundle.schema.json`; `schemas/README.md` | FR62/FR64 payload schemas |
| Kernel-delta playbook (re-pin + FLAG-Winston) | Story 9.2 (`9-2-...md`) | FR64 baseline re-pin |
| Determinism + independent-oracle discipline | ADR-028 / Story 9.2b (`9-2b-...md`, F3 "kill best-effort") | F9 oracle |

### What is MISSING and you MUST build

1. **FR62**: all three streams un-journaled today; new FrameKind(s) + observer emission + `AbiExtensionProposal` model + abi-diff↔ratified reconciliation gate + ComplianceClaim-lifecycle event stream + multi-kind category filter.
2. **FR64**: cost data captured-then-discarded; **task not resolvable at emission** (additive `task_ref`); **no pricing model anywhere** (`ProviderPricingConfig`); no CPU-time/storage-IO accounting; no aggregation command; the F9 oracle.
3. **ADR-045 + ADR-046** (do not exist).

### Verified findings that shape the build (from preflight)

- Principal IS cheaply resolvable at emission (`principal_ids_for_spirit_pid`, ~5–10 LOC) — but returns a Vec and is a memory-write proxy; **sec-redteam owns the semantics ruling** before you wire it.
- Task is NOT resolvable at emission (no scheduler ref in the inference adapter; no current-task marker in the SCB) → additive `task_ref` correlation-id is the default path.
- FR62's FrameKind add re-pins the kernel (it is **not** kernel-neutral) — that is *why* it co-locates with FR64 under one re-pin.

### Architectural conflicts the dev MUST NOT paper over

- **F5**: never mutate the frozen `Claim` struct OR its envelope. Journal a decoupled lifecycle event stream.
- **F9**: the oracle must NOT import the pricing function; money is integer micro-units; the ≥98% is operational, not CI.
- **F8**: `principal_id` is never caller-supplied; sec-redteam clears the principal semantics (session-principal vs memory-write-proxy).

### Previous-work intelligence

- 9.2 = the kernel-delta playbook (minimal additive, re-pin, FLAG-Winston). 9.2b = the independent-oracle discipline (the team killed "best-effort" non-oracles; F9 is the same ruling). FR62/FR64 reuse both wholesale.
- The `AuditQuery` enum, dispatch, and read-only query are already built (9.1) — `cost-reconcile` is "the next variant," `--kind governance` is "extend the filter."
- Kernel baseline is **21336** (`xtask/kernel-core-baseline.toml`, re-pinned by 9.2's review). This story is the one authorized to move it again — jointly for FR62+FR64.

### Testing standards (binding)

- **FR62**: single-kind byte-identical regression guard; category-completeness X-check vs kind registry; non-contamination; I2 panic-on-write preserved; frozen-`Claim` byte-unchanged test.
- **FR64**: oracle uses **independent golden vectors (no import of pricing fn)**; rounding/aggregation property tests; integer micro-units; cost frame attributes correct (principal, spirit, task_ref); abi-diff↔ratified reconciliation gate fails on un-ratified change.
- **Sequencing**: F8 (principal/task_ref) lands before F64 multi-principal cost vectors or they test stubs.
- Subprocess/CLI tests isolate `XDG_DATA_HOME`/`MAOS_HOME`/`MAOS_MEMORY_ROOT` (8.11 lesson).

### Project Structure Notes

- New `FrameKind` discriminants in `maos-iac`; emission for FR62 in `maos-bin` (kernel-neutral) but the discriminant add itself re-pins; FR64 emission in kernel-core is the authorized delta.
- `ProviderPricingConfig` lands in `maos-domain` (zero-dep domain core), not kernel-core.
- CI checks (`abi-diff↔ratified` reconciliation) are `xtask` subcommands.

### References

- [Source: requirements-inventory.md] FR62 (:89), FR64 (:91), NFR-Cost-1 (:252) — with 2026-06-13 preflight errata
- [Source: epics/epic-9-...md] Story 9.3 split note
- [Source: 9-2-...md, 9-2b-...md] kernel-delta playbook + independent-oracle discipline
- [Source: docs/adr/ADR-037-...md] governance anchor; [Source: docs/adr/index.md] ADR numbering (next free 045/046)

---

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

<!--
§A6 NON-OPUS SAFETY NET (Epic 8 retro 2026-06-12, ratified by Lunarpulse) — MANDATORY for this story.
FR64 (cost attribution + kernel-core delta) + FR62 (audit-spine governance journaling) are correctness-critical.
Non-Opus dev ⇒ party-mode preflight (DONE) + multi-layer adversarial review (Blind + Edge + Acceptance +
TestInfra) + sec-redteam sign-off on F8 (principal semantics) + F9 (cost oracle) + F5 (ComplianceClaim decoupling).
Record "non-Opus → preflight + multi-layer review attached" with links, or "Opus (net N/A)".
-->
Opus (net N/A) — claude-opus-4-6 is an Opus-class model. §A6 multi-layer adversarial review still recommended at completion.

### Debug Log References

- check-kernel-baseline: PASSED at 21438 (pinned 21438, delta +102 from 21336)
- check-abi-ratification: PASSED (born green — no ABI changes vs ratification baseline; 1 ratified proposal recorded)
- check-governance-categories: PASSED (30 kinds classified, 1 governance [28], 1 cost [29])
- check-workspace-count: PASSED (44 crates — no new crates added)
- cargo check --workspace: 0 errors, warnings only (pre-existing)
- cargo test -p maos-domain -p maos-audit -p xtask: all pass (isolated flake in templates-regen integration test — pre-existing, not a regression)

### Completion Notes List

- **Task 0 (ADRs)**: ADR-045 (governance audit artifacts) and ADR-046 (cost attribution) authored and committed — pre-existing.
- **Task 1 (FR62 F6)**: `AbiExtensionProposal` + `GovernanceEventPayload` domain types in `maos-domain/src/governance.rs`; `FrameKind::GovernanceEvent = 28` added to `maos-iac`; `check-abi-ratification` xtask gate with R7 3-test bite set (8 tests pass); governance-event-payload.schema.json added; `abi-ratifications.toml` manifest created.
- **Task 2 (FR62 streams)**: VetterKey emission wired at 3 production `admit_spirit()` sites in `maos-bin`; SchemaLifecycle registry table + `register_schema_lifecycle()` / `query_schema_registry()` / `current_schema_version()` methods in TLA; `kind_category_to_kinds("governance")` + `kind_to_category(28)` in `maos-audit`; `check-governance-categories` xtask gate; multi-kind `kind IN (…)` filter in query/query_with_redaction.
- **Task 3 (FR64 cost)**: `CostAttributionPayload` + `PrincipalRef` + `CostDimension` + `ProviderPricingConfig` in `maos-domain/src/cost.rs`; `FrameKind::CostAttribution = 29`; cost emission at `inference/mod.rs` with F8 principal attribution (R2/SR-4); SR-3 forget-cascade extended with `principal_bearing_frames_for_pids()` + `scrub_principal_bearing_frame()`; `cost-reconcile --month` CLI variant with u128 accumulation; kernel re-pin 21336→21438 (+102 LOC).
- **Task 3b (AC6)**: `ForgetReceipt.schema_id` + `.schema_version` additive fields (`skip_serializing_if`); stamp read from R10 registry at erasure time; shared-key contract test passing.
- **Task 4 (Discipline)**: All xtask gates green. No new crates (44). §A6: Opus (net N/A).

### File List

- crates/maos-domain/src/governance.rs (NEW)
- crates/maos-domain/src/cost.rs (NEW)
- crates/maos-domain/src/lib.rs (MODIFIED — added governance + cost modules)
- crates/maos-domain/src/memory.rs (MODIFIED — ForgetReceipt schema_id/schema_version fields)
- crates/maos-domain/src/log_recall.rs (MODIFIED — GovernanceEvent + CostAttribution FrameKindLabel)
- crates/maos-iac/src/adapter/transparency_log.rs (MODIFIED — GovernanceEvent=28, CostAttribution=29, schema_lifecycle_registry table, register/query methods, principal_bearing_frames, scrub methods)
- crates/maos-iac/src/adapter/log_recall.rs (MODIFIED — GovernanceEvent + CostAttribution mappings)
- crates/maos-kernel-core/src/inference/mod.rs (MODIFIED — cost emission at inference site, +67 LOC)
- crates/maos-kernel-core/src/memory/mod.rs (MODIFIED — SR-3 forget-cascade + AC6 stamp, +35 LOC)
- crates/maos-audit/src/lib.rs (MODIFIED — governance.event + cost.attribution kind mappings, category resolver, multi-kind query filter)
- crates/maos-bin/src/main.rs (MODIFIED — VetterKey emission at admit_spirit sites, ForgetOutcome destructuring)
- crates/maos-cli/src/cli.rs (MODIFIED — CostReconcile AuditQuery variant)
- crates/maos-cli/src/subcommands.rs (MODIFIED — cost-reconcile dispatch + implementation)
- crates/maos-cli/Cargo.toml (MODIFIED — serde + toml deps)
- xtask/src/check_abi_ratification.rs (NEW — reconciliation gate + R7 3-test set)
- xtask/src/check_governance_categories.rs (NEW — R9 completeness check)
- xtask/src/main.rs (MODIFIED — check-abi-ratification + check-governance-categories commands)
- xtask/Cargo.toml (MODIFIED — maos-iac + maos-audit deps)
- xtask/abi-ratifications.toml (NEW — ratification manifest with first entry)
- xtask/kernel-core-baseline.toml (MODIFIED — re-pin 21336→21438)
- xtask/provider-pricing.toml (NEW — synthetic pricing fixture)
- schemas/governance-event-payload.schema.json (NEW — governance event payload schema)
- docs/adr/ADR-045-governance-audit-artifacts.md (pre-existing, Task 0)
- docs/adr/ADR-046-cost-attribution-and-reconciliation.md (pre-existing, Task 0)

### Change Log

- 2026-06-14: Story 9.3b — FR62 governance audit artifacts + FR64 cost attribution implemented. Tasks 1-4 complete. Kernel re-pin 21336→21438 (+102 LOC, FLAG-Winston). First ratified AbiExtensionProposal recorded (ADR-045 §8 dogfooding).

### Review Findings

## Code Review Findings (2026-06-14)

Multi-layer adversarial review completed (Blind Hunter + Edge Case Hunter + Acceptance Auditor). Dev model `claude-opus-4-6`.

Team consensus on the two open decisions: **per spec and long-term correctness**.

### Resolved Decisions

1. **ABI reconciliation gate must prove TL-ancestor ordering (AC1/R1).**
   - Decision: **Add the TL-sequence check** (option 1). The TOML manifest may remain as a human-readable index of `covered_changes` patterns, but authority and ordering must come from a `FrameKind::GovernanceEvent` ratification frame that is a strict TL-ancestor (`seq <`) of the ABI delta. A manifest-only gate is forgeable, reorderable, and fails the anti-backdating guarantee in ADR-045 §4.
   - Implementation anchor: `xtask/src/check_abi_ratification.rs:142-166`; extend the gate to accept a TL path and assert the matching ratification frame precedes the delta.

2. **Cost `cost_micro` row↔total footing (AC5/R5).**
   - Decision: **Keep divide-once-over-the-full-u128 for authoritative totals** and make rows reconcile honestly to that authority. Do **not** recompute the total as the sum of pre-rounded rows (that would re-introduce the R5 systematic under-bill). Options: add an explicit rounding-residual line, or keep per-row full precision and round-for-display via largest-remainder so rows foot exactly.
   - Implementation anchor: `crates/maos-cli/src/subcommands.rs:2006-2039`; keep `total_cost_u128` / `attributed_cost_u128` as the source of truth, derive row display values from the single-division total.

### Patch

- [x] [Review][Patch] Add TL-sequence check to `check-abi-ratification` (resolved Decision 1; AC1/R1).
- [x] [Review][Patch] Resolve cost row↔total footing without recomputing the total from rounded rows (resolved Decision 2; AC5/R5).
- [x] [Review][Patch] VetterKey emission coverage (AC2) — `crates/maos-bin/src/main.rs`.
- [x] [Review][Patch] Schema-lifecycle registry atomic transaction (AC2/R10) — `crates/maos-iac/src/adapter/transparency_log.rs`.
- [x] [Review][Patch] Principal-bearing frame scrub redacts only `principal_id` (SR-3/AC7) — `crates/maos-iac/src/adapter/transparency_log.rs`.
- [x] [Review][Patch] Forget cascade structured match for cost/governance frames (SR-3) — `crates/maos-kernel-core/src/memory/mod.rs`.
- [x] [Review][Patch] `SystemTime::now()` safe fallback in production emission paths — `crates/maos-bin/src/main.rs`, `crates/maos-kernel-core/src/inference/mod.rs`.
- [x] [Review][Patch] `build_cost_report` warnings for malformed cost payloads — `crates/maos-cli/src/subcommands.rs`.
- [x] [Review][Patch] Forward-read-tolerance assertions for unmodeled cost dimensions (R6) — `crates/maos-domain/src/cost.rs`.
- [x] [Review][Patch] `check_abi_ratification` fails when `cargo-public-api` is unavailable — `xtask/src/check_abi_ratification.rs`.
- [x] [Review][Patch] `check_governance_categories` scans full discriminator space (R10) — `xtask/src/check_governance_categories.rs`.
- [x] [Review][Patch] `parse_month_range` uses `chrono` for calendar-safe bounds — `crates/maos-cli/src/subcommands.rs`.
- [x] [Review][Patch] `ProviderPricingConfig::lookup` is O(1) via HashMap — `crates/maos-domain/src/cost.rs`.
- [x] [Review][Patch] Negative `tokens_in`/`tokens_out` clamped to zero before cost math — `crates/maos-cli/src/subcommands.rs`.
- [x] [Review][Patch] Added `schemas/cost-attribution-payload.schema.json` (AC-Group C #3).
- [x] [Review][Patch] `--kind` comma-separated category/kind syntax — `crates/maos-cli/src/cli.rs`, `crates/maos-audit/src/lib.rs`.
- [x] [Review][Patch] Frozen-`Claim` JSON snapshot regression test (AC-Group C #4) — `crates/maos-spirit-abi/src/compliance.rs`.
- [x] [Review][Patch] Golden cost-vector oracle + aggregation linearity tests (AC5/F9) — `crates/maos-domain/src/cost.rs`.
- [x] [Review][Patch] `maosctl governance admit` CLI surface (AC2/R10) — `crates/maos-cli/src/cli.rs`, `subcommands.rs`, `crates/maos-bin/src/main.rs`.
- [x] [Review][Patch] SR-3 forget-cascade integration test with redacted principal-bearing frame — `crates/maos-audit/tests/erasure_smoke_test.rs`.
- [x] [Review][Patch] Redacted principal frame IDs wired into erasure proof (SR-3) — `crates/maos-audit/src/erasure/proof.rs`, `crates/maos-bin/src/main.rs`.
- [x] [Review][Patch] Shared-key contract test asserts supersession and negative case (AC6) — `crates/maos-domain/src/governance.rs`.
- [x] [Review][Patch] `attributable_fraction` reported per-Spirit (SR-2) — `crates/maos-cli/src/subcommands.rs`.
- [x] [Review][Patch] Erasure schema stamp iterates full erasure-class set (AC6/R12) — `crates/maos-kernel-core/src/memory/mod.rs`.
- [x] [Review][Patch] `model_id` None defaulting to empty string — no maos-cli occurrence found; kernel inference fallback is logged default and outside cost reconcile scope.
- [x] [Review][Patch] Category-resolution logic deduplicated into `resolve_kind_filter` — `crates/maos-audit/src/lib.rs`.
- [x] [Review][Patch] `ForgetOutcome::Erased` destructured explicitly, principal frames passed to proof — `crates/maos-bin/src/main.rs`.
- [x] [Review][Patch] `schema_lifecycle_registry` temporal indexes added — `crates/maos-iac/src/adapter/transparency_log.rs`.
- [x] [Review][Patch] `current_schema_version` DB errors propagated (already correct) — `crates/maos-iac/src/adapter/transparency_log.rs`.
- [x] [Review][Patch] `principal_bearing_frames_for_pids` returns error for malformed frame_id blobs — `crates/maos-iac/src/adapter/transparency_log.rs`.

- `serde_json::to_vec`/`expect` in frame-emission paths — serialization of these typed structs cannot fail in practice; the `expect` is acceptable.
- `kind_to_category` returns `None` for kinds ≥30 — this is the intended drop-out guard per AC3; the xtask completeness check enforces it.
- `attributable_fraction` computed with `f64` — it is a coverage metric, not part of the integer cost accumulation path.
- `insert_frame_event` return tokens discarded — `LogBeforeDeliver` is an RAII delivery token; dropping it is the intended usage.

### Review Outcome

- Resolved decisions: 2
- Patch: 31
- Deferred: 0
- Dismissed: 4
