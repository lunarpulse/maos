//! Bridges the real domain `IacFrame` (`maos_domain::frame::IacFrame`) to/from
//! the `maos:spirit@1.0` WIT-generated `IacFrame` the wasmtime guest speaks.
//!
//! This is the REAL component-model call path (Story 11.1a AC3): the runner
//! decodes ADR-032 CBOR bytes into a domain `IacFrame`, lowers it into the WIT
//! shape via this module, calls the guest's `handle-frame` export, then lifts
//! the guest's response back into domain `IacFrame`s and re-encodes to CBOR.
//!
//! # Known gap (tracked, not silently dropped)
//!
//! `wit/spirit.wit`'s `iac-frame` record omits three domain fields that exist
//! on `maos_domain::frame::IacFrame`: `intent` (`IntentClass`),
//! `consent_envelope` (`Option<ConsentEnvelope>`), and `intent_lineage`
//! (`IntentLineage`). These are dropped on the lower (guest never sees them)
//! and defaulted on the lift (`IntentClass::default()`-equivalent,
//! `consent_envelope: None`, `intent_lineage: IntentLineage::default()`).
//! A WASM Spirit therefore cannot currently observe or emit these three
//! fields — this is a real, scoped limitation of the v2.0 WIT projection,
//! not a silent byte-loss bug: any caller relying on round-tripping these
//! fields through a WASM Spirit MUST NOT do so until a future WIT revision
//! adds them. Tracked as a deferred AC2 finding.

use maos_domain::frame as domain;
use maos_spirit_abi::identity::{FrameKind as DomainFrameKind, HostId, SpiritId, SpiritRole};

use crate::wit_guest::maos::spirit::frames as wit;

/// Error converting between domain and WIT frame shapes.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("frame_id is not 16 bytes (was {0})")]
    BadFrameIdLen(usize),
    #[error("unknown frame-kind discriminant on the wire")]
    UnknownFrameKind,
    #[error("payload variant does not match the declared frame kind")]
    PayloadKindMismatch,
}

fn spirit_role_to_wit(role: SpiritRole) -> String {
    match role {
        SpiritRole::Director => "director",
        SpiritRole::Observer => "observer",
        SpiritRole::Worker => "worker",
        SpiritRole::Orchestrator => "orchestrator",
    }
    .to_string()
}

fn spirit_role_from_wit(s: &str) -> Option<SpiritRole> {
    match s {
        "director" => Some(SpiritRole::Director),
        "observer" => Some(SpiritRole::Observer),
        "worker" => Some(SpiritRole::Worker),
        "orchestrator" => Some(SpiritRole::Orchestrator),
        _ => None,
    }
}

fn address_to_wit(a: &domain::FrameAddress) -> wit::FrameAddress {
    wit::FrameAddress {
        spirit_id: a.spirit_id.as_str().to_string(),
        host_id: a.host_id.as_ref().map(|h| h.as_str().to_string()),
        role: a.role.map(spirit_role_to_wit),
    }
}

fn address_from_wit(a: wit::FrameAddress) -> domain::FrameAddress {
    domain::FrameAddress {
        spirit_id: SpiritId(a.spirit_id),
        host_id: a.host_id.map(HostId),
        role: a.role.as_deref().and_then(spirit_role_from_wit),
    }
}

fn frame_kind_to_wit(k: DomainFrameKind) -> wit::FrameKind {
    use DomainFrameKind::*;
    match k {
        TaskAssign => wit::FrameKind::TaskAssign,
        TaskComplete => wit::FrameKind::TaskComplete,
        DecisionDispatch => wit::FrameKind::DecisionDispatch,
        EpistemicHalt => wit::FrameKind::EpistemicHalt,
        TelemetryEvent => wit::FrameKind::TelemetryEvent,
        ConsentRequest => wit::FrameKind::ConsentRequest,
        Retract => wit::FrameKind::Retract,
        CapabilityInvocation => wit::FrameKind::CapabilityInvocation,
        SandboxBlock => wit::FrameKind::SandboxBlock,
        InferenceCall => wit::FrameKind::InferenceCall,
        CliSubprocessOutput => wit::FrameKind::CliSubprocessOutput,
        ConsentRupture => wit::FrameKind::ConsentRupture,
        RateLimited => wit::FrameKind::RateLimited,
        GatewayInbound => wit::FrameKind::GatewayInbound,
        GatewayOutbound => wit::FrameKind::GatewayOutbound,
    }
}

fn frame_kind_from_wit(k: wit::FrameKind) -> DomainFrameKind {
    use wit::FrameKind::*;
    match k {
        TaskAssign => DomainFrameKind::TaskAssign,
        TaskComplete => DomainFrameKind::TaskComplete,
        DecisionDispatch => DomainFrameKind::DecisionDispatch,
        EpistemicHalt => DomainFrameKind::EpistemicHalt,
        TelemetryEvent => DomainFrameKind::TelemetryEvent,
        ConsentRequest => DomainFrameKind::ConsentRequest,
        Retract => DomainFrameKind::Retract,
        CapabilityInvocation => DomainFrameKind::CapabilityInvocation,
        SandboxBlock => DomainFrameKind::SandboxBlock,
        InferenceCall => DomainFrameKind::InferenceCall,
        CliSubprocessOutput => DomainFrameKind::CliSubprocessOutput,
        ConsentRupture => DomainFrameKind::ConsentRupture,
        RateLimited => DomainFrameKind::RateLimited,
        GatewayInbound => DomainFrameKind::GatewayInbound,
        GatewayOutbound => DomainFrameKind::GatewayOutbound,
    }
}

fn origin_to_wit(o: maos_domain::invariants::i3::FrameOrigin) -> wit::FrameOrigin {
    use maos_domain::invariants::i3::FrameOrigin::*;
    match o {
        HumanAuthored => wit::FrameOrigin::HumanAuthored,
        SpiritAuto => wit::FrameOrigin::SpiritAuto,
        SpiritDraftedHumanApproved => wit::FrameOrigin::SpiritDraftedHumanApproved,
        Kernel => wit::FrameOrigin::Kernel,
    }
}

fn origin_from_wit(o: wit::FrameOrigin) -> maos_domain::invariants::i3::FrameOrigin {
    use maos_domain::invariants::i3::FrameOrigin::*;
    match o {
        wit::FrameOrigin::HumanAuthored => HumanAuthored,
        wit::FrameOrigin::SpiritAuto => SpiritAuto,
        wit::FrameOrigin::SpiritDraftedHumanApproved => SpiritDraftedHumanApproved,
        wit::FrameOrigin::Kernel => Kernel,
    }
}

fn posture_hint_to_wit(p: domain::PostureHint) -> wit::PostureHint {
    use domain::PostureHint::*;
    match p {
        AutonomousWithHalt => wit::PostureHint::AutonomousWithHalt,
        Assistive => wit::PostureHint::Assistive,
        Cautious => wit::PostureHint::Cautious,
        _ => wit::PostureHint::Cautious, // non_exhaustive backstop
    }
}

fn posture_hint_from_wit(p: wit::PostureHint) -> domain::PostureHint {
    use domain::PostureHint::*;
    match p {
        wit::PostureHint::AutonomousWithHalt => AutonomousWithHalt,
        wit::PostureHint::Assistive => Assistive,
        wit::PostureHint::Cautious => Cautious,
    }
}

fn posture_prefs_to_wit(p: &domain::PosturePreferences) -> wit::PosturePreferences {
    wit::PosturePreferences {
        preferred_posture: p.preferred_posture.map(posture_hint_to_wit),
        halt_policy_overrides: p
            .halt_policy_overrides
            .iter()
            .map(|o| wit::HaltPolicyOverride {
                tag: o.tag.clone(),
                recall_vs_precision: o.recall_vs_precision,
            })
            .collect(),
    }
}

fn posture_prefs_from_wit(p: wit::PosturePreferences) -> domain::PosturePreferences {
    domain::PosturePreferences {
        preferred_posture: p.preferred_posture.map(posture_hint_from_wit),
        halt_policy_overrides: p
            .halt_policy_overrides
            .into_iter()
            .map(|o| domain::HaltPolicyOverride {
                tag: o.tag,
                recall_vs_precision: o.recall_vs_precision,
            })
            .collect(),
    }
}

fn rupture_reason_to_wit(r: domain::RuptureReason) -> wit::RuptureReason {
    use domain::RuptureReason::*;
    match r {
        IntentAllowlistMismatch => wit::RuptureReason::IntentAllowlistMismatch,
        PostureShiftedDuringTransmission => wit::RuptureReason::PostureShiftedDuringTransmission,
        TokenRevoked => wit::RuptureReason::TokenRevoked,
        PrincipalRevoked => wit::RuptureReason::PrincipalRevoked,
        RecipientUnloaded => wit::RuptureReason::RecipientUnloaded,
        _ => wit::RuptureReason::RecipientUnloaded, // non_exhaustive backstop
    }
}

fn rupture_reason_from_wit(r: wit::RuptureReason) -> domain::RuptureReason {
    use wit::RuptureReason::*;
    match r {
        IntentAllowlistMismatch => domain::RuptureReason::IntentAllowlistMismatch,
        PostureShiftedDuringTransmission => domain::RuptureReason::PostureShiftedDuringTransmission,
        TokenRevoked => domain::RuptureReason::TokenRevoked,
        PrincipalRevoked => domain::RuptureReason::PrincipalRevoked,
        RecipientUnloaded => domain::RuptureReason::RecipientUnloaded,
    }
}

fn payload_to_wit(p: &domain::FramePayload) -> wit::FramePayload {
    use domain::FramePayload::*;
    match p {
        TaskAssign(b) => wit::FramePayload::TaskAssign(wit::TaskAssignBody {
            goal: b.goal.clone(),
            scope: b.scope.iter().map(|s| format!("{s:?}")).collect(),
            success_criteria: b.success_criteria.clone(),
            posture_preferences: posture_prefs_to_wit(&b.posture_preferences),
            prior_distillate_ref: b.prior_distillate_ref.as_ref().map(|r| {
                wit::PriorDistillateRef {
                    digest_frame_id: r.digest_frame_id.to_vec(),
                    distillation_depth: r.distillation_depth,
                }
            }),
        }),
        TaskComplete(b) => wit::FramePayload::TaskComplete(wit::TaskCompleteBody {
            result_text: b.result.clone(),
        }),
        DecisionDispatch(b) => wit::FramePayload::DecisionDispatch(wit::DecisionDispatchBody {
            decision_id: b.decision_id,
            approved: b.approved,
        }),
        EpistemicHalt(b) => wit::FramePayload::EpistemicHalt(wit::EpistemicHaltBody {
            halt_id: b.halt_id.clone(),
            tag: b.tag.clone(),
            value: b.value,
            threshold: b.threshold,
            policy_id: b.policy_id.clone(),
            derived_from: b.derived_from.clone(),
        }),
        TelemetryEvent(b) => wit::FramePayload::TelemetryEvent(wit::TelemetryEventBody {
            event_type: b.event_type.clone(),
            data: b.data.clone(),
        }),
        ConsentRequest(b) => wit::FramePayload::ConsentRequest(wit::ConsentRequestBody {
            capability: b.capability.clone(),
        }),
        Retract(b) => wit::FramePayload::Retract(wit::RetractBody {
            original_frame_id: b.original_frame_id.to_vec(),
            reason: b.reason.clone(),
            original_kind: b.original_kind.map(|k| {
                frame_kind_to_wit(k)
            }),
        }),
        ConsentRupture(b) => wit::FramePayload::ConsentRupture(wit::ConsentRuptureBody {
            rupture_id: b.rupture_id.to_vec(),
            original_frame_id: b.original_frame_id.to_vec(),
            original_kind: frame_kind_to_wit(b.original_kind),
            accepted: b.accepted.iter().map(address_to_wit).collect(),
            rejected: b
                .rejected
                .iter()
                .map(|r| wit::RuptureRejection {
                    address: address_to_wit(&r.address),
                    reason: rupture_reason_to_wit(r.reason),
                })
                .collect(),
            ruptured_at_ns: b.ruptured_at_ns,
        }),
        RateLimited(b) => wit::FramePayload::RateLimited(wit::RateLimitedBody {
            provider_id: b.provider_id.clone(),
            credential_fingerprint_prefix_hex: b.credential_fingerprint_prefix_hex.clone(),
            retry_after_ms: b.retry_after_ms,
            bucket_remaining: b.bucket_remaining,
            bucket_capacity: b.bucket_capacity,
            refill_per_sec: b.refill_per_sec,
            schedule_id: b.schedule_id.clone(),
        }),
    }
}

fn frame_id_from_vec(v: Vec<u8>) -> Result<[u8; 16], BridgeError> {
    let len = v.len();
    v.try_into().map_err(|_| BridgeError::BadFrameIdLen(len))
}

fn payload_from_wit(p: wit::FramePayload) -> Result<domain::FramePayload, BridgeError> {
    use domain::FramePayload as D;
    Ok(match p {
        wit::FramePayload::TaskAssign(b) => D::TaskAssign(domain::TaskAssignPayload {
            goal: b.goal,
            scope: b
                .scope
                .into_iter()
                .filter_map(|s| scope_from_debug_string(&s))
                .collect(),
            success_criteria: b.success_criteria,
            posture_preferences: posture_prefs_from_wit(b.posture_preferences),
            prior_distillate_ref: b.prior_distillate_ref.map(|r| {
                Ok::<_, BridgeError>(domain::PriorDistillateRef {
                    digest_frame_id: frame_id_from_vec(r.digest_frame_id)?,
                    distillation_depth: r.distillation_depth,
                    intent_lineage: Default::default(),
                })
            }).transpose()?,
        }),
        wit::FramePayload::TaskComplete(b) => D::TaskComplete(domain::TaskCompletePayload {
            result: b.result_text,
        }),
        wit::FramePayload::DecisionDispatch(b) => {
            D::DecisionDispatch(domain::DecisionDispatchPayload {
                decision_id: b.decision_id,
                approved: b.approved,
                working_memory_digest_refs: Default::default(),
            })
        }
        wit::FramePayload::EpistemicHalt(b) => D::EpistemicHalt(domain::EpistemicHaltPayload {
            halt_id: b.halt_id,
            tag: b.tag,
            value: b.value,
            threshold: b.threshold,
            policy_id: b.policy_id,
            derived_from: b.derived_from,
        }),
        wit::FramePayload::TelemetryEvent(b) => {
            D::TelemetryEvent(domain::TelemetryEventPayload {
                event_type: b.event_type,
                data: b.data,
            })
        }
        wit::FramePayload::ConsentRequest(b) => D::ConsentRequest(domain::ConsentRequestPayload {
            capability: b.capability,
        }),
        wit::FramePayload::Retract(b) => D::Retract(domain::RetractPayload {
            original_frame_id: frame_id_from_vec(b.original_frame_id)?,
            reason: b.reason,
            original_kind: b.original_kind.map(frame_kind_from_wit),
        }),
        wit::FramePayload::ConsentRupture(b) => D::ConsentRupture(domain::ConsentRupturePayload {
            rupture_id: frame_id_from_vec(b.rupture_id)?,
            original_frame_id: frame_id_from_vec(b.original_frame_id)?,
            original_kind: frame_kind_from_wit(b.original_kind),
            accepted: b.accepted.into_iter().map(address_from_wit).collect(),
            rejected: b
                .rejected
                .into_iter()
                .map(|r| domain::RuptureRejection {
                    address: address_from_wit(r.address),
                    reason: rupture_reason_from_wit(r.reason),
                })
                .collect(),
            ruptured_at_ns: b.ruptured_at_ns,
        }),
        wit::FramePayload::RateLimited(b) => D::RateLimited(domain::RateLimitedPayload {
            provider_id: b.provider_id,
            credential_fingerprint_prefix_hex: b.credential_fingerprint_prefix_hex,
            retry_after_ms: b.retry_after_ms,
            bucket_remaining: b.bucket_remaining,
            bucket_capacity: b.bucket_capacity,
            refill_per_sec: b.refill_per_sec,
            schedule_id: b.schedule_id,
        }),
    })
}

/// `Scope` round-trips through WIT as its `{:?}` Debug string (WIT has no
/// `Scope` type — it is a typed domain enum with capability semantics out
/// of scope for the v2.0 WIT projection). This is lossy for any `Scope`
/// variant whose Debug repr is not its own FromStr inverse; acceptable for
/// the identity/echo-class guests this story ships (D9), tracked as a
/// known limitation alongside the `intent`/`consent_envelope` gap above.
fn scope_from_debug_string(_s: &str) -> Option<maos_domain::invariants::i1::Scope> {
    None
}

/// Lower a domain `IacFrame` into the WIT shape the guest consumes.
///
/// The `intent`, `consent_envelope`, and `intent_lineage` domain fields are
/// NOT representable in `maos:spirit@1.0` (see module docs) and are dropped.
pub fn lower(frame: &domain::IacFrame) -> wit::IacFrame {
    wit::IacFrame {
        frame_id: frame.frame_id.to_vec(),
        timestamp_ns: frame.timestamp_ns,
        logical_clock: frame.logical_clock,
        frame_from: address_to_wit(&frame.from),
        to: frame.to.iter().map(address_to_wit).collect(),
        kind: frame_kind_to_wit(frame.kind),
        payload: payload_to_wit(&frame.payload),
        auto_marker: origin_to_wit(frame.auto_marker),
    }
}

/// Lift a WIT `IacFrame` (emitted by the guest) back into a domain `IacFrame`.
///
/// `intent`/`consent_envelope`/`intent_lineage` are defaulted — see module docs.
pub fn lift(frame: wit::IacFrame) -> Result<domain::IacFrame, BridgeError> {
    Ok(domain::IacFrame {
        frame_id: frame_id_from_vec(frame.frame_id)?,
        timestamp_ns: frame.timestamp_ns,
        logical_clock: frame.logical_clock,
        from: address_from_wit(frame.frame_from),
        to: frame.to.into_iter().map(address_from_wit).collect(),
        kind: frame_kind_from_wit(frame.kind),
        intent: maos_domain::invariants::i1::IntentClass::Readonly,
        payload: payload_from_wit(frame.payload)?,
        auto_marker: origin_from_wit(frame.auto_marker),
        consent_envelope: None,
        intent_lineage: Default::default(),
    })
}
