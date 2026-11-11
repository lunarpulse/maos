---
story_id: '20.11'
story_key: 20-11-erasure-inventory-closure
created: 2026-11-02
context_commit: a8af3215
baseline_commit: 93d39cc7ca185f513dee2d97bed2058f11554d9a
security_sensitive: true
kernel_grant: >-
  RULED 2026-11-02 (ask s2011; KS) and 2026-11-04 (ask halt_grant; D-20-11-11 A = O4).
  Measured from 26841 physical / 21042 tokei: KS +10/+6 and O4 +16/+12, bundle +26/+18 = 26867/21060.
  Binding D02 A ×2 ceiling 26893 physical / 21078 tokei; O4 allowance +32/+24 adds to KS's +20/+12.
  Grant binds only at the final kernel landing after re-measurement/re-base; KS + O4 and AC3 land last.
  Shared cumulative measured STOP 21250 for the whole ledger at each landing (ask halt_grant);
  expected cumulative 21203 + 12 = 21215; ADR-069 alarm 22000 / envelope 23000 unchanged.
  Landing 2 (2026-11-11): re-based onto 26854/21051; measured KS + O4 = +26/+18 = grant (26880/21069), cumulative 21069 <= STOP 21250.
kloc_grant: >-
  RULED asks s2011 and halt_grant; D02 A ×2 measured / ×3 unmeasured.
  L1 review re-based 2026-11-05 onto 93d39cc7: kernel pins 26841/21042 unchanged; maos-iac 8551,
  maos-audit 7785, maos-bin 34823, maos-registry 4322, domain 10107, xtask 44673,
  aggregate 194099 / hardfail 194100. Retain the stricter assigned ceilings: IAC 8630,
  audit 8261, bin L1 35704, aggregate 195404; final-kernel ceilings unchanged.
  Registry ruling (Main, 2026-11-05): review-correction re-base: real production growth
  (streaming-tar recovery + rollback through proof/I10); 4322 -> 4412 after fmt (+90),
  exact review re-base4412 within the authorized cap4430; former4368 exceeded by44.
  Hosted-followup handoff fd4f60f1: bin34914/IAC8622/audit8174/registry4412/xtask44676,
  aggregate194743/hardfail194744. Latest exact formatted pins: bin35102 (+188), IAC8625 (+3),
  audit8174, registry4412, xtask44676; aggregate194934/hardfail194935 (+191).
  Every other crate0; kernel26841/21042 and domain10107 unchanged.
  IAC's unchanged504-code-line unit accounting transfer is not a ceiling relief:
  src8551 -> 8625, tests2315 -> 3640; no-transfer src9129 (+578).
  Counterfactual aggregate195438 exceeds195404 by34; actual counted194934 remains within
  the unchanged aggregate ceiling. No further source-to-tests accounting transfer or ceiling grant is introduced.
  L1 consumes only these non-kernel allowances; no kernel grant is consumed.
  Landing 2 (2026-11-11, orchestrator-adjudicated overdraft, disclosed to the operator, D02 A): audit ceiling 8261 -> 8313
  (pin 8227 -> 8313, +86 measured); bin pin 38061 -> 38202 (+141; story bin ceilings are stale-absolute, live ceiling = measured pin
  plus own remaining allowance); aggregate hardfail 202122 -> 202371; kernel 26880/21069 and domain 10215 inside the grant.
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): an attesting control (a signed proof-of-erasure, rule 11), cross-Spirit isolation (pid reuse across boots, legal holds), concurrency (EC-13 lock windows, forget/hold serialization) and, if ruled, a kernel FLAG-Winston row. Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC2.4, AC3, AC5.3 and the proof format (D-20-11-09), Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.11: Erasure inventory closure — per-Spirit proof of erasure

Status: in-progress

<!-- Created 2026-11-02 at a8af3215 (main; origin 47631284) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30). D-20-11-01…10 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran: cites re-read at HEAD (`creation/cites-a8af3215.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); prototypes saved as diffs, never applied. Stale epic cites are corrected here and owed to the epic (`creation/records-owed.md` §A). -->

**Status rationale — ready-for-dev.** Prerequisite 20-3c is `done` (PF:129). Nothing needs a machine the repository
lacks: the hermetic harness and door cover the local categories; only AC6 (a running spawned Spirit) needs the
hosted T3 substrate (`provision-t3-test-substrate`). The ruled final kernel landing is mandatory for AC3 and
watchdog cleanup; L1 remains kernel-free (ask `halt_landing`, 2026-11-04).

**T0 STOP history — 2026-11-04, HEAD `73e7afbc` (Dev2011).** Measurement 2 reproduced the unreceipted
second-drain race with the real scheduler and `invoke_halt`: an `Unloaded` SCB accepts a durable halt which
`scheduler_loop.rs:574` discards without a terminal receipt. No L1 implementation or status transition occurred
in that preflight. **The operator subsequently ruled D-20-11-11 A / O4 (`halt_grant`) and moved AC3 to the
final kernel landing (`halt_landing`) on 2026-11-04.** L1 AC1, AC2 and AC4–AC7 may proceed kernel-free once
the orchestrator hands over the cargo slot; it currently belongs to Dev2010. The observed red remains evidence,
not a repaired claim. See [T0 STOP evidence](20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md).

> **The capability:** *`maosctl uninstall <spirit>` signs only what it observed: every per-Spirit store FR65 names is
> enumerated from where it lives, removed or honestly labelled, and the signed bundle proves it — including for a
> Spirit that never touched a principal.*

**Closes:**
- Epic-20 §20-11 (E20@HEAD:279-284) as amended 2026-10-30 (D01 A; PF §3.5): epic AC1 → AC1–AC4, AC7; epic AC2 → AC5, AC6.
- FR65 (`prd/functional-requirements.md:108`) — intent lineage, pending halts, scheduled invocations; owner row `deferred-work.md:75`.
- NFR-Aud-12's "proof covers the complete erasure inventory" (owner row `deferred-work.md:78`) to the extent D-20-11-09/10 rule.
- EC-13 large-log forget scans (`deferred-work.md:1331`, from 18-6 review).

**Does NOT close, and says so:**
- **16-4 `maos purge`** (done): product data removal, not per-Spirit; it writes no proof and deletes the proofs
  directory with the product (`purge.rs:56,106`) — routed, not fixed here (`creation/records-owed.md` §C R1).
- **20-10 purge residue removal** (backlog): owned T3 containers/cgroups at purge. This story's T3 leg only observes
  the existing per-Spirit teardown (`operator_door.rs:609`, 20-7 K1w/K2r); it adds no container removal.
- Audit truth (20-3c, done): no second erasure implementation and no second halt-receipt writer (PF:129, PF:212).
- 20-14's cost frames: never scrubbed by uninstall; they stay readable (PF:129).
- Live-key or human legs: none needed.

## Story

As **the operator who uninstalls a Spirit, and the auditor who verifies the receipt off-substrate**,
I want **the proof-of-erasure to enumerate, remove and prove the Spirit's intent lineage, pending halts and scheduled
invocations (and its installed package), with every remaining store named rather than assumed**,
so that **FR65's "substrate-uninstall is a real guarantee, not a hope" is a verifiable artifact, not three constants.**

**Contract:**
- Epic §20-11 (E20@HEAD:279-284) as amended 2026-10-30; PRD FR65 / NFR-Aud-12; ADR-044 (distillate tombstone + marker).
- Tracker `sprint-status.yaml:309`; E20 confidence rules 4, 6, 8–11; proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers).
- "Do not delete labels before proof" (E20:283): a constant is removed only in the commit whose populated and empty witnesses are green.

## Inventory at HEAD — measured, not assumed

Method: every path read at `a8af3215` (`creation/cites-a8af3215.md` F1–F26). Two callers run one cascade: the
offline one-shot child (`MAOS_ONE_SHOT=uninstall`, LOCK_EX on the store set, `main.rs:7360-7399`) and the door
(`operator_door.rs:1609-1623`: unload first, stop the Worker, then `run_uninstall_cascade`, `main.rs:10097-10126`).

| Category | Where it lives | Enumeration at HEAD | Removal at HEAD | Proof label at HEAD (`main.rs:10812-10861`) | Truth |
|---|---|---|---|---|---|
| intent lineage (I13) | Distillate (kind 11) TL frames at the writer's `(boot_nonce, pid)`, `intent_lineage` in the body (`distillate.rs:121-142,404-416`) | none per Spirit; only per-principal forget closure (`transparency_log.rs:1636-1719`) | only principal-linked distillates | `VerifiedEmpty` (`:10845-10848`) | **false** whenever the Spirit wrote a principal-free distillate |
| pending halts | per-process `HaltRegistry` (`halt/mod.rs:97-112`); durable trace = EpistemicHalt frames (`:523-585`) | none in the cascade; door unload drains them (`scheduler_loop.rs:562-574`) but discards the receipts (`_receipts`) | door: `PlannedUnload` receipts; offline: nothing (fresh registry) | `CoverageGap` "no per-Spirit halt enumeration API" (`:10849-10854`) | door: removed but uncounted; offline: orphans of a dead root stay open |
| scheduled invocations | SCB runtime manifest `[[schedule]]`; watchdog `DashMap`s keyed `(spirit_id, id)` / `(pid, id)` (`schedule_watchdog.rs:90-101`) | none | door: SCB removal stops firing (`scheduler_loop.rs:577`); watchdog state kept | `CoverageGap` "no per-Spirit schedule enumeration API" (`:10855-10860`) | door: stopped, residue kept; installed Spirit: re-admitted at next boot |
| installed package | `installed-index.json` + `installed/<id>/<gen>/` (`storage.rs:136-220`) | — | none (no remove fn in `crates/maos-registry/src`) | absent | CLI says "removes the Spirit from the registry" (`cli.rs:63-66`) — false |
| other per-Spirit stores | FR11 manifest archive (`admission.rs:847`), skill queue entries by author pid (`maos-skill/src/store.rs:22,47`), Lifecycle Journal (`main.rs:10419-10451`) | — | — | absent | proof silent |

Two structural facts gate every category: a Spirit with no principal data returns `NotFound` (exit 4) and writes no
proof whatever else it left (`main.rs:10721-10741`); and the verifier rejects any `Removed{count>0}` without frame
proofs (P18, `proof.rs:383-396`; Python `verify.py:304-312`) — a schedules-only bundle cannot verify today.

## Current-tree corrections — read before implementing

Full OLD → NEW table: `creation/cites-a8af3215.md` (C1–C7, F1–F26).

| Premise (epic / preflight / tracker) | Fact at `a8af3215` and consequence |
|---|---|
| Inventory `main.rs:10570-10584` (E20:283, PF:248); `:9290-9305` (S:309, routing audit, RI) | `:10845-10860` (vector `:10812-10861`) (C1, C2) |
| EC-13 `deferred-work.md:1206` (E20:284, PF:129,249) | `:1331` (C3) |
| FR65 routed to 16-4 (RI:571,596) | 16-4 is `maos purge`; FR65 is the uninstall cascade; owner rows `:75`, `:78` already say 20-11 (C5) |
| "kernel-Δ unresolved pending prototype" (E20:84) | priced: kernel-free shapes for all; optional KH/KS/KF measured (C6) |
| "3c owns audit truth" (E20:284) | confirmed; reuse `run_uninstall_cascade`, `scrub_distillate_body`, `terminate_spirit` — never a second path |
| Forget/hold serialization "preserved" (E20:284) | already broken twice at HEAD: the late sweep runs outside the serializer (`hook_pump.rs:792-795`) and the door's hold release bypasses it (`operator_door.rs:2083-2100`) (F16, F17) — AC5.3 |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) on
`git archive a8af3215` copies; the untouched copy reproduces the pins exactly (26841 / 95 files / set hash
`db39bc4a…`; kernel 21042 tokei; aggregate 191826). Detail: `creation/measurements.md`,
`creation/kloc-kernel-a8af3215-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, optional, PROPOSED):**

| Shape | Kernel physical | Kernel tokei | Diff |
|---|---:|---:|---|
| KS `ScheduleWatchdog::forget_spirit` (D-04 A, **recommended**) | +10 (26851) | +6 (21048) | `ks-schedule-watchdog-forget-spirit.diff` |
| KH `unload_receipted` returns the receipts (D-03 A) | +10 (26851) | +6 (21048) | `kh-unload-returns-halt-receipts.diff` |
| KF late sweep under `forget_serialization` (D-06 B) | +37 (26878) | +33 (21075) | `kf-late-sweep-under-forget-serializer.diff` |
| KH + KS | +20 (26861) | +12 (21054) | `khks-kh-ks-bundle.diff` |
| KH + KS + KF | +57 (26898) | +45 (21087) | `kall-kh-ks-kf-bundle.diff` |
| kernel-free (D-03 B, D-04 B, D-06 A) | 0 | 0 | — |

**kloc forecast** (tokei; HEAD = every pin, zero headroom):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · KS (AC4) | **+6** (physical +10) | prototype `ks` |
| maos-iac · rowid windows (AC5) | **+30** | prototype `p1` |
| maos-iac · per-Spirit distillate enumeration + scrub (AC2) | **+60** | prototype `p2` |
| maos-iac · halt-receipt read-back, per-window hold re-check (AC3, AC5) | +20…+60 [INFERENCE] | P1 helper |
| maos-audit · orphan-halt query; per-category evidence proofs (AC3, AC1.4) | +80…+180 [INFERENCE] | `lib.rs:1679-1767`; `proof.rs:298,354-367` |
| maos-bin · cascade categories, door enumeration, watchdog handle, off-worker forget, serialized release (AC1–AC5, AC7) | +200…+400 [INFERENCE] | `main.rs:10765-10810`; `operator_door.rs:1609-1623` |
| maos-registry · `uninstall_package` (AC4.3) | +20…+40 [INFERENCE] | `storage.rs:179-220` |
| tests, `tools/verify-erasure/verify.py`, workflows, docs | uncharged | — |
| **aggregate** | **+96 measured; +416…+776 forecast** | sum |

**Envelope (D-20-11-08 A, PROPOSED; ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Operator ruling needed |
|---|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +10 / +6 | **26861 / 21054** | yes — FLAG-Winston (KS); add ×2 of KH/KF if ruled |
| maos-iac | 8270 | +110…+150 | **8630** (+360) | yes |
| maos-audit | 7721 | +80…+180 | **8261** (+540) | yes |
| maos-bin | 33498 | +200…+400 | **34698** (+1200) | yes |
| maos-registry | 4248 | +20…+40 | **4368** (+120) | yes (shares the crate with 20-1b; re-measure at landing) |
| aggregate | 191826 | +416…+776 | **194058** (+2232); hardfail = measured + 1 | yes |

**Operator-ruling list:** (1) D-20-11-01…10; (2) the kernel row(s) or none; (3) the envelope above. Each figure
lands exact, with a HISTORY row whose first segment cites this key with `->` (D11 A).

**Superseding grant — 2026-11-04 (ask `halt_grant`).** O4 is granted at +16 physical / +12 tokei kernel and
+4 maos-domain, measured at `73e7afbc`. KS + O4 measured = **26867 / 21060**; the binding D02 A ×2 ceiling is
**26893 / 21078** (KS allowance +20/+12, O4 allowance +32/+24). Domain measured = **10111**, ceiling
**10115** from pin 10107. The creation table above is historical. The grant binds only at final kernel landing
after re-measure/re-base; no pin moves now. The whole-ledger measured STOP is **21250**, superseding 21209,
20-9-only 21230 and the separate joint-21250 exception; **21203 + 12 = 21215** cumulative measured tokei.
The same ledger measures physical **27032 + 16 = 27048**. ADR-069 alarm 22000 / envelope 23000 are unchanged.

## Decisions — RULED asks s2011 (2026-11-02), halt_grant / halt_landing (2026-11-04)

**Rulings 2026-11-02 (ask s2011):** The operator ruled every D-20-11-01…10 row exactly as recommended, including D-20-11-02 B, D-20-11-03 B and D-20-11-04 A. Kernel KS and the ceilings are granted as recorded in frontmatter.

**Rulings 2026-11-04:** `halt_grant` selects D-20-11-11 **A / O4** and shared measured STOP **21250**;
`halt_landing` moves AC3 to final kernel KS + O4 while L1 implements AC1, AC2, AC4–AC7 kernel-free.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-11-01** | What "intent lineage references" are enumerated and removed | **A** the Spirit's own Distillate frames at its incarnations (D-02) via `distillates_written_by` (P2), body-scrubbed with the ADR-044 tombstone (reason `spirit-uninstall`) + a `distillate.redacted` marker naming the Spirit, ids into `erased_distillate_frame_ids` (pre-inclusion proofs); a distillate whose derivation names a held principal stays and is a named gap; IAC-frame `intent_lineage` arrays (NFR-Aud-14 audit chain) retained and named in the reason · **B** A + rewrite the lineage arrays of every IAC frame from/to the Spirit · **C** enumerate only: `CoverageGap` with count and ids | **A.** I13 lineage persists only in kind-11 bodies (`distillate.rs:136`); writer = citer (`:348-377`), so a Spirit's distillates are its own. B rewrites counterpart Spirits' audit chain (`frame.rs:36-50`) with no tombstone convention; C keeps data FR65 says is removed. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-02** | Which incarnations the new categories cover | **A** latest boot only (today, `all_boots=false`, `main.rs:10560`) · **B** every boot, keyed `(boot_nonce, pid)` (`resolve_spirit_name(…, true)`, `lib.rs:1758-1763`), for lineage and orphaned halts; existing principal discovery unchanged · **C** B + re-key the existing pid-only lookups (`principal_ids_for_spirit_pid`, `revoke_all_for_pid`) | **B.** Boot-local pids repeat across boots (`deferred-work.md:1329`); latest-only signs `Removed` over earlier boots' distillates. `(boot_nonce, pid)` never touches another Spirit. C reopens shipped principal discovery that 20-3b's stable-owner path covers (`erasure_uninstall_13_5b.rs:1197`). | RULED 2026-11-02: **B** (ask s2011) |
| **D-20-11-03** | Pending halts: how counted and removed | **A** KH (+10/+6): door counts `unload_receipted` receipts (minus the `term-…` marker); offline: orphaned halts (invocation frame at an incarnation, no terminal receipt, no `halt.resolve` decision) closed through `terminate_spirit` on a transient `HaltRegistry` with `UnplannedCrash` · **B** kernel-free: door pre-reads `drain_for_spirit_dry_run(pid)`, post-verifies `halt_metadata_for_spirit(pid, 0)` empty, counts the `PlannedUnload` receipts by TL read-back after the unload frame; offline as A · **C** enumerate only, no receipts | **B.** Exact without kernel lines; same single receipt writer (PF:212). A is the same evidence through a kernel API. C leaves open halts for an erased Spirit. T0 measurement 2 (second unreceipted drain, `scheduler_loop.rs:574`) can reopen this as STOP + D-row. | RULED 2026-11-02: **B** (ask s2011); amended 2026-11-04: O4 fence required and AC3 moves to final kernel landing (`halt_grant`, `halt_landing`) |
| **D-20-11-04** | Scheduled invocations | **A** KS (+10/+6): door enumerates `[[schedule]]` ids from the SCB snapshot before unload, then `forget_spirit` drops the watchdog's buckets and stamps; offline: ids from the installed manifest, or for a compiled-in Spirit a named gap ("re-admitted by the binary at next boot") · **B** as A without `forget_spirit`; the reason names the per-process state retained until root exit · **C** enumerate only | **A.** The rate bucket keyed by Spirit id (`schedule_watchdog.rs:97`) survives uninstall and a same-id reinstall in the same root inherits it; six tokei make `Removed` literally true. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-05** | The installed registry package | **A** on an `Erased` terminal remove the index entry and generation dirs (`uninstall_package`), category `installed_package` `Removed{1}` / `VerifiedEmpty`; `Held`/`Failed` keep it and say so; `cli.rs:63-66` becomes true · **B** keep it; `installed_package` `CoverageGap` "the next serving boot re-admits it" (`main.rs:4308-4313`); CLI doc corrected · **C** route removal to 20-1b | **A.** An uninstalled Spirit re-admitted at the next boot, schedules included, is not uninstalled; the crate change is small. Coordinate `storage.rs` with 20-1b. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-06** | EC-13 shape | **A** kernel-free: rowid windows (P1); sweeps re-check this principal's hold and the hold set per window under the TL lock; cascade and door forget leave the async worker (`block_in_place`, multi-thread root `main.rs:1856`); door release via `MemoryManagerAdapter::release_legal_hold` · **B** A + KF (+37/+33): the late sweep runs under the kernel serializer · **C** off-worker only, no paging | **A.** `place_legal_hold` writes under the same TL lock, so each window is linearized against placement; B buys the same guarantee for 33 kernel tokei; C keeps the writer stall EC-13 names. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-07** | Landing shape | **A** one key, two landings: L1 kernel-free; L2 ruled kernel rows last, re-based · **B** one landing · **C** split kernel key | **A.** Amended 2026-11-04 by ask `halt_landing`: L1 AC1, AC2, AC4–AC7 with honest pending-kernel reasons; AC3 (halt count / receipt category) moves to the final kernel landing with KS + O4. AC8 records follow each landing; no full-story closure before L2. | RULED **A** 2026-11-02 (`s2011`), amended 2026-11-04 (`halt_landing`) |
| **D-20-11-08** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants, every correction re-asks · **C** ×1.3 (rule 6) | **A.** 20-3b and 20-3c landed at 2–3× creation forecasts; most of this story is unmeasured. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-09** | Proof evidence for categories without frame proofs (schedules, halts, package, tokens) and principal-free Spirits | **A** each claimed status is backed by a `principal.uninstall.category` TL evidence frame written before the post-tree capture (excluded from forget scans by `transparency_log.rs:1589`); the bundle carries post-inclusion proofs for them; schema bump; Rust and Python verifiers check per category; `NotFound` only when every category is empty · **B** narrow P18 to frame-backed categories; others signature-attested and labelled so · **C** keep P18 and `NotFound` as is | **A.** NFR-Aud-12 asks for an externally verifiable receipt; B weakens verification; C fails FR65 for every principal-free Spirit (`main.rs:10721-10741`). Fixes the Python/Rust P18 mismatch (§C R2). | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-10** | Inventory beyond FR65's list | **A** every other per-Spirit store found at T0 (creation list: FR11 manifest archive, skill queue entries by author pid, Lifecycle Journal) is a named category with a counted status and reason — never silent · **B** erase them too · **C** FR65's list + package only | **A.** NFR-Aud-12's owner row (`deferred-work.md:78`) asks for the complete inventory; B reaches into FR11 migration refusals (`admission.rs:847-855`) and 18-7 skill revisions; C signs a silent omission. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-11** | T0 measurement 2 proves an unreceipted late halt; L1 AC3 cannot close under D-03 B. Which kernel repair is requested from FLAG-Winston, under a separate grant and a re-ruled landing dependency? Declining all four keeps the former **B**: the story stays stopped until an independently granted repair lands | **A** (O4) fence inside the single writer: `terminate_spirit` closes the pid in a private `HaltRegistry.closed` lock before its drain; `invoke_halt` holds that lock across its three writes and refuses a closed pid with a typed `InvokeHaltError::SpiritUnloading(pid)` (no durable row); the silent drain at `scheduler_loop.rs:573-574` is deleted. Kernel **+16 / +12** (26857 / 21054), maos-domain +4 · **B** (O1) the same fence, but `unload` closes before `terminate_spirit` and `:574` stays silent. +23 / +15, domain +4 · **C** (O2) route `:574`'s drain through `terminate_spirit`'s per-halt arm (`pub(crate) receipt_drained`), giving a terminal `PlannedUnload` receipt per late halt. +23 / +19, domain 0, no surface change · **D** (O3) B + C. +46 / +34, domain +4 | **A.** It is the cheapest race-free shape and keeps one receipt writer; it covers unload, crash and revocation at one site and removes the only silent drain. Hot swap (20-3c K1p carried set) never reaches the fence. B leaves a silent `:574` drain for a door-rollback re-pend; C is the only option that turns the T0 witness green unmodified, but it leaves a window from `:574` onward (the halt stays pending for a removed pid); D costs the most and crosses 20-9's single 21230. A, B and D each need one in-place `HaltRegistry` hash refresh in `kernel-surface-v0.1-beta.json:618`, and the T0 oracle is re-stated as typed refusal with no durable row. Historical pricing crossed the former STOP 21209. **RULED 2026-11-04 (`halt_grant`): A / O4**, measured +16/+12 kernel and +4 domain; KS + O4 measured 26867/21060, binding D02 A ×2 ceiling 26893/21078 and domain 10115 after re-measure/re-base at final kernel landing. Shared measured STOP **21250** for the whole ledger; cumulative **21203 + 12 = 21215**. Ask `halt_landing` moves AC3 to that final landing; L1 AC1, AC2, AC4–AC7 stays kernel-free. Measured on `git archive` copies by the prebuilt gates. [Pricing](20-11-erasure-inventory-closure-evidence/landing-1/late-halt-repair-pricing.md) · [T0 evidence](20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md). | **RULED 2026-11-04: A / O4 (`halt_grant`); AC3 final kernel landing (`halt_landing`); grant binds only after re-measure/re-base** |

### Review-correction rulings — Main, 2026-11-05

- **F1 / D-02:** kernel-free re-keying is required now: stable `id-<identity-hex>` principal ownership plus an admission-qualified `(boot_nonce, pid, identity-at-row)` lineage join. Cross-boot and same-boot-after-exit pid reuse must preserve the other Spirit. Ambiguous legacy numeric owners fail closed as named gaps, not Erased.
- **F4 / D-09:** `Removed{count:n}` requires exactly `n` matching typed evidence. Frames/packages/schedules use unique real item ids; capability revocation and memory namespace/index use their real returned batch receipts. Both verifiers reject positive counts without matching evidence, duplicate item evidence and mismatched totals.
- **P1/F13 / AC5:** finite initial high-water; bounded tail passes under the hold serializer; a hard cap returns pending/retention, never false completion. Completion is a durable transactional TL marker which rechecks high-water. Its claim covers erasure-eligible pre-marker frame bodies, not post-marker commits, handler-return time or retained audit metadata/numerical facts. Independent same-UID writers have no L1 writer barrier; the limitation is dispositioned under rule 10.
- **P5/P6/F10 / AC4:** retain a complete generation/index preimage through deletion, lifecycle finalization and proof publication. Ordinary errors explicitly recover and expose a truthful partial-proof path; recovery errors are surfaced, never hidden in Drop.
- **F8 boundary:** IAC body mutation and hold authority must share a database transaction. The frozen kernel's subsequent private/index phase still needs a separately measured shared lease and grant at final-kernel preflight. L1 names `kernel_private_index_hold_lease`; no KS/O4 allowance is silently extended.
- **Accounting / fairness:** a verbatim IAC unit-module transfer using the established 14-2b `#[path]` pattern is allowed only as **accounting transfer, not a ceiling relief**, with source/test counts, charged production growth and identical test-name/count evidence. The later FairMutex cutover may mechanically adapt those lock callers, retaining names/assertions and re-comparing nextest lists. Non-poisoning semantics must be disclosed; no new dependency version, sleep or timing waiver.
- **Registry envelope:** minimize first; if still above 4368, the authorized **review-correction re-base: real production growth (streaming-tar recovery + rollback through proof/I10)** is the exact measured registry pin, capped at 4430 and aggregate 195404. Record before/after counts. All other ceilings remain unchanged; excess is STOP.
- **AC5 shared-host disposition:** keep the 25% wall criterion and route/observation deadlines unchanged; do not alter append durability. Bounded-work evidence is local; the wall ratio and real 1M completion are **not demonstrated locally** and are decided by the hosted stable-runner run. AC5/EC-13 remains open under 20-11; the timing/I/O substrate half is owned by 20-7 with same-line rule-10 evidence. The instrumented diagnostic is not a verification pass.
- **P9 malformed-owned ruling B:** a malformed late kind-11 frame with verified `(boot_nonce, pid, Spirit identity)` attribution may be tombstoned as owned lineage (`spirit-uninstall`) and its real id/count proved. Clear its earlier malformed gap only after the final marker certifies completion. Held derivations remain retained/named; ambiguous attribution, including no verifiable incarnation, remains a retained named gap. Byte-level witnesses cover both outcomes.

## Acceptance Criteria

*(**local** = hermetic dev host, offline harness or a door test; **hosted** = GitHub-hosted branch run. Witness
harness for the offline path: `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` (TL and stores seeded in-test, the
real binary run). Every failing-before is observed at `a8af3215` before the change and recorded.)*

### AC1 — no category is labelled by assumption (epic AC1)

**Placement (ask `halt_landing`, 2026-11-04):** AC1 ships in L1 for the categories it implements. The
`pending_halts` category keeps an explicit `CoverageGap` naming AC3's final-kernel dependency; its signed
category evidence authenticates that limitation, never a halt count or a successful erasure claim.

1. `intent_lineage`, `pending_halts`, `scheduled_invocations`, `installed_package` (D-05) and the named stores
   (D-10) take their status from a query of the store that holds them; the constants at `main.rs:10845-10860` are
   deleted only in the commit whose AC2–AC4 populated and empty witnesses are green.
2. `NotFound` (`main.rs:10721-10741`) holds only when every category is empty; a principal-free Spirit with residue
   gets a signed proof (D-09).
3. Each claimed status is externally verifiable (D-09 A): `verify_erasure_proof` and `tools/verify-erasure/verify.py`
   both accept the bundle, and both reject it when one category's evidence is removed.
4. **Failing-before (local):** a Spirit whose only residue is one principal-free distillate → exit 4 `not_found`, no
   proof file, the distillate body intact.
5. **Plant (local):** restore the `NotFound` condition ⇒ AC1.4's vector reds.

### AC2 — intent lineage enumerated, removed, proved (D-20-11-01, -02)

1. The Spirit's distillates at all incarnations (D-02 B) are found by the admission-qualified
   `(boot_nonce, pid, Spirit identity-at-row)` join, scrubbed to the ADR-044 tombstone with a Spirit-named marker,
   and enter the pre-inclusion proofs. Stable `id-<identity-hex>` owners select principal data; ambiguous legacy
   numeric owners fail closed as named gaps. Held derivations stay listed; IAC lineage is retained/named;
   cost facts are untouched.
2. **Failing-before (local):** Spirit with one principal and one principal-free distillate → proof says
   `intent_lineage: VerifiedEmpty` while the body still carries `intent_lineage`. After: `Removed{1}`, tombstone,
   marker, inclusion proof verifies. Variant: the distillate written in an earlier boot is removed too.
3. **Empty witness:** no distillates → `VerifiedEmpty` from the query.
4. **Isolation (security):** another Spirit's lineage and stable-owner data at the same pid, in another boot or after an exit in the same boot, are byte-identical after.
5. **Plant (local):** restore the `VerifiedEmpty` constant ⇒ AC2.2 reds.

### AC3 — pending halts enumerated, removed, proved (D-20-11-03)

**Final kernel landing only** (ask `halt_landing`, 2026-11-04): both door and offline halt enumeration/count,
receipt category and witnesses/plants move with KS + O4. O4's typed late-invocation refusal and the single-writer
fence must be witnessed before AC3 can close. L1 must not infer AC3 completion from an empty registry.

1. Door: the live Spirit's pending halts end in the unload's `PlannedUnload` receipts; the count comes from those
   receipts (not the `term-…` marker); the registry holds none for the pid afterwards → `Removed{n}` / `VerifiedEmpty`.
2. Offline: orphaned halts of the Spirit's incarnations get one terminal receipt each through `terminate_spirit`
   (no second writer, `UnplannedCrash`) → `Removed{n}`; none → `VerifiedEmpty` from the TL query, never from the
   child's fresh registry.
3. **Failing-before (local, both paths):** one seeded orphan (offline) and one live pending halt (door) → category
   `CoverageGap`, and offline no receipt for that halt id.
4. **Empty witnesses** on both paths.
5. **Plant (local):** count from the child's fresh `HaltRegistry` ⇒ the offline orphan witness reads `VerifiedEmpty`
   and reds.

### AC4 — scheduled invocations and the installed package (D-20-11-04, -05)

**L1 / final-kernel split:** L1 queries the SCB/manifest and removes the installed package, but names retained
watchdog buckets/stamps as a `CoverageGap` until root exit. The final KS landing makes AC4.1's watchdog-state
removal literal. L1's plant drops snapshot enumeration; the final kernel plant drops `forget_spirit`.

1. Door: `[[schedule]]` ids read from the SCB snapshot before unload; after uninstall no SCB, no watchdog state
   (D-04 A: `last_fire_ns(pid, id) == 0`) and no `schedule.fire:<id>` frame for the pid over ≥ 5 polls under
   `MAOS_SCHEDULE_FAST=1` → `Removed{n}`.
2. Offline: ids from the installed manifest; a compiled-in Spirit gets the named gap.
3. Installed package (D-05 A): on `Erased`, `default_installed_manifests()` no longer lists it and the next serving
   boot does not admit it; `Held`/`Failed` keep it and the terminal says so; `cli.rs:63-66` states what is true.
4. **Failing-before (local):** door → `CoverageGap` and `last_fire_ns` still the firing stamp after uninstall;
   installed → `installed-index.json` still lists the package after `Erased`.
5. **Plant (local):** drop the `forget_spirit` call ⇒ AC4.1 reds (D-04 B: drop the snapshot enumeration ⇒ count 0 reds).
6. T0 measurement 4 records how an operator uninstalls an installed Spirit refused at boot (`spirit_not_loaded`).

### AC5 — large-log forget scans: bounded lock windows, serialization kept (epic AC2; D-20-11-06)

1. Every forget scan (tombstones, principal / kind-11, malformed kind-11, and AC2/AC3's new queries) examines at most
   one bounded row window per TL lock hold. The initial high-water is fixed; bounded tail passes cover matching
   pre-marker appends. Completion is the single durable TL writer transaction's marker, not handler return;
   a pass-cap exhaustion stays pending/retained, never complete. Post-marker commits are not claimed erased.
   Receipt order stays (timestamp, frame id). This is the 2026-11-05 governing-boundary ruling, not a writer barrier.
2. **Failing-before (local):** a ≥ 100k-frame TL (size from T0 measurement 3), a continuous concurrent
   `insert_frame_event` writer during a door forget: max single-insert latency < 25 % of the forget's wall time;
   at HEAD ≈ 100 %.
3. **Serialization (security):** (a) a hold placed on P between its forget and the late sweep → a late frame naming P
   survives, reported held — at HEAD scrubbed (red); (b) the door's hold release waits for an in-flight forget — at
   HEAD it returns first (red); (c) the 13.5b/18-6 hold suites stay green.
4. Cascade and door forget leave the async worker. No witness: on the multi-thread root a starvation probe is not
   deterministic; reviewed structurally.
5. **Plant (local):** window size `i64::MAX` ⇒ AC5.2 reds.

**Execution clarification (Main's implementation instruction after profiling).** Keep the 25% bound
and raw continuous-writer wall timings; on the loaded shared host, also bind the same quarter-scan
fraction to SQLite VM work of the actual production statements, independent of descheduling/fsync.
The page-size plant must still red. The companion work fixture isolates SQL work in memory; the
real door vectors retain 100k/1M rows with 1 KiB payloads on disk/WAL. An unchanged-deadline HTTP503
`handler_still_running` is only **pending**, never erased; its handler must finish, its completion
receipt be observed, and an identical subsequent request return a completed report with original
tombstone/unrelated bytes intact. No new refusal type or wider route/observation deadline.


### AC6 — a running spawned Spirit (epic AC2's T3 clause)

**Placement:** L1 observes the live categories it implements (AC2/AC4) and labels both pending halt receipts
(AC3) and watchdog cleanup honestly. The final kernel landing verifies the completed AC3/AC4 categories;
L1 must not claim that kernel-dependent proof or borrow a hosted run from an ancestor.

1. **Hosted** (`provision-t3-test-substrate`, `ubuntu-24.04`): a running T3 Worker whose `[[schedule]]` entry is
   firing (`schedule.fire` frames present) is uninstalled over the door; the proof's AC3/AC4 categories come from the
   live state, no `schedule.fire` follows, the container is gone by the existing teardown, the bundle verifies. Run
   id recorded. Elsewhere it skips with a named reason (rule 11(b)).
2. Preservation leg, run once green on the final tree; AC4.5 is its control's plant.
3. T0 measurement 7 decides whether a T3 Worker manifest is admitted with `[[schedule]]` and `on_schedule` enabled
   (the watchdog needs only a Running SCB and the lifecycle gate, `schedule_watchdog.rs:166-176`); if it is not, the
   leg uninstalls the running Worker with whichever AC2/AC3 category its live state holds and the record says why.

### AC7 — the rest of the inventory is named (D-20-11-10)

1. Each other per-Spirit store found by the T0 re-inventory appears as a named category with a counted status and
   reason; a typed list of stores is the single source the cascade iterates.
2. **Failing-before (local):** a Spirit with an archived manifest and an authored skill entry → HEAD's bundle names
   neither.
3. **Plant (local):** drop one entry from the typed list ⇒ a vector comparing the list with the bundle's category
   names reds.

### AC8 — records

1. `tests/coverage-matrix.yaml:896-903` (FR65) and `:959-965` (NFR-Aud-12) name the witnesses; no gate claimed that
   does not exist.
2. `deferred-work.md:1331` `CLOSED <date> by 20-11 @<commit>` with AC5's witness; owner rows `:75`, `:78` updated.
3. ADR-044 addendum: reason `spirit-uninstall`, the category set and the evidence format (D-09).
4. Landing: exact pins, HISTORY first segment citing this key with `->`; FLAG-Winston decision record if a kernel row
   lands (precedent `20-3c-flag-winston-decision.md`); revert `intent-lineage-coverage-report.md` if a run modifies it.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Rulings recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured
  here" 1–7; re-inventory per-Spirit stores (AC7); re-measure every cite this story touches (rule 9). A figure above
  the ruled envelope or a result contradicting a ruled option is STOP + D-row.
- [x] **T1 — Proof format (AC1, D-09).** Evidence frames, per-category verification in `proof.rs` and `verify.py`,
  `NotFound` rule; failing-before + plant.
- [x] **T2 — Intent lineage (AC2).** Admission-qualified stable-identity inventory, transactional scrub/marker and all-boot isolation; witnesses + plant.
- [x] **T3 — Pending halts (AC3, final kernel landing only).** Door count + post-verify, offline orphan query and
  receipts, O4 late-invocation refusal / in-flight / crash witnesses and plants (landing 2).
- [x] **T4 — Schedules and package (AC4).** L1 SCB/manifest snapshot enumeration, retained-watchdog reason,
  `uninstall_package`, CLI doc and snapshot plant; `forget_spirit` and its plant at the final KS landing.
  - [x] L1 snapshot/package implementation and restored pre-unload plant.
  - [x] Final KS watchdog cleanup and its plant (landing 2: `forget_spirit`, live-door and kernel plants).
- [ ] **T5 — EC-13 (AC5), REOPENED by review ruling 2026-11-05.** Hosted run `42533166219` passes the 100k writer and 1M completion/repeat vectors; full AC5/EC-13 remains open for final landing review of all clauses and controls. The shared-host timing/I/O half remains dispositioned under 20-7; earlier failures are retained. [Hosted evidence](20-11-erasure-inventory-closure-evidence/landing-1/hosted-evidence.md).
- [x] **T6 — Inventory (AC7).** Typed store list, named categories, vector + plant.
- [ ] **T7 — T3 leg (AC6).** Hosted branch run under `provision-t3-test-substrate`; run id.
- [ ] **T8 — L1 landing (kernel-free).** Exact pins inside the ruled envelope; `check-kernel-baseline` unchanged;
  `kloc-check`, `check-service-boundary`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`;
  one hosted discipline run on the final tree. No commit or push without the operator.
  - [x] Local source/focused/structural gates, exact pins, final clippy delta, one standalone workspace execution and once-only final prepush recorded.
  - [ ] Hosted discipline run `42533166219` failed on the 20-7 snapshot-owner timing family; signed-T3 Worker execution cannot be distinguished from the witness's successful early return using the log alone. Final landing review remains owed.
- [x] **T9 — Final kernel landing (L2).** Granted KS + O4 last, together with AC3 halt counts/receipt category;
  re-based onto 26854/21051, measured +26/+18, FLAG-Winston record, exact pins/content hashes, single HaltRegistry
  surface-hash refresh under ADR-066, O4 domain variant and AC8 landed 2026-11-11 (uncommitted); KH/KF not granted.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Bin | `crates/maos-bin/src/main.rs:10454-10949` (cascade, categories, `NotFound`), `:7360-7399` (offline arm), `:10097-10126` + `BinPrivateOpsImpl` (door private op, watchdog handle kept before `spawn` `:9717-9728`); `operator_door.rs:68` (`BinPrivateOps::run_uninstall_cascade` gains the door's pre-unload enumeration: halt ids, schedule ids), `:1609-1623` (uninstall), `:2039-2046` (forget off-worker), `:2083-2100` (release via kernel serializer); `hook_pump.rs:792-795` (late sweep) | AC1–AC5, AC7 |
| IAC | `transparency_log.rs` bounded row windows, `write_erasure_body` and hold identity; `transparency_log/erasure_sweep.rs` admission-qualified inventory, aggregate sweep and transactional marker | AC2, AC5; no AC3 writer |
| Audit | `crates/maos-audit/src/lib.rs` (orphan-halt query next to `resolve_spirit_name` `:1679-1767`); `erasure/proof.rs:20-33,167-399` (evidence, schema) | AC1, AC3 |
| Registry | `crates/maos-registry/src/storage.rs:136-220` (`uninstall_package`) | AC4 |
| CLI | `crates/maos-cli/src/cli.rs:63-66` (doc) | AC4 |
| Kernel ◆ (final landing only) | `scheduler/schedule_watchdog.rs` (KS); `halt/mod.rs`, `halt/termination.rs`, `scheduler/scheduler_loop.rs:573-574` deletion (O4); `maos-domain/src/halt.rs` typed refusal; kernel pins/content hashes and single `HaltRegistry` surface-hash refresh | AC3, AC4 |
| Tools / tests (uncharged) | `tools/verify-erasure/verify.py:304-312`; `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` + new door/T3 tests; `crates/maos-iac` unit tests | all |
| Records | `tests/coverage-matrix.yaml:896-903,959-965`; `docs/adr/ADR-044-…md`; `deferred-work.md:75,78,1331` | AC8 |

### Overlap (serialize; re-measure)

- **Kernel pins** — 20-1b (R = KQ1 + P, pending), 20-8, 20-9, 20-13, 20-14: one integration owner per pin update; the
  later landing re-bases.
- **`maos-registry/src/storage.rs`** — 20-1b (yank, quarantine); AC4.3's function is additive.
- **`main.rs` cascade / `operator_door.rs`** — 20-10 (purge, separate file), 20-14 (cost frames must stay readable),
  20-9 (spawned lifecycle may change how `on_schedule` reaches a Worker; AC6 observes the TL row, not delivery).
- **`maos-iac` `transparency_log.rs`** — 20-13 (crypto provider) and 20-14 touch the adapter; windows are local to
  the forget scans.

### Current state that must be preserved

- Terminal contract 0/3/4/5 and its durable `principal.uninstall.*` frame (`main.rs:9971-10024,10499-10505`).
- 13.5b false-success protections (`erasure_uninstall_13_5b.rs`, 26 vectors): held/partial proofs, shared/private
  gaps, region-key gap, write-failure exit 5.
- 18-6 redistillation and the late sweep's purpose (frames committed after the kernel scan are still scrubbed).
- Legal holds: durable, consulted by every forget/uninstall, never auto-erased on release (`memory/mod.rs:448-635`).
- Halt receipts: one writer (`terminate_spirit`); hot-swap carry-not-drain (20-3c K1p, `halt/mod.rs:457-483`).
- ADR-044: append-only marker + body scrub; Merkle pre/post roots recomputed from leaves (`proof.rs:316-339`).

### Guardrails

- Never a second erasure path or halt-receipt writer; reuse the cascade and transactional `write_erasure_body`; `terminate_spirit` remains the single halt writer.
- A new category never uses a bare pid: always `(boot_nonce, pid)` or the stable owner.
- No category status without a query; no `Removed` without evidence the verifier checks (D-09).
- Never scrub cost frames, `principal.uninstall.%`/`principal.forget*` records or another Spirit's rows.
- No kernel line before the final landing; any kernel change outside granted KS + O4 is STOP. KH/KF are not granted.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory; plants byte-restored
  in the real tree (`proof-contract.md:24-31`).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| `NotFound` only when all empty (AC1) | restore the old condition | principal-free-residue vector | local |
| Lineage category (AC2) | `VerifiedEmpty` constant | populated lineage vector | local |
| Halts category (AC3) | count from the fresh registry | offline orphan vector | local |
| Schedules / watchdog (AC4) | drop `forget_spirit` | watchdog-state vector | local |
| EC-13 windows (AC5) | window `i64::MAX` | writer-stall vector | local |
| Inventory (AC7) | drop one store | list-vs-bundle vector | local |
| Verifier evidence (AC1.3) | — (AC1.3's own negative vector) | both verifiers reject | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin -p maos-iac -p maos-audit -p maos-registry
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test erasure_uninstall_13_5b
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib        # only if a kernel row lands
python3 tools/verify-erasure/verify.py <bundle.json> <pubkey.hex>              # AC1.3 (usage `verify.py:181-183`; exit 0 valid / 1 invalid)
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC2.4 (pid reuse), AC3 (receipts),
AC5.3 (holds) and D-09's format, Test Infrastructure (glm-5.3). Focused suites per touched crate (+
`cargo test -p maos-kernel-core --lib` if a kernel row lands); the reviewer reads the hosted AC6 run, never re-runs it.

### Previous-story intelligence

- 20-3c: "one repair owner, no duplicate receipt writers" (PF:212); K1p carry-not-drain; its two-landing shape is
  D-07's precedent; review growth 2–3× is D-08's.
- 20-3b: uninstall discovers the stable owner after re-admission at a new pid (`erasure_uninstall_13_5b.rs:1197`);
  three review-correction raises.
- 18-6: the late sweep and EC-13 originate here (`deferred-work.md:1331`); held co-principals protect their derivation.
- 13.5b: every uninstall false-success path has a terminal code and a vector — extend, never bypass.
- 20-1a: installed inventory is boot-visible and re-admitted (`main.rs:4308-4313`); 20-7: T3 teardown K1w/K2r is
  what AC6 relies on to remove the container.

### Git intelligence

`a8af3215` (20-7 records), `47631284` (20-1a hosted fixes, registry storage), `8143326b` (20-1a review: installed
inventory), `ffd19698` (20-3c landing 1, `main.rs`), `1293f427` (20-3b round 3, private-tier identity). The category
vector at `main.rs:10812-10861` is unchanged since 13.5b/20-3b.

### Library / framework

Rust keeps the already-locked `rusqlite`, `parking_lot` (fair inner handoff),
tokio blocking boundary, `ring` Ed25519/HKDF and `sha2`; `tempfile` provides
owner-only durable proof staging and the complete streaming package preimage.
The FR4 scanner uses dev-only `syn` spans. The standalone Python verifier has
a mandatory hash-pinned `cryptography`/`cffi`/`pycparser` backend, provisioned by
both consuming CI jobs; it never silently skips malformed vectors or crypto.

### References

E20@HEAD:84,279-284; preflight `epic-20-preflight-2026-10-30.md:85,122,129,141,212-213,248-249`; tracker
`sprint-status.yaml:308-309`; `deferred-work.md:75,78,1320,1331`; routing audit `:25,185`; requirements inventory
`:92,158,571,596`; PRD `functional-requirements.md:108`, `non-functional-requirements.md:72-73`; ADR-044; ADR-069
(`:35-48`); 16-4 story `16-4-maos-uninstall-and-keyring.md:423-424`.

## Dev Agent Record

### Story context creation

Created 2026-11-02 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a8af3215`,
without cargo (parent rule).

**Inputs:** E20 §20-11 at HEAD (amended 2026-10-30, PF §3.5) — stale cites corrected here and owed to E20
(`creation/records-owed.md` §A); preflight rows; tracker `:308-309`; `deferred-work.md` rows naming 20-11; PRD FR65 /
NFR-Aud-12; ADR-044; direct reads of every cited range (`creation/cites-a8af3215.md`); kloc and kernel by the
prebuilt gate binary on `/tmp/proto2011` `git archive` copies (`creation/kloc-kernel-a8af3215-and-prototypes.json`);
prototypes KH, KS, KF, P1, P2 and bundles saved as diffs, `rustfmt`'d, never applied to the tree.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md` (§A epic, §B planning records, §C routed observations).

**Revert:** no crate source modified (prototypes only under `/tmp/proto2011`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS
CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197
done stories, 166 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26841);
`check-decision-register` rc 0 (23 rows, 6 open); `check-exit-commands` rc 0 (2 blocks, 10 tokens);
`check-kernel-baseline` rc 0 (26841 / 95 files, set hash `db39bc4a…`). In-tree `kloc-check` rc 1 (aggregate 191834,
`maos-wasm-host 2420 > 2412`) from a sibling's uncommitted `crates/maos-wasm-host/src/runner.rs` edit, not this
story's; the `git archive a8af3215` copy measured 191826, `over_budget=[]`.

### Landing 1 preflight — STOP (2026-11-04, Dev2011)

Baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d`; initial `git status --short` empty. Read the entire story,
including all ruled D-rows and creation measurements/records. No runtime, kernel, pins, tracker, coverage matrix,
ADR or closure claims changed: T0 measurement 2 hit the explicitly ruled STOP before T1–T8.

**Observed witness:** `t0_unload_must_receipt_halt_invoked_after_termination_drain`, saved
[source:48](20-11-erasure-inventory-closure-evidence/landing-1/t0-unload-race-witness.rs.txt#L48), terminal receipt
assertion at line 187. The existing TL redaction-policy seam pauses the first termination receipt after
`terminate_spirit`'s drain; a concurrent thread asserts the real SCB is `Unloaded`, calls real `invoke_halt`, and
checks pending metadata before resuming the real unload. Unload succeeds; the durable invocation remains, metadata
is empty and the SCB is gone, but the only receipt names `term-race-20-11-1-1793782986345734392`, not `late-20-11`.
No production plant or kernel modification was used. Full output and exact commands are in
[landing-1 evidence](20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md).

**Measurements:** baseline gates passed unchanged: physical kernel 26841 (95 files), kernel tokei 21042;
maos-bin 34504, maos-iac 8270, maos-audit 7721, maos-registry 4248, aggregate 193140. No grant consumed.
The assignment's re-based maos-bin L1 ceiling is 35704 (34504 + original 1200 allowance); other story frontmatter
ceilings remain unchanged. No re-pin is warranted for this evidence-only STOP.

**Record checks:** `check-service-boundary`, `check-dev-record-completeness`, `check-epic-close-coherence`,
`check-decision-register` and `check-exit-commands` each ran once with the prescribed prebuilt-gate environment,
all exit 0. These are structural record checks, not acceptance proof. The only cargo test run exited 101 at the
intended receipt oracle; no retry. Detailed command/exit ledger and raw output are in landing-1 evidence.

**Cleanup:** /proc scan found no cargo/rustc process; granular stale incremental-session pruning retained newest
and last-two-hour sessions and freed **10136645632 allocated bytes**. No deps/build/fingerprint removal.
The test's scoped temporary directory dropped on unwind; no named `/tmp` scratch was created.

**Preflight disposition, subsequently superseded:** the initial STOP left all runtime ACs open. On 2026-11-04
the operator granted D-20-11-11 A / O4 (`halt_grant`) and moved AC3 to the final kernel landing (`halt_landing`).
FR65/NFR-Aud-12 and EC-13 remain open until witnessed; no implementation or hosted proof is claimed by this
recording pass. The existing purge-proof route stays with 20-10. The temporary integration test was removed after
its exact formatted source and failure output were preserved as `.rs.txt` evidence, outside Rust/KLOC scanning.

### Ruling application — 2026-11-04, Dev2011 (docs only)

Recorded `halt_grant`: O4 granted, measured kernel +16/+12 and domain +4; KS + O4 measured 26867/21060,
binding D02 A ×2 ceiling 26893/21078, domain ceiling 10115. The single receipt writer closes the pid through
`HaltRegistry.closed`; `invoke_halt` holds the lock across all writes and refuses typed before any write; delete
the silent drain, refresh only the HaltRegistry surface hash at final landing under ADR-066. No source or pin
was changed. Shared measured STOP 21250 applies to the whole ledger; cumulative 21215 tokei.

Recorded `halt_landing`: L1 AC1, AC2, AC4–AC7 kernel-free, explicit pending halt/watchdog limitations;
final kernel KS + O4 carries AC3. Story/tracker stay `ready-for-dev`. Shared-STOP supersession notes preserve
historical pricing/rulings in pending stories, FLAG-Winston records, epic, preflight and tracker.
No cargo, tests or gates ran for this docs-only recording pass; Dev2010 owns the cargo slot.

### Landing 1 continuation and priority park — 2026-11-04, Dev2011b

Model: `openai-codex/gpt-6.1-sol`. Initial baseline `76efc19d`; inherited proof/IAC/registry/Python
hunks were adopted only after preserving their diff and observing original-HEAD witnesses.
The initial source restore used exact `git show HEAD:<path>` bytes, not an assumed prior result.
Evidence and the re-read inventory are in
[L1 current-tree cites](20-11-erasure-inventory-closure-evidence/landing-1/l1-head-cites.md).

**Priority park (operator/Main request).** All 14 owned tracked paths and 28 new files were saved to
`/tmp/2011-wip/{tracked.patch,new-files.tar.gz,SHA256SUMS,MANIFEST,runtime-state.json}`.
Tar members and every working-file SHA256 were checked before reverting only that owned work.
`git status --short && git rev-parse HEAD` exited 0 with empty status at `76efc19d`.
No cargo ran while parked. The park's Dev Agent Record lived only in the requested `/tmp` MANIFEST.
Main then granted the slot and supplied new exact pins at **`cb0abc06`**: bin **34552**,
xtask **44466**, aggregate **193202** / hardfail **193203**. The saved patch passed
`git apply --check`, reapplied without conflict, and all new files were extracted. The priority
network gate/model-currency hotfix was not reverted or overwritten.

**Boundary preserved.** No kernel, domain variant, kernel baseline/content hash or kernel surface hash
is changed. AC3, O4 and KS remain the final landing. L1 signs an explicit pending-kernel halt gap,
not a count or successful receipt-erasure claim; live schedules name retained watchdog state.
The original T0 race witness and its red log remain unchanged.

**T0 schedule finding.** Both production admission bundles currently default SCB schedules. A real
serving root accepted the schedule-bearing manifest but never fired. This is already owned by
`20-9` AC3 / D-20-9-08 A, not repaired by L1. Per AC6.3, the real live cascade uses a canonical
principal-free distillate seeded at the running Spirit's actual boot/pid. The separate AC4 boundary
oracle supplies a genuinely scheduled SCB to the real door and captures the ids before unload.
The hosted Worker test adds existing owned-CID teardown; signed T3 substrate is absent locally.

**Package failure witness.** Before the retention repair, forcing proof-directory failure returned
Failed after deleting the installed generation (`l1-before-failed-package-proof.log`, exit 101).
Removal now stages generation ownership and active inventory under a serialized, rollback-on-drop
guard; proof failure restores the package, and only proof persistence permits final deletion.
Cross-store crash atomicity remains the existing ADR-044 limitation, not a new recovery claim.

**Plant quality correction.** The first combined control run produced four intended reds but the
AC5 `i64::MAX` control was green: whole-door wall time includes several scans and diluted the
largest lock hold. That output is retained, never labelled a red or a flaky retry. The same vector
now also instruments one real constituent query, preserving the door's unchanged 25% threshold.
The same page mutation then reded at both calibrated sizes; all planted source files were
restored with explicit copy-back, `cmp`, and `touch`. No plant remains applied.

**Superseded interim handoff, not landing approval.** A harness wrap-up notice interrupted the step;
Main subsequently clarified that it was not an operator wrap-up request and instructed continuation.
The following interim observations remain historical evidence, not the final landing disposition.
The failed-proof retention repair is complete: the genuine before witness exits 101 and the final
inventory test passes. The live recovery witness now uses a plain serving root (the actual installed-boot
surface), with a package id distinct from compiled-in `hello-spirit`; it observes admission refusal,
live `spirit_not_loaded`, then successful offline removal and signed proof after root exit.
The negative verifier vector recursively sorts the unsigned envelope before re-signing and requires
both verifiers to reject specifically for the missing category evidence, not a stale signature.

**Observed final focused result:** `l1-current-step-focused.log`, cargo exit **101**.
Inventory **9/9**, local live lineage/refused-boot recovery **1/1**, late-held sweep **1/1** passed.
The T3 test returned its explicit missing-substrate skip; it is not hosted evidence.
Shell-host **11/13** passed; the unchanged REPL clarification timing family failed and the AC5
latency witness failed at 100k / remained `HandlerStillRunning` at 1M. The observed ratios are
**0.651344 door / 1.050325 constituent**, not compliant with 25%. No retry, timeout increase,
threshold weakening or claim of AC5 completion. Same-line rule-10 records are in `deferred-work.md`.

**Exact post-format pins:** bin **34845** (+293), IAC **8373** (+103), audit **7775** (+54),
registry **4320** (+72), xtask **44466** (+0), aggregate **193724** (+522), hardfail **193725**.
All remain inside the stricter L1 ceilings (**35704 / 8630 / 8261 / 4368 / 44466 / 195404**).
The kernel is unchanged at **26841 physical / 21042 tokei**, 95 files, identical set/content hash;
domain remains **10107**. `xtask/kloc.toml` rows move together and the xtask first HISTORY segment
cites this live story with `44466 -> 44466`, preserving the priority hotfix.

**Observed structural checks:** prescribed prebuilt `check-kernel-baseline`, `kloc-check`,
`check-service-boundary`, `check-dev-record-completeness`, `check-epic-close-coherence`,
`check-decision-register`, `check-exit-commands` all exit **0**; raw `l1-current-step-*.log`.
Earlier affected binary build and workspace clippy each exit **0**, but preceded the final
package guard and are not final-tree certification. Final fixture-builder/build/metadata-reuse
workspace nextest, formal final clippy delta, and the once-only prepush have **not run**.
Hosted T3, delegated cgroup/network and hosted discipline legs remain **unverified**.
This is **not** a merge-ready L1; AC5/EC-13 and landing verification remain open.

**Last cargo control and cleanup:** `cargo test -p maos-bin --no-default-features --test verb_table_15_3`
exits **0, 7/7** (`l1-final-no-default-control.log`); no subsequent cargo run.
Before any integration suite, rebuild default-feature binaries after the registered fixture builder.
A `/proc` scan found no cargo/rustc before cleanup. Own scratch/park files and **622** stale incremental
sessions were removed while retaining newest-per-crate and all sessions touched within 2h;
deps/build/fingerprint were untouched. **3223033786 logical bytes removed**, observed filesystem
free-space increase **2865713152 bytes** (`l1-cleanup.json`); custody records preserved in evidence.

**Continuation after Main's clarification.** Cargo ownership resumed at unchanged `cb0abc06`.
Profiling before further source repair found a genuine boundedness defect: SQLite chooses
`idx_tlog_kind` for `kind=11`, re-examining the whole kind population inside each nominal rowid
window. At 100k rows the first page takes **724600 VM steps**; at 1M, **7024500**.
Forcing rowid access on the same 1M data takes **45000–45100 steps** per 4096-row page.
Raw plans, VM counts and wall/CPU times: `l1-ac5-resume-profile-before.json`,
`l1-ac5-resume-profile-1m.json`. Wall scheduling/fsync is separately retained, not mistaken
for work under the scan lock. The binding work oracle keeps the quarter-scan bound and
independently limits actual production-statement VM steps, so the `i64::MAX` plant must still red.
Main accepts the existing typed HTTP503 only as **pending**, provided its handler completes
and an identical subsequent request succeeds without state corruption; no route deadline changes.

The superseded interim witness table remains in `l1-wrapup.md`; the final AC5 repair evidence
and current cites are in `l1-ac5-repair-evidence.md` and `l1-final-cites.md`.

### Landing 1 profiled repair and final focused verification — 2026-11-04, Dev2011b

**Actual bounded work, not a nominal SQL window.** The kind/timestamp plan revisited the
whole kind population inside each rowid range. Kind-only indexing now pins the rowid-ordered
access path; general predicates pin primary rowid access. Recovery's previously missed full-TL
anti-join is also paged, with indexed request/outcome lookups and off-worker invocation.
Receipt vectors retain timestamp/frame-id ordering. A guarded JSON expression index preserves
opaque outcome writes, JSON5 extraction semantics and explicit corrupt-recovery errors.
Genuine before controls caught the full recovery scan, incorrect ordering and unguarded index;
none was hidden behind a fallback. Raw outputs and exact plants remain in landing-1 evidence.

**AC5 final restored source.** Test-only instrumentation meters the actual production statements,
not a substitute implementation: four new boundary/work vectors pass, followed by the complete
IAC unit suite **103/103**, exit0. The final `i64::MAX` plant reds both100k/1M at the quarter-scan
assertion, exit101, then explicit copy-back + cmp + touch exits0. Fixed pages stay below0.25
and below the independent4096×128 VM-step cap across all five predicates. No kernel,
route deadline, timeout, observation budget or latency threshold was increased.

**Real disk/WAL door, full writer interval.** Final100k wall1.070610323s / max insert232.483050ms
gives0.217150017; final1M wall8.480294334s / max insert2.000848329s gives0.235940906.
Both satisfy the literal25% bound. The writer remains active until actual handler completion.
An earlier separately prescribed repaired run exercised real1M `handler_still_running`:
the response was **pending**, the original handler durably completed, and an identical request
then returned the completed report without rescan corruption. The final stricter run completed
inside the unchanged10s route deadline and also passed the repeat. No pending response is
called an erased report or proof.

**Final focused/preservation observations.** `l1-final-focused-and-preservation.log`, exit0:
inventory9, live local1 plus explicit hosted-T3 skip1, late-hold1,13.5b26,18-6 floors1,
shell14. Thus53 reported successful cases,52 exercised locally and one unavailable-substrate
runtime skip. `l1-final-kernel-audit-registry-preservation.log`, exit0: kernel cascade13,
audit smoke3, registry58 plus2 ignored. Earlier timing failures remain same-line rule-10
records under20-7; no retry-to-green or budget widening is claimed.

**AC5-stage exact formatted pins (before FR4 closure below).** bin **34842** (+290), IAC **8550** (+280), audit **7775** (+54),
registry **4322** (+74), xtask **44466** (+0), aggregate **193900** (+698), hardfail **193901**.
All remain inside **35704 / 8630 / 8261 / 4368 / 44466 / 195404** respectively.
Kernel remains **26841 physical /21042 tokei**,95 files, identical set/content hash;
domain remains10107. Exact rows and HISTORY move together; kernel grant consumption is zero.
`l1-final-cites.md` re-measures the resumed immutable HEAD and final working-tree entry points.

**AC5-stage preparation and clippy (before FR4 closure below).** Registered fixture builder exits0 (16 artifacts), then genuine
default-feature `cargo build --locked -p maos-bin -p maos-cli` exits0. Workspace/all-target clippy
exits0; the formal unchanged-baseline ratchet exits0 at **1329 occurrences /870 distinct**
versus **1383 /897**, a delta **-54 /-27**. Hosted-toolchain results remain unverified.
Source/structural gates exit0, including unchanged kernel baseline, exact KLOC, boundaries,
serde handling, env contract, composition roots, constructors, test cost, epic coherence,
decisions and exit commands. Deferred-owner parser ambiguity was fixed only by explicit
short20-7 aliases alongside the same existing owner, with no ownership/status changes;
the resulting record gate exits0 at199 done stories,180 owner assertions and zero violations.

### Full-workspace observation and final writer-inventory closure — 2026-11-05, Dev2011b

**One actual standalone workspace execution, no retry.** Locked full-workspace producers
exported58 workspace members /663 binary targets; the consumer reused both metadata files
at default concurrency, with no retry override. Nextest rejects `--workspace`/`--locked`
on a metadata consumer: the retained CLI-only control exits2 before any test. Dropping only
those incompatible build flags does not narrow the producer's full-workspace scope.
The actual run exits **100**: **5640 run,5639 passed,1 failed,138 skipped**,46.919s,
run id `16ed5fb9-66a1-4774-a94d-3e94c2321a0f`. Same-run JUnit is preserved.
All L1 witnesses pass, including actual100k/1M writer ratios **0.198724650 /0.207434044**;
the hosted T3 case remains its explicit unavailable-substrate runtime skip.

**The sole failure was actionable, not deferred as a flake.** FR4's source inventory
found the new `scrub_uninstalled_distillate` writer without a disposition. Its actual
kind1, pid0, absent-token `distillate.redacted` event carries exactly stable Spirit id
and redacted frame id. The production `WRITER_SHAPES` row and independently measured
fixture now classify that maintenance marker as NonCall, following existing redaction
conventions. The first repaired focused run then exposed the scanner treating the
external test-only VM module as production. An explicit leading file-level `cfg(test)`
and scanner support for that actual compilation guard fix the boundary; a genuine
red-before control and production-only `cfg(not(test))` inclusion control prevent a
basename exemption or blanket production suppression. Complete classifier target:
**20/20, exit0**. No second standalone workspace execution was used to erase its red.

**Final current exact pins after formatting/closure:** bin **34842** (+290), IAC **8551**
(+281), audit **7785** (+64), registry **4322** (+74), xtask **44466** (+0),
aggregate **193911** (+709), strict hardfail **193912**. All original stricter ceilings
are unchanged. Kernel26841 physical/21042 tokei and domain10107 remain unchanged.
`l1-fr4-complete-counts.json` records the pre-repin measurement; exact synchronized
rows/HISTORY yield `l1-fr4-final-kloc-check.json`, exit0. No kernel grant is consumed.

**Post-closure control and genuine binaries.** The explicitly requested final
`cargo test -p maos-bin --no-default-features --test verb_table_15_3` exits0, **7/7**.
After that feature-mode control, IAC's complete lib target passes **103/103**, and
a genuine `cargo build --locked -p maos-bin -p maos-cli` restores default binaries:
the sequential group exits0. No stale no-default executable is left for final prepush.

Metadata compilation's3000s cell timeout and a separate Eval30s interruption are
retained with no invented child exit code; each was followed by an empty cargo/rustc
process scan before cached compilation continued. The successful metadata cell exits0.
These were build/CLI repairs, not flaky-test retries. Exact provenance and scope:
`l1-metadata-execution-record.json`; raw final workspace stdout/stderr and JUnit remain.



**Final-source clippy refresh after both writer-boundary fixes.** Workspace/all-target
clippy exits0 (`l1-fr4-final-clippy.jsonl`); the formal unchanged-baseline ratchet exits0
at1329 occurrences/870 distinct versus1383/897, delta-54/-27. The final exact KLOC gate
also exits0. Hosted toolchain/discipline and signed-T3 execution remain unverified.

### L1 local landing disposition — 2026-11-05, Dev2011b

The prescribed final prepush ran **once**, with undelegated cgroup mode and no tracked
edits in flight: **exit0**,1186.646s. Raw92 records: **83 pass /8 skip /1 advisory**.
Final-tree workspace nextest, kernel-core lib, workspace doctests, fixture builder,
source/structural gates and exact kernel/KLOC checks pass. Correct same-run default-profile
JUnit is `l1-final-prepush-default-junit.xml`, UUID `ad63b740-ae30-44f2-8c68-1aee1df88f31`:
**5640 cases,0 failures/errors**,46.635s. Earlier standalone CI-profile red/JUnit remain
separate; its deterministic writer-inventory failure is fixed, not retried away.

The aggregate test-budget subcheck is honestly **advisory exit1**, not green: hook-pump32.528s,
one-door33.478s, shell-host48.677s and participant-recall42.116s each exceed unchanged30s
per-binary sums. All functional cases pass; shell includes this landing's required1M vector,
so it is not labelled an unchanged target. Same-line rule-10 records under owner20-7 retain
all four findings. No retry, threshold/budget increase or new exception. EC-13's kernel-free
bounded-work/hold repair is closed separately; FR65/NFR-Aud-12, hosted AC6/discipline and
final-kernel AC3/KS/O4 remain partial/open as applicable. Story/tracker status is not flipped.

Final full AC name:line / before / plant table, exact envelopes, commands/exits, advisory
dispositions and limitations: `20-11-erasure-inventory-closure-evidence/landing-1/l1-final-report.md`
and `l1-final-witnesses.md`. No commit, push, kernel/source hash refresh or hosted proof claim.

**Final record checks and cleanup.** After advisory/EC-13 dispositions, the prescribed
`check-dev-record-completeness`, `check-decision-register` and `check-exit-commands`
each exit0 (`l1-final-records-*.json`). No prepush rerun. Guarded cleanup then exits0:
empty cargo/rustc `/proc` scan, owned scratch/runner deleted,976 stale incremental sessions
removed while newest-per-crate and all last2h touches are retained; deps/build/fingerprint
and other worktrees untouched. Final pass removes11,480,789,636 logical bytes and observes
8,119,390,208 bytes more free space. Including the earlier explicit pass:1,598 sessions,
14,703,823,422 logical bytes; summed observed free-space increases10,985,103,360 bytes.
Accounting/paths are in `l1-final-cleanup.json` and the final report; no cargo/check after cleanup.

### L1 review corrections — 2026-11-05, Dev2011b

**Authority and scope.** Model `openai-codex/gpt-6.1-sol`; baseline
`93d39cc7ca185f513dee2d97bed2058f11554d9a`, original L1 `cbed4dad`.
This record supersedes that candidate's local closure claims, not its preserved
T0/history. Main's F1/F4, finite-scan/marker, P9-B, FairMutex, accounting and
registry re-base rulings are recorded above. No kernel/domain source, grant,
tracker status, commit, push, other worktree or `.devcontainer` was changed.
Correction evidence lives in
[`landing-1/review-corrections/`](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/).

**Implemented contract, not a second erasure path.** Stable `id-<identity-hex>`
principal ownership and admission-qualified identity-at-row isolate cross-boot
and same-boot-after-exit pid reuse; ambiguous legacy numeric owners fail closed.
Real component batches and unique frame/package/schedule ids back exact typed
counts. Both verifiers enforce the same complete v2 inventory, strict DTO/hex
language, total Merkle paths, disjoint frame kinds and signer-keyed subject
commitments. Publication uses owner-only unpredictable temporary files,
no-replace publication and file/directory fsync; a published-but-unsynced error
preserves its real path. All builder/smoke/reader callers were migrated.
The FR4 scanner parses Rust and cannot hide a production writer behind a
brace-less test item; the private erasure-row primitive is narrowly classified.

**Concurrency and failure closure.** Initial scans fix a high-water and use
4096 actual rows per lock window, including sparse histories. Linear graph
closure avoids 65 rereads for a 64-parent import; receipts retain timestamp/id
order. Bounded tails and the final single writer transaction certify only
matching erasure-eligible pre-marker bodies, not handler-return time or
post-marker writes. ABA-safe hold nonces and attached-authority transactions
cover TL mutation/request/marker ordering. The frozen private/index hold lease
remains a named gap requiring separate final-kernel measurement/grant.
Whole-cascade inventory/filesystem/proof/finalization work leaves the async
worker. Hook-held and malformed observations reach the package/terminal decision.
Verified-owned malformed bytes may really tombstone with their real proof id;
ambiguous bytes stay retained, and an obsolete malformed observation is cleared
only after the final marker certifies it.

**Package and receipt retention.** A complete streaming-tar preimage and real
boot inventory stay armed through partial deletion, lifecycle completion and
proof publication. Real EACCES controls prove full-byte restoration and surfaced
index-recovery errors; Drop logging is not the recovery API. Provisional signed
gaps precede irreversible scrubs; later failures retain a truthful partial proof
path rather than deleting the sole receipt. A repeat recovers subject scope
from durable audit history after index deletion.

**Fair handoff safety.** The already-locked `parking_lot` 0.12.5 FairMutex is
non-poisoning. No explicit poison-recovery branch was relied upon, but real
SQLite-trigger I2 panics exposed implicit manual-transaction/unbacked-last-id
dependencies. RAII rollback and commit-ordered id publication repair ordinary
append, schema lifecycle and kernel-event helper paths while preserving I2
panics and existing append durability. Three independent unwind regressions
pass; `cargo deny --locked --format json check` exits0.

**Accounting transfer, not a ceiling relief.** The gate explicitly excludes
test directories (`xtask/src/kloc_check.rs:294-318`). The original library's107
nextest names were identical around the verbatim unit-body transfer;111 names
were identical around the approved four mechanical lock-caller adaptations;
three new unwind tests then made114. Returning the168-line VM module verbatim
to `src` kept all114 names/count identical and all114 executed. Only the unit
body remains in `tests/support` (506 ->504 lines from the approved adaptations).
Baseline src/tests8551/2315 ->8622/3577; no-transfer counted src9126, delta+575
before accounting, versus published net+71. That counterfactual includes counted
test/helper code, not a claim every added line runs in production. Ceiling8630
is unchanged. See `accounting-transfer-final.json` and identity ledgers.

**Exact formatted measurement.** Bin34823 ->34914; IAC8551 ->8622;
audit7785 ->8174; registry4322 ->4412; xtask44673 ->44673.
Aggregate194099 ->194740, hardfail194741. Registry's **review-correction re-base:
real production growth (streaming-tar recovery + rollback through proof/I10)**
is +90, former4368 exceeded by44, exact grant4412 within Main's cap4430.
Other crate pins are unchanged; kernel26841 physical/21042 tokei and domain10107
are unchanged. `cargo fmt --all`, exact KLOC, kernel-baseline and service-boundary
checks exit0. Workspace/all-target clippy exits0; advisory ratchet1329/870
versus1383/897, delta-54/-27; warning debt remains, baseline unchanged.

**Preservation closure, not lowered oracles.** The first metadata consumer
refused an inexact `fr4` binary filter (exit94); its corrected all-targets
metadata then exposed a custom benchmark's non-libtest listing (exit104).
The normal workspace test-target producer is the correction, not a test filter:
58 members /669 default test targets; all-target compilation/clippy is separate.
The first actual focus ran403 cases,387 passed/16 failed, all failures in13.5b.
Its fixtures now bind admitted stable storage identity, while explicit legacy
and readmission vectors retain numeric setup. Both retained signed receipts are
authenticated and the returned terminal path is selected; no first-file shortcut
or weakened “at least one” assertion. After that input/receipt cutover,8 genuine
preservation violations remained. Independent orphan=true and regeneration
controls exposed the masked causes (`preservation-critical-before`).
Typed failure classes restore Held/3 with the independently signed regional-key
gap, Failed/5 for unrelated unindexed values, and the prior D-16-2-F populated-log
unknown-name failure. One metadata walk attributes held namespaces independently
of payload readability. A final marker alone supersedes earlier completion
errors; actual regeneration Pending reports remain visible. Signed private/key
reasons and the old erasure/hold/data-byte oracles are preserved. The redundant
explicit rollback attempt is removed. First preservation checkpoint: **333/333 pass**,
including all26 original13.5b cases and all114 IAC cases (exit0).


**Controls and limits.** `before-state.md` preserves actual reds and explicitly
excludes fixture/setup false starts. `current-plant-results.md` records restored
AC1/AC2/AC4/AC5-work/AC7 and recovery controls. The first combined AC7 red was
masked by the temporary AC1 guard and is NOT counted; the isolated omission
really reds its populated count assertion. Every plant restores with
copy-back + cmp + touch, exit0. AC5/EC-13 remains OPEN: bounded SQL work is green,
but shared-host wall0.2501131127 and the 1M completion timeout do not demonstrate
the unchanged wall/completion criterion. The diagnostic0.11442585 is explicitly
not verification; normal appends held the lock147ms–1.8s on fsync.
No threshold, deadline, retry count or append durability was widened.
Hosted callgrind, signed T3, delegated cgroups and network namespaces remain
unverified. AC3/KS/O4 and the private/index lease are final-kernel work.

**Named limitation — final-tail redistillation (Main ruling, 2026-11-05).**
Real tail requests are durably Pending/Refused(`late-frame`), with count and
original ids in the uninstall receipt; the signed frame marker attests erased
bodies, never completed regeneration. The uninstall tail does not dispatch to
another live Spirit. Recorded Pending outcomes are excluded from ordinary
outstanding-request recovery: no automatic/CLI redrive or cross-boot rebinding.
Outside uninstall an authorized embedding may obtain a fresh real
`ForgetOutcome::Erased` for a known subject, append the unchanged durable request
to its `redistill` list, and call `HookPump::finish_forget` with the live pump.
No invented receipt/counts/ids or promotion of a Suspended hold; an unknown
subject or unavailable/ambiguous writer incarnation stays pending.
ADR-044 and operator docs state this exact API-only replay. The same-line
rule-10 deferred row is ownerless/open, dispositioned by Main and trigger-owned
by the first story adding an operator redistill-retry surface. No new retry
API/policy/CLI is added in this landing.

**Final structural records.** Exact-source kernel-baseline, KLOC,
service-boundary, epic-close coherence, exit-command and decision-register gates
exit0. Dev-record completeness initially exits1 on two record-format defects:
the kernel source range `521-570` was read as a stale owner key, and the new
ownerless row said “disposition” rather than the recognized recorded
“explicitly dispositioned by Main” form. The full source citation and same-line
disposition were corrected without changing ownership/rulings; the repaired gate
exits0. Both raw results remain recorded, not classified as flakes.

**Final producer cells.** Registered fixture builder and genuine final native
bin/CLI build exit0; fresh Cargo metadata exits0. The first default-workspace
binary-metadata compilation cell reaches its2900s deadline (observed tool
wall2902.09s) without a completed producer status. It executes no test cases and
is not counted as a workspace test run or a passing producer. The bounded
build-only continuation keeps the identical target set/deadline; no test retry,
runtime-concurrency change or budget widening. Exact diagnostics/process guard
are recorded in `metadata-producer-build-cells.md`.

**Actual standalone workspace and final preservation repair.** The default
workspace producer completes exit0 with eight compiler jobs (not a nextest
thread override),58 members/669 targets. The one unfiltered metadata-reuse
consumer runs at default test concurrency with retries0: **5688 run,5686 pass,
2 fail,138 skipped**, exit100; test wall152.350s, command wall201.394s.
The real code regression is the unchanged kernel oracle
`an_unrelated_forget_does_not_treat_partial_redaction_as_prior_erasure:429`:
the IAC prior-source SQL incorrectly treated partial cost/governance redactions
as full-body erasure. Remove that broad OR, preserving numerical facts as citable;
no kernel source/test or assertion changes. After repair **346/346 pass**,
including all13 original cascade cases. Reintroducing the exact OR independently
reds the same assertion at447, exit100; copy-back + cmp + touch restore exit0.
The second failure is the unchanged real1M in-flight completion observation
deadline, not waived or retried. AC5/EC-13 stays OPEN and the same-line20-7
timing/I/O disposition records this occurrence. The100k vector genuinely passes
at0.1164494388, with recursive peak152236032 allocated bytes and explicit
directory-close/absence assertions reached;1M seed peak1487568896 bytes is
observed, but its post-completion peak/close assertion is not reached.
The watched refused-successor cold-swap sibling passes (1.316s), not deferred.
Final source is IAC8622 and aggregate194740/hardfail194741; all other measured
counts remain unchanged, all six pin histories move together after fmt.

**Final once-only prepush and docs repair.** Post-plant cascade13/13, genuine
bin/CLI build, final all-target workspace clippy and advisory ratchet exit0.
Clippy remains1329/870 versus1383/897 (-54/-27), baseline unchanged.
The registered fixture builder is refreshed after the final IAC repair (67.200s,
exit0), followed by genuine bin/CLI build (0.499s,exit0).
One prescribed prepush, with undelegated mode, declared Python backend,
compiler jobs8 only and retries0, exits1 in1195.222s:84 pass records,2 red,
1 aggregate-timing advisory,5 local marker-cardinality skips. Tracked records
stay frozen while it runs; no prepush rerun. Same-run JUnit is copied immediately:
5688 executed,5687 pass,1 fail. The sole test failure is the unchanged1M
completion observation (42.312s; route pending10.000086555s). The repaired
partial-cost oracle passes0.372s; the watched refused-successor sibling passes
1.029s. Six unchanged30s aggregate advisories have same-line20-7 dispositions
and the actual JUnit/JSON evidence; no budget/exception increase.
The other red is real EN/KO glossary drift (Spirit7 versus4), now repaired by
translating the actual affected Korean operator sections. Focused glossary,
bidi, anchor, full docs build and routes each exit0 from docs-site. A mistaken
first focused invocation from repo cwd fails MODULE_NOT_FOUND and is excluded
as setup error, not a feature witness. The original prepush red remains recorded.
Kernel-lib, workspace-doctest and cargo-deny prepush legs pass; five dynamic
marker cardinalities remain hosted-authoritative, not a local green.
The generated lineage report is restored from its pre-verification user preimage
with copy-back + cmp + touch exit0. Full report: `review-corrections/final-report.md`.

**Final cleanup.** Final record-completeness, epic-coherence and exit-command
gates exit0 before cleanup. The guarded cleanup exits0; `/proc` has no
cargo/rustc/cargo-nextest. Owned scratch/venv and486 stale incremental sessions
are removed; newest-per-crate/config and every last2h-touched session are kept.
Logical unlink6064487193 bytes; unique last-link allocation5121658880 bytes;
observed Cargo-volume free-space gain3897225216 bytes (not conflated with
allocation accounting). No deps/build/fingerprint, other worktree or Main's
`/tmp/2011-wip` archive is touched. `cleanup-final.json` records the exact paths
and accounting. No build/test/formatter/gate runs after cleanup.







### Agent Model Used

anthropic/claude-opus-5-5 (creation).

openai-codex/gpt-6.1-sol (landing-1 T0 STOP investigation and Dev2011b L1 continuation).

### Debug Log References

- [T0 STOP report and command ledger](20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md)
- [Full failing-before output](20-11-erasure-inventory-closure-evidence/landing-1/t0-unload-race-cargo-test.log)
- [Superseded interim observations](20-11-erasure-inventory-closure-evidence/landing-1/l1-wrapup.md)
- [Final restored-source AC witness table](20-11-erasure-inventory-closure-evidence/landing-1/l1-final-witnesses.md)
- [Final exact measurements, commands/exits, once-only prepush and advisory dispositions](20-11-erasure-inventory-closure-evidence/landing-1/l1-final-report.md)
- [Profiled AC5 repair and actual wall/work observations](20-11-erasure-inventory-closure-evidence/landing-1/l1-ac5-repair-evidence.md)
- [Rebased immutable HEAD and final source citations](20-11-erasure-inventory-closure-evidence/landing-1/l1-final-cites.md)
- [Review-correction genuine baseline/preservation reds and exclusions](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/before-state.md)
- [Final formatted-source restored controls](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/final-control-results.md)
- [Narrow accounting transfer and identical test-name lists](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/accounting-transfer-final.json)
- [Final exact formatted source measurement](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/exact-source-partial-redaction.json)
- [Re-measured correction working-tree source citations](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/current-cites.md)
- [Original review/security correction dispositions and remaining boundaries](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/review-resolution.md)
- [Bounded final metadata compilation cells, not test retries](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/metadata-producer-build-cells.md)
- [Correction final AC table, exact measurements, commands/exits and limitations](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/final-report.md)
- [Once-only prepush structured observations](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/prepush-observations.json)
- [Same-run once-only prepush JUnit](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/prepush-final-default-junit.xml)
- [Guarded cleanup paths, process guard and byte accounting](20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/cleanup-final.json)
- [Hosted follow-up original measurements and preserved failures](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/before-state.md)
- [Hosted follow-up real controls and restored checkpoint outcomes](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/control-results.md)
- [Paired native profile comparison](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/native-profile-comparison.json)

### Hosted follow-up 42224167600 — Dev2011b, 2026-11-06

- Adopted Main's clean handoff `fd4f60f15631ba09464b51024bce2c60236b8245` and the four hosted findings without changing their oracles. This continuation is still kernel-free: AC3/KS/O4, hosted-only T3 and the private-tier/index hold lease remain outside L1. Story/tracker status is unchanged.
- Compared the actual compressed native callgrind artifacts from42224167600 and41910498869. The routed-loop delta is41490.6595 instructions/iteration; sqlite prepare contributes30924.7995 (74.53%), not FairMutex, whose measured lock/drop path is cheaper than the reference. The unchanged fixed997449.662 baseline still decides hosted success. The row INSERT now reuses rusqlite's cached indexed program inside the original transaction/publication boundary; no durability, consent, fairness or failure-closed behavior is removed.
- The paired real native allocation oracle initially caught rusqlite0.31's per-cache-lookup Arc<str> key allocation:44483/2000=22.2415 exceeds21.24×1.02. Inlined only the private prepared-delivery accepted-address buffer using the already-locked smallvec dependency, promoted from dev to normal dependency; fan-out still spills, public methods and rupture Vec wire payload stay unchanged. Restoring Vec is genuinely red (22.239); restored production is21.2405, green against the unchanged21.6648 ceiling. The temporary ids buffer remains its original Vec. No local valgrind/callgrind executable is available: allocation evidence is not substituted for hosted instruction proof.
- Replaced the Apple-excluded rustix mkfifoat call with the approved safe POSIX mkfifo subprocess, retaining cfg(unix), forbid(unsafe_code), the real FIFO barrier and existing assertions. The real Linux case passes. The installed aarch64-apple-darwin target check was attempted, but ring's C build fails because GNU cc rejects Apple-specific flags; clang/zig are absent. Apple compile/runtime is explicitly unverified locally, rather than claimed fixed by a Linux compile.
- Completed identical decoded principal/optional-reason requests now replay their durable marker-scoped report from the existing correlation index. Lookup runs inside the started handler, preserving the10s submission receive deadline. A new transport handle/completion audit row links the original erasure operation id; the returned completed body preserves that original id, marker and proof. The stored typed byte report avoids JSON-per-byte construction and remains receipt metadata, not application-body retention.
- Replay validates the actual kernel completion marker, current hold authority and at most4096 insertion-ordered tail rows. New eligible subject bodies, any new distillate, an active hold or the tail cap force a fresh erasure, never a stale complete claim. Non-TL/index/private-state changes are not re-inventoried by historical replay; operators request a fresh inventory with a distinct existing reason. ADR-044 and both operator locales state this boundary, with no new retry policy/API, writer barrier or route deadline.
- The new generation tests exposed an existing composite DESC bug: ORDER BY timestamp_ns,frame_id DESC chooses the oldest timestamp. Corrected both descending keys while preserving the exact old ascending SQL/projection oracle; the first full IAC run's ascending-string failure is retained and repaired in production, not hidden by an assertion change. File-backed and in-memory stores now share the existing correlation index; no new production index/schema/replay table.
- Moved the unchanged physical100k/1M tests into `maos-bin::erasure_million_20_11`, extracting the existing real shell-world helper without duplicating it. Nextest serializes the entire heavy binary. Main explicitly approved the finite78s binary exception: hosted5.646+56.675=62.321s ×1.25=77.90125, rounded up; the other12 shell cases take3.919s. The existing Dev202 model-currency42s exception, literal25% latency,10s route, completion-observation and physical1M fixture oracles are unchanged. This is tests-to-tests organization, not a source-accounting transfer.
- Genuine before/plant evidence and positive checkpoints are in [the control table](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/control-results.md): cached-statement RUN0/2; oldest/newest report; real second forget receipt; fresh matching body; unchecked4096-row tail; current hold; and the native allocation ceiling. Plant A fails2 IAC+1 replay controls, plant B fails3 safety controls, allocation plant fails the actual native metric. Every planted source is copied back, cmp-checked and touched with observed exit0 before its restored positive; no planted branch remains.
- Restored IAC116/116 and restored bin7/7 checkpoints are recorded, including the real SCB schedule snapshot, paired Rust/Python verifier and FIFO. The first integrated34-case focus genuinely failed5 cases: two ordering defects corrected above, missing declared Python backend corrected with the existing hash-locked provisioner, unchanged REPL clarification timing, and the still-open1M first-completion observation. Those raw failures are retained. The hermetic backend uses PYTHONPATH with the runner's interpreter, not an invented verification backend or PATH replacement.
- Local1M baseline82.923s and integrated57.922s fail before reaching the hosted repeat assertion; neither is represented as a local reproduction of that exact repeat failure or as a demonstrated final1M footprint/close. Hosted first completion15.54s/ratio0.0019 is separate evidence; its repeat was red. AC5/EC-13 remains OPEN under20-11 until the full hosted vector passes. Same-line rule-10 timing/I/O dispositions remain under20-7; no retry-to-green, threshold, durability or baseline adjustment.
- Exact formatted production pins against the current handoff: bin35102 (+188), IAC8625 (+3), audit8174 (+0), registry4412 (+0), xtask44676 (+0), aggregate194934 (+191), hardfail194935. Kernel26841 physical/21042 tokei and domain10107 are unchanged. All actual ceilings remain unchanged and satisfied (IAC8625≤8630, registry4412≤4430, aggregate194934≤195404).
- The previously approved unchanged504-code IAC unit-module transfer remains an accounting transfer, not ceiling relief. Original comparison: src8551→8625 (+74 measured), tests2315→3640 (+1325); production would be9129 (+578) with that module still in src, and counterfactual aggregate195438 would exceed195404 by34. This continuation makes no new source-to-tests transfer or ceiling grant. The new bin test organization stays entirely in tests/. [Final source accounting](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/source-accounting-final.json) records both src and test counts.

- Full standalone workspace uses the actual locked670-binary/5846-listed-case producer and native binaries-only reuse schema, with no test/package/concurrency/retry selector. Compilation cells are bounded: the first3000s producer times out without executing cases; continuation exits0 in1144.365s. CLI argument rejection2 and full-list/native-binary-schema rejection96 also execute no case, are retained, and are corrected without test retries. Exact name comparison preserves all15 pre-transfer names, no duplicates/removals, with three new safety controls explaining shell12+heavy6=18.
- The once-executed default-concurrency workspace exits100:5708 run,5702 pass,4 fail,2 time out,138 default-filter skips;181.202s test runtime. Four failures are cert operator-read WouldBlock, hook-budget delivered-versus-quarantined, faithful-successor infer-frame bound, and floor048 typed pending; observer and1M time out120.023/120.006s. All six have same-line rule10 owner20-7 dispositions, functional20-11 ownership retained where applicable, no retry/threshold/deadline/budget change and no claim of unchanged production target or demonstrated cause. The specifically watched refused-successor sibling passes0.881s.
- Actual full-workspace100k vector passes: route pending10.000123606s, observed completion10.548058812s, max insert2.538956458s, ratio0.24070366910654267, recursive allocated peak152236032 bytes. It reaches completed replay, exact tombstone/unrelated-byte, explicit store-close and TempDir-absence assertions. The1M timeout emits no seed/final-peak marker in this run; no stage, final footprint or cleanup pass is invented. AC5/EC-13 remains OPEN for full hosted proof.
- Final source workspace/all-target clippy and unchanged-baseline ratchet exit0 at1329/870 (−54/−27 versus hosted1383/897). Affected EN/KO glossary, bidi, anchors, real build and routes all exit0. [Final follow-up report](20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/final-report.md), same-run workspace JUnit, exact source/name measurements and every failed preflight remain under the new evidence directory.

- Final20-command bundle exits0: fourteen prescribed structural/record gates, workspace fmt-check, and all five affected EN/KO documentation checks. Kloc/kernel gates validate exact194934/hardfail194935 and unchanged26841/21042 kernel. Source measurements, coverage, env contract, service/composition boundaries, serde/public constructors, exit/decision register and Dev Agent Record gates are all exercised, not invented aliases.
- Resource attribution after the1M timeout finds its deterministic seed frame in the case-time-matched directory, checkpoint max_rowid937988 (below physical1M), and no owned surviving process. The positively identified aborted fixture is removed before prepush: apparent1396526824 bytes / file allocation1396523008 bytes unlinked, observed free-space gain1294843904 bytes; other candidate directories are untouched. This is aborted-resource cleanup, not a passing1M final peak/TempDir assertion. Source/data probe and attribution records are preserved.

- Once-only prescribed prepush completes within its3000s cell: exit1 in2943.623s,84 pass/1 red/1 advisory/6 skips. The sole red is workspace-nextest; same-run JUnit5707/5708, sole1M first-completion observation failure50.162s after pending10.000723955s (seed allocated1487568896 bytes). Seven unchanged-budget advisories have same-line20-7 rows; the new heavy binary61.897s is below its approved78s. Earlier standalone occurrences are retained, not retired by these later positives; no second prepush or retry-to-green.
- Prepush's six skips are five local CI-marker cardinalities plus cargo-deny because Cargo.lock/deny.toml did not change. The promoted existing dependency edge is nevertheless explicitly checked: cargo deny check exits0 in1.351s. AC6 prints its real named skip: signed T3 substrate not provisioned, provision_t3_tests.py/t3.env required. Hosted native instructions, Apple compilation/runtime and fullAC5/T3 evidence remain unverified. The generated intent-lineage report is restored exactly from its saved preimage with copy-back+cmp+touch0.

- Closing exit-command, decision-register, Dev Agent Record and epic-coherence checks each exit0 after the final prepush/deferred records. Final guarded cleanup exits0: /proc has no cargo/rustc/clippy/rustdoc/nextest process, own scratch removed and544 stale incremental sessions pruned while retaining newest/recent2h sessions. Logical8111838202 bytes / last-link file allocation6504443904 bytes unlinked; observed Cargo-filesystem free-space gain5914886144 bytes. Earlier aborted-fixture cleanup is separate (1396526824 logical/1396523008 allocated bytes). All completed raw evidence is archived; no deps/build/fingerprint, other-worktree, .devcontainer or parked-archive deletion. No checks after final cleanup.

### Hosted discipline observation — 2026-11-08

Main run [`42533166219`](https://github.com/lunarpulse/maos/actions/runs/42533166219) at `a13140b847a1dfe888384670e56bcaaad4c4d116` is **failure**, not a green discipline run: workspace 5835/5836, sole failure the previously recorded 20-7 snapshot-owner teardown timing family. The 100k continuous-writer and 1M completion/repeat witnesses pass in 4.698 s and 40.957 s respectively (workspace job `128891635866`, raw log lines 8851 and 9145). These are first hosted passes of both full vectors, not printed numeric ratios; AC5/EC-13 remains open until final review accounts for all AC5 clauses and plants.

The run's epic-17-exit matrix, check-test-budget, check-perf-counts and macos-check jobs succeeded. IAC routing measured 960868.4 instructions against 997449.662 baseline; the hosted JUnit sum was 1133.876 s against 1440 s. Apple ARM all-target compilation passes, not FIFO runtime. Signed T3 substrate was provisioned and the AC6 test is reported PASS, but its skip guard makes live-Worker execution unverified from the log alone. AC3/KS/O4 and private/index hold lease remain final-kernel work. Exact job citations and AC5 clause boundaries: [hosted evidence](20-11-erasure-inventory-closure-evidence/landing-1/hosted-evidence.md). Status remains `ready-for-dev`.


### Landing 2 — final kernel landing (Dev2011k, 2026-11-11, base `345ae30f`)

**Scope:** KS `ScheduleWatchdog::forget_spirit`; O4 (closed-pid fence in the single receipt writer, typed `SpiritUnloading(pid)`, silent unload drain deleted); AC3 pending halts (door count from `planned_unload` receipts read back after the unload frame, pre/post registry checks; offline orphan closure through `terminate_spirit` on a transient registry; typed `halt` removal evidence in both verifiers; `NotFound` now also requires no orphans/pre-unload halts); AC4 door schedules `Removed{n}` only after `forget_spirit` and a stamp-residue check; one `ScheduleWatchdog` built (`new_schedule_watchdog`) only by its consumer — a door root builds it before its door and parks a `Weak` in a root-scope slot; the serving tail and `LifecycleCallers::start` upgrade it (or build their own). KH/KF not landed.

**T0 (measured before editing):** `landing-2/t0-measurement.md`. Prototype = creation KS diff + landing-1 O4 diff, rustfmt'd, prebuilt gate binary: kernel +26 physical/+18 tokei, domain +4 — exactly the grant; ceilings 26,893/21,078, STOP 21,250 not approached. Landed formatted code has the identical set hash `affee8b0…`.

**Exact pins:** kernel `src_lines` 26,880, set hash `affee8b007d65000b63a3776cf59bd5d3fafb874cc818bc68133bbf3ef41a883`, per-file `halt/mod.rs` `b77703f8…`, `halt/termination.rs` `043a2a54…`, `scheduler/schedule_watchdog.rs` `3b9d8431…`, `scheduler/scheduler_loop.rs` `95bd5a33…`; kloc kernel 21,069, domain 10,215, bin 38,202, audit 8,313, `_aggregate_hardfail` 202,371 (clean aggregate 202,370); xtask pin tests re-pinned (26,880 / 21,069). Surface: single `HaltRegistry` hash refresh (`kernel-surface-v0.1-beta.json:618`, ADR-066 hand-edit), no class row.

**Overdraft ruling (orchestrator, D02 A, disclosed to the operator):** audit ceiling 8,261 → 8,313 (+86 = `orphaned_halts`, `planned_unload_receipts`, typed `halt` evidence; relocating to maos-bin would only move the overdraft); bin's stale-absolute story ceilings are not enforceable against 38,061 — the live bin ceiling is the measured pin plus this story's own remaining allowance (602 at the L1 pin; 141 used). Knock-on: 20-1b's "audit remaining 82" is moot; later stories re-base on 8,313.

**Witnesses / plants:** `landing-2/plants.md` (witness name:line, failing-before log, plant log, final log). Final-tree green: kernel O4+KS 4/4, kernel `--lib` 253/253, audit `erasure_review_20_11` 11/11, bin `erasure_inventory_20_11` 20/20, door `shell_host_16_2 ac3_/ac4_` 2/2, composition `main.rs ac3_/ac4_/review_p9` 7/7, live `erasure_live_20_11` 2/2 (Worker leg skips without T3), xtask pin tests 9+25+12.

**Review-relevant facts:** (1) a halt in flight when the pid is closed is committed first and receipted by the drain; a later halt is refused and writes nothing (no TL row, no Journal entry). (2) `terminate_spirit` callers (unload, crash, revocation) all fence. (3) Hot swap (K1p) never calls the writer: unchanged. (4) The offline closure writes receipts stamped with the offline process's boot nonce at the old pid; the receipt payload carries the incarnation's `boot_nonce`/`pid`, and orphan detection keys on the payload's `halt_id` (ULID), not the row boot. (5) A `halt.resolve` approval row (target = Spirit, reasoning prefix `halt=<id>:`) counts as a resolution. (6) Plant (c) of the pricing (close in unload only) has no realistic variant in this design; plant (a) reds the crash witness.

**Defect found and fixed by the workspace nextest (observed):** the first full run (5987 cases) failed 20, 19 of them `audit writer drain timed out after 5s` (one-shot, journey, uninstall, butler, door-disabled `run`): my first cut built the shared `ScheduleWatchdog` unconditionally in `main`, and an unspawned instance holds the capability registry's audit sender, so the drain never closed. Repaired by building it only in a door root and parking a `Weak` (a strong slot in main scope still stalled door-disabled `run` roots: observed on re-test). The 20th failure was the known local `erasure_million_20_11` AC5 1M oracle (not retried or widened). The nine affected bin binaries and the two journey binaries then passed (log `final/retest2-bin.log`, `final/retest3-journey.log`); the final one-workspace nextest is in `final/workspace-nextest-final.log`.

**Unverified / open:** AC5/EC-13 (hosted serial 1M pass, `erasure_million_20_11` local oracle ~21 s vs 20 s allowance, root-caused, not widened), AC6 live T3 leg (hosted-only), private/index hold lease (`kernel_private_index_hold_lease` gap, separate grant required), hosted discipline run on the final tree.

### Completion Notes List

- The historical `cbed4dad` local landing claims above are superseded by the review-correction Dev Agent Record and evidence. The correction remains kernel-free; AC5/EC-13 is explicitly OPEN pending hosted <25% wall and 1M completion, and AC6's T3 substrate is unverified locally. AC3/KS/O4 and the private-tier/index hold lease remain final-kernel work, not implemented here. Original T0 evidence, failures and rulings are preserved; no tracker flip.
- The2026-11-06 hosted follow-up adds durable marker-scoped replay, indexed program reuse with paired allocation protection, portable FIFO creation, and the explicitly calibrated serial heavy-test binary. These implementation changes do not close the still-hosted fullAC5 or Apple/T3 evidence obligations.
- 2026-11-08 hosted evidence supersedes the preceding historical pending-hosted and Apple-compilation statements: both large-log vectors passed and Apple ARM all-targets compiled; AC5 formal closure, AC6 live-Worker execution and final-kernel work remain open.

### File List

- `20-11-erasure-inventory-closure.md` — T0 evidence history, ruled O4 grant, amended landing split and Dev Agent Record.
- `20-11-flag-winston-decision.md` — granted KS + O4, measured/ceiling distinction and final-landing protocol.
- `20-11-erasure-inventory-closure-evidence/landing-1/` — witness source, complete failing-before log,
  baseline measurements, command ledger and STOP report.
- `deferred-work.md` — existing second-drain risk upgraded to observed, same owner; no closure.
- `sprint-status.yaml` — dated shared-STOP/grant/landing notes; 20-11 remains ready-for-dev.
- `20-1b-vetting-yank-policy-and-quarantine.md`, `20-4-gemini-driver-and-endpoint-pin.md`,
  `20-8-manifest-contract-completion.md`, `20-9-spawned-form-lifecycle-hooks.md`,
  `20-13-pluggable-crypto-provider-selection.md`, `20-14-complete-cost-attribution.md`,
  `20-5-v1-0-rc-tag-and-lts-rule.md` — dated shared-STOP supersession, historical pricing retained.
- `20-4-flag-winston-decision.md`, `20-8-flag-winston-decision.md`, `20-9-flag-winston-decision.md`,
  `20-13-flag-winston-decision.md`, `20-14-flag-winston-decision.md` — same shared-STOP supersession.
- `../planning-artifacts/epics/epic-20-ship-it-w5.md`,
  `../planning-artifacts/epic-20-preflight-2026-10-30.md` — cumulative ledger and 20-11 landing amendment.
- `20-11-erasure-inventory-closure-evidence/landing-1/late-halt-repair-pricing.md`,
  `20-11-erasure-inventory-closure-evidence/landing-1/t0-stop.md` — dated subsequent rulings above preserved evidence.
- L1 continuation source/tests: `crates/maos-audit/src/erasure/proof.rs`, `crates/maos-audit/tests/erasure_smoke_test.rs`, `crates/maos-iac/src/adapter/transparency_log.rs`, `crates/maos-registry/src/storage.rs`, `crates/maos-bin/src/{main,operator_door,hook_pump}.rs`, `crates/maos-cli/src/cli.rs`, `tools/verify-erasure/verify.py`, `crates/maos-bin/tests/{erasure_inventory_20_11,erasure_live_20_11,erasure_scan_20_11,shell_host_16_2}.rs`, `crates/maos-bin/tests/support/worker_revoke_root.rs`.
- L1 profiled scan companion: `crates/maos-iac/src/adapter/erasure_scan_work.rs` — test-only actual SQLite VM-work meter,100k/1M predicates, ordering and opaque-outcome contract vectors.
- L1 writer-inventory closure: `crates/maos-audit/src/fr4_classifier.rs`, `crates/maos-audit/tests/fr4_classifier_16_2.rs` — measured production redaction marker disposition and explicit external test-file compilation boundary.
- L1 continuation records/pins: this story, `deferred-work.md`, `docs/adr/ADR-044-gdpr-article-17-distillate-redaction.md`, `tests/coverage-matrix.yaml`, `xtask/kloc.toml`, and `20-11-erasure-inventory-closure-evidence/landing-1/l1-*` (original T0 artifacts retained).
- Review-correction proof/scanner cutover: `crates/maos-audit/src/{lib,fr4_classifier}.rs`, `crates/maos-audit/src/erasure/{proof,merkle}.rs`, `crates/maos-audit/tests/erasure_review_20_11.rs`, smoke/classifier callers, and `crates/maos-loom-lite/src/store.rs`.
- Review-correction IAC identity/serializer/marker/index work: `crates/maos-iac/src/adapter/transparency_log.rs`, its `transparency_log/erasure_sweep.rs` child, `erasure_scan_work.rs`, `crates/maos-iac/tests/support/{transparency_log_unit,erasure_races_20_11}.rs`, `crates/maos-iac/tests/erasure_index_20_11.rs`, and dependency edges in crate manifests/`Cargo.lock`.
- Review-correction finalization/real recovery: existing bin cascade/door/pump sources, `crates/maos-registry/src/storage.rs`, `crates/maos-registry/tests/uninstall_review_20_11.rs`, inventory/scan/live/shell tests, `crates/maos-bin/tests/support/uninstall_review_20_11.rs`, and preserved26-case `crates/maos-bin/tests/erasure_uninstall_13_5b.rs`.
- Review-correction hermetic verification/fixture lifetime: `.github/actions/provision-erasure-verifier/action.yml`, `.github/workflows/discipline.yml`, `tools/verify-erasure/{verify.py,requirements.txt}`, `scripts/run_cgroup_ceiling_17_2.py`, `xtask/tests/erasure_verifier_runner_20_11.rs`, and `.config/nextest.toml`.
- Review-correction records: this story/frontmatter, `deferred-work.md`, ADR-044, `docs-site/docs/run-maos.md`, coverage matrix, all six moving `xtask/kloc.toml` rows, and `20-11-erasure-inventory-closure-evidence/landing-1/review-corrections/`. Historical file-list entries above are retained, not a claim that this correction edits the tracker or other stories.
- Affected Korean operator mirror: `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md` — real marker/pending/receipt/retry/backend/numerical-fact translation, fixing the observed glossary drift; no keyword padding or waiver.
- Hosted-follow-up production: `crates/maos-iac/src/adapter/{transparency_log,mailbox}.rs`, `crates/maos-iac/Cargo.toml`, and `crates/maos-bin/src/operator_door.rs` — cached INSERT, inline accepted buffer, existing correlation lookup and bounded durable replay; no kernel file.
- Hosted-follow-up tests: `crates/maos-iac/tests/support/erasure_races_20_11.rs`, `crates/maos-bin/tests/erasure_inventory_20_11.rs`, `crates/maos-bin/tests/shell_host_16_2.rs`, new `crates/maos-bin/tests/erasure_million_20_11.rs` and `crates/maos-bin/tests/support/shell_world_16_2.rs` — real regression controls, portable FIFO and unchanged heavy vectors/helper transfer.
- Hosted-follow-up records/configuration: `.config/nextest.toml`, `xtask/test-budgets.toml`, `xtask/kloc.toml`, this story, `deferred-work.md`, ADR-044, EN/KO operator docs, coverage matrix and `20-11-erasure-inventory-closure-evidence/landing-1/hosted-followup-42224167600/`; existing T0 and prior failed evidence retained.
- Hosted evidence record: `20-11-erasure-inventory-closure-evidence/landing-1/hosted-evidence.md`.
- Landing 2 kernel: `crates/maos-kernel-core/src/{halt/mod.rs,halt/termination.rs,scheduler/schedule_watchdog.rs,scheduler/scheduler_loop.rs}`, `crates/maos-domain/src/halt.rs`, `docs/ci-baselines/kernel-surface-v0.1-beta.json` (single HaltRegistry hash), `xtask/kernel-core-baseline.toml`, `xtask/kloc.toml`, `xtask/tests/{kernel_pin_content_hash_16_0,recovery_lane_ceiling_rule}.rs`; new `crates/maos-kernel-core/tests/{halt_unload_fence_20_11,schedule_forget_spirit_20_11}.rs`.
- Landing 2 production: `crates/maos-audit/src/{lib.rs,erasure/proof.rs}`, `tools/verify-erasure/verify.py`, `crates/maos-bin/src/{main,operator_door,spawned_lifecycle}.rs`.
- Landing 2 tests/records: `crates/maos-audit/tests/erasure_review_20_11.rs`, `crates/maos-bin/tests/{erasure_inventory_20_11,erasure_live_20_11,shell_host_16_2}.rs`, `crates/maos-bin/tests/support/{shell_world_16_2,uninstall_review_20_11,worker_revoke_root}.rs`, `20-11-flag-winston-decision.md`, `deferred-work.md`, ADR-044, `tests/coverage-matrix.yaml`, `sprint-status.yaml`, epic-20/preflight mirrors, `20-11-erasure-inventory-closure-evidence/landing-2/`.

## Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D-20-11-01…10 PROPOSED). Inventory measured: `intent_lineage`
  `VerifiedEmpty` is false for principal-free distillates; pending halts are drained uncounted (door) or left open
  (offline); schedules stop at unload but watchdog state stays and installed Spirits are re-admitted; a principal-free
  Spirit gets `not_found` and no proof. Kernel optional: KS +10/+6 recommended, KH +10/+6, KF +37/+33 priced by
  reverted prototype; maos-iac +90 measured.
- 2026-11-02: Operator rulings recorded (ask s2011): D-20-11-01…10 as recommended; kernel KS and ceilings per frontmatter. Status remains ready-for-dev.
- 2026-11-04: Landing-1 T0 STOP at `73e7afbc`: real unload silently discards a halt invoked after its first drain;
  witness exit 101. D-20-11-11 proposed; no kernel/runtime/pin/status changes and no implementation closure.
- 2026-11-04: Operator ask `halt_grant` ruled D-20-11-11 **A / O4** and raised the whole-ledger measured STOP
  to **21250**; KS + O4 measured 26867/21060, binding D02 A ×2 ceiling 26893/21078, domain ceiling 10115.
  Ask `halt_landing` moves AC3 to final kernel KS + O4; L1 AC1, AC2, AC4–AC7 remains kernel-free.
  T0 STOP history/red preserved; status stays ready-for-dev; no source, pins, cargo or gates in this recording pass.
- 2026-11-04: Dev2011b adopted inherited L1 work, established original-HEAD reds and restored plants, parked/rebased onto `cb0abc06` for the priority hotfix, and implemented kernel-free inventory/proof/scan/package changes. The interim harness wrap-up notice was not an operator stop; Main instructed continuation. Profiling then repaired the actual kind access path and missed recovery scan; final focused/preservation and literal100k/1M wall/work witnesses pass, exact pins remain within grant. Historical reds are retained; hosted T3 and final-kernel work remain unverified/untouched. No tracker flip, commit, push or kernel change.
- 2026-11-05: Final default-concurrency standalone workspace observed one real FR4 inventory failure; measured production disposition and guarded external-test scanner boundary fixed with red-before controls and20/20 focused verification. Exact current pins193911/hardfail193912, kernel/domain unchanged; final clippy delta-54/-27. Once-only final prepush exits0 with5640/5640 final-tree cases and four unchanged-budget aggregate advisories explicitly owned20-7. Hosted/final-kernel proof remains owed; no tracker flip or commit.
- 2026-11-05: Dev2011b review corrections at baseline `93d39cc7`: stable/admission-qualified identity, strict paired verifiers/evidence, recoverable last-operation package finalization, durable aggregate marker and held/malformed truth, partial indexes, retained stdin and bounded-lifetime real disk fixtures. Genuine preservation reds repaired without lowering old oracles; final focus333/333, final source gates0, clippy0/advisory delta-54/-27. Exact aggregate194741/hardfail194742; registry re-base4412 explicitly ruled; IAC accounting transfer disclosed, ceilings unchanged. Final-source plants5+1+1 red/restored. AC5/EC-13 remains OPEN and hosted/T3/kernel legs unverified; final-tail API-only replay is an accepted ownerless/open limitation. No tracker flip, commit, push or kernel edit.
- 2026-11-05: The required standalone workspace exits100 (5686/5688,138 skipped): one genuine IAC partial-cost prior-source regression and the still-open1M completion deadline. The generic prior-source predicate is repaired kernel-free; unchanged kernel oracle/346-case focus pass, exact OR plant reds/restores. Exact final aggregate194740/hardfail194741, IAC8622; numerical facts remain citable. Real100k ratio0.1164494388 and recursive peak152236032 bytes pass;1M seed1487568896 bytes observed, completion/post-peak unproven. No retry-to-green, threshold/deadline/budget change, kernel edit or tracker flip.
- 2026-11-05: Once-only correction prepush exits1 in1195.222s (84 pass/2 red/1 advisory/5 local marker skips); same-run JUnit5687/5688, sole1M completion deadline. The genuine EN/KO glossary red is repaired with the affected Korean translation and five focused docs checks0, without another prepush. Six unchanged-budget aggregate advisories and the1M occurrence have same-line20-7 dispositions. Final clippy0/1329-870, post-restore cascade13/13, refreshed fixtures/native build0; final exact pins194740/hardfail194741. Hosted AC5/T3 and final-kernel work remain open, no tracker flip.
- 2026-11-05: Final record gates0; cleanup0 with no active cargo/rustc/nextest. Removed own scratch and486 stale incremental sessions, keeping newest/recent; logical6064487193 bytes and last-link allocation5121658880 bytes unlinked, observed Cargo free-space gain3897225216 bytes. No checks after cleanup, no deps/build/fingerprint, other-worktree or parked-archive deletion.
- 2026-11-06: Hosted42224167600 follow-up from cleanfd4f60f1 implements cached indexed INSERT plus allocation-neutral inline delivery storage, safe POSIX FIFO creation, durable marker-scoped replay with generation/hold/tail controls and preserved transport deadline, and unchanged physical100k/1M vectors in the explicitly approved78s serial binary. Genuine before/plant failures and restored positives are preserved; no perf/latency/deadline/oracle widening, new crate version, kernel change or tracker flip. Exact aggregate194934/hardfail194935; AC5/full hosted proof remains open.
- 2026-11-06: Final follow-up source/record/fmt/docs bundle20/20 and explicit cargo-deny0; actual default-concurrency standalone5702/5708 with six retained timing/I/O observations, once-only prepush5707/5708 with sole1M completion observation and seven unchanged-budget advisories. Prepush2943.623s stays within3000s; no retry, baseline/deadline widening, kernel edit or tracker flip. Exact pins194934/hardfail194935; source/name/control records and resource cleanup are retained. AC5/full hosted proof stays open.
- 2026-11-06: Closing four record gates0; guarded owned-scratch/incremental cleanup0 removes544 stale sessions,8111838202 logical bytes /6504443904 last-link allocated-file bytes, with5914886144 observed Cargo free-space gain. Earlier positively identified aborted1M fixture cleanup is separately recorded; no source/pin/status change or checks after cleanup.
- 2026-11-08: Recorded hosted run `42533166219` @ `a13140b8`: 100k/1M vectors pass; workspace fails only the known 20-7 snapshot-owner timing family. AC5/EC-13 stays open for final review, AC6 live Worker remains unverified from the log, final-kernel work remains owed, and status is unchanged.
- 2026-11-11: **Loom whole-package leg skips the 1M vector** (Dev20Hosted; operator ruling 2026-11-11, ask `m1`, option 3 only): `check-multi-tenant-loom`'s `maos-bin-whole-package-parallel-isolation` leg now runs `cargo test -p maos-bin -- --skip ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` (default parallelism and the three required `cross_team_consent_13_3` tests unchanged; test `whole_package_leg_skips_only_the_million_row_test`, red at `459a606a`). The heavy vector keeps its hosted serial-group coverage in the workspace nextest run (`.config/nextest.toml`); the loom leg is the `MAOS_HOME` race witness, not a 1M witness. No 20-11 oracle, deadline or budget changed. **AC5/EC-13 stays OPEN** until a hosted serial pass of that vector is cited at the final 20-11 landing; this skip is not that evidence.
- 2026-11-11: **Landing 2 (final kernel landing: KS + O4 + AC3), Dev2011k, base `345ae30f`** (uncommitted, awaiting orchestrator commit/review). T0 on a reverted `git archive HEAD` prototype: kernel 26854/21051 → 26880/21069 (**+26/+18 = grant**), domain 10211 → 10215 (+4); one HaltRegistry surface-hash refresh `b6d65243…` → `a917eddc…`. Every witness green in the final tree with red-before and plants (`landing-2/plants.md`). **Orchestrator-adjudicated overdraft, disclosed to the operator (D02 A):** maos-audit 20-11 ceiling 8,261 → 8,313 (pin 8,227 → 8,313, +86); maos-bin pin 38,061 → 38,202 (+141); aggregate hardfail 202,122 → 202,371. **Status stays in-progress:** AC5/EC-13 (hosted serial 1M pass), AC6 live T3 leg (hosted-only) and the private/index hold lease (named gap, separate grant) remain open.
