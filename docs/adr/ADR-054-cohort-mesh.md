---
Status: ratified-v2.2
Gate: Story 12.1 — `check-cohort-mesh`, disposition `{ v1_0 = advisory, v1_5 = advisory, v2_0 = advisory, v2_2 = blocking }` (first v2.2 gate — extends `gate-registry.toml` PHASE_ORDER with `v2_2`); live at N=8, anti-canned per §A7
Decided: 2026-07-09
Accepted-in-PR: Story-12.1
Amends: ADR-003 (revisit clause — "three or more Hosts … a different architecture" — by construction)
Repeats: ADR-020 (migrator mechanism), ADR-036 (swap-plan precondition surface)
Reuses: ADR-049 (independent-verifier discipline; derive verify-key from claimed identity), ADR-012 (typed-intent consent, confused-deputy)
Covers: J3 Marcus Team Nexus §10.7.1 (no new PRD FRs — composes FR21–FR26 / FR52–FR54 under one signed manifest)
---

# ADR-054 — Cohort mesh via signed static manifest

**Decision.** The N-host topology (J3's 8-host team, Reza's cross-team links) is a **full pairwise mesh of the existing bilateral A2A channels, declared by a static, Ed25519-signed cohort manifest.** No peer discovery, no DHT, no gateway node. A mesh link *is* a §7.2 bilateral channel — wire format, mTLS+TOFU semantics, and logical-clock discipline are unchanged. Membership and trust changes are **manifest re-issues, never runtime negotiation.**

> **Numbering (ratification hygiene, 2026-07-09).** This ADR was drafted as ADR-052 in architecture §15.2, but `docs/adr/` already carries ADR-052 (FKCS, Story 11.5) and ADR-053 (third-party trial attestation, Story 11.7). The v2.2 ADRs therefore shifted **+2**: cohort mesh 052→**054**, multi-tenant Loom 053→055, FR37 vetting 054→056, constitutional ceiling 055→057. All `ADV-052-N` sub-decision tags in §15.2 are renumbered `ADV-054-N` here; §15.8 and §15.11 are authoritative.

## Context

ADR-003 fixed cross-Host IAC at exactly two pre-paired Hosts (mTLS+TOFU, per-frame typed-intent consent), and declared a revisit clause: *"a use case emerges that requires three or more Hosts coordinating in real-time. (At that point this is a different architecture, not an extension.)"* J3 (Marcus's 8-host Team Nexus) and Reza (single-org cross-team Cortex) are exactly that use case.

The revisit clause is satisfied **by construction, not by a new protocol**: a mesh is the bilateral primitive (ADR-003) composed under one signed roster. Epic 11 shipped every substrate — the 25/30-host churn envelope (11.3), cross-region convergent replication with the independent-verifier discipline (11.2a/b, ADR-049), live cross-host A2A TCP/mTLS + typed-intent consent (Epic 8, ADR-012). ADR-054 declares how they compose into a governed team, with a single trust-and-membership artifact.

The alternatives — a gateway node, or peer-DHT discovery — were rejected: a gateway becomes a de-facto fifth protocol and a single point of trust/failure; DHT discovery destroys the per-pair TOFU semantics ADR-003 relies on; both introduce divergent per-deployment trust models. A static signed manifest keeps trust operator-declared and auditable.

## Decision

### 1. Signed cohort manifest (schema v1)

A cohort = a **versioned, Ed25519-signed TOML manifest** listing:
- **members** — `host_id`, pinned leaf-cert fingerprint (the §7.2 TOFU pin), declared roles;
- the **per-(peer,role) consent matrix** as **separate send/accept tables** (no transposition ambiguity);
- the two schema-mandatory **reserved always-allowlisted intent subtypes** (§4);
- a **strictly-monotonic integer `version`**;
- (for Reza) the team↔region↔datname mapping — that belongs to ADR-055/§15.3, not here.

The signature is over a **canonical, domain-separated, length-prefixed** byte pre-image (`SIG_DOMAIN = b"maos.cohort-manifest.v1"`, u32-BE length prefix on every field, fixed field order — the `canonical_kv_leaf` idiom; the length prefix is load-bearing against boundary-shift collisions). Parse, schema-validate, and signature-verify live **out of kernel** (new `maos-cohort` crate → `maos-a2a-core` + `maos-domain`, never `maos-kernel-core`).

### 2. Manifest authority + re-issue (ADV-054-1)

The manifest declares the genesis cohort-authority `{ keys, threshold }` **inside manifest v1 at cohort genesis**; only that authority signs re-issues; the version is a strictly-monotonic integer allocated by the authority.

**Genesis trust is operator-pinned out-of-band** (Story-12.1 preflight, 2026-07-10): each member holds the authority pubkey in its deployment config (the §7.2 cert-pin posture) and refuses any manifest whose declared authority ∉ the pinned key **set** → `ECohortAuthorityUnpinned`. A v1 cannot bootstrap its own trust root — so the verify-key is the **pinned genesis key for every version**, never a key carried in the manifest body and never re-derived from the re-issue's own declaration (the ADR-049 rule — "else forgery is a one-liner"). This is a **cryptographic** signature verification, not a presence check (a present-but-unverified manifest is the 11.2b D1 forged-stamp residual). **The pin is set-valued** (Round-2, RR5): `{current}` in steady state, `{current, next}` during an operator-declared rotation window — a strict *single-key* equality would refuse the legitimately-rotated key and brick the rotation clause below. **Note (RR3):** "verify" here means the signature **verifies under a held/pinned public key** — Ed25519 has no key-recovery (`ecrecover`), so a signer's identity is *never* "derived from the signature"; there is no existing Ed25519-pubkey pin field (`cert_fingerprint` is a TLS-cert hash), so Story 12.1 adds a new out-of-kernel cohort-level pin surface (RR4, still zero kernel delta).

A member **MUST refuse** a non-authority signature, a version regression, **or** a concurrent valid fork → typed **`ECohortManifestFork`** carrying a `reason` discriminant `{ NonAuthoritySigner, VersionRegression, ConcurrentFork }` and naming both versions. **Honest fork guarantee (Round-2, RR7):** this is a no-consensus CRDT mesh (no Raft, no total order) — `ConcurrentFork` is detectable **only at a node that receives both conflicting v(n+1)s**. If two forks land on disjoint members, each sees one valid monotonic version and neither raises; the mesh splits *silently*. So conflicting-same-version bytes are **caught locally where both are seen**, and cross-node divergence is **bounded, not prevented** — by the single-authority-writer discipline plus `T_stale` re-convergence (§3). This is a deliberate consequence of rejecting a gateway/consensus layer, not a defect; it is stated so no operator reads "refuses concurrent fork" as mesh-wide fork-*prevention*. **k-of-n multisig** is forward-declared but **v1 rejects `threshold > 1` at parse** → `EUnsupportedAuthorityScheme`; single cohort-authority key is the genesis default (§15.11 Fork 2), real m-of-n verification a clean follow-up with its own proven-red — never accept-and-single-verify.

Re-issue is journaled to the authority's TL; **every member journals its own acceptance of v(n+1)** to its local TL — per-member adoption is observable, not assumed. The authority key's custody follows the 9.4b signing-key runbook (§15.7); its rotation reuses the §7.2.1.a one-generation-overlap idiom.

### 3. Distribution + staleness ceiling (ADV-054-2), fail-closed

Manifest v(n+1) propagates as the reserved always-allowlisted `cohort:manifest-reissue` subtype (the §7.1 `retract` capacity-bypass idiom), **pushed** by the authority, **pull-on-connect** as fallback. A host that cannot confirm it holds the current version within **`T_stale` (default = §7.2's 30s partition-NACK window × 4 = 120s; a signed per-cohort manifest field, **parse-clamped to a code-constant `[T_STALE_MIN, T_STALE_MAX]`** — Round-2 RR6, so the authority tunes *within* code-owned bounds but cannot sign away its own fail-closed staleness; out-of-range → `ECohortStaleBoundViolation`)** marks its cohort links **degraded** and **refuses new consent-sensitive frames under the stale matrix** — fail-closed, consistent with the Story-8.8 posture. A member revoked in v(n+1) is refused mesh-wide within `T_stale`. Verify-at-load + cache-by-version + refuse-when-stale routes through a **single grep-provable chokepoint** (the 11.2b `region_guard` chokepoint discipline, composed with cryptographic verify).

### 4. Reserved cohort-infrastructure intent subtypes

`{ cohort:manifest-reissue, cohort:halt-receipt }` — always-allowlisted by schema requirement; everything else remains fail-closed per Story 8.8.

> **Grammar correction (verified against `crates/maos-domain/src/invariants/i8.rs:85`).** The architecture draft wrote these with dots (`cohort.manifest.reissue`). The canonical A2A intent grammar is `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$` — **dots are invalid**, so the dotted form can never match an allowlist (the router would `warn!` it unreachable). The binding form is **colon-kebab**: class `cohort`, subtype `manifest-reissue` / `halt-receipt`. Reserving these requires **no** grammar change → zero `maos-domain` delta.

### 5. Per-(peer,role) consent tuples (ADV-054-3) — declared here, evaluated in 12.2

The role in a tuple is the **counterparty's manifest-declared role as seen from the evaluating seam**: the sender checks `(receiver_peer, receiver_role)` against its send-allowlist; the receiver checks `(sender_peer, sender_role)` against its accept-allowlist; send and accept tables are separate in the schema. Multi-role members: the frame's consent envelope carries the **single acting role; match is exact, never any-role OR** (ADR-012's confused-deputy rationale, extended). Version skew: frames carry the sender's manifest version; a receiver on a newer version evaluates under its own (fail-closed wins); mismatch beyond ±1 → typed `ECohortManifestSkew`, distinct from `EIntentDenied`. In-flight frames at a role change drain under the admitted version; new admissions under v(n+1) only. Role queries are answered **from the manifest** — a read of signed versioned state, not a discovery protocol. *(Story 12.1 declares + parses + validates this matrix; Story 12.2 implements the two-seam evaluation and `ECohortManifestSkew`.)*

### 6. Cohort hot-swap + migration chains (ADV-054-4) — Story 12.5

Per-member `drain → swap → re-pin` (I14, NFR-Rel-6, 11.3 re-pin playbook). Linear-chain constraint: the migrator set per Spirit MUST form a linear chain — a second outgoing migrator for one source version is a **manifest-validation error**, not a runtime choice; the kernel chains hop-by-hop, refusing `EMigratorMissing` (names the missing hop). `maosctl swap --plan` hashes the resolved chain; the kernel refuses a chain whose hash differs from the plan's (`EMigrationPlanDrift`) — extends ADR-036, repeats the ADR-020 migrator mechanism, near-zero kernel delta. *(Story 12.5, not 12.1.)*

### 7. Cross-agent halt-on-conflict (ADV-054-5) — Story 12.3

Halts stay local (single-halt-owner unchanged). The cohort surface is **receipt-presence observability**: halt receipts are journaled locally (I2) and shipped as the reserved `cohort:halt-receipt` subtype, consumed by the digest Spirit. For each member: a receipt frame **or an explicit transport-level absence marker** (NACK/timeout per §7.2's 30s) within T — absence is a first-class observable (11.2b's point). Receipt-presence is *observability, not arbitration* — arbitration is the Director's, never the kernel's. *(Story 12.3; the `cohort:halt-receipt` class is reserved in 12.1, consumed in 12.3.)*

### 8. No-surveillance posture (J3 journey acceptance) — Story 12.4

The digest Spirit reads only consented topics under its own per-(peer,role) tuples; every cross-member read is consent-checked and journaled; the J3 acceptance corpus includes a surveillance-negative control (an out-of-matrix digest query refused **and visible to the affected member**). *(Story 12.4 — the J3 day-30 scene closer.)*

## Consequences

- New out-of-kernel crate `maos-cohort` (workspace 53 → 54); it must stay outside the `maos-kernel-core` + `maos-domain` dependency closure (`check-dependency-closure`). **Kernel baseline stays unchanged at `src_lines = 23081`** (ZERO-Δ; a FLAG-Winston re-pin only if preflight proves a seam genuinely required).
- New `xtask` gate `check-cohort-mesh` (first v2.2 gate — extends `gate-registry.toml` PHASE_ORDER with `v2_2`).
- ADR-003's revisit clause is discharged: recorded here as an explicit amendment-by-construction.
- The consent-composition matrix (per-(peer,role) × version-skew × in-flight-drain) is the named first candidate for the §15.6 formal-methods evaluation (§15.11 Fork 8).

## Alternatives considered and rejected

- **Gateway-mediated coordination.** Rejected: a gateway becomes a de-facto fifth protocol and a single trust/failure point; membership becomes runtime state instead of a signed artifact.
- **Peer-DHT discovery.** Rejected: destroys per-pair TOFU semantics; introduces divergent per-deployment trust; membership is no longer operator-declared/auditable.
- **Presence-only manifest check** (accept a manifest that is merely present/well-formed). Rejected: this is the 11.2b D1 forged-stamp residual — a planted unsigned/forged manifest would stay green. The verify is cryptographic, against a key derived from claimed identity.
- **k-of-n multisig as the genesis default.** Deferred (not rejected): single cohort-authority key is the genesis default; k-of-n remains an explicit manifest-declared option with identical mechanism (§15.11 Fork 2).

## Gate

`check-cohort-mesh` — manifest round-trip **with cross-issuer verification** (artifact produced by one code path, verified by an independently-derived verifier — ADR-049 independence); concurrent-re-issue `ECohortManifestFork` proven-red on a REAL fork; stale-member leg (revoked member refused mesh-wide within `T_stale`); per-(peer,role) consent corpus (role-mismatch-on-allowed-peer, acting-role exact-match, `ECohortManifestSkew`) — 12.2; linear-chain validation error + `EMigrationPlanDrift` proven-red — 12.5; receipt-presence per member under one induced member loss plus one induced connectivity loss — 12.3; surveillance-negative control — 12.4; `kernel-abi-diff`. Live at N=8 (**28 bilateral channels / 56 directed handshakes = N·(N−1)** — Round-2 RR1; the earlier "64" was N² incl. self-dials and is wrong); the **manifest-authority-identity reflex** — the accepted manifest's signature **verifies under the operator-pinned genesis key set** (Ed25519 has no key-recovery — the signer is *not* "derived from the signature"; RR3), reconciled against the pin, not a label. Counts derive-and-reconcile `N·(N−1)` from N (never a literal); the hermetic + N=8 legs hard-block **by construction** independent of the phase ladder (RR8 — `CURRENT_PHASE` stays where the project is; do not globally advance it); an attempted leg that ran 0 tests hard-fails; absent/unmeasured → BLOCK at the v2.2 ship gate. A cohort is green only if a machine derived the roster's validity and a planted lie (forged authority signature, stale-matrix frame, version regression) would have turned it red.
