//! Story 12.2 — per-(peer,role) consent and manifest-skew live-wire corpus.

mod support;

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use futures_util::{SinkExt, StreamExt};
use maos_a2a_core::router::A2ATransport;
use maos_a2a_core::transport::json_rpc::{A2AJsonRpcRequest, CODE_INTENT_DENIED};
use maos_a2a_core::{A2AJsonRpcResponse, CohortManifestGate};
use maos_cohort::{
    CohortAuthority, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;
use tokio_util::bytes::Bytes;

const INTENT: &str = "readonly";
const RECEIVER_VERSION: u64 = 4;
const SENDER_NONCE: u64 = 1;
const RECEIVER_NONCE: u64 = 2;

fn consent_manifest(
    authority: &SigningKey,
    sender_fingerprint: &maos_a2a_core::PeerCertFingerprint,
    receiver_fingerprint: &maos_a2a_core::PeerCertFingerprint,
) -> CohortManifest {
    CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-12-2-live-consent".into(),
        version: RECEIVER_VERSION,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: sender_fingerprint.wire(),
                roles: vec!["architect".into(), "reviewer".into()],
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: receiver_fingerprint.wire(),
                roles: vec!["receiver".into()],
            },
        ],
        consent: ConsentMatrix {
            send: vec![ConsentTuple {
                peer: "host_b".into(),
                role: "receiver".into(),
                intent: INTENT.into(),
            }],
            accept: vec![
                ConsentTuple {
                    peer: "host_a".into(),
                    role: "architect".into(),
                    intent: INTENT.into(),
                },
                ConsentTuple {
                    peer: "host_b".into(),
                    role: "receiver".into(),
                    intent: INTENT.into(),
                },
            ],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
    }
    .signed_with(authority)
}

async fn live_receiver(
    clock: &Clock,
    ca: &Ca,
    sender: &Leaf,
    receiver: &Leaf,
    manifest: &CohortManifest,
    authority: &SigningKey,
) -> maos_a2a_tcp::TcpA2ATransport {
    let manifest_toml = toml::to_string(manifest).expect("signed manifest serializes");
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    let state = Arc::new(
        CohortManifestState::load(
            HostId("host_b".into()),
            &manifest_toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("receiver loads signed manifest"),
    );
    let gate: Arc<dyn CohortManifestGate> = state;
    let pems = write_pem(receiver, Some(ca));
    let config = tcp_config(
        &pems,
        vec![pin("host_a", &sender.fingerprint, SENDER_NONCE)],
        std::time::Duration::from_secs(30),
    );
    maos_a2a_tcp::TcpA2ATransport::bind_with_cohort_manifest_gate(
        config,
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &sender.fingerprint,
            &[INTENT],
            &[INTENT],
        )],
        RECEIVER_NONCE,
        maos_a2a_tcp::TcpTimeouts::test_profile(),
        no_retry(),
        Some(clock.unix()),
        None,
        Some(gate),
    )
    .await
    .expect("bind live cohort receiver")
}

async fn send_recv(
    framed: &mut tokio_util::codec::Framed<
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        tokio_util::codec::LengthDelimitedCodec,
    >,
    request: &A2AJsonRpcRequest,
) -> A2AJsonRpcResponse {
    framed
        .send(Bytes::from(serde_json::to_vec(request).unwrap()))
        .await
        .expect("send request");
    let bytes = framed
        .next()
        .await
        .expect("response frame")
        .expect("response codec");
    serde_json::from_slice(&bytes).expect("decode response")
}

fn request(role: &str, version: u64, sequence: u64) -> A2AJsonRpcRequest {
    A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_a", "host_b", IntentClass::Readonly, sequence),
        sequence,
    )
    .with_boot_nonce(SENDER_NONCE)
    .with_cohort_acting_role(role)
    .with_cohort_manifest_version(version)
}

fn manifest_accepts(manifest: &CohortManifest, role: &str, version: u64) -> bool {
    version.abs_diff(manifest.version) <= 1
        && manifest
            .members
            .iter()
            .find(|member| member.host_id == "host_a")
            .is_some_and(|member| member.roles.iter().any(|declared| declared == role))
        && manifest
            .consent
            .accept
            .iter()
            .any(|grant| grant.peer == "host_a" && grant.role == role && grant.intent == INTENT)
}

type LiveClient = tokio_util::codec::Framed<
    tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
    tokio_util::codec::LengthDelimitedCodec,
>;

async fn live_fixture(
    seed: u8,
    ca_name: &str,
) -> (CohortManifest, maos_a2a_tcp::TcpA2ATransport, LiveClient) {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, ca_name);
    let sender = valid_leaf(&ca, &clock);
    let receiver = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[seed; 32]);
    let manifest = consent_manifest(&authority, &sender.fingerprint, &receiver.fingerprint);
    let transport = live_receiver(&clock, &ca, &sender, &receiver, &manifest, &authority).await;
    let address = transport.local_addr().expect("receiver address");
    let framed =
        raw_client_connect(address, &sender, &receiver.fingerprint, Some(&ca), &clock).await;
    (manifest, transport, framed)
}

#[tokio::test]
#[ignore = "Story 12.2 — role identity exact-match over real TCP/mTLS"]
async fn t_12_2_role_mismatch_on_allowed_peer_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-2-role");
    let sender = valid_leaf(&ca, &clock);
    let receiver = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[52; 32]);
    let manifest = consent_manifest(&authority, &sender.fingerprint, &receiver.fingerprint);
    let transport = live_receiver(&clock, &ca, &sender, &receiver, &manifest, &authority).await;
    let address = transport.local_addr().expect("receiver address");
    let mut framed =
        raw_client_connect(address, &sender, &receiver.fingerprint, Some(&ca), &clock).await;

    let permitted_role = "architect";
    assert!(manifest_accepts(
        &manifest,
        permitted_role,
        RECEIVER_VERSION
    ));
    assert!(matches!(
        send_recv(&mut framed, &request(permitted_role, RECEIVER_VERSION, 1)).await,
        A2AJsonRpcResponse::Ack(_)
    ));

    let relabeled_role = "reviewer";
    assert!(!manifest_accepts(
        &manifest,
        relabeled_role,
        RECEIVER_VERSION
    ));
    match send_recv(&mut framed, &request(relabeled_role, RECEIVER_VERSION, 2)).await {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED);
            assert_eq!(
                nack.error
                    .data
                    .as_ref()
                    .and_then(|data| data["reason"].as_str()),
                Some("no_grant")
            );
        }
        other => panic!("relabeled declared role must NACK, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "Story 12.2 — manifest-skew distinct cause over real TCP/mTLS"]
async fn t_12_2_manifest_skew_cause_live_tcp() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-2-skew");
    let sender = valid_leaf(&ca, &clock);
    let receiver = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[53; 32]);
    let manifest = consent_manifest(&authority, &sender.fingerprint, &receiver.fingerprint);
    let transport = live_receiver(&clock, &ca, &sender, &receiver, &manifest, &authority).await;
    let address = transport.local_addr().expect("receiver address");
    let mut framed =
        raw_client_connect(address, &sender, &receiver.fingerprint, Some(&ca), &clock).await;

    let within_one = RECEIVER_VERSION - 1;
    assert!(manifest_accepts(&manifest, "architect", within_one));
    assert!(matches!(
        send_recv(&mut framed, &request("architect", within_one, 3)).await,
        A2AJsonRpcResponse::Ack(_)
    ));

    let stale = RECEIVER_VERSION - 2;
    assert!(!manifest_accepts(&manifest, "architect", stale));
    match send_recv(&mut framed, &request("architect", stale, 4)).await {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED);
            let data = nack.error.data.expect("skew data");
            assert_eq!(data["reason"], "cohort_manifest_skew");
            assert_eq!(data["sender_version"], stale);
            assert_eq!(data["receiver_version"], RECEIVER_VERSION);
            assert_eq!(data["delta"], 2);
        }
        other => panic!("manifest skew must NACK distinctly, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "Story 12.2 — manifest-derived acting role over an N-host live mesh"]
async fn t_12_2_acting_role_exact_match_live_mesh() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-12-2-live-mesh");
    let sender = valid_leaf(&ca, &clock);
    let receiver = valid_leaf(&ca, &clock);
    let authority = SigningKey::from_bytes(&[54; 32]);
    let manifest = consent_manifest(&authority, &sender.fingerprint, &receiver.fingerprint);
    let manifest_toml = toml::to_string(&manifest).unwrap();
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    let state_a = Arc::new(
        CohortManifestState::load(
            HostId("host_a".into()),
            &manifest_toml,
            pins.clone(),
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap(),
    );
    let state_b = Arc::new(
        CohortManifestState::load(
            HostId("host_b".into()),
            &manifest_toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap(),
    );
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = vec![Some(state_a), Some(state_b)];
    let names = vec!["host_a".to_string(), "host_b".to_string()];
    let leaves = [&sender, &receiver];
    let mesh =
        build_mesh_n_with_gates(&clock, &ca, &names, &leaves, &leaves, no_retry(), &gates).await;
    let results = concurrent_dial_pairs(&mesh, &[(0, 1)], 5, IntentClass::Readonly).await;

    assert_eq!(results.len(), 1, "one directed live-mesh leg was requested");
    assert!(
        results[0].2.is_ok(),
        "manifest-derived architect role must satisfy host_b's exact accept tuple: {:?}",
        results[0].2
    );
}

#[tokio::test]
#[ignore = "Story 12.2 — positive plus relabeled-negative exact role over live TCP"]
async fn t_12_2_acting_role_exact_match_live_tcp() {
    let (manifest, _transport, mut framed) = live_fixture(55, "ca-12-2-exact").await;
    let version = manifest.version;
    let positive = request("architect", version, 10);
    let mut relabeled = positive.clone();
    relabeled.id = 11;
    relabeled.params.frame_id[7] = 11;
    relabeled.cohort_acting_role = Some("reviewer".into());

    assert!(manifest_accepts(&manifest, "architect", version));
    assert!(matches!(
        send_recv(&mut framed, &positive).await,
        A2AJsonRpcResponse::Ack(_)
    ));
    assert!(!manifest_accepts(&manifest, "reviewer", version));
    assert!(matches!(
        send_recv(&mut framed, &relabeled).await,
        A2AJsonRpcResponse::Nack(_)
    ));
}

#[tokio::test]
#[ignore = "Story 12.2 — undeclared acting-role entitlement over live TCP"]
async fn t_12_2_entitlement_accept_live_tcp() {
    let (manifest, _transport, mut framed) = live_fixture(56, "ca-12-2-entitlement").await;
    assert!(!manifest_accepts(&manifest, "operator", manifest.version));
    match send_recv(&mut framed, &request("operator", manifest.version, 12)).await {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED);
            assert_eq!(nack.error.data.unwrap()["reason"], "role_not_entitled");
        }
        other => panic!("unheld acting role must NACK, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "Story 12.2 — absent role/version fail closed over live TCP"]
async fn t_12_2_fail_closed_none_live_tcp() {
    let (manifest, _transport, mut framed) = live_fixture(57, "ca-12-2-none").await;
    let missing_role = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_a", "host_b", IntentClass::Readonly, 13),
        13,
    )
    .with_boot_nonce(SENDER_NONCE)
    .with_cohort_manifest_version(manifest.version);
    let missing_version = A2AJsonRpcRequest::new(
        "iac.deliver",
        make_frame("host_a", "host_b", IntentClass::Readonly, 14),
        14,
    )
    .with_boot_nonce(SENDER_NONCE)
    .with_cohort_acting_role("architect");

    for (request, reason) in [
        (missing_role, "acting_role_absent"),
        (missing_version, "manifest_version_absent"),
    ] {
        match send_recv(&mut framed, &request).await {
            A2AJsonRpcResponse::Nack(nack) => {
                assert_eq!(nack.error.code, CODE_INTENT_DENIED);
                assert_eq!(nack.error.data.unwrap()["reason"], reason);
            }
            other => panic!("missing cohort field must NACK, got {other:?}"),
        }
    }
}

#[tokio::test]
#[ignore = "Story 12.2 — reserved intent bypasses absent role/version over live TCP"]
async fn t_12_2_reserved_intent_without_role_or_version_live_tcp() {
    let (_manifest, _transport, mut framed) = live_fixture(58, "ca-12-2-reserved").await;
    let mut frame = make_frame("host_a", "host_b", IntentClass::Readonly, 15);
    frame
        .consent_envelope
        .as_mut()
        .expect("classified frame")
        .intent_class = Some(maos_domain::invariants::i8::A2AIntent::new(
        RESERVED_INTENT_HALT_RECEIPT,
    ));
    let request = A2AJsonRpcRequest::new("iac.deliver", frame, 15).with_boot_nonce(SENDER_NONCE);
    assert!(matches!(
        send_recv(&mut framed, &request).await,
        A2AJsonRpcResponse::Ack(_)
    ));
}
