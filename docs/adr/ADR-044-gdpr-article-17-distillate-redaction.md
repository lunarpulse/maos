# ADR-044: GDPR Article 17 Distillate Redaction — Marker + Body-Scrub-on-Embed

## Status

Accepted — Story 9.2 (2026-06-12).  Implements Decision C from the Story 9.2
preflight.

## Context

Story 9.2 wires `maosctl forget --principal <id> [--reason <legal-hold>]`
(FR45) and proof-of-erasure on Spirit uninstall (FR65).  The existing kernel
cascade already removes principal-namespace entries from the private memory tier,
removes `principal_index` rows, and journals a `principal.forget` Transparency
Log frame.  What was unresolved was how to handle **distillates** whose
effective audit chain references a forgotten source frame.

The `effective_source_log_ref` stored in a Distillate frame is a *reference*
set, but real distillate *bodies* may embed source content inline.  A redaction
marker alone does not remove embedded plaintext.

## Decision

1. **Append-only redaction marker.**  On forget, the kernel appends a new
   Transparency Log frame (`FrameKind::TaskComplete`, intent
   `"distillate.redacted"`) that references the affected Distillate `frame_id`.
   The original Distillate frame is **never mutated in place**.

2. **Body-scrub-on-embed gate.**  The cascade scrubs the Distillate body
   (`payload_redacted`) by overwriting it with a deterministic tombstone JSON
   object `{ "redacted": true, "reason": "gdpr-forget",
   "original_kind": "Distillate" }` **whenever** the scenario calls for it.
   The corpus carries a unique canary token planted in a forgotten source frame
   and embedded into the distillate body; after forget, a raw-byte scan of all
   Distillate bodies MUST NOT find the canary.

3. **Defer live re-distillation.**  Regenerating a redacted distillate from
   surviving source frames is not required for the v1.0 floor.  The floor is
   leakage, not regeneration, and is enforced by the canary gate.

## Consequences

- The audit chain remains append-only; redaction is observable as a new frame.
- A body-scrub is a write to the Distillate row's `payload_redacted` column.
  This is the authorized kernel-core delta for Story 9.2.
- The canary gate closes the false-clean risk where embedded content survives.
- Re-distillation is tracked as a named follow-up; it does not block v1.0.

## Story 18-6 implementation

Story `18-6-fr45-forget-redistillation-and-cascade-closure` supersedes Decision 3
and the re-distillation follow-up above. The kernel scans every writer and frame
kind for token-bounded principal ids, preserving forget/redaction lifecycle
records and `principal.uninstall.%` evidence. Direct distillate parents are
recorded alongside effective raw references, and closure follows parents
transitively, including summaries whose own text names the principal while
their raw sources do not. Frames also naming a legally held principal, and
their derived distillates, are preserved and reported as typed skips.

The daemon root dispatches a typed forget notice to live compiled-in Spirits,
independently of ordinary hook quarantine accounting. The owning Researcher
clears cached claims and its last output, rechecks cached ids against the TL,
and regenerates only from current `payload_redacted` fetched through governed
`MediatedLogRecall`, never cached MCP responses. The root validates writer,
timestamp, non-tombstone body and surviving sources, scans regenerated text
again, and completes a second principal sweep after live notices acknowledge.
Undelivered purges, malformed lineage gaps and regeneration errors are typed.
An error after erasure does not turn a completed erasure into a failed one or
prevent uninstall token revocation and signed proof.

Each dispatched request ends in `distillate.redistilled` or typed
`distillate.redistill.pending`; a same-boot retry recovers journaled requests
without an outcome, including their redacted ids. Late hook results append
their eventual outcome. Offline and cross-boot writers remain pending, not
automatically resumed. Legal holds still refuse without mutation.
Non-distillate tombstones have `frame.redacted` markers; cost attribution
retains its numerical fields and is not a whole-body retry tombstone.

Detection remains content-blind and id-based. Unicode letters/digits belong to
ids; punctuation `.@_+-` continues an id only when it bridges id characters.
Sentence-final punctuation is a boundary; a longer email suffix is not.
An id glued directly to CJK text without spaces may therefore not match.
Content copied without the id, escaped/transformed ids, external re-ingestion,
and non-principal Shared/Loom coordination data remain explicit limits.
Cross-store atomicity remains `v25-erasure-crash-reconciliation`: scrubs precede
deletions, and a retry recovers outstanding requests as well as remaining
scrubs, but mutation and audit still do not form a cross-store transaction.
Large-log locking/paging is owned by `20-11-erasure-inventory-closure`.

## Compliance

- `xtask check-kernel-baseline` re-pinned from 21197 to 21276 lines.
- `xtask check-empty-kernel` passes: the new write primitive is inside the
  I9-sanctioned `TransparencyLogAdapter` holder and carries no new persistent
  state-bearing structs outside it.
- `maos-audit` remains read-only: all writes stay in `maos-kernel-core` /
  `maos-iac` / `maos-bin`.
