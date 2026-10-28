//! Story 19-7 (AC2/AC3, D-19-7-01…08 A) — the re-pin consent door: the first
//! production caller of `TofuPinStore::await_repin_consent`, and the detection
//! journal that makes a refused restarted peer visible on the receiving host.
//!
//! In the LIBRARY, not `main.rs`, on this crate's doctrine (see `lib.rs`): the
//! operator door's port lives here, and `crates/maos-bin/tests/` must drive the
//! REAL handler against a real pin store rather than a re-implementation.
//!
//! The control is SET-ONCE, on the [`crate::cert_rotation`] pattern: the door
//! is constructed in every door root thousands of lines before any transport
//! exists, and only `run_cohort_a2a_daemon` — the one root where the concrete
//! `TcpA2ATransport` and its `pins()` survive — installs the store. Every
//! other root (`maos run`, `maos shell`) keeps an empty control and answers
//! 404, never an empty list (CR-1).
//!
//! Fail-closed by construction (CR-1/CR-2): nothing here writes operator
//! config, the route carries no fingerprint, the Approval Decision Log row is
//! written BEFORE the pin can change, and a row that cannot be written
//! withdraws the recorded verdict so the pin stays invalidated.

use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use maos_a2a_core::tofu::{
    InMemoryTofuPinStore, RePinDecision, RepinDecisionError, RepinRequest, RepinSink, RepinVerdict,
    TofuPinStore as _,
};
use maos_a2a_core::PeerId;
use maos_control::{OperatorOutcome, RepinRequestRow};
use maos_domain::invariants::i4::ApprovalDecision;

/// The I4 capability every re-pin decision row carries.
pub const REPIN_CAPABILITY: &str = "a2a.repin";
/// The I4 intent every re-pin decision row carries.
pub const REPIN_INTENT: &str = "nfr-rel-6.restart-repin";
/// The Transparency-Log intent of the per-detection row (D-19-7-08 A).
pub const REPIN_REQUESTED_INTENT: &str = "a2a:repin-requested";
/// Detection rows queued for the journaling task. Bounded like every other
/// peer-driven queue on this root; a full queue drops the ROW (announced on
/// stderr), never the pending request, which `GET /v1/a2a/repins` still lists.
const DETECTION_QUEUE: usize = 64;

/// The set-once re-pin control the operator door holds.
#[derive(Default)]
pub struct RepinControl {
    pins: OnceLock<Arc<InMemoryTofuPinStore>>,
    /// Decisions are serialized (a superset of per-peer): `record → I4 row →
    /// consent` must not interleave with another decision, so a later record
    /// can never overwrite a verdict whose audit row is still being written.
    decisions: tokio::sync::Mutex<()>,
}

impl RepinControl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the LIVE transport's pin store. `false`: one is already
    /// installed and `pins` is dropped.
    #[must_use]
    pub fn install(&self, pins: Arc<InMemoryTofuPinStore>) -> bool {
        self.pins.set(pins).is_ok()
    }

    /// Every pending request, sorted by peer. `None` when no store is
    /// installed in this process (the route answers 404).
    pub fn pending(&self) -> Option<Vec<RepinRequestRow>> {
        let pins = self.pins.get()?;
        let now = SystemTime::now();
        let mut rows: Vec<RepinRequestRow> = pins
            .pending_repins()
            .into_iter()
            .map(|request| RepinRequestRow {
                peer: request.peer.as_str().to_owned(),
                pinned_fingerprint: request.pinned_fingerprint.wire(),
                prior_boot_nonce: request.prior_boot_nonce,
                observed_boot_nonce: request.observed_boot_nonce,
                detected_at_secs: unix_secs(request.detected_at),
                expires_at_secs: unix_secs(request.expires_at),
                expired: now >= request.expires_at,
            })
            .collect();
        rows.sort_by(|left, right| left.peer.cmp(&right.peer));
        Some(rows)
    }

    /// One operator decision: record the verdict on the pending request
    /// (nonce echo, D-19-7-03 A), write the I4 row through `journal` (CR-2),
    /// then let `await_repin_consent` consume it against the request's OWN
    /// pinned fingerprint and observed nonce (D-19-7-04/05 A).
    pub async fn decide(
        &self,
        peer: &str,
        boot_nonce: u64,
        approve: bool,
        actor: &str,
        journal: impl FnOnce(ApprovalDecision) -> Result<(), String>,
    ) -> OperatorOutcome {
        let Some(pins) = self.pins.get() else {
            return OperatorOutcome::NotFound {
                code: "not_found".into(),
                detail: "this process holds no re-pin control; only the cohort-a2a-daemon root \
                         owns the receiving pin store"
                    .into(),
            };
        };
        let _serialized = self.decisions.lock().await;
        let mut approval_id = [0_u8; 16];
        if let Err(error) = getrandom::fill(&mut approval_id) {
            return OperatorOutcome::Failed {
                code: "entropy_unavailable".into(),
                detail: format!("no approval id could be minted: {error}; nothing was recorded"),
            };
        }
        let approval_hex = hex::encode(approval_id);
        let peer_id = PeerId::new(peer);
        let verdict = if approve {
            RepinVerdict::Approve { approval_id }
        } else {
            RepinVerdict::Reject {
                reason: format!("rejected by operator {actor} (approval id {approval_hex})"),
            }
        };
        let request = match pins.record_repin_decision(&peer_id, boot_nonce, verdict) {
            Ok(request) => request,
            Err(error) => return refusal(&error),
        };
        if let Err(error) = journal(approval_row(&request, approve, actor, &approval_hex)) {
            pins.withdraw_repin_decision(&peer_id);
            return OperatorOutcome::Failed {
                code: "audit_unavailable".into(),
                detail: format!(
                    "the Approval Decision Log row could not be written ({error}); the decision \
                     was withdrawn and the pin is unchanged"
                ),
            };
        }
        let decision = match pins
            .await_repin_consent(
                &peer_id,
                &request.pinned_fingerprint,
                request.observed_boot_nonce,
            )
            .await
        {
            RePinDecision::AcceptedByOperator { .. } => "accepted",
            RePinDecision::RejectedByOperator { .. } => "rejected",
            RePinDecision::RejectedByStore { error } => {
                return OperatorOutcome::Conflict {
                    code: "repin_refused_by_store".into(),
                    detail: format!("{error}; the pin stays invalidated"),
                };
            }
            RePinDecision::TimedOut => {
                return OperatorOutcome::Conflict {
                    code: "repin_request_expired".into(),
                    detail: "the request expired or was superseded by a newer detection before \
                             the consent applied; the pin stays invalidated"
                        .into(),
                };
            }
        };
        OperatorOutcome::Completed(serde_json::json!({
            "peer": peer,
            "boot_nonce": request.observed_boot_nonce.to_string(),
            "approval_id": approval_hex,
            "decision": decision,
        }))
    }
}

/// D-19-7-08 A — spawn the task that journals ONE `a2a:repin-requested`
/// `TelemetryEvent` row per new or superseding detection, and return the
/// sink to install on the live store. The sink never blocks intake: it only
/// `try_send`s. Must be called inside a Tokio runtime.
///
/// Row: kind `TelemetryEvent`, pid 0, no token, origin `Kernel`, intent
/// [`REPIN_REQUESTED_INTENT`], payload `{"event":"repin_requested","peer",
/// "prior_boot_nonce","observed_boot_nonce","detected_at","expires_at"}`
/// (nonces decimal strings, times unix secs). The fingerprint is NOT in the
/// row: the Transparency Log's redaction policy scrubs 64-hex strings as
/// token-shaped, so it would land as a redaction marker; `GET /v1/a2a/repins`
/// and the I4 decision row carry it.
pub fn spawn_detection_journal(
    transparency_log: Arc<maos_iac::TransparencyLogAdapter>,
) -> RepinSink {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<RepinRequest>(DETECTION_QUEUE);
    tokio::spawn(async move {
        while let Some(request) = receiver.recv().await {
            let payload = serde_json::json!({
                "event": "repin_requested",
                "peer": request.peer.as_str(),
                "prior_boot_nonce": request.prior_boot_nonce.to_string(),
                "observed_boot_nonce": request.observed_boot_nonce.to_string(),
                "detected_at": unix_secs(request.detected_at),
                "expires_at": unix_secs(request.expires_at),
            });
            let _logged = transparency_log.insert_frame_event(
                maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent,
                0,
                None,
                REPIN_REQUESTED_INTENT,
                payload.to_string().as_bytes(),
                maos_domain::invariants::i3::FrameOrigin::Kernel,
            );
        }
    });
    Arc::new(move |request: &RepinRequest| {
        if let Err(error) = sender.try_send(request.clone()) {
            eprintln!(
                "maos: {REPIN_REQUESTED_INTENT} row for peer {} dropped ({error}); the request \
                 is still listed by `maosctl a2a repin list`",
                request.peer
            );
        }
    })
}

/// The I4 Approval Decision Log row (AC3.3).
fn approval_row(
    request: &RepinRequest,
    approve: bool,
    actor: &str,
    approval_hex: &str,
) -> ApprovalDecision {
    ApprovalDecision {
        actor: actor.to_owned(),
        target: request.peer.as_str().to_owned(),
        capability: REPIN_CAPABILITY.to_owned(),
        intent: REPIN_INTENT.to_owned(),
        decision: approve,
        reasoning: Some(format!(
            "approval_id={approval_hex}; prior_boot_nonce={}; observed_boot_nonce={}; \
             pinned_fingerprint={}",
            request.prior_boot_nonce,
            request.observed_boot_nonce,
            request.pinned_fingerprint.wire()
        )),
    }
}

/// A verdict the store refused to record; nothing changed.
fn refusal(error: &RepinDecisionError) -> OperatorOutcome {
    let code = match error {
        RepinDecisionError::NotPending { .. } => {
            return OperatorOutcome::NotFound {
                code: "repin_not_pending".into(),
                detail: error.to_string(),
            };
        }
        RepinDecisionError::NonceMismatch { .. } => "repin_nonce_mismatch",
        RepinDecisionError::Expired { .. } => "repin_request_expired",
    };
    OperatorOutcome::Conflict {
        code: code.into(),
        detail: format!("{error}; the pin stays invalidated"),
    }
}

fn unix_secs(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
