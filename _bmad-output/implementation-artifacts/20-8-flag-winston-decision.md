# 20-8 FLAG-Winston decision record

**Date:** 2026-11-03  
**Status:** `PROPOSED`. The operator rules D-20-8-07 (and D-20-8-01 for landing order). No grant, implementation or landing is claimed.  
**Scope:** the kernel seam of `20-8-manifest-contract-completion`: how the `[swap_invariants]` declaration reaches the swap that must honour it.  
**Story:** `_bmad-output/implementation-artifacts/20-8-manifest-contract-completion.md` (frontmatter, §Kernel and kloc pricing, §Decisions, §Contract for 20-9).  
**Evidence:** `20-8-manifest-contract-completion-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `prototypes/{kb,kb4,ks}-kernel-only.diff` (full-tree variants alongside).  
**Precedent:** `20-4-flag-winston-decision.md`, `20-7-flag-winston-decision.md`.

## Why the kernel is touched at all

A hot swap parses its successor in the kernel (`load_bundle_from_file`, `crates/maos-kernel-core/src/lifecycle/upgrade.rs:307-429`, caller
`hot_swap/coordinator.rs:464`). The predecessor's sections live in the SCB bundle (`scheduler/control_block.rs:195-238`). A declaration
that neither carries reaches no swap. The other three FR8 sections have consumers outside the kernel: admission (maos-bin), IAC delivery
(maos-iac), and none for `explanation_shape` (D-20-8-05). The kernel-free option K0 keeps the declaration in maos-bin. It leaves the
coordinator blind and moves the carriage, and its FLAG-Winston review, to 20-9.

## Options (measured at `8c809f9c` from 26841 physical / 21042 tokei, reverted prototypes)

| Option | Shape | Kernel physical | Kernel tokei | Other |
|---|---|---:|---:|---|
| **A — KS (recommended)** | KB, plus the coordinator refuses a successor that does not preserve every predecessor-declared invariant. The check runs after the halt-continuity gate and before `on_swap_out`, with saga `RestorePredecessor`, typed `HotSwapError::SwapInvariantDropped` (maos-domain) | **+38** (26879) | **+33** (21075) | domain +2, bin +1 |
| B — KB | `SpiritManifestBundle.swap_invariants` + `Default` + the kernel's successor parser; no check (20-9 adds it) | +10 (26851) | +8 (21050) | bin +1 |
| C — K0 | kernel 0; 20-9 prices KB/KS under its own grant | 0 | 0 | — |
| D — KB4 | all four declarations carried on the SCB | +34 (26875) | +32 (21074) | bin +4 |

Ceilings requested under A (×2 measured, STOP above): **kernel ≤ 26917 physical / 21108 tokei**, standalone from 26841 / 21042. L2,
kernel last.

## ADR-069 room and the cumulative STOP

Expected cumulative measured tokei with the granted, unlanded rows: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) = **21125**.
With A: **21158** (51 below the 21209 cumulative STOP, 842 below the 22000 alarm). At A's ×2 ceiling: **21191** (18 below the STOP).
B: 21133; D: 21157. No option crosses 21209, so no split/exchange/bump row is raised. The later kernel lander among 20-1b, 20-4, 20-11
and 20-8 re-bases against the then-landed pins before pinning. **Stated risk:** after A, 20-9 (watchdog source hook, D07 suppression,
lifecycle ABI, all forecast kernel) has at most 51 lines under the STOP and will need its own bump decision. Same-day PROPOSED rows also
claim the room: 20-14 KT +25 and 20-13 KT3 −1. With all measured, the cumulative is **21182** (27 left). At every ×2 ceiling it exceeds 21209,
and the measured cumulative governs (later lander returns to the operator). B (KB) would leave 25 more lines than A.

## Landing and accounting (if granted)

L1 lands AC1–AC6 with no kernel line. L2 lands AC7 under the ruled option, re-measures the formatted code, and pins
`xtask/kernel-core-baseline.toml` (src_lines, content hashes of the changed files, set hash) and `xtask/kloc.toml:264` exactly, with
HISTORY rows citing this key with `->`. Unused room is not reserved. A measured figure above the ceiling is STOP and a new D-row.
