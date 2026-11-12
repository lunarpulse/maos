# 20-9 FLAG-Winston decision record

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


**Date:** 2026-11-03  
**Status:** `RULING RECORDED` 2026-11-03 (ask `d_batch4`): D-20-9-13 A, D-20-9-14 A, D-20-9-15 A (targeted STOP 21230 for 20-9 L2 only), and D-20-9-11 A. R remains a measured request, binding at dev start; no implementation or landing claimed.  
**Scope:** the kernel row of `20-9-spawned-form-lifecycle-hooks`: the schedule watchdog's D07 A suppression (ruled 2026-11-01, ask `rows203c`) and the PF-04 refused-poll observer. The spawned-form hooks, WIT 2.4.0, the hot swap, upgrade and cold swap, and the schedules carriage are kernel-free.  
**Story:** `_bmad-output/implementation-artifacts/20-9-spawned-form-lifecycle-hooks.md` (frontmatter, §Kernel and kloc pricing, §Decisions).  
**Evidence:** `20-9-spawned-form-lifecycle-hooks-evidence/creation/measurements.md`, `kloc-kernel-8c809f9c-and-prototypes.json`, `prototypes/*.diff`.  
**Precedent:** `20-8-flag-winston-decision.md`, `20-11-…-evidence/creation/ks-schedule-watchdog-forget-spirit.diff` (same file), `20-4-flag-winston-decision.md`.

## Why the kernel is touched at all

Both received items live inside the kernel's private `ScheduleWatchdog::check_and_fire`
(`crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs:158-268`):
- **D07 A.** The fire proceeds after a refused side-effect mint (`:216-221`, `:331-342`), and `first_id` keeps a partial mint (`:313-345`). Suppression is a change to the firing decision.
- **PF-04.** A revoked or rate-refused poll emits nothing (`:199-201`, `:212-214`), so no test can observe one without wall-clock (`tests/schedule_watchdog_fr26.rs:537-578`).

Spawned lifecycle hooks need no kernel line: the kernel already calls every hook on the spawned SCB's `WorkerSpirit`, which
no-ops them (`crates/maos-bin/src/supervision.rs:2612-2633`). The hot swap keeps the pid, the tokens and the working
memory (`hot_swap/coordinator.rs:120-455`).

## Options (measured at `8c809f9c` from 26841 physical / 21042 tokei, reverted prototypes)

| Option | Shape | Kernel physical | Kernel tokei | Other | Surface gate |
|---|---|---:|---:|---|---|
| **R = KD07A-lean + KO-poll-x (recommended: D-13 A, D-14 A)** | Suppress, revoke the fire's minted scopes, cadence consumed; `poll_once() -> Vec<PollOutcome>`; record, journal fn and enum in `maos_iac::payload` | **+16** (26857) | **+21** (21063) | iac +39 | pass |
| KD07B-lean (D-14 B) | suppress without revoke (TTL expiry) | +2 | +8 | iac +32 | pass |
| KD07A-inline (D-14 C) | KD07A journaled in the kernel | +27 | +32 | iac +7 | pass |
| KO-poll (D-13 B) | R with `PollOutcome` in the kernel | +24 | +27 | iac +32 | classes row needed |
| KO-count (D-13 C) | refused-poll counters on the struct | +28 | +30 | iac +32 | **baseline re-issue** (struct signature) |
| KO-row (D-13 D) | a TL row per refused poll | +20 | +26 | iac +48 | pass; row flood |
| KW (D-20-9-11 B) | `post_swap_window_open(pid)` | +9 | +6 | — | pass |

Ceilings requested under R (×2 measured, D02 A, STOP above): **kernel ≤ 26873 physical / 21084 tokei**, standalone from 26841 / 21042.
With KW: ≤ 26891 / 21096. L2, kernel last.

## ADR-069 room and the cumulative STOP

Expected cumulative measured tokei: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = **21203**. The shared internal STOP remains **21209** for everything else; D-20-9-15 A sets **21230 for 20-9 L2**, covering R's ×2 ceiling of 21224. D-20-14-12 C (ask `d2014_12`, 2026-11-03) sets **21250 for the combined 20-9 L2 + 20-14 L2 cumulative**. ADR-069's **22000** alarm is unchanged. Every later kernel lander re-bases against the exact landed pins.

## Landing and accounting (if granted)

L1 lands every kernel-free AC (hooks, WIT 2.4.0, swap, upgrade, cold swap, schedules carriage) with no kernel line. L2 lands
AC7 and AC8 under the ruled rows last. It re-cuts on 20-11's `forget_spirit` hunk, which is in the same file and distinct. It
re-measures the formatted code and pins `xtask/kernel-core-baseline.toml` (`src_lines` `:505`, the
`scheduler/schedule_watchdog.rs` hash `:1005`, and `hot_swap/coordinator.rs` `:958` under KW, set hash `:939`, EOF HISTORY)
and `xtask/kloc.toml:264` exactly. Each HISTORY row cites this key with `->` (D11 A). Unused room is not reserved. A measured
figure above the ceiling, or a re-based cumulative above the ruled STOP, is STOP plus a new D-row. Any
`crates/maos-kernel-core/src` change outside `schedule_watchdog.rs` (and `coordinator.rs` under KW) is STOP.

## Landing 1 (kernel-free) — 2026-11-11

L1 (AC1–AC5: WIT 2.4.0 lifecycle world, spawned triggers, `[[schedule]]` at admission, spawned hot swap/upgrade, door cold swap) is implemented at `fc7373f0`, uncommitted, awaiting review. **No kernel line was written.** `check-kernel-baseline` exits 0 against the unchanged pins (kernel-core 26841 physical / 21042 tokei, `kloc-check` per-crate 21042); `crates/maos-kernel-core/src` is untouched. The ruled row R (D-20-9-13 A + D-20-9-14 A: `poll_once` observer and D07 A suppression) is **not** landed: AC7 and AC8 stay with L2, and the interim state (the FW-10 untokened fire still proceeds) is labelled in `docs-site/docs/cookbook/lifecycle-hooks.md`. The kernel grant, the shared 21250 STOP and the L2 ceilings are unchanged; L2 re-measures and re-pins exactly.

L1 kloc (exact after `cargo fmt --all`, `kloc-check --json`): maos-bin 36234 → 37350 (+1116; re-based ceiling 38422), maos-wasm-host 2538 → 3103 (+565; 3348), xtask 44935 → 44936 (+1; 44980), aggregate 199198 → 200880 (hardfail 200881; ceiling 202209). Domain 10203, iac 8770 and the kernel are unchanged.

## Landing 2 (kernel, AC7/AC8) — 2026-11-12

Implemented by `Dev209K` (anthropic/claude-sonnet-5-5) on the live base `e1a44cd5` (kernel 27,044 physical / 21,202 tokei), uncommitted, awaiting hosted run and review. Operator rulings of 2026-11-12 (asks relayed by Main): row R as priced; options (a) with the typed `NotLoaded` refusal (a1), (b) atomic archive record, (c) idle-watchdog parallel fire, and the kernel-free bin halves of (a) and (b), all within the shared STOP 22,000; AC7.2 "no sleep" accepted as a bounded `yield_now` retry loop asserting only on the outcome sequence; AC9.1 (`docs/invariants/I14.md`) skipped, dispositioned as a deferred row (ADR-037).

**Landed (exact formatted, `check-kernel-baseline` / `kloc-check --json`):** kernel 27,044 -> **27,103 physical (+59)**, 21,202 -> **21,256 tokei (+54)**, 95 files; priced +51 / +47 (R +16 / +21, a1 +12 / +10, b +14 / +11, c +9 / +5). The +8 / +7 over the price is the revoke-failure diagnostic of a suppressed fire (an `eprintln!` when revoking a minted token fails: a silent failure would leave a live token to its TTL). **Surface gate: flags nothing** (`check-service-boundary` passed, all 302 item hashes identical to the base; `poll_once` is an inherent method). Nothing in the kernel outside the four granted files (`scheduler/schedule_watchdog.rs`, `hot_swap/coordinator.rs`, `hot_swap/archive.rs`, `scheduler/idle_watchdog.rs`). Cumulative measured tokei 21,256 (shared STOP 22,000, 744 below).

**Prototype gap found at re-cut, now priced in the landing:** the FR4 inventory doorbell (`crates/maos-audit/tests/fr4_classifier_16_2.rs`, roots `maos-kernel-core/src`, `maos-bin/src`, `maos-iac/src`) keys writer sites by `(file, enclosing fn, ordinal)`. R renames the `check_and_fire` site to `poll_once` and adds a tokenless kind-7 writer in maos-iac, so `crates/maos-audit/src/fr4_classifier.rs` gained one `WriterShapeEntry` (and the renamed site string): maos-audit 8,308 -> 8,320 (+12). `refused_scope` is therefore an `Option<String>` (the scope's `Debug` text, a `StrOrNull` payload type) rather than an `Option<Scope>` object.

**Orchestrator-adjudicated overdraft, disclosed to the operator** (the ruled L2 grant named every crate other than the kernel and maos-iac at 0): maos-audit +12 (8,320), maos-bin +38 (38,564 -> 38,602), maos-wasm-host +8 (3,141 -> 3,149, the `snapshot-then-exit` mode of the test twin), maos-iac +44 (8,831 -> 8,875; priced +39, the +5 over is the row-serialize failure diagnostic). Unchanged: maos-domain 10,274, maos-capability 1,454, maos-cli 6,612, xtask 45,355. Aggregate 202,979 -> 203,134 measured (`_aggregate_hardfail` 203,135).

Pins re-fixed exactly: `xtask/kernel-core-baseline.toml` (`src_lines` at line 505, set hash, four per-file digests, HISTORY block citing this key with `->`), `xtask/kloc.toml` (kernel, iac, audit, bin, wasm-host rows and `_aggregate_hardfail`, each HISTORY segment first citing `20-9-spawned-form-lifecycle-hooks ->`), `xtask/tests/kernel_pin_content_hash_16_0.rs`, `xtask/tests/recovery_lane_ceiling_rule.rs`. Evidence: `20-9-spawned-form-lifecycle-hooks-evidence/landing-2/`.
