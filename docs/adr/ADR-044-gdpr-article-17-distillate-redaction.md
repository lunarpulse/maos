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

## Compliance

- `xtask check-kernel-baseline` re-pinned from 21197 to 21276 lines.
- `xtask check-empty-kernel` passes: the new write primitive is inside the
  I9-sanctioned `TransparencyLogAdapter` holder and carries no new persistent
  state-bearing structs outside it.
- `maos-audit` remains read-only: all writes stay in `maos-kernel-core` /
  `maos-iac` / `maos-bin`.
