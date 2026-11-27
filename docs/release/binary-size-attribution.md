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

## Forecast of the pending stories (forecasts, NOT measurements; every one is an `[INFERENCE]` or an unverified estimate)

Story files are in the docs worktree `/home/lunarpulse/dev_ws-course28/_bmad-output/implementation-artifacts/`; the design is in
`/home/lunarpulse/dev_ws-course28/_bmad-output/planning-artifacts/`.

| Story | Forecast growth | Source record | Status of the figure |
|---|---:|---|---|
| 23-1 (MCP OAuth) | +60 KB .. +180 KB | `23-1-mcp-oauth-client.md:91` (also D-23-1-16, `:123`) | story-file forecast, `[INFERENCE]`, scaled from 23-8 (+126,336 B) and 23-0b (about +17 KB) |
| 23-4 (SIEM sink) | +236 KB | no record found: `23-4-siem-network-sink-and-durable-watermark.md` and its evidence README (`/home/lunarpulse/dev_ws-e234/.../23-4-siem-network-sink-and-durable-watermark-evidence/README.md`) carry no such figure; the evidence README `:9` only measures the branch at 39,082,192 B stripped (no base recorded next to it) | **unverified estimate**; the only traceable datum is that measurement |
| 23-5 (A2A v1 agent card, JSON-RPC) | +300 KB .. +600 KB | `23-5-a2a-v1-agent-card-and-jsonrpc-binding.md:95`, D-23-5-09 `:120` (T1 is a size spike before development) | story-file forecast, `[INFERENCE]`; **precondition: the measure-first spike** |
| 23-6 (enterprise posture, air-gap) | +1 KB .. +4 KB | `23-6-enterprise-posture-in-air-gap.md:105` (+15 KB .. +40 KB under D-23-6-07 B, not ruled) | story-file forecast, `[INFERENCE]` |
| 23-7 (peer identity rotation) | **+200 KB .. +320 KB** (redesign) | `23-7-rotation-protocol-design-2026-11-26.md:457`: net over main for the designed protocol | design-document forecast. The earlier figure, +183 KB, was the MEASURED +182,848 B of the paused implementation (`/home/lunarpulse/dev_ws-e237/.../23-7-peer-identity-rotation-evidence/README.md:102`: 39,093,904 B vs main 38,911,056 B); the design supersedes it |

The arithmetic below is **not evidence of headroom**: it adds unverified estimates (23-4 in particular) and forecasts to one
measured base. Treat it as an order-of-magnitude plan only; each story still measures at landing against the limit file.

| | Bytes |
|---|---:|
| 23-2 measured | 39,035,352 |
| + forecast low (60 + 236 + 300 + 1 + 200 = 797 KB, decimal) | 39,832,352 |
| + forecast high (180 + 236 + 600 + 4 + 320 = 1,340 KB, decimal) | 40,375,352 |
| New limit | 41,943,040 |
| Arithmetic difference to the limit at the high forecast (unverified) | 1,567,688 |

Even the low end exceeds the former 39,069,022 B limit (by 763,330 B). The raise is sized for the Epic-23 stories' forecasts,
not for new surface; the sum is dominated by the 23-5 spike result and the unsourced 23-4 figure.

## Follow-up: `later-binary-size-attribution`

Open work item (a separate story, not part of any Epic-23 story):

1. Build the release `maos` as the gate does, keep an unstripped copy, and record `nm -C --print-size --size-sort` and
   `size -A` for the largest contributors, grouped by crate.
2. Attribute the growth in the table above to concrete crates, monomorphisations and dependencies (the 23-0/23-0b/23-2/23-8
   figures to the online-adapter stack: TLS, HTTP, KMS, OIDC, SIEM).
3. Re-evaluate the executable-only `codegen-units = 1` override (measured -3,550,080 B on 2026-10-26; not validated for
   latency or behavior) and any feature split of the default binary.
4. Propose a tighter limit with a rule-compliant attributed headroom.
