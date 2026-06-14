# ADR-045: Governance Audit Artifacts (FR62)

## Status

**Proposed** — stub authored at Story 9.3b party-mode preflight (2026-06-13, Winston·Murat·John·Amelia). To be ratified/finalized at Story 9.3b Task 0 before implementation. Anchored on [ADR-037](ADR-037-constitutional-amendment-process.md) (Constitutional amendment process).

## Context

FR62 requires the substrate to expose audit-queryable artifacts for governance: (a) vetter-key admission/rotation events, (b) ABI-extension proposals and their ratification status, (c) ComplianceClaim schema versions and their effective dates. None of these streams journal today; the binding-v0.1 `ComplianceClaim` struct is **frozen**; ABI governance is purely a mechanical CI gate (`xtask abi-diff`) with no provenance record.

## Decision (preflight consensus — to ratify)

1. **Three governance streams journaled to the Transparency Log** via new `FrameKind` discriminant(s) in `maos-iac` (next free = 28), emitted kernel-neutrally through the `maos-bin` observer pattern (`TlYankObserver`-style), respecting the I2 panic-on-write invariant.
2. **F5 — ComplianceClaim schema lifecycle is a DECOUPLED event stream.** Schema version + effective-date + supersedes + ratified-by are journaled **on the event**, referencing claim identity. The frozen `Claim` struct and its envelope are **never mutated** (a `#[serde(default)]` field on either is a freeze violation). This preserves Story 9.2b's 21336 bytes-identity construction.
3. **F6 — `AbiExtensionProposal` governance object + reconciliation gate.** A proposal carries (proposal-id, summary, ratification-status {Proposed, Ratified, Rejected}, ADR-ref). `xtask abi-diff` stays the mechanical truth (did the ABI change); the journal is the human-decision provenance. The **new CI gate reconciles them**: every abi-diff-detected ABI change must have a corresponding *ratified* proposal in the journal, or CI fails. No voting UI in scope.
4. **F7 — `--kind governance` category filter** (read-only): an additive `kind_category_to_kinds` resolver + `kind IN (…)` builder in `maos-audit`, with completeness cross-check against the kind registry, non-contamination, and single-kind backward-compat.
5. **Dogfooding:** Story 9.3b's own FR64 kernel re-pin is recorded as the **first ratified `AbiExtensionProposal`** through this model.

## Consequences

- FR62's FrameKind addition re-pins the kernel-core baseline (it is **not** kernel-neutral) — re-pinned jointly with FR64 under one authorization (FLAG-Winston).
- The frozen ComplianceClaim guarantee holds; schema evolution (binding-v0.2+) flows through ADR-037 and is recorded as a lifecycle event.

## Gate

`xtask` abi-diff↔ratified-proposal reconciliation gate; `--kind governance` completeness + non-contamination + backward-compat tests; frozen-`Claim` byte-unchanged regression test.

## Sign-off required

Winston (ADR-037 owner) + Mary (ComplianceClaim schema co-author) on the F5 decoupling.
