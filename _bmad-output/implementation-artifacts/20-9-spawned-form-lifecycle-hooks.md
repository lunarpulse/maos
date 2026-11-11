---
story_id: '20.9'
story_key: 20-9-spawned-form-lifecycle-hooks
created: 2026-11-03
context_commit: 8c809f9c
baseline_commit: 8c809f9c
security_sensitive: true
kernel_grant: "Ruled 2026-11-03 (operator, ask d_batch4; binding at dev start after remeasurement/rebase; ◆ FLAG-Winston D-20-9-13 A + D-20-9-14 A = R; D-20-9-15 A): measured at 8c809f9c from 26841 physical / 21042 tokei, R +16 / +21 (KD07A-lean +8/+14, KD07B-lean +2/+8, KD07A-inline +27/+32, KO-poll +24/+27, KO-count +28/+30, KO-row +20/+26, KW +9/+6 priced). Ruled ceilings (STOP above; re-base at L2 dev start and pin exactly at landing): kernel ≤ 26873 physical / 21084 tokei (≤ 26891 / 21096 with KW under D-20-9-11 B); L2 kernel last. Expected cumulative with 20-1b +68, 20-4 +9, 20-11 +6, 20-8 KS +33, 20-14 KT +25, 20-13 KT3 −1 and R +21 is 21203 (6 below the 21209 STOP); R's ×2 ceiling gives 21224. D-20-9-15 A rules a targeted STOP of 21230 for 20-9 L2; D-20-14-12 C (ask d2014_12) then sets a JOINT targeted STOP of 21250 shared by 20-9 L2 + 20-14 L2 (combined cumulative must not exceed 21250; both stories' ×2 allowances: 21249). Shared STOP stays 21209 for everything else; ADR-069 alarm 22000 untouched."
kloc_grant: "Ruled 2026-11-03 (operator, ask d_batch4; D-20-9-16 A; D02 A ×2 measured / ×3 unmeasured); binding at dev start after remeasurement/rebase. Ruled ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): maos-bin ≤ 36379; maos-wasm-host ≤ 3222; maos-iac ≤ 8348 (L2); maos-domain ≤ 10117; xtask ≤ 44451; kernel ≤ 26873 / 21084 (see kernel_grant); aggregate ≤ 195700 (L1 ≤ 195580). Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): a kernel FLAG-Winston row (D-20-9-13/14), a WIT ABI bump with a new exported interface across the T2 boundary (AC1), token and working-memory preservation across a spawned swap (AC4), and successor admission inside the swap (AC4.1). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC2.3 (telemetry disclosure), AC4.1-4.3 (successor capability ⊆, tokens, retire path) and AC8 (suppression, revoke), Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.9: Spawned-form lifecycle hooks and state-preserving swap

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


Status: in-progress (L1 kernel-free implemented 2026-11-11 at `fc7373f0`, uncommitted, awaiting review; L2 kernel last, AC7/AC8/AC9 open)

<!-- Created 2026-11-03 at 8c809f9c (main; `git log -1 -- crates xtask` = 8c809f9c) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30); D07 A (2026-11-01, ask rows203c, "decided at 20-9"); D-19-5-10 C (2026-10-28, ask w3_19_5_d10, the received FR10 clause); R5 (2026-10-24, the two watchdog items); D-20-3c-01 A (I14 owner). D-20-9-01…16 were ruled 2026-11-03 by the operator (ask d_batch4), all as recommended A; D-20-14-12 C (ask d2014_12) sets the joint targeted STOP described below. Grants bind at dev start after remeasurement/rebase. No cargo ran (parent rule). Cites were re-read in a `git archive 8c809f9c` copy (`creation/cites-8c809f9c.md`), because the working tree carries Dev204L1's uncommitted 20-4 L1 edits. Kernel and kloc were measured by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`). Prototypes are saved as diffs and were never applied. Stale E20 §20-9 cites were corrected in place (`creation/records-owed.md` §A). Builds on 20-8 (ruled as recommended 2026-11-03, created the same day): WIT `maos:spirit@2.3.0`, manifest schema 7, `emit_manifest_declaration_event`, `SpiritManifestBundle.swap_invariants` / `HotSwapError::SwapInvariantDropped` (20-8 §Contract for 20-9). -->

**Status rationale — ready-for-dev, not blocked.** Every AC except AC6 is hermetic on the dev host. That includes the WASM
legs under `--features maos-bin/wasm-host` with the guest toolchain `build-test-fixtures` uses. AC6 is the hosted T2 leg on
existing jobs. No paid key, human, T3 runtime or external author is needed. The `[cli_wrapper]` Worker is not a Spirit form
(ADR-060 `:27-31`), so no T3 leg exists. Start gate (D-20-9-02 A): 20-8 L1 has landed (WIT 2.3.0, the declaration emitter).
L2 waits for 20-8 L2 and 20-11 L2, which share the kernel files.

> **The capability:** *a third-party WASM Spirit receives the same budgeted lifecycle triggers a compiled-in Spirit does: idle, consolidation, telemetry, schedule, pause/resume. The operator can hot-swap or upgrade it to a new version at the same pid. Its capability tokens, working memory and the state it chose to snapshot survive the swap, and every refusal is typed before anything is replaced. A scheduled fire whose declared side-effect authority is refused no longer runs untokened. A test can observe a refused poll without a clock.*

**Closes:**
- Epic-20 §20-9 (E20@HEAD:264-270) as amended 2026-10-30 (D01 A; PF §3.5) and its received items (E20:362-370). Epic AC1 maps to AC2 + AC3 + AC4 + AC5 + AC6; AC2 maps to AC1; AC3 maps to AC7 + AC8.
- FR55 (`prd/functional-requirements.md:69`) for spawned forms: the triggers reach a `wasm-component` Spirit under the manifest's declared budgets (17-4 closed compiled-in only, `17-4-hooks-observer-and-activity-corpus.md:53,341`).
- FR10's spawned-form clause received from 19-5 (D-19-5-10 C; S:309; RA:173): a version swap of a `wasm-component` Spirit preserving in-flight capability tokens and working-memory state, plus upgrade (`--policy migrator`) and cold swap.
- FR26 at admission (F4): `[[schedule]]` is carried for every admitted Spirit, not only for a hot-swap successor (D-20-9-08 A).
- `deferred-work.md:1260` (PF-04 refused-poll observer) and `:1275` (untokened fire, ruled D07 A), both owner 20-9.
- I14's FR53 owner row (`docs/invariants/I14.md:5,10,46-49`): the spawned swap inherits the coordinator's halt carry. The claim stays `asserted`.

**Does NOT close, and says so:**
- **The `[cli_wrapper]` Worker.** It is not a Spirit form. It keeps `409 cli_wrapper_is_not_a_spirit_form` (`operator_door.rs:1190-1199`) and its next-spawn semantics (ADR-060).
- **Mid-export state capture.** A WASM export cannot be interrupted to snapshot it. A swap waits for the turn boundary (D-20-9-10 A), and an active turn finishes on the predecessor under its own leases.
- **Auto-revert after the predecessor child is retired** (D-20-9-11 A). A post-swap revert of a spawned Spirit ends its session with a typed, journaled reason. Nothing restores the retired child.
- **A capability token handed to a guest.** The hook ABI carries no token (unchanged, `schedule_watchdog.rs:216-221`). D07 A changes only whether an unauthorized fire happens.
- **`maosctl load` / `start` of a WASM manifest over the door.** `prepare_spirit`'s `operator door` refusal (`main.rs:10212`) stays. Cold swap uses the spawned admission path internally (D-20-9-12 A); it is not a new operator verb.
- **20-8's structural swap-invariant check** (D-20-8-07). 20-9's swap passes through it unchanged and adds the state-preservation proof.
- **External-author cohort evidence** (operator lane). The mechanism is proven with internal keys and guests.

## Story

As **the operator of a v1.0 daemon running a third-party WASM/TypeScript Spirit, and that Spirit's author**,
I want **the kernel's lifecycle triggers to reach the spawned form under its declared budgets, and the Spirit to be swappable or upgradable in place with its tokens, working memory and snapshot state intact**,
so that **a spawned Spirit is a first-class Spirit: the hook ABI the milestone promises is real, an upgrade is not a restart, and the watchdog never fires without the authority it declared.**

**Contract:**
- Epic §20-9 (E20@HEAD:264-270) as amended 2026-10-30 (D01 A, PF §3.5, §2 row 20-9) and the received items E20:362-370 (R5, D07 A). Create-story may tighten the ACs but not add uncited ones. AC3 is the consumer that makes FR26's `[[schedule]]` true for spawned forms (rule 11; the 20-4 `[providers]` precedent).
- 20-8 §Contract for 20-9 (`20-8-manifest-contract-completion.md:176-197`). Points 1, 4, 5 and 7 are consumed here; see §Consumed from 20-8.
- Tracker `sprint-status.yaml:309`; rules 9–11 (E20:59-61); proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run plus one plant per AC) and `:33-71` (review, tiers). A WIT change and manifest-reader change widen the focused set (`:48-53`).

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `8c809f9c` in a `git archive` copy (`creation/cites-8c809f9c.md` C1–C11, F1–F15); no cargo.

| # | Charter item | Disposition | Code at HEAD | Witness / measurement | AC |
|---|---|---|---|---|---|
| 1 | Lifecycle exports for WASM | **absent**: `handle-frame`, `on-start`, `on-shutdown` only | `wit/spirit.wit:30,378-391`; runner `runner.rs:257-320` | `wit_corpus.rs` | AC1 |
| 2 | Kernel hook calls on a spawned SCB | **fired, then no-op**: `WorkerSpirit` implements only `on_unload` | `supervision.rs:2612-2633`; dispatcher `hook_dispatch.rs:271-357` | — | AC2 |
| 3 | Idle trigger | form-agnostic caller exists | `idle_watchdog.rs:65-90` | — | AC2 |
| 4 | Consolidation and telemetry callers | **exclude spawned pids** | `admission.rs:960-968`; `hook_pump.rs:305,1018-1050` | `hook_pump_17_4.rs` | AC2 |
| 5 | `[[schedule]]` at admission | **never carried** (any form); only a hot-swap successor's | `admission.rs:471-484,590-745` vs `upgrade.rs:388-400` | prototype `sched`: bin +14 | AC3 |
| 6 | Spawned hot-swap successor | **refused** `spawned_surface_unsupported: hot-swap successor` | `main.rs:3423-3432` | `wasm_form_admission_17_3b.rs:338` | AC4 |
| 7 | Pid, tokens, working memory across a swap | kept by the kernel (same pid, same SCB `Arc`) | `coordinator.rs:120-455` (`replace_runtime` `:301-319`) | `routed_halt_swap_20_3b.rs` | AC4 |
| 8 | Spawned token lifetime | turn leases revoked at TurnComplete; finalize revokes every pid token | `spirit_session.rs:445-447,523-527` | — | AC4 |
| 9 | Post-swap auto-revert | restores the predecessor object within 30 s / 300 ms | `coordinator.rs:510-571`; `post_swap_monitor.rs:39-58` | — | AC4 |
| 10 | Spawned cold swap | **refused before unload** `cold_swap_unrestorable` | `main.rs:10198-10212`; `operator_door.rs:1415-1436` | — | AC5 |
| 11 | Worker upgrade target | typed refusal | `operator_door.rs:1190-1199` | 19-5 AC6 | AC5 |
| 12 | Refused-poll observer | **none**: `check_and_fire` private; refused polls silent | `schedule_watchdog.rs:158-268,199-201,212-214` | `schedule_watchdog_fr26.rs:537-578` (labelled non-proof) | AC7 ◆ |
| 13 | Untokened fire | **proceeds** with `TokenId::ZERO`; partial mint reads as authorized | `schedule_watchdog.rs:216-232,313-345` | `schedule_watchdog_fr26.rs:656-716` (asserts the FW-10 fire) | AC8 ◆ |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-8c809f9c.md` (C1–C11).

| Premise (epic / preflight / audit / tracker) | Fact at `8c809f9c` and consequence |
|---|---|
| "Successor factory refuses spawned forms (`main.rs:3388-3402`)" (E20:268, PF:244); `:3331-3340 @61af7461` (S:309) | The refusal is `main.rs:3423-3432`, inside the closure `:3417-3461` (C1, C2). |
| `deferred-work.md:1193,1204` (E20:270, :367) | `:1260` (PF-04) and `:1275` (untokened fire) (C5). |
| `schedule_watchdog_fr26.rs:505-542` (E20:367) | `:537-578`; the FW-10 vector D07 A inverts is `:656-716` (C6). |
| `first_id` `:326-342` (E20:369) | `:313-345` (C7). |
| `17-4…md:51,135` (RA:23, :182) | `:53,71,135,341` (C8). |
| `S:306; E20:271-276` (PF:83, :127) | `S:309; E20:264-270,362-370` (C9, C10). |
| "kernel and ABI changes measured before grant"; "kernel-Δ measured" (E20:82, :266; PF:127 "unmeasured") | Priced (§Kernel): R +16 / +21. Hooks, WIT, swap and cold swap are **kernel-free** (F1, F7). |
| "spawned forms receive neither new hook" (RA:23) | Every kernel hook already reaches the spawned SCB, where it is a no-op (F1). The fix is `WorkerSpirit` forwarding, not a kernel caller. |
| FR26 `[[schedule]]` treated as live | Admission never carries it (F4). The production watchdog fires for no admitted Spirit (D-20-9-08). |
| "Hosted real T2/T3 for spawned lifecycle/upgrade" (PF:127) | T2 only: no Spirit form runs under T3 (the Worker is not a form). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`,
`check-service-boundary`) run from a `git archive 8c809f9c` copy root per prototype. The untouched copy reproduces the pins
exactly (26841 / 95 files / set hash `db39bc4a…`; kernel 21042 tokei; aggregate 192519; service boundary 0 violations).
Detail and diffs: `creation/measurements.md`, `creation/prototypes/`, `creation/kloc-kernel-8c809f9c-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, ruled 2026-11-03; `20-9-flag-winston-decision.md`):**

| Shape | Kernel physical | Kernel tokei | Other | Surface gate | Diff |
|---|---:|---:|---|---|---|
| **R = KD07A-lean + KO-poll-x (D-13 A + D-14 A, recommended)** | **+16 (26857)** | **+21 (21063)** | iac +39 | pass | `r-kd07a-lean-plus-ko-poll-outcome-in-iac.diff` |
| KD07A-lean alone (D07 A only) | +8 (26849) | +14 (21056) | iac +32 | pass | `kd07a-lean-…diff` |
| KD07B-lean (D-14 B) | +2 (26843) | +8 (21050) | iac +32 | pass | `kd07b-lean-…diff` |
| KD07A-inline (D-14 C) | +27 (26868) | +32 (21074) | iac +7 | pass | `kd07a-inline-…diff` |
| KO-poll, enum in kernel (D-13 B) | +24 (26865) | +27 (21069) | iac +32 | classes row | `kd07a-lean-plus-ko-poll-outcome-in-kernel.diff` |
| KO-count (D-13 C) | +28 (26869) | +30 (21072) | iac +32 | **baseline re-issue** | `kd07a-lean-plus-ko-refused-counters.diff` |
| KO-row (D-13 D) | +20 (26861) | +26 (21068) | iac +48 | pass; row flood | `kd07a-lean-plus-ko-refused-poll-tl-row.diff` |
| KW (D-20-9-11 B) | +9 (26850) | +6 (21048) | — | pass | `kw-post-swap-window-accessor.diff` |
| everything else (hooks, WIT, swap, cold swap, schedules) | 0 | 0 | — | — | — |

**Cumulative (stated, not assumed):** 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) = 21125 for the ruled rows. The same-day
Rows ruled as recommended 2026-11-03 add 20-8 KS +33, 20-14 KT +25 and 20-13 KT3 −1, for **21182**. With R: **21203**, 6 below the 21209 STOP
and 797 below the ADR-069 alarm. With R + KW: 21209 (0 left). **R's ×2 ceiling gives 21224, which crosses the STOP**, so
D-20-9-15 A rules targeted STOP 21,230 for 20-9 L2; D-20-14-12 C (ask `d2014_12`) then sets JOINT targeted STOP 21,250 shared by 20-9 L2 + 20-14 L2 (combined cumulative must not exceed 21,250). Shared STOP stays 21,209 for everything else; ADR-069 alarm 22,000 untouched. The exchange is already applied: the record, the journal function and the
enum live in maos-iac, which is −11 tokei against the inline shapes. Every D-20-8-07 ruling leaves 20-9 at about 21203
(measurements §Contingencies). Physical expected: **27032**. 20-9 L2 is the last expected Epic-20 kernel lander and
re-bases on every landed pin.

**kloc forecast** (tokei; HEAD = every pin, 1 line aggregate room; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · R (AC7, AC8) | **+21** (physical +16) | `r-…diff` |
| maos-iac · `ScheduleSuppressedRecord`, `journal_schedule_suppressed`, `PollOutcome` (AC7, AC8) | **+39** | `r-…diff` |
| maos-bin · `[[schedule]]` carried at admission (AC3) | **+14** | `sched` |
| maos-bin · `WorkerSpirit` forwarding of nine hooks, bounded wait, typed failure (AC2) | +80…+140 [INFERENCE] | measurements 4 |
| maos-bin · session lifecycle turns, queue, in-export mapping (AC2) | +60…+120 [INFERENCE] | measurements 4 |
| maos-bin · spawned successor factory: gate, same-form, caps ⊆, launch, pre-warm, successor core, declaration row (AC4) | +80…+150 [INFERENCE] | measurements 4 |
| maos-bin · child switch at commit, retire without finalize, revert detection (AC4) | +50…+100 [INFERENCE] | measurements 4 |
| maos-bin · spawned HookPump/ConsolidationWatchdog registration, explicit telemetry listing (AC2) | +15…+40 [INFERENCE] | measurements 4 |
| maos-bin · door cold swap through the spawned admission path (AC5) | +60…+140 [INFERENCE] | measurements 4 |
| maos-bin · typed refusal codes, door mapping (AC4, AC5) | +15…+30 [INFERENCE] | measurements 4 |
| maos-wasm-host · WIT 2.4 lifecycle bindings, lifecycle wire, runner selection, `Unsupported` reply (AC1) | +80…+150 [INFERENCE] | 18-5 +157; 20-8 `wit` +51 |
| maos-wasm-host · `guests/lifecycle-probe` (charged, 20-8 F11) (AC1–AC4) | +60…+120 [INFERENCE] | 20-8 measurements 6 |
| maos-domain · refusal variants, only if needed (AC4) | 0…+6 [INFERENCE] | `UpgradeError::SuccessorFactory` carries codes today |
| xtask · `build-test-fixtures` entry for the new guest (AC1) | +5…+15 [INFERENCE] | 20-8 measurements 7 |
| WIT, TS template/example, docs, tests, fixtures, ratification/classes TOML | uncharged | `kloc_check.rs:294-317` |
| **aggregate** | **+74 measured; +579…+1085 total** | sum |

**Envelope (D-20-9-16 A, ruled 2026-11-03; ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | L1 ceiling | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +16 / +21 | **26873 / 21084** (26891 / 21096 with KW) | — (L2) | **yes — FLAG-Winston** |
| maos-bin | 34191 | +14 meas, +360…+720 | **36379** (35959 under D-12 B) | 36379 | yes |
| maos-wasm-host | 2412 | +140…+270 | **3222** | 3222 | yes |
| maos-iac | 8270 | +39 meas | **8348** | — (L2) | yes |
| maos-domain | 10099 | 0…+6 | **10117** | 10117 | yes |
| xtask | 44406 | +5…+15 | **44451** | 44451 | yes |
| aggregate | 192519 | +579…+1085 | **195700** (+148 + 3033); hardfail = measured + 1 | 195580 | yes |

**Interaction with other open envelopes (honest cumulative; every lander re-bases on the exact landed pins):**

| Crate | Pending ceilings over each story's own base (unlanded) | With 20-9 |
|---|---|---|
| maos-bin (34191) | 20-4 L1 ≤ 34643 (+452), 20-11 ≤ 34698 (+1200 over 33498), 20-2 ≤ 33534 (+36 over 33498), 20-1b ≤ 34410 (shared with 20-1a), 20-10 ≤ 34548, 20-8 ≤ 34767 (+576), 20-13 ≤ 34687 (+496), 20-14 ≤ 34441 (+250) | +2188 → up to ≈ +5200 plus the 20-1b/20-10 remainders if every ceiling were drawn |
| maos-wasm-host (2412) | 20-8 ≤ 2874 (+462) | +810 → up to +1272 |
| maos-iac (8270) | 20-11 ≤ 8630 (+360), 20-8 ≤ 8510 (+240) | +78 → up to +678 |
| maos-domain (10099) | 20-4 ≤ 10115, 20-1b ≤ 10153, 20-8 ≤ 10133, 20-13 ≤ 10114, 20-14 ≤ 10101 | +18 |
| xtask (44406) | 20-7 ≤ 45830, 20-2 ≤ 44860, 20-4 ≤ 44466, 20-8 ≤ 44459, 20-13 ≤ 44644 | +45 |
| aggregate (192519) | 20-4 L1 ≤ 193526, 20-11 ≤ 194058, 20-2 ≤ 192316, 20-1b ≤ 193768, 20-8 ≤ 194218, 20-13 ≤ 193564, 20-14 ≤ 192974 | +3181 → ≈ +10,800 if all drawn |

**Operator-ruling list (all as recommended, operator ask `d_batch4`, 2026-11-03; grants bind at dev start after remeasurement/rebase):** (1) FLAG-Winston kernel row (D-20-9-13/14, plus KW if D-11 B); (2) the cumulative
STOP (D-20-9-15); (3)–(8) the five crate ceilings above and the aggregate; (9) D-20-9-01…16. Each figure lands exact, with a
HISTORY row citing this key with `->` (D11 A).

## Decisions — ruled 2026-11-03 (operator, ask `d_batch4`; all as recommended)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-9-01** | Landing shape | **A** one key, two landings: L1 kernel-free (AC1–AC6), L2 kernel last (AC7, AC8, AC9 close); `done` after L2 · **B** one landing · **C** split keys `20-9a` (hooks + swap) / `20-9b` (watchdog) | **A.** The creation contract puts the kernel last, and no L1 AC needs a kernel line (F1, F7). C adds keys, which trips `check-exit-commands`/`check-decision-register` on renames (20-1a incident). Interim state: between L1 and L2, D07 A is still unimplemented (the FW-10 fire proceeds). That is labelled in the cookbook, and no release tag falls between the landings. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-02** | Start gate | **A** L1 after 20-8 L1 lands (WIT 2.3.0, schema 7, `emit_manifest_declaration_event`); L2 after 20-8 L2 and 20-11 L2 (both touch the kernel; 20-11's `forget_spirit` is in `schedule_watchdog.rs`) · **B** after 20-8 is `done` · **C** start now | **A.** 2.4.0 is cut on 2.3.0 (20-8 contract 1), and the successor's declaration row uses 20-8's single writer (contract 5). L1 does not need 20-8's coordinator gate; the spawned swap passes through it once it lands. C would cut WIT twice and leave two writers. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-03** | WIT shape for lifecycle | **A** `maos:spirit@2.4.0`: `interface lifecycle` exported by a new `world spirit-lifecycle { include spirit; export lifecycle; }`; `world spirit` unchanged apart from the version. The runner selects bindings by export presence (`bindgen!` with `with:` remaps so frame/recall/context/skills types stay shared). A lifecycle request to a 2.0–2.4 guest without the export is answered `Unsupported`, with no export call. Ratification `20-9-wit-spirit-2.4.0` · **B** add the exports to `world spirit` · **C** deliver triggers as new kernel-written frame kinds into `handle-frame` | **A.** It is the form 20-8's contract names (point 1: "a form old guests need not implement; the host selects bindings by export set", `wit_guest.rs:13-20`, `runner.rs:321-331`). B fails to instantiate every existing guest, because a world's exports are mandatory. C changes the frame ABI (`FrameKind` in maos-spirit-abi and maos-domain), and `handle-frame` cannot return a snapshot. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-04** | Export set | **A** FR55 parity: `idle`, `consolidate(batch)`, `telemetry(event)`, `schedule(payload)`, `pause`, `resume`, `snapshot() -> state`, `swap-in(state)`, `migrate(state) -> state`. `on_load`/`on_start` map to the existing `on-start`, `on_frame` to `handle-frame`, `on_unload` to `on-shutdown`. `on_swap_out` stays a host no-op (the guest's last call is `snapshot`) · **B** the six the epic names (snapshot, swap-in, idle, consolidate, telemetry, schedule) + `migrate`; pause/resume labelled not delivered · **C** the six only; `--policy migrator` refused for spawned forms | **A.** FR55 lists pause/resume (`functional-requirements.md:69`); without them the requirement stays partial for spawned forms. `migrate` is what makes "upgrade" (epic AC1) true across a major, because `run_migrator` calls the successor (`coordinator.rs:273-298`). The cost difference is one WIT func and one forwarding arm each [INFERENCE]. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-05** | Imports inside lifecycle exports (20-8 contract 1, 7) | **A** `idle`, `consolidate`, `telemetry` and `schedule` set `HostState.in_export`: `recall`, `context` and `skills` are callable and counted against `MAX_RECALLS_PER_TURN` (`spirit_session.rs:46`) per export. `pause`, `resume`, `snapshot`, `swap-in` and `migrate` are import-free (`unavailable`) · **B** no lifecycle export may import · **C** all may | **A.** `idle` is the anticipatory-reasoning anchor (FR55), and reasoning needs recall. 20-8 contract 7 leaves authoring to 20-9, and A allows it where the guest acts. A swap step must be side-effect-free: a `snapshot` that recalls or authors mid-swap would make the state depend on the IAC log at swap time. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-06** | What a lifecycle export may emit | **A** no frames: each export returns `result<_, lifecycle-error>` (`snapshot`/`migrate` return state). Effects go through imports (D-05) or on the next frame turn · **B** `idle`/`schedule`/`consolidate`/`telemetry` return `list<iac-frame>` delivered through `authorize_and_deliver` | **A.** An emitted frame needs an intent lineage, and a kernel trigger has no inbound lineage. The I14 guard refuses empty machine lineage (`xtask/error-catalog.toml:48`), so B would have to invent a "trigger lineage", a new I14 shape this story has no charter for. Native hooks act through `Ctx`, not returned frames, so A is the parity shape. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-07** | Telemetry for spawned forms | **A** a spawned pid is registered with HookPump and ConsolidationWatchdog at admission. It receives `on_telemetry_event` only when its manifest lists `on_telemetry_event` explicitly in `[lifecycle].enabled_hooks` (an empty list does **not** subscribe, unlike `kernel_invocation_allowed`'s default). It gets the same `governance:telemetry-subscription:granted/revoked` rows and the root stream Observer receives · **B** as A, filtered to events the Spirit itself emitted · **C** not delivered to spawned forms (labelled) | **A.** The right comes from operator admission recorded by the grant row, as for compiled-in Observer (`hook_pump.rs:14-25`). Explicit listing stops a third-party manifest with an empty list from receiving every scalar write silently. B delivers nothing: a WASM guest has no scalar-emission import (`ScalarTapEvent.spirit_id`, `i7.rs:45-57`). C leaves "live telemetry" (epic AC1) false. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-08** | `[[schedule]]` at admission (F4) | **A** `gate_manifest` parses `[[schedule]]` with the kernel successor parser's wrapper; `GatedManifest.schedules`; `bundle()` carries it for every form; malformed → typed `[schedule]` `section_parse` before a pid (prototype `sched`, bin +14) · **B** carry it for spawned forms only · **C** leave it, and label `[[schedule]]` parse-only | **A.** Admission and a hot swap disagree today (`admission.rs:471-484` vs `upgrade.rs:388-400`): a Spirit's schedule starts firing only after a swap. No shipped manifest declares `[[schedule]]` (grep 0), so A changes no shipped behaviour. B keeps the asymmetry for compiled-in Spirits. C keeps FR26 false at admission. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-09** | Spawned hot-swap mechanics | **A** the factory (`main.rs:3417-3461`) admits a `wasm-component` successor: reads the staged target (`SuccessorManifestSlot`, `operator_door.rs:374-398`), runs `gate_manifest`, and requires the same `forms` (`swap_form_mismatch`) and declared scopes ⊆ the predecessor's admitted `declared_scopes` (`swap_capability_widened`). It resolves `manifest_launch`, prepares a successor binding core at the predecessor's pid, and **pre-warms** the successor runner (Ready plus a lifecycle probe; no export → `spawned_swap_unsupported`). It journals the successor through `emit_manifest_declaration_event`. Dropping the successor object kills the pre-warmed child · **B** launch the successor inside `on_swap_in` · **C** stop the predecessor, then launch the successor with the state (no rollback after the stop) | **A.** The factory runs before any coordinator step (`upgrade.rs:82-123`), so every refusal leaves the predecessor untouched and Running. `run_migrator` calls the successor object before `replace_runtime` (`coordinator.rs:273-319`), so the child must already exist. B puts component compile and startup inside the hook budget (20-1a's debug startup incident). C breaks the saga's `RestorePredecessor`. A swap is not an escalation path: the successor cannot widen what the operator admitted at this pid without a new admission (cold swap). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-10** | "In-flight" tokens for a spawned form | **A** the swap runs at a turn boundary: `snapshot` is queued behind an active turn (bounded by the hook budget), so the turn finishes on the predecessor under its own leases, revoked at its own TurnComplete as today (`spirit_session.rs:445-447`). Every pid-scoped token survives (same pid), and the predecessor child is retired without the finalize closure's `revoke_all_for_pid` (`:523-527`) · **B** refuse the swap while a turn is active (`409 swap_turn_active`) · **C** carry the active turn's leases to the successor and replay its frame | **A.** No in-flight work is aborted and no token is revoked by the swap, which is what FR10 asks. C is infeasible: a WASM export cannot be interrupted to snapshot, and replaying a frame duplicates its effects. B pushes retries onto the operator for a condition the daemon can serialize. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-11** | Post-swap auto-revert of a spawned Spirit (F8) | **A** kernel-free: retire the predecessor child at commit. The session detects a revert (the SCB's runtime object is no longer its successor) and ends typed `swap_reverted_after_retire`, journaled; the Spirit is unloaded by the session's normal teardown · **B** park the predecessor child for the window, using **KW** `post_swap_window_open(pid)` (+9 / +6) · **C** park it, duplicating the 30 s / 300 ms window in maos-bin | **A.** A revert is the rare post-commit safety net (halt lost, nonce, pid missing; `post_swap_monitor.rs:105-160`). A fails closed and is visible with no kernel line. B costs a second T2 runner for 30 s per swap and 6 tokei of the last 6 lines (21209, 0 left). C drifts silently if the kernel window changes. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-12** | Cold swap of a spawned form | **A** the door's `cold_swap` admits the successor, and restores the predecessor, through the spawned admission path `InstalledSpirits::boot` uses (`prepare_spawned` → `manifest_launch` → `load_admit_start` → spawned `run_admitted_session`, `installed_spirits.rs:40-124`), at a new pid. No new operator verb; `maosctl load` of WASM stays refused · **B** typed refusal `cold_swap_spawned_unsupported` before unload (labelled; deferred row) · **C** a full door spawned surface (`maosctl load`/`start` of WASM manifests) | **A.** The epic names cold swap for spawned forms (E20:268; PF:127). Today it is refused only because `prepare_spirit` is in-process-only (F9). A reuses the one spawned admission implementation. C adds an FR9 verb surface no charter row asks for. B is the fallback if the operator wants the smaller story (bin −60…−140). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-13** ◆ | PF-04 refused-poll observer (FLAG-Winston) | **A** KO-poll-x: `ScheduleWatchdog::poll_once(fast) -> Vec<PollOutcome>` (`Fired`, `PrincipalRevoked`, `RateRefused`, `Suppressed`); the enum lives in `maos_iac::payload`; the spawn loop ignores the result (R: +16 / +21 with D-14 A) · **B** KO-poll with the enum in the kernel (+24 / +27; classes row) · **C** KO-count, refused counters on the struct (+28 / +30; kernel-surface baseline re-issue) · **D** KO-row, a `schedule.refused:<id>` TL row per refused poll (+20 / +26) | **A.** A test drives polls directly, a deterministic barrier with no wall clock (the PF-04 ask, D:1260), at the smallest kernel figure. It adds no kernel symbol and passes `check-service-boundary` unchanged. C changes a pinned struct signature (measured: "removed public kernel symbol"). D writes up to 3600 rows/h per drained schedule into the production TL. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-14** ◆ | D07 A implementation (policy RULED 2026-11-01; shape open) | **A** KD07A-lean: a refused declared scope, or no registry wired with declared scopes, suppresses the fire (no dispatch, no fire row). Scopes already minted for that fire are revoked. One `schedule.suppressed:<id>` row (`ScheduleSuppressedRecord{spirit_id, schedule_id, refused_scope, reason}`) is written by maos-iac, and the cadence is consumed (+8 / +14 alone) · **B** KD07B-lean: as A, minted scopes left to TTL ≤ 300 s (+2 / +8) · **C** KD07A-inline: journaled in the kernel (+27 / +32) | **A.** It closes the partial-mint observation (E20:369): no token minted for a suppressed fire stays live. Consuming the cadence bounds the suppression rows by cadence and bucket. Moving the record to maos-iac, beside `ScheduleFireRecord`, saves 18 tokei. B leaves live tokens for an action that never ran. C spends 18 kernel lines for no behaviour. Scope-free schedules fire unchanged (`TokenId::ZERO`, PF:291). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-15** ◆ | Cumulative STOP (21209) against R's ceiling | **A** exchange (applied) + one **joint** targeted STOP of **21250** ruled together with D-20-14-12 (covers 20-9's and 20-14's ×2 allowances, 21125 + 33 + 50 + 42 − 1 = 21249; 750 below the alarm); if D-20-14-12 is ruled differently, **21230** for 20-9 L2 alone (R's ×2 ceiling 21224) · **B** bump to the ADR-069 alarm line (22000) for the rest of Epic 20 · **C** split: D07 A here (KD07A-lean, 21196), PF-04 to a new key · **D** no bump: the measured figure governs, and L2 returns to the operator if the re-based figure exceeds 21209 | **A.** The measured figure fits (21203, 6 left), but the D02 A envelope does not. 20-3b/20-3c landed at 2–3× their creation figures, so a landing-time STOP is likely under D. A keeps the internal margin for Epic 21's kernel rows (`21-5`/`21-6`), which B would spend. C leaves PF-04 open and adds a key (`check-exit-commands` renames). Under D-20-8-07 B/C the figure is the same, about 21203 (measurements §Contingencies). | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A**; RULED A: targeted STOP of **21,230 tokei for 20-9 L2**. D-20-14-12 = **C** (ask `d2014_12`) then sets a **JOINT targeted STOP of 21,250** shared by **20-9 L2 + 20-14 L2**: their combined cumulative must not exceed **21,250**; shared STOP stays **21,209** for everything else; ADR-069 alarm **22,000** untouched |
| **D-20-9-16** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants only; every correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b/20-3c landed at 2–3× creation forecasts; 20-1a's review added +430 inside its ceiling. 20-9's bin figure is mostly unmeasured. | RULED 2026-11-03 (operator, ask d_batch4): as recommended **A** |
| **D-20-9-17** | Telemetry pump stall (GLM F1, review corrections) | **A** deliver one event to every subscriber side by side, each held to `min(95% of its [budget].time_cap_seconds, 5 s)`; an overrun is accounted as `BudgetExceeded` (quarantine); three consecutive deliveries over 80% of the budget also quarantine · **B** keep sequential delivery, only wrap each in a timeout · **C** per-subscriber dispatch queues | **A.** The pump was one task awaiting each subscriber to completion, so a guest sleeping `time_cap − ε` per event stalled every other subscriber and `flush`, and `BudgetWarning80` reset its strikes. A is the smallest change that bounds the stall; C costs a queue per subscriber and a lag policy. Consequence recorded in `deferred-work.md`: a compiled-in Observer hook needing more than the wall per event is now quarantined. | RULED 2026-11-11 (orchestrator, review correction round; reported to the operator in the final report) |
| **D-20-9-18** | Hooks that meet a busy, starting, or committed-but-unadopted session (GPT #1/#7, GLM F1) | **A** a spawned core expects its session from `prepare_spawned`, and a committed successor awaits adoption: their hooks queue for that session; a trigger that cannot start in half its budget returns `busy` and is dropped without a panic or strike (a snapshot waits the whole cap) · **B** return `Unsupported` until a session is attached (the L1 behaviour) · **C** queue without a start limit | **A.** B lost the daemon's only `on_idle` (it fires once per quiet period, during startup) and every hook in the handover gap; C charges a hook for another export's budget. | RULED 2026-11-11 (orchestrator, review correction round) |
| **D-20-9-19** | Artifact containment (GLM F2) | **A** `origin.join(artifact).canonicalize()` must stay under the canonical origin; absolute paths, `..` climbs and symlinks out are refused `artifact_escapes_origin` at launch and at the cold-swap restore preflight · **B** leave it to the manifest parser | **A.** The parser already refuses `..` and absolute paths; a symlinked directory was the open gap. | RULED 2026-11-11 (operator, via orchestrator): as **A** |
| **D-20-9-20** | AC1.3 premise ("at HEAD the guest fails to instantiate") | **A** reword: at the base the 2.4.0 component instantiates (it `include`s `spirit`) and what is missing is any path that calls a lifecycle export; failing-before is the base daemon witness plus code reading, not an instantiate failure · **B** keep the sentence | **A.** `failing-before-head-probe-ac1.log` shows `ready:true` at the base; B is false. | RULED 2026-11-11 (orchestrator, review correction round) |
| **D-20-9-21** | Pump walls, busy hooks and warning accounting (round 3: GPT r2 #3/#4, GLM R1-R3; refines D-20-9-17/18) | **A** (1) a spawned `telemetry`/`consolidate` trigger gives up for contention inside the pump's wall (queue give-up `min(cap/2, wall/2)`), and a per-pid busy counter lets the pump tell that drop from a delivery: it counts as neither delivered, a strike nor a clean run, and `last_consolidate` does not advance; (2) the near-budget count is a telemetry-only window, 3 slow of the last 5, a fast delivery does not erase it; (3) `on_consolidate` (watchdog pass, forget notices, late-notice sweep) runs side by side under `min(95% cap, 30 s)`, the privacy bypass of quarantine kept and bounded; a wall overrun's row names the wall · **B** per-subscriber dispatch queues (D-20-9-17 C) · **C** leave consolidation sequential | **A.** B costs a queue and a lag policy per subscriber and is not needed to bound one event; C leaves a GDPR erasure completion stallable for a manifest budget (≤ 24 h) by a quarantined subscriber. | RULED 2026-11-11 (orchestrator, round 3; the 30 s wall and the deferral of TL writes under the registry mutex approved) |
| **D-20-9-22** | Fence ownership of a swap's snapshot (GPT r2 #1; refines the L1 correction) | **A** the swap attempt owns the fence: the predecessor link records the live attempt, the successor's drop ends it and unfences under the same lock the session takes to fence on a snapshot reply, so a reply after a refused swap (by either the dispatcher's or the link's deadline) fences nothing · **B** share the dispatcher's cancellation with the link | **A.** The dispatcher times out on the predecessor's SCB budget and the link on its own cap, so a call-level cancellation cannot be the owner. | RULED 2026-11-11 (orchestrator, round 3) |
| **D-20-9-23** | FR11 archive record of a spawned cold-swap successor (GPT r2 #2) | **A** written only after the component answered its probe (`load_admit_start_unarchived` + `archive_admitted` in `admit_spawned`); a failed start also withdraws its pump registration · **B** roll the record back on failure | **A.** B needs the previous newest record's mtime restored. | RULED 2026-11-11 (orchestrator, round 3) |
| **D-20-9-24** | Which dispatch a busy drop and a wall belong to, and what a wall may stop (round 4: GPT r3 #1/#2, GLM r3 F4/F5; refines D-20-9-21) | **A** (1) each pump dispatch registers a per-invocation record (`invocation.rs`, keyed pid + hook kind + the event's `seq` or a digest of the batch) the spawned link attaches its call to; the session marks it started, the link marks it dropped, and the registering dispatch removes it, so a drop is attributed to the invocation that suffered it and no ledger outlives a dispatch (the shared per-pid counter is gone); (2) the wall is `min(95% cap, limit)` counted from when the export starts (a trigger queued behind a turn is not billed for the wait), and a dispatch runs as its own task: the wall ends the accounting, never the execution, so an overrun's task is parked and the watchdog's exit and `HookPump::shutdown` join it before anything is unloaded; (3) a spawned telemetry/consolidate call is cut loose by its link at `queue share + wall + 2 s`, so the join is not held for a manifest budget of up to a day · **B** keep the pid counter and drop the dispatch at the wall · **C** pass a disposition back through the kernel dispatcher | **A.** B misattributes a concurrent hook's drop (a 4 s successful notice read as busy: the purge reported undelivered) and detaches a within-budget hook that shutdown then unloads under; C is a kernel line. | RULED 2026-11-11 (orchestrator, round 4) |
| **D-20-9-25** | Swap-stage walls (round 4: GLM r3 F1) | **A** pre-warm `Ready`, the probe, every inline swap-stage exchange (`migrate`, `swap-in`) and the cold-swap readiness wait are bounded by `min(declared budget, PREWARM_WALL = 60 s)`, mirroring `fire_walled`; a failed inline exchange drops the successor's child and releases the predecessor's fence at once rather than leaving it to the link's drop; a per-spirit upgrade lock is a deferred row · **B** leave the successor's declared budget (up to 86,400 s) as the bound | **A.** B lets a silent successor fence the predecessor and hold the door's global `upgrade_lock` for a day per stage. | RULED 2026-11-11 (orchestrator, round 4) |
| **D-20-9-26** | The snapshot's own attempt (GPT r3 closure note; refines D-20-9-22) | **A** a snapshot call carries the swap attempt it was taken for (`CallState`), and the session fences the predecessor only if that attempt is still the live one, so a slow snapshot of a refused swap never fences a later attempt · **B** fence while any attempt is live | **A.** B fences the retry's predecessor and the retry's own snapshot is answered `Unsupported`. | RULED 2026-11-11 (orchestrator, round 4) |
| **D-20-9-27** | One record per live dispatch, and what the pump keeps (round 5: GPT r4 #1-#3, GLM r4 F3; refines D-20-9-24) | **A** (1) no two live dispatches share a key (`Registered::open` waits while its key is in use, which the holder's wall bounds): content cannot identify an invocation and the kernel dispatcher offers no channel for a token, so identical batches take turns instead of aliasing one record; (2) a trigger that cannot enter the session's one queue slot is a dropped invocation exactly like one that expires in it (`cancel_queued` on the full-channel branch); (3) parked overruns are reaped on every parking, so the pump holds what still runs; (4) the pump builds its dispatch SCB per dispatch and caches none, so no runtime object a swap installed and undid is kept alive by the pump · **B** claim the first unreserved record (swaps the two calls' records when their link calls arrive in the other order) · **C** pass a token through the kernel dispatcher (kernel source) | **A.** B misattributes under reordering; C is a kernel line. | RULED 2026-11-11 (orchestrator, round 5) |
| **D-20-9-28** | A swap attempt owns its token, and has one deadline (round 5: GLM r4 F1/F2; supersedes the per-stage 60 s of D-20-9-25) | **A** (1) the successor's link holds the attempt it began on the predecessor (`begin_attempt`, which ends and unfences whatever it replaces); only `end_attempt` of the INSTALLED token releases the fence, so a late drop of an older successor object never steals a newer attempt; a successor's guest refusal (`declined`/`failed`) releases its attempt at once like a transport error; (2) every pre-adoption stage (launch, `Ready`, probe, `migrate`, `swap-in`) takes its wall from ONE per-attempt deadline of `ATTEMPT_WALL = 90 s` set when the factory starts, instead of a fresh 60 s per stage; the wait for the predecessor's active turn counts against it · **B** keep per-stage walls · **C** bound `upgrade_lock` and add a per-spirit failed-attempt backoff (door; deferred) | **A.** B lets a successor slow in every stage hold the global lock and the fence for about 240 s per attempt. | RULED 2026-11-11 (orchestrator, round 5) |
| **D-20-9-29** | Hosted run 42987393747 @ `459a606a` reds `check-dev-model-tier` on this record's dev model `anthropic/claude-sonnet-5-5` (the same model the 20-5 L1 record names; `FRONTIER_FAMILIES` did not list it) | **A** add the `sonnet-5-5` token to `FRONTIER_FAMILIES` as the documented A1 maintenance (13.5a `opus-5`, 14-1 `glm-5.3`, 17-3c `gpt-6.1`, 20-3a `gpt-6-astra` precedents); `sonnet-4-5` and the haiku tier stay out; the §A6 review-marker requirement is unchanged · **B** re-run the dev records on an allowlisted model · **C** leave the gate red | **A.** Operator ruling 2026-11-11 (Lunarpulse, ask `tier` = allow): the model is frontier-class for the gate. The 20-9 and 20-5 records stay as written; nothing is re-attributed. | RULED 2026-11-11 (operator, ask `tier`) |

## Consumed from 20-8 (`20-8-manifest-contract-completion.md:176-197`; not re-decided)

1. **WIT.** The base is `maos:spirit@2.3.0`. 20-9 cuts **2.4.0** (D-03), does not re-cut 2.3.0, and adds ratification
   `20-9-wit-spirit-2.4.0` beside `20-8-wit-spirit-2.3.0`. D-05 answers 20-8's open in-export question.
2. **Manifest.** No new section and no schema 8: telemetry uses `[lifecycle].enabled_hooks` (D-07), and the swap uses
   `[hot_swap]`, `[migrates_from]` and 20-8's `[swap_invariants]`.
3. **Swap.** The spawned swap goes through the same coordinator. If 20-8 L2 has landed, `SwapInvariantDropped` refuses
   before `on_swap_out` as for in-process. Under D-20-8-07 C, 20-9 owns KB + KS (+33, priced by 20-8) and D-20-9-15 re-asks.
4. **Journal.** The successor's admission on a swap or upgrade hop writes its `governance:manifest-declaration` row through
   `emit_manifest_declaration_event`. There is no second writer.
5. **Skills.** Lifecycle exports under D-05 A reach 18-7's port through the same `MediatedLogRecall.skill`.

## Acceptance Criteria

*(Epic §20-9 as amended 2026-10-30. Clauses marked (D-…) carry the operator-ruled recommendation; a later different
ruling rewrites only the affected clauses. Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host with
`--features maos-bin/wasm-host` and the guest toolchain; **hosted** = the discipline run.)*

### L1 — kernel-free

#### AC1 — `maos:spirit@2.4.0` lifecycle interface, additive; old guests unchanged (epic AC2; D-03, D-04, D-05, D-06)

1. `wit/spirit.wit`: package `2.4.0`; `interface lifecycle { use frames.{halt}; record telemetry-event { seq: u64,
   spirit-id: string, tag: string, value: f64, timestamp-ms: u64 }; variant lifecycle-error { declined(string),
   failed(string) }; idle; consolidate(batch: list<u8>); telemetry(event); schedule(payload: list<u8>); pause; resume;
   snapshot() -> result<list<u8>, lifecycle-error>; swap-in(state: list<u8>); migrate(state: list<u8>) ->
   result<list<u8>, lifecycle-error> }`. Every other export returns `result<_, lifecycle-error>`. Also
   `world spirit-lifecycle { include spirit; export lifecycle; }`. Add `[[ratification]] 20-9-wit-spirit-2.4.0`. Re-issue the
   four provenance-pinned components by `build-test-fixtures`; `check-equiv-fixture-provenance` green.
2. `maos-wasm-host`: `lifecycle_wire` (private tags 60002/60003, the `recall_wire` pattern): `LifecycleRequest::{Probe,
   Idle, Consolidate, Telemetry, Schedule, Pause, Resume, Snapshot, SwapIn, Migrate}` → `LifecycleReply::{Supported(bool),
   Done, State, Declined, Failed, Unsupported}`, plus in-file round-trip vectors. The runner selects `spirit-lifecycle`
   bindings when the component exports `maos:spirit/lifecycle@2.4.x`, else the existing selection (`runner.rs:321-331`);
   a request to a guest without the export gets `Unsupported`. `in_export` per D-05 A; per-export fuel `args.fuel`. A
   trap or halt in a lifecycle export is a runner error exit, because the instance is poisoned; it is never retried.
   The major check covers `lifecycle@`, and the instantiate error text names 2.4.0.
3. **Failing-before (local):** `crates/maos-wasm-host/tests/wit_corpus.rs` + a new lifecycle vector set using
   `guests/lifecycle-probe` (new; counts each trigger, keeps a counter state, `snapshot`/`swap-in`/`migrate` round-trip it).
   At the base (`fc7373f0`) the 2.4.0 guest instantiates and loads (it `include`s `spirit`; D-20-9-20), but nothing ever calls a
   lifecycle export: `WorkerSpirit` has no lifecycle hooks and the runner has no lifecycle wire. After: each request reaches its
   export. The 2.0/2.1 legacy, 2.2 and 2.3 guests still run and answer `Probe` with `Supported(false)`.
4. **Plant (local):** the runner answers every lifecycle request `Unsupported` ⇒ the lifecycle-probe vector reds.
5. TS SDK and docs (uncharged): the `wit/spirit.wit` copies in `templates/spirit-ts/` and `examples/example-spirit-ts/` equal
   the root. The example exports `lifecycle` (`idle`, `snapshot`, `swap-in`) for AC6. README,
   `docs-site/docs/cookbook/lifecycle-hooks.md` + ko (spawned section: which export each trigger calls, budgets, in-export
   rule, no frames) and `docs-site/docs/cookbook/wasm-component-spirit.md` + ko. `gen-abi-docs --check` green.

#### AC2 — budgeted triggers reach a spawned Spirit (epic AC1; FR55; D-04, D-05, D-07)

1. `WorkerSpirit` (`supervision.rs:2612-2633`) implements `on_idle`, `on_consolidate`, `on_telemetry_event`, `on_schedule`,
   `on_pause` and `on_resume`. Each sends one lifecycle request to its session through the shared `BindingCore` (bounded slot)
   and blocks for the reply, never longer than the dispatcher's effective cap (`hook_dispatch.rs:356-363`). `Unsupported` is a
   no-op. `Declined`/`Failed`, a session that ended or a missed deadline panics with a typed message. That is the only failure
   channel a `Spirit` hook has (`lifecycle.rs:233-296`), so the dispatcher reports `HookOutcome::Panicked` and its existing
   accounting applies. The telemetry payload (JSON `ScalarTapEvent`, `hook_pump.rs:400`) is decoded without
   `unwrap`/`expect`. The kernel's enabled-hooks gate is unchanged (`hook_dispatch.rs:174-184`).
2. The session loop (`spirit_session.rs:371-497`) runs one export at a time: a lifecycle request is taken only when
   `ready && !active`, under the same per-export deadline (`turn_budget`) and protocol checks as a frame turn. A request
   pending when the session stops is answered `SessionEnded`.
3. Callers: `IdleWatchdog` is unchanged and reaches spawned SCBs. The `admission.rs:960-968` filter registers spawned pids
   with HookPump and ConsolidationWatchdog. Telemetry eligibility for a spawned pid is an explicit `on_telemetry_event` in
   `[lifecycle].enabled_hooks` (D-07 A), with the grant/revoke rows. `on_schedule` arrives through AC3.
4. Budget: a guest export past `[budget].time_cap_seconds` yields the kernel's `BudgetExceeded` row/frame
   (`hook_dispatch.rs` `emit_budget_frame`), and the session's deadline ends the child (fail-closed, journaled).
5. **Failing-before (local):** `crates/maos-bin/tests/spawned_lifecycle_20_9.rs` (new; `maos run <lifecycle-probe manifest>`
   with `MAOS_IDLE_FAST`). At HEAD the guest's counters stay 0 for idle, consolidate, pause/resume and telemetry. After:
   each trigger is counted as fired by its caller (the guest reports counts on its next `handle-frame` turn). Telemetry reaches
   only the listing guest, and an empty `enabled_hooks` does not subscribe. A 2.3 guest receives nothing and keeps
   running. A looping `idle` produces one `hook.budget.on_idle` row and a session ended with a journaled cause.
6. **Plant (local):** `WorkerSpirit::on_idle` reverted to the default no-op ⇒ the idle vector reds.

#### AC3 — `[[schedule]]` is carried at admission (FR26; D-08 A)

1. Apply `sched` (re-cut over 20-4 L1's `admission.rs` hunks): `GatedManifest.schedules`; `gate_manifest` parses
   `[[schedule]]` with the kernel parser's wrapper; `bundle()` carries it. A malformed entry is a typed `[schedule]`
   `section_parse` refusal before a pid.
2. **Failing-before (local):** in `spawned_lifecycle_20_9.rs`, a lifecycle-probe manifest with one `[[schedule]]` (cadence 1,
   `MAOS_SCHEDULE_FAST=1`) never fires `schedule` at HEAD (bundle has no entries). After: it fires, and a `schedule.fire:<id>`
   row precedes the guest's count. A compiled-in test manifest with the same entry fires too. A malformed entry is refused.
3. **Plant (local):** `bundle()` drops `schedules` ⇒ the schedule vector reds.

#### AC4 — hot swap and upgrade of a spawned Spirit preserve tokens, working memory and state (FR10; D-09, D-10, D-11)

1. The factory admits a `wasm-component` successor per D-09 A, and every refusal comes before any coordinator step, with the
   predecessor Running: `swap_form_mismatch`, `swap_capability_widened`, `spawned_swap_unsupported` (predecessor or successor
   without the lifecycle export), `section_parse`/ABI refusals from `gate_manifest`. The successor's declaration row is
   written through `emit_manifest_declaration_event` (20-8 contract 5). The `rust-inproc` arm is unchanged.
2. The coordinator path is unchanged (kernel): `snapshot` (the predecessor guest, queued behind an active turn, D-10 A), the
   envelope check, `migrate` under `--policy migrator` with `[migrates_from]` (the successor guest), `replace_runtime`, then
   `swap-in` (the successor guest). At commit the session switches its live child to the successor and retires the predecessor
   with a planned stop whose exit closure does **not** revoke pid tokens or remove the context budget. A refusal at any
   coordinator step kills the pre-warmed successor, and the predecessor keeps serving.
3. Preserved: pid; every pid-scoped token (`revoke_all_for_pid` is never called by a swap); kernel `WorkingMemoryStore`
   entries keyed by pid; the coordinator's I14 carry (a predecessor's pending halt survives, F13). Turn leases are revoked
   only at their own TurnComplete.
4. Auto-revert (D-11 A): a revert to the retired predecessor ends the session typed `swap_reverted_after_retire`, with one
   journaled row and the Spirit unloaded.
5. **Failing-before (local):** `crates/maos-bin/tests/spawned_swap_20_9.rs` (new). It runs `maos run <lifecycle-probe v1>`,
   drives N frames (guest counter N), holds a schedule-fire token (AC3, one declared scope), starts a long turn, and runs
   `maosctl spirit upgrade lifecycle-probe --to <v2>`. At HEAD that is `409 … spawned_surface_unsupported: hot-swap successor`
   (`main.rs:3427`). After, the command exits 0 and:
   - the pid is the same;
   - the in-flight turn completes on v1 before `snapshot`;
   - v2's counter continues from N;
   - the schedule token is still active, verifies, and has no `cap.revoke` row;
   - there is one `lifecycle.hot_swap` journal row and one successor declaration row;
   - a pending halt is carried (the `routed_halt_swap_20_3b.rs` shape);
   - under `--policy migrator` with `[migrates_from]`, v2's `migrate` sees v1's state.

   Refusal vectors: form mismatch, widened scopes, and a v1 2.3 guest without the export. Each refuses typed, and the
   predecessor stays Running and unchanged. With `MAOS_AUTO_REVERT_FAST=1` and a forced violation, the session ends
   `swap_reverted_after_retire`.
6. **Plant (local):** the retire path calls `revoke_all_for_pid` ⇒ the token-survival assertion reds.

#### AC5 — cold swap of a spawned Spirit; the Worker keeps its refusal (D-12 A)

1. `cold_swap` (`operator_door.rs:1372-1513`): for a `wasm-component` predecessor, the preflight (`:1415-1436`), the successor
   admission and the restore go through the spawned admission path (D-12 A). Cold-swap semantics are unchanged (new pid,
   tokens revoked by unload, state not carried; `:1361-1371`). `prepare_spirit`'s `operator door` refusal stays for
   `maosctl load`.
2. `[cli_wrapper]` targets keep `409 cli_wrapper_is_not_a_spirit_form` (`:1190-1199`), and the wasm successor vector of
   `wasm_form_admission_17_3b.rs:338` is updated to the new typed outcome.
3. **Failing-before (local):** in `spawned_swap_20_9.rs`, `--policy cold-swap` of a spawned Spirit is `409
   cold_swap_unrestorable` before unload at HEAD. After: completed at a new pid, with the predecessor's tokens revoked and the
   guest counter reset. A refused successor gives `cold_swap_reverted`, with the predecessor restored at a new pid. A Worker
   target gives `cli_wrapper_is_not_a_spirit_form`.
4. **Plant (local):** the restore path skips the spawned arm ⇒ the reverted vector reds.

#### AC6 — hosted T2 proof (epic AC1 "hosted")

1. `wasm-host-tests` (`discipline.yml:4550`) runs AC1–AC5's local legs. `epic-20-exit` (`:2898`) builds the TS example at
   2.4.0, publishes, installs and boots it, drives `idle`, publishes v+1, and hot-swaps to it. It asserts the same pid, the
   example's counter continuing, and one successor declaration row. `check-wasm-form-equiv` (`:5236`) is green on the
   re-issued fixtures. Run on release builds (20-1a's debug-startup lesson).
2. **Plant (hosted, one image):** the example drops its `swap-in` export ⇒ the exit job's swap assertion fails with
   `spawned_swap_unsupported` (`proof-contract.md:14-17`).

### L2 — kernel last

#### AC7 — deterministic refused-poll observer (PF-04; D-13 A) ◆ FLAG-Winston

1. Apply R's `poll_once` half (re-cut over 20-11's `forget_spirit` hunk). `ScheduleWatchdog::poll_once(fast) ->
   Vec<maos_iac::payload::PollOutcome>`; the spawn loop calls it. `check-service-boundary` stays unchanged (no new kernel
   symbol).
2. **Failing-before (local):** `schedule_watchdog_fr26.rs:537-578` is rewritten. At HEAD it can only bound a wait on the
   admitted-fire row and states it "does not prove" a refusal occurred. After: two `poll_once` calls on a `rate=1` entry
   return `[Fired]`, then `[RateRefused]`, with no sleep. A revoked principal returns `[PrincipalRevoked]`, and `0` fires are
   asserted from the same outcomes.
3. **Plant (local):** the bucket-refusal arm pushes no outcome ⇒ the `RateRefused` assertion reds.

#### AC8 — D07 A: a refused declared side-effect mint suppresses the fire (RULED 2026-11-01; D-14 A) ◆ FLAG-Winston

1. Apply R's suppression half: `issue_narrowed_token` returns `Result<Option<TokenId>, (Option<Scope>, String)>`. A refusal,
   or declared scopes with no registry, means no fire row, no hook dispatch, the fire's minted scopes revoked, one
   `schedule.suppressed:<id>` row, and the cadence consumed. Scope-free entries fire as today (`TokenId::ZERO`).
2. **Failing-before (local):** `schedule_17_9_exhausted_quota_records_fire_without_minting` (`:656-716`) asserts the FW-10
   fire at HEAD. It is inverted: counter 0, no `schedule.fire:` row, one suppressed row naming the refused scope, the quota
   evidence row kept. A new partial-mint vector (two scopes, the second refused by policy) shows one `Revoke` cap-audit event
   for the first token, no live token for the pid, and no fire row.
3. **Plant (local):** skip the suppression (fall through to the fire) ⇒ the inverted vector reds.
4. Pins: `xtask/kernel-core-baseline.toml` (`src_lines` `:505`, `scheduler/schedule_watchdog.rs` `:1005`, set hash `:939`, EOF
   HISTORY; `hot_swap/coordinator.rs` `:958` only under D-11 B), `xtask/kloc.toml:264` and `:406` exact, HISTORY rows citing
   this key with `->`. Re-base on whatever kernel rows have landed (20-1b, 20-4, 20-11, 20-8, 20-13, 20-14). Update
   `20-9-flag-winston-decision.md` with the landed figure.
5. Docs: the FW-10 sentences in ADR-013 (`:98`) and ADR-030 (`:50`) and the cookbook's schedule section say what D07 A does.

#### AC9 — records

1. `tests/coverage-matrix.yaml` FR55 (`:811`) names AC1/AC2 witnesses for spawned forms. FR10 (`:351`, `gates: []`) names AC4.
   FR26's row names AC3/AC7/AC8. `docs/invariants/I14.md` "Routed FR53 witness" names the spawned swap vector; the claim stays
   `asserted`.
2. At `done`: `deferred-work.md:1260` and `:1275` → `CLOSED <date> by 20-9 @<commit>` with witness. RA `:173`/`:182` and RI
   `:566`/`:594` are closed for spawned forms. `check-dev-record-completeness` passes. Epic, tracker, preflight, audit and
   inventory records are updated per `creation/records-owed.md`.

## Tasks / Subtasks

- [x] **T0 — Preflight.** Rulings applied; grants re-based (bin 38422, wasm-host 3348, xtask 44980, aggregate 202209; pins at start bin 36234, wasm-host 2538, xtask 44935, aggregate 199198). 20-8 L1 is in the tree (WIT 2.3.0). The T0 compile checks passed: `bindgen!` `with:` remaps and WIT `include` work on wasmtime 48.0.8 (`spirit-lifecycle` world, `lifecycle_runner_20_9`). The `[[schedule]]` re-cut compiled inside bin.
- [x] **T1 — WIT 2.4 + runner (AC1).** WIT, ratification, lifecycle wire, binding selection, guest, fixtures, vectors, plant. Plant observed red.
- [x] **T2 — Spawned triggers (AC2).** `WorkerSpirit` forwarding, session lifecycle turns, registration, `spawned_lifecycle_20_9.rs`, plant observed red.
- [x] **T3 — Schedules at admission (AC3).** `sched` re-cut, vectors, plant observed red.
- [x] **T4 — Spawned hot swap / upgrade (AC4).** Factory arm, pre-warm, child switch, retire, revert detection, `spawned_swap_20_9.rs`, door vector, plant observed red.
- [x] **T5 — Cold swap + Worker (AC5).** Door spawned path, in-process and door vectors (`spawned_door_20_9.rs`), plant observed red.
- [x] **T6 — TS + docs (AC1.5).** Template/example at `spirit-lifecycle`, cookbook en + ko, `gen-abi-docs --check` green. The TS legs on node 22/24 are hosted-only.
- [ ] **T7 — L1 landing (D-01 A).** Pins exact and inside the envelope; gates run (see Dev Agent Record); hosted discipline run and the AC6 plant outstanding (branch-only recipe `20-9-spawned-form-lifecycle-hooks-evidence/ci/ci-20-9-verify.yml`, unverified). No commit or push without the operator.
- [ ] **T8 — Kernel (AC7, AC8)** under the ruled rows: red-before tests, R re-cut, plants.
- [ ] **T9 — L2 landing + records (AC9).** Kernel pins + hashes (re-base); FLAG record updated; `cargo test -p maos-kernel-core --lib`; hosted run; ledger closure; coverage; docs.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| WIT / ABI | `wit/spirit.wit` (package `:30`, new interface + world after `:391`); `xtask/abi-ratifications.toml` (one entry); `tests/fixtures/wasm/*`; `STABILITY.md` if the gate lists WIT versions | AC1 |
| WASM host | `crates/maos-wasm-host/src/{runner.rs:172-450, wit_guest.rs:1-20, host_state.rs (in_export), lib.rs}`; NEW `src/lifecycle_wire.rs`; NEW `guests/lifecycle-probe`; `tests/wit_corpus.rs` | AC1 |
| Bin — triggers | `crates/maos-bin/src/supervision.rs:1710-1730,2612-2633` (`BindingCore` slot, `WorkerSpirit` hooks); `spirit_session.rs:371-497` (lifecycle turns); `admission.rs:960-968` (registration); `hook_pump.rs:305,858-885,1018-1050` (spawned eligibility) | AC2 |
| Bin — schedules | `crates/maos-bin/src/admission.rs:439-484,590-745` | AC3 |
| Bin — swap | `crates/maos-bin/src/main.rs:3417-3461` (factory arm); `spirit_session.rs:499-527` (retire without finalize); `operator_door.rs:374-398` (slot), `:1157-1359` (error mapping) | AC4 |
| Bin — cold swap | `crates/maos-bin/src/operator_door.rs:1372-1560`; `main.rs:10198-10230` (`prepare_spirit` unchanged for load); `installed_spirits.rs:40-124` (reused path) | AC5 |
| IAC (L2) | `crates/maos-iac/src/adapter/payload.rs` (after `:31`) | AC7, AC8 |
| Kernel ◆ (L2) | `crates/maos-kernel-core/src/scheduler/schedule_watchdog.rs:133-346`; `tests/schedule_watchdog_fr26.rs:537-578,656-716`; pins `xtask/kernel-core-baseline.toml`, `xtask/kloc.toml:264` | AC7, AC8 |
| TS / docs | `templates/spirit-ts/`, `examples/example-spirit-ts/`; `docs-site/docs/cookbook/{lifecycle-hooks,wasm-component-spirit}.md` + ko; ADR-013 `:98`, ADR-030 `:50` (text); `docs/invariants/I14.md` | AC1, AC8, AC9 |
| CI / records | `.github/workflows/discipline.yml:2898` (`epic-20-exit` swap steps), `:4550`; `tests/coverage-matrix.yaml:351,811`; `deferred-work.md:1260,1275`; `xtask/kloc.toml` rows | AC6, AC9 |

A new maos-bin `src` file (for example `spawned_lifecycle.rs` for the `BindingCore` slot and the wire mapping) must join
`SCANNED_SOURCE_FILES` (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:931`). No new env variable is planned
(`MAOS_IDLE_FAST`, `MAOS_SCHEDULE_FAST`, `MAOS_AUTO_REVERT_FAST` exist). Any new one must be registered in
`crates/maos-bin/src/env_contract.rs`.

### Overlap (serialize; re-measure)

- **`admission.rs` / `main.rs`** — 20-4 L1 (working tree now: `[providers]`, `AdmissionDeps`), 20-8 L1 (FR8 sections, record, `bundle()`), 20-1b AC10, 20-13. `sched` is re-cut after 20-4 L1 and 20-8 L1. One integration owner per file per landing.
- **`schedule_watchdog.rs`** — 20-11 L2 `forget_spirit` (distinct hunk, `:130`). The later lander re-cuts.
- **`coordinator.rs`** — 20-8 L2 KS (`SwapInvariantDropped`); KW only under D-11 B.
- **WIT / fixtures** — 20-8 L1 (2.3.0). 20-9 re-issues the fixtures again at 2.4.0.
- **maos-iac pin** — 20-11 (≤ 8630), 20-8 (≤ 8510); the later lander re-bases.
- **Kernel cumulative** — every Epic-20 kernel row (§Kernel). 20-9 L2 is expected last.

### Current state that must be preserved

- 17-3c/17-3d/18-5: one stdio buffer pair per session (`host_state.rs:24-27`), in-band recall, per-turn budgets, turn-scoped leases, protocol-violation closure journaling, and the 2.0/2.1 legacy bindings.
- 17-4: compiled-in HookPump/ConsolidationWatchdog semantics, quarantine, grant/revoke rows, Observer-class eligibility.
- 19-5: the door's forward-only, vetting and ABI prechecks (`operator_door.rs:1157-1300`), the FR11 archive at every granted admission, the halt-continuity gate first in the swap, cold-swap semantics, and `cli_wrapper_is_not_a_spirit_form`.
- 20-3c K1p: the swap **carries** pending halts (I14). Never re-add a drain (`I14.md:46-49`).
- 6.4 / 17-9: the rate bucket is consumed before minting, and one narrowed token per scope per admitted fire (`schedule_watchdog_fr26.rs:580-611`).
- 20-8: one declaration writer, schema 7, WIT 2.3.0 skills import.

### Guardrails

- No kernel line without the ruled FLAG-Winston row. Any `crates/maos-kernel-core/src` change outside `schedule_watchdog.rs` (and `coordinator.rs` under D-11 B) is STOP.
- No second admission implementation: the swap successor and the spawned cold swap reuse `gate_manifest`, `manifest_launch`, `prepare_spawned`, `load_admit_start` and `emit_manifest_declaration_event`.
- A swap never revokes a pid token, never widens the admitted scopes, and never aborts an active turn. Every refusal comes before `on_swap_out` and leaves the predecessor Running.
- A lifecycle wire request never carries a pid, class, token or author. The host binds them. A refusal carries a stable code, never free text.
- A hook never blocks past the dispatcher's cap. A lifecycle failure is a typed hook panic, never a silent success.
- No `unwrap`/`expect` after serde in production paths (telemetry decode, wire decode, suppressed record).
- No test spawns cargo without the expensive marker. Guest fixtures come from `build-test-fixtures`, never built inside a test.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it. Plants are byte-restored (`proof-contract.md:24-31`). Renaming or adding story keys is not planned; if done (D-01 C, D-15 C), run `check-exit-commands` and `check-decision-register` (20-1a incident).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Lifecycle exports (AC1) | runner answers `Unsupported` | lifecycle-probe vector | local |
| Spawned triggers (AC2) | `WorkerSpirit::on_idle` no-op | idle vector | local |
| Schedules carried (AC3) | `bundle()` drops `schedules` | schedule vector | local |
| Token survival (AC4) | retire calls `revoke_all_for_pid` | token-survival assertion | local |
| Spawned cold swap (AC5) | restore skips the spawned arm | reverted vector | local |
| Hosted T2 (AC6) | example drops `swap-in` | exit job swap assertion | hosted |
| Refused-poll observer (AC7) | bucket arm pushes no outcome | `RateRefused` assertion | local |
| D07 A suppression (AC8) | suppression skipped | inverted FW-10 vector | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- build-test-fixtures
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-wasm-host --test wit_corpus
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --features maos-bin/wasm-host --test spawned_lifecycle_20_9 --test spawned_swap_20_9 --test wasm_form_admission_17_3b --test hook_pump_17_4
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-iac
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test schedule_watchdog_fr26 --test routed_halt_swap_20_3b   # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib                                                              # L2
for g in check-equiv-fixture-provenance abi-diff check-stability-matrix check-manifest-schema-version check-corpus check-env-contract check-service-boundary check-kernel-baseline check-dev-record-completeness check-epic-close-coherence check-exit-commands check-decision-register; do env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- $g; done
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- gen-abi-docs --check
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), a Security reading on AC2.3, AC4.1–4.3 and AC8, and Test
Infrastructure (glm-5.3). The focused suites are each touched crate plus the WIT widening: every guest and fixture, `wit_corpus`,
`check-wasm-form-equiv`, and the manifest readers of `[[schedule]]` (`gate_manifest`, `load_bundle_from_file`,
`spirit_boundary_invariants`). L2 adds `cargo test -p maos-kernel-core --lib`. Reviewers read the plant record and never re-run
plants except for a disputed finding.

### Previous-story intelligence (20-8, 19-5, 18-5, 17-4, 17-3c, 20-3c, 20-11)

- 20-8 (created today): the 2.3.0 skills bridge prototype (`wit`), the in-export rule, and the single declaration writer. Its creator warned that 20-9 inherits ≤ 27–51 kernel lines. Measured: 27 before 20-9; R takes 21.
- 19-5: removed the kernel cold-swap arm, gates targets at the door, and labelled spawned forms (AC6). Its factory note: the pid is resolved before `create` and kept (`main.rs:3404-3412`).
- 18-5: the closest WIT-minor precedent (context import, version-selected legacy bindings, ratification); maos-wasm-host landed +157.
- 17-4: the first production hook callers. "Timed-out hook bodies are not joinable" (`17-4…md:341`), which is why `WorkerSpirit` must self-bound its wait.
- 17-3c: turn-scoped leases and one active export per session; spawned sessions revoke all pid tokens at finalize.
- 20-3c: I14 carry (K1p); the same-file deferred rows on the swap precheck (`deferred-work.md:28`, `:62`) stay ownerless and are not this story's charter.
- 20-11: `forget_spirit` in the same kernel file; 20-1a: hosted faults local runs missed, so use release builds for AC6.

### Git intelligence

`8c809f9c`/`16c35ef0` (20-10 hosted corrections; bin 34191, aggregate 192519), `ac1fc12a` (20-1a/20-7 cold-swap oracle). The
watchdog last changed at 17-9 (FW-10 mint-after-bucket) and 20-7. `coordinator.rs` changed at 20-3c K1p (`a41a143f`), and the
successor factory at 19-5 (`fcbec908`). The uncommitted 20-4 L1 edits are in the working tree (Dev204L1). 20-8, 20-13 and 20-14
were created the same day and are not landed.

### Library / framework

No new crate. `wasmtime` 48.0.8 (locked): `bindgen!` per world. Sharing interface types across two worlds needs the `with:`
option, and `include` in a WIT world must be verified at T0 against the locked version [INFERENCE]. `ciborium` 0.2.2 (wire
tags). TS: `@bytecodealliance/jco` 1.35.0, `componentize-js` 0.23.0 (`examples/example-spirit-ts/package.json`); exporting an
interface from a TS component runs on the hosted leg only.

### References

E20@HEAD:7,55-61,82,264-270,362-370; PF `epic-20-preflight-2026-10-30.md:83,127,141,244-245,291`; tracker `sprint-status.yaml:309`;
`deferred-work.md:1260,1275`; `routing-audit-2026-10-22.md:20,23,173,182`; `requirements-inventory.md:53,566,594`; PRD
`functional-requirements.md:39,65,69`; `19-5-cross-form-upgrade-and-cold-swap.md:30,137,184-188`;
`17-4-hooks-observer-and-activity-corpus.md:53,71,135,341`; `20-8-manifest-contract-completion.md:176-197`; `docs/invariants/I14.md`;
ADR-002, ADR-013, ADR-017, ADR-020, ADR-030, ADR-031, ADR-060, ADR-069.

## Dev Agent Record

### Agent Model Used

**Model:** anthropic/claude-sonnet-5-5 (dev agents `Dev209L1`, `Dev209L1b`, `Dev209L1c`).

**Landing 1 (kernel-free: AC1–AC5; AC6 hosted-only).** Models: `Dev209L1`, `Dev209L1b` (parked WIP), `Dev209L1c` (the landing): anthropic/claude-sonnet-5-5. Base `fc7373f0`; nothing committed. Evidence: `20-9-spawned-form-lifecycle-hooks-evidence/landing-1/evidence.md` (witness table with raw before/plant logs, measurements, gates, limitations); hosted recipe `20-9-spawned-form-lifecycle-hooks-evidence/ci/ci-20-9-verify.yml` (branch-only, unverified).

**What landed.** `maos:spirit@2.4.0` (`interface lifecycle`, `world spirit-lifecycle`; ratification `20-9-wit-spirit-2.4.0`; the four provenance-pinned components re-issued, two frozen 2.2/2.3 echo guests kept under `tests/fixtures/wasm/frozen/`). `maos-wasm-host`: `lifecycle_wire`, binding selection by export presence, `Unsupported` for a guest without the export, `guests/lifecycle-probe`. `maos-bin`: `spawned_lifecycle.rs` (`LifecycleLink`, `SwapHandles`/`SwapSlot` weak handle, `spawned_successor`, `HostedSessions`), `WorkerSpirit` forwarding for nine hooks with a typed panic on `Declined`/`Failed`/ended/deadline, session lifecycle turns (a queued frame turn first; a probe at start; `snapshot` fences the predecessor), child hand-over and `retire` (no `revoke_all_for_pid`), `swap_reverted_after_retire`, the door's cold swap through `admit_spawned`, `[[schedule]]` carried at admission, `swap_form_mismatch` / `swap_capability_widened` / `spawned_swap_unsupported` / `spawned_swap_not_ready`, the artifact directory (`origin`) carried through hot and cold swap, telemetry for a spawned form only on an explicit `on_telemetry_event` listing. TS template and example export `lifecycle`; EN/KO cookbook pages updated.

**Corrections to this story's premises (observed).**
- AC1.3 "At HEAD the guest fails to instantiate": false. The 2.4.0 component `include`s `spirit`, so HEAD instantiates it (`landing-1/failing-before-head-probe-ac1.log`); what HEAD lacks is any path that calls a lifecycle export. The failing-before is by code (`failing-before-head-code.log`) plus the plant.
- AC4.5 / AC5.3 failing-before captured against real HEAD binaries through the door with a frozen 2.3 component (`failing-before-head-door-echo-2_3.log`): hot swap `spawned_surface_unsupported: hot-swap successor … (HTTP 500)`, cold swap `cold_swap_unrestorable … (HTTP 409)`.
- AC6.2 plant "the example drops its swap-in export": `lifecycle` is exported whole or not at all, so the recipe componentizes the same build against the plain `spirit` world (no lifecycle export) and expects `spawned_swap_unsupported`.
- The FR11 archive is not under `MAOS_HOME` (`default_archive_dir` resolves `$XDG_DATA_HOME`/`$HOME`); the bin vectors set `MAOS_ARCHIVE_DIR` per process. The first WIP runs wrote `lifecycle-probe` into the operator's real archive; removed.
- Existing tests updated for the new truth (not weakened): `erasure_live_20_11` (`[[schedule]]` is now carried at admission, so `schedule.fire:daily` occurs while the Spirit lives; the assertion is now "never again after the uninstall completed"), `wasm_skill_authoring_20_8` (2.4.0), `wit_copies_17_3b` (the world-named declaration module of `spirit-lifecycle`), `cohort_daemon_smoke_13_5c` (39 → 40 scanned files: `spawned_lifecycle.rs`), `hook_pump_17_4` (`a_spawned_form_registers_but_telemetry_needs_an_explicit_listing`), `wasm_form_admission_17_3b`.

**Observed verification (exact commands and results are in the evidence).** Plants AC1–AC5 each red (raw logs `plant-ac1..5.log`), restored byte-for-byte; every focused vector green at the end; gates exit 0 (`kloc-check`, `check-kernel-baseline`, `check-dev-record-completeness`, `check-epic-close-coherence`, `check-exit-commands`, `check-decision-register`, `check-env-contract`, `check-equiv-fixture-provenance`, `check-manifest-schema-version`, `gen-abi-docs --check`, `stability-matrix --check`, `check-wasm-form-equiv`, `invariant-lock --changed-files`, `check-fkcs`); `clippy --workspace --all-targets` exit 0 and `clippy-ratchet` 1327 / 867 (baseline 1383 / 897); workspace nextest and the feature-on `maos-bin` run: see `evidence.md` §Workspace. `prepush` was not run (it wraps the workspace run and the gates already run individually).

**kloc (exact, after `cargo fmt --all`).** maos-bin 36234 → 37350, maos-wasm-host 2538 → 3103, xtask 44935 → 44936, aggregate 199198 → 200880 (`_aggregate_hardfail` 200881); all inside the re-based D-20-9-16 A ceilings 38422 / 3348 / 44980 / 202209. Kernel, domain and iac unchanged; no kernel line.

**Not done (open).** AC6 and every hosted-only leg (recipe unverified); L2 (AC7, AC8, AC9 closure: `deferred-work.md:1260` / `:1275`, kernel pins, FLAG landed figure, ADR-013/-030 FW-10 sentences, `docs/invariants/I14.md`: not edited in L1, ADR-037 invariant-lock would demand a phase commitment; the routed FR53 witness named in this story is `spawned_swap_20_9::a_hot_swap_keeps_the_pid_tokens_memory_and_state_and_waits_for_the_active_turn`, which asserts the carried halt, and in `tests/coverage-matrix.yaml`).

### Landing 1 review corrections (2026-11-11, `Dev209L1d`, anthropic/claude-sonnet-5-5)

Two independent reviews of `cdf3937d` (GPT: 6 P1 + 1 P2, "incorrect"; GLM-5.3 security: 1 Medium, 4 Low, 1 Info) are persisted verbatim under `20-9-spawned-form-lifecycle-hooks-evidence/landing-1/review/`. Evidence: `landing-1/review-corrections/evidence.md` (witness table, red-before logs from a scratch `git archive cdf3937d` under `red/`, plants under `plants/`, baseline/final daemon witness for AC1-AC3, pins, gates).

- **GPT #1** `maos run` of a component never reached the serving tail that starts the idle/schedule/consolidate callers: `LifecycleCallers` starts them before the session is awaited and the run arm joins them before supervision stops (AST guard extended). Real-daemon vector `spawned_door_20_9::a_component_run_by_maos_run_receives_the_idle_schedule_and_consolidate_triggers`. The same vector found that the daemon's only `on_idle` fired during startup, before the session could take it, and was lost: spawned cores now expect their session (D-20-9-18).
- **GPT #2** installed-component roots arm the same swap handles (`SwapHandles::arm`): `spawned_installed_20_9` boots a published and installed component under `maos shell` and upgrades it in place through the door.
- **GPT #3** `admit_spawned` returns only after the component answered its probe; a failed start is unwound (session stopped and joined, SCB unloaded) and refused, so the door restores the predecessor: door vector `a_cold_swap_to_an_existing_invalid_component_is_reverted_and_the_predecessor_serves` plus an in-process twin.
- **GPT #4** `swap_hook_disabled` before anything is fenced. **#5** hook/session race on `CallState`: a snapshot answered after its caller gave up fences nothing. **#6** prewarm deadline checked on every loop turn, late replies refused (hostile-twin `stderr-flood`/`probe-flood` through the real factory). **#7** a committed successor awaiting adoption queues its hooks.
- **GLM F1** concurrent telemetry fan-out, wall `min(95% cap, 5 s)`, three 80%+ deliveries quarantine, a trigger that cannot start in half its budget is dropped without panic or strike (D-20-9-17, D-20-9-18). **F2** `resolve_artifact` containment (D-20-9-19, operator-ruled). **F5** `MAOS_ARCHIVE_DIR` registered. **F3, F4, F5 second half, F6** are rule-10 rows with dispositions in `deferred-work.md` (they need design rulings); F4's second half is open, not closed: two back-to-back hot swaps end with `swap_reverted_after_retire`.
- **Probe guest.** `sleep-in NAME SECONDS`; each export's `recall` import asks for an export-specific page size, which the log's `log.recall` row records, so a daemon's own Transparency Log names the triggers that reached the guest. (A first attempt, a stderr line per export, crashed the runner under the T2 seccomp filter and was removed.)
- **AC1-AC3 failing-before.** `landing-1/review-corrections/baseline-daemon-triggers.log` (a real `maos run` of the probe under fast clocks at the base `fc7373f0`): 0 `log.recall` rows and 0 `schedule.fire:tick` rows; `final-daemon-triggers.log`: 379 and 60. The AC1.3 premise is reworded (D-20-9-20). The wire/runner half of AC1 keeps its code-reading failing-before plus the plant (no baseline-compatible executable witness: the lifecycle wire does not exist at the base).
- **kloc (exact, after `cargo fmt --all`).** maos-bin 37350 → 37633 (+283), maos-wasm-host 3103 → 3137 (+34), xtask 44936 (0), aggregate 200880 → 201197; `_aggregate_hardfail` 201198. Inside 38422 / 3348 / 44980 / 202209. Kernel 21042 unchanged.

### Landing 1 correction round 3 (2026-11-11, `Dev209L1d`, anthropic/claude-sonnet-5-5)

Reviews of the corrections (`8c669a21`; GPT r2: 2 P1 + 3 P2 "incorrect"; GLM-5.3 r2: 3 Medium, 2 Low, 2 Info) are persisted verbatim as `landing-1/review/{gpt-review-r2,security-r2}.md`. Evidence: `landing-1/review-corrections/round3/evidence.md` (red logs from a scratch `git archive c8e5d035`, plants, observed runs).

- **GPT r2 #1** the swap attempt, not the blocking call, owns the fence (D-20-9-22): a snapshot reply fences only while the attempt is live, and the successor's drop ends it and unfences under the same lock. Witness: the dispatcher's timeout (the predecessor SCB's budget, 2 s) fires before the link's (30 s), the reply arrives between them.
- **GPT r2 #2** FR11 archive record written only after the component answered its probe (`load_admit_start_unarchived`, D-20-9-23): cross-major successor with a permitted migration that never starts, in process and through the door → `cold_swap_reverted`, predecessor serving.
- **GPT r2 #3, GLM R2** a spawned telemetry/consolidate trigger gives up for contention inside the pump's wall and a per-pid busy counter keeps that drop out of the deliveries and the clean runs (D-20-9-21): a 6 s frame turn holding a 30 s component no longer quarantines the subscriber or runs the event late.
- **GPT r2 #4, GLM R3** the near-budget history is telemetry-only, 3 slow of the last 5, a quick delivery or a consolidation between does not erase it.
- **GPT r2 #5, GLM R7** `unwind_admitted` unregisters the pid from the pump (grant row revoked at once).
- **GLM R1** `on_consolidate` (the watchdog's pass, forget notices, the late-notice sweep) runs side by side under `min(95% cap, 30 s)`; the watchdog no longer joins a pass inside its tick (a slow subscriber is skipped until its pass ends, shutdown joins the passes); the privacy bypass of quarantine is kept and bounded; a wall overrun's row names the wall (R4 wording).
- **Residuals (rule-10 rows):** artifact open-time TOCTOU (R5), `authorize_principal_writes` release (R6, kernel, needs a measured FLAG-Winston grant at the final kernel landing), subscription-row TL writes under the registry mutex, the global pacing floor and the compiled-in Observer wall policy (R3/R4), and the narrower teardown claim below.
- **Claim narrowed.** Earlier text said the run arm's lifecycle callers are "joined on teardown". It holds for the normal success and error returns that reach the shared teardown. `LifecycleCallers` has no unwind cleanup (a panic or early return before its `shutdown().await` leaves the watchdogs on their detached token), its idle/schedule joins give up after 5 s, and the AST guard recognizes a syntactic `callers.shutdown()`; it does not prove awaited, path-complete ownership.
- **kloc (exact, after `cargo fmt --all`).** maos-bin 37633 → 37832 (+199), maos-wasm-host 3137, xtask 44936, clean aggregate 201396 (`_aggregate_hardfail` 201397); the tree also holds the operator's untracked `spikes/` (366 lines at the last measure, `(unknown:spikes)` over its own budget), so the retained `kloc-check` log is red for that reason alone. Inside 38422 / 3348 / 44980 / 202209.

### Landing 1 correction round 4 (2026-11-11, `Dev209L1d`, anthropic/claude-sonnet-5-5)

Reviews of the round-3 corrections (`f56959c2`; GPT r3: 2 P2 "incorrect"; GLM-5.3 r3: 1 High, 2 Medium, 1 Low, 2 Info) are persisted verbatim as `landing-1/review/{gpt-review-r3,security-r3}.md`. Base for the failing-before runs: a scratch `git archive HEAD` (`0e455c4e` = round 3 + the 20-5 L1 candidate, whose only `maos-bin` source change is `worker_spawn.rs` and which edits no file of this story) outside the repo and the shared target. Evidence: `landing-1/review-corrections/round4/evidence.md`.

- **GPT r3 #1** a busy drop is attributed to the invocation that suffered it (D-20-9-24): each pump dispatch registers a record (`invocation.rs`) that the link attaches its call to and the session starts; the shared per-pid counter is gone, which also closes GLM F5 (no ledger outlives a dispatch). Witness: a forget notice that runs 4 s successfully while a queued telemetry event is dropped keeps its purge delivered.
- **GPT r3 #2** the wall ends a dispatch's accounting and never its execution: the dispatch runs as its own task, an overrun's task is parked and joined by the watchdog's exit and `HookPump::shutdown` before anything is unloaded; a spawned hook is cut loose by its link at `queue share + wall + 2 s`. Witnesses: a compiled-in consolidation of 9.75 s under a 10 s budget (wall 9.5 s) is joined by shutdown; a 25 s telemetry export under a 60 s budget does not hold shutdown past the wall plus slack.
- **GPT r3 closure** a snapshot carries the swap attempt it was taken for and fences only that attempt (D-20-9-26). Witness: a snapshot of a refused swap still running when the retry installs its attempt fences nothing and the retry swaps.
- **GLM r3 F1** every successor-side stage (pre-warm `Ready`, the probe, `migrate`, `swap-in`, the cold-swap readiness wait) is bounded by `min(declared budget, 60 s)`; a failed inline exchange drops the successor's child and releases the predecessor at once (D-20-9-25). Witnesses: a successor declaring a 600 s budget that never becomes `Ready`, and one that never answers its probe, are each refused at the 60 s wall with their child gone and the predecessor serving. The inline-error release has no witness of its own (the probe guest cannot be made to hang inside `migrate`/`swap-in`); it shares `Prewarm::exchange` with the probe vector.
- **GLM r3 F4** the wall is counted from when the export starts (`Invocation::deadline`), so a trigger that queued behind a frame turn is not charged for the wait; the cookbook sentence is now true. Witness: 2 s queued plus 4 s run inside a 5 s wall.
- **Not fixed (rows):** GLM F2 (kernel `coordinator.rs` `expect` window; needs a measured FLAG-Winston grant at the final kernel landing, row R touches the coordinator), F3 (kernel `archive.rs` atomic write and the two-sided order), F6 (`redistill_results` slot pid), the per-spirit upgrade lock (F1's second half), and what the witnesses do not prove.
- **Claims narrowed.** The 30 s consolidate/forget wall is witnessed only at short holds (3-4 s notices; 9.75 s under a 10 s budget, whose wall is 9.5 s); "ordinary consolidation advances its timestamp" is asserted through the forget purge (the watchdog pass and the notice share `fire_walled` and its record), not by reading the private timestamp; a compiled-in overrun is joined for as long as the kernel's dispatcher lets it run (its budget), only a spawned hook is cut loose at the wall; the kernel dispatcher's own cap timer still counts queue wait. Round 3's "10 plants" were nine plant runs carrying ten textual replacements; this round's are stated the same way (nine runs, ten replacements).
- **kloc (exact, after `cargo fmt --all`).** maos-bin 37833 → 38019 (+186; the new `invocation.rs` is 74 of them), maos-wasm-host 3137, xtask 45315, clean aggregate 202000 (`_aggregate_hardfail` 202001); the tree also holds the operator's untracked `spikes/` and `*gpui*` files (+222 aggregate lines, `(unknown:spikes)` 366 over its own budget), so the retained `kloc-check` log is red for that reason alone. Inside 38422 / 3348 / 44980 / 202209.

### Landing 1 correction round 5, the last (2026-11-11, `Dev209L1d`, anthropic/claude-sonnet-5-5)

Reviews of the round-4 corrections (`fba6af2a`; GPT r4: 3 P2 "incorrect"; GLM-5.3 r4: 1 High, 1 Medium, 1 Low) are persisted verbatim as `landing-1/review/{gpt-review-r4,security-r4}.md`. Base for the failing-before runs: a scratch `git archive HEAD` (`e322c7ad` = round 4 plus the 20-5 corrections) outside the repo and the shared target. Evidence: `landing-1/review-corrections/round5/evidence.md`.

- **GPT r4 #1, GLM r4 F3** identical concurrent batches no longer share an invocation record (D-20-9-27): content cannot identify an invocation and the kernel dispatcher has no channel for a token, so two live dispatches never hold the same key and identical batches take turns (a claim-the-first-free-record scheme would swap the two calls' records whenever their link calls arrive in the other order). Witness: two byte-identical forget notices on one subscriber, the first holding the session past the second's queue allowance, each report matches what ran.
- **GPT r4 #2** a trigger that cannot enter the session's one queue slot is a dropped invocation (`cancel_queued` on the full-channel branch). Witness: a frame turn holds the session, a consolidation holds the slot, a telemetry event is neither run nor counted delivered.
- **GPT r4 #3** parked overruns are reaped on every parking: what the pump keeps is what still runs. Witness: four finished wall overruns without shutdown leave at most one entry (`HookPump::parked()`).
- **GLM r4 F1 (High)** a swap attempt owns its token (D-20-9-28): the successor's link holds the attempt it began, a new attempt ends and unfences the one it replaces, and only the end of the INSTALLED token releases the fence, so the late drop of an older successor object (kept alive by a dispatch, or by the pump's cached target) never steals a newer attempt. The pump no longer caches a dispatch SCB across dispatches (D-20-9-27 (4)), and a guest refusal (`declined`/`failed`) releases the attempt at once like a transport error.
- **GLM r4 F2** one per-attempt deadline of `ATTEMPT_WALL = 90 s` (set when the factory starts) bounds launch, `Ready`, the probe and every `migrate`/`swap-in` exchange, where round 4 gave each stage its own 60 s. The wait for the predecessor's active turn counts against it. Witness: a successor `Ready` after 50 s and then silent is refused at 90 s, not at about 140 s.
- **Closure notes recorded as rows (7):** a spawned hook's cancellation cancels the WAIT, not the running export (the child runs on until it replies or the session deadline ends the session), and an inline `write_frame` has no wall against stdin backpressure. Also rows: the idle watchdog's sequential `fire_on_idle` (kernel), the unbounded `upgrade_lock` wait and the missing per-spirit failed-attempt backoff (door), and what the witnesses do not prove.
- **Claims narrowed.** The late-drop theft is witnessed on the link's attempt API (`begin_attempt`/`end_attempt`) and by plants on its two guards, not through a pinned successor object (the probe guest cannot be told to decline a `migrate`/`swap-in`, and a pin cannot be arranged deterministically); the red-before of that vector used a scratch shim that puts the base's attempt-blind statements behind the new names, and the vector's first assertion (the new attempt ends the one it replaces) is red there before the stale-end assertion is reached. The prompt release on a guest refusal and the per-dispatch SCB are unwitnessed and unmeasured.
- **kloc (exact, after `cargo fmt --all`).** maos-bin 38019 → 38061 (+42), maos-wasm-host 3137 → 3143 (+6: the `hostile-twin` `slow-ready` mode), xtask 45333, clean aggregate 202066 (`_aggregate_hardfail` 202067); the tree also holds the operator's untracked `spikes/` and `*gpui*` files (+222 aggregate lines, `(unknown:spikes)` 366 over its own budget), so the retained `kloc-check` log is red for that reason alone. Inside bin 38422 / wasm-host 3348 / aggregate 202125 (20-5's binding ceiling).

### File List

Review corrections add: `crates/maos-bin/src/{spawned_lifecycle,spirit_session,supervision,hook_pump,admission,env_contract,main}.rs`; `crates/maos-wasm-host/guests/{lifecycle-probe,equiv-fixture/native-twin}`; tests: new `spawned_installed_20_9.rs`, updated `spawned_{swap,lifecycle,door}_20_9.rs`, `hook_pump_17_4.rs`, `root_shutdown_unload_16_3.rs`, `support/{spawned_world_20_9,yank_audit}.rs`; `.config/nextest.toml`, `xtask/kloc.toml`, the two cookbook pages, `deferred-work.md`, `ci-20-9-verify.yml`, `landing-1/{review,review-corrections}/*`.

- Creation: this story; `20-9-flag-winston-decision.md`; `20-9-spawned-form-lifecycle-hooks-evidence/creation/*`; records per `creation/records-owed.md`.
- WIT / ABI: `wit/spirit.wit`; `xtask/abi-ratifications.toml`; `tests/fixtures/wasm/{echo_spirit_component,equiv_*}.wasm`, `equiv-fixtures.provenance.toml`, `tests/fixtures/wasm/frozen/*`; `xtask/src/build_test_fixtures.rs`.
- maos-wasm-host: `src/{lib,runner,wit_guest}.rs`, new `src/lifecycle_wire.rs`, new `guests/lifecycle-probe/*`, `tests/wit_corpus.rs`, new `tests/lifecycle_runner_20_9.rs`, `tests/wit_copies_17_3b.rs`.
- maos-bin: new `src/spawned_lifecycle.rs`; `src/{admission,hook_pump,lib,main,migration_plan,operator_door,spirit_session,supervision}.rs`; tests: new `spawned_lifecycle_20_9.rs`, `spawned_swap_20_9.rs`, `spawned_door_20_9.rs`, `support/spawned_world_20_9.rs`; updated `hook_pump_17_4`, `live_escape_anomaly_17_10`, `participant_recall_17_8`, `recall_17_3d`, `spawned_scope_17_3c`, `wasm_form_admission_17_3b`, `wasm_skill_authoring_20_8`, `cohort_daemon_smoke_13_5c`, `erasure_live_20_11`.
- TS and docs: `templates/spirit-ts/*`, `examples/example-spirit-ts/*` (WIT copy, `package.json`, `src/index.ts`, tests, READMEs); `docs-site/docs/cookbook/{lifecycle-hooks,wasm-component-spirit}.md` and the KO pair.
- Records and gates: `.config/nextest.toml` (three serial binaries), `xtask/kloc.toml`, `tests/coverage-matrix.yaml` (FR10, FR26, FR55), `deferred-work.md` (§20-9 L1), `sprint-status.yaml`, `20-9-flag-winston-decision.md`, `20-9-spawned-form-lifecycle-hooks-evidence/{landing-1,ci}/*`.

### Change Log

- 2026-11-03: Operator (ask `d_batch4`) ruled D-20-9-01…16 all as recommended A. Grants are ruled, binding at dev start after remeasurement/rebase. D-20-9-15 A sets targeted STOP 21,230 tokei for 20-9 L2; D-20-14-12 C (ask `d2014_12`) then sets JOINT targeted STOP 21,250 shared by 20-9 L2 + 20-14 L2, whose combined cumulative must not exceed 21,250. Shared STOP remains 21,209 for everything else; ADR-069 alarm 22,000 untouched.
- 2026-11-11: **L1 review corrections** (Dev209L1d): all seven GPT findings, GLM F1 and F2 fixed with real daemon/session witnesses red at `cdf3937d` and red under plants; GLM F3/F4/F5b/F6 dispositioned in `deferred-work.md`; D-20-9-17…20 added; AC1.3 reworded; pins re-based (bin 37633, wasm-host 3137, aggregate hardfail 201198). Hosted legs and L2 still open.
- 2026-11-11: **L1 correction round 3** (Dev209L1d): GPT r2 five findings and GLM R1-R3 fixed with witnesses red at `c8e5d035` and red under plants; GLM R4/R5/R6 and the TL-write lock dispositioned in `deferred-work.md`; D-20-9-21…23 added; teardown claim narrowed; pins bin 37832, aggregate hardfail 201397. Hosted legs and L2 still open.
- 2026-11-11: **L1 correction round 4, the last** (Dev209L1d): GPT r3 two P2 and its snapshot-owner closure note, GLM r3 F1, F4 and F5 fixed with witnesses red at `0e455c4e` (HEAD, round 3 included) and red under plants; GLM r3 F2, F3, F6 and the per-spirit upgrade lock dispositioned in `deferred-work.md`; D-20-9-24…26 added; the 30 s wall is witnessed at short holds only; pins bin 38019, aggregate hardfail 202001 (clean 202000). Hosted legs and L2 still open.
- 2026-11-11: **L1 correction round 5, the last** (Dev209L1d): GPT r4 #1-#3 and GLM r4 F1-F3 fixed with witnesses red at `e322c7ad` and red under plants; the idle watchdog (kernel), the cancel-waits-not-the-export and `write_frame` closure notes, the unbounded `upgrade_lock` wait and what the witnesses do not prove dispositioned in `deferred-work.md`; D-20-9-27 and D-20-9-28 added (D-20-9-28 supersedes the per-stage 60 s of D-20-9-25); pins bin 38061, wasm-host 3143, aggregate hardfail 202067 (clean 202066). Hosted legs and L2 still open.

- 2026-11-11: **Hosted run 42987393747 @ `459a606a` follow-ups** (Dev20Hosted; operator rulings 2026-11-11): (1) D-20-9-29, ask `tier` = allow: `sonnet-5-5` joins `FRONTIER_FAMILIES` (`check-dev-model-tier` red naming this story reproduced locally, green after; `xtask` test `sonnet_5_5_is_allowlisted_and_neighbouring_models_are_not` red at `459a606a`, haiku-5-5 and sonnet-4-5 stay refused); (2) `wasm-host-tests` leg: the `ci-test-producer` `--only` list lacked `lifecycle_probe.wasm`, so `lifecycle_runner_20_9` found no manifest entry; added, and `producer_builds_every_fixture_the_wasm_host_tests_request` (xtask/tests/ci_shape_18_0.rs) pins every manifest fixture the wasm-host tests request, red at `459a606a`; (3) `check-test-budget`: `maos-bin::hook_pump_17_4` Σ 44.520 s over 33 tests (was 15.515 s over 29) against the 30 s default, exception 56 s owned by this story. The four round 3-5 vectors (`:354`, `:389`, `:1228`, `:1287` sleeps, about 29.5 s) are real wall-clock overruns against pump walls that are fractions of a whole-second `time_cap_seconds`, so none was shortened. Red before and green after with the hosted junit (artifact `workspace-test-suite-junit`, run 42987393747). Pins xtask 45379, aggregate hardfail 202113 (clean 202112). Evidence: `20-9-spawned-form-lifecycle-hooks-evidence/hosted-followup-42987393747/evidence.md`. Hosted legs and L2 still open.

- 2026-11-03: Story created at `8c809f9c` (ready-for-dev; D-20-9-01…16 PROPOSED). Inventory measured: spawned SCBs already receive every kernel hook as a `WorkerSpirit` no-op; 17-4's callers exclude spawned pids; `[[schedule]]` is never carried at admission for any form; the spawned successor factory refuses; spawned cold swap refuses before unload; the refused poll is silent and the untokened fire proceeds. Kernel priced by reverted prototypes: R = KD07A-lean + KO-poll-x +16 / +21 recommended (KD07B-lean, KD07A-inline, KO-poll, KO-count, KO-row, KW priced); cumulative 21203 ≤ 21209 measured, ×2 ceiling 21224 above → D-20-9-15. Out-of-kernel measured: iac +39, bin +14 (schedules). E20 §20-9 cites corrected in place.

- 2026-11-11: **L1 (kernel-free) implemented at `fc7373f0`, uncommitted** (Dev209L1c, completing Dev209L1/L1b's parked WIP): AC1–AC5 with plants observed red for each, failing-before captured against real HEAD binaries through the door; kloc bin 37350 / wasm-host 3103 / xtask 44936 / aggregate 200880 (hardfail 200881) inside the re-based ceilings; no kernel line; hosted AC6 and L2 (AC7, AC8, AC9) open; tracker `in-progress`. Premise corrections and review fixes (FR11 archive isolation, hook wait races, token assertion ordering, native test twins answering the lifecycle probe) are in the Dev Agent Record and `landing-1/evidence.md`.
