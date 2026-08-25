# Interview Cheat Sheet — Accenture AI LLM Architect (R00340604)

**Candidate substrate:** MAOS — Modular Agentic Operating System (`/home/lunarpulse/dev_ws/maos`)
**Date:** 2026-08-19 · **Status:** v0.1-alpha, 44-crate Rust workspace · **Prepared by:** the party (spine-first narrative)

---

## The one law (read first)

> **Every claim must survive the second question.** The interviewer's follow-up to any architecture claim is *"show me where that's enforced / what happens when it fails."* Every point below carries its refusal path. If you can't name the mechanism that says no, don't make the claim.

## 30-second opening pitch

> "I built the control plane this JD describes. MAOS is an agent operating system: a small invariant kernel that hosts LLM-backed agents as processes — capability-isolated, auditable by construction, model- and tool-agnostic. Models are pluggable drivers (Anthropic, OpenAI, Ollama). Tools flow through an MCP gateway. Admission is a certification gate — signed manifests, trust tiers, attestation, measured eval metrics before anything reaches production. Fourteen constitutional invariants are mechanically enforced on every PR. The platform-side work — fine-tuning pipelines, RAG serving, inference throughput — is exactly what I'd architect on top of this spine."

## The numbers (memorize cold)

| Fact | Value |
|---|---|
| Workspace | 44 crates, Rust 1.88 stable |
| Kernel core | ≤20 KLOC, tokei-enforced (alarm at 16) — ADR-038 |
| Decision records | 59 ADRs (`docs/adr/`) |
| Planning | 12 epics; PRD → architecture → epics → stories pipeline |
| Reference agents | 9 Spirits, each **zero kernel KLOC added** |
| Invariants | I1–I14, mechanically gated per PR (ADR-037) |
| Gates on every PR | invariant-lock, KLOC ceilings, `cargo public-api` ABI freeze, service-boundary lint, empty-kernel (I9) lint |
| Honesty marker | status is **v0.1-alpha** — the discipline is production-grade; say the status yourself before they find it |

---

## Point-by-point (13 JD bullets)

### 1. Model- and tool-agnostic multi-agent systems governed by an MCP Control Plane

- **Say:** "I built that substrate. Model-agnostic: one provider adapter trait, Anthropic / OpenAI / Ollama behind it. Tool-agnostic: tools consumed through an MCP gateway, every call mediated by the kernel's Capability Registry. Multi-agent: in-process IAC bus plus cross-host A2A over mTLS with typed-intent consent. And the registry itself speaks MCP — it *is* an MCP-Streamable-HTTP server, so third parties can serve agents to the fleet over MCP."
- **Evidence:** `crates/maos-providers` (ADR-005) · `crates/maos-mcp` · `crates/maos-registry` ("Spirit Registry — MCP-Streamable-HTTP server + kernel client") · `crates/maos-a2a-core`, `maos-a2a`, `maos-a2a-tcp` (ADR-012).
- **Second question — "So MCP is your control plane?":** "MCP is a protocol surface on two sides — tool ingress and registry serving. The control plane is the kernel: every external call is mediated by the Capability Registry and evaluated by an embedded Cedar PDP with a deny tripwire. MCP carries requests; the kernel authorizes them."
- ⚠️ **Never say** "MCP governs my system." First architect who asks "which MCP server authorizes a tool call?" owns you.

### 2. Agent Registry as mandatory system of record + AI Gateway for runtime policy enforcement

- **Say:** "The Spirit Registry is the system of record — nothing runs unless admitted through it: manifest-validated, signed, trust-tiered. Runtime policy enforcement is at the gateway seam: an in-process Cedar PDP (ADR-050) evaluates permit/forbid, decisions materialize into a copy-on-write policy table, and a per-commit gate fault-injects the deny path so enforcement can't rot into an advisory."
- **Evidence:** `crates/maos-registry` · `crates/maos-manifest` · `crates/maos-capability` · `crates/maos-pdp` (ADR-050) · `schemas/gateway-submodule.schema.json` · ADR-057 (governance-as-daemon posture).
- **Second question — "What if the PDP is down?":** "Fail-closed with a staleness TTL. No config → default deny. Down at startup → closed, loudly. Runtime drop → freeze last-known-good. Past the TTL → PDP-granted capabilities revert to deny, so a dead PDP can't keep serving a revoked grant forever. Four distinct cases, four proven-red tests."
- **Status:** strength.

### 3. Certification gate — no uncertified agents in production (identity, policies, evaluation metrics)

- **Say:** "Admission *is* the certification gate, and it validates all three things the JD names. Identity: signed manifests, trust tiers, attestation promotion — public-untrusted material only promotes after attestation, to the strictest tier involved (ADR-053, ADR-056). Policies: manifest scope ceilings checked against org Cedar policies. Evaluation metrics: halt-recall and precision gates from the eval harness are admission criteria. Rejections are journaled, never silently dropped."
- **Evidence:** ADR-053 (third-party trial attestation) · ADR-056 (FR37 vetting machinery) · `crates/maos-skill` admission queue (FR57: pending → approve/reject, journaled) · `crates/maos-compliance` (ComplianceClaim semantic evaluator + execution-context drift) · `crates/maos-eval` (measurement gates).
- **Second question — "How do you know the gate actually blocks?":** "The blocks are proven-red, not asserted. The deny arm is fault-injected in CI with two distinct engine evaluations — a memoizing cache passes a naive swap, which is exactly the fake we test for. Unattested submissions are rejected on enqueue with a journaled refusal. Every count has a per-item blind that reds it."
- **Status:** **the differentiator — lead with this.**

### 4. Abstracted core agent services; first-class memory service (semantic, episodic, procedural)

- **Say:** "Memory is a first-class kernel service with kernel-enforced scopes — per-Spirit private, shared per-host, collective across hosts on Postgres + pgvector. What's rarer than the taxonomy is provenance: every persisted digest references its raw source frames (I11), every byte in an agent's context traces to a logged recall or inbound entry (I12), digests carry intent provenance (I13), and cross-region collective memory converges under signed replication bundles (ADR-049). The semantic/episodic/procedural endpoints are the interface layer I'd add — the substrate that can *enforce* them already exists."
- **Evidence:** kernel Memory Manager · I5 (memory scopes kernel-enforced) · ADR-026 (principal namespaces) · ADR-022 (working-memory slots) · ADR-013 (kernel-mediated `log.recall`) · ADR-049 · `crates/maos-loom-lite` (pgvector).
- **Second question — "So you don't have the triad?":** "Not as typed endpoints. The honest mapping: episodic ≈ transparency-log recall, procedural ≈ distillates and skills, semantic ≈ Loom retrieval over pgvector. I'd formalize those as endpoints on the memory manager — the hard part, enforcement and provenance, is done."
- **Status:** partial — present as a design opinion, not a gap confessed under pressure.

### 5. End-to-end data pipeline (ingestion, preprocessing, sync for fine-tuning and RAG)

- **Say:** "On the evaluation side, yes: parameterized corpus generators, red-team and secret-redaction corpora, trajectory and trace-shape JSON schemas, a calibration harness. On fine-tuning dataset assembly and sync — not in this project. It's the first thing I'd build on the substrate, because the upstream half already exists: trajectories are captured in queryable, schema-validated form."
- **Evidence:** `crates/maos-corpus-gen` · `tests/corpora/` · `schemas/trajectory.schema.json` · `schemas/trace-shape.schema.json` · `crates/maos-eval` (fixtures, calibration).
- **Second question — "Where exactly is the gap?":** "Fine-tuning sync. Structured capture is built; training-set assembly is greenfield."
- **Status:** gap — name it yourself, one sentence, redirect.

### 6. Context layer — knowledge graphs, vector search, semantic retrieval, grounded RAG

- **Say:** "Vector search exists in the collective memory tier — pgvector under Loom-lite. Grounding is *enforced* rather than hoped for: context traceability is a kernel invariant (I12), distillation preserves source references (I11), and an agent with insufficient evidence hits an epistemic halt — a first-class, user-mediated, audit-trailed outcome — instead of hallucinating into production. A full RAG pipeline — embedding lifecycle, chunking, retrieval evaluation — is what I'd architect on top; the enforcement spine it needs is already there."
- **Evidence:** pgvector (Loom-lite, v1.5) · I11/I12/I13 · `maosctl halt list / halt resolve` · Researcher Spirit (participant-scoped recall walker + distillate chain).
- **Second question — "How do you know retrieval is grounded?":** "Every byte in context must trace to a logged `log.recall` or inbound entry — kernel-enforced — and the kernel refuses to deliver any frame the transparency log refused to record."
- **Status:** partial — enforcement strong, retrieval product thin.

### 7. Dynamic cost-and-performance-aware model routing and selection

- **Say:** "Multi-provider selection exists — the adapter trait makes Anthropic, OpenAI, and local Ollama interchangeable, with model provenance required at runtime. Cost is measured, not guessed: per-call cost attribution and reconciliation (ADR-046) with schema'd attribution payloads. A policy-driven router that re-ranks providers on live cost and latency budgets is the natural next step — the data that would drive it is already captured and reconciled."
- **Evidence:** `crates/maos-providers` · ADR-046 · `schemas/cost-attribution-payload.schema.json` · model-provenance runtime checks.
- **Second question — "Is routing dynamic today?":** "Selection is configuration + adapters; attribution is measured. I won't claim a policy router I haven't built — I'll tell you exactly where it plugs in."
- ⚠️ **Never say** "cost-aware routing." Attribution ≠ routing. Conflating them is a credibility event.
- **Status:** partial.

### 8. High-throughput, low-latency inference (response caching, request batching)

- **Say:** "That's not my project's layer — MAOS is a *client* of inference, not a serving platform. What I did measure and gate is the substrate's own hot path: a measurement gate benches founder-loop IPC and colocation latency, replay determinism is trace-shaped (ADR-028), and the runtime is an actor model on Tokio — bounded mailboxes, no shared mutable state on the hot path (ADR-011). If the role owns inference throughput, that discipline transfers directly: measure, gate, enforce."
- **Evidence:** `crates/maos-bench` · ADR-028 (replay determinism) · ADR-011 (actor model) · `crates/maos-journey-test` (cassette replay).
- **Status:** gap — honest, and framed as transferable discipline.

### 9. Security, governance, observability as centrally-enforced, by-design controls

- **Say:** "This is the project's core. Fourteen constitutional invariants — I1 to I14 — are mechanically enforced: `invariant-lock` requires a machine-checkable diff, a corpus delta, a phase-commitment update, and two maintainer sign-offs to change any invariant; weakening one trips the gate and forces a major-version bump (ADR-037). Every inter-agent frame is logged *before* delivery — the kernel will not deliver a frame the transparency log refused to record. Telemetry is broadcast with per-agent subscription. And the gates shipped *before* the kernel code did — by design."
- **Evidence:** I1–I14 (`docs/invariants/`, README) · xtask per-PR gates · `crates/maos-telemetry` · `schemas/audit-bundle.schema.json` · `schemas/governance-event-payload.schema.json` · ADR-045 (governance audit artifacts) · ADR-037.
- **Second question — "By design — concretely?":** "The constitution is structural, not procedural: per-PR mechanical gates, KLOC ceilings by tokei, ABI freeze by `cargo public-api`, an empty-kernel lint proving the kernel itself learns nothing (I9)."
- **Status:** **strength — the thesis. Own this room.**

### 10. Defense-in-depth security: per-agent identity, IAM/IAP binding, layered guardrails

- **Say:** "Per-agent identity *finer* than IAM/IAP: capability tokens are unforgeable, short-lived — TTL ≤60 seconds for high-privilege ops — and bound to the OS process ID (ADR-023). Identity down to the process; no replay across processes. Layers: OS-native sandbox tiers T0–T3 (ADR-004), a structural sandbox-escape detector (ADR-024), typed-intent consent for every cross-host interaction (ADR-012) over mTLS+TOFU with pinned cohort authorities (ADR-054), Cedar policy enforcement with staleness TTL (ADR-050), enterprise identity at rest + SIEM shipping (ADR-051), OS-keyring secret pass-through — the kernel stores no secrets (I9) — and GDPR Article-17 distillate redaction (ADR-044)."
- **Evidence:** all ADRs above · `crates/maos-sso` · `crates/maos-siem` · `crates/maos-secrets` · `crates/maos-escape-detector` · `docs/loom-threat-model.md` · `docs/red-team/` · `docs/pen-test/`.
- **Second question — "What residual keeps you up at night?":** Name one honestly — e.g., "cross-team read-path attestation: entry-side forgery under the derived key is closed; the read-side guard ships with the cross-team write path. It's in the threat model with a named successor." Knowing your open residuals *is* the architect signal.
- **Status:** strength — their bullet is a subset of the threat model.

### 11. Enterprise AI reference architecture, reusable patterns, approved component library

- **Say:** "The reference architecture is the repo: a documented kernel design — services, ABI, adapter ring — 59 ADRs with rationale for every contested call, an invariants register, a threat model, and a *curated* component surface: dependency allowlisting, a composition-root whitelist, ABI baselines, cargo-generate agent templates and SDKs for third-party authors, and a signed registry with trust tiers. The reusable patterns are codified as gates — anyone re-runs the whole discipline suite with `cargo xtask`."
- **Evidence:** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/` · `docs/adr/index.md` (59) · `docs/invariants/` · `deny.toml` · `xtask/composition-root-whitelist.toml` · `abi-baseline/` · `templates/` · `sdks/` · docs-site (Docusaurus).
- **Status:** strength.

### 12. PoC prototypes and foundational components validating architectural decisions

- **Say:** "Every load-bearing decision has a spike or a proof. A WASM-host spike and a measurement gate decided the agent form question — subprocess-only at v0.1, in-process unlocked only by measurement (ADR-002, ADR-040), WASM component model targeted for v2.0 (ADR-031). And nine reference agents each prove one substrate claim — cross-host diagnostic loops, founder-loop orchestration, anticipatory cognition — each adding **zero kernel KLOC**, which is itself the proof that behavior lives outside the kernel."
- **Evidence:** `spikes/story-11-0-wasm-host/` · `wit/spirit.wit` · ADR-002 / ADR-040 / ADR-031 · `spirits/` (Butler, Researcher, Observer, Orchestrator, Worker, Architect, Reviewer, Mira, Nash) · `examples/` (Rust + TypeScript).
- **Status:** strength.

### 13. Authoritative architecture artifacts — blueprints, sequence diagrams, design specs, ADRs

- **Say:** "All public in the repo: the architecture blueprint set, 59 ADRs, the invariants register, a 16 KB threat model, ABI stability and breaking-change logs, a coverage matrix tracing tests to requirements, runbooks, release holds, and a docs site. Point me at any flow — certification, hot-swap, cross-host consent — and I'll draw the sequence diagram live on the whiteboard."
- **Evidence:** `docs/adr/` · `docs/invariants/` · `docs/loom-threat-model.md` · `STABILITY.md` · `BREAKING.md` · `RELEASE-HOLDS.md` · `tests/coverage-matrix.yaml` · `docs/runbooks/` · `docs-site/`.
- **Status:** strength — offering to draw live beats claiming diagrams exist.

---

## Never-say list (credibility protection)

1. **"MCP governs my system"** — kernel + Capability Registry + Cedar PDP govern; MCP is a surface spoken on two sides.
2. **"Cost-aware dynamic routing"** — ADR-046 is *attribution*; selection is configuration. Don't conflate.
3. **"Semantic/episodic/procedural memory service"** — shipped memory is scopes + provenance + pgvector; the triad is your roadmap, not your inventory.
4. **"High-throughput inference layer"** — you're a client of inference; you bench IPC, not token throughput.
5. **"Production-ready"** — status is v0.1-alpha. Say it first: "the discipline is production-grade, the product is alpha." Volunteering that is a trust move; being caught is not.
6. **Any claim without a refusal path.** The law.

## The gap paragraph (memorize verbatim)

> "The platform-side work — fine-tuning pipelines, RAG serving, inference throughput — isn't in this project, and that's deliberate: those are the parts an enterprise can buy. What almost nobody has is the spine — attested admission, capability-enforced mediation, kernel-enforced transparency, invariant gates on every PR. I built the hard half. The other half is what I'd architect *on top of it*, and the JD lists the spine first — which tells me they know which half is hard."

## If they push on maturity (pre-empted)

> "v0.1-alpha, single-maintainer, 12 epics planned through v2.0 — but the *governance* isn't aspirational: invariant-lock, ABI freeze, KLOC ceilings, and the eval gates run on every PR today. Most enterprises can't say that about systems already in production."

---

## 2-minute spoken versions

*Pace: conversational, ~150 wpm. The em-dashes mark natural breath points. Numbers are written the way you say them.*

### English (~300 words · ≈2:00)

I've spent the last stretch of my career building the control plane this role describes. It's an open-source project called MAOS — a modular agentic operating system. The premise: treat LLM-backed agents the way an operating system treats processes. A small, invariant kernel — capped at twenty thousand lines, and that cap is enforced by tooling — hosts agents as sandboxed, hot-swappable processes. The kernel does infrastructure — agents do behavior. Nothing else.

Three design decisions map directly onto this job. First, model- and tool-agnosticism. Models are pluggable drivers — Anthropic, OpenAI, local Ollama behind one interface. Tools flow through an MCP gateway — and the registry itself serves agents over MCP, so third parties can plug into the fleet.

Second, governance is enforced, not documented. Every external call is mediated by a capability registry. Policy decisions run through an embedded Cedar policy engine — and if the policy engine goes down, the system fails closed. Admission is a certification gate: signed manifests, trust tiers, attestation, and measured evaluation metrics before anything reaches production. Rejections are journaled — never silent. Fourteen constitutional invariants are mechanically gated on every pull request — and those gates shipped before the kernel code did.

Third, per-agent identity finer than IAM: unforgeable capability tokens, sixty-second lifetimes, bound to the process ID. Sandbox tiers, escape detection, cross-host consent over mutual TLS. And memory is a first-class service with kernel-enforced scopes — its defining property is provenance: every byte in an agent's context traces back to a logged event.

The honest gaps: fine-tuning pipelines, RAG serving, inference throughput — deliberate, because those are the parts an enterprise can buy, and the spine is the part it can't. Forty-four crates, fifty-nine ADRs, nine reference agents that added zero kernel code. It's v-zero-point-one alpha and I'll say so myself — the discipline is production-grade. Give me this spine, and governance isn't a layer you bolt on later — it's the ground you build from.

### 기술 한국어 (≈2:00)

이 직무가 설명하는 컨트롤 플레인을 저는 직접 구축했습니다. MAOS — Modular Agentic Operating System이라는 오픈소스 프로젝트입니다. 핵심 전제는, LLM 에이전트를 운영체제가 프로세스를 다루는 방식으로 다루는 것입니다. 2만 줄로 제한된 — 그리고 그 제한을 툴링이 강제하는 — 작은 커널이, 에이전트를 샌드박스된 핫스왑 가능한 프로세스로 호스팅합니다. 커널은 인프라만, 에이전트는 행동만 담당합니다.

이 프로젝트의 설계 결정 세 가지가 이 직무에 그대로 대응됩니다. 첫째, 모델·도구 비종속입니다. 모델은 플러그형 드라이버라서 Anthropic, OpenAI, 로컬 Ollama가 하나의 인터페이스 뒤에서 교체되고 — 도구는 MCP 게이트웨이를 통과하며, 레지스트리 자체가 MCP 서버로 에이전트를 서빙하기 때문에 서드파티가 플릿에 플러그인됩니다.

둘째, 거버넌스는 문서가 아니라 강제입니다. 모든 외부 호출은 Capability Registry를 거치고, 정책 판정은 임베디드 Cedar 엔진이 내리며 — 엔진이 죽으면 시스템은 fail-closed로 닫힙니다. 어드미션은 인증 게이트입니다: 서명된 매니페스트, 신뢰 등급, attestation, 그리고 측정된 평가 지표까지 통과해야 프로덕션에 들어갑니다. 거부는 저널에 기록되지, 절대 조용히 사라지지 않습니다. 14개 헌법적 불변량이 매 PR마다 기계적으로 게이트되고 — 이 게이트는 커널 코드보다 먼저 만들어졌습니다.

셋째, 에이전트 단위 정체성은 IAM보다 세분화돼 있습니다. 위조 불가능한 capability 토큰, 60초 TTL, PID 바인딩. 샌드박스 티어, 탈출 탐지, mutual TLS 기반 크로스호스트 동의. 메모리도 퍼스트클래스 서비스인데 — 커널이 스코프를 강제하고, 결정적 특징은 출처 추적성입니다. 에이전트 컨텍스트의 모든 바이트가 로깅된 이벤트로 역추적됩니다.

솔직한 공백도 말씀드립니다: 파인튜닝 파이프라인, RAG 서빙, 추론 처리량. 의도적인 선택입니다 — 그쪽은 기업이 살 수 있는 부분이고, 스파인은 살 수 없는 부분이니까요. 44개 크레이트, ADR 59건, 참조 에이전트 9개는 커널 코드를 한 줄도 늘리지 않았습니다. v0.1-alpha라는 것도 제가 먼저 말씀드립니다 — 다만 그 규율은 프로덕션급입니다. 이 스파인 위에 플랫폼 절반을 아키텍처하는 것 — 거버넌스를 나중에 얹는 레이어가 아니라, 시작하는 바닥으로 만드는 것. 그게 이 직무에서 제가 할 일입니다.

### Delivery notes

- **Land the three-part structure audibly:** "First… Second… Third…" — interviewers score structure they can hear.
- The gaps sentence is deliberate in both languages: say it unhurried. Confidence on your own gaps is the tell senior interviewers listen for.
- Korean version keeps the English technical loanwords (Capability Registry, attestation, fail-closed) — that's the natural register of a Korean engineering interview; don't translate them.
