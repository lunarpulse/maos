//! Story 8.9 — A2A trust-binding & consent integrity hardening, live-wire
//! oracles over the real TCP/mTLS transport.
//!
//!   * AC1 (G8) — `g8_confused_deputy_negative`: a frame whose `from.host_id` is
//!     forged (≠ the TLS-verified peer) over a validly-pinned connection is
//!     rejected `CODE_PEER_IDENTITY_MISMATCH` and `intake_entered() == 0`; an
//!     honest frame on the SAME connection still ACKs (positive control).
//!   * AC3 (G10) — `g10_real_frame_consent_expiry`: a frame built via the REAL
//!     `prepare_outbound` (NOT a hand-built envelope) carries a synthesized
//!     expiry; a receiver whose pinned consent clock is advanced past it rejects
//!     `CODE_CONSENT_EXPIRED`.
//!   * AC6.2 (G5a) — `g5a_duplicate_peer_id_hard_fails`: `try_new` returns
//!     `ConfigInvalid` on a duplicate `peer_id` (no silent "last wins").
//!   * AC6.4 (G6) — `g6_intake_processing_and_write_bounded`: processing + the
//!     NACK write are inside the intake/idle bound; a stall after a served frame
//!     still aborts the task (gauge → 0), bounded `< 2s`.
//!
//! Reuses the hermetic H1–H6 harness (`support`) + the raw-dial pattern from
//! `t7_t10_liveness.rs`.

mod support;

use futures_util::{SinkExt, StreamExt};
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::transport::json_rpc::{
    CODE_CONSENT_GRANTER_MISMATCH, CODE_CONSENT_UNCLASSIFIED, CODE_INTENT_DENIED,
    CODE_INVALID_REQUEST, CODE_PEER_IDENTITY_MISMATCH,
};
use maos_a2a_core::{
    A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, A2ARouterCore, InMemoryTofuPinStore,
    TofuPinStore,
};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::frame::{ConsentEnvelope, FrameAddress};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{HostId, SpiritId};
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::*;
use tokio_util::bytes::Bytes;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;
const FINE: &str = "diagnosis-handoff:read-only-evidence";

/// Bind a Nash (host_b) endpoint pinning Mira(host_a) and accepting `accept`.
async fn bind_nash(
    clock: &Clock,
    ca: &Ca,
    mira: &Leaf,
    nash_leaf: &Leaf,
    accept: &[&str],
) -> maos_a2a_tcp::TcpA2ATransport {
    bind_endpoint(
        nash_leaf,
        Some(ca),
        NASH_NONCE,
        vec![pin("host_a", &mira.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira.fingerprint,
            &[],
            accept,
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await
}

/// Send one JSON-RPC request over a raw authenticated framed stream and await the
/// decoded response.
async fn send_recv(
    framed: &mut tokio_util::codec::Framed<
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        tokio_util::codec::LengthDelimitedCodec,
    >,
    req: &A2AJsonRpcRequest,
) -> A2AJsonRpcResponse {
    let body = serde_json::to_vec(req).expect("serialize request");
    framed.send(Bytes::from(body)).await.expect("send frame");
    let buf = framed
        .next()
        .await
        .expect("a response frame")
        .expect("response not a codec error");
    serde_json::from_slice(&buf).expect("decode response")
}

/// AC1 (G8) — confused-deputy negative: forged `from.host_id` over a valid pin.
#[tokio::test]
async fn g8_confused_deputy_negative() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &["readonly"]).await;
    let addr = nash.local_addr().unwrap();

    // Authenticated as Mira (TLS-verified peer = host_a).
    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // (1) FORGED: a well-formed frame claiming `from.host_id = host_b` (≠ the
    // verified peer host_a). Must be rejected at the binding, BEFORE intake.
    let forged = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_b", "host_b", IntentClass::Readonly, 1),
        1,
    )
    .with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &forged).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_PEER_IDENTITY_MISMATCH,
                "G8: forged from.host_id must be rejected as PeerIdentityMismatch"
            );
            let data = n
                .error
                .data
                .expect("mismatch NACK carries expected/asserted");
            assert_eq!(
                data.get("expected").and_then(|v| v.as_str()),
                Some("host_a")
            );
            assert_eq!(
                data.get("asserted").and_then(|v| v.as_str()),
                Some("host_b")
            );
        }
        other => panic!("G8: expected PeerIdentityMismatch NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        0,
        "G8: a forged frame must NOT increment intake_entered (binding precedes intake)"
    );

    // (2) HONEST positive control: same connection, `from.host_id = host_a`.
    let honest = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_a", "host_b", IntentClass::Readonly, 2),
        2,
    )
    .with_boot_nonce(MIRA_NONCE);
    assert!(
        matches!(
            send_recv(&mut framed, &honest).await,
            A2AJsonRpcResponse::Ack(_)
        ),
        "G8: an honest frame on the same connection must still ACK"
    );
    assert_eq!(
        nash.intake_entered(),
        1,
        "G8: only the honest frame entered intake"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "bounded < 2s (H5)"
    );
}

/// AC1.3 (G3) — a frame with ABSENT `from.host_id` mismatches the verified peer
/// and is rejected on the wire (the shared `None → loopback` fallback is
/// unreachable here).
#[tokio::test]
async fn g3_absent_from_host_id_rejected_on_wire() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &["readonly"]).await;
    let addr = nash.local_addr().unwrap();
    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    frame.from.host_id = None; // absent
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_PEER_IDENTITY_MISMATCH,
            "G3: absent from.host_id must mismatch the verified peer (no loopback fallback on the wire)"
        ),
        other => panic!("G3: expected PeerIdentityMismatch NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        0,
        "G3: absent-host frame never entered intake"
    );
}

/// AC3 (G10) — consent expiry fires on a REAL frame (built by `prepare_outbound`,
/// not a hand-built envelope).
#[tokio::test]
async fn g10_real_frame_consent_expiry() {
    const T0: u64 = 1_700_000_000_000_000_000;
    const TTL_NS: u64 = 300 * 1_000_000_000; // default consent_ttl_secs = 300

    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    // Nash's consent clock pinned PAST the synthesized expiry → expiry fires.
    let nash = bind_endpoint_consent_pinned(BindEndpointConfig {
        own_leaf: &nash_leaf,
        ca: Some(&ca),
        own_boot_nonce: NASH_NONCE,
        peer_pins: vec![pin("host_a", &mira.fingerprint, MIRA_NONCE)],
        peer_configs: vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira.fingerprint,
            &[],
            &[FINE],
        )],
        clock: &clock,
        timeouts: TcpTimeouts::test_profile(),
        retry: no_retry(),
        consent_now_ns: T0 + TTL_NS + 1,
    })
    .await;
    let nash_addr = nash.local_addr().unwrap();

    // Mira's consent clock pinned at T0 → `prepare_outbound` stamps
    // valid_until = T0 + TTL_NS on the (otherwise unbounded) envelope.
    let mira = bind_endpoint_consent_pinned(BindEndpointConfig {
        own_leaf: &mira,
        ca: Some(&ca),
        own_boot_nonce: MIRA_NONCE,
        peer_pins: vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        peer_configs: vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &[FINE],
            &[],
        )],
        clock: &clock,
        timeouts: TcpTimeouts::test_profile(),
        retry: no_retry(),
        consent_now_ns: T0,
    })
    .await;

    // A REAL frame: an envelope with NO explicit expiry (the dead-code gap).
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    frame.consent_envelope = Some(ConsentEnvelope::with_fine_grained_intent(
        frame.from.clone(),
        A2AIntent::new(FINE),
    ));

    let err = mira
        .route_outbound(frame, &HostId("host_b".into()))
        .await
        .expect_err("G10: a real frame past its synthesized expiry must be rejected");
    assert!(
        matches!(err, A2AError::ConsentExpired { .. }),
        "G10: must classify as ConsentExpired, got {err}"
    );
}

/// AC3 companion — an envelope that already carries an explicit `valid_until_ns`
/// (an authoritative grant) is NOT overridden by `prepare_outbound`.
#[tokio::test]
async fn ac3_explicit_valid_until_survives_prepare_outbound() {
    const T0: u64 = 1_700_000_000_000_000_000;
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let mira_t = bind_endpoint_consent_pinned(BindEndpointConfig {
        own_leaf: &mira,
        ca: Some(&ca),
        own_boot_nonce: MIRA_NONCE,
        peer_pins: vec![pin("host_b", &mira.fingerprint, NASH_NONCE)],
        peer_configs: vec![peer_cfg(
            "host_b",
            "tls://127.0.0.1:9",
            &mira.fingerprint,
            &[FINE],
            &[],
        )],
        clock: &clock,
        timeouts: TcpTimeouts::test_profile(),
        retry: no_retry(),
        consent_now_ns: T0,
    })
    .await;

    let core = mira_t.core();
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    let mut env =
        ConsentEnvelope::with_fine_grained_intent(frame.from.clone(), A2AIntent::new(FINE));
    let explicit = 42u64; // an authoritative grant's expiry, far in the past of T0
    env.valid_until_ns = Some(explicit);
    frame.consent_envelope = Some(env);

    let (req, _cfg, _id) = core
        .prepare_outbound(frame, &HostId("host_b".into()), MIRA_NONCE)
        .await
        .expect("prepare_outbound");
    let got = req
        .params
        .consent_envelope
        .and_then(|e| e.valid_until_ns)
        .expect("envelope retains an expiry");
    assert_eq!(
        got, explicit,
        "AC3: an explicit valid_until_ns (authoritative grant) must survive prepare_outbound unchanged"
    );
}

/// AC6.2 (G5a) — a duplicate `peer_id` is a HARD error from `try_new`.
#[test]
fn g5a_duplicate_peer_id_hard_fails() {
    let fp = maos_a2a_core::identity::PeerCertFingerprint::from_cert_der(b"x");
    let c1 = peer_cfg("dup", "tls://127.0.0.1:0", &fp, &[], &[]);
    let c2 = peer_cfg("dup", "tls://127.0.0.1:0", &fp, &[], &[]);
    let tofu = Arc::new(InMemoryTofuPinStore::new()) as Arc<dyn TofuPinStore>;
    // `A2ARouterCore` is not `Debug`, so match the `Result` directly rather than
    // `expect_err` (which would require `Debug` on the `Ok` variant).
    assert!(
        matches!(
            A2ARouterCore::try_new(vec![c1, c2], tofu),
            Err(A2AError::ConfigInvalid(_))
        ),
        "G5a: duplicate peer_id must hard-fail with ConfigInvalid (no silent last-wins)"
    );

    // And the non-duplicate case still succeeds.
    let ok1 = peer_cfg("a", "tls://127.0.0.1:0", &fp, &[], &[]);
    let ok2 = peer_cfg("b", "tls://127.0.0.1:0", &fp, &[], &[]);
    let tofu2 = Arc::new(InMemoryTofuPinStore::new()) as Arc<dyn TofuPinStore>;
    assert!(A2ARouterCore::try_new(vec![ok1, ok2], tofu2).is_ok());
}

/// AC6.4 (G6) — processing + the response WRITE complete within the idle bound,
/// and a subsequent stall on the same connection still aborts the per-connection
/// task (gauge → 0), bounded `< 2s`.
#[tokio::test]
async fn g6_intake_processing_and_write_bounded() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &["readonly"]).await;
    let addr = nash.local_addr().unwrap();
    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // A valid honest frame: processing + the ACK write happen INSIDE the idle
    // bound (G6). Receiving the ACK proves both completed.
    let honest = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_a", "host_b", IntentClass::Readonly, 1),
        1,
    )
    .with_boot_nonce(MIRA_NONCE);
    assert!(
        matches!(
            send_recv(&mut framed, &honest).await,
            A2AJsonRpcResponse::Ack(_)
        ),
        "G6: a valid frame is processed AND its ACK written within the bound"
    );

    // Now stall (send nothing further, hold the connection). The next read times
    // out → the per-connection task aborts → the gauge returns to 0.
    assert!(
        wait_until(
            || nash.active_connections() == 0,
            Duration::from_millis(1500)
        )
        .await,
        "G6: a stall after a served frame must still abort the task (gauge → 0)"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "G6: bounded < 2s (H5)"
    );
    drop(framed);
}
/// AC1 companion — a framing-invalid payload (wrong `jsonrpc` version) over a
/// valid TLS pin yields `CODE_INVALID_REQUEST` (NOT `CODE_PEER_IDENTITY_MISMATCH`).
/// Demonstrates that framing validation occurs AFTER the TLS-identity binding
/// in `handle_intake_verified` (identity passes → `handle_intake` → `validate()`).
#[tokio::test]
async fn g8_malformed_request_framing_nack() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &["readonly"]).await;
    let addr = nash.local_addr().unwrap();

    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // Build a valid-looking request (from.host_id = host_a, matching TLS peer)
    // but with a wrong `jsonrpc` version. This passes `try_from_bytes`
    // (structurally valid JSON) and the identity check in
    // `handle_intake_verified` (from.host_id matches TLS peer), then
    // `handle_intake::validate()` rejects it as `CODE_INVALID_REQUEST`.
    let honest_frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    let req = A2AJsonRpcRequest::new("iac.deliver", honest_frame, 1).with_boot_nonce(MIRA_NONCE);
    let mut raw = serde_json::to_value(&req).expect("serialize to value");
    raw["jsonrpc"] = serde_json::json!("1.0"); // wrong version
    let payload = serde_json::to_vec(&raw).expect("re-serialize modified payload");

    framed
        .send(Bytes::from(payload))
        .await
        .expect("send malformed frame");
    let buf = framed
        .next()
        .await
        .expect("a response frame")
        .expect("response not a codec error");
    let resp: A2AJsonRpcResponse = serde_json::from_slice(&buf).expect("decode response");

    match resp {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_INVALID_REQUEST,
                "framing-invalid jsonrpc must yield CODE_INVALID_REQUEST"
            );
            // Crucially NOT the identity-mismatch code — the peer identity was fine.
            assert_ne!(
                n.error.code, CODE_PEER_IDENTITY_MISMATCH,
                "framing errors must not be confused with identity errors"
            );
        }
        other => panic!("expected InvalidRequest NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        1,
        "malformed frame passed identity binding, so it entered intake"
    );
}

/// AC2 (G1) — stolen-envelope granter mismatch on the live wire: a frame whose
/// `consent_envelope.granter` does not match `frame.from` is rejected with
/// `CODE_CONSENT_GRANTER_MISMATCH` by the receiver.
#[tokio::test]
async fn g1_stolen_envelope_granter_mismatch_on_wire() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &[FINE]).await;
    let addr = nash.local_addr().unwrap();
    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // Build a valid frame from host_a → host_b with a consent envelope whose
    // granter is FORGED (points to host_b, not host_a). The frame's `from`
    // is honest (host_a = TLS-verified peer), so identity binding passes,
    // but the granter gate in `handle_intake` fires before the allowlist.
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    let forged_granter = FrameAddress {
        spirit_id: SpiritId::from("nash"),
        host_id: Some(HostId("host_b".into())),
        role: None,
    };
    frame.consent_envelope = Some(ConsentEnvelope::with_fine_grained_intent(
        forged_granter,
        A2AIntent::new(FINE),
    ));

    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_CONSENT_GRANTER_MISMATCH,
                "G1: forged granter must be rejected as ConsentGranterMismatch"
            );
            let data = n.error.data.expect("granter-mismatch NACK carries data");
            assert_eq!(
                data.get("granter").and_then(|v| v.as_str()),
                Some("nash@host_b")
            );
            assert_eq!(
                data.get("frame_from").and_then(|v| v.as_str()),
                Some("mira@host_a")
            );
        }
        other => panic!("G1: expected ConsentGranterMismatch NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        1,
        "G1: granter-mismatch frame passed identity binding, so it entered intake"
    );
}

/// Story 8.8 / AC1 (G7) — an UNCLASSIFIED frame (no consent envelope) sent over
/// the REAL wire to a fail-closed receiver gets `CODE_CONSENT_UNCLASSIFIED`
/// (-32009) and is NOT delivered (the intake sink stays empty), even though its
/// `from.host_id` is honest. Proves the fail-closed flip holds on the live
/// transport (which is constructed fail-closed by default), not just in-process.
#[tokio::test]
async fn g7_unclassified_frame_denied_on_wire() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    // Nash accepts the fine-grained intent — proving the deny is the UNCLASSIFIED
    // gate, not an allowlist miss (which would be -32001).
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &[FINE]).await;
    let addr = nash.local_addr().unwrap();

    // Observe deliveries: install a sink on the shared core; it must stay empty.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    nash.core().install_intake_sink(tx).await;

    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // Honest from.host_id (host_a == TLS-verified peer) but NO consent envelope.
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    frame.consent_envelope = None;
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_CONSENT_UNCLASSIFIED,
            "G7: an unclassified frame on the wire must be DENIED -32009 under fail-closed"
        ),
        other => panic!("G7: expected ConsentUnclassified NACK, got {other:?}"),
    }
    // Identity binding passed (honest host_a), so it entered intake...
    assert_eq!(
        nash.intake_entered(),
        1,
        "G7: honest-identity frame entered intake before the consent gate"
    );
    // ...but the fail-closed gate denied it before delivery: the sink is empty.
    assert!(
        rx.try_recv().is_err(),
        "G7: an unclassified frame must NOT be delivered to the intake sink"
    );
}

/// AC1 (G8) — envelope present but `intent_class` absent (None) is treated as
/// unclassified on the wire. The receiver must return `CODE_CONSENT_UNCLASSIFIED`
/// (-32009), proving the gate inspects `intent_class` presence, not just
/// `consent_envelope` presence.
#[tokio::test]
async fn g8_present_but_empty_envelope_denied_on_wire() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &[FINE]).await;
    let addr = nash.local_addr().unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    nash.core().install_intake_sink(tx).await;

    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // Honest from.host_id, consent_envelope present, but intent_class is None.
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    if let Some(env) = frame.consent_envelope.as_mut() {
        env.intent_class = None;
    }
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_CONSENT_UNCLASSIFIED,
            "G8: envelope present but intent_class absent must be denied -32009"
        ),
        other => panic!("G8: expected ConsentUnclassified NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        1,
        "G8: honest-identity frame entered intake before the consent gate"
    );
    assert!(
        rx.try_recv().is_err(),
        "G8: unclassified envelope must NOT be delivered to the intake sink"
    );
}

/// AC1 (G9) — a classified frame whose `intent_class` is NOT in the peer's
/// allowlist gets `-32001` (`CODE_INTENT_DENIED`), NOT `-32009`
/// (`CODE_CONSENT_UNCLASSIFIED`). Proves non-conflation: the receiver distinguishes
/// "classified-but-not-allowlisted" from "unclassified".
#[tokio::test]
async fn g9_classified_not_allowlisted_returns_32001_on_wire() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    // Nash only accepts the fine-grained FINE intent — a valid coarse-band
    // intent that is NOT in the allowlist triggers -32001.
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &[FINE]).await;
    let addr = nash.local_addr().unwrap();

    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // Build a frame with a valid but non-allowlisted intent_class.
    // Standard is a canonical 3-band intent that does NOT match the FINE entry.
    let frame = make_frame("host_a", "host_b", IntentClass::Standard, 1);
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_INTENT_DENIED,
                "G9: classified-but-not-allowlisted must be -32001, not -32009"
            );
            // Crucially NOT the unclassified code — proving non-conflation.
            assert_ne!(
                n.error.code, CODE_CONSENT_UNCLASSIFIED,
                "G9: -32001 must not be conflated with -32009"
            );
        }
        other => panic!("G9: expected IntentDenied NACK (-32001), got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        1,
        "G9: honest-identity frame entered intake before the allowlist gate"
    );
}

/// AC1 (G10) — explicit test that `handle_intake_verified` delegates
/// unclassified frames to `handle_intake`, which returns -32009. Mirrors G7
/// but with a name that makes the delegation path explicit.
#[tokio::test]
async fn g10_handle_intake_verified_delegates_unclassified() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf, &[FINE]).await;
    let addr = nash.local_addr().unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    nash.core().install_intake_sink(tx).await;

    let mut framed =
        raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;

    // No consent envelope at all — the most basic unclassified case.
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    frame.consent_envelope = None;
    let req = A2AJsonRpcRequest::new("iac.deliver", frame, 1).with_boot_nonce(MIRA_NONCE);
    match send_recv(&mut framed, &req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_CONSENT_UNCLASSIFIED,
                "G10: handle_intake must deny unclassified with -32009"
            );
            // Verify the error carries the expected reason field.
            let data = n
                .error
                .data
                .as_ref()
                .expect("G10: unclassified NACK carries data");
            assert!(
                data.get("reason").is_some(),
                "G10: NACK data must include 'reason'"
            );
        }
        other => panic!("G10: expected ConsentUnclassified NACK, got {other:?}"),
    }
    assert_eq!(
        nash.intake_entered(),
        1,
        "G10: identity binding passed, frame entered intake"
    );
    assert!(
        rx.try_recv().is_err(),
        "G10: unclassified frame must NOT be delivered"
    );
}
