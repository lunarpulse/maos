# Edge Case Hunter — 20-1b AC15/F5 + cohort teardown (f9921251 vs c656a6e7)

Method: path/boundary enumeration of the changed hunks, reading HEAD code. Read-only; nothing was executed. Only unhandled paths are findings; handled classes are tabulated at the end so coverage is auditable. `[INFERENCE]` = not observed, derived from code/std/crate semantics.

## Findings

### EH-01 — Accepted sockets never switched back to blocking (macOS/BSD inherit the listener's non-blocking flag)
- Where: `crates/maos-registry/src/server.rs:175` (`listener.set_nonblocking(true)`), `:190-213` (accept → `try_clone` → thread; no `stream.set_nonblocking(false)`); consumers `DeadlineStream::read/write` `:745-755`.
- Evidence: the only `set_nonblocking` in the crate is the listener one (grep). `DeadlineStream` sets read/write *timeouts* only. The TLS path is new: `rustls::StreamOwned::new(ServerConnection, DeadlineStream)` (`:~370`), so the handshake read happens on the accepted socket.
- Consequence: `[INFERENCE]` on macOS/BSD an accepted socket inherits `O_NONBLOCK`; the first `read` returns `WouldBlock` immediately → the handshake/headers read fails → `408 Request Timeout` (plain) or a rustls I/O error (TLS) for every connection. Linux (`accept4`) does not inherit, so every local witness is green; the story's own ledger says Apple is unverified. A no-default-features/macOS operator gets a registry that never answers.
- Fix: in the accept arm, `stream.set_nonblocking(false).map_err(|e| e.to_string())?;` before `try_clone()`/spawn (also before the 503 branch is irrelevant).
- Failing-before witness: unit test in `server.rs` tests that after `accept()` from a non-blocking listener the stream used by `handle_connection` has blocking semantics, e.g. start `serve()` and send the request with a 300 ms delay between connect and first byte; must return 200, not 408. Deterministic on Linux too only if the code under test explicitly asserts `!is_nonblocking` — expose a `fn accept_blocking(&TcpListener)` helper and assert via `libc::fcntl(O_NONBLOCK)` cleared.

### EH-02 — Client cursor ahead of authenticated head is a permanent, silent, unjournalled poll failure
- Where: `crates/maos-registry/src/security.rs:740-753` (`verify_head`), `crates/maos-registry/src/handlers/yanks_since.rs:12-22`, `crates/maos-registry/src/yank.rs:300-313` (`load_cursor`), `:117-141` (`apply_list` anomaly journalling).
- Evidence: handler returns `head = last.sequence` (0 when empty) and `entries = {seq > since}`. For `since > head` entries is empty, `next = since`, `head != Some(next)` → `"yank list does not reach authenticated head"`. The poller's persisted cursor is keyed only by `registry_uri` (`yank.rs:307`), not by the configured signing key or any registry epoch.
- Consequence: (a) registry restore-from-backup, store reset, or a signing-key rotation/new registry at the same URI leaves every client with `since > head`: every poll fails forever; (b) the failure is surfaced only as `eprintln!("yank fetch failed")` (`yank.rs:222`) — the `yank-sequence-anomaly` journal row in `apply_list` (`:130-139`) is now unreachable for signed registries because `verify_head` rejects any gap/duplicate before `apply_list` sees the list; (c) no remediation text (delete cursor / `--resync`). Fail-closed is right; the dead end is not.
- Fix: (1) bind the cursor file to `(registry_uri, signing_pubkey)` (schema_version 3) so a different root restarts at 0; (2) on `verify_head` head-behind-cursor, return a typed `RegistryError::HeadBehindCursor{cursor, head}` that the poller journals via `observer.on_rejected` (rollback evidence) and, only on operator action, resets; (3) message names the cursor path.
- Witness: test in `registry_tls_20_1b.rs`: sign a head=3 list, client cursor=5 → assert error is typed/journalled (currently only `authentication` string); and a rotated-key test: cursor saved under key A, poller started under key B must replay from 0 (today: stuck).

### EH-03 — `PollTask::shutdown()` is an unbounded await in every teardown arm
- Where: `crates/maos-bin/src/yank_policy.rs:428-436`; callers `main.rs:4538, 5898, 6120, 7013, 7104(macro), 9297, 9708`.
- Evidence: `shutdown()` `take()`s `(stop, task)` then `task.await` with no timeout. The poller loop runs `poller.begin_poll()` → `observer.on_poll()` and `apply_list` → `observer.on_yank` **synchronously on the runtime thread** (`yank.rs:200, 218`); `stop.send(true)` cannot preempt them. Only the writer drain has a `timeout(10s)` (`main.rs:9327`).
- Consequence: any observer call that blocks (scheduler/revocation lock held by the unload that now runs concurrently, TL write) turns SIGTERM into an indefinite hang where before the fix the worst case was the 10 s drain-timeout line. The previous failure was bounded; the fix replaced it with an unbounded one. Moved `task` handle is also dropped (detached, not aborted) if the shutdown future itself is cancelled mid-await — `Drop for PollTask` no longer sees it.
- Fix: `match tokio::time::timeout(Duration::from_secs(5), &mut task).await { Err(_) => { task.abort(); eprintln!("yank poller join timed out") } .. }`; keep the handle inside `self` until joined so `Drop` can still abort.
- Witness: unit test in `yank_policy.rs` with an observer whose `on_poll` blocks on a held mutex; `shutdown()` must return within the bound (today: hangs). Cohort-arm variant: same observer through the cohort harness, assert exit before 20 s.

### EH-04 — Join/unload ordering differs per arm; the guard cannot see it
- Where: join *before* supervision/unload: `main.rs:4538` (shell), `:7013` (run --once), `:5898` (topology; before door drain even). Join *after* `unload_all_loaded`: `:6120` (standalone `--once`), **`:9297` (cohort, the new line)**.
- Evidence: cohort arm order is `server.shutdown → drain_started_tasks → hook_pump → cohort_supervision.stop_and_join → unload_all_loaded → (9297) yank join → drop(audit_tx)`. The poller observer (`YankRuntime`: scheduler, revocation_applier, TL) keeps ticking through `stop_and_join` and `unload_all_loaded`.
- Consequence: a yank-policy tick (auto-revoke/quarantine request, or the local re-evaluation timer) can act on the scheduler/applier while spirits are being unloaded: receipts for a revoke that races the planned unload, or a revoke applied to an already-terminated pid. The D-16-3-M comment (`:5918-5933`) says the unload must come after *all joins* "never before"; the new line violates that stated invariant in the two arms that put it after.
- Fix: move `yank_poller_task.shutdown().await;` at `9297` (and `6120`) to immediately after `hook_pump.shutdown().await` and before `*.stop_and_join()`/`unload_all_loaded`. Update `cohort-teardown-audit.md` coordinates.
- Witness: extend the AST guard (see EH-05) to assert, within each block, that the join statement precedes the first `unload_all_loaded(` call; red on current `:9297` and `:6120`. Runtime witness: observer that records a timestamp on tick + unload hook that records start; assert no tick after unload start.

### EH-05 — AST guard blind spots (soundness boundaries enumerated)
Where: `crates/maos-bin/tests/root_shutdown_unload_16_3.rs:64-179` (`every_audit_sender_release_joins_the_yank_owner`).
Unhandled shapes (each silently passes or is only partially caught):
1. `std::mem::drop(audit_tx)` / `core::mem::drop` — `path.path.is_ident("drop")` is false for multi-segment paths; neither the visitor nor the raw `lines()==\"drop(audit_tx);\"` count sees it.
2. `audit_tx` moved into a helper/struct/closure (`release(audit_tx)`, `move ||`) — no `drop` call node, never audited; the channel closes there, unjoined.
3. Releases or `audit_writer` awaits inside non-item macros (`tokio::select!`, `tokio::join!`, any bodies syn does not parse): not visited. The line-count equality catches only the exact text `drop(audit_tx);`; the writer-drain side has **no** raw cross-check at all, only `drains >= 11` (a floor: a 12th drain inside a macro or a removed drain both pass).
4. Join recognised only as the exact statement `yank_poller_task.shutdown().await;`. An aliasing rebind (`let mut y = yank_poller_task; y.shutdown().await`) or `if let Some(..) = ..` form is rejected (false red) or, worse, a join *after* the release in a nested block never counts but one in a loop body `for _ in .. { yank_poller_task.shutdown().await; }` before the release in the same block counts as a join even if the loop runs zero times.
5. No ordering relative to unload/`stop_and_join` (EH-04), and no check that every serving arm *has* a release at all (an arm that just deletes its `drop(audit_tx)` lowers `releases` below 7 only if the count floor is exact: it is `>= 7`, and the cross-check compares against `source.lines()` which also counts `#[cfg(test)]` text and the macro body line).
6. `Visit` has no `visit_item_fn` boundary: `joined` is inherited into nested fn items and closures, so a closure defined after a join but run elsewhere inherits "joined".
- Consequence: the guard is a lexical contract on the spelling `drop(audit_tx);`; the real invariant (the channel's senders are released only after the poller task is joined) can regress through any of 1-3 with a green guard.
- Fix: make the auditor a pure function `fn audit(source: &str) -> Result<Counts, String>`; use exact equalities (`releases == 7`, `drains == 11`) + a raw-text cross-check for `audit_writer` awaits (`lines().filter(contains("audit_writer")&&contains(".await"))` vs AST count); reject `mem::drop(audit_tx)`, any bare `audit_tx` path expression that is not `audit_tx.clone()`/`&audit_tx`/the audited drop; reset `joined=false` in `visit_expr_closure`/`visit_item_fn`; add the ordering rule from EH-04.
- Witness: self-tests that feed the auditor mutated source strings and expect `Err`: (a) sibling-branch join, (b) `std::mem::drop(audit_tx)`, (c) release inside `tokio::select!`, (d) join after release, (e) join after `unload_all_loaded`, (f) extra unjoined drain inside a macro. Today (a) passes (by design) but (b), (c), (f), (e) all pass wrongly.

### EH-06 — Daemon boot now consumes a second, stricter registry config reader; fallback-to-Null removed
- Where: `crates/maos-bin/src/main.rs:3229-3240`; `crates/maos-registry/src/install.rs:312-369`; legacy reader `crates/maos-kernel-core/src/security/operator_config.rs:52-139`.
- Evidence: boot still prints/uses `registry_cfg` (lenient: `parse_tier` falls back to `local`, bad pubkey ignored, `TRUE` ⇒ false) but the client is built from `operator_config()` (strict: invalid tier floor, bad `MAOS_REGISTRY_ORG_SIGNING_PUBKEY`, non-UTF-8, `TRUE`, missing `HOME`, malformed toml ⇒ `Err`). `require_server_tier_signature: required || org_pubkey.is_some()` (install.rs:365) has no counterpart in the printed `registry_cfg.require_server_tier_signature`. The old `Err(e) => warn + NullSpiritRegistryClient` arm is deleted; `?` now aborts boot.
- Consequence: (a) the two readers can disagree about the same keys: the startup banner (`main.rs:3285`) reports a policy the client is not using; (b) a stale/invalid env var or `http://…` URI without `development_http` that previously degraded to Null now makes the daemon refuse to start — also true for `MAOS_REGISTRY_URI` differing from `[registry].uri` (`http()` binding check); (c) CLI `publish` (`publish.rs:264`) and `yank` (`yank.rs:44`) now fail on irrelevant policy keys (tier floor) and on unset `HOME`.
- Fix: derive the printed banner from the same `RegistryClientConfig` (or make `RegistrySection` call `operator_config`); decide explicitly whether boot aborts (then say so in SECURITY.md/changelog and keep the `?`), and split `operator_config()` into `security_only()` for publish/yank.
- Witness: boot test (child `maos` with `MAOS_REGISTRY_TIER_FLOOR=Bogus` + valid URI) asserting the documented behaviour; unit test asserting `RegistrySection` and `operator_config` agree over the same operator.toml/env matrix (today: `TRUE`, `Bogus`, short pubkey diverge).

### EH-07 — `verify_head` parses the request more strictly than the server/transport emit; unsigned non-200s are reported as “authentication”
- Where: `security.rs:695-722` (`Request{method:&str, params:Params}` — `params` required, `method`/`name` zero-copy borrowed); `http.rs:190-199` (verification precedes status handling); `streamable_http.rs:162, 199-205`.
- Evidence: (1) `notifications/initialized` is sent without `params` (`streamable_http.rs:162`); the server's `JsonRpcRequest.params` has `#[serde(default)]` (`server.rs:35`), the verifier does not ⇒ `missing field params` mapped to `authentication`. (2) `#[serde(borrow)] &str` fails on any escaped JSON string in `method`/`name`. (3) Every server rejection (`403/408/411/413/415/431`, and parse-error replies at `server.rs:406-420`) is unsigned; with a configured key the client reports `missing signed envelope/head` instead of the status, and `streamable_http` never sees `HttpStatus{400}` to start its legacy `initialize` fallback.
- Consequence: misleading diagnostics (operators chase key mismatch for a 403 Host mismatch or 413); legacy-session path unreachable against a signing registry.
- Fix: `#[serde(default)] params`, `Cow<str>`/owned `method`; in `http.rs`, if `status` is not 2xx and no envelope is present return the exchange (status-bearing) *without* trusting its body, or return `IoError::Transport(format!("registry HTTP {status}"))`.
- Witness: client-side test hitting the server with a bad Host (403) expecting an error containing `403`; test feeding `{"jsonrpc":"2.0","method":"notifications/initialized"}` to `verify_response` expecting Ok (non-yank).

### EH-08 — Signed claim does not cover HTTP status or the session header returned in `HttpExchange`
- Where: `security.rs:620-627` (`Claim`), `http.rs:200-204` (`status`, `header` returned from unauthenticated lines).
- Consequence: under TLS the transport authenticates them; in explicit dev mode **with** a configured key (allowed, `http.rs:255-269`), a loopback on-path actor can flip a signed 200 to 5xx/4xx (forcing legacy re-initialize flows) or inject `Mcp-Session-Id` while bodies stay valid.
- Fix: add `status: u16` and `session: Option<String>` to `Claim`; server fills them from `respond_http` arguments; client compares.
- Witness: tamper test through a proxy that rewrites the status line / session header; expects authentication error (today: accepted).

### EH-09 — Canonical URIs that are valid for `canonical_uri` but unusable for TLS
- Where: `security.rs:585-599`, `http.rs:347-350` (`ServerName::try_from(host)`), `server.rs:109-114` (`with_tls`).
- Evidence/classes: `https://registry.example.com./mcp` (trailing dot) is accepted by `canonical_uri` (url keeps the dot; `as_str()==uri`), `with_tls` accepts it and the server starts; `[INFERENCE]` rustls `ServerName`/pki-types rejects a trailing-dot DNS name, so every client fails at connect with `registry response authentication: invalid dns name`. Also `https://a_b/mcp`, hosts with underscore: url accepts, DNS-name validation may not.
- Consequence: a registry that starts but can never be contacted; failure surfaces on the client, not at `with_tls`.
- Fix: in `canonical_uri` (https only) additionally require `rustls::pki_types::ServerName::try_from(host).is_ok()` (cfg tls) so server and client reject at configuration time with one message; or reject `host.ends_with('.')`.
- Witness: `with_tls(.., "https://localhost./mcp")` must be `Err`; today `Ok`.

### EH-10 — Key-file parse error text echoes a character of the secret
- Where: `server.rs:300-301` (`format!("signing key hex: {e}")`).
- Evidence: `hex::FromHexError::InvalidHexCharacter{c,index}` displays the offending character and offset of the *seed file*; a seed with a stray non-hex byte leaks that byte to stderr/log.
- Consequence: one plaintext character (plus position) of private key material in the process log on a malformed key file.
- Fix: `.map_err(|_| "signing key file is not 64 hex characters")`.
- Witness: file containing `"zz".repeat(32)`; assert the error string does not contain `z`.

### EH-11 — PEM loading silently skips non-certificate blocks; operator CA extends, never pins, the roots
- Where: `security.rs:781-793` (`rustls_pemfile::certs`), `:795-813` (`roots.extend(webpki_roots…)` then `roots.add`).
- Evidence: `certs()` ignores every PEM section that is not `CERTIFICATE`; a `ca_certificate` pointing at the wrong file (a key, a CSR) yields `empty certificate chain` only if it has *no* cert sections — a file with one stray cert plus the intended CA being malformed is half-loaded; a bad entry aborts everything with `roots.add` error text lacking the path. WebPKI roots stay trusted alongside the operator CA, so `ca_certificate` cannot narrow transport trust (the envelope key is the real pin).
- Consequence: low (envelope still binds), but a mis-typed CA path error names no file; operators believe a private CA pins transport when it does not.
- Fix: include the path in errors; error if the file contains any non-certificate PEM block; document (SECURITY.md) that transport trust = WebPKI ∪ CA and the signing key is the sole registry identity.
- Witness: CA file containing a `PRIVATE KEY` block → `Err`.

### EH-12 — Missing boundary tests (plants cannot fail on these classes today)
`registry_tls_20_1b.rs` covers `localhost`, `192.0.2.1`, `?x=1`, `#x`, userinfo (`:273-291`) but not:
- Loopback classes: accept `127.0.0.2`, `[::1]`; reject `0.0.0.0`, `[::ffff:127.0.0.1]`, `[::]`, `127.1`/`2130706433` (non-canonical forms), server bound to `0.0.0.0`/`[::]` in dev mode, server bound by name `localhost` (literal after resolve), and **one-sided** dev (client dev + server TLS; server dev + client HTTPS pinned) beyond the two tests at `:786-815`.
- Canonicalization: uppercase scheme/host, explicit default port (`:80`/`:443`), trailing-dot host, `//mcp`, `/mcp/`, `/%6dcp`, empty `?`/`#`, whitespace, IDN, `https://[::1]/mcp` positive, port `0`.
- Head/freshness: `since > head` (EH-02), `since = u64::MAX`, duplicate/out-of-order/mid-list gap (only “stripped entry” is tested), `result: null`, `entries` omitted vs `[]` with `head==since`, head claimed on non-yank method, response for a different nonce *and* method.
- TLS: TLS 1.2-only and 1.3-only servers (`with_safe_default_protocol_versions` assumed), cert not-yet-valid (`not_before` in future; only expired is tested), IP-SAN cert for `https://[::1]/mcp`, SAN absent/CN-only, handshake stall (server accepts and says nothing: 30 s client timeout path).
- Custody: key mode `0o400`/`0o700` accepted, `0o640` rejected (tested), key owned by another uid, symlinked key, key in store via symlink (`key_outside_store` canonicalize).
- Config: `[registry]` typos (`signing_key`) silently ignored (no `deny_unknown_fields` at `security.rs:553-560`) → fails closed only because https then lacks a pubkey; `development_http = "true"` (string) type error; pubkey with whitespace/`0x`/uppercase.
- Fix/witness: table-driven tests over the above; each must be red against a plant of the relevant guard (`literal_loopback` → `|_| true`, `url.as_str()!=uri` removed, `entry.sequence != next` removed, `development != development_opt_in` removed).

### EH-13 — Unknown `[registry]` keys are accepted by the security reader
- Where: `security.rs:553-566` (`#[serde(default)]`, no `deny_unknown_fields`).
- Consequence: `signin_pubkey`, `developement_http`, `ca_cert` typos silently drop the setting. Fails closed for https (no pubkey ⇒ error) but a typo'd `ca_certificate` silently falls back to WebPKI-only; the shared `[registry]` table legitimately holds other keys (`tier_floor`, …), so `deny_unknown_fields` needs an explicit allow-list.
- Fix: validate the table's keys against the union of both readers' key sets and reject anything else.
- Witness: operator.toml with `ca_cert = "…"` expecting `Err`.

## Coverage tables (handled classes, silently discarded unless listed above)

URI canonicalization (`canonical_uri`, `security.rs:585-599`): scheme case → rejected (`as_str` differs); host case → rejected; default port `:80/:443` → rejected; non-canonical IPv4 (`127.1`, decimal, hex) → rejected; `[0:0:…:1]` long form → rejected; userinfo/query/fragment (even empty `?`/`#`) → rejected; path ≠ exactly `/mcp` (incl. `/mcp/`, `//mcp`, dot-segments, percent-encoding, `;params`) → rejected; leading/trailing whitespace → rejected; non-http(s) schemes → rejected; empty host → url parse error. Unhandled: trailing-dot host (EH-09).

Loopback (`literal_loopback`, `security.rs:600-606`; server `addr.ip().is_loopback()` `server.rs:167`): `127.0.0.0/8` accepted; `::1` accepted; `localhost` and any DNS name rejected; `::ffff:127.0.0.1` rejected (std `Ipv6Addr::is_loopback` is `::1` only; consistent both ends, conservative); `0.0.0.0`/`::` rejected. Client/server both-end handshake: `development != development_opt_in` ⇒ 403 in both directions; TLS server rejects the dev header; dev client never sends it for https.

Envelope/head (`verify_response`/`verify_head`): wrong uri/nonce/request/response hash, wrong key, duplicate envelope header, missing header, trailing CBOR, unknown fields, `u64` overflow on `checked_add`, gap/duplicate/out-of-order within the returned range, head short of returned tail, entries omitted when `since<head` → all refused. Unhandled: head behind cursor handling (EH-02), status/session not covered (EH-08).

Teardown (cohort arm `main.rs:9249-9340`): result captured before join; both Ok/Err pass 9297 → 9298 → 9327; `release_audit_tx_holders!` invocations (7650, 7806, 8021, 8318, 9571) join first via the macro body; early `return`/`?` before the arms drop `PollTask` (abort); `std::process::exit` paths skip destructors by design. Unhandled: unbounded join (EH-03), ordering vs unload (EH-04), guard shapes (EH-05).

No `unwrap`/`expect` after serde in the non-test diff hunks (checked `security.rs`, `server.rs`, `http.rs`, `yanks_since.rs`; `as_object_mut().ok_or` is a proper error). No-default-features: `RegistryHttp::new`/`handle_connection` cfg arms compile-consistent by inspection (partial move of `security.tls` then `security.development_http` is disjoint-field); only EH-01 is portability-relevant.
