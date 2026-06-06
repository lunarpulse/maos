//! Loopback A2A router — the only `A2ATransport` impl `maos-a2a` retains after
//! the Story 8.6 extraction. The transport-agnostic validation engine moved to
//! `maos_a2a_core::router::A2ARouterCore`; `LoopbackA2ARouter` is now a thin
//! wrapper that delegates intake/outbound to the core engine and routes frames
//! in-process (the "wire" is a `handle_intake` shortcut, not a socket).
//!
//! The live cross-Host wire is `maos_a2a_tcp::TcpA2ATransport`, the second
//! `A2ATransport` impl — it depends on `maos-a2a-core`, NOT on this crate.

use async_trait::async_trait;
use maos_a2a_core::router::{map_a2a_error_to_iac_bus, A2ARouterCore};

// Story 8.6: `pub use` (not plain `use`) the traits so the pre-extraction
// `maos_a2a::adapter::{A2APeerRouter, A2ATransport}` paths still resolve — this
// keeps a `cargo public-api` diff of `maos-a2a` Added-only (epic AC-A1/A6: the
// trait moved to `maos-a2a-core` but the historical sub-module path is
// retained). The import also brings the traits into scope for the impls below.
pub use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, A2APeerConfig, LamportClock, TofuPinStore};
use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_spirit_abi::identity::HostId;
use std::sync::Arc;

/// Loopback A2A router — `127.0.0.1`-bound endpoints with self-signed mTLS
/// + TOFU pinning. For v0.5 the loopback router routes frames in-process via
/// the same `handle_intake` path used by cross-Host (the wire is just a
/// `handle_intake` shortcut). The mTLS + TOFU substrate is exercised at
/// connection time, not per-frame.
///
/// Story 8.6: now a thin wrapper around [`A2ARouterCore`] (the shared
/// validation engine in `maos-a2a-core`); the public surface
/// (`new`/`install_intake_sink`/`clock`/the two router traits) is unchanged.
pub struct LoopbackA2ARouter {
    core: A2ARouterCore,
}

impl LoopbackA2ARouter {
    pub fn new(peer_configs: Vec<A2APeerConfig>, tofu: Arc<dyn TofuPinStore>) -> Self {
        Self {
            core: A2ARouterCore::new(peer_configs, tofu),
        }
    }

    /// Install an intake sink — test-only hook. Accepted frames are forwarded
    /// to the sink AFTER all validation passes.
    pub async fn install_intake_sink(
        &self,
        sink: tokio::sync::mpsc::UnboundedSender<IacFrame>,
    ) {
        self.core.install_intake_sink(sink).await;
    }

    pub fn clock(&self) -> Arc<LamportClock> {
        self.core.clock()
    }
}

#[async_trait]
impl A2APeerRouter for LoopbackA2ARouter {
    async fn route_outbound(
        &self,
        frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), A2AError> {
        // Shared steps (1)–(4): allowlist + TOFU verify + clock tick + build.
        // Loopback uses the v0.5-α `boot_nonce = 0` sentinel (the in-process
        // shortcut never crosses a restart boundary).
        let (request, peer_cfg, frame_id) =
            self.core.prepare_outbound(frame, peer, 0).await?;

        // Loopback "wire": hand the request straight to intake, bounded by the
        // peer's partition timeout per architecture §7.2.
        let timeout = std::time::Duration::from_secs(peer_cfg.partition_timeout_secs);
        let response = match tokio::time::timeout(timeout, self.core.handle_intake(request)).await {
            Ok(resp) => resp,
            Err(_) => {
                return Err(A2AError::PartitionTimeout {
                    peer: peer.as_str().to_string(),
                    frame_id,
                    timeout_secs: peer_cfg.partition_timeout_secs,
                });
            }
        };

        self.core.interpret_response(peer, response)
    }

    async fn handle_intake(&self, request: A2AJsonRpcRequest) -> A2AJsonRpcResponse {
        self.core.handle_intake(request).await
    }
}

/// Loopback never binds a socket — `local_addr` is `None` (the default).
impl A2ATransport for LoopbackA2ARouter {}

/// Bridge `LoopbackA2ARouter` to the `maos-domain` `A2ARouter` port trait so
/// `maos-kernel-core::iac::mailbox` can dyn-dispatch through it. Maps
/// `A2AError` variants to typed `IacBusError` sub-variants via the shared
/// `maos_a2a_core::router::map_a2a_error_to_iac_bus`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use maos_a2a_core::config::A2AProfile;
    use maos_a2a_core::error::IntentDirection;
    use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
    use maos_a2a_core::tofu::InMemoryTofuPinStore;
    use maos_a2a_core::ConsentAllowlists;
    use maos_domain::frame::{
        FrameAddress, FramePayload, PosturePreferences, TaskAssignPayload,
    };
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i8::A2AIntent;
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
        consent_ttl_secs: maos_a2a_core::config::DEFAULT_CONSENT_TTL_SECS,
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

    async fn pinned_router(allow: ConsentAllowlists) -> LoopbackA2ARouter {
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
        LoopbackA2ARouter::new(vec![cfg], tofu)
    }

    #[tokio::test]
    async fn outbound_send_admitted_intent_succeeds() {
        let allow = ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new("standard")],
            accept_allowlist: vec![A2AIntent::new("standard")],
        };
        let router = pinned_router(allow).await;
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
        let router = pinned_router(allow).await;
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
        let router = pinned_router(allow).await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        router.install_intake_sink(tx).await;
        let frame = make_frame(Some("loopback"));
        let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
        let resp = router.handle_intake(req).await;
        assert!(matches!(resp, A2AJsonRpcResponse::Ack(_)));
        let delivered = rx.recv().await.expect("delivered to sink");
        assert_eq!(delivered.from.spirit_id.as_str(), "a");
    }
}
