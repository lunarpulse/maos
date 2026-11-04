# 20-11 FLAG-Winston decision record

**Recorded:** 2026-11-04 by Dev2011 at `73e7afbcae81f4fb4858e9d52ded3d592bed636d`.
**Authority:** operator asks `s2011` (2026-11-02), `halt_grant` and `halt_landing` (2026-11-04), recorded in
[the story](20-11-erasure-inventory-closure.md). **GRANTED**, binding only at the final kernel landing after
re-measurement/re-base; no implementation or pin movement is claimed by this record.

## Granted scope — KS + O4, final kernel landing last

- KS: `ScheduleWatchdog::forget_spirit`, measured **+10 physical / +6 tokei**.
- O4 (D-20-11-11 **A**, ask `halt_grant`): measured **+16 physical / +12 tokei kernel**, **+4 maos-domain**.
  `terminate_spirit` closes the pid through private `HaltRegistry.closed` before draining. `invoke_halt` holds
  that lock across its TL, Journal and registry writes and returns typed `SpiritUnloading(pid)` before writing
  anything for a closed pid. Delete the silent drain at `scheduler/scheduler_loop.rs:573-574`.
- From base **26841 / 21042**, combined measured KS + O4 = **26867 / 21060**. Binding D02 A ×2 ceiling:
  **26893 physical / 21078 tokei** = base + KS allowance **20/12** + O4 allowance **32/24**.
  Domain pin **10107**, measured **10111**, ×2 ceiling **10115**. These figures re-base onto then-landed pins.
- Exactly one HaltRegistry surface-hash refresh at `docs/ci-baselines/kernel-surface-v0.1-beta.json:618`,
  `b6d65243…4eda4af8` → `a917eddc…3b19f910`, under ADR-066's hand-edit convention; no new symbol/class row.
- Ask `halt_grant` raises the **whole-ledger cumulative measured STOP to 21250 at each landing**. Former shared
  21209 and 20-9-only 21230 are superseded; D-20-14-12 C's joint 21250 now simply equals the shared STOP.
  Cumulative measured tokei **21203 + 12 = 21215**, physical **27032 + 16 = 27048**.
  ADR-069 alarm **22000** / envelope **23000** unchanged.
- Ask `halt_landing`: **L1 AC1, AC2, AC4–AC7 kernel-free**; **final kernel KS + O4 carries AC3** (door/offline
  halt count and receipt category). Pending halt and watchdog limitations remain explicit in L1.
  AC8 records follow each landing; no story closure until the final kernel witnesses pass. KH/KF not granted.

No kernel source, domain variant, surface hash or pin is changed in this docs-only recording pass.

## T0 STOP history — subsequently resolved by the grant and landing amendment

T0 measurement 2 reproduced the suspected loss. The real `SpiritSchedulerAdapter::unload` transitions the SCB
to `Unloaded` (`scheduler_loop.rs:542-545`), calls the single receipt writer (`:562-569`), then silently drains
again (`:574`). The writer first drains at `halt/termination.rs:39`; real `invoke_halt` has no lifecycle fence
(`halt/mod.rs:523-585`). A concurrent invocation between those drains succeeds and commits its durable row;
the second drain empties metadata without producing that halt's terminal receipt.

[Witness source](20-11-erasure-inventory-closure-evidence/landing-1/t0-unload-race-witness.rs.txt),
[complete failing-before output](20-11-erasure-inventory-closure-evidence/landing-1/t0-unload-race-cargo-test.log),
and [STOP report](20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md).

**Historical request, superseded 2026-11-04:** the initial T0 STOP required a separately measured kernel repair
and amended landing dependency. The operator selected O4 (`halt_grant`) and moved AC3 to the final kernel
landing (`halt_landing`). The observed red remains evidence; an empty registry alone still cannot prove AC3
until the granted repair lands and its witnesses pass. Story/tracker remain `ready-for-dev`.

## Late-halt repair pricing (historical request; O4 now granted as above)

Priced 2026-11-04 at `73e7afbc` by Price2011Halt. Full method, race analysis and plants are in
[late-halt-repair-pricing.md](20-11-erasure-inventory-closure-evidence/landing-1/late-halt-repair-pricing.md).
The figures were measured on `git archive` copies with the prebuilt gate binary (`check-kernel-baseline`,
`kloc-check`, `check-service-boundary`) and are recorded in
[raw JSON](20-11-erasure-inventory-closure-evidence/landing-1/late-halt-repair-measurements.json). No cargo ran.
Each prototype diff is kept beside the pricing report and passes `git apply --check`. No source, pin or baseline
changed in the tree.

| Option | Shape | Kernel physical | Kernel tokei | maos-domain | Late halt after `Unloaded` | Window left |
|---|---|---:|---:|---:|---|---|
| O1 | Fence in `invoke_halt`; `unload` closes the pid first; `:574` silent drain kept | +23 (26864) | +15 (21057) | +4 | typed refusal, no durable row | door-rollback re-pend still silently drained at `:574`; crash and revocation unfenced |
| O2 | `:574` drain routed through `terminate_spirit`'s per-halt arm (`pub(crate) receipt_drained`) | +23 (26864) | +19 (21061) | 0 | receipted `PlannedUnload` | after `:574`: the halt stays pending for a removed pid (visible, unreceipted) |
| O3 | O1 + O2 | +46 (26887) | +34 (21076) | +4 | typed refusal | rollback after `:574` stays pending; crash and revocation unfenced |
| **O4** | Fence inside the single writer: `terminate_spirit` closes the pid before its drain; `invoke_halt` checks; `:573-574` deleted | **+16 (26857)** | **+12 (21054)** | +4 | typed refusal, no durable row | none for `invoke_halt` on unload, crash or revocation; rollback re-pend becomes visible-pending, not discarded |

Mechanism: a private `HaltRegistry.closed: Mutex<HashSet<u32>>` field. `invoke_halt` holds it across the TL row,
the Journal entry and the registry insert; the close waits out an in-flight invocation. Pids are never reused within
a process (`scheduler_loop.rs:29-33`).

Rejected: the drain-until-empty loop under the `spirits` lock (+28/+24). `invoke_halt` takes no SCB or map lock, so
the loop cannot close the window.

**Recommended: O4.** It keeps `terminate_spirit` as the single receipt writer and adds no second writer.

**Receipt and public-surface change.**

- O1/O3/O4: a late invocation returns
  `InvokeHaltError::SpiritUnloading(pid)` (new `maos-domain` variant; no exhaustive match outside its own test).
  No TL row or Journal entry is written for it, and no new receipt shape is added.
- O2: adds one `PlannedUnload` marker+receipt pair per late halt, the same shape as `halt/termination.rs:84-121`.
- Surface: `check-service-boundary` hashes structs in full, so the private field moves `maos_kernel_core::halt::HaltRegistry`
  `b6d65243…4eda4af8` → `a917eddc…3b19f910`. That needs one in-place hash refresh in
  `docs/ci-baselines/kernel-surface-v0.1-beta.json:618` (ADR-066 hand-edit convention). The gate passed on every
  fence-option copy after it. O2 changes no surface.
- No new public symbol and no `xtask/kernel-api-classes.toml` row. The `invoke_halt` and `terminate_spirit` signature
  hashes are unchanged.

**Siblings.**

- Hot swap: the 20-3c K1p carried set (`halt/mod.rs:445-451`, `hot_swap/coordinator.rs:384-389`) keeps the pid
  and never reaches `unload` or `terminate_spirit`. It is unchanged under every option.
- Cold swap: the predecessor unload is at `operator_door.rs:1436`. A late predecessor halt is receipted (O2) or
  refused (O1/O3/O4). The successor and the restore get new pids.
- Revocation: under O4 its first `terminate_spirit` closes the pid, and `unload` then writes the usual `term-…` pair.

**Witness that must turn green:** the existing T0
[t0-unload-race-witness.rs.txt](20-11-erasure-inventory-closure-evidence/landing-1/t0-unload-race-witness.rs.txt), red at
HEAD at `:187`.

- O2 turns it green unmodified.
- O1/O3/O4 re-state its oracle. At `:144-157`, a typed refusal. At `:177-190`, no `ambiguity` row, no Journal
  `Halt`, an empty registry, the SCB removed, and only the `term-…` receipt.
- At HEAD the re-stated oracle reds because the invocation succeeds, so it is still a real failing-before.

**Plants (O4):**

- (a) Drop the close in `terminate_spirit` → the T0 witness reds.
- (b) Release `closed` before the TL write → seam S2 reds. S2 pauses the late invocation's own TL row while
  `unload` runs.
- (c) Close in `unload` only → seam S4 reds. S4 is the `handle_crash` race.

**Historical kernel ledger (before ask `halt_grant`, now superseded above).** Cumulative before the repair is
21203 tokei / 27032 physical, measured. With each priced option:

| Option | Tokei | Physical |
|---|---:|---:|
| O4 | 21215 | 27048 |
| O1 | 21218 | 27055 |
| O2 | 21222 | 27055 |
| O3 | 21237 | 27078 |

- Historically every option crossed shared STOP 21209; O4 crossed it by 6. O3 also crossed 20-9-only 21230.
- Ask `halt_grant` now sets shared measured STOP **21250 for the whole ledger**, not a targeted exception.
- Granted O4 gives cumulative **21215**, 35 below shared STOP and 785 below the unchanged ADR-069 alarm.
- Repair plus KS measured **26867 / 21060**; granted ×2 allowance yields binding ceiling **26893 / 21078**.
- O4 allowance **+32 / +24** kernel and domain **+8** (measured +4) is granted, binding after re-base at landing.
- Ask `halt_landing` settles the dependency: AC3 and KS + O4 land together in the final kernel landing;
  L1 is kernel-free AC1, AC2, AC4–AC7. No cargo, tests or gates ran for this ruling-record update.
