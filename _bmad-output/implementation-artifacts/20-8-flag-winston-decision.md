# 20-8 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03 (ask `d_batch4`): D-20-8-07 A, D-20-8-01 A and the envelope as recommended. KS remains a measured request, binding at dev start; no implementation or landing claimed.  
**Scope:** the kernel seam of `20-8-manifest-contract-completion`: how the `[swap_invariants]` declaration reaches the swap that must honour it.  
**Story:** `_bmad-output/implementation-artifacts/20-8-manifest-contract-completion.md` (frontmatter, §Kernel and kloc pricing, §Decisions, §Contract for 20-9).  
**Evidence:** `20-8-manifest-contract-completion-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `prototypes/{kb,kb4,ks}-kernel-only.diff` (full-tree variants alongside).  
**Precedent:** `20-4-flag-winston-decision.md`, `20-7-flag-winston-decision.md`.

**L1 audit ordering — 2026-11-09 (ask `ac13i2` A).** The declaration row
precedes pid issuance and keeps the I2 fail-stop writer; no kernel change.
Orchestrator clarification: pid issuance implies a declaration row, not the
reverse; a later refusal is separately recorded. L2's KS ruling is unchanged.
Operator ask `enc208` A records public digest/key bytes in colon-delimited hex;
this is an L1 representation ruling and does not alter KS or the I2 writer.
Orchestrator-adjudicated overdraft (2026-11-09), disclosed to the operator:
L1 domain ceiling 10,174 → 10,205 after formatted 10,190 from 10,144 (+46
versus creation estimate +11); the enc208 decoder and typed fields are
production code. Aggregate ceiling stays 199,078. 20-9/20-13/20-14 re-base
their domain draws on the eventual landed pin. Kernel KS is unchanged.

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

Expected cumulative measured tokei: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = **21203**. The shared internal STOP remains **21209**; D-20-9-15 A sets **21230** for 20-9 L2 only. ADR-069's alarm remains **22000**. Every later kernel lander re-bases on the exact landed pins.

D-20-14-12 C (ask `d2014_12`, 2026-11-03) sets joint STOP **21250** for the combined 20-9 L2 + 20-14 L2 cumulative; shared STOP **21209** applies otherwise.

## Landing and accounting (if granted)

L1 is kernel-free (AC1–AC7 and AC9; hosted AC7 remains unverified). L2 implements AC8 under the ruled option, re-measures the formatted code, and pins
`xtask/kernel-core-baseline.toml` (src_lines, content hashes of the changed files, set hash) and `xtask/kloc.toml:264` exactly, with
HISTORY rows citing this key with `->`. Unused room is not reserved. A measured figure above the ceiling is STOP and a new D-row.
