//! Shared hermetic test harness for the live TCP/mTLS transport (Story 8.6
//! H1–H6). Everything here is generated at test setup — NO dated `.pem`/`.crt`/
//! `.key` is committed (H1 guard `git ls-files` yields zero).
//!
//! * H1 — `mk_*` issue time-relative certs via `rcgen` at setup, offset from a
//!   single `T0` captured once per test (`Clock::capture`).
//! * H2 — the SAME pinned `T0` feeds rustls cert-validity (via the verifier's
//!   `validation_time`) and any rotation offset.
//! * H3 — listeners bind `127.0.0.1:0`; tests dial the `local_addr()` readback.
//! * H4 — readiness via `local_addr()` (the bind future completes before the
//!   address is observable), NOT a sleep.
//! * H5 — `TcpTimeouts::test_profile()` ≤ 250ms.
//! * H6 — `TcpA2ATransport`'s `ServeGuard` aborts the accept loop + conns on drop.

#![allow(dead_code)]

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::{A2APeerConfig, A2AProfile, ConsentAllowlists};
use maos_a2a_core::{HandshakeRetryPolicy, InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_tcp::{
    build_client_config, length_delimited_codec, PinnedFingerprint, TcpA2AConfig, TcpA2ATransport,
    TcpTimeouts, TrustPosture,
};
use maos_domain::invariants::i8::A2AIntent;
use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::Framed;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use time::OffsetDateTime;

/// H2 — a single pinned clock captured once per test.
#[derive(Clone, Copy)]
pub struct Clock {
    pub t0: SystemTime,
}

impl Clock {
    pub fn capture() -> Self {
        Self { t0: SystemTime::now() }
    }

    /// The rustls validation time (T0) fed to the verifier (H2).
    pub fn unix(&self) -> UnixTime {
        UnixTime::since_unix_epoch(self.t0.duration_since(UNIX_EPOCH).expect("t0 after epoch"))
    }

    /// An `OffsetDateTime` `secs` away from T0 (negative = before).
    pub fn offset(&self, secs: i64) -> OffsetDateTime {
        OffsetDateTime::from(self.t0) + time::Duration::seconds(secs)
    }
}

pub const HOUR: i64 = 3600;

/// A single-attempt retry policy (no retries) for tests that aren't exercising
/// AC-T5's retry path — keeps the non-retry security tests fast.
pub fn no_retry() -> HandshakeRetryPolicy {
    HandshakeRetryPolicy {
        backoff_ms: vec![],
        jitter_pct: 0,
        max_attempts: 1,
    }
}

/// A CA root + its key (chain mode, `ca_roots = Some`).
pub struct Ca {
    pub cert: rcgen::Certificate,
    pub key: KeyPair,
    pub der: CertificateDer<'static>,
}

/// Issue a self-signed CA root valid in a wide window around T0.
pub fn mk_ca(clock: &Clock, common_name: &str) -> Ca {
    let key = KeyPair::generate().expect("ca keypair");
    let mut params = CertificateParams::new(vec![common_name.to_string()]).expect("ca params");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.not_before = clock.offset(-10 * HOUR);
    params.not_after = clock.offset(10 * HOUR);
    let cert = params.self_signed(&key).expect("ca self-signed");
    let der = cert.der().clone();
    Ca { cert, key, der }
}

/// A leaf cert + its key + computed pin fingerprint + PEM material.
pub struct Leaf {
    pub der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub fingerprint: PeerCertFingerprint,
    pub cert_pem: String,
    pub key_pem: String,
}

fn leaf_params(clock: &Clock, before_off: i64, after_off: i64) -> (CertificateParams, KeyPair) {
    let key = KeyPair::generate().expect("leaf keypair");
    // "127.0.0.1" registers an IP SAN; the verifier ignores the name (pin is the
    // identity), but rcgen needs at least one SAN.
    let mut params = CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("leaf params");
    params.not_before = clock.offset(before_off);
    params.not_after = clock.offset(after_off);
    (params, key)
}

/// A leaf signed by `ca` (chain-mode corpus). Offsets are relative to T0.
pub fn mk_leaf_signed_by(ca: &Ca, clock: &Clock, before_off: i64, after_off: i64) -> Leaf {
    let (params, key) = leaf_params(clock, before_off, after_off);
    let cert = params
        .signed_by(&key, &ca.cert, &ca.key)
        .expect("leaf signed_by ca");
    finish_leaf(cert, key)
}

/// A self-signed leaf (pin-only `ca_roots = None` corpus). Offsets relative to T0.
pub fn mk_self_signed(clock: &Clock, before_off: i64, after_off: i64) -> Leaf {
    let (params, key) = leaf_params(clock, before_off, after_off);
    let cert = params.self_signed(&key).expect("self-signed leaf");
    finish_leaf(cert, key)
}

fn finish_leaf(cert: rcgen::Certificate, key: KeyPair) -> Leaf {
    let der = cert.der().clone();
    let fingerprint = PeerCertFingerprint::from_cert_der(der.as_ref());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    Leaf {
        der,
        key_der,
        fingerprint,
        cert_pem: cert.pem(),
        key_pem: key.serialize_pem(),
    }
}

/// The verifier trust posture for a `ca` (Some ⇒ chain mode; None ⇒ pin-only).
pub fn posture(ca: Option<&Ca>) -> TrustPosture {
    match ca {
        Some(c) => TrustPosture::ChainToRoots(Arc::new(vec![c.der.clone()])),
        None => TrustPosture::LeafSelfAnchor,
    }
}

/// Establish a RAW authenticated mTLS client connection to `nash_addr` using
/// `mira_leaf` as the client identity and pinning `nash_fp`. Returns the
/// length-delimited framed stream so liveness/DoS tests (AC-T7/T8) can send
/// crafted bytes over a genuinely authenticated channel.
pub async fn raw_client_stream(
    nash_addr: SocketAddr,
    mira_leaf: &Leaf,
    nash_fp: &PeerCertFingerprint,
    ca: Option<&Ca>,
    clock: &Clock,
) -> TlsStream<TcpStream> {
    let pins = Arc::new(InMemoryTofuPinStore::new());
    pins.pin_first_contact(&PeerId::new("host_b"), nash_fp, nash_fp, 0)
        .await
        .expect("pin nash");
    let cfg = build_client_config(
        &[mira_leaf.der.clone()],
        &mira_leaf.key_der,
        pins,
        posture(ca),
        None, // raw test client: unscoped (server-side intake is under test)
        Some(clock.unix()),
    )
    .expect("client config");
    let connector = TlsConnector::from(Arc::new(cfg));
    let tcp = TcpStream::connect(nash_addr).await.expect("tcp connect");
    let server_name = ServerName::IpAddress(nash_addr.ip().into());
    connector
        .connect(server_name, tcp)
        .await
        .expect("tls handshake")
}

/// Raw authenticated mTLS client, wrapped in the length-delimited codec.
pub async fn raw_client_connect(
    nash_addr: SocketAddr,
    mira_leaf: &Leaf,
    nash_fp: &PeerCertFingerprint,
    ca: Option<&Ca>,
    clock: &Clock,
) -> Framed<TlsStream<TcpStream>, tokio_util::codec::LengthDelimitedCodec> {
    let tls = raw_client_stream(nash_addr, mira_leaf, nash_fp, ca, clock).await;
    Framed::new(tls, length_delimited_codec())
}

/// A `valid` leaf (T0−1h .. T0+1h) issued by `ca`.
pub fn valid_leaf(ca: &Ca, clock: &Clock) -> Leaf {
    mk_leaf_signed_by(ca, clock, -HOUR, HOUR)
}

/// An `expired` leaf (T0−2h .. T0−1h) issued by `ca`.
pub fn expired_leaf(ca: &Ca, clock: &Clock) -> Leaf {
    mk_leaf_signed_by(ca, clock, -2 * HOUR, -HOUR)
}

/// A `not_yet_valid` leaf (T0+1h .. T0+2h) issued by `ca`.
pub fn not_yet_valid_leaf(ca: &Ca, clock: &Clock) -> Leaf {
    mk_leaf_signed_by(ca, clock, HOUR, 2 * HOUR)
}

/// On-disk PEM material for one endpoint identity, kept alive by the `TempDir`.
pub struct PemFiles {
    pub dir: TempDir,
    pub cert_chain: PathBuf,
    pub private_key: PathBuf,
    pub ca_roots: Option<PathBuf>,
}

/// Write `leaf` (and optionally a `ca` root bundle) to a fresh temp dir.
pub fn write_pem(leaf: &Leaf, ca: Option<&Ca>) -> PemFiles {
    let dir = tempfile::tempdir().expect("tempdir");
    let cert_chain = dir.path().join("cert.pem");
    let private_key = dir.path().join("key.pem");
    std::fs::write(&cert_chain, &leaf.cert_pem).expect("write cert");
    std::fs::write(&private_key, &leaf.key_pem).expect("write key");
    let ca_roots = ca.map(|c| {
        let p = dir.path().join("ca.pem");
        std::fs::write(&p, c.cert.pem()).expect("write ca");
        p
    });
    PemFiles {
        dir,
        cert_chain,
        private_key,
        ca_roots,
    }
}

/// Build an `A2APeerConfig` (core allowlists + dial endpoint).
pub fn peer_cfg(
    peer_id: &str,
    endpoint: &str,
    fp: &PeerCertFingerprint,
    send: &[&str],
    accept: &[&str],
) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(peer_id),
        endpoint: endpoint.to_string(),
        cert_fingerprint: fp.clone(),
        profile: A2AProfile::CrossHost,
        allowlists: ConsentAllowlists {
            send_allowlist: send.iter().map(|s| A2AIntent::new(*s)).collect(),
            accept_allowlist: accept.iter().map(|s| A2AIntent::new(*s)).collect(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

/// Build a `TcpA2AConfig` for an endpoint binding `127.0.0.1:0` (H3).
pub fn tcp_config(
    pems: &PemFiles,
    peer_pins: Vec<PinnedFingerprint>,
    handshake_timeout: Duration,
) -> TcpA2AConfig {
    TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
        own_cert_chain: pems.cert_chain.clone(),
        own_private_key: pems.private_key.clone(),
        peer_pins,
        handshake_timeout,
        ca_roots: pems.ca_roots.clone(),
    }
}

/// Poll `cond` every 5ms until it returns true or `budget` elapses. Returns
/// whether it became true (H4 — no fixed sleeps; bounded busy-wait on a gauge).
pub async fn wait_until<F: Fn() -> bool>(cond: F, budget: Duration) -> bool {
    let deadline = std::time::Instant::now() + budget;
    while std::time::Instant::now() < deadline {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    cond()
}

/// A pre-paired pin record for `peer_id` → `fp`.
pub fn pin(peer_id: &str, fp: &PeerCertFingerprint, boot_nonce: u64) -> PinnedFingerprint {
    PinnedFingerprint {
        peer_id: PeerId::new(peer_id),
        fingerprint: fp.clone(),
        boot_nonce,
    }
}

/// Bind one endpoint. All PEM material is read during `bind`, so the temp dir
/// is dropped on return (H1: nothing persists). `validation_time` is pinned to
/// `clock` (H2). `ca = Some` ⇒ chain mode; `None` ⇒ pin-only.
pub async fn bind_endpoint(
    own_leaf: &Leaf,
    ca: Option<&Ca>,
    own_boot_nonce: u64,
    peer_pins: Vec<PinnedFingerprint>,
    peer_configs: Vec<A2APeerConfig>,
    clock: &Clock,
    timeouts: TcpTimeouts,
    retry: HandshakeRetryPolicy,
) -> TcpA2ATransport {
    let pems = write_pem(own_leaf, ca);
    let cfg = tcp_config(&pems, peer_pins, Duration::from_secs(30));
    TcpA2ATransport::bind(
        cfg,
        peer_configs,
        own_boot_nonce,
        timeouts,
        retry,
        Some(clock.unix()),
        None, // consent expiry: real wall clock unless a test pins it
    )
    .await
    .expect("bind endpoint")
}

/// Configuration for [`bind_endpoint_consent_pinned`].
pub struct BindEndpointConfig<'a> {
    pub own_leaf: &'a Leaf,
    pub ca: Option<&'a Ca>,
    pub own_boot_nonce: u64,
    pub peer_pins: Vec<PinnedFingerprint>,
    pub peer_configs: Vec<A2APeerConfig>,
    pub clock: &'a Clock,
    pub timeouts: TcpTimeouts,
    pub retry: HandshakeRetryPolicy,
    pub consent_now_ns: u64,
}

/// Story 8.9 / AC3 — like [`bind_endpoint`] but pins the shared router's
/// consent-expiry clock to `consent_now_ns` so on-wire consent-expiry tests are
/// deterministic (the sender stamps `valid_until = consent_now + ttl`; a
/// receiver pinned past that rejects with `CODE_CONSENT_EXPIRED`).
pub async fn bind_endpoint_consent_pinned(cfg: BindEndpointConfig<'_>) -> TcpA2ATransport {
    let pems = write_pem(cfg.own_leaf, cfg.ca);
    let tcp = tcp_config(&pems, cfg.peer_pins, Duration::from_secs(30));
    TcpA2ATransport::bind(
        tcp,
        cfg.peer_configs,
        cfg.own_boot_nonce,
        cfg.timeouts,
        cfg.retry,
        Some(cfg.clock.unix()),
        Some(cfg.consent_now_ns),
    )
    .await
    .expect("bind endpoint (consent-pinned)")
}

/// Build a `CrossHost` advisory frame from `from_host` → `to_host` with the
/// given intent class and a unique `frame_id` (avoids router-cache collisions).
pub fn make_frame(
    from_host: &str,
    to_host: &str,
    intent: maos_domain::invariants::i1::IntentClass,
    seq: u64,
) -> maos_domain::frame::IacFrame {
    use maos_domain::frame::{FrameAddress, FramePayload, PosturePreferences, TaskAssignPayload};
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
    use smallvec::smallvec;

    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_be_bytes());
    maos_domain::frame::IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from("mira"),
            host_id: Some(HostId(from_host.to_string())),
            role: None,
        },
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: Some(HostId(to_host.to_string())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "diagnostic advisory".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}
