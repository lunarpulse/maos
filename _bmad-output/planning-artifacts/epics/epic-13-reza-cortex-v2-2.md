# Epic 13 — Reza Single-Org Cross-Team Cortex (v2.2)

**Status:** `draft-ready-for-preflight` — created 2026-07-09 (Step 3 of the full-PRD planning plan). Built on the **RATIFIED** full architecture §15 (party-mode 2026-07-09, §15.11 record) + the ratified PRD delta. Second epic of the **v2.2 functional-completeness** phase; consumes Epic 12 (cohort mesh) and Epic 11 (multi-region Loom, PDP, identity).

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = **GA ledger only, non-gating for v2.2 dev** (E11 retro A3).

**Model/review discipline (E11 retro A1):** frontier-class dev allowlist + **§A6 full-layer review net is the binding control**. The tenant wall (13.1), cross-team consent/provenance (13.2), and vetting-tier promotion (13.3) are **adversarial boundaries** → the full §A6 net (incl. Test-Infra + runtime) is **non-degradable** on those stories.

---

## Objective
Serve **Reza's single-org cross-team Cortex** (§10.7.2, committed v2.2) — a 400-person fintech running multiple teams as one governed Cortex on shared MAOS infrastructure. Epic 11 shipped the **enablers** (WASM form, multi-region Loom, PDP, identity/at-rest/SIEM); the journey itself is **unbuilt**. Epic 13 delivers the **tenant wall** (multi-tenant Loom, database-per-team, cryptographic per-team boundary), **cross-team asymmetric sharing with multi-hop distillation provenance**, the **FR37 vetting machinery** (the only unserved PRD FR — internal-vetter-first), the **Enterprise reference Spirit class**, and the **Reza Cortex scene E2E** on the 3-region substrate. **Zero new PRD FRs beyond FR37**; multi-tenancy arrives **outside the kernel** (the tenant wall is proven at the store gate, not in kernel-core).

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
| **13.5** | Enterprise reference Spirit class | The 11th reference Spirit, Spirit-side: PDP (11.4a) + identity/at-rest/SIEM (11.4c) composed into a reusable enterprise-governed Spirit; proven wired at the daemon seam (not just constructed). | 6 | frontier + §A6 | ZERO (Spirit-side) | 11.4a, 11.4c (both done) |
| **13.6** | Reza Cortex journey closer + NFR-Scale-5 envelope | 3 teams × regions on the 11.2b substrate composing 13.1–13.5; **Reza cross-team Cortex scene E2E**; NFR-Scale-5 14-institution capacity envelope (measured, gated not printed); no-leak under the live journey. | 6 | frontier + **full §A6** (journey) | ZERO | 13.1–13.5, 11.2b |

**Sequencing:** 13.1 (physical wall) → 13.2 (crypto boundary) → 13.3 (cross-team consent) → **13.6 (Reza journey closer)**, with 13.4 (FR37) and 13.5 (Enterprise Spirit) parallelizable after their deps and joining the closer. 6 large demo-anchored stories; the tenant wall is unpacked into a physical-absence proof + a forge-resistance proof; the closer *composes*, never builds.

**Demo-ability (smallest watchable multi-node demo):** two rungs — a **hermetic 2-team SQLite tenant-wall smoke** (13.1·AC5; blocking, no Postgres — the smallest observable wall, a team-B row refused to team-A) and the full **3-team × 3-region Reza Cortex scene** (13.6; real 3-Postgres = live-substrate-advisory per the A2 split). The small rung is the one you can watch anywhere; the journey-scale rung is gated where the multi-Postgres substrate exists — never silent-green when it doesn't.

---

## Per-story AC sketch (finalize at preflight)

**13.1 — Multi-tenant Loom — physical tenant wall** (ADR-055)
1. Database-per-team: distinct `datname` per team on operator-assigned Postgres; store-internal `team_guard` chokepoint **below** `CollectiveMemoryPort` (same layer as 11.2b `region_guard`); a team's rows live in a database the other team's connection cannot name. **BLOCKING chokepoint grep-proof** (single enforcement path).
2. Mapping ownership (ADV-055-2): the team↔region↔datname mapping is a **signed section of the org manifest** (ADR-054 artifact); `team_guard` loads only from the manifest, verifies the signature at load, caches by version, and **refuses reads/writes when its cached version trails the announced current** (`ETenantMapStale`, fail-closed). Store-local config holds connection credentials **only** — never membership/placement.
3. Team membership identity-keyed + single-valued (ADV-055-3): a Spirit belongs to exactly one team per the manifest; `team_guard` verifies `(spirit_pid → team)` **and** that the connection in use is the one assigned to that team (`ETenantConnectionMismatch`, never a silent allow); a Spirit needing both teams' data = **two Spirits with an ADR-012-consented channel**.
4. Physical row-ownership: a re-attested copy in team B's database is **team B's row** for capacity + GDPR-erasure, with `source_team` provenance — the forget-cascade target (the 9.2 spine must know whom to cascade to). (Physical ownership here; the cryptographic re-attestation is 13.2/13.3.)
5. `check-multi-tenant-loom` (physical legs): **physical-absence control** (team-B rows unreachable from team-A connection, distinct-`datname` witness a shared table cannot fake); `ETenantMapStale` + dual-connection `ETenantConnectionMismatch` negatives; chokepoint grep-proof (single `team_guard` enforcement path). **Smallest watchable demo = a hermetic 2-team SQLite tenant-wall smoke** (team-B row refused to team-A, observable) as the **blocking** leg per the E11 retro A2 split; **real multi-Postgres** proven-red as the live-substrate-advisory leg where CI has no Postgres — never mock, never silent-green.
6. ZERO kernel-Δ expected (maos-loom-lite + store-internal guard, 11.2b precedent; verify vs 11.2a readmit).

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

**13.5 — Enterprise reference Spirit class**
1. The 11th reference Spirit, composed **Spirit-side** (zero kernel): PDP (11.4a) + identity/at-rest/SIEM (11.4c) into a reusable enterprise-governed Spirit **class**.
2. Enterprise governance E2E through the Spirit lifecycle: SSO/OIDC principal → Enterprise PDP decision → at-rest AEAD → SIEM export — **reusing** the 11.4a/c subsystems, not re-implementing them.
3. Composition proven at the **daemon seam** (E11 retro lesson — real subsystems passed isolated tripwires while dead-wired in 11.4c): an **available-arm integration leg** proves the Spirit is actually wired end-to-end, not merely constructed.
4. Reference-Spirit **template + docs** so an operator can instantiate an enterprise-governed Spirit (registry/scaffold reuse, ADR-008).
5. Gate (enterprise-reference-spirit leg, folds into `check-multi-tenant-loom` or a sibling): real SSO→PDP→at-rest→SIEM round-trip **through the Spirit**; issuance-bypass-absence; **dead-wire negative control** (a constructed-but-unwired Spirit reds).
6. ZERO kernel-Δ (Spirit-side). Depends 11.4a + 11.4c (both `done`).

**13.6 — Reza Cortex journey closer + NFR-Scale-5 envelope**
1. Reza single-org cross-team Cortex journey **E2E**: 3 teams × regions on the 11.2b 3-region substrate, per-team Postgres, PDP + identity stack, cross-team asymmetric consent + multi-hop distillation (13.1–13.5 **composed**).
2. The Reza scene reproducible E2E on the journey-acceptance harness — the "14 prior cross-team schema decisions cited in one consolidated proposal" (multi-hop distillation provenance surfaced across the wall, consented).
3. **NFR-Scale-5** 14-institution capacity envelope: **derived from measured per-instance load** (not asserted); capacity floor **gated, not printed**.
4. No cross-team leak under the full live journey: physical-absence + team-identity source-reflex hold across the scene (leak-negative control that a shared table cannot fake).
5. Gate (`check-multi-tenant-loom` journey legs + NFR-Scale-5 envelope): the Reza scene runs green on the 3-region substrate; **anti-canned** — the scene derives from real multi-team activity, never scripted output; derive-and-reconcile on the capacity envelope.
6. ZERO kernel-Δ; release-gate artifacts (30-day soak NFR-Scale-1, absolute geo-SLO) tracked separately, **not closable ACs**.

---

## Gate discipline (§A7 reflexes named per gate — E11 retro carry-forward)
- **`check-multi-tenant-loom`** — derive-and-reconcile counts; **real multi-Postgres** proven-red (no mock/loopback for the tenant wall — the E11 §A6 real-subsystem rule); independently-derived verifier from the write codec (ADR-049); **NEW team-identity source-reflex** (every cross-team read verifies source-team identity from the **derived team key**, not a label — the region/language-identity reflex analogue, §15.9 open-question 5); physical-absence control a shared table cannot fake; absent-result → **BLOCK at the v2.2 ship gate**.
- **`check-vetting-attestation`** — forged-vetter-key negative (the trust-root walk must red an unenrolled valid signature); exact-hash upgrade-flap control; four-cause distinguishability derived, not asserted.
- Live-substrate legs (real 3-team Postgres): E11 retro A2 split — hermetic logic leg (SQLite) **blocking**, live-multi-Postgres leg **advisory-substrate-gated** with the WOULD-HAVE-BLOCKED banner where no CI Postgres, **never silent-green**.

## Kernel-delta budget
Baseline **23081** — **[STALE 2026-07-16: repin to 23202 at preflight.** The j1-tier2 live-agent bridge spent one authorized delta (23147→**23202**, +55, `spawn_and_bridge` closes worker stdin under `Signals` control; FLAG-Winston Lunarpulse 2026-07-15). `xtask/kernel-core-baseline.toml` is the single source of truth. The ZERO-Δ claims below are **unaffected** — only the pin number is stale.**]** **ZERO expected across all 6** (multi-tenant Loom in `maos-loom-lite`; `team_guard` store-internal below the port; FR37 out-of-kernel in registry + `maos-compliance`; Enterprise Spirit Spirit-side). **Watch 13.2/13.3:** cross-team re-attestation (crypto boundary + cross-team writes) reuses the 11.2a `CrossRegionReadmit` write path — a **bounded FLAG-Winston seam** is possible only if the readmit path needs a distinct kernel variant for the team dimension (verify at preflight; recall 11.2a landed +59 because in-`src` `#[cfg(test)]` modules count — E11 retro A6). Every re-pin names its surface in HISTORY; churn outside the named surface is RED.

## Cut / deferred (not Epic 13)
- 100-host churn scale-out, 10-host rotation chaos → **Epic 14**.
- Accredited external vetters (NFR-Comp-2), external FKCS authors, external N=12 → **v2.5** (non-gating).
- App-D.4 partner-org federation tier (deferred, trigger = first partner-org request).
- 30-day soak (NFR-Scale-1) + absolute geo-SLO → **release-gate artifacts**, not closable ACs.
- SAML, Vault/cloud-KMS backends → additive-per-port, **v2.2 sweep (Epic 15)** or deferred.

## Pre-dev checklist (per story, at preflight)
1. Name each gate's §A7 source (derive-and-reconcile numerator, real-multi-Postgres proven-red, team-identity source-reflex, forged-vetter-key negative).
2. Confirm/bound the 13.2/13.3 FLAG-Winston seam vs cross-team `CrossRegionReadmit` reuse (or prove ZERO); count in-`src` test modules (A6).
3. Record the §A5 model tier (frontier-allowlist) + pre-book the §A6 multi-layer net (incl. Test-Infra + runtime) — **non-degradable** for 13.1/13.2/13.3/13.4/13.6 (physical + crypto tenant wall, cross-team consent, vetting-tier, journey).
4. Decompose to ≤6 ACs; confirm demo/journey-anchored (13.6 = the Reza Cortex scene).
5. **Hygiene:** author `docs/adr/ADR-055-multi-tenant-loom.md` + `docs/adr/ADR-056-fr37-vetting-attestation.md` from the §15.11 decisions; author `loom-threat-model.md` **before** 13.1 ships (Fork-4 weld closes same-region insider-team forgery; threat model still covers org-authority-key compromise + malicious team member).
