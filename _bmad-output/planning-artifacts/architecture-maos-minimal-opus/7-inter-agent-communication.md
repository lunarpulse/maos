# 7. Inter-Agent Communication

## 7.1 Same-Host: the mailbox

The IAC Bus on a single Host uses `tokio::sync::mpsc` and `tokio::sync::broadcast` channels addressable by `SpiritId`. Bounded queues; backpressure via the Spirit Scheduler. Modeled on codex's `Mailbox`.

**Frame shape:**
```jsonc
{
  "frame_id": "ulid:01J...",
  "timestamp": "2026-05-06T18:42:01.234Z",
  "logical_clock": 12847,
  "from": { "spirit_id": "...", "host_id": "..." },
  "to": [ { "spirit_id": "...", "role": null } ],   // role is null for direct addressing
  "kind": "task.assign | task.complete | decision.dispatch | epistemic.halt | telemetry.event | consent.request | retract | ...",
  "intent": "delegate | consult | review | broadcast | request | ...",
  "payload": { ... },
  "auto_marker": "human-authored | spirit-auto | spirit-drafted-human-approved",
  "consent_envelope": { "intent_class": "...", "scope": "...", "valid_until": "..." }
}
```

Every frame is logged before delivery (I2). The kernel writes the Transparency Log entry first, then routes to the recipient mailbox; the IAC Bus does not deliver frames the log refused to record.

### 7.1.1 Per-frame-kind channel class

Each `kind` selects a Tokio channel class, cardinality, and backpressure policy. The assignments below are the normative contract Story 3.1's ACs reference verbatim. Channel primitives are modeled on codex's `Mailbox` (see `codex core/src/agent/mailbox.rs` — appendix-a-cohort-prior-art-map.md).

| `kind` | Channel class | Cardinality | Capacity floor | Drop policy on full |
|---|---|---|---|---|
| `task.assign` | `mpsc` | 1:1 (Director → Spirit) | 64 | Backpressure (await capacity); no drop |
| `task.complete` | `mpsc` | 1:1 (Spirit → Director) | 64 | Backpressure; no drop |
| `decision.dispatch` | `mpsc` | 1:N (sequential per recipient) | 128 | Backpressure; no drop |
| `epistemic.halt` | `mpsc` | 1:1 (Spirit → kernel) | 16 | **Never drop** — halt frames are I14-critical; queue overflow signals broader failure |
| `telemetry.event` | `broadcast` | 1:N (Spirit → subscribers) | 256 | **Drop oldest** (broadcast lag tolerated; not audit-critical) |
| `consent.request` | `mpsc` | 1:1 (Spirit → Director) | 32 | Backpressure |
| `retract` | `mpsc` | 1:1 (sender → recipient) | 32 | Backpressure |

### 7.1.2 Backpressure hook points (Spirit Scheduler integration)

- Bounded-channel `send().await` blocks the calling task; Spirit Scheduler observes via per-Spirit pending-frame metric (`iac_pending_frames_total{spirit_id, kind}`) exported through `IacRtMetrics` (Story 1b.4).
- Hot-path budget: `send().await` may not exceed 1ms P99 in steady state; sustained exceedance is a Spirit Scheduler signal to throttle the sender (Story 5.1 wires the throttle).
- `retract` frames bypass capacity check for `decision.dispatch` queues only — retraction must be able to overtake the dispatch it cancels (per ADR-022).
- Cross-Host equivalents (A2A) inherit the same channel-class assignments at the `tokio::mpsc` bridge; backpressure is signaled across mTLS via flow-control window (out of scope for this addendum).

## 7.2 Cross-Host: bilateral A2A

Cross-Host communication uses A2A over mTLS+TOFU between two pre-paired Hosts. This is the topology the Diagnostic Engineer + Senior Architect bilateral pair runs on.

**Pairing model.** Each Host's deployment configuration names the other Host's mTLS certificate fingerprint. There is no discovery protocol because there is nothing to discover — the operator names the two endpoints. First-contact TOFU pinning verifies the configured fingerprint; subsequent connections re-verify against the pinned cert.

**Per-frame consent (ADR-012 typed-intent).** Each Host's manifest declares which intent classes it sends to its peer and which it accepts from its peer. The kernel rejects frames whose typed intent is not in the sender's send-allowlist or the receiver's accept-allowlist with `EIntentDenied`. This is what makes Mira's `diagnosis-handoff:read-only-evidence` admissible at Nash while `code-mutation-directive` is rejected.

**Logical-clock frame ordering.** Cross-Host frame ordering uses logical clocks (Lamport or hybrid logical clock — final pick by v0.5); wall-clock is metadata only. Cross-Host frame ordering is consistent under clock skew.

**Network partition behavior.** A2A in-flight frames during partition are NACKed after a configurable timeout (default 30s); the kernel does NOT auto-retry. The application layer (the Spirit) decides retry/escalate/halt.

**Certificate rotation.** Cross-reference: full mTLS rotation chaos test specification in §7.2.1 below.

### 7.2.1 mTLS Rotation Chaos Test (Quarterly, Mandatory)

Steady-state mTLS verification is necessary but insufficient. The failure mode worth testing is *forced rotation under live load* — the moment when issuing CA, agent SDS endpoints, and active connections must reconcile within bounded time without dropping data-plane traffic.

**Test schedule.** Quarterly, on calendar (not opportunistic). Production-equivalent staging environment. Synthetic load at p95 of trailing-30-day production traffic.

#### 7.2.1.a Pre-staged-overlap rotation procedure

**Variable definitions.** `p99_handshake_rtt` = trailing 30-day p99 of TLS 1.3 handshake duration (ClientHello → Finished, measured at the initiator) for IAC service-to-service connections, computed in steady state (excluding any active rotation drill window). Source metric: `iac_handshake_duration_us` (histogram, see §4.7.1). Recomputed daily; cached value used for the duration of any single rotation drill. If <30 days of data exist (cold deployment), use the maximum observed handshake duration over available history, floored at 500 ms. Then: `T_grace = max(2 × p99_handshake_rtt, 5 s)`.

**Procedure.** Agents MUST be provisioned with the replacement cert at least `T_grace` before the old cert's revocation timestamp. During `[t_provision, t_revoke + T_grace]`, agents accept either cert on inbound handshakes; clients prefer the new cert on new connections. Handshake failures with `BAD_CERTIFICATE` or `CERTIFICATE_EXPIRED` MUST trigger client retry: 3 attempts at 100 ms / 300 ms / 1000 ms backoff (jittered ±20%), retrying handshake (NOT request) only. After `t_revoke + T_grace`, old cert is hard-revoked; subsequent handshake attempts with old cert MUST fail-closed (rejection logged as `cert_post_grace_reject`, NOT counted as data-plane error). Gate: `cert_post_grace_reject` count ≤ 0.1% of total handshakes during the rotation window.

**Backoff derivation.** Schedule is 100 ms / 300 ms / 1000 ms across 3 attempts (4 total tries including the original). Derivation: base interval = ⌈p50(handshake_rtt)⌉ rounded to nearest 100 ms (target ~100 ms in steady-state IAC handshake measurements); growth factor = 3× (vs. envoy/istio default 2×) chosen because IAC handshake failures cluster on transient peer-state-sync issues that resolve on the order of seconds, not hundreds of milliseconds — wider spacing converges faster than more retries at tighter intervals; cap at 3 attempts because pre-deployment expectation: ≥ 99% of recoverable failures resolve within attempt 3, with attempt 4+ contributing < 0.5% additional success at 4× the latency cost. Schedule jittered ±20% to prevent thundering-herd on shared peer recovery. *(Note: empirical floors will be re-validated against measured rotation-drill telemetry in v0.7; if measured p50 differs materially from ~100 ms, the schedule re-derives from that measurement.)*

This pre-staged-overlap mechanism resolves the apparent contradiction between "zero data-plane errors" and "no fail-open on cert mismatch": errors during the grace window are absorbed by client retry; errors after the grace window are isolated, logged separately, and bounded; fail-open never occurs because both certs are validly trusted during overlap.

#### 7.2.1.b Cert rotation timing gates

During the scheduled rotation drill, instrument three timestamps per agent:
- `t_0` — `revoke()` API call returns success at CA
- `t_1` — agent's OCSP/CRL check first returns `revoked` for old cert
- `t_2` — agent completes successful TLS handshake with replacement cert AND first data-plane request succeeds

Compute and gate three distributions across the agent fleet:

| Metric | Definition | Floor (p50) | Floor (p99) | Owner |
|---|---|---|---|---|
| Revocation propagation latency | `t_1 − t_0` | ≤ 30 s | ≤ 90 s | PKI |
| Re-handshake latency | `t_2 − t_1` | ≤ 30 s | ≤ 60 s | Platform |
| End-to-end rotation latency | `t_2 − t_0` | ≤ 60 s | ≤ 150 s | Joint |

A drill PASSES only if all three rows pass at both p50 and p99. Per-row failure routes to the owning team. Additionally: `cert_post_grace_reject` rate ≤ 0.1% (per §7.2.1.a above).

**Failure response.** Any breach of any floor is a release-blocking issue at v0.7+. v0.5 reports all three metrics without enforcement (calibration phase). v0.7 enforces revocation propagation and re-handshake latency floors. v1.0 enforces all four including the `cert_post_grace_reject` ≤0.1% rate.

**Why "zero data-plane errors" is achievable, not aspirational.** The pre-staged-overlap procedure (§7.2.1.a) absorbs all transient cert-mismatch errors into client-side retry — invisible to the application. Post-grace `cert_post_grace_reject` events are intentional rejections of straggler clients with stale certs, logged separately and not counted as data-plane errors. The "zero" floor refers to client-visible request failures, not handshake retries.

## 7.3 Transparency Log

Per-Host SQLite append-only log. Every IAC frame, every capability invocation, every lifecycle transition lands in the log before delivery (I2). Default retention: 90 days private tier, configurable per-deployment; Merkle-root anchoring optional for tamper-evidence in regulated deployments.

**Audit query surfaces.** Four complementary primitives:

| Surface | Stakeholder | Primitive |
|---|---|---|
| `audit query` | Internal auditor / SRE | Frame-by-frame log query with replay (covered by `log.recall`) |
| `audit subject-access` | DPO / data subject | Subject-indexed query — "show me everything about data subject X across all Spirits and Hosts." Indexes on PII tags in IAC frames; respects redaction policy |
| `audit posture-delta` | CISO / security operations | Posture-drift query — "what capability scopes / sandbox tiers / consent policies have changed across Spirits in the last 30 days, and what was the approval chain" |
| `audit sealed-export` | External auditor | Cryptographically sealed audit bundle — Ed25519-signed by the operator's audit key, third-party-verifiable; not raw log. Includes Merkle anchoring if enabled |

**Right-to-be-forgotten (GDPR Article 17).** Per-Spirit private memory is removable on operator command (`maos forget --principal <id>`). The Transparency Log is not removable (it is the audit spine), but personally identifying payloads in the log can be redacted via `maos audit redact --frame <id> --reason <legal-hold>`; redactions are themselves logged. Cross-Spirit cascade: forgetting cascades to working-memory references in other Spirits where principal data was shared; distillates containing principal data are marked redacted with re-distillation triggered. Floor: 50/50 clean removal at queryable surface; 50/50 redaction-marker present in immutable log; 0 leakage in 100 follow-up subject-access queries.

**Replay determinism.** Determinism is over the **shape of the trace** — IAC frame ordering, capability-token issuances, halt events, decision-frame emission — NOT over redacted payload content. Redacted slots replay as `<REDACTED:type=<class>, len=<bytes>, hash=<sha256-prefix>>` placeholders carrying the same structural shape. The trace-shape contract is specified in `schemas/trace-shape.schema.json` (JSON Schema draft-2020-12); the schema is validated in CI. v1.0 is best-effort; v1.5 is the hard target.

## 7.4 Notification UX (kernel-rendered)

Three notification levels — `immediate`, `queue`, `digest`. **These are kernel-rendered, not Spirit-rendered.** A Spirit cannot bypass the user's notification policy by emitting a different kind of event; the kernel intercepts every IAC frame whose recipient is the human and routes it through the configured notification surface.

Surfaces: TUI, editor (ACP), browser, mobile push (HTTP push at v1.0; native push at v1.5+).

**Approval Decision Log distinct from Transparency Log.** Full intent + decision + reasoning chain per Invariant I4. Both logs are queryable via the control-plane API; both can be exported for compliance.

## 7.5 ACP / MCP

**ACP (Agent Communication Protocol).** NDJSON over stdio for editor-hosted Spirits. v1.0 ships with Zed + VSCode tested. JetBrains via plugin-bridge at v1.5.

**MCP (Model Context Protocol).** All-three-transports MCP client (stdio / SSE / Streamable HTTP). Streamable HTTP is the default for Loom-lite, the Spirit registry, and most production tool servers. Tool-side WASM sandboxing for untrusted MCP tools is not in this version; trusted MCP tools run at their declared sandbox tier.

**Four-protocol commitment.** Kernel-internal IAC + bilateral A2A + ACP + MCP. The substrate invents no new wire protocols. A fifth protocol requires (a) a use case unsatisfiable by IAC + adapter, (b) a new ADR, (c) demonstration that adding the protocol does not violate kernel-stays-small.

### 7.3.1 Log recall surface — Story 4.4 wiring

Story 4.4 introduces `log.recall(filter, limit, cursor)` and `log.fetch(frame_id)` as participant-scoped read primitives over the Transparency Log. `LogRecallAdapter` implements `LogRecallPort` with emitter-side scoping at v0.3-β (`WHERE spirit_pid = ?1`, covered by `idx_tlog_spirit_pid`). Cursor pagination uses keyset-comparison `(timestamp_ns, frame_id) > (?cursor_ts, ?cursor_id)` pushed to SQL with `LIMIT limit+1` to detect `next_cursor`. `MAX_LIMIT = 1024` is silently clamped at the adapter. `fetch` performs a primary-key lookup (`WHERE frame_id = ?1`) and validates emitter-scope before returning payload; cross-Spirit fetches return `ScopeViolation`. Every `recall` / `fetch` emits a `FrameKind::CapabilityInvocation` audit row (FR4 mediation). A2A consent envelope honoring is structurally a no-op at v0.3-β (`ConsentEnvelope == None` on every TL row) with a scaffold-comment block documenting the v0.5 binding contract from §7.1 + Story 6.3. Recipient-side participant-scoping and the `transparency_log_recipients` companion table are deferred to v0.5+ (Story 8.2 or 9.1).


### 7.3.2 Cross-Spirit IAC frame intent-lineage — Story 4.5 wiring

Story 4.5 extends `IacFrame` with an `intent_lineage: IntentLineage` field (additive,
`#[serde(default)]`, backward-compatible). The field is auto-populated by
`IacBusAdapter::deliver_typed` for `FrameOrigin::HumanAuthored` cross-Spirit frames
(single-class lineage derived from `frame.intent`). Spirit-emitted cross-Spirit frames
with empty lineage are rejected with `EIntentLineageBroken`, closing the
consent-laundering-across-re-emission vector.

Same-Spirit frames and broadcast frames (empty `to`) bypass the check per ADR-018
("exploded header overhead for frames that never cross consent boundaries").
The complementary I13 distillate-side lineage (`DistillationReceipt::intent_lineage`)
lives on a separate type and closes a separate attack vector; the two lineages do
not collide.

v0.3-β enforcement: additive only; existing test fixtures deserialize via serde-default.
v0.5+ Story 6.3 wires A2A consent-envelope runtime integration, at which point the
lineage check and the envelope check compose (lineage first, envelope second).
