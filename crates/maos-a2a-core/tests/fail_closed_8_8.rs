//! Story 8.8 — fail-closed cross-Host A2A consent (closes audit G7).
//!
//! The cross-Host router (`A2ARouterCore`) defaults FAIL-CLOSED: a frame whose
//! consent is **unclassified** (absent / non-canonical / oversized `intent_class`)
//! is DENIED with the distinct `CODE_CONSENT_UNCLASSIFIED` (-32009) — at BOTH the
//! send seam (`prepare_outbound` → `A2AError::ConsentUnclassified { Send }`, frame
//! never leaves) and the accept seam (`handle_intake` → -32009 NACK) — and is
//! NEVER silently downgraded to the coarse 3-band projection. A classified frame
//! is handled exactly as in 8.7 (admitted iff allowlisted; -32001 otherwise — no
//! conflation). Story 8.8 Option 2 (team consensus 2026-06-07): there is NO
//! band-fallback opt-in — fail-closed is unconditional, so no silent-downgrade
//! surface exists at all (G7's mechanism is removed, not just toggled off).

use maos_a2a_core::error::{A2AError, IntentDirection, UnclassifiedReason};
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_CONSENT_UNCLASSIFIED, CODE_INTENT_DENIED,
    METHOD_IAC_DELIVER,
};
use maos_a2a_core::{
    map_a2a_error_to_iac_bus, A2APeerConfig, A2AProfile, A2ARouterCore, ConsentAllowlists,
    InMemoryTofuPinStore, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;
use std::sync::Arc;

const FINE: &str = "diagnosis-handoff:read-only-evidence";

fn allow(send: &[&str], accept: &[&str]) -> ConsentAllowlists {
    ConsentAllowlists {
        send_allowlist: send.iter().map(|s| A2AIntent::new(*s)).collect(),
        accept_allowlist: accept.iter().map(|s| A2AIntent::new(*s)).collect(),
    }
}

fn peer_cfg(allowlists: ConsentAllowlists) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new("loopback"),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
        profile: A2AProfile::Loopback,
        allowlists,
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

async fn fail_closed_core(allowlists: ConsentAllowlists) -> A2ARouterCore {
    let cfg = peer_cfg(allowlists);
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    // Fail-closed is unconditional (Option 2, team consensus 2026-06-07).
    A2ARouterCore::new(vec![cfg], tofu)
}

/// `intent_class`: `None` ⇒ no envelope; `Some(s)` ⇒ envelope with that intent
/// (granter == from so the 8.9 granter gate passes).
fn frame(intent_class: Option<&str>) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("mira"),
        host_id: Some(HostId("loopback".into())),
        role: None,
    };
    IacFrame {
        frame_id: [7u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: Some(HostId("loopback".into())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Readonly,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "diagnose".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: intent_class.map(|s| ConsentEnvelope {
            consent_id: [0u8; 16],
            granter: from,
            timestamp_ns: 0,
            intent_class: Some(A2AIntent::new(s)),
            valid_until_ns: None,
        }),
        intent_lineage: IntentLineage::default(),
    }
}

/// A frame with a `Some(envelope)` but `intent_class: None` (the present-empty case).
fn frame_envelope_no_intent() -> IacFrame {
    let mut f = frame(Some(FINE));
    if let Some(env) = f.consent_envelope.as_mut() {
        env.intent_class = None;
    }
    f
}

async fn assert_accept_unclassified(core: &A2ARouterCore, f: IacFrame, expect: UnclassifiedReason) {
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, f, 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_CONSENT_UNCLASSIFIED,
                "accept side must deny unclassified with -32009 (got {})",
                n.error.code
            );
            let reason = n
                .error
                .data
                .as_ref()
                .and_then(|d| d.get("reason"))
                .and_then(|v| serde_json::from_value::<UnclassifiedReason>(v.clone()).ok())
                .expect("NACK data carries the reason");
            assert_eq!(reason, expect, "deny reason must be legible");
        }
        other => panic!("expected -32009 NACK, got {other:?}"),
    }
}

async fn assert_send_unclassified(core: &A2ARouterCore, f: IacFrame, expect: UnclassifiedReason) {
    match core
        .prepare_outbound(f, &HostId("loopback".into()), 0)
        .await
    {
        Err(A2AError::ConsentUnclassified {
            direction: IntentDirection::Send,
            reason,
        }) => {
            assert_eq!(reason, expect, "send-side deny reason must be legible");
        }
        other => panic!("expected send-side ConsentUnclassified, got {other:?}"),
    }
}

// ── (a) absent envelope → -32009 / ConsentUnclassified, BOTH directions ───────

#[tokio::test]
async fn absent_envelope_denied_both_directions() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    assert_accept_unclassified(&core, frame(None), UnclassifiedReason::Absent).await;
    assert_send_unclassified(&core, frame(None), UnclassifiedReason::Absent).await;
}

// ── (b) envelope present but intent_class None → Absent ───────────────────────

#[tokio::test]
async fn envelope_without_intent_class_denied() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    assert_accept_unclassified(
        &core,
        frame_envelope_no_intent(),
        UnclassifiedReason::Absent,
    )
    .await;
    assert_send_unclassified(
        &core,
        frame_envelope_no_intent(),
        UnclassifiedReason::Absent,
    )
    .await;
}

// ── (c) non-canonical intent_class → NonCanonical ─────────────────────────────

#[tokio::test]
async fn non_canonical_intent_denied_both_directions() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    assert_accept_unclassified(
        &core,
        frame(Some("Diagnosis Handoff")),
        UnclassifiedReason::NonCanonical,
    )
    .await;
    assert_send_unclassified(
        &core,
        frame(Some("Diagnosis Handoff")),
        UnclassifiedReason::NonCanonical,
    )
    .await;
}

// ── (d) oversized (129 bytes) intent_class → Oversized ────────────────────────

#[tokio::test]
async fn oversized_intent_denied_both_directions() {
    let over = "a".repeat(129);
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    assert_accept_unclassified(&core, frame(Some(&over)), UnclassifiedReason::Oversized).await;
    assert_send_unclassified(&core, frame(Some(&over)), UnclassifiedReason::Oversized).await;
}

// ── (e) classified-but-not-allowlisted → -32001 (NO conflation with -32009) ───

#[tokio::test]
async fn classified_not_allowlisted_is_minus_32001_not_minus_32009() {
    // accept_allowlist is empty; the classified frame is denied as
    // classified-but-not-allowlisted (-32001), NOT unclassified (-32009).
    let core = fail_closed_core(allow(&[FINE], &[])).await;
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(Some(FINE)), 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_INTENT_DENIED,
            "classified-but-not-allowlisted must be -32001, never -32009"
        ),
        other => panic!("expected -32001 NACK, got {other:?}"),
    }
    // Send side: classified intent not in send_allowlist → IntentDenied (not Unclassified).
    let core2 = fail_closed_core(allow(&[], &[FINE])).await;
    match core2
        .prepare_outbound(frame(Some(FINE)), &HostId("loopback".into()), 0)
        .await
    {
        Err(A2AError::IntentDenied {
            direction: IntentDirection::Send,
            ..
        }) => {}
        other => panic!("expected send-side IntentDenied, got {other:?}"),
    }
}

// ── (f) classified-and-allowlisted → admitted (8.7 behavior unchanged) ────────

#[tokio::test]
async fn classified_allowlisted_admitted() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    core.install_intake_sink(tx).await;

    // Send admits.
    let (req, _cfg, _id) = core
        .prepare_outbound(frame(Some(FINE)), &HostId("loopback".into()), 0)
        .await
        .expect("classified send admitted");
    // Accept admits.
    assert!(matches!(
        core.handle_intake(req).await,
        A2AJsonRpcResponse::Ack(_)
    ));
    let delivered = rx.recv().await.expect("delivered");
    assert_eq!(
        delivered
            .consent_envelope
            .and_then(|e| e.intent_class)
            .map(|i| i.as_str().to_string()),
        Some(FINE.to_string())
    );
}

// ── (g) NO band-fallback path exists (Option 2): a band-token allowlist does
// NOT admit an unclassified frame — it is denied, never downgraded ─────────────

#[tokio::test]
async fn no_band_fallback_unclassified_denied_even_with_band_allowlist() {
    // Even when the allowlist holds the exact band token the frame would have
    // projected to (`readonly`), a None-envelope frame is DENIED — there is no
    // band-downgrade path to admit it. This is the G7 closure: channel-shaped
    // band consent can never stand in for an unclassified frame.
    let core = fail_closed_core(allow(&["readonly"], &["readonly"])).await;
    assert_send_unclassified(&core, frame(None), UnclassifiedReason::Absent).await;
    assert_accept_unclassified(&core, frame(None), UnclassifiedReason::Absent).await;
}

// ── round-trip: the -32009 NACK interprets back into ConsentUnclassifiedAtPeer ─

#[tokio::test]
async fn nack_round_trips_to_consent_unclassified_at_peer() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;
    let resp = core
        .handle_intake(A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(None), 1))
        .await;
    // Feed the receiver's NACK back through interpret_response (the sender path).
    match core.interpret_response(&HostId("loopback".into()), resp) {
        Err(A2AError::ConsentUnclassifiedAtPeer { peer, reason }) => {
            assert_eq!(peer, "loopback");
            assert_eq!(reason, UnclassifiedReason::Absent);
        }
        other => panic!("expected ConsentUnclassifiedAtPeer, got {other:?}"),
    }
}

// ── (h) map_a2a_error_to_iac_bus: ConsentUnclassified / ConsentUnclassifiedAtPeer
// both map to CrossHostRouteFailure ─────────────────────────────────────────────

#[test]
fn map_a2a_error_to_iac_bus_consent_unclassified() {
    // Send-side unclassified → CrossHostRouteFailure.
    let err = A2AError::ConsentUnclassified {
        direction: IntentDirection::Send,
        reason: UnclassifiedReason::Absent,
    };
    let bus = map_a2a_error_to_iac_bus(err, "peer-a");
    match bus {
        IacBusError::CrossHostRouteFailure(msg) => {
            assert!(
                msg.contains("absent"),
                "message must carry the reason: {msg}"
            );
            assert!(
                msg.contains("Send"),
                "message must carry the direction: {msg}"
            );
            assert!(msg.contains("peer-a"), "message must carry the peer: {msg}");
        }
        other => panic!("expected CrossHostRouteFailure, got {other:?}"),
    }

    // Receiver-side mirror: ConsentUnclassifiedAtPeer.
    let err = A2AError::ConsentUnclassifiedAtPeer {
        peer: "test".to_string(),
        reason: UnclassifiedReason::NonCanonical,
    };
    let bus = map_a2a_error_to_iac_bus(err, "caller");
    match bus {
        IacBusError::CrossHostRouteFailure(msg) => {
            assert!(
                msg.contains("non_canonical"),
                "message must carry the reason: {msg}"
            );
            assert!(
                msg.contains("test"),
                "message must carry the denied peer: {msg}"
            );
        }
        other => panic!("expected CrossHostRouteFailure, got {other:?}"),
    }
}

// ── (i) interpret_response malformed NACK falls back to Absent ────────────────

#[tokio::test]
async fn interpret_response_malformed_nack_fallback() {
    let core = fail_closed_core(allow(&[FINE], &[FINE])).await;

    // Case 1: data is entirely absent.
    let resp = A2AJsonRpcResponse::nack(2, CODE_CONSENT_UNCLASSIFIED, "unclassified");
    match core.interpret_response(&HostId("loopback".into()), resp) {
        Err(A2AError::ConsentUnclassifiedAtPeer { peer, reason }) => {
            assert_eq!(peer, "loopback");
            assert_eq!(reason, UnclassifiedReason::Absent);
        }
        other => panic!("expected ConsentUnclassifiedAtPeer with Absent, got {other:?}"),
    }

    // Case 2: data is present but reason is a non-deserializable value (malformed).
    let resp = A2AJsonRpcResponse::nack_with_data(
        3,
        CODE_CONSENT_UNCLASSIFIED,
        "unclassified",
        serde_json::json!({ "reason": 999 }),
    );
    match core.interpret_response(&HostId("loopback".into()), resp) {
        Err(A2AError::ConsentUnclassifiedAtPeer { reason, .. }) => {
            assert_eq!(reason, UnclassifiedReason::Absent);
        }
        other => panic!("expected ConsentUnclassifiedAtPeer with Absent fallback, got {other:?}"),
    }

    // Case 3: data is present but reason key is missing entirely.
    let resp = A2AJsonRpcResponse::nack_with_data(
        4,
        CODE_CONSENT_UNCLASSIFIED,
        "unclassified",
        serde_json::json!({ "peer": "other" }),
    );
    match core.interpret_response(&HostId("loopback".into()), resp) {
        Err(A2AError::ConsentUnclassifiedAtPeer { reason, .. }) => {
            assert_eq!(reason, UnclassifiedReason::Absent);
        }
        other => panic!("expected ConsentUnclassifiedAtPeer with Absent fallback, got {other:?}"),
    }
}

// ── (j) exactly 128 bytes is classified; 129 is Oversized ────────────────────

#[tokio::test]
async fn exactly_128_bytes_is_classified() {
    use maos_domain::invariants::i8::MAX_CANONICAL_INTENT_LEN;

    // 128 bytes of all lowercase-alphanumeric passes is_canonical.
    let intent_128 = A2AIntent::new("a".repeat(128));
    assert_eq!(intent_128.as_str().len(), MAX_CANONICAL_INTENT_LEN);
    assert!(
        intent_128.is_canonical(),
        "128-byte all-lowercase intent must be canonical"
    );

    // 129 bytes is Oversized (fails is_canonical due to length guard).
    let intent_129 = A2AIntent::new("a".repeat(129));
    assert_eq!(intent_129.as_str().len(), MAX_CANONICAL_INTENT_LEN + 1);
    assert!(
        !intent_129.is_canonical(),
        "129-byte intent must fail is_canonical"
    );

    // End-to-end: 128-byte intent passes the fail-closed classification gate
    // (is classified, not unclassified), even though it may not be allowlisted.
    let core = fail_closed_core(ConsentAllowlists::default()).await;

    let f_128 = frame(Some(intent_128.as_str()));
    // Send side: should NOT produce ConsentUnclassified — it is classified.
    match core
        .prepare_outbound(f_128, &HostId("loopback".into()), 0)
        .await
    {
        Err(A2AError::ConsentUnclassified { .. }) => {
            panic!("128-byte canonical intent must be classified, not unclassified")
        }
        Err(A2AError::IntentDenied { .. }) => {
            // Expected: classified but not allowlisted → -32001 IntentDenied.
        }
        Ok(_) => {}
        other => panic!("unexpected result: {other:?}"),
    }

    // 129-byte intent should be denied with Oversized.
    let f_129 = frame(Some(intent_129.as_str()));
    assert_send_unclassified(&core, f_129, UnclassifiedReason::Oversized).await;
    let f_129_accept = frame(Some(intent_129.as_str()));
    assert_accept_unclassified(&core, f_129_accept, UnclassifiedReason::Oversized).await;
}

// ── (k) Deserializer does NOT collapse missing/null into empty string ──────
// Story 8.8 review patch — red-team concern: if serde turns a missing
// `intent_class` into `""`, `Absent` and `NonCanonical` would collapse.
// `ConsentEnvelope` uses `Option<A2AIntent>` + `#[serde(default)]`, so
// missing → `None` (Absent) and `""` → `Some(A2AIntent(""))` (NonCanonical).
// These MUST remain distinct.
#[test]
fn deserializer_missing_vs_empty_vs_null() {
    use maos_domain::frame::ConsentEnvelope;
    // Missing field → None
    let json_missing = r#"{"consent_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"granter":{"host_id":null,"spirit_id":"s"},"timestamp_ns":0}"#;
    let env: ConsentEnvelope = serde_json::from_str(json_missing).expect("missing field parses");
    assert!(
        env.intent_class.is_none(),
        "missing intent_class must be None, not Some(\"\")"
    );
    // Explicit null → None
    let json_null = r#"{"consent_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"granter":{"host_id":null,"spirit_id":"s"},"timestamp_ns":0,"intent_class":null}"#;
    let env: ConsentEnvelope = serde_json::from_str(json_null).expect("null parses");
    assert!(env.intent_class.is_none(), "null intent_class must be None");
    // Empty string → Some("") (which is NonCanonical, NOT Absent)
    let json_empty = r#"{"consent_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"granter":{"host_id":null,"spirit_id":"s"},"timestamp_ns":0,"intent_class":""}"#;
    let env: ConsentEnvelope = serde_json::from_str(json_empty).expect("empty string parses");
    assert_eq!(
        env.intent_class.as_ref().map(|i| i.as_str()),
        Some(""),
        "empty-string intent_class must be Some(\"\"), not None"
    );
    // Valid string → Some("valid")
    let json_valid = r#"{"consent_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"granter":{"host_id":null,"spirit_id":"s"},"timestamp_ns":0,"intent_class":"rca-summary"}"#;
    let env: ConsentEnvelope = serde_json::from_str(json_valid).expect("valid parses");
    assert_eq!(
        env.intent_class.as_ref().map(|i| i.as_str()),
        Some("rca-summary")
    );
}
