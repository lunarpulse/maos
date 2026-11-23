---
Status: PROPOSED — Story 22-6 (`22-6-bounded-audit-store`), decisions D-22-6-01…10 taken at the recommended answers, operator not yet confirmed. The branch merges only after the `v1.0.0` GA tag.
Gate: `cargo test -p maos-bin --test audit_rotation_22_6 --test escape_anomaly_tail_17_10 --test maos_uninstall_16_4 --test host_config_22_1`; `cargo test -p maos-cli --test segment_verify_22_6`; `cargo test -p maos-iac --test invariant_i2_log_before_deliver`; `cargo test -p maos-kernel-core --test forget_forward_completion_21_12`; `cargo test -p maos-audit`
Decided: 2026-11-23
Accepted-in-PR: pending — Story 22-6
Revisits: ADR-044 (a forget now reaches sealed segments); ADR-067 (a new MAOS-written root, the segment directory); ADR-062 (the `audit rotate` door verb); ADR-071 (three `[host]` keys)
Supersedes: nothing
---

# ADR-072 — Audit-store rotation

## Context

The Transparency Log is one SQLite file per Host and nothing ever left it: there was no production
`DELETE FROM transparency_log` (the only one was a test) and no retention. FR79 says the audit store is
bounded by policy and that rotated segments stay independently verifiable. The epic names one consumer that
assumes the insertion id only grows (the escape-anomaly tail); four more do: the Worker progress stamper, the
forget proof (`through_rowid`, and the door's re-check of a completed forget), the governance ratification
sequence, and the forget sweep's rowid windows. A rotation that restarts ids would silently skip every new row
below an old cursor.

## Decision

1. **Prefix move in one live file** (D-22-6-02 A). The oldest prefix of the live log is copied, with its
   original rowids, into a SQLite segment file and then deleted from the live file in ONE transaction. The
   live file and its rowid sequence continue: the cut is always below the live high-water row, so
   `max(rowid)+1` keeps growing. The frame-keyed side tables (retractions, recipients, consent, ruptures)
   move with their frames. `approval_decision_log`, `schema_lifecycle_registry` and `legal_holds` stay live.
   **This is the first production `DELETE FROM transparency_log`.** Its scope is exactly a sealed,
   re-verified prefix: the copy runs outside the writer lock window by window; under the lock every copied row
   is compared with its live twin, and the delete uses the cut that was copied, never a re-read of
   `MAX(rowid)`. Never `VACUUM` the live file: that may renumber rowids.
2. **A segment is a SQLite file plus a signed manifest** (D-22-6-03 A): `<audit dir>/segments/<seq:010>.sqlite`
   and `<seq:010>.manifest.json`, schema `maos.audit-segment.v1`. The manifest holds the sequence, the
   exclusive lower id bound, the first and last insertion id, the row count and time window, the sha256 of
   the file, a content digest over every column of every row and every side row, the Merkle root over the
   frame ids (`backup.rs`), the predecessor manifest's sha256, `supersedes` and `scrubbed_frame_ids` (a
   re-seal), and `region`/`host` as in `AuditBundle`. `maosctl audit verify-bundle` dispatches on
   `schema_version`; the `maos.audit-bundle.v1` path is unchanged.
3. **Key custody** (D-22-6-04 A). The daemon signs at rotation with the operator audit seed
   (`load_audit_key_seed(&None)`), through the `sealed_export` primitives: canonical JSON, sha256, Ed25519,
   and the region-derived key when the Host has a region. The verifier never reads the `attester_pubkey` the
   manifest carries (R-RG1). With no key, rotation is refused with a typed line and the live log keeps
   growing: logging never fails because of sealing.
4. **A chain** (D-22-6-05 A). Each manifest names its predecessor's sha256 and the id ranges are contiguous
   (`after_rowid` = the predecessor's `last_rowid`). `maosctl audit verify-segments <dir>` reports a missing,
   reordered or forked segment. A re-seal keeps the chain: the successor's `prev` may name the superseded
   digest, which the re-sealed manifest lists in `supersedes`.
5. **Two stages of bounding** (D-22-6-06 A). Rotation is automatic by policy and deletes nothing that is not
   sealed and verified. Expiry is opt-in (`audit_keep_days`, unset by default, floor 90 days) and deletes only
   whole sealed segments, never one that holds a frame of a principal under legal hold or of an open erasure.
   An `audit.segment.expired` row is journaled in the live log before the unlink; the registry row stays, so
   the chain and the id floor survive.
6. **Forget reaches sealed segments** (D-22-6-07 A). The adapter's forget sweep scrubs the principal's frames,
   and the distillates that cite a scrubbed frame, in every sealed segment: a scrubbed copy, a new manifest
   with `supersedes: <old sha256>` and the scrubbed frame ids, an `audit.segment.resealed` journal row, then
   the file swap. `scrub_forget_frames` keeps its 5-tuple, so the kernel is unchanged. A live sweep counts as
   an active erasure: rotation defers while one exists, so a frame the sweep has yet to scrub never moves.
7. **Policy** (D-22-6-08 A). Three `[host]` keys under the ADR-071 one-key rule: `audit_rotate_bytes` →
   `MAOS_AUDIT_ROTATE_BYTES` (default 256 MiB, compared with the USED live bytes, pages minus the free list,
   because the file does not shrink), `audit_rotate_days` → `MAOS_AUDIT_ROTATE_DAYS` (default 30) and
   `audit_keep_days` → `MAOS_AUDIT_KEEP_DAYS` (default unset; below 90 is a `host_config_setting` refusal).
   A daemon task checks at boot and every 60 s. `maosctl audit rotate` is an `OperatorCommand` (door verb,
   operation id and completion row, ADR-062); with no Host answering it rotates offline.
8. **Peer replay** (D-22-6-09 A). The live file keeps `transparency_log_sealed_ids(frame_id, segment_seq)`.
   Only the peer-id path (`insert_frame_event_with_id(Some(id), …)` and the routed writers) checks it, so
   kernel-minted appends pay nothing and the I2 hot path is unchanged. A replayed sealed id is `Duplicate`.
   Rows are pruned when their segment expires.
9. **The cut floor** (D-22-6-10 A). `cut = min(policy cut; every registered floor; the oldest open
   principal.forget.intent − 1; the oldest outstanding redistill request − 1; the newest completed-forget
   marker − 1; MAX(rowid) − 1)`. The floors are an in-process registry in `maos-bin`
   (`audit_rotation::RotationFloor`): the escape tail and the progress stamper each publish the cursor they
   have COMMITTED. The reads of segments are merged: insertion order is segment order, time order is
   `(timestamp_ns, frame_id)` over the union, so a clock step across a boundary orders as in an unrotated log.

## Consequences

- `escape_detector_consumer.rs` states its precondition as "ids only grow; rotation removes only a sealed
  prefix below every registered floor".
- `maos_roots` enumerates `<audit dir>/segments` as a MAOS-written directory (ADR-067); `--keep-log` keeps it
  with the log "as a unit" and discloses the segment count and bytes.
- A segment signature proves that the holder of the operator audit key sealed these bytes. It does not prove
  the rows are complete or were written at their stated times. Removing the newest segments together with the
  live log is not detected: there is no external anchor (NFR-Aud-4 anchoring is optional and not built).
- Not rotated: `approval_decision_log`, `schema_lifecycle_registry`, `legal_holds`, the unbounded Worker
  workspaces. Not covered: backup and the Postgres migration (live log only), Spirit-uninstall lineage
  inside a sealed segment (the sweep scrubs principal subjects there), copies outside the segment directory.
- The kernel is unchanged; the delta lives in `maos-iac`, `maos-audit`, `maos-cli`, `maos-bin` and
  `maos-control`.
