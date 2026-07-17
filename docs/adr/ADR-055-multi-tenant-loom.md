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

The production collective store is Postgres-only. The kernel exposes one `CollectiveMemoryPort`, not a per-team router, and no production Spirit→collective route exists yet. Story 13.1 therefore ships and proves the storage mechanism per store. Story 13.5c owns production multi-store selection, Spirit mediation, refresh wiring, and tenant-refusal audit isolation.

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

### 4. Staged trust semantics and D1 caveat

Story 13.1 verifies manifest authority, team presence, stable identity membership, and physical database assignment. It does **not** derive or verify per-team cryptographic keys. A same-region insider able to forge a team stamp remains live until Story 13.2. The boundary test must demonstrate that such a forged stamp is still served; a test claiming rejection would either absorb Story 13.2 or be tautological.

Story 13.2 atomically adds `source_team`, canonical leaf v2, and per-team key derivation. Until then Story 13.1 may claim physical absence and chokepoint singularity, not forge resistance or valid re-attestation.

### 5. Freshness and boot posture

Tenant map lookups require both lease freshness and local-host membership in the current verified roster. A peerless/N=1 source is not refreshable under the shipped authority model and is refused at boot. In the primary daemon, configured tenant mode without a refreshable source fails immediately; non-tenant configuration remains quietly disabled. Hot-path heartbeats and a new announcement protocol are rejected.

The current cohort daemon owns the only refresh loop but is a parallel composition root with separate state and audit wiring. Joining that source to production team-store routing is explicitly Story 13.5c.

## Consequences and carried gaps

- Database-per-team makes cross-team leakage impossible to express through a second table predicate; the gate witnesses distinct `current_database()` values and physical absence through the unguarded provenance reader.
- `maos-loom-lite` and `maos-cohort` gain no dependency edge. The adapter lives in `maos-bin`; no crate is added.
- No `maos-kernel-core/src` line moves; the baseline remains 23202.
- Collective GDPR erase/legal-hold reach does not exist. Story 13.5b owns the port and kernel cascade.
- Tenant refusal auditing and per-operator Transparency Log isolation do not exist below the store port. Story 13.5c owns them. An unaudited refusal is a named intermediate gap, not ADR-055 completion.
- Production multi-store routing and a Spirit-facing collective path remain Story 13.5c.

## Rejected alternatives

- **Shared database/table plus team predicate:** rejected; one omitted predicate defeats the wall.
- **Separate tenant manifest:** rejected; it creates competing authorities and rollout lineages.
- **Sign runtime `spirit_pid`:** rejected; pids are ephemeral and unknown to the operator.
- **Direct Loom↔cohort dependency:** rejected; it couples storage to the A2A control graph.
- **Silent v1 fallback or lease expiry after boot:** rejected; both make tenancy availability or isolation implicit.
