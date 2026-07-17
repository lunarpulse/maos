# Epic 13 — Reza Single-Org Cross-Team Cortex (v2.2)

**Status:** `in-progress — plan-hardened 2026-07-17` — Story 13.1 is `ready-for-dev`: Lunarpulse ratified F4 Option A+ with dual-read v1/v2 compatibility, exact schema↔teams invariants, distinct signature domains, tenant-required-v2 boot, and downgrade refusal. The Epic-13 plan includes two evidence-discovered prerequisites before the Reza closer: collective-tier erasure/legal-hold and a real Spirit/daemon collective path with tenant audit isolation.

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = **GA ledger only, non-gating for v2.2 dev** (E11 retro A3).

**Model/review discipline (E11 retro A1):** frontier-class dev allowlist + **§A6 full-layer review net is the binding control**. The physical and cryptographic tenant walls (13.1/13.2), cross-team consent/provenance (13.3), vetting promotion (13.4), collective erasure/hold (13.5b), production tenant-audit path (13.5c), and Reza journey (13.6) are adversarial boundaries → the full §A6 net (incl. Test-Infra + runtime) is **non-degradable**.

---

## Objective
Serve **Reza's single-org cross-team Cortex** (§10.7.2, committed v2.2) — a 400-person fintech running multiple teams as one governed Cortex on shared MAOS infrastructure. Epic 11 shipped the **enablers** (WASM form, multi-region Loom, PDP, identity/at-rest/SIEM); the journey itself is **unbuilt**. Epic 13 delivers the **tenant wall** (multi-tenant Loom, database-per-team, cryptographic per-team boundary), **cross-team asymmetric sharing with multi-hop distillation provenance**, the **FR37 vetting machinery** (the only unserved PRD FR — internal-vetter-first), the **Enterprise reference Spirit class**, collective-tier erasure/legal-hold closure, a real Spirit/daemon collective path with tenant audit isolation, and the **Reza Cortex scene E2E** on the 3-region substrate. The closer composes proven controls and refuses to translate absent or indeterminate evidence into a product claim.

## What Epic 13 stands on
| Substrate | From | Epic 13 use |
|---|---|---|
| Cross-region convergent replication, `region_guard` chokepoint, `canonical_kv_leaf`, per-region Merkle oracle, `CrossRegionReadmit` re-attestation (ADR-049) | 11.2a/b | `team_guard` reuses the guard-chokepoint + physical-absence pattern; `canonical_kv_leaf` **v2** adds `source_team`; cross-team re-attestation reuses the readmit path |
| Signed cohort/org manifest, per-(peer,role) consent tuples, full-pairwise mesh (ADR-054) | Epic 12 | the org manifest **owns** the team↔region↔datname mapping; cross-team A2A rides cohort links; consent tuples extend to the team axis |
| Enterprise PDP out-of-kernel policy port, fail-closed (ADR-050) | 11.4a (done) | Enterprise reference Spirit (13.4) + Reza cross-team policy |
| SSO/OIDC identity + org-KMS at-rest AEAD + SIEM export (ADR-051) | 11.4c (done) | Enterprise reference Spirit + at-rest for per-team Loom rows |
| Region signing-key HKDF weld, sealed-export root (9.4b, §7.3) | Epic 9 | per-team HKDF key-weld mirrors the 9.4b derivation exactly; vetter-key lifecycle rooted at the operator audit key |
| Registry publish/install/trust-tiers, ComplianceClaim (Epic 7) | Epic 7 | FR37 `public-vetted` is the 4th trust tier; attestation rides the CRL/yank path |

## Ratified architecture basis
- **ADR-055 — Multi-tenant Loom** (§15.3): database-per-team (distinct `datname`) + store-internal `team_guard` + per-team Merkle roots. **Fork-4 ratified = per-team HKDF key-weld** (same-region cross-team forgery cryptographically closed; `team_guard` upgrades to signature-verify, not presence-only). Mapping owned by the signed org manifest. Row-ownership = destination team's row. Multi-hop distillation provenance in the re-attested bundle.
- **ADR-056 — FR37 vetting machinery** (§15.4): out-of-kernel `VettingAttestation`, exact-hash + refuse-at-next-load, internal-vetter-first, vetter-key lifecycle rooted at the operator audit key, four distinguishable terminal causes.

---

## Story list (decompose ACs at each story's preflight; ≤6 ACs)

| # | Title | Scope (one-line) | ACs | Model | Kernel-Δ risk | Depends |
|---|-------|------------------|-----|-------|---------------|---------|
| **13.1** | Multi-tenant Loom — **physical tenant wall** (ADR-055) | Database-per-team (distinct `datname`) + store-internal `team_guard` chokepoint; manifest-owned team↔region↔datname mapping (`ETenantMapStale`); identity-keyed single-team Spirits (`ETenantConnectionMismatch`); physical row-ownership; `check-multi-tenant-loom` physical legs. | 6 | frontier + **full §A6** (tenant wall) | **ZERO expected** (maos-loom-lite + store-internal guard, 11.2b precedent); FLAG-Winston bounded only if cross-team readmit needs a kernel variant beyond `CrossRegionReadmit` (verify) | 11.2a/b, Epic 12 (12.1) |
| **13.2** | Multi-tenant Loom — **cryptographic tenant boundary** (ADR-055, Fork-4) | Per-team HKDF key-weld + `canonical_kv_leaf` v2 (`source_team` in pre-image, v1 byte-compat); `team_guard` upgrades to signature-verify (closes 11.2b D1); per-team Merkle independence; forged-team-stamp negative; team-identity source-reflex. | 6 | frontier + **full §A6** (crypto boundary) | ZERO expected (HKDF + crypto in maos-loom-lite; verify) | 13.1 |
| **13.3** | Cross-team asymmetric consent + multi-hop distillation provenance | Cross-team re-attested writes over cohort links; asymmetric consent envelopes (team axis); flattened I11 chain in the crossing bundle; consented cross-wall `log.recall`; refusal first-class. | 6 | frontier + **full §A6** (cross-team consent) | ZERO expected (reuses 11.2a re-attestation; team dimension in maos-loom-lite; verify) | 13.2, Epic 12 (12.2) |
| **13.4** | FR37 vetting machinery (ADR-056) | `VettingAttestation` issue→install→promote→revoke, internal vetter keys; exact-hash + refuse-at-next-load; vetter-key lifecycle at the operator audit key; four-cause distinguishability; `check-vetting-attestation`. | 6 | frontier + **full §A6** (trust-tier) | ZERO (out-of-kernel registry + maos-compliance) | Epic 7 registry, §7.3 audit key |
| **13.5a** | Enterprise reference Spirit class | The 11th reference Spirit, Spirit-side: PDP (11.4a) + identity/at-rest/SIEM (11.4c) composed into a reusable enterprise-governed Spirit; proven wired at the daemon seam (not just constructed). | 6 | frontier + §A6 | ZERO expected (Spirit-side) | 11.4a, 11.4c (both done) |
| **13.5b** | Collective-tier erasure + legal-hold cascade | Extend the shipped Epic-9 forget/hold spine to collective rows and cross-team destination copies using 13.2 `source_team`; reconcile success, hold, and partial-failure outcomes. | 6 | frontier + **full §A6** (lifecycle boundary) | **FLAG-Winston expected** — current `CollectiveMemoryPort` has no erase operation and kernel `forget*` never reaches `MemoryTier::Collective`; do not preserve a ZERO claim by routing around the owner | 13.2, Epic 9 |
| **13.5c** | Reza production collective path + tenant audit isolation | Make mediated Spirit/daemon collective read/write real; bind per-team/per-operator TL isolation and cross-team correlation so 13.6 exercises production code, not a wall around an unused door. | 6 | frontier + **full §A6** (runtime/audit boundary) | **FLAG-Winston verify** — `SpiritMemoryView` exposes no collective path at the 13.1 preflight baseline; no ZERO assumption before preflight | 13.1, 13.3, 13.5a |
| **13.6** | Reza Cortex journey closer + NFR-Scale-5 envelope | 3 teams × 3 regions composing 13.1–13.5c; allowed collaboration, explainable refusal/recovery, minimum-disclosure provenance, collective erase/hold reconciliation, live production wiring, and the measured—not executed—14-institution envelope. | 6 | frontier + **full §A6** (journey) | No blanket ZERO claim; verify the final baseline after 13.5b/13.5c | 13.1–13.5c, 11.2b |

**Sequencing:** 13.1 (physical wall; F4 Option A+ ratified/dev-ready) → 13.2 (crypto boundary) → 13.3 (cross-team consent). 13.4 (FR37) and 13.5a (Enterprise Spirit) remain parallelizable; 13.5b closes collective lifecycle after `source_team`, and 13.5c makes the tenant wall reachable through the real Spirit/daemon path. **13.6 is last and only judges; it never invents a missing mechanism inside the journey harness.** Eight stories total.

**Demo-ability:** the smallest real wall is a **CI-provisioned Postgres service with two distinct `datname`s** (13.1·AC5). Loom-lite has no SQLite collective store; a SQLite twin would prove code that does not ship. The full rung is the **3-team × 3-region Reza scene** (13.6). Absence of its live-substrate evidence may leave development lanes advisory, but it blocks the Reza/v2.2 product claim.

---

## Per-story AC sketch (finalize at preflight)

**13.1 — Multi-tenant Loom — physical tenant wall** (ADR-055)
1. Database-per-team: distinct `datname` per team on operator-assigned Postgres; private store-internal `team_guard` below `CollectiveMemoryPort`, invoked once at each Spirit-facing `read`/`scan`/`write` entry point. The static chokepoint leg proves each site and its unguarded negative twin.
2. Mapping ownership: schema-v2 `[[teams]]` is part of the operator-signed cohort manifest; a `TenantMapPort` implementation lives in a leaf `maos-tenant-map` crate and is injected at the composition root. Staleness uses the shipped `t_stale_secs` time lease—there is no announced-version channel. **F4 schema-v2 canonical-signature compatibility must be ratified before development starts.**
3. Membership is signed by stable `SpiritId`, then registered to runtime-local `spirit_pid` at composition. `team_guard` compares the manifest team with `StoreConfig.home_team`; connection construction verifies manifest `datname == current_database()`. Refusal is typed at `StoreError` and consciously lossy at the frozen collective port.
4. Physical row ownership only: the owning team is the database containing the row, proven by an unguarded physical-absence witness. **13.1 does not claim `source_team`, collective forget, or forge resistance**; 13.2 adds source/key identity and 13.5b closes erasure/hold.
5. `check-multi-tenant-loom`: hermetic Blocking chokepoint/tenant-map legs plus a CI-provisioned real-Postgres two-`datname` physical leg. A forged team stamp is deliberately documented as served at 13.1; crypto/provenance/lifecycle/runtime/journey legs emit **ABSENT**, never disappear or silently green.
6. ZERO kernel-Δ @23202 for this story only; ADR-055 and `docs/loom-threat-model.md` are AC deliverables. Any pressure to change kernel-core stops for FLAG-Winston rather than being absorbed.

**13.2 — Multi-tenant Loom — cryptographic tenant boundary** (ADR-055, Fork-4)
1. Per-team HKDF key-weld (Fork-4): second HKDF stage over the region seed with a **frozen versioned `TEAM_INFO_PREFIX`** grammar (mirrors 9.4b exactly); `verify_bundle` for cross-team bundles derives the pubkey from `(claimed_region, claimed_team)`, **never from bundle contents**.
2. `team_guard` upgrades to **signature verification** (not presence-only) — closes the **11.2b D1 presence-only residual** for the tenant wall: a foreign-team row without a valid re-attested signature under the derived team key is **refused, never served**.
3. `canonical_kv_leaf` v2: `source_team` enters the leaf pre-image under a **versioned domain tag**; 11.2a v1 leaves untouched (byte-compat by construction, the 9.2b idiom).
4. Per-team Merkle roots **independently re-derived** (mutating team B's store does not move team A's root); the payload-oracle + row-count-oracle catch what the SET-root is blind to (11.2a L3).
5. `check-multi-tenant-loom` (crypto legs): guard proven-red on a **live read** (foreign-team row without valid re-attestation refused, verifier **independently derived from the write codec**); per-team Merkle independence; **forged-team-stamp negative** (a same-region team forging another team's bundle is rejected — the Fork-4 payoff); **team-identity source-reflex** (source-team identity derived from the derived team key, not a label).
6. ZERO kernel-Δ expected (HKDF + crypto in maos-loom-lite; verify).

**13.3 — Cross-team asymmetric consent + multi-hop distillation provenance**
1. Cross-team sharing = an explicit, consented, **re-attested write into the other team's database**, never a shared table; the re-attested copy is the destination team's row (Merkle/capacity/GDPR-erasure) with `source_team` provenance for the forget-cascade (9.2 erasure spine must know whom to cascade to).
2. Asymmetric cross-team consent envelopes: governed by per-(peer,role) tuples (Epic 12) **extended to the team axis**; asymmetry (A shares X with B, B does not reciprocate) expressed in the manifest schema and enforced two-seam.
3. Multi-hop distillation provenance (the Reza "14 prior schema decisions cited in one consolidated proposal"): a cross-team distillate carries its **flattened I11 chain** (`source_log_ref` flattened-to-raw + `distillation_depth` + `intent_lineage`, per ADR-014/018) inside the re-attested crossing bundle — provenance lands with the row; ordinary traceback dereferences within the consumer team's own database.
4. Cross-wall raw traceback: dereferencing another team's TL is an **ADR-012-consented `log.recall`** to the source team, journaled on both sides; refusal is a **first-class surfaced outcome** (provenance-presence, ADR-049 §7 orphan discipline).
5. `check-multi-tenant-loom` (provenance legs): cross-team distillate provenance round-trip (flattened chain lands with the row; consented raw traceback works; unconsented refusal surfaced); **asymmetric-consent negative** (B→A share refused when only A→B consented). Proven-red on **real** cross-team writes.
6. ZERO kernel-Δ expected (reuses 11.2a re-attestation; cross-team dimension in maos-loom-lite; FLAG-Winston bounded only if a cross-team readmit seam is genuinely required — verify at preflight, recall the 11.2a +59 precedent).

**13.4 — FR37 vetting machinery** (ADR-056)
1. `VettingAttestation` = Ed25519-signed envelope binding (manifest **exact-hash**, from-tier, to-tier, vetter-key-id, expiry, `revocation_semantics`, optional `successor_policy`); out-of-kernel (registry + `maos-compliance`); `public-vetted` = the 4th trust tier; promotion is an **attestation artifact, never a registry flag**; kernel admission unchanged (the strictest-of floor already reads the tier).
2. Full flow with **INTERNAL vetter keys**: issue → install → promote → revoke round-trip on a clean host, verifier independently derived; accredited external vetters (NFR-Comp-2) explicitly **v2.5**.
3. Upgrade semantics (ADV-056-1): exact-hash → upgrade-without-current-attestation = **admission refusal at the floor** (the flap is the feature); `successor_policy` (`exact-only` | `re-issue-required-with-expedited-review`); the target version's attestation is evaluated **before the chain starts** (folded into `maosctl swap --plan` precondition, ADR-036).
4. Expiry/revocation vs running Spirits (ADV-056-2): v2.2 ships **`refuse-at-next-load` only** + a **mandatory journaled observation event** when the compliance layer detects expiry/revocation while an affected Spirit runs; audit distinguishes **four terminal causes** (vetting-revocation / expiry-lapse / registry-yank / operator-local); `drain-and-refuse` named as the v2.5 slot (honest zero-kernel-Δ).
5. Vetter-key lifecycle (ADV-056-3): enrollment/rotation/revocation are Ed25519-signed events **signed by the operator audit key** (§7.3 root), journaled; `verify` walks attestation → vetter-key enrollment → operator root, refusing attestations whose vetter key lacks a journaled enrollment predating issuance.
6. `check-vetting-attestation`: issue→install→promote→revoke round-trip (independently-derived verifier); forged-signature, expired-attestation, **and forged-vetter-key (unenrolled key, valid signature)** negatives; upgrade-flap control (new version without attestation refused at the floor); running-Spirit lapse produces the journaled observation; four-cause distinguishability. ZERO kernel-Δ.

**13.5a — Enterprise reference Spirit class**
1. The 11th reference Spirit, composed **Spirit-side** (zero kernel): PDP (11.4a) + identity/at-rest/SIEM (11.4c) into a reusable enterprise-governed Spirit **class**.
2. Enterprise governance E2E through the Spirit lifecycle: SSO/OIDC principal → Enterprise PDP decision → at-rest AEAD → SIEM export — **reusing** the 11.4a/c subsystems, not re-implementing them.
3. Composition proven at the **daemon seam** (E11 retro lesson — real subsystems passed isolated tripwires while dead-wired in 11.4c): an **available-arm integration leg** proves the Spirit is actually wired end-to-end, not merely constructed.
4. Reference-Spirit **template + docs** so an operator can instantiate an enterprise-governed Spirit (registry/scaffold reuse, ADR-008).
5. Gate (enterprise-reference-spirit leg, folds into `check-multi-tenant-loom` or a sibling): real SSO→PDP→at-rest→SIEM round-trip **through the Spirit**; issuance-bypass-absence; **dead-wire negative control** (a constructed-but-unwired Spirit reds).
6. ZERO kernel-Δ (Spirit-side). Depends 11.4a + 11.4c (both `done`).

**13.5b — Collective-tier erasure + legal-hold cascade**
1. Add an explicit collective erase contract to `CollectiveMemoryPort`; kernel `forget`/`forget_with_reason` reaches the collective tier rather than stopping at local memory.
2. Resolve every destination-team copy from 13.2's cryptographically-bound `source_team` provenance; never infer ownership from a mutable label or audit text.
3. A valid erase reconciles the source row and every authorized cross-team copy with independently-derived row and audit evidence.
4. A destination copy under legal hold is not deleted; the source and destination audit surfaces distinguish `held` from `erased`, `not-found`, and `failed`.
5. Partial failure cannot report success: remaining copies, responsible team, retry safety, and correlation IDs are explicit; a planted one-sided erase reds the gate.
6. Preflight owns the port/kernel design and baseline budget. A kernel arm is currently required by evidence; any repin is named and FLAG-Winston-reviewed rather than hidden behind an out-of-kernel shim.

**13.5c — Reza production collective path + tenant audit isolation**
1. An Enterprise reference Spirit performs collective read/write through a mediated production Spirit/daemon path; direct test-only store calls do not satisfy this AC.
2. A constructed-but-unwired collective adapter is a proven-red control; the path must reach the same real store guards proven by 13.1/13.2.
3. Implement the still-unserved NFR-Ops-11 tenant audit axis: per-team/per-operator TL isolation with stable task/team correlation and no cross-tenant raw-log visibility.
4. Cross-team actions reconcile requester team, destination team, consent decision, store row, and both audit references under one correlation ID.
5. Another team's raw TL is available only through consented `log.recall`; direct read, missing consent, or label-only team identity reds the gate.
6. Preflight verifies whether exposing the production collective path touches kernel-core. No ZERO-Δ claim exists until that route is grounded and counted.

**13.6 — Reza Cortex journey closer + NFR-Scale-5 envelope**
1. **Real composition:** one 3-team × 3-region run composes per-team Postgres, PDP/identity, physical+crypto tenant walls, asymmetric consent, vetting, the Enterprise Spirit, collective lifecycle, and the production collective/audit path from 13.1–13.5c. Constructed-but-unwired controls fail.
2. **Allowed collaboration + minimum disclosure:** Reza obtains the consolidated proposal through an allowed A→B asymmetric share. The crossing bundle contains only policy-allowed provenance; raw payload, secret-bearing fields, and unconsented TL references are negative controls.
3. **Explainable refusal and recovery:** reverse B→A share, stale tenant map, and vetting lapse/hold produce distinguishable operator outcomes naming the responsible authority and safe next action. Retry succeeds only after a valid manifest/consent/vetting repair.
4. **Lifecycle reconciliation:** source-team erase or legal hold exercises 13.5b against destination copies. Both audit sides reconcile `erased`/`held`/`failed`; a one-sided result or an unauthorized hold bypass is RED.
5. **Evidence-grade gate:** the Reza scene derives from real multi-team activity on the 3-region Postgres substrate. The 14-institution result remains a measured capacity envelope—never an assertion that 14 institutions executed. Each leg emits evidence state + artifact reference.
6. **Boundary preservation:** physical absence, team-key source-reflex, provenance minimum-disclosure, tenant TL isolation, duplicate/correlation reconciliation, and the post-13.5 baseline all hold. Any **ABSENT** or **INDETERMINATE** required leg blocks the Reza/v2.2 product claim.

---

## Gate discipline (§A7 reflexes named per gate — E11 retro carry-forward)
- **`check-multi-tenant-loom`** — derive-and-reconcile counts; a CI-provisioned real-Postgres two-`datname` physical leg; independently-derived verifier from the write codec; team-identity source-reflex after 13.2; physical-absence control a shared table cannot fake; explicit ABSENT declarations for unbuilt crypto/provenance/lifecycle/runtime/journey legs.
- **`check-collective-erasure`** — source→destination copy resolution from cryptographically-bound `source_team`; legal-hold negative; one-sided/partial-failure proven-red; independently reconciled store and TL evidence.
- **`check-reza-production-path`** — real Spirit/daemon→collective-store call path, dead-wire negative, tenant TL isolation, consented `log.recall`, and cross-team correlation.
- **`check-vetting-attestation`** — forged-vetter-key negative; exact-hash upgrade-flap control; four-cause distinguishability derived, not asserted.

### Evidence state is separate from enforcement class
Every journey-relevant leg emits exactly one evidence state: **`PROVEN_BLOCKING`**, **`PROVEN_LIVE_SIGNED`**, **`ABSENT`**, or **`INDETERMINATE`**, plus its artifact reference when proven. This is orthogonal to `BindingClass::{Blocking, AdvisorySubstrate}`: an unavailable live substrate can remain advisory for a development lane while its evidence state is `ABSENT`, which prohibits the Reza completion claim. `ABSENT` never becomes green; `INDETERMINATE` is neither failure nor completion and requires reconciliation.

## Kernel-delta budget
Baseline **23202** at the 13.1 preflight. Stories 13.1–13.5a retain their per-story ZERO/verify posture; **there is no Epic-wide ZERO claim.** 13.5b currently requires a collective port operation plus a kernel forget arm, and 13.5c may require a kernel-visible Spirit collective route. Both are FLAG-Winston seams: preflight must name the minimal surface, count it, update HISTORY, and prove no unrelated churn. Preserving the old ZERO claim by building a parallel out-of-kernel lifecycle owner is forbidden.

## Cut / deferred (not Epic 13)
- 100-host churn scale-out, 10-host rotation chaos → **Epic 14**.
- Accredited external vetters (NFR-Comp-2), external FKCS authors, external N=12 → **v2.5** (non-gating).
- App-D.4 partner-org federation tier (deferred, trigger = first partner-org request).
- 30-day soak (NFR-Scale-1) + absolute geo-SLO → **release-gate artifacts**, not closable ACs.
- SAML, Vault/cloud-KMS backends → additive-per-port, **Epic 14 sweep** or deferred.

## Pre-dev checklist (per story, at preflight)
1. Ratify F4 before 13.1 development: schema-v2 `[[teams]]`, accepted versions `{1,2}`, canonical signature compatibility, and `teams.is_some() ⇒ v2`. Until then 13.1 remains `draft`.
2. Name each gate's §A7 source and evidence state: derive-and-reconcile numerator, real-Postgres proven-red, team-identity source-reflex, erasure/hold reconciliation, dead-wire negative, forged-vetter-key negative.
3. Confirm/bound the 13.2/13.3 `CrossRegionReadmit` seam and the 13.5b/13.5c lifecycle/runtime seams; count in-`src` test modules. ZERO is a per-story result, not an Epic premise.
4. Record the frontier model + pre-book the complete §A6 net (incl. Test-Infra + runtime) for every adversarial boundary through 13.6.
5. Keep each story at ≤6 ACs. If 13.5b/13.5c grounding reveals another independently shippable mechanism, split it rather than hiding implementation in 13.6.
6. Author ADR-055/ADR-056 and `docs/loom-threat-model.md` at their owning stories; the threat model includes same-region forgery, authority compromise, malicious member, prompt-injection disclosure, erase/hold partial failure, and tenant-audit leakage.
