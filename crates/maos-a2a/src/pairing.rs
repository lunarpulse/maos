#![forbid(unsafe_code)]

//! Bilateral loopback pairing — one router carrying **both** endpoints.
//!
//! `LoopbackA2ARouter::new` takes its peer configs at construction and there is
//! no post-construction peer-add, so an in-process exchange needs a single router
//! instance that knows both sides:
//!
//! * outbound resolves the **destination** host and checks its `send_allowlist`
//!   (`A2ARouterCore::prepare_outbound`);
//! * intake resolves the **source** host from `frame.from.host_id` and checks
//!   THAT peer's `accept_allowlist` (`A2ARouterCore::handle_intake`).
//!
//! Every caller that got this wrong keyed the accept side by the destination. The
//! helper below makes the asymmetry explicit and does the TOFU first-contact pins
//! that both directions require before use.
//!
//! Lives here rather than in a consumer because it is pure `maos-a2a` surface:
//! `maos-a2a-core` has zero KLOC headroom and must not grow, and duplicating this
//! prologue per consumer is how the send/accept asymmetry drifts.

use std::sync::Arc;

use maos_a2a_core::config::{A2APeerConfig, A2AProfile, DEFAULT_CONSENT_TTL_SECS};
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_core::{ConsentAllowlists, A2AError};
use maos_domain::frame::IacFrame;
use maos_domain::invariants::i8::A2AIntent;

use crate::adapter::LoopbackA2ARouter;

/// One endpoint of a loopback pair. `peer_id` MUST equal the `HostId` string the
/// frames carry, because both the outbound and the intake lookups are keyed by it.
#[derive(Debug, Clone)]
pub struct LoopbackEndpoint {
    pub host: String,
    pub port: u16,
    pub send_allowlist: Vec<A2AIntent>,
    pub accept_allowlist: Vec<A2AIntent>,
}

impl LoopbackEndpoint {
    /// An endpoint that may only SEND `intent` (the delegation destination).
    pub fn sender_of(host: &str, port: u16, intent: &A2AIntent) -> Self {
        Self {
            host: host.to_string(),
            port,
            send_allowlist: vec![intent.clone()],
            accept_allowlist: Vec::new(),
        }
    }

    /// An endpoint that may only ACCEPT `intent` (the delegation source, because
    /// on loopback the accept allowlist is keyed by `frame.from.host_id`).
    pub fn acceptor_of(host: &str, port: u16, intent: &A2AIntent) -> Self {
        Self {
            host: host.to_string(),
            port,
            send_allowlist: Vec::new(),
            accept_allowlist: vec![intent.clone()],
        }
    }

    fn config(&self) -> A2APeerConfig {
        A2APeerConfig {
            peer_id: PeerId::new(&self.host),
            endpoint: format!("tls://127.0.0.1:{}", self.port),
            cert_fingerprint: PeerCertFingerprint::from_cert_der(self.host.as_bytes()),
            profile: A2AProfile::Loopback,
            allowlists: ConsentAllowlists {
                send_allowlist: self.send_allowlist.clone(),
                accept_allowlist: self.accept_allowlist.clone(),
            },
            partition_timeout_secs: 30,
            consent_ttl_secs: DEFAULT_CONSENT_TTL_SECS,
        }
    }
}

/// Build a loopback router carrying every supplied endpoint, validate each config,
/// TOFU-pin all of them, and install an intake sink.
///
/// Returns the router plus the receiver a caller pumps: the loopback "wire" pushes
/// accepted frames onto it **after** every validation passes, so a frame arriving
/// here is a frame the peer admitted.
pub async fn paired_loopback_router(
    endpoints: &[LoopbackEndpoint],
) -> Result<
    (
        Arc<LoopbackA2ARouter>,
        tokio::sync::mpsc::UnboundedReceiver<IacFrame>,
    ),
    A2AError,
> {
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    let mut configs = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        let cfg = endpoint.config();
        cfg.validate()?;
        tofu.pin_first_contact(&cfg.peer_id, &cfg.cert_fingerprint, &cfg.cert_fingerprint, 1)
            .await?;
        configs.push(cfg);
    }
    let router = Arc::new(LoopbackA2ARouter::new(configs, tofu));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    router.install_intake_sink(tx).await;
    Ok((router, rx))
}
