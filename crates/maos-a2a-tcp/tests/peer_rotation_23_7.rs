//! Story 23-7 — peer identity rotation, in-process scenes on real sockets and real mTLS.
//!
//! Nothing here sleeps for a rotation: grace is a VIRTUAL clock (`VirtualTimer`) the scene
//! advances, so the order of events is the only thing asserted (no timing assumption), and the
//! closers are the production ones (`PeerCertRotation::schedule_close`).
//!
//! * AC2(d) plane C moves with an own-leaf swap (the Send-seam team declaration follows it);
//! * AC3 grace `G` covers one pull lag: three nodes, two peers applying one reissue a confirmation
//!   interval apart, the rotating host swapping between them;
//! * AC6 a peer that missed the rotation fails closed, heals on the next pull, and the promotion
//!   survives (and is refused un-carried) across restarts;
//! * AC7 the durable manifest-version floor refuses a replayed superseded reissue after restart.

mod support;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{
    A2APeerConfig, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    CohortReissueDisposition, CohortReissueRejection, ConsentRuptureSink,
};
use maos_a2a_tcp::{OwnLeafReload, TcpA2AConfig, TcpA2ATransport, TcpTimeouts, TcpTransportError};
use maos_cohort::rotation::RotationGraceTimer;
use maos_cohort::OwnLeafCommit;
use maos_cohort::{
    CohortAuditEvent, CohortAuthority, CohortError, CohortManifest, CohortManifestForkReason,
    CohortManifestState, CohortMember, ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink,
    ManifestSignature, PinnedAuthorityKeys, ReissueOutcome, COHORT_SCHEMA_V1,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{FramePayload, IacFrame, RuptureReason};
use maos_domain::invariants::i1::IntentClass;
use maos_persistence::{FilePinStore, PinFileError, PIN_FILE_NAME};
use maos_spirit_abi::identity::{FrameKind, HostId};
use support::*;

const BOOT_NONCE: u64 = 5;
/// The signed lease of every manifest here: `confirmation_interval() = (30 + 1) / 2 = 15 s`.
const T_STALE: u64 = 30;

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

// ───────────────────────────── a virtual grace clock ─────────────────────────────

#[derive(Default)]
struct VirtualTimer {
    now: Mutex<Duration>,
    pending: Mutex<Vec<(Duration, Box<dyn FnOnce() + Send>)>>,
    armed: Mutex<Vec<Duration>>,
}

impl RotationGraceTimer for VirtualTimer {
    fn schedule(&self, grace: Duration, on_expiry: Box<dyn FnOnce() + Send>) {
        let deadline = *self.now.lock().unwrap() + grace;
        self.armed.lock().unwrap().push(grace);
        self.pending.lock().unwrap().push((deadline, on_expiry));
    }
}

impl VirtualTimer {
    /// Move the clock to `t` and run every closer due by then, in deadline order.
    fn advance_to(&self, t: Duration) {
        *self.now.lock().unwrap() = t;
        let mut due = Vec::new();
        {
            let mut pending = self.pending.lock().unwrap();
            let mut index = 0;
            while index < pending.len() {
                if pending[index].0 <= t {
                    due.push(pending.remove(index));
                } else {
                    index += 1;
                }
            }
        }
        due.sort_by_key(|(deadline, _)| *deadline);
        for (_, closer) in due {
            closer();
        }
    }

    fn armed(&self) -> Vec<Duration> {
        self.armed.lock().unwrap().clone()
    }
}

// ───────────────────────────── a recording rupture journal ─────────────────────────────

#[derive(Default)]
struct Journal(parking_lot::Mutex<Vec<IacFrame>>);

impl Journal {
    fn reasons(&self) -> Vec<RuptureReason> {
        self.0
            .lock()
            .iter()
            .filter(|frame| frame.kind == FrameKind::ConsentRupture)
            .filter_map(|frame| match &frame.payload {
                FramePayload::ConsentRupture(payload) => {
                    payload.rejected.first().map(|rejection| rejection.reason)
                }
                _ => None,
            })
            .collect()
    }
}

impl ConsentRuptureSink for Journal {
    fn append(&self, frame: &IacFrame) -> Result<(), String> {
        self.0.lock().push(frame.clone());
        Ok(())
    }
}

// ───────────────────────────── signed manifests ─────────────────────────────

fn manifest_toml(
    version: u64,
    signer: &SigningKey,
    members: &[(&str, &PeerCertFingerprint)],
) -> String {
    let tuples = || {
        members
            .iter()
            .map(|(host, _)| ConsentTuple {
                peer: (*host).to_string(),
                role: "worker".into(),
                intent: "readonly".into(),
            })
            .collect::<Vec<_>>()
    };
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-23-7-rotation".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signer.verifying_key().to_bytes())],
        },
        members: members
            .iter()
            .map(|(host, fingerprint)| CohortMember {
                host_id: (*host).to_string(),
                fingerprint: fingerprint.wire(),
                roles: vec!["worker".into()],
                team: None,
            })
            .collect(),
        consent: ConsentMatrix {
            send: tuples(),
            accept: tuples(),
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: T_STALE,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(signer);
    toml::to_string(&manifest).expect("manifest serializes")
}

fn state_for(
    local: &str,
    v1: &str,
    signer: &SigningKey,
) -> (Arc<CohortManifestState>, Arc<InMemoryCohortAuditSink>) {
    let audit = Arc::new(InMemoryCohortAuditSink::default());
    let pinned = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("authority");
    let state = CohortManifestState::load(
        HostId(local.to_string()),
        v1,
        pinned,
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("the signed manifest loads");
    (Arc::new(state), audit)
}

// ───────────────────────────── node plumbing ─────────────────────────────

fn peer_config(name: &str, fingerprint: &PeerCertFingerprint) -> A2APeerConfig {
    peer_cfg(
        name,
        "tls://127.0.0.1:0",
        fingerprint,
        &["readonly"],
        &["readonly"],
    )
}

/// Bind one node. `pems` is kept by the caller so the PKI can overwrite the files.
async fn bind_node(
    clock: &Clock,
    pems: &PemFiles,
    pins: &[(&str, &PeerCertFingerprint)],
    persistence: Option<Arc<FilePinStore>>,
) -> Result<TcpA2ATransport, TcpTransportError> {
    let cfg: TcpA2AConfig = tcp_config(
        pems,
        pins.iter()
            .map(|(peer, fingerprint)| pin(peer, fingerprint, BOOT_NONCE))
            .collect(),
        secs(30),
    );
    let peers = pins
        .iter()
        .map(|(peer, fingerprint)| peer_config(peer, fingerprint))
        .collect();
    TcpA2ATransport::bind_with_intake_sink(
        cfg,
        peers,
        BOOT_NONCE,
        TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        persistence.map(|store| store as Arc<dyn maos_a2a_core::tofu::PinPersistence>),
    )
    .await
}

async fn send(from: &TcpA2ATransport, from_name: &str, to_name: &str, seq: u64) -> bool {
    from.route_outbound(
        make_frame(from_name, to_name, IntentClass::Readonly, seq),
        &HostId(to_name.to_string()),
    )
    .await
    .is_ok()
}

fn wire(a: &TcpA2ATransport, a_name: &str, b: &TcpA2ATransport, b_name: &str) {
    a.set_peer_endpoint(
        &HostId(b_name.to_string()),
        format!("tls://{}", b.local_addr().unwrap()),
    );
    b.set_peer_endpoint(
        &HostId(a_name.to_string()),
        format!("tls://{}", a.local_addr().unwrap()),
    );
}

async fn open_store(dir: &Path) -> Arc<FilePinStore> {
    // The previous boot's aborted accept tasks release the single-writer lock only when the
    // runtime next polls (drops) them: retry the open against a deadline, yielding to the runtime
    // in between, never a fixed pause.
    let path = dir.join(PIN_FILE_NAME);
    let deadline = std::time::Instant::now() + secs(10);
    loop {
        match FilePinStore::open(&path) {
            Ok(store) => return Arc::new(store),
            Err(PinFileError::Held { .. }) if std::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Err(error) => panic!("pin file: {error}"),
        }
    }
}

// ───────────────────────────── AC2(d): plane C moves with the swap ─────────────────────────────

/// A gate that stamps a team only when the host's own leaf (plane C) equals the signed one, and
/// records what plane C said on every Send.
struct StampingGate {
    signed: PeerCertFingerprint,
    seen: Mutex<Vec<(Option<PeerCertFingerprint>, Option<String>)>>,
}

impl CohortManifestGate for StampingGate {
    fn consent_decision(
        &self,
        _: CohortConsentSeam,
        _: &HostId,
        _: Option<&str>,
        _: &str,
        _: Option<u64>,
    ) -> CohortConsentVerdict {
        CohortConsentVerdict::Defer
    }
    fn apply_reissue(
        &self,
        _: &HostId,
        _: u64,
        _: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        Err(CohortReissueRejection {
            reason: "not under test".into(),
            seen_version: None,
            rejected_version: None,
        })
    }
    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        _: &HostId,
        endpoint_fingerprint: Option<&PeerCertFingerprint>,
        _: Option<&str>,
        _: &str,
        _: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>) {
        let team = (seam == CohortConsentSeam::Send && endpoint_fingerprint == Some(&self.signed))
            .then(|| "team-a".to_string());
        if seam == CohortConsentSeam::Send {
            self.seen
                .lock()
                .unwrap()
                .push((endpoint_fingerprint.cloned(), team.clone()));
        }
        (CohortConsentVerdict::Defer, team)
    }
}

#[tokio::test]
async fn a_swap_moves_plane_c_so_the_send_seam_stamps_the_declaration() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-planec");
    let old = valid_leaf(&ca, &clock);
    let new = valid_leaf(&ca, &clock);
    let pems = write_pem(&old, Some(&ca));
    let gate = Arc::new(StampingGate {
        signed: new.fingerprint.clone(),
        seen: Mutex::new(Vec::new()),
    });
    let cfg = tcp_config(&pems, vec![], secs(30));
    let x = TcpA2ATransport::bind_with_cohort_manifest_gate(
        cfg,
        vec![peer_cfg(
            "peer",
            &dead_endpoint(),
            &old.fingerprint,
            &["readonly"],
            &[],
        )],
        BOOT_NONCE,
        TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
        Some(gate.clone() as Arc<dyn CohortManifestGate>),
    )
    .await
    .expect("X binds");

    // Before: plane C holds the retired leaf, so the declaration is NOT stamped.
    let _ = send(&x, "x", "peer", 1).await;
    // The PKI writes the declared leaf; the reconcile step swaps it.
    std::fs::write(&pems.cert_chain, &new.cert_pem).unwrap();
    std::fs::write(&pems.private_key, &new.key_pem).unwrap();
    assert!(matches!(
        x.reload_own_identity(&new.fingerprint).unwrap(),
        OwnLeafReload::Swapped { .. }
    ));
    let _ = send(&x, "x", "peer", 2).await;

    let seen = gate.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "one Send-seam gate call per frame: {seen:?}");
    assert_eq!(seen[0], (Some(old.fingerprint.clone()), None));
    assert_eq!(
        seen[1],
        (Some(new.fingerprint.clone()), Some("team-a".to_string())),
        "after the swap plane C is the served leaf and the team declaration is stamped"
    );
}

/// Delivers a peer's `Pull` frame (built by the production `CohortDistributor::pull_from`) to the
/// target host's manifest gate, as the router would after the TLS peer binding.
struct PullDelivery {
    target: Arc<CohortManifestState>,
    from: String,
}

#[async_trait::async_trait]
impl A2APeerRouter for PullDelivery {
    async fn route_outbound(
        &self,
        frame: IacFrame,
        _peer: &HostId,
    ) -> Result<(), maos_a2a_core::A2AError> {
        CohortManifestGate::apply_reissue(
            &*self.target,
            &HostId(self.from.clone()),
            BOOT_NONCE,
            &frame,
        )
        .map(|_| ())
        .map_err(|rejection| maos_a2a_core::A2AError::TransportFailed(rejection.reason))
    }

    async fn handle_intake(
        &self,
        _request: maos_a2a_core::A2AJsonRpcRequest,
    ) -> maos_a2a_core::A2AJsonRpcResponse {
        unreachable!("the pull-delivery double only carries outbound pulls")
    }
}

// ───────────────────────────── AC3: G covers one pull lag ─────────────────────────────

#[tokio::test]
async fn g_covers_one_pull_lag_and_the_old_leaf_is_refused_only_after_it() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-23-7-scene");
    let x_old = valid_leaf(&ca, &clock);
    let x_new = valid_leaf(&ca, &clock);
    let p1_leaf = valid_leaf(&ca, &clock);
    let p2_leaf = valid_leaf(&ca, &clock);
    let signer = SigningKey::from_bytes(&[23_u8; 32]);

    let x_pems = write_pem(&x_old, Some(&ca));
    let p1_pems = write_pem(&p1_leaf, Some(&ca));
    let p2_pems = write_pem(&p2_leaf, Some(&ca));
    let x = bind_node(
        &clock,
        &x_pems,
        &[("p1", &p1_leaf.fingerprint), ("p2", &p2_leaf.fingerprint)],
        None,
    )
    .await
    .unwrap();
    let p1 = bind_node(
        &clock,
        &p1_pems,
        &[("x", &x_old.fingerprint), ("p2", &p2_leaf.fingerprint)],
        None,
    )
    .await
    .unwrap();
    let p2 = bind_node(
        &clock,
        &p2_pems,
        &[("x", &x_old.fingerprint), ("p1", &p1_leaf.fingerprint)],
        None,
    )
    .await
    .unwrap();
    wire(&x, "x", &p1, "p1");
    wire(&x, "x", &p2, "p2");
    wire(&p1, "p1", &p2, "p2");
    let p1_journal = Arc::new(Journal::default());
    p1.core().install_rupture_sink(p1_journal.clone()).await;

    let v1 = manifest_toml(
        1,
        &signer,
        &[
            ("x", &x_old.fingerprint),
            ("p1", &p1_leaf.fingerprint),
            ("p2", &p2_leaf.fingerprint),
        ],
    );
    let v2 = manifest_toml(
        2,
        &signer,
        &[
            ("x", &x_new.fingerprint),
            ("p1", &p1_leaf.fingerprint),
            ("p2", &p2_leaf.fingerprint),
        ],
    );
    let timer = Arc::new(VirtualTimer::default());
    let base = maos_cohort::cold_deployment_t_grace();
    let mut states = Vec::new();
    for (name, node) in [("p1", &p1), ("p2", &p2)] {
        let (state, _audit) = state_for(name, &v1, &signer);
        state
            .install_cert_rotation(node.pins(), node.core(), timer.clone(), base)
            .expect("rotation installs");
        states.push(state);
    }
    let interval = states[0].confirmation_interval().unwrap();
    assert_eq!(interval, secs((T_STALE + 1) / 2));
    // D-23-7-03 A + review B5, as an equality and never a literal: G = the §7.2.1.a base + TWO pull
    // intervals — one for the spread of applies, one for the confirmation lag of X's own-leaf swap.
    let g = base + 2 * interval;

    // X's own state: the gate that decides WHEN its own-leaf swap may commit.
    let (x_state, _x_audit) = state_for("x", &v1, &signer);
    x_state
        .install_cert_rotation(x.pins(), x.core(), timer.clone(), base)
        .expect("X's rotation installs");
    // Each peer's production `pull_from`, delivered to X's gate (the frame is the shipped one; the
    // handshakes asserted below are the real ones).
    let courier = |peer: &str, state: &Arc<CohortManifestState>| {
        maos_cohort::CohortDistributor::new(
            Arc::clone(state),
            Arc::new(PullDelivery {
                target: Arc::clone(&x_state),
                from: peer.to_string(),
            }),
            maos_domain::frame::FrameAddress {
                spirit_id: maos_spirit_abi::identity::SpiritId::from("cohort-control"),
                host_id: Some(HostId(peer.to_string())),
                role: None,
            },
        )
    };
    let p1_courier = courier("p1", &states[0]);
    let p2_courier = courier("p2", &states[1]);
    // One reconcile attempt of X: read and validate the files off the lock, commit under the gate.
    let try_swap = || {
        let maos_a2a_tcp::OwnLeafPrepare::Ready(prepared) =
            x.prepare_own_identity(&x_new.fingerprint).unwrap()
        else {
            panic!("the PKI wrote the declared leaf");
        };
        x_state
            .commit_own_leaf(&x_new.fingerprint, || x.commit_own_identity(prepared))
            .unwrap()
    };

    // Baseline: the old generation everywhere, all four directions.
    assert!(send(&x, "x", "p1", 1).await && send(&p1, "p1", "x", 2).await);
    assert!(send(&x, "x", "p2", 3).await && send(&p2, "p2", "x", 4).await);

    // t = 0: P1 applies the reissue. P2 and X have not yet.
    assert_eq!(
        states[0].apply_reissue(&v2).unwrap(),
        ReissueOutcome::Applied { version: 2 }
    );
    assert_eq!(
        timer.armed(),
        vec![g],
        "the window P1 opened lasts G, not the bare base"
    );

    // One pull lag passes (less one second): X applies the reissue (its signed declaration names
    // x_new) and its PKI already wrote the files. X's swap DECISION POINT: no peer has confirmed v2
    // to X, so X holds its old leaf ...
    timer.advance_to(interval - secs(1));
    assert_eq!(
        x_state.apply_reissue(&v2).unwrap(),
        ReissueOutcome::Applied { version: 2 }
    );
    std::fs::write(&x_pems.cert_chain, &x_new.cert_pem).unwrap();
    std::fs::write(&x_pems.private_key, &x_new.key_pem).unwrap();
    let decision = try_swap();
    // ... so a handshake attempted AFTER that decision and BEFORE P2 applies the reissue is
    // accepted in both directions (P2 still pins only the retiring leaf and X still serves it),
    // and so is P1's, whose window is open. (Asserted BEFORE the decision itself, so that a
    // gate-less X, which swapped here, fails at the P2 handshake assertion first.)
    assert!(
        send(&x, "x", "p2", 5).await && send(&p2, "p2", "x", 6).await,
        "X<->P2 between X's swap decision and P2's apply: accepted both ways"
    );
    assert!(
        send(&x, "x", "p1", 7).await && send(&p1, "p1", "x", 8).await,
        "P1 must still accept the retiring leaf one pull lag after it applied the reissue"
    );
    assert!(
        matches!(decision, OwnLeafCommit::Held { unconfirmed } if unconfirmed == ["p1", "p2"]),
        "X must wait for the peers to confirm the reissue"
    );
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_old.fingerprint.clone())
    );

    // P1 confirms (its pull is the shipped frame, over to X's gate): X still waits for P2.
    p1_courier.pull_from(&HostId("x".into())).await.unwrap();
    assert!(matches!(
        try_swap(),
        OwnLeafCommit::Held { unconfirmed } if unconfirmed == ["p2"]
    ));
    assert!(send(&x, "x", "p2", 9).await && send(&p2, "p2", "x", 10).await);

    // t = interval: P2 applies it and confirms; X's gate opens and X swaps.
    timer.advance_to(interval);
    assert_eq!(
        states[1].apply_reissue(&v2).unwrap(),
        ReissueOutcome::Applied { version: 2 }
    );
    p2_courier.pull_from(&HostId("x".into())).await.unwrap();
    assert!(matches!(
        try_swap(),
        OwnLeafCommit::Committed { unconfirmed, .. } if unconfirmed.is_empty()
    ));
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_new.fingerprint.clone())
    );
    // Nothing has been refused in either direction, before any fire.
    assert!(send(&x, "x", "p1", 11).await && send(&p1, "p1", "x", 12).await);
    assert!(
        send(&x, "x", "p2", 13).await && send(&p2, "p2", "x", 14).await,
        "P2, a pull later than P1, accepts the new leaf both ways"
    );

    // P1's G elapses: it promotes. X (new) is still fine everywhere.
    timer.advance_to(g);
    assert!(send(&x, "x", "p1", 15).await && send(&x, "x", "p2", 16).await);

    // The ghost still serves and presents the RETIRED leaf. Before P2's G elapses P2 still takes it.
    let ghost_pems = write_pem(&x_old, Some(&ca));
    let ghost = bind_node(
        &clock,
        &ghost_pems,
        &[("p1", &p1_leaf.fingerprint), ("p2", &p2_leaf.fingerprint)],
        None,
    )
    .await
    .unwrap();
    wire(&ghost, "x", &p1, "p1");
    wire(&ghost, "x", &p2, "p2");
    assert!(
        !send(&ghost, "x", "p1", 15).await,
        "after P1's close the retired leaf is refused (listen side)"
    );

    // P2's G elapses too ("fire()"): the retired leaf is refused in every direction.
    timer.advance_to(interval + g);
    assert!(
        !send(&ghost, "x", "p2", 16).await,
        "retired leaf, listen side, P2"
    );
    // Dial side: re-point both peers' "x" at the ghost.
    p1.set_peer_endpoint(
        &HostId("x".into()),
        format!("tls://{}", ghost.local_addr().unwrap()),
    );
    p2.set_peer_endpoint(
        &HostId("x".into()),
        format!("tls://{}", ghost.local_addr().unwrap()),
    );
    assert!(
        !send(&p1, "p1", "x", 17).await,
        "retired leaf, dial side, P1"
    );
    assert!(
        !send(&p2, "p2", "x", 18).await,
        "retired leaf, dial side, P2"
    );
    // The dial-side refusal is the typed post-grace row: P1 holds a promotion for x.
    assert!(
        wait_until(
            || p1_journal
                .reasons()
                .contains(&RuptureReason::CertPostGraceReject),
            secs(5)
        )
        .await,
        "{:?}",
        p1_journal.reasons()
    );
    // And the live X is accepted again once the peers point back at it.
    wire(&x, "x", &p1, "p1");
    wire(&x, "x", &p2, "p2");
    assert!(send(&x, "x", "p1", 19).await && send(&x, "x", "p2", 20).await);
}

// ───────────────────────────── AC6 / AC7: restarts ─────────────────────────────

struct Rig {
    clock: Clock,
    x_old: Leaf,
    x_new: Leaf,
    x_mid: Leaf,
    p_leaf: Leaf,
    x: TcpA2ATransport,
    p_pems: PemFiles,
    pin_dir: tempfile::TempDir,
    signer: SigningKey,
    v1: String,
    _x_pems: PemFiles,
}

impl Rig {
    async fn new() -> Self {
        let clock = Clock::capture();
        let ca = mk_ca(&clock, "ca-23-7-restart");
        let x_old = valid_leaf(&ca, &clock);
        let x_new = valid_leaf(&ca, &clock);
        let x_mid = valid_leaf(&ca, &clock);
        let p_leaf = valid_leaf(&ca, &clock);
        let signer = SigningKey::from_bytes(&[24_u8; 32]);
        // X already rotated: it serves x_new from the start of this scene.
        let x_pems = write_pem(&x_new, Some(&ca));
        let x = bind_node(&clock, &x_pems, &[("p", &p_leaf.fingerprint)], None)
            .await
            .unwrap();
        let v1 = manifest_toml(
            1,
            &signer,
            &[("x", &x_old.fingerprint), ("p", &p_leaf.fingerprint)],
        );
        let p_pems = write_pem(&p_leaf, Some(&ca));
        Self {
            clock,
            x_old,
            x_new,
            x_mid,
            p_leaf,
            x,
            p_pems,
            pin_dir: tempfile::tempdir().unwrap(),
            signer,
            v1,
            _x_pems: x_pems,
        }
    }

    fn manifest(&self, version: u64, x: &Leaf) -> String {
        manifest_toml(
            version,
            &self.signer,
            &[("x", &x.fingerprint), ("p", &self.p_leaf.fingerprint)],
        )
    }

    /// Boot P from its UNCHANGED config (x pinned to the old leaf), its pin file and the on-disk v1.
    async fn boot_p(&self) -> Result<P, TcpTransportError> {
        let store = open_store(self.pin_dir.path()).await;
        let node = bind_node(
            &self.clock,
            &self.p_pems,
            &[("x", &self.x_old.fingerprint)],
            Some(store),
        )
        .await?;
        wire(&self.x, "x", &node, "p");
        let journal = Arc::new(Journal::default());
        node.core().install_rupture_sink(journal.clone()).await;
        let (state, audit) = state_for("p", &self.v1, &self.signer);
        let timer = Arc::new(VirtualTimer::default());
        state
            .install_cert_rotation(
                node.pins(),
                node.core(),
                timer.clone(),
                maos_cohort::cold_deployment_t_grace(),
            )
            .expect("rotation installs");
        Ok(P {
            node,
            state,
            audit,
            timer,
            journal,
        })
    }
}

struct P {
    node: TcpA2ATransport,
    state: Arc<CohortManifestState>,
    audit: Arc<InMemoryCohortAuditSink>,
    timer: Arc<VirtualTimer>,
    journal: Arc<Journal>,
}

#[tokio::test]
async fn a_peer_that_missed_the_rotation_fails_closed_heals_on_a_pull_and_keeps_the_promotion() {
    let rig = Rig::new().await;

    // P restarts from its unchanged config, pin file and on-disk manifest. X presents x_new.
    let p = rig.boot_p().await.expect("P boots");
    assert!(
        !send(&p.node, "p", "x", 1).await,
        "P refuses a leaf it holds no promotion for"
    );
    assert!(wait_until(|| !p.journal.reasons().is_empty(), secs(5)).await);
    assert!(
        p.journal
            .reasons()
            .iter()
            .all(|reason| *reason == RuptureReason::PeerIdentityUnverified),
        "never promoted ⇒ the untyped reason: {:?}",
        p.journal.reasons()
    );
    assert!(
        !send(&rig.x, "x", "p", 2).await,
        "and accepts nothing undeclared in the other direction"
    );
    assert_eq!(p.node.intake_entered(), 0);

    // P pulls the reissue from a non-rotated member (Y): a window opens, x_new is accepted.
    let v2 = rig.manifest(2, &rig.x_new);
    assert_eq!(
        p.state.apply_reissue(&v2).unwrap(),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(send(&p.node, "p", "x", 3).await && send(&rig.x, "x", "p", 4).await);

    // A restart in the middle of the window loses it (never persisted): refused again until the
    // next pull reopens it.
    drop(p);
    let p = rig.boot_p().await.expect("P reboots mid-window");
    assert!(
        !send(&p.node, "p", "x", 5).await,
        "the window did not survive the restart"
    );
    assert_eq!(
        p.state.apply_reissue(&v2).unwrap(),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(
        send(&p.node, "p", "x", 6).await,
        "the next pull reopens it (floor == version is not a replay)"
    );

    // fire(): the window closes, promoting and persisting.
    p.timer.advance_to(Duration::from_secs(3600));
    assert!(
        !p.node.pins().rotation_next(&PeerId::new("x")).is_some(),
        "the window is closed"
    );
    assert!(p.node.pins().is_promoted(&PeerId::new("x")));
    drop(p);

    // The promotion is durable and the floor with it ...
    let store = open_store(rig.pin_dir.path()).await;
    let records = maos_a2a_core::tofu::PinPersistence::load(&*store);
    let x_record = records.iter().find(|r| r.pin.peer.as_str() == "x").unwrap();
    assert_eq!(x_record.pin.fingerprint, rig.x_new.fingerprint);
    assert!(x_record.promoted);
    assert_eq!(x_record.generation, 1);
    assert_eq!(maos_a2a_core::tofu::PinPersistence::load_floor(&*store), 2);
    drop(store);

    // ... and a restart WITHOUT carrying the promotion into config is 21-3's typed refusal.
    let refusal = rig
        .boot_p()
        .await
        .err()
        .expect("config still names the retired leaf");
    assert!(
        matches!(refusal, TcpTransportError::PinPromotionNotCarried { .. }),
        "{refusal}"
    );
}

#[tokio::test]
async fn a_replayed_superseded_reissue_is_refused_below_the_durable_floor() {
    let rig = Rig::new().await;
    let p = rig.boot_p().await.expect("P boots");
    // v3 (declares x_new) is applied and its floor persisted; v2 (declares x_mid) is superseded.
    let v2 = rig.manifest(2, &rig.x_mid);
    let v3 = rig.manifest(3, &rig.x_new);
    assert_eq!(
        p.state.apply_reissue(&v3).unwrap(),
        ReissueOutcome::Applied { version: 3 }
    );
    assert_eq!(p.node.pins().manifest_floor(), 3);
    drop(p);

    // Restart: the in-memory seen_version is the on-disk manifest's (1); the floor is durable (3).
    let p = rig.boot_p().await.expect("P reboots");
    assert_eq!(p.node.pins().manifest_floor(), 3);
    assert_eq!(
        p.state.version().unwrap(),
        1,
        "boot is unchanged: the on-disk manifest"
    );
    let error = p
        .state
        .apply_reissue(&v2)
        .expect_err("a replayed superseded reissue");
    assert!(
        matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::VersionBelowDurableFloor,
                ..
            }
        ),
        "{error}"
    );
    assert!(
        p.audit.events().iter().any(|event| matches!(
            event,
            CohortAuditEvent::ReissueRejected {
                rejected_version: 2,
                ..
            }
        )),
        "a ReissueRejected row names the refusal"
    );
    assert!(
        p.node.pins().rotation_next(&PeerId::new("x")).is_none(),
        "no window opened to the superseded leaf"
    );
    // The current manifest still applies.
    assert_eq!(
        p.state.apply_reissue(&v3).unwrap(),
        ReissueOutcome::Applied { version: 3 }
    );
}
