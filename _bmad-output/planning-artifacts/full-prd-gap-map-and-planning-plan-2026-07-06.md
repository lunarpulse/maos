# Full-PRD Planning — Gap Map and Planning Plan

**Author:** John (PM) · **Date:** 2026-07-06 · **Status:** STEP 1 EXECUTED (operator-directed, functionality-first) — see `prd-delta-full-spectrum-2026-07-06.md`; Steps 2–4 pending. **Naming supersession:** the functional wave proposed below as "v2.5" was numbered **v2.2** at execution (v2.5 keeps its established adoption-only identity, now explicitly non-gating); read this plan's "v2.5 = J3+Reza" references as v2.2.
**Trigger:** Epic 11 (v2.0 technical phase) is nearly closed (11.0–11.4a done, 11.4b in review, 11.4c/11.5/11.7 backlog). The minimal architecture (`architecture-maos-minimal-opus/`) declared v1.5/J4 its terminal milestone and its roadmap table stops there. The operator has directed: plan the **full PRD** — every user journey, full architecture, full epics and stories.

**Sources (read in full for this plan):** `prd/` (all 13 files), `maos-product-brief.md`, `architecture-maos-minimal-opus/` (scope, traceability, roadmap, open questions, appendices C/D/E, ADR index, foundational commitments), `epics/` (12-epic structure, requirements-inventory, Epic 11, open-items, dependency docs), `sprint-status.yaml`.

---

## 1. Where we stand

- **Delivered or in-flight through Epic 11:** kernel substrate at baseline 23081, 14 invariants, journeys J0 / J-Butler / J-Researcher / J1 / J6 / J4 proven; Epic 11 added WASM component-model Spirit form + cross-form equivalence (ADR-031 binding), cross-region convergent-replication Loom (ADR-049 binding), 3-region receipt-presence observability, 25/30-host churn envelope, enterprise PDP (fail-closed, wired into daemon), sandbox-escape detector (review), with 11.4c (identity/at-rest/SIEM), 11.5 (FKCS infra), 11.7 (trial infra) remaining.
- **Standing holds:** the shippable line is still gated on the two external v1.5 holds — real external pen-test (zero P0/P1) and export-control counsel (5D002.c.1). Full-PRD **planning** is not gated (same carve-out logic as Epic 11's hold-window); full-PRD **dev** inherits whatever holds remain when it starts.
- **Process constraints carried forward:** §A5 risk-gated model tiers per story; §A6 review net non-negotiable below opus-4-8; §A7 gate reflexes; gate-registry disposition idiom {v1_0, v1_5, v2_0, …}; kernel ≤20 KLOC (NFR-Maint-1 commits this only "through v2.0" — the full architecture must decide the post-v2.0 ceiling); 3–5 large stories per epic (operator preference); observable end-to-end demos as the validation frame.

## 2. The full-PRD target

The PRD's finish line is explicit and binary — **Tier 3 success criterion: a 14-site research consortium deploys a 28-agent Cortex on the unmodified public kernel and publishes about it** — reached through eight journeys teaching three cognitive modes (anticipatory → exploratory → compositional). Six journeys are served. The full plan must serve the remaining two and the ecosystem phase that cashes the substrate thesis ("has someone we've never met shipped something that depends on MAOS's protocol surface?").

## 3. Gap map

### 3.1 Journey gaps (the backbone)

| Journey | PRD tag | Status after Epic 11 | What's missing |
|---|---|---|---|
| **J3 — Marcus's day-30 Tuesday standup** (8-person team, peer A2A mesh) | v1.0 in PRD; deferred to v2.0 by architecture §10.7.1 | **UNSERVED.** Epic 11 contains no J3 story. 11.3 proved a 30-host *churn envelope*, not the journey. | Cohort-topology operator UX; per-(peer,role) consent tuples; cohort hot-swap/migration choreography (App-D.5 territory); narrative team digest across the mesh; cross-agent halt-on-conflict; "no surveillance" posture proof. Architecture asserted "no v1.5 decision forecloses J3" — never validated by construction. |
| **Reza — single-org cross-team Cortex** (400-person fintech) | v2.0/2.5 | **ENABLERS ONLY.** 11.1a/b (WASM form), 11.2a/b (multi-region Loom, 3-region observability), 11.4a/c (PDP, identity/SIEM) shipped the §10.7.2 prerequisites (a)–(c). | The journey itself: cross-team Spirits negotiating shared schema via **asymmetric consent envelopes**; **multi-hop distillation provenance** (digest→raw across Cortex hops — named in innovation doc, no FR/story); **Loom-tier pattern libraries**; registry vetting attestations beyond 3 tiers (§10.7.2(d), = FR37); cross-team A2A topology beyond bilateral (§10.7.2(e), App-D.1); **multi-tenant Loom w/ per-team data-residency** (NFR-Tenancy-1: out of scope before v2.5); NFR-Scale-5 14-institution capacity envelope; 30-day soak + geo-SLO (release-gate artifacts, still unexecuted); 10-host mTLS rotation (NFR-Sec-13 v2.0 half). |
| **J6 — Diego, final leg** | v1.0 (leg deferred v2.5) | Registry + 3 tiers + staged onboarding shipped. | **FR37**: vetter attestation promoting `public-untrusted` → `public-vetted` (the only v2.5-deferred FR; Diego's journey scene ends on this promotion). Requires NFR-Comp-2 vetter accreditation params in anger. |

### 3.2 Requirement-level residue (beyond Epic 11's plan)

**v2.5-tagged (committed, unplanned):** FR37; NFR-Scale-2/Rel-7 100-host churn; NFR-Test-5 FKCS *populated* (3 genuine external-authored Spirits: Negotiator / Tutor / Wet-Lab Coordinator + negative-control 4th that MUST fail); NFR-Test-8 external N=12 trial cohort (v2.0 ran Chinese-wall proxies); NFR-Doc-7 RTL; NFR-Tenancy-1 full multi-tenant; ecosystem outcomes (first third-party ComplianceClaim, ≥3 cert bodies, ≥20 external Spirits, Cortex consortium case study).

**v2.0 phase-list items Epic 11 did NOT absorb (verify-then-disposition):** sentinel canary auto-rollback; native mobile push (v1.5 shipped HTTP push); optional skill registry; Vault/cloud-KMS secret backends; official distro packages + one-line installer; full multi-provider (Bedrock/Vertex/local); registry v2.0 vetting machinery; **Enterprise reference Spirit** (PRD Spirit-count progression: 11th = Enterprise; 11.4a shipped the PDP port, not the Spirit); formal methods (TLA+/Alloy for I5/I6/I9, "landed by v2.0 if property tests insufficient" — disposition never recorded); `loom-threat-model.md` (poison-pattern attacks explicitly unaddressed pre-v1.5).

**Deferred indefinitely (do NOT silently resurrect; re-affirm or re-scope explicitly):** ja/zh-Hans docs (Epic 11 §8 supersedes NFR-Doc-6's v1.5 target; re-introduction bar = real human translation + script-identity gate).

### 3.3 Architecture gaps (what "full architecture" must decide)

1. **The five Appendix-D terminal shapes — commit or retire each:**
   - **D.1** multi-host topologies beyond bilateral (J3 mesh + Reza cross-team demand this; "primitives extend additively, wire format unchanged" is the claim to prove)
   - **D.2** rust-inproc via §13.1 measurement gate (Story 5.5e decision = defer-to-v2.0+; NFR-Test-7 rust↔subprocess leg still nominally v1.5 — reconcile)
   - **D.3** predicate stdlib beyond universal arithmetic (ADR-039 number reserved; Negotiator/Tutor FKCS classes are the named justification — FKCS population forces this question)
   - **D.4** federation trust tier between `org-internal` and `public-untrusted` (partner-org consumption; adjacent to FR37 vetting)
   - **D.5** multi-step hot-swap migration chains (operator UX for chain composition is the named open question)
2. **Minimal-architecture open questions 1–10** — each needs a disposition (several have matured: #3 A2A churn playbook partially cashed by 11.3; #5 Loom contention now measurable on real Postgres; #6 PDP-integration test now exists via 11.4a; #9 Loom schema review superseded by 10.4a/11.2a reality).
3. **ADR registry consolidation** — planning ADRs 001–040 vs implementation-era ADRs (through at least ADR-049, several flipped binding-v2.0 by Epic 11 stories). One authoritative index before new planning stacks on top.
4. **Invariant preservation proof-obligation** — the 8 foundational commitments + 14 invariants are declared phase-stable (§10.7.3); the full architecture must show J3 mesh, multi-tenant Loom, and federation tier land without weakening any (else ADR-037 constitutional amendment + major bump).
5. **Post-v2.0 KLOC ceiling** (NFR-Maint-1 expires at v2.0) and single-tenant-per-kernel commitment expiry (declared through v2.0).

### 3.4 PRD hygiene debt (fix before planning on top)

- `product-scope.md` still carries the **pre-restructure phasing** (Architect-at-v0.1, six Spirits at v0.5) — contradicts the operative scheme in `project-scoping-phased-development.md`; must be reconciled or annotated-superseded.
- PRD "line-239" FKCS milestone prose conflicts with NFR-Test-5 (flagged as Q4 in epic planning; never corrected).
- J3's **v1.0 tag** in `user-journeys.md` vs its actual v2.0+ deferral — re-tag honestly ("no silent de-scoping" is our own commitment).
- NFR-Doc-6 ja/zh v1.5 target vs indefinite deferral — record the supersession in the PRD, not just in Epic 11.
- Product brief's stale journey numbering / 20-week timeline — mark superseded (low stakes, one banner).

## 4. Proposed planning sequence

Four steps, in order, each producing a ratified artifact before the next starts. Party-mode convenes at steps 1 and 2 (the fork-bearing steps), per standing practice.

### Step 1 — PRD delta-pass (John, ~small)
Not a rewrite. Surgical: fix §3.4 hygiene debt; add the two full-form journey scenes as first-class planning anchors (J3 day-30 scene; Reza cross-team negotiation scene — both already written in the PRD, promote from "deferred" to "target"); ratify the **v2.5/v3.0 phase boundary** — my recommendation: **v2.5 = J3 + Reza journeys + ecosystem infrastructure** (engineering-ownable), **v3.0/vNext = consortium-scale Tier-3 proof** (14-site/28-agent — partially outside engineering's control, tracked as release gates not stories, per my own rule that engineering never gates on external actors).
**Output:** PRD delta document + corrected files. **Fork to ratify:** the phase boundary and whether Tier-3 consortium proof is in the committed plan or remains a falsifiable-thesis milestone.

### Step 2 — Full architecture (Winston, party-mode, ~the big one)
Extend `architecture-maos-minimal-opus` (do not fork a rival document — same constitutional spine, new phases past v1.5): disposition all five App-D shapes; disposition open questions 1–10; consolidate the ADR registry; architect J3 mesh (cohort topology, per-(peer,role) consent, cohort choreography), multi-tenant Loom (per-team residency), federation tier, FR37 vetting flow; extend the phased roadmap past v1.5 with one observable validation milestone per phase (v2.5 = J3 reproducible + Reza pilot reproducible is my proposal); re-affirm or amend the 8 commitments + 14 invariants + KLOC ceiling for post-v2.0.
**Output:** architecture v2 sections + ADR set. **Known forks for party-mode:** D.1 topology model (gateway-mediated vs supervisor-fanout vs peer-DHT), D.4 commit-or-retire, multi-tenant Loom shape (namespace-per-team vs instance-per-team), predicate stdlib (D.3/ADR-039) commit-or-defer, post-v2.0 KLOC ceiling.

### Step 3 — Full epics + stories (John + team, CE workflow)
Only after Step 2 ratifies. Draft shape (deliberately few, large epics — 3–5 stories each, every story an observable end-to-end demo):
- **Epic 12 — J3 Team Nexus:** peer-mesh normalcy journey end-to-end (cohort topology + consent tuples + team digest + cross-agent halt + day-30 scene reproducible).
- **Epic 13 — Reza Cortex:** cross-team asymmetric consent + multi-hop distillation provenance + multi-tenant Loom + Enterprise reference Spirit + Reza scene reproducible on 3-region substrate (consumes Epic 11 enablers).
- **Epic 14 — Ecosystem proof infrastructure (v2.5 engineering half):** FR37 vetting flow + FKCS population harness (external-author program mechanics, negative-control 4th Spirit) + external N=12 trial execution + 100-host churn + 10-host rotation. Ecosystem *outcomes* (cert bodies, ≥20 Spirits, consortium) tracked as release-gate artifacts, not stories.
- **Epic 15 (candidate, may fold into 12–14) — v2.0 remainder sweep:** the §3.2 unabsorbed items that survive Step-2 disposition (canary auto-rollback, native push, KMS backends, installers, Bedrock/Vertex, skill registry, formal-methods disposition).
**Output:** epics + stories listing, requirements-inventory v2 with every residual FR/NFR mapped or explicitly re-deferred.

### Step 4 — Implementation-readiness check (IR workflow)
PRD ↔ architecture ↔ epics alignment gate before any story enters dev. Also re-checks the two external holds' status at that date.

## 5. What this plan refuses to do

- No planning-time resurrection of indefinitely-deferred ja/zh. No silent de-scoping either — every residual requirement in §3.2 gets an explicit disposition in Step 3.
- No epic that gates engineering on external actors (cert bodies, volunteer authors, consortium formation). Infrastructure is ours; adoption is measured, not scheduled.
- No rival architecture document. One constitutional spine, extended.
- No dev start under the two external v1.5 holds without an explicit carve-out decision (same discipline as Epic 11's hold-window).
