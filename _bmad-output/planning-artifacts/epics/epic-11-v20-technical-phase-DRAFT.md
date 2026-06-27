# Epic 11 — v2.0 Technical Phase (DRAFT)

**Status:** DRAFT — created by the Epic 10 retro action item §A1 to resolve `epic_update_required: true` (Epic 11/12 were never elaborated; post-v1.5 lived only as PRD phases). **Requires party-mode preflight ratification (Winston · John · Murat · Amelia) + Lunarpulse sign-off before any 11.x dev.**

**Hard precondition:** the v1.5 release holds must clear first — 10.5 clean re-review (`story-10-5-rereview-2026-06-26.md`), real external pen-test zero-P0/P1, export-counsel confirmation. Do not open 11.x dev while v1.5 is 🔴.

---

## Objective
Take MAOS from a 2-host bilateral deployment (v1.5) to a **multi-region, multi-instance, externally-extensible** platform: a WASM Spirit form, a 3-region Cortex pilot with cross-region collective memory, enterprise policy integration, and the first externally-authored Spirits validated end-to-end (FKCS). v2.0 is the **technical** phase only; ecosystem adoption (v2.5) is decoupled and planned separately (John's rule — engineering must not gate on cert-body MOUs).

## What v2.0 means
Cortex-ready: ≥3 regions, ≥10 agents, multi-instance Loom with cross-region replication, sandbox-escape detection, enterprise PDP, and a proven third-party authoring path — all without breaking the v1.0-frozen ABI.

## Dependencies inherited from Epic 10
- **Loom-lite single-instance** (10.4a) → multi-instance Loom with cross-region replication + consensus.
- **Mira–Nash bilateral A2A latency SLO** (10.4b, J4 P95) → Cortex multi-region SLO baseline.
- **Skill-format adapter** (10.5 AC1, ADR-027) → justification + scaffolding for the WASM Component-Model Spirit form.
- **3-host rotation chaos floor** (10.4b/10.5 AC6, ADR-022) → 28-Spirit / 3-region failure semantics.
- **Windows sandbox + restricted-token model** (10.5 AC3, R1/R4 fixed + CI-verified green) → cross-platform enterprise deployment.
- **ja/zh-Hans i18n scaffold** (10.5 AC5, DESCOPED per Epic-10 retro §A2 finding R2) → real Japanese + Chinese translation. The `i18n/{ja,zh-Hans}` trees exist only as untranslated Korean-placeholder scaffold (coverage gates report-only at v1.5); v2.0 replaces the content and re-instates 100% enforcement.

## Proposed stories (decompose at preflight; target 4–6 ACs each)

| # | Title | Scope | Risk class → model (§A5) |
|---|-------|-------|--------------------------|
| **11.1** | WASM Component-Model Spirit form | WIT contract `maos:spirit@1.0`; wasm host in kernel; cross-form semantic equivalence (any-rust ↔ wasm-component) ≥75% gate. Likely a significant FLAG-Winston kernel delta — budget at preflight. | kernel/unsafe → **opus-4-8** |
| **11.2** | Cortex multi-region + multi-instance Loom | 3-region pilot (≥10 agents); cross-region Loom replication + consensus on cross-incident pattern propagation; multi-region halt-metric expectations. | kernel/data-integrity → **opus-4-8** |
| **11.3** | Scale envelope (the cut 10.4 AC4) | 100-host churn at 10% turnover/week — detection ≤1h, recovery ≤24h (NFR-Scale-2/5); RTO-at-scale 4h-breach falsifiability (deferred from 10.4a). Decide real-hosts vs synthetic scaffold at preflight (10.4 used a synthetic 3-host scaffold). | scale/chaos → opus-4-8 preferred |
| **11.4** | Enterprise hardening | PDP integration (OPA / Cedar / Vault-style) for the Enterprise Spirit class; ADR-024 sandbox-escape detection (anomaly detector on Landlock/seccomp/Job-Object syscall patterns). | security → **opus-4-8** |
| **11.5** | FKCS protocol validation | 3 external-authored Spirits; ≥27/30 success per Spirit, ≥85/90 aggregate; diff-oracle confirms **zero kernel changes** from external authoring. | integration/validation → opus-4-6 + §A6 review |
| **11.6** | Real ja/zh-Hans localization (descoped 10.5 AC5 / R2) | Replace the untranslated Korean-placeholder scaffold in `i18n/{ja,zh-Hans}` with real machine translation under the locked-term model; **add a language-identity gate dimension** (the file-presence coverage + glossary-lock gates provably cannot detect wrong-language content — the root cause of the R2 fabrication); re-instate `JA_COVERAGE_MIN=100` / `ZHHANS_COVERAGE_MIN=100` as blocking. | docs/i18n → opus-4-6 + §A6 review |

## Cut / deferred to v2.5 (ecosystem phase, separate planning)
First third-party ComplianceClaim issuance, ≥20 external Spirits in registry, certification-body engagement, Cortex consortium case study, RTL layout localization. DevRel/BD-driven, parallelizable from v1.5 — **not** in Epic 11.

## Open questions for preflight
1. **WASM host kernel-delta budget** — 11.1 likely the largest kernel delta since E2; needs an explicit FLAG-Winston plan and a re-pin ceiling before dev.
2. **Cross-region consensus model** — what consistency guarantee for Loom pattern propagation (CRDT vs Raft-style vs TL-anchored)? Ties to the 9.4 region-pin/TL-anchored decisions.
3. **100-host churn realism** — real hosts vs synthetic scaffold; what makes the gate genuinely falsifiable at scale (avoid the 10.2 canned trap).
4. **FKCS external-author logistics** — who authors the 3 external Spirits; how to keep the diff-oracle honest.
5. **Apply §A7 reflexes** to every new gate (derive-and-reconcile, real-subsystem proven-red, language/identity where relevant).
