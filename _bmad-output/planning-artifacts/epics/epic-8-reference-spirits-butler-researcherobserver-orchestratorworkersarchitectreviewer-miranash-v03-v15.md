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

## Story 8.6: Ship the Live Cross-Host A2A TCP/mTLS Transport (`maos-a2a-tcp`)

> **Split from Story 8.5 (2026-06-04); ACs hardened via architecture+test roundtable (2026-06-04).**
> Story creation found the live `A2AProfile::CrossHost` transport FULLY ABSENT (`LoopbackA2ARouter`
> is in-memory `mpsc` only; no socket code; the CrossHost enum is never dispatched; "v0.7 TCP
> connector"/"deferred" markers in `maos-a2a`). Story 8.5 ships the **loopback-simulated** bilateral
> pair (the cross-Host *protocol*: pre-paired fingerprints, TOFU, ADR-012 consent, rotation chaos);
> this story ships the live two-process *transport* — a distinct, **security-critical networking**
> risk class. Depends on Story 8.5 + Story 6.3.
>
> **Structural decision (LOCKED — Winston):** the seam is introduced by **extraction, not insertion**.
> A new `maos-a2a-core` crate is carved out to own `trait A2ATransport` + the transport-agnostic
> protocol substrate; `maos-a2a` keeps only `LoopbackA2ARouter` (re-exporting from core for
> backward-compat); `maos-a2a-tcp` depends ONLY on `maos-a2a-core`. This resolves the `maos-a2a`
> 1500-LOC ceiling overage (currently 2550) AND the in-memory-vs-wire seam in one move, and keeps the
> wire crate's dep graph free of the loopback router. **Workspace count therefore moves 39 → 41**
> (`maos-a2a-core` + `maos-a2a-tcp`). *(Corrected 2026-06-04 at story-prep: this note was authored when 8.6 was scoped to follow Story 8.4 at 37 members; Story 8.5 then merged Mira+Nash, taking the workspace to 39 — verified via `cargo metadata --no-deps`. The invariant is `+2`; pin the live count at dev time.)*
>
> **FORK RESOLVED (Option A — team consensus 2026-06-04: Winston + Murat + security red-team, unanimous):** `ca_roots`
> posture (AC-A5) ships **BOTH** modes. Default `Some(bundle)` = CA-chain-to-root **then** TOFU pin (defense-in-depth),
> which is the test/prod corpus default (and the only posture in which AC-T4's "untrusted-CA rejected *even if
> coincidentally pinned*" oracle is constructible). `None` = pin-only (the FR23a self-signed bilateral posture) is kept
> first-class and separately tested (AC-T4b), with the SAME ordered validity prelude (NOT a `danger_accept_any` noop) —
> only the trusted-root chain step is gated on `ca_roots.is_some()`. Zero ABI churn: the fork lives only in
> `maos-a2a-tcp`'s verifier construction; `verify_pinned` stays byte-identical (AC-A6-safe). Decided on spec-fidelity +
> long-term-correctness. Full rationale in the Story 8.6 spec Dev Notes (`ca_roots security-posture fork` section).

As a v1.5 operator running Mira and Nash on two separate Hosts,
I want the live `A2AProfile::CrossHost` transport — a real TCP listener/dialer with operator-managed
mTLS (custom TOFU-pinning cert verification), length-delimited JSON-RPC framing over the socket,
handshake retry, and real partition-timeout behavior — shipped as a NEW `maos-a2a-tcp` crate over a
freshly-extracted `maos-a2a-core` seam,
So that the bilateral pair coordinates over a genuine two-process network connection (not the
in-process loopback simulation Story 8.5 ships), realizing FR23b cross-Host at v1.5 with the TOFU
security model actually enforced on the wire.

### Architecture / Structure ACs (AC-A1–AC-A7 — Winston)

**AC-A1 — Extract `maos-a2a-core`; define the transport seam there; resolve the ceiling by extraction**
**Given** `maos-a2a` is at 2550 lines against its own 1500 ceiling and owns both protocol substrate and the in-memory router
**When** Story 8.6 lands
**Then** a NEW crate `maos-a2a-core` at `crates/maos-a2a-core/` owns the transport-agnostic surface: it declares `pub trait A2ATransport` bound to the real, frozen `A2APeerRouter` surface — `async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), A2AError>` + `async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse` (`adapter.rs:37-70,255,334`) + a `fn local_addr(&self) -> Option<SocketAddr>` readiness hook *(Corrected 2026-06-04 at story-prep: the original sketch `dispatch(&self, peer: PeerId, frame: A2AJsonRpcRequest) -> Result<A2ANack, TransportError>` named types `A2ANack`/`TransportError` that do not exist — the real types are `NackResponse`/`A2AError`; and there is no literal `match A2AProfile::CrossHost` arm — `A2AProfile::CrossHost` is never dispatched on, so the new trait binds to the profile-agnostic `route_outbound`/`handle_intake` instead. If a `dispatch`/`A2ANack`/`TransportError` shape is preferred, add them as NEW core types, do not diverge from `A2AError`/`NackResponse`)*; it **moves (not copies)** the shared substrate (`A2AJsonRpcRequest::try_from_bytes`, `handle_intake` + its types, `HandshakeRetryPolicy`, `RotationDrillReport`, TOFU `verify_pinned` + `InMemoryTofuPinStore`, `boot_nonce`, `LamportClock`/`logical_clock`, the `CODE_PARSE_ERROR` constructor) — ALL `pub` at the core root (the surface ABOVE the seam)
**And** `maos-a2a` retains ONLY `LoopbackA2ARouter` as `impl A2ATransport`, depends on `maos-a2a-core`, and `pub use`-re-exports the moved symbols so no downstream import path breaks
**And** after extraction `kloc-check` is GREEN for BOTH `maos-a2a` (now < 1500) and `maos-a2a-core` (ceiling set in the kloc manifest; record the post-extraction count in evidence), with NO ceiling bump to any existing crate

**AC-A2 — `maos-a2a-tcp` is the second `A2ATransport` impl; dependency arrow points only at core**
**Given** the seam from AC-A1
**When** the wire crate is created
**Then** a NEW crate `maos-a2a-tcp` at `crates/maos-a2a-tcp/` declares `pub struct TcpA2ATransport` with `impl A2ATransport`, and its `Cargo.toml` lists `maos-a2a-core` (NOT `maos-a2a`, NOT `maos-kernel-core`) + `tokio`, `tokio-rustls`, `rustls`, `tokio-util` (codec feature) as its only first-party/transport deps
**And** `maos-a2a-core` contains ZERO references to `TcpListener`/`TcpStream`/`tokio_util`/`tokio_rustls` (grep-asserted — interlocks with AC-T13)
**And** the workspace member count moves **39 → 41** exactly (`cargo metadata --no-deps` member-count assertion pinned to the live count at dev time; *corrected 2026-06-04 from "37 → 39" — pre-8.5-merge framing; the `+2` invariant is unchanged*), and `abi-diff` is **Added-only** (the AC-A1 `pub use` re-exports preserve symbol paths so nothing reads as Removed)

**AC-A3 — `TofuPinningVerifier`: named deliverable bridging WebPKI into `verify_pinned`**
**Given** stock `WebPkiServerVerifier`/`WebPkiClientVerifier` would terminate the cert decision without consulting the pin store
**When** a live mTLS connection is established by `TcpA2ATransport`
**Then** `maos-a2a-tcp` provides a NEW named type `TofuPinningVerifier` implementing `rustls::client::danger::ServerCertVerifier` (dialing side) AND a `rustls::server::danger::ClientCertVerifier` twin (listening side) — **both directions MUST pin**; each `verify_*_cert` FIRST performs WebPKI structural/chain validation (delegating to a wrapped stock verifier) and ONLY on success calls `maos_a2a_core::verify_pinned` against the leaf fingerprint + `InMemoryTofuPinStore`, returning `rustls::Error` on mismatch
**And** it is wired via `.dangerous().with_custom_certificate_verifier(...)` so it runs on the REAL handshake (not post-connection), consuming the EXISTING `verify_pinned` signature unchanged (AC-A6)
**And** the WebPKI-then-pin ordering is deliberate: an expired/malformed cert is rejected before the pin is consulted, preserving the `CERTIFICATE_EXPIRED` retry path (AC-T5)

**AC-A4 — `LengthDelimitedCodec` framing between socket and `try_from_bytes`**
**Given** TCP is a byte stream with no message boundaries and `try_from_bytes` expects one complete frame
**When** `TcpA2ATransport` reads from / writes to a `tokio_rustls` stream
**Then** it wraps the stream in `tokio_util::codec::Framed` with `LengthDelimitedCodec` (4-byte big-endian `u32` length prefix, explicit `max_frame_length` cap — name a concrete bound, e.g. 1 MiB); each inbound decoded frame is handed to `A2AJsonRpcRequest::try_from_bytes` → `handle_intake`; a frame that decodes structurally but fails JSON parse yields `CODE_PARSE_ERROR` (interlocks AC-T2)
**And** length-prefix framing is the ONLY message-boundary mechanism — no newline-delimited or read-to-EOF fallback exists (grep-asserted)

**AC-A5 — Cert/PKI provisioning, config schema, and `maos-bin` binding**
**Given** v0.6 uses operator-supplied PEM (no embedded CA, no auto-issuance)
**When** a `maos-bin` daemon enables the TCP transport
**Then** a `TcpA2AConfig` struct (deserialized via the existing config layer) is defined with EXACTLY these fields: `listen_addr: SocketAddr` (`:0` in tests, readback via `local_addr` — AC-T/H3); `own_cert_chain: PathBuf` (PEM); `own_private_key: PathBuf` (PKCS#8 PEM); `peer_pins: Vec<PinnedFingerprint>` (pre-paired peer leaf-cert fingerprints loaded into `InMemoryTofuPinStore` at startup — this makes ADR-012 "pre-paired fingerprints" real); `handshake_timeout: Duration` (default 30s, MUST be injectable — AC-T/H5); `ca_roots: Option<PathBuf>` (WebPKI trust bundle — **RESOLVED Option A, see header:** `Some(bundle)` ⇒ CA-chain-to-root **then** pin (default, defense-in-depth); `None` ⇒ pin-only FR23a self-signed posture, same ordered validity prelude, separately tested via AC-T4b)
**And** `maos-bin` gains a daemon-mode binding that, when `TcpA2AConfig` is present, constructs `TcpA2ATransport`, registers it as the `A2ATransport` for `CrossHost` dispatch, and binds the listener — with `maos-kernel-core` receiving NO new public fn (the registration reuses existing Spirit/router wiring); the AC names the `maos-bin` file where the binding lands

**AC-A6 — No protocol-surface churn: 8.5's signatures consumed unchanged**
**Given** Story 8.5 proved the cross-Host protocol in-memory and 8.6 must add "only the WIRE"
**When** `maos-a2a-tcp` consumes the protocol substrate
**Then** the 8.5-frozen signatures are called byte-identically (asserted by an `abi-diff` of `maos-a2a-core`'s public surface showing them **unchanged** — not Added/Removed/Modified): `verify_pinned(...)`; the ADR-012 consent fn (consent rides the EXISTING JSON-RPC field, NOT a new TCP-specific field); `handle_intake(...)` and `A2AJsonRpcRequest::try_from_bytes(...)`; `boot_nonce` + the Lamport `logical_clock` travel in the EXISTING JSON-RPC field where 8.5 placed them (cite the literal field name), with no re-wrapping
**And** if any of these would require a signature change to make TCP work, that is a RED flag the seam (AC-A1) is misplaced — the change is rejected and the seam is moved instead

> **Noted gap — consent vocabulary vs enforcement granularity (Winston, 2026-06-04; DEFERRED beyond 8.6's no-churn scope).** Story 8.5 surfaced that `ConsentAllowlists` holds a free-form `Vec<A2AIntent>` (open vocabulary) but the router enforces consent by matching only `frame.intent.a2a_consent_intent_str()` — the 3-value `IntentClass` projection `{highprivilege, standard, readonly}` (`maos-a2a/src/adapter.rs:144-164`). Consequence: an operator can write a specific intent like `A2AIntent::new("diagnosis-handoff:read-only-evidence")` or `"rca-summary"` into an allowlist (as the `smoke-a2a-loopback-6-3` arm aspirationally does) and it will **silently never match** — ADR-012 is, today, "typed-*class* consent," not fine-grained typed-intent consent. **This is NOT 8.6 work:** 8.6 is deliberately churn-free (AC-A6 — a consent-fn signature change is a RED flag), and `maos-a2a-core` must consume 8.5's signatures byte-identically. The note lives here because 8.6 is the moment the protocol crate reopens (`maos-a2a` → `maos-a2a-core` + `maos-a2a-tcp`), so a future **consent-vocabulary story** (widen the intent taxonomy, or match a real per-frame intent field instead of the 3-band projection — an ADR-012 refinement) should be scoped *after* 8.6 lands the seam, against `maos-a2a-core`. Until then the coarse 3-band gate is the accepted v1.5 behavior; mitigation is the clarifying note in Story 8.5's Dev Agent Record. **Recommended:** open this as a v1.5+ backlog story once 8.6 establishes `maos-a2a-core`.

**AC-A7 — Kernel-KLOC zero-delta + doc reconciliation**
**Given** the zero-kernel-KLOC mandate
**When** 8.6 lands
**Then** `maos-kernel-core` is **byte-identical** to its pre-story state (assert exact equality, as Story 8.4 did with 15505 — interlocks AC-T12); the kernel-KLOC sentinel is GREEN; `4-kernel-design.md` is reconciled to describe `maos-a2a-core` (protocol seam) + `maos-a2a-tcp` (live wire) with the dependency arrows `maos-a2a-tcp → maos-a2a-core ← maos-a2a` drawn explicitly; all discipline gates GREEN at HEAD (not flipped-while-red)

### Test harness preconditions (H1–H6 — Murat; referenced by every AC-T below)

- **H1 — Time-relative cert fixtures (rcgen, generated at setup, never committed).** A helper `mk_cert(role, not_before_offset, not_after_offset)` issues certs at test-setup via `rcgen`, offsets from a single `T0 = SystemTime::now()` captured once per test: at minimum `valid` (T0−1h..T0+1h), `expired` (T0−2h..T0−1h), `not_yet_valid` (T0+1h..T0+2h) issued by `ca_good`, plus an independent `ca_evil` root. **No dated `.pem` committed.** Guard: `git ls-files` under the test dir yields zero `*.pem`/`*.crt`/`*.key`.
- **H2 — Single pinned clock.** TLS validation wall-clock and the rotation-drill injected clock are the SAME injected `Clock` (default `T0`). Guard: shared-`Arc` identity check; no test reads `SystemTime::now()` after `T0` for an expiry decision.
- **H3 — Ephemeral port + readback.** Listeners bind `127.0.0.1:0`; the test reads `local_addr()` and dials THAT. Guard: no host:port literal in networking tests except `:0`.
- **H4 — Readiness handshake, not sleep.** Server sends its resolved `SocketAddr` over a `oneshot` AFTER `local_addr()` succeeds; client awaits before dialing. Guard: zero `sleep` in setup paths (any present must be `tokio::time::advance` under `start_paused`).
- **H5 — Injectable timeouts.** Intake/handshake/idle timeouts are constructor params with a `test_profile()` ≤ 250ms (the 30s prod default lives only behind the prod constructor); timeout-path tests complete `< 2s` wall.
- **H6 — Deterministic teardown.** Every spawned process uses `Command::kill_on_drop(true)`; every spawned task is held by a `JoinHandle` aborted in a drop guard. Guard: a teardown-leak test spawns→drops→asserts the bound port is re-bindable within 250ms.

### Test / Risk ACs (AC-T1–AC-T13 — Murat)

**AC-T1 — Live mTLS round-trip over a real socket (happy path; replaces the AC3 intake half)**
**Given** two endpoints bound `127.0.0.1:0` (H3), pre-paired fingerprints, `valid`/`ca_good` certs (H1) under the pinned clock (H2)
**When** the client dials the readback address (H4), completes the mTLS handshake, sends one well-formed `CrossHost` consent frame (ADR-012)
**Then** `handle_intake` returns ACK; the decoded frame's `boot_nonce` and Lamport timestamp are **byte-equal** to sent; the server-observed peer fingerprint equals the pinned fingerprint. **Oracle:** `ack.code == ACK`; `decoded.boot_nonce == sent.boot_nonce`; `decoded.lamport == sent.lamport`; `observed_fp == pinned_fp`. No latency assertion. First test exercising a real handshake — gates everything below.

**AC-T2 — Malformed frame over a live, authenticated connection → typed NACK**
**Given** an established mTLS connection (AC-T1) **When** the client sends bytes that fail `try_from_bytes` (induce: truncate a valid frame to half its length-delimited payload; variant 2: corrupted discriminant byte) **Then** the server replies `NACK{code: CODE_PARSE_ERROR}` and the connection stays open for a subsequent valid frame. **Oracle:** `nack.code == CODE_PARSE_ERROR` both variants; a follow-up valid frame on the same connection returns ACK (codec resynced / not poisoned).

**AC-T3 — TOFU pin mismatch (valid cert, wrong identity) → handshake REJECTED *(MANDATORY — whole security model)***
**Given** the server presents a `valid`/`ca_good` cert whose fingerprint is NOT pinned (pin `fp_A`, server presents `fp_B ≠ fp_A`) **When** the client dials and `TofuPinningVerifier` runs `verify_pinned` **Then** the handshake fails **before any application frame**, the client surfaces a pin-mismatch error, and `handle_intake` is never entered. **Oracle:** dial returns `Err` classified as TOFU pin mismatch (NOT generic IO); server `intake_entered: AtomicUsize == 0`; **no NACK frame** (rejection at TLS layer) — app read side observes connection-closed. Primary negative test for AC-A3's verifier.

**AC-T4 — Wrong CA (valid-but-untrusted root) → handshake REJECTED**
**Given** the server presents a `ca_evil`-issued cert, otherwise well-formed and in-validity **When** the client dials **Then** the WebPKI layer of `TofuPinningVerifier` rejects the chain before TOFU is consulted. **Oracle:** dial `Err` = bad-cert/untrusted-issuer; `intake_entered == 0`; connection-closed. Asserts WebPKI→TOFU ordering: untrusted-CA rejected even if a fingerprint were coincidentally pinned.

**AC-T5 — Expired / not-yet-valid cert → REJECTED, retry policy engages on cert codes**
**Given** the server presents `expired` (and case 2: `not_yet_valid`) under the pinned clock T0 (H2) **When** the client dials with `HandshakeRetryPolicy` (test_profile backoff, ≤3 attempts) **Then** the handshake fails with the cert-expiry/not-yet-valid code, `HandshakeRetryPolicy` retries (these ARE its codes), and after exhausting attempts the client surfaces a terminal cert error. **Oracle:** observed retries `== policy.max_attempts` (proves retry fired on `CERTIFICATE_EXPIRED`); terminal `Err` cert-class; `intake_entered == 0`. Pinned clock ⇒ no slow-runner flake.

**AC-T6 — MITM cert-swap after pin (TOFU defends rotation) → REJECTED**
**Given** the client has pinned `fp_A` from a prior connection (run AC-T1) **When** a new connection presents `fp_C ≠ fp_A` issued by `ca_good` **Then** `verify_pinned` rejects despite WebPKI success. **Oracle:** dial `Err` (TOFU mismatch); `intake_entered == 0`; pin store still holds `fp_A` (`store.get(peer_id) == fp_A` — not silently overwritten). Distinct from AC-T3: here a valid prior pin exists and must win.

**AC-T7 — Slow-loris / stalling intake → bounded timeout, task does NOT hang *(MANDATORY — the test 8.5 deferred twice)***
**Given** an authenticated connection (AC-T1) and intake timeout ≤ 250ms (H5) **When** the client (a) advertises N bytes and stalls after N−1; (b) sends zero application bytes; (c) dribbles one byte every 100ms past the idle timeout **Then** the server aborts intake within the timeout, replies `NACK{code: CODE_TIMEOUT}` (or closes if past handshake — per AC-A4 framing), and the intake task **completes** (not leaked). **Oracle:** whole test completes `< 2s` (H5); the server intake `JoinHandle::is_finished() == true` after the window; no growth in an active-intake gauge after teardown. **No third deferral.**

**AC-T8 — Oversized / unbounded frame → rejected before allocation blow-up**
**Given** an authenticated connection **When** the client advertises a length-delimited frame exceeding the codec cap (header claims `MAX+1`) **Then** the server rejects with `NACK{code: CODE_FRAME_TOO_LARGE}` (or AC-A4's codec cap error) without buffering the full payload. **Oracle:** `nack.code` is the cap code; peak intake buffer ≤ cap (test allocator counter, OR assert the reject fires after only the header is sent). No OOM, no hang.

**AC-T9 — Plaintext client hits the TLS port → rejected, no panic, no hang**
**Given** the server listening for mTLS **When** a raw `TcpStream` writes plaintext bytes (no ClientHello) **Then** the rustls handshake fails, the connection closes within the handshake timeout, `handle_intake` is never entered, and the server survives to accept a subsequent valid mTLS connection. **Oracle:** `intake_entered == 0`; a follow-up real mTLS connection on the same listener succeeds (accept loop didn't die); `< 2s`.

**AC-T10 — Half-open connection (client drops mid-handshake / mid-frame) → cleaned up**
**Given** the server listening **When** the client establishes TCP, begins TLS, then drops (`drop(stream)` after partial ClientHello) **Then** the per-connection task observes EOF/reset and terminates without leaking. **Oracle:** active-connection gauge returns to its pre-connection value within the timeout; accept loop still live (follow-up valid connection succeeds).

**AC-T11 — REAL-socket cert-rotation chaos as its OWN AC (extracted from the old AC4)**
**Given** a 3-endpoint topology (H3/H4) over **real sockets and real TLS handshakes** — explicitly NOT the synthetic `RotationDrillReport` timing model — under the single pinned clock (H2), with `mk_cert` issuing `fp_old`/`fp_new` at deterministic offsets **When** the drill rotates each endpoint's serving cert `fp_old → fp_new` while peers hold live pins and re-pin per the documented rotation protocol **Then** during the window each dial either succeeds against an already-re-pinned peer OR fails TOFU-mismatch and engages `HandshakeRetryPolicy`, and after the window all endpoints converge to `fp_new` pins with the connectivity matrix fully green. **Oracle:** final pin-store state on all 3 == `fp_new`; full NxN reachability ACK post-convergence; retry counters bounded by `max_attempts`; grep guard `RotationDrillReport` NOT referenced in this AC's module (it is the OLD class). **Scope note:** the synthetic `cert_rotation_chaos_3_host.rs` may remain as a fast smoke but MUST NOT be the evidence for this AC.

**AC-T12 — Falsifiable absence-assertions (replaces old-AC4 "no kernel retry" / old-AC5 "kernel unchanged" prose)**
**Given** the full live-transport suite **When** any cert/partition failure path executes (AC-T3..T11) **Then** (a) the kernel performs **zero** auto-retry — kernel-side retry counter `== 0` (the ONLY retrier is `HandshakeRetryPolicy` on the transport side); and (b) `maos-kernel-core` is **byte-identical** to its pre-story state. **Oracle:** (a) `kernel_retry_count: AtomicUsize == 0` (or a fail-on-call test double); (b) a checksum / `git diff --stat`-empty gate for the crate (analogous to Story 8.4's 15505 check). Prose is NOT acceptable evidence — the checksum/diff is.

**AC-T13 — CI determinism conformance (the harness mandates, made gate-checkable)**
**Given** the new `maos-a2a-tcp` integration test suite **When** CI runs it (plus a stress variant: 50× loop or `--test-threads=8`) **Then** the suite is hermetic: no hardcoded ports (H3), no fixed sleeps in setup (H4), injectable timeouts (H5), kill-on-drop teardown (H6), time-relative certs (H1) under one clock (H2). **Oracle:** the H1–H6 guard tests all pass AND a CI repeat-runner (looped 50× / nextest `--retries 0 --test-threads=8`) is **100% green** — any single flake fails the AC. This is the gate that prevents this security story becoming the next §A2-style CI-only-flake debt.

> **Red-phase note for the dev:** start with the H1–H6 harness + AC-T1 (the only happy path); then AC-T3 and AC-T7 most change the security posture and most expose missing verifier/timeout wiring. AC-T3–T6 consume AC-A3's `TofuPinningVerifier` as the unit under test — coordinate its error taxonomy (concrete enum variants) so the "TOFU-mismatch vs bad-cert" oracles match. Two facts to cite from source before AC-A1 closes: the file:line of the `CrossHost` dispatch match the trait binds to, and the literal JSON-RPC field name carrying Lamport/`boot_nonce` in 8.5's source.

---
