---
baseline_commit: b568a052 + the UNCOMMITTED Story 13.6e working tree (29 files, +5231/−1340). ⚠ 13.6e is `done` but **not committed**. Measure the working tree, not `HEAD`.
depends_on: 13-6a (DONE @a414f922), 13-6b (DONE @05e7e967), 13-6c (DONE @c571a2b9), 13-6d (DONE @b400d127), 13-6e (DONE — five-chunk review complete 2026-08-04, uncommitted)
blocked_by: NONE. All four forks recorded on 2026-08-06 were **resolved by measurement on 2026-08-07** (see `## Resolutions`) — three were defects with a single correct fix, one is answered by precedent. Nothing awaits an operator choice; ratify the reasoning if you wish, but dev is unblocked.
kernel_grant: NONE — ZERO maos-kernel-core Δ expected, pin **23679** (verified by execution 2026-08-06: `maos-kernel-core/src = 23679`, pinned `23679`)
inherited_residuals: (a) the kernel collapses **eight** `TransportCause` variants into **one** `CollectiveErrorKind::Transport` at `maos-kernel-core/src/memory/mod.rs:206` — 13.6e registered `kernel-collective-cause-distinguishable` as a machine-readable successor and **names Story 13.6 as its owner in code** (`check_multi_tenant_loom.rs:167-172`); this story rules the claim in writing and re-assigns the owner without implementing the widening; (b) `CollectiveMemoryPort` has exactly four verbs — `write`/`read`/`scan`/`erase`, no `share` (13.6b Residual 7, re-verified).
---

# Story 13.6 — The Reza Cortex journey closer: compose, judge, and refuse to over-claim

Status: **ready-for-dev** — 13.6a/b/c/d/e all `done`

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
> **And it does not claim the journey is `PROVEN` from CI — because it cannot be, correctly.** CI holds no operator key by ratified design, so `PROVEN_LIVE_SIGNED` is unreachable there. The Reza journey is proven on an **operator-run lane** (real 3-team Postgres + the operator key), which is exactly what 13.6e's dirty-worktree binding was built to support. **No leg in this repo has ever reached `PROVEN_LIVE_SIGNED` because nobody has run that lane yet. This story is the first that does.**

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

✅ **RESOLVED — see F-3.** It is a **gate bug**, and the fix is 8 lines the gate already contains in its P4 walk. Do **not** classify the symbols.

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

✅ **RESOLVED — see F-1.** This is not a deadlock: `ledger_enforced()` is an **unsanctioned third enforcement axis** that violates `gate_common.rs:31-33`. The remedy is a workflow declaration (`MAOS_LEDGER_ENFORCE=0`, **measured**: exit 0, ledger still published), and the claim refusal is already correctly declared at `v2_2`. `PROVEN_LIVE_SIGNED` is unreachable in CI **by correct design** — it belongs to the operator-run lane this story is the first to execute.

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

**And** the read side's minimum disclosure is judged **as measured**: `build_entry` (`log_recall.rs:240-249`) returns exactly six fields — `frame_id`, `timestamp_ns`, `kind`, `intent`, `spirit_pid`, `payload_present: bool` (`!payload_redacted.is_empty()`). **No payload bytes cross,**

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

- [ ] **T0a (D-0/F-3)** — Port the existing cfg-skip from `walk_p4_mod`/`walk_p4_inline_item` (`check_service_boundary.rs:1093-1101`, `:1196-1202`) into `walk_mod:331`/`walk_inline_mod_item:425`; prove it red-then-green. File the gate bug in `deferred-work.md`; correct `13-6c…md:372`'s false "service-boundary green" claim. **Do not touch `kernel-api-classes.toml`.**
- [ ] **T0b (D-1/F-1)** — Set `MAOS_LEDGER_ENFORCE=0` on the four journey-gate steps in `discipline.yml`; confirm each publishes its ledger and its `product_claim` unchanged. File three machinery findings against 13.6e: the third enforcement axis, the empty-string parse, and the three-team leg on a two-team gate. **Do not patch `evidence_ledger.rs`.**
- [ ] **T1 (AC1)** — Commit proven-red controls for all **three** topology-fraud limbs in the `check_loom_substrate_drift.rs:815-890` idiom; close or record the drift gate's value blindness (**axis-scoped**); document local setup for all four jobs; correct `cross_region_live.rs:12-17` and `migration_live.rs:12`; record (d) as already-closed.
- [ ] **T2 (AC2)** — Write the composed topology down (6 processes, chain shape, shared `MAOS_HOME`); extend `cross_team_crossing_13_6b.rs:1204` from 2 → 3 daemons via a **new** 3-team builder.
- [ ] **T3 (AC2)** — Per-site dead-wire falsification across **seven** targets incl. `main.rs:9459-9481`. Serialized, byte-identical restore.
- [ ] **T4 (AC3)** — Minimum-disclosure negatives; exercise `maos traceback` against a **daemon-written** tenant TL; record the three honest limits.
- [ ] **T5 (AC4)** — Cover the two open slices: the refused-crossing operator tail, and retry/recovery after a valid repair. Do **not** re-file (a)/(b).
- [ ] **T6 (AC4)** — Author findings for (c) and (d) with **live** owners; rule the kernel question in writing as **8 → 1** naming the correct six Spirit-path causes; re-assign the owner string at `check_multi_tenant_loom.rs:167-172` and fix `:170`'s "five"; record the `CrossWallRecallRefusal` 6→1 collapse.
- [ ] **T7a (AC5)** — Apply the re-drawn rule: remove the journey leg from `NOT_REQUIRED_LEGS`; **re-home it to `check-multi-tenant-loom`** (F-2 — no substrate change needed); run the **operator lane** (real 3-team Postgres + operator key) and capture the signed `PROVEN_LIVE_SIGNED` evidence — the first in this repo's history.
- [ ] **T7b (AC5)** — Build the mechanical stale-owner sweep. **Extract** `load_sprint_status` into a shared helper rather than writing a third copy; classify into the **four** buckets (F-4); carry the non-vacuity control (reds on a planted `Owner: 13-6a`; must find all seven). Disposition every stale owner and every ownerless row. Budget: 705 lines of `xtask` headroom, no new grant.
- [ ] **T8 (AC6)** — Author the capacity envelope in `docs/release/`, derived from `11-3…md:303` + `check-scale-churn`; state the axis, exclusions and measurement limits; supersede `v1.5-topology-support.md:3`.
- [ ] **T9 (AC6)** — Correct ~22 stale sites; repair the three bad citations first; verify the final baseline at the measured number; record the HISTORY gap (do not fix — it needs a gate, which is a successor).
- [ ] **T10** — Gates: `check-kernel-baseline`, `kloc-check`, `check-service-boundary`, `check-multi-tenant-loom`, `check-reza-production-path`, `check-cross-region-consensus`, `check-multi-region-slo`, `check-loom-substrate-drift`, `check-scale-churn`, `check-ship-gate-completeness`, `cargo fmt --all -- --check`. Record the dev model; pre-book the **full §A6 net on a different model** (trap 17).

---

## Dev Notes

### Budget — verified by execution 2026-08-06

- **kernel-core: ZERO expected @ 23679.** Verified: `maos-kernel-core/src = 23679 lines, pinned 23679`. *"kernel-core ZERO" ≠ "zero delta"* — state both.
- **fkcs:** frozen `23081`, byte-untouched.
- **kloc:** `kloc-check` **PASSES** at aggregate **141396 / 144224**; `xtask 35226/35931`. Re-measure after this story's code. ⚠ 13.6e re-based `xtask` **twice** and moved `_aggregate_hardfail`; do not spend its grant on unrelated surfaces, and expect the retro to ask about the fifth consecutive re-base.
- Two kernel measurements diverge **by design**: `spill_test_faults.rs` (107 lines) is kloc-excluded but baseline-counted. **23679 physical vs 18210 logical** — and it is also the source of D-0.

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

### Debug Log References

### Completion Notes List

### File List

---

## Change Log

| Date | Change |
|---|---|
| 2026-07-28 | Created from the grounding pass at `cb412348`. Crossing mechanism split OUT to 13.6a; evidence ledger + substrate scoped IN. 6 ACs. Status `blocked`. |
| 2026-07-30 | Judge-machinery design confirmed (party-mode preflight). |
| 2026-08-04 | Re-grounded at `b568a052` by five scouts and SPLIT (Epic 13's sixth): AC5's judge machinery → **13.6e**. Pin corrected 23401 → 23679. AC1 → close-out. AC2's "one run" → one composed topology. AC4 → 2-of-4. AC6 surface re-measured to 16 sites. |
| **2026-08-07** | **Resolution round — all four forks closed by measurement; none was an operator preference.** **F-1:** 13.6e's `ledger_enforced()` is an **unsanctioned THIRD enforcement axis** violating the two-axis invariant stated at `gate_common.rs:31-33` (*"NEVER dev-time enforcement"*) — in the same file 13.6e extended, three lines below a `project_gate_binding_decay` citation. `epic-13:200` explicitly permits a development lane to remain advisory while `ABSENT`; the claim refusal is **already** correctly declared (`v2_2 = "blocking"` on both Family-A gates, fired by `check_ship_gate_completeness.rs:143-149`) and correctly dormant at `CURRENT_PHASE = "v1_5"`. Remedy is a workflow declaration — **measured**: `MAOS_LEDGER_ENFORCE=0` under `GITHUB_ACTIONS=true` → exit 0 with the ledger still published. Corollary: `PROVEN_LIVE_SIGNED` is unreachable in CI **by correct design**; the journey is proven on an **operator-run lane**, and this story is the first to run it. **F-2 REVERSED from the 2026-08-06 recommendation:** the contract table shows `check-multi-tenant-loom` already requires `TEAM_A/B/C` and already runs **8+** legs from the same harness file, while the reza gate requires two teams and touches that file only for the leg 13.6e just added — so **move the leg**, at zero substrate/workflow/CONTRACTS cost. This dissolves **two of AC5's four blockers**, which were artefacts of 13.6e registering a three-team control on a two-team gate. **F-3:** the `check-service-boundary` RED is a **gate bug** — cfg is inspected at only `:1094`/`:1197`, both in the P4 walk, whose `contains("test")` rule would already skip `spill_test_faults`; the main surface walk never received it. Fix is porting 8 existing lines; classifying the symbols would assert something false. **F-4:** `epic-13-retrospective: optional` **is** a valid owner — `epic-12-retrospective` went `optional` → `done` and ratified B1–B6, including the Option-C binding F-1 rests on — but with **measured one-epic slippage** (E11 A1/A2/A3 → E12 B1/B2/B3), so the sweep needs a **fourth** bucket, `owned-but-deferred`. **Scope note:** the AC5 sweep is evidence tooling, not a journey mechanism (trap #1 reads *"inside the journey harness"*), and fits the **705-line** `xtask` headroom; `load_sprint_status` must be **extracted**, not triplicated. Tasks split T0→T0a/T0b and T7→T7a/T7b; traps 19–20 added. |
| **2026-08-06** | **Re-grounded a fourth time against the UNCOMMITTED 13.6e working tree by six adversarial scouts.** Two independent CI reds found, neither previously filed: **D-0** `check-service-boundary` RED at HEAD on 5 `spill_test_faults` symbols (blocking, in `aggregate`, and falsifying 13.6c's own "all gates green" claim); **D-1** the enforced lane **cannot go green** — no operator key in CI by ratified design ⇒ every green live leg projects `INDETERMINATE` ⇒ all four journey gates exit non-zero once 13.6e is pushed (measured: `exit 1`). **AC5's "does not modify the ledger" proved unsatisfiable** (4 blockers) and was re-drawn as *machinery vs declarations*. **AC4's premise inverted**: (a)/(b) are already built, proven **and ledger-bound legs** — the genuinely open slices are the refused-crossing operator tail and retry/recovery (**zero** coverage anywhere); (c)/(d) confirmed unbuildable and **stronger** than filed ((d) has no *join key*, not merely no code); the kernel collapse is **8 → 1, not 8 → 5**, and the six erased Spirit-path causes are named. **AC1(d) found already-closed** — struck to avoid a fabricated requirement. **AC2**: "arithmetic" holds only for a **chain**; harness is single-region; 7 deletion targets not 6; 6 processes. **AC5**: ownerless table re-measured — `check-fkcs` row **DELETED** (fixed by 13.6e) and replaced by its stale-at-birth successor; ship-gate row re-framed; kloc row found *stronger* (control unchanged, fifth consecutive re-base). Stale-owner sweep found **seven** live instances, four missed by the draft, and is **net-new tooling**. **AC6**: 16/16 sites still stale (zero fabricated), **three draft citations repaired** (the churn test is in `maos-a2a-tcp`, not `maos-bench` — that file does not exist), six NEW sites → ~22. Four forks (**F-1**…**F-4**) recorded for operator ratification. 6 ACs held. |
