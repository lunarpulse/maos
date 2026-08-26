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

## Binding rules

1. **No residual may be closed by implication.** A row closes only when a
   decision is recorded against its ID with named evidence. Shipping adjacent
   work does not close a row.
2. **Deadlines are mechanical, not calendar.** Each is anchored to a state
   transition observable in `sprint-status.yaml`, so "did we miss it" is a
   query, not a judgement. This follows C1's single-source rule.
3. **`D1`–`D4` block `14-1`.** C2 requires them settled *before 14.1 opens*;
   `14-1` and `14-2` inherit the weaker half of the evidence discipline
   (retro C3) and must not rely on an unsettled advisory classification.
4. A row whose decision outcome is "no change" is still **closed by decision**,
   with the rationale recorded. Silence is not an outcome.

## Decisions

| ID | Residual (retro §4) | Decision required | Target story | Deadline (mechanical) | Owner |
|---|---|---|---|---|---|
| **D1** | Eight Family-B gates sit outside the four-contract evidence ledger; `check-vetting-attestation` and `check-wasm-form-equiv` cannot represent `ABSENT` at all. | Does the ledger expand to Family-B, or do Family-B gates get a separate evidence contract? The ledger is deliberately derived from four journey contracts and must not silently expand. | **14-0** decides; implementation lands with retro **C3** | Before `14-1` leaves `backlog` | Murat + Winston |
| **D2** | Skipped `AdvisorySubstrate` legs report `passed: true` under the shared house pattern. | Ratify `absent != green, skipped != passed` for advisory legs, or ratify the current pattern with an explicit claim boundary. Unavailable substrate is unmeasured, never proof. | **14-0** decides; enforced in **14-1**, **14-2** | Before `14-1` leaves `backlog` | Murat + Winston |
| **D3** | Fallible `record_invocation` can drop an audit event while returning `Ok` (`crates/maos-kernel-core/src/capability/mod.rs:332-351`). | Kernel-core fallible-path repair requires a **new FLAG-Winston decision**. Approve the kernel-Δ, or ratify the drop with a stated boundary. | **14-0** obtains the ruling; implementation story assigned by the ruling | Before any Epic 14 kernel-core edit | Winston (FLAG-Winston) |
| **D4** | `MAOS_REGION_HOME` is never reconciled against the signed `TeamEntry.region`. | Enforce equality in production, or register the variable and state the deployment-integrity boundary. The final harness derives region honestly from the signed entry; production does not. | **14-8** (full workspace env surface) — `MAOS_*` read registration is already its scope | Before `14-1` leaves `backlog` (classification only; enforcement may follow 14-7) | Winston + John |
| **D5** | Legal-hold check-then-act race; revocation failure signed as zero; decommission receipt claims completion; mutation-to-audit crash gap. | Scope into **distinct controls**. These are four lifecycle/atomicity questions and must not be folded into the already-proven collective erase. | **14-0** decomposes into named stories | Before `14-4` leaves `backlog` | Winston + Murat |
| **D6** | Private-tier residue, erase races, directory-iteration hazards. | Kernel filesystem/atomicity design, or retain the recorded security limits until separately scoped. | **14-0** rules scope-in vs defer-to-v2.5 | Before `14-4` leaves `backlog` | Winston |
| **D7** | `CrossWallRecallRefusal` collapses six variants into the token `refused`. | A shipped CLI operator surface preserving cause only in free text is insufficient for a machine-readable operator outcome. Decide the typed outcome shape. | **14-4** (v2.0 sweep — operational surfaces) | Before `14-4` leaves `backlog` | John + Amelia |
| **D8** | `check-fkcs` `admission-path-unmodified` is RED, held advisory. | Frozen-kernel-conformant re-pin, or retain the exact bounded hold. Admission sources genuinely changed in 13.4; re-pinning without review would re-can an unreviewed floor. | **14-3** | Before `14-3` leaves `backlog` | Winston + Murat |
| **D9** | A vetting lapse cannot refuse a crossing. | The only demonstrable vetting boundary is upgrade/promotion, not crossing. Accept that boundary explicitly, or scope a crossing mechanism — **no invented mechanism**. | **14-3** (already owns the v2.5 vetting graduation ledger) | Before `14-3` leaves `backlog` | Murat |
| **D10** | `maos-a2a-core` third consecutive KLOC grant / v2.5 team-crypto identity. | The v2.5 ecosystem graduation ledger is the explicit home for an external/team-identity boundary. **No retro may ratify a third unscoped grant by implication.** | **14-3** preflight | Before `14-3` leaves `backlog` | Winston + Lunarpulse |
| **D11** | No gate reconciles kernel-pin HISTORY; **budget-charged code with no execution path** (widened 2026-08-14 — see D11-E1); `EXPECTED_GATES` is hand-maintained (**36 entries vs 67 `check_*.rs` at `af788c3e`; 37 / 68 once the J1 loopback gate lands** — corrected 2026-08-14). | Make pin history, execution coverage, and workflow-derived gate registration mechanically auditable. Carries **E11-A6** forward to mechanical close. | **14-6** (ceiling instrument + retro-residual discipline) — retro **C5** | Before `14-6` leaves `backlog` | Winston + Murat |
| **D12** | `audit_query_latency` bench has been broken since 9.1 (`"capability.invoke"` vs accepted `"capability.invocation"`); it has run **zero** times. | Repair or retire the bench. | **14-4** (already owns the v2.0 operational-surface sweep) | Before `14-4` leaves `backlog` | Amelia |
| **D13** | **`maos-kernel-core` kloc ceiling breached: 18933 tokei CODE vs 18248 (+685)** while its PIN is green (24472 = pinned). The two instruments read the same tree differently and both are correct: the pin counts **physical lines in every `.rs`** (`xtask/src/check_kernel_baseline.rs:99-110`, `content.lines()`), the ceiling counts **tokei CODE with tests/benches/examples/fuzz excluded** (`xtask/src/kloc_check.rs:163-213`) — anti-drift vs anti-growth (`kloc.toml:53-59`). One was updated, the other was not. **Arithmetic attribution:** `sprint-status.yaml:230` records that as of Story 13.5j the ceiling still had **273 spare** (17941→17975 / 18248). Then the Epic-5 review-findings closure spent it: `af788c3e` put net **+878** into `crates/maos-kernel-core/src` across 34 files and took a documented **FLAG-Winston PIN grant** (`xtask/kernel-core-baseline.toml`: "net +741 … authorized as the bounded repair of the reopened Stories 5.1/5.2/5.4/5.5a findings") but **never the paired kloc ceiling grant**; `2688c6d0` added a further +52 and is that same vehicle's own `baseline_commit` (`spec-epic-5-review-finding-closure.md:7`, scope includes `maos-kernel-core/src` at `:47-50`). Attribution rests on diff stats and the baseline file's own authorization text, **not on commit titles** — titles are unreliable here (`13-6-reza…md:388` records that `b568a052`'s title names 13.6a while containing zero 13.6a work). | **Split, because the repair and the instrument are different questions.** (a) The *breach repair* belongs to the vehicle that caused it and already holds the authorized measured delta: take the paired measured ceiling grant with a HISTORY row, or show the decomposition. `kloc.toml` states the rule "must never block a correctness or compliance repair", and this is one. **Its closure gate list is the hole that let this through — `spec-epic-5-review-finding-closure.md:79` runs `cargo fmt --all --check` and the workspace suites but NOT `kloc-check`; add it.** (b) The *instrument* question — whether kernel-core may be this large, decomposition vs policy — stays with D11/C5 and must NOT be granted away: `kloc.toml:407-408` keeps kernel-core deliberately tight *because Epic 14 declares ZERO kernel-Δ*, and a retroactive grant from a ZERO-Δ story would invert ADR-038. **14-6 may not erase this red with a grant it has no measured delta to justify.** | (a) **`spec-epic-5-review-finding-closure`** (status `in-progress`); (b) **14-6** — instrument only, via **D11** / retro **C5** | (a) Before `spec-epic-5-review-finding-closure` reaches `done`; (b) before `14-6` leaves `backlog` | (a) Winston + Amelia; (b) Winston + Murat | | **(a) RESOLVED 2026-08-26 — founder grant taken, breach discharged.** `xtask/kloc.toml:195` `maos-kernel-core = 18248 -> 18933`, **EXACT MEASURED / ZERO HEADROOM**, authorized by Lunarpulse under `kloc.toml:61`'s "explicitly authorized measured grant" door. `kloc-check` now reports `| maos-kernel-core | 18933 | 18933 | ok |`; the +685 red is gone. Chosen over the formula's 19312 (measured + ceil(2%) = 379) deliberately: free growth capacity in the kernel under a ZERO-kernel-Δ epic is the ADR-038 inversion **(b)** forbids of 14-6, and at zero headroom every further kernel-core line still costs its own measured FLAG-Winston grant and a HISTORY row. Precedent: `maos-bin` 16870/16870 (D15, same founder, 2026-08-15, also tighter-than-formula). ⚠ **THIS ROW'S OWN ARITHMETIC WAS WRONG AND IS CORRECTED, NOT OVERWRITTEN.** Measured by `git log --numstat` scoped to `crates/maos-kernel-core/src`: `af788c3e` is **net +741 over 36 file-rows**, not the "+878 … across 34 files" asserted above — and +741 is *exactly* the figure `kernel-core-baseline.toml:465` had already authorized, so this row overstated the delta its own baseline file recorded correctly. The row's second figure survives: `2688c6d0` **+52** (+59/−7) reproduces. Full attribution is in the `kloc.toml:195` annotation. ⚠ **The deadline was never queryable.** *"Before `spec-epic-5-review-finding-closure` reaches `done`"* names a story that has **no `sprint-status.yaml` key** — only a file with its own frontmatter `status: 'in-progress'`. A status no tracker records cannot transition, so binding rule 2 was unsatisfiable on this row from the day it was written; the grant is discharged ahead of a deadline that could not have fired. That defect is 14-0's to fix (one sprint key also un-blinds `governed_story_keys()`). **STILL OPEN from this row's own prescription:** the closure-gate hole — `spec-epic-5-review-finding-closure.md:79` runs `cargo fmt --all --check` and the workspace suites but NOT `kloc-check` (verified at HEAD: zero `kloc` hits in that file). Taking the grant does not add the gate that would have caught this. **(b) remains OPEN and untouched** — whether kernel-core may *be* this large is the instrument question, still 14-6's via D11/C5, and a grant is not an answer to it. `_aggregate_hardfail` is **not** moved by this grant and stays RED under **D17**.
| **D14** | **`maos-domain` ceiling breached: 8694 vs 8644 (+50).** Cause is NOT an Epic-14 driver: `baf83880` grew `crates/maos-domain/src/halt.rs` by +156/−14 — the Story **3.3** halt-resolution work (`sprint-status.yaml:41`, `done` 2026-08-14), corroborated by `j1-crosshost-1a-frame-borne-delegation.md:315-316` attributing the breach to Story-3.3 halt lines. | Story 3.3 closed the same day with a full audit; reopening it to carry a +50 grant is churn. `kloc.toml:218` names **14-7** as this crate's growth driver by design ("the placement was chosen specifically to avoid adding a kernel-crate-set member"), and 14-7 must measure `maos-domain` at its own closure regardless. So: does 14-7 **explicitly expand an AC** to absorb the pre-existing +50 as a measured grant, or does 3.3 reopen for the grant? **Recording 14-7 as target does not decide the substance** — 14-7's ACs today cover extraction *into* the crate, not Story-3.3 halt growth, so absorbing it requires a stated AC expansion, not a silent inheritance. | **14-7** (with an explicit AC expansion) | Before `14-7` leaves `backlog` | Winston + John |
| **D15** | **`maos-bin` ceiling breached: 16211 vs 16178 (+33) at HEAD.** Cause: `6827dc87` (`j1-crosshost-1a`) added `delegation.rs` +290, `topology.rs` +110, `lib.rs` +10, `main.rs` net +146, `env_contract.rs` −5 (716 insertions / 165 deletions; net ≈ +551 physical). 1a is `done`. Story `j1-demo-one-command-scene` added a further **+8** (the `--once` drain drop-order fix), taking it to 16219 / **+41**; that story's grant was **xtask-only** and explicitly does not cover this. | 1a is closed and its ceiling debt has no vehicle. **`j1-crosshost-1b`** is the open successor in the same lane, already carries the measurement discipline ("re-measure, and only ask WITH the measurement attached"), and will itself touch `maos-bin`/`xtask` — so it can take one measured grant covering 1a's +33 and the demo's +8 together. Alternative considered and rejected as primary: **14-7**, whose AC2 migrates the `maos-bin` env registry out and may *reduce* the crate — a speculative future reduction is not a repair, and it is far behind 1b in the queue. | **`j1-crosshost-1b`** | Before `j1-crosshost-1b` leaves `backlog` | Winston + Amelia | **RESOLVED 2026-08-15 — Lunarpulse ratified `maos-bin = 16219`, EXACT MEASURED / ZERO HEADROOM, over the formula's 16544** (round-table consensus: Winston, Murat, Amelia, John, Mary, Vex; Dana dissenting-and-answered). Applied at `xtask/kloc.toml:264` with per-commit attribution and the `kloc.toml:87` correctness-repair pointer in the annotation. `maos-bin` is GREEN at HEAD. Rationale: the formula would grant 325 lines to a crate that is 73% `main.rs` with no decomposition scheduled; tighter-than-formula is house style (`xtask` grant, line 203); zero headroom is precedented and deliberate (`maos-a2a-core` 4654/4654). Does NOT cover the aggregate — see **D17**. |
| **D16** | **Suite-wide test-isolation defect in `crates/maos-bin/tests/`, not a single flake.** `cross_wall_recall_live_path_uses_verified_state_and_home_team` (`cross_team_consent_13_3.rs:243`) fails **5/5 at HEAD** under default parallel `cargo test -p maos-bin` and passes **3/3** with `--test-threads=1`. `std::env::set_var("MAOS_HOME", …)` is process-global and the locking is inconsistent across three files: `cross_team_consent_13_3.rs` (`RestoreMaosHome` Drop `:40-47`, locked test `:243-247`, **unlocked** `set_var` `:502-505`, `LIVE_LOCK` `:534`); `cross_team_crossing_13_6b.rs` (`:2725-2727` mutates with no lock in context, Restore `:2761-2768`); `cross_wall_log_read_13_6d.rs` (`env_lock` `:15-17`, locked `:65-68`, but `seed_remote_artifact` mutates at `:31-33`). Pattern authored by Story **13.3** (`e58d0df0`) and propagated through 13.6a/b/d/e — all closed. | Decide the isolation mechanism for the whole suite, not a patch to one test: a shared env guard every toucher must take, or per-test process isolation, or removal of process-global env from the suite. **`--test-threads=1` is a masking workaround, not a resolution** — if it is adopted it must be recorded as a stated boundary with the reason, per binding rule 4. No existing Epic-14 vehicle's ACs cover runtime test isolation (14-6 is the ceiling instrument, 14-7 is a static env registry/scan, 14-4 is canary/push/installers), and the authoring stories are closed — so this follows the **D5/D6 pattern**: 14-0 decomposes it into a named story rather than inventing an ID here. | **14-0** decomposes into a named story | Before `14-1` leaves `backlog` (a red suite under default flags poisons every later story's evidence) | Murat |
| **D17** | **`_aggregate_hardfail` is RED at 147549 / 147057 (+492) and it is NOT self-clearing.** Two things were established by measurement at the `j1-crosshost-1b` round-table (2026-08-15) and must not be re-litigated from memory: (a) the breach is **arithmetic downstream of D13 (+685), D14 (+50) and D15 (+41)** — it is not a fourth independent overrun; and (b) **re-basing those crate ceilings does NOT move the measured aggregate** (measured stays 147549), so the key stays red even after all three D-rows land in full. It is the only instrument that catches distributed growth no per-crate reserve can see, and it is *meant* to sit red while debtors repair. **Prior framing corrected:** the aggregate is neither 'unowned' nor unreachable by a bridge story — `kloc.toml:61` permits recalculation *'at an epic retrospective, **or** under an explicitly authorized measured grant'*, and Stories 13.6d, 13.6e and the epic-orphaned `j1-demo-one-command-scene` all used the second door. | **Who re-bases it, and on what evidence.** `j1-crosshost-1b` REFUSED the grant deliberately (its contribution is zero, and granting it would turn the CI signal holding D13's +685 to account green — which D13 already forbids of 14-6, and 1b is further from the delta). So the re-base belongs to a vehicle with a **measured delta to justify it**, or to the next epic retrospective. Decide which, and record that `kloc-check` exits 1 until then — a standing red with named debtors is an honest state, not an outage. | **14-6** (ceiling instrument), or the Epic-14 retrospective | Before the v2.2 wave closes | Winston + Murat |
| **D18** | **`map_a2a_error_to_iac_bus` flattens the A2A deny vocabulary above the router** (`crates/maos-a2a-core/src/router.rs:1671-1783`). Both `IntentDenied{Send}` (`:1673-1683`) and `IntentDeniedAtPeer` (`:1684-1690`) collapse to the same `IacBusError::CrossHostIntentDenied`, and **both** `ConsentUnclassified` variants collapse to a stringly `IacBusError::CrossHostRouteFailure` (`:1773-1782`); `DelegationLeg::delegate` (`crates/maos-bin/src/delegation.rs:149-171`) then stringifies even that. **Consequence:** a cross-host operator cannot distinguish `-32001` (policy refused you) from `-32009` (policy could not classify you) — one is the system working, the other is the system blind — and the unclassified *reason* (`Absent`/`NonCanonical`/`Oversized`) is unrecoverable. The non-conflation invariant is real and pinned at the router seam (`fail_closed_8_8.rs:216-240`); it is destroyed one layer up. Found by the `j1-crosshost-1b` preflight, which proves the refusals at the router seam **because it cannot prove them anywhere else**. | **Decide the typed cross-host deny outcome an operator sees.** `j1-crosshost-1b` correctly does NOT fix it: `maos-a2a-core` is at ZERO kloc headroom and **D10** forbids a third unscoped grant, so widening this is a scoped decision, not a side effect. Same shape as **D7** (`CrossWallRecallRefusal` collapsing six variants into `refused`) — consider deciding them together. **Deadline is deliberately BEFORE rung 2 writes code, not before it closes:** `j1-crosshost-2` builds the first real cross-host operator surface, and if it is built on the flattened error the defect becomes load-bearing. NOTE: this was NOT filed 'against `j1-crosshost-2`' — that story has a sprint-status row and (at filing time) no story file; a deferral into a document that does not exist is not a deferral. | **14-4** (v2.0 operational-surface sweep, already owns D7) | Before `j1-crosshost-2` writes its first line | John + Vex | **RESOLVED 2026-08-15 (Lunarpulse) — the 'precondition with no budget' paradox was built on an UNMEASURED premise and does not survive measurement.** Three corrections: (1) **The `-32001` pair is ALREADY distinguishable.** `IntentDenied{direction}` (`router.rs:1673-1683`) and `IntentDeniedAtPeer` (`:1684-1690`) both produce `CrossHostIntentDenied`, but with `direction: Send` vs `direction: Accept` — a consumer CAN tell the send seam from the accept seam today. The residual defect there is narrower and semantic: `IntentDeniedAtPeer` stuffs the NACK **message** into a field named `intent`, while the sibling arm puts a real intent string in it. The field lies about itself; it does not erase the distinction. (2) **The real loss is the UNCLASSIFIED pair.** `ConsentUnclassified` and `ConsentUnclassifiedAtPeer` (`:1773-1782`) both collapse into stringly `CrossHostRouteFailure(String)`, discarding the typed `UnclassifiedReason` (`Absent`/`NonCanonical`/`Oversized`) and the direction. That — not the deny pair — is D18's core. (3) **MEASURED COST: `maos-a2a-core` ≈ ZERO net lines.** The two arms are 5-line `format!` constructions; replacing each with a 5-line typed struct construction is net ~0. The new variant lands in `maos-domain::iac_bus_types` (`:14-40`) at ~+6 lines. **So D10's ZERO-headroom wall was never in the way, and no maos-a2a-core grant is required.** The `maos-domain` +6 rides with **D14**, whose owner (14-7) is already required to make an explicit AC expansion for that crate — fold it there rather than opening a second vehicle. **DEADLINE RE-PINNED: 'before `j1-crosshost-2b` writes its first line'** (was: before `j1-crosshost-2`). This is not a weakening — it is the same rule applied to the correct vehicle now that rung 2 is split (ratified 2026-08-15): `2a` is one-host worker hardening and **cannot surface a cross-host deny at all**, so it is unblocked immediately; `2b` is where host B first makes this error operator-visible. **Fallback if the typed variant is refused at 14-4:** preserve the reason in the existing string field and record the typed outcome as still-open alongside **D7** — but that is a worse answer and the measurement says it is not necessary. |
| **D19** | **Seven blocking CI gates cannot see a bridge-lane story file, so story-file discipline is unenforced for the entire J1 series.** Five walk `_bmad-output/implementation-artifacts/` behind a digit-prefix filter and skip any name that does not start with a number: `check_bare_review_findings.rs:35`, `check_dev_model_tier.rs:103`, `check_dev_model_used_populated.rs:136`, `check_dev_record_completeness.rs:245-247`, `check_review_findings_resolved.rs:57-60`. Two more skip by a different mechanism: `check_epic_close_coherence.rs:215-217` (`head.parse().ok()?`, its comment naming `j1-crosshost-1` explicitly) and `check_epic_6_bridge.rs:820-828` (hardcoded `"6-2"`/`"6-3"` prefixes). **CORRECTED 2026-08-16 (`j1-crosshost-2b`/`2c` preflight): the shared-filter defect is SEVEN walkers, not five.** `check_epic_6_bridge.rs` is blind by TWO mechanisms — besides the hardcoded prefix, it carries two more digit-prefix directory walkers of its own at **`:2563`** (`check_7_1_5_bare_rf_count`) and **`:2608`** (`check_7_1_5_dmu_missing_count`), both `name.ends_with(".md") && name.starts_with(|c| c.is_ascii_digit())`. A fix scoped to the five originally listed would leave two walkers behind and the single-source claim would be false at birth. All five original directory-walkers are BLOCKING jobs (`discipline.yml:1720, 1734, 1748, 1762, 1778`). **Net effect: a `j1-*` story can ship with no dev record, no `dev_model_used`, no §A6 marker and no review-findings closure, and zero gates notice — a green CI does not mean the review net ran.** Filed by the `j1-crosshost-2a` preflight round-table (2026-08-16). The hole has been open across `1a` (done), `1b`, `j1-demo-one-command-scene` (done) and now `2a`; each disclosed it in prose and none of them closed it, which is why disclosure is no longer an acceptable disposition. | **Decide the filename contract, not a patch to one gate.** Either (a) replace the digit-prefix filter with the sprint-status key set — every gate then governs exactly the stories the tracker knows about, including epic-orphaned lanes; or (b) ratify that bridge-lane story files are outside story-file discipline and state the boundary in `RELEASE-HOLDS.md` §Claim boundaries, per binding rule 4. **Do not fix one gate**: five walkers sharing one filter is the single-source defect this project has already paid for twice (gate-binding decay, Epic-13 tracking). If (a), the walkers should share one helper so the next filter change is one edit. `j1-crosshost-2a` continues to DISCLOSE in its Dev Agent Record; disclosure is the interim state, not the resolution. | **14-0** decomposes into a named story (no existing Epic-14 vehicle's ACs cover story-file gate scope) | Before the next `j1-*` story leaves `ready-for-dev` | Mary + John |

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
