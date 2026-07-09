# 15. Full-Spectrum v2.2 Architecture (J3 Team Nexus + Reza Cortex)

**Status:** `proposed-v2.2` — drafted 2026-07-06 (Step 2 of the full-PRD planning plan, operator-directed functionality-first); **pending party-mode ratification.** Fork choices carry `[ASSUMPTION]` tags; the ratification agenda is §15.10. Decision trail: `_bmad-output/planning-artifacts/architecture/architecture-maos-2026-07-06/.memlog.md`. **Reviewer gate applied 2026-07-06** (reality-check · adversarial · rubric/reconcile — 3× PASS-WITH-FIXES; all 14 adversarial rule-tightenings folded in below; full reviews in `…/architecture-maos-2026-07-06/reviews/`).

This section extends the minimal architecture to the two remaining PRD journeys — **J3 Marcus Team Nexus** (§10.7.1, committed at v2.2) and **Reza single-org cross-team Cortex** (§10.7.2, committed at v2.2) — plus the FR37 vetting machinery and the post-v2.0 constitutional ceiling. Everything here is **additive**: the §0.6 foundational commitments and the §3.2 invariants I1–I14 are unchanged, exactly as §10.7.3 promised. Working posture inherited from Epics 9–11: **zero-kernel-Δ by default**; any kernel-core delta is a FLAG-Winston pinned re-pin, never a side effect.

## 15.1 What v2.2 stands on

**Shipped by Epic 11 (sprint-status `done`, gates GREEN at HEAD):**

| Substrate | Shipped by | v2.2 consumes it for |
|---|---|---|
| WASM component-model Spirit form + cross-form equivalence gate (ADR-031 binding-v2.0) | 11.1a / 11.1b | Reza cross-team third-party Spirits, capability-isolated by construction |
| Cross-region convergent replication, `canonical_kv_leaf`, per-region Merkle oracle, `region_guard` (ADR-049) | 11.2a / 11.2b | multi-tenant Loom (§15.3) reuses the guard-chokepoint + physical-absence pattern |
| 25/30-host churn envelope, two-surface detection, re-pin playbook | 11.3 | J3 8-host mesh (§15.2) runs far inside the proven envelope; 100-host closer scales this substrate |
| Enterprise PDP out-of-kernel policy port, fail-closed (ADR-050) | 11.4a | Reza cross-team policy decisions |
| Sandbox-escape structural detector, out-of-kernel (ADR-024 binding-v2.0, bound 2026-07-06) | 11.4b | Cortex operational posture |

**In flight (NOT yet built — v2.2 designs that lean on these carry the dependency explicitly):** 11.4c enterprise identity/at-rest/SIEM (`ready-for-dev`, preflight resolved 2026-07-06; **ADR-051 is a reservation, not a landed ADR**) — §15.3 org-KMS composition and the §15.6 Enterprise reference Spirit depend on it. 11.5 FKCS infrastructure and 11.7 trial infrastructure (`backlog`, sequence last). Epic 11 lives on branch `epic-11`, un-merged; the shippable line still holds on the two external v1.5 items (pen-test, export counsel).

## 15.2 J3 cohort mesh — proposed ADR-052

**Decision.** The N-host topology (J3's 8-host team, Reza's cross-team links) is a **full pairwise mesh of the existing bilateral A2A channels, declared by a static, Ed25519-signed cohort manifest.** No peer discovery, no DHT, no gateway node.

- **Binds:** a cohort = a versioned signed TOML manifest listing members (`host_id`, pinned cert fingerprint, declared roles), the per-(peer,role) consent matrix, and (for Reza) the team↔region↔datname mapping (§15.3). A mesh link is exactly a §7.2 bilateral channel; wire format, mTLS+TOFU semantics, and logical-clock discipline unchanged (cashes the App-D.1 claim). **This ADR amends ADR-003's revisit clause** ("three or more Hosts … a different architecture") by construction: the mesh IS the bilateral primitive composed under one manifest — ratification of ADR-052 records that amendment explicitly.
- **Prevents:** divergent discovery/trust models per deployment; a gateway becoming a de-facto fifth protocol; DHT discovery destroying per-pair TOFU semantics.
- **Rule:** membership changes are manifest re-issues, never runtime negotiation. `[ASSUMPTION — topology fork for party-mode: full-pairwise-manifest over gateway-mediated and peer-DHT. N=8 → 28 pairs; 11.3 proves N=30 under churn.]`

**Manifest authority (ADV-052-1).** The manifest declares a **single cohort-authority key** (or explicit k-of-n multisig set) **inside manifest v1 at cohort genesis**; only that authority signs re-issues; the version is a strictly monotonic integer allocated by the authority. A member Host MUST refuse a manifest not signed by the genesis-declared authority and MUST refuse version regressions — typed `ECohortManifestFork`, naming both versions. The re-issue is journaled to the authority's TL; every member journals its own **acceptance** of v(n+1) to its local TL (per-member adoption is observable).

**Distribution and staleness (ADV-052-2).** Manifest v(n+1) propagates as a **reserved, always-allowlisted intent class** `cohort.manifest.reissue` (schema-mandatory in every manifest — the §7.1 `retract` capacity-bypass idiom), pushed by the authority, pull-on-connect as fallback. **Staleness ceiling:** a host that cannot confirm it holds the current version within T_stale (default: §7.2's 30s partition-NACK window × 4) marks its cohort links degraded and refuses new consent-sensitive frames under the stale matrix — fail-closed, consistent with the Story-8.8 posture. A member revoked in v(n+1) is refused mesh-wide within T_stale.

**Reserved cohort-infrastructure intent classes (specified once, used twice):** `{cohort.manifest.reissue, cohort.halt.receipt}` — always-allowlisted by schema requirement; everything else remains fail-closed per Story 8.8.

**Per-(peer, role) consent tuples (ADV-052-3).** Semantics committed in the ADR text: **the role in a tuple is the counterparty's manifest-declared role as seen from the evaluating seam** — the sender checks `(receiver_peer, receiver_role)` against its send-allowlist; the receiver checks `(sender_peer, sender_role)` against its accept-allowlist; send and accept tables are separate in the manifest schema (no transposition ambiguity — §7.2's two-seam model unchanged). Multi-role members: **the frame's consent envelope carries the single acting role; match is exact, never any-role OR** (ADR-012's confused-deputy rationale, extended). Version skew: frames carry the sender's manifest version; a receiver on a newer version evaluates under its own (fail-closed wins); mismatch beyond ±1 → typed `ECohortManifestSkew`, distinct from `EIntentDenied`. In-flight frames at a role change drain under the version they were admitted under; new admissions under v(n+1) only. Role queries are answered **from the manifest** — a read of signed, versioned cohort state, not a discovery protocol.

**Cohort hot-swap choreography (absorbs App-D.5).** Per-member `drain → swap → re-pin`, honoring I14 and NFR-Rel-6 pin invalidation, using the 11.3 re-pin playbook. **Migration chains committed at v2.2 with a linear-chain constraint (ADV-052-4):** the migrator set per Spirit MUST form a linear chain — registering a second outgoing migrator for the same source version is a **manifest-validation error**, not a runtime choice. The kernel chains hop-by-hop, refusing with `EMigratorMissing` naming the specific missing hop. `maosctl swap --plan` **hashes the resolved chain**; the kernel refuses to execute a chain whose hash differs from the plan's (`EMigrationPlanDrift`) — extends ADR-036. Mechanism is repeated ADR-020; near-zero kernel delta. `[ASSUMPTION]`

**Cross-agent halt-on-conflict (ADV-052-5).** Halts stay local (single-halt-owner unchanged). The cohort surface is **receipt-presence observability**: halt receipts are journaled locally (I2) **and shipped as the reserved `cohort.halt.receipt` intent class**, consumed by the digest Spirit. The observability assertion: for each member, either a receipt frame **or an explicit transport-level absence marker** (NACK/timeout per §7.2's 30s) within T — absence is a first-class observable, 11.2b's whole point. Receipt-presence frames are *observability, not arbitration* — arbitration is the Director's, never the kernel's.

**No-surveillance posture (J3 journey acceptance).** The digest Spirit reads only consented topics under its own per-(peer,role) tuples; every cross-member read is consent-checked and journaled; the J3 acceptance corpus includes a surveillance-negative control (a digest query outside the consent matrix is refused and visible to the affected member).

**Gate (proposed): `check-cohort-mesh`** — manifest round-trip **with cross-issuer verification** (artifact produced by one code path, verified by an independently-derived verifier — the ADR-049 independence discipline); concurrent-re-issue negative control (`ECohortManifestFork` proven); stale-member leg (revoked member refused mesh-wide within T_stale); per-(peer,role) consent corpus incl. role-mismatch-on-allowed-peer, acting-role exact-match, and skew (`ECohortManifestSkew`) negatives; linear-chain validation error + `EMigrationPlanDrift` proven-red; receipt-presence per member under one induced member loss **plus one induced connectivity loss** (absence marker observed); surveillance-negative control. Live at N=8; anti-canned per §A7.

## 15.3 Multi-tenant Loom — proposed ADR-053

**Decision.** Per-team residency in the Reza Cortex is **database-per-team** (distinct `datname` per team on operator-assigned Postgres instances) with a **store-internal `team_guard` chokepoint** below `CollectiveMemoryPort`, and **per-team Merkle convergence roots**.

- **Binds:** tenant isolation is physical (a team's rows live in a database the other team's connection cannot name), enforced at the same store-internal layer as 11.2b's `region_guard`; team→region placement composes with 11.2a residency (a team's database lives in its region; cross-team reads cross the guard, never the wall).
- **Prevents:** tenant leakage being one `WHERE` clause away; two teams' convergence proofs entangling; hidden multi-tenancy (NFR-Tenancy-1's boundary stays loud).
- **Rule:** cross-team sharing is an explicit, consented, re-attested write into the other team's database, never a shared table. `[ASSUMPTION — shape fork for party-mode: database-per-team over namespace-per-team row filtering; the 11.2b ratified precedent (distinct-`datname` + physical-absence controls) is dispositive.]`

**Cross-team attestation keys — NAMED FORK, party-mode must choose (ADV-053-1, BLOCKER-class).** ADR-049's verification chain is keyed **per-region** (`derive_region_signing_seed(base_seed, region)`); Reza's teams will normally share a region, where a region-keyed bundle is forgeable by any same-region team — the crypto boundary would be vacuous exactly where the tenant wall matters. Two closures, both sound; **the ADR must ship one, explicitly:**

1. **Per-team key weld (recommended):** a second HKDF stage over the region seed with a frozen team-tag grammar (mirrors the 9.4b derivation exactly; new versioned `TEAM_INFO_PREFIX`); `verify_bundle` for cross-team bundles derives the pubkey from `(claimed_region, claimed_team)`, never from bundle contents. `source_team` becomes a persisted crossing column and **enters the leaf pre-image via `canonical_kv_leaf` v2** (versioned domain tag; 11.2a v1 leaves untouched — byte-compat by construction, the 9.2b idiom).
2. **Honest downgrade:** intra-region cross-team isolation is guard + consent + physical `datname` separation with **no cryptographic team boundary**; "re-attested" applies only when a region boundary is crossed; `loom-threat-model.md` carries same-region insider-team forgery as an accepted risk, in writing.

`[ASSUMPTION: I recommend fork 1 — the tenant wall is an adversarial boundary, and "guard + operator honesty" is not the discipline this project ships.]`

**Mapping has one owner (ADV-053-2).** The team↔region↔datname mapping is a section of the **signed cohort/org manifest (ADR-052's artifact)** — versioned, signed, journaled. `team_guard` loads it only from the manifest, verifies the signature at load, caches by version, and **refuses reads/writes when its cached version trails the announced current version** (`ETenantMapStale`, fail-closed). Store-local config may hold connection credentials, never team membership or placement.

**Team membership is identity-keyed and single-valued (ADV-053-3).** A Spirit belongs to exactly one team, declared in the signed manifest; `team_guard` verifies `(spirit_pid → team)` against the manifest **and** that the connection in use is the one assigned to that team (identity authoritative; mismatch is typed `ETenantConnectionMismatch`, never a silent allow). A Spirit needing both teams' data is **two Spirits with an ADR-012-consented channel** — that is the substrate's shape. **Row ownership:** a re-attested copy in team B's database is **team B's row** for Merkle, capacity, and GDPR-erasure purposes, with `source_team` provenance for forensics and the forget-cascade (the 9.2 erasure spine must know whom to cascade to).

**Multi-hop distillation provenance across the wall (rubric HIGH — the Reza scene's "14 prior schema decisions cited in one consolidated proposal").** A cross-team distillate carries its **flattened I11 chain** (`source_log_ref` flattened-to-raw + `distillation_depth` + `intent_lineage`, per ADR-014/018) **inside the re-attested crossing bundle** — the provenance copy lands with the row, so ordinary traceback dereferences within the consumer team's own database. Raw traceback **across** the wall (dereferencing another team's TL) is an ADR-012-consented `log.recall` to the source team, journaled on both sides; refusal is a first-class, surfaced outcome (provenance-presence, not provenance-promise — the ADR-049 §7 orphan discipline reused).

**Guard-semantics caveat carried forward (reality-check F5).** The shipped `region_guard` enforces provenance-**presence**, not cryptographic validity (11.2b Decision D1 Refined-A; forged-stamp residual has a named v2.x successor). `team_guard` in a *tenant* wall is a stronger adversarial setting: under fork 1 above, the guard's check upgrades to signature verification against the derived team key; under fork 2, the D1 presence-semantics caveat MUST be restated in this ADR and the threat model.

**Gate (proposed): `check-multi-tenant-loom`** — physical-absence control (team-B rows unreachable from team-A connection, distinct-`datname` witness); guard proven-red on a live read (foreign-team row without valid re-attestation refused, never served) **with the verifier independently derived from the write codec**; per-team Merkle independence (mutating B's store does not move A's root); stale-map leg (`ETenantMapStale` proven within the staleness ceiling); dual-connection negative control (foreign-team read via a second connection refused + audited, `ETenantConnectionMismatch`); cross-team distillate provenance round-trip (flattened chain lands with the row; consented raw traceback works; unconsented refusal surfaced); NFR-Scale-5 capacity envelope derived from measured per-instance load.

## 15.4 FR37 vetting machinery — proposed ADR-054

**Decision.** The vetting flow is **out-of-kernel** (registry + `maos-compliance`). A `VettingAttestation` is an Ed25519-signed envelope binding (manifest hash — **exact-hash, by design**, from-tier, to-tier, vetter key id, expiry, `revocation_semantics`, optional `successor_policy`); issuance, verification, revocation, **and vetter-key lifecycle events** are journaled to the TL; revocation rides the CRL/yank path, FR59-distinguishable.

- **Binds:** `public-vetted` is the fourth trust tier (the §10.7.2(d) slot); promotion is an attestation artifact, never a registry flag; kernel admission is unchanged (the strictest-of floor already reads the tier).
- **Prevents:** tier promotion without a signed, journaled, revocable artifact; kernel growth for registry policy (I9/ADR-006); unvetted code running at a vetted tier under a stale attestation.
- **Rule:** v2.2 exercises the full flow with **internal vetter keys**; accredited external vetters (NFR-Comp-2) remain v2.5, never gating engineering. **App-D.4 stays deferred** — `public-vetted` promotes within the public axis; the partner-org federation slot remains structure-reserved (trigger: first partner-org deployment request).

**Upgrade semantics (ADV-054-1).** Exact-hash binding means **upgrade-without-current-attestation = admission refusal at the floor, by design** — the flap is the feature, not a bug to soften into family-binding (which would run code the vetter never saw at `public-vetted`). The vetter's `successor_policy` (`exact-only` | `re-issue-required-with-expedited-review`) makes the re-vetting cadence explicit. **Hot-swap interaction:** the target version's attestation is evaluated **before the chain starts** — folded into `maosctl swap --plan`'s precondition (ADR-036); intermediate migration-hop states are execution steps of one admission decision, never separately-admitted artifacts.

**Expiry/revocation vs running Spirits (ADV-054-2).** `revocation_semantics` is committed in the envelope: **v2.2 ships `refuse-at-next-load` only** (honest about zero-kernel-Δ — there is no out-of-kernel lever that demotes a running Spirit's tier), **plus** a mandatory **journaled observation event** the moment the compliance layer detects expiry/revocation while an affected Spirit runs ("Spirit X running at tier T with lapsed attestation A since ts") — surfaced, never laundered (ADR-049 §7 orphan discipline). `drain-and-refuse` (runtime action via the existing drain machinery, not a kernel hook) is the named v2.5 upgrade slot. Audit output distinguishes **four** terminal causes: vetting-revocation / expiry-lapse / registry-yank / operator-local.

**Vetter-key lifecycle (ADV-054-3).** Enrollment, rotation, and revocation of a vetter key are first-class Ed25519-signed events, signed by the **operator audit key** (the §7.3 sealed-export root), journaled to the TL. `verify` walks attestation → vetter-key enrollment → operator root, refusing attestations whose vetter key lacks a journaled enrollment predating issuance.

**Gate (proposed): `check-vetting-attestation`** — issue→install→promote→revoke round-trip on a clean host **with independently-derived verifier**; forged-signature, expired-attestation, **and forged-vetter-key (unenrolled key, valid signature)** negative controls; upgrade-flap control (new version without attestation refused at floor); running-Spirit lapse produces the journaled observation event; four-cause distinguishability in audit output.

## 15.5 Post-v2.0 constitutional ceiling — proposed ADR-055 (constitutional; ADR-037 gate applies)

**Reality, stated in both instruments (reality-check F2 — the two regimes are never compared to each other):**

- `xtask/kernel-core-baseline.toml` (`check-kernel-baseline`): **23,081 raw `src/` lines** — includes in-src test code and doc comments by its own HISTORY notes. This is the **drift tripwire**: zero-Δ by default, every delta a FLAG-Winston HISTORY entry. It held 15505→23081 across three epics with every line accounted.
- `xtask/kloc.toml` (tokei, production Rust only, tests/benches excluded — **the regime NFR-Maint-1's "excluding tests" letter names**): `maos-kernel-core` measured **17,687** at the Epic-10 retro — **under** the 20 KLOC letter, with ~2.3K headroom that the unfinished Phase-3/4 decomposition debt makes unearned.

So NFR-Maint-1 is **not** "already false" — the earlier draft's claim rested on a metric mismatch and is withdrawn. The honest ratification argument: the number was never the discipline; the **pin protocol** was. ADR-055 ratifies the instrument that actually held and gives the post-v2.0 era a measured, gate-backed ceiling.

**Decision.**

1. **The pinned-baseline + FLAG-Winston re-pin protocol is the primary constitutional instrument.** `check-kernel-baseline` (raw src lines, tripwire) and the kloc regime (production LOC, ceilings) are both retained, **each cited only in its own units, never compared**.
2. **Per-crate ceiling discipline, as actually lived (ADV-055-2):** ceilings move only **(a) downward at any time, or (b) at epic retro, to the tight measured residual (+≤1% slack)**, with the measured value and driver recorded in the same commit — never to round headroom, never mid-epic, never for planned growth. New crates minted by extraction get initial ceiling = measured LOC at extraction +≤1%, recorded in the extraction commit. (The Epic-10 retro's 17,000→17,750-style re-pins are this discipline working, not a violation of it; the original 6,000 decomposition **target** was never amended and stands.)
3. **ADR-041 decomposition continues inside the v2.2 wave.** Honest arithmetic (reality-check F4): the kloc.toml plan's remaining Phase-3/4 extractions (~6.5 KLOC) from the measured 17,687 leave a residual of **~11.2 KLOC — not the plan's stale ~5.4K estimate**. Reaching ≤6,000 requires additional extraction scope beyond the current plan. `[ASSUMPTION — party-mode chooses: (a) commit extra extraction scope to hit ≤6,000, or (b) re-target the residual to the measured-honest ~11K with the ceiling discipline of clause 2. I recommend (b): the 6,000 figure was Epic-5-era arithmetic, and clause 2's discipline is the actual control.]`
4. **Kernel-crate-set aggregate ceiling (ADV-055-1):** a **new** pinned membership file `xtask/kernel-crate-set.toml` (distinct from `kernel-crates.toml`, which is the check-loom scan list) enumerates the member crates at ratification — recommended set: residual `maos-kernel-core` + the ADR-041 extracts + `maos-iac` + `maos-manifest` + `maos-capability` (the post-decomposition trusted computing base, ≈22.5 KLOC today) — changes FLAG-Winston only. The ceiling: **≤25 KLOC, alarm at 23.5K, measured in kloc.toml units (production-only)**; this is a **new aggregate key + xtask leg**, declared as such (the existing `_aggregate_alarm=16000/_aggregate_hardfail=103000` cover the whole workspace and are unrelated). **Sequencing:** alarm live from ratification in **advisory** mode (§A7.5 WOULD-HAVE-BLOCKED banner idiom); ceiling **binds at v2.2-wave close**. `[ASSUMPTION — set membership + 25K/23.5K numbers for party-mode.]`
5. **Single-tenant guarantee expires on schedule at v2.0**; multi-tenancy arrives at v2.2 **outside the kernel** (§15.3) — the kernel's tenancy posture is unchanged **because the tenant wall is proven at the store gate**: clause 5's enforcement is `check-multi-tenant-loom` (ADV-055-3), not narrative.

NFR-Maint-1's text is amended at the next PRD delta touch: the excluding-tests letter is retained with the clause-4 aggregate as its post-v2.0 successor instrument (Step-3 hand-off item).

## 15.6 v2.0 remainder sweep + scale closers — dispositions

| Item | Disposition |
|---|---|
| **100-host churn (NFR-Scale-2/Rel-7 second half)** | v2.2; **scale-out of the Story-11.3 substrate to N=100** (same real-mesh, per-event derivation, two-surface detection); lands as a new leg/param of `check-scale-churn`, floors unchanged (detection ≤1h median, blast ≤5 peers, recovery ≤24h); no new design — the envelope grows, the falsifiers travel with it |
| **10-host mTLS rotation chaos (NFR-Sec-13 v2.0 half)** | v2.2; scale-out of the ratified `rotation.rs` floors (10.4b/10.5 3-host drill) to 10 hosts; zero conversation drops under rotation-during-load |
| **30-day soak + geo-SLO (NFR-Scale-1/1-SLO(b))** | Release-gate pilot artifacts tracked at v2.2-wave close — per the 11.2b ratified split these are **never claimed as CI-validated**; absent/unmeasured → BLOCK at the v2.2 ship gate, exactly as at v2.0 |
| **NFR-Test-7 reconcile** | rust↔subprocess leg formally removed (5.5e decision + D.2 retirement); any-rust↔wasm ≥75% leg satisfied by 11.1b and carried forward as a regression gate |
| Sentinel canary auto-rollback | v2.2; composes ADR-020 rollback (≤30s auto-revert) with Loom pattern-scan pre-deploy; out-of-kernel orchestration |
| Native mobile push | v2.2; delivery-channel adapter behind §7.4; no kernel change (OQ-7 closes here) |
| Optional skill registry | v2.2; same registry protocol (ADR-008), separate namespace; skills user-space forever |
| Vault/cloud-KMS secret backends | v2.2; `maos-secrets` backends behind the existing provider trait; composes with 11.4c org-KMS (**dependency: 11.4c is ready-for-dev, not shipped**) |
| Distro packages + one-line installer | v2.2; release engineering, no architecture |
| Bedrock/Vertex/local full multi-provider | v2.2; ADR-005 driver additions |
| Enterprise reference Spirit **class** | v2.2; the 11th reference Spirit composing PDP (11.4a, shipped) + identity/at-rest/SIEM (11.4c, **in flight**); Spirit-side, zero kernel |
| Formal methods (TLA+/Alloy for I5/I6/I9) | **Disposition recorded: NOT landed.** Property-test + structural-lint coverage held through 11 epics with zero invariant violations; commit only if a v2.2 design surfaces a property the corpus cannot falsify — the ADR-052 consent-composition matrix (per-(peer,role) × version-skew × in-flight drain) is the named candidate to evaluate first. `[ASSUMPTION]` |
| `loom-threat-model.md` | v2.2, **before** multi-tenant Loom ships. Scope now explicitly includes (rubric finding): same-region insider-team forgery (per the §15.3 fork outcome), cohort-manifest authority-key compromise, malicious cohort member, and the Sec-14a/14b threat-model split extended to N-host cohort topology |

## 15.7 Operational envelope (rubric HIGH — previously silent)

- **Cohort manifest at rest:** the authority's copy is the source of truth (journaled per §15.2); every member persists its accepted copy beside its TL (SQLite/file, operator-backed-up with the §7.3 TL backup/DR discipline, RPO ≤1h per NFR-Ops-9). Authority-key custody follows the 9.4b signing-key runbook; authority-key rotation is a manifest re-issue signed by both old and new keys (one-generation overlap, the §7.2.1.a pre-staged-overlap idiom).
- **Per-team Postgres ownership (Reza):** each team's database is operator-provisioned per the 10.4a runbook (provisioning, backup, migration corpus); the org manifest names placement (§15.3); credentials live in `maos-secrets`, never in the manifest.
- **Deployment topologies:** §11 gains two shapes at v2.2 ratification — **11.4 J3 team mesh** (8 Hosts, cohort manifest, digest Spirit) and **11.5 Reza single-org Cortex** (3 teams × regions, per-team Postgres, PDP + identity stack). Runbooks land with the epics (Step-3), not this section.
- **Ops posture:** all new gates enter `gate-registry.toml` with the sibling disposition idiom `{v1_0 = advisory, v1_5 = advisory, v2_0 = advisory, v2_2 = blocking}`; absent-result → BLOCK at the v2.2 ship gate (Epic-11 idiom carried forward).

## 15.8 New ADR stubs (enter §12.0 and `docs/adr/` at ratification)

| ADR | Title | Status | Gate |
|---|---|---|---|
| 052 | Cohort mesh via signed static manifest over pairwise bilateral A2A (authority key, per-(peer,role) tuples, reserved intent classes, linear migration chains; amends ADR-003 revisit clause) | `proposed-v2.2` | `check-cohort-mesh` live at N=8 (legs per §15.2) |
| 053 | Multi-tenant Loom — database-per-team + `team_guard` + per-team Merkle roots + cross-team provenance (key-weld fork per §15.3) | `proposed-v2.2` | `check-multi-tenant-loom` (legs per §15.3) |
| 054 | FR37 vetting attestation machinery, out-of-kernel, internal-vetter-first, exact-hash + refuse-at-next-load | `proposed-v2.2` | `check-vetting-attestation` (legs per §15.4) |
| 055 | Post-v2.0 constitutional ceiling — pin protocol + retro-residual discipline + `kernel-crate-set.toml` ≤25 KLOC (kloc units) | `proposed-v2.2` (constitutional; `invariant-lock` + party-mode) | `check-kernel-baseline` (tripwire) + NEW kernel-crate-set aggregate leg (advisory → binding at wave close) |

Numbering: 051 is reserved by Story 11.4c. The live registry `docs/adr/` is authoritative for landed ADRs — §12.0.1.

## 15.9 What v2.2 explicitly does not decide

External-author FKCS population, external N=12 cohort, accredited vetters, cert bodies, consortium case study (v2.5, non-gating). App-D.4 federation tier (deferred, trigger named). Predicate stdlib (App-D.3 — deferred; fresh number when proposed). rust-inproc (App-D.2 — retired; §13.1 gate is the escape hatch). Tier-3 14-site/28-agent consortium proof (v3.0/thesis milestone — release-gate evidence, never stories).

## 15.10 Party-mode ratification agenda (the named forks)

1. **ADR-052 topology:** full-pairwise-manifest (recommended) vs gateway-mediated vs peer-DHT.
2. **ADR-052 authority:** single cohort-authority key vs k-of-n multisig (mechanism identical; choose the genesis default).
3. **ADR-053 shape:** database-per-team (recommended, precedent-dispositive) vs namespace-per-team.
4. **ADR-053 cross-team keys (BLOCKER-class, must choose):** per-team HKDF key weld + `canonical_kv_leaf` v2 (recommended) vs honest-downgrade (no intra-region crypto boundary, risk in writing).
5. **ADR-055 residual target:** re-target to measured-honest ~11K under the retro-residual discipline (recommended) vs commit extra extraction scope to hit the original 6,000.
6. **ADR-055 numbers + set:** kernel-crate-set membership (recommended: Unit-B trusted-computing-base set ≈22.5K today) and the 25K/23.5K figures, kloc units.
7. **D.5 chaining + linear-chain constraint** (recommended as written) — cheap to ratify with 052.
8. **Formal-methods trigger** (§15.6): accept the ADR-052 consent-composition matrix as the named evaluation candidate, or drop formal methods outright.
