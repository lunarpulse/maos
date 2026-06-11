# ATDD Checklist — Story 8.14c: Researcher MCP Driver Set

## J-Researcher Journey Acceptance

### AC1 — MCP response contract & seed fixture
- [x] `drivers::researcher` arg builders carry `source_key` fields (arxiv_id, url, repo, paper_id) — **GREEN-at-8.14c**
- [x] `drivers::researcher` response extractors parse `claim` + `source_key` from Phase-2 responses — **GREEN-at-8.14c**
- [x] `ResearcherMcpError` covers `CallFailed` / `TokenIssuanceFailed` / `Unauthorized` / `NoResults` / `Decode` — **GREEN-at-8.14c**
- [x] Fixture corpus at `fixtures/researcher-mcp-corpus-v0.5.jsonl` with 14 calls (4 search + 10 fetch) including Chen-vs-Tanaka contradictory pair — **GREEN-at-8.14c**

### AC2 — MCP driver wiring + parallelism
- [x] `ResearcherMcpPort` trait (sync) returns `Vec<FetchedClaim { claim, source_key }>` — **GREEN-at-8.14c**
- [x] `FakeResearcherMcpPort` test double ships in same commit — **GREEN-at-8.14c**
- [x] `RESEARCHER_PARALLELISM = 8` const in `researcher/src/lib.rs` — **GREEN-at-8.14c**
- [x] `LiveResearcherMcpPort` in `maos-bin` implements two-phase fan-out with `JoinSet` + `Semaphore(8)` — **GREEN-at-8.14c**
- [x] `LiveResearcherMcpPort` captures `Handle` at construction; sync method uses `Handle::block_on` (FORK 3) — **GREEN-at-8.14c**

### AC3 — Live fan-out → I11 → halt → output_shape → replay
- [x] `on_idle` fans out via `mcp_port.survey_literature(query)` when `Some` — **GREEN-at-8.14c**
- [x] `on_idle` walks scoped log for `McpInvocation` frames after fan-out — **GREEN-at-8.14c**
- [x] `join_claims_to_frames` joins by exact `source_key` match — **GREEN-at-8.14c**
- [x] Joined frames carry `ClaimPayload` as payload + McpInvocation `frame_id` as `source_log_ref` — **GREEN-at-8.14c**
- [x] `survey()` is UNCHANGED; it cites the joined frames normally — **GREEN-at-8.14c**
- [x] Deterministic path (no `--live`) falls back to `self.pending` — byte-identical v0.5 — **GREEN-at-8.14c**
- [x] `--live` path wires inference seam AND MCP seam in parallel — **GREEN-at-8.14c**

### AC4 — Budget warning + journey tests + determinism floor
- [x] Budget envelope: `time_cap_seconds = 60`, `cpu_cores = 1`, `max_tokens = 8192` — **GREEN-at-8.14c**
- [x] No secret leakage in driver code (no hardcoded API keys, env-var only) — **GREEN-at-8.14c**
- [x] Unit tests: `mcp_fan_out_joins_claims_to_invocation_frames`, `on_idle_surveys_via_mcp_port_when_wired`, `on_idle_falls_back_to_pending_when_mcp_port_is_none` — **GREEN-at-8.14c**
- [x] Determinism: `FakeResearcherMcpPort` + fake `LogRecallPort` → reproducible survey output — **GREEN-at-8.14c**
- [ ] BudgetWarning@80% observability test — **RED-deferred-to-8.15-PTY** (requires subprocess harness)
- [ ] Two-sided barrier-gated parallelism test (N=16, peak=8) — **RED-deferred-to-8.15-PTY**
- [ ] Citation replay positive + negative falsifiability tests — **RED-deferred-to-8.15-PTY**
- [ ] Golden-snapshot determinism floor test (byte-identical + zero-side-effect) — **RED-deferred-to-8.15-PTY**
- [ ] `CapabilityDenied` test for undeclared `(server, tool)` token — **RED-deferred-to-8.15-PTY**
- [ ] `researcher_8_14c.rs` subprocess test — **RED-deferred-to-8.15-PTY**
- [ ] `journey_researcher.rs` journey test — **RED-deferred-to-8.15-PTY**

### AC5 — Discipline gates
- [ ] `cargo xtask check-empty-kernel` — zero NEW kernel KLOC (researcher is Spirit-side only)
- [ ] `cargo xtask check-workspace-count` — workspace stays at declared count
- [ ] `cargo xtask abi-diff` — additive-only (no removals/renames on existing surfaces)
- [ ] `cargo test` green across workspace

## Verification Commands

```bash
# Unit tests
cargo test -p researcher
cargo test -p maos-mcp --lib
cargo test -p maos-bin --test butler_8_14b  # ensure Butler still passes

# Discipline gates
cargo xtask check-empty-kernel
cargo xtask check-workspace-count
cargo xtask abi-diff
cargo test --workspace
```
