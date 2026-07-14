#![forbid(unsafe_code)]

//! Story 12.4a — cohort digest-read: the no-surveillance MECHANISM.
//!
//! A cross-member "read" travels over the NON-reserved `cohort:digest-read` A2A
//! intent (see [`maos_a2a_core::COHORT_INTENT_DIGEST_READ`]) so it is fully
//! consent-gated at both seams + the cohort role/version overlay (AC1 — a
//! *reserved* read would be an ungated read, the 12.3 P4 trap). The read is ONE
//! consent decision — the target's accept-gate — plus an intrinsic **correlated
//! reply** authorized by that admit, NOT re-gated by a second consent check
//! (AC2). Correlation is proven by a stable [`request_id`](DigestReadControl):
//! the reader mints it, the reply carries it, the digest is idempotent per
//! `request_id` (AC2b — NEVER the resetting envelope `frame_id`).
//!
//! This module owns the wire envelope (rendered onto the existing
//! [`TelemetryEventPayload`] carrier — F1, zero `maos-domain` touch), the
//! composition-owned [`CohortDigestDistributor`] courier, and the production
//! rupture-journal helper that lands a refused read's `ConsentRupture` in the
//! target member's Transparency Log (AC4 — the real F4 fix). The correlation
//! oracle + dedup live on [`crate::state::CohortManifestState`] (the
//! `DigestReadPort` impl).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{ConsentRuptureSink, COHORT_INTENT_DIGEST_READ};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::adapter::{FrameKind as TlFrameKind, TransparencyLogAdapter};
use maos_spirit_abi::identity::{FrameKind, HostId};
use serde::{Deserialize, Serialize};

use crate::error::CohortError;
use crate::state::CohortManifestState;

/// Event-type discriminant carried in the `TelemetryEventPayload` (F1 carrier),
/// mirroring [`crate::control::CONTROL_EVENT_TYPE`] /
/// [`crate::halt_receipt::HALT_RECEIPT_EVENT_TYPE`].
pub const DIGEST_READ_EVENT_TYPE: &str = "maos.cohort-digest-read.v1";
pub const DIGEST_DAILY_SCOPE: &str = "daily";
pub const MAX_DIGEST_REQUEST_ID_LEN: usize = 128;
pub const MAX_DIGEST_SCOPE_LEN: usize = 64;
static NEXT_DIGEST_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DIGEST_REQUEST_ID_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn validate_control(control: &DigestReadControl) -> Result<(), CohortError> {
    if !valid_request_id(control.request_id()) {
        return Err(CohortError::EControlEnvelopeInvalid);
    }
    if let DigestReadControl::Request { scope, .. } = control {
        if scope != DIGEST_DAILY_SCOPE || scope.len() > MAX_DIGEST_SCOPE_LEN {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
    }
    Ok(())
}

/// A member's self-reported daily activity — the typed summary a target chooses
/// to expose in a correlated reply (AC2). Serialized into the existing
/// `TelemetryEvent` carrier so 12.4a stays zero-`maos-domain`-delta; 12.4b
/// renders it into the J3 day-30 narrative digest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigestSummary {
    /// Frames the member processed in the reporting window.
    pub frames: u64,
    /// Halt receipts the member owns in the window.
    pub halts: u64,
    /// Cross-agent conflicts the member observed in the window.
    pub conflicts: u64,
}

/// The `cohort:digest-read` wire envelope: a request naming the scope it wants,
/// or a reply carrying the correlated `request_id` + the target's chosen
/// summary. Rendered onto [`TelemetryEventPayload`] — no new `FramePayload`
/// variant, no new `A2AJsonRpcRequest` field (F1 idiom).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DigestReadControl {
    /// Reader → target: "send me your digest for `scope`". `request_id` is the
    /// stable correlation + replay-dedup key (mirrors 12.3 `halt_id`).
    Request { request_id: String, scope: String },
    /// Target → reader: the correlated reply, tagged with the request's id.
    Reply {
        request_id: String,
        summary: DigestSummary,
    },
}

impl DigestReadControl {
    /// Validate + decode a `cohort:digest-read` frame. Mirrors
    /// [`crate::control::CohortManifestControl::from_frame`] with the
    /// non-reserved digest-read intent + this module's event type.
    pub fn from_frame(frame: &IacFrame) -> Result<Self, CohortError> {
        let intent = frame
            .consent_envelope
            .as_ref()
            .and_then(|envelope| envelope.intent_class.as_ref())
            .map(|intent| intent.as_str());
        if intent != Some(COHORT_INTENT_DIGEST_READ) || frame.kind != FrameKind::TelemetryEvent {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        let FramePayload::TelemetryEvent(payload) = &frame.payload else {
            return Err(CohortError::EControlEnvelopeInvalid);
        };
        if payload.event_type != DIGEST_READ_EVENT_TYPE {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        let control = serde_json::from_str(&payload.data)
            .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))?;
        validate_control(&control)?;
        Ok(control)
    }

    pub fn telemetry_payload(&self) -> Result<TelemetryEventPayload, CohortError> {
        Ok(TelemetryEventPayload {
            event_type: DIGEST_READ_EVENT_TYPE.into(),
            data: serde_json::to_string(self)
                .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))?,
        })
    }

    /// The stable correlation id — the AC2b dedup key, invariant across
    /// re-ships (unlike the resetting per-ship envelope `frame_id`).
    pub fn request_id(&self) -> &str {
        match self {
            Self::Request { request_id, .. } | Self::Reply { request_id, .. } => request_id,
        }
    }
}

/// Composition-owned courier for the `cohort:digest-read` request/reply pair. A
/// structural sibling of [`crate::distribution::CohortDistributor`] /
/// [`crate::halt_receipt::HaltReceiptDistributor`], swapping the intent + event
/// type. It owns no trust decision: the router enforces consent on the request
/// and the correlated-reply exemption on the reply; this type only emits frames.
/// One intent + one correlated reply — NOT a general request/response framework.
pub struct CohortDigestDistributor {
    state: Arc<CohortManifestState>,
    router: Arc<dyn A2APeerRouter>,
    from: FrameAddress,
    next_frame_id: AtomicU64,
}

impl CohortDigestDistributor {
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

    /// Reader side — mint a bounded, host-namespaced request id, publish a
    /// provisional correlation capability, and ship the consent-gated request.
    /// A NACK or transport failure rolls the capability back; only an ACK leaves
    /// it live for the single correlated reply.
    pub async fn request_read(&self, target: &HostId, scope: &str) -> Result<String, CohortError> {
        if scope != DIGEST_DAILY_SCOPE {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        let sequence = NEXT_DIGEST_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let minted_at_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| CohortError::EInvalidDigestRequest(error.to_string()))?
            .as_nanos();
        let host = self
            .from
            .host_id
            .as_ref()
            .map_or("local", |host| host.as_str());
        let request_id = format!("{host}:{minted_at_ns:032x}:{sequence:016x}");
        self.state
            .note_digest_request_sent(target, &request_id, scope)?;
        let control = DigestReadControl::Request {
            request_id: request_id.clone(),
            scope: scope.to_string(),
        };
        let result = async {
            let payload = control.telemetry_payload()?;
            let frame = self.frame_for(target, payload);
            self.router
                .route_outbound(frame, target)
                .await
                .map_err(|error| CohortError::EDistributionFailed(error.to_string()))
        }
        .await;
        if let Err(error) = result {
            self.state.cancel_digest_request(target, &request_id)?;
            return Err(error);
        }
        Ok(request_id)
    }

    /// Target side — ship ONE correlated reply (send-exempt) to `requester`,
    /// tagged with `request_id` + the chosen `summary`. Send-exempt only because
    /// the router's port confirms this host admitted the matching request (AC2);
    /// a reply for an unadmitted `request_id` is denied by the seam. Shipping the
    /// same reply twice is safe — the reader dedups per `request_id` (AC2b).
    pub async fn reply_read(
        &self,
        requester: &HostId,
        request_id: &str,
        summary: &DigestSummary,
    ) -> Result<(), CohortError> {
        let control = DigestReadControl::Reply {
            request_id: request_id.to_string(),
            summary: summary.clone(),
        };
        validate_control(&control)?;
        let payload = control.telemetry_payload()?;
        let frame = self.frame_for(requester, payload);
        self.router
            .route_outbound(frame, requester)
            .await
            .map_err(|error| CohortError::EDistributionFailed(error.to_string()))?;
        self.state
            .complete_admitted_digest_reply(requester, request_id)?;
        Ok(())
    }

    /// Target side — service every reply obligation this host accrued by
    /// admitting a request, shipping each correlated reply (send-exempt) with
    /// the target's chosen `summary`. The reader dedups by `request_id`, so a
    /// re-serviced reply never double-counts.
    pub async fn service_pending_replies(
        &self,
        summary: &DigestSummary,
    ) -> Result<usize, CohortError> {
        let pending = self.state.drain_pending_digest_replies()?;
        let mut shipped = 0usize;
        let mut first_error = None;
        for (requester, request_id, scope) in pending {
            if scope != DIGEST_DAILY_SCOPE {
                self.state
                    .requeue_pending_digest_reply(requester, request_id, scope)?;
                first_error.get_or_insert(CohortError::EControlEnvelopeInvalid);
                continue;
            }
            match self
                .reply_read(&HostId(requester.clone()), &request_id, summary)
                .await
            {
                Ok(()) => shipped += 1,
                Err(error) => {
                    self.state
                        .requeue_pending_digest_reply(requester, request_id, scope)?;
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        first_error.map_or(Ok(shipped), Err)
    }

    fn frame_for(&self, peer: &HostId, payload: TelemetryEventPayload) -> IacFrame {
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
        let envelope = ConsentEnvelope::with_fine_grained_intent(
            self.from.clone(),
            A2AIntent::new(COHORT_INTENT_DIGEST_READ),
        );
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

/// Story 12.4a / AC4 — journal a production-produced `ConsentRupture` frame into
/// the Transparency Log, preserving the router-minted `rupture_id`. This is the
/// receiver-drain endpoint the F4 fix wires: `A2ARouterCore::install_rupture_sink`
/// pushes the rupture (earned by the accept-cohort-deny path, `router.rs:1024`),
/// the composition root drains the channel and calls this. The row is written as
/// `FrameKind::ConsentRupture` on the denier=target host, where the affected
/// member queries it via `maosctl audit query --frame-kind ConsentRupture`.
///
/// Non-rupture frames are ignored (defense-in-depth: only a genuine rupture is
/// journaled here — the sink never hand-writes a row for anything else).
pub fn journal_rupture_frame(log: &TransparencyLogAdapter, frame: &IacFrame) -> Result<(), String> {
    if frame.kind != FrameKind::ConsentRupture {
        return Err("rupture sink received a non-rupture frame".into());
    }
    let intent = frame
        .consent_envelope
        .as_ref()
        .and_then(|envelope| envelope.intent_class.as_ref())
        .map(|intent| intent.as_str())
        .ok_or_else(|| "rupture frame is missing denied intent attribution".to_string())?;
    let payload_bytes = serde_json::to_vec(&frame.payload).map_err(|error| error.to_string())?;
    let to_spirit_id = frame.to.first().map_or("", |a| a.spirit_id.as_str());
    log.insert_frame_event_with_id(
        Some(frame.frame_id),
        TlFrameKind::ConsentRupture,
        0,
        frame.from.spirit_id.as_str(),
        to_spirit_id,
        None,
        intent,
        &payload_bytes,
        frame.auto_marker,
    );
    Ok(())
}

/// Production adapter that durably journals the rupture before returning to
/// the router's deny path.
pub struct CohortRuptureLogSink {
    log: Arc<TransparencyLogAdapter>,
}

impl CohortRuptureLogSink {
    pub fn new(log: Arc<TransparencyLogAdapter>) -> Self {
        Self { log }
    }
}

impl ConsentRuptureSink for CohortRuptureLogSink {
    fn append(&self, frame: &IacFrame) -> Result<(), String> {
        journal_rupture_frame(&self.log, frame)
    }
}
