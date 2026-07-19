---
Status: ratified-v2.2
Gate: Story 13.1 — `check-multi-tenant-loom`, blocking at v2.2
Decided: 2026-07-17
Accepted-in-PR: Story-13.1
Extends: ADR-054 (team placement hand-off at §1)
Reuses: ADR-049 (independent-verifier and source-identity discipline), ADR-012 (typed-intent consent)
---

# ADR-055 — Multi-tenant Loom physical tenant wall

## Context

ADR-054 owns the signed cohort roster and explicitly hands the team↔region↔database mapping to this decision. Reza is one governed cohort with stable Spirit identities partitioned into teams. A shared table with a team predicate is insufficient: one omitted `WHERE` clause would cross the boundary.

The production collective store is Postgres-only. The kernel exposes one `CollectiveMemoryPort`, not a per-team router, and no production Spirit→collective route exists yet. Story 13.1 therefore ships and proves the storage mechanism per store. **Story 13.5c makes the cohort daemon a single composition root so the verified manifest reaches the tenant map and `MAOS_LOOM_HOME_TEAM` boots — it owns refresh wiring only.** Production multi-store selection and Spirit mediation are **Story 13.5d**; per-operator tenant-refusal audit isolation is **Story 13.5e**.

## Decision

### 1. One signed artifact, two explicit schemas

The cohort reader accepts exactly `COHORT_SCHEMA_V1 = 1` and `COHORT_SCHEMA_V2 = 2`.

- v1 iff `teams` is absent. Its canonical body and `SIG_DOMAIN_V1 = b"maos.cohort-manifest.v1"` remain byte-for-byte frozen.
- v2 iff `teams` is present and non-empty. It signs under `SIG_DOMAIN_V2 = b"maos.cohort-manifest.v2"` and appends canonical teams after the unchanged common fields.
- An unknown schema is rejected before strict full deserialization. A schema/team-shape mismatch is a distinct typed refusal.
- Tenant-enabled composition requires a fresh, locally verified v2 manifest containing the local host. There is no v1 tenant fallback.

Team entries are sorted by canonical `TeamId` and member references by stable `SpiritId` only for canonical encoding; parsed declaration order is not mutated. `TeamId` is the shared pure `maos-domain` identity with frozen grammar `^[a-z0-9-]{2,32}$`. Each team declares one canonical `Region`, one unique canonical Postgres `datname`, and a non-empty disjoint Spirit member set.

### 2. Dual-reader-first rollout and downgrade refusal

Rollout order is binding:

1. deploy the `{v1,v2}` reader to every cohort host while the authority continues issuing v1;
2. confirm every host is upgraded;
3. issue a signed v2 reissue;
4. each member verifies and locally journals `MemberReissueAccepted`;
5. only then enable tenant stores.

An old v1 reader fails closed on v2; premature issuance is an availability split, not successful compatibility. After a node has accepted v2, any v1 reissue is rejected and locally audited as `SchemaDowngrade`, even when its manifest revision is higher. Recovery is a higher v2 revision. Restarted tenant mode also refuses v1.

### 3. Physical boundary and single guard

A team owns the rows physically present in its database. Every `LoomLiteStore` has exactly one pool and one `home_team`; it cannot name another team's database. `team_guard` is a private, pre-query, per-call refusal at exactly the Spirit-facing `write`, `read`, and `scan` methods. It compares the live pid's manifest-resolved stable Spirit team with the store's `home_team`.

`write_with_source`, `read_all_rows_from`, and `pool()` remain deliberately unguarded infrastructure paths. They operate only on the store's one configured database and are used for verified replication, convergence, schema work, and the physical-absence witness. The Spirit-facing `CollectiveMemoryPort` stays unchanged.

At construction, the manifest assignment for `home_team` must equal `SELECT current_database()`. A mismatch is `StoreError::TenantConnectionMismatch`. Stale state and unmapped live pids are separate typed store refusals. The current port error vocabulary is consciously lossy and carries these as `Transport(reason)`; Story 13.3 may widen the caller-facing taxonomy if cross-team mediation requires it.

### 4. Staged trust semantics and the D1 team-axis closure (Story 13.2)

Story 13.1 verifies manifest authority, team presence, stable identity membership, and physical database assignment. It does **not** derive or verify per-team cryptographic keys; at 13.1 a same-region insider able to forge a team stamp is served, and the `d1-forged-stamp-served-boundary` leg documents that (a rejection there would either absorb 13.2 or be tautological).

**Story 13.2 (Fork-4 / ADV-055-1) closes the forgery at ENTRY (Option A).** It adds three pieces atomically: (a) a per-team Ed25519 signing key derived by a **second HKDF-SHA256 stage** whose IKM is the region signing seed (`derive_team_signing_seed`/`derive_team_pubkey` in `maos-audit::sealed_export`, welded over `derive_region_signing_seed`); (b) `canonical_kv_leaf` **v2** — `source_team` enters the leaf pre-image under the `maos.collective-kv-leaf.v2` domain tag, while a `None`-team (v1) leaf stays byte-identical so 11.2a cross-region convergence for existing rows is unchanged; (c) `verify_replication_bundle` derives the verifying key from the bundle's **claimed** `(region, team)` — never a key the bundle carries (R-RG1). A team-B member who stamps `source_team = A` (same region) cannot sign under team-A's derived key, so the bundle is refused at `verify` with `BundleError::SignatureVerificationFailed`; because the public `apply_replication_bundle` verifies before invoking its private row-apply helper, the forged row **never lands**. The team verifying key is **derived, not stored**: the manifest remains the authority for *placement*, the operator seed the root for *keys*, and the two trust roots stay separate (no cross-wiring the pinned cohort-authority key at `maos-cohort/src/pin.rs`).

The key is **derived from the claimed identity, never looked up** — there is no team-pubkey field on any port and the store holds neither `base_seed` nor any verifying key. `team_guard` is **unchanged** (still presence-only, three sites); the read-path row-level guard is NOT introduced here.

**Scope of the 13.2 closure and the 13.3 hand-off.** 13.2 closes D1 on the **team axis at entry only**. It does **not** close: the read-time D1 residual on either axis (nothing is persisted per row for a read to verify — `collective_memory` carries no signature/root/attestation column); the region `source_log_ref` presence residual, which stays **OPEN with no named successor** (the trusted-applied-root registry that was its successor was cut in preflight). Per-row attestation persistence (`source_team` + `merkle_root` + `region_sig` + Merkle inclusion path), the read-path verify-with-inclusion, and production seed→store + cross-team apply wiring all land with **Story 13.3**, which builds the real cross-team write that gives a read guard something to verify. 13.2 supplies the `source_team` provenance and the entry-verify those depend on.

### 5. Freshness and boot posture

Tenant map lookups require both lease freshness and local-host membership in the current verified roster. A peerless/N=1 source is not refreshable under the shipped authority model and is refused at boot. In the primary daemon, configured tenant mode without a refreshable source fails immediately; non-tenant configuration remains quietly disabled. Hot-path heartbeats and a new announcement protocol are rejected.

The cohort daemon is **not** a parallel composition root. `run_cohort_a2a_daemon` is dispatched from *inside* `async fn main`, so a `cohort-a2a-daemon` process already runs the entire primary root — Transparency Log, boot nonce, store, memory manager — before reaching the dispatch, then discarded it and rebuilt a second set. **Story 13.5c closes this by parameter-passing, not by joining two roots:** it loads the verified `CohortManifestState` above the store, constructs the `TenantMapAdapter` from it (13.1's physical wall and 13.2's cryptographic wall go LIVE for the first time — `TenantMapAdapter`'s first production construction), and hands the same `Arc` plus the primary Transparency Log and boot nonce to the daemon function, deleting the daemon's second Transparency Log. Two consequences a future reader must not mistake for regressions: (a) the daemon's A2A boot nonce becomes **per-boot random and single-sourced** where it was an operator-static `own_boot_nonce` — a deliberate behavior change repairing the NFR-Rel-6 restart detector (`router.rs:897` `invalidate_if_boot_nonce_differs`), scoped to *per-boot random, single-sourced* (no test drives restart detection through a live transport, so peer-side detection is not claimed); (b) tenant mode now **BOOTS but does not SERVE** — `register_spirit` has no production caller until Story 13.5d, so every collective read/write/scan fails closed with `TenantSpiritUnmapped`. Refresh wiring closes here; production Spirit→collective routing is **Story 13.5d**.

## Consequences and carried gaps

- Database-per-team makes cross-team leakage impossible to express through a second table predicate; the gate witnesses distinct `current_database()` values and physical absence through the unguarded provenance reader.
- `maos-loom-lite` and `maos-cohort` gain no dependency edge. The adapter lives in `maos-bin`; no crate is added.
- No `maos-kernel-core/src` line moves; the baseline remains 23202.
- Story 13.2 adds per-team key derivation (`maos-audit::sealed_export`), `canonical_kv_leaf` v2, and bundle v2 verify-from-claimed-`(region,team)`, all out-of-kernel; the baseline stays 23202, `WriteEntryPoint` is untouched, and the crypto refusal reuses `BundleError::SignatureVerificationFailed` (no new `maos-domain` `E*`).
- Story 13.2 closes the same-region cross-team forgery at ENTRY (D1, team axis). The **read-time** D1 residual (both axes) and per-row attestation persistence defer to Story 13.3. The **region** `source_log_ref` presence residual remains OPEN with **no named successor** (its intended trusted-applied-root registry was cut in preflight).
- Collective GDPR erase/legal-hold reach does not exist. Story 13.5b owns the port and kernel cascade.
- Tenant refusal auditing and per-operator Transparency Log isolation do not exist below the store port. **Story 13.5e** owns them. An unaudited refusal is a named intermediate gap, not ADR-055 completion.
- Production multi-store routing and a Spirit-facing collective path remain **Story 13.5d**.
- **Story 13.5c closed refresh wiring** — single composition root, one Transparency Log, one per-boot nonce, and the first production construction of `TenantMapAdapter` (13.1/13.2's walls go live). It changed the A2A boot nonce to per-boot random/single-sourced (NFR-Rel-6, §5). Tenant mode BOOTS but does not SERVE until 13.5d wires `register_spirit`. Baseline stays 23202; ~180 LOC `maos-bin` + ~300 LOC test, no new gate, crate, or dependency.

## Rejected alternatives

- **Shared database/table plus team predicate:** rejected; one omitted predicate defeats the wall.
- **Separate tenant manifest:** rejected; it creates competing authorities and rollout lineages.
- **Sign runtime `spirit_pid`:** rejected; pids are ephemeral and unknown to the operator.
- **Direct Loom↔cohort dependency:** rejected; it couples storage to the A2A control graph.
- **Silent v1 fallback or lease expiry after boot:** rejected; both make tenancy availability or isolation implicit.
