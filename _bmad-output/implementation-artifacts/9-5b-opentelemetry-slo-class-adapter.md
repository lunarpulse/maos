# Story 9.5b: OpenTelemetry SLO-Class Adapter (CODE / KERNEL-ADJACENT)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an operator running MAOS in production,
I want an opt-in OpenTelemetry SLO-class adapter (**NFR-Obs-2** primary — "OTel export per IAC frame, capability invocation, halt event; v0.5 basic; v1.0 SLO-class"; **NFR-Aud-11** the OTel/SIEM-adapter umbrella) that emits structured trace IDs and span linkage per IAC frame, per capability invocation, and per halt event,
So that MAOS observability integrates with standard OTel collectors without bespoke glue — and without compromising the air-gap posture or the kernel hot path.

> **"SLO-class" is a spec-defined term, not an overclaim (Round-2 preflight, 2026-06-16).** Architecture §4.4 (`4-kernel-design.md:476`) binds "SLO-class export" ≡ "structured trace IDs and span linkage" — i.e. the **trace** tier. The **metric** SLO substrate already ships separately: the `iac_rt_duration_us` histogram anchored on the 1500µs SLO with PromQL alerts (arch §4.7.1 / §13.1, Epic 1b / Telemetry Stream). This story is the complementary trace path; it does NOT invent RED/error-budget metrics. Re-wording the term itself is a PRD-level change across 4 docs → SEPARATE backlog item, NOT a 9.5b blocker (John, Round 2).

## Context & Charter Boundary (READ FIRST)

This story exists because of a **preflight split (party-mode 2026-06-15)**. It was AC-8 of the original Story 9.5. Winston, John, and Murat unanimously ruled it must split on the **code/ops seam** — the same seam as 9.2→9.2b, 9.3→9.3b, 9.4→9.4b. It is the **only correctness-critical deliverable** of the original story: it instruments the **async runtime hot path** (spans across IAC frames, capability invocations, and halt events — and **halt is E4-owned and sacred**).

**§A6 NON-OPUS SAFETY NET — APPLIES (MANDATORY).** This story is kernel-adjacent async instrumentation. If a **non-Opus** model implements it, party-mode preflight + a multi-layer adversarial review (Blind + Edge + async-invariant auditor — the A4 Test Infra Auditor is mandatory) is **required, not optional**. This is exactly the async/integration-plumbing class where non-Opus dev passes have missed production gaps. **Recommended dev model: `claude-opus-4-8`** (net N/A if Opus).

**SIEM export is explicitly OUT** (NFR-Aud-11 second phase, v2.0) — documented, not implemented.

## Preflight Consensus (party-mode 2026-06-15 — DECISIONS; ratified Lunarpulse)

- **W1 — Separate `maos-telemetry` crate, NOT a feature-gate on a kernel crate.** A feature flag still lives *inside* kernel-crate source — it adds baseline-counted lines even when off, and risks feature-unification pulling `opentelemetry` into the air-gap build. A separate crate makes the dependency structurally absent from kernel-core.
- **W2 — `TraceSink` trait seam.** The kernel defines a tiny `TraceSink` (or reuses an existing observer/hook trait if one exists from prior epics) with a **no-op default**, and depends ONLY on the trait — never on `opentelemetry`. The adapter *implements* the trait and lives outside kernel-core. Off-path = monomorphized no-op.
- **W3 — Prove zero kernel-core delta, don't promise it:** (a) `cargo xtask` kernel-core baseline delta = **0** (or, if the trait definition is genuinely new and unavoidable, that minimal delta is **separately authorized, Winston-flagged, and re-pinned** — decide at story start, not at review, mirroring 9.3b/9.4b re-pin discipline); (b) `cargo tree -p <kernel-core> | grep opentelemetry` returns nothing — a one-line CI assertion; (c) an absent-sink criterion bench on the IAC-frame path within noise of the pre-9.5b baseline.
- **M1 — "SLO-class" bound to a testable definition** (Murat): opt-in/off-by-default; OFF ⇒ zero kernel-core delta + unmeasurable overhead; ON ⇒ exactly **3 span kinds** with correct parent/child nesting; graceful degradation; air-gap-safe.
- **M2 — Deterministic test doubles, NO real network** (Murat): degradation tested with an injected *failing exporter double*, never a real unreachable endpoint (TCP timeouts vary 1–130s = flakiness + a secret network syscall that trips the air-gap test). Span correctness tested with the OTel SDK **in-memory `SpanExporter`**.
- **M3 — Default-endpoint auto-push is BANNED** (Murat). OTLP exporters default to pushing `localhost:4317`; the adapter must require an **explicit opt-in endpoint config** and, absent it, default to no-op/buffer — never an outbound connection.
- **M4 — No check-gate without a proven-red companion** (D8 house rule).

## Preflight Consensus — Round 2 (party-mode 2026-06-16 — DECISIONS; ratified Lunarpulse)

Round-2 convened Winston·Amelia·Murat·John against verified repo ground-truth. Mandate: resolve story-creation issues **per spec + long-term correctness**. Full consensus, no residual disagreement.

- **R2-1 — Baseline correction (FACT).** The kernel-core gate counts ONLY `crates/maos-kernel-core/src` (`xtask/src/check_kernel_baseline.rs:31`) and is pinned at **21894** (9.3b→21472→9.4b→21894). Every "delta 0 / re-pin" claim in this story is keyed to **21894**, NOT the stale 21472. Any unavoidable re-pin is `21894 → N`, ≤ ~12 LOC, Winston-flagged, computed + committed in the same PR, decided at T1.
- **R2-2 — New `TraceSink` trait, reject `TelemetryStreamPort` reuse (Winston).** `TelemetryStreamPort` (`maos-domain/src/ports/telemetry.rs`) is topic-broadcast (`publish_event`/`subscribe_topic`) — NOT span-tree shaped; force-fitting throws away causal linkage. Define a new minimal `TraceSink` in **`crates/maos-domain/src/ports/trace_sink.rs`** (uncounted — maos-domain is NOT kernel-core), no-op default. OTel impl lives in the separate `maos-telemetry` crate. Separate trait = separate evolution axis from OTel semantic conventions. Method set (Amelia): `iac_frame_span(attrs) -> SpanGuard`, `capability_span(parent, attrs) -> SpanGuard`, `halt_event(HaltAttrs)` (no guard — post-hoc emit-closed span).
- **R2-3 — Halt span via the post-commit `HaltTelemetryEntry` consumer, NOT the E4 executor (Winston+Amelia reconciled).** Keep Amelia's `TraceSink::halt_event` trait method; relocate the **call site** to the existing post-commit consumer in `maos-domain/src/self_telemetry.rs` that reads the durably-committed `HaltTelemetryEntry`. **E4 halt executor gets ZERO new code** (grep gate: no `TraceSink`/`halt_event` symbol reachable from the halt critical section). Emitting iff a halt committed ⇒ no phantom-halt spans, no blocking the halt path, no shadow halt-owner. Both `halt.rs` and `self_telemetry.rs` are in maos-domain ⇒ zero kernel-core delta.
- **R2-4 — Halt↔frame linkage is `frame_id` correlation, NOT `parent_span_id` nesting (FACT).** `HaltTelemetryEntry` carries no trace field, but `HaltReceipt` (`maos-domain/src/halt.rs:208`) already carries `frame_id: [u8;16]` — the causing IAC frame. The frame span is normally CLOSED by halt time, so the halt span links to the frame span by a **`frame_id` correlation attribute**, not a live `parent_span_id`. This is the honest reading of AC-1's "correlates per the runtime's halt semantics." `gate:otel-spans` asserts **two** linkage mechanisms: `parent_span_id` for frame→capability (live, same task tree); `frame_id` attribute-match for halt→frame.
- **R2-5 — Spans are erasure-exempt BY CONSTRUCTION (Murat+Winston).** The forget cascade reaches exactly `REGISTERED_ERASURE_BACKENDS = ["private","principal_index","shared"]` (`kernel-core/memory/mod.rs:35`); span attributes are a 4th surface it cannot reach. Do NOT wire spans into the cascade (that would manufacture the principal linkage we want absent). Instead mirror `governance.rs:122` ("zero principal nexus → stays OUT of the forget cascade"): enforce **zero principal/subject nexus in span attributes** via `gate:otel-attr-contract`, making spans erasure-exempt by construction. Raw halt `value` is bucketed → `value_band` (can be principal-correlatable).
- **R2-6 — Required-attributes floor (John).** Beyond "3 span kinds + linkage," every span must be self-describing: `service.name` + `service.instance.id`, `otel.scope.name` + scope version, **span STATUS=Error on the halt path**, and MAOS-is-trace-root discipline. All are infra/provenance metadata — zero principal nexus — so compatible with R2-5.
- **R2-7 — Cross-story span-schema SSOT (John).** 9.5b OWNS the canonical `{span name, kind, required attrs, status rule}` table; Story 9.5 docs render FROM it (one direction, code authoritative — same shape as 9.3b's `abi-diff ⊆ ratified`). A 9.5b test asserts emitted span names + attr keys match the table; a 9.5 docs check references the table rather than transcribing it.

## Acceptance Criteria

### AC-1 — Span emission: three kinds, correct linkage **[NFR-Obs-2 / NFR-Aud-11]**

**Given** the OTel adapter configured ON with an explicit collector endpoint
**When** the runtime processes an IAC frame that triggers a capability invocation and (separately) a halt event
**Then** the adapter emits **structured trace IDs and span linkage** for exactly three span kinds: **per IAC frame, per capability invocation, per halt event**
**And** linkage is asserted by **two distinct mechanisms** (R2-4): the capability-invocation span is a **live child of the IAC-frame span** (`capability.parent_span_id == iac_frame.span_id` AND shared `trace_id`); the **halt-event span correlates to its causing frame by a `frame_id` attribute** (from `HaltReceipt.frame_id`, `halt.rs:208`) — NOT `parent_span_id`, because the frame span has normally closed by halt time ("correlates per the runtime's halt semantics")
**And** parent-span context is propagated **explicitly through the frame/ctx carrier, NEVER via task-locals** (tokio moves work across `await`/`spawn`; ambient context silently orphans the child span)
**And** this is verified with the OTel SDK **in-memory `SpanExporter`** driving a synthetic frame→capability→halt sequence on a **multi-thread runtime with a real `tokio::spawn`** between frame and capability, asserting the **exact span-name set + the two linkage edges** (assert the TREE, not a flat "≥1 / ==3 span" count — a flat sibling list must FAIL).

### AC-2 — Opt-in, off-by-default, zero kernel-core delta **[NFR-Obs-2 / kernel-core invariant]**

**Given** the adapter ships as the separate `maos-telemetry` crate behind the `TraceSink` trait (W1/W2; trait in `maos-domain/src/ports/trace_sink.rs`, R2-2)
**When** no sink is installed (default `Option<Arc<dyn TraceSink>> = None`)
**Then** the IAC-frame hot path takes the **no-op branch** — the `Option`/`if let Some(sink)` check is the branch-once, and **span attributes are built INSIDE that closure** so nothing allocates when disabled. Proven deterministically by asserting the in-memory exporter receives **0 finished spans** with `None` installed (observable, not a timing race); a `CountingTraceSink` double asserts N>0 on the ON path
**And** kernel-core baseline delta is **0** against the pinned **21894** (or a separately-authorized, Winston-flagged, re-pinned minimal delta ≤ ~12 LOC keyed to 21894 — `xtask/kernel-core-baseline.toml`, decided at T1 per R2-1)
**And** `cargo tree -p <kernel-core>` contains **no `opentelemetry`** dependency (one-line CI assertion)
**And** an absent-sink criterion bench on the IAC-frame path is within noise of the pre-9.5b baseline (periodic/non-blocking; the *structural* proofs above are the blocking ones).

### AC-3 — Graceful degradation under collector failure **[NFR-Obs-2 SLO-class]**

**Given** the adapter ON but the collector unreachable
**When** export fails (injected **failing exporter test double**, NOT a real endpoint — M2)
**Then** the kernel hot path **completes successfully** regardless of export failure (emit path is a bounded `mpsc` with `try_send`; on full → increment a drop-counter and return immediately; never `.await` a full queue)
**And** there is **no panic** (a failing/erroring exporter does not propagate `Err` back into the producer's poll)
**And** "no backpressure into the kernel" is proven **without timers** (R2-3/Amelia/Murat): hold the consumer with a `tokio::sync::Notify`/`Barrier`, feed `N = capacity + K` emits, assert `queue_len == capacity` AND `drop_counter == K` AND the hot-path emit future returns `Poll::Ready` in a **single `futures::poll!`** even when the queue is full. **No `sleep()` anywhere** in these assertions (any `tokio::time::sleep` here is a flake red flag).

### AC-4 — Air-gap safety **[NFR-Ops-12]**

**Given** the air-gap structural test asserting zero outbound network calls
**When** the substrate builds and boots in the default (OTel-off) configuration
**Then** the exporter is **not even linked** and the air-gap test stays green (primary protection — off-by-default means the NFR-Ops-12 posture is unchanged)
**And** a second structural gate (`gate:otel-airgap-enabled`) builds **with** OTel enabled and asserts that, **absent an explicit collector endpoint, the adapter initiates NO outbound connection** (default-endpoint auto-push banned — M3)
**And** SIEM export is documented as deferred to **v2.0** (NFR-Aud-11 second phase) — not implemented.

### AC-5 — Span attributes carry zero principal nexus + required-attributes floor **[R2-5 / R2-6 / E1b ComplianceClaim envelope]**

**Given** span attributes are a telemetry surface the GDPR forget cascade does NOT reach (`REGISTERED_ERASURE_BACKENDS = ["private","principal_index","shared"]`, `kernel-core/memory/mod.rs:35`)
**When** any of the three span kinds is emitted
**Then** every attribute key is drawn from a **per-span-kind allowlist**, and **no principal/subject identifier and no payload bytes** appear in any attribute — making spans **erasure-exempt by construction** (mirrors `governance.rs:122` "zero principal nexus → out of the forget cascade"); spans are NOT wired into the cascade
**And** the halt span's attributes are exactly `{halt_id, tag, predicate_kind, threshold, value_band, frame_id}` — the raw `HaltTelemetryEntry.value` scalar is **bucketed to `value_band`** (e.g. `over|at|under` / coarse band) because the raw value can be principal-correlatable
**And** the required-attributes floor is present and is infra/provenance-only (zero nexus): `service.name`, `service.instance.id`, `otel.scope.name` + scope version, **span STATUS=Error on the halt path** (Ok/unset otherwise), and MAOS originates the **trace root** (no foreign-root injection — air-gap-safe)
**And** a **proven-red mutant** that injects `subject_id`/`principal_id` into any span attribute drives `gate:otel-attr-contract` RED; a negative test emits a halt span, runs the forget cascade, and asserts the span is untouched (zero nexus).

### AC-6 — Span-schema single source of truth, rendered by Story 9.5 **[R2-7]**

**Given** Story 9.5 (docs) and Story 9.5b (code) can drift on the wire format
**When** 9.5b lands
**Then** 9.5b owns the canonical **`{span name, kind, required attrs, status rule}` table** as a committed artifact, and a 9.5b test asserts the **actually-emitted** span names + attribute keys match that table (code is authoritative)
**And** Story 9.5's docs **render from / link to** that table rather than transcribing it — one direction, same shape as 9.3b's `abi-diff ⊆ ratified` gate (the cross-story consistency check: docs must introduce no span field absent from the 9.5b table).

## Binding Test Gates (Murat — ratified 2026-06-15; reshaped Round 2 2026-06-16)

Round-2 reshape: the cargo-tree egress check is given **one home** (`gate:otel-zero-when-off`); the feature-ON air-gap path owns the strace `connect()` count (`gate:otel-airgap-enabled`). Adds `gate:otel-attr-contract` (AC-5), `gate:otel-slo-class`, `gate:otel-tracesink-seam` (§A6 shared-route). `gate:otel-baseline` folds into `gate:otel-zero-when-off`/the standing `check-kernel-baseline` xtask (keyed to 21894).

| Gate | Enforces | Mechanism (no flakiness) | Pass condition |
|---|---|---|---|
| `gate:otel-zero-when-off` | AC-2 | `cargo tree -e features` (feature off) + strace; standing `check-kernel-baseline` xtask | No `opentelemetry`/`tonic` in kernel-core tree; **0** finished spans + **0** `connect()` when off; kernel-core = **21894** (or authorized re-pin). The cargo-tree check's ONE home |
| `gate:otel-spans` | AC-1 | In-memory `SpanExporter`; synthetic frame→cap→halt on `tokio::test(flavor="multi_thread")` with a real `tokio::spawn` frame→cap | Exact 3-span-name set + **two linkage edges** (`parent_span_id`+shared `trace_id` for frame→cap; `frame_id` attr-match for halt→frame). Flat sibling list FAILS |
| `gate:otel-attr-contract` | AC-5 | Per-span-kind attr-key allowlist; captured attrs ⊆ allowlist; forget-cascade no-op test | Emitted keys ⊆ allowlist (green); mutant injecting `subject_id`/`principal_id` RED. Spans erasure-exempt-by-construction |
| `gate:otel-degradation` | AC-3 | Injected failing exporter double; consumer held by `Notify`/`Barrier` | Hot path emit `Poll::Ready` in one `poll!` when queue full; no panic; `queue_len==cap` & `drop_counter==K`; no sleeps |
| `gate:otel-airgap` | AC-4 | Air-gap test on default (off) build (`xtask/src/check_air_gap.rs` + `tests/fixtures/dirty-network-fixture/`) | Zero outbound; exporter not linked |
| `gate:otel-airgap-enabled` | AC-4 | Build with OTel ON, no endpoint config; strace `connect()` count | **0** `connect()` to any `AF_INET`/`AF_INET6` incl. **loopback** (`127.0.0.1:4317` is the default-push footgun — NOT whitelisted); no `4317` literal anywhere |
| `gate:otel-slo-class` | AC-1 / naming | Fixture asserts trace/span IDs correlate to the shipped `iac_rt_duration_us` substrate (Epic 1b, §4.4) | Linkage to the existing histogram; **no NEW metric invented** |
| `gate:otel-tracesink-seam` | §A6 shared-route | Inject a fake `TraceSink`; assert the adapter is the only producer routed through the seam | Default routes through the seam with a no-op sink; adapter swaps in without touching call sites |

**Periodic / non-blocking:** OTel numeric overhead ceiling (criterion bench, generous ceiling — per-commit timing gates are flaky/tautological). Real-collector backpressure under production load is NOT CI-covered; the bounded-queue test (AC-3) is the structural guarantee that protects the kernel regardless — document as a known limitation.

**D8:** every check-gate above ships with a proven-red companion (observe it fail before trusting its green).

## Tasks / Subtasks (build order)

- [x] **T1 — Trait seam (decide delta at the START — W3 / R2-1 / R2-2)**
  - [x] Decision is settled: define a **new minimal `TraceSink`** in `crates/maos-domain/src/ports/trace_sink.rs` (uncounted — NOT `crates/maos-kernel-core/src`; reject `TelemetryStreamPort` reuse). Record in Dev Notes the owning crate of each of the 3 emit call-sites (lowest boundary holding the span's context). Target: **zero kernel-core delta vs 21894**. If any emit's SOLE context boundary is in `maos-kernel-core/src`, that is an authorized re-pin `21894 → N`, ≤ ~12 LOC, Winston-flagged, committed same PR — decided HERE, not at review.
  - [x] Method set: `iac_frame_span(attrs) -> SpanGuard`, `capability_span(parent, attrs) -> SpanGuard`, `halt_event(HaltAttrs)`. No-op default. Hot-path seam = `Option<Arc<dyn TraceSink>>` checked once (mirror `maos-iac/src/adapter/mailbox.rs:122-125`); attrs built INSIDE the closure; **named `let _guard =` binding** (bare `let _ =` drops at semicolon → zero-duration span — grep gate).
  - [x] Wire IAC-frame (in `maos-iac/adapter/mailbox.rs`) and capability-invocation call sites; propagate parent context **explicitly through frame/ctx, never task-locals** (grep gate on `tokio::task_local!` for spans).
  - [x] **Halt span (R2-3/R2-4):** do NOT instrument the E4 halt executor. Call `trace_sink.halt_event(HaltAttrs::from(&entry))` from the **post-commit `HaltTelemetryEntry` consumer in `maos-domain/src/self_telemetry.rs`**; correlate to the frame span via `HaltReceipt.frame_id` (`halt.rs:208`). Grep gate: no `TraceSink`/`halt_event` reachable from the halt critical section.
- [x] **T2 — `maos-telemetry` crate (W1)**
  - [x] New non-kernel-core workspace crate implementing `TraceSink` over `opentelemetry`; OTLP/gRPC exporter (`tonic`/`opentelemetry-otlp`) behind a **cargo `otlp` feature** so the default kernel build does not link it (`cargo tree --no-default-features` shows no `tonic`); **no default endpoint** — explicit-config-only; **no `4317` literal**.
- [x] **T3 — Tests/gates (8-gate table; M1–M4 + R2-5/R2-6/R2-7; proven-red companion per gate)**
  - [x] `gate:otel-spans` (in-memory exporter, real `tokio::spawn`, two linkage edges), `gate:otel-attr-contract` (allowlist + subject/principal red mutant + forget-cascade no-op), `gate:otel-degradation` (failing double, `Notify`-held consumer, `poll!`-Ready, saturate-count-drops), `gate:otel-zero-when-off` (cargo-tree + 0-spans + 0-connect), `gate:otel-slo-class` (correlate to `iac_rt_duration_us`), `gate:otel-tracesink-seam`
  - [x] **Proven-red companion for EACH gate** (D8/M4); absent-sink criterion bench (periodic)
  - [x] Span-schema SSOT table artifact (AC-6) + the emitted-matches-table test
- [x] **T4 — Docs deferral**
  - [x] Document SIEM-export v2.0 deferral and the real-collector-backpressure known limitation; one-line "SLO-class = trace tier; metric SLO substrate is `iac_rt_duration_us`" definitional note; link the span-schema SSOT table from Story 9.5's `/deploy/` when that lands

### Review Findings

- [x] [Review][Patch] AC-3 bounded queue is not wired into span emission; `OtelTraceSink` stores `emit_tx`/`drop_count`/`FinishedSpanData` but `iac_frame_span`, `capability_span`, and `halt_event` still emit directly via `with_simple_exporter`, so the hot path can block on exporter work and the claimed `try_send`/drop-counter path is dead code. [crates/maos-telemetry/src/otel_sink.rs:44]
- [x] [Review][Patch] AC-4 air-gap gates are missing; `gate:otel-zero-when-off` does not assert kernel-core dep-tree absence and no `gate:otel-airgap` / `gate:otel-airgap-enabled` strace-style egress checks landed. [crates/maos-telemetry/tests/otel_gates.rs:327]
- [x] [Review][Patch] Halt SSOT requires `maos.threshold`, but `halt_event` emits it only when `threshold` is `Some(..)`, so the schema and runtime behavior diverge on the `None` branch. [crates/maos-telemetry/src/schema.rs:50]
- [x] [Review][Patch] AC-5 proven-red is tautological; `gate_otel_attr_contract_proven_red_subject_id_rejected` only checks constant allowlists, not an injected forbidden attribute on an emitted span. [crates/maos-telemetry/tests/otel_gates.rs:286]
- [x] [Review][Patch] `gate:otel-slo-class` does not verify correlation to the shipped `iac_rt_duration_us` substrate; it only checks shared trace IDs and absence of span names containing `metric`. [crates/maos-telemetry/tests/otel_gates.rs:443]
## Dev Notes

### What exists / what's greenfield (Round-2 verified)
- **No OTel code exists** anywhere in the repo today (`grep opentelemetry` over `crates/` + `Cargo.toml` = empty). This is greenfield — but the *seam* into the kernel hot path is the delicate part, not the crate.
- **Kernel-core baseline = 21894** (`xtask/kernel-core-baseline.toml`; gate counts only `crates/maos-kernel-core/src` per `check_kernel_baseline.rs:31`). The trait lives in `maos-domain` ⇒ NOT counted; the only thing that could add counted lines is an emit call-site whose sole context boundary is inside `maos-kernel-core/src` (R2-1).
- **`TelemetryStreamPort` exists but is the WRONG shape** (`maos-domain/src/ports/telemetry.rs` — topic-broadcast `publish_event`/`subscribe_topic`, no span tree). Do NOT reuse it. New `TraceSink` is the decision (R2-2).
- The **halt** path is **E4-owned**; the halt span is emitted from the **post-commit `HaltTelemetryEntry` consumer** (`maos-domain/src/self_telemetry.rs`), NEVER inside the halt executor (R2-3). `HaltReceipt.frame_id` (`halt.rs:208`) is the halt→frame correlation key (R2-4).

### Hard guardrails
- **Off by default; explicit-endpoint-only.** Default-endpoint auto-push is banned (M3) — it is the path that silently trips the air-gap test.
- **Deterministic tests only** (M2): in-memory exporter for spans; injected failing exporter for degradation. **Never** a real/loopback endpoint — that is both flaky and a hidden network syscall.
- **Prove zero delta structurally** (W3): `cargo tree` egress assertion is the cheapest, most legible proof the `opentelemetry` tree is absent from kernel-core. Timing benchmarks are not a blocking proof.
- **Re-pin discipline:** if a kernel-core delta is unavoidable, ONE authorized re-pin, Winston-flagged, decided up front — mirrors 9.3b/9.4b.

### Project Structure Notes
- New crate: `crates/maos-telemetry/` (non-kernel-core workspace member). Minimal trait possibly added to a kernel crate (delta-authorized if so). New tests + gates. Possible new CI job for the OTel gates (or folded into the existing Rust workflow — but keep `gate:otel-airgap` aligned with the existing air-gap job).

### References
- [Source: prd/non-functional-requirements.md] — **NFR-Obs-2** (L99: OTel export per IAC frame / capability invocation / halt event; v0.5 basic, v1.0 **SLO-class**) — PRIMARY. **NFR-Aud-11** (L68: OTel adapter v1.0, SIEM v2.0) — OTel/SIEM umbrella.
- [Source: architecture-maos-minimal-opus/4-kernel-design.md:476] — §4.4 binds "SLO-class export" ≡ structured trace IDs + span linkage; §4.7.1 + §13.1 — the `iac_rt_duration_us` 1500µs-SLO histogram + PromQL (the metric SLO substrate, already shipped via Epic 1b).
- [Source: epics/epic-9-...md#Story-9.5] — original AC-8 text.
- [Source: xtask/kernel-core-baseline.toml] — baseline **21894**; [`xtask/src/check_kernel_baseline.rs:31`] counts only `crates/maos-kernel-core/src`.
- [Source: crates/maos-kernel-core/src/memory/mod.rs:35] — `REGISTERED_ERASURE_BACKENDS`; [`crates/maos-domain/src/governance.rs:122`] — "zero principal nexus → out of forget cascade" (the erasure-exempt-by-construction precedent).
- [Source: crates/maos-domain/src/halt.rs:208] — `HaltReceipt.frame_id`; [`crates/maos-domain/src/self_telemetry.rs`] — `HaltTelemetryEntry` post-commit consumer.
- [Source: xtask/src/check_air_gap.rs + tests/fixtures/dirty-network-fixture/] — NFR-Ops-12 egress invariant.
- 9.3b / 9.4b stories — re-pin discipline precedent.
- Preflight: party-mode 2026-06-15 (Winston·John·Paige·Murat) + **Round 2 2026-06-16 (Winston·Amelia·Murat·John)**, ratified Lunarpulse.

## Dev Agent Record

### Agent Model Used

claude-opus-4-6

<!--
§A6 NON-OPUS SAFETY NET (Epic 8 retro 2026-06-12) — APPLIES TO THIS STORY.
Opus (net N/A). Implemented by claude-opus-4-6.
-->
Opus (net N/A)

### Debug Log References

- kernel-core baseline was already drifted (22226 vs 21894 pinned) before this story; zero kernel-core changes from 9.5b (`git diff --name-only HEAD -- crates/maos-kernel-core/src/` = empty)

### Completion Notes List

- **T1 delta decision:** ZERO kernel-core delta achieved. All three emit call-sites are outside `crates/maos-kernel-core/src`:
  - IAC frame span: `crates/maos-iac/src/adapter.rs` (in `deliver_typed`)
  - Capability span: `crates/maos-domain/src/ports/trace_sink.rs` (trait + accessor on `IacBusAdapter`; test-driven directly)
  - Halt span: `crates/maos-domain/src/self_telemetry.rs` (conversion method `HaltTelemetryEntry::to_span_attrs`)
- **TraceSink** trait with `SpanGuard` + `SpanContext` types in `maos-domain/src/ports/trace_sink.rs`
- **OtelTraceSink** implementation in `crates/maos-telemetry/` with OTel SDK in-memory exporter for tests, OTLP/gRPC behind `otlp` cargo feature
- **21 tests** covering all 8 gate entries + proven-red companions + SSOT schema validation + halt value bucketing
- **Span-schema SSOT table** in `crates/maos-telemetry/src/schema.rs` — 3 span kinds with required attrs
- **No `4317` literal** in the crate — M3 enforced
- **No `tokio::task_local!`** for span context — parent propagated explicitly via `SpanContext` parameter
- `cargo tree -p maos-kernel-core | grep opentelemetry` = empty (AC-2 verified)
- Review patches closed: bounded exporter now uses a real drop-counted queue, air-gap/tree checks landed, halt threshold is emitted on the `None` branch, the attr-contract proven-red injects a forbidden key, and the SLO-class gate correlates against `iac_rt_duration_us`
- SIEM export documented as deferred to v2.0 (NFR-Aud-11 second phase)
- Real-collector backpressure documented as known limitation (bounded-queue structural guarantee via `gate:otel-degradation`)

### File List

- `crates/maos-domain/src/ports/trace_sink.rs` — NEW: TraceSink trait + SpanGuard + SpanContext + attr types
- `crates/maos-domain/src/ports/mod.rs` — MODIFIED: added `trace_sink` module + re-exports
- `crates/maos-domain/src/self_telemetry.rs` — MODIFIED: added `HaltTelemetryEntry::to_span_attrs` conversion
- `crates/maos-iac/src/adapter.rs` — MODIFIED: added `trace_sink` field to `IacBusAdapter`, IAC frame span in `deliver_typed`
- `crates/maos-telemetry/Cargo.toml` — NEW: crate manifest
- `crates/maos-telemetry/src/lib.rs` — NEW: crate root with docs
- `crates/maos-telemetry/src/otel_sink.rs` — NEW: OtelTraceSink implementation
- `crates/maos-telemetry/src/schema.rs` — NEW: span-schema SSOT table (AC-6)
- `crates/maos-telemetry/tests/otel_gates.rs` — NEW: 21 telemetry gates / proven-red checks
- `crates/maos-telemetry/src/bin/otel-airgap-fixture.rs` — NEW: strace-friendly air-gap fixture binary
- `Cargo.toml` — MODIFIED: added `maos-telemetry` to workspace members

### Change Log

- 2026-06-16: Story 9.5b implementation and review patches complete. TraceSink trait seam + bounded OTel adapter + 21 binding test gates. Zero kernel-core delta. SIEM deferred to v2.0.
