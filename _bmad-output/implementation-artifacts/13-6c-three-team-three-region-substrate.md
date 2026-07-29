---
baseline_commit: cb412348
depends_on: none — parallel enabler, no code dependency on 13.6a/13.6b
blocks: 13-6-reza-cortex-journey-closer-nfr-scale-5
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401. Work is CI workflow + test fixtures + env contract.
track: Track 2 (parallel) — authorized to start alongside 13.6a
---

# Story 13.6c — Two gates declare region legs, and neither job has a database

Status: **ready-for-dev**

**Kernel-core Δ: ZERO.** No crate source changes expected beyond test fixtures and `env_contract.rs`. Work lands in `.github/workflows/discipline.yml`, `crates/maos-bin/src/env_contract.rs`, and test fixtures.

> **Why this is a story and not a checklist item.** Story 13.6 is the Epic-13 closer and a **judge**: its AC6 says any required `ABSENT` or `INDETERMINATE` leg blocks the Reza/v2.2 claim. Every live leg it needs runs on a substrate that does not exist. This story is the long pole — real CI runtime, real service configuration — and it has **zero code dependency** on 13.6a or 13.6b, so it runs in parallel with them instead of behind them.
>
> It also pays for itself before 13.6 arrives: it turns on region coverage that has **never once executed**.

---

## Story

**As** the operator who has to believe the Reza verdict,
**I want** the three-team, three-region substrate to actually exist in CI,
**so that** the closer's live legs return a measured result instead of a banner, and the region legs two gates have been declaring since Story 11.2b start telling the truth.

---

## The gap, measured at `cb412348`

### D-1 — "3 teams × 3 regions" is **3 databases**, not 9. The manifest says so.

`validate_team_map` (`crates/maos-cohort/src/manifest.rs:652-666`) enforces uniqueness on **both** axes:

```rust
if !team_ids.insert(team_id.to_string()) { return Err(EDuplicateTeamId  { .. }); }
if !datnames.insert(team.datname.clone()) { return Err(EDuplicateTeamDatname { .. }); }
```

and `TeamEntry { team_id, region, datname, members }` (`:130`) carries **one** region and **one** datname. **A team cannot span regions in the signed manifest.** So the Reza topology is three teams, each pinned to its own region, each with its own database — and **one set of three databases serves both the tenant axis and the region axis.**

⇒ Provision `maos_team_a`, `maos_team_b`, `maos_team_c`; pin `team-a→region-a`, `team-b→region-b`, `team-c→region-c`.

### D-2 — CI provisions two databases, in two of the four jobs that need them

| Job | Postgres service | Databases | Env |
|---|---|---|---|
| `check-multi-tenant-loom` (`:2678`) | ✅ | `maos_team_a` + `CREATE DATABASE maos_team_b` (`:2706`) | `TEAM_A`, `TEAM_B`, `MAOS_TEST_POSTGRES` |
| `check-reza-production-path` (`:2715`) | ✅ | same, `:2743` | same |
| `check-cross-region-consensus` (`:2611`) | ❌ **none** | — | — |
| `check-multi-region-slo` (`:2632`) | ❌ **none** | — | — |

No `MAOS_TEST_POSTGRES_TEAM_C` exists anywhere in the repo. `MAOS_TEST_POSTGRES_{A,B,C}` — the three-region pilot's own variables (`crates/maos-loom-lite/tests/cross_region_live.rs:1877-1881`) — appear **nowhere under `.github/`**.

### D-3 — ⚠ Two gates declare region legs and run with no database at all

`check-multi-region-slo`'s own header (`discipline.yml:2626-2631`) enumerates its five legs, first among them **`three-region-convergence`**. The job body has **no `services:` block, no `env:`, no Postgres**. Same for `check-cross-region-consensus`.

So `three_region_convergence_all_three_equal` (`cross_region_live.rs:1997`) and its **topology-fraud negatives** — *"region-a and region-b share a database — topology fraud"* (`:2020-2024`), the one control a single stand-in cannot fake — **have never executed in CI since Story 11.2b shipped.**

This is not a substrate gap this story merely fills. It is a **declared leg with no sensor**, the same null-control shape this epic has catalogued twenty-six times. The story must prove the legs were dark, not just make them green.

---

## Acceptance Criteria (4)

### AC1 — Three databases, one substrate, both axes

**Given** the manifest binds each team to exactly one region and one datname (D-1),
**When** CI provisions the substrate,
**Then** three distinct databases exist — `maos_team_a`, `maos_team_b`, `maos_team_c` — with `team-N` pinned to `region-N` in the shared manifest fixture,
**And** both env namespaces resolve onto them: `MAOS_TEST_POSTGRES_TEAM_{A,B,C}` (tenant axis) and `MAOS_TEST_POSTGRES_{A,B,C}` (region axis),
**And** ⚠ **if the two axes turn out to interfere** — shared `collective_memory` table lifecycles across test binaries — the fallback is a **second three-database set** (`maos_region_{a,b,c}`), and the choice is **recorded with the observation that drove it**, never assumed either way at design time,
**And** the databases are proven **physically distinct** by `current_database()` per store, not by their names.

### AC2 — Every job that needs the substrate has it

**Given** four gates carry substrate-bound legs and only two have a service (D-2),
**When** this story lands,
**Then** `check-multi-tenant-loom` and `check-reza-production-path` are extended with `maos_team_c` + `MAOS_TEST_POSTGRES_TEAM_C`,
**And** `check-cross-region-consensus` and `check-multi-region-slo` each gain a Postgres service and the `MAOS_TEST_POSTGRES_{A,B,C}` env — the first time either has had one,
**And** the service definition is **single-sourced** (a reusable step or composite action), so a fifth job cannot be added later with a silently divergent substrate.

### AC3 — The dark legs run, and are proven to have been dark

**Given** `three-region-convergence` has been a declared leg with no sensor since 11.2b (D-3),
**When** the substrate lands,
**Then** `three_region_convergence_all_three_equal` and the **topology-fraud negatives** execute, and a fixture pointing two regions at one database **hard-fails**,
**And** ⚠ **the prior blindness is evidenced, not asserted**: capture the gate's own output before and after (leg attempted / not attempted), so the record shows a leg that changed from unmeasured to measured — *"we turned it on"* is a claim; the before/after is the control,
**And** any leg that reds on first real execution is a **FINDING**, handled by the RED-at-HEAD contingency: validate the harness first, then fix or hold advisory with a loud banner, owner, and tracking entry. **Never re-can a fixture, never silently relax a floor.**

### AC4 — Absent substrate is loud, never green

**Given** the house `AdvisorySubstrate` pattern still emits `passed: true` when substrate is missing (deferred-work 2026-07-25),
**When** a variable is unset,
**Then** every live leg `.expect()`s **its own** variable and fails loudly — skipped ≠ passed (the 13.5g pattern),
**And** all new variables are registered in `crates/maos-bin/src/env_contract.rs` so `check-env-contract` covers them,
**And** ⚠ this story does **not** attempt the cross-gate fix for `passed: blockers.is_empty()` — that belongs to **13.6's evidence ledger** (its AC5). Name the boundary here so the two do not collide.

---

## Traps

1. **Do not provision nine databases.** The manifest cannot express a team spanning regions (D-1). Nine would be a topology the product does not have.
2. **`three-region-convergence` is a declared leg with no sensor.** Turning the substrate on may red it for the first time. That is a **finding**, not a blocker to route around — and it is the most likely outcome of this story.
3. **Single-source the service block** (AC2). Four jobs with four hand-copied Postgres definitions is how substrates drift.
4. **Do not fix the `AdvisorySubstrate` vacuous-pass here** — 13.6 owns it. Overlapping edits on the same gate files across parallel tracks is the collision risk this story carries.
5. **Watch CI runtime.** Four jobs × a Postgres service is real wall-clock. Measure it and report the delta; if it is material, say so rather than letting the pipeline quietly get slower.
6. **`cargo run -q -p xtask -- <cmd>`.** No `cargo xtask` alias.

---

## Tasks

- [ ] **T1 (AC1)** — Decide and record the one-set-vs-two-set question by **observation**: run the region pilot and the tenant legs against one shared set, look for interference, record the result either way.
- [ ] **T2 (AC2)** — Single-sourced Postgres service + `CREATE DATABASE` step; wire into all four jobs; add `maos_team_c` and both env namespaces.
- [ ] **T3 (AC3)** — Capture gate output **before** the change (legs unattempted) as the blindness evidence; land the substrate; capture after.
- [ ] **T4 (AC3)** — Execute the 3-region pilot legs and the topology-fraud negatives; prove the shared-database fixture hard-fails.
- [ ] **T5 (AC3)** — Triage anything that reds on first real execution under the RED-at-HEAD contingency; record findings with owners.
- [ ] **T6 (AC4)** — `env_contract.rs` registration; `.expect()` on every live leg; confirm `check-env-contract` green.
- [ ] **T7** — Measure and report the CI wall-clock delta. Run `check-kernel-baseline` (**23401**), `kloc-check`, `cargo fmt --all -- --check`, and the four touched gates.
- [ ] **T8** — Record the dev model (`check-dev-model-used-populated` is live).

---

## Dev Notes

| Need | Where |
|---|---|
| Existing 2-datname provisioning to copy | `.github/workflows/discipline.yml:2682-2711` (multi-tenant-loom), `:2719-2768` (reza) |
| Jobs missing a service entirely | `:2611` cross-region-consensus, `:2632` multi-region-slo |
| Region pilot env reader | `crates/maos-loom-lite/tests/cross_region_live.rs:1877-1890` (`pg_conn_for`) |
| Topology-fraud negatives | `cross_region_live.rs:2020-2024` |
| Team/datname uniqueness | `crates/maos-cohort/src/manifest.rs:652-666` |
| Env contract registry | `crates/maos-bin/src/env_contract.rs` (see `:290` for the `MAOS_CROSS_TEAM_BASE_SEED` entry shape) |

**Budget.** kernel-core ZERO @ **23401**; fkcs frozen `23081`; no new crate, no new dependency.

**Parallelism.** Runs alongside 13.6a. The only overlap risk is gate-file edits — 13.6a touches `check_multi_tenant_loom.rs` leg *definitions*, this story touches the *workflow*. Coordinate on `discipline.yml` if 13.6a needs a job change.

### References

- [Source: `epics/epic-13-reza-cortex-v2-2.md#73`] — *"the smallest real wall is a CI-provisioned Postgres service with two distinct datnames … the full rung is the 3-team × 3-region Reza scene."*
- [Source: `epics/epic-13-reza-cortex-v2-2.md#180`] — evidence state vs enforcement class.
- [Source: `_bmad-output/implementation-artifacts/11-3-scale-envelope-25-30-host-churn.md`] — RED-at-HEAD contingency wording to reuse.

---

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created as Track 2 of the Epic-13 completion plan — parallel enabler, no code dependency on 13.6a/13.6b. Grounding corrected the scope **down** from 9 databases to 3 (the manifest cannot express a team spanning regions), and **up** in severity: `check-multi-region-slo` declares `three-region-convergence` as a leg and runs with **no Postgres service at all**, so that leg and its topology-fraud negative have never executed since 11.2b. 4 ACs, ZERO kernel Δ. |
