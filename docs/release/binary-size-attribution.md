# Release binary size: ceiling history, per-story attribution, and the 2026-11-26 raise

The release-size gate (`tests/integration/onb_nfr2_timing.sh` step 5, `xtask prepush` `binary-size`) measures the stripped
`maos` produced by `cargo build --locked --release -p maos-bin --bin maos` followed by `strip`. Both read the one number in
`tests/integration/release-size-limit`.

## Ruling (operator, 2026-11-26)

- The limit is raised **once**, from **39,069,022 B** to **40 MiB = 41,943,040 B** (+2,874,018 B).
- The rule "attribute with `nm`/`size` before re-basing" (set 2026-09-20, waived once on 2026-11-12) is **waived one more
  time** for this raise. The waiver is recorded, not met: the growth below is attributed **per story from the stories' own
  evidence READMEs**, not by an `nm --size-sort` decomposition of the binary.
- The attribution that was skipped is the follow-up **`later-binary-size-attribution`**: an `nm -C --print-size --size-sort`
  (and `size -A`) attribution of the largest contributors on the release `maos`, and a plan to bring the binary back under a
  tighter limit (candidates already measured on 2026-10-26: executable-only `codegen-units = 1`, see
  `_bmad-output/implementation-artifacts/18-4-evidence/binary-size-attribution.md`; splitting reference Spirits out of the
  default binary). Until it lands, 41,943,040 B is the ceiling and no further raise is pre-approved.

## Ceiling history

| Date | Limit | Basis |
|---|---:|---|
| 2026-06-11 | 16 MiB | raised from 10 MiB |
| 2026-07-14 | 24 MiB | Epic-12 retrospective |
| 2026-09-08 | 28 MiB | Story 15-1 review (26,473,608 B measured) |
| 2026-09-20 | 32 MiB = 33,554,432 B | Epic-16 retrospective, measured re-base (30,574,448 B + headroom) |
| 2026-11-12 | 34 MiB = 35,651,584 B | operator ruling `size`; +119,808 B past 32 MiB unattributed; attribution waived once |
| 2026-11-21 | 39,069,022 B | D-21-9-04 A: wasm-host default-on, measured 37,208,592 B, +5 % rounded up |
| **2026-11-26** | **41,943,040 B = 40 MiB** | **this ruling; attribution waived a second time** |

## Measured growth on main (stripped release `maos`, same method)

Figures are those recorded in each story's evidence README; "not recorded" means the record does not carry a before/after pair.
A v1.0.0 stripped size is **not recorded** in `docs/release/v1.0.0.md`; the nearest records are the 2026-09-20 Epic-16 close
(30,574,448 B) and the 21-9 default-on measurement (37,208,592 B).

| Story | Before (B) | After (B) | Delta (B) | Source |
|---|---:|---:|---:|---|
| Epic-15 close `843d5365` | | 26,222,192 | | `onb_nfr2_timing.sh` 2026-09-20 note |
| Epic-16 close `cc3e7089` | 26,222,192 | 30,574,448 | +4,352,256 | same |
| 18-4 (Butler behavior) | 33,601,008 | 33,795,056 | +194,048 (local; CI +292,864) | `18-4-evidence/binary-size-attribution.md` |
| 18-5 (context budget) | 33,795,056 | 33,854,712 | +59,656 (local) | same |
| 21-9 (wasm-host default-on) | | 37,208,592 | not recorded | `21-9-wasm-host-default-on.md` |
| 22-x stories | | | not recorded | no per-story size pair in the 22-x evidence |
| 23-0 (online adapters skeleton, outbound policy) | 38,507,088 | 38,524,752 | +17,664 | `23-0-...-evidence/README.md` AC7 |
| 23-8 (external KMS adapter) | 38,524,752 | 38,651,088 | +126,336 | `23-8-external-kms-adapter-evidence/README.md` |
| 23-0b (outbound connection binding) | | 38,911,056 | not recorded as a pair (the README gives only the absolute value; the 23-2 README names 38,911,056 as "main before 23-0b" base of its own measurement) | `23-0b-...-evidence/README.md` |
| 23-2 (OIDC discovery, key rotation) | 38,911,056 | 39,035,352 (final) | +124,296 (round 0: 38,993,104, +82,048) | `23-2-...-evidence/README.md` |

Headroom under the former 39,069,022 B limit at 23-2: 33,670 B. The 23-8 figures and the 23-2 figures were taken on different
stacked bases; they are not additive on one tree. The deltas above are not re-measured by this document.

## Forecast of the pending stories (story-file forecasts, not measurements)

| Story | Forecast growth | Note |
|---|---:|---|
| 23-1 (MCP OAuth) | +60 KB .. +180 KB | |
| 23-4 (SIEM sink) | +236 KB | |
| 23-5 | +300 KB .. +600 KB | **precondition: a measure-first spike** before the story is accepted |
| 23-6 | +1 KB .. +4 KB | |
| 23-7 (cohort mesh) | +183 KB | implementation paused; protocol and state-machine design first |

Sum of the forecasts: **+780 KB .. +1,203 KB** (decimal KB), added to the 23-2 measured 39,035,352 B:

| | Bytes |
|---|---:|
| 23-2 measured | 39,035,352 |
| + forecast low (60 + 236 + 300 + 1 + 183 = 780 KB, decimal) | 39,815,352 |
| + forecast high (180 + 236 + 600 + 4 + 183 = 1,203 KB, decimal) | 40,238,352 |
| New limit | 41,943,040 |
| Headroom at the high forecast | 1,704,688 |

Even the low end exceeds the former 39,069,022 B limit (by 746,330 B). The raise therefore buys the Epic-23 stories their
forecast plus about 1.7 MB; it is not headroom for new surface.

## Follow-up: `later-binary-size-attribution`

Open work item (a separate story, not part of any Epic-23 story):

1. Build the release `maos` as the gate does, keep an unstripped copy, and record `nm -C --print-size --size-sort` and
   `size -A` for the largest contributors, grouped by crate.
2. Attribute the growth in the table above to concrete crates, monomorphisations and dependencies (the 23-0/23-0b/23-2/23-8
   figures to the online-adapter stack: TLS, HTTP, KMS, OIDC, SIEM).
3. Re-evaluate the executable-only `codegen-units = 1` override (measured -3,550,080 B on 2026-10-26; not validated for
   latency or behavior) and any feature split of the default binary.
4. Propose a tighter limit with a rule-compliant attributed headroom.
