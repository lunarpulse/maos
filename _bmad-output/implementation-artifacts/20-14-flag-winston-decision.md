# 20-14 FLAG-Winston decision record

**Date:** 2026-11-03  
**Status:** `PROPOSED` — awaiting the operator's ruling on D-20-14-02 and D-20-14-03 (and the envelope row D-20-14-09). Nothing is granted by this file.  
**Scope:** the kernel row of `20-14-complete-cost-attribution`: the cost-attribution emission site reads a root-bound task / session-principal scope; no implementation or landing claimed.  
**Story:** `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution.md`, frontmatter, §Decisions, §Kernel and kloc pricing.  
**Evidence:** `20-14-complete-cost-attribution-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `kt-token-bound-scope.diff`, `km-scb-marker.diff`, `km-naive-scb-marker.diff`, `kp-root-cost-scope-port.diff`, `k0-token-on-cost-row.diff`, `kr-request-correlation-id-on-km-naive.diff`, `rec-kt-domain-cli-audit-bin-shell-slice.diff`.  
**Precedent:** `20-4-flag-winston-decision.md`, `20-7-flag-winston-decision.md`.

## Decision requested

| ID | Options priced (reverted prototypes on `git archive 8c809f9c`) | Recommended | Status |
|---|---|---|---|
| D-20-14-02 | **A** KM SCB marker +44/+30 (naive formatting +59/+47) · **B** KT token-bound scope in the capability registry +35/+25 · **C** KP root `CostScopePort` +31/+25 (domain +7) · **D** K0 token bytes on the cost row, report-side join, 0/0 lines (content hash moves) | **B** | PROPOSED |
| D-20-14-12 | **A** split (no allowance; ceiling = measured) · **B** exchange to K0 · **C** joint targeted cumulative STOP 21250 with D-20-9-15 | **C** | PROPOSED |
| D-20-14-03 | **A** `InferenceRequest.correlation_id` as Spirit-declared `request_ref` (+1/+1 on top, domain +9) · **B** none · **C** request id as the task source | **B** | PROPOSED |

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

Cumulative: the ruled, unlanded rows (20-1b +68, 20-4 +9, 20-11 +6) give the recorded expected **21125**; with KT
21150; with the other rows filed PROPOSED at the same HEAD (20-8 +33, 20-9 +21, 20-13 −1) **21203** — 6 under the
cumulative STOP **21209**, 797 under ADR-069's **22000** alarm. With KT at its ×2 ceiling **21228**, and with 20-9's too
21249 — above the STOP. **D-20-14-12** asks split / exchange / bump; recommended **C**, one joint targeted STOP of
**21250** ruled with D-20-9-15. The later kernel lander (expected this story, preflight `:142`) re-bases against the
exact then-landed pins and returns to the operator above whatever STOP is ruled.

## Landing and accounting (if ruled)

L1 implements every kernel-free part (payload field, report axes, Spirit naming, docs) without a kernel line; L2
implements KT plus the root binding sites last, re-measures formatted code, and pins `xtask/kernel-core-baseline.toml`
(`src_lines`, the `capability/mod.rs` and `inference/mod.rs` content hashes, an EOF HISTORY row) and `xtask/kloc.toml`
`maos-kernel-core` exactly, each HISTORY row's first segment citing this key with `->` (D11 A). Unused room is not
reserved. Any `crates/maos-kernel-core/src` change outside KT's two files is STOP.
