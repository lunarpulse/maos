# Sprint Change Proposal — Workspace Env-Contract promoted to Epic 14

**Date:** 2026-07-13
**Author:** Developer (course-correction workflow) · with `{user_name}` Lunarpulse
**Change scope classification:** **Moderate** (backlog reorganization — promote a backlog story into an existing epic as new stories; epic-doc + sprint-status updates). Not Major (no PRD replan, no new epic, no rollback).
**Mode:** Batch (analysis was completed in the 12.6/12.7 party-mode preflights; this proposal presents the full set at once).

---

## Section 1 — Issue Summary

**Trigger story:** `12-6-env-contract-registry-remediation` (tech-debt remediation surfaced by the Story 12.5 code review). Its party-mode preflight spawned a ring-3 split, `12-7-workspace-env-contract-and-secret-vars` (backlog), to hold the *workspace-wide* env contract.

**Core problem (issue type: scope discovery during planning):** A preflight scan of the whole workspace showed `12-7` is not a story — it is epic-sized, with real architectural forks. `check-env-contract` (the gate 12.6 hardened) walks **only `crates/maos-bin/src/**`**. Every `MAOS_*` env read in the other 15 crates is ungoverned.

**Evidence (grounded, `eval` grep across `crates/**` diffed vs the maos-bin registry — no `cargo`, read-only env):**
- **47 distinct `MAOS_*` vars / 51 (var,crate) reads across 15 crates** outside maos-bin; **44 unregistered**. (The `12-7` backlog estimate of "~30" was low.)
- **`maos-kernel-core` reads env directly in ~18 sites, including production `src/`** — `security/operator_config.rs` (7 registry vars), `hot_swap/post_swap_monitor.rs` `MAOS_AUTO_REVERT_FAST` (the exact file Story 12.5 fixed), idle/schedule/supervision watchdogs, `revocation/poller.rs`, t3 sandbox.
- **Secret classification is a trap:** a naive `KEY`-substring filter misclassifies. `MAOS_REGISTRY_ORG_SIGNING_PUBKEY` / `MAOS_TRIAL_PRODUCER_PUBKEY` are **public** keys (not secret); the real secrets are `MAOS_ANTHROPIC_API_KEY`, `MAOS_OPENAI_API_KEY` (`maos-providers`), `MAOS_AUDIT_KEY` (`maos-domain`), `MAOS_TRIAL_PRODUCER_SEED` (`maos-eval`).
- **Test-timing seams (`*_FAST`/`_TEST`/`_SMOKE`) are read in production `src/`** (e.g. `MAOS_AUTO_REVERT_FAST` in the kernel monitor) — a `HarnessOnly`-in-production smell, not a simple `tests/` classification.
- **No shared env-contract crate exists** — `env_contract.rs` (`EnvVar` / `EnvStability` / `MAOS_ENV_REGISTRY`) lives only in `maos-bin`.

**Foundation already in place:** Story 12.6 landed (dev-implemented; 67 `maos-bin/src` vars registered, read-shape detector, `check-env-contract` enrolled as a blocking ship gate scoped to `maos-bin`). The workspace work builds directly on it.

---

## Section 2 — Impact Analysis

**Epic impact.** Epic 14 (*v2.2 Hardening + Closers*, `draft-ready-for-preflight`, stories 14.1–14.6) is the correct home: its remit is the **v2.0 remainder sweep + hardening**, and a workspace env-contract is exactly an operational-hardening closer (NFR-Maint alignment). No new epic; no other epic affected.

**Story impact.** `12-7` (backlog) is **promoted and decomposed into 3 new Epic-14 stories** (14.7 / 14.8 / 14.9) and removed as a standalone. The static-scan + ZERO-kernel-Δ finding collapses the preflight's "4 concerns" into 3 clean stories (kernel-core enrollment is not a separate kernel-risk story — see below). Story 12.6 is unaffected in scope; its forward-pointers to "Story 12.7" are re-homed to Epic 14.

**Artifact conflicts.**
- **PRD:** none. Zero new FRs; hardening of the existing `NFR-Maint` configuration-surface concern, already inside Epic 14's remit.
- **Architecture:** one real interaction to record — **14.7's shared registry placement touches 14.6's `kernel-crate-set.toml` (ADR-057 TCB ceiling).** If `maos-kernel-core` gains a *new* crate dependency to reach the registry, that crate counts toward the ≤25K ceiling. **Mitigation:** place the shared `EnvVar`/`EnvStability`/registry in **`maos-domain`** (already a `maos-kernel-core` dependency, already TCB-accounted) — or, if a dedicated leaf crate is chosen at preflight, explicitly account it in `kernel-crate-set.toml`. Captured as a 14.7 AC; no new ADR required.
- **CI/CD:** the workspace gate generalizes `check-env-contract` (14.7 renames/rescopes it, making the name workspace-true — the churn 12.6 deliberately deferred). `discipline.yml` + `gate-registry.toml` + `check-ship-gate-completeness` updated within 14.7/14.8.

**Technical impact.** All 3 stories are **ZERO kernel-Δ**: the gate is a **static scan**, and a registry that lives in `maos-domain` needs **no `maos-kernel-core` source edit** — the kernel's var *names* are registered elsewhere; nothing in the kernel imports the registry at runtime. (Current kernel baseline is **23141** post-12.5; these stories do not touch it.)

---

## Section 3 — Recommended Approach

**Option 1 — Direct Adjustment (add stories to the existing Epic 14 structure). SELECTED.**
- Effort: **Medium** (a new crate/module + a workspace gate + bulk registration/classification). Risk: **Low–Medium** (mechanical registration is low; the mechanism/migration and secret-handling carry the real review weight, isolated into their own stories).
- Rationale: Epic 14 already owns the hardening/remainder sweep; the work is additive and ZERO-kernel-Δ. No rollback, no MVP change.

**Option 2 — Rollback:** N/A (nothing to revert; 12.6 stays as the maos-bin foundation).

**Option 3 — PRD/MVP review:** N/A (zero new FRs; MVP unaffected).

**Why 3 stories, not the preflight's 4:** the ZERO-kernel-Δ static-scan finding removes "kernel-core enrollment" as a distinct risk — it folds into bulk registration (14.8). The remaining separation is by *review character*: risky mechanism/migration (14.7) · mechanical bulk registration (14.8) · security-sensitive secrets (14.9). Each ≤6 ACs, consistent with Epic 14's story sizing.

---

## Section 4 — Detailed Change Proposals

### 4.1 — New Epic-14 stories

**Story 14.7 — Workspace env-contract: shared registry + static-scan gate mechanism**
Extract `EnvVar` / `EnvStability` / registry into a shared home (**`maos-domain`** recommended — avoids a new `kernel-crate-set` member; final placement decided at preflight and, if a leaf crate, accounted in `kernel-crate-set.toml`). Migrate maos-bin's 12.6 registry + `check-env-contract` onto it. Generalize the gate to scan **all workspace crates** (rename to make the name workspace-true — the churn 12.6 deferred), preserving 12.6's read-shape detection (`env::var`/`var_os` literal + helper-indirected literal + `any_env_with_prefix` prefix-guard) with **per-crate proven-red** (the workspace-scale guard against the vacuous-green 12.6 killed). Enroll **advisory-first**. ZERO kernel-Δ. **Depends on 12.6.**

**Story 14.8 — Register + classify the full workspace env surface**
Register the ~43 non-secret `MAOS_*` reads across the 15 crates — **including `maos-kernel-core`'s ~18 via the shared registry, with NO kernel source edit**. Classify `HarnessOnly` vs `UserFacing` correctly; the `*_FAST`/`_TEST`/`_SMOKE` seams are `HarnessOnly` (flag the `HarnessOnly`-in-production reads, e.g. `MAOS_AUTO_REVERT_FAST`, as a documented smell, not a silent pass). Flip the workspace gate to **blocking**; per-crate proven-red holds. ZERO kernel-Δ. **Depends on 14.7.**

**Story 14.9 — Secret-var governance + provider keys**
Add a third `EnvStability::Secret` variant. Classify the real secrets — `MAOS_ANTHROPIC_API_KEY`, `MAOS_OPENAI_API_KEY`, `MAOS_AUDIT_KEY`, `MAOS_TRIAL_PRODUCER_SEED` — and correctly keep public keys (`*_PUBKEY`) NON-secret. The registry stores **name + purpose only, never the value**; a gate asserts `Secret`-classified vars are never logged/echoed/serialized. ZERO kernel-Δ. **§A6 security-sensitive (careful review). Depends on 14.7.**

**Sequencing:** 14.7 → (14.8 ∥ 14.9). All depend on 12.6. All ZERO kernel-Δ.

### 4.2 — Epic-14 document edits
Add 14.7/14.8/14.9 to the story-list table, per-story AC sketch, sequencing line, gate-discipline section, and kernel-delta budget (count 6 → 9). *(Applied with this proposal.)*

### 4.3 — sprint-status.yaml edits
Remove standalone `12-7-workspace-env-contract-and-secret-vars`; add `14-7-*` / `14-8-*` / `14-9-*` (backlog) after `14-6`; note the 12.6 → Epic-14 promotion; update the `epic-14` count 6 → 9 stories. *(Applied with this proposal.)*

### 4.4 — Story 12.6 doc
Re-home the "Out of Scope → Story 12.7" pointer to "Epic 14 (14.7–14.9)". Historical inline mentions left as narrative. Follow-up nit: the enrolled gate's runtime message references "Story 12.7" — updated when 14.7 renames the gate.

---

## Section 5 — Implementation Handoff

**Scope: Moderate → Product Owner / Developer.**
- **Applied now (this proposal):** Epic-14 doc + sprint-status reorganization + 12.6 pointer re-home.
- **Next (create-story, when Epic 14 reaches preflight):** context-engineer 14.7 → 14.8/14.9, each ≤6 ACs, per Epic 14's pre-dev checklist (name each gate's §A7 source; confirm ZERO kernel-Δ; §A6 net — 14.9 security-sensitive).
- **Dependency gate:** 14.7 requires 12.6 landed (met — 12.6 implemented; Task 4 verification pending).
- **Success criteria:** every `MAOS_*` workspace read registered + classified; workspace gate blocking with per-crate proven-red; secrets `Secret`-classified with a never-leak assertion; ZERO kernel-Δ (baseline 23141).

---

*Course-correction workflow (bmad-correct-course) — checklist sections 1–6 worked; change scope Moderate; routed to PO/DEV.*
