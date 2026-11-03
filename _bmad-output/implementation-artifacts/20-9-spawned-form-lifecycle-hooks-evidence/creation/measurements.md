# 20-9 creation — measurements (2026-11-03, HEAD `8c809f9c`)

No cargo ran: no build or test, and no target directory was set or cleaned. No crate source in `/home/lunarpulse/dev_ws`
was modified. Each prototype was made on its own copy of `git archive 8c809f9c` under `/tmp/proto209/<name>/`, and
every changed file went through `rustfmt --edition 2021` (rustfmt 1.9.0). Each copy was measured from its own root with
the prebuilt gate binary inherited from the session target directory:

`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto209/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline --json, kloc-check --json, check-service-boundary}`

Raw results: `kloc-kernel-8c809f9c-and-prototypes.json`. Diffs: `prototypes/`. Every diff passes `git apply --check`
against the HEAD copy. Against the working tree, every kernel diff applies, but `sched` conflicts in
`crates/maos-bin/src/admission.rs` with the uncommitted 20-4 L1 edits. The prototypes were not compiled ([INFERENCE] that
they build). The copies were deleted after measurement.

## Instrument replication (untouched copy `base`)

| Figure | Method | Result | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26841**, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`, rc 0 | `xtask/kernel-core-baseline.toml:505,939` |
| Kernel tokei | `kloc-check --json` | **21042** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed`, `alarm: true`, `over_budget: []`, rc 0) | bin **34191** · wasm-host **2412** · iac **8270** · domain **10099** · spirit-abi **1042** (pin 3000) · manifest **4604** · xtask **44406** · aggregate **192519** | `:517` · `:703` · `:406` · `:425` · `:270` · `:415` · `:339` · `_aggregate_hardfail` `:718` = 192520 |
| Kernel public surface | `check-service-boundary` | PASSED, 0 violations | `docs/ci-baselines/kernel-surface-v0.1-beta.json`, `xtask/kernel-api-classes.toml` |

## Kernel-touching options, priced by throwaway prototype (◆ FLAG-Winston)

All deltas are from 26841 physical / 21042 tokei. Every watchdog option changes only `scheduler/schedule_watchdog.rs`
(plus `crates/maos-iac/src/adapter/payload.rs`, which is outside the kernel).

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | maos-iac | `check-service-boundary` |
|---|---|---|---:|---:|---:|---|
| KD07A-lean | D07 A: refused declared scope ⇒ no dispatch, no fire row; already-minted scopes of that fire revoked (`CapabilityRegistryPort::revoke`); registry absent with declared scopes ⇒ refused; cadence consumed; `schedule.suppressed:<id>` row written by `maos_iac::payload::journal_schedule_suppressed` (`ScheduleSuppressedRecord`) (D-20-9-14 A) | `kd07a-lean-suppress-revoke-journal-in-iac.diff` | **+8** (26849) | **+14** (21056) | +32 | pass |
| KD07B-lean | as KD07A-lean, no revoke: minted scopes expire by TTL ≤ 300 s (D-20-9-14 B) | `kd07b-lean-suppress-no-revoke.diff` | +2 (26843) | +8 (21050) | +32 | pass |
| KD07A-inline | KD07A with the record built and journaled in the kernel (exchange not applied) (D-20-9-14 C) | `kd07a-inline-suppress-revoke.diff` | +27 (26868) | +32 (21074) | +7 | pass |
| KD07B-inline | KD07B, journaled in the kernel | `kd07b-inline-suppress-no-revoke.diff` | +21 (26862) | +26 (21068) | +7 | pass |
| **R = KD07A-lean + KO-poll-x (recommended)** | KD07A-lean, plus private `check_and_fire` becomes `pub async fn poll_once(&self, fast_mode) -> Vec<PollOutcome>`, with one outcome per due entry (`Fired`, `PrincipalRevoked`, `RateRefused`, `Suppressed`). `PollOutcome` lives in `maos_iac::payload`, so no new kernel symbol. The spawn loop calls it and ignores the result (D-20-9-13 A) | `r-kd07a-lean-plus-ko-poll-outcome-in-iac.diff` | **+16** (26857) | **+21** (21063) | **+39** | **pass** (set hash `bc7a4f4c…`) |
| KO-poll | as R, `PollOutcome` in the kernel (D-20-9-13 B) | `kd07a-lean-plus-ko-poll-outcome-in-kernel.diff` | +24 (26865) | +27 (21069) | +32 | **fails** without a `kernel-api-classes.toml` row for `…::schedule_watchdog::PollOutcome`; passes with the row (in the diff) |
| KO-count | KD07A-lean + `refused_polls: DashMap<(u32, String), u64>` + `refused_polls(pid, id)` accessor (the `last_fire_ns` pattern) (D-20-9-13 C) | `kd07a-lean-plus-ko-refused-counters.diff` | +28 (26869) | +30 (21072) | +32 | **fails**: the new field changes `ScheduleWatchdog`'s signature, which reports "removed public kernel symbol" + "new … class other". This needs a kernel-surface baseline re-issue and a classes row |
| KO-row | KD07A-lean + a `schedule.refused:<id>` TL row per refused poll (`journal_schedule_refused`, maos-iac) (D-20-9-13 D) | `kd07a-lean-plus-ko-refused-poll-tl-row.diff` | +20 (26861) | +26 (21068) | +48 | pass. **Production defect:** one row per poll per refused entry (1 s polls ⇒ up to 3600 rows/h per drained schedule) |
| KW | `HotSwapCoordinator::post_swap_window_open(pid)` (the `active_monitors` handle is not finished), so a spawned session can park its predecessor child for the window (D-20-9-11 B) | `kw-post-swap-window-accessor.diff` | +9 (26850) | +6 (21048) | 0 | pass (method only; `hot_swap/coordinator.rs`) |
| K0 | everything else in this story (hooks, WIT, swap, cold swap, schedules carriage) | — | 0 | 0 | — | — |

R + KW (two independently measured prototypes on disjoint files) = **+25 / +27**.

**Why R is the lean shape.** The kernel keeps only the decision: mint-or-suppress, revoke, outcome. The payload type, its
serialization and the TL write move to `maos-iac`, where `ScheduleFireRecord` already lives
(`crates/maos-iac/src/adapter/payload.rs:13-31`). This exchange saves 18 tokei against KD07A-inline, and moving the enum
saves 6 tokei and one invariant-lock classification against KO-poll.

## ADR-069 / cumulative STOP (tokei)

| Row | Tokei | Running expected |
|---|---:|---:|
| Creation base `8c809f9c` (20-7 K1w+K2r landed) | 21042 | 21042 |
| 20-1b R = KQ1 + P (ruled, unlanded) | +68 | 21110 |
| 20-4 KA + KIO1 + KIO2 (ruled) | +9 | 21119 |
| 20-11 KS (ruled) | +6 | 21125 |
| 20-8 KS (PROPOSED, D-20-8-07 A) | +33 | 21158 |
| 20-14 KT (PROPOSED, D-20-14-02 B) | +25 | 21183 |
| 20-13 KT3 (PROPOSED, D-20-13-04 A) | −1 | **21182** |
| **20-9 R (PROPOSED, D-20-9-13 A + D-20-9-14 A)** | **+21** | **21203** (6 below 21209) |
| 20-9 R + KW (if D-20-9-11 B) | +27 | 21209 (0 below) |
| 20-9 R at its ×2 ceiling (+42) | +42 | **21224 — above the 21209 STOP** |
| 20-9 KD07A-inline + KO-poll (no exchange) | ≈ +45 [INFERENCE: inline +32 plus KO-poll's +13 increment] | ≈ 21227 — above |

ADR-069 alarm 22000 (797 below at 21203); envelope ≤ 23000. Physical expected: 26841 + 78 + 13 + 10 + 38 + 35 + 1 + 16 =
**27032**.

**Contingencies on 20-8's ruling (D-20-8-07).** Under B (KB +8 instead of KS +33), 20-9 owns the coordinator check
(KS − KB, about +25 [INFERENCE]); the cumulative is 21157 + 21 + 25 ≈ 21203. Under C (K0), 20-9 owns KB + KS (+33, priced
by 20-8): 21149 + 21 + 33 = 21203. Every 20-8 ruling therefore lands near 21203 [INFERENCE, arithmetic on 20-8's measured
figures]. 20-9 L2 has the smallest room of any Epic-20 kernel row, so it is the expected "later lander" that returns to the
operator (D-20-9-15).

## Out-of-kernel, priced by prototype

| # | Prototype | Diff | Measured (tokei) |
|---|---|---|---:|
| SCHED | `GatedManifest.schedules`. `gate_manifest` parses `[[schedule]]` by the kernel successor parser's wrapper (`upgrade.rs:388-400`), refusing typed `[schedule]` `section_parse`, and `bundle()` carries it (D-20-9-08 A) | `sched-admission-carries-schedule.diff` | **maos-bin +14** |
| R (iac half) | `ScheduleSuppressedRecord`, `journal_schedule_suppressed`, `PollOutcome` | in R | **maos-iac +39** |

## Not measured here (owed to the dev pass; a figure above the ruled envelope is STOP + D-row)

1. Compilation of every prototype (`cargo check --locked -p maos-iac -p maos-kernel-core -p maos-bin`), in particular that
   `crate::transparency_log` resolves inside `maos-iac/src/adapter/payload.rs` (glob re-export, `lib.rs:15`) and the
   `CapabilityRegistryPort::revoke` call on `Arc<CapabilityRegistryAdapter>`.
2. WIT 2.4.0 `interface lifecycle` + `world spirit-lifecycle { include spirit; export lifecycle; }`, `bindgen!` with
   `with:` remaps so the frames/recall/context types stay shared with `wit_guest` (else a third frame-bridge copy). Lifecycle
   wire (tags 60002/60003), runner selection by export presence, `Unsupported` reply for 2.0–2.3 guests, in-export rule.
   Forecast **maos-wasm-host +80…+150** [INFERENCE; 18-5's context import landed +157, 20-8's `wit` prototype +51].
3. Guest `crates/maos-wasm-host/guests/lifecycle-probe` (charged to maos-wasm-host): **+60…+120** [INFERENCE; 20-8 F11].
4. maos-bin, all [INFERENCE]: `WorkerSpirit` forwarding of nine hooks with a bounded wait and a typed panic (+80…+140); session
   lifecycle turns serialized with frame turns, queue and in-export mapping (+60…+120); spawned successor factory (gate,
   same-form, caps ⊆, launch, pre-warm, successor core at the pid, declaration row) (+80…+150); child switch at commit, retire
   without the finalize closure, revert detection (+50…+100); spawned registration in HookPump/ConsolidationWatchdog with the
   explicit telemetry listing (+15…+40); door cold swap through the spawned admission path (+60…+140); typed refusal codes
   and door mapping (+15…+30). Total **+360…+720**.
5. maos-domain 0…+6 [INFERENCE] (only if a refusal needs a domain variant; `UpgradeError::SuccessorFactory { reason }` carries
   the typed code today). xtask `build-test-fixtures` entry for the new guest +5…+15 [INFERENCE]. maos-control 0 (policy
   parsing unchanged, `lib.rs:1469-1475`).
6. Fixture re-issue (`tests/fixtures/wasm/equiv-fixtures.provenance.toml`; every WIT byte feeds `source_sha256`,
   `wit/spirit.wit:21-23`), TS template/example, docs, tests: uncharged (`kloc_check.rs:294-317`).
7. Hosted T2 legs (`wasm-host-tests`, `check-wasm-form-equiv`, `epic-20-exit`) on the final tree.
