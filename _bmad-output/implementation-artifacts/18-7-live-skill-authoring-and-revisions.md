---
story_id: '18.7'
story_key: 18-7-live-skill-authoring-and-revisions
created: 2026-10-26
context_commit: 8fdf497f
baseline_commit: 8fdf497f
security_sensitive: true
kernel_grant: "◆ FLAG-Winston, RATIFIED 2026-10-26 by operator Lunarpulse: D-18-7-01 A + D-18-7-02 A = +5 physical / +4 tokei over LANDED 18-6 pins: 26362 → ≤26367 physical (95 files), 20606 → ≤20610 tokei. Exactly capability/cap_policy/decision.rs, capability/cap_policy/mod.rs, capability/mod.rs, scheduler/scheduler_loop.rs; Intent surface re-pin only. Original creation figures 26407/20650 are historical, not landing limits. Exact measured pins after cargo fmt --all; any other kernel file/surface or excess requires a new FLAG ruling."
kloc_grant: "D-18-7-15 resumed operator addendum: maos-bin ≤29500, maos-skill ≤1350 (supersedes 29150/1138 and intermediate bin29300); exact measured landing pins. Other re-derived T0 STOPs unchanged: manifest≤4629, CLI≤6198, IAC≤8295, audit≤7658; zero growth domain9975/capability1432/wasm-host2412/SDK1085; control<1791, journey<1104, FKCS<591, ABI<3000. Main approved mandatory schema-v6 inventory xtask46363→46364 (+1) under operator D-02 A; reserved ceilings retained. _aggregate_hardfail = measured +1; per-file attribution in Dev Agent Record."
review: "bmad-code-review under docs/testing/proof-contract.md. Difficult story (◆ kernel FLAG-Winston grant + security-critical authoring/admission ACs): creation anthropic/claude-opus-5-5; dev/finisher openai-codex/gpt-6.1-sol; Blind, Edge and Acceptance on opus-5.5, Test Infrastructure on glm-5.3, plus a security-focused reading of AC1–AC3 (who may author, propose and apply; pid fixing; evidence provenance; approval-target identity; single writer; restart availability). FLAG-Winston signs off the kernel diff. fable-5.1 only with operator permission."
---

# Story 18.7: Live skill authoring and telemetry-evidenced revisions

Status: review

<!-- Created 2026-10-26 at 8fdf497f by bmad-create-story under the lighter creation contract (no party-mode preflight). 18-0…18-4, 18-8 done; 18-5 done pending one CI marker pin; 18-6 implemented and under review, its code treated as landing. Last story of Epic 18. Direct reads of every decision-bearing range; the host's only cargo process (CARGO_TARGET_DIR=/mnt/build/cargo, `env -u CI -u GITHUB_ACTIONS`) ran `check-kernel-baseline` (PASSED 26407 / 95) and `kloc-check --json` (passed, aggregate 186040) at HEAD, two throwaway prototypes (D-01 A alone; D-01 A + D-02 A with its workspace ripple), `cargo check --locked --workspace`, `check-service-boundary` and `kloc-check` on each, then `git checkout -- crates` (tree clean, `check-kernel-baseline` PASSED 26407 again). FLAG record `18-7-flag-winston-decision.md`. RULED 2026-10-26 by the operator (Lunarpulse), relayed by Main: D-18-7-01…16 all as recommended (06 B, 13 B, 16 B; the rest A); kernel grant +5 / +4 ratified; maos-bin named budget 28914 → 29150 → `ready-for-dev`. Rulings applied docs-only (no cargo). -->

> **The capability:** *a live Spirit learns from its own performance on the production path. Holding a minted `skill.author.self` capability, the Researcher writes a `maos.skill.v1` skill into the operator-admission queue, and after a pass it reads its own telemetry and proposes a revision of that skill carrying the target id and version, the diff and the evidence the root attached. Nothing activates until the operator admits it over the authenticated door; everything — the skill text, the diff, the evidence, every decision — survives a daemon restart and is referenced by the audit rows. A Spirit that did not declare the capability, or that reaches outside its own skills, is refused, typed and journaled.*

**Closes:**
- PRD FR39 (`prd/functional-requirements.md:89`): `skill.author.self` at runtime, landing pending operator admission — on the production path, not the `MAOS_ONE_SHOT=smoke-skill-7-4` demo.
- PRD FR57 (`prd/functional-requirements.md:91`): telemetry-evidenced revision proposals carrying (a) target id/version, (b) diff, (c) evidence, entering the same admission queue under the same vetting and audit. Consumes FR56 (`:79`) through the existing `SelfTelemetryAggregator`; no new telemetry plumbing.
- Architecture ADR-024 "Spirit-authored skills" gate (`architecture-maos-minimal-opus/12-architecture-decision-records.md:385-395`: "`skill.author.self` capability + operator-admission queue operational"). Not `docs/adr/ADR-024-…` (that number is the sandbox-escape detector; numbering collision recorded, not fixed).
- Recovery Coverage Map `epics/requirements-inventory.md:562` (holds; routed by `:590`); routing audit `routing-audit-2026-10-22.md:19,97,106`; coverage-matrix rows FR39 / FR57 (`tests/coverage-matrix.yaml:631-642`, `:805-817`), whose only gate today is the demo.
- **Inherited `deferred-work.md` rows owned by this story: none** (grep `18-7` and `18-7-live`: zero owner rows). Rows inside the blast radius, adopted at origin if ruled (rule 10 (a)): `:485` `from_stored` rewrites unknown labels to `PackageShipped` (D-18-7-10); `:479` `/tmp` fallback for `queue.json` (D-18-7-14); `:488` provenance fidelity — its constraint names exactly this story's daemon-written enqueue rows (AC3.4); `:651` FKCS admission-path pin covers `crates/maos-skill/src/admission.rs` (D-18-7-10). Adjacent, recorded not inherited: `:484` unauthenticated `actor` (narrowed by D-18-7-07 A, not closed), `:480` `default_transparency_log_path` exit, `:979` XDG resolvers (`21-4-one-instrument-and-env-registry`), `:118` unknown-field string coupling, `:1156` `serde_yaml`.

**Does NOT close, and says so (rule 11: stated as limits):**
- **Skill consumption.** No Spirit loads skills at `on_load` at HEAD (`crates/maos-skill/src/lib.rs:16-17`; architecture `5-spirit-abi.md:207`); an admitted skill is discoverable, not executed. The manifest `skills` field is `20-8-manifest-contract-completion`'s.
- **WASM / TypeScript Spirits.** WIT `maos:spirit@2.2.0` imports only `recall` and `context` (`wit/spirit.wit:30,378-381`); under D-18-7-11 A only native Spirits get the port.
- **Content-blind privacy.** The payload check (D-18-7-16) finds only principal ids indexed to the authoring pid, token-bounded; a later `maosctl forget` does not reach the queue file (`20-11-erasure-inventory-closure`).
- **Operator identity.** The door bearer authorizes the decision; the recorded `actor` stays a label (`deferred-work.md:484`).
- **Revision auto-apply** (unless D-18-7-06 C is ruled) and bulk admission (ADR-024 revisit clause).

## Story

As **a director who runs a Spirit that should get better at its job**,
I want **the Spirit to write and revise its own skills from its own performance, with every change waiting for my decision and recorded so I can see exactly what it proposed and why**,
so that **"actors learn from each performance" is real on my daemon, and no Spirit changes its behaviour, or another Spirit's, without my say.**

**Contract:**
- Epic-18 §18-7 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:243-256` after the in-place corrections and rulings; creation corrections `:247`); stories row `:120`; §R R18-21 `:94` (tracker `:283`, edge 18-1 → 18-7); Kernel-Δ `:32` (18-7 note `:42`); kloc rulings `:58` (maos-bin 28914) and `:60` (18-7: 29150 + STOP lines); Dependencies `:274,280,282`; confidence rules 9–11 (`:62-64`); `:123` (create-story may tighten, not add uncited ACs — four ACs kept).
- PRD FR39 `:89`, FR56 `:79`, FR57 `:91`; inventory `:562,590`; `routing-audit-2026-10-22.md:19,97,106`.
- ADR-024 (architecture `12-…:385-395`), ADR-027 (`docs/adr/ADR-027-skill-package-external-standard-interop.md:19-20,35-42`), ADR-069 envelope; `docs/testing/proof-contract.md` (binding; one plant per AC `:26`).
- 7-4 (the library), 9-7 (durable store, journal-first decide), Epic 10 §A3 (daemon enforcement), 17-11 (entitlement at issuance), 18-1/18-3 (one binding on every construction path), 18-4 (root fixes pids), 18-5 (context exits, weak bus holds, FR4 doorbell), 18-6 (token-bounded principal matcher).

## Current-tree corrections — read before implementing

Measured at `8fdf497f` by direct reads, `check-kernel-baseline`, `kloc-check --json`, two reverted prototypes, a workspace `cargo check` and `check-service-boundary`.

| Premise (epic / §R / audit) | Fact at `8fdf497f` and consequence |
|---|---|
| Why: both paths exist only in the demo — `enqueue_skill` `main.rs:13217`, `enqueue_proposal` `:13243`, queue `:13215`, demo `:13168-13250` | **Holds, re-anchored:** `smoke_skill_7_4` `crates/maos-bin/src/main.rs:13508-13651` (dispatch `:8563-8566`, verb `verbs.rs:190`); in-memory queue `:13547`, `enqueue_skill(…, AuthorSelf, "spirit:1")` `:13549`, `enqueue_proposal(…, "spirit:7")` `:13575`; the report is a literal (`:13562`), the actor strings are literals, no capability is consulted. CI asserts 6 step lines (`.github/workflows/discipline.yml:3455-3473`). → D-18-7-13. |
| Why / AC3: the persisted queue keeps only id/version/entry-path/state (`store.rs:49-74`, "NOT persisted" `:51-52`) | **Holds.** `SCHEMA_VERSION = "maos.skill-queue.v1"` `crates/maos-skill/src/store.rs:43`; `QueueEntry` `:56-65`; `QueueFile` `:69-74`; `PendingEntry.skill` and the proposal live only in memory (`admission.rs:79-90`); `from_stored` sets `skill: None` and maps an unknown label to `PackageShipped` (`:280-296`, fallback `:286-287`, `deferred-work.md:485`). `Skill` and `SkillRevisionProposal` derive no serde (`schema.rs:94-101`, `proposal.rs:21-31`); `SelfTelemetryReport` does (`crates/maos-domain/src/self_telemetry.rs:14`). → D-18-7-04. |
| AC2: "Authoring and proposing require a minted capability" | **No mint is possible at HEAD.** `Scope::SkillAuthorSelf` (`crates/maos-domain/src/invariants/i1.rs:112-119`) has a policy arm (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:189-201`) but no `scope_to_intent` arm (`capability/mod.rs:58-111`; fall-through `PolicyDenied` `:106-109`), so `issue_with_mediation` (`:189-250`) refuses it for everyone and `is_mediated_scope` (`:115-117`) calls it unmediated — 17-3c pins it as THE unmediated exemplar (`crates/maos-bin/src/spirit_session.rs:899-928`; `crates/maos-bin/tests/spawned_scope_17_3c.rs:2021,2185`). → D-18-7-01 (◆). |
| (unstated) a Spirit can declare `skill.author.self` | **It cannot.** `[capabilities.required]` has provider/mcp/loom/iac/log only (`crates/maos-manifest/src/manifest.rs:603-609`, raw `:688-699`); `capabilities_required_to_scopes` (`:869-917`, called by kernel admission `crates/maos-kernel-core/src/security/mod.rs:336`) never yields `SkillAuthorSelf`. Capability tables are schema-gated (`degrade_for_schema_version` `:674-683`; v5 = `crates/maos-spirit-abi/src/lib.rs:118`). A new field ripples to the kernel harness literal `scheduler/scheduler_loop.rs:326-332` and three maos-bin literals (`admission.rs:615-625`, `main.rs:7334-7344`, `:13724-13730`) — measured. → D-18-7-02 (◆). |
| AC1: the operator's admit/deny decision references the payload | **Identity collision.** The approval target is `id@version` (`crates/maos-skill/src/approval_target.rs:13-15`) for a new skill AND for a revision of it (`admission.rs:162-174`); `admission_view` drops a stored entry whose id is on the search path (`crates/maos-cli/src/subcommands.rs:1121-1129`), so a revision of a discovered skill cannot be addressed, and approving it would write the base skill's target. → D-18-7-05 (security). |
| AC1: decisions after a restart | `maosctl skills approve/reject` is offline: it opens the TL directly and rewrites `queue.json` (`subcommands.rs:991-1031`, `decide_skill` `:1161-1226`, cache write ignored `:1224`); `actor` is `--actor`/`$USER`/`"operator"` (`:885-890`). A daemon writing submissions to the same file races it (lost update). The door has bearer auth and a typed command enum (`crates/maos-control/src/lib.rs:270`; route pattern `:1241-1259`) and no skills verb. → D-18-7-07 (security). |
| (unstated) a rejected skill is inert | **It bricks the next boot.** Daemon enforcement counts every entry of the view, including queue-only ones (`main.rs:4342-4417`; FATAL on any `Rejected` `:4387-4404`). Once Spirits enqueue, an operator rejecting a Spirit-authored entry would stop `maos run` from starting. → D-18-7-12 (availability). |
| (unstated) evidence is the kernel's | The domain port fixes the pid kernel-side (`crates/maos-domain/src/ports/self_telemetry.rs:9-26`); `build_proposal` accepts any report the caller hands it (`proposal.rs:40-70`). Telemetry is keyed by boot-local pid with no admission instant on the SCB (`crates/maos-kernel-core/src/memory/self_telemetry.rs:107-218`; `scheduler/control_block.rs:258-280`). Root aggregator `main.rs:2582-2589`. → D-18-7-08 (security). |
| (unstated) an authored skill stays inside its author's authority | `SkillManifest.required_capabilities` is a free set (`schema.rs:72-73`, `CapabilityId(String)` `maos-spirit-abi/src/compliance.rs:243`) and ids are global; nothing ties an authored skill or a revision target to its author. → D-18-7-03 (security). |
| AC1: "a live Spirit" | No Spirit authors at HEAD. The Researcher has the one construction path every variant passes (`crates/maos-bin/src/researcher_root.rs:109-166`; boot `main.rs:5735-5741`, door load `:1037-1051`, successor) and writes Spirit-authored artifacts through root-bound ports (`bind` `:188-205`). → D-18-7-09. |
| (unstated) admission touches only the queue | `crates/maos-skill/src/admission.rs` is one of two files hashed by `check-fkcs`'s `admission-path-unmodified` leg (`xtask/fkcs-baseline.toml:36-41`; every mismatch blocks). → D-18-7-10. |
| 18-5 context exits | `is_exit` admits only `ProviderInfer`, `MemWrite`, `IacSend` (`crates/maos-capability/src/context_budget.rs:54-59`; `admits` `:110-112`): an exhausted Spirit's `SkillAuthorSelf` mint is refused `ContextExhausted` (`capability/mod.rs:213-218`) — intended; AC2 witnesses it. |
| Queue location | `~/.local/share/maos/skills/queue.json` from `$HOME`, `/tmp` when unset (`store.rs:131-150`, `:236-240`; `deferred-work.md:479`); discovery roots `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/` (`crates/maos-skill/src/discovery.rs:40-49`; architecture `5-spirit-abi.md:69-70`); flat `*.md` and `dir/SKILL.md` only. Witnesses need a scratch `HOME`. |
| Tracker `sprint-status.yaml:282`; audit `main.rs:13135-13164`, `store.rs:50-75` | **Stale.** Tracker `:283` (R18-21 already said so); the audit's cites are @3a7c4260 — outside this story's doc set, recorded, not edited. |
| Holds | FR39 `:89`, FR57 `:91`, FR56 `:79`; inventory `:562`; audit `:19`; `proof-contract.md:26`; `store.rs:56-74`; `ApprovalDecision` writer `crates/maos-iac/src/adapter/transparency_log.rs:1985`, reader `query_approvals` `:2371`; `principal_ids_for_spirit_pid` `:1489`; `principal_id_occurs` `crates/maos-domain/src/memory.rs:454`. |

**Epic cites corrected in place (OLD → NEW @`8fdf497f`):**
- §18-7 intro: tracker `sprint-status.yaml:282` → `:283`.
- §18-7 Why: `main.rs:13217` → `:13549`; `:13243` → `:13575`; `:13215` → `:13547`; demo `:13168-13250` → `:13508-13651` (dispatch `:8563-8566`). `store.rs:49-74` and "NOT persisted" `:51-52` hold.
- §18-7 Closes · Δ (now `:249`) and stories row (`:116` → now `:120`): "kernel-Δ 0 [INFERENCE, measured at creation]" → measured: kernel 0 only without a minted capability; the ruled stack is ◆ FLAG-Winston **+5 physical / +4 tokei** (D-18-7-01 A + 02 A, RULED and ratified 2026-10-26).
- §18-7 AC1 "A test named at creation" → `crates/maos-journey-test/tests/journey_skill_authoring_18_7.rs`.
- §18-7 AC2 gains the HEAD fact (unmediated, undeclarable) and the D-01/D-02 pointer; AC3 "schema bump measured at creation" → `maos.skill-queue.v1` → `v2` (D-04). AC4 holds.
- Kernel-Δ paragraph `:32` "18-0, 18-2, 18-4 and 18-7 are kernel-Δ 0 by design" → 18-7 struck, measured ◆ request (new note `:42`, after the 18-6 note `:40`).

## Kernel and kloc pricing (E16-A7) — measured at creation

**Prototypes (throwaway, at `8fdf497f`, `cargo fmt --all` clean, reverted with `git checkout -- crates`; `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exit 0; `check-kernel-baseline` PASSED 26407 / 95 after):**

| Option | Kernel physical (`check-kernel-baseline`) | Kernel tokei (`kloc-check`) | Files | Other measured |
|---|---:|---:|---|---|
| D-01 A alone (`Intent::SkillAuthorSelf` + 2 arms) | 26407 → **26411 (+4)** | 20650 → **20653 (+3)** | `cap_policy/decision.rs`, `cap_policy/mod.rs`, `capability/mod.rs` | `cargo check -p maos-kernel-core` clean; `check-service-boundary` FAILED: `Intent` signature hash (re-baseline item) |
| **D-01 A + D-02 A (recommended stack)** | **26412 (+5)** | **20654 (+4)** | + `scheduler/scheduler_loop.rs` (harness literal) | `cargo check --locked --workspace` clean after maos-bin's 3 literal ripples; maos-manifest 4583 → **4605 (+22)**; maos-bin 28914 → **28917 (+3)**; aggregate 186069 |
| D-02 A / B alone | +1 | +1 | `scheduler/scheduler_loop.rs` | B differs from A only outside the kernel (no schema bump) |
| D-01 B, D-02 C, D-03…16 | 0 | 0 | none (by construction: root, maos-skill, maos-manifest, maos-cli/control, spirits) | — |

Test-only ripple (not in the kernel baseline, counted for dev effort): 7 files build `CapabilitiesRequired` literally (`crates/maos-bin/tests/researcher_log_grant_17_3d.rs`, `crates/maos-kernel-core/tests/{abi_stability_admit,operator_posture_ceiling_16_6,provider_switched_journal,sandbox_admission,manifest_field_coverage}.rs`, `crates/maos-manifest/tests/log_capability_17_3d.rs`); a v6 bump also moves the N-1 window tests (`crates/maos-manifest/tests/manifest_n_minus_1_compat.rs`, `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs`) and the schema-version references in 10 more test files (grep). Under D-01 A, 17-3c's "unmediated" case swaps its exemplar to `GatewaySend` (`spawned_scope_17_3c.rs:2021,2185`); its CI pins stay 23 (`discipline.yml:3890-3891,4032-4036`).

**kloc at `8fdf497f`** (`kloc-check --json`, `passed: true`, `over_budget: []`, aggregate 186040; `alarm: true` is the pre-existing aggregate alarm):

| Crate | Measured / ceiling (`xtask/kloc.toml`) | 18-7 forecast (measured / [INFERENCE] ×1.3 where stated) | Consequence |
|---|---|---|---|
| maos-kernel-core | 20650 / 20650 (`:264`); pin 26407 (`kernel-core-baseline.toml:505`) | **+4 tokei / +5 physical (measured, recommended)** | FLAG (D-01, D-02) |
| maos-bin | 28914 / 28914 (`:517`); Epic 18 named budget 28914 | +3 measured ripple; root skill port (mint+verify, pid fix, evidence, journal, weak holds) +70–110; wiring `main.rs` +10–20; `researcher_root::construct` +4–8; door decide/show handler +35–60 (D-07 A); boot enforcement fix +3–8 (D-12 A); demo steps 3–4 −30–35 (D-13 B; code-read count of `main.rs:13546-13583`, not prototyped) → **net +95–180, ×1.3 ≈ +125–235 [INFERENCE]** | **D-18-7-15** |
| maos-skill | 800 / 900 (`:693`) | v2 store + v1 read +45–70; entry id (sha256) +10; submission vetting (D-03, D-05, D-16) +60–100; `from_stored`/labels typed +15–30; materialize on admit (D-06 B) +20–35; errors +15–25 → **+165–270 [INFERENCE]** (+60–100 more under D-06 C) | measured request (100 headroom) |
| maos-manifest | 4583 / 4583 (`:415`) | **+22 measured** (+0–15 in-src tests) | measured request |
| maos-cli | 6104 / 6104 (`:489`) | decide over the door + offline arm, `skills show` +40–75 [INFERENCE] | measured request |
| maos-control | 1286 / 1791 (`:484`) | `OperatorCommand::SkillDecision` + routes +25–45 | fits |
| maos-iac | 8080 / 8080 (`:406`) | 0–20 (a skill refusal receipt helper beside `record_mcp_refusal`, if homed there) | measured request if > 0 |
| maos-audit | 7625 / 7625 (`:409`) | FR4 `WRITER_SHAPES` rows for each new `insert_frame_event*` site +6–18 | measured request |
| maos-journey-test | 946 / 1104 (`:681`) | 0–25 | fits |
| maos-spirit-abi / sdk | 1042 / 3000; 1085 / 3000 | 0–2 (schema constant 5 → 6 under D-02 A) / 0 | fits |
| maos-fkcs | 491 / 591 (`:677`) | 0–10 (API ripple from `SkillAdmissionQueue`) | fits |
| maos-domain, maos-capability, maos-wasm-host | 9887, 1432, 2412 (all zero headroom) | 0 | — |
| spirits/researcher | ungoverned (`xtask/src/kloc_check.rs:310-311`) | authoring/proposal logic + prompts +120–220 | — |
| aggregate | 186040; `_aggregate_hardfail` 186041 (`:718`) | ≈ +480–760 | re-pin = measured + 1 |

Binary size: release + strip 29,968,248 B at 18-6 (3,586,184 B under 32 MiB); re-measure at landing.

## Decisions — RULED 2026-10-26 by the operator (Lunarpulse)

**Ruling record.** D-18-7-01 **A**, 02 **A**, 03 **A**, 04 **A**, 05 **A**, 06 **B**, 07 **A**, 08 **A**, 09 **A**, 10 **A**, 11 **A** (WASM/TS routed to `20-8-manifest-contract-completion`; received row filed in `deferred-work.md`), 12 **A**, 13 **B**, 14 **A**, 15 **A** (Epic 18 named maos-bin budget 28914 → **29150**; exact re-pin; above → STOP), 16 **B** — every row as recommended, for the rationale in its row. Relayed by Main; recorded the same day by `anthropic/claude-opus-5-5` (docs only, no cargo). ◆ **FLAG-Winston kernel grant ratified (ADR-069 envelope):** D-01 A + D-02 A = **+5 physical / +4 tokei** — `check-kernel-baseline` 26407 → **≤ 26412** (95 files), `maos-kernel-core` 20650 → **≤ 20654**, exact measured figure at landing after `cargo fmt --all`; files `capability/cap_policy/decision.rs`, `capability/cap_policy/mod.rs`, `capability/mod.rs`, `scheduler/scheduler_loop.rs` **only**, plus the `Intent` item re-pin in `docs/ci-baselines/kernel-surface-v0.1-beta.json`. Any other kernel file, any other surface item, or a figure above the grant returns as a FLAG request. **18-6's review patches (running at ruling time) may move the pin inside 18-6's remaining grant: T0 re-measures from the landed pin and applies +5 / +4 over it, never widened.** FLAG record: `18-7-flag-winston-decision.md`. CR-n are creation defaults carried into the rulings.

**Out-of-kernel rows — measured requests at landing, STOP at dev above expected + 25 %** (expected = prototype measurement + upper creation forecast; a dev figure above the STOP line → stop and report the exact `kloc-check --json` figure to the operator before any further production edit or re-pin):
- **maos-bin:** ≤ **29150** (D-15 A, named budget); exact re-pin at landing; above 29150 → STOP, re-ask.
- **maos-skill** (800 / 900): expected up to +270 → **STOP above +338 (1138)**; ceiling re-pinned exact.
- **maos-manifest** (4583, zero headroom): expected +22 measured + 15 = +37 → **STOP above +46 (4629)**.
- **maos-cli** (6104, zero headroom): expected up to +75 → **STOP above +94 (6198)**.
- **maos-iac** (8080, zero headroom): expected up to +20 → **STOP above +25 (8105)**.
- **maos-audit** (7625, zero headroom): expected up to +18 → **STOP above +23 (7648)**.
- **maos-control** (1286 / 1791), **maos-journey-test** (946 / 1104), **maos-fkcs** (491 / 591), **maos-spirit-abi** (1042 / 3000): land exact inside their ceilings; reaching a ceiling → STOP.
- **maos-domain** (9887), **maos-capability** (1432), **maos-wasm-host** (2412), **maos-spirit-sdk** (1085): expected 0 → any growth → STOP.
- `_aggregate_hardfail` = final measured aggregate + 1; no speculative headroom.

◆ marks a kernel-touching row (FLAG-Winston). "Security" marks rows that decide who may author, propose or apply. CR-n are creation defaults the rows assume.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D-18-7-01** ◆ security | How authoring/proposing is capability-mediated | **A — kernel: `Intent::SkillAuthorSelf`, `scope_to_intent` and `intent_action_key` arms (+4 / +3, 3 files + surface re-hash); the root port mints one `SkillAuthorSelf` token per submission through `issue_enterprise_governed_capability` (`worker_spawn.rs:386-462`: SSO/PDP → policy → 18-5 context → quota) at the pid the ROOT resolves, verifies it (`verify_and_audit` `capability/mod.rs:304-313`) before the queue write, and journals a typed receipt on refusal; `SkillAuthorSelf` stays non-delegable through `TaskAssign` (`spirit_session.rs:899-928` refuses it typed, so the root port is the only consumer)** · B — kernel 0: the root checks the pid's declared scopes and the operator deny map itself (`policy().inner()`), no token, no quota/context/tier checks, labelled "not minted" (deviates from epic AC2) · C — kernel 0: keep the demo path (no mediation) | **A.** The policy arm exists since 7-4 but is unreachable; only A gives the epic's "minted capability" with the full issuance chain (org deny, 18-5 exhaustion, quota, audit) and no second copy of the policy logic outside the kernel. +4 / +3 measured, ADR-069 envelope 2,350 to 23,000. | **RULED 2026-10-26 — A** (grant ratified) |
| **D-18-7-02** ◆ | How a Spirit declares `skill.author.self` | **A — schema v6 `[capabilities.required.skill] author_self = true` → `Scope::SkillAuthorSelf`; degraded below v6; `MANIFEST_SCHEMA_VERSION` 5 → 6; docs manifest v6 page en + ko (kernel +1 / +1 literal; maos-manifest +22 measured)** · B — the same table inside v5 without a bump (same kernel line; breaks the degrade rule `manifest.rs:669-683`) · C — kernel 0: a top-level `[skills]` table read by the root, which inserts the scope into the live policy after admission (a second grant path outside admission) | **A.** Capability tables have always been schema-gated (v4 loom, v5 iac/log); C reopens the grant-outside-admission hole 17-11 closed. Cost: N-1 tests move and the v6 docs page. | **RULED 2026-10-26 — A** (grant ratified) |
| **D-18-7-03** security | What a Spirit may author or propose against | **A — own namespace: authored ids must start `<class>.` (class from the pid's SCB `spirit_id`, never from the Spirit); a revision target must be an entry this class authored (queue provenance) at its current admitted version; every `required_capabilities` entry must equal the `scope_action_key` of a scope the author declares (no escalation through a skill); refusals typed (`ForeignNamespace`, `TargetNotOwned`, `StaleTargetVersion`, `CapabilityEscalation`)** · B — any id, any target; the operator is the only gate · C — targets listed in the manifest (needs 20-8's `skills` field first) | **A.** ADR-024's loop is "the Spirit improves its own skill library"; B lets any Spirit propose rewrites of skills other Spirits or the operator ship. Keyed on class name, not pid (pids are boot-local; 20-3b). | **RULED 2026-10-26 — A** |
| **D-18-7-04** | Where the payload lives | **A — `maos.skill-queue.v2`: each entry carries `entry_id`, `author {class, pid, boot_nonce}`, the full skill document (AuthorSelf) or `{target_id, target_version, diff, evidence: SelfTelemetryReport}` (revision) and `payload_sha256`; v1 files read (label-only entries kept), written back as v2; unknown versions still hard-fail; a v2 file that fails to load is a typed refusal on submit/decide/show and is never overwritten** · B — the payload as a TL frame at the author pid; the queue holds `{entry_id, frame_id, sha256}` (18-6 forget reaches it by construction) · C — content-addressed sidecar files beside a v1 queue | **A.** The epic names `QueueEntry`/`QueueFile` (AC3); one atomic file with the existing helper (`store.rs:184-229`). B couples admission state to TL redaction (a scrubbed submission loses its payload) and needs new TL writers. The queue stops being a rebuildable cache, so `:1224`'s ignored save and the "proceed from discovery" warnings (`subcommands.rs:895-924`, `main.rs:4353-4361`) become typed failures for payload entries. | **RULED 2026-10-26 — A** |
| **D-18-7-05** security | Entry identity and the approval target | **A — `entry_id` = first 16 hex of sha256(canonical payload); new skills keep target `id@version`; revisions use `id@version#<entry_id>` (`#`, `@` are outside the `SkillId` charset, `schema.rs` / `proposal.rs:49-57`); `maosctl skills approve|reject|show <entry_id or id>`; `admission_view` keeps every queued entry (no on-path drop)** · B — today's one pending entry per id; the collision labelled · C — revisions target the proposed next version `id@<next>` | **A.** At HEAD approving a revision writes the base skill's target, and a revision of a discovered skill is invisible (`subcommands.rs:1121-1129`). C collides when two proposals name the same next version. | **RULED 2026-10-26 — A** |
| **D-18-7-06** security | What admission does | A — decision only; content via `maosctl skills show`; nothing materialized · **B — admitting an AuthorSelf entry writes its document atomically to `~/.maos/skills/<id>.md` (one file per id; a foreign file with that name refuses typed `MaterializeCollision`, never overwritten); revisions are decision-only (diff opaque, §4.0.7) and the author submits the revised skill as a new AuthorSelf version, itself admitted** · C — B + an admitted revision is applied by a strict in-house unified-diff applier (no fuzz) to the admitted target text and materialized as the new version (maos-skill +60–100 more; no new dependency — `Cargo.lock` has no diff crate) | **B.** Only the operator's admit makes content discoverable, and the operator reviews exactly what lands; C makes the reviewed diff the landed text but adds a patch engine to a crate that promises not to interpret content (`lib.rs:10-17`). | **RULED 2026-10-26 — B** |
| **D-18-7-07** security | Who decides, and the single writer | **A — door verbs `POST /v1/skills/decide` and `GET /v1/skills[/{entry}]` (bearer-authenticated; `OperatorCommand::SkillDecision`); while a daemon runs it is the only writer of `queue.json` and the decision row; `maosctl skills` uses `durable_verb` (`subcommands.rs:171-185`) with the existing offline arm when no daemon answers; the Spirit-facing port has no decide method** · B — offline only, both writers take an exclusive `File::lock` (std, rustc 1.98) on the queue · C — as today (lost-update race labelled) | **A.** The door's bearer is the one operator authorization MAOS has; B serializes writes but leaves any local process that can run `maosctl` able to decide. The `actor` label stays forgeable offline (`deferred-work.md:484`, narrowed). | **RULED 2026-10-26 — A** |
| **D-18-7-08** security | Where the evidence comes from | **A — root-attached: the Spirit asks to propose; the port reads `SelfTelemetryAggregator::self_telemetry(pid, since)` for the ROOT-fixed pid with `since` = the port's bind instant (recorded at `construct`, so another boot's pid never aliases in) and stores it verbatim with its sha** · B — the Spirit supplies a `SelfTelemetryReport`; the root checks `spirit_pid` only (fabrication possible, labelled) | **A.** FR57's evidence is "its own performance telemetry"; a Spirit-built report is self-asserted. | **RULED 2026-10-26 — A** |
| **D-18-7-09** | Which live Spirit authors, and from what | **A — the Researcher, model-authored through its bound Inference Port (per-pass mint, 18-5 context reported; strict cassette in CI)** · B — Butler (halt evidence; its prompts carry FR31 principal data) · C — the Researcher from a deterministic Spirit-side template (no inference) | **A.** One construction path already proven on boot/door/successor; distillation outcomes are FR56 fields; seed-claim replay is non-personal. C proves the plumbing but not "learns from performance". | **RULED 2026-10-26 — A** |
| **D-18-7-10** | The FKCS admission pin | **A — change `admission.rs` (typed `from_stored`, serde on entries, no `PackageShipped` fallback) and re-pin `fkcs-baseline.toml:41` in the same commit after review, per its rule** · B — keep `admission.rs` byte-identical; new types in a new module; `:485` stays open | **A.** The lossy fallback is in the blast radius (rule 10 (a)); the pin's own comment prescribes the re-pin. | **RULED 2026-10-26 — A** |
| **D-18-7-11** | Third-party (WASM/TS) reach | **A — native Spirits only; labelled; routed to `20-8-manifest-contract-completion` with the `skills` field** · B — WIT 2.3.0 `skills` import (author/propose), host bridge, TS SDK, generated ABI docs (maos-wasm-host has zero headroom) | **A.** No third-party Spirit consumes skills yet; B is a WIT bump for a surface with no consumer. | **RULED 2026-10-26 — A** (received row filed for 20-8, `deferred-work.md` §18-7 rulings) |
| **D-18-7-12** availability | Queue-only rejected entries at boot | **A — boot enforcement blocks only on discovered (on-path) `Rejected` skills; queue-only rejected entries are terminal, listed, inert** · B — a reject prunes the entry (payload and evidence lost) · C — keep (a rejected Spirit draft stops `maos run`) | **A.** `main.rs:4387-4404` would turn every operator rejection of a Spirit draft into a boot failure; B destroys the audit payload AC1 requires. | **RULED 2026-10-26 — A** |
| **D-18-7-13** | `smoke-skill-7-4` steps 3–4 | A — keep, relabelled "library smoke, not the production path" · **B — retire steps 3–4 (the smoke-only enqueue the audit flagged); CI asserts 4 lines; FR39/FR57 coverage rows re-pointed to this story's witnesses** · C — retire the whole demo | **B.** Cutover rule; the coverage rows claim "capability-gated authoring" from a path with no capability check (rule 11 (a)). Steps 1, 2, 5, 6 still witness schema, discovery, output-shape and LCAS. | **RULED 2026-10-26 — B** |
| **D-18-7-14** security | `queue.json` with `HOME` unset | **A — the store refuses typed (`NoDataHome`) instead of `/tmp/.local/share/maos/skills/` (`deferred-work.md:479`)** · B — keep the fallback | **A.** With payloads persisted, a world-writable fallback lets any local user plant or edit a pending skill. The XDG migration stays 21-4's. | **RULED 2026-10-26 — A** |
| **D-18-7-15** | maos-bin budget | **A — Epic 18 named budget 28914 → 29150; exact re-pin at landing; above → STOP and re-ask** · B — hold 28914: the port becomes closures over maos-skill and the door handler a measured request at landing | **A.** Forecast +125–235 [INFERENCE] after the −30–35 demo cut; the mint, door and scheduler are only in the root. Zero-headroom rows return as measured requests with the STOP lines above. | **RULED 2026-10-26 — A** (maos-bin ≤ 29150) |
| **D-18-7-16** security | Personal data in Spirit-written payloads | A — labelled limit, routed to `20-11-erasure-inventory-closure` · **B — refuse a submission whose document or diff contains any principal id indexed to the author pid (`principal_ids_for_spirit_pid` `transparency_log.rs:1489`, token-bounded `principal_id_occurs` `memory.rs:454`), typed `PrincipalDataRefused`, receipt names no principal** · C — D-04 B (payload in the TL so forget scrubs it) | **B.** The queue sits outside FR45's forget reach; B keeps the author's known principals out at the door for ~+15–25 lines; content-blind cases stay labelled (A's route). | **RULED 2026-10-26 — B** |
| CR-1 | Port shape (root fixes pids) | `SkillAuthorPort` (trait in `spirits/researcher`, implemented in maos-bin as 18-3's `ResearcherMcpPort` is): `author(document) -> Result<Receipt, SkillPortError>`, `propose(target_id, target_version, diff) -> Result<Receipt, …>`, `own_entries() -> Result<Vec<OwnEntry>, …>`. No pid, class or evidence parameter; no decide method. The binder is filled once in the Researcher wiring (`main.rs:4034-4062`) and bound in `researcher_root::construct` (`:109-166`), so the boot arm, door load and successor are all bound. | — |
| CR-2 | Weak holds | The port and its factory hold the capability registry, TL, telemetry aggregator, store and SCB map weakly (18-5 lesson: shutdown audit drains); after shutdown every call returns typed `HostShutDown`, never panics. | — |
| CR-3 | Typed, journaled refusals | Every refusal (undeclared/denied, context-exhausted, quota, D-03, D-16, schema, queue unreadable) returns a typed `SkillPortError` and writes one non-personal receipt `skill.author.refused:<code>` at the Spirit pid (as `record_mcp_refusal`, `worker_spawn.rs:497-503`); nothing enqueued, no proceed-on-refusal. Each new `insert_frame_event*` site gets a `WRITER_SHAPES` row (`crates/maos-audit/src/fr4_classifier.rs:195`; doorbell `crates/maos-audit/tests/fr4_classifier_16_2.rs:598-783`). | — |
| CR-4 | Journal-first enqueue | The enqueue decision row (`skill.admission.enqueue`, actor `spirit:<class>@<pid>`, target per D-05, reasoning = entry id + payload sha, no content) commits before the queue write, mirroring `decide_skill` (`subcommands.rs:1204-1225`); a failed queue write after a committed row is reported typed and reconciled on next load. The decided-set stays keyed on approve/reject rows (`:1040-1053`), never on cached labels (`deferred-work.md:488` constraint). | — |
| CR-5 | Serde | Serde results in `src` are never `.unwrap/.expect/.unwrap_or*` (`check-serde-error-handling`). | — |
| CR-6 | Modes and volume | An unbound (ADR-064 unset-mode, deterministic) Researcher authors and proposes nothing — its v0.5 pass shape holds; under D-09 A the authoring step runs only on a bound pass and at most once per pass. Volume is bounded by the issuance quota (one `SkillAuthorSelf` token per submission, `capability/mod.rs:220-232`) and by one `Pending` entry per authored id / per (target, author). | — |

### Review rulings D-18-7-R1…R5 — operator 2026-10-26

- **R1 (AA-6 / BH-4 / EC-3 / EC-4), A:** skill-queue availability never aborts daemon boot. `NoDataHome` leaves the authoring binder unwired (`queue-unavailable`); corrupt/unreadable queues warn and boot from discovery plus the Transparency Log. Submit/decide/show still refuse typed and never overwrite corruption; offline forget/uninstall/legal-hold children remain independent.
- **R2 (BH-5):** enforce rejected discovered skills on every daemon mode, including door-only roots; retain only the discovered/on-search-path filter, with queue-only rejected authored drafts and decision-only revisions inert.
- **R3 (AA-5):** authoring is non-fatal to a successful survey, retaining `last_output` and typed authoring refusals. Check the trusted `SkillAuthorSelf` declaration before the model completion; each actual submission still mints and verifies its governed token.
- **R4 (EC-5), B:** derive the live queue from `maos_home()` (`<home>/skills/queue.json`), the same home as the writer lock and TL. Validate, journal and consume the former `$HOME/.local/share/maos/skills/queue.json` once under the canonical home lock and a legacy-directory migration lock. No shared legacy live writer remains.
- **R5 (AA-8), A:** the root summarizes the kernel telemetry report for its fixed pid since bind before the model generates a revision diff. The prompt includes that trusted summary; deterministic replay fixtures pin the real revised prompt hashes. Author-only fixtures and entry counts remain unchanged where their prompts do not change.


## Acceptance Criteria

*(Epic §18-7 as corrected at creation and ruled 2026-10-26 (D-18-7-01…16 as recommended: 06 B, 13 B, 16 B, the rest A). The four epic ACs kept, tightened with cites (epic `:123`). One plant per control AC, `docs/testing/proof-contract.md:26`. Security-critical: AC1, AC2, AC3.)*

**Inherited deferred rows owned by this story:** none (owner grep: zero). Blast-radius rows `:479`, `:485`, `:488`, `:651` are adopted only through D-14, D-10, CR-4 and D-10 respectively.

### AC1 — command: a Spirit authors and proposes on the live path; a restart keeps everything (D-01…09, 12, 13; security-critical)

1. Process leg (daemon-level), `crates/maos-journey-test/tests/journey_skill_authoring_18_7.rs`, keyless runner, scratch `HOME` with `maos init` (control.json → door), `MAOS_IDLE_FAST`:
   - Daemon 1: `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/skill-authoring-18-7.json maos run spirits/researcher/manifest.toml` (no `--once`; the root hosts the door; manifest v6 declaring `[capabilities.required.skill] author_self = true`). Pass 1 (one idle fire per quiet period — never relied on twice) surveys, distils, then authors `researcher.<name>@0.1.0` through the port: a `SkillAuthorSelf` mint and verification at the Researcher's pid in the TL, one enqueue decision row, a v2 queue entry holding the full document.
   - `maosctl skills approve <entry>` over the door → approve row; the document lands at `$HOME/.maos/skills/<id>.md` (D-06 B).
   - Pass 2 is triggered by a door reload (`maosctl unload researcher` + `maosctl load spirits/researcher/manifest.toml`; new pid, bound through the same `construct`), never by waiting on the watchdog. After its distillate the Researcher proposes a revision of its admitted skill: target id/version, a model-written diff, root-attached evidence for the NEW pid since its bind (D-08 A); target `id@0.1.0#<entry_id>`.
   - Stop daemon 1 (clean shutdown, audit drained). Daemon 2 boots on the same `HOME`: boot enforcement passes; `maosctl skills show <proposal-entry>` over the door prints target, version, diff and evidence byte-equal to what was submitted (sha256 equal to the enqueue row's); `maosctl skills reject <proposal-entry>` → reject row with the `#<entry_id>` target; the base skill stays `Admitted`. Daemon 3 boots (D-12 A: the rejected entry does not block).
   - Every decision row (enqueue, approve, reject) and every receipt names the entry id or target, never content.
2. The same binary loads `tests/fixtures/researcher-no-skill/manifest.toml` (v5, no declaration): its pass's authoring attempt is refused `capability-denied`, receipt journaled, queue unchanged.
3. `smoke-skill-7-4` prints 4 step lines; `discipline.yml:3467-3473` asserts 4; coverage rows FR39 `:631-642` and FR57 `:805-817` name this story's witnesses (D-13 B).
4. **Proven red (1 plant):** drop the payload from the persisted v2 entry → daemon 1's first persisted proposal read (`journey_skill_authoring_18_7.rs:207`) has no diff/evidence → red, before restart. The later restart/show assertions retain the durability witness.

### AC2 — capability-mediated, pid-fixed; every refusal typed and journaled (D-01, 02, 03, 08, 16; CR-1…CR-3; security-critical)

1. Kernel (◆, exactly the ruled files and figure): `Intent::SkillAuthorSelf`, `scope_to_intent` and `intent_action_key` arms; the harness literal; surface baseline re-hashed for `Intent` only. Root mint per submission through `issue_enterprise_governed_capability`; `TaskAssign` delegation of the scope refused typed in `spirit_session.rs` (17-3c's unmediated case moves to `GatewaySend`, count unchanged).
2. Root witnesses, `crates/maos-bin/tests/skill_port_18_7.rs` (real kernel adapters, no mocks of the capability registry): the port never receives a pid (compile-level: no such parameter) and writes at the pid the root resolved; undeclared → `capability-denied`; an org deny on `skill.author.self` → denied; an exhausted pid (18-5) → `context-exhausted`; quota exhaustion typed; evidence equals `SelfTelemetryAggregator::self_telemetry(pid, bind_instant)` for that pid; after root shutdown every call is `HostShutDown` and the audit drain completes (no stall).
3. D-03 A library legs (`crates/maos-skill/tests/live_submission_18_7.rs`): foreign namespace, foreign target, stale target version, capability escalation, malformed document (`ESkillSchema`), empty diff — each typed, nothing enqueued. D-16 B: a document naming a principal indexed to the author pid → `PrincipalDataRefused`, receipt carries no principal id.
4. **Proven red (1 plant):** bypass the mint in the port (enqueue without `issue_*`) → AC1.2's undeclared Researcher enqueues → red.

### AC3 — durable payload, one writer, no silent loss (D-04, 05, 07, 10, 14; CR-4, CR-5; security-critical)

1. `QueueEntry`/`QueueFile` (`crates/maos-skill/src/store.rs:56-74`) become `maos.skill-queue.v2` per D-04 A: a v1 file loads (label-only entries preserved) and is written back v2; an unknown version hard-fails; an unreadable v2 file refuses submit/decide/show typed and is never overwritten; `HOME` unset → `NoDataHome` (D-14 A). Atomic write unchanged (`:184-229`).
2. `SkillAdmissionQueue::from_stored` reconstructs `RevisionProposal` from the payload; an unknown label is a typed error, never `PackageShipped` (`admission.rs:280-296`); FKCS pin re-hashed in the same commit (D-10 A).
3. Entry identity per D-05 A: two revisions of one skill coexist; approving one changes neither the base skill nor the other; `admission_view` lists every queued entry.
4. Single writer per D-07 A: door verbs carry decide/list/show; the offline arm runs only when no daemon answers; a submission racing an offline decide cannot lose either (witness: the door path while the daemon writes).
5. Library legs in `crates/maos-skill/tests/admission_store_test.rs` (v1 → v2, unreadable, `NoDataHome`) and `live_submission_18_7.rs` (identity); CLI legs in `crates/maos-cli/tests/skill_queue_integration_test.rs` updated for v2 and entry ids.

### AC4 — proven red in CI (rule 11)

1. One hosted run on `ubuntu-24.04` (`ci/18-7-verify`, plant diff + sha256 in `18-7-evidence/ci/`) applies AC1.4's plant and reds the AC1 process leg; the run id is recorded. AC2.4's plant and the D-05 / D-12 controls (revert the `#<entry_id>` target → approving the proposal flips the base skill; revert the on-path filter → daemon 3 refuses to boot) are proven red locally, commands and outputs in `18-7-evidence/plants.md`, restored from byte-identical backups.
2. The new suites run in CI: the journey binary in `journey-hermetic-tier-1` (`discipline.yml:3327-3357`), `skill_port_18_7` beside the maos-bin suites, maos-skill tests in the workspace job; any CI test-count pin over an enrolled binary is updated with the new count; each binary Σ ≤ 30 s (`xtask/test-budgets.toml:26`; no exception requested).

## Tasks / Subtasks

- [x] **T0 — Preflight at the dev HEAD.** D-18-7-01…16 RULED 2026-10-26 (all as recommended); tracker `ready-for-dev` → `in-progress`. Re-anchor: `git diff --stat 8fdf497f -- crates/maos-skill crates/maos-cli/src/subcommands.rs crates/maos-control/src/lib.rs crates/maos-manifest/src/manifest.rs crates/maos-kernel-core/src/capability crates/maos-kernel-core/src/scheduler/scheduler_loop.rs crates/maos-bin/src/{main.rs,researcher_root.rs,operator_door.rs,spirit_session.rs,worker_spawn.rs} spirits/researcher/src`. **Re-measure `check-kernel-baseline` / `kloc-check --json` from the LANDED pin** — 18-6's review patches may have moved it inside 18-6's remaining grant (26407 / 20650 at creation); the ratified +5 physical / +4 tokei applies over the landed values, never widened, and the re-stated figure is written into the FLAG record's §Re-pin before any kernel edit. Re-measure every out-of-kernel row and re-state its STOP line over the landed figure (same expected + 25 % rule). Confirm 18-6 landed unchanged in `researcher_root::construct`. Process deviation: the FLAG T0 annotation was written late; the measurement itself preceded kernel edits (Debug Log).
- [x] **T1 — Kernel** (AC2.1; ◆ only as ruled): D-01 / D-02 lines in exactly the four ruled files; `check-service-boundary` re-baseline of the `Intent` item only; the re-pin set of `18-7-flag-winston-decision.md` §Re-pin (via `check-kernel-baseline --emit-pin`).
- [x] **T2 — Manifest** (D-02): v6 table, degrade, scope mapping, `MANIFEST_SCHEMA_VERSION`, N-1 tests; Researcher manifest → v6 with the declaration; fixture `tests/fixtures/researcher-no-skill/manifest.toml`; docs-site manifest v6 page en + ko.
- [x] **T3 — maos-skill** (AC3; D-03…06, 10, 14, 16): v2 store; payload types with serde; entry id; submission vetting; `from_stored`; materialize on admit; typed errors + `docs-site/docs/errors/ESkill*-*.md` pages (`error-catalog-check`); FKCS re-pin.
- [x] **T4 — Root port** (AC2; CR-1…CR-4; D-01, 08): mint/verify, pid fix, evidence attach, enqueue row, receipts + `WRITER_SHAPES`, weak holds; binder in the wiring; bound in `construct`; `TaskAssign` refusal; new maos-bin src file (if any) listed in `SCANNED_SOURCE_FILES` (`crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:927`).
- [x] **T5 — Door and CLI** (D-07): `OperatorCommand::SkillDecision`, routes, door handler; `maosctl skills` over `durable_verb`, `show`, offline arm.
- [x] **T6 — Boot enforcement** (D-12): on-path filter at `main.rs:4342-4417`.
- [x] **T7 — Researcher** (D-09): authoring and proposal logic, prompts, `own_entries` use; typed outcomes; cassette `skill-authoring-18-7.json` (`provenance: seed`, real prompt hashes, as 18-4 CR-3; `cassette_provenance_15_6` green).
- [x] **T8 — Demo and coverage** (D-13): retire steps 3–4; CI 6 → 4; coverage rows FR39/FR57; inventory `:562` "delivered by 18-7" at landing.
- [x] **T9 — Witnesses and plants** (AC1–AC4): journey leg, root leg, library legs; plants red and byte-restored; hosted `ci/18-7-verify` diff + checksum + workflow checked in. Hosted execution/run id is integrator-owned by this assignment; local simulation is complete.
- [x] **T10 — Governance:** kernel re-pin set, raised zero-headroom ceilings exact; reserved ceilings retained with exact consumed-figure HISTORY, `_aggregate_hardfail` = measured + 1; `check-kernel-baseline`, `check-service-boundary`, `kloc-check`, `check-fkcs`, `check-manifest-schema-version`, `check-skill-schema`, `check-serde-error-handling`, `check-test-cost`, `check-test-budget`, `check-coverage-matrix-completeness`, `error-catalog-check`, `check-reza-production-path`, `gen-abi-docs --check` (if the schema constant is in the generated docs), Korean coverage + glossary; release binary < 32 MiB.
- [x] **T11 — Regression and record:** focused suites (below); `deferred-work.md` annotations for `:479`, `:485`, `:488`, `:651` per the rulings; Dev Agent Record, File List, Change Log; tracker. Do not edit `intent-lineage-coverage-report.md` (revert if a run modifies it). No commit or push.

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Kernel ◆ | `crates/maos-kernel-core/src/capability/{mod.rs:58-111, cap_policy/decision.rs:37-73, cap_policy/mod.rs:406-426}`, `scheduler/scheduler_loop.rs:326-332` | unmediated, undeclarable → mintable `SkillAuthorSelf` |
| Manifest | `crates/maos-manifest/src/manifest.rs:603-917`, `lib.rs:15-28`; `crates/maos-spirit-abi/src/lib.rs:118` | no skill table → v6 table |
| Skill | `crates/maos-skill/src/{store.rs, admission.rs, proposal.rs, schema.rs, errors.rs}` (+ new module) | label-only v1 cache → v2 payload store, vetting, identity |
| Root | `crates/maos-bin/src/researcher_root.rs:109-166`, `main.rs:4034-4062,4342-4417,13546-13583`, `spirit_session.rs:899-928`, `operator_door.rs` | demo-only → port, door verbs, boot fix |
| Door/CLI | `crates/maos-control/src/lib.rs:270,1241-1259`; `crates/maos-cli/src/{cli.rs:311-340, subcommands.rs:871-1226}` | offline decide → door + offline arm |
| Spirit | `spirits/researcher/src/lib.rs:661-705` (pass), manifest | — → authors and proposes |
| Audit | `crates/maos-audit/src/fr4_classifier.rs:195` | — → receipt shapes |
| Data/docs | cassette; fixture manifest; `tests/coverage-matrix.yaml:631-642,805-817`; `discipline.yml:3455-3473`; `xtask/fkcs-baseline.toml:41`; `docs/ci-baselines/kernel-surface-v0.1-beta.json:62`; docs-site errors + manifest v6 (en, ko) | — |

### Current state that must be preserved

- **No auto-admit** (ADR-024; `admission.rs:1-15`): every path lands `Pending`; `skill.author.self` authorizes the write, never activation.
- **Journal-first decide** and LWW by `decision_id` (`subcommands.rs:1035-1053,1204-1225`; 9-7).
- **Package-shipped discovery** path and its `package_shipped` label for discovered skills (`:1107-1120`).
- **17-11** entitlement at issuance; **18-5** context exits and the shared receipt pattern; **18-3** one Researcher construction path, per-pass inference mint, typed pass refusal; **18-6** forget route, cache purge and `on_consolidate` handling.
- **FKCS** proxy-Spirit admission (`crates/maos-fkcs/src/lib.rs:215-250`) keeps working on the changed queue API.
- `smoke-skill-7-4` steps 1, 2, 5, 6.

### Guardrails

- The root fixes pids and resolves the class; the Spirit never names a pid, a class or evidence.
- One binding on every construction path (boot arm, door load, successor) — through `construct`, never a call-site patch.
- Factories and ports hold buses weakly; shutdown audit drains are never pinned.
- Typed refusals, journaled; no silent fallback (no `PackageShipped` default, no `/tmp`, no ignored queue save for payload entries).
- Nothing personal or skill content in TL rows; content lives in the queue only.
- Kernel diff exactly the ruled files and figure; anything else returns as a FLAG request.
- One target dir; `env -u CI -u GITHUB_ACTIONS`; no cargo inside a `#[test]`; plants restored from byte-identical backups; Σ per-test ≤ 30 s per binary.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Durable payload (AC1) | drop the payload from the v2 entry | daemon 2 `show` lacks diff/evidence |
| Mint (AC2) | bypass `issue_*` in the port | undeclared Researcher enqueues |
| Identity (AC3, D-05) | revert the `#<entry_id>` target | approving the proposal flips the base skill |
| Boot availability (D-12) | revert the on-path filter | daemon 3 refuses to boot |

### Tests and commands

```bash
# local; one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-skill -p maos-manifest -p maos-control -p maos-fkcs -p researcher
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-cli --test skill_queue_integration_test --test skill_queue_reconcile_test
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test scope_selector_entitlement_17_11 --test cap_policy_decision --test manifest_field_coverage
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test skill_port_18_7 --test researcher_log_grant_17_3d --test j_researcher_survey_18_3
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --features network,wasm-host --test spawned_scope_17_3c -- --test-threads=1
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test --test journey_skill_authoring_18_7 --test journey_researcher --test journey_forget_18_6 --test cassette_provenance_15_6
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-audit --test fr4_classifier_16_2
MAOS_ONE_SHOT=smoke-skill-7-4 cargo run -q --locked -p maos-bin
cargo run -q --locked -p xtask -- check-kernel-baseline && cargo run -q --locked -p xtask -- check-service-boundary && cargo run -q --locked -p xtask -- kloc-check --json
cargo run -q --locked -p xtask -- check-fkcs && cargo run -q --locked -p xtask -- check-manifest-schema-version && cargo run -q --locked -p xtask -- check-skill-schema
cargo run -q --locked -p xtask -- check-serde-error-handling && cargo run -q --locked -p xtask -- check-test-cost && cargo run -q --locked -p xtask -- error-catalog-check
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-67`)

- **Models/tiering:** difficult story — Blind, Edge, Acceptance on opus-5.5; Test Infrastructure on glm-5.3; dev/finisher gpt-6.1-sol never reviews its own code.
- **Security layer (AC1–AC3):** who may author/propose (D-03) and apply (D-06, D-07); mint on every submission and no delegation; pid and class fixed by the root; evidence provenance; approval-target identity; single writer; nothing personal in rows; restart availability.
- **FLAG-Winston:** kernel diff = the ruled files, figure exact, surface re-hash = `Intent` only.
- **Daemon-level witnesses** for submission, door decisions, reload and restart.

### Previous-story intelligence

- **18-6:** the Researcher's `construct` carries the forget route and context port — add the skill binder there, not at call sites; `principal_id_occurs` is token-bounded; serde `unwrap_or_default` in a writer was caught at review.
- **18-5:** factories hold buses weakly (audit-drain stalls were real); refusals share a receipt shape; new TL writers need `WRITER_SHAPES`; new maos-bin src files enter the cohort roster; the CI marker pin moved twice.
- **18-4:** the root fixes the pid (TIA-1/BH-2); a door reload, not the watchdog, drives a second pass; Markdown/durability limits were measured, not assumed.
- **18-3 / 18-1:** one construction path; per-pass mint; typed pass refusal; strict cassettes with real prompt hashes.
- **9-7 / Epic 10 §A3:** journal-first decide; the TL decided-set is the source of truth for state — keep it, and keep payloads out of it.

### Git intelligence

| Commit | Content |
|---|---|
| `8fdf497f` | 18-6 dev pass (review pending); kernel 26407 / 20650; maos-bin 28914 |
| `3aad2f4c` | 18-5 landing fixes (IAC bus held weakly, independent journal) |
| `5495863e` | 18-5 review patches |
| `be12ca0b` | 18-5 dev pass (context ledger, kinds 32/33) |
| `79f49709` | 18-4 review patches (root-fixed pid) |

`crates/maos-skill` and `subcommands.rs:871-1226` are unchanged since 10.5 (`0d2b18ee`, `git log -L`); the demo `smoke_skill_7_4` last changed at 17-1 (`b19556ab`, CliWrapper step).

### References

- Epic `epic-18-spirits-that-think-w3.md` (post-ruling lines): §18-7 `:243-256`, creation corrections `:247`, stories row `:120`, R18-21 `:94`, Kernel-Δ `:32` + 18-7 note `:42`, kloc `:58,60`, rules `:62-64`, `:123`, Dependencies `:274,280,282`.
- `prd/functional-requirements.md:79,89,91`; `requirements-inventory.md:562,590`; `routing-audit-2026-10-22.md:19,97,106`.
- Architecture `architecture-maos-minimal-opus/12-architecture-decision-records.md:385-395` (ADR-024), `5-spirit-abi.md:69-70,207,242`; `docs/adr/ADR-027-…md:19-42`; `docs/testing/proof-contract.md:26,31-67`.
- `deferred-work.md:118,479,480,484,485,488,651,979,1156`.

## Dev Agent Record

### Story context creation

Created 2026-10-26 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter contract) at `8fdf497f`, clean tree.

**Inputs:** direct re-reads of every decision-bearing range; `check-kernel-baseline` (PASSED 26407 / 95) and `kloc-check --json` (`passed: true`, aggregate 186040) at HEAD.

**Prototypes** (one cargo process; fmt-clean):
- D-01 A alone: `cargo check -p maos-kernel-core` clean; `check-kernel-baseline` FAILED as expected at 26411 (changed: `capability/cap_policy/decision.rs`, `capability/cap_policy/mod.rs`, `capability/mod.rs`); `kloc-check` maos-kernel-core 20653; `check-service-boundary` FAILED on `Intent` (signature hash). Reverted.
- D-01 A + D-02 A: workspace `cargo check --locked` clean after the four literal ripples (kernel `scheduler_loop.rs:326-332`; maos-bin `admission.rs:615-625`, `main.rs:7334-7344`, `:13724-13730`); `check-kernel-baseline` FAILED as expected at 26412 (4 files); `kloc-check` maos-kernel-core 20654, maos-manifest 4605, maos-bin 28917, aggregate 186069.

**Revert:** `git checkout -- crates`; `git diff --quiet HEAD -- crates xtask spirits Cargo.lock Cargo.toml` exit 0; `check-kernel-baseline` PASSED 26407 / 95. `intent-lineage-coverage-report.md` not modified.

### Agent Model Used

anthropic/claude-opus-5-5 (creation); openai-codex/gpt-6.1-sol (development).

### Debug Log References

- T0 (2026-10-26), landed HEAD `d155f08f`: `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo cargo run -q --locked -p xtask -- check-kernel-baseline` PASS, **26362 physical / 95 files**; `kloc-check --json` PASS (`over_budget: []`), **20606 kernel tokei**, aggregate **186483**. Original baseline_commit is preserved. Re-anchor against `8fdf497f`: only main.rs, operator_door.rs, researcher_root.rs and Researcher src changed among T0's paths (322 insertions / 119 deletions).
- T0 landed grants and STOP lines: kernel **26362 → ≤26367 physical**, **20606 → ≤20610 tokei**, four ruled files + Intent surface only; maos-bin **29113 → ≤29150** (37 remaining). maos-skill **800 → ≤1138** (+338); maos-manifest **4583 → ≤4629** (+46); maos-cli **6104 → ≤6198** (+94); maos-iac **8270 → ≤8295** (+25); maos-audit **7635 → ≤7658** (+23). Zero-growth STOPs: domain **9975**, capability **1432**, wasm-host **2412**, spirit-sdk **1085**. Control **1286 <1791**, journey **946 <1104**, fkcs **491 <591**, spirit-abi **1042 <3000**; reaching those ceilings STOPs. Aggregate hardfail at landing = measured +1. Forecasts and 25% rounding unchanged; no grant widened.
- **Historical halted checkpoint (superseded by the resumed D-15 addendum below):** post-`cargo fmt --all` STOP measured maos-bin **29275 >29150 by125**, landed **29113→29275 (+162)**; aggregate **186993** (`18-7-evidence/stop-kloc.log`). Expansion and re-pins stopped until the operator's new ruling. At that checkpoint kernel20610, skill1138, manifest4604, CLI6100, FKCS480, IAC8270, audit7635, domain9975, capability1432, wasm-host2412, SDK1085, control1286, journey946, ABI1042. These are not final measurements.
- Exercised: T1 mediation witness failed on unmediated `SkillAuthorSelf`, then the focused `cap_policy_decision` binary passed **4/4** (`t1-mediation-{red,green}.log`); these are development red/green runs, **not AC plants**. Manifest+ABI focused run reached **208 passed /1 failed /28 not run** on a hardcoded forward-version probe; replaced that probe with `MAX_SUPPORTED+1`, not re-run before STOP (`t2-schema-readers.log`). The adapter and migrated CLI compiled as dependencies of the STOP gate; no final-tree build/test claim is made.
- `cargo generate-lockfile --offline` initially produced unrelated resolver churn while adding the specified sha2 edge. That churn was removed at halt: lock restored from landed HEAD with only `maos-skill→sha2` and `researcher→maos-skill` dependency edges. The STOP gate therefore ran before this lock cleanup; its LOC figures remain exact, but compilation is not claimed for the restored lock graph.
- Historical halt process note: landed figures were recorded before kernel edits, but their FLAG annotation was written only at halt, later than T0 required. Original `baseline_commit` preserved. No 18-6 TI patches had been applied at that checkpoint; the separately requested resumed-pass fixes are recorded below.

### Completion Notes List

#### D-18-7-15 addendum — operator ruling 2026-10-26 (resumed)

Operator Lunarpulse, relayed by Main: **maos-bin 29150 → ≤29500**, **maos-skill 1138 → ≤1350** pre-ratified; intermediate bin29300 is superseded. Kernel remains **≤26367 physical / ≤20610 tokei** in the four ruled files; other T0 STOPs unchanged. Main additionally approved the mandatory schema-v6 ratified-inventory line as implied by operator D-02 A, exact xtask +1. Reserved ceilings are retained by Main's explicit ruling, not silently ratcheted down.

- Implemented native model-authored Researcher drafts/revisions, root-fixed pid/class and trusted manifest declarations, weak bus/store bindings on every construction path, governed issuance/verification, root-attached telemetry, typed journaled refusals, strict v2 durable payload/identity, journal-first admission, authenticated door/CLI read/decide, offline locking, author materialization and inert queue-only rejection at boot. Third-party authoring, skill consumption, revision auto-apply, queue erasure and operator identity remain the expressly stated non-goals above.
- Kernel physical **26362 → 26367**, tokei **20606 → 20610**, **95 files**, exactly the four ruled files. Full physical/hash/set/content-test/ceiling-rule/kloc pins moved together; only the `Intent` surface item was re-pinned. FKCS admission hash re-pinned per D-10 A.
- Exact formatted source measurements: maos-bin **29113 → 29443** (+330, grant29500); skill **800 → 1138** (+338, grant1350); manifest **4583 → 4604**; audit **7635 → 7645**; CLI **6104 → 6154**; control **1286 → 1344**; FKCS **491 → 480**. IAC8270/domain9975/capability1432/wasm-host2412/SDK1085/journey946/ABI1042 unchanged. Every T0 STOP is respected.
- xtask 46363 → 46364 (+1), mandatory `capabilities.required.skill` ratified post-v1 inventory; Main approved under operator D-02 A. Exact aggregate **186483 → 187284** (+801); `_aggregate_hardfail` **186484 → 187285**, measured+1.
- Reserved ceilings deliberately retained: CLI6856, control1791, journey1104, FKCS591, ABI3000; each receives the full story-key HISTORY and exact consumed figure. Other raised rows are exact, zero-headroom pins.
- Bin per-file tokei delta (sum+330): `skill_authoring.rs` +249, `operator_door.rs` +80, `spirit_session.rs` +8, `researcher_root.rs` +7, `lib.rs` +2, `admission.rs` +1, `main.rs` −17. Exact old/new figures: `18-7-evidence/bin-attribution.log`.
- All four distinct AC plants were red and copy-back/`cmp`/`touch` restored; timings, commands and outputs are checked into `18-7-evidence/plants.md`. CI payload-plant SHA256, diff, branch-only workflow and one local simulation are checked in; hosted ubuntu-24.04 run/run id is the integrator's, not claimed locally.
- Separate 18-6 TI hunks: shared door wait5s→15s; `assert_absent` uses Butler's real `butler.acceptance/v1` acceptance-record canary. Focused journey-forget + kernel cascade: **15 passed** (`18-6-ti-green.log`); not mixed into the 18-7 feature attribution.
- Regression repairs preserve production meaning: schema-v6 readers retain log grants but drop self-authoring under older versions; survey-only counters exclude the extra authoring completion; door reload waits for the durable draft before cancelling the old pid; CI exit-line3 derives survey3/total4; unmediated GatewaySend uses its typed JSON representation; FR4 has the real kind7 refusal fixture and scanned site.
- Host disk `wait_for_commit` stalls were observed on daemon scratch. Only local daemon acceptance uses `TMPDIR=/dev/shm`; no timeout/budget widened. An attempted custom shared scratch root caused false ancestor-lock/0700 failures in broad legacy fixtures, so the supported broad run used normal scratch and truthful `MAOS_17_2_CGROUP_MODE=undelegated` (cgroup mount read-only).
- Final new acceptance binaries: **39 passed / 0 skipped** — root9, door2, CLI6, journey2, manifest3, library submission9, Researcher8 (`new-acceptance-green.log`). Existing library/schema readers **594 passed**; kernel lib **258 passed**; final root+FR4 **30 passed**; required d11/recovery/kernel-content/CI-shape plus complete Researcher reload reader **64 passed**; cognitive journey/cassette readers **16 passed /1 inherited skip**. Complete green runs cover every selected reader binary after fixes; `broad-status.log` preserves initial failures and the full fix-to-green provenance. No workspace suite was run.
- Strict `check-test-budget` **PASS: 16 binaries, 206.440 s test time /800 s** from preserved complete focused reports (all five existing exception binaries included, not deleted or loosened; `budget-provenance.log`). Every new binary is below30 s; maximum **1.685 s**. Actual retired smoke **exit0, four steps [1,2,5,6]**; CI source/marker pins and cassette provenance green.
- All named governance gates PASS: kernel baseline26367/95, service boundary, exact kloc187284, FKCS8 legs, manifest current6/min1/max6, skill schema, serde0 violations, test-cost560 files, test-budget, coverage55 v1.0 rows, error catalog65/65, corpus15 entries, Reza production-path advisory, generated ABI `--check`, formatting `--check`, KO canonical42/42 and glossary. Full outputs are in `18-7-evidence/gate-*.log`. The pre-existing aggregate alarm and Reza's four inherited `ABSENT` successors (`product_claim=NOT_PROVEN`) are unchanged, not hidden as product proof.
- Shipped release + strip: **29,968,248 → 30,445,560 bytes**, below **33,554,432** by **3,108,872 bytes** (`release-size.log`). Plant backups and temporary attribution file removed; no scaffold remains in the repo.
- Development model **openai-codex/gpt-6.1-sol**. Story and sprint row → **review**; original `baseline_commit` retained, `intent-lineage-coverage-report.md` untouched, no commit/push. Only the hosted ubuntu-24.04 plant run/run id is handed to the integrator; no feature implementation blocker.

### Review (2026-10-26, opus Blind/Edge/Acceptance + glm Test-Infra)

Review sources: `agent://Blind187` (BH), `agent://Edge187` (EC), `agent://Accept187` (AA), `agent://TestInfra187b` (TI). This is the instructed patch/disposition pass on local commit `3d31b8c1`, not a new self-review. Operator rulings R1–R5 above govern the decision-bearing findings. Security readings cover the authenticated door, no offline fallback on rejected bearer, single-writer locks, root-fixed identity, per-submission mint/verify, evidence provenance and durable admission identity.

| Finding | Disposition and final witness |
|---|---|
| **BH-1 / EC-1 / AA-1 — HIGH, reused approval identity** | **Patched.** `prepare_author` requires semver precedence strictly above every same-id non-revision historical entry, regardless of state/provenance, and every discovered package. Rejected text cannot become a base; changed text cannot reuse its rejected version. Library state/provenance/discovery/build-metadata/rejected-base witnesses and the library control plant are green/red respectively. |
| **BH-2 — HIGH, air-gap build** | **Patched.** The serialization lock lives in always-compiled `operator_door` and authoring reuses it. The air-gap build also exposed network gating around the network-free hook pump used by the pre-existing forget door; those two obsolete cfg gates were removed rather than adding a no-op forget fallback. Supported air-gap verb build: **7 passed**; dependency-boundary plant fails compilation and restores green. |
| **BH-3 / EC-2 — pre-journal materialization and id length** | **Patched.** `prepare_decision` probes destination shape, foreign collision and exact admitted-byte/class ownership before returning the approval row; apply rechecks against intervening changes. Authored ids are capped at128 bytes at submission/stored validation. CLI witness verifies a collision leaves the TL empty and payload unchanged; boundary, missing-home, directory and ownership witnesses plus plants pass. |
| **BH-4 tail — non-finite evidence** | **Patched.** Non-finite f32 halt values or thresholds refuse typed `NonFiniteEvidence` before enqueue; stored validation also rejects them. Finite evidence round-trips. The dedicated value/threshold witness is red under its plant and green restored. |
| **BH-4 boot / EC-3 / EC-4 / AA-6 — queue availability** | **R1 A applied.** Missing home leaves the binder unwired and authoring typed `queue-unavailable`; corrupt/unreadable queues warn and boot from discovery + TL. Submit/decide/show refuse without overwrite. No-home and corruption process witnesses and plants pass. Offline durable children bypass daemon-only admission output/enforcement, preserving their one terminal JSON; the complete FR45 process regression passes. |
| **BH-5 — all daemon modes** | **R2 applied.** Rejected discovered skills still block every daemon root, including non-`run` roots. Only offline durable children are outside daemon boot enforcement. The non-`run` rejection witness is red if the old `run_args` guard returns; queue-only authored drafts and decision-only revisions remain inert. |
| **BH-6 — FKCS journal truthfulness** | **Patched.** `AdmissionResult.decision_receipt` returns the actual approve/reject row after applying the genuine verdict; `journaled` derives from that retained receipt, never merely an enqueue row. The comment and contract test inspect the real decision; the receipt-removal plant reds it. |
| **EC-5 — queue/lock/TL home split** | **R4 B applied.** The live path is `<maos_home>/skills/queue.json`; validated legacy data is journaled and consumed once under the canonical home lock plus the legacy-directory migration lock. Invalid bytes/journal failure are typed and preserve source/destination. Canonical path, isolated homes, one-time consumption and journal-order witnesses and separate plants pass. |
| **EC-6 — id `decide` route** | **Patched.** Only POST selects the decide command; authenticated GET falls through to normal show. Real TCP route witness passes and its old-match plant returns405/red. |
| **AA-2 — xtask grant regex boundary** | **Patched.** HISTORY now says `formatted 46363 -> 46364`; d11 ratchet is green. xtask remains46364 with no source growth. |
| **AA-3 — stale AC2 plant** | **Patched/re-proven.** `ac2-mint.diff` was regenerated against the reviewed authorization code, bypassing both mint/verify and the new pre-model declaration gate. The real undeclared process witness reds `undeclared Researcher cannot enqueue`, then copy-back/`cmp`/`touch` and real binary rebuild restore green. Both logs and the diff are refreshed; final applicability exits0. |
| **AA-4 — actual D-12 authored-draft case** | **Patched.** A real Researcher authors a draft, the operator rejects it before admission, the daemon stops, and the next reviewer daemon boots with that rejection intact. The on-path-filter plant specifically reds this authored-draft boot, independently of the existing revision rejection leg. |
| **AA-5 — whole-pass refusal and wasted completion** | **R3 applied.** Authoring refusals retain successful survey output and remain typed; actual port refusals are journaled. The trusted declaration preflight precedes author generation. Malformed draft and undeclared witnesses retain `last_output`; undeclared spends no author completion. Root `pass_verdict` accepts the retained successful survey. Draft preflight skips unused report construction/allocation; only revisions request a summary. |
| **AA-7 — bind instant witness** | **Patched.** A real `binder()` test asserts proposal `window_start_ns` is at/after the instant before construction, verifies the trusted preflight summary comes from a real TaskComplete row, and refuses an undeclared binding. Setting bind time0 reds the window assertion. |
| **AA-8 — model did not consult evidence** | **R5 A applied.** Root-fixed pid/bind-scoped kernel counts (`successes`, `failures`, `halts`, `distillations`) enter the revision prompt before generation. Each count independently changes the observed model prompt/hash. The real replay journey validates sequence7 SHA256 `201199bdc738ae2a2e73245cfc6d0f9420bdcdd5bdde4d80bf62423a0b2e185e`, length915. Author-only survey/forget fixtures, their per-entry pins and the exit-line3 count derivation intentionally remain byte-identical because those prompts/counts did not change. |
| **AA-9 — contradictory planning/delivery records** | **Patched.** Epic, FLAG and story record the resumed bin≤29500/skill≤1350 addendum. Inventory says **under review; hosted proof owed**, not Delivered. Status remains `review`. |
| **TI-1 — a second air-gap test target not enrolled** | **Dismissed as ruled.** The inherited single-binary air-gap leg remains the supported contract; the actual verb-table air-gap build was run and passed. No future extra-target coverage is claimed. |
| **TI-2 — reviewer boots inherit a Researcher cassette** | **Dismissed as ruled.** The reviewer has no inference binding today; this is latent coupling, not a false current proof. No unrelated cassette-env redesign was added. |
| **TI-3 — wrong AC1 red location in records** | **Patched.** AC1.4 and `plants.md` now name daemon1's first persisted proposal read at test`:207`, before restart, matching the observed red. The later restart/show assertions still witness durability. |

**Focused verification (one cargo process; no workspace suite).** Every cargo command used `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo`. The final complete root/door/CLI/Researcher/Butler/skill/FR45 reader selection passed **236 tests /1 inherited skip**, wall-clock **128.603 s** (`review-final-root-spirit-readers-serial-green.log`); `-j1` avoids the observed init/boot ephemeral-port collision from the highly concurrent attempt, without adding retries or weakening the boot oracle. Complete library/manifest/ABI/FKCS/Researcher/Butler readers passed **456 /2 inherited skips**, **19.320 s**. Kernel lib **258 passed**, **3.868 s**. Cap-policy, FKCS oracle, d11/recovery/kernel-content/CI-shape readers **75 passed**, **29.779 s**. Retained budget-exception binaries **59 passed**, **25.784 s**. These complete final reports merge to **77 binaries /826 passed /0 failures**; the supported air-gap build separately has **7 passed**.

The seven new18-7 binaries now have **52 passed /0 skipped**: root10, door2, CLI6, journey6, manifest3, library submission16, Researcher9. Every new binary is below30s Σ per-test; the maximum is **4.815 s** (journey). The existing five budget exceptions and all floors are untouched. `review-focused-budget-junit.xml` merges actual complete latest binary reports, not hand-written test timings; `check-test-budget` **PASS:77 binaries,257.743 s /800 s**. The bounded `--once` root now releases the retained Researcher pass handle alongside Butler before audit drain; real reader runs no longer spend five seconds waiting on that strong audit owner.

**Plants.** Original AC1/AC3/AC4 remain valid; AC2 is freshly red/restored. Six additional named review bundles cover the new controls; every bundle is checked in with diff, red output, restored green output and exact `cp`/`cmp`/`touch` exit0 logs. Details and per-bundle wall-clock are in `18-7-evidence/plants.md`; every final diff and the unchanged CI payload plant pass `git apply --check`. The CI patch needs no regeneration. Hosted ubuntu-24.04 run/run id remains Main/integrator-owned, not claimed locally.

**Final exact formatted figures and pins.** Review delta: maos-bin **29443 →29497 (+54;≤29500)**; maos-skill **1138 →1234 (+96;≤1350)**; CLI **6154 →6155** (ceiling6856 retained); FKCS **480 →482** (ceiling591 retained). Aggregate **187284 →187437 (+153)**; `_aggregate_hardfail` **187285 →187438**, measured+1. Kernel **26367 physical /20610 tokei /95 files** unchanged: the original grant is fully used, and no kernel source was changed by these patches. Manifest4604/audit7645/xtask46364/IAC8270/domain9975 and the other zero-headroom rows stay unchanged. Reserved control1791/journey1104/ABI3000 ceilings stay unchanged. The combined path-stamped FKCS admission hash is **`9b66a40a2d25e4c8fabca5d3cd6185d8eade65c1de8989480d8dd107fd2451d6`**, re-pinned after review.

**Gates (full output and wall-clock: `18-7-evidence/review-gate-*.log`).** All required gates passed: kloc (`over_budget:[]`), kernel baseline26367/95, service boundary, serde0 violations, test-cost560 files, corpus15 entries, coverage-matrix, dev-record183 done stories, env-contract110 vars/0 violations, exit-commands12 resolved/3 explicitly owed, generated ABI `--check`, FKCS8 legs, manifest current6/min1/max6, skill schema, error catalog67/67, coverage completeness55 v1.0 entries, fmt `--check`, test-budget, enforced KO canonical42/42 and glossary. Representative wall-clocks: baseline0.267s, boundary1.460s, serde0.464s, test-cost0.468s, corpus0.389s, coverage0.281s, dev-record0.374s, env0.618s, exit0.558s, ABI1.439s, FKCS9.297s; exact full timings remain in each log. The inherited aggregate alarm and explicitly owed exit commands remain visible, not waived as product proof.

**Deferred/blockers.** No new deferred finding or implementation blocker. Existing native-only WASM/TS work stays with `20-8-manifest-contract-completion`; broader queue erasure stays with20-11, systemic XDG/env resolution with21-4, and operator `actor` identity remains the stated inherited limit. Adopted queue-home and admission-pin deferred annotations are updated without erasing history. The hosted proof remains owed before delivery/done. No commit/push; the protected intent-lineage report was not written.

### File List

- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md` (review addendum and R5 evidence claim)
- `crates/maos-bin/src/hook_pump.rs` (air-gap availability; no networking dependency)
- `crates/maos-fkcs/tests/fkcs_contract.rs`
- `docs-site/docs/errors/ESkillQueue-AuthoredIdTooLong.md`
- `docs-site/docs/errors/ESkillQueue-NonFiniteEvidence.md`
- `docs-site/docs/errors/ESkillStore-Io.md`
- `.github/workflows/discipline.yml`
- `Cargo.lock`
- `_bmad-output/implementation-artifacts/18-7-flag-winston-decision.md`
- `_bmad-output/implementation-artifacts/18-7-live-skill-authoring-and-revisions.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/requirements-inventory.md`
- `crates/maos-audit/src/fr4_classifier.rs`
- `crates/maos-audit/tests/fr4_classifier_16_2.rs`
- `crates/maos-bin/src/admission.rs`
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/operator_door.rs`
- `crates/maos-bin/src/researcher_root.rs`
- `crates/maos-bin/src/skill_authoring.rs`
- `crates/maos-bin/src/spirit_session.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/j_researcher_survey_18_3.rs`
- `crates/maos-bin/tests/researcher_inference_18_3.rs`
- `crates/maos-bin/tests/researcher_log_grant_17_3d.rs`
- `crates/maos-bin/tests/shell_host_16_2.rs`
- `crates/maos-bin/tests/skill_door_18_7.rs`
- `crates/maos-bin/tests/skill_port_18_7.rs`
- `crates/maos-bin/tests/spawned_scope_17_3c.rs`
- `crates/maos-cli/src/cli.rs`
- `crates/maos-cli/src/subcommands.rs`
- `crates/maos-cli/tests/skill_queue_integration_test.rs`
- `crates/maos-cli/tests/skill_queue_reconcile_test.rs`
- `crates/maos-cli/tests/skills_door_18_7.rs`
- `crates/maos-control/src/lib.rs`
- `crates/maos-control/tests/post_surface_16_1.rs`
- `crates/maos-control/tests/submit_and_wait_16_2.rs`
- `crates/maos-fkcs/src/lib.rs`
- `crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json`
- `crates/maos-journey-test/cassettes/j-researcher/skill-authoring-18-7.json`
- `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json`
- `crates/maos-journey-test/tests/journey_forget_18_6.rs`
- `crates/maos-journey-test/tests/journey_skill_authoring_18_7.rs`
- `crates/maos-kernel-core/src/capability/cap_policy/decision.rs`
- `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`
- `crates/maos-kernel-core/src/capability/mod.rs`
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs`
- `crates/maos-kernel-core/tests/abi_stability_admit.rs`
- `crates/maos-kernel-core/tests/cap_policy_decision.rs`
- `crates/maos-kernel-core/tests/operator_posture_ceiling_16_6.rs`
- `crates/maos-kernel-core/tests/provider_switched_journal.rs`
- `crates/maos-kernel-core/tests/sandbox_admission.rs`
- `crates/maos-manifest/src/lib.rs`
- `crates/maos-manifest/src/manifest.rs`
- `crates/maos-manifest/tests/skill_capability_18_7.rs`
- `crates/maos-skill/Cargo.toml`
- `crates/maos-skill/src/admission.rs`
- `crates/maos-skill/src/errors.rs`
- `crates/maos-skill/src/lib.rs`
- `crates/maos-skill/src/proposal.rs`
- `crates/maos-skill/src/store.rs`
- `crates/maos-skill/tests/admission_queue_test.rs`
- `crates/maos-skill/tests/admission_store_test.rs`
- `crates/maos-skill/tests/live_submission_18_7.rs`
- `crates/maos-skill/tests/proposal_test.rs`
- `crates/maos-spirit-abi/src/lib.rs`
- `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs`
- `docs-site/abi/v1/cancellation.md`
- `docs-site/abi/v1/compliance.md`
- `docs-site/abi/v1/constants.md`
- `docs-site/abi/v1/ctx.md`
- `docs-site/abi/v1/deprecation.md`
- `docs-site/abi/v1/gateway.md`
- `docs-site/abi/v1/identity.md`
- `docs-site/abi/v1/index.md`
- `docs-site/abi/v1/lifecycle.md`
- `docs-site/docs/cookbook/manifest-fields.md`
- `docs-site/docs/errors/ESkillQueue-AlreadyDecided.md`
- `docs-site/docs/errors/ESkillQueue-AmbiguousEntry.md`
- `docs-site/docs/errors/ESkillQueue-CapabilityEscalation.md`
- `docs-site/docs/errors/ESkillQueue-DuplicateSkillId.md`
- `docs-site/docs/errors/ESkillQueue-EntryNotFound.md`
- `docs-site/docs/errors/ESkillQueue-EvidenceMismatch.md`
- `docs-site/docs/errors/ESkillQueue-ForeignNamespace.md`
- `docs-site/docs/errors/ESkillQueue-InvalidStoredEntry.md`
- `docs-site/docs/errors/ESkillQueue-PrincipalDataRefused.md`
- `docs-site/docs/errors/ESkillQueue-Proposal.md`
- `docs-site/docs/errors/ESkillQueue-Schema.md`
- `docs-site/docs/errors/ESkillQueue-StaleTargetVersion.md`
- `docs-site/docs/errors/ESkillQueue-TargetNotOwned.md`
- `docs-site/docs/errors/ESkillQueue-UnknownEntryPath.md`
- `docs-site/docs/errors/ESkillStore-InvalidEntry.md`
- `docs-site/docs/errors/ESkillStore-Json.md`
- `docs-site/docs/errors/ESkillStore-MaterializeCollision.md`
- `docs-site/docs/errors/ESkillStore-NoDataHome.md`
- `docs-site/docs/errors/ESkillStore-UnknownSchemaVersion.md`
- `docs-site/docs/manifest/index.md`
- `docs-site/docs/manifest/v5.md`
- `docs-site/docs/manifest/v6.md`
- `docs-site/docs/migrate/index.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/cancellation.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/compliance.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/constants.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/ctx.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/deprecation.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/gateway.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/identity.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/index.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs-abi/current/lifecycle.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook/manifest-fields.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/index.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/v5.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/manifest/v6.md`
- `docs-site/i18n/ko/docusaurus-plugin-content-docs/current/migrate/index.md`
- `docs/ci-baselines/kernel-surface-v0.1-beta.json`
- `docs/errors/error-catalog.json`
- `spirits/researcher/Cargo.toml`
- `spirits/researcher/manifest.toml`
- `spirits/researcher/src/lib.rs`
- `spirits/researcher/src/skill_author.rs`
- `spirits/researcher/tests/inference_seam_18_3.rs`
- `spirits/researcher/tests/skill_authoring_18_7.rs`
- `tests/coverage-matrix.yaml`
- `tests/fixtures/researcher-no-skill/manifest.toml`
- `xtask/abi-ratifications.toml`
- `xtask/error-catalog.toml`
- `xtask/fkcs-baseline.toml`
- `xtask/kernel-core-baseline.toml`
- `xtask/kloc.toml`
- `xtask/src/check_manifest_schema_version.rs`
- `xtask/tests/ci_shape_18_0.rs`
- `xtask/tests/kernel_pin_content_hash_16_0.rs`
- `xtask/tests/recovery_lane_ceiling_rule.rs`
- `_bmad-output/implementation-artifacts/18-7-evidence/**` (all checked-in commands, outputs, timing XML, plants and CI simulation artifacts)

#### Separate requested 18-6 Test-Infrastructure hunks

- `tests/harness/forget_18_6.rs` — shared door wait5s→15s.
- `crates/maos-journey-test/tests/journey_forget_18_6.rs` — real Butler acceptance-record canary; the same file also has the feature cassette-count migration listed above.

### Change Log

- 2026-10-26: Story created (backlog; D-18-7-01…16 PROPOSED) at `8fdf497f`. Kernel priced by reverted prototypes (recommended +5 physical / +4 tokei, ◆ FLAG-Winston request); epic §18-7, its stories row and the Kernel-Δ note corrected in place; tracker row comment refreshed.
- 2026-10-26: RULED by the operator (Lunarpulse), relayed by Main: D-18-7-01…16 all as recommended (06 B, 13 B, 16 B; the rest A); ◆ kernel grant +5 physical / +4 tokei ratified (four files + `Intent` surface re-pin); Epic 18 named maos-bin budget 28914 → 29150; out-of-kernel STOP lines written into §Decisions; FLAG record `18-7-flag-winston-decision.md` created; D-11 received row filed for `20-8-manifest-contract-completion`. T0 re-measures from the landed pin (18-6 review patches). Status → `ready-for-dev`. Docs only, no cargo.
- 2026-10-26: Development HALTED by D-18-7-15 measured STOP: maos-bin29275 exceeds29150 by125. Landed preflight and partial implementation recorded by openai-codex/gpt-6.1-sol. No ceiling/pin widening, AC completion, plant or CI claim; story/tracker remain in-progress pending operator ruling.
- 2026-10-26: Resumed under operator D-15 bin≤29500/skill≤1350 and Main-approved schema-v6 inventory +1; completed 18-7-live-skill-authoring-and-revisions with native governed authoring/revisions, v2 payload durability, authenticated operator admission and all reader migrations. Four AC plants red/restored, CI verify simulated once; exact full kernel/FKCS/kloc pins and all named gates PASS, 39 new acceptance tests green, release30,445,560 B. Reserved ceilings retained. Separate 18-6 TI hunks green15. Story/tracker → review; hosted run remains integrator-owned; no commit/push.
- 2026-10-26: Applied all 18-7 review dispositions (BH1–6, EC1–6, AA1–9, TI1–3) under operator R1–R5; TI1/TI2 dismissed. Closed resubmission identity escalation, pre-journal materialization refusal, finite evidence, boot availability/all-daemon enforcement, single-home migration, nonfatal/preflight authoring and telemetry-informed revision generation. Refreshed AC2 and six new-control plant bundles, all copy-back/`cmp`/`touch` restored; 52 new acceptance tests and every selected focused suite/gate green. Exact bin29497/skill1234/aggregate187437/hardfail187438, unchanged kernel26367/20610; FKCS hash re-pinned after review. Story stays review; hosted proof remains integrator-owned; no commit/push.
