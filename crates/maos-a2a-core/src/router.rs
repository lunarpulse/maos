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

use crate::cohort::{
    CohortConsentDenial, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    ConsentRuptureSink, CrossTeamCrossingPort, CrossingOutcome, CrossingRefusal, DigestFrameClass,
    DigestReadPort, DigestReplyObservation, HaltReceiptObserver, LegacyCohortManifestGate,
    LegacyCrossTeamCrossingPort, LegacyDigestReadPort, LegacyHaltReceiptObserver,
    COHORT_INTENT_COLLECTIVE_SHARE, COHORT_INTENT_DIGEST_READ, CROSSING_EVENT_TYPE,
    CROSS_TEAM_COLLECTIVE_ERASE_INTENT, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use crate::config::A2APeerConfig;
use crate::consent::{AllowlistDirection, ConsentAllowlists, EIntentDenied};
use crate::error::{A2AError, IntentDirection, UnclassifiedReason};
use crate::identity::{PeerCertFingerprint, PeerId};
use crate::tofu::TofuPinStore;
use crate::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, AckBody, CODE_CONSENT_EXPIRED,
    CODE_CONSENT_GRANTER_MISMATCH, CODE_CONSENT_UNCLASSIFIED, CODE_CROSSING_SOURCE_TEAM_UNBOUND,
    CODE_CROSS_TEAM_CROSSING_REFUSED, CODE_INTENT_DENIED, CODE_INTERNAL,
    CODE_PEER_IDENTITY_MISMATCH, CODE_PIN_MISMATCH_NOT_PINNED, CODE_SPIRIT_RESTART_DETECTED,
    CODE_TEAM_IDENTITY_MISMATCH,
};
use crate::transport::logical_clock::LamportClock;
use async_trait::async_trait;
use dashmap::DashMap;
use maos_domain::frame::{FramePayload, IacFrame};
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
    async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), A2AError>;

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

/// Story 8.8 / AC2 — the result of the single shared consent-classification seam
/// [`A2ARouterCore::consent_decision`]. Either a **Classified** frame (carrying
/// the canonical fine-grained match key) or an **Unclassified** frame (carrying
/// the precise reason it cannot be safely matched). Private — the public surface
/// exposes only the typed `A2AError`/NACK consequences.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConsentDecision {
    /// A present, ≤128-byte, canonical `intent_class` — the match key for the
    /// allowlist (fine-grained, never a band token).
    Classified(String),
    /// No safely-matchable fine-grained intent (absent / non-canonical /
    /// oversized) — denied under fail-closed; band-projected under band-fallback.
    Unclassified { reason: UnclassifiedReason },
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
    /// Out-of-kernel cohort manifest currentness port. The legacy default keeps
    /// non-cohort deployments byte-for-byte compatible; cohort composition
    /// injects a verified cache before any transport is started.
    cohort_manifest_gate: Arc<dyn CohortManifestGate>,
    /// Story 12.3 — out-of-kernel halt-receipt presence observer. The legacy
    /// no-op default keeps non-cohort deployments byte-for-byte compatible;
    /// cohort composition injects the same `CohortManifestState` used for the
    /// gate (P7b — single-object wiring, no transposition footgun). Observed on
    /// the TLS-verified intake path only (P5r).
    halt_receipt_observer: Arc<dyn HaltReceiptObserver>,
    /// Story 12.4a — out-of-kernel cohort digest-read correlation port. The
    /// legacy no-op default keeps non-cohort deployments byte-for-byte
    /// compatible; cohort composition injects the same `CohortManifestState`
    /// used for the gate/observer (P7b — single-object wiring). Authorizes the
    /// correlated-reply send/accept exemptions WITHOUT touching the unchanged
    /// `send_admits`/`accept_admits` seam bodies (AC2).
    digest_read_port: Arc<dyn DigestReadPort>,
    /// Story 13.6b — out-of-kernel cross-team crossing applier. The legacy
    /// default classifies every frame as `NotCrossing`, so non-cohort
    /// deployments and every pre-13.6b test are byte-for-byte unaffected;
    /// cohort composition injects the adapter that owns the single
    /// `LoomLiteStore` (D-5). Reached ONLY from `handle_intake_verified`, only
    /// after the intake body ACKs, and only with a team this router has already
    /// authenticated against the signed V4 manifest (D-13).
    crossing_port: Arc<dyn CrossTeamCrossingPort>,
    /// Story 13.6a (review P1) — this host's own TLS leaf fingerprint, wired by
    /// the transport at bind from the loaded identity chain. The Send-seam team
    /// declaration is returned only when this equals the signed
    /// `CohortMember.fingerprint` for the local host, so a signed reissue that
    /// rotates the local fingerprint away stops this host from originating a
    /// crossing until the operator realigns. `None` (non-cohort deployments)
    /// fails closed exactly like an unauthenticated endpoint.
    local_leaf_fingerprint: Option<PeerCertFingerprint>,
    /// Optional intake sink for tests — when set, accepted frames are
    /// pushed here so test code can observe them.
    intake_sink: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<IacFrame>>>>,
    /// Fail-closed persistence seam for consent ruptures. The sink appends
    /// synchronously before the deny response is returned, so queue pressure,
    /// shutdown, or a detached drain task cannot silently erase evidence.
    rupture_sink: Arc<tokio::sync::Mutex<Option<Arc<dyn ConsentRuptureSink>>>>,
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
            rupture_sink: Arc::new(tokio::sync::Mutex::new(None)),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            consent_now_ns: None,
            local_leaf_fingerprint: None,
            cohort_manifest_gate: Arc::new(LegacyCohortManifestGate),
            halt_receipt_observer: Arc::new(LegacyHaltReceiptObserver),
            digest_read_port: Arc::new(LegacyDigestReadPort),
            crossing_port: Arc::new(LegacyCrossTeamCrossingPort),
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

    /// Inject the single verified cohort-manifest currentness authority.
    /// This is a builder so existing constructors/callers remain compatible.
    pub fn with_cohort_manifest_gate(mut self, gate: Arc<dyn CohortManifestGate>) -> Self {
        self.cohort_manifest_gate = gate;
        self
    }

    /// Story 13.6a (review P1) — wire the local TLS leaf fingerprint the
    /// Send-seam team binding is checked against. Builder, same discipline as
    /// [`Self::with_cohort_manifest_gate`].
    pub fn with_local_leaf_fingerprint(mut self, fingerprint: PeerCertFingerprint) -> Self {
        self.local_leaf_fingerprint = Some(fingerprint);
        self
    }

    /// Story 12.3 — inject the out-of-kernel halt-receipt presence observer.
    /// A builder (mirrors [`Self::with_cohort_manifest_gate`]) so the daemon
    /// wires it in one step with the SAME `CohortManifestState` used for the
    /// gate (`.with_cohort_manifest_gate(state.clone()).with_halt_receipt_observer(state.clone())`)
    /// — the two named builders eliminate the adjacent-`Arc` transposition
    /// footgun a positional `bind` param would carry (P7b).
    pub fn with_halt_receipt_observer(mut self, observer: Arc<dyn HaltReceiptObserver>) -> Self {
        self.halt_receipt_observer = observer;
        self
    }

    /// Story 12.4a — inject the out-of-kernel cohort digest-read correlation
    /// port. A builder (mirrors [`Self::with_halt_receipt_observer`]) so the
    /// daemon wires the SAME `CohortManifestState` used for the gate/observer.
    pub fn with_digest_read_port(mut self, port: Arc<dyn DigestReadPort>) -> Self {
        self.digest_read_port = port;
        self
    }

    /// Story 13.6b — inject the out-of-kernel cross-team crossing applier.
    /// A builder (mirrors [`Self::with_digest_read_port`]) so the daemon wires
    /// it before the accept loop spawns and no inbound connection observes a
    /// legacy-applier window (12.4a's P7c).
    pub fn with_cross_team_crossing_port(mut self, port: Arc<dyn CrossTeamCrossingPort>) -> Self {
        self.crossing_port = port;
        self
    }

    /// Read a string field out of a NACK `data` object, defaulting to empty.
    /// Shared by the two Story 13.6b `interpret_response` arms.
    fn nack_str(data: Option<&serde_json::Value>, key: &str) -> String {
        data.and_then(|d| d.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    }

    /// Extract the operator-requested destination from a crossing control body
    /// without depending on the composition-root control type.
    fn crossing_to_team(frame: &IacFrame) -> String {
        let FramePayload::TelemetryEvent(payload) = &frame.payload else {
            return String::new();
        };
        serde_json::from_str::<serde_json::Value>(&payload.data)
            .ok()
            .and_then(|value| {
                value
                    .get("to_team")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default()
    }

    fn is_crossing_frame(frame: &IacFrame) -> bool {
        matches!(
            &frame.payload,
            FramePayload::TelemetryEvent(payload) if payload.event_type == CROSSING_EVENT_TYPE
        )
    }

    /// Story 13.6b / AC2+AC3 — the wire NACK for an applier refusal. The code +
    /// `data` taxonomy lives on [`CrossingRefusal::wire`].
    ///
    /// `pub` so the refusal contract is provable through the public surface: the
    /// legs that assert AC3's weld rides its OWN code, and that denied / stale /
    /// unavailable stay three distinct emitter-side outcomes, exercise THIS
    /// production shaping rather than a hand-built NACK that could drift from it.
    pub fn crossing_refusal_nack(id: u64, refusal: &CrossingRefusal) -> A2AJsonRpcResponse {
        let (code, data) = refusal.wire();
        A2AJsonRpcResponse::nack_with_data(
            id,
            code,
            format!("cross-team crossing refused: {}", refusal.reason()),
            data,
        )
    }

    /// The "now" used for consent-envelope expiry: the pinned test clock if set,
    /// else the real wall clock.
    fn consent_now_ns(&self) -> u64 {
        self.consent_now_ns.unwrap_or_else(wall_now_ns)
    }

    /// Install the intake sink — the seam a receiving Host attaches its frame
    /// consumer through. Live transports install this before exposing their
    /// listener; tests may replace it explicitly.
    ///
    /// **This doc said "test-only hook" until j1-crosshost-2b, and that sentence
    /// was load-bearing wrong** (G1 / G14.i). The loopback pair installs it in
    /// PRODUCTION (`crates/maos-a2a/src/pairing.rs:112`) and the J1 composition
    /// root DRAINS it in production (`crates/maos-bin/src/delegation.rs:173`,
    /// `:190`); the live TCP transport now installs it inside its bind chain
    /// before `TcpListener::bind` (`maos-a2a-tcp/src/transport.rs`,
    /// `bind_with_intake_sink`). Because the word "test-only" was believed, a
    /// real daemon authenticated its peer, ran TOFU, checked the boot nonce,
    /// evaluated consent, advanced the Lamport clock,
    /// ACKed `delivered: true` — and dropped the frame on the floor, because
    /// nothing production-side ever called this. Do not restore that wording.
    pub async fn install_intake_sink(&self, sink: tokio::sync::mpsc::Sender<IacFrame>) {
        let mut guard = self.intake_sink.lock().await;
        *guard = Some(sink);
    }

    /// Hand an accepted frame to the installed intake sink.
    ///
    /// Three outcomes, and the third one is why this function exists
    /// (j1-crosshost-2b AC1.2 / G1b.ii; §A6 review D2/P17 bounded the channel):
    ///
    /// * **no sink installed** → `Ok(())`. A deployment with no J1 consumer (the
    ///   cohort daemon's reserved-intent paths, `smoke-a2a-tcp-8-6`) is not an
    ///   error, and this is the pre-existing behaviour, stated rather than
    ///   inherited.
    /// * **sink installed, send accepted** → `Ok(())`.
    /// * **sink installed, receiver DROPPED *or channel FULL*** → `Err(())`. The
    ///   old code was `let _ = sink.send(frame.clone());` at BOTH push sites,
    ///   which folded every failure into success: a consumer whose receiver had
    ///   been dropped silently discarded every frame **while the sender was still
    ///   told `delivered: true`** — G1's exact shape one layer in. The §A6 review
    ///   (D2, ratified) additionally BOUNDED the channel: an authenticated peer
    ///   must not be able to queue unbounded frames behind one slow worker, so a
    ///   FULL channel is the same verdict as a dead consumer — `try_send`, never
    ///   a blocking `send` that would park the accept loop. Callers MUST convert
    ///   `Err` into a NACK; an ACK here is a lie about durability, and the sender
    ///   has no other way to learn the frame is gone.
    ///   *Known rendering gap, filed for `j1-crosshost-2c`:* the resulting
    ///   `CODE_INTERNAL` NACK itself falls through `interpret_response`'s
    ///   catch-all into `TransportFailed` at the sender — the same misattribution
    ///   shape as H13, on a code this story newly emits. 2c's fault-injection
    ///   preflight owns typing it.
    async fn push_to_intake_sink(&self, frame: &IacFrame) -> Result<(), ()> {
        let guard = self.intake_sink.lock().await;
        match guard.as_ref() {
            Some(sink) => sink.try_send(frame.clone()).map_err(|_| ()),
            None => Ok(()),
        }
    }

    /// Install the fail-closed rupture persistence seam. Live transports install
    /// this before exposing their listener; tests may replace it explicitly.
    pub async fn install_rupture_sink(&self, sink: Arc<dyn ConsentRuptureSink>) {
        let mut guard = self.rupture_sink.lock().await;
        *guard = Some(sink);
    }

    /// Emit and durably persist a genuine typed `ConsentRupture` before the
    /// denial response is returned. Digest-read denials require a sink; missing
    /// or failed persistence is surfaced as an internal fail-closed NACK rather
    /// than masquerading as a normal, visible refusal.
    async fn emit_consent_rupture(&self, frame: &IacFrame) -> Result<(), String> {
        let sink = {
            let guard = self.rupture_sink.lock().await;
            guard.as_ref().cloned()
        };
        let Some(sink) = sink else {
            if Self::consent_match_key(frame).eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ) {
                return Err("digest-read rupture sink is not installed".into());
            }
            return Ok(());
        };
        let now_ns = self.consent_now_ns();
        // Story 8.13.1-review / P4 — unique-per-denial correlation id: the first
        // half preserves the original frame_id for correlation; the second half
        // is a monotonic nonce so retries / re-routing cannot collide.
        let rupture_id = {
            let mut id = frame.frame_id;
            let nonce = self.alloc_id();
            id[8..16].copy_from_slice(&nonce.to_le_bytes());
            id
        };
        // Story 8.13.1-review / P1 — the denier is the receiver host; if frame.to
        // is empty (protocol invariant violation at this point), skip emission
        // rather than fall back to the sender identity, which would be semantically
        // wrong (the rupture would appear to come from the denied party).
        let Some(denier) = frame.to.first().cloned() else {
            return Err("denied frame has no recipient to attribute rupture".into());
        };
        let payload = maos_domain::frame::ConsentRupturePayload {
            rupture_id,
            original_frame_id: frame.frame_id,
            original_kind: frame.kind,
            accepted: vec![],
            rejected: vec![maos_domain::frame::RuptureRejection {
                address: denier.clone(),
                reason: maos_domain::frame::RuptureReason::IntentAllowlistMismatch,
            }],
            ruptured_at_ns: now_ns,
        };
        let rupture = IacFrame {
            frame_id: rupture_id,
            timestamp_ns: now_ns,
            logical_clock: frame.logical_clock,
            from: denier,
            to: std::iter::once(frame.from.clone()).collect(),
            kind: maos_spirit_abi::identity::FrameKind::ConsentRupture,
            intent: frame.intent,
            payload: maos_domain::frame::FramePayload::ConsentRupture(payload),
            auto_marker: maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
            // Preserve the denied fine-grained intent for truthful audit
            // attribution; the rupture itself is never routed through consent.
            consent_envelope: frame.consent_envelope.clone(),
            intent_lineage: frame.intent_lineage.clone(),
        };
        sink.append(&rupture)
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
    /// Story 8.7 — the 3-band projection primitive. **Story 8.8 (Option 2,
    /// 2026-06-07):** the cross-Host router no longer band-downgrades unclassified
    /// frames (they are denied fail-closed), so this is NO LONGER called by
    /// [`Self::consent_match_key`]. It is retained `pub` and unchanged because
    /// removing it would register as an abi-diff Removed (the 8.6/8.7 lesson) and
    /// it remains a useful band-projection utility for external callers; it is NOT
    /// a consent-enforcement path.
    #[deprecated(
        since = "8.8.0",
        note = "band-projection is no longer on the consent-enforcement path; use fine-grained intent_class instead"
    )]
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
        match Self::consent_decision(frame) {
            ConsentDecision::Classified(s) => s,
            // Story 8.8 Option 2 (team consensus 2026-06-07: Winston + Murat +
            // security red-team) — there is NO band-downgrade path. The
            // unconditional fail-closed gate in `prepare_outbound` / `handle_intake`
            // denies every unclassified frame BEFORE any allowlist match, so this
            // arm is unreachable in production; it returns an empty key (matches no
            // canonical allowlist entry — deny-shaped) rather than band-projecting,
            // so no silent downgrade can ever occur even if a future caller reaches
            // `consent_match_key` directly.
            // Story 8.8 review fix — use an explicitly non-canonical sentinel
            // (underscores violate the canonical grammar) instead of an empty
            // string, so there is zero chance of an accidental allowlist match
            // if a future caller reaches this arm directly.
            ConsentDecision::Unclassified { .. } => "__UNREACHABLE_UNCLASSIFIED__".to_string(),
        }
    }

    /// Story 8.8 / AC2 — the **single, shared** classification seam. Both the
    /// allowlist callers (`send_admits`/`accept_admits` via [`Self::consent_match_key`])
    /// AND the fail-closed deny-construction sites consult this one function, so
    /// "the key tested == the key reported" (the 8.7 invariant) holds and send and
    /// accept can never diverge on classification.
    ///
    /// A frame is **Classified** iff it carries an `intent_class` that is present,
    /// ≤ [`MAX_CANONICAL_INTENT_LEN`] bytes, AND canonical
    /// (`A2AIntent::is_canonical`). Otherwise it is **Unclassified**, carrying the
    /// precise [`UnclassifiedReason`] (`Absent` / `Oversized` / `NonCanonical`).
    ///
    /// Note (AC1 widening): unlike the pre-8.8 `consent_match_key`, a
    /// *present-but-non-canonical* `intent_class` is now **Unclassified**
    /// (`NonCanonical`) rather than used verbatim as an unmatchable key — so under
    /// fail-closed a garbage intent denies legibly instead of silently never
    /// matching.
    ///
    /// [Source: docs/adr/ADR-012-typed-intent-a2a-consent.md] — consent is
    /// `(peer-identity, intent-class)` with an open intent vocabulary.
    fn consent_decision(frame: &IacFrame) -> ConsentDecision {
        let Some(intent) = frame
            .consent_envelope
            .as_ref()
            .and_then(|e| e.intent_class.as_ref())
        else {
            return ConsentDecision::Unclassified {
                reason: UnclassifiedReason::Absent,
            };
        };
        let reason = if intent.as_str().len() > MAX_CANONICAL_INTENT_LEN {
            UnclassifiedReason::Oversized
        } else if !intent.is_canonical() {
            UnclassifiedReason::NonCanonical
        } else {
            return ConsentDecision::Classified(intent.as_str().to_string());
        };
        ConsentDecision::Unclassified { reason }
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

    fn is_reserved_cohort_intent(intent: &str) -> bool {
        intent.eq_ignore_ascii_case(RESERVED_INTENT_REISSUE)
            || intent.eq_ignore_ascii_case(RESERVED_INTENT_HALT_RECEIPT)
    }

    /// The sole router call to the role/version consent port. Both route seams
    /// use this synchronous chokepoint; no verdict can straddle an await.
    fn cohort_consent_decision(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        self.cohort_manifest_gate.consent_decision(
            seam,
            counterparty,
            acting_role,
            intent,
            sender_manifest_version,
        )
    }

    fn cohort_denial_data(reason: &CohortConsentDenial) -> serde_json::Value {
        match reason {
            CohortConsentDenial::ManifestSkew {
                sender_version,
                receiver_version,
                delta,
            } => serde_json::json!({
                "reason": reason.reason(),
                "sender_version": sender_version,
                "receiver_version": receiver_version,
                "delta": delta,
            }),
            _ => serde_json::json!({ "reason": reason.reason() }),
        }
    }

    /// Send-allowlist enforcement. The peer's `send_allowlist` enumerates
    /// `A2AIntent` strings the operator wills THIS Host to SEND to that peer.
    fn send_admits(&self, allow: &ConsentAllowlists, frame: &IacFrame, peer_id: &str) -> bool {
        let s = Self::consent_match_key(frame);
        if Self::is_reserved_cohort_intent(&s) {
            return true;
        }
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
        if Self::is_reserved_cohort_intent(&s) {
            return true;
        }
        let admitted = allow
            .accept_allowlist
            .iter()
            .any(|i| i.as_str().eq_ignore_ascii_case(&s));
        if !admitted {
            self.warn_unreachable_entries(
                &allow.accept_allowlist,
                AllowlistDirection::Accept,
                peer_id,
            );
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

        // (0) Story 8.8 / AC1 (G7) — UNCONDITIONAL fail-closed cross-Host consent
        // (team consensus 2026-06-07, Option 2: no band-fallback toggle exists).
        // An unclassified frame (absent/non-canonical/oversized intent_class) is
        // DENIED here, BEFORE the send-allowlist, and the frame NEVER leaves — it
        // is NEVER silently downgraded to the 3-band projection (the red-team
        // non-negotiable: "deny ONLY unclassified, never silently downgrade").
        // This is also the runtime sender-completeness backstop (AC3, NON-
        // exemptible): a reference sender that fails to populate a canonical
        // intent_class gets `ConsentUnclassified { Send }` and the frame is not
        // transmitted. Same-Host IAC never reaches this core (`iac_bus.rs`).
        if let ConsentDecision::Unclassified { reason } = Self::consent_decision(&frame) {
            return Err(A2AError::ConsentUnclassified {
                direction: IntentDirection::Send,
                reason,
            });
        }
        if Self::consent_match_key(&frame).eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
            && matches!(
                self.digest_read_port.classify(&frame),
                DigestFrameClass::Invalid
            )
        {
            return Err(A2AError::ConfigInvalid(
                "malformed or out-of-bounds cohort:digest-read control".into(),
            ));
        }

        // Story 12.4a — correlated digest-read reply send-exemption (AC2). A
        // reply the target OWES for a request it ADMITTED is authorized by that
        // admit, NOT re-gated by a second send-allowlist check — this closes the
        // split story's dead-air (admit-the-ask / deny-the-answer) and the
        // topic-exists-but-withheld side-channel. Scoped narrowly: ONLY a
        // `cohort:digest-read` Reply whose `request_id` the port authorizes for
        // THIS peer bypasses the send seam + Send cohort overlay. Requests,
        // unsolicited "replies", and every other intent take the unchanged seam
        // path below (an unauthorized reply is denied, never silently exempted).
        // The `send_admits`/`cohort_consent_decision` bodies are untouched.
        let digest_reply_send_exempt = Self::consent_match_key(&frame)
            .eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
            && matches!(
                self.digest_read_port.classify(&frame),
                DigestFrameClass::Reply { request_id }
                    if self.digest_read_port.authorize_reply_send(peer, &request_id)
            );

        // Story 13.6a — is this the cross-team crossing intent? Computed BEFORE
        // `frame` is consumed, and used three times below: the `Defer`-as-refusal
        // rule (AC4), the emitter self-check, and the source-team stamp (AC1/AC2).
        let crossing = {
            let intent = Self::consent_match_key(&frame);
            intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE)
                || intent.eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT)
        };

        // Story 13.6a review P2 — the Send-seam team declaration, captured from
        // the SAME locked manifest snapshot as the consent verdict below, so a
        // hot reissue cannot change the local host's team between the consent
        // decision and the source-team stamp.
        let mut send_team_declaration: Option<String> = None;
        let cohort_context = if digest_reply_send_exempt {
            None
        } else {
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

            let intent = Self::consent_match_key(&frame);
            if Self::is_reserved_cohort_intent(&intent) {
                None
            } else {
                // Review P1/P2 — ONE gate call, ONE locked snapshot: the
                // verdict decides consent, the declaration stamps the team, and
                // the declaration is returned only when this host's own leaf
                // fingerprint equals its signed `CohortMember.fingerprint`.
                let (verdict, declaration) = self.cohort_manifest_gate.consent_and_team(
                    CohortConsentSeam::Send,
                    peer,
                    self.local_leaf_fingerprint.as_ref(),
                    None,
                    &intent,
                    None,
                );
                send_team_declaration = declaration;
                match verdict {
                    // Story 13.6a / AC4 — on the crossing intent ONLY, `Defer`
                    // is a refusal, routed through the SHIPPED `Deny` path under
                    // its own attributable cause. `Defer` is returned BEFORE the
                    // gate's self-membership check and is otherwise a pass, which
                    // is correct for 12.1's bilateral path and wrong for a
                    // crossing. Every other intent is byte-for-byte unchanged.
                    CohortConsentVerdict::Defer if crossing => {
                        return Err(A2AError::CohortConsentDenied {
                            direction: IntentDirection::Send,
                            reason: CohortConsentDenial::CrossingDeferRefused,
                        });
                    }
                    CohortConsentVerdict::Defer => None,
                    CohortConsentVerdict::AdmitOutbound {
                        acting_role,
                        manifest_version,
                    } => Some((acting_role, manifest_version)),
                    CohortConsentVerdict::Admit => None,
                    CohortConsentVerdict::NotCurrent => {
                        return Err(A2AError::ConfigInvalid(format!(
                            "cohort manifest is not current for peer {}",
                            peer.as_str()
                        )));
                    }
                    CohortConsentVerdict::Deny(reason) => {
                        return Err(A2AError::CohortConsentDenied {
                            direction: IntentDirection::Send,
                            reason,
                        });
                    }
                }
            }
        };

        // Story 13.6a / AC1+AC2 — the emitter-side team binding. The stamp the
        // receiver checks is read from THIS host's own signed V4 declaration,
        // never from caller-supplied input, so a compromised caller cannot pick
        // the team it speaks for. Fail-closed: a member with no declared team
        // cannot originate a crossing at all.
        let source_team_stamp = match (crossing, send_team_declaration) {
            (true, Some(team)) => Some(team),
            // Fail-closed: a member with no signed declaration — or one whose
            // local leaf no longer equals the signed `CohortMember.fingerprint`
            // after a rotation — cannot originate a crossing at all.
            (true, None) => {
                return Err(A2AError::CohortTeamIdentityRefused {
                    direction: IntentDirection::Send,
                    // The failing endpoint is the LOCAL emitter (the Send-seam
                    // declaration looks up `self.local_host` in the gate), never
                    // the destination — name the frame's own origin so the audit
                    // record points at the member whose manifest row is missing.
                    host: frame
                        .from
                        .host_id
                        .as_ref()
                        .map(|host| host.as_str().to_string())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    claimed_team: None,
                    declared: None,
                });
            }
            (false, _) => None,
        };

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
        let mut request =
            A2AJsonRpcRequest::new("iac.deliver", frame, id).with_boot_nonce(boot_nonce);
        if let Some((acting_role, manifest_version)) = cohort_context {
            request = request
                .with_cohort_acting_role(acting_role)
                .with_cohort_manifest_version(manifest_version);
        }
        if let Some(source_team) = source_team_stamp {
            request = request.with_cohort_source_team(source_team);
        }
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
                                d.get("expected")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                                d.get("asserted")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
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
                                d.get("granter")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                                d.get("frame_from")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                            )
                        })
                        .unwrap_or_default();
                    Err(A2AError::ConsentGranterMismatch {
                        granter,
                        frame_from,
                    })
                }
                // Story 8.8 / AC1 (G7) — the receiver fail-closed-denied an
                // unclassified frame. Interpret it back into the distinct typed
                // signal (NOT conflated with -32001 IntentDeniedAtPeer), carrying
                // the reason so the sender can log/act on it legibly.
                CODE_CONSENT_UNCLASSIFIED => {
                    let reason = n
                        .error
                        .data
                        .as_ref()
                        .and_then(|d| d.get("reason"))
                        .and_then(|v| serde_json::from_value::<UnclassifiedReason>(v.clone()).ok()) // xtask-serde-allow: graceful — .ok() already discards the error; .unwrap_or(Absent) is the deliberate fail-closed fallback
                        .unwrap_or(UnclassifiedReason::Absent);
                    Err(A2AError::ConsentUnclassifiedAtPeer {
                        peer: peer.as_str().to_string(),
                        reason,
                    })
                }
                // Story 13.6a / AC2 — the receiver refused the frame's team
                // claim at its accept seam. Decode the claim/declaration pair
                // into the typed mirror (NOT the transport-failure catch-all),
                // so an audit can tell a cross-team impersonation refusal from
                // an unrelated fault — the same distinction AC2 draws between
                // -32010 and -32007 on the emit side.
                CODE_TEAM_IDENTITY_MISMATCH => {
                    let (host, claimed_team, declared) = n
                        .error
                        .data
                        .as_ref()
                        .map(|d| {
                            (
                                d.get("peer")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                                d.get("claimed_team")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string),
                                d.get("declared_team")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string),
                            )
                        })
                        .unwrap_or_default();
                    Err(A2AError::CohortTeamIdentityRefused {
                        // The local operation that failed was the SEND; the
                        // refusal itself happened at the peer's accept seam.
                        direction: IntentDirection::Send,
                        host: if host.is_empty() {
                            peer.as_str().to_string()
                        } else {
                            host
                        },
                        claimed_team,
                        declared,
                    })
                }
                // Story 13.6b / AC3 — the applier refused because the crossing
                // PAYLOAD named a team the mesh did not authenticate. Its own
                // typed mirror: never folded into -32012's honest-but-ungranted
                // refusal, and never into the transport catch-all.
                CODE_CROSSING_SOURCE_TEAM_UNBOUND => {
                    let data = n.error.data.as_ref();
                    Err(A2AError::CrossingSourceTeamUnbound {
                        envelope_team: Self::nack_str(data, "envelope_team"),
                        payload_team: Self::nack_str(data, "payload_team"),
                    })
                }
                // Story 13.6b / AC2 — the applier refused an AUTHENTICATED
                // crossing on its own merits. The ordered pair + intent + the
                // stable reason token keep denied / stale / unavailable three
                // distinct observable outcomes at the emitter (D-15).
                CODE_CROSS_TEAM_CROSSING_REFUSED => {
                    let data = n.error.data.as_ref();
                    Err(A2AError::CrossTeamCrossingRefused {
                        reason: Self::nack_str(data, "reason"),
                        detail: Self::nack_str(data, "detail"),
                        from_team: Self::nack_str(data, "from_team"),
                        to_team: Self::nack_str(data, "to_team"),
                        intent: Self::nack_str(data, "intent"),
                    })
                }
                // j1-crosshost-2b AC4.1 / H13 — the ONE arm this story owes, and it
                // is a security repair, not a nicety.
                //
                // Host B detects a boot-nonce mismatch, PERMANENTLY invalidates the
                // pin (`tofu.rs` sets `Invalidated::SpiritRestarted`) and NACKs
                // `-32004`. Until this arm existed that NACK fell through the
                // catch-all below into `A2AError::TransportFailed`, which
                // `map_a2a_error_to_iac_bus` turns into
                // `IacBusError::CrossHostTransportFailure` — so the operator was told
                // **the network broke**, and the action that implies, retry, is the
                // one action that can never work: the pin is dead until a config is
                // edited and a process restarted (`await_repin_consent` has zero
                // production callers and its default hook returns `TimedOut`).
                // A permanent security refusal presented as a transient fault.
                //
                // It was structurally UNREACHABLE in rung 1: `if request.boot_nonce
                // != 0` guards the detection and loopback stamps the zero sentinel,
                // so the branch had never executed. Two processes with real nonces
                // make it reachable — this story. A defect that was unreachable,
                // inherited by the story that makes it reachable.
                //
                // `PinInvalidated` is not a new type and is semantically exact:
                // restart detection IS pin invalidation, re-pin IS the designed
                // recovery, and the variant already maps typed to
                // `IacBusError::CrossHostPinMismatch`. Lands in `maos-a2a-core` at
                // ZERO headroom as a correctness repair on a security path, which
                // **`xtask/kloc.toml:87`** says a ceiling must never block.
                //
                // SCOPE WALL: this repairs ONLY the code this story makes reachable.
                // `interpret_response` types **9 of the 16** defined NACK codes; the
                // remaining six fall-throughs (`PARSE_ERROR`, `INVALID_REQUEST`,
                // `METHOD_NOT_FOUND`, `TIMEOUT`, `FRAME_TOO_LARGE`, `INTERNAL`) are
                // not newly reachable here. The 9-of-16 census is recorded in the
                // j1-crosshost-2b dev record with a named owner; a nine-arm refactor
                // inside a frozen crate is a different story.
                CODE_SPIRIT_RESTART_DETECTED => Err(A2AError::PinInvalidated {
                    peer: peer.as_str().to_string(),
                    awaiting_repin: true,
                }),
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
        self.handle_intake_inner(request, None).await
    }

    /// The shared intake body. `precomputed_verdict` is the Accept-seam consent
    /// verdict [`Self::handle_intake_verified`] already read from the SAME
    /// locked manifest snapshot as the team declaration (Story 13.6a review
    /// P2), so admission cannot race a hot reissue that changed the host's team
    /// between the identity check and the consent decision. `None` — every
    /// direct caller — re-decides from a fresh snapshot, byte-for-byte the
    /// pre-existing behavior.
    async fn handle_intake_inner(
        &self,
        request: A2AJsonRpcRequest,
        precomputed_verdict: Option<CohortConsentVerdict>,
    ) -> A2AJsonRpcResponse {
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
                    return A2AJsonRpcResponse::nack_with_data(
                        request.id,
                        CODE_SPIRIT_RESTART_DETECTED,
                        format!(
                            "Spirit restart detected on peer {}: prior_boot_nonce={prior} observed_boot_nonce={observed}",
                            peer_cfg.peer_id.as_str()
                        ),
                        data,
                    );
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
                    envelope
                        .granter
                        .host_id
                        .as_ref()
                        .map(|h| h.as_str())
                        .unwrap_or("<none>")
                );
                let frame_from = format!(
                    "{}@{}",
                    frame.from.spirit_id.as_str(),
                    frame
                        .from
                        .host_id
                        .as_ref()
                        .map(|h| h.as_str())
                        .unwrap_or("<none>")
                );
                let data = serde_json::json!({
                    "granter": granter,
                    "frame_from": frame_from,
                });
                return A2AJsonRpcResponse::nack_with_data(
                    request.id,
                    CODE_CONSENT_GRANTER_MISMATCH,
                    format!("consent granter {granter} does not match frame from {frame_from}"),
                    data,
                );
            }
            // (2b) Expiry (G2 — the existing check, moved ahead of the allowlist).
            if let Some(valid_until_ns) = envelope.valid_until_ns {
                let now_ns = self.consent_now_ns();
                if now_ns > valid_until_ns {
                    let data = serde_json::json!({
                        "expired_at_ns": valid_until_ns,
                        "now_ns": now_ns,
                    });
                    return A2AJsonRpcResponse::nack_with_data(
                        request.id,
                        CODE_CONSENT_EXPIRED,
                        format!("consent envelope expired at {valid_until_ns} (now {now_ns})"),
                        data,
                    );
                }
            }
        }

        // (2.5) Story 8.8 / AC1 (G7) — UNCONDITIONAL fail-closed cross-Host consent
        // (team consensus 2026-06-07, Option 2: no band-fallback toggle exists).
        // Evaluated immediately BEFORE the accept-allowlist (it replaces the point
        // where an unclassified frame would otherwise silently band-fall-back). The
        // 8.9 consent block (granter → expiry) ran first, so a stolen/expired
        // envelope already failed; here the WELL-FORMEDNESS of the classification is
        // checked before allowlist matching. An unclassified frame is DENIED with
        // the distinct CODE_CONSENT_UNCLASSIFIED (-32009) — NOT -32001 (which means
        // classified-but-not-allowlisted) — and the NACK carries the peer + the
        // reason so the deny is legible in the Transparency Log.
        if let ConsentDecision::Unclassified { reason } = Self::consent_decision(frame) {
            let peer = peer_cfg.peer_id.as_str();
            let data = serde_json::json!({ "reason": reason, "peer": peer });
            return A2AJsonRpcResponse::nack_with_data(
                request.id,
                CODE_CONSENT_UNCLASSIFIED,
                format!("cross-Host consent unclassified ({reason}) from peer {peer} — fail-closed deny (no 3-band downgrade)"),
                data,
            );
        }
        if Self::consent_match_key(frame).eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
            && matches!(
                self.digest_read_port.classify(frame),
                DigestFrameClass::Invalid
            )
        {
            return A2AJsonRpcResponse::nack(
                request.id,
                CODE_CONSENT_UNCLASSIFIED,
                "malformed or out-of-bounds cohort:digest-read control",
            );
        }

        // Story 12.4a — correlated digest-read reply accept-exemption (AC2). A
        // reply THIS host is awaiting (it sent the matching request to this peer)
        // is accepted without a fresh consent decision: the target's admit was
        // the SINGLE decision, and a reader re-gating its OWN solicited reply is
        // the dead-air the split story forbade. Scoped narrowly: ONLY a
        // `cohort:digest-read` Reply whose `request_id` the port confirms this
        // host is awaiting FROM THIS peer is exempt (identity is TLS-bound by
        // `handle_intake_verified` before delegation). An unsolicited "reply"
        // fails this gate and falls through to the unchanged accept seam below —
        // where it is denied and ruptured, exactly as an out-of-matrix read
        // should be. Dedup is idempotent per `request_id` (AC2b), NEVER the
        // resetting envelope `frame_id`.
        if Self::consent_match_key(frame).eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
            && matches!(
                self.digest_read_port.classify(frame),
                DigestFrameClass::Reply { .. }
            )
        {
            match self.digest_read_port.observe_reply(&peer_host, frame) {
                Ok(DigestReplyObservation::Accepted) => {
                    let new_clock = self.clock.recv_advance(frame.logical_clock);
                    // j1-crosshost-2b AC1.2 (G1b.i) — the SECOND intake-sink push
                    // site. A fix applied only to the delegation path at `(5)`
                    // below would leave this one still discarding the send result.
                    if self.push_to_intake_sink(frame).await.is_err() {
                        return A2AJsonRpcResponse::nack(
                            request.id,
                            CODE_INTERNAL,
                            "intake sink full or receiver dropped — digest reply NOT delivered",
                        );
                    }
                    return A2AJsonRpcResponse::ack(
                        request.id,
                        AckBody {
                            delivered: true,
                            receiver_logical_clock: new_clock,
                        },
                    );
                }
                Ok(DigestReplyObservation::Duplicate) => {
                    return A2AJsonRpcResponse::ack(
                        request.id,
                        AckBody {
                            delivered: true,
                            receiver_logical_clock: self.clock.recv_advance(frame.logical_clock),
                        },
                    );
                }
                Ok(DigestReplyObservation::Unauthorized) => {}
                Err(error) => {
                    return A2AJsonRpcResponse::nack(
                        request.id,
                        CODE_INTERNAL,
                        format!("digest reply audit/state transition failed: {error}"),
                    );
                }
            }
        }

        // (3) ADR-012 accept-allowlist check.
        if !self.accept_admits(&peer_cfg.allowlists, frame, peer_cfg.peer_id.as_str()) {
            // Story 8.13.1 / AC3 — EARN the rupture: emit a genuine typed
            // ConsentRupture frame on this classified-but-policy-denied (-32001)
            // leg, produced by the production deny path itself (NOT hand-inserted
            // by a smoke). Mirrors the same-host 6.4 `iac_bus` rupture-frame
            // semantics. Emitted BEFORE the NACK so an observer that drains the
            // rupture sink sees the record bound to the verified peer + denied
            // intent. The wire NACK below is byte-for-byte unchanged.
            if let Err(error) = self.emit_consent_rupture(frame).await {
                return A2AJsonRpcResponse::nack(
                    request.id,
                    CODE_INTERNAL,
                    format!("consent denied but rupture persistence failed: {error}"),
                );
            }
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

        let cohort_intent = Self::consent_match_key(frame);
        if !Self::is_reserved_cohort_intent(&cohort_intent) {
            let verdict = match precomputed_verdict {
                // Story 13.6a review P2 — the verdict handle_intake_verified
                // read from the SAME snapshot as the team declaration.
                Some(verdict) => verdict,
                None => self.cohort_consent_decision(
                    CohortConsentSeam::Accept,
                    &peer_host,
                    request.cohort_acting_role.as_deref(),
                    &cohort_intent,
                    request.cohort_manifest_version,
                ),
            };
            // Story 13.6a / AC4 — `Defer` is returned BEFORE the gate's
            // self-membership check and is otherwise a PASS, so a sender the
            // roster has legitimately dropped could push a crossing into a
            // healthy applier. On the crossing intent ONLY it becomes a refusal,
            // reusing the SHIPPED `Deny` path (rupture emission + typed data)
            // under its own attributable cause; every other intent keeps 12.1's
            // mixed-deployment bilateral fallback unchanged.
            let verdict = match verdict {
                CohortConsentVerdict::Defer
                    if cohort_intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE)
                        || cohort_intent
                            .eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT) =>
                {
                    CohortConsentVerdict::Deny(CohortConsentDenial::CrossingDeferRefused)
                }
                other => other,
            };
            match verdict {
                CohortConsentVerdict::Defer => {}
                CohortConsentVerdict::Admit | CohortConsentVerdict::AdmitOutbound { .. } => {}
                CohortConsentVerdict::NotCurrent => {
                    let reason = format!(
                        "cohort manifest is not current for peer {}",
                        peer_cfg.peer_id.as_str()
                    );
                    if (cohort_intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE)
                        || cohort_intent.eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT))
                        && Self::is_crossing_frame(frame)
                    {
                        let refusal = CrossingRefusal::ConsentStale {
                            reason,
                            from_team: request.cohort_source_team.clone().unwrap_or_default(),
                            to_team: Self::crossing_to_team(frame),
                            intent: cohort_intent,
                        };
                        return Self::crossing_refusal_nack(request.id, &refusal);
                    }
                    return A2AJsonRpcResponse::nack(request.id, CODE_INTERNAL, reason);
                }
                CohortConsentVerdict::Deny(reason) => {
                    let data = Self::cohort_denial_data(&reason);
                    if let Err(error) = self.emit_consent_rupture(frame).await {
                        return A2AJsonRpcResponse::nack(
                            request.id,
                            CODE_INTERNAL,
                            format!(
                                "cohort consent denied but rupture persistence failed: {error}"
                            ),
                        );
                    }
                    return A2AJsonRpcResponse::nack_with_data(
                        request.id,
                        CODE_INTENT_DENIED,
                        format!(
                            "cohort consent denied for peer {}: {reason}",
                            peer_cfg.peer_id.as_str()
                        ),
                        data,
                    );
                }
            }
        }
        if cohort_intent.eq_ignore_ascii_case(RESERVED_INTENT_REISSUE) {
            let verified_peer = HostId(peer_cfg.peer_id.as_str().to_string());
            return match self
                .cohort_manifest_gate
                .apply_reissue(&verified_peer, frame)
            {
                Ok(_) => A2AJsonRpcResponse::ack(
                    request.id,
                    AckBody {
                        delivered: true,
                        receiver_logical_clock: self.clock.recv_advance(frame.logical_clock),
                    },
                ),
                Err(rejection) => A2AJsonRpcResponse::nack_with_data(
                    request.id,
                    CODE_INTERNAL,
                    rejection.reason,
                    serde_json::json!({
                        "seen_version": rejection.seen_version,
                        "rejected_version": rejection.rejected_version,
                    }),
                ),
            };
        }
        if cohort_intent.eq_ignore_ascii_case(RESERVED_INTENT_HALT_RECEIPT) {
            return A2AJsonRpcResponse::ack(
                request.id,
                AckBody {
                    delivered: true,
                    receiver_logical_clock: self.clock.recv_advance(frame.logical_clock),
                },
            );
        }

        // (4) Lamport recv_advance.
        let new_clock = self.clock.recv_advance(frame.logical_clock);

        // (5) Hand the accepted frame to the receiving Host's intake consumer.
        // NOT a "test hook" — that comment was the same false claim as the one on
        // `install_intake_sink`, and it is why this line dropped every cross-host
        // delegation frame in production while ACKing `delivered: true` (G1).
        // A configured-but-dead consumer must NEVER be reported as delivered.
        if self.push_to_intake_sink(frame).await.is_err() {
            return A2AJsonRpcResponse::nack(
                request.id,
                CODE_INTERNAL,
                "intake sink full or receiver dropped — frame NOT delivered",
            );
        }

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
    /// `peer_leaf_fingerprint` is the TLS-negotiated leaf fingerprint of the
    /// client that presented `verified_peer` (Story 13.6a review P1): the team
    /// declaration is returned by the gate only when it equals the signed
    /// `CohortMember.fingerprint`, so a peer cannot speak a host's team on a
    /// certificate the signed manifest does not name — including a stale
    /// certificate after a fingerprint rotation by reissue.
    pub async fn handle_intake_verified(
        &self,
        request: A2AJsonRpcRequest,
        verified_peer: &PeerId,
        peer_leaf_fingerprint: Option<&PeerCertFingerprint>,
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
            let resp = A2AJsonRpcResponse::nack_with_data(
                request.id,
                CODE_PEER_IDENTITY_MISMATCH,
                format!(
                    "frame.from.host_id {} does not match TLS-verified peer {}",
                    asserted.unwrap_or("<none>"),
                    verified_peer.as_str()
                ),
                data,
            );
            return (resp, false);
        }

        // Story 13.6a / AC2 — the TEAM axis, bound at the SAME spoof-proof site
        // as the host axis above and reported under its OWN code so the two
        // never collapse. A frame that claims a source team is refused unless the
        // TLS-verified peer speaks for that team under the signed V4 manifest;
        // the crossing intent additionally REQUIRES a claim, so absence refuses.
        //
        // This is the binding a shared replication seed cannot forge: the claim
        // is checked against `CohortMember.team`, gated on the negotiated leaf
        // fingerprint EQUALING the signed `CohortMember.fingerprint` —
        // operator-pinned, cert-bound, not derived from `base_seed`.
        //
        // Review P2: the consent verdict is computed by the SAME gate call, from
        // the SAME locked manifest snapshot as the declaration, and threaded
        // into `handle_intake_inner` so the admission decision cannot race a hot
        // reissue that changes the host's team between the two checks.
        let claimed_team = request.cohort_source_team.as_deref();
        let team_intent = Self::consent_match_key(&request.params);
        let mut precomputed_verdict = None;
        if claimed_team.is_some()
            || team_intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE)
            || team_intent.eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT)
        {
            let peer = HostId(verified_peer.as_str().to_string());
            let (verdict, declared) = self.cohort_manifest_gate.consent_and_team(
                CohortConsentSeam::Accept,
                &peer,
                peer_leaf_fingerprint,
                request.cohort_acting_role.as_deref(),
                &team_intent,
                request.cohort_manifest_version,
            );
            if declared.is_none() || declared.as_deref() != claimed_team {
                let resp = A2AJsonRpcResponse::nack_with_data(
                    request.id,
                    CODE_TEAM_IDENTITY_MISMATCH,
                    format!(
                        "TLS-verified peer {} does not speak for claimed source team {}",
                        verified_peer.as_str(),
                        claimed_team.unwrap_or("<none>")
                    ),
                    serde_json::json!({
                        "peer": verified_peer.as_str(),
                        "claimed_team": claimed_team,
                        "declared_team": declared,
                    }),
                );
                return (resp, true);
            }
            precomputed_verdict = Some(verdict);
        }

        // Story 13.6b / AC1+AC3 — retain the crossing frame together with the
        // team the block above just AUTHENTICATED, so the applier decides
        // consent from a team the mesh PROVED rather than one the payload
        // asserts. `claimed_team` is `Some` here by construction: the crossing
        // intent forces the check above, and absence already refused.
        //
        // The apply itself runs only after `handle_intake_inner` ACKs, so the
        // Story 12.1/12.3 cohort gate — including `NotCurrent` for a host a
        // signed reissue no longer rosters (D-12) — refuses an evicted applier
        // before any bundle touches the store.
        let crossing_to_apply = ((team_intent
            .eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE)
            || team_intent.eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT))
            && Self::is_crossing_frame(&request.params))
        .then(|| claimed_team.map(|team| (team.to_string(), request.params.clone())))
        .flatten();

        // Story 12.3 (P5r/P8) — retain the TLS-bound member and the receipt
        // frame while the shared body applies framing and consent validation.
        // Presence becomes observable only after the reserved intent is ACKed:
        // a TLS-authenticated but malformed, expired, or wrong-granter frame is
        // rejected rather than being counted as a received halt receipt.
        let receipt_to_observe = Self::consent_match_key(&request.params)
            .eq_ignore_ascii_case(RESERVED_INTENT_HALT_RECEIPT)
            .then(|| {
                (
                    HostId(verified_peer.as_str().to_string()),
                    request.params.clone(),
                )
            });
        // Story 12.4a — retain an admitted digest-read REQUEST so, after the
        // accept-gate ACKs it (the single consent decision), the port records the
        // reply obligation + authorizes the future correlated reply. Identity is
        // TLS-bound here (P5r); a malformed/denied request never reaches the
        // ACK arm, so no unauthorized reply can be minted.
        let digest_request_to_admit = Self::consent_match_key(&request.params)
            .eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
            .then(|| self.digest_read_port.classify(&request.params))
            .and_then(|class| match class {
                DigestFrameClass::Request { request_id } => Some((
                    HostId(verified_peer.as_str().to_string()),
                    request_id,
                    request.params.clone(),
                )),
                _ => None,
            });
        let response_id = request.id;
        let mut response = self.handle_intake_inner(request, precomputed_verdict).await;
        if matches!(&response, A2AJsonRpcResponse::Ack(_)) {
            if let Some((member, frame)) = receipt_to_observe {
                self.halt_receipt_observer.observe_receipt(&member, &frame);
            }
            if let Some((requester, request_id, frame)) = digest_request_to_admit {
                if let Err(error) =
                    self.digest_read_port
                        .note_admitted_request(&requester, &request_id, &frame)
                {
                    response = A2AJsonRpcResponse::nack(
                        response_id,
                        CODE_INTERNAL,
                        format!("digest request audit/state transition failed: {error}"),
                    );
                }
            }
            if let Some((authenticated_team, frame)) = crossing_to_apply {
                match self
                    .crossing_port
                    .apply_crossing(&authenticated_team, &frame)
                    .await
                {
                    CrossingOutcome::Applied { .. } => {}
                    CrossingOutcome::Refused(refusal) => {
                        response = Self::crossing_refusal_nack(response_id, &refusal);
                    }
                    CrossingOutcome::NotCrossing => {
                        let refusal = CrossingRefusal::StateUnavailable {
                            reason: "cross-team crossing applier is not configured".to_string(),
                            from_team: authenticated_team,
                            to_team: Self::crossing_to_team(&frame),
                            intent: Self::consent_match_key(&frame).to_string(),
                        };
                        response = Self::crossing_refusal_nack(response_id, &refusal);
                    }
                }
            }
        }
        (response, true)
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
        A2AError::CohortConsentDenied { direction, reason } => {
            IacBusError::CrossHostRouteFailure(format!(
                "cohort consent denied ({direction:?}) for peer {peer}: {reason}"
            ))
        }
        // Story 13.6a — the team-identity refusal rides the SAME generic
        // route-failure port type (no new kernel variant; the 8.9/8.8 pattern).
        // Its `Display` already names the claimed and manifest-declared teams.
        error @ A2AError::CohortTeamIdentityRefused { .. } => {
            IacBusError::CrossHostRouteFailure(format!("{error} (peer {peer})"))
        }
        // Story 13.6b — both crossing refusals ride the SAME generic
        // route-failure port type. No new `IacBusError` variant, so
        // `maos-kernel-core` stays byte-identical (Trap 10's sibling on the
        // A2A axis) and the ZERO-Δ pin holds. Their `Display` impls already
        // name the envelope/payload pair and the ordered pair + intent, so the
        // three-way distinction survives into the message even though the port
        // type is shared.
        error @ (A2AError::CrossingSourceTeamUnbound { .. }
        | A2AError::CrossTeamCrossingRefused { .. }) => {
            IacBusError::CrossHostRouteFailure(format!("{error} (peer {peer})"))
        }
        // Story 8.8 — fail-closed unclassified-consent denials map to the generic
        // route-failure port type (no new kernel variant; `maos-kernel-core` stays
        // byte-identical — the 8.9 pattern). The reason + direction/peer are
        // preserved in the message for the audit trail.
        A2AError::ConsentUnclassified { direction, reason } => {
            IacBusError::CrossHostRouteFailure(format!(
                "cross-Host consent unclassified ({reason}) on {direction:?} — fail-closed deny for peer {peer}"
            ))
        }
        A2AError::ConsentUnclassifiedAtPeer { peer: denied_peer, reason } => {
            IacBusError::CrossHostRouteFailure(format!(
                "cross-Host consent unclassified ({reason}) at peer {denied_peer} — fail-closed deny"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::A2AProfile;
    use crate::identity::PeerCertFingerprint;
    use crate::identity::PeerId;
    use crate::tofu::InMemoryTofuPinStore;
    use crate::{CohortReissueDisposition, CohortReissueRejection};
    use maos_domain::frame::{
        FrameAddress, FramePayload, PosturePreferences, RuptureReason, TaskAssignPayload,
        TelemetryEventPayload,
    };
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i8::A2AIntent;
    use maos_spirit_abi::identity::{FrameKind, SpiritId};
    use smallvec::smallvec;

    #[derive(Default)]
    struct RecordingRuptureSink {
        frames: std::sync::Mutex<Vec<IacFrame>>,
    }

    impl ConsentRuptureSink for RecordingRuptureSink {
        fn append(&self, frame: &IacFrame) -> Result<(), String> {
            self.frames
                .lock()
                .map_err(|_| "recording rupture sink lock poisoned".to_string())?
                .push(frame.clone());
            Ok(())
        }
    }

    struct FixedCrossingGate(CohortConsentVerdict);

    impl CohortManifestGate for FixedCrossingGate {
        fn consent_decision(
            &self,
            _seam: CohortConsentSeam,
            _counterparty: &HostId,
            _acting_role: Option<&str>,
            _intent: &str,
            _sender_manifest_version: Option<u64>,
        ) -> CohortConsentVerdict {
            self.0.clone()
        }

        fn apply_reissue(
            &self,
            _verified_peer: &HostId,
            _frame: &IacFrame,
        ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
            Ok(CohortReissueDisposition::PullRequested)
        }

        fn consent_and_team(
            &self,
            _seam: CohortConsentSeam,
            _counterparty: &HostId,
            _endpoint_fingerprint: Option<&PeerCertFingerprint>,
            _acting_role: Option<&str>,
            _intent: &str,
            _sender_manifest_version: Option<u64>,
        ) -> (CohortConsentVerdict, Option<String>) {
            (self.0.clone(), Some("team-a".to_string()))
        }
    }

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
        let from = FrameAddress {
            spirit_id: SpiritId::from("a"),
            host_id: host_id.map(|s| HostId(s.to_string())),
            role: None,
        };
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: from.clone(),
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
            // Story 8.8 (Option 2) — fail-closed is unconditional, so the plumbing
            // tests use a CLASSIFIED frame (canonical "standard" intent, granter ==
            // from). Tests asserting unclassified-deny set `consent_envelope = None`
            // explicitly.
            consent_envelope: Some(maos_domain::frame::ConsentEnvelope {
                consent_id: [0u8; 16],
                granter: from,
                timestamp_ns: 0,
                intent_class: Some(A2AIntent::new("standard")),
                valid_until_ns: None,
            }),
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
        // Story 8.8 (Option 2) — fail-closed is unconditional; these plumbing tests
        // use classified frames (`make_frame` populates a canonical `intent_class`).
        // Fail-closed deny coverage lives in `tests/fail_closed_8_8.rs`.
        A2ARouterCore::new(vec![cfg], tofu)
    }

    fn crossing_request() -> A2AJsonRpcRequest {
        let from = FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("loopback".to_string())),
            role: None,
        };
        let frame = IacFrame {
            frame_id: [7u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: from.clone(),
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("cohort-control"),
                host_id: Some(HostId("receiver".to_string())),
                role: None,
            }],
            kind: FrameKind::TelemetryEvent,
            intent: IntentClass::Readonly,
            payload: FramePayload::TelemetryEvent(TelemetryEventPayload {
                event_type: crate::cohort::CROSSING_EVENT_TYPE.to_string(),
                data: serde_json::json!({
                    "kind": "share",
                    "to_team": "team-b",
                    "bundle": {}
                })
                .to_string(),
            }),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(
                maos_domain::frame::ConsentEnvelope::with_fine_grained_intent(
                    from,
                    A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE),
                ),
            ),
            intent_lineage: IntentLineage::default(),
        };
        A2AJsonRpcRequest::new("iac.deliver", frame, 17).with_cohort_source_team("team-a")
    }

    fn erase_crossing_frame() -> IacFrame {
        let mut frame = crossing_request().params;
        frame.intent = IntentClass::Standard;
        let FramePayload::TelemetryEvent(payload) = &mut frame.payload else {
            unreachable!("crossing fixture is telemetry");
        };
        payload.data = serde_json::json!({
            "kind": "erase",
            "to_team": "team-b",
            "spirit_pid": 7,
            "namespace": "default",
            "key": "crossed-key"
        })
        .to_string();
        frame
            .consent_envelope
            .as_mut()
            .expect("crossing fixture has a consent envelope")
            .intent_class = Some(A2AIntent::new(CROSS_TEAM_COLLECTIVE_ERASE_INTENT));
        frame
    }

    async fn crossing_core(verdict: CohortConsentVerdict) -> A2ARouterCore {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE)],
            accept_allowlist: vec![A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE)],
        };
        pinned_core(allow)
            .await
            .with_cohort_manifest_gate(Arc::new(FixedCrossingGate(verdict)))
    }

    #[tokio::test]
    async fn erase_crossing_is_denied_by_share_route_and_admitted_by_erase_route() {
        let peer = HostId("loopback".to_string());
        let share_only = crossing_core(CohortConsentVerdict::Admit).await;
        let error = share_only
            .prepare_outbound(erase_crossing_frame(), &peer, 0)
            .await
            .expect_err("a share-only route must reject an erase crossing");
        assert!(matches!(
            error,
            A2AError::IntentDenied {
                direction: IntentDirection::Send,
                ..
            }
        ));

        let erase_only = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(CROSS_TEAM_COLLECTIVE_ERASE_INTENT)],
            accept_allowlist: vec![A2AIntent::new(CROSS_TEAM_COLLECTIVE_ERASE_INTENT)],
        };
        let erase_route = pinned_core(erase_only)
            .await
            .with_cohort_manifest_gate(Arc::new(FixedCrossingGate(CohortConsentVerdict::Admit)));
        erase_route
            .prepare_outbound(erase_crossing_frame(), &peer, 0)
            .await
            .expect("an erase-only route must admit an erase crossing");
    }

    #[tokio::test]
    async fn verified_crossing_without_an_applier_fails_closed() {
        let core = crossing_core(CohortConsentVerdict::Admit).await;
        let fingerprint = PeerCertFingerprint::from_cert_der(b"x");
        let (response, bound) = core
            .handle_intake_verified(
                crossing_request(),
                &PeerId::new("loopback"),
                Some(&fingerprint),
            )
            .await;
        assert!(bound);
        match response {
            A2AJsonRpcResponse::Nack(nack) => {
                assert_eq!(nack.error.code, CODE_CROSS_TEAM_CROSSING_REFUSED);
                let data = nack.error.data.expect("typed crossing data");
                assert_eq!(data["reason"], "crossing_state_unavailable");
                assert_eq!(data["from_team"], "team-a");
                assert_eq!(data["to_team"], "team-b");
            }
            other => panic!("an unavailable applier must NACK, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stale_crossing_gate_keeps_the_typed_stale_outcome() {
        let core = crossing_core(CohortConsentVerdict::NotCurrent).await;
        let fingerprint = PeerCertFingerprint::from_cert_der(b"x");
        let (response, bound) = core
            .handle_intake_verified(
                crossing_request(),
                &PeerId::new("loopback"),
                Some(&fingerprint),
            )
            .await;
        assert!(bound);
        match response {
            A2AJsonRpcResponse::Nack(nack) => {
                assert_eq!(nack.error.code, CODE_CROSS_TEAM_CROSSING_REFUSED);
                let data = nack.error.data.expect("typed crossing data");
                assert_eq!(data["reason"], "crossing_consent_stale");
                assert_eq!(data["from_team"], "team-a");
                assert_eq!(data["to_team"], "team-b");
            }
            other => panic!("a stale crossing must retain its typed outcome, got {other:?}"),
        }
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
        let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
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
    async fn intake_policy_deny_emits_consent_rupture() {
        // Story 8.13.1 / AC3 + AC5 (red-first): the classified-but-policy-denied
        // (-32001 CODE_INTENT_DENIED) intake leg MUST push a production-produced
        // typed `ConsentRupture` frame to the rupture sink, BEFORE the NACK.
        // Before the emission wiring this assertion FAILS (the channel stays
        // empty) — proving the row was never earned and the smokes faked it.
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![], // denies the classified "standard" intent
        };
        let core = pinned_core(allow).await;
        let sink = Arc::new(RecordingRuptureSink::default());
        core.install_rupture_sink(sink.clone()).await;
        let frame = make_frame(Some("loopback"));
        let original_id = frame.frame_id;
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = core.handle_intake(req).await;
        // The wire result is UNCHANGED: still a -32001 NACK (no protocol break).
        match resp {
            A2AJsonRpcResponse::Nack(n) => assert_eq!(n.error.code, CODE_INTENT_DENIED),
            _ => panic!("expected Nack"),
        }
        // ...but the deny path now emitted a genuine typed ConsentRupture frame —
        // the record the smokes used to hand-insert. Information only the deny
        // decision possesses (the reason) is carried, so it cannot be re-faked
        // from the bare error code (Murat's information-content bar).
        let rupture = sink
            .frames
            .lock()
            .expect("recording rupture sink lock")
            .first()
            .cloned()
            .expect("classified-policy-deny path must emit a ConsentRupture frame");
        assert_eq!(rupture.kind, FrameKind::ConsentRupture);
        // Bound to the verified peer (sender A) + the denied intent class.
        assert_eq!(rupture.to[0].spirit_id.as_str(), "a");
        assert_eq!(rupture.intent, IntentClass::Standard);
        match &rupture.payload {
            FramePayload::ConsentRupture(p) => {
                assert_eq!(p.original_frame_id, original_id);
                assert_eq!(p.original_kind, FrameKind::TaskAssign);
                assert_eq!(p.rejected.len(), 1);
                assert!(matches!(
                    p.rejected[0].reason,
                    RuptureReason::IntentAllowlistMismatch
                ));
            }
            other => panic!("expected ConsentRupture payload, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn intake_accept_emits_no_rupture() {
        // Story 8.13.1 negative control (Murat's standing guardrail): the ACCEPT
        // path must NEVER emit a rupture, so the emission can never degrade into
        // an "always fire" self-fulfilling row.
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let sink = Arc::new(RecordingRuptureSink::default());
        core.install_rupture_sink(sink.clone()).await;
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = core.handle_intake(req).await;
        assert!(matches!(resp, A2AJsonRpcResponse::Ack(_)));
        assert!(
            sink.frames
                .lock()
                .expect("recording rupture sink lock")
                .is_empty(),
            "ACCEPT path must not emit a ConsentRupture"
        );
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

    /// Story 8.8 (Option 2) — supersedes the 8.9 `prepare_outbound_leaves_none_
    /// envelope_unchanged` passthrough test: under unconditional fail-closed, a
    /// frame with NO consent envelope is DENIED at the send seam (it never reaches
    /// the envelope-stamp step), so it cannot leave the Host. The 8.9 concern (no
    /// accidental envelope insertion) is moot — None is rejected, not forwarded.
    #[tokio::test]
    async fn prepare_outbound_denies_none_envelope_fail_closed() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let core = pinned_core(allow).await;
        let mut frame = make_frame(Some("loopback"));
        frame.consent_envelope = None;
        let err = core
            .prepare_outbound(frame, &HostId("loopback".to_string()), 0)
            .await
            .expect_err("fail-closed must deny a None-envelope cross-Host frame at the sender");
        assert!(matches!(
            err,
            A2AError::ConsentUnclassified {
                direction: IntentDirection::Send,
                reason: UnclassifiedReason::Absent,
            }
        ));
    }
}
