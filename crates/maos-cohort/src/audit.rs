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
}

/// Deterministic in-memory implementation for focused state tests. Production
/// composition must inject the out-of-kernel Transparency Log adapter.
#[derive(Debug, Default)]
pub struct InMemoryCohortAuditSink {
    events: Mutex<Vec<CohortAuditEvent>>,
}

impl InMemoryCohortAuditSink {
    pub fn events(&self) -> Vec<CohortAuditEvent> {
        self.events.lock().expect("cohort audit lock poisoned").clone()
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
        let payload = match event {
            CohortAuditEvent::AuthorityReissueIssued {
                cohort_id,
                version,
                canonical_hash,
            } => format!(
                "{{\"event\":\"authority_reissue_issued\",\"cohort_id\":{cohort_id:?},\"version\":{version},\"canonical_hash\":\"{}\"}}",
                hex::encode(canonical_hash)
            ),
            CohortAuditEvent::MemberReissueAccepted {
                cohort_id,
                version,
                canonical_hash,
            } => format!(
                "{{\"event\":\"member_reissue_accepted\",\"cohort_id\":{cohort_id:?},\"version\":{version},\"canonical_hash\":\"{}\"}}",
                hex::encode(canonical_hash)
            ),
            CohortAuditEvent::ReissueRejected {
                cohort_id,
                seen_version,
                rejected_version,
                reason,
            } => format!(
                "{{\"event\":\"reissue_rejected\",\"cohort_id\":{cohort_id:?},\"seen_version\":{seen_version},\"rejected_version\":{rejected_version},\"reason\":{reason:?}}}"
            ),
        };
        self.log.insert_frame_event(
            FrameKind::TelemetryEvent,
            0,
            None,
            "cohort:manifest-audit",
            payload.as_bytes(),
            FrameOrigin::Kernel,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use maos_iac::adapter::TransparencyLogAdapter;

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
}
