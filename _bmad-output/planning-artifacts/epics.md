---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-02-party-mode-convergence", "step-03-create-stories", "step-03-party-mode-story-review", "step-03-retrofit-pass", "step-04-final-validation"]
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture-maos-minimal-opus.md
  - _bmad-output/planning-artifacts/maos-kernel-implementation-guide.md
phasingNote: |
  PRD uses phase labels v0.1 / v0.3 / v0.5 / v0.8 / v1.0 / v1.5 / v2.0 / v2.5.
  Canonical architecture (architecture-maos-minimal-opus.md) uses v0.1 / v0.3 / v0.5 / v0.7 / v0.9 / v1.0 / v1.5.
  PRD's "v0.8 founder-loop wedge demo" maps to architecture's "v0.9".
  Architecture is canonical for ADRs and architecture phases; PRD remains canonical for FR/NFR phasing labels.
  When epics group by phase, downstream consumers MUST reconcile FR-cited "v0.8" to architecture's "v0.9".
uxDocument: null  # Confirmed: no UX design document — kernel/infrastructure project.
---

# maos - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for **maos** (Multi-Agent Operating Substrate), decomposing requirements from the PRD and the canonical architecture (architecture-maos-minimal-opus.md), augmented by the kernel implementation guide, into implementable stories.

The substrate ships in phases v0.1 → v1.5 with deferred items at v2.0+ / v2.5. **Epic 1 carries a starter-template flag**: the architecture specifies a canonical Cargo workspace layout (architecture-maos-minimal-opus.md §4.0.2) that Story 1.1 must scaffold.

## Glossary

A dev agent picking up any story should be able to resolve every term below without leaving this document. Definitions cross-reference the PRD, architecture-maos-minimal-opus.md, and the first story where the term becomes load-bearing.

| Term | Expansion / One-line definition | First load-bearing in |
|---|---|---|
| **A2A** | Agent-to-Agent peer mesh — loopback at v0.8 (127.0.0.1 mTLS+TOFU); cross-Host at v1.0 (operator-managed PKI + ADR-012 typed-intent consent) | Story 6.3 |
| **ABI Stability Triple** | `(kernel_version, abi_version, manifest_schema_version)` — three independently-evolving versions governing kernel/Spirit compatibility; N-1 supported / N-2 hard refusal | Story 7.5 |
| **ACP** | Agent Control Protocol — NDJSON over stdio for editor-hosted Spirits (Zed + VSCode at v1.0; JetBrains plugin-bridge at v1.5) | Story 5.5c |
| **bmad-eval** | The standard halt-recall/halt-precision evaluation corpus. v0.3 = synthetic 50-scenario corpus in `crates/maos-eval/fixtures/halt-corpus-v0/`; v1.0 = E8-authored reference-Spirit corpora replace it | Story 4.1 |
| **BREAKING.md** | Grep-enforced file at repo root; every breaking change requires a dated entry with migration steps (NFR-Maint-7) | Story 7.5a |
| **CCAC** | ComplianceClaim Adversarial Corpus — v1.0 ship-gate corpus N=600 (200 well-formed + 400 malformed via parameterized generator); per-class floor ≥27/30; cross-Spirit agreement ±2% (NFR-Aud-9) | Story 7.3 |
| **ComplianceClaim envelope** | Ed25519-signed first-class object binding (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) to a compliance attestation; kernel verifies at admission with `EComplianceContextDrift` | Story 7.3 |
| **ComplianceClaim schema** | The wire-stable type definitions in `crates/maos-spirit-abi/src/compliance.rs`; FROZEN at Story 1b.4 after E0 adversarial review (Story 0.4); ABI-break required to change | Story 0.4 |
| **CRL** | Signed Revocation List artifact; registry-pushed (kernel polls every 5min) + offline-import path (FR13) | Story 5.4 |
| **DRR** | Deficit Round Robin fairness scheduler in front of the log writer; per-Spirit weight=1 default; operator-configurable `[scheduler.weights]`; max-min P99 latency ratio ≤3.0 under uneven load (NFR-Scale-3) | Story 6.1 |
| **Director** | The human end-user of MAOS — assigns tasks, resolves halts (3-pathway), shifts posture, instant pause/resume/revoke. Stakeholder for E3 stories | Story 3.1 |
| **Director's-surface metaphor** | Theater/actor/director framing — Spirits are actors, kernel is the stage, Director controls the production with bounded-time intervention (FR51) | Story 3.4 |
| **Evaluator** | First-time MAOS user; the 5-minute install-to-hello-Spirit-response stakeholder (NFR-Onb-2). Distinct from operator/director/Spirit author | Story 1b.5a |
| **HSIS** | Hot-Swap Invariant Suite — 6 Spirit-class-specific corpora × 50 scenarios = 300 scenarios; ≥95% pass per class; zero invariant violations CVSS-7 class (NFR-Rel-3) | Story 4.5 |
| **I1–I14** | The 14 architectural invariants codified in `crates/maos-domain/src/invariants/`. I1 = total capability mediation; I2 = log-before-deliver; I4 = Approval Decision Log distinct from Transparency Log; I5 = memory tier namespace isolation; I9 = empty-kernel; I10 = lifecycle journal durability; I11 = digest audit chain; I12 = working-memory digest refs; I13 = intent-lineage propagation; I14 = halt continuity across hot-swap | Story 1a.1 |
| **IAC bus** | Inter-Actor Communication bus — kernel-internal mailbox + broadcast + `retract` channels routing frames by SpiritId; log-before-deliver guarantee I2 | Story 3.1 |
| **J0 / J1 / J4 / J6** | Per-journey latency budgets from architecture §13.1. J0 = Butler conversational <400ms P95 / IPC <60ms; J1 = founder-loop CliWrapper IPC <25ms P95; J4 = Mira-Nash Observer colocation <10ms P95; J6 = Diego cold-start <500ms | Story 1b.5a |
| **KLOC budget** | Kernel trusted core ≤20 KLOC excluding tests through v2.0; `tokei` alarm at 16 (NFR-Maint-1); enforced per-merge via E0 | Story 0.1 |
| **LCAS** | Long-context Ambiguity Stress corpus — N=210 in 3 buckets (70 clearly-decidable v0.3 in Story 2.4 / 70 genuinely-ambiguous post-A2A in Story 7.4 / 70 adversarially-misleading post-A2A in Story 7.4); planted load-bearing claim contradicting louder repeated claim | Story 2.4 |
| **MCP** | Model Context Protocol — kernel-hosted tool-server interface with three transports (stdio / SSE / Streamable HTTP); Streamable HTTP default at v0.5 | Story 5.5c |
| **NFR-Onb-1** | 30-Min First Spirit Validation Gate — N=12 stratified authors, ≥10/12 succeed in 14 days zero-DM-support against Butler-class corpus; **v0.3 release criterion** despite gate-execution living in E7 | Story 7.5b |
| **NFR-Sec-7** | External pen-test report with zero P0/P1 findings at v1.0 ship; OWASP Risk Rating Methodology frozen at engagement start | Story 10.1 |
| **NFR-Aud-9** | CCAC ship gate at v1.0 — see CCAC entry | Story 7.3 |
| **NFR-Rel-3** | HSIS ship gate at v1.0 — see HSIS entry | Story 5.2 |
| **Operator** | Substrate deployment owner — configures providers, sandbox tiers, audit policy, GDPR forget cascades, distribution channels. Distinct from Director (end-user) and Spirit Author (developer) | Story 5.5a |
| **Principal namespace** | Typed namespace `principal:<principal_id>:<schema>` within Spirit's private memory tier (ADR-026); auto-eligible for subject-access, GDPR Art. 17 cascade, redaction-on-export | Story 4.3 |
| **§13.1 measurement** | Subprocess vs rust-inproc latency measurement story in E5; go/no-go gate before v0.5 ships — if subprocess meets J1+J4 budgets, rust-inproc form deferred to v2.0+; else rust-inproc unlocks with NFR-Test-7 cross-form equivalence at v1.5 | Story 5.5e |
| **scalar.tap** | Dedicated read-only stream from Capability Registry's tagged-scalar slot (ADR-035 binding-v0.5); every `working_memory.set_scalar(tag, value, derived_from)` write emits `(spirit_id, tag, value, timestamp)`; Observer Spirits subscribe to detect pre-halt drift | Story 4.2 |
| **Spirit** | LLM-backed agent hosted by the MAOS kernel; subprocess form at v0.1 (ADR-002); rust-inproc form gated on §13.1 | Story 1a.1 |
| **Spirit Author** | Developer writing Spirit code; primary stakeholder for E2 + E7 ecosystem stories | Story 2.1 |
| **Spirit class** | Typed category of Spirit (e.g., Butler / Researcher / Observer / Orchestrator / Worker / CliWrapper / Architect / Reviewer / Mira / Nash); halt-recall/precision floors measured per class | Story 4.1 |
| **Spirit form** | Spirit execution model — `subprocess` (v0.1; OS-process-isolated, LSP-style wire protocol over stdio) vs `rust-inproc` (gated on §13.1; same-process Rust dynamic library) | Story 1b.5a |
| **T0 / T1 / T2 / T3 / T4** | Sandbox tier ladder (ADR-004): T0 = no sandbox (trusted local); T1 = process isolation + UID separation; T2 = Linux Landlock+seccomp / macOS Seatbelt / Windows restricted-token; T3 = T2 + container (Docker/Podman, v0.5); T4 = WASM (v2.0 deferred). Strictest-of-(manifest, trust-tier, operator-policy) floor applies | Story 1b.3 |
| **three-door page** | `docs.maos.dev` landing page with three onboarding paths ("write a Spirit" / "run MAOS" / "understand MAOS"); NFR-Onb-3 | Story 7.5b |
| **TOFU pinning** | Trust-on-First-Use mTLS certificate pinning for A2A loopback at v0.8; second-connection pin-mismatch triggers 100% detect/block/alert (NFR-Sec-12); restart invalidates prior pins with re-pin consent confirmation (NFR-Rel-6) | Story 6.3 |
| **Transparency Log** | Per-Host SQLite append-only log of every IAC frame, approval, capability use, retract event; the personal audit trail. Distinct from `tracing` (debug) and the Telemetry Stream (typed broadcast events) | Story 1b.1 |

## Dependency DAG

Story-level dependency graph (cross-epic only — intra-epic ordering covered per-epic). Forward dependencies must be resolved by either (a) ordering the dependency before the dependent in sprint plan, or (b) a documented stub interface (`MockHaltResolver`-style pattern).

```
                    E0 Quality Substrate
                    ├──→ ALL EPICS (CI gates run on every PR)
                    └──→ Story 0.4 ComplianceClaim adversarial review BLOCKS Story 1b.4 schema freeze

E1a Workspace Bootstrap + Skeleton
├──→ E1b (workspace + ABI types must exist)
├──→ E2 (Spirit ABI types must exist)
└──→ Story 1a.1 → ALL: starter-template flag

E1b Evaluator Path + Audit Spine
├──→ E2 (manifest schema frozen)
├──→ E3 (IAC bus skeleton, Approval Decision Log, Transparency Log)
├──→ E4 (Memory Manager + Capability Registry runtime)
└──→ E5 (lifecycle hooks + sandbox infrastructure)

E2 Spirit ABI + Developer SDK
├──→ E3 (Spirit ABI lifecycle hooks)
├──→ E4 (halt-protocol Spirit-side declarations)
├──→ E7 (full SDK extends E2 seed)
└──→ Story 2.3 thin cargo-generate slice → Story 7.5b NFR-Onb-1 v0.3 gate execution

E3 Director's Surface — IAC Bus, Task Assignment, Posture
├──→ E4 Story 4.1 (halt-resolution UX surface — MockHaltResolver pattern allows unit isolation but integration gates on Story 3.3 shipping)
├──→ E6 (IAC bus skeleton)
└──→ Story 3.4 kernel log-composition primitives → Story 8.1 Butler morning-digest implementation

E4 Halt Protocol + Memory + Cognition (SINGLE HALT OWNER)
├──→ E5 Story 5.2 (halt-continuity-across-hot-swap I14 enforcement)
├──→ E5 Story 5.3 (halt-receipt production rate measurement)
├──→ E8 (cognitive primitives consumed by reference Spirits)
└──→ INTRA-E4 ORDERING: Story 4.5 (HSIS corpus 100 scenarios) MUST precede Story 4.1 AC4 (halt-recall/precision measurement)

E5 Lifecycle + Hot-Swap + Multi-Provider
├──→ E6 (lifecycle triggers + crash supervision required for A2A peers)
├──→ E7 (Spirit registry over MCP-Streamable-HTTP)
└──→ INTRA-E5 ORDERING: §13.1 measurement gate (Story 5.5e) MUST be last in E5 (go/no-go on rust-inproc)

E6 Multi-Spirit + A2A
├──→ E7 (CCAC cross-Spirit scenarios require A2A)
├──→ E8 (Orchestrator + Workers require IAC bus full features; Mira+Nash requires A2A cross-Host)
└──→ E9 (cross-Spirit memory isolation 200-corpus and GDPR cascade depend on multi-Spirit runtime)

E7 Spirit Ecosystem
├──→ E8 (reference Spirits published via registry)
├──→ E10 (CCAC corpus authored here, cross-validated in Story 10.1)
└──→ Story 7.5b (NFR-Onb-1 v0.3 gate execution) DEPENDS ON: Story 2.3 (thin cargo-generate from E2) + Story 8.1 (Butler reference Spirit from E8). Forward-resolved by slicing Story 2.3 forward to v0.3 sprint.

E8 Reference Spirits
├──→ E9 (audit queries validated against reference-Spirit production traces)
└──→ E10 (Butler/Researcher/Orchestrator+Workers/Mira+Nash gate the v1.0 + v1.5 ship)

E9 Audit + Compliance + Operator Productionization
└──→ E10 (multi-operator tenancy primitive-reservation declared here; full impl v1.5+ in Story 10.4)

E10 v1.0 Ship Gate + v1.5 Collective Tier
└──→ Coordination epic — consumes corpora authored in E4 (HSIS 100), E5 (HSIS 200), E7 (CCAC 600), E9 (red-team 80→640 generator), E0 (secret-redaction generator)
```

**Sprint-plan invariants (must hold for above DAG to be coherent):**

1. **v0.3 sprint:** Story 0.4 → Story 1a.1 → Story 1b.4 (schema freeze) → Story 2.3 (cargo-generate) → Story 3.3 (halt UX) → Story 3.4 (digest primitives) → Story 4.1 (halt mechanism) → Story 4.5 (HSIS 100 corpus) → Story 8.1 (Butler) → Story 7.5b (NFR-Onb-1 gate execution)
2. **v0.5 sprint:** Stories 5.1–5.4 → Story 5.5e (§13.1 go/no-go) → Stories 8.2, 8.3 (Researcher, Observer)
3. **v0.8/v0.9 sprint:** Story 5.2 (HSIS 200 corpus) → Stories 6.1–6.5 → Story 8.4 (Orchestrator + Workers)
4. **v1.0 sprint:** Stories 7.1–7.5a → Story 7.3 (CCAC 600) → Story 9.6 (red-team 80→640 generator) → Story 10.1 (HSIS verification + CCAC cross-validation + pen-test) → Story 10.2 (third-party trial + adversarial red-team execution) → Story 10.3 (export-control + manifest fuzz + wire fuzz + Korean docs)
5. **v1.5 sprint:** Story 10.4 (Postgres Loom-lite + Mira+Nash + SQLite→Postgres migration) → Story 10.5 (skill-format conformance + JetBrains + Windows + 2-year LTS + Japanese/CN-S i18n) → Story 8.5 (Mira+Nash safety-critical corpus 150 + κ≥0.7)

## Requirements Inventory

### Functional Requirements

**Total: 65 FRs across 7 capability areas. Numeric ship-gate floors are integral to FR text where the PRD includes them.**

#### A. Kernel Substrate Operations (9 FRs)

- **FR1:** User can install MAOS kernel via OS package manager (Homebrew/AUR/deb/rpm), `cargo install`, or signed GitHub Releases binary with mandatory Ed25519 signature verification.
- **FR2:** User can uninstall MAOS kernel cleanly, removing all installed Spirits, capability tokens, sandbox mounts, ACP sockets, and operator caches without leaving orphaned state.
- **FR3:** Operator can configure provider drivers (Anthropic, OpenAI, Gemini, Kimi, local-LLM via Ollama, air-gapped Bedrock) per Spirit, including locking provider endpoints for air-gapped deployment.
- **FR4:** Operator can verify every Spirit's external call (file op, network, exec, provider call, sub-Spirit spawn) was mediated by kernel-issued capability tokens by reading the Transparency Log; verification floor is 100% mediation in any 1000-call sample.
- **FR5:** Operator can configure sandbox tier per Spirit (T0/T1/T2/T3/T4); kernel enforces strictest-of-(manifest, trust-tier, operator-policy) floor. Spirit cannot exfiltrate data outside its declared capability scope — sandbox enforcement combined with FR4 capability mediation makes this property mechanically auditable.
- **FR6:** Operator can configure per-Spirit resource caps (CPU, memory, file descriptors) via cgroups v2 on Linux or platform equivalent.
- **FR7:** Operator can disable anonymous telemetry; default is opt-in with published schema and redaction layer.
- **FR47:** Spirit obtains all model inference exclusively via the kernel-provided Inference Port; the kernel routes to the configured provider driver and records the call in the Transparency Log. Spirit binaries do not import vendor LLM SDKs directly. (Closes ADR-005 coverage gap.)
- **FR48:** Operator can configure pluggable cryptographic provider for kernel signature verification, sealed-export encryption, and capability-token signing — enabling FIPS-validated, hardware-backed, or post-quantum implementations without recompiling Spirits. (FIPS / NIAP / export-control readiness.)

#### B. Spirit Lifecycle Management (8 FRs)

- **FR8:** Spirit author can declare a Spirit class via manifest (TOML) covering `class`, `capabilities`, `posture`, `output_shape`, `explanation_shape`, `epistemic_policy`, `budget`, `skills`, `hot_swap`, `halt_protocol_compatibility`, `intent_promotion_set`, `migrates_from`, `swap_invariants`, `schedule`, `min_substrate_version` (kernel rejects load if its own version is below the declared minimum). Manifest declarations are signed and journaled.
- **FR9:** User can load, start, pause, resume, and unload Spirits at runtime via authenticated control plane (CLI, ACP editor surface, or operator API).
- **FR10:** Operator can hot-swap a Spirit class to a new version preserving in-flight capability tokens and working-memory state per the kernel-enforced migration decision tree (ADR-020). Both Spirit forms (in-process Rust ABI and subprocess) ship with parity on lifecycle and IAC semantics (ADR-002).
- **FR11:** Spirit author can declare cross-major migration via `migrates_from` manifest field and a `migrate(predecessor_state)` entry point; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared.
- **FR12:** Kernel detects Spirit-process crash within 2s and emits `task.orphaned` IAC frames to in-flight task originators within 5s with exit-cause journaled. Floor: ≥99/100 detected within 2s in a SIGKILL crash corpus; ≥99/100 NACKed within 5s. Hung-Spirit detection (alive but no progress IAC for >30s) emits `task.stalled` event; ≥48/50 reclassified within 60s on a hang corpus.
- **FR13:** User or operator can revoke a Spirit at runtime via signed Revocation List artifact; running Spirit instances receive `SpiritRevoked` event and execute their declared revocation policy (terminate-immediately / drain-then-terminate / quarantine).
- **FR49:** Operator can upgrade a Spirit (replace v0.3.1 with v0.3.2) with declared migration policy: hot-swap with state preservation (default), cold-swap with re-init, or migrator-mediated cross-major upgrade. Distinct from FR9 (lifecycle verbs); FR49 covers state-bearing version transitions.
- **FR50:** Spirit author can declare dead-Spirit task disposition policy in manifest (`on_crash.action`); kernel applies the policy to in-flight tasks held by the dead Spirit (NACK / reassign-to-replica / escalate-to-operator). Operational-failure handling distinct from epistemic halt (FR15).

#### C. Human–Spirit Interaction — Director's Surface (8 FRs)

- **FR14:** User can assign a task to a Spirit via natural-language `task.assign` IAC frame (terminal shell, ACP editor surface, mobile push) with goal + scope + success criteria + posture preferences.
- **FR15:** User can resolve a Spirit-emitted `epistemic.halt` via three documented resolution pathways — supplying missing context, accepting the halt as final, or authorizing override under operator policy; kernel journals the resolution with full reasoning chain. Halt-recall floor ≥0.7 and halt-precision floor ≥0.85 per Spirit class on the `bmad-eval` standard corpus.
- **FR16:** User can shift Spirit posture at runtime ("be more cautious for the next hour"; "switch to autonomous-with-halt"); the shift is journaled and applied to subsequent capability-scope decisions. Posture-shift propagation latency: P99 ≤2s, P99.9 ≤5s in a 1000-shift corpus.
- **FR17:** User can read a per-Spirit morning digest containing: (a) tasks completed in the last 24h with outcome tags, (b) open halts requiring resolution, (c) flagged anomalies with confidence ≥0.6, (d) trust-bar reflecting yesterday's predicate-fire rate. Digest is generated by a digest-shipping Spirit (Butler at v0.3 / Researcher at v0.5 / Orchestrator at v0.8+ — NOT kernel) within 30s of the user's first session of the day, using kernel-provided log-composition primitives and the §9.5 distillation pattern. Hallucination floor: 0 hallucinated tasks tolerated in any 100-digest corpus, verified against the actual Transparency Log; ≥95/100 digests must include all open halts and cite source log refs for all claimed completions.
- **FR18:** User can audit any Spirit decision retrospectively; every `decision.*` frame carries `working_memory_digest_refs` (I12) so post-hoc audit can reconstruct what the agent reasoned over at decision time.
- **FR19:** User can configure halt-recall vs halt-precision preference per Spirit per tag via a halt-policy schema (extension to ADR-013); kernel parses the preference into the Spirit's runtime epistemic policy thresholds.
- **FR20:** User can buffer multiple instructions to an Orchestrator Spirit (NOT kernel-buffered — Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives); the Orchestrator processes queued instructions at safe sequence points between task completions, never preempting in-flight delegations. (Phase: v0.8, advanced from v0.5 — required for the founder-loop wedge demo's halt-and-resume-overnight pattern.)
- **FR51:** Director can instantaneously pause, resume, or shift posture of any Spirit including: (a) interrupting in-flight autonomous actions with bounded-time guarantee (P99 ≤2s), (b) preserving Spirit state across pause/resume without reload, (c) recalling pending Orchestrator-buffered actions per FR20, (d) revoking any active capability token with in-flight operations failing-safe within bounded time. Override is auditable per FR42 with director identity and reason. Operationalizes the director's autonomy-spectrum control surface that defends the theater/actor/director metaphor.

#### D. Multi-Spirit Coordination (11 FRs)

- **FR21:** Orchestrator Spirit can dispatch `task.assign` IAC frames to Worker Spirits with named skill (e.g., `bmad-dev-story`), scoped capability set, posture preferences, and halt policy. Orchestrator dispatches subsequent tasks against the distillate of prior Worker output, not the raw output (closes raw-output context-overflow loophole). Sustained fan-out floor (per NFR-Perf-8): 50 concurrent Worker Spirits with task-dispatch latency P99 ≤500ms; 0 dropped tasks under 10 tasks/sec for 1h.
- **FR22:** Spirits on the same Host can communicate via the kernel-internal IAC bus with mailbox-per-Spirit routing and log-before-deliver guarantee (I2). Orchestrator-Worker communication uses distillate frames in steady state; raw frames recallable via `log.recall` for ground-truth verification.
- **FR23a:** (v0.8 loopback) Spirits across Hosts can communicate via A2A peer mesh on `127.0.0.1`-bound endpoints with self-signed mTLS certs and TOFU pinning. Test corpus: mTLS replay 100/0; TOFU pin-mismatch 100/100 detected; handshake-fault 20/0; cross-Spirit consent 30 scenarios with 100% disallowed blocked.
- **FR23b:** (v1.0 full mesh) FR23a extends to cross-host with operator-managed PKI, full mTLS handshake corpus, certificate rotation chaos test (10-host Cortex, zero conversation drops), revocation latency median ≤60s p99 ≤5min, clock-skew tolerance ±5min, partial-partition fail-safe within 10s.
- **FR24:** Spirit can run autonomously under `autonomous-with-halt` posture, halting only when its `[epistemic_policy]` triggers; user can shift to `assistive` (every action prompts) or `cautious` (auto-approve routine, prompt for novel) at runtime. All cross-Spirit IAC frames carry intent provenance metadata linking each intent to its originating task envelope, preserved across re-emission (per ADR-018 / I13).
- **FR25:** Worker Spirit can be a wrapped agent CLI process (Claude Code, opencode, gemini-cli, kimi-cli) with `maos-bridge` + persona skills loaded; kernel-builtin CliWrapperSpirit hosts it with declared `output_shape_version` (fail-loud on shape mismatch).
- **FR26:** Spirit can declare scheduled invocations via manifest `[schedule]` table; kernel fires `on_schedule(ctx, schedule_id, payload)` at declared cadence with rate-limit, ComplianceClaim-stamp, principal-revocability, and side-effect allowlist (per ADR-025).
- **FR52:** Spirit can invoke external CLI subprocess (e.g., `claude code`, `opencode`) under capability-token authority; stdout/stderr captured into the Transparency Log with provenance to the invoking Spirit. Tier-3 sandbox profile; explicit manifest declaration required. (v0.8 wedge-critical — operationalizes Worker Spirit's CLI-shelling pattern.)
- **FR53:** Active halts associated with a Spirit retain identity, replay context, and resumption guarantees across hot-swap (per ADR-019 / I14); the kernel rejects swaps that would orphan halts unless the Spirit-author has declared `halt_protocol_compatibility = true` for the predecessor's halt schema. Closes I14 coverage gap.
- **FR54:** Spirit author can declare gateway sub-modules in manifest (e.g., Telegram, Slack, Discord, Signal, email) running as long-lived connection holders under the Spirit's principal namespace (FR31); kernel hosts the lifecycle and capability-scope contracts; gateway implementation is Spirit-side. Defends the v1.0 hermes-tenant positioning claim.
- **FR55:** Spirits SHALL be able to register for kernel-emitted lifecycle triggers including `on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate` (Spirit-author-defined cadence for memory-curation passes). Each trigger carries declared resource budgets per manifest. Butler's `on_idle` substrate for anticipatory reasoning is the v0.3 anchor.

#### E. Memory, Cognition Substrate, and Distillation (7 FRs)

- **FR27:** Spirit can write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics — kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Per §4.0.7: kernel performs no Spirit-specific cognitive computation (no variance, entropy, EFE, KL, ensemble disagreement, derivatives, or statistical tests — Spirit computes those itself).
- **FR28:** Spirit can write to private, shared, or collective memory tiers (per I5); memory compaction is Spirit-authored — the Spirit's persona logic declares compaction policy; kernel provides persistence and quota enforcement only.
- **FR29:** Spirit can recall historical Transparency Log frames it was a participant in via `log.recall(filter, limit, cursor)` with payload-on-demand fetch via `log.fetch(frame_id)`; kernel scopes results to participant frames and honors A2A consent envelopes. Distillation work — selecting which frames to preserve, summarizing, abstracting — is Spirit-authored.
- **FR30:** Spirit can produce distillates (digests) via Spirit-side LLM compression; kernel enforces I11 audit-chain on digest writes (mandatory `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage`).
- **FR31:** Spirit can write principal-related data to the `principal:<principal_id>:<spirit-author-defined-schema>` namespace (per ADR-026; principal-namespace pattern informed by hermes-agent's principal-scoped memory model lifted into a kernel-allocated contract); data inherits subject-access query, right-to-be-forgotten, and redaction-on-export operations. The kernel allocates the namespace and enforces isolation; the kernel does not index or interpret content.
- **FR32:** Spirit author can declare per-tag epistemic policies referencing tagged scalars and the four universal-arithmetic predicates; kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from). Cognitive work — choosing the threshold, designing the predicate semantics, computing the underlying scalars — is Spirit-authored. Predicate-firing recall floor ≥0.85 per Spirit class; precision floor ≥0.85.
- **FR56:** Spirit can read its own performance telemetry (success/failure counts, latency distributions, halt-recall events, distillation outcomes) scoped to its principal namespace per FR31, without requiring per-read operator admission. Self-telemetry feeds Spirit-side calibration and skill-revision proposals (FR57). Spirit's own data; Spirit reads it.

#### F. Spirit Ecosystem and Distribution (12 FRs)

- **FR33:** Spirit author can scaffold a new Spirit via `cargo generate maos-spirit` (Rust v0.1+) or per-language template (TypeScript v0.5+, Python v1.0+, Go v1.5+).
- **FR34:** Spirit author can test a Spirit via `spirit-test` SDK harness without spinning up a kernel; harness covers lifecycle hooks, IAC frame I/O, halt resolution, manifest self-check, and class-specific regression corpus. Coverage floor: 80% of Spirit author's manifest-declared capabilities reachable via SDK fixtures, validated by external-author trial in 5 third-party Spirits.
- **FR35:** Spirit author can publish a Spirit package via `maos-spirit publish --tier=<tier>` with Ed25519 signing; package conforms to `maos.spirit.v1` schema.
- **FR36:** User can install third-party Spirits via authenticated control plane with mandatory signature verification, trust-tier floor enforcement, and ComplianceClaim envelope verification at admission.
- **FR37 (DEFERRED to v2.5):** Vetter (third-party authority) can issue a vetting attestation promoting a Spirit from `public-untrusted` to `public-vetted` tier; attestation is Ed25519-signed and journaled with revocation semantics. Phase deferred from v1.0 → v2.5 ecosystem-adoption phase per John's first-cut-if-slip rule (vetter ecosystem requires public-Spirit marketplace which v1.0 team-readiness doesn't need).
- **FR38:** Third-party assessor can issue a ComplianceClaim envelope binding (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) to a compliance attestation; kernel verifies at admission and refuses to load Spirits whose runtime context drifts.
- **FR39:** Spirit author can author skills (markdown with TOML frontmatter conforming to `maos.skill.v1`) and either ship them in the Spirit's package or write them dynamically at runtime via the `skill.author.self` capability scope; new skills land in pending state pending operator admission.
- **FR40:** Spirit author can publish a CLI-wrapper Spirit configuration declaring `output_shape_version`; kernel-builtin CliWrapperSpirit refuses to start if observed CLI shape doesn't match declared version (fail-loud, never silent).
- **FR57:** Spirit can query its own performance telemetry within its principal namespace (FR31) and emit skill-revision proposals carrying (a) the target skill id and version, (b) the proposed diff, (c) the telemetry evidence supporting the proposal. Such proposals enter the operator-admission queue (FR39) and are subject to the same vetting and audit obligations. Operationalizes the "actors learn from each performance" claim that defends the hermes-tenant positioning.
- **FR58:** User can complete the zero-config path from `install` to first Spirit response within the J0 evaluator budget. At v0.1: response is `hello-spirit` acknowledgement with structured introduction (capability scope, posture, expected halt-tags, link to local Transparency Log), demonstrating the kernel's ABI surface. At v0.3+: response is from a working reference Spirit (Butler at v0.3; Researcher at v0.5; Architect/Worker/Reviewer at v0.8+). At least one bundled or auto-fetchable reference Spirit is suitable for evaluation at the named phase. Onboarding-validation gate (NFR-Onb-2 v0.1; NFR-Onb-1 v0.3+).
- **FR59:** Spirit registry supports publisher- and vetter-initiated yank events that propagate to operators on next sync (≤5min poll cadence default), distinguishable in audit from operator-local revocation (FR13), with documented operator response semantics (warn / quarantine / auto-revoke per operator policy).
- **FR60:** Substrate supports import of signed Spirit and skill artifacts (with vetter attestations and ComplianceClaims) from offline media or mirrored registries, preserving the full verification chain (FR36). Air-gapped deployments operational.

#### G. Audit, Compliance, and Operator Surfaces (11 FRs)

- **FR41:** Operator can run frame-by-frame log queries via authenticated audit interface with filters by Spirit, capability, time-range, frame-kind, and tag. Query latency floor: P99 ≤2s for queries scoped to a single Spirit on a 30-day window; P99 ≤10s for global queries; completeness floor on a per-commit log-completeness corpus with N=100 injected events: ≥98/100 events recoverable from logs (per NFR-Aud-1). Query language is specified separately (audit-query-surface ADR — extension to ADR-013).
- **FR42:** DPO can run subject-access queries via `maosctl audit subject-access --principal <id>`; returns all principal-namespace entries across all Spirits with provenance (Spirit, time, derived-from observations).
- **FR43:** CISO can run posture-delta queries via `maosctl audit posture-delta --range=<timespan>` surfacing capability-scope changes, sandbox-tier changes, and consent-policy changes over a configurable time-range with approval-chain attribution.
- **FR44:** External regulator can request a sealed-export via `maosctl audit sealed-export <bundle-spec>`; bundle is Ed25519-signed by the operator's audit key, third-party-verifiable, conforms to `maos.audit-bundle.v1` schema.
- **FR45:** User can exercise GDPR Article 17 right-to-be-forgotten via `maosctl forget --principal <id> [--reason <legal-hold>]`; kernel removes all principal-namespace entries; the deletion event itself is journaled (preserving lifecycle invariant) but the principal data is gone. Cross-Spirit cascade: forgetting cascades to working-memory references in other Spirits where principal data was shared; distillates containing principal data are marked redacted with re-distillation triggered. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries.
- **FR46:** Operator can export filtered raw trajectories via `journal.export(filter, redaction_policy)` per ADR-023; bundle conforms to versioned `maos.trajectory.v1` schema with Ed25519 signing and applied-redaction flag.
- **FR61:** Substrate project publishes and maintains `SECURITY.md` documenting (a) disclosure contact (`security@maos.dev` with published GPG key), (b) coordinated-disclosure window and CVE-assignment process, (c) supported-versions matrix for security backports, (d) advisory-publication channel. v0.1 binding — not deferred; security disclosure pipeline must exist before any Spirit is shipped to a third party.
- **FR62:** Substrate exposes audit-queryable artifacts for governance: (a) vetter-key admission and rotation events, (b) ABI-extension proposals and their ratification status, (c) ComplianceClaim schema versions and their effective dates. Operationalizes Constitutional Substrate Evolution (Innovation #7 from Step 6).
- **FR63:** All kernel-emitted errors carry stable typed codes from a published catalog at `https://docs.maos.dev/errors/<ERR_NAME>` with documented retryability, cause-chain semantics, and version-stability guarantees consistent with the LTS policy. CI-enforced metadata per error variant; v1.0 binding (catalog initial set covers the 14+ named errors documented in architecture-maos.md).
- **FR64:** Operator can attribute cost (token-spend per provider, subprocess CPU-time, storage I/O) per Spirit per task per principal in the Transparency Log. Enterprise-readiness gate — no enterprise deployment without per-tenant cost accounting.
- **FR65:** Operator can uninstall a Spirit; kernel emits a proof-of-erasure record enumerating all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations). Defends the v1.0 hermes-tenant positioning claim that substrate-uninstall is a real guarantee, not a hope.

### NonFunctional Requirements

**Total: ~85 NFRs across 13 categories.**

#### Performance (8 NFRs)

- **NFR-Perf-1:** IAC frame routing latency P50 < 5ms, P99 < 50ms on a typical Linux box (NVMe + 16-core tier). v0.5.
- **NFR-Perf-2:** Sustained IAC frame throughput 5,000–10,000 frames/sec single-host before log writer becomes bottleneck. Per-Spirit fairness scheduler in front of log writer (NOT FIFO). v0.5.
- **NFR-Perf-3:** Capability-token validation latency P99 < 100µs per check; 100% re-validation at use against current state, not cached state (TOCTOU correctness). v0.1.
- **NFR-Perf-4:** Posture-shift propagation P99 ≤ 2s, P99.9 ≤ 5s in 1000-shift corpus. v0.3.
- **NFR-Perf-5:** Audit query latency P99 ≤ 2s for single-Spirit queries on 30-day window; P99 ≤ 10s for global queries. v0.5 (basic), v1.0 (signed-export tier).
- **NFR-Perf-6:** Distillation step latency budget declared per Spirit class via manifest `[budget].time_cap`; soft warning at 80%; kernel emits `BudgetWarning` IAC frame. v0.5.
- **NFR-Perf-7:** Hot-swap latency P99 < 500ms (mode switch + state transfer + capability rebinding) for same-major same-additive swaps. v0.8.
- **NFR-Perf-8:** Orchestrator fan-out — sustained 50 concurrent Worker Spirits with task-dispatch latency P99 ≤500ms; 0 dropped tasks under 10 tasks/sec sustained for 1 hour. Backs FR21's fan-out floor. v0.8.

#### Reliability (11 NFRs)

- **NFR-Rel-1:** Spirit-process crash detection ≤ 2s; `task.orphaned` IAC frame ≤ 5s. Floor: ≥99/100 detected within 2s on SIGKILL crash corpus. v0.8.
- **NFR-Rel-2:** Hung-Spirit detection (no-progress IAC for >30s) → `task.stalled` event within 60s. Floor: ≥48/50 reclassified within 60s on hang corpus. v0.8.
- **NFR-Rel-3:** HSIS (Hot-Swap Invariant Suite) ≥ 95% pass per Spirit class; zero invariant violations (CVSS-7 class). 6 class-specific corpora at 50 scenarios each; stratified swap-lifecycle phase distribution. v1.0.
- **NFR-Rel-4:** Silent-failure detection. Kernel emits `silent_failure_suspect` event when Spirit emits no progress IAC frames for >30s despite healthy heartbeats. Floor: ≥45/50 detected on adversarial zombie-heartbeat corpus. v1.0.
- **NFR-Rel-5:** Hot-swap rollback within 30s if successor health-check fails. Kernel auto-reverts to predecessor and emits `HotSwapAborted` IAC frame. v1.0.
- **NFR-Rel-6:** Spirit-restart invalidates prior A2A TOFU pins; re-pin protocol with consent confirmation. v1.0.
- **NFR-Rel-7:** A2A trust establishment under churn — 100-host Cortex (or compressed 30-host scale per Murat's cost-compression), 10–20% host turnover/week for 4 weeks, 3 planted adversarial hosts. Floor: detection latency ≤ 1h median, blast radius ≤ 5 peers, recovery ≤ 24h. v2.0 (compressed) / v2.5 (full 100-host).
- **NFR-Rel-8:** Lifecycle journal durability — fsync per state transition; ring-buffer flush latency < 1ms. v0.1.
- **NFR-Rel-9:** Revocation propagation latency ≤ 5s p99 under 10⁴ concurrent capability-token validations. Closes Winston's "A2A trust establishment under churn" production-risk gap. v0.8.
- **NFR-Rel-10:** Kernel cold-restart ≤ 30s with no data loss on graceful shutdown; ≤ 1 in-flight message loss on hard kill. v0.8.
- **NFR-Rel-11:** Halt-receipt production rate ≥ 99.9%. Every Spirit termination, planned or unplanned, produces a halt receipt before process exit. Closes I14 directly (separate from HSIS aggregate). v0.8.

#### Security (16 NFRs)

- **NFR-Sec-1:** Sandbox tier enforced per Spirit; strictest-of-(manifest, trust-tier, operator-policy) floor. v0.1 (T0/T1/T2); v0.5 (T3); v2.0 (T4 WASM).
- **NFR-Sec-2:** Capability-token TTL ≤ 60s for high-privilege operations; bound to Spirit-PID + boot-nonce; audit-logged at every use with origin-Spirit-ID. v1.5 (ADR-023). [Note: architecture marks ADR-023 as binding-v0.1.]
- **NFR-Sec-3:** Sandbox-escape **structural** anomaly detection (syscall pattern divergence from manifest declaration, fd-table growth, unexpected outbound IAC connections). The kernel raises a structural alarm; the *interpretation* of whether the alarm constitutes malice is Spirit-side or operator-side. The kernel does not classify intent. v2.0 (ADR-024).
- **NFR-Sec-4:** Pre-write secret-redaction filter at Transparency Log boundary. Floor: 0 secrets across the bounded test populations — 10⁴-case corpus per-commit (0/10⁴), 10⁵-case quarterly audit (0/10⁵), and production canary system (1000 unique synthetic secrets/month with cryptographic markers; 0 leak per month). Discovery latency ≤ 24h p95. Any false negative is P0 ship-blocker. v0.5.
- **NFR-Sec-5:** Manifest parser fuzz: 24h `cargo-fuzz`, zero crashes/OOMs/infinite loops. v1.0 ship gate.
- **NFR-Sec-6:** Wire protocol adversarial-input fuzz: 24h, zero crashes. v1.0.
- **NFR-Sec-7:** External pen-test report with zero P0/P1 findings open at v1.0 ship. Triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to PRD-author tiebreak. P0/P1 definitions per OWASP Risk Rating Methodology, frozen at engagement start.
- **NFR-Sec-8:** Negative-capability assertion via manifest `forbidden_capabilities`; kernel enforces never holding tokens for forbidden surfaces, even transitively via A2A. v1.0.
- **NFR-Sec-9:** Zero `unsafe` blocks in kernel capability-validation path (Rust). v0.1 ship gate.
- **NFR-Sec-10:** Adversarial-Spirit red-team 80-scenario corpus across 8 attack classes (capability confusion, IAC frame injection, distillation poisoning, ledger tampering, cross-Spirit privilege escalation, resource exhaustion, side-channel timing, kernel-syscall abuse), N=10 per class. Floor: ≥9/10 per class detected/blocked by kernel; ≥72/80 aggregate; 0 unmitigated category. Authored by external pen-tester (not MAOS team) using published ABI; pre-frozen corpus, content-addressed. v1.5.
- **NFR-Sec-11:** mTLS handshake replay-attack test: 1000 captured handshakes replayed, 0 succeed. v0.5 (loopback) / v1.0 (cross-host).
- **NFR-Sec-12:** TOFU pin-mismatch on second connection: 100% detected, blocked, alerted. v0.5.
- **NFR-Sec-13:** mTLS cert rotation chaos test: 3-host at v1.5; 10-host at v2.0; rotation under load with zero conversation drops; revocation latency median ≤ 60s, p99 ≤ 5min.
- **NFR-Sec-14:** Cross-Spirit memory isolation corpus — 200-scenario adversarial corpus where Spirit-A actively attempts to enumerate, read, side-channel, or timing-attack Spirit-B's substrate state. Categories: namespace enumeration, working-memory read-across, decision-frame observation, halt-signal observation, transparency-log cross-read, working-memory-digest cross-read, capability-token forgery cross-Spirit, sandbox-escape lateral. Floor: 200/200 isolation maintained; any leak = P0 ship-block. Defends the v1.0 hermes-tenant positioning sentence. v0.8.
- **NFR-Sec-15:** Crypto-module pluggability with FIPS 140-3-validated default option. Kernel-internal cryptographic operations (signature verification, sealed-export encryption, capability-token signing) route through a provider trait permitting substitution of FIPS-validated, hardware-backed, or post-quantum implementations without recompilation of Spirits. v1.0.
- **NFR-Sec-16:** Manifest-evolution lint forcing binary `secret`/`non-secret` annotation on every new manifest field — no default. Mitigates structural-vs-semantic redaction tension by shifting cost from runtime detection (forbidden by §4.0.7) to authoring time. v0.5.

#### Auditability & Compliance (14 NFRs)

- **NFR-Aud-1:** Capability-contract introspection via `maosctl capability inspect <spirit>`. Returns machine-readable list of declared capabilities, observed capabilities used in last 30d, capability-token issuance count per type. Log-completeness corpus with N=100 injected events; floor ≥98/100 events recoverable from logs. v1.0.
- **NFR-Aud-2:** Drift detection — kernel compares Spirit's set-membership and frequency-distribution (capabilities used, tags written, halts emitted) against manifest declarations. Set-membership and frequency-distribution comparison only — no semantic interpretation. Per §4.0.7, the kernel does not classify whether observed behavior is "suspicious" or "malicious"; it surfaces structural divergence and the operator (or Spirit-side cognition) interprets. v1.0.
- **NFR-Aud-3:** Deterministic replay anchored by ADR-028. Replay determinism is over the shape of the trace (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders. v1.0 best-effort; v1.5 hard target.
- **NFR-Aud-4:** Audit retention ≥ 90 days private tier (default); configurable per-deployment; Merkle-root anchoring optional for tamper-evidence. v0.5.
- **NFR-Aud-5:** Right-to-explanation via I12 — 100% of `decision.*` frames carry `working_memory_digest_refs` for explainability replay. EU AI Act adjacent compliance. v0.8.
- **NFR-Aud-6:** Sealed-export Ed25519-signed by operator audit key; third-party-verifiable; conforms to `maos.audit-bundle.v1` schema. Bundle includes both working-memory digest refs (I12) AND distilled-output content (I11). v1.0.
- **NFR-Aud-7:** Five-metric distillation gate per distillation-shipping Spirit: digest-recall ≥ 0.90; digest-faithfulness ≥ 0.98 unflagged contradictions; digest-hedge-preservation ≥ 0.95; digest-traceability = 100% (kernel-enforced via I11); digest-secret-leakage = 0% (zero-tolerance).
- **NFR-Aud-8:** Two-tier corpus: N=100 calibration per-commit (CI width 0.124, fine for trend detection) + N=500 quarterly audit (CI width ≤0.05 at p=0.90 for digest-recall; tight statistical confidence). Plus 10⁵-case secret-leakage corpus + production canary system per NFR-Sec-4. v0.5 (per-commit), v1.0 (quarterly).
- **NFR-Aud-9:** ComplianceClaim Adversarial Corpus (CCAC) v1.0 — N=600 (200 well-formed + 400 malformed). Per-class N=30, floor ≥ 27/30. 100 context-drift claims (100/100 rejected). Cross-validation across ≥3 reference Spirits, agreement within ±2%. v1.0 ship gate.
- **NFR-Aud-10:** GDPR Article 17 right-to-be-forgotten — 50-scenario corpus with cross-Spirit cascade. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries. v1.0.
- **NFR-Aud-11:** SIEM export at v2.0. OpenTelemetry adapter at v1.0.
- **NFR-Aud-12:** Storage cascade erasure completeness + externally-verifiable uninstall receipt. Substrate-uninstall produces a portable, externally-verifiable erasure receipt (signed Merkle inclusion + signed Merkle exclusion proof, retained independent of the substrate). 100% of registered storage backends prove erasure within bounded window for any given principal. Closes the weakest leg of the hermes-tenant positioning sentence. v1.0.
- **NFR-Aud-13:** Time-to-erasure SLA. Floor: 95% of right-to-be-forgotten requests complete within 30 days (configurable to 7 for enterprise tier); audit log entry within 24h of request acceptance. v1.0.
- **NFR-Aud-14:** Intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain back to originating principal intent. Closes ADR-018/I13 NFR coverage gap. v0.8.

#### Testability (14 NFRs)

- **NFR-Test-1:** All ship-gate test corpora are static artifacts content-addressed in the repo (SHA-256 of JSONL); generation provenance is documented but not required to be reproducible. Pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash committed alongside, retry budget=1, quarterly re-baseline with ≥98% agreement on golden snapshot. v1.0.
- **NFR-Test-2:** Kernel-API surface invariant test (per-commit gate). Build-time reflection enumerates every kernel API exported to Spirits via `kernel::api::*`; classifies each function by computational class (universal-arithmetic / data-movement / supervision / **other**); floor: 0 functions in class "other"; new function entering class "other" is build-break. Static analyzer on Rust `syn` walking allowlist-based predicate definitions; decidable for permitted subset (no theorem prover). Kernel-utility crate (`kernel::util::*`) has separate looser invariant: no I/O except via injected trait, no global state. v0.1 build gate (surface-diff only); v0.5 adds static analyzer for predicates.
- **NFR-Test-3:** spirit-test SDK harness coverage ≥ 80% of Spirit author's manifest-declared capabilities reachable via fixtures; validated by external-author trial in 5+ third-party Spirits. v1.0.
- **NFR-Test-4:** Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class on `bmad-eval` standard corpus. v0.5.
- **NFR-Test-5:** FKCS (Frozen-Kernel Conformance Suite). FKCS-infrastructure (diff oracle, test harness, kernel-frozen-vN.0 commit-tagging) at v2.0; FKCS-populated (3 future Spirits implemented by external authors) at v2.5. Floor: ≥27/30 per Spirit, ≥85/90 aggregate; diff oracle confirms zero kernel changes; negative-control "fourth Spirit" deliberately uses undocumented kernel internal and MUST fail.
- **NFR-Test-6:** LCAS (Long-context Ambiguity Stress) corpus — N=210 scenarios in 3 buckets (clearly-decidable n=70 / genuinely-ambiguous n=70 / adversarially-misleading n=70). Adversarial trajectories contain a planted load-bearing claim contradicting a louder repeated claim. v0.5 ship gate.
- **NFR-Test-7:** Cross-form Semantic equivalence (rust-inproc ↔ subprocess) ≥ 90%; (any-rust ↔ wasm-component) ≥ 75%. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs). v1.5 (rust↔subprocess; cohort interop at v1.0 is rust-rust); v2.0 (any-rust↔wasm).
- **NFR-Test-8:** Black-box third-party trial v1.0 — N=12 stratified (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). 14-day no-DM-support window. Floor: ≥10/12 produce working signed Spirit binary that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85. Wilson CI [0.552, 0.962] meaningful at N=12; meaningless at N=5. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot. Run only at major releases (v1.0, v2.0); minor releases use NFR-Onb-1 (12-author onboarding) as proxy.
- **NFR-Test-9:** Loom-not-in-kernel structural test. `grep` of kernel crate for orchestration/planning symbols returns ∅. Per-commit gate. Covers ADR-006's negative commitment (Loom is user-space). v0.5.
- **NFR-Test-10:** Skill-format conformance — at least one third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification. Covers ADR-027's external-standard interop assertion empirically. v1.5.
- **NFR-Test-11:** Namespace grammar lock test. Grammar `.lark` (or equivalent) hash pinned in CI; any change requires architecture-lock review process, not regular PR. v0.5.
- **NFR-Test-12:** v0.3 architecture lock script as per-commit gate. `scripts/check_v0_3_lock.sh` runs four mechanical checks: (1) `LICENSE` matches ADR-decided license string; (2) consortium-target ADR exists with status `accepted` and ≥2 maintainer sign-offs; (3) `ROADMAP.md` has trust-anchor decision section with status `decided` linking to ADR; (4) failure-semantics doc exists with at least one fully-specified route. No v0.3 tag without script in green. v0.3.
- **NFR-Test-13:** Manifest field test coverage ≥ 3 cases per field (well-formed, malformed-rejected, edge-case); CI-enforced. v0.1.
- **NFR-Test-14:** Wire protocol cross-language byte-equal golden corpus per frame variant per SDK (Rust + TS v0.5 + Python v1.0 + Go v1.5+). v1.0.

#### Meta-Testing (3 NFRs)

- **NFR-Meta-1:** Corpus-quality audit. Each ship-gate corpus reviewed by independent assessor (not corpus author) on a 10-point rubric (representativeness, edge-case coverage, label correctness, distribution match to production). Floor: ≥8/10 per corpus. Cadence: at corpus creation + every 12 months. v1.0.
- **NFR-Meta-2:** Corpus-staleness. Every corpus carries a `valid_until` date in metadata. CI fails if any active gate references an expired corpus. Default validity: 12 months. Extension requires explicit "no-update justification" PR with assessor sign-off. v1.0.
- **NFR-Meta-3:** Coverage matrix. Single source-of-truth file `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}. CI fails if any FR/NFR with phase-status `delivered ≤ current-phase` has zero corpus coverage. Floor at v1.0: 100% coverage of FRs/NFRs delivered ≤ v1.0. v1.0.

#### Observability (5 NFRs)

- **NFR-Obs-1:** Author-observability contract — Spirit author can read same diagnostic surface as operator for their own Spirit, redacted of cross-Spirit data. Metric M is queryable in <500ms with cardinality ≤10⁴. v1.0.
- **NFR-Obs-2:** OpenTelemetry export per IAC frame, capability invocation, halt event. v0.5 basic; v1.0 SLO-class.
- **NFR-Obs-3:** Per-Spirit telemetry stream with topic-based broadcast + filtered subscription. v0.3 (Butler narrow); v0.5 (Observer broad).
- **NFR-Obs-4:** Transparency Log per-Host SQLite (append-only), exportable to JSONL/SIEM with redaction policy applied. v0.5.
- **NFR-Obs-5:** Approval Decision Log distinct from Transparency Log; full intent + decision + reasoning chain per Invariant I4. v0.3.

#### Documentation Quality (7 NFRs)

- **NFR-Doc-1:** Every public ABI method has ≥ 1 doctested example; CI broken-link blocking on doc site at v0.5; doctest CI gate at v0.1.
- **NFR-Doc-2:** Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` covering all 14+ named typed errors. CI-enforced metadata: each variant has 6 fields (code, severity, recovery-class, owner, kernel-or-spirit, since-version). v1.0.
- **NFR-Doc-3:** API reference site at `https://docs.maos.dev/abi/<version>/`; versioned, searchable, deep-linkable, archived ≥ 2 minor versions back. v1.0.
- **NFR-Doc-4:** Five canonical doc deliverables published with CI-verifiable minima — Manifest schema reference (≥1 example per field); Pattern cookbook (≥10 patterns); Migration runbooks; Troubleshooting guide (covers 100% of FR63 catalog); Deployment topology guide. v1.0.
- **NFR-Doc-5:** WCAG AA compliance for doc site. v1.0.
- **NFR-Doc-6:** Localization v1.0 = Korean only (shipped); Japanese + Chinese-simplified at v1.5. `LOCALES.md` with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- **NFR-Doc-7:** Doc tooling supports per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown. RTL layout deferred to v2.5. mdBook + i18n / Docusaurus / VitePress decision by v0.5; v1.0 in production.

#### Onboarding (4 NFRs)

- **NFR-Onb-1:** 30-Min First Spirit Validation Gate. N=12 stratified external Spirit authors (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). Floor: median ≤ 45 min, p95 ≤ 90 min, AND ≥ 10/12 succeed where "succeed" = author produces Spirit binary that (a) compiles against published ABI, (b) passes the v0.3-grade Butler-class regression corpus (30-scenario calendar/comms; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall), (c) does so within 14 calendar days from kit handoff with zero direct-message support. v0.3 release criterion.
- **NFR-Onb-2:** First-time installer J0 evaluator path — install + first useful Spirit response within 5 minutes. v0.1.
- **NFR-Onb-3:** Three-door page at `docs.maos.dev` ("write a Spirit" / "run MAOS" / "understand MAOS"). v0.5.
- **NFR-Onb-4:** 30-Min Gate iteration cadence. If floor missed, run fresh 6-author cohort within 2 weeks; three consecutive misses escalate to v0.3 release-criterion review. v0.3.

#### Maintainability (9 NFRs)

- **NFR-Maint-1:** Kernel trusted core ≤ 20 KLOC excluding tests through v2.0 (core scheduler + IAC bus + capability check + journal). Integration adapters in separate crates with their own LOC budgets. v2.0.
- **NFR-Maint-2:** Capability-registry fuzz coverage ≥60% line at v0.1; ≥80% line / ≥60% branch at v0.5 on 1M-iteration libFuzzer run; zero crashes.
- **NFR-Maint-3:** ABI compatibility matrix 100% within current major; 100% N-1 boundary including negative typed-error cases. v0.1 (within-major); v1.0 (N-1).
- **NFR-Maint-4:** STABILITY.md publishes live (kernel_version, abi_version, manifest_schema_version) compatibility matrix. v1.0.
- **NFR-Maint-5:** Deprecation timeline: 2 minor releases of warning, 1 major release to remove. v1.0.
- **NFR-Maint-6:** 1-year LTS commitment at v1.0; 2-year LTS commitment at v1.5 once support load is known; security-only patches after year 1.
- **NFR-Maint-7:** BREAKING.md required entry for every breaking change with migration steps; CI grep-enforced. v1.0.
- **NFR-Maint-8:** Capability-token TOCTOU test: 100% re-validation at use against current state. v1.0.
- **NFR-Maint-9:** Manifest schema N-1 compatibility — kernel version V can load manifests written for V-1 with documented degradation paths. Closes ADR-025 NFR-coverage gap. v1.0.

#### Scalability (5 NFRs)

- **NFR-Scale-1:** Cortex 3-region pilot at v2.0 with ≥ 10 agents minimum; sustained operation for 30 days; zero substrate-invariant violations.
- **NFR-Scale-2:** 25-host churn test at v2.0; 100-host churn at v2.5 (cost compression: 100→30 hosts at v2.0 same churn-events-per-week, full 100-host moves to v2.5).
- **NFR-Scale-3:** Per-Spirit fairness scheduler in front of log writer (NOT FIFO). Algorithm: Deficit Round Robin (DRR) with per-Spirit weight=1 by default; operator-configurable weights via `[scheduler.weights]`. Floor: under uneven load (1 noisy Spirit at 10× the median write rate alongside ≥4 normal Spirits sustained for 60s), max-min P99 latency ratio across Spirits ≤ 3.0. v0.5.
- **NFR-Scale-4:** Provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame. v0.5.
- **NFR-Scale-5:** Multi-host A2A peer mesh scales to 14-institution Cortex; v2.0 target with documented capacity envelope.

#### Operational (12 NFRs)

- **NFR-Ops-1:** Substrate operations checklist fully delivered: install, upgrade, yank, uninstall, revoke. v0.1 (install/uninstall) → v0.5 (upgrade/yank) → v1.0 (revoke).
- **NFR-Ops-2:** Signed Revocation List (CRL) artifact; registry-pushed (kernel polls every 5min) + offline-import path. v1.0.
- **NFR-Ops-3:** Telemetry opt-in default; `PRIVACY.md` with retention, jurisdiction, deletion path; per-field redaction layer. v1.0.
- **NFR-Ops-4:** `SECURITY.md` with disclosure address (`security@maos.dev`), GPG key, embargo window (90-day default), advisory-publication channel, supported-versions matrix. v0.1 ship gate. CNA registration through MITRE moves to v0.5.
- **NFR-Ops-5:** maosctl `--plain` flag + `NO_COLOR` + `TERM=dumb` accessibility. v0.1.
- **NFR-Ops-6:** Onboarding artifacts — `RFC_TEMPLATE.md` at v0.8, `GOVERNANCE.md` at v0.5 (basic) + v0.8 (locked), `CODE_OF_CONDUCT.md` at v0.5, `LOCALES.md` at v1.0, `TRADEMARK.md` at v1.0, `BREAKING.md` at v1.0.
- **NFR-Ops-7:** Sustainability vehicle — declared-intent at v0.5 (Open Collective open, accepting $0 expected); legal/fiscal-sponsor work at v0.8.
- **NFR-Ops-8:** Trust-anchor framing carry-forward decision. Published ADR by v0.3 declaring which competitive framing is committed (substrate-as-substrate vs substrate-as-trust-anchor); absence = v0.3 release-block. v0.3.
- **NFR-Ops-9:** Transparency Log backup/DR. RPO ≤ 1h, RTO ≤ 4h, backup integrity verified weekly via Merkle-root cross-check. v1.0.
- **NFR-Ops-10:** Database migration test corpus. SQLite→Postgres at v1.5. Floor: forward-migration test on 10⁶-row corpus, byte-identical Merkle-root preservation post-migration, rollback path tested. v1.4 (gates v1.5).
- **NFR-Ops-11:** Multi-operator tenancy isolation — primitive-reservation only at v1.0 (declared as primitive-reserved in namespace grammar so v0.5 grammar lock doesn't paint us into a corner; full implementation v1.5+). Per-operator namespace, per-operator transparency-log shard, per-operator capability-token signing key, per-operator GDPR-erasure scope. v1.0 (reserved); v1.5+ (implemented).
- **NFR-Ops-12:** Air-gapped deployment validation. Substrate boots, runs, produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation; documented Spirit-author guidance for air-gapped capability tokens. v1.0.

#### Compliance & Regulatory (5 NFRs)

- **NFR-Comp-1:** Export-control classification artifact. ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel. v0.8 (before any v1.0 enterprise-distribution conversation).
- **NFR-Comp-2:** Vetter accreditation parameters — published vetter qualification matrix, conflict-of-interest disclosure required, vetter rotation policy (no single vetter on >40% of Spirit-class promotions in any 12-month window), vetter audit-trail retained 7 years. v1.0.
- **NFR-Comp-3:** Substrate-self compliance scope declaration. `STABILITY.md` contains scope-disclaimer paragraph explicitly stating SOC 2 / ISO 27001 / FedRAMP scope is the *operator's* responsibility, with kernel-as-service boundary drawn. v0.5.
- **NFR-Comp-4:** Region-pinning primitive (PIPL §40 / data localization). Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication. v1.0.
- **NFR-Comp-5:** Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent). Manifest declares covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission. v1.0.

#### Cost & Tenancy (2 NFRs)

- **NFR-Cost-1:** Cost-attribution accuracy ≥ 98% reconciliation against provider billing, sampled monthly. Per-Spirit per-task per-principal attribution. Without this NFR, FR64 (cost accounting) is theater. v1.0.
- **NFR-Tenancy-1:** Explicit single-tenant per kernel instance commitment through v2.0; multi-tenant primitive-reserved at v1.0 per NFR-Ops-11; full multi-tenant out of scope before v2.5. v0.1 (declared); v2.0 (single-tenant guaranteed).

### Additional Requirements

#### Starter template / project skeleton **[FLAG: drives Epic 1 Story 1]**

- **[ARCH]** Kernel language is **Rust + Tokio** (ADR-001, binding-v0.1). Alternative-language proposals require ADR + benchmark.
- **[ARCH/KERNEL]** **Cargo workspace** is the canonical project structure. Architecture's canonical layout (`architecture-maos-minimal-opus.md` §4.0.2):

  ```
  maos/
  ├── crates/
  │   ├── maos-domain/                # v0.1 — Pure types, invariants I1-I14, pure functions (zero deps)
  │   ├── maos-spirit-abi/            # v0.1 — Wire-stable types ONLY. #![no_std]. src/compliance.rs (ComplianceClaim)
  │   ├── maos-kernel-core/           # v0.1 — Five services + two internal modules
  │   │   ├── scheduler/              #         Spirit Scheduler + journal + budget
  │   │   ├── memory/                 #         Memory Manager + namespace enforcement
  │   │   ├── security/               #         Security Manager + sandbox + approval
  │   │   ├── io/                     #         I/O module — internal at v0.1
  │   │   ├── iac/                    #         IAC Bus (mailbox, broadcast, retract)
  │   │   ├── capability/             #         Capability Registry decomposed:
  │   │   │   ├── cap-tokens/         #           Hot path: token issue/verify, lock-free
  │   │   │   ├── cap-policy/         #           Consent rules + intent allowlist
  │   │   │   ├── cap-audit/          #           Audit/lineage writer (slow path)
  │   │   │   └── cap-quota/          #           Budget tracking + ContextPressure
  │   │   ├── compliance/             #         ComplianceClaim structural validator (~200 LOC, v0.1)
  │   │   ├── pipeline/               #         Emit pipeline (IACFrame + ComplianceClaim co-located)
  │   │   ├── telemetry/              #         Telemetry module + scalar.tap — internal at v0.1
  │   │   └── hot_swap/               #         Hot-Swap Coordinator
  │   ├── maos-spirit-sdk/            # v0.1 — Spirit-author helpers; #[spirit] proc-macro
  │   ├── maos-spirit-hello/          # v0.1 — Reference Spirit; validates SDK end-to-end
  │   ├── maos-providers/             # v0.1 — Anthropic at v0.1; ≥3 providers in CI by v0.5
  │   ├── maos-mcp/                   # v0.5
  │   ├── maos-acp/                   # v0.5
  │   ├── maos-a2a/                   # v0.9 — Bilateral A2A peer (loopback at v0.9, cross-Host at v1.0)
  │   ├── maos-persistence/           # v0.1 — SQLite at v0.1; Postgres+pgvector (Loom-lite) at v1.5
  │   ├── maos-secrets/               # v0.1 — OS keyring adapter
  │   ├── maos-compliance/            # v0.9 🔒 — Semantic evaluator + N=600 corpus (App-E)
  │   ├── maos-control/               # v0.5 — Control-plane HTTP API
  │   ├── maos-cli/                   # v0.1 — maosctl
  │   └── maos-bin/                   # v0.1 — Composition root
  ├── spirits/
  ├── schemas/
  │   ├── trace-shape.schema.json
  │   ├── halt-registry/<spirit-class>.toml
  │   └── gateway-submodule.schema.json
  ├── docs/
  ├── fuzz/                           # Fuzz harnesses (manifest, wire, replay)
  └── wit/spirit.wit                  # WIT contract (v2.0)
  ```

- **[KERNEL]** Kernel implementation guide proposes a 15-crate (+6 reference Spirit) layout that uses different naming (`maos-spirit-runtime`, `maos-sandbox`, `maos-control-plane`). **Architecture-minimal-opus is canonical**; Story 1.1 uses the architecture's layout, not the kernel-guide's earlier naming.
- **[ARCH]** **Single multi-threaded Tokio runtime**, worker count = number of CPU cores. Composition root (`maos-bin/main.rs`) uses `#[tokio::main(flavor = "multi_thread")]`.
- **[ARCH]** **Hexagonal architecture** for static structure + **actor model** on runtime hot path (ADR-010 + ADR-011, both binding-v0.1). Crate boundary lint enforces port/adapter ring. Domain core compiles without async runtime.
- **[KERNEL]** Initial dependency budget — `maos-domain`: `serde + thiserror`. `maos-spirit-abi`: + `async-trait + serde_json`. `maos-kernel-core`: `tokio (full) + tokio-stream + async-trait + serde_json + tracing + arc-swap + dashmap + parking_lot + uuid`.
- **[ARCH]** **CI gates from day one (binding-v0.1):** `cargo xtask check-service-boundary` (P1–P4 four-property test for each supervised service); `tokei` for KLOC budget enforcement (`xtask/kloc.toml`, aggregate ≤20 KLOC, alarm at 16); ABI-diff lint; `invariant-lock` CI gate (ADR-037) on every PR touching I1–I14.
- **[ARCH]** **Reproducible build at v0.1:** `cargo build --locked` on Rust stable; no nightly; **zero `unsafe` in `maos-kernel` core / capability-validation path** (NFR-Sec-9, AC-V01-5).

#### Infrastructure / deployment requirements

- **[ARCH]** **Two deployment topologies** ship at v1.5 (configuration alone, no architectural rewrite): (1) Single-user single-Host (laptop/workstation, 5–10 Spirits cooperatively scheduled, optional Loom-lite Postgres for founder loop); (2) Diagnostic-architect bilateral 2-Host pair (Host A prod-edge running Mira; Host B dev-environment running Nash; pre-paired with mTLS cert fingerprints; mobile push to operator).
- **[ARCH]** **Persistence layered:** SQLite per-Host (Transparency Log + Approval Decision Log + Journal) at v0.1; Postgres+pgvector for Loom-lite (collective tier) at v1.5 only. Migration test (SQLite→Postgres) gates v1.5 (NFR-Ops-10).
- **[ARCH]** **Distribution:** v0.1 source build (`cargo install --path crates/maos-bin`) Linux + macOS only. v0.5 pre-built binaries (Linux amd64/arm64, macOS arm64) via GitHub Releases with SHA256 + Ed25519 verification mandatory. v1.0 Homebrew tap, AUR, deb, rpm; container images on Docker Hub / GHCR. Windows binary at v1.5. v2.0 official Linux distro packages + one-line install script.
- **[ARCH]** **Air-gapped deployment** validation in CI via network-namespace isolation (NFR-Ops-12, v1.0). Offline-import path for signed artifacts (FR60).
- **[ARCH]** **Logical clock discipline:** Lamport or hybrid logical clock (final pick by v0.5) for cross-Host frame ordering; wall-clock is metadata only. Certificate validity windows remain wall-clock.
- **[ARCH]** **Backup/DR:** Loom-lite RPO ≤1h, RTO ≤4h, backup integrity verified weekly via Merkle-root cross-check (NFR-Ops-9, v1.0).

#### Integration requirements

- **[ARCH]** **Four-protocol commitment:** kernel-internal IAC + bilateral A2A + ACP + MCP. Substrate invents no new wire protocols.
- **[ARCH]** **Same-Host IAC:** `tokio::sync::mpsc` + `tokio::sync::broadcast` channels addressable by SpiritId; bounded queues; backpressure via Spirit Scheduler.
- **[ARCH]** **Subprocess Spirit Wire Protocol (ADR-032, binding-v0.1):** LSP-style `Content-Length: <decimal>\r\n\r\n` framing followed by N bytes of CBOR-encoded payload. Header ASCII case-insensitive, max header block 4 KiB. `BufReader` cap = 1 MiB; oversize = `WireError::Oversize`. Writer over bounded `mpsc<Frame>(64)`. Stderr piped to `tracing` at WARN; never multiplexed onto stdout. Clean EOF after frame = `Halt::Voluntary`; mid-frame EOF = `Halt::Fault(Truncated)`. SIGTERM → 5-second grace → SIGKILL.
- **[ARCH]** **Cross-Host (bilateral A2A):** mTLS over TCP, JSON-RPC framing. Each Host's deployment configuration names the other Host's mTLS cert fingerprint (no discovery). Per-frame ADR-012 typed-intent consent. Logical-clock ordering. Network-partition: in-flight frames NACKed after configurable timeout (default 30s); kernel does NOT auto-retry.
- **[ARCH]** **MCP client (ADR-008):** all-three transports (stdio / SSE / Streamable HTTP). Streamable HTTP is default. T4 WASM tool sandboxing for untrusted MCP at v1.0+; Spirits via WASM at v2.0.
- **[ARCH]** **ACP server:** NDJSON over stdio for editor-hosted Spirits. v1.0 with Zed + VSCode tested. JetBrains via plugin-bridge at v1.5.
- **[ARCH]** **Spirit registry (ADR-008, binding-v0.5):** itself an MCP-Streamable-HTTP server. `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`. **Three trust tiers** at v1.0: `local`, `org-internal`, `public-untrusted`. (`public-vetted` adds at v2.5 via FR37 — deferred. PRD's four-tier mention reconciled to architecture's three at v1.0.)
- **[ARCH]** **LLM provider drivers (ADR-005):** v0.1 = Anthropic. v0.5 = ≥3 providers in CI (Anthropic + OpenAI + local-LLM via Ollama). v1.5 = MAOS-mediated provider proxies. v2.0 = full multi-provider including Bedrock/Vertex AI/local LLMs.
- **[ARCH]** **Skill ecosystem:** filesystem-discovered at v0.5 (conventional locations `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`); optional skill registry at v2.0; `maos.skill.v1` format intentionally close to Anthropic Skills format.

#### Data persistence / migration / setup requirements

- **[ARCH]** **Three memory tiers (Memory Manager, I5-enforced):** `private` (per-Spirit); `shared` (Host-wide, SQLite-backed kv with namespace prefix per writer); `collective` (Loom-lite Postgres+pgvector via MCP-Streamable-HTTP, v1.5).
- **[ARCH]** **Principal Memory Namespace (ADR-026, binding-v0.5):** typed namespace `principal:<principal_id>:<schema>` within private tier. Inherits subject-access query, right-to-be-forgotten, redaction-on-export.
- **[ARCH]** **`memory.md` convention:** Spirits MAY persist a `memory.md` file in their private namespace (universal cohort convention). Kernel does not interpret.
- **[ARCH]** **Hot-swap state-transfer wire format (ADR-017, binding-v0.3):** CBOR-encoded payloads conforming to per-Spirit-class schema declared in manifest (`[hot_swap].state_schema_uri` + `state_schema_version`). Compatibility rules: same-major + additive forward = forward-compat; same-major + breaking = forbidden; cross-major requires explicit migrator. Saga-style compensation; auto-revert within 30s on post-swap invariant violation.
- **[ARCH]** **Cross-major migration (ADR-020):** `migrate(predecessor_state) -> Result<successor_state, Error>` declared via `migrates_from`; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator declared.
- **[ARCH]** **Lifecycle journal (I10):** append-only on-disk log of all lifecycle transitions; fsync per state transition; ring-buffer flush latency < 1ms (NFR-Rel-8). Crash recovery rehydrates from journal.
- **[ARCH]** **Replay determinism (ADR-028):** over the **shape of the trace** (frame ordering, capability-token issuances, halt events, decision-frame emission), NOT redacted payload content. `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12) validated in CI. v1.0 best-effort, v1.5 hard target.

#### Observability requirements

- **[ARCH]** **Three logs coexist; do not conflate:** `tracing` (internal spans/debug); Telemetry Stream (typed broadcast events); Transparency Log (every IAC frame, approval, capability use, retract; SQLite append-only; durable; the personal audit trail).
- **[ARCH]** **Approval Decision Log (I4):** separate from Transparency Log. Captures `(actor, target, capability, intent, decision, reasoning_if_any)` for every approval prompt resolution.
- **[ARCH]** **Telemetry Stream IAC round-trip metrics (binding from v0.1):** `iac_rt_duration_us` (histogram, microseconds; labels: `service ∈ {security, memory, iac, capability, spirit_scheduler}`, `outcome ∈ {ok, err, timeout}`); `iac_rt_inflight` (gauge); `iac_rt_errors_total` (counter). Histogram buckets anchored on 1500µs SLO: `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]`.
- **[ARCH]** **`scalar.tap` channel (ADR-035, binding-v0.5):** dedicated read-only stream from Capability Registry's tagged-scalar slot. Every `working_memory.set_scalar(tag, value, derived_from)` write emits `(spirit_id, tag, value, timestamp)`. Observer Spirits subscribe to see pre-halt scalar drift.
- **[ARCH]** **OpenTelemetry export adapter:** v0.5 basic; v1.0 SLO-class. SIEM export at v2.0.
- **[ARCH]** **Spirit-form measurement gate (§13.1):** `benches/iac_roundtrip.rs` using `criterion`. Three workloads (J1 floor, J-Butler, J-Researcher). Per-journey latency budgets (subprocess v0.1): J0 Butler conversational < 400ms P95 / IPC < 60ms; J1 Founder loop CliWrapper IPC < 25ms P95; J4 Mira-Nash Observer colocation < 10ms P95; J6 Diego cold-start < 500ms.

#### API versioning / compatibility

- **[ARCH]** **ABI Stability Triple:** `(kernel_version, abi_version, manifest_schema_version)`. `abi_version` governs Spirit/KernelHandle vtable + capability ID space. `manifest_schema_version` governs TOML surface independently. `kernel_version` is product-facing.
- **[ARCH]** **N-1 supported, N-2 hard refusal** with typed `EAbiTooOld`. Deprecation timeline: 2 minor releases of warning, 1 major to remove. Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`.
- **[ARCH]** **STABILITY.md:** carries live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- **[ARCH]** **`min_substrate_version` manifest field:** kernel rejects load if its own version is below the declared minimum.
- **[ARCH]** **ComplianceClaim schema (ADR, binding-v0.1):** schema is frozen, structural validator implemented, emit pipeline live on every Spirit decision. Schema validation 100%, emit-rate 100%. Adding any required field, removing any field, renaming, type-changing, or removing/reordering enum variants of `Verdict` / `PrincipleRef` / `EvidenceKind` bumps `ABI_VERSION`. Adding optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`, additive enum variants with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback — does NOT bump.

#### Security / sandbox / capability implementation requirements

- **[ARCH]** **Sandbox tiers (ADR-004, binding-v0.1):** T0 (no sandbox; trusted local-tier) → T1 (process isolation; UID separation) → T2 (Linux: Landlock+seccomp; macOS: Seatbelt with `.sbpl`; Windows: restricted-token, default for `public-untrusted`) → T3 (T2 + container, Docker/Podman) → T4 (WASM, v2.0 for tools at v1.0).
- **[ARCH]** **Strictest-of-(manifest, trust-tier, operator-policy) floor.** Public-untrusted Spirit declaring T0 is forced to T2.
- **[ARCH]** **Per-Spirit resource isolation (cgroups v2 / setrlimit / Job Object):** kernel sets at spawn, OS-enforced not Tokio-cooperation. Defaults declared in manifest `[resources]`; kernel applies strictest-of (manifest, operator policy).
- **[ARCH]** **Capability Registry decomposition (ADR-030, binding-v0.1):** `cap-tokens` (sharded `Arc<[CapShard; 64]>` lock-free, hot path <5µs P99); `cap-policy` (read-mostly, copy-on-write); `cap-audit` (bounded `tokio::sync::mpsc::channel(8192)` to single audit-writer task; slow path); `cap-quota` (per-Spirit atomic counters; emits `ContextPressure` at 80%, `ContextLimit` at 95%, `EContextExhausted` above 100%).
- **[ARCH]** **Capability-token TTL ≤60s for high-privilege ops (ADR-023, binding-v0.1).** Tokens bound to (Spirit-PID + boot-nonce + expiry); ed25519-signed; non-transferable. TOCTOU re-validation at every use against current state.
- **[ARCH]** **Pre-write secret-redaction filter at Transparency Log boundary** (universal to all logged frames). Floors per NFR-Sec-4 (10⁴ per-commit, 10⁵ quarterly, 1000-canary/month).
- **[ARCH]** **Approval class taxonomy (6 classes):** `readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Default policies operator-overridable per Spirit.
- **[ARCH]** **Pluggable crypto provider (`CryptoProvider` trait):** default `ring`/`rustls`. Alternates for FIPS 140-3, hardware-backed, post-quantum, on-prem HSMs. v1.0 architectural commitment (NFR-Sec-15).
- **[ARCH]** **ComplianceClaim envelope (binding-v1.0 first-class object):** Ed25519-signed, references execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity). Kernel verifies at admission with typed `EComplianceContextDrift` on drift.

#### Kernel subsystem requirements

- **[ARCH]** **Five services + two internal modules + one contract.** P1–P4 four-property test (§4.0.8) classifies: one supervisor (Spirit Scheduler), four supervised services (Security Manager, Memory Manager, IAC Bus, Capability Registry), two internal modules (I/O Subsystem, Telemetry Stream), one contract (Spirit ABI). Internal modules eligible for service extraction at v0.5+.
- **[ARCH]** **Spirit Scheduler:** lifecycle (load/start/pause/swap/migrate/snapshot/restore/unload); journal (I10); resource budgets per Spirit. Cooperative + priority-weighted. OS-level CPU/memory budget enforcement via cgroups v2 (Linux) / setrlimit (macOS) / Job Objects (Windows).
- **[ARCH]** **Memory Manager:** three tiers + namespace enforcement (I5); hot-swap and migration; principal namespace; kernel does not interpret memory contents.
- **[ARCH]** **Security Manager:** sandbox tiers; secret materialization (just-in-time pass-through to OS keyring; Vault/cloud-KMS at v2.0); approval mediation; posture enforcement; Token Lifecycle Manager.
- **[ARCH]** **IAC Bus:** same-Host frame routing (mailbox), cross-Host bilateral A2A, `retract` primitive, notification surface dispatch. Logged-before-deliver guarantee (I2). Partial-consent failure semantics (`ConsentRupture` event, ADR-034 binding-v0.9).
- **[ARCH]** **Capability Registry:** mediate every external call (I1); issue/verify/revoke capability tokens; enforce manifest-declared capability surfaces; validate I11/I12/I13 audit-chain fields on digest writes; track per-Spirit budget (ADR-016).
- **[ARCH]** **Telemetry Stream:** topic-based broadcast + filtered subscription; `scalar.tap` channel; OpenTelemetry export adapter; author-observability contract.
- **[ARCH]** **Halt detection three-layer composition (§4.6.1):** (1) Spirit-self-invocation primary (`epistemic.halt(payload)` from `[epistemic_policy]` rules + four predicates); (2) Budget-based stall detection secondary (kernel emits `task.stalled` after `timeout_no_progress` default 30s); (3) Scalar trajectory tap tertiary (Observer Spirits subscribe to `scalar.tap`).
- **[ARCH]** **Halt resolution kinds:** `provided_context`, `accepted_halt`, `authorized_override`. `authorized_override` adds mandatory `override_marker` to subsequent output for `output_shape` predicates.
- **[ARCH]** **Hot-Swap Coordinator (I6, I14 enforcement):** validates state-transfer schema compatibility before successor activation; checks `halt_set` before swap; rejects swap with `EHaltContinuityViolation` otherwise.
- **[ARCH]** **Cancellation discipline:** every long-lived task takes a `CancellationToken` (from `tokio-util`). Composition root cancels root token on Host shutdown; tasks unwind cleanly via `select!` with cancellation arm.
- **[ARCH]** **mTLS rotation chaos test (§7.2.1):** quarterly forced rotation under live load. Pre-staged-overlap with `T_grace = max(2 × p99_handshake_rtt, 5s)`. Three timing gates: revocation propagation (`t_1 - t_0`) ≤30s p50 / ≤90s p99; re-handshake (`t_2 - t_1`) ≤30s p50 / ≤60s p99; end-to-end rotation (`t_2 - t_0`) ≤60s p50 / ≤150s p99. `cert_post_grace_reject` rate ≤0.1%.
- **[ARCH]** **Wire-protocol fuzz tiered cadence (§5.2):** T1 per-commit (10 min, N=4 workers); T2 nightly (4h, N=8); T3 pre-release (24h, N=8). Per-target floor ≥72 CPU-hours per fuzz target across 90 days pre-GA; aggregate floor ≥1,000 CPU-hours pre-GA.
- **[KERNEL]** **Kernel performance budgets:** `iac/send` <10µs P99; capability token issuance (cached posture, no prompt) <5µs P99; capability invocation dispatch (excluding adapter) <5µs P99; `memory/read` (cached) <50µs P99 / (uncached SQLite) <5ms P99; Transparency Log append (batched flush) <1ms P99 to durability; Spirit cold-load (rust-inproc) <10ms / (subprocess) <100ms; Hot-swap (rust-inproc) <50ms P99 / (subprocess) <500ms P99; Telemetry broadcast (one event, 10 subscribers) <1µs.
- **[KERNEL]** **Tokio task topology:** ~5 long-lived coordination tasks (Scheduler, IAC, Journal, control-plane HTTP, persistence) + one Spirit actor task per Spirit + 0–N transient outbound tasks. Capability Registry, Memory Manager, Security Manager (mostly), Telemetry broadcaster are NOT tasks — service-as-functions over `Arc<DashMap>` or pool.
- **[KERNEL]** **Error handling convention:** every fallible kernel function returns `Result<T, KernelError>`. `panic!` reserved for irrecoverable invariant violations. `unwrap()`/`expect()` forbidden in production paths. Errors carry context.

#### Innovation primitives (load-bearing, novelty-claim-grade)

- **[ARCH]** Empty-kernel invariant (I9, ADR-006) — structural lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}`. Caching is structural; learning is forbidden.
- **[ARCH]** Epistemic halt (Layer-1, ADR-022) — tagged-scalar slot + four universal-arithmetic predicates.
- **[ARCH]** Distillation pattern with kernel-enforced audit chain (§9.5; I11+I12+I13).
- **[ARCH]** ComplianceClaim runtime-context attestation (ADR + §8.5).
- **[ARCH]** Typed-intent A2A consent + intent_lineage (ADR-012, ADR-018).
- **[ARCH]** Skill-package overlay model for heterogeneous CLI Spirits (ADR-021).
- **[ARCH]** Constitutional substrate evolution (ADR-037, `invariant-lock` CI gate).

#### Notable cross-cutting concerns / risks

- **Kernel non-interpretability principle (§4.0.7):** kernel does NOT interpret tag semantics, author cognitive content, embed an orchestration policy, write/rank/curate skills, host Loom-class collective knowledge, or own application-layer concerns. Load-bearing across every component — kernel-API surface invariant test (NFR-Test-2) classifies functions; "other" class is build-break.
- **Capability mediation (I1) is in every path:** every external call MUST traverse Capability Registry. Touches every adapter, every Spirit lifecycle hook, every IAC frame.
- **Transparency Log log-before-deliver (I2):** kernel writes log before routing to mailbox; if log write fails, kernel panics rather than silently drop. Affects every IAC send path.
- **Five-metric distillation gate** applies to every distillation-shipping Spirit (Researcher, Orchestrator, Mira) — five metrics span auditability, security, cognition.
- **Audit log mandatory in every action path:** Transparency Log (I2), Approval Decision Log (I4), Lifecycle Journal (I10), digest audit-chain (I11), decision-context refs (I12), intent_lineage (I13), halt continuity across hot-swap (I14).
- **Phasing terminology mismatch:** PRD v0.8 = arch v0.9 for the founder-loop wedge demo. Reconcile when grouping FRs into epics.
- **Two Spirit forms only at v1.5:** architecture-minimal-opus retires ADR-007 (3-form portability). ADR-002 commits to subprocess at v0.1, rust-inproc gated on §13.1 measurement; WASM not in scope through v1.5.
- **Three trust tiers at v1.0:** architecture commits to `local`, `org-internal`, `public-untrusted`. PRD's `public-vetted` at v2.5 via FR37 (deferred). Stories use the architecture's three-tier model.
- **Crypto pluggability + FIPS readiness** crosses every adapter doing signing/encryption.
- **Manifest schema evolution** affects FR8, FR11, ABI triple, ComplianceClaim binding (§8.5 ABI break rule).
- **Hot-swap × halt continuity × crash matrix (ADR-033, binding-v0.3)** — most subtle correctness boundary; intersection of subprocess form + hot-swap state-transfer + halt continuity at the moment a subprocess Spirit dies.
- **Loom-lite is user-space (ADR-006), not a kernel module.** Kernel mediates access via MCP-Streamable-HTTP. NFR-Test-9 is the structural CI guard.
- **Two journeys (J3 Marcus, Reza Cortex) deferred from v1.5 architecture scope to v2.0+.** §10.7.3: no v1.5 architectural decision forecloses either.
- **§0.6 Foundational Commitments are non-negotiable across all phases:** kernel/Spirit separation; kernel learns nothing; human transparency as kernel invariant; one Spirit form at v0.1 (subprocess); every external call mediated; capability tokens unforgeable+short-lived+bound; epistemic halt as Layer-1; constitutional governance is structural.
- **OSS supply-chain hygiene from day one** (Apache 2.0 + MIT dual-license; SBOM per release; SLSA attestations; reproducible builds with `cargo build --locked`; signed Spirit-registry artifacts; `cargo deny check`).

### UX Design Requirements

_N/A — this is a kernel/infrastructure project with no UX design document. Director's-surface user interactions (FR14–FR20, FR51) are CLI / ACP / mobile-push flows specified in the PRD as functional requirements; no separate UX spec exists._

### FR Coverage Map (Revised after party-mode convergence — 12-epic structure)

| FR | Owner Epic(s) | Theme |
|---|---|---|
| FR1 | E1a (basic source build) + E9 (full distribution channels) | Install via package manager / cargo / signed binary |
| FR2 | E1a (basic stub) + E9 (FR65 proof-of-erasure full) | Clean uninstall |
| FR3 | E5 | Provider drivers per Spirit |
| FR4 | E1b (basic mediation) + E5 (full enforcement + audit completeness floor) | 100% capability mediation |
| FR5 | E1b (T0/T1/T2) + E5 (T3 v0.5; T4 WASM deferred v2.0) | Sandbox tiers |
| FR6 | E1b (basic cgroups) + E5 (full resource caps) | Per-Spirit resource caps |
| FR7 | E1a | Telemetry opt-in |
| FR8 | E1a | Manifest schema frozen v0.1 |
| FR9 | E1a (basic load/start/unload) + E5 (full pause/resume + lifecycle verbs) | Lifecycle verbs |
| FR10 | E5 | Hot-swap |
| FR11 | E5 | Cross-major migration (`migrates_from`) |
| FR12 | E5 | Crash detection ≤2s |
| FR13 | E5 | Runtime revocation via signed CRL |
| FR14 | E3 | `task.assign` IAC frame |
| FR15 | **E4 (SINGLE HALT OWNER)** | Halt resolution (3 pathways: provided_context / accepted_halt / authorized_override) |
| FR16 | E3 | Posture shift |
| FR17 | E3 (kernel log-composition primitives) + E8 (Butler/Researcher/Orchestrator Spirit-side digest) | Morning digest split |
| FR18 | E3 | Decision audit (I12 working_memory_digest_refs) |
| FR19 | E3 | Halt-policy schema |
| FR20 | E3 | Orchestrator instruction buffering |
| FR21 | E6 | Orchestrator dispatch with distillates |
| FR22 | E3 (basic routing) + E6 (full IAC features: mailbox/broadcast/retract) | IAC bus same-Host |
| FR23a | E6 | A2A loopback v0.8 (127.0.0.1 mTLS+TOFU) |
| FR23b | E6 | A2A cross-Host v1.0 |
| FR24 | E6 | Posture + intent provenance (I13) |
| FR25 | E6 | CliWrapperSpirit |
| FR26 | E6 | Scheduled invocations (ADR-025) |
| FR27 | E4 | Tagged scalars + 4 universal-arithmetic predicates |
| FR28 | E4 | Three memory tiers (private/shared/collective scaffold; full collective E10) |
| FR29 | E4 | `log.recall` / `log.fetch` |
| FR30 | E4 | Distillates + I11 audit chain |
| FR31 | E4 | Principal namespace (ADR-026) |
| FR32 | E4 | Per-tag epistemic policies |
| FR33 | E2 (thin cargo-generate slice for NFR-Onb-1 v0.3 readiness) + E7 (full per-language templates) | Spirit scaffolding |
| FR34 | E2 (SDK seed: local runner) + E7 (full spirit-test SDK with assertion macros) | spirit-test SDK |
| FR35 | E7 | Publish with Ed25519 signing |
| FR36 | E7 | Install third-party with admission verification + ComplianceClaim |
| FR37 | E7 (DEFERRED v2.5) | Vetter attestations |
| FR38 | E1b (schema FROZEN after E0 adversarial review) + E7 (envelope + admission verification v1.0) | ComplianceClaim |
| FR39 | E7 | Skill authoring + admission queue |
| FR40 | E2 (skeleton) + E7 (full fail-loud) | CliWrapper output_shape_version |
| FR41 | E9 | Audit frame-by-frame query |
| FR42 | E9 | DPO subject-access |
| FR43 | E9 | CISO posture-delta |
| FR44 | E9 | Sealed-export |
| FR45 | E9 | GDPR Article 17 right-to-be-forgotten + cross-Spirit cascade |
| FR46 | E9 | Trajectory export (ADR-023) |
| FR47 | E1a (Inference Port type skeleton) + E1b (Anthropic implementation) | Inference Port |
| FR48 | E1a (CryptoProvider trait def + default ring/rustls) + E9 (FIPS readiness audit) | Pluggable crypto |
| FR49 | E5 | Spirit upgrade with declared migration policy |
| FR50 | E5 | Dead-Spirit task disposition (`on_crash.action`) |
| FR51 | E3 | Director instant pause/resume/revoke (P99 ≤2s) |
| FR52 | E5 (T3 sandbox enforcement) + E6 (subprocess CLI under capability authority) | Subprocess CLI |
| FR53 | E5 (DEPENDENT on E4 halt mechanism) | Halt continuity across hot-swap (I14, ADR-019) |
| FR54 | E6 | Gateway sub-modules under principal namespace (ADR-029) |
| FR55 | E2 (ABI hook signatures) + E5 (full trigger firing) | Lifecycle triggers (on_load/on_idle/on_swap_in/etc.) |
| FR56 | E4 | Spirit self-telemetry within principal namespace |
| FR57 | E7 | Skill-revision proposals |
| FR58 | E1b (v0.1 hello-spirit) + E8 (v0.3+ per-phase reference Spirits) | Zero-config evaluator path |
| FR59 | E7 | Registry yank events |
| FR60 | E7 | Air-gapped artifact import |
| FR61 | E1a | SECURITY.md |
| FR62 | E9 | Governance audit-queryable artifacts |
| FR63 | E9 | Typed error catalog (`https://docs.maos.dev/errors/<ERR_NAME>`) |
| FR64 | E9 | Cost attribution per Spirit/task/principal |
| FR65 | E9 | Proof-of-erasure on uninstall (externally-verifiable Merkle proof) |

**Coverage:** All 65 FRs mapped to ≥1 epic. Per-NFR ownership is embedded in each epic section below; full corpus authoring schedule (~2,249 items via parameterized generators) lives in E0 + per-epic corpus stories.

## Epic List (Revised — 12-Epic Consensus Structure)

> **Crate-path retrofit status (round-3 + retrofit pass):**
>
> Stories with **full crate-path treatment** — every AC cites a concrete file or test path that a dev agent can act on directly:
> - **Story 1a.1** (workspace bootstrap — original exemplar Amelia praised)
> - **Story 1b.5a / 1b.5b / 1b.5c** (split from original 1b.5 with concrete paths throughout)
> - **Story 4.1** (halt mechanism + `MockHaltResolver` pattern + `crates/maos-eval/fixtures/halt-corpus-v0/` synthetic corpus)
> - **Story 3.3** (Director halt UX — integration partner of 4.1)
> - **Story 5.2** (Hot-Swap Coordinator + HSIS corpus authoring schedule — integration partner of 4.1)
> - **Story 5.5a–5.5e** (5 stories split from original 5.5 with paths)
> - **Story 7.5a / 7.5b** (2 stories split from original 7.5 with paths)
> - **Story 0.5** (corpus generators in `crates/maos-corpus-gen/`)
> - **Story 9.2** (GDPR cascade + proof-of-erasure with paths)
>
> Stories with **partial crate-path treatment** (some ACs cite paths, some still say "the kernel"):
> - **Stories 0.1–0.4** (CI gates cite `xtask/`, `tests/coverage-matrix.yaml` but not all kernel-touching ACs)
> - **Stories 1a.2 / 1a.3 / 1a.4** (cite `maos-kernel-core/` and `maos-spirit-abi/` but variably)
> - **Stories 1b.1–1b.4** (cite some crate paths but not consistently)
> - **Stories 2.1–2.4** (cite `maos-spirit-sdk` etc. but not all ACs)
> - **Stories 3.1 / 3.2 / 3.4** (some kernel-touching ACs still generic)
> - **Stories 4.2–4.5** (cite ADR numbers but not all crate paths)
> - **Stories 5.1 / 5.3 / 5.4** (cite some component names but not all)
> - **Stories 6.1–6.5** (cite ADR numbers but not all crate paths)
> - **Stories 7.1–7.4** (cite some interfaces but not all)
> - **Stories 8.1–8.5** (mostly cite `spirits/<class>/` paths which is appropriate since these are subprocess Spirit stories)
> - **Stories 9.1 / 9.3 / 9.4 / 9.5** (cite some `maosctl` subcommand paths but not all)
> - **Stories 10.1–10.5** (ship-gate coordination — cite gate artifacts but less crate-internal detail since these are integration-test stories)
>
> **For dev-agent consumption:** when an AC says "the kernel" without a crate path, the implementing agent should consult `architecture-maos-minimal-opus.md` §4.0.2 for the canonical crate-to-responsibility mapping. The 17-crate workspace is bounded; "the kernel" almost always maps to `crates/maos-kernel-core/<service>/` based on the AC's subject matter.
>
> **User decisions (party-mode convergence):** (1) **E4 is the single halt owner** — schema in E1a/E1b types only; mechanism + I14 invariant in E4; continuity-across-hot-swap dependent in E5. (2) **ComplianceClaim schema frozen at E1b** after E0 adversarial review; ABI break required to change thereafter. (3) **rust-inproc form gated on §13.1 measurement** — story lives in E5 with go/no-go before v0.5 ships; if subprocess form meets latency budgets, rust-inproc may be deferred. (4) **12 epics** per Winston's structure with E1 split explicitly and E8 single epic with sub-stories per Spirit cohort.

> **Architectural seam discipline carried by E0 (Murat's adoption, Winston's yield):** the kernel-API surface invariant (NFR-Test-2), empty-kernel invariant I9 (ADR-006), Loom-not-in-kernel grep (NFR-Test-9), KLOC budget alarm (`tokei`, ≤20 KLOC, alarm at 16), reproducible build gate, zero-`unsafe` capability-path gate, content-addressed corpus infrastructure (NFR-Test-1), coverage matrix CI gate (NFR-Meta-3), and ComplianceClaim schema adversarial review all live in E0 and run on every PR forever. E0 is a founding sprint with a v0.1 acceptance criterion that thereafter transitions to a maintenance discipline owned by whoever holds the repo.

> **KLOC budget tally (Winston's estimate):** ~18–27 KLOC for kernel trusted core across E1a + E1b + E2 + E3 + E4 + E5 + E6 + E7 (env+adapter) + E9 + E10. Alarm at 16 expected to fire during E6/E7 — budget tracked per-merge via E0's `tokei` gate. Reference Spirits (E8) and Spirit-side ecosystem code carry zero kernel KLOC. Upper bound bleeds past 20 KLOC; if alarm fires hard, scope-cut decisions happen at merge time, not ship time.

### Epic 0: Quality Substrate (cross-cutting; founding sprint v0.1; maintenance track thereafter)

**Goal:** Every CI gate green from day one. Without E0, every subsequent epic's gated NFR is a check against a bank that doesn't exist. This is the substrate-of-the-substrate.

**Owns (continuous CI gates):**
- `cargo xtask check-service-boundary` P1–P4 stub (full implementation in E2) — kernel-API surface invariant (NFR-Test-2): build-time reflection classifies every kernel API as universal-arithmetic / data-movement / supervision / **other**; new function in "other" class is build-break.
- Empty-kernel invariant I9 (ADR-006) — structural lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}`.
- Loom-not-in-kernel grep (NFR-Test-9) — `grep` of kernel crate for orchestration/planning symbols returns ∅.
- KLOC budget enforcement (`tokei`, `xtask/kloc.toml`, aggregate ≤20 KLOC, alarm at 16) — NFR-Maint-1.
- Reproducible build gate (`cargo build --locked` on Rust stable; no nightly).
- Zero-`unsafe` in kernel capability-validation path (NFR-Sec-9 — gate from day one).
- Content-addressed corpora infrastructure (NFR-Test-1: SHA-256 of JSONL, pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash, retry budget=1, quarterly re-baseline ≥98% on golden snapshot).
- Coverage matrix CI gate (NFR-Meta-3: `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}; CI fails if any delivered FR/NFR has zero corpus).
- Corpus-quality audit rubric (NFR-Meta-1: ≥8/10 per corpus, 12-month re-audit).
- Corpus-staleness `valid_until` enforcement (NFR-Meta-2: CI fails if any active gate references an expired corpus; default validity 12 months).
- Invariant-lock CI gate (ADR-037) on every PR touching I1–I14.
- ABI-diff lint (per architecture-minimal-opus §CI-gates).
- Calibration harness infrastructure (NFR-Aud-8: N=100 per-commit pipeline + N=500 quarterly audit runner — corpus content authored per-epic).
- **ComplianceClaim schema adversarial review** before E1b freezes (Mary + Winston joint demand).

**Corpora authored in E0:**
- Calibration seed corpus N=100 (NFR-Aud-8 per-commit slice).
- Coverage matrix skeleton: 0-item rows for every FR + NFR, populated by owning epics.

**v0.1 founding-sprint acceptance:** CI pipeline green on empty workspace; coverage matrix template populated; corpus harness operational; calibration seed corpus committed; ComplianceClaim schema adversarial review report signed off; PR adding a persistent field outside I9 whitelist is rejected by CI.

**No FRs.** Cross-cutting infrastructure that gates every subsequent epic.

#### Stories

### Story 0.1: Workspace CI Pipeline + Build Discipline Gates

As a maintainer of MAOS,
I want every PR to be gated by build-discipline checks (reproducible build, zero-`unsafe` in capability-validation path, KLOC budget alarm, ABI-diff lint, invariant-lock CI gate),
So that architectural commitments cannot erode silently between v0.1 and v2.0.

**Acceptance Criteria:**

**Given** a fresh checkout of the MAOS workspace
**When** `cargo build --locked` runs on Rust stable
**Then** the build produces a reproducible artifact
**And** the build fails if any nightly feature is referenced

**Given** a PR that introduces `unsafe { … }` anywhere in `maos-kernel-core/capability/`
**When** CI runs `cargo xtask check-unsafe`
**Then** the PR is rejected with `NFR-Sec-9 violation: zero-unsafe gate failed in capability-validation path`

**Given** the workspace exceeds 16 KLOC of kernel trusted core measured by `tokei` per `xtask/kloc.toml`
**When** CI runs the KLOC budget check
**Then** a warning alarm fires labelled `NFR-Maint-1 alarm — 16 KLOC threshold reached`
**And** the build hard-fails at 20 KLOC aggregate

**Given** a PR that changes the public ABI surface of `maos-spirit-abi`
**When** CI runs the ABI-diff lint
**Then** the diff is annotated against the previous tagged ABI version
**And** the lint enforces the ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` rules

**Given** a PR that touches any of the 14 invariants I1–I14 in `maos-domain`
**When** CI runs the `invariant-lock` job (ADR-037)
**Then** the PR is blocked unless ≥2 maintainer sign-offs are present on the lock-edit commit
**And** the journal records the invariant-lock decision

**Given** the founding-sprint acceptance for E0
**When** CI runs on an empty workspace (no production code yet)
**Then** every build-discipline gate is green
**And** the green run is committed as the v0.1-α CI baseline

### Story 0.2: Enforce Empty-Kernel Invariants via Structural CI Lints

As a MAOS architect,
I want structural lints that block kernel growth in ways that would violate the empty-kernel invariant (I9, ADR-006), smuggle orchestration logic into the kernel (NFR-Test-9), or add functions outside the permitted computational classes (NFR-Test-2),
So that the kernel-as-substrate commitment is mechanically enforced at PR-merge time, not merely a code-review aspiration.

**Acceptance Criteria:**

**Given** a PR that adds a persistent struct field outside the three sanctioned locations (`Journal`, `TransparencyLog`, `CapabilityRegistry::tokens`)
**When** CI runs the I9 structural lint
**Then** the PR is rejected with `I9 violation: persistent field <field_name> not in I9 whitelist`

**Given** the MAOS kernel crate (`maos-kernel-core/`)
**When** CI runs `grep` for orchestration/planning symbols (`Loom`, `Planner`, `Goal`, `Orchestrator` types in the kernel crate)
**Then** the grep result returns ∅
**And** the PR is rejected with `NFR-Test-9 violation: Loom-not-in-kernel grep matched <symbol>` if any match is found

**Given** a PR that adds a new public kernel API function exported via `kernel::api::*`
**When** the `cargo xtask check-service-boundary` job classifies the function via Rust `syn` static analysis
**Then** the function MUST be classified as one of: `universal-arithmetic`, `data-movement`, `supervision`
**And** the build hard-fails if the function falls into class `other`
**And** the violation surfaces with NFR-Test-2 reference

**Given** the I9 / NFR-Test-2 / NFR-Test-9 gates are wired
**When** an attempt PR is opened that deliberately violates each gate
**Then** all three gates fail independently
**And** the failure messages are actionable (include the offending file, line, and rule citation)

### Story 0.3: Content-Addressed Corpora Infrastructure + Coverage Matrix CI Gate

As a test architect,
I want the content-addressed corpus harness (SHA-256-pinned JSONL, pinned model versions, temperature=0 judge calls, deterministic retry budget, quarterly re-baseline pipeline) AND the `tests/coverage-matrix.yaml` CI gate to ship together,
So that every gated NFR has a measurable corpus and CI fails the moment any delivered FR/NFR has zero corpus coverage.

**Acceptance Criteria:**

**Given** a corpus JSONL file committed to `tests/corpora/`
**When** the corpus is loaded by any CI gate
**Then** the load verifies the corpus's SHA-256 against the committed manifest
**And** mismatches fail the build with `NFR-Test-1 violation: corpus integrity broken`

**Given** a judge-LLM call inside any test gate
**When** the call is dispatched
**Then** the model version is pinned to a fixed identifier
**And** `temperature=0, top_p=1.0, seed` are set where the provider supports them
**And** the retry budget is exactly 1
**And** the prompt-version hash is committed alongside the corpus

**Given** the quarterly re-baseline pipeline
**When** the runner re-executes all gated corpora against pinned models
**Then** agreement with the golden snapshot is ≥98%
**And** any deviation triggers a re-baseline review issue (NFR-Test-1)

**Given** `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}
**When** CI runs the coverage-matrix gate (NFR-Meta-3)
**Then** the build fails if any FR or NFR with phase-status `delivered ≤ current-phase` has zero corpus rows

**Given** a corpus row in `coverage-matrix.yaml`
**When** the corpus's `valid_until` date is in the past
**Then** CI fails with `NFR-Meta-2 violation: corpus expired <date>; either extend or rebuild`
**And** an explicit no-update justification PR with assessor sign-off is required to extend

**Given** the calibration harness
**When** the N=100 per-commit pipeline runs
**Then** the pipeline emits CI-width ≈ 0.124 (sufficient for trend detection per NFR-Aud-8)
**And** the quarterly N=500 audit pipeline emits CI-width ≤ 0.05 at p=0.90 for digest-recall

### Story 0.4: ComplianceClaim Schema Adversarial Review + Calibration Seed Corpus

As a substrate-of-the-substrate maintainer,
I want the ComplianceClaim schema adversarially reviewed before E1b freezes it, AND the v0.1 calibration seed corpus N=100 committed alongside the coverage-matrix template,
So that the schema's binding-v0.1 ABI commitment is not built on shaky ground and the corpus discipline runs from day one.

**Acceptance Criteria:**

**Given** the ComplianceClaim schema draft from `maos-spirit-abi/src/compliance.rs`
**When** the adversarial review panel (≥2 reviewers external to the schema author) examines the schema
**Then** the panel produces a signed-off review report in `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`
**And** the report enumerates each field's `secret`/`non-secret` classification (NFR-Sec-16)
**And** the report explicitly checks context-drift attack surfaces (manifest hash, version, trust tier, sandbox tier, capability scope, provider-endpoint, crypto-provider)

**Given** the review report is signed off
**When** E1b moves to freeze the schema
**Then** the schema's `ABI_VERSION` is committed and the freeze event is journaled

**Given** the calibration seed corpus N=100 (clearly-decidable bucket, distributed across categories per NFR-Aud-8)
**When** the corpus is committed to `tests/corpora/calibration-seed-v0.1.jsonl`
**Then** the corpus is SHA-256-pinned per Story 0.3
**And** the corpus is registered in `tests/coverage-matrix.yaml`
**And** the corpus carries a `valid_until` date 12 months out

**Given** the coverage-matrix template
**When** initial population occurs
**Then** every FR (FR1–FR65) and every NFR has at least a 0-item row in `coverage-matrix.yaml`
**And** the gate runs in warning-only mode for v0.1 founding sprint before becoming a hard gate at v0.3

### Story 0.5: Parameterized Corpus Generators — Secret-Redaction + Red-Team Frameworks

As the test-architecture lead facing ~2,249 hand-authored corpus items across the v1.0 + v1.5 ship gates,
I want two parameterized generator frameworks committed early: `crates/maos-corpus-gen/src/secret_redaction/` (produces the 10⁴ per-commit + 10⁵ quarterly secret-leakage corpora from ~200 seed patterns) AND `crates/maos-corpus-gen/src/red_team/` (produces the 640-item adversarial-Spirit red-team corpus from 80 canonical scenarios across 8 attack classes),
So that scheduling fictions (hand-authoring 10,000+ items) collapse to engineering artifacts — generator + seed + expansion rules — and downstream Stories 6.x / 10.2 / 10.3 can consume large corpora without inventing them at gate time.

**Acceptance Criteria:**

**Given** the `crates/maos-corpus-gen/` workspace crate
**When** the crate is compiled
**Then** the crate exposes a `CorpusGenerator` trait declared in `crates/maos-corpus-gen/src/lib.rs` with methods: `seed_corpus()`, `expand(n: usize)`, `validate(item: &Item) -> ValidationOutcome`, `coverage_report() -> CoverageReport`
**And** generator output is deterministic given a seed file SHA and an expansion-rule version
**And** generator output is SHA-256-pinned per Story 0.3's corpus discipline

**Given** the secret-redaction generator (`crates/maos-corpus-gen/src/secret_redaction/`)
**When** the generator runs with ~200 seed patterns covering all secret classes (API keys / OAuth tokens / private keys / database URLs / JWT / AWS / GCP / Azure / SSH / GPG)
**Then** the per-commit run produces 10⁴ deduplicated items in `tests/corpora/secret-redaction-1e4-<sha>.jsonl` (NFR-Sec-4)
**And** the quarterly run produces 10⁵ items via wider parameter sweep
**And** the 1000-canary-per-month production canary corpus is produced independently with cryptographic markers (NFR-Sec-4 floor)
**And** any expansion rule that produces a false negative (i.e., a real secret missed by the redactor) is a P0 ship-block

**Given** the red-team generator (`crates/maos-corpus-gen/src/red_team/`)
**When** the generator runs with 80 canonical seed scenarios across 8 attack classes (capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse — N=10 per class)
**Then** the expansion produces ≥640 deduplicated items in `tests/corpora/red-team-640-<sha>.jsonl` (NFR-Sec-10)
**And** the per-class floor is ≥80 items after expansion (8× from N=10 seed)
**And** deduplication preserves coverage: every seed scenario appears in expanded form

**Given** the generator coverage report
**When** `cargo run -p maos-corpus-gen -- coverage --corpus <name>` runs
**Then** the report shows attack-class coverage, parameter-space coverage, and any unexpanded seed slots
**And** the report is consumed by Story 10.2 (red-team gate) and Story 9.4 (secret-redaction operational canary review)

**Given** the v0.5 readiness handoff
**When** Story 5.5b (multi-provider CI) needs secret-redaction tests in CI
**Then** the 10⁴ per-commit corpus is already available
**And** Story 6.x ConsentRupture testing has the red-team generator available for adversarial fixtures

---

### Epic 1a: Workspace Bootstrap + ABI Freeze + Kernel Skeleton (v0.1-α)

**Goal:** `cargo new` the canonical 17-crate Cargo workspace per `architecture-maos-minimal-opus.md` §4.0.2. Land 14 binding-v0.1 ADRs simultaneously. Wire kernel-core skeleton as empty shells with frozen ABI types. Story 1.1 carries the **starter-template flag**.

**Owns:**
- 17-crate Cargo workspace scaffold (`maos-domain`, `maos-spirit-abi`, `maos-kernel-core/*`, `maos-spirit-sdk`, `maos-spirit-hello`, `maos-providers`, `maos-mcp`, `maos-acp`, `maos-a2a`, `maos-persistence`, `maos-secrets`, `maos-compliance`, `maos-control`, `maos-cli`, `maos-bin`, `spirits/`, `schemas/`, `fuzz/`, `wit/spirit.wit`).
- `maos-domain` codifies invariants I1–I14 (zero deps; no tokio/reqwest/sqlx; `serde + thiserror`).
- `maos-spirit-abi` frozen with `src/compliance.rs` (ComplianceClaim schema types) — `#![no_std]`, wire-stable.
- `maos-kernel-core` skeleton: five services (scheduler / memory / security / iac / capability) + two internal modules (io / telemetry) as empty shells with hexagonal port boundaries declared (ADR-010).
- `maos-bin` composition root with `#[tokio::main(flavor = "multi_thread")]` (ADR-011: single multi-threaded Tokio runtime).
- `maosctl` skeleton (`install`, `start`, `stop`, `unload` stubs).
- SECURITY.md (`security@maos.dev` GPG key, 90-day embargo, advisory-publication channel, supported-versions matrix — NFR-Ops-4).
- `cargo xtask check-service-boundary` STUB (boundary types defined; full P1–P4 enforcement in E2 once Spirit ABI exists).
- `CryptoProvider` trait definition + default `ring`/`rustls` implementation (FR48 architectural commitment).

**ADRs binding simultaneously (14 binding-v0.1):** ADR-001 (Rust+Tokio), ADR-002 (subprocess form at v0.1; rust-inproc gated on §13.1), ADR-004 (sandbox tier ladder declared), ADR-006 (empty-kernel I9 — enforced by E0), ADR-010 (hexagonal architecture), ADR-011 (actor model on hot path), ADR-012 (typed-intent A2A consent — types only; runtime in E6), ADR-014 (storage/journal foundation), ADR-022 (epistemic halt skeleton — types only; mechanism in E4), ADR-023 (capability-token TTL ≤60s + PID-binding — types only; runtime in E1b), ADR-026 (principal namespace types — runtime in E4), ADR-030 (capability registry decomposition — types only), ADR-032 (subprocess wire protocol LSP-style + CBOR — types only), ADR-037 (invariant-lock CI gate — enforced by E0).

**FRs covered:** FR1 (basic source install path `cargo install --path crates/maos-bin`), FR2 (basic uninstall stub), FR7 (telemetry opt-in declared default), FR8 (manifest schema frozen; signed + journaled at runtime), FR47 (Inference Port type skeleton), FR48 (CryptoProvider trait + default), FR61 (SECURITY.md).

**Key NFRs:** NFR-Sec-9 (zero-`unsafe` in capability path), NFR-Maint-2 v0.1 floor (capability-registry fuzz ≥60% line), NFR-Tenancy-1 (single-tenant declared).

**KLOC budget:** ~2–3 KLOC. Alarm if this exceeds 4 — means logic smuggled in.

**Acceptance demo:** `cargo build --locked` produces signed `maos-bin` binary; `cargo xtask check-service-boundary` passes (stub mode); SECURITY.md renders; `maosctl --version` runs.

#### Stories

### Story 1a.1: Initialize 17-Crate Cargo Workspace + Frozen ABI Types (Starter Template)

As a founding MAOS contributor,
I want the canonical 17-crate Cargo workspace per `architecture-maos-minimal-opus.md` §4.0.2 scaffolded with `maos-domain` invariants I1–I14 codified and `maos-spirit-abi` frozen with the ComplianceClaim schema types,
So that all subsequent epics build against a stable, ADR-bound workspace shape from day one. **This story carries the starter-template flag.**

**Acceptance Criteria:**

**Given** an empty repository
**When** the workspace bootstrap story is executed
**Then** the repository contains the exact crate layout from §4.0.2 (17 crates under `crates/`, plus `spirits/`, `schemas/`, `docs/`, `fuzz/`, `wit/spirit.wit`)
**And** `cargo build --locked` succeeds on Rust stable for the empty workspace

**Given** the `maos-domain` crate
**When** the crate is compiled
**Then** the crate has zero async dependencies (no tokio/reqwest/sqlx; only `serde + thiserror`)
**And** invariants I1 through I14 are codified as types with doctested invariant statements
**And** the crate compiles without a Tokio runtime present

**Given** the `maos-spirit-abi` crate
**When** the crate is compiled
**Then** the crate is `#![no_std]`
**And** the crate contains `src/compliance.rs` with the frozen ComplianceClaim schema types
**And** the crate contains the wire-stable Spirit ABI types

**Given** the 14 binding-v0.1 ADRs (ADR-001, 002, 004, 006, 010, 011, 012, 014, 022, 023, 026, 030, 032, 037)
**When** the workspace bootstrap completes
**Then** each ADR is committed to `docs/adr/` with status `accepted`
**And** the ADR identifiers are journaled in `docs/adr/index.md`

**Given** the workspace
**When** an external author runs `git clone` and `cargo build --locked`
**Then** the starter-template flag is satisfied: the build reproduces the v0.1-α baseline without bespoke setup

### Story 1a.2: Wire the Five-Service Kernel Skeleton with a Multi-Threaded Tokio Composition Root

As a kernel implementer,
I want the five supervised kernel services (Spirit Scheduler / Security Manager / Memory Manager / IAC Bus / Capability Registry) and two internal modules (I/O / Telemetry) wired as empty hexagonal shells with their port/adapter boundaries declared, AND the `maos-bin` composition root driving a single multi-threaded Tokio runtime,
So that all subsequent feature epics have a ready socket to plug runtime logic into without re-litigating service boundaries.

**Acceptance Criteria:**

**Given** the `maos-kernel-core` crate
**When** the crate is compiled
**Then** the crate exports five service modules (`scheduler/`, `memory/`, `security/`, `iac/`, `capability/`) and two internal modules (`io/`, `telemetry/`)
**And** the `capability/` module is decomposed per ADR-030 into `cap-tokens/`, `cap-policy/`, `cap-audit/`, `cap-quota/` subdirectories with empty type shells
**And** each service has its hexagonal port trait declared in `maos-domain` and adapter implementations stubbed in `maos-kernel-core/<service>/`

**Given** the `maos-bin` composition root
**When** the binary is compiled
**Then** `main.rs` uses `#[tokio::main(flavor = "multi_thread")]` per ADR-011
**And** the worker count is configured to the number of CPU cores
**And** every long-lived coordination task takes a `CancellationToken` (from `tokio-util`)
**And** root-level shutdown cancels all child tasks via `select!` with cancellation arm

**Given** the kernel-core skeleton
**When** `cargo xtask check-service-boundary` runs in stub mode
**Then** the xtask passes with all five services classified by computational class (universal-arithmetic / data-movement / supervision)
**And** no service exposes methods in the `other` class

**Given** the hexagonal architecture (ADR-010)
**When** the crate-boundary lint runs
**Then** `maos-domain` does not import any I/O adapter
**And** services depend only on their port traits, never on adapter implementations
**And** the lint hard-fails on any port→adapter direct reference

### Story 1a.3: CryptoProvider Trait + xtask Service-Boundary Stub Implementation

As a kernel security architect,
I want the `CryptoProvider` trait plumbed end-to-end as the indirection point for signature verification, sealed-export encryption, and capability-token signing (with a default `ring`/`rustls` implementation) AND the `cargo xtask check-service-boundary` P1–P4 four-property test stub committed,
So that FIPS-validated, hardware-backed, and post-quantum crypto can be substituted in later phases without recompiling Spirits, and the kernel-API surface invariant has a stub enforcer from day one.

**Acceptance Criteria:**

**Given** the `CryptoProvider` trait in `maos-kernel-core/security/crypto.rs`
**When** the trait is compiled
**Then** the trait declares operations for signature verification, sealed-export encryption, and capability-token signing
**And** the trait is implemented by the default `ring`/`rustls` adapter
**And** all kernel call sites for cryptographic operations route through the trait, never the default adapter directly

**Given** the FR48 architectural commitment
**When** a v1.0+ alternate provider (FIPS-validated / HSM-backed / post-quantum) is plugged in
**Then** the swap is a composition-root-level change in `maos-bin/main.rs`
**And** no Spirit binary requires recompilation (verified by ABI-diff lint)

**Given** the `cargo xtask check-service-boundary` P1–P4 stub
**When** the xtask runs against the empty kernel-core skeleton
**Then** P1 (service has a single supervising owner) passes for all five services
**And** P2 (service exposes ports, not adapters) passes for all five services
**And** P3 (service is stateless or owns its state behind `Arc<DashMap>`/`RwLock`) passes for all five services
**And** P4 (audit-chain integrity at service boundary) is stubbed pending E2's full ABI types
**And** the stub clearly reports which properties are stubbed vs enforced

### Story 1a.4: Ship the maosctl CLI Scaffold with SECURITY.md and Accessibility Defaults

As an evaluator,
I want a `maosctl` CLI scaffold with v0.1 subcommands stubbed (`install`, `start`, `stop`, `unload`, `run`, `audit`) plus accessibility flags (`--plain`, honors `NO_COLOR` and `TERM=dumb`) AND a complete `SECURITY.md` shipped before any external Spirit can run,
So that the operator surface and security disclosure pipeline exist on day one — not after the first vulnerability report.

**Acceptance Criteria:**

**Given** the `maos-cli` crate compiled to `maosctl`
**When** `maosctl --help` runs
**Then** the help output lists the v0.1 subcommands (`install`, `start`, `stop`, `unload`, `run`, `audit`)
**And** the help respects `NO_COLOR` and `TERM=dumb` environment variables (NFR-Ops-5)
**And** the `--plain` flag suppresses all ANSI color sequences

**Given** the SECURITY.md file at the repo root
**When** the file is read
**Then** the file documents `security@maos.dev` as the disclosure contact with a published GPG key
**And** the file documents the 90-day coordinated-disclosure embargo window (NFR-Ops-4)
**And** the file documents the supported-versions matrix for security backports
**And** the file documents the advisory-publication channel

**Given** the v0.1 ship gate
**When** the SECURITY.md presence check runs
**Then** the gate passes only if `SECURITY.md` exists, parses, and includes all four required sections (disclosure address, embargo, supported-versions, advisory channel)
**And** the gate is part of E0's continuous CI

**Given** a fresh OS install (Linux or macOS)
**When** the user runs `cargo install --path crates/maos-bin`
**Then** the install succeeds without nightly features (FR1 v0.1 source-build slice)
**And** `maosctl --version` reports the workspace version from `Cargo.toml`

---

### Epic 1b: Evaluator Path + Audit Spine + Capability Mediation Baseline (v0.1-β)

**Goal:** An evaluator clones the repo, runs `maosctl install && maosctl run hello-spirit` within 5 minutes, gets a structured hello-spirit response, AND verifies via the Transparency Log that every external call was capability-mediated.

**Owns:**
- Inference Port implementation (Anthropic provider at v0.1; ADR-005).
- Sandbox tiers T0/T1/T2 enforcement (ADR-004: T0 trusted local-tier; T1 process isolation/UID separation; T2 Linux Landlock+seccomp / macOS Seatbelt / Windows restricted-token).
- **ComplianceClaim schema FROZEN** (after E0 adversarial review) + structural validator (~200 LOC in `maos-kernel-core/compliance/`).
- Transparency Log per-Host SQLite (append-only; `log-before-deliver` I2 guarantee).
- Approval Decision Log (separate from Transparency Log; full intent + decision + reasoning chain per I4).
- Lifecycle Journal (append-only on-disk log of lifecycle transitions; fsync per transition; ring-buffer flush <1ms).
- Telemetry IAC round-trip metrics (binding from v0.1: `iac_rt_duration_us` histogram with buckets `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]`, labels `service ∈ {security, memory, iac, capability, spirit_scheduler}`).
- Per-Spirit resource caps basic (cgroups v2 on Linux; setrlimit on macOS; Job Object on Windows — declared in manifest `[resources]`).
- `maos-spirit-hello` reference Spirit (validates SDK end-to-end at v0.1 ABI level — structured introduction with capability scope, posture, expected halt-tags, Transparency Log link).
- `maosctl` v0.1 commands: `install`, `start`, `stop`, `unload`, `run`, `--plain` flag, `NO_COLOR`/`TERM=dumb` accessibility (NFR-Ops-5).
- Capability Registry runtime decomposition (cap-tokens hot-path: sharded `Arc<[CapShard; 64]>` lock-free, P99 <5µs; cap-policy read-mostly copy-on-write; cap-audit bounded `mpsc::channel(8192)` to single audit-writer task; cap-quota per-Spirit atomic counters).
- Manifest field test coverage ≥3 cases per field (NFR-Test-13: well-formed / malformed-rejected / edge-case).
- Pluggable Inference Port routing — Spirit binaries do not import vendor LLM SDKs directly (FR47 closure).

**FRs covered:** FR4 (basic 100% mediation in any 1000-call sample), FR5 (T0/T1/T2 strictest-of-(manifest, trust-tier, operator-policy) floor), FR6 (basic resource caps via cgroups v2 / setrlimit / Job Object), FR9 (basic load/start/unload via authenticated control plane), FR38 (ComplianceClaim schema FROZEN — envelope + admission verification deferred to E7), FR47 (Inference Port Anthropic implementation), FR58 (v0.1 hello-spirit J0 evaluator path).

**Key NFRs:** NFR-Onb-2 (5-min installer J0 path), NFR-Obs-4 (Transparency Log SQLite append-only with JSONL export), NFR-Rel-8 (lifecycle journal fsync per transition, flush <1ms), NFR-Perf-3 (capability-token validation P99 <100µs with 100% re-validation at use against current state — TOCTOU correctness, NFR-Maint-8), NFR-Test-13 (manifest field ≥3 cases), NFR-Sec-1 v0.1 slice (T0/T1/T2 enforcement floor).

**Corpora authored in E1b:**
- Kernel-API invariant cases ~30 (NFR-Test-2 population).
- Manifest field cases ~15 (NFR-Test-13 population for kernel manifest fields).

**KLOC budget:** ~1–2 KLOC. The three sanctioned persistent locations (Journal, TransparencyLog, CapabilityRegistry::tokens) populated here.

**Acceptance demo:** `maosctl run hello-spirit` within 5 minutes of fresh install; Transparency Log query shows every external call with capability token + Spirit-PID + boot-nonce; J0 evaluator hello-spirit budget met.

#### Stories

### Story 1b.1: Three Audit Logs — Transparency / Approval Decision / Lifecycle Journal

As a substrate auditor,
I want three distinct, durable, append-only logs (Transparency Log per-Host SQLite, Approval Decision Log per I4, Lifecycle Journal per I10) with kernel-level log-before-deliver guarantees,
So that every external call, every approval-prompt resolution, and every lifecycle state transition is journaled before it takes effect — making the substrate's behavior mechanically auditable.

**Acceptance Criteria:**

**Given** a Spirit emits an IAC frame routable through the kernel
**When** the kernel writes the frame to the Transparency Log
**Then** the write completes before routing the frame to the recipient mailbox (I2 log-before-deliver guarantee)
**And** if the log write fails, the kernel panics rather than silently dropping the frame
**And** the log entry includes the capability token, Spirit-PID, boot-nonce, and timestamp

**Given** an Approval Manager prompt resolution event (intent + decision + reasoning)
**When** the kernel processes the approval decision
**Then** the resolution is written to the Approval Decision Log (distinct from Transparency Log per I4)
**And** the log entry captures `(actor, target, capability, intent, decision, reasoning_if_any)`

**Given** a Spirit lifecycle state transition (load / start / pause / resume / unload / crash / swap)
**When** the kernel records the transition
**Then** the transition is appended to the Lifecycle Journal (I10)
**And** an `fsync` is issued per state transition
**And** the ring-buffer flush latency is < 1ms P99 (NFR-Rel-8)
**And** crash recovery on next boot rehydrates Spirit state from the journal

**Given** all three logs are SQLite-backed per-Host
**When** an operator queries any log via `maosctl audit` (full audit surface in E9)
**Then** logs export to JSONL with applied redaction policy
**And** the pre-write secret-redaction filter (corpus from E0; adapter wired here) blocks secret leakage at the Transparency Log boundary
**And** logs are append-only — no deletion path except via GDPR Article 17 cascade (E9)

### Story 1b.2: Capability Registry Decomposition Runtime — cap-tokens / cap-policy / cap-audit / cap-quota

As a capability-mediation guarantor,
I want the Capability Registry decomposed at runtime per ADR-030 into four cooperating components (`cap-tokens` lock-free hot path, `cap-policy` read-mostly copy-on-write, `cap-audit` slow-path single-writer task, `cap-quota` per-Spirit atomic counters) with capability-token TTL ≤60s + Spirit-PID/boot-nonce binding + TOCTOU re-validation at every use,
So that every external call (file op, network, exec, sub-Spirit spawn) is mechanically mediated and the mediation hot path stays under 5µs P99.

**Acceptance Criteria:**

**Given** the `cap-tokens` shard (`Arc<[CapShard; 64]>` lock-free)
**When** a capability-token issuance or verification is dispatched
**Then** the hot-path latency is P99 < 5µs (per architecture-minimal-opus performance budgets)
**And** validation is 100% re-validation at use against current state, never cached state (TOCTOU correctness, NFR-Maint-8)
**And** tokens are bound to `(Spirit-PID + boot-nonce + expiry)` and Ed25519-signed via `CryptoProvider`
**And** capability-token TTL ≤60s for high-privilege operations (ADR-023)

**Given** the `cap-policy` component
**When** an operator updates a policy at runtime
**Then** the update is read-mostly copy-on-write — readers never block on writers
**And** policy reads include the strictest-of-(manifest, trust-tier, operator-policy) floor

**Given** the `cap-audit` component
**When** a capability use is observed
**Then** the audit event is enqueued onto a bounded `tokio::sync::mpsc::channel(8192)` to a single audit-writer task
**And** the audit-writer task writes to the Transparency Log
**And** the hot path never blocks on audit writes

**Given** the `cap-quota` component
**When** a Spirit's quota approaches its budget
**Then** the kernel emits `ContextPressure` at 80% utilization
**And** emits `ContextLimit` at 95%
**And** rejects further capability requests with `EContextExhausted` above 100%

**Given** the v0.1 ship gate
**When** capability-token validation is benchmarked on a typical Linux box (NVMe + 16-core tier)
**Then** P99 < 100µs is verified per NFR-Perf-3
**And** a corpus of 1000 capability calls shows 100% mediation (FR4 floor)

### Story 1b.3: Sandbox Tier T0/T1/T2 Enforcement + Per-Spirit Resource Caps

As an operator,
I want sandbox tiers T0 (trusted local) / T1 (process isolation + UID separation) / T2 (Linux Landlock+seccomp; macOS Seatbelt; Windows restricted-token) enforced per-Spirit with strictest-of-(manifest, trust-tier, operator-policy) floor, AND per-Spirit resource caps enforced via cgroups v2 / setrlimit / Job Object,
So that a `public-untrusted` Spirit declaring T0 is forced to T2 — sandbox enforcement cannot be downgraded by a Spirit's manifest claim.

**Acceptance Criteria:**

**Given** a Spirit manifest declaring a sandbox tier
**When** the Security Manager admits the Spirit
**Then** the effective tier is the strictest of (manifest, trust-tier, operator-policy)
**And** a `public-untrusted` Spirit declaring T0 is forced to T2 regardless of manifest
**And** the effective tier is journaled to the Lifecycle Journal

**Given** a Spirit running under T2 on Linux
**When** the Spirit attempts a syscall outside its declared capability scope
**Then** Landlock + seccomp blocks the syscall at the kernel boundary
**And** the block is recorded in the Transparency Log via `cap-audit`

**Given** a Spirit running under T2 on macOS
**When** the Spirit attempts a forbidden operation
**Then** the Seatbelt `.sbpl` profile blocks the operation
**And** the block is journaled

**Given** a Spirit running under T2 on Windows
**When** the Spirit attempts an operation outside its restricted token
**Then** the Windows access check fails
**And** the failure is journaled

**Given** per-Spirit resource caps declared in manifest `[resources]` (FR6 basic)
**When** the kernel spawns the Spirit
**Then** cgroups v2 (Linux) / setrlimit (macOS) / Job Object (Windows) enforces CPU and memory caps OS-natively, not via Tokio cooperation
**And** the strictest-of (manifest, operator-policy) applies

### Story 1b.4: Freeze the ComplianceClaim Schema and Wire the Inference Port + IAC Telemetry

As a kernel observability lead,
I want the ComplianceClaim schema FROZEN (after the E0 adversarial review report is signed off), the Inference Port operational with the Anthropic provider, AND the IAC round-trip telemetry metrics binding from v0.1 (`iac_rt_duration_us` histogram with documented buckets, `iac_rt_inflight` gauge, `iac_rt_errors_total` counter),
So that the substrate's compliance posture is ABI-stable, every Spirit obtains LLM inference exclusively through the kernel (FR47), and operators can observe runtime SLOs from day one.

**Acceptance Criteria:**

**Given** the E0 adversarial-review report for the ComplianceClaim schema is signed off
**When** the schema is frozen in `maos-spirit-abi/src/compliance.rs`
**Then** the schema's `ABI_VERSION` is committed
**And** the structural validator (~200 LOC in `maos-kernel-core/compliance/`) accepts well-formed claims with 100% schema validation and 100% emit-rate
**And** any future schema change to required fields, removed fields, renames, type-changes, or `Verdict`/`PrincipleRef`/`EvidenceKind` enum reorderings triggers an ABI break (`ABI_VERSION` bump) per §8.5

**Given** a Spirit attempts to import a vendor LLM SDK directly (e.g., `anthropic` crate)
**When** the kernel-API surface invariant runs (Story 0.2) or the manifest-time capability check runs
**Then** the build fails with `FR47 violation: Spirit must obtain inference via kernel Inference Port`

**Given** the Inference Port with Anthropic provider configured
**When** a Spirit invokes `kernel.infer(prompt, options)`
**Then** the call is routed through `maos-providers` to Anthropic
**And** the call is recorded in the Transparency Log with provider attribution
**And** the response is returned to the Spirit without exposing provider-specific SDK types

**Given** the IAC telemetry binding-v0.1
**When** any kernel service call traverses the IAC pipeline
**Then** `iac_rt_duration_us` is observed with labels `service ∈ {security, memory, iac, capability, spirit_scheduler}` and `outcome ∈ {ok, err, timeout}`
**And** the histogram buckets are exactly `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]` (anchored on 1500µs SLO)
**And** `iac_rt_inflight` and `iac_rt_errors_total` are exposed as Prometheus-compatible metrics

### Story 1b.5a: Ship hello-Spirit Reference Binary and Hit NFR-Onb-2 5-Minute Evaluator Path

As a first-time evaluator,
I want to clone the MAOS repo and reach a structured hello-Spirit response within 5 minutes on a fresh OS install,
So that NFR-Onb-2 (the v0.1 ship gate that says "trust the substrate in 5 minutes") is mechanically reproducible — not aspirational.

**Acceptance Criteria:**

**Given** `spirits/hello-spirit/src/lib.rs` compiled against the frozen `maos-spirit-abi` crate from Story 1a.1
**When** `cargo test -p maos-spirit-hello -- test_manifest_validates` runs
**Then** the manifest at `spirits/hello-spirit/manifest.toml` parses without error against the schema published by Story 1b.4
**And** the manifest declares non-empty `capability_scope`, `expected_halt_tags`, and `transparency_log_url` fields
**And** the test is wired into the CI matrix

**Given** `maosctl run hello-spirit` on a clean Linux or macOS install (no prior cargo cache, no MAOS state directories)
**When** an operator times the path from `git clone` to first structured response on stdout
**Then** elapsed wall-clock is ≤5 minutes (NFR-Onb-2)
**And** the response JSON contains keys `introduction`, `capability_scope`, `halt_tags`, `transparency_log`
**And** `tests/integration/onb_nfr2_timing.sh` reproduces this measurement in CI and fails if elapsed > 300s

**Given** `spirits/hello-spirit/src/lib.rs` invoking the Inference Port from Story 1b.4
**When** the latency benchmark `crates/maos-bench/benches/hello_spirit_p95.rs` (using `criterion`) runs over 20 consecutive calls
**Then** P95 latency is ≤400ms (J0 budget per §13.1)
**And** the bench is CI-gated via `cargo bench --bench hello_spirit_p95 -- --test` in fail-on-regress mode

**Given** the ABI freeze in Story 1a.1
**When** `cargo build -p maos-spirit-hello --locked` runs
**Then** the build succeeds with zero `unsafe` blocks outside `crates/maos-kernel-core/`
**And** the stripped Spirit binary is ≤10MB on Linux x86_64

### Story 1b.5b: maosctl audit query + FR4 100%-Mediation Mechanical Verification

As an evaluator who has just seen hello-Spirit respond,
I want `maosctl audit query --spirit hello-spirit` to enumerate every external call the Spirit made with its issuing capability token, Spirit-PID, and boot-nonce, AND a 1000-call fixture proving 100% mediation,
So that FR4 (every external call mediated) is mechanically verified — not asserted in a README.

**Acceptance Criteria:**

**Given** `crates/maos-bin/src/cmd/audit.rs` implementing `maosctl audit query --spirit <name>`
**When** the command runs against a hello-Spirit session log at `~/.local/share/maos/audit/<session-id>.jsonl`
**Then** stdout is NDJSON where each line contains `{ "call_id", "capability_token", "spirit_pid", "boot_nonce", "call_type", "timestamp_ns" }`
**And** missing any of the five fields fails with exit code 2
**And** the schema is enforced by `crates/maos-audit/tests/query_schema_test.rs`

**Given** the FR4 verification fixture at `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl` (1000 pre-recorded audit entries generated by `scripts/gen_hello_spirit_fixture.sh`)
**When** `cargo test -p maos-audit -- test_fr4_full_mediation` runs
**Then** 1000/1000 entries carry non-null `capability_token`, `spirit_pid`, and `boot_nonce`
**And** the test fails fast on the first missing field (no silent pass)
**And** the fixture-generation script is checked into the repo and reproducible

**Given** `maosctl audit query --spirit hello-spirit --format plain`
**When** `TERM=dumb` or `NO_COLOR=1` is set in the environment
**Then** stdout contains no ANSI escape codes
**And** the assertion is wired in `crates/maos-bin/tests/audit_no_color_test.rs` by checking `stdout.bytes().filter(|b| *b == 0x1b).count() == 0`

**Given** the Transparency Log adapter in `crates/maos-audit/src/journal.rs` from Story 1b.1
**When** `maosctl audit query` joins capability-token data from `cap-audit` (Story 1b.2) with frame data from the Transparency Log
**Then** every call surfaces its issuing token + Spirit-PID + boot-nonce
**And** the join is documented as the canonical FR4 verification path in `crates/maos-audit/README.md`

### Story 1b.5c: maosctl v0.1 Lifecycle Subcommands + Accessibility Flags

As an operator scripting MAOS into a deployment pipeline,
I want `maosctl install`, `start`, `stop`, `unload`, `run` working reliably on Linux + macOS with `--plain` / `NO_COLOR` / `TERM=dumb` honored across every subcommand AND manifest field test coverage ≥3 cases per field (NFR-Test-13),
So that maosctl integrates with CI tooling and screen readers without ad-hoc workarounds.

**Acceptance Criteria:**

**Given** the five maosctl v0.1 subcommands (`install`, `start`, `stop`, `unload`, `run`) shipped in `crates/maos-bin/src/cmd/`
**When** each command runs against hello-Spirit on a fresh install
**Then** each subcommand exits 0 with the expected side-effect (lifecycle journal entry, Transparency Log row, or process state change)
**And** smoke-tested in `tests/integration/maosctl_smoke.sh` (required CI gate before v0.1 ships)

**Given** any maosctl subcommand
**When** invoked with `--plain` or with `NO_COLOR=1` or with `TERM=dumb`
**Then** stdout/stderr contain no ANSI escape codes
**And** the assertion runs in `crates/maos-bin/tests/accessibility_test.rs` for all five subcommands

**Given** the manifest field test coverage gate (NFR-Test-13)
**When** the kernel-side manifest parser is exercised against `crates/maos-kernel-core/manifest/tests/fixtures/`
**Then** every manifest field has ≥3 fixture cases (well-formed, malformed-rejected, edge-case)
**And** CI enforces ≥3 cases per field via the coverage-matrix gate (Story 0.3)
**And** missing-fixture-coverage on any field is a build break

**Given** an integration test running the full v0.1 evaluator path (Story 1b.5a + 1b.5b + 1b.5c)
**When** `tests/integration/v01_evaluator_path.sh` runs end-to-end
**Then** install completes, hello-Spirit responds, audit query enumerates calls, FR4 fixture passes, and accessibility flags work — all green in one sequential CI job
**And** this composite test gates the v0.1 release tag

---

### Epic 2: Spirit ABI + Developer SDK + Boundary Contracts (v0.1 → v0.3)

**Goal:** A Spirit author at 9pm Tuesday clones a template, implements `on_idle`, runs `spirit-test` harness locally, and ships a binary Spirit without ever touching kernel internals. NFR-Onb-1 v0.3 gate prerequisites land here.

**Owns:**
- Full Spirit ABI contract crate (`maos-spirit-abi` extended with full vtable + lifecycle hook signatures).
- `maos-spirit-sdk` with `#[spirit]` proc-macro and Spirit-author helpers.
- **`cargo xtask check-service-boundary` P1–P4 FULL implementation** (boundary enforcer against real Spirit ABI types — resolves circular dependency from E1a stub).
- Thin `cargo generate maos-spirit` template (Rust only at v0.5; per-language at E7) — **enough for NFR-Onb-1 v0.3 gate**.
- `spirit-test` SDK seed: local runner without kernel + manifest self-check + class-specific regression corpus skeleton.
- Spirit ABI lifecycle hook signatures: `on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`.
- `output_shape` declaration skeleton (full fail-loud `output_shape_version` mismatch in E7).
- Spirit boundary contract test cases (~20 cases asserting FR17 + FR58 Spirit-side boundary).
- NFR-Sec-14 framework hooks (cross-Spirit memory isolation test scaffolding; corpus 200 authored in E4).
- NFR-Test-6 LCAS framework + **clearly-decidable bucket** (70 of 210 items authored at v0.3).

**FRs covered:** FR33 (thin cargo-generate slice — full per-language in E7), FR34 (spirit-test SDK seed — full SDK with assertion macros in E7), FR40 (output_shape_version skeleton — full fail-loud in E7), FR55 (lifecycle hook ABI signatures — runtime firing in E5), Spirit-side of FR17 (Spirit's manifest capability + halt declaration).

**Key NFRs:** **NFR-Onb-1 prerequisites** (cargo-generate template + local runner + ≥1 example Spirit with passing CI — full gate execution at E7 against Butler in E8), NFR-Test-3 SDK coverage ≥80% (validated by external-author trial in 5+ third-party Spirits — full at E7).

**Corpora authored in E2:**
- Spirit boundary contract cases ~20 (FR17 + FR58 boundary assertions).
- LCAS clearly-decidable bucket 70 items.

**Acceptance demo:** External developer clones `spirit-template`, implements `on_idle`, runs `cargo test` (which invokes spirit-test SDK harness), gets passing report — **without** reading kernel internals.

#### Stories

### Story 2.1: Ship the Full Spirit ABI with `#[spirit]` Proc-Macro and 11 Lifecycle Hooks

As a Spirit author,
I want the full Spirit ABI contract crate with a `#[spirit]` proc-macro that derives the Spirit boilerplate plus all 11 lifecycle hook signatures (`on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`),
So that I can implement a Spirit by writing only the hooks I need without re-deriving the trait machinery for every Spirit.

**Acceptance Criteria:**

**Given** the `maos-spirit-sdk` crate
**When** a Spirit author writes `#[spirit] impl MySpirit { fn on_idle(&self, ctx: &mut Ctx) {...} }`
**Then** the proc-macro derives the Spirit trait implementation, registers manifest entries, and wires the ABI vtable
**And** the resulting binary is `#[no_std]`-compatible at the ABI boundary

**Given** the 11 lifecycle hook signatures
**When** the Spirit ABI is exported
**Then** every hook is declared in `maos-spirit-abi` with a stable signature carrying a `CancellationToken` for cancellation discipline
**And** each hook declares the resource budget envelope per manifest `[budget]`
**And** the kernel calls only hooks the Spirit has declared in its manifest

**Given** the `output_shape` declaration skeleton
**When** a Spirit declares `[output_shape]` in its manifest
**Then** the kernel parses the declaration into a shape predicate (full fail-loud enforcement in E7)
**And** the parser rejects malformed shape declarations at admission

**Given** Spirit-side capability declarations (FR17 Spirit half)
**When** a Spirit declares `[capabilities.required]` in its manifest
**Then** the kernel enforces these as the issued capability scope at admission
**And** mismatches between declared and observed capabilities surface as drift events (full drift detection in E9)

### Story 2.2: `xtask check-service-boundary` P1–P4 Full Implementation + Spirit-Boundary Invariant Cases

As an architectural-discipline maintainer,
I want the full P1–P4 four-property test enforced against real Spirit ABI types (resolving E1a's stub) plus ~20 spirit-boundary invariant test cases,
So that the kernel-API surface invariant (NFR-Test-2) is mechanically enforced from v0.3 onward and any new kernel function landing outside the permitted computational classes is a build-break.

**Acceptance Criteria:**

**Given** the full `cargo xtask check-service-boundary` against real Spirit ABI types
**When** the xtask runs on every PR
**Then** P1 (single supervising owner per service) is enforced via supervision-tree static analysis
**And** P2 (ports not adapters at service boundary) is enforced via trait-direction lint
**And** P3 (state ownership behind `Arc<DashMap>`/`RwLock`/atomic) is enforced via type analysis
**And** P4 (audit-chain integrity at service boundary) is enforced via call-graph reachability — every external call reaches Capability Registry before exit

**Given** build-time reflection over `kernel::api::*` (Rust `syn` static analyzer)
**When** the analyzer classifies a function
**Then** the classification is decidable for the permitted subset (allowlist-based predicate definitions; no theorem prover)
**And** functions falling outside `{universal-arithmetic, data-movement, supervision}` are build-break

**Given** the spirit-boundary invariant test cases
**When** the test suite runs
**Then** ≥20 cases exercise the FR17/FR58 boundary (Spirit-side capability declaration, ComplianceClaim emit, output_shape conformance)
**And** the cases are registered in `coverage-matrix.yaml` per Story 0.3

### Story 2.3: Thin `cargo-generate` Template + Local Runner (NFR-Onb-1 v0.3 Prerequisite)

As a Spirit author working in a 9pm-Tuesday window,
I want a thin `cargo generate maos-spirit` Rust template that produces a compilable Spirit + a local runner that invokes lifecycle hooks without a kernel instance,
So that I can build and test a Spirit on my laptop within 30 minutes without learning kernel internals — meeting the v0.3 NFR-Onb-1 gate prerequisites.

**Acceptance Criteria:**

**Given** an installed `cargo-generate` tool
**When** the author runs `cargo generate maos-spirit --name my-spirit`
**Then** the template scaffolds a `my-spirit` crate with a working `on_idle` hook, a TOML manifest, and a passing `cargo test`
**And** the scaffold uses the `#[spirit]` proc-macro from Story 2.1
**And** the README documents the 30-minute first-Spirit path

**Given** the local runner shipped in `maos-spirit-sdk`
**When** the author runs `cargo test` against their Spirit
**Then** the runner invokes lifecycle hooks via the ABI without spinning up a real kernel
**And** the runner emits IAC frames into a mock bus that the test asserts against

**Given** the v0.3 NFR-Onb-1 gate prerequisites
**When** the gate runs (E7 Story 7.5 owns execution)
**Then** cargo-generate template + local runner + ≥1 example Spirit with passing CI are all present
**And** the Butler reference Spirit (E8 Story 8.1) uses this exact template

**Given** the `30-Min First Spirit` recruitment criteria
**When** a participant clones the template and follows the README
**Then** they reach a passing `cargo test` within median ≤45 min, p95 ≤90 min (per NFR-Onb-1)

### Story 2.4: Seed the spirit-test SDK with LCAS Framework and Cross-Spirit Isolation Hooks

As a test architect,
I want the spirit-test SDK seed (lifecycle hooks + IAC frame I/O + halt resolution + manifest self-check + class-specific regression-corpus skeleton) AND the LCAS (Long-context Ambiguity Stress) framework + clearly-decidable bucket (70 of 210 items) AND the cross-Spirit memory isolation framework hooks,
So that Story 4.5's 200-corpus authoring (NFR-Sec-14) and Story 8.x reference-Spirit acceptance tests have a working harness from v0.3 — not retrofitted at v1.0.

**Acceptance Criteria:**

**Given** the spirit-test SDK seed
**When** a Spirit author calls `spirit_test::run(&my_spirit, &fixture)`
**Then** the harness invokes every declared lifecycle hook with the fixture
**And** the harness verifies IAC frame I/O against the fixture's expected frames
**And** the harness exercises halt resolution under all three resolution kinds
**And** the harness runs the manifest self-check (well-formed/malformed/edge-case per NFR-Test-13)

**Given** the LCAS framework
**When** corpus authoring begins
**Then** the 70-item clearly-decidable bucket is committed to `tests/corpora/lcas-v0.3.jsonl`
**And** the remaining 140 items (genuinely-ambiguous + adversarially-misleading) are explicitly deferred to E2 + E7/E8 (require A2A scenarios from E6 to be valid)
**And** each item carries gold labels for halt-recall/precision measurement

**Given** the NFR-Sec-14 cross-Spirit memory isolation framework hooks
**When** a future test (E4 Story 4.5) attempts an adversarial cross-Spirit read
**Then** the framework provides hook points to inject Spirit-A's attempt and observe Spirit-B's state
**And** the framework is registered in `coverage-matrix.yaml` with a `valid_until` date

**Given** all of the above
**When** the SDK seed is published
**Then** external authors can extend the harness for their own Spirit classes
**And** the SDK seed counts toward NFR-Test-3's ≥80% coverage floor (full validation at E7)

---

### Epic 3: Director's Surface — IAC Bus, Task Assignment & Posture Control (v0.3 → v0.8)

**Goal:** The director — at 2:47am, on mobile, half-asleep — gets a halt notification, resolves it in three taps, and can revoke any active capability token in under two seconds. The kernel-side log-composition primitives that Butler/Researcher/Orchestrator use to ship the morning digest live here; the digest implementation itself lives in E8.

**Owns:**
- Same-Host IAC bus basic routing (`tokio::sync::mpsc` + `tokio::sync::broadcast` channels addressable by SpiritId; bounded queues; backpressure via Spirit Scheduler) — modeled on codex's `Mailbox`.
- `task.assign` IAC frame (natural-language goal + scope + success criteria + posture preferences).
- Posture management: `autonomous-with-halt`, `assistive`, `cautious` — runtime shifts via authenticated control plane.
- Halt-policy schema (extension to ADR-013) — per-Spirit per-tag halt-recall vs halt-precision preference.
- Orchestrator instruction buffering (FR20, v0.8 wedge): Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives; processes queued instructions at safe sequence points between task completions, never preempting in-flight delegations.
- Instant pause/resume/revoke (FR51, P99 ≤2s): interrupting in-flight autonomous actions with bounded time; preserving state across pause/resume; recalling Orchestrator-buffered actions; revoking any active capability token with in-flight ops failing-safe within bounded time.
- Decision-context refs I12 (every `decision.*` frame carries `working_memory_digest_refs` for retrospective audit).
- Kernel log-composition primitives for FR17 (Spirit-side digest implementation in E8).
- Notification surface dispatch (terminal / ACP editor surface / mobile push).
- Approval Decision Log I4 surface (full intent + decision + reasoning chain).

**Halt protocol status:** **Halt resolution UX surface lives here** — director receives notification, sees three resolution choices (provided_context / accepted_halt / authorized_override), submits resolution. **Halt mechanism + I14 invariant + halt-receipt + halt-recall/precision floors OWNED BY E4.** Authorized override adds `override_marker` to subsequent output for `output_shape` predicates.

**FRs covered:** FR14, FR16, FR17 (kernel primitives only), FR18, FR19, FR20, FR22 (basic routing — full features in E6), FR51, FR24 partial (posture enforcement at director surface; intent provenance I13 in E6).

**Key NFRs:** NFR-Perf-4 (posture-shift propagation P99 ≤2s, P99.9 ≤5s in 1000-shift corpus), NFR-Aud-5 (right-to-explanation via I12: 100% of `decision.*` frames carry `working_memory_digest_refs`), NFR-Obs-3 v0.3 (Butler-narrow per-Spirit telemetry), NFR-Obs-5 (Approval Decision Log distinct from Transparency Log).

**Acceptance demo:** Director uses `maosctl posture <spirit> --shift autonomous-with-halt`; posture propagates within 2s P99; staged epistemic halt triggers mobile push notification; director resolves via three-tap flow; full reasoning chain journaled.

#### Stories

### Story 3.1: Route `task.assign` Frames Over the IAC Bus with Notification Surface Dispatch

As a director,
I want to send a natural-language `task.assign` IAC frame to a Spirit via terminal / ACP editor / mobile push and have the kernel route it through the IAC bus with bounded queues and log-before-deliver guarantees,
So that the director's first interaction with a Spirit is mediated, journaled, and visible across all three input surfaces.

**Acceptance Criteria:**

**Given** the IAC bus basic routing on a single Host
**When** a `task.assign` frame is dispatched
**Then** routing uses `tokio::sync::mpsc` + `tokio::sync::broadcast` channels addressable by SpiritId
**And** queues are bounded with backpressure via the Spirit Scheduler
**And** the frame is written to the Transparency Log before delivery to the recipient mailbox (I2)

**Given** a `task.assign` frame from the director
**When** the frame is constructed
**Then** the frame carries `(goal, scope, success_criteria, posture_preferences)` per FR14
**And** the frame is authenticated via the control-plane session

**Given** the notification surface dispatch
**When** a kernel event requires director attention (halt, approval prompt, anomaly)
**Then** the kernel dispatches notifications across terminal / ACP editor / mobile push channels per the operator's configured preferences
**And** the dispatcher exposes hook points for Spirit-side gateway sub-modules (full gateway implementation in E6 Story 6.5)

**Given** the Approval Manager surface
**When** an approval prompt is required
**Then** the prompt routes through the same notification surface
**And** the prompt classification is one of the 6 approval classes (`readonly_scoped` / `readonly_search` / `mutating` / `exec_capable` / `control_plane` / `interactive`)
**And** every resolution lands in the Approval Decision Log (E1b Story 1b.1)

### Story 3.2: Manage Director Posture with a Halt-Policy Schema and Bounded Shift Propagation

As a director,
I want three runtime postures (`autonomous-with-halt`, `assistive`, `cautious`) with shifts propagating within 2s P99 across all of a Spirit's in-flight capability decisions, AND a halt-policy schema that lets me tune halt-recall vs halt-precision per Spirit per tag,
So that I can dial Spirit autonomy up or down in real time without restarting the Spirit.

**Acceptance Criteria:**

**Given** a Spirit running under one of the three postures
**When** the director runs `maosctl posture <spirit> --shift <new_posture>`
**Then** the shift is journaled to the Approval Decision Log
**And** subsequent capability-scope decisions reflect the new posture
**And** propagation latency is P99 ≤2s, P99.9 ≤5s in a 1000-shift corpus (NFR-Perf-4)

**Given** posture `autonomous-with-halt`
**When** a Spirit's `[epistemic_policy]` predicate fires
**Then** the Spirit halts (halt mechanism owned by E4)
**And** other actions proceed without prompting

**Given** posture `assistive` (every action prompts)
**When** any Spirit action triggers
**Then** the director receives an approval prompt before the action commits

**Given** posture `cautious` (auto-approve routine, prompt for novel)
**When** a Spirit action is classified as `mutating` or `exec_capable`
**Then** the director receives an approval prompt
**And** `readonly_scoped` / `readonly_search` actions auto-approve

**Given** the halt-policy schema (extension to ADR-013)
**When** the director sets per-Spirit per-tag halt-recall vs halt-precision preference
**Then** the kernel parses the preference into the Spirit's runtime `[epistemic_policy]` thresholds
**And** thresholds inform Story 4.2's predicate-firing decisions

### Story 3.3: Director's Halt Resolution UX + Decision Audit (I12)

As a director (at 2:47am, on mobile, half-asleep),
I want a three-tap halt resolution flow that surfaces a Spirit's halt with its reasoning chain AND requires me to choose exactly one of three documented resolution pathways (`provided_context` / `accepted_halt` / `authorized_override`),
So that the director's-surface metaphor is operationalized as a real UX path with full retrospective auditability (I12).

**Acceptance Criteria:**

**Given** a Spirit emits `epistemic.halt(payload)` via Story 4.1's `crates/maos-kernel-core/src/halt/mod.rs::invoke_halt`
**When** `crates/maos-director-surface/src/notification.rs::dispatch_halt(halt_id, payload)` runs
**Then** the notification surfaces on the director's configured channel (terminal / ACP via Story 5.5c / mobile push via Story 6.5 gateway sub-modules)
**And** the notification includes the structured halt payload (tag, value, threshold, policy_id, derived_from)
**And** the notification renders within the J0 director-surface budget on mobile (≤3 taps to resolution per `crates/maos-director-surface/src/halt_ui.rs::resolve_flow`)

**Given** the director chooses `provided_context` via `crates/maos-director-surface/src/halt_ui.rs::submit_resolution(halt_id, Resolution::ProvidedContext { text })`
**When** the resolution submits to `crates/maos-kernel-core/src/halt/resolver.rs::resolve` (Story 4.1's `HaltResolver` trait — production impl wires here; `MockHaltResolver` exists for E4 unit tests)
**Then** the Spirit resumes with the supplied context appended to its working memory via the Memory Manager from Story 4.3
**And** the resolution is journaled to `crates/maos-audit/src/journal.rs::write_halt_resolution_entry` with full reasoning chain

**Given** the director chooses `accepted_halt`
**When** the resolution submits via the same `submit_resolution` path
**Then** the Spirit terminates the in-flight task via Story 5.1's lifecycle path and writes a halt receipt (Story 5.3 / NFR-Rel-11)
**And** the task originator receives `task.orphaned` per Story 5.3's FR12 path

**Given** the director chooses `authorized_override`
**When** the resolution submits with operator-policy reference
**Then** the Spirit resumes WITHOUT the halt condition resolved
**And** the kernel attaches a mandatory `OutputMarker::Override` to subsequent output for `output_shape` predicates (Story 4.2's predicate enforcement)
**And** the override is journaled with director identity and operator-policy reference

**Given** every `decision.*` IAC frame emitted by any Spirit
**When** the frame is processed by `crates/maos-kernel-core/src/iac/decision_logger.rs`
**Then** the frame carries `working_memory_digest_refs` (I12) — refs computed from Story 4.3's principal namespace
**And** 100% of `decision.*` frames carry the references (NFR-Aud-5, right-to-explanation)
**And** post-hoc reconstruction is testable in `crates/maos-audit/tests/i12_decision_audit_test.rs`

### Story 3.4: Buffer Orchestrator Instructions and Honor Director Pause/Resume/Revoke (P99 ≤2s)

As a director driving an Orchestrator Spirit overnight,
I want to buffer multiple instructions to the Orchestrator at safe sequence points without preempting in-flight delegations, AND to be able to instantly pause / resume / revoke ANY active Spirit with P99 ≤2s including in-flight capability tokens,
So that the founder-loop wedge demo (v0.8) actually works — and I retain god-mode control over the Spirit team without race conditions.

**Acceptance Criteria:**

**Given** an Orchestrator Spirit using kernel checkpoint/resume primitives
**When** the director queues multiple instructions via `maosctl orchestrator queue <instruction>`
**Then** the Orchestrator processes queued instructions at safe sequence points between task completions
**And** queued instructions never preempt in-flight delegations to Worker Spirits (FR20)

**Given** the director invokes `maosctl pause <spirit>` on any active Spirit
**When** the pause command dispatches
**Then** the Spirit's in-flight autonomous actions are interrupted with bounded time
**And** the pause P99 is ≤2s (FR51 a)
**And** Spirit state is preserved across pause/resume without reload (FR51 b)

**Given** the director invokes `maosctl resume <spirit>`
**When** the resume command dispatches
**Then** Orchestrator-buffered pending actions are recalled per FR20 (FR51 c)
**And** the Spirit continues from its preserved state

**Given** the director invokes `maosctl revoke-token <token-id>`
**When** the revocation dispatches
**Then** the active capability token is invalidated
**And** in-flight operations using that token fail-safe within bounded time (FR51 d)
**And** the revocation is journaled with director identity and reason per FR42 audit
**And** revocation propagation is ≤5s p99 under 10⁴ concurrent capability-token validations (NFR-Rel-9, full validation in E5 Story 5.4)

**Given** the kernel log-composition primitives for FR17
**When** a digest-shipping Spirit (Butler v0.3 / Researcher v0.5 / Orchestrator v0.8+) queries kernel primitives
**Then** the primitives expose ranged log-recall over Transparency Log + Approval Decision Log + Lifecycle Journal
**And** the Spirit-side morning digest implementation (E8 Story 8.1 / 8.2 / 8.4) consumes these primitives without re-implementing log access

---

### Epic 4: Halt Protocol + Memory Substrate + Cognition Primitives (v0.3 → v1.0) — **SINGLE HALT OWNER**

**Goal:** The kernel performs ONLY universal-arithmetic comparisons. Spirit author declares cognitive policies via four predicates over tagged scalars; kernel triggers halts when predicates fire; every distillation is auditable end-to-end via I11+I12+I13. Cross-Spirit memory isolation is mechanically provable. **Halt protocol — schema types in E1a, mechanism + I14 invariant + halt-receipt + recall/precision floors OWNED HERE.**

**Owns:**
- Halt protocol mechanism (ADR-019 + ADR-022): three resolution kinds (`provided_context`, `accepted_halt`, `authorized_override`); `epistemic.halt(payload)` invocation; halt-receipt production rate ≥99.9% on every Spirit termination planned or unplanned.
- Tagged-scalar slot in Capability Registry (ADR-022): `working_memory.set_scalar(tag, value, derived_from)`.
- Four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`) — kernel performs NO Spirit-specific cognitive computation (no variance, entropy, EFE, KL, ensemble disagreement, derivatives, statistical tests — Spirit computes those itself per §4.0.7).
- Three memory tiers (Memory Manager, I5-enforced): `private` (per-Spirit `Arc<RwLock<HashMap>>` + per-Spirit-namespaced filesystem); `shared` (Host-wide SQLite-backed kv with namespace prefix per writer); `collective` scaffold (full Loom-lite Postgres+pgvector implementation in E10 at v1.5).
- Principal Memory Namespace ADR-026 full implementation: `principal:<principal_id>:<schema>` typed namespace within private tier with subject-access query, right-to-be-forgotten, redaction-on-export.
- `log.recall(filter, limit, cursor)` + `log.fetch(frame_id)` (ADR-013) — participant-scoped with A2A consent envelope honoring.
- Distillates with kernel-enforced I11 audit chain: mandatory `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage`. Distillation work itself (selection, summarization) is Spirit-authored.
- Per-tag `[epistemic_policy]` parser; kernel triggers halts when predicates fire and journals halt reason with structured payload (`tag`, `value`, `threshold`, `policy_id`, `derived_from`).
- Spirit self-telemetry within principal namespace (FR56): success/failure counts, latency distributions, halt-recall events, distillation outcomes — read without per-read operator admission.
- `scalar.tap` channel (ADR-035, binding-v0.5): dedicated read-only stream from Capability Registry's tagged-scalar slot; every `set_scalar` write emits `(spirit_id, tag, value, timestamp)`.
- Hot-Swap Coordinator I14 enforcement check (`halt_set` validation before swap; `EHaltContinuityViolation` if Spirit-author hasn't declared `halt_protocol_compatibility = N`) — **halt-continuity runtime path in E5; halt schema verified here**.
- Cross-Spirit memory isolation 200-corpus authoring + execution (NFR-Sec-14: 8 categories — namespace enumeration / working-memory read-across / decision-frame observation / halt-signal observation / transparency-log cross-read / working-memory-digest cross-read / capability-token forgery cross-Spirit / sandbox-escape lateral).

**FRs covered:** FR15 (halt resolution mechanism — E3 owns UX surface), FR27, FR28, FR29, FR30, FR31, FR32, FR56.

**Key NFRs:** NFR-Test-4 (halt-recall ≥0.7, halt-precision ≥0.85 per Spirit class on bmad-eval — full gate against E8 Spirits), NFR-Test-6 LCAS framework completion (full corpus across E2 + E7), NFR-Sec-14 (cross-Spirit memory iso 200-corpus, P0 ship-block), NFR-Aud-7 (5-metric distillation gate: digest-recall ≥0.90 / digest-faithfulness ≥0.98 unflagged contradictions / digest-hedge-preservation ≥0.95 / digest-traceability = 100% kernel-enforced via I11 / digest-secret-leakage = 0%), NFR-Aud-14 (intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain).

**Corpora authored in E4:**
- HSIS partial (Researcher + Observer Spirit class corpora — 50+50 = 100 of 300 total; remaining 200 in E5).
- Cross-Spirit memory isolation 200-corpus.
- Five-metric distillation eval corpus ~200 annotated digests.

**Acceptance demo:** Spirit declares `[epistemic_policy]` with predicate `on_value_above(tag="uncertainty", threshold=0.8)`; Spirit writes scalar above threshold; kernel emits halt with structured reason; Spirit-A cannot enumerate or read Spirit-B's principal namespace under any of 200 adversarial scenarios.

#### Stories

### Story 4.1: Halt Protocol Mechanism — Three Resolution Kinds + Halt-Receipt 99.9% (SINGLE HALT OWNER)

As the substrate's halt-protocol owner,
I want the halt mechanism (ADR-019 + ADR-022) to be the SINGLE owner of: halt invocation primitive, the three resolution kinds, the I14 halt-continuity invariant, halt-receipt production, and the halt-recall/precision floor measurement — while E1a holds halt schema types only, E3 holds halt resolution UX only, and E5 holds halt-continuity-across-hot-swap only,
So that halt logic never fragments into multiple owners and every Spirit termination (planned or unplanned) produces an audit-grade receipt.

**Acceptance Criteria:**

**Given** `crates/maos-kernel-core/src/halt/mod.rs::invoke_halt(payload: HaltPayload) -> HaltReceipt`
**When** a Spirit calls `epistemic.halt(payload)` from its `[epistemic_policy]` rules
**Then** `maos-kernel-core` journals a `HaltEntry` to `crates/maos-audit/src/journal.rs::write_halt_entry()` with fields `{ tag, value, threshold, policy_id, derived_from, spirit_pid, boot_nonce, timestamp_ns }`
**And** the kernel suspends the Spirit thread and enters `HaltState::PendingResolution`
**And** this is unit-tested in `crates/maos-kernel-core/tests/halt_invoke_test.rs` against `MockHaltResolver` (no integration dependency on E3 Story 3.3 at this AC's gate)

**Given** the `HaltResolver` trait defined in `crates/maos-kernel-core/src/halt/resolver.rs` with `MockHaltResolver` for unit isolation
**When** unit tests exercise the three resolution kinds (`provided_context`, `accepted_halt`, `authorized_override`)
**Then** `authorized_override` appends `OutputMarker::Override` to the Spirit's output queue (consumed by `output_shape` predicates from Story 4.2)
**And** `accepted_halt` transitions the Spirit to `HaltState::Terminated` and emits `task.orphaned` per FR12
**And** `provided_context` resumes with the supplied context appended to working memory
**And** all three paths produce a `HaltReceipt` with resolution fields populated
**And** a comment block in `resolver.rs` states: "Integration with E3 Story 3.3 UX surface wires here — see `crates/maos-director-surface/src/halt_ui.rs`." (the actual UX integration test is owned by Story 3.3, not this story)

**Given** any termination path in `crates/maos-kernel-core/src/lifecycle/` (planned unload, unplanned crash, or halt-rejection)
**When** `terminate_spirit()` is called
**Then** a `HaltReceipt` is written to `crates/maos-audit/src/journal.rs` before the OS process exits
**And** the receipt production rate is ≥99.9% measured against the 1000-termination corpus at `crates/maos-eval/fixtures/termination-corpus-v0/`
**And** `cargo test -p maos-kernel-core -- test_halt_receipt_production_rate` asserts ≥999/1000 receipts present

**Given** the v0.3 provisional halt corpus at `crates/maos-eval/fixtures/halt-corpus-v0/` (N=50 hand-authored synthetic scenarios — round-3 fix per Amelia's defect finding; the E8 reference-Spirit corpus replaces this at v1.0)
**When** `cargo test -p maos-eval -- test_halt_recall_floor` runs against the synthetic corpus
**Then** halt-recall is ≥0.7 across the 50 scenarios
**And** halt-precision is ≥0.85
**And** the predicate-firing recall floor is ≥0.85 (FR32)
**And** the test output names any failing scenario by file path for triage
**And** the corpus is tagged `synthetic-v0` to distinguish from E8 reference corpora at v1.0
**And** **intra-E4 ordering: Story 4.5 (HSIS corpus 100 scenarios) MUST close before Story 4.1 AC closes at v1.0** to provide the production-grade corpus replacing `synthetic-v0`

**Given** the halt-continuity-across-hot-swap I14 invariant
**When** Hot-Swap Coordinator (E5 Story 5.2) calls `validate_halt_set(spirit_manifest)` in `crates/maos-kernel-core/src/halt/mod.rs`
**Then** the function returns `Err(EHaltContinuityViolation { schema_mismatch: ... })` if the incoming Spirit hasn't declared `halt_protocol_compatibility = N` matching the predecessor's halt schema version
**And** the integration test that exercises this end-to-end lives in `crates/maos-lifecycle/tests/hot_swap_halt_continuity_test.rs` and is owned by Story 5.2 (not this story)
**And** the unit test for `validate_halt_set` returning the typed error lives in `crates/maos-kernel-core/tests/halt_continuity_test.rs` and is owned here

### Story 4.2: Implement the Tagged-Scalar Slot with Four Universal-Arithmetic Predicates

As a Spirit author,
I want to write tagged scalars via `working_memory.set_scalar(tag, value, derived_from)` AND declare per-tag `[epistemic_policy]` rules using the four universal-arithmetic predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`), AND have those writes streamed to subscribers via `scalar.tap`,
So that the kernel performs ONLY universal-arithmetic comparison — never variance, entropy, EFE, KL, or any Spirit-specific cognitive computation (§4.0.7).

**Acceptance Criteria:**

**Given** a Spirit calls `working_memory.set_scalar("uncertainty", 0.83, "derived_from_observation_42")`
**When** the kernel persists the scalar
**Then** the kernel records `(spirit_id, tag, value, derived_from, timestamp)` to the working-memory store
**And** the kernel does NOT interpret tag-specific semantics — only routes by tag identity
**And** the write emits to the `scalar.tap` channel (ADR-035, binding-v0.5) as `(spirit_id, tag, value, timestamp)`

**Given** a Spirit declares `[epistemic_policy] on_value_above(tag="uncertainty", threshold=0.8)`
**When** a `set_scalar` for `tag="uncertainty"` writes a value > 0.8
**Then** the kernel triggers `epistemic.halt(payload)` per Story 4.1
**And** the predicate evaluation involves only the four universal-arithmetic predicates (no statistical tests, no derivatives, no Spirit-specific math)

**Given** the kernel-API surface invariant test (Story 0.2)
**When** any kernel function involving Spirit-specific cognitive computation is added
**Then** the function is classified as `other` and the build hard-fails
**And** the test enforces §4.0.7's non-interpretability principle structurally

**Given** an Observer Spirit subscribed to `scalar.tap`
**When** any other Spirit writes a scalar
**Then** the Observer receives `(spirit_id, tag, value, timestamp)` in real time
**And** the Observer can detect pre-halt drift before the predicate fires (consumed by E8 Story 8.3)

**Given** the predicate-firing recall and precision floors
**When** measured against the bmad-eval corpus per Spirit class
**Then** predicate-firing recall is ≥0.85 per Spirit class (FR32)
**And** precision is ≥0.85 per Spirit class

### Story 4.3: Provide Three Memory Tiers with Principal Namespace and Spirit Self-Telemetry

As a Spirit author,
I want three memory tiers — `private` (per-Spirit), `shared` (Host-wide), and `collective` (scaffold; full Postgres+pgvector Loom-lite at v1.5) — AND a typed Principal Memory Namespace `principal:<principal_id>:<schema>` under the private tier with subject-access / right-to-be-forgotten / redaction-on-export contracts, AND the ability to read my own performance telemetry within that namespace without per-read operator admission,
So that I can build cognitive Spirits with proper memory hygiene and the substrate enforces I5 namespace isolation mechanically.

**Acceptance Criteria:**

**Given** the Memory Manager three tiers
**When** a Spirit calls `memory.write(tier, key, value)`
**Then** `private` writes go to per-Spirit `Arc<RwLock<HashMap>>` + per-Spirit-namespaced filesystem
**And** `shared` writes go to Host-wide SQLite-backed kv with namespace prefix per writer
**And** `collective` writes are rejected at v0.5 with a clear error (full Loom-lite at v1.5 via E10 Story 10.4)
**And** every write is namespace-enforced per I5 — Spirit-A cannot write outside its own namespace

**Given** a Spirit writes to `principal:alice@example.org:calendar`
**When** the kernel persists the entry
**Then** the entry lives in the Spirit's private tier under the `principal:` typed namespace (ADR-026)
**And** the entry is automatically eligible for subject-access query (E9 Story 9.1)
**And** the entry is eligible for GDPR Art. 17 forget cascade (E9 Story 9.2)
**And** the entry is eligible for redaction-on-export

**Given** a Spirit opts into the `memory.md` convention
**When** the Spirit writes `memory.md` to its private namespace
**Then** the kernel persists it like any other private-tier write
**And** the kernel does NOT interpret the contents (universal cohort convention)

**Given** a Spirit reads its own performance telemetry within its principal namespace (FR56)
**When** the Spirit calls `telemetry.self()`
**Then** the kernel returns success/failure counts, latency distributions, halt-recall events, distillation outcomes
**And** the call does NOT require per-read operator admission (Spirit's own data, Spirit reads it)
**And** the data is scoped to the Spirit's principal namespace per FR31

### Story 4.4: Enforce the I11 Audit Chain on Distillates with `log.recall` and the Five-Metric Gate

As a Spirit author building a Researcher-class Spirit,
I want `log.recall(filter, limit, cursor)` + `log.fetch(frame_id)` participant-scoped with A2A consent honoring, AND the ability to produce distillates with a kernel-enforced I11 audit chain (mandatory `source_log_ref`, `distillation_depth`, `intent_lineage`), AND a measurement harness for the five-metric distillation gate (NFR-Aud-7),
So that I can build memory-distilling Spirits whose every digest is provably traceable back to raw frames and measurable against the five quality metrics.

**Acceptance Criteria:**

**Given** a Spirit calls `log.recall(filter, limit, cursor)`
**When** the kernel processes the recall
**Then** the kernel scopes results to participant frames (Spirit was sender or receiver)
**And** the kernel honors A2A consent envelopes — frames marked private to a peer are excluded
**And** payloads fetch on-demand via `log.fetch(frame_id)` with the same scoping

**Given** a Spirit produces a distillate via Spirit-side LLM compression
**When** the Spirit writes the digest
**Then** the kernel enforces the I11 audit chain — the digest MUST include `source_log_ref` flattened to original raw frames, `distillation_depth`, and `intent_lineage`
**And** the kernel rejects digest writes missing any of the three with `EDigestAuditChainMissing`

**Given** the five-metric distillation gate harness
**When** a distillation-shipping Spirit's digests are measured against the eval corpus
**Then** digest-recall is ≥0.90 (NFR-Aud-7)
**And** digest-faithfulness is ≥0.98 unflagged contradictions
**And** digest-hedge-preservation is ≥0.95
**And** digest-traceability is 100% (kernel-enforced via I11)
**And** digest-secret-leakage is 0% (zero-tolerance)

**Given** the corpus tiers (NFR-Aud-8)
**When** the harness runs the N=100 per-commit slice
**Then** CI-width ≈ 0.124 is observed (sufficient for trend detection)
**And** the quarterly N=500 audit gives CI-width ≤ 0.05 at p=0.90 for digest-recall

### Story 4.5: Author the Cross-Spirit Isolation 200-Corpus and Enforce I14 Halt-Continuity in Hot-Swap

As the substrate's cross-Spirit-isolation guarantor,
I want a 200-scenario adversarial corpus (NFR-Sec-14) where Spirit-A actively attempts to enumerate / read / side-channel / timing-attack Spirit-B's substrate state, AND the Hot-Swap Coordinator's I14 enforcement check (validate `halt_set` before swap; reject with `EHaltContinuityViolation`), AND 100% intent-lineage propagation across re-emission (NFR-Aud-14),
So that the v1.0 hermes-tenant positioning sentence is defended by mechanical evidence, not asserted.

**Acceptance Criteria:**

**Given** the cross-Spirit memory isolation 200-corpus
**When** the corpus is authored and committed
**Then** the corpus covers 8 categories (≥25 scenarios per category): namespace enumeration / working-memory read-across / decision-frame observation / halt-signal observation / transparency-log cross-read / working-memory-digest cross-read / capability-token forgery cross-Spirit / sandbox-escape lateral
**And** each scenario has Spirit-A actively attacking and Spirit-B's expected state un-leaked

**Given** the corpus runs as a CI gate
**When** all 200 scenarios execute
**Then** isolation is maintained in 200/200 (NFR-Sec-14 floor)
**And** any leak is a P0 ship-block

**Given** a Hot-Swap operation (E5 Story 5.2)
**When** the Hot-Swap Coordinator validates `halt_set` against the manifest's `halt_protocol_compatibility = N` declaration
**Then** the swap proceeds if the schemas are compatible
**And** the swap is rejected with `EHaltContinuityViolation` if active halts would be orphaned by the schema change

**Given** any cross-Spirit IAC frame
**When** the frame is emitted or re-emitted
**Then** the frame carries unbroken `intent_lineage` chain back to the originating principal intent (NFR-Aud-14, I13)
**And** 100% of cross-Spirit frames carry the lineage
**And** missing lineage is rejected at the IAC bus with `EIntentLineageBroken`

---

### Epic 5: Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)

**Goal:** Operator runs `maosctl spirit upgrade butler --to 0.3.2` with hot-swap policy; all in-flight tokens, working-memory state, and active halts preserved across the swap. Subprocess Spirit crashes within 2s; tasks NACKed within 5s. Three providers run in CI by v0.5.

**Owns:**
- Full Spirit Scheduler lifecycle: `load`, `start`, `pause`, `resume`, `unload` via authenticated control plane (CLI, ACP editor surface, operator HTTP API).
- Hot-swap state-transfer wire format (ADR-017, binding-v0.3): CBOR-encoded payloads conforming to per-Spirit-class schema declared in manifest `[hot_swap].state_schema_uri + state_schema_version`. Same-major+additive = forward-compat; same-major+breaking = forbidden; cross-major requires migrator. Saga-style compensation: on `on_swap_out` failure restore predecessor; on `on_swap_in` failure discard successor; auto-revert within 30s on post-swap invariant violation.
- Cross-major migration (ADR-020): `migrate(predecessor_state) -> Result<successor_state, Error>` declared via `migrates_from` manifest field; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator declared.
- Crash detection ≤2s for SIGKILL corpus (≥99/100 floor); `task.orphaned` IAC frame within 5s with exit-cause journaled.
- Hung-Spirit detection: no-progress IAC for >30s emits `task.stalled` event; ≥48/50 reclassified within 60s on hang corpus.
- Silent-failure detection (NFR-Rel-4): `silent_failure_suspect` event when no progress IAC for >30s despite healthy heartbeats; ≥45/50 on adversarial zombie-heartbeat corpus.
- Signed Revocation List artifact (CRL): registry-pushed (kernel polls every 5min) + offline-import path. Running Spirit instances receive `SpiritRevoked` event and execute declared revocation policy (terminate-immediately / drain-then-terminate / quarantine).
- Spirit upgrade with declared migration policy (FR49): hot-swap with state preservation (default) / cold-swap with re-init / migrator-mediated cross-major.
- Dead-Spirit task disposition (FR50): manifest `[on_crash].action` — NACK / reassign-to-replica / escalate-to-operator.
- Halt-continuity-across-hot-swap (I14, FR53) — **DEPENDENT on E4's halt schema; runtime path here**. Hot-Swap Coordinator rejects swaps that would orphan halts unless Spirit-author has declared `halt_protocol_compatibility = true` for predecessor's halt schema.
- Full lifecycle triggers: `on_load`, `on_start`, `on_frame`, `on_idle` (Butler v0.3 anchor), `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate` — each carrying declared resource budgets per manifest.
- Cooperative + priority-weighted scheduling; OS-level CPU/memory budget enforcement via cgroups v2 / setrlimit / Job Objects.
- Sandbox tier T3 v0.5 (containers Docker/Podman).
- **Multi-Provider**: Inference Port full (Anthropic v0.1 → ≥3 providers v0.5 in CI: + OpenAI + local-LLM via Ollama; v1.5 = MAOS-mediated provider proxies; v2.0 = Bedrock/Vertex AI).
- MCP client all-three transports (stdio / SSE / Streamable HTTP) — Streamable HTTP default at v0.5.
- ACP server (NDJSON over stdio) for editor-hosted Spirits — Zed + VSCode tested at v1.0.
- Spirit registry over MCP-Streamable-HTTP (ADR-008, binding-v0.5): `registry.search` / `manifest` / `artifact` / `publish` / `deprecate` — three trust tiers (`local`, `org-internal`, `public-untrusted`).
- **§13.1 rust-inproc measurement story with go/no-go gate** before v0.5 ships: subprocess form measured against latency budgets (J1 <25ms P95 IPC; J4 <10ms P95). If subprocess meets budgets, rust-inproc form may be deferred. Otherwise, rust-inproc development unlocks.

**FRs covered:** FR3 (full provider config), FR9 (full lifecycle pause/resume + load/start/unload), FR10, FR11, FR12, FR13, FR49, FR50, FR53, FR55, FR5 (T3 v0.5), FR48 (CryptoProvider runtime usage).

**Key NFRs:** NFR-Rel-1 (crash detection ≤2s, `task.orphaned` ≤5s), NFR-Rel-2 (hung-Spirit ≤60s), NFR-Rel-3 (HSIS ≥95% per Spirit class — 6 class-specific corpora × 50 = **300 scenarios**), NFR-Rel-4, NFR-Rel-5 (rollback ≤30s), NFR-Rel-9 (revocation propagation ≤5s p99 under 10⁴ concurrent validations), NFR-Rel-10 (cold-restart ≤30s graceful / ≤1 in-flight message loss on hard kill), NFR-Rel-11 (halt-receipt ≥99.9%), NFR-Perf-7 (hot-swap P99 <500ms), NFR-Test-7 partial (rust-inproc ↔ subprocess semantic equivalence ≥90% — full gate in E10).

**Corpora authored in E5:**
- HSIS additional 4 Spirit-class corpora × 50 = 200 scenarios (Butler + Orchestrator + Worker + CliWrapper; cumulative HSIS 300/300).
- Provider contract corpus ~40 cases (Pact-style; ≥3 providers × interface variations).
- spirit-test SDK integration tests ~25 cases.

**Acceptance demo:** Hot-swap from Butler v0.3.1 to v0.3.2 preserves in-flight `task.assign` frames and active halt context; SIGKILL of subprocess Spirit detected in 1.8s; `task.orphaned` emitted at 3.2s; signed CRL polled and applied; halt-receipt journaled on clean shutdown.

#### Stories

### Story 5.1: Ship Full Lifecycle Verbs and 11 Triggers with Priority-Weighted Scheduling

As an operator,
I want full lifecycle verbs (`load` / `start` / `pause` / `resume` / `unload`) via authenticated control plane (CLI / ACP / operator HTTP API) AND all 11 lifecycle triggers firing at declared cadence with manifest-budgeted resources, AND cooperative + priority-weighted scheduling with OS-level CPU/memory caps,
So that I can drive Spirit lifecycle without trusting Tokio-cooperation alone — OS enforcement is the floor.

**Acceptance Criteria:**

**Given** an authenticated control plane (CLI, ACP, operator HTTP API)
**When** the operator invokes `maosctl <load|start|pause|resume|unload> <spirit>`
**Then** the Spirit Scheduler executes the verb (FR9 full)
**And** the state transition is journaled to the Lifecycle Journal
**And** the transition is auditable via E9 audit-query surface

**Given** a Spirit declares lifecycle hooks in its manifest
**When** the kernel reaches the corresponding lifecycle event
**Then** the kernel fires the declared hook (`on_load`, `on_start`, `on_frame`, `on_idle`, `on_telemetry_event`, `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_unload`, `on_consolidate`)
**And** the hook executes within its manifest-declared resource budget
**And** budget overruns emit `BudgetWarning` IAC frame at 80% (NFR-Perf-6)

**Given** cooperative + priority-weighted scheduling
**When** multiple Spirits compete for runtime
**Then** the Spirit Scheduler dispatches per declared weights with operator-configurable defaults
**And** OS-level CPU/memory budgets are enforced via cgroups v2 (Linux) / setrlimit (macOS) / Job Objects (Windows) — not via Tokio cooperation

**Given** the Butler `on_idle` substrate (v0.3 anchor)
**When** the Spirit Scheduler detects an idle window
**Then** the kernel fires `on_idle(ctx)` on Butler
**And** Butler can perform anticipatory reasoning within its budget (E8 Story 8.1 implements the Butler behavior)

### Story 5.2: Implement Hot-Swap State Transfer and Cross-Major Migration Against HSIS ≥95%

As an operator upgrading a running Spirit,
I want the Hot-Swap Coordinator to preserve in-flight tokens + working memory + active halts across the swap with CBOR state-transfer per ADR-017, AND `migrate(predecessor_state)` cross-major migration per ADR-020, AND HSIS ≥95% pass per Spirit class measured against a 6×50=300 scenario corpus,
So that Spirit upgrades are routine operations — not high-risk redeploys.

**Acceptance Criteria:**

**Given** a Spirit with manifest `[hot_swap].state_schema_uri` and `state_schema_version`
**When** `crates/maos-lifecycle/src/hot_swap/coordinator.rs::initiate_swap` runs against a same-major successor with additive schema changes
**Then** predecessor state is CBOR-encoded per `crates/maos-lifecycle/src/hot_swap/state_codec.rs` (ADR-017)
**And** the successor's `on_swap_in(state)` ABI hook receives the decoded state
**And** swap completes with P99 <500ms measured by `crates/maos-bench/benches/hot_swap_latency.rs` (NFR-Perf-7)

**Given** a cross-major upgrade
**When** the manifest declares `migrates_from = "0.x"` and the predecessor archive exists at `~/.local/share/maos/spirit-archives/<spirit_id>/<predecessor_version>/`
**Then** `crates/maos-lifecycle/src/migration/migrator.rs::run_migrator(predecessor_state)` invokes the Spirit's declared `migrate(predecessor_state) -> Result<successor_state, Error>` entry point (ADR-020)
**And** the kernel refuses to load with `EMigratorMissing` if predecessor archive exists and no migrator declared

**Given** the saga-style compensating transactions in `crates/maos-lifecycle/src/hot_swap/saga.rs`
**When** `on_swap_out` fails
**Then** the kernel restores the predecessor (no successor activation)
**And** when `on_swap_in` fails, the kernel discards the successor and restores predecessor

**Given** a post-swap invariant violation (state-shape, halt continuity, capability mismatch)
**When** the violation is detected within 30s of swap completion by `crates/maos-lifecycle/src/hot_swap/post_swap_monitor.rs`
**Then** the kernel auto-reverts to the predecessor (NFR-Rel-5)
**And** the kernel emits `HotSwapAborted` IAC frame to interested parties via `crates/maos-kernel-core/src/iac/`

**Given** the I14 halt-continuity enforcement (round-3 Amelia + Murat: explicit integration partner of Story 4.1)
**When** the Hot-Swap Coordinator calls Story 4.1's `crates/maos-kernel-core/src/halt/mod.rs::validate_halt_set(spirit_manifest)` before swap
**Then** the swap is rejected with `EHaltContinuityViolation` if Spirit hasn't declared `halt_protocol_compatibility = N` matching the predecessor's halt schema version
**And** active halts retain identity, replay context, and resumption guarantees across the swap (FR53)
**And** the end-to-end integration test lives in `crates/maos-lifecycle/tests/hot_swap_halt_continuity_test.rs` (owned here, not in Story 4.1)
**And** the test exercises ≥10 scenarios from `crates/maos-eval/fixtures/halt-continuity-corpus-v0/` (committed alongside this story)

**Given** the HSIS corpus (NFR-Rel-3)
**When** 6 Spirit-class corpora × 50 scenarios = 300 scenarios run through `crates/maos-eval/tests/hsis_runner.rs`
**Then** ≥95% pass per Spirit class
**And** zero invariant violations CVSS-7 class are tolerated
**And** **corpus authoring schedule (round-3 split):** 100 scenarios authored in Story 4.5 (Researcher + Observer Spirit classes) and committed at `crates/maos-eval/fixtures/hsis-researcher-observer-v0/`; **the remaining 200 scenarios are authored HERE in Story 5.2** for Butler / Orchestrator / Worker / CliWrapper classes and committed to `crates/maos-eval/fixtures/hsis-butler-orchestrator-worker-cliwrapper-v0/` — total 300
**And** **intra-E5 ordering:** Story 5.2 authoring MUST close before Story 4.1's halt-receipt gate AC closes at v1.0 (so the production-grade HSIS corpus replaces Story 4.1's `synthetic-v0` corpus)

### Story 5.3: Detect Spirit Crashes, Hangs, and Silent Failures with Halt-Receipt 99.9%

As a Spirit Scheduler operator,
I want the kernel to detect Spirit-process crashes within 2s and emit `task.orphaned` within 5s (FR12), detect hung Spirits via no-progress IAC >30s, detect silent failures despite healthy heartbeats, produce halt-receipts at 99.9% on every termination, AND support kernel cold-restart in ≤30s on graceful shutdown / ≤1 in-flight message loss on hard kill,
So that the substrate's reliability floor is mechanical not aspirational.

**Acceptance Criteria:**

**Given** a Spirit subprocess receives SIGKILL
**When** the kernel observes the process exit
**Then** crash detection completes within 2s (≥99/100 on the SIGKILL crash corpus, NFR-Rel-1)
**And** `task.orphaned` IAC frames are emitted to in-flight task originators within 5s
**And** exit cause is journaled

**Given** a Spirit alive but emitting no progress IAC for >30s
**When** the kernel's hung-Spirit detection runs
**Then** `task.stalled` event is emitted within 60s
**And** ≥48/50 reclassified within 60s on the hang corpus (NFR-Rel-2)

**Given** a Spirit emits healthy heartbeats but no progress IAC for >30s
**When** the silent-failure detector runs (NFR-Rel-4)
**Then** `silent_failure_suspect` event is emitted
**And** ≥45/50 detected on the adversarial zombie-heartbeat corpus

**Given** a Spirit terminates (planned `unload` or unplanned crash/halt)
**When** the kernel processes the termination
**Then** a halt-receipt is produced before process exit (cross-ref E4 Story 4.1)
**And** halt-receipt production rate is ≥99.9% across termination corpora (NFR-Rel-11)

**Given** the dead-Spirit task disposition (FR50)
**When** a Spirit dies with in-flight tasks
**Then** the kernel applies the manifest-declared `[on_crash].action`: `NACK` / `reassign-to-replica` / `escalate-to-operator`
**And** the disposition is journaled

**Given** kernel cold-restart
**When** `maosctl restart` is invoked gracefully
**Then** restart completes ≤30s with no data loss (NFR-Rel-10)
**And** on hard kill, ≤1 in-flight message is lost

### Story 5.4: Run Spirit Upgrades and Propagate Signed Revocations in ≤5s

As an operator,
I want `maosctl spirit upgrade <spirit> --to <version> --policy <hot-swap|cold-swap|migrator>` (FR49) AND a signed Revocation List (CRL) artifact polled every 5min + offline-import path (FR13), AND revocation propagation ≤5s p99 under 10⁴ concurrent capability-token validations,
So that I can roll forward and roll back Spirits safely and revoke any compromised Spirit across the whole substrate within seconds.

**Acceptance Criteria:**

**Given** a Spirit version upgrade with `--policy hot-swap` (default)
**When** `maosctl spirit upgrade butler --to 0.3.2 --policy hot-swap` runs
**Then** the upgrade uses Story 5.2's hot-swap path with state preservation
**And** the upgrade journals the version transition

**Given** `--policy cold-swap`
**When** the upgrade runs
**Then** the kernel performs an `unload` + `load` cycle with re-init (no state preservation)
**And** in-flight tasks are NACKed per Story 5.3's disposition

**Given** `--policy migrator`
**When** the upgrade is across a major version
**Then** the kernel invokes the manifest-declared `migrate(predecessor_state)` entry point per Story 5.2
**And** failure paths trigger saga-style compensation

**Given** the signed Revocation List artifact
**When** a Spirit is revoked (publisher or operator origin)
**Then** the CRL is registry-pushed and kernel polls every 5min
**And** offline-import via `maosctl revocations import <signed-crl>` is supported (FR60 path)
**And** running Spirit instances receive `SpiritRevoked` event and execute declared revocation policy (`terminate-immediately` / `drain-then-terminate` / `quarantine`)

**Given** revocation propagation under load
**When** 10⁴ concurrent capability-token validations are in flight and a revocation arrives
**Then** propagation latency is ≤5s p99 (NFR-Rel-9)
**And** subsequent token uses against the revoked Spirit fail with `ECapabilityRevoked`

### Story 5.5a: Sandbox Tier T3 — Container Isolation via Docker / Podman

As an operator hosting `public-untrusted` Spirits at v0.5,
I want sandbox tier T3 (containers wrapping T2 protections, via Docker or Podman) implemented in `crates/maos-sandbox/src/t3/`,
So that public-untrusted Spirits run under defense-in-depth (container boundary + Landlock/Seatbelt/restricted-token inside) without operator-managed container orchestration glue.

**Acceptance Criteria:**

**Acceptance Criteria:**

**Given** a Spirit manifest declaring `sandbox_tier = "T3"` (or forced to T3 by trust-tier floor in Story 1b.3)
**When** the Spirit Scheduler spawns the Spirit via `crates/maos-sandbox/src/t3/spawn.rs::spawn_t3`
**Then** the Spirit runs inside a Docker or Podman container chosen by `crates/maos-sandbox/src/runtime_detect.rs`
**And** T2 protections (Landlock+seccomp on Linux inside the container) are also applied
**And** the strictest-of-(manifest, trust-tier, operator-policy) floor from Story 1b.3 is maintained

**Given** the container image used for T3
**When** the kernel builds or pulls it
**Then** the image SHA is pinned in `crates/maos-sandbox/t3-image.lock`
**And** Ed25519 signature verification runs against the image before spawn
**And** image-mismatch fails with `ESandboxImageMismatch`

**Given** a T3-spawned Spirit
**When** the operator runs `maosctl spirit inspect <id> --sandbox`
**Then** the report shows the active container runtime, image SHA, applied T2 protections, and the strictest-tier reasoning chain
**And** the report is journaled to the Lifecycle Journal at spawn time

**Given** the v0.5 ship gate
**When** the T3-escape corpus runs (a Spirit attempting forbidden file/network/exec ops from inside the container)
**Then** the corpus lives in `crates/maos-sandbox/tests/fixtures/t3-escape-attempts/`
**And** 100% of escape attempts are blocked
**And** every block is recorded via `cap-audit` (Story 1b.2) to the Transparency Log

### Story 5.5b: Run the Multi-Provider CI Matrix Across Anthropic, OpenAI, and Ollama

As an operator who refuses provider lock-in,
I want `crates/maos-providers/` hosting driver implementations for Anthropic + OpenAI + Ollama (local-LLM) with a CI matrix that runs the same Spirit-test fixture suite across all three providers and emits a behavioral-comparison report,
So that v0.5 ecosystem expansion is real, provider behavior drift is detectable, and Spirit authors can choose providers per-Spirit without rewriting their code (FR3).

**Acceptance Criteria:**

**Given** `crates/maos-providers/` with three driver crates (`maos-providers/anthropic/`, `maos-providers/openai/`, `maos-providers/ollama/`)
**When** each driver implements the `ProviderDriver` trait declared in `crates/maos-providers/src/lib.rs`
**Then** every driver routes through the kernel-side `Inference Port` from Story 1b.4
**And** Spirit binaries do not import vendor SDKs directly (FR47 enforcement from Story 0.2's kernel-API surface lint)

**Given** the CI matrix configured in `.github/workflows/multi-provider.yml`
**When** the matrix runs the spirit-test fixture suite from Story 2.4 against each provider
**Then** all three providers execute the same fixture inputs
**And** results are normalized into `tests/reports/multi-provider-<sha>.json` with one row per (fixture, provider) pair
**And** behavioral-difference outliers (any provider deviating ≥10% from the median on any fixture) are flagged in the CI report

**Given** the operator-configurable per-Spirit provider declaration in manifest `[providers]`
**When** an operator switches a Spirit from Anthropic → OpenAI mid-deployment
**Then** the switch requires only a manifest change (no Spirit rebuild)
**And** the switch is journaled to the Lifecycle Journal with both provider identities

**Given** air-gapped deployments
**When** the operator configures Ollama as the only provider and disables outbound network
**Then** the substrate runs end-to-end with zero outbound provider calls
**And** this is structurally validated in CI via the air-gapped network-namespace isolation test (later wired in Story 9.4)

### Story 5.5c: MCP Client + ACP Server — Tool Servers and Editor Hosts

As a Spirit author at v0.5 wanting to consume MCP tool servers from inside my Spirit AND host my Spirit inside an IDE (Zed, VSCode),
I want a fully-featured MCP client in `crates/maos-mcp/` supporting all three transports (stdio / SSE / Streamable HTTP) AND an ACP server in `crates/maos-acp/` exposing the Spirit via NDJSON over stdio,
So that Spirits can call external tools AND be edited/debugged in their authors' IDEs without ad-hoc adapters.

**Acceptance Criteria:**

**Given** `crates/maos-mcp/src/client.rs` implementing the MCP client
**When** a Spirit calls `kernel.mcp.call(server_uri, tool, args)`
**Then** the client supports all three MCP transports (stdio / SSE / Streamable HTTP)
**And** Streamable HTTP is the default for Loom-lite, Spirit registry, and production tool servers
**And** transport selection is operator-configurable per server URI

**Given** the ACP server in `crates/maos-acp/src/server.rs`
**When** an editor connects via NDJSON over stdio
**Then** the editor hosts the Spirit's UX surface (task.assign in, halt-resolution out)
**And** ACP-hosted Spirits emit `task.assign` IAC frames equivalently to terminal-hosted Spirits
**And** halt notifications reach the editor via the notification surface from Story 3.1

**Given** Zed and VSCode integrations at v1.0
**When** the integration test runs against each editor's plugin in `tests/integration/acp-editors/`
**Then** both editors successfully host a Spirit through full lifecycle (load → task.assign → halt → resolve → unload)
**And** the editor displays the Spirit's structured introduction in a native UI surface

**Given** the kernel-API surface invariant (Story 0.2)
**When** MCP and ACP code is committed
**Then** neither crate introduces kernel-API surface functions classified `other`
**And** the boundary remains adapter-only — `maos-domain` does not import MCP or ACP types

### Story 5.5d: Spirit Registry over MCP-Streamable-HTTP with Three Trust Tiers

As an operator at v0.5 installing third-party Spirits,
I want the Spirit registry implemented as an MCP-Streamable-HTTP server in `crates/maos-registry/` supporting all five operations (`registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`) with three trust tiers enforced at admission (`local`, `org-internal`, `public-untrusted`; `public-vetted` deferred to v2.5 via FR37),
So that the v0.5 ecosystem expansion has a working publish → discover → install → deprecate loop without operator-managed registry glue.

**Acceptance Criteria:**

**Given** `crates/maos-registry/src/server.rs` exposing the registry as an MCP-Streamable-HTTP server (ADR-008)
**When** the operator points `maosctl registry use <uri>` at the registry endpoint
**Then** all five operations succeed against the configured endpoint:
  - `registry.search(query)` returns matching Spirits
  - `registry.manifest(spirit_id, version)` returns the signed manifest
  - `registry.artifact(spirit_id, version)` returns the signed binary
  - `registry.publish(signed_package)` uploads with Ed25519 verification
  - `registry.deprecate(spirit_id, version)` marks the version yanked

**Given** the three trust tiers at v0.5
**When** an operator attempts to install a Spirit
**Then** admission enforces the strictest-of-(manifest declared tier, registry tier, operator policy) floor
**And** `local` Spirits bypass the registry entirely
**And** `org-internal` Spirits require the registry's org-key signature
**And** `public-untrusted` Spirits require both publisher signature AND admission-time ComplianceClaim verification (full envelope from Story 7.3 lands at v1.0; v0.5 uses the frozen schema from Story 1b.4)
**And** FR37 `public-vetted` tier is explicitly excluded from v1.0 scope per round-2 decision

**Given** the registry yank surface (FR59 full at Story 7.2; v0.5 baseline here)
**When** a yank event arrives via `registry.deprecate`
**Then** the kernel polls every 5min and propagates the yank to running Spirit instances
**And** the yank is distinguishable from operator-local revocation (FR13 via signed CRL from Story 5.4)

**Given** the v0.5 ship gate
**When** the registry roundtrip corpus runs (`crates/maos-registry/tests/fixtures/registry-roundtrip-v05/`)
**Then** 100% of well-formed publish→search→manifest→artifact flows succeed
**And** 100% of malformed flows are rejected with typed errors from the FR63 catalog

### Story 5.5e: §13.1 rust-inproc Measurement Gate — Subprocess vs In-Process Latency Decision

As the architecture lead deciding whether to invest in a second Spirit form,
I want the §13.1 measurement story executed as `crates/maos-bench/benches/section_13_1.rs` measuring J1 (founder-loop CliWrapper IPC) and J4 (Mira-Nash colocation) latency on subprocess form AND a published ADR recording the go/no-go decision before v0.5 ships,
So that the rust-inproc Spirit form is a DATA-DRIVEN unlock, not an architectural aspiration — and downstream stories (NFR-Test-7 cross-form equivalence in Story 10.2) only ship if the measurement requires them.

**Acceptance Criteria:**

**Given** `crates/maos-bench/benches/section_13_1.rs` instrumenting subprocess-form Spirits
**When** the bench runs the J1 workload (founder-loop CliWrapper invocation chain) and J4 workload (two Spirits emitting via the IAC bus at colocation latency)
**Then** P95 IPC latency for J1 and P95 colocation latency for J4 are measured with statistical significance over ≥1000 invocations
**And** the bench report is committed to `tests/reports/section-13-1-<sha>.json`

**Given** the measurement outcome
**When** the architecture lead reviews the J1 + J4 P95 numbers
**Then** **IF** J1 P95 ≤25ms AND J4 P95 ≤10ms (both budgets met by subprocess form): the decision is `defer-rust-inproc-to-v2.0+`, the rust-inproc crate is NOT created in v1.5, NFR-Test-7 cross-form equivalence is REMOVED from v1.5 scope, and CLI-wrapper-only behavioral equivalence runs in Story 10.2
**And** **OTHERWISE**: the decision is `unlock-rust-inproc-in-v0.5`, the `maos-spirit-rust-inproc` crate is scaffolded in v0.5 with NFR-Test-7 cross-form equivalence (rust-inproc ↔ subprocess ≥90%) gating v1.5 in Story 10.2

**Given** the go/no-go decision
**When** the v0.5 release process runs
**Then** a new ADR (numbered after ADR-037) is committed to `docs/adr/` with status `accepted` recording: the measurement methodology, the J1+J4 numbers, the decision, and the rollback criteria
**And** the ADR is linked from STABILITY.md (E10 Story 10.1)
**And** v0.5 release is BLOCKED until the ADR exists with status `accepted`

**Given** the measurement bench
**When** any subsequent v0.x release adds Spirit-class-specific workloads
**Then** the bench can be extended without re-deciding the v0.5 outcome
**And** the decision history is preserved in the ADR ledger

---

### Epic 6: Multi-Spirit Coordination — Full IAC Bus, A2A Peer Mesh & Worker Patterns (v0.5 → v1.5)

**Goal:** Multi-Spirit teams run on a single Host with Orchestrator dispatching to Workers via distillate frames, then across two Hosts via mTLS peer mesh. Subprocess CLI agents (Claude Code, opencode, gemini-cli, kimi-cli) wrap as Worker Spirits.

**Owns:**
- Same-Host IAC bus full features: mailbox-per-Spirit + broadcast + `retract` primitive + log-before-deliver guarantee I2 + Deficit Round Robin fairness scheduler (NFR-Scale-3) in front of log writer.
- Orchestrator dispatching via distillate frames not raw output (FR21): sustained fan-out 50 concurrent Worker Spirits with task-dispatch P99 ≤500ms; 0 dropped tasks under 10 tasks/sec for 1h.
- A2A loopback v0.8 (FR23a): `127.0.0.1`-bound endpoints with self-signed mTLS + TOFU pinning. Test corpus: mTLS replay 100/0; TOFU pin-mismatch 100/100 detected; handshake-fault 20/0; cross-Spirit consent 30 scenarios with 100% disallowed blocked.
- A2A cross-Host v1.0 (FR23b): operator-managed PKI, JSON-RPC framing over mTLS/TCP, ADR-012 typed-intent consent per frame (sender send-allowlist + receiver accept-allowlist; reject with `EIntentDenied`), logical-clock frame ordering (Lamport or hybrid logical clock — final pick by v0.5), network-partition NACK after 30s timeout, no kernel auto-retry.
- mTLS cert rotation chaos test (§7.2.1, v1.5 staging through v1.0): pre-staged-overlap with `T_grace = max(2 × p99_handshake_rtt, 5s)`. Revocation propagation `t_1-t_0` ≤30s p50 / ≤90s p99; re-handshake `t_2-t_1` ≤30s p50 / ≤60s p99; end-to-end `t_2-t_0` ≤60s p50 / ≤150s p99; `cert_post_grace_reject` ≤0.1%.
- `CliWrapperSpirit` class (kernel-builtin): wraps `claude code` / `opencode` / `gemini-cli` / `kimi-cli` with `maos-bridge` + persona skills; declared `output_shape_version` with fail-loud on shape mismatch (FR25 + FR40 full).
- Subprocess CLI invocation under capability-token authority (FR52): stdout/stderr captured to Transparency Log with provenance to invoking Spirit; T3 sandbox profile; explicit manifest declaration required.
- Scheduled invocations (FR26, ADR-025): manifest `[schedule]` table with rate-limit + ComplianceClaim-stamp + principal-revocability + side-effect allowlist; kernel fires `on_schedule(ctx, schedule_id, payload)`.
- Intent provenance / `intent_lineage` (FR24, ADR-018 / I13): all cross-Spirit IAC frames carry intent provenance linking each intent to originating task envelope; preserved across re-emission.
- Gateway sub-modules (FR54, ADR-029): Telegram / Slack / Discord / Signal / email as long-lived connection holders under Spirit's principal namespace (FR31); kernel hosts lifecycle + capability-scope contracts; gateway implementation is Spirit-side.
- Partial-consent failure semantics (`ConsentRupture` event, ADR-034 binding-v0.9).
- Provider rate-limit isolation per-(provider, credential) token bucket; typed `RateLimited` IAC frame.

**FRs covered:** FR21, FR22 (full features — basic in E3), FR23a, FR23b, FR24 (full intent_lineage), FR25, FR26, FR52, FR54.

**Key NFRs:** NFR-Perf-1 (IAC routing P50 <5ms, P99 <50ms), NFR-Perf-2 (5–10K frames/sec sustained), NFR-Perf-8 (Orchestrator fan-out 50 concurrent / 10 tasks-per-sec for 1h), NFR-Sec-11 (mTLS handshake replay-attack: 1000 captured, 0 succeed), NFR-Sec-12 (TOFU pin-mismatch 100% detect/block/alert), NFR-Sec-13 (cert rotation chaos: 3-host v1.5 / 10-host v2.0; revocation ≤60s median / ≤5min p99), NFR-Rel-6 (Spirit-restart invalidates prior A2A TOFU pins; re-pin with consent confirmation), NFR-Rel-7 (A2A churn compressed v2.0; full 100-host v2.5), NFR-Scale-2 (25-host churn v2.0; 100-host v2.5), NFR-Scale-3 (DRR fairness ratio ≤3.0 under 10× noisy Spirit), NFR-Scale-4 (provider rate-limit isolation), NFR-Scale-5 (14-institution Cortex capacity envelope).

**Corpora authored in E6:**
- mTLS handshake replay corpus 1000 captures.
- TOFU pin-mismatch scenarios 100/100.
- A2A cross-Spirit consent 30 scenarios.
- Cert rotation chaos scenarios (3-host).

**Acceptance demo:** Orchestrator dispatches `task.assign` to two Worker Spirits using distillate frames; Workers complete; Transparency Log shows full intent_lineage chain back to originating principal intent; A2A loopback handshake completes with TOFU pin; revoked cert causes immediate block within 30s.

#### Stories

### Story 6.1: Ship the Full IAC Bus with Retract Primitive and DRR Fairness Scheduler

As a kernel hot-path engineer,
I want the same-Host IAC bus's full feature set (mailbox-per-Spirit + broadcast + `retract` primitive + log-before-deliver guarantee I2) AND a Deficit Round Robin fairness scheduler in front of the log writer with operator-configurable per-Spirit weights,
So that one noisy Spirit cannot starve the others and the IAC routing budget (P50 <5ms, P99 <50ms, 5–10K frames/sec sustained) is hit reliably.

**Acceptance Criteria:**

**Given** the IAC bus full features
**When** any frame is dispatched
**Then** mailbox-per-Spirit routing delivers to the addressed Spirit
**And** broadcast routing fans out to multiple subscribers via `tokio::sync::broadcast`
**And** the `retract` primitive supports cancellation of in-flight frames not yet delivered
**And** log-before-deliver (I2) is preserved end-to-end (E1b Story 1b.1)

**Given** the DRR (Deficit Round Robin) fairness scheduler in front of the log writer
**When** writers compete for log-write bandwidth
**Then** per-Spirit weight=1 default applies with operator-configurable `[scheduler.weights]` in policy file
**And** under uneven load (1 noisy Spirit at 10× median write rate + ≥4 normal Spirits sustained 60s) the max-min P99 latency ratio across Spirits is ≤3.0 (NFR-Scale-3)

**Given** the IAC routing budgets
**When** measured on a typical Linux box (NVMe + 16-core tier)
**Then** P50 latency is <5ms (NFR-Perf-1)
**And** P99 latency is <50ms
**And** sustained throughput is 5,000–10,000 frames/sec single-host before log writer becomes bottleneck (NFR-Perf-2)

### Story 6.2: Dispatch Orchestrator Distillates with Intent-Lineage and CliWrapperSpirit Worker Pattern

As a director running an Orchestrator over Workers,
I want the Orchestrator to dispatch `task.assign` frames to Workers using DISTILLATE frames (not raw output) AND every frame to carry unbroken intent_lineage back to my originating intent, AND the CliWrapperSpirit class to wrap external CLI agents (Claude Code / opencode / gemini-cli / kimi-cli) with `output_shape_version` fail-loud,
So that the v0.8 founder-loop wedge demo actually works — Orchestrators don't drown in raw Worker output and external CLI agents become first-class Workers.

**Acceptance Criteria:**

**Given** an Orchestrator dispatching `task.assign` to Workers
**When** the Orchestrator processes Worker output between dispatches
**Then** subsequent dispatches use the distillate of prior Worker output (not raw output) — closing the raw-output context-overflow loophole (FR21)
**And** the distillation pattern uses kernel primitives from E4 (Story 4.4)

**Given** sustained Orchestrator fan-out
**When** 50 concurrent Worker Spirits run under 10 tasks/sec for 1 hour
**Then** task-dispatch latency is P99 ≤500ms (NFR-Perf-8)
**And** 0 tasks are dropped

**Given** any cross-Spirit IAC frame
**When** the frame is emitted or re-emitted (cross-ref E4 Story 4.5)
**Then** the frame carries unbroken `intent_lineage` chain back to the originating principal intent (I13, ADR-018)
**And** 100% of cross-Spirit frames carry the lineage (NFR-Aud-14)

**Given** the kernel-builtin CliWrapperSpirit class
**When** a Worker Spirit declares `[cli_wrapper]` with `command = "claude code"` and `output_shape_version = "1.0.0"`
**Then** the kernel spawns the CLI subprocess under T3 sandbox + capability-token authority
**And** stdout/stderr are captured into the Transparency Log with provenance to the invoking Spirit (FR52)
**And** observed CLI output that doesn't match `output_shape_version` causes the CliWrapperSpirit to refuse start with `EOutputShapeAdapterMismatch` (FR25 + FR40)

**Given** a Spirit invokes external CLI via the CliWrapperSpirit
**When** the CLI exits cleanly
**Then** the kernel records the exit + captured output to the Transparency Log
**And** the capability-token authority used for the invocation is journaled

### Story 6.3: Build the A2A Peer Mesh from Loopback to Cross-Host with mTLS Rotation Chaos

As an operator running a Diagnostic-Architect bilateral 2-Host pair (Host A prod-edge + Host B dev-environment),
I want A2A peer mesh: loopback v0.8 (127.0.0.1 mTLS + TOFU pinning) → cross-Host v1.0 (operator-managed PKI + ADR-012 typed-intent consent + logical-clock ordering) AND mTLS cert rotation chaos test with timing gates,
So that Mira on Host A and Nash on Host B coordinate without operator-managed certificate juggling and rotation under load doesn't drop conversations.

**Acceptance Criteria:**

**Given** A2A loopback at v0.8 (FR23a)
**When** Spirits across "Hosts" communicate via `127.0.0.1`-bound endpoints
**Then** the handshake uses self-signed mTLS with TOFU pinning
**And** mTLS handshake replay-attack corpus: 1000 captured handshakes replayed, 0 succeed (NFR-Sec-11)
**And** TOFU pin-mismatch on second connection: 100% detected, blocked, alerted (NFR-Sec-12)
**And** handshake-fault test: 20/0 succeed
**And** cross-Spirit consent: 30 scenarios with 100% disallowed blocked

**Given** A2A cross-Host at v1.0 (FR23b)
**When** Host A and Host B communicate over operator-managed PKI
**Then** the framing is JSON-RPC over mTLS/TCP
**And** every frame carries ADR-012 typed-intent consent (sender send-allowlist + receiver accept-allowlist; reject with `EIntentDenied`)
**And** frame ordering uses logical clocks (Lamport or hybrid logical clock, final pick by v0.5; wall-clock is metadata only)
**And** network-partition NACKs in-flight frames after configurable timeout (default 30s); kernel does NOT auto-retry

**Given** Spirit-restart on Host A
**When** Host A's Spirit comes back up
**Then** prior A2A TOFU pins on Host B are invalidated (NFR-Rel-6)
**And** re-pin protocol with consent confirmation is required before re-establishment

**Given** mTLS cert rotation under live load (§7.2.1, NFR-Sec-13)
**When** rotation is forced quarterly
**Then** `T_grace = max(2 × p99_handshake_rtt, 5s)` pre-staged-overlap applies
**And** revocation propagation latency `t_1 - t_0` ≤30s p50 / ≤90s p99
**And** re-handshake latency `t_2 - t_1` ≤30s p50 / ≤60s p99
**And** end-to-end rotation `t_2 - t_0` ≤60s p50 / ≤150s p99
**And** `cert_post_grace_reject` rate ≤0.1%
**And** rotation chaos test: 3-host at v1.5 / 10-host at v2.0 with zero conversation drops

**Given** A2A trust establishment under churn (NFR-Rel-7)
**When** the compressed 30-host Cortex runs with 10–20% turnover/week × 4 weeks with 3 planted adversarial hosts
**Then** detection latency ≤1h median
**And** blast radius ≤5 peers
**And** recovery ≤24h
**And** v2.0 ships compressed scale; v2.5 ships full 100-host

### Story 6.4: Wire Scheduled Invocations with ConsentRupture and Provider Rate-Limit Isolation

As a Spirit author writing scheduled work,
I want manifest `[schedule]` declarations firing `on_schedule(ctx, schedule_id, payload)` with rate-limit + ComplianceClaim-stamp + principal-revocability + side-effect allowlist (ADR-025), AND partial-consent ConsentRupture event semantics (ADR-034) when only some recipients accept a frame, AND per-(provider, credential) token-bucket rate limit isolation,
So that scheduled invocations can't bypass consent and one Spirit's provider quota exhaustion doesn't starve others.

**Acceptance Criteria:**

**Given** a Spirit declares `[[schedule]]` in its manifest
**When** the kernel reaches the declared cadence
**Then** the kernel fires `on_schedule(ctx, schedule_id, payload)` (FR26)
**And** the invocation is rate-limited per the manifest declaration
**And** the invocation carries a ComplianceClaim-stamp per Story 7.3's envelope
**And** the principal-revocability check passes (revoked principal = no fire)
**And** side-effects are constrained to the manifest-declared allowlist

**Given** a multi-recipient IAC frame where some recipients accept and others reject the typed-intent consent
**When** the kernel processes the frame (ADR-034 binding-v0.9)
**Then** the kernel emits a `ConsentRupture` event capturing accepted/rejected recipients
**And** the frame is delivered only to consenting recipients
**And** the sending Spirit observes the rupture and can decide whether to proceed

**Given** the per-(provider, credential) token bucket
**When** a Spirit exhausts its provider rate limit
**Then** the kernel emits typed `RateLimited` IAC frame to the Spirit (NFR-Scale-4)
**And** other Spirits using the same provider with different credentials are NOT throttled
**And** the bucket refills per the provider's published rate

### Story 6.5: Gateway Sub-Modules (ADR-029) — Telegram / Slack / Discord / Signal / Email

As a Spirit author building a Director's mobile-push integration,
I want manifest gateway sub-module declarations (e.g., Telegram, Slack, Discord, Signal, email) running as long-lived connection holders under my Spirit's principal namespace (FR31), with kernel-hosted lifecycle and capability-scope contracts,
So that the v1.0 hermes-tenant positioning claim is defended — gateway integration is principal-scoped, audit-traced, and uninstall-clean.

**Acceptance Criteria:**

**Given** a Spirit declares `[[gateway]] type = "telegram"` in its manifest (per `schemas/gateway-submodule.schema.json`)
**When** the kernel admits the Spirit
**Then** the gateway sub-module is hosted under the Spirit's principal namespace (FR31)
**And** lifecycle hooks (`on_connect`, `on_disconnect`, `on_inbound_message`) fire per the kernel's contract
**And** the gateway implementation itself is Spirit-side code

**Given** the gateway has issued capability tokens
**When** any operation routes through the gateway
**Then** the operation traverses the Capability Registry per I1
**And** every external message is recorded in the Transparency Log with provenance back to the Spirit

**Given** Spirit uninstall (FR65)
**When** the operator runs `maosctl uninstall <spirit>`
**Then** all gateway-side state under the principal namespace is enumerated in the proof-of-erasure record
**And** the gateway connection is terminated cleanly with no orphaned credentials

**Given** the gateway sub-module schema (`schemas/gateway-submodule.schema.json`)
**When** a Spirit declares any gateway
**Then** the manifest validates against the schema at admission
**And** schema violations are rejected with actionable errors

---

### Epic 7: Spirit Ecosystem — Authoring SDK, Registry, Signing, ComplianceClaim Envelope & Trust Tiers (v0.5 → v1.0; FR37 deferred v2.5)

**Goal:** A third-party Spirit author scaffolds, tests, signs, and publishes a Spirit; operator installs it across three trust tiers (`local`, `org-internal`, `public-untrusted`) with mandatory Ed25519 signature verification + ComplianceClaim envelope verification at admission + revocation propagation. NFR-Onb-1 30-Min First Spirit Validation Gate executes at v0.3 (Butler-driven) using E2 prerequisites + E8 Butler reference.

**Owns:**
- Full `cargo generate maos-spirit` per-language (Rust v0.5; TypeScript v0.5; Python v1.0; Go v1.5).
- Full `spirit-test` SDK with assertion macros (extends E2 SDK seed): covers lifecycle hooks, IAC frame I/O, halt resolution, manifest self-check, class-specific regression corpus. Coverage floor ≥80% of Spirit-author manifest-declared capabilities reachable via fixtures.
- `maos-spirit publish --tier=<tier>` CLI with Ed25519 signing; package conforms to `maos.spirit.v1` schema.
- Spirit registry full features (over MCP-Streamable-HTTP): `registry.search`, `registry.manifest`, `registry.artifact`, `registry.publish`, `registry.deprecate`.
- Three trust tiers at v1.0: `local`, `org-internal`, `public-untrusted` (PRD's `public-vetted` deferred to v2.5 via FR37).
- ComplianceClaim envelope (binding-v1.0 first-class object): Ed25519-signed, references execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity). Kernel verifies at admission with typed `EComplianceContextDrift` on drift.
- ComplianceClaim Adversarial Corpus (CCAC) v1.0: **N=600 = 200 well-formed + 400 malformed** (authored via parameterized generator: 20 well-formed templates × 10 variations = 200; 40 malformed templates × 10 variations = 400). Per-class N=30 floor ≥27/30. 100 context-drift claims 100/100 rejected. Cross-validation across ≥3 reference Spirits, agreement within ±2%.
- `maos-compliance` semantic evaluator (v0.9 binding) for ComplianceClaim envelope validation.
- Skill authoring (`maos.skill.v1` markdown + TOML frontmatter): ship in Spirit package OR write dynamically at runtime via `skill.author.self` capability scope; new skills enter operator-admission queue.
- CliWrapper `output_shape_version` fail-loud (FR40 full): kernel-builtin CliWrapperSpirit refuses to start if observed CLI shape doesn't match declared version.
- Skill-revision proposals (FR57): Spirit queries own performance telemetry within principal namespace (FR31 + FR56); emits proposal carrying target skill id + version + proposed diff + telemetry evidence; enters operator-admission queue (FR39).
- Registry yank events (FR59): publisher- and vetter-initiated yanks propagate to operators on next sync (≤5min poll cadence default); distinguishable from operator-local revocation (FR13).
- Air-gapped artifact import (FR60): signed artifacts (Spirit + skills) from offline media or mirrored registries preserve full verification chain.
- ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` with N-1 supported / N-2 hard refusal (typed `EAbiTooOld`).
- STABILITY.md live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- 1-year LTS commitment at v1.0; 2-year LTS at v1.5 (deferred to E10).
- BREAKING.md grep-enforced entry for every breaking change with migration steps.
- Deprecation timeline: 2 minor releases warning + 1 major to remove.
- `min_substrate_version` manifest field enforcement; manifest schema N-1 compatibility with documented degradation paths (NFR-Maint-9).
- Skill ecosystem: filesystem-discovered at v0.5 (`~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`); `maos.skill.v1` format intentionally close to Anthropic Skills format.

**FRs covered:** FR33 (full), FR34 (full), FR35, FR36, FR37 (DEFERRED v2.5), FR38 (envelope + admission verification v1.0 — schema frozen E1b), FR39, FR40 (full fail-loud), FR57, FR59, FR60.

**Key NFRs:** **NFR-Aud-9 (CCAC N=600 ship gate)**, NFR-Comp-2 (vetter accreditation parameters — qualification matrix, conflict-of-interest, 7-year audit retention), NFR-Test-3 (spirit-test SDK ≥80% coverage validated by 5+ third-party Spirits), **NFR-Onb-1 (30-Min First Spirit Gate execution: N=12 stratified — ≥4 no prior MAOS contribution / ≥3 never written Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only; floor median ≤45min, p95 ≤90min, ≥10/12 succeed in 14 days zero-DM-support — Butler-class corpus 30-scenario calendar/comms; halt-recall ≥0.90 calendar-conflict; halt-precision ≥0.85)**, NFR-Onb-3 (three-door page `docs.maos.dev`), NFR-Onb-4 (gate iteration cadence: 3 consecutive misses escalates to v0.3 release review), NFR-Maint-3 (ABI compat 100% within major, 100% N-1), NFR-Maint-4 (STABILITY.md), NFR-Maint-5 (deprecation 2-minor + 1-major), NFR-Maint-6 (1-year LTS at v1.0), NFR-Maint-7 (BREAKING.md), NFR-Maint-9 (manifest N-1).

**Corpora authored in E7:**
- **CCAC corpus N=600** (generator-driven: 20 well-formed templates × 10 variations + 40 malformed templates × 10 variations).
- 30-Min First Spirit Gate task scripts (~24 task scripts for N=12 trial — 2 per participant).
- Three-door page content (`docs.maos.dev`).
- 5+ third-party Spirit external trials for NFR-Test-3 coverage validation.

**Acceptance demo:** External author scaffolds Spirit, signs and publishes to local registry; operator installs from `org-internal` tier with ComplianceClaim envelope verification at admission; signed Revocation List propagates within 5min; air-gapped operator imports same artifact preserving signature chain; 30-Min Gate cohort succeeds 10/12.

#### Stories

### Story 7.1: Full `cargo generate` Per-Language + Full spirit-test SDK with Assertion Macros

As a Spirit author across multiple languages,
I want the full `cargo generate maos-spirit` per-language templates (Rust v0.5; TypeScript v0.5; Python v1.0; Go v1.5) AND the full spirit-test SDK with assertion macros covering lifecycle hooks + IAC frame I/O + halt resolution + manifest self-check + class-specific regression corpus,
So that the v0.5 ecosystem expansion supports non-Rust authors and the SDK coverage floor ≥80% (NFR-Test-3) is mechanically verifiable.

**Acceptance Criteria:**

**Given** the `cargo generate maos-spirit --lang <rust|typescript|python|go>` invocation
**When** the template scaffolds a Spirit
**Then** Rust + TypeScript templates work at v0.5
**And** Python template lands at v1.0
**And** Go template lands at v1.5
**And** every template ships with a working `on_idle` example, manifest, README, and passing CI

**Given** the full spirit-test SDK extending the E2 seed
**When** an author calls `spirit_test::assert!` / `spirit_test::expect_halt!` / `spirit_test::expect_frame!` macros
**Then** the macros provide compile-time-checked assertions against the Spirit ABI
**And** the macros render readable failure messages with file + line + suggested-fix

**Given** the SDK coverage floor (NFR-Test-3)
**When** measured against 5+ third-party Spirits authored by external developers
**Then** ≥80% of each Spirit-author's manifest-declared capabilities are reachable via SDK fixtures
**And** the measurement is committed to `coverage-matrix.yaml`

**Given** the Spirit-side `kernel.deprecation_warnings()` channel
**When** a Spirit uses a deprecated API
**Then** `spirit-test` surfaces the deprecation in test output
**And** the channel is consulted by the ABI compatibility matrix gate (NFR-Maint-3)

### Story 7.2: Ship End-to-End Registry — Publish, Install, Yank, and Air-Gapped Import

As an operator distributing Spirits,
I want `maos-spirit publish --tier=<tier>` with Ed25519 signing (FR35), full Spirit registry over MCP-Streamable-HTTP with all five operations (FR36), three trust tiers at v1.0 (`local`, `org-internal`, `public-untrusted`; `public-vetted` deferred to v2.5 via FR37), registry yank events propagating ≤5min (FR59), AND air-gapped artifact import preserving full verification chain (FR60),
So that the full publish → discover → install → revoke → air-gap-import loop works end-to-end at v1.0.

**Acceptance Criteria:**

**Given** the `maos-spirit publish --tier=<tier>` CLI
**When** an author publishes a Spirit
**Then** the published package conforms to `maos.spirit.v1` schema
**And** the package is Ed25519-signed
**And** the tier is one of `local` / `org-internal` / `public-untrusted` (FR37 `public-vetted` deferred v2.5)

**Given** the Spirit registry (Story 5.5 + this story extends)
**When** an operator invokes `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`
**Then** all five operations succeed against the configured registry endpoint
**And** mandatory signature verification + trust-tier floor enforcement runs at admission (FR36)

**Given** a publisher- or vetter-initiated yank event
**When** the kernel polls the registry (≤5min default cadence)
**Then** running Spirit instances receive the yank notification within 5min (FR59)
**And** the yank is distinguishable in audit from operator-local revocation (FR13)
**And** operator response semantics (warn / quarantine / auto-revoke) apply per operator policy

**Given** air-gapped operator import
**When** the operator runs `maosctl import --offline <signed-bundle.tar>`
**Then** the kernel verifies the Ed25519 signing chain on the bundle (FR60)
**And** vetter attestations and ComplianceClaim envelopes in the bundle verify locally
**And** the imported Spirit is admitted equivalently to registry-served Spirits

### Story 7.3: Verify ComplianceClaim Envelopes at Admission with the CCAC N=600 Ship Gate

As a substrate compliance lead,
I want the ComplianceClaim envelope as a binding-v1.0 first-class Ed25519-signed object referencing the execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope + provider-endpoint + crypto-provider) AND the CCAC corpus N=600 (NFR-Aud-9) as a v1.0 ship gate AND the `maos-compliance` semantic evaluator (v0.9 binding),
So that admission verification mechanically rejects context-drifted Spirits and the v1.0 ship-gate evidence is third-party-reproducible.

**Acceptance Criteria:**

**Given** a Spirit declared with execution-context fingerprint
**When** the operator admits the Spirit
**Then** the kernel verifies the ComplianceClaim envelope's Ed25519 signature
**And** the kernel computes the runtime execution-context fingerprint
**And** drift between declared and runtime fingerprint triggers admission rejection with typed `EComplianceContextDrift` (FR38)

**Given** the CCAC corpus N=600 (generator-driven per Murat's discipline)
**When** the corpus is authored
**Then** it comprises 200 well-formed (20 templates × 10 variations) + 400 malformed (40 templates × 10 variations) ComplianceClaim envelopes
**And** per-class N=30 minimum
**And** 100 context-drift claims are present (100/100 rejected at admission)

**Given** the CCAC v1.0 ship gate
**When** the corpus runs against ≥3 reference Spirits
**Then** per-class floor ≥27/30 passes
**And** cross-validation across the 3 Spirits shows agreement within ±2%
**And** failure is a P0 ship-blocker

**Given** the `maos-compliance` crate (v0.9 binding)
**When** ComplianceClaim envelopes flow through the semantic evaluator
**Then** the evaluator validates structural correctness + signature + execution-context match
**And** validation latency does not bottleneck admission (<10ms P99 per envelope on a typical Linux box)

### Story 7.4: Author Skills and Propose Revisions with Output-Shape Fail-Loud

As a Spirit author authoring skills,
I want to author skills as `maos.skill.v1` (markdown + TOML frontmatter) shipped in the Spirit package OR written dynamically at runtime via `skill.author.self` capability scope, AND skill-revision proposals (FR57) from Spirit's self-telemetry entering the operator-admission queue, AND CliWrapper `output_shape_version` fail-loud (FR40 full),
So that the substrate's skill ecosystem is real (filesystem-discovered at v0.5) and Spirits can propose evidence-backed improvements to their own skills.

**Acceptance Criteria:**

**Given** the `maos.skill.v1` format (markdown + TOML frontmatter)
**When** an author writes a skill
**Then** the skill validates against the schema at admission
**And** the skill can be shipped in the Spirit's package
**And** the skill can be written dynamically at runtime via `skill.author.self` capability scope, entering the operator-admission queue (FR39)

**Given** filesystem-discovered skills at v0.5
**When** the kernel scans skill paths
**Then** conventional locations are checked: `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`
**And** discovered skills are surfaced via `maosctl skills list`

**Given** a Spirit emits a skill-revision proposal (FR57)
**When** the Spirit queries its own performance telemetry (E4 Story 4.3 FR56)
**Then** the proposal carries (target skill id + version, proposed diff, telemetry evidence)
**And** the proposal enters the operator-admission queue (FR39 path)
**And** the proposal is subject to the same vetting and audit obligations as new skills

**Given** a CliWrapperSpirit with declared `output_shape_version`
**When** the wrapped CLI's output shape changes (e.g., `claude code` updates and breaks parsing)
**Then** the kernel refuses to start the CliWrapperSpirit with `EOutputShapeAdapterMismatch` (FR40 full fail-loud)
**And** the failure is journaled with version diff
**And** the operator must publish an updated CliWrapperSpirit configuration before resumption

**Given** the LCAS (Long-context Ambiguity Stress) corpus extension — round-3 resolution of the orphaned 140 items
**When** Story 7.4 closes its acceptance gate at v1.0
**Then** the LCAS corpus has been extended from 70 items (Story 2.4 clearly-decidable bucket) to the full N=210 — adding 70 genuinely-ambiguous items + 70 adversarially-misleading items
**And** the genuinely-ambiguous bucket exercises Spirit decisions where multiple defensible answers exist
**And** the adversarially-misleading bucket exercises A2A scenarios with planted load-bearing claims contradicting louder repeated claims (requires E6 A2A loopback from Story 6.3 to be testable; therefore authored at v0.8 after E6 ships, but acceptance lives in this Story 7.4)
**And** all 210 items are committed to `tests/corpora/lcas-v1.0-<sha>.jsonl` (SHA-256-pinned per Story 0.3)
**And** the corpus is registered in `tests/coverage-matrix.yaml` with `valid_until` 12 months out

### Story 7.5a: Publish and Enforce v1.0 ABI Stability Commitments

As a substrate maintainer at v1.0 ship,
I want the ABI Stability Triple `(kernel_version, abi_version, manifest_schema_version)` enforced at kernel load (N-1 supported / N-2 hard refusal with typed `EAbiTooOld`), STABILITY.md + BREAKING.md published with the live compatibility matrix and LTS policy, AND `min_substrate_version` rejected loudly,
So that v1.0's ABI stability promise is mechanically enforced — not a marketing claim — and the 1-year LTS clock starts on a verifiable artifact.

**Acceptance Criteria:**

**Given** the ABI Stability Triple in `crates/maos-spirit-abi/src/version.rs`
**When** a Spirit declares `min_substrate_version` in its manifest
**Then** the kernel rejects load if its own version is below the declared minimum
**And** N-1 within current major is fully supported by the load path
**And** N-2 hard-refuses with typed `EAbiTooOld`
**And** the deprecation timeline (2 minor releases warning, 1 major to remove) is enforced by `crates/maos-spirit-abi/src/deprecation.rs` via tagged-warning emission per NFR-Maint-5

**Given** STABILITY.md at the repo root
**When** v1.0 ships
**Then** the file contains the live `(kernel_version, abi_version, manifest_schema_version)` compatibility matrix generated by `xtask/stability_matrix.rs` from the workspace state
**And** the file declares the LTS branch policy (1-year at v1.0; 2-year at v1.5)
**And** the file contains the substrate-self compliance scope clause (NFR-Comp-3 — full content authored in Story 9.5)
**And** the file contains the export-control classification (NFR-Comp-1 — full content authored in Story 10.3)
**And** BREAKING.md (NFR-Maint-7) is grep-enforced via CI — every breaking change requires a dated entry with migration steps

**Given** manifest N-1 compatibility (NFR-Maint-9)
**When** kernel version V loads a manifest written for V-1
**Then** the load succeeds with documented degradation paths emitted to `tracing` at WARN level
**And** unrecognized fields are warned, not rejected
**And** an integration test in `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs` exercises the V→V-1 path for every supported field

**Given** the 1-year LTS commitment
**When** v1.0 ships
**Then** the LTS clock starts and is published in STABILITY.md with the v1.0 commit SHA + tag
**And** security-only patches after year 1 are documented in the policy section

### Story 7.5b: Execute NFR-Onb-1 30-Minute First Spirit Validation Gate at v0.3

As the human-research lead validating MAOS's onboarding floor,
I want the NFR-Onb-1 gate executed at the v0.3 release with N=12 stratified Spirit authors (with documented recruitment, screener, support-log, and outcome-tracking artifacts) AND the three-door page live at `docs.maos.dev`,
So that the v0.3 release criterion is met via reproducible human-trial evidence — not a vibe — and we have a real signal on whether the substrate is learnable by people we haven't met.

**Prerequisites resolved per round 2 + round 3:**
- **Story 2.3** (thin cargo-generate slice + local runner) ships at v0.3-α — provides the SDK scaffold participants use
- **Story 8.1** (Butler v0.3 reference Spirit) ships at v0.3 — provides the regression corpus participants run against
- **Story 0.3** (corpus infrastructure + coverage matrix) ships at v0.1 — provides the corpus harness for measurement

**Acceptance Criteria:**

**Given** the recruitment process documented in `docs/research/nfr-onb-1-protocol.md`
**When** participants are recruited for the v0.3 trial
**Then** the cohort of N=12 meets the stratification: ≥4 with no prior MAOS contribution / ≥3 never written a Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only
**And** participant credentials are verified via a screener form committed to `docs/research/nfr-onb-1-screener.md`
**And** the recruitment log lives at `_research/nfr-onb-1/v0.3/recruitment-log.jsonl` (private; not in main repo)

**Given** the 14-day trial window with zero direct-message support
**When** the trial runs
**Then** participants receive only the published documentation (cargo-generate template README + three-door page + Butler reference Spirit code)
**And** all support requests are routed to a public issue tracker (a private DM channel violates the protocol)
**And** any DM-channel breach invalidates the trial and triggers re-recruitment per NFR-Onb-4

**Given** the gate floor
**When** the trial completes
**Then** ≥10 of 12 participants produce a working signed Spirit binary that passes the Butler-class corpus (30-scenario calendar/comms from Story 8.1; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall)
**And** time-to-success across the cohort: median ≤45 min, p95 ≤90 min
**And** outcome data is committed to `_research/nfr-onb-1/v0.3/outcomes.jsonl` (private)

**Given** NFR-Onb-4 iteration cadence
**When** the gate misses the floor
**Then** a fresh 6-author cohort runs within 2 weeks
**And** 3 consecutive misses escalate to v0.3 release-criterion review (PRD-author + architecture lead + research lead in the room)

**Given** the three-door page at `docs.maos.dev` (NFR-Onb-3)
**When** the gate runs
**Then** the page hosts three onboarding paths ("write a Spirit" / "run MAOS" / "understand MAOS")
**And** the Spirit-author path links to the cargo-generate template from Story 2.3
**And** the page passes WCAG AA (deferred polish; v1.0 in Story 9.5)

---

### Epic 8: Reference Spirits — Butler → Researcher/Observer → Orchestrator+Workers+Architect+Reviewer → Mira+Nash (v0.3 → v1.5)

**Goal:** Each phase release ships at least one production-quality reference Spirit anchoring a real user journey (J0 / J-Butler / J-Researcher / J1 founder loop / J4 Mira-Nash diagnostic-architect / J6 Diego cold-start) and validating the substrate end-to-end. **Zero kernel KLOC — all subprocess Spirit code in `spirits/` directory.** Reference Spirits are *deliverables* (operators expect them out-of-the-box) AND *validation fixtures* (they exercise NFR-Test-4 halt-recall floors, NFR-Rel-3 HSIS per Spirit class, NFR-Test-6 LCAS, NFR-Test-8 third-party trial benchmarks).

**Sub-stories per Spirit class anchored to release phase:**

- **Butler v0.3** — `on_idle` substrate for anticipatory reasoning; calendar/comms 30-scenario regression corpus; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall; bmad-eval baseline ≥0.85; **ships morning digest implementation (FR17 Spirit-side)** via §9.5 distillation pattern with hallucination floor 0/100 verified against actual Transparency Log; ≥95/100 digests must include all open halts and cite source log refs. **Drives NFR-Onb-1 v0.3 gate execution.**

- **Researcher v0.5** — distillation pattern reference; `log.recall` walker; Spirit-side LLM compression with kernel-enforced I11 audit chain (mandatory `source_log_ref`, `distillation_depth`, `intent_lineage`); sources morning digest at v0.5+ phase; subscribes to `scalar.tap` channel.

- **Observer v0.5** — broad telemetry stream subscriber; pre-halt scalar drift watchdog; emits structural-anomaly events (sandbox-escape syscall pattern divergence, fd-table growth, unexpected outbound IAC connections — NFR-Sec-3 v2.0) for operator review.

- **Orchestrator + Worker + Architect + Reviewer v0.8/v0.9** — founder-loop wedge demo (v0.8 PRD = v0.9 architecture phase); Orchestrator with instruction buffering (FR20); distillate-fed dispatch (FR21); Worker = wrapped CLI agent (Claude Code / opencode / gemini-cli / kimi-cli); halt-and-resume-overnight pattern; sources morning digest at v0.8+. The PRD's wedge demo is the proving artifact.

- **Mira + Nash v1.5** — diagnostic-architect bilateral pair across two Hosts; A2A cross-Host operational; safety-critical Spirit corpus methodology N≥150 with inter-annotator agreement κ≥0.7; pre-paired mTLS cert fingerprints (no discovery); mobile push to operator on halt; J4 latency budget <10ms P95 Observer colocation.

**FRs covered:** FR58 (per-phase reference Spirit deliverable at each phase v0.3+). Underwrites FR17 (Spirit-side morning digest implementation at each phase), J0/J-Butler/J-Researcher/J1/J4/J6 reproducibility gates.

**Key NFRs:** NFR-Test-4 (halt-recall ≥0.7, halt-precision ≥0.85 per Spirit class on bmad-eval — needs Spirit classes to exist), NFR-Test-6 LCAS additional buckets (genuinely-ambiguous + adversarially-misleading — adversarial bucket REQUIRES A2A scenarios from E6; therefore authored at v0.8 in conjunction with E6 + E8), NFR-Onb-1 (30-Min First Spirit Gate — Butler is the proving Spirit), per-journey latency budgets §13.1 (J0 Butler conversational <400ms P95 / IPC <60ms; J1 Founder-loop CliWrapper IPC <25ms P95; J4 Mira-Nash Observer colocation <10ms P95; J6 Diego cold-start <500ms).

**Corpora authored in E8:**
- Butler calendar/comms regression corpus 30 scenarios.
- LCAS genuinely-ambiguous + adversarially-misleading buckets 140 items (E2 owns clearly-decidable; E8 owns the remaining 140 — **timed for v0.8 when A2A exists**).
- Mira+Nash safety-critical corpus N≥150 with IAA κ≥0.7.

**Acceptance demos:**
- **v0.3:** Butler ships; on_idle anticipatory reasoning visible; 30-scenario calendar/comms passes; 30-Min Gate cohort succeeds 10/12.
- **v0.5:** Researcher distills corpus end-to-end with I11 audit chain; Observer surfaces scalar.tap drift event before halt fires.
- **v0.8/v0.9:** Founder-loop wedge: Director assigns overnight task → Orchestrator buffers + dispatches to Workers → distillate-frame audit complete by morning → digest cited from actual log refs.
- **v1.5:** Mira on Host A and Nash on Host B coordinate over A2A cross-Host; mTLS rotation chaos passes; safety-critical κ≥0.7 verified.

#### Stories

### Story 8.1: Butler v0.3 — `on_idle` Anticipatory Reasoning + Morning Digest Spirit-Side

As a director using MAOS for the first time at v0.3,
I want the Butler reference Spirit shipped with `on_idle` anticipatory reasoning, a 30-scenario calendar/comms regression corpus, AND the morning digest implementation (FR17 Spirit-side) consuming kernel log-composition primitives from E3 Story 3.4,
So that the v0.3 release has a real reference Spirit that drives the 30-Min First Spirit Validation Gate (NFR-Onb-1 owned by E7 Story 7.5) and proves the substrate's audit trail can produce a hallucination-free morning digest.

**Acceptance Criteria:**

**Given** the Butler reference Spirit in `spirits/butler/`
**When** Butler is loaded
**Then** the Spirit declares `on_idle` in its manifest with a budgeted resource envelope
**And** the kernel fires `on_idle(ctx)` during idle windows
**And** Butler performs anticipatory reasoning (calendar conflict detection, comms triage) within its budget

**Given** the 30-scenario calendar/comms regression corpus
**When** Butler runs the corpus via `spirit-test`
**Then** the corpus is **authored here in Story 8.1** and committed to `spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl` (SHA-256-pinned per Story 0.3); Story 7.5b is the single CONSUMER for the NFR-Onb-1 gate execution — no other story authors this corpus
**And** halt-recall is ≥0.90 on the calendar-conflict subset
**And** halt-precision is ≥0.85 overall
**And** bmad-eval baseline ≥0.85 is met
**And** Butler latency: conversational <400ms P95 / IPC <60ms (§13.1 J0)

**Given** the morning-digest path (FR17 Spirit-side)
**When** Butler is queried on the director's first session of the day
**Then** the digest contains (a) tasks completed in last 24h with outcome tags, (b) open halts requiring resolution, (c) flagged anomalies with confidence ≥0.6, (d) trust-bar reflecting yesterday's predicate-fire rate
**And** the digest cites source log refs for all claimed completions
**And** hallucination floor: 0/100 hallucinated tasks across the digest corpus (verified against actual Transparency Log)
**And** ≥95/100 digests include all open halts

**Given** Butler is the Spirit driving NFR-Onb-1 v0.3 gate (E7 Story 7.5)
**When** the 30-Min First Spirit Gate runs
**Then** Butler-class corpus is the proving suite
**And** Butler ships zero kernel KLOC (subprocess Spirit form)

### Story 8.2: Ship the Researcher Reference Spirit with Distillation Pattern and `log.recall` Walker

As a v0.5 substrate user,
I want the Researcher reference Spirit shipped with the distillation pattern as a canonical example, a `log.recall` walker selecting which Transparency Log frames to preserve, Spirit-side LLM compression with kernel-enforced I11 audit chain, AND scalar.tap subscription,
So that the v0.5 distillation primitives are demonstrably composable and the 5-metric distillation gate (NFR-Aud-7) has its primary reference implementation.

**Acceptance Criteria:**

**Given** the Researcher reference Spirit in `spirits/researcher/`
**When** Researcher is loaded with a corpus to distill
**Then** the Spirit calls `log.recall(filter, limit, cursor)` to walk the Transparency Log
**And** the walker is participant-scoped per E4 Story 4.4

**Given** Researcher writes a distillate
**When** the kernel processes the digest write
**Then** the digest includes `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage` (I11 audit chain)
**And** missing audit chain elements cause `EDigestAuditChainMissing`

**Given** the five-metric distillation gate (NFR-Aud-7) measured against Researcher
**When** the eval corpus runs
**Then** digest-recall ≥0.90 / faithfulness ≥0.98 / hedge-preservation ≥0.95 / traceability 100% / secret-leakage 0%
**And** all five metrics are reported per quarterly N=500 corpus (NFR-Aud-8)

**Given** Researcher subscribes to `scalar.tap`
**When** scalars are written by other Spirits
**Then** Researcher receives the stream and can include patterns in subsequent digests
**And** Researcher contributes the morning digest at v0.5+ phase (extending Butler's v0.3 implementation)

**Given** Researcher latency: J-Researcher workload <100ms P95 distillation step on the §13.1 bench
**When** the benchmark runs
**Then** the per-journey latency budget is met
**And** budget overruns emit `BudgetWarning` (NFR-Perf-6)

### Story 8.3: Observer v0.5 — Telemetry Stream Subscriber + Pre-Halt Scalar Drift Watchdog

As an operator at v0.5 watching for pre-halt instability,
I want the Observer reference Spirit shipped as a broad telemetry-stream subscriber that watches `scalar.tap` for pre-halt drift AND emits structural-anomaly events (sandbox-escape syscall pattern divergence, fd-table growth, unexpected outbound IAC connections),
So that the "kernel raises structural alarm; interpretation is Spirit-side" pattern is operationalized — and the kernel itself remains non-interpretive.

**Acceptance Criteria:**

**Given** the Observer reference Spirit in `spirits/observer/`
**When** Observer is loaded
**Then** the Spirit subscribes broadly to the Telemetry Stream including `scalar.tap`
**And** the subscription is filtered to events under Observer's principal namespace per FR31

**Given** Observer watches `scalar.tap` for drift
**When** a Spirit's scalar value approaches its `[epistemic_policy]` threshold before firing
**Then** Observer detects the drift and emits an early-warning event
**And** the operator can intervene before the halt fires

**Given** Observer detects sandbox-escape structural anomalies
**When** syscall pattern divergence from manifest declaration / fd-table growth / unexpected outbound IAC connections occur
**Then** Observer emits a `structural_anomaly_suspect` IAC frame (NFR-Sec-3 v2.0 surfaces become operator-actionable here)
**And** the *interpretation* of malice is Observer-side or operator-side, never kernel-side (§4.0.7)

**Given** the kernel-API surface invariant test (Story 0.2)
**When** Observer's structural-anomaly logic is added
**Then** the logic lives in Observer's Spirit code, not in `maos-kernel-core`
**And** the kernel-API does not gain anomaly-classification functions (would be class `other` → build-break)

### Story 8.4: Ship the Founder-Loop Wedge Spirits — Orchestrator, Workers, Architect, Reviewer

As a founder running a v0.8/v0.9 overnight loop,
I want the Orchestrator + Worker + Architect + Reviewer reference Spirits shipped together as the founder-loop wedge demo, with Orchestrator buffering instructions at safe sequence points, distillate-fed dispatch (not raw output), Worker = wrapped CLI agent via CliWrapperSpirit, AND the halt-and-resume-overnight pattern,
So that the v0.8 wedge demo is real — the founder assigns an overnight task at 11pm and finds an audit-traced result at 7am.

**Acceptance Criteria:**

**Given** the Orchestrator reference Spirit in `spirits/orchestrator/`
**When** Orchestrator receives buffered instructions from the director (FR20 via E3 Story 3.4)
**Then** Orchestrator processes them at safe sequence points between Worker task completions
**And** Orchestrator never preempts in-flight delegations

**Given** Orchestrator dispatches to Worker Spirits
**When** Worker output is produced
**Then** Orchestrator distills the output via the E4 Story 4.4 path before subsequent dispatch
**And** subsequent dispatches receive distillates, not raw output (FR21)
**And** the founder-loop wedge demo passes with halt-and-resume-overnight: 11pm assign → distillate dispatch overnight → 7am digest cites actual log refs

**Given** Worker = wrapped CLI agent
**When** Worker invokes `claude code` / `opencode` / `gemini-cli` / `kimi-cli` via CliWrapperSpirit (E6 Story 6.2)
**Then** stdout/stderr captured to Transparency Log with provenance
**And** capability-token authority used is journaled
**And** `output_shape_version` mismatch fails loud per FR40

**Given** Architect and Reviewer reference Spirits for the code-review loop
**When** the founder-loop wedge demo runs
**Then** Architect proposes design → Reviewer critiques → distillate flows through Orchestrator → halt-and-resume preserves work across overnight pause/resume

**Given** the J1 latency budget (Founder-loop CliWrapper IPC <25ms P95 per §13.1)
**When** the founder-loop benchmark runs
**Then** the budget is met or §13.1 measurement triggers rust-inproc evaluation in E5 Story 5.5

### Story 8.5: Ship the Mira+Nash Diagnostic-Architect Bilateral Pair with Safety-Critical Corpus

As a v1.5 operator deploying a diagnostic-architect bilateral 2-Host pair,
I want Mira on Host A (prod-edge) + Nash on Host B (dev-environment) coordinating over A2A cross-Host with pre-paired mTLS cert fingerprints, mobile push to operator on halt, AND a safety-critical corpus methodology N≥150 with inter-annotator agreement κ≥0.7,
So that the v1.5 release ships the bilateral-pair user journey (J4) as a working, audit-traced, safety-critical reference deployment.

**Acceptance Criteria:**

**Given** Mira and Nash reference Spirits in `spirits/mira/` and `spirits/nash/`
**When** Mira on Host A and Nash on Host B are deployed
**Then** both Hosts have each other's mTLS cert fingerprints in deployment configuration (no discovery)
**And** A2A cross-Host (E6 Story 6.3) connects with TOFU pinning verified

**Given** J4 latency budget: Mira-Nash Observer colocation <10ms P95 (§13.1)
**When** the J4 benchmark runs
**Then** colocation latency is within budget
**And** budget overruns emit `BudgetWarning`

**Given** a halt fires on Mira (e.g., prod-edge anomaly)
**When** the kernel dispatches halt notification
**Then** the notification routes to mobile push (operator's configured channel)
**And** Nash on Host B is informed via A2A typed-intent consent (ADR-012)
**And** the director can resolve the halt via E3 Story 3.3's three-tap flow

**Given** the safety-critical Spirit corpus methodology
**When** Mira+Nash corpora are authored
**Then** corpus N≥150 scenarios per Spirit
**And** inter-annotator agreement κ≥0.7 is verified across ≥2 annotators
**And** the methodology is documented in `docs/safety-critical-corpus-methodology.md`

**Given** J6 cold-start budget (Diego cold-start <500ms per §13.1)
**When** a Mira or Nash Spirit is cold-loaded
**Then** the cold-load completes within 500ms
**And** the budget is reported per release

---

### Epic 9: Audit & Compliance Surfaces + Operator Productionization (v0.5 → v1.0)

**Goal:** DPO, CISO, external regulator, and operator can query, export, forget, attribute cost, and prove substrate state. Substrate as a hermes-tenant — uninstall is a real, externally-verifiable guarantee. Single epic with two internal concerns (Winston's split, John yielded).

**Internal Concern A — Audit & Compliance Rail (legal-facing):**

- `maosctl audit subject-access --principal <id>` (FR42): returns all principal-namespace entries across all Spirits with provenance (Spirit, time, derived-from observations).
- `maosctl audit posture-delta --range=<timespan>` (FR43): capability-scope changes + sandbox-tier changes + consent-policy changes with approval-chain attribution.
- `maosctl audit sealed-export <bundle-spec>` (FR44): Ed25519-signed by operator audit key; third-party-verifiable; conforms to `maos.audit-bundle.v1` schema; includes both working-memory digest refs (I12) AND distilled-output content (I11).
- `maosctl forget --principal <id> [--reason <legal-hold>]` (FR45): GDPR Article 17 right-to-be-forgotten with cross-Spirit cascade (forgetting cascades to working-memory references in other Spirits; distillates containing principal data marked redacted with re-distillation triggered); 50/50 clean removal + 50/50 redaction-marker in immutable log + 0 leakage in 100 follow-up subject-access queries.
- `journal.export(filter, redaction_policy)` (FR46, ADR-023): `maos.trajectory.v1` schema with Ed25519 signing and applied-redaction flag.
- Frame-by-frame log query (FR41): authenticated audit interface with filters by Spirit / capability / time-range / frame-kind / tag; P99 ≤2s single-Spirit on 30-day window / ≤10s global; ≥98/100 events recoverable on per-commit log-completeness corpus.
- Deterministic replay (ADR-028): over **shape of the trace** (IAC frame ordering, capability-token issuances, halt events, decision-frame emission), NOT redacted payload content; redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders; `schemas/trace-shape.schema.json` JSON Schema draft-2020-12 validated in CI.
- Governance audit-queryable artifacts (FR62): vetter-key admission and rotation events / ABI-extension proposals and ratification status / ComplianceClaim schema versions and effective dates.
- Proof-of-erasure record on Spirit uninstall (FR65): enumerates all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations); externally-verifiable Merkle inclusion + exclusion proof.

**Internal Concern B — Operator Productionization (ops-facing):**

- Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` (FR63): CI-enforced metadata per error variant (code / severity / recovery-class / owner / kernel-or-spirit / since-version); `cargo run --bin error-metadata-check` exits non-zero if any variant missing any field; catalog covers 14+ named typed errors.
- Cost attribution per Spirit per task per principal in Transparency Log (FR64) — enterprise-readiness gate; ≥98% reconciliation against provider billing sampled monthly (NFR-Cost-1).
- Pre-built binaries (Linux amd64/arm64, macOS arm64) via GitHub Releases with SHA256 + Ed25519 verification mandatory (v0.5); Homebrew tap, AUR, deb, rpm (v1.0); container images Docker Hub / GHCR (v1.0).
- Transparency Log backup/DR (NFR-Ops-9): RPO ≤1h, RTO ≤4h, backup integrity verified weekly via Merkle-root cross-check.
- Air-gapped deployment validation (NFR-Ops-12): substrate boots/runs/produces transparency-log entries with zero outbound network calls; structural test in CI via network-namespace isolation.
- Multi-operator tenancy primitive-reservation (NFR-Ops-11 v1.0; full impl v1.5+ in E10): per-operator namespace + transparency-log shard + capability-token signing key + GDPR-erasure scope declared as primitive-reserved so v0.5 grammar lock doesn't paint into a corner.
- Region-pinning primitive (PIPL §40 / data localization — NFR-Comp-4): Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication.
- Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent — NFR-Comp-5): covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission.
- **5 canonical doc deliverables** (NFR-Doc-4) with CI-verifiable minima: manifest schema reference (≥1 example per field) at `docs.maos.dev/manifest/<version>/`; pattern cookbook (≥10 patterns) at `docs.maos.dev/cookbook/`; migration runbooks at `docs.maos.dev/migrate/`; troubleshooting guide (covers 100% of FR63 catalog) at `docs.maos.dev/troubleshoot/`; deployment topology guide at `docs.maos.dev/deploy/`.
- API reference site (NFR-Doc-3) at `docs.maos.dev/abi/<version>/`: versioned, searchable, deep-linkable, archived ≥2 minor versions.
- WCAG AA compliance for doc site (NFR-Doc-5).
- **Korean i18n v1.0** (NFR-Doc-6 — Japanese + Chinese-simplified at v1.5 in E10); LOCALES.md with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- Doc tooling (NFR-Doc-7): per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown; mdBook+i18n / Docusaurus / VitePress decision by v0.5; v1.0 in production.
- Onboarding artifacts (NFR-Ops-6): `RFC_TEMPLATE.md` v0.8, `GOVERNANCE.md` v0.5 basic + v0.8 locked, `CODE_OF_CONDUCT.md` v0.5, `LOCALES.md` v1.0, `TRADEMARK.md` v1.0, `BREAKING.md` v1.0.
- Sustainability vehicle (NFR-Ops-7): Open Collective declared-intent v0.5; legal/fiscal-sponsor work v0.8.
- Substrate-self compliance scope declaration (NFR-Comp-3): `STABILITY.md` scope-disclaimer that SOC 2 / ISO 27001 / FedRAMP scope is operator's responsibility.
- Trust-anchor framing carry-forward decision (NFR-Ops-8): published ADR by v0.3 declaring committed competitive framing (substrate-as-substrate vs substrate-as-trust-anchor).
- SIEM export (NFR-Aud-11) at v2.0; OpenTelemetry adapter at v1.0 SLO-class.

**FRs covered:** FR41, FR42, FR43, FR44, FR45, FR46, FR62, FR63, FR64, FR65, FR48 partial (FIPS readiness gate).

**Key NFRs:** NFR-Perf-5 (audit query latency), NFR-Aud-1 through NFR-Aud-6 (capability introspection / drift detection / deterministic replay / audit retention 90d / right-to-explanation / sealed-export), NFR-Aud-10 (GDPR 50-scenario corpus), NFR-Aud-11 (SIEM/OTel), NFR-Aud-12 (storage cascade erasure + externally-verifiable uninstall receipt), NFR-Aud-13 (erasure SLA 95% within 30 days, configurable 7 days enterprise), NFR-Aud-14, NFR-Comp-3, NFR-Comp-4, NFR-Comp-5, NFR-Cost-1, NFR-Doc-1 through NFR-Doc-7, NFR-Ops-1 through NFR-Ops-12, NFR-Tenancy-1.

**Corpora authored in E9:**
- GDPR Art. 17 cross-Spirit cascade 50-scenario corpus.
- Per-commit log-completeness corpus N=100 injected events.
- Trace-shape schema validation corpus.

**Acceptance demo:** DPO runs `maosctl audit subject-access --principal alice@example.org` — returns all entries across all Spirits in <2s; sealed-export bundle verifies on third-party machine; GDPR forget cascades + 0 leakage in 100 follow-up queries; cost reconciliation ≥98% against provider billing; air-gapped CI run passes; Korean-localized docs render with deep-link preservation.

#### Stories

### Story 9.1: Ship `maosctl audit` Subcommands — Query, Subject-Access, Posture-Delta, Sealed-Export

As a DPO / CISO / external regulator,
I want `maosctl audit query` for frame-by-frame queries with filters (FR41), `maosctl audit subject-access --principal <id>` returning all principal-namespace entries with provenance (FR42), `maosctl audit posture-delta --range=<timespan>` for capability/sandbox/consent-policy changes with approval-chain attribution (FR43), AND `maosctl audit sealed-export <bundle-spec>` producing Ed25519-signed third-party-verifiable bundles (FR44),
So that legal-facing queries are first-class operations with audit-grade latency floors and signed export bundles.

**Acceptance Criteria:**

**Given** `maosctl audit query --spirit <id> --range <timespan> --frame-kind <kind> --tag <tag>`
**When** the query runs on a 30-day window scoped to a single Spirit
**Then** P99 latency is ≤2s (NFR-Perf-5)
**And** for global queries (no Spirit filter): P99 ≤10s
**And** the log-completeness corpus (N=100 injected events) shows ≥98/100 events recoverable (NFR-Aud-1)

**Given** `maosctl audit subject-access --principal alice@example.org` (FR42)
**When** the query runs across all Spirits
**Then** the result enumerates every entry under `principal:alice@example.org:*` across all Spirits
**And** each entry carries provenance: Spirit id, time, derived-from observations
**And** completion within the latency floor

**Given** `maosctl audit posture-delta --range=<timespan>` (FR43)
**When** the query runs
**Then** the result surfaces capability-scope changes, sandbox-tier changes, consent-policy changes
**And** each change has approval-chain attribution from the Approval Decision Log

**Given** `maosctl audit sealed-export <bundle-spec>` (FR44)
**When** the operator generates a sealed-export
**Then** the bundle is Ed25519-signed by the operator's audit key
**And** the bundle is third-party-verifiable
**And** the bundle conforms to `maos.audit-bundle.v1` schema
**And** the bundle includes both working-memory digest refs (I12) AND distilled-output content (I11)
**And** corpus tier validation: signed-export tier at v1.0 (NFR-Aud-6)

### Story 9.2: Execute GDPR Article 17 Cascade with Deterministic Replay and Proof-of-Erasure

As a regulator enforcing GDPR Article 17,
I want `maosctl forget --principal <id>` (FR45) performing cross-Spirit cascade with 50/50 + 0/100-leakage floors, `journal.export(filter, redaction_policy)` (FR46), deterministic replay (ADR-028) over trace-shape (not redacted payload), AND proof-of-erasure record on Spirit uninstall (FR65) with externally-verifiable Merkle inclusion/exclusion proof,
So that the substrate-uninstall guarantee is a real proof, not a hope, and replay determinism is anchored at v1.0 best-effort / v1.5 hard target.

**Acceptance Criteria:**

**Given** `crates/maos-cli/src/cmd/forget.rs` implementing `maosctl forget --principal <id> [--reason <legal-hold>]` (FR45)
**When** the command dispatches `crates/maos-audit/src/gdpr/cascade.rs::run_forget(principal_id, reason)`
**Then** all `principal:<principal_id>:*` entries are removed across all Spirit private tiers (Story 4.3's Memory Manager)
**And** the deletion event itself is journaled to `crates/maos-audit/src/journal.rs::write_gdpr_event` (preserving lifecycle invariant — the act of forgetting is recorded)
**And** principal data is gone from queryable surfaces
**And** cross-Spirit cascade: distillates containing principal data are marked redacted in `crates/maos-audit/src/i11_chain.rs::mark_redacted` with re-distillation triggered downstream

**Given** the 50-scenario GDPR Art. 17 cross-Spirit cascade corpus (NFR-Aud-10) at `crates/maos-audit/tests/fixtures/gdpr-cascade-v0/`
**When** `cargo test -p maos-audit -- test_gdpr_art17_cascade` runs
**Then** 50/50 scenarios show clean removal at queryable surface (verified via Story 9.1's subject-access query)
**And** 50/50 scenarios show redaction-marker present in immutable Transparency Log
**And** 0 leakage in 100 follow-up subject-access queries from a separate fixture
**And** time-to-erasure: 95% within 30 days (configurable to 7 days for enterprise tier in `config.toml`) per NFR-Aud-13
**And** audit log entry within 24h of request acceptance (timed in `crates/maos-audit/tests/erasure_sla_test.rs`)

**Given** `journal.export(filter, redaction_policy)` per ADR-023 (FR46) implemented in `crates/maos-cli/src/cmd/audit_export.rs`
**When** the operator exports a filtered trajectory
**Then** the bundle conforms to `maos.trajectory.v1` schema defined in `schemas/trajectory.schema.json`
**And** the bundle is Ed25519-signed via Story 1a.3's `CryptoProvider` with applied-redaction flag
**And** redaction policy is honored end-to-end, verified by `crates/maos-audit/tests/trajectory_redaction_test.rs`

**Given** deterministic replay (ADR-028, NFR-Aud-3) over trace-shape implemented in `crates/maos-audit/src/replay/`
**When** `crates/maos-audit/src/replay/runner.rs::replay(bundle)` executes against a sealed-export bundle
**Then** replay determinism is verified over IAC frame ordering, capability-token issuances, halt events, and decision-frame emission
**And** redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders generated by `crates/maos-audit/src/replay/redaction_placeholder.rs`
**And** `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12) validates the replay in CI via `crates/maos-audit/tests/replay_schema_test.rs`
**And** v1.0 best-effort target; v1.5 hard target

**Given** Spirit uninstall (FR65) via `crates/maos-cli/src/cmd/uninstall_spirit.rs`
**When** the operator runs `maosctl uninstall <spirit>`
**Then** `crates/maos-audit/src/erasure/proof.rs::emit_proof_of_erasure(spirit_id)` enumerates all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations)
**And** the record carries signed Merkle inclusion + signed Merkle exclusion proof generated by `crates/maos-audit/src/erasure/merkle.rs` (NFR-Aud-12)
**And** the proof is retained independent of the substrate at `~/.local/share/maos/erasure-proofs/<spirit_id>-<timestamp>.bundle` (third-party-verifiable via the published `tools/verify-erasure/` toolchain shipped at v1.0)
**And** 100% of registered storage backends prove erasure within bounded window — tested in `crates/maos-audit/tests/multi_backend_erasure_test.rs`
**And** the proof is retained independent of the substrate (third-party verifiable)
**And** 100% of registered storage backends prove erasure within bounded window

### Story 9.3: Publish the Typed Error Catalog + Governance Audit Artifacts and Wire Cost Attribution

As an enterprise operator,
I want the typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` with CI-enforced metadata for every error variant (FR63), governance audit-queryable artifacts surfacing vetter-key admission / ABI-extension proposals / ComplianceClaim schema versions (FR62), AND cost attribution per Spirit per task per principal with ≥98% reconciliation against provider billing (FR64 + NFR-Cost-1),
So that v1.0 is production-grade for enterprise: errors are diagnosable, governance is auditable, costs are attributable.

**Acceptance Criteria:**

**Given** the typed error catalog (FR63)
**When** any kernel-emitted error is raised
**Then** the error carries a stable typed code from the published catalog
**And** the catalog covers all 14+ named typed errors documented in architecture-maos-minimal-opus.md
**And** each variant has 6 CI-enforced metadata fields: code / severity / recovery-class / owner / kernel-or-spirit / since-version (NFR-Doc-2)
**And** `cargo run --bin error-metadata-check` exits non-zero if any variant is missing any field

**Given** the documentation site at `docs.maos.dev/errors/<ERR_NAME>`
**When** any error code is documented
**Then** the URL renders 200 with retryability + cause-chain semantics + version-stability guarantees
**And** consistency with the LTS policy is enforced (no breaking error-code changes within an LTS cycle)

**Given** governance audit-queryable artifacts (FR62)
**When** the kernel processes governance events
**Then** vetter-key admission and rotation events are journaled and queryable
**And** ABI-extension proposals and their ratification status are journaled
**And** ComplianceClaim schema versions and their effective dates are journaled
**And** all three artifact streams are exposed via `maosctl audit query --kind governance`

**Given** cost attribution per Spirit per task per principal (FR64)
**When** the kernel records external-call costs in the Transparency Log
**Then** token-spend per provider, subprocess CPU-time, storage I/O are attributed to the originating Spirit + task + principal
**And** an enterprise operator can produce a per-tenant cost report

**Given** monthly reconciliation against provider billing (NFR-Cost-1)
**When** the operator runs `maosctl audit cost-reconcile --month <YYYY-MM>`
**Then** reconciliation accuracy is ≥98% against provider billing statements
**And** discrepancies are flagged for investigation

### Story 9.4: Productionize the Operator Surface — Distribution, Backup/DR, Air-Gap, Region-Pinning

As an enterprise operator deploying MAOS to production,
I want pre-built binaries (Linux amd64/arm64, macOS arm64) via signed GitHub Releases (v0.5) progressing to Homebrew/AUR/deb/rpm/container images (v1.0), Transparency Log backup/DR with RPO ≤1h / RTO ≤4h, air-gapped deployment validation in CI, region-pinning primitive (PIPL §40), Spirit model-provenance manifest field (SB-1047), AND multi-operator tenancy primitive-reservation (v1.0 reserved, v1.5+ implemented),
So that v1.0 ships to production-tenant operators without ad-hoc deployment glue.

**Acceptance Criteria:**

**Given** pre-built binaries at v0.5
**When** the operator runs `maosctl install` from a GitHub Releases artifact
**Then** the artifact has SHA256 + Ed25519 verification mandatory (FR1)
**And** Linux amd64/arm64 + macOS arm64 binaries are signed and published

**Given** package manager distribution at v1.0
**When** the operator installs via Homebrew tap, AUR, deb, or rpm
**Then** install succeeds with the same signature verification
**And** container images on Docker Hub / GHCR pass the same verification
**And** Windows binary lands at v1.5 (E10 Story 10.5)

**Given** Transparency Log backup/DR (NFR-Ops-9)
**When** backup runs
**Then** RPO ≤1h, RTO ≤4h
**And** backup integrity verified weekly via Merkle-root cross-check
**And** restore drill is documented and tested

**Given** air-gapped deployment validation (NFR-Ops-12)
**When** the substrate boots in network-namespace-isolated CI
**Then** the substrate boots, runs, and produces transparency-log entries with zero outbound network calls
**And** the structural test is enforced in CI
**And** documented Spirit-author guidance for air-gapped capability tokens

**Given** region-pinning primitive (NFR-Comp-4 / PIPL §40)
**When** the operator configures region pinning
**Then** Transparency Log + working-memory store are pinned to a single jurisdictional region
**And** cryptographic enforcement prevents cross-region replication
**And** any attempt to cross-region replicate fails with `ERegionViolation`

**Given** Spirit model-provenance manifest field (NFR-Comp-5 / SB-1047)
**When** a Spirit declares `[model_provenance]` with `covered_model_id`, `training_data_lineage`, `last_eval_timestamp`
**Then** the substrate validates field presence at admission
**And** missing or stale provenance is rejected

**Given** multi-operator tenancy primitive-reservation (NFR-Ops-11 v1.0)
**When** the namespace grammar reserves multi-operator primitives
**Then** per-operator namespace, per-operator transparency-log shard, per-operator capability-token signing key, per-operator GDPR-erasure scope are declared as primitive-reserved
**And** the grammar lock (NFR-Test-11) doesn't paint future implementations into a corner
**And** full implementation arrives v1.5+

### Story 9.5: Publish Five Canonical Docs with WCAG AA, Korean i18n, and Onboarding Artifacts

As a v1.0 substrate published to the world,
I want the 5 canonical doc deliverables (manifest schema reference + pattern cookbook + migration runbooks + troubleshooting + deployment topology) with WCAG AA compliance + Korean i18n + onboarding artifacts (RFC_TEMPLATE.md / GOVERNANCE.md / CODE_OF_CONDUCT.md / LOCALES.md / TRADEMARK.md / BREAKING.md) AND the trust-anchor framing carry-forward ADR published by v0.3,
So that the documentation surface is real, accessible, localized, and the competitive framing decision is locked in before v1.0.

**Acceptance Criteria:**

**Given** the 5 canonical doc deliverables (NFR-Doc-4)
**When** the doc site is built
**Then** each URL renders 200: `docs.maos.dev/manifest/<version>/` (≥1 example per field) / `docs.maos.dev/cookbook/` (≥10 patterns) / `docs.maos.dev/migrate/` / `docs.maos.dev/troubleshoot/` / `docs.maos.dev/deploy/`
**And** the troubleshooting guide covers 100% of FR63 error catalog
**And** the API reference at `docs.maos.dev/abi/<version>/` is versioned, searchable, deep-linkable, archived ≥2 minor versions back (NFR-Doc-3)

**Given** WCAG AA compliance (NFR-Doc-5)
**When** the doc site is audited
**Then** WCAG AA conformance is verified for color contrast, keyboard navigation, screen reader support
**And** automated accessibility tests run in CI

**Given** Korean i18n at v1.0 (NFR-Doc-6)
**When** the doc site is built with `--lang ko`
**Then** Korean translations render with deep-link preservation
**And** the LOCALES.md glossary lock applies — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes
**And** Japanese + Chinese-simplified land at v1.5 (E10 Story 10.5)
**And** RTL layout deferred to v2.5

**Given** doc tooling (NFR-Doc-7)
**When** the operator builds docs
**Then** per-locale builds work with fallback to English
**And** language switcher preserves deep-link
**And** version dropdown switches between archived ABI versions
**And** mdBook + i18n / Docusaurus / VitePress decision is made by v0.5; in production by v1.0

**Given** onboarding artifacts (NFR-Ops-6)
**When** v1.0 ships
**Then** `RFC_TEMPLATE.md` (v0.8) / `GOVERNANCE.md` (v0.5 basic + v0.8 locked) / `CODE_OF_CONDUCT.md` (v0.5) / `LOCALES.md` (v1.0) / `TRADEMARK.md` (v1.0) / `BREAKING.md` (v1.0) all exist at the repo root
**And** each artifact is referenced from the doc site

**Given** sustainability vehicle (NFR-Ops-7)
**When** v1.0 ships
**Then** Open Collective declared-intent is published (v0.5)
**And** legal/fiscal-sponsor work is initiated (v0.8)

**Given** trust-anchor framing carry-forward ADR (NFR-Ops-8)
**When** v0.3 release approaches
**Then** a published ADR declares which competitive framing is committed (substrate-as-substrate vs substrate-as-trust-anchor)
**And** absence of this ADR is a v0.3 release-block
**And** STABILITY.md contains the substrate-self compliance scope clause (NFR-Comp-3) — SOC 2 / ISO 27001 / FedRAMP scope is operator's responsibility

**Given** OpenTelemetry adapter at v1.0 SLO-class (NFR-Aud-11)
**When** the operator configures OTel export
**Then** structured trace IDs and span linkage are exported per IAC frame, capability invocation, halt event
**And** SIEM export lands at v2.0 (NFR-Aud-11 second phase)

---

### Epic 10: v1.0 Ship Gate + v1.5 Collective Tier (v1.0 → v1.5)

**Goal:** v1.0 release certification (pen-test + CCAC + HSIS + third-party trial + cross-form equivalence) AND v1.5 collective tier maturation (Postgres+pgvector Loom-lite, SQLite→Postgres migration, Mira+Nash diagnostic-architect bilateral pair, skill-format conformance, JetBrains plugin-bridge, Windows binary, 2-year LTS commitment). Two phase sub-clusters in one epic (user's selected option).

**Sub-cluster A — v1.0 Ship Gate (coordination + final certification gates):**

- **Pen-test report zero P0/P1 (NFR-Sec-7)**: external pen-test; triage by joint panel of pen-test lead + MAOS security owner; disagreements escalate to PRD-author tiebreak; P0/P1 definitions per OWASP Risk Rating Methodology frozen at engagement start.
- **CCAC N=600 cross-validation execution** (NFR-Aud-9 gate — corpus authored E7): ≥3 reference Spirits, agreement within ±2%.
- **HSIS ≥95% verification** (NFR-Rel-3 gate — corpus authored E5): 6 class-specific corpora × 50 scenarios; zero invariant violations CVSS-7 class.
- **Third-party trial N=12 stratified execution** (NFR-Test-8): ≥4 no prior MAOS contribution / ≥3 never written Rust Spirit / ≥2 never written Rust at all / ≥2 non-English-native / ≥1 working offline-only. 14-day no-DM-support window. Floor ≥10/12 produce working signed Spirit binary that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85. Wilson CI [0.552, 0.962] meaningful at N=12. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot.
- **Cross-form semantic equivalence** (NFR-Test-7) — if rust-inproc form active per E5 §13.1 measurement gate: rust-inproc ↔ subprocess ≥90%; CLI-wrapper distributional via Mann-Whitney U-test p>0.05 over 30 runs.
- **STABILITY.md publication** (NFR-Maint-4): live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- BREAKING.md publication (NFR-Maint-7).
- 1-year LTS clock starts (NFR-Maint-6 v1.0).
- Korean localization shipped (NFR-Doc-6 v1.0).
- Export-control classification artifact (NFR-Comp-1): ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel.
- CNA registration through MITRE (per NFR-Ops-4).
- Adversarial-Spirit red-team **80-scenario corpus authoring + gate** (NFR-Sec-10): 8 attack classes × N=10 — capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse. Floor ≥9/10 per class, ≥72/80 aggregate, 0 unmitigated category. Authored by external pen-tester using published ABI. Phase per PRD = v1.5, but corpus authoring can begin v1.0 prep.

**Sub-cluster B — v1.5 Collective Tier:**

- Postgres+pgvector Loom-lite (collective tier of memory via MCP-Streamable-HTTP — kernel mediates access; Loom is user-space per ADR-006 / NFR-Test-9).
- **SQLite→Postgres migration test corpus** (NFR-Ops-10): forward-migration on 10⁶-row corpus; byte-identical Merkle-root preservation post-migration; rollback path tested. **v1.4 gate (gates v1.5).**
- Mira + Nash diagnostic-architect bilateral pair v1.5 (overlap with E8 Mira+Nash sub-story; coordinate ownership): pre-paired mTLS cert fingerprints; mobile push to operator; J4 latency budget <10ms P95.
- Skill-format conformance (NFR-Test-10): ≥1 third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification.
- JetBrains plugin-bridge for ACP (NFR-Ops adjacent).
- Windows binary v1.5.
- 2-year LTS commitment v1.5 (NFR-Maint-6).
- Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5; RTL deferred to v2.5).
- mTLS cert rotation chaos test 3-host v1.5 / 10-host v2.0 (NFR-Sec-13 full).
- Manifest parser fuzz 24h `cargo-fuzz` (NFR-Sec-5) + wire protocol adversarial-input fuzz 24h (NFR-Sec-6) — v1.0 ship gates.
- ComplianceClaim envelope schema migration validation (test continuity through v1.5).

**FRs covered:** Closes any FR not closed in earlier epics; v1.5-phase implementations of FR42 (subject-access at Postgres scale), FR44 (sealed-export with Loom-lite content), FR58 v1.5 reference Spirit cohort (Mira+Nash overlap with E8).

**Key NFRs (this is the densest NFR-execution epic):**
- v1.0 gates: NFR-Sec-5 (manifest fuzz 24h zero crashes), NFR-Sec-6 (wire fuzz 24h zero crashes), NFR-Sec-7 (pen-test zero P0/P1), NFR-Sec-10 (adversarial red-team v1.5), NFR-Aud-9 (CCAC N=600 execution), NFR-Rel-3 (HSIS ≥95% execution), NFR-Test-1 (corpus discipline at v1.0), NFR-Test-5 (FKCS infrastructure v2.0 deferred; lay groundwork), NFR-Test-7 (cross-form equivalence — if rust-inproc active), NFR-Test-8 (third-party trial N=12 execution), NFR-Test-10 (skill-format conformance v1.5), NFR-Maint-6 (1-year LTS v1.0; 2-year v1.5), NFR-Comp-1 (export-control), NFR-Doc-6 (Korean v1.0; Japanese+CN-S v1.5).

**Corpora authored in E10:**
- **Adversarial red-team 80-scenario corpus** (generator: 8 attack classes × 10 canonical scenarios × 8× expansion via parameter variation = 640 effective items per Murat's generator discipline) — P0 ship-block if any false negative.
- SQLite→Postgres migration corpus 10⁶ rows.
- Skill-format conformance corpus ~50 cases.
- mTLS cert rotation chaos scenarios (3-host v1.5).
- Manifest parser fuzz inputs (24h cargo-fuzz).
- Wire protocol adversarial fuzz inputs (24h).

**Acceptance demos:**
- **v1.0 ship gate passes:** pen-test report zero P0/P1; CCAC ≥27/30 per-class with ≥3 Spirits ±2% agreement; HSIS ≥95% across 6 corpora × 50 scenarios; N=12 third-party trial ≥10/12 succeed; STABILITY.md + BREAKING.md published; 1-year LTS clock starts; Korean docs live; export-control letter on file.
- **v1.5 collective tier:** Mira on Host A + Nash on Host B coordinate over A2A; SQLite→Postgres migration on 10⁶-row corpus produces byte-identical Merkle root; skill-format adapter executes ≥1 third-party skill without kernel modification; Windows binary builds on CI; 2-year LTS commitment.

#### Stories

### Story 10.1: Execute the v1.0 Release Gate — Pen-Test, CCAC Cross-Validation, HSIS Verification

As a substrate release manager certifying v1.0,
I want the v1.0 ship-gate coordination story: external pen-test report zero P0/P1 (NFR-Sec-7), CCAC N=600 cross-validation against ≥3 reference Spirits with agreement ±2% (NFR-Aud-9 — corpus authored E7), HSIS ≥95% verification against the 6×50=300 scenario corpus (NFR-Rel-3 — corpus authored E5), AND STABILITY.md + BREAKING.md publication with 1-year LTS clock start,
So that v1.0 release is gated by mechanically-verifiable evidence, not aspirational claims.

**Acceptance Criteria:**

**Given** an external pen-test engagement
**When** the pen-test concludes
**Then** the report shows zero P0/P1 findings open at v1.0 ship (NFR-Sec-7)
**And** triage is performed by a joint panel of pen-test lead + MAOS security owner
**And** disagreements escalate to PRD-author tiebreak
**And** P0/P1 definitions per OWASP Risk Rating Methodology are frozen at engagement start

**Given** the CCAC corpus N=600 (authored E7 Story 7.3)
**When** the corpus runs cross-validation against ≥3 reference Spirits
**Then** per-class floor ≥27/30 passes per Spirit
**And** cross-Spirit agreement is within ±2%
**And** 100/100 context-drift claims are rejected at admission
**And** failure on this gate is a P0 ship-block

**Given** the HSIS corpus 6×50=300 scenarios (authored E4+E5)
**When** the HSIS gate runs
**Then** ≥95% pass per Spirit class (NFR-Rel-3)
**And** zero invariant violations CVSS-7 class
**And** stratified swap-lifecycle phase distribution is verified

**Given** STABILITY.md publication (NFR-Maint-4 — content authored E7 Story 7.5)
**When** v1.0 release is cut
**Then** STABILITY.md is published with live `(kernel, abi, manifest_schema)` compatibility matrix
**And** 1-year LTS commitment clock starts (NFR-Maint-6)
**And** BREAKING.md (NFR-Maint-7) is grep-enforced — every breaking change requires an entry with migration steps

### Story 10.2: Run the Third-Party Trial N=12 and Adversarial Red-Team Gate at v1.0

As a substrate quality lead at v1.0,
I want the black-box third-party trial with N=12 stratified humans executed (NFR-Test-8), cross-form semantic equivalence verified IF rust-inproc form is active per E5 §13.1 measurement (NFR-Test-7), AND the adversarial-Spirit red-team 80-scenario corpus authored and run (NFR-Sec-10 v1.5),
So that v1.0 has externally-validated authorship + behavioral-equivalence + adversarial-resistance evidence.

**Acceptance Criteria:**

**Given** the N=12 stratified recruitment (≥4 with no prior MAOS contribution; ≥3 never written Rust Spirit; ≥2 never written Rust at all; ≥2 non-English-native; ≥1 working offline-only)
**When** the trial runs with a 14-day no-DM-support window
**Then** floor ≥10/12 produce a working signed Spirit binary
**And** the binary loads on a fresh Host VM
**And** runs ≥1000 frames
**And** achieves halt-recall ≥0.85
**And** Wilson CI [0.552, 0.962] meaningful at N=12 (meaningless at N=5 per NFR-Test-8)
**And** auditable via SBOM + signing chain re-loaded on clean VM by CI bot
**And** trials run only at major releases (v1.0, v2.0); minor releases use NFR-Onb-1 as proxy

**Given** the E5 §13.1 measurement outcome
**When** the gate evaluates rust-inproc form status
**Then** if rust-inproc form is active: cross-form semantic equivalence (NFR-Test-7) is verified — rust-inproc ↔ subprocess ≥90% / any-rust ↔ wasm ≥75%
**And** CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs)
**And** if rust-inproc form is DEFERRED to v2.0+ per Story 5.5 outcome: NFR-Test-7 cross-form is removed from v1.5 scope and CLI-wrapper test runs only

**Given** the adversarial-Spirit red-team corpus (NFR-Sec-10)
**When** the corpus is authored
**Then** 8 attack classes × N=10 = 80 canonical scenarios, expanded via parameter variation to 640 effective items (Murat's generator discipline)
**And** attack classes: capability confusion / IAC frame injection / distillation poisoning / ledger tampering / cross-Spirit privilege escalation / resource exhaustion / side-channel timing / kernel-syscall abuse
**And** the corpus is authored by an external pen-tester (not MAOS team) using published ABI
**And** the corpus is pre-frozen and content-addressed

**Given** the adversarial-Spirit red-team gate
**When** the corpus runs
**Then** floor ≥9/10 per class detected/blocked by kernel
**And** floor ≥72/80 aggregate
**And** 0 unmitigated categories
**And** failure is a v1.5 ship-block (NFR-Sec-10 phase v1.5)

### Story 10.3: Close v1.0 Compliance Gates — Export-Control, Fuzz Hardening, Korean Docs, CNA Registration

As a v1.0 compliance/security lead,
I want export-control classification artifact (NFR-Comp-1) + manifest parser fuzz 24h cargo-fuzz zero crashes (NFR-Sec-5) + wire-protocol adversarial-input fuzz 24h zero crashes (NFR-Sec-6) + Korean localization shipped (NFR-Doc-6) + CNA registration through MITRE (per NFR-Ops-4),
So that the substrate is regulatory-ready, fuzz-hardened, localized, and vulnerability-pipeline ready for v1.0 enterprise distribution.

**Acceptance Criteria:**

**Given** the export-control classification artifact (NFR-Comp-1)
**When** v1.0 ships
**Then** ECCN classification letter is on file
**And** EAR99 vs 5D002 determination is published in STABILITY.md §Export
**And** dual-use review for crypto primitives in kernel is complete
**And** absence is a v1.0 ship-block

**Given** manifest parser fuzz (NFR-Sec-5)
**When** 24h `cargo-fuzz` runs
**Then** zero crashes / OOMs / infinite loops
**And** results are published as a v1.0 ship-gate artifact

**Given** wire-protocol adversarial-input fuzz (NFR-Sec-6)
**When** 24h fuzz runs
**Then** zero crashes
**And** tiered cadence per §5.2: T1 per-commit (10 min, N=4 workers) / T2 nightly (4h, N=8) / T3 pre-release (24h, N=8)
**And** per-target floor ≥72 CPU-hours per fuzz target across 90 days pre-GA
**And** aggregate floor ≥1,000 CPU-hours pre-GA

**Given** Korean localization (NFR-Doc-6 v1.0)
**When** v1.0 doc site is built
**Then** Korean translations are present for all 5 canonical doc deliverables (E9 Story 9.5)
**And** LOCALES.md glossary lock applies
**And** deep-link preservation works across language switcher

**Given** CNA registration through MITRE (per NFR-Ops-4)
**When** v1.0 ship gate runs
**Then** CNA registration is complete (moved from v0.5 to v1.0 per NFR-Ops-4)
**And** advisory-publication channel is operational
**And** disclosure pipeline is exercised with at least one synthetic advisory before v1.0 ship

### Story 10.4: Ship the v1.5 Collective Tier with Postgres+pgvector and SQLite→Postgres Migration

As a v1.5 operator deploying the diagnostic-architect bilateral 2-Host pair,
I want the Postgres+pgvector Loom-lite collective tier (kernel-mediated via MCP-Streamable-HTTP per ADR-006 / NFR-Test-9), SQLite→Postgres migration test corpus on 10⁶ rows with byte-identical Merkle-root preservation (NFR-Ops-10), AND the Mira+Nash bilateral pair coordination (overlap with E8 Story 8.5),
So that v1.5 ships the collective-memory tier and the 2-Host deployment topology as a working, audit-traced operation — not a slide.

**Acceptance Criteria:**

**Given** the Postgres+pgvector Loom-lite collective tier
**When** the substrate boots with collective tier configured
**Then** the kernel mediates collective-tier access via MCP-Streamable-HTTP (no kernel module — Loom-lite is user-space per ADR-006)
**And** the Loom-not-in-kernel grep (Story 0.2 / NFR-Test-9) continues to return ∅
**And** RPO ≤1h / RTO ≤4h backups are verified weekly via Merkle-root cross-check

**Given** the SQLite→Postgres migration test corpus (NFR-Ops-10)
**When** the migration runs against a 10⁶-row corpus
**Then** forward-migration is byte-identical Merkle-root preserving
**And** rollback path is tested
**And** v1.4 gates v1.5 — migration test must pass before v1.5 release

**Given** Mira + Nash diagnostic-architect bilateral pair (cross-ref E8 Story 8.5)
**When** Host A (prod-edge with Mira) and Host B (dev-environment with Nash) are deployed
**Then** A2A cross-Host (E6 Story 6.3) operates between them
**And** pre-paired mTLS cert fingerprints are configured (no discovery)
**And** mobile push to operator works on halt
**And** J4 latency budget met: Mira-Nash Observer colocation <10ms P95 (§13.1)

**Given** 14-institution Cortex capacity envelope (NFR-Scale-5)
**When** the v1.5 capacity test runs at scale
**Then** the capacity envelope is documented for the v1.5 release
**And** 25-host churn test passes (NFR-Scale-2 v2.0 compressed scope; full 100-host at v2.5)

### Story 10.5: Mature v1.5 — Skill-Format Conformance, JetBrains, Windows, 2-Year LTS, Japanese/CN-S i18n

As a v1.5 operator on the long-term-support promise,
I want skill-format conformance demonstrating ≥1 third-party skill format (Anthropic Skills) executes via Spirit-form adapter without kernel modification (NFR-Test-10), JetBrains plugin-bridge for ACP (v1.5), Windows binary, 2-year LTS commitment, AND Japanese + Chinese-simplified localization,
So that v1.5 is the long-term-support release with proven extensibility across editor / OS / language boundaries.

**Acceptance Criteria:**

**Given** skill-format conformance (NFR-Test-10)
**When** the test runs
**Then** ≥1 third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification
**And** the kernel ABI is unchanged by the adapter — verified via ABI-diff lint
**And** the conformance result is journaled as a v1.5 release artifact

**Given** JetBrains plugin-bridge for ACP (v1.5)
**When** the plugin is installed in JetBrains IDE
**Then** Spirits hosted via ACP are routable through JetBrains (NDJSON over stdio extended)
**And** the bridge does not require kernel modification

**Given** Windows binary (v1.5)
**When** the operator runs `maosctl install` on Windows
**Then** the install succeeds with the same signature verification as Linux/macOS (FR1)
**And** sandbox tier T2 uses Windows restricted-token (Story 1b.3 cross-ref)
**And** per-Spirit resource caps use Job Objects (Story 1b.3 cross-ref)

**Given** 2-year LTS commitment (NFR-Maint-6 v1.5)
**When** v1.5 ships
**Then** the LTS clock extends from 1-year (v1.0) to 2-year once support load is known
**And** STABILITY.md is updated with the new LTS span

**Given** Japanese + Chinese-simplified localization (NFR-Doc-6 v1.5)
**When** v1.5 doc site is built
**Then** Japanese and Chinese-simplified translations are present for all 5 canonical doc deliverables
**And** LOCALES.md glossary lock continues to exclude Spirit / Worker / kernel / ADR identifiers / error codes
**And** RTL layout support remains deferred to v2.5

**Given** mTLS cert rotation chaos test full execution (NFR-Sec-13)
**When** the 3-host v1.5 rotation chaos runs
**Then** zero conversation drops are observed
**And** revocation latency median ≤60s / p99 ≤5min
**And** 10-host rotation defers to v2.0

**Given** ComplianceClaim envelope schema migration validation through v1.5
**When** schema evolution is tested
**Then** any ABI-breaking change to required fields, removed fields, renames, type-changes, or enum reorderings triggers an `ABI_VERSION` bump per §8.5
**And** additive optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]` and additive enum variants with explicit `#[repr(u8)]` discriminants + `#[serde(other)]` fallback do NOT bump

---

## Dependency Verification (12-Epic Ordering)

Strict ordering ensures no epic requires a later epic to function. Where forward dependencies exist (e.g., NFR-Onb-1 at v0.3 needs E7 ecosystem tooling), they are resolved by **slicing minimum prerequisites forward**, not by reordering.

- **E0** stands alone — cross-cutting; founding sprint with v0.1 acceptance, then maintenance.
- **E1a** depends on E0 (CI gates must be green before workspace lands).
- **E1b** depends on E0 (ComplianceClaim schema adversarially reviewed before freeze) + E1a (workspace + ABI types).
- **E2** depends on E1a (Spirit ABI types) + E1b (manifest schema frozen, hello-Spirit roundtrip). Provides NFR-Onb-1 prerequisites that E7 + E8 Butler consume at v0.3.
- **E3** depends on E1b (IAC bus skeleton + Approval Decision Log + Transparency Log) + E2 (Spirit ABI lifecycle hooks). Halt UX surface; halt mechanism dependent on E4.
- **E4** depends on E1a (halt schema types in `maos-domain`) + E2 (Spirit ABI hooks) + E3 (halt-resolution surface). **SINGLE HALT OWNER.**
- **E5** depends on E4 (halt mechanism for FR53 halt-continuity-across-hot-swap) + E1b (sandbox tier infrastructure) + E2 (Spirit ABI lifecycle hooks). §13.1 measurement gate lives here.
- **E6** depends on E1b (IAC bus skeleton) + E2 (Spirit ABI) + E3 (`task.assign` frame definition) + E4 (intent_lineage/cognition substrate types) + E5 (lifecycle triggers + crash supervision; A2A peers require Spirits to crash-survive).
- **E7** depends on E1b (ComplianceClaim schema frozen) + E2 (SDK seed) + E5 (Spirit registry over MCP) + E6 (IAC bus for spirit-test integration scenarios). Executes NFR-Onb-1 v0.3 gate using E2 prerequisites + E8 Butler.
- **E8** depends on E4 (memory + halt for Butler v0.3) + E5 (lifecycle for Butler-class), E6 (IAC + A2A for Orchestrator+Workers v0.8 and Mira+Nash v1.5), E7 (Spirit ecosystem tooling for publishing reference Spirits).
- **E9** depends on logs/frames/state from E1b–E6 to query/export/forget; on E4 principal namespace for GDPR cascade; on E7 ComplianceClaim envelope for verification queries.
- **E10** depends on **all prior epics** — coordination gates fire against work owned elsewhere (HSIS corpus authored E5, CCAC corpus authored E7, etc.). v1.5 sub-cluster work depends on E9 (multi-operator tenancy primitive-reservation) + E6 (cert rotation infrastructure) + E4 (memory tiers for Loom-lite collective).

**Forward-dependency resolution (NFR-Onb-1 v0.3 phasing tension):**
- E2 ships thin cargo-generate template + local runner + ≥1 example Spirit with passing CI **at v0.3-era completion** (before Butler reference Spirit ships in E8).
- Full spirit-test SDK with assertion macros lands in E7 (v0.5+ feature work).
- E7 RUNS the 30-Min First Spirit Gate at v0.3 using Butler from E8 + thin tooling from E2.

**Halt protocol dependency chain (resolved):**
- E1a defines halt schema types in `maos-domain` (data only).
- E4 owns halt mechanism + I14 invariant + halt-receipt 99.9% + recall/precision floors.
- E3 owns halt resolution UX surface (notification, 3-tap mobile flow) — calls into E4 primitives.
- E5 owns halt-continuity-across-hot-swap (FR53 I14 runtime check) — Hot-Swap Coordinator validates `halt_set` before swap using E4's schema.

**rust-inproc gating (resolved):**
- ADR-002 commits to subprocess form at v0.1; rust-inproc gated on §13.1 measurement.
- E5 carries the §13.1 measurement story with go/no-go gate before v0.5 ships.
- If subprocess form meets latency budgets (J1 <25ms P95 IPC; J4 <10ms P95), rust-inproc form may be deferred to v2.0+ (eliminating NFR-Test-7 cross-form equivalence from v1.5 scope).
- If gates fail, rust-inproc development unlocks within E5 with cross-form equivalence test in E10.

---

## Open Items for Story Creation (Step 3)

Three items flagged by the agents that may surface in story-level design but don't block the epic structure:

1. **Mary's question:** Should E4 (Halt Protocol) split E4a (local halt v0.3) vs E4b (cross-Spirit cascade v0.8)? Decision: address at story level — phase boundaries within E4's story sequence.

2. **John's open demand:** ~1390 corpus gold items in E8 — Murat's resolution: parameterized generators with seeded templates make this tractable (~2,249 items if you count generator expansions; CCAC, red-team, secret-redaction all generator-driven).

3. **Winston's KLOC alarm prediction:** ~18–27 KLOC bleed past 20. Monitored continuously by E0's `tokei` gate, with budget cut decisions made at merge time, not ship time.

## Open Items Carried Forward to Implementation

The following items were knowingly carried into implementation rather than closed during step-03/04. None block development — each has a documented fallback path or downstream-closure plan. Dev agents picking up stories should be aware of these.

### 1. Partial crate-path retrofit on ~49 stories

**Status:** ~15 stories have full crate-path treatment in every AC (the exemplar set); ~49 stories retain some "the kernel" / generic-component references in select ACs.

**Why deferred:** Retrofitting 300+ ACs across 49 stories was diminishing-return work after the exemplar set established the pattern. Story 1a.1, 1b.5a/b/c, 3.3, 4.1, 5.2, 5.5a–e, 7.5a/b, 0.5, and 9.2 demonstrate the target conventions.

**Fallback path for dev agents:** When an AC says "the kernel" without a crate path, consult `architecture-maos-minimal-opus.md` §4.0.2 for the canonical crate-to-responsibility mapping. The 17-crate workspace is bounded — "the kernel" almost always maps to `crates/maos-kernel-core/<service>/` where `<service>` ∈ {scheduler, security, memory, iac, capability}. The full retrofit-status note lives inline at the top of the Epic List.

**Closure path:** During story execution via `/bmad-create-story`, the dev agent or PM can convert "the kernel" references to specific crate paths on a per-story basis as stories land in sprint planning. This is preferable to a speculative retrofit since the actual crate boundaries may evolve slightly during E1a's bootstrap.

### 2. v0.3 Halt corpus is provisional `synthetic-v0` N=50

**Status:** Story 4.1's halt-recall/precision floor measurement (NFR-Test-4: ≥0.7 / ≥0.85) cites a provisional corpus at `crates/maos-eval/fixtures/halt-corpus-v0/` containing 50 hand-authored synthetic scenarios.

**Why deferred:** Round-3 stress-test (Amelia + Murat) flagged that the original AC pointed to "bmad-eval standard corpus against E8 reference Spirits" — a corpus that does not exist when Story 4.1 is implemented (E8 reference Spirits don't ship until v0.3+ per their respective phase anchors). Writing tests against a future corpus is a forward-dependency leak.

**Fallback:** The synthetic-v0 corpus is sufficient to gate Story 4.1's v0.3 release. It validates the halt mechanism's measurement plumbing end-to-end. Floor numbers are real (≥35/50 recall, ≥43/50 precision); they're just measured against synthetic prompts rather than reference-Spirit production traces.

**Closure path:** At v1.0, the E8 reference-Spirit corpora (Butler 30-scenario calendar/comms from Story 8.1; Researcher distillation eval from Story 8.2; Orchestrator+Workers founder-loop scenarios from Story 8.4; Mira+Nash safety-critical N≥150 from Story 8.5) replace `synthetic-v0` as the bmad-eval gate. Story 4.1's AC4 explicitly tags the corpus `synthetic-v0` to distinguish it from the v1.0 reference corpora.

### 3. Intra-E4 ordering: Story 4.5 (HSIS 100) must close before Story 4.1's halt-receipt gate at v1.0

**Status:** Story 4.1 (halt mechanism + halt-receipt ≥99.9%) and Story 4.5 (cross-Spirit memory isolation 200-corpus + Hot-Swap I14 enforcement + HSIS Researcher+Observer 100 scenarios) are both in E4. Story 4.1's halt-receipt gate is measured against the HSIS termination corpus that Story 4.5 authors.

**Why deferred:** Round-3 (Murat's risk-rating) identified that if Story 4.5's 100 HSIS scenarios are sprinted *after* Story 4.1's gate-closure attempt, the gate has no production-grade corpus and falls back to synthetic-v0 (per item 2 above). The fix is sprint-ordering, not story-rewriting.

**Closure path:** Sprint plan must enforce Story 4.5 corpus authoring closes before Story 4.1's v1.0 halt-receipt gate runs. This is documented in:
- Story 4.1 AC4: "**intra-E4 ordering: Story 4.5 (HSIS corpus 100 scenarios) MUST close before Story 4.1 AC closes at v1.0**"
- Story 5.2: HSIS additional 200 scenarios (Butler/Orchestrator/Worker/CliWrapper classes) must also land before Story 4.1's v1.0 gate

**Composite gate at v1.0:** Story 4.1 halt-receipt ≥99.9% across the **cumulative 300-scenario HSIS corpus** = 100 from Story 4.5 (Researcher+Observer) + 200 from Story 5.2 (Butler/Orchestrator/Worker/CliWrapper). The Dependency DAG section above captures this in the v1.0 sprint invariants.

---

### Summary

These three items are known shapes of the v0.1 → v1.0 sprint plan, not defects in the epic/story breakdown. Dev agents implementing E4 should treat Story 4.1's `synthetic-v0` corpus as a v0.3-shippable measurement floor, not a permanent target. Sprint planners should sequence Story 4.5 + Story 5.2 corpus authoring **before** any v1.0 gate-closure attempt on Story 4.1. PMs running `/bmad-create-story` to extract individual story specs should consult `architecture-maos-minimal-opus.md` §4.0.2 to concretize "the kernel" references into specific crate paths at story-extraction time.

