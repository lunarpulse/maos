//! Story 17-3b AC2 — direct lossless round-trip coverage for
//! `frame_bridge::lower`/`lift`.
//!
//! The e2e suite proves a real guest subprocess path. These tests isolate the
//! codec and cover every payload and frame-kind discriminator without a
//! wasmtime process. Every WIT-representable domain field must survive:
//! `encode_cbor(lift(lower(frame))) == encode_cbor(frame)`.
//!
//! The hostile vectors exercise guest-controlled fixed-size byte fields and
//! future domain variants. The bridge is a codec rather than an authorization
//! point, but it must never construct invalid fixed-size domain values or
//! silently map an unrepresentable variant.

#![cfg(test)]

use maos_domain::frame::{
    self, ConsentEnvelope, ConsentRequestPayload, ConsentRupturePayload, DecisionDispatchPayload,
    EpistemicHaltPayload, FrameAddress, FramePayload, HaltPolicyOverride, IacFrame, PostureHint,
    PosturePreferences, PriorDistillateRef, RateLimitedPayload, RetractPayload, RuptureRejection,
    TaskAssignPayload, TaskCompletePayload, TelemetryEventPayload,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use smallvec::SmallVec;

use maos_wasm_host::codec;
use maos_wasm_host::frame_bridge::{lift, lower, BridgeError};
use maos_wasm_host::wit_guest::maos::spirit::frames as wit;

// ── Envelope scaffolding ────────────────────────────────────────────────

const FRAME_ID: [u8; 16] = [0xAB; 16];

fn address(role: Option<SpiritRole>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId("spirit-7".into()),
        host_id: Some(HostId("host-a".into())),
        role,
    }
}

/// Build a frame envelope around `payload`, cycling `kind` independently so the
/// 6 `FrameKind`s that carry no `FramePayload` variant (CapabilityInvocation,
/// SandboxBlock, InferenceCall, CliSubprocessOutput, GatewayInbound,
/// GatewayOutbound) can still exercise the kind discriminator round-trip below.
fn envelope(kind: FrameKind, payload: FramePayload) -> IacFrame {
    let mut to: SmallVec<[FrameAddress; 1]> = SmallVec::new();
    to.push(address(Some(SpiritRole::Worker)));
    to.push(address(None));
    IacFrame {
        frame_id: FRAME_ID,
        timestamp_ns: 1_700_000_000_000,
        logical_clock: 42,
        from: address(Some(SpiritRole::Director)),
        to,
        kind,
        intent: IntentClass::Standard,
        payload,
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

/// Assert all envelope fields survive the lossless WIT projection.
fn assert_envelope_round_trips(original: &IacFrame, round: &IacFrame) {
    assert_eq!(
        round.frame_id, original.frame_id,
        "frame_id must round-trip"
    );
    assert_eq!(round.timestamp_ns, original.timestamp_ns);
    assert_eq!(round.logical_clock, original.logical_clock);
    assert_eq!(round.kind, original.kind, "FrameKind must round-trip");
    assert_eq!(
        round.auto_marker, original.auto_marker,
        "FrameOrigin must round-trip"
    );
    assert_eq!(round.from, original.from);
    assert_eq!(round.to, original.to);
    assert_eq!(round.intent, original.intent);
    assert_eq!(round.consent_envelope, original.consent_envelope);
    assert_eq!(round.intent_lineage, original.intent_lineage);
}

fn round_trip(frame: &IacFrame) -> IacFrame {
    let lowered = lower(frame).expect("lower must represent a well-formed frame");
    lift(lowered).expect("lift must accept a lowered well-formed frame")
}

/// Minimal defaults — `TaskAssignPayload` has no `Default`, so spell it out.
fn task_assign_defaults() -> TaskAssignPayload {
    TaskAssignPayload {
        goal: String::new(),
        scope: vec![],
        success_criteria: String::new(),
        posture_preferences: PosturePreferences::default(),
        prior_distillate_ref: None,
    }
}

// ── FrameKind discriminator: all 15 variants ────────────────────────────

#[test]
fn all_15_frame_kinds_round_trip_through_lower_lift() {
    // Only TaskAssign was covered by the e2e suite; the other 14 discriminants
    // (including the 6 payload-less audit/gateway kinds) must also survive the
    // WIT kind enum round-trip — a new WIT variant or a renumbering would
    // otherwise hide here.
    let dummy = FramePayload::TaskComplete(TaskCompletePayload {
        result: "carrier".into(),
    });
    let all = [
        FrameKind::TaskAssign,
        FrameKind::TaskComplete,
        FrameKind::DecisionDispatch,
        FrameKind::EpistemicHalt,
        FrameKind::TelemetryEvent,
        FrameKind::ConsentRequest,
        FrameKind::Retract,
        FrameKind::CapabilityInvocation,
        FrameKind::SandboxBlock,
        FrameKind::InferenceCall,
        FrameKind::BudgetWarning,
        FrameKind::BudgetExceeded,
        FrameKind::CliSubprocessOutput,
        FrameKind::ConsentRupture,
        FrameKind::RateLimited,
        FrameKind::GatewayInbound,
        FrameKind::GatewayOutbound,
    ];
    assert_eq!(all.len(), 17, "sanity: the frame set has 17 kinds");

    for kind in all {
        let frame = envelope(kind, dummy.clone());
        let round = round_trip(&frame);
        assert_eq!(
            round.kind, kind,
            "FrameKind {kind:?} must round-trip through the WIT kind enum"
        );
    }
}

// ── Payload variants: all 9 ─────────────────────────────────────────────

#[test]
fn task_assign_payload_round_trips() {
    let payload = TaskAssignPayload {
        goal: "ship 11.1a".into(),
        success_criteria: "all gates green".into(),
        ..task_assign_defaults()
    };
    let frame = envelope(FrameKind::TaskAssign, FramePayload::TaskAssign(payload));
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::TaskAssign(got) = &round.payload else {
        panic!("payload variant must round-trip to TaskAssign");
    };
    assert_eq!(got.goal, "ship 11.1a");
    assert_eq!(got.success_criteria, "all gates green");
    assert_eq!(got.prior_distillate_ref, None);
}

#[test]
fn task_complete_payload_round_trips() {
    let payload = TaskCompletePayload {
        result: "done".into(),
    };
    let frame = envelope(FrameKind::TaskComplete, FramePayload::TaskComplete(payload));
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::TaskComplete(got) = &round.payload else {
        panic!("expected TaskComplete");
    };
    assert_eq!(got.result, "done");
}

#[test]
fn decision_dispatch_payload_round_trips() {
    let payload = DecisionDispatchPayload {
        decision_id: 99,
        approved: true,
        working_memory_digest_refs: Default::default(),
    };
    let frame = envelope(
        FrameKind::DecisionDispatch,
        FramePayload::DecisionDispatch(payload),
    );
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::DecisionDispatch(got) = &round.payload else {
        panic!("expected DecisionDispatch");
    };
    assert_eq!(got.decision_id, 99);
    assert!(got.approved);
}

#[test]
fn epistemic_halt_payload_round_trips() {
    let payload = EpistemicHaltPayload {
        halt_id: "halt-1".into(),
        tag: "confidence".into(),
        value: 0.42,
        threshold: Some(0.9),
        policy_id: "policy-7".into(),
        derived_from: "frame-9".into(),
    };
    let frame = envelope(
        FrameKind::EpistemicHalt,
        FramePayload::EpistemicHalt(payload),
    );
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::EpistemicHalt(got) = &round.payload else {
        panic!("expected EpistemicHalt");
    };
    assert_eq!(got.halt_id, "halt-1");
    assert_eq!(got.tag, "confidence");
    assert!(
        (got.value - 0.42_f32).abs() < 1e-6,
        "f32 value must round-trip"
    );
    assert_eq!(
        got.threshold,
        Some(0.9_f32),
        "Option<f32> threshold must round-trip"
    );
    assert_eq!(got.policy_id, "policy-7");
    assert_eq!(got.derived_from, "frame-9");
}

#[test]
fn telemetry_event_payload_round_trips() {
    let payload = TelemetryEventPayload {
        event_type: "latency".into(),
        data: r#"{"p99_ms":42}"#.into(),
    };
    let frame = envelope(
        FrameKind::TelemetryEvent,
        FramePayload::TelemetryEvent(payload),
    );
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::TelemetryEvent(got) = &round.payload else {
        panic!("expected TelemetryEvent");
    };
    assert_eq!(got.event_type, "latency");
    assert_eq!(got.data, r#"{"p99_ms":42}"#);
}

#[test]
fn consent_request_payload_round_trips() {
    let payload = ConsentRequestPayload {
        capability: "fs:read:/tmp".into(),
    };
    let frame = envelope(
        FrameKind::ConsentRequest,
        FramePayload::ConsentRequest(payload),
    );
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::ConsentRequest(got) = &round.payload else {
        panic!("expected ConsentRequest");
    };
    assert_eq!(got.capability, "fs:read:/tmp");
}

#[test]
fn retract_payload_round_trips() {
    let payload = RetractPayload {
        original_frame_id: [0x01; 16],
        reason: "superseded".into(),
        original_kind: Some(FrameKind::TaskAssign),
    };
    let frame = envelope(FrameKind::Retract, FramePayload::Retract(payload));
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::Retract(got) = &round.payload else {
        panic!("expected Retract");
    };
    assert_eq!(got.original_frame_id, [0x01; 16]);
    assert_eq!(got.reason, "superseded");
    assert_eq!(got.original_kind, Some(FrameKind::TaskAssign));
}

#[test]
fn consent_rupture_peer_identity_unverified_round_trips() {
    let payload = ConsentRupturePayload {
        rupture_id: [0x02; 16],
        original_frame_id: [0x03; 16],
        original_kind: FrameKind::DecisionDispatch,
        accepted: vec![address(Some(SpiritRole::Worker))],
        rejected: vec![RuptureRejection {
            address: address(None),
            reason: frame::RuptureReason::PeerIdentityUnverified,
        }],
        ruptured_at_ns: 9_999,
    };
    let frame = envelope(
        FrameKind::ConsentRupture,
        FramePayload::ConsentRupture(payload),
    );
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::ConsentRupture(got) = &round.payload else {
        panic!("expected ConsentRupture");
    };
    assert_eq!(got.rupture_id, [0x02; 16]);
    assert_eq!(got.original_frame_id, [0x03; 16]);
    assert_eq!(got.original_kind, FrameKind::DecisionDispatch);
    assert_eq!(got.accepted.len(), 1);
    assert_eq!(got.rejected.len(), 1);
    assert_eq!(
        got.rejected[0].reason,
        frame::RuptureReason::PeerIdentityUnverified
    );
    assert_eq!(got.ruptured_at_ns, 9_999);
}

#[test]
fn rate_limited_payload_round_trips() {
    let payload = RateLimitedPayload {
        provider_id: "openai".into(),
        credential_fingerprint_prefix_hex: "deadbeef".into(),
        retry_after_ms: 1500,
        bucket_remaining: 0,
        bucket_capacity: 60,
        refill_per_sec: 1,
        schedule_id: Some("sched-3".into()),
    };
    let frame = envelope(FrameKind::RateLimited, FramePayload::RateLimited(payload));
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::RateLimited(got) = &round.payload else {
        panic!("expected RateLimited");
    };
    assert_eq!(got.provider_id, "openai");
    assert_eq!(got.retry_after_ms, 1500);
    assert_eq!(got.bucket_capacity, 60);
    assert_eq!(got.schedule_id.as_deref(), Some("sched-3"));
}

// ── Lossless nested-field oracles and hostile guest vectors ─────────────

fn lineage(values: &[&str]) -> IntentLineage {
    IntentLineage::new(values.iter().map(|value| A2AIntent::new(*value)).collect())
}

fn all_scope_variants() -> Vec<Scope> {
    vec![
        Scope::FsRead {
            subtree: "/read".into(),
        },
        Scope::FsWrite {
            subtree: "/write".into(),
        },
        Scope::NetHttps {
            domain: "api.example.test".into(),
        },
        Scope::ProcExec {
            binary: "/usr/bin/true".into(),
        },
        Scope::SubSpiritSpawn {
            class: "worker".into(),
        },
        Scope::ProviderInfer {
            provider: "example-provider".into(),
        },
        Scope::IacSend {
            peer_class: "director".into(),
        },
        Scope::MemRead {
            scope: "notes".into(),
        },
        Scope::MemWrite {
            scope: "notes".into(),
        },
        Scope::SelfTelemetryRead,
        Scope::LogRecall,
        Scope::LogFetch,
        Scope::DistillateWrite,
        Scope::McpCall {
            server: "tools".into(),
            tool: "search".into(),
        },
        Scope::CliSubprocessSpawn {
            cli_binary_path: "/usr/bin/git".into(),
            argv_prefix_hash: [0x41; 32],
            output_shape_version: "v1".into(),
        },
        Scope::GatewaySend {
            gateway_id: "gateway-a".into(),
            recipient: "recipient-a".into(),
        },
        Scope::SkillAuthorSelf,
        Scope::LoomRead,
        Scope::LoomWrite,
        Scope::LoomScan,
    ]
}

fn task_assign_oracle_frame() -> IacFrame {
    let mut frame = envelope(
        FrameKind::TaskAssign,
        FramePayload::TaskAssign(TaskAssignPayload {
            goal: "preserve every scope".into(),
            scope: all_scope_variants(),
            success_criteria: "byte-identical canonical CBOR".into(),
            posture_preferences: PosturePreferences {
                preferred_posture: Some(PostureHint::Assistive),
                halt_policy_overrides: vec![HaltPolicyOverride {
                    tag: "confidence".into(),
                    recall_vs_precision: 0.25,
                }],
            },
            prior_distillate_ref: Some(PriorDistillateRef {
                digest_frame_id: [0xD1; 16],
                distillation_depth: 3,
                intent_lineage: lineage(&["source", "distillate"]),
            }),
        }),
    );
    frame.intent = IntentClass::HighPrivilege;
    frame.intent_lineage = lineage(&["request", "assignment"]);
    frame
}

fn assert_canonical_round_trip(frame: &IacFrame) -> usize {
    let expected = codec::encode_cbor(frame).expect("canonical input must encode");
    let actual =
        codec::encode_cbor(&round_trip(frame)).expect("canonical round-trip output must encode");
    assert_eq!(
        actual, expected,
        "lower/lift must preserve canonical CBOR bytes"
    );
    actual.len()
}

#[test]
fn task_assign_nested_fields_and_all_scope_variants_are_byte_lossless() {
    let bytes = assert_canonical_round_trip(&task_assign_oracle_frame());
    println!("TaskAssign oracle canonical CBOR byte count: {bytes}");
}

#[test]
fn decision_dispatch_digest_refs_are_byte_lossless() {
    let mut frame = envelope(
        FrameKind::DecisionDispatch,
        FramePayload::DecisionDispatch(DecisionDispatchPayload {
            decision_id: 47,
            approved: true,
            working_memory_digest_refs: WorkingMemoryDigestRefs::new(vec![
                "digest-a".into(),
                "digest-b".into(),
            ]),
        }),
    );
    frame.intent_lineage = lineage(&["decision", "operator-approved"]);
    assert_canonical_round_trip(&frame);
}

#[test]
fn consent_envelope_intent_and_lineage_are_byte_lossless() {
    let mut frame = envelope(
        FrameKind::TaskComplete,
        FramePayload::TaskComplete(TaskCompletePayload {
            result: "completed with consent".into(),
        }),
    );
    frame.intent = IntentClass::HighPrivilege;
    frame.consent_envelope = Some(ConsentEnvelope {
        consent_id: [0xC0; 16],
        granter: address(Some(SpiritRole::Observer)),
        timestamp_ns: 9_876_543,
        intent_class: Some(A2AIntent::new("diagnosis-handoff:read-only-evidence")),
        valid_until_ns: Some(9_999_999),
    });
    frame.intent_lineage = lineage(&["operator", "handoff"]);
    assert_canonical_round_trip(&frame);
}

#[test]
fn guest_frame_id_with_15_bytes_is_rejected() {
    let mut wire = lower(&task_assign_oracle_frame()).expect("oracle frame must lower");
    wire.frame_id = vec![0; 15];
    assert!(matches!(lift(wire), Err(BridgeError::BadFrameIdLen(15))));
}

#[test]
fn guest_argv_prefix_hash_with_31_bytes_is_rejected() {
    let mut wire = lower(&task_assign_oracle_frame()).expect("oracle frame must lower");
    let wit::FramePayload::TaskAssign(task) = &mut wire.payload else {
        panic!("oracle frame must lower as TaskAssign");
    };
    let Some(wit::Scope::CliSubprocessSpawn(spawn)) = task
        .scope
        .iter_mut()
        .find(|scope| matches!(scope, wit::Scope::CliSubprocessSpawn(_)))
    else {
        panic!("oracle frame must carry a CLI subprocess scope");
    };
    spawn.argv_prefix_hash = vec![0; 31];

    assert!(matches!(lift(wire), Err(BridgeError::BadHashLen(31))));
}

#[test]
fn four_mib_guest_intent_lineage_lifts_without_panicking_or_truncation() {
    const ENTRIES: usize = 4_096;
    const ENTRY_BYTES: usize = 1_024;
    let mut guest_frame = lower(&task_assign_oracle_frame()).expect("oracle frame must lower");
    let suffix = "x".repeat(ENTRY_BYTES - 4);
    guest_frame.intent_lineage = (0..ENTRIES)
        .map(|index| format!("{index:04}{suffix}"))
        .collect();
    let guest_lineage = guest_frame.intent_lineage.clone();

    let domain = lift(guest_frame).expect("4 MiB guest lineage must lift");
    assert_eq!(domain.intent_lineage.as_slice().len(), ENTRIES);
    assert!(
        domain
            .intent_lineage
            .as_slice()
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>()
            >= 4 * 1024 * 1024
    );
    let returned = lower(&domain).expect("lifted lineage must lower");
    assert_eq!(returned.intent_lineage, guest_lineage);
    let encoded = codec::encode_cbor(&domain).expect("4 MiB frame fits ADR-032 cap");
    assert!(encoded.len() > 4 * 1024 * 1024);
    assert!(encoded.len() <= codec::MAX_FRAME_BYTES);
}
