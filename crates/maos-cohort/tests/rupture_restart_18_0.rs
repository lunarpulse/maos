#![forbid(unsafe_code)]
//! Story 18-0 AC7 (R4, D-18-0-11 C) — a consent-rupture id is unique across
//! restarts. The router used to mint `frame_id[0..8] ‖ counter` with a
//! process-local counter that restarts at 1, so a restarted router denying the
//! SAME frame again re-minted an id already journaled: the Transparency Log
//! answered `Duplicate`, the deny path answered `CODE_INTERNAL`, and the
//! grant-scoped withhold rows (D-17-8-03) rolled back with the base row.
//!
//! Two boots over ONE file-backed log, each with a fresh `A2ARouterCore` and
//! the production `CohortRuptureLogSink`: both denials answer
//! `CODE_INTENT_DENIED`, two durable rupture rows with distinct ids exist, and
//! the boot-2 frame is withheld from the rejected recipient.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_INTENT_DENIED, METHOD_IAC_DELIVER,
};
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, A2ARouterCore, ConsentAllowlists, InMemoryTofuPinStore,
    PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_cohort::CohortRuptureLogSink;
use maos_domain::frame::{
    ConsentEnvelope, ConsentRecall, FrameAddress, FramePayload, IacFrame, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::log_recall::LogRecallError;
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{
    FrameFilter, FrameKind, FrameRecipient, TransparencyLogAdapter,
};
use maos_spirit_abi::identity::{FrameKind as WireKind, HostId, SpiritId};

const C: u32 = 7;
const INTENT: &str = "diagnosis-handoff:read-only-evidence";
/// The peer's host id as this router knows it (its pinned peer config).
const PEER_HOST: &str = "loopback";
/// The frame the peer sends in both boots and this host denies in both.
const DENIED: [u8; 16] = [0xF0; 16];

fn address(spirit: &str, host: Option<&str>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(spirit),
        host_id: host.map(|host| HostId(host.into())),
        role: None,
    }
}

fn peer() -> FrameAddress {
    address("peer", Some(PEER_HOST))
}

fn granted(intent: &str) -> ConsentEnvelope {
    ConsentEnvelope {
        consent_id: [0xC0; 16],
        granter: peer(),
        timestamp_ns: 1,
        intent_class: Some(A2AIntent::new(intent)),
        valid_until_ns: None,
        recall: ConsentRecall::Full,
    }
}

fn frame(id: [u8; 16], to: FrameAddress) -> IacFrame {
    IacFrame {
        frame_id: id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: peer(),
        to: std::iter::once(to).collect(),
        kind: WireKind::TaskAssign,
        intent: IntentClass::Readonly,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "peer delegation".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(granted(INTENT)),
        intent_lineage: Default::default(),
    }
}

/// A granted same-intent frame from the peer, routed to local recipient `c`.
fn received(log: &TransparencyLogAdapter, id: [u8; 16]) {
    let recipients = [FrameRecipient {
        pid: C,
        spirit_id: "c".into(),
        role: None,
    }];
    log.insert_routed_frame(
        &frame(id, address("c", None)),
        FrameKind::TaskAssign,
        0,
        None,
        "readonly",
        b"x",
        &recipients,
    );
}

/// One boot: the log file reopened under `nonce`, a fresh router whose accept
/// allowlist denies `INTENT`, and the production rupture sink.
async fn boot(path: &Path, nonce: u64) -> (Arc<TransparencyLogAdapter>, A2ARouterCore) {
    let log = Arc::new(TransparencyLogAdapter::open(path, nonce).expect("open the log file"));
    let admitted = Arc::new(RwLock::new(BTreeMap::from([("c".to_string(), C)])));
    log.install_recipient_resolver(admitted as _).unwrap();
    let config = A2APeerConfig {
        peer_id: PeerId::new(PEER_HOST),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![],
            accept_allowlist: vec![],
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    };
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new(PEER_HOST),
        &config.cert_fingerprint,
        &config.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let router = A2ARouterCore::new(vec![config], tofu);
    router
        .install_rupture_sink(Arc::new(CohortRuptureLogSink::new(Arc::clone(&log))))
        .await;
    (log, router)
}

/// The peer delivers `DENIED` to `c@host-a`; this host's policy denies it.
async fn deny(router: &A2ARouterCore, boot: &str) {
    let request = A2AJsonRpcRequest::new(
        METHOD_IAC_DELIVER,
        frame(DENIED, address("c", Some("host-a"))),
        1,
    );
    match router.handle_intake(request).await {
        A2AJsonRpcResponse::Nack(nack) => assert_eq!(
            nack.error.code, CODE_INTENT_DENIED,
            "{boot}: the denial must be journaled and answered as a policy deny, \
             never CODE_INTERNAL: {}",
            nack.error.message
        ),
        other => panic!("{boot}: expected a NACK, got {other:?}"),
    }
}

fn rupture_ids(log: &TransparencyLogAdapter) -> Vec<[u8; 16]> {
    log.query_frames(FrameFilter {
        kind: Some(FrameKind::ConsentRupture),
        ..Default::default()
    })
    .expect("query rupture rows")
    .into_iter()
    .map(|row| row.frame_id)
    .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn the_same_denied_frame_after_a_restart_journals_a_second_rupture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transparency.sqlite");

    // Boot 1.
    {
        let (log, router) = boot(&path, 0x18_01).await;
        received(&log, [1; 16]);
        deny(&router, "boot 1").await;
        let recall = LogRecallAdapter::new(Arc::clone(&log));
        assert_eq!(
            recall.fetch(C, [1; 16]),
            Err(LogRecallError::ConsentWithheld { frame_id: [1; 16] }),
            "boot 1: the rupture withholds the peer's earlier same-intent frame"
        );
        assert_eq!(rupture_ids(&log).len(), 1);
    }

    // Boot 2: same log file, new nonce, fresh router (its counter would restart).
    let (log, router) = boot(&path, 0x18_02).await;
    received(&log, [2; 16]);
    let recall = LogRecallAdapter::new(Arc::clone(&log));
    assert!(
        recall.fetch(C, [2; 16]).is_ok(),
        "boot 2: the boot-1 rupture does not withhold a frame of this boot"
    );
    deny(&router, "boot 2").await;

    let ids = rupture_ids(&log);
    assert_eq!(
        ids.len(),
        2,
        "two durable ConsentRupture base rows: {ids:02x?}"
    );
    assert_ne!(ids[0], ids[1], "the restarted router minted a fresh id");
    let grant_rows: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT count(*) FROM transparency_log_consent_ruptures WHERE recipient_pid = ?1",
            [C],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(grant_rows, 2, "one grant-scoped rupture row per denial");
    assert_eq!(
        recall.fetch(C, [2; 16]),
        Err(LogRecallError::ConsentWithheld { frame_id: [2; 16] }),
        "boot 2: the second rupture withholds the boot-2 frame (D-17-8-03)"
    );
}
