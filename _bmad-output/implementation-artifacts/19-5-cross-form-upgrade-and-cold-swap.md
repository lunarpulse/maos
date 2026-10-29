---
story_id: '19.5'
story_key: 19-5-cross-form-upgrade-and-cold-swap
created: 2026-10-28
context_commit: 61af7461
baseline_commit: 61af7461
security_sensitive: true
kernel_grant: "◆ FLAG-Winston (scheduler) — RULED 2026-10-28 (Lunarpulse, ask `w3_19_5`: D-19-5-01 A, D-19-5-07 B, D-19-5-09 A; E16-A7: priced by reverted prototype at creation). The ruled rows are net NEGATIVE — a LOWER re-pin at landing: K1 (delete the kernel cold-swap arm, `lifecycle/upgrade.rs:162-210`) −57/−53 + K6 (`check_abi_triple` extracted from `admit_spirit`, `security/mod.rs:283-332`) +8/+4 + K8 (upgrade-target `[scheduling]`/`[lifecycle]` defaults, `upgrade.rs:401-402`) +8/+6 = **−41 physical / −43 tokei** (HEAD 26378 → 26337, 20619 → 20576; tree with 19-3's uncommitted K2 26389 → 26348, 20628 → 20585) — a LOWER re-pin of `xtask/kernel-core-baseline.toml:505` (+HISTORY, content pins `:920`, `:954`), `xtask/kloc.toml:264`, surface pin `docs/ci-baselines/kernel-surface-v0.1-beta.json` (`UpgradePolicy`, `UpgradeError`; new `check_abi_triple` row in `xtask/kernel-api-classes.toml`). Kernel-positive alternatives, each needing its own grant because the E19 kernel endpoint (≤ 26483 / ≤ 20686 = tree + 19-2's capped +93/+58) leaves 19-5 zero lines: K2 +21/+19 (D-01 C), K3 +53/+48 (D-05 B), K4 +73/+66 (D-05 C), K5 +16/+13 (D-03 B), K7 +10/+6 (D-06 C). All measured on /tmp copies (`creation/measurements.md`); cargo confirmation owed (Measurements owed 1–4)."
kloc_grant: "D-19-5-12 SUPERSEDED 2026-10-28 by the E19 kloc ruling (Lunarpulse, ask `w3_kloc`, 'Raise to cover forecasts ×1.3'): maos-bin envelope 31353 → **31710**, xtask 47624 → **47688**, aggregate hardfail 191248 → **191669**; 19-5 re-pins its exact measured figure inside it at landing (STOP + D-row if the cumulative would exceed 31710). Forecast under the ruled rows [component table; two legs measured by maos-bin prototype]: maos-bin **+186…+201** (admission.rs +103 measured, operator_door.rs +75 measured, main.rs −16 measured + 10–18, door policy/ABI/Worker-refusal +14–21) against the epic's 150 (`epics/epic-19-founder-loop-w4.md:130`); maos-control +4; maos-cli −7; kernel −43 (measured); xtask, maos-bench, maos-shell, maos-domain 0; tests uncharged (`xtask/src/kloc_check.rs:303`). E19 maos-bin envelope 31710 (raised from 31353 by ask `w3_kloc`), measured 30380 at HEAD (`kloc.toml:517`) and 30847 in the working tree (19-3/19-6 uncommitted) ⇒ 863 left for 19-2 (+196–279, its record), 19-4 (100, × 1.3 130) and 19-5 (× 1.3 242–261) — fits at every upper bound (454 ≥ 261)."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston re-pin (deletion + one extracted pub fn + parser defaults), a NEW admission refusal that can block a `maos run` / `maosctl load` (FR11 — fail-closed semantics), and concurrency (cold swap's unload → admit window under the door's per-Spirit lock and `upgrade_lock`, `operator_door.rs:961-963`). Separate Blind (opus-5.5) and Edge (opus-5.5), Acceptance on opus-5.5, Test Infrastructure on glm-5.3. Creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol. D-19-5-13 RULED A (DIFFICULT), ask `w3_19_5`."
---

# Story 19.5: Cross-form upgrade and cold swap ◆ FLAG-Winston (scheduler)

Status: ready-for-dev (RULED 2026-10-28)

<!-- Created 2026-10-28 at 61af7461 (19-1, 19-7 done; 19-3 and 19-6 code uncommitted in the working tree, being verified; 19-2 ruled ready-for-dev) by bmad-create-story under the lighter creation contract, in parallel with 19-4 (wave 3). D-19-5-01…14 are PROPOSED, not ruled — the operator rules; Status reads ready-for-dev by the parent's instruction, pending those rulings. The tracker row `sprint-status.yaml:291`, the epic, the routing audit and deferred-work are NOT edited here: every correction is filed in `19-5-…-evidence/creation/records-owed.md`. NO cargo ran (a build held the host): kloc by the kloc-check instrument replicated read-only (tokei 14.0.0, argv `xtask/src/kloc_check.rs:294-318`), kernel physical by `wc -l` over git-tracked kernel `.rs`; eight kernel options and two maos-bin components priced by throwaway prototypes on `/tmp` copies of `git archive 61af7461` (`creation/measurements.md`, `*.diff`, each `git apply --check`-clean against the tree and never applied). Cites into files carrying uncommitted 19-3/19-6 edits (`main.rs`, `supervision.rs`, `worker_spawn.rs`, `discipline.yml`) are HEAD lines with the tree line in parentheses — re-measure at T0. The epic has no `### 19-5` section; the ACs below are derived from the tracker row, the routing audit (`routing-audit-2026-10-22.md:20,173-175`), the PRD (FR10 `prd/functional-requirements.md:39`, FR11 `:40`, FR49 `:43`), ADR-020 and the 16-1 record lines the tracker cites. -->

> **The capability:** *an operator can move a running Spirit to a new version by any of the three policies FR49 names, and each one is honest. Hot swap keeps the Spirit's pid, so its capability tokens and working memory survive — and that is now asserted, not assumed. Cold swap unloads the old version and admits the new one through the same admission chokepoint `maos run` and `maosctl load` use, at a new pid, with a re-init; if the successor is refused, the predecessor comes back. Migrator-mediated upgrade is a door policy, not an exit-2. And once a version of a Spirit has run on this host, loading a cross-major successor that declares no migrator is refused with `EMigratorMissing` — on `maos run`, on `maosctl load`, on every path — instead of quietly starting on state it cannot read.*

**Closes (under the ruled rows, 2026-10-28):**
- FR49 (`prd/functional-requirements.md:43`): all three declared policies over the door — hot-swap (exists), **cold-swap with re-init** (today `409 cold_swap_unsupported`, `crates/maos-bin/src/operator_door.rs:841-852`), **migrator** (today refused locally with exit 2, `crates/maos-cli/src/subcommands.rs:2379-2387`).
- FR11 (`:40`): "kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared" — an archive record now exists (zero production writers today) and the daemon's admission chokepoint refuses (D-19-5-05 labels "kernel").
- FR10 (`:39`), in-process clause: in-flight capability tokens and working memory survive a hot swap (asserted), and the ADR-020 decision tree reads the predecessor's real `[hot_swap]` schema (today dropped at admission).
- `deferred-work.md:97` (successor bypasses admission/ABI checks; stale owner "Story 5.5x") — cold half by D-19-5-01 A, hot half by D-19-5-09 A.
- The `epic-16-retrospective` cold-swap items (tracker `sprint-status.yaml:254`: unadmitted successor under a new pid; raw spirits-map rollback `upgrade.rs:203`, `:204-207`, `:208`).
- Routing audit `routing-audit-2026-10-22.md:20` (story), `:173-175`; tracker `:291`; requirements inventory `requirements-inventory.md:591`.

**Does NOT close, and says so:**
- FR10's spawned-form clause (an in-flight version swap of a non-in-process Spirit) — **D-19-5-10 RULED C** (ask `w3_19_5_d10`, operator-approved scope cut): routed to `20-9-spawned-form-lifecycle-hooks`; the `[cli_wrapper]` Worker is not a Spirit form (ADR-060 `:27-31`) and its upgrade target is refused typed.
- A state archive: the FR11 record is an identity record (manifest text, empty snapshot); `SpiritArchive::read` keeps zero production readers (D-19-5-03).
- ~~`[on_crash]`, `[on_revocation]`, `[supervision]` are also dropped from the admitted SCB bundle (`admission.rs:404-412`) — FR50/FR13 charters, filed as cross-story note X1 (`records-owed.md`), not fixed here.~~ **r19_x1 (2026-10-28): the drop is ADDED to 19-5** by extending D-19-5-06 A — `bundle()` carries the three sections too (AC4.3, kloc note ~+6–10 maos-bin); the FR50/FR13 charters stay filed in `records-owed.md` (X1 flips at landing).
- FR53 halt identity across swaps (`20-3b`, routing audit Appendix C); a drained halt's missing receipt (`deferred-work.md:894`, owner `20-3c`).
- The archive root ignoring `MAOS_HOME` (`deferred-work.md:862`, owner `21-4`).

## Story

As **the operator running MAOS Spirits on my host**,
I want **to upgrade a Spirit by hot swap, cold swap or a declared migrator, and to have the daemon refuse a cross-major successor that cannot read its predecessor's state**,
so that **version changes are a governed lifecycle step with a typed outcome — never an unadmitted process, a silent state loss, or a flag that only exits 2.**

**Contract:**
- Tracker `sprint-status.yaml:291` ("FR10/FR11/FR49 were routed to 16-1, whose record claims a compiled Butler swap only (16-1…md:478-486 → `:303-309`) — subprocess-form state/token parity is unclaimed, admission has no archived-predecessor check (05-fr1-20-scout.md:13), and cold swap is `ColdSwapUnsupported` (16-1…md:43-46). ◆ FLAG-Winston (scheduler) — kernel figure by prototype at creation (E16-A7). After 19-3").
- Epic stories row `epics/epic-19-founder-loop-w4.md:155`; kernel-Δ paragraph `:34`; §R kloc row `:130`; order `:159` ("19-3 → 19-5"; "their ACs are written at story creation (rule 9) from the tracker rows"); Dependencies `:225`; envelope `:38-48`.
- Routing audit `routing-audit-2026-10-22.md:20` ("state/token-preserving upgrade parity for the subprocess form; admission rejects an archived predecessor without a migrator; cold-swap policy ships (`ColdSwapUnsupported` today) | ◆ FLAG-Winston (scheduler)"), `:173-175`.
- PRD FR10 `prd/functional-requirements.md:39`, FR11 `:40`, FR49 `:43`; ADR-020 `architecture-maos-minimal-opus/12-architecture-decision-records.md:335-345`; kernel design `4-kernel-design.md:573-580`; ADR-060 `docs/adr/ADR-060-spirit-forms-by-trust-tier.md:16-31,54-57,69-87,179-183`; ADR-069 `:35-46`.
- 16-1 record `16-1-daemon-post-surface-and-verb-retarget.md:43-45` (cold swap not closed), `:180` (D-16-1-W), `:303-309` (root E), `:432` (§11 row 11).
- `docs/testing/proof-contract.md:24-31` (one plant per AC), `:57-72` (review tier, models), `:182-187` (local profile).
- Confidence rules 9, 10, 11 (`epics/epic-19-founder-loop-w4.md:54-56`).

## Current-tree corrections — read before implementing

Measured at `61af7461` (`creation/cites-61af7461.md`).

| Premise (tracker / audit / brief) | Fact at `61af7461` and consequence |
|---|---|
| "16-1's record claims a compiled Butler swap only (`16-1…md:478-486`)" | **Moved to `:303-309`** by the 2026-10-27 archive split (`d614eb9b`); content holds: root E hot-swaps butler 0.3.0 → 0.3.1 at the same pid and asserts a new `belief_variance` row; nothing asserts that tokens or working memory survive (`crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs:98-…` neither) → AC4.2, **D-19-5-11**. |
| "admission has no archived-predecessor check (`05-fr1-20-scout.md:13`)" | **Holds and is wider: the archive is never written.** `SpiritArchive::write` (`crates/maos-kernel-core/src/hot_swap/archive.rs:56-85`) has zero production callers; the coordinator stores `archive_dir` (`coordinator.rs:62`, created `:80-99`) and never reads it. FR11's condition "predecessor archive exists" is unreachable on every path → **D-19-5-03…05**. |
| "cold swap is `ColdSwapUnsupported` (`16-1…md:43-46`)" | **Holds.** Door refusal `operator_door.rs:841-852` (`cold_swap_unsupported`); kernel arm `crates/maos-kernel-core/src/lifecycle/upgrade.rs:162-210` builds the successor SCB with `allocate_pid` + a raw `map.insert` (`:187-199`, boot nonce hard-coded `0` `:186`), never calls `admit_spirit`, and its rollback is a raw `map.remove` + `AtomicU8::store` + re-insert (`:203-208`). Its ONLY production caller is the `MAOS_ONE_SHOT=smoke-upgrade-revoke-5` arm (`crates/maos-bin/src/main.rs:7546-7562` HEAD, tree `:7631-7647`), whose script `tests/integration/smoke_upgrade_revoke_5.sh` no CI job runs → **D-19-5-01**. |
| (unstated) what admission keeps of the manifest | **The SCB bundle drops the swap sections.** `GatedManifest::bundle()` (`crates/maos-bin/src/admission.rs:404-412`) keeps scheduling, lifecycle, class, budget; `[hot_swap]`, `[migrates_from]`, `[halt_protocol_compatibility]` are never parsed (`grep hot_swap admission.rs` = 0). The hot-swap coordinator then reads the predecessor's state schema and halt-protocol version as the default `1` (`coordinator.rs:151-156,235-245`), so a predecessor admitted by `maos run` with `[hot_swap] state_schema_version = 0x0001_0001` is treated as major 0 and a same-major successor is mis-classified cross-major → **D-19-5-06**. (The same function also drops `[on_crash]`/`[on_revocation]`/`[supervision]` — X1, not this story.) |
| (unstated) hot-swap target shape | **Upgrade targets must declare `[scheduling]` and `[lifecycle]`** (`upgrade.rs:401-402`) while admission defaults both (`admission.rs:565-574`); shipped manifests have neither, so root E appends them (`one_daemon_one_door_16_1.rs:995-998`). Under D-19-5-01 A one file would be a valid cold-swap target and an invalid hot-swap target → **D-19-5-07**. |
| (unstated) the migrator policy | **Kernel arm exists, door does not carry it.** `UpgradePolicy::Migrator` (`upgrade.rs:212-232`; `MigratorNotDeclared` `:213-215`) calls `initiate_swap`; `UpgradePolicyRequest` has only HotSwap/ColdSwap (`crates/maos-control/src/lib.rs:278-287`); maosctl refuses `--policy migrator` locally with exit 2 (`subcommands.rs:2379-2387`, `unreachable!` `:2437`). A cross-major hot swap already runs `run_migrator` automatically (`coordinator.rs:273-299`) → **D-19-5-08**. |
| `deferred-work.md:97` "ColdSwap/HotSwap upgrade paths bypass `admit_spirit` … Story 5.5x tracks the fix" | **Owner stale** (no 5.5x). Hot swap re-checks nothing about the successor's ABI triple (`admit_spirit` legs `security/mod.rs:283-332`; the coordinator reads only halt-protocol and state-schema fields, `coordinator.rs:117-450`) [INFERENCE: a target declaring `min_substrate_version = "99.0.0"` completes a hot swap today] → **D-19-5-09**. |
| tracker "After 19-3 (the subprocess Worker is E19's form)"; audit "parity for the subprocess form" | **The Worker is not a Spirit form.** ADR-060 `:27-31`: `[cli_wrapper]` is "a third manifest shape, not a form string"; forms are `rust-inproc`, `subprocess` (compatibility token; the native subprocess host was never built, `:54-57,78-79`) and `wasm-component`. The successor factory refuses any non-`rust-inproc` successor `spawned_surface_unsupported` (`main.rs:3331-3340` HEAD, tree `:3334-3343`; test `wasm_form_admission_17_3b.rs:338`); a Worker's SCB Spirit implements only `on_unload` (`supervision.rs:2324-2345` HEAD, tree `:2440-2461`); a `[cli_wrapper]` upgrade target fails untyped as `target_manifest_unparseable` (`operator_door.rs:869-877`, `:1926-1944`) → premise correction P1, **D-19-5-10**. |
| 16-1 "maos-bin's `pid_by_spirit_id` (`main.rs:2428`) is not refreshed" | Map now `main.rs:2692-2693`; the door's `unload` never removes the entry (`operator_door.rs:583-592`; removal only on admission rollback, `admission.rs:865-876`); `load_admit_start` overwrites it on every granted admission (`:787-793`) — under D-19-5-01 A the cold-swap successor refreshes it by construction. |
| "Epic section `### 19-5-…`" | **Absent** (`grep -n '^### '`: 19-1 `:161` … 19-4 `:195`, 19-6 `:211`, 19-7 `:217`). A §19-5 section is owed after the ruling (`records-owed.md` E1). |

**Epic / tracker / audit cites corrected (OLD → NEW @`61af7461`; filed, not applied — `creation/records-owed.md`):** `16-1…md:478-486` → `:303-309` (tracker `:291` R1, audit `:20` R2, `:173` R3); `16-1…md:484-486` → `:307-309` (audit `:175` R4); `deferred-work.md:97` owner "Story 5.5x" → 19-5, cite `upgrade.rs:131-171` → `:139-233` (R5). Held: `16-1…md:43-46`; `05-fr1-20-scout.md:13`; `requirements-inventory.md:591`; PRD `:39,:40,:43`.

## Kernel and kloc pricing — measured at creation

**Kernel (◆ FLAG-Winston):** instrument replicated read-only — HEAD 26378 physical (= pin `kernel-core-baseline.toml:505`) / 20619 tokei (= `kloc.toml:264`); working tree 26389 / 20628 (19-3's uncommitted K2: `runtime.rs`, `t3/argv.rs`, `t3/spawn.rs`). None of the files below carries an uncommitted edit, so each Δ is the same at HEAD and in the tree.

| # | Option | Kernel Δ physical / tokei | HEAD endpoint | Tree endpoint | Diff |
|---|---|---:|---|---|---|
| K1 | delete the kernel cold-swap arm, variant and `UpgradeError::Lifecycle` (D-01 A) | **−57 / −53** | 26321 / 20566 | 26332 / 20575 | `k1-delete-kernel-coldswap.diff` |
| K2 | kernel cold swap via an injected `SuccessorAdmitter` + `load_obj` (D-01 C) | +21 / +19 | 26399 / 20638 | 26410 / 20647 | `k2-kernel-coldswap-admitter.diff` |
| K3 | FR11 gate `SpiritArchive::refuse_without_migrator` (D-05 B) | +53 / +48 | 26431 / 20667 | 26442 / 20676 | `k3-kernel-archive-gate.diff` |
| K4 | K3 + `scheduler.load_obj` refuses (D-05 C) | +73 / +66 | 26451 / 20685 | 26462 / 20694 | `k4-scheduler-load-gate.diff` |
| K5 | coordinator writes the predecessor archive + state blob at commit (D-03 B) | +16 / +13 | 26394 / 20632 | 26405 / 20641 | `k5-coordinator-archive-writer.diff` |
| K6 | `pub fn check_abi_triple` extracted from `admit_spirit` (D-09 A) | +8 / +4 | 26386 / 20623 | 26397 / 20632 | `k6-abi-triple-extract.diff` |
| K7 | `pub fn load_bundle_from_str` (D-06 C) | +10 / +6 | 26388 / 20625 | 26399 / 20634 | `k7-shared-bundle-parser.diff` |
| K8 | upgrade targets default `[scheduling]`/`[lifecycle]` (D-07 B) | +8 / +6 | 26386 / 20625 | 26397 / 20634 | `k8-upgrade-target-section-defaults.diff` |
| **R** | **recommended: K1 + K6 + K8** | **−41 / −43** | **26337 / 20576** | **26348 / 20585** | `krec-recommended-bundle-k1-k6-k8.diff` |

E19 kernel endpoint ≤ 26483 / ≤ 20686 = tree + 19-2's capped +93 / +58 (D-19-2-20 B) ⇒ 19-5 has **0 lines** of positive room; R frees 43 tokei (20585 is 1,415 under the ADR-069 alarm 22,000). Gate confirmation: Measurements owed 1–4.

**kloc** (`creation/tokei-61af7461-and-tree.json`): HEAD maos-bin 30380 (`kloc.toml:517`), aggregate 189877 (hardfail 189878, `:718`); tree maos-bin 30847, xtask 47454, aggregate 190356.

Component forecast for the recommended rows (tokei code; "measured" = prototype on a HEAD copy, `creation/mb-*.diff`):

| Crate · component | Forecast | Basis |
|---|---:|---|
| maos-bin `admission.rs` · `[hot_swap]`/`[migrates_from]`/`[halt_protocol_compatibility]` parsed and carried by `bundle()` (D-06 A) | +27 | measured |
| maos-bin `admission.rs` · FR11 gate before Gate 8 + `AdmissionRefusal::MigratorMissing` + write-if-absent admission record (D-03 A, D-04 A, D-05 A) | +76 | measured (D-05 B instead: +45) |
| maos-bin `operator_door.rs` · cold swap composed from the admission triple, restore on refusal, `spirit.upgrade` TL row, journal `Upgrade`; refusal deleted; `BinPrivateOps::admitted_manifest` (D-01 A, D-02 A) | +75 | measured (D-02 B: ≈ −25) |
| maos-bin `main.rs` · `admitted_manifest` impl + path recorded at the three admission sites | +10–18 | [INFERENCE] |
| maos-bin `main.rs` · smoke arm's cold-swap step deleted (`:7546-7562` HEAD) | −16 | counted |
| maos-bin door · migrator mapping (D-08 A) · ABI pre-gate call (D-09 A) · typed `[cli_wrapper]` refusal (D-10 C) | +3 · +5–8 · +6–10 | [INFERENCE] |
| **Σ maos-bin** | **+186…+201** (× 1.3 ≈ 242–261) | epic 150 (`epic-19:130`) |
| maos-control · `UpgradePolicyRequest::Migrator` + parse arm + doc (D-08 A) | +4 | [INFERENCE] |
| maos-cli · local migrator refusal deleted, arm mapped | −7 | [INFERENCE] |
| kernel | **−43** | measured (R) |
| xtask · maos-bench · maos-shell · maos-domain · maos-wasm-host | 0 | — |
| **aggregate** | ≈ +140…+155 | Σ |

**kloc note — r19_x1 (2026-10-28):** D-19-5-06 A extended (AC4.3: `bundle()` also carries `[on_crash]`/`[on_revocation]`/`[supervision]`) adds **~+6–10 maos-bin** [INFERENCE, unmeasured, on top of the measured +27 admission row] ⇒ maos-bin ≈ **+192…+211** (× 1.3 ≈ 250–274), aggregate ≈ +146…+165 — still inside the ruled envelopes 31710 (863 − 279 − 130 = 454 ≥ 274) and 191669; exact figure re-pinned at landing.

Not-recommended deltas: D-01 B kernel 0 instead of −53, maos-bin +16 (the smoke arm's cold step stays); D-01 C kernel +19 and maos-bin admitter impl +20–35 [INFERENCE]; D-02 B maos-bin ≈ −35; D-03 B kernel +13 and maos-bin −12; D-05 B kernel +48, maos-bin −31 (measured); D-05 C kernel +66; D-06 C kernel +6 (+ K8); D-08 B maos-bin −3, maos-control −4, maos-cli +7; D-09 B kernel −4, maos-bin −6; D-10 A maos-bin +15–25 (+ `worker_spawn.rs`, 19-3's file); D-10 B maos-wasm-host / WIT +150–300 [INFERENCE, outside every E19 envelope].

**Cumulative use against the E19 envelope** (`epic-19:38-48`):

| Crate | HEAD | Tree | E19 endpoint | Left from tree | Others still to land (records) | 19-5 | Fits? |
|---|---:|---:|---:|---:|---|---:|---|
| maos-bin | 30380 | 30847 | **31710** (was 31353; E19 kloc ruling `w3_kloc`) | 863 | 19-2 +196–279; 19-4 100 (× 1.3 130); 19-3/19-6 still being measured in the tree | +186…+201 (× 1.3 242–261) | **yes** — 863 − 279 − 130 = 454 ≥ 261 at every upper bound (D-19-5-12 superseded) |
| maos-control | 1442 | 1442 | 1539 | 97 | 19-2 +39–48 | +4 | yes |
| maos-cli | 6317 | 6317 | 6382 | 65 | 19-2 +41–49 | −7 | yes (frees 7) |
| xtask | 47451 | 47454 | **47688** (was 47624) | 234 | 19-4 100 (130) | 0 | yes |
| kernel tokei | 20619 | 20628 | 20686 | 58 (19-2's cap) | 19-2 +46 (≤ 58) | −43 | yes (frees 43) |
| aggregate hardfail | 189878 | 190356 | **191669** (was 191248) | 1313 | — | ≈ +140…+155 | yes |

**Landing envelope ruling 2026-10-29 (Lunarpulse, ask `w5_kloc`, "Raise to measured + ~150"):** maos-bin 32086 (`w4_kloc`) → **32860** (+774), aggregate hardfail ~192045 → **~192685**; measured 19-4/19-5 review corrections round 3 maos-bin **32826**, xtask **47682** ≤ 47688, maos-cli **6371** ≤ 6382, maos-control **1496** ≤ 1539, aggregate **192650** (strict pin **192651** = measured + 1); kernel LOWER re-pin **26463 → 26409 physical / 20674 → 20618 tokei**, inside the <=26483/20686 grant.

## Decisions — RULED 2026-10-28 (Lunarpulse; asks `w3_19_5`, `w3_19_5_d10`, `w3_kloc`)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-19-5-01** | Where FR49's cold swap lives | **A — maos-bin, door-composed, kernel arm deleted:** the door unloads the predecessor (`scheduler.unload`, `operator_door.rs:587`) and admits the successor through `BinPrivateOps::load_spirit` → `admission::load_admit_start` (`admission.rs:693-849`), then `start`, at a new pid (re-init: tokens revoked by unload, working memory not carried — FR49 "cold-swap with re-init"); writes the `spirit.upgrade` TL row + journal `Upgrade` the kernel writes today (`upgrade.rs:238-273`); K1 deletes the kernel arm, `UpgradePolicy::ColdSwap`, `UpgradeError::Lifecycle` (**−57 / −53**), the smoke arm's cold step and the kernel tests' cold cases are cut over · **B — as A, kernel arm kept** (kernel 0; the arm stays a public, unadmitted-successor path with one smoke caller, labelled under rule 11) · **C — kernel arm reshaped**: `SuccessorAdmitter` injected, successor via `load_obj` (K2 **+21 / +19**, its own grant; maos-bin implements the admitter from a re-gated manifest, +20–35) | **A.** 16-6 made `load_admit_start` the "one extracted function [that] serves the door, the standalone arm and the topology arm" (tracker `:254`): Gate 8, `admit_spirit`, the vetter event, `pid_by_spirit_id`, hook registration and rollback come for free (`admission.rs:703-866`) — C rebuilds the same triple behind a kernel callback. A removes three of the six raw spirits-map writes (tracker `:254`) and frees 53 tokei when the E19 kernel endpoint has 0. B ships a kernel verb that admits nothing (`deferred-work.md:97`). | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-02** | A cold-swap successor refused AFTER the predecessor was unloaded | **A — restore:** re-admit the predecessor from the manifest path it was admitted from (recorded per Spirit at the three admission sites), new pid; outcome `reverted`, typed `cold_swap_reverted`; if the restore is refused too, `cold_swap_failed` names both refusals and the Spirit is absent · **B — fail closed, no restore:** Spirit absent, typed `cold_swap_failed`; the operator re-loads with `maosctl load` · **C — A, restoring from the retained manifest TEXT** (no path map; the butler arm needs a path for `[epistemic_policy]`, `operator_door.rs:148-185`, so the door writes a temp file) | **A.** ADR-017's saga restores the predecessor when a hot swap's `on_swap_in` fails (`coordinator.rs` step 9; `4-kernel-design.md:557-562`); a cold swap that can silently delete a running Spirit would be the only policy without a compensation. The successor is pre-checked before the unload (identity, forward-only SemVer, vetting, `operator_door.rs:869-930`; ABI under D-09 A), so restore is the rare path. Cost ≈ 25 of the 75 measured door lines + the path map. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-03** | FR11: when the archive record is written | **A — maos-bin, at every granted admission** (`load_admit_start`, after `admit_spirit` Ok) **and every completed door upgrade** (the successor's manifest): write-if-absent `<root>/<spirit_id>/<version>/manifest.toml` = the admitted text, `snapshot.cbor` empty — an identity record, labelled as such; kernel 0 · **B — kernel coordinator at hot-swap commit** with the predecessor's real state blob (K5 **+16 / +13**, own grant): a fresh boot never writes one, so stop → start of a cross-major version is never refused · **C — A + B** | **A.** FR11's gate fires at *load*; only an admission-time record makes it reachable for the restart path ADR-020 is about ("Predecessor's historical journal stays in cold storage", `12-architecture-decision-records.md:339`). The blob has no reader (`SpiritArchive::read` has no production caller). Safe to introduce: no shipped manifest declares `[hot_swap]` or `[migrates_from]` (`grep` = 0), so every shipped Spirit is schema major 0 and cannot be refused by its own record. `maos purge` already removes the root (`crates/maos-bin/src/purge.rs:52`; ADR-067). Tests set `MAOS_ARCHIVE_DIR` (the `operator_door_16_1.rs:93`, `maos_uninstall_16_4.rs:31`, `skill_door_18_7.rs:50` precedent). | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-04** | What "predecessor archive" and "cross-major" mean at load | **A —** predecessor = the most recently written record of this `spirit_id` at another version (directory mtime); cross-major = hot-swap step 7's rule, `detect_compat` on `[hot_swap].state_schema_version` (default 1) (`state_codec.rs:180-188`); declared = a `[migrates_from].versions` pattern matches the predecessor's version (`run_migrator`'s rule, `migrator.rs:48-68`, `matches_version_pattern` `:101-120`) · **B —** every archived version other than the successor's must be same-major or declared · **C —** class SemVer major differs | **A.** One rule for load and swap — no drift between the two FR11 surfaces. B refuses a 2.x declaring `["1.x"]` because a 0.x ran here once (a migrator covers the hop it was written for; chains are `migration_plan.rs:42-133`'s job). C contradicts ADR-020's schema-major test and treats every 0.y Spirit as one major. Edge: a downgrade loads with the newer record as predecessor — refused unless declared, which is the honest answer. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-05** | Where the FR11 load gate lives | **A — maos-bin `load_admit_start`, before pid allocation** (Gate 8's slot, `admission.rs:703-715`), typed `AdmissionRefusal::MigratorMissing` → `migrator_missing`, Display names `EMigratorMissing`; covers `maos run` (standalone and topology) and `maosctl load`; kernel 0 (+76 with D-03 A, measured) · **B — kernel `SpiritArchive::refuse_without_migrator`** (K3 **+53 / +48**) called from the same place (maos-bin +45, measured) · **C — kernel `scheduler.load_obj` refuses** (K4 **+73 / +66**; also catches kernel-internal `load<T>`; `scheduler_loop.rs` is 19-2's FR51 file) | **A.** The kernel endpoint has 0 lines for 19-5 (B/C need a new grant; B nets +5 tokei even with K1). The admission chokepoint already hosts the daemon's refusals ("the daemon's own gate", ADR-060 `:38-41`) and every Spirit load reaches it (16-6). FR11/ADR-020 say "kernel refuses load": under A the record states the refusal is the daemon's admission chokepoint, not `maos-kernel-core` (rule 11 label). B is the literal reading and costs 48 kernel tokei to save 31 maos-bin. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-06** | The admitted SCB bundle drops the swap sections (finding) | **A —** `gate_manifest` parses `[hot_swap]`, `[migrates_from]`, `[halt_protocol_compatibility]` into `GatedManifest`; `bundle()` carries them (+27, measured) · **B —** carry every kernel section (adds `[on_crash]`, `[on_revocation]`, `[supervision]`, schedules, gateways — changes FR50/FR13 behaviour of every admitted class Spirit) · **C —** kernel `load_bundle_from_str` as the one parser (K7 **+10 / +6**; still requires `[scheduling]`/`[lifecycle]`, so K8 too; replaces `gate_manifest`'s lenient parse for every admission) | **A.** The ADR-020 tree (FR10 "per the kernel-enforced migration decision tree") reads exactly these three sections (`coordinator.rs:151-156,235-245`; `migrator.rs:48`). B widens into 19-6's FR50 and 20-1's FR13 charters (X1, routed not decided). C re-parses every admission through a stricter parser for no fact A lacks. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-07** | Hot-swap / migrator targets must declare `[scheduling]`/`[lifecycle]` | **A —** leave; document (root E appends them) · **B —** kernel defaults when absent (K8 **+8 / +6**), as admission does (`admission.rs:565-574`) · **C —** the door rewrites the target into a temp manifest with empty sections | **B.** FR49's example is "replace v0.3.1 with v0.3.2" — the shipped manifest with its version bumped; under D-01 A that same file is a valid cold-swap target and an invalid hot-swap target. +6 tokei, netted inside K1. C is a workaround that hides the parser split. | RULED 2026-10-28: **B** (ask `w3_19_5`) |
| **D-19-5-08** | The migrator policy over the door (FR49) | **A — expose it:** maosctl drops its local refusal (`subcommands.rs:2379-2387`, `:2437`), maos-control parses `"migrator"` (`lib.rs:1466-1476`) into a new `UpgradePolicyRequest::Migrator`, the door passes `UpgradePolicy::Migrator` (`upgrade.rs:212-232`); `MigratorNotDeclared` → typed `409 migrator_not_declared`; kernel 0 · **B — keep the refusal**: FR49's migrator clause is served by the hot swap's automatic step 7 (`coordinator.rs:273-299`); document it and add a cross-major hot-swap vector | **A.** FR49 names three operator-selectable policies; the refusal's own reason ("D-16-1-W is hot-swap only, because the cold-swap arm starts an unadmitted successor", `subcommands.rs:2380-2381`) never applied to the migrator arm, which calls `initiate_swap`. B keeps a shipped `--policy` value that always exits 2. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-09** | `deferred-work.md:97` hot-swap half — the successor's ABI triple is never checked | **A —** K6 `check_abi_triple` (**+8 / +4**) called by the door on every upgrade target before any swap (hot, migrator; cold also passes admission), typed `400 substrate_too_old` / `abi_too_old` / `abi_too_new`, predecessor untouched · **B —** route the hot-swap half to `20-8-manifest-contract-completion` (admission completeness; signed manifest at admission, tracker `:305`); 19-5 closes only the cold half (D-01 A admits) · **C —** maos-bin re-implements legs 1–2 (a copy of `security/mod.rs:283-332`) | **A.** Rule 10(a): the upgrade verb is this story's blast radius and the row's stale owner gives it nowhere else to go; +4 tokei inside K1's room; C duplicates a refusal (two answers to one manifest). | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-10** | FR10's subprocess-form clause (premise P1) | **A — the Worker as FR10's subprocess form, parity at the task boundary:** each Worker spawn re-reads and re-admits its manifest (`main.rs:4702-4714` HEAD, tree `:4721-4733`; `admit_cli_wrapper_journaled` `worker_spawn.rs:823` HEAD), so a manifest change takes effect at the next section; `cli_wrapper_loaded` gains `manifest_sha256`; the in-flight Worker's token and workspace (19-3 D-19-3-02 A) are untouched until its own exit; section k+1 carries section k's distillate (FR21); the door refuses `[cli_wrapper]` targets typed; an ADR-060 amendment names `[cli_wrapper]` as FR10's subprocess shape; depends on 19-3 (`worker_spawn.rs`) and 19-2's hold seam (D-19-2-04 A) for a deterministic vector; maos-bin +15–25 · **B — wasm-component in-flight swap here:** WIT `snapshot`/`swap-in` exports, runner protocol, successor-factory arm (`wasm-host` builds only); unpriced, [INFERENCE] +150–300 across `wit/`, maos-wasm-host, maos-bin, outside every E19 envelope · **C — route the spawned-form swap clause to `20-9-spawned-form-lifecycle-hooks`** (charter: lifecycle ABI reaching spawned forms, ◆ FLAG-Winston priced there; tracker `:306`); 19-5 closes FR10's in-process clause (D-19-5-11) and labels the boundary: a `[cli_wrapper]` target → typed `409 cli_wrapper_is_not_a_spirit_form` (today 400 `target_manifest_unparseable`), wasm keeps `spawned_surface_unsupported`; maos-bin +6–10 · **D — A + C** | **C.** ADR-060 `:27-31` — `[cli_wrapper]` is "not a form string"; the only long-lived subprocess Spirit form is `wasm-component`, whose swap needs exactly the spawned-form lifecycle ABI 20-9 exists to build. A Worker has no in-process state to carry (`supervision.rs:2324-2345`) and a vendor agent CLI exposes no snapshot, so A's "swap" is a file edit between tasks dressed as FR10. A is the operator's call if the tracker's intent ("the subprocess Worker is E19's form") stands; D adds 20-9's routing to it. | RULED 2026-10-28: **C** (ask `w3_19_5_d10`; operator-approved scope cut — FR10's subprocess-form clause routed to `20-9-spawned-form-lifecycle-hooks`; a Worker upgrade target gets a typed refusal) |
| **D-19-5-11** | FR10 in-process witness — tokens and working memory survive a hot swap | **A — kernel integration test** in `crates/maos-kernel-core/tests/hot_swap_same_major_lifecycle.rs`: issue a token at the predecessor pid (`CapTokens::issue`, `crates/maos-capability/src/cap_tokens/mod.rs:150`), `set_scalar` a working-memory slot (`working_memory/store.rs:45`, keyed `(pid, tag)`), hot-swap, then `verify` (`:220`) and `get_scalar` (`:88`) succeed and the pid is unchanged; hermetic · **B — daemon-level in root E** (`one_daemon_one_door_16_1.rs:973-…`): needs a token minted by the live butler and a working-memory read path through the door [INFERENCE: no door route reads a scalar slot] · **C — A + B** | **A.** The property is the coordinator's (same SCB Arc, same pid — `coordinator.rs:301-319`); a kernel test proves it without a daemon. Proven red is a swap that re-keys the pid. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-12** | kloc for 19-5 (+186…+201 vs the epic's 150) | **A — absorb inside the E19 maos-bin envelope 31353**: exact re-pin at landing with the cumulative-use line; STOP and return a D-row if the cumulative would exceed 31353 · **B — raise now by 19-5's excess**: (201 − 150) × 1.3 = +66 ⇒ maos-bin 31353 → **31419**, aggregate 191248 → **191314** · **C — trim to ≈ 150**: D-02 B (≈ −35), D-09 B (−6), D-08 B (−3) ⇒ ≈ 142–157 | **A.** Whether 19-5 breaches depends on 19-2 and 19-4, which are not built (506 left from the tree; fits at their lower bounds); "a grant is measured, never estimated" (`xtask/kloc.toml:342`); the kernel's −43 lowers the aggregate. C drops the cold-swap compensation and an ABI control to save ≈ 44 lines. | SUPERSEDED 2026-10-28 by the E19 kloc ruling (ask `w3_kloc`, "Raise to cover forecasts ×1.3"): maos-bin envelope 31353 → **31710**, xtask 47624 → **47688**, aggregate hardfail 191248 → **191669**; exact re-pin at landing with the cumulative-use line |
| **D-19-5-13** | Review tier | **A — DIFFICULT** (separate opus Blind + Edge, opus Acceptance, glm Test-Infra) · **B — standard** | **A.** Kernel FLAG-Winston re-pin, a new load refusal (fail-closed vs fail-open on unreadable archives), and a concurrency window (unload → admit under `upgrade_lock` and the per-Spirit lock) — `proof-contract.md:67-71`. | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| **D-19-5-14** | Where the vectors live and how CI runs them | **A — extend `one_daemon_one_door_16_1.rs`** (job `one-daemon-one-door`, `discipline.yml:6063` HEAD, `--test-threads=1`, 30-min ceiling): `ac4_cold_swap_is_refused_typed` (`:674-706`) cut over to the cold-swap vectors; FR11, migrator, ABI and Worker-refusal vectors beside root E; kernel tests in `upgrade_orchestrator_three_policies.rs` (job `upgrade-policy-corpus` `:1807`) and `hot_swap_same_major_lifecycle.rs` (workspace suite) · **B — NEW `crates/maos-bin/tests/cross_form_upgrade_19_5.rs`** + a step in `one-daemon-one-door` (YAML +2) · **C — new file, workspace-test-suite only** (no named job) | **A.** Zero YAML; the test that must change (clean cutover) lives there; the job is named and already builds `maosctl` (tracker `:254`). Its wall-clock with 4–6 more daemon roots is owed (measurement 5). | RULED 2026-10-28: **A** (ask `w3_19_5`) |
| CR-1 | Fail-closed (all options) | FR11: an absent archive root or spirit directory ⇒ admit; an archived record that cannot be read or parsed ⇒ refuse (`manifest_unreadable` / `manifest_parse_failed` naming the archive path), never admit; a `read_dir` error other than NotFound ⇒ refuse. Cold swap: no recorded predecessor manifest ⇒ refuse before unload (`cold_swap_predecessor_unknown`); a persisted migration plan for the Spirit ⇒ cold swap refused typed (plans are hot-swap/migrator hops, `migration_plan.rs:42-54`). | — |
| CR-2 | Log before act (I2) | Every step journals its own rows in order: predecessor `lifecycle.unload` + `cap.revoke` (scheduler), successor `lifecycle.load` + vetter-key event (admission), then the `spirit.upgrade` TL row and journal `Upgrade`; a refused successor still writes the vetter "admission rejected" row (`admission.rs:764-771`). | — |

## Acceptance Criteria

*(Derived at creation from the tracker row, the routing audit, the PRD and the 16-1 record — the epic carries no §19-5 (`:159`). Worded to the recommended rows; a different ruling rewrites only that clause. No vector needs T3.)*

### AC1 — command and observable: cold swap over the door (FR49; D-01 A, D-02 A)

1. Command (root E's shape, `one_daemon_one_door_16_1.rs:973-…`, `MAOS_IDLE_FAST=1`): a live `maos run spirits/butler/manifest.toml`, then `maosctl spirit upgrade butler --to <butler manifest with version 0.3.1 and NO appended sections> --policy cold-swap`.
2. Observable: exit 0; the report names `policy:"cold-swap"`, `outcome:"completed"` and the new pid; `maosctl spirit inspect butler` ⇒ `Running` at a pid ≠ the predecessor's; the TL holds, in order, the predecessor's `lifecycle.unload` and its `cap.revoke` rows, the successor's `lifecycle.load` and vetter-key event `admission granted` at the new pid, then one `spirit.upgrade` row `{policy:"cold-swap", predecessor_version:"0.3.0", successor_version:"0.3.1", outcome:"completed"}`; the journal holds `Upgrade`; a NEW `belief_variance` row appears at the new pid (the successor is built by the same class switch as `maosctl load`, so it is faithful).
3. Reverted (D-02 A): a target the admission triple refuses after the unload (`[sandbox] tier = "T2"` ⇒ `rust_inproc_requires_t0`) ⇒ typed `cold_swap_reverted` naming the refusal; butler `Running` again at a new pid with version 0.3.0; `spirit.upgrade` row `outcome:"reverted"`.
4. Cutover: `ac4_cold_swap_is_refused_typed` (`:674-706`) is replaced, not kept beside; `crates/maos-cli/tests/spirit_upgrade_test.rs:160-193` re-worded to map the door's cold-swap outcomes; the maos-control doc `lib.rs:283-285` and the door comment are rewritten.
5. **Proven red:** in the door's cold swap, replace the admission triple with a bare `scheduler.load_obj` (the old kernel arm's shape) ⇒ no vetter-key event at the new pid ⇒ AC1.2 fails. Planted once in place, restored, recorded in `19-5-cross-form-upgrade-and-cold-swap-evidence/plants.md` (`proof-contract.md:24-31`).

### AC2 — the kernel has no unadmitted cold-swap path (FLAG-Winston; D-01 A, D-07 B, D-09 A)

1. Kernel diff = `creation/krec-recommended-bundle-k1-k6-k8.diff`'s shape: the `ColdSwap` arm, variant and `UpgradeError::Lifecycle` deleted (`grep -rn 'UpgradePolicy::ColdSwap' crates` = 0); `check_abi_triple` extracted (`admit_spirit` calls it); `load_bundle_from_file` defaults `[scheduling]`/`[lifecycle]`. Nothing else in `crates/maos-kernel-core/src`.
2. Re-pin in the same commit, LOWER: `kernel-core-baseline.toml:505` (+HISTORY; content pins `:920`, `:954`), `kloc.toml:264`, the surface pin and a class row for `check_abi_triple` (`check-service-boundary`). Expected −41 / −43 on top of whatever 19-2/19-3 landed first.
3. Kernel tests cut over: `upgrade_orchestrator_three_policies.rs:188-216` → two policies; `upgrade_cold_swap_with_inflight_tasks.rs` deleted or re-targeted at the hot-swap saga; smoke arm `main.rs:7546-7562` (HEAD) step removed and `tests/integration/smoke_upgrade_revoke_5.sh:16-23,31-35` updated (3 JSON lines).
4. The door pre-checks every upgrade target with `check_abi_triple` before any unload or swap: a target declaring `min_substrate_version = "99.0.0"` ⇒ `400 substrate_too_old` for hot, migrator and cold; predecessor untouched (same pid, `Running`). **Proven red:** remove the pre-check call ⇒ the hot swap completes.
5. A hot-swap target that is the shipped butler manifest with only its version bumped completes; root E's fixture drops its `[scheduling]\n[lifecycle]` append (`:995-998`).

### AC3 — FR11: the archived-predecessor gate (D-03 A, D-04 A, D-05 A)

1. Record: after `maos run` admits butler 0.3.0 with `MAOS_ARCHIVE_DIR=$T/archives`, `$T/archives/butler/0.3.0/manifest.toml` equals the admitted text and `snapshot.cbor` is empty; a second boot does not rewrite it (write-if-absent); after AC1's cold swap and after a completed hot swap, the successor's record exists.
2. Refusal: with that record, `maos run <butler 1.0.0 with [hot_swap] state_schema_version = 65536 and no [migrates_from]>` exits non-zero before any pid is allocated (no `lifecycle.load` row), stderr names `migrator_missing` and `EMigratorMissing`, the archived and successor versions; `maosctl load` of the same file ⇒ `400 migrator_missing`.
3. Admit: the same successor with `[migrates_from] versions = ["0.3.x"]` is admitted; a same-major successor (`65536` → `65537` record) is admitted with no `[migrates_from]`.
4. Fail-closed (CR-1): an unreadable `manifest.toml` in the latest record ⇒ refused naming the archive path; no archive root ⇒ admitted.
5. **Proven red:** remove the gate call in `load_admit_start` ⇒ AC3.2's load is admitted.
6. Rule-11 label: the refusal's doc and the record state it is the daemon's admission chokepoint (`load_admit_start`), not `maos-kernel-core` (D-19-5-05 A).

### AC4 — FR10 in-process: the ADR-020 tree reads the real predecessor, and tokens / working memory survive (D-06 A, D-11 A)

1. A butler predecessor admitted by `maos run` with `[hot_swap] state_schema_version = 65537` (major 1) hot-swaps to a 0.3.1 successor declaring the same `65537` with no `[migrates_from]` ⇒ `completed` (same major; note `validate_logical_envelope` refuses a same-major successor whose schema is HIGHER than the predecessor's, `state_codec.rs:163-169` — unchanged here, so the vector uses equal schemas). **Proven red:** revert D-06 A (bundle drops the section) ⇒ the predecessor reads as `1` (major 0), the swap is cross-major and fails `EMigratorMissing` (`coordinator.rs:273-299`, `migrator.rs:48-68`).
2. Kernel witness (`hot_swap_same_major_lifecycle.rs`): a token issued at the predecessor pid before the swap verifies after it; a working-memory scalar slot set before reads back after; the pid is unchanged. **Proven red:** perform the transition as unload + load (new pid) ⇒ both reads fail.
3. **r19_x1 (2026-10-28; D-19-5-06 A extended):** `GatedManifest::bundle()` (`crates/maos-bin/src/admission.rs:404-412`) also carries `[on_crash]`, `[on_revocation]` and `[supervision]` for class Spirits, parsed by `gate_manifest` and carried beside the swap sections, so a class Spirit admitted by `maos run` or `maosctl load` keeps its declared crash-disposition and supervision sections (cross-story note X1, ruled into 19-5). Observable: a vector admits a class Spirit whose manifest declares the three sections and asserts the admitted SCB bundle carries them. Kloc note: ~+6–10 maos-bin.

### AC5 — FR49: migrator over the door (D-08 A)

1. `maosctl spirit upgrade butler --to <successor declaring [migrates_from]> --policy migrator` ⇒ exit 0, report `policy:"migrator"`; without `[migrates_from]` ⇒ `409 migrator_not_declared`; maosctl no longer exits 2 for `--policy migrator`. `spirit_upgrade_test.rs` header and migrator vector cut over.
2. maos-control parses `migrator`; an unknown policy stays `400 bad_request` (`lib.rs:1469-1475`).

### AC6 — spawned forms are labelled, not swapped (D-10 C)

1. `maosctl spirit upgrade worker --to spirits/worker/manifest.toml` ⇒ `409 cli_wrapper_is_not_a_spirit_form` ("a `[cli_wrapper]` Worker is spawned per task; a manifest change takes effect at its next spawn (ADR-060)") instead of `400 target_manifest_unparseable`; the wasm successor refusal (`wasm_form_admission_17_3b.rs:338`) is unchanged.
2. Records: FR10's spawned-form clause routed to `20-9-spawned-form-lifecycle-hooks` (`records-owed.md` T2); routing audit `:173` split at landing.

### AC7 — the record tells the truth (rule 11)

Comments that describe the old behaviour are updated where they live outside the kernel: `maos-control/src/lib.rs:278-287`, `operator_door.rs` (the cold-swap refusal comment, `upgrade_command`'s doc "hot-swap only"), `subcommands.rs:2379-2381`, `migration_plan.rs:5-7` if it names cold swap. The stale kernel doc `hot_swap/archive.rs:3-7` ("Used by the migrator path to detect…") is not edited under D-05 A (no kernel byte in `archive.rs`) and is listed in the Dev Agent Record as known-stale (owner: the next FLAG-Winston story touching `archive.rs`, rule 10(b)). `deferred-work.md:97` and the tracker `:254` cold-swap item close at landing (`records-owed.md`).

## Tasks / Subtasks

- [ ] **T0 — Preflight.** D-19-5-01…14 RULED; tracker `:291` → `in-progress`; records-owed R1–R5/E1–E5/T1–T2 applied by the parent. Re-measure every HEAD cite in `main.rs` / `supervision.rs` / `worker_spawn.rs` / `discipline.yml` after 19-3/19-6 land. Run measurements owed 1–4; if R measures other than −41 / −43, STOP and return a FLAG-Winston D-row.
- [ ] **T1 — Kernel** (AC2.1–2.2): apply R's shape; re-pins; surface + class row; FLAG-Winston obligations (`kernel-core-baseline.toml` procedure).
- [ ] **T2 — Admission** (AC3, AC4.1, AC4.3): `GatedManifest` swap sections + r19_x1's `[on_crash]`/`[on_revocation]`/`[supervision]`; FR11 gate + `MigratorMissing`; write-if-absent record; manifest-path record at the three admission sites.
- [ ] **T3 — Door** (AC1, AC2.4, AC5, AC6): cold swap from the admission triple with restore; ABI pre-check; migrator policy (maos-control, maos-cli); typed `[cli_wrapper]` refusal; successor record on completed upgrades.
- [ ] **T4 — Cutover** (AC1.4, AC2.3, AC5.1): door/CLI tests; kernel tests; smoke arm + script; root E fixture append dropped.
- [ ] **T5 — Vectors + plants** (AC1.5, AC2.4, AC3.5, AC4.1–4.2): one plant per AC, restored byte-identical, `plants.md` with wall-clock.
- [ ] **T6 — Truth** (AC7): comments; Dev Agent Record lists the known-stale kernel doc.
- [ ] **T7 — Close-out:** `cargo fmt --all`; `kloc-check --json` exact re-pins (kernel `:264` lower, maos-bin `:517`, maos-control `:484`, maos-cli `:489`, `:718`) with the cumulative-use line inside the ruled D-19-5-12; `check-kernel-baseline` at the new lower pin; `check-service-boundary`; `check-test-cost`; `prepush` exit 0 recorded (`proof-contract.md:144-148`); Dev Agent Record, File List, Change Log; tracker. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Kernel | `crates/maos-kernel-core/src/lifecycle/upgrade.rs:3-6,18,112-130,138,162-210,301-326,373-374,401-402`; `security/mod.rs:283-332`; `lifecycle/mod.rs` (unchanged under R); `xtask/kernel-core-baseline.toml:505,920,954`; `xtask/kloc.toml:264`; surface pin; `xtask/kernel-api-classes.toml:583-587` family | unadmitted cold arm → deleted; ABI legs → shared fn; strict target parse → admission's defaults |
| Admission | `crates/maos-bin/src/admission.rs:201-283,385-412,477-600,693-849` | bundle drops swap sections, no FR11 gate, no record → carried, gated, recorded |
| Door | `crates/maos-bin/src/operator_door.rs:54-137,830-985,1926-1944`; `crates/maos-control/src/lib.rs:278-287,1466-1476`; `crates/maos-cli/src/subcommands.rs:2379-2387,2437` | cold refused / migrator exit 2 → both performed |
| Composition root | `crates/maos-bin/src/main.rs` — `BinPrivateOps` impl (`:9419-9582` HEAD, tree `:9505-9668`), the admission call sites (topology class arm, standalone arm, door `load_spirit`), smoke arm `:7546-7562` HEAD | + `admitted_manifest`; smoke cold step deleted |
| Tests | `crates/maos-bin/tests/one_daemon_one_door_16_1.rs:674-706,973-…`; `crates/maos-cli/tests/spirit_upgrade_test.rs:1-25,160-193`; `crates/maos-kernel-core/tests/{upgrade_orchestrator_three_policies,upgrade_cold_swap_with_inflight_tasks,hot_swap_same_major_lifecycle}.rs`; `tests/integration/smoke_upgrade_revoke_5.sh` | cut over + new vectors |

**File overlap with siblings (rule: serialize the merges):**
- `main.rs` — all of 19-2 (FR20 door + safe point, topology loop `:4853-5128` HEAD), 19-3 (uncommitted: Worker spawn/workspace), 19-6 (uncommitted: resolver install `:2996-3017` HEAD region, loop crash branch). 19-5 touches only the `BinPrivateOps` impl, the three admission call sites and the smoke arm — no shared lines; re-read at T0.
- `operator_door.rs`, `maos-control/src/lib.rs`, `maos-cli/src/{cli,subcommands}.rs` — 19-2 adds the queue/recall/pick routes and verbs (D-19-2-05, AC4); 19-5 edits `upgrade_command`, `BinPrivateOps` and the upgrade policy parse only. maos-cli has 16 lines of room after 19-2's upper bound; 19-5 is −7 there.
- `xtask/kernel-core-baseline.toml`, `xtask/kloc.toml` — 19-2 and 19-3 are FLAG-Winston too; whichever lands last re-pins on top (epic `:159`); 19-5's is the only lower one.
- No overlap: `admission.rs`, `lifecycle/upgrade.rs`, `security/mod.rs` (19-2's kernel prototypes touch `scheduler_loop.rs`, `control_block.rs`, `hook_dispatch.rs`, `orchestrator/*`, `capability/mod.rs`, …; 19-3's touch `runtime.rs`, `t3/argv.rs`, `t3/spawn.rs`).

**Dependencies:** 19-3 — under the recommended D-10 C only file overlap (`main.rs`); under D-10 A/D a real dependency on `worker_spawn.rs` (manifest digest on `cli_wrapper_loaded`) and per-run workspaces (D-19-3-02 A), plus 19-2's hold seam (D-19-2-04 A) for a deterministic between-sections vector. 19-2 — door/control/cli files, the maos-bin and maos-cli envelopes, and the kernel endpoint (its +58 cap). 19-6 — `main.rs` only; cross-story finding X1 (class Spirits' `[on_crash]` dropped at admission).

### Current state that must be preserved

- Hot swap's 12-step protocol, saga and post-swap monitor (`coordinator.rs:117-450`); root E's faithful successor (the butler arm via `SuccessorManifestSlot`, `operator_door.rs:148-185`; `main.rs:3294-3378` HEAD).
- D-16-1-W's forward-only SemVer check and vetting precondition run for every policy (`operator_door.rs:887-930`); the per-Spirit lock and `upgrade_lock` (`:961-963`).
- `load_admit_start`'s ordering and rollback (`admission.rs:693-866`); Gate 8 stays before pid allocation; the FR11 gate joins it there.
- The multi-hop migration plan guard (`migration_plan.rs:42-133`) for hot swap / migrator.
- `spawned_surface_unsupported` for wasm successors; the door's `CliWrapperUnsupported` on `load`.

### Guardrails

- Kernel diff = R's shape; any extra positive kernel line is a FLAG-Winston D-row before landing (E16-A7).
- Cold swap never writes the spirits map directly; it goes through `scheduler.unload` and `load_admit_start` only.
- Pre-check (identity, forward-only, vetting, ABI) before the unload; nothing after the unload may be skipped on the restore path.
- The FR11 gate fails closed on anything but "no record" (CR-1); the record is written only after a GRANTED admission.
- Tests set `MAOS_ARCHIVE_DIR` and a fresh `MAOS_HOME`; one cargo process; `env -u CI -u GITHUB_ACTIONS`; `CARGO_TARGET_DIR=/mnt/build/cargo`; plants restored byte-identical.

### Proven-red controls (rule 11; proof contract `:24-31`)

| Control | Plant | Expected red |
|---|---|---|
| AC1 cold swap admits the successor | bare `load_obj` instead of the triple | no vetter-key event at the new pid |
| AC2.4 ABI pre-check | delete the pre-check call | `99.0.0` target hot-swaps |
| AC3 FR11 gate | delete the gate call | cross-major load admitted |
| AC4.1 swap sections carried | `bundle()` drops `hot_swap` | same-major swap fails `EMigratorMissing` |
| AC4.2 token/WM survive | transition by unload + load | `verify` / `get_scalar` fail |

### Tests and commands

```bash
# local; one cargo process; no T3 needed
P="env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo"
$P cargo build --locked -p maos-bin --bin maos && $P cargo build --locked -p maos-cli --bin maosctl
$P cargo test --locked -p maos-bin --test one_daemon_one_door_16_1 -- --test-threads=1   # the job's command, discipline.yml:6087 HEAD
$P cargo test --locked -p maos-cli --test spirit_upgrade_test
$P cargo test --locked -p maos-kernel-core --test upgrade_orchestrator_three_policies --release   # discipline.yml:1819
$P cargo nextest run --locked -p maos-kernel-core --test hot_swap_same_major_lifecycle --test hot_swap_cross_major_migration
$P cargo test --locked -p maos-bin --test wasm_form_admission_17_3b
$P cargo run -q --locked -p xtask -- check-kernel-baseline && $P cargo run -q --locked -p xtask -- check-service-boundary && $P cargo run -q --locked -p xtask -- kloc-check --json && $P cargo run -q --locked -p xtask -- check-test-cost
$P cargo run -q --locked -p xtask -- prepush
```

### Review plan (proof contract `:57-72`)

- DIFFICULT (D-19-5-13 A): Blind and Edge separate on opus; Acceptance (opus) re-reads every AC against the ruled rows; Test-Infra (glm) checks the archive isolation (`MAOS_ARCHIVE_DIR` in every new vector), the plants, and that no vector passes vacuously when the daemon fails to boot.
- Edge replays: concurrent `upgrade` and `unload` of the same Spirit; a cold swap while a halt is pending on the predecessor (its receipt on unload); a cold swap of the Orchestrator during an `--epic` run (19-1 loop holds its handle); restore refused; two archived records with equal mtimes; a downgrade; an archive record left by a crashed admission (written only after grant).

### Previous-story intelligence

- **16-1 (D-16-1-W):** hot-swap only, forward only, faithful successor; the cold-swap spike measured "successor under a new pid without `admit_spirit`" (`16-1…md:180`, `:851`).
- **16-6:** one `load → admit → start` triple for door, standalone and topology; `maosctl load` leaves `Loaded`, so cold swap must `start` explicitly.
- **17-3b / 17-3c:** spawned forms refused on in-process surfaces (`spawned_surface_unsupported`).
- **5.2 / 5.4:** hot-swap coordinator and the three-policy orchestrator — the cold arm was never admitted (deferred `:97`).
- **19-3 / 19-6 creation:** no-cargo creation by tokei replication and `/tmp` prototypes; the parent confirmed both within a line (`19-6-…/creation/measurements-results.md`).

### Git intelligence

| Commit | Content |
|---|---|
| `61af7461` | 19-1, 19-7 done (landing 40687349154) — this creation's HEAD |
| `a361bd1b` / `c80964d8` | 19-2, 19-3, 19-6 created and ruled; E19 maos-bin envelope 31353 |
| `d614eb9b` | 2026-10-27 archive split — moved 16-1's root E to `:303-309` |
| `ed2118a0` | 20-15 conmon FLAG-Winston (kernel 26367 → 26378) — last landed kernel src change |

### References

- Tracker `sprint-status.yaml:254,291,305,306`; epic `epic-19-founder-loop-w4.md:34,38-48,54-56,91,130,155,159,225`; routing audit `routing-audit-2026-10-22.md:20,173-175`; `requirements-inventory.md:591`; PRD `functional-requirements.md:39,40,43`; ADR-020 `12-architecture-decision-records.md:335-345`; `4-kernel-design.md:543-580`; ADR-060 `:16-31,54-57,69-87,179-183`; ADR-069 `:35-46`; 16-1 `16-1-daemon-post-surface-and-verb-retarget.md:43-45,180,303-309,432,851`; `05-fr1-20-scout.md:12-13,25`; `deferred-work.md:97,862,894`.
- Kernel: `lifecycle/upgrade.rs:28-33,76-296,299-375,378-491`; `hot_swap/coordinator.rs:53-67,80-99,117-450`; `hot_swap/archive.rs:3-7,40-102`; `hot_swap/migrator.rs:21-90,101-120`; `hot_swap/state_codec.rs:146-188`; `security/mod.rs:246-332`; `scheduler/scheduler_loop.rs:276-305,525-570`; `crates/maos-domain/src/hot_swap.rs:105-111`; `crates/maos-capability/src/cap_tokens/mod.rs:150,220`; `working_memory/store.rs:28-88`.
- maos-bin: `admission.rs:201-283,385-426,477-600,693-876`; `operator_door.rs:54-185,541-646,665-741,830-985,1518,1926-1944`; `main.rs:960-971,2692-2693,3107-3118,3294-3378,4702-4714,7420-7562,9419-9582` (HEAD); `migration_plan.rs:42-133`; `supervision.rs:1931,2324-2345` (HEAD); `purge.rs:52`; `crates/maos-audit/src/lib.rs:1594-1618`.
- Control/CLI/tests/CI: `crates/maos-control/src/lib.rs:278-287,1466-1476`; `crates/maos-cli/src/cli.rs:947-952`, `subcommands.rs:2379-2387,2437`; `one_daemon_one_door_16_1.rs:674-706,973-998`; `spirit_upgrade_test.rs:1-25,160-193`; `wasm_form_admission_17_3b.rs:338`; kernel tests above; `.github/workflows/discipline.yml:1807-1819,6063-6087` (HEAD); `docs/testing/proof-contract.md:24-31,57-72,144-148,182-187`.

## Dev Agent Record

### Story context creation

Created 2026-10-28 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `61af7461`, with 19-3/19-6 code uncommitted in the working tree and a build holding the host — so no cargo, build, test or xtask gate was run.

**Inputs:** direct reads of every cited range (tree, and `git show 61af7461:` for the files with uncommitted edits); `git show 3a7c4260:` of the 16-1 record for the moved cite; the kloc-check instrument replicated read-only on HEAD (`git archive`) and the tree (tokei 14.0.0; HEAD reproduces every pin, aggregate 189877; `creation/tokei-61af7461-and-tree.json`); kernel physical by `wc -l` (26378 HEAD / 26389 tree); eight kernel options and two maos-bin components priced on `/tmp/proto19x5/` copies only (`creation/measurements.md`, `k1…k8`, `krec`, `mb-*` diffs, each `git apply --check`-clean against the tree, none applied).

**Measurements owed (the parent runs them; one cargo process; exact commands in `creation/measurements.md`):**
1. `check-kernel-baseline` — HEAD PASS 26378; tree 26389 (19-3 un-pinned).
2. `kloc-check --json` — HEAD aggregate 189877, maos-bin 30380; tree 190356 / 30847.
3. R applied → `cargo check -p maos-kernel-core --lib`; gates → 26337 / 20576 (HEAD) or 26348 / 20585 (tree); `check-service-boundary` names `UpgradePolicy`, `UpgradeError`, `check_abi_triple`; revert; gates re-PASS.
4. K1…K8 one at a time, same procedure; expected figures in the kernel table.
5. Hosted wall-clock of job `one-daemon-one-door` (30-min ceiling) on the latest green main run.
6. Hosted wall-clock of job `upgrade-policy-corpus`.

**Records owed** (`creation/records-owed.md`): R1–R5 cite corrections (tracker `:291`, audit `:20`, `:173`, `:175`, deferred `:97`); P1 premise correction (the Worker is not FR10's form); E1–E5 epic (new §19-5, stories row, kernel-Δ paragraph, kloc row, order/Dependencies); T1–T4 tracker; X1 cross-story finding (admission drops `[on_crash]`/`[on_revocation]`/`[supervision]`); landing rows.

**Revert:** nothing to revert — no crate source was touched; prototypes lived only under `/tmp/proto19x5/`.

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

- `19-5-cross-form-upgrade-and-cold-swap-evidence/creation/{cites-61af7461.md, measurements.md, records-owed.md, tokei-61af7461-and-tree.json, k1-delete-kernel-coldswap.diff, k2-kernel-coldswap-admitter.diff, k3-kernel-archive-gate.diff, k4-scheduler-load-gate.diff, k5-coordinator-archive-writer.diff, k6-abi-triple-extract.diff, k7-shared-bundle-parser.diff, k8-upgrade-target-section-defaults.diff, krec-recommended-bundle-k1-k6-k8.diff, mb-admission-d03a-d04a-d05a-d06a-prototype.diff, mb-admission-d05b-variant-prototype.diff, mb-door-cold-swap-d01a-d02a-prototype.diff}`.

### Completion Notes List

### File List

### Change Log

- 2026-10-28: Story created at `61af7461` (D-19-5-01…14 PROPOSED; Status ready-for-dev pending the rulings; tracker `:291`, epic, routing audit and deferred-work untouched — corrections R1–R5, premise P1, the missing epic §19-5 and finding X1 filed in `creation/records-owed.md`). Kernel priced by reverted `/tmp` prototype (E16-A7): recommended −41 / −43; alternatives +8…+73 physical. maos-bin re-priced +186…+201 against the epic's 150 (D-19-5-12).
- 2026-10-28: Operator rulings recorded (Lunarpulse; ask `w3_19_5`): D-19-5-01 A, 02 A, 03 A, 04 A, 05 A, 06 A, 07 B, 08 A, 09 A, 11 A, 13 A, 14 A — including the FLAG-Winston kernel change K1 + K6 + K8 (net −41 physical / −43 tokei, a LOWER re-pin at landing). D-19-5-10 RULED **C** (ask `w3_19_5_d10`; operator-approved scope cut): FR10's subprocess-form clause routed to `20-9-spawned-form-lifecycle-hooks`; an upgrade whose target is a `[cli_wrapper]` Worker gets a typed refusal (AC6). D-19-5-12 superseded by the E19 kloc ruling (ask `w3_kloc`, "Raise to cover forecasts ×1.3"): maos-bin envelope 31353 → 31710, xtask 47624 → 47688, aggregate hardfail 191248 → 191669. Status → ready-for-dev (RULED 2026-10-28).
- 2026-10-28: Operator ruling **r19_x1** recorded: X1 — `GatedManifest::bundle()` (`crates/maos-bin/src/admission.rs:404-412`) also drops `[on_crash]`, `[on_revocation]`, `[supervision]` for class Spirits — ADDED to 19-5 by extending D-19-5-06 A (carry the three sections too): AC4.3 added, T2 covers it, kloc note ~+6–10 maos-bin (⇒ ≈ +192…+211, × 1.3 ≈ 250–274, still inside 31710); the "Does NOT close" X1 bullet amended (FR50/FR13 charters stay filed).
