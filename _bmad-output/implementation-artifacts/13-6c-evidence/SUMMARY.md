# Story 13.6c — substrate blindness evidence (AC3 / T4)

Captured against the four-database substrate (`maos_team_{a,b,c}` + `maos_shared`)
on a local Postgres 17 + pgvector stand-in for CI's `pgvector/pgvector:pg16`
service. The two gates that declared region legs and ran with **no Postgres
service at all** now execute them.

## Before (no substrate — the blindness)

Both files (`before-cross-region-consensus.json`, `before-multi-region-slo.json`)
show the exact null-control the story catalogued:

- `postgres_available: false`
- the declared `three-region-convergence` leg (and the consensus live legs):
  `attempted: false, status: "skipped"`
- `oracle_green: false`, yet `passed: true` (exit 0) — because
  `current_phase: "v1_5"` makes a RED/skipped oracle **advisory**, not blocking.
  This is the D-5 mechanism: `LegResult::skipped` sets `green: false`; what
  yields exit 0 is `CURRENT_PHASE = "v1_5"`, **not** `passed: blockers.is_empty()`.

## After (substrate present — the legs now run)

`after-cross-region-consensus.json`:
- `postgres_available: true`; all four live legs `attempted: true, green: true,
  passed: 31`; `oracle_green: true`. The whole 33-`#[ignore] `cross_region_live`
  binary (incl. the new `three_team_databases_are_physically_distinct`) ran green.

`after-multi-region-slo.json`:
- `postgres_available: true`; **`three-region-convergence: attempted=true,
  green=true, passed:3`** — the headline declared-with-no-sensor leg now executes.
- `live-read-region-identity: green, passed:10` (was `red` without substrate).
- `roundtrip-slo: attempted=true, green=FALSE, failed=1` → **RED-at-HEAD finding**,
  see T6 below. `oracle_green: false`, `passed: true` (still advisory → exit 0).

## T5 — topology-fraud negatives hard-fail (proven)

- Region axis: pointing `MAOS_TEST_POSTGRES_A` and `_B` at the **same** database
  → `three_region_convergence_all_three_equal` panics
  `"region-a and region-b share a database — topology fraud (F2)"`. (exit 101)
- Team axis (new T2 reader): `TEAM_A` and `TEAM_B` on the same database →
  `three_team_databases_are_physically_distinct` panics
  `"team-a and team-b share a database — role-disjoint violation (AC1)"`. (exit 101)

## T6 — RED-at-HEAD triage (these legs are NON-BINDING at v1_5)

1. **`roundtrip-slo` p95 latency floor exceeded** (finding, owner: Story 11.2b /
   Epic-13 closer 13.6). First real execution ever: `p95=36099µs > floor
   MULTI_REGION_SLO_P95_US=30000µs`. The 30ms floor was set speculatively in
   11.2b against a substrate that **never existed** (D-3). Per the RED-at-HEAD
   contingency the floor was **NOT relaxed** (Trap #7). The gate is advisory at
   `v1_5` (exit 0, WOULD-HAVE-BLOCKED banner); CI's co-located PG16 container is
   the authoritative measurement. If CI also reds, the floor needs a deliberate
   re-calibration discussion — never a silent relax.
2. **`live-crossing-runs-through-two-daemons` / `tenant-mode-boots-live` /
   reza live legs** — fail on the LOCAL dev shell only (live `maos run` daemon
   boot; `maos_run_boot_loud_8_11`). These are 13.5d/13.6b legs, NOT 13.6c; they
   pass in CI (those stories are `done`) and the local failure is an
   environmental artifact (no daemon binary/config in this shell), **not a
   13.6c regression** — `live_substrate_present` was already true in CI via
   TEAM_A/B before this story, so these legs ran there regardless of TEAM_C.

**AC3 non-binding statement (D-5):** turning the region legs on does **not** make
them binding. Both gates are `CURRENT_PHASE = "v1_5"`; a red region leg still
exits 0 behind a WOULD-HAVE-BLOCKED banner. Neither gate carries the leg-level
`BindingClass` its siblings have — that is 13.6/13.6e judge machinery, explicitly
out of scope here (AC4 boundary).
