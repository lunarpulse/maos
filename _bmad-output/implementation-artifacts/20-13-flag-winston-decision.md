# 20-13 FLAG-Winston decision record

**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03 (ask `d_batch4`): D-20-13-04 A and D-20-13-13 A as recommended. KT3 remains a measured request, binding at dev start; no implementation or landing claimed.  
**Scope:** kernel row **KT3** for `20-13-pluggable-crypto-provider-selection` (AC7, L2 kernel last); no implementation or landing claimed.  
**Story:** `_bmad-output/implementation-artifacts/20-13-pluggable-crypto-provider-selection.md`, frontmatter, §Decisions, §Kernel and kloc pricing.  
**Evidence:** `20-13-pluggable-crypto-provider-selection-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `kt3-required-ctor-param.diff`, `kt1-builder-ring-default.diff`, `ktd-builder-debug-supertrait.diff`, `ki-trait-identity.diff`, `ko-trait-open-sealed.diff`, `gate-sim-base.txt`.  
**Precedent:** `20-4-flag-winston-decision.md`, `20-7-flag-winston-decision.md`.

## Why the kernel is touched

`SecurityManagerAdapter::admit_spirit`'s T3 arm builds its own `RingCryptoProvider` to verify the image lock
(`crates/maos-kernel-core/src/security/mod.rs:350` at `8c809f9c`), whatever crypto provider the composition root selected.
It is the one kernel-internal FR48 verification not reached through an injected instance, and the structural check (AC3)
finds it (`gate-sim-base.txt`).

## Requested shape and alternatives (reverted prototypes, `git archive 8c809f9c`)

| ID | Shape | Kernel physical | Kernel tokei | Status |
|---|---|---:|---:|---|
| **KT3** | `new(policy, crypto)` required; T3 arm uses `self.crypto`; `impl Default` removed; manual `Debug` | **+1** (26842) | **−1** (21041) | RULED 2026-11-03: **A** (ask d_batch4); measured request |
| KT1 | builder `with_crypto_provider`, `ring` default kept | +20 (26861) | +14 (21056) | priced (D-04 B) |
| KTd | KT1 with a `Debug` supertrait on `CryptoProvider` | +11 (26852) | +6 (21048) | priced (D-04 B variant) |
| KI | `CryptoProvider::identity()` | +19 (26860) | +18 (21060) | priced (D-20-13-03 B; not requested) |
| KO | `CryptoProvider::open_sealed` | +38 (26879) | +37 (21079) | priced (D-20-13-11 B; not requested) |
| none | inline `ring` stays | 0 | 0 | priced (D-04 C) |

Requested ceilings (STOP above): **kernel ≤ 26843 physical / 21042 tokei**. Every shape moves `security/mod.rs`'s content hash.

## Kernel grant and ADR-069 room

Expected cumulative measured tokei: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = **21203**. Shared internal STOP **21209**; D-20-9-15 A sets **21230** for 20-9 L2 only; ADR-069 alarm **22000** unchanged. Later kernel landers re-base on the exact landed pins.

D-20-14-12 C (ask `d2014_12`, 2026-11-03) sets joint STOP **21250** for the combined 20-9 L2 + 20-14 L2 cumulative; shared STOP **21209** applies otherwise.

## Landing and accounting (if granted)

L1 implements AC1–AC6 without a kernel line; the kernel T3 line is a named pending entry in the structural check. L2
implements AC7 under the ruled row last, re-bases on whichever of 20-1b / 20-4 / 20-11 landed first, re-measures formatted
code and pins `xtask/kernel-core-baseline.toml:505` (+ content hash, EOF HISTORY) and `xtask/kloc.toml:264` exactly, with a
HISTORY first segment citing this key with `->`. The kernel-surface baseline (`docs/ci-baselines/kernel-surface-v0.1-beta.json:912`)
is re-checked after the private field and the removed `Default`. Unused room is not reserved.
