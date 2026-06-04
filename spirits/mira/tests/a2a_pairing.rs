//! AC2 — Mira (Host A) + Nash (Host B) bilateral pair coordinate over the REAL
//! `LoopbackA2ARouter` with **pre-paired cert fingerprints** + **TOFU pinning**
//! (loopback-simulated two-Host; Decision B — the live CrossHost TCP/mTLS
//! transport is Story 8.6, explicitly out of scope here).
//!
//! The router holds a per-side `A2APeerConfig` for each `HostId` (`host_a` /
//! `host_b`), each carrying the other side's pre-paired `PeerCertFingerprint`
//! (no discovery) and ADR-012 `ConsentAllowlists`. The `LoopbackA2ARouter` keys
//! peers by `peer_id == HostId`; `route_outbound` enforces the destination's
//! `send_allowlist` and `handle_intake` enforces the source's `accept_allowlist`.
//!
//! Mira's advisory is read-only diagnostic evidence, so it crosses the boundary
//! as `IntentClass::Readonly` (consent projection `"readonly"` —
//! `mira::ADVISORY_CONSENT_INTENT`). The advisory itself rides the frame as a
//! JSON payload; Nash deserializes it (`Nash::from_wire`) into its own input
//! type — the genuine cross-Host contract is the serde shape, not a crate
//! dependency.

use maos_a2a::error::{A2AError, IntentDirection};
use maos_a2a::{
    A2APeerConfig, A2APeerRouter as LocalRouter, A2AProfile, ConsentAllowlists, EPinMismatch,
    InMemoryTofuPinStore, LoopbackA2ARouter, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_domain::frame::{
    FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use mira::{AnomalySignal, Mira, ADVISORY_CONSENT_INTENT};
use nash::Nash;
use smallvec::smallvec;
use std::sync::Arc;

const SCENARIOS: &str = include_str!("fixtures/diagnostic-scenarios.json");

fn peer(peer_id: &str, fp: PeerCertFingerprint, send: &[&str], accept: &[&str]) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(peer_id),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: fp,
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: send.iter().map(|s| A2AIntent::new(*s)).collect(),
            accept_allowlist: accept.iter().map(|s| A2AIntent::new(*s)).collect(),
        },
        partition_timeout_secs: 30,
    }
}

/// A cross-Host advisory frame carrying `advisory_json` from `from_host` →
/// `to_host`. Spirit-authored frames carry `FrameOrigin::SpiritAuto` (the 8.4
/// review patch) and `SpiritRole::Worker` (Decision C).
fn advisory_frame(
    from_spirit: &str,
    from_host: &str,
    to_spirit: &str,
    to_host: &str,
    advisory_json: String,
    intent: IntentClass,
    seq: u64,
) -> IacFrame {
    let mut fid = [0u8; 16];
    fid[0..8].copy_from_slice(&seq.to_be_bytes());
    fid[8] = 0xA5; // test-namespace marker
    IacFrame {
        frame_id: fid,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(from_spirit),
            host_id: Some(HostId(from_host.to_string())),
            role: Some(SpiritRole::Worker),
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from(to_spirit),
            host_id: Some(HostId(to_host.to_string())),
            role: Some(SpiritRole::Worker),
        }],
        kind: FrameKind::TaskAssign,
        intent,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: advisory_json,
            scope: vec![],
            success_criteria: "architect a fix for the diagnosed anomaly".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

fn mira_advisory_json() -> String {
    let signals: Vec<AnomalySignal> =
        serde_json::from_str(SCENARIOS).expect("diagnostic scenarios parse");
    let mira = Mira::default();
    // The unknown-severe scenario — the one that reaches Mira's halt boundary.
    let diag = mira.diagnose(&signals[1]);
    serde_json::to_string(&mira.advisory(&diag)).expect("advisory serializes")
}

#[tokio::test]
async fn bilateral_pair_routes_advisory_over_real_loopback_with_tofu() {
    let host_a_fp = PeerCertFingerprint::from_cert_der(b"mira-host-a-cert-v1");
    let host_b_fp = PeerCertFingerprint::from_cert_der(b"nash-host-b-cert-v1");

    // Per-side configs — pre-paired fingerprints (no discovery), both allowlists
    // admit the `readonly` advisory intent.
    let cfg_a = peer(
        "host_a",
        host_a_fp.clone(),
        &[ADVISORY_CONSENT_INTENT],
        &[ADVISORY_CONSENT_INTENT],
    );
    let cfg_b = peer(
        "host_b",
        host_b_fp.clone(),
        &[ADVISORY_CONSENT_INTENT],
        &[ADVISORY_CONSENT_INTENT],
    );

    // Pre-pin both peers' fingerprints (TOFU first-contact, declared == observed).
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(&PeerId::new("host_a"), &host_a_fp, &host_a_fp, 1)
        .await
        .expect("pin host_a");
    tofu.pin_first_contact(&PeerId::new("host_b"), &host_b_fp, &host_b_fp, 1)
        .await
        .expect("pin host_b");

    let router = LoopbackA2ARouter::new(vec![cfg_a, cfg_b], tofu.clone());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    router.install_intake_sink(tx).await;

    let advisory_json = mira_advisory_json();

    // ── Mira(host_a) → Nash(host_b): advisory delivered, Nash architects ──
    let frame = advisory_frame(
        "mira",
        "host_a",
        "nash",
        "host_b",
        advisory_json.clone(),
        IntentClass::Readonly,
        1,
    );
    LocalRouter::route_outbound(&router, frame, &HostId("host_b".into()))
        .await
        .expect("Mira→Nash advisory delivered");
    let delivered = rx.recv().await.expect("Nash receives the advisory frame");
    assert_eq!(delivered.from.host_id, Some(HostId("host_a".into())));
    assert_eq!(delivered.to[0].host_id, Some(HostId("host_b".into())));
    assert_eq!(delivered.to[0].role, Some(SpiritRole::Worker));

    // Nash deserializes the advisory off the wire and produces an architecture.
    let goal = match &delivered.payload {
        FramePayload::TaskAssign(t) => t.goal.clone(),
        other => panic!("unexpected payload: {other:?}"),
    };
    let adv_in = Nash::from_wire(&goal).expect("advisory deserializes off the wire");
    let proposal = Nash::default().architect(&adv_in);
    assert_eq!(proposal.subject, "edge-cache");
    assert_eq!(
        proposal.source_log_ref, "tl:row:2002",
        "FR17 source_log_ref threads Mira→Nash across the A2A boundary"
    );
    assert!(proposal.proposed_fix.contains("circuit-breaker"));

    // ── Nash(host_b) → Mira(host_a): reverse direction also routes ──
    let reply = advisory_frame(
        "nash",
        "host_b",
        "mira",
        "host_a",
        serde_json::to_string(&proposal).unwrap(),
        IntentClass::Readonly,
        2,
    );
    LocalRouter::route_outbound(&router, reply, &HostId("host_a".into()))
        .await
        .expect("Nash→Mira reply delivered (bidirectional)");
    let reply_delivered = rx.recv().await.expect("Mira receives the reply");
    assert_eq!(reply_delivered.from.host_id, Some(HostId("host_b".into())));
    assert_eq!(reply_delivered.to[0].host_id, Some(HostId("host_a".into())));
}

#[tokio::test]
async fn tofu_verify_admits_matching_and_rejects_tampered_fingerprint() {
    let host_b_fp = PeerCertFingerprint::from_cert_der(b"nash-host-b-cert-v1");
    let tofu = InMemoryTofuPinStore::new();
    tofu.pin_first_contact(&PeerId::new("host_b"), &host_b_fp, &host_b_fp, 1)
        .await
        .expect("pin host_b");

    // The pre-paired fingerprint is admitted.
    tofu.verify_pinned(&PeerId::new("host_b"), &host_b_fp)
        .await
        .expect("matching pinned fingerprint admitted");

    // A tampered / rotated fingerprint is rejected with EPinMismatch::Mismatch.
    let tampered = PeerCertFingerprint::from_cert_der(b"attacker-rotated-cert");
    let err = tofu
        .verify_pinned(&PeerId::new("host_b"), &tampered)
        .await
        .expect_err("tampered fingerprint must be rejected");
    assert!(
        matches!(err, EPinMismatch::Mismatch { .. }),
        "expected EPinMismatch::Mismatch, got {err:?}"
    );
}

#[tokio::test]
async fn non_allowlisted_intent_denied_at_accepting_peer() {
    // AC4 negative — a frame whose intent is NOT in the accepting side's
    // accept_allowlist is rejected with a consent error (IntentDeniedAtPeer).
    let host_a_fp = PeerCertFingerprint::from_cert_der(b"mira-host-a-cert-v1");
    let host_b_fp = PeerCertFingerprint::from_cert_der(b"nash-host-b-cert-v1");

    // The source (host_a) config accepts NOTHING → the advisory is refused at
    // intake even though the destination admits the send.
    let cfg_a = peer("host_a", host_a_fp.clone(), &[ADVISORY_CONSENT_INTENT], &[]);
    let cfg_b = peer(
        "host_b",
        host_b_fp.clone(),
        &[ADVISORY_CONSENT_INTENT],
        &[ADVISORY_CONSENT_INTENT],
    );
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(&PeerId::new("host_a"), &host_a_fp, &host_a_fp, 1)
        .await
        .unwrap();
    tofu.pin_first_contact(&PeerId::new("host_b"), &host_b_fp, &host_b_fp, 1)
        .await
        .unwrap();
    let router = LoopbackA2ARouter::new(vec![cfg_a, cfg_b], tofu);

    let frame = advisory_frame(
        "mira",
        "host_a",
        "nash",
        "host_b",
        mira_advisory_json(),
        IntentClass::Readonly,
        3,
    );
    let err = LocalRouter::route_outbound(&router, frame, &HostId("host_b".into()))
        .await
        .expect_err("advisory must be refused at the accepting peer");
    assert!(
        matches!(err, A2AError::IntentDeniedAtPeer { .. }),
        "expected IntentDeniedAtPeer, got {err:?}"
    );
}

#[tokio::test]
async fn send_side_denial_carries_eintentdenied() {
    // The literal `EIntentDenied` rides the send-side denial: an intent not in
    // the destination's send_allowlist is rejected before the wire.
    let host_a_fp = PeerCertFingerprint::from_cert_der(b"mira-host-a-cert-v1");
    let host_b_fp = PeerCertFingerprint::from_cert_der(b"nash-host-b-cert-v1");
    // host_b admits only `readonly` on send; a `standard` frame is denied.
    let cfg_a = peer("host_a", host_a_fp.clone(), &["standard"], &["standard"]);
    let cfg_b = peer(
        "host_b",
        host_b_fp.clone(),
        &[ADVISORY_CONSENT_INTENT],
        &[ADVISORY_CONSENT_INTENT],
    );
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(&PeerId::new("host_a"), &host_a_fp, &host_a_fp, 1)
        .await
        .unwrap();
    tofu.pin_first_contact(&PeerId::new("host_b"), &host_b_fp, &host_b_fp, 1)
        .await
        .unwrap();
    let router = LoopbackA2ARouter::new(vec![cfg_a, cfg_b], tofu);

    let frame = advisory_frame(
        "mira",
        "host_a",
        "nash",
        "host_b",
        mira_advisory_json(),
        IntentClass::Standard,
        4,
    );
    let err = LocalRouter::route_outbound(&router, frame, &HostId("host_b".into()))
        .await
        .expect_err("send-side denial");
    assert!(
        matches!(
            err,
            A2AError::IntentDenied {
                direction: IntentDirection::Send,
                ..
            }
        ),
        "expected send-side IntentDenied(EIntentDenied), got {err:?}"
    );
}
