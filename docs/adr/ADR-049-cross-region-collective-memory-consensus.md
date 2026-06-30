---
Status: Proposed — architecture ratified 2026-06-29 (Epic 11 party-mode, decision §2); binding-v2.0 at Story 11.2a. Authored in the v1.5 hold-window per the Epic 11 ratification.
Gate: Story 11.2a — cross-region propagation is mediated re-attestation (no transparent row replication); independent per-region convergence oracle (reuse the 10.4b canonical-leaf/Merkle/payload/row-count oracle) re-derives byte-identical state; region-identity reflex (each propagated row's source-region identity verified, not just counted); AP-local-degrade proven-red; replication lives in `maos-loom-lite`, verification in `maos-audit` — kernel-core delta proven ~0 via abi-diff (one bounded FLAG-Winston `WriteEntryPoint` variant only if re-admitted cross-region writes need a distinct provenance arm).
Decided: 2026-06-29 (architecture); lands at Story 11.2a
Accepted-in-PR: <PR_NUMBER>
Supersedes: none (extends ADR-047 / the 9.4b TL-anchored trust root to the multi-region case)
Revisits: ADR-006 (kernel learns no patterns); ADR-009 (operator-local trust); ADR-047 (substrate-as-substrate); §9.4b region-pin
---

# ADR-049 — Cross-region collective-memory consensus (TL-anchored CRDT + mediated re-attestation)

**Decision.** Cross-region collective memory converges via a **TL-anchored CRDT** — a grow-only set of target tuples with **timestamp-aware last-writer-wins per target**, **per-region-sovereign**. Cross-region propagation is a **mediated re-attestation**, never transparent row replication: a mediator verifies a source-region bundle against that region's Transparency-Log anchor, re-admits it, **re-signs it under the destination region's key**, and stamps cross-region `source_log_ref` provenance. **Raft-style CP consensus is REJECTED.** Convergence is proven by **independent per-region re-derivation** of the Story-10.4b oracle triple (canonical-leaf serde + Merkle root + payload + row-count). On partition, the Collective tier **degrades locally** to Private/Shared with **no global halt**, fail-closed on region identity. Replication lives in `maos-loom-lite`, verification in `maos-audit`; **`maos-kernel-core` is untouched** save one possible bounded, compile-gated `WriteEntryPoint` variant.

## Context

Story 11.2a multi-instances the Loom collective tier across regions. The governing constraint is the **9.4b region weld** (ADR-047 §3, the TL-anchored trust root): the canonical region tag is mixed into the HKDF derivation of the per-region Transparency-Log signing key, and the resulting Ed25519 signature covers the region tag. This makes a region's data **cryptographically unusable** in another region unless it is deliberately re-attested under the destination key. Any "just replicate the rows" design is therefore not merely discouraged — it is **structurally impossible** to do silently, which is the property this ADR builds on rather than fights.

### Code grounding (survey, 2026-06-29) — and one correction to the ratified wording

This ADR is grounded in the landed tree, not an aspirational design:

- **Region type + canonical form.** `Region(String)` newtype, frozen `ascii-v1`, grammar `^[a-z0-9-]{2,32}$` (`crates/maos-domain/src/region.rs:39,44,56-80`; the encoding id is tripwire-frozen at `region.rs:284-288`). `RegionError::ERegionViolation { expected, found, detail }` (`region.rs:141-148`). (Note: the types are `Region` / `RegionError` — there is no `RegionTag`/`RegionSection`.)
- **The weld is HKDF→signing-key, in `maos-audit`, not kernel-core.** `derive_region_signing_seed(base_seed, region)` mixes `region.as_bytes()` into the HKDF-SHA256 `info` (`REGION_INFO_PREFIX = b"maos.region.ascii-v1:"`, salt `REGION_TL_SIGNING_SALT`) to produce the per-region Ed25519 signing seed; `derive_region_pubkey` is the verify-side companion (`crates/maos-audit/src/sealed_export.rs:11-42`). `maos-kernel-core` contains no HKDF.
- **CORRECTION — there is no AEAD/AAD.** The ratified plan §2 says the region tag is welded into "the TL signing key (HKDF info) **+ sealed-export AEAD AAD**." Against the code, **the AEAD half does not exist.** Under the re-ratified **Option A**, sealed TL/export artifacts are **signed, not encrypted**, and working-memory rows are **plaintext** (`crates/maos-audit/src/erasure/regional_teardown.rs:6-10`; D1 plaintext-at-rest waiver, ADR-047 §3). The actual region binding is two-fold: **(a)** HKDF `info` → the Ed25519 signing key, and **(b)** the region tag is **covered by the signature** (R-RG4′ tamper test, `sealed_export.rs:396-413`). This ADR commits the **sign-only** mechanism; it does **not** assume an AEAD AAD site (none exists, and citing one would be inventing code).
- **Fail-closed chokepoints already exist and are wired.** The non-wildcard `WriteEntryPoint { DirectWrite, ReplayApply{source_region}, BackupRestore{source_region} }` + `enforce_region` (`crates/maos-kernel-core/src/memory/write_entry_point.rs:26-86`) and the mirror `ReadEntryPoint` (`read_entry_point.rs:21-74`), wired into the live read/scan/subject-access paths (`memory/mod.rs:770-774, 824-828, 863-867`). A cross-region or untagged-foreign store access is rejected `ERegionViolation`; a foreign TL/export bundle fails Ed25519 verification under the wrong region key.
- **The convergence oracle is fully built** (Story 10.4b, byte-identity-proven): canonical-leaf serde (`crates/maos-loom-lite/src/canonical.rs:60-127`, pinned layout), payload oracle over the sorted multiset of row hashes (`canonical.rs:142-150`), Merkle root (`canonical.rs:160-165` → `crates/maos-audit/src/erasure/merkle.rs:40-102`), row-count + triple-oracle verify-before-commit (`maos-loom-lite/src/migration.rs:7-22`), and engine-independent readers that re-derive from **uncommitted** rows (`canonical.rs:185-326`). It is applied today to a SQLite→Postgres migration; it generalizes directly to region↔region.
- **`source_log_ref` provenance exists — intra-region only.** A `source_log_ref TEXT` column (`crates/maos-loom-lite/src/schema.rs:47`, CHECK-enforced for patterns at `:57-59`), the sealed-export `I11Content { source_log_ref, distillation_depth }` (`sealed_export.rs:67-71`), and the domain distillation types (`crates/maos-domain/src/distillation.rs:18`, `invariants/i11.rs:36`). It names a single region's TL frame_id. A **cross-region** provenance (naming a *foreign* region's anchor) extends this field's meaning and is greenfield.
- **CRDT-shaped store semantics exist, single-instance.** `store.rs:169-205` writes via `INSERT … ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key) DO UPDATE` over a `UNIQUE` target tuple (`schema.rs:51`) — a grow-only set of targets with **blind** LWW (unconditional overwrite, **no timestamp comparison**). True timestamp-aware merge across instances is greenfield.
- **Degrade primitives exist; no global halt.** `MemoryError::CollectiveNotYetAvailable` (the `:709` variant, `memory.rs:284-288`) and the typed `CollectivePortError::Unreachable`/`Timeout` (10.4a, `ports/collective_memory.rs:23-39`) surface collective-tier failure as a per-call typed `Err`; Private/Shared tiers are unaffected; **nothing halts globally.** An *automatic* Collective→Private/Shared downgrade router is greenfield.
- **Export/verify/sign surface exists; the re-attestation orchestration does not.** `sealed_export.rs` has `AuditBundle` (`:46-65`), `sign_bundle` (region-derived key, `:196-233`), `verify_bundle` (against a **caller-supplied** pubkey, `:235-272`), and the shared canonicalizer (`:278-320`). A mediator *could* `verify_bundle(source_pubkey)` then `sign_bundle(dest_region)`, but the **verify-source → re-admit → re-sign-dest → stamp-provenance** function does not exist. `regional_teardown.rs` (control-plane-key signing, region-neutral) is the closest precedent.

**Replication, CRDT-merge, gossip, anti-entropy, Raft — none exist** (the workspace sweep returns zero). So Raft is not being torn out; it was never present. The decision is which model to *build*.

## Decision

### 1. The model is a TL-anchored CRDT, per-region-sovereign

Collective memory is a **grow-only set of target tuples** `(spirit_pid, namespace, key)` (the existing `UNIQUE` keyspace, `schema.rs:51`) with a **last-writer-wins register per target**. To be conflict-free and convergent regardless of propagation order, LWW resolves by a **total order**: `(timestamp_ns, region_tag, frame_id)` lexicographically — replacing today's blind unconditional overwrite (`store.rs:188-191`) with a timestamp-aware compare. Every region is **sovereign over its own writes**; it never serves a foreign region's row that has not been re-attested under its own key.

### 2. Cross-region propagation = mediated re-attestation, never transparent replication

A row crosses a region boundary only via a mediator that:

1. **verifies** the source-region bundle against that region's anchor — `verify_bundle` with `derive_region_pubkey(source_region)` (`sealed_export.rs:235-272, 40-42`);
2. **re-admits** it through the destination's region chokepoint (the `WriteEntryPoint` provenance arm — see §6);
3. **re-signs** it under the destination region's key — `sign_bundle` with `derive_region_signing_seed(dest_region)` (`:196-233, 26-42`);
4. **stamps** cross-region `source_log_ref` provenance (the source region + its TL frame_id), extending the existing field (§Context).

Transparent row replication is **rejected and impossible**: a source region's rows carry signatures that do not verify under the destination key (the 9.4b weld), so a naive copy fails closed at verify time. Re-attestation is the *only* path across a boundary, and it is auditable (every re-admitted row names its origin).

### 3. Raft-style CP consensus is REJECTED

A cross-region Raft/quorum leader is rejected because it is **architecturally incompatible** with the substrate, on three independent grounds:

- **It contradicts the region weld.** Raft assumes a single replicated log copied verbatim across nodes. The 9.4b weld makes a verbatim foreign-region log cryptographically unusable; CP replication would require either dissolving the per-region signing keys (unwinding 9.4b/ADR-047) or re-signing on every entry (which is re-attestation, i.e. this ADR, not Raft).
- **It contradicts operator-local sovereignty.** A global consensus leader centralizes write authority across regions, contradicting ADR-009 (operator-local trust) and ADR-047 (substrate-as-substrate, no central authority). Regions are sovereign; there is no global leader to elect.
- **It sacrifices availability the substrate wants to keep.** CP halts the minority partition. The substrate's posture (air-gap-first, ADR-047; per-tier isolation, 10.4a) favors **AP-local-degrade** (§5): a partitioned region keeps serving its own and previously-re-attested data. CRDT is the AP-consistent model; LWW-register + grow-only-set are the canonical conflict-free types.

### 4. Convergence is proven by independent per-region re-derivation

Convergence is not asserted — it is **measured**. Each region independently re-derives the Story-10.4b oracle triple over its converged state — canonical-leaf bytes (`canonical.rs:81-119`), Merkle root (`merkle.rs:99-102`), payload oracle (`canonical.rs:142-150`), and row count (`migration.rs`). Two regions have converged iff **all** oracles match. The payload oracle (sorted multiset of per-row hashes) catches any single-byte divergence the frame-id-only Merkle root is blind to. This is the same byte-identity-proven machinery 10.4b shipped for SQLite↔Postgres, re-pointed region↔region — reuse, not rebuild.

### 5. Partition → AP-local-degrade, no global halt, fail-closed on region identity

On a cross-region partition, the Collective tier **degrades locally**: a region keeps serving its own + previously-re-attested data; un-propagated foreign writes are simply absent until the partition heals (grow-only + LWW means healing is a deterministic re-merge, no rollback). The degrade is a per-call typed condition (the existing `CollectiveNotYetAvailable` / `CollectivePortError::Unreachable` surfaces, §Context) — **never a global halt**, and Private/Shared tiers are unaffected. 11.2a authors the *automatic* Collective→Private/Shared downgrade router on top of these primitives. Degrade is **fail-closed on region identity**: a region under partition must never serve foreign-region data to satisfy a Collective read (that would be the transparent-replication failure §2 forbids).

### 6. Placement — out of kernel; one possible bounded kernel touch

Replication + the re-attestation orchestration + the timestamp-aware merge live in **`maos-loom-lite`**; verification + re-signing live in **`maos-audit`** (`sealed_export.rs`); the region primitives are in `maos-domain` (pure) + `maos-audit` (crypto). They surface to the kernel **only** through the existing `CollectiveMemoryPort` seam (ADR-006/ADR-041). **Expected `maos-kernel-core` delta = 0.** The single plausible exception: re-admitted cross-region writes may need a **distinct `WriteEntryPoint` provenance arm**. `ReplayApply { source_region }` already exists and may suffice; if a dedicated `CrossRegionReadmit { source_region, source_log_ref }` arm is required, the AC-9 **non-wildcard** enum design forces a deliberate, **compile-gated** kernel-core edit to `write_entry_point.rs` — a bounded FLAG-Winston re-pin, recorded in `kernel-core-baseline.toml` HISTORY with the named surface. Verified at 11.2a prep; likely reused, not new.

## Alternatives considered and rejected

- **Raft / CP quorum across regions** — rejected (§3): incompatible with the region weld, operator-local sovereignty, and AP-degrade; and moot (no CP code exists).
- **Transparent row replication (copy the Postgres rows / logical replication)** — rejected (§2): cryptographically fails closed under the 9.4b weld; would require dissolving per-region keys; loses provenance and auditability.
- **Vector-clock / multi-value register CRDT** — rejected for v2.0: a timestamp-ordered LWW-register + grow-only-set is sufficient for the collective-memory access pattern and reuses the existing target-tuple keyspace; multi-value conflict surfacing adds resolution UX with no journey demand. (Revisit if a use case needs concurrent-value preservation.)
- **Keep the current blind LWW** — rejected: unconditional overwrite is **not** convergent across regions (final state depends on arrival order); a total-order tiebreak is required for the CRDT property.
- **Host replication/re-attestation in `maos-kernel-core` "for the region chokepoint"** — rejected: the chokepoints already exist; verification belongs in `maos-audit` (where the keys are) and replication in `maos-loom-lite` (where the store is), per ADR-006/ADR-041 and the ADR-038 ceiling.

## Consequences

- **No transparent replication anywhere**; every cross-region row is re-attested and provenance-stamped — auditable by construction, and consistent with the 9.4b weld rather than fighting it.
- **The 10.4b oracle is reused** as the cross-region convergence proof — byte-identity-proven, no new trust in a hand-rolled comparison.
- **`source_log_ref` gains a cross-region meaning** (an additive extension of an existing field).
- **The blind LWW becomes timestamp-aware** (a `store.rs` merge change, in `maos-loom-lite`).
- **Kernel-core stays at ~0 delta**; the only possible touch is a compile-gated `WriteEntryPoint` arm (bounded, FLAG-Winston, verified at prep).
- **Region-identity reflex** (Epic-11 §9): a count gate over propagated patterns must verify each pattern's *source-region identity*, not merely count — the direct analogue of the language-identity reflex.
- **AP-degrade is a release-class behavior**, proven-red on a real partition (not a mocked unreachable).

## Gate

Binding at **Story 11.2a** (binding-v2.0):

- **Mediated re-attestation proven-red.** A foreign-region row that is *copied* (not re-attested) fails closed at verify; a *re-attested* row carries destination-key signature + cross-region `source_log_ref`. No transparent-replication path exists.
- **Independent convergence oracle.** Two regions' states are declared converged only when the 10.4b oracle triple (canonical-leaf + Merkle + payload + row-count) matches under **independent** per-region re-derivation. A planted single-byte divergence is caught by the payload oracle (proven-red).
- **Region-identity reflex** over propagated rows (source-region identity verified per row, not counted).
- **AP-local-degrade proven-red** on a real partition: Collective degrades to Private/Shared, no global halt, no foreign-region data served.
- **Kernel-core abi-diff proven-red** against baseline 22964 — delta 0, or a single named `WriteEntryPoint` arm with a HISTORY re-pin.
- Registered in `docs/adr/index.md`.

## Ratification

Architecture ratified by the Epic 11 party-mode consensus authority (Winston · John · Murat · Amelia + Lunarpulse sign-off, 2026-06-29, workflow `wyksr4yce`, decision §2), consistent with ADR-047 / 9.4b (TL-anchored trust root, the weld this design rests on), ADR-009 (operator-local sovereignty → no global leader), ADR-006/ADR-041 (replication + verification out of kernel), and ADR-038 (kernel ceiling). The design corrects the ratified §2 wording (sign-only weld, **no AEAD AAD**) against the landed code. Lands at Story 11.2a; binding-v2.0 follows the 11.2a gate. Drafted during the v1.5 hold-window (a ratified hold-window carve-out: ADR authoring has no Epic-11-dev dependency).
