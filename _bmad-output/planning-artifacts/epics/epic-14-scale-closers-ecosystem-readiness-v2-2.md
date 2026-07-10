# Epic 14 — v2.2 Hardening + Closers (Scale · Rotation · Ecosystem-Readiness · v2.0 Sweep)

**Status:** `draft-ready-for-preflight` — created 2026-07-10 (Step 3 of the full-PRD planning plan). Built on the **RATIFIED** full architecture §15 (§15.5 ADR-057 ceiling + §15.6 scale-closer/remainder dispositions) + the ratified PRD delta. **Third and final epic of the v2.2 functional-completeness phase** — **absorbs the former Epic-15 v2.0 remainder sweep** (folded 2026-07-10; the gap-map's "Epic 15 may fold into 12–14"). v2.2 = **3 epics**: 12 (J3 journey) · 13 (Reza journey) · 14 (this — hardening/closers).

**Scope note (supersession):** the v2.2 PRD delta **re-targeted 100-host churn (NFR-Scale-2 / NFR-Rel-7) and NFR-Scale-5 from v2.5 → v2.2** — the Epic-11 plan's "100-host → v2.5" is superseded by the functional-completeness phase. (NFR-Scale-5 14-institution envelope lives in Epic 13.6; this epic carries the host-count closers.)

**Dev-gate:** external holds (pen-test NFR-Sec-7 + export counsel NFR-Comp-1) = **GA ledger only, non-gating for v2.2 dev** (E11 retro A3). **All genuine external actors** (external Spirit authors, accredited vetters, N=12 trial participants) stay **v2.5, non-gating** — this epic proves the *infrastructure*, never recruits externals.

**Model/review discipline (E11 retro A1):** frontier-class dev allowlist + **§A6 full-layer review net is the binding control**. The two scale/chaos stories (14.1, 14.2) are the **highest test-infra risk** (the 11.3 "derived-metric-green-by-construction" trap) → the full §A6 net incl. Test-Infra + runtime is **non-degradable**.

---

## Objective
Close everything left to make v2.2 **functionally complete** once the two journeys (Epics 12–13) land: the **scale and reliability envelopes** that prove the substrate scales to ecosystem size (**100-host churn**, **10-host mTLS rotation chaos**); **ecosystem-infrastructure readiness** (FKCS 11.5, trial 11.7 proven at the v2.2 gate, every genuine external actor deferred and non-gating); the **v2.0 remainder sweep** — the product/operational surfaces deferred at v2.0 (sentinel canary auto-rollback, native mobile push, KMS secret backends, Bedrock/Vertex/local multi-provider, distro installers); and the **ADR-057 constitutional ceiling instrument** + the formal-methods disposition. **Zero new PRD FRs**; this is scale-out of proven substrates + deferred-surface completion + honest constitutional accounting.

## What Epic 14 stands on
| Substrate | From | Epic 14 use |
|---|---|---|
| 25/30-host churn envelope, real-timestamp derivation, two-surface detection, `check-scale-churn`, canned-scaffold deletion | 11.3 | **14.1** scales the SAME real mesh to N=100 — the envelope grows, the falsifiers travel with it (no new design) |
| `rotation.rs` floors, 3-host rotation drill, rotation-during-load, one-generation-overlap | 10.4b / 10.5 | **14.2** scales the ratified floors to 10 hosts (NFR-Sec-13 v2.0 half) |
| FKCS diff-oracle + `ProxyCohort` (Chinese-wall), negative-control 4th Spirit | 11.5 | **14.3** verifies FKCS-infra green at the v2.2 gate via the proxy cohort |
| Trial CI-bot derivation harness + `ProxyCohort` + `release_verify` | 11.7 | **14.3** verifies trial-infra green at the v2.2 gate; de-canned derivation holds |
| FR37 vetter-key lifecycle (internal-vetter machinery) | 13.4 | **14.3** structure-reserves the external-vetter enrollment path (disabled until v2.5 accreditation) |

## Ratified architecture basis — §15.6 (scale closers + remainder dispositions)
- **100-host churn** = scale-out of the Story-11.3 substrate to N=100 (same real-mesh, per-event derivation, two-surface detection); a new leg/param of `check-scale-churn`, **floors unchanged** (detection ≤1h median, blast ≤5 peers, recovery ≤24h, RTO 4h).
- **10-host rotation chaos** = scale-out of the ratified `rotation.rs` floors to 10 hosts; zero conversation drops under rotation-during-load.
- **30-day soak (NFR-Scale-1) + absolute geo-SLO** = release-gate pilot artifacts, **never CI-claimed**, absent/unmeasured → BLOCK at the v2.2 ship gate. **Not closable ACs in this epic.**

---

## Story list (decompose ACs at each story's preflight; ≤6 ACs)

| # | Title | Scope (one-line) | ACs | Model | Kernel-Δ risk | Depends |
|---|-------|------------------|-----|-------|---------------|---------|
| **14.1** | 100-host churn scale envelope (NFR-Scale-2/Rel-7) | Scale the real 11.3 mesh substrate to N=100; per-event derivation; two-surface detection; floors unchanged; blind-one-detector proven-red; new N=100 leg of `check-scale-churn`. | 6 | frontier + **full §A6** (test-infra risk) | ZERO @23081 (bench/test-infra) | 11.3 |
| **14.2** | 10-host mTLS rotation chaos (NFR-Sec-13) | Scale the ratified `rotation.rs` floors to 10 hosts; rotation-during-load → zero conversation drops; p99 + one-generation-overlap; real-timestamp derivation; `check-rotation` 10-host leg. | 6 | frontier + **full §A6** (test-infra risk) | ZERO (rotation.rs out-of-kernel, maos-a2a-tcp) | 10.4b / 10.5 |
| **14.3** | Ecosystem-readiness verification + v2.5 graduation ledger | FKCS-infra (11.5) + trial-infra (11.7) proven green at the v2.2 gate via the Chinese-wall proxy; FR37 external-vetter enrollment path structure-reserved-but-disabled; explicit v2.5 graduation ledger. | 6 | frontier + §A6 | ZERO (xtask + maos-fkcs + maos-eval infra) | 11.5, 11.7, 13.4 |
| **14.4** | v2.0 sweep — operational surfaces | Sentinel canary auto-rollback (ADR-020 ≤30s revert + Loom pattern-scan pre-deploy); native mobile push adapter (§7.4, OQ-7); distro packages + one-line installer (clean-host verified). | 6 | frontier + §A6 | ZERO (orchestration + channel adapter + packaging, out-of-kernel) | 11.7 (clean-env harness) |
| **14.5** | v2.0 sweep — backends + multi-provider | Vault/cloud-KMS secret backends behind the `maos-secrets` provider trait (composes 11.4c org-KMS); Bedrock/Vertex/local full multi-provider (ADR-005 driver additions, Inference Port FR47); provider-parity CI matrix. | 6 | frontier + §A6 | ZERO (backends + drivers behind traits; no vendor SDK in kernel) | 11.4c, ADR-005 |
| **14.6** | v2.0 sweep — constitutional ceiling + formal-methods disposition (ADR-057) | Land `kernel-crate-set.toml` ≤25K/23.5K aggregate leg (advisory → binding at v2.2-wave close); retro-residual discipline + measured-honest ~11.2K target; NFR-Maint-1 amendment; formal-methods disposition (evaluate the ADR-054 consent-composition matrix). | 6 | frontier + §A6 (constitutional) | ZERO (xtask instrument measures kernel, does not change it) | §15.5 ratification |

**Sequencing:** 14.1 · 14.2 · 14.3 · 14.4 · 14.5 · 14.6 are largely **parallelizable** (independent surfaces); 14.3 needs 13.4 (FR37 internal machinery); **14.6's kernel-crate-set ceiling binds at the v2.2-wave close** (the last gate to flip). 6 large stories; absorbs the former Epic-15 v2.0 remainder sweep.

---

## Per-story AC sketch (finalize at preflight)

**14.1 — 100-host churn scale envelope** (NFR-Scale-2 / NFR-Rel-7 second half)
1. Scale the **real** 11.3 mesh substrate to **N=100** (real kernel-process/container instances, mTLS mesh); the canned `churn.rs::run_scaffold` **stays deleted** (verify — the 11.3 deletion is the live 10.2-trap fix); every metric derived per-event from real timestamps.
2. Two-surface detection at scale: handshake `TcpTransportError` (verifier layer) + router NACK (the 11.3 F-new pattern), at N=100.
3. Floors **UNCHANGED** from 11.3, all **derived per-event** not asserted: detection ≤1h median, blast-radius ≤5 peers, recovery ≤24h, RTO 4h-breach. **The teeth are the falsifiers, not the clean pass** (11.3 L5).
4. Real planted adversarial hosts (per-class: pin-spoof, cert-rotation-race, consent-bypass) at N=100; the 10–20% turnover/wk × 4wk churn profile driven from real evictions/re-dials.
5. `check-scale-churn` gains a **new N=100 leg/param** (not a rewrite): blind-one-detector proven-red on **REAL** events (the 11.3 rework lesson — derived-but-loopback-trivial is still vacuous; falsifiers must bite on real evictions/reconvergence at scale); per-leg independence; live at N=100.
6. ZERO kernel-Δ @23081 (bench/test-infra); `churn-fault-inject` compiled-**OUT** of the release tree (11.3 discipline); the N=100 live leg is **advisory-substrate-gated** where CI cannot host 100 instances (E11 retro A2), never silent-green; NFR-Scale-1 30-day soak + geo hosts remain **release-gate artifacts, evicted from the AC set**.

**14.2 — 10-host mTLS rotation chaos** (NFR-Sec-13 v2.0 half)
1. Scale the ratified `rotation.rs` floors (10.4b/10.5 3-host drill) to **10 hosts** with real timestamps — floors stay **as strict as the ratified `rotation.rs`**, never relaxed to looser story numbers (the 10.5 AC6 regression-trap lesson).
2. Rotation-during-load: cert/key rotation across the 10-host mesh under active A2A conversation → **zero conversation drops** (the NFR-Sec-13 assertion), proven on real traffic.
3. Rotation p99 + **one-generation-overlap** (§7.2.1.a pre-staged-overlap idiom) proven at 10-host scale.
4. Real-timestamp derivation (not canned): rotation timing derived per-event; blind mutations red the rotation floor.
5. `check-rotation` gate scale-out (10-host leg): rotation-drop → RED, rotation-p99-breach → RED, floor-relaxation → RED; proven-red on real rotation events.
6. ZERO kernel-Δ (`rotation.rs` out-of-kernel in `maos-a2a-tcp`); the live 10-host leg **advisory-substrate-gated** per E11 A2.

**14.3 — Ecosystem-readiness verification + v2.5 graduation ledger**
1. **FKCS-infra (11.5)** verified green at the v2.2 gate via the in-house Chinese-wall **proxy cohort** (proof-of-mechanism): the diff-oracle **DERIVES** zero-kernel-change (no self-report flag); the negative-control 4th Spirit **fails**.
2. **Trial-infra (11.7)** verified green at the v2.2 gate via the proxy: the CI-bot harness **DERIVES** binary_loads / frames_run / halt_recall / sbom / signing (no self-report); a planted-lie negative control **reds**.
3. **FR37 external-vetter enrollment path structure-reserved**: the 13.4 vetter-key lifecycle accepts an external-vetter enrollment **shape**, but external vetters are **DISABLED / gated** — no external accreditation gates v2.2; enabling requires v2.5 NFR-Comp-2 accreditation (structure-reserved, trigger named).
4. **v2.5 graduation ledger** (explicit, in-writing): 3 genuine external-authored FKCS Spirits, external N=12 trial cohort, accredited external vetters → **DEFERRED to v2.5, NON-GATING**; the honesty clause is carried in the requirement text (Mary's "both cited or it traces to nothing"), not a footnote.
5. Gate (ecosystem-readiness aggregate): FKCS + trial infra run green via the proxy at the v2.2 gate; the proxy score is **advisory proof-of-mechanism, NEVER a blocking floor** (the 11.5/11.7 two-trust-boundaries discipline); v2.5-deferred items asserted **absent-by-design**.
6. ZERO kernel-Δ (xtask + `maos-fkcs` + `maos-eval` infra).

**14.4 — v2.0 sweep: operational surfaces**
1. **Sentinel canary auto-rollback**: pre-deploy Loom pattern-scan + ADR-020 **≤30s auto-revert** on canary failure; out-of-kernel orchestration; proven-red (a canary that trips the sentinel auto-reverts within the budget; a healthy canary does not).
2. **Native mobile push**: a real delivery-channel adapter behind §7.4 — replaces the `MobilePushCapture`/fixture path (8.13); **OQ-7 closes here**; no kernel change.
3. **Distro packages + one-line installer**: reproducible packaging + a one-line install, **verified on a clean host** (reuse the 11.7 hermetic clean-env harness — install from the packaged artifact, not the repo).
4. Gates: canary-auto-rollback proven-red (sentinel trip → revert within ≤30s; no-trip → no revert); push-delivery real (a real push received, not a captured fixture); installer clean-host verification (no prior MAOS state).
5. ZERO kernel-Δ (orchestration + channel adapter + packaging all out-of-kernel).
6. Demo-anchored: a canary deploy that auto-reverts on a planted regression + a real mobile push on halt + a one-line install on a clean VM.

**14.5 — v2.0 sweep: backends + multi-provider**
1. **KMS secret backends**: Vault + cloud-KMS (AWS/GCP) behind the existing `maos-secrets` provider trait; **composes with 11.4c org-KMS**; additive-per-port (no kernel change); the reference `LocalMasterKeyKms` (11.4c) stays **dev/CI-only**, Vault/cloud-KMS are the production backends.
2. **Full multi-provider**: Bedrock + Vertex + local (Ollama/llama.cpp) drivers via **ADR-005** driver additions behind the **Inference Port (FR47 — no vendor SDKs in kernel)**.
3. **Provider parity**: the multi-provider CI matrix (the 5.5b idiom) extended to Bedrock/Vertex/local; each provider passes the **same conformance suite**.
4. Gates: kms-backend conformance (each backend round-trips seal/unseal, wrong-key fails); provider-matrix (each provider passes the shared suite); provider-driver additions do not touch the Inference Port ABI.
5. ZERO kernel-Δ (secret backends + inference drivers behind traits; FR47/ADR-005 keep vendor SDKs out of the kernel).
6. Demo-anchored: a Spirit running against Bedrock and Vertex, with secrets sealed via Vault.

**14.6 — v2.0 sweep: constitutional ceiling + formal-methods disposition** (ADR-057)
1. **ADR-057 ceiling instrument landed**: new `xtask/kernel-crate-set.toml` enumerating the trusted computing base {residual `maos-kernel-core` + ADR-041 extracts + `maos-iac` + `maos-manifest` + `maos-capability` ≈22.5K}; a **new xtask aggregate leg**; ceiling **≤25K, alarm 23.5K** (kloc/production units); **advisory from landing → binds at the v2.2-wave close** (the WOULD-HAVE-BLOCKED banner idiom); membership change is FLAG-Winston.
2. **Retro-residual discipline** (ADR-057 clause 2): ceilings move down-only or to the tight measured residual at retro (+≤1% slack), recorded in the same commit; the **measured-honest ~11.2K** kernel-core residual is the target (the 6,000 figure is retired). ADR-041 Phase-3/4 extraction continues inside the v2.2 wave, tracked by this ceiling (preflight decides whether the extraction warrants its own story).
3. **NFR-Maint-1 amendment**: the excluding-tests letter is retained with the `kernel-crate-set` aggregate as its **post-v2.0 successor instrument** (PRD-delta touch).
4. **Formal-methods disposition**: NOT landed by default (property-test + structural-lint held 11 epics with zero invariant violations); the **ADR-054 consent-composition matrix** (per-(peer,role) × version-skew × in-flight drain) is the named first candidate — commit TLA+/Alloy **only if** the corpus cannot falsify a property (honest disposition, §15.6).
5. Gate: `kernel-crate-set` aggregate leg (advisory → binding at wave close); the ADR-057 **pin protocol** verified (a re-pin outside the named surface reds — the 10.5 R3 discipline).
6. ZERO kernel-Δ (the instrument measures the kernel, it does not change it).

---

## Gate discipline (§A7 reflexes named per gate — E11 retro carry-forward)
- **`check-scale-churn` (N=100 leg)** — derive-and-reconcile per-event (never canned constants — the deleted `run_scaffold`); real-mesh proven-red at scale (no loopback — the 11.3 rework); blind-one-detector; per-leg independence; absent-result → **BLOCK at v2.2 ship gate**.
- **`check-rotation` (10-host leg)** — floors as-strict-as-ratified (no relaxation — 10.5 regression trap); real-timestamp derivation; rotation-drop / p99 / floor-relaxation proven-red.
- **ecosystem-readiness aggregate** — derivation-provenance (FKCS/trial derive, never self-report — 11.5/11.7); proxy score advisory-only (two trust boundaries by tier); v2.5-deferred items absent-by-design, not silently-green.
- **v2.0-sweep gates** — `canary-auto-rollback` proven-red (sentinel trip → ≤30s revert; no-trip → no revert); `kms-backend` + `provider-matrix` (each backend/provider round-trips the shared conformance suite; wrong-key / wrong-provider negatives); **`kernel-crate-set` aggregate** (advisory → binding at the v2.2-wave close; ADR-057 pin protocol — a re-pin outside the named surface reds, the 10.5 R3 discipline).
- Live-substrate legs (N=100 mesh, 10-host rotation): E11 retro A2 split — hermetic logic leg **blocking**, live-scale leg **advisory-substrate-gated** with the WOULD-HAVE-BLOCKED banner where CI can't host the fleet, **never silent-green**.

## Kernel-delta budget
Baseline **23081**. **ZERO expected across all 6** — 14.1 bench/test-infra; 14.2 `rotation.rs` out-of-kernel (`maos-a2a-tcp`); 14.3 xtask + `maos-fkcs` + `maos-eval` infra; 14.4 orchestration + channel adapter + packaging (out-of-kernel); 14.5 secret backends + inference drivers behind traits (FR47/ADR-005 keep vendor SDKs out of the kernel); 14.6 the ceiling **instrument** measures the kernel, it does not change it. **Note (14.6):** ADR-041 Phase-3/4 extraction (kernel-core → port-trait crates) *reduces* kernel-core toward the measured-honest ~11.2K target — a **downward** move under the retro-residual discipline, recorded per-extraction in HISTORY. No upward FLAG-Winston seam anticipated; churn outside a named surface is RED.

## Cut / deferred (not Epic 14)
- **Genuine external actors** — 3 external-authored FKCS Spirits, external N=12 trial cohort, accredited external vetters (NFR-Comp-2) → **v2.5, non-gating** (14.3 reserves the structure + ledgers them).
- **30-day soak (NFR-Scale-1) + absolute geo-SLO** → release-gate pilot artifacts, not closable ACs.
- **v2.0 remainder sweep** — now **IN this epic** (14.4 operational · 14.5 backends/providers · 14.6 constitutional ceiling); the former Epic-15 is folded. **Exception:** `loom-threat-model.md` is **NOT** here — it is an **Epic-13 prerequisite** (must land before 13.1 multi-tenant Loom ships).

## Pre-dev checklist (per story, at preflight)
1. Name each gate's §A7 source (derive-and-reconcile per-event, real-fleet proven-red, derivation-provenance, proxy-advisory-not-blocking).
2. Confirm ZERO kernel-Δ (bench/rotation/infra all out-of-kernel); FLAG-Winston only if a seam is genuinely surfaced.
3. Record the §A5 model tier (frontier-allowlist) + pre-book the §A6 multi-layer net (incl. Test-Infra + runtime) — **non-degradable** for 14.1/14.2 (highest test-infra risk); 14.6 is constitutional (careful review).
4. Decompose to ≤6 ACs; confirm demo-anchored (14.1 = live N=100 churn run; 14.2 = 10-host rotation-under-load; 14.3 = FKCS+trial green via proxy; 14.4 = canary auto-revert + real push + clean-VM install; 14.5 = Spirit on Bedrock/Vertex + Vault-sealed secrets; 14.6 = kernel-crate-set ceiling live).
