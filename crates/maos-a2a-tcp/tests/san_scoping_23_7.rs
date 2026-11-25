//! Story 23-7 / AC5 — listen-side SAN scoping (D-23-7-04 A, -05 A, -06 A).
//!
//! Real sockets, real mTLS, `rcgen` leaves carrying SPIFFE URI SANs. Under
//! `peer_san = "required"` the listen side verifies a client leaf against the pin record of the
//! peer its SAN NAMES: the SAN selects the record, the fingerprint decides. Under `off` the flat
//! lookup is byte for byte what it was.

mod support;

use std::sync::Arc;
use std::time::Duration;

use maos_a2a_core::identity::PeerId;
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{ConsentRuptureSink, HandshakeRetryPolicy};
use maos_a2a_tcp::{PeerSanMode, TcpA2AConfig, TcpA2ATransport, TcpTimeouts, TcpTransportError};
use maos_domain::frame::{FramePayload, IacFrame, RuptureReason};
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::{FrameKind, HostId};
use parking_lot::Mutex;
use support::*;

const DOMAIN: &str = "corp.example";
const X_NONCE: u64 = 9;
const CLIENT_NONCE: u64 = 3;

fn spiffe(domain: &str, peer: &str) -> String {
    format!("spiffe://{domain}/maos/peer/{peer}")
}

fn san_leaf(ca: &Ca, clock: &Clock, uris: &[String]) -> Leaf {
    mk_san_leaf(ca, clock, uris)
}

#[derive(Default)]
struct Journal {
    frames: Mutex<Vec<IacFrame>>,
}

impl Journal {
    fn reasons(&self) -> Vec<RuptureReason> {
        self.frames
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
        self.frames.lock().push(frame.clone());
        Ok(())
    }
}

fn retry() -> HandshakeRetryPolicy {
    no_retry()
}

/// Listener X under `mode`: pins `pins` (peer → leaf), serves `x_leaf`.
async fn bind_x(
    clock: &Clock,
    ca: &Ca,
    x_leaf: &Leaf,
    mode: PeerSanMode,
    pins: &[(&str, &Leaf)],
) -> (TcpA2ATransport, Arc<Journal>) {
    let pems = write_pem(x_leaf, Some(ca));
    let mut cfg: TcpA2AConfig = tcp_config(
        &pems,
        pins.iter()
            .map(|(peer, leaf)| pin(peer, &leaf.fingerprint, CLIENT_NONCE))
            .collect(),
        Duration::from_secs(30),
    );
    cfg.peer_san = mode;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let configs = pins
        .iter()
        .map(|(peer, leaf)| {
            peer_cfg(
                peer,
                "tls://127.0.0.1:0",
                &leaf.fingerprint,
                &[],
                &["readonly"],
            )
        })
        .collect();
    let x = TcpA2ATransport::bind(
        cfg,
        configs,
        X_NONCE,
        TcpTimeouts::test_profile(),
        retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("X binds");
    let journal = Arc::new(Journal::default());
    x.core().install_rupture_sink(journal.clone()).await;
    (x, journal)
}

/// A client endpoint presenting `leaf`, named `name` on the wire, dialing X.
async fn dial_x(
    clock: &Clock,
    ca: &Ca,
    x: &TcpA2ATransport,
    x_leaf: &Leaf,
    leaf: &Leaf,
    name: &str,
) -> Result<(), maos_a2a_core::A2AError> {
    let addr = x.local_addr().unwrap();
    let client = bind_endpoint(
        leaf,
        Some(ca),
        CLIENT_NONCE,
        vec![pin("host_x", &x_leaf.fingerprint, X_NONCE)],
        vec![peer_cfg(
            "host_x",
            &format!("tls://{addr}"),
            &x_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        clock,
        retry(),
    )
    .await;
    client
        .route_outbound(
            make_frame(name, "host_x", IntentClass::Readonly, 1),
            &HostId("host_x".into()),
        )
        .await
}

#[tokio::test]
async fn required_scopes_the_listen_side_to_the_san_named_peer() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san");
    let x_leaf = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let p = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let p_next = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    // Q is pinned with a leaf whose SAN names P: the CA (or a careless operator) issued it so.
    let q_wearing_p = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let no_san = san_leaf(&ca, &clock, &[]);
    let doubled = san_leaf(
        &ca,
        &clock,
        &[spiffe(DOMAIN, "host_d"), spiffe(DOMAIN, "host_d2")],
    );
    let foreign = san_leaf(&ca, &clock, &[spiffe("other.example", "host_f")]);

    let (x, journal) = bind_x(
        &clock,
        &ca,
        &x_leaf,
        PeerSanMode::Required,
        &[
            ("host_p", &p),
            ("host_q", &q_wearing_p),
            ("host_n", &no_san),
            ("host_d", &doubled),
            ("host_f", &foreign),
        ],
    )
    .await;

    // A client whose SAN names P and whose leaf is P's pin is accepted.
    dial_x(&clock, &ca, &x, &x_leaf, &p, "host_p")
        .await
        .expect("P with its own pinned leaf");
    assert!(
        journal.reasons().is_empty(),
        "an accepted peer journals nothing"
    );

    // ... and so is P's OPEN next generation.
    x.pins()
        .open_rotation_window(&PeerId::new("host_p"), &p_next.fingerprint)
        .expect("window on P");
    dial_x(&clock, &ca, &x, &x_leaf, &p_next, "host_p")
        .await
        .expect("P's open next leaf");
    assert!(journal.reasons().is_empty());

    // Each refusal is typed and journaled; none reaches intake.
    let entered = x.intake_entered();
    let mut refusals = 0;
    for (leaf, name, what) in [
        (&q_wearing_p, "host_q", "P's SAN with Q's pinned leaf"),
        (&no_san, "host_n", "a pinned leaf with no URI SAN"),
        (&doubled, "host_d", "two URI SANs"),
        (&foreign, "host_f", "a foreign trust domain"),
    ] {
        let result = dial_x(&clock, &ca, &x, &x_leaf, leaf, name).await;
        assert!(result.is_err(), "{what} must be refused");
        refusals += 1;
        assert!(
            wait_until(
                || journal.reasons().len() >= refusals,
                Duration::from_secs(3)
            )
            .await,
            "{what}: the refusal must be journaled"
        );
    }
    assert!(journal
        .reasons()
        .iter()
        .all(|reason| *reason == RuptureReason::PeerIdentityUnverified));
    assert_eq!(
        x.intake_entered(),
        entered,
        "no refused client reaches intake"
    );
}

/// Control: under `off` the same Q-wearing-P's-SAN leaf is accepted, so the refusal above IS the
/// scoping and not some other check.
#[tokio::test]
async fn off_keeps_the_flat_listen_side() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san-off");
    let x_leaf = san_leaf(&ca, &clock, &[]);
    let q_wearing_p = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let (x, journal) = bind_x(
        &clock,
        &ca,
        &x_leaf,
        PeerSanMode::Off,
        &[("host_q", &q_wearing_p)],
    )
    .await;
    dial_x(&clock, &ca, &x, &x_leaf, &q_wearing_p, "host_q")
        .await
        .expect("flat lookup accepts any active pin");
    assert!(journal.reasons().is_empty());
}

#[tokio::test]
async fn boot_under_required_refuses_what_a_required_peer_would_refuse() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san-boot");
    let peer = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let bind = |own: Leaf, pins: Vec<(String, Leaf)>, domain: Option<String>| {
        let ca_ref = &ca;
        async move {
            let pems = write_pem(&own, Some(ca_ref));
            let mut cfg = tcp_config(
                &pems,
                pins.iter()
                    .map(|(peer, leaf)| pin(peer, &leaf.fingerprint, CLIENT_NONCE))
                    .collect(),
                Duration::from_secs(30),
            );
            cfg.peer_san = PeerSanMode::Required;
            cfg.peer_trust_domain = domain;
            TcpA2ATransport::bind(
                cfg,
                vec![],
                X_NONCE,
                TcpTimeouts::test_profile(),
                retry(),
                Some(clock.unix()),
                None,
            )
            .await
            .err()
        }
    };

    // An own leaf without its SAN: typed refusal.
    let no_san = san_leaf(&ca, &clock, &[]);
    let error = bind(
        no_san,
        vec![("host_p".into(), peer.clone())],
        Some(DOMAIN.into()),
    )
    .await
    .expect("boot must refuse");
    assert!(
        matches!(error, TcpTransportError::OwnLeafSanRefused { .. }),
        "{error}"
    );

    // A configured peer_id outside [a-zA-Z0-9._-]: refused at config load.
    let own = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let error = bind(
        own.clone(),
        vec![("host p".into(), peer.clone())],
        Some(DOMAIN.into()),
    )
    .await
    .expect("boot must refuse");
    assert!(matches!(error, TcpTransportError::Config(_)), "{error}");

    // `required` without a trust domain: refused.
    let error = bind(own, vec![], None).await.expect("boot must refuse");
    assert!(matches!(error, TcpTransportError::Config(_)), "{error}");
}

/// AC2/AC5 — an own-leaf swap to a leaf lacking its SAN is a typed refusal, and the leaf stays.
#[tokio::test]
async fn a_swap_to_a_leaf_without_the_san_is_refused_under_required() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san-swap");
    let x_old = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let x_no_san = san_leaf(&ca, &clock, &[]);
    let x_new = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let pems = write_pem(&x_old, Some(&ca));
    let mut cfg = tcp_config(&pems, vec![], Duration::from_secs(30));
    cfg.peer_san = PeerSanMode::Required;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let x = TcpA2ATransport::bind(
        cfg,
        vec![],
        X_NONCE,
        TcpTimeouts::test_profile(),
        retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("X binds");
    assert!(x.bind_own_peer(PeerId::new("host_x")));

    std::fs::write(&pems.cert_chain, &x_no_san.cert_pem).unwrap();
    std::fs::write(&pems.private_key, &x_no_san.key_pem).unwrap();
    let error = x
        .reload_own_identity(&x_no_san.fingerprint)
        .expect_err("a SAN-less own leaf is refused");
    assert!(
        matches!(error, TcpTransportError::OwnLeafSanRefused { .. }),
        "{error}"
    );
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_old.fingerprint.clone())
    );

    std::fs::write(&pems.cert_chain, &x_new.cert_pem).unwrap();
    std::fs::write(&pems.private_key, &x_new.key_pem).unwrap();
    let swapped = x.reload_own_identity(&x_new.fingerprint).expect("swap");
    assert!(matches!(
        swapped,
        maos_a2a_tcp::OwnLeafReload::Swapped { .. }
    ));
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_new.fingerprint.clone())
    );
}

/// Review B3 — an own leaf that names ANOTHER in-domain host is refused at swap (it would pass a
/// "names some peer" check, and every `required` peer would scope our handshake to that host), on
/// the declared-leaf path AND on the public `swap_serving_cert`; an unbound own peer id refuses
/// too. The served leaf never changes.
#[tokio::test]
async fn an_own_leaf_naming_another_host_is_refused_by_every_swap_path() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san-host");
    let x_old = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let names_y = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_y")]);
    let no_san = san_leaf(&ca, &clock, &[]);
    let x_new = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let pems = write_pem(&x_old, Some(&ca));
    let mut cfg = tcp_config(&pems, vec![], Duration::from_secs(30));
    cfg.peer_san = PeerSanMode::Required;
    cfg.peer_trust_domain = Some(DOMAIN.to_string());
    let x = TcpA2ATransport::bind(
        cfg,
        vec![],
        X_NONCE,
        TcpTimeouts::test_profile(),
        retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("X binds");

    // Before the own peer id is bound, no swap can be tied to this host: refused, even a good leaf.
    let error = x
        .swap_serving_cert(vec![x_new.der.clone()], x_new.key_der.clone_key())
        .expect_err("unbound own peer id");
    assert!(
        matches!(error, TcpTransportError::OwnIdentityRefused(_)),
        "{error}"
    );
    assert!(x.bind_own_peer(PeerId::new("host_x")));
    assert!(!x.bind_own_peer(PeerId::new("host_z")), "set once");

    std::fs::write(&pems.cert_chain, &names_y.cert_pem).unwrap();
    std::fs::write(&pems.private_key, &names_y.key_pem).unwrap();
    let error = x
        .reload_own_identity(&names_y.fingerprint)
        .expect_err("a leaf naming host_y is not this host's");
    assert!(
        matches!(&error, TcpTransportError::OwnLeafSanNamesOtherHost { named, expected }
            if named == "host_y" && expected == "host_x"),
        "{error}"
    );
    let error = x
        .swap_serving_cert(vec![names_y.der.clone()], names_y.key_der.clone_key())
        .expect_err("the public swap enforces the host SAN too");
    assert!(
        matches!(error, TcpTransportError::OwnLeafSanNamesOtherHost { .. }),
        "{error}"
    );
    let error = x
        .swap_serving_cert(vec![no_san.der.clone()], no_san.key_der.clone_key())
        .expect_err("the public swap enforces a SAN at all");
    assert!(
        matches!(error, TcpTransportError::OwnLeafSanRefused { .. }),
        "{error}"
    );
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_old.fingerprint.clone()),
        "nothing was served"
    );

    // The right host is accepted by the public swap.
    x.swap_serving_cert(vec![x_new.der.clone()], x_new.key_der.clone_key())
        .expect("this host's own leaf swaps");
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_new.fingerprint.clone())
    );
}

/// Review B3 — a PEM read that catches the leaf and its matching key BEFORE the intermediate is
/// written must not install an incomplete chain (every peer would refuse it, and plane C would
/// then match the declaration so the reconciler would stop looking). The refusal is a typed error
/// with the old identity untouched, and the completed PEM swaps on the next attempt.
#[tokio::test]
async fn an_incomplete_chain_is_refused_and_the_completed_pem_heals_on_the_next_try() {
    let clock = Clock::capture();
    let root = mk_ca(&clock, "ca-chain-root");
    let inter = mk_intermediate(&root, &clock, "ca-chain-intermediate");
    let x_old = mk_san_leaf(&root, &clock, &[]);
    let x_new = mk_san_leaf(&inter, &clock, &[]);
    let pems = write_pem(&x_old, Some(&root));
    let x = TcpA2ATransport::bind(
        tcp_config(&pems, vec![], Duration::from_secs(30)),
        vec![],
        X_NONCE,
        TcpTimeouts::test_profile(),
        retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("X binds");

    // The PKI has written the leaf and its key; the intermediate is not in the chain file yet.
    std::fs::write(&pems.cert_chain, &x_new.cert_pem).unwrap();
    std::fs::write(&pems.private_key, &x_new.key_pem).unwrap();
    let error = x
        .reload_own_identity(&x_new.fingerprint)
        .expect_err("a leaf without its intermediate does not chain to the roots");
    assert!(
        matches!(error, TcpTransportError::OwnIdentityRefused(_)),
        "{error}"
    );
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_old.fingerprint.clone()),
        "the incomplete chain was not installed"
    );

    // The chain file is completed: the next try swaps.
    std::fs::write(
        &pems.cert_chain,
        format!("{}{}", x_new.cert_pem, inter.cert.pem()),
    )
    .unwrap();
    let swapped = x.reload_own_identity(&x_new.fingerprint).expect("healed");
    assert!(matches!(
        swapped,
        maos_a2a_tcp::OwnLeafReload::Swapped { .. }
    ));
    assert_eq!(
        x.core().local_leaf_fingerprint(),
        Some(x_new.fingerprint.clone())
    );
}

/// Review B3 — a leaf valid for only ONE of the two TLS usages is refused: this host both serves
/// (`serverAuth`) and dials (`clientAuth`) with the same identity.
#[tokio::test]
async fn a_leaf_missing_either_tls_usage_is_refused() {
    use rcgen::ExtendedKeyUsagePurpose::{ClientAuth, ServerAuth};
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-eku");
    let x_old = mk_san_leaf(&ca, &clock, &[]);
    let server_only = mk_leaf_eku(&ca, &clock, vec![ServerAuth]);
    let client_only = mk_leaf_eku(&ca, &clock, vec![ClientAuth]);
    let both = mk_leaf_eku(&ca, &clock, vec![ServerAuth, ClientAuth]);
    let pems = write_pem(&x_old, Some(&ca));
    let x = TcpA2ATransport::bind(
        tcp_config(&pems, vec![], Duration::from_secs(30)),
        vec![],
        X_NONCE,
        TcpTimeouts::test_profile(),
        retry(),
        Some(clock.unix()),
        None,
    )
    .await
    .expect("X binds");
    for (name, leaf) in [
        ("serverAuth only", &server_only),
        ("clientAuth only", &client_only),
    ] {
        let error = x
            .swap_serving_cert(vec![leaf.der.clone()], leaf.key_der.clone_key())
            .expect_err(name);
        assert!(
            matches!(error, TcpTransportError::OwnIdentityRefused(_)),
            "{name}: {error}"
        );
    }
    x.swap_serving_cert(vec![both.der.clone()], both.key_der.clone_key())
        .expect("both usages");
}

/// A server-cert verifier for the raw client below: it accepts X (the test is about X's view of
/// the CLIENT), and checks handshake signatures the ordinary way.
#[derive(Debug)]
struct AcceptAnyServer(rustls::crypto::WebPkiSupportedAlgorithms);

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServer {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0)
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.supported_schemes()
    }
}

/// One connection by a rustls client that HAS session resumption enabled (the library default),
/// then one malformed frame so the session tickets the server sent (if any) are processed.
/// Returns how the handshake completed.
async fn resuming_connect(
    config: &Arc<rustls::ClientConfig>,
    addr: std::net::SocketAddr,
) -> Option<rustls::HandshakeKind> {
    use futures_util::{SinkExt, StreamExt};
    let tcp = tokio::net::TcpStream::connect(addr).await.expect("tcp");
    let tls = tokio_rustls::TlsConnector::from(config.clone())
        .connect(
            rustls::pki_types::ServerName::IpAddress(addr.ip().into()),
            tcp,
        )
        .await
        .expect("TLS 1.3 client handshakes complete before the server judges the client leaf");
    let kind = tls.get_ref().1.handshake_kind();
    let mut framed = tokio_util::codec::Framed::new(tls, maos_a2a_tcp::length_delimited_codec());
    let _ = framed
        .send(tokio_util::bytes::Bytes::from_static(b"{}"))
        .await;
    // A reply, an error or EOF: any of them means the server is done with this connection.
    let _ = framed.next().await;
    kind
}

/// Review B4 — the server keeps NO session resumption, so a connection can never be bound to the
/// identity a CACHED handshake once presented. P's leaf is admitted; its fingerprint is then
/// re-assigned to Q (P rotated to another leaf, Q was re-paired to P's old leaf); a resuming client
/// presenting P's old leaf again must run the full verifier, which refuses it (the SAN names P, P's
/// pin is the new leaf). Planted red: re-enabling the session cache and tickets resumes the second
/// connection without `verify_client_cert`, and the flat lookup binds it to Q.
#[tokio::test]
async fn a_resumed_connection_cannot_bind_a_reassigned_fingerprint() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-san-resume");
    let x_leaf = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_x")]);
    let p_old = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let p_next = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_p")]);
    let q = san_leaf(&ca, &clock, &[spiffe(DOMAIN, "host_q")]);
    let (x, journal) = bind_x(
        &clock,
        &ca,
        &x_leaf,
        PeerSanMode::Required,
        &[("host_p", &p_old), ("host_q", &q)],
    )
    .await;
    let addr = x.local_addr().unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let client = Arc::new(
        rustls::ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyServer(
                provider.signature_verification_algorithms,
            )))
            .with_client_auth_cert(vec![p_old.der.clone()], p_old.key_der.clone_key())
            .unwrap(),
    );

    // 1. P's leaf is admitted by a full handshake; nothing is journaled.
    assert_eq!(
        resuming_connect(&client, addr).await,
        Some(rustls::HandshakeKind::Full)
    );
    assert!(journal.reasons().is_empty(), "{:?}", journal.reasons());

    // 2. Re-assign P's old fingerprint to Q: P promotes to its next leaf, then Q is re-paired onto
    // the freed fingerprint.
    let pins = x.pins();
    pins.open_rotation_window(&PeerId::new("host_p"), &p_next.fingerprint)
        .expect("P's window");
    pins.close_rotation_window(&PeerId::new("host_p"))
        .expect("P promotes");
    pins.open_rotation_window(&PeerId::new("host_q"), &p_old.fingerprint)
        .expect("the freed fingerprint is no longer P's");
    pins.close_rotation_window(&PeerId::new("host_q"))
        .expect("Q promotes onto it");
    assert_eq!(
        pins.find_active_pin_by_fingerprint(&p_old.fingerprint),
        Some(PeerId::new("host_q")),
        "the flat lookup now resolves P's old leaf to Q"
    );

    // 3. The same resuming client comes back with P's old leaf: no resumption, so the verifier
    // judges it and refuses (journaled typed refusal), and nothing reaches intake.
    let entered = x.intake_entered();
    assert_ne!(
        resuming_connect(&client, addr).await,
        Some(rustls::HandshakeKind::Resumed),
        "the server must not resume a session"
    );
    assert!(
        wait_until(|| !journal.reasons().is_empty(), Duration::from_secs(3)).await,
        "the re-presented leaf must be refused by the verifier"
    );
    assert_eq!(
        x.intake_entered(),
        entered,
        "the refused client never reaches intake"
    );
}
