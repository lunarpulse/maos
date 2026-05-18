//! Canonical IAC frame types — domain-level single source of truth.
//!
//! Architecture §7.1 + §7.1.1 + FR14 define the on-wire frame shape that
//! Stories 3.2 / 3.3 / 3.4 / Epic 6 inherit. These types are pure domain
//! types; the kernel adapter (`maos-kernel-core::iac::frame`) re-exports
//! and extends them.
//!
//! ## Field ownership guide
//!
//! | Field | This story | Filled by |
//! |---|---|---|
//! | `FrameAddress.host_id: None` | y | Story 6.3 (A2A cross-Host) |
//! | `EpistemicHaltPayload` body | stub | Story 3.3 |
//! | `PosturePreferences` extension | done (Story 3.2) | Story 3.2 |
//! | `RetractPayload` body | stub | Story 6.1 |
//! | `ConsentEnvelope` | `None` | Story 6.3 (ADR-012) |

use crate::invariants::i1::{IntentClass, Scope};
use crate::invariants::i3::FrameOrigin;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use smallvec::SmallVec;

/// The universal IAC frame envelope per architecture §7.1.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IacFrame {
    pub frame_id: [u8; 16],
    pub timestamp_ns: u64,
    pub logical_clock: u64,
    pub from: FrameAddress,
    pub to: SmallVec<[FrameAddress; 1]>,
    pub kind: FrameKind,
    pub intent: IntentClass,
    pub payload: FramePayload,
    pub auto_marker: FrameOrigin,
    pub consent_envelope: Option<ConsentEnvelope>,
}

/// Reusable identity + role for frame addressing per architecture §7.1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameAddress {
    pub spirit_id: SpiritId,
    pub host_id: Option<HostId>,
    pub role: Option<SpiritRole>,
}

/// Payload carrier — one variant per `FrameKind` (0..=6).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FramePayload {
    TaskAssign(TaskAssignPayload),
    TaskComplete(TaskCompletePayload),
    DecisionDispatch(DecisionDispatchPayload),
    EpistemicHalt(EpistemicHaltPayload),
    TelemetryEvent(TelemetryEventPayload),
    ConsentRequest(ConsentRequestPayload),
    Retract(RetractPayload),
}

/// FR14: natural-language task assignment payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskAssignPayload {
    pub goal: String,
    pub scope: Vec<Scope>,
    pub success_criteria: String,
    pub posture_preferences: PosturePreferences,
}

/// v0.3 placeholder — Story 3.2 populates the body.
///
/// Story 3.2 extends this struct with halt-policy preferences
/// per FR19 + ADR-013 — additive-only; serde defaults preserve 3.1-era
/// wire compatibility.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PosturePreferences {
    #[serde(default)]
    pub preferred_posture: Option<PostureHint>,

    /// Story 3.2 — per-tag halt-policy override; missing tag means inherit
    /// the Spirit's manifest-declared policy unchanged. Each override declares
    /// a recall-vs-precision tilt in [-1.0, +1.0]: negative biases for higher
    /// halt-precision (tighten threshold, fewer false halts); positive biases
    /// for higher halt-recall (loosen threshold, fewer missed halts).
    /// Range-validated by `EpistemicPolicySection::apply_director_preferences`.
    #[serde(default)]
    pub halt_policy_overrides: Vec<HaltPolicyOverride>,
}

impl Default for PosturePreferences {
    fn default() -> Self {
        Self {
            preferred_posture: None,
            halt_policy_overrides: Vec::new(),
        }
    }
}

/// Story 3.2 — per-tag halt-policy override with recall-vs-precision tilt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HaltPolicyOverride {
    pub tag: String,
    /// Tilt in [-1.0, +1.0]; clamped at apply time. NaN rejected at apply time.
    pub recall_vs_precision: f32,
}

impl PartialEq for HaltPolicyOverride {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
            && self.recall_vs_precision.total_cmp(&other.recall_vs_precision)
                == std::cmp::Ordering::Equal
    }
}

impl Eq for HaltPolicyOverride {}

/// Posture hint for the director's task assignment.
///
/// Story 3.2 owns the full posture mechanism (three postures:
/// `autonomous-with-halt`, `assistive`, `cautious`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PostureHint {
    AutonomousWithHalt,
    Assistive,
    Cautious,
}

/// TODO(Story 3.2): body filled when the halt-policy schema lands.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskCompletePayload {
    pub result: String,
}

/// TODO(Story 3.3): `working_memory_digest_refs` (I12) field filled.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionDispatchPayload {
    pub decision_id: u64,
    pub approved: bool,
}

/// TODO(Story 3.3): shape pinned by Story 3.3.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EpistemicHaltPayload {
    pub halt_id: String,
}

/// TODO(NFR-Obs-3 v0.3): shape pinned by NFR-Obs-3.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TelemetryEventPayload {
    pub event_type: String,
    pub data: String,
}

/// TODO(Story 4.1): consent request payload filled.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsentRequestPayload {
    pub capability: String,
}

/// TODO(Story 6.1): shape filled by E6 Story 6.1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetractPayload {
    pub original_frame_id: [u8; 16],
}

/// Consent envelope for ADR-012 — None at v0.3 (Story 6.3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsentEnvelope {
    pub consent_id: [u8; 16],
    pub granter: FrameAddress,
    pub timestamp_ns: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    fn make_address() -> FrameAddress {
        FrameAddress {
            spirit_id: SpiritId::from("test-spirit"),
            host_id: None,
            role: None,
        }
    }

    fn make_frame(payload: FramePayload) -> IacFrame {
        IacFrame {
            frame_id: [0u8; 16],
            timestamp_ns: 0,
            logical_clock: 0,
            from: make_address(),
            to: smallvec![make_address()],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload,
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
        }
    }

    #[test]
    fn posture_preferences_3_1_shape_backward_compat() {
        // A 3.1-emitted frame with only preferred_posture deserializes
        // successfully; halt_policy_overrides defaults to empty.
        let json = r#"{"preferred_posture":null}"#;
        let prefs: PosturePreferences = serde_json::from_str(json).unwrap();
        assert!(prefs.preferred_posture.is_none());
        assert!(prefs.halt_policy_overrides.is_empty());
    }

    #[test]
    fn posture_preferences_3_2_shape_round_trip() {
        let overrides = vec![HaltPolicyOverride {
            tag: "x".into(),
            recall_vs_precision: 0.3,
        }];
        let prefs = PosturePreferences {
            preferred_posture: Some(PostureHint::Cautious),
            halt_policy_overrides: overrides,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let back: PosturePreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(back.preferred_posture, Some(PostureHint::Cautious));
        assert_eq!(back.halt_policy_overrides.len(), 1);
        assert_eq!(back.halt_policy_overrides[0].tag, "x");
        assert_eq!(back.halt_policy_overrides[0].recall_vs_precision, 0.3);
    }

    #[test]
    fn halt_policy_override_valid_f32_parses_at_serde_layer() {
        // serde_json does not admit NaN/Inf for f32 by default — these are
        // rejected at JSON parse time. NaN validation lives at
        // `apply_director_preferences` which explicitly rejects NaN.
        // This test confirms valid f32 values round-trip through serde.
        let json = r#"{"tag":"x","recall_vs_precision":0.5}"#;
        let parsed: HaltPolicyOverride = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.recall_vs_precision, 0.5);
    }

    #[test]
    fn posture_preferences_default_has_empty_overrides() {
        let default = PosturePreferences::default();
        assert!(default.preferred_posture.is_none());
        assert!(default.halt_policy_overrides.is_empty());
    }

    #[test]
    fn posture_preferences_serde_round_trip() {
        let prefs = PosturePreferences {
            preferred_posture: Some(PostureHint::Assistive),
            halt_policy_overrides: Vec::new(),
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let back: PosturePreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(back.preferred_posture, Some(PostureHint::Assistive));
    }

    #[test]
    fn iac_frame_task_assign_serde_round_trip() {
        let payload = FramePayload::TaskAssign(TaskAssignPayload {
            goal: "review the PR".into(),
            scope: vec![Scope::FsRead {
                subtree: "/src".into(),
            }],
            success_criteria: "PR approved".into(),
            posture_preferences: PosturePreferences::default(),
        });
        let frame = make_frame(payload);
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frame_id, frame.frame_id);
        assert_eq!(back.kind, FrameKind::TaskAssign);
        match back.payload {
            FramePayload::TaskAssign(p) => {
                assert_eq!(p.goal, "review the PR");
                assert_eq!(p.success_criteria, "PR approved");
            }
            _ => panic!("expected TaskAssign payload"),
        }
    }

    #[test]
    fn frame_address_serde_round_trip() {
        let addr = FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: None,
            role: Some(SpiritRole::Worker),
        };
        let json = serde_json::to_string(&addr).unwrap();
        let back: FrameAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(back.spirit_id.as_str(), "nash");
        assert_eq!(back.role, Some(SpiritRole::Worker));
    }

    #[test]
    fn iac_frame_task_complete_serde_round_trip() {
        let payload = FramePayload::TaskComplete(TaskCompletePayload { result: "done".into() });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::TaskComplete;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::TaskComplete(p) => assert_eq!(p.result, "done"),
            _ => panic!("expected TaskComplete"),
        }
    }

    #[test]
    fn iac_frame_decision_dispatch_serde_round_trip() {
        let payload = FramePayload::DecisionDispatch(DecisionDispatchPayload {
            decision_id: 42,
            approved: true,
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::DecisionDispatch;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::DecisionDispatch(p) => {
                assert_eq!(p.decision_id, 42);
                assert!(p.approved);
            }
            _ => panic!("expected DecisionDispatch"),
        }
    }

    #[test]
    fn iac_frame_epistemic_halt_serde_round_trip() {
        let payload = FramePayload::EpistemicHalt(EpistemicHaltPayload {
            halt_id: "halt-001".into(),
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::EpistemicHalt;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::EpistemicHalt(p) => assert_eq!(p.halt_id, "halt-001"),
            _ => panic!("expected EpistemicHalt"),
        }
    }

    #[test]
    fn iac_frame_telemetry_event_serde_round_trip() {
        let payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
            event_type: "metric".into(),
            data: "cpu=90".into(),
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::TelemetryEvent;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::TelemetryEvent(p) => {
                assert_eq!(p.event_type, "metric");
                assert_eq!(p.data, "cpu=90");
            }
            _ => panic!("expected TelemetryEvent"),
        }
    }

    #[test]
    fn iac_frame_consent_request_serde_round_trip() {
        let payload = FramePayload::ConsentRequest(ConsentRequestPayload {
            capability: "fs.write".into(),
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::ConsentRequest;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::ConsentRequest(p) => assert_eq!(p.capability, "fs.write"),
            _ => panic!("expected ConsentRequest"),
        }
    }

    #[test]
    fn iac_frame_retract_serde_round_trip() {
        let payload = FramePayload::Retract(RetractPayload {
            original_frame_id: [0xAB; 16],
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::Retract;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::Retract(p) => assert_eq!(p.original_frame_id, [0xAB; 16]),
            _ => panic!("expected Retract"),
        }
    }
}
