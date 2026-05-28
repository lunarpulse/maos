//! A2A router — the bridge between `maos-kernel-core::iac::mailbox` and the
//! cross-Host wire (loopback or cross-Host).
//!
//! The trait `A2ARouter` lives in `maos-domain` (port) and the concrete
//! `LoopbackA2ARouter` impl lives here (adapter) per ADR-010 hexagonal
//! layering. The kernel-core code calls the trait; only the composition
//! root in `maos-bin` knows about the adapter.

use crate::config::A2APeerConfig;
use crate::consent::{AllowlistDirection, ConsentAllowlists, EIntentDenied};
use crate::error::{A2AError, IntentDirection};
use crate::tofu::TofuPinStore;
use crate::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, AckBody, CODE_CONSENT_EXPIRED,
    CODE_INTENT_DENIED, CODE_INTERNAL, CODE_PIN_MISMATCH_NOT_PINNED,
    CODE_SPIRIT_RESTART_DETECTED,
};
use crate::transport::logical_clock::LamportClock;
use async_trait::async_trait;
use dashmap::DashMap;
use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_spirit_abi::identity::HostId;
use std::sync::Arc;

/// Monotonic nanosecond counter — used for consent envelope expiry checks
/// when the kernel-core monotonic source is not accessible (hexagonal layering).
///
/// Returns strictly positive values on every call. The counter starts at `1`
/// (not `0`) so that envelopes with `valid_until_ns = 0` are correctly
/// classified as expired on first check — Story 6.3 §A1 P2 regression fix
/// (Epic 6 retro 2026-05-28). A counter that returns `0` on its first
/// observation would silently admit a `valid_until_ns = 0` envelope because
/// `0 > 0` is false; the production hot path failed open for that
/// configuration before this fix.
fn monotonic_now_ns() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed).saturating_add(1)
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

/// Loopback A2A router — `127.0.0.1`-bound endpoints with self-signed mTLS
/// + TOFU pinning. For v0.5 the loopback router routes frames in-process via
/// the same `handle_intake` path used by cross-Host (the wire is just a
/// `tokio::sync::mpsc` shortcut). The mTLS + TOFU substrate is exercised at
/// connection time, not per-frame.
///
/// `peers` maps `HostId` → `A2APeerConfig`.
/// `tofu` is the pin store (in-memory at v0.5; persistence-backed in follow-up).
/// `clock` is the per-router Lamport clock.
pub struct LoopbackA2ARouter {
    peers: Arc<DashMap<String, A2APeerConfig>>,
    tofu: Arc<dyn TofuPinStore>,
    clock: Arc<LamportClock>,
    /// Optional intake sink for tests — when set, accepted frames are
    /// pushed here so test code can observe them.
    intake_sink: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<IacFrame>>>>,
    /// Atomic counter for outbound request ids.
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl LoopbackA2ARouter {
    pub fn new(peer_configs: Vec<A2APeerConfig>, tofu: Arc<dyn TofuPinStore>) -> Self {
        let peers = Arc::new(DashMap::new());
        for cfg in peer_configs {
            let key = cfg.peer_id.as_str().to_string();
            if peers.contains_key(&key) {
                // Duplicate peer_id — log warning but don't panic; last wins.
                // Production operator config should be validated at admission.
                eprintln!("maos-a2a: WARNING: duplicate peer config for peer_id {key}; overwriting");
            }
            peers.insert(key, cfg);
        }
        Self {
            peers,
            tofu,
            clock: Arc::new(LamportClock::new()),
            intake_sink: Arc::new(tokio::sync::Mutex::new(None)),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
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

    fn alloc_id(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn lookup_peer(&self, host_id: &HostId) -> Result<A2APeerConfig, A2AError> {
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
    fn frame_intent_str(frame: &IacFrame) -> String {
        frame.intent.a2a_consent_intent_str().to_string()
    }

    /// Send-allowlist enforcement. The peer's `send_allowlist` enumerates
    /// `A2AIntent` strings the operator wills THIS Host to SEND to that peer.
    fn send_admits(allow: &ConsentAllowlists, frame: &IacFrame) -> bool {
        let s = Self::frame_intent_str(frame);
        allow
            .send_allowlist
            .iter()
            .any(|i| i.as_str().eq_ignore_ascii_case(&s))
    }

    fn accept_admits(allow: &ConsentAllowlists, frame: &IacFrame) -> bool {
        let s = Self::frame_intent_str(frame);
        allow
            .accept_allowlist
            .iter()
            .any(|i| i.as_str().eq_ignore_ascii_case(&s))
    }
}

/// Bridge `LoopbackA2ARouter` to the `maos-domain` `A2ARouter` port trait so
/// `maos-kernel-core::iac::mailbox` can dyn-dispatch through it.
/// Maps A2AError variants to typed IacBusError sub-variants.
#[async_trait]
impl maos_domain::ports::a2a::A2ARouter for LoopbackA2ARouter {
    async fn route_outbound(
        &self,
        frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), IacBusError> {
        match <Self as A2APeerRouter>::route_outbound(self, frame, peer).await {
            Ok(()) => Ok(()),
            Err(e) => Err(map_a2a_error_to_iac_bus(e, peer.as_str())),
        }
    }
}

/// Map an A2AError variant to its typed IacBusError sub-variant.
/// Preserves structured information so callers can programmatically
/// distinguish intent-denial from partition-timeout from pin-mismatch.
fn map_a2a_error_to_iac_bus(err: A2AError, peer: &str) -> IacBusError {
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
        | A2AError::HandshakeFailed(detail) => {
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
    }
}

#[async_trait]
impl A2APeerRouter for LoopbackA2ARouter {
    async fn route_outbound(
        &self,
        mut frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), A2AError> {
        let peer_cfg = self.lookup_peer(peer)?;

        // (1) ADR-012 send-allowlist check (defense-in-depth — sender side).
        if !Self::send_admits(&peer_cfg.allowlists, &frame) {
            return Err(A2AError::IntentDenied {
                direction: IntentDirection::Send,
                inner: EIntentDenied {
                    peer: peer.as_str().to_string(),
                    intent: Self::frame_intent_str(&frame),
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

        // (4) Build JSON-RPC request and hand to intake (loopback shortcut).
        let id = self.alloc_id();
        let frame_id = frame.frame_id;
        let request = A2AJsonRpcRequest::new("iac.deliver", frame.clone(), id);
        let timeout = std::time::Duration::from_secs(peer_cfg.partition_timeout_secs);
        let intake_fut = self.handle_intake(request);
        let response = match tokio::time::timeout(timeout, intake_fut).await {
            Ok(resp) => resp,
            Err(_) => {
                return Err(A2AError::PartitionTimeout {
                    peer: peer.as_str().to_string(),
                    frame_id,
                    timeout_secs: peer_cfg.partition_timeout_secs,
                });
            }
        };

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
                _ => Err(A2AError::TransportFailed(n.error.message)),
            },
        }
    }

    async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse {
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
            if let Some(pin) = self.tofu.get_pin(&peer_cfg.peer_id).await {
                if request.boot_nonce != pin.boot_nonce {
                    // Boot nonce rolled — the Spirit has restarted (or an
                    // attacker is racing the legitimate Spirit). Invalidate
                    // the prior pin and refuse the frame; the operator must
                    // approve a re-pin via `await_repin_consent` before
                    // further A2A traffic resumes.
                    let prior = pin.boot_nonce;
                    let observed = request.boot_nonce;
                    if let Err(e) = self
                        .tofu
                        .invalidate_for_restart(&peer_cfg.peer_id, prior)
                        .await
                    {
                        // Invalidation itself failed — surface as INTERNAL.
                        return A2AJsonRpcResponse::nack(
                            request.id,
                            CODE_INTERNAL,
                            format!("invalidate_for_restart failed: {e}"),
                        );
                    }
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
            }
        }

        // (2) ADR-012 accept-allowlist check.
        if !Self::accept_admits(&peer_cfg.allowlists, frame) {
            return A2AJsonRpcResponse::nack(
                request.id,
                CODE_INTENT_DENIED,
                format!(
                    "intent {} not in accept_allowlist for peer {}",
                    Self::frame_intent_str(frame),
                    peer_cfg.peer_id.as_str()
                ),
            );
        }

        // (3) Consent envelope expiry check.
        if let Some(envelope) = &frame.consent_envelope {
            if let Some(valid_until_ns) = envelope.valid_until_ns {
                let now_ns = monotonic_now_ns();
                if now_ns > valid_until_ns {
                    let data = serde_json::json!({
                        "expired_at_ns": valid_until_ns,
                        "now_ns": now_ns,
                    });
                    let mut resp = A2AJsonRpcResponse::nack(
                        request.id,
                        CODE_CONSENT_EXPIRED,
                        format!(
                            "consent envelope expired at {valid_until_ns} (now {now_ns})"
                        ),
                    );
                    // Attach timestamp data to the NACK.
                    if let A2AJsonRpcResponse::Nack(ref mut n) = resp {
                        n.error.data = Some(data);
                    }
                    return resp;
                }
            }
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

    #[tokio::test]
    async fn outbound_send_admitted_intent_succeeds() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let cfg = make_peer_cfg(allow.clone());
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        // Pre-pin required for TOFU verification in route_outbound.
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        let router = LoopbackA2ARouter::new(vec![cfg], tofu);
        let frame = make_frame(Some("loopback"));
        router
            .route_outbound(frame, &HostId("loopback".to_string()))
            .await
            .expect("outbound");
    }

    #[tokio::test]
    async fn outbound_send_denied_intent_rejects_at_sender() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let cfg = make_peer_cfg(allow);
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        // Pin so TOFU check doesn't block outbound.
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        let router = LoopbackA2ARouter::new(vec![cfg], tofu);
        let frame = make_frame(Some("loopback"));
        let err = router
            .route_outbound(frame, &HostId("loopback".to_string()))
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
        let cfg = make_peer_cfg(allow.clone());
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        // Pin for TOFU verification.
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        let router = LoopbackA2ARouter::new(vec![cfg], tofu);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        router.install_intake_sink(tx).await;
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = router.handle_intake(req).await;
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
        let cfg = make_peer_cfg(allow.clone());
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        // Pin for TOFU verification.
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        let router = LoopbackA2ARouter::new(vec![cfg], tofu);
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = router.handle_intake(req).await;
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
        let cfg = make_peer_cfg(allow.clone());
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        // Pin for TOFU verification.
        tofu.pin_first_contact(
            &PeerId::new("loopback"),
            &cfg.cert_fingerprint,
            &cfg.cert_fingerprint,
            1,
        )
        .await
        .expect("pin");
        let router = LoopbackA2ARouter::new(vec![cfg], tofu);
        let mut frame = make_frame(Some("loopback"));
        frame.logical_clock = 100;
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = router.handle_intake(req).await;
        match resp {
            A2AJsonRpcResponse::Ack(a) => {
                assert_eq!(a.result.receiver_logical_clock, 101);
            }
            _ => panic!("expected Ack"),
        }
    }
}
