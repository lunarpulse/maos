//! Story 12.3 — Fact-3: transport observability of cross-agent halt receipts +
//! the WIRING PROOF (P7c) + absence classification (P2/P2a/P3) + replay-dedup
//! (P4) + source-identity (P5r). All legs derive-and-reconcile against the
//! observer's REAL read; a planted lie (dropped receipt, wrong marker, double
//! count, unverified source) turns each red.
//!
//! `maos-a2a-tcp` MUST NOT depend on `maos-kernel-core` (the enforced `t12a`
//! gate), so these legs ship a genuinely-produced `HaltReceipt` fixture built
//! via the public `HaltReceipt::new` constructor (NOT a serialized blob — a blob
//! would drift, the 10.2 trap). The REAL `invoke_halt` provenance is Fact-2's
//! (`maos-bin`).

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::identity::PeerId;
use maos_a2a_core::router::{A2APeerRouter, A2ARouterCore, A2ATransport};
use maos_a2a_core::{
    A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, CohortManifestGate, HaltReceiptObserver,
    InMemoryTofuPinStore, PeerCertFingerprint, TofuPinStore,
};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_cohort::{
    AbsenceKind, CohortAuthority, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, HaltPresence, HaltReceiptControl, HaltReceiptDistributor,
    InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys, COHORT_SCHEMA_V1,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{FrameAddress, IacFrame};
use maos_domain::halt::{HaltId, HaltReceipt};
use maos_spirit_abi::identity::{HostId, SpiritId};
use support::*;

/// A recording observer whose presence table IS the oracle for the wiring proof
/// (P7c): a shipped `cohort:halt-receipt` must land in ITS table via the real
/// `bind → handle_intake_verified → observer` path. A mis-wired / inert / wrong-
/// instance observer records nothing and reds the assertion.
#[derive(Default)]
struct RecordingObserver {
    seen: Mutex<Vec<String>>,
}

impl RecordingObserver {
    fn recorded(&self) -> Vec<String> {
        self.seen.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl HaltReceiptObserver for RecordingObserver {
    fn observe_receipt(&self, _member: &HostId, frame: &IacFrame) {
        if let Ok(control) = HaltReceiptControl::from_frame(frame) {
            if let Ok(mut guard) = self.seen.lock() {
                guard.push(control.halt_id().to_string());
            }
        }
    }
}

/// A router double capturing the courier's frame (used only to build a genuine
/// halt-receipt request for the direct-core source-identity leg).
#[derive(Default)]
struct CapturingRouter {
    captured: Mutex<Vec<IacFrame>>,
}

#[async_trait::async_trait]
impl A2APeerRouter for CapturingRouter {
    async fn route_outbound(
        &self,
        frame: IacFrame,
        _peer: &HostId,
    ) -> Result<(), maos_a2a_core::A2AError> {
        if let Ok(mut guard) = self.captured.lock() {
            guard.push(frame);
        }
        Ok(())
    }
    async fn handle_intake(
        &self,
        _request: A2AJsonRpcRequest,
    ) -> maos_a2a_core::A2AJsonRpcResponse {
        unreachable!("capturing router never handles intake")
    }
}

/// Router double for fan-out behavior: the middle roster member fails, but the
/// courier must still attempt each later member.
#[derive(Default)]
struct FanoutRouter {
    attempted: Mutex<Vec<String>>,
}

impl FanoutRouter {
    fn attempted(&self) -> Vec<String> {
        self.attempted.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl A2APeerRouter for FanoutRouter {
    async fn route_outbound(
        &self,
        _frame: IacFrame,
        peer: &HostId,
    ) -> Result<(), maos_a2a_core::A2AError> {
        self.attempted
            .lock()
            .expect("fan-out attempt table is available")
            .push(peer.as_str().to_string());
        if peer.as_str() == "host_b" {
            Err(A2AError::Io("induced middle-peer loss".into()))
        } else {
            Ok(())
        }
    }

    async fn handle_intake(
        &self,
        _request: A2AJsonRpcRequest,
    ) -> maos_a2a_core::A2AJsonRpcResponse {
        unreachable!("fan-out router never handles intake")
    }
}

fn signed_manifest(
    authority: &SigningKey,
    fp_a: &PeerCertFingerprint,
    fp_b: &PeerCertFingerprint,
) -> String {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-12-3-halt-receipt".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: fp_a.wire(),
                roles: vec!["worker".into()],
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: fp_b.wire(),
                roles: vec!["worker".into()],
            },
        ],
        consent: ConsentMatrix {
            send: vec![ConsentTuple {
                peer: "host_b".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
            accept: vec![ConsentTuple {
                peer: "host_a".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(authority);
    toml::to_string(&manifest).expect("signed manifest serializes")
}

fn load_state(host: &str, toml: &str, authority: &SigningKey) -> Arc<CohortManifestState> {
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(
            HostId(host.into()),
            toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("state loads signed manifest"),
    )
}

fn from_addr(host: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from("cohort-control"),
        host_id: Some(HostId(host.into())),
        role: None,
    }
}

#[allow(clippy::too_many_arguments)]
async fn bind_node(
    clock: &Clock,
    ca: &Ca,
    own_leaf: &Leaf,
    own_nonce: u64,
    peer_host: &str,
    peer_leaf: &Leaf,
    peer_nonce: u64,
    gate: Option<Arc<dyn CohortManifestGate>>,
    observer: Option<Arc<dyn HaltReceiptObserver>>,
) -> Arc<TcpA2ATransport> {
    let pems = write_pem(own_leaf, Some(ca));
    let tcp = tcp_config(
        &pems,
        vec![pin(peer_host, &peer_leaf.fingerprint, peer_nonce)],
        Duration::from_secs(30),
    );
    let cfg = peer_cfg(
        peer_host,
        "tls://127.0.0.1:0",
        &peer_leaf.fingerprint,
        &["readonly"],
        &["readonly"],
    );
    Arc::new(
        TcpA2ATransport::bind_with_cohort_wiring(
            tcp,
            vec![cfg],
            own_nonce,
            TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            gate,
            observer,
        )
        .await
        .expect("bind cohort node"),
    )
}

fn receipt(id: &str) -> HaltReceipt {
    HaltReceipt::new(HaltId::new(id).unwrap(), 111, 42, 7, [9u8; 16])
}

#[tokio::test]
async fn broadcast_attempts_members_after_a_middle_peer_failure() {
    let authority = SigningKey::from_bytes(&[9u8; 32]);
    let fp_a = fingerprint_of(0xa1);
    let fp_b = fingerprint_of(0xb2);
    let mut manifest: CohortManifest =
        toml::from_str(&signed_manifest(&authority, &fp_a, &fp_b)).expect("fixture parses");
    manifest.members.push(CohortMember {
        host_id: "host_c".into(),
        fingerprint: fingerprint_of(0xc3).wire(),
        roles: vec!["worker".into()],
    });
    let toml = toml::to_string(&manifest.signed_with(&authority)).expect("fixture serializes");
    let state = load_state("host_a", &toml, &authority);
    let router = Arc::new(FanoutRouter::default());
    let courier = HaltReceiptDistributor::new(state, router.clone(), from_addr("host_a"));

    assert!(
        courier.broadcast(&receipt("halt-fanout")).await.is_err(),
        "the failed middle-peer delivery remains visible to the caller"
    );
    assert_eq!(
        router.attempted(),
        vec!["host_b", "host_c"],
        "a failed middle peer must not prevent the courier from attempting later members"
    );
}

/// A silent TCP endpoint that accepts connections and holds them open WITHOUT
/// ever speaking TLS — a peer reachable-then-partitioned, forcing the client
/// handshake to time out (§7.2 connectivity loss → `TransportFailed("timeout:")`).
async fn silent_endpoint() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock); // hold open, never respond
        }
    });
    (addr, handle)
}

// ─────────────────────────────────────────────────────────────────────────────

/// Leg `halt-receipt-observer-wired` (P7c, the anti-silent-green): a shipped
/// `cohort:halt-receipt` is recorded by the INJECTED observer through the real
/// `bind → handle_intake_verified → observer` path. A mis-wired composition reds.
#[tokio::test]
#[ignore = "Story 12.3 — check-cohort-mesh owns the real-TCP observer-wiring proof"]
async fn t_12_3_observer_wired_over_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-3-wired");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[3u8; 32]);
    let toml = signed_manifest(&authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let state_a = load_state("host_a", &toml, &authority);

    let recorder = Arc::new(RecordingObserver::default());
    let observer: Arc<dyn HaltReceiptObserver> = recorder.clone();

    let transport_a = bind_node(&clock, &ca, &leaf_a, 1, "host_b", &leaf_b, 2, None, None).await;
    let transport_b = bind_node(
        &clock,
        &ca,
        &leaf_b,
        2,
        "host_a",
        &leaf_a,
        1,
        None,
        Some(observer),
    )
    .await;
    let addr_b = transport_b.local_addr().unwrap();
    transport_a.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{addr_b}"));

    let router: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor = HaltReceiptDistributor::new(state_a, router, from_addr("host_a"));
    let r = receipt("halt-wired-1");
    distributor
        .push_receipt_to(&HostId("host_b".into()), &r)
        .await
        .expect("receipt ships over live TCP");

    // Derive-and-reconcile: the injected observer's REAL table holds exactly the
    // shipped halt_id — proving the composition wired THIS instance onto the
    // verified-intake path.
    assert_eq!(
        recorder.recorded(),
        vec!["halt-wired-1".to_string()],
        "the injected observer recorded the shipped receipt"
    );
}

/// Leg `halt-receipt-replay-dedup` (P4): the SAME receipt shipped twice keeps the
/// presence count at 1 (dedup by `halt_id`, NOT the per-ship envelope frame_id);
/// a DISTINCT receipt raises it to 2 (non-vacuous).
#[tokio::test]
#[ignore = "Story 12.3 — check-cohort-mesh owns the replay-dedup proof"]
async fn t_12_3_replay_dedup_over_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-3-dedup");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[4u8; 32]);
    let toml = signed_manifest(&authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let state_a = load_state("host_a", &toml, &authority);
    let state_b = load_state("host_b", &toml, &authority);

    let observer: Arc<dyn HaltReceiptObserver> = state_b.clone();
    let gate_b: Arc<dyn CohortManifestGate> = state_b.clone();
    let transport_a = bind_node(&clock, &ca, &leaf_a, 1, "host_b", &leaf_b, 2, None, None).await;
    let transport_b = bind_node(
        &clock,
        &ca,
        &leaf_b,
        2,
        "host_a",
        &leaf_a,
        1,
        Some(gate_b),
        Some(observer),
    )
    .await;
    let addr_b = transport_b.local_addr().unwrap();
    transport_a.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{addr_b}"));

    let router: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor = HaltReceiptDistributor::new(state_a, router, from_addr("host_a"));
    let host_a = HostId("host_a".into());

    let r1 = receipt("halt-dedup-1");
    distributor
        .push_receipt_to(&HostId("host_b".into()), &r1)
        .await
        .unwrap();
    distributor
        .push_receipt_to(&HostId("host_b".into()), &r1)
        .await
        .unwrap();
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        1,
        "the SAME receipt shipped twice dedups to one (keyed on halt_id, not frame_id)"
    );

    let r2 = receipt("halt-dedup-2");
    distributor
        .push_receipt_to(&HostId("host_b".into()), &r2)
        .await
        .unwrap();
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        2,
        "a distinct receipt raises the count — dedup is by identity, not a stuck constant"
    );
}

/// Leg `halt-receipt-absence-member-loss` (P2a/P3): a probe to a DROPPED member
/// returns `A2AError::Io`, classified ABSENT(MemberLoss); paired with a PRESENT
/// positive before the drop so up/down is proven distinguished.
#[tokio::test]
#[ignore = "Story 12.3 — check-cohort-mesh owns the member-loss absence proof"]
async fn t_12_3_absence_member_loss_over_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-3-member-loss");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[5u8; 32]);
    let toml = signed_manifest(&authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let state_a = load_state("host_a", &toml, &authority);
    let state_b = load_state("host_b", &toml, &authority);

    let gate_b: Arc<dyn CohortManifestGate> = state_b.clone();
    let transport_a = bind_node(&clock, &ca, &leaf_a, 1, "host_b", &leaf_b, 2, None, None).await;
    let transport_b = bind_node(
        &clock,
        &ca,
        &leaf_b,
        2,
        "host_a",
        &leaf_a,
        1,
        Some(gate_b),
        None,
    )
    .await;
    let addr_b = transport_b.local_addr().unwrap();
    transport_a.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{addr_b}"));

    let router: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor = HaltReceiptDistributor::new(state_a.clone(), router, from_addr("host_a"));
    let host_b = HostId("host_b".into());

    // PRESENT positive: an up member Acks the reserved manifest-PULL probe.
    assert_eq!(
        distributor.classify_presence(&host_b).await.unwrap(),
        HaltPresence::Present,
        "an up member is PRESENT"
    );

    // Induce a clean member loss and re-probe.
    drop(transport_b);
    let verdict = distributor.classify_presence(&host_b).await.unwrap();
    assert_eq!(
        verdict,
        HaltPresence::Absent(AbsenceKind::MemberLoss),
        "a dropped member probes to Io → ABSENT(MemberLoss), NOT PartitionTimeout"
    );
    assert_eq!(
        state_a.absence_of(&host_b),
        Some(AbsenceKind::MemberLoss),
        "the observer persists the first-class member-loss marker for the digest"
    );
    assert!(
        transport_a.last_dial_attempts() <= 1,
        "zero-retry: one dial attempt for a non-retryable Io"
    );
}

/// Leg `halt-receipt-absence-connectivity-loss` (P2a/P3): a probe to a member at
/// a dead (silent, never-TLS) endpoint times out → `TransportFailed("timeout:")`,
/// classified ABSENT(ConnectivityLoss) — a DISTINCT variant from member loss,
/// paired with a PRESENT positive.
#[tokio::test]
#[ignore = "Story 12.3 — check-cohort-mesh owns the connectivity-loss absence proof"]
async fn t_12_3_absence_connectivity_loss_over_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-3-conn-loss");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[6u8; 32]);
    let toml = signed_manifest(&authority, &leaf_a.fingerprint, &leaf_b.fingerprint);
    let state_a = load_state("host_a", &toml, &authority);
    let state_b = load_state("host_b", &toml, &authority);

    let gate_b: Arc<dyn CohortManifestGate> = state_b.clone();
    let transport_a = bind_node(&clock, &ca, &leaf_a, 1, "host_b", &leaf_b, 2, None, None).await;
    let transport_b = bind_node(
        &clock,
        &ca,
        &leaf_b,
        2,
        "host_a",
        &leaf_a,
        1,
        Some(gate_b),
        None,
    )
    .await;
    let addr_b = transport_b.local_addr().unwrap();
    let host_b = HostId("host_b".into());

    let router: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor = HaltReceiptDistributor::new(state_a.clone(), router, from_addr("host_a"));

    // PRESENT positive at the real endpoint.
    transport_a.set_peer_endpoint(&host_b, format!("tls://{addr_b}"));
    assert_eq!(
        distributor.classify_presence(&host_b).await.unwrap(),
        HaltPresence::Present,
        "a reachable member is PRESENT"
    );

    // Repoint at a silent endpoint (TCP accepts, TLS never completes) → timeout.
    let (silent_addr, _silent) = silent_endpoint().await;
    transport_a.set_peer_endpoint(&host_b, format!("tls://{silent_addr}"));
    let verdict = distributor.classify_presence(&host_b).await.unwrap();
    assert_eq!(
        verdict,
        HaltPresence::Absent(AbsenceKind::ConnectivityLoss),
        "a partitioned member times out → ABSENT(ConnectivityLoss), a DISTINCT marker"
    );
    assert_eq!(
        state_a.absence_of(&host_b),
        Some(AbsenceKind::ConnectivityLoss),
        "the observer persists the first-class connectivity-loss marker for the digest"
    );
    // Keep the live node alive until the assertions complete.
    drop(transport_b);
}

/// Leg `halt-source-identity` (P5r): a halt receipt reaching the router via the
/// UNVERIFIED direct `handle_intake` path (no TLS anchor), or with `from` ≠ the
/// TLS-verified peer, is NOT counted. Only the verified path with a matching
/// `from` increments presence. Driven against the core directly (deterministic).
#[tokio::test]
#[ignore = "Story 12.3 — check-cohort-mesh owns the source-identity proof"]
async fn t_12_3_source_identity_over_core() {
    let authority = SigningKey::from_bytes(&[8u8; 32]);
    let fp_a = fingerprint_of(0xa1);
    let fp_b = fingerprint_of(0xb2);
    let toml = signed_manifest(&authority, &fp_a, &fp_b);
    let state_a = load_state("host_a", &toml, &authority);
    let state_b = load_state("host_b", &toml, &authority);

    // Build a genuine halt-receipt request via the real courier (capturing router).
    let capture = Arc::new(CapturingRouter::default());
    let cap_router: Arc<dyn A2APeerRouter> = capture.clone();
    let distributor = HaltReceiptDistributor::new(state_a, cap_router, from_addr("host_a"));
    distributor
        .push_receipt_to(&HostId("host_b".into()), &receipt("halt-src-1"))
        .await
        .unwrap();
    let frame = capture.captured.lock().unwrap()[0].clone();
    let request = A2AJsonRpcRequest::new("iac.deliver", frame, 1);

    // Build a verified receiver whose ordinary intake accepts the valid control
    // frame. The observer must still remain absent from the unverified path.
    let cfg = peer_cfg(
        "host_a",
        "tls://127.0.0.1:0",
        &fp_a,
        &["readonly"],
        &["readonly"],
    );
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &cfg.peer_id,
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("TOFU pin installs");
    let observer: Arc<dyn HaltReceiptObserver> = state_b.clone();
    let core = A2ARouterCore::new(vec![cfg], tofu)
        .with_pinned_consent_clock(1)
        .with_halt_receipt_observer(observer);
    let host_a = HostId("host_a".into());

    // (1) Unverified direct entry — the observer is NOT on this path.
    assert!(matches!(
        core.handle_intake(request.clone()).await,
        A2AJsonRpcResponse::Ack(_)
    ));
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        0,
        "a receipt via the unverified handle_intake path is NOT counted"
    );

    // (2) Verified path but `from` (host_a) ≠ TLS-verified peer (host_z) → refused
    //     BEFORE observe.
    let (_resp, passed) = core
        .handle_intake_verified(request.clone(), &PeerId::new("host_z"))
        .await;
    assert!(!passed, "identity mismatch fails the binding");
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        0,
        "a from≠verified-peer receipt is NOT counted (spoof-proof site, P5r)"
    );

    // (3) A valid, verified receipt reaches an ACK and is then counted.
    assert!(matches!(
        core.handle_intake_verified(request.clone(), &PeerId::new("host_a"))
            .await
            .0,
        A2AJsonRpcResponse::Ack(_)
    ));
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        1,
        "only a TLS-anchored receipt accepted by normal intake is counted"
    );

    // (4) A matching TLS peer is insufficient: an expired envelope NACKs and
    // must not create a second presence record.
    let mut expired = request;
    expired.id = 2;
    expired
        .params
        .consent_envelope
        .as_mut()
        .expect("courier supplies a consent envelope")
        .valid_until_ns = Some(0);
    assert!(matches!(
        core.handle_intake_verified(expired, &PeerId::new("host_a"))
            .await
            .0,
        A2AJsonRpcResponse::Nack(_)
    ));
    assert_eq!(
        state_b.present_receipt_count(&host_a),
        1,
        "an expired receipt is NACKed and never counted as presence"
    );
}

fn fingerprint_of(byte: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", hex::encode([byte; 32]))).unwrap()
}
