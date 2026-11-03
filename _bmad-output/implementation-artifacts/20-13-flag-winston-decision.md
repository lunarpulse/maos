# 20-13 FLAG-Winston decision record

**Date:** 2026-11-03  
**Status:** `REQUESTED` — PROPOSED at story creation; **not granted**. The operator (Lunarpulse) rules D-20-13-04 and D-20-13-13; this record is rewritten to the ruled shape then.  
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
| **KT3** | `new(policy, crypto)` required; T3 arm uses `self.crypto`; `impl Default` removed; manual `Debug` | **+1** (26842) | **−1** (21041) | REQUESTED (D-20-13-04 A) |
| KT1 | builder `with_crypto_provider`, `ring` default kept | +20 (26861) | +14 (21056) | priced (D-04 B) |
| KTd | KT1 with a `Debug` supertrait on `CryptoProvider` | +11 (26852) | +6 (21048) | priced (D-04 B variant) |
| KI | `CryptoProvider::identity()` | +19 (26860) | +18 (21060) | priced (D-20-13-03 B; not requested) |
| KO | `CryptoProvider::open_sealed` | +38 (26879) | +37 (21079) | priced (D-20-13-11 B; not requested) |
| none | inline `ring` stays | 0 | 0 | priced (D-04 C) |

Requested ceilings (STOP above): **kernel ≤ 26843 physical / 21042 tokei**. Every shape moves `security/mod.rs`'s content hash.

## Kernel grant and ADR-069 room

Base `8c809f9c`: **26841 physical / 21042 tokei** (20-7 K1w+K2r landed). Ruled, unlanded rows: 20-1b +68 (computed from
21011, re-base owed), 20-4 +9, 20-11 +6 → expected **21125**. With KT3: **21124** expected, ceiling sum 21125; physical
26943. STOP **21209**; ADR-069 alarm **22000** (876 below at the KT3 expectation), envelope **23000**. No alarm row.

**Concurrent PROPOSED rows at the same HEAD:** 20-8 KS +33, 20-14 KT +25, 20-9 R +21 → expectation 21204 before this
row, **21203 with KT3** (6 below 21209). Every costlier shape crosses the STOP: KT1 21218, KTd 21210, KT3+KI 21221, any KO
≥ 21240 → story D-20-13-16 (split / exchange / bump, operator).

## Landing and accounting (if granted)

L1 implements AC1–AC6 without a kernel line; the kernel T3 line is a named pending entry in the structural check. L2
implements AC7 under the ruled row last, re-bases on whichever of 20-1b / 20-4 / 20-11 landed first, re-measures formatted
code and pins `xtask/kernel-core-baseline.toml:505` (+ content hash, EOF HISTORY) and `xtask/kloc.toml:264` exactly, with a
HISTORY first segment citing this key with `->`. The kernel-surface baseline (`docs/ci-baselines/kernel-surface-v0.1-beta.json:912`)
is re-checked after the private field and the removed `Default`. Unused room is not reserved.
