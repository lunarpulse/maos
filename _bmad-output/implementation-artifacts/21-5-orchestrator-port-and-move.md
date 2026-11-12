---
story_id: '21.5'
story_key: 21-5-orchestrator-port-and-move
created: 2026-11-12
context_commit: 0baa76fb
baseline_commit: 0baa76fb
security_sensitive: false
kernel_grant: "GRANTED for the KP shape as ruled (FLAG-Winston, negative; `21-5-flag-winston-decision.md`; ask `d21`, 2026-11-12, accepted as recommended: D-01 A, -02 A, -08 A, -11 A, -12 A); the SpiritSchedulerAdapter surface refresh and the 21 symbol removals execute at L2 under ADR-070 (D-08 A): KP measured at 0baa76fb from 27040 physical / 95 files / 21201 tokei: −1041 / −827 → 25999 / 90 / 20374. Kernel STOP for this story: Δ above −817 tokei or −1028 physical, or a change to any kernel file other than lib.rs, scheduler/scheduler_loop.rs, isolation/mod.rs (doc) and the five deleted orchestrator files. Binding after re-base onto the then-landed pins (20-9 L2, 20-13 KT3, 20-14 corrections)."
kloc_grant: "RULED (D-21-5-10 A, -12 A; ask `d21`, 2026-11-12, accepted as recommended). Creation-priced, recommended options, ceilings re-base onto exact pins at landing: maos-kernel-core exact measured (lowered; 20374 on the creation base); maos-orchestrator-buffer exact measured (829 on the creation base; row 917 today); maos-domain ≤ 10223 (+3 measured ×2); xtask ≤ 45381 (+1 ×2); maos-bin 38549 (0 measured); aggregate ≤ 202867 (+6 ×2), hardfail = measured + 1. Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-70`): a kernel FLAG-Winston row and a public kernel surface removal. Blind and Edge on opus-5.5, Acceptance on opus-5.5, Test Infrastructure on glm-5.3; a Security reading only if D-21-5-14 A. Creation anthropic/claude-opus-5-5."
---

# Story 21.5: Orchestrator port and move — the FR20 buffer leaves the kernel

Status: done

<!-- Created 2026-11-12 at 0baa76fb (`git log -1 -- crates xtask` = edeedd25; docs-only commits since) by bmad-create-story under the lighter measured creation contract (Create215). Pulled forward by operator ruling `s21` (2026-11-12, `planning-artifacts/decisions-2026-11-12-acceleration.md:34`): 21-5 follows 20-9 as a kernel story. Shared kernel STOP 22000 = ADR-069 alarm (`lim` = stop22, `:36`); envelope 23000. D-21-5-01…14 were PROPOSED at creation and are RULED (ask `d21`, 2026-11-12, accepted as recommended; see Decisions). A dev agent was editing the tree and tracker during creation, so only new files were written; every edit to an existing file is listed in `creation/records-owed.md`. Cites re-read at HEAD on a `git archive` copy (`creation/cites-0baa76fb.md`); kernel, kloc and surface figures by the prebuilt gate binary on reverted prototypes (`creation/measurements.md`). -->

**Creation rationale — ready-for-dev at filing, gated on rulings.** Every AC is hermetic (no network, no key, no
human). The kernel row is negative and measured; all fourteen D-rows are RULED (ask `d21`, 2026-11-12, accepted as recommended) and it binds once the dev re-bases
onto the then-landed pins. L1 needs no kernel line; L2 is the kernel move, last.

> **The capability:** *the Orchestrator instruction buffer (FR20), its journal helpers and the parked FR54 gateway
> host leave `maos-kernel-core` for a leaf crate the composition root owns; the kernel keeps one domain-port call on
> resume, and a gate refuses any path back.* The operator sees no behaviour change; the kernel row drops by about 827
> tokei, 1 626 below the ADR-069 alarm.

**Closes:**
- PRD FR20 drift: "NOT kernel-buffered — Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives"
  (`prd/functional-requirements.md:54`); at HEAD the buffer is `crates/maos-kernel-core/src/orchestrator/buffer.rs`.
- Architecture §4.0.7 "The kernel does NOT embed an orchestration policy" (`architecture-maos-minimal-opus/4-kernel-design.md:180,186`).
- Epic-21 §21-5 (E21:90-94) as corrected (`creation/records-owed.md` §A) and the ADR-069 relief valve
  (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:42-45,56`).
- D-J (PF-R2:59): the named leaf-crate destination. D-J's ordering "after 21-4" is superseded by `s21` (D-21-5-04).

**Does NOT close, and says so:**
- FR20 frame-level delivery of buffered instructions on resume ("deferred to Story 5.4", `scheduler_loop.rs:496-498`):
  the replay stays an identity on the queue (D-21-5-07).
- FR54 gateway sub-modules (parked to v2.5, `functional-requirements.md:68`): the dispatcher moves (D-21-5-01 A) but
  stays unwired; real handles remain stubs (`gateway_dispatcher.rs:411-531`).
- 21-4's instrument unification and env registry (D-21-5-04): this story re-pins both instruments as they are.
- The FR21 no-prior-distillate exemption routed here by `r19_6_d04` (`deferred-work.md:1463`) — D-21-5-14 B RULED: the row is re-routed out of this story (see AC6 and `deferred-work.md:1463`).

## Story

As **the operator holding the kernel row at 21 201 of a 22 000 alarm**,
I want **the Orchestrator buffer and the parked gateway host moved out of `maos-kernel-core` behind a one-method domain
port, with the moved lines removed from the kernel ceiling**,
so that **the kernel stops embedding orchestration state FR20 says it must not hold, and the next kernel grants draw on
about 1 600 lines of real distance to the alarm instead of 800**.

**Contract:**
- Epic §21-5 (E21:90-94), corrected; create-story may tighten, not add uncited ACs: AC1 is the epic's command; AC2/AC3
  its move and re-pin; AC4/AC5 the kernel-free port and equivalence the move needs; AC6 is the routed row (D-14).
- Rulings `s21`, `lim`, `prepaid`, `nextest`, `repin` (`decisions-2026-11-12-acceleration.md:25-36`); ADR-069; ADR-066;
  architecture §15.5 Decision 2(a) (`15-full-spectrum-v2-2.md:100`); proof contract `docs/testing/proof-contract.md:10-31,33-71`.
- E21 confidence rules 4, 6, 8–11 (E21:30-32).

## Inventory at HEAD — measured, not assumed

Method: every cite read at `0baa76fb` (`creation/cites-0baa76fb.md` C1–C20, K1–K26); gates by the prebuilt binary on
`git archive` copies (`creation/measurements.md`); no crate source changed.

| # | Item | Disposition | Code at HEAD | Consumers | AC |
|---|---|---|---|---|---|
| 1 | FR20 buffer (`OrchestratorBuffer`, cap 32, FIFO, `QueueFull`) | **in the kernel**, against FR20's letter | `orchestrator/buffer.rs:20-95` (170 tokei incl. tests `:103-220`) | door queue/recall/status `operator_door.rs:2040-2095,2852`; root drain `orchestrator_root.rs:693-708`; `spirits/orchestrator/tests/{orchestrator_buffer,fixtures_pin}.rs` | AC2 |
| 2 | Registry (`OrchestratorBufferRegistry`, DashMap) | in the kernel | `orchestrator/registry.rs:19-45` (65) | ctor `main.rs:2355-2356`; into scheduler `:3037`, door `:3617`, root `:5491`, status `:6091`; door `:518,573`; bin test support (C7) | AC2 |
| 3 | Kernel's own use | **one site** | `scheduler_loop.rs:89` field, `:113` param, `:141` init, **`:490-505` resume replay** (drain + re-enqueue) | `SpiritSchedulerAdapter::resume`; only `main.rs:3037` passes `Some`; 37 callers pass `None` (K26) | AC2, AC5 |
| 4 | Journal helpers ×3 | in the kernel, kernel never calls them | `orchestrator/mod.rs:38-97` over `maos_iac::transparency_log` | `operator_door.rs:1031,1142,2028,2178` | AC2, AC5 |
| 5 | FR54 gateway host | in the kernel, **no production caller** | `gateway_dispatcher.rs` (441), `echo_gateway.rs` (74) | kernel tests `gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs` only | AC2 (D-01) |
| 6 | Public kernel surface | 21 triples + 1 struct hash | `kernel-surface-v0.1-beta.json` (K12); classes `kernel-api-classes.toml:367-375` | `check-service-boundary` | AC7 (D-08) |
| 7 | Kernel → orchestrator dependency control | **absent** | `check_dependency_closure.rs:42-67` forbids `maos-egress`, not this crate | — | AC3 (D-09) |
| 8 | Direct witness of the resume replay | **absent** | no test passes a registry into the scheduler (K26) | — | AC2.4 |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-0baa76fb.md`. Owed to E21 (not applied: `creation/records-owed.md` §A).

| Premise (epic / preflight / tracker) | Fact at `0baa76fb` and consequence |
|---|---|
| Kernel pin 24472/24474, kloc `:195` 18933 (E21:26, :94) | **27040 physical / 95 files / 21201 tokei** (`kernel-core-baseline.toml:505`, `kloc.toml:264`), zero headroom (C2). |
| `scheduler_loop.rs:89,111`; "+2–6 port-type lines" (E21:26) | `:89,113` + the replay `:490-505`; KP measures the file at **−12 physical / −9 tokei**, not +2–6 (C3). |
| maos-bin users `main.rs:1995,5696,5788,5859` (E21:94) | ctor `main.rs:2355-2356`; the journal calls live in the door since 16-1: `operator_door.rs:2028,1031,1142,2178` (C4). |
| `nfr_perf_4_pause_resume_latency.rs` (E21:94) | renamed `lifecycle_journal_write_latency.rs` by `75e1e493` (C5). |
| `check_cohort_mesh.rs:674` (E21:94) | `:703`; the gate builds maos-bin (`:26,48,157`) (C8). |
| maos-domain +30–60 (E21:28, :46, :92) | **+3** for a one-method port (KP, C13). |
| "re-pinned in tokei units because 21-4 lands first" (E21:26, :119; D-J) | `s21` puts 21-5 after 20-9; 21-4 is backlog. Every Epic-20 kernel landing re-pins both instruments in their current units (C14) → D-21-5-04. |
| Epic closes §4.0.7 only (E21:92) | FR20's "NOT kernel-buffered" is the PRD clause the move closes (C17); FR54's "kernel hosts the lifecycle" bears on the gateway half (C18) → D-21-5-01. |
| `deferred-work.md:1463` cites maos-iac "exact 8270, `kloc.toml:406`" | `kloc.toml:406` = **8770**; the row's owner is this key by `r19_6_d04` (C19) → D-21-5-14. |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`,
`check-service-boundary --json`) from `git archive 0baa76fb` copy roots; the untouched copy reproduces the tree
exactly (27040 / 95 files / set hash `5551f890…`; kernel 21201 tokei; aggregate 202855). Prototypes reverted (copies
deleted); diffs in `creation/`.

**Kernel (◆ FLAG-Winston, negative; `21-5-flag-winston-decision.md`, GRANTED for the KP shape as ruled by ask `d21`, 2026-11-12):**

| Shape | Kernel physical | Kernel tokei | Kernel files | New crate (tokei) | Other | Diff |
|---|---:|---:|---|---:|---|---|
| **KP** domain port, whole module (D-02 A, D-01 A, **recommended**) | **−1041 (25999)** | **−827 (20374)** | 95 → 90; `lib.rs`, `scheduler_loop.rs`, `isolation/mod.rs` (doc) | **829** | domain +3; xtask +1 (D-09 A); bin 0 | `kp-domain-port-leaf-move.diff` |
| KB kernel depends on the leaf (D-02 B) | −1029 (26011) | −818 (20383) | 95 → 90 | 817 | — ; D-09's control impossible | `kb-kernel-depends-on-leaf.diff` |
| KD replay dropped, ctor param removed (D-02 C) | −1049 (25991) | −833 (20368) | 95 → 90 | 817 | bin −1; 37 callers lose an argument | `kd-drop-kernel-replay.diff` |
| KH port, buffer half only (D-01 B) | −407 (26633) | −304 (20897) | 95 → 93 | 308 | domain +3 | `kh-buffer-half-only.diff` |

**ADR-069 room (STOP = alarm 22000; envelope 23000):** HEAD 21201 → 799 to the alarm. KP on HEAD → **20374: 1626 to the
alarm, 2626 to the envelope**. After the owed 20-9 R (+21) and 20-13 KT3 (±1), about 20395 → about 1605
[INFERENCE: not landed; no file overlap per their records]. No shape crosses 22000; no `ENVELOPE ALARM` text is owed.
At the close of creation the working tree already carried Dev2014d's uncommitted 20-14 corrections at **27043 / 21203**
(`kernel-core-baseline.toml:505`, `kloc.toml:264`, not committed, not measured here): the dev re-bases at T0.

**kloc forecast** (tokei; HEAD = every pin; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · module out, port call, `lib.rs` | **−827** (physical −1041) | prototype KP |
| maos-orchestrator-buffer · moved module + port impl | **+829** (src physical 1046) | KP |
| maos-orchestrator-buffer · atomic replay (D-07 B) | −5…+5 [INFERENCE] | replaces the 12-line moved body |
| maos-domain · `OrchestratorResumePort` | **+3** | KP |
| maos-bin · path re-point, port coercion at `main.rs:3037` | **0** | KP |
| xtask · `FORBIDDEN_CRATES` entry (D-09 A) | **+1** | KP+X |
| spirits/orchestrator | ungoverned | `kloc_check.rs` excludes `spirits` |
| tests (3 moved, new witnesses), docs, ADR-070, JSON/TOML records | uncharged | `-e tests`; non-Rust |
| **aggregate** | **+6** (202855 → 202861; hardfail 202862) | KP+X |

**Envelope (D-21-5-12 A; ×2 measured, D02 A; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling | Operator ruling needed |
|---|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 27040 / 21201 | −1041 / −827 | **exact measured, lowered**; STOP if Δ > −1028 / −817 | **yes — FLAG-Winston** |
| maos-orchestrator-buffer | 0 (row 917) | 829 | **exact measured** (D-10 A) or 917 (D-10 B) | yes |
| maos-domain | 10217 | +3 | **10223** | yes |
| xtask | 45379 | +1 | **45381** | yes |
| maos-bin | 38549 | 0 | **38549** | no (0) |
| aggregate | 202855 | +6 | **202867**; hardfail = measured + 1 | yes |

Each ceiling re-bases onto the exact pins at this story's landings; each figure lands exact after `cargo fmt --all`,
with a HISTORY row whose first segment cites this key with `->` (D11 A).

**Operator-ruling list:** (1) FLAG-Winston row KP (D-01, D-02, D-11, D-12); (2) surface removal sanction (D-08);
(3) kloc rows (D-10, D-12); (4) D-03…07, D-09, D-13, D-14.

## Decisions — RULED (ask `d21`, 2026-11-12, accepted as recommended; every row RULED: D01 A, D02 A, D03 A, D04 A, D05 A, D06 B, D07 B, D08 A, D09 A, D10 A, D11 A, D12 A, D13 A, D14 B)

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-21-5-01** | What moves (◆) | **A** the whole module — buffer, registry, journal fns, FR54 gateway host (KP: −1041/−827), with dated notes on FR20 (closed), FR54 ("kernel hosts the lifecycle" → contracts = `maos-spirit-abi::gateway` traits, which stay) and §4.0.7 · **B** buffer half only; the gateway host stays at `maos_kernel_core::orchestrator` (KH: −407/−304) · **C** whole module into two crates (`maos-orchestrator-buffer`, `maos-gateway-host`): kernel side = KP, +1 workspace member and kloc row | **A.** The gateway host has no production caller (grep `crates/maos-bin` = 0) and FR54 is parked to v2.5; it is 515 of the 817 tokei. B keeps FR54's letter but forfeits 63 % of the relief `s21` asked for. C adds a crate for code with no caller. A's FR54 reading is an interpretation, hence the dated PRD note. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-02** | Port shape (◆) | **A** KP: `maos_domain::orchestrator::OrchestratorResumePort { fn replay_pending(&self, spirit_id: &str) }`; kernel holds `Option<Arc<dyn …>>`; the leaf registry implements it · **B** KB: the kernel depends on the leaf, concrete type · **C** KD: the kernel drops the replay and the ctor param | **A.** FR20 and §4.0.7 ask that the kernel not hold the buffer; only A stops the kernel linking the crate (KP closure: 0 occurrences, `cargo tree --edges all`), which D-09's control needs. B is the maos-iac/maos-capability extraction shape (`K/iac.rs:13`), where the kernel still calls the buffer. C is a behaviour change and edits 37 callers. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-03** | Leaf crate name | **A** `maos-orchestrator-buffer` (D-J; kloc row `kloc.toml:684`; E21) · **B** `maos-orchestration` (covers the gateway half; row renamed) | **A.** The name, row and D-J record exist; under D-01 A the gateway half's final home is re-decided when FR54 is un-parked (v2.5). | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-04** | Must 21-4 precede (D-J's order) | **A** no: re-pin both instruments in their current units (physical `src_lines` + file set + hashes; tokei kloc row), as every Epic-20 kernel landing does; 21-4 re-measures at its own commit · **B** yes: wait for 21-4 (backlog; xtask +150–350 at zero headroom) · **C** pull only 21-4 AC1 (instrument) ahead | **A.** D-J ordered 21-4 first only to fix the re-pin's unit (SN21:146); both units are re-pinned today (`kloc.toml:264` 20-14 L2). Running first requires: both pins exact in L2; the xtask constants (`kernel_pin_content_hash_16_0.rs:34,820,838,847,858`, `recovery_lane_ceiling_rule.rs:279`); E21:26/:119 corrected; 21-4 AC1's tokei value re-measured at its commit (it already says so). The orchestrator files read no env var (grep 0), so 21-4's scan is unaffected; 21-4 AC4 deletes only the three named phantom rows. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-05** | Landing split | **A** L1 kernel-free: the domain port, equivalence goldens recorded against the unmoved kernel, PRD/architecture notes; L2 kernel last: the move, pins, surface · **B** one landing | **A.** Kernel-last precedent (20-1b, 20-14); L1 proves the goldens on the unmoved kernel in a hosted run, so L2's equivalence claim compares like with like. L1's trait has no implementor until L2 (same story). | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-06** | Equivalence witnesses | **A** unit: leaf replay vectors + a kernel recording-port vector, plus the existing suites · **B** A + a golden of the three journal rows and the buffer/registry semantics, recorded in L1 against `maos_kernel_core::orchestrator` and re-run in L2 against the leaf with only the import changed · **C** B + the T3 scene `orchestrator_queue_19_2::a_paused_orchestrator_holds_dispatch_until_resume` named as a required hosted leg | **B.** No test passes a registry into the scheduler today (K26); the journal rows are audit rows (`ApprovalDecision` fields `actor, target, capability, intent, decision, reasoning`, `mod.rs:44-56`). The T3 scene is gated on a T3 substrate (`require_t3`, `orchestrator_queue_19_2.rs:789`); whether the hosted runner executes it is not measured here [INFERENCE], so C would add a runner dependency. | **RULED** — option **B** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-07** | The resume replay's race | **A** move verbatim (two lock scopes; a concurrent enqueue can land ahead of older items) · **B** replay under one lock in the leaf (observably identical in sequential use; the window closes) · **C** drop the replay (= D-02 C) | **B.** Rule 10(a): the item is inside this story's blast radius (the body moves); B changes no sequential result, so D-06's goldens still hold; the eprintln path becomes unreachable and goes. Moot under D-02 C. | **RULED** — option **B** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-08** | Sanction for removing 21 public kernel symbols | **A** ADR-070 "sanctioned removal of `maos_kernel_core::orchestrator`" in ADR-066's form; baseline edited surgically (401 → 380); nine class rows deleted; the `SpiritSchedulerAdapter` refresh under `prepaid` (2a) · **B** the ruling recorded in the FLAG-Winston record only | **A.** `check-service-boundary` treats a removed triple as an NFR-Test-2 violation (`check_service_boundary.rs:219-232`); ADR-066 (`:22-29,53-61`) set that a removal is sanctioned by an ADR because the JSON cannot hold the note; `prepaid` (2a) names refreshes, not removals (`decisions…:28-31`). | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-09** | Rule-11 control: the kernel never re-acquires the buffer | **A** `maos-orchestrator-buffer` in `FORBIDDEN_CRATES` (`check_dependency_closure.rs:42-64`, xtask +1; CI `discipline.yml:4315-4327`) + an uncharged structural test · **B** the structural test only · **C** none | **A.** The closure gate is wired and runs in CI (rule 11 a/b); the 17-1 `maos-egress` entry (`:63`) is the precedent. Impossible under D-02 B. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-10** | kloc row of the new crate | **A** exact measured (829 on the creation base), zero headroom · **B** keep 917 (15-2 move rule, `kloc.toml:682-683`) | **A.** §15.5 Decision 2(a) lowers ceilings when lines move; D11 A exact pins; 88 free lines in an orchestration crate would be unattributed growth room. B is the ratified 15-2 figure. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-11** | Kernel pins and the ADR-069 record (◆) | **A** both pins exact; kloc ceiling LOWERED by the moved amount; `RATIFIED_AT_EASING` lowered; HISTORY `ADR-069 RELIEF VALVE (21-5, ruling s21)` with before/after, alarm distance, envelope · **B** baseline re-pinned, kloc ceiling kept at 21201 | **A.** ADR-069 Decision 2 (zero headroom; the envelope is not a reservation) and §15.5 2(a); B leaves 827 lines of grant-free room. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-12** | Envelope | **A** ×2 measured on growing crates (domain ≤ 10223, xtask ≤ 45381, aggregate ≤ 202867), bin 0, kernel exact with the STOP above · **B** exact measured only | **A.** D02 A; the review-correction precedent of 20-14 (overdrafts). | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-13** | Key and epic membership (`s21`: "to be decided by story creation") | **A** keep `21-5-orchestrator-port-and-move` in Epic 21, sequenced after 20-9; the ruling records that `s21` waives the epic-20-retrospective precondition (S:409) for this key only; epic-21 → in-progress · **B** re-key `20-17-orchestrator-port-and-move` in Epic 20 (the STOP ledger's epic) · **C** keep the key, account it in the Epic-20 kernel ledger, epic-21 stays backlog | **A.** ADR-069:44 names the full key; ADR-069:56, `recovery_lane_ceiling_rule.rs:404` and `s21` cite `21-5`; B renames an ADR citation and the test message. C leaves an in-progress story inside a backlog epic. | **RULED** — option **A** (ask `d21`, 2026-11-12, accepted as recommended) |
| **D-21-5-14** | Received row `deferred-work.md:1463` (FR21 exemption keyed on the reassigned task_id; owner 21-5 by `r19_6_d04`) | **A** in scope as AC6, kernel-free (maos-iac + maos-bin), priced at T0, security reading · **B** re-route by a new ruling to a key the operator names (no open story touches `maos-iac/src/adapter/orchestrator_dispatch.rs`) | **B.** The move touches no maos-iac file (all four prototypes); the exemption relaxes an FR21 control (`orchestrator_dispatch.rs:63-123`) at a zero-headroom crate (8770) and needs its own design; carrying it couples the relief valve to a security change. The operator chose this owner in `r19_6_d04`, so moving it needs a ruling. | **RULED** — option **B** (ask `d21`, 2026-11-12, accepted as recommended) |

## Acceptance Criteria

*(Epic §21-5, corrected; clauses marked (D-…) assume the recommended option and change if the operator rules otherwise.
Every AC names a failing-before witness or an equivalence golden, and a plant. **local** = hermetic on the dev host;
**hosted** = the discipline run.)*

### L1 — kernel-free

#### AC4 — the port exists in the domain; the records say why (D-02 A, D-01 A)

1. `crates/maos-domain/src/orchestrator.rs` gains `pub trait OrchestratorResumePort: Send + Sync { fn replay_pending(&self, spirit_id: &str); }`
   beside `OrchestratorInstruction` (KP: +3 tokei). No `maos-domain` dependency is added.
2. Dated notes (no FR text deleted): FR20 (`functional-requirements.md:54`) "buffer moves out of the kernel by 21-5";
   FR54 (`:68`) "the lifecycle contracts are the `maos-spirit-abi::gateway` traits; the unwired dispatcher moves with
   21-5 (D-21-5-01)"; `4-kernel-design.md:186`.
3. **Plant (local):** none for a type; AC2.4 is the trait's witness.

#### AC5 — equivalence goldens recorded against the unmoved kernel (D-21-5-06 B)

1. NEW `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs` + fixture `crates/maos-bin/tests/fixtures/orchestrator_equivalence_21_5.json`:
   (a) the three journal fns against `TransparencyLogAdapter::open_in_memory` — the approval rows' `actor, target,
   capability, intent, decision, reasoning` equal the fixture; (b) buffer semantics — FIFO, `QueueFull(32)` at the 33rd,
   `recall_all_pending` order and drain, `pending_count`; registry `get` vs `get_or_create` (no buffer created by
   `get`); (c) the replay outcome on a three-item buffer (same order, count 3).
2. L1 imports `maos_kernel_core::orchestrator::*`; L2 changes only the import (diff of the test = the `use` lines).
3. Green at HEAD (this is the baseline, not a red); **plant (local):** change `journal_orchestrator_queue`'s reasoning
   format ⇒ (a) reds; append instead of push-back in `enqueue` ⇒ (b) reds.
4. **L1 outcome (2026-11-12, Dev215L1; `landing-1/evidence.md`):** the same file also holds the D-21-5-07 replay-race witness
   `a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue` (a producer racing 200 pause/resume cycles; full queue).
   It is RED on the unmoved two-lock replay (10 of 10 runs; each accepted producer enqueue is one refused re-enqueue = one lost
   queued instruction) and committed `#[ignore]`d at L1; L2 removes the attribute and the test must go green (that is AC2.2's
   failing-before/green-after). The diff of the file at L2 = the `use` lines + that one attribute. Plants run: reasoning format,
   push-front, replay drop-last, `get` creating a buffer (`landing-1/plants.md`; push-front does not red (c): the replay round trip
   of a reversed queue coincides, so (c) is planted by the replay drop).

### L2 — kernel last ◆ FLAG-Winston

#### AC2 — the move (D-01 A, D-02 A, D-03 A, D-07 B)

1. `crates/maos-kernel-core/src/orchestrator/` (5 files) leaves the kernel for NEW `crates/maos-orchestrator-buffer/`
   (`lib.rs` = old `mod.rs`; `crate::iac::transparency_log` → `maos_iac::transparency_log`); dependencies `dashmap`,
   `maos-attrs`, `maos-domain`, `maos-iac`, `maos-manifest`, `maos-spirit-abi`, `thiserror` (KP closure). Kernel changes
   only: `lib.rs:37` deleted; `scheduler_loop.rs:89,113` typed `Option<Arc<dyn maos_domain::orchestrator::OrchestratorResumePort>>`,
   `:490-505` → `if let Some(ref port) = self.orchestrator_registry { port.replay_pending(&scb.spirit_id); }`;
   `isolation/mod.rs:10-12` doc. Anything else under `crates/maos-kernel-core/src` is STOP.
2. The leaf's `OrchestratorBufferRegistry` implements the port: the replay under one lock (D-07 B), same order and
   count as HEAD in sequential use.
3. Re-points (paths only): `crates/maos-bin/src/{main.rs,operator_door.rs,orchestrator_root.rs}`, `tests/support/{shell_world_16_2,worker_revoke_root}.rs`;
   `main.rs:3037` passes the registry as the port; `spirits/orchestrator/tests/{fixtures_pin,orchestrator_buffer}.rs`
   and its dev-dependency; doc `spirits/orchestrator/src/lib.rs:40-47`. The kernel tests
   `gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs`, `lifecycle_journal_write_latency.rs` move to
   `crates/maos-orchestrator-buffer/tests/` (imports: `maos_capability::cap_tokens`, `maos_iac::transparency_log`).
   `Cargo.toml` members +1; `4-kernel-design.md:117` count 58 → 59.
4. **Failing-before (local):** NEW `crates/maos-kernel-core/tests/orchestrator_resume_port_21_5.rs` — a recording port
   passed to `SpiritSchedulerAdapter::new`; load, pause, resume ⇒ exactly one call with the Spirit id; a resume that
   fails (not paused) ⇒ no call. At HEAD the ctor takes a concrete registry and no test can observe the replay (K26).
   Leaf: `crates/maos-orchestrator-buffer/tests/resume_replay_21_5.rs` — three queued ⇒ replay ⇒ same FIFO, count 3;
   unknown Spirit ⇒ no buffer created.
5. **Plants (local):** delete the port call in `resume` ⇒ the kernel vector reds; make the leaf replay drop the last
   item ⇒ the leaf vector and AC5(c) red.

#### AC3 — the kernel cannot take the buffer back (D-21-5-09 A) ◆ control

1. `xtask/src/check_dependency_closure.rs` `FORBIDDEN_CRATES` gains `"maos-orchestrator-buffer"` (+1 xtask).
2. NEW `xtask/tests/orchestrator_out_of_kernel_21_5.rs` (uncharged): no `crates/maos-kernel-core/src/orchestrator/`;
   no `pub mod orchestrator` in kernel `lib.rs`; no `.rs` under `crates/`, `spirits/`, `xtask/src` names
   `maos_kernel_core::orchestrator`.
3. **Failing-before (measured at HEAD):** the directory exists and **15** occurrences in **10** files name the path.
4. **Plants (local):** add `maos-orchestrator-buffer` to `crates/maos-kernel-core/Cargo.toml` ⇒ `check-dependency-closure`
   reds; restore `pub mod orchestrator;` with a stub file ⇒ the structural test reds.

#### AC1 — the command (epic AC1)

1. **Command (local, final tree):** `cargo run -p xtask -- check-kernel-baseline && cargo run -p xtask -- check-cohort-mesh`
   exit 0, with the kernel at or below base − 1028 physical / base − 817 tokei (KP on the creation base:
   25999 / 20374) and 90 files.
2. Also green on the final tree: `kloc-check` (kernel row lowered, new row, domain, xtask, aggregate), `check-service-boundary`
   (AC7.2), `check-workspace-count` (59), `stability-matrix --check`, `check-loom`, `check-empty-kernel`,
   `check-dependency-closure`, `check-serde-error-handling`.
3. **Measured at HEAD:** 27040 / 21201 / 95 files; KP's untouched pins red `check-kernel-baseline`, `kloc-check`,
   `check-service-boundary`, `stability-matrix` exactly as listed in `creation/measurements.md`.
4. **Plant (local):** leave `orchestrator/registry.rs` in the kernel ⇒ `check-kernel-baseline` reds on file set and count.

#### AC7 — records

1. Pins exact in current units (D-04 A, D-11 A): `kernel-core-baseline.toml` (`src_lines`, file set 95 → 90,
   `set_hash`, three content hashes, five rows deleted, EOF HISTORY); `kloc.toml:264` lowered, `:684`, `:425`, `:339`,
   `:718`; HISTORY first segment cites this key with `->`; the kernel row says `ADR-069 RELIEF VALVE (21-5, ruling s21)`,
   before/after, alarm distance, envelope. Constants: `kernel_pin_content_hash_16_0.rs:34,820,838,847,858`,
   `recovery_lane_ceiling_rule.rs:279` (lowered, comment row), or the ruling-4 `repin` script if landed.
2. Surface (D-08 A): `kernel-surface-v0.1-beta.json` hand-edited — the 21 triples in `21-5-flag-winston-decision.md`
   deleted, `SpiritSchedulerAdapter` refreshed to the measured hash; `kernel-api-classes.toml:367-375` deleted;
   ADR-070 + `docs/adr/index.md`; `serde-error-allowlist.toml:733,736` deleted (stale).
3. `STABILITY.md` regenerated (58 → 59); `21-5-flag-winston-decision.md` gains the landing record; tracker; `deferred-work.md:1463` already re-routed per D-14 B (2026-11-12); revert `intent-lineage-coverage-report.md` if a run modifies it.

#### AC6 — VOID (D-21-5-14 B RULED, ask `d21`, 2026-11-12): the routed FR21 exemption is not in this story

1. As `deferred-work.md:1463` states: an FR21 exemption keyed on the reassigned task_id in
   `maos-iac/src/adapter/orchestrator_dispatch.rs:63-123`, so a no-prior-distillate section declared
   `reassign-to-replica` is re-dispatched; priced at T0 by prototype; L1; security reading. D-14 B is ruled: this AC is absent; the row is ownerless-and-open (re-route to a key the operator names; trigger: next maos-iac story), the operator named no key.

## Tasks / Subtasks

- [x] **T0 — Preflight.** (Satisfied by the creation-time measurements `creation/measurements.md`, the D-row rulings of 2026-11-12, and the L1/L2 pins at `257a0fd7`/`637721bf`; no separate preflight run was recorded.) Operator rulings on D-21-5-01…14 recorded in this file and the FLAG-Winston record; tracker
  → `in-progress`. Re-base onto the then-landed pins (20-9 L2, 20-13 KT3, 20-14 corrections); re-measure every cite
  this story touches (rule 9); run `creation/measurements.md` "Not measured here" 1–6. A figure above the ruled
  envelope, or a result contradicting a ruled option, is STOP + D-row.
- [x] **T1 — Port and notes (AC4).** (L1, 2026-11-12; `crates/maos-domain/src/orchestrator.rs`; PRD `:54,:68`, `4-kernel-design.md:186` dated notes.)
- [x] **T2 — Goldens (AC5)** against `maos_kernel_core::orchestrator`, plants. (L1: `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs` + fixture; plants `landing-1/plants.md`; plus the D-21-5-07 replay-race witness, red on the unmoved kernel.)
- [x] **T3 — L1 landing (D-05 A).** (L1 `257a0fd7`; `landing-1/evidence.md`; hosted run 43153299381 red only on the 1M erasure oracle, fixed by `5fe97eda` ruling `ac5`; the following run red only on the `ac4_v3b` ROOT_GRACE load flake, 20-7 family, dispositioned.) Exact domain pin; `check-kernel-baseline` unchanged; `kloc-check`,
  `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; hosted run on the final tree + plants.
- [x] **T4 — Kernel-free red witnesses first (AC2.4 leaf vector, AC3.2)**, then the move (AC2.1–2.3), the control
  (AC3.1), the kernel vector (AC2.4), plants (AC2.5, AC3.4, AC1.4). (L2, 2026-11-12, Dev215L2; `landing-2/evidence.md`.)
- [x] **T5 — Records and L2 landing (AC1, AC7).** (L2 `637721bf`; hosted discipline run 43175946640 green except the advisory `reproducible-build` disk-exhaustion event; reviews `41b937cc`.) Pins, constants, surface, classes, ADR-070, STABILITY, workspace
  count; `cargo test -p maos-kernel-core --lib`; focused suites; hosted run; FLAG-Winston landing record. (Local legs done 2026-11-12, Dev215L2; the hosted run and the review are open.)

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Domain (L1) | `crates/maos-domain/src/orchestrator.rs` (trait beside `OrchestratorInstruction`, after `:32`) | AC4 |
| Bin goldens (L1) | NEW `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs`, fixture | AC5 |
| Kernel ◆ (L2) | delete `crates/maos-kernel-core/src/orchestrator/*`; `src/lib.rs:37`; `src/scheduler/scheduler_loop.rs:89,113,490-505`; `src/isolation/mod.rs:10-12`; NEW `tests/orchestrator_resume_port_21_5.rs`; move three tests out | AC2, AC1 |
| Leaf (L2) | NEW `crates/maos-orchestrator-buffer/{Cargo.toml,src/{lib,buffer,registry,gateway_dispatcher,echo_gateway}.rs,tests/*}` | AC2 |
| Bin (L2) | `crates/maos-bin/Cargo.toml:55`; `src/main.rs:2355-2356,3037`; `src/operator_door.rs:518,573,1031,1142,2028,2178`; `src/orchestrator_root.rs:31,693`; `tests/support/shell_world_16_2.rs:23`, `worker_revoke_root.rs:270` | AC2 |
| Spirit (L2) | `spirits/orchestrator/Cargo.toml:21`; `tests/fixtures_pin.rs:6`, `tests/orchestrator_buffer.rs:8`; doc `src/lib.rs:40-47` | AC2 |
| xtask (L2) | `src/check_dependency_closure.rs:42-64`; NEW `tests/orchestrator_out_of_kernel_21_5.rs`; constants (AC7.1); `kernel-api-classes.toml:367-375`; `serde-error-allowlist.toml:733,736`; pins | AC3, AC7 |
| Records | `Cargo.toml`, `Cargo.lock`, `STABILITY.md`, `docs/ci-baselines/kernel-surface-v0.1-beta.json`, ADR-070, `4-kernel-design.md:117,186`, PRD `:54,68` | AC4, AC7 |

No new `maos-bin/src` file (so `SCANNED_SOURCE_FILES` in `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs` is
unchanged).

### Overlap (serialize; re-measure)

- **Kernel pins** — 20-9 L2 (R, `schedule_watchdog.rs`), 20-13 KT3 (security manager), the in-flight 20-14
  corrections (`capability/mod.rs`, `inference/mod.rs`) all re-pin `kernel-core-baseline.toml` and `kloc.toml:264`;
  `s21` orders this story after 20-9. None touches this story's kernel files (their records), but the set hash and
  counts move: re-base at T0.
- **`scheduler_loop.rs`** — 20-1b's landing edited it (`608735d2`); re-read `:89,113,490-505` at T0.
- **`operator_door.rs`, `main.rs`** — every Epic-20 bin story edits them; path re-points only, re-read at T0.
- **Ruling-4 `repin` script** (not landed) — if it lands first, use it for AC7.1.

### Current state that must be preserved

- FR20: FIFO, capacity 32, `QueueFull` backpressure, `recall_all_pending` order; the root drains only at safe points
  (`orchestrator_root.rs:693-708`); the door's queue/recall/status JSON and approval rows (`operator_door.rs:2028-2095,2852`).
- FR51: pause/resume journaling (`operator_door.rs:1031,1142`), the 19-2 pause fence (`scheduler_loop.rs:488`), resume
  hook order (port call before `fire_on_resume`, `:507`).
- FR42: `token.revoke` row shape (`journal_token_revocation`).
- The FR54 kernel tests' assertions, unchanged except imports.

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any kernel change outside AC2.1's list is STOP.
- No re-export shim (`pub use maos_orchestrator_buffer as orchestrator` or similar) in the kernel: clean cutover.
- Baseline JSON edited surgically, never re-emitted (ADR-066); `--write-baseline` is not used.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory; plants byte-restored
  (`proof-contract.md:24-31`).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Kernel never links the buffer (AC3) | dependency in kernel `Cargo.toml`; restored module | `check-dependency-closure`; structural test | local + hosted |
| Resume calls the port (AC2) | delete the call | kernel recording vector | local |
| Replay equivalence (AC2, AC5) | drop an item; reasoning format | leaf vector, golden | local |
| Pin (AC1) | leave `registry.rs` in the kernel | `check-kernel-baseline` | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-orchestrator-buffer -p maos-domain -p orchestrator
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test orchestrator_resume_port_21_5
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test orchestrator_equivalence_21_5 --test shell_host_16_2 --test erasure_million_20_11 --test worker_freeze_19_2 --test orchestrator_queue_19_2
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-cli --test orchestrator_queue_test --test orchestrator_recall_pick_19_2 --test pause_resume_test
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-control --test post_surface_16_1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p xtask --test orchestrator_out_of_kernel_21_5 --test kernel_pin_content_hash_16_0 --test recovery_lane_ceiling_rule --test d11_xtask_ceiling_ratchet
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-cohort-mesh
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-service-boundary --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dependency-closure --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-workspace-count
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- stability-matrix --check
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

Test binaries touching the orchestrator today (all must stay green or move): kernel `gateway_dispatcher_fr54`,
`gateway_uninstall_fr65_v05`, `lifecycle_journal_write_latency` (move); in-src unit tests of `buffer.rs`, `registry.rs`
(move with the files); `orchestrator` (`orchestrator_buffer`, `fixtures_pin`; CI `discipline.yml:1227`); maos-bin
`shell_host_16_2`, `erasure_million_20_11`, `worker_freeze_19_2` (support files), `orchestrator_queue_19_2`; maos-cli
`orchestrator_queue_test`, `orchestrator_recall_pick_19_2`, `pause_resume_test`; maos-control `post_surface_16_1`;
the 35 test files and 2 benches calling `SpiritSchedulerAdapter::new` (compile only; `None` still infers under KP).

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Test Infrastructure (glm-5.3). Focused suites per touched
crate + `cargo test -p maos-kernel-core --lib` at L2 (ruling `nextest`); the reviewer reads the plant record. Findings
per ruling `sev`.

### Previous-story intelligence

- 20-14 L2 (`edeedd25`): chose a hash-neutral kernel shape to avoid a second surface refresh; this story cannot avoid
  the `SpiritSchedulerAdapter` refresh (the field's type is hashed) — it is named for `prepaid` (2a).
- 20-8 L2 and 20-13 KT3: one operator-granted surface-hash refresh each — the precedent for the refresh, not for removals.
- 15-1 / ADR-066: the only sanctioned kernel surface removal; surgical edit, ADR as the record.
- 17-1: the last new leaf crate (`maos-egress`): workspace count sentence, `FORBIDDEN_CRATES` entry, kloc row.
- 6.1 / 6.5 (maos-capability, maos-iac extractions): kernel → crate dependency with `pub use` re-export — the KB shape,
  not recommended here (D-02).
- 19-2 (`75e1e493`): renamed the pause/resume latency test; its fence sits next to the replay (`scheduler_loop.rs:488`).

### Git intelligence

`0baa76fb` and `9e4be177` (operator rulings, docs only); `edeedd25` (20-14 L2, kernel 27040/21201); `1fd26dcc`, `a2bbd687`.
The orchestrator module last changed at `75e1e493` (2026-10-28); `scheduler_loop.rs` at `608735d2` (20-1b, 2026-11-11).

### Library / framework

No new external dependency: `dashmap 6.1`, `thiserror 2.0` (already workspace deps, `Cargo.lock` resolves
`dashmap 6.2.1`, `thiserror 2.0.18`); `tokio` and `maos-capability` as the leaf's dev-dependencies.

### References

E21:5,26,28,46,90-94,119; PF-R2:59,68; SN21:39,98-103,136,146; tracker `sprint-status.yaml:409,410,414,415`;
`deferred-work.md:1463`; rulings `decisions-2026-11-12-acceleration.md:25-36,38-45`; PRD
`functional-requirements.md:54,55,68`; architecture `4-kernel-design.md:117,180,186`, `15-full-spectrum-v2-2.md:100`;
ADR-066, ADR-069; 19-6 `r19_6_d04` (`19-6-…md:181`); `epic-15-foundations-w0.md:99-100,128` (917 rule).

## Dev Agent Record

### Agent Model Used

anthropic/claude-opus-5-5 (creation); anthropic/claude-sonnet-5-5 (L1 implementation, Dev215L1; L2 implementation, Dev215L2, 2026-11-12). L2 reviewers (2026-11-12): GPT gpt-6.1-sol (code review), GLM glm-5.3 (security review).

### Story context creation

Created 2026-11-12 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract; `Create215`) at
`0baa76fb`.

**Inputs:** E21 §21-5 and its preflight rows (PF-R2 D-J, SN21 §A/§C/§D); ADR-069; ADR-066; rulings of 2026-11-12;
tracker rows; `deferred-work.md:1463`; PRD FR20/FR51/FR54; direct reads of every cited range on a `git archive` copy
(`creation/cites-0baa76fb.md`); kernel, kloc and surface by the prebuilt gate binary on prototypes KP, KB, KD, KH
(`creation/measurements.md`, `creation/kloc-kernel-0baa76fb-and-prototypes.json`, `creation/surface-delta-0baa76fb.json`),
saved as diffs that pass `git apply --check -p1`, never applied.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–6.

**Records owed:** `creation/records-owed.md` (§A epic, §B tracker, §C planning, §D landing, §E routed).

**Revert:** no crate source modified (prototypes only under `/tmp/c215`, deleted). One `cargo tree --offline` ran in a
prototype copy; one accidental `check-cohort-mesh` build in the untouched copy was killed after about 40 s
(`creation/measurements.md` §Instrument). `intent-lineage-coverage-report.md` untouched (hash equal before and after
the gate runs).

**Gates after the new files** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS`, `--json`,
in-tree, read-only; `creation/gates/*.json`): `check-epic-close-coherence` rc 0 (21 epics; it read the working tree's
in-flight pin 27043); `check-decision-register` rc 0 (23 rows, 6 open); `check-exit-commands` rc 0 (2 blocks, 10 tokens);
`check-dev-record-completeness` rc 0 (199 done stories, 0 violations).

### Hosted proof

Discipline run 43175946640 on `637721bf`: `success` over 199 jobs (`abi-diff`, `check-kernel-baseline`, `check-empty-kernel`, `workspace-test-suite`, ship gates success) except the ADVISORY `reproducible-build` (job 131040106237): `No space left on device` / `collect2: ld terminated with signal 7 [Bus error]`, 79 MB free = hosted-runner disk exhaustion, infrastructure; it passed on run 43150757804. Filed as a rule-10 row in `deferred-work.md`. Closure note: `21-5-orchestrator-port-and-move-evidence/closure/closure-note.md` (AC table LANDED/VOID, claim boundaries).

**Claim boundaries:** the move is a verbatim relocation (plus the D-21-5-07 B one-lock replay); the FR54 gateway host remains unwired (zero production callers, allow-all stubs; deferred row F1); the `journal_*` audit writers left the kernel freeze gates, pinned only by a maos-bin golden (row F2); `resume` skips the replay when the port is `None` (row F3); AC6 is VOID (D-21-5-14 B) and the FR21 exemption and capacity-zero rows stay open with no 21-5 owner. Unverified: no local clean-tree `abi-diff` run (hosted job only).

### Debug Log References

`21-5-orchestrator-port-and-move-evidence/landing-1/` (`evidence.md`, `plants.md`, `red/`, `plants/`, `gates/`); `…/landing-2/` (`evidence.md`, `plants/`, `green/`, `gates/`).

### Completion Notes List

- **2026-11-12 — L1 landed UNCOMMITTED until Main commits (Dev215L1, kernel-free, base `d00802aa`).** Contents: `OrchestratorResumePort` in `maos-domain` (+3 tokei, no dependency added); the AC5 equivalence goldens recorded against the unmoved `maos_kernel_core::orchestrator` (5 green) with a fixture; the D-21-5-07 replay-race witness, **RED on the unmoved replay in 10 of 10 runs (and the first), committed `#[ignore]`d** so hosted CI stays green, L2 removes the attribute when the one-lock replay lands; dated FR20 / FR54 / §4.0.7 notes (no FR text deleted). No kernel source line changed (kernel 27103 / 21256 / 95 files unchanged, `repin.py --check` in sync). Pins: `maos-domain` 10274 → 10277 (+3, inside the D-21-5-12 A ceiling of +6; the script's overdraft flag was adjudicated against that ruling), aggregate hardfail 203241 → 203244. Evidence: `landing-1/evidence.md`.
- **What L1 taught L2:** see `landing-1/evidence.md` §L2 plan (the lost-instruction race is real; the one-lock replay degenerates to an identity; the `Some(registry)` coercion keeps every `new` call site).
- **2026-11-12 — L2 landed UNCOMMITTED until Main commits (Dev215L2, base `257a0fd7`).** What moved: the whole `orchestrator` module (buffer, registry, three journal fns, FR54 gateway host; 5 files) → NEW leaf `maos-orchestrator-buffer` (deps `maos-domain, maos-iac, maos-spirit-abi, maos-manifest, maos-attrs, maos-capability, dashmap, thiserror, tokio`; the prototype's closure omitted `maos-capability` and `tokio`, never compiled), three kernel tests moved with it, the kernel keeps one `OrchestratorResumePort` call. Measured: kernel **26062 physical / 20429 tokei / 90 files** (−1041 / −827, exactly the KP price; STOP −1028/−817 not crossed), leaf **833** (priced 829, +4 = the one-lock replay method), xtask **45462** (+1, priced), domain 10277 and bin 38602 unchanged, aggregate **203250**, hardfail 203251. Surface: 401 → 380 baseline items (21 deleted, `SpiritSchedulerAdapter` `2daf34b2…` → `d2170d38…`), `check-service-boundary` rc 0; ADR-070 written. Witnesses: the race witness un-ignored, RED 11/11 on the unmoved replay (L1) → GREEN 10/10 on the one-lock replay; kernel recording-port vector and leaf vector green; plants P1 (delete the port call), P2a/P2b (replay drops items / off by one), P6 (two-lock replay), P3 (kernel depends on the leaf), P4/P4b (module restored / old path named), P5 (registry left in the kernel) all red and restored. Gates observed rc 0: `check-kernel-baseline`, `check-cohort-mesh` (39 legs), `kloc-check`, `check-service-boundary`, `check-dependency-closure`, `check-workspace-count` (59), `stability-matrix --check`, `gen-abi-docs --check`, `check-loom`, `check-empty-kernel`, `check-serde-error-handling`, `check-dev-record-completeness`, `check-epic-close-coherence`. `abi-diff` not run (dirty tree; spirit-abi untouched). Not done: the hosted run and the review (Status stays `in-progress`). Evidence: `landing-2/evidence.md`.
- **2026-11-12 — L2 review dispositions (landed at commit `637721bf`; Rec215L2).** Measured at `637721bf`: kernel **26,062 physical / 20,429 tokei, 90 files**; leaf `maos-orchestrator-buffer` **833**; xtask **45,462**; hardfail **203,251**. GPT review (gpt-6.1-sol): **0 findings**. GLM security review (glm-5.3): **0 P0-P2, 3 P3** (F1 FR54 gateway host with allow-all stubs in the unfrozen leaf; F2 journal writers left the kernel freeze gates; F3 `resume` skips replay when the port is `None`). Under the operator's severity policy (`decisions-2026-11-12-acceleration.md` ruling 3) the three P3 are rule-10 rows in `deferred-work.md` ("Observed during: 21-5 L2 review"), no correction round. Reviews: `21-5-orchestrator-port-and-move-evidence/landing-2/review/`. The hosted run for `637721bf` is pending; **`done` is NOT claimed**; Status stays `in-progress`.

### File List

- `_bmad-output/implementation-artifacts/21-5-orchestrator-port-and-move-evidence/closure/closure-note.md` (closure)

- `crates/maos-domain/src/orchestrator.rs` (port)
- NEW `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs`, `crates/maos-bin/tests/fixtures/orchestrator_equivalence_21_5.json`
- `xtask/kloc.toml` (domain row, aggregate hardfail; `repin.py`)
- `_bmad-output/planning-artifacts/prd/functional-requirements.md` (`:54`, `:68`), `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (`:186`)
- NEW `_bmad-output/implementation-artifacts/21-5-orchestrator-port-and-move-evidence/landing-1/`
- L2 (Dev215L2): NEW `crates/maos-orchestrator-buffer/` (`Cargo.toml`, `src/{lib,buffer,registry,gateway_dispatcher,echo_gateway}.rs`, `tests/{gateway_dispatcher_fr54,gateway_uninstall_fr65_v05,lifecycle_journal_write_latency,resume_replay_21_5}.rs`); DELETED `crates/maos-kernel-core/src/orchestrator/*` (5) and `crates/maos-kernel-core/tests/{gateway_dispatcher_fr54,gateway_uninstall_fr65_v05,lifecycle_journal_write_latency}.rs`; `crates/maos-kernel-core/src/{lib.rs,isolation/mod.rs,scheduler/scheduler_loop.rs}`; NEW `crates/maos-kernel-core/tests/orchestrator_resume_port_21_5.rs`
- L2: `crates/maos-bin/{Cargo.toml,src/{main,operator_door,orchestrator_root}.rs,tests/support/{shell_world_16_2,worker_revoke_root}.rs,tests/orchestrator_equivalence_21_5.rs}` (imports; the one `#[ignore]` removed); `spirits/orchestrator/{Cargo.toml,src/lib.rs,tests/{fixtures_pin,orchestrator_buffer}.rs}`
- L2: `Cargo.toml`, `Cargo.lock`, `STABILITY.md`; `xtask/src/check_dependency_closure.rs`, NEW `xtask/tests/orchestrator_out_of_kernel_21_5.rs`, `xtask/tests/{kernel_pin_content_hash_16_0,recovery_lane_ceiling_rule}.rs`, `xtask/{kernel-core-baseline,kloc,kernel-api-classes}.toml`, `xtask/serde-error-allowlist.toml`; `docs/ci-baselines/kernel-surface-v0.1-beta.json` (surgical), NEW `docs/adr/ADR-070-…md`, `docs/adr/index.md`, `docs/adr/ADR-069-…md` (relief note); `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (`:117` workspace-count sentence); `21-5-flag-winston-decision.md` (L2 landing record); NEW `…-evidence/landing-2/`

## Change Log

- 2026-11-12: Created (`ready-for-dev`, D-21-5-01…14 PROPOSED) at `0baa76fb`; pulled forward by ruling `s21`.
- 2026-11-12: all fourteen D-rows RULED (ask `d21`, 2026-11-12, accepted as recommended: D01 A, D02 A, D03 A crate `maos-orchestrator-buffer`, D04 A, D05 A, D06 B, D07 B, D08 A ADR-070, D09 A, D10 A, D11 A, D12 A, D13 A key kept in Epic 21 after 20-9, D14 B). Story Status stays `ready-for-dev`. D13 A: `s21` waives the epic-20-retrospective precondition for this key only; `epic-21` → `in-progress`. D14 B: AC6 void; the maos-iac FR21 exemption row re-pointed to ownerless-and-open (operator named no key). Records applied: epic-21 corrections A1–A6, tracker B1–B3, acceleration note C1, deferred-work C2.
- 2026-11-12: L1 (kernel-free) landed uncommitted (Dev215L1): port, goldens, replay-race witness (red, `#[ignore]`d until L2), PRD/architecture notes; Status `ready-for-dev` → `in-progress`; domain +3 (10277), aggregate hardfail 203244, kernel pins unchanged.
- 2026-11-12: L2 (kernel move) landed uncommitted (Dev215L2): `maos-orchestrator-buffer` leaf crate (new workspace member, 59), the module out of the kernel (KP; kernel 27103/21256/95 → 26062/20429/90), `SpiritSchedulerAdapter` holds `Option<Arc<dyn OrchestratorResumePort>>` (the one surface refresh `2daf34b2…` → `d2170d38…`), 21 baseline triples removed under ADR-070 (401 → 380), one-lock replay (D-21-5-07 B; the race witness un-ignored, green), `maos-orchestrator-buffer` in `FORBIDDEN_CRATES` (xtask +1), ADR-069 relief note; pins exact (leaf 833, xtask 45462, aggregate 203250/hardfail 203251). Status stays `in-progress` until the hosted run and review.
- 2026-11-12: CLOSED `done` (Close215): hosted discipline run 43175946640 on `637721bf` green except the advisory `reproducible-build` disk exhaustion (row filed); closure note under `…-evidence/closure/`; T0/T3/T5 ticked; AC6 VOID per D-21-5-14 B.
- 2026-11-12: L2 landed at `637721bf`; independent reviews recorded (GPT gpt-6.1-sol 0 findings; GLM glm-5.3 security 0 P0-P2 + 3 P3, rows added to `deferred-work.md` under rule 10, no correction round); hosted run for `637721bf` pending; `done` NOT claimed; Status stays `in-progress`.
