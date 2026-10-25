---
story_id: '18.8'
story_key: 18-8-conformant-mcp-client
created: 2026-10-25
context_commit: c816c544
baseline_commit: c816c544
split_from: 18-1-butler-inference-seam-and-conformant-mcp
security_sensitive: true
kernel_grant: "◆ FLAG-Winston on `crates/maos-kernel-core/src/io/mod.rs` — RATIFIED 2026-10-25 by the operator (Lunarpulse) with D-18-8-01 A (full exchange): **+15 physical / +13 tokei** (pin 26361 → 26376, `maos-kernel-core` 20594 → 20607 as measured by the reverted prototype against c816c544), inside the ADR-069 envelope (≤ 23,000, alarm 22,000). Supersedes the +7 / +4 ratified earlier the same day with D-18-1-06 B (header capture only). Exact figures re-measured at landing; pin `xtask/kernel-core-baseline.toml:505` and ceiling `xtask/kloc.toml:264` move in one commit with a dated HISTORY note. Ordering (operator 2026-10-25): `18-1-butler-inference-seam-and-conformant-mcp` lands first (its +2 / +1 in `inference/`); **18-8 lands second and re-measures** from 18-1's landed pin."
kloc_grant: "Measured at landing after `cargo fmt --all` (RECOVERY-LANE CEILING RULE, `xtask/kloc.toml:60-70,86-95`). maos-domain **+70 ratified with D-18-8-01 A** (measured by prototype: closed `SecretKey::McpBearer(McpServer)` +51, IO port exchange +19; 9743 at zero headroom → exact request at landing); maos-secrets 0 (measured: the backends key off `as_str`, compile unchanged); kernel per kernel_grant; [INFERENCE] maos-mcp +150–250 (999 / 1194 — likely above the 195 headroom, measured request), maos-journey-test +120–250 (626 / 1104), maos-bin +20–40 (27809, zero headroom). `_aggregate_hardfail` = measured + 1. Test files uncharged."
review: "bmad-code-review under `docs/testing/proof-contract.md`. Reviewer model `anthropic/claude-opus-5-5` ONLY — operator ruling 2026-10-25 (fable-5.1 declined for the security layer). Security-critical: bearer storage and transmission (AC2), session-id handling (AC1), the kernel IO port (AC4); the Security layer runs on opus-5-5 and is named in the review record. ◆ FLAG-Winston signs off the kernel diff."
---

# Story 18.8: A conformant (dual-era) MCP client

Status: ready-for-dev

<!-- Created 2026-10-25 at c816c544 by bmad-create-story (lighter creation contract) as the SPLIT of 18-1 ruled by the operator (D-18-1-01 B): 18-1's former AC2 (conformant client) and AC5 (stdio early-close witness) with decisions D-18-1-06/07/12, all three ruled B. D-18-8-01 RULED A the same day (full exchange; kernel +15 / +13, maos-domain +70) → `ready-for-dev`. 18-1 lands first; 18-8 re-measures the kernel pin after it. Prototypes measured and reverted (one cargo process, /mnt/build/cargo). -->

> **The capability:** *Butler's and the Researcher's MCP calls are accepted by a conformant server of either protocol era — modern 2026-07-28 or legacy handshake — carry a bearer the secret store holds, and a stdio server that dies or hangs is a typed error, never a panic or a wedge.*

**Closes:** the conformant-MCP half of S3 / J-Butler that 18-1 carried (epic §18-8 `:206-217`; was §18-1 AC2/AC5) · the CI half of `ops-google-calendar-oauth-mcp` (epic `:30`) · R18-8, R18-P1 (§R, re-anchored to 18-8) · `deferred-work.md` rows owned by this key: none (grep at c816c544; 18-1's only row `:1213` stays 18-1's).

**Does NOT close, and says so:**
- Live Google Calendar with real OAuth acquisition/refresh — operator lane (`ops-google-calendar-oauth-mcp`); CI uses the conformant fixture only. A static bearer in the secret store is this story's credential.
- Era negotiation over **stdio** (the spec's `server/discover` probe) — the epic's AC is Streamable HTTP; the stdio transport (spawn per call, `crates/maos-mcp/src/transport/stdio.rs:30-37`) gets only AC3's robustness. The deprecated HTTP+SSE transport (`sse.rs`) is untouched.
- MCP features beyond `tools/call` (subscriptions, MRTR, tasks, `server/discover` as a client call).
- The inference seam (18-1).

## Story

As **the director whose Butler reads a real calendar**,
I want **the MCP client to speak whichever protocol era the server speaks — per-request versioned headers for a 2026-07-28 server, the `initialize` handshake and a persisted `Mcp-Session-Id` for a legacy one — with a bearer held in the secret store, and stdio servers that die or hang to fail typed**,
so that **an MCP server that follows either revision of the specification accepts MAOS, and no credential lives in a plain environment variable unless the operator chose the env backend.**

**Contract:** epic-18 §18-8 (`_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:206-217`), §R R18-8 (`:57`) and R18-P1 (`:78`); the rulings recorded in `18-1-butler-inference-seam-and-conformant-mcp.md` §Decisions (D-18-1-01/06/07/12); confidence rules 9–11 (`:38-40`); `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `c816c544` (18-1 creation: MCP scout, direct reads, prototypes) and at the split (secret-store and IO-port prototypes), plus the MCP specification (§Latest technical information).

| Premise (epic / §R) | Fact at `c816c544` and consequence |
|---|---|
| AC1: implement `initialize`, `Mcp-Session-Id` persistence, `Authorization: Bearer`; kernel-Δ 0 because "the `ureq` adapter already passes headers" (`io/mod.rs:103-105`) | **Request** headers only. `IoSubsystemPort::http_post` returns the body (`crates/maos-domain/src/ports/io_subsystem.rs:28-33`) and the adapter drops the response headers (`crates/maos-kernel-core/src/io/mod.rs:106-109`); a non-2xx becomes `IoError::Transport("ureq POST …: status code N")` with the body discarded (`:106-108`). A server-issued `Mcp-Session-Id` and the 400 body the dual-era probe inspects are both unreadable without kernel code → §Kernel pricing, D-18-8-01. The kernel MCP adapter maps transport errors by `to_string()` (`crates/maos-kernel-core/src/mcp/mod.rs:96`), so new `McpTransportError` variants need no kernel edit. |
| "Conformant" = `initialize` + `Mcp-Session-Id` (epic as filed) | **MCP 2026-07-28** removed the handshake and the session header; every request carries `MCP-Protocol-Version` (header + `params._meta`) and Streamable HTTP POSTs carry `Mcp-Method`/`Mcp-Name`. Legacy servers (2025-11-25 and earlier) still require `initialize` and MAY issue `Mcp-Session-Id`. Ruled D-18-1-06 **B — dual-era**. |
| MockMcp `crates/maos-journey-test/src/lib.rs:202-323`, unconditional 200 `:311-318` | `MockMcp` spans `:202-355` (struct `:202`, `Default` `:212-226`, impl `:228-346`, `Drop` `:348-355`); the unconditional `HTTP/1.1 200 OK` is `:316-322`. `McpRequestCapture { method, path, body }` (`:192-196`) keeps **no headers**; the server cannot emit a non-200. Users: `journey_butler.rs:62-63,248-251`, `journey_j3.rs:25-28,96-99`, `journey_researcher.rs:59-62`, `crates/maos-bin/tests/butler_8_14b.rs:343-347,433` (own server `:149`), `researcher_8_14c.rs:257-260`. |
| Every transport builds the same `tools/call` body | `build_streamable_http_request_body` (`streamable_http.rs:54-70`), `build_stdio_request_body` (`stdio.rs:86-101`) and SSE hard-code `{"jsonrpc":"2.0","id":1,"method":"tools/call",…}`; `McpRequest` is `{server, tool, args}` (`crates/maos-domain/src/ports/mcp.rs:37-49`); `McpClientImpl::call` (`crates/maos-mcp/src/client.rs:79-121`) never initializes. |
| Bearer source (D-18-1-07 B: the secret store) | `SecretKey` is a **closed** `Copy` enum (`crates/maos-domain/src/ports/secret_store.rs:10-31`: `AnthropicApiKey`, `OpenAiApiKey`; `ALL`, `as_str`, `environment_variable`) — closed "so purge never enumerates or deletes operator-owned keyring entries" (`:7-8`). The trait is `get/put/delete/is_healthy` (`:49-68`). Backends are generic over the key: env (`crates/maos-secrets/src/lib.rs:45-81`), keyring username `namespace:as_str` (`:216-217`), encrypted file `<as_str>.sealed` (`:331-332`). Consumers of the set: env capture over `SecretKey::ALL` (`crates/maos-bin/src/main.rs:3384-3392`), keyring migration (`:3459,3498,3510`), `maos purge`'s fixed list (`crates/maos-bin/src/purge.rs:304-315`), providers (`crates/maos-providers/src/{anthropic.rs:37,openai.rs:38}`). The eight MCP servers with a URI row: calendar, slack, linear, figma, web, arxiv, github, citation-graph (`crates/maos-bin/src/env_contract.rs:84-123`); provider-key env rows `:75-80`. Prototype (reverted): `SecretKey::McpBearer(McpServer)` + 8-variant `McpServer`, `ALL` 2 → 10 — maos-domain +51 tokei; `cargo check -p maos-domain -p maos-secrets -p maos-bin -p maos-kernel-core -p maos-providers` clean with no backend change. |
| Streamable-HTTP construction sites | `main.rs:5187` (Butler `--live`, URIs `:5207-5212`), `:5382` (Researcher `--live`), `:3010` (spirit registry client — not one of the eight servers; no bearer). |
| AC3 (was 18-1 AC5): write-side EPIPE is fatal (R18-P1 already corrected it) | Holds as corrected: `write_all` → `McpTransportError::Transport("write: …")` (`stdio.rs:48-50`), returning before the read (`:54-64`); Rust std ignores SIGPIPE in test binaries. No test asserts the early-close outcome — 15-1's fixtures drain stdin to avoid it (`crates/maos-mcp/tests/stdio_transport_test.rs:40-46`, 7/20 EPIPE without). A server that reads then exits gives `Transport("child produced no output")` (`stdio.rs:63`). A server that **hangs** blocks the read forever: `McpTransportError::Timeout` (`transport/mod.rs:39`) is never constructed. Ruled D-18-1-12 **B**: bounded read here. |

## Kernel pricing (E16-A7) — AC1/AC4

Method as 18-1 (§Kernel pricing there): in place at `c816c544`, `cargo fmt`, physical count (`find crates/maos-kernel-core/src -name '*.rs' | xargs cat | wc -l`) and `kloc-check --json` (tokei 14.0.0), `cargo check -p maos-kernel-core -p maos-domain -p maos-providers -p maos-mcp` (and `-p maos-bin -p maos-secrets` for the full-exchange variant), restored by copy-back; tree clean after (pin 26361 / 95 files).

| Variant (IO port + `IoSubsystemAdapter`; provider mocks untouched via a default trait method) | Physical (26361) | Tokei kernel (20594) | maos-domain IO | Notes |
|---|---|---|---|---|
| (i) `http_post_with_response_header` → `(body, Option<header>)`; `http_post`/`http_get` share `post`/`read_capped` | 26368 (+7) | 20598 (+4) | +9 | ratified first with D-18-1-06 B, **superseded** by D-18-8-01 A; non-2xx stays `Transport(String)` — the fallback could only parse "status code N" out of text |
| (ii) (i) + typed `IoError::Status(u16)` for a non-2xx on the new method | 26370 (+9) | 20601 (+7) | +11 | status typed, 400 body not readable — not taken |
| **(iii) RULED (D-18-8-01 A)** — `http_post_exchange` → `HttpExchange { status, body, header }`, non-2xx returned as data | **26376 (+15)** | **20607 (+13)** | +19 | the spec's "inspect the body of a 400"; grant ratified 2026-10-25 |

maos-domain totals with the secret-store keys: (ii) 9743 → **9805 (+62)**, (iii) → **9813 (+70)**, both measured.

## Decisions

**Ruling record (2026-10-25, operator Lunarpulse):** D-18-1-06 **B** dual-era · D-18-1-07 **B** secret store · D-18-1-12 **B** bounded read (via the 18-1 rulings) · **D-18-8-01 A** full exchange — kernel grant +15 physical / +13 tokei, maos-domain +70, ◆ FLAG-Winston, ADR-069 envelope; 18-8 lands after 18-1 and re-measures. Review `anthropic/claude-opus-5-5` only. No open row.

| ID | Question | Options | Recommendation / ruling and evidence | Status |
|---|---|---|---|---|
| **D-18-8-01** | Which IO-port change, and which kernel grant? The ratified +7 / +4 (variant i) cannot type the dual-era fallback trigger. | **A** — variant (iii) full exchange: **+15 physical / +13 tokei**, maos-domain +70 (with keys); the client reads the 400 body, a recognised modern JSON-RPC error (`HeaderMismatch` −32020, `UnsupportedProtocolVersion` −32022) stays modern, anything else falls back to legacy — exactly the spec's rule. **B** — variant (ii) typed status: **+9 / +7**, domain +62; with one supported modern version, any non-401 4xx on the modern attempt ⇒ legacy; a modern server's `HeaderMismatch` (a client bug) would trigger a wrong fallback that then fails typed. **C** — keep the ratified (i): parse the status out of `IoError::Transport` text — untyped, brittle. | **A (operator, as recommended).** It implements the specification's detection rule ("HTTP: attempt a modern request and inspect the body of a `400 Bad Request` before falling back", versioning page) rather than an approximation, and it is the only variant that can surface a modern server's error body; `io/mod.rs:106-109` drops both today. Grant +15 / +13 kernel, +70 maos-domain ratified. | RULED 2026-10-25 — A |
| D-18-1-06 | MCP revision | A modern only · **B dual-era** · C legacy only | **B (operator).** Interop with servers of either era (compatibility matrix: Dual-era × Modern and × Legacy both work). | RULED 2026-10-25 — B |
| D-18-1-07 | Bearer source | A env var · **B secret store** · C OAuth now | **B (operator; deviates from the creation recommendation A).** Credentials at rest follow the 16-4 provider-key path (keyring / encrypted file / env backend, fallback journaled `main.rs:3405-3425`). Concrete shape: AC2. | RULED 2026-10-25 — B |
| D-18-1-12 | stdio hang | A deferred row · **B bounded read + typed `Timeout` here** | **B (operator).** Rule 10(a): same file and harness; `Timeout` exists unused. | RULED 2026-10-25 — B |
| **D-18-8-02** | kloc / kernel governance | Exact measured requests at landing; kernel pin + ceiling in one commit under FLAG-Winston with a HISTORY entry; maos-domain and (likely) maos-mcp measured requests returned before the landing commit. | Standing contract 2026-10-23; E16-A7. | STANDING (not an option) |
| CR-1 | Era cache | Per origin (scheme+host+port), per process, in the transport (`Mutex<HashMap<Origin, Era>>` or a per-transport `OnceLock`): the first call probes modern; a legacy verdict is reused; a later modern-shaped error on a cached-legacy origin re-probes once (versioning page: "re-probing if the cached assumption later fails"). | — |
| CR-2 | Legacy session | `initialize` (requesting `protocolVersion: "2025-11-25"`) → `notifications/initialized` → the `Mcp-Session-Id` from the InitializeResult response is stored (`Mutex<Option<String>>`) and sent on every later request with `MCP-Protocol-Version: <negotiated>`; a 404 on a request carrying the session id re-initializes once and retries once, then fails typed. | — |
| CR-3 | Typed errors | `McpTransportError` gains `HttpStatus { status: u16, body: Option<String> }` (body only under D-18-8-01 A, truncated) and `Unauthorized` for 401; the kernel adapter keeps mapping by `to_string()` (`crates/maos-kernel-core/src/mcp/mod.rs:96`). No variant carries the bearer. | — |
| CR-4 | Fixture | `crates/maos-journey-test/src/lib.rs`: `McpRequestCapture` gains `headers`; a `ConformantMcp` server (sibling of `MockMcp`, which keeps its permissive default for the existing users) with modes `Modern`, `Legacy`, `DualEra`, emitting 400/401/404 with JSON-RPC error bodies; tests in NEW `crates/maos-journey-test/tests/mcp_conformance_18_8.rs` drive the real `StreamableHttpTransport` over the real `IoSubsystemAdapter`. | — |
| CR-5 | Bearer plumbing | `StreamableHttpTransport::new(io, uri, bearer: Option<String>)`; at `main.rs:5187`/`:5382` the bearer is `secret_store.get(SecretKey::McpBearer(server))` for the server named by the URI variable (`MAOS_MCP_CALENDAR_URI` ↔ `McpServer::Calendar`, …); `:3010` (registry) passes `None`. Missing bearer = no `Authorization` header (the server decides). | — |
| CR-6 | Deterministic EPIPE | A request larger than the pipe buffer (≥ 1 MiB of `args`) to `sh -c 'exit 0'`: `write_all` blocks after 64 KiB until the child exits, then EPIPE — no race. | — |
| CR-7 | Bounded stdio read | `StdioTransport` gains a per-call bound (default 30 s, the Butler `time_cap_seconds`, `spirits/butler/manifest.toml:65`; constructor parameter for tests); the read runs on a helper thread with `recv_timeout`; on expiry the child is killed and reaped and `Timeout(ms)` returned. | — |

## Acceptance Criteria

*(Epic §18-8 as ruled; AC4 is D-18-8-01 A. One plant per control AC, `docs/testing/proof-contract.md:26`.)*

### AC1 — command: a dual-era Streamable-HTTP client (was 18-1 AC2; D-18-1-06 B)

1. `StreamableHttpTransport` (`crates/maos-mcp/src/transport/streamable_http.rs:13-51`) sends every first request modern: `MCP-Protocol-Version: 2026-07-28`, `Mcp-Method: tools/call`, `Mcp-Name: <tool>`, `Content-Type`/`Accept` (`:32-35`), `Authorization: Bearer` when a bearer is configured (AC2), and `params._meta` carrying `io.modelcontextprotocol/protocolVersion` and `…/clientInfo`.
2. On the D-18-8-01 trigger the origin is cached legacy (CR-1) and the client runs `initialize` + `notifications/initialized`, persists the `Mcp-Session-Id` across calls (CR-2) and sends it with the negotiated `MCP-Protocol-Version`.
3. `crates/maos-journey-test/tests/mcp_conformance_18_8.rs` (CR-4): against a **modern** fixture two calls succeed without a handshake, and the fixture rejects a missing/mismatched `MCP-Protocol-Version` (400 `HeaderMismatch`) and a missing `Mcp-Method`/`Mcp-Name` (400); against a **legacy** fixture two calls succeed with exactly one `initialize` and the same session id, and the fixture rejects `tools/call` before `initialize` and an unknown session id; against **both**, a missing bearer is 401 → typed `Unauthorized`. Each negative is asserted against a hand-built request and surfaces from the real client typed (CR-3).
4. **Proven red (1 plant):** stop storing the session id → the legacy two-call test reds.

### AC2 — the bearer comes from the secret store (D-18-1-07 B)

1. `crates/maos-domain/src/ports/secret_store.rs`: `SecretKey::McpBearer(McpServer)` over a closed `McpServer { Calendar, Slack, Linear, Figma, Web, Arxiv, Github, CitationGraph }` (the eight `MAOS_MCP_<SERVER>_URI` rows, `env_contract.rs:84-123`); `as_str` = `mcp-<server>-bearer`, `environment_variable` = `MAOS_MCP_<SERVER>_BEARER`; `SecretKey::ALL` 2 → 10. No backend change (keyring `namespace:as_str`, encrypted file `<as_str>.sealed`, env capture over `ALL`).
2. The eight `MAOS_MCP_<SERVER>_BEARER` names are registered `UserFacing` in `crates/maos-bin/src/env_contract.rs` beside the provider-key rows (`:75-80`); `check-env-contract` passes. `maos purge`'s known keys (`crates/maos-bin/src/purge.rs:304-315`) include the eight bearers (they are MAOS-owned entries; the set stays closed).
3. The bearer reaches the transport per CR-5 and never appears in a log line, error message, TL row, `Debug` output or test snapshot (asserted by grepping captured stderr/stdout for the canary token).
4. Tests: `crates/maos-secrets` round trip for an `McpBearer` key on the env and encrypted-file backends (the at-rest seal test's pattern, `crates/maos-secrets/tests/at_rest_seal.rs:112-170`); `mcp_conformance_18_8.rs`'s positive cases take the bearer from an `EnvSecretStore` built over `SecretKey::ALL`.
5. **Proven red (1 plant):** drop the `Authorization` header in the transport → the AC1 positive cases red with 401.

### AC3 — stdio early close and hang are typed (was 18-1 AC5; R18-P1; D-18-1-12 B)

1. A one-shot NDJSON server that exits before the request is written yields `McpTransportError::Transport("write: …")` from `spawn_and_invoke` (`crates/maos-mcp/src/transport/stdio.rs:30-51`, mapping `:48-50`), deterministically (CR-6); a server that reads then exits silently yields `Transport("child produced no output")` (`:63`); never a panic.
2. A server that never answers yields `McpTransportError::Timeout(ms)` within the bound (CR-7) and the child is reaped (no zombie: its pid is gone from `/proc` after the call).
3. Tests in `crates/maos-mcp/tests/stdio_transport_test.rs`.
4. **Proven red (1 plant):** `.unwrap()` the write (`stdio.rs:48-50`) → the early-close witness panics.

### AC4 — the IO port carries what the dual-era client needs (◆ FLAG-Winston; D-18-8-01 A, grant ratified)

1. Per the ruled variant (§Kernel pricing): `IoSubsystemPort` gains one default method (`crates/maos-domain/src/ports/io_subsystem.rs`), overridden by `IoSubsystemAdapter` (`crates/maos-kernel-core/src/io/mod.rs:71-122`), with `http_post`/`http_get` sharing the request/read helpers so the 10 MiB cap (`:32`) and the `io_call_journal` record (`:99-100`) apply once; the seven provider `MockTransport`s are untouched.
2. Witness: NEW `crates/maos-kernel-core/tests/io_exchange_18_8.rs` against a local TCP server: a 2xx returns `HttpExchange { status, body, header }` with the named header; a 4xx (400, 401, 404) returns `Ok(HttpExchange)` carrying its status and body — never `Err`; a transport failure (refused connection) is still `Err(IoError::Transport)`; a body over 10 MiB is still `Decode`.
3. Kernel figure re-measured at landing against the ruled grant; pin + ceiling re-pinned in one commit.
4. **Proven red (1 plant):** restore the old non-2xx `Transport` mapping on the new method → the 4xx case reds.

## Tasks / Subtasks

- [ ] **T0 — Preflight** — §Decisions reads RULED (all rows, 2026-10-25); story + tracker `ready-for-dev`; Winston's FLAG sign-off recorded before landing. 18-1 lands first: re-measure the kernel pin from 18-1's landed value before claiming the +15 / +13. `git diff --stat c816c544 -- crates/maos-mcp crates/maos-journey-test crates/maos-kernel-core/src/io crates/maos-domain/src/ports crates/maos-secrets crates/maos-bin/src/main.rs crates/maos-bin/src/env_contract.rs crates/maos-bin/src/purge.rs`; drift → Debug Log, re-anchor.
- [ ] **T1 — IO port** (AC4) — domain default method + adapter override + helpers; `io_exchange_18_8.rs`.
- [ ] **T2 — Secret store keys** (AC2.1–2.2) — `McpServer`, `McpBearer`, `ALL`; env rows; purge list; `maos-secrets` round-trip tests.
- [ ] **T3 — Client** (AC1.1–1.2, AC2.3; CR-1/2/3/5) — headers, `_meta`, era cache, legacy handshake + session, typed errors, bearer plumbing at `main.rs:5187,5382`.
- [ ] **T4 — Fixture + conformance tests** (AC1.3; CR-4).
- [ ] **T5 — stdio** (AC3; CR-6/7) — early-close witnesses; bounded read.
- [ ] **T6 — Plants and records** — AC1–AC4 plants recorded once in `18-8-evidence/plants.md` (command, output, wall-clock).
- [ ] **T7 — Governance** (D-18-8-02) — `cargo fmt --all`; `kloc-check --json`; `check-kernel-baseline`; `check-env-contract`; `check-test-cost`; `check-dev-record-completeness`; measured requests before landing; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` before the closing commit.

## Dev Notes

### What changes, by file

| Area | Files |
|---|---|
| Kernel ◆ | `crates/maos-kernel-core/src/io/mod.rs:71-122`; new `crates/maos-kernel-core/tests/io_exchange_18_8.rs`; `xtask/kernel-core-baseline.toml:505`, `xtask/kloc.toml:264` |
| Domain | `crates/maos-domain/src/ports/io_subsystem.rs`, `crates/maos-domain/src/ports/secret_store.rs` |
| MCP | `crates/maos-mcp/src/transport/{streamable_http.rs,stdio.rs,mod.rs}`, `crates/maos-mcp/src/client.rs` (only if the era cache lives above the transport), `crates/maos-mcp/tests/stdio_transport_test.rs` |
| Fixture | `crates/maos-journey-test/src/lib.rs` (`:192-196`, new `ConformantMcp` beside `:202-355`), new `crates/maos-journey-test/tests/mcp_conformance_18_8.rs` |
| Composition | `crates/maos-bin/src/main.rs:5187,5207-5212,5382` (bearer), `crates/maos-bin/src/env_contract.rs:75-123`, `crates/maos-bin/src/purge.rs:304-315` |
| Secrets tests | `crates/maos-secrets/tests/` (round trip for an `McpBearer` key) |

### Guardrails

- The bearer never leaves the transport's header map: no `Debug` derive that prints it, no error string, no TL row, no stdout; tests grep for a canary.
- `SecretKey` stays closed — no string-keyed lookup (`secret_store.rs:7-8`).
- Existing `MockMcp` users keep the permissive server; only the new conformance tests use `ConformantMcp`.
- Kernel edits only in `io/mod.rs`; FLAG-Winston sign-off; in-`src` test lines count toward the pin and tokei.
- One plant per control AC, in place, restored by `git checkout -- <file>`; one target dir; local runs `env -u CI -u GITHUB_ACTIONS`; no `cargo` inside a `#[test]` (`proof-contract.md:22-27,68-76,84-87`).

### Tests and commands

```bash
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-mcp -p maos-secrets -p maos-domain
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test   # mcp_conformance_18_8 + every MockMcp user
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test butler_8_14b --test researcher_8_14c --test shell_8_14a
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core && env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-providers   # IoSubsystemPort mocks
cargo run -p xtask -- check-kernel-baseline && cargo run -p xtask -- kloc-check --json && cargo run -p xtask -- check-env-contract
cargo fmt --all -- --check
```

### Review plan (proof contract `:31-57`)

- **Reviewer model:** `anthropic/claude-opus-5-5` only (operator ruling 2026-10-25; fable-5.1 declined, including the security layer).
- **Focused suites:** `-p maos-mcp`, `-p maos-journey-test`, `-p maos-secrets`, `-p maos-domain`, `-p maos-providers` (port mocks), `-p maos-bin` (MCP users, purge), `-p maos-kernel-core` plus `cargo test -p maos-kernel-core --lib`.
- **Gates:** `check-kernel-baseline`, `kloc-check`, `check-env-contract`, `check-test-cost`.
- **Security-critical:** AC2 (credential storage and transmission), AC1 (session id handling), AC4 (kernel IO port) — Security layer on opus-5-5, named in the review record; ◆ FLAG-Winston sign-off.

### Latest technical information

- **MCP 2026-07-28** ([changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)): sessions and `Mcp-Session-Id` removed (SEP-2567); `initialize`/`notifications/initialized` removed, version and client info per request in `_meta` (SEP-2575); `Mcp-Method`/`Mcp-Name` required on Streamable HTTP POSTs (SEP-2243); error codes `HeaderMismatch` −32020, `UnsupportedProtocolVersion` −32022.
- **Versioning / dual-era** ([page](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)): HTTP clients attempt a modern request and inspect the body of a `400` before falling back; a recognised modern JSON-RPC error means "stay modern"; the era is cached per origin and re-probed when the assumption fails.
- **Legacy Streamable HTTP** ([2025-11-25 transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)): the server MAY return `Mcp-Session-Id` on the InitializeResult; the client MUST echo it; a 404 for a session means "start a new session".
- **ureq 2**: a non-2xx is `Err(ureq::Error::Status(code, response))` — the response (headers, body) is still readable there, which is what variants (ii)/(iii) use.

### Previous-story intelligence

- **18-1 (ready-for-dev, same day):** the creation that measured these premises and priced variant (i); its §Kernel pricing method is this story's. **15-1:** the stdio fixture race (drain stdin first). **16-4:** the secret store, keyring migration and the closed-key rule. **8.14b/8.14c:** the `--live` MCP wiring this story feeds.

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-8 `:206-217`, stories table `:97`, Kernel-Δ `:32`, kloc asks `:36`, §R R18-8 `:57`, R18-P1 `:78`, Dependencies `:221`; tracker `sprint-status.yaml:284`; `prd/project-scoping-phased-development.md:406` (amendment (b), the split record).

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract) at `c816c544` as the operator-ruled split of 18-1 (D-18-1-01 B). Inputs: the 18-1 creation's MCP scout and IO prototype (variant i), plus two prototypes at the split as the host's only cargo process — the closed `SecretKey::McpBearer(McpServer)` (+51 maos-domain) with the full-exchange IO port (iii), and the typed-status IO port (ii) — each measured (`find … | wc -l`, `kloc-check --json`, `cargo check` over kernel/domain/secrets/providers/mcp/bin) and reverted by copy-back (`git status` clean for crates; pin 26361 / 95 files). D-18-8-01 ruled A the same day; status `ready-for-dev`.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25)

### Debug Log References

### Completion Notes List

### File List

### Change Log

- 2026-10-25 — Story created by the split of 18-1 (operator ruling D-18-1-01 B): old 18-1 AC2 → AC1, AC5 → AC3; D-18-1-06/07/12 ruled B; AC2 (secret-store bearer) and AC4 (IO port) written concretely from measured prototypes; the ratified kernel grant (+7 / +4) does not cover the dual-era 4xx probe (+15 / +13 or +9 / +7) → D-18-8-01 OPEN; status `backlog`.
- 2026-10-25 — RULED (operator, Lunarpulse): D-18-8-01 **A** full exchange — kernel grant +15 physical / +13 tokei (◆ FLAG-Winston, ADR-069 envelope), maos-domain +70; 18-1 lands first, 18-8 re-measures; AC4 witness written for `HttpExchange`. Status → `ready-for-dev`.
