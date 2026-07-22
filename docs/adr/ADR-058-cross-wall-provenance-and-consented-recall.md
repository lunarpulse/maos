---
Status: ratified-v2.2
Gate: Story 13.3b — `check-multi-tenant-loom`, blocking at v2.2
Decided: 2026-07-21
Accepted-in-PR: Story 13.3b
Amends: ADR-013 (`log.recall` cross-host revisit), ADR-049 §7 (origin-provenance scope), ADR-055 (multi-tenant Loom)
Reuses: ADR-018 (I13 intent lineage)
---

# ADR-058 — Cross-wall origin provenance and consented recall

## Context

Two predecessor decisions constrain this change. ADR-049 §7 says a signed replication bundle establishes **provenance-of-origin**, never **provenance-of-authorization**. The archived and now ported ADR-013 makes `log.recall` participant-scoped and explicitly rejects a general cross-Spirit or admin override, while its revisit clause requires a new ADR if cross-host recall becomes common. That trigger has fired.

ADR numbers 056 and 057 are reserved in planning prose and remain intentionally unwritten. This record therefore uses 058 rather than renumbering either reservation.

## Decision 1 — Leaf v3 binds copied origin metadata

`CollectiveKvLeaf` has three valid data-presence shapes:

1. no team, no depth, no lineage → frozen v1 domain and bytes;
2. team only → frozen v2 domain and bytes;
3. team plus `distillation_depth` plus `intent_lineage` → v3 domain `maos.collective-kv-leaf.v3`.

V3 appends the copied depth and canonical I13 lineage after the length-prefixed source team. Partial or team-less v3 shapes are invalid and fail bundle verification before Merkle verification, and `distillation_depth` must be >= 1 — the only shape the row decoder can read back. Bundle verification additionally requires **envelope–leaf coherence**: every leaf in a v1 envelope is team- and provenance-free, and every leaf in a v2/v3 envelope carries the envelope's own `source_team` **and `source_region`** — a mismatch on either axis fails closed before Merkle verification, so an envelope can never unbind a leaf's provenance from its attesting team, and a hand-crafted v2/v3 bundle cannot launder a foreign origin region. (Only v1 region transport is unconstrained: it legitimately carries foreign-`source_region` leaves, and its bytes are frozen.) The destination persists both fields and reconstructs them through the same row/apply seam; bundle serialization and rebundling preserve them.

The existing `CollectiveRow.source_log_ref: String` remains excluded. It is a destination-local re-attestation stamp, not the I11 `effective_source_log_ref: Vec<[u8; 16]>` chain, and apply deliberately derives a new one-hop value. This decision does not claim the bundle proves that a capability check passed or that an authorization audit record landed. It proves only the signed origin of the copied depth and lineage.

## Decision 2 — Cross-wall recall is a separate directional capability

The existing `LogRecallPort::recall` and `fetch` signatures and emitter-scope behavior remain unchanged. A third additive method, `recall_cross_wall(spirit_pid, team, filter)`, is separately mediated. `LogRecallAdapter::new` remains one-argument; an unconditional `with_cross_wall_consent` builder injects a `CrossWallRecallConsentPort` from the composition root.

The production adapter reads one fresh `CohortManifestState` snapshot. For a home team disclosing to a remote team, only the exact ordered grant `(home_team, remote_team, "log:recall")` admits the request. A reverse-only grant is not symmetric and is reported separately. Missing injection, no grant, reverse-only grant, stale lease, and unavailable state all fail closed as typed `LogRecallError::ECrossWallRecallDenied` reasons. A legitimately empty page remains `Ok(LogRecallPage { entries: [] })`; it is never folded into refusal. Existing cross-Spirit fetch remains the distinct `ScopeViolation` outcome.

This is the narrow federated capability ADR-013's revisit clause anticipated. It does not create an admin override, expose path-addressed `ranged_recall`, establish per-team Transparency Log isolation, or claim that refusal journaling is complete.

## Decision 3 — Citer authorization stays fail-closed; DAG traversal is corrected

Story 8.10's citer-authorization rule remains unchanged: flattened source frames must still belong to the citing `spirit_pid`. The requested team-axis exception is cut because `TransparencyLogEntry` and `FrameFilter` carry no team or stable-principal field, and adding the storage dependencies needed to infer one would violate the crate boundary. Conjunctive same-principal/team authorization therefore remains an open §15 architecture gap; this story does not weaken the existing control to simulate closure.

The independent provenance-DAG defect is fixed. Traversal tracks nodes resolved globally separately from nodes on the current DFS path. A shared raw dependency in a diamond is deduplicated and succeeds; re-entering a node still on the current path remains a true cycle and fails.

Distillation depth is explicitly **unbounded** at v2.2. The Spirit-side "halt-and-escalate at depth 3+" convention is prose with no enforcing constant; traversal terminates only via the resolved set. A cross-wall chain is therefore attacker-influenceable in length in a way a local chain is not, and bounding it is an open §15 item — this record states the absence of the bound rather than inheriting the doc's claim of one.

## Consequences and limits

- V1 and v2 leaf hashes are frozen by byte-level goldens; v3 is additive.
- Origin depth and intent lineage survive a team-wall apply and rebundle. This is origin evidence only, never authorization evidence.
- Cross-wall recall consent is caller-legible and direction-sensitive. Empty success, no grant, reverse-only, stale state, and scope violation are distinguishable without string matching.
- The manifest-backed consent adapter is injected only when verified cohort state and `MAOS_LOOM_HOME_TEAM` are both available; otherwise the builder remains absent and the method fails closed.
- Per-team TL isolation and refusal journaling remain outside this decision. They must not be claimed by a gate or operational document until implemented.
- No `maos-iac → maos-cohort` or `maos-iac → maos-loom-lite` dependency edge is introduced. `TeamId` and the consent trait remain in `maos-domain`.
