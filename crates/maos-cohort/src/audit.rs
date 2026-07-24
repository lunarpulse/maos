#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::{FrameKind, TransparencyLogAdapter};

use crate::error::CohortError;

/// A required, fail-closed append target for cohort manifest lifecycle events.
///
/// Implementations must durably append before returning `Ok(())`; callers never
/// publish a reissue or return a rejection after an append failure.
pub trait CohortAuditSink: Send + Sync {
    fn append(&self, event: &CohortAuditEvent) -> Result<(), CohortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CohortAuditEvent {
    AuthorityReissueIssued {
        cohort_id: String,
        version: u64,
        canonical_hash: [u8; 32],
    },
    MemberReissueAccepted {
        cohort_id: String,
        version: u64,
        canonical_hash: [u8; 32],
    },
    ReissueRejected {
        cohort_id: String,
        seen_version: u64,
        rejected_version: u64,
        reason: String,
    },
    /// Story 12.4a — a consent-gated `cohort:digest-read` request was ADMITTED
    /// by this host's accept-gate (the single consent decision). Journaled to
    /// the I2 TL so the cross-member read is auditable (AC2).
    DigestReadRequested {
        requester: String,
        request_id: String,
        scope: String,
    },
    /// Story 12.4a — a correlated digest-read reply was received + recorded
    /// (idempotent per `request_id`). Journaled once per distinct reply (AC2).
    DigestReplyReceived { member: String, request_id: String },
}

/// Deterministic in-memory implementation for focused state tests. Production
/// composition must inject the out-of-kernel Transparency Log adapter.
#[derive(Debug, Default)]
pub struct InMemoryCohortAuditSink {
    events: Mutex<Vec<CohortAuditEvent>>,
}

impl InMemoryCohortAuditSink {
    pub fn events(&self) -> Vec<CohortAuditEvent> {
        self.events
            .lock()
            .expect("cohort audit lock poisoned")
            .clone()
    }
}

impl CohortAuditSink for InMemoryCohortAuditSink {
    fn append(&self, event: &CohortAuditEvent) -> Result<(), CohortError> {
        self.events
            .lock()
            .map_err(|_| CohortError::EAuditAppendFailed("cohort audit lock poisoned".into()))?
            .push(event.clone());
        Ok(())
    }
}

/// The concrete out-of-kernel I2 Transparency Log writer for cohort lifecycle
/// records. It logs only bounded identifiers, versions, outcomes, and hashes;
/// neither raw manifests nor private signing material are written.
pub struct CohortTransparencyLogSink {
    log: Arc<TransparencyLogAdapter>,
}

impl CohortTransparencyLogSink {
    pub fn new(log: Arc<TransparencyLogAdapter>) -> Self {
        Self { log }
    }
}

impl CohortAuditSink for CohortTransparencyLogSink {
    fn append(&self, event: &CohortAuditEvent) -> Result<(), CohortError> {
        // Story 13-5f: a cross-team digest action carries its `request_id` as the
        // stable correlation ID onto BOTH team logs (target admits the read,
        // requester records the reply), so a reader holding both physical
        // artifacts can reconcile the two halves (AC3 end-to-end). Non-digest
        // lifecycle events keep a NULL correlation.
        let (intent, payload, correlation): (&str, String, Option<&str>) = match event {
            CohortAuditEvent::AuthorityReissueIssued {
                cohort_id,
                version,
                canonical_hash,
            } => (
                "cohort:manifest-audit",
                format!(
                    "{{\"event\":\"authority_reissue_issued\",\"cohort_id\":{cohort_id:?},\"version\":{version},\"canonical_hash\":\"{}\"}}",
                    hex::encode(canonical_hash)
                ),
                None,
            ),
            CohortAuditEvent::MemberReissueAccepted {
                cohort_id,
                version,
                canonical_hash,
            } => (
                "cohort:manifest-audit",
                format!(
                    "{{\"event\":\"member_reissue_accepted\",\"cohort_id\":{cohort_id:?},\"version\":{version},\"canonical_hash\":\"{}\"}}",
                    hex::encode(canonical_hash)
                ),
                None,
            ),
            CohortAuditEvent::ReissueRejected {
                cohort_id,
                seen_version,
                rejected_version,
                reason,
            } => (
                "cohort:manifest-audit",
                format!(
                    "{{\"event\":\"reissue_rejected\",\"cohort_id\":{cohort_id:?},\"seen_version\":{seen_version},\"rejected_version\":{rejected_version},\"reason\":{reason:?}}}"
                ),
                None,
            ),
            CohortAuditEvent::DigestReadRequested {
                requester,
                request_id,
                scope,
            } => (
                "cohort:digest-audit",
                format!(
                    "{{\"event\":\"digest_read_requested\",\"requester\":{requester:?},\"request_id\":{request_id:?},\"scope\":{scope:?}}}"
                ),
                Some(request_id.as_str()),
            ),
            CohortAuditEvent::DigestReplyReceived { member, request_id } => (
                "cohort:digest-audit",
                format!(
                    "{{\"event\":\"digest_reply_received\",\"member\":{member:?},\"request_id\":{request_id:?}}}"
                ),
                Some(request_id.as_str()),
            ),
        };
        match correlation {
            Some(correlation_id) => {
                self.log.insert_frame_event_with_correlation(
                    FrameKind::TelemetryEvent,
                    0,
                    None,
                    correlation_id,
                    intent,
                    payload.as_bytes(),
                    FrameOrigin::Kernel,
                );
            }
            None => {
                self.log.insert_frame_event(
                    FrameKind::TelemetryEvent,
                    0,
                    None,
                    intent,
                    payload.as_bytes(),
                    FrameOrigin::Kernel,
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maos_domain::team::TeamId;

    use maos_iac::adapter::{reconcile_correlated_frames, TransparencyLogAdapter};

    use super::*;

    #[test]
    fn transparency_sink_persists_a_reissue_rejection() {
        let log = Arc::new(TransparencyLogAdapter::open_in_memory(12_100));
        let sink = CohortTransparencyLogSink::new(Arc::clone(&log));

        sink.append(&CohortAuditEvent::ReissueRejected {
            cohort_id: "cohort-a".into(),
            seen_version: 4,
            rejected_version: 4,
            reason: "concurrent_fork".into(),
        })
        .unwrap();

        assert_ne!(log.last_frame_id(), [0; 16]);
    }
    #[test]
    fn digest_cross_team_action_correlates_via_request_id() {
        // 13-5f proven-red: a cross-team digest action writes the SAME request_id
        // as the correlation_id on BOTH team logs, so reconcile_correlated_frames
        // re-joins them; a NULL-correlation row (a non-digest event) is excluded.
        let target = Arc::new(TransparencyLogAdapter::open_in_memory(1));
        let requester = Arc::new(TransparencyLogAdapter::open_in_memory(2));
        let target_sink = CohortTransparencyLogSink::new(Arc::clone(&target));
        let requester_sink = CohortTransparencyLogSink::new(Arc::clone(&requester));

        target_sink
            .append(&CohortAuditEvent::DigestReadRequested {
                requester: "support".into(),
                request_id: "req-42".into(),
                scope: "daily".into(),
            })
            .unwrap();
        requester_sink
            .append(&CohortAuditEvent::DigestReplyReceived {
                member: "security".into(),
                request_id: "req-42".into(),
            })
            .unwrap();

        let security = TeamId::new("security").unwrap();
        let support = TeamId::new("support").unwrap();
        let reconciled =
            reconcile_correlated_frames(&[(&security, &target), (&support, &requester)], "req-42")
                .unwrap();
        assert_eq!(
            reconciled.len(),
            2,
            "both halves of the cross-team action reconcile by request_id"
        );

        // A wrong correlation_id reconciles nothing.
        assert!(reconcile_correlated_frames(
            &[(&security, &target), (&support, &requester)],
            "req-OTHER"
        )
        .unwrap()
        .is_empty());

        // A NULL-correlation row (a non-digest event) cannot participate.
        target_sink
            .append(&CohortAuditEvent::AuthorityReissueIssued {
                cohort_id: "c".into(),
                version: 1,
                canonical_hash: [0u8; 32],
            })
            .unwrap();
        let after_null =
            reconcile_correlated_frames(&[(&security, &target), (&support, &requester)], "req-42")
                .unwrap();
        assert_eq!(
            after_null.len(),
            2,
            "the NULL-correlation reissue row must be excluded"
        );
    }
}
