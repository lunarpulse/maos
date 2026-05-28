//! Story 6.3 §A1 (Epic 6 retro 2026-05-28) — security regression guards for
//! P1 / P2 / P5 / P6 / P7 closure.
//!
//! These tests pin the behavior the retro called out as "structurally false
//! at HEAD". They exercise the `handle_intake` validation order end-to-end
//! AND exercise the `try_from_bytes` parse-error wrapper. If any test
//! regresses, NFR-Sec-12 (TOFU pin-mismatch 100% detect), NFR-Rel-6 (Spirit
//! restart detection), or ADR-012 (typed-intent consent allowlists) loses
//! its production-path enforcement.
//!
//! Test coverage map (retro §A1 P-numbers):
//!   * P1 — TOFU pin verify IS invoked in `handle_intake` → NACK on mismatch
//!   * P2 — Consent envelope `valid_until_ns` IS validated → NACK on expiry
//!   * P5 — `lookup_peer` failure returns NACK (no fallback to first peer)
//!   * P6 — Wire-carried `boot_nonce` mismatch triggers
//!          `invalidate_for_restart` + `CODE_SPIRIT_RESTART_DETECTED` NACK
//!   * P7 — `A2AJsonRpcRequest::try_from_bytes` on malformed JSON emits a
//!          `CODE_PARSE_ERROR (-32700)` NACK (not a raw serde error)

use maos_a2a::{
    A2APeerConfig, A2APeerRouter, A2AProfile, ConsentAllowlists, InMemoryTofuPinStore,
    LoopbackA2ARouter, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_a2a::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_CONSENT_EXPIRED, CODE_INTERNAL,
    CODE_PARSE_ERROR, CODE_PIN_MISMATCH_NOT_PINNED, CODE_SPIRIT_RESTART_DETECTED,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences,
    TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;
use std::sync::Arc;

fn fp(seed: &str) -> PeerCertFingerprint {
    PeerCertFingerprint::from_cert_der(seed.as_bytes())
}

fn make_peer_cfg(peer_id: &str, allowlists: ConsentAllowlists) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(peer_id),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: fp(peer_id),
        profile: A2AProfile::Loopback,
        allowlists,
        partition_timeout_secs: 30,
    }
}

fn make_frame_with_host(host_id: Option<&str>) -> IacFrame {
    IacFrame {
        frame_id: [0u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("sender"),
            host_id: host_id.map(|s| HostId(s.to_string())),
            role: None,
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("receiver"),
            host_id: host_id.map(|s| HostId(s.to_string())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "g".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

// ---- P1 — TOFU pin verify IS invoked in handle_intake ---------------------

#[tokio::test]
async fn p1_handle_intake_emits_pin_mismatch_nack_when_tofu_unpinned() {
    let allow = ConsentAllowlists {
        send_allowlist: vec![A2AIntent::new("standard")],
        accept_allowlist: vec![A2AIntent::new("standard")],
    };
    let cfg = make_peer_cfg("loopback", allow);
    // Deliberately do NOT pin — verify_pinned will return EPinMismatch::NotPinned.
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    let router = LoopbackA2ARouter::new(vec![cfg], tofu);
    let frame = make_frame_with_host(Some("loopback"));
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
    let resp = router.handle_intake(req).await;
    match resp {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_PIN_MISMATCH_NOT_PINNED,
                "P1 regression: handle_intake admitted a frame from a peer with no TOFU pin — NFR-Sec-12 detection floor breached"
            );
        }
        _ => panic!("P1 regression: unpinned peer admitted (expected NACK)"),
    }
}

// ---- P2 — Consent envelope expiry is validated ----------------------------

#[tokio::test]
async fn p2_handle_intake_rejects_expired_consent_envelope() {
    let allow = ConsentAllowlists {
        send_allowlist: vec![A2AIntent::new("standard")],
        accept_allowlist: vec![A2AIntent::new("standard")],
    };
    let cfg = make_peer_cfg("loopback", allow);
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let router = LoopbackA2ARouter::new(vec![cfg], tofu);

    let mut frame = make_frame_with_host(Some("loopback"));
    // valid_until_ns = 0 is in the past relative to any monotonic_now_ns() > 0.
    frame.consent_envelope = Some(ConsentEnvelope {
        consent_id: [0u8; 16],
        granter: FrameAddress {
            spirit_id: SpiritId::from("granter"),
            host_id: Some(HostId("loopback".to_string())),
            role: None,
        },
        timestamp_ns: 0,
        intent_class: Some(A2AIntent::new("standard")),
        valid_until_ns: Some(0),
    });
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
    let resp = router.handle_intake(req).await;
    match resp {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_CONSENT_EXPIRED,
                "P2 regression: handle_intake silently admitted a frame with expired consent — ADR-012 broken"
            );
            // Verify the NACK carries the timestamp data for typed-error
            // reconstruction at the sender.
            let data = n.error.data.expect("expired NACK must carry timestamps");
            assert!(
                data.get("expired_at_ns").is_some(),
                "P2 regression: NACK data missing expired_at_ns"
            );
            assert!(
                data.get("now_ns").is_some(),
                "P2 regression: NACK data missing now_ns"
            );
        }
        _ => panic!("P2 regression: expired consent envelope silently admitted (expected NACK)"),
    }
}

// ---- P5 — lookup_peer failure returns NACK, no fallback -------------------

#[tokio::test]
async fn p5_handle_intake_fails_closed_on_unknown_host_id() {
    let allow = ConsentAllowlists {
        send_allowlist: vec![A2AIntent::new("standard")],
        accept_allowlist: vec![A2AIntent::new("standard")],
    };
    let cfg_a = make_peer_cfg("peer-a", allow.clone());
    let cfg_b = make_peer_cfg("peer-b", allow);
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    // Pin both — the test verifies routing fails BEFORE the TOFU check
    // even has a peer to look up against. The forged host_id has no config.
    tofu.pin_first_contact(
        &PeerId::new("peer-a"),
        &cfg_a.cert_fingerprint,
        &cfg_a.cert_fingerprint,
        1,
    )
    .await
    .expect("pin a");
    tofu.pin_first_contact(
        &PeerId::new("peer-b"),
        &cfg_b.cert_fingerprint,
        &cfg_b.cert_fingerprint,
        1,
    )
    .await
    .expect("pin b");
    let router = LoopbackA2ARouter::new(vec![cfg_a, cfg_b], tofu);
    // Frame from a host_id that is NOT in the peer config — must NOT fall
    // back to peer-a (the first configured peer).
    let frame = make_frame_with_host(Some("forged-host-id"));
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1);
    let resp = router.handle_intake(req).await;
    match resp {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_INTERNAL,
                "P5 regression: unknown host_id should emit CODE_INTERNAL (no fallback to first peer)"
            );
            assert!(
                n.error.message.contains("forged-host-id"),
                "P5 regression: NACK message must name the unknown host_id; got {:?}",
                n.error.message
            );
        }
        _ => panic!(
            "P5 regression: handle_intake admitted a frame with a forged host_id — security bypass detected"
        ),
    }
}

// ---- P6 — boot_nonce mismatch fires CODE_SPIRIT_RESTART_DETECTED ----------

#[tokio::test]
async fn p6_wire_carried_boot_nonce_mismatch_invalidates_pin_and_nacks() {
    let allow = ConsentAllowlists {
        send_allowlist: vec![A2AIntent::new("standard")],
        accept_allowlist: vec![A2AIntent::new("standard")],
    };
    let cfg = make_peer_cfg("loopback", allow);
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    // Pin the peer with boot_nonce = 1.
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let router = LoopbackA2ARouter::new(vec![cfg], Arc::clone(&tofu) as Arc<dyn TofuPinStore>);

    let frame = make_frame_with_host(Some("loopback"));
    // Send with boot_nonce = 2 — the Spirit has restarted since the pin.
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(2);
    let resp = router.handle_intake(req).await;
    match resp {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_SPIRIT_RESTART_DETECTED,
                "P6 regression: boot_nonce mismatch on the wire must emit CODE_SPIRIT_RESTART_DETECTED"
            );
            let data = n
                .error
                .data
                .expect("restart NACK must carry boot_nonce data");
            assert_eq!(
                data.get("prior_boot_nonce").and_then(|v| v.as_u64()),
                Some(1),
                "P6 regression: NACK data must name prior_boot_nonce"
            );
            assert_eq!(
                data.get("observed_boot_nonce").and_then(|v| v.as_u64()),
                Some(2),
                "P6 regression: NACK data must name observed_boot_nonce"
            );
        }
        _ => panic!("P6 regression: Spirit restart silently admitted (expected NACK)"),
    }

    // Pin must now be invalidated.
    let post = tofu.get_pin(&PeerId::new("loopback")).await;
    assert!(
        post.is_some_and(|p| p.invalidated.is_some()),
        "P6 regression: pin not invalidated after restart detection — NFR-Rel-6 floor breached"
    );
}

#[tokio::test]
async fn p6_zero_boot_nonce_is_unspecified_sentinel_and_admits() {
    // Backward-compat: v0.5-α loopback callers leave boot_nonce at its
    // default `0`. The receiver MUST treat that as "unspecified" (skip the
    // restart-detection check) so existing flows don't break.
    let allow = ConsentAllowlists {
        send_allowlist: vec![A2AIntent::new("standard")],
        accept_allowlist: vec![A2AIntent::new("standard")],
    };
    let cfg = make_peer_cfg("loopback", allow);
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        7, // arbitrary non-zero stored nonce
    )
    .await
    .expect("pin");
    let router = LoopbackA2ARouter::new(vec![cfg], tofu);
    let frame = make_frame_with_host(Some("loopback"));
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1); // boot_nonce = 0 by default
    let resp = router.handle_intake(req).await;
    assert!(
        matches!(resp, A2AJsonRpcResponse::Ack(_)),
        "P6 regression: boot_nonce=0 (unspecified) must admit for v0.5-α backward-compat"
    );
}

// ---- P7 — try_from_bytes emits CODE_PARSE_ERROR on malformed JSON ---------

#[test]
fn p7_try_from_bytes_emits_parse_error_nack_on_malformed_json() {
    let bad = b"not-json-at-all";
    let err = A2AJsonRpcRequest::try_from_bytes(bad).expect_err("malformed JSON must NACK");
    assert_eq!(
        err.error.code, CODE_PARSE_ERROR,
        "P7 regression: malformed JSON must emit CODE_PARSE_ERROR (-32700), not raw serde error"
    );
    assert_eq!(
        err.jsonrpc, "2.0",
        "P7 regression: parse-error NACK must carry jsonrpc=2.0 envelope"
    );
    assert_eq!(
        err.id, 0,
        "P7 regression: parse-error NACK id must be 0 (JSON-RPC 2.0 §5.1 null sentinel for unparseable id)"
    );
    assert!(
        err.error.message.contains("JSON parse error"),
        "P7 regression: NACK message must be human-readable; got {:?}",
        err.error.message
    );
}

#[test]
fn p7_try_from_bytes_round_trips_well_formed_request() {
    // Confirm the helper is not a one-way reject — well-formed bytes
    // produce a parsed `A2AJsonRpcRequest`.
    let frame = make_frame_with_host(Some("loopback"));
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 42).with_boot_nonce(7);
    let bytes = serde_json::to_vec(&req).expect("serialize");
    let parsed = A2AJsonRpcRequest::try_from_bytes(&bytes).expect("parse");
    assert_eq!(parsed.id, 42);
    assert_eq!(parsed.method, "iac.deliver");
    assert_eq!(parsed.boot_nonce, 7);
}
