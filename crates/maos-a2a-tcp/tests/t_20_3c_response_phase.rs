//! Story 20-3c AC11 (member 14) — the RESPONSE phase of a dial is a typed
//! `PartitionTimeout` carrying the frame id, not a generic `TransportFailed`.
//!
//! `t_2c_fault_windows.rs` stalls the handshake (a plain TCP peer that never
//! speaks TLS); nothing exercised `transport.rs` `"awaiting response"`. This
//! peer completes the mutual-TLS handshake, READS the request frame, and never
//! answers.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use maos_a2a_core::identity::PeerId;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{A2AError, InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_tcp::{build_server_config, length_delimited_codec};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;

#[tokio::test]
async fn a_peer_that_reads_the_request_and_never_answers_is_a_typed_response_partition() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-20-3c-response");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    // The silent peer: real mTLS server config pinning Mira, one frame read,
    // then the connection is held open without a reply.
    let pins = Arc::new(InMemoryTofuPinStore::new());
    pins.pin_first_contact(
        &PeerId::new("host_a"),
        &mira_leaf.fingerprint,
        &mira_leaf.fingerprint,
        0,
    )
    .await
    .expect("pin mira");
    let (server_config, _resolver) = build_server_config(
        std::slice::from_ref(&nash_leaf.der),
        &nash_leaf.key_der,
        pins,
        posture(Some(&ca)),
        Some(clock.unix()),
    )
    .expect("server config");
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&requests);
    let _held = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(tls) = acceptor.accept(socket).await else {
                continue;
            };
            let mut framed = Framed::new(tls, length_delimited_codec());
            if framed.next().await.is_some() {
                seen.fetch_add(1, Ordering::SeqCst);
            }
            held.push(framed); // never reply
        }
    });

    let mira = bind_endpoint(
        &mira_leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )],
        &clock,
        no_retry(),
    )
    .await;

    let frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    let expected_id = frame.frame_id;
    let started = std::time::Instant::now();
    let err = mira
        .route_outbound(frame, &HostId("host_b".into()))
        .await
        .expect_err("a peer that never answers must not report delivery");

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the response phase is bounded by the partition window, took {:?}",
        started.elapsed()
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        1,
        "the peer read the request: this is the RESPONSE phase, not the handshake or the send"
    );
    match err {
        A2AError::PartitionTimeout { peer, frame_id, .. } => {
            assert_eq!(peer, "host_b");
            assert_eq!(frame_id, expected_id, "the partition names the frame");
        }
        other => panic!("the response phase must be a typed PartitionTimeout, got {other:?}"),
    }
}
