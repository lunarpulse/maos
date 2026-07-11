#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maos_a2a_core::router::A2APeerRouter;
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId};

use crate::control::{CohortManifestControl, CONTROL_EVENT_TYPE};
use crate::error::CohortError;
use crate::state::CohortManifestState;
use crate::RESERVED_INTENT_REISSUE;

/// Composition-owned courier for the signed manifest control plane. It owns no
/// trust decision: state verifies pushes and the shared router authenticates the
/// peer. This type only emits normal A2A control frames.
pub struct CohortDistributor {
    state: Arc<CohortManifestState>,
    router: Arc<dyn A2APeerRouter>,
    from: FrameAddress,
    next_frame_id: AtomicU64,
}

impl CohortDistributor {
    pub fn new(
        state: Arc<CohortManifestState>,
        router: Arc<dyn A2APeerRouter>,
        from: FrameAddress,
    ) -> Self {
        Self {
            state,
            router,
            from,
            next_frame_id: AtomicU64::new(1),
        }
    }

    /// Send the exact signed artifact previously verified by this member.
    pub async fn push_to(&self, peer: &HostId) -> Result<(), CohortError> {
        let frame = self.frame_for(
            peer,
            CohortManifestControl::Push {
                manifest_toml: self.state.signed_toml()?,
            },
        )?;
        self.router
            .route_outbound(frame, peer)
            .await
            .map_err(|error| CohortError::EDistributionFailed(error.to_string()))
    }

    /// Ask a verified peer for its signed current manifest. The original
    /// application caller remains responsible for explicit resubmission after
    /// a successful refresh; this courier never retries ordinary work.
    pub async fn pull_from(&self, peer: &HostId) -> Result<(), CohortError> {
        let frame = self.frame_for(
            peer,
            CohortManifestControl::Pull {
                known_version: self.state.version()?,
                known_hash: hex::encode(self.state.canonical_hash()?),
            },
        )?;
        self.router
            .route_outbound(frame, peer)
            .await
            .map_err(|error| CohortError::EDistributionFailed(error.to_string()))
    }

    /// Service all verified pull requests that the router queued while handling
    /// inbound control frames. A failed push is returned to the composition
    /// owner; it never makes the requester look fresh.
    pub async fn service_pending_pulls(&self) -> Result<usize, CohortError> {
        let peers = self.state.take_pull_requests()?;
        let count = peers.len();
        for peer in peers {
            self.push_to(&peer).await?;
        }
        Ok(count)
    }

    fn frame_for(
        &self,
        peer: &HostId,
        control: CohortManifestControl,
    ) -> Result<IacFrame, CohortError> {
        let payload = control.telemetry_payload()?;
        let mut frame_id = [0u8; 16];
        frame_id[8..].copy_from_slice(&self.next_frame_id.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        let recipient = FrameAddress {
            spirit_id: self.from.spirit_id.clone(),
            host_id: Some(peer.clone()),
            role: None,
        };
        let mut recipients = smallvec::SmallVec::new();
        recipients.push(recipient);
        let envelope = ConsentEnvelope::with_fine_grained_intent(
            self.from.clone(),
            A2AIntent::new(RESERVED_INTENT_REISSUE),
        );
        debug_assert_eq!(payload.event_type, CONTROL_EVENT_TYPE);
        Ok(IacFrame {
            frame_id,
            timestamp_ns: 0,
            logical_clock: 0,
            from: self.from.clone(),
            to: recipients,
            kind: FrameKind::TelemetryEvent,
            intent: IntentClass::Readonly,
            payload: FramePayload::TelemetryEvent(TelemetryEventPayload {
                event_type: payload.event_type,
                data: payload.data,
            }),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(envelope),
            intent_lineage: IntentLineage::default(),
        })
    }
}
