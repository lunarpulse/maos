# Loom multi-tenant threat model

**Status:** binding for ADR-055 / Story 13.1 (physical wall) + Story 13.2 (cryptographic team boundary, closed at entry against a **seed-less** forger) + Story 13.6a (authenticated team identity — `COHORT_SCHEMA_V4` host→team edge enforced on both A2A seams). Read-time closure remains Story 13.3; the production cross-team crossing remains Story 13.6b.

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

### T1 — Same-region insider-team provenance forgery: CLOSED against a SEED-LESS forger (Story 13.2), and the seed-holder gap is closed on the IDENTITY axis by Story 13.6a

At Story 13.1 a member of team B in the same region could forge a non-empty provenance-presence stamp accepted by the 11.2b `region_guard`, because 13.1 verifies manifest membership and physical placement, not a per-team signing key. The `d1-forged-stamp-served-boundary` leg documents that residual by requiring the forged stamp to be **served** through a valid same-team read.

**Story 13.2 (Fork-4 / ADV-055-1) closes this on the team axis at ENTRY (Option A) — against a forger who does NOT hold the replication base seed.** A per-team Ed25519 key is derived by a second HKDF-SHA256 stage over the region seed; `source_team` enters the `canonical_kv_leaf` v2 pre-image (v1 leaves byte-identical); and `verify_replication_bundle` derives the verifying key from the **claimed** `(region, team)`. A team-B member that stamps `source_team = A` in the **same region** and signs under **team-B's** key cannot satisfy team-A's derived key, so the relabelled bundle is refused at `verify` (`BundleError::SignatureVerificationFailed`) and never applied. This is proven by the `forged-team-stamp-refused-at-verify` gate leg (same-region, independently-derived verifier + positive control — not a cross-region tautology).

**⚠ SCOPE CORRECTION (Story 13.6a).** The sentence "same-region cross-team forgery is cryptographically infeasible" was **too strong** and is withdrawn. `derive_team_signing_seed(base_seed, region, team)` (`maos-audit/src/sealed_export.rs`) accepts **any** `(region, team)`, so a holder of `base_seed` derives the *correct* team key and signs correctly. The shipped 13.2 negative forges by **relabel** — sign under team-B, swap the label to team-A — which is a **seed-less** forger; that control and the seed-holder threat do not intersect at any point. 13.2's close therefore reads: *cryptographically infeasible for a forger without the base seed.*

**Blast radius (Vex, non-negotiable; extended by Story 13.6a from a BREACH to a DESIGN PROPERTY).** Welding the team key over the region signing seed means **region-seed compromise is team-wide** — an attacker who recovers a region's signing seed can derive every team key in that region. As originally written this was scoped to an attacker who *recovers* the seed. That scoping no longer holds: the cross-team crossing (Story 13.6b) requires **every emitter to hold `MAOS_CROSS_TEAM_BASE_SEED` by design**, so seed possession is the normal operating condition of a crossing emitter, not a breach. The bundle signature therefore proves *"signed by someone holding the seed"*, never *"signed by team X"*.

**Story 13.6a closes the gap on the IDENTITY axis, not the key axis.** `COHORT_SCHEMA_V4` adds an operator-signed `CohortMember.team` — the first host→team edge in the signed manifest — and both A2A seams enforce it: `prepare_outbound` stamps the crossing's source team from the **local** host's own signed declaration (never from caller input), and `handle_intake_verified` refuses a frame whose claimed source team is not the team the **TLS-verified peer** speaks for, under its own code `CODE_TEAM_IDENTITY_MISMATCH` (-32010), distinct from the host-axis `CODE_PEER_IDENTITY_MISMATCH` (-32007). The binding is safe against a seed-holder because it rides `CohortMember.fingerprint`: operator-pinned, cert-bound, and **not** derived from `base_seed`. team-a's host cannot present team-b's certificate. Absence is fail-closed — a member with no declared team cannot originate a crossing at all.

**The surviving limit, stated plainly.** A seed-holder can still forge **within its own team**, and an operator who controls a host controls that host's team. **Isolation is enforced against a peer, not against the operator** — which is the correct scope for a single-org product (Reza is one operator, so no Reza claim is made truer by closing it), and it is written down here rather than implied. True per-team key **provisioning** — each host holding only its own team's key material — stays in the `v25-signed-shard` family (ADR-055 residual #6, v2.5). No artifact may say "Fork-4 proven" of the crossing path.

**Not a blanket close.** 13.2 closes D1 on the **team axis at entry** only. The **read-time** D1 residual (both axes) is NOT closed — `collective_memory` persists no per-row signature/root/attestation a read could verify — and defers to Story 13.3, which builds the real cross-team write plus per-row attestation (`source_team` + `merkle_root` + `region_sig` + Merkle inclusion path) and the read-path verify-with-inclusion. The **region** `source_log_ref` presence residual (a different axis) remains **OPEN with no named successor**: the trusted-applied-root registry that was its successor was cut in preflight. `team_guard`'s read path stays presence-only (13.1); no read-path row-level guard is introduced at 13.2. Story 13.6a introduces **no** read-path guard either — it binds identity on the A2A seams, and the crossing itself does not yet exist.

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

Story 13.1 has no cross-team read path, and Story 13.2 closes the forgery at bundle-*entry* (not at read) — so these rules still define the required successor boundary for the mediated cross-team **read** path that Story 13.3 will build, rather than pretending a non-existent read path is protected. 13.2 does add the crypto foundation (declared source-team + derived-key verification) those rules' "declared source team" labelling will consume.

## Carried security debt

- **F16 / Story 13.5b — collective cascade gap:** `CollectiveMemoryPort` still has no delete/legal-hold surface and kernel forget does not reach Collective memory. No GDPR cascade claim is permitted.
- **F14 / Story 13.5e — team-axis Transparency Log boundary:** per-team physical artifacts, manifest-reconciled sidecar binding, scoped recall, correlation, and wrong-team backup refusal are shipped. SQLite cryptographic integrity remains absent; escape-anomaly wiring is Story 11.4b and collective erase/legal-hold fan-out is Story 13.5b.
- **Story 13.5d — production routing residual:** the first-party Spirit→team-store route is shipped; cross-team replication still has no production initiator.
- **Story 13.3 — error taxonomy gap:** tenant refusals are consciously compressed to `CollectivePortError::Transport(reason)` at the existing port boundary.
- **Story 13.6 — journey gap:** no three-team Reza product flow is claimed by the mechanism gate.
- **Story 13.6a — operator-trust limit (OPEN, documented, deliberately out of scope):** isolation is enforced **against a peer, not against the operator**. A `base_seed` holder can still forge within its own team, and an operator who controls a host controls that host's team. Per-team key **provisioning** is `v25-signed-shard` (ADR-055 residual #6, v2.5).
- **Story 13.6b — crossing initiator residual:** the cross-team crossing still has no production initiator. `replication-crossing-has-no-production-initiator` and `cross-wall-recall-no-production-caller` remain **green**; 13.6a inverts neither.

These are explicit residual declarations in `check-reza-production-path`, not silent follow-up assumptions.
