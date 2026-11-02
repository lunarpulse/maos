# 20-4 FLAG-Winston decision record

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

20-7's kernel landing is already in the creation base. The later kernel lander among 20-1b, 20-4 and 20-11 MUST re-base against the exact then-landed pins before pinning. Expected cumulative measured tokei: **21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) = 21125**, leaving **875** below the alarm. Cumulative STOP is **21209** (791 below the alarm), not the sum of independent standalone ceilings; return to the operator above it.

## Landing and accounting

L1 implements AC1–AC6 and partial records without a kernel line. L2 implements AC7/AC8 under R, re-measures formatted code, and pins both `xtask/kernel-core-baseline.toml` and `xtask/kloc.toml` exactly, with content hashes and HISTORY rows citing this story. Unused room is not reserved. All non-kernel ceilings and aggregate measured + 1 rule remain the story frontmatter's granted values. The story remains `ready-for-dev`; no prototype was applied and no implementation evidence is created by this ruling record.
