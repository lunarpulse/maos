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
