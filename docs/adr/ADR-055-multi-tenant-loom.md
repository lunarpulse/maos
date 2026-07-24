---
Status: ratified-v2.2
Gate: Stories 13.1–13.3 — `check-multi-tenant-loom`, blocking at v2.2
Decided: 2026-07-17; amended 2026-07-20
Accepted-in-PR: Stories 13.1, 13.2, 13.3
Extends: ADR-054 (team placement hand-off at §1)
Reuses: ADR-049 (independent-verifier and source-identity discipline), ADR-012 (typed-intent consent)
---

# ADR-055 — Multi-tenant Loom physical tenant wall

## Context

ADR-054 owns the signed cohort roster and explicitly hands the team↔region↔database mapping to this decision. Reza is one governed cohort with stable Spirit identities partitioned into teams. A shared table with a team predicate is insufficient: one omitted `WHERE` clause would cross the boundary.

The production collective store is Postgres-only. The kernel exposes one `CollectiveMemoryPort`, not a per-team router. Story 13.5c made the cohort daemon a single composition root so the verified manifest reaches the tenant map and tenant mode boots. Story 13.5d added the mediated first-party Spirit route and registration. Story 13.3 adds the verified cross-team apply seam described below, but **no production replication initiator calls it yet**; refusal audit isolation remains Story 13.5e.

## Decision

### 1. One signed artifact, three explicit schemas

The cohort reader accepts exactly `COHORT_SCHEMA_V1 = 1`, `COHORT_SCHEMA_V2 = 2`, and `COHORT_SCHEMA_V3 = 3`.

- v1 iff `teams` is absent. Its canonical body and `SIG_DOMAIN_V1 = b"maos.cohort-manifest.v1"` remain byte-for-byte frozen.
- v2 and v3 require a present, non-empty `teams` map. V2 signs under the frozen `SIG_DOMAIN_V2 = b"maos.cohort-manifest.v2"`.
- v3 signs under distinct `SIG_DOMAIN_V3 = b"maos.cohort-manifest.v3"` and appends canonical directional `(from_team, to_team, intent)` grants after teams. Grants require declared unequal endpoints, canonical intent, and no duplicate ordered triples. `(A,B,intent)` never implies `(B,A,intent)`.
- Unknown schemas are rejected before strict full deserialization. Schema/team/cross-team-shape mismatches and each invalid grant condition are distinct typed refusals.
- Tenant-enabled composition requires a fresh, locally verified schema-v2-or-newer manifest containing the local host. There is no v1 tenant fallback.

Team entries are sorted by canonical `TeamId` and member references by stable `SpiritId` only for canonical encoding; parsed declaration order is not mutated. `TeamId` is the shared pure `maos-domain` identity with frozen grammar `^[a-z0-9-]{2,32}$`. Each team declares one canonical `Region`, one unique canonical Postgres `datname`, and a non-empty disjoint Spirit member set.

### 2. Dual-reader-first rollout and downgrade refusal

Rollout order is binding:

1. deploy the `{v1,v2,v3}` reader to every cohort host before the authority issues a higher schema;
2. confirm every host is upgraded;
3. issue a signed higher-schema reissue;
4. each member verifies and locally journals `MemberReissueAccepted`;
5. only then enable tenant stores.

An old reader fails closed on an unknown schema; premature issuance is an availability split, not successful compatibility. After a node accepts schema $N$, any lower-schema reissue is rejected and locally audited as `SchemaDowngrade`, even when its manifest revision is higher. Recovery is a higher revision at schema $N$ or newer. Restarted tenant mode likewise enforces schema v2 as a floor.

### 3. Physical boundary and single guard

A team owns the rows physically present in its database. Every `LoomLiteStore` has exactly one pool and one `home_team`; it cannot name another team's database. `team_guard` is a private, pre-query, per-call refusal at exactly the Spirit-facing `write`, `read`, and `scan` methods. It compares the live pid's manifest-resolved stable Spirit team with the store's `home_team`.

`write_with_source`, `read_all_rows_from`, and `pool()` remain deliberately unguarded infrastructure paths. They operate only on the store's one configured database and are used for verified replication, convergence, schema work, and the physical-absence witness. The Spirit-facing `CollectiveMemoryPort` stays unchanged.

At construction, the manifest assignment for `home_team` must equal `SELECT current_database()`. A mismatch is structured `StoreError::TenantConnectionMismatch`; stale state, unmapped live pids, consent denial, and invalid row attestation remain separate structured store refusals. Story 13.3 preserves the existing `CollectivePortError::Transport(_)` outer shape for kernel compatibility but replaces its lossy string with `TransportCause`, making `consent-denied`, `map-stale`, `attestation-invalid`, `unmapped-spirit`, and `connection-mismatch` distinguishable without string parsing. These refusals are caller-visible but remain unaudited; Story 13.5e owns refusal auditing and per-operator Transparency Log isolation.

### 4. Staged trust semantics and the D1 team-axis closure (Story 13.2)

Story 13.1 verifies manifest authority, team presence, stable identity membership, and physical database assignment. It does **not** derive or verify per-team cryptographic keys; at 13.1 a same-region insider able to forge a team stamp is served, and the `d1-forged-stamp-served-boundary` leg documents that (a rejection there would either absorb 13.2 or be tautological).

**Story 13.2 (Fork-4 / ADV-055-1) closes the forgery at ENTRY (Option A).** It adds three pieces atomically: (a) a per-team Ed25519 signing key derived by a **second HKDF-SHA256 stage** whose IKM is the region signing seed (`derive_team_signing_seed`/`derive_team_pubkey` in `maos-audit::sealed_export`, welded over `derive_region_signing_seed`); (b) `canonical_kv_leaf` **v2** — `source_team` enters the leaf pre-image under the `maos.collective-kv-leaf.v2` domain tag, while a `None`-team (v1) leaf stays byte-identical so 11.2a cross-region convergence for existing rows is unchanged; (c) `verify_replication_bundle` derives the verifying key from the bundle's **claimed** `(region, team)` — never a key the bundle carries (R-RG1). A team-B member who stamps `source_team = A` (same region) cannot sign under team-A's derived key, so the bundle is refused at `verify` with `BundleError::SignatureVerificationFailed`; because the public `apply_replication_bundle` verifies before invoking its private row-apply helper, the forged row **never lands**. The team verifying key is **derived, not stored**: the manifest remains the authority for *placement*, the operator seed the root for *keys*, and the two trust roots stay separate (no cross-wiring the pinned cohort-authority key at `maos-cohort/src/pin.rs`).

Story 13.3 keeps `team_guard` presence-only at its three Spirit-facing sites and adds a distinct two-site post-query `attestation_guard`. The store receives only public `(Region, TeamId)` verification keys derived at composition; neither `base_seed` nor a signing key crosses the port.

**Cross-team crossing semantics.** `apply_replication_bundle` verifies the v2 team signature first, then requires an explicit destination team and intent, rejects self/destination mismatch, and consults fresh directional consent in the same function that lands the row. The write persists the verified `source_team`, uses a distinct cross-team namespace detail, and adds a source-team equality condition to its LWW upsert so a destination first-party row cannot be clobbered. Per-row attestation columns bind the landed row to the verified bundle as described below.

**Kernel-mediated writes clarification.** This does not add a `WriteEntryPoint` or a new kernel-mediated cross-team operation. The apply seam is an infrastructure ingress whose signature, destination, and consent checks are colocated and fail closed. Story 13.5d's production Spirit route remains first-party; `build_replication_bundle*` and `apply_replication_bundle` still have no production initiator. The gate carries that dead-wire negative. The region `source_log_ref` presence residual remains OPEN with no named successor.

#### Story 13.3 per-row attestation: claims and posture

For a row carrying `source_team`, the destination persists the source leaf's canonical hash, Merkle root, region/team signature, bundle schema version, and inclusion proof. The post-query `attestation_guard` verifies inclusion and the root signature against composition-injected public `(Region, TeamId)` keys. The root seed remains above the store boundary.

The read verifier is scoped to the default **plaintext-at-rest posture**. To bind the served database row to the signed leaf rather than merely trusting adjacent persisted hash columns, it reconstructs and hashes the complete canonical plaintext row before verifying inclusion. When an at-rest seal hook is configured, cross-team reads fail closed: the missing composition-root unseal path is owned by **Story 13.5a**. First-party rows keep nullable attestation columns and remain unaffected.

| May not claim | Binding limit |
|---|---|
| Authorization provenance | The bundle and per-row attestation establish origin integrity only. Consent is a separate apply-time decision. |
| Merkle multiplicity or ordering | `build_tree` sorts and deduplicates leaf hashes; inclusion proves set membership only. |
| Seal-independent cross-team reads | Configured sealing refuses cross-team reads until Story 13.5a provides unseal. |
| Region-axis closure | The existing `source_log_ref` presence residual remains open and the forged-stamp live leg remains served. |

#### Story 13.3b origin-provenance and recall amendment

Story 13.3b adds a third, data-presence-selected leaf canonical form. A source-team leaf carrying both copied `distillation_depth` and I13 `intent_lineage` signs those fields under `maos.collective-kv-leaf.v3`; v1 and v2 bytes remain frozen. The destination persists and rebundles those fields. The region `source_log_ref` column remains a destination-local re-attestation stamp and is not the flattened I11 frame-ref chain. As required by ADR-049 §7, these fields establish provenance-of-origin only, never provenance-of-authorization.

Raw-frame access behind another team's wall is a separate directional capability governed by ADR-058. Existing emitter-scoped `log.recall` remains unchanged; `recall_cross_wall` requires a fresh exact `(home_team, remote_team, "log:recall")` manifest grant and returns typed refusal reasons distinct from an empty page. The consent adapter lives at the composition root; `maos-iac` remains free of `maos-cohort` and `maos-loom-lite` dependencies. This does not establish per-team Transparency Log isolation or refusal journaling.

#### Story 13.5e per-team Transparency Log authority

Story 13.5e makes the **team** the physical Transparency Log addressing operand without putting `TeamId` into the frozen kernel API. When and only when both `MAOS_LOOM_POSTGRES` and a non-empty canonical `MAOS_LOOM_HOME_TEAM` are present, the global audit root resolves to `teams/<team>/<audit-file>`. Untenanted mode retains the historical global path byte-for-byte. The manifest-team **authority** is the store's `connection_assignment_guard` — datname validated against the verified cohort manifest, fail-closed in `init_schema`. Stage 1 is provisional addressing; Stage 2 reconciles the opened path as a **defense-in-depth drift check** (today both operands derive from the configured home team, so it guards path/team drift, not a manifest disagreement — see Story 13.5e review D1) and writes a `<audit-file>.team` sidecar. The sidecar is a path-adjacent **label, not artifact identity** — it detects neither row mutation nor a foreign shard planted before first bind; `SQLITE_OPEN_NOFOLLOW` opens and a restore-time target-team check close the reachable planting vectors, but cryptographic shard identity (signed genesis) is **OPEN, deferred to v2.5, owner TBD**. An arbitrary `Path` still reads (the M5 addressing-as-control residual, OPEN for v2.5 capability-mediation).

The seven persistent tables keep their intended ownership. The team artifact owns `transparency_log`, `transparency_log_retractions`, `approval_decision_log`, and `schema_lifecycle_registry`. The Host-global artifact owns `shared_memory`, `principal_namespace_index`, and the principal-global `legal_holds`. The one team TL connection attaches the global hold database and removes its shard-local compatibility table; this preserves Story 9.2 hold semantics without opening a second Transparency Log or changing kernel-core.

Tenant backups embed canonical team provenance; wrong-team cold restore refuses by validating the target path's implied team against the backup's declared team **before** any bytes are copied. NFR-Ops-9 now fans out from one Host artifact to $N$ team artifacts: operators must enumerate every manifest team and retain each sidecar/backup provenance record; the gate proves one-team round-trip and wrong-team refusal, not fleet-wide RPO/RTO scheduling. Cross-team actions define an additive nullable `correlation_id` + an `insert_frame_event_with_correlation` writer and a two-log reconciler; **the production producer that mints the ID onto both team logs is Story 13.5d's route (which landed without it) — deferred as a named correct-course, so until wired real rows carry NULL and the reconciler is API-only**. `reconcile_correlated_frames` provenance is caller/path-asserted, not cryptographically verified. The existing SQLite log remains append-oriented, not cryptographically tamper-evident: intentional row mutation is an explicit residual, escape-anomaly wiring belongs to Story 11.4b, and collective erase/legal-hold fan-out belongs to Story 13.5b.

This serves NFR-Ops-11 on the **team axis only**. Per-operator namespace, capability-token signing keys, and GDPR-erasure scope remain open; no v2.2 artifact may claim the full multi-operator NFR closed.

### 5. Freshness and boot posture

Tenant map lookups require both lease freshness and local-host membership in the current verified roster. A peerless/N=1 source is not refreshable under the shipped authority model and is refused at boot. In the primary daemon, configured tenant mode without a refreshable source fails immediately; non-tenant configuration remains quietly disabled. Hot-path heartbeats and a new announcement protocol are rejected.

The cohort daemon is **not** a parallel composition root. `run_cohort_a2a_daemon` is dispatched from *inside* `async fn main`, so a `cohort-a2a-daemon` process already runs the entire primary root — Transparency Log, boot nonce, store, memory manager — before reaching the dispatch, then discarded it and rebuilt a second set. **Story 13.5c closes this by parameter-passing, not by joining two roots:** it loads the verified `CohortManifestState` above the store, constructs the `TenantMapAdapter` from it (13.1's physical wall and 13.2's cryptographic wall go LIVE for the first time — `TenantMapAdapter`'s first production construction), and hands the same `Arc` plus the primary Transparency Log and boot nonce to the daemon function, deleting the daemon's second Transparency Log. Two consequences a future reader must not mistake for regressions: (a) the daemon's A2A boot nonce becomes **per-boot random and single-sourced** where it was an operator-static `own_boot_nonce` — a deliberate behavior change repairing the NFR-Rel-6 restart detector (`router.rs:897` `invalidate_if_boot_nonce_differs`), scoped to *per-boot random, single-sourced* (no test drives restart detection through a live transport, so peer-side detection is not claimed); (b) tenant mode now **BOOTS but does not SERVE** — `register_spirit` has no production caller until Story 13.5d, so every collective read/write/scan fails closed with `TenantSpiritUnmapped`. Refresh wiring closes here; production Spirit→collective routing is **Story 13.5d**.

## Consequences and carried gaps

- Database-per-team makes cross-team leakage impossible to express through a second table predicate; the gate witnesses distinct `current_database()` values and physical absence through the unguarded provenance reader.
- `maos-loom-lite` and `maos-cohort` retain no dependency edge. Manifest/consent/key adapters live in `maos-bin`; no crate is added.
- Story 13.3 moves no `maos-kernel-core/src` line. The current baseline is 23228 after Story 13.5d's separately authorized +26 delta; `WriteEntryPoint` remains four variants.
- Story 13.2 closes same-region cross-team forgery at entry. Story 13.3 adds directional consent, source-team persistence, namespace/clobber controls, and read-time team-axis attestation. The **region** `source_log_ref` presence residual remains OPEN with no named successor.
- Caller-visible refusal causes are structured inside the existing `CollectivePortError::Transport(_)` tuple, so kernel matching stays unchanged. Story 13.5e adds the per-team TL artifact boundary and manifest-reconciled artifact binding; refusal taxonomy remains out of kernel-core.
- Story 13.5d's mediated Spirit route and registration now serve first-party collective operations. Cross-team replication still has no production initiator.
- Story 13.3b carries signed v3 origin metadata, preserves the Story 8.10 citer-authorization default, fixes diamond traversal, and adds manifest-consented cross-wall recall. Story 13.5e now provides per-team TL isolation and team-scoped recall; collective erase/legal-hold fan-out remains Story 13.5b, and the three-team journey remains Story 13.6.
- Story 13.5c closed refresh wiring and made tenant mode bootable. Its per-boot random, single-sourced A2A nonce remains the NFR-Rel-6 restart detector.
- The gate has no dedicated xtask self-test that independently inventories its leg registry or cross-checks `ABSENT_SUCCESSORS` against this ADR/coverage matrix. That meta-gap is named, not silently claimed closed.
- Dead-wire clause `(f-ii) tenant-mode-unbootable` was inverted by Story 13.5c and is covered by the existing live boot leg. Clause `(f-i) no production crossing initiator` remains green with no assigned inverter.

## Rejected alternatives

- **Shared database/table plus team predicate:** rejected; one omitted predicate defeats the wall.
- **Separate tenant manifest:** rejected; it creates competing authorities and rollout lineages.
- **Sign runtime `spirit_pid`:** rejected; pids are ephemeral and unknown to the operator.
- **Direct Loom↔cohort dependency:** rejected; it couples storage to the A2A control graph.
- **Silent v1 fallback or lease expiry after boot:** rejected; both make tenancy availability or isolation implicit.
