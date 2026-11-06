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

Uninstall joins kind-11 writers to the stable Spirit admission identity at the
frame's row, within each `(boot_nonce, pid)` incarnation. Neither cross-boot nor
same-boot pid reuse attributes another Spirit's lineage or principal index rows
to the target. Principal discovery uses stable `id-<identity-hex>` ownership.
Ambiguous `legacy_numeric_owner` and unattributed lineage are named retention
gaps, not guessed erasures. Principal-free owned distillates use the same
transactional body scrub and `distillate.redacted` marker. Durable holds protect
whole derivations. IAC-frame lineage audit arrays remain retained; cost/governance
numerical facts keep structured redaction.

The unreleased proof format is `maos.erasure-proof.v2`, with a mandatory,
exact sixteen-category inventory. Every observation gets a distinct
`principal.uninstall.category` TL row and post-tree inclusion. `Removed{count:n}`
must sum to exactly `n` matching typed evidence: unique frame/package/schedule
identities, or the real memory/index ForgetReceipt and capability-revocation
batch counts. Missing, duplicate, wrong-kind or mismatched evidence is rejected
by both verifiers. Schema types, unknown/duplicate JSON fields, UTF-8 canonical
signing, hex spellings, Merkle inclusion versus exclusion, subject count and
the 128 MiB input/output bound have the same acceptance language.

Subject leaves are signer-secret HKDF-SHA256 commitments bound to this bundle,
not public dictionary hashes of principal names. Published bundles are owner-only
and use unpredictable create-new staging, file/directory fsync and no-replace
publication. This does not hide Spirit identity, timing or frame-id correlation.
Bundle verification trusts the signer about observations; frame-id membership
does not authenticate TL payload contents or prove exhaustiveness without the log.

The cascade names memory namespace, principal index, tokens, shared/private
tiers, principal frames, intent lineage, pending halts, schedules and installed
package, plus a typed retained-store inventory: FR11 manifests, authored skill
queue entries, Lifecycle Journal and Worker output workspaces. Only an otherwise
Erased uninstall removes the installed package. A refused installed package is
removed offline after stopping the serving root; the live door cannot unload an
SCB that admission refused.

Installed removal retains a complete streaming-tar preimage and boot inventory
through fallible deletion, proof publication and lifecycle finalization. Ordinary
partial-delete failures restore every generation byte before re-advertising it;
rollback/index failures are surfaced to the caller, not only logged by Drop.
A signed provisional gap receipt precedes irreversible scrubs. A later failure
retains a truthful partial proof path with installed-package retention/recovery
status, never deletes the sole available receipt, and does not report Erased.
The final proof is published after lifecycle completion and immediately precedes
accepting the recoverable deletion. Cross-store crash atomicity remains outside L1.
The provisional artifact normally remains beside the terminal inventory. Readers
select the returned terminal `proof_path`, not directory/filename order, and may
authenticate both. If final publication fails, the earlier gap receipt remains
the honest available artifact; no provisional receipt certifies completion.


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

Scans fix an entry high-water once and stream at most 4096 actual rows per lock
window (including sparse rowid histories). A bounded tail pass covers later
matching commits, with a hard pass cap rather than chasing a sustained writer.
**Completion is the durable transactional linearization marker**: one TL writer
transaction rechecks the high-water and records completion. At that marker, no
matching erasure-eligible frame body committed before it survives; commits after
the marker are post-completion data and are **not** claimed erased. Retained
forget/hold/redaction and `principal.uninstall.%` audit metadata, IAC lineage
arrays and the numerical facts of structured cost/governance redactions are
explicit exceptions, not erased rows. This is not a promise about
everything committed before the handler returns. A held or unverifiable/ambiguous
scope stays pending/retained, never complete. A malformed kind-11 body with verified
`(boot_nonce, pid, Spirit identity)` attribution is its own lineage: it may be
scrubbed with reason `spirit-uninstall` and its real frame id evidenced. Its earlier
malformed observation is cleared only when the final marker certifies the sweep;
an ambiguous malformed body stays byte-for-byte retained and named as a gap.
Partial cost/governance redactions remain citable numerical-fact sources. They
are not prior full-body tombstones: an unrelated forget must not erase a neutral
derivative merely because its source has a partial redaction flag.

Body mutation, its real redistillation request and redaction marker share an
IMMEDIATE transaction with the attached global hold authority. A hold identity
nonce prevents an ABA replacement from being removed by stale release. The
frozen kernel's later private/index deletion still lacks this shared cross-store
hold lease: `kernel_private_index_hold_lease` is an explicit L1 gap requiring
separate measurement and a grant at the final kernel preflight; the existing
KS/O4 allowance is not extended. No private/index lease or AC3 repair is claimed here.
Durable `principal.uninstall.scope` audit metadata preserves stable subject
bindings across a partial uninstall, even after the live index was deleted.
Private retained counts and held-namespace attribution share one metadata-only
walk. An unreadable held payload cannot cover an unrelated unindexed namespace;
that residue is still Failed and named. Typed failure classes preserve Held for
the established held-plus-regional-key case without disguising genuine errors.
An earlier completion error is superseded only by the final durable frame marker;
real regeneration-only Pending outcomes remain in the public report.

**Named L1 limitation: final-tail redistillation is explicit-replay only.**
The uninstall tail durably records each real request as
`Pending/Refused("late-frame")`; its count and unchanged request/source ids appear
in the durable uninstall receipt's `redistillation` report. The signed frame
proof certifies body removal at its marker, not completed regeneration. The tail
does not dispatch these requests, including requests addressed to another live
Spirit: no cross-Spirit regeneration side effect is introduced by uninstall.
`outstanding_redistill_requests` excludes requests with any recorded outcome,
including Pending. Ordinary forget/uninstall retries therefore do not redrive
these outcomes automatically. An authorized embedding can explicitly replay a
known subject's unchanged durable request outside uninstall: obtain a fresh real
`ForgetOutcome::Erased` from `memory.forget_with_reason`, append the request to
that outcome's `redistill` list, then call
`HookPump::finish_forget(tl, &outcome, Some(live_pump))`. Keep the real receipt and
counts; never invent ids or promote a Suspended outcome. An unknown subject or
unavailable/unverifiable writer incarnation remains pending. L1 adds no retry
API, CLI surface, automatic redrive or cross-boot rebinding. The ownerless/open
rule-10 disposition is trigger-owned by the first story adding an operator
redistill-retry surface (`deferred-work.md`).


Kind-11 and redistillation-request readers pin predicate-only partial indexes;
schema initialization removes the former full kind/intent indexes. Outcome
lookups keep their partial expression index. SQLite VM accounting includes the
page-boundary query and retains an independent absolute work cap; both real
100k/1M 1 KiB disk/WAL door witnesses still enforce the unchanged <25% wall bound.
Recovery remains paged and timestamp/frame-id ordered. Full cascade inventory,
filesystem, proof and package work runs inside the blocking boundary, with async
hook dispatch driven on the runtime; window handoff uses the fair IAC mutex,
not a Tokio task yield. This inner mutex is non-poisoning: a panic no longer
disables future lock acquisition. SQLite writes still panic under I2 before
delivery; RAII transactions roll back on unwind, and last-frame ids are published
only after successful writes/commits, rather than relying on poisoned state.
The constituent/VM-work proof is green locally, but AC5/EC-13 remains open:
the unchanged wall bound and real 1M completion await hosted stable-runner proof.

The unchanged route deadline may return HTTP503 `handler_still_running`:
**pending**, not erased or complete. The in-flight handler is not cancelled.
After the completed operator report is durable, an identical decoded principal
and optional reason replays that report through the existing correlation index.
The lookup validates the original transactional marker and reads at most 4096
post-marker rows; a matching new application body, any new distillate, an active
hold or reaching that finite limit starts a fresh forget instead. Retained
control receipts are not application bodies. There is no rescan of old history
on a valid replay, and no second memory cascade or frame marker.

The returned report preserves the original erasure `operation_id`, component
counts and marker, and says `replayed: true`. It is a **historical marker-scoped
report**, not certification of the stores at replay-return time. Each transport
submission still has its ordinary fresh CSPRNG handle; its completion audit row
links `replayed_operation_id` to the original, so a contended replay remains
findable without moving lookup work outside the unchanged route deadline.
The original completion row stores the serialized control-report snapshot, not
application frame payloads. Regeneration-only Pending is replayed as recorded,
not redriven or promoted to completed regeneration.

The bounded check covers the TL tail and current hold authority, not a fresh
inventory of private/index or other stores. New state outside that observable
tail needs a fresh request with a distinct reason using the existing command;
replay never claims that post-marker state erased. No new retry API, deadline,
writer barrier or frozen-kernel presence API is introduced.
Independent same-UID process writers have no writer barrier in L1: SQLite
orders the marker against their commits, but they may introduce new data after
it. That named limitation is not erased by a successful verification result.


## Compliance

- `xtask check-kernel-baseline` re-pinned from 21197 to 21276 lines.
- `xtask check-empty-kernel` passes: the new write primitive is inside the
  I9-sanctioned `TransparencyLogAdapter` holder and carries no new persistent
  state-bearing structs outside it.
- `maos-audit` remains read-only: all writes stay in `maos-kernel-core` /
  `maos-iac` / `maos-bin`.
