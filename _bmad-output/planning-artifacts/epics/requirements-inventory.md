# Requirements Inventory

## Functional Requirements

**Total: 65 FRs across 7 capability areas. Numeric ship-gate floors are integral to FR text where the PRD includes them.**

### A. Kernel Substrate Operations (9 FRs)

- **FR1:** User can install MAOS kernel via OS package manager (Homebrew/AUR/deb/rpm), `cargo install`, or signed GitHub Releases binary with mandatory Ed25519 signature verification.
- **FR2:** User can uninstall MAOS kernel cleanly, removing all installed Spirits, capability tokens, sandbox mounts, ACP sockets, and operator caches without leaving orphaned state.
- **FR3:** Operator can configure provider drivers (Anthropic, OpenAI, Gemini, Kimi, local-LLM via Ollama, air-gapped Bedrock) per Spirit, including locking provider endpoints for air-gapped deployment.
- **FR4:** Operator can verify every Spirit's external call (file op, network, exec, provider call, sub-Spirit spawn) was mediated by kernel-issued capability tokens by reading the Transparency Log; verification floor is 100% mediation in any 1000-call sample.
- **FR5:** Operator can configure sandbox tier per Spirit (T0/T1/T2/T3/T4); kernel enforces strictest-of-(manifest, trust-tier, operator-policy) floor. Spirit cannot exfiltrate data outside its declared capability scope — sandbox enforcement combined with FR4 capability mediation makes this property mechanically auditable.
- **FR6:** Operator can configure per-Spirit resource caps (CPU, memory, file descriptors) via cgroups v2 on Linux or platform equivalent.
- **FR7:** Operator can disable anonymous telemetry; default is opt-in with published schema and redaction layer.
- **FR47:** Spirit obtains all model inference exclusively via the kernel-provided Inference Port; the kernel routes to the configured provider driver and records the call in the Transparency Log. Spirit binaries do not import vendor LLM SDKs directly. (Closes ADR-005 coverage gap.)
- **FR48:** Operator can configure pluggable cryptographic provider for kernel signature verification, sealed-export encryption, and capability-token signing — enabling FIPS-validated, hardware-backed, or post-quantum implementations without recompiling Spirits. (FIPS / NIAP / export-control readiness.)

### B. Spirit Lifecycle Management (8 FRs)

- **FR8:** Spirit author can declare a Spirit class via manifest (TOML) covering `class`, `capabilities`, `posture`, `output_shape`, `explanation_shape`, `epistemic_policy`, `budget`, `skills`, `hot_swap`, `halt_protocol_compatibility`, `intent_promotion_set`, `migrates_from`, `swap_invariants`, `schedule`, `min_substrate_version` (kernel rejects load if its own version is below the declared minimum). Manifest declarations are signed and journaled.
- **FR9:** User can load, start, pause, resume, and unload Spirits at runtime via authenticated control plane (CLI, ACP editor surface, or operator API).
- **FR10:** Operator can hot-swap a Spirit class to a new version preserving in-flight capability tokens and working-memory state per the kernel-enforced migration decision tree (ADR-020). Both Spirit forms (in-process Rust ABI and subprocess) ship with parity on lifecycle and IAC semantics (ADR-002).
- **FR11:** Spirit author can declare cross-major migration via `migrates_from` manifest field and a `migrate(predecessor_state)` entry point; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator is declared.
- **FR12:** Kernel detects Spirit-process crash within 2s and emits `task.orphaned` IAC frames to in-flight task originators within 5s with exit-cause journaled. Floor: ≥99/100 detected within 2s in a SIGKILL crash corpus; ≥99/100 NACKed within 5s. Hung-Spirit detection (alive but no progress IAC for >30s) emits `task.stalled` event; ≥48/50 reclassified within 60s on a hang corpus.
- **FR13:** User or operator can revoke a Spirit at runtime via signed Revocation List artifact; running Spirit instances receive `SpiritRevoked` event and execute their declared revocation policy (terminate-immediately / drain-then-terminate / quarantine).
- **FR49:** Operator can upgrade a Spirit (replace v0.3.1 with v0.3.2) with declared migration policy: hot-swap with state preservation (default), cold-swap with re-init, or migrator-mediated cross-major upgrade. Distinct from FR9 (lifecycle verbs); FR49 covers state-bearing version transitions.
- **FR50:** Spirit author can declare dead-Spirit task disposition policy in manifest (`on_crash.action`); kernel applies the policy to in-flight tasks held by the dead Spirit (NACK / reassign-to-replica / escalate-to-operator). Operational-failure handling distinct from epistemic halt (FR15).

### C. Human–Spirit Interaction — Director's Surface (8 FRs)

- **FR14:** User can assign a task to a Spirit via natural-language `task.assign` IAC frame (terminal shell, ACP editor surface, mobile push) with goal + scope + success criteria + posture preferences.
- **FR15:** User can resolve a Spirit-emitted `epistemic.halt` via three documented resolution pathways — supplying missing context, accepting the halt as final, or authorizing override under operator policy; kernel journals the resolution with full reasoning chain. Halt-recall floor ≥0.7 and halt-precision floor ≥0.85 per Spirit class on the `bmad-eval` standard corpus.
- **FR16:** User can shift Spirit posture at runtime ("be more cautious for the next hour"; "switch to autonomous-with-halt"); the shift is journaled and applied to subsequent capability-scope decisions. Posture-shift propagation latency: P99 ≤2s, P99.9 ≤5s in a 1000-shift corpus.
- **FR17:** User can read a per-Spirit morning digest containing: (a) tasks completed in the last 24h with outcome tags, (b) open halts requiring resolution, (c) flagged anomalies with confidence ≥0.6, (d) trust-bar reflecting yesterday's predicate-fire rate. Digest is generated by a digest-shipping Spirit (Butler at v0.3 / Researcher at v0.5 / Orchestrator at v0.8+ — NOT kernel) within 30s of the user's first session of the day, using kernel-provided log-composition primitives and the §9.5 distillation pattern. Hallucination floor: 0 hallucinated tasks tolerated in any 100-digest corpus, verified against the actual Transparency Log; ≥95/100 digests must include all open halts and cite source log refs for all claimed completions.
- **FR18:** User can audit any Spirit decision retrospectively; every `decision.*` frame carries `working_memory_digest_refs` (I12) so post-hoc audit can reconstruct what the agent reasoned over at decision time.
- **FR19:** User can configure halt-recall vs halt-precision preference per Spirit per tag via a halt-policy schema (extension to ADR-013); kernel parses the preference into the Spirit's runtime epistemic policy thresholds.
- **FR20:** User can buffer multiple instructions to an Orchestrator Spirit (NOT kernel-buffered — Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives); the Orchestrator processes queued instructions at safe sequence points between task completions, never preempting in-flight delegations. (Phase: v0.8, advanced from v0.5 — required for the founder-loop wedge demo's halt-and-resume-overnight pattern.)
- **FR51:** Director can instantaneously pause, resume, or shift posture of any Spirit including: (a) interrupting in-flight autonomous actions with bounded-time guarantee (P99 ≤2s), (b) preserving Spirit state across pause/resume without reload, (c) recalling pending Orchestrator-buffered actions per FR20, (d) revoking any active capability token with in-flight operations failing-safe within bounded time. Override is auditable per FR42 with director identity and reason. Operationalizes the director's autonomy-spectrum control surface that defends the theater/actor/director metaphor.

### D. Multi-Spirit Coordination (11 FRs)

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

### E. Memory, Cognition Substrate, and Distillation (7 FRs)

- **FR27:** Spirit can write working-memory tagged scalars via `working_memory.set_scalar(tag, value, derived_from)`. The kernel persists and routes tagged scalars by tag identity without interpreting tag-specific semantics — kernel performs only universal-arithmetic comparison via four predicates (`on_value_above`, `on_value_below`, `on_value_within`, `on_value_outside`). Per §4.0.7: kernel performs no Spirit-specific cognitive computation (no variance, entropy, EFE, KL, ensemble disagreement, derivatives, or statistical tests — Spirit computes those itself).
- **FR28:** Spirit can write to private, shared, or collective memory tiers (per I5); memory compaction is Spirit-authored — the Spirit's persona logic declares compaction policy; kernel provides persistence and quota enforcement only.
- **FR29:** Spirit can recall historical Transparency Log frames it was a participant in via `log.recall(filter, limit, cursor)` with payload-on-demand fetch via `log.fetch(frame_id)`; kernel scopes results to participant frames and honors A2A consent envelopes. Distillation work — selecting which frames to preserve, summarizing, abstracting — is Spirit-authored.
- **FR30:** Spirit can produce distillates (digests) via Spirit-side LLM compression; kernel enforces I11 audit-chain on digest writes (mandatory `source_log_ref` flattened to original raw frames, `distillation_depth`, `intent_lineage`).
- **FR31:** Spirit can write principal-related data to the `principal:<principal_id>:<spirit-author-defined-schema>` namespace (per ADR-026; principal-namespace pattern informed by hermes-agent's principal-scoped memory model lifted into a kernel-allocated contract); data inherits subject-access query, right-to-be-forgotten, and redaction-on-export operations. The kernel allocates the namespace and enforces isolation; the kernel does not index or interpret content.
- **FR32:** Spirit author can declare per-tag epistemic policies referencing tagged scalars and the four universal-arithmetic predicates; kernel triggers halts when predicates fire and journals halt reason with structured payload (tag, value, threshold, policy_id, derived_from). Cognitive work — choosing the threshold, designing the predicate semantics, computing the underlying scalars — is Spirit-authored. Predicate-firing recall floor ≥0.85 per Spirit class; precision floor ≥0.85.
- **FR56:** Spirit can read its own performance telemetry (success/failure counts, latency distributions, halt-recall events, distillation outcomes) scoped to its principal namespace per FR31, without requiring per-read operator admission. Self-telemetry feeds Spirit-side calibration and skill-revision proposals (FR57). Spirit's own data; Spirit reads it.

### F. Spirit Ecosystem and Distribution (12 FRs)

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

### G. Audit, Compliance, and Operator Surfaces (11 FRs)

- **FR41:** Operator can run frame-by-frame log queries via authenticated audit interface with filters by Spirit, capability, time-range, frame-kind, and tag. Query latency floor: P99 ≤2s for queries scoped to a single Spirit on a 30-day window; P99 ≤10s for global queries; completeness floor on a per-commit log-completeness corpus with N=100 injected events: ≥98/100 events recoverable from logs (per NFR-Aud-1). Query language is specified separately (audit-query-surface ADR — extension to ADR-013).
- **FR42:** DPO can run subject-access queries via `maosctl audit subject-access --principal <id>`; returns all principal-namespace entries across all Spirits with provenance (Spirit, time, derived-from observations).
- **FR43:** CISO can run posture-delta queries via `maosctl audit posture-delta --range=<timespan>` surfacing capability-scope changes, sandbox-tier changes, and consent-policy changes over a configurable time-range with approval-chain attribution.
- **FR44:** External regulator can request a sealed-export via `maosctl audit sealed-export <bundle-spec>`; bundle is Ed25519-signed by the operator's audit key, third-party-verifiable, conforms to `maos.audit-bundle.v1` schema.
- **FR45:** User can exercise GDPR Article 17 right-to-be-forgotten via `maosctl forget --principal <id> [--reason <legal-hold>]`; kernel removes all principal-namespace entries; the deletion event itself is journaled (preserving lifecycle invariant) but the principal data is gone. Cross-Spirit cascade: forgetting cascades to working-memory references in other Spirits where principal data was shared; distillates containing principal data are marked redacted with re-distillation triggered. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries.
- **FR46:** Operator can export filtered raw trajectories via `journal.export(filter, redaction_policy)` per ADR-023; bundle conforms to versioned `maos.trajectory.v1` schema with Ed25519 signing and applied-redaction flag.
- **FR61:** Substrate project publishes and maintains `SECURITY.md` documenting (a) disclosure contact (`security@maos.dev` with published GPG key), (b) coordinated-disclosure window and CVE-assignment process, (c) supported-versions matrix for security backports, (d) advisory-publication channel. v0.1 binding — not deferred; security disclosure pipeline must exist before any Spirit is shipped to a third party.
- **FR62:** Substrate exposes audit-queryable artifacts for governance: (a) vetter-key admission and rotation events, (b) ABI-extension proposals and their ratification status, (c) ComplianceClaim schema versions and their effective dates. Operationalizes Constitutional Substrate Evolution (Innovation #7 from Step 6). **[9.3 preflight errata 2026-06-13 · party-mode F5, Winston]** (c) is satisfied by a governance **schema-lifecycle event stream** that *references* ComplianceClaim identity and carries schema-version + effective-date **on the event**, NOT on the claim — the binding-v0.1 `Claim` struct is FROZEN and MUST NOT gain fields (neither on the struct nor its envelope). Implemented in Story **9.3b**.
- **FR63:** All kernel-emitted errors carry stable typed codes from a published catalog at `https://docs.maos.dev/errors/<ERR_NAME>` with documented retryability, cause-chain semantics, and version-stability guarantees consistent with the LTS policy. CI-enforced metadata per error variant; v1.0 binding (catalog initial set covers the 14+ named errors documented in architecture-maos.md). **[9.3 preflight errata 2026-06-13 · party-mode F2/F3, John+Winston]** "14+ named errors" → the **complete kernel-emitted `E*` error set (N as of v1.0)**, locked as a bidirectionally-CI-checked registry (count is an output, not a target). CI invocation is **`cargo xtask error-catalog-check`** (repo convention — CI checks are xtask subcommands), NOT `cargo run --bin`. FR63 is the kernel-neutral half, implemented in Story **9.3**.
- **FR64:** Operator can attribute cost (token-spend per provider, subprocess CPU-time, storage I/O) per Spirit per task per principal in the Transparency Log. Enterprise-readiness gate — no enterprise deployment without per-tenant cost accounting. **[9.3 preflight errata 2026-06-13 · party-mode F8, Winston/Amelia + verified]** Attribution is sourced AT THE INFERENCE EMISSION SITE: **principal** via `transparency_log.principal_ids_for_spirit_pid` (lineage-derived, **never caller-supplied** — anti-spoofing); **task** via an additive `task_ref` **correlation-id** (verified: lineage cannot resolve the current task — SCB has no current-task marker). `principal_id` is NOT a caller-supplied ABI field. Implemented in Story **9.3b**.
- **FR65:** Operator can uninstall a Spirit; kernel emits a proof-of-erasure record enumerating all removed substrate state (memory namespace per ADR-026, capability tokens, pending halts, intent lineage references, scheduled invocations). Defends the v1.0 hermes-tenant positioning claim that substrate-uninstall is a real guarantee, not a hope.

## NonFunctional Requirements

**Total: ~85 NFRs across 13 categories.**

### Performance (8 NFRs)

- **NFR-Perf-1:** IAC frame routing latency P50 < 5ms, P99 < 50ms on a typical Linux box (NVMe + 16-core tier). v0.5.
- **NFR-Perf-2:** Sustained IAC frame throughput 5,000–10,000 frames/sec single-host before log writer becomes bottleneck. Per-Spirit fairness scheduler in front of log writer (NOT FIFO). v0.5.
- **NFR-Perf-3:** Capability-token validation latency P99 < 100µs per check; 100% re-validation at use against current state, not cached state (TOCTOU correctness). v0.1.
- **NFR-Perf-4:** Posture-shift propagation P99 ≤ 2s, P99.9 ≤ 5s in 1000-shift corpus. v0.3.
- **NFR-Perf-5:** Audit query latency P99 ≤ 2s for single-Spirit queries on 30-day window; P99 ≤ 10s for global queries. v0.5 (basic), v1.0 (signed-export tier).
- **NFR-Perf-6:** Distillation step latency budget declared per Spirit class via manifest `[budget].time_cap`; soft warning at 80%; kernel emits `BudgetWarning` IAC frame. v0.5.
- **NFR-Perf-7:** Hot-swap latency P99 < 500ms (mode switch + state transfer + capability rebinding) for same-major same-additive swaps. v0.8.
- **NFR-Perf-8:** Orchestrator fan-out — sustained 50 concurrent Worker Spirits with task-dispatch latency P99 ≤500ms; 0 dropped tasks under 10 tasks/sec sustained for 1 hour. Backs FR21's fan-out floor. v0.8.

### Reliability (11 NFRs)

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

### Security (19 NFRs)

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
- **NFR-Sec-17:** Enterprise PDP integration — capability-authorization decisions sourced from a real external Policy Decision Point behind out-of-kernel `PolicyDecisionPort` (maos-domain), evaluated by the PDP's real engine (Cedar in-process reference in `maos-pdp`); fail-closed on PDP unavailability; absent→BLOCK@v2.0. Anchor: ADR-050.
- **NFR-Sec-18:** Enterprise identity assertion (OIDC) — out-of-kernel `IdentityAssertionPort` + `maos-sso` reference verifier; explicit algorithm allowlist; reject `alg:none` and HS256-confusion; enforce signature/issuer/audience/time claims; configured-but-down SSO denies issuance. Honesty: one reference OIDC path; SAML/social/discovery adapters deferred. Gate: `check-enterprise-identity`; anchor: ADR-051.
- **NFR-Sec-19:** Opt-in adapter-store at-rest AEAD — out-of-kernel `KeyManagementPort` + `maos-secrets` envelope helper; ciphertext≠plaintext, right-key opens, wrong-key fails, KMS-down refuses sealed writes. Honesty: opt-in adapter-store scope only; Option-A plaintext remains default; kernel-core Private/Shared and production KMS adapters deferred. Gate: `check-enterprise-identity`; anchor: ADR-051.

### Auditability & Compliance (14 NFRs)

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
- **NFR-Aud-11:** SIEM export at v2.0; OpenTelemetry adapter at v1.0. Story 11.4c delivers the SIEM half via `maos-siem` read-only TL export through `query_with_redaction` before NDJSON+CEF/RFC5424 projection; empty TL reports N/A, not a green zero. Honesty: production network sinks are additive; HTTPS/network must be TLS-only and plaintext/file localhost-only. Gate: `check-enterprise-identity`; anchor: ADR-051.
- **NFR-Aud-12:** Storage cascade erasure completeness + externally-verifiable uninstall receipt. Substrate-uninstall produces a portable, externally-verifiable erasure receipt (signed Merkle inclusion + signed Merkle exclusion proof, retained independent of the substrate). 100% of registered storage backends prove erasure within bounded window for any given principal. Closes the weakest leg of the hermes-tenant positioning sentence. v1.0.
- **NFR-Aud-13:** Time-to-erasure SLA. Floor: 95% of right-to-be-forgotten requests complete within 30 days (configurable to 7 for enterprise tier); audit log entry within 24h of request acceptance. v1.0.
- **NFR-Aud-14:** Intent-lineage propagation completeness — 100% of cross-Spirit IAC frames carry unbroken lineage chain back to originating principal intent. Closes ADR-018/I13 NFR coverage gap. v0.8.

### Testability (14 NFRs)

- **NFR-Test-1:** All ship-gate test corpora are static artifacts content-addressed in the repo (SHA-256 of JSONL); generation provenance is documented but not required to be reproducible. Pinned model versions, temperature=0 for judge calls, top_p=1.0, seed where supported, prompt-version hash committed alongside, retry budget=1, quarterly re-baseline with ≥98% agreement on golden snapshot. v1.0.
- **NFR-Test-2:** Kernel-API surface invariant test (per-commit gate). Build-time reflection enumerates every kernel API exported to Spirits via `kernel::api::*`; classifies each function by computational class (universal-arithmetic / data-movement / supervision / **other**); floor: 0 functions in class "other"; new function entering class "other" is build-break. Static analyzer on Rust `syn` walking allowlist-based predicate definitions; decidable for permitted subset (no theorem prover). Kernel-utility crate (`kernel::util::*`) has separate looser invariant: no I/O except via injected trait, no global state. v0.1 build gate (surface-diff only); v0.5 adds static analyzer for predicates.
- **NFR-Test-3:** spirit-test SDK harness coverage ≥ 80% of Spirit author's manifest-declared capabilities reachable via fixtures; validated by external-author trial in 5+ third-party Spirits. v1.0.
- **NFR-Test-4:** Halt-recall ≥ 0.7 / halt-precision ≥ 0.85 per Spirit class on `bmad-eval` standard corpus. v0.5.
- **NFR-Test-5** [PHASE-SPLIT per John]: FKCS (Frozen-Kernel Conformance Suite). FKCS-infrastructure (diff oracle, test harness, kernel-frozen-vN.0 commit-tagging) DELIVERED at v2.0 (ADR-052 / Story 11.5; the v2.0 cohort is in-house Chinese-wall proxy authors, NOT genuine externals); FKCS-populated — 3 genuine externally-authored Spirits — REMAINS v2.5. The N=12 stratified black-box third-party trial is owned separately by **NFR-Test-8** and was never part of FKCS-populated scope (it was silently dropped from earlier FKCS-populated wording; restored here as an explicit cross-reference). Floor: ≥27/30 per Spirit, ≥85/90 aggregate; the v2.0 in-house Chinese-wall proxy cohort demonstrates the scoring mechanism only and does **not** satisfy the genuine-external FKCS floor. Diff oracle DERIVES zero frozen-surface changes (never a self-reported flag); negative-control "fourth Spirit" declares an off-frozen-surface / `pub(crate)`-style kernel internal and is **rejected at admission by `maos_registry::admission::admit_spirit`** (a journaled, falsifiable `AdmissionError::OffFrozenSurface`), not only by an FKCS-internal gate. **[ADR-052 · Story 11.5; scope source: PRD NFR-Test-5 + Epic-11 Decision 7]**
- **NFR-Test-6:** LCAS (Long-context Ambiguity Stress) corpus — N=210 scenarios in 3 buckets (clearly-decidable n=70 / genuinely-ambiguous n=70 / adversarially-misleading n=70). Adversarial trajectories contain a planted load-bearing claim contradicting a louder repeated claim. v0.5 ship gate.
- **NFR-Test-7:** Cross-form Semantic equivalence (rust-inproc ↔ subprocess) ≥ 90%; (any-rust ↔ wasm-component) ≥ 75%. CLI-wrapper requires distributional behavioral equivalence (Mann-Whitney U-test p > 0.05 over 30 runs). v1.5 (rust↔subprocess; cohort interop at v1.0 is rust-rust); v2.0 (any-rust↔wasm).
- **NFR-Test-8:** Black-box third-party trial v1.0 — N=12 stratified (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). 14-day no-DM-support window. Floor: ≥10/12 produce working signed Spirit binary that loads on fresh Host VM, runs ≥1000 frames, halt-recall ≥0.85 on the class-appropriate subset. Wilson CI [0.552, 0.962] meaningful at N=12; meaningless at N=5. Auditable via SBOM + signing chain re-loaded on clean VM by CI bot. Phase split per Story 11.7 / ADR-053: v2.0 infrastructure delivered by `check-trial-attestation` with in-house Chinese-wall proxy advisory proof-of-mechanism and SBOM/signing derived-and-asserted; CycloneDX/SBOM emission and genuine external N=12 execution are deferred to v2.5/Epic-14. Minor releases use NFR-Onb-1 (12-author onboarding) as proxy.
- **NFR-Test-9:** Loom-not-in-kernel structural test. `grep` of kernel crate for orchestration/planning symbols returns ∅. Per-commit gate. Covers ADR-006's negative commitment (Loom is user-space). v0.5.
- **NFR-Test-10:** Skill-format conformance — at least one third-party skill format (Anthropic Skills format OR equivalent) executes via Spirit-form adapter without kernel modification. Covers ADR-027's external-standard interop assertion empirically. v1.5.
- **NFR-Test-11:** Namespace grammar lock test. Grammar `.lark` (or equivalent) hash pinned in CI; any change requires architecture-lock review process, not regular PR. v0.5.
- **NFR-Test-12:** v0.3 architecture lock script as per-commit gate. `scripts/check_v0_3_lock.sh` runs four mechanical checks: (1) `LICENSE` matches ADR-decided license string; (2) consortium-target ADR exists with status `accepted` and ≥2 maintainer sign-offs; (3) `ROADMAP.md` has trust-anchor decision section with status `decided` linking to ADR; (4) failure-semantics doc exists with at least one fully-specified route. No v0.3 tag without script in green. v0.3.
- **NFR-Test-13:** Manifest field test coverage ≥ 3 cases per field (well-formed, malformed-rejected, edge-case); CI-enforced. v0.1.
- **NFR-Test-14:** Wire protocol cross-language byte-equal golden corpus per frame variant per SDK (Rust + TS v0.5 + Python v1.0 + Go v1.5+). v1.0.

### Meta-Testing (3 NFRs)

- **NFR-Meta-1:** Corpus-quality audit. Each ship-gate corpus reviewed by independent assessor (not corpus author) on a 10-point rubric (representativeness, edge-case coverage, label correctness, distribution match to production). Floor: ≥8/10 per corpus. Cadence: at corpus creation + every 12 months. v1.0.
- **NFR-Meta-2:** Corpus-staleness. Every corpus carries a `valid_until` date in metadata. CI fails if any active gate references an expired corpus. Default validity: 12 months. Extension requires explicit "no-update justification" PR with assessor sign-off. v1.0.
- **NFR-Meta-3:** Coverage matrix. Single source-of-truth file `tests/coverage-matrix.yaml` mapping {FR, NFR} → {corpora, gates}. CI fails if any FR/NFR with phase-status `delivered ≤ current-phase` has zero corpus coverage. Floor at v1.0: 100% coverage of FRs/NFRs delivered ≤ v1.0. v1.0.

### Observability (5 NFRs)

- **NFR-Obs-1:** Author-observability contract — Spirit author can read same diagnostic surface as operator for their own Spirit, redacted of cross-Spirit data. Metric M is queryable in <500ms with cardinality ≤10⁴. v1.0.
- **NFR-Obs-2:** OpenTelemetry export per IAC frame, capability invocation, halt event. v0.5 basic; v1.0 SLO-class.
- **NFR-Obs-3:** Per-Spirit telemetry stream with topic-based broadcast + filtered subscription. v0.3 (Butler narrow); v0.5 (Observer broad).
- **NFR-Obs-4:** Transparency Log per-Host SQLite (append-only), exportable to JSONL/SIEM with redaction policy applied. v0.5.
- **NFR-Obs-5:** Approval Decision Log distinct from Transparency Log; full intent + decision + reasoning chain per Invariant I4. v0.3.

### Documentation Quality (7 NFRs)

- **NFR-Doc-1:** Every public ABI method has ≥ 1 doctested example; CI broken-link blocking on doc site at v0.5; doctest CI gate at v0.1.
- **NFR-Doc-2:** Typed error catalog at `https://docs.maos.dev/errors/<ERR_NAME>` covering all 14+ named typed errors. CI-enforced metadata: each variant has 6 fields (code, severity, recovery-class, owner, kernel-or-spirit, since-version). v1.0. **[9.3 preflight errata 2026-06-13 · party-mode F1/F2, Murat]** "all 14+ named typed errors" → the **complete kernel-emitted `E*` set (N as of v1.0)**; the catalog is a **bidirectionally**-CI-checked registry (`xtask/error-catalog.toml`) — both an un-catalogued emitted error AND a stale registry entry fail CI — plus a negative meta-test proving the checker can fail. See Story **9.3**.
- **NFR-Doc-3:** API reference site at `https://docs.maos.dev/abi/<version>/`; versioned, searchable, deep-linkable, archived ≥ 2 minor versions back. v1.0.
- **NFR-Doc-4:** Five canonical doc deliverables published with CI-verifiable minima — Manifest schema reference (≥1 example per field); Pattern cookbook (≥10 patterns); Migration runbooks; Troubleshooting guide (covers 100% of FR63 catalog); Deployment topology guide. v1.0.
- **NFR-Doc-5:** WCAG AA compliance for doc site. v1.0.
- **NFR-Doc-6:** Localization v1.0 = Korean only (shipped); Japanese + Chinese-simplified at v1.5. `LOCALES.md` with glossary lock — terms NEVER translated: Spirit, Worker, kernel, ADR identifiers, error codes.
- **NFR-Doc-7:** Doc tooling supports per-locale builds + fallback to English + language switcher with deep-link preservation + version dropdown. RTL layout deferred to v2.5. mdBook + i18n / Docusaurus / VitePress decision by v0.5; v1.0 in production.

### Onboarding (4 NFRs)

- **NFR-Onb-1:** 30-Min First Spirit Validation Gate. N=12 stratified external Spirit authors (≥4 with no prior MAOS contribution; ≥3 who've never written Rust Spirit; ≥2 who've never written Rust at all; ≥2 non-English-native; ≥1 working offline-only). Floor: median ≤ 45 min, p95 ≤ 90 min, AND ≥ 10/12 succeed where "succeed" = author produces Spirit binary that (a) compiles against published ABI, (b) passes the v0.3-grade Butler-class regression corpus (30-scenario calendar/comms; halt-recall ≥0.90 on calendar-conflict subset; halt-precision ≥0.85 overall), (c) does so within 14 calendar days from kit handoff with zero direct-message support. v0.3 release criterion.
- **NFR-Onb-2:** First-time installer J0 evaluator path — install + first useful Spirit response within 5 minutes. v0.1.
- **NFR-Onb-3:** Three-door page at `docs.maos.dev` ("write a Spirit" / "run MAOS" / "understand MAOS"). v0.5.
- **NFR-Onb-4:** 30-Min Gate iteration cadence. If floor missed, run fresh 6-author cohort within 2 weeks; three consecutive misses escalate to v0.3 release-criterion review. v0.3.

### Maintainability (9 NFRs)

- **NFR-Maint-1:** Kernel trusted core ≤ 20 KLOC excluding tests through v2.0 (core scheduler + IAC bus + capability check + journal). Integration adapters in separate crates with their own LOC budgets. v2.0.
- **NFR-Maint-2:** Capability-registry fuzz coverage ≥60% line at v0.1; ≥80% line / ≥60% branch at v0.5 on 1M-iteration libFuzzer run; zero crashes.
- **NFR-Maint-3:** ABI compatibility matrix 100% within current major; 100% N-1 boundary including negative typed-error cases. v0.1 (within-major); v1.0 (N-1).
- **NFR-Maint-4:** STABILITY.md publishes live (kernel_version, abi_version, manifest_schema_version) compatibility matrix. v1.0.
- **NFR-Maint-5:** Deprecation timeline: 2 minor releases of warning, 1 major release to remove. v1.0.
- **NFR-Maint-6:** 1-year LTS commitment at v1.0; 2-year LTS commitment at v1.5 once support load is known; security-only patches after year 1.
- **NFR-Maint-7:** BREAKING.md required entry for every breaking change with migration steps; CI grep-enforced. v1.0.
- **NFR-Maint-8:** Capability-token TOCTOU test: 100% re-validation at use against current state. v1.0.
- **NFR-Maint-9:** Manifest schema N-1 compatibility — kernel version V can load manifests written for V-1 with documented degradation paths. Closes ADR-025 NFR-coverage gap. v1.0.

### Scalability (5 NFRs)

- **NFR-Scale-1:** Cortex 3-region pilot at v2.0 with ≥ 10 agents minimum; sustained operation for 30 days; zero substrate-invariant violations.
- **NFR-Scale-2:** 25-host churn test at v2.0; 100-host churn at v2.5 (cost compression: 100→30 hosts at v2.0 same churn-events-per-week, full 100-host moves to v2.5).
- **NFR-Scale-3:** Per-Spirit fairness scheduler in front of log writer (NOT FIFO). Algorithm: Deficit Round Robin (DRR) with per-Spirit weight=1 by default; operator-configurable weights via `[scheduler.weights]`. Floor: under uneven load (1 noisy Spirit at 10× the median write rate alongside ≥4 normal Spirits sustained for 60s), max-min P99 latency ratio across Spirits ≤ 3.0. v0.5.
- **NFR-Scale-4:** Provider rate-limit isolation — per-(provider, credential) token bucket; typed `RateLimited` IAC frame. v0.5.
- **NFR-Scale-5:** Multi-host A2A peer mesh scales to 14-institution Cortex; v2.0 target with documented capacity envelope.

### Operational (12 NFRs)

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

### Compliance & Regulatory (5 NFRs)

- **NFR-Comp-1:** Export-control classification artifact. ECCN classification letter on file, EAR99 vs 5D002 determination published in `STABILITY.md §Export`, dual-use review for crypto primitives in kernel. v0.8 (before any v1.0 enterprise-distribution conversation).
- **NFR-Comp-2:** Vetter accreditation parameters — published vetter qualification matrix, conflict-of-interest disclosure required, vetter rotation policy (no single vetter on >40% of Spirit-class promotions in any 12-month window), vetter audit-trail retained 7 years. v1.0.
- **NFR-Comp-3:** Substrate-self compliance scope declaration. `STABILITY.md` contains scope-disclaimer paragraph explicitly stating SOC 2 / ISO 27001 / FedRAMP scope is the *operator's* responsibility, with kernel-as-service boundary drawn. v0.5.
- **NFR-Comp-4:** Region-pinning primitive (PIPL §40 / data localization). Transparency Log + working-memory store configurable to single jurisdictional region with cryptographic enforcement against cross-region replication. v1.0.
- **NFR-Comp-5:** Spirit model-provenance manifest field (SB-1047 / Colorado AI Act adjacent). Manifest declares covered-model identifier, training-data lineage, last-eval timestamp; substrate validates field presence at admission. v1.0.

### Cost & Tenancy (2 NFRs)

- **NFR-Cost-1:** Cost-attribution accuracy ≥ 98% reconciliation against provider billing, sampled monthly. Per-Spirit per-task per-principal attribution. Without this NFR, FR64 (cost accounting) is theater. v1.0. **[9.3 preflight errata 2026-06-13 · party-mode F9, Murat/Winston/John]** Split into two non-conflated claims: **(CI gate)** computed cost reconciles **100% / deterministic** against a committed **synthetic price-book fixture**, where the oracle is **independent golden vectors that do NOT import the pricing function** + rounding/aggregation property tests + integer **micro-units (no `f64` in the accumulation path)**; **(operational SLO)** the **≥98%** band reconciles against *real* provider invoices via a **weekly sampling runbook** — explicitly NOT a CI gate (real invoices are non-deterministic external input, the same untestable-non-oracle killed in 9.2b F3). Implemented in Story **9.3b**.
- **NFR-Tenancy-1:** Explicit single-tenant per kernel instance commitment through v2.0; multi-tenant primitive-reserved at v1.0 per NFR-Ops-11; full multi-tenant out of scope before v2.5. v0.1 (declared); v2.0 (single-tenant guaranteed).

## Additional Requirements

### Starter template / project skeleton **[FLAG: drives Epic 1 Story 1]**

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

### Infrastructure / deployment requirements

- **[ARCH]** **Two deployment topologies** ship at v1.5 (configuration alone, no architectural rewrite): (1) Single-user single-Host (laptop/workstation, 5–10 Spirits cooperatively scheduled, optional Loom-lite Postgres for founder loop); (2) Diagnostic-architect bilateral 2-Host pair (Host A prod-edge running Mira; Host B dev-environment running Nash; pre-paired with mTLS cert fingerprints; mobile push to operator).
- **[ARCH]** **Persistence layered:** SQLite per-Host (Transparency Log + Approval Decision Log + Journal) at v0.1; Postgres+pgvector for Loom-lite (collective tier) at v1.5 only. Migration test (SQLite→Postgres) gates v1.5 (NFR-Ops-10).
- **[ARCH]** **Distribution:** v0.1 source build (`cargo install --path crates/maos-bin`) Linux + macOS only. v0.5 pre-built binaries (Linux amd64/arm64, macOS arm64) via GitHub Releases with SHA256 + Ed25519 verification mandatory. v1.0 Homebrew tap, AUR, deb, rpm; container images on Docker Hub / GHCR. Windows binary at v1.5. v2.0 official Linux distro packages + one-line install script.
- **[ARCH]** **Air-gapped deployment** validation in CI via network-namespace isolation (NFR-Ops-12, v1.0). Offline-import path for signed artifacts (FR60).
- **[ARCH]** **Logical clock discipline:** Lamport or hybrid logical clock (final pick by v0.5) for cross-Host frame ordering; wall-clock is metadata only. Certificate validity windows remain wall-clock.
- **[ARCH]** **Backup/DR:** Loom-lite RPO ≤1h, RTO ≤4h, backup integrity verified weekly via Merkle-root cross-check (NFR-Ops-9, v1.0).

### Integration requirements

- **[ARCH]** **Four-protocol commitment:** kernel-internal IAC + bilateral A2A + ACP + MCP. Substrate invents no new wire protocols.
- **[ARCH]** **Same-Host IAC:** `tokio::sync::mpsc` + `tokio::sync::broadcast` channels addressable by SpiritId; bounded queues; backpressure via Spirit Scheduler.
- **[ARCH]** **Subprocess Spirit Wire Protocol (ADR-032, binding-v0.1):** LSP-style `Content-Length: <decimal>\r\n\r\n` framing followed by N bytes of CBOR-encoded payload. Header ASCII case-insensitive, max header block 4 KiB. `BufReader` cap = 1 MiB; oversize = `WireError::Oversize`. Writer over bounded `mpsc<Frame>(64)`. Stderr piped to `tracing` at WARN; never multiplexed onto stdout. Clean EOF after frame = `Halt::Voluntary`; mid-frame EOF = `Halt::Fault(Truncated)`. SIGTERM → 5-second grace → SIGKILL.
- **[ARCH]** **Cross-Host (bilateral A2A):** mTLS over TCP, JSON-RPC framing. Each Host's deployment configuration names the other Host's mTLS cert fingerprint (no discovery). Per-frame ADR-012 typed-intent consent. Logical-clock ordering. Network-partition: in-flight frames NACKed after configurable timeout (default 30s); kernel does NOT auto-retry.
- **[ARCH]** **MCP client (ADR-008):** all-three transports (stdio / SSE / Streamable HTTP). Streamable HTTP is default. T4 WASM tool sandboxing for untrusted MCP at v1.0+; Spirits via WASM at v2.0.
- **[ARCH]** **ACP server:** NDJSON over stdio for editor-hosted Spirits. v1.0 with Zed + VSCode tested. JetBrains via plugin-bridge at v1.5.
- **[ARCH]** **Spirit registry (ADR-008, binding-v0.5):** itself an MCP-Streamable-HTTP server. `registry.search` / `registry.manifest` / `registry.artifact` / `registry.publish` / `registry.deprecate`. **Three trust tiers** at v1.0: `local`, `org-internal`, `public-untrusted`. (`public-vetted` adds at v2.5 via FR37 — deferred. PRD's four-tier mention reconciled to architecture's three at v1.0.)
- **[ARCH]** **LLM provider drivers (ADR-005):** v0.1 = Anthropic. v0.5 = ≥3 providers in CI (Anthropic + OpenAI + local-LLM via Ollama). v1.5 = MAOS-mediated provider proxies. v2.0 = full multi-provider including Bedrock/Vertex AI/local LLMs.
- **[ARCH]** **Skill ecosystem:** filesystem-discovered at v0.5 (conventional locations `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`); optional skill registry at v2.0; `maos.skill.v1` format intentionally close to Anthropic Skills format.

### Data persistence / migration / setup requirements

- **[ARCH]** **Three memory tiers (Memory Manager, I5-enforced):** `private` (per-Spirit); `shared` (Host-wide, SQLite-backed kv with namespace prefix per writer); `collective` (Loom-lite Postgres+pgvector via MCP-Streamable-HTTP, v1.5).
- **[ARCH]** **Principal Memory Namespace (ADR-026, binding-v0.5):** typed namespace `principal:<principal_id>:<schema>` within private tier. Inherits subject-access query, right-to-be-forgotten, redaction-on-export.
- **[ARCH]** **`memory.md` convention:** Spirits MAY persist a `memory.md` file in their private namespace (universal cohort convention). Kernel does not interpret.
- **[ARCH]** **Hot-swap state-transfer wire format (ADR-017, binding-v0.3):** CBOR-encoded payloads conforming to per-Spirit-class schema declared in manifest (`[hot_swap].state_schema_uri` + `state_schema_version`). Compatibility rules: same-major + additive forward = forward-compat; same-major + breaking = forbidden; cross-major requires explicit migrator. Saga-style compensation; auto-revert within 30s on post-swap invariant violation.
- **[ARCH]** **Cross-major migration (ADR-020):** `migrate(predecessor_state) -> Result<successor_state, Error>` declared via `migrates_from`; kernel refuses load with `EMigratorMissing` if predecessor archive exists and no migrator declared.
- **[ARCH]** **Lifecycle journal (I10):** append-only on-disk log of all lifecycle transitions; fsync per state transition; ring-buffer flush latency < 1ms (NFR-Rel-8). Crash recovery rehydrates from journal.
- **[ARCH]** **Replay determinism (ADR-028):** over the **shape of the trace** (frame ordering, capability-token issuances, halt events, decision-frame emission), NOT redacted payload content. `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12) validated in CI. v1.0 best-effort, v1.5 hard target.

### Observability requirements

- **[ARCH]** **Three logs coexist; do not conflate:** `tracing` (internal spans/debug); Telemetry Stream (typed broadcast events); Transparency Log (every IAC frame, approval, capability use, retract; SQLite append-only; durable; the personal audit trail).
- **[ARCH]** **Approval Decision Log (I4):** separate from Transparency Log. Captures `(actor, target, capability, intent, decision, reasoning_if_any)` for every approval prompt resolution.
- **[ARCH]** **Telemetry Stream IAC round-trip metrics (binding from v0.1):** `iac_rt_duration_us` (histogram, microseconds; labels: `service ∈ {security, memory, iac, capability, spirit_scheduler}`, `outcome ∈ {ok, err, timeout}`); `iac_rt_inflight` (gauge); `iac_rt_errors_total` (counter). Histogram buckets anchored on 1500µs SLO: `[50, 75, 100, 150, 200, 300, 450, 700, 1000, 1500, 2200, 3300, 5000, 7500, 11000, 16000, 25000, +Inf]`.
- **[ARCH]** **`scalar.tap` channel (ADR-035, binding-v0.5):** dedicated read-only stream from Capability Registry's tagged-scalar slot. Every `working_memory.set_scalar(tag, value, derived_from)` write emits `(spirit_id, tag, value, timestamp)`. Observer Spirits subscribe to see pre-halt scalar drift.
- **[ARCH]** **OpenTelemetry export adapter:** v0.5 basic; v1.0 SLO-class. SIEM export at v2.0.
- **[ARCH]** **Spirit-form measurement gate (§13.1):** `benches/iac_roundtrip.rs` using `criterion`. Three workloads (J1 floor, J-Butler, J-Researcher). Per-journey latency budgets (subprocess v0.1): J0 Butler conversational < 400ms P95 / IPC < 60ms; J1 Founder loop CliWrapper IPC < 25ms P95; J4 Mira-Nash Observer colocation < 10ms P95; J6 Diego cold-start < 500ms.

### API versioning / compatibility

- **[ARCH]** **ABI Stability Triple:** `(kernel_version, abi_version, manifest_schema_version)`. `abi_version` governs Spirit/KernelHandle vtable + capability ID space. `manifest_schema_version` governs TOML surface independently. `kernel_version` is product-facing.
- **[ARCH]** **N-1 supported, N-2 hard refusal** with typed `EAbiTooOld`. Deprecation timeline: 2 minor releases of warning, 1 major to remove. Spirit-side `kernel.deprecation_warnings()` channel surfaces deprecations in `spirit-test`.
- **[ARCH]** **STABILITY.md:** carries live (kernel, abi, manifest_schema) compatibility matrix + LTS branch policy + substrate-self compliance scope clause + export-control classification.
- **[ARCH]** **`min_substrate_version` manifest field:** kernel rejects load if its own version is below the declared minimum.
- **[ARCH]** **ComplianceClaim schema (ADR, binding-v0.1):** schema is frozen, structural validator implemented, emit pipeline live on every Spirit decision. Schema validation 100%, emit-rate 100%. Adding any required field, removing any field, renaming, type-changing, or removing/reordering enum variants of `Verdict` / `PrincipleRef` / `EvidenceKind` bumps `ABI_VERSION`. Adding optional fields with `#[serde(default, skip_serializing_if = "Option::is_none")]`, additive enum variants with explicit `#[repr(u8)]` discriminants and `#[serde(other)]` fallback — does NOT bump.

### Security / sandbox / capability implementation requirements

- **[ARCH]** **Sandbox tiers (ADR-004, binding-v0.1):** T0 (no sandbox; trusted local-tier) → T1 (process isolation; UID separation) → T2 (Linux: Landlock+seccomp; macOS: Seatbelt with `.sbpl`; Windows: restricted-token, default for `public-untrusted`) → T3 (T2 + container, Docker/Podman) → T4 (WASM, v2.0 for tools at v1.0).
- **[ARCH]** **Strictest-of-(manifest, trust-tier, operator-policy) floor.** Public-untrusted Spirit declaring T0 is forced to T2.
- **[ARCH]** **Per-Spirit resource isolation (cgroups v2 / setrlimit / Job Object):** kernel sets at spawn, OS-enforced not Tokio-cooperation. Defaults declared in manifest `[resources]`; kernel applies strictest-of (manifest, operator policy).
- **[ARCH]** **Capability Registry decomposition (ADR-030, binding-v0.1):** `cap-tokens` (sharded `Arc<[CapShard; 64]>` lock-free, hot path <5µs P99); `cap-policy` (read-mostly, copy-on-write); `cap-audit` (bounded `tokio::sync::mpsc::channel(8192)` to single audit-writer task; slow path); `cap-quota` (per-Spirit atomic counters; emits `ContextPressure` at 80%, `ContextLimit` at 95%, `EContextExhausted` above 100%).
- **[ARCH]** **Capability-token TTL ≤60s for high-privilege ops (ADR-023, binding-v0.1).** Tokens bound to (Spirit-PID + boot-nonce + expiry); ed25519-signed; non-transferable. TOCTOU re-validation at every use against current state.
- **[ARCH]** **Pre-write secret-redaction filter at Transparency Log boundary** (universal to all logged frames). Floors per NFR-Sec-4 (10⁴ per-commit, 10⁵ quarterly, 1000-canary/month).
- **[ARCH]** **Approval class taxonomy (6 classes):** `readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Default policies operator-overridable per Spirit.
- **[ARCH]** **Pluggable crypto provider (`CryptoProvider` trait):** default `ring`/`rustls`. Alternates for FIPS 140-3, hardware-backed, post-quantum, on-prem HSMs. v1.0 architectural commitment (NFR-Sec-15).
- **[ARCH]** **ComplianceClaim envelope (binding-v1.0 first-class object):** Ed25519-signed, references execution-context fingerprint (manifest hash + version + trust tier + sandbox tier + capability scope set + provider-endpoint pinning + crypto-provider identity). Kernel verifies at admission with typed `EComplianceContextDrift` on drift.

### Kernel subsystem requirements

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

### Innovation primitives (load-bearing, novelty-claim-grade)

- **[ARCH]** Empty-kernel invariant (I9, ADR-006) — structural lint blocks new persistent fields outside `{Journal, TransparencyLog, CapabilityRegistry::tokens}`. Caching is structural; learning is forbidden.
- **[ARCH]** Epistemic halt (Layer-1, ADR-022) — tagged-scalar slot + four universal-arithmetic predicates.
- **[ARCH]** Distillation pattern with kernel-enforced audit chain (§9.5; I11+I12+I13).
- **[ARCH]** ComplianceClaim runtime-context attestation (ADR + §8.5).
- **[ARCH]** Typed-intent A2A consent + intent_lineage (ADR-012, ADR-018).
- **[ARCH]** Skill-package overlay model for heterogeneous CLI Spirits (ADR-021).
- **[ARCH]** Constitutional substrate evolution (ADR-037, `invariant-lock` CI gate).

### Notable cross-cutting concerns / risks

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

## UX Design Requirements

_N/A — this is a kernel/infrastructure project with no UX design document. Director's-surface user interactions (FR14–FR20, FR51) are CLI / ACP / mobile-push flows specified in the PRD as functional requirements; no separate UX spec exists._

## FR Coverage Map (Revised after party-mode convergence — 12-epic structure)

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
