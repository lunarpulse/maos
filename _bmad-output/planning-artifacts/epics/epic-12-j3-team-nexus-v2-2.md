# Epic 12 — J3 Marcus Team Nexus (v2.2)

**Status:** `draft-ready-for-preflight` — created 2026-07-09 (Step 3 of the full-PRD planning plan). Built on the **RATIFIED** full architecture §15 (party-mode 2026-07-09, §15.11 record) and the ratified PRD delta. First epic of the **v2.2 functional-completeness** phase.

**Dev-gate:** v2.2 **dev** inherits the two standing external holds as a **GA ledger only** (real external pen-test zero-P0/P1 NFR-Sec-7 + export counsel NFR-Comp-1) — **non-gating for v2.2 planning and dev** per the founder functionality-first directive (E11 retro A3). Story-level preflight is not gated.

**Model/review discipline (E11 retro A1):** frontier-class dev allowlist {opus-4-8, gpt-5.5, glm-5.2, equivalent}; the **§A6 full-layer review net (Blind+Edge+Acceptance+Test-Infra+runtime-execution) is the binding control** — completion requires the artifact. Consent/security-adjacent stories (12.2, 12.4) hold the full net non-degradably.

---

## Objective
Serve **J3 Marcus Team Nexus** — the 8-Host peer-mesh "team of agents at day-30 standup" journey — the last **UNSERVED** PRD journey (§10.7.1, committed v2.2). Architecture Epics 1–11 shipped every bilateral/mesh primitive; Story 11.3 proved a 30-host *churn envelope*. Epic 12 composes them into the **normalcy journey**: a declared cohort of 8 Hosts running as one team, with per-(peer,role) consent, cross-agent halt visibility, a narrative team digest, and a provable no-surveillance posture. **Zero new PRD FRs** — J3 is served by composing FR21–FR26/FR52–FR54 under one signed cohort manifest.

## What Epic 12 stands on (Epic 11 substrate, `done` on branch `epic-11`)
| Substrate | From | Epic 12 use |
|---|---|---|
| 25/30-host churn envelope, two-surface detection, re-pin playbook | 11.3 | N=8 mesh runs far inside the proven envelope; hot-swap reuses the re-pin playbook |
| Cross-region convergent replication + `region_guard` chokepoint + independent-verifier discipline (ADR-049) | 11.2a/b | `check-cohort-mesh` cross-issuer verification reuses the independence discipline |
| Live cross-host A2A TCP/mTLS + typed-intent consent + fail-closed (8.6–8.9, ADR-012) | Epic 8 | a mesh link **is** a §7.2 bilateral channel; per-(peer,role) tuples extend ADR-012 |
| Halt-continuity across hot-swap (I14), journey-acceptance harness (PTY/vt100 + ReplayInference) | 4.x / 8.15 | 12.5 continuity + 12.4 day-30 scene reuse the harness |

## Ratified architecture basis — **ADR-054** (cohort mesh; §15.2, ratified 2026-07-09)
Full-pairwise mesh of existing bilateral A2A channels, declared by a static Ed25519-signed cohort manifest. No discovery, no DHT, no gateway. Single cohort-authority key (k-of-n optional). Reserved always-allowlisted intent classes `{cohort.manifest.reissue, cohort.halt.receipt}`. Per-(peer,role) consent tuples, two-seam, acting-role exact-match. Linear-chain migrators. Receipt-presence halt observability. Amends ADR-003's "three or more Hosts" revisit clause by construction.

---

## Story list (decompose ACs at each story's preflight; ≤6 ACs)

| # | Title | Scope (one-line) | ACs | Model | Kernel-Δ risk | Depends |
|---|-------|------------------|-----|-------|---------------|---------|
| **12.1** | Cohort manifest + full-pairwise mesh foundation | Signed static cohort manifest schema v1 (members, roles, consent matrix, genesis authority key + monotonic version); N=8 full-pairwise A2A mesh; authority-signing + `ECohortManifestFork`; distribution + `T_stale` staleness ceiling; `check-cohort-mesh` core legs. | 6 | frontier + §A6 | **ZERO @23081** (composed bilateral + manifest, out-of-kernel; verify) | 11.3, Epic-8 A2A |
| **12.2** | Per-(peer,role) consent tuples | Two-seam send/accept allowlists; acting-role exact-match (no any-role OR); version-skew `ECohortManifestSkew` + in-flight drain; role queries from the manifest; consent corpus proven-red. | 6 | frontier + **full §A6** (consent) | ZERO (evaluation at the A2A seam, ADR-012/006) | 12.1 |
| **12.3** | Cross-agent halt-on-conflict (receipt-presence mechanism) | Halt receipts shipped as reserved `cohort.halt.receipt`; receipt-presence + explicit absence-marker observability per member; observability-not-arbitration by construction; halt-on-conflict scenario across the mesh. | 6 | frontier + §A6 | ZERO (receipts ride the reserved intent class; consumer is out-of-kernel) | 12.1 |
| **12.4a** | No-surveillance MECHANISM — cohort digest-read consent gate *(SPLIT from 12.4, preflight 2026-07-12)* | NEW non-reserved `cohort:digest-read` consent-gated read (request + consented response-push); wire the **production** rupture sink (`install_rupture_sink` — 12.2 wired the emit call, the sink is `#[cfg(test)]`-only); surveillance-negative control (out-of-matrix read refused **and visible to the target**) on the real N=8 mesh. | 6 | frontier + **full §A6** (consent/surveillance) | ZERO @23082 (new intent in maos-a2a-core + sink wiring in maos-bin; net-new BEHAVIOR, not compose) | 12.2, 12.3 |
| **12.4b** | Team digest + **J3 day-30 scene** (journey closer) *(SPLIT from 12.4)* | New `spirits/digest` Spirit; narrative I11-distillate digest (existing `write_distillate` seam) consuming 12.4a consented reads + the 12.3 receipt stream; **J3 day-30 Marcus standup scene E2E** on the 8.15 PTY harness, driven by a dataset **captured from a real 12.4a mesh run** (the harness has no live mesh). | 6 | frontier + **full §A6** (journey) | ZERO @23082 (digest Spirit-side; +1 workspace crate) | **12.4a**, 12.2, 12.3 |
| **12.5** | Cohort hot-swap + linear-chain migration *(cohort-lifecycle hardening)* | Per-member `drain→swap→re-pin` honoring I14/NFR-Rel-6; linear-chain migrators (`EMigratorMissing`); `maosctl swap --plan` chain-hash + `EMigrationPlanDrift` (extends ADR-036); halt-continuity across the swap. | 6 | frontier + §A6 | **FLAG-Winston bounded** iff `EMigrationPlanDrift`/`EMigratorMissing` need a kernel seam (verify at preflight; possibly ZERO); re-pin from 23081 named + disclosed | 12.1 |

**Sequencing:** 12.1 → 12.2 → 12.3 → **12.4a (no-surveillance mechanism) → 12.4b (J3 journey closer — demonstrable here)** → 12.5 (hardening, after the journey). 6 demo-anchored stories after the 2026-07-12 preflight split of 12.4 (the mechanism must be correct and shippable before the journey demo is anything but theater; F1 already forced two test surfaces — real N=8 mesh for the mechanism, PTY harness for the render). Hot-swap deliberately follows the showable journey (it is not on the day-30 *normalcy* critical path).

**Demo-ability (smallest watchable multi-node demo):** two rungs — a **hermetic N=3 minimal-cohort smoke** (12.1·AC2; the smallest "three or more" mesh, runs anywhere with zero infra) and the full **N=8 J3 day-30 standup scene** (12.4). Both run on the 8.15 journey-acceptance harness (PTY/vt100 + ReplayInference) — you *watch* the mesh work end-to-end, not just read a gate number. The N=3 rung means the mechanism is observable before the journey-scale demo.

---

## Per-story AC sketch (finalize at preflight)

**12.1 — Cohort manifest + full-pairwise mesh foundation**
1. Cohort manifest schema v1: signed TOML — members (`host_id`, pinned cert fingerprint, declared roles), per-(peer,role) consent matrix, genesis cohort-authority key (or explicit k-of-n set); Ed25519-signed; strictly-monotonic integer version.
2. Full-pairwise mesh, **smallest-watchable-first**: a **minimal N=3 cohort** (3 bilateral channels — the smallest ADR-003 "three or more" mesh) as a hermetic **watchable smoke** on the 8.15 journey harness, *then* N=8 → 28 bilateral §7.2 channels for the J3 journey; wire/mTLS+TOFU/logical-clock unchanged; membership changes = manifest re-issue, never runtime negotiation.
3. Manifest authority (ADV-054-1): only genesis-declared authority signs re-issues; member refuses non-authority signature + version regressions → `ECohortManifestFork` (names both versions); re-issue journaled to authority TL; each member journals its own acceptance of v(n+1).
4. Distribution + staleness (ADV-054-2): reserved always-allowlisted `cohort.manifest.reissue` (push + pull-on-connect fallback); `T_stale` (default §7.2 30s × 4) → degraded links + refuse consent-sensitive frames under stale matrix (fail-closed, Story-8.8 posture); revoked member refused mesh-wide within `T_stale`.
5. `check-cohort-mesh` (core legs): manifest round-trip with **cross-issuer verification** (independently-derived verifier, ADR-049 discipline); concurrent-re-issue negative control (`ECohortManifestFork` proven-red); stale-member leg. Live at N=8; anti-canned §A7 (manifest-authority-identity reflex).
6. ZERO kernel-Δ @23081 verified (mesh = composed bilateral primitive + manifest; out-of-kernel); FLAG-Winston only if a seam is genuinely required.

**12.2 — Per-(peer,role) consent tuples** (ADV-054-3)
1. Two-seam evaluation: sender checks `(receiver_peer, receiver_role)` vs send-allowlist; receiver checks `(sender_peer, sender_role)` vs accept-allowlist; **separate** send/accept tables (no transposition ambiguity).
2. Acting-role exact-match: consent envelope carries the single acting role; match exact, **never any-role OR** (ADR-012 confused-deputy extended).
3. Version skew: frames carry sender manifest version; receiver evaluates under its own (fail-closed wins); mismatch beyond ±1 → `ECohortManifestSkew` (distinct from `EIntentDenied`); in-flight frames drain under admitted version, new admissions under v(n+1) only.
4. Role queries answered from the signed, versioned manifest — a state read, not a discovery protocol.
5. Consent corpus proven-red: role-mismatch-on-allowed-peer refused; acting-role exact-match; skew negatives — each a real derive-and-reconcile gate leg, no synthetic rows.
6. ZERO kernel-Δ; evaluation at the A2A seam (out-of-kernel).

**12.3 — Cross-agent halt-on-conflict (receipt-presence mechanism)** (ADV-054-5)
1. Halt receipts stay local (single-halt-owner unchanged) + shipped as the reserved always-allowlisted `cohort.halt.receipt` intent class.
2. Receipt-presence observability: for each member, a receipt frame **or** an explicit transport-level absence marker (NACK/timeout per §7.2 30s) within T; absence is a first-class observable (11.2b principle).
3. Receipt-presence = observability, **not** arbitration — proven by construction (the receipt consumer has no arbitration/decision path; arbitration is the Director's, never the kernel's).
4. Halt-on-conflict scenario: a cross-agent conflict between two cohort members produces observable receipts / absence markers across the mesh (real induced halts, not synthetic rows).
5. `check-cohort-mesh` (receipt legs): receipt-presence per member under one induced member loss **plus** one induced connectivity loss (absence marker observed); derive-and-reconcile counts; proven-red on a blinded receipt consumer.
6. ZERO kernel-Δ @23081; **halt-source-identity reflex** (each receipt traces to a real emitting member, derived not labelled) + replay-dedup (a duplicate receipt `frame_id` must not inflate the presence count).

**12.4 — Team digest + no-surveillance — J3 day-30 scene (journey closer)** *(SPLIT 2026-07-12 into 12.4a mechanism + 12.4b journey; the AC sketch below is the pre-split intent — the binding ACs now live in the two story files. Preflight found the original premise false: the 12.3 receipt stream is reserved/consent-bypassed, no consent-gated cross-host read existed, and the rupture sink was a production no-op — 12.4a builds the missing `cohort:digest-read` primitive + wires the sink; 12.4b renders the scene from a captured mesh dataset since the PTY harness has no live mesh.)*
1. Digest Spirit reads only consented topics under its own per-(peer,role) tuples (12.2); every cross-member read is consent-checked and journaled.
2. Narrative team digest across the 8-Host mesh, **consuming the 12.3 receipt-presence stream** — halt-on-conflict events and member-absence surface in the digest.
3. No-surveillance posture: J3 acceptance corpus includes a **surveillance-negative control** — a digest query outside the consent matrix is refused **and visible to the affected member**.
4. **J3 day-30 Marcus standup scene reproducible E2E** on the journey-acceptance harness (hermetic; PTY/vt100 + ReplayInference reuse from 8.15), including a halt-on-conflict event surfaced through the digest.
5. `check-cohort-mesh` (journey legs): surveillance-negative control proven-red; the day-30 scene runs green at N=8; anti-canned — the scene **derives from real mesh activity**, never scripted output (a blinded mesh event must move the digest through the gate's own comparator).
6. ZERO kernel-Δ; digest is Spirit-side.

**12.5 — Cohort hot-swap + linear-chain migration** (ADV-054-4, absorbs App-D.5; cohort-lifecycle hardening after the journey)
1. Per-member `drain → swap → re-pin` honoring I14 + NFR-Rel-6 pin invalidation, using the 11.3 re-pin playbook.
2. Linear-chain constraint: migrator set per Spirit MUST form a linear chain; a second outgoing migrator for one source version = **manifest-validation error** (not a runtime choice); kernel chains hop-by-hop, refusing `EMigratorMissing` (names the missing hop).
3. `maosctl swap --plan` hashes the resolved chain; kernel refuses a chain whose hash ≠ the plan's (`EMigrationPlanDrift`; extends ADR-036).
4. Gate leg: linear-chain validation error + `EMigrationPlanDrift` proven-red on a REAL migration (not synthetic).
5. Kernel-Δ: verify at preflight — `EMigrationPlanDrift`/`EMigratorMissing` may need a **bounded FLAG-Winston seam** extending ADR-036 (else ZERO); any re-pin from 23081 named + disclosed in `kernel-core-baseline.toml` HISTORY.
6. Halt-continuity (I14) across the cohort hot-swap proven (reuse the 8.15 continuity beat).

---

## Gate discipline (§A7 reflexes named per gate — E11 retro carry-forward)
- **`check-cohort-mesh`** — derive-and-reconcile counts (never a self-reported flag); real-subsystem proven-red (live N=8 mesh, no mock/loopback for the consent + fork negatives); cross-issuer independent verifier (ADR-049); **manifest-authority-identity reflex** (accepted manifest's signer must be the genesis-declared authority, derived from the signature, not a label — the region/language-identity reflex analogue); absent-result → **BLOCK at the v2.2 ship gate** (`gate-registry.toml` `{…, v2_0=advisory, v2_2=blocking}`); anti-canned tripwire that moves the number through the gate's own comparator.
- Live-substrate legs (real N=8 mesh) follow the E11 retro A2 split where CI substrate is unavailable: hermetic logic leg blocking, live-mesh leg advisory-substrate-gated with the WOULD-HAVE-BLOCKED banner, never silent-green.

## Kernel-delta budget
Baseline **23081** (Epic 11 HEAD, branch `epic-11`). **ZERO expected** for 12.1/12.2/12.3/12.4 (mesh = composed bilateral primitive + manifest + reserved-intent receipts + Spirit-side digest). **12.5 only**: a bounded FLAG-Winston seam **iff** `EMigrationPlanDrift`/`EMigratorMissing` cannot be expressed by extending the existing ADR-036 plan machinery out-of-kernel (verify at preflight; recall §15.2 "near-zero kernel delta"). Every re-pin names its surface in HISTORY; churn outside the named surface (even a `cargo fmt` reflow, per 10.5 R3) is RED. Recall the 11.2a lesson (A6): count in-`src` `#[cfg(test)]` modules when estimating the seam.

## Cut / deferred (not Epic 12)
- Reza cross-team Cortex, multi-tenant Loom, FR37 machinery → **Epic 13**.
- 100-host churn scale-out, 10-host rotation chaos → **Epic 14**.
- App-D.4 partner-org federation tier (deferred, trigger = first partner-org request).
- External-author / consortium proof → v2.5 (non-gating).

## Pre-dev checklist (per story, at preflight)
1. Name each gate's §A7 source (derive-and-reconcile numerator, real-subsystem proven-red, manifest-authority-identity reflex).
2. Confirm/bound the 12.5 FLAG-Winston seam vs `EMigrationPlanDrift`/`EMigratorMissing` (or prove ZERO).
3. Record the §A5 model tier (frontier-allowlist) + pre-book the §A6 multi-layer net (incl. Test-Infra + runtime) — non-degradable for 12.2/12.4.
4. Decompose to ≤6 ACs; confirm the story is demo/journey-anchored (12.4 = the J3 day-30 scene).
