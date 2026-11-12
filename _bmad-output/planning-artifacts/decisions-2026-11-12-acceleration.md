# Operator decisions — 2026-11-12 development acceleration

Recorded 2026-11-12. Source: operator ask answers plus free text. Supersedes only what is named below.
Earlier same-day pointer: scope ruling commit `9e4be177` (`ga` = boundary, `rc` = b, `kb` = keep).
`kb` = keep is **superseded by `lim` = stop22 for the STOP value** (see ruling 7); `ga` and `rc` stand.

## Operator question

> would raising limits help development quick and fast? how can we accelerate the development? how can we make those steps impact the development less? what can we do instead of increasing the limit?

## Where time goes (session observations, not a measured benchmark)

| Step | Observation |
|---|---|
| Dev landing | about 35 min (Dev208K) |
| Dev full verification chain | about 15-20 min (Dev201bK observation) |
| Review round | 4-11 min each; 3-5 rounds per story |
| Hosted run | about 59 min wall (`wait.sh` 3561 s on run 43104682092) |
| Cargo | one serial slot |
| Operator ask round trips | each blocks the lane until answered |
| Long-lived agents | reached 840K / 1M context |

## Rulings (operator answers; ask key in backticks)

1. **`nextest`** — Dev agents run FOCUSED tests plus gates locally. The full workspace nextest runs once per batch; hosted CI is authoritative.
   Main checked `docs/testing/proof-contract.md:40-41`: the workspace suite runs in CI, and the required prepush is the only local-workspace exception, so this is consistent.
   The contract's focused-suite rules (`proof-contract.md` lines 33-54) remain binding: focused suites of touched crates; kernel `cargo test -p maos-kernel-core --lib`; closing a story re-runs `d11_xtask_ceiling_ratchet` + `recovery_lane_ceiling_rule`; a manifest/admission change widens the focused set.
2. **`prepaid`** — STANDING PRE-APPROVAL.
   (a) A surface-hash refresh (ADR-066) whose symbol is named in a measured FLAG-Winston grant is pre-approved; the dev records it, and the list of refreshes is reviewed at epic close.
   (b) A push of a fully gated green phase to main is pre-approved, with a post-hoc report; the gates named in the commit routine must be 0.
   The per-event ask remains required for anything outside a green gated phase, force pushes, release tags and history rewrites.
3. **`sev`** — Review severity policy: P0/P1/P2 correctness and security findings are fixed in the correction round. Low and Info findings become rule-10 deferred rows with a disposition and a trigger and get no further correction round, unless the reviewer marks them blocking.
4. **`repin`** — Build a pin-repin script in `scripts/` (NOT xtask, to spare the xtask line ceiling) that updates `kernel-core-baseline`, the kloc rows and `_aggregate_hardfail`, the two test constants (`kernel_pin_content_hash_16_0.rs`, `recovery_lane_ceiling_rule.rs`) and the d11 HISTORY cite in one command. Status: dev task, NOT yet done.
5. **`s21`** — Pull story 21-5 (orchestrator port and extraction, about -1,000 tokei, optional negative FLAG-Winston, currently Epic 21 backlog) forward to follow 20-9 as a kernel story. Status: story created 2026-11-12 (`21-5-orchestrator-port-and-move.md`; all fourteen D-rows RULED by ask `d21`, 2026-11-12, accepted as recommended; story `ready-for-dev`). Epic 21 membership/key: to be decided by story creation.
6. **Agent hygiene** (operator free text): "do not revive long-lived agents, make a new one (preferred if context is not so critical), or compact or summarise it". Dev/review agents are spawned fresh with a self-contained brief; a long-context agent is replaced or compacted, not revived.
7. **`lim` = stop22** — The Epic 20 shared kernel STOP is raised from 21,250 to **22,000 tokei** for the whole cumulative measured ledger at each landing. ADR-069's alarm (22,000) and envelope (23,000) are unchanged.
8. **`r1` = rescope** (ask 2026-11-12, after the Epic 20 closure audit) — Story 20-7 closes at L1 plus the rc waiver. Its L2 (T12 shadow, T13 bind, AC2, AC11) and every stable-runner-only row move to the named UNCLAIMED row `ops-stable-perf-runner` as the claim boundary "perf verdicts ABSENT; no stable-runner timing claim". The 20-7-owned flake rows are dispositioned as hosted-runner timing noise with their same-line citations. Not a widening of any time budget.
9. **`r2` = yes** — Agents may dispatch the T3 `fuzz-prerelease` chain (24 h+ hosted job) at the rc candidate SHA with the `gh` token. Hosted minutes only.
10. **`r3` = ask** — The signed rc tag `v1.0.0-rc.1` is NOT pushed by an agent without a per-event ask (a tag push is a release act; signing runs in CI via `release.yml`).
11. **`q20` = supersede** (ask 2026-11-12) — The `q` clause "20-1b stays not `done`" is superseded for the rc: 20-1b closes after its hosted legs, the quarantine gaps move to a NEW post-rc backlog key `later-registry-quarantine-closure` (not a `20-*` key). Claim text: FR13 = capability-mediated fence only, not isolation; private/shared memory writes, scalar mutation, native IAC/IO, hooks and telemetry are NOT fenced; per-pid in memory; no release verb; I5 FR13 stays `asserted`.
12. **`p4` = accept** (ask 2026-11-12) — Exception for 20-4: the green hosted discipline run 43150757804 (199/199 on d00802aa, descendant of 251dc692) substitutes for local `prepush` and is the final-tree proof.

## ADR-069 consequence (STOP now equals the alarm)

`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:42-45`:

> 3. **Alarm at 22,000.** The first grant that takes the row above 22,000 must say so in the row's own HISTORY text (`ENVELOPE ALARM`), and the story's FLAG-Winston decision record must state the remaining envelope and whether `21-5-orchestrator-port-and-move` (negative, ≈ −1,028 lines) is promoted from optional to required.

Also line 56: "`21-5` stays optional until the alarm fires; it is the pre-named relief valve." Lines 35-36: envelope ≤ 23,000; line 46: beyond 23,000 is a new operator decision.
With STOP = alarm = 22,000, any landing that would cross 22,000 triggers the alarm rule, so 21-5 becomes mandatory per the ADR (the promotion must be stated in that grant's decision record). (Quoted text is from the file as read; the "mandatory" reading is the ruling's application of ADR line 42-45.)

## Kernel room at ruling time

Live kernel at ruling time: 27,040 physical / 21,201 tokei (HEAD `edeedd25` + docs commit `9e4be177`; Dev2014d's 20-14 corrections in flight).
Room under the new STOP: 22,000 − 21,201 = **799 tokei** before the other owed rows.

Owed after 20-14:
- 20-13 KT3: +1/−1 literal; an operator-granted `SecurityManagerAdapter` surface refresh (ask `kt3` = refresh, 2026-11-12).
- 20-9 row R: +21.

Previously deferred kernel items now unblocked for pricing — each needs its own measured grant; **none is granted by this ruling**:
- 20-1b quarantine closure (+30..+80, unmeasured);
- 20-9 coordinator expect window / `archive.rs` atomic record / idle-watchdog sequential fire;
- 20-8 precheck `PrecheckOutcome` variant;
- 20-11 private/index hold lease.

## Open work created by these rulings

- `repin` script (dev task) — not done.
- 21-5 story creation (`s21`) — story created 2026-11-12 (`21-5-orchestrator-port-and-move.md`; D-rows ruled by `d21`, 2026-11-12); implementation not done.
