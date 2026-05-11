# Epic 8: Reference Spirits — Butler → Researcher/Observer → Orchestrator+Workers+Architect+Reviewer → Mira+Nash (v0.3 → v1.5)

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

### Stories

## Story 8.1: Butler v0.3 — `on_idle` Anticipatory Reasoning + Morning Digest Spirit-Side

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

## Story 8.2: Ship the Researcher Reference Spirit with Distillation Pattern and `log.recall` Walker

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

## Story 8.3: Observer v0.5 — Telemetry Stream Subscriber + Pre-Halt Scalar Drift Watchdog

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

## Story 8.4: Ship the Founder-Loop Wedge Spirits — Orchestrator, Workers, Architect, Reviewer

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

## Story 8.5: Ship the Mira+Nash Diagnostic-Architect Bilateral Pair with Safety-Critical Corpus

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
