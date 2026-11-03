# 20-9 FLAG-Winston decision record

**Date:** 2026-11-03  
**Status:** `PROPOSED`. The operator rules D-20-9-13, D-20-9-14 and D-20-9-15, plus D-20-9-11 (whose option B adds KW). This record grants nothing and claims no implementation or landing.  
**Scope:** the kernel row of `20-9-spawned-form-lifecycle-hooks`: the schedule watchdog's D07 A suppression (ruled 2026-11-01, ask `rows203c`) and the PF-04 refused-poll observer. The spawned-form hooks, WIT 2.4.0, the hot swap, upgrade and cold swap, and the schedules carriage are kernel-free.  
**Story:** `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks.md` (frontmatter, §Kernel and kloc pricing, §Decisions).  
**Evidence:** `20-9-spawned-form-lifecycle-hooks-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `prototypes/*.diff`.  
**Precedent:** `20-8-flag-winston-decision.md`, `20-11-…-evidence/creation/ks-schedule-watchdog-forget-spirit.diff` (same file), `20-4-flag-winston-decision.md`.

## Why the kernel is touched at all

Both received items live inside the kernel's private `ScheduleWatchdog::check_and_fire`
(`crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs:158-268`):
- **D07 A.** The fire proceeds after a refused side-effect mint (`:216-221`, `:331-342`), and `first_id` keeps a partial mint (`:313-345`). Suppression is a change to the firing decision.
- **PF-04.** A revoked or rate-refused poll emits nothing (`:199-201`, `:212-214`), so no test can observe one without wall-clock (`tests/schedule_watchdog_fr26.rs:537-578`).

Spawned lifecycle hooks need no kernel line: the kernel already calls every hook on the spawned SCB's `WorkerSpirit`, which
no-ops them (`crates/maos-bin/src/supervision.rs:2612-2633`). The hot swap keeps the pid, the tokens and the working
memory (`hot_swap/coordinator.rs:120-455`).

## Options (measured at `8c809f9c` from 26841 physical / 21042 tokei, reverted prototypes)

| Option | Shape | Kernel physical | Kernel tokei | Other | Surface gate |
|---|---|---:|---:|---|---|
| **R = KD07A-lean + KO-poll-x (recommended: D-13 A, D-14 A)** | Suppress, revoke the fire's minted scopes, cadence consumed; `poll_once() -> Vec<PollOutcome>`; record, journal fn and enum in `maos_iac::payload` | **+16** (26857) | **+21** (21063) | iac +39 | pass |
| KD07B-lean (D-14 B) | suppress without revoke (TTL expiry) | +2 | +8 | iac +32 | pass |
| KD07A-inline (D-14 C) | KD07A journaled in the kernel | +27 | +32 | iac +7 | pass |
| KO-poll (D-13 B) | R with `PollOutcome` in the kernel | +24 | +27 | iac +32 | classes row needed |
| KO-count (D-13 C) | refused-poll counters on the struct | +28 | +30 | iac +32 | **baseline re-issue** (struct signature) |
| KO-row (D-13 D) | a TL row per refused poll | +20 | +26 | iac +48 | pass; row flood |
| KW (D-20-9-11 B) | `post_swap_window_open(pid)` | +9 | +6 | — | pass |

Ceilings requested under R (×2 measured, D02 A, STOP above): **kernel ≤ 26873 physical / 21084 tokei**, standalone from 26841 / 21042.
With KW: ≤ 26891 / 21096. L2, kernel last.

## ADR-069 room and the cumulative STOP

Expected cumulative measured tokei: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8 KS, PROPOSED) + 25 (20-14 KT,
PROPOSED) − 1 (20-13 KT3, PROPOSED) = **21182**. With R: **21203** (6 below the 21209 STOP, 797 below the 22000 alarm).
With R + KW: 21209 (0 below). **At R's ×2 ceiling: 21224, which crosses the STOP.** Every 20-8 ruling (KS, KB, K0) leaves 20-9
at about 21203 (measurements §Contingencies).

The measured cumulative fits by 6 lines, but the ceiling the D02 A envelope asks for does not. The creation contract
therefore raises **D-20-9-15** (split / exchange / bump). The exchange is already applied in R (−11 tokei against the inline
shapes). Recommended: **A**, one joint targeted bump of the internal cumulative STOP from 21209 to **21250**, ruled together with
20-14's D-20-14-12 (both ×2 allowances: 21125 + 33 + 50 + 42 − 1 = 21249; 750 below the alarm). If D-20-14-12 is ruled
differently, 21230 for 20-9 L2 alone (covers R's ×2 ceiling 21224; 770 below the alarm). The alternatives: B bump to the ADR-069 alarm line (22000);
C split (PF-04 to a new key, D07 A here); D no bump, so the measured figure governs and L2 returns to the operator if the re-based
figure exceeds 21209.

## Landing and accounting (if granted)

L1 lands every kernel-free AC (hooks, WIT 2.4.0, swap, upgrade, cold swap, schedules carriage) with no kernel line. L2 lands
AC7 and AC8 under the ruled rows last. It re-cuts on 20-11's `forget_spirit` hunk, which is in the same file and distinct. It
re-measures the formatted code and pins `xtask/kernel-core-baseline.toml` (`src_lines` `:505`, the
`scheduler/schedule_watchdog.rs` hash `:1005`, and `hot_swap/coordinator.rs` `:958` under KW, set hash `:939`, EOF HISTORY)
and `xtask/kloc.toml:264` exactly. Each HISTORY row cites this key with `->` (D11 A). Unused room is not reserved. A measured
figure above the ceiling, or a re-based cumulative above the ruled STOP, is STOP plus a new D-row. Any
`crates/maos-kernel-core/src` change outside `schedule_watchdog.rs` (and `coordinator.rs` under KW) is STOP.
