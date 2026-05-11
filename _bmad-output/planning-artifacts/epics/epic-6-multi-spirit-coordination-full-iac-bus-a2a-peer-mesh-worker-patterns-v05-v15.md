# Epic 6: Multi-Spirit Coordination — Full IAC Bus, A2A Peer Mesh & Worker Patterns (v0.5 → v1.5)

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

### Stories

## Story 6.1: Ship the Full IAC Bus with Retract Primitive and DRR Fairness Scheduler

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

## Story 6.2: Dispatch Orchestrator Distillates with Intent-Lineage and CliWrapperSpirit Worker Pattern

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

## Story 6.3: Build the A2A Peer Mesh from Loopback to Cross-Host with mTLS Rotation Chaos

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

## Story 6.4: Wire Scheduled Invocations with ConsentRupture and Provider Rate-Limit Isolation

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

## Story 6.5: Gateway Sub-Modules (ADR-029) — Telegram / Slack / Discord / Signal / Email

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
