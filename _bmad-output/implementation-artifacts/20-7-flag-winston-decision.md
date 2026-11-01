# 20-7 FLAG-Winston decision record

**Date:** 2026-11-02  
**Status:** `GRANTED` 2026-11-01/02 by the operator (Lunarpulse), ask `rows207` (D-rows, kernel row R and the proposed ceilings), ruled as recommended. Landing 1 of the story uses it; the record is the landing's own accounting.  
**Scope:** the kernel bundle **R = K1w + K2r** for story `20-7-stable-runner-perf-gates` (the T3 teardown kill and removal path in `crates/maos-kernel-core/src/security/sandbox/t3/child.rs`), and the per-crate ceilings for the same story.  
**Model after:** `20-3c-flag-winston-decision.md`.  
**Story:** `_bmad-output/implementation-artifacts/20-7-stable-runner-perf-gates.md`, §Decisions and §Kernel and kloc pricing.  
**Evidence:** `20-7-stable-runner-perf-gates-evidence/creation/` (prototype diffs `k1w-wait-snapshot-then-remove.diff`, `k2r-pidfd-kill-rustix.diff`, `k1wr-k1w-k2r-bundle.diff`; `measurements.md`), `20-7-stable-runner-perf-gates-evidence/landing-1/plants.md`, `20-7-stable-runner-perf-gates-evidence/ci/ci-20-7-verify.yml`.

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-20-7-01** | S3: **A, K1w** — before removal, teardown waits (bounded by its own deadline) for an in-flight snapshot owner to publish, then removes; snapshot-before-removal order kept. | RULED 2026-11-01/02: **A** |
| **D-20-7-02** | ac4: **A, K2r** — teardown SIGKILLs the verified payload pidfd (`rustix::process::pidfd_send_signal`, feature `process`) before the bounded runtime `kill`; gated on the hosted diagnostic (`ci/20-7-diag`). | RULED: **A** |
| D-20-7-03…08 | supervisor drop-panic repair (A); cap-token claim relocated (A); one key, two landings (A); real-spawn bench via `WorkerSupervisor` (A); Tier C valgrind/callgrind + counting allocator (A); per-crate envelope ×2/×3 (A). | RULED: **A** each |

## Kernel grant R = K1w + K2r

Measured by the creation prototype from base **26801 physical / 21011 tokei**: +40 / +31 (K1w +18/+15, K2r +22/+16). **Ceiling** (×2 measured, D-20-7-08 A): **26881 physical / 21073 tokei**; STOP above.

**Landed (exact, formatted, `kloc-check --json`):** **26841 physical / 21042 tokei** — identical to the prototype; 95 files; public surface unchanged (no `kernel-surface` or `kernel-api-classes.toml` edit); only `t3/child.rs` content changed (plus `Cargo.toml`: `rustix` gains feature `process`). `kernel-core-baseline.toml` `src_lines` stays on line 505; `kernel_pin_content_hash_16_0.rs` and `recovery_lane_ceiling_rule.rs` re-pinned.

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): 21042 is 958 under the 22000 alarm; no ENVELOPE ALARM row.

## Cumulative re-base rule (two kernel grants from one base)

The `20-1b` (FR13 real quarantine) kernel grant and this one were each computed from HEAD **26801 / 21011**. If both land, **the second to land MUST re-base** against the first's landed pin before pinning. Expected cumulative: 20-1b measured +68 and 20-7 measured +31 ⇒ **21110 tokei**. Cumulative ceilings must not exceed **21209 tokei** (= 791 under the 22000 alarm): STOP and return to the operator otherwise.

## Per-crate ceilings and what landed (tokei, `kloc-check --json`)

| Crate | HEAD | Ceiling | Landed | Headroom |
|---|---:|---:|---:|---:|
| maos-kernel-core | 21011 | 21073 | 21042 | 31 |
| xtask | 42860 | 45830 | 43910 | 1920 |
| maos-bench | 2451 | 3051 | 2737 | 314 |
| maos-bin | 33195 | 33315 | 33215 | 100 |
| aggregate hardfail (measured + 1) | 188817 | 192568 | 190204 | — |

Pins are exact; unused headroom is not held. Hosted-only legs (K2r diagnostic, S3/ac4 witnesses, Tier C instruction baselines, the real-spawn smoke) are unrun at landing; see the story's Dev Agent Record.
