---
Status: ACCEPTED — Winston (System Architect) ratified 2026-06-12 (ACCEPTED-WITH-CONDITIONS; the 2 truthfulness corrections are applied below). Story 8.16 — Epic 8→9 readiness bridge.
Gate: removal verified by `xtask check-epic-close-green` (zero `if: false` jobs in discipline.yml) + the live successor gates enumerated below
Decided: 2026-06-12
Accepted-in-PR: <PR_NUMBER>
Revisits: Epic 6 retro §A2/§A3/§A5/§A6; Epic 7 §A1 (7.1.5/7.1.6 gate migration); Story 7.1 spirit-authoring template
Supersedes: the `check-epic-6-bridge` debt-beacon job and the `smoke-spirit-author-7-1` advisory smoke (both DISABLED `if: false` 2026-06-12, round 6 of the Epic-8 CI remediation)
---

# ADR-043 — Retire two parked discipline gates; enforcement is carried by live gates

**Context.** At Epic-8 close the only way the integrated CI run (`27388044071`) reached green was to DISABLE two advisory gates with `if: false` (round 6, commit `0707f21`): `smoke-spirit-author-7-1` and `check-epic-6-bridge`. The Epic-8 retrospective (`epic-8-retro-2026-06-12.md`, actions §A1) ruled that `if: false` is not an acceptable terminal state — each gate must be either **repaired and re-enabled** or **formally retired with a ratified ADR proving its enforcement is carried elsewhere or is obsolete**. This ADR ratifies the **retire** path for both, executed by Story 8.16. Neither retirement removes live coverage; each is justified item-by-item below.

## Decision 1 — Retire `check-epic-6-bridge`

**Retire the job** (delete from `discipline.yml`, remove from the aggregate `needs:` list and `report-aggregate`, leave the `xtask/src/check_epic_6_bridge.rs` module in tree as archived history). The Epic-6 debt-beacon's responsibilities are all carried by live, permanent hard-fail gates at HEAD:

| Beacon concern | Live successor gate (hard-fail at HEAD) | Status |
|---|---|---|
| §A2 — review-findings resolved | `check-review-findings-resolved` (no `continue-on-error`, discipline.yml:1660 post-8.16) | green at HEAD |
| §A2 — dev-record completeness | `check-dev-record-completeness` (discipline.yml:1674 post-8.16) | green at HEAD |
| §A2 — dev_model_used populated | `check-dev-model-used-populated` | green at HEAD |
| §A2 — no bare review-findings | `check-bare-review-findings` | green at HEAD |
| §A3 — serde error handling | `check-serde-error-handling` (the beacon's own A3 check confirmed it "exists and wired") | live |
| §A5/§A6 — gate-existence | migrated to the above hard-fail gates in Story 7.1.5 (per the beacon's own header comment, `check_epic_6_bridge.rs:36`: "§A2/A5/A6 rows REMOVED in Story 7.1.5 — now enforced as hard-fail gates") | superseded |
| §A4-Debt-2b/2c — exemptions/hook-count | already PASS in the beacon | n/a |
| §A4-Debt-1 — i9 holder-path discipline | `check-empty-kernel` + `check-service-boundary` (live I9 enforcement) | live |

The beacon's last remaining red (`A4-Debt-1`) is a **stale check, not real debt**: `check_a4_debt_1()` (`check_epic_6_bridge.rs:716-723`) counts lines in `xtask/i9-whitelist.toml` that contain the substring `rationale` and requires `>= 5` (the OLD array-of-tables-with-per-entry-rationale schema). The current file uses a `paths = [...]` array (3 sanctioned holder paths, no `rationale` lines) and `docs/invariants/i9-exemptions.md` exists — both present; only the beacon's entry-counting predicate is outdated against the file's current shape. The underlying I9 invariant is enforced live by `check-empty-kernel` and `check-service-boundary` (both in the aggregate `needs:`). Additionally, the beacon's `--story 6.2/6.3/6.4` sub-invocations have been CLI-broken (exit 2, arg-parse) since the job was disabled. Retiring removes a 4504-line legacy module that duplicates live enforcement and carries a stale predicate; **no Epic-6 §-item goes dark.**

## Decision 2 — Retire `smoke-spirit-author-7-1`

**Retire the job** (delete from `discipline.yml`, aggregate `needs:`, and `report-aggregate`). It was an advisory smoke for the v0.5 spirit-authoring template; it broke on three Epic-7 template-bit-rot defects (cargo-generate ≥0.23 reserving the `crate_name` placeholder; a referenced-but-absent `post-generate.rhai` hook; the unpublished `@maos/spirit-ts` SDK package). Repair requires a cargo-generate 0.23-compat template pass + SDK publication/vendoring — a dedicated **template-repair story**, out of scope for a discipline bridge. Spirit-authoring is NOT left uncovered: `example-spirit-tests`, `example-spirit-drift`, `example-spirit-ts-tests`, and `spirit-test-tests` (all live, in the aggregate `needs:`) exercise the generated example spirit and SDK surface. When the template-repair story lands, it MAY re-introduce a fixed authoring smoke; until then the advisory smoke is retired rather than parked `if: false`.

## Rationale

1. **`if: false` is the worst form of the decay pattern** (`[[feedback_mechanical_gates_compound_promises_decay]]`): a red gate disabled to fake green reads as "covered" while enforcing nothing. The Epic-8 retro named this; the four-epic `check-epic-6-bridge` beacon is its canonical instance. Retiring (with proof of live successors) or repairing are the only honest terminal states.
2. **Retire ≠ hide debt.** Each retired concern maps to a live hard-fail gate or is a proven-stale predicate. The `check-epic-close-green` gate (Story 8.16) makes re-introducing `if: false` mechanically impossible going forward.
3. **Modules stay in tree** (archived), so the history and any future re-enable are a documented revert, not a reconstruction.

## Consequences

- `discipline.yml` has zero `if: false` jobs; the aggregate `needs:`/`report-aggregate` reference only live jobs (no dangling `needs.<job>.result`).
- Epic-6 discipline is enforced by the five live §A2/§A3 gates + the two live I9 gates.
- A future template-repair story owns any re-introduction of an authoring smoke.
- Winston (System Architect) ratified this retirement 2026-06-12 (ACCEPTED-WITH-CONDITIONS — the 2 truthfulness corrections applied above). He independently verified at HEAD that all four §A2 gates + check-serde-error-handling are hard-fail and exit 0, that I9 holder-path discipline is enforced live by check-service-boundary (which actually reads `i9-whitelist.toml`) + check-empty-kernel, that the authoring surface stays covered by four live jobs, and that no `needs.*.result` reference dangles. No enforcement goes dark.
