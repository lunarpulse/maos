# Sprint Change Proposal — Epic 13 rescoped 8 → 11 stories (H1 split + 13.5c scoping pass)

**Date:** 2026-07-18
**Author:** Bob (Scrum Master) running the course-correction workflow · with Lunarpulse (operator)
**Change scope classification:** **Moderate** (backlog reorganization — one story split in two, one story split in three, one dependency-row correction, two requirement-disposition corrections). **Not Major:** no PRD replan, **no new epic**, no rollback, **zero new FRs**, no change to any ratified ADR decision.
**Mode:** Batch — both rescopes were derived in one working session (two party-mode rounds + a 13.5c scoping pass), so the full set is presented at once.
**Evidence base:** 10 code-verification scouts + 1 independent checklist validator, all against HEAD `652347b8`. Every premise below was checked against code before being kept.
**Operator decisions ratified:** H1 split (approved); 13.5c three-way split (approved after Dana withdrew the two-way position); NFR-Ops-11 team-axis-only disposition (approved).

---

## Section 1 — Issue Summary

**Trigger:** Story 13.3 create-story analysis, then its party-mode preflight.

**Issue type:** *scope discovery during planning* — twice, at two different altitudes. Neither is a failed approach, a technical limitation, or a stakeholder pivot.

### 1.1 First issue — 13.3's provenance half has no buildable foundation

The epic's 13.3 AC sketch was written against `§15.3`, which was itself **a draft response to a rubric-review finding, not a design validated against code**. The architecture's own reviewer had already recorded the problem (`review-rubric-reconcile.md:62`):

> *"**NO §15 HOME** — the flagged question, answer: no, nowhere … This is the single most important reconciliation failure."*

Scouts confirmed the pessimistic branch of that finding:

| Premise | Code at HEAD |
|---|---|
| *"a cross-team distillate carries its flattened I11 chain inside the crossing bundle"* | `CollectiveKvLeaf` (`replication/leaf.rs:61-78`) and `CrossRegionReplicationBundle` (`bundle.rs:66-80`) carry **no `source_log_ref`, no `distillation_depth`, no `intent_lineage`** — no wire representation exists |
| *"ordinary traceback dereferences within the consumer team's own database"* | `flatten_source_log_ref` (`distillate.rs:196-253`) walks one local SQLite TL; no store selector, no remote fetch |
| *"an ADR-012-consented `log.recall`"* | ADR-012 binds cross-**Host** *write-side* frame admission on `(peer, intent-class)`; silent on reads, recall, and teams. `LogRecallPort` is 2 methods, `spirit_pid`-only; the consent check is **commented out** (`log_recall.rs:378-386`); `LogRecallError` has **no refusal variant** |
| *"per ADR-014/018"* | **`ADR-018` does not exist** in `docs/adr/` (index jumps 012 → 014). **`ADR-013`** — cited as the `log.recall` authority by `prd/domain-specific-requirements.md:9` — **also does not exist** |
| *(implicit)* cross-team citation is possible | `distillate.rs:330-359` (Story 8.10 AC2b) rejects `CiterUnauthorized` on `e.spirit_pid != spirit_pid` **over the flattened refs**, explicitly *"so a digest-of-digest cannot launder a cross-principal raw frame through an intermediate hop"* — **13.3's ask is what the control was built to prevent** |

**Root cause:** the sketch inherited a rubric *question* as if it were a ratified *answer*.

### 1.2 Second issue — 13.5c had become the epic's risk sink

Raised in preflight by the consensus-challenger seat: *"Four mechanism stories and every door piled into 13.5c — reading me an ADR that says the door is in 13.5c doesn't answer the risk, it **is** the risk."*

Grounded: the old 13.5c owned **seven** concerns (multi-store routing · Spirit mediation · refresh wiring · refusal audit · TL isolation · cross-team correlation · an unscoped daemon merge) and was **the only story in Epic 13 with no ZERO-Δ claim**. Three consecutive mechanism stories (13.1/13.2/13.3) had deferred every production wiring into it.

**Three scouts then found the load-bearing facts nobody had:**

1. **The "two composition roots" premise — which this workflow itself supplied — is FALSE.** `run_cohort_a2a_daemon_from_env()` is dispatched from *inside* `async fn main` at `main.rs:6934`, **4 648 lines after the primary root already constructed the TL, `LoomLiteStore`, `MemoryManagerAdapter` and `CapabilityRegistryAdapter`** (verified: no top-level `return` between `:1955` and `:4296`). It is a **zero-argument function that ignores all of it and builds a second TL.** The merge is *"give an existing function its arguments"* — ~180 LOC, ZERO kernel-Δ.
2. **Loom scopes are not manifest-declarable.** `capabilities_required_to_scopes` (`maos-manifest/src/manifest.rs:512-539`) emits only `ProviderInfer` and `McpCall`; `CapabilitiesRequired` has two fields. `Scope::LoomRead/LoomWrite/LoomScan` exist but no manifest can declare them ⇒ the first `issue_with_mediation(pid, Scope::LoomWrite, …)` returns `PolicyDecision::Deny` (`cap_policy/mod.rs:143`).
3. **The audit spine has never heard of the tenant wall.** `grep "team\|tenant\|operator_id" crates/maos-iac/src/` → **zero matches**. `maos_audit::query` (`lib.rs:184`) is path-addressed and capability-free; `ranged_recall` **discards its own filter** (`log_composition.rs:125`, `let _ = spirit_filter;`).

### 1.3 Third issue (discovered, not sought) — NFR-Ops-11 is mis-mapped

NFR-Ops-11 is titled **multi-*operator*** tenancy isolation and names four sub-axes. **Reza is one operator with three teams.** The PRD conflated operator and team; Epic 13 inherited the conflation. Of four sub-axes, 13.5c legitimately owned **exactly one**.

---

## Section 2 — Impact Analysis

### 2.1 Epic impact

| Epic | Impact |
|---|---|
| **Epic 13** | **Rescoped 8 → 11 stories.** Story list, sequencing, per-story AC sketches, gate discipline, kernel-delta budget, and pre-dev checklist all amended. **Critical path changes: 13.5c becomes the unblocker.** |
| **Epic 14** | **None.** Nothing here is scale-out, deferred-surface completion, or constitutional accounting — Epic 14's three permitted modes. Its sweep list stays closed. |
| **Epics 1–12** | **None**, with one documentation correction: 13.1's F14 note carried a misattribution (below). |
| **New epic** | **Not needed** — that would classify this Major. |

### 2.2 Requirement-disposition corrections (both are honesty corrections, not scope changes)

**NFR-Ops-11 — served on the TEAM axis, NOT closed.**

| Sub-axis | Prior mapping | Corrected disposition |
|---|---|---|
| (i) per-operator namespace | E13.1/13.5c implied | **Deferred past v2.2** — operator-axis; the v1.0 "reservation" is two string literals (`reserved_namespaces.rs:32`) with no non-test consumers, deliberately held *outside* the grammar-lock hash |
| (ii) per-operator TL shard | "addressed by NO Epic-13 story" (13.1 F14) → 13.5c | **13.5e, on the team axis** |
| (iii) per-operator capability-token signing key | *"13.2 covers the signing key"* (13.1 F14) | **MISATTRIBUTION — corrected.** 13.2 shipped `derive_team_signing_seed` in `maos-audit/src/sealed_export.rs` (the **replication-bundle** key). NFR-Ops-11(iii) means the **capability-token** signing key (`main.rs:2116`) — one global random-per-boot value carrying a `FIXME`. **Had no owner in Epic 13. Deferred past v2.2** (operator-axis) |
| (iv) per-operator GDPR-erasure scope | 13.5c implied | **13.5b's — always was.** 13.5c never owned it |

**Consequence: no artifact may claim NFR-Ops-11 complete at v2.2.** `requirements-inventory.md:514` and `implementation-readiness-report-2026-07-10.md:54` both need this qualification when next revised.

### 2.3 Dependency corrections

- **The old 13.5c dependency row (`13.1, 13.3, 13.5a`) was stale** — it predates the H1 split and never listed 13.3b, even though its AC5 restated 13.3b's deliverable list verbatim.
- **13.6 now depends on 13.3b** in addition to 13.1–13.5e.

### 2.4 Artifact impact

| Artifact | Change |
|---|---|
| `epics/epic-13-reza-cortex-v2-2.md` | Status header; story table (+13.3b, +13.5d, +13.5e, 13.5c rescoped, 13.6 deps); sequencing; **new "13.5c scoping pass" section**; AC sketches for 13.3b/13.5c/13.5d/13.5e; gate discipline (+`check-tenant-audit-isolation`, `check-reza-production-path` split); kernel-delta budget → per-story table; pre-dev checklist items 3/5/6 amended + **items 7 and 8 added** |
| `implementation-artifacts/sprint-status.yaml` | 13.3 → `ready-for-dev`; +13-3b, +13-5d, +13-5e; 13-5c renamed/rescoped; 13-6 dependency note. **Epic 13 = 11 stories (YAML validated).** |
| `implementation-artifacts/13-3-*.md` | Rescoped in place — headline reframed, D12–D16 added, 6 ACs held |
| `implementation-artifacts/13-1-*.md` | **F14 signing-key misattribution struck and corrected in place** |
| `docs/adr/ADR-055` | Amendment pending at 13.3/13.5e (crossing semantics, `WriteEntryPoint` scope clarification, NFR-Ops-11 disposition) |

---

## Section 3 — Recommended Path Forward

**Chosen: Option A — split both stories along the seams the evidence draws.** Considered and rejected: (B) compress 13.3 and accept an ADR-grade security reversal inside a story task; (C) re-sequence 13.5c ahead of 13.3 — **dead on the dependency cycle** (13.5c depended on 13.1 + 13.3 + 13.5a); (D) absorb the daemon merge into 13.3 — **dead on ADR-055 §5**, which assigns it to 13.5c verbatim: *"Joining that source to production team-store routing is **explicitly Story 13.5c**."*

### Story ledger

| # | Story | ACs | kernel-core Δ | Depends |
|---|---|---|---|---|
| 13.3 | Cross-team asymmetric consent + the governed cross-team row *(rescoped)* | 6 | ZERO | 13.2, 12.2 |
| **13.3b** | Provenance crosses the wall *(new)* | ~5 | verify | 13.3 |
| **13.5c** | Single composition root + bootable tenant mode *(rescoped)* | 5 | ZERO | 13.1 |
| **13.5d** | Production Spirit→collective route *(new)* | 6 | ZERO (+`maos-manifest` ~75 LOC + schema bump) | 13.5c, 13.5a, 13.3 |
| **13.5e** | Tenant audit isolation — NFR-Ops-11 team axis only *(new)* | 5 | not ZERO | 13.5c |

### Rationale that decided each seam

- **13.3 ÷ 13.3b:** the provenance half needs a leaf **v3**, an **ADR-grade reversal** of a shipped security control, **two ADRs that must first be written**, and a `LogRecallPort` signature change. None of that is a story task.
- **13.5c ÷ 13.5d ÷ 13.5e:** three mechanisms sharing no code. The merge is `maos-bin` wiring; the Spirit route is `maos-manifest` + composition root; the audit shard is `maos-iac`/`maos-audit` storage. **13.5c alone makes tenant mode boot** — a real deliverable, which is what turned the two-way position into three-way.
- **AC5 → 13.3b:** it restated 13.3b's deliverables. 13.5e retains only *"label-only team identity reds the gate"*, emitting **ABSENT** until 13.3b lands.

### Sequencing note

**13.5a before 13.5d** — scouts judged that landing the Enterprise Spirit first gives 13.5d a live Spirit to hang the collective port on, roughly **halving** its wiring cost. Previously marked freely parallelizable; now an advisory ordering.

---

## Section 4 — Success Criteria

1. Epic 13 lists eleven stories with correct dependencies; `sprint-status.yaml` parses and agrees. ✅ *(validated)*
2. No artifact claims NFR-Ops-11 complete at v2.2. ✅ *(epic + sprint-status; inventory/readiness-report flagged for next revision)*
3. The F14 signing-key misattribution is corrected wherever it appears. ✅
4. Every new story carries its true delta posture, with **"kernel-core ZERO" and "zero delta" stated as separate sentences**. ✅
5. 13.3 remains at 6 ACs and `ready-for-dev`. ✅

## Section 5 — Carried forward (not closed by this proposal)

- **Region `source_log_ref` presence residual** — open since 11.2b D1, **still no named successor** after three sessions of trying.
- **At-rest seal is unpaired at HEAD** (`open_at_rest` has zero callers; `seal.rs:118` promises a composition-root unseal that does not exist) — a live defect in shipped 11.4c work, now **owned by 13.5a**.
- **`memlog.py` / `.memlog.md` format mismatch** — the script requires frontmatter the file lacks; appends are being done directly.
- **Two process lessons ratified into the pre-dev checklist:** *scout the story you are deferring into* (a deferral is a claim), and *every mechanism story's gate carries a dead-wire negative* (forward-only).
