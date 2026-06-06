---
dev_model_used: claude-opus-4-8
---
# Story 8.9: A2A Trust-Binding & Consent Integrity Hardening

Status: done
<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

> **Registered 2026-06-06** (party-mode implementation audit, two rounds). **Phase 1 — Trust restoration.** Charter-safe: lands ONLY in `maos-a2a-core` + `maos-a2a-tcp`; **`maos-kernel-core` stays byte-identical** (zero kernel KLOC). Closes audit gaps **G1, G2, G3, G4, G5, G6, G8, G9, G10** (G7 is NOT in this story's scope). **UNBLOCKS Story 8.8** (its G10 + sender-completeness preconditions overlap this work). Makes the J4 confused-deputy guarantee real on the live wire.
> **Recommended dev model:** `claude-opus-4-8` — small-surface, high-precision security-semantics work where correctness of each enforcement decision-point dominates throughput (consistent with the 8.5–8.7 A2A-security recommendation).

## Story

As a v1.5 operator running Mira and Nash over the live `maos-a2a-tcp` transport,
I want the router's peer identity bound to the TLS-verified certificate (not a self-asserted frame field) and the consent envelope's granter and expiry actually enforced,
so that a mesh peer holding any one validly-pinned leaf cannot impersonate another Host, replay a stolen consent envelope, or bypass consent expiry — closing the confused-deputy class on the wire.

---

## ⚠️ CRITICAL CONTEXT — read before touching code

This is a **security-critical defect-closure story**. The transport (8.6) and the consent vocabulary (8.7) shipped `done`, but a two-round audit found the receiver-side *trust binding* is incomplete: the live mTLS handshake **learns** the peer's verified identity and then **throws it away** (`verifier.rs:177` binds `Some(_peer)` and discards it), so intake re-derives "who sent this" from `frame.from.host_id` — an **attacker-controlled field** on the wire. Any peer holding *one* validly-pinned leaf can therefore forge `from.host_id` and act as a **confused deputy** for another Host. Several adjacent consent checks are also dead or mis-ordered.

**Two hard rules for this story:**
1. **`maos-kernel-core` MUST stay byte-identical.** All work is in `maos-a2a-core` (shared engine) and `maos-a2a-tcp` (live wire). Verify with the kloc/byte gate (see AC7).
2. **Do NOT regress the loopback path.** `maos-a2a/tests/a1_security_regression_guards.rs` (P1/P2/P5/P6/P7) and `maos-a2a-core/tests/cross_host_consent_v1_5.rs` (8.7's fine-grained suite) drive the SAME `A2ARouterCore` you are editing. The loopback in-process router has no wire identity to bind, so it legitimately keeps `frame.from.host_id` as its peer key (including the `None → "loopback"` fallback). **The TLS-binding enforcement is added on the CROSS-HOST path only** — via a NEW verified-intake entry point — never by removing the shared core's existing loopback behavior. (AC4's reorder and AC2's granter check DO land in the shared core; they are additive and must keep both gates GREEN.)

---

## Acceptance Criteria

Each AC closes one or more audit gaps (G-number in brackets). The G-number → file:line map is the source of truth; line numbers below were verified against HEAD (commit `0b6cbc5`).

### AC1 — Bind intake identity to the TLS-verified peer [G8 / G3]
1. The live cross-Host intake path binds the router's peer identity to the **TLS-verified leaf certificate**, NOT to `frame.from.host_id`. On every accepted mTLS connection, `serve_connection` (`crates/maos-a2a-tcp/src/transport.rs:381`) resolves the verified `PeerId` and rejects any decoded frame whose `frame.from.host_id` does not equal that verified peer.
2. A forged frame (`from.host_id` set to a Host other than the TLS-verified peer, sent over a *validly-pinned* connection) is rejected with a typed `A2AError::PeerIdentityMismatch` → NACK `CODE_PEER_IDENTITY_MISMATCH` (`-32007`), and **`intake_entered` is NOT incremented** for that frame.
3. The cross-Host path's `None → HostId("loopback")` fallback (the shared core's `router.rs:438-441`) is unreachable on the wire: a frame with absent `from.host_id` mismatches the verified peer and is rejected. (The fallback is **not deleted from the shared `A2ARouterCore`** — the in-process loopback router still relies on it; it is bypassed on the verified path.)
4. NEW test `g8_confused_deputy_negative` in `crates/maos-a2a-tcp/tests/` proves the oracle: two endpoints validly pinned to each other; the dialer sends a frame whose `from.host_id` is forged → `EPeerIdentityMismatch` classification on the wire response AND receiver `intake_entered() == 0`. An **honest** frame over the same connection still ACKs (positive control).

### AC2 — Enforce consent-envelope granter binding [G1]
1. `A2ARouterCore::handle_intake` rejects any frame whose `consent_envelope.granter` does not match the frame's own `from` address (compare `spirit_id` AND `host_id`). Closes stolen-envelope replay: an envelope granted by Host X, replayed inside a frame `from` Host Y, is denied.
2. The rejection is a typed `A2AError::ConsentGranterMismatch` → NACK `CODE_CONSENT_GRANTER_MISMATCH` (`-32008`), carrying both addresses in the NACK `data` for audit reconstruction.
3. Honestly-built frames are unaffected: `ConsentEnvelope::with_fine_grained_intent(from, intent)` (`maos-domain/src/frame.rs:433`) already sets `granter = from`, so every reference-fleet frame passes. NEW regression test proves a granter≠from frame is denied and a granter==from frame is admitted.

### AC3 — Populate and enforce consent expiry on real frames [G10]
> **Team-consensus decision (2026-06-06, 3-0 — see Decisions §D1):** TTL is **operator-configurable per peer**, NOT a buried constant; and an explicit grant always wins. "Per spec + long-term correctness" — mirrors `partition_timeout_secs`; the authoritative `valid_until_ns` source is the consent grant, the synthesized fallback is transitional. Cross-host **fail-closed-on-absent-expiry is Story 8.8's end-state**, NOT 8.9.
1. Add a per-peer **`consent_ttl_secs: u64`** field to `A2APeerConfig` (`crates/maos-a2a-core/src/config.rs:39`) with `#[serde(default = "default_consent_ttl_secs")]` (default **`300`**) and `validate()` range **`1..=86400`** (mirror `validate_rejects_out_of_range_timeout`, `config.rs:82`). Additive + defaulted → existing TOML still parses under `#[serde(deny_unknown_fields)]`.
2. `A2ARouterCore::prepare_outbound` (`router.rs:339`) populates `consent_envelope.valid_until_ns` **only when an envelope is present AND its `valid_until_ns` is `None`**: stamp `self.consent_now_ns().saturating_add(peer_cfg.consent_ttl_secs * 1_000_000_000)`. An envelope that already carries an explicit `valid_until_ns` (an authoritative grant) is **left untouched** — the transport must never override the granter. (Use a documented module const `DEFAULT_CONSENT_TTL_SECS = 300` for the `serde` default fn, not a magic literal.)
3. The dead-code gap is closed: `with_fine_grained_intent` builds `valid_until_ns: None`, and pre-8.9 `prepare_outbound` never set it — so expiry never fired on any real (non-hand-built) frame. After this AC, a frame that traversed `prepare_outbound` carries a bounded expiry. Add a `// TRANSITIONAL` comment naming the authoritative-grant end-state + the 8.8 fail-closed hand-off.
4. NEW integration test in `crates/maos-a2a-tcp/tests/` proves expiry fires on a **real** frame: build a frame via `prepare_outbound` (NOT a hand-built `ConsentEnvelope`), send it over the wire to a receiver whose **pinned consent clock** is advanced past `valid_until_ns`, and assert `CODE_CONSENT_EXPIRED`. (Both ends pin the consent clock via `with_pinned_consent_clock` for determinism.) Add config round-trip + `validate()` out-of-range tests for `consent_ttl_secs`, and a unit test proving an explicit `valid_until_ns` survives `prepare_outbound` unchanged.

### AC4 — Check expiry before accept-allowlist [G2]
1. In `handle_intake`, the consent block (granter match from AC2 + expiry from the existing `router.rs:530-553`) is evaluated **before** the ADR-012 accept-allowlist check (`router.rs:516-528`). An expired or wrong-granter consent is rejected regardless of whether the intent is allowlisted.
2. New intake validation order is exactly: framing → peer lookup → (1) TOFU verify → (1.5) restart detection → (2) **consent granter-match + expiry** → (3) accept-allowlist → (4) Lamport advance → (5) sink.
3. The `a1_security_regression_guards` P2 test and the 8.7 `cross_host_consent_v1_5` accept-side tests stay GREEN under the new order.

### AC5 — Unify the intent length bound [G9]
1. `consent_match_key` (`router.rs:255-263`) replaces its ad-hoc `len() <= 1024` sanity bound (line 260) with `maos_domain::invariants::i8::MAX_CANONICAL_INTENT_LEN` (`= 128`, `i8.rs:44`). An intent string longer than 128 bytes falls back to the 3-band projection consistently with `A2AIntent::is_canonical` (which already enforces 128).
2. A test pins the unified bound: a 129-byte `intent_class` falls back to band; a 128-byte one is used as the fine-grained key.

### AC6 — Typed-error classification, restart-TOCTOU atomicity, duplicate-peer hard-fail, intake-timeout coverage [G4 / G5 / G6]
1. **[G4] No `Debug`/lowercased-string error classification on security paths.** `verifier.rs:202` (`map_webpki_error`) matches `webpki::Error` **typed variants** (e.g. `CertExpired`, `CertNotValidYet`, `UnknownIssuer`) instead of `format!("{e:?}").contains(...)`. `mtls.rs:63` (`HandshakeRetryPolicy::is_retryable`) classifies on a **typed discriminant**, not `s.to_lowercase().contains("bad_certificate")`. The handshake error taxonomy stays stable so AC-T5's retry oracle and the H2 expired-cert oracle still pass.
2. **[G5a] Duplicate-peer config is a hard error, not a silent overwrite.** `A2ARouterCore::new` (`router.rs:135-145`) currently `eprintln!`s a warning and lets "last wins". Add a validating `try_new(...) -> Result<Self, A2AError>` that returns `A2AError::ConfigInvalid` on a duplicate `peer_id`; `new` delegates and panics (`.expect`) so existing infallible callers keep compiling, and `TcpA2ATransport::bind` (`transport.rs:150`) uses `try_new` to surface the error.
3. **[G5b] Restart-detection is atomic (no TOCTOU).** The boot-nonce path (`router.rs:474-514`) currently reads via `get_pin` then acts via `invalidate_for_restart` — a check-then-act race. Add a single atomic store primitive (compare-and-invalidate holding the `DashMap` entry lock across read+set) and use it so a concurrent intake cannot interleave between the nonce read and the invalidation.
4. **[G6] The intake timeout covers processing + the NACK write, not just the read.** `serve_connection`'s `timeouts.intake` (`transport.rs:403`) currently bounds only `framed.next()`. Wrap `handle_intake(_verified)` AND the response `send_response` in the intake/idle bound so a slow processor or a stalled write cannot hang the per-connection task.

### AC7 — Gate, placement, and discipline
1. `maos-a2a/tests/a1_security_regression_guards.rs` (P1/P2/P5/P6/P7) and `maos-a2a-core/tests/cross_host_consent_v1_5.rs` (8.7 suite) stay GREEN. All NEW negative tests (AC1/AC2/AC3/AC5/AC6) GREEN.
2. A CI **50× stress** run of the new TCP negative tests holds (no flake) — reuse the H1–H6 harness discipline and `TcpTimeouts::test_profile()` (≤250ms) so timeout-path tests complete `<2s`.
3. **`maos-kernel-core` byte-identical** (assert the pinned baseline). `check-workspace-count` = **41** (NO new crate). `kloc-check` GREEN for `maos-a2a-core` and `maos-a2a-tcp`. `abi-diff` GREEN (it scans `maos-spirit-abi`, untouched; new `A2AError` variants/`CODE_*`/`pub fn` are Added-only on `maos-a2a-core`'s surface — `A2AError` is `#[non_exhaustive]`, so additive).
4. `check-dev-model-used-populated` + `check-dev-record-completeness` GREEN (this story's frontmatter carries `dev_model_used`).

---

## Tasks / Subtasks

- [x] **Task 1 — New typed errors + NACK codes (foundation)** (AC: 1, 2)
  - [x] Add `A2AError::PeerIdentityMismatch { expected: String, asserted: String }` and `A2AError::ConsentGranterMismatch { granter: String, frame_from: String }` to `crates/maos-a2a-core/src/error.rs` (`A2AError` is `#[non_exhaustive]` — additive).
  - [x] Add `CODE_PEER_IDENTITY_MISMATCH = -32007` and `CODE_CONSENT_GRANTER_MISMATCH = -32008` to `crates/maos-a2a-core/src/transport/json_rpc.rs` (next free slots after `CODE_FRAME_TOO_LARGE = -32006`).
  - [x] Extend `map_a2a_error_to_iac_bus` (`router.rs:582`) with arms for both new variants (map to the closest `IacBusError` — likely `CrossHostRouteFailure`/a transport-failure variant; check `maos-domain/src/iac_bus_types.rs` for the exact set; do NOT add a kernel variant).
- [x] **Task 2 — AC1: thread the TLS-verified peer into cross-Host intake** (AC: 1)
  - [x] Add `A2ARouterCore::handle_intake_verified(&self, request, verified_peer: &PeerId) -> A2AJsonRpcResponse` (additive; the existing `handle_intake` stays for loopback). It runs framing validation, then **before any other work** checks `frame.from.host_id == Some(HostId(verified_peer.as_str()))`; mismatch → `CODE_PEER_IDENTITY_MISMATCH` NACK. On match it delegates to the shared validation body.
  - [x] In `transport.rs`: thread `pins: Arc<InMemoryTofuPinStore>` through `accept_loop` → `serve_connection` (the transport already holds `self.pins`). After `acceptor.accept(tcp)` succeeds, resolve the verified peer from the negotiated client cert: `tls.get_ref().1.peer_certificates()` → leaf `[0]` → `PeerCertFingerprint::from_cert_der(leaf.as_ref())` → `pins.find_active_pin_by_fingerprint(&fp)` (the SAME oracle `verifier.rs:177` already used — re-derived deterministically). If no verified peer resolves, close the connection without entering intake.
  - [x] Restructure the `Ok(Some(Ok(buf)))` arm (`transport.rs:422-442`): decode → call `core.handle_intake_verified(req, &verified_peer)`. Move `intake_entered.fetch_add` so it increments ONLY when the verified-peer binding passes (forged frame → `intake_entered` stays 0). Keep the `last_intake` observation for the happy path.
  - [x] NEW `tests/g8_confused_deputy_negative` (or fold into `t3_t6_security.rs`): forged-`from` over a valid connection → wire NACK classified `PeerIdentityMismatch` + `nash.intake_entered() == 0`; honest frame ACKs (positive control). Use the `support` harness (`mk_ca`/`valid_leaf`/`bind_endpoint`/`make_frame`) + the raw-`Framed` dial pattern from `t7_t10_liveness.rs` (raw `TlsConnector` + `client_config()` + length-delimited `send`).
- [x] **Task 3 — AC2 + AC4: granter binding + reorder in the shared core** (AC: 2, 4)
  - [x] In `handle_intake` (shared body, also reached by `handle_intake_verified`): add a consent block that (a) if `frame.consent_envelope` is `Some`, requires `envelope.granter == frame.from` (compare `spirit_id` and `host_id`) → else `CODE_CONSENT_GRANTER_MISMATCH` NACK with both addresses in `data`; (b) keeps the existing expiry check (`router.rs:530-553`).
  - [x] **Reorder**: move this consent block to run **before** the accept-allowlist check (`router.rs:516-528`). Final order per AC4.2.
  - [x] NEW core tests in `cross_host_consent_v1_5.rs` (or a sibling): granter≠from → `CODE_CONSENT_GRANTER_MISMATCH`; granter==from → ACK; expired-AND-denied-intent → `CODE_CONSENT_EXPIRED` wins (proves ordering).
- [x] **Task 4 — AC3: operator-configurable consent TTL + stamp `valid_until_ns` in `prepare_outbound`** (AC: 3) — _per Decision §D1_
  - [x] Add `consent_ttl_secs: u64` to `A2APeerConfig` (`config.rs`) with `#[serde(default = "default_consent_ttl_secs")]`; add `default_consent_ttl_secs() -> u64 { DEFAULT_CONSENT_TTL_SECS }` and `pub const DEFAULT_CONSENT_TTL_SECS: u64 = 300`. Extend `A2APeerConfig::validate()` to reject `consent_ttl_secs` outside `1..=86400`. Update the in-crate `A2APeerConfig` literals (router/test fixtures construct it directly — add the field) so the workspace compiles.
  - [x] In `prepare_outbound`, after the existing steps, if `frame.consent_envelope` is `Some` with `valid_until_ns == None`, set `valid_until_ns = Some(self.consent_now_ns().saturating_add(peer_cfg.consent_ttl_secs.saturating_mul(1_000_000_000)))`. Leave an explicit `Some(_)` untouched (authoritative grant wins). Add the `// TRANSITIONAL` comment (authoritative source = consent grant; cross-host fail-closed-on-absent = Story 8.8).
  - [x] NEW integration test in `maos-a2a-tcp/tests/`: build the frame via `prepare_outbound` (real path), send over the wire, advance the receiver's `with_pinned_consent_clock` past `valid_until_ns`, assert `CODE_CONSENT_EXPIRED`. Pin BOTH ends' consent clocks for determinism.
  - [x] NEW config tests (`config.rs`): `consent_ttl_secs` TOML round-trip + default-applies + `validate()` rejects `0` and `86401`. NEW unit test: an envelope with an explicit `valid_until_ns` is unchanged by `prepare_outbound`.
- [x] **Task 5 — AC5: unify the intent length bound** (AC: 5)
  - [x] `consent_match_key` (`router.rs:260`): replace `<= 1024` with `<= maos_domain::invariants::i8::MAX_CANONICAL_INTENT_LEN`. Add the import. Test 129-byte → band fallback, 128-byte → fine-grained key.
- [x] **Task 6 — AC6: typed errors, TOCTOU, duplicate-peer, intake-timeout** (AC: 6)
  - [x] **G4**: rewrite `map_webpki_error` (`verifier.rs:200-213`) to match `webpki::Error` typed variants; rewrite `HandshakeRetryPolicy::is_retryable` (`mtls.rs:63-74`) to classify on a typed discriminant rather than a lowercased substring. Keep the wire/sentinel taxonomy stable (AC-T5 retry + H2 expired oracles must still pass — run those tests).
  - [x] **G5a**: add `A2ARouterCore::try_new(...) -> Result<Self, A2AError>` (dedupe-detecting); make `new` delegate via `.expect(...)`; switch `TcpA2ATransport::bind` (`transport.rs:150`) to `try_new` and propagate `A2AError`. Test: duplicate `peer_id` → `ConfigInvalid` from `try_new`.
  - [x] **G5b**: add an atomic compare-and-invalidate to `InMemoryTofuPinStore` (hold the `DashMap` `get_mut` entry lock across the boot-nonce read + invalidation) + mirror on the `TofuPinStore` trait if needed; use it in `handle_intake`'s restart path (`router.rs:474-514`) instead of `get_pin` + separate `invalidate_for_restart`. Keep P6 (`a1_security_regression_guards`) GREEN.
  - [x] **G6**: in `serve_connection`, wrap `handle_intake_verified` + `send_response` inside the `timeouts.intake`/`idle` bound (not only `framed.next()`).
- [x] **Task 7 — AC7: gate, discipline, evidence** (AC: 7)
  - [x] Run `a1_security_regression_guards` + `cross_host_consent_v1_5` + all new tests GREEN. CI 50× stress on the new TCP negatives.
  - [x] Assert `maos-kernel-core` byte-identical; `check-workspace-count == 41`; `kloc-check` GREEN (`maos-a2a-core`, `maos-a2a-tcp`); `abi-diff` GREEN; dev-record gates GREEN.
  - [x] Update the Dev Agent Record (File List, Completion Notes, Change Log).


### Review Findings

- [x] [Review][Patch] Trait default `invalidate_if_boot_nonce_differs` preserves TOCTOU race for non-InMemory stores — Removed default body from `TofuPinStore` trait (`tofu.rs:130-142`) to force all implementors to provide an atomic implementation. [decision: patch, team consensus §per-spec]
- [x] [Review][Patch] AC4 within-block order reversed: expiry checked before granter — Reordered `handle_intake` consent block to granter-match FIRST, then expiry (`router.rs:586-640`), matching spec AC4.2. Updated a1 P2 test fixture to have granter == from. [decision: patch, team consensus §per-spec]
- [x] [Review][Patch] `is_retryable` classifies on structured string split instead of typed discriminant — Refactored `A2AError::HandshakeFailed(String)` to carry a `HandshakeFailureClass` enum (`CertExpired`, `CertNotValidYet`, `UnknownIssuer`, `BadCertificate`, `PinMismatch`, `Other`). Updated `map_webpki_error`, `is_retryable`, `to_a2a_error`, and all test assertions. [decision: patch, team consensus §per-spec]
- [x] [Review][Patch] AC6.4 uses `timeouts.idle` for processing+write instead of `timeouts.intake` — Switched the processing+write timeout in `serve_connection` (`transport.rs:456`) from `timeouts.idle` to `timeouts.intake`. [decision: patch, team consensus §per-spec]

#### Patch

- [x] [Review][Patch] Sentinel mismatch: `CERT_EXPIRED` vs `CERTIFICATE_EXPIRED` breaks retry classification [`verifier.rs:212`, `mtls.rs:75`] — Verified the chain: `map_webpki_error` emits `CERT_EXPIRED` → `classify_handshake` matches on `contains("expired")` → `to_a2a_error` emits `CERTIFICATE_EXPIRED`. `is_retryable` already matches `CERTIFICATE_EXPIRED`. Added empty-tag guard.
- [x] [Review][Patch] `resolve_verified_peer` silently drops connections with zero observability [`transport.rs:425-428`] — Added `tracing::warn!` log when peer resolution fails after mTLS handshake.
- [x] [Review][Patch] Double framing validation in `handle_intake_verified` → `handle_intake` [`router.rs:699`, `router.rs:493`] — Removed `request.validate()` from `handle_intake_verified`; framing validation is now performed once by the shared `handle_intake` body.
- [x] [Review][Patch] AC4 reorder test coverage incomplete [`cross_host_consent_v1_5.rs`] — Added `ac4_expired_but_allowlisted_still_rejected` and `ac4_granter_mismatch_with_allowlisted_intent` tests.
- [x] [Review][Patch] Hard-coded `300` literal across ~15 fixture sites instead of `DEFAULT_CONSENT_TTL_SECS` [`adapter.rs`, `a1_security_regression_guards.rs`, `cross_host_consent_v1.rs`, `maos-bin/src/main.rs`, `spirits/mira/tests/`] — Replaced all hard-coded `300` literals with `DEFAULT_CONSENT_TTL_SECS`.
- [x] [Review][Patch] No concurrency stress test for atomic TOCTOU fix (G5b) [`tofu.rs`, `router.rs`] — Added `invalidate_if_boot_nonce_differs_is_atomic_under_race` 50× concurrent stress test.
- [x] [Review][Patch] Missing negative test: malformed request over cross-host verified path [`trust_binding_8_9.rs`] — Added `g8_malformed_request_framing_nack` test.
- [x] [Review][Patch] `frame_with_foreign_granter` only varies `spirit_id`, not `host_id` [`cross_host_consent_v1_5.rs`] — Added `ac2_granter_mismatch_host_id_only` test.
- [x] [Review][Patch] No test for `prepare_outbound` with `consent_envelope: None` [`router.rs`] — Added `prepare_outbound_leaves_none_envelope_unchanged` test.
- [x] [Review][Patch] `is_peer_identity_mismatch` couples intake counter to error code integer [`transport.rs:512-514`] — Changed `handle_intake_verified` to return `(A2AJsonRpcResponse, bool)` so the transport uses `binding_passed` instead of pattern-matching on error codes. Removed `is_peer_identity_mismatch`.
- [x] [Review][Patch] Peer configs not validated in `TcpA2ATransport::bind` [`transport.rs:160-163`] — Added `cfg.validate()` loop before `try_new` in `bind()`.
- [x] [Review][Patch] No TCP-level test for stolen-but-unexpired envelope (granter mismatch on live wire) [`trust_binding_8_9.rs`] — Added `g1_stolen_envelope_granter_mismatch_on_wire` test.
- [x] [Review][Patch] `bind_endpoint_consent_pinned` suppresses clippy with 10 parameters instead of refactoring [`tests/support/mod.rs:327`] — Introduced `BindEndpointConfig` struct and refactored `bind_endpoint_consent_pinned` to accept it. Removed clippy suppression.
## Dev Notes

### The defect, precisely (why this story exists)

The audit charter amendment (epic-8 md, line 5) names the class: **"A2A confused-deputy peer-identity bypass (G8)"**. The live listener verifies the client cert and *learns which peer it belongs to* — `verifier.rs:177`:

```rust
None => match self.pins.find_active_pin_by_fingerprint(&observed) {
    Some(_peer) => Ok(()),   // ← the verified PeerId is bound to `_peer` and DISCARDED
    None => Err(reject("PIN_MISMATCH", ...)),
},
```

Then `serve_connection` (`transport.rs:433`) calls `core.handle_intake(req)`, and `handle_intake` (`router.rs:438-441`) re-derives the peer from the wire:

```rust
let peer_host = match &frame.from.host_id {
    Some(h) => h.clone(),
    None => HostId("loopback".to_string()),  // ← cross-host fallback (G3)
};
```

So the receiver trusts a self-asserted field. A mesh peer with one valid pin can set `from.host_id` to any other Host and be treated as that Host (intent allowlists, consent, audit attribution all key off it). **The fix is to carry the verified identity from the handshake into intake and require the frame to agree with it.** Because rustls's verifier callback returns only `Ok(())`, re-derive the verified peer in `serve_connection` from the post-handshake `peer_certificates()` using the same `find_active_pin_by_fingerprint` oracle — deterministic and no frozen-signature change (AC-A6 safe).

### G-finding → AC → file:line map (verified at HEAD `0b6cbc5`)

| G | What's wrong today | Where | AC |
|---|---|---|---|
| **G8/G3** | TLS-verified peer discarded; intake trusts `frame.from.host_id`; cross-host `"loopback"` fallback | `verifier.rs:177`; `router.rs:438-441` | AC1 |
| **G1** | `consent_envelope.granter` never checked against `frame.from` → stolen-envelope replay | `router.rs` consent block (`530-553`) — no granter check | AC2 |
| **G10** | `prepare_outbound` never sets `valid_until_ns`; `with_fine_grained_intent` builds `None` → expiry is dead code | `router.rs:339-377`; `frame.rs:449` | AC3 |
| **G2** | expiry checked *after* accept-allowlist | `router.rs:516` (accept) before `530` (expiry) | AC4 |
| **G9** | intent length bound `≤1024` ≠ canonical `128` | `router.rs:260` vs `i8.rs:44` | AC5 |
| **G4** | webpki/io errors classified by `Debug`/lowercased-string match | `verifier.rs:202`; `mtls.rs:63` | AC6.1 |
| **G5** | duplicate-peer "last wins" + restart-detection TOCTOU | `router.rs:139`; `router.rs:474-514` | AC6.2/6.3 |
| **G6** | intake timeout wraps only the read, not processing/NACK write | `transport.rs:403` | AC6.4 |

### Source-tree components to touch

- `crates/maos-a2a-core/src/error.rs` — 2 new `A2AError` variants (additive; `#[non_exhaustive]`).
- `crates/maos-a2a-core/src/transport/json_rpc.rs` — 2 new `CODE_*` consts.
- `crates/maos-a2a-core/src/router.rs` — `handle_intake_verified` (new), consent granter block + reorder (AC2/AC4), `prepare_outbound` expiry stamp + `DEFAULT_CONSENT_TTL_NS` (AC3), `consent_match_key` bound (AC5), `try_new`/`new` (AC6.2), atomic restart (AC6.3), `map_a2a_error_to_iac_bus` arms.
- `crates/maos-a2a-core/src/tofu.rs` — atomic compare-and-invalidate primitive (AC6.3).
- `crates/maos-a2a-core/src/mtls.rs` — typed `is_retryable` (AC6.1).
- `crates/maos-a2a-tcp/src/transport.rs` — thread `pins`, derive verified peer, restructure intake arm, wrap timeout (AC1/AC6.4); `bind` → `try_new`.
- `crates/maos-a2a-tcp/src/verifier.rs` — typed `map_webpki_error` (AC6.1).
- Tests: `crates/maos-a2a-tcp/tests/` (g8 negative, real-frame expiry, dup-peer, timeout-coverage), `crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs` (granter, reorder, length-bound).

### What must be PRESERVED (regression surface — do not break)

1. **`maos-a2a/tests/a1_security_regression_guards.rs`** — P1 (TOFU NACK), **P2 (expiry NACK — your reorder must keep this GREEN; it uses an allowlisted intent so expiry still reached)**, P5 (unknown host_id → `CODE_INTERNAL`, no fallback to first peer), P6 (boot-nonce restart → `CODE_SPIRIT_RESTART_DETECTED` + pin invalidated — your atomic rewrite must preserve this exact behavior incl. the NACK `data` keys), P7 (parse-error NACK). These drive `LoopbackA2ARouter` over the shared `A2ARouterCore`.
2. **`maos-a2a-core/tests/cross_host_consent_v1_5.rs`** (8.7) — fine-grained match, band fallback, confused-deputy-at-intent, defense-in-depth, case-insensitive matching, unreachable-entry `warn!`. Your AC4 reorder and AC2 granter check sit upstream of the accept-allowlist these tests exercise — confirm the consent envelopes they build all have `granter == from` (the 8.7 `frame()` helper uses `with_fine_grained_intent(from, …)`, so they do).
3. **`maos-a2a-tcp/tests/{h_guards,t1_live_roundtrip,t3_t6_security,t7_t10_liveness,t11_t12_chaos_absence}.rs`** — especially H2 (pinned-clock expired cert), AC-T3/T4 (`intake_entered == 0` on TLS rejection — your intake-counter move must keep these 0), AC-T5 (retry oracle — your typed `is_retryable`/`map_webpki_error` must keep the same retry classification).
4. **`verify_pinned` / `verify_pinned_sync` byte-identical** (AC-A6). Only the verifier's *error mapping* (`map_webpki_error`) and *construction* may change; the pin-comparison body stays.
5. **`A2ARouterCore::handle_intake` for loopback** — keep its `None → HostId("loopback")` fallback. The cross-host hardening is the NEW `handle_intake_verified`, not a deletion in the shared method.

### Key implementation notes / gotchas

- **`PeerId` ↔ `HostId` binding**: the verified identity is a `PeerId` (`identity.rs:18`, `.as_str()`); `frame.from.host_id` is a `HostId`. The mesh keys them by equal strings (peer config `peer_id == host_id`; memory: *"loopback peer lookup keys HostId==peer_id"*). Compare `verified_peer.as_str() == host_id.as_str()`.
- **Intake counter ordering (AC1.2)**: `intake_entered.fetch_add` is currently at `transport.rs:423`, *before* decode. For the `intake_entered==0` oracle on a forged frame you must decode + check the binding first, and increment only on a pass. Don't double-count the happy path.
- **`peer_certificates()` source**: on the listening side, after `acceptor.accept(tcp)` the `tokio_rustls::server::TlsStream`'s `get_ref().1` is the `rustls::ServerConnection`; `.peer_certificates()` returns the client chain (present because mTLS requires a client cert — the verifier already enforced it). Leaf is index `[0]`.
- **Consent TTL + pinned clock (Decision §D1)**: `prepare_outbound` stamps using `self.consent_now_ns()` (pinned in tests via `with_pinned_consent_clock`, `router.rs:160`) + the per-peer `consent_ttl_secs` (default 300). For the AC3 integration test, pin the SENDER clock at `T0` (so `valid_until = T0 + ttl_ns`) and the RECEIVER clock past `valid_until` so expiry fires deterministically — no wall-clock flake. Use `saturating_add`/`saturating_mul` (a misconfigured huge `consent_ttl_secs` must not wrap to a past instant — though `validate()` caps it at 86400s, defense-in-depth on the arithmetic).
- **Atomic restart (G5b)**: `invalidate_for_restart` (`tofu.rs:295`) uses `get_mut` (entry lock). The race is in the router: it `get_pin`s (clone) then later calls `invalidate_for_restart`. Add e.g. `invalidate_if_boot_nonce_differs(peer, observed_nonce) -> Result<Option<u64> /* prior */, A2AError>` that, holding the `DashMap` `get_mut` lock, reads `pin.boot_nonce`, and if it differs from `observed`, sets `invalidated` and returns the prior nonce — one critical section. Then the router builds the NACK from the returned prior.
- **Duplicate-peer hard-fail (G5a)**: `new` is called in `router.rs` tests and `transport.rs:150`. Make `new` = `Self::try_new(...).expect("A2ARouterCore::new: duplicate peer_id — use try_new")`; `bind` uses `try_new` and `?`-propagates as `A2AError`/`TcpTransportError`.
- **Typed webpki classification (G4)**: `webpki::Error` (crate `rustls-webpki` 0.103) is an enum with variants like `CertExpired`, `CertNotValidYet`, `UnknownIssuer`, `InvalidSignatureForPublicKey`, etc. Match those instead of `format!("{e:?}").contains("Expired")`. Keep mapping to the existing `reject(tag, …)` sentinels so `TcpTransportError::classify_handshake` downstream is unchanged.
- **`is_retryable` typed (G4)**: today it stringly-matches `A2AError::HandshakeFailed(String)`. Minimal-churn option: keep the variant but classify via the sentinel tags the verifier already emits (`CERT_EXPIRED`/`BAD_CERTIFICATE`) by parsing the *structured tag prefix* rather than a free lowercased `contains`; cleaner option: carry a typed `HandshakeFailureClass` enum. Pick the smallest change that removes the fragile substring match and keeps the AC-T5 retry oracle GREEN — document the choice in the Dev Record.

### Testing standards

- Rust + Tokio; `#[tokio::test]` for async. Reuse the `maos-a2a-tcp/tests/support/mod.rs` harness (`Clock::capture`, `mk_ca`, `valid_leaf`/`expired_leaf`, `bind_endpoint`, `peer_cfg`, `pin`, `make_frame`, `no_retry`) and `TcpTimeouts::test_profile()` (≤250ms, H5). Raw-dial pattern (forged-frame send) follows `t7_t10_liveness.rs`: `TlsConnector::from(client_config)` + `Framed::new(tls, length_delimited_codec())` + `framed.send(Bytes)`.
- Negative-test oracles must distinguish the typed error class (NOT a generic IO error) and assert `intake_entered() == 0` where the AC requires it.
- H1 discipline: **no `*.pem`/`*.crt`/`*.key` committed under `tests/`** — generate cert material at runtime via `rcgen` (dev-dep already present).
- CI 50× stress on the new TCP negatives; all timeout-path tests `<2s`.

### Decisions (team consensus)

- **§D1 — Consent TTL is operator-configurable, not a buried constant (3-0; Winston + Murat + security red-team, 2026-06-06).** AC3 adds a per-peer `consent_ttl_secs` (default 300, validated `1..=86400`) mirroring the existing `partition_timeout_secs` pattern, and `prepare_outbound` only synthesizes `valid_until_ns` when absent — an explicit grant always wins.
  - *Per spec:* `valid_until_ns` belongs to the consent grant (`frame.rs:431` already says callers needing real expiry construct the envelope directly); the transport must not override it, and a security window must be operator-tunable with a validated range — not a global magic number (the same loaded-gun pattern the team removed in 8.7 Q5).
  - *Long-term correctness:* the synthesized TTL is marked `// TRANSITIONAL`. The authoritative path is the consent grant populating `valid_until_ns` directly; the cross-host **fail-closed-on-absent-expiry** end-state is owned by **Story 8.8** (sender-completeness gate), deliberately kept out of charter-safe 8.9. 8.9 only makes expiry *live* (closes the dead code), it does not flip fail-closed.
  - *Rejected:* (A) hard-coded `DEFAULT_CONSENT_TTL_NS` constant — invisible, un-tunable security knob; (B) config-only without respecting an explicit grant — would let the wire override the granter.

### Project Structure Notes

- **No new crate.** Workspace stays **41** members. Changes are confined to the two A2A crates + their tests. `maos-kernel-core` byte-identical (zero kernel KLOC — this is the Phase-1 charter-safe boundary).
- `abi-diff` gate scans `maos-spirit-abi` (untouched) → GREEN; additive `maos-a2a-core` public surface (new `A2AError` variants are fine — `#[non_exhaustive]`; new `CODE_*` and `pub fn` are Added-only).
- Watch `kloc-check` for `maos-a2a-core` (was 2644/3000 at 8.7) and `maos-a2a-tcp` (~1250/ceiling) — this story adds modest LOC; keep helpers terse.

### References

- [Source: _bmad-output/planning-artifacts/epics/epic-8-…miranash-v03-v15.md#Story 8.9] — AC sketch + G-finding map (lines 391-406) + Charter Amendment (line 5).
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml] — Story 8.9 registration comment (G8/G10 summary; "UNBLOCKS 8.8").
- [Source: crates/maos-a2a-tcp/src/verifier.rs:166-188] — the discarded `Some(_peer)` (G8).
- [Source: crates/maos-a2a-tcp/src/transport.rs:381-445] — `serve_connection` intake loop (AC1/AC6.4).
- [Source: crates/maos-a2a-core/src/router.rs:425-572] — `handle_intake` validation order (AC2/AC4); `:255-263` `consent_match_key` (AC5); `:339-377` `prepare_outbound` (AC3); `:135-145` dup-peer (AC6.2); `:474-514` restart TOCTOU (AC6.3).
- [Source: crates/maos-domain/src/frame.rs:408-452] — `ConsentEnvelope` + `with_fine_grained_intent` (`valid_until_ns: None` → G10).
- [Source: crates/maos-domain/src/invariants/i8.rs:44] — `MAX_CANONICAL_INTENT_LEN = 128` (AC5).
- [Source: crates/maos-a2a/tests/a1_security_regression_guards.rs] — the regression gate (preserve).
- [Source: crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs] — 8.7 fine-grained suite (preserve).
- [Source: crates/maos-a2a-core/src/error.rs:22-88] — `A2AError` (`#[non_exhaustive]`) for new variants.
- [Source: crates/maos-a2a-core/src/transport/json_rpc.rs:27-47] — `CODE_*` table (next free `-32007`).

## Dev Agent Record

### Agent Model Used

claude-opus-4-8

### Debug Log References

- `cargo test -p maos-a2a-core` — 80 unit + 13 `cross_host_consent_v1_5` GREEN.
- `cargo test -p maos-a2a-tcp` — t1/t3–t6/t7–t10/h_guards/t11–t12 + NEW `trust_binding_8_9` (6) GREEN.
- `cargo test -p maos-a2a` — `a1_security_regression_guards` (P1/P2/P5/P6/P7) + `cross_host_consent_v1` + cert-rotation + restart-NFR-Rel-6 GREEN.
- 50× stress `trust_binding_8_9` → 0 failures; 20× stress `t7_t10_liveness` → 0 failures (AC7.2, no flake).
- Gates: `check-workspace-count` PASS (41=41); `kloc-check` per-crate PASS (maos-a2a-core 2869/3000, maos-a2a-tcp 926/1500, maos-a2a 202/1500); `abi-diff --base abi-baseline/v1-pre-bump.txt` GREEN (`breaking: []`); `check-dev-model-used-populated` PASS; `check-dev-record-completeness` PASS.

### Completion Notes List

**All 7 ACs delivered; `maos-kernel-core` byte-identical (zero kernel KLOC, git-verified).**

- **AC1/G8 (confused-deputy)** — NEW `A2ARouterCore::handle_intake_verified(req, &verified_peer)` binds the frame's `from.host_id` to the TLS-verified peer BEFORE any trust-bearing work → `CODE_PEER_IDENTITY_MISMATCH (-32007)`. `serve_connection` re-derives the verified peer from `tls.get_ref().1.peer_certificates()` → leaf SHA-256 → `find_active_pin_by_fingerprint` (the same oracle `verifier.rs:177` discarded). `intake_entered`/`last_intake` now increment ONLY when the binding passes (forged frame → stays 0). AC1.3/G3: absent `from.host_id` mismatches the verified peer; the shared `None → "loopback"` fallback is unreachable on the wire and is NOT deleted (loopback still needs it).
- **AC2/G1 (granter binding)** — `handle_intake` rejects `envelope.granter ≠ frame.from` (spirit_id AND host_id) → `CODE_CONSENT_GRANTER_MISMATCH (-32008)` with both addresses in `data`.
- **AC4/G2 (reorder)** — the consent block now runs BEFORE the accept-allowlist. **Ordering (review-corrected to match spec AC4.2):** WITHIN the block, granter binding is checked FIRST, then expiry. A stolen-but-unexpired envelope (the real G1 replay) fails closed at the granter gate; an expired envelope with valid granter fails at expiry. The a1 P2 fixture was updated to have `granter == from` so it isolates the expiry path. Final order: framing → peer lookup → TOFU → restart → **(granter → expiry)** → accept-allowlist → Lamport → sink.
- **AC3/G10 (live expiry, Decision §D1)** — NEW per-peer `A2APeerConfig::consent_ttl_secs` (default `300`, validated `1..=86400`, `DEFAULT_CONSENT_TTL_SECS` const). `prepare_outbound` stamps `valid_until_ns = consent_now + ttl` ONLY when an envelope is present AND carries no explicit expiry (authoritative grant wins; `saturating_*`). Marked `// TRANSITIONAL` (fail-closed-on-absent = Story 8.8). `bind()` gained an optional `consent_now_ns: Option<u64>` so on-wire expiry tests pin both ends deterministically.
- **AC5/G9 (length bound)** — `consent_match_key` `<= 1024` → `<= MAX_CANONICAL_INTENT_LEN (128)`, consistent with `A2AIntent::is_canonical`.
- **AC6/G4-G5-G6** — G4: `map_webpki_error` matches `webpki::Error` TYPED variants. **Review-corrected:** `HandshakeRetryPolicy::is_retryable` now classifies on a `HandshakeFailureClass` typed enum (`CertExpired`/`BadCertificate`/`PinMismatch`/`Other`) instead of a string sentinel — per spec AC6.1's "typed discriminant" requirement. G5a: `try_new` hard-fails on duplicate `peer_id`. G5b: atomic `invalidate_if_boot_nonce_differs` (trait default REMOVED — all implementors must provide their own atomic impl). G6: processing+write timeout switched to `timeouts.intake` per spec AC6.4.
- **Typed-error plumbing** — 2 new `#[non_exhaustive]` `A2AError` variants + 2 `CODE_*` (-32007/-32008) Added-only; `map_a2a_error_to_iac_bus` + `interpret_response` arms (no kernel variant — maps to `CrossHostRouteFailure`).
- **`consent_ttl_secs` field propagation** — added to every `A2APeerConfig` literal across the workspace (adapter, a1/cross-host/mira tests, support harness, 9× in maos-bin) so it compiles under `deny_unknown_fields` + default.
- **Pre-existing REDs verified story-neutral:** workspace-total `kloc-check` NFR-Maint-1 alarm (74620 ≫ 16k, red since epic 1; per-crate budgets all GREEN); `maos-mcp` `fixture_replay` feature-gated test-compile break (reproduced at HEAD with changes stashed — untouched by this story). `abi-diff` no-base `HEAD~1` default false-positive is the known 8.3 lesson — the CI `--base` gate is GREEN.

### File List

- `crates/maos-a2a-core/src/error.rs` — +2 `A2AError` variants + `HandshakeFailureClass` enum (review-corrected: `HandshakeFailed` now carries typed class).
- `crates/maos-a2a-core/src/transport/json_rpc.rs` — +`CODE_PEER_IDENTITY_MISMATCH (-32007)`, +`CODE_CONSENT_GRANTER_MISMATCH (-32008)`.
- `crates/maos-a2a-core/src/config.rs` — +`consent_ttl_secs` field, `DEFAULT_CONSENT_TTL_SECS`, `default_consent_ttl_secs`, `validate()` range; +TTL config tests.
- `crates/maos-a2a-core/src/router.rs` — `try_new`/`new`; `consent_match_key` 128-bound; `prepare_outbound` expiry stamp; `handle_intake` consent reorder + granter binding; NEW `handle_intake_verified`; atomic restart call; `interpret_response` + `map_a2a_error_to_iac_bus` arms; fixtures updated.
- `crates/maos-a2a-core/src/tofu.rs` — `invalidate_if_boot_nonce_differs` (review-corrected: trait default REMOVED, `InMemoryTofuPinStore` provides only impl; +50× concurrency stress test).
- `crates/maos-a2a-core/src/mtls.rs` — `is_retryable` on `HandshakeFailureClass` typed enum (review-corrected from string split); updated unit tests.
- `crates/maos-a2a-core/tests/cross_host_consent_v1_5.rs` — +AC2/AC4/AC5 core tests; +`consent_ttl_secs` in fixture.
- `crates/maos-a2a-tcp/src/transport.rs` — thread `pins`; `resolve_verified_peer` (+`tracing::warn!` observability, review-corrected); `handle_intake_verified` returns `(response, binding_passed)` (review-corrected: decoupled from error code); intake binding + counter gate; AC6.4 timeout switched to `timeouts.intake` (review-corrected); peer config `validate()` in `bind` (review-corrected); `bind` → `try_new` + `consent_now_ns`.
- `crates/maos-a2a-tcp/src/verifier.rs` — typed `map_webpki_error`.
- `crates/maos-a2a-tcp/src/error.rs` — `to_a2a_error` emits `HandshakeFailureClass` typed variants (review-corrected).
- `crates/maos-a2a/tests/cert_rotation_chaos_3_host.rs` — updated `is_retryable` assertions for typed enum (review-corrected).
- `crates/maos-a2a-tcp/tests/support/mod.rs` — `bind_endpoint` `None` consent clock; `bind_endpoint_consent_pinned` refactored to `BindEndpointConfig` struct (review-corrected: removed clippy suppression); +`consent_ttl_secs` in `peer_cfg`.
- `crates/maos-a2a-tcp/tests/trust_binding_8_9.rs` — **NEW** — g8/g3/g10/AC3-explicit/g5a/g6 wire oracles + review-added malformed-request framing NACK + stolen-envelope granter-mismatch tests.
- `crates/maos-a2a/src/adapter.rs`, `crates/maos-a2a/tests/{a1_security_regression_guards,cross_host_consent_v1}.rs` — +`consent_ttl_secs` literals.
- `crates/maos-bin/src/main.rs` — +`consent_ttl_secs` (9 literals); daemon `bind` `None` consent clock.
- `spirits/mira/tests/{a2a_pairing,halt_bilateral}.rs` — +`consent_ttl_secs` literals.

### Change Log

| Date | Change |
|---|---|
| 2026-06-06 | Story created (`bmad-create-story`, claude-opus-4-8): A2A trust-binding & consent integrity hardening — closes audit G1/G2/G3/G4/G5/G6/G8/G9/G10 across `maos-a2a-core` + `maos-a2a-tcp`, zero kernel KLOC, unblocks Story 8.8. Status → ready-for-dev. |
| 2026-06-06 | Team consensus §D1 (3-0, Winston + Murat + security red-team) resolved AC3's consent-TTL fork: operator-configurable per-peer `consent_ttl_secs` (default 300, range 1..=86400) + `prepare_outbound` respects an explicit grant; rejected the hard-coded constant. "Per spec + long-term correctness." Folded into AC3 / Task 4 / Dev Notes. |
| 2026-06-06 | Implemented (`bmad-dev-story`, claude-opus-4-8): all 7 ACs closing G1/G2/G3/G4/G5/G6/G8/G9/G10. Status → review. |
| 2026-06-06 | Code review (`bmad-code-review`): adversarial 3-layer review (Blind Hunter + Edge Case Hunter + Acceptance Auditor). 17 patches applied (4 decision-needed → patch per team consensus "per spec + long-term correctness", 13 patch). Key corrections: (a) AC4 within-block order flipped to granter-before-expiry per spec AC4.2, a1 P2 fixture updated; (b) `is_retryable` refactored to `HandshakeFailureClass` typed enum per AC6.1; (c) `TofuPinStore::invalidate_if_boot_nonce_differs` trait default removed; (d) AC6.4 timeout switched to `timeouts.intake`; (e) `handle_intake_verified` returns `(response, binding_passed)` to decouple transport from error codes; (f) 9 new tests added (AC4 coverage, host_id granter, malformed request, stolen envelope, concurrency stress, prepare_outbound None). 163 tests GREEN. Status → done. |
