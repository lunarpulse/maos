//! Codec between domain `IacFrame` values and the `maos:spirit@2.0.0`
//! WIT-generated `IacFrame` the wasmtime guest speaks.
//!
//! The runner decodes ADR-032 CBOR bytes into a domain frame, lowers it into
//! this WIT shape, calls the guest's `handle-frame` export, then lifts guest
//! responses back into domain frames and re-encodes them. The projection is
//! lossless for every WIT-representable field, including intent, consent,
//! lineage, typed scopes, and the nested provenance fields.
//!
//! This bridge is a codec, not an authorization point. A guest-emitted intent
//! or consent envelope is a claim; the kernel's frame ingress judges that
//! claim (17-3c AC2), just as it does for a native Spirit. Fixed-size domain
//! identifiers and hashes are validated while lifting because those types
//! cannot represent arbitrary-length guest bytes.
//!
//! Future domain variants that this WIT cannot name are rejected as typed
//! [`BridgeError::Unrepresentable`] errors rather than silently mapped.

use maos_domain::frame as domain;
use maos_spirit_abi::identity::{FrameKind as DomainFrameKind, HostId, SpiritId, SpiritRole};

use crate::wit_guest::maos::spirit::frames as wit;

/// Error converting between domain and WIT frame shapes.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("frame_id is not 16 bytes (was {0})")]
    BadFrameIdLen(usize),
    #[error("argv-prefix-hash is not 32 bytes (was {0})")]
    BadHashLen(usize),
    #[error("domain {ty} variant is not representable by maos:spirit@2.0.0")]
    Unrepresentable { ty: &'static str },
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
        BudgetWarning => wit::FrameKind::BudgetWarning,
        BudgetExceeded => wit::FrameKind::BudgetExceeded,
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
        BudgetWarning => DomainFrameKind::BudgetWarning,
        BudgetExceeded => DomainFrameKind::BudgetExceeded,
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

fn posture_hint_to_wit(p: domain::PostureHint) -> Result<wit::PostureHint, BridgeError> {
    use domain::PostureHint::*;
    Ok(match p {
        AutonomousWithHalt => wit::PostureHint::AutonomousWithHalt,
        Assistive => wit::PostureHint::Assistive,
        Cautious => wit::PostureHint::Cautious,
        _ => return Err(BridgeError::Unrepresentable { ty: "PostureHint" }),
    })
}

fn posture_hint_from_wit(p: wit::PostureHint) -> domain::PostureHint {
    use domain::PostureHint::*;
    match p {
        wit::PostureHint::AutonomousWithHalt => AutonomousWithHalt,
        wit::PostureHint::Assistive => Assistive,
        wit::PostureHint::Cautious => Cautious,
    }
}

fn posture_prefs_to_wit(
    p: &domain::PosturePreferences,
) -> Result<wit::PosturePreferences, BridgeError> {
    Ok(wit::PosturePreferences {
        preferred_posture: p.preferred_posture.map(posture_hint_to_wit).transpose()?,
        halt_policy_overrides: p
            .halt_policy_overrides
            .iter()
            .map(|o| wit::HaltPolicyOverride {
                tag: o.tag.clone(),
                recall_vs_precision: o.recall_vs_precision,
            })
            .collect(),
    })
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

fn rupture_reason_to_wit(r: domain::RuptureReason) -> Result<wit::RuptureReason, BridgeError> {
    use domain::RuptureReason::*;
    Ok(match r {
        IntentAllowlistMismatch => wit::RuptureReason::IntentAllowlistMismatch,
        PostureShiftedDuringTransmission => wit::RuptureReason::PostureShiftedDuringTransmission,
        TokenRevoked => wit::RuptureReason::TokenRevoked,
        PrincipalRevoked => wit::RuptureReason::PrincipalRevoked,
        RecipientUnloaded => wit::RuptureReason::RecipientUnloaded,
        PeerIdentityUnverified => wit::RuptureReason::PeerIdentityUnverified,
        _ => {
            return Err(BridgeError::Unrepresentable {
                ty: "RuptureReason",
            })
        }
    })
}

fn rupture_reason_from_wit(r: wit::RuptureReason) -> domain::RuptureReason {
    use wit::RuptureReason::*;
    match r {
        IntentAllowlistMismatch => domain::RuptureReason::IntentAllowlistMismatch,
        PostureShiftedDuringTransmission => domain::RuptureReason::PostureShiftedDuringTransmission,
        TokenRevoked => domain::RuptureReason::TokenRevoked,
        PrincipalRevoked => domain::RuptureReason::PrincipalRevoked,
        RecipientUnloaded => domain::RuptureReason::RecipientUnloaded,
        PeerIdentityUnverified => domain::RuptureReason::PeerIdentityUnverified,
    }
}

fn payload_to_wit(p: &domain::FramePayload) -> Result<wit::FramePayload, BridgeError> {
    use domain::FramePayload::*;

    Ok(match p {
        TaskAssign(b) => wit::FramePayload::TaskAssign(wit::TaskAssignBody {
            goal: b.goal.clone(),
            scope: b
                .scope
                .iter()
                .map(scope_to_wit)
                .collect::<Result<Vec<_>, _>>()?,
            success_criteria: b.success_criteria.clone(),
            posture_preferences: posture_prefs_to_wit(&b.posture_preferences)?,
            prior_distillate_ref: b.prior_distillate_ref.as_ref().map(|r| {
                wit::PriorDistillateRef {
                    digest_frame_id: r.digest_frame_id.to_vec(),
                    distillation_depth: r.distillation_depth,
                    intent_lineage: r
                        .intent_lineage
                        .as_slice()
                        .iter()
                        .map(|intent| intent.as_str().to_string())
                        .collect(),
                }
            }),
        }),
        TaskComplete(b) => wit::FramePayload::TaskComplete(wit::TaskCompleteBody {
            result_text: b.result.clone(),
        }),
        DecisionDispatch(b) => wit::FramePayload::DecisionDispatch(wit::DecisionDispatchBody {
            decision_id: b.decision_id,
            approved: b.approved,
            working_memory_digest_refs: b.working_memory_digest_refs.as_slice().to_vec(),
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
            original_kind: b.original_kind.map(frame_kind_to_wit),
        }),
        BudgetWarning(b) => wit::FramePayload::BudgetWarning(wit::BudgetEnvelope {
            spirit_pid: b.spirit_pid,
            hook_name: b.hook_name.clone(),
            wall_ns: b.wall_ns,
            cap_seconds: b.cap_seconds,
        }),
        BudgetExceeded(b) => wit::FramePayload::BudgetExceeded(wit::BudgetEnvelope {
            spirit_pid: b.spirit_pid,
            hook_name: b.hook_name.clone(),
            wall_ns: b.wall_ns,
            cap_seconds: b.cap_seconds,
        }),
        ConsentRupture(b) => wit::FramePayload::ConsentRupture(wit::ConsentRuptureBody {
            rupture_id: b.rupture_id.to_vec(),
            original_frame_id: b.original_frame_id.to_vec(),
            original_kind: frame_kind_to_wit(b.original_kind),
            accepted: b.accepted.iter().map(address_to_wit).collect(),
            rejected: b
                .rejected
                .iter()
                .map(|r| {
                    Ok(wit::RuptureRejection {
                        address: address_to_wit(&r.address),
                        reason: rupture_reason_to_wit(r.reason)?,
                    })
                })
                .collect::<Result<Vec<_>, BridgeError>>()?,
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
    })
}

fn frame_id_from_vec(v: Vec<u8>) -> Result<[u8; 16], BridgeError> {
    let len = v.len();
    v.try_into().map_err(|_| BridgeError::BadFrameIdLen(len))
}

fn hash_from_vec(v: Vec<u8>) -> Result<[u8; 32], BridgeError> {
    let len = v.len();
    v.try_into().map_err(|_| BridgeError::BadHashLen(len))
}

fn payload_from_wit(p: wit::FramePayload) -> Result<domain::FramePayload, BridgeError> {
    use domain::FramePayload as D;

    Ok(match p {
        wit::FramePayload::TaskAssign(b) => D::TaskAssign(domain::TaskAssignPayload {
            goal: b.goal,
            scope: b
                .scope
                .into_iter()
                .map(scope_from_wit)
                .collect::<Result<Vec<_>, _>>()?,
            success_criteria: b.success_criteria,
            posture_preferences: posture_prefs_from_wit(b.posture_preferences),
            prior_distillate_ref: b
                .prior_distillate_ref
                .map(|r| {
                    Ok(domain::PriorDistillateRef {
                        digest_frame_id: frame_id_from_vec(r.digest_frame_id)?,
                        distillation_depth: r.distillation_depth,
                        intent_lineage: maos_domain::invariants::i13::IntentLineage::new(
                            r.intent_lineage
                                .into_iter()
                                .map(maos_domain::invariants::i8::A2AIntent::new)
                                .collect(),
                        ),
                    })
                })
                .transpose()?,
        }),
        wit::FramePayload::TaskComplete(b) => D::TaskComplete(domain::TaskCompletePayload {
            result: b.result_text,
        }),
        wit::FramePayload::DecisionDispatch(b) => {
            D::DecisionDispatch(domain::DecisionDispatchPayload {
                decision_id: b.decision_id,
                approved: b.approved,
                working_memory_digest_refs:
                    maos_domain::invariants::i12::WorkingMemoryDigestRefs::new(
                        b.working_memory_digest_refs,
                    ),
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
        wit::FramePayload::TelemetryEvent(b) => D::TelemetryEvent(domain::TelemetryEventPayload {
            event_type: b.event_type,
            data: b.data,
        }),
        wit::FramePayload::ConsentRequest(b) => D::ConsentRequest(domain::ConsentRequestPayload {
            capability: b.capability,
        }),
        wit::FramePayload::Retract(b) => D::Retract(domain::RetractPayload {
            original_frame_id: frame_id_from_vec(b.original_frame_id)?,
            reason: b.reason,
            original_kind: b.original_kind.map(frame_kind_from_wit),
        }),
        wit::FramePayload::BudgetWarning(b) => D::BudgetWarning(domain::BudgetEnvelope {
            spirit_pid: b.spirit_pid,
            hook_name: b.hook_name,
            wall_ns: b.wall_ns,
            cap_seconds: b.cap_seconds,
        }),
        wit::FramePayload::BudgetExceeded(b) => D::BudgetExceeded(domain::BudgetEnvelope {
            spirit_pid: b.spirit_pid,
            hook_name: b.hook_name,
            wall_ns: b.wall_ns,
            cap_seconds: b.cap_seconds,
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

fn scope_to_wit(scope: &maos_domain::invariants::i1::Scope) -> Result<wit::Scope, BridgeError> {
    use maos_domain::invariants::i1::Scope as D;

    Ok(match scope {
        D::FsRead { subtree } => wit::Scope::FsRead(subtree.clone()),
        D::FsWrite { subtree } => wit::Scope::FsWrite(subtree.clone()),
        D::NetHttps { domain } => wit::Scope::NetHttps(domain.clone()),
        D::ProcExec { binary } => wit::Scope::ProcExec(binary.clone()),
        D::SubSpiritSpawn { class } => wit::Scope::SubSpiritSpawn(class.clone()),
        D::ProviderInfer { provider } => wit::Scope::ProviderInfer(provider.clone()),
        D::IacSend { peer_class } => wit::Scope::IacSend(peer_class.clone()),
        D::MemRead { scope } => wit::Scope::MemRead(scope.clone()),
        D::MemWrite { scope } => wit::Scope::MemWrite(scope.clone()),
        D::SelfTelemetryRead => wit::Scope::SelfTelemetryRead,
        D::LogRecall => wit::Scope::LogRecall,
        D::LogFetch => wit::Scope::LogFetch,
        D::DistillateWrite => wit::Scope::DistillateWrite,
        D::McpCall { server, tool } => wit::Scope::McpCall(wit::ScopeMcpCall {
            server: server.clone(),
            tool: tool.clone(),
        }),
        D::CliSubprocessSpawn {
            cli_binary_path,
            argv_prefix_hash,
            output_shape_version,
        } => wit::Scope::CliSubprocessSpawn(wit::ScopeCliSubprocessSpawn {
            cli_binary_path: cli_binary_path.clone(),
            argv_prefix_hash: argv_prefix_hash.to_vec(),
            output_shape_version: output_shape_version.clone(),
        }),
        D::GatewaySend {
            gateway_id,
            recipient,
        } => wit::Scope::GatewaySend(wit::ScopeGatewaySend {
            gateway_id: gateway_id.clone(),
            recipient: recipient.clone(),
        }),
        D::SkillAuthorSelf => wit::Scope::SkillAuthorSelf,
        D::LoomRead => wit::Scope::LoomRead,
        D::LoomWrite => wit::Scope::LoomWrite,
        D::LoomScan => wit::Scope::LoomScan,
        _ => return Err(BridgeError::Unrepresentable { ty: "Scope" }),
    })
}

fn scope_from_wit(scope: wit::Scope) -> Result<maos_domain::invariants::i1::Scope, BridgeError> {
    use maos_domain::invariants::i1::Scope as D;

    Ok(match scope {
        wit::Scope::FsRead(subtree) => D::FsRead { subtree },
        wit::Scope::FsWrite(subtree) => D::FsWrite { subtree },
        wit::Scope::NetHttps(domain) => D::NetHttps { domain },
        wit::Scope::ProcExec(binary) => D::ProcExec { binary },
        wit::Scope::SubSpiritSpawn(class) => D::SubSpiritSpawn { class },
        wit::Scope::ProviderInfer(provider) => D::ProviderInfer { provider },
        wit::Scope::IacSend(peer_class) => D::IacSend { peer_class },
        wit::Scope::MemRead(scope) => D::MemRead { scope },
        wit::Scope::MemWrite(scope) => D::MemWrite { scope },
        wit::Scope::SelfTelemetryRead => D::SelfTelemetryRead,
        wit::Scope::LogRecall => D::LogRecall,
        wit::Scope::LogFetch => D::LogFetch,
        wit::Scope::DistillateWrite => D::DistillateWrite,
        wit::Scope::McpCall(call) => D::McpCall {
            server: call.server,
            tool: call.tool,
        },
        wit::Scope::CliSubprocessSpawn(spawn) => D::CliSubprocessSpawn {
            cli_binary_path: spawn.cli_binary_path,
            argv_prefix_hash: hash_from_vec(spawn.argv_prefix_hash)?,
            output_shape_version: spawn.output_shape_version,
        },
        wit::Scope::GatewaySend(send) => D::GatewaySend {
            gateway_id: send.gateway_id,
            recipient: send.recipient,
        },
        wit::Scope::SkillAuthorSelf => D::SkillAuthorSelf,
        wit::Scope::LoomRead => D::LoomRead,
        wit::Scope::LoomWrite => D::LoomWrite,
        wit::Scope::LoomScan => D::LoomScan,
    })
}

fn intent_to_wit(intent: maos_domain::invariants::i1::IntentClass) -> wit::IntentClass {
    match intent {
        maos_domain::invariants::i1::IntentClass::HighPrivilege => wit::IntentClass::HighPrivilege,
        maos_domain::invariants::i1::IntentClass::Standard => wit::IntentClass::Standard,
        maos_domain::invariants::i1::IntentClass::Readonly => wit::IntentClass::Readonly,
    }
}

fn intent_from_wit(intent: wit::IntentClass) -> maos_domain::invariants::i1::IntentClass {
    match intent {
        wit::IntentClass::HighPrivilege => maos_domain::invariants::i1::IntentClass::HighPrivilege,
        wit::IntentClass::Standard => maos_domain::invariants::i1::IntentClass::Standard,
        wit::IntentClass::Readonly => maos_domain::invariants::i1::IntentClass::Readonly,
    }
}

fn consent_to_wit(consent: &domain::ConsentEnvelope) -> wit::ConsentEnvelope {
    wit::ConsentEnvelope {
        consent_id: consent.consent_id.to_vec(),
        granter: address_to_wit(&consent.granter),
        timestamp_ns: consent.timestamp_ns,
        intent_class: consent
            .intent_class
            .as_ref()
            .map(|intent| intent.as_str().to_string()),
        valid_until_ns: consent.valid_until_ns,
    }
}

fn consent_from_wit(consent: wit::ConsentEnvelope) -> Result<domain::ConsentEnvelope, BridgeError> {
    Ok(domain::ConsentEnvelope {
        consent_id: frame_id_from_vec(consent.consent_id)?,
        granter: address_from_wit(consent.granter),
        timestamp_ns: consent.timestamp_ns,
        intent_class: consent
            .intent_class
            .map(maos_domain::invariants::i8::A2AIntent::new),
        valid_until_ns: consent.valid_until_ns,
    })
}

/// Lower a domain `IacFrame` into the WIT shape the guest consumes.
pub fn lower(frame: &domain::IacFrame) -> Result<wit::IacFrame, BridgeError> {
    Ok(wit::IacFrame {
        frame_id: frame.frame_id.to_vec(),
        timestamp_ns: frame.timestamp_ns,
        logical_clock: frame.logical_clock,
        frame_from: address_to_wit(&frame.from),
        to: frame.to.iter().map(address_to_wit).collect(),
        kind: frame_kind_to_wit(frame.kind),
        intent: intent_to_wit(frame.intent),
        payload: payload_to_wit(&frame.payload)?,
        auto_marker: origin_to_wit(frame.auto_marker),
        consent_envelope: frame.consent_envelope.as_ref().map(consent_to_wit),
        intent_lineage: frame
            .intent_lineage
            .as_slice()
            .iter()
            .map(|intent| intent.as_str().to_string())
            .collect(),
    })
}

/// Lift a WIT `IacFrame` emitted by the guest back into its domain shape.
pub fn lift(frame: wit::IacFrame) -> Result<domain::IacFrame, BridgeError> {
    Ok(domain::IacFrame {
        frame_id: frame_id_from_vec(frame.frame_id)?,
        timestamp_ns: frame.timestamp_ns,
        logical_clock: frame.logical_clock,
        from: address_from_wit(frame.frame_from),
        to: frame.to.into_iter().map(address_from_wit).collect(),
        kind: frame_kind_from_wit(frame.kind),
        intent: intent_from_wit(frame.intent),
        payload: payload_from_wit(frame.payload)?,
        auto_marker: origin_from_wit(frame.auto_marker),
        consent_envelope: frame.consent_envelope.map(consent_from_wit).transpose()?,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::new(
            frame
                .intent_lineage
                .into_iter()
                .map(maos_domain::invariants::i8::A2AIntent::new)
                .collect(),
        ),
    })
}
