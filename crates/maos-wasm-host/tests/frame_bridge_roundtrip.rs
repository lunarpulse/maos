//! Story 11.1a AC2/AC3 — direct round-trip coverage for `frame_bridge::lower`/`lift`.
//!
//! The e2e suite (`e2e_roundtrip.rs`) proves ONE frame kind (`TaskAssign`) end to
//! end through a real wasmtime guest subprocess. That is the right integration
//! depth for "the pipe works" but it leaves 14 of 15 `FrameKind` discriminants
//! and 8 of 9 `FramePayload` variants unexercised across the WIT boundary — the
//! single most likely place for a silent lower/lift asymmetry to hide.
//!
//! These tests pin the round-trip contract directly (no subprocess, no wasmtime):
//! for every payload variant and every frame-kind discriminator, `lift(lower(f))`
//! MUST preserve the fields the WIT world carries, and MUST drop/default the
//! three documented-lossy fields (`intent`, `consent_envelope`, `intent_lineage`)
//! and the lossy `Scope` Debug-string projection — explicitly, so a future WIT
//! revision that adds them flips these assertions RED rather than passing
//! silently on stale assumptions.
//!
//! Per D5, the byte-equal oracle's correctness rests on the lower/lift pair
//! being a faithful (if intentionally narrowed) projection; this file makes
//! that faithfulness mechanical, not hoped.

#![cfg(test)]

use maos_domain::frame::{
    self, ConsentRequestPayload, ConsentRupturePayload, DecisionDispatchPayload, EpistemicHaltPayload,
    FrameAddress, FramePayload, IacFrame, PosturePreferences, RateLimitedPayload, RetractPayload,
    RuptureRejection, TaskAssignPayload, TaskCompletePayload, TelemetryEventPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i13::IntentLineage;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use smallvec::SmallVec;

use maos_wasm_host::frame_bridge::{lift, lower};

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

/// Assert the WIT-carried envelope fields survive `lower`/`lift`. The three
/// lossy fields (`intent`, `consent_envelope`, `intent_lineage`) are checked in
/// their own dedicated tests below — never silently folded into a pass here.
fn assert_envelope_round_trips(original: &IacFrame, round: &IacFrame) {
    assert_eq!(round.frame_id, original.frame_id, "frame_id must round-trip");
    assert_eq!(round.timestamp_ns, original.timestamp_ns);
    assert_eq!(round.logical_clock, original.logical_clock);
    assert_eq!(round.kind, original.kind, "FrameKind must round-trip");
    assert_eq!(round.auto_marker, original.auto_marker, "FrameOrigin must round-trip");
    assert_eq!(round.from.spirit_id, original.from.spirit_id);
    assert_eq!(round.from.host_id, original.from.host_id);
    assert_eq!(round.from.role, original.from.role);
    assert_eq!(round.to.len(), original.to.len(), "recipient count must round-trip");
    for (got, want) in round.to.iter().zip(original.to.iter()) {
        assert_eq!(got.spirit_id, want.spirit_id);
        assert_eq!(got.host_id, want.host_id);
        assert_eq!(got.role, want.role);
    }
}

fn round_trip(frame: &IacFrame) -> IacFrame {
    lift(lower(frame)).expect("lower/lift must succeed for a well-formed frame")
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
        FrameKind::CliSubprocessOutput,
        FrameKind::ConsentRupture,
        FrameKind::RateLimited,
        FrameKind::GatewayInbound,
        FrameKind::GatewayOutbound,
    ];
    assert_eq!(all.len(), 15, "sanity: the 11.1a frame set has 15 kinds");

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
fn task_assign_scope_round_trips_lossy_debug_projection() {
    // D5 documented limitation: `Scope` has no WIT type and rides as its Debug
    // string; `scope_from_debug_string` returns None, so a non-empty scope
    // round-trips to EMPTY. Pin this explicitly — if a future WIT revision adds
    // a real Scope type, this assertion flips RED and forces the fix.
    let payload = TaskAssignPayload {
        scope: vec![
            Scope::FsRead { subtree: "/tmp".into() },
            Scope::SelfTelemetryRead,
        ],
        ..task_assign_defaults()
    };
    let frame = envelope(FrameKind::TaskAssign, FramePayload::TaskAssign(payload));
    let round = round_trip(&frame);
    let FramePayload::TaskAssign(got) = &round.payload else {
        panic!("expected TaskAssign");
    };
    assert!(
        got.scope.is_empty(),
        "Scope is a lossy Debug projection today — non-empty scope MUST collapse to empty \
         (tracked D5 limitation); got {} entries",
        got.scope.len()
    );
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
    let frame = envelope(FrameKind::DecisionDispatch, FramePayload::DecisionDispatch(payload));
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
    let frame = envelope(FrameKind::EpistemicHalt, FramePayload::EpistemicHalt(payload));
    let round = round_trip(&frame);
    assert_envelope_round_trips(&frame, &round);
    let FramePayload::EpistemicHalt(got) = &round.payload else {
        panic!("expected EpistemicHalt");
    };
    assert_eq!(got.halt_id, "halt-1");
    assert_eq!(got.tag, "confidence");
    assert!((got.value - 0.42_f32).abs() < 1e-6, "f32 value must round-trip");
    assert_eq!(got.threshold, Some(0.9_f32), "Option<f32> threshold must round-trip");
    assert_eq!(got.policy_id, "policy-7");
    assert_eq!(got.derived_from, "frame-9");
}

#[test]
fn telemetry_event_payload_round_trips() {
    let payload = TelemetryEventPayload {
        event_type: "latency".into(),
        data: r#"{"p99_ms":42}"#.into(),
    };
    let frame = envelope(FrameKind::TelemetryEvent, FramePayload::TelemetryEvent(payload));
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
    let frame = envelope(FrameKind::ConsentRequest, FramePayload::ConsentRequest(payload));
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
fn consent_rupture_payload_round_trips() {
    let payload = ConsentRupturePayload {
        rupture_id: [0x02; 16],
        original_frame_id: [0x03; 16],
        original_kind: FrameKind::DecisionDispatch,
        accepted: vec![address(Some(SpiritRole::Worker))],
        rejected: vec![RuptureRejection {
            address: address(None),
            reason: frame::RuptureReason::TokenRevoked,
        }],
        ruptured_at_ns: 9_999,
    };
    let frame = envelope(FrameKind::ConsentRupture, FramePayload::ConsentRupture(payload));
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
    assert_eq!(got.rejected[0].reason, frame::RuptureReason::TokenRevoked);
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

// ── Documented-lossy fields: pinned explicitly (flip RED when WIT grows) ─

#[test]
fn intent_field_round_trips_lossy_to_readonly() {
    // The WIT `iac-frame` omits `intent`; lift defaults to Readonly. Pinned so a
    // future WIT revision carrying intent flips this RED instead of silently
    // discarding a Standard frame.
    let frame = envelope(
        FrameKind::TaskAssign,
        FramePayload::TaskAssign(task_assign_defaults()),
    );
    let round = round_trip(&frame);
    assert_eq!(
        round.intent,
        IntentClass::Readonly,
        "intent is dropped on lower and defaulted to Readonly on lift (tracked gap)"
    );
    assert_ne!(round.intent, frame.intent, "sanity: the original was NOT Readonly");
}

#[test]
fn consent_envelope_round_trips_lossy_to_none() {
    let frame = envelope(
        FrameKind::TaskAssign,
        FramePayload::TaskAssign(task_assign_defaults()),
    );
    let round = round_trip(&frame);
    assert_eq!(
        round.consent_envelope, None,
        "consent_envelope is not representable in WIT v2.0 (tracked gap)"
    );
}
