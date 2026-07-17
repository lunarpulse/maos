# Loom multi-tenant threat model

**Status:** binding for ADR-055 / Story 13.1; staged cryptographic closure remains Story 13.2.

## Scope and security claim

Story 13.1 establishes a **physical database-per-team wall** for the production Postgres collective store. It proves that a Spirit-facing store call is admitted only when a fresh, locally verified cohort v2 manifest maps the live Spirit identity to the store's `home_team`, and that the store's connection names the manifest-authoritative database. It does not claim per-team cryptographic provenance, cross-team mediation, collective erase cascade, tenant-specific refusal audit, or product-journey completeness.

The row-ownership rule is physical: a row belongs to the team whose database contains it. There is no `source_team` column and no row-level tenant predicate. `write`, `read`, and `scan` share one pre-query `team_guard`; `write_with_source` and `read_all_rows_from` remain infrastructure paths confined to one configured database.

## Assets and trust boundaries

| Asset | Authority / boundary | Required property |
|---|---|---|
| Team identity and Spirit membership | Ed25519-signed cohort manifest v2 | canonical, non-overlapping, fresh, local host still rostered |
| Team database assignment | signed `TeamEntry.datname` plus `SELECT current_database()` | exact construction-time equality |
| Collective rows | one Postgres database per team | physical absence from every other team database |
| Live pid binding | daemon-owned pid→stable `SpiritId` registration | unknown pid refuses; pid is never signed manifest material |
| Manifest authority key | operator-pinned authority key set | compromise is cohort-wide control-plane compromise |
| Postgres credentials | operator deployment/KMS boundary | each store credential names only its assigned database |
| Refusal/audit evidence | current store error and future tenant audit sink | Story 13.5c must add tenant-scoped durable evidence |

## Threats, posture, and evidence

### T1 — Same-region insider-team provenance forgery: LIVE until Story 13.2

A member of team B in the same region can currently forge a non-empty provenance-presence stamp accepted by the Story 11.2b `region_guard`. Story 13.1 verifies manifest membership and physical placement, not a per-team signing key. The `d1-forged-stamp-served-boundary` gate leg therefore requires the forged stamp to be **served** through a valid same-team read. Rejection would be a false security claim or an accidental absorption of Story 13.2.

Story 13.2 closes this threat atomically by adding `source_team`, canonical leaf v2, and per-team key derivation. Until all three land together, Story 13.1 must not claim forge resistance or valid cross-team re-attestation.

### T2 — Cohort authority-key compromise

The authority can issue a valid v2 manifest that remaps every Spirit and database. A compromised authority key therefore defeats team placement globally; the store cannot distinguish maliciously authorized placement from legitimate reissue. Existing mitigations are operator-pinned key sets, signature verification, strictly monotonic revisions, local adoption audit, stale-lease refusal, and audited v2→v1 downgrade rejection. Operational response is authority-key rotation/reissue plus database credential containment. No runtime voting or member consensus is invented here.

### T3 — Malicious or compromised cohort member

A member may replay stale state, claim a removed role, register an unknown pid, attempt a cross-team call, inject rows through an infrastructure credential, or exploit another team's database credential. The adapter checks both freshness and local-host self-membership on every lookup; unknown pids and cross-team assignments refuse before query. Physical isolation still depends on least-privilege Postgres credentials and composition-root integrity. Direct database access remains outside the Spirit-facing wall and must be controlled operationally.

### T4 — Stale, split, or downgraded manifest

Expired leases refuse. An evicted local host stops serving even while its cached bytes exist. Tenant boot requires verified schema v2 with non-empty teams and never falls back to v1. Once v2 is cached, any v1 reissue is rejected and audited as `SchemaDowngrade` regardless of revision. Dual-reader-first rollout prevents intentional mixed-reader activation; disjoint valid forks remain bounded by ADR-054's single-authority-writer discipline and lease ceiling, not consensus.

### T5 — Connection misassignment and query omission

A store configured for team A but connected to team B refuses during `init_schema` when `TeamEntry.datname != current_database()`. The hot path never selects a database dynamically. A missing SQL tenant predicate cannot leak data because no such predicate exists; the gate proves distinct datnames and physical row absence with `read_all_rows_from`.

### T6 — Availability attack on the tenant map

Tenant mode with no refreshable source, a peerless/N=1 source, stale state, or a missing local roster member refuses. Non-tenant mode quietly preserves existing single-tenant behavior. This chooses isolation over availability and deliberately avoids hot-path heartbeats or a second announcement protocol.

## Sec-14a / Sec-14b extension to N-host cohorts

The existing NFR-Sec-14 split remains, extended from one Host or one bilateral pair to every member and team edge in the signed cohort:

- **Sec-14a — same Host:** pid rebinding, namespace enumeration, capability-token forgery, sandbox lateral movement, infrastructure-path abuse, connection-string substitution, and two Spirits on one Host assigned to different teams. Expected result: unknown/cross-team Spirit calls refuse before query; database assignment mismatch refuses before schema work.
- **Sec-14b — cross Host / N-host:** malicious member reissue replay, role or team spoofing, stale-manifest use, evicted-host serving, forged A2A source identity, and one compromised member attempting another team's database. Every directed member edge retains mTLS/TOFU and typed-intent consent from ADR-054; team placement additionally requires the same verified v2 manifest and local self-membership.

The corpus must vary member count, team count, host eviction, manifest revision skew, pid reuse, and connection assignment. A bilateral green result cannot stand in for the N-host matrix.

## OQ-8 prompt-injection rule-pack extension

The tenant wall creates a new tool-output boundary: collective rows may originate from another team after future mediation. Filter content remains data, not kernel code. The default rule pack must add these tenant-aware rules before a cross-team read path ships:

1. label every collective result with source class `loom.collective`, local team, declared source team when available, and manifest revision;
2. treat imperative text in collective values as untrusted data, never as system/developer instruction;
3. flag instruction-like content whose claimed team, provenance, or typed intent disagrees with its source metadata;
4. quarantine or require explicit consent for cross-team tool instructions, credential requests, policy overrides, and requests to suppress Transparency Log evidence;
5. preserve the original bytes and rule decision in the tenant-scoped audit projection without leaking another team's row content.

Story 13.1 has no cross-team read path, so these rules define the required successor boundary rather than pretending a non-existent path is protected.

## Carried security debt

- **F16 / Story 13.5b — collective cascade gap:** `CollectiveMemoryPort` still has no delete/legal-hold surface and kernel forget does not reach Collective memory. No GDPR cascade claim is permitted.
- **F14 / Story 13.5c — Transparency Log shard gap:** tenant-specific refusal audit and per-operator TL isolation are absent below the store port. Current typed refusals are not durable tenant evidence.
- **Story 13.5c — production routing gap:** the cohort refresh loop and Loom store live in parallel composition roots; production Spirit→team-store routing is not shipped.
- **Story 13.3 — error taxonomy gap:** tenant refusals are consciously compressed to `CollectivePortError::Transport(reason)` at the existing port boundary.
- **Story 13.6 — journey gap:** no three-team Reza product flow is claimed by the mechanism gate.

These are explicit ABSENT declarations in `check-multi-tenant-loom`, not silent follow-up assumptions.
