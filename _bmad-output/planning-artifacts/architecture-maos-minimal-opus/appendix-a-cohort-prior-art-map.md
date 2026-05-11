# Appendix A — Cohort prior-art map

For each major design decision, here is the project we mined it from. Useful when reviewing.

| MAOS feature | Survey project | Why we picked it |
|---|---|---|
| In-process Spirit mailbox | codex `core/src/agent/{registry,mailbox}.rs` | Cleanest actor model in the cohort; supervised, depth-limited |
| 7-state ToolCall FSM | rustain `src/domain/services/tool_scheduler.rs` | Cancellation as first-class state |
| Sandbox primitives (Landlock+seccomp / Seatbelt / Win) | codex `codex-rs/sandboxing/` + `linux-sandbox/` | Only project shipping all three OS-native |
| ACP server NDJSON stdio | openclaw `src/acp/server.ts`, opencode `acp/agent.ts`, hermes `acp_adapter/server.py` | Convergent; ratified by Zed |
| MCP all-three transports (stdio / SSE / Streamable HTTP) | opencode `mcp/index.ts` | Future-compatible |
| Approval class taxonomy (6 classes) | openclaw `src/acp/approval-classifier.ts` | Most expressive taxonomy; battle-tested |
| Headless server + thin TUI | opencode | Clean split; mDNS optional discovery |
| Sub-Spirit `delegate_task` with depth cap | hermes-agent `tools/delegate_tool.py` | Recursion safety pattern |
| `*.md` memory file convention | universal in cohort | Table stakes |
| Hot-swap with token rebinding | codex's `SpawnAgentForkMode` lifted to whole-Spirit | Preserves in-flight context across role changes |
| Provider hot-swap via `ArcSwap` | rustain `src/infrastructure/startup.rs` | Live provider replacement is non-trivial |
| Hexagonal `domain/adapters/infrastructure` for the kernel layout | rustain `CLAUDE.md` | Survives kernel evolution |
| Approval Decision Log + Transparency Log split | openclaw audit suite | Two distinct audit needs |
| `cargo-deny`-style dep policy + SBOM/provenance | openclaw `docker-release.yml` + ironclaw `deny.toml` | Supply chain hygiene from day one |
| Daily-rotated journal for crash recovery | rustain `rustain.log.YYYY-MM-DD` | Cheap durability |
| Posture preset hierarchy (presets + custom) | gemini-cli `ApprovalMode` enum + openclaw classifier | Composable |
| Distillation-after-execution (`trajectory_compressor.py`) | hermes-agent | Canonical reference for the §9.5 pattern |
| Principal-namespaced memory model | hermes-agent | Lifted into kernel-allocated contract per ADR-026 |
| LSP-style Content-Length framing | LSP / language servers ecosystem | Boring, well-understood, abundant implementations |
| In-kernel notification rendering (TUI + ACP + native + push) | new — no precedent in survey | Required by the substrate's transparency invariants |
