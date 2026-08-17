//! `TcpA2ATransport` — the live cross-Host `A2ATransport` impl (Story 8.6).
//!
//! A real TCP listener/dialer with operator-managed mTLS (TOFU-pinning cert
//! verification via [`crate::verifier::TofuPinningVerifier`]), length-delimited
//! JSON-RPC framing over the socket (4-byte BE `u32` length prefix, 1 MiB cap),
//! handshake retry on cert-class failures, and bounded intake/idle timeouts that
//! abort the per-connection task rather than racing a dangling future (AC-T7 —
//! the gap Story 8.5 deferred twice).
//!
//! All validation reuses `maos_a2a_core::router::A2ARouterCore` byte-for-byte
//! (AC-A6): `prepare_outbound` (allowlist + TOFU + clock tick), `handle_intake`
//! (receiver-side checks), `interpret_response`. This crate adds ONLY the wire.

use crate::config::{clone_key, TcpA2AConfig};
use crate::error::TcpTransportError;
use crate::verifier::{TofuPinningVerifier, TrustPosture, VerifyDirection};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::{A2APeerRouter, A2ARouterCore, A2ATransport};
use maos_a2a_core::transport::json_rpc::{CODE_FRAME_TOO_LARGE, CODE_TIMEOUT};
use maos_a2a_core::{
    A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, A2APeerConfig, CohortManifestGate,
    ConsentRuptureSink, DigestReadPort, HaltReceiptObserver, HandshakeRetryPolicy,
    InMemoryTofuPinStore, TofuPinStore,
};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, ServerConfig};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// 1 MiB length-delimited frame cap (AC-A4 / AC-T8).
pub const MAX_FRAME_LEN: usize = 1024 * 1024;

/// Build the length-delimited codec: 4-byte big-endian `u32` length prefix,
/// explicit 1 MiB `max_frame_length`. This is the ONLY message-boundary
/// mechanism (AC-A4 — no newline/EOF fallback exists anywhere).
pub fn length_delimited_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_length(4)
        .max_frame_length(MAX_FRAME_LEN)
        .new_codec()
}

/// Injectable timeouts (H5). `test_profile()` keeps timeout-path tests `< 2s`.
#[derive(Debug, Clone, Copy)]
pub struct TcpTimeouts {
    pub handshake: Duration,
    /// Max wall to receive one complete inbound frame (slow-loris bound, AC-T7).
    pub intake: Duration,
    /// Max idle wall between frames on an established connection.
    pub idle: Duration,
}

impl TcpTimeouts {
    /// Production defaults (handshake default 30s per AC-A5).
    pub fn production(handshake: Duration) -> Self {
        Self {
            handshake,
            intake: Duration::from_secs(30),
            idle: Duration::from_secs(60),
        }
    }

    /// Test profile — all ≤ 250ms so timeout-path tests complete `< 2s` (H5).
    pub fn test_profile() -> Self {
        Self {
            handshake: Duration::from_millis(250),
            intake: Duration::from_millis(250),
            idle: Duration::from_millis(250),
        }
    }
}

/// Drop guard (H6) — aborts the accept loop AND every per-connection task when
/// the transport is dropped, so the bound port is promptly re-bindable.
struct ServeGuard {
    accept: JoinHandle<()>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Drop for ServeGuard {
    fn drop(&mut self) {
        self.accept.abort();
        if let Ok(mut conns) = self.conns.lock() {
            for h in conns.drain(..) {
                h.abort();
            }
        }
    }
}

/// Live cross-Host TCP/mTLS transport endpoint (serves + dials).
pub struct TcpA2ATransport {
    core: Arc<A2ARouterCore>,
    pins: Arc<InMemoryTofuPinStore>,
    /// Shared (unscoped) dialing config — kept for the raw-socket test helpers
    /// (AC-T2/7/8/9) and as the `client_config()` accessor. Real
    /// `route_outbound` dials build a PER-PEER scoped config instead (review
    /// patch P1) via [`Self::scoped_client_config`].
    client_config: Arc<ClientConfig>,
    /// Materials to rebuild a per-dial `ClientConfig` whose `ServerCertVerifier`
    /// is scoped to the EXPECTED peer (review patch P1) — the dial target is only
    /// known per `route_outbound` call, so the verifier cannot be fixed at bind.
    own_chain: Vec<CertificateDer<'static>>,
    own_key: PrivateKeyDer<'static>,
    posture: TrustPosture,
    validation_time: Option<UnixTime>,
    local_addr: SocketAddr,
    own_boot_nonce: u64,
    timeouts: TcpTimeouts,
    retry_policy: HandshakeRetryPolicy,
    intake_entered: Arc<AtomicUsize>,
    active_connections: Arc<AtomicUsize>,
    last_dial_attempts: Arc<AtomicUsize>,
    /// Server-side observation of the LAST decoded inbound request's
    /// `(boot_nonce, lamport)` — `route_outbound` discards the ACK, so AC-T1's
    /// wire round-trip oracles read this instead.
    last_intake: Arc<Mutex<Option<(u64, u64)>>>,
    _serve_guard: Arc<ServeGuard>,
}

impl TcpA2ATransport {
    /// Bind the listener, build both TLS configs, and spawn the accept loop.
    ///
    /// `peer_configs` carry the ADR-012 allowlists + dial endpoints (from the
    /// existing `A2AConfig`); `tcp_config` carries the certs/pins/listen address.
    /// Both share ONE `InMemoryTofuPinStore` (the engine's TOFU source and the
    /// verifier's sync pin source are the same `Arc`).
    pub async fn bind(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
    ) -> Result<Self, TcpTransportError> {
        Self::bind_with_cohort_manifest_gate(
            tcp_config,
            peer_configs,
            own_boot_nonce,
            timeouts,
            retry_policy,
            validation_time,
            consent_now_ns,
            None,
        )
        .await
    }

    /// Cohort-enabled construction path (gate only). Delegates to
    /// [`Self::bind_with_cohort_wiring`] with no halt-receipt observer — the
    /// existing callers stay byte-for-byte unchanged (Story 12.3 avoids the
    /// 6-caller `bind` churn P7a flagged by adding a NEW wiring fn instead).
    #[allow(clippy::too_many_arguments)]
    pub async fn bind_with_cohort_manifest_gate(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
        cohort_manifest_gate: Option<Arc<dyn CohortManifestGate>>,
    ) -> Result<Self, TcpTransportError> {
        Self::bind_with_cohort_wiring(
            tcp_config,
            peer_configs,
            own_boot_nonce,
            timeouts,
            retry_policy,
            validation_time,
            consent_now_ns,
            cohort_manifest_gate,
            None,
        )
        .await
    }

    /// Story 12.3 — cohort-enabled construction path wiring BOTH the manifest
    /// gate and the halt-receipt presence observer. Delegates to
    /// [`Self::bind_with_cohort_wiring_and_digest`] with no digest-read port —
    /// existing callers stay byte-for-byte unchanged (Story 12.4a avoids caller
    /// churn by adding a NEW wiring fn, mirroring 12.3's P7a discipline).
    #[allow(clippy::too_many_arguments)]
    pub async fn bind_with_cohort_wiring(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
        cohort_manifest_gate: Option<Arc<dyn CohortManifestGate>>,
        halt_receipt_observer: Option<Arc<dyn HaltReceiptObserver>>,
    ) -> Result<Self, TcpTransportError> {
        Self::bind_with_cohort_wiring_and_digest(
            tcp_config,
            peer_configs,
            own_boot_nonce,
            timeouts,
            retry_policy,
            validation_time,
            consent_now_ns,
            cohort_manifest_gate,
            halt_receipt_observer,
            None,
            None,
        )
        .await
    }

    /// Story 12.4a — cohort-enabled construction path wiring the manifest gate,
    /// the halt-receipt observer, AND the digest-read correlation port.
    /// Delegates to [`Self::bind_with_cohort_wiring_and_crossing`] with no
    /// cross-team crossing applier — existing callers stay byte-for-byte
    /// unchanged (Story 13.6b keeps 12.4a's P7a no-caller-churn discipline).
    #[allow(clippy::too_many_arguments)]
    pub async fn bind_with_cohort_wiring_and_digest(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
        cohort_manifest_gate: Option<Arc<dyn CohortManifestGate>>,
        halt_receipt_observer: Option<Arc<dyn HaltReceiptObserver>>,
        digest_read_port: Option<Arc<dyn DigestReadPort>>,
        rupture_sink: Option<Arc<dyn ConsentRuptureSink>>,
    ) -> Result<Self, TcpTransportError> {
        Self::bind_with_cohort_wiring_and_crossing(
            tcp_config,
            peer_configs,
            own_boot_nonce,
            timeouts,
            retry_policy,
            validation_time,
            consent_now_ns,
            cohort_manifest_gate,
            halt_receipt_observer,
            digest_read_port,
            rupture_sink,
            None,
        )
        .await
    }

    /// Story 13.6b — cohort-enabled construction path that ALSO wires the
    /// cross-team crossing applier. Every port is installed via the named
    /// `A2ARouterCore` builders (no adjacent-`Arc` positional transposition
    /// footgun, P7b) BEFORE the core is wrapped and the accept loop spawns, so
    /// no inbound connection observes a legacy policy / observation /
    /// correlation / applier window (P7c). In production the gate, observer,
    /// and digest port are the SAME `CohortManifestState` (`state.clone()`);
    /// the crossing applier is the composition root's store-owning adapter.
    #[allow(clippy::too_many_arguments)]
    pub async fn bind_with_cohort_wiring_and_crossing(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
        cohort_manifest_gate: Option<Arc<dyn CohortManifestGate>>,
        halt_receipt_observer: Option<Arc<dyn HaltReceiptObserver>>,
        digest_read_port: Option<Arc<dyn DigestReadPort>>,
        rupture_sink: Option<Arc<dyn ConsentRuptureSink>>,
        crossing_port: Option<Arc<dyn maos_a2a_core::CrossTeamCrossingPort>>,
    ) -> Result<Self, TcpTransportError> {
        Self::bind_with_intake_sink(
            tcp_config,
            peer_configs,
            own_boot_nonce,
            timeouts,
            retry_policy,
            validation_time,
            consent_now_ns,
            cohort_manifest_gate,
            halt_receipt_observer,
            digest_read_port,
            rupture_sink,
            crossing_port,
            None,
        )
        .await
    }

    /// j1-crosshost-2b AC1.2 — the SIXTH optional seam, and the one that makes a
    /// receiving Host act on a frame instead of acknowledging and discarding it.
    ///
    /// Before this existed, **zero** `install_intake_sink` calls lived anywhere in
    /// `crates/maos-a2a-tcp/src/`. A real daemon authenticated the peer, bound the
    /// wire identity, ran TOFU, checked the boot nonce, evaluated consent, advanced
    /// the Lamport clock and ACKed `delivered: true` — then dropped the frame,
    /// because `A2ARouterCore::intake_sink` was `None` and the doc on
    /// `install_intake_sink` called it a "test-only hook" (G1). The whole receiving
    /// mechanism was one missing call and a consumer behind it.
    ///
    /// The sink is installed **beside `install_rupture_sink`, before
    /// `TcpListener::bind`** — deliberately, not incidentally. `install_intake_sink`
    /// is already filed as racy against in-flight frames
    /// (`deferred-work.md:309-314`); installing before the listener exists is the
    /// ordering under which no inbound connection can observe a sink-less router
    /// (Trap 17, and the same P7c discipline 13.6b applied to its ports).
    #[allow(clippy::too_many_arguments)]
    pub async fn bind_with_intake_sink(
        tcp_config: TcpA2AConfig,
        peer_configs: Vec<A2APeerConfig>,
        own_boot_nonce: u64,
        timeouts: TcpTimeouts,
        retry_policy: HandshakeRetryPolicy,
        validation_time: Option<UnixTime>,
        consent_now_ns: Option<u64>,
        cohort_manifest_gate: Option<Arc<dyn CohortManifestGate>>,
        halt_receipt_observer: Option<Arc<dyn HaltReceiptObserver>>,
        digest_read_port: Option<Arc<dyn DigestReadPort>>,
        rupture_sink: Option<Arc<dyn ConsentRuptureSink>>,
        crossing_port: Option<Arc<dyn maos_a2a_core::CrossTeamCrossingPort>>,
        intake_sink: Option<tokio::sync::mpsc::Sender<IacFrame>>,
    ) -> Result<Self, TcpTransportError> {
        // j1-crosshost-2b §A6 review P8 (AC4.1) — the cross-host path REFUSES the
        // `boot_nonce = 0` sentinel. Loopback stamps it to skip restart detection
        // (`crates/maos-a2a/src/adapter.rs`); carrying that habit onto TCP would
        // ship the first cross-host wire with `router.rs`'s
        // `if request.boot_nonce != 0` restart check structurally dead — the exact
        // escape AC4.1 forbids. Checked FIRST, before any identity work, so the
        // refusal is reachable by a unit test with no certs.
        if own_boot_nonce == 0 {
            return Err(TcpTransportError::Config(
                "cross-host transport refuses boot_nonce = 0 — the loopback sentinel \
                 would disable NFR-Rel-6 restart detection on a live wire"
                    .to_string(),
            ));
        }
        for pin in &tcp_config.peer_pins {
            if pin.boot_nonce == 0 {
                return Err(TcpTransportError::Config(format!(
                    "peer pin '{}' carries boot_nonce = 0 — the loopback sentinel is \
                     not a valid cross-host pin; NFR-Rel-6 restart detection would be dead",
                    pin.peer_id
                )));
            }
        }
        let pins = tcp_config.build_pin_store().await?;
        let posture = tcp_config.trust_posture()?;
        let (own_chain, own_key) = tcp_config.load_identity()?;

        // Story 8.9 / AC6.2 (G5a) — `try_new` HARD-FAILS on a duplicate `peer_id`
        // instead of the prior silent "last wins" overwrite; surface it to the
        // operator as a config error.
        for cfg in &peer_configs {
            cfg.validate()
                .map_err(|e| TcpTransportError::Config(e.to_string()))?;
        }
        let mut core_inner =
            A2ARouterCore::try_new(peer_configs, pins.clone() as Arc<dyn TofuPinStore>)
                .map_err(|e| TcpTransportError::Config(e.to_string()))?;
        // Story 13.6a (review P1) — this endpoint's own leaf fingerprint, from
        // the SAME identity chain the server/client configs below present on
        // the wire. The Send-seam team declaration is gated on it equalling the
        // local host's signed `CohortMember.fingerprint`.
        if let Some(own_leaf) = own_chain.first() {
            core_inner = core_inner
                .with_local_leaf_fingerprint(PeerCertFingerprint::from_cert_der(own_leaf.as_ref()));
        }
        // Story 8.8 — the live wire is genuine cross-Host → fail-closed is
        // unconditional in `A2ARouterCore` (Option 2, no toggle). Unclassified
        // frames are denied with CODE_CONSENT_UNCLASSIFIED (-32009).
        if let Some(t) = consent_now_ns {
            core_inner = core_inner.with_pinned_consent_clock(t);
        }
        if let Some(gate) = cohort_manifest_gate {
            core_inner = core_inner.with_cohort_manifest_gate(gate);
        }
        if let Some(observer) = halt_receipt_observer {
            core_inner = core_inner.with_halt_receipt_observer(observer);
        }
        if let Some(port) = digest_read_port {
            core_inner = core_inner.with_digest_read_port(port);
        }
        if let Some(port) = crossing_port {
            core_inner = core_inner.with_cross_team_crossing_port(port);
        }
        let core = Arc::new(core_inner);
        if let Some(sink) = rupture_sink {
            core.install_rupture_sink(sink).await;
        }
        // j1-crosshost-2b AC1.2 — install BEFORE `TcpListener::bind` below, for the
        // same reason `install_rupture_sink` is here: no inbound connection may ever
        // observe a sink-less router (Trap 17 / the P7c window).
        if let Some(sink) = intake_sink {
            core.install_intake_sink(sink).await;
        }

        let server_config = Arc::new(build_server_config(
            &own_chain,
            &own_key,
            pins.clone(),
            posture.clone(),
            validation_time,
        )?);
        let client_config = Arc::new(build_client_config(
            &own_chain,
            &own_key,
            pins.clone(),
            posture.clone(),
            None, // shared/unscoped — real dials scope per peer (patch P1)
            validation_time,
        )?);

        let listener = TcpListener::bind(tcp_config.listen_addr)
            .await
            .map_err(|e| TcpTransportError::Io(format!("bind {}: {e}", tcp_config.listen_addr)))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| TcpTransportError::Io(format!("local_addr: {e}")))?;

        let intake_entered = Arc::new(AtomicUsize::new(0));
        let active_connections = Arc::new(AtomicUsize::new(0));
        let last_intake: Arc<Mutex<Option<(u64, u64)>>> = Arc::new(Mutex::new(None));
        let conns: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));

        let accept = tokio::spawn(accept_loop(
            listener,
            TlsAcceptor::from(server_config.clone()),
            core.clone(),
            pins.clone(),
            timeouts,
            intake_entered.clone(),
            active_connections.clone(),
            last_intake.clone(),
            conns.clone(),
        ));

        Ok(Self {
            core,
            pins,
            client_config,
            own_chain,
            own_key,
            posture,
            validation_time,
            local_addr,
            own_boot_nonce,
            timeouts,
            retry_policy,
            intake_entered,
            active_connections,
            last_dial_attempts: Arc::new(AtomicUsize::new(0)),
            last_intake,
            _serve_guard: Arc::new(ServeGuard { accept, conns }),
        })
    }

    /// The shared engine. Tests drive intake through it directly, and the J1
    /// composition root reads it to reach the router's installed seams.
    ///
    /// **This doc said "for tests that drive intake directly" until
    /// j1-crosshost-2b** (G14.ii) — the same false test-only framing as the one on
    /// `A2ARouterCore::install_intake_sink`. A production intake consumer is wired
    /// through [`Self::bind_with_intake_sink`], which is the supported path;
    /// `core()` remains the general accessor and is not test-scoped.
    pub fn core(&self) -> Arc<A2ARouterCore> {
        self.core.clone()
    }

    /// Patch a peer's dial endpoint after bind (AC-T11 ephemeral-port mesh: bind
    /// all listeners `:0` first, then wire in each readback `SocketAddr`).
    pub fn set_peer_endpoint(&self, host_id: &HostId, endpoint: impl Into<String>) {
        self.core.set_peer_endpoint(host_id, endpoint);
    }

    /// The shared TOFU pin store (AC-T6/T11 post-conditions read it).
    pub fn pins(&self) -> Arc<InMemoryTofuPinStore> {
        self.pins.clone()
    }

    /// A client `ClientConfig` carrying this endpoint's own auth cert + the
    /// TOFU verifier — tests use it to craft raw mTLS connections (AC-T2/7/8/9).
    pub fn client_config(&self) -> Arc<ClientConfig> {
        self.client_config.clone()
    }

    /// Count of frames that entered intake processing (AC-T3/T4 oracle == 0).
    pub fn intake_entered(&self) -> usize {
        self.intake_entered.load(Ordering::SeqCst)
    }

    /// Live per-connection gauge (AC-T7/T10 return-to-baseline oracle).
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Number of dial attempts on the LAST `route_outbound` (AC-T5 retry oracle).
    pub fn last_dial_attempts(&self) -> usize {
        self.last_dial_attempts.load(Ordering::SeqCst)
    }

    /// The LAST decoded inbound request's `(boot_nonce, lamport)` (AC-T1).
    pub fn last_intake_observed(&self) -> Option<(u64, u64)> {
        self.last_intake.lock().ok().and_then(|g| *g)
    }

    /// Resolve a peer's dial `SocketAddr` from its `tls://host:port` endpoint.
    fn dial_addr(&self, cfg: &A2APeerConfig) -> Result<SocketAddr, TcpTransportError> {
        let rest = cfg.endpoint.strip_prefix("tls://").ok_or_else(|| {
            TcpTransportError::Config(format!("endpoint must be tls://: {}", cfg.endpoint))
        })?;
        rest.parse::<SocketAddr>()
            .map_err(|e| TcpTransportError::Config(format!("bad endpoint addr '{rest}': {e}")))
    }

    /// Build a dialing `ClientConfig` whose `ServerCertVerifier` is scoped to the
    /// EXPECTED peer (review patch P1), so a leaf pinned for a different peer
    /// cannot satisfy this dial. Built once per `route_outbound` call.
    fn scoped_client_config(
        &self,
        expected: &PeerId,
    ) -> Result<Arc<ClientConfig>, TcpTransportError> {
        Ok(Arc::new(build_client_config(
            &self.own_chain,
            &self.own_key,
            self.pins.clone(),
            self.posture.clone(),
            Some(expected.clone()),
            self.validation_time,
        )?))
    }

    /// One dial+handshake+send+recv round, returning the typed transport result
    /// or a classified error. `client_config` is the per-peer scoped dialing
    /// config (patch P1).
    async fn dial_once(
        &self,
        addr: SocketAddr,
        request: &A2AJsonRpcRequest,
        client_config: &Arc<ClientConfig>,
    ) -> Result<A2AJsonRpcResponse, TcpTransportError> {
        let connector = TlsConnector::from(client_config.clone());
        let tcp = TcpStream::connect(addr)
            .await
            .map_err(|e| TcpTransportError::Io(format!("connect {addr}: {e}")))?;
        let server_name = ServerName::IpAddress(addr.ip().into());

        let tls = match tokio::time::timeout(
            self.timeouts.handshake,
            connector.connect(server_name, tcp),
        )
        .await
        {
            Err(_) => return Err(TcpTransportError::Timeout("client handshake".into())),
            Ok(Err(e)) => return Err(TcpTransportError::classify_handshake(&e.to_string())),
            Ok(Ok(s)) => s,
        };

        let mut framed = Framed::new(tls, length_delimited_codec());
        let body = serde_json::to_vec(request)
            .map_err(|e| TcpTransportError::Protocol(format!("serialize request: {e}")))?;
        framed
            .send(Bytes::from(body))
            .await
            .map_err(|e| TcpTransportError::Io(format!("send: {e}")))?;

        match tokio::time::timeout(self.timeouts.idle, framed.next()).await {
            Err(_) => Err(TcpTransportError::Timeout("awaiting response".into())),
            Ok(None) => Err(TcpTransportError::Io(
                "connection closed before response".into(),
            )),
            Ok(Some(Err(e))) => Err(TcpTransportError::Io(format!("recv: {e}"))),
            Ok(Some(Ok(buf))) => serde_json::from_slice::<A2AJsonRpcResponse>(&buf)
                .map_err(|e| TcpTransportError::Protocol(format!("deserialize response: {e}"))),
        }
    }
}

/// The accept loop — one per-connection `tokio::spawn` with its `JoinHandle`
/// held in the drop-guard registry (H6).
#[allow(clippy::too_many_arguments)]
async fn accept_loop(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    core: Arc<A2ARouterCore>,
    pins: Arc<InMemoryTofuPinStore>,
    timeouts: TcpTimeouts,
    intake_entered: Arc<AtomicUsize>,
    active_connections: Arc<AtomicUsize>,
    last_intake: Arc<Mutex<Option<(u64, u64)>>>,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
) {
    loop {
        let (tcp, _peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => {
                // Keep the loop alive on a transient accept error — but back off
                // briefly so a PERSISTENT error (e.g. EMFILE/ENFILE fd
                // exhaustion) cannot spin this loop hot at 100% CPU (review
                // patch P4). A transient error still recovers within 50ms.
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let core = core.clone();
        let pins = pins.clone();
        let intake_entered = intake_entered.clone();
        let active_connections = active_connections.clone();
        let last_intake = last_intake.clone();

        let handle = tokio::spawn(serve_connection(
            tcp,
            acceptor,
            core,
            pins,
            timeouts,
            intake_entered,
            active_connections,
            last_intake,
        ));
        if let Ok(mut guard) = conns.lock() {
            guard.retain(|h| !h.is_finished());
            guard.push(handle);
        }
    }
}

/// Decrement-on-drop guard for the active-connection gauge (H6).
struct ConnGauge(Arc<AtomicUsize>);
impl Drop for ConnGauge {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One connection: TLS accept (runs the client-cert pin verifier), then the
/// length-delimited read loop, each read bounded by the intake/idle timeout so a
/// stalling peer cannot hang the task (AC-T7).
async fn serve_connection(
    tcp: TcpStream,
    acceptor: TlsAcceptor,
    core: Arc<A2ARouterCore>,
    pins: Arc<InMemoryTofuPinStore>,
    timeouts: TcpTimeouts,
    intake_entered: Arc<AtomicUsize>,
    active_connections: Arc<AtomicUsize>,
    last_intake: Arc<Mutex<Option<(u64, u64)>>>,
) {
    active_connections.fetch_add(1, Ordering::SeqCst);
    let _gauge = ConnGauge(active_connections);

    // Handshake — bounded; a plaintext client (AC-T9) or a half-open client
    // (AC-T10) fails here and the task ends without ever entering intake.
    let tls = match tokio::time::timeout(timeouts.handshake, acceptor.accept(tcp)).await {
        Ok(Ok(s)) => s,
        _ => return,
    };

    // Story 8.9 / AC1 (G8) — re-derive the TLS-VERIFIED peer identity from the
    // negotiated client leaf (the handshake already ran the TOFU pin verifier;
    // we resolve which peer that pinned leaf belongs to via the SAME oracle
    // `verifier.rs` used). intake binds to THIS, never to `frame.from.host_id`.
    // If no verified peer resolves, close without ever entering intake.
    // Story 13.6a (review P1) — the negotiated leaf fingerprint rides along:
    // the gate returns the peer's team declaration only when this equals the
    // signed `CohortMember.fingerprint`.
    let (verified_peer, peer_leaf_fingerprint) = match resolve_verified_peer(&tls, &pins) {
        Some(resolved) => resolved,
        None => {
            tracing::warn!("resolve_verified_peer: no active pin for negotiated client cert — closing connection without intake");
            return;
        }
    };

    let mut framed = Framed::new(tls, length_delimited_codec());

    loop {
        match tokio::time::timeout(timeouts.intake, framed.next()).await {
            // Intake/idle timeout — abort this task (do NOT hang). Best-effort
            // CODE_TIMEOUT NACK, then end.
            Err(_) => {
                let nack = A2AJsonRpcResponse::nack(0, CODE_TIMEOUT, "intake timeout");
                let _ = send_response(&mut framed, &nack).await;
                return;
            }
            Ok(None) => return, // clean EOF
            Ok(Some(Err(e))) => {
                // Codec error: oversized frame is rejected after only the header
                // (no buffer blow-up) — surface CODE_FRAME_TOO_LARGE best-effort.
                if is_frame_too_large(&e) {
                    let nack = A2AJsonRpcResponse::nack(
                        0,
                        CODE_FRAME_TOO_LARGE,
                        "frame exceeds 1 MiB cap",
                    );
                    let _ = send_response(&mut framed, &nack).await;
                }
                return;
            }
            Ok(Some(Ok(buf))) => {
                // Story 8.9 / AC6.4 (G6) — bound PROCESSING + the NACK WRITE under
                // the idle timeout, not just the read above, so a slow processor
                // or a stalled write cannot hang the per-connection task.
                // Story 8.9 / AC6.4 (G6) — bound PROCESSING + the NACK WRITE under
                // the intake timeout (not just the read above), so a slow processor
                // or a stalled write cannot hang the per-connection task.
                let handled = tokio::time::timeout(timeouts.intake, async {
                    // Funnel raw bytes through the frozen `try_from_bytes` (AC-A4
                    // / security boundary 5) — malformed JSON → CODE_PARSE_ERROR.
                    let (resp, _binding_passed) = match A2AJsonRpcRequest::try_from_bytes(&buf) {
                        Ok(req) => {
                            let observed = (req.boot_nonce, req.params.logical_clock);
                            // AC1: bind to the TLS-verified peer in the shared core.
                            let (resp, binding_passed) = core
                                .handle_intake_verified(
                                    req,
                                    &verified_peer,
                                    Some(&peer_leaf_fingerprint),
                                )
                                .await;
                            // AC1.2: count + observe ONLY a frame whose verified-peer
                            // binding passed (a forged `from` → `intake_entered`
                            // stays 0, `last_intake` is NOT recorded).
                            if binding_passed {
                                if let Ok(mut g) = last_intake.lock() {
                                    *g = Some(observed);
                                }
                                intake_entered.fetch_add(1, Ordering::SeqCst);
                            }
                            (resp, binding_passed)
                        }
                        Err(nack) => (A2AJsonRpcResponse::Nack(nack), true),
                    };
                    send_response(&mut framed, &resp).await
                })
                .await;
                match handled {
                    // Wrote the response — a follow-up valid frame on the SAME
                    // connection is served (AC-T2 codec resync / not poisoned).
                    Ok(Ok(())) => {}
                    // Processing/write timed out, or the write errored — end task.
                    _ => return,
                }
            }
        }
    }
}

/// Story 8.9 / AC1 (G8) — resolve the TLS-verified peer from the listening
/// side's negotiated client certificate. After `acceptor.accept` succeeds the
/// `ServerConnection` (`get_ref().1`) carries the client chain (present because
/// mTLS required a client cert — the verifier already enforced it). The leaf is
/// hashed and looked up against the SAME active-pin oracle `verifier.rs:177`
/// used, so the identity is re-derived deterministically with no frozen-signature
/// change. Returns `None` if no cert / no active pin (close without intake).
fn resolve_verified_peer(
    tls: &tokio_rustls::server::TlsStream<TcpStream>,
    pins: &InMemoryTofuPinStore,
) -> Option<(PeerId, PeerCertFingerprint)> {
    let (_io, conn) = tls.get_ref();
    let leaf = conn.peer_certificates()?.first()?;
    let fp = PeerCertFingerprint::from_cert_der(leaf.as_ref());
    pins.find_active_pin_by_fingerprint(&fp)
        .map(|peer| (peer, fp))
}

async fn send_response<S>(
    framed: &mut Framed<S, LengthDelimitedCodec>,
    resp: &A2AJsonRpcResponse,
) -> Result<(), TcpTransportError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(resp)
        .map_err(|e| TcpTransportError::Protocol(format!("serialize response: {e}")))?;
    framed
        .send(Bytes::from(body))
        .await
        .map_err(|e| TcpTransportError::Io(format!("send response: {e}")))
}

fn is_frame_too_large(e: &std::io::Error) -> bool {
    let s = e.to_string().to_lowercase();
    s.contains("frame size too big") || s.contains("too big") || s.contains("max_frame_length")
}

/// Build the listening-side `ServerConfig` with the `TofuPinningVerifier` as the
/// `ClientCertVerifier` (mTLS — both directions pin, AC-A3).
pub fn build_server_config(
    chain: &[CertificateDer<'static>],
    key: &PrivateKeyDer<'static>,
    pins: Arc<InMemoryTofuPinStore>,
    posture: TrustPosture,
    validation_time: Option<UnixTime>,
) -> Result<ServerConfig, TcpTransportError> {
    let verifier = Arc::new(TofuPinningVerifier::new(
        pins,
        posture,
        VerifyDirection::Client,
        None, // listen side learns the peer from the cert — flat TOFU lookup
        validation_time,
    ));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| TcpTransportError::Config(format!("protocol versions: {e}")))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(chain.to_vec(), clone_key(key))
        .map_err(|e| TcpTransportError::Config(format!("server cert: {e}")))
}

/// Build the dialing-side `ClientConfig` with the `TofuPinningVerifier` as the
/// `ServerCertVerifier`, wired via `.dangerous().with_custom_certificate_verifier`
/// so it runs on the REAL handshake (AC-A3), and presenting this endpoint's own
/// auth cert (mTLS).
pub fn build_client_config(
    chain: &[CertificateDer<'static>],
    key: &PrivateKeyDer<'static>,
    pins: Arc<InMemoryTofuPinStore>,
    posture: TrustPosture,
    expected_peer: Option<PeerId>,
    validation_time: Option<UnixTime>,
) -> Result<ClientConfig, TcpTransportError> {
    let verifier = Arc::new(TofuPinningVerifier::new(
        pins,
        posture,
        VerifyDirection::Server,
        expected_peer,
        validation_time,
    ));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| TcpTransportError::Config(format!("protocol versions: {e}")))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(chain.to_vec(), clone_key(key))
        .map_err(|e| TcpTransportError::Config(format!("client auth cert: {e}")))
}

#[async_trait]
impl A2APeerRouter for TcpA2ATransport {
    async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), A2AError> {
        // Shared steps (1)–(4): allowlist + TOFU verify + clock tick + build
        // request (carrying THIS Host's boot_nonce — Correction #3 / NFR-Rel-6).
        let (request, peer_cfg, _frame_id) = self
            .core
            .prepare_outbound(frame, peer, self.own_boot_nonce)
            .await?;
        let addr = self.dial_addr(&peer_cfg).map_err(A2AError::from)?;
        // Scope the dial-side cert verifier to the peer we INTEND to reach (P1).
        let scoped_cfg = self
            .scoped_client_config(&peer_cfg.peer_id)
            .map_err(A2AError::from)?;

        // Transport-side retry (AC-T12: the ONLY retrier). Retries fire ONLY on
        // cert-class failures per `HandshakeRetryPolicy::is_retryable`.
        let max = self.retry_policy.max_attempts.max(1);
        let mut attempt: u8 = 1;
        let mut last_err: TcpTransportError;
        loop {
            self.last_dial_attempts
                .store(attempt as usize, Ordering::SeqCst);
            match self.dial_once(addr, &request, &scoped_cfg).await {
                Ok(response) => {
                    return self.core.interpret_response(peer, response);
                }
                Err(e) => {
                    last_err = e;
                    let a2a = last_err.to_a2a_error();
                    if attempt >= max || !self.retry_policy.is_retryable(&a2a) {
                        return Err(a2a);
                    }
                    let delay = self
                        .retry_policy
                        .delay_for_attempt(attempt + 1, Some(attempt as u64));
                    if delay > 0 {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    }
                    attempt += 1;
                }
            }
        }
    }

    async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse {
        self.core.handle_intake(request).await
    }
}

impl A2ATransport for TcpA2ATransport {
    fn local_addr(&self) -> Option<SocketAddr> {
        Some(self.local_addr)
    }
}

/// Bridge `TcpA2ATransport` to the `maos-domain` `A2ARouter` port so the kernel
/// mailbox can dyn-dispatch CrossHost frames through the live wire (AC-A5),
/// mapping `A2AError` → `IacBusError` via the shared
/// `maos_a2a_core::router::map_a2a_error_to_iac_bus` (identical to the loopback
/// router's bridge — AC-A6). `maos-kernel-core` gains NO new public fn.
#[async_trait]
impl maos_domain::ports::a2a::A2ARouter for TcpA2ATransport {
    async fn route_outbound(
        &self,
        frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), maos_domain::iac_bus_types::IacBusError> {
        match <Self as A2APeerRouter>::route_outbound(self, frame, peer).await {
            Ok(()) => Ok(()),
            Err(e) => Err(maos_a2a_core::router::map_a2a_error_to_iac_bus(
                e,
                peer.as_str(),
            )),
        }
    }
}
