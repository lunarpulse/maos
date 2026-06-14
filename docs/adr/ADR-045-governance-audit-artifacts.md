# ADR-045: Governance Audit Artifacts (FR62)

## Status

**Accepted** — ratified at Story 9.3b Task 0 (2026-06-14). binding-v0.5. Supersedes the 2026-06-13 preflight stub; folds in Round-2 rulings (R1, R7, R9, R10, R11, R12) and pre-Task-0 closeouts (C3, C4, C5). Anchored on [ADR-037](ADR-037-constitutional-amendment-process.md) (Constitutional amendment process). Implementation pending (Story 9.3b). Cross-references [ADR-046](ADR-046-cost-attribution-and-reconciliation.md) (the kernel re-pin is dogfooded through this ADR's F6 model; the forget-cascade obligation in ADR-046 §SR-3 also covers this ADR's principal-bearing governance frames).

## Context

FR62 requires the substrate to expose audit-queryable artifacts for governance: (a) vetter-key admission/rotation events, (b) ABI-extension proposals and their ratification status, (c) ComplianceClaim schema versions and their effective dates. Today:

- **None of these streams journal.** Admission decisions (`crates/maos-registry/src/admission.rs`) are not emitted; ABI governance is purely a mechanical CI gate (`xtask abi-diff`) with no provenance record; ComplianceClaim schema lifecycle has no representation at all.
- **The `ComplianceClaim` struct is frozen** (binding-v0.1, `crates/maos-spirit-abi/src/compliance.rs`). Story 9.2b's HARD byte-identity replay (ADR-028) depends on that frozen serialization (the 21336 bytes-identity construction).
- **No schema registry exists.** The frozen `Claim` is a *type*, not a registry — it carries no notion of version, effective-date, or supersession. Verified at preflight: there is no source of truth that could author a "schema v2 supersedes v1" decision.

## Decision

### 1. Three governance streams journaled to the Transparency Log

New `FrameKind` discriminant(s) in `maos-iac` (next free = **28**), emitted **kernel-neutrally** through the `maos-bin` observer pattern (`TlYankObserver`-style, `FrameOrigin::Kernel`), respecting the **I2 panic-on-write** invariant (no silent drop). The discriminant addition itself re-pins the kernel-core baseline (it is *not* kernel-neutral) and is therefore made **jointly with FR64** under one authorization (see Consequences).

The three streams: **vetter-key admission/rotation** (wire emission at the `admit_spirit()` decision points), **ComplianceClaim schema-lifecycle** (§2), **ABI-extension proposals/ratification** (§4).

### 2. F5 — ComplianceClaim schema lifecycle is a DECOUPLED event stream

Schema lifecycle (version / effective-date / supersedes / ratified-by) is a property of **the schema**, owned by a registry (§3) — never of a claim instance or its envelope. The frozen `Claim` struct and its envelope are **never mutated** (a `#[serde(default)]` field on either is a freeze violation in additive costume). The lifecycle is journaled as a **decoupled event** referencing schema identity (§5). This preserves Story 9.2b's 21336 bytes-identity construction.

### 3. R10 — schema-lifecycle REGISTRY is the authority (stubbed minimal, this story)

A decoupled event stream is meaningless if nothing authors the decisions it records ("a paper trail with no author"). No registry exists today, so this story **stubs a minimal append-only registry** whose sole job is to be the source of truth that emits lifecycle events:

- **Rows:** `(schema_id, version, effective_at, supersedes_hash, ratified_by, recorded_at)`.
- **Authority chain:** ADR ratification (human) → an explicit authorized governance action (`maosctl governance admit`) that **HARD-REJECTS any entry lacking a `ratified_by: ADR-id`** → registry append → lifecycle frame journaled. No ADR ⇒ no entry ⇒ no event. That single constraint makes the stream non-forgeable.
- **Atomicity:** the registry append and the journal frame are **one governance act** — the journal can never claim a lifecycle the registry does not hold.
- **Placement (closeout C5):** an append-only table **co-located in the journal SQLite DB** (the established "reuse the same DB" pattern, cf. `principal_index`), **written via the maos-iac TL write path** (so the append + the frame are atomic). The `admit` surface + `ratified_by`-or-reject validation live in `maos-cli`. Kernel-neutral. `maos-audit` stays read-only (it never writes the registry).

### 4. F6 / R1 — `AbiExtensionProposal` object + a ONE-DIRECTIONAL reconciliation gate

A proposal carries `(proposal-id, summary, ratification-status {Proposed, Ratified, Rejected}, ADR-ref)`. `xtask abi-diff` stays the **mechanical truth** (did the ABI change) and is **not softened**. The journal is the human-decision provenance. The **new CI gate reconciles them**, one-directionally:

> **`abi-diff ⊆ ratified`** — every abi-diff-detected ABI change must be *covered by* a **ratified** proposal whose ratification frame is a **strict Transparency-Log-sequence ancestor** (`seq <`) of the delta, or CI fails.

- The base case is the empty set (`∅ ⊆ anything`), so the gate is **born green against the zero-delta baseline** on the commit that introduces it (introducing the gate moves no ABI). **There is NO `genesis`/bootstrap exemption flag** — the base case is set algebra, not a special-case branch (an exemption flag becomes a permanent loophole).
- Ordering is proven on the **TL frame sequence, not git history** (squash-merge erases git order; TL seq is monotonic and tamper-evident).
- No voting UI in scope.

### 5. R11 — schema reference key references SCHEMA identity, never an instance

Three-part key on every lifecycle event:
- **`schema_id`** — stable, version-*independent* reverse-DNS lineage name (e.g. `compliance.claim.gdpr-erasure`), **not** a Rust type path/discriminant (those drift on refactor and break the join). The correlation key.
- **`schema_content_hash`** — per-version fingerprint. The integrity anchor.
- **`supersedes: Option<schema_content_hash>`** — references the prior version's *hash* (a verifiable chain v3→v2→v1), not its number.

The event references **schema identity only** — zero claim-*instance* ids (upholds the F5 boundary).

### 6. F7 / R9 — `--kind governance` category filter (read-only, completeness-guarded)

Additive resolver `kind_category_to_kinds(&str) -> Option<Vec<i64>>` (`"governance" → vec![…]`, multi-kind-capable `--kind governance,cost` from day one) + a `kind IN (…)` builder in `maos-audit`. Single-kind callers stay byte-identical (the `kind = ?` path is untouched). The **completeness cross-check** is the anti-under-reporting guard (silent omission is the worst audit failure mode) and must use an **independent enumeration**, not a hand-list:

- Independent source = `(0i64..).map_while(FrameKind::from_i64)` — `from_i64` is the *deserialization* forcing function, maintained independently of the category map.
- The check (in **`xtask/src/check_governance_categories.rs`**, sibling to `check_error_catalog.rs`) asserts `kind_category_to_kinds` and its inverse `governance_category(i64) -> Option<Category>` **round-trip** over that set, with two assertions: **exhaustive-no-`Unclassified`** (drop-out guard) + **known-governance-positive** (mis-bin guard). **No catch-all `_ => Other` arm.**
- Closeout C3: a `FrameKind → &str` accessor is **not** needed (the check works on the i64 domain). Closeout C4: the pre-existing `kind_from_string` gap (15/28 kinds mapped) is **not** on this path; add only the new governance/cost kinds for single-kind queryability and record the 13-kind backfill as flagged debt (separate hygiene PR).

### 7. R12 — dual timestamps on every governance event

Every governance event carries **both** `recorded_at` (monotonic journal position) **and** `effective_at` (when the decision takes governance effect). They genuinely differ (an ADR ratified June 14 can make a schema effective July 1); a single timestamp makes as-of-T incident reconstruction impossible.

### 8. Dogfooding

Story 9.3b's own FR64 kernel re-pin is recorded as the **first ratified `AbiExtensionProposal`** through this model (its ratification frame a strict TL-ancestor of the baseline-move delta, §4).

## Consequences

- FR62's `FrameKind` addition re-pins the kernel-core baseline — re-pinned **jointly with FR64** under one authorization (FLAG-Winston); see [ADR-046](ADR-046-cost-attribution-and-reconciliation.md) §Consequences for the revised figure (~21400–21440, which includes this ADR's emission lines).
- The frozen ComplianceClaim guarantee holds; binding-v0.2+ schema evolution flows through ADR-037 and is recorded as a lifecycle event in the §3 registry.
- A new durable authority (the schema-lifecycle registry) becomes the source of truth for "which ComplianceClaim schema version is effective." [ADR-046](ADR-046-cost-attribution-and-reconciliation.md) AC6 (the erasure schema-version stamp) **depends on this registry** (closeout C2) — sequence R10 (this ADR) before AC6.
- **Principal-bearing governance frames are subject to the GDPR forget obligation.** Any governance frame that embeds a `principal_id` (e.g. an admission event naming a subject) is covered by the [ADR-046](ADR-046-cost-attribution-and-reconciliation.md) §SR-3 forget-cascade extension — it must be erasable, or it must not be emitted.

## Gate

- `xtask` **abi-diff↔ratified-proposal reconciliation gate**, proven by the **R7 3-test bite set**: (a) real ABI change + matching ratification → PASS; (b) **same change + withheld ratification → FAIL** (kills always-pass); (c) mutated-canary change + non-matching ratification → FAIL (fails-closed + kills rubber-stamp).
- `--kind governance` **completeness** (round-trip over `from_i64`) + **non-contamination** + **single-kind backward-compat** (byte-identical) tests.
- **Frozen-`Claim` byte-unchanged** regression test.
- **Schema-gate** for the governance-event payload schema(s) (wired into `schemas/audit-bundle.schema.json`), including the dual-timestamp and R11 key fields.

## Sign-off required

Winston (ADR-037 owner) + Mary (ComplianceClaim schema co-author) on the **F5 decoupling**, the **R10 registry authority**, and the **R11 schema-identity key**. Recorded at Story 9.3b Task 0.
