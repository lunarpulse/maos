---
story_id: '20.3c'
story_key: 20-3c-audit-truth-and-halt-receipt-residuals
created: 2026-10-31
context_commit: 3341f9e4
baseline_commit: 3341f9e4
security_sensitive: true
kernel_grant: "◆ FLAG-Winston GRANTED 2026-11-01 (operator rulings rows203c + grant203c): grant R = K1p (D-20-3c-01 A) + E3 (D-20-3c-02 A), measured +12 physical / +26 tokei from 26793 / 20992 (R′ = K1p + E1, −1/+13, not granted). Ceilings: kernel ≤ 26817 physical / 21044 tokei; STOP above; pins re-fixed exactly at landing. ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): measured 21018 is 982 under the 22000 alarm; ceiling 21044 is 956 under; the alarm is not crossed, so no ENVELOPE ALARM row is required. Decision record: `20-3c-flag-winston-decision.md`."
kloc_grant: "RULED 2026-11-01 (grant203c; D02 A as ruled 2026-10-30; D-20-3c-09 A, ×2 measured / ×3 unmeasured). Ceilings (STOP above; pins re-fixed exactly at landing; aggregate hardfail = measured + 1): kernel ≤ 26817 physical / 21044 tokei; maos-capability ≤ 1434; maos-bin ≤ 33559; maos-domain ≤ 10097; maos-cli ≤ 6446 (raised above its 6382 STOP); maos-audit ≤ 7797; maos-shell ≤ 626; maos-iac ≤ 8345; xtask ≤ 42901; aggregate ≤ 189478."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston grant (D-20-3c-01/02) and a cross-Spirit read boundary — participant recall selects on the journaled pid (`maos-iac/src/adapter/transparency_log.rs:351-352`), so AC6's attribution decides which Spirit can recall a frame. Blind and Edge on opus-5.5, Acceptance on opus-5.5, Security reading on AC6, Test Infrastructure on glm-5.3 (standing policy `proof-contract.md:57-71`; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.3c: Audit-truth and halt-receipt residuals

Status: in-progress

<!-- Created 2026-10-31 at 3341f9e4 (= 714a1428, 20-3b's final landing, + one docs commit; `git log -1 -- crates` = 714a1428) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30). D05, D07, D10, D13 (preflight §5) and D-20-3c-01…11 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran (parent rule): cites re-read at HEAD (`creation/cites-3341f9e4.md`); kernel and kloc by the prebuilt gate binary run from `git archive` copies; members 11 and 16 re-measured on the prebuilt HEAD `maos` binary in isolated homes (`creation/runtime/`). Stale E20 §20-3c cites corrected in place (OLD→NEW in `creation/cites-3341f9e4.md` C11, C19–C21, D1–D6). -->

> **The capability:** *every record the runtime writes about what happened is true: a hot swap carries an active halt with its identity and replay context instead of silently ending it; a rate-limit frame that cannot be delivered is journaled or counted, never lost; a one-shot that could not drain its audit writer does not say it exited cleanly; the operator endpoint on disk is the one the root actually holds, and a refusal names who really holds it; a delegation row names the Spirit that emitted it; and the operator's audit query, the governance log and the gate legs report what they actually ran.*

**Closes:**
- Epic-20 §20-3c (E20@HEAD:174-196) as amended 2026-10-30 (D01 A; preflight §3.3): the **live residuals only** — members 11, 12, 16, 17 and the received items (E20:182) — renumbered AC2–AC11 here; AC1 is the ledger/governance truth (D05).
- `deferred-work.md:10` (FR53 identity across swap; witness `crates/maos-kernel-core/tests/routed_halt_swap_20_3b.rs:53-55`, `#[ignore]` owner 20-3c), `:955` (member 12), `:954` (member 11), `:1022` (member 16, incl. the E19 `wasm_form_admission_17_3b` recurrence), `:1062` (member 17), `:64` (EC5), `:1268` (teardown anomaly), `:1194` (FR4 CLI / `--intent-contains`), `:996` (Reza stage2 leg), `:26` (FR62 coverage owner), `:932` (member 5 stale owner — reconciled, not re-repaired).
- Routing audit `routing-audit-2026-10-22.md:46` (FR41 `--tag`, FR62(b) emitter, NFR-Ops-9 operational RPO/RTO) per D-20-3c-05/06/11.

**Does NOT close, and says so:**
- Members 1–4, 6–10, 13–15: **shipped**, with evidence (§Member inventory). Not re-implemented, not re-priced (D05; E20:178).
- The root-shutdown `ac4_v4`/`ac4_v5` orphan-bound flake family (`deferred-work.md:1216-1218`) — **owner 20-7** (preflight:307). The S3 SIGKILL/pidfd and supervisor kill-call barrier families — 20-7.
- FR10/FR55 spawned-form swap parity (20-9); the watchdog's refused-mint semantics (D07 → 20-9); the native compiled-in bus-send attribution (`deferred-work.md:1166`, kernel send path); erasure inventory categories (20-11, `deferred-work.md:1206` large-log scan); cost axes (20-14).
- External/operational evidence the repo cannot produce: production RPO/RTO cadence on a real deployment (D-20-3c-11 B routes it to an operator lane).

## Story

As **Winston and Murat (the charter's owners) and the operator who reads the audit surface**,
I want **every surviving member of E16-A3's charter repaired at origin and every shipped member closed on evidence rather than re-done**,
so that **no receipt, record, journal row, redaction, proof or hold asserts something the system did not actually do — and the ledger stops naming this key the day it is true.**

**Contract:**
- Epic §20-3c (E20@HEAD:174-196) as amended 2026-10-30 (D01 A; preflight `epic-20-preflight-2026-10-30.md` §3.3, §2 row 20-3c, §6 EndpointInUse) — create-story may tighten, not add uncited ACs; AC8–AC10 are the received items E20:182 names.
- Tracker `sprint-status.yaml:300` (MEMBER (17) routed by D-17-9-PF-R4a A); rules 9–11 (E20 confidence rules); proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers), `:144-170` (prepush).
- Sequence satisfied: 20-3a done (S:298), 20-3b done (S:299) — the gate inventory is stable.

## Member inventory at HEAD — measured, not assumed

Method: every cite re-read at `3341f9e4`; witnesses located by `grep` over `crates/*/tests` and inline test modules; no test run (no cargo). "Shipped" = code at HEAD plus a named test that fails if the repair is removed; "shipped, unwitnessed" = code at HEAD and no such test found (search stated); "live" = defect present at HEAD; "stale" = ledger row still names 20-3c for a repair another story shipped.

| # | Member | Disposition | Code at HEAD | Witness at HEAD | Ledger |
|---|---|---|---|---|---|
| 1 | audit `try_send` returned `Ok` | **shipped** (16-5) | `crates/maos-kernel-core/src/capability/mod.rs:438-448` (`AuditSinkUnavailable` `:446`); 8 sites `crates/maos-capability/src/cap_audit/mod.rs:26-38,69-98` | `crates/maos-kernel-core/tests/audit_sink_truth_16_5.rs:129` `audit_sink_truth_lifecycle_healthy_then_closed`, `:277` `every_audit_drop_site_counts_into_its_own_counter`; `crates/maos-bin/tests/audit_refusal_16_5.rs:142` | D:547-548 CLOSED |
| 2 | legal-hold check-then-act race | **shipped** (16-5) | `memory/mod.rs:86,130` serializer, guard `:457,629` | `memory/mod.rs:1102` `legal_hold_cannot_commit_inside_an_in_progress_forget` (hook `:435`) | D:566-567 CLOSED |
| 3 | journal overwrite | **shipped** (16-5) | `journal/mod.rs:101` `.append(true)` | `crates/maos-kernel-core/tests/journal_append_16_5.rs:7,43` | D:921-922 CLOSED |
| 4 | `Resumed` before context write | **shipped** (16-5) | `halt/resolver.rs:139-181` (write `:173-181` before transition) | `crates/maos-kernel-core/tests/halt_durability_16_5.rs:164` `failed_context_write_leaves_halt_retryable` | D:930-931 CLOSED |
| 5 | loaded-never-started / hook-failure skip receipt | **shipped by 16-6; ledger STALE** | `scheduler/control_block.rs:453-458` `(Loaded, Unloaded)`; `scheduler_loop.rs:562` receipts before `fire_on_unload` `:580` | `control_block.rs:582` `loaded_to_unloaded_allowed`; `crates/maos-kernel-core/tests/loaded_spirit_exit_16_6.rs:183` `loaded_but_never_started_spirit_unloads_with_a_receipt`, `:254` `unload_cleanup_runs_even_when_the_on_unload_hook_panics` | **D:932 still open, owner 20-3c** → AC1 |
| 6 | serde-failure receipt labelled `raised` | **shipped** (16-5) | `crates/maos-cli/src/subcommands.rs:2111-2137` | `crates/maos-cli/tests/halt_record_16_5.rs:2` | D:933-934 CLOSED |
| 7 | approval-journal failure loses the row | **shipped** (16-5) | `crates/maos-bin/src/operator_door.rs:1716-1726` | `halt_durability_16_5.rs:212` `journal_failure_rolls_back_and_retry_succeeds` (registry level) | D:935-936 CLOSED |
| 8 | REPL clarification uncapped | **shipped, unwitnessed** | `crates/maos-shell/src/lib.rs:424,462-469` | none: `grep -rn 'clarification refused\|MAX_CLARIFICATION'` hits only `lib.rs` | D:937-938 CLOSED → AC11 |
| 9 | bare `sk-` scrubs `task-` ids | **shipped** (16-5) | `crates/maos-iac/src/adapter/redaction.rs:81,145-149` | `crates/maos-iac/tests/audit_truth_16_5.rs:29` `secret_prefixes_require_a_token_boundary` | D:948-949 CLOSED |
| 10 | kind-1 rows invisible to FR4 | **shipped** (16-5); the distinct FR4 CLI defect is live (AC8) | `transparency_log.rs` kind/token pass-through | `audit_truth_16_5.rs:5`; `crates/maos-audit/tests/fr4_worker_rows_16_3.rs:110` | D:952-953 CLOSED |
| 11 | one-shot drains time out, exit 0 | **LIVE — measured** | `crates/maos-bin/src/main.rs:7486-7490,7644-7648,7861-7865,9459-9463` | runtime at HEAD binary: all four arms print `audit writer drain timed out after 5s`, rc 0, 3/3, 5.21–6.27 s (`creation/runtime/oneshot-drain-3341f9e4.txt`) | D:954 open → AC4 |
| 12 | swap drains a pending halt, no receipt | **LIVE** | `crates/maos-kernel-core/src/halt/mod.rs:466`; precheck predicts migration `hot_swap/precheck.rs:50-70` | `routed_halt_swap_20_3b.rs:55` (`#[ignore]` `:54`, red per 20-3b) | D:955 + D:10 open → AC2 |
| 13 | uninstall emptiness from shared tier only | **shipped** (16-5) | `main.rs:10355,10571` private-tier counts | `crates/maos-audit/tests/private_tier_count_16_5.rs:4`; `crates/maos-bin/tests/erasure_uninstall_13_5b.rs:1083` | D:961-962 CLOSED |
| 14 | `awaiting response` untyped | **shipped, response phase unwitnessed** | `crates/maos-a2a-tcp/src/transport.rs:687-690` | none for the response phase: `"awaiting response"` occurs only at `transport.rs:689`; `t_2c_fault_windows.rs:109-178` stalls the handshake | D:827-831 CLOSED → AC11 |
| 15 | cohort pin mismatch | **shipped** (19-7) | — | `crates/maos-bin/tests/f4_pairing.rs:1174` V4; runs 40671241869 / 40687349154 | D:994 closed |
| 16 | `control.json` endpoint not held | **LIVE — reproduced at HEAD** | `crates/maos-domain/src/operator_door.rs:362-372,410-438`; `main.rs:3516-3531,3633-3651` | `creation/runtime/endpoint-reproduction-3341f9e4.json` (init holds nothing; foreign holder ⇒ exit 78 "another MAOS root"; released ⇒ rc 0) | D:1022 open → AC5 |
| 17 | delegation rows journal pid 0 | **LIVE** | `crates/maos-bin/src/delegation.rs:455,584,713,782` | none (all four pass `0`) | D:1062 open → AC6 |
| R1 | FR53 identity/replay across swap | **LIVE** (= member 12) | as 12 | as 12 | D:10 → AC2 |
| R2 | EC5 RateLimited no-runtime / failed delivery | **LIVE** | `crates/maos-kernel-core/src/inference/mod.rs:139,195-199` (no runtime ⇒ nothing; `let _ =` discards delivery error) | none | D:64 → AC3 |
| R3 | teardown anomaly after TL close | **LIVE** | `crates/maos-bin/src/escape_detector_consumer.rs:94-108`; `main.rs:5874,6042,6955,9738` | none | D:1268 → AC7 |
| R4 | FR4 seed classification / shell `--intent-contains` | **LIVE** (recorded, not re-run) | `crates/maos-shell/src/lib.rs:183-194` (no intent filter) | D:1194 records `Fr4SchemaViolation` on the 17-3d root | D:1194 → AC8 |
| R5 | FR41 `--tag` | **LIVE** | `subcommands.rs:2768` refusal, `_tag` `:2940` | `subcommands.rs:5637` pins the refusal parse | routing-audit:46 → AC8 |
| R6 | FR62(b) ABI proposal/ratification emitter | **LIVE — no writer** | reader `crates/maos-audit/src/lib.rs:525-590`; ratifications in `xtask/abi-ratifications.toml` (19) via `check_abi_ratification.rs:1-13` | `crates/maos-audit/tests/ratification_loader_17_9.rs` (reader only) | D:26 → AC9 |
| R7 | NFR-Ops-9 operational RPO/RTO | **LIVE claim boundary** | `xtask/src/check_rto.rs:20-24` synthetic default; `check_rto_gate.rs:1-12`; `.github/workflows/rpo-rto-cadence.yml:115-140` | ledger contents not inspected | routing-audit:46 → AC10 |
| R8 | Reza stage2 leg names an absent test | **LIVE** | `xtask/src/check_reza_production_path.rs:1070,1078`; `evidence_ledger.rs:252-253`; duplicate of `check_multi_tenant_loom.rs:1583` | — | D:996 → AC10 |

## Current-tree corrections — read before implementing

Full OLD → NEW table: `creation/cites-3341f9e4.md` (C1–C21, D1–D9, T1–T8).

| Premise (epic / preflight / tracker) | Fact at `3341f9e4` and consequence |
|---|---|
| Member 11 at `main.rs:4489-4494,5859-5872,6025-6040` (E20:180) | Those are the `maos shell`/`maos run` drains; `maos shell` does **not** time out (0/2). The live sites are the four `MAOS_ONE_SHOT` arms `:7486-7490,7644-7648,7861-7865,9459-9463`; precedent repair `--once` `:6880-6955`. Corrected in E20 (C11). |
| "Per-arm current failure unmeasured" (E20:180, PF:211) | Measured: 4/4 arms, 3/3 runs time out at 5 s and exit 0 (T3). |
| Member 12 is "no receipt" | Also a precheck lie: `HotSwapPrecheck::check` predicts `SafeMigrated` for any pending halt of **any** Spirit (`precheck.rs:39` pid unused, `:50-70`) while the swap drains the predecessor's (`halt/mod.rs:466`) — T4; and a compensated swap (`coordinator.rs:177-186`) restores the predecessor with its halts already drained. |
| "No separate recurrence log" for the E19 `EndpointInUse` (PF:216) | Recorded in `19-1-…md:329` and `19-7-…md:297` (orchestrator-reported, no run log); mechanism re-reproduced twice at HEAD (T1). |
| Root-shutdown / flake families belong here | Only member 16's `EndpointInUse`; ac4 family is 20-7's (T2). |
| Members 8 and 14 closed | Code shipped, no witness (T5) → AC11 adds witnesses only, zero production lines. |
| FR62(b) "add emitter" | Reader and domain type exist; no writer; ADR-045 §4 claims TL ratification frames while the gate reads a TOML (T6). |
| I14 owner | `docs/invariants/I14.md:4-10` names 20-3c — stale at close (T7) → AC2.6. |
| Kernel cite `kernel-core-baseline.toml:27-60` (E20:194) | Obligation `:27-28` + EOF; pins `:505`, `:915`; `kloc.toml:264` (C21). |
| Ledger anchors D:1130/3/1202/933/1070/1171 | D:1194/64/1268/996/1062/1166 (D1–D6). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`, tokei 14.0.0) run from a `git archive 3341f9e4` copy root per prototype; the untouched copy reproduces the tree exactly (26793 / 95 files / set hash `a7719696…`; aggregate 188451). Detail and diffs: `creation/measurements.md`, `creation/kloc-kernel-3341f9e4-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, PROPOSED):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| K1 carry-not-drain (D-01 B) | −15 (26778) | +1 (20993) | — | `k1-carry-not-drain.diff` |
| K1p K1 + precheck on the carried set (D-01 A) | −8 (26785) | +7 (20999) | — | `k1p-carry-and-precheck.diff` |
| K2 drain + `SwapDrained` receipt (D-01 C) | +15 (26808) | +14 (21006) | maos-domain +2 | `k2-drain-with-swap-receipt.diff` |
| E3 journal-or-count (D-02 A) | +20 (26813) | +19 (21011) | maos-capability +1 | `e3-ratelimited-journal-or-count.diff` |
| E1 counted drop (D-02 B) | +7 (26800) | +6 (20998) | maos-capability +1 | `e1-ratelimited-counted-drop.diff` |
| E2 sync journal (D-02 C) | +14 (26807) | +13 (21005) | — | `e2-ratelimited-sync-journal.diff` |
| **R = K1p + E3 (recommended)** | **+12 (26805)** | **+26 (21018)** | maos-capability +1 | `krec-k1p-e3-bundle.diff` |
| R′ = K1p + E1 | −1 (26792) | +13 (21005) | maos-capability +1 | `kalt-k1p-e1-bundle.diff` |

Both pins move with any shape (physical decreases still re-pin: content hashes change). Not compiled ([INFERENCE]; measurement owed item 1).

**kloc forecast** (tokei; HEAD = every pin, zero headroom; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · R (AC2, AC3) | **+26** (physical +12) | prototype `krec` |
| maos-capability · `RateLimited` drop site (AC3) | **+1** | prototype `e3`/`e1` |
| maos-bin · sender attribution (AC6, D-03 A) | **+2** | prototype `p17` |
| maos-bin · endpoint lease + publication + holder attribution (AC5, D10 A) | +25…+50 [INFERENCE] | `main.rs:3516-3560,3621-3657` (≈ 80 lines touched) |
| maos-bin · one-shot drains complete + typed failure (AC4, D-04 A) | +0…+40 [INFERENCE] | `--once` repair `:6880-6955` dropped ≈ 20 holders; four arms |
| maos-bin · teardown anomaly journaled (AC7) | +5…+25 [INFERENCE] | `escape_detector_consumer.rs:94-108,310` |
| maos-bin · FR62(b) emitter (AC9, D-05 B) | +30…+60 [INFERENCE] | `governance-admit` arm `main.rs:7249-7290` (≈ 40 lines) |
| maos-domain · control-file runtime publication / lease (AC5, D10 A) | +20…+40 [INFERENCE] | `operator_door.rs:321-438` |
| maos-cli · `--tag` filter (AC8, D-06 A) | +10…+25 [INFERENCE] | `subcommands.rs:2756-2770,2940` |
| maos-audit · tag filter + seed TaskAssign classification (AC8) | +10…+30 [INFERENCE] | `AuditFilter`; `fr4_classifier.rs` writer shapes |
| maos-shell · `--intent-contains` parity (AC8) | +3…+8 [INFERENCE] | `lib.rs:183-194` |
| maos-iac · governance write path for proposals (AC9, D-05 B) | 0…+25 [INFERENCE] | `register_schema_lifecycle` analogue |
| xtask · Reza leg relabel (D-07 B) / RPO-RTO evidence label (D-11 B) | ±0 / +5…+20 [INFERENCE] | string rename; `check_rto_gate.rs` |
| maos-a2a-tcp, maos-control, maos-cli (FR4) | 0 | tests only / untouched |
| **aggregate** | **+29 measured; +137…+352 forecast** | sum |

**Envelope (D-20-3c-09 A, RULED 2026-11-01 grant203c; D02 A ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26793 / 20992 | +12 / +26 | **26817 / 21044** (+24 / +52) | 0 | **yes — FLAG-Winston** |
| maos-capability | 1432 | +1 | **1434** | 0 | yes |
| maos-bin | 33030 | +2 measured, +60…+175 | **33559** (+4 + 525) | 0 (E19 STOP 33010 already exceeded by w10) | yes |
| maos-domain | 9977 | +20…+40 | **10097** | 0 | yes |
| maos-cli | 6371 | +10…+25 | **6446** | 11 (STOP 6382) | yes (above STOP) |
| maos-audit | 7707 | +10…+30 | **7797** | 0 | yes |
| maos-shell | 602 | +3…+8 | **626** | 0 | yes |
| maos-iac | 8270 | 0…+25 | **8345** | 0 | yes (if D-05 A/B) |
| xtask | 42841 | ±0…+20 | **42901** | 0 | yes (if D-11 B) |
| aggregate | 188451 | +137…+352 | **189478** (+58 + 969); hardfail = measured + 1 | 0 | yes |

Under D-20-3c-10 A, L1 (kernel-free) lands first and lowers nothing; L2 asks its kernel/capability rows at its dev-plan creation against L1's exact pins. Pins land exact after `cargo fmt --all`; a lower measurement lowers the pin.

**Operator-ruling list (every draw above the inherited room; RULED 2026-11-01, grant203c: all ten granted as the ceilings above):** (1) FLAG-Winston kernel row R (D-01 + D-02; R′ not granted); (2) maos-capability +1…+2; (3) maos-bin envelope; (4) maos-domain (D10 A); (5) maos-cli above STOP 6382 (D-06 A); (6) maos-audit; (7) maos-shell; (8) maos-iac (D-05 A/B); (9) xtask (D-11 B); (10) aggregate hardfail. Each figure lands exact, with a HISTORY row citing this key.

## Decisions — RULED 2026-11-01 (rows203c; grant203c for the envelope)

Preflight rows that bear on this story stay as filed (`epic-20-preflight-2026-10-30.md:289,291,294,297`); their 20-3c consequence is added.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D05** (preflight) | 3c inventory/governance truth | **A** reconcile closed 1–10/13–15, preserve live items and received scope; stage2 proof implemented if required or honestly relabelled · **B** reimplement the old 14-member list · **C** close on summary while live owners remain | **A.** Measured inventory above: 13 members shipped with witnesses (8 and 14 unwitnessed → AC11 tests only), member 5 shipped by 16-6 but D:932 still names 20-3c; B would re-buy shipped kernel bytes; C reds `check-dev-record-completeness` at close (owner key `done`). 20-3c consequence: AC1 reconciles D:932 by measurement; the stage2 fork is D-20-3c-07. | RULED 2026-11-01 (rows203c): A |
| **D07** (preflight) | Watchdog fire after mint refusal | **A** suppress dispatch when a declared required mint is refused; journal typed refusal · **B** fire untokened, attest missing authority, prove no downstream use · **C** keep current | **A, at 20-9** (preflight). 20-3c consequence: none of this story's ACs touch `schedule_watchdog.rs`. Observation filed for 20-9 (T8): `first_id` (`:326-342`) records the first issued token even when another declared scope was refused, so the fire row (`:225-232`) can read as fully authorized — a record-truth corner 20-9's A/B must cover. | RULED 2026-11-01 (rows203c): A, at 20-9 |
| **D10** (preflight) | Endpoint record ownership (member 16) | **A** boot-time socket lease + atomically published discovery: the root binds, then publishes the bound endpoint (`write_atomic`, `operator_door.rs:321`); a lease held for the root's life distinguishes a genuine second root (refuse `EndpointInUse` naming that root's pid) from a foreign holder (named as foreign; root re-binds and republishes) · **B** init starts/retains a long-lived holder · **C** retry/skip failing tests · **D** narrow: keep the fixed port and refusal, but attribute the holder truthfully (lease check + `/proc` holder pid) | **A.** Reproduced at HEAD: init holds nothing, an unrelated listener makes boot refuse as "another MAOS root" (`runtime/endpoint-reproduction-3341f9e4.json`). D fixes the lie but keeps the flake (`wasm_form_admission_17_3b`, 1 in 5261, `19-1-…md:329`); B adds a daemon to `maos init`; C masks a production lie (D13). Kernel 0. Forecast maos-domain +20…+40, maos-bin +25…+50 (D: maos-bin +10…+25 [INFERENCE]). Clients read `control.json` through `ControlFile::load` (`maos-cli/src/door_client.rs:203-214`, `maos-shell/src/lib.rs:175`) — publication must stay in that file or the loader. | RULED 2026-11-01 (rows203c): A |
| **D13** (preflight) | Reliability evidence policy | **A** repair real causes, fail-closed tests, record any authorized full-run retry · **B** bounded full-workflow retry exception by named flake and run ids · **C** skip/increase timeouts silently | **A.** 20-3c consequence: AC5 fixes `EndpointInUse` at cause; no retry/skip; known 20-7 flakes (`check-multi-tenant-loom`, `workspace-test-suite`, D:1216-1218) are recorded by name and run id if they red the landing run, never counted as this story's green. | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-01** | FR53 / member 12 kernel shape | **A** K1p: carry the predecessor's pending halts across a hot swap (no drain); validate only its carried set; precheck uses the same set (−8/+7) · **B** K1 without the precheck change (−15/+1) · **C** K2: keep draining, write a typed `SwapDrained` receipt per drained halt (+15/+14, maos-domain +2) | **A.** A hot swap keeps the SCB and pid (`routed_halt_swap_20_3b.rs:135` `Arc::ptr_eq`; both upgrade policies call `initiate_swap`, `lifecycle/upgrade.rs:141,158`), so carrying needs no re-keying; nothing ends, so member 12's missing receipt disappears with the drain, and a compensated swap (`coordinator.rs:177-186`) no longer loses halts. B leaves the precheck promising migration on other Spirits' halts (T4). C makes the record honest but destroys FR53's identity (the 20-3b witness stays red and FR53 needs a new owner). Consequence: two drain-pinning tests are rewritten (`hot_swap_halt_continuity_corpus_integration.rs:90,140`); I14 ledger per AC2.6. | RULED 2026-11-01 (rows203c): A (K1p) |
| **D-20-3c-02** | EC5 shape | **A** E3: no runtime ⇒ journal the kind-23 row synchronously (`inference/mod.rs` already holds the TL, `:41`); failed spawned delivery ⇒ counted `RateLimited` drop (+20/+19, capability +1) · **B** E1: counted drop in both cases (+7/+6, capability +1) · **C** E2: synchronous journal only, delivery error still discarded (+14/+13) · **D** close as unreachable: prove every production caller runs inside a runtime, keep code | **A.** "Never silent" in both branches and the row survives where it can (I2 log-before-deliver). B is cheaper but loses the row the operator would query; C leaves `let _ =` (`:197`); D needs a reachability proof nobody has measured (measurement 4). | RULED 2026-11-01 (rows203c): A (E3) |
| **D-20-3c-03** | Member 17 attribution per site | **A** `delegate` ⇒ the admitted Orchestrator emitter's pid (`main.rs:4908`); host-A `journal_completion` ⇒ the Worker's admitted pid (`worker_run.spirit_pid`, `main.rs:5394,5485`; frame `from` names the Worker, `delegation.rs:1133-1159`); host-B intake (`:713`) stays 0 (verified-peer ingress); host-B outcome (`:782`) stays 0 (root-minted verdict, `:751-760`) — maos-bin +2 measured · **B** as A, plus host-B outcome attributed to host-B's local Worker pid (thread it out of `handle_one_inbound` `:893`), +5…+15 [INFERENCE] · **C** keep completions at 0 and relabel them root-authored (`FrameOrigin::Kernel`, `from` = root) | **A.** Matches the contract "admitted local emitter, 0 for root/verified-peer ingress, never invent" (`adapter.rs:324-325`); the host-B verdict is minted by the root from the adapter's label, not emitted by the Worker. B is right only if the operator treats that verdict as the Worker's emission. C hides a real Spirit emission from participant recall (`transparency_log.rs:351-352`). | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-04** | Member 11 one-shot drains | **A** fix at origin: drop every `audit_tx` holder per arm (the `--once` precedent `main.rs:6880-6955`) so the drain completes, and make a drain timeout a typed non-zero exit · **B** typed non-zero exit only, holders left (arms and `onb_nfr2_timing.sh` then fail every run) · **C** retire the three smoke arms (callers `tests/integration/smoke_{epic_4,spirit_5,supervision_5}.sh`) and fix hello-spirit only | **A.** Measured: rows land (TL counts stable 3/3) but the writer never completes because a sender lives on; B turns a lie into a permanent red on the FR58 evaluator path (`tests/integration/onb_nfr2_timing.sh:78`, `server_exit_drain.sh:39`); C retires contracts this story did not set out to retire. | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-05** | FR62(b) emitter | **A** writer + reconcile: operator verb journals `GovernanceEventKind::AbiExtension` proposals and ratification status (GovernanceEvent 28) and `check-abi-ratification` reads the TL ancestor rule of ADR-045 §4 (`:38-40`) instead of the TOML · **B** writer only: journal proposals/ratifications; the gate keeps `xtask/abi-ratifications.toml`; ADR-045 §4 amended to say so · **C** route FR62(b) to a new named story; amend ADR-045 §4's TL claim now | **B.** Closes the missing record with the existing reader (`maos-audit/src/lib.rs:525-590`, test `ratification_loader_17_9.rs`) and makes the ADR say what the gate does; A rewrites a passing gate (xtask room 0) for no closed defect; C leaves FR62 (v1.0) unowned in a story with the charter. Forecast maos-bin +30…+60, maos-iac 0…+25. | RULED 2026-11-01 (rows203c): B |
| **D-20-3c-06** | FR41 `--tag` | **A** implement: `--tag` = exact match on the TL `intent` column (halts journal their tag there, `halt/mod.rs` `invoke_halt` → `insert_frame_event(…, payload.tag, …)`), in `maos-audit::AuditFilter` and both CLIs · **B** PRD DELTA: FR41's tag filter is `--intent-contains`; delete the refusal and alias · **C** defer FR41 tag to v1.x by DELTA | **A.** FR41 (`functional-requirements.md:98`, v0.5) lists tag explicitly; the refusal (`subcommands.rs:2768`) is honest but not the requirement. B/C are scope changes only the operator may make. maos-cli above its 11-line STOP room. | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-07** | Reza `stage2-refusal-live` leg (D05 fork) | **A** write the real spirit-route + tenant-audit stage-2 refusal live test (Postgres, hosted) and point leg + ledger at it · **B** relabel: rename the leg to what it runs, fix `evidence_ledger.rs:252-253`, record that no requirement names a live stage-2 refusal · **C** delete the duplicate leg (tenant boot stays in `check-multi-tenant-loom`) | **B.** `grep -i 'stage.?2' prd/*.md` is empty; 13.5g's stage-2 verdicts are already hermetic Blocking legs (`check_reza_production_path.rs:332-345`); the leg is `AdvisorySubstrate`. A invents a requirement; C drops the Reza gate's only live-substrate leg. xtask ±0. | RULED 2026-11-01 (rows203c): B |
| **D-20-3c-08** | Witnesses for shipped-but-unwitnessed members 8 and 14 | **A** add kloc-free regression vectors (REPL > 64 KiB refused before resolve; response-phase silent peer ⇒ typed `PartitionTimeout` with frame id) · **B** leave closed on code alone | **A.** Rule 11: a closed claim needs a control that can fail; zero production lines, no re-implementation. | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-09** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b needed three review-correction raises (w8, w9, w10) after creation (`kloc.toml:264,517`); ×1.3 is not demonstrated adequate (preflight §1.3). | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-10** | Landing shape | **A** one key, two landings: L1 kernel-free (AC1 partial, AC4–AC11); L2 FLAG-Winston (AC2, AC3) + AC1 close; `done` after L2 · **B** one landing · **C** split AC2/AC3 to a new key | **A.** L1 needs no kernel grant and clears the bulk of the ledger; L2 serializes with 20-9/20-13/20-14 kernel pin updates (one integration owner). C re-opens D01 A's ratified scope. | RULED 2026-11-01 (rows203c): A |
| **D-20-3c-11** | NFR-Ops-9 operational RPO/RTO | **A** in-repo operational proof: scheduled backup + restore drill on a real root's TL (hosted), gate refuses synthetic-only evidence for the operational claim · **B** label honestly: `check-rto`/`check-rto-gate` evidence carries `mode: synthetic|operational` and the gate never reports the operational claim from synthetic drills; production cadence proof → operator lane · **C** leave as is | **B.** The routing audit asks for operational RPO/RTO "not collector honesty" (`routing-audit:46`); a real deployment's cadence is operator evidence the repo cannot mint; A's hosted "real root" is still a fixture. xtask +5…+20. | RULED 2026-11-01 (rows203c): B |

## Acceptance Criteria

*(Epic §20-3c as amended 2026-10-30; clauses marked (D-…) follow the PROPOSED recommendation and are rewritten by a different ruling. Every live AC names a failing-before witness and a plant; T = tier: local = hermetic on the dev host; hosted = the discipline run.)*

### AC1 — the ledger stops naming this key the day it is true (D05)

1. `cargo run -q --locked -p xtask -- check-dev-record-completeness` passes at L2 with **no open `deferred-work.md` row owned by `20-3c-…`**: each live row (D:10, :26, :64, :954, :955, :996, :1022, :1062, :1194, :1268) carries `CLOSED <date> by 20-3c @<commit>` naming its witness, or is re-owned by rule 10 with a reason.
2. D:932 (member 5): `CLOSED by 16-6 (measured at 20-3c creation)` citing `control_block.rs:453-458,582`, `scheduler_loop.rs:562,580` and `loaded_spirit_exit_16_6.rs:183,254` — no code change.
3. Shipped members' rows are not rewritten (their CLOSED lines stand); no owner line is globally replaced (E20:182).
4. **Negative control:** with one live row left naming `20-3c` and S:300 flipped to `done` in a scratch copy, the gate reds (owner key `done`).

### AC2 — a hot swap carries an active halt; nothing ends without a record (D-20-3c-01 A) ◆ FLAG-Winston

1. Kernel: `k1p-carry-and-precheck.diff` shape — `validate_swap_halt_continuity` (`halt/mod.rs:456`) neither drains nor reads other Spirits' halts; returns `SafeMigrated{n}` for the predecessor's carried set or `SafeDrained{0}`; an incompatible successor (`validate_halt_set`) refuses the swap with the halts intact; `HotSwapPrecheck::check` (`precheck.rs:37-70`) predicts from the same set.
2. **Failing-before witness:** remove `#[ignore]` from `routed_halt_swap_20_3b.rs:54`; `active_halt_keeps_identity_and_replay_context_across_hot_swap` is red at `3341f9e4` (recorded at T0) and green after. T local.
3. New vectors (kloc-free, `crates/maos-kernel-core/tests/`): (a) another Spirit's pending halt does not change the predecessor's verdict or precheck; (b) precheck verdict == swap verdict for 0, 1, 2 carried halts; (c) a swap compensated after the I14 gate (`coordinator.rs:177-186`) leaves the halt `PendingResolution` with its metadata; (d) the resolved halt after swap writes exactly one approval row (resolver path unchanged).
4. Rewrite the drain-pinning tests (`hot_swap_halt_continuity_corpus_integration.rs:90,140`) to the carry contract; the HSIS corpus expectations (`crates/maos-eval/tests/hsis_runner.rs:125-140`) stay `SafeDrained|SafeMigrated`.
5. **Plant (one, local):** restore `registry.drain_for_spirit(predecessor_spirit_pid)` in `validate_swap_halt_continuity` ⇒ AC2.2 and (c) red.
6. I14 ledger (`docs/invariants/I14.md:4-10`): FR53 claim reads "witnessed by `routed_halt_swap_20_3b` (green)"; `claim` stays `asserted` (no property test + adversarial corpus pair); `owner` re-pointed to `20-9-spawned-form-lifecycle-hooks` (RULED 2026-11-01, D-20-3c-01 A, rows203c; spawned-form swap parity).
7. Pins: both kernel pins, content hashes for `halt/mod.rs`, `hot_swap/precheck.rs`; HISTORY rows cite this key; FLAG-Winston decision record (precedent `17-4-flag-winston-decision.md`).

### AC3 — a rate-limit frame is journaled or counted, never lost (EC5; D-20-3c-02 A) ◆ FLAG-Winston

1. Kernel: `e3-ratelimited-journal-or-count.diff` shape — no runtime ⇒ synchronous kind-23 TL row (`FrameOrigin::Kernel`, payload the `RateLimitedPayload`); spawned delivery error ⇒ `record_drop(AuditDropSite::RateLimited, …)`; `maos-capability` gains the `RateLimited` site (`COUNT` 9). `InferenceError::RateLimited` stays the primary contract.
2. **Failing-before witness (kernel test, local):** drive a 429 through the rate limiter from a plain `std::thread` (no runtime) ⇒ at `3341f9e4` zero kind-23 rows and `audit_health_snapshot()` unchanged; after: one kind-23 row. Second vector: closed IAC sink inside a runtime ⇒ the `RateLimited` counter +1.
3. `self.iac == None` (`inference/mod.rs:139`) stays a documented no-op only if no production composition root constructs the port without IAC (grep at T0, recorded); otherwise the same journal fallback applies.
4. **Plant (one, local):** make the no-runtime arm a no-op ⇒ AC3.2 red.
5. Pins: content hash `inference/mod.rs`; `maos-capability` exact; `audit_sink_truth_16_5.rs:277`'s per-variant vector extends to all nine `AuditDropSite` variants (it calls `record_drop` directly: it pins the counter mapping, not the production call sites).

### AC4 — a one-shot that could not drain says so; the four arms drain (member 11; D-20-3c-04 A)

1. Each arm (`main.rs:7486-7490` smoke-epic-4, `:7644-7648` smoke-spirit-5, `:7861-7865` smoke-supervision-5, `:9459-9463` hello-spirit) drops every `audit_tx` holder before awaiting the writer, as `--once` does (`:6880-6955`); a timed-out drain returns a typed error (non-zero exit), never "complete"/"exiting cleanly".
2. **Failing-before witness (measured at HEAD, `creation/runtime/oneshot-drain-3341f9e4.txt`):** a `crates/maos-bin/tests` vector runs the prebuilt `maos` (`fixtures::profile_binary`) per arm in an isolated home and asserts rc 0, no `drain timed out`, wall < 5 s — red today on all four (5.21–6.27 s with the timeout line).
3. Inventory recorded in the Dev Agent Record: every other timed drain in the one-shot dispatch (`main.rs:8160` smoke-upgrade-revoke-5, `:9167` cohort-a2a-daemon, `:9642,9650,9732` — owning arm not resolved at creation; modes `verbs.rs:164`, 32) measured once — fixed here if it lies the same way, else listed with its rc.
4. **Plant (one, local):** leak one `audit_tx` clone (`std::mem::forget`) in the hello-spirit arm ⇒ the arm exits non-zero with the typed drain error (proves the failure path) and AC4.2 red.
5. `tests/integration/onb_nfr2_timing.sh` and `server_exit_drain.sh` unchanged and green (FR58 evaluator path).

### AC5 — the recorded endpoint is the one the root holds (member 16; D10 A)

1. Boot holds a lease for its lifetime and publishes the endpoint it actually bound, atomically, where `ControlFile::load` reads it; a second root on the same home refuses `EndpointInUse` naming the lease holder's pid; a non-MAOS holder of the recorded port is named as foreign (pid when attributable) and does not stop boot.
2. **Failing-before witness (local, loopback):** `crates/maos-bin/tests` vector reproducing `creation/runtime/endpoint-reproduction.py` — init, foreign listener on the recorded port, boot ⇒ at `3341f9e4` exit 78 "another MAOS root" (reproduced twice at HEAD); after: boot succeeds, `maosctl` reaches the published endpoint, stderr names no MAOS root.
3. Genuine second root vector: two roots, one home ⇒ the second exits 78 `EndpointInUse` naming the first root's pid (never a fallback port for a real root).
4. `wasm_form_admission_17_3b` (`crates/maos-bin/tests/wasm_form_admission_17_3b.rs`) needs no retry/ignore (D13 A); its private-home boot path takes the same lease.
5. **Plant (one, local):** skip the lease acquisition ⇒ AC5.3 red.

### AC6 — a delegation row names the Spirit that emitted it (member 17; D-20-3c-03 A)

1. `delegate` passes the admitted emitter pid (`main.rs:4908` → `delegation.rs:455`); host-A `journal_completion` passes the Worker's admitted pid (`:584`); host-B intake (`:713`) and outcome (`:782`) keep 0; root seeds/orphans (`spirit_session.rs:384`, `supervision.rs:2516`) keep 0. Never a fabricated local pid for a remote sender.
2. **Failing-before witness (local):** per site, the journaled `spirit_pid` is the emitter (local) or 0 (root/peer) — red today at sites `:455`/`:584` (all four pass `0`); and the emitter recalls its own delegation frame through participant recall (`transparency_log.rs:351-352`) while an unrelated Spirit does not. Extends `crates/maos-bin/tests/delegation_leg_1a.rs:224` and `repin_hold_19_7.rs:171`.
3. Cross-host arm on the mTLS loopback harness (`two_host_delegation_2b`), local.
4. **Plant (one, local):** revert `delegate` to pass `0` ⇒ AC6.2 red.

### AC7 — a teardown-time escape anomaly reaches the journal (`deferred-work.md:1268`)

1. The final escape scan runs before the audit drain and journals its anomalies, or post-drain frames go through a short-lived TL handle; `maos: escape-anomaly not journaled (the Transparency Log is closed)` (`escape_detector_consumer.rs:108`) is unreachable on the four teardown paths (`main.rs:5874,6042,6955,9738`).
2. **Failing-before witness (local, hermetic):** a kind-8 kill row inserted after the last tick appears as an `anomaly.flagged:escape-detector` frame — red today (the scan runs after the drain).
3. **Plant (one, local):** move the scan back after the drain ⇒ AC7.2 red. The live T2 vector (`live_escape_anomaly_17_10.rs`) stays green on the hosted run. *(Disclosed limitation, landing 1 review: this named plant has no local witness — the local vector drives the consumer's handle, not the `main.rs` teardown order; see Dev Record.)*

### AC8 — the operator's audit query answers the question asked (`deferred-work.md:1194`; FR41; D-20-3c-06 A)

1. `maos audit query --spirit recall-test --format plain` on the 17-3d test root (`crates/maos-bin/tests/recall_mediation_17_3d.rs:957-969`) exits 0: the root's tokenless seed `TaskAssign` is classified as a non-call by the kind rule `TOKENLESS_NON_CALL_KINDS = ["task.assign"]` (`maos-audit/src/fr4_classifier.rs`), not `Fr4SchemaViolation`. *(Amended 2026-11-01 at landing 1 review, D: this was "a writer shape"; the 16-2 doorbell's reverse check cannot see `insert_routed_frame` writers, so a shape entry is not buildable. **Consequence:** a token-less `task.assign` row is not projected as a call in the NDJSON feed; token-bearing rows still project.)*
2. `maos audit query` honours `--intent-contains` exactly as `maosctl audit query` does (`maos-shell/src/lib.rs:183-194`); parity vector over one fixture DB.
3. `maosctl audit query --tag <t>` (and `maos`) filter on exact tag; the refusal (`subcommands.rs:2768`) and its parse pin (`:5637`) are replaced by a filter vector; `audit_9_1_headline_smoke.sh:181-194` updated with the code.
4. **Failing-before witnesses:** AC8.1 red today (recorded at D:1194; re-run at T0); AC8.2/8.3 red against `3341f9e4`. **Plant (one, local):** drop the shell intent filter ⇒ AC8.2 red.

### AC9 — ABI-extension proposals and ratifications are journaled (FR62(b); D-20-3c-05 B)

1. An operator path journals `GovernanceEventKind::AbiExtension` (FrameKind 28) for a proposal and for each status change, carrying `recorded_at` and `effective_at` (ADR-045 §7, `:63-65`); `maos audit query --kind governance` returns them; the ratification loader (`maos-audit/src/lib.rs:525-590`) reads them.
2. ADR-045 §4 (`:36-40`) states that `check-abi-ratification` reads `xtask/abi-ratifications.toml` until reconciled; FR62 coverage row (`tests/coverage-matrix.yaml:862-867`) names the new witness.
3. **Failing-before witness (local):** emit + query ⇒ zero `AbiExtension` rows at `3341f9e4` (no writer). **Plant (one, local):** skip the emission ⇒ red.

### AC10 — gate evidence names what ran (D-20-3c-07 B, D-20-3c-11 B)

1. The `spirit-route-and-tenant-audit-stage2-refusal-live` leg (`check_reza_production_path.rs:1070`) and its ledger mapping (`evidence_ledger.rs:252-253`) are renamed to the claim the test makes; the published ledger no longer asserts a stage-2 refusal proof; an `xtask/tests` vector fails when a leg's name claims a test its args do not run (the two legs' names vs args).
2. `check-rto` evidence records `mode: synthetic|operational`; `check-rto-gate` never reports the operational NFR-Ops-9 claim from synthetic drills (vector in `xtask/tests`); production cadence proof filed to an operator lane in the tracker.
3. **Plant (one, local):** restore the old leg name ⇒ AC10.1's vector red.

### AC11 — shipped repairs keep a control that can fail (members 8, 14; D-20-3c-08 A)

1. REPL: a clarification of `64 KiB + 1` bytes is refused before `resolve_with_context` and nothing reaches memory or the journal (`maos-shell/tests/`).
2. Transport: a TLS peer that completes the handshake and reads the request but never answers ⇒ `A2AError::PartitionTimeout { frame_id, … }` from the response phase (`transport.rs:687-690`), bounded by the partition window (`maos-a2a-tcp/tests/`).
3. Zero production lines. **Plants (local, one per vector — no other control falsifies these):** raise the REPL bound; map the response timeout to `TransportFailed` ⇒ each red.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D05, D07, D10, D13, D-20-3c-01…11 recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured here" items 1–4; re-measure every cite this story touches (rule 9). If any measured figure exceeds the ruled envelope, STOP and return a D-row.
- [ ] **T1 — L1 ledger + witnesses (AC1.2–1.3, AC11).** Reconcile D:932; add the member 8/14 witnesses and their plants.
- [ ] **T2 — One-shot drains (AC4).** Per-arm holder drop + typed failure; inventory the other drain sites.
- [ ] **T3 — Endpoint (AC5).** Lease + publication + attribution per D10; the reproduction vector.
- [ ] **T4 — Attribution (AC6).** Four sites per D-03; recall vector.
- [ ] **T5 — Teardown anomaly (AC7); audit query (AC8); FR62(b) (AC9); gate evidence (AC10).**
- [ ] **T6 — L1 landing (D-20-3c-10 A).** Exact pins for bin/domain/cli/audit/shell/iac/xtask/aggregate inside the ruled envelope; records.
- [ ] **T7 — L2 kernel (AC2, AC3)** under the ruled FLAG-Winston row: apply the ruled bundle, rewrite the drain-pinning tests, un-ignore the FR53 witness, I14 ledger, both kernel pins + hashes, decision record.
- [ ] **T8 — Plants** (one per AC unless named, `proof-contract.md:24-31`), all local; wall-clock per plant in the Dev Agent Record.
- [ ] **T9 — L2 landing.** `check-kernel-baseline`, `check-service-boundary`, `check-pub-field-constructors`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on the final tree (known 20-7 flakes recorded by name/run id, D13). Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Kernel ◆ | `crates/maos-kernel-core/src/halt/mod.rs:443-490`, `hot_swap/precheck.rs:37-70`, `inference/mod.rs:128-200`; pins `xtask/kernel-core-baseline.toml:27-28,505,915`, `xtask/kloc.toml:264` | AC2, AC3 |
| Capability | `crates/maos-capability/src/cap_audit/mod.rs:26-38` | AC3 |
| Bin | `crates/maos-bin/src/main.rs:3516-3560,3621-3657,4908,5229,5493,7486-7490,7644-7648,7861-7865,9459-9463` (+ teardown paths `:5874,6042,6955,9738`), `delegation.rs:442-455,574-590`, `escape_detector_consumer.rs:94-108,310`, governance verb | AC4–AC7, AC9 |
| Domain | `crates/maos-domain/src/operator_door.rs:321-438` | AC5 |
| CLI / shell / audit | `crates/maos-cli/src/subcommands.rs:2756-2770,2940,5637`; `crates/maos-shell/src/lib.rs:183-194`; `crates/maos-audit/src/{lib.rs,fr4_classifier.rs}` | AC8 |
| xtask | `check_reza_production_path.rs:1070-1080`, `evidence_ledger.rs:252-253`, `check_rto.rs`, `check_rto_gate.rs` | AC10 |
| Tests (kloc-free) | `crates/maos-kernel-core/tests/{routed_halt_swap_20_3b,hot_swap_halt_continuity_corpus_integration}.rs` + new; `crates/maos-bin/tests/` (one-shot, endpoint, delegation, anomaly, audit query); `crates/maos-shell/tests/`; `crates/maos-a2a-tcp/tests/`; `xtask/tests/` | all |
| Docs / records | `docs/invariants/I14.md`; ADR-045 §4; `tests/coverage-matrix.yaml:862-867`; `tests/integration/audit_9_1_headline_smoke.sh:181-194`; `deferred-work.md` rows (AC1) | AC1, AC2, AC8, AC9 |

### Overlap (serialize; re-measure)

- **`maos-kernel-core` pins** — 20-1 (D04), 20-8, 20-9 (I14 parity, watchdog), 20-13, 20-14 forecast kernel touches; one integration owner per pin update (preflight §6 "Pooled write surfaces").
- **`main.rs`** — 20-1/20-4/20-13 alter the composition root; this story's edits are local to the cited ranges.
- **`maos-audit`/CLI audit surface** — 20-11 (erasure inventory) and 20-14 (cost) alter the same query surface.
- **`halt/mod.rs` / `coordinator.rs`** — 20-9's spawned-form swap must inherit carry, not re-add a drain.

### Current state that must be preserved

- Every shipped member's behaviour and witness (§Member inventory) — no regression, no re-implementation.
- Cold swap (unload + load, new pid) still writes `PlannedUnload` receipts via `terminate_spirit` (`scheduler_loop.rs:562`).
- I2 log-before-deliver on every new journal write; `insert_frame_event` panics on write failure by design.
- The genuine second-root refusal (`main.rs:3633-3651` intent) and the authenticated door (`control.json` mode/token rules, `operator_door.rs:255-320`).
- Pid 0 for root/kernel and verified-peer ingress (`adapter.rs:324-325`); participant-recall predicates unchanged (`transparency_log.rs:342-355`).

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC2/AC3 is STOP.
- Never weaken a witness to pass; never retry/ignore `EndpointInUse` (D13).
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it; plants byte-restored in the real tree.
- A shared target dir across worktrees evicts sibling binaries tests resolve by path (`deferred-work.md:994` lesson) — no second checkout builds into it.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| FR53 carry (AC2) | restore `drain_for_spirit` | FR53 witness + compensation vector |
| EC5 (AC3) | no-op no-runtime arm | kind-23 row absent |
| One-shot drain (AC4) | leaked `audit_tx` clone | non-zero typed exit; timing vector |
| Endpoint (AC5) | skip lease | second-root vector |
| Attribution (AC6) | `delegate` passes 0 | per-site pid vector |
| Teardown anomaly (AC7) | scan after drain | anomaly frame absent |
| Shell parity (AC8) | drop intent filter | parity vector |
| FR62(b) (AC9) | skip emission | zero governance rows |
| Leg names (AC10) | old leg name | name-vs-args vector |
| Members 8/14 (AC11) | raise bound; untyped mapping | each vector |

All plants local (no T3 subject in this story); one hosted discipline run on the final tree.

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core -p maos-capability -p maos-bin -p maos-shell -p maos-audit -p maos-cli -p maos-a2a-tcp -p xtask
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test routed_halt_swap_20_3b   # no --ignored after AC2
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC6 (who can recall what) and AC5 (door endpoint/lease), Test Infrastructure (glm-5.3). Focused suites per touched crate + `cargo test -p maos-kernel-core --lib`.

### Previous-story intelligence (20-3b, 19-4, 19-5)

- 20-3b landed at 2–3× its creation forecast through three review-correction raises (w8 → w9 → w10: kernel +86/+55/+103 tokei) — hence D-20-3c-09's allowance and the ruling list.
- 20-3b's final hosted runs were not fully green on known 20-7 flakes (`check-multi-tenant-loom`, `workspace-test-suite`); record them by run id, do not absorb them (D13).
- 20-3b moved the FR53 witness here labelled red with its owner (rule 11(b)); un-ignore it, never weaken it to successor re-firing (`docs/invariants/I14.md:34-43`).
- 714a1428 taught seven test helpers to tolerate a TL read before table creation — new audit-query vectors use the same helpers.
- 19-5 is the swap/upgrade precedent: both upgrade policies go through `initiate_swap` (`lifecycle/upgrade.rs:141,158`); cold swap is a door composition, not a kernel path.

### Git intelligence

`3341f9e4` (20-3b done records), `2bc25937` (devcontainer), `714a1428` (20-3b hosted follow-up: TL-before-table tolerance in seven helpers; clippy ratchet baseline 1456/904). No crate change since `714a1428`.

### Library / framework

No new dependency. `tokio::runtime::Handle::try_current` (already used at `inference/mod.rs:195`) decides AC3's branch; `serde_json` for the kind-23 payload. No web research needed.

### References

E20@HEAD:75,174-196; preflight `epic-20-preflight-2026-10-30.md:197-221,289,291,294,297,307,309`; `deferred-work.md` rows above; `routing-audit-2026-10-22.md:46`; PRD `functional-requirements.md:98,105`, `non-functional-requirements.md:164`; ADR-045 (`docs/adr/ADR-045-governance-audit-artifacts.md:19-40`), ADR-069 (`:35-46`); `docs/invariants/I14.md`.

## Dev Agent Record

### Story context creation

Created 2026-10-31 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `3341f9e4`, without cargo (parent rule).

**Inputs:** E20 §20-3c at HEAD (amended 2026-10-30, preflight §3.3) — stale cites corrected in place; preflight §1–§6 and D05/D07/D10/D13; tracker `sprint-status.yaml:300`; every `deferred-work.md` row naming 20-3c; direct reads of every cited range (`creation/cites-3341f9e4.md`); kloc and kernel by the prebuilt gate binary on `/tmp/proto203c` `git archive` copies (`creation/kloc-kernel-3341f9e4-and-prototypes.json`); prototypes K1, K1p, K2, E1, E2, E3, R, R′, P17 saved as diffs, `rustfmt`'d, never applied to the tree; members 11 and 16 measured on the prebuilt HEAD binary (`creation/runtime/`).

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–6.

**Records owed:** `creation/records-owed.md`.

**Revert:** no crate source modified (prototypes only under `/tmp/proto203c`, deleted); no cargo/test ran, `intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (196 done stories, 152 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26793, 0 violations); `kloc-check` rc 0 (aggregate 188451, `alarm=true`, `over_budget=[]`); `check-kernel-baseline` rc 0 (26793 / 95 files, set hash `a7719696…`, no drift).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

anthropic/claude-sonnet-5-5 (Dev203c2, landing 2).

### Debug Log References

- Creation evidence: `20-3c-audit-truth-and-halt-receipt-residuals-evidence/creation/{measurements.md, cites-3341f9e4.md, records-owed.md, kloc-kernel-3341f9e4-and-prototypes.json, k1-carry-not-drain.diff, k1p-carry-and-precheck.diff, k2-drain-with-swap-receipt.diff, e1-ratelimited-counted-drop.diff, e2-ratelimited-sync-journal.diff, e3-ratelimited-journal-or-count.diff, krec-k1p-e3-bundle.diff, kalt-k1p-e1-bundle.diff, p17-sender-pid-attribution.diff, runtime/*}`.
- Landing 2 evidence (`…-evidence/landing-2/`): `plants.md`, `plant-cuts/*.diff` + `plant-cuts.sha256`, `plant-logs/{P0…P5}*.log`, `build.log`, `kernel-lib.log`, `nextest-serial.log`, `nextest-default.log`, `check-test-budget.log`, `cargo-deny.log`, `clippy-ratchet.log`, `clippy-stderr.log`, `clippy-live.jsonl.gz`, `gates.jsonl` + `gate-*.log`, `ac1-{1,4}-*.log`, `prepush.log`; hosted legs in `…-evidence/ci/ci-20-3c-verify.yml` (job `kernel-l2`).

### Completion Notes List

**Landing 1 (L1, kernel-free) — Dev203c1, 2026-11-01. Verified locally; NOT committed, NOT pushed, statuses unflipped. AC1 record edits, AC2/AC3 (L2) remain.**
- AC4 (member 11) DONE+witnessed: `release_audit_tx_holders!` macro + `supervision::await_audit_drain`/`AuditDrainError` (typed, non-zero exit) in `main.rs` arms smoke-epic-4 (also `drop(resolver)`), smoke-spirit-5, smoke-supervision-5, smoke-upgrade-revoke-5 (sets `MAOS_AUTO_REVERT_FAST=1`: the post-swap monitor holds capability/iac/halt clones for 30 s and has no non-kernel cancel), hello-spirit. Measured after: rc 0, 0.2–1.2 s, no drain line (5 arms). Witness `crates/maos-bin/tests/one_shot_drain_20_3c.rs` (5 tests); red-before plant (macro reduced to audit_tx+inference) 5/5 FAILED, green after. Inventory: `cohort-a2a-daemon` (`:9167`) and the serving tail are the 16-1/16-3 full-list drains, not changed; upgrade-revoke printed "acceptable in smoke test", rc 0, 5.5 s at HEAD (fixed here).
- AC5 (member 16, D10 A) DONE+witnessed: `maos-domain` `EndpointLease` (flock `<home>/control.lease`, pid recorded), `listener_pid`, `ControlFile::{with_endpoint,publish_atomic}`; boot takes the lease before bind, foreign holder is named (pid via /proc) and boot rebinds `127.0.0.1:0` and republishes; env-pair roots keep a plain EndpointInUse. Witness `crates/maos-bin/tests/endpoint_lease_20_3c.rs` (2 tests green); plants: lease skipped → second-root test red on the stderr assertion, NOT on the exit code (corrected by the landing-1 review, traced by reading `main.rs:3670-3678`, plant not re-run: with `endpoint_lease` left `None` the second root still hits `AddrInUse` on the first root's port and takes the `endpoint_lease.is_none()` arm, which prints `maos: operator endpoint <bind> is already in use (<holder>) — EndpointInUse` and exits 78 — the same code the test expects — but that message never contains `control.json`, so the `stderr.contains("control.json")` clause at `endpoint_lease_20_3c.rs:189-194` is what reds; the earlier "rc 0 ≠ 78" was wrong); foreign branch reverted to exit 78 → foreign test red (60 s timeout).
- AC6 (member 17) DONE+witnessed locally: `delegate(.., emitter_pid)` / `journal_completion(.., worker_pid)`; callers in `main.rs`, `delegation_leg_1a.rs` (new `delegation_rows_name_their_emitter_and_recall_follows_the_pid`), `repin_hold_19_7.rs`; `two_host_delegation_2b.rs` asserts emit pid≠0, host-B intake/outcome pid 0 (T3, hosted-only, NOT run). Plant (pid 0) → vector red.
- AC7 DONE+witnessed: `tl_journal` takes `Arc<TransparencyLogAdapter>` (closed-log arm deleted); vector `a_kill_after_the_last_tick_is_journaled_by_the_teardown_scan`; plant (weak) → red. Note: at HEAD the production root kept its own Arc alive, so the "not journaled" message could not be reproduced on the binary; the fix removes the dependence by construction.
- AC8 DONE+witnessed: `AuditFilter.tag` (exact intent match at both SQL sites); `maosctl --tag` filters (refusal and parse pin deleted); `maos audit query --intent-contains/--tag` through `run_audit_query(spirit, format, intent_contains, tag, color)`; FR4 `TOKENLESS_NON_CALL_KINDS = ["task.assign"]` in `fr4_classifier.rs` + vector `tokenless_task_assign_is_a_non_call_and_a_token_bearing_one_still_projects`. **Why a kind rule, not a writer shape (AC8.1 text):** `fr4_classifier_16_2.rs`'s doorbell has a REVERSE check — every `WRITER_SHAPES` entry must match a scanned `insert_frame_event*` source site, and the bus's `deliver_typed` writes through `insert_routed_frame`, which the scanner does not see; a shape entry was built and reverted for exactly that reason. A kind rule limited to the tokenless case keeps token-bearing `TaskAssign` rows projecting. Witnesses: `crates/maos-bin/tests/audit_query_20_3c.rs` (`intent_contains_and_tag_agree_between_maos_and_maosctl`, `spirit_view_renders_a_tokenless_seed_task_assign`); `audit_9_1_headline_smoke.sh:181-194` updated (hosted script, not run locally). Plants: tag clauses disabled + classifier rule disabled → both tests red (`left: [...all rows...]`, `Fr4SchemaViolation { line: 2 }`); shell `intent_contains: None` → parity red. The 17-3d test root's own seed (`recall_mediation_17_3d.rs`) is unchanged: the CLI witness uses a fixture with the same row shape (tokenless `TaskAssign`, intent `Standard`, raw payload).
- AC9 DONE+witnessed (D-20-3c-05 B): `maosctl governance abi-extension --proposal-id --summary --adr-ref --status proposed|ratified|rejected --covered-change … --effective-at-ns` → `governance-admit` one-shot branch on new env `MAOS_GOVERNANCE_ABI_PROPOSAL` (registered in `env_contract.rs`) → `admission::emit_abi_extension_event` journals kind 28, intent `governance:abi-extension`, `recorded_at` now, `effective_at` operator-supplied. Witness `abi_extension_journal_20_3c.rs`: two verb runs, `maosctl audit query --frame-kind governance` returns both rows, `load_ratification_frames` returns only the ratified one. Plant (emission skipped) → "one frame per status: []" red. ADR-045 §4 states the gate still reads `xtask/abi-ratifications.toml`; FR62 coverage row `notes` names the witness (`gates: []` unchanged: no gate consumes the frames).
- AC10 DONE+witnessed: Reza leg `spirit-route-and-tenant-audit-stage2-refusal-live` → `tenant-mode-boots-live` (+ `evidence_ledger.rs`); `xtask/tests/gate_evidence_names_20_3c.rs` scans both gates' `TestLeg`s: a name containing `stage2` must run a `stage2` test, and one test has one leg name. Plant (old name) → red naming the leg. `check-rto` writes `mode = synthetic|operational` (operational iff `--source` supplied), `check-rto-gate` reads it (legacy rows ⇒ synthetic) and emits `mode` + `operational_claim` (true only for a passing operational drill); vector `a_synthetic_drill_never_reports_the_operational_claim` (drives the built `xtask` binary; no RTO plant run). Production cadence proof stays on the operator lane `ops-rpo-rto-cadence-proof`.
- AC11 DONE+witnessed, zero production lines: `shell_halt_repl_16_2.rs` `an_oversized_clarification_is_refused_before_it_reaches_the_host` (64 KiB+1 refused with nothing reaching the host, exactly 64 KiB accepted) — plant (bound ×1024) red; `maos-a2a-tcp/tests/t_20_3c_response_phase.rs` (real mTLS peer reads the request, never answers ⇒ typed `PartitionTimeout` naming the frame; the peer's request counter proves it is the response phase) — plant (`Io("awaiting response")`) red.
- Tests that count the MAOS home: `maos_uninstall_16_4`, `erasure_uninstall_13_5b`, `init_test`, `root_spawn_home_isolation_16_1` and every other home-snapshot test passed with `control.lease` present (serial 5466/5466). One default-profile run showed `maos_uninstall_16_4::snapshot_restore_undoes_lock_acquisition_on_dry_run_and_contention_paths` fail on a `transparency.sqlite-shm` presence race (left had the `-shm` file, right did not; `control.lease` identical in both); 3/3 green in isolation and green serial — recorded by name, not absorbed (D13).
- Fixtures: `build-test-fixtures` had to be re-run because `maos-domain` changed (19 `maos-wasm-host` tests failed "fixture stale" in the first serial run); `invariant_claims_20_3b.rs` follows the ruled I14 owner (`20-9-spawned-form-lifecycle-hooks`).
- **kloc (tokei, after `cargo fmt --all`, `kloc-check --json` rc 0, `over_budget []`):** maos-bin 33195 (ceiling 33559), maos-domain 10081 (10097), maos-cli 6396 (6446; pin 6856 unchanged, STOP 6382 exceeded under the grant), maos-audit 7721 (7797), maos-shell 608 (626), xtask 42860 (42901), maos-iac 8270 (8345), aggregate 188784 (189478); kernel 20992, capability 1432, control 1496, a2a-tcp 1442 unchanged. Re-pinned in `xtask/kloc.toml` (first HISTORY segment `20-3c-audit-truth-and-halt-receipt-residuals landing 1`, xtask arrow `42841 -> 42860`): bin 33195, domain 10081, audit 7721, shell 608, xtask 42860, `_aggregate_hardfail` 188785. (`kloc.toml` line edits made with a scripted substring replace — the edit tool cannot express a substring of a >768-char line.)
- **Verification (L1 tree):** `cargo fmt --all` rc 0; `cargo build --workspace --all-targets` rc 0; `cargo nextest run --workspace --profile ci --test-threads 1` 5466/5466 passed (139 skipped), rc 0; default profile 5465/5466 (the `-shm` race above); `cargo clippy --workspace --all-targets` rc 0 → ratchet 1416 occurrences / 883 source-distinct (hosted baseline 1456/904 unchanged; local pre-landing 1410/881); `cargo deny check` rc 0; gates rc 0: kloc-check, check-kernel-baseline, check-service-boundary, check-env-contract (118 vars), check-serde-error-handling, check-composition-root-completeness, check-j1-loopback-delegation, check-dev-record-completeness, check-epic-close-coherence, check-review-findings-resolved, check-test-cost, invariant-lock, coverage-matrix, check-coverage-matrix-completeness; `prepush --json` rc 0 (92 records). **`check-test-budget` on the serial junit rc 1** (host-timing, not touched by this landing): `maos-bin::one_daemon_one_door_16_1` 30.513 s, `maos-iac::participant_recall_17_8` 46.791 s, `maos-kernel-core::cli_wrapper_t3_boundary_17_1` 30.353 s, `xtask::bin/xtask` 34.967 s, each over the 30 s budget.
- Unverified: hosted/T3 legs (`two_host_delegation_2b` pid assertions, `live_escape_anomaly_17_10`, `audit_9_1_headline_smoke.sh`) — wired into `…-evidence/ci/ci-20-3c-verify.yml` (diff `ci/t3-two-host-delegation-2b.diff`, sha256 `b07547c92134f4b09da864a444c70c1ccb578a98766a8160274520bb9e60a43c`); no RTO-gate plant.
- **Landing-1 review dispositions (BmadReview203c1, 2026-11-01; orchestrator-ruled).** Patched: AC8.1 `spirit_view_renders_a_tokenless_seed_task_assign` now asserts the seed row's rendered frame id, boot nonce, pid column and intent (the `|| !stdout.is_empty()` disjunction is gone); `docs/invariants/README.md` FR53 owner now `20-9-spawned-form-lifecycle-hooks` (agrees with `I14.md`); AC5.3 plant result corrected above; the `audit_9_1_headline_smoke.sh` comment now says the script proves only the unknown-tag leg (the exact-vs-substring leg is `audit_query_20_3c.rs`). **Finding 4 (maos-cli pin 6,856) DISMISSED:** 6,856 is the recorded reserved ceiling and the measured consumption 6,396 is recorded above, as in earlier waves; `xtask/kloc.toml` is unchanged. Deviations disclosed, not repaired:
  - **D1 — `smoke-upgrade-revoke-5` closes by window shortening, not holder release (ACCEPTED as a recorded deviation).** AC4.1 names four arms; this fifth arm (AC4.3) sets `MAOS_AUTO_REVERT_FAST=1` (`main.rs:7999`) because the post-swap monitor's holders are kernel-side and cannot be released without a kernel change. Its drain witness is labelled *timing-bound, not holder-release* (`one_shot_drain_20_3c.rs:65-71`): a leaked holder in this arm would pass. Deferred-work row: the 30 s kernel monitor holder.
  - **D2 — AC8.1 is a kind rule, not a writer shape (ACCEPTED).** AC8.1 text amended above; consequence: token-less `task.assign` rows are not projected as calls in the NDJSON feed. Deferred-work row.
  - **D3 — the AC7 named plant has no local witness (disclosed limitation).** "Move the scan back after the drain" does not turn `a_kill_after_the_last_tick_is_journaled_by_the_teardown_scan` red (it drives the consumer's handle); the substituted weak-`tl_journal` plant above is the local evidence. The teardown-order witness is routed to the hosted `live_escape_anomaly_17_10` leg wired in `ci-20-3c-verify.yml` (not run locally). Deferred-work row.
  - Reviewer DEFER findings that are real residuals (door-bearer to a foreign listener, lease symlink custody, kind-28 self-asserted ratification, `two_host_delegation_2b` green without T3, ABI-proposal env shadowing a schema env, unreachable `else` in `governance_offline_arm`) are deferred-work rows, all dispositioned by 20-3c landing 1 review.


**Landing 2 (L2: AC2, AC3 kernel; AC1 close) — Dev203c2, 2026-11-01, anthropic/claude-sonnet-5-5. Verified locally; NOT committed, NOT pushed, statuses unflipped. Hosted-only legs not run.**
- **AC2 DONE+witnessed (K1p, D-20-3c-01 A).** Re-cut against `3ea4bbb9`, not the prototype's patch. `validate_swap_halt_continuity` (`halt/mod.rs:463-483`) reads the predecessor's own pending halts through one new `pub(crate)` helper `HaltRegistry::carried_for_spirit` (`halt/mod.rs:442-451`: the existing dry-run, filtered to `PendingResolution`), never drains and never reads another Spirit's halts; `SafeMigrated{n}` / `SafeDrained{0}`; an incompatible successor refuses with the halts intact. `HotSwapPrecheck::check` (`precheck.rs:52`) takes the same helper, so the two cannot drift (the prototype duplicated the filter). The prototype's inline test `precheck_halt_continuity_violation` seeded an owner-less halt and became meaningless under owner-keyed carry; it moved into the integration vectors. **Found beyond the prototype, fixed (kernel, inside AC2, +0 net lines):** the 12-5 `PostSwapMonitor` snapshots the post-gate pending set and flags any snapshot halt that leaves `pending_halt_ids()`, so under carry an operator resolving the carried halt inside the 30 s window would have tripped `HaltSetLoss` and auto-reverted a valid swap (before, the swapped Spirit's halts were drained and never in the snapshot). The snapshot is now the carried set (`coordinator.rs:380-389`) and the monitor treats a halt as lost only if it left the registry (`post_swap_monitor.rs:116-127`: a resolved halt stays there terminal). Witnesses (all `crates/maos-kernel-core/tests/`): FR53 `routed_halt_swap_20_3b.rs:55` `active_halt_keeps_identity_and_replay_context_across_hot_swap` (`#[ignore]` removed; **red at the pre-change tree, green after**); `halt_carry_20_3c.rs`: (a) `:94` another Spirit's (and a legacy owner-less) halt changes neither verdict nor precheck (successor accepts only protocol 2); (b) `:116` precheck == swap verdict for 0/1/2 carried halts × compatible/incompatible successor, nothing drained; (c) `:330` an incompatible successor refuses the swap with halt + metadata intact and `:357` a swap compensated AFTER the gate (predecessor `on_swap_out` panics ⇒ `SwapOutFailed`) leaves the halt `PendingResolution` with identical metadata; (d) `:380` the carried halt resolved after a completed swap writes exactly one approval row through the unchanged resolver; (e) `:462` resolving the carried halt inside the (`MAOS_AUTO_REVERT_FAST`) monitor window is not a loss, `:480` a carried halt that leaves the registry is still a loss. Drain-pinning tests rewritten to the carry contract: `hot_swap_halt_continuity_corpus_integration.rs:90,146`, inline `halt/mod.rs:651`; the corpus wrapper vectors run on an empty registry and stay `SafeDrained{0}`. HSIS (`maos-eval/tests/hsis_runner.rs`): the runner seeded owner-less halts, which K1p correctly ignores, so it now seeds halts owned by the scenario's pid; the corpus expectations and the `SafeDrained|SafeMigrated` rule are unchanged and green. I14 ledger: `docs/invariants/I14.md` FR53 claim "asserted, owner 20-9-spawned-form-lifecycle-hooks; witnessed by routed_halt_swap_20_3b (green, 20-3c)", README row, `claim` stays `asserted`; `xtask/tests/invariant_claims_20_3b.rs` now requires the FR53 witness to stay un-ignored (the SDK witness stays a labelled red); `tests/coverage-matrix.yaml` FR53 note names the witnesses (the `invariant-lock` gate requires a matrix touch when an invariant doc moves). **Not changed (outside AC2.1's text, recorded as a deferred-work row): `HotSwapCoordinator::precheck` (`coordinator.rs:485`) still feeds the predecessor's own protocol version in as the successor's accepted list, so `maos upgrade --precheck` cannot predict a halt-protocol refusal the real swap raises.**
- **AC3 DONE+witnessed (EC5, E3, D-20-3c-02 A).** `emit_rate_limited_frame` (`inference/mod.rs:129-223`): a runtime present ⇒ spawn delivery and, on `deliver_typed` error, `record_drop(AuditDropSite::RateLimited, Full)`; no runtime ⇒ journal the kind-23 row synchronously through `insert_frame_event_with_sender` (`FrameOrigin::Kernel`, sender `__kernel`, recipient the invoking Spirit, intent `standard` — the same columns the delivered row carries, so a Spirit-recall of the undelivered row still finds it); a payload that cannot serialize is counted, not dropped silently. `maos-capability` `AuditDropSite::RateLimited`, `COUNT` 9. **Reason is `Full`, not the prototype's `Closed` (deliberate):** `Closed` latches the process-wide `degraded` flag that `maos shell` and the operator door report as a dead audit writer, and a frame with no mailbox is not that (deferred-work row, landing 2). **AC3.3 grep (T0):** `InferencePortAdapter::new` in production = `crates/maos-bin/src/main.rs:4063` and the per-Spirit `PortBuilder` `:4090`, both call `.with_iac`; the other constructors are tests, `maos-bench` and a kernel bench, so `self.iac == None` stays a documented no-op. Witnesses (`context_budget_gate_18_5.rs`): `:445` `a_429_with_no_runtime_journals_the_rate_limited_frame_synchronously` (plain test thread, `Handle::try_current()` is `Err`, one kind-23 row with `Kernel` origin, `__kernel` → `spirit:7`, payload names the provider, counter unchanged) and `:482` `a_429_whose_spawned_delivery_fails_is_counted_as_a_rate_limited_drop` (no mailbox ⇒ counter +1, `degraded` unchanged, the I2 row still journaled); **both red at the pre-change tree, green after.** `audit_sink_truth_16_5.rs:277`'s per-variant vector now covers all nine `AuditDropSite` variants (the counter mapping only: it calls `record_drop` directly and does not witness the production call sites; the `RateLimited` wiring is witnessed by `context_budget_gate_18_5.rs`).
- **AC1 closed.** `check-dev-record-completeness` passes with `20-3c-…` flipped to `done` in a scratch copy (`landing-2/ac1-1-scratch-done-after-edit.log`, 197 done stories) and **reds with exactly 21 `STALE owner` rows before the edits** (`ac1-4-negative-control-before-edit.log`, AC1.4) and with exactly one when one closed row is re-opened (`ac1-4-negative-control-one-live-row.log`). Edits to `deferred-work.md` (a scripted substring replace — the edit tool cannot express a substring of a >768-char line): each of the ten live rows (FR53, the FR62 coverage-table row, EC5, members 11, 12, 16, 17, the Reza leg, FR4/`--intent-contains`, the teardown anomaly) carries `CLOSED 2026-11-01 by 20-3c` with its witness and the commit (`@ffd19698` for L1 work; L2 rows say "landing 2 (commit a41a143f, working tree base 3ea4bbb9)" — **the hash does not exist yet and must be appended by whoever commits**); the eleven shipped rows that still carried the `Owner: 20-3c` cue (members 1, 2, 3, 4, 6, 7, 8, 9, 10, 13 and the `root_shutdown` pin-mismatch row) plus the member-14 row keep their CLOSED lines and only lose the cue (now "Routed to … ; shipped and CLOSED before this story"); the member-8 and member-14 rows gain their AC11 witness lines. Member 5 (D:932, now `:944`) was already closed by 16-6 and passes unedited. Three words in unrelated rows had been masked by the old cue and surfaced once it was gone (`stale-owner sweep` ×2, `names the designed recovery`) and were re-worded neutrally. New rows: the precheck successor-version gap, owner-less halts outside the swap gate (unreachable in production: `invoke_halt` always inserts with metadata), and the `Full` reason. AC1.3: no shipped row's CLOSED line rewritten, no global owner replace.
- **Failing-before and plants** (`landing-2/plants.md`, logs, cut diffs + sha256 `plant-cuts.sha256`): the five kernel files reset to HEAD bytes ⇒ all 12 new or rewritten L2 integration vectors RED (`P0`, 12.4 s: the FR53 witness, seven `halt_carry_20_3c`, two AC3, two rewritten corpus vectors), the old-tree controls (`audit_sink_truth_16_5`, the 18-5 delivered-frame test, the corpus scenario wrapper) green; named plants: P1 (AC2.5, restore `drain_for_spirit`) 7.5 s — FR53 witness, carry vectors (b)–(e) and the two seeded corpus vectors (`swap_continuity_with_seeded_halts_carries_them_instead_of_draining`, `chain_walk_carries_every_real_pending_halt_at_every_hop_with_zero_dropped`) red; the isolation-corpus wrapper `halt_signal_observation_corpus_exercises_swap_continuity_wrapper` stays green (empty registry); P2 (precheck reads the global set) 8.0 s — (a), (b) red; P3 (monitor reads the pending set) 7.2 s — `resolving_the_carried_halt…` red; P4 (AC3.4, no-runtime arm a no-op) 7.6 s — `a_429_with_no_runtime…` red on "exactly one kind-23 row"; P5 (delivery error discarded) 11.8 s — `a_429_whose_spawned_delivery_fails…` red on "the frame was lost silently". All five restored byte-identically (copy-back, `cmp`, sha256, `touch`). Hosted: the same five cut diffs re-apply in job `kernel-l2` of `ci/ci-20-3c-verify.yml` after `sha256sum -c plant-cuts.sha256` (not run, not pushed).
- **kloc (tokei, after `cargo fmt --all`, `kloc-check --json` rc 0, `over_budget []`) vs ceilings:** kernel physical **26801** (ceiling 26817; 26793 + 8) / tokei **21011** (21044; 20992 + 19; the prototype's R was +12 / +26, measured here +8 / +19); maos-capability **1433** (1434); aggregate **188804** (189478), hardfail 188805. Unchanged and inside their ceilings: maos-bin 33195 (33559), maos-domain 10081 (10097), maos-cli 6396 (6446), maos-audit 7721 (7797), maos-shell 608 (626), maos-iac 8270 (8345), xtask 42860 (42901). Re-pinned exactly: `xtask/kernel-core-baseline.toml` `src_lines = 26801` stays on line 505, HISTORY row `20-3c landing 2` directly below it, `[kernel_src]` block from the production `check-kernel-baseline --emit-pin` (set hash `c023d0d7…`, 95 files; changed files `halt/mod.rs`, `hot_swap/{coordinator,post_swap_monitor,precheck}.rs`, `inference/mod.rs`); `xtask/kloc.toml` kernel 21011 / capability 1433 / `_aggregate_hardfail` 188805 with first HISTORY segment `20-3c-audit-truth-and-halt-receipt-residuals landing 2 (20-3c landing 2)`; xtask test literals `kernel_pin_content_hash_16_0.rs` (26801 ×4), `recovery_lane_ceiling_rule.rs` (`RATIFIED_AT_EASING` 21_011). Public surface unmoved (`pub(crate)` helper; `check-service-boundary` rc 0, surface JSON and `kernel-api-classes.toml` untouched). ADR-069: 21011 is 989 under the 22000 alarm; no ENVELOPE ALARM row needed.
- **Verification (L2 tree):** `cargo fmt --all` rc 0; `cargo build --locked --workspace --all-targets` rc 0; `cargo test --locked -p maos-kernel-core --lib` rc 0, 253 passed; touched integration binaries `routed_halt_swap_20_3b` 1/1, `halt_carry_20_3c` 7/7, `hot_swap_halt_continuity_corpus_integration` 3/3, `context_budget_gate_18_5` 6/6, `audit_sink_truth_16_5` 2/2; `cargo nextest run --workspace --profile ci --test-threads 1` 5476/5476 passed (138 skipped), rc 0, 763 s wall (615 s nextest summary); default profile once 5476/5476, rc 0, 46 s; `check-test-budget --junit` on the serial junit rc 0 (584 binaries, no findings); `cargo clippy --workspace --all-targets --message-format=json` rc 0 → `clippy-ratchet` rc 0 at **1416 occurrences / 883 source-distinct** (hosted baseline 1456 / 904 unchanged; identical to the L1 count, so L2 added no warning); `cargo deny check` rc 0 (advisories, bans, licenses, sources ok); gates rc 0 on the final tree: kloc-check, check-kernel-baseline, check-service-boundary, check-env-contract, check-serde-error-handling, check-composition-root-completeness, check-j1-loopback-delegation, check-dev-record-completeness, check-epic-close-coherence, check-review-findings-resolved, check-test-cost, coverage-matrix, check-coverage-matrix-completeness, invariant-lock (rc 1 first: touching `I14.md` requires a `tests/coverage-matrix.yaml` touch in the same diff; FR53 row note added, then rc 0). Logs: `landing-2/*.log`, `gates.jsonl`.
- **Unverified / not run:** every hosted/T3 leg (`ci-20-3c-verify.yml` jobs `t3-legs` and `kernel-l2`, nothing pushed); a hosted-toolchain clippy count; `kernel-l2` and `t3-legs` were never run (the workflow file is local-only evidence, not under `.github/workflows`), so the plant proof is the local run in `landing-2/plants.md`; the post-swap-monitor FAST window is exercised with `MAOS_AUTO_REVERT_FAST` set in-process, now serialized: every `halt_carry_20_3c.rs` vector that completes a swap holds the `MONITOR_WINDOW` lock and sets or clears the variable inside it (landing 2 review T-6). The prescribed `prepush` result is the next bullet.
- **Prescribed prepush (run after every code, pin and doc edit above; only this record's own bullets followed):** `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/home/lunarpulse/dev_ws/xtask MAOS_17_2_CGROUP_MODE=undelegated $CARGO_TARGET_DIR/debug/xtask prepush --json` rc 0, 429 s wall (`landing-2/prepush.log`): 92 records — 83 pass, 8 skip (`docs:build`, `docs:routes`, `cargo-deny` (run separately above, rc 0), five `ci-pins:marker`), 1 advisory. The advisory is `test-budget` on prepush's own default-profile (parallel) junit: three binaries over their Σ-per-test budget under load (`maos-bin::hook_pump_17_4` 30.873 s vs 30 s and two more, text in the log); the prescribed `check-test-budget` on the SERIAL junit is rc 0 with no finding (above). Embedded in it and green: `workspace-nextest` (exit 0, 174 s), `kernel-core-lib`, `workspace-doc`, `fmt`, the mirrored gates. SKIPs recorded by the run itself include strace-based otel air-gap canaries (`strace` is absent on this host). `intent-lineage-coverage-report.md` was not modified by any run.
- **Landing 2 review patches (Patch203c2; test, fixture and doc only, no production line changed: kernel 26801 / tokei 21011 / capability 1433 unchanged).** (1) `deferred-work.md` teardown-anomaly row: closed for the consumer-handle half only; the teardown-ORDER half is open (hosted `live_escape_anomaly_17_10` leg, never run). (2) Halt-continuity corpus: scenarios 002/003/011/012 now expect `SafeMigrated{n}` (carry), scenario 006 expects `EHaltContinuityViolation{orphan_count:1}` (an empty successor list; the gate never returns `MissingHaltProtocolCompatibility` from the swap); `hot_swap_halt_continuity_test.rs` `every_scenario_expected_outcome_matches_the_gate_verdict` seeds each scenario and compares `expected_outcome` to the real gate verdict; the isolation-corpus wrapper (`hot_swap_halt_continuity_corpus_integration.rs`) asserts the expected variant instead of mapping `SafeMigrated`/`Violation` to `SafeDrained`, and its comment claims only what an empty registry proves. No sha pin or manifest covers the corpus (grep). (3) `ci-20-3c-verify.yml`: each plant leg asserts its named witnesses on `FAILED` lines (`--no-fail-fast`, captured log); the header says `kernel-l2`/`t3-legs` were never run. (4) `plants.md` P1 names the red corpus vectors; `halt_carry_20_3c.rs` serializes the monitor-window vectors with a lock; `audit_sink_truth_16_5.rs` doc and the "nine sites" wording corrected; `context_budget_gate_18_5.rs` `a_provider_429_journals_and_delivers…` takes `RATE_LIMITED_HEALTH`. (5) New ownerless-and-open rows in `deferred-work.md` (Security F1/F3/F4/F5, BMAD #7/#10; F2/BMAD #3 cited on the existing row).

### File List

- Creation: this story file; `20-3c-…-evidence/creation/*`; `sprint-status.yaml:300`; `epics/epic-20-ship-it-w5.md:75,180,182,192,194` (cite corrections).
- Landing 1 (code): `crates/maos-bin/src/{main,supervision,delegation,escape_detector_consumer,admission,env_contract}.rs`, `crates/maos-domain/src/operator_door.rs`, `crates/maos-audit/src/{lib,fr4_classifier}.rs`, `crates/maos-shell/src/lib.rs`, `crates/maos-cli/src/{cli,subcommands}.rs`, `xtask/src/{check_reza_production_path,evidence_ledger,check_rto,check_rto_gate}.rs`, `xtask/kloc.toml`.
- Landing 1 (tests/docs): new `crates/maos-bin/tests/{one_shot_drain_20_3c,endpoint_lease_20_3c,audit_query_20_3c,abi_extension_journal_20_3c}.rs`, `crates/maos-a2a-tcp/tests/t_20_3c_response_phase.rs`, `xtask/tests/gate_evidence_names_20_3c.rs`; edited `delegation_leg_1a`, `repin_hold_19_7`, `two_host_delegation_2b`, `escape_anomaly_tail_17_10`, `live_escape_anomaly_17_10`, `shell_halt_repl_16_2`, `fr4_classifier_16_2`, `xtask/tests/invariant_claims_20_3b`; `docs/adr/ADR-045-…md` §4, `tests/coverage-matrix.yaml` FR62, `tests/integration/audit_9_1_headline_smoke.sh`; evidence `…-evidence/ci/{ci-20-3c-verify.yml,t3-two-host-delegation-2b.diff}`.
- Landing 2 (code): `crates/maos-kernel-core/src/{halt/mod.rs, hot_swap/{precheck,coordinator,post_swap_monitor}.rs, inference/mod.rs}`, `crates/maos-capability/src/cap_audit/mod.rs`, `xtask/src/gen_isolation_corpus.rs` (a stale comment only; plain comment lines, kloc-neutral).
- Landing 2 (pins): `xtask/kernel-core-baseline.toml` (`src_lines` 26801 on line 505, HISTORY row, production `--emit-pin` block), `xtask/kloc.toml` (kernel 21011, capability 1433, aggregate hardfail 188805), `xtask/tests/{kernel_pin_content_hash_16_0,recovery_lane_ceiling_rule}.rs` (pin literals).
- Landing 2 (tests/docs): new `crates/maos-kernel-core/tests/halt_carry_20_3c.rs`; edited `routed_halt_swap_20_3b.rs` (un-ignored), `hot_swap_halt_continuity_corpus_integration.rs`, `context_budget_gate_18_5.rs` (two AC3 vectors), `audit_sink_truth_16_5.rs` (nine `AuditDropSite` variants, counter mapping only), `crates/maos-eval/tests/hsis_runner.rs` (owned halts), `xtask/tests/invariant_claims_20_3b.rs` (FR53 witness must stay un-ignored); `docs/invariants/{I14.md,README.md}`, `tests/coverage-matrix.yaml` (FR53 row), `_bmad-output/implementation-artifacts/deferred-work.md` (21 owner rows + 3 landing-2 rows), this story file; evidence `…-evidence/landing-2/**` and `…-evidence/ci/ci-20-3c-verify.yml` (job `kernel-l2`).

### Change Log

- 2026-10-31: Story created at `3341f9e4` (ready-for-dev; D05, D07, D10, D13 and D-20-3c-01…11 PROPOSED). Inventory measured: 13 members shipped (8 and 14 without witnesses), member 5 shipped by 16-6 with a stale ledger owner, members 11/12/16/17 and eight received items live; member 11 measured on the HEAD binary (4/4 arms time out, exit 0), member 16 reproduced twice; kernel priced by prototype (R = K1p + E3 +12 physical / +26 tokei; R′ −1/+13; K2 +15/+14); member 17 maos-bin +2; envelope and operator-ruling list filed; E20 §20-3c cites corrected (member 11 target, ledger anchors, AC7 sites, kernel pin cite).
- 2026-11-01: Operator rulings (asks `rows203c`, `grant203c`) recorded: every D-row ruled as recommended (D05 A; D07 A at 20-9; D10 A; D13 A; D-20-3c-01…11 per the Decisions table). Kernel grant R = K1p + E3, ceiling 26817 physical / 21044 tokei; per-crate ceilings in `kloc_grant`. FLAG-Winston record `20-3c-flag-winston-decision.md`. I14 owner → `20-9-spawned-form-lifecycle-hooks`. D:932 closed by 16-6 (measured at creation, AC1 item 2). Tracker lane `ops-rpo-rto-cadence-proof` (D-20-3c-11 B). Status → in-progress. T0 and T6–T9 unchecked; L1 then L2 landing not started; no pin, ceiling, code or test changed by this record.
- 2026-11-01: Landing 1 (L1, kernel-free) implemented and locally verified by Dev203c1: AC4–AC11 witnessed with red-before plants restored byte-identically; kloc pins re-set exactly inside the grants; records AC1 items and L2 (AC2, AC3, kernel pins, I14 witness line) remain owed. Not committed, not pushed.
- 2026-11-01: Landing 2 (L2, FLAG-Winston R = K1p + E3; AC2, AC3, AC1 close) implemented and locally verified by Dev203c2 against `3ea4bbb9`: the creation prototypes were re-cut (never compiled; not applied blindly), the FR53 witness un-ignored and green, the swap precheck and the real swap judge one carried set, a never-resolved-vs-lost correction in the post-swap monitor (found while carrying; see Completion Notes), EC5 journaled-or-counted, 21 `deferred-work.md` owner rows reconciled (the gate passes with `20-3c` flipped to `done` in a scratch copy; reds with one live row), I14 FR53 witness line, kernel pins re-set exactly inside the grant (26801 / 21011, grant 26817 / 21044), maos-capability 1433 (≤ 1434), aggregate 188804 (≤ 189478). No status flipped; not committed, not pushed.
