# 20-14 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03: D-20-14-02 B, D-20-14-03 B and D-20-14-09 A (ask `d_batch4`); D-20-14-12 C (ask `d2014_12`). KT remains a measured request, binding at dev start; no implementation or landing claimed.  
**Scope:** the kernel row of `20-14-complete-cost-attribution`: the cost-attribution emission site reads a root-bound task / session-principal scope; no implementation or landing claimed.  
**Story:** `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution.md`, frontmatter, §Decisions, §Kernel and kloc pricing.  
**Evidence:** `20-14-complete-cost-attribution-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `kt-token-bound-scope.diff`, `km-scb-marker.diff`, `km-naive-scb-marker.diff`, `kp-root-cost-scope-port.diff`, `k0-token-on-cost-row.diff`, `kr-request-correlation-id-on-km-naive.diff`, `rec-kt-domain-cli-audit-bin-shell-slice.diff`.  

**L1 amendment (operator ask d1408, D-20-14-13 A):** the existing kernel
literal and `CostAttributionPayload` stay unchanged in L1. `CostFrameBody`
flattens that payload and carries `task_ref`; L2 wraps the existing literal
and must re-measure its kernel hunk against the KT grant (the original
prototype did not price that wrapper). Orchestrator-adjudicated overdrafts,
disclosed to the operator: domain ceiling 10200 -> 10215 (d1408 wrapper),
CLI 6524 -> 6545 (formatted report and new witness); audit 8237 and aggregate
199369 remain unchanged. 20-9 and 20-5 re-base domain on the exact landed pin.
**L1 review-corrections overdraft (security F1-F8, triage P1-P4):** an
orchestrator-adjudicated overdraft, disclosed to the operator (D02 A): CLI
ceiling 6545 -> 6610. Before: pin 6529, ceiling 6545. After: measured 6599
(formatted), ceiling 6610 (+11 slack). Audit measured 8227 inside 8237; iac
stays at its 8728 pin; aggregate 199146 inside the rebased 199378. No kernel
line; kernel pins 26841/21042 unchanged. **Round 2 (CR1–CR3):** CLI re-measured 6603
(inside the same orchestrator-adjudicated overdraft ceiling 6610, disclosed to the
operator), iac 8726 (down 2; ceiling 8728 unchanged), aggregate 199148, hardfail 199149.
**Precedent:** `20-4-flag-winston-decision.md`, `20-7-flag-winston-decision.md`.

## Decision requested

| ID | Options priced (reverted prototypes on `git archive 8c809f9c`) | Recommended | Status |
|---|---|---|---|
| D-20-14-02 | **A** KM SCB marker +44/+30 (naive formatting +59/+47) · **B** KT token-bound scope in the capability registry +35/+25 · **C** KP root `CostScopePort` +31/+25 (domain +7) · **D** K0 token bytes on the cost row, report-side join, 0/0 lines (content hash moves) | **B** | RULED 2026-11-03: **B** (ask d_batch4) |
| D-20-14-12 | **A** split (no allowance; ceiling = measured) · **B** exchange to K0 · **C** joint targeted cumulative STOP 21250 with D-20-9-15 | **C** | RULED 2026-11-03: **C** (ask d2014_12) |
| D-20-14-03 | **A** `InferenceRequest.correlation_id` as Spirit-declared `request_ref` (+1/+1 on top, domain +9) · **B** none · **C** request id as the task source | **B** | RULED 2026-11-03: **B** (ask d_batch4) |

Why B for D-02 (evidence in the story's D-row): no Inference Port caller is ever dispatched a `task.assign` at HEAD, so
the SCB marker would have no production writer at the point a task starts; every production port caller already
mints its own token per turn or pass (`shell_host.rs:112-122`, the shared seam mint `main.rs:4179-4200`); the token
is verified before emission (`inference/mod.rs:250-272`); a token-keyed scope cannot go stale after a failed turn
and cannot be shared by two concurrent tasks of one Spirit (the SCB ledger is a `Vec` for exactly that reason,
`control_block.rs:278`); and both adapter construction sites (`main.rs:4073,4100`) need no new wiring.

## Kernel grant and ADR-069 room (if ruled as recommended)

Creation base `8c809f9c`: **26841 physical / 21042 tokei**. KT measured **+35 / +25 → 26876 / 21067**
(`capability/mod.rs` +19/+12, `inference/mod.rs` +16/+13; 95 files; no file added or removed). Ceilings asked
(×2 measured, D02 A): **26911 physical / 21092 tokei**; STOP above.

Expected cumulative measured tokei: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = **21203**. Shared internal STOP **21209** remains for everything else. D-20-9-15 A sets **21230 for 20-9 L2**, and D-20-14-12 C (ask `d2014_12`) sets **21250 for the combined 20-9 L2 + 20-14 L2 cumulative**. ADR-069 alarm **22000** unchanged. Later kernel landers re-base against exact landed pins.

## Landing and accounting (if ruled)

L1 implements every kernel-free part (payload field, report axes, Spirit naming, docs) without a kernel line; L2
implements KT plus the root binding sites last, re-measures formatted code, and pins `xtask/kernel-core-baseline.toml`
(`src_lines`, the `capability/mod.rs` and `inference/mod.rs` content hashes, an EOF HISTORY row) and `xtask/kloc.toml`
`maos-kernel-core` exactly, each HISTORY row's first segment citing this key with `->` (D11 A). Unused room is not
reserved. Any `crates/maos-kernel-core/src` change outside KT's two files is STOP.
