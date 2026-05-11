# Epic 3: Director's Surface — IAC Bus, Task Assignment & Posture Control (v0.3 → v0.8)

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

### Stories

## Story 3.1: Route `task.assign` Frames Over the IAC Bus with Notification Surface Dispatch

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

## Story 3.2: Manage Director Posture with a Halt-Policy Schema and Bounded Shift Propagation

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

## Story 3.3: Director's Halt Resolution UX + Decision Audit (I12)

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

## Story 3.4: Buffer Orchestrator Instructions and Honor Director Pause/Resume/Revoke (P99 ≤2s)

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
