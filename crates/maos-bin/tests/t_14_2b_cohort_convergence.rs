//! Story 14-2b — cohort convergence observability over the REAL wire.
//!
//! The story's whole deliverable is that a value stops being discarded, and the
//! named failure mode of such a story is *a test that passes against an empty
//! table*. Both tests here are therefore written so that an empty table CANNOT
//! satisfy them:
//!
//! * [`t_14_2b_peers_at_different_versions_are_told_apart`] asserts that two
//!   peers pulling at DIFFERENT signed versions produce two records this host
//!   can distinguish by version and by canonical hash. *"A peer that never
//!   pulls has no record"* would have been vacuous — an empty table satisfies
//!   it under every mutation of the retention write.
//! * [`t_14_2b_a_restarted_peer_is_not_a_convergence_claim`] asserts that ONE of
//!   two live records is invalidated by a peer restart while the OTHER stays
//!   valid, and then that the survivor ages out. An empty table has nothing to
//!   invalidate and nothing to leave standing, so it fails both halves.
//!
//! Real mTLS over real TCP, three hosts, the shipped `CohortDistributor` and the
//! shipped `CohortManifestGate` receive arm. Nothing here re-implements the
//! production path.

#[path = "../../maos-a2a-tcp/tests/support/mod.rs"]
mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{CohortManifestGate, CohortReissueDisposition};
use maos_bin::cert_rotation::CohortPeerVersions;
use maos_cohort::rotation::RotationGraceTimer;
use maos_cohort::{
    CohortAuthority, CohortClock, CohortDistributor, CohortManifest, CohortManifestControl,
    CohortManifestState, CohortMember, ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink,
    ManifestSignature, PeerConvergence, PinnedAuthorityKeys, COHORT_SCHEMA_V1,
    CONVERGENCE_OBSERVED, CONVERGENCE_RESTARTED, CONVERGENCE_STALE, RESERVED_INTENT_HALT_RECEIPT,
    RESERVED_INTENT_REISSUE,
};
use maos_control::{
    CohortConvergenceSource, OperatorHttpConfig, OperatorHttpServer, SandboxReportSource,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::sandbox::SandboxInspectReport;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use support::*;

/// An odd signed lease proves expiry uses the exact signed bound rather than
/// `2 * confirmation_interval()`, which rounds odd values up by one second.
const T_STALE_SECS: u64 = 31;

/// Each host's own `boot_nonce`, which is also the value its peers pin. A pin
/// whose `boot_nonce` disagrees with the sender's is exactly what
/// `invalidate_if_boot_nonce_differs` refuses, so these must be paired.
const NONCE_A: u64 = 1;
const NONCE_B: u64 = 2;
const NONCE_C: u64 = 3;

struct TestCohortClock(AtomicU64);

impl TestCohortClock {
    fn new(now: u64) -> Self {
        Self(AtomicU64::new(now))
    }

    fn set(&self, now: u64) {
        self.0.store(now, Ordering::Release);
    }
}

impl CohortClock for TestCohortClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}

/// The rotation control has to be INSTALLED for the pin generation to be
/// reachable from the cohort state, so this test needs a timer. It never fires:
/// the manifest re-declares the fingerprints already pinned, so the reload is
/// the no-op `RotationOutcome::unchanged` path.
struct NeverFires;

impl RotationGraceTimer for NeverFires {
    fn schedule(&self, _grace: Duration, _on_expiry: Box<dyn FnOnce() + Send>) {}
}

fn signed_manifest(
    version: u64,
    signer: &SigningKey,
    members: &[(&str, &str)],
    t_stale_secs: u64,
) -> String {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-14-2b-convergence".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signer.verifying_key().to_bytes())],
        },
        members: members
            .iter()
            .map(|(host_id, fingerprint)| CohortMember {
                host_id: (*host_id).into(),
                fingerprint: (*fingerprint).into(),
                roles: vec!["worker".into()],
                team: None,
            })
            .collect(),
        consent: ConsentMatrix {
            send: members
                .iter()
                .map(|(peer, _)| ConsentTuple {
                    peer: (*peer).into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
            accept: members
                .iter()
                .map(|(peer, _)| ConsentTuple {
                    peer: (*peer).into(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(signer);
    toml::to_string(&manifest).expect("cohort manifest serializes")
}

/// A live three-host cohort: `host_a` holds v3 and installs the rotation control
/// (so pin generations are observable); `host_b` holds v1 and `host_c` holds v2.
struct LiveCohort {
    clock_a: Arc<TestCohortClock>,
    state_a: Arc<CohortManifestState>,
    state_without_observer: Arc<CohortManifestState>,
    restarted_b: CohortDistributor,
    longer_lease: String,
    removed_b_manifest: String,
    hash_b: String,
    hash_c: String,
    // Held for the duration of the test: dropping a transport closes its
    // listener and every record under test would then be about a dead mesh.
    _transports: Vec<Arc<maos_a2a_tcp::TcpA2ATransport>>,
    distributor_a: CohortDistributor,
}

async fn bind_host_b_sender(
    clock: &Clock,
    ca: &Ca,
    leaf_a: &Leaf,
    leaf_b: &Leaf,
    addr_a: std::net::SocketAddr,
    state_b: Arc<CohortManifestState>,
    boot_nonce: u64,
) -> (Arc<maos_a2a_tcp::TcpA2ATransport>, CohortDistributor) {
    let pems = write_pem(leaf_b, Some(ca));
    let transport = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_config(
                &pems,
                vec![pin("host_a", &leaf_a.fingerprint, NONCE_A)],
                Duration::from_secs(30),
            ),
            vec![peer_cfg(
                "host_a",
                &format!("tls://{addr_a}"),
                &leaf_a.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            boot_nonce,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_b.clone()),
        )
        .await
        .expect("bind alternate host_b transport"),
    );
    let router: Arc<dyn A2APeerRouter> = transport.clone();
    let distributor = CohortDistributor::new(
        state_b,
        router,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host_b".into())),
            role: None,
        },
    );
    (transport, distributor)
}

async fn stand_up_live_cohort() -> LiveCohort {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2b-convergence");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let leaf_c = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[42u8; 32]);
    let pinned =
        PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).expect("authority pins");
    let fps = [
        leaf_a.fingerprint.wire(),
        leaf_b.fingerprint.wire(),
        leaf_c.fingerprint.wire(),
    ];
    let members = [
        ("host_a", fps[0].as_str()),
        ("host_b", fps[1].as_str()),
        ("host_c", fps[2].as_str()),
    ];

    let clock_a = Arc::new(TestCohortClock::new(0));
    let state_a = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host_a".into()),
            &signed_manifest(3, &authority, &members, T_STALE_SECS),
            pinned.clone(),
            Arc::new(InMemoryCohortAuditSink::default()),
            clock_a.clone(),
        )
        .expect("authority state at v3"),
    );
    let state_b = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host_b".into()),
            &signed_manifest(1, &authority, &members, T_STALE_SECS),
            pinned.clone(),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestCohortClock::new(0)),
        )
        .expect("member state at v1"),
    );
    let state_c = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host_c".into()),
            &signed_manifest(2, &authority, &members, T_STALE_SECS),
            pinned,
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestCohortClock::new(0)),
        )
        .expect("member state at v2"),
    );
    let longer_lease = signed_manifest(4, &authority, &members, 60);
    let removed_b_manifest = signed_manifest(
        5,
        &authority,
        &[("host_a", fps[0].as_str()), ("host_c", fps[2].as_str())],
        60,
    );

    let pems_a = write_pem(&leaf_a, Some(&ca));
    let transport_a = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_config(
                &pems_a,
                vec![
                    pin("host_b", &leaf_b.fingerprint, NONCE_B),
                    pin("host_c", &leaf_c.fingerprint, NONCE_C),
                ],
                Duration::from_secs(30),
            ),
            vec![
                peer_cfg(
                    "host_b",
                    "tls://127.0.0.1:0",
                    &leaf_b.fingerprint,
                    &["readonly"],
                    &["readonly"],
                ),
                peer_cfg(
                    "host_c",
                    "tls://127.0.0.1:0",
                    &leaf_c.fingerprint,
                    &["readonly"],
                    &["readonly"],
                ),
            ],
            NONCE_A,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_a.clone()),
        )
        .await
        .expect("bind authority transport"),
    );
    let addr_a = transport_a.local_addr().expect("authority address");

    // The rotation control is what makes the peer pin generation reachable from
    // the cohort state (AC2a). Installing it here is not test scaffolding: it is
    // exactly what `main.rs` does on the cohort-daemon arm.
    state_a
        .install_cert_rotation(
            transport_a.pins(),
            transport_a.core(),
            Arc::new(NeverFires),
            Duration::from_secs(5),
        )
        .expect("install the live rotation control");

    let mut transports = vec![transport_a.clone()];
    for (host, leaf, nonce, state) in [
        ("host_b", &leaf_b, NONCE_B, &state_b),
        ("host_c", &leaf_c, NONCE_C, &state_c),
    ] {
        let pems = write_pem(leaf, Some(&ca));
        let transport = Arc::new(
            maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
                tcp_config(
                    &pems,
                    vec![pin("host_a", &leaf_a.fingerprint, NONCE_A)],
                    Duration::from_secs(30),
                ),
                vec![peer_cfg(
                    "host_a",
                    &format!("tls://{addr_a}"),
                    &leaf_a.fingerprint,
                    &["readonly"],
                    &["readonly"],
                )],
                nonce,
                maos_a2a_tcp::TcpTimeouts::test_profile(),
                no_retry(),
                Some(clock.unix()),
                None,
                Some(state.clone()),
            )
            .await
            .expect("bind member transport"),
        );
        let addr = transport.local_addr().expect("member address");
        transport_a.set_peer_endpoint(&HostId(host.into()), format!("tls://{addr}"));

        let router: Arc<dyn A2APeerRouter> = transport.clone();
        CohortDistributor::new(
            (*state).clone(),
            router,
            FrameAddress {
                spirit_id: SpiritId::from("cohort-control"),
                host_id: Some(HostId(host.into())),
                role: None,
            },
        )
        .pull_from(&HostId("host_a".into()))
        .await
        .expect("reserved pull crosses the live wire");
        transports.push(transport);
    }
    let (restarted_transport, restarted_b) = bind_host_b_sender(
        &clock,
        &ca,
        &leaf_a,
        &leaf_b,
        addr_a,
        state_b.clone(),
        NONCE_B + 100,
    )
    .await;
    transports.push(restarted_transport);

    let router_a: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor_a = CohortDistributor::new(
        state_a.clone(),
        router_a,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host_a".into())),
            role: None,
        },
    );

    LiveCohort {
        clock_a,
        hash_b: hex::encode(state_b.canonical_hash().expect("host_b hash")),
        hash_c: hex::encode(state_c.canonical_hash().expect("host_c hash")),
        state_a,
        state_without_observer: state_b,
        restarted_b,
        longer_lease,
        removed_b_manifest,
        _transports: transports,
        distributor_a,
    }
}

fn row<'a>(rows: &'a [PeerConvergence], peer: &str) -> &'a PeerConvergence {
    rows.iter()
        .find(|row| row.peer == peer)
        .unwrap_or_else(|| panic!("no retained record for {peer}: {rows:?}"))
}
struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

fn operator_get(server: &OperatorHttpServer, token: &str) -> String {
    let mut stream = TcpStream::connect(server.local_addr()).expect("connect operator HTTP");
    write!(
        stream,
        "GET /v1/cohort/peer-versions HTTP/1.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    )
    .expect("write operator request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read operator response");
    response
}

fn pull_frame(peer: &str, target: &str, version: u64, hash: String) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-control"),
        host_id: Some(HostId(peer.into())),
        role: None,
    };
    let payload = CohortManifestControl::Pull {
        known_version: version,
        known_hash: hash,
    }
    .telemetry_payload()
    .expect("pull payload");
    IacFrame {
        frame_id: [9; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec::smallvec![FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId(target.into())),
            role: None,
        }],
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(TelemetryEventPayload {
            event_type: payload.event_type,
            data: payload.data,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(RESERVED_INTENT_REISSUE),
        )),
        intent_lineage: IntentLineage::default(),
    }
}

/// AC1 + AC3 + AC4 — two peers pulling at DIFFERENT signed versions are told
/// apart by version AND by canonical hash, on a live three-host mTLS mesh.
///
/// This is the non-vacuous assertion the story demands: an empty table, or a
/// table that records mere presence, fails it.
#[tokio::test]
#[ignore = "Story 14-2b — check-cohort-mesh owns the real three-host convergence sweep"]
async fn t_14_2b_peers_at_different_versions_are_told_apart() {
    let cohort = stand_up_live_cohort().await;

    assert!(
        CohortPeerVersions::new(cohort.state_without_observer.clone())
            .peer_manifest_versions()
            .is_none(),
        "a loaded manifest without the live daemon observer must render as 404, not healthy-empty"
    );
    let rows = cohort
        .state_a
        .peer_convergence()
        .expect("the convergence table is readable");
    assert_eq!(
        rows.len(),
        2,
        "exactly the two peers that pulled are retained, and the local host is not: {rows:?}"
    );

    let b = row(&rows, "host_b");
    let c = row(&rows, "host_c");
    assert_eq!(b.declared_version, 1, "{b:?}");
    assert_eq!(c.declared_version, 2, "{c:?}");
    assert_ne!(
        b.declared_version, c.declared_version,
        "the whole capability is TELLING TWO PEERS APART, not proving a row exists"
    );
    assert_eq!(
        b.declared_hash, cohort.hash_b,
        "the retained hash is the peer's own canonical hash, hex-encoded exactly as it sent it"
    );
    assert_eq!(c.declared_hash, cohort.hash_c, "{c:?}");
    assert_ne!(b.declared_hash, c.declared_hash, "{rows:?}");
    for record in [b, c] {
        assert_eq!(record.state, CONVERGENCE_OBSERVED, "{record:?}");
        assert!(record.is_valid(), "{record:?}");
        assert_eq!(
            record.observed_at_secs, 0,
            "the observation instant comes from the injected CohortClock, never SystemTime::now: \
             {record:?}"
        );
        assert_eq!(
            record.expires_at_secs, T_STALE_SECS,
            "each row captures the exact odd signed lease deadline: {record:?}"
        );
        assert_ne!(
            record.pin_generation, 0,
            "the pin generation was captured at the observation, so AC2a is evaluable: {record:?}"
        );
    }
    assert_eq!(b.pin_generation, NONCE_B, "{b:?}");
    assert_eq!(c.pin_generation, NONCE_C, "{c:?}");
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback("correct-token".into()),
        Arc::new(NoSandbox),
        None,
        Some(Arc::new(CohortPeerVersions::new(cohort.state_a.clone()))),
    )
    .expect("bind production convergence reader");
    let response = operator_get(&server, "correct-token");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .expect("operator response body");
    let parsed: serde_json::Value = serde_json::from_str(body).expect("operator JSON");
    let surface_rows = parsed["peer_versions"]
        .as_array()
        .expect("peer_versions array");
    assert_eq!(surface_rows.len(), 2, "{body}");
    assert_eq!(surface_rows[0]["peer"], "host_b");
    assert_eq!(surface_rows[0]["declared_version"], 1);
    assert_eq!(surface_rows[0]["declared_hash"], cohort.hash_b);
    assert_eq!(surface_rows[1]["peer"], "host_c");
    assert_eq!(surface_rows[1]["declared_version"], 2);
    assert_eq!(surface_rows[1]["declared_hash"], cohort.hash_c);

    // Retention did not displace the shipped behaviour it sits beside: both
    // pulls are still queued and still serviced.
    assert_eq!(
        cohort
            .distributor_a
            .service_pending_pulls()
            .await
            .expect("signed pushes"),
        2,
        "two verified pulls still produce two signed pushes"
    );
    let zero_nonce = pull_frame("host_b", "host_a", 99, "ff".repeat(32));
    assert_eq!(
        CohortManifestGate::apply_reissue(
            cohort.state_a.as_ref(),
            &HostId("host_b".into()),
            0,
            &zero_nonce,
        )
        .expect("zero-nonce pull remains serviceable"),
        CohortReissueDisposition::PullRequested,
    );
    let after_zero = cohort.state_a.peer_convergence().expect("readable");
    let b_after_zero = row(&after_zero, "host_b");
    assert_eq!(b_after_zero.declared_version, 1, "{b_after_zero:?}");
    assert_eq!(b_after_zero.observed_at_secs, 0, "{b_after_zero:?}");
}

/// AC2 + AC4 — a restarted peer's record is invalidated while its sibling's
/// stays valid, and the survivor then ages out past the derived bound.
///
/// The discrimination is the point. A mutation that drops either invalidation
/// leaves one row valid that must not be, and an empty table has no row to
/// leave standing at all.
#[tokio::test]
#[ignore = "Story 14-2b — check-cohort-mesh owns the real invalidation sweep"]
async fn t_14_2b_a_restarted_peer_is_not_a_convergence_claim() {
    let cohort = stand_up_live_cohort().await;
    let before = cohort.state_a.peer_convergence().expect("readable");
    assert!(
        row(&before, "host_b").is_valid() && row(&before, "host_c").is_valid(),
        "both records start as convergence claims, so the invalidations below are not vacuous: \
         {before:?}"
    );

    // AC2a — host_b restarts: the shipped detector compares the wire-carried
    // boot nonce against the pin and invalidates the pin
    // (`crates/maos-a2a-core/src/router.rs:1344-1380`). The version record must
    // stop being a claim on its own terms.
    let restart_error = cohort
        .restarted_b
        .pull_from(&HostId("host_a".into()))
        .await
        .expect_err("a changed wire nonce must trigger the live restart detector");
    assert!(
        restart_error.to_string().contains("distribution failed"),
        "the changed-nonce frame must receive a live-router NACK: {restart_error}"
    );

    let after_restart = cohort.state_a.peer_convergence().expect("readable");
    let b = row(&after_restart, "host_b");
    let c = row(&after_restart, "host_c");
    assert_eq!(
        b.state, CONVERGENCE_RESTARTED,
        "a record taken under a generation the peer no longer holds is not a convergence claim: \
         {b:?}"
    );
    assert!(!b.is_valid(), "{b:?}");
    assert_eq!(
        b.declared_version, 1,
        "the declaration is still REPORTED — an operator needs to see what was claimed and that \
         it is no longer trusted, not a silently vanished row: {b:?}"
    );
    assert_eq!(
        c.state, CONVERGENCE_OBSERVED,
        "one peer restarting must not invalidate another's record: {c:?}"
    );
    assert!(c.is_valid(), "{c:?}");

    // AC2b — expiry is the exact signed lease captured with the observation.
    // The odd value proves rounding in `confirmation_interval()` cannot extend
    // it by one second.
    let interval = cohort
        .state_a
        .confirmation_interval()
        .expect("the signed lease yields an interval")
        .as_secs();
    assert_eq!(
        interval,
        T_STALE_SECS.div_ceil(2),
        "the derivation is the shipped `(t_stale_secs + 1) / 2` of `confirmation_interval()`, \
         spelled as the equivalent `div_ceil` only because clippy rejects the literal form"
    );
    let bound = T_STALE_SECS;

    cohort.clock_a.set(bound);
    assert_eq!(
        row(
            &cohort.state_a.peer_convergence().expect("readable"),
            "host_c"
        )
        .state,
        CONVERGENCE_OBSERVED,
        "AT the bound the observation still stands — the boundary is inclusive, like `is_fresh`"
    );

    cohort.clock_a.set(bound + 1);
    let aged = cohort.state_a.peer_convergence().expect("readable");
    let c = row(&aged, "host_c");
    assert_eq!(
        c.state, CONVERGENCE_STALE,
        "past the derived bound the peer has missed a pull tick and the record is not a claim: \
         {c:?}"
    );
    assert!(!c.is_valid(), "{c:?}");
    assert_eq!(
        row(&aged, "host_b").state,
        CONVERGENCE_RESTARTED,
        "restart outranks staleness: an aged record from a restarted peer is still reported as \
         the restart, which is the actionable fact"
    );
    cohort
        .state_a
        .apply_reissue(&cohort.longer_lease)
        .expect("apply a later manifest with a longer lease");
    let after_extension = cohort.state_a.peer_convergence().expect("readable");
    assert_eq!(
        row(&after_extension, "host_c").state,
        CONVERGENCE_STALE,
        "a longer later lease must not revive an observation that already expired"
    );
    cohort
        .state_a
        .apply_reissue(&cohort.removed_b_manifest)
        .expect("apply a signed manifest that removes host_b");
    let after_removal = cohort.state_a.peer_convergence().expect("readable");
    assert!(
        after_removal.iter().all(|record| record.peer != "host_b"),
        "removed members are purged from the current-cohort surface: {after_removal:?}"
    );

    let removed_pull = pull_frame("host_b", "host_a", 99, "ff".repeat(32));
    assert_eq!(
        CohortManifestGate::apply_reissue(
            cohort.state_a.as_ref(),
            &HostId("host_b".into()),
            NONCE_B,
            &removed_pull,
        )
        .expect("boot-static removed peer still reaches the reserved Pull seam"),
        CohortReissueDisposition::PullRequested,
    );
    let after_refresh = cohort.state_a.peer_convergence().expect("readable");
    assert!(
        after_refresh.iter().all(|record| record.peer != "host_b"),
        "a removed peer cannot refresh itself back into the table: {after_refresh:?}"
    );
}
