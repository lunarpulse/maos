# ADR-028: Replay Determinism over Trace-Shape

## Status

Accepted — Story 9.2b (2026-06-13).  Implements preflight decisions F2, F3, F5
from the Story 9.2b party-mode consensus.

**Errata:** The Epic 9 spec and PRD reference "ADR-023" for FR46 / replay
determinism — that is incorrect; ADR-023 is the capability-token-TTL ADR.
**This** ADR-028 is the canonical home for replay determinism and trace-shape
contracts.  John owns the PRD/epic errata sweep for the bad ADR-023 reference.

## Context

FR46 (`journal.export`) produces a portable, Ed25519-signed trajectory bundle
from the immutable Transparency Log.  To serve its audit purpose the bundle must
be **replayable**: a third party with only the bundle + public key can verify
both signature integrity *and* structural determinism of the audit trail.

The audit trail carries redacted slots — payloads scrubbed by a
`redaction_policy`.  A replay that exposes any information about redacted content
violates GDPR Art. 5(1)(c) data-minimisation.

Three open design questions led to this ADR:

1. **What surface is deterministic?**  Full payload replay is impossible once
   redaction is applied.  The meaningful invariant is the *shape* of the trace:
   frame ordering, capability-token issuances, halt events, decision-frame
   emission — NOT redacted payload content.

2. **What is the determinism guarantee?**  "Best-effort byte-identical" is an
   untestable non-oracle.  The input is a sorted immutable TL; the transform is
   pure; byte-identity is achievable at v1.0.

3. **What does a redacted placeholder carry?**  Exact byte-length + a content-
   derived hash over a low-entropy field is a **confirmation oracle** — the
   bundle holder enumerates candidates, hashes each, and recovers the content.

## Decision

### D1 — Trace-shape as the deterministic surface

Replay operates over the **shape** of the trace, not the raw payloads:

- IAC frame ordering (sorted by `(timestamp_ns, frame_id)`)
- Capability-token issuances (`FrameKind::CapabilityInvocation` = 7,
  `SpiritRevoked` = 17)
- Halt events (`EpistemicHalt` = 3)
- Decision-frame emission (`Decision` = 10, `DecisionDispatch` = 2)

Redacted slots are projected as typed placeholders.  The trace-shape document is
validated against `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12).

### D2 — HARD byte-identity at v1.0

Two replays of the same bundle MUST produce **byte-identical** output.  This is
a **hard** CI gate, not "best-effort".

Anything that cannot be made deterministic is **excluded** from the signed
surface rather than shipped soft.

The **v1.5 scope** applies ONLY to the orthogonal cross-platform /
cross-toolchain-version / cross-schema-revision envelope.  The v1.0 gate is
single-platform / single-toolchain / single-schema-revision byte-identity.

### D3 — Placeholder grammar: class + bucketed length ONLY

A redacted placeholder carries:

```
<REDACTED:type=<class>, len=<bucket>>
```

Where:
- `type` is the redaction **class** (from frame metadata)
- `len` is a **bucketed** original payload byte length (power-of-two bucket,
  NOT the exact byte count)

**No content-derived hash.** No exact byte length.

Bundle integrity is already carried end-to-end by the Ed25519 signature
envelope.  Adding a content hash to the placeholder would create a confirmation
oracle for low-entropy fields (booleans, enums, known-format IDs) — the bundle
holder could enumerate, hash, and match.

### D4 — Redaction metadata via `query_with_redaction()`

An additive `Option<RedactionMeta>` field on `AuditEntry` carries per-frame
redaction state:

```rust
#[serde(rename = "redaction", skip_serializing_if = "Option::is_none", default)]
pub redaction: Option<RedactionMeta>,
```

This mirrors the existing `capability_token_hex` precedent.

**`query_with_redaction()` is the ONLY sanctioned populator** of this field.
The existing `query()` always returns `redaction: None`, preserving 9.1 sealed-
bundle byte-identity (the field is absent from JSON when `None` due to
`skip_serializing_if`).

### D5 — Determinism pin-list (binding)

The following sources of non-determinism are eliminated in the replay path:

| # | Source | Mitigation |
|---|--------|------------|
| a | `HashMap`/`HashSet` iteration | `BTreeMap`/`BTreeSet` or collect-then-sort in serialisation path |
| b | Multiple canonicalizers | Reuse 9.1's `canonicalize` — one canonicalizer, not two (anti-tautology) |
| c | Raw `f64` in shape | Slot-presence only, or fixed-precision string; no IEEE-754 float |
| d | Freshly-read clocks | ZERO fresh clock reads in replay output — timestamps from `timestamp_ns` column only |
| e | Implicit row ordering | Explicit total `ORDER BY timestamp_ns ASC, frame_id ASC` on every replay read |
| f | Placeholder field order | Fixed placeholder field order (type, then len) |
| g | `{:?}`/Debug repr | No Debug repr in output (unstable across compiler versions) |

### D6 — Binding test matrix

| Test | Oracle | Purpose |
|------|--------|---------|
| `sealed_export_bytes_unchanged_with_redaction_field_none` | Committed golden byte vector | 9.1 regression — `redaction:None` adds zero bytes |
| `serde_no_key_when_none` | `serde_json::to_value` has no "redaction" key | `skip_serializing_if` is load-bearing |
| `serde_key_present_when_some` | bytes-differ with `Some(_)` | Proves `skip` is load-bearing |
| `redaction_field_is_none_for_all_non_replay_callers` | Call-path oracle | Guards `query()` + 9.1 sealed-export callers |
| `replay_byte_identical_two_process` | Two separately-spawned OS processes | Hard determinism (HashMap seed stable within process, not across) |
| `verify_trajectory_rejects_open_writer` | Negative | DB must be quiesced (WAL-checkpointed) |
| `one_byte_tamper_replay_diverges` | Anti-tautology | Tamper must produce different output |
| `redaction_k_anonymity` | ≥K candidates collide per placeholder bucket | No confirmation oracle on low-entropy fields |
| `trajectory_schema_valid` | `trajectory.schema.json` validates export | Schema gate |
| `trace_shape_schema_valid` | `trace-shape.schema.json` validates replay | Schema gate |

## Consequences

- Replay is **pure read-only** over the TL; it projects frames to a canonical
  trace-shape document and re-derives the structural skeleton.  It does NOT
  re-execute Spirits.
- The `query_with_redaction()` invariant means all non-replay callers continue
  to see `redaction: None`, preserving backward compatibility.
- The placeholder grammar carries no information that could be used to invert
  the redaction of low-entropy fields.
- v1.0 ships a hard determinism gate; v1.5 extends it to cross-platform.

## Compliance

- `maos-audit` remains `#![forbid(unsafe_code)]` and read-only
  (`SQLITE_OPEN_READ_ONLY`).
- Zero kernel-core KLOC delta.
- Workspace stays at 44 crates.
- All new schemas follow the `draft-2020-12` canonical-bytes convention
  established by `schemas/audit-bundle.schema.json`.
