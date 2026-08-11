---
baseline_commit: b568a052 + the then-UNCOMMITTED Story 13.6e working tree (29 files, +5231/−1340). Story 13.6e was `done` at this baseline and was reopened by this story's 2026-08-08 documentation review. Measure the working tree, not the historical baseline alone.
depends_on: 13-6a (DONE @a414f922), 13-6b (DONE @05e7e967), 13-6c (DONE @c571a2b9), 13-6d (DONE @b400d127), 13-6e (REOPENED 2026-08-08 — published-ledger omission validation)
blocked_by: NONE. All four forks recorded on 2026-08-06 were **resolved by measurement on 2026-08-07** (see `## Resolutions`) — three were defects with a single correct fix, one is answered by precedent. Nothing awaits an operator choice; ratify the reasoning if you wish, but dev is unblocked.
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin **23679** (verified by execution 2026-08-06: `maos-kernel-core/src = 23679`, pinned `23679`)
inherited_residuals: (a) the kernel collapses **eight** `TransportCause` variants into **one** `CollectiveErrorKind::Transport` at `maos-kernel-core/src/memory/mod.rs:206` — 13.6e registered `kernel-collective-cause-distinguishable` as a machine-readable successor and **names Story 13.6 as its owner in code** (`check_multi_tenant_loom.rs:167-172`); this story rules the claim in writing and re-assigns the owner without implementing the widening; (b) `CollectiveMemoryPort` has exactly four verbs — `write`/`read`/`scan`/`erase`, no `share` (13.6b Residual 7, re-verified).
---

# Story 13.6 — The Reza Cortex journey closer: compose, judge, and refuse to over-claim

Status: **done** — reopened and re-closed 2026-08-11 for the budget close-out this story's own Budget section ordered. Functional close stands at operator-lane commit `9160eecb`: all six journey processes execute through their production entries, the collective erase reconciles both sides under provenance-bound authorization, the fourteen-institution isolation axis is measured live, and all four substrate gates publish `product_claim: PROVEN` with the required `reza-three-team-three-region-journey` leg `PROVEN_LIVE_SIGNED`. The published-ledger omission vulnerability that reopened Story 13.6e is fixed and that story is closed with this one. The first close was **incomplete**: `kloc-check` was never re-run and three ceilings were red. Re-measured, FLAG-Winston granted, and green — see *Budget close-out*.

**Kernel-Δ: ZERO expected @ 23679.** Work lands in tests, `xtask` declarations, `.github/workflows`, `docs/`, and `_bmad-output/`. **No new gate. No new mechanism.**

> **Read this first — this story has been re-grounded four times, then a fifth resolution round (2026-08-07) closed every open fork. Reframes 1–3 are historical; see the Change Log.**
>
> The fourth pass (2026-08-06, six adversarial scouts) found two independent ways CI is red and left four forks. **The fifth round resolved all four by measurement — none was an operator preference.** See **Resolutions**. What you are starting from:
>
> 1. **CI is RED at `HEAD` on a blocking gate, filed nowhere.** `check-service-boundary` fails on 5 `spill_test_faults` symbols (D-0). ✅ **RESOLVED (F-3): it is a gate bug** — the gate's own P4 walk already contains the 8-line cfg-skip that would suppress it; the main surface walk never received it.
> 2. **13.6e, once pushed, reds all four journey gates in CI.** ✅ **RESOLVED (F-1): 13.6e introduced an unsanctioned THIRD enforcement axis**, violating the two-axis invariant stated in `gate_common.rs:31-33` — the same file it extended, three lines below a citation of `project_gate_binding_decay`. `epic-13:200` explicitly permits a development lane to stay advisory while `ABSENT`. Fix is a workflow declaration; the claim refusal is **already** correctly declared at `v2_2`.
> 3. **AC5's "consumes the ledger and does not modify it" is unsatisfiable** (D-2). Re-drawn as *machinery vs declarations*. ✅ **Two of its four blockers dissolved** once F-2 measured the contract table: they were artefacts of 13.6e registering a **three-team** control on the only ledger gate with a **two-team** substrate.
> 4. **AC4's premise was wrong in the story's favour.** Causes (a) and (b) are not "buildable" — they are **already built, already proven, and already ledger-bound legs**. Filing them as work would be filing closed work. What is genuinely open is narrower and was never named: the **refused**-crossing operator tail, and **retry/recovery, which has zero coverage anywhere** (D-3′).
> 5. **The kernel collapse is 8 → 1, not 8 → 5.** The previous draft understated the erasure five-fold (D-4′).
> 6. **AC1 item (d) is already closed** by 13.6c. Carrying it forward would file a fabricated requirement.
> 7. **AC2's "2 → 3 is arithmetic" is true only for a chain.** A hub needs a production change, which trap #1 forbids (D-5′).
>
> **What this story does NOT claim.** It does not close NFR-Ops-11 (team axis served; operator sub-axes (i) and (iii) open, and **(i) appears in no residual register at all**). It does not execute 14 institutions. It does not build a mechanism.
>
> **After review, this story does not claim the journey is currently `PROVEN` on any lane.** CI cannot reach `PROVEN_LIVE_SIGNED` because it holds no operator key. The published operator artifacts bind the pre-review oracle that incorrectly signed expected pre-dispatch failures; they remain historical evidence of that run, not evidence for the corrected worktree. The current required journey leg is `ABSENT`.

---

## Story

**As** Reza, running a 400-person fintech as one governed Cortex on shared MAOS infrastructure,
**I want** the platform's claim that my three teams can collaborate under governance to rest on a recorded, machine-derived verdict over a real multi-team substrate,
**so that** "the Reza journey works" is something a gate asserted from evidence it produced — and, where a mechanism is absent or a proof is unreachable, something the ledger **refuses** to assert and the epic records with a live owner.

---

## Grounded state of the six mechanisms this story judges

**Re-measured 2026-08-06 against the working tree.** Every line below was re-pinned; the ones that moved are marked.

| Mechanism | Production-reachable? | Production entry point (verified) |
|---|---|---|
| Tenant boot / physical + crypto wall | **YES**, live-substrate only | `main.rs:2773-2847` (store `:2847`), tenant map `:2802-2825`, consent `:2826-2831`, team keys `:2832-2840`, datname guard `:2879-2916` |
| Spirit→collective route (13.5d) | **YES** | `maos run <researcher-manifest>` → `main.rs:4484` → `LiveResearcherCollectivePort` `:4497-4512` → `spirits/researcher/src/lib.rs:587,600` cap-gated |
| Collective erase (13.5b) | **YES** | `MAOS_ONE_SHOT=collective-erase` → `main.rs:5044-5099` → `port.erase` `:5078` → TL `collective.operator.erase` `:5087-5091` |
| Vetting promotion (13.4) | **YES**, but **orthogonal** | `spirit-upgrade` `main.rs:6493` / `hot-swap-precheck` `:6211` → `enforce_vetted_upgrade_precondition` `:106`, short-circuit `:140-142`. **Touches nothing in the crossing** — see AC4 |
| Enterprise Spirit at the daemon seam (13.5a) | **YES** | `main.rs:7755-7782` → `build_enterprise_daemon_governance` `:7763` (def `:9337`); boot self-check `:9562-9575`, **call site `:9491`** |
| **Cross-team crossing — WRITE** (13.6b) | **YES** ✅ | emitter `run_cohort_a2a_daemon` `main.rs:9419`, dispatch `:9523-9529` → `emit_cross_team_share` `:9832`; applier port built `:9459-9481`; `router.rs:1629-1649` → `cross_team_crossing.rs:192` → `apply_replication_bundle` `:255` |
| **Cross-team crossing — READ** (13.6d) | **YES**, ⚠ **same-filesystem** | `maos traceback --team <T>` `main.rs:2386`/`:3057`; `recall_cross_wall` `crates/maos-iac/src/adapter/log_recall.rs:378` → `read_remote` `:422`. ⚠ `cross_wall_log_read.rs:25`, body `:31-72` opens a **local SQLite path** — no socket, no A2A frame, **no network hop** |

**Dead-wire ledger: both clauses inverted, none unassigned.** `(f-i)` deleted in `05e7e967`; `cross-wall-recall-no-production-caller` deleted in `b400d127`. Each replaced same-commit by a reachability walk plus negatives that only became falsifiable because the path is live.

---

## The defects this story closes — measured 2026-08-06

### D-0 — ⚠ NEW: CI is RED at `HEAD` on a blocking gate, and it is filed nowhere

`check-service-boundary` **fails**, reproducibly, in CI (run `30881082656` at `b568a052`: 154 jobs, exactly one real failure) and locally:

```
passed: false
NFR-Test-2 violation: new public kernel symbol
  maos_kernel_core::memory::spill_test_faults::{PausePoint, FailurePoint, arm_failure, arm_pause, disarm}
  has class 'other' (must be one of: universal-arithmetic, data-movement, supervision)
```

The module is correctly cfg-gated (`memory/mod.rs:21` `#[cfg(any(test, debug_assertions))]`, `:22` `pub mod spill_test_faults;`). The defect is that **three gates treat one module three different ways**: `kloc_check.rs:189` **excludes** it; `check-kernel-baseline` **counts** it (its 107 lines are inside the 23679 pin); `check-service-boundary` walks a **debug** build, where `debug_assertions` holds, and so counts it as public kernel API.

`check-service-boundary` is in `aggregate`'s 122 `needs`. It is **not** in `deferred-work.md`. And 13.6c's change log (`13-6c…md:372`) claims *"All gates green (baseline/kloc/drift/ship-gate/**service-boundary**/fmt)"* at the very commit that reds it — a live instance of *a claim standing in for a control*, inside the epic that catalogued the shape.

✅ **RESOLVED AND FIXED — `accf763c`.** It was a **gate bug**: the P4 walk's cfg-skip was never given to the surface walk. Both walks now share `is_test_cfg_mod`, and the baseline's single mis-captured `::tests::` entry was removed. Gate PASSES; proven still a real control. **This defect is closed — it is no longer a starting condition.**

### D-1 — ⚠ NEW: the enforced lane cannot go green — because 13.6e added a third enforcement axis

Measured by execution (`GITHUB_ACTIONS=true` + full GitHub binding, working tree):

```
check-reza-production-path → EXIT 1
passed: false | ledger_enforced: true | operator_key_available: false
product_claim: NOT_PROVEN(4 required live legs)
```

The chain is closed and each link is verified:

1. `ledger_enforced()` (`evidence_ledger.rs:126-135`) is true whenever `GITHUB_ACTIONS` is set.
2. CI holds **no operator key** — `grep -rn "MAOS_AUDIT_KEY\|audit-signing.key\|MAOS_LEDGER_ENFORCE" .github/` → **zero hits**. This is deliberate: `evidence_ledger.rs:984` — *"a CI that holds the operator key would be theatre."*
3. `EvidenceVerdict::project` (`gate_common.rs:197-211`): `AdvisorySubstrate + attempted + green + !signature_verified` → **`Indeterminate`**.
4. `blocks_product_claim(enforced)` (`evidence_ledger.rs:825-834`): `Indeterminate` blocks when `enforced`.
5. The workflow steps (`discipline.yml:2828`, `:2898`) carry **no `continue-on-error`**.

So **with** Postgres the live legs are green-but-unsigned → `INDETERMINATE` → block; **without** Postgres they are `ABSENT` → block. Both paths red. `check-multi-tenant-loom` alone carries 13 required `AdvisorySubstrate` legs.

⚠ **Blast radius is bounded and must be stated precisely.** The four journey gates are **NOT** in `aggregate`'s `needs`; `check-ship-gate-completeness` **is**, and it `needs:` all four with `if: always()`. Because `finish_ledger_gate` writes the artifact *before* returning `Err` (`:1372` precedes `:1431`), the ledgers still publish, and all four gates are `v1_5 = "advisory"` so the ship badge's `NOT_PROVEN` arm stays dormant. Net effect: **four visibly-red jobs, a failing workflow conclusion, and a passing ship badge.**

**Zero legs anywhere in this repo have ever reached `PROVEN_LIVE_SIGNED`.** That state is currently unwitnessed.

✅ **RESOLVED AND FIXED AT SOURCE — `c45df0be` (13.6e reopened).** Not a deadlock: the posture blocked on a state CI can never reach. `blocks_product_claim` now blocks `INDETERMINATE` only when the leg is actually RED, so a green-but-unsigned leg refuses the **claim** without blocking a **lane** — `epic-13:200`'s split. `ABSENT` semantics unchanged (measured on the enforced lane, no Postgres: exit 1, 15 ABSENT — the correct 'substrate did not come up' block). **This defect is closed — it is no longer a starting condition.** `PROVEN_LIVE_SIGNED` remains unreachable in CI **by correct design**; it belongs to the operator-run lane this story is the first to execute.

### D-2 — AC5's non-modification rule is unsatisfiable as written

Making the journey leg *run* is a clean hand-off needing **zero** xtask edits: declare `reza_three_team_three_region_production_journey` in `crates/maos-bin/tests/cross_team_crossing_13_6b.rs` with `evidence_record::attest(...)` as its first statement, and the probe (`check_reza_production_path.rs:145-157`) plus the trusted mapping (`evidence_ledger.rs:230-232`) already exist. **That part is well designed.** Four things nonetheless force declaration edits:

| # | Blocker | Where |
|---|---|---|
| 1 | The leg is `required: false`, so even a proven journey contributes **nothing** to `product_claim` | `evidence_ledger.rs:90-96` (`NOT_REQUIRED_LEGS`), filtered at `:1049`; the consumer cross-checks `leg_is_required` and hard-errors on disagreement (`:1197-1203`), so it cannot be overridden anywhere else |
| 2 | The leg can **never** be `PROVEN_LIVE_SIGNED` in CI | `journey_successor` short-circuits to ABSENT when `!verifier.key_available()` (`check_reza_production_path.rs:195-203`) — see D-1 |
| 3 | It is registered on the **two-database** gate | `check-reza-production-path` provisions `maos_team_{a,b}` only (`discipline.yml:2849,:2872`), exports 2 vars (`:2895-2897`), `live_substrate_present()` checks A/B (`check_reza_production_path.rs:25-29`), and the harness's `pg_conn_team` **panics** on `team-c` (`cross_team_crossing_13_6b.rs:948-957`) |
| 4 | Fixing #3 forces an edit to the file that **defines the ledger set** | `check_loom_substrate_drift::CONTRACTS` (`:143-185`) pins the reza contract; `contract_jobs()` **is** `evidence_ledger::ledger_gates()` (`evidence_ledger.rs:140-142`); `run_env_consistency` (`:463-520`) enforces bidirectional exact match, so a new var without a reachable reader reds as a D-7 phantom |

**Ratified re-draw (this story's AC5):** 13.6 may not touch the **machinery** — `EvidenceState`, `EvidenceVerdict::project`, verification, or the artifact schema. It **may, and must,** change **declarations**: remove the journey leg from `NOT_REQUIRED_LEGS`, **re-home the leg to `check-multi-tenant-loom`** (F-2 — that gate's contract already requires `TEAM_A/B/C`, so blocker #3 and #4 dissolve entirely), and set `MAOS_LEDGER_ENFORCE=0` on the four gate steps (F-1). **Blockers #3 and #4 above are therefore RESOLVED, not carried** — they were artefacts of 13.6e's mis-placement, not properties of the design.

⚠ **Latent trap:** a journey that runs **green but unsigned** is force-downgraded to `absent_successor(...)` (`check_reza_production_path.rs:205-214`) — reported **ABSENT, not INDETERMINATE**. In the artifact it is indistinguishable from "13.6 never wrote the test" except via the `detail` string. **Any evidence claim keyed on `evidence_state` alone will misread it.**

### D-3 — AC4's premise was wrong: (a) and (b) are already built, proven, and ledger-bound

| Cause | Previous draft | **Measured** |
|---|---|---|
| (a) reverse share without grant | "buildable" | **ALREADY BUILT + PROVEN** at every hop |
| (b) stale consent lease | "buildable" | **ALREADY BUILT + PROVEN**, from **two** independent production sources |
| (c) vetting lapse | not buildable | **NOT BUILDABLE — confirmed 3/3**, stronger than filed |
| (d) legal hold | not buildable | **NOT BUILDABLE — no join key exists**, stronger than filed |

Distinguishability is already asserted and already a leg: `crossing_wire_13_6b.rs:124-190` asserts `["crossing_consent_denied","crossing_consent_stale","crossing_state_unavailable"]` with `unique.len() == 3`, bound as `crossing-causes-stay-distinguishable-on-the-wire` (`check_multi_tenant_loom.rs:637`) and `crossing-stale-gate-keeps-typed-outcome` (`:765`). The true B→A reverse denial plus staleness are proven live in one test: `cross_team_consent_13_3.rs:649-746` (`:692-711` denial, `:733-745` `clock.advance(121)`), already the leg `asymmetric-consent-reverse-share-refused` (`check_multi_tenant_loom.rs:351`).

**What is genuinely open — and was never named:**
- **The refused-crossing tail.** `grep crossing_applied|crossing_outcome_label` returns **zero hits outside `main.rs`**. Nothing reads the operator label or the TL `status` field for a *refused* crossing. The two-daemon live test exercises only the happy path.
- **Retry / recovery.** `grep -rn "retry\|recover\|repair"` across all three crossing test files → **zero**. The AC's *"retry succeeds only after a valid manifest/consent repair"* has **no coverage at all**.

⚠ **Three draft citations are wrong and must not be copied forward:**
- `main.rs:9912-9913` → the range is **`:9910-9922`**; `crossing_consent_stale` is at **`:9914`**, outside the old range.
- `cross_team_crossing_13_6b.rs:1484-1503` → actual **`:1482-1510`**, and its source team is **`team-c`** — an ungranted third team, **not the reverse** of the granted A→B pair. Cite `cross_team_consent_13_3.rs:649-746` instead.
- *"stale tenant-map / consent lease"* conflates two different things. **Consent lease** stale is host-path and distinguishable. **Tenant map** stale is a different error from `connection_assignment_guard` (`store.rs:1088-1120`) / `team_guard` (`:1136-1184`) on the **Spirit** path, behind the kernel collapse, and **not** distinguishable. `cross_team_crossing.rs:146` relabels it, but that result reaches only an `eprintln!` (`:268-270`) — no TL, no wire.

### D-4 — the kernel collapse is 8 → 1, and the ruling must name the right six causes

`crates/maos-kernel-core/src/memory/mod.rs:206` — `CollectivePortError::Transport(_) => CollectiveErrorKind::Transport`. **All eight** `TransportCause` variants (`collective_memory.rs:24-58`) land on **one** kind. The previous draft's *"eight into five"* understates the erasure five-fold.

**Refinement that changes the ruling.** `StoreError::ConsentDenied` and `ConsentStateStale` have **no production constructor** (documented in-code, `adapter.rs:196-200`). On the Spirit path causes (a)/(b) are therefore **unreachable**, not erased. Writing *"the consent-denial cause is erased on the Spirit path"* would be **false**. The six causes that path *can* produce and that **do** arrive as the single word `Transport`: `TenantMapStale`, `TenantConnectionMismatch`, `TenantSpiritUnmapped`, `AttestationInvalid`, `PartitionRefused`, `ErasureTombstoneDominates`.

⚠ **`check_multi_tenant_loom.rs:167-172` already names *"owned by Story 13.6"* in code**, and `:170` says *"all **five** collective causes"* (there are eight). Leaving both untouched means the machinery names a `done` story as owner the day this story closes — **the exact stale-owner defect AC5's sweep exists to catch, inside the epic's own instrument.**

### D-5 — AC2: "2 → 3 is arithmetic" holds for a chain, not a hub

`CrossTeamShareRequest` (`cross_team_crossing.rs:285-292`) is singular in every field; `from_env()` (`:296`) returns **at most one**; `main.rs:9523` calls it **once**, then parks on `ctrl_c()` (`:9531`). One crossing per daemon boot.

| Option | Processes | Production change? |
|---|---|---|
| **(a) Chain A→B, B→C** | **3 daemons** | **NONE** ✅ |
| (b) Hub via a 4th host in team-a | 4 daemons | NONE |
| (c) True hub from one host | 3 | **YES — forbidden by trap #1** |

⚠ The chain is **two independent originations, not a transitive flow** — `originate_team_row` (`bundle.rs:458`) mints a *new* row from B's own store stamped `source_team=team-b`. **Describe it honestly or the ledger repeats the failure shape this story exists to catch.**

⚠ **The harness is single-region.** `daemon_command:1189` hard-codes `MAOS_REGION_HOME=region-a`; `manifest_with:708-759` puts both TeamEntries in `region-a`. The ×3-region half is *permitted* but not existing. Nothing reconciles `MAOS_REGION_HOME` against `TeamEntry.region` — a finding, not a thing to fix.

**Minimum honest process count: 6** — 3 daemons + `maos run` (`:4484`) + `collective-erase` (`:5044`) + `maos traceback` (`:3057`). CLI one-shots return at `:3072`/`:3091`, **4682 lines** above the daemon dispatch at `:7773`. **Do not write "one run."**

### D-6 — AC1 shrinks to three items; the fourth is already closed

13.6c shipped the substrate. Re-measured:

- **(a) topology-fraud proven-red — OPEN, but NARROWER than drafted.** The two negatives are inside `three_region_convergence_all_three_equal` (`cross_region_live.rs:2058`): distinct-`datname` reconcile **`:2070-2087`**, physical absence **`:2135-2147`** (both +17 from the draft). A **third**, team-axis negative the draft never named: `three_team_databases_are_physically_distinct` **`:528`**. 13.6e now machine-derives *"the negative ran"* (`evidence_ledger.rs:160-164`, `:188-190`); only *"it reds on fraud"* is still prose (`13-6c-evidence/SUMMARY.md:35-42`, a one-off local exit-101). **Scope AC1 to the second half only.**
- **(b) drift-gate value blindness — FULLY OPEN.** `collect_env_keys` **`:338-346`** still `env.keys()`. ⚠ **A naive all-distinct reconcile would FALSE-RED**: `_A`/`TEAM_A` deliberately alias onto `maos_team_a`, and `MAOS_TEST_POSTGRES` aliases onto `maos_team_b` in `check-multi-tenant-loom`. The rule must be **axis-scoped**: `{_A,_B,_C}` pairwise distinct; `{TEAM_A,TEAM_B,TEAM_C}` pairwise distinct; cross-axis aliasing allowlisted.
- **(c) local-setup docs — FULLY OPEN.** `grep -rn "maos_team_c" docs/ README.md` → **zero**. `cross_region_live.rs:12-17` still documents singular `MAOS_TEST_POSTGRES` against `dbname=maos_test`; same staleness at `migration_live.rs:12`.
- **(d) dead `check-env-contract` clause — ✅ ALREADY DELETED by 13.6c.** Verified: no substrate clause survives in `gate-registry.toml`, `check_ship_gate_completeness.rs`, the workflows, or `ABSENT_SUCCESSORS` (which is no longer a const). **Record as a no-op. Do not re-file.**

**Re-pinned CI jobs** (all drifted): consensus **`:2630`**, slo **`:2700`**, loom **`:2787`**, reza **`:2840`**, drift **`:2917`**.

---

## Acceptance Criteria (6)

### AC1 — Substrate close-out: the topology cannot be faked, and it can be reproduced

**Given** 13.6c shipped 3 team `datname`s + 3 region aliases and left three gaps (D-6),

**Then** the **topology-fraud negatives become committed, re-runnable proven-red controls** — all **three** limbs (`cross_region_live.rs:2070-2087`, `:2135-2147`, `:528`) — in the idiom `check_loom_substrate_drift.rs:815-890` already uses: *load the real config → plant one specific defect in an in-memory clone → assert not-green **and** that the problem names the defect by token*. Restore is by construction (nothing on disk is mutated),

**And** the drift gate's **value blindness** is closed or recorded with an owner — and if closed, the reconcile is **axis-scoped** per D-6(b), or it false-reds on the ratified aliases,

**And** local reproduction is **documented for all four substrate jobs**, and the stale singular-`MAOS_TEST_POSTGRES` headers at `cross_region_live.rs:12-17` **and** `migration_live.rs:12` are corrected,

**And** ⚠ "three physically-distinct region databases" means **three** databases. `validate_team_map` (`manifest.rs:727`) rejects duplicate `team_id` (`:738-741`) and duplicate `datname` (`:743-747`); `TeamEntry.region` (`:154-159`) is a scalar and is **not** a uniqueness axis. **Not six, not nine,**

**And** the dead `check-env-contract` clause is **recorded as already-closed by 13.6c**, not re-filed as work.

### AC2 — One composed 3-team × 3-region topology, through production entry points only

**Given** "one run" is structurally impossible (D-5),

**Then** the scene is **one composed topology, explicitly written down** — **3 daemon processes + 3 CLI one-shot processes = 6**, under one signed manifest, one authority key, one base seed, three distinct datnames — and the **chain shape (A→B, B→C)** is chosen so that **zero production lines change**,

**And** it **extends `cross_team_crossing_13_6b.rs:1204`** (`live_crossing_runs_through_two_daemon_processes`). Building a second harness is **forbidden**. The ~10 required edits are all test-side: `pg_conn_team:948-957` (+`team-c`), a **new** 3-team manifest builder (do **not** mutate `manifest_with:708-759` — 7 call sites across 18 tests), `write_daemon_manifest:1010-1040` (slice), `write_daemon_file:1053-1105` (both target fields are already `Vec`), `daemon_command:1189` (parameterize `MAOS_REGION_HOME`), a third boot + `NONCE_C`, a third raw oracle client,

**And** the composed scene sets a **shared `MAOS_HOME`** across every process — otherwise `maos traceback --team team-b` derives a per-process basename and reads a path that does not exist (`main.rs:88-103`; `transparency_log_path_for_team`, `maos-audit/src/lib.rs:898`),

**And** a **constructed-but-unwired control fails per site** — **seven** targets, not six; the previous draft omitted the applier-port construction at **`main.rs:9459-9481`** (delete it and `CrossingOutcome::NotCrossing` → `StateUnavailable` NACK at `router.rs:1639-1646`). Serialized, byte-identical restore, proven per limb,

**And** ⚠ the story **writes no mechanism**. If the run needs something absent, that is a finding with a named owner — never a harness-local implementation.

### AC3 — Allowed collaboration with minimum disclosure

**Given** 13.6b made the write crossing production-reachable and 13.6d the read side,

**Then** the crossing bundle carries **only policy-allowed provenance** — `originate_team_row` (`bundle.rs:458`) stamps `source_region` `:479`, `source_team` `:487`, `distillation_depth` `:488`, `intent_lineage` `:489`, `RowAttestation` `:500` (**all five verified 2026-08-06**); raw payload, secret-bearing fields and unconsented TL references are **negative controls that red**,

**And** provenance lands **with the row** (destination namespace `xteam:<team>:`) and dereferences inside the consumer team's own database,

**And** the read side's minimum disclosure is judged **as measured**: `build_entry` (`log_recall.rs:240-249`) returns exactly six fields — `frame_id`, `timestamp_ns`, `kind`, `intent`, `peer_spirit_pid`, `payload_available: bool` (`!payload_redacted.is_empty()`). **No payload bytes cross,**

**And** ⚠ **provenance-presence, never provenance-promise** (ADR-049 §7),

**And** ⚠ **three honest limits are stated, not buried.** (i) `MAOS_CROSS_TEAM_BASE_SEED` is a sign-side secret every emitter holds; the envelope/payload weld (`cross_team_crossing.rs:287-296`) is the only thing between that and impersonation — an **operator-trust limit**. (ii) The cross-wall **read crosses no host boundary** (D-6 / `cross_wall_log_read.rs:31-72`) — write and read cross **different walls**. (iii) The traceback surface has **never** been exercised against a tenant TL a real daemon wrote (`cross_wall_log_read_13_6d.rs:30-49` hand-seeds it in-process) — **this is the single most valuable thing AC2's scene can add.**

### AC4 — Explainable refusal: prove the two open slices, rule the kernel question, file the two that cannot be built

**Given** measurement shows (a) and (b) are **already built, proven and ledger-bound** (D-3),

**Then** the story does **not** re-file them as work, and instead closes the two slices that are genuinely uncovered:
- **the refused-crossing operator tail** — nothing outside `main.rs` reads `crossing_outcome_label` or the TL `status` for a refusal (`main.rs:9910-9922`, TL kind `"collective.host.cross-team-share"` at `:9891`),
- **retry / recovery** — currently **zero** coverage in any crossing test; retry must succeed **only** after a valid manifest/consent repair,

**And** ⚠ **(c) vetting lapse and (d) legal hold are findings with named, LIVE owners — NOT built here**:
- **(c)** confirmed 3/3, and stronger: `grep -rn "TrustTier|trust_tier|Vetted"` returns **zero** in `maos-loom-lite/src`, `maos-a2a-core/src`, `maos-cohort/src`, and `main.rs:9300-10000`. **There is no code path from a vetting lapse to a refused crossing.** It may be demonstrated only on the *upgrade* surface — say which, precisely.
- **(d)** `grep -rni "legal.hold" crates/maos-loom-lite/` → **zero, including tests**. Deeper than "no code": a hold is keyed by `principal_id`, and the collective tier is **principal-namespace-free by construction** (Decision D, `memory/mod.rs:180-193`). **There is no join key.** `CollectiveEraseReceipt` (`collective_memory.rs:86-89`) is `{deleted_rows, tombstone_recorded}` — **`held` is not representable**. So erase/`failed` reconciliation is judgeable; **`held` is not**, and "unauthorized hold bypass is RED" is **not constructible** without first giving collective rows a principal nexus that Decision D forbids,

**And** the story **rules the inherited kernel-cause question it is the named owner of** (D-4), in writing: the claim *"the operator can see why the wall refused"* is **allowed on the host-initiated crossing path** and **NOT allowed on the Spirit path**, because the six causes that path can produce all arrive as the single word `Transport`. State the collapse as **8 → 1**. Record the kernel widening as a named FLAG-Winston successor — **do not close it here** — and **re-assign the owner string at `check_multi_tenant_loom.rs:167-172`** (and fix `:170`'s "five") so the machinery does not name a `done` story,

**And** ⚠ **a third refusal surface is recorded** that no leg covers and no prior story named: `CrossWallRecallRefusal` (`log_recall.rs:291-304`) has **six** distinguishable variants — including `WrongDirection` — and the production `cross_wall_traceback` one-shot collapses **all six into the single token `"refused"`** (`main.rs:3075-3080`), preserving the cause only inside a free-text `"error"` string. A real operator-visible cause collapse on a shipped surface, distinct from the kernel one,

**And** a one-sided erase result is **RED** on the reconciliation that *is* buildable.

### AC5 — Run the ledger, publish the verdict, and surface what has no live owner

**Given** 13.6e built the instrument and D-2 shows the non-modification rule is unsatisfiable as written,

**Then** the rule is applied as **re-drawn**: 13.6 must not touch the **machinery** (`EvidenceState`, `EvidenceVerdict::project`, verification, artifact schema); it **must** change three **declarations** — (1) remove `reza-three-team-three-region-journey` from `NOT_REQUIRED_LEGS` (`evidence_ledger.rs:95`); (2) **re-home the leg from `check-reza-production-path` to `check-multi-tenant-loom`** (F-2), whose contract already requires `TEAM_A/B/C` and which already runs 8+ legs from the same harness file — **no new database, no `discipline.yml` substrate change, no `CONTRACTS` change**; (3) set `MAOS_LEDGER_ENFORCE=0` on the four gate steps, restoring E12-B1's two-axis invariant (F-1). **A defect in the machinery is a finding against 13.6e, not a patch here** — file three: `ledger_enforced()`'s third axis, its empty-string parse, and the three-team leg registered on a two-team gate,

**And** the journey's evidence states and `product_claim` are **recorded as published output** — with **F-1's ruling applied**, and with the D-2 trap stated: a green-but-unsigned journey reports **ABSENT**, not INDETERMINATE,

**And** a **mechanical stale-owner sweep** is executed — not by reading. It must **reuse `check_dev_record_completeness.rs:50`'s `load_sprint_status`**, whose docstring records the bug worth inheriting: a parser that keeps the trailing `# …` comment yields `"done  # …"`, matches no terminal status, and **silently skipped 58 of 141 done stories — all of Epics 9–13**. Owner phrasings in `deferred-work.md` are **not uniform** (seven forms: `Owner:`, `Owner candidate:`, `Candidate owner:`, `owner N.Nx`, `owned by X`, `owner is the next kernel-touching story`, `explicitly assigned to X`) across **three** story-token spellings. The sweep must carry a **non-vacuity control**: it reds on a planted `Owner: 13-6a` line and **finds the seven instances below** rather than returning an empty set. ⚠ **`Ownerless and open` must NOT be a defect** — six items and two ADR-059 rows use it deliberately; a sweep that reds on honesty will be disabled,

**And** the sweep's measured result is dispositioned. **Seven live stale owners** (the draft named three; four were missed):

| # | `deferred-work.md` | Item | Named owner | Status |
|---|---|---|---|---|
| 1 | `:569` | `consent_grant` records intent constant, not grant id/version/lease | "13.6a, done" | `done` — self-admitting |
| 2 | `:553` | private-tier filesystem residue | `13-5h` | `done` |
| 3 | **`:544`** | legal-hold check-then-act race | `13-5h` | `done` — **missed by the draft** |
| 4 | `:526` | fallible `record_invocation` | "the next kernel-touching story" | **four** have shipped (13.5h, 13.5i, 13.5j, 13.6c) — draft said three |
| 5 | **`:529`** | consent adapter ignores local-host membership | `13.5c` | `done` — **missed; self-executing trigger already fired** |
| 6 | **`:538`** | successful cross-wall recalls journaled as plain local `log.recall` | `13.5e` | `done` — **missed** |
| 7 | **`:641`** | `check-fkcs` `admission-path-unmodified` RED since 13.4, held advisory | "Epic-13 retro + Story 11.5" | **STALE AT BIRTH** — 11.5 `done`, retro `optional` |

⚠ **#7 is the sharpest finding in the story:** a residual authored *inside the story that built the judge* named a `done` owner on the day it was written,

**And** every **ownerless item** is dispositioned. **Re-measured — three rows changed, and two would have published a false ledger:**

| Item | Measured 2026-08-06 |
|---|---|
| ~~`check-fkcs` exits 0 on a red oracle~~ | ✅ **FIXED by 13.6e** — `check_fkcs.rs:388` now gates on `dev_blocks`. **DELETE the row; REPLACE with #7 above** |
| in-`src` kernel test modules budget-charged, CI-unexecuted | **TRUE** — strict **42 in 42 files**; broadened **44 in 43**; **17 of 17** CI invocations carry `--test <target>`, so **0** run the lib target. Report the counting rule with the number |
| no gate reconciles the kernel pin with HISTORY | **TRUE** — `grep -rn "HISTORY" xtask/src/` → **0**. `kernel-core-baseline.toml:438` says "23596 → 23517" while its own prose says "+116 from the **23401** pin" (23401+116 = 23517). **The `from` value could be arbitrary and nothing would red** |
| `check_ship_gate_completeness` never validates CI→registry | **TRUE but RE-FRAME.** 36 `EXPECTED_GATES` / 61 `check-*` jobs / **30 invisible** — all unchanged. But 13.6e added a genuine third check in the *opposite* direction (`ledger_ship_badge_problems`), so *"never validates anything CI produced"* is now **false**. Surviving defect: `EXPECTED_GATES` is hand-maintained and nothing derives it from the workflow |
| `maos-bench --bench audit_query_latency` broken since 9.1 | **TRUE** — `audit_query_latency.rs:235` (`"capability.invoke"`) vs accepted `"capability.invocation"` (`maos-audit/src/lib.rs:708`). Run **zero** times; four other benches run across 7 invocations |
| ~20 unlisted crates escape the kloc ceiling | ⚠ **BOTH HALVES CHANGED.** Data closed (49 keys / 0 unbudgeted). **But the *control* is unchanged** — `kloc_check.rs:229-235` silently skips a measured root with no key, so a crate added tomorrow escapes exactly as before: **live and unowned**. And the re-base count is now **fifth** consecutive with **three re-bases inside 13.6e alone**; `kloc.toml:371` still says "fourth" and the staged comment's "aggregate NOT touched" is reversed by the unstaged `_aggregate_hardfail` 140707→144224 |
| NFR-Ops-11 (i) and (iii) | **TRUE, asymmetric** — (iii) has `ADR-059:147`; **(i) appears in no residual register at all** |
| `maos-a2a-core` kloc grants | **THIRD consecutive unratified** — `13-6b:500`, `13-6d:344`, ladder `kloc.toml:245 = 4654`. No retro has ratified |
| 13.5g `init_schema` second pooled client | **TRUE** at `store.rs:419-433`; `:433` shadows the guarded client and `:421-428`/`:474-476` assert the opposite. Hangs at the legal `pool_size: 1` |
| `cargo-deny` effectively non-blocking | **TRUE** — `discipline.yml:38` `continue-on-error: true`; `cargo deny check` at `:120-121` rides a waiver written for build determinism |
| ⚠ **NEW** — `check-service-boundary` RED at HEAD | **D-0.** Blocking, in `aggregate`, filed nowhere |
| ⚠ **NEW** — 8 Family-B gates outside the ledger; 2 cannot express `ABSENT` | `deferred-work.md:632-633`, owner `Epic-13 retrospective` = `optional` — **weak owner** |
| ⚠ **NEW** — role-owners that do not exist | `:619`, `:634` — 13.6e's *unstaged* edit rewrote "Ownerless — needs an owner" into "maos-loom-lite performance maintainers" / "xtask gate-infrastructure maintainers". No such roles exist in `sprint-status`. **This converts ownerless into unfalsifiable** |

**And** the sweep classifies owners into **four** buckets, not two (F-4): `open`-owner = OK · `done`-owner = **STALE** · `Ownerless and open` = **OK, by design** · **`Epic-13 retrospective` = owned-but-deferred**, a distinct third bucket. `optional` is the *pre-run* status (`epic-12-retrospective` went `optional` → `done` and ratified B1–B6), so it is a **valid** owner — but retro items have **measured one-epic slippage** (E11's A1/A2/A3 → E12's B1/B2/B3), so "owned-but-deferred" must never render as a pass.

### AC6 — NFR-Scale-5 as a correctly-axed measured envelope, boundary preservation, every stale claim corrected

**Given** NFR-Scale-5 names the **host/institution** axis (`prd/non-functional-requirements.md:152`, verbatim, line confirmed) and Reza is **one** institution with three teams,

**Then** the envelope is a **documented artifact in `docs/release/`, derived from and reconciled against `check-scale-churn`** — measured **GREEN, exit 0, hermetic, `BindingClass::Blocking`** on 2026-08-06 — from `11-3-scale-envelope-25-30-host-churn.md:303`: **30 distinct fingerprints/addrs, 3/3 adversaries detected (~2.1 / ~4.5 / ~6.5 ms), max_blast_radius = 2, 16 real join/leave events over 4 rounds, 0.79 s**. ⚠ **NEVER from `a2a-churn-report.md:18-32`**, which still publishes the v0.5 canned scaffold (`host_count: 3, detection 30, blast 3, recovery 60`) that Story 11.3 **deleted** (`churn.rs:3-6`),

**And** the axis is stated explicitly: **never** an assertion that 14 institutions executed, never a re-labelling of 3 teams as 14 institutions,

**And** the envelope names what it does **not** cover — the 30-day soak (NFR-Scale-1), absolute geo-SLO, 100-host churn (Epic 14) — **and the measurement limits**: N=30 is a **host** count of in-process `tokio::spawn` endpoints in **one OS process** on a **co-located loopback** mesh, reachability is **hub-and-spoke `2×(N−1)=58` dials, not full N×N**, completing in **0.79 s** — so the floors pass by 3–4 orders of magnitude and **the falsifiers are where the teeth are** (`churn.rs:14-17`),

**And** boundary preservation holds at close: physical absence, team-key source-reflex, provenance minimum-disclosure, tenant TL isolation, duplicate/correlation reconciliation,

**And** the **final kernel baseline is verified at the measured number** — **23679, verified by execution 2026-08-06**,

**And** every stale claim is corrected. **All 16 drafted sites re-measured: 16 of 16 STILL STALE — zero fabricated requirements.** ⚠ **Three draft citations must be repaired first:**
- the churn test is `crates/maos-a2a-tcp/tests/t_11_3_scale_churn.rs` — **not `maos-bench`; that file does not exist** (cited lines 78/82/182-183/206-243 are correct *in the real file*),
- `check_multi_tenant_loom.rs` delegates the baseline at **`:1651`**, not `:1603`,
- `check_scale_churn.rs` sets `BindingClass::Blocking` at **`:362`**, not `:430`.

⚠ **ADR-055 `:166` needs softening**: it does **not** claim `(f-i)` is unassigned — it names 13.6b. The contradiction with `:164` is **"retired" vs "remains green"**, and **`:166` is the stale half**.

**Six NEW sites raise the surface from 16 to ~22:** `kloc.toml:322` ("~20 real production crates absent" — false at HEAD), `kloc.toml:345-346` (superseded by the 13.6d migration), `epics/index.md:146` ("8 stories"), `epics/index.md:142`-zone ("24 stories" — true is **37**), ADR-055 header `:3`/`:4`/`:5` (stops at 13.6a; no §section for 13.6c or 13.6e), ADR-055 `:16`/`:113`/`:151`/`:161` (four more present-tense claims naming `done` stories). Also `deferred-work.md:13` quotes a 3-element `REGISTERED_ERASURE_BACKENDS` at `memory/mod.rs:35`; HEAD has **four** at `:37`.

**Already fixed — do NOT re-file:** `epic-13:57` (the 21-count is correct), `:63` (sequencing), `:54` (the 13.6e row — the *entire* uncommitted epic diff is this one line), `sprint-status.yaml:1/:235/:236`, `epic-13-context.md:25/:52`, `13-6e…md:12`, and this file's own frontmatter. **No artifact anywhere still says 13.6e is `ready-for-dev` or 13.6 is `blocked`.** The `13-6…md:317`/`:319` change-log rows are correctly past-tense — **leave them**.

---

## Resolutions — all four forks closed by measurement, 2026-08-07

The 2026-08-06 pass recorded four forks for operator ratification. A second resolution round found **all four are determinate**: three are defects with a single correct fix, and the fourth is answered by precedent. **None is an operator preference.** Ratify the reasoning, not a choice.

### ✅ F-1 RESOLVED — the enforced lane is a defect in 13.6e, not a deadlock. It is an unsanctioned THIRD axis.

E12-B1 separated exactly **two** axes, and `gate_common.rs:31-33` states the invariant in the very file 13.6e extended:

> *"This governs **ONLY** the GA ship-gate ladder (`is_blocking_at`) — **NEVER dev-time enforcement, which is governed by `BindingClass`**."*

Dev-time enforcement is `dev_enforced_red_blocks(class, substrate_present)` (`:97-102`) — and it fires only on a **red** leg. 13.6e's `ledger_enforced()` (`evidence_ledger.rs:126-135`) hard-fails CI at HEAD — that *is* dev-time enforcement — but is governed by neither axis: it keys off **`GITHUB_ACTIONS`**. `blocks_product_claim`'s own doc concedes the dev-lane rule is "independent", then layers CI-presence enforcement on top. **That layer is a third axis, and `project_gate_binding_decay` is cited three lines above it.**

The epic already authorizes the correct behaviour (`epic-13:200`, verbatim): *"an unavailable live substrate **can remain advisory for a development lane** while its evidence state is `ABSENT`, which prohibits the Reza completion claim."* **CI is a development lane.** The refusal belongs on the **claim**, not the job exit — and the claim refusal is **already correctly declared**: `gate-registry.toml` sets `v2_2 = "blocking"` on both Family-A gates, and `check_ship_gate_completeness.rs:143-149` refuses a non-`PROVEN` claim whenever the gate is blocking at the current phase. `CURRENT_PHASE = "v1_5"`, so it is *correctly dormant today* and activates exactly when the Reza claim is made.

**Resolution (declaration-only, permitted under trap #2):** set `MAOS_LEDGER_ENFORCE=0` on the four gate steps in `discipline.yml`, restoring the two-axis invariant. **Measured:** `MAOS_LEDGER_ENFORCE=0` (also `false`, also empty) under `GITHUB_ACTIONS=true` → **exit 0**, `ledger_enforced:false`, and the ledger is **still published** with its `product_claim` intact. The instrument loses nothing: a missing ledger is still a ship-gate problem regardless of disposition (`:135-136`).

**File against 13.6e as a machinery finding (not a patch here):** `ledger_enforced()` must consult `BindingClass`/phase, not `GITHUB_ACTIONS`. ⚠ Also file the parsing defect measured in passing — `MAOS_LEDGER_ENFORCE=""` **disables** enforcement rather than falling through to the `GITHUB_ACTIONS` default.

⚠ **Consequence for AC5, and it is the honest one:** `PROVEN_LIVE_SIGNED` is unreachable in CI **by correct design**, because CI holds no operator key. The Reza journey is therefore proven on an **operator-run lane** — real 3-team Postgres plus the operator key — which is precisely what 13.6e's dirty-worktree binding exists to support. **Story 13.6 is the first story that runs that lane.** That is why no leg has ever reached `PROVEN_LIVE_SIGNED`: nobody has run it yet.

### ✅ F-2 RESOLVED — move the leg to `check-multi-tenant-loom`. It is strictly smaller and needs no new substrate.

Measured contracts (`check_loom_substrate_drift.rs:143-187`):

| Gate | `required` | Legs from `cross_team_crossing_13_6b.rs` |
|---|---|---|
| `check-multi-tenant-loom` | `TEAM_A`, `TEAM_B`, **`TEAM_C`**, `MAOS_TEST_POSTGRES` | **8+** (`:469,485,500,517,533,549,565,582`, plus `:777-790`) |
| `check-reza-production-path` | `TEAM_A`, `TEAM_B` only | **1** — the journey successor 13.6e just added (`:40`, `:168`) |

`check-multi-tenant-loom` **already requires and provisions all three team databases, each with a reader**. Moving the leg costs **zero** `discipline.yml` change, **zero** `CONTRACTS` change, **zero** new database. Growing the reza gate costs all three plus `live_substrate_present()` — and strains D-7 (*"never provision a database with no reader"*) against the reza job's own comment, written in the same commit, that it grows no third database.

13.6e's placement was the error: it registered a **three-team** control on the only ledger gate with a **two-team** substrate, while its own doc comment claims the single registration means *"the two gates cannot disagree."* **This supersedes the 2026-08-06 recommendation of option (i)**, which was made before the contract table was measured.

### ✅ F-3 RESOLVED — `check-service-boundary`'s RED is a GATE BUG. The fix is 8 lines the gate already contains.

The gate inspects `cfg` at exactly two lines in the file — `:1094` and `:1197` — **both inside the P4 walk**. The main surface walk (`walk_mod:331`, `walk_inline_mod_item:425`) has **none**. The P4 rule is:

```rust
a.meta.path().is_ident("cfg")
    && ml.tokens.to_string().contains("test")   // → skip this mod
```

`#[cfg(any(test, debug_assertions))]` contains `"test"`, so **the P4 walk would already skip `spill_test_faults`.** The main walk never received the rule, and `spill_test_faults` is the first `pub mod` under a test-bearing cfg predicate to reach it.

**Resolution: port the existing cfg-skip from `walk_p4_mod`/`walk_p4_inline_item` into `walk_mod`/`walk_inline_mod_item`.** This aligns the three mechanisms that currently disagree (`kloc_check.rs:189` excludes; P4 walk would skip; main walk counts).

⚠ **Do NOT add the five symbols to `xtask/kernel-api-classes.toml`.** That would bless test-fault-injection functions as permanent public kernel API under a real class, require invariant-lock review, and assert something false — they do not exist in a release build. Classifying them would be a claim standing in for a control.

### ✅ F-4 RESOLVED — "Epic-13 retrospective" IS a valid owner; `optional` is the pre-run state.

Measured: `epic-10-retrospective`, `epic-11-retrospective`, `epic-12-retrospective` are **all `done`**. Git history shows `epic-12-retrospective: optional` → `done`, and that retro ratified **six** items (B1–B6) — including the Option-C leg-level gate binding this story's entire F-1 argument rests on. **`optional` is the normal pre-run status, not "may be skipped."**

⚠ **Honest caveat to record with the disposition:** retro ownership has **measured slippage of one full epic** — E11's A1/A2/A3 slipped into E12 as B1/B2/B3. So "Owner: Epic-13 retrospective" is *valid but soft*. AC5 should classify it as **owned-but-deferred**, distinct from both `done`-owner (stale) and `Ownerless` (honest) — a third bucket, not a pass.

### Scope note — the AC5 sweep is in scope, and it fits

Trap #1's actual wording is *"it never invents a missing mechanism **inside the journey harness**"* (`epic-13:57`). A stale-owner sweep is **evidence tooling in `xtask`**, not a journey mechanism — the same category as every gate this epic shipped. **Budget confirmed by execution:** `xtask` measures 35226 against a 35931 ceiling = **705 lines headroom**; a ~250-line sweep fits with no new grant. ⚠ `load_sprint_status` already exists in **two** copies (`check_dev_record_completeness.rs`, `check_review_findings_resolved.rs`) — **extract it, do not write a third**, or this story recreates the "change both or they drift" hazard 13.6e just eliminated for `LegResult`.

---

## Traps

1. **Do not build a mechanism.** The single rule that makes this story worth having. AC4(c), AC4(d), the kernel collapse, and AC2's hub option are its four live tests.
2. **Do not patch the ledger machinery.** Declarations only, per AC5's re-drawn rule. A machinery defect is a finding against 13.6e.
3. **Do not build a second multi-daemon harness.** `cross_team_crossing_13_6b.rs:1204` exists; extend it. Add a **new** 3-team manifest builder rather than mutating `manifest_with` (7 call sites, 18 tests).
4. **Institution ≠ team ≠ host.** Three axes. Re-labelling 3 teams as 14 institutions repeats, inside the closer, the conflation the epic caught for NFR-Ops-11.
5. **Never derive the envelope from `a2a-churn-report.md`.** It holds the constants 11.3 deleted.
6. **Never re-can a fixture, never silently relax a floor.** The 30 000 µs roundtrip floor is untouched; the paired clean/injected median-delta oracle guards it. A future RED is a finding, not a licence.
7. **The stale-owner sweep must be mechanical, must reuse `load_sprint_status`, and must carry a non-vacuity control.** A hand-rolled parser that keeps the `# …` comment reports zero stale owners and looks green — it already did this to 58 of 141 stories.
8. **`Ownerless and open` is honesty, not a defect.** A sweep that reds on it will be disabled.
9. **`skipped ≠ passed`.** Live legs `.expect()` their own env var (13.5g pattern).
10. **One `#[test]` per `--exact` leg** — the anti-vacuity oracle is `"running 1 test"` + `"1 passed"` and is blind to a null assertion.
11. **Proven-red per limb**, byte-identical restore, **serialized** — do not batch.
12. **A green-but-unsigned journey reports `ABSENT`, not `INDETERMINATE`** (D-2). Never key an evidence claim on `evidence_state` alone.
13. **The chain is two independent originations, not a transitive flow.** Say so, or repeat the failure shape.
14. **Carry 13.5g's open finding:** `store.rs:419-433`. If you touch `init_schema`, fix it.
15. **CI is RED at HEAD** (D-0) and **will be red on four more jobs once 13.6e is pushed** (D-1). **Both are inherited, both are resolved (F-3, F-1), and T0a/T0b fix them first.** Push and re-run before any evidence claim; do not mistake either red for your own regression — and do not "fix" D-1 by patching `evidence_ledger.rs`.
16. **`cargo run -q -p xtask -- <cmd>`.** No `cargo xtask` alias. **`abi-diff` is not evaluable on a dirty worktree** — `git stash` first.
17. **The §A6 review must run on a different model than the dev pass.** 13.6c's round-2 and 13.6d's reviewer both matched their dev pass, structurally disarming the Test-Infra layer. `epic-13:13` calls this net **non-degradable**, and this is the epic's last adversarial boundary.
18. **Attribution hazard.** `b568a052` is titled `13-6a-authenticated-team-identity` but contains **zero** 13.6a work — it is 13.6c review round 2. `a414f922` carries a pre-split name. **Three story identities on one delta**; a ledger keyed on commit titles will mis-attribute.
19. **`PROVEN_LIVE_SIGNED` requires the operator lane, not CI.** If you find yourself wanting a key in CI, re-read F-1: CI is a *development* lane by `epic-13:200`, and a CI that holds the operator key would be theatre (`evidence_ledger.rs:984`).
20. **Do not re-file a resolved fork as an open question.** F-1…F-4 are closed by measurement, with citations. If you disagree, disprove the citation — do not reopen the fork by restating it.

---

## Tasks

- [x] **T0a (D-0/F-3) — DONE `accf763c`.** Shared `is_test_cfg_mod` ported into both surface walks; the mis-captured `crypto::tests::MockCryptoProvider` removed from the ABI baseline (371→370, the only `::tests::` entry). `check-service-boundary: PASSED (0 violations)`. Proven a real control — a planted non-cfg-gated `pub fn` still reds it, restored byte-identically. Filed in `deferred-work.md`. `kernel-api-classes.toml` untouched.
- [x] **T0b (D-1/F-1) — SUBSUMED, no workflow change needed.** Rather than declaring `MAOS_LEDGER_ENFORCE=0` around the defect, 13.6e was **reopened and the root cause fixed** (`c45df0be`): `blocks_product_claim` now blocks `INDETERMINATE` only when the leg is actually RED, so a green-but-unsigned leg refuses the **claim** without blocking a **lane**. `ABSENT` semantics are unchanged — measured on the enforced lane with no Postgres: **exit 1**, 83 `PROVEN_BLOCKING` + 15 `ABSENT`, which is the correct "substrate did not come up" block. All three machinery findings were fixed at source, not filed.
- [x] **T1 (AC1) — DONE.** All three topology-fraud limbs are now pure oracles (`cross_region_live.rs::topology_fraud`) with hermetic proven-red controls in the drift-gate idiom (plant one defect in an in-memory clone, assert not-green AND the token), each registered as its own `Blocking` leg on `check-multi-tenant-loom` so a break reds exactly the limb that broke. The drift gate's **value blindness is CLOSED**: a new `topology-value-distinctness` leg reconciles the exported connection strings **axis-scoped** (`TOPOLOGY_AXES` region/team pairwise-distinct; cross-axis collisions must appear in `RATIFIED_ALIASES`, which is itself held non-vacuous). Five proven-red controls, including the false-red guard D-6(b) warned about. Local setup documented for all four substrate jobs in `docs/testing/local-loom-substrate.md`; both stale singular-`MAOS_TEST_POSTGRES` headers corrected. AC1(d) verified already-closed by 13.6c and NOT re-filed.
- [x] **T2 (AC2) — DONE ON THE PRODUCTION PATH.** The harness starts the three-daemon A→B→C chain and runs all three CLI processes under one signed manifest, authority, base seed, shared `MAOS_HOME`, and three distinct datnames. All six processes now reach their production entries: tenant mode classifies the `traceback` and cohort-backed `collective-erase` one-shots as bounded-refreshable tenant-map modes, so neither fails at composition-root construction any more. The journey emits its attestation and the required ledger leg is `PROVEN_LIVE_SIGNED`.
- [x] **T3 (AC2) — DONE, seven sites, per-limb.** The chain assertions became a pure oracle `crossing_chain_problems`; `every_crossing_wiring_site_is_individually_falsifiable` unwires each site in an **in-memory clone** and asserts the problem naming that exact site fires. Site 7 is split: 7a the applier-port CONSTRUCTION, 7b the paren-balanced hand-off to `build_cohort_a2a_daemon_runtime` (dropping the argument while keeping `let crossing_port = …` used to stay green). Restore is by construction and re-asserted byte-identically at the end.
- [x] **T4 (AC3) — DONE, one measured limit closed.** Minimum disclosure is judged on the rows that actually crossed: five provenance fields, a re-attestation marker of exactly `{source_region, merkle_root}`, no payload bytes and no base seed in the marker. The read side meets a producer through the SHIPPED `CrossWallLogReadAdapter` reading team B's daemon-written tenant TL, and every entry discloses exactly six fields with `payload_available` a boolean. The `maos traceback` CLI is now **reachable in production** — the journey drives it to a JSON `ok` outcome and the direct-adapter disclosure proof is retained beside it.
- [x] **T5 (AC4) — DONE.** `refused_crossing_is_operator_visible_and_retry_needs_a_consent_repair` drives a real two-daemon refusal, reads the emitter's TL `status` field for a REFUSED crossing (zero prior readers anywhere), retries unrepaired and gets the same typed refusal, then re-signs the manifest WITH the grant and only then lands the row. (a)/(b) were not re-filed.
- [x] **T6 (AC4) — DONE.** The kernel question is ruled in writing as **8 → 1** naming the six Spirit-path-reachable causes; the owner string at `check_multi_tenant_loom.rs` is re-assigned to `Epic-13 retrospective` (it named Story 13.6 itself) and its "five" corrected to "eight" there and in `evidence_ledger.rs`. (c) and (d) filed with live owners. The `CrossWallRecallRefusal` **6 → 1** collapse is recorded as a second, independent operator-visible cause erasure.
- [x] **T7a (AC5) — DONE, SIGNED ON EVIDENCE.** The journey leg remains required and homed on `check-multi-tenant-loom`. With all six processes dispatching, the operator-lane run at `9160eecb` records `reza-three-team-three-region-journey=PROVEN_LIVE_SIGNED` and `product_claim: PROVEN` on every one of the four substrate gates. The rejected 2026-08-08 artifacts remain retained as `.pre-review.json` history and are non-ingestible by filename.
- [x] **T7b (AC5) — REVIEW-CORRECTED.** `load_sprint_status` is single-sourced into `xtask/src/sprint_status.rs` (the inferior copy deleted, not a third written). The sweep lives inside the existing `check-dev-record-completeness` gate — no new gate. Eight owner phrasings, three token spellings, four buckets, and a non-vacuity control that reds on a planted `Owner: 13-6a`. Explicit `Ownerless and open` rows are classified but remain honest and non-failing as AC5 requires. Current gate: 31 owner assertions, 16 owned-but-deferred rows, zero violations.
- [x] **T8 (AC6) — DONE, INSTITUTION AXIS MEASURED.** `docs/release/v2.2-capacity-envelope.md` records the 30-host churn drill and explicitly separates the 2/3-endpoint adversary metrics; it supersedes only v1.5's 25-host churn limitation. The separate 14-institution Cortex target is now **measured** by `cortex_fourteen_institution_isolation_live` (fourteen independent authorities, signed V4 manifests, physical per-institution datnames, typed cross-institution consent refusal, cross-authority clone rejection, removal independence), registered as the required `cortex-fourteen-institution-isolation` leg. The doc states what that proves and what it still does not (14-institution throughput SLOs, 30-day soak, geo-distribution, full N×N mesh, 100-host churn).
- [x] **T9 (AC6) — DONE.** kloc.toml `:322`/`:345-346`/`:371`, `epics/index.md` (Epic 13 = 21 stories, v2.2 total = 37 = 7+21+9), ADR-055 header + `:16`/`:113`/`:151`/`:161`/`:166` and two NEW sections (§4e 13.6c substrate, §4f 13.6e ledger). Kernel baseline **verified at the measured number: 23679 actual == 23679 pinned**. The HISTORY gap is RECORDED, not fixed — closing it needs a new gate, which this story may not add.
- [x] **T10 — DONE.** See Completion Notes for the full gate table and the dev model.

### Review Findings

- [x] [Review][Patch] Correct AC3 to the shipped cross-wall disclosure schema [_bmad-output/implementation-artifacts/13-6-reza-cortex-journey-closer-nfr-scale-5.md:214] — preserved `peer_spirit_pid` and `payload_available`.
- [x] [Review][Patch] Required journey signs success when two CLI legs never dispatch [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1981]
- [x] [Review][Patch] Journey blesses the one-sided erase AC4 requires to red [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:2397]
- [x] [Review][Patch] CLI children inherit stale mode switches [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1921]
- [x] [Review][Patch] Accept-only daemons inherit share requests [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1366]
- [x] [Review][Patch] Per-limb falsification mutates multiple wiring sites [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:443]
- [x] [Review][Patch] Crossed-row oracle accepts arbitrary intent lineage [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1936]
- [x] [Review][Patch] Daemon stderr reader exits after readiness [crates/maos-bin/tests/cross_team_crossing_13_6b.rs:1306]

The topology-control finding was dismissed after full-context verification: `check_loom_substrate_drift` loads `real_workflow()`, plants region/team defects in cloned configuration, and holds the real topology green; the physical-absence oracle is data-state-specific and is paired with the live database observation.

#### Chunk 2 — xtask gates

- [x] [Review][Patch] Missing deferred-work register disables the owner gate [xtask/src/check_dev_record_completeness.rs:527]
- [x] [Review][Patch] Completed retrospective owners remain exempt forever [xtask/src/check_dev_record_completeness.rs:174]
- [x] [Review][Patch] Topology oracle accepts missing or empty database values [xtask/src/check_loom_substrate_drift.rs:741]
- [x] [Review][Patch] Closed-heading substring matching hides open sections [xtask/src/check_dev_record_completeness.rs:75]
- [ ] [Review][Defer] Published ledgers may omit required journey legs [xtask/src/evidence_ledger.rs:1247] — Owner: reopened Story 13.6e; this story's attempted verifier patch was reverted to preserve AC5's machinery boundary.

Verification: the complete xtask suite passed 443 tests with 1 ignored; the real topology gate is green; the real dev-record gate passes with 31 classified assertions, 16 owned-but-deferred rows, and zero violations.

#### Chunk 3a — documentation and tracking

- [x] [Review][Patch] Ledger verifier patch crosses AC5's machinery boundary — reverted the Story 13.6 verifier change, reopened Story 13.6e, and filed the mandatory-leg omission there.
- [x] [Review][Patch] Restore AC5 ownerless semantics without hiding ownerless rows [xtask/src/check_dev_record_completeness.rs:121]
- [x] [Review][Patch] Sprint tracking publishes the rejected PROVEN result [_bmad-output/implementation-artifacts/sprint-status.yaml:236]
- [x] [Review][Patch] Linked operator evidence remains falsely current [_bmad-output/implementation-artifacts/13-6-reza-cortex-journey-closer-nfr-scale-5.md:491]
- [x] [Review][Patch] Local PostgreSQL recipe omits pgvector [docs/testing/local-loom-substrate.md:47]
- [x] [Review][Patch] Docker provisioning omits the database password [docs/testing/local-loom-substrate.md:56]
- [x] [Review][Patch] Local cluster provisioning races PostgreSQL startup [docs/testing/local-loom-substrate.md:65]
- [x] [Review][Patch] Gate commands lack a workspace-root precondition [docs/testing/local-loom-substrate.md:97]
- [x] [Review][Patch] Runbook misstates unsigned live evidence as ABSENT [docs/testing/local-loom-substrate.md:126]
- [x] [Review][Patch] Operator evidence pipeline masks gate failures [docs/testing/local-loom-substrate.md:119]
- [x] [Review][Patch] Operator recipe never requires product claim PROVEN [docs/testing/local-loom-substrate.md:119]
- [x] [Review][Patch] Alias claims exceed the single-server oracle [docs/testing/local-loom-substrate.md:34]
- [x] [Review][Patch] Host churn cannot supersede the unmeasured 14-institution limit [docs/release/v2.2-capacity-envelope.md:123]
- [x] [Review][Patch] Full-mesh dial formula is arithmetically wrong [docs/release/v2.2-capacity-envelope.md:98]
- [x] [Review][Patch] Runbook attributes schema initialization to the constructor [docs/testing/local-loom-substrate.md:73]
- [x] [Review][Patch] Capacity table conflates N=30 churn with small-mesh adversary metrics [docs/release/v2.2-capacity-envelope.md:30]

#### Chunk 4a — current evidence

- [x] [Review][Patch] Story misclassifies the current evidence index as superseded [_bmad-output/implementation-artifacts/13-6-reza-cortex-journey-closer-nfr-scale-5.md:512]
- [x] [Review][Patch] Dirty-worktree ledger binding is self-invalidating [_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-multi-tenant-loom.json:20]
- [x] [Review][Patch] Current evidence publishes workstation and key paths [_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-multi-tenant-loom.json:1540]

#### Chunk 4b — superseded historical evidence

- [x] [Review][Patch] Superseded ledgers retain ingestible canonical filenames [_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-cross-region-consensus.pre-review.json:121]
- [x] [Review][Patch] Tenant history exposes the operator workstation path [_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-multi-tenant-loom.pre-review.json:3]
- [x] [Review][Patch] Reza history exposes the operator workstation path [_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-reza-production-path.pre-review.json:3]

#### Closure round — production completion, 2026-08-11

Implemented against the plan that answered "why is this still in progress": the four
`NOT_PROVEN` causes were removed at the source and the claim was re-earned on a clean commit.

- [x] [Closure] Published-ledger omission accepted a partial leg set — `validate_against` now requires the exact gate-owned set, derived per gate from the same declarations the gate executes; missing and unknown legs fail before claim comparison [xtask/src/evidence_ledger.rs]. Closes the finding that reopened Story 13.6e.
- [x] [Closure] `traceback` and cohort-backed `collective-erase` one-shots failed at tenant-map construction — both are now bounded-refreshable tenant modes; cohort-less invocation still fails closed [crates/maos-bin/src/main.rs].
- [x] [Closure] `collective-erase` left the source origin row — `CrossTeamCrossingControl::Erase` now reconciles both sides over the authenticated intake path [crates/maos-bin/src/cross_team_crossing.rs].
- [x] [Closure] The 14-institution Cortex target was unmeasured — `cortex_fourteen_institution_isolation_live` measures it on the live substrate and the required `cortex-fourteen-institution-isolation` leg carries it [crates/maos-bin/tests/cross_team_crossing_13_6b.rs].

##### Adversarial review of the closure (four rounds, three layers each)

- [x] [Review][Patch] [HIGH] Erase authorized only by a team-level grant, deleting arbitrary rows — the origin side now requires its own journaled `collective.host.cross-team-share` provenance for the exact operation before any deletion.
- [x] [Review][Patch] [HIGH] Local delete committed before reconciliation, making retries unrecoverable — reconciliation ACKs now precede the local delete; remote tombstones make retries idempotent.
- [x] [Review][Patch] [HIGH] Origin resolution and deletion could target different physical rows — the crossed branch deletes exactly the resolved crossed row.
- [x] [Review][Patch] [HIGH] Erase travelled under the `collective:share` read-only wire class — erase frames carry `collective:erase` + `IntentClass::Standard`; both axes are validated at decode and the route allowlists distinguish them.
- [x] [Review][Patch] [HIGH] Provenance was locator-text based and host-local — it is now bound to a CSPRNG operation id and a redaction-stable locator digest, and reconciliation routes only to the host that journaled the share.
- [x] [Review][Patch] [HIGH] Operation metadata rode the unique/tombstone address, breaking tombstone dominance — metadata moved to additive nullable `cross_*` columns; the physical address is stable.
- [x] [Review][Patch] [HIGH] Generation check and deletion were not atomic — `erase_at_generation` compares and deletes under one advisory-locked transaction; a rewritten row yields `StaleGeneration`, and an absent row records the dominating tombstone before ACK.
- [x] [Review][Patch] [HIGH] `emitter_host` was peer-asserted — it must equal the TLS-authenticated frame host.
- [x] [Review][Patch] [MEDIUM] The institution live test leaked databases on any failure path — a drop guard force-drops every created database on unwind.
- [x] [Review][Patch] [MEDIUM] The team-guard chokepoint test hard-coded a signature list and count — it now enumerates every public `LoomLiteStore` method and reds on any unguarded, non-exempt one.
- [x] [Review][Dismiss] Legacy `:xmeta:` row migration — that encoding existed only in this session's intermediate commits (`0d8ea7af`..`f4611f2e`) and never shipped; no deployed database can contain it. Local test residue was removed.

##### Budget close-out finding — self-review, 2026-08-11

- [x] [Review][Patch] [HIGH] **The first close declared `done` over a red blocking gate.** `kloc-check` is a `discipline.yml` job (`:154`) in the aggregator's `needs` (`:3228`), so the closure commit would have failed CI. Three ceilings were breached (`maos-bin` +756, `maos-loom-lite` +430, `xtask` +185) and the closing verification claimed "full xtask suite green" from `cargo test -p xtask` alone. The story's own Budget section had ordered the re-measurement (*"Re-measure after this story's code"*) and it was skipped. Fixed: re-measured at both the session entry and closure commits, ratified-formula grants recorded in `xtask/kloc.toml` with named drivers, FLAG-Winston authorized post-measurement, `kloc-check` PASSES at aggregate 143734 / 144224. **Process lesson for the retrospective: a story that names a verification step in its own Budget must run that step in its close checklist — "the suite passed" is not "the gates passed".**
- [x] [Review][Patch] [HIGH] **A second blocking gate was also red and also unrun: `check-composition-root-completeness`.** Found by the same sweep, after extracting all 64 `cargo run -p xtask -- <gate>` invocations from `discipline.yml` and executing every one. The closure's `with_erase_reconciliation` wiring added a second `CrossTeamConsentAdapter::new` in `main.rs` (`:2831` store setup, `:9566` crossing-port construction), which the gate flags as a possible duplicate shared-state instance. Verified PASS at session entry `c6977b35` (11 adapters / 20 constructions) and FAIL at close — introduced here. Resolved by exemption rather than by threading the instance through the daemon-runtime builder's signature, because the adapter is `struct CrossTeamConsentAdapter { state: Arc<CohortManifestState> }` — one field, both sites cloning the same `bootstrap.state`, so two instances have nothing to diverge on and re-shaping a signature the crossing legs assert against buys zero behavioural change. **The exemption is machine-checked, not asserted:** `cross_team_consent_adapter_is_a_pure_shared_state_projection` (`crates/maos-bin/tests/cross_team_consent_13_3.rs`) pins the adapter to exactly that one field. Proven red — a planted `planted_cache: u64` field fails it with the exemption's own reasoning in the message; restored, green. All 58 hermetic gates now PASS, plus `abi-diff` under its CI arguments; the four substrate gates need Postgres.

---

## Dev Notes

### Budget — verified by execution 2026-08-06

- **kernel-core: ZERO expected @ 23679.** Verified: `maos-kernel-core/src = 23679 lines, pinned 23679`. *"kernel-core ZERO" ≠ "zero delta"* — state both.
- **fkcs:** frozen `23081`, byte-untouched.
- **kloc:** ⚠ **THE RE-MEASUREMENT THIS LINE ORDERED WAS NOT DONE AT FIRST CLOSE — see the 2026-08-11 budget close-out below.** Design-time state was `kloc-check` **PASSES** at aggregate **141396 / 144224**; `xtask 35226/35931`. ⚠ 13.6e re-based `xtask` **twice** and moved `_aggregate_hardfail`; do not spend its grant on unrelated surfaces, and expect the retro to ask about the fifth consecutive re-base.
- Two kernel measurements diverge **by design**: `spill_test_faults.rs` (107 lines) is kloc-excluded but baseline-counted. **23679 physical vs 18210 logical** — and it is also the source of D-0.

#### Budget close-out — re-measured 2026-08-11 (the step this story ordered and its first close skipped)

The closure's production code breached **three** ceilings. Measured at the session's entry commit `c6977b35` and at the closure commit `5ebf83d6`, formatted tree both times:

| Crate | Entry `c6977b35` | Close `5ebf83d6` | Old ceiling | Over | Granted |
|---|---|---|---|---|---|
| `maos-bin` | 15033 | 15860 | 15104 | +756 | **16178** |
| `maos-loom-lite` | 4756 | 5277 | 4847 | +430 | **5383** |
| `xtask` | 35372 | 36116 | 35931 | +185 | **36839** |
| aggregate | — | 143734 | 144224 | — | untouched (not breached) |

All three entered the session **under** ceiling (headroom 71 / 91 / 559) and left it over. Grants computed by the ratified formula `formatted_measured + max(100, ceil(2%))` and **FLAG-Winston authorized by Lunarpulse on 2026-08-11 after the code existed and was measured** — never on an estimate. Reduction was evaluated and rejected: the +2092 lines are `src/` only (kloc excludes `tests`), and each block closes a named HIGH finding from the four review rounds — the erase provenance gate, atomic generation-conditional delete, tombstone-address stability, tenant-mode production dispatch, and the exact gate-owned leg set. Deleting any of it re-opens a finding. `kloc-check` **PASSES** after the grants: aggregate 143734 / 144224.

**This is the sixth consecutive `xtask` re-base.** The Epic-13 retrospective owns the question the streak raises: whether the gate machinery's growth rate is itself the defect.

### What 13.6e delivered (verified against code, not its story file)

Four states in `gate_common.rs` with wire spellings pinned by test; sealed `EvidenceVerdict` whose inner field is private to `gate_common`, so **a leg cannot name a state it did not derive**; `required` as a **rule** (`leg_is_required` / five-name `NOT_REQUIRED_LEGS`), not a per-leg field — so **13.6 cannot mark a leg required at its construction site**; 30 harness guards across **7** `#[path]`-including test files (verified: 18+6+2+1+1+1+1); artifacts at `tests/reports/evidence-ledger-<gate>.json`; four `upload-artifact` producers + one `download-artifact` consumer with full re-validation (binding reconstruction, reprojection, claim recomputation).

**There is no new xtask subcommand.** `git diff HEAD -- xtask/src/main.rs` is three lines (`mod evidence_ledger;`). "Run the ledger" = run the four gates; "publish the verdict" = `check-ship-gate-completeness`.

⚠ **New cascade risk:** with `needs:` + `if-no-files-found: error`, any producer that dies before writing its ledger reds `check-ship-gate-completeness` → reds `v1.0-ship-gate`. Producer infra flakiness now reaches the ship gate.

### Measured starting state (2026-08-06, working tree, no Postgres, no operator key)

| Gate | legs | states | `product_claim` |
|---|---|---|---|
| `check-cross-region-consensus` | 5 | 1 PB, 4 ABSENT | `NOT_PROVEN(4)` |
| `check-multi-region-slo` | 5 | 2 PB, 2 ABSENT, 1 INDETERMINATE | `NOT_PROVEN(3)` |
| `check-multi-tenant-loom` | 97 | 83 PB, 14 ABSENT | `NOT_PROVEN(14)` |
| `check-reza-production-path` | 77 | 71 PB, 6 ABSENT | `NOT_PROVEN(4)` |

`operator_key_available: false` on all four. **No leg in this repo has ever reached `PROVEN_LIVE_SIGNED`.**

### References

- [Source: `epic-13-reza-cortex-v2-2.md#176-182`] — the original 13.6 AC sketch.
- [Source: `epic-13-reza-cortex-v2-2.md#200`] — evidence state vs enforcement class.
- [Source: `epic-13-reza-cortex-v2-2.md#57`] — *"13.6 is last and only judges; it never invents a missing mechanism."*
- [Source: `epic-13-reza-cortex-v2-2.md#84-88`] — the operator/team conflation precedent.
- [Source: `prd/non-functional-requirements.md#152`] — NFR-Scale-5, verbatim.
- [Source: `11-3-scale-envelope-25-30-host-churn.md#303`] — the **real** measured N=30 numbers.
- [Source: `13-6e-judge-machinery-evidence-ledger.md#463-486`] — 13.6e's completion notes, incl. the self-declared T4 deviation.
- [Source: `deferred-work.md#526,529,538,544,553,569,641`] — the seven stale-owner instances.
- [Source: `13-6d-cross-wall-recall-production-initiator.md#370`] — Residual 1: *"look for stale owners, not just missing ones."*
- [Source: `13-6c-evidence/SUMMARY.md#35-42`] — the prose-only topology-fraud proven-red.

---

## Dev Agent Record

### Agent Model Used

`anthropic/claude-opus-5` (dev pass, 2026-08-08). ⚠ **Trap 17: the §A6 review
net MUST run on a different model.** 13.6c's round-2 and 13.6d's reviewer both
matched their dev pass, structurally disarming the Test-Infra layer;
`epic-13:13` calls this net non-degradable and this is the epic's last
adversarial boundary.

### Debug Log References

- **Current evidence index:** `13-6-evidence/SUMMARY.md`.
- **Current unbound observation:**
  `13-6-evidence/review-observation-check-multi-tenant-loom.json`
  (`product_claim: NOT_PROVEN`, required journey `ABSENT`). No raw dirty-worktree
  ledger, artifact references, workstation paths, or key paths are published.
- **Superseded operator history:** all four sanitized ledgers use
  `.pre-review.json` filenames and carry `SUPERSEDED_PRE_REVIEW`; the
  gate/filename invariant therefore rejects them if copied into `tests/reports`.
- **Substrate:** Postgres 17 on one server, four databases (`maos_shared` +
  `maos_team_{a,b,c}`), provisioned exactly as
  `docs/testing/local-loom-substrate.md` documents.
- **Live crossing suite:**
  `cargo test -p maos-bin --features network --test cross_team_crossing_13_6b -- --ignored --test-threads 1`
  → 4 passed, 0 failed (the composed journey, the refusal/retry leg, the
  two-daemon witness, the destination-adapter leg).
- **Hermetic suite for the same binary:** 17 passed, 0 failed, 4 ignored.
- **`cargo test -p xtask --bin xtask`:** 438 passed, 0 failed, 1 ignored.
- **Full regression, `cargo test --workspace --lib --bins --tests`:** 402
  suites, **3613 passed, 0 failed**, 96 ignored.
- ⚠ `--all-targets` additionally builds benches and fails on
  `maos-bench --bench audit_query_latency` with
  `UnknownKind("capability.invoke")`. **Pre-existing at HEAD** —
  `crates/maos-bench` is untouched by this story — and it is the AC5
  ownerless row this story re-measured and dispositioned (the accepted kind
  is `"capability.invocation"`, `crates/maos-audit/src/lib.rs:675`).
- ⚠ `--all-features` fails at HEAD on `maos-registry`'s
  `registry_roundtrip_test` (`AdmissionConfig` missing
  `runtime_crypto_provider` / `runtime_provider_endpoint`). Also
  pre-existing and out of scope.

### Completion Notes List

**The corrected headline.** The required Reza journey is `ABSENT`, not
`PROVEN`. Review found that `collective-erase` and `traceback` were counted as
journey processes even though both failed before their production dispatches.
The corrected harness withholds its signed journey record. A current
`check-multi-tenant-loom --json` smoke run exits 0 and reports
`product_claim: NOT_PROVEN(...reza-three-team-three-region-journey=ABSENT)`.
The earlier operator artifacts describe the rejected pre-review oracle.

**T1 — substrate close-out (AC1).** The three topology-fraud limbs became pure
oracles with hermetic proven-red controls; each is now its own `Blocking` leg,
so a break reds exactly the limb that broke. The drift gate compared KEYS and
never VALUES; the new `topology-value-distinctness` leg closes that
**axis-scoped** — the region and team axes are pairwise distinct within
themselves, cross-axis collisions must be named in `RATIFIED_ALIASES`, and the
allowlist is itself held non-vacuous so an exemption nobody re-earned reds. The
false-red D-6(b) warned about is guarded by a control that asserts the REAL
topology stays green. AC1(d) verified already-closed by 13.6c; not re-filed.

**T2/T3/T4 — the composed scene (AC2/AC3).** Six processes: three daemons
chained A→B, B→C, plus `maos run <researcher-manifest> --once`,
`MAOS_ONE_SHOT=collective-erase`, and `maos traceback --team team-b`. The chain
is **two independent originations, not a transitive flow**, and the test
asserts that rather than only saying it. Zero production lines changed. Seven
wiring sites are individually falsifiable against in-memory clones, including
the seventh the previous draft omitted — the applier-port construction, split
into 7a (constructed) and 7b (paren-balanced hand-off), because dropping the
argument while keeping `let crossing_port = …` used to stay green.

**Two measured findings the scene produced, both filed, neither fixed
(trap 1).** On a tenant host, `MAOS_LOOM_POSTGRES` forces `MAOS_LOOM_HOME_TEAM`,
which forces a tenant-map source, which only the cohort bootstrap provides,
which `TenantMapAdapter::new` refuses outside `cohort-a2a-daemon` / `run --once`.
So **`MAOS_ONE_SHOT=collective-erase` and `maos traceback --team <T>` have NO
reachable configuration** — a consented, served cross-wall read is impossible in
production. Both refusals remain pinned by the unsigned diagnostic test, while
the required ledger successor stays `ABSENT` until the production entries are
reachable. The read side itself IS exercisable and Story 13.6
exercised it: the shipped `CrossWallLogReadAdapter` read team B's tenant
Transparency Log **written by a real daemon** — the first producer/consumer
meeting on that surface — disclosing exactly six fields with no payload bytes.

**A pre-existing RED live leg, repaired.** `live_destination_adapter_applies_and_refuses_expected_shapes`
failed at HEAD on any live substrate (verified by reverting the file and
re-running): its stores declared `home_team` with no tenant map, so
`init_schema` died at `TenantMapStale` before a single assertion ran. A leg that
could never be green is a null control; it now carries the production wiring
from the same signed manifest state and passes.

**T5 — the refused-crossing tail and retry (AC4).** Nothing outside `main.rs`
had ever read `crossing_outcome_label` or the emitter's TL `status` for a
refusal, and `retry|recover|repair` had zero hits across all three crossing test
files. Both are covered by one leg that refuses, retries unrepaired to the SAME
typed refusal, then re-signs the manifest with the grant and only then lands the
row — `["crossing_consent_denied", "crossing_consent_denied", "crossing_applied"]`
read back off the durable audit surface.

**T6 — the rulings (AC4).** The kernel collapse is **8 → 1**, and on the Spirit
path the six reachable causes are `MapStale`, `ConnectionMismatch`,
`UnmappedSpirit`, `AttestationInvalid`, `PartitionRefused`,
`ErasureTombstoneDominates` (`ConsentDenied` has no production constructor;
`Other` is a free-text fallback). The claim *"the operator can see why the wall
refused"* is ALLOWED on the host-initiated crossing path and NOT on the Spirit
path. The owner string that named Story 13.6 was re-assigned to
`Epic-13 retrospective`, and its "five" corrected to "eight" in both the gate
and `evidence_ledger.rs`. `CrossWallRecallRefusal`'s **6 → 1** collapse into the
token `"refused"` is recorded as a second, independent operator-visible erasure.
(c) and (d) filed with live owners and NOT built.

**T7b — the sweep (AC5).** `load_sprint_status` is single-sourced (the inferior
copy deleted, not a third written). The sweep runs inside the existing
`check-dev-record-completeness` gate — **no new gate**. Eight owner phrasings,
three token spellings, four buckets, a non-vacuity control that reds on a
planted `Owner: 13-6a`, and a closed-section filter so historical prose does not
red. First run: **20 violations**, including all seven the story predicted.
Every one dispositioned; two turned out to be CLOSED and were recorded as such
with citations (`:538` by 13.6d, `:553` by 13.5i). The current gate PASSES
with 31 classified owner assertions, 16 owned-but-deferred rows, and zero
violations; explicit ownerless rows remain visible and non-failing by AC5.

**T10 — verification.** Rows not explicitly marked current are pre-review
measurements retained for audit history. The current review smoke is the
`check-multi-tenant-loom` row.

| Gate | Result |
|---|---|
| `check-kernel-baseline` | PASSED — `maos-kernel-core/src = 23679`, pinned 23679 (**ZERO kernel-core Δ**) |
| `kloc-check` | PASSED — aggregate 141992 / 144224; `xtask` 35797 / 35931 (134 lines headroom, no new grant) |
| `check-service-boundary` | PASSED (0 violations) |
| `check-loom-substrate-drift` | PASS — 4 env-consistent, 4 byte-identical service blocks, **4 axis-scoped topologies physically distinct** |
| `check-dev-record-completeness` | **CURRENT:** PASS — 31 owner assertions, 16 owned-but-deferred, 0 violations |
| `check-multi-tenant-loom` | **CURRENT:** exit 0, required journey `ABSENT`, `product_claim: NOT_PROVEN` |
| `check-reza-production-path` | pre-review operator artifact: exit 0, `PROVEN` |
| `check-cross-region-consensus` | pre-review operator artifact: exit 0, `PROVEN` |
| `check-multi-region-slo` | pre-review operator artifact: exit 0, `PROVEN` |
| `check-scale-churn` | PASSED — oracle green (4 legs); BLOCKING at v1_5 |
| `check-ship-gate-completeness` | PASSED |
| `cargo fmt --all -- --check` | clean |

**What this story does NOT claim.** It does not close NFR-Ops-11 (team axis
served; operator sub-axes (i) and (iii) open). It does not execute 14
institutions — Reza is ONE institution with three teams. It did not build a
mechanism: every gap it found is filed with a live owner. And CI still cannot
reach `PROVEN_LIVE_SIGNED`, correctly, because CI holds no operator key.

### File List

**New**

- `docs/testing/local-loom-substrate.md`
- `docs/release/v2.2-capacity-envelope.md`
- `xtask/src/sprint_status.rs`
- `_bmad-output/implementation-artifacts/13-6-evidence/SUMMARY.md`
- `_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-cross-region-consensus.pre-review.json`
- `_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-multi-region-slo.pre-review.json`
- `_bmad-output/implementation-artifacts/13-6-evidence/review-observation-check-multi-tenant-loom.json`
- `_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-multi-tenant-loom.pre-review.json`
- `_bmad-output/implementation-artifacts/13-6-evidence/evidence-ledger-check-reza-production-path.pre-review.json`

**Modified**

- `crates/maos-bin/tests/cross_team_crossing_13_6b.rs`
- `crates/maos-loom-lite/tests/cross_region_live.rs`
- `crates/maos-loom-lite/tests/migration_live.rs`
- `xtask/src/check_loom_substrate_drift.rs`
- `xtask/src/check_multi_tenant_loom.rs`
- `xtask/src/check_dev_record_completeness.rs`
- `xtask/src/check_review_findings_resolved.rs`
- `xtask/src/evidence_ledger.rs`
- `xtask/src/main.rs`
- `xtask/kloc.toml`
- `docs/adr/ADR-055-multi-tenant-loom.md`
- `docs/release/v1.5-topology-support.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/13-6-reza-cortex-journey-closer-nfr-scale-5.md`
- `_bmad-output/implementation-artifacts/13-6e-judge-machinery-evidence-ledger.md`
- `_bmad-output/planning-artifacts/epics/index.md`

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created from the grounding pass at `cb412348`. Crossing mechanism split OUT to 13.6a; evidence ledger + substrate scoped IN. 6 ACs. Status `blocked`. |
| 2026-07-30 | Judge-machinery design confirmed (party-mode preflight). |
| 2026-08-04 | Re-grounded at `b568a052` by five scouts and SPLIT (Epic 13's sixth): AC5's judge machinery → **13.6e**. Pin corrected 23401 → 23679. AC1 → close-out. AC2's "one run" → one composed topology. AC4 → 2-of-4. AC6 surface re-measured to 16 sites. |
| **2026-08-07** | **Resolution round — all four forks closed by measurement; none was an operator preference.** **F-1:** 13.6e's `ledger_enforced()` is an **unsanctioned THIRD enforcement axis** violating the two-axis invariant stated at `gate_common.rs:31-33` (*"NEVER dev-time enforcement"*) — in the same file 13.6e extended, three lines below a `project_gate_binding_decay` citation. `epic-13:200` explicitly permits a development lane to remain advisory while `ABSENT`; the claim refusal is **already** correctly declared (`v2_2 = "blocking"` on both Family-A gates, fired by `check_ship_gate_completeness.rs:143-149`) and correctly dormant at `CURRENT_PHASE = "v1_5"`. Remedy is a workflow declaration — **measured**: `MAOS_LEDGER_ENFORCE=0` under `GITHUB_ACTIONS=true` → exit 0 with the ledger still published. Corollary: `PROVEN_LIVE_SIGNED` is unreachable in CI **by correct design**; the journey is proven on an **operator-run lane**, and this story is the first to run it. **F-2 REVERSED from the 2026-08-06 recommendation:** the contract table shows `check-multi-tenant-loom` already requires `TEAM_A/B/C` and already runs **8+** legs from the same harness file, while the reza gate requires two teams and touches that file only for the leg 13.6e just added — so **move the leg**, at zero substrate/workflow/CONTRACTS cost. This dissolves **two of AC5's four blockers**, which were artefacts of 13.6e registering a three-team control on a two-team gate. **F-3:** the `check-service-boundary` RED is a **gate bug** — cfg is inspected at only `:1094`/`:1197`, both in the P4 walk, whose `contains("test")` rule would already skip `spill_test_faults`; the main surface walk never received it. Fix is porting 8 existing lines; classifying the symbols would assert something false. **F-4:** `epic-13-retrospective: optional` **is** a valid owner — `epic-12-retrospective` went `optional` → `done` and ratified B1–B6, including the Option-C binding F-1 rests on — but with **measured one-epic slippage** (E11 A1/A2/A3 → E12 B1/B2/B3), so the sweep needs a **fourth** bucket, `owned-but-deferred`. **Scope note:** the AC5 sweep is evidence tooling, not a journey mechanism (trap #1 reads *"inside the journey harness"*), and fits the **705-line** `xtask` headroom; `load_sprint_status` must be **extracted**, not triplicated. Tasks split T0→T0a/T0b and T7→T7a/T7b; traps 19–20 added. |
| **2026-08-06** | **Re-grounded a fourth time against the UNCOMMITTED 13.6e working tree by six adversarial scouts.** Two independent CI reds found, neither previously filed: **D-0** `check-service-boundary` RED at HEAD on 5 `spill_test_faults` symbols (blocking, in `aggregate`, and falsifying 13.6c's own "all gates green" claim); **D-1** the enforced lane **cannot go green** — no operator key in CI by ratified design ⇒ every green live leg projects `INDETERMINATE` ⇒ all four journey gates exit non-zero once 13.6e is pushed (measured: `exit 1`). **AC5's "does not modify the ledger" proved unsatisfiable** (4 blockers) and was re-drawn as *machinery vs declarations*. **AC4's premise inverted**: (a)/(b) are already built, proven **and ledger-bound legs** — the genuinely open slices are the refused-crossing operator tail and retry/recovery (**zero** coverage anywhere); (c)/(d) confirmed unbuildable and **stronger** than filed ((d) has no *join key*, not merely no code); the kernel collapse is **8 → 1, not 8 → 5**, and the six erased Spirit-path causes are named. **AC1(d) found already-closed** — struck to avoid a fabricated requirement. **AC2**: "arithmetic" holds only for a **chain**; harness is single-region; 7 deletion targets not 6; 6 processes. **AC5**: ownerless table re-measured — `check-fkcs` row **DELETED** (fixed by 13.6e) and replaced by its stale-at-birth successor; ship-gate row re-framed; kloc row found *stronger* (control unchanged, fifth consecutive re-base). Stale-owner sweep found **seven** live instances, four missed by the draft, and is **net-new tooling**. **AC6**: 16/16 sites still stale (zero fabricated), **three draft citations repaired** (the churn test is in `maos-a2a-tcp`, not `maos-bench` — that file does not exist), six NEW sites → ~22. Four forks (**F-1**…**F-4**) recorded for operator ratification. 6 ACs held. |
| **2026-08-08** | **IMPLEMENTED (dev pass `anthropic/claude-opus-5`); Status `ready-for-dev` → `review`.** T1–T10 complete. **The Reza journey is `PROVEN` on an operator-run lane and 26 legs reached `PROVEN_LIVE_SIGNED` — the first in this repository's history** (four ledger gates, all exit 0, `operator_key_available: true`; artifacts in `13-6-evidence/`). AC1: three topology-fraud limbs became pure oracles with hermetic per-limb proven-red legs, and the drift gate's **value blindness is closed** by an axis-scoped `topology-value-distinctness` leg with a non-vacuous ratified-alias list. AC2/AC3: a 6-process, 3-team, **genuinely 3-region** composed scene (chain A→B, B→C, shared `MAOS_HOME`, zero production lines changed), with all seven wiring sites individually falsifiable against in-memory clones. AC4: the refused-crossing operator tail and retry-only-after-repair are covered for the first time; the kernel collapse is ruled **8 → 1** naming the six Spirit-path causes; the owner string that named this story was re-assigned; `CrossWallRecallRefusal`'s **6 → 1** collapse recorded. AC5: `load_sprint_status` single-sourced, the mechanical stale-owner sweep added to the EXISTING dev-record gate (no new gate) with a planted-owner non-vacuity control — 20 violations found, every one dispositioned, gate now green with 13 owned-but-deferred rows surfaced. AC6: `docs/release/v2.2-capacity-envelope.md` on the correct host/institution axis; kernel baseline verified at the measured **23679 == 23679**. **THREE NEW FINDINGS, all filed with live owners and none fixed (trap 1):** (1) `MAOS_ONE_SHOT=collective-erase` has NO reachable configuration on a tenant host; (2) neither does `maos traceback --team <T>`, so a consented, served cross-wall read is **impossible in production** — both refusals now pinned by the journey leg; (3) `live_destination_adapter_applies_and_refuses_expected_shapes` was **RED at HEAD on any live substrate** (verified by revert-and-rerun) — a leg that could never be green — and was repaired with the production tenant-map wiring. ZERO kernel-core Δ @23679; `xtask` 35797/35931 with no new grant. |
| **2026-08-08** | **CODE REVIEW CHUNK 1/3 (`openai-codex/gpt-5.6-sol`, different from the dev model).** Rejected the operator-lane completion claim: two of the six claimed processes failed before dispatch, yet the enclosing test emitted a signed `PASSED` record. Withdrew the journey attestation, made one-sided erase explicitly red, isolated child-process control variables, made wiring falsification exact, pinned the exact intent lineage, kept daemon stderr drained, and corrected AC3 to the shipped DTO field names. Verification: crossing harness 18 passed / 4 ignored; current multi-tenant gate exit 0 with the required journey `ABSENT` and `product_claim: NOT_PROVEN`. One topology-control finding was dismissed after whole-diff verification. Story remains `in-progress` pending review chunks 2 and 3. |
| **2026-08-08** | **CODE REVIEW CHUNK 2/3, corrected during documentation review.** Missing/vacuous deferred registers now fail, completed retrospective owners expire, malformed topology values red by key, and open `Unresolved` headings remain visible. The initial review incorrectly made explicit ownerless rows fail despite AC5; that patch was reverted while preserving complete classification. The mandatory-ledger-leg verifier crossed AC5's machinery boundary, so it was reverted and filed by reopening Story 13.6e. Current dev-record gate: 31 assertions, 16 owned-but-deferred, zero violations. |
| **2026-08-08** | **CODE REVIEW CHUNK 3a/4.** Corrected story/sprint current state, restored AC5's explicit ownerless-is-honest policy while making every ownerless marker visible, hardened the local substrate runbook, narrowed the v2.2 supersession to the measured 30-host churn axis, corrected the dial formula, and separated 2/3-endpoint adversary metrics from the N=30 drill. The out-of-scope mandatory-ledger verifier was reverted and filed by reopening Story 13.6e. Current dev-record gate: PASS, 31 assertions / 16 owned-but-deferred / 0 violations. Current multi-tenant claim remains `NOT_PROVEN`; the generated evidence group receives the final review. |
| **2026-08-08** | **CODE REVIEW EVIDENCE CLOSE-OUT.** Replaced the self-invalidating dirty-worktree current ledger with a sanitized `CURRENT_REVIEW_OBSERVATION_UNBOUND`, separated the current `SUMMARY.md` index from superseded history, renamed every historical ledger to `.pre-review.json` so the consumer rejects it by filename, and removed operator-local paths outside signed transcript payloads. All requested review groups are complete. Story remains `in-progress`: journey `ABSENT`, product `NOT_PROVEN`, 14 institutions unmeasured, and mandatory-leg omission owned by reopened 13.6e. |
| **2026-08-11** | **CLOSURE — `review` → `done` at `9160eecb`.** The four `NOT_PROVEN` causes were removed at the source rather than re-declared: (1) `PublishedLedger::validate_against` now requires the exact gate-owned leg set derived from each gate's own declarations, closing the vulnerability that reopened 13.6e; (2) tenant mode classifies the `traceback` and cohort-backed `collective-erase` one-shots as bounded-refreshable, so all six journey processes reach production dispatch; (3) `CrossTeamCrossingControl::Erase` reconciles destination and origin under provenance-bound, operation-scoped, generation-conditional authorization over the authenticated intake path; (4) the 14-institution Cortex axis is measured live by `cortex_fourteen_institution_isolation_live`. Four adversarial review rounds (three layers each) produced 11 fixed findings and 1 evidenced dismissal. Operator-lane verification at clean commit `9160eecb`: all four substrate gates `product_claim: PROVEN`, required journey leg `PROVEN_LIVE_SIGNED`, institution leg `PROVEN_LIVE_SIGNED`, ship-gate completeness PASS, dev-record gate PASS (31 assertions / 16 owned-but-deferred / 0 violations), full xtask suite green, no workstation paths in any published artifact. |
| **2026-08-11** | **REOPENED by self-review, then re-closed — budget close-out.** The first close was declared over a **red blocking gate**: `kloc-check` was never re-run, and three ceilings were breached by the closure's own production code (`maos-bin` 15860/15104, `maos-loom-lite` 5277/4847, `xtask` 36116/35931). All three entered the session under ceiling; the +2092 `src/` lines are the four review rounds' HIGH repairs, so reduction was evaluated and rejected — deleting any block re-opens a named finding. Ratified-formula grants (`formatted_measured + max(100, ceil(2%))`) recorded in `xtask/kloc.toml` with per-crate drivers and **FLAG-Winston authorization from Lunarpulse dated after the measurement**: `maos-bin` → 16178, `maos-loom-lite` → 5383, `xtask` → 36839. Aggregate 143734 / 144224 was **not** breached and was left untouched. `kloc-check` PASSES. Functional evidence is unchanged and still binds `9160eecb`. Two items handed to the Epic-13 retrospective: the **sixth consecutive** `xtask` re-base, and the process defect that let "the test suite passed" stand in for "the gates passed". |
