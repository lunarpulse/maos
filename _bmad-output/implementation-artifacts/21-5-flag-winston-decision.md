# 21-5 FLAG-Winston decision record

**Date:** 2026-11-12  
**Status:** `PROPOSED` — nothing granted. Every row below needs an operator ruling; no implementation or landing claimed.  
**Scope:** the kernel row of `21-5-orchestrator-port-and-move`: `crates/maos-kernel-core/src/orchestrator/` leaves the
kernel for a new leaf crate; the kernel keeps one domain-port call on resume. Negative kernel delta; public kernel
surface shrinks.  
**Story:** `_bmad-output/implementation-artifacts/21-5-orchestrator-port-and-move.md` (frontmatter, §Decisions, §Kernel and kloc pricing).  
**Evidence:** `21-5-orchestrator-port-and-move-evidence/creation/measurements.md`, `kloc-kernel-0baa76fb-and-prototypes.json`,
`surface-delta-0baa76fb.json`, `kp-domain-port-leaf-move.diff`, `kb-kernel-depends-on-leaf.diff`,
`kd-drop-kernel-replay.diff`, `kh-buffer-half-only.diff`, `cites-0baa76fb.md`.  
**Ruling basis:** `s21` (pull 21-5 forward to follow 20-9 as a kernel story), `lim` = stop22 (shared STOP 22 000 =
ADR-069 alarm), `prepaid` (2a) (`planning-artifacts/decisions-2026-11-12-acceleration.md:28-36`).  
**Precedent:** `20-14-flag-winston-decision.md`, `20-9-flag-winston-decision.md`; ADR-066 (sanctioned surface removal).

## Decision requested

| ID | Options priced (reverted prototypes on `git archive 0baa76fb`; kernel physical / tokei) | Recommended | Status |
|---|---|---|---|
| D-21-5-01 | **A** whole module moves (KP) −1041 / −827 · **B** buffer half only, FR54 gateway host stays (KH) −407 / −304 · **C** whole module into two crates (buffer, gateway host) — kernel side identical to KP | **A** | PROPOSED |
| D-21-5-02 | **A** `maos_domain::orchestrator::OrchestratorResumePort`, kernel holds `Option<Arc<dyn …>>` (KP) −1041 / −827, domain +3 · **B** kernel depends on the leaf crate, concrete type (KB) −1029 / −818 · **C** kernel drops the resume replay, ctor param removed (KD) −1049 / −833, 37 callers edited | **A** | PROPOSED |
| D-21-5-08 | **A** operator-sanctioned removal of the 21 kernel symbols below by ADR-070 (ADR-066 form) + surgical baseline edit; the `SpiritSchedulerAdapter` hash refresh under `prepaid` (2a) · **B** removal sanctioned by this record alone, no ADR | **A** | PROPOSED |
| D-21-5-11 | **A** both kernel pins re-pinned to the exact measured values in their current units; `maos-kernel-core` kloc ceiling LOWERED by the moved amount; HISTORY records the ADR-069 relief · **B** baseline re-pinned, kloc ceiling kept at 21201 (827 lines of headroom) | **A** | PROPOSED |
| D-21-5-12 | **A** ×2 envelope on the growing crates (domain ≤ +6, xtask ≤ +2, aggregate ≤ +12, maos-bin 0), kernel exact with STOP if Δ > −817 tokei or > −1028 physical · **B** exact measured only | **A** | PROPOSED |

Why A for D-02: FR20 says the instruction buffer is "NOT kernel-buffered" (`prd/functional-requirements.md:54`) and
§4.0.7 says the kernel embeds no orchestration policy (`4-kernel-design.md:186`); only a domain port lets the kernel
stop linking the buffer crate, and only then can `check-dependency-closure` forbid it (D-21-5-09; under B the kernel
depends on the crate). A keeps behaviour exact: the replay body moves verbatim behind the port (equivalence witnesses
D-21-5-06). C removes the replay — a drain and re-enqueue of the same queue, an identity with a race window
(`scheduler_loop.rs:490-505`) — but is a behaviour change and edits 37 callers.

Why A for D-01: the FR54 gateway dispatcher has no production caller (grep `crates/maos-bin` = 0), FR54 is parked to
v2.5 (`functional-requirements.md:68`), and it holds 515 of the module's 817 tokei (`gateway_dispatcher.rs` 441, `echo_gateway.rs` 74). FR54's "kernel hosts the lifecycle
and capability-scope contracts" reads, at HEAD, onto the `maos-spirit-abi::gateway` traits, which stay; this reading is
an interpretation, so A carries a PRD/architecture note and B is the option that keeps FR54's letter.

## Kernel figures (if ruled as recommended: D-01 A, D-02 A, D-09 A)

Creation base `0baa76fb`: **27040 physical / 95 files / 21201 tokei**. KP measured **−1041 physical / −827 tokei →
25999 / 90 files / 20374** (module −1028 / −817; `scheduler/scheduler_loop.rs` −12 / −9; `lib.rs` −1 / −1;
`isolation/mod.rs` doc 0 / 0; set hash `bb22880f…`). Kernel STOP for this story: kernel Δ above −817 tokei or −1028
physical (the module not fully out), or any change to a kernel file other than `lib.rs`, `scheduler/scheduler_loop.rs`,
`isolation/mod.rs` (doc only) and the five deleted files.

Other crates (tokei): new `maos-orchestrator-buffer` **829** (src physical 1046); `maos-domain` **+3**; `maos-bin` **0**;
`xtask` **+1** (D-21-5-09 A); aggregate **+6** (202855 → 202861; hardfail = measured + 1 = 202862).

## Public kernel surface (`check-service-boundary`, `docs/ci-baselines/kernel-surface-v0.1-beta.json`)

**Refresh (symbol persists; pre-approved by `prepaid` (2a) once this grant is ruled — named here as required):**

| Kind | Path | HEAD hash | After KP |
|---|---|---|---|
| struct | `maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter` | `2daf34b24f2563ac0d1861c9736e1d4f7a7231e7d616e18cc8df6b3dd4b0e40c` | `d2170d389c72d69275cbad202942db89f6d5295bb63104fee3f86c30d7896bf1` |

(Under KB: `77bfe43017690a920d96b16560356b480ff042460c48e02fb2f55116766ec500`; under KD:
`891bd15e7a12c6d48d532ed64fea82c1854b264dd22dcf2df0e91aacff517f9d`. The hash covers the private field's type; the
`use` leaves `…scheduler_loop::SpiritSchedulerAdapter` `bd25e470…` and `…api::crate::scheduler::SpiritSchedulerAdapter`
`a0572416…` do not change.)

**Removal (symbol leaves the kernel; NOT covered by `prepaid` (2a), which names refreshes — needs D-21-5-08):**

| Kind | Path | HEAD hash | New home (KP) |
|---|---|---|---|
| enum | `maos_kernel_core::orchestrator::buffer::OrchestratorBufferError` | `c8b994d0791687357c8a6a6aa7a62a379ffd0bd360fa9d8da6d298df246b74e6` | `maos_orchestrator_buffer::buffer::OrchestratorBufferError` |
| struct | `maos_kernel_core::orchestrator::buffer::OrchestratorBuffer` | `c2f19b8a5a4d608a14eec0ec84060f2bf76efa832e72d06b73c82da9c1baeb17` | `maos_orchestrator_buffer::buffer::OrchestratorBuffer` |
| struct | `maos_kernel_core::orchestrator::registry::OrchestratorBufferRegistry` | `f07938833d9959b14189bacb7fc7dc24595a584fb93769d79893daefbca316d7` | `maos_orchestrator_buffer::registry::OrchestratorBufferRegistry` |
| fn | `maos_kernel_core::orchestrator::journal_orchestrator_queue` | `a430e2f986fac353d1acb98cc30bf5e0b519d7d5fd97d964806826922c59bf7d` | `maos_orchestrator_buffer::journal_orchestrator_queue` |
| fn | `maos_kernel_core::orchestrator::journal_director_lifecycle_action` | `29f7bf96e33dd41b405bec65b5b31747675f1e91af1f70002ac75c1585ebc86e` | `maos_orchestrator_buffer::journal_director_lifecycle_action` |
| fn | `maos_kernel_core::orchestrator::journal_token_revocation` | `bb5d648b449b75a4397d4d3b10c72f95d657342dd6f316ab799368697626c3b6` | `maos_orchestrator_buffer::journal_token_revocation` |
| struct | `maos_kernel_core::orchestrator::echo_gateway::EchoGatewayFactory` | `7f04203c6946f7b6b0166916a5ee69614f0b6f52d1b6231e288456944900d08c` | `maos_orchestrator_buffer::echo_gateway::EchoGatewayFactory` (D-01 A/C) |
| struct | `maos_kernel_core::orchestrator::echo_gateway::EchoGatewaySubmodule` | `ff590b62d552c0598f4a2d2815b68a788b6b51b09d55e24a9252e704402377f5` | same crate (D-01 A/C) |
| struct | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewayDispatcher` | `e687b4485353b93506414f2c51110f6bc7d9591e6c85c2c95846d2929dfc4872` | same crate (D-01 A/C) |
| struct | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewayInstance` | `6e9f58998b68d91dea0a4ebbdd0bb1cf2abd2dbe260c6fc1462e41fa8b18a187` | same crate (D-01 A/C) |
| struct | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewaySubmoduleRegistry` | `93820db9d6428033904a2803e98e2f9943454c44468bc56676a79e053f5f518b` | same crate (D-01 A/C) |
| trait | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewaySubmoduleFactory` | `5a5e01e8f2fb5731afe6e06cc4c31c591153aa2d2d8974c37cfa91ecf57e7721` | same crate (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::buffer::OrchestratorBuffer` | `fab11b646d107620cb2b0e8b05fafef61542a9fcd796118d3b90493611e11e7b` | crate-root re-export |
| use | `maos_kernel_core::orchestrator::buffer::OrchestratorBufferError` | `b6b819a36eae64478f8ab8302d7142f2ac49cbea952a7b8728072ceca789ef6b` | crate-root re-export |
| use | `maos_kernel_core::orchestrator::registry::OrchestratorBufferRegistry` | `a330507117b40627aa3d100632b9a0af4268eb3caa41f06fd5944a68a151d3d3` | crate-root re-export |
| use | `maos_kernel_core::orchestrator::echo_gateway::EchoGatewayFactory` | `ead79c101ed4c354bda83069dd02151d5129af1239889b15ca0b2bf67c5528eb` | crate-root re-export (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::echo_gateway::EchoGatewaySubmodule` | `998c737b27d3ee5dfd15e65b3e5659158bbfb72b7206e87cae4f16dedd02d0e2` | crate-root re-export (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewayDispatcher` | `9f8b7959d07f42e3e4bf12e0687b3e7080920373579087a81849354d4e2c69d4` | crate-root re-export (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewayInstance` | `f93095489864c794c33db350ee737bcb31e0859b731c38655bb15bfdd988740a` | crate-root re-export (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewaySubmoduleFactory` | `610108159a9698d4b1c7137927e7e64fa603b08eeae088b628b866908e03cebb` | crate-root re-export (D-01 A/C) |
| use | `maos_kernel_core::orchestrator::gateway_dispatcher::GatewaySubmoduleRegistry` | `6e48c02949956a079be882a94dee081526b4dbbec9dea92f08729202c2b2ebaf` | crate-root re-export (D-01 A/C) |

Under D-01 B (KH) only the first nine removal rows apply (6 items, 3 `use` leaves) plus the refresh.
Baseline after KP: 401 → **380** items (21 deleted, 1 hash refreshed), hand-edited surgically, never re-emitted
(ADR-066 provenance). `xtask/kernel-api-classes.toml:367-375` (nine `orchestrator` rows) are deleted with them.
The new trait `maos_domain::orchestrator::OrchestratorResumePort` is outside the kernel surface. `abi-diff`
(`maos-spirit-abi` only) and `xtask/abi-baseline/*.txt` are unaffected. `STABILITY.md:30` (workspace crates 58 → 59)
is regenerated by `stability-matrix`.

## ADR-069 (alarm 22 000, envelope 23 000; shared STOP 22 000 by `lim` = stop22)

Remaining envelope after KP on the creation base: **23 000 − 20 374 = 2 626**; to the alarm / STOP: **1 626** (HEAD:
799). After the owed 20-9 R (+21) and 20-13 KT3 (±1): about 1 604–1 606 to the alarm [INFERENCE: those rows are not
landed; no file overlap with this story]. No shape crosses 22 000, so no row carries `ENVELOPE ALARM`.

**21-5 promotion (ADR-069 §3 wording):** 21-5 is promoted from optional to **scheduled** by operator ruling `s21`
(2026-11-12), not by the alarm firing; ADR-069:56 ("optional until the alarm fires") is satisfied in spirit because
STOP = alarm since `lim` = stop22 (`decisions-2026-11-12-acceleration.md:38-45`). The landing's kloc HISTORY row says
`ADR-069 RELIEF VALVE (21-5, ruling s21)`, the before/after figures, the remaining envelope and the alarm distance.

## Landing and accounting (if ruled)

L1 (kernel-free): the domain port, the equivalence goldens recorded against the unmoved kernel, docs. L2 (kernel last):
the move, both pins exact in their current units — `xtask/kernel-core-baseline.toml` (`src_lines`, file set 95 → 90,
`set_hash`, the three edited files' content hashes, five rows deleted, EOF HISTORY) and `xtask/kloc.toml:264` (lowered
by the measured amount), `:684` (new crate), `:425` (domain), `:339` (xtask), `:718` (hardfail); each HISTORY row's
first segment cites this key with `->` (D11 A). The xtask test constants move with them:
`kernel_pin_content_hash_16_0.rs:34` (95 → 90) and `:820,838,847,858` (27040 → new), `recovery_lane_ceiling_rule.rs:279`
(`RATIFIED_AT_EASING` lowered to the new ceiling — the freed lines are not growth capacity, §15.5 Decision 2(a),
`15-full-spectrum-v2-2.md:100`). Unused room is not reserved.
