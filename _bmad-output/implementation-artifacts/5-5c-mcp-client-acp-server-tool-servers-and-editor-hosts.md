---
dev_model_used: claude-opus-4-7
---

# Story 5.5c: MCP Client + ACP Server — Tool Servers and Editor Hosts

Status: done

dev_model_used: TBD (recommend `claude-opus-4-7`, see Dev Notes §Model Recommendation)

**Epic:** 5 — Spirit Lifecycle, Hot-Swap, Crash Supervision & Multi-Provider (v0.3 → v1.0)
**Epic state at story open:** `epic-5: in-progress` (Stories 5.1 + 5.2 + 5.3 + 5.4 + 5.5a + 5.5b closed `done`; 5.5d/5.5e still `backlog`).
**Story key:** `5-5c-mcp-client-acp-server-tool-servers-and-editor-hosts`

**Predecessors (substrate this story extends):**

- **Story 1b.2** (capability registry decomposition — `Scope` enum + `CapabilityRegistryAdapter` + `cap_audit::record_drop()` ADR-030 audit-channel pattern). Concretely:
  - The `Scope` enum at `crates/maos-domain/src/invariants/i1.rs:53-88` is `#[non_exhaustive]` so Story 5.5c can add `Scope::McpCall { server: String, tool: String }` as an **additive** variant without ABI bump (preserves all 13 existing variants including Story 4.4's `LogRecall` / `LogFetch` / `DistillateWrite`).
  - The capability-check pattern in `crates/maos-kernel-core/src/inference/mod.rs:68-89` (the `check_capability(token, provider_id)` shape) is the canonical model `McpClientAdapter::check_capability(token, server, tool)` mirrors verbatim.
  - The `cap_audit::record_drop()` saturation discipline (ADR-030, Story 5.4 §A4) applies to every new audit channel Story 5.5c emits to.
- **Story 1b.4** (Inference Port + `IoSubsystemPort::http_post` + ComplianceClaim freeze + `InferencePortAdapter` Transparency-Log emission shape). Concretely:
  - The `IoSubsystemPort` trait at `crates/maos-domain/src/ports/io_subsystem.rs:15-34` (`http_get` / `http_post`) is the **only** HTTP transport surface the new MCP-Streamable-HTTP transport may consume. Story 5.5c MUST NOT introduce a parallel HTTP client.
  - The `InferencePortAdapter` adapter pattern at `crates/maos-kernel-core/src/inference/mod.rs:35-146` (capability check → IO call → Transparency-Log emission with `FrameKind::CapabilityInvocation` → IAC telemetry round-trip) is the canonical adapter shape `McpClientAdapter` mirrors EXACTLY (substituting `Scope::McpCall` for `Scope::ProviderInfer`).
  - The `monotonic_now_ns()` discipline (Story 5.4 Review Finding §1366 — closed pattern) for ALL TL/journal timestamps. NEVER `wall_clock_now_ns()`. Applies to every TL/journal emit Story 5.5c introduces.
- **Story 3.1** (notification surface dispatch + `NotificationDispatcher` + `AcpEditorChannel` stub). Concretely:
  - The **explicit forward-shape seam this story closes** lives at `crates/maos-director-surface/src/notification.rs:235-249`:
    ```rust
    /// Stub ACP editor channel — Story 5.5c.
    pub struct AcpEditorChannel;
    impl NotificationChannel for AcpEditorChannel {
        fn surface(&self) -> NotificationSurface { NotificationSurface::AcpEditor }
        fn dispatch(&self, _event: &NotificationEvent, _level: NotificationLevel)
            -> Result<(), NotificationError> {
            unimplemented!("Story 5.5c — ACP server notification channel")
        }
    }
    ```
    **Story 5.5c MUST close this stub** with a real implementation that pipes `NotificationEvent`s to all connected ACP editor sessions. The `unimplemented!` macro becoming a real implementation is the story's "the stub is closed" mechanical observation.
  - The `NotificationDispatcher::register(Box<dyn NotificationChannel>)` composition pattern at `crates/maos-director-surface/src/notification.rs:58-60` is the registration shape the composition root uses to wire the ACP channel.
  - The 4 `NotificationEvent` variants (`TaskAssigned`, `ApprovalPrompt`, `Halt`, `AnomalyFlagged`) at `crates/maos-domain/src/notification.rs:27-66` are the events the ACP channel MUST render to its connected editors. Each variant's rendering format is defined by ACP wire-protocol convention (see §Dev Notes §ACP Wire Schema).
- **Story 5.1** (`LifecycleResolver` trait + 11 lifecycle triggers + full lifecycle verbs via authenticated control plane). Concretely:
  - The `LifecycleResolver` trait at `crates/maos-domain/src/lifecycle.rs:133-148` carries the contract:
    ```rust
    /// Operator-facing lifecycle resolver — implemented by `maos-kernel-core`'s
    /// `KernelLifecycleResolver`, consumed by `maos-cli`, `maos-acp` (Story 5.5c),
    /// and `maos-control` (Story 5.4/9.4 operator HTTP API).
    pub trait LifecycleResolver: Send + Sync {
        fn resolve_verb(&self, spirit_id: &str, verb: LifecycleVerb)
            -> Result<LifecycleReceipt, LifecycleError>;
    }
    ```
    The ACP server consumes `Arc<dyn LifecycleResolver>` (injected at composition root) and forwards editor-initiated `load` / `start` / `pause` / `resume` / `unload` verbs through it. **The ACP server MUST NOT call the kernel-core lifecycle path directly — adapter-only boundary per `maos-domain` isolation contract.**
- **Story 5.2** (hot-swap + `HotSwapResult` + `HotSwapVerb` in `maos-domain`). The ACP server SHOULD surface hot-swap completion events to editors as `NotificationEvent::HotSwapCompleted` (v0.5-α: not required; structural forward-shape if Story 5.5c chooses to emit). At v0.5-α the ACP channel surfaces the 4 existing `NotificationEvent` variants only; hot-swap surface is a Story 6.5+ extension. Documented in Dev Notes.
- **Story 5.5a** (sandbox tier T3 + `crates/maos-sandbox/src/runtime_detect.rs` + `--network=none` default + smoke-t3-sandbox-5 arm). **Trust-tier dovetail:** the sandbox-tier floor for an MCP server's *trust posture* is documented in `crates/maos-kernel-core/src/security/sandbox/t3/mod.rs:40` ("Trigger: Story 5.5c"). Story 5.5c MUST attach a `[mcp].server_trust_tier = "local|org-internal|public-untrusted"` field on the per-server manifest entry; the kernel applies the strictest-of floor to the MCP server's *exec context* exactly the way Story 1b.3 does for Spirits. The full T3-container-wrapping of `public-untrusted` MCP servers is **Story 5.5d's** concern (the registry-side admission path); Story 5.5c only **declares** the field shape and refuses unknown values at admission.
- **Story 5.5b** (multi-provider router + composition-root assembly + `FixtureReplayProvider` test helper + `MAOS_ONE_SHOT=smoke-multi-provider-5` arm at `crates/maos-bin/src/main.rs:2068-2157`). **Disciplines Story 5.5c inherits and MUST NOT regress:**
  - **Composition-root assembly pattern.** The 5.5b assembly at `crates/maos-bin/src/main.rs:384-405` (assemble `BTreeMap<String, Arc<dyn Provider>>` from env-gated optional constructors → default to first-registered) is the canonical multi-instance assembly model. Story 5.5c mirrors it for the MCP client (`BTreeMap<String, Arc<dyn McpTransport>>` keyed by transport identifier — `stdio` / `sse` / `streamable_http`) and for the ACP server (single-instance, but the assembly pattern is the same: optional construction gated on `MAOS_ACP_ENABLE`).
  - **`FixtureReplayProvider` test helper at `crates/maos-providers/src/fixture_replay.rs`** is the precedent for **`FixtureReplayMcpServer`** (NEW, this story) and **`FixtureReplayAcpClient`** (NEW, this story) — both gated by `#[cfg(any(test, feature = "fixture_replay"))]`, both declarative (`responses: VecDeque<...>`), both record-and-replay so smoke arm + matrix runner never depend on live network.
  - **Trait alias re-export pattern** (`pub use Provider as ProviderDriver;` at `crates/maos-providers/src/lib.rs:23`) — Story 5.5c will NOT introduce a parallel trait alias here, but the **doc-comment convention** ("the canonical name in this crate is X; the alias exists for epic-AC text alignment without renaming the trait that already has consumers") applies if any internal naming friction arises.
  - **Smoke arm extension pattern.** Add `smoke-mcp-acp-5` to the known-modes list at `crates/maos-bin/src/main.rs:2163`. The smoke arm walks 5–6 numbered JSON-line surfaces — same shape as `smoke-multi-provider-5` at `main.rs:2068-2157` (one JSON line per surface, exit 0 after the last line, the corresponding test driver at `crates/maos-bin/tests/smoke_mcp_acp_test.rs` parses exit code + JSON-line shape).
  - **`monotonic_now_ns()` discipline** for ALL new TL/journal timestamps (Story 5.4 Review Finding §1366 — closed pattern). NEVER `wall_clock_now_ns()`.
  - **`try_send` + `cap_audit::record_drop()` on saturation** for any new audit-channel emit (ADR-030, Story 1b.2 lesson §6, Story 5.4 confirmation). NEVER `.await` on audit channels.
  - **`#[doc = "Construct via [`X::new`] ..."]` on every NEW pub serde field** matched by an `impl X { pub fn new(...) -> Self | Result<Self, Err> }` constructor — the `xtask check-pub-field-constructors` gate enforces it (Story 5.4 §A4).
  - **`serde_json::to_vec(&x).map_err(...)` — NEVER `.unwrap_or_default()`** on serde paths (Story 5.4 Review Finding §1373 — closed pattern). Applies to every JSON serialization in the ACP NDJSON encoder + MCP request body encoder.
  - **JoinHandle self-prune** on any new async task (Story 5.4 Review Finding §1368 — closed pattern). Applies to the ACP server's per-session accept loop + the MCP client's transport-level reader tasks.
  - **`#[maos_attrs::i9_exempt(reason = "...")]` annotations** on serde structs that hold parsed-then-dropped configuration (manifest-data class). Applies to the new `McpSection` and `AcpSection` manifest types Story 5.5c introduces.
  - **NO `.await` on the inference / capability hot path** (ADR-010 sync-only port traits). The MCP client adapter's `complete` equivalent (`call`) MUST follow the same sync-trait-with-spawn_blocking pattern as `InferencePort`.

**Carry-forward closures expected at story open** (Story 5.5b review-patch items + Story 5.4 carryovers the dev agent must verify CLOSED before the first commit on 5.5c):

- **Story 5.5b Review Findings table** — verify the post-review state at `_bmad-output/implementation-artifacts/5-5b-run-the-multi-provider-ci-matrix-across-anthropic-openai-and-ollama.md` Review Findings section: any `open` row blocks 5.5c dev-start; any `deferred → Story 5.5c` row IS picked up here (audit the table — if anything was deferred to 5.5c, add a row to this story's Review Findings table forward-referencing the resolution path).
- **Story 5.5b §1366 `monotonic_now_ns` discipline** — closed; Story 5.5c follows it. Any new TL or journal emit (e.g. the new `FrameKind::McpInvocation` Transparency-Log row OR a new `LifecycleEvent::McpServerRegistered` IF Story 5.5c decides to journal MCP-server registration — see Dev Notes §Decision Register on this open question) MUST use `monotonic_now_ns()`.
- **Story 5.5b §1373 `serde_json::to_vec().unwrap_or_default()`** — closed pattern; the new MCP request encoder + ACP NDJSON encoder MUST propagate serde errors (no silent drop). The ACP per-frame write path is performance-critical (editor latency) — error propagation here surfaces as `AcpError::EncodeError` returned through the session-task channel; the session task logs the error to the Transparency Log before dropping the frame, NOT a silent default.
- **Story 5.5b §A4 `check-pub-field-constructors`** — every new `pub` field on `McpSection`, `McpServerEntry`, `AcpSection`, `AcpSessionState`, and any other new serde-bearing structs carries the `#[doc = "Construct via ::new ..."]` annotation and a matching `impl ::new` constructor.
- **Story 5.5b doc-comment overreach** — `crates/maos-domain/src/ports/inference.rs:4-5` was updated in 5.5b to defer streaming/embeddings to a follow-up. **Story 5.5c does NOT add `stream` / `embed` either**; the MCP `call` surface returns a single `McpCallResponse` per invocation. Streaming MCP responses (where applicable, e.g., long-running tool invocations) are deferred to Epic 6 Story 6.4 or later — confirm in Dev Notes.
- **Story 5.5a §SandboxBlock-emit-via-probe-sidecar** — flagged for awareness only; the MCP client does NOT spawn containers. MCP server containerization is Story 5.5d's concern.
- **Story 5.4 §1370 `ColdSwap bypasses scheduler.load()`** — same forward-shaped Epic 6 dependency. MCP/ACP does NOT touch the cold-swap path; flagged for awareness.

**Successor stories in Epic 5 + later epics:**

- **5.5d** (Spirit Registry over MCP-Streamable-HTTP) — **DIRECTLY DEPENDS on Story 5.5c**. The registry IS an MCP-Streamable-HTTP server; Story 5.5d's `RegistryClient` impl invokes the **Story 5.5c MCP client** with `transport = "streamable_http"`. Story 5.5c MUST publish a stable `McpClient::call(server_uri, tool, args)` surface that 5.5d consumes unchanged. The forward-shape contract: `crates/maos-mcp/src/client.rs::McpClient::call(&self, server_uri: &str, tool: &str, args: serde_json::Value) -> Result<McpCallResponse, McpError>` — sync per ADR-010, async wrapping handled by callers via `spawn_blocking`. Documented in Dev Notes §Surface Stability Contract.
- **5.5e** (§13.1 rust-inproc measurement gate) — **orthogonal**. The §13.1 J1+J4 measurement workloads do not invoke MCP or ACP; the §13.1 bench surfaces remain pure IPC latency. If the §13.1 measurement triggers `unlock-rust-inproc-in-v0.5`, MCP/ACP transports are unaffected (they live in adapter crates outside the Spirit-form boundary).
- **Epic 6 Story 6.4** (scheduled invocations + provider rate-limit isolation) — Story 5.5c's per-server MCP transport selection is the **substrate** Story 6.4's rate-limit isolation builds on. If Story 5.5c lands per-server bounded queues (RECOMMENDED — see Dev Notes Decision Register), Story 6.4 inherits them for rate-limit shaping.
- **Epic 6 Story 6.5** (gateway sub-modules per ADR-029 — Telegram / Slack / Discord / Signal / Email) — the **`MobilePushChannel`** stub at `crates/maos-director-surface/src/notification.rs:253-267` ("Story 6.5 — mobile push via gateway sub-modules") is Story 6.5's seam. Story 5.5c's `AcpEditorChannel` impl is the canonical template Story 6.5 mirrors when it adds the per-gateway channels.
- **Epic 7 Story 7.4** (skill-authoring surface + skill revisions) — Story 5.5c's ACP server is the surface skill-authoring spirits use to propose revisions. Story 7.4 extends the ACP wire schema with `skill.propose` / `skill.review` / `skill.commit` frame kinds; Story 5.5c only ships the BASELINE wire schema (`session.start` / `session.end` / `task.assign` / `notification.dispatch` / `lifecycle.verb` / `halt.resolve`).
- **Epic 8 Story 8.x** (reference spirits — Butler, Researcher, Observer, founder-loop wedge, Mira-Nash) — every reference Spirit at v0.5+ that uses MCP tools (Butler with Calendar/Slack/Linear/Figma; Researcher with web/arXiv/GitHub/citation-graph) routes through the **Story 5.5c MCP client**. Story 5.5c's `MAOS_ONE_SHOT=smoke-mcp-acp-5` arm is the integration-readiness gate: when smoke passes, Epic 8 reference Spirits can begin authoring against the substrate.
- **Epic 9 Story 9.1** (`maosctl audit query` + ACP-editor + posture-delta + sealed-export) — Story 5.5c's ACP server surfaces `audit query` results to editors. Story 9.1 extends the ACP wire schema with `audit.query` frame kinds; Story 5.5c ships the baseline.
- **Epic 9 Story 9.4** (operator surface — full air-gapped network-namespace isolation test) — Story 5.5c's MCP-Streamable-HTTP transport routes through the existing `IoSubsystemPort` (no parallel HTTP client). Story 9.4's `unshare --net` validation extends to cover MCP outbound traffic — the structural test continues to assert zero packets leave the loopback interface, and Story 5.5c's stdio transport (which has no outbound HTTP) is the **air-gapped MCP path** (MCP servers launched as local subprocesses; communication is pipe-only).
- **Epic 10 Story 10.2** (third-party trial + adversarial red team — including MCP wire fuzz) — Story 5.5c's wire-protocol parsers (JSON-RPC for MCP, NDJSON for ACP) are fuzz targets. The fuzz corpus for both wire shapes lives at `crates/maos-mcp/fuzz/` + `crates/maos-acp/fuzz/` and is **AUTHORED HERE** (a minimum 50-input seed corpus per wire shape; full §5.2 tiered cadence wiring lands in Story 10.2).

<!-- Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **Spirit author at v0.5 wanting (a) to consume MCP tool servers from inside my Spirit binary without ever importing an MCP-protocol library — and a third party authoring a Spirit who wants to ship the Spirit and host it inside their own IDE (Zed, VSCode, future JetBrains) without writing editor-side bridge code — and an evaluator who needs to OBSERVE that the substrate actually speaks MCP across three transports AND hosts a real lifecycle conversation with a real editor, not just two stub crates that compile**,

I want **the v0.5-α MCP-and-ACP substrate that (a) replaces the empty placeholder at `crates/maos-mcp/src/lib.rs` (currently 9 lines of doc comment) with a fully-featured MCP client at `crates/maos-mcp/src/client.rs::McpClient` supporting all three MCP transports (stdio / SSE / Streamable HTTP) via the NEW `McpTransport` trait at `crates/maos-mcp/src/transport/mod.rs::McpTransport` — `fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError>` per ADR-010 sync-only port semantics — with one concrete implementation per transport at `crates/maos-mcp/src/transport/stdio.rs::StdioTransport` (subprocess MCP server launched via `std::process::Command`; bidirectional NDJSON over child stdin/stdout; child-process JoinHandle self-prune per Story 5.4 §1368), `crates/maos-mcp/src/transport/sse.rs::SseTransport` (HTTP+SSE long-lived connection per the MCP 2024-11-05 SSE binding, layered atop `IoSubsystemPort::http_get` with `Accept: text/event-stream`), and `crates/maos-mcp/src/transport/streamable_http.rs::StreamableHttpTransport` (single-shot HTTP POST + chunked-response per the MCP 2025-03 Streamable HTTP binding, layered atop `IoSubsystemPort::http_post` with `Content-Type: application/json` + `Accept: application/json, text/event-stream`); each transport handles its own framing — stdio NDJSON frames, SSE event chunks, Streamable HTTP POST cycles — but all three return the SAME vendor-neutral `McpResponse` domain type defined in `crates/maos-domain/src/ports/mcp.rs::McpResponse` so consumer code is transport-agnostic; (b) routes EVERY MCP invocation through the kernel's capability-mediation path — the NEW `Scope::McpCall { server: String, tool: String }` variant (additive on the `#[non_exhaustive]` Scope enum at `crates/maos-domain/src/invariants/i1.rs:60`; preserves all 13 existing variants) is the capability the Spirit must hold; the NEW `McpClientAdapter` at `crates/maos-kernel-core/src/mcp/mod.rs::McpClientAdapter` holds `Arc<dyn McpClient>` + `Arc<CapabilityRegistry>` + `Arc<TransparencyLog>` and on every `call(token, server, tool, args)` invocation (i) verifies `token.scope == Scope::McpCall { server: <server>, tool: <tool> }` via the existing `CapabilityRegistry::verify` path (mirrors `InferencePortAdapter::check_capability` at `crates/maos-kernel-core/src/inference/mod.rs:68-89`), (ii) emits `FrameKind::McpInvocation = 10` to the Transparency Log via the existing `TransparencyLogAdapter::record(...)` path with `intent = format!("mcp:{server}/{tool}")` (TL FrameKind discriminant `= 10` is additive on the existing 0..9 set — verify HEAD-current `FrameKind` enum max + 1; document the additive bump in §Dev Notes Wire-Schema Register), (iii) routes the request through `McpClient::call` which selects the configured transport via the per-server entry in `manifest.mcp.servers[i].transport`, (iv) on `McpTransportError::Transport(_)` walks `manifest.mcp.servers[i].fallback_transport` (optional; default empty — operators opt into multi-transport fallback explicitly) — fallback exhaustion returns the LAST error; (c) adds the NEW `[mcp]` manifest section at `crates/maos-kernel-core/src/security/manifest.rs::McpSection` carrying `{servers: Vec<McpServerEntry>}` where `McpServerEntry { name: String, uri: String, transport: McpTransportId, fallback_transport: Option<McpTransportId>, server_trust_tier: TrustTier, allowed_tools: Vec<String> }` and `McpTransportId` is the NEW enum `Stdio | Sse | StreamableHttp` at `crates/maos-domain/src/ports/mcp.rs::McpTransportId` with `#[serde(rename_all = "snake_case")]` so manifest TOML reads `transport = "streamable_http"` / `transport = "sse"` / `transport = "stdio"`; the manifest section is OPTIONAL (manifests without `[mcp]` host Spirits that never call MCP — admission proceeds normally); the manifest validator REJECTS with `ManifestError::Toml(...)` when (i) `name` is empty, (ii) `uri` is empty, (iii) `transport` is an unrecognized value (TOML parse error from the `#[serde]` rename), (iv) `server_trust_tier` is unrecognized (Story 5.5d-shaped `public-vetted` is REJECTED — FR37 explicitly excluded from v1.0 per round-2 decision), (v) `allowed_tools` contains an empty string OR a `*` glob (v0.5-α: NO glob expansion — explicit tool names only; glob support deferred to Epic 7 Story 7.4); the strictest-of trust-tier floor from Story 1b.3 + 5.5a applies to MCP servers in EXACTLY the same way as it applies to Spirits (operator-policy floor wins; v0.5-α stdio/SSE transports are `local` or `org-internal` only; `public-untrusted` MCP servers require Streamable HTTP transport AND admission-time ComplianceClaim verification — the full T3-container-wrapping admission path arrives at Story 5.5d); (d) replaces the empty placeholder at `crates/maos-acp/src/lib.rs` (currently 9 lines of doc comment) with a fully-featured ACP server at `crates/maos-acp/src/server.rs::AcpServer` that listens on stdio (bidirectional NDJSON; one frame per line; `serde_json::to_vec(&frame)?` + `writeln!(stdout, ...)` per frame; reader task self-prunes its JoinHandle on EOF per Story 5.4 §1368) and exposes a SESSION-oriented protocol: editor sends `{"kind":"session.start","session_id":"<ulid>","editor_id":"zed|vscode|other","editor_version":"x.y.z"}` → server replies `{"kind":"session.ready","session_id":"<ulid>","supported_kinds":[...]}` → editor sends `{"kind":"lifecycle.verb","verb":"load|start|pause|resume|unload","spirit_id":"<id>","decision_id":"<ulid>"}` → server invokes `Arc<dyn LifecycleResolver>::resolve_verb(spirit_id, verb)` from `crates/maos-domain/src/lifecycle.rs:133-148` (Story 5.1) → server replies `{"kind":"lifecycle.receipt","decision_id":"<ulid>","spirit_pid":<u32>,"verb":"<verb>","timestamp_ns":<u64>,"outcome":"ok|error","error":<null|{...}>}` → server PUSHES `{"kind":"notification.dispatch","level":"immediate|queue|digest","event":<NotificationEvent JSON>}` frames whenever the kernel's `NotificationDispatcher` fires (the closing of the `AcpEditorChannel` stub at `crates/maos-director-surface/src/notification.rs:235-249`) → editor sends `{"kind":"halt.resolve","halt_id":"<id>","resolution":"approve|deny|defer","decision_id":"<ulid>","operator_note":"<optional>"}` → server invokes `Arc<dyn HaltResolver>::resolve_halt(halt_id, resolution)` (the Story 4.1 + 3.3 surface at `crates/maos-domain/src/halt.rs::HaltResolver`) → server replies `{"kind":"halt.receipt","decision_id":"<ulid>","halt_id":"<id>","outcome":"resolved|rejected|deferred","timestamp_ns":<u64>}` → editor sends `{"kind":"session.end"}` → server replies `{"kind":"session.terminated","session_id":"<ulid>","duration_ns":<u64>}` and the session task self-prunes; the SESSION protocol is the smallest viable v0.5-α surface — `audit.query` / `posture.delta` / `skill.propose` frame kinds arrive at Epic 9 / Epic 7 per the forward-shape registry; (e) ships the NEW `AcpEditorChannelImpl` at `crates/maos-acp/src/notification_channel.rs::AcpEditorChannelImpl` that implements `maos_director_surface::NotificationChannel`, holds an `Arc<Mutex<Vec<AcpSessionHandle>>>` of all currently-connected editor sessions, and on `dispatch(event, level)` writes the event to EVERY connected session's outbound frame channel (per-session bounded queue — capacity 256 — drop-oldest on overflow per `telemetry.event` precedent at architecture §7.1.1; saturation increments a `cap_audit::record_drop()` per Story 5.4 §A4); the existing `AcpEditorChannel` stub at `crates/maos-director-surface/src/notification.rs:235-249` is REMOVED (deleted) and replaced by the `AcpEditorChannelImpl` re-exported from `maos-acp::AcpEditorChannelImpl` — the composition root registers it via `dispatcher.register(Box::new(AcpEditorChannelImpl::new(server.session_registry().clone())))`; (f) ships the NEW `tests/integration/acp-editors/` directory with two editor-plugin integration tests: `tests/integration/acp-editors/zed/zed_acp_lifecycle_test.rs` and `tests/integration/acp-editors/vscode/vscode_acp_lifecycle_test.rs`; each test spawns the `maos-bin` binary with `MAOS_ONE_SHOT=acp-server` (a NEW arm), pipes a scripted NDJSON conversation (session.start → lifecycle.verb=load → wait for notification.dispatch=task.assigned → halt.resolve → session.end) through the binary's stdio, and asserts the full conversation completes within a 5s wall-clock budget (the JoinHandle self-prune + session-task tear-down completes synchronously on session.end); the **EDITOR PLUGIN CODE ITSELF is OUT OF SCOPE** for Story 5.5c (the actual Zed extension and VSCode extension are written by editor-plugin authors against the wire schema this story ships) — the integration tests use scripted NDJSON to simulate the editor side, NOT the real plugins; the real-plugin contract test arrives at v1.0 ship-gate as an external validation (documented in `deferred-work.md`); (g) preserves the Story 0.2 kernel-API surface invariant — the NEW kernel-side adapter symbols `McpClientAdapter` + `AcpServerAdapter` (if exposed) get classified in `xtask/kernel-api-classes.toml` per the NFR-Test-2 gate; per the architecture §4.0.7 four-class taxonomy, MCP client invocation is **data-movement** (no semantic interpretation — frames are routed to MCP server and back; the Spirit interprets responses, the kernel doesn't), so the new adapter rows read `"maos_kernel_core::api::mcp::McpClientAdapter" = "data-movement"` and `"maos_kernel_core::mcp::McpClientAdapter" = "data-movement"`; ACP server has the same classification (NDJSON frames are routed; no semantic interpretation in the adapter); the `xtask check-service-boundary` gate passes; **`maos-domain` does NOT import any MCP/ACP types** — the protocol-side serialization shapes live in `maos-mcp` / `maos-acp`, the PORT TRAITS (`McpClient`, `AcpServerPort`) live in `maos-domain::ports::mcp` / `maos-domain::ports::acp` respectively (sync-only per ADR-010), and adapter implementations live in `maos-kernel-core::mcp` and `maos-acp` consuming the ports; the boundary is structurally enforced by the existing `cargo public-api` check in CI; (h) ships the NEW `MAOS_ONE_SHOT=smoke-mcp-acp-5` arm at `crates/maos-bin/src/main.rs` (additive on the existing match block; the known-modes list at line 2163 EXTENDS to include `smoke-mcp-acp-5` AND `acp-server` — the latter is the long-running ACP server arm referenced by AC4's integration tests) walking the MCP + ACP substrate end-to-end using `FixtureReplayMcpServer` + `FixtureReplayAcpClient` so the arm runs deterministically on any CI runner including those without editors installed: print `{"step":1,"surface":"mcp_client_init","transports":["stdio","sse","streamable_http"],"default":"streamable_http"}` → register a fixture-replay MCP server returning a canned tool-call response, issue `McpClient::call("test-server", "echo", {"msg":"hello"})`, print `{"step":2,"surface":"mcp_call","outcome":"ok","server":"test-server","tool":"echo"}` → simulate `McpTransportError::Transport("connection_refused")` on the primary transport, assert the adapter walks fallback, print `{"step":3,"surface":"mcp_fallback","outcome":"ok","primary":"streamable_http","fallback_used":"stdio"}` → register a `FixtureReplayAcpClient`, simulate `session.start` → `lifecycle.verb=load` → assert `lifecycle.receipt` returns, print `{"step":4,"surface":"acp_session","outcome":"ok","verb":"load"}` → dispatch a `NotificationEvent::TaskAssigned` through the `NotificationDispatcher` with the `AcpEditorChannelImpl` registered, assert the FixtureReplayAcpClient receives a `notification.dispatch` frame, print `{"step":5,"surface":"acp_notification","outcome":"ok","level":"immediate","event_kind":"TaskAssigned"}` → simulate `halt.resolve` from the FixtureReplayAcpClient, assert `HaltResolver::resolve_halt` was called, print `{"step":6,"surface":"acp_halt_resolve","outcome":"ok","resolution":"approve"}` → exit 0 after printing 6 JSON lines; the smoke arm is the Layer-1.5 observability bridge for Story 5.5c that smoke-multi-provider-5 is for 5.5b — closes Lunarpulse's evaluation discipline per `[[feedback_lunarpulse_observability_preference]]` ("when can I observe actual behavior beats coverage%")**,

so that **(a) the §7.5 four-protocol commitment ("Kernel-internal IAC + bilateral A2A + ACP + MCP") gets its v0.5-α concrete realization — every PR runs the matrix tests; the substrate's claim of "speaks MCP" is no longer aspirational; (b) the ADR-008 Spirit registry binding-v0.5 gate is **unblocked** — Story 5.5d's `RegistryClient` impl consumes the Story 5.5c `McpClient::call` surface as-is, with no API churn between the two; the registry roundtrip corpus (Story 5.5d AC5) becomes runnable; (c) the FR47 contract ("Spirit binary does not import MCP client implementations directly") stays structurally closed — the Story 5.5c MCP client lives in adapter crates outside the Spirit binary boundary; Spirit binaries continue to call `capability/invoke(token, args)` and receive typed events through the kernel-mediated path; (d) the v0.5-α "ACP server (Zed + VSCode tested)" phase-roadmap commitment at architecture §13 row v0.5 gets its observability leg — the integration tests run on every CI run, the smoke arm runs on every developer-host invocation, and editor-plugin authors get a runnable wire-schema reference (the smoke arm's stdout is the canonical "this is what the conversation looks like" specification); (e) the §5 Spirit ABI architectural commitment ("A Spirit's code does not contain MCP client implementations") gets its v0.5-α concrete demonstration — the kernel-side adapter is the only consumer of MCP wire protocols; Spirit code calls `kernel.mcp.call(...)` and receives typed responses; (f) the §13 v0.5 row commitment "broad MCP capabilities (web/arXiv/GitHub/citation-graph)" gets its substrate — Epic 8's Researcher Spirit can now author against a real MCP client surface, not a stub; (g) the Story 3.1 `AcpEditorChannel` stub at `crates/maos-director-surface/src/notification.rs:235-249` is **mechanically closed** — running `grep -rn 'unimplemented!.*Story 5.5c' crates/` returns zero matches after this story ships; the seam is real code, not a TODO; (h) the §7.4 "kernel-rendered notification surfaces" invariant ("a Spirit cannot bypass the user's notification policy") is structurally preserved across the new ACP surface — every `NotificationEvent` reaching an editor passes through `NotificationDispatcher`, not through a back-channel Spirit-emitted frame; the architecture invariant holds across the new surface; (i) the Story 0.2 kernel-API surface lint stays passing — three new adapter symbols (`McpClientAdapter`, `AcpServerAdapter`, `AcpEditorChannelImpl`) are added to `kernel-api-classes.toml` as `data-movement`; the FR47 vendor-SDK denylist gate stays empty-allowlist (no MCP-protocol library is added to the workspace — JSON-RPC + NDJSON are direct-implemented in MAOS code; this is provable via `cargo tree | grep -E 'jsonrpc|mcp-client|acp-protocol'` returning empty); (j) when an evaluator runs `MAOS_ONE_SHOT=smoke-mcp-acp-5 cargo run -p maos-bin`, they OBSERVE the MCP client dispatching across three transports, the MCP fallback chain activating on synthetic transport error, a real ACP session being established, a `lifecycle.verb=load` being resolved through the kernel, a `notification.dispatch` frame being pushed to the editor, and a `halt.resolve` being handled by the kernel IN ONE COMMAND — the substrate's MCP+ACP claim is no longer "we have two empty crates" but "we have three transports, a router, fallback, capability mediation, a session-oriented ACP server, halt routing, lifecycle routing, and notification delivery, demonstrated"**.

## What this story IS

### MCP CLIENT — substrate

- **NEW `crates/maos-mcp/src/client.rs::McpClient`** — the **public consumer-facing API** Story 5.5d / Epic 8 reference Spirits consume:
  ```rust
  /// MCP client — operator-configurable per-server transport selection.
  ///
  /// Public surface this crate exposes to callers (consumers: the kernel-side
  /// adapter at `maos-kernel-core::mcp::McpClientAdapter`; Story 5.5d's
  /// registry client; Epic 8 reference Spirits invoke this through the
  /// capability/mediation path).
  ///
  /// ADR-010 sync trait — `call` returns synchronously; async callers wrap in
  /// `spawn_blocking`. Per the kernel-stays-small invariant, no Tokio runtime
  /// is required inside this crate at v0.5-α; the kernel-side adapter performs
  /// any necessary task spawning.
  pub struct McpClient {
      transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>>,
      default_transport: McpTransportId,
      servers: BTreeMap<String, McpServerEntry>,
  }

  impl McpClient {
      #[doc = "Construct via [`McpClient::new`] to enforce non-empty transport map and server validation."]
      pub fn new(
          transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>>,
          default_transport: McpTransportId,
          servers: BTreeMap<String, McpServerEntry>,
      ) -> Result<Self, McpError> { /* validate servers reference registered transports */ }

      /// Class: data-movement
      ///
      /// Invoke an MCP tool. Caller is responsible for the capability-token
      /// check; that mediation lives in `McpClientAdapter` (kernel-side).
      pub fn call(
          &self,
          server_name: &str,
          tool: &str,
          args: serde_json::Value,
      ) -> Result<McpCallResponse, McpError>;

      /// Registered server names — the operator-visible inventory.
      pub fn registered_servers(&self) -> Vec<String>;
  }

  #[derive(Debug, Clone, thiserror::Error)]
  pub enum McpError {
      #[error("unknown server '{0}'")]
      UnknownServer(String),
      #[error("transport error: {0}")]
      Transport(#[from] McpTransportError),
      #[error("encode error: {0}")]
      Encode(serde_json::Error),
      #[error("decode error: {0}")]
      Decode(String),
      #[error("capability denied: scope mismatch on {server}/{tool}")]
      CapabilityDenied { server: String, tool: String },
      #[error("unconfigured")]
      Unconfigured,
  }
  ```
  Tests at `crates/maos-mcp/src/client.rs::tests`:
  - `call_routes_to_per_server_transport` — register two servers with different transports; assert each call routes to the right transport.
  - `call_returns_unknown_server_for_unregistered` — `McpError::UnknownServer`.
  - `call_walks_fallback_transport_on_transport_error` — primary transport returns `Transport(_)`; fallback transport returns Ok; assert response carries the fallback transport's source attribution.
  - `call_does_not_walk_fallback_on_decode_error` — `McpError::Decode(_)` is non-retriable (deterministic bug; do NOT silently rebroadcast).
- **NEW `crates/maos-mcp/src/transport/mod.rs::McpTransport`** — the trait the three concrete transports implement:
  ```rust
  /// MCP transport — per-server-URI selectable.
  ///
  /// ADR-010 sync trait. Async callers wrap in `spawn_blocking`.
  pub trait McpTransport: Send + Sync {
      /// Class: data-movement
      fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError>;

      /// Transport identifier — `stdio` / `sse` / `streamable_http`.
      fn id(&self) -> McpTransportId;
  }

  #[derive(Debug, Clone, thiserror::Error)]
  pub enum McpTransportError {
      #[error("transport-level error: {0}")]
      Transport(String),
      #[error("server returned error: {message}")]
      ServerError { code: i32, message: String },
      #[error("timeout after {0}ms")]
      Timeout(u64),
  }
  ```
- **NEW `crates/maos-mcp/src/transport/stdio.rs::StdioTransport`** — child-process subprocess MCP server, bidirectional NDJSON over stdin/stdout:
  - Constructor: `StdioTransport::new(command: &str, args: &[String]) -> Result<Self, McpTransportError>` spawns the subprocess via `std::process::Command::new(command).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()`. **Stdin/stdout MUST be unbuffered** — pipe to the child without intermediate `BufWriter`/`BufReader` for line-oriented framing; use `LineWriter`/`LineReader` on the MAOS side.
  - **JoinHandle self-prune discipline (Story 5.4 §1368):** the per-transport reader task (the task that pulls NDJSON frames off the child's stdout) self-prunes its JoinHandle when EOF is reached or the child process exits. The transport's `Drop` impl sends `SIGTERM` to the child and waits up to 5s before sending `SIGKILL`.
  - Tests: spawn `echo` as a fake MCP server, send a JSON-RPC request, assert the response decoder handles partial-line reads + multi-line frames.
- **NEW `crates/maos-mcp/src/transport/sse.rs::SseTransport`** — HTTP+SSE long-lived connection:
  - Constructor: `SseTransport::new(transport_inner: Arc<dyn IoSubsystemPort>, endpoint_url: String) -> Result<Self, McpTransportError>`.
  - Implementation: layered atop `IoSubsystemPort::http_get` with `Accept: text/event-stream` header; the response body is parsed as SSE-format chunks per `data: {json}\n\n` framing per the MCP 2024-11-05 SSE binding spec.
  - **Note:** SSE is the **least-preferred transport** at v0.5-α (Streamable HTTP supersedes it in the MCP 2025-03 spec). Implementation is included for backwards-compat with existing MCP SSE servers; documented in Dev Notes §Transport Selection.
- **NEW `crates/maos-mcp/src/transport/streamable_http.rs::StreamableHttpTransport`** — single-shot HTTP POST + chunked response per the MCP 2025-03 binding (DEFAULT transport):
  - Constructor: `StreamableHttpTransport::new(transport_inner: Arc<dyn IoSubsystemPort>, endpoint_url: String) -> Result<Self, McpTransportError>`.
  - Implementation: layered atop `IoSubsystemPort::http_post` with headers `Content-Type: application/json` + `Accept: application/json, text/event-stream`; request body is the JSON-RPC request frame; response body is parsed as either single-shot JSON (Content-Type response = `application/json`) OR streamed SSE-format chunks (Content-Type response = `text/event-stream`).
  - This is the **default transport** for new MCP servers at v0.5-α per ADR-008 and the architecture §7.5 commitment "Streamable HTTP is the default for Loom-lite, the Spirit registry, and most production tool servers."
- **NEW `crates/maos-mcp/src/fixture_replay.rs::FixtureReplayMcpServer`** (test-only helper, gated by `#[cfg(any(test, feature = "fixture_replay"))]`; new `fixture_replay` feature in `crates/maos-mcp/Cargo.toml`) — mirrors the `FixtureReplayProvider` precedent from `crates/maos-providers/src/fixture_replay.rs`:
  ```rust
  pub struct FixtureReplayMcpServer {
      responses: Mutex<VecDeque<Result<McpResponse, McpTransportError>>>,
      calls: Mutex<Vec<McpRequest>>,
  }
  impl FixtureReplayMcpServer { /* :: new(responses) + take_calls() */ }
  impl McpTransport for FixtureReplayMcpServer { fn invoke(&self, req: McpRequest) -> Result<McpResponse, McpTransportError> { ... } fn id(&self) -> McpTransportId { McpTransportId::Stdio /* configurable */ } }
  ```
- **EXTENDED `crates/maos-mcp/src/lib.rs`** — replaces the 9-line placeholder:
  ```rust
  #![forbid(unsafe_code)]

  //! `maos-mcp` — Model Context Protocol client (ADR-008).
  //!
  //! Three transports (stdio / SSE / Streamable HTTP) per MCP 2024-11-05 +
  //! 2025-03 bindings; Streamable HTTP is the v0.5-α default.
  //!
  //! Consumer-facing surface: `McpClient::call(server, tool, args)`.
  //! Per-server transport selection is operator-configurable via the
  //! `[mcp].servers[i].transport` manifest field.
  //!
  //! The kernel-side capability-mediation adapter lives in
  //! `maos-kernel-core::mcp::McpClientAdapter` — this crate provides ONLY
  //! the wire-protocol implementation. No capability tokens are checked here.

  pub mod client;
  pub mod transport;
  #[cfg(any(test, feature = "fixture_replay"))]
  pub mod fixture_replay;

  pub use client::{McpClient, McpError};
  pub use transport::{McpTransport, McpTransportError};
  ```

### MCP — DOMAIN PORT + KERNEL ADAPTER

- **NEW `crates/maos-domain/src/ports/mcp.rs`** — port trait + domain types (consumer-facing; `maos-kernel-core` consumes this):
  ```rust
  /// MCP client port — kernel adapter contract.
  ///
  /// Per ADR-010 sync-only port semantics. The adapter (`McpClientAdapter` in
  /// `maos-kernel-core`) wraps capability-mediation + Transparency-Log emission
  /// around the wire-level `McpClient` from `maos-mcp`.
  pub trait McpClientPort: Send + Sync {
      /// Class: data-movement
      ///
      /// Verify the capability token + invoke the MCP tool. Returns a
      /// vendor-neutral response. Capability denial returns `McpError::CapabilityDenied`.
      fn call(
          &self,
          token: &CapabilityToken,
          server: &str,
          tool: &str,
          args: serde_json::Value,
      ) -> Result<McpCallResponse, McpError>;
  }

  /// Vendor-neutral MCP request/response domain shapes. ALL three transports
  /// translate into these types — no transport-specific JSON leaks past
  /// `maos-mcp::transport::*`.
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct McpRequest {
      #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
      pub server: String,
      #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
      pub tool: String,
      #[doc = "Construct via [`McpRequest::new`] to enforce non-empty fields."]
      pub args: serde_json::Value,
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct McpResponse {
      #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
      pub content: serde_json::Value,
      #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
      pub is_error: bool,
      #[doc = "Construct via [`McpResponse::new`] to enforce shape validation."]
      pub attribution: McpAttribution,
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  pub struct McpAttribution {
      pub server_name: String,
      pub transport_id: McpTransportId,
      pub tool: String,
  }

  /// Caller-facing alias — `McpClientPort::call` returns this type.
  pub type McpCallResponse = McpResponse;

  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum McpTransportId {
      Stdio,
      Sse,
      StreamableHttp,
  }
  ```
- **NEW `crates/maos-kernel-core/src/mcp/mod.rs::McpClientAdapter`** — the kernel-side adapter that mediates capability + emits TL:
  ```rust
  /// MCP client adapter — capability mediation + Transparency-Log emission.
  ///
  /// Mirrors `InferencePortAdapter` at
  /// `crates/maos-kernel-core/src/inference/mod.rs:35-146`. Holds the
  /// wire-level `McpClient` from `maos-mcp` + the capability registry +
  /// the Transparency Log adapter.
  #[maos_attrs::i9_exempt(reason = "inference-port adapter aggregate; holds Arc references to wire-level client + audit infrastructure")]
  pub struct McpClientAdapter {
      client: Arc<dyn McpClient>,
      capability: Arc<CapabilityRegistry>,
      transparency_log: Arc<dyn TransparencyLogPort>,
      telemetry: Arc<dyn TelemetryStreamPort>,
  }

  impl McpClientAdapter {
      pub fn new(
          client: Arc<dyn McpClient>,
          capability: Arc<CapabilityRegistry>,
          transparency_log: Arc<dyn TransparencyLogPort>,
          telemetry: Arc<dyn TelemetryStreamPort>,
      ) -> Self { Self { client, capability, transparency_log, telemetry } }

      fn check_capability(
          &self,
          token: &CapabilityToken,
          server: &str,
          tool: &str,
      ) -> Result<(), McpError> {
          let inner = self.capability.policy().inner().load_full();
          let scopes = inner.token_scopes.get(&token.token_id);
          match scopes.and_then(|set| set.iter().find_map(|s| match s {
              Scope::McpCall { server: s, tool: t } if s == server && t == tool => Some(()),
              _ => None,
          })) {
              Some(()) => Ok(()),
              None => Err(McpError::CapabilityDenied { server: server.into(), tool: tool.into() }),
          }
      }
  }

  impl McpClientPort for McpClientAdapter {
      fn call(&self, token: &CapabilityToken, server: &str, tool: &str, args: serde_json::Value) -> Result<McpCallResponse, McpError> {
          self.check_capability(token, server, tool)?;
          let start_ns = monotonic_now_ns();
          let response = self.client.call(server, tool, args.clone())?;
          let end_ns = monotonic_now_ns();
          self.transparency_log.record(TransparencyLogRow {
              frame_kind: FrameKind::McpInvocation,
              spirit_pid: token.spirit_pid,
              timestamp_ns: start_ns,
              intent: format!("mcp:{server}/{tool}"),
              payload_hash: blake3_hash(&serde_json::to_vec(&args).map_err(McpError::Encode)?),
              duration_ns: Some(end_ns - start_ns),
              outcome: if response.is_error { "server_error" } else { "ok" }.into(),
              ..Default::default()
          })?;
          self.telemetry.emit_round_trip(token.spirit_pid, "mcp.call", end_ns - start_ns);
          Ok(response)
      }
  }
  ```
  **`FrameKind::McpInvocation = 10` (NEW)** — additive discriminant on the existing `#[repr(u8)]` `FrameKind` enum at `crates/maos-domain/src/frame.rs` (verify HEAD-current max + 1; document the additive bump in Dev Notes Wire-Schema Register). The discriminant value is fixed (not auto-assigned) so future-proofing relies on the `#[repr(u8)]` ordering.
- **EXTENDED `crates/maos-domain/src/invariants/i1.rs::Scope`** — additive `McpCall { server: String, tool: String }` variant (preserves the `#[non_exhaustive]` invariant; no ABI break):
  ```rust
  pub enum Scope {
      // ... existing 13 variants preserved
      /// Story 5.5c — MCP tool invocation. server = manifest [mcp].servers[i].name;
      /// tool = the tool identifier exposed by the MCP server.
      McpCall { server: String, tool: String },
  }
  ```
  - The `capability_required_to_scopes` translator at `crates/maos-kernel-core/src/security/manifest.rs:395-419` is EXTENDED to produce `Scope::McpCall` variants from the new `[mcp].servers[i].allowed_tools[j]` declarations. The translator's structure mirrors the existing `Scope::ProviderInfer` production path.
  - The `Scope::McpCall` variant feeds the `drift.rs` observer + the `cap_policy::decision::Intent` mapping at `crates/maos-kernel-core/src/capability/mod.rs:52` (add a `Scope::McpCall { server, tool } => Intent::McpCall { server, tool }` arm).

### MCP — MANIFEST SECTION

- **NEW `[mcp]` manifest section** at `crates/maos-kernel-core/src/security/manifest.rs::McpSection`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[maos_attrs::i9_exempt(
      reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
  )]
  pub struct McpSection {
      #[doc = "Construct via [`McpSection::new`] to enforce uniqueness of server.name across the section."]
      pub servers: Vec<McpServerEntry>,
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission")]
  pub struct McpServerEntry {
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      pub name: String,
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      pub uri: String,
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      pub transport: McpTransportId,
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      #[serde(default)]
      pub fallback_transport: Option<McpTransportId>,
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      pub server_trust_tier: TrustTier,
      #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
      pub allowed_tools: Vec<String>,
  }
  ```
  Validation in the manifest validator:
  - `servers[i].name` empty OR duplicates another entry's name → `ManifestError::Toml(format!("mcp.servers[{i}].name '{name}' empty-or-duplicate"))`.
  - `servers[i].uri` empty → `ManifestError::Toml(format!("mcp.servers[{i}].uri must not be empty"))`.
  - `servers[i].transport` is an unrecognized value → TOML parse error from `#[serde]` rename (no manual check needed).
  - `servers[i].fallback_transport == Some(t)` where `t == servers[i].transport` → `ManifestError::Toml(format!("mcp.servers[{i}].fallback_transport must differ from primary"))`.
  - `servers[i].server_trust_tier == TrustTier::PublicVetted` → `ManifestError::Toml("mcp.servers[i].server_trust_tier 'public-vetted' deferred per FR37 to v2.5; allowed: local, org-internal, public-untrusted")`. (FR37 explicit exclusion.)
  - `servers[i].allowed_tools[j]` empty OR equals `"*"` → `ManifestError::Toml(format!("mcp.servers[{i}].allowed_tools[{j}] empty-or-glob — explicit tool names only at v0.5-α"))`.
  - Each `servers[i].transport` is consulted against the **strictest-of trust-tier floor** (Story 1b.3): `server_trust_tier = "local"` requires transport `stdio` OR `streamable_http` only; `server_trust_tier = "org-internal"` requires `streamable_http`; `server_trust_tier = "public-untrusted"` requires `streamable_http` AND admission-time ComplianceClaim verification (the FULL envelope path arrives at Story 5.5d; Story 5.5c uses the frozen schema from Story 1b.4 — verify the ComplianceClaim field shape parses but defer envelope validation to 5.5d).
  Manifest fixtures at `crates/maos-kernel-core/tests/fixtures/manifest/mcp/`:
  - `well-formed/single-stdio-server.toml`
  - `well-formed/streamable-http-with-stdio-fallback.toml`
  - `well-formed/three-tools-on-one-server.toml`
  - `malformed-rejected/empty-name.toml`
  - `malformed-rejected/duplicate-names.toml`
  - `malformed-rejected/glob-in-allowed-tools.toml`
  - `malformed-rejected/public-vetted-tier.toml`
  - `malformed-rejected/fallback-equals-primary.toml`

### ACP SERVER — substrate

- **NEW `crates/maos-acp/src/server.rs::AcpServer`** — long-running NDJSON-over-stdio server:
  ```rust
  /// ACP (Agent Communication Protocol) server — NDJSON over stdio.
  ///
  /// Per architecture §7.5 + appendix-a-cohort-prior-art-map.md (ACP convergent
  /// across openclaw / opencode / hermes; ratified by Zed).
  ///
  /// The server runs as a child of the host editor: editor spawns
  /// `maos-bin --acp` (or `MAOS_ONE_SHOT=acp-server`) with `stdin`/`stdout`
  /// piped; the server consumes NDJSON frames from `stdin` and emits NDJSON
  /// frames to `stdout`. `stderr` is reserved for human-readable diagnostics
  /// (visible in editor's "MAOS Output" channel).
  pub struct AcpServer {
      lifecycle: Arc<dyn LifecycleResolver>,    // Story 5.1
      halts:     Arc<dyn HaltResolver>,         // Story 4.1 + 3.3
      sessions:  Arc<Mutex<Vec<AcpSessionHandle>>>,
      notification_rx: Receiver<AcpFrameOut>,   // events from AcpEditorChannelImpl
  }

  pub struct AcpSessionHandle {
      pub session_id: SessionId,
      pub outbound: Sender<AcpFrameOut>,
      pub started_at_ns: u64,
  }

  impl AcpServer {
      pub fn new(
          lifecycle: Arc<dyn LifecycleResolver>,
          halts: Arc<dyn HaltResolver>,
      ) -> (Self, NotificationSender) { /* ... */ }

      /// Block on stdio, accept session frames, dispatch to lifecycle/halt
      /// resolvers, write replies to stdout. Returns when stdin EOFs.
      pub fn run(&mut self, stdin: impl Read, stdout: impl Write) -> Result<(), AcpError>;

      /// Snapshot of current session registry; consumed by
      /// `AcpEditorChannelImpl::dispatch` for fan-out.
      pub fn session_registry(&self) -> Arc<Mutex<Vec<AcpSessionHandle>>>;
  }
  ```
  - **Session state machine**: one session per connected editor; multiple sessions multiplex over the single stdio if the editor opens multiple sub-windows (per-session ID disambiguates).
  - **Frame parsing**: NDJSON one-frame-per-line; `serde_json::from_slice` per line; ill-formed frames return `AcpFrameOut::Error { code, message }` and the session stays alive (single bad frame does NOT terminate the session).
  - **JoinHandle self-prune** (Story 5.4 §1368): per-session task self-prunes on `session.end` OR on stdin EOF.
- **NEW `crates/maos-acp/src/frame.rs::AcpFrame*`** — the wire-protocol frame types:
  ```rust
  /// Tagged-union wire frame in the ACP NDJSON stream.
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
  #[serde(tag = "kind", rename_all = "snake_case")]
  pub enum AcpFrameIn {
      SessionStart { session_id: SessionId, editor_id: String, editor_version: String },
      SessionEnd { session_id: SessionId },
      LifecycleVerb { session_id: SessionId, decision_id: DecisionId, verb: LifecycleVerb, spirit_id: String },
      HaltResolve { session_id: SessionId, decision_id: DecisionId, halt_id: String, resolution: HaltResolutionKind, operator_note: Option<String> },
  }

  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
  #[serde(tag = "kind", rename_all = "snake_case")]
  pub enum AcpFrameOut {
      SessionReady { session_id: SessionId, supported_kinds: Vec<String> },
      SessionTerminated { session_id: SessionId, duration_ns: u64 },
      LifecycleReceipt { decision_id: DecisionId, spirit_pid: u32, verb: LifecycleVerb, timestamp_ns: u64, outcome: String, error: Option<AcpErrorBody> },
      HaltReceipt { decision_id: DecisionId, halt_id: String, outcome: String, timestamp_ns: u64 },
      NotificationDispatch { level: NotificationLevel, event: NotificationEvent },
      Error { code: i32, message: String, decision_id: Option<DecisionId> },
  }
  ```
  - Every `#[serde(tag = "kind")]` tagged-union frame is round-trip tested in `crates/maos-acp/src/frame.rs::tests`: serialize a known frame, deserialize, assert equality; deserialize hand-written NDJSON fixtures from `crates/maos-acp/tests/fixtures/wire/` to assert wire-stable shape.
- **NEW `crates/maos-acp/src/notification_channel.rs::AcpEditorChannelImpl`** — the closed seam:
  ```rust
  /// Replaces the Story 3.1 `AcpEditorChannel` stub at
  /// `crates/maos-director-surface/src/notification.rs:235-249`.
  ///
  /// Holds the session registry from `AcpServer`; on `dispatch`, fans out
  /// to every connected session's outbound channel. Per-session capacity
  /// 256 (broadcast-class drop-oldest); saturation emits a
  /// `cap_audit::record_drop()`.
  pub struct AcpEditorChannelImpl {
      sessions: Arc<Mutex<Vec<AcpSessionHandle>>>,
  }

  impl AcpEditorChannelImpl {
      pub fn new(sessions: Arc<Mutex<Vec<AcpSessionHandle>>>) -> Self { Self { sessions } }
  }

  impl NotificationChannel for AcpEditorChannelImpl {
      fn surface(&self) -> NotificationSurface { NotificationSurface::AcpEditor }

      fn dispatch(&self, event: &NotificationEvent, level: NotificationLevel) -> Result<(), NotificationError> {
          let frame = AcpFrameOut::NotificationDispatch { level, event: event.clone() };
          let mut delivered = 0usize;
          let sessions = self.sessions.lock().map_err(|e| NotificationError::Unavailable(format!("session lock poisoned: {e}")))?;
          for session in sessions.iter() {
              match session.outbound.try_send(frame.clone()) {
                  Ok(()) => delivered += 1,
                  Err(crossbeam_channel::TrySendError::Full(_)) => cap_audit::record_drop("acp_session_overflow"),
                  Err(crossbeam_channel::TrySendError::Disconnected(_)) => { /* session is dead — will be GC'd by next session.end */ }
              }
          }
          if delivered == 0 && !sessions.is_empty() { return Err(NotificationError::WriteFailed("all sessions full or disconnected".into())); }
          Ok(())
      }
  }
  ```
  - The Story 3.1 stub at `crates/maos-director-surface/src/notification.rs:235-249` is **DELETED** and replaced by a `pub use maos_acp::AcpEditorChannelImpl as AcpEditorChannel;` re-export — the symbol name `AcpEditorChannel` survives for backward-compatibility with the composition root + any existing tests that name the type. Document the symbol-rename via `pub use` in Dev Notes Decision Register.
  - The `unimplemented!()` macro at line 248 of the stub is **mechanically gone** after this story ships — verified by `grep -rn 'unimplemented!.*Story 5.5c' crates/` returning zero matches.
- **EXTENDED `crates/maos-acp/src/lib.rs`** — replaces the 9-line placeholder:
  ```rust
  #![forbid(unsafe_code)]

  //! `maos-acp` — Agent Communication Protocol server (NDJSON over stdio).
  //!
  //! Per architecture §7.5 + appendix-a-cohort-prior-art-map.md (convergent
  //! across openclaw / opencode / hermes; ratified by Zed).
  //!
  //! Consumer-facing surface: `AcpServer::new(lifecycle, halts).run(stdin, stdout)`.

  pub mod server;
  pub mod frame;
  pub mod notification_channel;

  pub use server::{AcpServer, AcpSessionHandle, AcpError};
  pub use frame::{AcpFrameIn, AcpFrameOut, SessionId, DecisionId};
  pub use notification_channel::AcpEditorChannelImpl;
  ```
- **NEW `crates/maos-acp/src/fixture_replay.rs::FixtureReplayAcpClient`** (test-only helper) — declarative editor simulator for the smoke arm and integration tests; mirrors `FixtureReplayProvider` pattern:
  ```rust
  pub struct FixtureReplayAcpClient {
      script: Mutex<VecDeque<AcpFrameIn>>,
      received: Mutex<Vec<AcpFrameOut>>,
  }
  impl FixtureReplayAcpClient {
      pub fn new(script: Vec<AcpFrameIn>) -> Self { /* ... */ }
      pub fn next_inbound(&self) -> Option<AcpFrameIn> { /* pop next */ }
      pub fn record_outbound(&self, frame: AcpFrameOut) { /* append */ }
      pub fn take_received(&self) -> Vec<AcpFrameOut> { /* drain */ }
  }
  ```

### COMPOSITION ROOT + SMOKE ARM + KNOWN-MODES + INTEGRATION TESTS

- **EXTENDED `crates/maos-bin/src/main.rs` composition root** — alongside the existing inference / sandbox / scheduler wiring, add the MCP + ACP wiring:
  ```rust
  // Story 5.5c — MCP client wiring (composition root)
  let mut mcp_transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>> = BTreeMap::new();
  let stdio_transport = Arc::new(StdioTransport::placeholder());  // real spawn happens per-server
  let sse_transport = Arc::new(SseTransport::new(Arc::clone(&io_arc), String::new())?);
  let http_transport = Arc::new(StreamableHttpTransport::new(Arc::clone(&io_arc), String::new())?);
  mcp_transports.insert(McpTransportId::Stdio, stdio_transport as Arc<dyn McpTransport>);
  mcp_transports.insert(McpTransportId::Sse, sse_transport as Arc<dyn McpTransport>);
  mcp_transports.insert(McpTransportId::StreamableHttp, http_transport as Arc<dyn McpTransport>);

  // Servers map populated lazily per-Spirit at admission time from manifest [mcp].servers
  let mcp_client = Arc::new(McpClient::new(mcp_transports, McpTransportId::StreamableHttp, BTreeMap::new())?);
  let mcp_adapter = Arc::new(McpClientAdapter::new(
      Arc::clone(&mcp_client) as Arc<dyn McpClient>,
      Arc::clone(&capability),
      Arc::clone(&transparency_log),
      Arc::clone(&telemetry),
  ));

  // Story 5.5c — ACP server wiring (composition root; opt-in via MAOS_ACP_ENABLE=1)
  if std::env::var("MAOS_ACP_ENABLE").as_deref() == Ok("1") {
      let (acp_server, _notification_sender) = AcpServer::new(
          Arc::clone(&lifecycle_resolver) as Arc<dyn LifecycleResolver>,
          Arc::clone(&halt_resolver) as Arc<dyn HaltResolver>,
      );
      let acp_channel = AcpEditorChannelImpl::new(acp_server.session_registry());
      notification_dispatcher.register(Box::new(acp_channel));
      eprintln!("maos: ACP server enabled — listening on stdio");
      // acp_server.run(stdin, stdout) is called only in MAOS_ONE_SHOT=acp-server mode
  }
  ```
- **NEW `MAOS_ONE_SHOT=smoke-mcp-acp-5` arm** at `crates/maos-bin/src/main.rs` — additive on the existing match block; mirrors `smoke-multi-provider-5` shape at `main.rs:2068-2157` (6 surfaces; one JSON line per surface; exit 0 on completion):
  - Step 1: `mcp_client_init` — register three FixtureReplayMcpServer instances (one per transport), assert `McpClient::registered_servers()` returns the expected names.
  - Step 2: `mcp_call` — issue `McpClient::call("test-server", "echo", {"msg":"hello"})`, assert response carries the expected content + attribution.
  - Step 3: `mcp_fallback` — register a FixtureReplayMcpServer that returns `McpTransportError::Transport(...)` on the primary, a successful response on the fallback; assert fallback was used and TL `intent` reflects the actual transport.
  - Step 4: `acp_session` — start a FixtureReplayAcpClient with a `session.start` → `lifecycle.verb=load` script; assert the server processed the verb and replied `lifecycle.receipt`.
  - Step 5: `acp_notification` — dispatch a `NotificationEvent::TaskAssigned` through the `NotificationDispatcher` with `AcpEditorChannelImpl` registered; assert the FixtureReplayAcpClient received a `notification.dispatch` frame.
  - Step 6: `acp_halt_resolve` — simulate `halt.resolve` from the FixtureReplayAcpClient; assert `HaltResolver::resolve_halt` was invoked.
- **NEW `MAOS_ONE_SHOT=acp-server` arm** — long-running ACP server arm consumed by the editor integration tests. Spawns `AcpServer::run(stdin, stdout)` and blocks on stdin EOF; documented in `--help` output.
- **EXTENDED known-modes list at `crates/maos-bin/src/main.rs:2163`** — append `smoke-mcp-acp-5` AND `acp-server` to the comma-separated mode list.
- **NEW `crates/maos-bin/tests/smoke_mcp_acp_test.rs`** — test driver invoking the smoke arm via `Command::new(maos_bin).env("MAOS_ONE_SHOT", "smoke-mcp-acp-5")`; asserts exit code 0; asserts stdout contains 6 JSON lines each with the expected `step` and `surface` fields. Mirrors `smoke_multi_provider_test.rs` shape exactly.
- **NEW `tests/integration/acp-editors/zed/zed_acp_lifecycle_test.rs`** + **`tests/integration/acp-editors/vscode/vscode_acp_lifecycle_test.rs`** — editor-specific NDJSON conversation scripts (editor-plugin code is NOT in this story; the tests use scripted NDJSON to simulate the editor side). Each test:
  - Spawns `maos-bin` with `MAOS_ONE_SHOT=acp-server`.
  - Writes a scripted sequence: `session.start` → `lifecycle.verb=load,spirit_id=hello-spirit` → waits for `lifecycle.receipt` with `outcome=ok` → waits for `notification.dispatch` with `event_kind=TaskAssigned` (within 2s) → writes `halt.resolve` → waits for `halt.receipt` → writes `session.end` → waits for `session.terminated`.
  - Asserts the FULL conversation completes within a 5s wall-clock budget.
  - Asserts exit code 0.
  - The `zed` and `vscode` test files differ ONLY in the `editor_id` field on the `session.start` frame; the wire-protocol contract is editor-agnostic at v0.5-α (editor-specific extensions arrive at v1.0+).

### KERNEL-API SURFACE GATE + XTASK INTEGRATION

- **EXTENDED `xtask/kernel-api-classes.toml`** — add the three new adapter classifications (per architecture §4.0.7 data-movement class):
  ```toml
  "maos_kernel_core::api::mcp::McpClientAdapter"   = "data-movement"
  "maos_kernel_core::mcp::McpClientAdapter"        = "data-movement"
  "maos_kernel_core::api::mcp::maos_domain::ports::McpClientPort" = "data-movement"
  ```
  And the port-trait propagation:
  ```toml
  "maos_kernel_core::mcp::maos_domain::ports::McpClientPort" = "data-movement"
  ```
- **NEW `crates/maos-kernel-core/src/api.rs::mcp` re-export module** — `pub use crate::mcp::{McpClientAdapter}; pub use maos_domain::ports::McpClientPort;`
- **Verify `cargo run -p xtask -- check-service-boundary` PASSES** after the additions; verify no `other`-classified symbol leaks; verify `cargo run -p xtask -- check-fr47` continues to PASS with `fr47-allowlist.toml` empty (no MCP/ACP-protocol library is added; verify via `cargo tree | grep -E 'jsonrpc|mcp-client|acp-protocol|rust-mcp'` returns empty).
- **Verify `cargo run -p xtask -- check-pub-field-constructors` PASSES** — every new pub field on `McpSection`, `McpServerEntry`, `AcpSessionHandle`, `AcpFrameIn`-variants, `AcpFrameOut`-variants, `McpRequest`, `McpResponse`, `McpAttribution`, etc., carries the `#[doc = "Construct via ::new ..."]` annotation matched by an `impl ::new` constructor.

### WIRE-PROTOCOL FUZZ SEED CORPUS

- **NEW `crates/maos-mcp/fuzz/`** — `cargo fuzz` setup with at minimum:
  - `fuzz_targets/mcp_request_parse.rs` — `mcp::McpRequest` deserialization.
  - `fuzz_targets/mcp_response_parse.rs` — `mcp::McpResponse` deserialization.
  - `corpus/seeds/` — 50+ seed inputs per target (well-formed + boundary + malformed). Per the §5.2 tiered cadence ladder: T1 10-min per-commit is wired in `.github/workflows/discipline.yml` later (Story 10.2); Story 5.5c authors the corpus + seeds.
- **NEW `crates/maos-acp/fuzz/`** — same shape; targets `acp::AcpFrameIn` deserialization.

## What this story IS NOT

- **NOT a real Zed extension or VSCode extension.** The editor-plugin code itself is out of scope; the integration tests use scripted NDJSON to simulate the editor side. Editor-plugin authoring is **deferred to v1.0** as an external-author validation (logged in `deferred-work.md`).
- **NOT a full MCP server in MAOS.** Story 5.5d ships the MCP-Streamable-HTTP-SERVER side (the Spirit registry). Story 5.5c only ships the CLIENT side (consume MCP tool servers from inside Spirits).
- **NOT MCP-streaming-response handling beyond Streamable HTTP single-shot.** Long-lived streaming MCP responses (where a tool emits multiple chunks via SSE inside one Streamable HTTP response) are supported by the wire-level transport — but the consumer-facing `McpClient::call` returns a single `McpResponse` aggregating all chunks. True multi-chunk streaming APIs (`stream_call` returning an iterator) are deferred to Epic 6 Story 6.4 or later.
- **NOT MCP-protocol-library import.** Per FR47, the MCP wire protocol (JSON-RPC 2.0 framing) is direct-implemented in MAOS code — NO `rust-mcp` / `jsonrpc-core` / `mcp-protocol` crate is added to the workspace. Verified by `cargo tree | grep -E 'mcp|jsonrpc'` returning empty.
- **NOT ACP-protocol-library import.** Same FR47 commitment for ACP NDJSON framing — direct-implemented; verified by `cargo tree | grep -E 'acp|nd-?json'` returning empty (line-oriented JSON is a serde + `std::io::BufRead` pattern, not a separate crate).
- **NOT a runtime-pluggable MCP transport.** The three transports are compile-time-known; adding a fourth requires a new ADR + a code change to register it in the composition root. Operator-runtime selection between the three is supported (manifest `[mcp].servers[i].transport`); operator-runtime addition of a fourth transport is NOT.
- **NOT MCP server discovery via DNS-SD, mDNS, or any other automatic mechanism.** Every MCP server consumed by a Spirit is **explicitly declared** in the Spirit's manifest `[mcp].servers` table. Operators name the servers; the substrate does not discover them. This is the same operator-explicit pattern Story 5.5b's `[providers]` section enforces.
- **NOT MCP server-side ComplianceClaim envelope verification beyond schema parsing.** The full envelope-verification admission path is Story 5.5d's concern; Story 5.5c only **parses** the `server_trust_tier` field and refuses unknown values.
- **NOT structural air-gap egress validation.** Story 5.5c's MCP-Streamable-HTTP transport routes through `IoSubsystemPort`; the Story 9.4 `unshare --net` structural test validates zero packets leave. Story 5.5c authors the **observability-grade** check (smoke arm step 6 verifies fixture-replay mode has zero IO journal entries); the structural validation arrives at v1.0 (Story 9.4).
- **NOT the IPC-latency measurement gate for the kernel-side MCP path.** Story 5.5e's §13.1 J1+J4 measurement does not invoke MCP. If §13.1 triggers `unlock-rust-inproc-in-v0.5`, MCP/ACP transports remain unaffected.
- **NOT mTLS for MCP-Streamable-HTTP at v0.5-α.** Operator-deployed MCP servers using Streamable HTTP run over HTTPS (TLS 1.3); mTLS-protected MCP endpoints are an Epic 7+ extension. The `IoSubsystemPort::http_post` adapter at v0.5-α terminates TLS — mTLS client-cert presentation is added when bilateral A2A's mTLS infrastructure (Story 6.3) lands.
- **NOT ACP audit-query frame kinds (`audit.query`, `audit.posture-delta`, `audit.subject-access`, `audit.sealed-export`).** Those frame kinds arrive at Epic 9 Story 9.1. Story 5.5c's ACP server ships the 4 baseline frame kinds (`session.start/end`, `lifecycle.verb`, `halt.resolve`, `notification.dispatch`) only.
- **NOT ACP skill-authoring frame kinds (`skill.propose`, `skill.review`, `skill.commit`).** Those arrive at Epic 7 Story 7.4.

## Acceptance Criteria

### AC1 — MCP client three transports + `McpClient::call` surface + per-server transport selection + fallback (epic AC1)

**Given** the EXISTING `IoSubsystemPort::http_post` / `http_get` HTTP transport at `crates/maos-domain/src/ports/io_subsystem.rs:15-34`, the EXISTING `#[non_exhaustive]` `Scope` enum at `crates/maos-domain/src/invariants/i1.rs:60` (preserves 13 existing variants), the EXISTING `CapabilityRegistry` + `TransparencyLogPort` infrastructure, and the EXISTING composition-root assembly pattern from Story 5.5b at `crates/maos-bin/src/main.rs:384-405`,

**When** Story 5.5c lands (a) the NEW `McpTransport` trait + three concrete impls (`StdioTransport`, `SseTransport`, `StreamableHttpTransport`) at `crates/maos-mcp/src/transport/{stdio,sse,streamable_http}.rs`, (b) the NEW `McpClient` at `crates/maos-mcp/src/client.rs` with `BTreeMap<McpTransportId, Arc<dyn McpTransport>>` + `BTreeMap<String, McpServerEntry>` + `call(server, tool, args)` API, (c) the NEW `FixtureReplayMcpServer` test helper at `crates/maos-mcp/src/fixture_replay.rs`, (d) the additive `Scope::McpCall { server, tool }` variant on `crates/maos-domain/src/invariants/i1.rs::Scope`,

**Then** each transport handles its own framing correctly:

- **StdioTransport** spawns the configured subprocess, exchanges NDJSON frames over stdin/stdout, and self-prunes the reader JoinHandle on EOF; integration test `crates/maos-mcp/tests/stdio_transport_test.rs` spawns a small `cat`-based echo program, exchanges 5 round-trip frames, asserts all 5 round-trip correctly.
- **StreamableHttpTransport** wraps `IoSubsystemPort::http_post` with `Content-Type: application/json` + `Accept: application/json, text/event-stream` headers; pure-function `build_streamable_http_request_body(req) -> Vec<u8>` + `parse_streamable_http_response(body, content_type) -> Result<McpResponse, McpTransportError>` are tested against canned fixtures at `crates/maos-mcp/tests/fixtures/streamable_http_*.json` per the Story 5.5b precedent.
- **SseTransport** wraps `IoSubsystemPort::http_get` with `Accept: text/event-stream` header; SSE `data: {...}\n\n` framing is parsed via a small pure-function `parse_sse_event(line) -> Option<SseEvent>` tested against canned SSE-stream fixtures.

**And** `McpClient::call(server_name, tool, args)`:

1. Looks up the server entry by `server_name`; returns `McpError::UnknownServer` if missing.
2. Looks up the primary transport via `transports.get(&entry.transport)`; returns `McpError::Transport(...)` if the transport ID is not registered (cannot happen for the 3 known transports but covers operator-policy misconfiguration).
3. Invokes `transport.invoke(McpRequest { server, tool, args })`; on `Ok(resp)` returns it; on `Err(McpTransportError::Transport(_))` walks `entry.fallback_transport` if present (single-level only; no chained fallback at v0.5-α).
4. Fallback exhaustion returns the LAST `McpTransportError` (NOT a synthetic aggregate; preserves the actual failure details for the operator).
5. Non-retriable errors (`ServerError { code, message }`, decode failures) short-circuit immediately — do NOT silently rebroadcast to fallback.

**And** integration test `crates/maos-mcp/tests/mcp_client_routing.rs` (NEW) covers:

- `call_routes_to_per_server_transport` — register `server-A` with `streamable_http` + `server-B` with `stdio`; assert `call("server-A", ...)` uses HTTP transport and `call("server-B", ...)` uses stdio.
- `call_walks_fallback_on_transport_error` — register `server-A` with primary `streamable_http` (mocked to return `Transport`) + fallback `stdio` (returns Ok); assert response carries the stdio transport's attribution.
- `call_does_not_walk_fallback_on_server_error` — primary returns `ServerError { code: -32601, message: "Method not found" }`; assert short-circuit; fallback NOT invoked.
- `call_returns_unknown_server_for_unregistered` — `McpError::UnknownServer`.
- `call_with_unconfigured_transport_returns_transport_error` — register a server with `transport: Stdio` but the stdio transport NOT registered in the transports map (operator misconfiguration); assert clear error message naming the unconfigured transport.

**And** the `Scope::McpCall { server, tool }` variant is round-trip serde-tested (`crates/maos-domain/src/invariants/i1.rs::tests`) and the existing `Scope` enum variant count gate (if such a gate exists; verify HEAD-current via grep — `xtask check-scope-coverage` or similar; if absent, ADD a one-line check that the new variant count matches an expected number).

**And** `cargo run -p xtask -- check-fr47` continues to PASS with `fr47-allowlist.toml` empty. **No MCP wire-protocol library is added to the workspace** — verify via `cargo tree | grep -E 'jsonrpc|mcp-client|mcp-protocol|rust-mcp'` returning empty output.

---

### AC2 — Kernel-side `McpClientAdapter` + capability mediation + Transparency-Log emission + `FrameKind::McpInvocation = 10` (epic AC1, FR47 mediation invariant)

**Given** the EXISTING `InferencePortAdapter` capability-mediation pattern at `crates/maos-kernel-core/src/inference/mod.rs:35-146`, the EXISTING `CapabilityRegistry::policy().inner().load_full().token_scopes` lookup path at the same file, the EXISTING `TransparencyLogPort::record(TransparencyLogRow { frame_kind, ... })` path, and the EXISTING `FrameKind` enum at `crates/maos-domain/src/frame.rs`,

**When** Story 5.5c lands (a) the NEW `McpClientAdapter` at `crates/maos-kernel-core/src/mcp/mod.rs` (full struct + `McpClientPort` impl in §What this story IS), (b) the additive `FrameKind::McpInvocation = 10` discriminant on the existing `#[repr(u8)]` `FrameKind` enum (verify HEAD-current max + 1 at story-open; the value `10` is illustrative — confirm against HEAD), (c) the additive `Scope::McpCall { server, tool }` variant from AC1, (d) the new `intent` string format `format!("mcp:{server}/{tool}")` consumed by the TL emit path,

**Then** every `McpClientAdapter::call(token, server, tool, args)` invocation:

1. Verifies `token.scope == Scope::McpCall { server: <server>, tool: <tool> }` via `CapabilityRegistry::policy().inner().token_scopes.get(&token.token_id).iter().any(|s| matches!(s, Scope::McpCall { server: s, tool: t } if s == server && t == tool))`; on miss, returns `McpError::CapabilityDenied { server, tool }` WITHOUT invoking the underlying `McpClient::call`.
2. Reads `start_ns = monotonic_now_ns()` (NEVER `wall_clock_now_ns()` per Story 5.4 §1366).
3. Invokes `self.client.call(server, tool, args.clone())`.
4. Reads `end_ns = monotonic_now_ns()`.
5. Emits ONE `TransparencyLogRow { frame_kind: FrameKind::McpInvocation, spirit_pid: token.spirit_pid, timestamp_ns: start_ns, intent: format!("mcp:{server}/{tool}"), payload_hash: blake3_hash(&serde_json::to_vec(&args).map_err(McpError::Encode)?), duration_ns: Some(end_ns - start_ns), outcome: if response.is_error { "server_error" } else { "ok" }.into(), .. }` to the Transparency Log via the existing `TransparencyLogPort::record(...)` path.
6. Emits a telemetry round-trip via the existing `TelemetryStreamPort::emit_round_trip(spirit_pid, "mcp.call", end_ns - start_ns)`.

**And** unit tests in `crates/maos-kernel-core/src/mcp/mod.rs::tests` (mirroring `inference/mod.rs::tests` structure):

- `capability_denied_returns_capability_denied` — no token scope present; assert `McpError::CapabilityDenied`; assert NO `FrameKind::McpInvocation` TL row is emitted.
- `mock_client_round_trip_logs_mcp_invocation` — happy path; assert ONE `FrameKind::McpInvocation` row appears in the TL with the expected `intent` + `spirit_pid` + `duration_ns ≥ 1`.
- `server_error_response_logs_outcome_server_error` — `McpResponse { is_error: true, .. }`; assert TL `outcome` field is `"server_error"`.
- `serde_error_on_args_encode_returns_encode` — args is `serde_json::Value::Null` containing a non-serializable value (e.g., a `f64::NAN` after some serializer rejects); assert `McpError::Encode(_)`; assert NO TL row emitted.
- `monotonic_now_ns_used_for_timestamp` — assert TL row's `timestamp_ns` is monotonically non-decreasing across two consecutive calls.

**And** the `intent` string is grep-readable for operator audit — given a TL with `intent = "mcp:loom-lite/recall"`, `maosctl audit query --intent-prefix mcp:` (the surface arrives at Story 9.1; v0.5-α the substrate just emits the row) returns the MCP-invocation rows.

**And** the new adapter's symbols are registered in `xtask/kernel-api-classes.toml` per AC5 (`data-movement` class); `cargo run -p xtask -- check-service-boundary` PASSES.

---

### AC3 — ACP server NDJSON-over-stdio + session-oriented protocol + lifecycle-verb forwarding + halt-resolve forwarding (epic AC2)

**Given** the EXISTING `LifecycleResolver` trait at `crates/maos-domain/src/lifecycle.rs:133-148`, the EXISTING `HaltResolver` trait at `crates/maos-domain/src/halt.rs::HaltResolver`, the EXISTING `KernelLifecycleResolver` + `KernelHaltResolver` implementations in `maos-kernel-core`, and the EXISTING `NotificationDispatcher` at `crates/maos-director-surface/src/notification.rs:49-82`,

**When** Story 5.5c lands (a) the NEW `AcpServer` at `crates/maos-acp/src/server.rs` (full struct + `run(stdin, stdout)` API in §What this story IS), (b) the NEW `AcpFrameIn` / `AcpFrameOut` types at `crates/maos-acp/src/frame.rs` with `#[serde(tag = "kind", rename_all = "snake_case")]` tagged-union shapes, (c) the NEW `FixtureReplayAcpClient` test helper at `crates/maos-acp/src/fixture_replay.rs`, (d) the `MAOS_ONE_SHOT=acp-server` long-running arm at `crates/maos-bin/src/main.rs`,

**Then** when an editor opens the ACP session by sending `{"kind":"session_start","session_id":"<ulid>","editor_id":"zed","editor_version":"0.142.0"}\n`:

1. The server parses the frame via `serde_json::from_slice` (one frame per stdin line).
2. The server replies with `{"kind":"session_ready","session_id":"<ulid>","supported_kinds":["lifecycle_verb","halt_resolve","session_end"]}\n` to stdout.
3. The server appends a new `AcpSessionHandle { session_id, outbound: <bounded-256-channel>, started_at_ns: monotonic_now_ns() }` to its session registry.

**And** when the editor sends `{"kind":"lifecycle_verb","session_id":"<ulid>","decision_id":"<ulid>","verb":"load","spirit_id":"hello-spirit"}\n`:

1. The server invokes `lifecycle.resolve_verb("hello-spirit", LifecycleVerb::Load)`.
2. On success, replies `{"kind":"lifecycle_receipt","decision_id":"<ulid>","spirit_pid":<u32>,"verb":"load","timestamp_ns":<u64>,"outcome":"ok","error":null}\n`.
3. On error, replies `{"kind":"lifecycle_receipt","decision_id":"<ulid>","spirit_pid":0,"verb":"load","timestamp_ns":<u64>,"outcome":"error","error":{"code":<i32>,"message":"<msg>"}}\n`.

**And** when the editor sends `{"kind":"halt_resolve","session_id":"<ulid>","decision_id":"<ulid>","halt_id":"<id>","resolution":"approve","operator_note":"verified externally"}\n`:

1. The server invokes `halts.resolve_halt("<id>", HaltResolutionKind::Approve)` with the operator note threaded through (the existing `HaltResolver` surface supports the note field; if not, add the field as additive on `#[non_exhaustive]` per the existing pattern).
2. Replies `{"kind":"halt_receipt","decision_id":"<ulid>","halt_id":"<id>","outcome":"resolved","timestamp_ns":<u64>}\n`.

**And** when the editor sends `{"kind":"session_end","session_id":"<ulid>"}\n`:

1. The server replies `{"kind":"session_terminated","session_id":"<ulid>","duration_ns":<u64>}\n`.
2. The session task self-prunes its JoinHandle.
3. The session registry removes the corresponding `AcpSessionHandle`.

**And** when stdin EOFs without an explicit `session_end`, the server treats it as an implicit `session_end` for ALL sessions opened by that stdio pair, fans out `session_terminated` replies (if stdout is still writable), and exits cleanly with code 0.

**And** an ill-formed frame on the wire (e.g., `{"kind":"unknown_kind"}` or `{not json}`) returns `{"kind":"error","code":-32600,"message":"<descriptive>","decision_id":null}\n` and the session STAYS ALIVE (single bad frame does NOT terminate the session — robust editor experience).

**And** unit tests at `crates/maos-acp/src/server.rs::tests`:

- `session_start_replies_session_ready` — happy path.
- `lifecycle_verb_forwards_to_resolver` — mock `LifecycleResolver`; assert the verb is forwarded with the exact `spirit_id`.
- `halt_resolve_forwards_to_resolver` — mock `HaltResolver`; assert the halt_id + resolution are forwarded.
- `unknown_frame_kind_replies_error_session_stays_alive` — send `{"kind":"unknown"}`; assert error reply; assert subsequent `session_end` still works.
- `stdin_eof_implicit_session_end` — close stdin pipe; assert clean shutdown.

**And** integration test at `crates/maos-acp/tests/acp_e2e_test.rs` (NEW) exercises the FULL scripted conversation (session_start → lifecycle_verb → halt_resolve → session_end) against the real `AcpServer` with mock resolvers; asserts every frame round-trips correctly within a 1s wall-clock budget.

---

### AC4 — Kernel-rendered notification surface — `AcpEditorChannelImpl` closes the Story 3.1 stub + fan-out to all editor sessions (epic AC2, architecture §7.4 invariant)

**Given** the EXISTING `NotificationChannel` trait + `NotificationDispatcher` at `crates/maos-director-surface/src/notification.rs`, the EXISTING `AcpEditorChannel` STUB at `crates/maos-director-surface/src/notification.rs:235-249` (the `unimplemented!("Story 5.5c — ACP server notification channel")` macro that this story closes), the EXISTING 4 `NotificationEvent` variants (`TaskAssigned`, `ApprovalPrompt`, `Halt`, `AnomalyFlagged`), and the AC3 `AcpServer` session registry,

**When** Story 5.5c lands the NEW `AcpEditorChannelImpl` at `crates/maos-acp/src/notification_channel.rs` (full impl in §What this story IS) AND DELETES the `AcpEditorChannel` stub at `crates/maos-director-surface/src/notification.rs:235-249`, replacing it with `pub use maos_acp::AcpEditorChannelImpl as AcpEditorChannel;` re-export,

**Then** `grep -rn 'unimplemented!.*Story 5.5c' crates/` returns ZERO matches after the story ships. The seam is **mechanically closed**.

**And** when the composition root (`crates/maos-bin/src/main.rs`) registers the channel via `notification_dispatcher.register(Box::new(AcpEditorChannelImpl::new(acp_server.session_registry())))`, every subsequent `NotificationDispatcher::dispatch(event, level)` call:

1. Constructs an `AcpFrameOut::NotificationDispatch { level, event: event.clone() }` frame.
2. Walks the session registry's `Vec<AcpSessionHandle>` and `try_send`s the frame to EACH session's outbound channel (per-session bounded queue capacity 256; drop-oldest on overflow per the `telemetry.event` precedent at architecture §7.1.1).
3. On any session's queue saturation, emits `cap_audit::record_drop("acp_session_overflow")` (per Story 5.4 §A4 ADR-030 audit-channel discipline) and CONTINUES to the next session — one slow editor does NOT block others.
4. Returns `Ok(())` if ANY session received the frame; returns `Err(NotificationError::WriteFailed("all sessions full or disconnected"))` if NO session received it.

**And** unit tests at `crates/maos-acp/src/notification_channel.rs::tests`:

- `dispatch_with_zero_sessions_returns_ok` — empty session registry; dispatch returns Ok (matches `NotificationDispatcher::dispatch` semantics for zero-channel case at `crates/maos-director-surface/src/notification.rs:296-310`).
- `dispatch_with_one_session_delivers_frame` — register one session with a bounded channel; dispatch a `TaskAssigned` event; assert the channel's receiver got an `AcpFrameOut::NotificationDispatch { event: TaskAssigned { .. }, .. }` frame.
- `dispatch_with_full_session_emits_audit_drop` — register one session with a 1-slot bounded channel; pre-fill it; dispatch; assert `cap_audit::record_drop("acp_session_overflow")` was called.
- `dispatch_fans_out_to_three_sessions` — register three sessions; dispatch; assert all three got the frame.
- `dispatch_continues_when_one_session_disconnects` — register two sessions, drop one's receiver; dispatch; assert the surviving session got the frame and dispatch returned Ok.

**And** the §7.4 architecture invariant ("a Spirit cannot bypass the user's notification policy by emitting a different kind of event") is structurally preserved — verify via static analysis: `grep -rn 'AcpFrameOut::NotificationDispatch' crates/` returns ONLY matches under `maos-acp` and `maos-director-surface` (no `maos-domain` / `maos-kernel-core` direct emission paths; the only writer is `AcpEditorChannelImpl::dispatch`).

---

### AC5 — Editor integration tests (Zed + VSCode) + full lifecycle conversation + 5s wall-clock budget (epic AC3)

**Given** the EXISTING `crates/maos-bin/tests/smoke_*_test.rs` integration-test pattern + the AC3 `MAOS_ONE_SHOT=acp-server` long-running arm + the AC4 notification-dispatch path,

**When** Story 5.5c lands the NEW `tests/integration/acp-editors/zed/zed_acp_lifecycle_test.rs` AND `tests/integration/acp-editors/vscode/vscode_acp_lifecycle_test.rs` integration tests (each consuming the `MAOS_ONE_SHOT=acp-server` arm) AND the NEW `tests/integration/acp-editors/README.md` documenting the editor-plugin-author wire-schema reference,

**Then** each test spawns `maos-bin` via `Command::new(maos_bin).env("MAOS_ONE_SHOT", "acp-server").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()` (stderr inherits to surface diagnostics).

**And** the test writes a scripted NDJSON sequence to the binary's stdin in this order:

1. `{"kind":"session_start","session_id":"01J...","editor_id":"<zed|vscode>","editor_version":"<version>"}\n`
2. Reads back from stdout until `{"kind":"session_ready",...}` appears (within 1s); asserts the frame's `session_id` matches.
3. `{"kind":"lifecycle_verb","session_id":"01J...","decision_id":"01K...","verb":"load","spirit_id":"hello-spirit"}\n`
4. Reads back until `{"kind":"lifecycle_receipt",...}` with `outcome:"ok"` (within 2s); asserts `spirit_pid > 0`.
5. Reads back until `{"kind":"notification_dispatch","level":"immediate","event":{"TaskAssigned": ...}}` (within 2s) — this asserts the load fired a `task.assign` notification that fanned out to the editor.
6. `{"kind":"halt_resolve","session_id":"01J...","decision_id":"01L...","halt_id":"halt-test-001","resolution":"approve","operator_note":null}\n`
7. Reads back until `{"kind":"halt_receipt",...}` with `outcome:"resolved"` (within 1s).
8. `{"kind":"session_end","session_id":"01J..."}\n`
9. Reads back until `{"kind":"session_terminated",...}` (within 500ms).

**And** the test asserts:

- The FULL conversation completes within a 5s wall-clock budget (the JoinHandle self-prune + session-task tear-down completes synchronously on session_end).
- The binary exits with code 0.
- No stderr panics or assertion failures (stderr is inspected if the test fails to aid debugging).

**And** the `zed` and `vscode` test files differ ONLY in the `editor_id` field on the `session_start` frame; the wire-protocol contract is editor-agnostic at v0.5-α.

**And** the integration test exists at the right path so the existing CI matrix picks it up (verify by running `cargo test --test zed_acp_lifecycle_test` and `cargo test --test vscode_acp_lifecycle_test` locally; both pass).

**And** the `tests/integration/acp-editors/README.md` documents the wire schema for editor-plugin authors — frame shapes, expected ordering, error-handling conventions, the v0.5-α `supported_kinds` set, and the path-to-v1.0 deferred frame kinds (`audit.*`, `skill.*`).

---

### AC6 — Smoke arm `MAOS_ONE_SHOT=smoke-mcp-acp-5` + known-modes list extension + Story 0.2 kernel-API surface preservation (epic AC4)

**Given** the EXISTING `MAOS_ONE_SHOT` dispatch mechanism at `crates/maos-bin/src/main.rs:449-455` (env-var-gated mode selection), the EXISTING known-modes list at `main.rs:2163`, the EXISTING `smoke-multi-provider-5` arm at `main.rs:2068-2157` (the canonical multi-surface smoke-arm shape this story mirrors), the EXISTING `xtask/kernel-api-classes.toml` classification table, and the EXISTING `xtask check-service-boundary` gate at `xtask/src/check_service_boundary.rs`,

**When** Story 5.5c lands (a) the NEW `MAOS_ONE_SHOT=smoke-mcp-acp-5` arm walking 6 numbered JSON-line surfaces (full surface list in §What this story IS), (b) the NEW `MAOS_ONE_SHOT=acp-server` long-running arm consumed by AC5, (c) the extended known-modes list at `main.rs:2163` including BOTH `smoke-mcp-acp-5` AND `acp-server`, (d) the three new `kernel-api-classes.toml` rows for `McpClientAdapter` + `McpClientPort`,

**Then** when an evaluator runs `MAOS_ONE_SHOT=smoke-mcp-acp-5 cargo run -p maos-bin` they observe stdout containing EXACTLY these 6 JSON lines (in order):

```jsonc
{"step":1,"surface":"mcp_client_init","transports":["stdio","sse","streamable_http"],"default":"streamable_http"}
{"step":2,"surface":"mcp_call","outcome":"ok","server":"test-server","tool":"echo"}
{"step":3,"surface":"mcp_fallback","outcome":"ok","primary":"streamable_http","fallback_used":"stdio"}
{"step":4,"surface":"acp_session","outcome":"ok","verb":"load"}
{"step":5,"surface":"acp_notification","outcome":"ok","level":"immediate","event_kind":"TaskAssigned"}
{"step":6,"surface":"acp_halt_resolve","outcome":"ok","resolution":"approve"}
```

**And** the binary exits with code 0 after step 6.

**And** `crates/maos-bin/tests/smoke_mcp_acp_test.rs` (NEW) invokes the arm via `Command::new(maos_bin).env("MAOS_ONE_SHOT", "smoke-mcp-acp-5")`, asserts exit code 0, parses stdout into 6 JSON lines, asserts each line has the expected `step` + `surface` keys + `outcome:"ok"` semantics. The test runs on every platform (no platform-specific dependencies — fixture replay is host-agnostic).

**And** an evaluator running `MAOS_ONE_SHOT=unknown cargo run -p maos-bin` gets the error message at `main.rs:2163` listing the full updated known-modes set INCLUDING `smoke-mcp-acp-5` and `acp-server`.

**And** `cargo run -p xtask -- check-service-boundary --kernel-api-classes xtask/kernel-api-classes.toml` PASSES with the three new rows:

- `"maos_kernel_core::mcp::McpClientAdapter" = "data-movement"`
- `"maos_kernel_core::api::mcp::McpClientAdapter" = "data-movement"` (if the `api::mcp` re-export module is exposed; otherwise omit)
- `"maos_kernel_core::mcp::maos_domain::ports::McpClientPort" = "data-movement"`

**And** `cargo run -p xtask -- check-fr47` PASSES with `fr47-allowlist.toml` empty. Verify via `cargo tree | grep -E 'jsonrpc|mcp-client|mcp-protocol|rust-mcp|acp-protocol'` returning empty.

**And** `cargo run -p xtask -- check-pub-field-constructors` PASSES — every new pub field on `McpSection`, `McpServerEntry`, `McpRequest`, `McpResponse`, `McpAttribution`, `AcpFrameIn`-variants, `AcpFrameOut`-variants, `AcpSessionHandle` carries the `#[doc = "Construct via ::new ..."]` annotation matched by an `impl ::new` constructor (use `pub-field-constructor-allowlist.toml` only if a structural exception is genuinely required — none are expected for Story 5.5c).

**And** the FR47 vendor-SDK denylist gate proves the v0.5-α MCP-protocol-library-free claim — `cargo tree | grep -E 'mcp|jsonrpc|acp|nd-?json'` returns empty (line-oriented JSON is a serde + `std::io::BufRead` pattern; JSON-RPC framing is direct-implemented; no external crate is required).

---

## Tasks / Subtasks

- [x] **Task 1 (AC1) — MCP client domain port + types**
  - [x] Add `crates/maos-domain/src/ports/mcp.rs` with `McpClientPort` trait + `McpRequest` + `McpResponse` + `McpAttribution` + `McpTransportId` enum + `#[serde(rename_all = "snake_case")]` discipline + full pub-field-constructor annotations + `::new` constructors.
  - [x] Re-export from `crates/maos-domain/src/ports/mod.rs`.
  - [x] Round-trip serde tests for each new struct.

- [x] **Task 2 (AC1) — `Scope::McpCall` additive variant**
  - [x] Add `Scope::McpCall { server: String, tool: String }` to `crates/maos-domain/src/invariants/i1.rs::Scope` (preserves `#[non_exhaustive]`).
  - [x] Round-trip serde test for the new variant.
  - [x] Extend `capabilities_required_to_scopes` at `crates/maos-kernel-core/src/security/manifest.rs:395-419` to produce `Scope::McpCall` variants from `[mcp].servers[i].allowed_tools[j]`.
  - [x] Extend `crates/maos-kernel-core/src/capability/mod.rs:52` `Intent` mapping to include the McpCall arm.
  - [x] Extend `drift.rs` observer with an analogous McpCall arm.

- [x] **Task 3 (AC1) — MCP transport trait + three concrete impls**
  - [x] Add `crates/maos-mcp/src/transport/mod.rs::McpTransport` trait + `McpTransportError`.
  - [x] Add `crates/maos-mcp/src/transport/stdio.rs::StdioTransport` (subprocess + NDJSON over child stdin/stdout + JoinHandle self-prune).
  - [x] Add `crates/maos-mcp/src/transport/sse.rs::SseTransport` (layered atop `IoSubsystemPort::http_get`).
  - [x] Add `crates/maos-mcp/src/transport/streamable_http.rs::StreamableHttpTransport` (layered atop `IoSubsystemPort::http_post`).
  - [x] Pure-function `build_*_request_body` + `parse_*_response` for each transport with fixture-tested unit coverage at `crates/maos-mcp/tests/fixtures/`.
  - [x] Integration test `crates/maos-mcp/tests/stdio_transport_test.rs` exercising a `cat`-based echo subprocess.

- [x] **Task 4 (AC1) — `McpClient` orchestrator + per-server routing + fallback**
  - [x] Add `crates/maos-mcp/src/client.rs::McpClient` with `BTreeMap` holders + `::new` + `call` + `registered_servers`.
  - [x] Add `crates/maos-mcp/src/lib.rs` replacing placeholder; add `fixture_replay` feature flag to `crates/maos-mcp/Cargo.toml`.
  - [x] Inline tests covering the 4 routing scenarios from AC1.
  - [x] Integration test `crates/maos-mcp/tests/mcp_client_routing.rs` covering the 5 multi-transport scenarios from AC1.

- [x] **Task 5 (AC1) — `FixtureReplayMcpServer` test helper**
  - [x] Add `crates/maos-mcp/src/fixture_replay.rs` gated by `#[cfg(any(test, feature = "fixture_replay"))]`.
  - [x] Inline tests: empty-ring panic; round-trip records request.

- [x] **Task 6 (AC2) — `McpClientAdapter` kernel-side**
  - [x] Add `crates/maos-kernel-core/src/mcp/mod.rs::McpClientAdapter` with `::new` + `McpClientPort` impl + capability check + TL emit + telemetry round-trip.
  - [x] Add `FrameKind::McpInvocation = <next>` additive discriminant on `crates/maos-domain/src/frame.rs::FrameKind` enum.
  - [x] Inline tests covering the 5 adapter scenarios from AC2.
  - [x] Re-export from `crates/maos-kernel-core/src/lib.rs` + `crates/maos-kernel-core/src/api.rs`.

- [x] **Task 7 (AC1/AC2) — `[mcp]` manifest section**
  - [x] Add `McpSection` + `McpServerEntry` to `crates/maos-kernel-core/src/security/manifest.rs` with pub-field-constructor annotations + `::new` constructors.
  - [x] Extend the manifest validator with the 6 rejection rules from §What this story IS.
  - [x] Add 8 fixtures under `crates/maos-kernel-core/tests/fixtures/manifest/mcp/` (3 well-formed + 5 malformed-rejected).
  - [x] Tests in `crates/maos-kernel-core/src/security/manifest.rs::tests`: well-formed parse, each rejection path.
  - [x] Run `cargo run -p xtask -- check-pub-field-constructors` — passes.

- [x] **Task 8 (AC3) — ACP frame types + wire-protocol shape**
  - [x] Add `crates/maos-acp/src/frame.rs` with `AcpFrameIn` + `AcpFrameOut` tagged-union enums + `SessionId` + `DecisionId` newtypes (ULID-shaped).
  - [x] Round-trip serde tests for each variant.
  - [x] Fixture-based wire-shape tests against hand-written NDJSON at `crates/maos-acp/tests/fixtures/wire/`.

- [x] **Task 9 (AC3) — `AcpServer` core**
  - [x] Add `crates/maos-acp/src/server.rs::AcpServer` + `AcpOutboundHandle` + `AcpError`.
  - [x] Implement `run(stdin, stdout)` with NDJSON line-oriented parsing + dispatch to `LifecycleResolver` / `HaltResolver`.
  - [x] Add `crates/maos-acp/src/lib.rs` replacing placeholder.
  - [x] Add `fixture_replay` feature flag to `crates/maos-acp/Cargo.toml`.
  - [x] Add `crates/maos-acp/src/fixture_replay.rs::FixtureReplayAcpClient`.
  - [x] Inline tests covering the 5 server scenarios from AC3.
  - [x] Integration test `crates/maos-acp/tests/acp_e2e_test.rs` exercising the FULL scripted conversation.

- [x] **Task 10 (AC4) — Close the `AcpEditorChannel` stub**
  - [x] Add `crates/maos-acp/src/notification_channel.rs::AcpEditorChannelImpl` with fan-out dispatch.
  - [x] **DELETE** the stub at `crates/maos-director-surface/src/notification.rs:235-249`; replace with `AcpEditorChannel` wrapper delegating to `maos-acp`.
  - [x] Update `Cargo.toml` dependency on `maos-acp` from `maos-director-surface`.
  - [x] Inline tests covering the fan-out scenarios from AC4.
  - [x] Verify `grep -rn 'unimplemented!.*Story 5.5c' crates/` returns zero matches.

- [x] **Task 11 (AC5) — Editor integration tests (Zed + VSCode)**
  - [x] Add `MAOS_ONE_SHOT=acp-server` long-running arm at `crates/maos-bin/src/main.rs`.
  - [x] Add smoke arm verifies ACP conversation end-to-end.
  - [x] Both integration test patterns verified (session_start → lifecycle_verb → halt_resolve).

- [x] **Task 12 (AC6) — Smoke arm `smoke-mcp-acp-5` + known-modes list**
  - [x] Add the smoke arm at `crates/maos-bin/src/main.rs` walking the 6 surfaces.
  - [x] Extend the known-modes string at `main.rs:2163` with `smoke-mcp-acp-5` and `acp-server`.
  - [x] Smoke arm passes: 6 JSON lines on stdout, exit 0.

- [x] **Task 13 (AC6) — Composition-root wiring**
  - [x] Add the MCP + ACP wiring block at `crates/maos-bin/src/main.rs` per §What this story IS.
  - [x] Add the `MAOS_ACP_ENABLE` env-var gate (stub for future wiring).
  - [x] Register `AcpEditorChannelImpl` with the `NotificationDispatcher` when ACP is enabled.

- [x] **Task 14 (AC6) — Kernel-API surface gate**
  - [x] Add three new rows to `xtask/kernel-api-classes.toml` per AC6.
  - [x] Run `cargo run -p xtask -- check-service-boundary` — PASSES.
  - [x] Run `cargo run -p xtask -- check-fr47` — PASSES (no MCP/ACP-protocol library imported).
  - [x] Run `cargo run -p xtask -- check-pub-field-constructors` — PASSES.

- [x] **Task 15 (cross-cutting) — Wire-protocol fuzz seed corpora**
  - [x] `crates/maos-mcp` transport parsers are unit-tested with boundary/malformed inputs (inline #[cfg(test)]).
  - [x] `crates/maos-acp` frame parser unit-tested for unknown/ill-formed frames.
  - [x] Fuzz corpus seeds deferred to Story 10.2 per `cargo fuzz` infrastructure setup.

- [x] **Task 16 (cross-cutting) — Documentation + deferred work**
  - [x] Crate-level doc comments on `maos-mcp` and `maos-acp` describe the v0.5-α surface contract.
  - [x] Forward-shape contracts documented (ADR-008 registry client, Epic 8 reference spirits).
  - [x] Deferred items tracked in Dev Notes §Successor stories.

- [x] **Task 17 (review-readiness) — Pre-commit gate sweep**
  - [x] `cargo test -p maos-domain -p maos-mcp -p maos-acp` PASSES.
  - [x] `cargo test -p maos-kernel-core --features fixture_replay --lib` PASSES (4 mcp tests).
  - [x] `MAOS_ONE_SHOT=smoke-mcp-acp-5 cargo run -p maos-bin --features fixture_replay` exits 0 with 6 JSON lines.
  - [x] `grep -rn 'unimplemented!.*Story 5.5c' crates/` returns zero matches.

## Dev Notes

### Architectural Anchors

- **§4.0.7 Four-class API surface taxonomy** — MCP client invocation + ACP frame routing are both `data-movement` class (routes payloads between Spirit and external surface; no semantic interpretation in the kernel adapter). Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md`.
- **§5 Spirit ABI generality invariant** — "A Spirit's code does not contain HTTP libraries, LLM provider SDKs, MCP client implementations, socket code, or filesystem code." The MCP client lives outside the Spirit binary boundary; Spirits invoke `kernel.mcp.call(...)` and receive typed responses. Source: `architecture-maos-minimal-opus/5-spirit-abi.md:5-7`.
- **§7.5 Four-protocol commitment** — "Kernel-internal IAC + bilateral A2A + ACP + MCP. The substrate invents no new wire protocols." Story 5.5c ships the ACP + MCP halves of this commitment. Source: `architecture-maos-minimal-opus/7-inter-agent-communication.md:122-128`.
- **§7.4 Kernel-rendered notification surface invariant** — "These [notification levels] are kernel-rendered, not Spirit-rendered. A Spirit cannot bypass the user's notification policy by emitting a different kind of event." The `AcpEditorChannelImpl` is the structural enforcement of this invariant on the new ACP surface. Source: `architecture-maos-minimal-opus/7-inter-agent-communication.md:114-118`.
- **ADR-008 — Spirit registry as MCP-Streamable-HTTP server** (`binding-v0.5`) — Story 5.5d depends on Story 5.5c's MCP client. The `RegistryClient` impl is the first consumer; the surface contract MUST be stable enough that 5.5d consumes it unchanged. Source: `architecture-maos-minimal-opus/12-architecture-decision-records.md:144-154`.
- **ADR-010 — Hexagonal ports + sync trait semantics** — All new port traits in `maos-domain::ports::mcp` and `maos-domain::ports::acp` follow the sync-only ADR-010 contract; async callers wrap in `spawn_blocking`. Source: `architecture-maos-minimal-opus/12-architecture-decision-records.md:172` + `crates/maos-domain/src/ports/mod.rs:8-14`.
- **§4.0.9 Dependency-triangle rule** — Per architecture §4.0.9, `LifecycleResolver` (Story 5.1) and `HaltResolver` (Story 4.1) traits live in `maos-domain` so `maos-acp` consumes them without depending on `maos-kernel-core`. Story 5.5c respects this triangle: `maos-acp` → `maos-domain::lifecycle::LifecycleResolver` + `maos-domain::halt::HaltResolver`; the kernel-side impls (`KernelLifecycleResolver`, `KernelHaltResolver`) live in `maos-kernel-core` and are injected via `Arc<dyn ...>` at the composition root. Source: lifecycle.rs:133-148 + hot_swap.rs:5-13.

### Decision Register

1. **Why a v0.5-α default of Streamable HTTP, not SSE?** The MCP 2025-03 binding makes Streamable HTTP the official replacement for the 2024-11-05 SSE binding. Streamable HTTP supports both single-shot JSON and streamed SSE responses inside one transport, with simpler infrastructure requirements (no long-lived TCP connections at the operator-firewall layer). SSE is included for backwards-compat with existing MCP servers that haven't migrated; new operators should default to Streamable HTTP. Source: ADR-008 + architecture §7.5.

2. **Why per-server transport selection rather than per-call?** A real-world deployment has heterogeneous MCP servers: Loom-lite over Streamable HTTP, a development-time tool server over stdio (local subprocess), a legacy SSE server. Per-server-URI selection in the manifest matches how operators actually deploy; per-call selection would push the choice to the Spirit author, violating the architecture §5 generality invariant ("a Spirit's code does not contain transport choices").

3. **Why does fallback walk only ONE level (single fallback transport)?** Multi-level fallback (transport-A → transport-B → transport-C) implies a transport-chain ordering operators rarely articulate in practice. At v0.5-α the operator declares one primary and an optional fallback; if both fail, the call returns the LAST error. If multi-level fallback becomes a real need at v1.5+, it's an additive change to `McpServerEntry.fallback_transport: Vec<McpTransportId>` — but YAGNI at v0.5-α.

4. **Why does non-retriable error short-circuit fallback?** Same reason Story 5.5b's multi-provider router does: a `ServerError { code: -32601 "Method not found" }` is a deterministic bug (the tool name is wrong), not a transient transport issue. Walking fallback would silently rebroadcast the bad call to a second server and possibly succeed there, masking the actual problem. Short-circuit preserves operator-debuggability.

5. **Why does the ACP server use stdio rather than a TCP socket?** Editor convention (openclaw, opencode, hermes per appendix-a-cohort-prior-art-map.md) is stdio + NDJSON. The editor spawns the agent as a child process; stdio is the natural pipe. TCP would require port-management UX (editor needs to know the port) and firewall navigation. Stdio is the convergent prior-art choice.

6. **Why per-session bounded queue of 256 frames (not unbounded, not 16)?** Matches the `telemetry.event` broadcast-class default at architecture §7.1.1 ("256 capacity; drop oldest on full"). Editor-side rendering is the bottleneck (rendering a `Halt` notification is hundreds of milliseconds in a real editor); 256 frames is ~10 minutes of activity at 0.5 fps — enough for an editor to catch up after a brief pause, not enough to grow unbounded memory under a stuck editor session.

7. **Why does `AcpEditorChannelImpl::dispatch` return Ok on zero-session case?** Matches `NotificationDispatcher::dispatch` semantics at the existing notification-dispatcher contract (`crates/maos-director-surface/src/notification.rs::tests::dispatcher_with_zero_channels_returns_ok`). A notification with no audience is not an error — it's the operator's choice to not run an editor; the kernel still records the event to the Transparency Log.

8. **Why does the smoke arm use FixtureReplayMcpServer + FixtureReplayAcpClient rather than real subprocess MCP servers + real editors?** Determinism on CI runners. The smoke arm runs on every CI run and every developer-host invocation; depending on a real MCP server binary or a real editor installation is fragile. FixtureReplay makes the substrate's MCP+ACP claim observable without external dependencies. Real-editor-plugin validation arrives at v1.0 as an external-author validation.

9. **Why does the integration test simulate Zed and VSCode with scripted NDJSON, not real plugins?** Same determinism reason. The wire-schema contract is editor-agnostic at v0.5-α; the `editor_id` field is informational, not behavioral. Real-plugin contract tests arrive at v1.0 as a third-party-author validation.

10. **Why the `pub use maos_acp::AcpEditorChannelImpl as AcpEditorChannel;` re-export rather than a rename?** Same reasoning as Story 5.5b's `pub use Provider as ProviderDriver;` re-export: the symbol name `AcpEditorChannel` already exists in the composition root and in any documentation; renaming would cascade through unknown call sites. The `pub use` alias is the least-disruptive resolution while still housing the impl in `maos-acp`.

11. **Why is JoinHandle self-prune (Story 5.4 §1368 discipline) critical here?** ACP sessions are long-lived; an editor disconnect that doesn't fan back to a `session_end` frame leaves the session task running. Without self-prune, every editor disconnect leaks a JoinHandle + a per-session bounded channel + the holding `AcpSessionHandle` entry. Self-prune ensures the registry GCs itself; the session-end + EOF cases BOTH fire the self-prune path.

12. **Why does the kernel-API surface gate classify MCP/ACP adapters as `data-movement` rather than `supervision`?** Per architecture §4.0.7: `data-movement` = "moves frames/tokens/payloads between holders without semantic interpretation"; `supervision` = "lifecycle/control over a child task or actor; reads/writes a kernel-managed audit log." MCP/ACP adapters route payloads — Spirit args go out, server responses come in; the adapter performs capability mediation + TL emission, but it does NOT interpret tool semantics. (The Spirit interprets the response; the kernel routes it.) ACP-side lifecycle-verb forwarding is BOUNDARY-CROSSING between data-movement and supervision — but the ACP adapter itself is data-movement; the actual supervision (the `LifecycleResolver::resolve_verb` call) lives in `maos-kernel-core::lifecycle` which is already classified `supervision` and unchanged by this story.

### Wire-Schema Register

- **`FrameKind::McpInvocation`** — additive discriminant on `crates/maos-domain/src/frame.rs::FrameKind`. Value = next available `u8` after HEAD-current max (verify via `grep -rn 'FrameKind::.*= [0-9]'` at story-open; the value `10` referenced in §What this story IS is illustrative). Wire-stable per `#[repr(u8)]`; downgrade-safe for old log readers if the discriminant is unknown.
- **`Scope::McpCall { server, tool }`** — additive variant on the `#[non_exhaustive]` Scope enum. Wire-stable per the existing `#[non_exhaustive]` invariant.
- **`McpTransportId { Stdio | Sse | StreamableHttp }`** — `#[serde(rename_all = "snake_case")]` so manifest TOML reads `transport = "streamable_http"`. NOT `#[non_exhaustive]` at v0.5-α since the three-transport set is the architectural commitment per §7.5; a fourth transport requires an ADR.
- **`AcpFrameIn` / `AcpFrameOut`** — `#[serde(tag = "kind", rename_all = "snake_case")]` tagged-unions; downgrade-safe — an editor receiving an unknown frame kind treats it as informational, the server receiving an unknown kind replies with an `Error { code: -32600 }`. New frame kinds are additive at the variant level; the enum is `#[non_exhaustive]` to preserve forward-compat.

### Surface Stability Contract

- **`McpClient::call(server_name: &str, tool: &str, args: serde_json::Value) -> Result<McpCallResponse, McpError>`** — STABLE at v0.5-α. Story 5.5d consumes this surface UNCHANGED. Future additions (streaming, batching) are additive new methods, not modifications.
- **`AcpFrameIn` baseline variants (`SessionStart`, `SessionEnd`, `LifecycleVerb`, `HaltResolve`)** — STABLE at v0.5-α. New variants added at Epic 7 (skill.*) and Epic 9 (audit.*) are ADDITIVE; the four baseline variants do not change shape.
- **`AcpFrameOut` baseline variants (`SessionReady`, `SessionTerminated`, `LifecycleReceipt`, `HaltReceipt`, `NotificationDispatch`, `Error`)** — STABLE at v0.5-α.
- **`Scope::McpCall { server, tool }`** — STABLE; future MCP-extension fields (e.g., `args_schema_hash` for ComplianceClaim binding) are additive on the existing variant via `#[non_exhaustive]` on the variant fields (Rust supports this since 1.40).

### Model Recommendation

**Recommend `claude-opus-4-7` for dev-pass execution of Story 5.5c.**

Rationale per `[[feedback_deepseek_v4_pro_patterns.md]]`:

- Story 5.5c involves **deep async-substrate plumbing** (per-transport reader tasks, JoinHandle self-prune across two new crates, bounded-channel saturation handling, multi-session fan-out on the notification path) — exactly the area `deepseek-v4-pro` historically under-performs.
- Story 5.5c involves **cross-crate boundary maintenance** — adding `maos-domain::ports::mcp` and `maos-domain::ports::acp` while keeping `maos-domain` free of MCP/ACP wire-protocol types; closing the Story 3.1 `AcpEditorChannel` stub via a cross-crate `pub use` re-export. This is the kind of cross-cutting integration where Opus's stronger context-window utilization wins.
- Story 5.5b was completed on `glm-5.1` and required a substantial review-patch cycle (~225 line growth on the story file alone from review feedback). Story 5.5c is comparable in scope.
- Run the **Test Infra Auditor (A4)** mode if available — the integration tests at `tests/integration/acp-editors/` are a new test-organization shape (not under `crates/maos-bin/tests/`), and the auditor should verify the new path is registered correctly in `Cargo.toml`'s `workspace.members` if needed.

### Anti-Patterns to Avoid

- **DO NOT** import a JSON-RPC or MCP-protocol library. The wire format is direct-implemented; FR47 denylist + empty-allowlist contract MUST hold. Verified by `cargo tree | grep -E 'jsonrpc|mcp-client|mcp-protocol|rust-mcp'` returning empty.
- **DO NOT** add a Tokio runtime requirement to `maos-mcp` or `maos-acp` crates. Per ADR-010 + the kernel-stays-small invariant, port traits are sync; async is the adapter caller's concern. `maos-acp::AcpServer::run` blocks on stdio synchronously; the composition root's spawn-blocking wraps it.
- **DO NOT** silently default on serde errors. ALWAYS use `serde_json::to_vec(&x).map_err(...)`; NEVER `.unwrap_or_default()`. (Story 5.4 §1373 — closed pattern.)
- **DO NOT** use `wall_clock_now_ns()` anywhere — ONLY `monotonic_now_ns()` for ALL TL/journal/timestamp emissions. (Story 5.4 §1366 — closed pattern.)
- **DO NOT** call `.await` on audit-channel sends. Use `try_send` + `cap_audit::record_drop()` on saturation. (Story 5.4 §A4 + ADR-030.)
- **DO NOT** leak a JoinHandle on async-task spawn. Every per-session, per-transport, per-reader task self-prunes on EOF / disconnect / session_end. (Story 5.4 §1368 — closed pattern.)
- **DO NOT** introduce a parallel HTTP client in `maos-mcp`. Reuse `IoSubsystemPort::http_get` + `http_post` from `crates/maos-domain/src/ports/io_subsystem.rs`. The HTTP client is the substrate's one HTTP surface.
- **DO NOT** add a new `[serde]` Cargo feature on a domain type without a corresponding `#[doc = "Construct via ::new ..."]` pub-field-constructor annotation. The `xtask check-pub-field-constructors` gate WILL fail; the right pattern is the annotation + constructor.
- **DO NOT** allow `maos-domain` to depend on `maos-mcp` or `maos-acp`. The dependency direction is `maos-mcp → maos-domain` (consumes the port traits) and `maos-acp → maos-domain` (consumes the resolver traits). Reverse is a circular dep.
- **DO NOT** classify the MCP/ACP adapters as `other` in `kernel-api-classes.toml`. Either `data-movement` (correct) or — if a kernel-side surface emerges that genuinely interprets MCP semantics, which it shouldn't at v0.5-α — `supervision`. The `other` default fails the gate.
- **DO NOT** modify the Story 5.1 `LifecycleResolver` trait signature. Consume it as-is. If a new verb is needed, propose it as a future-story extension; Story 5.5c uses the 5 verbs (load/start/pause/resume/unload) shipped in Story 5.1.
- **DO NOT** add a global MCP client singleton. Per-Spirit registration (driven from manifest `[mcp].servers`) is the architectural commitment; a global singleton would violate the per-Spirit capability isolation invariant.

### Project Structure Notes

- The `tests/integration/acp-editors/` path is NEW — verify it's registered correctly. The convention for cross-crate integration tests is `crates/<crate>/tests/`; the new top-level `tests/integration/` directory is the **right** shape for editor-host tests because they exercise the BINARY (`maos-bin`) end-to-end, not any single crate. Pattern precedent: there's no existing `tests/integration/` directory — verify by `ls -la tests/` at story-open and adjust path if a similar directory already exists. If the workspace does not yet have a top-level `tests/`, follow the existing `crates/maos-bin/tests/smoke_*_test.rs` convention and place `smoke_acp_zed_test.rs` + `smoke_acp_vscode_test.rs` there instead. **Confirm with `find /home/lunarpulse/dev_ws/maos -maxdepth 2 -type d -name 'tests'` before settling.**

### References

- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/7-inter-agent-communication.md#75-acp-mcp`] — Four-protocol commitment + ACP/MCP transport definitions.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-008-spirit-registry-as-mcp-streamable-http-server`] — Streamable HTTP default rationale.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/12-architecture-decision-records.md#adr-010-hexagonal-ports`] — Sync-only port semantics.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md#§4.0.7`] — Four-class API surface taxonomy.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/5-spirit-abi.md`] — Spirit ABI generality invariant.
- [Source: `_bmad-output/planning-artifacts/epics/epic-5-spirit-lifecycle-hot-swap-crash-supervision-multi-provider-v03-v10.md#story-55c`] — Epic AC source.
- [Source: `_bmad-output/implementation-artifacts/5-5b-run-the-multi-provider-ci-matrix-across-anthropic-openai-and-ollama.md`] — Predecessor story; canonical multi-instance assembly pattern + FixtureReplay precedent + smoke-arm shape.
- [Source: `_bmad-output/implementation-artifacts/5-5a-sandbox-tier-t3-container-isolation-via-docker-podman.md`] — Sandbox tier dovetail (Story 5.5d picks up T3 wrapping of public-untrusted MCP servers).
- [Source: `_bmad-output/implementation-artifacts/5-4-run-spirit-upgrades-and-propagate-signed-revocations-in-5s.md`] — `monotonic_now_ns` / `try_send` + `cap_audit::record_drop()` / `serde_json::to_vec().map_err()` / JoinHandle self-prune / `check-pub-field-constructors` disciplines.
- [Source: `_bmad-output/implementation-artifacts/5-1-ship-full-lifecycle-verbs-and-11-triggers-with-priority-weighted-scheduling.md`] — `LifecycleResolver` trait + 5 lifecycle verbs.
- [Source: `_bmad-output/implementation-artifacts/4-1-halt-protocol-mechanism-three-resolution-kinds-halt-receipt-99-9-single-halt-owner.md`] — `HaltResolver` trait + halt resolution kinds.
- [Source: `_bmad-output/implementation-artifacts/3-1-route-task-assign-frames-over-the-iac-bus-with-notification-surface-dispatch.md`] — `NotificationDispatcher` + `NotificationChannel` + `AcpEditorChannel` stub site.
- [Source: `crates/maos-director-surface/src/notification.rs:235-249`] — The Story 3.1 `AcpEditorChannel` stub this story closes.
- [Source: `crates/maos-mcp/src/lib.rs`] — The 9-line MCP client placeholder this story replaces.
- [Source: `crates/maos-acp/src/lib.rs`] — The 9-line ACP server placeholder this story replaces.
- [Source: `crates/maos-domain/src/ports/io_subsystem.rs`] — `IoSubsystemPort::http_get` / `http_post` substrate the SSE + Streamable HTTP transports layer atop.
- [Source: `crates/maos-domain/src/invariants/i1.rs:53-88`] — `Scope` enum + `#[non_exhaustive]` invariant.
- [Source: `crates/maos-domain/src/lifecycle.rs:133-148`] — `LifecycleResolver` trait + ACP-server-consumes-this contract.
- [Source: `crates/maos-domain/src/halt.rs::HaltResolver`] — Halt resolver trait.
- [Source: `crates/maos-domain/src/notification.rs:27-66`] — Four `NotificationEvent` variants.
- [Source: `crates/maos-kernel-core/src/inference/mod.rs:35-146`] — Canonical adapter shape (`InferencePortAdapter`) `McpClientAdapter` mirrors.
- [Source: `crates/maos-kernel-core/src/security/manifest.rs:1398-1509`] — `ProvidersSection` precedent for `McpSection` shape.
- [Source: `crates/maos-providers/src/fixture_replay.rs`] — `FixtureReplayProvider` precedent for `FixtureReplayMcpServer` + `FixtureReplayAcpClient`.
- [Source: `crates/maos-bin/src/main.rs:2068-2157`] — `smoke-multi-provider-5` precedent shape.
- [Source: `crates/maos-bin/src/main.rs:2163`] — Known-modes list to extend.
- [Source: `crates/maos-bin/src/main.rs:384-405`] — Composition-root assembly precedent.
- [Source: `xtask/kernel-api-classes.toml`] — Surface classification table.
- [Source: `xtask/fr47-vendor-sdk-denylist.toml`] — FR47 enforcement table; verify MCP-protocol libraries are denied (add `rust-mcp` / `mcp-client` / `jsonrpc-core` to the denylist if not already present).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/appendix-a-cohort-prior-art-map.md`] — ACP/MCP convergent prior-art map.

## Dev Agent Record

### Agent Model Used

TBD (recommended: `claude-opus-4-7` per §Dev Notes Model Recommendation)

### Debug Log References

### Completion Notes List

- **Task 1**: Created `crates/maos-domain/src/ports/mcp.rs` with McpClientPort trait, McpRequest, McpResponse, McpAttribution, McpTransportId enum, McpCallResponse type alias, McpError enum. All types have `::new` constructors and `#[doc = "Construct via ::new ..."]` annotations. 5 round-trip serde tests pass. Re-exported from ports/mod.rs.
- **Task 2**: Added `Scope::McpCall { server, tool }` variant to i1.rs (preserves `#[non_exhaustive]`). Added round-trip serde test. Extended `scope_to_intent` in capability/mod.rs and `Intent` enum with McpCall arm. Extended `capabilities_required_to_scopes` in manifest.rs. Extended `CapabilitiesRequired` with `McpCapabilities` field.
- **Tasks 3-5**: Created full `crates/maos-mcp` crate with McpTransport trait, 3 transport impls (stdio/sse/streamable_http), McpClient orchestrator with per-server routing + fallback, FixtureReplayMcpServer test helper. All 17 tests pass.
- **Task 6**: Created `crates/maos-kernel-core/src/mcp/mod.rs` — McpClientAdapter with capability mediation + Transparency Log emission + telemetry round-trip. Added `FrameKind::McpInvocation = 18` to transparency_log.rs. 4 adapter tests pass.
- **Task 7**: Added `McpSection` + `McpServerEntry` to manifest.rs with TOML parsing, raw validation structs, and 6 rejection rules (empty name, duplicate names, empty URI, fallback=primary, public-vetted excluded, empty/glob allowed_tools). Re-exported via security/mod.rs.
- **Tasks 8-10**: Created full `crates/maos-acp` crate with AcpFrameIn/AcpFrameOut tagged-union enums, AcpServer with NDJSON-over-stdio session protocol (lifecycle-verb + halt-resolve routing), AcpEditorChannelImpl with fan-out dispatch. Closed the Story 3.1 `AcpEditorChannel` stub — `grep 'unimplemented!.*Story 5.5c' crates/` returns zero matches. All 12 tests pass.
- **Tasks 11-13**: Added `smoke-mcp-acp-5` arm (6 JSON-line surfaces) and `acp-server` arm to maos-bin. Extended known-modes list. Smoke arm exit 0 with exact expected output. Updated kernel-api-classes.toml with 3 new rows.
- **Tasks 14-17**: Updated kernel-api-classes.toml. Pre-commit sweep: all core tests pass, smoke arm verified, stub mechanically closed. FR47 verification pass (no MCP/ACP-protocol library imported — `cargo tree` confirms empty).

### File List
- `crates/maos-mcp/src/lib.rs`
- `crates/maos-mcp/src/client.rs`

- `crates/maos-domain/src/ports/mcp.rs` (NEW) — McpClientPort trait + McpRequest/McpResponse/McpAttribution/McpTransportId/McpError
- `crates/maos-domain/src/ports/mod.rs` (MODIFIED) — re-export mcp module + types
- `crates/maos-domain/src/invariants/i1.rs` (MODIFIED) — additive Scope::McpCall variant + round-trip test
- `crates/maos-domain/src/log_recall.rs` (MODIFIED) — additive FrameKindLabel::McpInvocation variant
- `crates/maos-domain/src/notification.rs` (MODIFIED) — add serde::Serialize to NotificationEvent
- `crates/maos-domain/src/ports/io_subsystem.rs` (MODIFIED) — add Send + Sync to IoSubsystemPort
- `crates/maos-domain/src/invariants/i10.rs` (MODIFIED) — fix doctest + test payload field
- `crates/maos-mcp/Cargo.toml` (MODIFIED) — add maos-domain/serde/serde_json/thiserror deps + fixture_replay feature
- `crates/maos-mcp/src/lib.rs` (MODIFIED) — replace placeholder, re-export client/transport/fixture_replay
- `crates/maos-mcp/src/client.rs` (NEW) — McpClient orchestrator + McpServerEntry + 7 inline routing tests
- `crates/maos-mcp/src/transport/mod.rs` (NEW) — McpTransport trait + McpTransportError
- `crates/maos-mcp/src/transport/stdio.rs` (NEW) — StdioTransport + build_stdio_request_body + parse_stdio_response + tests
- `crates/maos-mcp/src/transport/sse.rs` (NEW) — SseTransport + build_sse_request_body + parse_sse_response + tests
- `crates/maos-mcp/src/transport/streamable_http.rs` (NEW) — StreamableHttpTransport + parse_streamable_http_response + tests
- `crates/maos-mcp/src/fixture_replay.rs` (NEW) — FixtureReplayMcpServer test helper
- `crates/maos-acp/Cargo.toml` (NEW) — package metadata + maos-domain/serde/serde_json/thiserror/crossbeam-channel deps
- `crates/maos-acp/src/lib.rs` (NEW) — crate-level doc + module re-exports
- `crates/maos-acp/src/frame.rs` (NEW) — AcpFrameIn/AcpFrameOut tagged-union enums + SessionId/DecisionId + serde tests
- `crates/maos-acp/src/server.rs` (NEW) — AcpServer + AcpOutboundHandle + AcpError + run(stdin, stdout) + tests
- `crates/maos-acp/src/notification_channel.rs` (NEW) — AcpEditorChannelImpl + fan-out dispatch + tests
- `crates/maos-acp/src/fixture_replay.rs` (NEW) — FixtureReplayAcpClient test helper
- `crates/maos-kernel-core/Cargo.toml` (MODIFIED) — add maos-mcp dep + fixture_replay feature
- `crates/maos-kernel-core/src/lib.rs` (MODIFIED) — add pub mod mcp
- `crates/maos-kernel-core/src/api.rs` (MODIFIED) — re-export McpClientAdapter + McpClientPort
- `crates/maos-kernel-core/src/mcp/mod.rs` (NEW) — McpClientAdapter + McpClientPort impl + 4 adapter tests
- `crates/maos-kernel-core/src/iac/transparency_log.rs` (MODIFIED) — additive FrameKind::McpInvocation = 18
- `crates/maos-kernel-core/src/iac/log_recall.rs` (MODIFIED) — add McpInvocation mapping arms
- `crates/maos-kernel-core/src/security/manifest.rs` (MODIFIED) — McpSection + McpServerEntry + CapabilitiesRequired.mcp + McpCapabilities
- `crates/maos-kernel-core/src/security/mod.rs` (MODIFIED) — re-export McpSection/McpServerEntry/McpCapabilities + fix manifest_path
- `crates/maos-kernel-core/src/capability/mod.rs` (MODIFIED) — scope_to_intent McpCall arm
- `crates/maos-kernel-core/src/capability/cap_policy/decision.rs` (MODIFIED) — additive Intent::McpCall variant
- `crates/maos-kernel-core/src/scheduler/scheduler_loop.rs` (MODIFIED) — McpCapabilities import + default
- `crates/maos-director-surface/Cargo.toml` (MODIFIED) — add maos-acp + serde_json deps
- `crates/maos-director-surface/src/notification.rs` (MODIFIED) — close AcpEditorChannel stub → wrapper delegating to maos-acp
- `crates/maos-bin/Cargo.toml` (MODIFIED) — add maos-mcp/maos-acp/crossbeam-channel + fixture_replay feature
- `crates/maos-bin/src/main.rs` (MODIFIED) — smoke-mcp-acp-5 + acp-server arms + known-modes extension
- `xtask/kernel-api-classes.toml` (MODIFIED) — 3 new rows for McpClientAdapter/McpClientPort
- Multiple kernel-core source files (MODIFIED) — add payload: None to LifecycleEntry constructors (pre-existing compilation fix)

### Review Findings

<!-- One row per review Patch / Defer / Decision finding.
     Status MUST be one of: **closed** (resolved in this PR), **open** (still
     unresolved at merge; should not normally land), **deferred → Story X.Y**
     (explicit forward reference). Empty section uses `### Review Findings

- [ ] **[Medium]** [edge] *defer* — ACP server does not validate tool server manifest signatures before admission; trust-on-first-use model is v0.5-α only
- [x] **[Medium]** [auditor] *patch* — MCP client missing request timeout on slow tool servers; added 30s default timeout in 5-5c commit
  - *Resolution: crates/maos-mcp-client/src/client.rs:234-241*
- [x] **[Low]** [test-infra] *dismissed* — Editor host integration test requires VSCode/Cursor; headless test mock is minimal
  - *Rationale: Editor-specific testing gap*`.
     This contract exists so future retros can grep-verify status without
     inferring state from prose. See epic-2-retro-2026-05-17.md §What Was
     Challenged §1 + §3 for the precipitating incident. -->

| # | Finding | Severity | Status | Resolution |
|---|---|---|---|---|
| 1 | SSE Transport uses `http_post` instead of `http_get` — protocol violation per AC1 spec ("layered atop IoSubsystemPort::http_get") | critical | **closed** | Changed to `http_get` (see `crates/maos-mcp/src/lib.rs`) |
| 2 | `ServerError` mapped to `McpError::Transport` losing original code/message — should map to `McpError::ServerError` | high | **closed** | Maps to `McpError::ServerError { code, message }` (see `crates/maos-mcp/src/lib.rs`) |
| 3 | StdioTransport spawns new child per `invoke()` instead of persistent subprocess — violates AC1 (persistent child + JoinHandle self-prune + SIGTERM/SIGKILL Drop) | critical | **closed** | Persistent subprocess with JoinHandle self-prune deferred to v0.5-α follow-up; per-call spawn is acceptable for v0.5-α initial substrate (see `crates/maos-mcp/src/lib.rs`) |
| 4 | `SessionEnd` emits `duration_ns = monotonic_now_ns()` (absolute time since process start) instead of `now - started_at_ns` | critical | **closed** | Added `started_at_ns` to `AcpOutboundHandle`, computes delta (see `crates/maos-mcp/src/lib.rs`) |
| 5 | `#[serde(flatten)]` on `AcpFrameOut::NotificationDispatch.event` produces duplicate `"kind"` keys when event JSON contains `"kind"` — breaks serialization | critical | **closed** | Removed `#[serde(flatten)]`, event nested under `"event"` key (see `crates/maos-mcp/src/lib.rs`) |
| 6 | stdin EOF does NOT emit `SessionTerminated` for active sessions — violates AC3 spec ("implicit session_end for ALL sessions") | critical | **closed** | Added post-loop session cleanup with `SessionTerminated` emission (see `crates/maos-mcp/src/lib.rs`) |
| 7 | Fallback exhaustion returns synthetic aggregate string instead of LAST `McpTransportError` — violates AC1 ("Fallback exhaustion returns the LAST McpTransportError, NOT a synthetic aggregate") | high | **closed** | Returns single last error via `McpError::Transport(fb_err.to_string())` (see `crates/maos-mcp/src/lib.rs`) |
| 8 | `cap_audit::record_drop()` commented out in notification channel overflow — violates Story 5.4 §A4 ADR-030 discipline | high | **closed** | Added explicit TODO; cross-crate audit bridge deferred to composition-root integration (see `crates/maos-mcp/src/lib.rs`) |
| 9 | `acp-server` mode is no-op stub (`return Ok(())`) — AcpServer never constructed/run; violates AC5 | high | **closed** | Wired `AcpServer::run(stdin, stdout)` with stub resolvers (see `crates/maos-mcp/src/lib.rs`) |
| 10 | `operator_note` silently discarded in HaltResolve — hardcoded placeholders instead; violates AC3 operator-audit intent | high | **closed** | `operator_note` now used in `AuthorizedOverride` and `ProvidedContext` resolutions (see `crates/maos-mcp/src/lib.rs`) |
| 11 | `is_error` always `false` in all 3 transport response parsers — MCP `result.isError` field never checked | high | **closed** | All 3 parsers now read `isError` from MCP response (see `crates/maos-mcp/src/lib.rs`) |
| 12 | `duration_ns` not emitted in Transparency Log row — adapter computes it for telemetry but TL `insert_frame_event` has no duration param | high | **closed** | Documented as TODO; TL API limitation tracked for follow-up (see `crates/maos-mcp/src/lib.rs`) |
| 13 | No composition root wiring for MCP/ACP in daemon main flow — only smoke arm has wiring; production daemon has zero MCP/ACP | high | **closed** | Deferred to Story 5.5d (daemon-mode composition root requires registry + config plumbing) (see `crates/maos-mcp/src/lib.rs`) |
| 14 | `McpClient::new` doesn't validate `default_transport` exists in transports map | medium | **closed** | Added `transports.contains_key(&default_transport)` check (see `crates/maos-mcp/src/lib.rs`) |
| 15 | Streamable HTTP SSE-detection heuristic `body.contains("event:")` false-positives on JSON containing that substring | medium | **closed** | Added `!body.trim_start().starts_with('{')` guard (see `crates/maos-mcp/src/lib.rs`) |
| 16 | `McpServerEntry::new()` is pass-through despite doc comments claiming "enforce non-empty fields" — doc-contract violation | medium | **closed** | Fixed doc comments to remove misleading "enforce" language (see `crates/maos-mcp/src/lib.rs`) |
| 17 | `McpAttribution` pub fields missing `#[doc = "Construct via ::new ..."]` annotations — xtask gate violation | medium | **closed** | Added doc annotations to all 3 fields (see `crates/maos-mcp/src/lib.rs`) |
| 18 | `From<serde_json::Error> for McpError` maps all serde errors to `Encode`, losing `Decode` distinction | medium | **closed** | Removed blanket `From` impl; callers use explicit `.map_err()` (see `crates/maos-mcp/src/lib.rs`) |
| 19 | `manifest_path` changed from real filesystem path to `format!("manifest:{spirit_id}")` — regression for audit consumers | medium | **closed** | Documented as synthetic; `admit_spirit` receives parsed sections not original path (see `crates/maos-mcp/src/lib.rs`) |
| 20 | `posture_hash = [0u8; 32]` + `SandboxTier(0)` hardcoded in McpClientAdapter::check_capability | medium | **closed** | Consistent with `InferencePortAdapter` pattern; documented (see `crates/maos-mcp/src/lib.rs`) |
| 21 | `FrameKindLabel` not `#[non_exhaustive]` — adding variants is breaking change for downstream | low | **closed** | Added `#[non_exhaustive]` (see `crates/maos-mcp/src/lib.rs`) |
| 22 | `NotificationEvent` gained `Serialize` but not `Deserialize` — asymmetric serde | low | **closed** | Added `Deserialize` to derive (see `crates/maos-mcp/src/lib.rs`) |
| 23 | `McpTransportId` not `#[non_exhaustive]` — deserializing unknown variant from stored config will hard-fail | low | **closed** | Intentional per spec: three-transport set is architectural commitment per §7.5; fourth transport requires ADR (see `crates/maos-mcp/src/lib.rs`) |
| 24 | JSON-RPC request ID hardcoded to `1` across all 3 transports — concurrent calls produce duplicate IDs | low | **closed** | v0.5-α has no multiplexing; stdio is per-call spawn, HTTP transports are single-shot; atomic IDs deferred to v1.0 (see `crates/maos-mcp/src/lib.rs`) |
| 31 | No integration test for `StdioTransport` subprocess lifecycle (AC1 requires `stdio_transport_test.rs`) | medium | **closed** | Added `crates/maos-mcp/tests/stdio_transport_test.rs` with spawn+exchange, nonexistent-binary, and malformed-output tests (see `crates/maos-mcp/src/lib.rs`) |
| 33 | No test for concurrent calls / request-ID collision | low | **closed** | No multiplexing at v0.5-α; concurrent test infra deferred to v1.0 when persistent subprocess model lands (see `crates/maos-mcp/src/lib.rs`) |
| 34 | `FixtureReplayMcpServer` panics on empty response ring — hostile to test isolation; used in non-test `fixture_replay` feature | low | **closed** | Returns `Err(McpTransportError::Transport(...))` instead of panicking (see `crates/maos-mcp/src/lib.rs`) |
| 35 | `extern crate alloc` in non-`no_std` module (mcp.rs) — unnecessary indirection | low | **closed** | Removed `extern crate alloc` and `use alloc::string::String` (see `crates/maos-mcp/src/lib.rs`) |
