# 20-3c FLAG-Winston decision record

**Date:** 2026-11-01  
**Status:** `GRANTED` 2026-11-01 by the operator (Lunarpulse), asks `rows203c` (D-rows) and `grant203c` (envelope). Ruled as recommended.  
**Scope:** the kernel bundle **R = K1p + E3** for story `20-3c-audit-truth-and-halt-receipt-residuals`, and the per-crate ceilings for the same story. This record changes no runtime, kernel, baseline, KLOC ledger, or ADR. The kernel and KLOC pins move only at the landing (L2), see §Grant protocol.  
**Model after:** `17-2-flag-winston-decision.md` (numerical grant with a landing protocol).  
**Story:** `_bmad-output/implementation-artifacts/20-3c-audit-truth-and-halt-receipt-residuals.md`, §Decisions (lines 150–170) and §Kernel and kloc pricing (line 92).  
**Evidence:** `20-3c-audit-truth-and-halt-receipt-residuals-evidence/creation/` (prototype diffs `k1p-carry-and-precheck.diff`, `krec-k1p-e3-bundle.diff`, `e3-ratelimited-journal-or-count.diff`, `e1-ratelimited-counted-drop.diff`, `e2-ratelimited-sync-journal.diff`, `k1-carry-not-drain.diff`, `k2-drain-with-swap-receipt.diff`, `kalt-k1p-e1-bundle.diff`, `p17-sender-pid-attribution.diff`; measurements in `creation/kloc-kernel-3341f9e4-and-prototypes.json` and `creation/measurements.md`; cites in `creation/cites-3341f9e4.md`).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D05** (preflight) | 3c inventory and governance truth: reconcile closed members 1–10 and 13–15, keep live items, stage2 handled by D-20-3c-07. | RULED 2026-11-01 (rows203c): **A** |
| **D07** (preflight) | Watchdog fire after mint refusal: decided at 20-9, not in this story. | RULED 2026-11-01 (rows203c): **A, at 20-9** |
| **D10** (preflight) | Endpoint record ownership (member 16): boot-time socket lease and atomically published discovery. | RULED 2026-11-01 (rows203c): **A** |
| **D13** (preflight) | Reliability evidence policy: repair real causes, fail-closed tests, no silent retry or skip. | RULED 2026-11-01 (rows203c): **A** |
| **D-20-3c-01** | FR53 / member 12 kernel shape: **K1p** (carry pending halts across a hot swap, no drain; precheck uses the same set; −8 physical / +7 tokei). | RULED 2026-11-01: **A** |
| **D-20-3c-02** | EC5 shape: **E3** (synchronous kind-23 journal row; failed spawned delivery is a counted `RateLimited` drop; +20 / +19, maos-capability +1). | RULED 2026-11-01: **A** |
| **D-20-3c-03** | Member 17 attribution per site (maos-bin +2 measured). | RULED 2026-11-01: **A** |
| **D-20-3c-04** | Member 11 one-shot drains: fix at origin, drain timeout typed non-zero exit. | RULED 2026-11-01: **A** |
| **D-20-3c-05** | FR62(b) emitter: writer only; the gate keeps `xtask/abi-ratifications.toml`; ADR-045 §4 amended to say so. | RULED 2026-11-01: **B** |
| **D-20-3c-06** | FR41 `--tag`: implement exact match on the TL `intent` column. | RULED 2026-11-01: **A** |
| **D-20-3c-07** | Reza `stage2-refusal-live` leg: relabel (no requirement names a live stage-2 refusal; `evidence_ledger.rs:252-253`). | RULED 2026-11-01: **B** |
| **D-20-3c-08** | Witnesses for shipped-but-unwitnessed members 8 and 14: add kloc-free regression vectors. | RULED 2026-11-01: **A** |
| **D-20-3c-09** | Per-crate envelope: ×3 unmeasured / ×2 measured. | RULED 2026-11-01: **A** |
| **D-20-3c-10** | Landing shape: L1 kernel-free (AC1 partial, AC4–AC11), then L2 FLAG-Winston (AC2, AC3) and AC1 close; `done` after L2. | RULED 2026-11-01: **A** |
| **D-20-3c-11** | NFR-Ops-9 RPO/RTO: label honestly (`mode: synthetic|operational`); the production cadence proof goes to the operator lane `ops-rpo-rto-cadence-proof`. | RULED 2026-11-01: **B** |

Still open in the preflight: D06, D09, D12 (not in this record).

## Kernel grant R = K1p + E3

**Measured at creation** (reverted prototypes on `git archive` copies at 3341f9e4; prebuilt gates), from base **26793 physical / 20992 tokei**:

- K1p: **−8 physical / +7 tokei**.
- E3: **+20 physical / +19 tokei**.
- R (sum): **+12 physical / +26 tokei** → measured **26805 / 21018**.

**Granted ceiling** = base + 2 × R (D-20-3c-09 A, ×2 measured):

- Physical: **26817** (26793 + 2×12).
- Tokei: **21044** (20992 + 2×26).

STOP above these ceilings. A landing figure above them returns to the operator as a new request.

**Not granted** (measured alternatives, see story §Kernel and kloc pricing):

| Alternative | Physical / tokei |
|---|---|
| R′ = K1p + E1 | −1 / +13 |
| K1 (no precheck change) | −15 / +1 |
| K2 (drain with `SwapDrained` receipts) | +15 / +14 |
| E1 (counted drop only) | +7 / +6 |
| E2 (synchronous journal, delivery error discarded) | +14 / +13 |

Note: the kernel figure is the epic's `kernel-Δ TBD by measurement`, resolved here to R only.

## ADR-069 room

`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`: envelope 23000; alarm 22000 (`:42-45`).

- Measured tokei after R: 21018, which is **982 under** the 22000 alarm.
- Granted ceiling tokei 21044, which is **956 under** the alarm.

The alarm is not crossed, so no `ENVELOPE ALARM` row is needed.

## Per-crate ceilings (grant203c)

| Crate / ledger row | Ceiling (≤) | Note |
|---|---:|---|
| kernel (physical / tokei) | 26817 / 21044 | R = K1p + E3 (this record) |
| maos-capability | 1434 | E3 +1 |
| maos-bin | 33559 | member 17 +2 measured |
| maos-domain | 10097 | |
| maos-cli | 6446 | raised above its 6382 STOP (D-20-3c-06 FR41 `--tag`) |
| maos-audit | 7797 | |
| maos-shell | 626 | |
| maos-iac | 8345 | |
| xtask | 42901 | |
| aggregate | 189478 | |

Hardfail = measured + 1 (the established aggregate rule). Pins are re-fixed exactly at landing, not now.

## Grant protocol (landing order)

The landing re-measures after `cargo fmt --all` and re-pins the **exact lower** figures. The pins the epic cites (`epic-20-ship-it-w5.md` §20-3c) are:

1. `xtask/kernel-core-baseline.toml:27-28` (both kernel pins), `:505` (`src_lines`), `:915` (set hash).
2. `xtask/kloc.toml:264` (kernel tokei and the per-crate rows).
3. Run `check-kernel-baseline` and `kloc-check` at HEAD after the re-pin.

Order: **L1** kernel-free (AC1 partial, AC4–AC11) lands first. **L2** (AC2, AC3, kernel pins, AC1 close) lands second. The story is `done` only after L2. Kernel pins move in one change at L2, serialized with 20-9, 20-13 and 20-14 kernel pin updates (one integration owner).

Any figure above a ceiling returns to the operator as a measured request **before** landing.
