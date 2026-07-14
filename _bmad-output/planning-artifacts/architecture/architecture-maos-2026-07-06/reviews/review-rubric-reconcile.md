# Review — Rubric-Walker + Input-Reconciliation Lens

**Deliverable under review:** `architecture-maos-minimal-opus/15-full-spectrum-v2-2.md` + the 2026-07-06 edits (appendix-D disposition ledger, §14 dispositions, §13 new roadmap rows, §10.7 updates, §12.0.1 consolidation note).
**Inputs reconciled:** `full-prd-gap-map-and-planning-plan-2026-07-06.md` (§3.3 + Step 2), `prd-delta-full-spectrum-2026-07-06.md` (§4 hand-off), `prd/user-journeys.md` J3 + Reza delta banners.
**Reviewer:** rubric-walker / input-reconciliation lens · **Date:** 2026-07-06

## Verdict: PASS-WITH-FIXES

The deliverable lands the large majority of the two inputs' commitments, with real rigor in the places that were hardest (KLOC honesty, ADR-registry divergences, App-D ledger). The fixes below are pre-ratification items — none require restructuring §15, but two are HIGH because they are exactly the kind of silence that lets Epic-12/13 stories diverge.

---

## Part 1 — Input reconciliation

### 1.1 Gap-map §3.3 checklist (architecture gaps)

| §3.3 commitment | Status in deliverable |
|---|---|
| Five App-D dispositions (commit or retire each) | **LANDED.** App-D ledger (2026-07-06): D.1 COMMIT→§15.2/ADR-052; D.2 RETIRED (escape hatch retained, §13.1 note); D.3 DEFERRED w/ trigger + ADR-039 number correction; D.4 DEFERRED structure-reserved w/ trigger; D.5 COMMIT→§15.2. Note: D.4 "deferred" is neither commit nor retire, but it is explicit, trigger-named, and consistent with §15.4/§15.8 — acceptable. **Residue dropped:** §3.3-D.2's own text says "NFR-Test-7 rust↔subprocess leg still nominally v1.5 — reconcile" — NFR-Test-7 appears nowhere in the deliverable set (F-7). |
| OQ 1–10 dispositions | **LANDED.** All ten carry dated `Disposition:` lines in §14 with the CLOSED/OPEN/STANDS taxonomy; CLOSED items cite shipped evidence (11.3, 10.4a/11.2a/11.2b, 11.4a); OPEN items name v2.2 homes; #10 correctly reframed under the non-gating rule. One dangling pointer: OQ-2 dispositions "OPEN → v2.2 J3 scope" but §15.2 contains no approval-batching design element and `check-cohort-mesh` has no prompt-fatigue clause (F-8). |
| ADR registry consolidation | **LANDED, crisp.** §12.0.1 fixes authority (live `docs/adr/` wins), names the three known number divergences (031 WASM, 039 unsafe-policy, 040 inproc-gate vs threat-model-split), covers 041–050 + 051 reservation, and sets v2.2 numbering from 052. The stale reserved-ADR-039 line in §12.0 is banner-marked. This is the strongest edit in the set. |
| KLOC ceiling + single-tenant expiry | **LANDED, honest.** §15.5 admits the 20K letter-breach (23,081 pinned), ratifies the pinned-baseline + FLAG-Winston mechanism as the instrument, keeps ADR-038 per-crate ceilings unraised, commits ADR-041 completion (residual core ≤6K) inside the wave, sets kernel-crate-set ≤25K/alarm 23.5K through v3.0 `[ASSUMPTION]`, and expires single-tenant-per-kernel on schedule with tenancy landing outside the kernel. NFR-Maint-1 amendment is a recorded Step-3 hand-off. Correctly flagged constitutional (ADR-037 gate). |
| J3 / multi-tenant / FR37 designs | **LANDED** — §15.2/§15.3/§15.4, each with a named proposed gate. See Part 2 for envelope gaps. |
| Roadmap extension, one observable milestone per phase | **LANDED.** v2.0 (shipped, gate-set-green milestone), v2.2 (both scenes reproducible + zero-FRs-without-home), v2.5 (non-gating, signals-on-ledger) rows added; v1.5 "terminal" correctly rescoped. |
| Invariant-preservation proof-obligation (§3.3 item 4) | **ASSERTED, not walked.** §15 preamble declares everything additive and §0.6/I1–I14 unchanged; §10.7.3's claims are cited as "what §15 cashes." No per-invariant walk for the mesh or tenant wall (e.g., I14 under cohort choreography is asserted via the 11.3 playbook; I9 for the digest Spirit is cited). Acceptable at `proposed-v2.2`, but party-mode should demand the walk for ADR-052/053 specifically (F-9, LOW). |

### 1.2 Gap-map Step-2 named party-mode forks

| Named fork | Status |
|---|---|
| D.1 topology: gateway-mediated vs supervisor-fanout vs peer-DHT | §15.2 chooses a fourth option (full-pairwise-manifest) and dispositions gateway + peer-DHT in the ASSUMPTION tag. **Supervisor-fanout is never dispositioned** — party-mode will ask; add one clause (F-10, LOW). |
| D.4 commit-or-retire | Dispositioned (deferred + trigger). |
| Multi-tenant Loom shape (namespace vs instance-per-team) | §15.3 ASSUMPTION dispositions namespace-per-team explicitly; database-per-team ≈ instance-per-team. Landed. |
| Predicate stdlib (D.3) commit-or-defer | Deferred, trigger + number correction. Landed. |
| Post-v2.0 KLOC ceiling | §15.5 ASSUMPTION with rejected alternative named. Landed. |

### 1.3 PRD-delta §4 hand-off list

All items present: five App-D shapes ✓, OQ 1–10 ✓, ADR consolidation ✓, post-v2.0 ceiling ✓, J3 mesh + multi-tenant Loom + FR37 flow ✓, commitments/invariants preservation asserted ✓. However the PRD-delta **§1 v2.2 phase-content table** also lists **"100-host churn (NFR-Scale-2/Rel-7 second half)"** — it appears in the §13 v2.2 roadmap row but **has no §15 subsection, no ADR, and no gate** (the other five phase-content items all have one). See F-3.

### 1.4 Journey delta-banner capability audit

**J3 banner** (user-journeys.md ¶205) enumerates: cohort-topology operator UX / per-(peer,role) consent tuples / cohort hot-swap choreography / cross-agent halt-on-conflict / narrative team digest.

| Capability | §15 home |
|---|---|
| Per-(peer,role) consent tuples | §15.2 ✓ (manifest-carried, gate-covered incl. role-mismatch corpus) |
| Cohort hot-swap choreography | §15.2 ✓ (drain→swap→re-pin, migration chains, `EMigratorMissing` hop-naming, `maosctl swap --plan`) |
| Cross-agent halt-on-conflict | §15.2 ✓ (receipt-presence generalization; arbitration explicitly Director-side) |
| Narrative team digest | §15.2 ✓ (FR17-pattern Spirit, NOT kernel). Minor wording tension with PRD's "**kernel-rendered** narrative digest UX" — the PRD itself says the digest is "produced by a per-Host summarization step," so Spirit-produces/kernel-renders is coherent, but ADR-052 should say so in one line to prevent a story arguing for kernel-side digest assembly (F-11, LOW). |
| Cohort-topology operator UX | **THIN.** The manifest is operator-authored/signed/versioned, but the operator UX itself (authoring, distribution, at-rest location, skew during rollout) is undesigned — folds into F-2 (HIGH). |
| *(gap-map J3 row)* "no surveillance" posture proof | **DROPPED.** PRD J3 capability "Transparency Log per-Host (no team-wide surveillance)" and the gap map's posture-proof item have no §15.2 clause and no `check-cohort-mesh` negative control (e.g., member A cannot read member B's TL absent consent). See F-4. |

**Reza banner** (¶229) enumerates: cross-team asymmetric consent envelopes / multi-hop distillation provenance / multi-tenant Loom + residency / FR37 machinery / NFR-Scale-5.

| Capability | §15 home |
|---|---|
| Multi-tenant Loom + per-team residency | §15.3 ✓ (database-per-team, `team_guard`, per-team Merkle, physical-absence gate) |
| FR37 machinery | §15.4 ✓ (internal-vetter-first, revocation-vs-yank distinguishable, negative controls) |
| NFR-Scale-5 capacity envelope | §15.3 gate ✓ (derivation from measured per-instance load) |
| Cross-team asymmetric consent envelopes | **IMPLICIT ONLY.** §15.2's (peer,role) tuples + ADR-012's send/accept asymmetry arguably cover it, but "asymmetric consent envelope" (the PRD's payload-shape consent: fraud reads support's evidence read-only, neither authors the other's writes) is never named in §15. One sentence in §15.2 or §15.3 binding payload-shape consent to the cross-team case would close it (F-6, MED-LOW). |
| **Multi-hop distillation provenance (digest→raw across Cortex hops)** | **NO §15 HOME — the flagged question, answer: no, nowhere.** Not in §15.2/3/4, not in the roadmap v2.2 row, not in §15.6, not in §15.8's does-not-decide list. §10.7.2 treats it as already-substrate (I11/ADR-014 flattening, binding-v0.5) — but the gap map explicitly lists it as missing ("named in innovation doc, no FR/story") and the PRD banner lists it as *Outstanding for v2.2*. Worse, §15.3's own tenant wall creates a genuinely new question ADR-014 never faced: Reza's one-hop traceback follows `source_log_ref` from a digest in (or shared into) team B's context back to **raw frames that live behind team A's wall**, while §15.3's rule is "cross-team reads cross the guard, never the wall." Either the provenance read path crosses the guard with re-attestation (needs design + a gate clause), or the flattened refs are unresolvable cross-team and the Reza 4:55 PM scene does not reproduce. This is the single most important reconciliation failure. See F-1 (HIGH). |
| *(gap-map Reza row)* Loom-tier pattern libraries | Implicitly the per-team databases' content; never named in §15.3. One clause suffices (F-12, LOW). |
| *(gap-map Reza row)* 30-day soak + geo-SLO artifacts; 10-host mTLS rotation (NFR-Sec-13 v2.0 half) | **DROPPED.** No disposition in §15, §15.6, §15.8, or the roadmap v2.2 row. The gap map's own rule ("every residual requirement gets an explicit disposition") is violated by silence; even "release-gate artifacts, Step-3/Epic-14 material, not architecture" would be a valid disposition — but it must be written (F-5, MED). |

### 1.5 §15.6 remainder sweep vs gap-map §3.2 v2.0-phase-list

All ten items present and dispositioned: canary auto-rollback ✓, native push (closes OQ-7, cross-consistent) ✓, skill registry ✓, Vault/KMS ✓, distro packages ✓, Bedrock/Vertex/local ✓, vetting machinery (→§15.4) ✓, Enterprise reference Spirit class (correctly distinguished from 11.4a's PDP port) ✓, formal-methods disposition finally recorded ✓, `loom-threat-model.md` ordered **before** multi-tenant Loom ✓ (also cross-consistent with OQ-8's STANDS). Clean.

---

## Part 2 — Rubric (good-spine checklist)

### 2.1 Does §15 fix the real divergence points for story-level work?

Mostly yes: topology model, tenancy shape, tier-promotion mechanics, and the ceiling instrument were the four places two stories could have invented incompatible answers, and each now has a decision + rejected alternatives + gate. Two divergence points remain open — manifest lifecycle (F-2) and cross-team provenance path (F-1).

### 2.2 Is every proposed rule enforceable (each ADR names a gate)?

**Yes.** 052→`check-cohort-mesh` (live N=8, anti-canned per §A7), 053→`check-multi-tenant-loom` (physical-absence + proven-red — correctly inherits the 11.2b discipline), 054→`check-vetting-attestation` (round-trip + forged/expired negatives), 055→`check-kernel-baseline` + `xtask/kloc.toml` aggregate. Gate contents are falsifier-shaped, not coverage-shaped, consistent with house style. Gaps: no gate clause anywhere for 100-host churn (F-3) and no surveillance negative control (F-4).

### 2.3 Could anything under §15.8 "does not decide" let two stories diverge?

The listed non-decisions are safe (all v2.5/non-gating or trigger-deferred). The danger is what §15.8 does **not** list because §15 silently assumes it decided:

- **Cohort-manifest lifecycle** — where the signed TOML lives at rest, how it is distributed to 8 hosts, which key class signs it (operator root? a new cohort-authority key?), and above all **version-skew semantics during a rolling re-issue** (member on manifest v6 meshing with members on v7: refuse? grace window? strictest-of?). Two Epic-12 stories can and will answer these differently. (F-2, HIGH)
- **Cross-team provenance read path** (F-1) — same divergence mechanics.
- 100-host churn: without a §15 posture, one story will "scale 11.3" and another will treat it as a new envelope with new falsifiers. (F-3)

### 2.4 Is any whole dimension silent?

- **Operational/environmental envelope: YES, largely silent.** §11 (deployment topologies, untouched since May) was not extended for the 8-host cohort or the 3-team Cortex. Who provisions/runs the per-team Postgres instances is one adjective ("operator-assigned"); connection-string secret handling for `team_guard` (composes with the §15.6 KMS item?), cohort-manifest at-rest location, and the ops/runbook posture (the 11.3 re-pin playbook is cited for churn, but nothing covers cohort-manifest re-issue runbooks or per-team DB backup/restore vs Merkle roots) are absent. (F-2, HIGH)
- **Security envelope: HALF silent.** The Loom side is genuinely covered (loom-threat-model.md ordered pre-ship, poison-pattern surface named, OQ-8 rule-pack extension). The **cohort-mesh side has no threat model**: manifest-signing-key compromise (one key now grants mesh-wide role/consent authority — a higher-value target than any single TOFU pin), malicious/compromised cohort member, role spoofing via stale manifest, and vetter-key compromise for ADR-054 (gate covers forged/expired, not compromised-then-revoked-vetter cascade). The §8/NFR-Sec-14 threat-model split is same-host + cross-host-bilateral; nothing extends it to N-host cohort. (F-6-sec → rolled into F-6, MED)

### 2.5 Ratify-not-contradict the existing spine?

- **§7.2 bilateral A2A:** ratified by construction — "a mesh link is exactly a §7.2 bilateral channel," wire format unchanged, cashing App-D.1's claim. Good. **But ADR-003's own revisit clause says** "a use case requiring three or more Hosts coordinating in real-time … is a different architecture, not an extension." J3 is eight hosts coordinating in near-real-time. The pairwise-composition argument is exactly the right rebuttal — ADR-052 must make it explicitly against ADR-003 (compose-don't-amend note, or a scoped amendment via the ADR-037 path since 003 is binding-v0.1). Silence here invites a party-mode derail and, worse, a later claim that ADR-052 amended a binding-v0.1 ADR without the invariant-lock ceremony. (F-6, MED)
- **§9.3 Loom-lite:** ratified — §15.3 extends the store-internal layer, mirrors `region_guard` placement, keeps kernel tenancy posture unchanged; consistent with §10.7.3's extraction-without-API-churn claim and OQ-9's closure. Good.
- **§0.6 commitments / §3.2 invariants:** declared unchanged; ADR-055 correctly routed through ADR-037. Good (subject to F-9's proof-walk note).
- **§10.7 / §14 / App-D / §12.0.1 / §13 cross-consistency:** checked pairwise; consistent (OQ-3↔§15.2, OQ-7↔§15.6, OQ-8↔§15.6, D.2↔§13.1 note, §15.7 numbering↔§12.0.1). One internal contradiction found: **§15.1 titles its table "already shipped by Epic 11" and lists the sandbox-escape detector (11.4b — in review) and ADR-051 identity/at-rest/SIEM (11.4c — backlog, and §15.7's own text says 051 is *reserved*, i.e., not landed).** v2.2 is being stood on substrate that has not merged. Retitle the rows honestly ("shipped or in-flight (11.4b review / 11.4c backlog)") or party-mode ratifies a false premise — the 10.4b "false premise" lesson applies. (F-13 → rolled into findings as MED)

---

## Findings ledger

| # | Sev | Finding | Fix |
|---|---|---|---|
| F-1 | **HIGH** | Multi-hop distillation provenance (digest→raw across Cortex hops) — explicitly *Outstanding for v2.2* in the Reza PRD banner and the gap map — has **no §15 home**, and §15.3's tenant wall ("reads never cross the wall") makes the existing ADR-014 flattening claim non-obvious cross-team: Reza's one-hop traceback dereferences `source_log_ref`s that live behind another team's guard. | Add a §15.3 (or §15.2) subsection: provenance-read crosses the guard with re-attestation (design the path) + a `check-multi-tenant-loom` clause proving a cross-team digest's flattened refs resolve to raw evidence; or scope it out explicitly with the Reza-scene consequence stated. |
| F-2 | **HIGH** | Operational envelope silent: cohort-manifest lifecycle (at-rest location, distribution, signing-key class, **version-skew semantics during rolling re-issue**), per-team Postgres ownership/provisioning + connection-secret handling, §11 not extended, no runbook posture for manifest re-issue or per-team DB restore. These are story-divergence points §15.8 does not even register as open. | Add an ADR-052 lifecycle paragraph (skew rule at minimum: e.g., strictest-of or refuse-below-N-1) + one §11 topology row each for the 8-host cohort and 3-team Cortex; name the runbook artifacts. |
| F-3 | MED | 100-host churn (NFR-Scale-2/Rel-7 second half) is in the PRD-delta v2.2 phase table and the §13 v2.2 roadmap row, but §15 has no design posture, no ADR, no gate — the only v2.2 phase-content item without one. | One §15.6-style row or §15.2 clause: extend 11.3 substrate + `check-scale-churn` disposition idiom to N=100, or re-disposition explicitly. |
| F-4 | MED | J3 "no surveillance" posture proof (gap-map J3 row; PRD capability "Transparency Log per-Host (no team-wide surveillance)") has no §15.2 clause and no gate negative-control. | Add a `check-cohort-mesh` clause: member A cannot read member B's TL/scratchpad absent explicit consent tuple; digest carries only consented shares. |
| F-5 | MED | Quiet drops with no disposition anywhere: 30-day soak + geo-SLO release-gate artifacts; 10-host mTLS rotation (NFR-Sec-13 v2.0 half); NFR-Test-7 rust↔subprocess-leg reconciliation (named in gap-map D.2 itself). | One disposition line each (Step-3/Epic-14 release-gate artifact is a valid answer — but write it; NFR-Test-7 likely closes via ADR-031's cross-form gate — say so). |
| F-6 | MED | Spine-tension + security-envelope items: (a) ADR-052 silent on ADR-003's "three-or-more-Hosts = different architecture" revisit clause — needs the explicit compose-not-amend note; (b) no cohort-mesh threat model (manifest-key compromise, malicious member, role spoofing) — NFR-Sec-14 split not extended to N-host; (c) cross-team *asymmetric consent envelope / payload-shape consent* never named in §15 (implicit via ADR-012). | (a) one paragraph in ADR-052; (b) either extend `loom-threat-model.md`'s charter to a v2.2 threat-model doc covering the mesh, or add Sec-14c; (c) one binding sentence. |
| F-7 | MED | §15.1 claims 11.4b (in review) and 11.4c/ADR-051 (backlog; "reserved" per §15.7's own text) as substrate "already shipped by Epic 11" — internal contradiction and a false premise for party-mode. | Retitle/annotate the two rows with true status; state what v2.2 does if 11.4c slips. |
| F-8 | LOW | OQ-2 disposition points to "v2.2 J3 scope" but §15.2 has no approval-batching element or gate clause — dangling pointer. | Add the cached-decision heuristic as a §15.2 line item or re-point OQ-2 at Step-3 story scope. |
| F-9 | LOW | Invariant-preservation obligation (§3.3 item 4) asserted, not walked, for ADR-052/053. | Party-mode agenda item: per-invariant walk (I9 digest Spirit, I11/I13 across tenant wall, I14 under cohort choreography). |
| F-10 | LOW | Supervisor-fanout (a gap-map-named D.1 candidate) never dispositioned in §15.2's ASSUMPTION tag. | Add one rejection clause. |
| F-11 | LOW | PRD "kernel-rendered narrative digest UX" vs §15.2 "explicitly NOT kernel" — coherent (Spirit produces, kernel renders) but unstated. | One clarifying sentence in ADR-052. |
| F-12 | LOW | "Loom-tier pattern libraries" (Reza capability) only implicitly covered by §15.3's per-team databases. | Name it in §15.3's Binds clause. |

## What landed well (for the party-mode record)

- §12.0.1 is the best single edit: it resolves three live number collisions (031/039/040) that would otherwise have poisoned every future citation, and fixes authority direction cleanly.
- §15.5 is honest where it would have been easy to be cosmetic: it admits the 23,081 breach, refuses to raise per-crate ceilings, and ratifies the mechanism that actually held rather than a number that didn't.
- Every proposed ADR names a falsifier-shaped gate; §15.3 correctly imports the 11.2b physical-absence + proven-red discipline; §15.4 correctly keeps kernel admission unchanged.
- The §14 disposition pass is evidence-cited throughout, and §15.6 finally records the formal-methods disposition the PRD had left dangling since the v2.0 phase list was written.
