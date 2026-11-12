<!-- Commit reviewed 6bb4f339. Model family: GPT (agent Review209K). Persisted verbatim by Main's delegate (Rec209). -->

## Verdict

**Correct, with 2 P3 findings; no confirmed P0–P2 defects.** Confidence: **0.82**. No builds, tests, or gates were executed during this review.

### P3 — Journal the actual phase for the new NotLoaded refusal
**Location:** `crates/maos-kernel-core/src/hot_swap/coordinator.rs:362–368`  
**Confidence:** 0.99

The new branch sends `DiscardSuccessor` while the saga remains `SnapshotTaken`. Its consumer in `saga.rs:127–130` hard-codes `SwapInFired`, so this refusal journals a phase that never occurred.

**Fix:** Derive the phase from `current_phase` for `DiscardSuccessor`, as already done for `RestorePredecessor`.  
**Witness needed:** Extend the unloaded-pid vector to assert the abort row’s phase is `SnapshotTaken`.

### P3 — Remove temporary archive files after failed writes
**Location:** `crates/maos-kernel-core/src/hot_swap/archive.rs:47–51`  
**Confidence:** 0.98

Failures from `write_all`, `sync_all`, or `rename` leave the newly created temporary file behind. A failed snapshot write can leave `snapshot.tmp` permanently: the manifest has already been published, and admission’s write-if-absent guard skips later repairs.

**Fix:** Use an RAII-owned temporary file, cleaning it up unless publication succeeds.  
**Witness needed:** Inject write/sync/rename failures and assert temporary files are removed while published files remain intact.

## Review conclusions

- Suppression prevents dispatch and fire journaling, revokes partial mints, and consumes cadence under the existing fence. Suppression rows are cadence/bucket bounded.
- FR4 consumes the new record explicitly; missing/extra keys and incorrect field types remain fail-closed.
- The session’s blocking sleep runs inside `spawn_blocking`, **not** on an async executor worker.
- Idle tasks are drained before the next poll; ordinary hook panics are converted by the dispatcher. Task count is bounded by that poll’s candidates.
- Hash-shaped scope text can be redacted; lossless refused-scope fidelity is not witnessed.
- The recorded measurements support **27,103/21,256** and the disclosed crate deltas. The extra revoke diagnostic does **not** establish live-token recovery: current revoke errors mean unknown or already-revoked tokens.
- Archived red/green/plant logs support the named witnesses. They do not independently establish final-commit provenance, crash durability, macOS runtime behavior, or hosted completion.
- AC9.1’s skip has an explicit disposition. Hosted AC6, syscall-order observation, clean-tree ABI-diff, and prepush remain unverified. The gate summary records `check-dev-record-completeness rc=1`, despite its JSON reporting `passed:true`.

**Counts: 0 P0 · 0 P1 · 0 P2 · 2 P3 — incorrect abort phase; temporary-file cleanup.**