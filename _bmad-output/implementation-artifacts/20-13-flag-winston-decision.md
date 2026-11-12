# 20-13 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03 (ask `d_batch4`): D-20-13-04 A and D-20-13-13 A as recommended. KT3 remains a measured request, binding at dev start. **2026-11-12 (ask `kt3`): the literal shape ruled and the L2 candidate measured (+1 / −1), see §L2 ruling below; landing is claimed only after the hosted run and review.**  
**Scope:** kernel row **KT3** for `20-13-pluggable-crypto-provider-selection` (AC7, L2 kernel last).  
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

## L2 ruling and measured landing — 2026-11-12 (ask `kt3`)

**Operator ruling (2026-11-12, ask `kt3`): the LITERAL KT3 shape.** `SecurityManagerAdapter` gains the
`crypto: Arc<dyn CryptoProvider>` field on the public struct (`new(policy, crypto)` required, `impl Default`
removed, manual `Debug`), and exactly ONE surface-hash refresh is granted: `maos_kernel_core::security::SecurityManagerAdapter`
(struct hash `76:bb:bb:10:…:c4:96:7e` -> `60:35:04:56:…:50:73:a3:79`; the full hashes are in `landing-2/evidence.md`). The earlier
"private `T3ImageVerificationConfig`" alternative priced by the pre-landing pricing is NOT adopted. The gate was
required to flag nothing else, and did not (`check-service-boundary` before the refresh: one violation, the named
symbol; no other symbol changed or was added). Per the 2026-11-12 standing pre-approval (`prepaid` (a)) the
refresh also names a symbol in this measured grant; both authorisations are recorded.

**Measured L2 grant (live base `456793ca`, after `cargo fmt --all`):** kernel **27043 -> 27044 physical (+1),
21203 -> 21202 tokei (-1)**, 95 files, none added or removed — the creation-priced KT3 figure (+1 / -1) exactly.
Cumulative measured tokei **21202**, under the shared STOP **22000** set by the 2026-11-12 operator ruling
(`decisions-2026-11-12-acceleration.md`); ADR-069 alarm 22000 / envelope 23000 untouched. Zero headroom; unused room
is not reserved. Other crates against their live pins: maos-capability 1454, maos-domain 10274, maos-iac 8771,
maos-cli 6612 unchanged; **maos-bin 38556 -> 38563 (+7)**, an overdraft over the live pin from migrating the four
`main.rs` call sites (disclosed for Main adjudication; the ceilings in the story are stale absolutes), xtask
45379 -> 45355 (-24, the pending kernel T3 gate exception removed); aggregate measured 202917, hardfail 202918.

**Landing:** the L1 pending gate entry (`pending_t3` / `security_impl` visitor state) is removed; the structural
check now refuses every production `RingCryptoProvider` naming outside its allowlist, `security/mod.rs`
included. Evidence, witnesses and plants: `20-13-pluggable-crypto-provider-selection-evidence/landing-2/evidence.md`.

**Adjudication (2026-11-12, Main):** the maos-bin +7 overdraft (38,556 -> 38,563) recorded at the L2 landing `998b36e6` is adjudicated as the cost of migrating the four `main.rs` `SecurityManagerAdapter` call sites; the kernel stays +1/-1 (27,044 physical / 21,202 tokei, below STOP 22,000).
