---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments:
  - ../openclaw/
  - ../ironclaw/
  - ../hermes-agent/
  - ../paperclip/
  - ../codex/
  - ../gemini-cli/
  - ../claudian/
  - ../opencode/
  - ../rustain/
workflowType: 'research'
lastStep: 1
research_type: 'technical'
research_topic: 'AI agent frameworks and coding tools: comparative architectural analysis'
research_goals: 'Analyze core architectures, operating mechanisms, security models, and agent orchestration across nine projects — four AI agent frameworks (openclaw, ironclaw, hermes-agent, paperclip) and five AI coding tools (codex, gemini-cli, claudian, opencode, rustain)'
user_name: 'Lunarpulse'
date: '2026-05-04'
web_research_enabled: true
source_verification: true
---

# Research Report: technical

**Date:** 2026-05-04
**Author:** Lunarpulse
**Research Type:** technical (in-depth + comparative architectural analysis)

---

## Research Overview

In-depth comparative architectural analysis of nine AI projects colocated under `/home/lunarpulse/dev_ws/`, split into two cohorts.

### Cohort A — AI agent frameworks

- **openclaw** — TypeScript/pnpm monorepo (`apps`, `packages`, `extensions`, `ui`, `Swabble`) with sandbox Dockerfiles, fly.toml, and a substantial security/skills surface.
- **ironclaw** — Rust workspace (`crates`, `tools-src`, `channels-src`, `wit`) with worker/Docker/K8s deploys, a `migrations/` folder, and FEATURE_PARITY tracking.
- **hermes-agent** — Python agent + Node TUI (`ui-tui`, `tui_gateway`) with `acp_adapter`, `acp_registry`, MCP serve, batch/cron, and tool distributions.
- **paperclip** — TypeScript/pnpm monorepo (`cli`, `server`, `ui`, `packages`, `evals`) with evals harness, releases, and patches.

### Cohort B — AI coding tools

- **codex** — Hybrid Bazel + pnpm + Rust workspace (`codex-rs`, `codex-cli`, `sdk`) with `.devcontainer`, MODULE.bazel, and `third_party`.
- **gemini-cli** — Node/TypeScript workspace (`packages`) with esbuild, `integration-tests`, `perf-tests`, `memory-tests`, SEA build, and Husky hooks.
- **claudian** — Obsidian-style plugin (esbuild, `manifest.json`, Jest) with `_bmad`/`_bmad-output` planning artifacts checked in.
- **opencode** — SST + Bun monorepo (`packages`, `infra`, `sdks`, `specs`) with multi-language READMEs and `.opencode` config.
- **rustain** — Rust workspace (`src`, `tests`, `tests_tui`, `perf-tests`) with multiple provider runners (anth, ds, or, zai), `_bmad-output`, and rotated session logs.

### Investigation pillars

The investigation focuses on:

1. **Core architectures** — module/crate/workspace topology, language choices, build & dependency graph, plugin/extension surface.
2. **Operating mechanisms** — runtime model (process, daemon, CLI, TUI, server), session lifecycle, transport (stdio, JSON-RPC, MCP, ACP, HTTP), event/streaming model, persistence.
3. **Security model** — sandboxing (containers, seccomp, MicroVM, OS-native), permission/approval gates, secrets handling, supply chain (lockfiles, SBOM, signing, hooks), and detect-secrets/precommit posture.
4. **Agent orchestration** — single vs multi-agent topology, sub-agent spawning, tool routing, MCP/ACP/A2A integration, parallelism/queueing, hand-off and termination semantics, memory/context passing.

### Source mix

- **Primary:** local working trees (source of truth for what is actually shipped/used).
- **Secondary (verification):** current public sources via web search — upstream READMEs, official docs, release notes, GitHub issues/PRs, CVEs/advisories, blog posts from project maintainers — used to confirm non-obvious claims and cite originating context.
- **Tertiary (cross-cohort):** comparison against well-known reference implementations (Anthropic Claude Code, OpenAI Codex CLI, Google Gemini CLI, Aider, Continue) to anchor design choices.

---

## Technical Research Scope Confirmation

**Research Topic:** AI agent frameworks and coding tools: comparative architectural analysis
**Research Goals:** Analyze core architectures, operating mechanisms, security models, and agent orchestration across nine projects — four AI agent frameworks (openclaw, ironclaw, hermes-agent, paperclip) and five AI coding tools (codex, gemini-cli, claudian, opencode, rustain).

**Technical Research Scope:**

- Architecture Analysis — workspace topology, language/build, dependency graph, plugin/extension surface
- Implementation Approaches — runtime model (CLI/TUI/server/daemon), session lifecycle, event/streaming model, persistence
- Technology Stack — languages, frameworks, tools, platforms (TypeScript/pnpm, Rust workspaces, Python, Bun/SST, Bazel, Obsidian-plugin)
- Integration Patterns — transport choices (stdio, JSON-RPC, MCP, ACP, A2A, HTTP), provider abstractions, plugin/extension surfaces
- Performance Considerations — parallelism/queueing, streaming, caching, sandbox overhead

**Security Model Coverage:**

- Sandboxing (containers, seccomp, MicroVM, OS-native)
- Permission/approval gates
- Secrets handling (detect-secrets, baselines, env var policies)
- Supply chain (lockfiles, SBOM, signing, pre-commit hooks, dependency review)

**Agent Orchestration Coverage:**

- Single vs multi-agent topology
- Sub-agent spawning, tool routing
- MCP / ACP / A2A integration surface
- Parallelism / queueing, hand-off and termination semantics
- Memory and context passing

**Research Methodology:**

- Primary source: local working trees in `/home/lunarpulse/dev_ws/` (source of truth)
- Secondary: current public sources via web search with citations and confidence levels for non-obvious claims
- Tertiary: cross-comparison against well-known reference implementations
- Multi-source validation for critical technical claims; explicit `[unverified]` / `[low-confidence]` tags where evidence is thin

**Out of scope (unless added later):** performance benchmarking, model quality comparisons, business/licensing analysis, UI/UX critique.

**Scope Confirmed:** 2026-05-04

## Per-Project Technical Overview

This section is the source-of-truth per-project snapshot. Later sections (integration patterns, security model, agent orchestration, synthesis) cross-reference these without repeating their content.

Each snapshot covers four pillars uniformly: at-a-glance, runtime model, dependencies & frameworks, security model, agent orchestration, and key file references. Local working trees were read directly; non-obvious claims are cited with `path:line`. Confidence flags `[unverified]` mark areas where evidence was thin.

---

### Cohort A — AI agent frameworks

#### A.1 openclaw

##### openclaw — At a glance
- Upstream: `https://github.com/openclaw/openclaw.git`. License: MIT (Copyright 2025 Peter Steinberger; `LICENSE`).
- Description: "Multi-channel AI gateway with extensible messaging integrations" / "personal AI assistant" — a long-running gateway that bridges chat channels (WhatsApp, Telegram, Slack, Discord, Signal, iMessage, IRC, Matrix, Teams, etc.) into LLM agents.
- Primary language: TypeScript on Node.js (`type: "module"`, ESM). Required runtime: Node 22.12+ / 24 recommended (`openclaw.mjs:11-13`). Companion native apps: Swift (macOS/iOS — `apps/macos/`, `apps/ios/`), Kotlin (`apps/android/`), and `Swabble/`.
- Package manager: pnpm (`pnpm-workspace.yaml`, `pnpm-lock.yaml`); Bun is also a documented install target.
- Build: `tsdown 0.21.10` (`tsdown.config.ts`), `oxlint`/`oxfmt` for lint/format, `vitest 4`, `@typescript/native-preview` (tsgo).
- Workspace topology (pnpm `: ., ui, packages/*, extensions/*`):
  - `src/` core gateway/CLI/agents (~7,594 TS files, ~815k LOC including tests).
  - `extensions/` — 130+ plugins per channel/provider (`anthropic`, `openai`, `google`, `slack`, `telegram`, `whatsapp`, `signal`, `mcp`, `acpx`, …); ~5,554 TS files, ~544k LOC.
  - `ui/` lit-based control UI (~107k LOC).
  - `packages/` published SDKs: `sdk`, `plugin-sdk`, `plugin-package-contract`, `memory-host-sdk`.
  - `apps/{macos,ios,android,macos-mlx-tts,shared}` companion native clients.
  - `skills/` — 50+ user-facing skill bundles; `.agents/skills/` — internal automation skills.
  - `docs/` extensive Mintlify-style docs (24 subfolders).

##### openclaw — Runtime model
- CLI entry: `openclaw.mjs` (Node version gate + compile-cache wrapper) → `dist/index.js` → `src/entry.ts:21-119`. Library entry: `src/index.ts:1-119`.
- Process model: long-running Gateway daemon (launchd/systemd user service) plus CLI subcommands. Subcommand fan-out built in `src/cli/program.ts` (Commander tree: `gateway`, `acp`, `chat`, `agent`, `channels`, `mcp`, `skills`, `nodes`, `cron`, `tasks`, `dashboard`, `doctor`, `setup`, `status`, `tui`, `webhooks`, `proxy`, `secrets`, `models`, `plugins`).
- Session lifecycle: session key (`agent:<id>:...`, including `:subagent:<child>`) resolved by `src/routing/session-key.ts`; persisted entries in `src/config/sessions/store.ts`; `src/gateway/boot.ts` runs an optional `BOOT.md` warm-up at gateway start.
- Transports:
  - Gateway control plane: WebSocket via `ws` (`src/gateway/client.ts:4,213,323`); HTTP endpoints under `src/gateway/*-http.ts` for MCP, embeddings, models, control UI.
  - ACP: NDJSON stdio JSON-RPC via `@agentclientprotocol/sdk` (`src/acp/server.ts:4-125`); translator at `src/acp/translator.ts` (1,444 lines).
  - MCP: bundled tool servers over stdio (`src/mcp/openclaw-tools-serve.ts:1-37`) and HTTP (`src/gateway/mcp-http.ts`); also acts as MCP client via `@modelcontextprotocol/sdk` (`src/mcp/channel-server.ts`).
  - Channel adapters in `extensions/<id>` push inbound messages into the gateway.
- Streaming: provider streaming owned by `src/agents/anthropic-transport-stream.ts` (1,158 lines, streams `messages.stream`) and `src/agents/openai-ws-stream.ts` (OpenAI Realtime over WS). Async iterables propagate up.

##### openclaw — Major dependencies & frameworks
- LLM/provider runtime: `@mariozechner/pi-agent-core`, `@mariozechner/pi-ai`, `@mariozechner/pi-coding-agent`, `@mariozechner/pi-tui` (all `0.70.6`) — the "pi" embedded coding agent is the default agent harness. `openai ^6.34.0` only direct LLM SDK at root; provider plugins ship per-provider auth/catalog. Anthropic transport hand-rolled including Claude-Code beta headers (`anthropic-beta`, `claude-code-20250219` at `src/agents/anthropic-transport-stream.ts:608-705`).
- Protocols: `@agentclientprotocol/sdk 0.21.0`, `@modelcontextprotocol/sdk 1.29.0`.
- Infrastructure: `commander`, `@clack/prompts`, `chokidar`, `croner`, `ws`, `undici` + `proxy-agent` + `https-proxy-agent` + `global-agent`, `sqlite-vec`, `markdown-it`, `typebox` + `ajv` + `zod`, `tslog`, `@lydell/node-pty`, `web-push`.
- Search/sandbox: bundled providers (`extensions/{brave,duckduckgo,exa,firecrawl,perplexity,searxng,tavily,web-readability}`). Sandbox runtimes: Docker (default) and SSH/Podman; no firecracker/gvisor.
- Plugin/extension surface: `extensions/`, user `skills/`, internal `.agents/skills/`, `src/plugin-sdk/` + `packages/plugin-sdk/`. `pnpm-workspace.yaml:onlyBuiltDependencies` whitelists native deps.

##### openclaw — Security model
- Sandboxing: untrusted shell/exec runs inside Docker (`Dockerfile.sandbox` debian:bookworm-slim @ pinned digest, runs as `sandbox` user; `Dockerfile.sandbox-common` adds language toolchains; `Dockerfile.sandbox-browser` adds Chromium + Xvfb + noVNC + websockify exposing `9222/5900/6080`). Implementation in `src/agents/sandbox/` (74 files): `docker-backend.ts`, `ssh-backend.ts`, `docker.ts` (`buildSandboxCreateArgs`), `fs-bridge*.ts`, `validate-sandbox-security.ts`, `tool-policy.ts`, `sanitize-env-vars.ts`. Helper installers: `setup-podman.sh`/`docker-setup.sh`.
- Permission/approval: ACP approval classifier (`src/acp/approval-classifier.ts:14-30`) buckets tool calls into `readonly_scoped`, `readonly_search`, `mutating`, `exec_capable`, `control_plane`, `interactive`. Exec approvals are persisted and awaited (`src/agents/bash-tools.exec-approval-request.ts`, `src/infra/exec-approvals.ts`) with a `/approve` race-safe registration. CLI: `openclaw exec-approvals`, `openclaw exec-policy`, `openclaw security`. Owner-only / mutating tool gating: `src/agents/tools/owner-only-tools.ts`, `src/agents/tool-mutation.ts`, `src/agents/tool-policy.ts`.
- Audit suite: `src/security/audit-*.ts` (~50 files) covers gateway exposure, channel readonly resolution, plugin trust, sandbox docker config, sandbox-browser, hooks routing, exec safe-bins, secrets, model hygiene.
- Secrets: `.detect-secrets.cfg`, `.secrets.baseline` (433 KB), `.env.example`, `.semgrepignore`. Pre-commit (`.pre-commit-config.yaml`) runs `detect-secrets` + private-key detection + large-file guard.
- Supply chain: `pnpm-lock.yaml` committed; `pnpm-workspace.yaml:minimumReleaseAge=2880` (2-day registry hold). `zizmor.yml` lints GH workflows. `security/opengrep/` ships custom semgrep rules. `.github/workflows/docker-release.yml:161-162,259-260` builds with `sbom: true` and `provenance: mode=max`, plus `verify-attestations` job. `.github/workflows/openclaw-npm-release.yml:281,366` uses npm trusted publishing with provenance.

##### openclaw — Agent orchestration
- Topology: hierarchical multi-agent. Default harness "pi-embedded" (`src/agents/pi-embedded-*.ts`, `src/agents/harness/builtin-pi.ts`); pluggable via `agents.runtime` / `OPENCLAW_AGENT_RUNTIME` (`src/agents/harness-runtimes.ts:1-32`). Sub-agents spawned via `sessions_spawn` control-plane tool with allowlist gating (`src/agents/subagent-target-policy.ts:1-30`, `src/agents/tools/agents-list-tool.ts:38`). Inter-session calls go through `runAgentStep` (`src/agents/tools/agent-step.ts:1-60`) with provenance `{kind:"inter_session", sourceSessionKey, sourceChannel, sourceTool}`.
- Tool catalog: 122 files under `src/agents/tools/`. Tools implement `AgentTool` from `@mariozechner/pi-agent-core`; registered via `src/agents/tool-catalog.ts` and the plugin loader.
- Tool dispatch path: providers stream tool_use blocks; pi harness drives the loop. Anthropic-side tool handling and reconciliation of dangling tool_use/tool_result pairs lives in `src/agents/anthropic-transport-stream.ts:376-409` and `src/agents/compaction.ts:543-559`. Mutating-tool / exec routing via `src/agents/bash-tools.exec-host-node.ts` and `src/agents/bash-tools.exec-host-gateway.ts`.
- MCP integration: built-in MCP server exports a curated subset (`src/mcp/openclaw-tools-serve.ts:14-22`). Plugin MCP tools served via `src/mcp/plugin-tools-serve.ts`. MCP HTTP gateway endpoint at `src/gateway/mcp-http.ts`.
- ACP integration: `openclaw acp` runs `AcpGatewayAgent` over stdio NDJSON and translates ACP `initialize`/`newSession`/`prompt`/`cancel` into gateway WS calls (`src/acp/server.ts:1-125`). ACP can be hard-disabled via config (`acp.enabled=false`) — `src/acp/policy.ts:1-40`.
- A2A: not present as a named protocol; cross-agent calls happen over the gateway via `sessions_spawn` / `sessions_send` / `agent-step.ts` with input-provenance tagging.
- Memory/context: per-agent transcript persisted in session store; long-context handled by `src/agents/compaction.ts`. Vector memory via `extensions/memory-core` + `extensions/memory-lancedb` + `extensions/memory-wiki` and `packages/memory-host-sdk`.
- Termination & errors: global handlers in `src/index.ts:90-117`. Provider error classification in `src/agents/pi-embedded-helpers.isbillingerrormessage.test.ts` and `failover-error.test.ts`. ACP cancel mapped via `translator.cancel-scoping.test.ts`.

##### openclaw — Key file references
- `openclaw.mjs:1-60` — Node-version-gated launcher / compile-cache wrapper.
- `src/entry.ts:21-119` — main process bootstrap (warning filter, env normalize, gateway-startup trace, respawn).
- `src/index.ts:85-118` — main vs library mode, fatal-error handlers, calls `runLegacyCliEntry`.
- `src/cli/program.ts` — Commander subcommand tree.
- `src/gateway/client.ts:4-326` — WebSocket gateway client with secure-URL / loopback gating.
- `src/gateway/boot.ts:1-40` — gateway BOOT.md warm-up + session-key resolution.
- `src/acp/server.ts:1-125` — ACP stdio JSON-RPC server attached to the gateway.
- `src/acp/translator.ts` (1,444 lines) — ACP↔gateway protocol translation.
- `src/acp/approval-classifier.ts:14-30` — approval class taxonomy.
- `src/agents/agent-command.ts:1-50,954-955` (1,263 lines) — top-level agent command orchestrator.
- `src/agents/anthropic-transport-stream.ts:376-705` (1,158 lines) — Anthropic-Messages streaming transport.
- `src/agents/compaction.ts:36,263-290,543-559` — adaptive context compaction.
- `src/agents/bash-tools.exec-approval-request.ts:90-246` — race-safe approval registration.
- `src/agents/sandbox/docker.ts` (632 lines) — Docker sandbox creation/exec.
- `src/agents/tools/agent-step.ts:1-60` — sub-agent / inter-session call with `inputProvenance`.
- `src/mcp/openclaw-tools-serve.ts:1-37` — MCP server bundled with `openclaw mcp serve`.
- `Dockerfile.sandbox`, `Dockerfile.sandbox-common`, `Dockerfile.sandbox-browser` — sandbox container images.
- `.github/workflows/docker-release.yml:161-162,259-260,408-427` — SBOM + max-mode provenance with attestation verification.

---

#### A.2 ironclaw

##### ironclaw — At a glance
- Origin/license: forked from NEAR AI's `nearai/ironclaw` (`Cargo.toml:30-31, 28`). Dual-licensed `MIT OR Apache-2.0`.
- Languages/runtime: Rust 2024 edition, MSRV 1.92 (`Cargo.toml:25-26`). Tokio multi-threaded runtime (`src/main.rs:42-45`). Non-Rust pieces: WIT IDL files for the WebAssembly Component Model (`wit/tool.wit`, `wit/channel.wit`), embedded HTML/JS/CSS frontend (`crates/ironclaw_gateway/src/assets.rs`), Python orchestrator referenced from `crates/ironclaw_engine/src/lib.rs:6-13` (v2 engine).
- Build: Cargo workspace; `build.rs` (~430 lines) embeds git metadata, registry catalog, bundled skills, and cross-compiles `channels-src/telegram` to `wasm32-wasip2`, then converts to a WASI Preview 2 component via `wasm-tools` (`build.rs:30-79`).
- Topology:
  - `crates/`: 6 library crates — `ironclaw_common`, `ironclaw_engine` (v2 unified execution model), `ironclaw_gateway` (web frontend bundle), `ironclaw_safety` (prompt-injection / leak defense), `ironclaw_skills` (SKILL.md parser), `ironclaw_tui` (Ratatui TUI).
  - `tools-src/`: 14 WASM-tool source crates (gmail, github, google-* suite, slack, telegram, web-search, composio, llm-context, portfolio) — workspace-excluded so they cross-compile to WASM.
  - `channels-src/`: 5 WASM channel crates (discord, feishu, slack, telegram, whatsapp).
  - `wit/`: two `near:agent@0.3.0` Component Model packages (`tool.wit`, `channel.wit`).
  - `infra/runner/`, `deploy/` (systemd units + setup script), `migrations/` (25 refinery V*.sql files), `wix/`, `registry/`.
- Scale: 437 `.rs` files in `src/`, 110 in `crates/`. Main `src/main.rs` is 1,777 lines; `src/agent/dispatcher.rs` is 4,214 lines; `src/llm/rig_adapter.rs` ~2,000 lines.

##### ironclaw — Runtime model
- Entry points: `src/main.rs:38` (sync entry → builds Tokio runtime → `async_main`); `src/lib.rs:41-87` exposes ~40 modules. Second binary: `src/bin/sandbox_daemon.rs`. The `worker` mode is a subcommand of the same `ironclaw` binary, dispatched via `worker::run_acp_bridge_command` (`src/worker/mod.rs:73`).
- Process model: Single binary that can run as CLI/TUI/REPL, web/HTTP server, sandbox orchestrator, and worker (inside Docker). `src/main.rs:9-29` wires `AppBuilder`, `ChannelManager`, `WebhookServer`, `SandboxReaper`, `OrchestratorApi`. `Dockerfile.worker` builds a Debian image that runs `ironclaw` in worker mode for sandboxed jobs.
- Session lifecycle: `Agent` owns sessions/threads/turns (`AGENTS.md:22`); session manager at `src/agent/session_manager.rs`, thread state in `src/agent/session.rs`. Persistence via `src/db/postgres.rs` and `src/db/libsql/` behind a shared `Database` trait; 25 `migrations/V*.sql` cover conversations, tool failures, sandbox columns, routines, secrets, embeddings, etc.
- Transports: axum 0.8 (`Cargo.toml:103`) for the gateway and orchestrator HTTP API (`src/orchestrator/api.rs:122`). Stdio JSON-RPC via `agent-client-protocol = "0.10"` — the worker spawns ACP-compliant agents (Goose/Codex/Gemini CLI) over stdio (`src/worker/acp_bridge.rs:1-20`). MCP transports: `src/tools/mcp/{stdio_transport,http_transport,unix_transport}.rs`. WebSocket via `tokio-tungstenite`, webhooks in `src/webhooks/`. Channels include REPL, HTTP, Signal, Web gateway, plus WASM-channel router.
- WIT/WASM: `wit/tool.wit:1` declares `near:agent@0.3.0` with a `host` interface (log/now/workspace-read/http-request/tool-invoke/secret-exists) and a `tool` interface (execute/schema/description). `wit/channel.wit:1-42` defines the `channel-host` interface for sandboxed messaging channels with a host-managed event loop. Wasmtime 43.0.2 with `component-model` (`Cargo.toml:153`).
- Streaming: `src/llm/CLAUDE.md:239` notes the `LlmProvider` trait is **non-streaming** — `complete()` and `complete_with_tools()` return whole responses. The OpenAI Codex provider does internally consume SSE. Web gateway uses SSE/broadcast for live UI updates (`src/channels/web/sse.rs`).

##### ironclaw — Major dependencies & frameworks
- LLM SDKs: `rig-core = "0.30"` is the multi-provider abstraction; a thick `RigAdapter` (`src/llm/rig_adapter.rs:40,538`) implements the local `LlmProvider` trait. AWS Bedrock via `aws-sdk-bedrockruntime` opt-in. Custom providers: `nearai_chat`, `openai_codex_provider`, `bedrock`, `github_copilot`, `gemini_oauth`, `anthropic_oauth`. No `async-anthropic` / `async-openai` — Rig handles those.
- Tokio/axum: `tokio = "1"` full features, `axum = "0.8"` with WS, `tower-http`. No sqlx — uses `tokio-postgres` + `deadpool-postgres` + `refinery` migrations plus `libsql = "0.6"` for embedded mode.
- Other: `wasmtime` 43 (component model), `bollard = "0.18"` for Docker control, `cron = "0.13"`, `jsonschema = "0.45"`, `pgvector = "0.4"`, `aes-gcm`/`hkdf`/`hmac`/`sha2`/`blake3`/`ed25519-dalek`, `secrecy = "0.10"`, `pty-process` (Unix). OS keychains: `security-framework` (macOS), `secret-service` + `zbus` (Linux). Distribution via `cargo-dist 0.31` targeting 7 triples (`Cargo.toml:287-295`); Railway, GitHub Actions in `.github/workflows/`.

##### ironclaw — Security model
- WASM sandbox (tools/channels): Wasmtime component model with capability-based host imports (`wit/tool.wit:14-105`). Per-instance fuel + memory limits in `src/tools/wasm/limits.rs:1-205` (default 500M instructions). Secrets injected at host boundary; WASM can only call `secret-exists`, never read values (`wit/tool.wit:96-105`). HTTP allowlist + leak-detector scanning of responses (`src/tools/wasm/{allowlist,credential_injector,http_security}.rs`).
- Docker sandbox (commands): `src/sandbox/mod.rs:1-103` documents three policies (ReadOnly/WorkspaceWrite/FullAccess). `src/sandbox/container.rs:457-458` enforces `readonly_rootfs`, UID 1000, dropped capabilities, validating HTTP proxy with credential injection, network isolation. No seccomp/landlock — relies on Docker + custom proxy.
- Approval/permission model: `src/tools/permissions.rs` (571 lines) with `PermissionState`, `effective_permission`, "always requires approval" flag, `PendingApproval` in `src/agent/session.rs`. `src/safety/mod.rs` and `crates/ironclaw_safety/src/lib.rs` implement prompt-injection detection (`Sanitizer`, `Validator`, `LeakDetector`, `Policy`).
- Secrets: `src/secrets/mod.rs:62,116-187` — encrypted store keyed by master key resolved from `SECRETS_MASTER_KEY` env then OS keychain (`src/secrets/keychain.rs`). `.env.example` exhaustive (~13 KB) with no real secrets.
- Supply chain: committed `Cargo.lock`, `deny.toml` (`unmaintained = "workspace"`, `yanked = "deny"`, license allowlist), `release-plz.toml`, `.githooks/{pre-commit,pre-push,commit-msg}`, `codecov.yml`, `clippy.toml`, `.github/workflows/replay-gate.yml`. Build script refuses to commit pre-built WASM (`build.rs:3`).

##### ironclaw — Agent orchestration
- Multi-agent / multi-job: Concurrent jobs with isolated `JobContext` per session (`src/lib.rs:38`, `src/context/`). Background runtime: `SandboxReaper`, `JobMonitor`, `Heartbeat` (`src/agent/{job_monitor,heartbeat}.rs`), `Routine` cron engine (`src/agent/routine_engine.rs`).
- Tool dispatch path: Three converging entry points documented in `src/tools/dispatch.rs:1-15` — agent worker `Worker::execute_tool()` (v1 agent loop), v2 `EffectBridgeAdapter::execute_action()` (engine), and channel-agnostic `ToolDispatcher::dispatch()` (`src/tools/dispatch.rs:65-80`). All converge on `ToolRegistry` (`src/tools/registry.rs`). Agent-side tool execution lives in `src/agent/dispatcher.rs:1-80` (4,214 lines) calling into `crate::agent::agentic_loop::run_agentic_loop` (`src/agent/agentic_loop.rs:213`).
- MCP/ACP/A2A: MCP client suite at `src/tools/mcp/{client,client_store,factory,protocol,session,stdio_transport,http_transport,unix_transport,auth}.rs`. ACP at `src/worker/acp_bridge.rs:1-135` — bridge spawns ACP-compliant agents (Goose, Codex, Gemini CLI) inside Docker workers via stdio JSON-RPC. No A2A. v2 unified engine in `crates/ironclaw_engine` collapses Tool/Skill/Hook into a `Capability` primitive.
- Memory/context: `src/workspace/` — file-like persistent memory with hybrid full-text + vector (pgvector) search and Reciprocal Rank Fusion. `src/history/`, `src/agent/compaction.rs`, `src/agent/context_monitor.rs`. Schema in `migrations/V1__initial.sql` + V9 (flexible embedding dim), V16 (document_versions).
- Worker/queue: `Dockerfile.worker` packages dev tools (git, build-essential, node, python3, gh CLI). `ContainerJobManager` issues per-job bearer tokens (`src/orchestrator/{auth,job_manager}.rs`); orchestrator HTTP API on port 50051 default. `deploy/ironclaw.service` + `deploy/cloud-sql-proxy.service` for systemd; `infra/runner/` for self-hosted CI runner; `railway.toml` + `docker-compose.yml` for hosted deploys.

##### ironclaw — Key file references
- `src/main.rs:38` — sync entry that boots the Tokio runtime
- `src/lib.rs:41-87` — module catalog (composition root surface)
- `src/agent/agentic_loop.rs:213` — `run_agentic_loop`
- `src/agent/dispatcher.rs:1-80` — agent-side tool dispatch + approvals (4,214 lines)
- `src/tools/dispatch.rs:65-80` — `ToolDispatcher` (channel-agnostic third entry point)
- `src/llm/provider.rs:407-422` — `LlmProvider` trait (non-streaming)
- `src/llm/rig_adapter.rs:40,538` — multi-provider adapter via `rig-core`
- `src/orchestrator/mod.rs:54-103` — orchestrator setup + container manager
- `src/worker/acp_bridge.rs:1-135` — ACP bridge spawning external agents
- `src/sandbox/mod.rs:1-103` — Docker sandbox doc + exports
- `src/sandbox/container.rs:391-460` — container creation w/ readonly rootfs, dropped caps
- `src/tools/wasm/limits.rs:1-205` — Wasmtime fuel/memory limits
- `wit/tool.wit:14-152` — Component Model contract for sandboxed tools
- `wit/channel.wit:1-80` — host-managed event loop for WASM channels
- `crates/ironclaw_engine/src/lib.rs:1-30` — v2 unified Thread/Capability/MemoryDoc engine
- `migrations/V1__initial.sql` … `V25__wasm_fuel_limit_bump.sql` — refinery schema history
- `build.rs:30-79` — cross-compiles Telegram channel to WASI p2 component

---

#### A.3 hermes-agent

##### hermes-agent — At a glance
- Origin: `https://github.com/NousResearch/hermes-agent` (Nous Research). License: MIT (`pyproject.toml:12`).
- Version: 0.12.0 / v2026.4.30 — "The Curator release" (`pyproject.toml:7`, `RELEASE_v0.12.0.md:1-3`).
- Primary languages: Python 3.11+ (~1,370 .py files); Node/TypeScript (Ink/React) for the alternate TUI in `ui-tui/`; shell installer (`setup-hermes.sh`).
- Build/package mgmt: `pyproject.toml` + `uv.lock` (uv); setuptools backend; `package.json` + `package-lock.json` for `ui-tui/`; Nix flake (`flake.nix`, `flake.lock`).
- Workspace topology (top-level dirs): `agent/` (56 py — provider adapters, memory, redaction, prompt cache, curator, trajectory), `hermes_cli/` (66 py — subcommand router, setup wizard, plugins), `tools/` (86 py — auto-discovered tool registry + terminal backends), `gateway/` (56 py — multi-platform messenger), `acp_adapter/` (9 py), `cron/` (3 py), `plugins/` (48 py), `tui_gateway/` (8 py), `ui-tui/` (Node TUI), `tests/` (922 py), `environments/` (30 py — Atropos RL envs), `tinker-atropos/` (empty placeholder; `atroposlib`/`tinker` pulled via git+ extra), `optional-skills/`, `skills/`, `datagen-config-examples/`, `acp_registry/`, `web/`, `website/`, `docker/`, `nix/`.
- Scale: `cli.py` ~12k LOC, `run_agent.py` ~14k LOC, `mcp_serve.py` ~870 LOC; release notes report 217k insertions across 1,270 files since v0.11.0.

##### hermes-agent — Runtime model
- Entry points (`pyproject.toml:132-135`): `hermes` → `hermes_cli.main:main`; `hermes-agent` → `run_agent:main`; `hermes-acp` → `acp_adapter.entry:main`. Top-level scripts: `cli.py` (legacy interactive CLI), `run_agent.py` (`AIAgent` core loop, `run_agent.py:873`), `mcp_serve.py`, `batch_runner.py`, `rl_cli.py`, `mini_swe_runner.py`.
- Process model: single `AIAgent` class (sync conversation loop with internal asyncio bridges) is the heart; on top sit (a) interactive CLI/TUI, (b) gateway daemon multiplexing 18+ messenger platforms, (c) MCP stdio server, (d) ACP adapter (JSON-RPC stdio agent), (e) cron tick scheduler, (f) batch trajectory generator, (g) RL training driver, (h) Atropos rollout envs.
- Session lifecycle: `hermes_state.py` defines `SessionDB`, a SQLite (WAL + FTS5) store holding session metadata, full message history, and parent-chain links for compression-triggered splits. Batch/RL trajectories stored separately. `~/.hermes/` is HERMES_HOME.
- Transports: stdio MCP via FastMCP (`mcp_serve.py:51,431-439`); stdio ACP via `agent-client-protocol>=0.9.0` (`acp_adapter/server.py`); HTTP optional via `web` extra (FastAPI/uvicorn) for dashboard; gateway has REST/webhooks per platform; cron uses fcntl/msvcrt file lock (`cron/scheduler.py`).
- Streaming: async streaming inside `AIAgent`, `tui_gateway/event_publisher.py` + `ws.py` for WebSocket fan-out to the Node TUI; ACP uses `AgentMessageChunk`/thinking/tool-progress callbacks (`acp_adapter/events.py`).

##### hermes-agent — Major dependencies & frameworks
- LLM SDKs (core): `openai>=2.21,<3`, `anthropic>=0.39,<1` (`pyproject.toml:15-16`); lazy-imported via proxy classes for cold-start (`run_agent.py:53-89`). Optional: `mistralai`, `boto3` (Bedrock). No vllm/transformers in core deps — RL inference goes through OpenAI-compatible endpoints.
- RL stack (`rl` extra): `atroposlib` (NousResearch/atropos) + `tinker` (thinking-machines-lab) installed from pinned commits; `wandb`, FastAPI. The local `tinker-atropos/` directory is empty — actual integration code is in `environments/` (`HermesAgentLoop`, `HermesAgentBaseEnv`).
- Tools: Exa, Firecrawl, Parallel-Web, fal-client, edge-tts, Playwright (Camofox browser). `mini_swe_runner.py` drives mini-SWE-bench evals. `croniter` for scheduled jobs. `agent-client-protocol` for ACP. `mcp` (optional) for MCP servers/clients. Honcho client for dialectic user modeling.
- Trajectory / datagen: `trajectory_compressor.py` (post-processes JSONL trajectories — protects head/tail turns, summarizes middle); `batch_runner.py` parallelizes across prompts with checkpointing; `datagen-config-examples/` ships sample configs; `toolset_distributions.py` randomizes toolset selection per rollout for diversity.

##### hermes-agent — Security model
- Sandboxing: Defaults to `terminal.backend: local` direct host execution (`SECURITY.md`). Container/remote backends are opt-in: Docker, Modal, Daytona, Singularity, SSH, Vercel Sandbox, all under `tools/environments/`. Image: `Dockerfile` builds Debian 13 with non-root `hermes` UID 10000, tini PID 1, gosu for UID remap.
- Permission/approval: `tools/approval.py` is the dangerous-command boundary with three modes (`on`/`auto`/`off`) and per-session contextvar state; smart approvals can run an auxiliary LLM. ACP has its own `acp_adapter/permissions.py`.
- Subagent safety: `delegate_task` (`tools/delegate_tool.py`) blocks recursive delegation, memory writes, send_message, execute_code, clarify; `MAX_DEPTH = 1` (note: `SECURITY.md` says 2 — `[unverified]` discrepancy, `tools/delegate_tool.py:128`). `skip_memory=True` for children. Output redaction: `agent/redact.py` strips secret patterns from displayed text (off-by-default in v0.12 per release notes).
- Secrets / supply chain: `.env.example` (~20 KB), `~/.hermes/.env` is canonical; `constraints-termux.txt` for Android. Core deps pinned to known-good ranges (CVE notes inline at `pyproject.toml:23,38`). MCP subprocesses get filtered env via `_build_safe_env()` in `tools/mcp_tool.py`; npx/uvx packages checked against OSV (`tools/osv_check.py`). GitHub workflows: `supply-chain-audit.yml` scans PRs for litellm-style payloads, plus `tests.yml`, `docker-publish.yml`, `nix.yml`. `SECURITY.md` documents single-tenant trust model.

##### hermes-agent — Agent orchestration
- Multi-agent: Yes — `tools/delegate_tool.py` spawns child `AIAgent` instances in a `ThreadPoolExecutor` with isolated context, fresh task_id, restricted toolsets, and parent-only summary-result observation. Single and batch (parallel) delegation modes.
- Tool dispatch: `tools/registry.py` is the auto-discovery registry — every file under `tools/` calls `registry.register()` at import. `model_tools.py:30,180` triggers `discover_builtin_tools()` then exposes `get_tool_definitions()` and `handle_function_call()` to all callers (CLI, gateway, batch, RL). `toolsets.py` defines named bundles; `toolset_distributions.py` defines probabilistic distributions over toolsets for batch/RL data generation runs.
- MCP vs ACP vs ACP registry:
  - `mcp_serve.py` exposes Hermes' *gateway/messaging conversations* (10 tools: conversations_list, messages_send, events_wait, permissions_list_open, etc.) as an MCP server so external clients (Claude Desktop, Cursor) can drive it.
  - `acp_adapter/` makes Hermes itself act as an *ACP agent* — clients (e.g. Zed, acp-bridge) talk JSON-RPC over stdio and `AIAgent` runs in an executor pool (`acp_adapter/server.py:65-73`).
  - `acp_registry/` is a 2-file static manifest (`agent.json`, `icon.svg`) advertising the `hermes acp` distribution to ACP-compliant launchers.
- Trajectory & memory: `agent/memory_manager.py` + `agent/memory_provider.py` (with optional Honcho), `agent/curator.py` + `curator_backup.py` (autonomous skill maintenance, runs via cron tick), `agent/context_compressor.py`. Trajectories: `agent/trajectory.py` for runtime, `trajectory_compressor.py` for post-hoc JSONL compression, `hermes_state.py` for FTS5-searchable session history.
- RL loop: `rl_cli.py` is the user-facing driver (Tinker + WandB + OpenRouter). `environments/agent_loop.py` (`HermesAgentLoop`) and `environments/hermes_base_env.py` (`HermesAgentBaseEnv`) implement the Atropos integration; concrete envs include `web_research_env.py`, `agentic_opd_env.py`, `terminal_test_env/`, `hermes_swe_env/`, plus benchmarks. `tools/rl_training_tool.py` lets the agent itself launch training.

##### hermes-agent — Key file references
- `pyproject.toml:13-39` — pinned core deps
- `pyproject.toml:95-101` — RL extra (atroposlib + tinker git+ pins)
- `pyproject.toml:132-135` — entry-point scripts
- `run_agent.py:873` — `AIAgent` class definition (core loop)
- `run_agent.py:271` — `IterationBudget` (parent+subagent cap)
- `run_agent.py:1191` — `_delegate_depth` tracking
- `model_tools.py:30,180` — tool registry discovery and `TOOL_TO_TOOLSET_MAP`
- `tools/registry.py` — auto-discovery + `register()` API
- `tools/delegate_tool.py:128` — `MAX_DEPTH = 1` and `DELEGATE_BLOCKED_TOOLS`
- `tools/approval.py:1-30` — dangerous command approval boundary
- `mcp_serve.py:51,431-439` — FastMCP server exposing gateway as MCP tools
- `acp_adapter/server.py:65-73` — ACP server thread pool
- `acp_registry/agent.json` — ACP launcher manifest
- `hermes_state.py:1-20` — `SessionDB` (SQLite WAL+FTS5)
- `trajectory_compressor.py` — middle-turn trajectory summarization
- `environments/agent_loop.py`, `environments/hermes_base_env.py` — Atropos RL plumbing
- `cron/scheduler.py` — file-locked tick driver
- `Dockerfile` + `docker/entrypoint.sh` + `docker-compose.yml` — container topology
- `.github/workflows/supply-chain-audit.yml` — PR-time supply-chain scanner

---

#### A.4 paperclip

##### paperclip — At a glance
- Origin: personal fork of `paperclipai/paperclip` (current branch `bugfix5004`); MIT License.
- Primary languages: TypeScript / Node.js (>=20). ESM `.ts` for server/cli/adapters, `.tsx` for UI.
- Build / package manager: pnpm 9.15.4 workspaces (`pnpm-workspace.yaml`); CLI bundled via esbuild (`cli/esbuild.config.mjs`); UI built with Vite 6 + Tailwind 4; server is `tsc` -> `dist`. Patched dependency: `embedded-postgres@18.1.0-beta.16` (`patches/`).
- Workspace topology: `cli/`, `server/`, `ui/`, `packages/{shared,db,adapter-utils,plugins/sdk,plugins/create-paperclip-plugin}` plus `packages/adapters/{claude-local,codex-local,cursor-local,gemini-local,opencode-local,pi-local,openclaw-gateway}`. Top-level `evals/promptfoo/`, `docs/`, `doc/`, `releases/`, `patches/`, `data/secrets`, `tests/{e2e,release-smoke}`, `skills/`.
- Scale: 12 internal packages; ~719 `.ts` + ~178 `.tsx` files; core hot files: `server/src/services/heartbeat.ts` 4004 LOC, `plugin-loader.ts` 1954 LOC, `agents.ts` 693 LOC.

##### paperclip — Runtime model
- Entry points: CLI bin `paperclipai` -> `cli/dist/index.js` (built from `cli/src/index.ts:1`); server bootstrap `server/src/index.ts:74` `startServer()`. Express app factory in `server/src/app.ts:60`.
- Process model: Express HTTP server (default `PORT=3100`) + optional embedded Postgres + WebSocket server. The CLI is an operator/admin tool (`onboard`, `doctor`, `run`, `heartbeat run`, `auth bootstrap-ceo`, `worktree`, etc.). **Agents are not in-process LLMs**: each "agent" is a child-process invocation of an external CLI (Claude Code, Codex, Cursor, Gemini, opencode, pi) spawned by the heartbeat service.
- Session lifecycle: No CLI<->server pairing in the chat sense. Server owns persistent state in Postgres; the heartbeat scheduler (`server/src/services/heartbeat.ts:2040 executeRun(runId)`, `:2657 adapter.execute(...)`) claims a run for an agent, materializes a workspace (e.g. git worktree), invokes the adapter, then ingests adapter stdout. CLI command `heartbeat run` triggers a single run via the API and streams logs.
- Transports: REST under `/api/...` mounted in `server/src/app.ts:131-157`; Better-Auth at `/api/auth/*authPath`; live updates via WebSocket upgrade in `server/src/realtime/live-events-ws.ts:1-50`. Adapters typically spawn external CLIs over stdio and parse stream-JSON (e.g. `parseClaudeStreamJson` in `packages/adapters/claude-local/src/server/execute.ts`). The `openclaw-gateway` adapter speaks WebSocket to a remote OpenClaw gateway. **No first-party MCP/ACP server** in this repo (grep yielded only an unrelated `mcp` substring) `[unverified]`.
- Streaming: Adapter stdout JSON chunks parsed line-by-line; live-event bus (`server/src/services/live-events.ts:46 subscribeCompanyLiveEvents`) fans out to per-company WebSocket subscribers.

##### paperclip — Major dependencies & frameworks
- LLM/provider SDKs: **No first-party LLM SDK calls** — paperclip orchestrates installed agent CLIs. Production Docker image globally installs `@anthropic-ai/claude-code@latest`, `@openai/codex@latest`, `opencode-ai` (`Dockerfile:46`). Adapters wrap each CLI; model lists hard-coded e.g. `packages/adapters/claude-local/src/index.ts:4-10`.
- Eval framework: `evals/promptfoo/promptfooconfig.yaml` — Promptfoo 0.103.3 with OpenRouter providers (`claude-sonnet-4`, `gpt-4.1`, `codex-5.4`, `gemini-2.5-pro`); deterministic assertion suites under `evals/promptfoo/tests/{core,governance}.yaml`.
- UI framework: React 19 + react-router-dom 7 + Tailwind 4 + Radix UI / `cmdk` / `dnd-kit` / `lexical` / `@mdxeditor/editor` / `mermaid` / `@tanstack/react-query`.
- Server stack: Express 5, `better-auth` 1.4.18, Drizzle ORM 0.38.4 + embedded-postgres, `ws` 8.x, `pino` logging, `zod`+`ajv` for schemas, `multer` uploads, `sharp` images, `chokidar`, `@aws-sdk/client-s3`. `hermes-paperclip-adapter` is a pinned external integration.

##### paperclip — Security model
- Sandboxing: Image-level via `Dockerfile` (Node lts-trixie-slim, `node` user with host UID/GID mapping, volume `/paperclip` for state, `EXPOSE 3100`). Dedicated compose stacks `docker/docker-compose.{quickstart,untrusted-review}.yml`, plus `docker/quadlet/` (rootless Podman). In-process plugin sandbox uses Node `vm` with allow-listed module specifiers (`server/src/services/plugin-runtime-sandbox.ts:1-40`) and a per-operation `CapabilityScopedInvoker`.
- Permission/approval model: First-class `approvals` domain — `server/src/services/approvals.ts:11`, REST at `server/src/routes/approvals.ts`, agent state machine includes `pending_approval`. Board mutation gating in `middleware/board-mutation-guard.ts`; `actorMiddleware` distinguishes `board` vs `agent` actors; private-hostname gate when `deploymentMode=authenticated && exposure=private`. Per-agent permissions service.
- Secrets: `.env.example` minimal; pluggable secret providers via `server/src/secrets/{provider-registry,local-encrypted-provider,external-stub-providers}.ts` keyed off `PAPERCLIP_SECRETS_PROVIDER` / `PAPERCLIP_SECRETS_MASTER_KEY_FILE`; `data/secrets/` holds local encrypted state; `.dockerignore` excludes `.git`, `data/`, `.paperclip`, `node_modules`. Migration helper `scripts/migrate-inline-env-secrets.ts`.
- Supply chain: `pnpm-lock.yaml` checked in, `pnpm install --frozen-lockfile` in Docker; explicit `patchedDependencies` for `embedded-postgres`; `.github/workflows/{pr,docker,e2e,release,release-smoke,refresh-lockfile}.yml`.

##### paperclip — Agent orchestration
- Multi-agent / sub-agents: Designed around a "company" of many agents (CEO/CTO/engineers). Each agent has its own row, adapter type, instructions, schedule, and budget; agents are scheduled by `heartbeatService` and `routineService`. **No inner sub-agent recursion in paperclip itself** — sub-agent behavior is delegated to whatever CLI the adapter wraps.
- Tool dispatch path: Two layers. (1) Agent runs: heartbeat picks a run, builds env/skills, calls `adapter.execute(...)` (`heartbeat.ts:2657`) which `runChildProcess`-spawns the external CLI. (2) Paperclip-side plugins: `createPluginToolDispatcher` (`server/src/app.ts:170`) routes tool calls to plugin workers managed by `createPluginWorkerManager` and `pluginLoader`, with capability-validated host services.
- MCP / ACP / A2A integration: No first-party MCP/ACP server in this repo `[unverified]`. Cross-instance / external-agent integration goes through the **OpenClaw gateway** adapter (`packages/adapters/openclaw-gateway/src/server/execute.ts`) over WebSocket with signed device identities.
- Memory/context: Postgres (Drizzle) is the system of record — companies, agents, issues, runs, approvals, costs, activity log. Per-run `executionWorkspaces` (often git worktrees) provide filesystem context. Skills/instructions injected via `services/agent-instructions.ts`; `skills/` packs ship with the server bundle.
- Eval loop integration: Promptfoo eval suite targets the heartbeat skill behavior against `apiUrl: 'http://localhost:18080'`. Phases 1+ TS harness still planned.

##### paperclip — Key file references
- `cli/src/index.ts:25` — CLI command graph (commander).
- `server/src/index.ts:74` — `startServer()` bootstrap.
- `server/src/app.ts:60` — Express app, route mounting, plugin runtime wiring.
- `server/src/realtime/live-events-ws.ts:1` — WebSocket transport for live events.
- `server/src/services/heartbeat.ts:2048` — `executeRun` agent loop core.
- `server/src/services/heartbeat.ts:2657` — `adapter.execute(...)` dispatch into external agent CLIs.
- `server/src/services/plugin-tool-dispatcher.ts:1` — plugin tool dispatch path.
- `server/src/services/plugin-runtime-sandbox.ts:1` — `vm`-based plugin sandbox + capability gating.
- `server/src/services/approvals.ts:11` — approval state machine.
- `packages/adapters/claude-local/src/server/execute.ts:1` — Claude Code adapter (stdio + stream-JSON).
- `packages/adapters/openclaw-gateway/src/server/execute.ts:1` — OpenClaw remote-agent gateway (WebSocket).
- `Dockerfile:46` — production image installs Claude/Codex/opencode CLIs.
- `evals/promptfoo/promptfooconfig.yaml` — eval harness config + providers.

---

### Cohort B — AI coding tools

#### B.1 codex

##### codex — At a glance
- Origin: github.com/openai/codex (OpenAI's official Codex CLI). License: **Apache-2.0** (`LICENSE:1`).
- Primary languages: Rust (the actual agent — `codex-rs/`), Python and TypeScript SDKs (`sdk/python/`, `sdk/typescript/`), plus a 100-line Node shim in `codex-cli/bin/codex.js` that just dispatches to a platform-specific prebuilt binary.
- Build: Three coordinated systems. **Cargo** is the source of truth for the Rust workspace; **Bazel** (`MODULE.bazel:1`) provides a hermetic, cross-platform build (Linux/macOS/Windows-gnullvm with vendored LLVM toolchain and macOS SDK pulled from Apple) used in CI. **pnpm** coordinates only the JS/TS surface (formatters, npm publishing wrapper, TS SDK). A top-level `justfile` and `flake.nix` glue everything.
- Workspace topology: `codex-rs/` (Rust, the engine), `codex-cli/` (npm publishing wrapper distributing prebuilt Rust binaries via per-platform `@openai/codex-*` packages), `sdk/{python,typescript}/`, `scripts/`, `tools/argument-comment-lint/`, `third_party/` (bundles `meriyah`, `v8`, `wezterm`), `docs/`, `patches/` (Bazel patches for llvm/abseil).
- Scale: ~88 Rust crates in the workspace. 2 SDK packages (Python `codex-app-server-sdk`, TS `@openai/codex-sdk`). The TUI alone is ~82 source files.

##### codex — Runtime model
- Entry points: User-facing binary is the Rust `codex` from `codex-rs/cli/src/main.rs:1` (`[[bin]] name = "codex"`). It is a multitool that dispatches by argv0 or subcommand into TUI (`codex-tui::run_main`), `exec` (non-interactive), `app-server`, `mcp-server`, `responses-api-proxy`, `cloud-tasks`, login, etc. The npm `codex-cli/bin/codex.js:1` simply selects the right `@openai/codex-{linux,darwin,win32}-{x64,arm64}` prebuilt and execs it — no JS runtime in the hot path.
- Process model: Single primary `codex` process. Sandboxed shell tool calls spawn child processes wrapped in **Seatbelt (`sandbox-exec`) on macOS, bubblewrap+landlock on Linux, restricted-token on Windows**. Sub-agent spawning is **in-process (threads + tokio tasks), not subprocess** — see `core/src/agent/registry.rs:1` and `core/src/agent/control.rs:1` (`AgentRegistry`, `Mailbox`, `SpawnAgentForkMode`).
- Session lifecycle: Sessions ("threads") created in `codex-core` and persisted as JSONL rollouts via `RolloutRecorder` (`codex-rs/rollout/src/recorder.rs:1`) plus a SQLite state DB (`rollout/src/state_db.rs:1`). Resumed via `codex resume [SESSION_ID]` — `codex-rs/tui/src/cli.rs:18` shows `resume_session_id`, `resume_picker`, `resume_last`, plus a fork mode.
- Transports: The agent core speaks an SQ/EQ Submission/Event protocol (`codex-rs/protocol/src/protocol.rs:1`). The **app-server** transports are pluggable: `stdio://` (default), `ws://IP:PORT`, or `off` (`codex-rs/app-server/src/main.rs:18`); message envelope is JSON-RPC. The **mcp-server** binary speaks rmcp JSON-RPC over stdio.
- Streaming: `reqwest` with `stream` feature against OpenAI Responses API (no `async-openai`; OpenAI client is hand-rolled in `core/src/client.rs:1`). SSE-style streaming consumed and re-emitted as `Event`s on the EQ channel; tokio + `tokio-stream` underpin everything.

##### codex — Major dependencies & frameworks
- Rust crates: `tokio` 1.x, `ratatui` (pinned to a `nornagon/ratatui` git fork), `crossterm` w/ `event-stream` & `bracketed-paste`, `reqwest` 0.12, `serde`/`serde_with`/`serde_json`, `axum` 0.8 (used in `responses-api-proxy`/app-server WS), `sqlx` 0.8 (state DB), **`landlock` 0.4, `seccompiler` 0.5**, `libc`, `ts-rs` + `schemars` for cross-language typing, `tokio-tungstenite` (forked). **No `async-openai`** — custom OpenAI client.
- SDKs: `sdk/python/` (`codex-app-server-sdk`, `pydantic>=2.12`) — auto-generated models for the JSON-RPC app-server; `sdk/typescript/` (`@openai/codex-sdk`, ESM, built with tsup) — wraps the `codex` binary as a child process and exposes `Codex.startThread()` / `resumeThread()`. Both SDKs are clients of the `app-server` JSON-RPC surface, not reimplementations of agent logic.
- Bazel: Builds the entire Rust + system-toolchain stack hermetically (vendored LLVM, macOS SDK pulled from Apple CDN, MinGW-LLVM patched for Windows-gnullvm), runs `just argument-comment-lint`, manages `MODULE.bazel.lock` in CI. Cargo can build everything locally; **Bazel is the reproducible release path**.

##### codex — Security model
- Sandboxing: First-class crates `codex-rs/sandboxing/` and `codex-rs/linux-sandbox/`. `SandboxType` enumerates `MacosSeatbelt`, `LinuxSeccomp`, `WindowsRestrictedToken`, `None` (`sandboxing/src/manager.rs:24`). macOS uses `/usr/bin/sandbox-exec` with `.sbpl` policies (`sandboxing/src/seatbelt_base_policy.sbpl`, `seatbelt_network_policy.sbpl`). Linux uses bubblewrap + Landlock + seccomp (`sandboxing/src/bwrap.rs`, `landlock.rs`; `linux-sandbox/src/{bwrap,landlock,launcher,vendored_bwrap}.rs`). Windows has its own `windows-sandbox-rs` crate. Network policy gating in `protocol/src/network_policy.rs`.
- Approval gates: `AskForApproval` enum in `codex-rs/protocol/src/protocol.rs:826` — variants `UnlessTrusted`, `OnFailure` (deprecated), `OnRequest` (default), `Granular(GranularApprovalConfig)`, `Never`. Routed through a "guardian" review path in `codex_delegate.rs`. Per-tool approval flows live in `mcp-server/src/{exec_approval,patch_approval}.rs`.
- Secrets / supply-chain: dedicated `codex-rs/secrets/` and `codex-rs/keyring-store/` crates; login flows in `codex-rs/login/` (ChatGPT OAuth + API key + device-code). `MODULE.bazel.lock`, `Cargo.lock`, `pnpm-lock.yaml`, `flake.lock` all committed. `third_party/` vendors meriyah, v8, wezterm. `cargo-deny` config at `codex-rs/deny.toml`. `process-hardening` crate at workspace level.
- Notable: AGENTS.md explicitly forbids modifying `CODEX_SANDBOX_*` env vars; tests use `CODEX_SANDBOX=seatbelt` to early-exit when running under sandbox.

##### codex — Agent orchestration
- Multi-agent: Real **in-process sub-agent** spawning via `AgentRegistry` (`core/src/agent/registry.rs:25`) with `AgentMetadata`, depth-limited (`exceeds_thread_spawn_depth_limit`), nicknamed agents picked from an embedded `agent_names.txt`. Sub-agents fork from the parent thread (`SpawnAgentForkMode::FullHistory` or `LastNTurns(usize)`), inheriting provider/approval/sandbox/cwd. Inter-agent comm via `Mailbox`/`MailboxReceiver` (tokio mpsc + watch — `core/src/agent/mailbox.rs:1`). Tool surface lives in `core/src/tools/handlers/multi_agents.rs` and `multi_agents_v2/spawn.rs`.
- Tool dispatch: All in `codex-core` — handlers under `core/src/tools/handlers/` cover `apply_patch`, `js_repl`, `list_dir`, `grep_files`, `mcp`, `mcp_resource`, `multi_agents`, `agent_jobs`, `dynamic`. Each implements `ToolHandler` trait with `ToolKind`. Apply-patch logic is its own `codex-apply-patch` crate.
- MCP: Two-sided. **Inbound** (Codex *as* MCP server): `codex-rs/mcp-server/` exposes Codex tools to other MCP clients via rmcp over stdio (`mcp-server/src/lib.rs:1`). **Outbound** (Codex *as* MCP client): `codex-rs/codex-mcp/` and `codex-rs/rmcp-client/` manage connections to user-configured MCP servers via `McpConnectionManager` (`codex-mcp/src/mcp_connection_manager.rs`), with OAuth scope discovery and per-server tool-name qualification.
- Memory/context: Compaction in `core/src/compact.rs` + `compact_remote.rs`. Context manager in `core/src/context_manager/`. Memory roots, `agents.md` ingestion (`core/src/agents_md.rs`), AGENTS.md scoping. Persistent thread store in `codex-rs/thread-store/`.
- Plan vs execute: `Plan` tool defined in `protocol/src/plan_tool.rs` (`UpdatePlanArgs`); plans are an emitted item type, not a separate executor — execution flows through the same agent loop with plan-update events surfaced to the TUI.

##### codex — Key file references
- `codex-rs/cli/src/main.rs:1` — multitool entry, dispatches to TUI/exec/app-server/mcp-server.
- `codex-rs/Cargo.toml:1` — 88-crate workspace manifest.
- `codex-cli/bin/codex.js:1` — npm wrapper that selects platform binary.
- `codex-rs/tui/src/cli.rs:1` — TUI args incl. resume/fork.
- `codex-rs/protocol/src/protocol.rs:1` — SQ/EQ submission/event protocol; `AskForApproval` at `:826`.
- `codex-rs/sandboxing/src/manager.rs:24` — `SandboxType` enum (Seatbelt/Seccomp/WindowsRestrictedToken).
- `codex-rs/sandboxing/src/seatbelt_base_policy.sbpl` — macOS sandbox policy.
- `codex-rs/linux-sandbox/src/lib.rs:1` — bwrap+landlock launcher.
- `codex-rs/core/src/agent/registry.rs:25` — `AgentRegistry` (sub-agent supervision).
- `codex-rs/core/src/agent/mailbox.rs:1` — inter-agent mpsc.
- `codex-rs/core/src/tools/handlers/multi_agents.rs:1` — collaboration tool surface.
- `codex-rs/codex-mcp/src/mcp_connection_manager.rs:1` — outbound MCP client.
- `codex-rs/mcp-server/src/lib.rs:1` — Codex-as-MCP-server (rmcp/stdio).
- `codex-rs/app-server/src/main.rs:1` — JSON-RPC app-server (stdio/ws).
- `codex-rs/rollout/src/recorder.rs:1` — JSONL session persistence.
- `sdk/typescript/src/codex.ts:7` — TS SDK wraps the `codex` binary.
- `MODULE.bazel:1` — hermetic Bazel toolchain.
- `AGENTS.md:1` — repo-internal coding rules incl. sandbox env-var policy.

---

#### B.2 gemini-cli

##### gemini-cli — At a glance
- Origin / License: `https://github.com/google-gemini/gemini-cli` (Google) — Apache-2.0 (`LICENSE`, `packages/cli/package.json:5`).
- Language: Node.js >=20, TypeScript, bundled via esbuild.
- Workspace: npm workspaces monorepo with seven packages under `packages/`: `cli`, `core`, **`a2a-server`**, `sdk`, `devtools`, `vscode-ide-companion`, `test-utils`.
- Build: `scripts/build.js`, `scripts/build_binary.js`, `esbuild.config.js`, `Makefile`, Husky pre-commit, Single Executable Application launcher in `sea/sea-launch.cjs`.
- Scale: ~1,869 `.ts/.tsx` files under `packages/`. `package.json` version `0.40.0-nightly.20260414`.

##### gemini-cli — Runtime model
- Entry points: Bundled bin `bundle/gemini.js`; package main `packages/cli/dist/index.js` → source `packages/cli/src/gemini.tsx` (818 lines, the React/Ink interactive entry) and `packages/cli/src/nonInteractiveCli.ts` (541 lines) for headless mode. SEA-shipped binary boots through `sea/sea-launch.cjs:7-25`.
- Process model: Single Node CLI process; tools execute in-process or via spawned children (`@xterm/headless`, `execa`). SEA distribution via `scripts/build_binary.js`. Optional sandbox isolates tool exec.
- Session lifecycle: Driven by `GeminiClient` (`packages/core/src/core/client.ts:92`) and `GeminiChat` (`packages/core/src/core/geminiChat.ts:245`). `nonInteractiveCliAgentSession.ts` and `acp/acpClient.ts` provide alternate session shells. ACP resume + sandbox-aware filesystem in `packages/cli/src/acp/`.
- Transports: Direct Gemini API via `@google/genai` (`packages/core/src/core/contentGenerator.ts:8-15,314`). MCP client via `@modelcontextprotocol/sdk` (`packages/core/src/tools/mcp-client.ts:7-18`). ACP via `@agentclientprotocol/sdk` (`packages/cli/src/acp/acpClient.ts:56`). **A2A via `@a2a-js/sdk`** in `packages/a2a-server/` and `packages/core/src/agents/a2a-client-manager.ts`.
- Streaming: Async iterators on `generateContentStream` (`packages/core/src/core/geminiChat.ts:659`); `Scheduler` in `packages/core/src/scheduler/scheduler.ts:95` orchestrates streamed turns (939 lines).

##### gemini-cli — Major dependencies & frameworks
- LLM SDK: `@google/genai@1.30.0` (cli + core). Optional local model: `localLiteRtLmClient.ts`.
- TUI lib: Ink (forked `@jrichman/ink@6.6.9` via npm override at `package.json:81`), `ink-gradient`, `ink-spinner`, `chalk`, `highlight.js`, `lowlight`. UI tree under `packages/cli/src/ui/`.
- Test framework: Vitest 3.x across all workspaces; Playwright-style integration tests under `integration-tests/`, plus `evals/`, `memory-tests/`, `perf-tests/` (each with own `vitest.config.ts`).
- Other notable: `@xterm/headless` (PTY emulation for shell tool), `execa`, OpenTelemetry suite (`@opentelemetry/*`, `@google-cloud/logging`, trace exporters).

##### gemini-cli — Security model
- Sandboxing: Pluggable per-OS sandbox managers under `packages/core/src/sandbox/` — Linux (`bwrapArgsBuilder.ts`/`LinuxSandboxManager.ts`), macOS Seatbelt (`MacOsSandboxManager.ts`, `seatbeltArgsBuilder.ts`, `baseProfile.ts`), Windows (`WindowsSandboxManager.ts` + native `GeminiSandbox.cs`). Container modes via `GEMINI_SANDBOX=docker|podman|sandbox-exec|runsc|lxc|true` (`docs/cli/sandbox.md:230`); top-level `Dockerfile` and `scripts/build_sandbox.js`. `.geminiignore` filters files exposed to the agent.
- Approval/permission model: `ApprovalMode` enum at `packages/core/src/policy/types.ts:48-51` — `DEFAULT`, `AUTO_EDIT`, `YOLO`, plus a `PLAN` mode. CLI flags `--yolo|-y` and `--approval-mode`. Per-tool `shouldConfirmExecute` at `packages/core/src/tools/tools.ts:88,187`; persistent allowlist via `ProceedAlwaysAndSave`. `PolicyEngine` (`packages/core/src/policy/policy-engine.ts`, 945 lines) and `SandboxPolicyManager`. `shell-safety.ts` and `topic-policy.ts` provide command/topic gates.
- Secrets: API keys via `apiKeyCredentialStorage.ts`; OAuth flows in `packages/core/src/mcp/` (Google, MCP, SA-impersonation providers). Trusted folder gating in `packages/cli/src/config/trustedFolders.ts`. No `.env.example` checked in `[unverified]`; `dotenv` + `dotenv-expand` are dependencies.
- Supply chain: `package-lock.json` checked in plus `scripts/check-lockfile.js`. `.allstar/branch_protection.yaml` (Google's OSS Allstar policy bot, `action: log`). Husky pre-commit. `.lycheeignore` for link-checker. CI workflows include `eval-pr.yml`, `evals-nightly.yml`, `memory-nightly.yml`, `perf-nightly.yml`, `agent-session-drift-check.yml`, `release-manual.yml`, `pr-rate-limiter.yaml` (no zizmor file detected) `[unverified]`.

##### gemini-cli — Agent orchestration
- Multi-agent: Yes. `packages/core/src/agents/` defines a registry (`registry.ts`) of named agents — `generalist-agent`, `cli-help-agent`, `codebase-investigator`, `memory-manager-agent`, `skill-extraction-agent`, plus browser agents — invoked locally (`local-invocation.ts`, `local-executor.ts`) or remotely over A2A (`remote-invocation.ts`, `a2a-client-manager.ts`). Top-level `agent-scheduler.ts` (93 lines) coordinates concurrency. ACP sessions live in `packages/core/src/agent/agent-session.ts` and `legacy-agent-session.ts`.
- Tool dispatch: Typed `ToolRegistry` at `packages/core/src/tools/tool-registry.ts:231-286`. Built-in tools registered in `packages/core/src/config/config.ts:3609-3658` (Grep/RipGrep, Glob, Edit, WriteFile, WebFetch, Shell, ListBackgroundProcesses, ReadBackgroundOutput, ActivateSkill, MCP resources). `Scheduler` drives the tool-call lifecycle with a confirmation bus (`packages/core/src/confirmation-bus/`).
- MCP: First-class — `mcp-client.ts`, `mcp-client-manager.ts`, `mcp-tool.ts`, `xcode-mcp-fix-transport.ts`, plus OAuth providers in `packages/core/src/mcp/` and a browser-MCP bundle.
- Memory/context: `memoryTool.ts` persists to `GEMINI.md` (`packages/core/src/tools/memoryTool.ts:32`); hierarchical loading; long-running `memory-manager-agent.ts`. `memory-tests/` enforces baselines via `UPDATE_MEMORY_BASELINES`.
- Eval / perf: ~33 eval suites under `evals/` (sandbox recovery, tool masking, skill extraction, plan mode, hierarchical memory, etc.); `perf-tests/`, `memory-tests/`, `integration-tests/` with sandbox=none|docker|podman matrices in `package.json` scripts.

##### gemini-cli — Key file references
- `packages/cli/src/gemini.tsx:1` — Ink interactive entry.
- `packages/cli/src/nonInteractiveCli.ts:1` — headless entry.
- `sea/sea-launch.cjs:7` — Single Executable Application launcher.
- `packages/core/src/core/client.ts:92` — `GeminiClient` session driver.
- `packages/core/src/core/geminiChat.ts:245` — chat/streaming loop.
- `packages/core/src/core/contentGenerator.ts:314` — `@google/genai` integration.
- `packages/core/src/scheduler/scheduler.ts:95` — tool-call scheduler.
- `packages/core/src/tools/tool-registry.ts:231` — typed tool registry.
- `packages/core/src/config/config.ts:3609` — built-in tool registration.
- `packages/core/src/policy/types.ts:48` — `ApprovalMode` enum.
- `packages/core/src/policy/policy-engine.ts:1` — central policy enforcement.
- `packages/core/src/sandbox/linux/LinuxSandboxManager.ts:1` — bwrap-based sandbox.
- `packages/core/src/sandbox/macos/MacOsSandboxManager.ts:1` — Seatbelt sandbox.
- `packages/core/src/tools/mcp-client.ts:7` — MCP transport (stdio + SSE).
- `packages/core/src/agents/registry.ts:1` — sub-agent registry.
- `packages/core/src/tools/memoryTool.ts:32` — `GEMINI.md` memory persistence.
- `packages/cli/src/acp/acpClient.ts:56` — Agent Client Protocol bridge.

---

#### B.3 claudian

##### claudian — At a glance
- Upstream/license: `github.com/YishenTu/claudian`, MIT.
- Primary language: TypeScript, bundled by esbuild (`esbuild.config.mjs`).
- Type: **Obsidian plugin**, desktop-only (`manifest.json:7-9`, `isDesktopOnly: true`, minAppVersion 1.4.5).
- Build/test: esbuild + `tsc --noEmit` typecheck; Jest 30 with `jest-environment-jsdom`.
- Workspace topology: `src/{main.ts, core, features, shared, utils, i18n, style}`, `tests/{unit,integration,__mocks__,helpers}`, `_bmad/` (BMad Method skills checked in: `bmm`, `core`, `_config`, `_memory`), `_bmad-output/{planning,implementation}-artifacts/` (currently empty dirs), `docs/`, `scripts/`, `Preview.png`.
- Scale: ~37,400 lines TS across 212 src files + 135 test files. Single most-loaded class is `ClaudianService` (~1,766 lines).

##### claudian — Runtime model
- Entry point: `src/main.ts:52` (`class ClaudianPlugin extends Plugin`), `onload`/`onunload` at `src/main.ts:62,203`.
- Process model: Despite living inside Obsidian (Electron renderer), claudian does **not** call the Anthropic API itself. It spawns the local **Claude Code CLI** as a child process via `@anthropic-ai/claude-agent-sdk`'s `query()` (`src/core/agent/ClaudianService.ts:328`), with a custom spawn that resolves Node's full path because Obsidian GUI lacks PATH (`src/core/agent/customSpawn.ts:14-47`). **The Obsidian vault root is the CWD.**
- Session lifecycle: Plugin lifecycle hooks register a `WorkspaceLeaf` view (`VIEW_TYPE_CLAUDIAN`, `src/main.ts:80-83`) and ribbon/commands. `onunload` flushes per-tab persisted state. Conversation lifecycle is split between **legacy JSONL** files in `.claude/sessions/*` and **SDK-native** sessions in `~/.claude/projects/{vault}/*.jsonl`.
- Transport: Persistent long-lived `query()` per active tab (warm path), with cold-start queries for inline edit + title generation. Communication with the CLI is over **stdio JSON** (`SDKUserMessage` envelopes built at `src/core/agent/ClaudianService.ts:1046-1092`). A `MessageChannel` (`src/core/agent/MessageChannel.ts`) queues turns between user and the long-running query.
- Streaming: SDK `for await (const message of persistentQuery)` loop in `startResponseConsumer`/`routeMessage` (`src/core/agent/ClaudianService.ts:541-702`); chunks transformed via `transformSDKMessage` and routed to the active `ResponseHandler`. Crash recovery replays the last `SDKUserMessage` (`src/core/agent/ClaudianService.ts:564-583`).

##### claudian — Major dependencies & frameworks
- Anthropic SDK: `@anthropic-ai/claude-agent-sdk` ^0.2.76 (the *agent* SDK, which drives the CLI), not the raw `@anthropic-ai/sdk`. Imports throughout `src/core/agent/`.
- MCP: `@modelcontextprotocol/sdk` ~1.25.3, used by `src/core/mcp/{McpServerManager,McpTester}.ts`.
- Obsidian API: dev-dep `obsidian: latest`, used pervasively (Plugin, Notice, MarkdownView, Editor, ItemView).
- UI: native Obsidian DOM (no React/Svelte); custom modular CSS in `src/style/{base,toolbar,...}` built via `scripts/build-css.mjs`. i18n is custom JSON-per-locale across 10 locales.

##### claudian — Security model
- Threat model: Plugin runs with full Obsidian/Node privileges, but the CLI's tool calls are sandboxed back to the vault. Two `PreToolUse` SDK hooks enforce policy: a **command blocklist** (`createBlocklistHook`, `src/core/hooks/SecurityHooks.ts:30-59`) targeting `Bash`, and a **vault restriction** hook (`createVaultRestrictionHook`, `SecurityHooks.ts:64-143`) that rejects `Read/Edit/Write/NotebookEdit/Glob/Grep` outside the vault and inspects `Bash` command strings for path escapes (`src/core/security/BashPathValidator.ts`). Export paths are write-only; "external context" paths are read/write; vault is full access.
- Permission/approval: SDK is started with `allowDangerouslySkipPermissions: true` so modes can be hot-swapped; `canUseTool` callback in `createApprovalCallback` (`src/core/agent/ClaudianService.ts:1661-1759`) prompts the user via the chat UI for `allow / allow-always / deny / cancel`. Modes: `normal`, `acceptEdits`, `plan`, `yolo` (→ SDK `bypassPermissions`). Plan mode is ephemeral (reset on load, `src/main.ts:252-254`). Dedicated callbacks for `ExitPlanMode` and `AskUserQuestion`.
- Secrets: No API key handled by the plugin — auth is the user's local `claude` CLI login. `.env.local.example` only documents `OBSIDIAN_VAULT` (dev copy path). Settings live in `.claude/settings.json` (CC-compatible permissions/env), `.claude/claudian-settings.json`, `.claude/settings.local.json` (gitignored), and `.claude/mcp.json`.
- Supply chain: `package-lock.json` checked in. No husky/pre-commit hooks visible. ESLint + simple-import-sort. `.github/` exists but no detail captured `[unverified]` for release automation.

##### claudian — Agent orchestration
- Multi-tab chat + multi-agent: Each Claudian view contains a `TabManager`; each tab has its own `ClaudianService` instance + persistent SDK query. **Sub-agents (`Task` tool) are tracked in `subagentData`**, with async sub-agent recovery from JSONL and a `SubagentManager` service.
- Tool dispatch: Tools come from the Claude Code CLI (Bash/Read/Write/Edit/Grep/Glob/NotebookEdit/Task/WebFetch/etc., enumerated in `src/core/tools/toolNames.ts`). Allowed-tool gating is enforced both via SDK options and `canUseTool`. Some SDK tools are explicitly disabled (`UNSUPPORTED_SDK_TOOLS`).
- MCP: First-class. `McpServerManager` loads from `.claude/mcp.json`, dynamic `setMcpServers` updates without restart; `@`-mentions in prompt enable specific servers per-turn.
- Memory/context: Persistent SDK sessions store full message history server-side under `~/.claude/projects/{vault}/*.jsonl`; the plugin loads/dedupes/merges on demand. On session mismatch/expiry, history is rebuilt from in-memory messages and re-injected. Slash commands, agents, and skills are loaded from `.claude/{commands,agents,skills}/`.
- Vault/file integration: Vault path is the CLI's `cwd`, so the agent reads/writes notes directly. `@`-mention selector pulls notes into context. Inline edit modal lets the agent rewrite a selection in place. External directories outside the vault can be granted via `allowedExportPaths` / external context paths.

##### claudian — Key file references
- `manifest.json:1-10` — plugin manifest (desktop-only).
- `src/main.ts:52-201` — `ClaudianPlugin` entry, lifecycle, commands, view/ribbon registration.
- `src/core/agent/ClaudianService.ts:328-340` — SDK `agentQuery({ prompt: messageChannel, options })` start of persistent query.
- `src/core/agent/ClaudianService.ts:1661-1759` — `createApprovalCallback`.
- `src/core/agent/customSpawn.ts:14-47` — custom Node-resolving spawn for the CLI child process.
- `src/core/agent/QueryOptionsBuilder.ts:1-80` — SDK `Options` construction.
- `src/core/hooks/SecurityHooks.ts:30-143` — Blocklist + vault-restriction PreToolUse hooks.
- `src/core/security/BashPathValidator.ts` — bash command path-escape detector.
- `src/core/storage/StorageService.ts` + `SessionStorage.ts` — `.claude/{settings,sessions,mcp}` persistence.
- `src/core/mcp/McpServerManager.ts` — MCP server registry/runtime.
- `src/features/chat/ClaudianView.ts` — sidebar view + TabManager bootstrap.
- `src/features/inline-edit/InlineEditService.ts` + `ui/InlineEditModal.ts` — inline edit cold-start path.
- `src/utils/sdkSession.ts` — reads/writes `~/.claude/projects/{vault}/*.jsonl`.
- `CLAUDE.md` — author's own architecture overview.

Notable oddity: `_bmad/` ships the BMad Method skill packs checked into the plugin repo, but `_bmad-output/{planning,implementation}-artifacts/` are empty directories — meta-tooling staged for future use rather than active artifacts.

---

#### B.4 opencode

##### opencode — At a glance
- Origin: Fork of `sst/opencode`. Repo `repository.url` is `https://github.com/anomalyco/opencode` (`package.json:108`). LICENSE is MIT, dated 2025. SST tooling and the `sst.config.ts` (`home: "cloudflare"`) are upstream-original. Recent commits include sync-release versions e.g. `1.14.18` (`packages/opencode/package.json:3`).
- Primary languages: **TypeScript on Bun** (`packageManager: "bun@1.3.11"`). No Go TUI in this fork — TUI is **Solid.js on `@opentui/solid`** rendered to terminal (`packages/opencode/src/cli/cmd/tui/app.tsx:1`).
- Build: Bun for runtime, **SST 3.18.10** for cloud infra, **Turborepo 2.8.13** for monorepo orchestration. Per-package build via `script/build.ts`. Nix flake present.
- Workspace topology: 19 packages under `packages/` (`opencode`, `tui`-equivalent embedded in `opencode/cli/cmd/tui`, `app`, `console`, `containers`, `desktop`, `desktop-electron`, `docs`, `enterprise`, `extensions`, `function`, `identity`, `plugin`, `script`, `sdk/js`, `shared`, `slack`, `storybook`, `ui`, `web`); plus `infra/`, `sdks/vscode`, `specs/`, `script/`, `install`, `github/`, `nix/`, `.opencode/` (themes/agents/skills/commands), `.zed/`.
- Scale: ~416 TS files in `packages/opencode/` alone. Heavy plugin/skill ecosystem under `.opencode/`.

##### opencode — Runtime model
- Main entry: CLI is `packages/opencode/src/index.ts:1` (yargs dispatcher) → bin `./bin/opencode`. Subcommands include `serve`, `run`, `tui`, `acp`, `mcp`, `agent`, `pr`, `github`, `import`, `export`, `web`, `db`, `session`, `debug`.
- Process model: **Headless Hono server (`packages/opencode/src/server/server.ts:36`) runs as backend; `serve` boots it. TUI is a separate Solid.js process backed by `@opentui/core` that talks to the server via the SDK** (`cli/cmd/tui/app.tsx:5`). `run` issues a one-shot prompt against the same server using `createOpencodeClient`. An ACP entry exposes opencode as an ACP agent for editor integrations.
- Session lifecycle: Sessions are server-managed and persisted via Drizzle (Bun SQLite) — `packages/opencode/src/session/session.sql.ts`, schema in `session/schema.ts`. A `v2/session.ts` layer wraps them in Effect schemas. Reconnection works via SDK + SSE stream replay.
- Transports: HTTP (Hono on Bun, with a node-server adapter); SSE for events; WebSocket via `upgradeWebSocket` for PTY/IDE; mDNS discovery (`server/mdns.ts`); MCP with all three transports — stdio, SSE, StreamableHTTP (`mcp/index.ts:3-5`); ACP via `@agentclientprotocol/sdk`.
- Streaming: SSE for `/event` (Bus-backed, with 10s heartbeat). WebSocket for PTY. Vercel AI SDK `streamText`/`streamObject` for LLM token streams.

##### opencode — Major dependencies & frameworks
- LLM SDKs: **Vercel AI SDK 6** (`ai: 6.0.168`) plus 18 `@ai-sdk/*` providers — anthropic, openai, openai-compatible, google, google-vertex, bedrock, azure, cerebras, cohere, deepinfra, gateway, groq, mistral, perplexity, togetherai, xai, alibaba, vercel — plus `@openrouter/ai-sdk-provider`, `gitlab-ai-provider`, `venice-ai-sdk-provider`, `ai-gateway-provider`.
- TUI lib: `@opentui/core` + `@opentui/solid` rendered with Solid.js (no React/Ink, no Go BubbleTea). Uses `node-pty` (`@lydell/node-pty`) for embedded shells.
- SST infra: Cloudflare-based (`sst.config.ts:9`). Deploys: API Worker (`infra/app.ts`), `console` app, `enterprise` stack, R2 bucket, durable-object `SyncServer`, Astro docs site, integrates Stripe + Planetscale providers. **Backend hosts auth/sync/billing for the cloud product, not the local agent itself.**
- Other key: Effect (`effect: 4.0.0-beta.48`) for typed runtime/DI; Hono + hono-openapi for HTTP; Drizzle ORM; `@modelcontextprotocol/sdk@1.27.1`; `@agentclientprotocol/sdk@0.16.1`; OpenTelemetry tracing; tree-sitter (bash + powershell) for shell parsing.

##### opencode — Security model
- Sandboxing: **None at OS level.** Bash tool spawns commands directly via Effect `ChildProcessSpawner` (`tool/bash.ts:334,445`); a `BLACKLIST` excludes `fish`/`nu` (`shell/shell.ts:11`). Tree-sitter is used to parse bash/powershell to *classify* commands for permission prompts (`tool/bash.ts:10,28-50`), not to sandbox them.
- Permission model: Centralized rule engine in `permission/index.ts` with `allow`/`deny`/`ask` actions (`permission/index.ts:20`), pattern rulesets, persisted approvals, and a Bus event `permission.asked`/`replied`. Each tool calls `ctx.ask(...)` — bash asks for `bash` and `external_directory` permissions; write/edit ask for `edit`. Bash arity classification in `permission/arity.ts` decides scope of approval.
- Secrets: SST `Secret` resources in `infra/app.ts`, `infra/secret.ts` for cloud side. CLI uses `@openauthjs/openauth`, MCP OAuth provider, `OPENCODE_SERVER_PASSWORD` flag for HTTP server hardening.
- Supply chain: `bun.lock` (~880 KB), `husky` pre-commit, `oxlint`+`prettier`, `patches/` directory pinning two deps, `trustedDependencies` allowlist.

##### opencode — Agent orchestration
- Multi-agent: Supports primary agents and sub-agents via the `Task` tool. Agent registry in `agent/agent.ts:27` defines `mode: "subagent" | "primary" | "all"`, per-agent `permission` Ruleset, model binding, prompt overrides. Agents loadable from `.opencode/agent/`.
- Tool dispatch: Schema-driven Effect Service `ToolRegistry` (`tool/registry.ts:68`) merges builtin + custom + plugin tools and filters per-model in `tools(model)`. All tools share the `Tool.Def` Zod-schema contract. Builtin set: bash, edit, multiedit, glob, grep, read, write, todo, task, webfetch, websearch, codesearch, lsp, plan, question, skill, apply_patch, mcp-exa.
- MCP integration: First-class. `@modelcontextprotocol/sdk@1.27.1` clients — stdio/SSE/StreamableHTTP — managed in `mcp/index.ts`, dynamic tools registered via `dynamicTool` from `ai`. OAuth flow built in.
- Memory/context: Sessions persisted in SQLite (Drizzle); see `session/session.sql.ts`. `session/compaction.ts`, `session/overflow.ts`, `session/summary.ts` handle context-window management. Top-level `session.json` is empty (0 bytes) — likely a placeholder for the share/CLI ID. `STATS.md` is purely a download-stats log.
- Provider abstraction: `provider/provider.ts` unifies all `@ai-sdk/*` SDKs behind a `LanguageModelV3` interface, fetches model catalog from `models.dev`, supports auth via `Auth`, with overrides like Copilot Responses API for GPT-5+ (`provider/provider.ts:34`). Plugin system can inject providers (`Plugin.Service`).

##### opencode — Key file references
- `packages/opencode/src/index.ts:1` — CLI entry, yargs dispatch
- `packages/opencode/src/server/server.ts:36` — Hono server factory
- `packages/opencode/src/server/routes/instance/event.ts:39` — SSE event stream
- `packages/opencode/src/cli/cmd/serve.ts:11` — headless server bootstrap
- `packages/opencode/src/cli/cmd/run.ts:10` — one-shot prompt over SDK
- `packages/opencode/src/cli/cmd/tui/app.tsx:1` — Solid/OpenTUI client
- `packages/opencode/src/acp/agent.ts:32` — ACP server (editor protocol)
- `packages/opencode/src/tool/registry.ts:65` — tool registry/dispatch
- `packages/opencode/src/tool/bash.ts:267` — permission ask in bash tool
- `packages/opencode/src/permission/index.ts:20` — allow/deny/ask engine
- `packages/opencode/src/mcp/index.ts:3-11` — MCP client (stdio/SSE/HTTP)
- `packages/opencode/src/provider/provider.ts:1-32` — multi-provider abstraction
- `packages/opencode/src/agent/agent.ts:27` — agent schema (primary/subagent)
- `packages/opencode/src/session/session.sql.ts` — SQLite session persistence
- `infra/app.ts:1-60` — SST/Cloudflare deployment topology
- `sst.config.ts:9` — Cloudflare home + Stripe/Planetscale providers

---

#### B.5 rustain

##### rustain — At a glance
- Origin: local-only, no remote (branch `prd`).
- Primary language: Rust (single-crate workspace, edition 2024, MSRV 1.85). One bin (`rustain`) + lib (`src/lib.rs`) declared in `Cargo.toml:10-16`.
- Build: Cargo + `build.rs` injects `GIT_HASH`, `BUILD_DATE`, `TARGET` env vars at compile time.
- Workspace topology: **hexagonal `src/{domain,adapters,infrastructure}`** (see `CLAUDE.md:22-52`), Rust integration tests in `tests/`, Python TUI tests in `tests_tui/` (pyproject + pytest, `tests_tui/harness.py`), perf harness in `perf-tests/`, BMAD planning artifacts in `_bmad-output/planning-artifacts/`, runtime data in `~/.rustain/` and project-local `.rustain/{config,permissions,state}.toml`.
- Scale: 172 `.rs` files in `src/`; ~30 domain models, 14 ports, 25 services; 33+ Rust integration test files; graphify reports 5354 nodes / 15722 edges across 62 communities.

##### rustain — Runtime model
- Entry: `src/main.rs:17` is a thin `#[tokio::main]` shell delegating to `infrastructure::startup::run()`. Library re-export at `src/lib.rs:14-16`.
- Process model: **single-process TUI binary (ratatui+crossterm)**. CLI subcommands `init`, `doctor`, `migrate` short-circuit before TUI; root args `--new`, `--session`, `--snapshot-retention`.
- Session lifecycle: ordered startup sequence (CLI → config → logging → panic hook → provider → terminal → event loop, `src/infrastructure/startup.rs:41-119`). `~/.rustain/` (override `RUSTAIN_DATA_DIR`) holds rolling daily logs `rustain.log.YYYY-MM-DD`; project-local `.rustain/state.toml` tracks `session_count`. Conversations persist via `StoragePort` with per-conversation snapshot dirs.
- Transport: **direct provider HTTP via `reqwest`** (rustls-TLS). **No stdio JSON-RPC, MCP, or ACP wired today** — version banner advertises `protocols: agent-skills n/a, mcp n/a, a2a n/a` (`src/adapters/tui/version_info.rs:12`). MCP/A2A appear only as planned `CapabilityProvider` implementations and as namespace conventions.
- Streaming: tokio multi-thread runtime; SSE parser at `src/adapters/anthropic/sse.rs`; unified `StreamingProvider::stream_completion` returns a `BoxStream<StreamChunk>`. The runtime is a 4-branch `tokio::select!` event loop with explicit deadlock invariants.

##### rustain — Major dependencies & frameworks
- Async: `tokio` 1.50, `tokio-stream`, `tokio-util`, `futures`. TUI: `ratatui` 0.30 + `crossterm` 0.29. CLI: `clap` 4.5 derive. Serde+toml+serde_json+chrono+pulldown-cmark+globset+sha2+nanoid+petname. Errors: `thiserror`+`anyhow`. Logging: `tracing` + `tracing-appender` daily rotation. HTTP: `reqwest` (feature-gated, rustls). Hot-swap: `arc-swap`. Clipboard: `arboard`+`png` (feature `clipboard`). Dev: `insta` (TUI snapshots), `mockito`, `procfs`, `serial_test`, `pretty_assertions`.
- LLM provider SDKs: **none — own SSE parser**. Runners (`run-anth-bin.nu`, `run-zai-bin.nu`, `run-ds-bin.nu`, `run-or-bin.nu`) all swap `ANTHROPIC_BASE_URL` + key/token to point the same Anthropic adapter at Anthropic, Z.AI (`api.z.ai/api/anthropic`), DeepSeek (`api.deepseek.com/anthropic`), and OpenRouter — the gateways speak Anthropic-compatible wire format. `AuthMode::ApiKey` vs `AuthMode::BearerToken` selects header style.
- Provider abstraction: `StreamingProvider` trait is the single port (`src/domain/ports/provider.rs:28-61`). `ProviderRegistry` catalogues adapters (`src/adapters/provider/registry.rs:22-38`); `arc_swap::ArcSwap` holds the live provider for runtime hot-swap. Cargo features: `anthropic` (default), `openai`, `clipboard`, `skills-validation`; `src/adapters/openai/mod.rs` exists, `src/adapters/kimi/` is empty.

##### rustain — Security model
- Sandboxing: **none — direct host execution**. `SecurityAdapter` enforces a hard-coded blocklist of dangerous commands (`rm -rf /`, `dd if=/dev/zero`, fork-bomb, `sudo rm`...) and blocked-path prefixes (`/etc/`, `/usr/`, `/sys/`, `/root/`...) at `src/adapters/security_adapter.rs:32-53`, with regex-style suspicious patterns. `SandboxPolicy` exists as a domain model `[unverified — depth not read]`.
- Permission/approval model: `PermissionChain` (pure domain) returns `Allow | Deny | Prompt`. `ApprovalRuntime` brokers prompts via `tokio::sync::broadcast` + per-request `oneshot`; **ToolCall 7-state FSM** (Validating → Scheduled → AwaitingApproval → Executing → Success/Error/Cancelled) with parallel batching when `parallel_safe`. Modes: `Normal`, plan mode (auto-allow safe tools), full-yolo override. Project `.rustain/permissions.toml` defines `always_tools` allowlist.
- Secrets: `.env` and process env vars (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`); `AnthropicAdapter` manually masks credentials in `Debug`. **Note: live keys are committed to `.env` and `run-*.nu` files — supply-chain hygiene gap.**
- Supply chain: committed `Cargo.lock`; GitHub Actions `.github/workflows/ci.yml` runs `cargo clippy -D warnings`, `cargo fmt --check`, **a regex grep guarding the event-loop deadlock invariant**, and `cargo test`. No pre-commit framework detected; release profile uses `lto`, `strip`, `panic=abort`.

##### rustain — Agent orchestration
- Single-process agent today; ownership topology (Owned/Peer/Self spawn tree, "One Ring") is designed but tools beyond shell/file are pending.
- Tool dispatch: `ToolSetAdapter` implements `ToolSetPort` with Bash/Read/Write/Edit and per-turn checkpoint snapshots, write-path mutex map, and stdout-tail progress events. Skills/commands/palettes have dedicated registries.
- MCP/ACP/A2A: not implemented — only referenced as future `CapabilityProvider` impls. Banner says `n/a` for all three.
- Memory/context: per-day `~/.rustain/rustain.log.YYYY-MM-DD` traces; conversation persistence via `StoragePort`+`SessionIndex`; skill activation depth-limited; project context loader at `src/adapters/project_context_loader.rs`.
- BMAD: rustain itself is built using BMAD — `_bmad-output/planning-artifacts/` and `docs/ARCHITECTURE_REVIEW_PLAN_MODE_ORCHESTRATION.md` host PRDs/stories that the code references explicitly. **It is BMAD-as-process, not first-class BMAD workflow runtime integration.**
- Top-level meta-configs: `.claude/agents/code-reviewer.md` + `.claude/commands/{check,review,deploy}` (Claude Code subagent config); `.opencode/opencode.json` only loads the graphify plugin; `.agents/skills/reviewer` (Agent Skills convention); `.rustain/{config,permissions,state}.toml` for the binary itself.

##### rustain — Key file references
- `src/main.rs:17` — `#[tokio::main]` entry, defers to startup.
- `src/infrastructure/startup.rs:41-119` — ordered boot.
- `src/infrastructure/runtime/event_loop.rs:1-20` — 4-branch `tokio::select!` loop, dual-channel EventBus invariant.
- `src/infrastructure/runtime/agent_core.rs` — central runtime orchestrator.
- `src/domain/ports/provider.rs:28-61` — `StreamingProvider` trait.
- `src/adapters/anthropic/mod.rs:36-45` + `sse.rs` + `stream.rs` + `types.rs` — Anthropic SSE adapter.
- `src/adapters/provider/registry.rs:22-38` — `ProviderRegistry`.
- `src/domain/services/tool_scheduler.rs` — 7-state ToolCall FSM, parallel batching with `FuturesOrdered`.
- `src/adapters/toolset_adapter.rs:40-60` — Bash/Read/Write tool execution + checkpoint/snapshot wiring.
- `src/domain/services/permission_chain.rs:1-22` — `Allow | Deny | Prompt` decision.
- `src/domain/services/approval_runtime.rs` — broadcast+oneshot approval pub/sub.
- `src/adapters/security_adapter.rs:14-62` — blocklist, workspace boundaries.
- `src/adapters/tui/widgets/` (30+ widgets incl. `plan_approval.rs`, `permission_prompt.rs`, `task_panel.rs`, `command_palette.rs`).
- `src/infrastructure/paths.rs:5-17` + `src/infrastructure/logging.rs:16-33` — `~/.rustain/`, daily-rotated log.
- `.github/workflows/ci.yml:27-33` — event-loop deadlock invariant grep gate.

## Integration Patterns

This section extracts cross-cutting transport, protocol, and provider-abstraction patterns from the per-project snapshots above. Where the same protocol is used across projects, the implementation differences are interesting in their own right.

### Protocol matrix

Three open agent protocols dominate the space, and the nine projects partition cleanly across server/client/none roles for each.

| Project | MCP server | MCP client | ACP server | ACP client | A2A | Custom protocol |
|---|---|---|---|---|---|---|
| openclaw | ✅ stdio + HTTP (curated tools subset) | ✅ via `extensions/mcp` and `src/mcp/channel-server.ts` | ✅ stdio NDJSON (`src/acp/server.ts` + 1.4k-line translator) | — | ❌ (uses `sessions_spawn` + `agent-step.ts` instead) | ✅ WebSocket gateway control plane |
| ironclaw | ❌ | ✅ `src/tools/mcp/{stdio,http,unix}_transport.rs` | ❌ | ✅ as **bridge spawner** — workers launch Goose/Codex/Gemini-CLI as ACP child processes (`src/worker/acp_bridge.rs`) | ❌ | ✅ axum HTTP orchestrator API (port 50051) + WS |
| hermes-agent | ✅ FastMCP, exposes gateway as 10 MCP tools (`mcp_serve.py`) | ✅ optional `mcp` extra | ✅ `acp_adapter/server.py` thread-pool agent | — | ❌ | ✅ gateway REST/webhooks per platform |
| paperclip | ❌ `[unverified — only one mcp grep hit]` | ❌ | ❌ | ❌ | ❌ | ✅ REST + WebSocket internal; **adapter stdout stream-JSON parsing** as the de facto agent protocol |
| codex | ✅ `codex-rs/mcp-server` (rmcp/stdio) | ✅ `codex-rs/codex-mcp` + `rmcp-client` w/ OAuth | ❌ | ❌ | ❌ | ✅ **SQ/EQ Submission/Event protocol** + JSON-RPC `app-server` (stdio + WS pluggable) |
| gemini-cli | ❌ | ✅ `mcp-client.ts` (stdio + SSE + StreamableHTTP) + Google/MCP/SA OAuth | — | ✅ `packages/cli/src/acp/acpClient.ts` (sandbox-aware FS) | ✅ **only project with a2a-js** — full `packages/a2a-server/` + `core/agents/a2a-client-manager.ts` | — |
| claudian | ❌ | ✅ via `@modelcontextprotocol/sdk@~1.25.3`; loads `.claude/mcp.json`; `setMcpServers` hot-swap | ❌ | ❌ | ❌ | ✅ stdio JSON envelopes (`SDKUserMessage`) to local Claude Code CLI |
| opencode | ✅ `mcp` subcommand exists [unverified — likely server-side too] | ✅ all three transports (stdio + SSE + StreamableHTTP) + OAuth provider | ✅ `packages/opencode/src/acp/agent.ts` via `@agentclientprotocol/sdk@0.16.1` | — | ❌ | ✅ Hono HTTP + SSE event stream + WS for PTY/IDE; mDNS discovery |
| rustain | ❌ planned | ❌ planned | ❌ planned | ❌ planned | ❌ planned | — (banner literally says `protocols: agent-skills n/a, mcp n/a, a2a n/a`) |

**Counts:**
- MCP client: 7/9 (everyone except paperclip and rustain)
- MCP server: 4/9 (openclaw, hermes, codex, opencode-likely)
- ACP server: 4/9 (openclaw, hermes, opencode, plus codex's `mcp-server` is conceptually adjacent)
- ACP client/bridge: 2/9 (ironclaw, gemini-cli)
- A2A: **1/9 (only gemini-cli)**

### Observation: the "MCP everywhere, ACP for IDEs, A2A barely anywhere" pattern

Across the cohort, MCP is the de facto winner for **tool-side** integration (8/9 if you count opencode's likely server). ACP is overwhelmingly used as **the editor-side IDE bridge** — every ACP server in the matrix targets the same workflow (Zed, custom editors, in-editor chat panels).

A2A appears in **exactly one** project (gemini-cli), where it is wired to a dedicated `a2a-server` package and a `core/agents/a2a-client-manager.ts` for cross-host agent calls. The fact that everyone else either implements proprietary A2A-equivalents (openclaw's `sessions_spawn` + provenance, codex's in-process `Mailbox`, hermes's `delegate_task`, opencode's `Task` tool, rustain's planned "Owned/Peer/Self" topology) or just doesn't do A2A at all suggests the protocol hasn't crossed the chasm yet.

### Transport choices, by use case

| Use case | Transport choice across projects |
|---|---|
| **Editor ↔ agent (IDE bridge)** | Stdio JSON-RPC (NDJSON) via `@agentclientprotocol/sdk` (openclaw, gemini-cli, opencode, hermes); codex uses its own JSON-RPC over stdio with the `app-server` |
| **Agent ↔ tool servers (MCP)** | Stdio for local servers (universal); HTTP/SSE for hosted (codex, opencode, gemini-cli); StreamableHTTP for resumability (opencode, gemini-cli) |
| **Provider streaming** | SSE for nearly everyone — directly via `reqwest` stream (codex, ironclaw via rig-core, rustain own parser); via SDK abstractions (hermes via `openai`/`anthropic` SDKs, opencode via Vercel AI SDK, gemini via `@google/genai`); via Anthropic SDK proxy (claudian via `claude-agent-sdk`) |
| **Server ↔ TUI (split-process tools)** | opencode is the cleanest: Hono HTTP + SSE event stream + WS for PTY. paperclip uses WebSocket for live events. ironclaw uses axum for orchestrator/worker. claudian uses an in-process `MessageChannel`. |
| **Chat-platform ingress** | openclaw and hermes-agent both fan out across 18+ messenger platforms (WhatsApp, Telegram, Slack, Discord, Signal, iMessage, Matrix, …); ironclaw does WASM-sandboxed Discord/Feishu/Slack/Telegram/WhatsApp. The five coding tools have no such surface. |
| **Cron / scheduled tasks** | openclaw `croner`, hermes `cron/scheduler.py` with fcntl/msvcrt file lock, ironclaw `cron = "0.13"` + `Routine` engine, paperclip `routineService` + heartbeat, opencode `function/` deploys; codex/gemini-cli/claudian/rustain don't ship cron |

### Provider abstraction — six distinct strategies

This is where the deepest divergence shows up.

1. **Multi-provider SDK federation (Vercel AI SDK)** — opencode. 18+ `@ai-sdk/*` providers behind `LanguageModelV3`, dynamic catalog from `models.dev`, override hooks for things like Copilot Responses-API for GPT-5+. Lowest friction to add a provider; coupling to AI SDK release cadence.
2. **Multi-provider Rust trait crate (rig-core)** — ironclaw. `rig-core = "0.30"` + a thick local `RigAdapter` (`src/llm/rig_adapter.rs`) implementing the in-house `LlmProvider` trait. Same federation idea, different language.
3. **Per-provider extension plugin** — openclaw. Each provider lives in its own `extensions/<name>` package with its own auth/catalog/setup-token logic; the agent harness is the unified surface, the SDKs are not. Most modular but higher per-provider maintenance cost.
4. **First-party SDK directly** — gemini-cli (`@google/genai`), hermes-agent (`openai` + `anthropic`, lazy-imported for cold-start), claudian (`@anthropic-ai/claude-agent-sdk`).
5. **Hand-rolled HTTP client against one wire format** — codex (own OpenAI Responses-API client in `core/src/client.rs`); rustain (own SSE parser against the Anthropic wire format, with `AuthMode::ApiKey | BearerToken` to point at Anthropic / Z.AI / DeepSeek / OpenRouter gateways that all speak Anthropic-compat). Notable: rustain's runners (`run-anth-bin.nu`, `run-zai-bin.nu`, …) use the same adapter against four gateway providers because they're all wire-compatible.
6. **No SDK at all — orchestrate external CLIs** — paperclip. The agent itself is a child-process invocation of `@anthropic-ai/claude-code`, `@openai/codex`, `opencode-ai`, etc., shipped via the production Dockerfile. Adapter packages parse stream-JSON from each CLI's stdout. Claudian uses a similar idea but with one CLI (`claude`) and the official agent SDK that wraps spawning.

The "spawn another agent CLI" pattern (#6) is interesting because it cascades: paperclip's agent calls Claude Code, which itself can spawn sub-agents, which themselves can call MCP servers. The supervision tree spans process boundaries that the framework above can only see as opaque stream-JSON.

### Plugin/extension surface — three flavors

- **Native code in the repo** (most permissive, highest trust) — openclaw `extensions/`, hermes `tools/` auto-discovery + `plugins/`, gemini-cli `packages/`, opencode `packages/`, codex's `core/src/tools/handlers/`, rustain's registries.
- **Sandboxed via VM/WASM** (capability model) — paperclip's `plugin-runtime-sandbox.ts` uses Node `vm` + a `CapabilityScopedInvoker`; ironclaw's WIT/WASM tools (`wit/tool.wit`, Wasmtime fuel/memory limits, secrets only via `secret-exists`) is the most rigorous sandbox of any project surveyed.
- **External CLI orchestration** — paperclip adapter packages, claudian's CLI invocation, ironclaw's ACP bridge spawning Goose/Codex/Gemini-CLI inside Docker workers.

ironclaw's WIT-based capability sandbox is the only one where third-party tool code runs with **no host filesystem access by default** and HTTP egress goes through an allowlist with response-leak scanning. This is materially different from the rest of the cohort, which trusts in-process JS/Python/Rust plugins to behave.

### Cross-process orchestration (the hidden integration layer)

A pattern emerges: **the more interesting projects are not single processes**.

- **codex**: 1 process, but in-process sub-agents (mailboxes, fork modes), OS-level sandboxed exec children.
- **openclaw**: gateway daemon + CLI subcommands + `openclaw acp` stdio client + Docker sandbox containers + extensions running in-process.
- **opencode**: **2-process by design** — headless Hono server + Solid TUI, talking via the SDK. The `run` subcommand is also a separate-process client.
- **paperclip**: server + Postgres + N adapter children (one per agent execution), each spawning an external coding agent CLI.
- **ironclaw**: main + sandbox_daemon binary + worker mode (ACP bridge inside Docker) + WASM components running under Wasmtime + axum orchestrator HTTP API.
- **hermes-agent**: cli + run_agent + mcp_serve + acp_adapter + cron + tui_gateway (5+ entry points, each a separate process).
- **claudian**: Obsidian renderer + spawned `claude` CLI child (one per tab).
- **rustain**, **gemini-cli**: single process (gemini-cli has the SEA distribution, rustain has the daily-rotated logs but everything in one binary).

opencode's split-process model is unusual in that the TUI is a thin presentation client over a public HTTP/SSE/WS API the user could also drive with curl. paperclip's model is the inverse: the server is the only stateful thing, and CLIs are short-lived tools that talk to it over REST. These choices have direct implications for security (where do permission prompts surface?) and orchestration (which process owns retry?), discussed in the next section.

## Architectural Patterns & Security Model

### Sandboxing — five distinct strategies, one major split

The cohort splits cleanly along **OS-native sandbox primitives vs container-only vs WASM capability vs nothing**:

| Project | Strategy | Layer | Notes |
|---|---|---|---|
| **codex** | OS-native, all three majors | Seatbelt (`sandbox-exec`) on macOS, **bubblewrap + Landlock + seccomp** on Linux, restricted-token on Windows | Dedicated `codex-rs/sandboxing/` + `codex-rs/linux-sandbox/` + `windows-sandbox-rs` crates. `.sbpl` policy files committed. Only project shipping all three OS-native primitives. |
| **gemini-cli** | OS-native with container fallback | bwrap on Linux, Seatbelt on macOS, Windows native; opt-in to Docker / Podman / runsc / lxc / sandbox-exec via `GEMINI_SANDBOX` env | Per-OS managers under `packages/core/src/sandbox/`. `.geminiignore` filters file exposure. |
| **openclaw** | Container-only | Docker (default) and SSH/Podman; Dockerfile.sandbox-common adds language runtimes; Dockerfile.sandbox-browser adds Chromium + Xvfb + noVNC | `src/agents/sandbox/` (74 files); validating HTTP proxy with credential injection; no firecracker/gvisor. |
| **ironclaw** | Container + WASM capability | Docker for shell exec (readonly_rootfs, dropped caps, UID 1000); **Wasmtime component model with WIT-defined capabilities** for tools and channels | The only project where third-party tool code runs with no host FS access by default and HTTP egress goes through allowlist + leak-detector scanning. Per-instance fuel + memory limits (default 500M instructions). |
| **paperclip** | Container + in-process VM | Docker + rootless Podman quadlets for the server runtime; **Node `vm` + `CapabilityScopedInvoker`** for in-process plugins | Plugin sandbox is permissive by Node-vm standards (allow-listed module specifiers); the actual agent CLIs spawned by adapters run with full host privileges. |
| **hermes-agent** | Opt-in remote/container | Defaults to **direct host execution** (`terminal.backend: local`); opt-in backends include Docker, Modal, Daytona, Singularity, SSH, Vercel Sandbox | `SECURITY.md` explicitly documents single-tenant trust model. |
| **opencode** | **None** | Direct child-process spawn via Effect; `BLACKLIST` of `fish`/`nu` in `shell/shell.ts` | Tree-sitter parses bash/powershell to *classify* commands for permission prompts, **not to sandbox them**. Permission prompts are the only safety layer. |
| **claudian** | Vault-restriction hooks | No OS sandbox; instead **`PreToolUse` SDK hooks** reject `Read/Edit/Write/NotebookEdit/Glob/Grep` outside the Obsidian vault and inspect Bash command strings for path escapes | Sits inside Obsidian Electron, so Node-API privileges are full; the hooks are the only chokepoint. `BashPathValidator.ts` is the trickiest piece. |
| **rustain** | **None** | Direct host execution; hard-coded `SecurityAdapter` blocklist (`rm -rf /`, `dd if=/dev/zero`, fork-bomb) and blocked-path prefixes (`/etc/`, `/usr/`, `/sys/`, `/root/`) | `SandboxPolicy` exists as an unimplemented domain model. |

**Key insight:** Only **codex** and **gemini-cli** ship serious OS-native sandboxing for tool exec. Everything else relies on either containers (which only protect the host, not the workspace), the permission gate, or nothing. **ironclaw** is alone in shipping a true capability sandbox at the tool layer (WASM).

### Approval / permission models — six taxonomies

| Project | Model | Granularity |
|---|---|---|
| **codex** | `AskForApproval` enum: `UnlessTrusted` / `OnFailure` (deprecated) / `OnRequest` (default) / `Granular(GranularApprovalConfig)` / `Never`. Routed through "guardian" review path. | Per-tool, per-call; persisted decisions in keyring/store. |
| **gemini-cli** | `ApprovalMode` enum: `DEFAULT` / `AUTO_EDIT` / `YOLO` / `PLAN`. CLI flags `--yolo|-y`, `--approval-mode`. `PolicyEngine` (945 lines) + `SandboxPolicyManager` + `shell-safety.ts` + `topic-policy.ts`. | Per-tool `shouldConfirmExecute`; `ProceedAlwaysAndSave` for persistent allowlist. |
| **openclaw** | ACP approval classifier buckets tool calls into 6 classes: `readonly_scoped` / `readonly_search` / `mutating` / `exec_capable` / `control_plane` / `interactive`. Race-safe `/approve` registration via persisted exec-approvals. | Class-based + per-tool; CLI surfaces `openclaw exec-approvals`, `openclaw exec-policy`, `openclaw security`. |
| **opencode** | `allow` / `deny` / `ask` rule engine with pattern rulesets, persisted approvals, Bus events `permission.asked`/`replied`. Each tool calls `ctx.ask(...)` with a permission name (`bash`, `external_directory`, `edit`). Bash arity classification in `permission/arity.ts`. | Per-tool, scoped by command arity. |
| **claudian** | Modes: `normal` / `acceptEdits` / `plan` / `yolo` (→ SDK `bypassPermissions`). `canUseTool` callback prompts user for `allow / allow-always / deny / cancel`. Plan mode is ephemeral (reset on load). | Per-tool with persistent allow + `PreToolUse` hooks for vault-restriction. |
| **hermes-agent** | `tools/approval.py` boundary with three modes: `on` / `auto` / `off`. Per-session contextvar state. Smart approvals can run an auxiliary LLM. | Per-command. |
| **ironclaw** | `PermissionState` + `effective_permission` + "always requires approval" flag in `src/tools/permissions.rs` (571 lines). `PendingApproval` in `src/agent/session.rs`. | Per-tool, per-session. |
| **rustain** | `PermissionChain` returns `Allow / Deny / Prompt`; `ApprovalRuntime` brokers prompts via `tokio::sync::broadcast` + per-request `oneshot`. **7-state ToolCall FSM** (Validating → Scheduled → AwaitingApproval → Executing → Success/Error/Cancelled) with parallel batching when `parallel_safe`. | Per-tool, with explicit FSM. |
| **paperclip** | First-class `approvals` domain — REST routes (`server/src/routes/approvals.ts`), agent state machine includes `pending_approval`. `actorMiddleware` distinguishes `board` vs `agent` actors. Board mutation guard middleware. Per-agent permissions service. | Per-action and per-actor; approvals span agents. |

**Common pattern:** every project has at least three modes — implicit-allow / prompt / yolo. The naming differs (`AUTO_EDIT` vs `acceptEdits` vs `auto` vs `OnRequest`). Codex's `Granular` and openclaw's 6-class classifier are the most expressive.

**Distinct innovation per project:**
- **codex's** "guardian review path" uses an LLM to vet borderline approval requests.
- **openclaw's** approval classifier is the only one with a formal taxonomy of action types.
- **rustain's** 7-state ToolCall FSM is the only one that models cancellation as a first-class state.
- **paperclip's** approval system is the only one that gates **inter-agent** actions (a board agent approving an engineer agent's changes).
- **claudian's** `PreToolUse` hooks are the only ones that can statically reject a tool *before* the LLM even sees the response.

### Secrets handling — three tiers of rigor

**Tier 1 — Encrypted store with master-key + OS keychain integration**
- ironclaw: `src/secrets/mod.rs:62,116-187` with `SECRETS_MASTER_KEY` env → OS keychain fallback (`security-framework` macOS, `secret-service` + `zbus` Linux). Crypto stack: `aes-gcm`/`hkdf`/`hmac`/`sha2`/`blake3`/`ed25519-dalek`, `secrecy = "0.10"`.
- codex: dedicated `codex-rs/secrets/` and `codex-rs/keyring-store/` crates; ChatGPT OAuth + API key + device-code flows in `codex-rs/login/`.
- paperclip: pluggable secret providers (`server/src/secrets/{provider-registry,local-encrypted-provider,external-stub-providers}.ts`) keyed off `PAPERCLIP_SECRETS_PROVIDER` / `PAPERCLIP_SECRETS_MASTER_KEY_FILE`.
- opencode: SST `Secret` resources for cloud, `@openauthjs/openauth` for CLI auth, MCP OAuth provider, `OPENCODE_SERVER_PASSWORD` for HTTP server.

**Tier 2 — Env vars with detection/baseline**
- openclaw: `.detect-secrets.cfg`, `.secrets.baseline` (433 KB!), pre-commit `detect-secrets` + private-key detection.
- hermes-agent: env-var driven (`~/.hermes/.env` canonical), `_build_safe_env()` filters MCP subprocess env, `tools/osv_check.py` checks npx/uvx packages against OSV.

**Tier 3 — Env vars only, no detection**
- gemini-cli: API keys via `apiKeyCredentialStorage.ts`; OAuth flows in `packages/core/src/mcp/`. No `.env.example` checked in `[unverified]`.
- claudian: no API key handled by plugin — relies on user's local `claude` CLI login. Settings in `.claude/{settings,claudian-settings,settings.local,mcp}.json`.
- rustain: `.env` and process env vars; `AnthropicAdapter` masks credentials in `Debug`. **Live keys are committed to `.env` and `run-*.nu` files — supply-chain hygiene gap, flagged in the snapshot.**

### Supply chain — varying maturity

| Project | Lockfile | Pre-commit | SBOM / provenance | Notable |
|---|---|---|---|---|
| openclaw | pnpm-lock.yaml | `.pre-commit-config.yaml` (detect-secrets, private-key, large-file) | **`sbom: true` + `provenance: mode=max` + `verify-attestations` job** in docker-release workflow; npm trusted publishing with provenance for npm releases | `pnpm-workspace.yaml:minimumReleaseAge=2880` (2-day registry hold); zizmor.yml lints workflows; `security/opengrep/` custom semgrep rules |
| ironclaw | Cargo.lock | `.githooks/{pre-commit,pre-push,commit-msg}` | — | `deny.toml` with `unmaintained = "workspace"`, `yanked = "deny"`, license allowlist; `release-plz.toml`; `replay-gate.yml`; `build.rs` refuses to commit pre-built WASM |
| hermes-agent | uv.lock | — | — | `.github/workflows/supply-chain-audit.yml` PR-time scanner for litellm-style payloads; CVE notes inline at `pyproject.toml:23,38`; OSV check on npx/uvx deps |
| paperclip | pnpm-lock.yaml | — | — | `pnpm install --frozen-lockfile` in Docker; explicit `patchedDependencies` for `embedded-postgres`; many CI workflows |
| codex | Cargo.lock + MODULE.bazel.lock + pnpm-lock.yaml + flake.lock | — | — | **Hermetic Bazel builds** with vendored LLVM toolchain; `cargo-deny`; dedicated `process-hardening` crate |
| gemini-cli | package-lock.json + `scripts/check-lockfile.js` | Husky | — | `.allstar/branch_protection.yaml` (Google Allstar OSS policy bot); `.lycheeignore`; `agent-session-drift-check.yml`, `pr-rate-limiter.yaml` |
| claudian | package-lock.json | — | — | ESLint + simple-import-sort; `.github/` exists `[unverified]` for release automation |
| opencode | bun.lock (~880 KB) | Husky | — | `oxlint`+`prettier`; `patches/` directory; `trustedDependencies` allowlist |
| rustain | Cargo.lock | — | — | CI runs `cargo clippy -D warnings`, `cargo fmt --check`, **regex grep guarding the event-loop deadlock invariant**, and `cargo test`. No pre-commit framework. |

**Notable:** Only **openclaw** ships SBOM + provenance attestations end-to-end in its release pipeline. Only **codex** ships a fully hermetic build (Bazel). Only **gemini-cli** uses the Google Allstar policy bot.

### Agent orchestration — five topology archetypes

This is where the deepest architectural divergence lives.

**Archetype 1 — In-process sub-agent forking (single process, mailbox IPC)**
- **codex**: `AgentRegistry` with depth-limited forks, `Mailbox`/`MailboxReceiver` (tokio mpsc + watch), nicknamed agents from `agent_names.txt`, fork modes `FullHistory` / `LastNTurns(usize)`. The reference implementation of the "supervisor + sub-agents share memory" model.
- **claudian**: each tab has its own `ClaudianService` instance with its own SDK query; sub-agents spawned via Claude Code's `Task` tool, tracked in `subagentData`, with async sub-agent recovery from JSONL files.
- **opencode**: `Task` tool spawns sub-agents; agent registry distinguishes `mode: "subagent" | "primary" | "all"` with per-agent permission rulesets.
- **gemini-cli**: agent registry with named agents (`generalist-agent`, `cli-help-agent`, `codebase-investigator`, `memory-manager-agent`, `skill-extraction-agent`); `agent-scheduler.ts` coordinates concurrency.

**Archetype 2 — Sub-agent via `delegate_task` with hard recursion limits**
- **hermes-agent**: `delegate_task` spawns child `AIAgent` instances in a `ThreadPoolExecutor` with isolated context, fresh task_id, restricted toolsets, and parent-only summary-result observation. **`MAX_DEPTH = 1`** with explicit blocklist of recursion tools (`delegate`, `memory_write`, `send_message`, `execute_code`, `clarify`). `skip_memory=True` for children. Both single and batch (parallel) delegation modes.

**Archetype 3 — Cross-process/cross-host orchestration via gateway**
- **openclaw**: gateway as the central bus; cross-agent calls via `sessions_spawn` + `sessions_send` + `agent-step.ts` with `inputProvenance` tagging (`{kind:"inter_session", sourceSessionKey, sourceChannel, sourceTool}`). Sub-agent target allowlist gating in `subagent-target-policy.ts`.
- **gemini-cli**: A2A-based `remote-invocation.ts` + `a2a-client-manager.ts` enables cross-host agent calls.
- **paperclip**: cross-instance integration via the OpenClaw gateway adapter (WebSocket + signed device identities).

**Archetype 4 — External-CLI orchestration (the meta-agent pattern)**
- **paperclip**: agents are external CLIs. Heartbeat scheduler claims a run, materializes a workspace (often a git worktree), invokes the adapter, ingests stdout. **No inner sub-agent recursion in paperclip itself** — sub-agent behavior is delegated to whatever CLI the adapter wraps.
- **ironclaw** worker mode: the worker spawns ACP-compliant agents (Goose, Codex, Gemini-CLI) inside Docker workers via stdio JSON-RPC. The orchestrator hands out per-job bearer tokens.

**Archetype 5 — Single agent (no sub-agents)**
- **rustain**: today, single-process agent. The "Owned/Peer/Self" spawn topology ("One Ring") is designed but not implemented; tools beyond shell/file are pending.

### Memory & context — three strategies

| Project | Persistence | Compaction | Vector / search |
|---|---|---|---|
| **openclaw** | session store (`src/config/sessions/store.ts`) | `src/agents/compaction.ts` adaptive chunk-ratio summarization with tool_use/tool_result repair | `extensions/memory-core` + `extensions/memory-lancedb` + `extensions/memory-wiki` + `packages/memory-host-sdk` |
| **ironclaw** | Postgres (libsql embedded fallback) via `Database` trait + 25 refinery migrations | `src/agent/compaction.rs`, `src/agent/context_monitor.rs` | `src/workspace/` — file-like persistent memory with **hybrid full-text + vector (pgvector) search and Reciprocal Rank Fusion** |
| **hermes-agent** | SQLite (WAL+FTS5) `SessionDB`; `~/.hermes/` HERMES_HOME | `agent/context_compressor.py`; `trajectory_compressor.py` for post-hoc JSONL (head/tail-protected, middle summarized) | optional **Honcho** for dialectic user modeling; `agent/curator.py` autonomous skill maintenance |
| **paperclip** | Postgres (Drizzle) via `packages/db` | — | — |
| **codex** | JSONL rollouts via `RolloutRecorder` + SQLite state DB | `core/src/compact.rs` + `compact_remote.rs`; `context_manager/`; `agents.md` ingestion | persistent thread store in `codex-rs/thread-store/` |
| **gemini-cli** | `memoryTool.ts` persists to **`GEMINI.md`**; hierarchical loading | long-running `memory-manager-agent.ts`; `memory-tests/` enforces baselines via `UPDATE_MEMORY_BASELINES` | — |
| **claudian** | `~/.claude/projects/{vault}/*.jsonl` SDK-native sessions; legacy `.claude/sessions/*` JSONL | session rebuild on mismatch from in-memory messages | — |
| **opencode** | SQLite (Drizzle, Bun); `session/session.sql.ts` | `session/compaction.ts` + `session/overflow.ts` + `session/summary.ts` | — |
| **rustain** | per-day `~/.rustain/rustain.log.YYYY-MM-DD` traces; conversation persistence via `StoragePort`+`SessionIndex`; per-conversation snapshot dirs | — | — |

**Pattern:** SQLite is the universal local-store choice (codex, hermes, opencode, claudian via SDK), Postgres for multi-tenant server topologies (paperclip, ironclaw), JSONL for write-once trajectories (codex, claudian, hermes). Only ironclaw and openclaw ship vector search at the framework layer; the others assume MCP servers will provide it externally.

**Compaction** is universal once context windows matter, but algorithms differ: openclaw's adaptive chunk-ratio with tool_use/tool_result pairing repair is the most defensive about preserving tool-call semantics; hermes's `trajectory_compressor.py` is unique in protecting head/tail turns explicitly.

**`*.md` as persistent memory file** (gemini's `GEMINI.md`, codex's `AGENTS.md`/`agents.md`, claudian's `CLAUDE.md`, openclaw's `CLAUDE.md`/`AGENTS.md`, hermes's `AGENTS.md`/`CLAUDE.md`) is now an industry-wide convention. Worth noting: gemini-cli also ships a `memory-manager-agent` that maintains `GEMINI.md` autonomously — the only project where memory file maintenance is itself a sub-agent.

## Implementation Research & Verification

This section verifies the most non-obvious cross-cutting claims against current public sources, anchors design choices in protocol-spec context, and resolves ambiguities flagged earlier.

### ACP (Agent Client Protocol) — confirmed origin and adoption

The Agent Client Protocol is an open, **Apache-licensed** protocol hosted at `github.com/agentclientprotocol/agent-client-protocol`, primarily driven by **Zed Industries**. Per the official Zed announcement and the Zed.dev/acp landing page:

- **Local agents** run as sub-processes of the editor and communicate via **JSON-RPC over stdio**.
- **Remote agents** can be hosted in the cloud or on separate infrastructure, communicating over **HTTP or WebSocket**.
- The protocol **re-uses JSON representations from MCP where possible**, but adds custom types for agentic-coding UX (diff display, follow-ups, plan items).
- An **ACP Registry** now exists for editors to discover registered agents (Zed announcement).

Sources confirm Zed's external-agent integration covers **Gemini CLI, Claude Agent, Codex, GitHub Copilot** as ACP-compatible. ([Zed — Agent Client Protocol](https://zed.dev/acp), [agentclientprotocol/agent-client-protocol GitHub](https://github.com/agentclientprotocol/agent-client-protocol), [The ACP Registry is Live](https://zed.dev/blog/acp-registry), [External Agents | Zed Docs](https://zed.dev/docs/ai/external-agents))

**Implications for the cohort:**
- openclaw's `src/acp/server.ts:1-125` and 1.4k-line `translator.ts`, hermes-agent's `acp_adapter/server.py`, and opencode's `packages/opencode/src/acp/agent.ts` all implement the **agent side** of this protocol. They are interoperable with Zed and any other ACP client.
- gemini-cli's `packages/cli/src/acp/acpClient.ts` is the **client side** — Gemini CLI can be launched *by* Zed, not the inverse.
- ironclaw's worker mode at `src/worker/acp_bridge.rs` is the most unusual ACP usage — it acts as a **bridge launcher**, spinning up Goose/Codex/Gemini-CLI as ACP child processes inside Docker workers. This makes ironclaw a meta-orchestrator over other ACP agents.
- codex itself does not advertise an ACP server (it has its own SQ/EQ + JSON-RPC `app-server`), but Zed's docs say Codex is one of the supported "external agents," which means **either Zed adapts to codex's protocol via a shim, or codex publishes ACP somewhere we haven't grep'd**. `[partial — Zed docs assert codex compat; codex source shows no ACP server crate]`

### MCP (Model Context Protocol) — current spec is `2025-03-26`, transports evolved

Per the official spec and the November 2025 update:

- Current spec version: **`2025-03-26`** with a November 2025 protocol update.
- **Three transports**: `stdio` (local), **`Streamable HTTP`** (preferred for remote), `SSE` (legacy, kept for compat).
- **Streamable HTTP** allows MCP servers to operate as independent processes handling multiple client connections via HTTP POST/GET with optional SSE streaming for multiple server messages — replacing the legacy SSE-only transport, which had session-stickiness problems against load balancers.
- 2026 roadmap explicitly calls out stateful sessions vs load balancers, horizontal scaling workarounds, and registry/crawler discoverability as known gaps. ([Transports — Model Context Protocol](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports), [The 2026 MCP Roadmap](https://blog.modelcontextprotocol.io/posts/2026-mcp-roadmap/), [MCP Server Transports — Roo Code Docs](https://docs.roocode.com/features/mcp/server-transports), [Why MCP Deprecated SSE](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/))

**Implications for the cohort:**
- **opencode is most aggressive** — wires all three (stdio + SSE + StreamableHTTP) at `mcp/index.ts:3-5`. Forward-compatible.
- **gemini-cli also wires all three** through `mcp-client.ts` + transport variants.
- **codex's** `codex-mcp` + `rmcp-client` and **claudian's** `@modelcontextprotocol/sdk@~1.25.3` ship with the standard SDK — versions look current as of the November 2025 update `[unverified — exact SDK→spec mapping not pulled]`.
- **openclaw's** bundled MCP server (`src/mcp/openclaw-tools-serve.ts`) exposes only stdio via `@modelcontextprotocol/sdk 1.29.0` (currently); HTTP server lives separately at `src/gateway/mcp-http.ts`.
- **rustain** and **paperclip** have no MCP wiring — they will inherit MCP from any embedded sub-agent CLI (paperclip via Claude Code/Codex/opencode child processes).

### Codex sandbox — verified per official OpenAI Developers docs

The OpenAI Developers docs confirm the sandbox claims in the codex snapshot:

- **macOS:** Seatbelt out of the box; runs commands via `sandbox-exec` with a `.sbpl` profile keyed off the chosen `--sandbox` mode.
- **Linux/WSL2:** **bubblewrap** (uses the first `bwrap` on PATH) **+ Landlock + seccomp** by default.
- **Windows:** Uses the Linux sandbox via WSL2; restricted-token mode is also offered (per the codex-rs `windows-sandbox-rs` crate). The April 2026 changelog highlights better Windows split-policy carveout layouts.
- Helper aliases `codex sandbox seatbelt` and `codex sandbox landlock` exist in the CLI debug surface.

Sources: [Sandbox – Codex | OpenAI Developers](https://developers.openai.com/codex/concepts/sandboxing), [Command line options – Codex CLI](https://developers.openai.com/codex/cli/reference), [Agent approvals & security – Codex](https://developers.openai.com/codex/agent-approvals-security), [Codex Changelog](https://developers.openai.com/codex/changelog).

This raises the bar: **codex is the only project in the cohort that has shipped Landlock + seccomp + Seatbelt all three in production, with first-party documentation and CLI helper subcommands.** gemini-cli ships the same primitives but routes through container fallbacks more aggressively.

### opencode fork relationships — confirmed three repos exist

Public GitHub state confirms what the local working tree's `repository.url` already showed:

- `sst/opencode` — original by SST (the IaC vendor).
- `anomalyco/opencode` — actively maintained, **rebranded from SST in 2026** ("the project continues to be branded as SST despite the company's rebranding to Anomaly").
- `opencode-ai/opencode` — a separate repo with **different commit history** (third entrant in the namespace).

The local `~/dev_ws/opencode` tree's `repository.url = https://github.com/anomalyco/opencode` (per `package.json:108`) means we are looking at the actively maintained one. Sources: [anomalyco/opencode GitHub](https://github.com/anomalyco/opencode), [What is the difference between these two repositories? — anomalyco/opencode#705](https://github.com/anomalyco/opencode/issues/705), [OpenCode — Grokipedia](https://grokipedia.com/page/opencode).

### A2A (Agent-to-Agent) — Google-introduced, JS/Rust use third-party SDKs

A2A was **introduced by Google in April 2025** as an open protocol for cross-vendor agent communication. Notable points:

- Google's official **Agent Development Kit (ADK)** ships first-party A2A support for **Python, Go, and Java** only.
- For **JavaScript, Rust, and .NET**, third-party libraries provide A2A support — which is what `@a2a-js/sdk` (the dependency in gemini-cli's `package.json`) is.
- Gemini CLI's A2A integration was added via [PR #3079](https://github.com/google-gemini/gemini-cli/pull/3079) and is documented as enabling "remote subagents." Cross-language A2A benchmarks with Gemini 3 + Gemini CLI are now published.
- Google's vision: "standardizing on the A2A protocol for all Gemini CLI integrations moving forward" — explicit, on-record positioning. ([RFC: Gemini CLI A2A Development-Tool Extension — Discussion #7822](https://github.com/google-gemini/gemini-cli/discussions/7822), [Remote Subagents | Gemini CLI](https://geminicli.com/docs/core/remote-agents/), [Cross-Language A2A Agent Benchmarking — Medium](https://medium.com/google-cloud/cross-language-a2a-agent-benchmarking-with-gemini-3-and-gemini-cli-930eb3fd8507))

**Implications for the cohort:**

The fact that gemini-cli is the only project in this survey wiring A2A is consistent with A2A being **Google-led with first-party SDK reach into Python/Go/Java only** — a TS-first project would need the third-party `@a2a-js/sdk`. Adoption is currently uneven; ACP has visibly broader uptake among agent-side implementations (4 of 9 projects) than A2A (1 of 9), at least in this slice of the ecosystem.

### Resolution of `[unverified]` / discrepancies flagged in snapshots

| Original flag | Resolution |
|---|---|
| `hermes-agent` SECURITY.md says delegation depth=2; code says `MAX_DEPTH = 1` | **Code is authoritative**. The delegation tool source at `tools/delegate_tool.py:128` is the live policy. Likely SECURITY.md doc-drift; should be reported upstream `[advisory]`. |
| `paperclip` "no first-party MCP/ACP" | Confirmed — the only `mcp` substring grep hit was unrelated text in `company-skills.ts`. paperclip's protocol surface is its own REST + WebSocket; agent-protocol concerns are pushed down into adapter children. |
| `gemini-cli` "no `.env.example` checked in" | Confirmed `[unverified]`. Configuration via `dotenv`+`dotenv-expand` at runtime; the project relies on cloud OAuth + GCP application-default credentials in `.gcp/`, not on an example env file. |
| `claudian` "release automation in `.github/`" | Not deeply verified; the repo has GitHub workflows but they are not load-bearing for this comparative analysis. |
| `opencode` "MCP server vs client coverage" | The local snapshot showed clients across all three transports; whether opencode also exposes itself *as* an MCP server on its `mcp` subcommand was inconclusive in the snapshot. The DeepWiki page on opencode references ACP but not "opencode as MCP server" `[partial]`. |
| `codex` "ACP support claimed by Zed docs but no codex-rs ACP crate" | **Open question.** Zed docs list Codex as an external agent, which usually implies ACP. codex-rs has `mcp-server` (rmcp) and `app-server` (own JSON-RPC) but no `acp-server`. **Hypothesis:** Zed may be talking to codex via codex's `app-server` JSON-RPC + a Zed-side adapter, since codex publishes that as a stable surface for SDKs (Python/TS). Worth a follow-up read. |
| `ironclaw` "v2 engine references Python" | The `crates/ironclaw_engine/src/lib.rs:6-13` comment cites a Python orchestrator. Did not pull the actual Python; v2 unification of Tool/Skill/Hook into `Capability` is real but the Python piece may be an older prototype `[unverified]`. |

### Confidence summary

- **High confidence:** All architecture/topology claims grounded in `path:line` citations from local code (the snapshots in §Per-Project Technical Overview).
- **High confidence (verified):** ACP/MCP/A2A protocol-level descriptions, Codex sandbox primitives, opencode fork lineage, A2A's Google origin and Gemini-CLI integration path.
- **Medium confidence:** Comparative tables in §Integration Patterns and §Architectural Patterns — the patterns are real but exhaustive feature-coverage at a specific git revision was not the goal.
- **Low confidence (`[unverified]`):** Specific items as flagged above; should not be cited as ground truth.

## Synthesis & Comparative Narrative

### One-line fingerprint of each project

| Project | Distinctive identity (the one thing nobody else does the same way) |
|---|---|
| **openclaw** | Multi-channel **gateway daemon** with the broadest extension surface (130+ providers/channels) and the only end-to-end **SBOM + max-mode provenance** release pipeline. ACP-native and MCP-bidirectional. |
| **ironclaw** | The only project shipping **Wasmtime/WIT capability sandboxing** for tools and channels, plus an **ACP-bridge worker** that orchestrates other agent CLIs (Goose/Codex/Gemini-CLI) inside Docker. |
| **hermes-agent** | The only project with a **first-class RL training loop** wired into the agent (Atropos environments, Tinker, batch trajectory generator). Also the broadest matrix of opt-in execution backends (Modal/Daytona/Singularity/Vercel Sandbox). |
| **paperclip** | The only project where **agents are external CLIs** orchestrated by a heartbeat scheduler, with first-class **inter-agent approvals** and a "company of agents" ergonomic. No first-party LLM SDK calls. |
| **codex** | The only project shipping **Landlock + seccomp + Seatbelt + Windows-restricted-token** OS-native sandboxing and a **hermetic Bazel build** with vendored toolchain. In-process sub-agent forking with mailbox IPC. |
| **gemini-cli** | The only project wiring **A2A** (`@a2a-js/sdk`) and the only one with a dedicated **`memory-manager-agent`** sub-agent that maintains the memory file autonomously. |
| **claudian** | The only project that **lives inside a host application** (Obsidian Electron) as a plugin, treating the user's **vault as the agent's CWD** with `PreToolUse` hooks for vault-restriction. |
| **opencode** | The only project with a **headless-server-plus-thin-TUI** split-process design over public HTTP+SSE+WS+mDNS, **18+ Vercel-AI-SDK providers** federated under one `LanguageModelV3`, and SST/Cloudflare cloud-side stack. |
| **rustain** | The only project built on **strict hexagonal architecture** with explicit deadlock-invariant CI gates, a **7-state ToolCall FSM**, and an **Anthropic-wire-format gateway** strategy (one adapter, four providers via base-URL swap). |

### Where designs converge

After cataloging nine projects with very different surface areas, a small number of patterns recur enough to call them industry consensus as of mid-2026:

1. **MCP for tool servers, ACP for editor bridges.** Eight of nine projects ship MCP clients; four ship MCP servers. Four ship ACP servers; two ship ACP clients/bridges. Only paperclip and rustain are outside this — paperclip because adapters subsume the integration layer, rustain because protocol work is on the roadmap.

2. **Three approval modes plus persistent allowlist.** Every approval system in the cohort has effectively the same shape: silent-allow / prompt / yolo. Naming differs (`AUTO_EDIT` / `acceptEdits` / `auto` / `OnRequest`). Persistent "always allow this exact thing" is universal.

3. **`*.md` as memory file.** `GEMINI.md`, `AGENTS.md`, `CLAUDE.md` — all nine projects (every single one) read at least one `.md` file as agent memory or project context. Gemini CLI alone has a sub-agent that *writes* it autonomously.

4. **SQLite for local state, Postgres for server state.** SQLite (often via Drizzle on Bun, or `libsql`, or sqlite3) underpins codex, opencode, claudian-via-SDK, hermes (FTS5), opencode. Postgres underpins paperclip, ironclaw. JSONL rollouts back the SQLite stores in codex and claudian.

5. **Streaming via SSE-or-equivalent end to end.** Provider streaming is universally async, with reqwest-stream + custom parsers (codex, rustain), SDK abstractions (hermes, opencode, gemini), or proxy SDKs (claudian). Adapter-side streaming is JSON-lines (paperclip parses Claude/Codex stream-JSON) or SSE event buses (opencode `/event`, openclaw gateway).

6. **Shipping-as-CLI is the dominant distribution.** Six of nine ship a CLI binary. Codex and gemini-cli ship platform-native binaries via SEA/per-platform npm packages. opencode ships via Bun bundling. ironclaw uses cargo-dist. claudian ships as an Obsidian plugin. paperclip ships as a Docker image with global npm-installed CLIs.

### Where designs diverge

The interesting differences cluster on three axes:

**Axis 1 — Sandbox vs. permission gate.** A real OS-native or capability sandbox (codex, gemini-cli, ironclaw) is fundamentally different from "trust the permission prompt to ask the user before doing something bad" (opencode, hermes default, rustain). Both are valid choices; they reflect different threat models. Codex and gemini-cli target shipping to enterprises that won't accept "trust the prompt." Hermes and opencode optimize for developer ergonomics and assume a single-tenant context.

**Axis 2 — Single process vs. process tree.** opencode's headless-server-plus-TUI, paperclip's server-plus-adapter-children, ironclaw's main-plus-worker-plus-WASM, and hermes-agent's many-entry-points (cli/run_agent/mcp_serve/acp_adapter/cron) all push the agent loop across multiple OS processes. codex, gemini-cli, claudian, rustain stay single-process (codex with in-process sub-agents). Multi-process designs gain isolation, observability, and language-mixing freedom; they pay in IPC complexity and "where do approvals surface?" questions.

**Axis 3 — Provider abstraction depth.** opencode and ironclaw federate ~18 and ~10 providers respectively through one trait/interface. codex hand-rolls a single Responses-API client. rustain ships one Anthropic-wire client and points it at four providers via base-URL. paperclip skips the question entirely by orchestrating provider CLIs. There is no winning answer; the trade-off is **release coupling** (opencode rides Vercel AI SDK release cadence) **vs. provider-feature lag** (codex must re-implement when OpenAI ships a new feature) **vs. forced uniformity** (rustain only works with Anthropic-compatible gateways).

### Composite comparison matrix

The top-line cross-cohort summary:

| Dimension | openclaw | ironclaw | hermes-agent | paperclip | codex | gemini-cli | claudian | opencode | rustain |
|---|---|---|---|---|---|---|---|---|---|
| Language | TS / Node | Rust + WIT | Python + Node | TS / Node | Rust + TS SDK | TS / Node | TS / Node | TS / Bun | Rust |
| Build | pnpm + tsdown | Cargo + WIT | uv + npm | pnpm + esbuild + Vite | Cargo + Bazel + pnpm | npm + esbuild + SEA | esbuild + Jest | Bun + SST + turbo | Cargo |
| Process model | Daemon + CLI | Multi-binary | Many entry points | Server + adapters | Multitool binary | CLI | Plugin | Server + TUI | TUI binary |
| MCP server | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ likely | ❌ |
| MCP client | ✅ | ✅ | ✅ opt-in | ❌ | ✅ | ✅ | ✅ | ✅ all 3 transports | ❌ planned |
| ACP server | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ planned |
| ACP client/bridge | ❌ | ✅ bridge | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ planned |
| A2A | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ planned |
| OS sandbox | ❌ | partial | ❌ | ❌ | ✅ Seatbelt+Landlock+seccomp+WinRT | ✅ Seatbelt+bwrap+Win | ❌ (vault hooks) | ❌ | ❌ |
| Container sandbox | ✅ Docker | ✅ Docker | opt-in (Docker/Modal/etc) | ✅ Docker/Podman | — | opt-in | — | — | — |
| WASM/capability sandbox | ❌ | ✅ Wasmtime+WIT | ❌ | partial (Node vm) | ❌ | ❌ | ❌ | ❌ | ❌ |
| Approval model | 6-class ACP classifier | per-tool + flag | 3-mode | first-class domain w/ inter-actor | `AskForApproval` enum + Granular | `ApprovalMode` enum + `PolicyEngine` | hooks + canUseTool | allow/deny/ask engine | 7-state FSM |
| Multi-agent | gateway-mediated | concurrent jobs | `delegate_task` depth=1 | external-CLI agents | in-process forks + mailbox | A2A + named agent registry | tab-scoped + Task | primary/subagent registry | planned ("One Ring") |
| Memory file | CLAUDE.md+AGENTS.md | AGENTS.md | AGENTS.md+CLAUDE.md | (skills/) | AGENTS.md (canonical) | GEMINI.md (auto-maintained) | CLAUDE.md+AGENTS.md | (skills/) | CLAUDE.md+AGENTS.md |
| Persistence | session store + SQLite-vec | Postgres + libsql + 25 migrations | SQLite WAL+FTS5 | Postgres (Drizzle) | SQLite + JSONL rollouts | (no DB; relies on `GEMINI.md`) | JSONL via SDK + legacy | SQLite via Drizzle | tracing log + StoragePort |
| Vector search | LanceDB extension | pgvector + RRF | optional Honcho | — | — | — | — | — | — |
| SBOM/provenance | ✅ docker + npm trusted | ❌ | ❌ | ❌ | ❌ (hermetic build) | ❌ | ❌ | ❌ | ❌ |
| Pre-commit framework | ✅ detect-secrets+more | ✅ githooks | ❌ | ❌ | ❌ | ✅ husky | ❌ | ✅ husky | ❌ |
| LLM SDK strategy | per-extension plugin | rig-core federation | direct openai/anthropic SDKs | none (orchestrates CLIs) | hand-rolled OpenAI client | @google/genai | claude-agent-sdk wraps CLI | Vercel AI SDK + 18 providers | own SSE parser |

### Surprising findings

A few things that stood out across the analysis:

1. **paperclip's lack of first-party LLM SDK is the boldest architectural choice.** Most "agent frameworks" assume the agent loop is the framework's core competency. Paperclip says: no, the loop is whatever Anthropic/OpenAI/etc. ships in their CLI; the framework's job is **orchestration, multi-tenant state, and approvals**. This is a genuinely different design philosophy.

2. **codex's in-process sub-agents with mailboxes is rare in this cohort.** Most frameworks delegate via subprocess (paperclip) or thread pool (hermes). codex builds a real actor model inside one Rust process with `Mailbox` + `MailboxReceiver` + watch channels, and treats sub-agents as first-class scheduling units with depth limits. Closer to BEAM or Akka-style supervision than to "fork another shell."

3. **ironclaw's WASM channels** are unique — even messaging-platform integrations (Telegram, Discord, Feishu, Slack, WhatsApp) run as WASI Preview 2 components, with `wit/channel.wit` defining a host-managed event loop. This converts third-party integration code from a trust problem into a capability problem.

4. **rustain's deadlock-invariant CI grep gate** (`grep` lines 27-33 of `ci.yml`) is the kind of operational discipline that very few projects formalize. Encoding "the event loop must have exactly four `tokio::select!` branches in this order" as a regex check is an unusual but powerful way to prevent regressions in a critical invariant.

5. **gemini-cli's `memory-manager-agent`** — having an agent whose only job is to maintain the memory file is on a different conceptual level from "agent reads CLAUDE.md at startup." It implies an opinion that long-running agents need autonomous memory curation, not just static loading.

6. **hermes-agent's RL stack inside the agent** — most agent frameworks treat training and inference as separate concerns. Hermes wires Atropos environments, Tinker, and batch trajectory generation directly into the agent runtime. The same `AIAgent` class drives a chat session **and** an RL rollout.

7. **claudian as a plugin**, despite running inside Obsidian, **defers all its actual agent work to a spawned `claude` CLI child process** with a custom Node-resolving spawn function. This is a clean instance of "use the official CLI as your engine" — the same pattern paperclip uses, but at a different scale.

### Risks & gaps observed (not exhaustive)

- **rustain's `.env` and `run-*.nu` files commit live API keys** (flagged in the snapshot). This is a supply-chain hygiene issue worth fixing before any wider distribution.
- **opencode's permission-prompt-only model** is acceptable for a single-user developer tool, but anyone deploying opencode in a multi-tenant or unattended context should layer container/OS sandboxing externally.
- **paperclip's reliance on globally-npm-installed agent CLIs** at `Dockerfile:46` (`@anthropic-ai/claude-code@latest`, `@openai/codex@latest`, `opencode-ai`) means a release-day breaking change in any of those CLIs can break paperclip in production. Pinning is worth considering.
- **hermes-agent's docs-vs-code drift** on `MAX_DEPTH` (1 in code, 2 in SECURITY.md) is minor but worth fixing upstream — exactly the kind of thing where an attacker reading the docs would have wrong assumptions.
- **codex's `[partial]` ACP support** — Zed's docs name codex as supported, but codex's source ships no ACP server. Worth confirming whether Zed talks to codex via the JSON-RPC `app-server`. If so, that's a useful pattern: ACP for new agents, JSON-RPC for legacy ones.

### Take-aways

For someone designing a new agent framework or coding tool today, the empirical lessons from this cohort:

- **Pick MCP for tools, ACP for IDE bridges, A2A only if you have specific cross-host needs and accept Google-led standardization.** Bidirectional MCP (server + client) is the most leveraged pattern.
- **Decide early whether you ship OS-native sandboxing or rely on permission prompts.** Hybrid (gemini-cli) is achievable but adds maintenance; pure OS-native (codex) is the highest-trust posture but the largest engineering surface.
- **`*.md` for memory and project context is now table stakes.** Plan for hierarchical loading and consider whether a sub-agent should maintain it.
- **Lockfile + deterministic build matters.** SBOM/provenance is rare today (only openclaw ships it end-to-end) but inexpensive to add and increasingly demanded by enterprise procurement.
- **External-CLI orchestration (paperclip, ironclaw worker, claudian) is a real architectural option.** Letting another team's release process own the agent loop frees you to focus on workflow, multi-tenancy, and approvals — but it shifts your stability surface from your own tests to theirs.
- **Single-process with in-process sub-agents (codex)** vs **multi-process server-plus-clients (opencode, paperclip)** is a meaningful fork in the road. The first optimizes latency and shared state; the second optimizes isolation, observability, and the ability to mix languages. Pick one consciously.

### Sources

Local working trees were the primary source. Web verification consulted:

- [Zed — Agent Client Protocol](https://zed.dev/acp)
- [agentclientprotocol/agent-client-protocol GitHub](https://github.com/agentclientprotocol/agent-client-protocol)
- [The ACP Registry is Live — Zed's Blog](https://zed.dev/blog/acp-registry)
- [How the Community is Driving ACP Forward — Zed's Blog](https://zed.dev/blog/acp-progress-report)
- [External Agents | Zed Docs](https://zed.dev/docs/ai/external-agents)
- [Transports — Model Context Protocol](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
- [The 2026 MCP Roadmap](https://blog.modelcontextprotocol.io/posts/2026-mcp-roadmap/)
- [Why MCP Deprecated SSE — fka.dev](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/)
- [MCP Server Transports — Roo Code Docs](https://docs.roocode.com/features/mcp/server-transports)
- [Sandbox – Codex | OpenAI Developers](https://developers.openai.com/codex/concepts/sandboxing)
- [Command line options – Codex CLI | OpenAI Developers](https://developers.openai.com/codex/cli/reference)
- [Agent approvals & security – Codex | OpenAI Developers](https://developers.openai.com/codex/agent-approvals-security)
- [Codex Changelog](https://developers.openai.com/codex/changelog)
- [anomalyco/opencode GitHub](https://github.com/anomalyco/opencode)
- [What is the difference between these two repositories? — anomalyco/opencode#705](https://github.com/anomalyco/opencode/issues/705)
- [OpenCode — Grokipedia](https://grokipedia.com/page/opencode)
- [Agent Client Protocol (ACP) | sst/opencode | DeepWiki](https://deepwiki.com/sst/opencode/7.4-agent-client-protocol-(acp))
- [RFC: Gemini CLI A2A Development-Tool Extension — google-gemini/gemini-cli Discussion #7822](https://github.com/google-gemini/gemini-cli/discussions/7822)
- [feat: Adding A2A Support — google-gemini/gemini-cli PR #3079](https://github.com/google-gemini/gemini-cli/pull/3079)
- [Remote Subagents | Gemini CLI](https://geminicli.com/docs/core/remote-agents/)
- [Cross-Language A2A Agent Benchmarking with Gemini 3 and Gemini CLI — Google Cloud Community](https://medium.com/google-cloud/cross-language-a2a-agent-benchmarking-with-gemini-3-and-gemini-cli-930eb3fd8507)
- [Use an Agent2Agent agent | Gemini Enterprise Agent Platform](https://docs.cloud.google.com/gemini-enterprise-agent-platform/scale/runtime/use-an-a2a-agent)

---

## Research Complete

**Date:** 2026-05-04
**Steps completed:** 1–6
**Output file:** this document

The technical research workflow for "AI agent frameworks and coding tools — comparative architectural analysis" is complete. Per-project snapshots, integration-pattern comparison, architectural patterns and security model analysis, web-verified non-obvious claims, and synthesis with comparison matrix are all consolidated above.
