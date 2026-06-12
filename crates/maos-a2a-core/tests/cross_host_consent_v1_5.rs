//! Story 8.7 — fine-grained typed-intent consent over `maos-a2a-core`.
//!
//! Mirrors (and makes *truthful*) the aspirational specific-intent assertion in
//! `maos-a2a/tests/cross_host_consent_v1.rs:75`: that suite holds
//! `diagnosis-handoff:read-only-evidence` in an allowlist but routes frames that
//! collapse to the 3-band projection, so the fine-grained string never actually
//! drives the decision. These scenarios route frames whose
//! `consent_envelope.intent_class` carries the real per-frame `A2AIntent`, so the
//! enforcement decision is the fine-grained key — ADR-012 "typed-*intent*
//! consent" rather than "typed-*class* consent".
//!
//! Enforcement lives at the `A2ARouterCore` decision points (post-8.6):
//!   - send side  → `prepare_outbound` (defense-in-depth, sender refuses first)
//!   - accept side→ `handle_intake`    (receiver NACK -32001)
//!
//! Coverage: AC1 (fine-grained match + band fallback), AC2 (round-trip
//! byte-equality), AC3 (confused-deputy negative at fine granularity), AC4
//! (defense-in-depth both directions), AC5 (unreachable-entry `warn!`), AC7
//! (band fallback preserved).

use maos_a2a_core::error::{A2AError, IntentDirection, UnclassifiedReason};
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_INTENT_DENIED, METHOD_IAC_DELIVER,
};
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, A2ARouterCore, ConsentAllowlists, InMemoryTofuPinStore,
    PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;
use std::sync::Arc;

const FINE_READONLY: &str = "diagnosis-handoff:read-only-evidence";
const FINE_MUTATION: &str = "code-mutation-directive";

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

async fn pinned_core(allowlists: ConsentAllowlists) -> A2ARouterCore {
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
    A2ARouterCore::new(vec![cfg], tofu)
}

/// `band_intent` is the coarse `IntentClass` (the band-fallback projection);
/// `fine` is the optional per-frame fine-grained `A2AIntent` that, when present,
/// supersedes the band at the consent decision point.
fn frame(band_intent: IntentClass, fine: Option<&str>) -> IacFrame {
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
        intent: band_intent,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "diagnose".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: fine
            .map(|s| ConsentEnvelope::with_fine_grained_intent(from, A2AIntent::new(s))),
        intent_lineage: IntentLineage::default(),
    }
}

// ── AC1 — fine-grained match on the send side ─────────────────────────────────

#[tokio::test]
async fn fine_grained_send_admitted() {
    // Frame projects to the `readonly` band, but its fine-grained intent is the
    // specific string; the send_allowlist holds ONLY the fine-grained string
    // (NOT the `readonly` band) → admitted because the fine-grained key matches.
    let core = pinned_core(allow(&[FINE_READONLY], &[])).await;
    let f = frame(IntentClass::Readonly, Some(FINE_READONLY));
    core.prepare_outbound(f, &HostId("loopback".into()), 0)
        .await
        .expect("fine-grained send admitted");
}

#[tokio::test]
async fn fine_grained_send_denied_reports_literal_string_not_band() {
    // send_allowlist holds only `readonly`-fine; the frame carries the mutation
    // intent → denied, and the reported `intent` is the LITERAL fine-grained
    // string, never the `readonly`/`standard` band token.
    let core = pinned_core(allow(&[FINE_READONLY], &[])).await;
    let f = frame(IntentClass::Readonly, Some(FINE_MUTATION));
    let err = core
        .prepare_outbound(f, &HostId("loopback".into()), 0)
        .await
        .expect_err("must deny at sender");
    match err {
        A2AError::IntentDenied {
            direction: IntentDirection::Send,
            inner,
        } => {
            assert_eq!(
                inner.intent, FINE_MUTATION,
                "EIntentDenied.intent must carry the fine-grained string, got {:?}",
                inner.intent
            );
            assert_ne!(inner.intent, "readonly", "must NOT report a band token");
        }
        other => panic!("expected send-side IntentDenied, got {other:?}"),
    }
}

// ── AC3 + AC4 — confused-deputy negative, accept side, defense-in-depth ────────

#[tokio::test]
async fn confused_deputy_closed_at_fine_granularity_on_accept() {
    // Nash accepts ONLY `diagnosis-handoff:read-only-evidence`. Both candidate
    // frames project to the SAME `readonly` band, so a band-only gate would admit
    // BOTH (reopening ADR-012's confused-deputy gap). The fine-grained gate
    // admits the read-only evidence and rejects the mutation directive.
    let core = pinned_core(allow(&[], &[FINE_READONLY])).await;

    // admitted
    let ok = A2AJsonRpcRequest::new(
        METHOD_IAC_DELIVER,
        frame(IntentClass::Readonly, Some(FINE_READONLY)),
        1,
    );
    assert!(
        matches!(core.handle_intake(ok).await, A2AJsonRpcResponse::Ack(_)),
        "fine-grained read-only evidence must be admitted at Nash"
    );

    // denied — confused-deputy directive, same band, fine-grained mismatch
    let deny = A2AJsonRpcRequest::new(
        METHOD_IAC_DELIVER,
        frame(IntentClass::Readonly, Some(FINE_MUTATION)),
        2,
    );
    match core.handle_intake(deny).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(n.error.code, CODE_INTENT_DENIED, "must NACK -32001");
            assert!(
                n.error.message.contains(FINE_MUTATION),
                "NACK must name the fine-grained directive, got: {}",
                n.error.message
            );
            assert!(
                !n.error.message.contains("readonly"),
                "NACK must NOT report the band token: {}",
                n.error.message
            );
        }
        other => panic!("expected -32001 NACK, got {other:?}"),
    }
}

#[tokio::test]
async fn defense_in_depth_independent_on_fine_grained_key() {
    // Send side admits the fine-grained intent; accept side does NOT → the frame
    // leaves the sender but is rejected at intake (IntentDeniedAtPeer surfaces via
    // the interpret_response path; here we assert the intake NACK directly).
    let core = pinned_core(allow(&[FINE_READONLY], &[FINE_MUTATION])).await;
    // send admitted
    let (req, _cfg, _id) = core
        .prepare_outbound(
            frame(IntentClass::Readonly, Some(FINE_READONLY)),
            &HostId("loopback".into()),
            0,
        )
        .await
        .expect("send admits fine-grained");
    // accept denied independently (accept_allowlist lacks the read-only intent)
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(n.error.code, CODE_INTENT_DENIED),
        other => panic!("expected accept-side NACK, got {other:?}"),
    }
}

// ── AC2 — round-trip byte-equality through the wire decode ─────────────────────

#[tokio::test]
async fn intent_class_round_trips_byte_equal_through_wire() {
    let core = pinned_core(allow(&[FINE_READONLY], &[FINE_READONLY])).await;
    let (req, _cfg, _id) = core
        .prepare_outbound(
            frame(IntentClass::Readonly, Some(FINE_READONLY)),
            &HostId("loopback".into()),
            0,
        )
        .await
        .expect("send admits");

    // Serialize onto the wire and decode via the real framing path.
    let bytes = serde_json::to_vec(&req).expect("serialize request");
    let decoded = A2AJsonRpcRequest::try_from_bytes(&bytes).expect("wire decode");
    let got = decoded
        .params
        .consent_envelope
        .as_ref()
        .and_then(|e| e.intent_class.as_ref())
        .expect("intent_class survives the wire");
    assert_eq!(
        got.as_str(),
        FINE_READONLY,
        "round-tripped intent_class must be byte-equal to what the sender set"
    );

    // And it is what the receiver actually consults: delivery succeeds.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    core.install_intake_sink(tx).await;
    assert!(matches!(
        core.handle_intake(decoded).await,
        A2AJsonRpcResponse::Ack(_)
    ));
    let delivered = rx.recv().await.expect("delivered");
    assert_eq!(
        delivered
            .consent_envelope
            .and_then(|e| e.intent_class)
            .map(|i| i.as_str().to_string()),
        Some(FINE_READONLY.to_string()),
    );
}

// ── Story 8.8 (Option 2) — NO band fallback: a band allowlist never matches a
// fine-grained frame, and an unclassified frame is denied (not band-projected) ──

#[tokio::test]
async fn band_allowlist_does_not_match_fine_grained_frame() {
    // A `standard` band-token allowlist does NOT match a fine-grained frame — the
    // fine-grained key is the only key (no band collapse). (The pre-8.8 companion
    // "band fallback admits a None frame" assertion is deleted: under Option 2 a
    // None/unclassified frame is denied fail-closed, covered by
    // `fail_closed_8_8::absent_envelope_denied_both_directions`.)
    let core = pinned_core(allow(&["standard"], &[])).await;
    let err = core
        .prepare_outbound(
            frame(IntentClass::Standard, Some(FINE_READONLY)),
            &HostId("loopback".into()),
            0,
        )
        .await
        .expect_err("fine-grained key supersedes the band");
    assert!(matches!(
        err,
        A2AError::IntentDenied {
            direction: IntentDirection::Send,
            ..
        }
    ));
}

// ── AC5 — unreachable-entry warning is loud + regression-pinned ───────────────

#[derive(Clone, Default)]
struct BufWriter(Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn warn_fires_on_unreachable_non_canonical_allowlist_entry() {
    let buf = Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(BufWriter(buf.clone()))
        .with_max_level(tracing::Level::WARN)
        .without_time()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // A non-canonical typo ENTRY in the allowlist that can never match any
            // frame's key. Story 8.8 (Option 2): a CLASSIFIED frame reaches the
            // allowlist (the gate admits classified); its fine-grained key is not in
            // the typo'd send_allowlist → miss → `warn_unreachable_entries` fires on
            // the non-canonical entry.
            let core = pinned_core(allow(&["Not A Canonical Intent"], &[])).await;
            let _ = core
                .prepare_outbound(
                    frame(IntentClass::Readonly, Some(FINE_READONLY)),
                    &HostId("loopback".into()),
                    0,
                )
                .await;
        });
    });

    let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("may not match canonical frame intents"),
        "expected an unreachable-entry warning, captured: {logged:?}"
    );
    assert!(
        logged.contains("Not A Canonical Intent"),
        "warning must name the offending entry, captured: {logged:?}"
    );
}

#[test]
fn warn_fires_on_accept_side_unreachable_entry() {
    let buf = Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(BufWriter(buf.clone()))
        .with_max_level(tracing::Level::WARN)
        .without_time()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // Story 8.8 (Option 2): a CLASSIFIED frame reaches the accept-allowlist
            // (the gate admits classified); its key is not in the typo'd
            // accept_allowlist → miss → `warn_unreachable_entries` fires.
            let core = pinned_core(allow(&[], &["Not A Canonical Intent"])).await;
            let req = A2AJsonRpcRequest::new(
                METHOD_IAC_DELIVER,
                frame(IntentClass::Readonly, Some(FINE_READONLY)),
                1,
            );
            let _ = core.handle_intake(req).await;
        });
    });

    let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("may not match canonical frame intents"),
        "expected accept-side unreachable-entry warning, captured: {logged:?}"
    );
}

#[tokio::test]
async fn case_insensitive_matching_for_fine_grained_intents() {
    // The allowlist holds mixed-case; the frame carries lowercase.
    // `eq_ignore_ascii_case` must admit the frame (review finding).
    let core = pinned_core(allow(&["Diagnosis-Handoff:Read-Only-Evidence"], &[])).await;
    let f = frame(
        IntentClass::Readonly,
        Some("diagnosis-handoff:read-only-evidence"),
    );
    core.prepare_outbound(f, &HostId("loopback".into()), 0)
        .await
        .expect("case-insensitive match must admit");
}

// ── Story 8.9 / AC2 (G1) — consent-envelope granter binding ───────────────────

use maos_a2a_core::transport::json_rpc::{CODE_CONSENT_EXPIRED, CODE_CONSENT_GRANTER_MISMATCH};

/// An honestly-built frame (granter == from), but with the granter overwritten to
/// a FOREIGN address — the stolen-envelope replay shape. `valid_until` stays
/// `None` so the expiry check passes and the GRANTER gate is what fires.
fn frame_with_foreign_granter() -> IacFrame {
    let mut f = frame(IntentClass::Readonly, Some(FINE_READONLY));
    if let Some(env) = f.consent_envelope.as_mut() {
        env.granter = FrameAddress {
            spirit_id: SpiritId::from("attacker"),
            host_id: Some(HostId("loopback".into())),
            role: None,
        };
    }
    f
}

#[tokio::test]
async fn ac2_granter_mismatch_denied_on_intake() {
    // Accept-allowlist HOLDS the intent — proving the granter gate (not the
    // allowlist) is what denies the replayed envelope.
    let core = pinned_core(allow(&[], &[FINE_READONLY])).await;
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame_with_foreign_granter(), 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(
                n.error.code, CODE_CONSENT_GRANTER_MISMATCH,
                "must NACK -32008"
            );
            let data = n
                .error
                .data
                .expect("granter-mismatch NACK carries both addresses");
            assert!(data.get("granter").is_some(), "NACK data names the granter");
            assert!(
                data.get("frame_from").is_some(),
                "NACK data names the frame from"
            );
        }
        other => panic!("expected granter-mismatch NACK, got {other:?}"),
    }
}

#[tokio::test]
async fn ac2_granter_match_admitted_on_intake() {
    // `with_fine_grained_intent(from, …)` sets granter == from → admitted.
    let core = pinned_core(allow(&[], &[FINE_READONLY])).await;
    let req = A2AJsonRpcRequest::new(
        METHOD_IAC_DELIVER,
        frame(IntentClass::Readonly, Some(FINE_READONLY)),
        1,
    );
    assert!(
        matches!(core.handle_intake(req).await, A2AJsonRpcResponse::Ack(_)),
        "granter == from must be admitted"
    );
}

// ── Story 8.9 / AC4 (G2) — consent evaluated BEFORE the accept-allowlist ──────

#[tokio::test]
async fn ac4_expiry_wins_over_accept_allowlist_denial() {
    // The accept_allowlist is EMPTY (the intent would be denied), AND the
    // envelope is expired. Because the consent block now runs BEFORE the
    // accept-allowlist, the EXPIRED code wins — proving the reorder.
    const T0: u64 = 1_700_000_000_000_000_000;
    let cfg = peer_cfg(allow(&[], &[]));
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let core = A2ARouterCore::new(vec![cfg], tofu).with_pinned_consent_clock(T0);

    let mut f = frame(IntentClass::Readonly, Some(FINE_READONLY)); // granter == from
    if let Some(env) = f.consent_envelope.as_mut() {
        env.valid_until_ns = Some(T0 - 1); // expired
    }
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, f, 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_CONSENT_EXPIRED,
            "expiry must be evaluated before the accept-allowlist (reorder)"
        ),
        other => panic!("expected CONSENT_EXPIRED, got {other:?}"),
    }
}

#[tokio::test]
async fn ac4_expired_but_allowlisted_still_rejected() {
    // The accept_allowlist HOLDS the intent, AND the envelope is expired with
    // granter == from. Because the consent block runs BEFORE the allowlist,
    // EXPIRED wins even though the intent would have been admitted.
    const T0: u64 = 1_700_000_000_000_000_000;
    let cfg = peer_cfg(allow(&[], &[FINE_READONLY]));
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let core = A2ARouterCore::new(vec![cfg], tofu).with_pinned_consent_clock(T0);

    let mut f = frame(IntentClass::Readonly, Some(FINE_READONLY)); // granter == from
    if let Some(env) = f.consent_envelope.as_mut() {
        env.valid_until_ns = Some(T0 - 1); // expired
    }
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, f, 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_CONSENT_EXPIRED,
            "expiry must win over allowlisted intent (consent-before-allowlist)"
        ),
        other => panic!("expected CONSENT_EXPIRED, got {other:?}"),
    }
}

#[tokio::test]
async fn ac4_granter_mismatch_with_allowlisted_intent() {
    // The accept_allowlist HOLDS the intent, AND the envelope has a wrong granter
    // (foreign spirit). Because granter is checked BEFORE allowlist, GRANTER_MISMATCH wins.
    let core = pinned_core(allow(&[], &[FINE_READONLY])).await;
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame_with_foreign_granter(), 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => assert_eq!(
            n.error.code, CODE_CONSENT_GRANTER_MISMATCH,
            "granter mismatch must win over allowlisted intent (consent-before-allowlist)"
        ),
        other => panic!("expected GRANTER_MISMATCH, got {other:?}"),
    }
}

#[tokio::test]
async fn ac2_granter_mismatch_host_id_only() {
    // spirit_id matches, ONLY host_id differs. The granter binding compares BOTH
    // fields, so a host_id mismatch alone fires GRANTER_MISMATCH.
    let mut f = frame(IntentClass::Readonly, Some(FINE_READONLY));
    if let Some(env) = f.consent_envelope.as_mut() {
        // granter.spirit_id == from.spirit_id ("mira"), but host_id is different
        env.granter = FrameAddress {
            spirit_id: f.from.spirit_id.clone(),
            host_id: Some(HostId("other-host".into())),
            role: None,
        };
    }
    let core = pinned_core(allow(&[], &[FINE_READONLY])).await;
    let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, f, 1);
    match core.handle_intake(req).await {
        A2AJsonRpcResponse::Nack(n) => {
            assert_eq!(n.error.code, CODE_CONSENT_GRANTER_MISMATCH);
            let data = n
                .error
                .data
                .expect("granter-mismatch NACK carries addresses");
            assert!(data.get("granter").is_some());
            assert!(data.get("frame_from").is_some());
        }
        other => panic!("expected GRANTER_MISMATCH, got {other:?}"),
    }
}
// ── Story 8.9 / AC5 (G9) — intent length bound unified to the canonical 128 ───

#[tokio::test]
async fn ac5_intent_length_bound_unified_to_canonical_128() {
    // 129-byte intent_class exceeds the canonical bound → UNCLASSIFIED. Story 8.8
    // (Option 2): under unconditional fail-closed it is DENIED (Oversized), NOT
    // band-projected. (Pre-8.8 this asserted band fallback; that path is gone.)
    let over = "a".repeat(129);
    let core = pinned_core(allow(&["readonly"], &[])).await;
    let err = core
        .prepare_outbound(
            frame(IntentClass::Readonly, Some(&over)),
            &HostId("loopback".into()),
            0,
        )
        .await
        .expect_err("129-byte intent_class is oversized → fail-closed deny");
    assert!(matches!(
        err,
        A2AError::ConsentUnclassified {
            direction: IntentDirection::Send,
            reason: UnclassifiedReason::Oversized,
        }
    ));

    // 128-byte intent_class is at the bound → used as the fine-grained key; a
    // band-only allowlist must NOT match it (fine-grained supersedes the band).
    let exact = "a".repeat(128);
    let core2 = pinned_core(allow(&["readonly"], &[])).await;
    let err = core2
        .prepare_outbound(
            frame(IntentClass::Readonly, Some(&exact)),
            &HostId("loopback".into()),
            0,
        )
        .await
        .expect_err(
            "128-byte intent_class is the fine-grained key (band allowlist must not match)",
        );
    assert!(matches!(
        err,
        A2AError::IntentDenied {
            direction: IntentDirection::Send,
            ..
        }
    ));
}
