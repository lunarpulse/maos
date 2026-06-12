---
epic: epic-9
epic_title: "Audit & Compliance Surfaces + Operator Productionization (v0.5 → v1.0)"
dev_model_used: claude-opus-4-8  # RECOMMENDED (stub; set to actual on done) — scheduler/runtime + async-invariant work; §A6 correctness-critical
---

# Story 9.6: Multi-Spirit Scheduler + Founder-Class Standalone Load

**Status:** ready-for-dev (STUB — authored by Story 8.16 AC8 per Epic-8 retro §A2; full spec at Epic-9 sprint planning)

**Type:** NEW Epic-9 story. The Epic-8 retrospective (`epic-8-retro-2026-06-12.md`, action §A2; `[[project_epic_8_retro_outcomes]]`) surfaced the multi-Spirit scheduler / founder-class-standalone-load gap as the single biggest unresolved item — it gates J1/J4 Grade-A and the multi-tool orchestration that operator-productionization (9.4) implicitly assumes — yet it was **homeless** (Epic 9's original stub is audit/compliance/operator only). Lunarpulse ratified at retro: **make it Story 9.6, sequence the value now.** This stub stages it; the full AC set is written at Epic-9 sprint planning.

## Story

As **an operator running MAOS in production**,
I want **`maos run` to load and schedule MORE than one Spirit — including founder-class `[class]` Spirits that today short-circuit at admission (8.12 FORK B: `classify_spirit` → `FounderLoopClass` directional error, not standalone-loadable)**,
so that **the founder-loop topology (Orchestrator + Workers + Architect + Reviewer) and the Mira↔Nash pair run as first-class `maos run` daemons, J1/J4 upgrade from Grade-B smoke wraps to Grade-A end-to-end journeys, and the J1 resume-continuity beat (authored RED in 8.15, D4) auto-activates**.

## Scope sketch (to be expanded at sprint planning)

- **Close the `classify_spirit` founder-class short-circuit** so `[class]` Spirits are standalone-loadable under `maos run` (per `[[project_story_8_12_founder_class_gap]]`: founder spirits lack a caps section + scalar port today; `classify→FounderLoopClass` short-circuits with the FORK-B directional error). Flag Winston/John on the caps-section + scalar-port surface.
- **Multi-Spirit scheduling** in the daemon composition root — more than one Spirit registered + scheduled concurrently (today `maos run` is effectively single-Spirit; `spirit_pid = 0` is hardcoded across Butler/Researcher per the 8.14b/c tech-debt note — promote to per-Spirit pid assignment here).
- **Upgrade J1/J4 to Grade-A**: the founder-loop + Mira-Nash journeys run via real `maos run` daemons, not the 8.15 Grade-B smoke wraps. The 8.15 **J1 resume-continuity beat** (D4, authored RED, "director halts Orchestrator → resume → digest cites pre-halt refs") **auto-activates** on closure.
- **(Folded from Epic-7 §A3, verified open by Story 8.16 AC6)** durable **skill-queue persistence** + functional **`maosctl skills approve/reject`** (today in-memory `Vec` + acknowledgement-only stubs — `maos-skill/src/admission.rs:38-40`, `maos-cli/src/subcommands.rs:73-82`). EITHER land here with the scheduler/runtime work, OR split to a dedicated skill-queue story at sprint planning. See `deferred-work.md` "Epic-7 §A3 ... 2 of 4 items still OPEN".

## Dependencies / sequencing

- DEPENDS ON the live runtime spine (8.11) + CliWrapper bridge (8.12) — both landed.
- **Sequence before/around Story 9.4** (operator productionization) — 9.4's multi-tenant/distribution surface implicitly assumes a runtime that can run multiple Spirits; 9.6 makes that real.
- UNBLOCKS J1/J4 Grade-A; closes the 8.12/8.14c/8.15 founder-class carry-forward.

## Notes

- Likely a **charter kernel delta** (scheduler lives in `maos-kernel-core`) — re-pin the kernel baseline in `xtask/kernel-core-baseline.toml` (Story 8.16 §A4 single source of truth) alongside an authorized delta + FLAG-Winston; do NOT edit the count anywhere else.
- Recommended dev model `claude-opus-4-8`; if a non-Opus model is used, §A6 mandatory safety-net applies (scheduler = async-invariant + correctness-critical).

## Acceptance Criteria

_TBD at Epic-9 sprint planning (this is a retro-staged stub; the AC set is written when 9.6 is picked up)._

## Dev Agent Record

### Agent Model Used

### Completion Notes List

### File List

### Change Log
