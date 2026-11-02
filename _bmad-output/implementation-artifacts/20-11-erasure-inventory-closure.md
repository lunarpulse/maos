---
story_id: '20.11'
story_key: 20-11-erasure-inventory-closure
created: 2026-11-02
context_commit: a8af3215
baseline_commit: a8af3215
security_sensitive: true
kernel_grant: "RULED 2026-11-02 (ask s2011; D-20-11-04 A): KS ScheduleWatchdog::forget_spirit measured at a8af3215 from 26841 physical / 21042 tokei, +10 / +6. Ceilings (STOP above): kernel ≤ 26861 physical / 21054 tokei; kernel lands last. The second or later kernel landing re-bases; cumulative STOP above 21209 tokei, ADR-069 alarm 22000; expected cumulative with 20-1b +68 and 20-4 +9 is 21125."
kloc_grant: "RULED 2026-11-02 (ask s2011; D-20-11-08 A; D02 A ×2 measured / ×3 unmeasured). Ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): kernel ≤ 26861 / 21054; maos-iac ≤ 8630; maos-audit ≤ 8261; maos-bin ≤ 34698; maos-registry ≤ 4368; aggregate ≤ 194058. Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:57-71`): an attesting control (a signed proof-of-erasure, rule 11), cross-Spirit isolation (pid reuse across boots, legal holds), concurrency (EC-13 lock windows, forget/hold serialization) and, if ruled, a kernel FLAG-Winston row. Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC2.4, AC3, AC5.3 and the proof format (D-20-11-09), Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.11: Erasure inventory closure — per-Spirit proof of erasure

Status: ready-for-dev

<!-- Created 2026-11-02 at a8af3215 (main; origin 47631284) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30). D-20-11-01…10 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran: cites re-read at HEAD (`creation/cites-a8af3215.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); prototypes saved as diffs, never applied. Stale epic cites are corrected here and owed to the epic (`creation/records-owed.md` §A). -->

**Status rationale — ready-for-dev.** Prerequisite 20-3c is `done` (PF:129). Nothing needs a machine the repository
lacks: every category is provable on the hermetic offline harness and the door; only AC6 (a running spawned Spirit)
needs the hosted T3 substrate, which exists (`provision-t3-test-substrate`). Kernel rows are optional and land last
(D-20-11-07 A).

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

## Decisions — RULED 2026-11-02 (ask s2011)

**Rulings 2026-11-02 (ask s2011):** The operator ruled every D-20-11-01…10 row exactly as recommended, including D-20-11-02 B, D-20-11-03 B and D-20-11-04 A. Kernel KS and the ceilings are granted as recorded in frontmatter.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-20-11-01** | What "intent lineage references" are enumerated and removed | **A** the Spirit's own Distillate frames at its incarnations (D-02) via `distillates_written_by` (P2), body-scrubbed with the ADR-044 tombstone (reason `spirit-uninstall`) + a `distillate.redacted` marker naming the Spirit, ids into `erased_distillate_frame_ids` (pre-inclusion proofs); a distillate whose derivation names a held principal stays and is a named gap; IAC-frame `intent_lineage` arrays (NFR-Aud-14 audit chain) retained and named in the reason · **B** A + rewrite the lineage arrays of every IAC frame from/to the Spirit · **C** enumerate only: `CoverageGap` with count and ids | **A.** I13 lineage persists only in kind-11 bodies (`distillate.rs:136`); writer = citer (`:348-377`), so a Spirit's distillates are its own. B rewrites counterpart Spirits' audit chain (`frame.rs:36-50`) with no tombstone convention; C keeps data FR65 says is removed. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-02** | Which incarnations the new categories cover | **A** latest boot only (today, `all_boots=false`, `main.rs:10560`) · **B** every boot, keyed `(boot_nonce, pid)` (`resolve_spirit_name(…, true)`, `lib.rs:1758-1763`), for lineage and orphaned halts; existing principal discovery unchanged · **C** B + re-key the existing pid-only lookups (`principal_ids_for_spirit_pid`, `revoke_all_for_pid`) | **B.** Boot-local pids repeat across boots (`deferred-work.md:1320`); latest-only signs `Removed` over earlier boots' distillates. `(boot_nonce, pid)` never touches another Spirit. C reopens shipped principal discovery that 20-3b's stable-owner path covers (`erasure_uninstall_13_5b.rs:1197`). | RULED 2026-11-02: **B** (ask s2011) |
| **D-20-11-03** | Pending halts: how counted and removed | **A** KH (+10/+6): door counts `unload_receipted` receipts (minus the `term-…` marker); offline: orphaned halts (invocation frame at an incarnation, no terminal receipt, no `halt.resolve` decision) closed through `terminate_spirit` on a transient `HaltRegistry` with `UnplannedCrash` · **B** kernel-free: door pre-reads `drain_for_spirit_dry_run(pid)`, post-verifies `halt_metadata_for_spirit(pid, 0)` empty, counts the `PlannedUnload` receipts by TL read-back after the unload frame; offline as A · **C** enumerate only, no receipts | **B.** Exact without kernel lines; same single receipt writer (PF:212). A is the same evidence through a kernel API. C leaves open halts for an erased Spirit. T0 measurement 2 (second unreceipted drain, `scheduler_loop.rs:574`) can reopen this as STOP + D-row. | RULED 2026-11-02: **B** (ask s2011) |
| **D-20-11-04** | Scheduled invocations | **A** KS (+10/+6): door enumerates `[[schedule]]` ids from the SCB snapshot before unload, then `forget_spirit` drops the watchdog's buckets and stamps; offline: ids from the installed manifest, or for a compiled-in Spirit a named gap ("re-admitted by the binary at next boot") · **B** as A without `forget_spirit`; the reason names the per-process state retained until root exit · **C** enumerate only | **A.** The rate bucket keyed by Spirit id (`schedule_watchdog.rs:97`) survives uninstall and a same-id reinstall in the same root inherits it; six tokei make `Removed` literally true. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-05** | The installed registry package | **A** on an `Erased` terminal remove the index entry and generation dirs (`uninstall_package`), category `installed_package` `Removed{1}` / `VerifiedEmpty`; `Held`/`Failed` keep it and say so; `cli.rs:63-66` becomes true · **B** keep it; `installed_package` `CoverageGap` "the next serving boot re-admits it" (`main.rs:4308-4313`); CLI doc corrected · **C** route removal to 20-1b | **A.** An uninstalled Spirit re-admitted at the next boot, schedules included, is not uninstalled; the crate change is small. Coordinate `storage.rs` with 20-1b. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-06** | EC-13 shape | **A** kernel-free: rowid windows (P1); sweeps re-check this principal's hold and the hold set per window under the TL lock; cascade and door forget leave the async worker (`block_in_place`, multi-thread root `main.rs:1856`); door release via `MemoryManagerAdapter::release_legal_hold` · **B** A + KF (+37/+33): the late sweep runs under the kernel serializer · **C** off-worker only, no paging | **A.** `place_legal_hold` writes under the same TL lock, so each window is linearized against placement; B buys the same guarantee for 33 kernel tokei; C keeps the writer stall EC-13 names. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-07** | Landing shape | **A** one key, two landings: L1 kernel-free (every AC; a pending kernel category reports its kernel-free reason), L2 the ruled kernel row(s) last, re-based · **B** one landing · **C** split key for the kernel row | **A** when any kernel row is ruled (kernel last, 20-3c/20-7 precedent); collapses to one landing if none is. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-08** | Per-crate envelope (D02 A applied) | **A** the envelope above (×2 measured / ×3 unmeasured) · **B** exact measured grants, every correction re-asks · **C** ×1.3 (rule 6) | **A.** 20-3b and 20-3c landed at 2–3× creation forecasts; most of this story is unmeasured. | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-09** | Proof evidence for categories without frame proofs (schedules, halts, package, tokens) and principal-free Spirits | **A** each claimed status is backed by a `principal.uninstall.category` TL evidence frame written before the post-tree capture (excluded from forget scans by `transparency_log.rs:1589`); the bundle carries post-inclusion proofs for them; schema bump; Rust and Python verifiers check per category; `NotFound` only when every category is empty · **B** narrow P18 to frame-backed categories; others signature-attested and labelled so · **C** keep P18 and `NotFound` as is | **A.** NFR-Aud-12 asks for an externally verifiable receipt; B weakens verification; C fails FR65 for every principal-free Spirit (`main.rs:10721-10741`). Fixes the Python/Rust P18 mismatch (§C R2). | RULED 2026-11-02: **A** (ask s2011) |
| **D-20-11-10** | Inventory beyond FR65's list | **A** every other per-Spirit store found at T0 (creation list: FR11 manifest archive, skill queue entries by author pid, Lifecycle Journal) is a named category with a counted status and reason — never silent · **B** erase them too · **C** FR65's list + package only | **A.** NFR-Aud-12's owner row (`deferred-work.md:78`) asks for the complete inventory; B reaches into FR11 migration refusals (`admission.rs:847-855`) and 18-7 skill revisions; C signs a silent omission. | RULED 2026-11-02: **A** (ask s2011) |

## Acceptance Criteria

*(**local** = hermetic dev host, offline harness or a door test; **hosted** = GitHub-hosted branch run. Witness
harness for the offline path: `crates/maos-bin/tests/erasure_uninstall_13_5b.rs` (TL and stores seeded in-test, the
real binary run). Every failing-before is observed at `a8af3215` before the change and recorded.)*

### AC1 — no category is labelled by assumption (epic AC1)

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

1. The Spirit's distillates at all its incarnations (D-02 B) are found by `(boot_nonce, pid)`, scrubbed to the
   ADR-044 tombstone with a marker naming the Spirit, and enter the bundle's pre-inclusion proofs; held derivations
   stay and are listed; IAC-frame lineage is retained and named; cost frames are untouched.
2. **Failing-before (local):** Spirit with one principal and one principal-free distillate → proof says
   `intent_lineage: VerifiedEmpty` while the body still carries `intent_lineage`. After: `Removed{1}`, tombstone,
   marker, inclusion proof verifies. Variant: the distillate written in an earlier boot is removed too.
3. **Empty witness:** no distillates → `VerifiedEmpty` from the query.
4. **Isolation (security):** another Spirit's distillate at the same pid in another boot is byte-identical after.
5. **Plant (local):** restore the `VerifiedEmpty` constant ⇒ AC2.2 reds.

### AC3 — pending halts enumerated, removed, proved (D-20-11-03)

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
   one rowid window per TL lock hold; rows appended during a scan are reached by later windows; receipt order stays
   (timestamp, frame id).
2. **Failing-before (local):** a ≥ 100k-frame TL (size from T0 measurement 3), a continuous concurrent
   `insert_frame_event` writer during a door forget: max single-insert latency < 25 % of the forget's wall time;
   at HEAD ≈ 100 %.
3. **Serialization (security):** (a) a hold placed on P between its forget and the late sweep → a late frame naming P
   survives, reported held — at HEAD scrubbed (red); (b) the door's hold release waits for an in-flight forget — at
   HEAD it returns first (red); (c) the 13.5b/18-6 hold suites stay green.
4. Cascade and door forget leave the async worker. No witness: on the multi-thread root a starvation probe is not
   deterministic; reviewed structurally.
5. **Plant (local):** window size `i64::MAX` ⇒ AC5.2 reds.

### AC6 — a running spawned Spirit (epic AC2's T3 clause)

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
- [ ] **T1 — Proof format (AC1, D-09).** Evidence frames, per-category verification in `proof.rs` and `verify.py`,
  `NotFound` rule; failing-before + plant.
- [ ] **T2 — Intent lineage (AC2).** `distillates_written_by`, scrub + marker, all-boot incarnations; witnesses + plant.
- [ ] **T3 — Pending halts (AC3).** Door count + post-verify; offline orphan query and receipts; witnesses + plant.
- [ ] **T4 — Schedules and package (AC4).** SCB snapshot enumeration, watchdog handle, `uninstall_package`, CLI doc;
  witnesses + plant.
- [ ] **T5 — EC-13 (AC5).** Rowid windows, per-window hold re-check, off-worker forget, serialized door release;
  witnesses + plant.
- [ ] **T6 — Inventory (AC7).** Typed store list, named categories, vector + plant.
- [ ] **T7 — T3 leg (AC6).** Hosted branch run under `provision-t3-test-substrate`; run id.
- [ ] **T8 — L1 landing (kernel-free).** Exact pins inside the ruled envelope; `check-kernel-baseline` unchanged;
  `kloc-check`, `check-service-boundary`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`;
  one hosted discipline run on the final tree. No commit or push without the operator.
- [ ] **T9 — L2 kernel (if ruled).** KS (and KH/KF if ruled) last; re-base onto the then-current pin; FLAG-Winston
  record; exact kernel pins + content hashes; AC8.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Bin | `crates/maos-bin/src/main.rs:10454-10949` (cascade, categories, `NotFound`), `:7360-7399` (offline arm), `:10097-10126` + `BinPrivateOpsImpl` (door private op, watchdog handle kept before `spawn` `:9717-9728`); `operator_door.rs:68` (`BinPrivateOps::run_uninstall_cascade` gains the door's pre-unload enumeration: halt ids, schedule ids), `:1609-1623` (uninstall), `:2039-2046` (forget off-worker), `:2083-2100` (release via kernel serializer); `hook_pump.rs:792-795` (late sweep) | AC1–AC5, AC7 |
| IAC | `crates/maos-iac/src/adapter/transparency_log.rs:1555-1719` (windows, hold re-check), new `distillates_written_by` / `scrub_uninstalled_distillate`, halt-receipt read-back | AC2, AC3, AC5 |
| Audit | `crates/maos-audit/src/lib.rs` (orphan-halt query next to `resolve_spirit_name` `:1679-1767`); `erasure/proof.rs:20-33,167-399` (evidence, schema) | AC1, AC3 |
| Registry | `crates/maos-registry/src/storage.rs:136-220` (`uninstall_package`) | AC4 |
| CLI | `crates/maos-cli/src/cli.rs:63-66` (doc) | AC4 |
| Kernel ◆ (if ruled) | `scheduler/schedule_watchdog.rs` (KS); `scheduler/scheduler_loop.rs:531-593` (KH); `memory/mod.rs:619-635` (KF); pins `xtask/kernel-core-baseline.toml:505` + HISTORY, `xtask/kloc.toml:264` | AC3–AC5 |
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

- Never a second erasure path or receipt writer; reuse the cascade, `scrub_distillate_body`, `terminate_spirit`.
- A new category never uses a bare pid: always `(boot_nonce, pid)` or the stable owner.
- No category status without a query; no `Removed` without evidence the verifier checks (D-09).
- Never scrub cost frames, `principal.uninstall.%`/`principal.forget*` records or another Spirit's rows.
- No kernel line without the ruled row; any `crates/maos-kernel-core/src` change outside KS/KH/KF is STOP.
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

No new dependency: `rusqlite` (rowid windows, `prepare_cached`), `dashmap` (`retain`, KS), tokio
`block_in_place` (multi-thread root), `ed25519-dalek`/`sha2` (existing proof). The Python verifier keeps the stdlib +
its existing Ed25519 backend (`verify.py:11-12`).

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

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D-20-11-01…10 PROPOSED). Inventory measured: `intent_lineage`
  `VerifiedEmpty` is false for principal-free distillates; pending halts are drained uncounted (door) or left open
  (offline); schedules stop at unload but watchdog state stays and installed Spirits are re-admitted; a principal-free
  Spirit gets `not_found` and no proof. Kernel optional: KS +10/+6 recommended, KH +10/+6, KF +37/+33 priced by
  reverted prototype; maos-iac +90 measured.
- 2026-11-02: Operator rulings recorded (ask s2011): D-20-11-01…10 as recommended; kernel KS and ceilings per frontmatter. Status remains ready-for-dev.
