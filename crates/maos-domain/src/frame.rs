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
use crate::invariants::i13::IntentLineage;
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
    /// Story 4.5 — NFR-Aud-14 intent-lineage propagation. The unbroken
    /// chain back to the originating principal intent for cross-Spirit
    /// frames. Defaults to empty (serde-default) for ABI-additivity —
    /// existing test fixtures and the v0.3-β wire-frame writers still
    /// deserialize correctly. Cross-Spirit emission paths through
    /// `IacBusAdapter::deliver_typed` enforce non-empty lineage via
    /// `EIntentLineageBroken` rejection per AC4. The complementary
    /// I13 distillate-side lineage (`DistillationReceipt::intent_lineage`)
    /// is a SEPARATE field on a SEPARATE type — distillates are kernel-side
    /// audit annotations, not IAC frames, so the two lineages do NOT collide
    /// and live in different invariants (I13 distillate / I14-adjacent IAC).
    #[doc = "Construct via [`IacFrame::new`] (or the IAC adapter's typed-deliver path) to enforce non-empty lineage validation on cross-Spirit emissions; struct literals bypass the kernel-side EIntentLineageBroken check by allowing empty lineage to slip through to the bus — the bus rejects but at higher cost. NFR-Aud-14 binding-v0.8."]
    #[serde(default)]
    pub intent_lineage: IntentLineage,
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
    /// Story 6.2 — FR21. Optional reference to the `DistillationReceipt::digest_frame_id`
    /// of the prior Worker output this dispatch is built upon. `None` for the FIRST
    /// dispatch in a fan-out (no predecessor exists). Required for every subsequent
    /// dispatch in the same Orchestrator session: AC2's `EOrchestratorDispatchRawOutput`
    /// fires if the Orchestrator emits a follow-up `task.assign` with `prior_distillate_ref = None`
    /// when a prior Worker completion exists in the session's log_recall window.
    #[serde(default)]
    pub prior_distillate_ref: Option<PriorDistillateRef>,
}

/// Story 6.2 — reference to a prior Worker's distilled output, used by the Orchestrator
/// to dispatch follow-up tasks against the distillate rather than raw output.
///
/// The `digest_frame_id` MUST resolve to a `FrameKind::Distillate` row in the
/// Transparency Log; the AC2 runtime check `check_orchestrator_distillate_required`
/// rejects references to raw `TaskComplete` rows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PriorDistillateRef {
    /// The `FrameKind::Distillate` row id in the Transparency Log.
    pub digest_frame_id: [u8; 16],
    /// Effective distillation depth at this hop
    /// (`DistillationReceipt::effective_distillation_depth`).
    pub distillation_depth: u32,
    /// The IntentLineage union the kernel computed for this digest (I13).
    #[serde(default)]
    pub intent_lineage: crate::invariants::i13::IntentLineage,
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
            && self
                .recall_vs_precision
                .total_cmp(&other.recall_vs_precision)
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

/// Story 3.3 — FR18 + NFR-Aud-5 right-to-explanation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionDispatchPayload {
    pub decision_id: u64,
    pub approved: bool,
    /// I12 — `working_memory_digest_refs` populated by the kernel-side
    /// decision logger (`crates/maos-kernel-core/src/iac/decision_logger.rs`)
    /// BEFORE the frame is enqueued onto the Mailbox.
    /// Pre-3.3 wire payloads default to the empty refs set.
    #[serde(default)]
    pub working_memory_digest_refs: crate::invariants::i12::WorkingMemoryDigestRefs,
}

/// Story 3.3 — structured halt payload per architecture §4.6.1.
///
/// Pre-3.3 wire payloads carrying only `halt_id` deserialize with
/// the new fields defaulted (per `#[serde(default)]`), preserving
/// Story 3.1's additive-only contract.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EpistemicHaltPayload {
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    pub halt_id: String,
    /// The `[epistemic_policy]` tag that fired (e.g.
    /// `"claim.security_vulnerability"`). Cross-references the
    /// `EpistemicPolicyRule.tag` parsed in Story 3.2 AC1.
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    #[serde(default)]
    pub tag: String,
    /// The observed scalar value the predicate compared against.
    /// f32 to match Story 4.2's `working_memory.set_scalar` shape.
    /// `PartialEq` derived — bit-equal comparison; NaN payloads are
    /// rejected at construction by `HaltPayload::new`.
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    #[serde(default)]
    pub value: f32,
    /// The configured threshold from `on_confidence_below`
    /// (or `None` when the rule fired on `on_evidence_conflict`).
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    #[serde(default)]
    pub threshold: Option<f32>,
    /// Stable identifier of the rule that fired — Spirit-supplied
    /// (mirrors `EpistemicPolicyRule.tag` namespacing).
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    #[serde(default)]
    pub policy_id: String,
    /// Provenance chain — the `derived_from` Spirit-supplied marker
    /// passed to `working_memory.set_scalar`. Free-form string at v0.3;
    /// Story 4.4 (`log.recall` + I11 chain) wires the typed lineage.
    #[doc = "Construct via [`EpistemicHaltPayload::new`] to enforce validation; struct literals bypass NaN / empty / range checks."]
    #[serde(default)]
    pub derived_from: String,
}

impl EpistemicHaltPayload {
    /// Construct a structured payload — rejects `f32::NAN` for `value`
    /// or `threshold` so resolved halts cannot poison the audit log
    /// with non-comparable scalars.
    pub fn new(
        halt_id: String,
        tag: String,
        value: f32,
        threshold: Option<f32>,
        policy_id: String,
        derived_from: String,
    ) -> Result<Self, HaltPayloadError> {
        if halt_id.is_empty() {
            return Err(HaltPayloadError::EmptyHaltId);
        }
        if value.is_nan() {
            return Err(HaltPayloadError::NanValue);
        }
        if let Some(t) = threshold {
            if t.is_nan() {
                return Err(HaltPayloadError::NanThreshold);
            }
        }
        Ok(Self {
            halt_id,
            tag,
            value,
            threshold,
            policy_id,
            derived_from,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HaltPayloadError {
    #[error("halt payload value is NaN; predicate scalars must be comparable")]
    NanValue,
    #[error("halt payload threshold is NaN; predicate thresholds must be comparable")]
    NanThreshold,
    #[error("halt_id must be non-empty")]
    EmptyHaltId,
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

/// Retract payload — Story 6.1.
///
/// Per architecture §4.5: "a Spirit can issue `retract(message_id, reason)`;
/// the kernel marks the original log entry as retracted, sends a structured
/// `retract` frame to the peer, and the peer's IAC Bus surfaces it to its
/// human. **Retract is not delete** — the Transparency Log is append-only."
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetractPayload {
    pub original_frame_id: [u8; 16],
    /// Free-form retraction reason — surfaced through the notification dispatcher
    /// to the original recipient's human. Empty string permitted; max 4096 bytes
    /// enforced at construction time (`RetractPayload::new`) to prevent log inflation.
    #[serde(default)]
    pub reason: String,
    /// The `FrameKind` of the frame being retracted — captured at retraction time
    /// because the original frame's TL row may be redacted before this Retract
    /// frame is read; the discriminator is needed for retract-corpus replay tests.
    #[serde(default)]
    pub original_kind: Option<maos_spirit_abi::identity::FrameKind>,
}

/// Error raised when `RetractPayload::new` rejects invalid input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetractPayloadError {
    #[error("retract reason exceeds 4096-byte cap (was {0} bytes)")]
    ReasonTooLong(usize),
}

impl RetractPayload {
    /// Construct a `RetractPayload` with validation.
    ///
    /// Enforces the 4096-byte reason cap per architecture §4.5.
    pub fn new(
        original_frame_id: [u8; 16],
        reason: String,
        original_kind: Option<maos_spirit_abi::identity::FrameKind>,
    ) -> Result<Self, RetractPayloadError> {
        if reason.len() > 4096 {
            return Err(RetractPayloadError::ReasonTooLong(reason.len()));
        }
        // Ensure truncation safety: validate at char boundary for UTF-8 display
        let _ = reason.get(..4096.min(reason.ceil_char_boundary(4096)));
        Ok(Self {
            original_frame_id,
            reason,
            original_kind,
        })
    }
}

/// Consent envelope for ADR-012 — v0.3 skeleton; Story 6.3 ADR-012
/// binding-v0.9 adds the typed-intent + expiry projection.
///
/// The new fields are `#[serde(default)]` so v0.3-era wire payloads still
/// deserialize correctly (ABI-additive contract).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsentEnvelope {
    pub consent_id: [u8; 16],
    pub granter: FrameAddress,
    pub timestamp_ns: u64,
    /// Story 6.3 / ADR-012 binding-v0.9 — typed-intent for cross-Host consent.
    /// Filled by the sender's A2A outbound path; verified by the receiver's
    /// A2A intake. Same-Host frames use `None`.
    #[serde(default)]
    pub intent_class: Option<crate::invariants::i8::A2AIntent>,
    /// Story 6.3 — consent envelope expiry. Receiver rejects with
    /// `A2AError::ConsentExpired` if `now > valid_until_ns`. `None` = open-ended.
    #[serde(default)]
    pub valid_until_ns: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariants::i12::WorkingMemoryDigestRefs;
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
            intent_lineage: IntentLineage::default(),
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
            prior_distillate_ref: None,
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
        let payload = FramePayload::TaskComplete(TaskCompletePayload {
            result: "done".into(),
        });
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
            working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
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
        let payload = FramePayload::EpistemicHalt(
            EpistemicHaltPayload::new(
                "halt-001".into(),
                "claim.security".into(),
                0.3,
                Some(0.5),
                "pol-1".into(),
                "derived".into(),
            )
            .unwrap(),
        );
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::EpistemicHalt;
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        match back.payload {
            FramePayload::EpistemicHalt(p) => {
                assert_eq!(p.halt_id, "halt-001");
                assert_eq!(p.tag, "claim.security");
                assert_eq!(p.value, 0.3);
                assert_eq!(p.threshold, Some(0.5));
                assert_eq!(p.policy_id, "pol-1");
                assert_eq!(p.derived_from, "derived");
            }
            _ => panic!("expected EpistemicHalt"),
        }
    }

    #[test]
    fn epistemic_halt_payload_rejects_nan_value() {
        let result = EpistemicHaltPayload::new(
            "h".into(),
            "t".into(),
            f32::NAN,
            None,
            "p".into(),
            "d".into(),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HaltPayloadError::NanValue));
    }

    #[test]
    fn epistemic_halt_payload_rejects_nan_threshold() {
        let result = EpistemicHaltPayload::new(
            "h".into(),
            "t".into(),
            0.0,
            Some(f32::NAN),
            "p".into(),
            "d".into(),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HaltPayloadError::NanThreshold
        ));
    }

    #[test]
    fn epistemic_halt_payload_rejects_empty_halt_id() {
        let result =
            EpistemicHaltPayload::new("".into(), "t".into(), 0.0, None, "p".into(), "d".into());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HaltPayloadError::EmptyHaltId));
    }

    #[test]
    fn epistemic_halt_payload_3_1_shape_backward_compat() {
        // A 3.1-era payload with only halt_id deserializes with new fields defaulted.
        let json = r#"{"halt_id":"x"}"#;
        let payload: EpistemicHaltPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.halt_id, "x");
        assert_eq!(payload.tag, "");
        assert_eq!(payload.value, 0.0);
        assert_eq!(payload.threshold, None);
        assert_eq!(payload.policy_id, "");
        assert_eq!(payload.derived_from, "");
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
    fn decision_dispatch_3_1_shape_backward_compat() {
        // A 3.1-era payload with only decision_id + approved deserializes
        // with working_memory_digest_refs defaulted to empty.
        let json = r#"{"decision_id":42,"approved":true}"#;
        let payload: DecisionDispatchPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.decision_id, 42);
        assert!(payload.approved);
        assert!(payload.working_memory_digest_refs.as_slice().is_empty());
    }

    #[test]
    fn decision_dispatch_3_3_shape_round_trip() {
        let payload = DecisionDispatchPayload {
            decision_id: 99,
            approved: false,
            working_memory_digest_refs: WorkingMemoryDigestRefs::new(vec![
                "f1".into(),
                "f2".into(),
            ]),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let back: DecisionDispatchPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decision_id, 99);
        assert!(!back.approved);
        assert_eq!(back.working_memory_digest_refs.as_slice(), &["f1", "f2"]);
    }

    #[test]
    fn iac_frame_retract_serde_round_trip() {
        let payload = FramePayload::Retract(RetractPayload {
            original_frame_id: [0xAB; 16],
            reason: String::new(),
            original_kind: None,
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

    #[test]
    fn iac_frame_intent_lineage_serde_round_trip_non_empty() {
        use crate::invariants::i8::A2AIntent;

        let payload = FramePayload::TaskAssign(TaskAssignPayload {
            goal: "test".into(),
            scope: vec![],
            success_criteria: "done".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        });
        let mut frame = make_frame(payload);
        frame.kind = FrameKind::TaskAssign;
        frame.intent_lineage = IntentLineage::new(vec![A2AIntent::new("standard")]);
        let json = serde_json::to_string(&frame).unwrap();
        let back: IacFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.intent_lineage.as_slice().len(), 1);
        assert_eq!(back.intent_lineage.as_slice()[0].as_str(), "standard");
    }

    #[test]
    fn iac_frame_intent_lineage_serde_default_backward_compat() {
        // JSON without the intent_lineage field must deserialize to empty lineage
        let json = r#"{
            "frame_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
            "timestamp_ns":0,
            "logical_clock":0,
            "from":{"spirit_id":"test-spirit","host_id":null,"role":null},
            "to":[{"spirit_id":"test-spirit","host_id":null,"role":null}],
            "kind":"TaskAssign",
            "intent":"Standard",
            "payload":{"TaskAssign":{"goal":"test","scope":[],"success_criteria":"done","posture_preferences":{"preferred_posture":null}}},
            "auto_marker":"HumanAuthored",
            "consent_envelope":null
        }"#;
        let frame: IacFrame = serde_json::from_str(json).unwrap();
        assert!(frame.intent_lineage.is_empty());
    }
}
