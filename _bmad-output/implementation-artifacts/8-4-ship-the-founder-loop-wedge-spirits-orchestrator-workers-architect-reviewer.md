---
dev_model_used: claude-opus-4-8
---

# Story 8.4: Ship the Founder-Loop Wedge Spirits — Orchestrator, Workers, Architect, Reviewer

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- dev_model_used frontmatter is set/confirmed by the dev agent in AC1 (§A2 hard-fail gate). Recommended: claude-opus-4-8 (Decision J). -->

## Story

As a founder running a v0.8/v0.9 overnight loop,
I want the **Orchestrator + Worker + Architect + Reviewer** reference Spirits shipped **together** as the founder-loop wedge demo — Orchestrator buffering director instructions at safe sequence points (FR20), distillate-fed dispatch (FR21, never raw output), Worker = a **CliWrapperSpirit** wrapping a CLI agent, Architect→Reviewer code-review loop, AND the **halt-and-resume-overnight** pattern — all built **on the substrate that already exists** (E3 `OrchestratorBuffer`, E6 FR21 dispatch gate + CliWrapper admission, E4 distillation chain, E5 hot-swap state codec),
So that the **v0.8/v0.9 wedge demo is real and runnable** — the founder assigns an overnight task at 11pm and finds an **audit-traced** result whose morning digest cites **actual** Transparency-Log refs at 7am — and the substrate's "moment of full ambition observable in one demo" (§13.1 J1) is proven end-to-end with **zero kernel KLOC**.

## What this story IS and IS NOT (read first — scope is deliberately bounded)

This is the **fourth reference-Spirit deliverable** and the **first multi-Spirit wedge** — four cooperating Spirits proving the founder loop, not one Spirit proving one capability. Unlike Butler (8.1, anticipatory compute), Researcher (8.2, scoped distillation), and Observer (8.3, read-only watchdog), Story 8.4 ships a **coordination demo** over substrate that **already exists**: the FR20 buffer, the FR21 distillate-dispatch gate, CliWrapperSpirit admission, the I11 distillation chain, and the hot-swap state codec all landed in Epics 3–6. **The four Spirits are thin reference consumers of that substrate; they add no kernel code.** Scope is drawn to prevent over-building (a live multi-CLI stdio bridge — that is *kernel* work, deferred from 6.2; a live-LLM Architect/Reviewer; real cross-Host A2A) and under-building (manifest-only stubs that never drive the real FR20/FR21/CliWrapper path, or a wedge "demo" that is not runnable).

**This story IS:**
- Four new **workspace-member Spirit crates** under `spirits/` (Decision A; workspace **33 → 37**):
  - `spirits/orchestrator/` — **rust-inproc** `[class]` Spirit. Drains the **real** `OrchestratorBuffer` (FR20) at safe sequence points, builds `TaskAssignPayload` with a `PriorDistillateRef` (FR21), and **never preempts an in-flight delegation**. Posture `autonomous-with-halt`, halt policy preferring recall over precision (§6.7).
  - `spirits/worker/` — a **CliWrapperSpirit** (`[cli_wrapper]` manifest, **NOT** `[class]`; mutually exclusive — `EManifestSchemaConflict`) that wraps a **real fixture-CLI binary** shipped in-crate. The fixture-CLI answers `--maos-bridge-probe` with the declared `output_shape_version` and echoes canned output, so the **existing real admission/probe/shape-assertion/FR40-journaling path is exercised end-to-end** while the *content* is fixture-replayed (Decision B).
  - `spirits/architect/` — **rust-inproc** Spirit; proposes a design (deterministic at v0.8, Decision E). Maps to `SpiritRole::Worker` (Decision C).
  - `spirits/reviewer/` — **rust-inproc** Spirit; critiques the Architect's proposal (deterministic at v0.8). Maps to `SpiritRole::Worker` (Decision C).
- The **Orchestrator** proven against the **real** FR21 gate: a follow-up `task.assign` carrying a `PriorDistillateRef` is **accepted**; one carrying `None` (or a raw `TaskComplete` frame id) after a completion is **REJECTED** with `EOrchestratorDispatchRawOutput` — exactly the 4-scenario matrix `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs` already encodes, driven here from the Orchestrator Spirit as a dev-dep.
- The **Worker** proven against the **real** CliWrapper admission path: PATH/explicit-path resolution, the 2s `--maos-bridge-probe` probe, the `output_shape_version` assertion (**fail-loud** `EOutputShapeAdapterMismatch`, journaled as `FrameKindLabel::CliWrapperShapeMismatch`), the `Scope::CliSubprocessSpawn` cap-token TOCTOU `argv_prefix_hash` binding, and `FrameKind::CliSubprocessOutput = 21` provenance rows — all over the real `maos-kernel-core` adapters as dev-deps.
- The **Architect→Reviewer code-review loop**: Orchestrator dispatches `task.assign` to Architect → Architect emits `task.complete` → Orchestrator distills (I11) → dispatches to Reviewer with the **distillate** ref → Reviewer critiques → distillate flows back through Orchestrator. Halt-and-resume preserves the in-flight work across an overnight pause/resume.
- The **runnable headline artifact**: a **`smoke-founder-loop-8-4`** one-shot in `maos-bin` (mirrors `smoke-orchestrator-fanout-6-2`, Decision G), wired into `discipline.yml`, running the full loop at a **compressed timeline** (11pm-assign → distillate dispatch overnight → halt-and-resume across the pause → 7am digest) and proving the wedge's distillates carry **citable `source_log_ref`s** (the I11 chain), reusing the **existing FR17 digest path** (Butler 8.1 / Researcher 8.2) — no new digest engine (Decision H).
- **Halt-and-resume-overnight** over the **existing** `OrchestratorBuffer::recall_all_pending()` (FR20) + the hot-swap `state_codec` CBOR envelope (FR51 / ADR-017) — no new resume mechanism (Decision I).
- The **J1 founder-loop latency budget** (CliWrapper IPC < **25ms P95**, §13.1) verified via the **already-shipped** `maos-bench` J1 harness (`crates/maos-bench/src/harness/j1.rs`, `J1_P95_BUDGET_US = 25_000`, the `section_13_1` Criterion bench), with a `founder-loop-bench` CI job mirroring `researcher-bench` (Decision F).
- **Fixtures** (canned CLI output, deterministic Architect/Reviewer inputs, wedge scenario) authored under each crate's `tests/fixtures/`, **SHA-256-pinned per Story 0.3** and registered in `tests/coverage-matrix.yaml` / corpus-staleness.

**This story IS NOT:**
- It does **NOT** complete the live CliWrapper **runtime stdio bridge** (line-by-line live `claude code` / `opencode` / `gemini-cli` / `kimi-cli` stdout/stderr → Transparency Log). That bridge is **scaffolding in `maos-kernel-core`** (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`), explicitly **deferred from Story 6.2** ("v0.5-α scaffolding by design; full bridge lands in Story 6.5 / Epic 8 Worker pattern", `deferred-work.md`). Completing it is **kernel KLOC** and collides with epic-8's "**Zero kernel KLOC**" mandate + the Story 0.2 kernel-API invariant. The Worker fixture-replays its CLI exactly as Butler fixture-replays calendar/comms (8.1 Decision B) and Observer fixture-replays syscalls (8.3 Decision E). Live multi-CLI capture = a dedicated **non-Epic-8** kernel story (Decision B/D, user-confirmed; carry-forward).
- It does **NOT** add a `FrameKind`, `FramePayload`, or `SpiritRole` variant. The frozen v1.0 ABI already has `TaskAssign=0`, `TaskComplete=1`, `CliSubprocessOutput=21`, `Distillate`, `EpistemicHalt`, and `SpiritRole={Director,Observer,Worker,Orchestrator}`. **Architect and Reviewer use `SpiritRole::Worker`** (Decision C) — adding `Architect`/`Reviewer` variants would break the ABI freeze. `abi-diff` stays Added-only/`removed=[]`.
- It does **NOT** add the `orchestrator-bmad` / Developer-Worker / Reviewer-Worker **skill-package overlays** or a `maos-bridge` crate (the §6.7 *production* form). Those are the v0.9+ live-CLI form; at this substrate level the wedge is realized as the reference Spirit crates + the CliWrapper `[cli_wrapper]` config + fixture-replay (user-confirmed Q1; carry-forward).
- It does **NOT** add a live **LLM** to Architect/Reviewer. Their cognition is deterministic/fixture-driven at v0.8 (no live `provider.complete` in CI), exactly the Butler/Researcher/Observer fixture-replay precedent. Live generative behavior is application-layer (v0.9+/Epic 9) (Decision E).
- It does **NOT** require real **cross-Host A2A**. The single-machine founder loop uses the in-process bus; the loopback A2A profile already exists (E6 6.3) but the cross-Host bilateral pair is **Story 8.5 / v1.5** (Mira+Nash). No A2A AC here.
- It does **NOT** add any cognition (dispatch policy, distillation, anomaly classification, threshold comparison) to `maos-kernel-core`. All four Spirits' logic is Spirit-side; the kernel-API surface invariant (Story 0.2) stays GREEN (any new kernel public fn = class `other` → build-break). The FR20 buffer, FR21 gate, CliWrapper admission, I11 distillation, and hot-swap codec are **consumed, not modified**.
- It does **NOT** re-implement the morning digest. The FR17 digest path shipped in Butler (8.1) and Researcher (8.2); 8.4 proves the wedge's distillates carry citable `source_log_ref`s and feeds that path (Decision H).

## LOCKED Design Decisions (do NOT silently re-decide — chosen during story creation; flagged for Winston)

> **RESOLVED by Winston 2026-06-03 — see "Architect Rulings" at the tail.** All six flagged decisions are CONFIRMED; Decisions B/F carry refinements and a new **Decision K** (distillation authorship) was added. The `FLAG Winston` notes below are retained as the original rationale; the rulings are authoritative.

**Decision A — Four founder-loop reference Spirit crates under `spirits/`; workspace count 33 → 37. (USER-CONFIRMED.)**
The user selected "Rust crates + fixture-replay" over the §6.7 skill-overlay form. Ship `spirits/orchestrator/`, `spirits/worker/`, `spirits/architect/`, `spirits/reviewer/` as workspace members (mirrors Butler/Researcher/Observer Decision A). This reconciles the epic AC ("the Orchestrator reference Spirit in `spirits/orchestrator/`") with architecture §6.7 ("skill-package overlays … NOT first-class Rust Spirit crates") — the crate form is chosen for the v0.8/v0.9 reference deliverable; the §6.7 skill-overlay form is the v0.9+ *production* realization (carry-forward). `check-workspace-count` floor moves **33 → 37**; AC8 updates the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` (currently 33) to 37. **FLAG Winston:** confirm four crates under `spirits/` (count → 37) vs. §6.7 skill overlays.

**Decision B — Worker is a `CliWrapperSpirit` (`[cli_wrapper]` manifest) wrapping a real in-crate fixture-CLI; the live multi-CLI stdio bridge is deferred. (USER-CONFIRMED.)**
`spirits/worker/manifest.toml` declares `[cli_wrapper]` (`command`, `argv_prefix`, `output_shape_version`, `skill_bundle`, `recovery_policy`, `posture{stdio_shape, control_channel, shutdown_signal}`) — **NOT** `[class]` (mutually exclusive; the validator fires `EManifestSchemaConflict` if both appear). The crate ships a **real fixture-CLI binary** (`src/bin/worker-cli-fixture.rs`) that responds to `--maos-bridge-probe` with the declared `output_shape_version` and echoes canned output. This exercises the **real** Story-6.2 admission path (PATH resolution, 2s probe, shape assertion, FR40 journaling, `Scope::CliSubprocessSpawn` cap-token + `argv_prefix_hash` TOCTOU binding, `FrameKind::CliSubprocessOutput=21` rows) over real `maos-kernel-core` adapters as dev-deps — but **does not** complete the live line-by-line stdio bridge for `claude`/`opencode`/`gemini-cli`/`kimi-cli` (that is kernel code scaffolded in `runtime.rs`, deferred from 6.2 to a non-Epic-8 story; completing it would breach the zero-kernel-KLOC mandate + Story 0.2). **FLAG Winston:** confirm the fixture-CLI-binary Worker (real admission path, fixture content) vs. completing the live stdio bridge now.

**Decision C — Architect and Reviewer map to `SpiritRole::Worker`; NO new `SpiritRole` variant (ABI frozen).**
`SpiritRole` is `{Director, Observer, Worker, Orchestrator}` (`crates/maos-spirit-abi/src/identity.rs:127`) — there is **no** `Architect`/`Reviewer` variant and the ABI is frozen at v1.0. Architect and Reviewer are **specialized Workers** in the founder loop; they carry `SpiritRole::Worker` in their `FrameAddress.role`. Adding enum variants would break `abi-diff` (Added-only/`removed=[]`). This mirrors 8.3's Decision B reconciliation ("emits an IAC frame" realized via an existing surface, not a new ABI variant). **FLAG Winston:** confirm Architect/Reviewer carry `SpiritRole::Worker` vs. a future ABI bump adding the roles.

**Decision D — Zero kernel KLOC: the four Spirits consume existing substrate; the kernel gains no public fn.**
The FR20 `OrchestratorBuffer`, the FR21 dispatch gate (`EOrchestratorDispatchRawOutput`), CliWrapper admission, the I11 distillation chain (`DistillationPort`/`LogRecallPort`), and the hot-swap `state_codec` **already exist** (E3/E4/E5/E6). The four Spirits drive them via **dev-dep integration tests** (the resolved 8.1/8.2/8.3 in-proc-bridge pattern) and add **no** kernel code. `check-empty-kernel` + `check-service-boundary` stay at 0 violations; `kloc-check` is unaffected (Spirits live in `spirits/`, not the kernel KLOC ceiling). **FLAG Winston:** confirm no kernel edits are required (all substrate present).

**Decision E — Architect/Reviewer cognition is deterministic/fixture-driven at v0.8; no live LLM in CI.**
Butler (8.1), Researcher (8.2), and Observer (8.3) all fixture-replay their external drivers and run deterministic compute in CI. Architect and Reviewer do the same: `Architect::propose(spec)` and `Reviewer::review(design)` are pure, seeded, bit-identical (NFR-Testability-1). The manifest's `provider.complete` (if mandatory per the validator — verify, per 8.3 Decision G) is declared-but-unused-at-v0.8; live generation is application-layer (v0.9+/Epic 9). **FLAG Winston:** confirm deterministic Architect/Reviewer at v0.8 (live LLM deferred).

**Decision F — J1 latency AC IS in scope; measured via the existing `maos-bench` J1 harness.**
Unlike 8.3 (no latency AC), the epic gives 8.4 a J1 budget: "Founder-loop CliWrapper IPC < 25ms P95 (§13.1)". The harness **already exists** — `crates/maos-bench/src/harness/j1.rs` (spawns a synthetic CliWrapper-shaped Spirit, ≥1000 echo invocations, `J1_P95_BUDGET_US = 25_000`), runnable via `cargo bench -p maos-bench --bench section_13_1 -- --test`. AC7 verifies it and wires a `founder-loop-bench` CI job mirroring `researcher-bench` (`discipline.yml:570`). The epic's escape hatch — "the budget is met **or** §13.1 measurement triggers rust-inproc evaluation in E5 Story 5.5" — is honored: a breach is **recorded** (not silently passed) as a §13.1/E5-5.5 trigger, not masked by migrating to inproc (§13.1: "Fix our code first"). **FLAG Winston:** confirm the existing J1 harness satisfies the 8.4 latency AC (vs. a new founder-loop-specific bench), and that a breach routes to the E5-5.5 rust-inproc evaluation rather than failing the gate outright.

**Decision G — The runnable headline artifact is a `smoke-founder-loop-8-4` one-shot in `maos-bin`.**
Mirroring `smoke-orchestrator-fanout-6-2` (`crates/maos-bin/src/main.rs:3540`, `MAOS_ONE_SHOT` dispatch, wired into `discipline.yml`), the wedge demo is a **runnable** one-shot exercising the full loop at a compressed timeline: Director assigns → Orchestrator buffers (FR20) + dispatches distillate-fed (FR21) → Architect proposes → Reviewer critiques → distillate flows through Orchestrator → halt-and-resume across a (compressed) overnight pause → digest cites actual log refs, plus one deliberate `EOrchestratorDispatchRawOutput` rejection (observable in the TL). `maos-bin` smoke code is **not** kernel KLOC (`kloc-check` counts `maos-kernel-core`; 6.2 set this precedent). This is the observable end-to-end demo `[[feedback_lunarpulse_observability_preference]]`. **FLAG Winston:** confirm the wedge headline is a `maos-bin` `smoke-founder-loop-8-4` one-shot (vs. a spirits-side integration test only).

**Decision H — Morning digest reuses the EXISTING FR17 path; 8.4 proves citable `source_log_ref`s, not a new digest engine.**
The FR17 morning-digest implementation shipped in Butler (8.1) and Researcher (8.2). 8.4 does **not** re-implement it; it proves the wedge's distillates carry **citable `source_log_ref`s** (the I11 chain via the real `DistillationPort`) so the existing digest path "cites source log refs for all claimed completions" against the **actual** Transparency Log. **FLAG Winston:** confirm digest reuse (8.4 ships the citable distillate chain, not a digest re-impl).

**Decision I — Halt-and-resume-overnight uses the EXISTING `OrchestratorBuffer::recall_all_pending` + hot-swap `state_codec`.**
The pause drains the buffer (`recall_all_pending()` → FIFO `Vec`), snapshots via the CBOR `state_codec` (ADR-017 `schema_version` envelope), and the resume re-enqueues. No new resume mechanism. **FLAG Winston:** confirm the FR20-buffer-drain + hot-swap-codec resume path (vs. a persistence-layer snapshot).

**Decision J — Recommended dev model: `claude-opus-4-8`.**
Rationale: the most integration-heavy story in Epic 8 — four cooperating Spirits over the real FR20 buffer, the FR21 dispatch gate, CliWrapper admission (real subprocess probe), the I11 distillation chain, the hot-swap codec, AND the J1 bench, all driven as dev-deps. Memory records deepseek-v4-pro is weak on async invariants / integration plumbing / env-var threading; the in-proc Spirit→adapter bridge (the 8.1/8.2/8.3 risk class) recurs four-fold here, plus a real subprocess-spawning Worker. 8.1/8.2/8.3 all used `claude-opus-4-8`.

**Decision K — Distillation authorship is PRODUCER-side; the Orchestrator REFERENCES + the kernel runs I13 `admit_for_consumer`. (Winston ruling 2026-06-03.)**
The I11 chain is participant-scoped (`LogRecallPort` / `write_distillate(spirit_pid, …)`; the 8.2/8.3 scope-wall) — the Orchestrator **cannot** walk a Worker's emitter frames without a `ScopeViolation`. So the **producing** Spirit (Worker/Architect/Reviewer) distills its OWN output via `write_distillate(producer_pid, …)` (it owns those source frames), emitting a `Distillate` frame; the **Orchestrator references** that distillate in the next `task.assign` (`PriorDistillateRef`), and the kernel enforces `DistillationPort::admit_for_consumer` (I13: digest `intent_lineage ⊆` Orchestrator's allowed-promotion set). This matches the 6.2 reference (`orchestrator_distillate_dispatch.rs` scenario 2.2: the distillate row pre-exists; the Orchestrator only references it) and reconciles the epic's loose "Orchestrator distills the output" with the participant-scoped substrate. The Orchestrator's contract is narrow and FR21-enforced: **ALWAYS attach a distillate ref, NEVER raw output.**

## Prerequisites (verified present at story-creation time — re-verify in AC1)

| Prerequisite | Status | Path / Evidence |
|---|---|---|
| Spirit ABI + lifecycle hooks, `#[spirit]` proc-macro, `Ctx` | ✅ PRESENT | `crates/maos-spirit-abi/src/lifecycle.rs`, `…/identity.rs`; `crates/maos-spirit-derive/src/lib.rs` |
| Spirit SDK + local runner + spirit-test harness + v0.5 assert macros | ✅ PRESENT | `crates/maos-spirit-sdk/src/{local_runner.rs,spirit_test/{harness.rs,assert.rs,manifest.rs}}` |
| **`OrchestratorBuffer` (FR20)** — enqueue / `dequeue_at_safe_point` / `recall_all_pending` / capacity-32 backpressure | ✅ PRESENT | `crates/maos-kernel-core/src/orchestrator/buffer.rs` (full API + 7 integration tests) |
| **`OrchestratorInstruction`** (`id`, `goal` non-empty, `enqueued_at_ns`) | ✅ PRESENT | `crates/maos-domain/src/orchestrator.rs:25-52` (`EmptyGoal` validation) |
| **FR21 dispatch gate** — `TaskAssignPayload{goal,scope,success_criteria,posture_preferences,prior_distillate_ref}` + `PriorDistillateRef{digest_frame_id,distillation_depth,intent_lineage}` + `EOrchestratorDispatchRawOutput` | ✅ PRESENT | `crates/maos-domain/src/frame.rs:77-110`; `crates/maos-domain/src/iac_bus_types.rs:100`; **4-scenario reference test** `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs` |
| **IAC bus typed deliver** (`IacBusPort::deliver(IacFrame)`), `FrameAddress{spirit_id,host_id,role}`, `SpiritRole` | ✅ PRESENT | `crates/maos-domain/src/ports/iac_bus.rs:44`; `crates/maos-spirit-abi/src/identity.rs:54-59,127-132` (`SpiritRole={Director,Observer,Worker,Orchestrator}` — **no Architect/Reviewer**, Decision C) |
| **CliWrapperSpirit admission** (real subprocess `--maos-bridge-probe`, 2s timeout, T3 enforce, `EOutputShapeAdapterMismatch`) | ✅ PRESENT | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs:47-226` (`probe_and_verify_shape`, `admit_cli_wrapper_journaled`) |
| **`[cli_wrapper]` manifest section** (`CliWrapperConfig` + `validate`; mutually exclusive with `[class]`) | ✅ PRESENT | `crates/maos-manifest/src/manifest.rs:3479-3598` (`EManifestSchemaConflict` on both) |
| **FR40 journaled refusal** — `FrameKindLabel::CliWrapperShapeMismatch` (`{cli,declared,observed}` diff) + resumption gate | ✅ PRESENT | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs:187-226`; `crates/maos-domain/src/log_recall.rs:82-85`; `crates/maos-kernel-core/tests/cli_wrapper_shape_mismatch_journal_7_4.rs` |
| **`FrameKind::CliSubprocessOutput = 21`** + `Scope::CliSubprocessSpawn{cli_binary_path,argv_prefix_hash,output_shape_version}` (TOCTOU) | ✅ PRESENT | `crates/maos-spirit-abi/src/identity.rs:34`; `crates/maos-domain/src/invariants/i1.rs:96-102` |
| **CliWrapper recovery policy** (`RespawnWithContext`/`RespawnFresh`/`Escalate`) + `handle_subprocess_death` | ✅ PRESENT | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/lifecycle.rs` |
| **I11 distillation chain** — `DistillationPort::write_distillate` / `admit_for_consumer`; `LogRecallPort::recall`/`fetch` (participant-scoped); `AuditChainMissing`/`EDigestAuditChainMissing` | ✅ PRESENT | `crates/maos-domain/src/ports/{distillation.rs,log_recall.rs}`; `crates/maos-domain/src/distillation.rs:13-158`; **Researcher reference consumer** `spirits/researcher/src/lib.rs:309+` |
| **Hot-swap state codec** (FR51 / ADR-017 CBOR `schema_version` envelope) | ✅ PRESENT | `crates/maos-kernel-core/src/hot_swap/state_codec.rs` (`encode`/`decode`) |
| **J1 founder-loop bench** (`J1_P95_BUDGET_US = 25_000`; `section_13_1` Criterion bench; fixture-replay path) | ✅ PRESENT | `crates/maos-bench/src/harness/j1.rs:3-25,121-165`; `crates/maos-bench/src/fixture_replay.rs:36`; `cargo bench -p maos-bench --bench section_13_1 -- --test` |
| **`smoke-orchestrator-fanout-6-2`** (the wedge-demo precedent to mirror) | ✅ PRESENT | `crates/maos-bin/src/main.rs:3540-3799`; `MAOS_ONE_SHOT` dispatch; `.github/workflows/discipline.yml:1155-1171` |
| **FR17 morning-digest path** (Butler/Researcher; `source_log_ref` citation) | ✅ PRESENT | `spirits/butler/src/lib.rs` (digest), `spirits/researcher/src/lib.rs` (distillation+cite) |
| Butler + Researcher + Observer reference crates (structure to mirror) | ✅ PRESENT | `spirits/{butler,researcher,observer}/{Cargo.toml,manifest.toml,src/lib.rs,tests/}` |
| Workspace count gate + authoritative sentinel | ✅ PRESENT (=33) | root `Cargo.toml` members (33 incl. `spirits/{butler,researcher,observer}`); `xtask check-workspace-count`; sentinel `<!-- workspace-count-authoritative -->` in `4-kernel-design.md:115` (declares **33** post-8.3) |
| Kernel-API surface invariant (Story 0.2) | ✅ PRESENT | `.github/workflows/discipline.yml` (`check-service-boundary` → class `other` = build-break); `xtask/src/check_service_boundary.rs`; `check-empty-kernel` |
| CI new-spirit wiring (job + aggregate `needs:`) + bench-job pattern | ✅ PRESENT | `discipline.yml` — `butler-tests`/`researcher-tests`/`observer-tests` + `researcher-bench` (the bench pattern) wired into `aggregate` `needs:` |
| **`spirits/{orchestrator,worker,architect,reviewer}/` + fixtures** | ❌ **ABSENT** — **this story creates them** | none exist today |
| coverage-matrix slots for the four Spirits | ❌ ABSENT (fresh ADDs) | `tests/coverage-matrix.yaml` `reference_spirits` has hello/example/example-ts/butler/researcher/observer; no orchestrator/worker/architect/reviewer |
| Live CliWrapper runtime stdio bridge (claude/opencode/gemini/kimi) | ❌ DEFERRED (Decision B) | `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs` (scaffold); `deferred-work.md` ("full bridge lands … Epic 8 Worker pattern") — **fixture-replayed here; live bridge = non-Epic-8 kernel story** |
| `maos-bridge` crate / skill-package overlays (§6.7 production form) | ❌ DEFERRED (Decision A/B) | concept only (`skill_bundle` placeholder); v0.9+ carry-forward |
| Nash (Senior Architect §6.4) / cross-Host A2A | ❌ DEFERRED | Story 8.5 / v1.5 — out of scope |

## Acceptance Criteria

### AC1 — Prerequisites & scope classified mechanically before wedge work opens

**Given** the prerequisite table above
**When** AC1 runs first
**Then** the dev confirms each ✅ path/symbol still exists — `OrchestratorBuffer::{enqueue,dequeue_at_safe_point,recall_all_pending}`, `TaskAssignPayload`/`PriorDistillateRef`/`EOrchestratorDispatchRawOutput`, `IacBusPort::deliver` + `SpiritRole={Director,Observer,Worker,Orchestrator}`, CliWrapper `probe_and_verify_shape`/`admit_cli_wrapper_journaled` + the `[cli_wrapper]` validator, `FrameKind::CliSubprocessOutput=21` + `Scope::CliSubprocessSpawn`, the I11 `DistillationPort`/`LogRecallPort`, the hot-swap `state_codec`, the J1 bench (`J1_P95_BUDGET_US=25_000`), `smoke-orchestrator-fanout-6-2`, the workspace-count sentinel (=33), the kernel-API gate — and records the result in the Dev Agent Record
**And** the absence of `spirits/{orchestrator,worker,architect,reviewer}/` (and their coverage-matrix slots) is confirmed, and Decisions A–J are recorded as the chosen resolutions, not silently re-decided
**And** `dev_model_used` is recorded/confirmed in the story frontmatter (§A2 hard-fail gate).

### AC2 — Orchestrator buffers director instructions and processes them at safe sequence points (FR20)

**Given** the Orchestrator reference Spirit in `spirits/orchestrator/` (rust-inproc `[class]`, posture `autonomous-with-halt`, §6.7)
**When** Orchestrator receives buffered instructions from the director (FR20 via E3 Story 3.4)
**Then** instructions are enqueued into the **real** `OrchestratorBuffer` (capacity-32 FIFO, `QueueFull` backpressure) and Orchestrator drains them via `dequeue_at_safe_point()` **only between Worker task completions** — proven by an integration test driving the real buffer as a dev-dep: an instruction enqueued while a delegation is in flight is **not** processed until the delegation completes (FR20 "never preempt in-flight delegations")
**And** the Orchestrator's dispatch decision (order, target role, distillate attachment) lives entirely in `spirits/orchestrator/` — the kernel buffer is consumed, not modified.

### AC3 — Orchestrator distillate-fed dispatch; raw output is rejected (FR21)

**Given** Orchestrator dispatches to Worker/Architect/Reviewer Spirits
**When** a follow-up `task.assign` is built after a prior `task.complete` in the same session
**Then** the **producing** Spirit (Worker/Architect/Reviewer) distills its OWN output via `DistillationPort::write_distillate(producer_pid, …)` (it owns those emitter frames — no `ScopeViolation`; I11 chain), and the **Orchestrator references** the resulting `Distillate` via `PriorDistillateRef` (never the raw `TaskComplete` frame) while the kernel enforces `admit_for_consumer` (I13: digest `intent_lineage ⊆` Orchestrator's allowed-promotion set) — **Decision K** — proven against the **real** FR21 gate as a dev-dep, reproducing the 4-scenario matrix of `orchestrator_distillate_dispatch.rs`: first dispatch (`None`) **accepted**; follow-up with a distillate ref **accepted**; follow-up with `None` after a completion **REJECTED** with `EOrchestratorDispatchRawOutput`; follow-up pointing at a raw `TaskComplete` id **REJECTED**
**And** subsequent dispatches receive **distillates, not raw output** (FR21), and the dispatched frame's `intent_lineage` chains unbroken back to the originating director instruction (I13).

### AC4 — Worker = CliWrapperSpirit; admission fail-loud + cap-token journaling + provenance (FR40/FR52)

**Given** the Worker reference Spirit in `spirits/worker/` declared as a **CliWrapperSpirit** (`[cli_wrapper]` manifest — `command`, `argv_prefix`, `output_shape_version`, `recovery_policy`, `posture{stdio_shape,control_channel,shutdown_signal}`; **NOT** `[class]`)
**When** Worker is admitted wrapping the in-crate fixture-CLI binary (responds to `--maos-bridge-probe`, echoes canned output)
**Then** the **real** admission path runs over `maos-kernel-core` as a dev-dep: PATH/explicit-path resolution, the 2s `--maos-bridge-probe` probe, and the `output_shape_version` assertion — a **matching** shape **admits**; a **mismatched** shape **fails loud** with `EOutputShapeAdapterMismatch`, journaled as a `FrameKindLabel::CliWrapperShapeMismatch` row carrying the `{cli, declared, observed}` diff (FR40; **no** best-effort fallback parsing), and the resumption gate refuses a silent restart until the config is corrected
**And** the `Scope::CliSubprocessSpawn` cap-token's `argv_prefix_hash` TOCTOU binding is verified, the capability-token authority used is **journaled**, and the fixture-CLI's stdout/stderr is captured to the Transparency Log as `FrameKind::CliSubprocessOutput = 21` rows **with provenance** (the invoking Spirit's `intent_lineage`)
**And** a manifest declaring both `[cli_wrapper]` and `[class]` is rejected with `EManifestSchemaConflict` (negative)
**And** the live multi-CLI stdio bridge (`claude`/`opencode`/`gemini-cli`/`kimi-cli`) is **explicitly out of scope** (Decision B; deferred kernel work) — the fixture-CLI proves the real admission/journaling path without it.

### AC5 — Architect → Reviewer code-review loop (deterministic at v0.8)

**Given** the Architect (`spirits/architect/`) and Reviewer (`spirits/reviewer/`) reference Spirits (rust-inproc, `SpiritRole::Worker` — Decision C; deterministic compute — Decision E)
**When** the founder-loop wedge runs
**Then** Architect proposes a design (`Architect::propose`, seeded/bit-identical) → Orchestrator distills it (I11) → dispatches to Reviewer with the **distillate** ref → Reviewer critiques (`Reviewer::review`, seeded) → the critique distillate flows back through Orchestrator
**And** each hop's `task.assign` carries a `PriorDistillateRef` (never raw), so the loop is FR21-clean end-to-end
**And** neither Spirit invokes a live LLM in CI (fixture-driven), and their manifests declare `SpiritRole::Worker` (no new ABI variant).

### AC6 — Founder-loop wedge demo: halt-and-resume-overnight, runnable, digest cites actual log refs (the headline)

**Given** the runnable `smoke-founder-loop-8-4` one-shot in `maos-bin` (Decision G; mirrors `smoke-orchestrator-fanout-6-2`)
**When** it runs at a compressed timeline (11pm-assign → overnight dispatch → pause → 7am)
**Then** the full loop executes end-to-end: Director assigns an overnight task → Orchestrator buffers (FR20) + dispatches **distillate-fed** (FR21) to Architect → Architect proposes → Reviewer critiques → distillate flows through Orchestrator → a **halt-and-resume across an overnight pause** preserves the in-flight work (Orchestrator drains via `recall_all_pending()` and snapshots via the hot-swap `state_codec`; resume re-enqueues — Decision I), and one deliberate `EOrchestratorDispatchRawOutput` rejection is observable in the Transparency Log
**And** the morning digest **cites actual `source_log_ref`s** for all claimed completions, resolved against the **real** Transparency Log via the existing FR17 path (Decision H) — the wedge's distillates carry the I11 chain so the citation is to real frames, not synthetic ones
**And** the one-shot is wired into `discipline.yml` (`MAOS_ONE_SHOT=smoke-founder-loop-8-4`, with `timeout-minutes`) and exits 0 on the happy path.

### AC7 — J1 founder-loop latency budget (CliWrapper IPC < 25ms P95, §13.1)

**Given** the J1 latency budget (Founder-loop CliWrapper IPC < 25ms P95 per §13.1)
**When** the founder-loop benchmark runs (the existing `maos-bench` J1 harness — `J1_P95_BUDGET_US = 25_000`; `cargo bench -p maos-bench --bench section_13_1 -- --test` — Decision F)
**Then** the budget is **met**, OR a breach is **recorded** as a §13.1 measurement that triggers the rust-inproc evaluation in E5 Story 5.5 (the epic escape hatch) — not masked by migrating to inproc (§13.1: "Fix our code first"; do not migrate to mask code-path overhead)
**And** a `founder-loop-bench` CI job is added (mirroring `researcher-bench`, `--test` mode) and wired into the gate aggregation
**And** the J1 measurement basis (synthetic CliWrapper-shaped Spirit, ≥1000 echo invocations) and the pass/breach outcome are recorded in the Dev Agent Record.

### AC8 — Zero kernel KLOC; ABI frozen; workspace count reconciled; manifests conform

**Given** the four Spirits are rust-inproc / CliWrapper reference crates (zero kernel KLOC)
**When** their logic is added
**Then** all logic lives in `spirits/{orchestrator,worker,architect,reviewer}/`, **not** in `maos-kernel-core` — the Story 0.2 kernel-API surface invariant stays GREEN (`check-empty-kernel` + `check-service-boundary` = 0 violations; **no** new kernel public fn; the kernel gains no dispatch/distillation/anomaly function), and each crate keeps `maos-kernel-core` / `maos-director-surface` / `maos-bench` adapters in `[dev-dependencies]` only (the 8.1/8.2/8.3 pattern: Spirit-side deps = `maos-spirit-sdk` + `maos-spirit-abi` + `maos-domain` + serde)
**And** **no new `FrameKind`, `FramePayload`, or `SpiritRole` variant** is added — `abi-diff` is **Added-only with `removed=[]`** (Architect/Reviewer use `SpiritRole::Worker`, Decision C); if any ABI delta appears, STOP and flag (mis-scoped)
**And** `check-workspace-count` is reconciled to **37** (Decision A): root `Cargo.toml` members + the `<!-- workspace-count-authoritative -->` sentinel in `4-kernel-design.md` both updated 33 → 37
**And** every manifest passes `maos-manifest` validation with each section verified against the authoritative validators before authoring (`deny_unknown_fields`; Worker uses `[cli_wrapper]` not `[class]`; Orchestrator posture `autonomous-with-halt`; Architect/Reviewer resolve the `provider.complete` mandatory-vs-optional question per Decision E).

### AC9 — Fixtures authored, SHA-pinned, and registered (Story 0.3); CI / discipline green end-to-end

**Given** the deterministic test inputs (canned CLI output, seeded Architect/Reviewer inputs, the wedge scenario — no live CLI, no live LLM in CI)
**When** the fixtures are authored under each crate's `tests/fixtures/`
**Then** they are SHA-256-pinned per Story 0.3 (a pin test mirroring `spirits/observer`'s `fixtures_pin` / `spirits/researcher`'s `corpus_pin`) and registered in the corpus-staleness / `tests/coverage-matrix.yaml` surfaces; the new `orchestrator`/`worker`/`architect`/`reviewer` slots are added to `reference_spirits` (`path: spirits/<name>`, `ships_at: "v0.8"`, `third_party: false`)
**And** per-crate `*-tests` jobs (`cargo test -p <name> --locked`, with `timeout-minutes`) + the `founder-loop-bench` job + the `smoke-founder-loop-8-4` step are added to `.github/workflows/discipline.yml` and wired into the gate-aggregation `needs:` list (mirrors `researcher-tests`/`observer-tests`/`researcher-bench`)
**And** `check-service-boundary` (0 new violations), `check-empty-kernel` (0), `check-workspace-count` (37/37), `coverage-matrix`, `corpus-staleness`, `abi-diff` (Added-only/`removed=[]`), `kloc-check`, and the §A2 `check-dev-model-used-populated` gate are all GREEN at HEAD — **no flipped-while-red** (the Epic 7 §A2 trap)
**And** the full `cargo test -p {orchestrator,worker,architect,reviewer} --locked` suites pass, the Butler + Researcher + Observer regressions stay clean (0 failures), and the Dev Agent Record lists every file created/modified, with any pre-existing RED verified wedge-neutral (identical clean-HEAD-vs-changes) and flagged, not introduced.

## Tasks / Subtasks

- [x] **T1 — Prerequisite + scope pre-check (AC1)**
  - [x] Re-verify every ✅ row (paths + key symbols): `OrchestratorBuffer`, `TaskAssignPayload`/`PriorDistillateRef`/`EOrchestratorDispatchRawOutput`, `IacBusPort::deliver` + `SpiritRole` (no Architect/Reviewer), CliWrapper `probe_and_verify_shape`/`admit_cli_wrapper_journaled` + `[cli_wrapper]` validator, `FrameKind::CliSubprocessOutput=21` + `Scope::CliSubprocessSpawn`, `DistillationPort`/`LogRecallPort`, `state_codec`, J1 bench (25_000 us), `smoke-orchestrator-fanout-6-2`, workspace sentinel (=33), kernel-API gate; record in Dev Agent Record
  - [x] Confirm the four `spirits/*` crates + coverage-matrix slots ABSENT; record Decisions A–J
  - [x] Confirm/set `dev_model_used` frontmatter (§A2 gate)
- [x] **T2 — Scaffold Orchestrator crate (AC2, AC8; Decision A/D)**
  - [x] Create `spirits/orchestrator/` mirroring `spirits/researcher/` shape; Spirit-side deps = `maos-spirit-sdk[local_runner]` + `maos-spirit-abi` + `maos-domain` + serde; dev-deps = `maos-spirit-sdk[local_runner,mock,spirit_test]` + `maos-kernel-core` + `maos-manifest` + tokio/tempfile/sha2/toml
  - [x] `manifest.toml`: `[class]` (orchestrator, 0.8.0, abi=1.0, schema=2, min_substrate_version, forms=["rust-inproc"], trust_tier="local"); `[posture] default=allowed_max="autonomous-with-halt"` (§6.7); `[output_shape]` (dispatch surface — verify fields); `[budget]`/`[resources]`; `[sandbox]`; `[epistemic_policy]` halt-policy preferring recall (verify rule shape); validate via `tests/spirit_smoke.rs`
  - [x] Implement Orchestrator: drain `OrchestratorBuffer` at safe points (never preempt in-flight); build `TaskAssignPayload` + attach `PriorDistillateRef`; dispatch decision Spirit-side
  - [x] Integration test `tests/orchestrator_buffer.rs` (FR20): enqueue-while-in-flight ⇒ not processed until completion; FIFO; `QueueFull` at 32 — real buffer as dev-dep
- [x] **T3 — Orchestrator FR21 distillate-fed dispatch (AC3)**
  - [x] **Producer** distills its OWN output via `DistillationPort::write_distillate(producer_pid, …)` (Decision K — producer owns the source frames; Orchestrator distilling them would `ScopeViolation`); **Orchestrator references** the `Distillate` via `PriorDistillateRef`; kernel runs `admit_for_consumer` (I13)
  - [x] Integration test `tests/distillate_dispatch.rs` reproducing the 4-scenario matrix against the real FR21 gate (accept None-first / accept distillate-ref / REJECT None-after-completion / REJECT raw-TaskComplete-ref ⇒ `EOrchestratorDispatchRawOutput`); write the distillate row BEFORE the Orchestrator references it (mirror `orchestrator_distillate_dispatch.rs` 2.2); assert unbroken `intent_lineage` (I13)
- [x] **T4 — Worker CliWrapperSpirit crate (AC4; Decision B)**
  - [x] Create `spirits/worker/`; `manifest.toml` with `[cli_wrapper]` (command → in-crate fixture-CLI, `output_shape_version`, posture `{stdio_shape,control_channel,shutdown_signal}`, recovery_policy) — NOT `[class]`; **`[sandbox] tier = "T3"`** (Story 6.2 AC6 rejects CliWrapper below T3 — `ECliWrapperRequiresT3`; Decision B refinement); validate (and assert `EManifestSchemaConflict` on a both-sections negative fixture)
  - [x] Ship `src/bin/worker-cli-fixture.rs`: answers `--maos-bridge-probe` with the declared `output_shape_version`; echoes canned output
  - [x] Integration test `tests/cli_wrapper_admission.rs` over real `maos-kernel-core` (dev-dep): matching shape admits; mismatched shape ⇒ `EOutputShapeAdapterMismatch` journaled as `CliWrapperShapeMismatch` (`{cli,declared,observed}`); resumption-gate no silent restart; `Scope::CliSubprocessSpawn` `argv_prefix_hash` TOCTOU bind + cap-token journaled; `CliSubprocessOutput=21` provenance rows
- [x] **T5 — Architect + Reviewer crates (AC5; Decision C/E)**
  - [x] Create `spirits/architect/` + `spirits/reviewer/` (rust-inproc, `SpiritRole::Worker`); deterministic `propose`/`review` (seeded, bit-identical); resolve `provider.complete` mandatory-vs-optional per validator (declare-but-unused if mandatory)
  - [x] Integration test `tests/code_review_loop.rs`: Architect→distill→Reviewer hop, each `task.assign` carries a distillate ref (FR21-clean); no live LLM
- [x] **T6 — Founder-loop wedge demo: `smoke-founder-loop-8-4` (AC6; Decision G/H/I)**
  - [x] Add `smoke_founder_loop_8_4()` to `maos-bin/src/main.rs` (mirror `smoke_orchestrator_fanout_6_2`): full loop at compressed timeline; FR20 buffer + FR21 distillate dispatch + Architect→Reviewer + halt-and-resume via `recall_all_pending()` + `state_codec` + one deliberate `EOrchestratorDispatchRawOutput`; digest cites actual `source_log_ref`s via the FR17 path
  - [x] Wire `MAOS_ONE_SHOT=smoke-founder-loop-8-4` dispatch + the `discipline.yml` step (`timeout-minutes`)
- [x] **T7 — J1 latency (AC7; Decision F)**
  - [x] Run the existing J1 harness (`cargo bench -p maos-bench --bench section_13_1 -- --test`); record pass or the §13.1/E5-5.5 breach trigger
  - [x] Add `founder-loop-bench` CI job (mirror `researcher-bench`, `--test`) + wire into aggregate
- [x] **T8 — Fixtures: author, SHA-pin, register (AC9)**
  - [x] Author each crate's `tests/fixtures/` (canned CLI output, seeded Architect/Reviewer inputs, wedge scenario); deterministic generator if used (env-gated, bit-identical)
  - [x] SHA-256 pin tests (mirror observer/researcher); ADD `orchestrator`/`worker`/`architect`/`reviewer` slots to `tests/coverage-matrix.yaml` (`ships_at: v0.8`); run `coverage-matrix` + `corpus-staleness`
- [x] **T9 — Zero-kernel-KLOC / ABI / workspace count (AC8)**
  - [x] Confirm no `maos-kernel-core` edits; `check-empty-kernel` + `check-service-boundary` (0 violations); `abi-diff` Added-only/`removed=[]` (no new FrameKind/FramePayload/SpiritRole)
  - [x] Add the four `spirits/*` to root `Cargo.toml` members (→37); bump the `4-kernel-design.md` sentinel 33→37; run `check-workspace-count` (37/37)
- [x] **T10 — CI / discipline green (AC9)**
  - [x] Add `orchestrator-tests`/`worker-tests`/`architect-tests`/`reviewer-tests` jobs + the `founder-loop-bench` + `smoke-founder-loop-8-4` step; wire all into the `aggregate` `needs:` list
  - [x] Verify all AC9 gates GREEN at HEAD; pre-existing reds verified wedge-neutral; no flipped-while-red; File List complete

## Dev Notes

### Spirit form & scaffolding (mirror Researcher/Observer 8.2/8.3 — the closer templates)
- **Three crates rust-inproc `[class]`** (orchestrator/architect/reviewer); **one crate CliWrapper `[cli_wrapper]`** (worker). Scaffold by copying `spirits/observer/` shape: Spirit-side deps only in `[dependencies]`; real kernel adapters in `[dev-dependencies]` so integration is PROVEN without violating Story 0.2. Keep state in `Arc<Mutex<...>>` with poison-safe `unwrap_or_else(|e| e.into_inner())` (the 8.2/8.3 review fix). The `#[spirit]` macro synthesizes no-op bodies for unused hooks.
- **`Ctx` exposes only opaque handles** (`cancellation()`, `capability()`, `mailbox()`, `deprecation_warnings()`). A lifecycle hook cannot reach kernel services directly — the FR20 buffer, FR21 gate, CliWrapper admission, distillation, and hot-swap codec integrations are proven in tests that drive the **real adapters as dev-dependencies** (the resolved 8.1/8.2/8.3 pattern). This is the single most likely place to lose a review cycle — do NOT reach into `maos-kernel-core` from any `spirits/*` lib.

### FR20 — OrchestratorBuffer (AC2)
- `crates/maos-kernel-core/src/orchestrator/buffer.rs`: `enqueue(OrchestratorInstruction) -> Result<(),OrchestratorBufferError>` (capacity 32, `QueueFull`), `dequeue_at_safe_point() -> Option<_>` (**between task completions ONLY — never from kernel hooks**; doc: "would violate FR20's 'never preempt in-flight delegations'"), `recall_all_pending() -> Vec<_>` (FIFO drain for FR51 resume), `pending_count()`, `capacity()`.
- `OrchestratorInstruction{id, goal (non-empty/`EmptyGoal`), enqueued_at_ns}` (`crates/maos-domain/src/orchestrator.rs:25-52`).

### FR21 — distillate-fed dispatch + the gate (AC3)
- `TaskAssignPayload{goal, scope, success_criteria, posture_preferences, prior_distillate_ref}` + `PriorDistillateRef{digest_frame_id (must resolve to a `Distillate` row), distillation_depth, intent_lineage}` (`crates/maos-domain/src/frame.rs:77-110`).
- The **gate is kernel-side**: a follow-up `task.assign` with `prior_distillate_ref=None` (or pointing at a raw `TaskComplete`) after a completion is rejected with `IacBusError::EOrchestratorDispatchRawOutput{orchestrator,task_id}` (`crates/maos-domain/src/iac_bus_types.rs:100`). The Orchestrator Spirit's job is to ALWAYS distill + attach the ref correctly; the reference 4-scenario matrix is `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs` — mirror it from the Spirit as a dev-dep.
- Distill via `DistillationPort::write_distillate(spirit_pid, DistillationRequest{source_log_ref (non-empty), distillation_depth (≥1), digest_payload, segment_hint})` → `DistillationReceipt{digest_frame_id, intent_lineage (kernel-computed I13 union), effective_source_log_ref (flattened to raws), effective_distillation_depth}`; `AuditChainMissing` ⇒ `EDigestAuditChainMissing` (I11). Researcher (`spirits/researcher/src/lib.rs:309+`) is the reference consumer.
- **Distillation authorship (Decision K — Winston):** the `spirit_pid` arg to `write_distillate` is the **producer** (Worker/Architect/Reviewer), because the I11 source frames are participant-scoped to whoever emitted them — the Orchestrator distilling a Worker's frames would `ScopeViolation` (the 8.2/8.3 scope-wall). So the **producer distills its own output**; the **Orchestrator references** the resulting `Distillate` (`PriorDistillateRef`) and the kernel runs `admit_for_consumer(digest_frame_id, orchestrator_allowed_promotion_set)` (I13). The 6.2 reference (`orchestrator_distillate_dispatch.rs` 2.2) writes the distillate row first, then the Orchestrator references it — mirror that ordering. The Orchestrator's only FR21 contract: ALWAYS attach a distillate ref, NEVER raw.

### FR40/FR52 — Worker CliWrapperSpirit (AC4; Decision B)
- `[cli_wrapper]` manifest (`crates/maos-manifest/src/manifest.rs:3479-3598`): `command`, `argv_prefix`, `output_shape_version` (semver), `skill_bundle`, `recovery_policy` (`RespawnWithContext`/`RespawnFresh`/`Escalate`), `posture{stdio_shape∈{NdjsonOverStdio,JsonRpcOverStdio,Raw}, control_channel∈{Signals,NamedPipe,StdinCommands}, shutdown_signal∈VALID_SIGNALS}`. **Mutually exclusive with `[class]`** — both ⇒ `EManifestSchemaConflict`. `command` must be non-empty.
- Admission (`crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs:47-226`): real `std::process::Command::new(resolved).args([...argv_prefix, "--maos-bridge-probe"]).spawn()`, 2s timeout, T3 tier enforced; first stdout line = JSON envelope or bare semver; `observed != declared` ⇒ `EOutputShapeAdapterMismatch`. `admit_cli_wrapper_journaled` writes a `FrameKindLabel::CliWrapperShapeMismatch` row (`{cli,declared,observed}`) before returning the error (I2 — panics on journal-write failure; no silent-failure path). Resumption gate: a stale config re-fails on restart; only a corrected config admits (`crates/maos-kernel-core/tests/cli_wrapper_shape_mismatch_journal_7_4.rs`).
- The **in-crate fixture-CLI** (`src/bin/worker-cli-fixture.rs`) makes this a **real** subprocess admission — no mocking of `Command`. `Scope::CliSubprocessSpawn{cli_binary_path, argv_prefix_hash:[u8;32], output_shape_version}` (`crates/maos-domain/src/invariants/i1.rs:96-102`) — re-derive the manifest's `argv_prefix_hash` and assert TOCTOU equality (`runtime.rs::argv_prefix_hash`). Captured output ⇒ `FrameKind::CliSubprocessOutput=21` rows with the invoking Spirit's `intent_lineage`.
- **DO NOT** implement the live line-by-line stdio bridge for real `claude`/`opencode`/`gemini-cli`/`kimi-cli` — that is `runtime.rs` kernel scaffolding deferred from 6.2 (`deferred-work.md`); the fixture-CLI proves the admission/journaling path without it (Decision B; carry-forward).

### Architect/Reviewer (AC5; Decision C/E)
- `SpiritRole={Director,Observer,Worker,Orchestrator}` (`crates/maos-spirit-abi/src/identity.rs:127-132`) — **no Architect/Reviewer**; both carry `SpiritRole::Worker`. Adding variants breaks `abi-diff`.
- Deterministic `propose`/`review` (seeded, bit-identical, NFR-Testability-1) — no live LLM in CI (the Butler/Researcher/Observer fixture-replay precedent). Per 8.3 Decision G, verify whether `[capabilities.required]`/`provider.complete` is mandatory in the validator; if optional, omit; if mandatory, declare-but-unused-at-v0.8.

### Wedge demo + halt-and-resume (AC6; Decision G/H/I)
- Mirror `smoke_orchestrator_fanout_6_2` (`crates/maos-bin/src/main.rs:3540-3799`; `MAOS_ONE_SHOT` dispatch ~line 3130/3198; `discipline.yml:1155-1171`). `maos-bin` smoke code is NOT kernel KLOC (`kloc-check` counts `maos-kernel-core`).
- Halt-and-resume: `OrchestratorBuffer::recall_all_pending()` (FIFO drain) + the hot-swap CBOR `state_codec` (`crates/maos-kernel-core/src/hot_swap/state_codec.rs`, ADR-017 `schema_version` envelope); resume re-enqueues. No new resume mechanism (Decision I).
- Digest: reuse the FR17 path (Butler/Researcher); 8.4 proves the distillates carry citable `source_log_ref`s against the **real** Transparency Log (Decision H) — not a digest re-impl.

### J1 latency (AC7; Decision F)
- [Source: 13-phased-roadmap.md#13.1] "J1 Founder loop CliWrapperSpirit per-tool-call — IPC overhead < 25ms P95". Harness exists: `crates/maos-bench/src/harness/j1.rs` (`J1_P95_BUDGET_US=25_000`, synthetic CliWrapper-shaped Spirit, ≥1000 echo invocations); `cargo bench -p maos-bench --bench section_13_1 -- --test`. §13.1: "J1 is the floor reference … Fix our code first. Do not migrate to inproc to mask code-path overhead." Epic escape hatch: breach ⇒ §13.1 measurement triggers rust-inproc evaluation in E5 Story 5.5 (record it; do not silently pass or mask).

### Testing standards
- SDK spirit-test harness + v0.5 macros (`spirit_test_assert!`, `spirit_test_expect_frame!`, `assert_no_deprecations!`). Real `maos-kernel-core` adapters (`OrchestratorBuffer`, the FR21 IacBus gate, CliWrapper admission, `DistillationPort`/`LogRecallPort`, `state_codec`) driven via dev-deps. All inputs deterministic — fixtures, no live CLI / live LLM in CI. SHA-pin fixtures per Story 0.3; register in corpus-staleness/coverage-matrix. ≥500ms timeouts on any async/telemetry test bound (the 8.2 flake fix).

### Project Structure Notes
- **Four new crates** under `spirits/`: each `Cargo.toml`, `manifest.toml`, `src/lib.rs` (worker also `src/bin/worker-cli-fixture.rs`), `tests/` (spirit_smoke + the per-crate integration tests + fixture pin), `tests/fixtures/`. Add all four to root `Cargo.toml` members (→37); bump the sentinel 33→37; ADD four coverage-matrix slots.
- **No edits** to `maos-kernel-core` (Story 0.2). `maos-bin` gains only the `smoke-founder-loop-8-4` one-shot (not kernel KLOC). The wedge logic is Spirit-side; the real adapters are reached as dev-deps in tests.

### References
- [Source: epics/epic-8-…miranash-v03-v15.md#Story 8.4] — story statement + 5 BDD AC blocks (FR20 buffering / FR21 distillate dispatch / Worker CliWrapperSpirit + FR40 / Architect→Reviewer loop / J1 latency); v0.8/v0.9 acceptance demo "11pm assign → distillate dispatch overnight → 7am digest cites actual log refs"
- [Source: architecture-maos-minimal-opus/6-reference-spirits.md#6.7] — skill-package overlays on CliWrapperSpirit; the §6.7 production form (carry-forward); CliWrapperSpirit spec (output_shape_version assertion, fail-loud, recovery semantics)
- [Source: architecture-maos-minimal-opus/6-reference-spirits.md#6.4/6.5] — Senior Architect (Nash, v1.5 — NOT this story); Observer founder-loop use case
- [Source: architecture-maos-minimal-opus/13-phased-roadmap.md#13.1] — v0.9 Founder Loop wedge demo; J1 CliWrapper IPC <25ms P95; "fix our code first" / rust-inproc escape to E5 5.5
- [Source: architecture-maos-minimal-opus/4-kernel-design.md] — workspace-count sentinel (33→37); §4.0.7 kernel non-interpretive
- [Source: crates/maos-kernel-core/src/orchestrator/buffer.rs + crates/maos-domain/src/orchestrator.rs] — FR20 buffer + `OrchestratorInstruction`
- [Source: crates/maos-domain/src/frame.rs:77-110 + iac_bus_types.rs:100 + crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs] — FR21 `TaskAssignPayload`/`PriorDistillateRef` + `EOrchestratorDispatchRawOutput` + 4-scenario matrix
- [Source: crates/maos-kernel-core/src/lifecycle/cli_wrapper/{admission.rs,lifecycle.rs,runtime.rs} + crates/maos-manifest/src/manifest.rs:3479-3598 + crates/maos-kernel-core/tests/cli_wrapper_shape_mismatch_journal_7_4.rs] — CliWrapper admission/journaling/recovery + `[cli_wrapper]` validator + FR40 mismatch journal
- [Source: crates/maos-spirit-abi/src/identity.rs] — `FrameKind::CliSubprocessOutput=21`, `SpiritRole={Director,Observer,Worker,Orchestrator}` (Decision C), `FrameAddress`
- [Source: crates/maos-domain/src/invariants/i1.rs:96-102] — `Scope::CliSubprocessSpawn` TOCTOU `argv_prefix_hash`
- [Source: crates/maos-domain/src/ports/{distillation.rs,log_recall.rs} + distillation.rs:13-158 + spirits/researcher/src/lib.rs] — I11 distillation chain + reference consumer
- [Source: crates/maos-kernel-core/src/hot_swap/state_codec.rs] — FR51/ADR-017 resume codec
- [Source: crates/maos-bench/src/harness/j1.rs + fixture_replay.rs + .github/workflows/discipline.yml:570 researcher-bench] — J1 harness + bench-job pattern
- [Source: crates/maos-bin/src/main.rs:3540-3799 + discipline.yml:1155-1171] — `smoke-orchestrator-fanout-6-2` wedge-demo precedent (mirror for `smoke-founder-loop-8-4`)
- [Source: spirits/{butler,researcher,observer}/{Cargo.toml,manifest.toml,src/lib.rs,tests/}] — reference crate structure (dev-dep adapters, SHA-pinned fixtures, poison-safe locks, ≥500ms timeouts)
- [Source: _bmad-output/implementation-artifacts/{8-2,8-3}-….md + deferred-work.md] — the in-proc Spirit→adapter-as-dev-dep bridge pattern; the 6.2 CliWrapper-runtime-stdio-bridge deferral
- [Source: tests/coverage-matrix.yaml `reference_spirits`] — slot shape to ADD (orchestrator/worker/architect/reviewer, ships_at v0.8)

## Dev Agent Record

### Agent Model Used

`claude-opus-4-8` (Decision J) — recorded in frontmatter `dev_model_used` per §A2.

### Debug Log References

### Completion Notes List

**T1 — Prerequisite + scope pre-check (AC1) — DONE.** Re-verified every ✅ prereq path/symbol against HEAD `93f7b1f`:
- `OrchestratorBuffer::{new,with_capacity,enqueue,dequeue_at_safe_point,recall_all_pending,pending_count,capacity}` + `OrchestratorBufferError::QueueFull(usize)` — `crates/maos-kernel-core/src/orchestrator/buffer.rs` (capacity floor 32, FIFO).
- `OrchestratorInstruction{id:OrchestratorInstructionId(u64),goal:String,enqueued_at_ns:u64}` + `OrchestratorInstruction::new` (`EmptyGoal`) — `crates/maos-domain/src/orchestrator.rs`.
- `TaskAssignPayload{goal,scope:Vec<Scope>,success_criteria,posture_preferences,prior_distillate_ref:Option<PriorDistillateRef>}` + `PriorDistillateRef{digest_frame_id:[u8;16],distillation_depth:u32,intent_lineage:IntentLineage}` — `crates/maos-domain/src/frame.rs`.
- `IacBusError::EOrchestratorDispatchRawOutput{orchestrator:String,task_id:String}` — `crates/maos-domain/src/iac_bus_types.rs`; real gate exercised via `IacBusAdapter::{register_spirit_typed,deliver_typed}`; reference matrix `crates/maos-kernel-core/tests/orchestrator_distillate_dispatch.rs` (4 scenarios).
- `SpiritRole={Director,Observer,Worker,Orchestrator}` (NO Architect/Reviewer), `FrameKind::CliSubprocessOutput=21`, `FrameAddress{spirit_id,host_id,role}` — `crates/maos-spirit-abi/src/identity.rs`.
- CliWrapper: `probe_and_verify_shape(&CliWrapperConfig,SandboxTier)` + `admit_cli_wrapper_journaled(cfg,tier,pid,&TransparencyLogAdapter)` (`EOutputShapeAdapterMismatch`→`FrameKind::CliWrapperShapeMismatch=27` row; resumption gate) — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/admission.rs`; `CliWrapperConfig`+`CliWrapperPosture{stdio_shape,control_channel,shutdown_signal}`+`EManifestSchemaConflict`+`ECliWrapperRequiresT3` — `crates/maos-manifest/src/manifest.rs:3479+` (re-exported `maos_kernel_core::security::manifest`). `command` non-empty; `[cli_wrapper]`⊥`[class]`.
- `Scope::CliSubprocessSpawn{cli_binary_path,argv_prefix_hash:[u8;32],output_shape_version}` — `crates/maos-domain/src/invariants/i1.rs`; `argv_prefix_hash(&[String])->[u8;32]` — `crates/maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs`.
- I11: `DistillationPort::{write_distillate(pid,DistillationRequest),admit_for_consumer(id,&AllowedPromotionSet)}` + `DistillationRequest::new` + `DistillationReceipt{digest_frame_id,intent_lineage,effective_source_log_ref,effective_distillation_depth}` + `DistillationError::{AuditChainMissing,IntentPromotionDenied,SourceFrameNotFound}` — `crates/maos-domain`; real `DistillateWriter::new(tl,memory)` + `LogRecallAdapter::new(tl)` — `crates/maos-kernel-core/src/iac/{distillate,log_recall}.rs`.
- Hot-swap `state_codec::{encode(&[u8],u32),decode(&[u8],u32)->StateEnvelope}` (CBOR `schema_version` envelope) — `crates/maos-kernel-core/src/hot_swap/state_codec.rs`.
- J1 bench `J1_P95_BUDGET_US=25_000`, `section_13_1` (`cargo bench -p maos-bench --bench section_13_1 -- --test`) — `crates/maos-bench/src/harness/j1.rs`.
- `smoke-orchestrator-fanout-6-2` (`crates/maos-bin/src/main.rs:3540`, `MAOS_ONE_SHOT`) + `discipline.yml` wiring.
- Workspace sentinel `<!-- workspace-count-authoritative -->` = **33** (`4-kernel-design.md:115`); root `Cargo.toml` members = 33 (verified).
- Kernel-API gate (`check-empty-kernel`/`check-service-boundary`).
- **ABSENT (this story creates):** `spirits/{orchestrator,worker,architect,reviewer}/` + their `reference_spirits` coverage-matrix slots — confirmed not present.
- Decisions A–K recorded as the chosen resolutions (Winston rulings 2026-06-03, authoritative). `dev_model_used: claude-opus-4-8` present in frontmatter (§A2 gate).

**T2 — Orchestrator crate (AC2) — DONE.** `spirits/orchestrator/` rust-inproc `[class]`, posture `autonomous-with-halt` + recall-preferring `[epistemic_policy]` (no `[capabilities.required]`, Decision E). Spirit-side safe-point gate (`is_safe_point`/`begin_delegation`/`complete_delegation`/`drain_next`) drives the **real** `OrchestratorBuffer` as a dev-dep — kernel owns the FIFO, Spirit owns the "never preempt in-flight" policy. Typed `first_dispatch`(None)/`followup_dispatch`(ref) split encodes the FR21 contract by construction. `tests/orchestrator_buffer.rs`: enqueue-while-in-flight ⇒ not drained until completion, FIFO across rounds, `QueueFull(32)` (3 tests). 5 unit + 3 smoke green.

**T3 — Orchestrator FR21 distillate dispatch (AC3) — DONE.** `tests/distillate_dispatch.rs` reproduces the 4-scenario matrix against the **real** `IacBusAdapter::deliver_typed` gate: first(None)✅ / followup-with-producer-distillate✅ / None-after-completion❌ / raw-TaskComplete-ref❌ (both `EOrchestratorDispatchRawOutput`). **Decision K**: the producer (worker pid) distills its OWN frames via the real `DistillateWriter::write_distillate` (no ScopeViolation); the Orchestrator only references the `Distillate`; I13 `admit_for_consumer` asserted positive; distillate row written BEFORE the reference (6.2 ordering). 4 tests green. Fix: `register_spirit_typed` handles must be bound (dropping closes the mailbox → ChannelClosed).

**T4 — Worker CliWrapperSpirit (AC4) — DONE.** `spirits/worker/` `[cli_wrapper]` (NOT `[class]`) + `[sandbox] tier="T3"` (Decision B refinement). Ships the **real** in-crate `worker-cli-fixture` binary answering `--maos-bridge-probe` with the JSON output-shape envelope + echoing canned output (env-overridable shape for mismatch tests; no write-then-exec race). `tests/cli_wrapper_admission.rs` (7) over the real `maos-kernel-core` admission as dev-dep: matching shape admits; mismatch → `EOutputShapeAdapterMismatch` journaled as `CliWrapperShapeMismatch={cli,declared,observed}`; resumption gate (no silent restart, corrected admits); below-T3 → `ECliWrapperRequiresT3`; `argv_prefix_hash` TOCTOU bind re-derives (tamper changes hash); captured fixture stdout → `CliSubprocessOutput=21` provenance rows; `EManifestSchemaConflict` negative via `worker::detect_schema_conflict`. Live multi-CLI stdio bridge OUT OF SCOPE (Decision B). 4 unit + 3 smoke green.

**T5 — Architect + Reviewer (AC5) — DONE.** `spirits/architect/` + `spirits/reviewer/` rust-inproc `[class]`, `SpiritRole::Worker` (Decision C — no ABI variant), deterministic/bit-identical `propose`/`review` (Decision E — no `[capabilities.required]`, no live LLM). Reviewer decoupled (own `DesignUnderReview` input, no dep on architect; maps at the seam). `tests/code_review_loop.rs`: Orchestrator→Architect (first) → architect distills own proposal → Orchestrator→Reviewer (followup w/ distillate, **FR21-clean**) → reviewer distills own critique → flows back — all through the real gate + DistillateWriter; 3 dispatches, 0 raw. architect 5 unit + 1 loop + 2 smoke; reviewer 6 unit + 2 smoke.

**T6 — smoke-founder-loop-8-4 (AC6) — DONE.** `smoke_founder_loop_8_4()` in `maos-bin` (deps: orchestrator/architect/reviewer/worker), `MAOS_ONE_SHOT` dispatch wired. **Exits 0**: 11pm FR20 buffer (2 instr) → drain at safe point (never-preempt asserted) → FR21 first dispatch → Architect proposes 3 components, distills → FR21 distillate-fed dispatch to Reviewer → Reviewer approves, distills → deliberate raw dispatch **REJECTED** → 3× `CliSubprocessOutput=21` provenance rows → **halt-and-resume**: `recall_all_pending()` → CBOR `state_codec::encode/decode` (189 bytes) → re-enqueue (in-flight preserved) → **morning digest cites 2 distillate `source_log_ref`s resolving against the real TL** (FR17, Decision H). Reuses existing substrate — zero kernel KLOC.

**T7 — J1 latency (AC7) — DONE; BUDGET MET.** Existing `maos-bench` J1 harness (`section_13_1`, `J1_P95_BUDGET_US=25_000`, synthetic CliWrapper-shaped Spirit, ≥1000 echo invocations) — `--test` mode passes (regression guard). Real criterion measurement `j1_founder_loop_ipc` ≈ **95µs per 10-invocation run (~9.5µs/call)** — far under the 25ms P95 budget. No breach → no §13.1/E5-5.5 trigger needed (Decision F). `founder-loop-bench` CI job added (`--test`, mirrors `researcher-bench`).

**T8 — Fixtures + SHA-pin + register (AC9) — DONE.** Each crate ships a `tests/fixtures/` file that BOTH SHA-pins AND drives a real assertion: worker `canned-cli-output.json` (source of truth for the fixture-CLI constants), architect `design-specs.json` (drives `propose`), reviewer `review-cases.json` (drives `review`), orchestrator `wedge-scenario.json` (drives the real buffer FIFO drain). In-crate `fixtures_pin.rs` per crate (PINs: worker ff628a59…, architect d26f71b3…, reviewer 13d713f9…, orchestrator ccc3aecd…). 4 `reference_spirits` slots added to `tests/coverage-matrix.yaml` (`ships_at: v0.8`, `coverage_pct: null`, `third_party: false`). `coverage-matrix` EXIT 0, `corpus-staleness` PASSED.

**T9 — Zero kernel KLOC / ABI / workspace count (AC8) — DONE.** No `maos-kernel-core` edits — `check-empty-kernel` 0, `check-service-boundary` 0; `kloc-check` shows `maos-kernel-core` **identical** at 15505 lines HEAD-vs-changes (proof of zero kernel growth). `abi-diff` Added-only/no breaking changes (no new FrameKind/FramePayload/SpiritRole — Architect/Reviewer use `SpiritRole::Worker`). Root `Cargo.toml` members 33→37; `4-kernel-design.md` sentinel 33→37; `check-workspace-count` PASSED (37/37).

**T10 — CI / discipline green (AC9) — DONE.** Added `orchestrator-tests`/`worker-tests`/`architect-tests`/`reviewer-tests` (`cargo test -p <name> --locked`) + `founder-loop-bench` (`--test`) + `smoke-founder-loop-8-4` step to `discipline.yml`; all 6 wired into `aggregate` `needs:` (YAML validated, no dangling needs, 100 jobs). All AC9 gates GREEN at HEAD (no flipped-while-red). Butler+Researcher+Observer regressions clean. `cargo fmt` applied to the 4 new crates only (no fmt/clippy CI gate exists; maos-bin formatting avoided per the whole-crate-collateral lesson).

**Pre-existing RED, verified wedge-neutral (flagged, NOT introduced):** `kloc-check` is RED at clean HEAD (EXIT 1) — 15 crates OVER budget incl. `maos-kernel-core` (15505/6000), `xtask` (13858/4000), `maos-bin` (4451/1000 at HEAD), + an NFR-Maint-1 total-ceiling breach (HEAD 71644). Wedge-neutral for the kernel: `maos-kernel-core` byte-identical (15505) HEAD-vs-changes. `maos-bin` was already OVER (4451→4759; smoke is NOT kernel KLOC, Decision G). The 4 new spirit crates do not appear in the OVER list. The `coverage-matrix` NFR-Meta-3 "unknown gate" lints are pre-existing and identical at HEAD (gate EXIT 0).

### File List

**NEW — `spirits/orchestrator/`** (AC2/AC3)
- `spirits/orchestrator/{Cargo.toml,manifest.toml}`
- `spirits/orchestrator/src/lib.rs`
- `spirits/orchestrator/tests/{spirit_smoke,orchestrator_buffer,distillate_dispatch,fixtures_pin}.rs`
- `spirits/orchestrator/tests/fixtures/wedge-scenario.json`

**NEW — `spirits/worker/`** (AC4)
- `spirits/worker/{Cargo.toml,manifest.toml}`
- `spirits/worker/src/lib.rs`
- `spirits/worker/src/bin/worker-cli-fixture.rs`
- `spirits/worker/tests/{spirit_smoke,cli_wrapper_admission,fixtures_pin}.rs`
- `spirits/worker/tests/fixtures/canned-cli-output.json`

**NEW — `spirits/architect/`** (AC5)
- `spirits/architect/{Cargo.toml,manifest.toml}`
- `spirits/architect/src/lib.rs`
- `spirits/architect/tests/{spirit_smoke,code_review_loop,fixtures_pin}.rs`
- `spirits/architect/tests/fixtures/design-specs.json`

**NEW — `spirits/reviewer/`** (AC5)
- `spirits/reviewer/{Cargo.toml,manifest.toml}`
- `spirits/reviewer/src/lib.rs`
- `spirits/reviewer/tests/{spirit_smoke,fixtures_pin}.rs`
- `spirits/reviewer/tests/fixtures/review-cases.json`

**MODIFIED**
- `Cargo.toml` — workspace members 33→37 (the 4 `spirits/*`)
- `Cargo.lock` — new crate entries
- `crates/maos-bin/Cargo.toml` — orchestrator/architect/reviewer/worker deps
- `crates/maos-bin/src/main.rs` — `smoke_founder_loop_8_4()` + 2 helpers + `MAOS_ONE_SHOT` dispatch arm + help text (AC6)
- `.github/workflows/discipline.yml` — 4 `*-tests` + `founder-loop-bench` + `smoke-founder-loop-8-4`, wired into `aggregate` needs (AC7/AC9)
- `tests/coverage-matrix.yaml` — 4 `reference_spirits` slots (AC9)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` — workspace-count sentinel 33→37 (AC8)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — story 8-4 status

### Change Log

| Date | Change |
|---|---|
| 2026-06-03 | Story 8.4 implemented: four founder-loop wedge reference Spirit crates (`spirits/{orchestrator,worker,architect,reviewer}`, workspace 33→37) over existing FR20/FR21/CliWrapper/I11/hot-swap substrate (zero kernel KLOC — `maos-kernel-core` byte-identical at 15505). FR20 safe-point buffer drain, FR21 4-scenario distillate-dispatch matrix (Decision K producer-side distillation), Worker CliWrapperSpirit real admission over an in-crate fixture-CLI, deterministic Architect→Reviewer FR21-clean loop, `smoke-founder-loop-8-4` headline (11pm→halt-resume→7am digest cites real log refs, exits 0), J1 budget MET (~9.5µs/call ≪ 25ms). Fixtures SHA-pinned + coverage-matrix registered; 6 CI jobs wired. All AC gates green at HEAD; kloc-check pre-existing RED flagged wedge-neutral. ~53 new tests, all passing. |

## Architect Rulings (Winston) — RESOLVED 2026-06-03

All six flagged decisions are **CONFIRMED**; Decisions B and F carry refinements the dev MUST follow, and a new **Decision K** (distillation authorship) is added. These rulings are authoritative over the `FLAG Winston` rationale notes above.

1. **Decision A — CONFIRMED.** Four reference Spirit crates under `spirits/` (orchestrator/worker/architect/reviewer); `check-workspace-count` 33→37. The §6.7 skill-package-overlay form (`orchestrator-bmad` / Developer-Worker / Reviewer-Worker loaded into live CLI agent processes via a `maos-bridge` crate) is the **v0.9+ production realization** — recorded as a carry-forward, NOT this story. The crate form is the canonical v0.8/v0.9 *reference deliverable* (mirrors Butler/Researcher/Observer).

2. **Decision B — CONFIRMED, with a sandbox refinement.** Worker is a `[cli_wrapper]` CliWrapperSpirit wrapping the in-crate fixture-CLI binary; the live multi-CLI stdio bridge stays deferred (non-Epic-8 kernel story; zero-kernel-KLOC intact). **Refinement:** the Worker manifest MUST declare `[sandbox] tier = "T3"` — Story 6.2 AC6 admission rejects a CliWrapperSpirit below T3 (`ECliWrapperRequiresT3`). The `--maos-bridge-probe` itself is a plain `std::process::Command` spawn (not sandboxed); the T3 declaration is the admission *gate*, satisfied by the manifest tier. Keep the fixture-CLI trivial and hermetic (no network, deterministic stdout) so the probe is bit-stable in CI.

3. **Decision C — CONFIRMED.** Architect and Reviewer carry `SpiritRole::Worker`. No `Architect`/`Reviewer` ABI variant (v1.0 freeze); `abi-diff` stays Added-only/`removed=[]`.

4. **Decision E — CONFIRMED, sub-question resolved.** Deterministic Architect/Reviewer; no live LLM in CI; live generation = application-layer v0.9+/Epic 9. The open sub-question is settled: **OMIT `[capabilities.required]`/`provider.complete`** for Architect and Reviewer — `manifest_self_check` proves `[capabilities.required]` is OPTIONAL (confirmed in 8.3 Decision G) and neither does inference at v0.8. The Orchestrator likewise omits `provider.complete` unless its `[epistemic_policy]` halt path requires it (verify against the validator; its dispatch logic is deterministic).

5. **Decision F — CONFIRMED, breach-semantics clarified.** Use the **existing** `maos-bench` J1 harness (`J1_P95_BUDGET_US=25_000`, `section_13_1`) — do NOT author a founder-loop-specific bench; §13.1 defines J1 as the synthetic Echo/CliWrapper floor and that IS the canonical measurement. The `founder-loop-bench` CI job runs in `--test` mode (regression guard, mirrors `researcher-bench`). **Clarification:** a 25ms-budget *breach at HEAD* is NOT a hard story/gate failure — record it in the Dev Agent Record as the §13.1 trigger for the E5 Story 5.5 rust-inproc evaluation (the IPC overhead is owned by the substrate code path, not by this story). The job DOES fail on a *regression vs. the established baseline*. Never migrate to inproc to mask overhead (§13.1: "fix our code first").

6. **Decision G/H/I — CONFIRMED.** Headline = runnable `maos-bin` `smoke-founder-loop-8-4` one-shot (mirrors `smoke-orchestrator-fanout-6-2`; not kernel KLOC). Digest reuses the existing FR17 path (citable `source_log_ref`s; no re-impl). Halt-and-resume uses `recall_all_pending()` + the hot-swap `state_codec` (no new persistence). **Scope note:** Observer (8.3) witnessing the loop via `task.assign`/`task.complete` subscription (§6.5 founder-loop use case) is explicitly OUT of scope — a nice-to-have carry-forward; do NOT add it.

7. **Decision K (NEW) — Distillation authorship is producer-side; Orchestrator references + I13 `admit_for_consumer`.** See the LOCKED Decision K above and the AC3 / FR21 dev-note refinements. The Orchestrator does NOT distill the Worker's output (it would `ScopeViolation` against the participant-scoped I11 chain); the **producer** distills its own output and the Orchestrator only **references** the resulting `Distillate`. Matches the 6.2 reference ordering.

### Review Findings

- [x] [Review][Patch] Spirit-authored frames use `FrameOrigin::HumanAuthored` instead of `FrameOrigin::SpiritAuto` [`crates/maos-bin/src/main.rs`, `spirits/orchestrator/tests/distillate_dispatch.rs`, `spirits/architect/tests/code_review_loop.rs`] — **fixed**: all 4 occurrences changed to `SpiritAuto`.
- [x] [Review][Defer] Worker `CliSubprocessOutput` event uses PID `0` [`crates/maos-bin/src/main.rs`] — deferred, pre-existing: follows the established `smoke-orchestrator-fanout-6-2` convention (PID `0` + `FrameOrigin::Kernel` for CliSubprocessOutput rows).
