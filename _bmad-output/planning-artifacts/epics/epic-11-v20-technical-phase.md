# Epic 11 — v2.0 Technical Phase

**Status:** RATIFIED 2026-06-29 — party-mode preflight (Winston · John · Murat · Amelia) + Lunarpulse sign-off. Supersedes the 2026-06-29 DRAFT (Epic 10 retro §A1, `epic_update_required: true`). Full party-mode record: workflow `wyksr4yce`.

**Hard dev-gate:** Epic 11 **dev/merge** must NOT open while v1.5 is held. Remaining v1.5 holds = (1) real external pen-test returning zero P0/P1, (2) export-compliance counsel (5D002.c.1). The §A2 10.5 re-review hold is **CLEARED** (GO, 2026-06-29). **Ratified hold-window carve-outs (no Epic-11-dev dependency):** the 11.0 WASM de-risk spike (no-merge) ✓ DONE, ADR authoring (ADR-024 / ADR-031 WASM-component-form / ADR-049 Q2 consensus) ✓ DONE 2026-06-29, and the **ja/zh placeholder cleanup** ✓ DONE 2026-06-29 — the Korean-placeholder scaffold was **removed** and **ja/zh-Hans translation is deferred INDEFINITELY**; en + ko are the supported locales (LOCALES.md). _Story 11.6 (real ja/zh translation + language-identity gate) is **dropped from Epic 11**; only the cleanup + the deferral were kept._ **Entanglement:** 11.1's distributable form must not be finalized before export counsel clears — a WASM runtime can change the 5D002.c.1 classification.

---

## Objective
Take MAOS from a 2-host bilateral deployment (v1.5) to a **multi-region, multi-instance, externally-extensible** platform — a WASM Spirit form, a 3-region Cortex pilot with cross-region collective memory, enterprise policy/identity integration, and a proven third-party authoring path — **all without breaking the v1.0-frozen ABI**. v2.0 is the **technical** phase only; ecosystem adoption (v2.5) is decoupled and planned separately (John's rule — engineering must not gate on cert-body MOUs / author recruitment).

## Structure — single Epic 11, two waves (ratified)
~11 stories after the honest splits (Story 11.6 dropped — ja/zh deferred indefinitely, 2026-06-29), sequenced into two waves to pace the §A6 review load without renumbering v2.5:
- **Wave 1 — Foundation:** 11.0 → 11.1a → 11.1b · 11.2a → 11.2b
- **Wave 2 — Hardening / Validation:** 11.3 · 11.4a · 11.4b · 11.4c · 11.5 · 11.7

---

## Ratified architectural decisions (4-persona convergence)

1. **WASM host = adapter, not kernel.** Option A "WASM-in-subprocess": a wasmtime/component runner that *is* `spec.program`, sandboxed by the existing T2 path, speaking ADR-032 (Content-Length + CBOR over stdio). Host (WIT bindings, instantiation, fuel) lives in a NEW `maos-wasm-host` crate behind a `SpiritHostPort` injected at the daemon composition root (the Loom-lite/ADR-041 pattern). **In-kernel wasmtime embedding is FORBIDDEN** in Epic 11 — gated behind a future §13.1 measurement + superseding ADR. Protects the ADR-038 ≤6 KLOC kernel-core / ≤20 KLOC aggregate ceiling.
2. **Q2 cross-region consensus = TL-anchored CRDT** (grow-only / LWW-per-target, per-region-sovereign). **Raft-style CP REJECTED.** Cross-region propagation is a **mediated re-attestation** (verify against the source region's TL anchor → re-admit/re-sign under the destination region key with `source_log_ref` provenance), never transparent row replication — because `region.rs`/9.4b welds the region tag into the TL signing key (HKDF info) and the region tag is signature-covered, fail-closed (`ERegionViolation` on the store path; Ed25519 verify-failure on the TL/export path). **[Corrected 2026-06-29 per ADR-049 vs landed code: the weld is SIGN-ONLY (Option A, plaintext rows) — there is NO "sealed-export AEAD AAD"; that phrase was aspirational doc-comment wording, not landed code. The decision is unchanged.]** Convergence is proven by **independent per-region Merkle/payload-oracle re-derivation** (reuse 10.4b canonical-leaf-serde). Partition = AP-local-degrade (Collective→Private/Shared, no global halt); fail-closed on region-identity. Replication lives in `maos-loom-lite`, not kernel-core.
3. **Q3 scale = 25/30-host @v2.0** (NFR-Scale-2 / NFR-Rel-7 cost-compressed); **100-host → v2.5** (the DRAFT's 100-host is a scope error). **DELETE `churn.rs::run_scaffold` canned constants** (`detection=30/recovery=60` literals are the live 10.2 canned-trap at HEAD) and rebuild on the `rotation.rs` derive-from-real-timestamps pattern with real kernel-process/container instances. detection ≤1h median / blast-radius ≤5 / recovery ≤24h / RTO 4h-breach all **derived per-event**.
4. **ADR-024 authored first** (does not exist — `docs/adr/` jumps 023→026); the sandbox-escape detector lives **OUTSIDE kernel-core** (ADR-006 "the kernel learns no patterns"); the kernel raises only a **structural** alarm (NFR-Sec-3). A WASM-component-form ADR supersedes ADR-002/ADR-040 (folded into 11.1a).
5. **Story splits** (§A5 ≤6-AC ceiling): 11.1→a/b, 11.2→a/b, 11.4→a/b/c (enterprise full-bundle, ratified).
6. **11.1b equivalence metric** = behavioral-oracle **tiered** (100% on invariant-bearing effects — halt, frame sequence, capability denials, region-pin, audit frames; ≥75% slack only on cosmetic/latency surface), scoped to deterministic fixture Spirits, with a known-divergent component as the proven-red. The DRAFT's flat distributional ≥75% is **rejected** (it hits the `check_cross_form_equiv.rs` U-test NOT-APPLICABLE branch → vacuous on deterministic Spirits).
7. **FKCS diff-oracle honesty:** derive "zero kernel changes" from `abi-diff` vs `kernel-core-baseline.toml` named surfaces + `cargo-public-api` additive-only, measured before/after each admission — never a self-reported `abi_unchanged` flag. A negative-control "fourth Spirit" using an undocumented kernel internal MUST fail. Admission via the unmodified `SkillAdmissionQueue` (ADR-027).
8. **Language assurance — ja/zh-Hans DEFERRED INDEFINITELY** (2026-06-29; supersedes the original 11.6 plan). The Korean-placeholder scaffold (the R2 fabrication) was **removed**, ja/zh are **no longer configured locales**, and en + ko are the supported set (LOCALES.md). Story 11.6's "real ja/zh translation" is **dropped from Epic 11** — no committed target. The gate design is **retained in LOCALES.md as a hard precondition for any future re-introduction, not built now**: an automated Unicode-**script-identity** gate (ja = kana + 0 Hangul; zh-Hans = simplified-Han + 0 kana + 0 Hangul + a simp-vs-trad residual) + a native-reviewer runbook; **"MT + script-pass" must NEVER be claimed as "real translation."** Re-adding a locale without real human translation + this gate is forbidden.
9. **§A7 reflexes bind every new gate**, named per-gate at preflight, PLUS a **new region-identity reflex** (a count gate over propagated patterns must verify each pattern's source-region identity — the direct analogue of the language-identity reflex).
10. **Absent-result flips to BLOCK at the v2.0 ship gate** for every new Epic-11 gate (record `{v1_0, v1_5, v2_0}` per gate in `gate-registry.toml`; emit the §A7.5 "WOULD HAVE BLOCKED" banner during any advisory window).
11. **Model tiers + §A6 concentration:** 11.1a, 11.2a, 11.4b are **opus-4-8** and MUST hold the tier — pre-book the full multi-layer review **including the Test-Infra layer + a runtime-execution check** (the exact net that degraded on 10.5). A degraded/rate-limited review is not a review and hard-blocks completion.
12. **30-day soak (NFR-Scale-1)** is a release-gate artifact, NOT a closable in-sprint AC.

---

## Story list (decompose ACs at each story's preflight)

| # | Title | Scope (one-line) | ACs | Model (§A5) | Kernel-Δ risk | Depends |
|---|-------|------------------|-----|-------------|---------------|---------|
| **11.0** | WASM host de-risk spike *(no-merge; hold-window)* | Prototype `SpiritHostPort` + `maos-wasm-host`; fix the FLAG-Winston re-pin ceiling vs 22964; draft ADR-031 resolution + ADR-002/040 supersession. | 3 | opus-4-8 | ZERO (spike) | — (gates 11.1a) |
| **11.1a** | WASM Component-Model Spirit form — host + WIT | WIT `maos:spirit@1.0` as a typed projection of the ADR-032 frame set; `maos-wasm-host` adapter; subprocess launch via T2; launcher seam at composition root; WASM-form ADR. | 5 | opus-4-8 | **FLAG-Winston ≤ +150 LOC (→ ~23114), launcher seam ONLY; abi-diff proven-red** | 11.0; ADR supersession |
| **11.1b** | Cross-form equivalence gate | Behavioral-oracle tiered equivalence; per-scenario derive-and-reconcile; known-divergent component proven-red; anti-canned tripwire; ADR-031 → binding. | 4 | opus-4-8 *(falsifiability IS the risk)* | ZERO | 11.1a |
| **11.2a** | Multi-instance Loom + cross-region consensus | Replication over `CollectiveMemoryPort`; TL-anchored CRDT + mediated re-attestation; independent per-region convergence oracle; region-identity reflex; AP-local-degrade. Lead with the consensus ADR. | 5 | opus-4-8 | FLAG-Winston only if a region-mediation seam is required (likely ~ZERO; in `maos-loom-lite`) | 10.4a; consensus ADR |
| **11.2b** | Cortex 3-region pilot + multi-region SLO | 3-region ≥10 agents; multi-region halt-metric + cross-region SLO on the j4 histogram machinery + RTT budget; fail-closed region-identity proven-red on the LIVE read path. | 4 | opus-4-8 | ZERO | 11.2a |
| **11.3** | Scale envelope — 25/30-host churn | 25/30-host, 10–20% turnover/wk × 4wk, 3 planted adversarial hosts; DELETE canned scaffold → real-timestamp derivation; detection/recovery/RTO derived per-event; blind-one-detector proven-red. | 5 | opus-4-8 *(highest test-infra risk)* | ZERO | 11.2b |
| **11.4a** | Enterprise PDP integration | OPA/Cedar/Vault PDP behind an out-of-kernel policy port; decisions from real evaluation; deny-rule proven-red. | 5 | opus-4-6 + §A6 | ZERO | — |
| **11.4b** | ADR-024 sandbox-escape structural detector | Author ADR-024 first; out-of-kernel detector subscribing the SandboxViolation/seccomp/TraceSink stream; TP-floor + FP-ceiling; live-syscall proven-red (no mock); Windows runtime proven-red is windows-latest-CI-only. | 5 | opus-4-8 | FLAG-Winston bounded to an emission-seam ONLY if `SandboxViolation` lacks a subscribable sink (verify at prep; possibly ZERO) | ADR-024 |
| **11.4c** | Enterprise identity + at-rest + SIEM *(full PRD bundle, ratified)* | SSO/OIDC identity assertions + org-KMS encrypted-at-rest + SIEM export (NFR-Aud-11). The SSO→capability-token-issuance slice is security-adjacent → opus-4-8. | 5 | opus-4-6 *(token-issuance slice opus-4-8)* + §A6 | ZERO | 11.4a |
| **11.5** | FKCS infrastructure *(NFR-Test-5 first half)* | diff-oracle + harness + kernel-frozen-vN.0 tag; oracle derives zero-kernel-change from abi-diff/cargo-public-api; negative-control 4th Spirit MUST fail; counts derive-and-reconcile; absent→BLOCK@v2.0. **Infra-only** — 3 genuine external Spirits + N=12 → v2.5; validate in-house via Chinese-wall proxy authors. | 5 | opus-4-6 + §A6 | ZERO (by construction) | 11.1a + frozen-kernel-vN.0 tag (**sequence LAST**) |
| ~~**11.6**~~ | ~~Real ja/zh-Hans + language-identity gate~~ **— DROPPED 2026-06-29** | ja/zh-Hans translation **deferred indefinitely**; Korean-placeholder scaffold removed; en + ko are the supported locales (LOCALES.md). No committed target. | — | — | — | — |
| **11.7** | v2.0 third-party trial infrastructure *(NFR-Test-8)* | Build the v2.0 black-box trial infra (SBOM + signing-chain re-load on a clean VM by CI bot, halt-recall ≥0.85) + internal Chinese-wall proxy; genuine external N=12 → v2.5. May fold into 11.5. | 4 | opus-4-6 + §A6 | ZERO | 11.5 |

---

## Open-question resolutions
- **Q1 (WASM kernel-delta):** placement ratified (adapter crate; in-kernel embedding forbidden); ceiling = **+150 LOC from 22964 (→ ~23114), launcher seam only, abi-diff proven-red**. The 11.0 spike (running during the hold) validates/refines the number before 11.1a ACs commit. Cap at the magnitude of prior authorized deltas (10.4a +86 / 10.5 +355).
- **Q2 (consensus):** TL-anchored CRDT + mediated re-attestation; Raft rejected; independent per-region convergence oracle non-negotiable; in `maos-loom-lite`; AP-local-degrade; fail-closed on region-identity.
- **Q3 (scale realism):** 25/30-host @v2.0 (100→v2.5); delete canned scaffold; derive per-event from real instances.
- **Q4 (FKCS):** diff-oracle honesty ratified; **infra-only @v2.0** (genuine externals + N=12 → v2.5); Chinese-wall internal proxy authors; correct the conflicting prd line-239 milestone prose to match NFR-Test-5.
- **Q5 (§A7):** §A7 binds every gate + new region-identity reflex + real ja-vs-zh-Hans discriminator; absent-result → BLOCK at the v2.0 ship gate.

## Kernel-delta budget plan
Baseline verified at `src_lines = 22964`. Two ceilings both bind: the line-count tripwire AND the ADR-038/NFR-Maint-1 tokei ceiling (≤6 KLOC kernel-core / ≤20 KLOC aggregate, already overshoot-alarmed). **11.1a is the only large draw**: ≤ +150 LOC, launcher seam only, host code entirely in `maos-wasm-host`. 11.2a / 11.4b: FLAG-Winston bounded seams only if genuinely required (verify at prep; likely zero). All other stories: ZERO expected (11.5 zero by construction). Every re-pin recorded in `kernel-core-baseline.toml` HISTORY with the authorized surface named; churn outside the named surface (even a `cargo fmt` reflow, per 10.5 R3) is RED → revert or disclose.

## Cut / deferred to v2.5 (ecosystem phase, separate planning)
First third-party ComplianceClaim issuance, ≥20 external Spirits in registry, certification-body engagement, Cortex consortium case study, RTL layout localization, **100-host churn**, the **3 genuine external-authored FKCS Spirits + the external N=12 trial cohort**. DevRel/BD-driven, parallelizable from v1.5 — not in Epic 11.

**Deferred INDEFINITELY (no committed target):** Japanese + Chinese Simplified documentation translation. The Korean-placeholder scaffold was removed 2026-06-29; en + ko are the supported locales (LOCALES.md). Re-introduction requires real human translation + a language-identity gate — see LOCALES.md "Deferral Policy."

## Pre-dev checklist (per story, at preflight)
1. Name each gate's §A7 source (derive-and-reconcile numerator, real-subsystem proven-red, feature-flag≠measurement, region/language-identity where relevant).
2. Confirm/bound the FLAG-Winston re-pin surface and ceiling.
3. Record the §A5 model tier + risk rationale; pre-book §A6 multi-layer (incl. Test-Infra + runtime-execution check) for the three opus-4-8 stories.
4. Decompose to ≤6 ACs.
