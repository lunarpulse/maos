# 20-14 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03: D-20-14-02 B, D-20-14-03 B and D-20-14-09 A (ask `d_batch4`); D-20-14-12 C (ask `d2014_12`). KT was landed at edeedd25 (redesigned 456793ca; round 2 9996fe09 touches no kernel) inside the ruled row; story `done` 2026-11-12 (`closure/closure-note.md`).  
**Scope:** the kernel row of `20-14-complete-cost-attribution`: the cost-attribution emission site reads a root-bound task / session-principal scope; landed at edeedd25 / 456793ca (see the L2 landing record below).  
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
**Round 3 (R2-1/R2-2, GLM F-4):** CLI 6609 (inside 6610); iac ceiling raised 8728 -> 8770 as an
orchestrator-adjudicated D02 A overdraft, disclosed to the operator (before: pin 8726 / ceiling
8728; after: measured 8770, zero headroom); aggregate 199198, hardfail 199199. No kernel line.
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

## L2 landing record — 2026-11-12 (landed edeedd25; hosted run 43140995903 green on 9996fe09)

**Ruled row used:** D-20-14-02 B (KT, token-bound scope), D-20-14-03 B (no request field), grant +35 physical / +25 tokei priced at `8c809f9c` (x2 ceilings; shared STOP 21250).
**T0 re-cut at the live base 27014 / 21181** on a `git archive HEAD` copy, including the `CostFrameBody` wrapper hunk the creation prototype did not price
(`landing-2/evidence.md`, `landing-2/step0/`):

| Shape | Kernel | maos-capability | Surface gate |
|---|---|---|---|
| A: field on `CapabilityRegistryAdapter` | +38 / +28 -> 27052 / 21209 | 0 | RED (struct hash moves; second refresh not granted) -> rejected |
| **B (landed, redesigned in review round 1)**: scope in the token state in maos-capability + two narrow inherent delegates on `CapabilityRegistryAdapter` (the first landing's wide `tokens()` accessor is removed) | **+29 / +22 -> 27043 / 21203** | +21 (1433 -> 1454) | GREEN, no surface-hash refresh |

Changed kernel files: `capability/mod.rs` (+6, accessor), `inference/mod.rs` (+20, emission: scope read by the verified token id, session principal -> `Resolved` / `SessionPrincipal` / `Exact`, proxy only as the
tagged fallback, `CostFrameBody` schema 2 with `task_ref`). `kernel-core-baseline.toml`: `src_lines` 27040, the two content hashes and `set_hash` adopted from `--emit-pin`, HISTORY first segment citing this key with `->`.
Cumulative measured tokei **21203** after review round 1 (21201 at the first landing), 797 under the shared STOP 22000 (ruling `lim=stop22`, 2026-11-12; the earlier "47 under STOP 21250" annotation was stale; the 22000 STOP equals the ADR-069 alarm). **maos-capability: the overdraft (creation grant 0) is ADJUDICATED by Main 2026-11-12, disclosed to the operator, for the redesign that survives review (ceiling 1444 as ruled); the re-measured number is 1454 (+10 over the ruling) and is PENDING Main re-adjudication.** No file added or removed (95); no change outside KT's two kernel files.

## L2 review round 1 — 2026-11-12 (GPT P2 x3, GLM S1-S3; implemented locally, uncommitted)

Main's ruling on the shape B overdraft (2026-11-12): ADJUDICATED, disclosed to the operator, only for the redesign that survives review. Kernel growth from this round was ordered <= +0 net where possible
and a STOP above +4 tokei; measured **+2 tokei / +3 physical** (27040/21201 -> 27043/21203; 797 under the shared STOP 22000, ruling `lim=stop22` 2026-11-12; the earlier "47 under STOP 21250" annotation was stale): the two narrow delegates
(`bind_cost_scope`, `cost_scope`) replace the one wide accessor `tokens()` (+3 code lines) and the emission reads the delegate (-1 line). Surface gate GREEN, NO refresh (methods are not surface items; the removed accessor
was an inherent method too). Identity-text cover is outside the kernel (maos-domain serde, no kernel line). maos-capability re-measured **1454** (+21 over 1433, +10 over the adjudicated 1444): `TokenState.cost_scope`
(`OnceLock`), `CapShard::{bind_cost_scope, cost_scope}`, ring delegates; domain +57, iac +1, cli +3, bin +7 are disclosed in `kloc.toml` as PENDING Main adjudication.

## L2 correction round 2 — 2026-11-12 (GPT P1 + P2; implemented locally on `998b36e6`, uncommitted)

No kernel line and no surface change: kernel stays 27044 / 21202 (live after 20-13 L2), so the shared STOP **22000** (ruling `lim=stop22`, 2026-11-12) leaves **798** tokei (797 at the 21203 the review quoted); the earlier "47 left under STOP 21250" annotations are corrected in the story, this record, the tracker and the evidence. The two fixes are in `maos-iac` (the erasure walker decodes only the cost frame's `principal_id` / `task_ref`; `subject_in_frame` becomes public) and `maos-bin` (the completed-forget replay uses that predicate). Measured pins: maos-iac 8771 -> **8773** (+2), maos-bin 38563 -> **38564** (+1), aggregate 202920 / hardfail 202921: the ceiling draw over the live pins is for Main adjudication. maos-capability 1454, maos-domain 10274, maos-cli 6612 are unchanged and remain "Main-adjudicated 2026-11-12, supported as the cheapest correct design: see the review's overdraft assessment" (`landing-2/review/gpt-review-r2.md`).
