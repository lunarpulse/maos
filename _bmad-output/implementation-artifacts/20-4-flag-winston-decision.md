# 20-4 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-02  
**Status:** `GRANTED` by operator Lunarpulse, ask `s204`, D-20-4-06 A + D-20-4-07 A; every other D-row ruled as recommended, D06 D.  
**Scope:** kernel bundle **R = KA + KIO1 + KIO2** for `20-4-gemini-driver-and-endpoint-pin`; no implementation or landing claimed.  
**Story:** `_bmad-output/implementation-artifacts/20-4-gemini-driver-and-endpoint-pin.md`, frontmatter, §Decisions, §Kernel and kloc pricing.  
**Evidence:** `20-4-gemini-driver-and-endpoint-pin-evidence/creation/measurements.md`, `kloc-kernel-a8af3215-and-prototypes.json`, `ka-intern-gemini-kimi.diff`, `kio1-badheader-redaction.diff`, `kio2-non2xx-truncate-keep-status.diff`, `r-ka-kio1-kio2-bundle.diff`.  
**Precedent:** `20-7-flag-winston-decision.md`.

## Decision

| ID | Granted shape | Status |
|---|---|---|
| D-20-4-06 | A, KA: intern Gemini/Kimi in the kernel provider-id set; unknown ids still fail closed | RULED 2026-11-02: **A** (ask s204) |
| D-20-4-07 | A, KIO1 + KIO2: redact BadHeader credentials and retain non-2xx status at the response cap; provider-side credential byte checks also required | RULED 2026-11-02: **A** (ask s204) |
| D-20-4-08 | A, one key: L1 kernel-free, L2 kernel last; done only after L2 | RULED 2026-11-02: **A** (ask s204) |
| D-20-4-11 | A, ×2 measured / ×3 unmeasured ceilings in story frontmatter | RULED 2026-11-02: **A** (ask s204) |

## Kernel grant and ADR-069 room

Creation base `a8af3215`: **26841 physical / 21042 tokei**. Reverted prototype: KA **+2/+2**, KIO1 **+7/+5**, KIO2 **+4/+2**; R **+13/+9**, giving **26854 / 21051** measured. Granted ceilings: **26867 physical / 21060 tokei**; STOP above. ADR-069's alarm is **22000 tokei**, envelope **23000**: derived remaining room is **949** at the prototype measurement or **940** at the standalone ceiling. Neither is spending authority; no alarm is crossed by this prototype.

20-7's kernel landing is already in the creation base. Later kernel landers MUST re-base against the exact then-landed pins. Expected cumulative measured tokei: **21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = 21203**, leaving 797 below the ADR-069 alarm. The shared cumulative STOP remains **21209**, not the sum of independent standalone ceilings. D-20-9-15 A (ask `d_batch4`, 2026-11-03) sets **21230 for 20-9 L2 only**; ADR-069 alarm **22000** is unchanged.

D-20-14-12 C (ask `d2014_12`, 2026-11-03) also sets a joint STOP **21250** for the combined 20-9 L2 + 20-14 L2 cumulative; the shared **21209** STOP remains for everything else.

## Landing and accounting

L1 implements AC1–AC6 and partial records without a kernel line. L2 implements AC7/AC8 under R, re-measures formatted code, and pins both `xtask/kernel-core-baseline.toml` and `xtask/kloc.toml` exactly, with content hashes and HISTORY rows citing this story. Unused room is not reserved. All non-kernel ceilings and aggregate measured + 1 rule remain the story frontmatter's granted values. The story remains `ready-for-dev`; no prototype was applied and no implementation evidence is created by this ruling record.

## L2 landing — measured vs granted (2026-11-11)

**Status:** `LANDED` at working-tree base `89b64566` (commit hash recorded by Main). Implements AC7/AC8 under R = KA + KIO1 + KIO2; nothing outside the granted shapes; public surface unchanged (`check-service-boundary` rc 0 with zero surface/class edits; `check-fkcs` rc 0 incl. `kernel-abi-diff`).

| Quantity | Granted (creation base `a8af3215`: 26841 / 21042) | T0 re-measure at `89b64566` (reverted prototype on a `git archive HEAD` copy) | Landed (formatted, in-tree and clean-archive overlay) |
|---|---:|---:|---:|
| Kernel physical | +13 → 26854, ceiling 26867 | 26854 (+13) | **26854** (+13; 13 under the ceiling) |
| Kernel tokei | +9 → 21051, ceiling 21060 | 21051 (+9) | **21051** (+9; 9 under the ceiling) |
| Files / set hash | 95 / `db39bc4a…` | 95 / `2c223219…` | 95 / `2c22321948fb89df25f58b4abcb54e3ff81bd2c5ffa0c8f8429d27562164375b` |
| `inference/mod.rs` digest | `cbf9975f…` | `2ed62363…` | `2ed62363003eff9c8e6364ad7464a7a22ba544f54daa836926b68aa178356746` |
| `io/mod.rs` digest | `5cff233b…` | `57d2b794…` | `57d2b794bb4cbae5deb1fbf3332132fe4a2e6b239b12a95fe5b9f48eb195619b` |
| Aggregate (clean) | — | 202121 | 202121; `_aggregate_hardfail` 202113 → **202122** (+1 over measured) |

The re-cut prototype (`creation/r-ka-kio1-kio2-bundle.diff`, never compiled at creation) applied unchanged to the current tree, compiled, and measured exactly the creation figures: per-shape KA +2/+2, KIO1 +7/+5, KIO2 +4/+2 are unchanged by the intervening landings because neither file moved since `a8af3215`. **Precedent 20-7:** measured landed exactly at the prototype (no allowance consumed beyond the prototype). Unused room (13 physical / 9 tokei) is not reserved.

**Cumulative ledger at this landing:** the kernel measures 21051 tokei / 26854 physical at the current main (the pin was 21042 / 26841, so this landing adds exactly the granted +9 / +13). That is 199 tokei under the shared STOP 21250 (supersession 2026-11-04, ask `halt_grant`), 949 under the ADR-069 alarm 22000 and 1949 under the 23000 envelope (both tokei). The supersession header's 21215 / 27048 is the forecast for the whole ledger (including 20-11 O4 and other unlanded kernel rows), not a measurement; later kernel landers re-base against 21051 / 26854. No ceiling, STOP or alarm crossed.

**Witnesses and plants:** `crates/maos-kernel-core/tests/inference_provider_ids_20_4.rs` (AC7) and `io_redaction_and_status_20_4.rs` (AC8). Each witness was observed red at HEAD with raw logs retained, green after, and red under its named plant (drop the `gemini` arm; revert KIO1; revert KIO2), restored with copy-back + `cmp` + `touch` (`20-4-gemini-driver-and-endpoint-pin-evidence/landing-2/plants.md`). Pins re-minted exactly after `cargo fmt --all`: `xtask/kernel-core-baseline.toml` (`src_lines` stays on line 505, HISTORY directly below it, set hash, two digests), `xtask/kloc.toml` kernel row and `_aggregate_hardfail` (first HISTORY segments cite `20-4-gemini-driver-and-endpoint-pin` with `->`), `xtask/tests/kernel_pin_content_hash_16_0.rs`, `xtask/tests/recovery_lane_ceiling_rule.rs`. Surface pin and `kernel-api-classes.toml` untouched (no public symbol moved).
