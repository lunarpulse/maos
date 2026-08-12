# Epic 13 Context: Reza Single-Org Cross-Team Cortex (v2.2)

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Deliver the Reza single-organization, cross-team Cortex journey: independently operated teams can collaborate on shared MAOS infrastructure without dissolving team data boundaries. The epic makes tenant isolation, consented cross-team sharing, traceable distillation, Spirit vetting, enterprise governance, lifecycle handling, production wiring, tenant-aware audit, and the three-team/three-region journey demonstrably real. It turns Reza from a human routing table into a policy-setting platform lead while preserving minimum disclosure, explainable refusal, and evidence suitable for the v2.2 completion claim.

## Stories

- Story 13.1: Multi-tenant Loom — physical tenant wall
- Story 13.2: Multi-tenant Loom — cryptographic tenant boundary
- Story 13.3: Cross-team asymmetric consent + multi-hop distillation provenance
- Story 13.4: FR37 vetting machinery
- Story 13.5a: Enterprise reference Spirit class
- Story 13.5b: Collective-tier erasure + legal-hold cascade
- Story 13.3b: Provenance crosses the wall
- Story 13.5c: Single composition root + bootable tenant mode
- Story 13.5d: Production Spirit→collective route
- Story 13.5e: Tenant audit isolation
- Story 13.6a: Authenticated team identity
- Story 13.6b: Production cross-team crossing initiators
- Story 13.6c: Three-team / three-region substrate
- Story 13.6d: Cross-wall recall production initiator
- Story 13.6e: Judge machinery — derived evidence ledger and blocking `ABSENT`
- Story 13.6: Reza Cortex journey closer + NFR-Scale-5 envelope

## Requirements & Constraints

Team tenancy must be explicit rather than inferred: each Spirit has one authenticated team identity, each team owns its own data residency, and a cross-team use is an explicit asymmetric consent decision. A permitted share may provide only the allowed payload and provenance; it never authorizes another team's writes or unconsented raw-log access. Provenance must retain flattened references to original evidence across hops, but provenance presence is not authorization. A denied cross-wall recall must remain a surfaced, journaled refusal rather than appearing as an empty successful result.

The journey must support an allowed collaboration, distinguishable refusal and recovery for stale tenancy, consent, vetting, and hold conditions, and reconciliation of collective erasure or legal hold across source and destination copies. Partial deletion or a one-sided result cannot report success. The FR37 flow must offer a signed, journaled, revocable promotion to the fourth trust tier with internal vetter keys; externally accredited vetters are out of scope. Tenant audit isolation serves the team axis of multi-operator tenancy only; operator namespace and token-key axes remain deferred.

Evidence is a product requirement: required gates need real substrate legs, independently derived verification where applicable, physical-absence and forged-input negatives, and dead-wire controls for mechanisms not yet live. Each journey-relevant leg must report `PROVEN_BLOCKING`, `PROVEN_LIVE_SIGNED`, `ABSENT`, or `INDETERMINATE` with an artifact reference when proven. `ABSENT` and `INDETERMINATE` prohibit the Reza completion claim. The final journey is a three-team, three-region proof; the 14-institution result is a measured capacity envelope, not an executed deployment.

## Technical Decisions

Tenant isolation is database-per-team: distinct operator-provisioned Postgres databases, a store-internal guard beneath the collective-memory interface, and independent per-team Merkle roots. A shared table with a team predicate is not an acceptable substitute. The signed organization manifest is the sole owner of team-to-region-to-database placement and team membership; connection credentials are not manifest data. Manifest staleness fails closed, and identity or connection mismatch is refused. A cross-team copy belongs to the destination team for storage, capacity, and lifecycle purposes while retaining cryptographically bound source-team provenance.

The tenant boundary adds a per-team HKDF key weld over the region key. Verification derives the expected key from the claimed region and team rather than trusting bundle contents. Versioned leaves must preserve frozen prior encodings while including the data needed for tenant provenance and later cross-wall provenance. Cross-team sharing remains a re-attested write into the destination database, never direct access through a shared store. Consent extends the cohort's per-peer, per-role model to the team axis and must be enforced at both crossing seams.

Vetting remains outside the kernel in registry and compliance components. An attestation is exact-hash-bound, signed, journaled, and linked through an enrolled vetter key to the operator audit-key root. An upgrade lacking a current attestation is refused at its ordinary trust floor. v2.2 uses refuse-at-next-load plus a journaled observation for a running Spirit whose attestation lapses; audit must distinguish revocation, expiry, registry yank, and operator-local causes.

Maintain the kernel boundary honestly. Zero kernel-core delta is a per-story verified result, never an epic-wide premise; required owner-level changes must not be hidden behind parallel adapters. Production routes must be capability mediated, audit correlated, and fail closed. Existing enterprise PDP, identity, encryption-at-rest, SIEM, cohort, multi-region, and registry substrates are composed rather than reimplemented.

## UX & Interaction Patterns

Reza uses a platform-lead Orchestrator interaction to ask Spirits from separate teams for a unified recommendation. The experience must make the collaboration policy-visible: allowed asymmetric sharing can produce a consolidated, traceable proposal, while reverse or unconsented sharing gives an explainable refusal with a safe corrective action. A platform lead can trace a proposal to original evidence in one hop only when disclosure is consented; a refusal remains visible rather than being disguised as missing data. The final scene preserves team-owned write boundaries and supports a structured halt with a recommendation rather than requiring Reza to manually route every conflict.

## Cross-Story Dependencies

The physical wall precedes the cryptographic boundary; both precede cross-team consented sharing. The single composition root and bootable tenant mode are the production unblocker: tenant protections are not considered live merely because their mechanism exists. It must land before wall-crossing provenance can wire its consent layer, production collective routing, and tenant audit isolation. Cross-wall provenance and authenticated team identity precede the production crossing initiators; write-side and read-side crossing are separate production obligations. The three-team/three-region substrate enables the journey evidence and can proceed independently of authenticated identity. Collective erasure depends on cryptographically bound source-team provenance and closes the lifecycle path used by the final journey. FR37 vetting and the enterprise reference Spirit are parallelizable against the tenant-wall sequence. Story 13.6e supplies the required judge: derived leg states, signed live evidence, enforced product-claim refusal, workflow artifacts, and observational successors. It is complete and is the direct prerequisite that moves Story 13.6 to `ready-for-dev`; the closer consumes the ledger and does not rebuild it.