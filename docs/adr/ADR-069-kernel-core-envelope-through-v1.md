---
Status: ACCEPTED — ratified 2026-10-23 by the operator (Lunarpulse), routing-audit DECIDE-4 option (a) "increase", figure "≤ 23,000 (alarm 22,000)"
Gate: `xtask/tests/recovery_lane_ceiling_rule.rs` (`kernel_core_ceiling_stays_inside_the_adr_069_envelope`, `crossing_the_adr_069_alarm_is_acknowledged_in_the_row`) — kloc-free, tree-bound
Decided: 2026-10-23
Accepted-in-PR: pending — operator ratification 2026-10-23 (routing audit DECIDE-4)
Revisits: NFR-Maint-1 (`prd/non-functional-requirements.md:132`) through v1.0 GA; ADR-038's ≤20 KLOC letter for `maos-kernel-core`
Supersedes: no ADR in full — ADR-038's per-crate letter is already history (`maos-kernel-core ≤6 KLOC` was retired by architecture §15.5 Fork-5); the proposed constitutional ceiling (architecture §15.5 clause 4, `kernel-crate-set.toml` ≤25 KLOC) was never landed as a file and is not revived here
---

# ADR-069 — The kernel-core envelope through v1.0: ≤ 23,000 production lines, alarm at 22,000

## Context

NFR-Maint-1 promises a kernel trusted core of **≤ 20 KLOC excluding tests through v2.0**. Its working
instrument is the `maos-kernel-core` row of `xtask/kloc.toml` (tokei, production Rust only — the units
the NFR's "excluding tests" letter names; architecture §15.5). On 2026-10-23 that row reads **20,594**,
already 594 over the letter, at **zero headroom**: every line since Epic 15 has cost its own measured
FLAG-Winston grant and a HISTORY row (RECOVERY-LANE CEILING RULE, `kloc.toml:50-93`, which eased every
other crate's budget and explicitly did **not** ease this one).

The routing audit (`_bmad-output/planning-artifacts/routing-audit-2026-10-22.md` §4 DECIDE-4) found
seven unpriced ◆ FLAG-Winston stories still ahead — `17-11`, `18-5`, `18-6`, `19-5`, `20-8`, `20-9`,
`21-6` — plus smaller kernel touches (`18-3` +15–30, `19-3` +3–5, `20-3c`, `19-6`, `20-11`). Without a
ceiling decision every one of them re-litigates the letter on its own; the PRD DELTA-2026-10-22 made
that decision owed **once, before Epic 18 opens**.

Measured history of recovery-lane kernel grants (`kloc.toml` `maos-kernel-core` HISTORY): `17-6` +80,
`17-3c` +103 / +117, `17-1` review +40, `17-2` −58 / −3, `17-7` +558 then review +495, `17-3d` −18,
`17-4` +11, `17-9` +56; Epic 16 took +449 against a forecast of +5–25 (`epic-16-retro-2026-09-20.md:116`).
Median ◆ grant ≈ +80, mean ≈ +273. Forecast for the remaining lane: 7 × 273 + ≈ 200 small ≈ **+2,100**
→ ≈ 22,700.

## Decision

1. **Envelope.** Through `v1.0.0` GA, the `maos-kernel-core` production-line ceiling in
   `xtask/kloc.toml` may not exceed **23,000**. This replaces NFR-Maint-1's 20,000 letter for that period.
2. **Nothing else is eased.** The row keeps **zero headroom**; every kernel line still needs its own
   measured FLAG-Winston grant, cited to an open story, landed with its driver in the same commit.
   `xtask/kernel-core-baseline.toml`'s pin (anti-drift, raw `src/` lines + file set + content hash) is
   untouched and is never compared with this figure. The envelope is a maximum, not a reservation: no
   story may cite it as headroom.
3. **Alarm at 22,000.** The first grant that takes the row above 22,000 must say so in the row's own
   HISTORY text (`ENVELOPE ALARM`), and the story's FLAG-Winston decision record must state the remaining
   envelope and whether `21-5-orchestrator-port-and-move` (negative, ≈ −1,028 lines) is promoted from
   optional to required.
4. **Beyond 23,000** is a new operator decision, not a grant: the gate refuses it.
5. **Review point.** Re-measured at each epic retrospective; re-decided at `v1.0.0` GA, where NFR-Maint-1's
   v2.0 letter and the never-landed kernel-crate-set instrument are reconsidered together.

## Consequences

- Seven ◆ stories proceed under one ratified bound; the per-story ceremony (measure, grant, history) is
  unchanged, the per-story re-argument of the PRD letter ends.
- The "kernel stays small" claim (ADR-047 §1, ADR-038) is restated honestly: ≤ 23 KLOC production, with
  every line attributed — not ≤ 20 KLOC.
- `21-5` stays optional until the alarm fires; it is the pre-named relief valve.

## Enforcement

`xtask/tests/recovery_lane_ceiling_rule.rs` asserts against the tree that the `maos-kernel-core` row is
≤ 23,000, and that a row above 22,000 carries `ENVELOPE ALARM` in its comment. Both are kloc-free and run
in the existing xtask suite.
