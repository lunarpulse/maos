//! A2A router core engine — the transport-agnostic validation + framing logic
//! shared by every `A2ATransport` impl: the in-process `LoopbackA2ARouter`
//! (`maos-a2a`) and the live `TcpA2ATransport` (`maos-a2a-tcp`).
//!
//! Story 8.6 extraction: this module was `maos-a2a::adapter`. The validation
//! logic (`handle_intake`, the outbound allowlist/TOFU/clock prep, response
//! interpretation, and the `IacBusError` mapping) moved here UNCHANGED so both
//! transports reuse it byte-for-byte (epic AC-A6 — no protocol-surface churn).
//! The `A2APeerRouter` trait moved here too; the NEW `A2ATransport` seam
//! (epic AC-A1, Correction #2) is defined here, bound to the real
//! `route_outbound`/`handle_intake` surface via the `A2APeerRouter` supertrait.
//!
//! `LoopbackA2ARouter` itself stays in `maos-a2a` (the only `impl A2ATransport`
//! that crate retains); it is now a thin wrapper around [`A2ARouterCore`].

use crate::config::A2APeerConfig;
use crate::consent::{AllowlistDirection, ConsentAllowlists, EIntentDenied};
use crate::error::{A2AError, IntentDirection};
use crate::identity::PeerId;
use crate::tofu::TofuPinStore;
use crate::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, AckBody, CODE_CONSENT_EXPIRED,
    CODE_CONSENT_GRANTER_MISMATCH, CODE_INTENT_DENIED, CODE_INTERNAL,
    CODE_PEER_IDENTITY_MISMATCH, CODE_PIN_MISMATCH_NOT_PINNED,
    CODE_SPIRIT_RESTART_DETECTED,
};
use crate::transport::logical_clock::LamportClock;
use async_trait::async_trait;
use dashmap::DashMap;
use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i8::{A2AIntent, MAX_CANONICAL_INTENT_LEN};
use maos_spirit_abi::identity::HostId;
use std::net::SocketAddr;
use std::sync::Arc;

/// Real wall-clock "now" in nanoseconds since the Unix epoch — the reference
/// the consent-envelope `valid_until_ns` is compared against.
///
/// Story 8.6 review (F2): this REPLACES the previous per-call atomic *counter*
/// (`monotonic_now_ns`, starting at `1`). That counter was never a clock — a
/// real wall-clock `valid_until_ns` (≈1.7e18 ns) was never exceeded by a small
/// call count, so a genuinely-expired consent envelope was admitted (fail-OPEN).
/// The counter only happened to reject the degenerate `valid_until_ns = 0` case
/// (Story 6.3 §A1 P2). A real clock preserves that — `now > 0` for any post-1970
/// instant — while ALSO rejecting real-timestamp expiries.
///
/// Fails CLOSED: if the system clock is unreadable (pre-epoch), returns
/// `u64::MAX` so every bounded envelope is treated as expired rather than
/// silently admitted.
///
/// Determinism: tests that need a pinned "now" inject one via
/// [`A2ARouterCore::with_pinned_consent_clock`]; this free fn is the production
/// default (used only when no pin is set).
fn wall_now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(u64::MAX)
}

/// Internal A2A peer router trait — the loopback/cross-host routing surface.
/// Distinct from `maos_domain::ports::a2a::A2ARouter` (the hexagonal port).
/// The `A2APeerRouter` carries the full two-direction routing API including
/// `handle_intake` which the domain port does not expose.
#[async_trait]
pub trait A2APeerRouter: Send + Sync {
    /// Outbound: deliver this frame to the named peer Host via the configured
    /// transport.
    ///
    /// Validation order per architecture §7.3.2 + ADR-012:
    ///   1. ADR-012 send_allowlist check (peer.send_allowlist contains frame.intent?)
    ///   2. TOFU pin verify (cross-Host) or mTLS-only (loopback)
    ///   3. JSON-RPC frame serialization + send + await ACK/NACK
    async fn route_outbound(
        &self,
        frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), A2AError>;

    /// Intake: a peer just sent us a frame.
    ///
    /// Validation order:
    ///   1. TOFU pin verify against connection's cert fingerprint
    ///   2. ADR-012 accept_allowlist check
    ///   3. Consent envelope expiry check
    ///   4. Logical-clock advance
    ///   5. Hand to `IacBusAdapter::deliver_typed`
    async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse;
}

/// Story 8.6 (epic AC-A1, Correction #2) — the transport seam. Mirrors the
/// frozen `A2APeerRouter` surface (`route_outbound`/`handle_intake`, bound to
/// `router.rs` `prepare_outbound`/`interpret_response`/`handle_intake`) via a
/// supertrait so NO adapter glue is needed, and adds a `local_addr` readiness
/// hook. In-process transports (loopback) return `None`; the live
/// `TcpA2ATransport` returns its bound `SocketAddr`.
#[async_trait]
pub trait A2ATransport: A2APeerRouter {
    /// The bound listen address once the transport is live. `None` for
    /// in-process transports (loopback) that never bind a socket.
    fn local_addr(&self) -> Option<SocketAddr> {
        None
    }
}

/// A2A router core engine — `127.0.0.1`-bound endpoints with self-signed mTLS
/// + TOFU pinning at the loopback profile; operator-managed PKI at cross-Host.
/// Holds the transport-agnostic state (`peers`, `tofu`, `clock`) and the
/// validation logic both transports reuse.
///
/// `peers` maps `HostId` → `A2APeerConfig`.
/// `tofu` is the pin store (in-memory at v0.5; persistence-backed in follow-up).
/// `clock` is the per-router Lamport clock.
pub struct A2ARouterCore {
    peers: Arc<DashMap<String, A2APeerConfig>>,
    tofu: Arc<dyn TofuPinStore>,
    clock: Arc<LamportClock>,
    /// Optional intake sink for tests — when set, accepted frames are
    /// pushed here so test code can observe them.
    intake_sink: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<IacFrame>>>>,
    /// Atomic counter for outbound request ids.
    next_id: Arc<std::sync::atomic::AtomicU64>,
    /// Pinned consent-expiry clock (ns since Unix epoch). `None` ⇒ real wall
    /// clock ([`wall_now_ns`]); `Some(t0)` pins "now" for deterministic
    /// consent-expiry tests (Story 8.6 review F2 fix — replaces the old call
    /// counter). Additive: `new()` defaults to `None`, so no caller changes.
    consent_now_ns: Option<u64>,
    /// Deduplication set for AC5 unreachable-entry warnings. Each
    /// `(peer_id, entry, direction)` tuple is warned once per router lifetime
    /// to prevent DoS amplification on repeated denials (review finding).
    warned_entries: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl A2ARouterCore {
    /// Story 8.9 / AC6.2 (G5b) — fallible constructor that HARD-FAILS on a
    /// duplicate `peer_id` rather than silently letting "last wins" overwrite an
    /// earlier peer's pin/allowlist binding (the prior `new` only `eprintln!`d).
    /// `new` delegates here and panics on the error so existing infallible
    /// callers keep compiling; cross-Host `TcpA2ATransport::bind` calls `try_new`
    /// directly and surfaces `A2AError::ConfigInvalid` to the operator.
    pub fn try_new(
        peer_configs: Vec<A2APeerConfig>,
        tofu: Arc<dyn TofuPinStore>,
    ) -> Result<Self, A2AError> {
        let peers = Arc::new(DashMap::new());
        for cfg in peer_configs {
            let key = cfg.peer_id.as_str().to_string();
            if peers.contains_key(&key) {
                return Err(A2AError::ConfigInvalid(format!(
                    "duplicate peer config for peer_id {key}"
                )));
            }
            peers.insert(key, cfg);
        }
        Ok(Self {
            peers,
            tofu,
            clock: Arc::new(LamportClock::new()),
            intake_sink: Arc::new(tokio::sync::Mutex::new(None)),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            consent_now_ns: None,
            warned_entries: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        })
    }

    pub fn new(peer_configs: Vec<A2APeerConfig>, tofu: Arc<dyn TofuPinStore>) -> Self {
        Self::try_new(peer_configs, tofu)
            .expect("A2ARouterCore::new: duplicate peer_id — use try_new")
    }

    /// Pin the consent-expiry "now" clock to a fixed nanosecond value for
    /// deterministic tests (Story 8.6 review F2). Production leaves this unset
    /// and uses the real wall clock ([`wall_now_ns`]).
    pub fn with_pinned_consent_clock(mut self, now_ns: u64) -> Self {
        self.consent_now_ns = Some(now_ns);
        self
    }

    /// The "now" used for consent-envelope expiry: the pinned test clock if set,
    /// else the real wall clock.
    fn consent_now_ns(&self) -> u64 {
        self.consent_now_ns.unwrap_or_else(wall_now_ns)
    }

    /// Install an intake sink — test-only hook. Accepted frames are forwarded
    /// to the sink AFTER all validation passes.
    pub async fn install_intake_sink(
        &self,
        sink: tokio::sync::mpsc::UnboundedSender<IacFrame>,
    ) {
        let mut guard = self.intake_sink.lock().await;
        *guard = Some(sink);
    }

    pub fn clock(&self) -> Arc<LamportClock> {
        Arc::clone(&self.clock)
    }

    /// The pin store backing this engine — TCP transports read it from the
    /// synchronous rustls verifier callback (Story 8.6 AC-A3 sync bridge).
    pub fn tofu(&self) -> Arc<dyn TofuPinStore> {
        Arc::clone(&self.tofu)
    }

    fn alloc_id(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Story 8.6 — patch a peer's dial endpoint after construction. Needed for
    /// ephemeral-port mesh topologies (AC-T11): all listeners bind `:0` first,
    /// then each peer's real readback `SocketAddr` is wired in (H3/H4). Additive;
    /// does not change any frozen signature (AC-A6).
    pub fn set_peer_endpoint(&self, host_id: &HostId, endpoint: impl Into<String>) {
        if let Some(mut entry) = self.peers.get_mut(host_id.as_str()) {
            entry.endpoint = endpoint.into();
        }
    }

    pub fn lookup_peer(&self, host_id: &HostId) -> Result<A2APeerConfig, A2AError> {
        self.peers
            .get(host_id.as_str())
            .map(|r| r.value().clone())
            .ok_or_else(|| {
                A2AError::ConfigInvalid(format!("no peer config for host_id {}", host_id.as_str()))
            })
    }

    /// Project the frame's `IntentClass` to a stable A2A consent intent string
    /// for ADR-012 allowlist matching. Uses `IntentClass::a2a_consent_intent_str()`
    /// — the canonical (not Debug-derived) lowercase projection.
    ///
    /// Story 8.7 — this is now the **band-fallback primitive** that
    /// [`Self::consent_match_key`] calls when a frame declares no fine-grained
    /// per-frame intent. It stays `pub` (part of `maos-a2a-core`'s public surface
    /// since the 8.6 extraction); renaming it would register as an abi-diff
    /// Removed, so the fine-grained logic was added alongside it rather than
    /// replacing it.
    pub fn frame_intent_str(frame: &IacFrame) -> String {
        frame.intent.a2a_consent_intent_str().to_string()
    }

    /// Story 8.7 / AC1 — the single ADR-012 consent-match key for a frame.
    ///
    /// Precedence (this is what makes ADR-012 "typed-*intent* consent" rather
    /// than "typed-*class* consent"):
    ///
    /// 1. **Fine-grained when present** — if the frame carries an explicit
    ///    per-frame intent (`consent_envelope.intent_class`, the
    ///    [`A2AIntent`](maos_domain::invariants::i8::A2AIntent) the sender fills
    ///    and the receiver verifies), that literal string is the match key. So
    ///    an operator's `diagnosis-handoff:read-only-evidence` allowlist entry
    ///    matches the actual frame intent instead of silently collapsing to a
    ///    coarse band — and `code-mutation-directive` is rejected even though
    ///    both project to the `readonly`/`standard` band (ADR-012's worked
    ///    confused-deputy example).
    /// 2. **Band fallback otherwise** — same-Host frames, or legacy/unmigrated
    ///    cross-Host frames that declare no fine-grained intent, fall back to the
    ///    3-band `IntentClass` projection via [`Self::frame_intent_str`],
    ///    preserving the pre-8.7 `readonly`/`standard`/`highprivilege` behaviour
    ///    byte-for-byte (zero-regression — AC7).
    ///
    /// Used by BOTH `send_admits`/`accept_admits` AND the two
    /// `EIntentDenied.intent` construction sites, so the key actually tested and
    /// the key reported in a denial can never diverge.
    ///
    /// [Source: docs/adr/ADR-012-typed-intent-a2a-consent.md] — consent is
    /// `(peer-identity, intent-class)` with an open intent vocabulary.
    fn consent_match_key(frame: &IacFrame) -> String {
        frame
            .consent_envelope
            .as_ref()
            .and_then(|e| e.intent_class.as_ref())
            // Story 8.9 / AC5 (G9) — unify the length bound with the canonical
            // intent bound (`A2AIntent::is_canonical` enforces the same 128) so an
            // oversized intent_class falls back to the 3-band projection
            // consistently; replaces the ad-hoc `<= 1024`.
            .filter(|i| i.as_str().len() <= MAX_CANONICAL_INTENT_LEN)
            .map(|i| i.as_str().to_string())
            .unwrap_or_else(|| Self::frame_intent_str(frame))
    }

    /// Story 8.7 / AC5 — make the "silent never-match" failure mode loud.
    ///
    /// Emits a `tracing::warn!` once per unique `(peer, entry, direction)` for
    /// every allowlist entry that is neither a canonical fine-grained intent nor
    /// one of the 3 band tokens. The raw intent string is NOT emitted as a
    /// structured field to avoid leaking vocabulary to WARN-level log collectors
    /// (review finding); it appears only in the human-readable message.
    ///
    /// Deduplication prevents DoS amplification: a flood of denied frames does
    /// not produce unbounded log volume (review finding).
    fn warn_unreachable_entries(
        &self,
        allowlist: &[A2AIntent],
        direction: AllowlistDirection,
        peer_id: &str,
    ) {
        let mut warned = self.warned_entries.lock().unwrap();
        for entry in allowlist {
            if !entry.is_canonical() {
                let dedup_key = format!("{peer_id}:{direction:?}:{}", entry.as_str());
                if warned.insert(dedup_key) {
                    tracing::warn!(
                        target: "maos_a2a_core::router",
                        direction = ?direction,
                        peer = peer_id,
                        "ADR-012 consent allowlist holds a non-canonical intent that may not \
                         match canonical frame intents (matching is case-insensitive, but the \
                         canonical grammar is `namespace:verb` lowercase such as \
                         `diagnosis-handoff:read-only-evidence`, or a band token); \
                         entry={}",
                        entry.as_str()
                    );
                }
            }
        }
    }

    /// Send-allowlist enforcement. The peer's `send_allowlist` enumerates
    /// `A2AIntent` strings the operator wills THIS Host to SEND to that peer.
    fn send_admits(&self, allow: &ConsentAllowlists, frame: &IacFrame, peer_id: &str) -> bool {
        let s = Self::consent_match_key(frame);
        let admitted = allow
            .send_allowlist
            .iter()
            .any(|i| i.as_str().eq_ignore_ascii_case(&s));
        if !admitted {
            self.warn_unreachable_entries(&allow.send_allowlist, AllowlistDirection::Send, peer_id);
        }
        admitted
    }

    fn accept_admits(&self, allow: &ConsentAllowlists, frame: &IacFrame, peer_id: &str) -> bool {
        let s = Self::consent_match_key(frame);
        let admitted = allow
            .accept_allowlist
            .iter()
            .any(|i| i.as_str().eq_ignore_ascii_case(&s));
        if !admitted {
            self.warn_unreachable_entries(&allow.accept_allowlist, AllowlistDirection::Accept, peer_id);
        }
        admitted
    }

    /// Shared outbound preparation — steps (1)–(4) of `route_outbound`,
    /// transport-independent. Performs the ADR-012 send-allowlist check, the
    /// TOFU pin verify, the Lamport send-tick stamp, and builds the wire
    /// `A2AJsonRpcRequest`. The transport then either loops the request back
    /// (loopback) or serializes it onto the socket (TCP) before calling
    /// [`interpret_response`](Self::interpret_response).
    ///
    /// `boot_nonce` is the sender's Spirit boot-nonce (Story 6.3 §A1 P6 /
    /// Correction #3): loopback v0.5-α callers pass `0` (unspecified);
    /// cross-Host TCP callers pass the live nonce so the receiver can detect
    /// Spirit restarts (NFR-Rel-6).
    pub async fn prepare_outbound(
        &self,
        mut frame: IacFrame,
        peer: &HostId,
        boot_nonce: u64,
    ) -> Result<(A2AJsonRpcRequest, A2APeerConfig, [u8; 16]), A2AError> {
        let peer_cfg = self.lookup_peer(peer)?;

        // (1) ADR-012 send-allowlist check (defense-in-depth — sender side).
        if !self.send_admits(&peer_cfg.allowlists, &frame, peer_cfg.peer_id.as_str()) {
            return Err(A2AError::IntentDenied {
                direction: IntentDirection::Send,
                inner: EIntentDenied {
                    peer: peer.as_str().to_string(),
                    // Story 8.7 — report the SAME key `send_admits` tested
                    // (fine-grained when present), never a band token.
                    intent: Self::consent_match_key(&frame),
                    direction: AllowlistDirection::Send,
                },
            });
        }

        // (2) TOFU pin verify — ensure the peer's cert fingerprint matches the
        //     pinned record before writing to wire.
        self.tofu
            .verify_pinned(&peer_cfg.peer_id, &peer_cfg.cert_fingerprint)
            .await
            .map_err(A2AError::PinMismatch)?;

        // (3) Lamport send tick — stamp the frame.
        frame.logical_clock = self.clock.send_tick();

        // (3.5) Story 8.9 / AC3 (G10) — stamp a bounded consent expiry on a
        // present envelope that carries none. Before 8.9 `prepare_outbound`
        // NEVER set `valid_until_ns` and `with_fine_grained_intent` builds it as
        // `None`, so expiry was dead code on every real (non-hand-built) frame.
        //
        // TRANSITIONAL (Decision §D1): the AUTHORITATIVE expiry source is the
        // consent grant itself populating `valid_until_ns` — so an envelope that
        // ALREADY carries an explicit `Some(_)` is left untouched (the transport
        // must never override the granter). The cross-Host
        // fail-closed-on-absent-expiry end-state is owned by Story 8.8; 8.9 only
        // makes expiry LIVE. `saturating_*` keeps a misconfigured-huge TTL from
        // wrapping to a past instant (defense-in-depth; `validate()` caps it).
        if let Some(env) = frame.consent_envelope.as_mut() {
            if env.valid_until_ns.is_none() {
                let ttl_ns = peer_cfg.consent_ttl_secs.saturating_mul(1_000_000_000);
                env.valid_until_ns = Some(self.consent_now_ns().saturating_add(ttl_ns));
            }
        }

        // (4) Build JSON-RPC request.
        let id = self.alloc_id();
        let frame_id = frame.frame_id;
        let request =
            A2AJsonRpcRequest::new("iac.deliver", frame, id).with_boot_nonce(boot_nonce);
        Ok((request, peer_cfg, frame_id))
    }

    /// Interpret a peer's JSON-RPC response into the typed outbound result.
    /// Transport-independent — the loopback shortcut and the live TCP read both
    /// funnel their `A2AJsonRpcResponse` through here.
    pub fn interpret_response(
        &self,
        peer: &HostId,
        response: A2AJsonRpcResponse,
    ) -> Result<(), A2AError> {
        match response {
            A2AJsonRpcResponse::Ack(_) => Ok(()),
            A2AJsonRpcResponse::Nack(n) => match n.error.code {
                CODE_INTENT_DENIED => Err(A2AError::IntentDeniedAtPeer {
                    peer: peer.as_str().to_string(),
                    message: n.error.message,
                }),
                CODE_PIN_MISMATCH_NOT_PINNED => Err(A2AError::PinInvalidated {
                    peer: peer.as_str().to_string(),
                    awaiting_repin: true,
                }),
                CODE_CONSENT_EXPIRED => {
                    // Extract timestamps from NACK data if present.
                    let (expired_at_ns, now_ns) = n
                        .error
                        .data
                        .as_ref()
                        .and_then(|d| {
                            d.get("expired_at_ns")
                                .and_then(|v| v.as_u64())
                                .zip(d.get("now_ns").and_then(|v| v.as_u64()))
                        })
                        .unwrap_or((0, 0));
                    Err(A2AError::ConsentExpired {
                        expired_at_ns,
                        now_ns,
                    })
                }
                CODE_PEER_IDENTITY_MISMATCH => {
                    let (expected, asserted) = n
                        .error
                        .data
                        .as_ref()
                        .map(|d| {
                            (
                                d.get("expected").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                d.get("asserted").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            )
                        })
                        .unwrap_or_default();
                    Err(A2AError::PeerIdentityMismatch { expected, asserted })
                }
                CODE_CONSENT_GRANTER_MISMATCH => {
                    let (granter, frame_from) = n
                        .error
                        .data
                        .as_ref()
                        .map(|d| {
                            (
                                d.get("granter").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                d.get("frame_from").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            )
                        })
                        .unwrap_or_default();
                    Err(A2AError::ConsentGranterMismatch { granter, frame_from })
                }
                _ => Err(A2AError::TransportFailed(n.error.message)),
            },
        }
    }

    /// Receiver-side intake — TOFU verify, restart detection, accept-allowlist,
    /// consent-expiry, Lamport advance, then the (test) intake sink. Returns the
    /// JSON-RPC ACK/NACK. Transport-independent: the loopback router calls this
    /// directly; the TCP transport calls it from its per-connection read loop
    /// after `A2AJsonRpcRequest::try_from_bytes` decodes the framed bytes.
    pub async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse {
        // Validate framing
        if let Err(err) = request.validate() {
            return A2AJsonRpcResponse::Nack(crate::transport::json_rpc::NackResponse {
                jsonrpc: crate::transport::json_rpc::JSONRPC_VERSION.to_string(),
                error: err,
                id: request.id,
            });
        }

        let frame = &request.params;

        // Identify the peer by HostId from frame.from.host_id.
        let peer_host = match &frame.from.host_id {
            Some(h) => h.clone(),
            None => HostId("loopback".to_string()),
        };

        // Look up peer config — NO fallback to first peer on failure.
        let peer_cfg = match self.lookup_peer(&peer_host) {
            Ok(c) => c,
            Err(e) => {
                return A2AJsonRpcResponse::nack(
                    request.id,
                    CODE_INTERNAL,
                    format!("unknown peer {}: {e}", peer_host.as_str()),
                );
            }
        };

        // (1) TOFU pin verify — ensure the cert fingerprint matches the pinned record.
        if let Err(e) = self
            .tofu
            .verify_pinned(&peer_cfg.peer_id, &peer_cfg.cert_fingerprint)
            .await
        {
            return A2AJsonRpcResponse::nack(
                request.id,
                CODE_PIN_MISMATCH_NOT_PINNED,
                format!("TOFU pin verify failed: {e}"),
            );
        }

        // (1.5) Story 6.3 §A1 P6 — Spirit-restart detection via wire-carried
        // `boot_nonce` (NFR-Rel-6 detection floor). `boot_nonce == 0` is the
        // v0.5-α "unspecified" sentinel: backward-compat with loopback
        // callers that pre-date the wire field. Cross-Host v0.7+ callers
        // MUST populate the field; receivers compare against the stored
        // `TofuPin.boot_nonce`. Mismatch → `invalidate_for_restart` + NACK.
        if request.boot_nonce != 0 {
            // Story 8.9 / AC6.3 (G5b) — atomic compare-and-invalidate: the prior
            // `get_pin` (read) → `invalidate_for_restart` (write) split was a
            // check-then-act TOCTOU. `invalidate_if_boot_nonce_differs` performs
            // the boot-nonce read and the invalidation under ONE pin-entry lock,
            // returning the prior nonce iff it rolled.
            match self
                .tofu
                .invalidate_if_boot_nonce_differs(&peer_cfg.peer_id, request.boot_nonce)
                .await
            {
                Ok(Some(prior)) => {
                    let observed = request.boot_nonce;
                    let data = serde_json::json!({
                        "prior_boot_nonce": prior,
                        "observed_boot_nonce": observed,
                    });
                    let mut resp = A2AJsonRpcResponse::nack(
                        request.id,
                        CODE_SPIRIT_RESTART_DETECTED,
                        format!(
                            "Spirit restart detected on peer {}: prior_boot_nonce={prior} observed_boot_nonce={observed}",
                            peer_cfg.peer_id.as_str()
                        ),
                    );
                    if let A2AJsonRpcResponse::Nack(ref mut n) = resp {
                        n.error.data = Some(data);
                    }
                    return resp;
                }
                Ok(None) => { /* nonce matches (or no pin) — continue */ }
                Err(e) => {
                    return A2AJsonRpcResponse::nack(
                        request.id,
                        CODE_INTERNAL,
                        format!("invalidate_for_restart failed: {e}"),
                    );
                }
            }
        }

        // (2) Story 8.9 / AC4 (G2) — consent block, evaluated BEFORE the
        // accept-allowlist so an expired or wrong-granter consent is rejected
        // regardless of whether the intent is allowlisted.
        //
        // Ordering WITHIN the block: granter binding FIRST, then expiry.
        // Per spec AC4.2. A stolen-but-unexpired envelope (the real G1 replay
        // attack) fails closed at the granter gate. An expired envelope with a
        // valid granter fails at expiry. Both are rejected before the allowlist.
        if let Some(envelope) = &frame.consent_envelope {
            // (2a) Story 8.9 / AC2 (G1) — granter binding: the envelope's granter
            // MUST be the frame's own `from` (compare spirit_id AND host_id).
            // Closes stolen-envelope replay (granted by X, replayed from Y).
            if envelope.granter.spirit_id != frame.from.spirit_id
                || envelope.granter.host_id != frame.from.host_id
            {
                let granter = format!(
                    "{}@{}",
                    envelope.granter.spirit_id.as_str(),
                    envelope.granter.host_id.as_ref().map(|h| h.as_str()).unwrap_or("<none>")
                );
                let frame_from = format!(
                    "{}@{}",
                    frame.from.spirit_id.as_str(),
                    frame.from.host_id.as_ref().map(|h| h.as_str()).unwrap_or("<none>")
                );
                let data = serde_json::json!({
                    "granter": granter,
                    "frame_from": frame_from,
                });
                let mut resp = A2AJsonRpcResponse::nack(
                    request.id,
                    CODE_CONSENT_GRANTER_MISMATCH,
                    format!("consent granter {granter} does not match frame from {frame_from}"),
                );
                if let A2AJsonRpcResponse::Nack(ref mut n) = resp {
                    n.error.data = Some(data);
                }
                return resp;
            }
            // (2b) Expiry (G2 — the existing check, moved ahead of the allowlist).
            if let Some(valid_until_ns) = envelope.valid_until_ns {
                let now_ns = self.consent_now_ns();
                if now_ns > valid_until_ns {
                    let data = serde_json::json!({
                        "expired_at_ns": valid_until_ns,
                        "now_ns": now_ns,
                    });
                    let mut resp = A2AJsonRpcResponse::nack(
                        request.id,
                        CODE_CONSENT_EXPIRED,
                        format!("consent envelope expired at {valid_until_ns} (now {now_ns})"),
                    );
                    if let A2AJsonRpcResponse::Nack(ref mut n) = resp {
                        n.error.data = Some(data);
                    }
                    return resp;
                }
            }
        }

        // (3) ADR-012 accept-allowlist check.
        if !self.accept_admits(&peer_cfg.allowlists, frame, peer_cfg.peer_id.as_str()) {
            return A2AJsonRpcResponse::nack(
                request.id,
                CODE_INTENT_DENIED,
                format!(
                    "intent {} not in accept_allowlist for peer {}",
                    // Story 8.7 — report the SAME key `accept_admits` tested.
                    Self::consent_match_key(frame),
                    peer_cfg.peer_id.as_str()
                ),
            );
        }

        // (4) Lamport recv_advance.
        let new_clock = self.clock.recv_advance(frame.logical_clock);

        // (5) Push to intake sink (test hook).
        let sink_guard = self.intake_sink.lock().await;
        if let Some(sink) = sink_guard.as_ref() {
            let _ = sink.send(frame.clone());
        }
        drop(sink_guard);

        A2AJsonRpcResponse::ack(
            request.id,
            AckBody {
                delivered: true,
                receiver_logical_clock: new_clock,
            },
        )
    }

    /// Story 8.9 / AC1 (G8) — the CROSS-HOST verified intake entry point.
    ///
    /// The live mTLS handshake learns WHICH peer presented the (TOFU-pinned)
    /// client leaf; `serve_connection` re-derives that `verified_peer` from the
    /// post-handshake `peer_certificates()` and passes it here. Before any
    /// trust-bearing work, the frame's self-asserted `from.host_id` MUST equal
    /// the TLS-verified peer — otherwise a mesh peer holding any one validly
    /// pinned leaf could forge `from.host_id` and act as a confused deputy for
    /// another Host (the audit's headline G8 defect). On a match it delegates to
    /// the shared [`Self::handle_intake`] body byte-for-byte; the loopback router
    /// keeps calling `handle_intake` directly (no wire identity to bind).
    ///
    /// AC1.3: a frame with absent `from.host_id` mismatches the verified peer and
    /// is rejected here, so the shared body's `None → HostId("loopback")`
    /// fallback (still required by the in-process loopback router) is unreachable
    /// on the wire.
    /// Returns `(response, binding_passed)` so the transport layer can
    /// decide whether to increment `intake_entered` without reverse-engineering
    /// the NACK error code (Story 8.9 / AC1.2).
    pub async fn handle_intake_verified(
        &self,
        request: A2AJsonRpcRequest,
        verified_peer: &PeerId,
    ) -> (A2AJsonRpcResponse, bool) {
        // Bind the wire identity to the TLS-verified peer.
        // Framing validation is performed by the shared `handle_intake` body;
        // we check identity BEFORE delegating so a forged frame gets the
        // identity NACK, not a loopback-shaped framing NACK.
        let asserted = request.params.from.host_id.as_ref().map(|h| h.as_str());
        if asserted != Some(verified_peer.as_str()) {
            let data = serde_json::json!({
                "expected": verified_peer.as_str(),
                "asserted": asserted.unwrap_or("<none>"),
            });
            let mut resp = A2AJsonRpcResponse::nack(
                request.id,
                CODE_PEER_IDENTITY_MISMATCH,
                format!(
                    "frame.from.host_id {} does not match TLS-verified peer {}",
                    asserted.unwrap_or("<none>"),
                    verified_peer.as_str()
                ),
            );
            if let A2AJsonRpcResponse::Nack(ref mut n) = resp {
                n.error.data = Some(data);
            }
            return (resp, false);
        }

        (self.handle_intake(request).await, true)
    }
}

/// Map an A2AError variant to its typed IacBusError sub-variant.
/// Preserves structured information so callers can programmatically
/// distinguish intent-denial from partition-timeout from pin-mismatch.
///
/// `pub` so every `A2ATransport` impl (loopback in `maos-a2a`, TCP in
/// `maos-a2a-tcp`) maps its `A2AError` to the kernel's `IacBusError` port type
/// identically (Story 8.6 extraction — was a private fn in `maos-a2a::adapter`).
pub fn map_a2a_error_to_iac_bus(err: A2AError, peer: &str) -> IacBusError {
    match err {
        A2AError::IntentDenied { direction, inner } => {
            let dir = match direction {
                IntentDirection::Send => maos_domain::iac_bus_types::CrossHostIntentDirection::Send,
                IntentDirection::Accept => maos_domain::iac_bus_types::CrossHostIntentDirection::Accept,
            };
            IacBusError::CrossHostIntentDenied {
                peer: peer.to_string(),
                intent: inner.intent,
                direction: dir,
            }
        }
        A2AError::IntentDeniedAtPeer { peer: denied_peer, message } => {
            IacBusError::CrossHostIntentDenied {
                peer: denied_peer,
                intent: message,
                direction: maos_domain::iac_bus_types::CrossHostIntentDirection::Accept,
            }
        }
        A2AError::PinMismatch(e) => {
            IacBusError::CrossHostPinMismatch {
                peer: peer.to_string(),
                detail: e.to_string(),
            }
        }
        A2AError::PinInvalidated { peer: inv_peer, .. } => {
            IacBusError::CrossHostPinMismatch {
                peer: inv_peer,
                detail: format!("pin invalidated — re-pin consent required"),
            }
        }
        A2AError::ConsentExpired { expired_at_ns, now_ns } => {
            IacBusError::CrossHostConsentExpired {
                peer: peer.to_string(),
                expired_at_ns,
                now_ns,
            }
        }
        A2AError::PartitionTimeout { peer: p_peer, frame_id, timeout_secs } => {
            IacBusError::CrossHostPartitionTimeout {
                peer: p_peer,
                frame_id,
                timeout_secs,
            }
        }
        A2AError::TransportFailed(detail)
        | A2AError::DeserializationFailed(detail)
        | A2AError::Io(detail)
        | A2AError::HandshakeFailed { message: detail, .. } => {
            IacBusError::CrossHostTransportFailure {
                peer: peer.to_string(),
                detail,
            }
        }
        A2AError::ConfigInvalid(msg) => {
            IacBusError::CrossHostRouteFailure(msg)
        }
        A2AError::SpiritRestartDetected { peer, prior_boot_nonce, observed_boot_nonce } => {
            IacBusError::CrossHostRouteFailure(format!(
                "spirit restart detected on peer {peer}: prior={prior_boot_nonce} observed={observed_boot_nonce}"
            ))
        }
        // Story 8.9 — trust-binding rejections map to the generic route-failure
        // port type (no new kernel variant; `maos-kernel-core` stays
        // byte-identical). Both carry the security-relevant addresses in the msg.
        A2AError::PeerIdentityMismatch { expected, asserted } => {
            IacBusError::CrossHostRouteFailure(format!(
                "peer identity mismatch: TLS-verified peer {expected}, frame asserted {asserted}"
            ))
        }
        A2AError::ConsentGranterMismatch { granter, frame_from } => {
            IacBusError::CrossHostRouteFailure(format!(
                "consent granter mismatch: envelope granter {granter}, frame from {frame_from}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::A2AProfile;
    use crate::identity::PeerCertFingerprint;
    use crate::tofu::InMemoryTofuPinStore;
    use maos_domain::frame::{
        FrameAddress, FramePayload, PosturePreferences, TaskAssignPayload,
    };
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i8::A2AIntent;
    use crate::identity::PeerId;
    use maos_spirit_abi::identity::{FrameKind, SpiritId};
    use smallvec::smallvec;

    fn make_peer_cfg(allowlists: ConsentAllowlists) -> A2APeerConfig {
        A2APeerConfig {
            peer_id: PeerId::new("loopback"),
            endpoint: "tls://127.0.0.1:7443".into(),
            cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
            profile: A2AProfile::Loopback,
            allowlists,
            partition_timeout_secs: 30,
            consent_ttl_secs: crate::config::DEFAULT_CONSENT_TTL_SECS,
        }
    }

    fn make_frame(host_id: Option<&str>) -> IacFrame {
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: FrameAddress {
                spirit_id: SpiritId::from("a"),
                host_id: host_id.map(|s| HostId(s.to_string())),
                role: None,
            },
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("b"),
                host_id: host_id.map(|s| HostId(s.to_string())),
                role: None,
            }],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "g".into(),
                scope: vec![],
                success_criteria: "s".into(),
                posture_preferences: PosturePreferences::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: IntentLineage::default(),
        }
    }

    async fn pinned_core(allow: ConsentAllowlists) -> A2ARouterCore {
        let cfg = make_peer_cfg(allow);
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        A2ARouterCore::new(vec![cfg], tofu)
    }

    #[tokio::test]
    async fn prepare_outbound_send_admitted_intent_succeeds() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let frame = make_frame(Some("loopback"));
        let (req, _cfg, _id) = core
            .prepare_outbound(frame, &HostId("loopback".to_string()), 0)
            .await
            .expect("prepare");
        assert_eq!(req.method, "iac.deliver");
    }

    #[tokio::test]
    async fn prepare_outbound_send_denied_intent_rejects_at_sender() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let frame = make_frame(Some("loopback"));
        let err = core
            .prepare_outbound(frame, &HostId("loopback".to_string()), 0)
            .await
            .expect_err("must reject at sender");
        assert!(matches!(
            err,
            A2AError::IntentDenied {
                direction: IntentDirection::Send,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn intake_accept_admitted_intent_succeeds() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        core.install_intake_sink(tx).await;
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = core.handle_intake(req).await;
        assert!(matches!(resp, A2AJsonRpcResponse::Ack(_)));
        let delivered = rx.recv().await.expect("delivered to sink");
        assert_eq!(delivered.from.spirit_id.as_str(), "a");
    }

    #[tokio::test]
    async fn intake_denied_intent_returns_nack_with_code() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![],
        };
        let core = pinned_core(allow).await;
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = core.handle_intake(req).await;
        match resp {
            A2AJsonRpcResponse::Nack(n) => {
                assert_eq!(n.error.code, CODE_INTENT_DENIED);
            }
            _ => panic!("expected Nack"),
        }
    }

    #[tokio::test]
    async fn lamport_clock_advances_on_intake() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let mut frame = make_frame(Some("loopback"));
        frame.logical_clock = 100;
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = core.handle_intake(req).await;
        match resp {
            A2AJsonRpcResponse::Ack(a) => {
                assert_eq!(a.result.receiver_logical_clock, 101);
            }
            _ => panic!("expected Ack"),
        }
    }

    #[tokio::test]
    async fn interpret_response_ack_is_ok() {
        let core = pinned_core(ConsentAllowlists::default()).await;
        let resp = A2AJsonRpcResponse::ack(
            1,
            AckBody {
                delivered: true,
                receiver_logical_clock: 1,
            },
        );
        core.interpret_response(&HostId("loopback".to_string()), resp)
            .expect("ack ok");
    }

    /// Helper: attach a consent envelope with the given `valid_until_ns` to a
    /// standard-intent frame addressed to the pinned `loopback` peer.
    fn frame_with_consent_expiry(valid_until_ns: u64) -> IacFrame {
        let mut frame = make_frame(Some("loopback"));
        frame.consent_envelope = Some(maos_domain::frame::ConsentEnvelope {
            consent_id: [0u8; 16],
            // Story 8.9 / AC2 — granter MUST equal the frame's `from` (spirit_id
            // "a", host_id "loopback") so this fixture isolates the EXPIRY check;
            // a granter≠from envelope would now also trip the granter-binding gate.
            granter: FrameAddress {
                spirit_id: SpiritId::from("a"),
                host_id: Some(HostId("loopback".to_string())),
                role: None,
            },
            timestamp_ns: 0,
            intent_class: Some(A2AIntent::new("standard")),
            valid_until_ns: Some(valid_until_ns),
        });
        frame
    }

    /// Story 8.6 review F2 — the consent-expiry "now" is a REAL clock, not a
    /// call counter. A REAL-TIMESTAMP `valid_until_ns` in the past MUST be
    /// rejected (the old per-call counter — values 1,2,3,… — never exceeded a
    /// ~1.7e18 ns timestamp, so it silently ADMITTED expired consent: fail-open).
    /// Pinned clock keeps this deterministic (no wall-clock flake).
    #[tokio::test]
    async fn intake_rejects_real_timestamp_expired_consent_f2() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        // T0 ≈ a real epoch-ns instant; far beyond any plausible call count.
        const T0: u64 = 1_700_000_000_000_000_000;
        let core = pinned_core(allow).await.with_pinned_consent_clock(T0);

        // Expired one nanosecond ago — the exact case the counter admitted.
        let req = A2AJsonRpcRequest::new("iac.deliver", frame_with_consent_expiry(T0 - 1), 1);
        match core.handle_intake(req).await {
            A2AJsonRpcResponse::Nack(n) => assert_eq!(
                n.error.code, CODE_CONSENT_EXPIRED,
                "F2 regression: real-timestamp expired consent was admitted (fail-open)"
            ),
            _ => panic!("F2 regression: expired consent silently admitted (expected NACK)"),
        }
    }

    /// F2 companion — a not-yet-expired real-timestamp envelope is still ACKed
    /// (the fix rejects ONLY genuinely-expired consent, not all bounded consent).
    #[tokio::test]
    async fn intake_admits_unexpired_real_timestamp_consent_f2() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        const T0: u64 = 1_700_000_000_000_000_000;
        let core = pinned_core(allow).await.with_pinned_consent_clock(T0);

        // Valid for one more nanosecond.
        let req = A2AJsonRpcRequest::new("iac.deliver", frame_with_consent_expiry(T0 + 1), 1);
        assert!(
            matches!(core.handle_intake(req).await, A2AJsonRpcResponse::Ack(_)),
            "F2 regression: an unexpired consent envelope was wrongly rejected"
        );
    }

    /// Story 8.9 / AC3 — `prepare_outbound` leaves a frame with NO consent
    /// envelope untouched (no panic, no accidental insertion).
    #[tokio::test]
    async fn prepare_outbound_leaves_none_envelope_unchanged() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let mut frame = make_frame(Some("loopback"));
        frame.consent_envelope = None;
        let (req, _, _) = core
            .prepare_outbound(frame.clone(), &HostId("loopback".to_string()), 0)
            .await
            .expect("prepare_outbound must succeed");
        assert!(
            req.params.consent_envelope.is_none(),
            "prepare_outbound must NOT insert a consent envelope when none was present"
        );
    }
}
