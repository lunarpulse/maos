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

## Landing 2 — final kernel landing, measured vs granted (Dev2011k, 2026-11-11, base `345ae30f`)

**Authority:** the grant above (asks `s2011`, `halt_grant`, `halt_landing`) binds at this landing after re-base. KH/KF were not
requested and are not landed. T0 evidence: `20-11-erasure-inventory-closure-evidence/landing-2/t0-measurement.md` + raw JSON.

| | Base (HEAD) | Prototype (T0, reverted `git archive HEAD` copy) | Landed (in tree, formatted) | Grant / ceiling |
|---|---:|---:|---:|---|
| Kernel physical | 26,854 | 26,880 (+26) | **26,880** | measured +26; ×2 ceiling 26,893 |
| Kernel tokei | 21,051 | 21,069 (+18) | **21,069** | measured +18; ×2 ceiling 21,078; shared STOP 21,250; ADR-069 alarm 22,000 |
| maos-domain | 10,211 | 10,215 (+4) | **10,215** | measured +4; ×2 allowance +8 |
| Kernel file set | 95 files, `2c223219…` | `affee8b0…` | **`affee8b007d65000b63a3776cf59bd5d3fafb874cc818bc68133bbf3ef41a883`** | none added/removed |

The measurement equals the grant exactly (KS +10/+6 and O4 +16/+12 became +26/+18 together); nothing exceeded a ceiling, so no STOP
was needed for the kernel rows. Changed kernel files and new per-file hashes: `halt/mod.rs` `b77703f8…`, `halt/termination.rs`
`043a2a54…`, `scheduler/schedule_watchdog.rs` `3b9d8431…`, `scheduler/scheduler_loop.rs` `95bd5a33…`.

**Single public-surface change (explicit note, ADR-066 hand-edit convention).** `docs/ci-baselines/kernel-surface-v0.1-beta.json:618`
`maos_kernel_core::halt::HaltRegistry` `b6d6524333a6327adc96c8a3ef78e2a10134ea7e2a9c534d930d6a248eda4af8` →
`a917eddcb762e31e151adbbb2dad84de0ce445df29bfd8abbddd9a4d3b19f910`. Cause: the private `closed` field (`check-service-boundary`
hashes structs in full). It is the ONLY surface edit; no new symbol, no `kernel-api-classes.toml` row, `invoke_halt`/`terminate_spirit`
signature hashes unchanged, `ScheduleWatchdog::forget_spirit` does not move the struct-only hash. `check-service-boundary` passes.

**Shape landed (O4 as ruled, no deviation).** `terminate_spirit` closes the pid in `HaltRegistry.closed` before its drain; `invoke_halt`
holds that lock across TL row → Journal → registry insert and returns `InvokeHaltError::SpiritUnloading(pid)` (new `maos-domain`
variant) for a closed pid, writing nothing; the silent drain at `scheduler_loop.rs:573-574` is deleted. A halt in flight when the close
runs is committed first and receipted by the drain; a later one is refused. Crash and revocation drain through the same writer, so they
are fenced too. Hot swap (K1p carry-not-drain) never calls the writer and is unchanged. No kernel panic/unwrap added; I2 fail-stop untouched.

**Non-kernel overdrafts (orchestrator-adjudicated, disclosed to the operator, D02 A, 2026-11-11).**
- **maos-audit:** 20-11's absolute ceiling **8,261 → 8,313**; pin **8,227 → 8,313** (+86 measured: `orphaned_halts`,
  `planned_unload_receipts` — the two read-only TL queries — and the typed `halt` removal evidence in `proof.rs`; genuine production code,
  relocation to maos-bin would only move the overdraft). Knock-on: 20-1b's "remaining 82" is moot; later stories re-base on 8,313.
- **maos-bin:** pin **38,061 → 38,202** (+141). The story's bin ceilings (34,698 creation / 35,704 L1) are stale-absolute against
  38,061 (other stories' grants); the live bin ceiling is the measured pin plus this story's own remaining allowance (35,704 − 35,102 = 602;
  this landing uses 141).
- Aggregate hardfail **202,122 → 202,371** (clean aggregate 202,370 on a `git archive HEAD` copy overlaid with the landing).

**Witnesses and plants** (details: story Dev Agent Record and `landing-2/plants.md`): the T0 oracle is re-stated (typed refusal, no
durable row, no journal entry, empty registry, only the unload's `term-…` receipt); S2 (in-flight halt receipted, the next refused); S4
(crash path fenced). Plant (a) (no close in `terminate_spirit`) reds all three; plant (b) (release before the TL write) reds S2. Plant
(c) "close in `unload` only" is an O1 shape that needs a `pub(crate)` close method this repair deliberately does not have; no realistic
variant exists in the landed design, and plant (a) already reds the crash witness S4.

## Landing 2 review corrections — kernel P1 + F3 (Dev2011k, 2026-11-11, base `c402c29b`)

**Authority:** operator ask `s2` = allow (relayed by Main, 2026-11-11): P1 + F3 at the priced +7/+6, kernel 26,887/21,075, inside the already-granted
KS + O4 binding D02 A x2 ceiling **26,893 / 21,078**; one second `ScheduleWatchdog` surface-hash hand-refresh (ADR-066) allowed with an explicit note.

| | Landed L2 | T0 prototype (reverted `git archive HEAD` copy, `kernel-p1-f3-prototype.diff`) | Landed corrections (formatted) | Ceiling |
|---|---:|---:|---:|---|
| Kernel physical | 26,880 | 26,887 (+7) | **26,891 (+11)** | 26,893 |
| Kernel tokei | 21,069 | 21,075 (+6) | **21,075 (+6)** | 21,078; shared STOP 21,250; ADR-069 alarm 22,000 |

The tokei equals the priced figure (P1 +7, F3 -1). Physical is +4 over the prototype only because the landed `forget_spirit` carries four doc-comment
lines; still 2 under the physical ceiling. Set hash `f76a9937799ec53e59491efe427eddfa20932262d83ea3897431a25c72a6d8ba`; changed per-file hashes
`halt/mod.rs` `9f981811…`, `halt/termination.rs` `3364cd49…`, `scheduler/schedule_watchdog.rs` `e16a890a…`; `scheduler_loop.rs` unchanged.

**Shape.** P1: `ScheduleWatchdog.fence` (`tokio::sync::Mutex<()>`) is held by every queued candidate from admission to its stamp, with a `Running`
re-check under it (an unloaded Spirit's stale candidate is skipped before any bucket, fire row, stamp or dispatch); `forget_spirit` is `async` and
takes the same fence (an admitted firing finishes before anything is cleared or reported). F3: `terminate_spirit` marker rows use
`insert_kernel_event_returning_id` (same Kernel origin); `invoke_halt` uses `mint_frame_id()` + `insert_frame_event_with_id(Some(id), …, SpiritAuto)`
(origin unchanged), so a receipt always cites its own frame.

**Second and last surface edit (explicit note, ADR-066 hand-edit).** `docs/ci-baselines/kernel-surface-v0.1-beta.json` `ScheduleWatchdog` struct
`2feabbd714c29206b1fafe72b4611a98a8a02a001ec0be1f74a04c5c55430f02` -> `5e58c7d5ef9f066eb5976dacbad215e221d80aa88a29f1d0f0448d36ee35c55d`
(private `fence` field). `check-service-boundary` flagged only that symbol; with the edit it passes. Together with the L2 `HaltRegistry` refresh
that is two surface hash edits in total for 20-11; no new symbol, no class row.

**Non-kernel (all inside the story's envelope except as noted).** maos-audit 8,313 -> **8,286** (the bounded orphan scan moved to maos-bin; old SQL
version deleted); maos-bin 38,202 -> **38,335** (+133, inside the story's own bin allowance); maos-domain 10,215 -> **10,220** (+5, F6 cap, one over
the story's remaining +4: disclosed); maos-wasm-host 3,143 -> 3,141; aggregate hardfail 202,371 -> **202,486** (clean aggregate 202,485).

**Witnesses and plants:** `landing-2/review-corrections/plants.md`.
