# Epic 23 — Standards Interop (W8)

**Status:** `backlog` — created by correct-course 2026-11-13 (`sprint-change-proposal-2026-11-13.md` §4–§6, operator-approved 2026-11-13). Second epic of the Enterprise-Fit lane (Epics 22–26). The lane starts after `v1.0.0` (decision 1); one minor release per epic. `sprint-status.yaml` is authoritative for status.

**Wave:** W8; 8 stories, ≈ 11.25 median-story units `[INFERENCE]` · **Makes true:** MCP authorization, A2A v1.0, OpenTelemetry, OIDC discovery, network SIEM, external KMS, the enterprise posture in air-gap builds.

**Closes:** FR67 and NFR-Sec-20 (MCP authorization), NFR-Sec-18 extension (OIDC discovery), FR69, NFR-Obs-2 live and NFR-Obs-6 (OpenTelemetry), NFR-Aud-11 network half (SIEM), FR68 and `prd/success-criteria.md:50` (A2A v1.0; the one-session witness, ex 21-8), FR76 (air-gap), decision 3 (external KMS adapter); 21-3 AC2–AC4 and clause (c.2) of `RELEASE-HOLDS.md` § "Story 14-2 — mTLS rotation honesty clauses" (own-leaf rotation has no production caller) → `23-7`.

**Hermetic exit command (CI, no secrets; red until green):**

```
cargo test -p maos-journey-test --test interop_session_23   # 1 — one session with an editor over ACP, an authorized MCP server and an A2A v1.0 peer; absorbs 21-8 (23-5 AC5)
```

**Exit-block provenance (measured at `1a57035e`):**
- Line 1 — `interop_session_23` is created by 23-5 AC5 (applied clarification, 2026-11-13: the proposal names no creating story; `23-5` closes `prd/success-criteria.md:50` and absorbs 21-8). **HEAD: red** — no such test target (`ls crates/maos-journey-test/tests` has no `interop_session_23.rs`); `cargo test` exits non-zero. Green when the one session of 23-5 AC5 passes against fixture servers.
- Shape: one `cargo test` line with a trailing `#` comment; no secret, key, human, wall-clock or network (the authorization server, MCP server and A2A peer are fixtures).

**Kernel-Δ:** forecast 0.

**Lane rules (proposal §4, binding for Epics 22–26).** (1) Adapters stay outside the kernel behind existing ports; `24-3` is the only kernel-touching story. (2) No new `xtask` gate; an exit is a `cargo test` target or a `maos` verb created by a named AC. (3) One hermetic exit per epic. (4) Each external standard is pinned by version in its story, which re-reads the specification. (5) A control is wired or it is labelled. (6) Each epic is preflighted at the previous retrospective.

**Confidence rules.** Rules 1–11 are those printed in `epic-21-multi-host-on-live-substrate-w6.md` under **Confidence rules** and **Confidence rule 11**; they bind here and are not repeated.

**Risks (proposal §4).** `23-5` (a second A2A wire and a non-loopback listener) is the largest unknown. Assurance stays AI-agent review only.

---

## Stories

| Key | Story | Closes · size |
|---|---|---|
| `23-1-mcp-oauth-client` | MCP OAuth client | FR67, NFR-Sec-20 · L |
| `23-2-oidc-discovery-and-key-rotation` | OIDC discovery and key rotation | NFR-Sec-18 extension · M |
| `23-3-otel-wiring-and-genai-projection` | OpenTelemetry wiring and GenAI projection | FR69, NFR-Obs-2 live, NFR-Obs-6 · M–L |
| `23-4-siem-network-sink-and-durable-watermark` | SIEM network sink and durable watermark | NFR-Aud-11 network half · M |
| `23-5-a2a-v1-agent-card-and-jsonrpc-binding` | A2A v1.0 Agent Card and JSON-RPC binding | FR68; `success-criteria.md:50` · L–XL |
| `23-6-enterprise-posture-in-air-gap` | Enterprise posture in air-gap builds | FR76 (air-gap) · M |
| `23-7-peer-identity-rotation` | Peer identity rotation | 21-3 AC2–AC4; 14-2 clause (c.2) · M |
| `23-8-external-kms-adapter` | External KMS adapter | Decision 3 · S–M |

AC1 is the command and what the operator observes. `bmad-create-story` re-measures every cite and may tighten, not add uncited ACs.

### 23-1-mcp-oauth-client — MCP OAuth client

*Closes · Δ:* FR67, NFR-Sec-20 · size L · kernel-Δ forecast 0. Specification: MCP authorization 2026-07-28 (*search synthesis*), re-read at story creation (lane rule 4).

- *HEAD at `1a57035e`:* one static bearer per server (`crates/maos-bin/src/env_contract.rs:117-155`, `MAOS_MCP_*_BEARER`; `crates/maos-mcp/src/transport/streamable_http.rs:44-55`, `bearer: Option<String>` sent on every request); `rg -il oauth crates/maos-mcp/src` = 0.
- **AC1 (command).** Against a fixture authorization server and MCP server, a 401 leads to discovery, a token naming that server as its resource, and a successful call.
- **AC2.** Server A's token is never sent to server B (plant).
- **AC3.** Tokens live in the `SecretStore` and in no Spirit memory or log row.
- **AC4.** The daemon grant is client-credentials with a pre-registered client; a browser flow is a D-row.

### 23-2-oidc-discovery-and-key-rotation — OIDC discovery and key rotation

*Closes · Δ:* NFR-Sec-18 extension · size M · kernel-Δ forecast 0. Writes its part of the ADR-051 amendment.

- *HEAD at `1a57035e`:* static keys from the environment (`crates/maos-bin/src/enterprise_identity.rs:505-530`, `build_sso_verifier` from `MAOS_SSO_JWKS`); one process-wide assertion (`crates/maos-bin/src/worker_spawn.rs:420`, `MAOS_SSO_ASSERTION`).
- **AC1 (command).** Discovery with refresh on an unknown key id, failing closed past the cache TTL.
- **AC2.** The assertion can be supplied per request. Static keys remain for air-gap.

### 23-3-otel-wiring-and-genai-projection — OpenTelemetry wiring and GenAI projection

*Closes · Δ:* FR69, NFR-Obs-2 live, NFR-Obs-6 · size M–L · kernel-Δ forecast 0. GenAI semantic conventions pinned by version (*search synthesis*: the conventions are in Development status).

- *HEAD at `1a57035e`:* `with_trace_sink` (`crates/maos-iac/src/adapter.rs:127`) has no production caller (`rg -n with_trace_sink crates` outside tests: the definition and its doc line `:79` only); no crate depends on `maos-telemetry` (it is a workspace member only, `Cargo.toml:54`).
- **AC1 (command).** With an OTLP endpoint set the daemon installs the sink; unset, it opens no socket.
- **AC2.** Agent, tool and model spans use GenAI names at a pinned version, projected from redacted rows outside the kernel.
- **AC3.** One trace per task whose token counts equal the cost frames.

### 23-4-siem-network-sink-and-durable-watermark — SIEM network sink and durable watermark

*Closes · Δ:* NFR-Aud-11 network half · size M · kernel-Δ forecast 0. Writes its part of the ADR-051 amendment.

- *HEAD at `1a57035e`:* file sink only (`crates/maos-siem/src/lib.rs:16-18`); process-local watermark (`crates/maos-bin/src/main.rs:222-224`; runbook `docs/runbooks/ent-1-enterprise-daemon-posture.md:180`).
- **AC1 (command).** A TLS-only network sink behind a feature.
- **AC2.** A persisted watermark; a restart no longer re-sends.

### 23-5-a2a-v1-agent-card-and-jsonrpc-binding — A2A v1.0 Agent Card and JSON-RPC binding

*Closes · Δ:* FR68; `prd/success-criteria.md:50` (three-protocol coherence; absorbs 21-8) · size L–XL · kernel-Δ forecast 0. Specification: A2A v1.0.1 (*search synthesis*), re-read at story creation. New crate: a kloc row and a workspace-count bump. Writes the new ADR for the A2A HTTP listener.

- *HEAD at `1a57035e`:* length-delimited JSON-RPC over mTLS TCP (`crates/maos-a2a-tcp/src/transport.rs:3-8`); no Agent Card anywhere (`rg -il 'agent.?card' crates` = 0); intake is `A2ARouterCore::handle_intake` (`crates/maos-a2a-core/src/router.rs:1287`).
- **AC1 (command).** The Host serves a signed card for each exposed Spirit, built from its manifest.
- **AC2.** An inbound A2A v1.0 request maps to a typed-intent frame and passes `A2ARouterCore::handle_intake`; consent and log-before-deliver are unchanged; it lands as a `22-4` task.
- **AC3.** Outbound, a peer's card signature is verified before first contact.
- **AC4.** The mTLS wire stays for cohort peers.
- **AC5.** *(applied clarification, 2026-11-13: the proposal makes this the epic's exit and names no creating story.)* `cargo test -p maos-journey-test --test interop_session_23` runs one session with an editor over ACP, an authorized MCP server and an A2A v1.0 peer; it absorbs 21-8 and creates exit line 1.

### 23-6-enterprise-posture-in-air-gap — Enterprise posture in air-gap builds

*Closes · Δ:* FR76 (air-gap) · size M · kernel-Δ forecast 0. Writes its part of the ADR-051 amendment.

- *HEAD at `1a57035e`:* `maos-sso` and `maos-siem` sit under the `network` feature (`crates/maos-bin/Cargo.toml:16-31`; runbook `docs/runbooks/ent-1-enterprise-daemon-posture.md:182`, "Air-gap builds cannot take this posture").
- **AC1 (command).** An air-gap build accepts SSO with static keys, the at-rest seal and the SIEM file sink; discovery, the TLS sink and OTLP stay compiled out.

### 23-7-peer-identity-rotation — Peer identity rotation

*Closes · Δ:* 21-3 AC2–AC4 (moved out of GA by the proposal's §2); `RELEASE-HOLDS.md` Story 14-2 clause (c.2) · size M · kernel-Δ forecast 0.

- *HEAD at `1a57035e`:* `swap_serving_cert` (`crates/maos-a2a-tcp/src/transport.rs:541`) has no production caller (doc `:538`); the 21-3 AC2–AC4 text and its ADR-063 choice stay in `epic-21-multi-host-on-live-substrate-w6.md` §21-3.
- **AC1 (command).** Decided first as a D-row: peer certificates issued and rotated by the enterprise's own PKI, or the existing choreography completed as 21-3 AC2–AC4 specify. Either way the own-leaf swap gets a production caller and the post-grace refusal is typed.

### 23-8-external-kms-adapter — External KMS adapter

*Closes · Δ:* decision 3 (no vendor backend in-tree; one generic adapter) · size S–M · kernel-Δ forecast 0. Writes its part of the ADR-051 amendment.

- *HEAD at `1a57035e`:* the port forwards three calls: `wrap_data_key`, `unwrap_data_key` and `is_healthy` (`crates/maos-domain/src/ports/key_management.rs:21-37`).
- **AC1 (command).** With an endpoint configured, the at-rest seal wraps and unwraps through it; the endpoint down means sealed writes are refused, as ADR-051 requires.
- **AC2.** A reference sidecar for one open-source KMS lives under `examples/`, outside the release binary.

## Dependencies

- Epic 22 is done and its retrospective preflights this epic (lane rule 6).
- `23-5` AC2 lands inbound requests as `22-4` tasks. The exit session of `23-5` AC5 uses the authorized MCP server of `23-1`.
- `23-6` decides which parts of `23-2`, `23-3` and `23-4` stay compiled out of air-gap builds.
- `23-7` starts with its D-row (enterprise PKI or the 21-3 choreography); `21-3-durable-tofu-and-self-leaf-rotation` keeps AC1 for GA.
- ADR-051 amendment: `23-2`, `23-4`, `23-6`, `23-8`. New ADR for the A2A HTTP listener: `23-5`.
