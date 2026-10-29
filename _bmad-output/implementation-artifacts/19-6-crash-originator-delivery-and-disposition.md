---
story_id: '19.6'
story_key: 19-6-crash-originator-delivery-and-disposition
created: 2026-10-28
context_commit: d9bd59c4
baseline_commit: d9bd59c4
security_sensitive: false
kernel_grant: "none under the recommended rows — kernel-Δ 0 by construction: D-19-6-01 A / 05 A put delivery, resolver and loop handling in `maos-bin` (no `crates/maos-kernel-core/src` file in the change set). Kernel-touching options were priced by throwaway prototype on a `/tmp` copy of the git-tracked kernel src (never the tree; `creation/measurements.md`): D-19-6-01 B (kernel delivers) +27 physical / +24 tokei (26378 → 26405; 20619 → 20643); D-19-6-05 B ±0 / ±0 but a content-pin byte-break of `supervision/disposition.rs` (`xtask/kernel-core-baseline.toml:972`); D-19-6-05 C −1 / −1 plus maos-domain +11 (zero-headroom row `xtask/kloc.toml:425`). Each is FLAG-Winston if ruled. Pin 26378 physical (`kernel-core-baseline.toml:505`) / 20619 tokei (`kloc.toml:264`) replicated read-only at d9bd59c4; gate confirmation owed (measurements owed 1–5)."
kloc_grant: "PROPOSED D-19-6-10 (operator rules). Forecast [INFERENCE, component table §Kernel and kloc pricing] maos-bin +152–244 tokei (× 1.3 ≈ 198–317) against the epic's 100 (D19-1, `epics/epic-19-founder-loop-w4.md:131,156`); kernel, maos-iac (8270 exact, `kloc.toml:406`), maos-domain (9975 exact, `:425`), maos-control, maos-cli, maos-shell, maos-bench, xtask 0; `spirits/`, `tests/` uncharged (`xtask/src/kloc_check.rs:303,311`). E19 maos-bin envelope 31166 (`epics/epic-19-founder-loop-w4.md:40`), measured 30380 (`kloc.toml:517`) ⇒ 786 left for 19-2…19-6; recommended A absorbs 19-6 inside it with an exact re-pin at landing and a STOP if the cumulative would exceed 31166."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`) — concurrency: the delivery runs on the spawned crash-handler task (`crates/maos-bin/src/supervision.rs:2197-2205`) while the root loop drains the same mailbox and arms/disarms the resolver; and an audit control (rule 11): the disposition rows FR50 writes must say what was done. No kernel grant under the recommended rows (a ruled D-19-6-01 B / 05 B / 05 C adds FLAG-Winston). Separate Blind (opus-5.5) and Edge (opus-5.5) layers, Acceptance on opus-5.5, Test Infrastructure on glm-5.3; creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol (never reviews its own code). Pending D-19-6-11."
---

# Story 19.6: Crash delivery to the task originator and declared disposition

Status: in-progress (RULED 2026-10-28; dev candidate c535e434 + review corrections 2026-10-29)

<!-- Created 2026-10-28 at d9bd59c4 (19-1 and 19-7 in-progress on local HEAD; origin/main b35c9284) by bmad-create-story under the lighter creation contract (no party-mode preflight), in parallel with 19-2 and 19-3 (wave 2; each depends only on 19-1, whose loop is in the tree). D-19-6-01…11 are PROPOSED, not ruled — the operator rules; Status reads ready-for-dev by the parent's instruction, pending those rulings. The tracker row `sprint-status.yaml:292`, the epic and the routing audit are NOT edited here: every correction and addition is filed in `19-6-…-evidence/creation/records-owed.md`. No cargo ran (the host's cargo slot was held): per-crate kloc by the kloc-check instrument replicated read-only (tokei 14.0.0, argv `xtask/src/kloc_check.rs:294-318`; `creation/tokei-d9bd59c4.json`), kernel options by throwaway prototypes in `/tmp` (`creation/measurements.md`, three `.diff` files); the gate runs are owed to the parent (Measurements owed 1–7). The epic has no `### 19-6` section; the ACs below are derived from the tracker row, the routing audit (`routing-audit-2026-10-22.md:21,176-177,186`) and the PRD (FR12 `prd/functional-requirements.md:41`, FR50 `:44`, NFR-Rel-1 `prd/non-functional-requirements.md:20`). -->

> **The capability:** *when a Worker dies in the middle of the founder loop, the Orchestrator that gave it the task is told — not just the audit log. Within seconds of the death the Orchestrator's mailbox holds a kernel-sent `task.orphaned` for exactly that delegation, its in-flight count returns to zero, and the root applies the disposition the Worker's manifest declared: NACK (stop the epic and say which section died and why), reassign (run the same section once more on a fresh Worker) or escalate (stop and notify the operator). The production resolver no longer answers "no replica" to everything, and the 100-kill crash corpus measures the delivery, not only the journal row.*

**Closes:**
- FR12's delivery clause — "emits `task.orphaned` IAC frames **to in-flight task originators** within 5s" (`prd/functional-requirements.md:41`) — for originators that are local Spirits (the J1 Orchestrator), closing 16-3's declared cut (`16-3-subprocess-crash-to-handle-crash.md:35-37`, residual row `:429`).
- FR50 "kernel applies the policy … (NACK / reassign-to-replica / escalate-to-operator)" (`:44`) on the J1 loop: all three declared actions applied, and the production `NullReplicaResolver` replaced (`crates/maos-bin/src/main.rs:2997-2998`).
- NFR-Rel-1's "`task.orphaned` IAC frame ≤ 5s" (`prd/non-functional-requirements.md:20`) measured on the delivered frame, in the existing 100-kill corpus.
- Routing audit rows `routing-audit-2026-10-22.md:21` (story), `:176` FR12, `:177` FR50, `:186` NFR-Rel-1; tracker `sprint-status.yaml:292`; requirements inventory `requirements-inventory.md:592`.

**Does NOT close, and says so:**
- Delivery to an originator that is not a local Spirit: a standalone run's `operator`, a hostless topology entry's `topology` (`crates/maos-bin/src/supervision.rs:266-269`), and host B's Worker whose originator `orchestrator` lives on host A (cross-host return path: nothing returns cross-host, 19-1 D-19-1-02 A, `crates/maos-bin/src/orchestrator_root.rs:119-130`). These stay journal-only (today's rows) — D-19-6-07.
- The `[cli_wrapper] recovery_policy` executor (`respawn_fresh`, zero production callers) — process recovery, not task disposition; on the `epic-16-retrospective` row (`sprint-status.yaml:254`, item (b)). A D-19-6-04 A reassignment is a new task frame to a fresh Worker, not a respawn of the dead one.
- Hung-Worker handling (`task.stalled`, NFR-Rel-2) — 16-3, OWNED (`routing-audit-2026-10-22.md:187`).
- A `demo-j1` beat or J1 cassette for a crash — 19-4 owns the beats (`epics/epic-19-founder-loop-w4.md:188-202`).
- The scheduler's `on_unload`-failure orphan path (`crates/maos-kernel-core/src/scheduler/scheduler_loop.rs:689-731`) — kernel bytes; no Worker task reaches it under 16-3's binding (records are retired before unload, `supervision.rs:2267-2273`); unchanged.

## Story

As **the founder running the J1 loop**,
I want **a Worker that dies mid-epic to be reported to the Orchestrator that dispatched it, and its manifest's crash disposition to actually happen**,
so that **the loop reacts to a dead Worker — stops, retries or escalates as declared — instead of the crash being visible only to someone who reads the Transparency Log afterwards.**

**Contract:**
- Tracker `sprint-status.yaml:292` ("journals a crash but never delivers it to the task originator's mailbox … production composition root installs `NullReplicaResolver` … J1 needs a Worker crash to reach the Orchestrator. Kernel-Δ expected 0 … After 19-1"); epic stories row `epics/epic-19-founder-loop-w4.md:156`, order `:159` ("their ACs are written at story creation (rule 9) from the tracker rows"), Dependencies `:212`; §R kloc row `:131`; D19-1 A envelope `:36-48`.
- Routing audit `routing-audit-2026-10-22.md:21` ("crash → mailbox delivery to the task originator with reassignment; all declared crash dispositions applied (production `NullReplicaResolver` replaced) | 0–small"), `:176-177,186`.
- PRD FR12 (`prd/functional-requirements.md:41`), FR50 (`:44`), NFR-Rel-1 (`prd/non-functional-requirements.md:20`).
- `docs/testing/proof-contract.md:10-31` (one landing run + one planted red per AC), `:57-71` (review tier, models), `:76-97` (test cost, budgets, fixtures), `:182-187` (local profile; T3 vectors SKIP locally).
- Confidence rules 9 (refine at HEAD), 10 (route by charter), 11 (a control is real or labelled) (`epics/epic-19-founder-loop-w4.md:54-56`).

## Current-tree corrections — read before implementing

Measured at `d9bd59c4` (19-1 + 19-7 candidate code in the tree; `creation/cites-d9bd59c4.md`).

| Premise (tracker / audit / brief) | Fact at `d9bd59c4` and consequence |
|---|---|
| "the production composition root installs `NullReplicaResolver` (`main.rs:2795-2796`)" | **Moved twice:** `:2970` at f889b3e4/f34f6b4e, **`main.rs:2997-2998`** now (shared with `CrashDetector::with_replica_resolver` `:3009` and `set_replica_resolver` `:3016`). A **second** production site passes no resolver at all: the supervisor's `disposition` executor, `supervision.rs:2252-2265` (`None` at `:2263`). Both are replaced (AC4). |
| "16-3 journals a crash but never delivers it" (`16-3…md:35-37`) | **Holds, and is wider than "never delivered":** step 5 writes `task.orphaned` with `tl.insert_frame_event` (`crates/maos-kernel-core/src/supervision/crash_detector.rs:152-171`) — a TL row, no mailbox; the three FR50 helpers write one TL row each at pid 0 (`crates/maos-iac/src/adapter.rs:242-304`), and `reassign_task_to`'s doc says "enqueue a reassign frame" while its body enqueues nothing (`:282-304`). |
| (unstated) what "reassign-to-replica" does today | **It cannot reassign.** `enforce_disposition` resolves by the task's `IntentClass` string (`"standard"` for every Worker record, `disposition.rs:37-38`; record `supervision.rs:1578`), **discards** the resolved pid (`_replica_pid`, `:39`) and writes the ORIGINATOR id as the replica (`:41`). With `NullReplicaResolver` every `reassign-to-replica` falls through to `task.escalated` (`:43-49`). → D-19-6-04, D-19-6-05. |
| "J1 needs a Worker crash delivered to the Orchestrator; 19-1's loop at `main.rs ~4860-4930` treats a non-Completed Worker as a run error" | The refusal is **`main.rs:5072-5078`** (`"maos run: topology cli_wrapper worker did not complete (<label>)"`); `:4853-4879` is the pending-halt check and `:4880-4919` the frame build + `begin_delegation`. On a crash the Orchestrator's in-flight count is never released (`begin_delegation` `:4919`; `complete_delegation` only on success `:5115`; `spirits/orchestrator/src/lib.rs:452-476`). 19-1's TI-3 pins today's text (`crates/maos-bin/tests/epic_dispatch_19_1.rs:455-458`) — clean cutover in AC1. |
| (unstated) who the originator is | `WorkerTask::record_ids` (`supervision.rs:264-279`): delegation ⇒ `orchestrator` (`delegation::FROM_SPIRIT`, `delegation.rs:62`); hostless topology entry ⇒ `topology`; standalone ⇒ `operator`. Only `orchestrator` is a Spirit with a mailbox: `DelegationLeg::install_with_router` registers `orchestrator` and `developer-remote` (`delegation.rs:390-395`) and `journal_completion` already delivers a `TaskComplete` to it and drains it (`:571-589`). On host B the same leg registers a LOCAL `orchestrator` handle although the real originator is on host A — delivering there would be a false delivery → D-19-6-07 guard. |
| (unstated) a delivery path exists for kernel-originated frames | **Yes, kernel-0:** `IacBusAdapter::deliver_typed` (`crates/maos-iac/src/adapter.rs:329-630`) journals (I2) then delivers; a frame from `KERNEL_SENDER_SPIRIT_ID = "__kernel"` (`crates/maos-iac/src/adapter/mailbox.rs:61`) is exempt from the consent gate (`:338-346`); the 18-5 context signal is the kernel-sent precedent (`crates/maos-iac/src/adapter/context_signal.rs:75-87`). The recipient row lands in `transparency_log_recipients` (`crates/maos-iac/src/adapter/transparency_log.rs:552`; resolver `:685`), which is the observable "delivered to the originator". A repeated `frame_id` is the typed `Duplicate` and is not re-delivered (`adapter.rs:608-628`). |
| (unstated) when the Worker's verdict reaches the loop | After `handle_crash`: the binding joins the spawned handler within `CRASH_HANDLER_JOIN_BUDGET` 5 s (`supervision.rs:232`, executor `:2207-2244`) before `run_cli_wrapper_manifest` returns `NotCompleted(ProcessCrash)` (`crates/maos-bin/src/worker_cli.rs:46-48`, label `:90-92`). A delivery made inside the handler task is therefore in the mailbox before the loop looks. |
| (unstated) which manifests declare a disposition | None shipped (`grep on_crash spirits/` = 0): every J1 Worker is `Nack` (`crates/maos-domain/src/supervision.rs:22-30`). `[on_crash]` is parsed at pid 0 (`crates/maos-bin/src/worker_spawn.rs:620-639`) into the bundle (`:923-927`) and captured at bind (`supervision.rs:1581-1585`; vector `crates/maos-bin/tests/worker_supervision_16_3.rs:613-657`). |
| NFR-Rel-1 "measured" by 16-3 | Measured on **journal rows**: the 100-kill corpus asserts `lifecycle.crash` ≤ 2 s and `task.orphaned` + `task.nacked` ≤ 5 s for ≥ 99/100 (`crates/maos-bin/tests/worker_crash_corpus_16_3.rs:89-94,515`), every Worker bound `WorkerTask::Standalone` (`:614,715`) — originator `operator`, no mailbox. Job `nfr-rel-1-crash-detection-2s` (`.github/workflows/discipline.yml:1567-1609`, 20-min ceiling `:1590`). → D-19-6-08. |
| "Epic section `### 19-6-…`" | **Absent.** The epic has no `### 19-6` (nor `### 19-5`) heading; only `:156`, `:159`, `:212` name the story. A §19-6 section is owed after the ruling (`records-owed.md` E1). |

**Epic / tracker / audit cites corrected (OLD → NEW @`d9bd59c4`; filed, not applied — `creation/records-owed.md`):**
- `sprint-status.yaml:292`: `main.rs:2795-2796` → `main.rs:2997-2998` (R1).
- `epics/epic-19-founder-loop-w4.md:91` (R19-24): `main.rs:2970`, was `:2795-2796` → `main.rs:2997-2998` (R2); `:131`: `main.rs:2970` → `:2997-2998` (R3).
- `routing-audit-2026-10-22.md:21`, `:177`: `main.rs:2795-2796` → `:2997-2998` (R4, R5).
- Held: `16-3…md:35-37`, `:297`; `requirements-inventory.md:592`; PRD `:41,:44`, NFR `:20`.

## Kernel and kloc pricing — measured at creation

**Kernel:** Δ 0 under the recommended rows (frontmatter). Instrument replicated read-only at HEAD: 26378 physical (git-tracked kernel `.rs`, `wc -l`) = pin `xtask/kernel-core-baseline.toml:505`; tokei 20619 = `xtask/kloc.toml:264`. Kernel-touching options, priced on a `/tmp` copy with `rustfmt --edition 2021` then the same counters (`creation/measurements.md`):

| Option | Kernel physical | Kernel tokei | Also | Evidence |
|---|---:|---:|---|---|
| D-19-6-01 B — `handle_crash` step 5 delivers via `self.iac.deliver_typed` | +27 (26405) | +24 (20643) | content pin `crash_detector.rs` (`:971`) | `creation/k1-kernel-delivery-prototype.diff` |
| D-19-6-05 B — `task.reassigned` names `pid:<resolved>` | ±0 | ±0 | content pin `disposition.rs` (`:972`) moves — FLAG-Winston byte-break | `creation/k2-replica-pid-prototype.diff` |
| D-19-6-05 C — trait reshape `find_replica(&TaskAssignmentRecord) -> Option<ReplicaTarget>` | −1 (26377) | −1 (20618) | maos-domain 9975 → 9986 (+11) + in-src test edit [INFERENCE] +3–6, zero-headroom `kloc.toml:425` | `creation/k3-trait-reshape-prototype.diff` |

All three `git apply --check` cleanly at HEAD; none was applied to the tree. ADR-069: 20643 (K1) + 19-3's +9 = 20652, under the 22,000 alarm (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`). Gate confirmation: Measurements owed 1–5.

**kloc at `d9bd59c4`** (`creation/tokei-d9bd59c4.json`): maos-bin 30380 (exact, `kloc.toml:517`), maos-iac 8270 (exact, `:406`), maos-domain 9975 (exact, `:425`), kernel 20619 (`:264`), xtask 47451, aggregate **189877** (hardfail 189878, `:718`).

Component forecast for the recommended rows (tokei code, [INFERENCE]; analogues: `completion_frame` `delegation.rs:1101-1133`, `journal_completion` `:571-589`, `dispatch_stop` `orchestrator_root.rs:201-225`):

| Crate | Component | Forecast |
|---|---|---:|
| maos-bin `supervision.rs` | set-once originator-delivery seam on `WorkerSupervisor` + root install | 12–18 |
| | deliver after `handle_crash` `Ok` in the spawned handler (record carried into the task) and on the crash-phase `NotLoaded` disposition path; local-Spirit guard (D-07 A) | 18–30 |
| | executor `disposition` passes the root resolver (`:2263`, was `None`) | 2–4 |
| `delegation.rs` | `orphan_frame` (kernel sender, `TaskComplete{result:"orphaned"}`, id derived from the delegation frame id — D-02 A) | 22–30 |
| | `DelegationLeg::take_orphaned` (typed drain, fails closed on any other kind) | 12–18 |
| `orchestrator_root.rs` | `RootReplicaResolver` (`impl ReplicaResolver`; arm/disarm, budget 1) — D-05 A | 20–30 |
| | `delegation_orphaned` / stop events + escalation notification (D-06 A) | 18–28 |
| `main.rs` | resolver install `:2997-2998` shared with the supervisor | 3–6 |
| | crash branch replacing `:5072-5078`: drain, release, event, NACK/escalate stop, typed error (D-03 A) | 20–35 |
| | reassign: one bounded re-dispatch of section k to a fresh Worker of the same entry; arm/disarm around the run (D-04 A) | 25–45 |
| | **Σ maos-bin** | **152–244** |
| `spirits/orchestrator` | in-flight release on an orphan (uncharged, `kloc_check.rs:311`) | (5–10) |
| kernel · maos-iac · maos-domain · maos-control · maos-cli · maos-shell · maos-bench · xtask | — (tests uncharged; D-09 A needs no xtask line) | 0 |
| **Σ charged** | | **152–244 → × 1.3 ≈ 198–317** |

Not-recommended deltas: D-01 B −30–48 maos-bin, + kernel +27/+24; D-01 C −10–20 (no seam, loop-only); D-04 C −45–75 (no resolver arming, no re-dispatch); D-05 C + maos-domain ≥ +14; D-06 B ±5; D-06 C + maos-control 40–60, maos-cli 40–60; D-07 B +10–15; D-07 C cross-host return (unpriced, new protocol leg); D-09 B xtask +1 (`check_j1_loopback_delegation.rs:129`) against 4 lines of xtask slack (47624 − 47451 − 169).

**Cumulative use against the E19 envelope** (`epics/epic-19-founder-loop-w4.md:40-48`; ruled 2026-10-28 "measured + remaining forecasts × 1.3"):

| Crate | Measured @d9bd59c4 | E19 endpoint | Left | Others' share (19-2 140 · 19-3 125 · 19-4 100 · 19-5 150, × 1.3) | 19-6 (× 1.0 / × 1.3) | Fits? |
|---|---:|---:|---:|---:|---:|---|
| maos-bin | 30380 | 31166 | 786 | 670 (515 at × 1.0) | 152–244 / 198–317 | **only if the others land under their × 1.3 share**: with others at × 1.3, 116 left — 19-6's lower bound breaches by 36; with others at × 1.0, 271 left — 19-6's upper bound fits. → **D-19-6-10** |
| aggregate hardfail | 189878 | 191061 | 1183 | — | 152–244 | yes (maos-bin is the only charged crate) |
| kernel | 20619 / 26378 | 20628 / 26390 (19-3's grant) | 9 / 12 | 19-3 | 0 (A) · +24/+27 (01 B) | 01 B breaches the endpoint → needs its own FLAG-Winston grant |
| maos-domain · maos-iac | 9975 · 8270 (exact) | no E19 envelope | 0 | — | 0 (A) · +11–17 (05 C) | 05 C needs a grant |
| Cumulative consumed at creation (E19, after the 2026-10-28 raise) | | | | | | maos-bin +14 (19-7 correction, `kloc.toml:517`); 19-2…19-6: 0 |

## Decisions — RULED 2026-10-28 (D-19-6-01…11; ask ids in the Status column)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-19-6-01** | Where the crash reaches the originator's mailbox (FR12) | **A — maos-bin, in the supervisor's crash path:** after `CrashDetector::handle_crash` returns `Ok` inside the spawned handler (`supervision.rs:2197-2205`), and on the crash-phase `NotLoaded` disposition path (`:2252-2265`), a set-once root-installed seam delivers one frame per drained record through `IacBusAdapter::deliver_typed` (`adapter.rs:329-630`); the kernel's own rows are unchanged · **B — kernel:** `handle_crash` step 5 delivers via `self.iac.deliver_typed` (prototype K1: **+27 physical / +24 tokei**, FLAG-Winston; covers in-proc crashes too) · **C — root loop:** the loop delivers after `run_cli_wrapper_manifest` returns `ProcessCrash` (`main.rs:5072`) | **A.** Kernel-Δ 0 as the tracker expects (`sprint-status.yaml:292`); the handler task runs right after `task.orphaned` is written, so the 5 s floor is the kernel's own timing and is independent of pipe EOF (16-3 §2, `16-3…md:58`) — C is bounded by the pump's end and covers only the topology arm. B buys in-proc coverage nobody needs (the only production ledger writer is the Worker binding, `supervision.rs:2072-2091`) for +24 tokei against an E19 kernel endpoint with 9 tokei left. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-02** | The delivered frame | **A — kernel-sent `TaskComplete`:** `from` `__kernel` (`mailbox.rs:61`, consent-gate exempt `:338-346`), `to` the originator (local address), `auto_marker: Kernel`, payload `TaskComplete{result:"orphaned"}`, `frame_id` derived from the delegation frame id (as `DelegationLeg::outcome_frame_id`, `delegation.rs:791`, with its own mark) so it joins the `task.assign` row and a second delivery is the typed `Duplicate`; cause and disposition stay in the kernel's `task.orphaned` row (`crash_detector.rs:153-162`), joined by `task_id` · **B — mirror `completion_frame`:** `from` `developer-remote`, `SpiritAuto`, delegation lineage (`delegation.rs:1101-1133`), result `"orphaned"` · **C — a new `FramePayload::TaskOrphaned{task_id,cause,disposition}`** (maos-domain wire/ABI change, zero-headroom row) | **A.** The kernel — not the dead Worker — is the author, and saying so is the honest audit (B attributes a frame to a Spirit that is dead); the 18-5 context signal is the exact precedent (`context_signal.rs:75-87`). C changes a wire type for facts the kernel row already carries. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-03** | What the loop does on receipt (default `nack`) | **A — release, stop, fail typed:** the root drains the orphan (`take_orphaned`), releases the Orchestrator's in-flight delegation (`spirits/orchestrator/src/lib.rs:466-476`), prints `delegation_orphaned {section, task_id, cause, disposition, orchestrator_safe_point}`, stops dispatch (`epic_halted`, `main.rs:4683,4759`) and exits non-zero with a typed error naming the section, the cause and the disposition (replacing `:5072-5078` for `ProcessCrash`; other non-completions keep "did not complete") · **B — as A but exit 0** once the disposition is applied · **C — delivery only:** keep today's immediate `Err`; nothing consumes the frame | **A.** A crashed section is not a completed epic — B is the false-success surface j1-crosshost-2a AC1.5 closed (`main.rs:5529-5533`). C leaves "J1 needs a Worker crash to reach the Orchestrator" (`sprint-status.yaml:292`) unmet: the frame would sit in a handle nobody reads. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-04** | What `reassign-to-replica` means in the J1 loop | **A — same entry, fresh Worker, once:** section k is re-dispatched ONCE as a new `task.assign` (new seq/frame id, same goal, same `prior_distillate_ref` so the FR21 gate passes, `crates/maos-iac/src/adapter/orchestrator_dispatch.rs:63-146`) to a fresh Worker bound from the same entry manifest; on completion the loop proceeds to k+1; a second crash falls through to escalate (resolver budget spent ⇒ `None` ⇒ `task.escalated`, `disposition.rs:43-49`) · **B — next entry takes section k:** breaks D-19-1-03 A (section k ↔ entry k) and needs spare entries · **C — no reassignment in J1:** the resolver answers `None` honestly; FR50's reassign stays PARTIAL and the audit row is re-routed | **A.** Workers are spawned per task (`run_cli_wrapper_manifest`, `main.rs:5028-5048`); there is no standing replica pool, so "a replica" is a fresh instance of the declared Worker — bounded to one retry so a deterministic crasher cannot loop. B silently re-binds sections; C leaves the audit row's FR50 clause unclosed. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-05** | The production resolver, and what the `task.reassigned` row says | **A — maos-bin `RootReplicaResolver`** at `main.rs:2997-2998` (shared with the detector `:3009`, the scheduler `:3016` and the supervisor executor `supervision.rs:2263`): unarmed ⇒ `None`; armed by the J1 loop around ONE delegated Worker run whose manifest declares `reassign-to-replica` ⇒ `Some(<Orchestrator pid>)` once (the root, through the originator, performs the re-dispatch); the kernel row's `replica_spirit_id` then names the originator (existing `disposition.rs:41`), labelled as "re-dispatched via the originator" in the resolver doc and pinned by a test (rule 11) · **B — A + kernel K2:** the row names `pid:<resolved>` (±0 lines, content-pin byte-break, FLAG-Winston) · **C — reshape the trait** (K3): `find_replica(&TaskAssignmentRecord) -> Option<ReplicaTarget>`; the row names the target (kernel −1/−1, maos-domain +11–17, zero-headroom grant) | **A.** Kernel-0 and truthful under D-04 A: the originator IS the party that re-dispatches. The resolver key is `IntentClass` (`disposition.rs:37`), meaningless for selection, so A arms by root state (J1 runs one Worker at a time, `main.rs:5028`) — stated as a guardrail. B changes a pinned byte to swap a name for a number; C is the right long-term shape but spends two zero-headroom rows on a v0.8 story. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-06** | The `escalate-to-operator` surface | **A — notification + stop:** the kernel's `task.escalated` row (unchanged) plus one `NotificationEvent::AnomalyFlagged{observer:"kernel.crash-detector", subject:<worker>, summary:<section, task id, cause>, confidence:1.0}` (`crates/maos-domain/src/notification.rs:50-65`) through the root's dispatcher (`main.rs:2119-2125`; already held by the supervisor, `supervision.rs:2411-2425`); dispatch stops as D-03 A with the disposition named · **B — a pending halt on the Orchestrator** via `invoke_halt` (as `plant_test_halt`, `orchestrator_root.rs:247-258`), so 19-1's `dispatch_stop` stops and the operator resolves with `maosctl halt resolve` · **C — A + a door read route and `maosctl` listing** of escalations | **A.** FR50 is "operational-failure handling distinct from epistemic halt (FR15)" (`prd/functional-requirements.md:44`) — B writes an `EpistemicHalt` with a scalar tag for a non-epistemic event and would enter the halt registry 18-2 scores. C spends maos-control/maos-cli lines 19-2 needs (`epics/epic-19-founder-loop-w4.md:43,45`). | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-07** | Which originators receive the frame | **A — local Spirits only:** deliver iff `scheduler.resolve_pid(originator)` is `Some` on this root (the loopback J1 Orchestrator); `operator`, `topology` and host B's remote `orchestrator` stay journal-only, and the record says so · **B — A + the standalone root prints a `task_orphaned` event line** for `operator` · **C — A + host B returns the orphan to host A** over the A2A leg | **A.** Delivery to a non-Spirit is meaningless, and on host B the locally registered `orchestrator` handle (`delegation.rs:390-395`) is not the originator — delivering there is a false delivery. B is operator UX the standalone root already gives (non-zero exit + kernel rows, `main.rs:5529-5533`). C is a new cross-host protocol leg with no return path today (D-19-1-02 A, `orchestrator_root.rs:119-130`) — rule 10 destination: `epic-19-retrospective` / the Epic 21 multi-host lane. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-08** | Where NFR-Rel-1's 5 s floor is measured on the delivery | **A — extend the 16-3 corpus:** `worker_crash_corpus_16_3.rs` binds its 100 Workers as `WorkerTask::Delegation` (distinct frame ids) with an Orchestrator SCB and a registered `orchestrator` mailbox in its `KernelWorld`; the poller also drains the mailbox and asserts delivered ≤ 5 s after the kill for ≥ 99/100, beside the two existing floors; same job, same kills · **B — a separate 100-kill delivery corpus + job** · **C — one timing vector only** (no floor) | **A.** Zero new kills or jobs (job healthy 2m01s against a 20-min ceiling, `discipline.yml:1573,1590`); the corpus already owns the kill timing and the grandchild half that makes the floor real (`worker_crash_corpus_16_3.rs:11-55`). C would certify a floor on one sample. Owed: measurement 7. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-09** | Where the loop vectors live and how CI runs them | **A — extend `epic_dispatch_19_1.rs`** (already J1-gate-enrolled by the `_19_1.rs` suffix, `xtask/src/check_j1_loopback_delegation.rs:129`, and in `workspace-test-suite`): TI-3 cut over + one vector per disposition; seam/resolver vectors in `worker_supervision_16_3.rs` · **B — new `crash_delivery_19_6.rs` + `_19_6.rs` in `J1_TEST_SUFFIXES`** (xtask +1, job step) · **C — new file, `workspace-test-suite` only** (not J1-gate-enrolled) | **A.** Zero xtask/YAML change and the J1 gate keeps running every loop vector; TI-3 must change there anyway (clean cutover). Each T3 run ≈ 1.2 s (19-1 creation `ci-timings-40609213235.tsv`); the binary's Σ against its 30 s budget is owed (measurement 6). B spends 1 of 4 xtask slack lines; C drops the loop's crash path out of the J1 gate. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| **D-19-6-10** | kloc for 19-6 (forecast 152–244 vs the epic's 100) | **A — absorb inside the E19 maos-bin envelope 31166** (no raise now): exact re-pin at landing with the cumulative-use line; STOP and return a D-row if the cumulative would exceed 31166 · **B — raise now by 19-6's excess:** maos-bin 31166 → **31353** (+187 = (244 − 100) × 1.3), aggregate 191061 → **191248** · **C — trim to the epic's 130:** D-04 C (no reassignment) | **A.** The 2026-10-28 ruling sized the envelope as "measured + remaining forecasts × 1.3" (`epics/epic-19-founder-loop-w4.md:40`); 271 lines remain if 19-2…19-5 land at forecast, 116 at × 1.3 — whether 19-6 breaches depends on siblings not yet built, so the measured landing decides (the E18 R1 one-envelope rule). B prices an estimate (the CEILING RULE: "a grant is measured, never estimated", `xtask/kloc.toml:342`); C drops an FR50 clause to save ~60 lines. | RULED 2026-10-28: **A** (ask `w2_19_6`) — superseded by the E19 kloc ruling (ask `w2_kloc`, "Raise maos-bin now", 2026-10-28): maos-bin envelope 31166 → **31353** (+187, = 19-6's upper forecast × 1.3 coverage), aggregate hardfail 191061 → **191248** |
| **D-19-6-11** | Review tier | **A — DIFFICULT** (separate opus Blind + Edge, opus Acceptance, glm Test-Infra) · **B — standard** (Blind + Edge may merge on opus; Acceptance glm) | **A.** Concurrency (the handler task delivers while the root drains and the resolver is armed) and an audit control whose rows must not lie (rule 11) — `proof-contract.md:67-71`. Adds FLAG-Winston if D-01 B / D-05 B / C is ruled. | RULED 2026-10-28: **A** (ask `w2_19_6`) |
| CR-1 | Fail-closed (all options) | No local originator ⇒ no delivery (journal-only, as today); an unarmed resolver ⇒ `None` ⇒ the kernel's escalate fall-through; a second crash of a reassigned section ⇒ escalate; a duplicate orphan frame ⇒ typed `Duplicate`, never a second release; any non-`TaskComplete` frame in the Orchestrator's handle during the drain ⇒ the run fails loudly (precedent `delegation.rs:535-555`). | — |
| CR-2 | Log before deliver (I2) | The orphan frame is journaled by `deliver_typed` before it is delivered; the kernel's `task.orphaned` row precedes it (`crash_detector.rs:163-170` runs before the handler returns). | — |

## Acceptance Criteria

*(Derived at creation from the tracker row, the routing audit and the PRD — the epic carries no §19-6 ACs (`:159`). Worded to the recommended rows; re-worded at the ruling. Every T3 vector SKIPs locally without the substrate and runs in CI, `proof-contract.md:182-187`.)*

### AC1 — command and observable: a Worker crash in the epic loop reaches the Orchestrator (default `nack`)

1. Command (the 19-1 TI-3 shape, `epic_dispatch_19_1.rs:409-449`, with the second host-bearing entry a T3 `worker-cli-fixture --maos-fixture-mode=sigkill-self` manifest as in `worker_supervision_16_3.rs:512`): `maos run <topology> --epic tests/fixtures/j1/epic.md --once`.
2. Observable, in order: section 1 completes and is distilled (unchanged); section 2's Worker dies; the kernel journals `task.orphaned` at the Worker's pid and `task.nacked` at pid 0 (unchanged, `crash_detector.rs:141-179`); **one** `TaskComplete` row from `__kernel` with a `transparency_log_recipients` row naming the Orchestrator's pid is journaled, its timestamp ≤ 5 s after the Worker's death; stdout carries `{"event":"delegation_orphaned","section":2,"task_id":"delegation:…","cause":"fault.signaled","disposition":"nack","orchestrator_safe_point":true}`; no third `task.assign` row; the run exits non-zero and stderr names section 2, the cause and `nack`.
3. TI-3 (`epic_dispatch_19_1.rs:405-490`, exit-nonzero Worker) is cut over to the same contract (cause `fault.non-zero-exit`); its `"did not complete"` assertion (`:455-458`) is replaced, not kept beside.
4. **Proven red:** with the delivery call removed, AC1's vector fails on the missing recipient row and the missing event. Planted once in place, restored, recorded in `19-6-crash-originator-delivery-and-disposition-evidence/plants.md` (`proof-contract.md:24-31`).

### AC2 — delivery: one kernel-sent frame per orphaned delegation, local originators only

1. The supervisor delivers on the crash paths only — after `handle_crash` returns `Ok` in the spawned handler (`supervision.rs:2197-2205`) and on a `NotLoaded` outcome while the binding is `CrashHandled` (`:322-323`, executor `:2252-2265`); a planned stop (`PlannedStop`) or an abandoned binding delivers nothing.
2. Frame per D-19-6-02 A, through `IacBusAdapter::deliver_typed(frame, 0, None)`; the frame id is derived from the delegation frame id, so a second attempt is the typed `Duplicate` (`adapter.rs:608-628`) and nothing is released twice.
3. Delivered iff the originator resolves to a Spirit loaded on this root (D-19-6-07 A); `operator`, `topology` and host B's remote originator stay journal-only — proven by the existing standalone vectors' unchanged rows (`worker_supervision_16_3.rs:493-657`) plus one host-B assertion that no `orchestrator` recipient row is written there.
4. The kernel's rows and payloads are byte-for-byte unchanged (`crash_detector.rs:141-210`; `adapter.rs:242-304`); kernel-Δ 0.

### AC3 — the declared disposition is applied in the loop (FR50)

1. `nack` — AC1.
2. `reassign-to-replica` (`[on_crash] action = "reassign-to-replica"` on the entry manifest): the kernel writes `task.reassigned` (not `task.escalated`); the root re-dispatches section k once (D-19-6-04 A) — a new `task.assign` row with the same `prior_distillate_ref`, a new Worker pid — and, when that Worker completes, distils it and continues to k+1; `epic_next` does not advance on the orphan, so 19-2's FR20 safe-point drain (gated on `epic_next` advancing) does not open between the orphan and the re-dispatch. A second crash of the same section ⇒ `task.escalated` and AC3.3's stop.
3. `escalate-to-operator`: the kernel writes `task.escalated` (unchanged); one `AnomalyFlagged` notification reaches the root's dispatcher (D-19-6-06 A; asserted on the `TerminalChannel` stderr line or a test channel); dispatch stops as AC1 with `escalate-to-operator` named.
4. One vector per disposition in the loop (D-19-6-09 A); the existing 16-3 escalate vector (`worker_supervision_16_3.rs:613-657`) keeps passing.

### AC4 — `NullReplicaResolver` is replaced in production (D-19-6-05 A)

1. `main.rs:2997-2998` installs `RootReplicaResolver`, the same `Arc` reaching the detector (`:3009`), the scheduler (`:3016`) and the supervisor's executor (`supervision.rs:2263`, was `None`). `NullReplicaResolver` keeps no production caller (`grep -n NullReplicaResolver crates/*/src` outside `crates/maos-domain/src` = 0).
2. Unarmed ⇒ `None` on every root and every task it was not armed for; armed only by the J1 loop around one delegated Worker run whose manifest declares `reassign-to-replica`, disarmed after the run returns; budget one.
3. Rule 11 label: the resolver's doc and one test pin that the kernel's `task.reassigned` row names the originator as the re-dispatching party (`disposition.rs:41`), and that the fresh Worker's identity is in the new `task.assign` and bind rows.

### AC5 — NFR-Rel-1 / FR12's 5 s floor is measured on the delivery (D-19-6-08 A)

`hundred_sigkilled_workers_meet_both_floors_half_holding_a_grandchild` (`worker_crash_corpus_16_3.rs:515`) binds its 100 Workers as `WorkerTask::Delegation` with an Orchestrator SCB and a registered `orchestrator` mailbox, and additionally asserts: delivered frames = 100 (denominator first, as `:54-55`), and delivered ≤ 5 s after the kill for ≥ 99/100. The existing floors, the 50/50 grandchild split and job `nfr-rel-1-crash-detection-2s` (`discipline.yml:1567-1609`) are unchanged. Proven red: with the delivery seam uninstalled in the corpus world, the delivery floor fails at 0/100.

### AC6 — the record tells the truth (rule 11)

Comments that describe the old behaviour are updated in the same change where they live outside the kernel: `main.rs:3014-3015` ("production's NullReplicaResolver is the explicit no-replica implementation"), the supervisor executor's `None`; the stale kernel comments (`crash_detector.rs:45,74`, `disposition.rs:13-15,40,44-45`) are NOT edited under kernel-Δ 0 and are listed in the Dev Agent Record as known-stale kernel bytes (owner: the next FLAG-Winston story touching those files, rule 10(b)). The routing-audit and inventory rows flip only at landing (`records-owed.md`).

### Amendment — acceptance-review corrections S1–S4 (2026-10-28; `agent://Accept19x6`, code at `c535e434`)

Recorded as amendments to the ACs above; the AC text is not rewritten and no task is marked done — the vectors are re-worded at implementation.

- **S1 (AC1.1 / AC1.2 / AC1.3):** the T3 payload runs as PID 1 in its namespace (`t3/argv.rs:21` — "PID 1 is `/maos/spirit` … never `--init`"), so `sigkill-self`'s self-SIGKILL is ignored there; the `worker_supervision_16_3.rs:512` reference is a T0 fixture, where `sigkill-self` does work. AC1.1's command is therefore `--maos-fixture-mode=exit-nonzero`, AC1.2's cause is `fault.non-zero-exit` (not `fault.signaled`), and AC1.3 is folded into AC1 — TI-3 is now the AC1 vector itself (its `"did not complete"` assertion, `epic_dispatch_19_1.rs:455-458`, is replaced, not kept beside).
- **S2 (AC3.2 test shape):** the AC3.2 vector crashes its section through the debug-only `MAOS_TEST_ONLY_CRASH_SECTION_ONCE` root kill plant (registered in `crates/maos-bin/src/env_contract.rs`, implemented `orchestrator_root.rs:269-294`) and drives the fresh re-dispatched Worker with the new fixture mode `delay-then-complete`; neither appears in the AC text above.
- **S3 (AC3.2 / D-19-6-04 A):** D-04 A's premise "same `prior_distillate_ref` so the FR21 gate passes" does not hold for the first section (k = 1): the crash itself writes the `TaskComplete`-kind rows the FR21 gate keys on (`orchestrator_dispatch.rs:93-115`, via `check_orchestrator_distillate_required`), and section 1 has no prior distillate, so the re-dispatch is refused `EOrchestratorDispatchRawOutput`. Corrected in maos-bin by arming the resolver only when a prior distillate exists (`&& epic_prior.is_some()` on the `replica_arm` condition, `main.rs:5054-5057`): section 1 escalates honestly.
  - **RULED 2026-10-29 (ask `r19_6_d04`):** accept the narrowing — a crashed section with no prior distillate (section 1, or a loopback delegation without `--epic`) that declared `reassign-to-replica` escalates instead of re-dispatching; the stop line names the reason; the real fix (an FR21 exemption keyed on the reassigned task_id, a maos-iac change) is a deferred-work row owned by `21-5-orchestrator-port-and-move`.
- **S4 (AC6 stale-record list, amended):** the Dev Agent Record lists as known-stale, recorded and not edited (kernel-Δ 0; owner per rule 10(b)): `crates/maos-kernel-core/src/supervision/crash_detector.rs:45,74` and `crates/maos-kernel-core/src/supervision/disposition.rs:13-15,40,44-45` (kernel bytes), plus two records beyond AC6's original list — `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md:631` and `crates/maos-domain/src/supervision.rs:182`, both still reading "`NullReplicaResolver` is the v0.3-β default" (maos-domain is a zero-headroom row).

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-19-6-01…11 RULED; tracker `:292` → `in-progress`; records-owed R1–R5/E1–E3 applied by the parent. Re-measure every cite that moved (19-2/19-3 land in the same `main.rs` block, `:4853-5128`). Record measurements owed 1–7.
- [ ] **T1 — Delivery seam** (AC2): set-once originator-delivery on `WorkerSupervisor`; deliver on the crash paths only; local-Spirit guard; `orphan_frame` + derived frame id; `DelegationLeg::take_orphaned`.
- [ ] **T2 — Resolver** (AC4): `RootReplicaResolver` in `orchestrator_root.rs`; install at `main.rs:2997-2998`; executor `supervision.rs:2263`; arm/disarm API.
- [ ] **T3 — Loop** (AC1, AC3): crash branch replacing `:5072-5078` for `ProcessCrash`; in-flight release (`spirits/orchestrator`); `delegation_orphaned` event; NACK/escalate stop with typed error; reassign once; escalation notification.
- [ ] **T4 — Vectors + plants** (AC1–AC4): TI-3 cut over; three disposition vectors in `epic_dispatch_19_1.rs`; resolver/seam vectors in `worker_supervision_16_3.rs`; AC1 plant (remove the delivery) → red; restore; `plants.md` with wall-clock.
- [ ] **T5 — Corpus** (AC5): Delegation tasks + Orchestrator SCB + mailbox drain in the corpus world; delivery floor assert; plant (uninstall the seam) → red; restore.
- [ ] **T6 — Truth** (AC6): non-kernel comments; Dev Agent Record lists the known-stale kernel comments.
- [ ] **T7 — Close-out:** `cargo fmt --all`; `kloc-check --json` exact re-pins (maos-bin, aggregate) with the cumulative-use line inside the ruled D-19-6-10; `check-kernel-baseline` (26378 unchanged under A); `check-j1-loopback-delegation`, `check-test-cost`, `check-test-budget` on the hosted junit; `prepush` exit 0 recorded (`proof-contract.md:144-148`); records-owed landing rows; Dev Agent Record, File List, Change Log; tracker. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Delivery | `crates/maos-bin/src/supervision.rs` (`:2197-2205`, `:2252-2265`, seam beside the ctor `:2411-2446`), `crates/maos-bin/src/delegation.rs` (beside `journal_completion` `:571-589` and `completion_frame` `:1101-1133`) | journal-only → kernel-sent frame to a local originator |
| Resolver | `crates/maos-bin/src/orchestrator_root.rs` (new type), `crates/maos-bin/src/main.rs:2996-3017` | `NullReplicaResolver` + `None` → `RootReplicaResolver` everywhere |
| Loop | `crates/maos-bin/src/main.rs:4683,4759,4880-4919,5049-5128`; `spirits/orchestrator/src/lib.rs:452-476` | crash ⇒ `Err` with the delegation still in flight → drain, release, dispose, typed stop or one re-dispatch |
| Tests | `crates/maos-bin/tests/epic_dispatch_19_1.rs` (TI-3 `:405-490` + 3 vectors), `crates/maos-bin/tests/worker_supervision_16_3.rs`, `crates/maos-bin/tests/worker_crash_corpus_16_3.rs` | cut over + new vectors + delivery floor |
| Kloc | `xtask/kloc.toml:517,718` | exact re-pins (D-19-6-10) |

**File overlap with wave-2 siblings:** 19-2 edits `main.rs:4853-4917` (safe-point drain), `:5016-5048` (debug hold seam), `orchestrator_root.rs`, `spirits/orchestrator/src/lib.rs` (Create19x2, 2026-10-28); 19-6 edits `main.rs:2996-3017`, `:4683/4759`, `:4918-4919`, `:5049-5128`, the same two files. No shared lines; same block — serialize the merges. 19-3 touches the Worker spawn path (`run_cli_wrapper_manifest`, `worker_spawn.rs`) the crash path runs through — re-read `:5028-5048` at T0.

### Current state that must be preserved

- The kernel's crash protocol and rows (`crash_detector.rs:87-238`): 2 s / 5 s floors, `task.orphaned` before `lifecycle.crash`, disposition rows at pid 0 — the corpus, the FR4 doorbell (16-5) and the 16-3 vectors read them.
- 16-3's binding state machine (`supervision.rs:313-475`): one disposition per record (`dispositioned`, `:1152-1162`); planned stops never crash.
- 19-1's loop contract: section k ↔ entry k (D-19-1-03 A), FR21 distillate refs, the pending-halt stop (`dispatch_stop`), `assert_completions_at_safe_point`.
- `journal_completion`'s drained count for a completed Worker (`delegation.rs:584-588`); `demo-j1` and `journey_j1.rs` (no crash) unchanged.

### Guardrails

- No `crates/maos-kernel-core/src` edit, no new `FrameKind`/`FramePayload`, no maos-iac/maos-domain change under the recommended rows (both are zero-headroom rows).
- Deliver from the crash paths only; never from `PlannedStop`/abandon; never to a non-local originator.
- The resolver key (`IntentClass`) selects nothing: arm by root state around exactly one run; disarm on every exit path (a `Drop` guard, the `WorkerBinding` pattern `supervision.rs:1269-1273`).
- A reassign re-dispatch is not a completion: `epic_next` unchanged, no distillate for the crashed attempt, 19-2's drain stays closed.
- New tests outside `src` (uncharged); one cargo process, `CARGO_TARGET_DIR=/mnt/build/cargo`, `env -u CI -u GITHUB_ACTIONS`; plants restored byte-identical.

### Proven-red controls (rule 11; proof contract `:24-31`)

| Control | Plant | Expected red |
|---|---|---|
| AC1 delivery | remove the seam's `deliver_typed` call | no `__kernel` → orchestrator recipient row; no `delegation_orphaned`; AC1 vector fails |
| AC3.2 reassign | resolver returns `None` when armed | `task.escalated` instead of `task.reassigned`; no second `task.assign` for section k |
| AC5 floor | corpus world without the seam | delivered 0/100 < 99 |

### Tests and commands

```bash
# local; one cargo process; T3 vectors SKIP on this host (no rootless podman, epic R19-P2)
P="env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo"
$P cargo run -q --locked -p xtask -- build-test-fixtures --only worker-cli-fixture
$P cargo nextest run --locked -p maos-bin --test epic_dispatch_19_1 --test worker_supervision_16_3 --test two_host_delegation_2b
$P cargo nextest run --locked -p orchestrator
$P cargo test --locked -p maos-bin --release --test worker_crash_corpus_16_3 -- --ignored   # the NFR job's command, discipline.yml:1609
$P cargo run -q --locked -p xtask -- check-j1-loopback-delegation
$P cargo run -q --locked -p xtask -- kloc-check --json && $P cargo run -q --locked -p xtask -- check-kernel-baseline && $P cargo run -q --locked -p xtask -- check-test-cost
$P cargo run -q --locked -p xtask -- prepush
```

### Review plan (proof contract `:33-71`)

- DIFFICULT tier (D-19-6-11 A): Blind and Edge separate on opus; Acceptance (opus) re-reads every AC against the ruled rows; Test-Infra (glm) checks the corpus denominator, the plants and that no vector is vacuous on the SKIP path.
- Edge replays: crash during the 5 s join budget; concurrent door unload of the Worker (`NotLoaded`); SIGTERM during a reassigned run; a second crash of the replica; host B serving a crashing Worker (no delivery); a duplicate delivery.

### Previous-story intelligence

- **16-3:** the Worker SCB, the exit observer and both floors; the declared cut this story closes (`16-3…md:35-37,429`); D-16-3-F (exit → `CrashCause`), D-16-3-E (record ids), D-16-3-K (corpus).
- **5.3:** `CrashDetector` and `enforce_disposition` with a `NullReplicaResolver` placeholder ("multi-instance hosting lands at Story 6.1 + 8.4", `disposition.rs:13-15` — it never did).
- **19-1:** the loop, `epic_halted`/`dispatch_stop`, TI-3, the `_19_1.rs` gate suffix; per-T3-run cost ≈ 1.2 s.
- **18-5:** the kernel-sent mailbox frame precedent (`context_signal.rs`).
- **j1-crosshost-2a:** the false-success rule — a non-completed Worker never exits 0 (`main.rs:5529-5533`).

### Git intelligence

| Commit | Content |
|---|---|
| `d9bd59c4` / `e49b83c4` / `6f872383` | 19-1 + 19-7 candidate, corrections, halt-row fix — moved the resolver row `:2970` → `:2997-2998` |
| `f34f6b4e` / `b35c9284` | 20-15/20-16 landing (origin/main) |
| `ed2118a0` | 20-15 conmon FLAG-Winston (kernel 26367 → 26378) — last kernel src change |

### References

- Tracker `sprint-status.yaml:254,292`; epic `epic-19-founder-loop-w4.md:34,36-48,54-56,91,131,156,159,212`; routing audit `routing-audit-2026-10-22.md:21,176-177,186-187`; `requirements-inventory.md:592`; PRD `functional-requirements.md:41,44`, `non-functional-requirements.md:20`; 16-3 `16-3-subprocess-crash-to-handle-crash.md:35-37,58,297,429`.
- Kernel: `crates/maos-kernel-core/src/supervision/crash_detector.rs:45,74-78,85-238`; `disposition.rs:13-68`; `scheduler/scheduler_loop.rs:93,163-168,689-731`; pins `xtask/kernel-core-baseline.toml:505,971-972`.
- Domain/IAC: `crates/maos-domain/src/supervision.rs:19-30,181-196,326-329`; `notification.rs:29-66`; `crates/maos-iac/src/adapter.rs:239-304,329-630`; `adapter/mailbox.rs:61,324-393`; `adapter/context_signal.rs:75-87`; `adapter/transparency_log.rs:552,685,691`; `crates/maos-director-surface/src/notification.rs:49-66`.
- maos-bin: `main.rs:2119-2125,2940,2996-3017,4546-4556,4683,4759,4853-4919,5028-5128,5529-5533`; `supervision.rs:232,264-279,313-475,1152-1162,1269-1273,1557-1590,2072-2091,2197-2273,2411-2446`; `delegation.rs:58-70,298-322,390-395,535-555,571-589,791,1101-1133`; `orchestrator_root.rs:119-130,201-259`; `worker_cli.rs:46-48,87-106`; `worker_spawn.rs:620-639,923-927`; `spirits/orchestrator/src/lib.rs:452-476`.
- Tests/CI: `crates/maos-bin/tests/epic_dispatch_19_1.rs:22-23,82-101,405-490`; `worker_supervision_16_3.rs:493-657`; `worker_crash_corpus_16_3.rs:11-55,89-94,401-403,515,614,715`; `.github/workflows/discipline.yml:1567-1609`; `xtask/src/check_j1_loopback_delegation.rs:129,843-846`; `xtask/kloc.toml:264,406,425,517,718`; `xtask/src/kloc_check.rs:294-318,386-417`; `docs/testing/proof-contract.md:10-31,57-97,182-187`.

## Dev Agent Record

### Story context creation

Created 2026-10-28 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `d9bd59c4`, clean tree, in parallel with the 19-2 and 19-3 creators, while another process held the host's single cargo slot — so no cargo, build, test or xtask gate was run.

**Inputs:** direct reads of every cited range; `git grep` across `f889b3e4` / `f34f6b4e` / `d9bd59c4` for the moved resolver row (`creation/cites-d9bd59c4.md`); the kloc-check instrument replicated read-only (tokei 14.0.0, argv `kloc_check.rs:294-318`; per-crate figures equal the pins, aggregate 189877 — `creation/tokei-d9bd59c4.json`); kernel physical by `wc -l` over git-tracked kernel `.rs` = 26378 (pin); three kernel-touching options priced on `/tmp` copies only (`creation/measurements.md`, `k1-…`, `k2-…`, `k3-….diff`, each `git apply --check` clean at HEAD).

**Measurements owed (the parent runs them; one cargo process; exact commands and expected figures in `creation/measurements.md`):**
1. `check-kernel-baseline` at HEAD — expect PASSED 26378 / 95 files.
2. `kloc-check --json` at HEAD — expect passed, aggregate 189877, maos-bin 30380.
3. K1 applied → `cargo check -p maos-kernel-core --lib`, gates → expect 26405 / 20643 (+27 / +24); revert; gates re-PASS.
4. K2 applied → expect a `disposition.rs` content-hash failure at 26378 lines, tokei unchanged; revert.
5. K3 applied (+ domain in-src test edit) → expect kernel 26377 / 20618, maos-domain ≥ 9986 (> 9975, exit 1); revert.
6. `maos-bin::epic_dispatch_19_1` per-binary Σ from the latest hosted junit vs 30 s (D-19-6-09).
7. `nfr-rel-1-crash-detection-2s` latest wall-clock vs its 20-min ceiling (D-19-6-08).

**Records owed** (`creation/records-owed.md`): R1–R5 cite corrections (tracker `:292`, epic `:91`, `:131`, audit `:21`, `:177`); E1 a new epic §19-6 section (the epic has none); E2–E3 stories row / kernel paragraph after the ruling; T1–T2 tracker flip; landing rows.

**Revert:** nothing to revert — no crate source was touched; prototypes lived only under `/tmp/proto19x6/`.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- `19-6-crash-originator-delivery-and-disposition-evidence/creation/cites-d9bd59c4.md`, `tokei-d9bd59c4.json`, `measurements.md`, `records-owed.md`, `k1-kernel-delivery-prototype.diff`, `k2-replica-pid-prototype.diff`, `k3-trait-reshape-prototype.diff`.

### Completion Notes List

### File List

### Change Log

- 2026-10-28: Story created at `d9bd59c4` (D-19-6-01…11 PROPOSED; Status ready-for-dev pending the rulings; tracker `:292`, epic and routing audit untouched — corrections R1–R5 and the missing epic §19-6 filed in `creation/records-owed.md`). Kernel-Δ 0 under the recommended rows; kernel options priced by `/tmp` prototype (+27/+24; ±0 byte-break; −1/−1 + maos-domain +11); maos-bin forecast 152–244 (× 1.3 ≈ 198–317) against the epic's 100, D-19-6-10.
- 2026-10-28: Operator rulings recorded (ask `w2_19_6`): D-19-6-01…11 RULED **A**; D-19-6-10's "absorb inside 31166" outcome superseded by the E19 kloc ruling (ask `w2_kloc`, "Raise maos-bin now"): maos-bin envelope 31166 → **31353** (+187, = 19-6's upper forecast × 1.3 coverage), aggregate hardfail 191061 → **191248**; Status → ready-for-dev (RULED 2026-10-28).
- 2026-10-28: Acceptance-review corrections S1–S4 (`agent://Accept19x6`, code at `c535e434`) recorded as an amendment block under the ACs (`:172-179`): AC1.1's command is `--maos-fixture-mode=exit-nonzero` (T3 payload is PID 1, `sigkill-self` ignored) and AC1.2's cause `fault.non-zero-exit`, AC1.3 folded into AC1; AC3.2's debug-only `MAOS_TEST_ONLY_CRASH_SECTION_ONCE` plant and `delay-then-complete` fixture mode recorded; D-19-6-04 A's k = 1 premise corrected (resolver armed only when a prior distillate exists — section 1 escalates); AC6's stale-record list extended with `4-kernel-design.md:631` and `maos-domain/src/supervision.rs:182`. Tasks not marked done.
- 2026-10-29: RULED (ask `r19_6_d04`): the S3 narrowing accepted — a crashed section with no prior distillate (section 1, or a loopback delegation without `--epic`) that declared `reassign-to-replica` escalates honestly; the stop line names the reason; the FR21-exemption fix (keyed on the reassigned task_id, maos-iac) filed as a deferred-work row owned by `21-5-orchestrator-port-and-move` (`deferred-work.md` § "Deferred from: 19-6 review"). Tasks not marked done.
