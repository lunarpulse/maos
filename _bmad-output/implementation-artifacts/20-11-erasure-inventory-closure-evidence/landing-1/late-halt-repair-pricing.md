# 20-11 landing 1 — late-halt repair pricing (D-20-11-11, PROPOSED)

Date: 2026-11-04. Agent: Price2011Halt (`anthropic/claude-opus-5-5`). HEAD `73e7afbcae81f4fb4858e9d52ded3d592bed636d`.
Pricing only: nothing here is granted or implemented. No crate source, pin, surface baseline or tracker changed in
the repository.

**Subsequent operator ruling — 2026-11-04:** ask `halt_grant` grants **D-20-11-11 A / O4**, measured
kernel +16 physical / +12 tokei and maos-domain +4. With KS, measured **26867 / 21060**; binding D02 A ×2
ceiling **26893 / 21078**, domain ceiling **10115**. Binding only at final kernel landing after re-measure/re-base.
Shared measured STOP **21250 for the whole ledger** supersedes 21209 and the 20-9-only 21230; the former
joint-21250 exception now equals the shared STOP. Cumulative **21203 + 12 = 21215** tokei / **27048** physical;
ADR-069 alarm 22000 / envelope 23000 unchanged. Ask `halt_landing` moves AC3 to final kernel KS + O4;
L1 AC1, AC2, AC4–AC7 stays kernel-free. The original request, option comparisons and measurements below remain
historical evidence; this note does not claim the prototypes compiled or any repair landed.

## Instrument

No cargo ran. The prebuilt gate binary `/mnt/build/cargo-maos/debug/xtask` (inherited
`CARGO_TARGET_DIR=/mnt/build/cargo-maos`; `/mnt/build/cargo` is a 105-byte file, not a target dir) was run as
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto2011halt/<copy>/xtask <bin> {check-kernel-baseline,kloc-check,check-service-boundary} --json`
from each `git archive HEAD` copy's root under `/tmp/proto2011halt/` (deleted after measurement). Touched files
only were formatted with `rustfmt 1.9.0 --edition 2021`; `diff -rq` confirms no other file changed in any copy.
Prototypes were never compiled ([INFERENCE] that they build). Every `late-halt-*.diff` here passes
`git apply --check` against the working tree. Raw figures: [late-halt-repair-measurements.json](late-halt-repair-measurements.json).

Baseline copy reproduces the pins: **26841** physical (95 files, set hash `db39bc4a…`), kernel **21042** tokei,
maos-domain **10107**, aggregate **193140**, `check-service-boundary` passed, 0 violations.

## Options measured

| # | Shape | Diff | Kernel physical | Kernel tokei | maos-domain | Surface (`check-service-boundary`) |
|---|---|---|---:|---:|---:|---|
| O1 | Fence in `invoke_halt`: `HaltRegistry.closed` lock held across TL row → Journal → registry insert; typed `InvokeHaltError::SpiritUnloading(pid)` before any write; `unload` closes the pid before `terminate_spirit`; `:574` silent drain kept | [o1](late-halt-o1-invoke-fence.diff) | +23 (26864) | +15 (21057) | +4 | `HaltRegistry` hash refresh; passes after it |
| O2 | Receipted final drain: `terminate_spirit`'s per-halt arm becomes `pub(crate) receipt_drained`; `:574` routes its drain through it | [o2](late-halt-o2-receipted-final-drain.diff) | +23 (26864) | +19 (21061) | 0 | unchanged, passes |
| O3 | O1 + O2 | [o3](late-halt-o3-fence-plus-receipted-drain.diff) | +46 (26887) | +34 (21076) | +4 | `HaltRegistry` hash refresh; passes after it |
| O4 | Fence inside the single receipt writer: `terminate_spirit` closes the pid (child module reads the private field) before its drain; `invoke_halt` check as O1; `:573-574` silent drain **deleted** | [o4](late-halt-o4-fence-in-single-writer.diff) | **+16 (26857)** | **+12 (21054)** | +4 | `HaltRegistry` hash refresh; passes after it |

Sub-variants measured (not separate D-row options):

| Variant | Diff | Physical | Tokei | maos-domain | Note |
|---|---|---:|---:|---:|---|
| O4m — O4 with a `close_for_spirit` method | [o4m](late-halt-o4m-fence-in-single-writer-method.diff) | +21 | +14 | +4 | the method is only needed when a non-`halt` module closes (O1/O3) |
| O4r — O4m with untyped `RegistryInsertFailed(format!(…))` | [o4r](late-halt-o4r-untyped-refusal.diff) | +23 | +16 | 0 | typed variant is cheaper in kernel (+2 tokei) and caller-matchable |
| O2-loop — drain-until-empty under the `spirits` lock (assignment's example) | [o2l](late-halt-o2l-drain-until-empty-loop.diff) | +28 | +24 | 0 | rejected, see below |

O1/O3 carry the 10-line method because `scheduler` is not a descendant of `halt`; a `pub(crate)` field would
save about 5 lines ([INFERENCE], not measured).

## Surface impact

- `xtask/src/check_service_boundary.rs:571-578` hashes a struct in full, private fields included; adding `closed`
  changes `maos_kernel_core::halt::HaltRegistry` from `b6d65243…4eda4af8` to `a917eddc…3b19f910` (both full hashes
  in the JSON). Unrefreshed, the gate reports a removed symbol (`:220-231`) plus an unclassified new one.
  O1/O3/O4 diffs include a **single in-place hash refresh** in `docs/ci-baselines/kernel-surface-v0.1-beta.json:618`,
  the ADR-066 hand-edit convention (`ADR-066…md:65-68`). With the refresh, the gate passed on the O1, O3, O4m and O4 copies.
- No new public kernel symbol, so no `xtask/kernel-api-classes.toml` row. `invoke_halt` (`66e0273f…`) and
  `terminate_spirit` (`929eef33…`) signature hashes do not change. O2's helper is `pub(crate)`, so O2 has no surface change.
- `maos-domain` `InvokeHaltError` (`crates/maos-domain/src/halt.rs:330-340`, not `#[non_exhaustive]`) gains
  `SpiritUnloading(u32)`. No exhaustive match exists outside its own display test; the existing
  `matches!(…)` sites remain valid (`crates/maos-kernel-core/tests/halt_invoke_test.rs:121,545`).
- Kernel pin: `check-kernel-baseline` drifts (changed files only: `halt/mod.rs`, `halt/termination.rs`,
  `scheduler/scheduler_loop.rs`; none added or removed). The exact re-pin and HISTORY row are owed at landing.

## Receipt behaviour

| | Late `invoke_halt` after the SCB is `Unloaded` | Receipt rows for it | Caller sees |
|---|---|---|---|
| HEAD | Durable TL + Journal + pending, then silently drained at `scheduler_loop.rs:574` | none (only the synthetic `term-…` pair exists) | `Ok(receipt)`, then the halt silently disappears |
| O1/O3/O4 | Refused before step 1 (`halt/mod.rs:542-555` TL write); no TL row, no Journal entry, no pending | none needed (no invocation row) | `Err(SpiritUnloading(pid))`; `shell_host.rs:145-153` and `working_memory/orchestrator.rs:85-93` propagate it with `?` |
| O2 | Durable invocation, then one `PlannedUnload` pair (empty marker plus `HaltReceipt` naming it), the same shape as `termination.rs:84-121` | +2 EpistemicHalt rows per late halt | `Ok(receipt)`, and the halt is receipted |

## Race-freedom

- **O1:** an in-flight invocation holds `closed` until its insert has returned. `unload` acquires it before
  `terminate_spirit`, so the first drain captures every `invoke_halt` halt for that pid, and later invocations are
  refused. Between the last drain and the SCB removal, the window no longer matters for `invoke_halt`.
  - **Residual:** a door resolve whose approval-journal write fails rolls back after the drain
    (`operator_door.rs:1725-1726` → `halt/mod.rs:272-293`). The rollback re-pends the halt with its metadata, and
    the kept `:574` drain still discards it silently. This is [INFERENCE] from reading the code, not witnessed.
  - Crash (`supervision/crash_detector.rs:131-138`) and revocation (`revocation/applier.rs:151-155,182-186`) never
    close. Their late halts stay pending, which is visible, though no receipt is written.
- **O2:** there is no fence. A halt invoked after the receipted drain stays pending for a removed pid: no receipt,
  but no silent loss either, and D-20-11-03 B's post-check can detect it. The window stays open from `:574` onward:
  `invoke_halt` never consults the SCB or the map. The fully deterministic seam is `on_unload` at
  `scheduler_loop.rs:580`, which runs after the SCB removal.
- **O2-loop:** holding the `spirits` write lock does not fence `invoke_halt`, which takes no SCB or map lock.
  After the removal, an invocation is still accepted. It costs O2 plus 5 physical lines for the same window, so it is rejected.
- **O3:** O1's fence, plus the rollback case receipted if it happens before `:574`. After `:574` it stays pending
  (visible). Crash and revocation are not closed.
- **O4:** the fence sits where every terminator drains: unload via `terminate_spirit` (`scheduler_loop.rs:562-569`),
  crash and revocation. No drain in `unload` lacks a receipt any more. The rollback case becomes a visible pending
  halt instead of a silent discard. No window remains for `invoke_halt` on any terminator.
  - Lock order is `closed` → TL `inner` (`transparency_log.rs:1271-1274`) → registry maps. `terminate_spirit` holds
    `closed` for one `insert` only, so no cycle exists.
  - Cost: halt invocations serialize process-wide for one TL plus Journal write. A std `Mutex` taken inside async
    `unload` can block one executor thread for one in-flight invocation.
  - Pids are never reused within a process (`scheduler_loop.rs:29-33`, `AtomicU32` from 1), so the set is never
    cleared. Growth is 4 bytes per terminated pid.

## Sibling halt receipts

- **Hot swap (20-3c K1p carried set):** the carried set (`halt/mod.rs:445-451`, read at
  `hot_swap/coordinator.rs:384-389` and `hot_swap/precheck.rs:52`) keeps the same pid. The swap and its auto-revert
  never call `unload` or `terminate_spirit`, so under every option the fence is never set and the carried set, the
  post-swap loss monitor and the saga stay unchanged. Behaviour if an unload races a hot swap is also unchanged:
  the door's `upgrade_lock` (`operator_door.rs:1315-1318`) serializes upgrades, not unloads.
- **Cold swap:** the door unloads the predecessor (`operator_door.rs:1436`). Its pending halts get `PlannedUnload`
  receipts under every option, the same as today. A late predecessor halt is receipted under O2 and refused with
  a typed error under O1/O3/O4. The successor and the restore are admitted at a new pid, so the fence cannot
  affect them.
  - Observation, not changed: the cold-swap TL row writes the literal `"halt_receipts_produced": 0`
    (`operator_door.rs:1480`) under every option.
- **Revocation `TerminateImmediately`:** the flow is `terminate_spirit(RevocationTerminated)`, then `unload`
  (`applier.rs:151-155`). Under O4 the first call closes the pid, and `unload`'s call then writes the same synthetic
  `term-…` pair as today.
- **20-11 AC3.2 offline closure:** `terminate_spirit` runs on a transient registry populated with
  `insert_pending_with_metadata` (D-20-11-03), which is not fenced, so behaviour is unchanged.
- **Test callers:** none of `terminate_spirit`'s test callers invokes a halt afterwards on the same pid and
  registry. Checked: `shell_host_16_2.rs:262-306`, `halt_receipt_production_rate.rs:45-87` (raw inserts),
  `maos-bench/tests/t_11_2b_halt_presence.rs:91`. The landing must re-check this with a grep.

## Witnesses and plants

- **Failing-before (existing T0):** [t0-unload-race-witness.rs.txt](t0-unload-race-witness.rs.txt), observed red at HEAD at
  `:187`, cargo exit 101 ([log:589](t0-unload-race-cargo-test.log#L589)).
  - **O2** turns it green **unmodified**: `late-20-11` is receipted by the receipted `:574` drain.
  - **O1/O3/O4** need its oracle re-stated. At `:144-157`, `invoke_halt` must return `Err(SpiritUnloading(pid))`,
    with no pending metadata. At `:177-190`, there must be no `ambiguity` TL row for the pid, no Journal `Halt`
    for `race-20-11`, an empty registry, the SCB removed, and `receipt_ids` equal to the one `term-…` id.
  - At HEAD the re-stated witness reds at the refusal assertion, because the invocation succeeds, so it stays a
    real failing-before. Choosing an option therefore also chooses which oracle the T0 witness carries.
- **Seams:**
  - **S1** (existing): the TL redaction-policy pause before the TL lock (`transparency_log.rs:1270-1274`), on the
    `planned_unload` receipt, after the drain. This is T0.
  - **S2** (fence options): pause on the late halt's own invocation row. Unload must then wait at `closed`. Resume,
    and the first drain receipts the halt. At HEAD the halt is inserted after unload and stays unreceipted.
  - **S3:** a test Spirit's `on_unload` (`scheduler_loop.rs:580`) invoking a halt after the SCB removal. This is
    O2's open window; fence options refuse it. [INFERENCE] the hook can reach captured `Arc`s.
  - **S4:** `CrashDetector::handle_crash` with an S1-style pause on `unplanned_crash`. Only O4 refuses that late halt.
- **Plants:**
  - **O4:** (a) delete the `closed` insert in `terminate_spirit` → re-stated T0 reds. (b) `drop(closed)`
    immediately after the check in `invoke_halt` → S2 reds. (c) close in `unload` only → S4 reds.
  - **O1:** (a) and (b) with the close in `unload`.
  - **O2:** restore `let _ = drain_for_spirit` → T0 reds at `:187`. This is the observed HEAD failure.
  - **O3:** O1's plants, plus a rollback-path witness for the receipted drain. No seam that fails the door's
    approval-journal write was found at HEAD, so that half has a test-seam gap.

## Kernel forecast and ledger (ruled grants not yet landed)

Cumulative tokei before this repair is **21203**: 21042 + 68 + 9 + 6 + 33 + 25 − 1 + 21. Physical is **27032**:
26841 + 78 + 13 + 10 + 38 + 35 + 1 + 16. Both are measured, not ceiling, figures. The ADR-069 envelope (≤ 23000)
and alarm (22000) apply to the `kloc.toml` tokei row (`ADR-069…md:10,36,42`).

| Option | Tokei Δ | Cumulative tokei | Internal STOP 21209 | 20-9 single 21230 | Joint 21250 | Alarm 22000 headroom | Physical cumulative | ×2 ceiling (D02 A) |
|---|---:|---:|---|---|---|---:|---:|---|
| O4 | +12 | **21215** | **crossed by 6** | under | under | 785 | 27048 | +32 / +24 |
| O1 | +15 | 21218 | crossed by 9 | under | under | 782 | 27055 | +46 / +30 |
| O2 | +19 | 21222 | crossed by 13 | under | under | 778 | 27055 | +46 / +38 |
| O3 | +34 | 21237 | crossed by 28 | **crossed by 7** | under | 763 | 27078 | +92 / +68 |

**Operator question (not ruled):** every option crosses the internal cumulative STOP 21209, so the repair needs an
explicit grant that raises it (for example to 21215 for O4) or reorders/defers another kernel grant.

- 20-11's own ceiling (KS only, 26861 / 21054) would also need a re-grant. Repair plus KS, measured on the current
  pin, gives O4 26867 / 21060.
- maos-domain has zero headroom (10107): O1/O3/O4 need a +4 grant, or O4r spends +4 kernel tokei and +7 physical
  lines instead of +4 domain.
- Aggregate stays under the frontmatter's 194058 for every option.
- No `ENVELOPE ALARM` is triggered.

## Recommendation

**O4** (D-20-11-11 **A**; O1 = B, O2 = C, O3 = D). It is the cheapest race-free shape: +16 / +12 kernel and +4 domain. It keeps `terminate_spirit` as the
single receipt writer and adds no second writer. It closes the window for unload, crash and revocation at one site,
writes no late durable row, and deletes the only silent drain. Hot swap is untouched.

Costs of O4: one in-place surface hash refresh, a re-stated T0 oracle, and a typed refusal that callers propagate.
O2 is the only option whose T0 witness turns green unmodified. It leaves an open window from `:574` and costs more.

## Cites re-measured at HEAD (OLD → NEW)

| Where | OLD | NEW |
|---|---|---|
| Assignment brief | `crates/maos-kernel-core/src/scheduler_loop.rs:574` | `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:574` (line correct; path stale) |
| Assignment brief | `termination.rs:39` | `crates/maos-kernel-core/src/halt/termination.rs:39` (correct) |
| t0-stop.md:22 | `halt/termination.rs:26-122` | `:26-123` (closing brace) |
| t0-stop.md:24 | `halt/mod.rs:347-381` | `:347-382` (closing brace) |
| Epic-20 `epic-20-ship-it-w5.md:181` (owed; not edited, outside this assignment's file list) | swap halt drain `halt/mod.rs:465-474` | `validate_swap_halt_continuity` is `halt/mod.rs:463-483`, carry-not-drain since 20-3c |

Verified unchanged: `scheduler_loop.rs:531-593,542-545,562-569,577`; `halt/mod.rs:523-585`;
`shell_host.rs:131-153`; `transparency_log.rs:1269-1275`; `deferred-work.md:1396`;
`20-11-flag-winston-decision.md:21-23`.

## Cleanup

`/tmp/proto2011halt` was deleted after measurement. No cargo or rustc process ran, no target dir was created, and
no repository source was touched.
