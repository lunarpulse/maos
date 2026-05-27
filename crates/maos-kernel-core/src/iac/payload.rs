#![forbid(unsafe_code)]

//! Story 6.4 / FR26 / ADR-025 — typed payload records emitted to the
//! Transparency Log alongside existing `FrameKind` variants.
//!
//! The `[[schedule]]` firing path emits a `FrameKind::CapabilityInvocation`
//! TL row whose payload is `ScheduleFireRecord` (JSON-serialized). The kernel
//! is the canonical writer; Spirits read these rows via `log.recall` per
//! Story 4.4.

use maos_domain::invariants::i1::TokenId;

/// One firing of a `[[schedule]]` entry. Written to the TL atomically with
/// the per-firing cap-token issue.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleFireRecord {
    /// The Spirit whose manifest declared the schedule.
    pub spirit_id: String,
    /// The manifest `[[schedule]].id` value.
    pub schedule_id: String,
    /// Monotonic time-ns at firing decision.
    pub fired_at_ns: u64,
    /// Optional ComplianceClaim envelope hash (Story 7.3 pass-through).
    #[serde(default)]
    pub compliance_claim_ref: Option<[u8; 32]>,
    /// The token issued for the firing — narrowed to `side_effect_scopes`.
    pub side_effect_token_id: TokenId,
    /// Whether the manifest declared `principal_revocability = true` for the
    /// firing — informational for audit reconstruction.
    pub principal_revocability: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(r: &ScheduleFireRecord) -> ScheduleFireRecord {
        let json = serde_json::to_string(r).expect("serialize ScheduleFireRecord");
        serde_json::from_str::<ScheduleFireRecord>(&json).expect("deserialize ScheduleFireRecord")
    }

    #[test]
    fn schedule_fire_record_round_trip() {
        let r = ScheduleFireRecord {
            spirit_id: "butler".into(),
            schedule_id: "morning-digest".into(),
            fired_at_ns: 12345,
            compliance_claim_ref: Some([1u8; 32]),
            side_effect_token_id: TokenId([2u8; 16]),
            principal_revocability: true,
        };
        let back = round_trip(&r);
        assert_eq!(back, r);
    }

    #[test]
    fn schedule_fire_record_optional_compliance_claim_round_trip() {
        let r = ScheduleFireRecord {
            spirit_id: "researcher".into(),
            schedule_id: "arxiv-watcher".into(),
            fired_at_ns: 99,
            compliance_claim_ref: None,
            side_effect_token_id: TokenId::ZERO,
            principal_revocability: false,
        };
        let back = round_trip(&r);
        assert_eq!(back, r);
        assert!(back.compliance_claim_ref.is_none());
    }
}
