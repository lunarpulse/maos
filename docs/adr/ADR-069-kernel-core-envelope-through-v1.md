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

## Relief valve exercised — 2026-11-12 (Story 21-5 L2, ADR-070)

Operator ruling `s21` scheduled `21-5-orchestrator-port-and-move` ahead of the alarm (it did not fire: the row stood at
21,256, 744 below 22,000). L2 moved the FR20 buffer, its journal helpers and the parked FR54 gateway host out of
`maos-kernel-core` into `maos-orchestrator-buffer`: the `maos-kernel-core` row is **21,256 → 20,429 (−827 tokei)**,
raw `src/` lines 27,103 → 26,062 (−1,041), 95 → 90 files; **1,571 below the 22,000 alarm and 2,571 below the
23,000 envelope**. The freed lines are not growth capacity: the row was LOWERED to the measured figure
(`RATIFIED_AT_EASING` follows it) and keeps zero headroom; every later kernel line still needs its own grant
(Decision 2). The row's HISTORY says `ADR-069 RELIEF VALVE (21-5, ruling s21)`.

## Amendment — 2026-11-12 (operator ruling `kfree`)

Operator words: "we can relax the kernel limitation to accelerate the development, less permissions to increase every
time." Record: `_bmad-output/planning-artifacts/decisions-2026-11-12-dedrift.md`.

- **Decision 2 is amended.** A kernel change inside the envelope no longer needs a per-landing operator ask or a
  pre-priced FLAG-Winston grant. The developer measures at landing, re-pins with `scripts/repin.py` and writes the
  HISTORY row (driver, files, before and after) in the same commit. Kernel landings are listed at each epic
  retrospective. The sentence above that "every later kernel line still needs its own grant" is superseded by this.
- **Surface hashes.** An ADR-066 surface-hash refresh caused by such a landing is pre-approved on the same terms.
- **STOP withdrawn.** The Epic 20 shared STOP (22,000, ruling `lim`) no longer applies. The only numeric limits are
  this ADR's alarm (22,000: `ENVELOPE ALARM` in the row) and envelope (23,000: the gate refuses), both unchanged and
  both still enforced by `xtask/tests/recovery_lane_ceiling_rule.rs`.
- **Unchanged.** The exact pin, the HISTORY row and the review layers. They are what the gates check; they are not a
  permission.

The row stood at 20,429 tokei at the ruling, 2,571 below the envelope.
