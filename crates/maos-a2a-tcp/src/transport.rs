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
use futures_util::{FutureExt, SinkExt, StreamExt};
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::{A2APeerRouter, A2ARouterCore, A2ATransport};
use maos_a2a_core::transport::json_rpc::{CODE_FRAME_TOO_LARGE, CODE_TIMEOUT};
use maos_a2a_core::{
    A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, A2APeerConfig, CohortManifestGate,
    ConsentRuptureSink, DigestReadPort, HaltReceiptObserver, HandshakeRetryPolicy,
    InMemoryTofuPinStore, PeerRefusalDirection, TofuPinStore,
};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, ServerConfig};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
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
    /// §A6 mid-story (Blind-5): serializes generation swaps so two
    /// concurrent `swap_serving_cert` calls cannot interleave their
    /// dial/serve halves (dial-A, dial-B, serve-B, serve-A — a node serving
    /// one leaf while presenting another).
    swap_lock: std::sync::Mutex<()>,
    /// the `ServerConfig`, so `swap_serving_cert` can rotate the serving
    /// generation in place without a rebind.
    serving_cert: Arc<SwappableServingCert>,
    /// Shared (unscoped) dialing config — kept for the raw-socket test helpers
    /// (AC-T2/7/8/9) and as the `client_config()` accessor. Real
    /// `route_outbound` dials build a PER-PEER scoped config instead (review
    /// patch P1) via [`Self::scoped_client_config`].
    client_config: Arc<ClientConfig>,
    /// Materials to rebuild a per-dial `ClientConfig` whose `ServerCertVerifier`
    /// is scoped to the EXPECTED peer (review patch P1) — the dial target is only
    /// known per `route_outbound` call, so the verifier cannot be fixed at bind.
    /// Story 14-2 / AC2.4 — swappable with the SERVING generation: a node's
    /// leaf is ONE identity (it serves it AND presents it on outbound mTLS
    /// handshakes), so `swap_serving_cert` rotates BOTH or post-swap dials
    /// present the retired leaf and promoted peers refuse them (measured by
    /// the scene's beat-5 positive control, 2026-08-29).
    dial_materials: Arc<SwappableDialMaterials>,
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
    /// Story 14-2a review — the supervised-boundary policy for a panic
    /// escaping a per-connection task (fail-stop in production; the Observe
    /// seam exists so tests never abort the shared test runner). Read per
    /// connection at accept time, so `install_connection_panic_policy`
    /// governs connections accepted after it.
    panic_policy: Arc<Mutex<ConnectionPanicPolicy>>,
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

        let (server_config, serving_cert) = build_server_config(
            &own_chain,
            &own_key,
            pins.clone(),
            posture.clone(),
            validation_time,
        )?;
        let server_config = Arc::new(server_config);
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
        let panic_policy = Arc::new(Mutex::new(ConnectionPanicPolicy::FailStop));

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
            panic_policy.clone(),
        ));

        Ok(Self {
            core,
            pins,
            serving_cert,
            client_config,
            dial_materials: Arc::new(SwappableDialMaterials::new(
                own_chain.clone(),
                clone_key(&own_key),
            )),
            swap_lock: std::sync::Mutex::new(()),
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
            panic_policy,
            _serve_guard: Arc::new(ServeGuard { accept, conns }),
        })
    }

    /// Story 14-2a review — replace the connection-task panic policy. `bind`
    /// defaults to [`ConnectionPanicPolicy::FailStop`] (a panicking
    /// connection task terminates the daemon process); tests and embedders
    /// install [`ConnectionPanicPolicy::Observe`] instead so the supervised
    /// boundary is exercisable without aborting the host process. The policy
    /// is read per connection at accept time, so connections accepted after
    /// this call are governed by it.
    pub fn install_connection_panic_policy(&self, policy: ConnectionPanicPolicy) {
        if let Ok(mut guard) = self.panic_policy.lock() {
            *guard = policy;
        }
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

    /// Story 14-2 / AC2.4 — swap the cert generation IN PLACE: no rebind, no
    /// listener churn, no connection teardown. Rotates BOTH halves of the
    /// node's ONE identity — the SERVING resolver (new handshakes resolve the
    /// replacement from the next ClientHello; established connections are
    /// untouched, rustls never renegotiates) and the DIAL materials (outbound
    /// mTLS handshakes present the replacement; a half-swapped node would
    /// serve the new leaf while presenting the retired one, and promoted
    /// peers refuse it — measured by the scene's beat-5 control). Callers:
    /// the rotation drill and the N=3 scene — there is NO production caller;
    /// the production trigger is the operator provisioning act of AC2.0,
    /// which has no live reload path (AC2.2.d/AC6.5(c)).
    pub fn swap_serving_cert(
        &self,
        chain: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<(), TcpTransportError> {
        // §A6 (Edge-5/Blind-5 + close-pass CloseEdge-2): VALIDATE FIRST, then
        // ACQUIRE BOTH write guards under swap_lock BEFORE writing either —
        // a poisoned serving lock must not leave the dial half swapped —
        // then commit. A rejected rotation leaves the node UNCHANGED, and
        // concurrent swaps publish one generation each.
        let ck = SwappableServingCert::certified_key(chain.clone(), &key)?;
        let _guard = self
            .swap_lock
            .lock()
            .map_err(|_| TcpTransportError::Config("swap lock poisoned".into()))?;
        let mut dial_guard = self.dial_materials.write_guard()?;
        let mut serve_guard = self.serving_cert.write_guard()?;
        *dial_guard = (chain, key);
        *serve_guard = Arc::new(ck);
        Ok(())
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
    ///
    /// **Story 14-2 / AC2.4.b — STANDING CONSTRAINT: never cache
    /// `ClientConfig` per peer.** Every dial MUST remain a full handshake.
    /// `ServerConfig` defaults issue TLS 1.3 tickets; a resumed handshake
    /// sends no Certificate message, so a cached `ClientConfig` would resume
    /// sessions across a cert rotation and NEVER re-run `TofuPinningVerifier`
    /// — the dialer would keep verifying the PRE-ROTATION leaf indefinitely.
    /// The next person optimising throughput will reach for exactly that
    /// cache; this comment is the tripwire.
    fn scoped_client_config(
        &self,
        expected: &PeerId,
    ) -> Result<Arc<ClientConfig>, TcpTransportError> {
        let (chain, key) = self
            .dial_materials
            .snapshot()
            .ok_or_else(|| TcpTransportError::Config("dial materials lock poisoned".into()))?;
        Ok(Arc::new(build_client_config(
            &chain,
            &key,
            self.pins.clone(),
            self.posture.clone(),
            Some(expected.clone()),
            self.validation_time,
        )?))
    }

    /// One dial+handshake+send+recv round, returning the typed transport result
    /// or a classified error. `client_config` is the per-peer scoped dialing
    /// config (patch P1).
    ///
    /// `partition` is the operator-configured partition window for this peer
    /// (`j1-crosshost-2c` AC3.1/AC3.3). Every step that can block on a
    /// non-cooperating peer is bounded by it or by the injected handshake wall;
    /// nothing here waits on an OS backstop.
    async fn dial_once(
        &self,
        addr: SocketAddr,
        request: &A2AJsonRpcRequest,
        client_config: &Arc<ClientConfig>,
        partition: Duration,
    ) -> Result<A2AJsonRpcResponse, TcpTransportError> {
        let connector = TlsConnector::from(client_config.clone());
        // AC3.1 — `TcpStream::connect` was a bare `.await`. Against a black-holed
        // address it hangs on the kernel's SYN-retry backstop (~130s on Linux),
        // far past any partition window an operator can configure.
        let tcp = match tokio::time::timeout(partition, TcpStream::connect(addr)).await {
            Err(_) => {
                return Err(TcpTransportError::PartitionTimeout {
                    phase: format!("connect {addr}"),
                    secs: partition.as_secs(),
                })
            }
            Ok(Err(e)) => return Err(TcpTransportError::Io(format!("connect {addr}: {e}"))),
            Ok(Ok(s)) => s,
        };
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
        // AC3.1 — `framed.send` was ALSO unbounded, and it is the CHEAPER real
        // partition: a peer that completes the handshake and then stops reading
        // fills the socket buffer and hangs `route_outbound` forever with NO OS
        // backstop at all. Nothing bounded this before.
        match tokio::time::timeout(partition, framed.send(Bytes::from(body))).await {
            Err(_) => {
                return Err(TcpTransportError::PartitionTimeout {
                    phase: "sending request".to_string(),
                    secs: partition.as_secs(),
                })
            }
            Ok(Err(e)) => return Err(TcpTransportError::Io(format!("send: {e}"))),
            Ok(Ok(())) => {}
        }

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

    /// Route one frame and report whether a decoded ACK/NACK response reached
    /// the caller. The boolean is the exact conversation-verdict boundary:
    /// local validation, handshake, timeout, and transport failures are
    /// `false`; every decoded peer response is `true`, regardless of its typed
    /// interpretation.
    pub async fn route_outbound_observed(
        &self,
        frame: IacFrame,
        peer: &HostId,
    ) -> (Result<(), A2AError>, bool) {
        let (request, peer_cfg, frame_id) = match self
            .core
            .prepare_outbound(frame, peer, self.own_boot_nonce)
            .await
        {
            Ok(prepared) => prepared,
            Err(error) => return (Err(error), false),
        };
        let addr = match self.dial_addr(&peer_cfg).map_err(A2AError::from) {
            Ok(addr) => addr,
            Err(error) => return (Err(error), false),
        };
        let scoped_cfg = match self
            .scoped_client_config(&peer_cfg.peer_id)
            .map_err(A2AError::from)
        {
            Ok(config) => config,
            Err(error) => return (Err(error), false),
        };
        let partition =
            Duration::from_secs(peer_cfg.partition_timeout_secs).min(self.timeouts.idle);
        let configured = Duration::from_secs(peer_cfg.partition_timeout_secs);
        if configured > partition {
            tracing::warn!(
                peer = %peer.as_str(),
                configured_secs = peer_cfg.partition_timeout_secs,
                effective_secs = partition.as_secs(),
                "partition window exceeds the idle wall — clamped; raise the idle \
                 timeout or lower the configured window"
            );
        }

        let max = self.retry_policy.max_attempts.max(1);
        let mut attempt: u8 = 1;
        loop {
            self.last_dial_attempts
                .store(attempt as usize, Ordering::SeqCst);
            match self.dial_once(addr, &request, &scoped_cfg, partition).await {
                Ok(response) => return (self.core.interpret_response(peer, response), true),
                Err(last_err) => {
                    if let TcpTransportError::PartitionTimeout { phase, secs } = &last_err {
                        tracing::warn!(
                            peer = %peer.as_str(),
                            phase = %phase,
                            timeout_secs = *secs,
                            "a2a partition timeout — frame NOT delivered, no kernel auto-retry"
                        );
                        return (
                            Err(A2AError::PartitionTimeout {
                                peer: peer.as_str().to_string(),
                                frame_id,
                                timeout_secs: if *secs == 0 {
                                    peer_cfg.partition_timeout_secs
                                } else {
                                    *secs
                                },
                            }),
                            false,
                        );
                    }
                    if last_err.is_tofu_mismatch() {
                        if let Err(journal_err) = self
                            .core
                            .journal_peer_identity_refusal(
                                PeerRefusalDirection::Dial,
                                peer.as_str(),
                                &last_err.to_string(),
                            )
                            .await
                        {
                            tracing::error!(
                                "peer-identity refusal was NOT journaled ({journal_err}): {}",
                                last_err
                            );
                        }
                    }
                    let a2a = last_err.to_a2a_error();
                    if attempt >= max || !self.retry_policy.is_retryable(&a2a) {
                        return (Err(a2a), false);
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
}

/// The accept loop — one supervised per-connection `tokio::spawn` with its
/// `JoinHandle` held in the drop-guard registry (H6). The supervision seam
/// wraps every connection future so a task panic can no longer be silently
/// discarded by that registry (Story 14-2a review).
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
    panic_policy: Arc<Mutex<ConnectionPanicPolicy>>,
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

        // Story 14-2a review — the supervised boundary. The peer hint is
        // captured BEFORE `tcp` moves into the connection task; it is the
        // only label available once the task is gone.
        let peer_addr_hint = tcp
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        // Read per connection (not once at bind) so a policy installed after
        // `bind` governs connections accepted from then on; a poisoned lock
        // fails CLOSED to the production default.
        let policy = match panic_policy.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => ConnectionPanicPolicy::FailStop,
        };
        let handle = tokio::spawn(supervise_connection(
            policy,
            peer_addr_hint,
            serve_connection(
                tcp,
                acceptor,
                core,
                pins,
                timeouts,
                intake_entered,
                active_connections,
                last_intake,
            ),
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

/// Story 14-2a review — what the supervised connection boundary does when a
/// per-connection task PANICS (e.g. the production I2 audit-write panic
/// reached by a signed-manifest reissue, or any future bug in the read loop).
///
/// Before this seam the per-connection `JoinHandle`s in the drop-guard
/// registry were never awaited, so a panicking connection task was silently
/// discarded: the daemon kept serving with a piece of itself dead, and the
/// audit-write panic policy — fail-stop at DAEMON-PROCESS scope — had no
/// enforcement point. The boundary below is that enforcement point.
#[derive(Clone)]
pub enum ConnectionPanicPolicy {
    /// Production default: log, then `std::process::abort()` the daemon
    /// process. Abort (not exit) is deliberate — no destructors, no flushes,
    /// no further state mutation racing a dying daemon: a panicking
    /// connection can never leave the daemon serving.
    FailStop,
    /// Test/embedding seam: hand the observation to `hook` and keep serving,
    /// so the boundary is exercisable without aborting the host process
    /// (e.g. the shared `cargo test` runner). Never install outside tests.
    Observe(Arc<dyn Fn(&ConnectionPanic) + Send + Sync>),
}

/// What the supervised boundary reports about one panicking connection task.
#[derive(Debug, Clone)]
pub struct ConnectionPanic {
    /// Best-effort socket label of the connection whose task panicked
    /// (`"unknown"` when the socket was already gone).
    pub peer_addr: String,
    /// Display form of the panic payload. Diagnostics ONLY — the boundary's
    /// decision is the typed `catch_unwind` boundary itself; nothing parses
    /// this text.
    pub message: String,
}

/// Story 14-2a review — the explicit supervised task boundary around one
/// per-connection task. `serve_connection` stays panic-transparent; this
/// wrapper is the ONLY place a connection panic is observed, and the selected
/// [`ConnectionPanicPolicy`] — and nothing else — decides what it means. A
/// drop-guard `abort()` is NOT a panic: cancellation never reaches
/// `catch_unwind`, so the H6 shutdown path is untouched.
async fn supervise_connection<F>(policy: ConnectionPanicPolicy, peer_addr: String, fut: F)
where
    F: Future<Output = ()>,
{
    match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
        Ok(()) => {}
        Err(payload) => enforce_connection_panic_policy(policy, peer_addr, payload),
    }
}

/// Enforce [`ConnectionPanicPolicy`] for one observed connection-task panic.
fn enforce_connection_panic_policy(
    policy: ConnectionPanicPolicy,
    peer_addr: String,
    payload: Box<dyn std::any::Any + Send>,
) {
    let message = panic_payload_message(payload.as_ref());
    tracing::error!(
        peer = %peer_addr,
        panic = %message,
        "a per-connection task PANICKED — the supervised boundary is enforcing the \
         connection-panic policy"
    );
    match policy {
        ConnectionPanicPolicy::FailStop => {
            // stderr as well as tracing: `abort()` must never be the FIRST
            // someone hears of this, whatever subscriber the daemon runs.
            eprintln!(
                "FATAL (fail-stop): connection task to {peer_addr} panicked: {message}; \
                 aborting the daemon process"
            );
            std::process::abort();
        }
        ConnectionPanicPolicy::Observe(hook) => hook(&ConnectionPanic { peer_addr, message }),
    }
}

/// Best-effort DISPLAY of a panic payload for the log/observation only. This
/// is the standard `&'static str`/`String` payload downcast, not parsing:
/// control flow never keys on the text.
fn panic_payload_message(payload: &dyn std::any::Any) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
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

    // AC3.6 — the only identity available when the pin check fails is the socket
    // address. Captured BEFORE `tcp` moves into the acceptor.
    let peer_addr_hint = tcp
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Handshake — bounded; a plaintext client (AC-T9) or a half-open client
    // (AC-T10) fails here and the task ends without ever entering intake.
    //
    // `j1-crosshost-2c` AC3.6 — the old code was a blanket `_ => return`, so a
    // client whose leaf failed the TOFU pin verifier left **ZERO trace** on this
    // host: no rupture row, no typed error, nothing an operator could query. The
    // verifier itself provably cannot reach a Transparency Log, but `core` is in
    // scope here and already carries the installed synchronous
    // `ConsentRuptureSink`.
    let tls = match tokio::time::timeout(timeouts.handshake, acceptor.accept(tcp)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let classified = TcpTransportError::classify_handshake(&e.to_string());
            // Story 14-2a / AC4.4 — §7.2.1.a requires, verbatim, that the
            // post-grace rejection be *"logged as `cert_post_grace_reject`"*.
            // Until this arm it was the ONE failure mode this whole rotation
            // mechanism exists to produce and the ONLY one that left NO trace at
            // all: a validity rejection fell through the `is_tofu_mismatch()`
            // arm and returned bare, so an auditor asking "did anyone keep using
            // the old certificate?" had nothing to read.
            //
            // ⚠ WHICH ARM IS THE POST-GRACE ONE — MEASURED, AND IT IS NOT THE
            // OBVIOUS ONE. A retired leaf is normally still WITHIN its validity
            // window (a rotation is not an expiry), so after promotion the
            // verifier refuses it as `PIN_MISMATCH`, not `CERT_EXPIRED`. The
            // sentinel therefore rides the MISMATCH arm, and the validity arm
            // gets its own accurate label instead of being mislabelled a
            // post-grace refusal — an expired CURRENT certificate is a different
            // operator problem and must not be filed as a rotation event.
            //
            // ⚠ AND THE SENTINEL IS NON-EXCLUSIVE, BY CONSTRUCTION. The store
            // keeps NO retired-generation history (`TofuPin` has no previous
            // field and `close_rotation_window` returns the retired value to a
            // caller that drops it), and a failed handshake surfaces no observed
            // fingerprint here at all — so "retired leaf after the grace" and
            // "unknown leaf, i.e. an impersonation attempt" are ONE observable
            // at this seam. The row says so rather than claiming to know which.
            // The join an auditor actually uses is the rotation timeline: a
            // `cert_rotation_window_closed` row for the peer, then this refusal.
            let refusal = if classified.is_tofu_mismatch() {
                Some(format!(
                    "cert_post_grace_reject candidate (or unknown-leaf refusal — this seam \
                     cannot distinguish them: no retired-generation history exists): {classified}"
                ))
            } else if classified.is_cert_validity() {
                Some(format!("cert_validity_reject: {classified}"))
            } else {
                None
            };
            if let Some(detail) = refusal {
                // Best-effort: a journaling failure must not become a way to keep
                // the listener from refusing — but it must not be SILENT either
                // (§A6 review 2026-08-18): the Err contract on
                // `journal_peer_identity_refusal` says "fails loudly"; discarding
                // it here was the pre-AC3.6 silence with a different spelling.
                if let Err(journal_err) = core
                    .journal_peer_identity_refusal(
                        PeerRefusalDirection::Listen,
                        peer_addr_hint.as_str(),
                        detail.as_str(),
                    )
                    .await
                {
                    tracing::error!(
                        "peer-identity refusal was NOT journaled ({}): {}",
                        journal_err,
                        detail
                    );
                }
            }
            return;
        }
        Err(_) => return,
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
            // AC3.6 — this arm is the other half of the same silence: the
            // handshake passed the "any active pin" check but no peer owns the
            // negotiated leaf. Journal it rather than only warning.
            tracing::warn!("resolve_verified_peer: no active pin for negotiated client cert — closing connection without intake");
            if let Err(journal_err) = core
                .journal_peer_identity_refusal(
                    PeerRefusalDirection::Listen,
                    peer_addr_hint.as_str(),
                    "no active pin owns the negotiated client leaf",
                )
                .await
            {
                tracing::error!(
                    "peer-identity refusal was NOT journaled ({journal_err}): no \
                     active pin owns the negotiated client leaf"
                );
            }
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

/// Story 14-2 / AC2.4 — the node's DIAL identity (client-auth chain + key),
/// swappable in place with the serving generation for the same reason the
/// serving resolver is: rotation is a generation swap of ONE identity, and a
/// half-swapped node serves the new leaf while still PRESENTING the retired
/// one on outbound handshakes. Poison fails CLOSED (no materials read → no
/// dial), matching the file's existing `if let Ok(...)` idiom.
struct SwappableDialMaterials(
    std::sync::RwLock<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
);

impl SwappableDialMaterials {
    fn new(chain: Vec<CertificateDer<'static>>, key: PrivateKeyDer<'static>) -> Self {
        Self(std::sync::RwLock::new((chain, key)))
    }

    /// Clone the current dial materials for one per-dial `ClientConfig`
    /// build. `None` on poison — callers fail closed.
    fn snapshot(&self) -> Option<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        self.0.read().ok().map(|g| (g.0.clone(), g.1.clone_key()))
    }

    /// §A6 close-pass (CloseEdge-2): expose the write guard so a generation
    /// swap can acquire BOTH locks BEFORE writing either half.
    fn write_guard(
        &self,
    ) -> Result<
        std::sync::RwLockWriteGuard<'_, (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)>,
        TcpTransportError,
    > {
        self.0
            .write()
            .map_err(|_| TcpTransportError::Config("dial materials lock poisoned".into()))
    }
}

/// Story 14-2 / AC2.4.a — a swappable serving-cert resolver: interior
/// mutability behind a `std::sync::RwLock`, so the serving generation can be
/// rotated IN PLACE. `TlsAcceptor::accept` re-reads the same resolver `Arc`
/// per connection (tokio-rustls `server.rs`), `resolve` runs exactly once
/// per ClientHello (rustls `server/hs.rs`), and rustls performs NO
/// renegotiation at all — so in-flight connections are provably unaffected
/// by a swap and the accept loop needs ZERO changes. Connection continuity,
/// not dual-serving, is the reason this exists: the one-generation overlap
/// (§7.2.1.a) requires old and new cert to be acceptable DURING the swap
/// window, and a teardown-rebind physically cannot provide it.
#[derive(Debug)]
pub struct SwappableServingCert(RwLock<Arc<rustls::sign::CertifiedKey>>);

impl SwappableServingCert {
    fn new(
        chain: Vec<CertificateDer<'static>>,
        key: &PrivateKeyDer<'static>,
    ) -> Result<Self, TcpTransportError> {
        Ok(Self(std::sync::RwLock::new(Arc::new(Self::certified_key(
            chain, key,
        )?))))
    }

    /// §A6 mid-story (Blind-4/Edge-4/Acceptance-3): build through
    /// `CertifiedKey::from_der` — the path `with_single_cert` used — so an
    /// EMPTY chain or a key that does not match the leaf is refused HERE,
    /// at build/swap time, instead of succeeding and failing every later
    /// handshake.
    fn certified_key(
        chain: Vec<CertificateDer<'static>>,
        key: &PrivateKeyDer<'static>,
    ) -> Result<rustls::sign::CertifiedKey, TcpTransportError> {
        if chain.is_empty() {
            return Err(TcpTransportError::Config(
                "serving chain must carry at least the leaf certificate".into(),
            ));
        }
        let provider = rustls::crypto::ring::default_provider();
        rustls::sign::CertifiedKey::from_der(chain, clone_key(key), &provider).map_err(|e| {
            TcpTransportError::Config(format!(
                "serving cert/key pair unusable (mismatched key or unparseable): {e:?}"
            ))
        })
    }

    /// §A6 close-pass (CloseEdge-2): expose the write guard so a generation
    /// swap can acquire BOTH locks BEFORE writing either half.
    fn write_guard(
        &self,
    ) -> Result<std::sync::RwLockWriteGuard<'_, Arc<rustls::sign::CertifiedKey>>, TcpTransportError>
    {
        self.0
            .write()
            .map_err(|_| TcpTransportError::Config("serving cert lock poisoned".into()))
    }
}

impl rustls::server::ResolvesServerCert for SwappableServingCert {
    fn resolve(
        &self,
        _client_hello: rustls::server::ClientHello,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        // Poison idiom (the file's existing `if let Ok(...)` shape): a
        // poisoned lock fails CLOSED — no cert resolved, handshake refused.
        self.0.read().ok().map(|g| g.clone())
    }
}
/// Build the listening-side `ServerConfig` with the `TofuPinningVerifier` as
/// the `ClientCertVerifier` (mTLS — both directions pin, AC-A3), serving via
/// a [`SwappableServingCert`] resolver so Story 14-2's one-generation overlap
/// can swap the serving cert in place (AC2.4.a). Returns the config AND the
/// shared resolver handle — `with_cert_resolver` builds the
/// `rustls::sign::CertifiedKey` itself, which `with_single_cert` used to do
/// for us. The `?` on protocol versions survives, hence the tuple in a
/// `Result`, not a bare tuple.
pub fn build_server_config(
    chain: &[CertificateDer<'static>],
    key: &PrivateKeyDer<'static>,
    pins: Arc<InMemoryTofuPinStore>,
    posture: TrustPosture,
    validation_time: Option<UnixTime>,
) -> Result<(ServerConfig, Arc<SwappableServingCert>), TcpTransportError> {
    let verifier = Arc::new(TofuPinningVerifier::new(
        pins,
        posture,
        VerifyDirection::Client,
        None, // listen side learns the peer from the cert — flat TOFU lookup
        validation_time,
    ));
    let resolver = Arc::new(SwappableServingCert::new(chain.to_vec(), key)?);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| TcpTransportError::Config(format!("protocol versions: {e}")))?
        .with_client_cert_verifier(verifier)
        .with_cert_resolver(resolver.clone());
    Ok((config, resolver))
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
        self.route_outbound_observed(frame, peer).await.0
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
