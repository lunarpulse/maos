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
| **D11** | No gate reconciles kernel-pin HISTORY; in-`src` `#[cfg(test)]` modules are KLOC-budget-charged but never CI-executed; `EXPECTED_GATES` is hand-maintained (36 entries vs 66 `check_*.rs`). | Make pin history, execution coverage, and workflow-derived gate registration mechanically auditable. Carries **E11-A6** forward to mechanical close. | **14-6** (ceiling instrument + retro-residual discipline) — retro **C5** | Before `14-6` leaves `backlog` | Winston + Murat |
| **D12** | `audit_query_latency` bench has been broken since 9.1 (`"capability.invoke"` vs accepted `"capability.invocation"`); it has run **zero** times. | Repair or retire the bench. | **14-4** (already owns the v2.0 operational-surface sweep) | Before `14-4` leaves `backlog` | Amelia |

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
