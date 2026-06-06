---
dev_model_used: claude-opus-4-8
---
# Story 8.6: Ship the Live Cross-Host A2A TCP/mTLS Transport (`maos-a2a-tcp`, two-process)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **v1.5 operator running Mira and Nash on two separate Hosts**,
I want **the live `A2AProfile::CrossHost` transport — a real TCP listener/dialer with operator-managed mTLS (custom TOFU-pinning cert verification), length-delimited JSON-RPC framing over the socket, handshake retry, and real partition-timeout behavior — shipped as a NEW `maos-a2a-tcp` crate over a freshly-extracted `maos-a2a-core` seam**,
so that **the bilateral pair coordinates over a genuine two-process network connection (not the in-process loopback simulation Story 8.5 ships), realizing FR23b cross-Host at v1.5 with the TOFU security model actually enforced on the wire.**

> **Split from Story 8.5 (2026-06-04).** Story 8.5 shipped the **loopback-simulated** bilateral pair — it proved the cross-Host *protocol* (pre-paired `PeerCertFingerprint`s, TOFU `verify_pinned`, ADR-012 consent, rotation chaos) over the real `LoopbackA2ARouter` (in-process `mpsc`). Story 8.6 ships the live two-process *transport* — a **distinct, security-critical networking risk class** (~1000+ LOC of socket + TLS in a NEW crate). The live `A2AProfile::CrossHost` transport is **FULLY ABSENT today**, not scaffolding (see Dev Notes §"What exists vs. what's absent"). Depends on Story 8.5 (done) + Story 6.3 (done).

---

## ⚠️ THREE SPEC CORRECTIONS — read before touching code

The epic-8 ACs were authored when 8.6 was conceived as following Story **8.4** (workspace at 37). Story 8.5 then landed Mira+Nash, moving the workspace to **39**. Three epic assertions are therefore stale or use idealized names. **These corrections override the epic text where they conflict:**

1. **Workspace member count is `39 → 41`, NOT `37 → 39`.** Verified authoritatively: `cargo metadata --no-deps` reports **39 packages today** (the epic's "37" predates 8.5's Mira+Nash merge). Adding `maos-a2a-core` + `maos-a2a-tcp` ⇒ **41**. AC-A2's hardcoded "37 → 39 exactly" assertion **must be written as 41** (re-confirm with `cargo metadata --no-deps --format-version 1 | jq '.packages | length'` at dev time and pin THAT number). The `+2` delta (the real invariant) is unchanged.

2. **The proposed `A2ATransport::dispatch` signature uses placeholder type names that do not all exist.** The epic writes `async fn dispatch(&self, peer: PeerId, frame: A2AJsonRpcRequest) -> Result<A2ANack, TransportError>`. Reality: `PeerId` exists (`maos-a2a/src/identity.rs:11`), but **`A2ANack` and `TransportError` do not** — the real types are `NackResponse`/`NackError` (`transport/json_rpc.rs`) and `A2AError` (`error.rs:88`). `A2ATransport` is a **NEW** trait (not a frozen 8.5 signature), so you have design freedom for *its own* method shape — but bind it to the **real, frozen** surface it wraps. **Recommended:** define the trait to mirror the existing `A2APeerRouter` surface (`adapter.rs:37-70`) so no adapter glue is needed:
   ```rust
   #[async_trait::async_trait]
   pub trait A2ATransport: Send + Sync {
       async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), A2AError>;
       async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse;
       fn local_addr(&self) -> Option<SocketAddr> { None } // readiness hook; loopback returns None
   }
   ```
   If you keep the epic's `dispatch`/`A2ANack`/`TransportError` names, you MUST add them as new core types — do not invent a parallel taxonomy that diverges from `A2AError`/`NackResponse`. Cite the chosen binding to `adapter.rs:255` (`route_outbound`) / `adapter.rs:334` (`handle_intake`) in your AC-A1 evidence (the epic's "CrossHost dispatch match" does not exist as a literal `match A2AProfile::CrossHost` arm — `A2AProfile::CrossHost` is *never dispatched on*; see Dev Notes).

3. **`boot_nonce` and the Lamport clock travel in TWO DIFFERENT fields, not "the same JSON-RPC field."** AC-A6 says both "travel in the EXISTING JSON-RPC field." Grounded reality:
   - `boot_nonce` → **top-level** `A2AJsonRpcRequest.boot_nonce: u64` (`transport/json_rpc.rs`, ~line 44; `#[serde(default)]`).
   - Lamport timestamp → **inside `params`**: `IacFrame.logical_clock: u64` (`maos-domain/src/frame.rs:29`), set on send at `adapter.rs:282` (`frame.logical_clock = self.clock.send_tick()`) and read on receive at `adapter.rs:464` (`self.clock.recv_advance(frame.logical_clock)`).
   The no-rewrapping/no-churn intent of AC-A6 stands — just cite **both** literal field locations in evidence; AC-T1's `decoded.boot_nonce == sent.boot_nonce` and `decoded.lamport == sent.lamport` oracles read these two fields respectively.

---

## Acceptance Criteria

> Source: `epics/epic-8-…miranash-v03-v15.md:228-326`. Reproduced with the three corrections above applied. AC-A* = Winston (architecture/structure); H* = Murat (test harness preconditions); AC-T* = Murat (test/risk).

### Architecture / Structure (AC-A1–AC-A7)

**AC-A1 — Extract `maos-a2a-core`; define the transport seam there; resolve the ceiling by extraction.**
A NEW crate `maos-a2a-core` at `crates/maos-a2a-core/` owns the transport-agnostic surface: it declares `pub trait A2ATransport` (signature per Correction #2 + `fn local_addr(&self) -> Option<SocketAddr>` readiness hook, bound to the real `route_outbound`/`handle_intake` at `adapter.rs:255,334`). It **MOVES (not copies)** the shared substrate, ALL `pub` at the core root: `A2AJsonRpcRequest` + `try_from_bytes` + the response/error types; `handle_intake` and its helper types; `HandshakeRetryPolicy` (`mtls.rs:13`); `RotationDrillReport` + the chaos `rotation`/`harness_3_host` modules (`chaos/rotation.rs:63`); the TOFU surface (`TofuPinStore` trait + `verify_pinned` + `InMemoryTofuPinStore` + `TofuPin`, `tofu.rs`); the `boot_nonce` field (it is a **struct field** on `A2AJsonRpcRequest`, not a free fn — move the struct); `LamportClock` (`transport/logical_clock.rs:11`); the `CODE_PARSE_ERROR` constructor + sibling error-code constants (`transport/json_rpc.rs:27`); `consent` (`ConsentAllowlists`, `A2AConsentEnvelope`); `config` (`A2AConfig`, `A2APeerConfig`, `A2AProfile`); `identity` (`PeerId`, `PeerCertFingerprint`); `error` (`A2AError`); `corpus`.
**And** `maos-a2a` retains ONLY `LoopbackA2ARouter` as `impl A2ATransport`, depends on `maos-a2a-core`, and `pub use`-re-exports every moved symbol so no downstream import path (`maos-bin`, `spirits/mira`, `spirits/nash`, tests) breaks.
**And** after extraction `cargo xtask kloc-check` is GREEN for BOTH `maos-a2a` (now < 1500; today **2550** by the kloc metric / 3400 raw `wc -l`) and `maos-a2a-core` (add its ceiling to `xtask/kloc.toml`; record the post-extraction count in evidence), with NO ceiling bump to any existing crate (`maos-a2a` stays at `xtask/kloc.toml:67 = 1500`).

**AC-A2 — `maos-a2a-tcp` is the second `A2ATransport` impl; dependency arrow points only at core.**
A NEW crate `maos-a2a-tcp` at `crates/maos-a2a-tcp/` declares `pub struct TcpA2ATransport` with `impl A2ATransport`. Its `Cargo.toml` first-party/transport deps are EXACTLY: `maos-a2a-core` (NOT `maos-a2a`, NOT `maos-kernel-core`) + `tokio` + `tokio-rustls` + `rustls` + `tokio-util` (with the `codec` feature) + `maos-domain` (for `IacFrame`/`HostId` — these ride through `maos-a2a-core`'s re-exports, prefer those). Dev-deps: `rcgen = "0.13"`, `tokio` test features, `tempfile`.
**And** `maos-a2a-core` contains ZERO references to `TcpListener`/`TcpStream`/`tokio_util`/`tokio_rustls` (grep-asserted; interlocks AC-T13).
**And** the workspace member count moves **39 → 41** exactly (Correction #1 — pin the live `cargo metadata` count, not the literal "39"), and `abi-diff` is **Added-only** (the AC-A1 `pub use` re-exports preserve symbol paths so nothing reads as Removed).

**AC-A3 — `TofuPinningVerifier`: named deliverable bridging WebPKI into `verify_pinned`.**
`maos-a2a-tcp` provides a NEW named type `TofuPinningVerifier` implementing `rustls::client::danger::ServerCertVerifier` (dialing side) AND a `rustls::server::danger::ClientCertVerifier` twin (listening side) — **both directions MUST pin**. Each `verify_*_cert` FIRST performs WebPKI validation (delegating to a wrapped stock verifier) and ONLY on success calls `maos_a2a_core::…verify_pinned` against the leaf fingerprint + `InMemoryTofuPinStore`, returning `rustls::Error` on mismatch. **The wrapped verifier is selected by `ca_roots` (LOCKED Option A — Dev Notes):** when `Some(bundle)` (default), wrap `WebPkiServerVerifier`/`WebPkiClientVerifier` built from that root store (full chain-to-root); when `None`, wrap a structural/validity-only verifier (well-formed + not-expired + not-yet-valid against the leaf, no chain). In BOTH branches the WebPKI step runs FIRST and `verify_pinned` runs ONLY on its success — `None` is NOT a `danger_accept_any` noop.
**And** it is wired via `.dangerous().with_custom_certificate_verifier(...)` so it runs on the REAL handshake (not post-connection), consuming the EXISTING `verify_pinned` signature unchanged (`tofu.rs:90,191` — `async fn verify_pinned(&self, peer, observed) -> Result<…>`; AC-A6). **Note:** `verify_pinned` is `async` but rustls verifier callbacks are **sync** — you must bridge (block-on a current-thread handle, or pre-resolve the pin synchronously from `InMemoryTofuPinStore` inside the verifier). Document the bridge; do NOT change `verify_pinned`'s signature.
**And** the WebPKI-then-pin ordering is deliberate: an expired/malformed cert is rejected before the pin is consulted, preserving the `CERTIFICATE_EXPIRED` retry path (AC-T5).

**AC-A4 — `LengthDelimitedCodec` framing between socket and `try_from_bytes`.**
`TcpA2ATransport` wraps the `tokio_rustls` stream in `tokio_util::codec::Framed` with `LengthDelimitedCodec` (4-byte big-endian `u32` length prefix, explicit `max_frame_length` cap — **1 MiB**). Each inbound decoded frame is handed to `A2AJsonRpcRequest::try_from_bytes` → `handle_intake`; a frame that decodes structurally but fails JSON parse yields `CODE_PARSE_ERROR` (interlocks AC-T2).
**And** length-prefix framing is the ONLY message-boundary mechanism — no newline-delimited or read-to-EOF fallback exists (grep-asserted).

**AC-A5 — Cert/PKI provisioning, config schema, and `maos-bin` binding.**
A `TcpA2AConfig` struct (deserialized via the existing config layer; `#[serde(deny_unknown_fields)]` consistent with `A2AConfig`) with EXACTLY these fields: `listen_addr: SocketAddr` (`:0` in tests, readback via `local_addr` — H3); `own_cert_chain: PathBuf` (PEM); `own_private_key: PathBuf` (PKCS#8 PEM); `peer_pins: Vec<PinnedFingerprint>` (pre-paired peer leaf-cert fingerprints loaded into `InMemoryTofuPinStore` at startup — makes ADR-012 "pre-paired fingerprints" real; reuse `PeerCertFingerprint`, `identity.rs:46`); `handshake_timeout: Duration` (default 30s, MUST be injectable — H5); `ca_roots: Option<PathBuf>` (WebPKI trust bundle — **LOCKED Option A:** `Some` ⇒ CA-chain-to-root THEN pin, defense-in-depth, the test/prod corpus **default**; `None` ⇒ pin-only, leaf validity/structure THEN pin, the FR23a self-signed bilateral posture; both supported, separately tested — see Dev Notes).
**And** `maos-bin` gains a daemon-mode binding that, when `TcpA2AConfig` is present, constructs `TcpA2ATransport`, registers it as the `A2ATransport` for `CrossHost` dispatch, and binds the listener — with `maos-kernel-core` receiving NO new public fn (reuse existing Spirit/router wiring). The binding lands in **`crates/maos-bin/src/main.rs`** (the monolithic 6120-line composition root where the `smoke-a2a-loopback-6-3` arm already lives). Name the exact fn/section in evidence.

**AC-A6 — No protocol-surface churn: 8.5's signatures consumed unchanged.**
The 8.5-frozen signatures are called byte-identically (asserted by `abi-diff` of `maos-a2a-core`'s public surface showing them **unchanged** — not Added/Removed/Modified): `verify_pinned(...)`; the ADR-012 consent path (consent rides the EXISTING frame/JSON-RPC structure, NOT a new TCP-specific field — `route_outbound` validates send-allowlist at `adapter.rs:263`, `handle_intake` validates accept-allowlist at `adapter.rs:425-436`); `handle_intake(...)` and `A2AJsonRpcRequest::try_from_bytes(...)`; `boot_nonce` (top-level `A2AJsonRpcRequest.boot_nonce`) + the Lamport `logical_clock` (`IacFrame.logical_clock`) travel in their existing fields (Correction #3) with no re-wrapping.
**And** if any of these would require a signature change to make TCP work, that is a RED flag the seam (AC-A1) is misplaced — reject the change and move the seam instead.

> **DO NOT widen the consent vocabulary in this story.** Story 8.5 surfaced that `ConsentAllowlists` holds a free-form `Vec<A2AIntent>` but the router enforces only the 3-band `IntentClass` projection `{highprivilege, standard, readonly}` via `frame_intent_str()` → `a2a_consent_intent_str()` (`adapter.rs:144-146`), so a specific intent string silently never matches. **That is Story 8.7's job** (`8-7-fine-grained-typed-intent-consent-vocabulary-over-maos-a2a-core`, already `ready-for-dev`, lands against the `maos-a2a-core` you create here). 8.6 is deliberately churn-free — a consent-fn signature change is a RED flag (AC-A6). The coarse 3-band gate is accepted v1.5 behavior.

**AC-A7 — Kernel-KLOC zero-delta + doc reconciliation.**
`maos-kernel-core` is **byte-identical** to its pre-story state (assert exact equality, as Story 8.4 did with 15505 LOC; interlocks AC-T12); the kernel-KLOC sentinel is GREEN; `4-kernel-design.md` is reconciled to describe `maos-a2a-core` (protocol seam) + `maos-a2a-tcp` (live wire) with the dependency arrows `maos-a2a-tcp → maos-a2a-core ← maos-a2a` drawn explicitly; the workspace-count sentinel updated to 41; all discipline gates GREEN at HEAD (not flipped-while-red).

### Test harness preconditions (H1–H6 — referenced by every AC-T)

- **H1 — Time-relative cert fixtures (`rcgen`, generated at setup, never committed).** Helper `mk_cert(role, not_before_offset, not_after_offset)` issues certs at test-setup via `rcgen` (already a `maos-a2a` dev-dep, `Cargo.toml:26`), offsets from a single `T0 = SystemTime::now()` captured once per test: at minimum `valid` (T0−1h..T0+1h), `expired` (T0−2h..T0−1h), `not_yet_valid` (T0+1h..T0+2h) **issued as a chain from a real `ca_good` root** (for the default `Some(ca_good)` corpus), plus an **independent `ca_evil` root** (feeds AC-T4's chain-rejection), plus a **self-signed leaf** (feeds the `ca_roots = None` pin-only tests AC-T4b/smoke). rcgen issues all of these. **No dated `.pem` committed.** Guard: `git ls-files` under the test dir yields zero `*.pem`/`*.crt`/`*.key`.
- **H2 — Single pinned clock.** TLS validation wall-clock and the rotation-drill injected clock are the SAME injected `Clock` (default `T0`). Guard: shared-`Arc` identity check; no test reads `SystemTime::now()` after `T0` for an expiry decision. (rustls verifies cert validity against a `UnixTime` you can supply via the verifier — feed `T0`.)
- **H3 — Ephemeral port + readback.** Listeners bind `127.0.0.1:0`; the test reads `local_addr()` and dials THAT. Guard: no host:port literal in networking tests except `:0`.
- **H4 — Readiness handshake, not sleep.** Server sends its resolved `SocketAddr` over a `oneshot` AFTER `local_addr()` succeeds; client awaits before dialing. Guard: zero `sleep` in setup paths (any present must be `tokio::time::advance` under `start_paused`).
- **H5 — Injectable timeouts.** Intake/handshake/idle timeouts are constructor params with a `test_profile()` ≤ 250ms (the 30s prod default lives only behind the prod constructor); timeout-path tests complete `< 2s` wall.
- **H6 — Deterministic teardown.** Every spawned process uses `Command::kill_on_drop(true)`; every spawned task is held by a `JoinHandle` aborted in a drop guard. Guard: a teardown-leak test spawns→drops→asserts the bound port is re-bindable within 250ms.

### Test / Risk (AC-T1–AC-T13)

- **AC-T1 — Live mTLS round-trip over a real socket (happy path; gates everything below).** Two endpoints bound `127.0.0.1:0` (H3), pre-paired fingerprints, `valid`/`ca_good` certs (H1) under the pinned clock (H2). Client dials readback (H4), completes mTLS handshake, sends one well-formed `CrossHost` consent frame (ADR-012). **Oracle:** `ack.code == ACK`; `decoded.boot_nonce == sent.boot_nonce` (top-level field); `decoded.lamport == sent.lamport` (`params.logical_clock`); `observed_fp == pinned_fp`. No latency assertion.
- **AC-T2 — Malformed frame over a live, authenticated connection → typed NACK.** On an established connection (AC-T1), send bytes that fail `try_from_bytes` (variant 1: truncate a valid frame to half its length-delimited payload; variant 2: corrupted discriminant byte). **Oracle:** `nack.code == CODE_PARSE_ERROR` both variants; a follow-up valid frame on the SAME connection returns ACK (codec resynced / not poisoned).
- **AC-T3 — TOFU pin mismatch (valid cert, wrong identity) → handshake REJECTED. *(MANDATORY — whole security model.)*** Server presents `valid`/`ca_good` cert whose fingerprint is NOT pinned (pin `fp_A`, server presents `fp_B ≠ fp_A`). **Oracle:** dial returns `Err` classified as TOFU pin mismatch (NOT generic IO); server `intake_entered: AtomicUsize == 0`; **no NACK frame** (rejection at TLS layer) — app read side observes connection-closed. Primary negative test for AC-A3's verifier.
- **AC-T4 — Wrong CA (valid-but-untrusted root) → handshake REJECTED at the chain layer.** **Runs under `ca_roots = Some(ca_good)` (the default posture).** Server presents a `ca_evil`-issued cert, otherwise well-formed and in-validity — and to prove the discriminating clause, its fingerprint MAY even be coincidentally pinned. **Oracle:** dial `Err` = bad-cert/untrusted-issuer (NOT pin-mismatch); `intake_entered == 0`; connection-closed. Asserts the WebPKI→TOFU ordering: an untrusted-CA leaf is rejected at the chain layer *even if its fingerprint is pinned*. This sub-case is **only constructible in chain mode** — it is the reason `Some` is the default.
- **AC-T4b — Pin-only posture: unpinned leaf → REJECTED at the pin step.** **Runs under `ca_roots = None`.** Server presents a structurally-valid, in-validity self-signed leaf whose fingerprint is NOT pinned. **Oracle:** dial `Err` = TOFU pin mismatch; `intake_entered == 0`. In pin-only mode AC-T4's chain-rejection is N/A by construction; AC-T3/the pin step IS the trust oracle here. Pair with a `None`-mode happy-path smoke (a `valid`/pinned leaf round-trips with no roots configured) and a `None`-mode AC-T5 expiry check (validity gate still fires without a chain). ~3 targeted pin-only tests total — NOT a full re-run of the security suite.
- **AC-T5 — Expired / not-yet-valid cert → REJECTED, retry policy engages on cert codes.** Server presents `expired` (case 2: `not_yet_valid`) under pinned clock T0 (H2). Client dials with `HandshakeRetryPolicy` (`test_profile` backoff, ≤3 attempts). **Oracle:** observed retries `== policy.max_attempts` (proves retry fired on `CERTIFICATE_EXPIRED`); terminal `Err` cert-class; `intake_entered == 0`. `HandshakeRetryPolicy::is_retryable` already returns true ONLY for `BAD_CERTIFICATE`/`CERTIFICATE_EXPIRED` (`mtls.rs:63`).
- **AC-T6 — MITM cert-swap after pin (TOFU defends rotation) → REJECTED.** Client has pinned `fp_A` from a prior connection (run AC-T1); a new connection presents `fp_C ≠ fp_A` issued by `ca_good`. **Oracle:** dial `Err` (TOFU mismatch); `intake_entered == 0`; pin store still holds `fp_A` (`get_pin(peer) == fp_A` — not silently overwritten). Distinct from AC-T3: a valid prior pin exists and must win.
- **AC-T7 — Slow-loris / stalling intake → bounded timeout, task does NOT hang. *(MANDATORY — the test 8.5 deferred TWICE.)*** Authenticated connection (AC-T1), intake timeout ≤ 250ms (H5). Client (a) advertises N bytes, stalls after N−1; (b) sends zero application bytes; (c) dribbles one byte/100ms past idle timeout. **Oracle:** whole test `< 2s` (H5); server intake `JoinHandle::is_finished() == true` after window; no growth in an active-intake gauge after teardown. **No third deferral.** *(Note: `CODE_TIMEOUT` does NOT exist today — `json_rpc.rs` has only PARSE_ERROR/INVALID_REQUEST/METHOD_NOT_FOUND/INTENT_DENIED/PIN_MISMATCH_NOT_PINNED/CONSENT_EXPIRED/SPIRIT_RESTART_DETECTED/INTERNAL, lines 27-37. Add `CODE_TIMEOUT` and `CODE_FRAME_TOO_LARGE` as NEW constants in the moved `maos-a2a-core` json_rpc module — this is an additive ADD, abi-diff Added-only, and does NOT violate AC-A6 (which freezes existing signatures, not the addition of new error codes).)* *(This is the exact gap 8.5 logged: "A2A timeout leaves handle_intake future dangling — tokio::time::timeout returns PartitionTimeout on expiry but handle_intake may still be executing," `adapter.rs:289-298`. On TCP you OWN the read side — abort the per-connection task, do not just race a timeout against a dangling future.)*
- **AC-T8 — Oversized / unbounded frame → rejected before allocation blow-up.** Client advertises a length-delimited frame exceeding the codec cap (header claims `MAX+1`). **Oracle:** `nack.code` is the cap code (`CODE_FRAME_TOO_LARGE` — add the constant if absent; or the codec's `max_frame_length` error); peak intake buffer ≤ cap (test allocator counter, OR assert reject fires after only the header is sent). No OOM, no hang. (`LengthDelimitedCodec` enforces `max_frame_length` automatically — verify it errors rather than buffers.)
- **AC-T9 — Plaintext client hits the TLS port → rejected, no panic, no hang.** Raw `TcpStream` writes plaintext bytes (no ClientHello). **Oracle:** `intake_entered == 0`; a follow-up real mTLS connection on the SAME listener succeeds (accept loop didn't die); `< 2s`.
- **AC-T10 — Half-open connection (client drops mid-handshake / mid-frame) → cleaned up.** Client establishes TCP, begins TLS, then drops after a partial ClientHello. **Oracle:** active-connection gauge returns to its pre-connection value within the timeout; accept loop still live (follow-up valid connection succeeds).
- **AC-T11 — REAL-socket cert-rotation chaos as its OWN AC.** 3-endpoint topology (H3/H4) over **real sockets and real TLS handshakes** — explicitly NOT the synthetic `RotationDrillReport` timing model — under one pinned clock (H2), `mk_cert` issuing `fp_old`/`fp_new` at deterministic offsets. Drill rotates each endpoint's serving cert `fp_old → fp_new` while peers hold live pins and re-pin per the documented rotation protocol. **Oracle:** final pin-store state on all 3 == `fp_new`; full NxN reachability ACK post-convergence; retry counters bounded by `max_attempts`; grep guard `RotationDrillReport` NOT referenced in this AC's module (it is the OLD synthetic class). **Scope note:** the synthetic `cert_rotation_chaos_3_host.rs` may remain as a fast smoke but MUST NOT be the evidence for this AC.
- **AC-T12 — Falsifiable absence-assertions.** (a) kernel performs **zero** auto-retry — kernel-side retry counter `== 0` (the ONLY retrier is `HandshakeRetryPolicy` on the transport side); (b) `maos-kernel-core` is **byte-identical** to pre-story state. **Oracle:** (a) `kernel_retry_count: AtomicUsize == 0` (or a fail-on-call test double); (b) a checksum / `git diff --stat`-empty gate for the crate (analogous to Story 8.4's 15505 check). Prose is NOT acceptable evidence — the checksum/diff is.
- **AC-T13 — CI determinism conformance.** The new `maos-a2a-tcp` integration suite is hermetic: no hardcoded ports (H3), no fixed sleeps in setup (H4), injectable timeouts (H5), kill-on-drop teardown (H6), time-relative certs (H1) under one clock (H2). **Oracle:** H1–H6 guard tests all pass AND a CI repeat-runner (looped 50× / nextest `--retries 0 --test-threads=8`) is **100% green** — any single flake fails the AC. This is the gate that prevents this security story becoming the next §A2-style CI-only-flake debt.

> **Red-phase order (from the epic, line 328):** start with the H1–H6 harness + **AC-T1** (only happy path); then **AC-T3** and **AC-T7** most change the security posture and most expose missing verifier/timeout wiring. AC-T3–T6 consume AC-A3's `TofuPinningVerifier` as the unit under test — coordinate its error taxonomy (concrete enum variants) so the "TOFU-mismatch vs bad-cert" oracles match.

---

## Tasks / Subtasks

- [x] **Task 1 — Carve `maos-a2a-core` by extraction (AC-A1, AC-A6, AC-A7).**
  - [x] Create `crates/maos-a2a-core/` (Cargo.toml `version.workspace = true`); add to root `Cargo.toml` `members` (keep alphabetical-ish grouping with the other `crates/maos-a2a*`).
  - [x] **Move (git-mv the modules, then re-home)** the substrate listed in AC-A1 from `maos-a2a` into `maos-a2a-core`, all `pub` at the core root: `config`, `identity`, `consent`, `error`, `tofu`, `mtls`, `transport/{json_rpc,logical_clock}`, `chaos/*`, `corpus`, and the `A2APeerRouter` trait + `handle_intake`/`route_outbound` helper fns + `frame_intent_str`. Define `pub trait A2ATransport` here (Correction #2).
  - [x] In `maos-a2a`: keep ONLY `LoopbackA2ARouter` (`adapter.rs:81-…`) as `impl A2ATransport`; `maos-a2a` depends on `maos-a2a-core`; add `pub use maos_a2a_core::{…}` re-exports for EVERY moved symbol so `maos-bin`, `spirits/mira`, `spirits/nash`, and existing tests compile unchanged.
  - [x] `cargo build --workspace && cargo test -p maos-a2a -p maos-a2a-core` green; existing `spirits/mira` `a2a_pairing.rs` and `spirits/nash` tests green with no edits.
  - [x] Add `maos-a2a-core` ceiling to `xtask/kloc.toml`; run `cargo xtask kloc-check` → GREEN for both crates (`maos-a2a` < 1500); record both counts in evidence.
  - [x] `cargo public-api`/`abi-diff` of `maos-a2a` shows **Added-only** (re-exports preserve paths); `maos-a2a-core` is a brand-new surface.
- [x] **Task 2 — `maos-a2a-tcp` crate skeleton + `TcpA2ATransport` (AC-A2, AC-A4).**
  - [x] Create `crates/maos-a2a-tcp/`; deps EXACTLY per AC-A2; add to workspace `members`.
  - [x] `pub struct TcpA2ATransport { … }` with `impl A2ATransport`; prod + `test_profile()` constructors (H5 injectable timeouts).
  - [x] `LengthDelimitedCodec` (`max_frame_length = 1 MiB`, 4-byte BE `u32` prefix) over the `tokio_rustls` stream via `Framed`; inbound decoded bytes → `A2AJsonRpcRequest::try_from_bytes` → `handle_intake`; outbound JSON-RPC serialized through the codec. Grep-assert no newline/EOF fallback.
  - [x] Accept loop: per-connection `tokio::spawn` with `JoinHandle` held in a drop-guard registry (H6); `local_addr()` returns the bound `SocketAddr`.
- [x] **Task 3 — `TofuPinningVerifier` (AC-A3).**
  - [x] Implement `rustls::client::danger::ServerCertVerifier` + `rustls::server::danger::ClientCertVerifier`; WebPKI-structural-first then `verify_pinned` (sync bridge — document it; do NOT change `verify_pinned`).
  - [x] Wire both `ClientConfig`/`ServerConfig` via `.dangerous().with_custom_certificate_verifier(...)`; feed `T0` as the validation clock (H2).
  - [x] Define the concrete error taxonomy (TOFU-mismatch vs bad-cert vs expired) the AC-T3–T6 oracles read.
- [x] **Task 4 — `TcpA2AConfig` + `maos-bin` daemon binding (AC-A5).**
  - [x] `TcpA2AConfig` (exact fields, `deny_unknown_fields`) in `maos-a2a-tcp`; load `peer_pins` into `InMemoryTofuPinStore` at startup; PEM/PKCS#8 loaders for `own_cert_chain`/`own_private_key`.
  - [x] In `crates/maos-bin/src/main.rs`: when `TcpA2AConfig` present, construct `TcpA2ATransport`, register as the `CrossHost` `A2ATransport`, bind the listener — NO new `maos-kernel-core` public fn. Name the fn/section in evidence.
  - [x] Implement the LOCKED Option-A `ca_roots` posture (Dev Notes): default `Some(ca_good)` wraps a `WebPki*Verifier`; `None` wraps a validity/structure-only verifier with the SAME ordered prelude (NOT a `danger_accept_any` noop). Shared error taxonomy across both branches. Add the grep/guard against fail-open.
- [x] **Task 5 — H1–H6 harness + AC-T1 happy path (AC-T1, AC-T13/H*).**
  - [x] `mk_cert` (rcgen, time-relative, `ca_good`/`ca_evil`); single `T0`; `oneshot` readiness; ephemeral ports; kill-on-drop/JoinHandle teardown; `test_profile()` timeouts. Add the H1–H6 guard tests.
  - [x] AC-T1 live round-trip; assert `boot_nonce` (top-level) + `lamport` (`params.logical_clock`) byte-equal + `observed_fp == pinned_fp`.
- [x] **Task 6 — Security negative tests (AC-T3, AC-T4 chain-mode, AC-T4b pin-only-mode, AC-T5, AC-T6) — do AC-T3 next after T1.** Default-posture (`Some(ca_good)`) owns the full suite incl. AC-T4's coincidentally-pinned-`ca_evil` sub-case; add the ~3 targeted `None`-mode tests (AC-T4b + pin-only happy-path smoke + pin-only expiry). Verify TOFU-mismatch vs bad-cert/untrusted-issuer error variants are distinguishable in both branches.
- [x] **Task 7 — Liveness/DoS tests (AC-T7 slow-loris [MANDATORY], AC-T8 oversized, AC-T9 plaintext, AC-T10 half-open).**
- [x] **Task 8 — Real-socket rotation chaos (AC-T11) + falsifiable-absence gates (AC-T12).**
- [x] **Task 9 — Smoke arm + CI determinism (AC-T13, AC-A7).**
  - [x] Add a `smoke-a2a-tcp-8-6` arm to `crates/maos-bin/src/main.rs` (mirror the `smoke-a2a-loopback-6-3` arm) demonstrating a real two-process Mira(host_a)→Nash(host_b) advisory over live TCP/mTLS; wire it into `.github/workflows/discipline.yml` (mirror `smoke-founder-loop-8-4` at `discipline.yml:1364`). Update the discipline job-count sentinel if one gates it.
  - [x] CI repeat-runner (50× / nextest `--retries 0 --test-threads=8`) 100% green.
- [x] **Task 10 — Doc reconciliation + kernel byte-identity (AC-A7, AC-T12).**
  - [x] `git diff --stat` shows `crates/maos-kernel-core/` empty; checksum gate green.
  - [x] Reconcile `architecture-maos-minimal-opus/4-kernel-design.md`: add `maos-a2a-core`/`maos-a2a-tcp`, the `maos-a2a-tcp → maos-a2a-core ← maos-a2a` arrows, workspace count 41.
  - [x] All discipline gates GREEN at HEAD (not flipped-while-red).

---

## Dev Notes

### What exists vs. what's absent (the core premise)

Story 6.3 + 8.5 built the **A2A protocol substrate** but never a live socket. Grounded:

- `LoopbackA2ARouter` is **in-process `mpsc` only** — `route_outbound` (`adapter.rs:255-332`) hands the frame straight to `handle_intake` (`adapter.rs:334-480`) through an in-memory sink; **no `TcpListener`/`TcpStream` anywhere in `maos-a2a`**.
- `A2AProfile::CrossHost` (`config.rs:12-69`) is an **enum variant never dispatched on** — there is NO `match A2AProfile::CrossHost => …` in production code (only config-parse tests at `config.rs:126,134`). The epic's "CrossHost dispatch match" is aspirational; bind `A2ATransport` to the profile-agnostic `route_outbound`/`handle_intake` instead.
- Explicit deferral markers you are now closing: *"the cross-Host TCP connector at v0.7"* (`transport/json_rpc.rs:127-130`); *"behavioral integration test against a real stalling TCP intake … deferred to follow-up"* (`tests/cross_host_consent_v1.rs:177-185`); FR23b *declared-not-implemented* (`src/lib.rs:13`).
- `maos-bin` has **no daemon A2A composition / listen-address wiring** today — AC-A5 adds it.

So 8.6 is **purely additive wiring + a new crate**: you reuse 8.5's frozen protocol verbatim and add only the wire (sockets, rustls handshake, codec, config, daemon binding).

### Reuse map — call these, do not reinvent (all move into `maos-a2a-core`)

| Need | Existing symbol | Location |
|---|---|---|
| Parse one frame from bytes → NACK on bad JSON | `A2AJsonRpcRequest::try_from_bytes` | `transport/json_rpc.rs:131` |
| Receiver-side validation (allowlist, TOFU, restart, consent-expiry, clock) | `…::handle_intake` | `adapter.rs:334-480` |
| Sender-side validation + clock tick | `…::route_outbound` | `adapter.rs:255-332` |
| TOFU verify (async) | `TofuPinStore::verify_pinned` | `tofu.rs:90,191` |
| In-memory pin store | `InMemoryTofuPinStore` | `tofu.rs:122` |
| Pin record (holds `boot_nonce`) | `TofuPin` | `tofu.rs` |
| Cert-code retry policy (`is_retryable` ⇒ BAD_CERTIFICATE/CERTIFICATE_EXPIRED only; `max_attempts=4`) | `HandshakeRetryPolicy` | `mtls.rs:13,63` |
| Lamport clock (`send_tick`/`recv_advance`) | `LamportClock` | `transport/logical_clock.rs:11,28,35` |
| Parse-error code (−32700) + siblings | `CODE_PARSE_ERROR` etc. | `transport/json_rpc.rs:27` |
| Consent allowlists + intent projection | `ConsentAllowlists`, `frame_intent_str`→`a2a_consent_intent_str` | `consent.rs`, `adapter.rs:144-146` |
| Peer identity + fingerprint (`sha256:<hex64>`) | `PeerId`, `PeerCertFingerprint` | `identity.rs:11,46` |
| Synthetic rotation report (NOT for AC-T11 evidence) | `RotationDrillReport` | `chaos/rotation.rs:63` |
| Error enum | `A2AError` | `error.rs:88` |

The wire types that already exist serde-complete but are **never socketed**: `A2AJsonRpcRequest`/`A2AJsonRpcResponse`/`AckResponse`/`NackResponse`/`NackError` (`transport/json_rpc.rs`). You serialize/deserialize these THROUGH the `LengthDelimitedCodec` — the JSON-RPC layer is done; only the socket funnel is new.

### `ca_roots` security-posture fork — ✅ LOCKED: Option A (team consensus 2026-06-04)

> **Decision: support BOTH postures; default `ca_roots = Some(ca_good bundle)` (WebPKI chain-to-root THEN TOFU pin = defense-in-depth); keep `None` (pin-only) as a first-class, separately-tested operator posture.** Unanimous across Winston (architect), Murat (TEA), and security red-team. Rationale recorded below; decided on "spec fidelity + long-term correctness."

`TcpA2AConfig.ca_roots: Option<PathBuf>` (AC-A5) selects which stock verifier `TofuPinningVerifier` wraps:

- **`Some(bundle)` — DEFAULT (CA-chain + pin, defense-in-depth):** Full WebPKI chain-to-root THEN pin. AC-T4's `ca_evil` cert is rejected at the **chain layer before the pin is consulted** — this is the ONLY posture in which AC-T4's discriminating oracle ("untrusted-CA rejected *even if a fingerprint were coincidentally pinned*", epic line 299) is physically constructible. The test corpus default.
- **`None` (pin-only, FR23a self-signed posture):** TOFU pin is the sole trust anchor; certs are operator-supplied self-signed (matches `mtls.rs` which is already verifier-driven with no roots store, and §7.2 "operator names the fingerprint"). The WebPKI step **still runs** — reduced to validity/structure only (well-formed, not-expired, not-yet-valid against the leaf). AC-T4 is **N/A by construction** here (no chain to reject); **AC-T3 is the trust oracle**, realized as AC-T4b (unpinned leaf → rejected by pin step).

**Why Option A (not B or C):**
- **Spec:** B makes AC-T4's discriminating half *untestable* (collapses T4 into T3); C mandates a CA bundle that FR23a/§7.2 explicitly say need not exist and breaks the self-signed Mira/Nash bilateral model the corpus is built on.
- **Long-term correctness:** the pin is the load-bearing MITM anchor; the chain is a pluggable *subordinate* outer skin that can only ever reject (never admit what the pin wouldn't). An operator maturing into managed PKI at the v2.0 10-host mesh (NFR-Sec-13) flips `Some(...)` for chain-scoped rotation without re-pinning; a self-signed operator stays `None` and loses nothing.
- **Zero ABI churn (AC-A6 safe):** the fork lives ENTIRELY in `maos-a2a-tcp`'s verifier construction; `verify_pinned` (`tofu.rs:90`) and its `EPinMismatch` taxonomy are consumed byte-identically in both modes. Does not touch `maos-a2a-core` or the `maos-a2a-tcp → maos-a2a-core` dep arrow; `abi-diff` stays Added-only.
- **Determinism (AC-T13 safe):** chain validation is pure over `(leaf, intermediates, roots, T0)`; H2's single pinned `T0` clock covers it. No new wall-clock/DNS/entropy. 50×/8-thread green is equally satisfiable.

**HARD CONDITIONS (all three reviewers flagged — enforce these):**
1. **`None` ≠ skip checks ≠ fail-open.** The `None` branch keeps the *exact same ordered* `TofuPinningVerifier` prelude (validity + structure → then pin); ONLY the trusted-root chain step is gated on `ca_roots.is_some()`. Add a grep/guard that the `None` branch is NOT a `danger_accept_any`-style noop. Two divergent verifier code paths is a smell — escalate.
- 2. **Shared error taxonomy.** The verifier's concrete error variants must distinguish "TOFU-pin-mismatch" vs "bad-cert/untrusted-issuer" vs "expired/not-yet-valid" identically in BOTH branches, so AC-T3/T4/T4b/T5 oracles don't alias.
3. **WebPKI-first ordering must hold in both branches** (chain/validity runs before the synchronous `verify_pinned` snapshot read) — this is the property an attacker with a valid `ca_evil` leaf probes. Make it a *tested invariant*, not a comment.

### `async verify_pinned` inside a sync rustls callback (AC-A3 gotcha)

`verify_pinned` is `async fn` (`tofu.rs:90`) but rustls `ServerCertVerifier::verify_server_cert` is **synchronous**. Do NOT change `verify_pinned` (AC-A6 freeze). Bridge options: (a) the verifier holds a snapshot/`Arc<InMemoryTofuPinStore>` and does the pin comparison **synchronously** against the in-memory map (the async is only for the trait — the in-memory impl's body is non-blocking); (b) `tokio::runtime::Handle::current().block_in_place` + `block_on`. Prefer (a): read the pinned fingerprint synchronously from the `Arc<InMemoryTofuPinStore>` you already hold and compare leaf SHA-256 — no runtime juggling, no signature change. Document the chosen bridge in the Dev Agent Record.

### Previous-story (8.5) intelligence — directly applicable

- **AC-T7 is the test 8.5 deferred twice.** 8.5 logged the exact defect: *"A2A handle_intake future dangling — `tokio::time::timeout` returns `PartitionTimeout` on expiry, but `handle_intake` may still be executing" (`adapter.rs:289-298`).* On loopback you can't fix it (you don't own the future); on TCP you DO own the per-connection task — **abort the JoinHandle on timeout**, assert `is_finished()`. No third deferral.
- **Loopback peer lookup keys on `HostId` string** (`lookup_peer`, `adapter.rs:132-139`; pre-paired `A2APeerConfig` keyed `host_a`/`host_b`). For TCP, the peer identity comes from the **pinned leaf fingerprint** observed on the wire, mapped to the configured `peer_pins` — keep the `HostId`↔fingerprint mapping explicit (Mira=`host_a`, Nash=`host_b`).
- **Mira→Nash send path (the consumer 8.6 must satisfy):** `spirits/mira/tests/a2a_pairing.rs:151` calls `route_outbound(&router, frame, &HostId("host_b"))`; advisory is `IntentClass::Readonly` → consent band `"readonly"` (`mira::ADVISORY_CONSENT_INTENT`, `spirits/mira/src/lib.rs:53`). The smoke arm (Task 9) drives this same path over real TCP.
- **Consent vocabulary gap is OUT OF SCOPE** (→ Story 8.7). The free-form `Vec<A2AIntent>` only matches the 3-band projection; do not "fix" it here.
- **8.5 deferred A2A review items that are pre-existing, NOT yours to fix unless TCP forces them:** duplicate-peer-id overwrite (`adapter.rs:97-102`), un-pinned-peer path untested (`tofu.rs:196-200`), `install_intake_sink` race (`adapter.rs:115-121`), boot_nonce restart-detection race (`adapter.rs:383-423`). If TCP changes their reachability, note it.

### Project structure & conventions

- New crates live under `crates/` (`crates/maos-a2a-core/`, `crates/maos-a2a-tcp/`); add to root `Cargo.toml` `members` (currently 39 entries → 41). `version.workspace = true`; workspace version `0.1.0-alpha` (`[workspace.package]`).
- `maos-a2a` deps already pin the crypto stack: `rustls = { version = "0.23", default-features = false, features = ["ring","std","tls12"] }`, `tokio-rustls = { version = "0.26", default-features = false, features = ["ring","tls12"] }`, `rcgen = "0.13"` (dev), `sha2`, `hex`, `tokio` (`net,rt-multi-thread,macros,sync,time,io-util`). **Match these exact versions** in `maos-a2a-tcp` (`crates/maos-a2a/Cargo.toml:19-26`). Add `tokio-util = { version = "0.7", features = ["codec"] }` (the workspace already uses `tokio-util 0.7` elsewhere — `maos-bin/Cargo.toml:44`).
- `#![forbid(unsafe_code)]` at every crate root (matches `maos-a2a/src/lib.rs`).
- Smoke arms + their CI jobs live in `crates/maos-bin/src/main.rs` and `.github/workflows/discipline.yml` (see `smoke-a2a-loopback-6-3`, `smoke-founder-loop-8-4` at `discipline.yml:1364`).

### Security boundaries (preserve exactly — these are 8.5/6.3 invariants)

1. TOFU pin verification happens **before** any application frame (AC-T3); on TCP that means **during** the TLS handshake via `TofuPinningVerifier`, not after.
2. Consent-envelope expiry is checked **before** Lamport advance — rejection does NOT advance the clock (`handle_intake`, `adapter.rs:438-464`).
3. boot_nonce mismatch → `invalidate_for_restart` → `CODE_SPIRIT_RESTART_DETECTED` (`adapter.rs:383-423`); peer blocked until re-pin consent (NFR-Rel-6).
4. No fallback to "first peer" on unknown `HostId`/fingerprint — reject.
5. JSON parse errors emit `CODE_PARSE_ERROR` via `try_from_bytes` — the TCP receiver MUST route through that helper, not bespoke parsing.
6. Kernel does **zero** auto-retry (AC-T12) — the ONLY retrier is `HandshakeRetryPolicy` on the transport side.

### Latency / NFR posture

- **No latency assertion in any AC** (the epic is explicit — AC-T1 "No latency assertion"). J4 (Mira-Nash colocation < 10ms P95, §13.1) is about *co-located* Observer, NOT cross-Host TCP; do not add a J4 gate here. Correctness and determinism (AC-T13) dominate.
- This realizes **FR23b** (cross-Host, operator-managed PKI, mTLS, rotation chaos, partition fail-safe) at **v1.5** and satisfies **NFR-Sec-11/12/13** on the wire (replay 0-success, TOFU 100% detect, 3-host rotation chaos). FR23b's 10-host/revocation-latency gates are v2.0+ — out of scope; 3-host (AC-T11) is the v1.5 floor (NFR-Sec-13).

### Testing standards

- Integration tests in `crates/maos-a2a-tcp/tests/`; hermetic per H1–H6. `rcgen` certs generated at setup, never committed (H1 guard via `git ls-files`).
- Pinned clock `T0` fed to rustls validity check AND any rotation offset (H2). Ephemeral `:0` + `local_addr` readback + `oneshot` readiness (H3/H4). `test_profile()` ≤250ms timeouts (H5). `kill_on_drop(true)` + JoinHandle drop-guards (H6).
- Stress: nextest `--retries 0 --test-threads=8` (and/or 50× loop) must be 100% green (AC-T13).

### Project Structure Notes

- **Alignment:** clean — the extraction (`maos-a2a` → `maos-a2a-core` + `maos-a2a-tcp`) resolves the existing `maos-a2a` 1500-ceiling overage in the same move, and keeps the wire crate's dep graph free of the loopback router (`maos-a2a-tcp → maos-a2a-core ← maos-a2a`). Mirrors the Epic-8 zero-kernel-KLOC pattern (8.1–8.5 all added crates under `spirits/`/`crates/` with `maos-kernel-core` byte-identical).
- **Variance flagged:** epic's "37 → 39" and the idealized `dispatch`/`A2ANack`/`TransportError` names corrected (see top-of-file §"THREE SPEC CORRECTIONS"). These are factual reconciliations, not scope changes.

### References

- [Source: _bmad-output/planning-artifacts/epics/epic-8-…miranash-v03-v15.md#Story-8.6 (lines 197-330)] — full AC set (AC-A1–A7, H1–H6, AC-T1–T13), red-phase order.
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-04.md] — 8.5→8.6 split rationale + deferral-marker file:lines.
- [Source: crates/maos-a2a/src/adapter.rs:37-70,81-165,255-332,334-480,144-146] — `A2APeerRouter` trait, `LoopbackA2ARouter`, `route_outbound`/`handle_intake`, intent projection.
- [Source: crates/maos-a2a/src/{tofu.rs:90,122,191; mtls.rs:13,63; transport/json_rpc.rs:27,131; transport/logical_clock.rs:11; identity.rs:11,46; config.rs:12; chaos/rotation.rs:63}] — reused substrate.
- [Source: crates/maos-domain/src/frame.rs:29] — `IacFrame.logical_clock` (Lamport field).
- [Source: crates/maos-a2a/Cargo.toml:19-26] — pinned rustls/tokio-rustls/rcgen versions to match.
- [Source: Cargo.toml (members)] — 39 current members; xtask/kloc.toml:67 `maos-a2a = 1500`.
- [Source: crates/maos-bin/src/main.rs] — daemon composition root + smoke-arm home; .github/workflows/discipline.yml:1364 smoke-job pattern.
- [Source: _bmad-output/implementation-artifacts/8-5-…corpus.md:43,95,452 + 368 File List] — predecessor lessons: AC-T7 dangling-future, consent gap→8.7, HostId keying.
- [Source: prd/non-functional-requirements.md] — NFR-Sec-11/12/13 (mTLS replay / TOFU mismatch / 3-host rotation chaos, v1.5); prd/functional-requirements.md FR23a/FR23b.
- [Source: architecture-maos-minimal-opus/{4-kernel-design.md §4.0.4/§4.2.2, 7-inter-agent-communication.md §7.2/§7.2.1}] — cross-Host bilateral A2A design, rotation-chaos procedure.

---

## Dev Agent Record

### Agent Model Used

**claude-opus-4-8** (Amelia / dev-story). Security-critical networking; consistent with the 8.4/8.5 recommendation for the highest-risk Epic-8 stories.

### Debug Log References

- `MAOS_ONE_SHOT=smoke-a2a-tcp-8-6 cargo run -p maos-bin` → exit 0; "Nash(host_b) ACKed the advisory over live mTLS (boot_nonce=7, lamport=1, TOFU pin verified) ✓" on a real ephemeral socket (e.g. `127.0.0.1:41055`).
- `cargo test -p maos-a2a-tcp` → 7 test binaries green (T1, h_guards×7, T3–T6 ×8, T7–T10 ×4, T11/T12 ×3); 10×/50× repeat loops 100% green (AC-T13 determinism).
- `cargo run -p xtask -- kloc-check` → a2a-family GREEN: `maos-a2a 201/1500`, `maos-a2a-core 2576/3000`, `maos-a2a-tcp 847/1500`.
- `cargo run -p xtask -- check-workspace-count` → PASSED (actual=41, declared=41).
- `cargo run -p xtask -- check-unsafe / check-fr47 / check-empty-kernel / check-service-boundary` → all PASSED.
- maos-kernel-core sha256 (all `.rs`) `08e9a86ad20e06666cae104f35200818faa5d247c49dd60ee7841d4fe50cbd70` UNCHANGED pre→post; `git diff --stat -- crates/maos-kernel-core/` empty.

### Completion Notes List

**Extraction (Task 1 — AC-A1/A6/A7).** `git mv`'d the full substrate (config, identity, consent, error, tofu, mtls, transport/{json_rpc,logical_clock}, chaos/*, corpus, adapter→router) into NEW `crates/maos-a2a-core/`. The `adapter.rs` validation logic became the shared `A2ARouterCore` engine (handle_intake + `prepare_outbound`/`interpret_response` split so BOTH transports reuse it byte-for-byte). `maos-a2a` retains ONLY `LoopbackA2ARouter` as a thin wrapper and `pub use`-re-exports every moved module + symbol (incl. preserving `maos_a2a::adapter::A2APeerRouter` via `pub use`) → mira/nash/maos-bin compile unchanged. kloc-check resolved `maos-a2a` 2550→201 (overage closed by extraction, no ceiling bump).

**Seam decision (Correction #2).** `A2ATransport: A2APeerRouter` supertrait (`route_outbound`/`handle_intake` bound to the real `adapter.rs:255/334` surface) + `fn local_addr() -> Option<SocketAddr>`. No adapter glue; `abi-diff` Added-only intent preserved. The discipline `abi-diff` gate targets `maos-spirit-abi` ONLY (untouched) → stays green.

**TCP transport (Tasks 2–4).** `TcpA2ATransport` = real `TcpListener`/`TcpStream` + `tokio_rustls` + `Framed<…, LengthDelimitedCodec>` (4-byte BE `u32`, 1 MiB cap). `TofuPinningVerifier` impls BOTH rustls verifier directions; **WebPKI-first then TOFU pin** in both `ca_roots` postures (LOCKED Option A): `Some(roots)` ⇒ chain-to-root; `None` ⇒ leaf-as-its-own-anchor validity/structure (NOT a `danger_accept_any` noop). **Sync bridge** (Dev Notes option a): added additive `InMemoryTofuPinStore::find_active_pin_by_fingerprint`/`verify_pinned_sync`/`get_pin_sync` to core — the async `verify_pinned` signature is UNCHANGED (AC-A6). H2 clock pinned via the verifier's `validation_time` (rustls's `now` param ignored when injected). `TcpA2AConfig` exact AC-A5 fields + PEM/PKCS#8 loaders + `peer_pins`→pin-store. AC-A5 daemon binding = `build_a2a_tcp_daemon_router()` in `maos-bin/src/main.rs` returning `Arc<TcpA2ATransport>` (impls the kernel `A2ARouter` port); kernel got NO new public fn.

**New error codes (additive ADD, AC-A6-safe):** `CODE_TIMEOUT (-32005)`, `CODE_FRAME_TOO_LARGE (-32006)` in core json_rpc.

**Tests (Tasks 5–8).** Hermetic harness in `tests/support/mod.rs` (rcgen time-relative certs from a single `T0`; ephemeral `:0` + readback; kill-on-drop teardown; `test_profile()` ≤250ms). AC-T1 happy path + H1–H6 guards + AC-T3/T4/T4b/T5/T6 security + AC-T7 (slow-loris MANDATORY — per-connection task ABORTS on timeout, gauge→0, no third deferral) + AC-T8/T9/T10 + AC-T11 real-socket 3-host rotation chaos (NOT the synthetic `RotationDrillReport` — grep-guarded) + AC-T12 falsifiable absence (dep-graph excludes kernel; kernel-core line-count == 19950).

**Decision (retry semantics):** a TOFU pin mismatch maps to a NON-retryable `A2AError` (valid cert, wrong identity — retry cannot fix it); only `BAD_CERTIFICATE`/`CERTIFICATE_EXPIRED` are retryable per `HandshakeRetryPolicy` (AC-T5).

**Implementation-necessity deps added beyond AC-A2's list (documented judgment calls):** `maos-a2a-tcp` adds `maos-spirit-abi` (HostId — not re-exported by maos-domain), `serde`/`serde_json` (config + wire JSON), `async-trait` (impl A2APeerRouter), `rustls-pemfile` (PEM loaders), `rustls-webpki` (the `None`-mode leaf-validity step), `futures-util` (Sink/Stream over Framed). `maos-bin` adds `rcgen` (runtime self-signed certs for the smoke arm).

**Pre-existing breaks fixed to unblock compile (NOT A2A-caused; confirmed at clean HEAD via stash):** `maos-bin/src/main.rs` had 4 pre-existing compile errors in the Story-8.5 smoke/bench arms — `insert_frame_event(...).map_err()?` (returns a must-use `LogBeforeDeliver`, not `Result`) ×2, `decide(&j1,&j4)` (8.5 added a 3rd `j6: Option` arg), and a `let make_frame` closure needing `let mut`. Fixed minimally (analogous to 7.3/8.3 pre-existing maos-bin fixes) so AC-A5/AC-T13 could land in `main.rs`.

**Pre-existing reds verified NEUTRAL (not introduced, not mine):** kloc-check AGGREGATE stays RED (maos-kernel-core/maos-bin/maos-domain/xtask/maos-bench decomposition debt — a2a-family is green); `maos-mcp` test `mcp_client_trait_test` needs `--features fixture_replay` to compile (zero maos-a2a deps → unrelated). The background `revocation poller: poll_once failed: …CRL file…No such file` log during the smoke is a pre-existing unrelated daemon warning.

### File List

**NEW crate — `crates/maos-a2a-core/` (extracted substrate):**
- `crates/maos-a2a-core/Cargo.toml` (new)
- `crates/maos-a2a-core/src/lib.rs` (new)
- `crates/maos-a2a-core/src/router.rs` (moved from `maos-a2a/src/adapter.rs`; refactored to `A2ARouterCore` + `A2ATransport`/`A2APeerRouter` traits + `pub map_a2a_error_to_iac_bus` + `set_peer_endpoint`)
- `crates/maos-a2a-core/src/tofu.rs` (moved; +`find_active_pin_by_fingerprint`/`verify_pinned_sync`/`get_pin_sync`)
- `crates/maos-a2a-core/src/transport/json_rpc.rs` (moved; +`CODE_TIMEOUT`/`CODE_FRAME_TOO_LARGE`)
- `crates/maos-a2a-core/src/{config,consent,corpus,error,identity,mtls}.rs` (moved verbatim)
- `crates/maos-a2a-core/src/transport/{mod,logical_clock}.rs` (moved verbatim)
- `crates/maos-a2a-core/src/chaos/{mod,churn,harness_3_host,metrics,report,rotation}.rs` (moved verbatim)

**NEW crate — `crates/maos-a2a-tcp/` (live wire):**
- `crates/maos-a2a-tcp/Cargo.toml` (new)
- `crates/maos-a2a-tcp/src/lib.rs` (new)
- `crates/maos-a2a-tcp/src/transport.rs` (new — `TcpA2ATransport`, accept loop, dial+retry, codec, teardown, domain-port impl)
- `crates/maos-a2a-tcp/src/verifier.rs` (new — `TofuPinningVerifier`, WebPKI-then-pin, both directions)
- `crates/maos-a2a-tcp/src/config.rs` (new — `TcpA2AConfig`, `PinnedFingerprint`, PEM loaders, `clone_key`)
- `crates/maos-a2a-tcp/src/error.rs` (new — `TcpTransportError` taxonomy)
- `crates/maos-a2a-tcp/tests/support/mod.rs` (new — H1–H6 harness)
- `crates/maos-a2a-tcp/tests/t1_live_roundtrip.rs` (new — AC-T1)
- `crates/maos-a2a-tcp/tests/h_guards.rs` (new — H1–H6 guards)
- `crates/maos-a2a-tcp/tests/t3_t6_security.rs` (new — AC-T3/T4/T4b/T5/T6)
- `crates/maos-a2a-tcp/tests/t7_t10_liveness.rs` (new — AC-T7/T8/T9/T10)
- `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs` (new — AC-T11/T12)

**MODIFIED — `maos-a2a` (now thin):**
- `crates/maos-a2a/Cargo.toml` (deps → maos-a2a-core; trimmed)
- `crates/maos-a2a/src/lib.rs` (re-exports moved surface)
- `crates/maos-a2a/src/adapter.rs` (rewritten — `LoopbackA2ARouter` wrapper over `A2ARouterCore`)
- `crates/maos-a2a/src/{config,consent,corpus,error,identity,mtls,tofu}.rs` + `transport/*` + `chaos/*` (deleted — moved to core)

**MODIFIED — composition root + workspace + CI:**
- `Cargo.toml` (members += maos-a2a-core, maos-a2a-tcp → 41)
- `Cargo.lock` (new crates)
- `xtask/kloc.toml` (+`maos-a2a-core = 3000`, `maos-a2a-tcp = 1500`)
- `crates/maos-bin/Cargo.toml` (+maos-a2a-core, +maos-a2a-tcp, +rcgen)
- `crates/maos-bin/src/main.rs` (+`build_a2a_tcp_daemon_router` [AC-A5], +`smoke_a2a_tcp_8_6` + dispatch + known-modes; 4 pre-existing 8.5-arm compile fixes)
- `.github/workflows/discipline.yml` (+`a2a-tcp-tests-8-6`, +`smoke-a2a-tcp-8-6` jobs; both added to the aggregation `needs`)
- `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` (workspace count 41; A2A 3-crate layering + dependency arrows)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (8.6 → review)

### Change Log

| Date | Change |
|---|---|
| 2026-06-05 | Extracted `maos-a2a-core` (transport-agnostic ADR-012 substrate) from `maos-a2a`; resolved the 1500 kloc overage by extraction (2550→201). |
| 2026-06-05 | Shipped NEW `maos-a2a-tcp`: live cross-Host `TcpA2ATransport` (real TCP/mTLS, `TofuPinningVerifier` WebPKI-then-pin, 1 MiB length-delimited JSON-RPC framing, handshake retry, bounded intake/partition timeouts). |
| 2026-06-05 | Full AC-T1–T13 + H1–H6 integration suite (hermetic, 50× deterministic); `smoke-a2a-tcp-8-6` arm + 2 discipline CI jobs; workspace 39→41; kernel-core byte-identical; arch doc reconciled. |
| 2026-06-05 | Fixed 4 pre-existing (clean-HEAD-confirmed) maos-bin compile breaks in the 8.5 smoke/bench arms to unblock the AC-A5 binding + AC-T13 smoke landing in `main.rs`. |

---

## Review Findings

> **Code review 2026-06-05 (adversarial, 3-layer: Blind Hunter + Edge Case Hunter + Acceptance Auditor).**
> **SCOPE — security-critical-first PARTIAL pass.** Reviewed only the mTLS/TOFU/codec source surface: `maos-a2a-tcp/src/{transport,verifier,config,error,lib}.rs` + `maos-a2a-core/src/{router,tofu}.rs`. **Deferred to follow-up review runs:** (B) `maos-a2a-tcp/tests/*` (H1–H6 + AC-T oracles); (C) composition root + CI (`maos-bin/src/main.rs`, `discipline.yml`, `kloc.toml`, `Cargo.toml`); (D) core re-export `lib.rs` / `json_rpc.rs` / thinned `maos-a2a/adapter.rs`. AC-T*/AC-A5-binding/AC-A7-CI evidence lives in those deferred files and was NOT assessed here.

### Decision-needed (RESOLVED 2026-06-05 — both → patch)

- [x] [Review][Decision→Patch] Dial-side TLS verifier is not peer-scoped — flat-allowlist pinning enables cross-peer confusion in the AC-T11 3-host mesh — `crates/maos-a2a-tcp/src/verifier.rs:155,197` + `crates/maos-a2a-core/src/tofu.rs:166`. `verify_webpki_then_pin` accepts the leaf if `find_active_pin_by_fingerprint(observed)` matches **any** peer's active pin (`Some(_peer)` — peer discarded), and `verify_server_cert` ignores `_server_name`. A peer-scoped sync variant **already exists and is unused** — `InMemoryTofuPinStore::verify_pinned_sync(peer, observed)` (`tofu.rs:183`). **RESOLUTION:** Patch — scope the dial-side `ServerCertVerifier` to the `route_outbound` target peer via the existing `verify_pinned_sync` (strengthens to the frozen contract; listen side stays flat-lookup TOFU). See Patch P1.
- [x] [Review][Decision→Patch] `PinnedFingerprint.boot_nonce` `#[serde(default)] = 0` is a restart-detection footgun (spurious DoS or silently disabled detection) — `crates/maos-a2a-tcp/src/config.rs` (`PinnedFingerprint`) + `crates/maos-a2a-core/src/router.rs:348`. Omitting `boot_nonce` pins stored `0`; first real non-zero-nonce frame fires `invalidate_for_restart` + `CODE_SPIRIT_RESTART_DETECTED` (self-DoS); both-`0` disables detection. **RESOLUTION:** Patch — make `boot_nonce` a required field (remove `#[serde(default)]`) so a missing nonce is a hard config-load error. See Patch P2.

### Patch — ALL APPLIED & VERIFIED 2026-06-05

- [x] [Review][Patch] **P1** — Scope dial-side `ServerCertVerifier` to the intended peer via `verify_pinned_sync(peer, observed)` instead of unscoped `find_active_pin_by_fingerprint` [crates/maos-a2a-tcp/src/verifier.rs; crates/maos-a2a-tcp/src/transport.rs] — DONE: added `expected_peer: Option<PeerId>` to `TofuPinningVerifier` (`Some(peer)` on dial → `verify_pinned_sync`; `None` on listen → flat first-contact lookup); `route_outbound` builds a per-peer `scoped_client_config(&peer_cfg.peer_id)` before the retry loop. Listen-side TOFU unchanged.
- [x] [Review][Patch] **P2** — Make `PinnedFingerprint.boot_nonce` required (removed `#[serde(default)]`); a missing nonce is now a hard deserialize error [crates/maos-a2a-tcp/src/config.rs] — DONE. All in-repo constructions are explicit struct literals (smoke uses `boot_nonce=7`); no serde path omitted it, so zero breakage.
- [x] [Review][Patch] **P3** — `clone_key` unknown-`PrivateKeyDer`-variant arm now `panic!`s (fail-closed) instead of substituting an EMPTY key [crates/maos-a2a-tcp/src/config.rs] — DONE.
- [x] [Review][Patch] **P4** — Accept loop backs off 50ms on accept error so a persistent EMFILE/ENFILE error cannot busy-spin a core; transient errors still recover [crates/maos-a2a-tcp/src/transport.rs] — DONE.

**Verification:** `cargo build -p maos-a2a-core -p maos-a2a-tcp` green; `cargo test -p maos-a2a-tcp` = **23/23 green** (incl. the AC-T11 3-host mesh — legitimate per-peer reachability preserved under P1); `cargo build -p maos-bin` green; `MAOS_ONE_SHOT=smoke-a2a-tcp-8-6` → **exit 0**, "Nash(host_b) ACKed … over live mTLS (boot_nonce=7, lamport=1, TOFU pin verified) ✓" on a real ephemeral socket. Files modified by review patches: `crates/maos-a2a-tcp/src/{verifier,transport,config}.rs` + `crates/maos-a2a-tcp/tests/support/mod.rs` (build_client_config call-site arg).

### Resolved beyond review scope (user-directed 2026-06-05)

- [x] [Review][Patch] **F2** — Consent-expiry now uses a REAL (pinnable) clock, not a per-call counter — FIXED [crates/maos-a2a-core/src/router.rs]. The old `monotonic_now_ns` counter (values 1,2,3,…) never exceeded a real wall-clock `valid_until_ns` (~1.7e18 ns), so a genuinely-expired consent envelope was admitted (fail-OPEN); it only ever rejected the degenerate `valid_until_ns = 0` case. **Fix:** replaced with `wall_now_ns()` (SystemTime epoch-ns; fails CLOSED to `u64::MAX` if the clock is unreadable) plus an ADDITIVE `consent_now_ns: Option<u64>` field on `A2ARouterCore` (`None` ⇒ real clock; pinnable via the new `with_pinned_consent_clock(t0)` builder for deterministic tests). `A2ARouterCore::new` signature UNCHANGED → the two callers (loopback `adapter.rs`, tcp `transport.rs`) are untouched. Added 2 regression tests (`intake_rejects_real_timestamp_expired_consent_f2` → expired real timestamp REJECTED; `intake_admits_unexpired_real_timestamp_consent_f2` → future timestamp ACKed). The pre-existing `Some(0)` guard (`maos-a2a` P2) still passes (real `now > 0`). **Verification:** `maos-a2a-core` 77 tests green (incl. both F2 tests), `maos-a2a` P2 guard green, `maos-a2a-tcp` 23 green, `smoke-a2a-tcp-8-6` exit 0.
  > ⚠️ **FLAG FOR WINSTON (AC-A6 churn):** This was a *user-directed* fix that touches the moved-verbatim `A2ARouterCore` consent logic, which epic AC-A6 declares churn-free. The change is **additive** (new field defaults `None`, new builder, no constructor-signature change, no `verify_pinned`/intent surface touched; `abi-diff` stays Added-only) and the consent vocabulary is NOT widened (still Story 8.7's job), so it does not re-open the protocol surface — but it IS a logic change inside the AC-A6-frozen core and should be acknowledged in the Epic 8 retro. The dormant-but-real fail-open justified closing it now rather than waiting for 8.7/12-0.

### Deferred
- [x] [Review][Defer] End-to-end peer binding gap: receiver `handle_intake` derives peer from attacker-supplied `frame.from.host_id` (fallback `"loopback"`) and "TOFU verify" compares config-vs-config, not the wire cert [crates/maos-a2a-core/src/router.rs:312,330] — deferred, pre-existing (moved-verbatim loopback `handle_intake`). The real cert binding on TCP is the TLS-layer `TofuPinningVerifier`; tightening this is the receiver half of the Decision-needed peer-scoping item. Pre-existing pin-store races (boot_nonce restart-detection TOCTOU, duplicate-peer overwrite) are already logged as deferred in 8.5.
- [x] [Review][Defer] Brittle upstream-string classification: `map_webpki_error` (webpki `Debug` substring) and `is_frame_too_large` (io::Error `Display` substring) can silently regress AC-T5/AC-T8 if dep wording drifts [crates/maos-a2a-tcp/src/verifier.rs:179; crates/maos-a2a-tcp/src/transport.rs (is_frame_too_large)] — deferred, low robustness. Deps are version-pinned and the sentinel-tagged `classify_handshake` path is robust; recommend typed-error matching (codec `LengthDelimitedCodecError`, typed webpki errors) in a hardening follow-up.
- [x] [Review][Defer] Intake timeout wraps only the `framed.next()` READ — a slow `handle_intake` (or a best-effort timeout-NACK write to a non-reading slow-loris peer) is unbounded by that timeout [crates/maos-a2a-tcp/src/transport.rs (serve_connection)] — deferred, low/narrow. `handle_intake` is in-memory/non-blocking today so it completes fast; the per-connection task is still abortable via the H6 drop-guard. AC-T7's core byte-starvation fix is sound. Recommend wrapping `handle_intake` + the NACK write in the same timeout budget if intake ever gains I/O.

**Dismissed as noise (6):** (1) `client_auth_mandatory` not overridden — false positive, rustls 0.23 default is `true` (mandatory), behavior correct (hardening: an explicit override + no-client-cert-rejected test would document it); (2) `last_intake` mutex poison swallowed by `.lock().ok()` — test-observation plumbing only, not a production/security path; (3) `conns.retain` prunes finished handles only on next accept — bounded, acceptable under churn-then-idle; (4) timeout/oversize NACK `id` hardcoded `0` — cosmetic, no request pipelining in protocol; (5) `next_id`/`attempt` u64 arithmetic wrap — unreachable; (6) zero-length frame increments `intake_entered` — test-oracle nuance, not a vulnerability.
