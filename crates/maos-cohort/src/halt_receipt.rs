#![forbid(unsafe_code)]

//! Story 12.3 — halt-receipt shipping courier, control envelope, and
//! receipt-presence classifier.
//!
//! The kernel halt path is UNCHANGED: `invoke_halt` / `terminate_spirit`
//! already produce and locally journal (I2) a [`HaltReceipt`]. This module adds
//! only the out-of-kernel *shipping* (as the reserved `cohort:halt-receipt`
//! intent) and the *classifier* that turns a transport probe into first-class
//! receipt-presence/absence. `maos-cohort` does NOT depend on
//! `maos-kernel-core`, so the arbitration sink (`HaltRegistry::resolve` /
//! `KernelHaltResolver`) is graph-unreachable from here — the courier can ship
//! and observe but never resolve, resume, or override a halt (AC3).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::A2AError;
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::halt::HaltReceipt;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId};
use serde::{Deserialize, Serialize};

use crate::control::CohortManifestControl;
use crate::error::CohortError;
use crate::state::CohortManifestState;
use crate::{RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE};

/// Event-type discriminant carried in the `TelemetryEventPayload` (F1 carrier).
/// The `HaltReceiptControl` clone of `CohortManifestControl` swaps exactly this
/// constant + the reserved intent (P8 — structurally identical, not copy-paste).
pub const HALT_RECEIPT_EVENT_TYPE: &str = "maos.cohort-halt-receipt.v1";

/// The shipped envelope: the exact locally-journaled [`HaltReceipt`], rendered
/// onto the existing [`TelemetryEventPayload`] carrier (F1 — zero `maos-domain`
/// touch: no new `FramePayload` variant, no new `A2AJsonRpcRequest` field).
///
/// The emitting member is deliberately NOT carried here (P5): it is derived at
/// ingest from the authenticated `frame.from.host_id`. An `emitter` payload
/// field would introduce a second, unauthenticated source of identity (11.2a
/// origin ≠ authorization).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HaltReceiptControl {
    pub receipt: HaltReceipt,
}

impl HaltReceiptControl {
    pub fn new(receipt: HaltReceipt) -> Self {
        Self { receipt }
    }

    /// Validate + decode a halt-receipt control frame. Mirrors
    /// [`CohortManifestControl::from_frame`] with the two hardcoded constants
    /// swapped, so the wire contract stays identical.
    pub fn from_frame(frame: &IacFrame) -> Result<Self, CohortError> {
        let intent = frame
            .consent_envelope
            .as_ref()
            .and_then(|envelope| envelope.intent_class.as_ref())
            .map(|intent| intent.as_str());
        if intent != Some(RESERVED_INTENT_HALT_RECEIPT) || frame.kind != FrameKind::TelemetryEvent {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        let FramePayload::TelemetryEvent(payload) = &frame.payload else {
            return Err(CohortError::EControlEnvelopeInvalid);
        };
        if payload.event_type != HALT_RECEIPT_EVENT_TYPE {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        serde_json::from_str(&payload.data)
            .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))
    }

    pub fn telemetry_payload(&self) -> Result<TelemetryEventPayload, CohortError> {
        Ok(TelemetryEventPayload {
            event_type: HALT_RECEIPT_EVENT_TYPE.into(),
            data: serde_json::to_string(self)
                .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))?,
        })
    }

    /// The receipt's stable identity — the dedup key (P4). `halt_id` is set once
    /// at `invoke_halt` and is invariant across re-ships, unlike the per-ship
    /// A2A envelope `frame_id`.
    pub fn halt_id(&self) -> &str {
        self.receipt.halt_id.as_str()
    }
}

/// Why a member is classified ABSENT under an induced loss (P3/P2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsenceKind {
    /// Clean transport down — connection refused / EOF (`A2AError::Io`).
    MemberLoss,
    /// Reachable-then-partitioned — §7.2 handshake/idle timeout
    /// (`A2AError::TransportFailed` with a `"timeout:"` message).
    ConnectivityLoss,
}

/// The receipt-presence classification of a probe result (P2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltPresence {
    /// The member responded — `Ok` OR any mapped application NACK (all prove
    /// the peer is UP).
    Present,
    /// The member is absent, backed by a precise transport marker.
    Absent(AbsenceKind),
    /// An indeterminate transport result that MUST NOT be read as absence — a
    /// bare/unmapped `TransportFailed` (an up peer's unknown NACK) or a
    /// handshake/config failure (P2a).
    Indeterminate,
}

/// Classify a reserved-manifest-PULL probe result into receipt-presence.
///
/// - **PRESENT** = `Ok(())` ∨ any mapped application-NACK variant (the peer
///   answered → provably UP).
/// - **ABSENT** = `A2AError::Io` (clean member loss) ∨ `A2AError::TransportFailed`
///   whose message starts with `"timeout:"` (§7.2 connectivity loss).
/// - **INDETERMINATE** = everything else (a bare `TransportFailed`, a handshake
///   failure, `PartitionTimeout` (loopback-only), config, …). NEVER silently
///   "absent": a bare `TransportFailed` aliases a live peer's unknown NACK
///   (P2a), and `PartitionTimeout` is dead on the wire (P3).
pub fn classify_probe_result(result: &Result<(), A2AError>) -> HaltPresence {
    match result {
        Ok(()) => HaltPresence::Present,
        Err(A2AError::Io(_)) => HaltPresence::Absent(AbsenceKind::MemberLoss),
        Err(A2AError::TransportFailed(message)) if message.starts_with("timeout:") => {
            HaltPresence::Absent(AbsenceKind::ConnectivityLoss)
        }
        // Mapped application NACKs — the peer answered, so it is UP → PRESENT.
        Err(
            A2AError::IntentDeniedAtPeer { .. }
            | A2AError::ConsentExpired { .. }
            | A2AError::PinInvalidated { .. }
            | A2AError::PeerIdentityMismatch { .. }
            | A2AError::ConsentGranterMismatch { .. }
            | A2AError::ConsentUnclassifiedAtPeer { .. },
        ) => HaltPresence::Present,
        // Bare TransportFailed / HandshakeFailed / ConfigInvalid / PartitionTimeout / …
        // are indeterminate — NOT absence (P2a/P3).
        Err(_) => HaltPresence::Indeterminate,
    }
}

/// Composition-owned courier that ships the locally-produced [`HaltReceipt`] to
/// cohort peers as the reserved `cohort:halt-receipt` intent, and probes a
/// missing member for receipt-presence. A structural clone of
/// [`crate::distribution::CohortDistributor`] (P8 — swaps the intent + event
/// type). It owns no trust decision and never retries ordinary work.
pub struct HaltReceiptDistributor {
    state: Arc<CohortManifestState>,
    router: Arc<dyn A2APeerRouter>,
    from: FrameAddress,
    next_frame_id: AtomicU64,
}

impl HaltReceiptDistributor {
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

    /// Ship the exact locally-journaled receipt to one peer as the reserved
    /// `cohort:halt-receipt` intent.
    pub async fn push_receipt_to(
        &self,
        peer: &HostId,
        receipt: &HaltReceipt,
    ) -> Result<(), CohortError> {
        let payload = HaltReceiptControl::new(receipt.clone()).telemetry_payload()?;
        let frame = self.frame_for(peer, RESERVED_INTENT_HALT_RECEIPT, payload);
        self.router
            .route_outbound(frame, peer)
            .await
            .map_err(|error| CohortError::EDistributionFailed(error.to_string()))
    }

    /// Fan the receipt out to every OTHER roster member.
    ///
    /// **(P2) This is the SHIP path only — it does NOT detect member absence.**
    /// A per-peer `route_outbound` Err to a down peer P is a *P-not-receiving*
    /// signal attributed to the dialed peer, NEVER an "M is absent" marker;
    /// absence is a PROBE ([`Self::classify_presence`]).
    pub async fn broadcast(&self, receipt: &HaltReceipt) -> Result<usize, CohortError> {
        let manifest = self.state.manifest()?;
        let local = self.from.host_id.as_ref().map(|host| host.as_str());
        let mut shipped = 0usize;
        let mut first_error = None;
        for member in &manifest.members {
            if Some(member.host_id.as_str()) == local {
                continue;
            }
            let peer = HostId(member.host_id.clone());
            match self.push_receipt_to(&peer, receipt).await {
                Ok(()) => shipped += 1,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        first_error.map_or(Ok(shipped), Err)
    }

    /// Actively probe a member for receipt-presence (P2/P2a/P2b).
    ///
    /// There is NO passive liveness anywhere in the transport, so absence only
    /// exists as the return value of a probe. The probe is a reserved
    /// manifest-PULL — it bypasses both consent seams, so no consent verdict can
    /// confound the result — dialed AT the member whose receipt is missing (a
    /// failed *broadcast* to a different peer is never an M-absent marker, P2).
    /// One dial, zero-retry; classification is precise (P2a).
    pub async fn classify_presence(&self, member: &HostId) -> Result<HaltPresence, CohortError> {
        let control = CohortManifestControl::Pull {
            known_version: self.state.version()?,
            known_hash: hex::encode(self.state.canonical_hash()?),
        };
        let payload = control.telemetry_payload()?;
        let frame = self.frame_for(member, RESERVED_INTENT_REISSUE, payload);
        let result = self.router.route_outbound(frame, member).await;
        let presence = classify_probe_result(&result);
        if let HaltPresence::Absent(kind) = presence {
            self.state.record_absence(member, kind);
        }
        Ok(presence)
    }

    fn frame_for(&self, peer: &HostId, intent: &str, payload: TelemetryEventPayload) -> IacFrame {
        let mut frame_id = [0u8; 16];
        frame_id[8..].copy_from_slice(
            &self
                .next_frame_id
                .fetch_add(1, Ordering::Relaxed)
                .to_be_bytes(),
        );
        let recipient = FrameAddress {
            spirit_id: self.from.spirit_id.clone(),
            host_id: Some(peer.clone()),
            role: None,
        };
        let mut recipients = smallvec::SmallVec::new();
        recipients.push(recipient);
        let envelope =
            ConsentEnvelope::with_fine_grained_intent(self.from.clone(), A2AIntent::new(intent));
        IacFrame {
            frame_id,
            timestamp_ns: 0,
            logical_clock: 0,
            from: self.from.clone(),
            to: recipients,
            kind: FrameKind::TelemetryEvent,
            intent: IntentClass::Readonly,
            payload: FramePayload::TelemetryEvent(payload),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(envelope),
            intent_lineage: IntentLineage::default(),
        }
    }
}
