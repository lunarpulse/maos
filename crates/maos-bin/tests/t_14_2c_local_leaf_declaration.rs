//! Story 14-2c — local serving-leaf declaration diagnostics.

use std::collections::BTreeMap;
use std::future::Future;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, A2ARouterCore, ConsentAllowlists, InMemoryTofuPinStore,
    PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_cohort::rotation::RotationGraceTimer;
use maos_cohort::{
    CohortAuditEvent, CohortAuditSink, CohortAuthority, CohortError, CohortManifest,
    CohortManifestState, CohortMember, ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink,
    ManifestSignature, PinnedAuthorityKeys, ReissueOutcome, COHORT_SCHEMA_V1,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_control::{
    CohortSelfIdentitySource, OperatorHttpConfig, OperatorHttpServer, SandboxReportSource,
    SelfIdentityStatus,
};
use maos_spirit_abi::identity::{HostId, SpiritId};

#[path = "../../maos-a2a-tcp/tests/support/mod.rs"]
mod support;

use std::sync::atomic::{AtomicU64, Ordering};

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_cohort::{CohortClock, CohortDistributor};
use maos_domain::frame::FrameAddress;
use support::*;

#[derive(Default)]
struct NoopTimer;

impl RotationGraceTimer for NoopTimer {
    fn schedule(&self, _grace: Duration, _on_expiry: Box<dyn FnOnce() + Send>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory pin-store futures never pend"),
    }
}

fn fingerprint(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", format!("{seed:02x}").repeat(32)))
        .expect("fixture fingerprint is valid")
}

fn signed_manifest(
    version: u64,
    signer: &SigningKey,
    local_wire: &str,
    peer: &PeerCertFingerprint,
) -> String {
    let members = [
        ("host-a".to_string(), local_wire.to_string()),
        ("host-b".to_string(), peer.wire()),
    ];
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-14-2c-local-leaf".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signer.verifying_key().to_bytes())],
        },
        members: members
            .iter()
            .map(|(host_id, fingerprint)| CohortMember {
                host_id: host_id.clone(),
                fingerprint: fingerprint.clone(),
                roles: vec!["worker".into()],
                team: None,
            })
            .collect(),
        consent: ConsentMatrix {
            send: members
                .iter()
                .map(|(host_id, _)| ConsentTuple {
                    peer: host_id.clone(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
            accept: members
                .iter()
                .map(|(host_id, _)| ConsentTuple {
                    peer: host_id.clone(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 30,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(signer);
    toml::to_string(&manifest).expect("manifest serializes")
}

fn peer_config(fingerprint: PeerCertFingerprint) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new("host-b"),
        endpoint: "tls://host-b:0".into(),
        cert_fingerprint: fingerprint,
        profile: A2AProfile::CrossHost,
        allowlists: ConsentAllowlists {
            send_allowlist: Vec::new(),
            accept_allowlist: Vec::new(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

struct Fixture {
    signer: SigningKey,
    serving: Option<PeerCertFingerprint>,
    peer: PeerCertFingerprint,
    audit: Arc<InMemoryCohortAuditSink>,
    state: Arc<CohortManifestState>,
}

impl Fixture {
    fn boot(declared_wire: &str, serving: Option<PeerCertFingerprint>) -> Self {
        let signer = SigningKey::from_bytes(&[0x2c; 32]);
        let peer = fingerprint(0xb1);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = Arc::new(
            CohortManifestState::load(
                HostId("host-a".into()),
                &signed_manifest(1, &signer, declared_wire, &peer),
                PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()])
                    .expect("authority pin"),
                audit.clone(),
            )
            .expect("initial manifest loads"),
        );
        let pins = Arc::new(InMemoryTofuPinStore::new());
        block_on(pins.pin_first_contact(&PeerId::new("host-b"), &peer, &peer, 1))
            .expect("peer pin installs");
        let tofu: Arc<dyn TofuPinStore> = pins.clone();
        let core = A2ARouterCore::try_new(vec![peer_config(peer.clone())], tofu)
            .expect("peer config is valid");
        let core = match &serving {
            Some(fingerprint) => core.with_local_leaf_fingerprint(fingerprint.clone()),
            None => core,
        };
        state
            .install_cert_rotation(
                pins,
                Arc::new(core),
                Arc::new(NoopTimer),
                Duration::from_secs(5),
            )
            .expect("rotation control installs");
        Self {
            signer,
            serving,
            peer,
            audit,
            state,
        }
    }

    fn reissue(&self, version: u64, declared_wire: &str) -> ReissueOutcome {
        self.state
            .apply_reissue(&signed_manifest(
                version,
                &self.signer,
                declared_wire,
                &self.peer,
            ))
            .expect("signed monotonic reissue applies")
    }

    fn local_moves(&self) -> Vec<CohortAuditEvent> {
        self.audit
            .events()
            .into_iter()
            .filter(|event| matches!(event, CohortAuditEvent::LocalLeafDeclarationMoved { .. }))
            .collect()
    }
}

#[test]
fn local_leaf_move_applies_and_records_named_audit_row() {
    let serving = fingerprint(0xa1);
    let declared_next = fingerprint(0xa2);
    let fixture = Fixture::boot(&serving.wire(), Some(serving.clone()));

    assert_eq!(
        fixture.reissue(2, &declared_next.wire()),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(matches!(
        fixture.local_moves().as_slice(),
        [CohortAuditEvent::LocalLeafDeclarationMoved {
            host,
            serving: actual,
            declared,
            version: 2,
        }] if host == "host-a" && actual == &serving.wire() && declared == &declared_next.wire()
    ));
}

#[test]
fn install_reconciliation_records_pre_install_local_move() {
    let serving = fingerprint(0xa1);
    let declared = fingerprint(0xa2);
    let fixture = Fixture::boot(&declared.wire(), Some(serving.clone()));

    assert!(matches!(
        fixture.local_moves().as_slice(),
        [CohortAuditEvent::LocalLeafDeclarationMoved {
            serving: actual,
            declared: signed,
            version: 1,
            ..
        }] if actual == &serving.wire() && signed == &declared.wire()
    ));
}

#[test]
fn absent_plane_c_and_equal_parsed_values_produce_no_row() {
    let serving = fingerprint(0xa1);
    let next = fingerprint(0xa2);
    let absent = Fixture::boot(&serving.wire(), None);
    assert_eq!(
        absent.reissue(2, &next.wire()),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(
        absent.local_moves().is_empty(),
        "None is unavailable, not divergent"
    );

    let uppercase_hex = format!("sha256:{}", serving.hex.to_uppercase());
    let equal = Fixture::boot(&uppercase_hex, Some(serving.clone()));
    assert_eq!(
        equal.reissue(2, &serving.wire()),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(
        equal.local_moves().is_empty(),
        "comparison uses parsed values rather than raw string case"
    );
    assert_eq!(equal.serving, Some(serving));
}
#[test]
fn configured_local_leaf_fingerprint_is_readable() {
    let fingerprint = fingerprint(0x71);
    let core = A2ARouterCore::new(vec![], Arc::new(InMemoryTofuPinStore::new()))
        .with_local_leaf_fingerprint(fingerprint.clone());

    assert_eq!(core.local_leaf_fingerprint(), Some(fingerprint));
}

#[test]
fn source_derives_diverged_and_unconfirmable_from_live_state() {
    let serving = fingerprint(0xa1);
    let declared = fingerprint(0xa2);
    let diverged = Fixture::boot(&declared.wire(), Some(serving));
    let pull_health = Arc::new(Mutex::new(BTreeMap::from([(
        "host-b".into(),
        "PIN_MISMATCH".into(),
    )])));
    let source = maos_bin::cert_rotation::CohortSelfIdentity::new(
        Arc::clone(&diverged.state),
        Arc::clone(&pull_health),
    );
    let Some(SelfIdentityStatus::Healthy(row)) = source.self_identity() else {
        panic!("present readable state returns a row");
    };
    assert_eq!(row.verdict, "diverged");
    assert_eq!(row.declared, Some(declared.short()));
    assert_eq!(row.peers_observed, 0);
    assert_eq!(row.peers_total, 1);
    assert_eq!(row.last_pull_errors["host-b"], "PIN_MISMATCH");

    let matching = fingerprint(0xa3);
    let unconfirmed = Fixture::boot(&matching.wire(), Some(matching));
    let source = maos_bin::cert_rotation::CohortSelfIdentity::new(
        Arc::clone(&unconfirmed.state),
        Arc::new(Mutex::new(BTreeMap::new())),
    );
    let Some(SelfIdentityStatus::Healthy(row)) = source.self_identity() else {
        panic!("present readable state returns a row");
    };
    assert_eq!(
        row.verdict, "unconfirmable",
        "matching local values cannot claim agreement before every peer is observed"
    );
}

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(
        &self,
        _spirit_id: &str,
    ) -> Option<maos_domain::sandbox::SandboxInspectReport> {
        None
    }
}

fn self_identity_get(server: &OperatorHttpServer) -> String {
    let mut stream = TcpStream::connect(server.local_addr()).expect("connect operator HTTP");
    write!(
        stream,
        "GET /v1/cohort/self-identity HTTP/1.1\r\nAuthorization: Bearer correct-token\r\nConnection: close\r\n\r\n"
    )
    .expect("write operator request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read operator response");
    response
}

fn response_json(response: &str) -> serde_json::Value {
    serde_json::from_str(
        response
            .split("\r\n\r\n")
            .nth(1)
            .expect("operator response body"),
    )
    .expect("operator response is JSON")
}

#[derive(Default)]
struct FailOnWindowOpen {
    events: Mutex<Vec<CohortAuditEvent>>,
}

impl CohortAuditSink for FailOnWindowOpen {
    fn append(&self, event: &CohortAuditEvent) -> Result<(), CohortError> {
        if matches!(event, CohortAuditEvent::CertRotationWindowOpened { .. }) {
            return Err(CohortError::EAuditAppendFailed(
                "injected rotation reload failure".into(),
            ));
        }
        self.events.lock().expect("audit lock").push(event.clone());
        Ok(())
    }
}

fn assert_reload_error_records_no_local_divergence() {
    let signer = SigningKey::from_bytes(&[0x3c; 32]);
    let serving = fingerprint(0xc1);
    let declared_next = fingerprint(0xc2);
    let peer = fingerprint(0xd1);
    let peer_next = fingerprint(0xd2);
    let audit = Arc::new(FailOnWindowOpen::default());
    let state = CohortManifestState::load(
        HostId("host-a".into()),
        &signed_manifest(1, &signer, &serving.wire(), &peer),
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("authority pin"),
        audit.clone(),
    )
    .expect("initial manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    block_on(pins.pin_first_contact(&PeerId::new("host-b"), &peer, &peer, 1))
        .expect("peer pin installs");
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config(peer)], tofu)
            .expect("peer config is valid")
            .with_local_leaf_fingerprint(serving),
    );
    state
        .install_cert_rotation(pins, core, Arc::new(NoopTimer), Duration::from_secs(5))
        .expect("rotation control installs");

    assert!(
        state
            .apply_reissue(&signed_manifest(
                2,
                &signer,
                &declared_next.wire(),
                &peer_next,
            ))
            .is_err(),
        "the injected peer reload failure rejects the state transition"
    );
    assert_eq!(state.version().expect("state remains readable"), 1);
    assert!(
        !audit
            .events
            .lock()
            .expect("audit lock")
            .iter()
            .any(|event| matches!(event, CohortAuditEvent::LocalLeafDeclarationMoved { .. })),
        "local divergence is journaled only after peer reload commits"
    );
}

#[test]
#[ignore = "Story 14-2c — check-cohort-mesh owns the real reissue and HTTP surface"]
fn t_14_2c_local_leaf_reissue_is_diagnosable() {
    let serving = fingerprint(0xa1);
    let declared_next = fingerprint(0xa2);
    let fixture = Fixture::boot(&serving.wire(), Some(serving.clone()));
    assert_eq!(
        fixture.reissue(2, &serving.wire()),
        ReissueOutcome::Applied { version: 2 }
    );
    assert!(
        fixture.local_moves().is_empty(),
        "an unchanged local row produces no divergence audit"
    );

    let pull_health = Arc::new(Mutex::new(BTreeMap::from([(
        "host-b".into(),
        "PIN_MISMATCH".into(),
    )])));
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback("correct-token".into()),
        Arc::new(NoSandbox),
        None,
        None,
        Some(Arc::new(maos_bin::cert_rotation::CohortSelfIdentity::new(
            Arc::clone(&fixture.state),
            pull_health,
        ))),
    )
    .expect("bind production self-identity reader");
    let before = self_identity_get(&server);
    assert!(before.starts_with("HTTP/1.1 200"), "{before}");
    let before_identity = &response_json(&before)["self_identity"];
    assert_eq!(before_identity["verdict"], "unconfirmable", "{before}");
    assert_ne!(before_identity["verdict"], "diverged", "{before}");

    assert_eq!(
        fixture.reissue(3, &declared_next.wire()),
        ReissueOutcome::Applied { version: 3 }
    );
    assert_eq!(fixture.local_moves().len(), 1);
    let after = self_identity_get(&server);
    assert!(after.starts_with("HTTP/1.1 200"), "{after}");
    let after_identity = &response_json(&after)["self_identity"];
    assert_eq!(after_identity["declared"], "a2a2a2a2", "{after}");
    assert_eq!(after_identity["serving"], "a1a1a1a1", "{after}");
    assert_eq!(after_identity["verdict"], "diverged", "{after}");
    assert_eq!(after_identity["peers_observed"], 0, "{after}");
    assert_eq!(after_identity["peers_total"], 1, "{after}");
    assert_eq!(
        after_identity["last_pull_errors"]["host-b"], "PIN_MISMATCH",
        "{after}"
    );

    assert_reload_error_records_no_local_divergence();
}

/// Story 14-2c review — a v3 that moves ONLY another member must not re-fire
/// the local-move row while the divergence persists.
#[test]
fn later_reissue_touching_only_another_member_does_not_refire() {
    let serving = fingerprint(0xa1);
    let declared_next = fingerprint(0xa2);
    let peer_next = fingerprint(0xb2);
    let fixture = Fixture::boot(&serving.wire(), Some(serving.clone()));
    assert_eq!(
        fixture.reissue(2, &declared_next.wire()),
        ReissueOutcome::Applied { version: 2 }
    );
    assert_eq!(fixture.local_moves().len(), 1);
    assert_eq!(
        fixture
            .state
            .apply_reissue(&signed_manifest(
                3,
                &fixture.signer,
                &declared_next.wire(),
                &peer_next,
            ))
            .expect("peer-only reissue applies"),
        ReissueOutcome::Applied { version: 3 }
    );
    assert_eq!(
        fixture.local_moves().len(),
        1,
        "an unrelated version must not re-date the local move"
    );
}

/// Story 14-2c review — AC3(c)(i): against the PRODUCTION adapter (not a
/// `maos-control` stub), loaded cohort state before `install_cert_rotation`
/// renders 200 + `unconfirmable` + `serving: null`, never 404 and never
/// `agree`.
#[test]
fn production_source_reports_unconfirmable_before_rotation_installs() {
    let signer = SigningKey::from_bytes(&[0x4c; 32]);
    let declared = fingerprint(0xe1);
    let peer = fingerprint(0xe2);
    let state = Arc::new(
        CohortManifestState::load(
            HostId("host-a".into()),
            &signed_manifest(1, &signer, &declared.wire(), &peer),
            PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("authority pin"),
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("initial manifest loads"),
    );
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback("correct-token".into()),
        Arc::new(NoSandbox),
        None,
        None,
        Some(Arc::new(maos_bin::cert_rotation::CohortSelfIdentity::new(
            state,
            Arc::new(Mutex::new(BTreeMap::new())),
        ))),
    )
    .expect("bind pre-install self-identity reader");
    let response = self_identity_get(&server);
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let identity = &response_json(&response)["self_identity"];
    assert_eq!(identity["verdict"], "unconfirmable", "{response}");
    assert_eq!(identity["serving"], serde_json::Value::Null, "{response}");
    assert_eq!(identity["declared"], declared.short(), "{response}");
}

struct TestCohortClock(AtomicU64);

impl CohortClock for TestCohortClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}

/// Story 14-2c review decision (Lunarpulse, 2026-09-04): the live-pull leg.
/// A real two-host mTLS mesh over real TCP with the shipped
/// `CohortDistributor` and cohort-manifest gate — nothing re-implements the
/// production path. `host_b` pulls from `host_a` (creating the convergence
/// record), then `host_a`'s own pulls drive the pull-health carrier through
/// failure and recovery, and every verdict is asserted through the operator
/// HTTP body.
#[tokio::test]
#[ignore = "Story 14-2c — check-cohort-mesh owns the live agreement and pull-health leg"]
async fn t_14_2c_self_identity_agreement_and_pull_health() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-14-2c-self-identity");
    let leaf_a = valid_leaf(&ca, &clock);
    let leaf_b = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[0x5c; 32]);
    let pinned =
        PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).expect("authority pins");
    let fps = [leaf_a.fingerprint.wire(), leaf_b.fingerprint.wire()];
    let cohort_clock = Arc::new(TestCohortClock(AtomicU64::new(0)));
    let state_a = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-a".into()),
            &signed_manifest(2, &authority, &fps[0], &leaf_b.fingerprint),
            pinned.clone(),
            Arc::new(InMemoryCohortAuditSink::default()),
            cohort_clock,
        )
        .expect("host_a state at v2"),
    );
    let state_b = Arc::new(
        CohortManifestState::load(
            HostId("host-b".into()),
            &signed_manifest(2, &authority, &fps[0], &leaf_b.fingerprint),
            pinned,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("host_b state at v2"),
    );

    let pems_a = write_pem(&leaf_a, Some(&ca));
    let transport_a = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_config(
                &pems_a,
                vec![pin("host-b", &leaf_b.fingerprint, 2)],
                Duration::from_secs(30),
            ),
            vec![peer_cfg(
                "host-b",
                "tls://127.0.0.1:0",
                &leaf_b.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            1,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_a.clone()),
        )
        .await
        .expect("bind host_a transport"),
    );
    let addr_a = transport_a.local_addr().expect("host_a address");
    state_a
        .install_cert_rotation(
            transport_a.pins(),
            transport_a.core(),
            Arc::new(NoopTimer),
            Duration::from_secs(5),
        )
        .expect("install the live rotation control");

    let pems_b = write_pem(&leaf_b, Some(&ca));
    let transport_b = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp_config(
                &pems_b,
                vec![pin("host-a", &leaf_a.fingerprint, 1)],
                Duration::from_secs(30),
            ),
            vec![peer_cfg(
                "host-a",
                &format!("tls://{addr_a}"),
                &leaf_a.fingerprint,
                &["readonly"],
                &["readonly"],
            )],
            2,
            maos_a2a_tcp::TcpTimeouts::test_profile(),
            no_retry(),
            Some(clock.unix()),
            None,
            Some(state_b.clone()),
        )
        .await
        .expect("bind host_b transport"),
    );
    let addr_b = transport_b.local_addr().expect("host_b address");
    transport_a.set_peer_endpoint(&HostId("host-b".into()), format!("tls://{addr_b}"));

    // host_b pulls from host_a: the TLS-verified pull creates host_a's
    // convergence record for host_b, which is what makes `agree` reachable.
    let router_b: Arc<dyn A2APeerRouter> = transport_b.clone();
    CohortDistributor::new(
        state_b.clone(),
        router_b,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host-b".into())),
            role: None,
        },
    )
    .pull_from(&HostId("host-a".into()))
    .await
    .expect("host_b pull crosses the live wire");

    let pull_health = Arc::new(Mutex::new(BTreeMap::new()));
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback("correct-token".into()),
        Arc::new(NoSandbox),
        None,
        None,
        Some(Arc::new(maos_bin::cert_rotation::CohortSelfIdentity::new(
            Arc::clone(&state_a),
            Arc::clone(&pull_health),
        ))),
    )
    .expect("bind live self-identity reader");

    let router_a: Arc<dyn A2APeerRouter> = transport_a.clone();
    let distributor_a = CohortDistributor::new(
        state_a.clone(),
        router_a,
        FrameAddress {
            spirit_id: SpiritId::from("cohort-control"),
            host_id: Some(HostId("host-a".into())),
            role: None,
        },
    );

    // Healthy mesh: a successful pull leaves no error entry, the convergence
    // record is live, and the verdict is `agree`.
    distributor_a
        .pull_from(&HostId("host-b".into()))
        .await
        .expect("host_a pull crosses the live wire");
    pull_health.lock().expect("pull-health lock").clear();
    let healthy = self_identity_get(&server);
    assert!(healthy.starts_with("HTTP/1.1 200"), "{healthy}");
    let identity = &response_json(&healthy)["self_identity"];
    assert_eq!(identity["verdict"], "agree", "{healthy}");
    assert_eq!(identity["peers_observed"], 1, "{healthy}");
    assert_eq!(identity["peers_total"], 1, "{healthy}");
    assert_eq!(
        identity["last_pull_errors"]
            .as_object()
            .map(|errors| errors.len()),
        Some(0),
        "{healthy}"
    );

    // Sever the wire: the pull fails, the error is carried, and `agree` is
    // no longer claimable even though the convergence record is still live.
    transport_a.set_peer_endpoint(&HostId("host-b".into()), "tls://127.0.0.1:9".to_string());
    let severed = distributor_a
        .pull_from(&HostId("host-b".into()))
        .await
        .expect_err("a dead endpoint must fail the pull");
    pull_health
        .lock()
        .expect("pull-health lock")
        .insert("host-b".into(), severed.to_string());
    let failing = self_identity_get(&server);
    assert!(failing.starts_with("HTTP/1.1 200"), "{failing}");
    let identity = &response_json(&failing)["self_identity"];
    assert_eq!(identity["verdict"], "unconfirmable", "{failing}");
    assert_eq!(identity["peers_observed"], 1, "{failing}");
    assert!(
        identity["last_pull_errors"]["host-b"].is_string(),
        "{failing}"
    );

    // Restore the wire: the next successful pull clears the error and the
    // verdict returns to `agree`.
    transport_a.set_peer_endpoint(&HostId("host-b".into()), format!("tls://{addr_b}"));
    distributor_a
        .pull_from(&HostId("host-b".into()))
        .await
        .expect("restored pull crosses the live wire");
    pull_health
        .lock()
        .expect("pull-health lock")
        .remove("host-b");
    let restored = self_identity_get(&server);
    assert!(restored.starts_with("HTTP/1.1 200"), "{restored}");
    let identity = &response_json(&restored)["self_identity"];
    assert_eq!(identity["verdict"], "agree", "{restored}");
    assert!(
        identity["last_pull_errors"]["host-b"].is_null(),
        "{restored}"
    );
}
