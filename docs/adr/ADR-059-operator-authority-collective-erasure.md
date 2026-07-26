---
Status: ratified-v2.2
Gate: Story 13.5b — `check-reza-production-path`, blocking at v2.2 when substrate is present
Decided: 2026-07-25
Accepted-in-PR: Story 13.5b
Amends: ADR-055 §3 (the guarded store entry points are now write/read/scan/erase)
Reuses: ADR-026 (principal namespace and erasure), ADR-038 (measured KLOC ceilings)
---

# ADR-059 — Collective-tier erasure is operator-authority and out of kernel

## Context

The original 13.5b brief assumed that a principal forget should fan out through the collective tier. That premise conflicts with the existing Decision D control: `MemoryNamespace::Principal` is partitioned out of collective storage precisely to avoid creating an Article 15/17 substrate with no principal erasure path. The production tenant model also gives each `LoomLiteStore` one pool and one home team; ADR-055 forbids one team process from naming another team's database.

The existing uninstall orchestrator is an operator process at the composition root. No Spirit-facing collective erase capability, stable cross-team principal subject, all-team enumerator, or authorization-provenance column exists. Adding a kernel arm would therefore create reach without creating the missing subject or authority evidence.

## Decision 1 — Decision D is upheld

Collective storage continues to refuse `MemoryNamespace::Principal` at its own write entry point and at signed replication apply. The schema decoder no longer reconstructs a persisted `principal` namespace. The erasure backend registry names Loom because the partition itself is the Article 17 proof: a typed refusal plus a real-store scan must show that no principal-shaped collective row landed.

This does not close the analogous Shared-tier hole. Shared principal partition remains a separate blocker candidate owned by Story `13-5h`.

## Decision 2 — Erase authority is operator-only

`CollectiveMemoryPort::erase(spirit_pid, namespace, key)` is a store-addressed operator contract. The composition root invokes the synchronous adapter from `spawn_blocking`; the store applies its tenant guard before querying.

The erase operation is intentionally absent from:

- `SpiritMemoryView`;
- the kernel's `MemoryTier::Collective` arms;
- the capability `Scope` vocabulary.

The route has one production call site and writes one `collective.operator.erase` audit intent. Store-side or audit-side evidence alone is a reconciliation failure.

A new ADR and explicit kernel-baseline grant are required before any of these conditions change: Spirit-initiated erase, principal-addressed collective rows, kernel-mediated cross-team fan-out, or a capability scope for erase.

## Decision 3 — One store, one key tuple, no hidden fan-out

The contract erases at most one physical row selected by `(spirit_pid, namespace, key)` in the configured team's database. Cross-team physical namespace encoding is resolved inside that same store. The operation never calls the deliberately unguarded `pool()` escape hatch and never names another team's database.

Cross-team erase/hold fan-out remains absent. Story 13.6 owns the inverter and must choose an organization-level orchestrator or a per-team agent protocol; it may not weaken ADR-055's one-pool boundary.

## Decision 4 — Delete and tombstone are one serialized operation

A hard delete alone is invalid under the shipped CRDT-LWW write rule: replaying an old leaf would take the insert branch and resurrect the row. Erase and write therefore share a transaction-scoped advisory lock derived from the logical key tuple.

Erase deletes the selected physical row and upserts `collective_erasure_tombstones` in one transaction. Replication and direct writes compare `(source_ts, source_region)` against that tombstone. A stale or equal clock returns typed `ErasureTombstoneDominates`; it never silently lands. A genuinely newer local write may supersede the old tombstone under the same total-order rule.

Deleting a row changes `kv_merkle_root`, `compute_kv_payload_oracle`, and row count. The live witness re-derives all three and proves stale replay cannot restore the pre-erase state. Existing per-team Merkle independence remains the cross-database control.

## Decision 5 — Legal holds bind to one Host-global authority

Principal legal holds remain Host-global. Production construction explicitly binds either the global `main` schema or an attached Host-global schema. An unbound team shard returns `LegalHoldAuthorityUnbound`; it must never interpret a local empty table as "no hold".

Operators may list and release holds through `maosctl legal-hold`. Release changes eligibility only and returns `auto_erased: false`; it never starts erasure. Team-scoped hold semantics remain absent and are assigned to Story 13.6.

## Decision 6 — Audit failure remains fail-fast

The current kernel-event append is the durable audit commit point and panics on failure. Story 13.5b does not catch unwind after partial mutation because that would imply transactional recovery the system does not provide. No held, not-found, or failed uninstall writes an uninstall lifecycle success or a complete proof. Erased, held, not-found, and failed terminals are machine-readable and use exit codes 0, 3, 4, and 5.

The crash window between a store-side effect and its audit append remains explicit and ownerless; the one-sided reconciliation plant keeps it visible.

## Decision 7 — Kernel boundary and measured delta

No file under `crates/maos-kernel-core/src` changes line count. The source-line pin remains exactly 23228 before and after `cargo fmt`. The backend registry uses the name `"loom"`, keeping its formatted declaration on one 99-character line.

This is **ZERO kernel-core delta**, not zero project delta. `maos-domain`, `maos-loom-lite`, `maos-iac`, `maos-bin`, `maos-cli`, `maos-audit`, and `xtask` all carry measured changes. Their ceilings are re-based to the measured residuals under ADR-038.

## Decision 8 — Attestation scope is what was covered, never what exists

*Added by the 13.5b code review (party-mode consensus, 2026-07-25, 3/4 with Winston dissenting).*

`REQUIRED_STORES` no longer contains `"shared"`. `SharedMemoryStore` has no delete method at any visibility, so the cascade could never cover it and `ForgetCascadeAttestation::completed` was structurally unable to be `true` — the mirror image of the D-1 defect this story exists to close, and an Article 17 outage on every region-pinned Host. The Shared tier is now attested per run as `CategoryStatus::CoverageGap` naming Story `13-5h`, which is what AC1 asked for.

The removal changes a **durable verification contract**, not just a producer: `verify_regional_teardown_receipt` independently re-checks `completed`. The contract is therefore stated explicitly and in one place. A regional teardown receipt attests exactly the stores named in `forget_cascade.stores_covered`. It is **not** a standalone all-tier Article 17 artifact. A consumer treating it as one MUST also read the companion erasure-proof bundle for the same run and honour its `CoverageGap` entries. `REQUIRED_STORES` and the new `UNCOVERED_STORES` partition `KNOWN_STORES`, asserted by `store_sets_partition_known_stores`, so the two cannot drift when `13-5h` moves `"shared"` between them.

## Decision 9 — A held run attests what it destroyed

*Added by the 13.5b code review (party-mode consensus, 2026-07-25, 3/4 with Winston dissenting).*

A legal hold is a global boolean over the **principal**, not over the Spirit. Refusing an entire uninstall because one co-resident principal is held would convert that principal's litigation hold into an indefinite denial of an unrelated subject's Article 17 request. The cascade therefore continues to erase every unheld principal.

What changes is the artifact. A run that destroyed principal data always attests it. The `held` terminal carries `held_principal_ids`, `erased_principal_ids`, `deleted_entries`, `revoked_tokens`, and the path of a **partial** proof in which the held principals appear as `CategoryStatus::CoverageGap { reason }`. Exit code stays 3. No regional teardown receipt is written for a held run — a held run is not a teardown. A held run that destroyed nothing writes no proof at all, so "no proof" keeps meaning "nothing happened".

Consistent with Decision 6: a held uninstall still writes no lifecycle success and no *complete* proof.

## Decision 10 — The private tier's filesystem residue is an open hole

*Found by the 13.5b code review's own proven-red work; recorded, not fixed.*

`PrivateMemoryStore::forget_principal` derives its removal set exclusively from the in-memory map. The private tier deliberately does not cache `MemoryValue::Markdown` — it is filesystem-canonical so operator hand-edits remain visible — and always spills it to disk. A principal's Markdown record is therefore invisible to the removal set, its `fs::remove_dir_all` never runs, and the file survives the cascade while the signed proof records `memory_namespace` as `Removed { count: 0 }` and subject access reports the principal gone. The same hole swallows any value above the 4 KiB spill threshold once the writing process exits, because a fresh store never hydrates from disk — which is every real operator uninstall.

Story 13.5b's D-4 fix corrected the *count*; the *enumeration source* is upstream of it and was already wrong. Correcting it means walking `fs_root` inside `forget_principal` — kernel-core lines, outside this story's ratified ZERO-Δ fence. Escalate rather than absorb. The defect is pinned by `private_tier_markdown_survives_the_forget_cascade`, bound as a Blocking gate leg, so a successor's fix goes red and forces the proof category to be corrected with it. See Residual 8.

## Residual register

| # | Residual | Status | Owner |
|---|---|---|---|
| 1 | Cross-team erase/hold fan-out; 13.6 both creates the first production crossing subject and judges this contract | ABSENT | Story 13.6 |
| 2 | Shared-tier principal partition or namespace-aware erase; latent Article 15/17 hole and v2.2 blocker candidate | OPEN — must land before 13.6 | Story `13-5h` |
| 3 | Team-scoped legal holds and composite hold identity | ABSENT; do not emulate with global rows | Story 13.6 |
| 4 | Erasure correlation IDs and a multi-shard reconciliation reader | OPEN | Ownerless and open |
| 5 | CRDT-LWW resurrection after delete | CLOSED by transaction-serialized tombstones whose clock is `max(row_source_ts, now)`; the earlier bare-`now` stamp left a future-skewed-leaf window (13.5b review) | Story 13.5b |
| 6 | NFR-Ops-11(iii), per-operator capability-token signing key | OPEN | Ownerless and open |
| 7 | Crash atomicity between store mutation and audit append | OPEN; fail-fast and detected by one-sided reconciliation | Ownerless and open |
| 8 | Private-tier filesystem residue: `forget_principal` enumerates only the in-memory map, so `Markdown` records and post-restart spills survive while the proof says `Removed { count: 0 }` (Decision 10) | OPEN — kernel-core fix, needs FLAG-Winston; pinned RED-on-fix by `private_tier_markdown_survives_the_forget_cascade` | Ownerless and open |

## Rejected alternatives

- **Principal-scoped collective forget:** rejected; it reverses Decision D and creates the forbidden substrate.
- **One process fans out to every team database:** rejected; it violates ADR-055's physical authority boundary and no all-team port exists.
- **Kernel erase arm or new capability Scope:** rejected; operator authority already exists at the composition root, while Spirit authority does not.
- **Hard delete without tombstone:** rejected; stale replication deterministically resurrects the row.
- **Shard-local legal-hold fallback:** rejected; an empty local table is a fail-open answer.

## Consequences

- The Reza production gate now carries hermetic partition, terminal, hold, authority, and one-sided-reconciliation legs plus live Postgres partition/erase witnesses.
- Missing live substrate remains `AdvisorySubstrate` locally and blocks when provisioned in CI.
- Shared-tier principal state, cross-team fan-out, team-scoped holds, and cross-shard correlation remain named limits; no proof or operator document may claim them closed.
