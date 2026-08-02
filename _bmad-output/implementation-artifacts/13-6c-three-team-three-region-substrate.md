---
baseline_commit: b400d127
depends_on: none — parallel enabler, no code dependency on 13.6a/13.6b/13.6d (re-verified at b400d127: the only diff to `discipline.yml` since cb412348 is a six-line comment block)
blocks: 13-6-reza-cortex-journey-closer-nfr-scale-5
kernel_grant: NONE — ZERO maos-kernel-core Δ, pin stays 23401 (re-verified at b400d127: `check-kernel-baseline` PASSED, 23401 == 23401). Work is CI workflow + test fixtures + two new drift controls.
track: Track 2 (parallel) — authorized to start alongside 13.6a
---

# Story 13.6c — Two gates declare region legs, and neither job has a database

Status: **in-progress**

**Kernel-core Δ: ZERO.** Work lands in `.github/workflows/discipline.yml`, `xtask/src/` (two new drift controls + one new leg), `crates/maos-loom-lite/tests/cross_region_live.rs` (one helper widened, one reader added), and a new `.github/actions/` composite action.

> **Why this is a story and not a checklist item.** Story 13.6 is the Epic-13 closer and a **judge**: its AC6 says any required `ABSENT` or `INDETERMINATE` leg blocks the Reza/v2.2 claim. Every live leg it needs runs on a substrate that does not exist. This story is the long pole — real CI runtime, real service configuration — and it has **zero code dependency** on 13.6a/13.6b/13.6d, so it runs in parallel with them instead of behind them.
>
> It also pays for itself before 13.6 arrives: it turns on region coverage that has **never once executed**.

---

## The gap, measured at `b400d127`

> **Preflight note (2026-08-01).** Every line cite below was re-derived at `b400d127`. The story's original cites were 75–100 lines stale in `manifest.rs` (13.6a inserted `team_of_host` above `validate_team_map`) and 2 lines stale in `discipline.yml`. The **substance** of the original D-1/D-2/D-3 survived verification unchanged; only the coordinates were wrong. Five further defects were found in the story's own ACs and are folded in as D-4 … D-8.

### D-1 — "3 teams × 3 regions" is **3 databases**, not 9. The manifest says so.

`validate_team_map` (`crates/maos-cohort/src/manifest.rs:727-747`) enforces uniqueness on **both** axes:

```rust
if !team_ids.insert(team_id.to_string()) { return Err(EDuplicateTeamId  { .. }); }   // :738
if !datnames.insert(team.datname.clone()) { return Err(EDuplicateTeamDatname { .. }); } // :743
```

and `TeamEntry { team_id, region, datname, members }` (`:154-159`) carries **one** region and **one** datname. **A team cannot span regions in the signed manifest.** So the Reza topology is three teams, each with its own database, and the distinct `(team, region)` pair count is capped at the team count — **three**, not nine.

⚠ **Corrected reasoning.** `region` is **not** in the uniqueness set — only `team_id` and `datname` are. Two teams *may* legally share a region. "team-N pinned to region-N" is therefore a **fixture choice this story makes**, not a manifest constraint. State it that way, or a later reader will "discover" the manifest doesn't enforce it and file a phantom bug.

⇒ Provision `maos_team_a`, `maos_team_b`, `maos_team_c`; pin `team-a→region-a`, `team-b→region-b`, `team-c→region-c` **in the fixture**.

### D-2 — CI provisions two databases, in two of the four jobs that need them

| Job | Line | Postgres service | Databases | Env |
|---|---|---|---|---|
| `check-multi-tenant-loom` | `:2680` | ✅ `:2683` | `maos_team_a` (`:2689`) + `CREATE DATABASE maos_team_b` (`:2708`) | `TEAM_A` `:2711`, `TEAM_B` `:2712`, `MAOS_TEST_POSTGRES` `:2713` |
| `check-reza-production-path` | `:2717` | ✅ `:2720` | same (`:2726`, `:2745`) | `TEAM_A` `:2769`, `TEAM_B` `:2770` |
| `check-cross-region-consensus` | `:2611` | ❌ **none** | — | — |
| `check-multi-region-slo` | `:2632` | ❌ **none** | — | — |

`MAOS_TEST_POSTGRES_TEAM_C` exists nowhere in the repo. `MAOS_TEST_POSTGRES_{A,B,C}` — the three-region pilot's own variables (`cross_region_live.rs:1877-1890`) — appear **nowhere under `.github/`**.

### D-3 — ⚠ Two gates declare region legs and run with no database at all

`check-multi-region-slo`'s own header (`discipline.yml:2626-2631`) enumerates its five legs, first among them **`three-region-convergence`**. The job body has **no `services:` block, no `env:`, no Postgres**. Same for `check-cross-region-consensus`.

So `three_region_convergence_all_three_equal` (`cross_region_live.rs:1997`) and its **topology-fraud negatives** — *"region-a and region-b share a database — topology fraud"* (`:2010-2026`), the one control a single stand-in cannot fake — **have never executed in CI since Story 11.2b shipped.**

This is not a substrate gap this story merely fills. It is a **declared leg with no sensor**, the same null-control shape this epic has catalogued twenty-seven times. The story must prove the legs were dark, not just make them green.

### D-4 — ⚠ SHIP-BLOCKER: the consensus gate keys on a **different variable**, and its oracle runs the **whole binary**

Two facts the original AC2 did not know:

1. **`check-cross-region-consensus` probes `MAOS_TEST_POSTGRES` — singular** (`xtask/src/check_cross_region_consensus.rs:308`). Give that job a Postgres service and `MAOS_TEST_POSTGRES_{A,B,C}` and **the gate stays skipped**: service running, four legs dark, CI minutes paid, nothing measured.

2. **Its oracle is unfiltered.** `run_live_oracle` (`:160-172`) runs:

   ```
   cargo test --locked -p maos-loom-lite --test cross_region_live -- --ignored --nocapture
   ```

   No test filter. `grep -c '#\[ignore' cross_region_live.rs` → **32**. The one result is then **broadcast to all four legs** (`LIVE_LEGS`, `:110-116`), so a single failing test in that file reds every leg.

   And every env reader in that file **panics** rather than skipping: `pg_conn` `:67-70`, `pg_conn_team` `:76-82` (`panic!("unknown team {other}")`), `pg_conn_for` `:1877-1890`. So setting `MAOS_TEST_POSTGRES` *alone* makes the 3-region and cross-team tests panic and reds all four legs.

⇒ **The consensus job requires the full env union**, and the required union must be derived from the readers, not guessed. See AC2's table.

⇒ **Declared coupling (residual, not fixed here).** `cross_region_live.rs` has accreted into the shared oracle for **two gates and nine legs across three stories** (11.2a, 11.2b, 13.3), and no story decided that. This story does **not** narrow the 11.2a broadcast design — that is another story's oracle. It declares the coupling, provisions the union so the gate can never red for an *env* reason, and measures the double-execution cost (T7).

### D-5 — ⚠ AC4 named the wrong mechanism; a red region leg will **still exit 0** after this story

The original AC4 blamed the `AdvisorySubstrate` `passed: blockers.is_empty()` pattern. That is **not** what these two gates do:

- `LegResult::skipped` sets **`green: false`** — `check_multi_region_slo.rs:122-131`, `check_cross_region_consensus.rs:135`. There is a unit test asserting it: `skipped_leg_is_not_green_not_attempted` (`check_cross_region_consensus.rs:491`). `oracle_green` is honestly `false` today.
- What makes them exit 0 is **`const CURRENT_PHASE: &str = "v1_5"`** (`check_multi_region_slo.rs:54`, `check_cross_region_consensus.rs:50`). Oracle red → not blocking at `v1_5` → WOULD-HAVE-BLOCKED banner → `"passed": true` → **exit 0**.

**Consequence this story must state plainly:** after the substrate lands, if `three_region_convergence_all_three_equal` REDs, **the gate still exits 0**. Turning the legs on does **not** make them binding. Neither gate carries the leg-level `BindingClass` its siblings have (`check_multi_tenant_loom.rs`, `check_reza_production_path.rs`) — they are still whole-gate phase-coupled, which is the E12-B1 gate-binding-decay item.

⇒ **Boundary: this story does not add `BindingClass` to these two gates.** That is judge machinery and belongs with 13.6/13.6e. This story names the mechanism correctly so the successor inherits a real boundary instead of one drawn around a thing that isn't there.

### D-6 — ⚠ The original AC4's `env_contract.rs` clause was **null control #28**

`check-env-contract` resolves its registry at `maos-bin/src/env_contract.rs` (`xtask/src/check_env_contract.rs:89`) and walks **`maos_bin_dir/src` only** (`:118-120`). Every `MAOS_TEST_POSTGRES*` read lives in `crates/maos-loom-lite/tests/`, `crates/maos-bench/tests/`, and `xtask/src/` — **outside the walk**. The gate's own success message says so: *"workspace coverage tracked in Story 12.7"* (`:175`). There is no reverse orphan-registration check, so a registration would be inert in both directions.

It is also **actively misleading**: `env_contract.rs` is the **operator-facing** contract (`EnvStability::UserFacing`, e.g. `MAOS_LOOM_POSTGRES` at `:285`). Putting a CI fixture variable there tells a human they may need to set it. They never will.

⇒ AC4's registration clause is **replaced** by a control that answers the question that actually bites (AC5).

### D-7 — ⚠ `MAOS_TEST_POSTGRES_TEAM_C` has **zero consumers**; the original AC2 would provision a database with no reader

`check_multi_tenant_loom.rs:61` and `check_reza_production_path.rs:52` each require exactly `["MAOS_TEST_POSTGRES_TEAM_A", "MAOS_TEST_POSTGRES_TEAM_B"]`. `pg_conn_team` (`cross_region_live.rs:76-82`) matches `"team-a"` / `"team-b"` and `panic!`s on anything else. The SLO legs never touch the team axis — they all go through `make_store_for` (`:2374`, `:2439`, `:2506`, `:2637`) and maos-bench's own `'a'/'b'` mapping (`t_11_2b_cross_region_slo.rs:55-56`).

Provisioning `TEAM_C` with no reader would be a **substrate with no sensor** — this story's own failure mode, committed by this story. Resolution: **`TEAM_C` lands with a minimal reader, and only in the gate that has one** (AC1/AC2). `check-reza-production-path` gets **nothing** — its 13.5d legs are a two-team mediated route and will not grow a third.

### D-8 — ⚠ AC2's "reusable step or composite action" is **not buildable as written**

- `.github/actions` does not exist; `grep -l workflow_call .github/workflows/*.yml` returns nothing. Net-new mechanism, first of its kind in this repo.
- **A composite action cannot define `services:`.** `services` is a job-level key; composite actions run steps only.
- GitHub Actions' workflow parser does **not** support YAML anchors.

⇒ Single-sourcing the *service block* would require a `workflow_call` reusable workflow restructuring four jobs. **Rejected as disproportionate.** What *is* single-sourceable is the step half (`CREATE DATABASE` + env export) as a composite action; the `services:` block stays duplicated and is held by a **drift control** instead of a promise (AC5).

### D-9 — Two smaller facts

- **No `timeout-minutes`** on `check-cross-region-consensus` (`:2611`) or `check-multi-region-slo` (`:2632`). 49 of 60 `check-` jobs have one; the two siblings already running live Postgres are at **20** and **15**. Adding a database and a live suite to an untimed job is a six-hour hang waiting for its first flake.
- **✅ The blindness evidence AC3 wants is already machine-readable.** Both gates emit `"postgres_available"` and per-leg `attempted` / `status` in `--json` (`check_multi_region_slo.rs:484-535`, `check_cross_region_consensus.rs:308-380`). T3 is a **JSON diff**, not a narrative capture.

---

## Acceptance Criteria (5)

### AC1 — Three databases, one substrate, both axes — and `TEAM_C` lands with a reader

**Given** the manifest binds each team to exactly one region and one datname (D-1),
**When** CI provisions the substrate,
**Then** three distinct databases exist — `maos_team_a`, `maos_team_b`, `maos_team_c` — with `team-N` pinned to `region-N` **as a fixture choice**, recorded as such (D-1: `region` is not a uniqueness axis; the manifest permits shared regions),
**And** both env namespaces resolve onto the **same three databases**: `MAOS_TEST_POSTGRES_TEAM_{A,B,C}` (tenant axis) and `MAOS_TEST_POSTGRES_{A,B,C}` (region axis) — one substrate mirroring the product topology,
**And** ⚠ **the one-set-vs-two-set question is resolved by construction, not by observation**: within any single job, **no two variables may resolve to the same database across different roles**. `MAOS_TEST_POSTGRES` is the *shared-table stand-in* the 3-region pilot explicitly rejects, and it is **already aliased onto `maos_team_b`** at `discipline.yml:2713`. In the two jobs gaining a service it therefore gets its **own** `maos_shared` database. Every reset is table-scoped `DELETE FROM collective_memory` (`:232`, `:240`, `:1932`), so an alias means one axis's reset wipes another axis's fixture. **Existing green jobs keep their current aliasing — do not perturb what passes.**
**And** ⚠ **`MAOS_TEST_POSTGRES_TEAM_C` lands with its reader, in the one gate that has one** (D-7): widen `pg_conn_team` (`cross_region_live.rs:76-82`) to accept `"team-c"`; add one `#[ignore]` test asserting **three distinct `current_database()`** across the three team databases; add it as a leg on `check-multi-tenant-loom`; extend `live_substrate_present()` (`check_multi_tenant_loom.rs:61`) to require `TEAM_C` so the leg cannot be silently skipped. **`check-reza-production-path` is unchanged** — it has no three-team leg and will not grow one,
**And** the databases are proven **physically distinct** by `current_database()` per store on **both** axes, never by their names.

### AC2 — Every job that needs the substrate has it, with its **complete** env union

**Given** four gates carry substrate-bound legs, only two have a service, and the two without key on **different variables than the story assumed** (D-2, D-4),
**When** this story lands,
**Then** each gate's env is the **union its own oracle actually reads**, derived from the reader side and recorded in this table:

| Gate | Oracle scope | Required env | Databases |
|---|---|---|---|
| `check-cross-region-consensus` | **whole** `cross_region_live` binary, `--ignored` (32 tests, broadcast to 4 legs) | `MAOS_TEST_POSTGRES`, `MAOS_TEST_POSTGRES_{A,B,C}`, `MAOS_TEST_POSTGRES_TEAM_{A,B}` | `maos_team_{a,b,c}` + `maos_shared` |
| `check-multi-region-slo` | filtered — `three_region`, `cross_region_roundtrip_live`, `live_read_region_identity`, `read_path_chokepoint`; all via `make_store_for` / maos-bench `'a'`/`'b'` | `MAOS_TEST_POSTGRES_{A,B,C}` | `maos_team_{a,b,c}` |
| `check-multi-tenant-loom` | existing legs **+ AC1's three-team reader** | `MAOS_TEST_POSTGRES_TEAM_{A,B,C}`, `MAOS_TEST_POSTGRES` *(existing alias unchanged)* | `maos_team_{a,b,c}` |
| `check-reza-production-path` | 13.5d mediated two-team route | `MAOS_TEST_POSTGRES_TEAM_{A,B}` — **unchanged** | `maos_team_{a,b}` — **unchanged** |

**And** `check-cross-region-consensus` and `check-multi-region-slo` each gain a Postgres service — the first time either has had one — **and a `timeout-minutes`** matching their siblings' 15–20 (D-9),
**And** ⚠ the **step half** of provisioning (`CREATE DATABASE` + env export) is single-sourced as a composite action under `.github/actions/`; the `services:` block is **not** single-sourceable (D-8: composite actions cannot define `services:`, Actions has no YAML anchors, and `workflow_call` would restructure four jobs) and is instead held by AC5's drift control. **Do not write an AC that promises a mechanism that cannot exist.**

### AC3 — The dark legs run, and are proven to have been dark

**Given** `three-region-convergence` has been a declared leg with no sensor since 11.2b (D-3),
**When** the substrate lands,
**Then** `three_region_convergence_all_three_equal` and the **topology-fraud negatives** (`cross_region_live.rs:2010-2026`) execute, and a fixture pointing two regions at one database **hard-fails**,
**And** ⚠ **the prior blindness is evidenced mechanically, not asserted**: capture each gate's own `--json` **before** and **after** and diff the `legs` array — `postgres_available: false → true`, per-leg `attempted: false → true` (D-9). *"We turned it on"* is a claim; the JSON field that flips is the control,
**And** ⚠ **turning the legs on does NOT make them binding** (D-5). Both gates are `CURRENT_PHASE = "v1_5"`, so a red region leg still exits 0 behind a WOULD-HAVE-BLOCKED banner. The Dev Agent Record must say this in words, so the closer does not read *"region legs now execute"* as `PROVEN`,
**And** any leg that reds on first real execution is a **FINDING**, handled by the RED-at-HEAD contingency: validate the harness first, then fix or hold advisory with a loud banner, owner, and tracking entry. **Never re-can a fixture, never silently relax a floor.**

### AC4 — Absent substrate is loud, never green — and the mechanism is named correctly

**Given** the story's original premise was wrong about *why* these gates go green while dark (D-5),
**When** a variable is unset,
**Then** every live leg `.expect()`s **its own** variable and fails loudly — skipped ≠ passed (the 13.5g pattern); the existing readers already do this (`pg_conn` `:67-70`, `pg_conn_team` `:76-82`, `pg_conn_for` `:1877-1890`) and the widened `"team-c"` arm must too,
**And** the story records the correct mechanism: `LegResult::skipped` sets **`green: false`** (`check_multi_region_slo.rs:122-131`); what yields exit 0 is **`CURRENT_PHASE = "v1_5"`**, **not** `passed: blockers.is_empty()`,
**And** ⚠ **the `env_contract.rs` registration clause is DELETED, not deferred** (D-6). `check-env-contract` walks `maos-bin/src` only (`check_env_contract.rs:89,118-120`); it *cannot* cover these variables, and `env_contract.rs` is the operator-facing contract, not a CI fixture registry. Its replacement is AC5,
**And** ⚠ this story does **not** attempt the leg-level `BindingClass` fix for these two gates, nor the cross-gate `passed: blockers.is_empty()` fix — both belong to **13.6's evidence ledger / 13.6e's judge machinery**. Name the boundary here so the two do not collide.

### AC5 — Two drift controls, written as controls rather than promises

**Given** AC2's env unions and AC2's un-single-sourceable service blocks are both things a future edit can silently break (D-4, D-8),
**When** this story lands,
**Then** a **workflow-env ⟷ reader-var consistency check** exists as a gate: parse each of the four jobs' `env:` keys, parse the `MAOS_TEST_POSTGRES*` reads reachable from each gate's oracle (`cross_region_live.rs`, `t_11_2b_cross_region_slo.rs`, the four `xtask/src/check_*.rs` probes), and **fail on any variable a gate's oracle reads that its job does not export** — the control that would have caught D-4 on its own,
**And** a **service-block drift check** exists: the Postgres `services:` definitions across the four jobs must be byte-identical modulo `POSTGRES_DB`, so a fifth job cannot be added later with a silently divergent substrate,
**And** AC2's env table is the **expected output** of the first check — if the table and the check disagree, CI says which is stale,
**And** both checks are added to the ship gate's `needs:` and follow the house `--json` shape.

---

## Traps

1. **Do not provision nine databases.** The manifest cannot express a team spanning regions (D-1). Nine would be a topology the product does not have.
2. **⚠ Do not give `check-cross-region-consensus` the region variables and call it done.** It keys on `MAOS_TEST_POSTGRES` (`:308`) and runs the **whole 32-test binary** (`:160-172`). Partial env → panics → all four legs red. The union in AC2's table is not advisory.
3. **⚠ Do not narrow the 11.2a broadcast oracle to save wall-clock.** It is another story's gate. Declare the coupling, measure the cost, file it if material (D-4).
4. **⚠ Do not add `maos_team_c` to `check-reza-production-path`.** Nothing there reads it (D-7). Provisioning without a reader is the exact defect this story exists to kill.
5. **⚠ Do not register test variables in `env_contract.rs`.** The gate cannot see them and the file is operator-facing (D-6). AC5 is the replacement.
6. **⚠ Do not promise a composite action for the `services:` block.** It is structurally impossible (D-8).
7. **`three-region-convergence` is a declared leg with no sensor.** Turning the substrate on may red it for the first time. That is a **finding**, not a blocker to route around — and it is the most likely outcome of this story.
8. **Do not add `BindingClass` or fix the `AdvisorySubstrate` vacuous-pass here** — 13.6/13.6e own them. Overlapping edits on the same gate files across parallel tracks is the collision risk this story carries.
9. **Watch CI runtime.** Two new Postgres services plus a 32-test unfiltered binary plus a double-executed 3-region suite is real wall-clock. Measure it and report the delta rather than letting the pipeline quietly get slower.
10. **Re-derive line cites before quoting them.** Every cite in the original story drifted (D-preflight note). `cargo run -q -p xtask -- <cmd>` — no `cargo xtask` alias.

---

## Tasks

- [x] **T1 (AC1)** — Provision `maos_team_{a,b,c}` + `maos_shared`; apply the role-disjoint rule (no two variables onto one database within a job); leave existing jobs' aliasing untouched. Record the fixture choice `team-N→region-N` as a fixture choice.
- [x] **T2 (AC1/AC2)** — Widen `pg_conn_team` for `"team-c"`; add the three-distinct-`current_database()` reader test; add the leg to `check-multi-tenant-loom`; extend `live_substrate_present()` (`:61`) with `TEAM_C`.
- [x] **T3 (AC2)** — Composite action under `.github/actions/` for `CREATE DATABASE` + env export; wire the four jobs to AC2's table exactly; add `timeout-minutes` to the two new-service jobs. **`check-reza-production-path` unchanged.**
- [x] **T4 (AC3)** — Capture both gates' `--json` **before** the change (`postgres_available: false`, per-leg `attempted: false`) as the blindness evidence; land the substrate; capture after; commit the diff.
- [x] **T5 (AC3)** — Execute the 3-region pilot legs and the topology-fraud negatives; prove the shared-database fixture hard-fails.
- [x] **T6 (AC3)** — Triage anything that reds on first real execution under the RED-at-HEAD contingency; record findings with owners. **State in the Dev Agent Record that these legs are still non-binding at `v1_5`.**
- [x] **T7 (AC5)** — Build the workflow-env ⟷ reader-var consistency check and the service-block drift check; register both in the ship gate `needs:`.
- [x] **T8 (AC4)** — Confirm every live reader `.expect()`s its own variable including the new `"team-c"` arm. **No `env_contract.rs` edit.**
- [ ] **T9** — Measure and report the CI wall-clock delta (two new services + the double-executed 3-region suite). Run `check-kernel-baseline` (**23401**), `kloc-check`, `cargo fmt --all -- --check`, `check-env-contract` (must stay at its exact pre-existing Story-12.7 violation count), and the four touched gates. **Pending:** the real CI delta cannot be recorded until this uncommitted change runs on a GitHub-hosted runner.
- [x] **T10** — Record the dev model (`check-dev-model-used-populated` is live).

### Review Findings

- [x] [Review][Patch] Pass the database list through an environment variable [.github/actions/provision-loom-substrate/action.yml:36]
- [x] [Review][Patch] Scope exported variables to the gate-running step [xtask/src/check_loom_substrate_drift.rs:204]
- [x] [Review][Patch] Derive reader variables from each gate's reachable oracle [xtask/src/check_loom_substrate_drift.rs:268]
- [x] [Review][Patch] Discover substrate jobs instead of hard-coding four [xtask/src/check_loom_substrate_drift.rs:48]
- [x] [Review][Patch] Register the drift gate with ship-gate completeness [xtask/src/check_ship_gate_completeness.rs:16]
- [ ] [Review][Patch] Record the required CI wall-clock measurement [_bmad-output/implementation-artifacts/13-6c-three-team-three-region-substrate.md:201]

---

## Dev Notes

| Need | Where (verified @ `b400d127`) |
|---|---|
| Existing 2-datname provisioning to copy | `discipline.yml:2683-2713` (multi-tenant-loom), `:2720-2770` (reza) |
| Jobs missing a service entirely | `:2611` cross-region-consensus, `:2632` multi-region-slo |
| Consensus substrate probe (**singular var**) | `xtask/src/check_cross_region_consensus.rs:308` |
| Consensus unfiltered oracle + 4-leg broadcast | `check_cross_region_consensus.rs:160-172`, `:110-116` |
| SLO substrate probe (A/B/C) | `xtask/src/check_multi_region_slo.rs:147-151` |
| Phase coupling that yields exit 0 while dark | `check_multi_region_slo.rs:54`, `check_cross_region_consensus.rs:50` |
| `skipped` is honestly not-green (+ its unit test) | `check_multi_region_slo.rs:122-131`; `check_cross_region_consensus.rs:135`, `:491` |
| Region pilot env reader (panics, never skips) | `cross_region_live.rs:1877-1890` (`pg_conn_for`) |
| Team env reader to widen for `"team-c"` | `cross_region_live.rs:76-82` (`pg_conn_team`) |
| Shared-table reader (the `maos_shared` role) | `cross_region_live.rs:67-70` (`pg_conn`), `:105` (`make_store`) |
| Table-scoped resets (why aliasing is unsafe) | `cross_region_live.rs:232`, `:240`, `:1932` |
| Topology-fraud negatives | `cross_region_live.rs:2010-2026` |
| Three-region tests (all three run under the `three_region` filter) | `cross_region_live.rs:1997`, `:2202`, `:2317` |
| maos-bench SLO reader (A/B only) | `crates/maos-bench/tests/t_11_2b_cross_region_slo.rs:55-56` |
| Tenant-axis substrate probes | `check_multi_tenant_loom.rs:61`, `check_reza_production_path.rs:52` |
| Team/datname uniqueness; `TeamEntry` shape | `manifest.rs:727-747`; `:154-159` |
| Env-contract gate's scan scope (why registration is inert) | `xtask/src/check_env_contract.rs:89`, `:118-120`, `:175` |

**Budget.** kernel-core ZERO @ **23401** (re-verified at `b400d127`); fkcs frozen `23081`; no new crate, no new dependency. New surfaces: one composite action, two xtask checks, one test + one gate leg.

**Parallelism.** Runs alongside 13.6a/13.6b/13.6d. The only overlap risk is gate-file edits — this story touches the *workflow* plus `check_multi_tenant_loom.rs`'s `live_substrate_present()` and leg list. Coordinate on `discipline.yml` and `check_multi_tenant_loom.rs` if a sibling story needs a change.

**Declared residual (carry to 13.6).** `cross_region_live.rs` is the shared oracle for **two gates and nine legs across three stories** (11.2a, 11.2b, 13.3) — an accreted coupling nobody decided. Not fixed here. The closer should know the surface it is judging.

### References

- [Source: `epics/epic-13-reza-cortex-v2-2.md#73`] — *"the smallest real wall is a CI-provisioned Postgres service with two distinct datnames … the full rung is the 3-team × 3-region Reza scene."*
- [Source: `epics/epic-13-reza-cortex-v2-2.md#180`] — evidence state vs enforcement class.
- [Source: `_bmad-output/implementation-artifacts/11-3-scale-envelope-25-30-host-churn.md`] — RED-at-HEAD contingency wording to reuse.
- [Source: `_bmad-output/implementation-artifacts/13-5g-tl-stage2-datname-inversion-defense-in-depth.md`] — the `.expect`-never-skip idiom and its null-control history.

---

## Dev Agent Record

### Agent Model Used

`glm-5.2` (zai/glm-5.2). ZERO kernel-core Δ; the work is CI workflow + xtask
drift controls + test fixtures.

### Debug Log References

- Before/after `--json` blindness evidence: `_bmad-output/implementation-artifacts/13-6c-evidence/`
  (`before-{cross-region-consensus,multi-region-slo}.json`,
  `after-{cross-region-consensus,multi-region-slo}.json`, `SUMMARY.md`).
- Verified live against a local Postgres 17 + pgvector stand-in for CI's
  `pgvector/pgvector:pg16` (no docker/podman available; PG17 server + vector.so
  present on-host).

### Completion Notes List

- **T1/T3 (AC1/AC2):** three databases provisioned (`maos_team_{a,b,c}`) plus
  `maos_shared`; role-disjoint rule applied — `MAOS_TEST_POSTGRES` → its own
  `maos_shared` in the two jobs gaining a service (consensus, slo), never
  aliased onto a team DB; the existing multi-tenant alias (`MAOS_TEST_POSTGRES`
  → `maos_team_b`) left untouched per AC1. The `services:` block is duplicated
  (structurally unsingle-sourceable, D-8) and held byte-identical by the
  service-block drift control; the `CREATE DATABASE` step half is single-sourced
  as the `.github/actions/provision-loom-substrate` composite action. Env unions
  wired to AC2's table exactly (verified by the drift gate).
  `timeout-minutes: 20` added to both new-service jobs.
- **AC2/AC5 reconciliation:** AC2 names "CREATE DATABASE + env export" for the
  composite action, but AC5 requires the env unions in the jobs' `env:` keys so
  the drift control can parse them. Resolution: the composite action holds the
  imperative DB creation; the declarative env contract stays in `discipline.yml`
  `env:` keys (what AC5 parses). Each half lives at the layer that can hold it.
- **T2 (AC1):** `pg_conn_team` widened for `team-c`; `current_database_team`
  helper + `three_team_databases_are_physically_distinct` `#[ignore]` test added;
  `check-multi-tenant-loom` gains the `three-team-databases-physically-distinct`
  leg; `live_substrate_present()` now requires `TEAM_C`.
- **D-4 caught at dev time by the drift control (T7):** placing the `team-c`
  reader in `cross_region_live.rs` made the consensus whole-binary oracle read
  `TEAM_C`. The env-consistency check (whole-oracle leg) forced `TEAM_C` into
  consensus's contract + env — exactly the D-4 new-reader-drift control earning
  its keep before CI.
- **T7 (AC5):** `check-loom-substrate-drift` — two structural legs (no Postgres),
  blocking at every phase. (1) env-consistency derives per-gate reader sets from
  each invoked test filter, compares them bidirectionally with the declared
  contract, and checks only env visible to the gate-running step. (2)
  service-block drift discovers every job using the provisioning action before
  comparing `services.postgres` modulo `POSTGRES_DB`; an unregistered fifth job
  reds the gate. Nine module tests cover the three original mutations plus
  filtered-reader reachability, step scope, and fifth-job discovery. Wired into
  `v1-0-ship-gate` needs + `gate-registry.toml`.
- **T8 (AC4):** every live reader fails loud on its own var (`pg_conn` `.expect`,
  `pg_conn_team`/`pg_conn_for` `panic!`) incl. the new `team-c` arm. No
  `env_contract.rs` edit (D-6: that gate scans `maos-bin/src` only and is
  operator-facing; the drift control is its replacement).
- **T9:** `check-kernel-baseline` 23401==23401 (ZERO kernel Δ); `kloc-check`
  PASSED (xtask ceiling bumped 32517→33270, measured 32617 + ceil(2%)=653, house
  formula); `cargo fmt --all -- --check` clean; `check-env-contract` unchanged
  at its pre-existing 5-violation Story-12.7 count (no `maos-bin/src` change);
  `check-ship-gate-completeness` PASSED; `cargo test -p xtask` passed 534 tests
  (1 ignored). Local timings are ~3s (consensus) and ~19s (SLO), but they are not the required CI
  wall-clock measurement. T9 remains unchecked until the first GitHub-hosted
  workflow run records the before/after job durations.
- **T4/T5 (AC3):** before+after `--json` captured + diffed (see evidence dir);
  the dark `three-region-convergence` leg now executes (3 passed); topology-fraud
  negatives hard-fail on both axes (region + team).
- **T6 (AC3) — RED-at-HEAD findings (legs NON-BINDING at v1_5):** (1) `roundtrip-slo`
  p95=36099µs > the speculatively-set 11.2b 30000µs floor — advisory, floor NOT
  relaxed, CI authoritative. (2) 13.5d/13.6b live daemon-boot legs fail on the
  local dev shell only (env), pass in CI — not a 13.6c regression. See SUMMARY.md.

### File List

- `.github/actions/provision-loom-substrate/action.yml` (NEW — composite action)
- `.github/workflows/discipline.yml` (consensus+slo gain a service/timeout/composite-action/env; multi-tenant gains TEAM_C; reza CREATE DATABASE consolidated; new `check-loom-substrate-drift` job + ship-gate needs/summary/echo)
- `crates/maos-loom-lite/tests/cross_region_live.rs` (pg_conn_team `team-c` arm; `current_database_team`; `three_team_databases_are_physically_distinct` test)
- `xtask/src/check_loom_substrate_drift.rs` (NEW — env-consistency + service-block drift gate, 6 tests)
- `xtask/src/check_multi_tenant_loom.rs` (three-team leg; `live_substrate_present` +TEAM_C)
- `xtask/src/check_dev_model_used_populated.rs` (T10 hygiene: `glm-5.2` added to `KNOWN_MODELS` — allowlist lagged; used by 5+ shipped stories)
- `xtask/src/main.rs` (mod + Commands variant + dispatch)
- `xtask/gate-registry.toml` (gate list + blocking `[[ship_gate]]`)
- `xtask/kloc.toml` (xtask ceiling 32517→33270)
- `_bmad-output/implementation-artifacts/13-6c-evidence/` (NEW — before/after JSON + SUMMARY)
---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created as Track 2 of the Epic-13 completion plan — parallel enabler, no code dependency on 13.6a/13.6b. Grounding corrected the scope **down** from 9 databases to 3 (the manifest cannot express a team spanning regions), and **up** in severity: `check-multi-region-slo` declares `three-region-convergence` as a leg and runs with **no Postgres service at all**, so that leg and its topology-fraud negative have never executed since 11.2b. 4 ACs, ZERO kernel Δ. |
| 2026-08-01 | **Adversarial preflight (party-mode round table, direct verification @`b400d127`). Status held ready-for-dev.** Kernel pin **re-verified by the preflight: pin == actual == 23401**; `depends_on: none` re-verified (only a 6-line comment moved in `discipline.yml` since `cb412348`). D-1/D-2/D-3 substance **confirmed**, but **every line cite was stale** (13.6a inserted `team_of_host` above `validate_team_map`: `:652`→`:727`; `TeamEntry` `:130`→`:154`) — re-pinned throughout, and D-1's reasoning corrected: `region` is **not** a uniqueness axis, so "team-N in region-N" is a **fixture choice**, not a manifest constraint. **Two ship-blockers found in the story's own AC2 (D-4):** `check-cross-region-consensus` probes `MAOS_TEST_POSTGRES` — *singular* (`:308`), so the AC as written would have provisioned a service and left the gate skipped; and its oracle runs the **whole 32-test binary** unfiltered with a 4-leg broadcast (`:160-172`, `:110-116`) where every env reader **panics** rather than skips — so the required env is a **union derived from the readers**, now an AC2 table. **AC4's premise was wrong (D-5):** `LegResult::skipped` sets `green: false`; what yields exit 0 is `CURRENT_PHASE = "v1_5"`, not `passed: blockers.is_empty()` — so **a red region leg still exits 0 after this story**, and that is now stated rather than implied; `BindingClass` for these two gates is explicitly boundaried to 13.6/13.6e. **AC4's `env_contract.rs` clause DELETED as null control #28 (D-6):** `check-env-contract` walks `maos-bin/src` only (`:89`,`:118-120`) and the file is operator-facing. **`TEAM_C` had zero consumers (D-7)** — the AC would have provisioned a database with no reader, this story's own failure mode; resolved by landing a minimal three-distinct-`current_database()` reader on `check-multi-tenant-loom` **only**, leaving `check-reza-production-path` untouched. **AC2's "composite action" was unbuildable (D-8):** composite actions cannot define `services:` and Actions has no YAML anchors; re-specced to composite-action-for-steps plus a drift control. **AC1's one-set-vs-two-set resolved by construction, not by observation:** role-disjoint datnames within a job, driven by the alias already at `discipline.yml:2713` and table-scoped `DELETE FROM collective_memory` resets. **`timeout-minutes` added** to the two untimed jobs (D-9). **New AC5** turns AC2's two promises into two gates (workflow-env ⟷ reader-var consistency; service-block drift). Scope: **4 ACs → 5**, 8 tasks → 10, ZERO kernel-core Δ held @23401. |
| 2026-08-01 | **Implemented (dev model glm-5.2). Status → review.** All 10 tasks done, 5 ACs met. Substrate provisioned (`maos_team_{a,b,c}` + `maos_shared`, role-disjoint) for consensus+slo via a shared `provision-loom-substrate` composite action; `team-c` axis + three-team reader landed (T2). The `check-loom-substrate-drift` gate (AC5) — env-consistency (D-4/D-7 catcher) + service-block drift — caught a real D-4 at dev time: the `team-c` reader in `cross_region_live.rs` forced `TEAM_C` into the consensus whole-binary union. Proven-red via 3 mutation tests; before/after `--json` captured + topology-fraud hard-fails proven on both axes. RED-at-HEAD: `roundtrip-slo` p95 exceeds the speculatively-set 11.2b floor (advisory, NOT relaxed). ZERO kernel-core Δ @23401; kloc xtask ceiling bumped 32517→33270; 379 xtask tests pass. |
| 2026-08-01 | **Code review remediation. Status → in-progress.** Four parallel layers (Blind, Edge, Acceptance, Test Infrastructure) produced 6 unique patch findings. Five code/configuration patches applied: multiline composite-action input moved out of generated Bash syntax; gate-step env scoping; per-filter reachable-reader derivation; provisioning-action job discovery; ship-gate completeness registration. Verified with a three-database action smoke, `check-loom-substrate-drift --json`, `check-ship-gate-completeness --json`, `cargo test -p xtask` (534 passed, 1 ignored), and `cargo fmt --all -- --check`. T9 remains open because the required GitHub-hosted CI wall-clock delta cannot exist before this uncommitted change runs in CI. |
