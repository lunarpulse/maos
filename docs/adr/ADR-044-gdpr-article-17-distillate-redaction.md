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

## Story 20-11 landing 1 — Spirit inventory

Uninstall also queries kind-11 writers at every admitted `(boot_nonce, pid)`
incarnation, including principal-free distillates. It uses the same body-scrub
primitive with reason `spirit-uninstall` and appends `distillate.redacted` with
`spirit_id` and `redacted_distillate_frame_id`. A derivation protected by a durable
hold is retained and named by id. IAC-frame lineage audit arrays remain retained;
cost/governance numerical facts keep the existing structured-redaction contract.

The proof format is `maos.erasure-proof.v2`. Every category, including
`VerifiedEmpty` and `CoverageGap`, gets a distinct `principal.uninstall.category`
TL observation containing its Spirit and complete category status before the
post-tree is captured. The signed envelope carries the corresponding post-tree
inclusion proofs. Both verifiers require a one-to-one category/evidence match,
distinct names and evidence ids, the matching Spirit/status and valid inclusion.
The existing frame-id Merkle model still does not independently authenticate
payload contents or prove a supplied leaf set exhaustive without TL access.

The cascade names memory namespace, principal index, tokens, shared/private
tiers, principal frames, intent lineage, pending halts, schedules and installed
package, plus a typed retained-store inventory: FR11 manifests, authored skill
queue entries, Lifecycle Journal and Worker output workspaces. Only an otherwise
Erased uninstall removes the installed package. A refused installed package is
removed offline after stopping the serving root; the live door cannot unload an
SCB that admission refused.

Installed removal uses a serialized, same-filesystem staging guard. The active
index and generation are restored on proof-write failure; final deletion is
committed only after proof persistence. This is an in-process failure guarantee,
not cross-store crash atomicity. Rollback IO failures remain explicit errors in
the diagnostic log, not unconditional claims that restoration succeeded.

L1 remains kernel-free: pending halts explicitly name AC3's final KS + O4 landing,
and the live schedule category names the observed ids plus retained watchdog
buckets/stamps until root exit. No empty fresh registry is evidence of halt
erasure. Offline package removal removes its schedule admission source; a
compiled-in Spirit may be admitted again by the binary.

The serving admission paths currently default the SCB's schedule section, even
when the input manifest declares schedules. AC6.3 therefore uses a live
principal-free lineage witness for the hosted Worker, not an invented firing
schedule. The separate AC4 boundary witness supplies an actual scheduled SCB to
the real door; schedule parsing/admission itself is not repaired by L1.

Forget scans use stable 4096-rowid windows and release the TL lock between
windows, sorting results back into `(timestamp, frame_id)` order. Before each
body mutation, the complete durable hold set is re-read under the TL lock and
serialized with placement/release; a changed set rebuilds protected derivations
using bounded scans. This also protects a hold placed between initial forget and
the late sweep. The live door releases holds through the Memory Manager's
existing forget serializer. Blocking cascade/door scans leave Tokio workers.

The access path is part of that bound: a kind-only index has an implicit rowid
tail and supports `(kind, rowid range)` seeks. The older kind/timestamp index
cannot seek rowid without a timestamp constraint; merely adding a rowid WHERE
clause can therefore rescan all kind-11 rows per nominal window. Kind scans pin
the bounded index; general scans pin direct rowid access. Test-only SQLite VM
accounting observes the actual statements and keeps the <25% per-window work
bound independent of scheduler stalls and fsync latency. Raw wall timing is
retained rather than represented as compliant when the host is loaded.
Recovery of outstanding redistillation requests is paged too. An intent/rowid
index bounds the outer scan; a partial outcome-frame-id expression index turns
each anti-join into a point lookup, not a rescan per request. The hook-pump
recovery query also leaves the async worker. Recovery results retain
`(timestamp, frame_id)` ordering and the existing typed metadata conversion.

A shipped `handler_still_running` HTTP503 is **pending**, not an erased result.
The route deadline is unchanged; completion is recorded durably before the
handler sends its final report. The 1M-row witness requires completion and an
identical successful subsequent forget without changing the original tombstone
or unrelated bytes. This does not turn a pending response into a signed proof.


## Compliance

- `xtask check-kernel-baseline` re-pinned from 21197 to 21276 lines.
- `xtask check-empty-kernel` passes: the new write primitive is inside the
  I9-sanctioned `TransparencyLogAdapter` holder and carries no new persistent
  state-bearing structs outside it.
- `maos-audit` remains read-only: all writes stay in `maos-kernel-core` /
  `maos-iac` / `maos-bin`.
