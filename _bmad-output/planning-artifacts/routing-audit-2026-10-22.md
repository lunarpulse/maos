# Requirements Routing Audit — 2026-10-22

**Why:** at the 17-9 preflight (2026-10-22) FR28 and FR45 were found "routed" to story 21-4 whose ACs do not implement them, and ADR-016 had no story; three stories (18-5, 18-6, 21-6) were added in one day on top of four E17 additions since 2026-09-25 (17-6/7/10/11). The Recovery Coverage Map (`epics/requirements-inventory.md:534-582`) was never checked clause-by-clause against the owner stories' AC text. This audit does that once, for every FR, routed NFR, journey and v1.0 technical success criterion, so the remaining lane scope is fixed before Epic 18 opens.

**Method:** for each requirement, the owner story's AC text (epic file §, or the done story's record) was read for the PRD clause. Verdicts: **OWNED** (AC states the behaviour), **PARTIAL** (names the missing part), **MISROUTED** (owner's ACs do not implement it), **PARKED** (no story), **LIVE-VERIFIED / LIVE-UNVERIFIED** (claimed live in the 2026-09-04 review; product-binary witness found / not found — witnesses inspected, not executed). Three read-only scouts at HEAD `3a7c4260`; MISROUTED and ADD rows spot-checked by hand (`epic-19-founder-loop-w4.md:59-62`, `main.rs:9291-9303`, `17-8…md:114`, `17-1…md:19`).

**Totals (109 scout rows; 14 requirements appear in two slices and keep the stricter verdict in §1–§4):** OWNED 11 · PARTIAL 51 · MISROUTED 4 · PARKED 10 · LIVE-VERIFIED 29 · LIVE-UNVERIFIED 4. Full per-requirement tables: Appendix A–C.

**Disposition rule (PRD `project-scoping-phased-development.md:378` — "No silent de-scoping: every requirement … is mapped to a phase"):**
- **ADD** a story only when a clause has no owner, a lane milestone (v0.3/v0.5/v0.8/v1.0-rc) needs it, and it cannot be a widened AC of an existing backlog story.
- **WIDEN** when the owner is a backlog story — folded at that epic's preflight (operator directive 2026-09-06), inventory routing updated now.
- **PARK** only with a named phase and an existing operator ruling.
- **DECIDE** where the PRD clause and a ratified ruling conflict — operator call, not a story.

## 1. ADD — nine stories (fixed inventory for the E18–E21 preflights)

| Key (proposed) | After | Closes | Scope (one line) | Kernel-Δ | Evidence of gap |
|---|---|---|---|---|---|
| `18-7-live-skill-authoring-and-revisions` | 18-1 | FR39, FR57 | capability-mediated runtime `skill.author.self` submission with durable skill content, telemetry-evidenced revision proposals (target/version/diff/evidence) preserved through operator admission, vetting and audit | 0 [INFERENCE] | `requirements-inventory.md:562` promised "follow-on at Epic 18 preflight"; `main.rs:13135-13164` smoke-only enqueue; `crates/maos-skill/src/store.rs:50-75` payload not persisted |
| `19-5-cross-form-upgrade-and-cold-swap` | 19-3 | FR10, FR11, FR49 | state/token-preserving upgrade parity for the subprocess form; admission rejects an archived predecessor without a migrator; cold-swap policy ships (`ColdSwapUnsupported` today) | ◆ FLAG-Winston (scheduler) | `16-1…md:43-46,478-486` compiled Butler swap only; `05-fr1-20-scout.md:13` |
| `19-6-crash-originator-delivery-and-disposition` | 19-1 | FR12, FR50, NFR-Rel-1 | crash → mailbox delivery to the task originator with reassignment; all declared crash dispositions applied (production `NullReplicaResolver` replaced) | 0–small | `16-3…md:35-37` journaled, never mailbox-delivered; `main.rs:2795-2796` |
| `20-8-manifest-contract-completion` | 20-1 | FR8 | manifest declares `skills`, `explanation_shape`, `intent_promotion_set`, `swap_invariants`; signed manifest verified at admission | ◆ FLAG-Winston (admission) | `05-fr1-20-scout.md:10` four fields missing; unsigned admission |
| `20-9-spawned-form-lifecycle-hooks` | 20-8 | FR55 | budgeted lifecycle triggers (incl. 17-4's idle-quiescent consolidation + live telemetry hooks) reach spawned forms (subprocess Worker, WASM); v1.0 third-party Spirits otherwise lack the hook ABI | ◆ FLAG-Winston | `17-4…md:51,135` "spawned forms receive neither new hook" |
| `20-10-purge-residue-removal` | 20-2 | FR2 | `maos purge` removes owned T3 containers and cgroups it today only reports | 0 | `16-4-maos-uninstall-and-keyring.md:423-424` |
| `20-11-erasure-inventory-closure` | 20-3c | FR65 | per-Spirit erasure inventory enumerates and removes intent lineage, pending halts and scheduled invocations (today `intent_lineage` is `VerifiedEmpty` by assumption; `scheduled_invocations` is `CoverageGap`) and the proof covers them | 0–small | `crates/maos-bin/src/main.rs:9290-9305` |
| `21-7-production-consent-granter-and-gate` | 21-2 | FR29 (production half) | a production granter sets `HeaderOnly`/`Withheld`, a production `ConsentGate` is installed, native senders carry a real pid (not 0); J3/Reza consent is real, not suite-delivered | 0 | `17-8…md:114` (rule-11 label), `:45` |
| `21-8-three-protocol-session-witness` | 21-2 | success-criteria.md:50 | one binary-level session: launched by an ACP editor host, pulling tools from an MCP server, peering over A2A — ACP (5-5c), MCP (8-14b/c), A2A (8-6) each exist; the combined witness does not | 0 | no owner in the recovery map |

Totals after ADD: E17 14 · E18 7 · E19 6 · E20 11 · E21 8 (remaining backlog 26 → 35, +35 %). Rule 6 "≤6 stories/epic" is already exceeded in E17 and E20; it is recorded, not re-asserted.

## 2. WIDEN — ACs to fold at the owner's preflight (routing updated in the inventory now)

| Owner | Clause to add | Evidence |
|---|---|---|
| `18-1` | FR17 digest floors: first session ≤30 s, all fields, per-Spirit 100-digest floors (19-4 disclaims them: `epic-19:86`); FR58 Butler zero-config reference response (moved from 16-2) | `epic-18:63`; `16-2…md:25-34` |
| `18-2` | NFR-Test-4 Researcher and Orchestrator class corpora/gates (Butler only today); FR15 per-class halt floors (moved from 16-2) | `epic-18:77`; `16-2…md:25-26` |
| `18-3` | NFR-Aud-7 five thresholds binding per shipping class (faithfulness/hedge "never a CI floor" today); J-Researcher: eight-way tools, contradiction/resume, four output sections, citation/time floors | `epic-18:85-87` |
| `18-4` | FR31 principal-namespace persistence of the acceptance ledger ("deferred" today); J-Butler persistent learning, actual option effects, variance-led halt reconciliation | `epic-18:62,93-94` |
| `19-1` | FR14 human task envelope (goal/scope/success criteria/posture) with terminal/ACP/push delivery; FR19 per-Spirit per-tag halt-policy schema + CLI consumed live; FR24 whole re-emission provenance | `epic-19:59-62` — machine dispatch only |
| `19-2` | FR51(a)(d) bounded director interruption and revocation (16-1 has no interrupt mechanism); FR51(c) already AC3 | `16-1…md:35-42`; `sprint-status.yaml:284` |
| `19-3` | FR21 fan-out floors (50 concurrent Workers, P99 ≤500 ms, zero drops at 10/s for 1 h, envelope fields); FR25 bridge/persona skills asserted in agent context; FR40 declared-version mismatch refuses startup for real adapters; FR4 1000-call mediation proof | `epic-19:76-78`; `worker_spawn.rs:759-805`; `17-1…md:111-121` |
| `19-4` | J1 DAG/independent continuation, skill loading, overnight session; keep the 2026-09-21 re-pin precondition | `epic-19:78,84-88` |
| `20-1` | FR36 authenticated control-plane install; FR37 post-promotion lapse/revocation consequences (tracker-only today); FR59 ≤5 min yank sync + vetter-origin propagation; FR13 quarantine isolation | `epic-20:76-79`; `sprint-status.yaml:289`; `17-1…md:143` |
| `20-2` | FR1 RPM build/publication + cargo distribution; NFR-Ops-12 air-gap witness runs substrate activity + journaling, not `--version` | `epic-20:84-85`; `tests/air-gap-netns-corroborate.sh:51-55` |
| `20-3b` | LIVE-UNVERIFIED witnesses: FR31 namespace isolation, FR34 SDK external-author coverage, FR53 halt identity across swap (test asserts re-firing only, `one_daemon_one_door_16_1.rs:957-961`), FR63 catalog/retryability/cause-chain; I1–I14 invariant ledger published as *guaranteed* (property test in CI) or *asserted* (PRD `success-criteria.md:38` sanctions the downgrade); adversarial-corpus coverage per invariant (`success-criteria.md:39`) | `08-fr21-65-scout.md`; `docs/invariants/README.md:7` |
| `20-3c` | FR41 `--tag` filter implemented (`subcommands.rs:2756-2765` ignores `_tag`; `audit_9_1_headline_smoke.sh:181-194` expects refusal); FR62(b) Transparency-Log emitter for ABI proposals/ratifications; NFR-Ops-9 operational RPO/RTO, not collector honesty | `17-3b…md:37,252-253`; `check_rto.rs:23-26` |
| `20-4` | FR3 Kimi + per-Spirit provider selection | `epic-20:152-154` |
| `20-7` | FR16 posture-shift latency corpus (already `epic-20:190`); the six PRD hot-path budgets (`success-criteria.md:40-46`): issuance <5 µs, dispatch <5 µs, memory/read 50 µs/5 ms, hot-swap 50/500 ms, telemetry <1 µs, iac/send <10 µs | `epic-20:186-193` |
| `21-1` | J4 cross-host routing and "deployed fix", not proposal | `epic-21:54,56` |
| `21-2` | J3 live digest/conflict-halt production (fixture regeneration today); Reza live proposal/approval loop | `epic-21:64-65` |
| `21-3` | FR23b ±5 min skew, ≤10 s partition fail-safe, CRL closure; NFR-Sec-13 scale/percentile acceptance | `epic-21:72-79` |
| `21-4` | NFR-Maint-1: see DECIDE-4 | `epic-21:83-87` |
| `ops-external-cohort-and-pen-test` | NFR-Onb-1/Test-8/J6: ≥3 authors is the preliminary gate; N=12 with timing floors is the v1.0 PRD clause — record the honest N | `sprint-status.yaml:310` |

## 3. PARK — named phase, existing ruling

| Req | Ruling | Phase recorded |
|---|---|---|
| FR7 telemetry opt-in | no emitter exists (`requirements-inventory.md:546`) | non-goal until an emitter exists |
| FR6 macOS/Windows resource caps | Linux proof only (`17-2…md:127`) | v1.5 (Windows is 10.5's phase) |
| FR33 Python template | round-3 "Python toolchain later" (`sprint-change-proposal-2026-09-04-round2.md:79`) | v1.5 |
| FR54 gateway connectors | round-2 v2.5 parking (`…round2.md:28`) | v2.5 — see DECIDE-3 (**RULED 2026-10-25: keep v2.5** + PRD DELTA-2026-10-25) |
| FR61 GPG key | ADR-047 documented deviation (`SECURITY.md:11-22`) | unchanged |
| FR47 for Workers | ADR-061 narrows: the egress proxy is the Worker's sole egress, not the kernel Inference Port (`17-1…md:19`) | unchanged; first-party Spirits stay on the Inference Port |

## 4. DECIDE — operator rulings needed (PRD phase vs. ratified parking)

1. **FR48 operator-selectable crypto provider** — parked "later (post-v1.0)" (`requirements-inventory.md:564`); the PRD lists it in the v0.5 phase. Options: (a) ratify post-v1.0 as a DELTA in the PRD, (b) add a pluggable-crypto story. **RULED 2026-10-23 (operator): split** — v1.0 = one operator selection point + every kernel FR48 surface routed through the selected provider (story `20-13-pluggable-crypto-provider-selection`); the FIPS 140-3 module (NFR-Sec-15's "validated default option") = v1.5 (PRD DELTA-2026-10-23).
2. **FR64 cost-attribution dimensions** — parked "later" (`:570`); PRD phase v1.0 (`functional-requirements.md:122`). Options: (a) DELTA to v1.5, (b) add `20-14-complete-cost-attribution` (renumbered 2026-10-23: `20-13` is the FR48 story). **RULED 2026-10-25 (operator): split** — v1.0 = per-task and session-principal token attribution on the cost frame + `maosctl audit cost-reconcile` grouped by task and principal (story `20-14-complete-cost-attribution`, ◆ FLAG-Winston, kernel-Δ by prototype inside ADR-069's envelope); the CPU-time and storage-I/O dimensions = v1.1 (PRD DELTA-2026-10-25; ADR-046 R6 already defers them).
3. **FR54 gateway connector contracts** — the v2.5 label parks kernel machinery (lifecycle + scope contracts for manifest gateways), not only ecosystem adoption. Options: (a) keep v2.5, (b) `21-9-gateway-submodule-contracts`. **RULED 2026-10-25 (operator): (a) keep v2.5 + PRD DELTA** — the PRD's v1.0 phase entry for FR54 is retagged v2.5 in place (DELTA-2026-10-25); the v1.0 hermes-tenant positioning is defended by FR65/NFR-Aud-12 uninstall-and-erasure (`20-11`) and principal scoping, not by gateway sub-modules. Measured 2026-10-25: the manifest parses a `[[gateway]]` schema (`crates/maos-manifest/src/manifest.rs:4800-4822`; a post-v1 schema section, `:337`; the only non-network type is the `Echo` fixture, "NOT for production use"); kernel-core's v0.5 `GatewayDispatcher` (`crates/maos-kernel-core/src/orchestrator/gateway_dispatcher.rs:65`) has no production constructor — only `crates/maos-kernel-core/tests/gateway_dispatcher_fr54.rs` and `gateway_uninstall_fr65_v05.rs` build it; no shipped gateway, no production lifecycle wiring. No story is created.
4. **NFR-Maint-1 / PRD ≤20 KLOC trusted core** — kernel-core is 20,538 with zero headroom; 17-11, 18-5, 18-6, 19-5, 20-8, 20-9, 21-6 are unpriced ◆ FLAG-Winston. Options: (a) ADR re-ratifying the ceiling (ADR-057 already moved the crate-set figure to ≤25K), (b) promote 21-5 negative extraction from optional to required before 21-6. **RULED 2026-10-23 (operator): (a) increase** — `docs/adr/ADR-069-kernel-core-envelope-through-v1.md`: `maos-kernel-core` ≤ 23,000, alarm 22,000, through `v1.0.0` GA; zero headroom and per-story grants unchanged; enforced by `xtask/tests/recovery_lane_ceiling_rule.rs`; `21-5` stays optional until the alarm fires.
5. **Journey reductions (J3/J4/Reza hermetic scenes)** — the lane defines them as fixture/nightly mechanism scenes with live runs on the operator lane. This is by design (confidence rule 2); it is recorded here so the v1.0-rc ledger says "mechanism-proven, live on operator lane", not "reproducible by an 8-person team".

## 5. Routing corrections applied to the inventory
See `epics/requirements-inventory.md` § "Routing audit corrections (2026-10-22)". Tracker rows for §1 added as `backlog` under their epics with `⚠ ADDED 2026-10-22 (routing audit)`; full §-sections with ACs are written at each epic's preflight per the 2026-09-06 directive.

## Appendix A — Epic 18 / 19 slice

| Req | PRD clause | Routed home | Verdict | Evidence | Proposed action |
|---|---|---|---|---|---|
| FR14 | Human assigns goal, scope, success criteria and posture | 19-1 | MISROUTED | `epic-19-founder-loop-w4.md:59-62`: machine dispatch from epic sections; no human assignment surfaces | widen `19-1` |
| FR15 | Resolve halts through three pathways | 16-2; 19-4 | LIVE-VERIFIED | `crates/maos-cli/tests/halt_resolve_test.rs:49-124` | floors → `18-2` |
| FR16 | Runtime posture shift, journaled and bounded | 16-1 | LIVE-VERIFIED | `crates/maos-bin/tests/one_daemon_one_door_16_1.rs:387-404` | latency corpus → `20-7` |
| FR17 | Per-Spirit morning digest, complete, timely and grounded | 18-1, 19-4 | PARTIAL | `epic-18:63`; `epic-19:86` disclaims 100-digest floor | widen `18-1` |
| FR18 | Decision records reconstructable working-memory references | pre-existing | LIVE-VERIFIED | `crates/maos-bin/src/main.rs:2490-2523` | none |
| FR19 | Per-Spirit, per-tag recall-vs-precision policy | 19-1 | MISROUTED | `epic-19:59-62`: no policy configuration or threshold application | widen `19-1` |
| FR20 | Queue instructions without preempting active delegations | 19-2 | OWNED | `epic-19:68`: "only after the in-flight `delegation_completed`" | none |
| FR21 | Scoped, skill-named dispatch with distillates and sustained fan-out | 19-1, 19-3 | PARTIAL | `epic-19:62,78`: fan-out advisory; no hourly soak | widen `19-3` |
| FR22 | Mailbox routing, log-before-deliver, distillates, raw recall | pre-existing | LIVE-VERIFIED | `main.rs:2490-2523`; `two_host_delegation_2b.rs:337,619` | none |
| FR23a | Loopback authenticated, consent-enforced inter-host communication | pre-existing | LIVE-VERIFIED | `crates/maos-bin/tests/two_host_delegation_2b.rs:4-6,337-345` | none |
| FR23b | Full mesh tolerates skew, partitions and revocation | 21-3 | PARTIAL | `epic-21:73-79`: pins/rotation; no skew/partition floors | widen `21-3` |
| FR24 | Autonomous postures and preserved task-envelope provenance | 16-1 | PARTIAL | `one_daemon_one_door_16_1.rs:387-404` posture only; no re-emission provenance AC | widen `19-1` |
| FR25 | Wrapped agent CLI loads bridge and persona skills | 19-1, 19-3 | PARTIAL | `epic-19:76-78`: subprocess/effect oracle, not persona-skill loading | widen `19-3` |
| FR26 | Scheduled hooks obey capability and revocation contracts | pre-existing | LIVE-VERIFIED | `main.rs:8475-8486` ScheduleWatchdog | none |
| FR27 | Persist tagged scalars; four predicates | pre-existing | LIVE-VERIFIED | `main.rs:590-631`; `tests/integration/smoke_epic_4.sh:19-34` | none |
| FR38 | Verify ComplianceClaims; reject context drift | pre-existing | LIVE-VERIFIED | `crates/maos-cli/src/subcommands.rs:1350-1376`; `smoke_registry_5d_test.rs:39-48` | none |
| FR39 | Runtime self-authored skills await operator admission | "E18 preflight" | PARKED | `requirements-inventory.md:562`; `main.rs:13135-13147` smoke-only enqueue | ADD `18-7` |
| FR40 | Refuse CLI startup on output-shape mismatch | pre-existing | PARTIAL | `worker_spawn.rs:759-805`: checked at completion, not startup | widen `19-3` |
| FR41 | Authenticated filtered audit queries incl. tags | none | PARTIAL | `subcommands.rs:2756-2765`: `_tag` ignored; `audit_9_1_headline_smoke.sh:181-194` expects refusal | widen `20-3c` |
| FR42 | Cross-Spirit subject-access with provenance | pre-existing | LIVE-VERIFIED | `subcommands.rs:3888-3891`; `audit_9_1_headline_smoke.sh:197-208` | none |
| FR43 | Attributed posture deltas | pre-existing | LIVE-VERIFIED | `subcommands.rs:3977-3984`; `audit_9_1_headline_smoke.sh:214-226` | none |
| FR44 | Signed, independently verifiable sealed bundle | pre-existing | LIVE-VERIFIED | `subcommands.rs:2898,3040,3559`; `audit_9_1_headline_smoke.sh:240-291` | none |
| FR45 | Forget cascades; redact and re-distill | 18-6 | OWNED | `epic-18:118-121`: "a re-distillation is triggered" | none |
| FR46 | Signed, redacted, versioned trajectory export | pre-existing | LIVE-VERIFIED | `subcommands.rs:2669-2681,4081-4222` | none |
| FR52 | Authorized T3 subprocess output with Spirit provenance | 19-1, 19-3 | LIVE-VERIFIED | `smoke_cli_wrapper_8_12.rs:83-153`; `worker_spawn.rs:1145-1181` | none |
| FR57 | Telemetry-evidenced skill revisions enter admission | "E18 preflight" | PARKED | `main.rs:13150-13164`; `crates/maos-skill/src/store.rs:50-75` payload not persisted | ADD `18-7` |
| ADR-016 | Self-reported context pressure, limits, exhausted-tool refusal | 18-5 | OWNED | `epic-18:104-107`: "above 100 % returns `EContextExhausted`" | none |
| NFR-Test-4 | Recall/precision floors per Spirit class | 18-2, 21-1 | PARTIAL | `epic-18:77`: Butler only | widen `18-2` |
| NFR-Aud-7 | Five metric gates per distillation-shipping Spirit | 18-3 | PARTIAL | `epic-18:86`: faithfulness/hedge "never a CI floor" | widen `18-3` |
| J-Butler | Anticipation, integrations, episodic learning, supervised self-tuning | 18-1, 18-4 | PARTIAL | `epic-18:62,93-94`; private persistence deferred | widen `18-4` |
| J-Researcher | Survey, contradiction halt, structured findings, citations | 18-3 | PARTIAL | `epic-18:85-87`: replay/errors/metrics only | widen `18-3` |
| J1 | Skill-directed parallel work, clarification, continuation, overnight digest | 19-4 | PARTIAL | `epic-19:78,84-88` sequential reduced scene; `sprint-status.yaml:286` | widen `19-4` |
| FR31 | Principal-namespace persistence | unmapped | PARTIAL | `epic-18:94`: "principal-namespace persistence is deferred" | widen `18-4` + `20-3b` witness |
| FR51(c) | Recall pending buffered instructions | map 16-1; actual 19-2 | OWNED | `epic-19:70` | routing → `19-2` |

## Appendix B — Epic 20 / 21 / operator-lane / parking slice

| Req | PRD clause | Routed home | Verdict | Evidence | Proposed action |
|---|---|---|---|---|---|
| FR1 | Package-manager, cargo, or signature-verified install | 15-4; 20-2; ops-brew-tap | PARTIAL: RPM/cargo unowned | `epic-20:84-85`; `packaging/rpm/maos.spec:3-6` | widen `20-2` |
| FR3 | Six providers per Spirit with endpoint locking | 20-4; later-bedrock | PARTIAL: Kimi, per-Spirit selection | `epic-20:152-154`; `sprint-status.yaml:323` | widen `20-4` |
| FR23b | Cross-host PKI, rotation, revocation, skew, partition | 21-3 | PARTIAL: no ±5 min skew, ≤10 s partition, CRL closure | `epic-21:72-77`; `08-fr21-65-scout.md:3` | widen `21-3` |
| FR28 | Tiered persistence and quotas | 21-6 | OWNED | `epic-21:105-108` | none |
| FR35 | Publish signed schema-conformant packages | 20-1 | OWNED | `epic-20:20,76` | none |
| FR36 | Authenticated install verifies signatures, trust floors, claims | 20-1 | PARTIAL: authenticated control-plane install | `epic-20:76,79` | widen `20-1` |
| FR37 | Signed, journaled vetting promotion with revocation semantics | 20-1 | PARTIAL: lapse/revocation consequences tracker-only | `epic-20:77-78`; `sprint-status.yaml:289` | widen `20-1` |
| FR59 | Yanks synchronize ≤5 min with operator policy | 20-1 | PARTIAL: bounded sync, vetter-origin propagation | `epic-20:78` | widen `20-1` |
| FR48 | Operator-selectable crypto across three surfaces | later | PARKED; PRD v0.5 | `requirements-inventory.md:564`; `functional-requirements.md:116` | DECIDE-1 → RULED split: `20-13` (v1.0 mechanism), FIPS module v1.5 |
| FR54 | Gateway lifecycle and scope contracts | v2.5 | PARKED; parks kernel machinery | `requirements-inventory.md:565`; `functional-requirements.md:68,119` | DECIDE-3 → RULED keep v2.5 + PRD DELTA-2026-10-25; no story |
| FR64 | Cost attribution by Spirit/task/principal | later | PARKED; PRD v1.0 | `requirements-inventory.md:570`; `functional-requirements.md:122` | DECIDE-2 → RULED split: `20-14` (v1.0 per-task + per-principal tokens), CPU/storage-I/O v1.1 |
| NFR-Sec-5 | 24 h manifest fuzz | 20-3a; 20-5 | OWNED | `20-3a-gate-honesty.md:92` | none |
| NFR-Ops-9 | Hourly RPO, 4 h RTO, weekly Merkle verify | 20-3a | PARTIAL: operational RPO | `20-3a…md:30,89-91`; `check_rto.rs:23-26` | widen `20-3c` |
| NFR-Meta-3 | Generated coverage report blocks uncovered requirements | 20-3b | OWNED | `epic-20:114-115` | none |
| NFR-Onb-1 | Twelve external authors meet timing floors | ops-external-cohort | PARTIAL: ≥3 ≠ N=12 | `sprint-status.yaml:310`; `non-functional-requirements.md:125` | widen ops row |
| NFR-Test-8 | Twelve externals ship ten signed Spirits | ops-external-cohort | PARTIAL | `non-functional-requirements.md:85` | widen ops row |
| NFR-Sec-13 | 3-/10-host rotation loses no conversations | 21-3 | PARTIAL: no scale/percentile acceptance | `epic-21:73`; `14-2d…md:416-430` | widen `21-3` |
| NFR-Rel-6 | Restart invalidates TOFU until re-pin | 21-3 | OWNED | `epic-21:72` | none |
| NFR-Maint-1 | Trusted core ≤20 KLOC | 21-4 | PARTIAL: instrument, not bound | `epic-21:83-87`; `07-nfr-scout.md:7` | DECIDE-4 → RULED: ADR-069 envelope ≤23,000 |
| NFR-Ops-12 | Air-gapped substrate runs and journals | 20-2 | PARTIAL: witness runs `--version` only | `epic-20:84`; `tests/air-gap-netns-corroborate.sh:51-55` | widen `20-2` |
| J3 | Eight-host digest and conflict halts | 21-2 | PARTIAL: fixture regeneration | `epic-21:64-65`; `user-journeys.md:205-225` | widen `21-2`; DECIDE-5 |
| J4 | Cross-host diagnosis to deployed fix ≤90 min | 21-1 | PARTIAL: same-process; proposal = fix | `epic-21:54,56`; `user-journeys.md:169-195` | widen `21-1`; DECIDE-5 |
| Reza | Cross-team proposal, provenance, timed halt, approval | 21-2 | PARTIAL: no live loop | `epic-21:64`; `user-journeys.md:235-252` | widen `21-2`; DECIDE-5 |
| J6 | Independent author builds, publishes, installs, gets vetted | 20-1; ops | PARTIAL: independence unproven | `epic-20:76-79`; `sprint-status.yaml:310` | widen ops row |
| Tech: invariants | I1–I14 predicates + property tests in CI | none | PARKED | `success-criteria.md:38`; `docs/invariants/README.md:7` | widen `20-3b` (guaranteed/asserted ledger) |
| Tech: adversarial | Malicious Spirits attempt every invariant | none | PARKED | `success-criteria.md:39`; `20-3a…md:70-79` | widen `20-3b`; Hold 1 |
| Tech: performance | Hot-path budgets gate CI | 20-7 | PARTIAL: 5 of 6 budgets missing | `success-criteria.md:40-46`; `epic-20:186-193` | widen `20-7` |
| Tech: protocols | ACP + MCP + A2A in one session | none | PARKED | `success-criteria.md:50` | ADD `21-8` |

## Appendix C — "already LIVE" set and FRs routed to done stories

| Req | PRD clause | Routed home | Verdict | Evidence | Proposed action |
|---|---|---|---|---|---|
| FR8 | Complete signed manifest declarations | unchanged | PARTIAL | `05-fr1-20-scout.md:10`: skills, explanation_shape, intent_promotion_set, swap_invariants missing; unsigned admission | ADD `20-8` |
| FR30 | Distillate audit chains | unchanged | LIVE-VERIFIED | `smoke_epic_4.sh:45-50` | none |
| FR31 | Isolated principal namespaces | unchanged | LIVE-UNVERIFIED | adapters instantiated, no isolation witness | widen `20-3b` |
| FR32 | Predicate-triggered structured halts | unchanged | LIVE-VERIFIED | `smoke_epic_4.sh:19-34` | none |
| FR34 | SDK fixtures and external-author coverage | unchanged | LIVE-UNVERIFIED | no binary-level witness | widen `20-3b` |
| FR53 | Halt identity across swaps | unchanged | LIVE-UNVERIFIED | `one_daemon_one_door_16_1.rs:957-961` asserts re-firing, not continuity | widen `20-3b` |
| FR56 | Principal-scoped self-telemetry | unchanged | LIVE-VERIFIED | `smoke_epic_4.sh:19-42` | none |
| FR60 | Offline signed-artifact import | unchanged | LIVE-VERIFIED | `subcommands.rs:1228-1355` | none |
| FR63 | Stable catalogued typed errors | unchanged | LIVE-UNVERIFIED | no catalog/retryability/cause-chain witness | widen `20-3b` |
| FR2 | Uninstall without orphaned state | 16-4 | PARTIAL | `16-4…md:423-424`: containers/cgroups reported, not removed | ADD `20-10` |
| FR4 | Token-mediate every external call | 17-1 | PARTIAL | `17-1…md:111-121`: three refusals, not 1000-call proof | widen `19-3` |
| FR47 | All inference through the Inference Port | 17-1 | narrowed by ADR-061 | `17-1…md:19` | PARK (ruling exists) |
| FR5 | Sandbox floors and declared-scope confinement | 17-1/2/3b/3c | PARTIAL | `sprint-status.yaml:272` | `17-11` (exists) |
| FR6 | Per-Spirit CPU/memory/FD caps | 17-2 | PARTIAL: Linux only | `17-2…md:127` | PARK v1.5 |
| FR7 | Opt-in anonymous telemetry | non-goal | PARKED | `requirements-inventory.md:546` | keep |
| FR9 | load/start/pause/resume/unload | 16-1/16-6 | OWNED | `16-6…md:551`; `16-1…md:371-384` | none |
| FR13 | Declared revocation policies | 16-1 | PARTIAL: quarantine lacks isolation | `17-1…md:143` | widen `20-1` |
| FR16 | Posture shifts within latency floors | 16-1 | PARTIAL | `16-1…md:378-382`; `epic-20:190` | `20-7` |
| FR24 | Postures and preserved intent provenance | 16-1 | PARTIAL | `16-1…md:378-382` | widen `19-1` |
| FR51 | Bounded director interruption, preservation, recall, revocation | 16-1 | PARTIAL: (a)(d) absent | `16-1…md:35-42`; `sprint-status.yaml:284,481-483` | widen `19-2` |
| FR10 | State/token-preserving upgrades, both forms | 16-1 | PARTIAL: subprocess parity unclaimed | `16-1…md:478-486` | ADD `19-5` |
| FR11 | Reject archived predecessor without migrator | 16-1 | PARTIAL: admission lacks archive check | `05-fr1-20-scout.md:13` | ADD `19-5` |
| FR49 | Hot, cold, migrator upgrade policies | 16-1 | PARTIAL: `ColdSwapUnsupported` | `16-1…md:43-46,484-486` | ADD `19-5` |
| FR12 | Detect crashes, notify originators | 16-3 | PARTIAL: journaled, never delivered | `16-3…md:35-37` | ADD `19-6` |
| FR50 | All declared crash dispositions | 16-3 | PARTIAL | `main.rs:2795-2796` `NullReplicaResolver` | ADD `19-6` |
| FR15 | Three halt resolutions and per-class floors | 16-2 | PARTIAL: J0 only | `16-2…md:25-26` | `18-2` |
| FR58 | Zero-config reference response | 16-2 | PARTIAL: hello/J0 only | `16-2…md:25-34` | `18-1` |
| FR29 | Participant recall honoring production consent | 17-3d/17-8 | PARTIAL: no production granter/gate; native pid 0 | `17-8…md:45,114` | ADD `21-7` |
| FR33 | Language-specific templates | 17-3b | PARTIAL: TypeScript only | `17-3b…md:25,177-180` | PARK v1.5 |
| FR55 | Budgeted lifecycle triggers | 17-4 | PARTIAL: spawned forms excluded | `17-4…md:51,135` | ADD `20-9` |
| FR61 | Disclosure policy with GPG key | unchanged | PARKED (ADR-047) | `SECURITY.md:11-22` | keep |
| FR62(b) | Audit-queryable ABI proposals | 17-3b | PARTIAL: no TL emitter | `17-3b…md:37,252-253` | widen `20-3c` |
| FR65 | Complete per-Spirit erasure inventory | 16-4 | MISROUTED | `main.rs:9290-9305`: lineage `VerifiedEmpty` by assumption; schedules `CoverageGap` | ADD `20-11` |
| NFR-Rel-1 | Crash detection and orphan-frame deadlines | 16-3 | PARTIAL: no IAC delivery | `16-3…md:35-37,297` | ADD `19-6` |
| NFR-Rel-2 | Reclassify hung Spirits ≤60 s | 16-3 | OWNED | `16-3…md:297,689-695` | none |
| FR18/22/23a/26/27/38/40/42/43/44/46 | (as Appendix A) | unchanged | LIVE-VERIFIED | Appendix A rows | none |
