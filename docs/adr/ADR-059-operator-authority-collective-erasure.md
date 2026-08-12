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

This did not, at 13.5b, close the analogous Shared-tier hole. That hole is now closed by Decision 11, landed in Story `13-5h`: the same refusal is enforced at the Shared write, read and scan entry points by a single hoisted predicate.

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

This is **ZERO kernel-core delta**, not zero project delta. `maos-domain`, `maos-loom-lite`, `maos-iac`, `maos-bin`, `maos-cli`, `maos-audit`, and `xtask` all carry measured changes. Their ceilings are re-based to the measured residuals under ADR-038. **This ZERO-Δ statement is scoped to Story 13.5b.** Its successor 13.5h carries an authorized +6 — see Decision 11.

## Decision 8 — Attestation scope is what was covered, never what exists

*Added by the 13.5b code review (party-mode consensus, 2026-07-25, 3/4 with Winston dissenting).*

`REQUIRED_STORES` dropped `"shared"` at 13.5b. `SharedMemoryStore` has no delete method at any visibility, so the cascade could never cover it and `ForgetCascadeAttestation::completed` was structurally unable to be `true` — the mirror image of the D-1 defect this story exists to close, and an Article 17 outage on every region-pinned Host. The Shared tier was therefore attested per run as `CategoryStatus::CoverageGap` naming Story `13-5h`, which is what AC1 asked for. **Superseded by Decision 11:** `"shared"` is back in `REQUIRED_STORES`, not because a delete path appeared but because the partition makes the tier principal-empty by construction, and the producer now VERIFIES that per run before attesting.

The removal changes a **durable verification contract**, not just a producer: `verify_regional_teardown_receipt` independently re-checks `completed`. The contract is therefore stated explicitly and in one place. A regional teardown receipt attests exactly the stores named in `forget_cascade.stores_covered`. It is **not** a standalone all-tier Article 17 artifact. A consumer treating it as one MUST also read the companion erasure-proof bundle for the same run and honour its `CoverageGap` entries. `REQUIRED_STORES` and `UNCOVERED_STORES` partition `KNOWN_STORES`, asserted by `store_sets_partition_known_stores`, so the two cannot drift when a store moves between them — as `"shared"` did in `13-5h`. `UNCOVERED_STORES` is now empty and is retained deliberately as the single grep-able place a future no-erase-path backend is declared.

## Decision 9 — A held run attests what it destroyed

*Added by the 13.5b code review (party-mode consensus, 2026-07-25, 3/4 with Winston dissenting).*

A legal hold is a global boolean over the **principal**, not over the Spirit. Refusing an entire uninstall because one co-resident principal is held would convert that principal's litigation hold into an indefinite denial of an unrelated subject's Article 17 request. The cascade therefore continues to erase every unheld principal.

What changes is the artifact. A run that destroyed principal data always attests it. The `held` terminal carries `held_principal_ids`, `erased_principal_ids`, `deleted_entries`, `revoked_tokens`, and the path of a **partial** proof in which the held principals appear as `CategoryStatus::CoverageGap { reason }`. Exit code stays 3. No regional teardown receipt is written for a held run — a held run is not a teardown. A held run that destroyed nothing writes no proof at all, so "no proof" keeps meaning "nothing happened".

Consistent with Decision 6: a held uninstall still writes no lifecycle success and no *complete* proof.

## Decision 10 — Private-tier filesystem erasure is authoritative and exact

*Closed by Story 13.5i with operator-authorized Shape A4, 2026-07-26; hardened by the 13.5i code review, 2026-07-27.*

`PrivateMemoryStore::forget_principal` treats the on-disk tree as the authoritative superset and the in-memory map as a cache. It decodes every filesystem namespace through the same serde representation used by the writer, selects only `Principal` namespaces for the requested principal, fails closed on directory I/O, and refuses to traverse symlinks **at both the pid and the namespace level**. Production one-shot uninstall therefore erases filesystem-canonical Markdown and post-restart spills even though the newly constructed store has an empty cache.

`ForgetReceipt.deleted_entries` means distinct logical `(spirit_pid, namespace, key)` entries erased, counted exactly once whether an entry was cached, spilled, or both. Counting files under-reports inline-only PII; counting namespaces uses the wrong unit; a naive map-plus-files sum double-counts cached spills and overstates the signed Article 17 effect. The key is recovered by stripping the `file_ext_for_kind` extension `fs_path_for` appended, not by `file_stem()`, so the empty key round-trips instead of double-counting as `""` plus `".bin"`. Only regular files carrying a recognized extension are attested: the private tier's Markdown area is deliberately operator-editable, so a hand-created sub-directory or an editor backup beside a spill is destroyed with the subtree but is not a logical entry and must not inflate a signed receipt. The pre-13.5i `"private"` backend discharge was null control #24 because it wrote sub-threshold Text that never reached disk. Its replacement writes Markdown and a >4 KiB non-Markdown value, reopens the store, asserts content absence and bystander retention, and was falsified per limb before the source-line pin moved.

Shape B's serialized-prefix match was rejected because a serde field reorder would silently stop erasure while reporting success. Out-of-kernel Shape C was rejected because it duplicates the private encoding, bypasses both production inherent call sites, and cannot correct the count sealed inside the kernel. Index-driven Shape D was rejected because the index is a claim about the disk, while operator-created Markdown may have no index row. The formatted Shape A4 delta is +46 physical / +30 tokei (`23234 → 23280`, `17894 → 17924`), with the KLOC ceiling unchanged.

The independent stale-spill/duplicate-scan defect was carried by Story `13-5j` and is closed in Decision 12. Story 13.5i deliberately did not alter `write` or `scan`.

**Containment is a two-level property, and the review found the second level open.** The A4 landing type-checked only the pid entry. A symlink planted at `<fs_root>/<pid>/<hex-namespace>` decodes to the target principal, is *followed* by `read_dir`, has its external files counted into the erased set — and is then merely unlinked by `remove_dir_all`, which does not traverse. Measured on rustc 1.96.0. The result was an Ed25519-signed `Removed { count: n }` over bytes still on disk: this decision's own defect, one level down. The rule is now uniform — a name that decodes to the target namespace but is not a real directory is corruption or a containment attack and fails the erasure; a name that does not decode is junk and is skipped. Total 13.5i kernel spend against `dd4a908e` is +84 physical / +47 tokei (`23234 → 23318`, `17894 → 17941`), ceiling unchanged at 18248 with 307 spare.

## Decision 11 — The Shared tier is partitioned by an authorized +6 kernel delta

*Operator-ratified 2026-07-25 on a unanimous 4/4 panel. Supersedes Residual 2's "or namespace-aware erase" branch.*

Decision D names two tiers that must refuse subject-scoped PII and was implemented in one. `MemoryTier::Shared` accepts `MemoryNamespace::Principal` at all three trait arms while `SharedMemoryStore` has no delete at any visibility and `subject_access_query` reads only `principal_index` — so a Shared-tier principal row is neither erasable under Article 17 nor visible under Article 15.

The control that claimed otherwise is a **null control, measured not argued**: it plants its canary under `Coordination`, asserts that legitimate cross-Spirit row survives the forget, and then declares the tier principal-empty. Swapping only the canary's namespace to a principal one leaves the suite green — the discharge passes identically with and without unerasable PII in the tier.

**Fork resolution.** Namespace-aware *erasure* is rejected: no call site anywhere in the workspace writes `(Shared, Principal)`, `shared_memory` has no `principal_id` column, and the architecture places the principal namespace in the private tier. The subject set is empty, so erasure would be real design work for data that does not exist. **Partition wins.**

**Shape.** `reject_principal_collective` is generalized into `reject_principal_outside_private(tier, namespace)` and stated as a **negation of the allowlist**: `Principal` is legal only in `Private`, so a future tier is principal-rejecting by default and admitting one requires an explicit, reviewable change. One hoisted call sits above each of the three `match tier` blocks; the three in-arm collective calls are deleted; the three cap-gated call sites adopt the generalized helper so they cannot become an alternate bypass. A separate per-tier mirror was rejected 4/4 on architecture — it duplicates a single invariant and leaves a new tier's arm free to silently admit `Principal` — and independently cost +26 physical / +20 tokei, breaching the KLOC ceiling.

**Grant — SPENT AS AUTHORIZED.** Measured +6 physical (pin 23228 → 23234) and +4 tokei (17890 → 17894), landing under the then-standing 17 900 ceiling, so this grant did **not** depend on a ceiling move and `kloc.toml`'s extraction precondition was waived story-specifically rather than repealed. The ceiling has since moved to 18 248 under the founder-ratified policy replacement of 2026-07-25 (`measured + max(100, ceil(0.02 × measured))`, `kloc-check` flipped to BLOCKING); that is operating capacity, and it did not enlarge this grant. The pin moved only after the rewritten control was falsified **per leg**: deleting the hoisted predicate above the write, read and scan `match tier` block each turned `multi_backend_erasure_partition_invariant` red at its own assertion, proving all three independently load-bearing. The pre-13.5h control would not have redded at all, since it never wrote a principal namespace at Shared.

**Limit — the explicit position on unreachable-vs-erased.** The partition makes pre-existing Shared principal rows *unreachable*, not erased: `reject_principal_outside_private` refuses `Principal` at write, read and scan, but there is still no DELETE path in `SharedMemoryStore`, so a row written by a pre-partition build stays on disk. Refusal is nonetheless the correct behaviour — the row was never legitimately writable, and failing closed on read beats serving PII the erasure cascade cannot reach.

Because "unreachable" is not "erased", `CategoryStatus::VerifiedEmpty` is **earned, never asserted**. `maos_audit::shared_tier_principal_row_count` counts principal-namespaced rows in `shared_memory` — filtering on the namespace column, not the value blob — on every run. Zero rows yields `VerifiedEmpty` and admits `"shared"` to `stores_covered`. A non-zero count yields a `CoverageGap` stating the row count and that the rows are unreachable but not erased, withholds `"shared"` from `stores_covered`, and so drives `completed` false and refuses to sign a regional teardown receipt. That is fail-closed, and it means a Host carrying pre-partition residue cannot attest a completed teardown until the residue is removed out of band. Emitting `VerifiedEmpty` unconditionally would have rebuilt, in the fix itself, the very null control this decision exists to remove; the two behaviours are pinned against each other by `regional_uninstall_attests_shared_tier_verified_empty` and `regional_uninstall_refuses_to_attest_pre_partition_shared_residue`.

## Decision 12 — The private tier's read surface obeys the same rules as its erase surface

*Closed by Story 13.5j, 2026-07-27. Operator-authorized kernel delta, FLAG-Winston, `23318 → 23401`.*

Decision 10 made `forget_principal` authoritative about the filesystem and contained to its own directory. `write`, `read` and `scan` were left behind and disagreed with it in six ways, each confirmed against running code before the repair was designed. The rule is now uniform across all four functions:

**One logical key, at most one durable file.** `write` unlinks every spill for the key whose kind is not the one just written — after the new file is durable, so a crash in between leaves a recoverable superset rather than a gap. Without it, a value that changed kind or shrank below `inline_threshold` left its predecessor behind, and `read`'s fixed-order kind probe resurrected the superseded value on the next cold read (measured: `k.json` and `k.bin` coexisting, warm read `Blob`, cold read `Json`). This does **not** make sub-threshold values durable — those are process-lifetime working memory by design. It makes the durable state non-contradictory: after a write, disk holds this value or none, never a previous one.

**One logical key, one scan entry.** `scan` merges the in-memory cache and the filesystem *by logical key*, cache-first — the precedence `read` already applies. A union without that identity returned `["digest:aaa", "digest:aaa"]` for one key, and the sole production consumer, `memory_backed_digest_provider`, does not deduplicate: the duplicate rides into a signed `decision.*` frame's `working_memory_digest_refs` and halves the effective scan cap, so a Spirit with many digests would have had real refs silently dropped from an audit record.

**All four functions agree what a logical key IS.** `scan` recovered keys with `file_stem()` while `forget_principal` strips the `file_ext_for_kind` extension, so the empty key was invisible to one and counted by the other. Both now use the same stripping, over the single `ALL_KINDS` list, so the two cannot drift.

**Junk is skipped, never fatal and never attested.** The Markdown area is deliberately operator-editable, so a hand-created directory beside a spill is reachable residue. Reading it as a value failed the *whole* namespace scan with `IsADirectory`, and the digest provider swallows that into empty refs — a signed decision frame silently claiming the Spirit reasoned over nothing.

**Containment binds the read path too.** `scan`'s `is_dir()` and `read`'s `exists()` both traverse symlinks, so a link inside a Spirit's own area read entries from outside it (I5). Both now use `symlink_metadata`. The asymmetry with `forget_principal` is deliberate: erasure *fails* on a namespace symlink because a miscount would be signed into an Article 17 receipt, while a read simply reports nothing, which is already the safe direction.

**Severity is corrected, not inflated.** The 13.5i residual filed this as a live data-integrity and data-exposure bug. It is **latent at HEAD**: no shipped writer produces the trigger — the sole production private writer (`halt/resolver.rs`) uses unique `halt_context::` keys, and nothing writes the `digest:` keys the sole production scanner reads. The surface is nonetheless fully wired, since `KernelCtx::memory()` hands the adapter to Spirit-hosting code, so both halves are one Spirit write away. Ten `--exact` Blocking Reza legs bind the six properties; each was falsified independently (M1–M6), every mutant red at its own limb and green elsewhere, before the source-line pin moved.

## Residual register

| # | Residual | Status | Owner |
|---|---|---|---|
| 1 | Cross-team erase/hold fan-out; 13.6 both creates the first production crossing subject and judges this contract | ABSENT | Story 13.6 |
| 2 | Shared-tier principal partition; latent Article 15/17 hole and v2.2 blocker candidate | **CLOSED** by Story `13-5h`, landed 2026-07-25 (Decision 11). Grant spent exactly as authorized: +6 physical, +4 tokei. Guard falsified per leg; null control #23 replaced by a discriminating one | Story `13-5h` |
| 3 | Team-scoped legal holds and composite hold identity | ABSENT; do not emulate with global rows | Story 13.6 |
| 4 | Erasure correlation IDs and a multi-shard reconciliation reader | OPEN | Ownerless and open |
| 5 | CRDT-LWW resurrection after delete | CLOSED by transaction-serialized tombstones whose clock is `max(row_source_ts, now)`; the earlier bare-`now` stamp left a future-skewed-leaf window (13.5b review) | Story 13.5b |
| 6 | NFR-Ops-11(iii), per-operator capability-token signing key | OPEN | Ownerless and open |
| 7 | Crash atomicity between store mutation and audit append | OPEN; fail-fast and detected by one-sided reconciliation | Ownerless and open |
| 8 | Private-tier filesystem residue during one-shot uninstall (Decision 10) | **CLOSED** by Story `13-5i`, landed 2026-07-26, hardened 2026-07-27. Shape A4: authoritative decode walk, fail-closed I/O, two-level symlink containment, exact-once logical-key count; null control #24 replaced and M1–M11 falsified before each re-pin (`23234 → 23280 → 23318`) | Story `13-5i` |
| 10 | Private-tier stale spill and duplicate scan; the read surface disagreed with the erase surface (Decision 12) | **CLOSED** by Story `13-5j`, landed 2026-07-27. Six defects confirmed against running code, six fixed, ten `--exact` Blocking legs, M1–M6 falsified per limb before the re-pin (`23318 → 23401`). Severity corrected from "live" to latent-but-fully-wired | Story `13-5j` |
| 9 | Private-tier forget is not atomic across its two stores, and a failed attempt is unaccounted | OPEN. The in-memory removal completes before the first fallible filesystem operation, so a ≤4 KiB RAM-only value is destroyed even when `forget_principal` returns `Err` (13.5b D-4 consequence 1 — 13.5i closed only consequence 2, the self-sealing one, because the walk no longer depends on the map). A retry after the operator clears the fault cannot see those already-purged cache entries, so its signed `deleted_entries` under-reports what the failed attempt destroyed. Raised by the 13.5i code review; not a spec violation, and deliberately not absorbed | Ownerless and open |

## Rejected alternatives

- **Principal-scoped collective forget:** rejected; it reverses Decision D and creates the forbidden substrate.
- **One process fans out to every team database:** rejected; it violates ADR-055's physical authority boundary and no all-team port exists.
- **Kernel erase arm or new capability Scope:** rejected; operator authority already exists at the composition root, while Spirit authority does not.
- **Hard delete without tombstone:** rejected; stale replication deterministically resurrects the row.
- **Shard-local legal-hold fallback:** rejected; an empty local table is a fail-open answer.

## Consequences

- The Reza production gate now carries hermetic partition, terminal, hold, authority, and one-sided-reconciliation legs plus live Postgres partition/erase witnesses.
- Missing live substrate remains `AdvisorySubstrate` locally and blocks when provisioned in CI.
- Shared-tier principal state is closed by Decision 11's authorized partition, private-tier filesystem erasure by Decision 10's authorized Shape A4 repair, and the private tier's read surface by Decision 12. Cross-team fan-out, team-scoped holds, cross-shard correlation, and the non-atomic forget of Residual 9 remain named limits; no proof or operator document may claim them closed.
