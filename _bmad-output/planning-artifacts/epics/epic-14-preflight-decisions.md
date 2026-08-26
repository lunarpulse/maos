# Epic 14 — Preflight Decision Register (Story 14.0)

> **Source:** Epic 13 retrospective action **C2** (2026-08-11), implementing the
> retro's §7.4 takeaway that *"owned by a retrospective" is not an owner*.
> **Owner:** Winston + John. **Status:** open.

## What this register is

The Epic 13 retrospective §4 disposition table handed **12 residual groups** to
Epic 14 or its preflight. Seven of them had no target story — they pointed at
`epic-14`, which is an epic key, not a vehicle. This register is that vehicle.

It is a **decision queue, not a set of decisions.** Each row records *that a
decision is required, who owns it, which story receives the outcome, and the
event by which it must be settled*. Recording a target here does **not** decide
the substance — that is the named owner's call at preflight.

**It is also, since Story `14-0` (2026-08-26), a file a machine reads.** Before
that it was not: grepping `epic-14-preflight-decisions` across `*.rs` / `*.yml` /
`*.toml` returned **two hits, both prose comments**, and the register lives in
`planning-artifacts/epics/`, outside every gate's `STORY_DIR`. Binding rule 2
promised that *"did we miss it"* is a query; there was no query and no queryer,
so **every deadline in this file was a judgement**, and eight of nineteen rows
were already wrong at HEAD. `xtask check-decision-register` is that queryer.

## The machine contract

`xtask/src/check_decision_register.rs` parses the Decisions table below and
**fails closed**: an unreadable register, a table it cannot find, zero parsed
rows, zero resolved targets, or a table row it cannot name is an error, never a
pass. A gate that governs nothing passes for the wrong reason, and
`findings.is_empty()` cannot see it.

| Cell | Contract |
|---|---|
| `ID` | `**D<n>**` or `**D<n><letter>**`, then ` · ` and a status of `OPEN` or `CLOSED`. A row that declares neither is a finding — an undeclared status is exactly how **D18** sat `RESOLVED` over an unimplemented substance with a deadline four stories in the past. |
| `Target story` | Must resolve to a `development_status` key: exact, or a UNIQUE `<token>-…` expansion. An `epic-*` key is **not** a vehicle (the register's founding defect); a retrospective action (`C3`, `C5`) is **not** a vehicle (*"owned by a retrospective is not an owner"*); a phrase deferring the naming is **not** a vehicle either. |
| `Deadline (mechanical)` | Either `` before `<key>` leaves `backlog` `` or `` before `<key>` reaches `done` ``, resolved against `sprint-status.yaml` and nothing else — **or** a clause that says `UNQUERYABLE` and why. An unqueryable deadline that does not say so reds. Declared ones are reported in their own bucket and are **never counted green**; a row may attach a mechanical `RE-ANCHORED:` clause after the declaration so the obligation still binds. |
| `Status` semantics | `CLOSED` means the decision is recorded with named evidence and nothing is outstanding. `OPEN` means the obligation stands — including rows whose *decision* is ruled but whose *implementation* is not yet landed, so a recorded ruling can never make an outstanding obligation invisible. |

## Binding rules

1. **No residual may be closed by implication.** A row closes only when a
   decision is recorded against its ID with named evidence. Shipping adjacent
   work does not close a row.
2. **Deadlines are mechanical, not calendar.** Each is anchored to a state
   transition observable in `sprint-status.yaml`, so "did we miss it" is a
   query, not a judgement. This follows C1's single-source rule. **Since `14-0`
   this rule is enforced rather than asserted** — see the machine contract above.
3. **`D1`–`D4` block `14-1`.** C2 requires them settled *before 14.1 opens*;
   `14-1` and `14-2` inherit the weaker half of the evidence discipline
   (retro C3) and must not rely on an unsettled advisory classification.
4. A row whose decision outcome is "no change" is still **closed by decision**,
   with the rationale recorded. Silence is not an outcome.
5. **A row may not be repaired by re-pointing it at another non-vehicle.** Seven
   rows once pointed at `epic-14`; seven then pointed at *"14-0 decomposes into a
   named story"*, which is the same defect one level down. Every target must be a
   key the tracker can page, and `check-decision-register` now reds if it is not.

## Decisions

| ID | Residual (retro §4) | Decision required | Target story | Deadline (mechanical) | Owner |
|---|---|---|---|---|---|
| **D1** · OPEN | Eight Family-B gates sit outside the four-contract evidence ledger; `check-vetting-attestation` and `check-wasm-form-equiv` cannot represent `ABSENT` at all. | Does the ledger expand to Family-B, or do Family-B gates get a separate evidence contract? The ledger is deliberately derived from four journey contracts and must not silently expand. | `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` — RESTATED by 14-0 (see the ruling below). Its former target was retro action C3, which is not a vehicle. | Before `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` leaves `backlog` | Murat + Winston |
| **D2** · CLOSED | Skipped `AdvisorySubstrate` legs report `passed: true` under the shared house pattern. | Ratify `absent != green, skipped != passed` for advisory legs, or ratify the current pattern with an explicit claim boundary. Unavailable substrate is unmeasured, never proof. | `14-0-epic-14-preflight-decisions` — closed by decision; the surviving residual is filed as **D20**. | Before `14-1-100-host-churn-scale-envelope` leaves `backlog` | Murat + Winston |
| **D3** · OPEN | Fallible `record_invocation` can drop an audit event while returning `Ok` (`crates/maos-kernel-core/src/capability/mod.rs:332-351`). | Kernel-core fallible-path repair requires a **new FLAG-Winston decision**. Approve the kernel-Δ, or ratify the drop with a stated boundary. | `14-d3-audit-drop-observability` | UNQUERYABLE — "before any Epic 14 kernel-core edit" is a code event, not a `sprint-status.yaml` transition, and the ruling is ZERO kernel-Δ so the trigger can no longer fire; RE-ANCHORED: before `14-d3-audit-drop-observability` leaves `backlog` | Winston (FLAG-Winston) |
| **D4** · CLOSED | `MAOS_REGION_HOME` is never reconciled against the signed `TeamEntry.region`. | Enforce equality in production, or register the variable and state the deployment-integrity boundary. **⚠ BOTH HALVES OF THIS SENTENCE WERE MEASURED FALSE and are corrected here (Story `14-0` AC4.4, 2026-08-26): the final harness does NOT derive region — `cross_team_crossing_13_6b.rs:2138-2145` is a hard-coded `match team { "team-a" => "region-a", … }` and both the `TeamEntry` (`:2170`) and the daemon env (`:2267`) read that same literal, so they agree by shared constant, not by derivation; and production DOES derive — `cross_team_consent.rs:126-145` derives team keys from the verified manifest region. Production holds BOTH an honest path and an unreconciled one, and they can disagree in the same process.** | SPLIT three ways — `14-d4a-region-home-boot-reconciliation` (D4a), `14-8-register-classify-full-workspace-env-surface` (D4b), `14-9-secret-var-governance-provider-keys` (D4c). | Before `14-1-100-host-churn-scale-envelope` leaves `backlog` | Winston + John |
| **D4a** · OPEN | **D4a — enforcement (runtime).** `MAOS_REGION_HOME` is never reconciled against the signed `TeamEntry.region` at daemon boot. | Reconcile at boot. Constructible and cheaper than feared: `main.rs:9855` `reconcile_transport_identity_with_manifest` already reconciles four env-vs-signed axes including `team_id` — region is the OMITTED FIELD of an existing check, ~20 lines mirroring `cross_team_crossing.rs:892-916`. Must state that a daemon-boot leg does NOT cover `maosctl`. | `14-d4a-region-home-boot-reconciliation` | Before `14-8-register-classify-full-workspace-env-surface` leaves `backlog` | Winston + John |
| **D4b** · OPEN | **D4b — registration.** `MAOS_REGION_HOME` is absent from `env_contract.rs`'s 67 entries, and the gate cannot even see it: `check_env_contract.rs:119` scans only `crates/maos-bin/src` while both primitive reads live in `maos-kernel-core` and `maos-domain`. | Register and classify the variable once the workspace-wide registry exists. Stays where D4 put it, correctly sequenced behind 14-7. | `14-8-register-classify-full-workspace-env-surface` | Before `14-8-register-classify-full-workspace-env-surface` leaves `backlog` | Winston + John |
| **D4c** · OPEN | **D4c — classification + crypto, and there is no slot for it.** `sealed_export.rs:303-315` derives the signing key by HKDF over the region tag and `derive_team_signing_seed` welds the per-team key over that seed, so an unregistered env var SILENTLY SELECTS WHICH Ed25519 KEY SIGNS YOUR AUDIT BUNDLE. It is undetectable at every verifier: `resolve_verify_key` derives the expected key from *the bundle's own claimed region*, so a wrong-but-self-consistent region verifies GREEN and `key_a == key_b` never fires. | `EnvStability::Secret` is the WRONG slot — the value is not secret and must stay echoable (`operator_config.rs:227` prints it), and classifying it Secret would red an existing correct `eprintln!` under 14-9's own gate. Decide whether a slot for "non-secret value, key-derivation input, integrity-critical" is added, or state the boundary. | `14-9-secret-var-governance-provider-keys` | Before `14-9-secret-var-governance-provider-keys` leaves `backlog` | Winston + Vex |
| **D5** · OPEN | Legal-hold check-then-act race; revocation failure signed as zero; decommission receipt claims completion; mutation-to-audit crash gap. | Scope into **distinct controls**. These are four lifecycle/atomicity questions and must not be folded into the already-proven collective erase. | `14-e1-erasure-attestation-honesty`, `14-e2-legal-hold-erase-serialization`, `v25-erasure-crash-reconciliation` | Before `14-1-100-host-churn-scale-envelope` leaves `backlog` (RE-ANCHORED from 14-4 by AC4.7) | Winston + Murat |
| **D6** · OPEN | Private-tier residue, erase races, directory-iteration hazards. | Kernel filesystem/atomicity design, or retain the recorded security limits until separately scoped. | `v25-private-tier-erase-atomicity` — the three NAMED defects are closed on measurement; this is the general non-atomicity D6 never mentioned. | Before `14-1-100-host-churn-scale-envelope` leaves `backlog` (RE-ANCHORED from 14-4 by AC4.7) | Winston |
| **D7** · OPEN | `CrossWallRecallRefusal` collapses six variants into the token `refused`. | A shipped CLI operator surface preserving cause only in free text is insufficient for a machine-readable operator outcome. Decide the typed outcome shape. | `14-4-v2-0-sweep-operational-surfaces` | Before `14-4-v2-0-sweep-operational-surfaces` leaves `backlog` | John + Amelia |
| **D8** · OPEN | `check-fkcs` `admission-path-unmodified` is RED, held advisory. | Frozen-kernel-conformant re-pin, or retain the exact bounded hold. Admission sources genuinely changed in 13.4; re-pinning without review would re-can an unreviewed floor. | `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` | Before `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` leaves `backlog` | Winston + Murat |
| **D9** · OPEN | A vetting lapse cannot refuse a crossing. | The only demonstrable vetting boundary is upgrade/promotion, not crossing. Accept that boundary explicitly, or scope a crossing mechanism — **no invented mechanism**. | `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` | Before `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` leaves `backlog` | Murat |
| **D10** · CLOSED | `maos-a2a-core` third consecutive KLOC grant / v2.5 team-crypto identity. | The v2.5 ecosystem graduation ledger is the explicit home for an external/team-identity boundary. **No retro may ratify a third unscoped grant by implication.** | `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` | Before `14-3-ecosystem-readiness-verification-v2-5-graduation-ledger` leaves `backlog` | Winston + Lunarpulse |
| **D11** · OPEN | No gate reconciles kernel-pin HISTORY; **budget-charged code with no execution path** (widened 2026-08-14 — see D11-E1); `EXPECTED_GATES` is hand-maintained (**36 entries vs 67 `check_*.rs` at `af788c3e`; 37 / 68 once the J1 loopback gate lands** — corrected 2026-08-14). | Make pin history, execution coverage, and workflow-derived gate registration mechanically auditable. Carries **E11-A6** forward to mechanical close. | `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` — retro action C5 is not a vehicle and is no longer named as one. | Before `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` leaves `backlog` | Winston + Murat |
| **D12** · CLOSED | `audit_query_latency` bench has been broken since 9.1 (`"capability.invoke"` vs accepted `"capability.invocation"`); it has run **zero** times. | Repair or retire the bench. | `14-0-epic-14-preflight-decisions` — REPAIRED, not retired; see the ruling below. | Before `14-4-v2-0-sweep-operational-surfaces` leaves `backlog` | Amelia |
| **D13** · OPEN | **`maos-kernel-core` kloc ceiling breached: 18933 tokei CODE vs 18248 (+685)** while its PIN is green (24472 = pinned). The two instruments read the same tree differently and both are correct: the pin counts **physical lines in every `.rs`** (`xtask/src/check_kernel_baseline.rs:99-110`, `content.lines()`), the ceiling counts **tokei CODE with tests/benches/examples/fuzz excluded** (`xtask/src/kloc_check.rs:163-213`) — anti-drift vs anti-growth (`kloc.toml:53-59`). One was updated, the other was not. **Arithmetic attribution:** `sprint-status.yaml:230` records that as of Story 13.5j the ceiling still had **273 spare** (17941→17975 / 18248). Then the Epic-5 review-findings closure spent it: `af788c3e` put net **+878** into `crates/maos-kernel-core/src` across 34 files and took a documented **FLAG-Winston PIN grant** (`xtask/kernel-core-baseline.toml`: "net +741 … authorized as the bounded repair of the reopened Stories 5.1/5.2/5.4/5.5a findings") but **never the paired kloc ceiling grant**; `2688c6d0` added a further +52 and is that same vehicle's own `baseline_commit` (`spec-epic-5-review-finding-closure.md:7`, scope includes `maos-kernel-core/src` at `:47-50`). Attribution rests on diff stats and the baseline file's own authorization text, **not on commit titles** — titles are unreliable here (`13-6-reza…md:388` records that `b568a052`'s title names 13.6a while containing zero 13.6a work). | **Split, because the repair and the instrument are different questions.** (a) The *breach repair* belongs to the vehicle that caused it and already holds the authorized measured delta: take the paired measured ceiling grant with a HISTORY row, or show the decomposition. `kloc.toml` states the rule "must never block a correctness or compliance repair", and this is one. **Its closure gate list is the hole that let this through — `spec-epic-5-review-finding-closure.md:79` runs `cargo fmt --all --check` and the workspace suites but NOT `kloc-check`; add it.** (b) The *instrument* question — whether kernel-core may be this large, decomposition vs policy — stays with D11/C5 and must NOT be granted away: `kloc.toml:407-408` keeps kernel-core deliberately tight *because Epic 14 declares ZERO kernel-Δ*, and a retroactive grant from a ZERO-Δ story would invert ADR-038. **14-6 may not erase this red with a grant it has no measured delta to justify.** | (a) `spec-epic-5-review-finding-closure` — DISCHARGED; (b) `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` — the instrument question, still open. | Before `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` leaves `backlog` | (a) Winston + Amelia; (b) Winston + Murat |  | **(a) RESOLVED 2026-08-26 — founder grant taken, breach discharged.** `xtask/kloc.toml:195` `maos-kernel-core = 18248 -> 18933`, **EXACT MEASURED / ZERO HEADROOM**, authorized by Lunarpulse under `kloc.toml:61`'s "explicitly authorized measured grant" door. `kloc-check` now reports `| maos-kernel-core | 18933 | 18933 | ok |`; the +685 red is gone. Chosen over the formula's 19312 (measured + ceil(2%) = 379) deliberately: free growth capacity in the kernel under a ZERO-kernel-Δ epic is the ADR-038 inversion **(b)** forbids of 14-6, and at zero headroom every further kernel-core line still costs its own measured FLAG-Winston grant and a HISTORY row. Precedent: `maos-bin` 16870/16870 (D15, same founder, 2026-08-15, also tighter-than-formula). ⚠ **THIS ROW'S OWN ARITHMETIC WAS WRONG AND IS CORRECTED, NOT OVERWRITTEN.** Measured by `git log --numstat` scoped to `crates/maos-kernel-core/src`: `af788c3e` is **net +741 over 36 file-rows**, not the "+878 … across 34 files" asserted above — and +741 is *exactly* the figure `kernel-core-baseline.toml:465` had already authorized, so this row overstated the delta its own baseline file recorded correctly. The row's second figure survives: `2688c6d0` **+52** (+59/−7) reproduces. Full attribution is in the `kloc.toml:195` annotation. ⚠ **The deadline was never queryable.** *"Before `spec-epic-5-review-finding-closure` reaches `done`"* names a story that has **no `sprint-status.yaml` key** — only a file with its own frontmatter `status: 'in-progress'`. A status no tracker records cannot transition, so binding rule 2 was unsatisfiable on this row from the day it was written; the grant is discharged ahead of a deadline that could not have fired. That defect is 14-0's to fix (one sprint key also un-blinds `governed_story_keys()`). **STILL OPEN from this row's own prescription:** the closure-gate hole — `spec-epic-5-review-finding-closure.md:79` runs `cargo fmt --all --check` and the workspace suites but NOT `kloc-check` (verified at HEAD: zero `kloc` hits in that file). Taking the grant does not add the gate that would have caught this. **(b) remains OPEN and untouched** — whether kernel-core may *be* this large is the instrument question, still 14-6's via D11/C5, and a grant is not an answer to it. `_aggregate_hardfail` is **not** moved by this grant and stays RED under **D17**. |
| **D14** · OPEN | **`maos-domain` ceiling breached: 8694 vs 8644 (+50).** Cause is NOT an Epic-14 driver: `baf83880` grew `crates/maos-domain/src/halt.rs` by +156/−14 — the Story **3.3** halt-resolution work (`sprint-status.yaml:41`, `done` 2026-08-14), corroborated by `j1-crosshost-1a-frame-borne-delegation.md:315-316` attributing the breach to Story-3.3 halt lines. | Story 3.3 closed the same day with a full audit; reopening it to carry a +50 grant is churn. `kloc.toml:218` names **14-7** as this crate's growth driver by design ("the placement was chosen specifically to avoid adding a kernel-crate-set member"), and 14-7 must measure `maos-domain` at its own closure regardless. So: does 14-7 **explicitly expand an AC** to absorb the pre-existing +50 as a measured grant, or does 3.3 reopen for the grant? **Recording 14-7 as target does not decide the substance** — 14-7's ACs today cover extraction *into* the crate, not Story-3.3 halt growth, so absorbing it requires a stated AC expansion, not a silent inheritance. | `14-7-workspace-env-contract-shared-registry-static-scan-gate` (with an explicit AC expansion) | Before `14-7-workspace-env-contract-shared-registry-static-scan-gate` leaves `backlog` | Winston + John |
| **D15** · CLOSED | **`maos-bin` ceiling breached: 16211 vs 16178 (+33) at HEAD.** Cause: `6827dc87` (`j1-crosshost-1a`) added `delegation.rs` +290, `topology.rs` +110, `lib.rs` +10, `main.rs` net +146, `env_contract.rs` −5 (716 insertions / 165 deletions; net ≈ +551 physical). 1a is `done`. Story `j1-demo-one-command-scene` added a further **+8** (the `--once` drain drop-order fix), taking it to 16219 / **+41**; that story's grant was **xtask-only** and explicitly does not cover this. | 1a is closed and its ceiling debt has no vehicle. **`j1-crosshost-1b`** is the open successor in the same lane, already carries the measurement discipline ("re-measure, and only ask WITH the measurement attached"), and will itself touch `maos-bin`/`xtask` — so it can take one measured grant covering 1a's +33 and the demo's +8 together. Alternative considered and rejected as primary: **14-7**, whose AC2 migrates the `maos-bin` env registry out and may *reduce* the crate — a speculative future reduction is not a repair, and it is far behind 1b in the queue. | `j1-crosshost-1b-consent-proofs-and-gate` | Before `j1-crosshost-1b-consent-proofs-and-gate` leaves `backlog` | Winston + Amelia | **RESOLVED 2026-08-15 — Lunarpulse ratified `maos-bin = 16219`, EXACT MEASURED / ZERO HEADROOM, over the formula's 16544** (round-table consensus: Winston, Murat, Amelia, John, Mary, Vex; Dana dissenting-and-answered). Applied at `xtask/kloc.toml:264` with per-commit attribution and the `kloc.toml:87` correctness-repair pointer in the annotation. `maos-bin` is GREEN at HEAD. Rationale: the formula would grant 325 lines to a crate that is 73% `main.rs` with no decomposition scheduled; tighter-than-formula is house style (`xtask` grant, line 203); zero headroom is precedented and deliberate (`maos-a2a-core` 4654/4654). Does NOT cover the aggregate — see **D17**. |
| **D16** · CLOSED | **Suite-wide test-isolation defect in `crates/maos-bin/tests/`, not a single flake.** `cross_wall_recall_live_path_uses_verified_state_and_home_team` (`cross_team_consent_13_3.rs:243`) fails **5/5 at HEAD** under default parallel `cargo test -p maos-bin` and passes **3/3** with `--test-threads=1`. `std::env::set_var("MAOS_HOME", …)` is process-global and the locking is inconsistent across three files: `cross_team_consent_13_3.rs` (`RestoreMaosHome` Drop `:40-47`, locked test `:243-247`, **unlocked** `set_var` `:502-505`, `LIVE_LOCK` `:534`); `cross_team_crossing_13_6b.rs` (`:2725-2727` mutates with no lock in context, Restore `:2761-2768`); `cross_wall_log_read_13_6d.rs` (`env_lock` `:15-17`, locked `:65-68`, but `seed_remote_artifact` mutates at `:31-33`). Pattern authored by Story **13.3** (`e58d0df0`) and propagated through 13.6a/b/d/e — all closed. | Decide the isolation mechanism for the whole suite, not a patch to one test: a shared env guard every toucher must take, or per-test process isolation, or removal of process-global env from the suite. **`--test-threads=1` is a masking workaround, not a resolution** — if it is adopted it must be recorded as a stated boundary with the reason, per binding rule 4. No existing Epic-14 vehicle's ACs cover runtime test isolation (14-6 is the ceiling instrument, 14-7 is a static env registry/scan, 14-4 is canary/push/installers), and the authoring stories are closed — so this follows the **D5/D6 pattern**: 14-0 decomposes it into a named story rather than inventing an ID here. | `14-0-epic-14-preflight-decisions` — the lock AND the whole-package leg both ship here. | Before `14-1-100-host-churn-scale-envelope` leaves `backlog` (RE-ANCHORED from 14-4 by AC4.7) | Murat |
| **D17** · OPEN | **`_aggregate_hardfail` is RED at 147549 / 147057 (+492) and it is NOT self-clearing.** Two things were established by measurement at the `j1-crosshost-1b` round-table (2026-08-15) and must not be re-litigated from memory: (a) the breach is **arithmetic downstream of D13 (+685), D14 (+50) and D15 (+41)** — it is not a fourth independent overrun; and (b) **re-basing those crate ceilings does NOT move the measured aggregate** (measured stays 147549), so the key stays red even after all three D-rows land in full. It is the only instrument that catches distributed growth no per-crate reserve can see, and it is *meant* to sit red while debtors repair. **Prior framing corrected:** the aggregate is neither 'unowned' nor unreachable by a bridge story — `kloc.toml:61` permits recalculation *'at an epic retrospective, **or** under an explicitly authorized measured grant'*, and Stories 13.6d, 13.6e and the epic-orphaned `j1-demo-one-command-scene` all used the second door. | **Who re-bases it, and on what evidence.** `j1-crosshost-1b` REFUSED the grant deliberately (its contribution is zero, and granting it would turn the CI signal holding D13's +685 to account green — which D13 already forbids of 14-6, and 1b is further from the delta). So the re-base belongs to a vehicle with a **measured delta to justify it**, or to the next epic retrospective. Decide which, and record that `kloc-check` exits 1 until then — a standing red with named debtors is an honest state, not an outage. | `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` | UNQUERYABLE — "before the v2.2 wave closes" names no `sprint-status.yaml` transition, and the alternate target `epic-14-retrospective` is status `optional`; RE-ANCHORED: before `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` leaves `backlog` | Winston + Murat |
| **D18** · OPEN | **`map_a2a_error_to_iac_bus` flattens the A2A deny vocabulary above the router** (`crates/maos-a2a-core/src/router.rs:1671-1783`). Both `IntentDenied{Send}` (`:1673-1683`) and `IntentDeniedAtPeer` (`:1684-1690`) collapse to the same `IacBusError::CrossHostIntentDenied`, and **both** `ConsentUnclassified` variants collapse to a stringly `IacBusError::CrossHostRouteFailure` (`:1773-1782`); `DelegationLeg::delegate` (`crates/maos-bin/src/delegation.rs:149-171`) then stringifies even that. **Consequence:** a cross-host operator cannot distinguish `-32001` (policy refused you) from `-32009` (policy could not classify you) — one is the system working, the other is the system blind — and the unclassified *reason* (`Absent`/`NonCanonical`/`Oversized`) is unrecoverable. The non-conflation invariant is real and pinned at the router seam (`fail_closed_8_8.rs:216-240`); it is destroyed one layer up. Found by the `j1-crosshost-1b` preflight, which proves the refusals at the router seam **because it cannot prove them anywhere else**. | **Decide the typed cross-host deny outcome an operator sees.** `j1-crosshost-1b` correctly does NOT fix it: `maos-a2a-core` is at ZERO kloc headroom and **D10** forbids a third unscoped grant, so widening this is a scoped decision, not a side effect. Same shape as **D7** (`CrossWallRecallRefusal` collapsing six variants into `refused`) — consider deciding them together. **Deadline is deliberately BEFORE rung 2 writes code, not before it closes:** `j1-crosshost-2` builds the first real cross-host operator surface, and if it is built on the flattened error the defect becomes load-bearing. NOTE: this was NOT filed 'against `j1-crosshost-2`' — that story has a sprint-status row and (at filing time) no story file; a deferral into a document that does not exist is not a deferral. | `14-4-v2-0-sweep-operational-surfaces` | UNQUERYABLE — the original anchor "before `j1-crosshost-2b` writes its first line" is a code event AND IT BLEW FOUR STORIES AGO (2b, 2c, 2d and 2e are all `done`); RE-ANCHORED: before `14-4-v2-0-sweep-operational-surfaces` leaves `backlog` | John + Vex | **RESOLVED 2026-08-15 (Lunarpulse) — the 'precondition with no budget' paradox was built on an UNMEASURED premise and does not survive measurement.** Three corrections: (1) **The `-32001` pair is ALREADY distinguishable.** `IntentDenied{direction}` (`router.rs:1673-1683`) and `IntentDeniedAtPeer` (`:1684-1690`) both produce `CrossHostIntentDenied`, but with `direction: Send` vs `direction: Accept` — a consumer CAN tell the send seam from the accept seam today. The residual defect there is narrower and semantic: `IntentDeniedAtPeer` stuffs the NACK **message** into a field named `intent`, while the sibling arm puts a real intent string in it. The field lies about itself; it does not erase the distinction. (2) **The real loss is the UNCLASSIFIED pair.** `ConsentUnclassified` and `ConsentUnclassifiedAtPeer` (`:1773-1782`) both collapse into stringly `CrossHostRouteFailure(String)`, discarding the typed `UnclassifiedReason` (`Absent`/`NonCanonical`/`Oversized`) and the direction. That — not the deny pair — is D18's core. (3) **MEASURED COST: `maos-a2a-core` ≈ ZERO net lines.** The two arms are 5-line `format!` constructions; replacing each with a 5-line typed struct construction is net ~0. The new variant lands in `maos-domain::iac_bus_types` (`:14-40`) at ~+6 lines. **So D10's ZERO-headroom wall was never in the way, and no maos-a2a-core grant is required.** The `maos-domain` +6 rides with **D14**, whose owner (14-7) is already required to make an explicit AC expansion for that crate — fold it there rather than opening a second vehicle. **DEADLINE RE-PINNED: 'before `j1-crosshost-2b` writes its first line'** (was: before `j1-crosshost-2`). This is not a weakening — it is the same rule applied to the correct vehicle now that rung 2 is split (ratified 2026-08-15): `2a` is one-host worker hardening and **cannot surface a cross-host deny at all**, so it is unblocked immediately; `2b` is where host B first makes this error operator-visible. **Fallback if the typed variant is refused at 14-4:** preserve the reason in the existing string field and record the typed outcome as still-open alongside **D7** — but that is a worse answer and the measurement says it is not necessary. |
| **D19** · CLOSED | **Seven blocking CI gates cannot see a bridge-lane story file, so story-file discipline is unenforced for the entire J1 series.** Five walk `_bmad-output/implementation-artifacts/` behind a digit-prefix filter and skip any name that does not start with a number: `check_bare_review_findings.rs:35`, `check_dev_model_tier.rs:103`, `check_dev_model_used_populated.rs:136`, `check_dev_record_completeness.rs:245-247`, `check_review_findings_resolved.rs:57-60`. Two more skip by a different mechanism: `check_epic_close_coherence.rs:215-217` (`head.parse().ok()?`, its comment naming `j1-crosshost-1` explicitly) and `check_epic_6_bridge.rs:820-828` (hardcoded `"6-2"`/`"6-3"` prefixes). **CORRECTED 2026-08-16 (`j1-crosshost-2b`/`2c` preflight): the shared-filter defect is SEVEN walkers, not five.** `check_epic_6_bridge.rs` is blind by TWO mechanisms — besides the hardcoded prefix, it carries two more digit-prefix directory walkers of its own at **`:2563`** (`check_7_1_5_bare_rf_count`) and **`:2608`** (`check_7_1_5_dmu_missing_count`), both `name.ends_with(".md") && name.starts_with(|c| c.is_ascii_digit())`. A fix scoped to the five originally listed would leave two walkers behind and the single-source claim would be false at birth. All five original directory-walkers are BLOCKING jobs (`discipline.yml:1720, 1734, 1748, 1762, 1778`). **Net effect: a `j1-*` story can ship with no dev record, no `dev_model_used`, no §A6 marker and no review-findings closure, and zero gates notice — a green CI does not mean the review net ran.** Filed by the `j1-crosshost-2a` preflight round-table (2026-08-16). The hole has been open across `1a` (done), `1b`, `j1-demo-one-command-scene` (done) and now `2a`; each disclosed it in prose and none of them closed it, which is why disclosure is no longer an acceptable disposition. | **Decide the filename contract, not a patch to one gate.** Either (a) replace the digit-prefix filter with the sprint-status key set — every gate then governs exactly the stories the tracker knows about, including epic-orphaned lanes; or (b) ratify that bridge-lane story files are outside story-file discipline and state the boundary in `RELEASE-HOLDS.md` §Claim boundaries, per binding rule 4. **Do not fix one gate**: five walkers sharing one filter is the single-source defect this project has already paid for twice (gate-binding decay, Epic-13 tracking). If (a), the walkers should share one helper so the next filter change is one edit. `j1-crosshost-2a` continues to DISCLOSE in its Dev Agent Record; disclosure is the interim state, not the resolution. | `j1-crosshost-2c-two-host-signed-run` — where it ACTUALLY shipped, which is itself the contract violation AC1 exists to prevent. | UNQUERYABLE — "before the next `j1-*` story leaves `ready-for-dev`" was SELF-VOIDING: 2c is the lane closer and there is no next `j1-*` story. It bound at the round-table or never. | Mary + John |
| **D20** · OPEN | **E12-B1 is six of eight, and it is recorded `done`.** B1's ratified text was *"decouple blocking-disposition from `CURRENT_PHASE`"*. Eight gates still carry a private `const CURRENT_PHASE`; six also adopted `BindingClass`, so theirs is vestigial. **Two adopted nothing** — `check_escape_detector.rs` (`CURRENT_PHASE` `:62`, private `is_blocking_at` `:94`) and `check_cohort_mesh.rs`. OBSERVED, not derived: `cargo run -p xtask -- check-escape-detector` exits 0 while emitting `::warning::Escape-detector oracle RED — would block ship at v2.0`, and reports `"passed": true` (`:691`). | Finish B1 on the two gates that never adopted it, or state the boundary. Its own vacuity guard cannot catch this: on a seccomp-blocked host the legs return `failed=1, green=false`, so `passed==0 && failed==0` is false and the advisory tail converts RED into `passed: true`. **The fix for gate-binding decay decayed the same way** — and Epic 13's retro recorded B1–B6 as 6/6 done, noting its tracking had erred *pessimistic*; here it erred optimistic, which is the direction that hurts. | `e12-b1-gate-binding-decay-residual` | Before `14-6-v2-0-sweep-constitutional-ceiling-formal-methods-disposition` leaves `backlog` | Murat + Winston |

### D19 — RESOLVED 2026-08-17: option (a), implemented by `j1-crosshost-2c`

**Decision:** option **(a)**, chosen unanimously at the 2026-08-17 round-table under
vehicle **14-0** (Mary + John), per spec + long-term correctness. **Option (b) was
refused on grounds:** it would ratify that a *category* of story is exempt from
dev-record, model-tier and review-findings discipline — converting a defect into a
policy just as the defect was about to expire — and the lane it would exempt is the
one running cross-host mTLS, signed artifacts and a paid agent, i.e. where review
discipline matters most.

**The deadline was self-voiding and was called out as such.** *"Before the next
`j1-*` story leaves ready-for-dev"* — but `2c` is the closer of the lane and there is
no next `j1-*` story (verified: the lane holds `1a`/`1b`/`2a`/`2b`/`2c` and the demo).
Either it bound here or it never bound. It bound here.

**Implemented, one helper, seven walkers:**

- `xtask/src/gate_common.rs` — `governed_story_keys()` + `is_governed_story_file()`.
  The governed set is derived from `sprint-status.yaml`'s `development_status` keys,
  which is the project's own authoritative story list, so it cannot drift the way a
  filename convention did: adding a story to the sprint makes it governed, with
  nothing to remember. It reuses the already single-sourced
  `sprint_status::load_sprint_status` rather than adding a second parser — `xtask`'s
  `lib.rs` now exposes that module so the helper resolves in both compilations.
- **Fails closed.** An unreadable `sprint-status.yaml`, or a `development_status`
  block yielding zero keys, is an `Err` — never an empty set. A gate that governs
  nothing passes for the wrong reason, and `findings.is_empty()` is blind to it.
- Converted, all seven walk sites named in the corrected row above:
  `check_bare_review_findings.rs`, `check_dev_model_tier.rs`,
  `check_dev_model_used_populated.rs`, `check_dev_record_completeness.rs`,
  `check_review_findings_resolved.rs`, and `check_epic_6_bridge.rs` **×2**
  (`check_7_1_5_bare_rf_count`, `check_7_1_5_dmu_missing_count`).
- `check_dev_model_tier.rs` needed one extra repair the filter swap alone would not
  have delivered: its `ENFORCE_FROM_EPIC` scoping skipped any story with no epic
  number, so `j1-*` would still have been exempt after becoming visible. A governed
  story with no epic is now enforced. "No epic number" was never a reason to exempt
  a story from recording which model developed it.

**ACCEPTANCE — the planted red, not the helper.**
`xtask/tests/d19_story_file_governance.rs` (10 vectors), CI-enrolled on the Blocking
`check-dev-record-completeness` job. A `j1-*` story file with a missing dev record
REDS a Blocking gate; the same defect under a numeric key also reds, so each vector
is about the DEFECT and not the filename; a complete fixture is GREEN so the reds are
not vacuous; an ungoverned `.md` carrying every defect reds nothing; a missing or
empty story list REDS rather than governing nothing; `epic-*` roll-ups and
retrospectives stay out.

**Newly governed as a result — the whole J1 lane, six files:**
`j1-crosshost-1a-frame-borne-delegation`, `j1-crosshost-1b-consent-proofs-and-gate`,
`j1-crosshost-2a-signable-heterogeneous-worker`,
`j1-crosshost-2b-cross-host-delegation-mechanism`,
`j1-crosshost-2c-two-host-signed-run`, `j1-demo-one-command-scene`.
All five converted gates are **GREEN** against them, so the lane's records genuinely
satisfy the discipline that had never been applied to them. Also verified green:
`check-dev-model-tier`, `check-epic-6-bridge`, `check-epic-close-coherence`.

**Out of scope, deliberately.** The row names two OTHER blindness mechanisms:
`check_epic_close_coherence.rs:215` (`head.parse().ok()?`) and
`check_epic_6_bridge.rs:820-828`'s hardcoded `"6-2"`/`"6-3"` prefixes. Neither is a
story-file *walker*: the first is epic roll-up scoping, and a story belonging to no
epic correctly does not appear in an epic's roll-up; the second is an
Epic-6-specific bridge assertion. Option (a) is about which story files a
story-file gate governs, and those two are not that.

**One pre-existing RED closed in passing, reported not hidden.**
`check-dev-record-completeness` was already RED at HEAD `7aa07ee3` on
`deferred-work.md:817` and `:820` — both `j1-crosshost-2b` review text asserting an
owner ("owned by j1-crosshost-2c", "owner: worker-grant hardening lane") that
resolves to no sprint-status key. Restated in the gate's own `ownerless and open`
vocabulary. The deferred items themselves are unchanged and still open.

## Rulings recorded by Story `14-0` (2026-08-26, at `9c5ae2db`)

Every sub-item below closes **by decision against its ID** (binding rule 1), including the ones
whose outcome is "no change" (binding rule 4 — silence is not an outcome). Each carries named,
re-verified evidence. **Where a row's own stated cause, subject, site or rationale was measured and
found wrong, the correction is recorded here rather than silently applied** — this register's single
most repeated finding is that *a row can be true when written and false when read*, and until now it
had no mechanism that noticed.

### D3 — RULED: ZERO kernel-Δ, repaired out-of-kernel, and re-scoped from an instance to a class

The FLAG-Winston ruling D3 asks for is **granted as: repair out-of-kernel, class-wide.**

- **The premise that this needs a kernel decision is near-vacuous.** The signature is *already*
  `Result<(), CapError>`, so the propagate repair is **+1 kernel-core line, +3 `maos-domain`, and
  ZERO broken call sites** — all five production sites already type-handle it (three via `?`, two
  via `let _ =`).
- **But it does not need even that.** `record_drop()` lives in **`maos-capability`** (954 lines of
  headroom, outside `check-kernel-baseline`'s scope and outside `kernel-crates.toml`). Making the
  drop observable there costs **zero kernel-core lines and zero re-pin**, and repairs **all seven**
  drop sites instead of the one D3 names. In-repo precedent is exact: `maos-telemetry`'s
  `OtelSink::drop_count()` **is** read by a gate (`otel_gates.rs:818`).
- **Three facts D3 omits, all verified, all part of the ruling.**
  (a) `audit_drop_count()` has **four references repo-wide** — its definition, its own unit test
  twice, and one integration test. **Zero production readers.** Any ratification resting on "at
  least it is counted" is a claim standing in for a control.
  (b) **`cap_tokens/mod.rs:280-282` is the same defect UNCOUNTED**, on the operator revoke path:
  `let _ = self.audit.try_send(Revoke{..}); Ok(())`, with no `record_drop()`. An Epic-1b reviewer
  patched `issue()` and `revoke_all()` for precisely this and **missed `revoke()`**; it has survived
  to HEAD. The class is therefore **eight**, not seven.
  (c) Writer-task death puts the process in **silent no-audit mode for its lifetime**: the I2 panic
  at `transparency_log.rs:1364-1406` runs inside a spawned tokio task and surfaces only as an
  `eprintln!` at drain (`main.rs:8204-8208`), exit code unchanged.
- **Constraint carried to the vehicle:** `cap_audit_backpressure.rs:121-124` asserts `drops > 0`, so
  a "never drop" repair **reds a shipped test**. `14-d3-audit-drop-observability` must rule on that
  test in the same breath.

**Vehicle:** `14-d3-audit-drop-observability`. **Kernel-Δ: ZERO**, so D3's own deadline trigger
("before any Epic 14 kernel-core edit") can no longer fire — which is why the row now declares that
anchor UNQUERYABLE and carries a mechanical re-anchor instead of pretending the old one still binds.

### D5 — DECOMPOSED into three named story keys; the kernel half rides D3's ruling

All four items verified **INTACT at HEAD**, and the item this preflight's own brief could not find
**does exist**: `deferred-work.md:546`, *"`decommission_region_key` hardcodes `completed: true`"*.
`deferred-work.md:549` is **not** a D5 item — retro §4 dispositions it *accepted risk*
(`RELEASE-HOLDS.md:49`).

- **`14-e1-erasure-attestation-honesty`** — folds D5.2 + D5.3 under one control: *no signed erasure
  attestation may assert a completion it did not observe.* ZERO kernel-Δ.
  `CategoryStatus::CoverageGap` **already exists** (`erasure/proof.rs:22-27`), so the row's claim
  that this "reopens the AC1 vocabulary decision" is **false**. Crates `maos-bin` + `maos-audit`,
  both at zero headroom → one measured grant under `kloc.toml:87`. **Scope IN.**
- **`14-e2-legal-hold-erase-serialization`** — D5.1 alone. KERNEL-TOUCHING, and **routed through
  D3's ruling, not a second escalation**: same crate, same ZERO-Δ fence, same moment. Two rulings on
  one crate in one afternoon is the single-source defect this project has paid for three times.
- **`v25-erasure-crash-reconciliation`** — D5.4 alone; the largest (≥4 crates, a persisted intent
  record and a startup reader). Same design as **ADR-059 residual #4**, also open and ownerless —
  scope them together or neither. **DEFERRED WITH A NAMED KEY**, because "defer to v2.5" had exactly
  one existing key (`v25-signed-transparency-log-artifact-identity`) and it is not a home for this.
  Minting a phase label instead of a key is the move this register exists to stop. D5.4 had **two**
  open homes (`deferred-work.md:547` and ADR-059 #7); both now point at the one key.

### D6 — CLOSED by decision, on measurement. The fork it poses is moot.

All three named defects were **repaired 2026-08-02** by `608facde`, titled *"fix(ci): clear
discipline gate blockers"* — a title naming none of it, which is D13's own warning about unreliable
titles firing again. The Vec-collect fix the row itself prescribed is at `private.rs:281-294`;
`io_lock` now spans all four entry points (`:639` / `:687` / `:746` / `:813`); traversal is
`statat(SYMLINK_NOFOLLOW)` + descriptor-anchored `open_dir_component` + `unlinkat`. Its first item
(`deferred-work.md:553`) had already been annotated CLOSED by 13.5i. **So D6 names one closed row
and three repaired ones, and it stayed open for 24 days.**

Closing it does **not** close what it never mentioned. The surviving **general non-atomicity of
private-tier erase** is re-filed as `v25-private-tier-erase-atomicity` and is this row's live
target, so binding rule 1 is honoured in both directions: nothing closes by implication, and nothing
stays open by omission.

### D2 — CLOSED by decision; the real residual is elsewhere and is worse

- The named defect was closed by **13.6e, commit `c45df0be`, 2026-08-07**. The stale-owner sweep ran
  **2026-08-08**, walked that exact file, annotated the rows on either side, and **skipped this
  one**. The stale row then propagated verbatim → retro C3 → `sprint-status.yaml` → **D2**. *D2 was
  authored describing a code site that had not existed for four days.* This is a **rule-1 violation
  in the inverse direction** — not closed by implication, but left open by omission after being
  deliberately addressed.
- The surviving `passed` / `product_claim` split is **ratified as the explicit claim boundary**. It
  already is one: `evidence_ledger.rs:1538` emits `"passed": blockers.is_empty()` while the same run
  emits `product_claim: NOT_PROVEN`, per-leg `ABSENT`, and a WOULD-HAVE-BLOCKED banner.
- **The real residual is filed as D20**, not folded in here. See that row.

### D1 — RESTATED to the subject set that actually has the defect

- **D1's second clause is close to inverted.** `check_vetting_attestation.rs:220-231` and
  `check_wasm_form_equiv.rs:244-256` **hard-fail** on any unmeasured leg, and `gate_common.rs:244`
  names the former *the reference implementation* of the vacuity guard. They are the **strictest**
  gates in the repo. D1 asks to add ABSENT vocabulary to two hermetic gates with no substrate to be
  absent. (Both do carry a `ran: bool` proxy the row does not mention, so "cannot represent ABSENT
  at all" overstates by one field.)
- **The boundary D1 fears expanding is principled and measured**, not arbitrary: `discipline.yml`
  has exactly **four `services:` blocks across 158 job keys**, owned by exactly the four ledger
  gates. **Ledger membership == CI substrate provisioning.**
- **The real subject set is seven gates that report `passed: true` over absent evidence**, and D1
  names none of them: `check_pentest_gate.rs:79`, `check_red_team_gate.rs:127`,
  `check_third_party_trial.rs:202`, `check_cna_registration.rs:126`, `check_cross_form_equiv.rs:183`,
  `check_escape_detector.rs:691`, `check_migration_merkle.rs:276-281`.
- **RULING: ratify the out-of-ledger evidence contract; do NOT expand the ledger.** Prior art already
  ships — `check_rto_gate.rs:77-84` (*"No evidence — SKIPPED (not a silent PASS)"*) and
  `check_migration_merkle.rs:169-193`. That pattern becomes the separate evidence contract for the
  seven, and the four-contract ledger stays derived from four journey contracts.
- The Family-A/B vocabulary is **not** contradictory (a premise this preflight raised and then
  disproved): families are defined by leg-struct shape (`13-6e…md:85-87`) — 2 A + 10 B = 12, 13.6e
  migrated 2 B into the A shape, ledger = 4, outside = 8. Both usages are arithmetically consistent.

### D4 — SPLIT three ways; neither named vehicle could host the enforcement half

**Both halves of D4's rationale sentence are false**, and the sentence propagated verbatim through
four artifacts, each citing the previous:

- *"The final harness derives region honestly from the signed entry"* — it does not.
  `cross_team_crossing_13_6b.rs:2138-2145` is a hard-coded `match team { "team-a" => "region-a", … }`,
  and **both** the `TeamEntry` (`:2170`) and the daemon env (`:2267`) read that same literal. They
  agree by shared constant, not by derivation.
- *"production does not"* — also false. `cross_team_consent.rs:126-145` derives team keys from the
  **verified manifest** region. Production has both an honest path and an unreconciled one, **and
  they can disagree in the same process.**

The three obligations are now three rows — **D4a** (runtime enforcement, new vehicle, because 14-7
and 14-8 are both static-scan stories and cannot host a runtime boot check, so D4's own target *and*
its stated fallback were both wrong), **D4b** (registration, stays at 14-8 behind 14-7), and **D4c**
(classification + crypto at 14-9, where the open question is whether a slot for "non-secret value,
key-derivation input, integrity-critical" is added or the boundary is stated).

### D10 — RATIFIED and BOUNDED; the row as written was moot

D10 forbids *"a third unscoped grant by implication."* Since it was filed, **two more landed**:
`j1-crosshost-2b` 4654→4669 and `j1-crosshost-2c` 4669→**4785** (`kloc.toml:252`), each
self-authorizing via `kloc.toml:87`. The prohibition was overtaken by events before its deadline.

**RULING — ratify the escape valve, and bound it.** `kloc.toml:87` stands as the standing mechanism.
The bound is the one D10 was actually protecting: **no grant without a measured delta and a named
driver in the annotation**, and **no grant by implication** — an adjacent story's growth may never
be absorbed silently. That bound is now mechanical for this register (`check-decision-register`) and
for kernel-core (zero headroom by construction); for the remaining crates it stays an annotation
discipline, which is stated here as a boundary rather than claimed as a control.

### D17 — the load-bearing claim is FALSIFIED; withdrawn and re-derived

D17 asserts the breach is *"arithmetic downstream of D13 (+685), D14 (+50) and D15 (+41) — not a
fourth independent overrun."*

**Measured at HEAD: the aggregate is 151391 / 147057 = +4334**, of which D13 + D14 account for
**736**. The remaining **83% is growth in crates now green under re-based ceilings**. The claim is
**withdrawn**. The re-derivation is the honest form: the aggregate is the only instrument that
catches distributed growth no per-crate reserve can see, and most of what it is catching is not the
three named debtors.

D17's implied prohibition on re-basing has likewise been overtaken: `kloc.toml:61` permits
recalculation *"under an explicitly authorized measured grant"*, and that door has been used
repeatedly since — including by this row's own D13(a) predecessor.

### D18 — REOPENED. Marked RESOLVED; substance unimplemented; deadline blown four stories ago.

This is the most dangerous row in the register and it is the reason AC1 exists.

- `router.rs:2021-2030` still collapses both `ConsentUnclassified` variants into a stringly
  `CrossHostRouteFailure`, discarding the typed `UnclassifiedReason`.
- `grep Unclassified crates/maos-domain/src/iac_bus_types.rs` → **zero hits**. The promised variant
  was never added.
- Its re-pinned deadline — *"before `j1-crosshost-2b` writes its first line"* — **blew four stories
  ago**: 2b, 2c, 2d and 2e are all `done`, while `14-4` sits `backlog`.
- The register had **no expired-vs-resolved distinction**, so a reader skimming the RESOLVED tag
  could not see any of that. It has one now, and `check-decision-register` reds on it.

**Status returns to OPEN**, target `14-4-v2-0-sweep-operational-surfaces`, with the original
code-event anchor declared UNQUERYABLE and a mechanical re-anchor attached. The 2026-08-15
measurement recorded in the row's own resolution column stands and is not withdrawn — what is
withdrawn is the RESOLVED tag over an unimplemented substance.

### Deadline re-anchoring (D5, D6, D16): `14-4` is not a coherent anchor

Nothing blocks `14-4`; its only dependency (11-7) is `done`; and 14.1–14.6 are *"largely
parallelizable"*. So `14-4` can leave `backlog` **before** `14-1`, firing D5/D6's deadline earlier
than D1–D4's and inverting the register's own tiering — and if `14-4` is never picked up, they never
bind at all. **D5, D6 and D16 are re-anchored to *"before `14-1-100-host-churn-scale-envelope`
leaves `backlog`"***, which `14-0` already blocks, so the anchor is enforceable by construction.
`14-4` remains correct for **D7** and **D12**, which are causally its.

### The epic's honest opening posture, stated rather than implied

`kloc-check` is **Blocking and RED at HEAD** on `maos-domain` (D14 → 14-7) and `_aggregate_hardfail`
(D17). D13(a) is discharged and `maos-kernel-core` is green at **18933 / 18933 with zero headroom** —
deliberately, so every further kernel-core line still costs its own measured FLAG-Winston grant and a
HISTORY row. **"All gates green" is NOT an available done criterion for Epic 14's opening**, and no
story may manufacture one by absorbing another row's debt. A standing red with named debtors is an
honest state.

## Evidence filed against open rows

Evidence is **not** closure. A row below stays open at its stated deadline with its stated owner;
these entries exist so the owner arrives at preflight with the facts already gathered, and so binding
rule 1 is honored — the finding is recorded against the **ID**, not left in an adjacent story's
change log.

### D11-E1 — `example_spirit_regen`: budget-charged, zero execution path

**Filed** 2026-08-14 by the `j1-crosshost-1a` preflight round-table. **Owner unchanged** (Winston +
Murat). **D11 remains OPEN**; substance is settled at 14-6's preflight.

- **The finding.** `xtask/src/example_spirit_regen.rs` — **133 tokei-code lines**, declared
  `mod example_spirit_regen;` at `xtask/src/main.rs:108`, exposing one public item (`pub fn run`,
  `:12`) with **zero callers repo-wide**: no dispatch arm, no CI job, no test. Five of its functions
  have emitted dead-code warnings on every build since it was authored.
- **Why it widens D11(b) rather than instancing it.** D11(b) read *"in-`src` `#[cfg(test)]` modules
  are KLOC-budget-charged but never CI-executed."* This module is **not** `#[cfg(test)]` — it is
  ordinary production code, charged and never executed at all. The row was scoped around the
  instance that had been found rather than the category, and **would never have caught this.**
  D11(b) is therefore widened to *"budget-charged code with no execution path"*, of which
  `#[cfg(test)]`-but-unexecuted is one member.
- **Why the numbers moved.** `EXPECTED_GATES` measures **36 vs 67** at `af788c3e` (the row said 66),
  going to **37 / 68** when `check-j1-loopback-delegation` enrolls. C1's single-source rule applies to
  this register too: a mechanical deadline queried against stale numbers is not a query.
- **Proposed mechanical form (recorded, NOT built).** `#![deny(dead_code)]` on `xtask` plus
  `-D warnings` in CI. It reds today across ~12 further sites in live gates —
  `check_abi_ratification.rs:22`, `check_pentest_gate.rs:24`, `check_red_team_gate.rs:59`,
  `check_skill_conformance.rs:35`, `check_third_party_trial.rs` ×3, `check_epic_6_bridge.rs` ×3,
  `check_fkcs.rs` ×2, `evidence_ledger.rs` ×2. **That triage is 14-6's, explicitly not
  `j1-crosshost-1a`'s.**
- **What `j1-crosshost-1a` does and does not do.** It deletes **only** this one module, because the
  133 reclaimed lines fund its gate skeleton outright — so no `xtask` grant is requested on an
  estimate and `kloc.toml:60-65` stands unbent. It does **not** fix the class, and it does **not**
  close this row.
- **Note for the owner.** Seven consecutive `xtask` ceiling re-bases preceded the first audit that
  looked for dead code, and it found 133 lines. That is evidence bearing on D11's open question of
  whether the growth rate is instrument or accretion.

### D11-E2 — `clippy` is installed and never invoked

**Filed** 2026-08-14, same round-table. **Routing undecided — may belong to Epic 0 rather than D11;
14-6's owner re-homes it.**

`.github/workflows/discipline.yml:44` installs `clippy` as a toolchain component. `cargo clippy`
appears in **no** workflow, and no job sets `RUSTFLAGS` or `-D warnings`. An auditor reading
`components: rustfmt, clippy` reasonably concludes the repo lints; it does not. Same species as
D11-E1 — an instrument that is not an instrument — which is why it is filed here rather than
separately, pending the owner's routing call.

### D11-E3 — in-`src` test modules are load-bearing on the budget (MEASURED)

**Filed** 2026-08-14 from `j1-crosshost-1a`'s implementation, not its preflight. **Owner unchanged**
(Winston + Murat). **D11 remains OPEN.**

D11(b) said in-`src` `#[cfg(test)]` modules are budget-charged but never CI-executed. That was
recorded as an accounting oddity. Implementing `j1-crosshost-1a` turned it into a **binding
constraint with numbers**, which is the fact the owner needs at 14-6:

- The story's correct implementation put `maos-bin` at **16411 / 16178 (+233 OVER)** and `maos-iac`
  at **6889 / 6888 (+1 OVER)**. The story requests **no grant** and `kloc.toml:60-65` forbids one on
  an estimate, so the only lawful move was decomposition.
- Relocating in-`src` test modules to `tests/` (zero-cost per `xtask/src/kloc_check.rs:167-190`)
  recovered the whole breach: `maos-bin` **16411 → 16176**, `maos-iac` **6889 → 6851**. No behaviour
  changed; the tests became **CI-executable for the first time**.
- So D11(b) is not merely an oddity: it is **charging crates for code CI never runs, and that charge
  is large enough to force architectural decisions.** Two crates' budgets were decided by test text.
- Corollary for the owner: the mechanical form should probably *exclude* in-`src` test modules from
  the charge **or** refuse them outright — but either way the current state prices a crate for code
  that is never executed, while the same text in `tests/` is free and does run.

**A second instrument-that-is-not-an-instrument, found the same way.**
`crates/maos-iac/src/adapter/orchestrator_dispatch.rs:36-39` documents
`MAOS_ORCHESTRATOR_DISPATCH_WINDOW_NS` as *"operator-configurable … (composition root surfaces it on
the daemon)"*. That variable appears **nowhere else in the repo**: the only caller passes the
hardcoded `DEFAULT_ORCHESTRATOR_DISPATCH_WINDOW_NS`. Same species as D11-E2 and as
`priority_weight` (`j1-crosshost-1a` AC1.2) — a documented control that never existed. Routing is
14-6's call.

### FLAG-E4 — no gate observes `maos-kernel-core`'s **re-exported** ABI surface

**Filed** 2026-08-14 from `j1-crosshost-1a` AC5.1. **Routing undecided** — this is an instrument gap,
not an Epic-14 residual; 14-6's owner re-homes it or hands it to Winston as an ABI-governance row.

`j1-crosshost-1a` was written believing that adding `Mailbox::install_a2a_router` would be caught by
`abi-diff` / `check-abi-ratification`, because `crates/maos-kernel-core/src/iac.rs:13` is
`pub use maos_iac::*;` and the method therefore **does** grow kernel-core's public API while
`src_lines` stays pinned at 24472. All three candidate gates were run. **None of them can see it:**

- `abi-diff` (`xtask/src/abi_diff.rs:8`) and `check-abi-ratification`
  (`xtask/src/check_abi_ratification.rs:122-137`) both scope to
  `crates/maos-spirit-abi/Cargo.toml` **only**.
- `check-service-boundary` (`xtask/src/check_service_boundary.rs:312-329`) walks
  `maos-kernel-core/src`'s own AST via `syn`; line 13 is a `syn::Item::Use`, which never expands to
  the re-exported methods.

Consequence: `xtask/kernel-api-classes.toml`'s `Mailbox::*` rows — including the six that predate
this story and the one it adds — are **documentation, not enforcement**. Nothing reads them for
re-exported symbols. The card's ZERO-line-Δ claim is true and verified; the "no ABI change" claim it
implied is **unverifiable by any current instrument**, which is the more useful finding. The story
re-pinned nothing and demanded no ratification, because no gate demanded one.

### FLAG-E5 — FR21's dispatch gate is chronological, not causal (found by first production emitter)

**Filed** 2026-08-14 from `j1-crosshost-1a` AC3. **Belongs to Story 6.2's owners, not to Epic 14** —
recorded here only because it has no row of its own and binding rule 1 forbids leaving it in a
story's change log.

`check_orchestrator_distillate_required`
(`crates/maos-iac/src/adapter/orchestrator_dispatch.rs`) treats **any** `TaskComplete` row inside a
**60-second wall-clock window** as a predecessor of an Orchestrator `TaskAssign`. That proxy cannot
distinguish *"a follow-up dispatch inside one fan-out"* from *"the first dispatch of a new process"*.

`j1-crosshost-1a` is the **first production emitter** of an Orchestrator `TaskAssign` (before it,
`assign_frame` had zero production callers), so the false positive becomes reachable now.

**Reproduction (measured):** two `maos run spirits/topologies/j1-founder-loop.toml --once` invocations
against the **same** `XDG_DATA_HOME` inside 60s. Run 1 exits 0. Run 2 exits 1 with
`EOrchestratorDispatchRawOutput`, because run 1's own completion frame is in the window.

Why it is a false positive: the delegation frame carries `prior_distillate_ref: None` and an empty
`scope` — it **references nothing**. `docs-site/docs/errors/EOrchestratorDispatchRawOutput.md`
describes the error as firing when a dispatch *"references raw worker output"*. This dispatch does
not.

**What `j1-crosshost-1a` deliberately did NOT do**, and why each was rejected:

- **Not relaxed.** Setting the window to 0, or dropping `from.role`, would be "fixing a red assertion
  by relaxing it" — the exact move AC4 exists to stop.
- **Not faked.** Minting a synthetic `Distillate` row purely to satisfy the ref check would be a
  claim standing in for a control.
- **Not bypassed.** Emitting via `Mailbox::deliver` instead of `IacBusAdapter::deliver_typed` would
  route the frame around a kernel permission check from the composition root.
- **Not repaired.** Scoping succession to the emitting process (the TL already stores `boot_nonce`)
  is the likely correct fix, but it changes FR21 semantics and needs Story 6.2's owners.

It is instead **fail-closed and self-explaining**: the refusal names the window and the owner
(`crates/maos-bin/src/delegation.rs`). Every in-repo caller uses a fresh temp data home, so no test
is affected; the exposure is an operator re-running inside 60s on a real `MAOS_HOME`. The proper fix
is rung 2's stable `task_id` correlation, which `j1-crosshost-1a`'s "does NOT do" list already
assigns to `j1-crosshost-2`.

## Traceability

- Retro §4 disposition table: `_bmad-output/implementation-artifacts/epic-13-retro-2026-08-11.md`.
- Residual register with per-line owners:
  `_bmad-output/implementation-artifacts/deferred-work.md` — the handed rows now
  carry `epic-14`, `14-3`, `14-4`, `14-6` owner strings, verified by
  `xtask check-dev-record-completeness` (0 violations, 0 owned-but-deferred).
- Accepted risks (**not** in this register — they are closed by decision, with
  boundaries stated): `RELEASE-HOLDS.md` §Claim boundaries, retro action **C4**.

## Out of scope for this register

The retro also **closed 2** residuals and **accepted risk on 7** groups. Neither
set appears above. Accepted risks are not deferred work: they bound what the
release may assert and are recorded in the GA ledger, not in a decision queue.
