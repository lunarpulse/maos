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
use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{
    A2AError, A2APeerConfig, A2AProfile, CohortManifestGate, ConsentAllowlists, DigestReadPort,
    HaltReceiptObserver,
};
use maos_a2a_core::{HandshakeRetryPolicy, InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_tcp::{
    build_client_config, length_delimited_codec, PinnedFingerprint, TcpA2AConfig, TcpA2ATransport,
    TcpTimeouts, TrustPosture,
};
use maos_cohort::CohortRuptureLogSink;
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::TransparencyLogAdapter;
use maos_spirit_abi::identity::HostId;
use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::Framed;

/// H2 — a single pinned clock captured once per test.
#[derive(Clone, Copy)]
pub struct Clock {
    pub t0: SystemTime,
}

impl Clock {
    pub fn capture() -> Self {
        Self {
            t0: SystemTime::now(),
        }
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
///
/// `Clone` is manual because `PrivateKeyDer` deliberately does not implement
/// `Clone` (secret key semantics) — pki-types' `clone_key()` is the explicit,
/// auditable copy. Needed by the planted-mesh builders, which must place the
/// SAME claimed leaf in both the `serving` and `expected` slots.
pub struct Leaf {
    pub der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub fingerprint: PeerCertFingerprint,
    pub cert_pem: String,
    pub key_pem: String,
}

impl Clone for Leaf {
    fn clone(&self) -> Self {
        Self {
            der: self.der.clone(),
            key_der: self.key_der.clone_key(),
            fingerprint: self.fingerprint.clone(),
            cert_pem: self.cert_pem.clone(),
            key_pem: self.key_pem.clone(),
        }
    }
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
    use maos_domain::frame::{
        ConsentEnvelope, FrameAddress, FramePayload, PosturePreferences, TaskAssignPayload,
    };
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
    use smallvec::smallvec;

    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_be_bytes());
    let from = FrameAddress {
        spirit_id: SpiritId::from("mira"),
        host_id: Some(HostId(from_host.to_string())),
        role: None,
    };
    // Story 8.8 — the live wire is fail-closed, so a cross-Host frame must carry a
    // canonical `intent_class`. Use the canonical band token of `intent` (e.g.
    // "readonly"), which is itself canonical and matches the band-token allowlists
    // these tests already use — behaviorally identical to the pre-8.8 band
    // fallback, just made explicit (B-clean). `with_fine_grained_intent` sets
    // granter == from, satisfying the 8.9 granter binding. Tests that need a
    // forged granter / absent host_id / explicit expiry override this envelope.
    let canonical_intent =
        maos_domain::invariants::i8::A2AIntent::new(intent.a2a_consent_intent_str());
    maos_domain::frame::IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
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
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            canonical_intent,
        )),
        intent_lineage: IntentLineage::default(),
    }
}

// ─────────────── Story 11.3 — N-host mesh scale primitives ───────────────
//
// Extends the 10.4b 3-host template (`bind_node`/`build_mesh`/
// `directed_dial_sweep`, defined LOCALLY in `t_10_4b_rotation_real_timing.rs`
// — untouched, no shared-helper churn on an already-landed test) to a
// parameterized host count for `t_11_3_scale_churn.rs`.

/// Canonical per-index host name for an N-host mesh (`host_00`, `host_01`, …).
pub fn host_name(i: usize) -> String {
    format!("host_{i:02}")
}

/// A live N-host mTLS mesh node: the bound transport + its identity
/// witnesses (cert fingerprint, bound `SocketAddr`) — the derive-and-
/// reconcile raw material for the distinct-host-identity reflex (Story 11.3
/// D7/L8).
pub struct MeshNode {
    pub name: String,
    pub transport: TcpA2ATransport,
    pub fingerprint: PeerCertFingerprint,
    pub addr: SocketAddr,
    /// Monotonic bind timestamp (Story 14-1 AC3.4, round-2 P5): the JOIN
    /// event of this host — captured at its bind inside the builder, not
    /// after the whole mesh exists, so detection samples are
    /// `t_first_rejection − t_join` per adversary, not post-build wall time.
    pub bound_at: Instant,
}

/// Parameterized N-host analogue of the 10.4b `build_mesh` — every node pins
/// every OTHER node's `expected[j]` leaf; its own served identity is
/// `serving[i]`. Passing the SAME `&Leaf` at two different indices in
/// `serving` models a CLONED identity (11.3 AC1 duplicate-identity negative
/// control): the resulting nodes bind at DISTINCT `SocketAddr`s but report
/// the SAME `PeerCertFingerprint` — `ChurnDrillReport::distinct_host_count`
/// must catch it.
/// Thin wrapper over the ONE mesh primitive `build_mesh_inner` (AC2.1).
pub async fn build_mesh_n(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    retry: HandshakeRetryPolicy,
) -> Vec<MeshNode> {
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = vec![None; names.len()];
    build_mesh_inner(
        clock,
        ca,
        names,
        serving,
        expected,
        retry,
        &gates,
        &[],
        None,
    )
    .await
}

/// Story 14-2 / AC2.0.a — the PROVISIONED mesh: `serving`/`expected` stay the
/// OLD generation while every `A2APeerConfig` DECLARES the replacement
/// (`declared`), modelling the operator's `t_provision` act —
/// `A2APeerConfig.cert_fingerprint` moves to NEW at provision time, before
/// the wire does (AC2.0's ordering: provision → swap → close). A thin
/// options-wrapper over the ONE mesh primitive (`build_mesh_inner`), NOT a
/// second builder — extending the single primitive is explicitly carved out
/// of AC3.1's prohibition and is the shape 14-1 refactored toward.
pub async fn build_mesh_n_provisioned(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    declared: &[&Leaf],
    retry: HandshakeRetryPolicy,
) -> Vec<MeshNode> {
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = vec![None; names.len()];
    build_mesh_inner(
        clock,
        ca,
        names,
        serving,
        expected,
        retry,
        &gates,
        &[],
        Some(declared),
    )
    .await
}

/// Gate-wiring variant of [`build_mesh_n`]: per-node cohort manifest gates
/// over the SAME single mesh primitive (AC2.1 — a thin wrapper, not a second
/// builder).
pub async fn build_mesh_n_with_gates(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    retry: HandshakeRetryPolicy,
    gates: &[Option<Arc<dyn CohortManifestGate>>],
) -> Vec<MeshNode> {
    build_mesh_inner(clock, ca, names, serving, expected, retry, gates, &[], None).await
}

/// The ONE node-construction loop behind every mesh builder (AC2.1: the 11.3
/// parameterized mesh and the 14.1 planted mesh are OPTIONS on one primitive,
/// never siblings). Every node pins every OTHER node's `expected[j]` leaf;
/// its own served identity is `serving[i]`. `gates[i]` wires node i's cohort
/// manifest gate; `plants` marks adversary indices (AC3.1) whose
/// consent-bypass class carries the escalation-capable SEND allowlist
/// `["readonly","standard"]` while EVERY accept list stays `["readonly"]` —
/// the escalation denial must be a real receiver-side NACK, never a
/// sender-side short-circuit. Bind semantics: plain `TcpA2ATransport::bind`
/// IS `bind_with_cohort_manifest_gate(.., gate: None)` (transport.rs), so
/// this single gate-aware call site reproduces both former builders exactly —
/// the planted path (all-`None` gates) keeps plain-`bind` semantics.
#[allow(clippy::too_many_arguments)]
async fn build_mesh_inner(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    retry: HandshakeRetryPolicy,
    gates: &[Option<Arc<dyn CohortManifestGate>>],
    plants: &[Plant],
    // Story 14-2 / AC2.0.a — `Some(leaves)` makes every peer_cfg DECLARE
    // `leaves[j]`'s fingerprint (the provisioned state: operator config
    // names the replacement while pins/serving stay on the old generation);
    // `None` ⇒ today's behaviour (`expected`).
    declared: Option<&[&Leaf]>,
) -> Vec<MeshNode> {
    let n = names.len();
    assert_eq!(serving.len(), n, "serving leaves must match host count");
    assert_eq!(expected.len(), n, "expected leaves must match host count");
    assert_eq!(gates.len(), n, "cohort gates must match host count");
    if let Some(declared) = declared {
        assert_eq!(declared.len(), n, "declared leaves must match host count");
    }
    assert!(
        plants.iter().all(|(i, _)| *i < n),
        "plant index out of mesh range"
    );
    assert!(
        !plants.iter().any(|(i, _)| *i == 0),
        "index 0 is the stable hub — never a planted adversary"
    );
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let kind = plants.iter().find(|(idx, _)| *idx == i).map(|(_, k)| *k);
        let (send, accept): (&[&str], &[&str]) = match kind {
            Some(PlantKind::AdrLevel012ConsentBypass) => (&["readonly", "standard"], &["readonly"]),
            _ => (&["readonly"], &["readonly"]),
        };
        let peers: Vec<(usize, &Leaf)> = (0..n)
            .filter(|j| *j != i)
            .map(|j| (j, expected[j]))
            .collect();
        let peer_pins: Vec<_> = peers
            .iter()
            .map(|(j, leaf)| pin(&names[*j], &leaf.fingerprint, 1_000 + *j as u64))
            .collect();
        let peer_cfgs: Vec<_> = peers
            .iter()
            .map(|(j, leaf)| {
                // Story 14-2 / AC2.0.a — the DECLARED fingerprint is what
                // the operator's A2APeerConfig.cert_fingerprint carries
                // (sites 5/6 compare the store pin against THIS). Under
                // provision, cfg declares NEW while the store still pins
                // OLD and the wire still serves OLD.
                let declared_fp = declared
                    .map(|leaves| &leaves[*j].fingerprint)
                    .unwrap_or(&leaf.fingerprint);
                peer_cfg(&names[*j], "tls://127.0.0.1:0", declared_fp, send, accept)
            })
            .collect();
        let pems = write_pem(serving[i], Some(ca));
        let tcp = tcp_config(&pems, peer_pins, Duration::from_secs(30));
        let transport = TcpA2ATransport::bind_with_cohort_manifest_gate(
            tcp,
            peer_cfgs,
            1_000 + i as u64,
            TcpTimeouts::test_profile(),
            retry.clone(),
            Some(clock.unix()),
            None,
            gates[i].clone(),
        )
        .await
        .expect("bind mesh endpoint");
        let addr = transport.local_addr().expect("bound addr (H3/H4)");
        nodes.push(MeshNode {
            name: names[i].clone(),
            transport,
            fingerprint: serving[i].fingerprint.clone(),
            addr,
            bound_at: Instant::now(),
        });
    }
    // Wire real readback addresses (H3/H4) now that all listeners are bound.
    for i in 0..n {
        for j in 0..n {
            if j != i {
                nodes[i].transport.set_peer_endpoint(
                    &HostId(names[j].clone()),
                    format!("tls://{}", nodes[j].addr),
                );
            }
        }
    }
    nodes
}

/// Story 12.4a — a live mesh node whose transport is shareable as
/// `Arc<dyn A2APeerRouter>` for the `CohortDigestDistributor` couriers.
pub struct DigestMeshNode {
    pub name: String,
    pub transport: Arc<TcpA2ATransport>,
    pub fingerprint: PeerCertFingerprint,
    pub addr: SocketAddr,
    pub rupture_log: Arc<TransparencyLogAdapter>,
}

/// Story 12.4a — N-host mesh wiring the manifest gate, halt-receipt observer,
/// AND the digest-read correlation port per node (all the SAME
/// `CohortManifestState` in the caller, P7b), with caller-chosen transport
/// allowlists so the non-reserved `cohort:digest-read` intent clears the coarse
/// ADR-012 send/accept check and the signed manifest is the fine-grained
/// authority. Additive sibling of [`build_mesh_n_with_gates`] (no caller churn).
#[allow(clippy::too_many_arguments)]
pub async fn build_mesh_n_with_digest(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    retry: HandshakeRetryPolicy,
    gates: &[Option<Arc<dyn CohortManifestGate>>],
    observers: &[Option<Arc<dyn HaltReceiptObserver>>],
    digest_ports: &[Option<Arc<dyn DigestReadPort>>],
    allowlist: &[&str],
) -> Vec<DigestMeshNode> {
    let n = names.len();
    assert_eq!(serving.len(), n, "serving leaves must match host count");
    assert_eq!(expected.len(), n, "expected leaves must match host count");
    assert_eq!(gates.len(), n, "cohort gates must match host count");
    assert_eq!(observers.len(), n, "observers must match host count");
    assert_eq!(digest_ports.len(), n, "digest ports must match host count");
    let mut nodes = Vec::with_capacity(n);
    for i in 0..n {
        let peers: Vec<(usize, &Leaf)> = (0..n)
            .filter(|j| *j != i)
            .map(|j| (j, expected[j]))
            .collect();
        let peer_pins: Vec<_> = peers
            .iter()
            .map(|(j, leaf)| pin(&names[*j], &leaf.fingerprint, 1_000 + *j as u64))
            .collect();
        let peer_cfgs: Vec<_> = peers
            .iter()
            .map(|(j, leaf)| {
                peer_cfg(
                    &names[*j],
                    "tls://127.0.0.1:0",
                    &leaf.fingerprint,
                    allowlist,
                    allowlist,
                )
            })
            .collect();
        let pems = write_pem(serving[i], Some(ca));
        let tcp = tcp_config(&pems, peer_pins, Duration::from_secs(30));
        let rupture_log = Arc::new(TransparencyLogAdapter::open_in_memory(1_000 + i as u64));
        let rupture_sink = Arc::new(CohortRuptureLogSink::new(rupture_log.clone()));
        let transport = TcpA2ATransport::bind_with_cohort_wiring_and_digest(
            tcp,
            peer_cfgs,
            1_000 + i as u64,
            TcpTimeouts::test_profile(),
            retry.clone(),
            Some(clock.unix()),
            None,
            gates[i].clone(),
            observers[i].clone(),
            digest_ports[i].clone(),
            Some(rupture_sink),
        )
        .await
        .expect("bind mesh endpoint");
        let addr = transport.local_addr().expect("bound addr (H3/H4)");
        nodes.push(DigestMeshNode {
            name: names[i].clone(),
            transport: Arc::new(transport),
            fingerprint: serving[i].fingerprint.clone(),
            addr,
            rupture_log,
        });
    }
    for i in 0..n {
        for j in 0..n {
            if j != i {
                nodes[i].transport.set_peer_endpoint(
                    &HostId(names[j].clone()),
                    format!("tls://{}", nodes[j].addr),
                );
            }
        }
    }
    nodes
}

/// Bound on concurrently in-flight dials (Story 14-1 AC6.2.a): the pre-14-1
/// sweeps issued every dial at once via `join_all`, which held every future
/// live simultaneously (~2.0 GB at full-pairwise N=100 — an artifact of
/// unbounded join, not of N). Bounded `buffered` keeps the same ORDERED
/// results semantics while capping live futures, so the N=100 legs are safe
/// on a 16 GB CI runner beside a cargo build. Order preservation is
/// load-bearing: every caller indexes results positionally.
pub const DIAL_CONCURRENCY: usize = 16;

/// Concurrent directed-dial sweep over a caller-chosen SUBSET of `(i, j)`
/// pairs — bounded in-flight concurrency via `buffered(DIAL_CONCURRENCY)`
/// (real I/O-bound handshakes genuinely overlap under tokio's cooperative
/// scheduler, so wall-clock stays low even though every dial is a real socket
/// op, while live futures stay capped — AC6.2.a/AC2.4). Story 11.3's
/// disclosed CI-budget bound: at N=30 a full NxN sweep is 30×29=870 real
/// mTLS handshakes PER ROUND; `t_11_3_scale_churn.rs` bounds each round to a
/// hub-and-spoke topology (`2×(N−1)` dials) rather than full NxN — see that
/// file's module docs for the exact bound and rationale (Task-0 disclosure).
pub async fn concurrent_dial_pairs(
    nodes: &[MeshNode],
    pairs: &[(usize, usize)],
    seq_base: u64,
    intent: IntentClass,
) -> Vec<(usize, usize, Result<(), A2AError>)> {
    use futures_util::stream::{self, StreamExt};
    let futs = pairs.iter().enumerate().map(|(k, &(i, j))| {
        let frame = make_frame(&nodes[i].name, &nodes[j].name, intent, seq_base + k as u64);
        let to = HostId(nodes[j].name.clone());
        async move {
            let res = nodes[i].transport.route_outbound(frame, &to).await;
            (i, j, res)
        }
    });
    // `buffered` (not `buffer_unordered`): results come back in submission
    // order exactly as `join_all` delivered them, so no caller sees a
    // semantic change — only the peak live-future count drops.
    stream::iter(futs)
        .buffered(DIAL_CONCURRENCY)
        .collect()
        .await
}

/// An endpoint string that no listener answers — repointing a peer here makes
/// its next `route_outbound` fail with a real transport error (ECONNREFUSED
/// on loopback). Used to model a REAL isolation / a REAL re-pin failure /
/// a REAL eviction (14-1 AC1.2). Port 1 (tcpmux) is never bound in CI;
/// connecting to it needs no privilege and is refused immediately.
pub fn dead_endpoint() -> String {
    "tls://127.0.0.1:1".to_string()
}

/// Story 14-1 AC6.2.a — the fd ceiling is PROBED, never assumed. Reads the
/// SOFT `Max open files` limit at runtime and fails LOUDLY BY NAME when the
/// N=100 mesh cannot fit under it: 502 fds at hub-and-spoke is comfortable
/// under 524,288 but THIN under a 1024 soft default, and an `EMFILE`
/// discovered mid-drill at 3am is not a measurement. An `unlimited` soft
/// limit (`RLIM_INFINITY`) is unbounded capacity — sufficient for any
/// `required`, so it returns without asserting; any OTHER unparseable value
/// fails CLOSED (soft = 0). No-op on hosts without `/proc` (the gate runs on
/// Linux CI); a READABLE limits file with no `Max open files` row fails
/// closed by name (round-2 P2: unmeasured must fail, not pass).
pub fn assert_fd_headroom(required: u64) {
    let Ok(text) = std::fs::read_to_string("/proc/self/limits") else {
        return;
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Max open files") {
            let soft_token = rest.split_whitespace().next().unwrap_or("");
            if soft_token == "unlimited" {
                // RLIM_INFINITY: unbounded capacity can never be the binding
                // constraint, so the gate passes without asserting.
                return;
            }
            // Any other unparseable token fails CLOSED at 0 — the same shape
            // as a genuinely short limit, so the EMFILE-RISK assert fires.
            let soft: u64 = soft_token.parse().unwrap_or(0);
            assert!(
                soft >= required,
                "EMFILE-RISK: soft fd limit {soft} < {required} required to stand up the \
                 N-host mesh (AC6.2.a: read the limit at runtime, fail loudly by name)"
            );
            return;
        }
    }
    assert!(
        false,
        "EMFILE-RISK-UNREADABLE: /proc/self/limits is readable but carries no `Max open \
         files` row — the fd precondition cannot be evaluated (AC6.2.a), and unmeasured \
         must fail, not pass"
    );
}

/// Story 14-1 review D-C — the peak resident set of THIS process in kB
/// (`VmHWM`, the kernel's own high-water mark, so no sampling loop can miss
/// the peak). Returns `None` where the high-water mark cannot be read; the
/// runner envelope treats `None` as UNMEASURED and fails by name — never as
/// zero. Used to make AC6.2.a's runner precondition self-enforcing instead
/// of an obligation someone remembers.
pub fn peak_rss_kb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

/// Story 14-1 AC6.2.a — the fd headroom an n-host drill requires, derived
/// from n rather than pinned to the envelope scale (round-2 P3: the N=5
/// scene must derive from its own scale, not inherit the envelope's).
/// ~20 descriptors per host keeps the N=100 requirement at the ratified
/// ~2000 while leaving small scales honest.
pub const FD_PER_HOST: u64 = 20;
/// The soft-limit floor no scale drops below (a handful of listeners plus
/// the process's own descriptors).
pub const FD_HEADROOM_FLOOR: u64 = 64;

/// The derived requirement: `n × FD_PER_HOST`, floored.
pub fn fd_headroom_for(n: usize) -> u64 {
    (n as u64 * FD_PER_HOST).max(FD_HEADROOM_FLOOR)
}

/// Which adversary class a planted node performs (Story 14-1 AC3.1). The
/// class decides ONLY the node's served identity and send allowlist — the
/// detectors are the real production ones at both surfaces.
#[derive(Clone, Copy)]
pub enum PlantKind {
    /// Serves a DIFFERENT valid leaf than the identity every peer pinned →
    /// dialer-side `HandshakeFailed { PinMismatch }` (verifier TOFU check).
    TofuPinSpoofing,
    /// Serves an EXPIRED leaf → dialer-side `HandshakeFailed { CertExpired }`
    /// (WebPKI validity runs BEFORE any pin check).
    CertRotationRaceExploit,
    /// Valid identity, but its send allowlist carries `"standard"` so its
    /// escalation probe passes the SENDER-side check and is denied at the
    /// RECEIVER's accept-allowlist → `IntentDeniedAtPeer` (router NACK).
    AdrLevel012ConsentBypass,
}

/// `(index into names, class)` for one adversary planted into the mesh.
pub type Plant = (usize, PlantKind);

/// Story 14-1 AC3.1 — an N-host mesh with adversaries planted INTO it (never
/// a dedicated 2–3 node sub-mesh). The ONE mesh primitive `build_mesh_inner`
/// (shared with [`build_mesh_n_with_gates`] — AC2.1 forbids a second mesh
/// primitive) where every node pins every other's `expected[j]`;
/// `serving[i]` is what node i really serves, so `serving[i] != expected[i]`
/// is already the cloned/spoofed identity idiom — with two planted-node
/// deltas: the consent-bypass class
/// gets an escalation-capable SEND allowlist (`["readonly","standard"]` —
/// receiver accept lists stay `["readonly"]` so the denial is a real
/// receiver-side NACK, never a sender-side short-circuit). All `N` names are
/// mesh members: at N=100 an adversary has 99 dialable peers (AC3.3.a).
pub async fn build_mesh_with_planted(
    clock: &Clock,
    ca: &Ca,
    names: &[String],
    serving: &[&Leaf],
    expected: &[&Leaf],
    plants: &[Plant],
    retry: HandshakeRetryPolicy,
) -> Vec<MeshNode> {
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = vec![None; names.len()];
    build_mesh_inner(
        clock, ca, names, serving, expected, retry, &gates, plants, None,
    )
    .await
}

/// The real outcome of one mesh-level eviction (AC1.2): the beats a caller
/// asserts on, never narrates over.
pub struct EvictionOutcome {
    pub victim: String,
    pub witness: String,
    /// The witness's REAL re-dial to the victim after the repoint. A real
    /// eviction REQUIRES this to be `Err` — confirmed unreachability, not a
    /// loop-counter event.
    pub redial_result: Result<(), A2AError>,
}

/// Story 14-1 AC1.2 — the mesh-level eviction primitive. Lifts the real
/// repoint-to-dead-port sequence out of 11.3's 3-node consent drill (where it
/// was buried) into an operation usable at ANY N: repoint `witness`'s view of
/// `victim` to a dead endpoint through the PUBLIC `set_peer_endpoint`, then
/// CONFIRM the eviction with a real re-dial that must fail. The caller owns
/// reconvergence (a real legit↔legit re-dial or sweep) and every assertion.
pub async fn evict_and_confirm(
    mesh: &[MeshNode],
    victim: usize,
    witness: usize,
    seq: u64,
) -> EvictionOutcome {
    assert_ne!(victim, witness, "a host cannot evict itself");
    mesh[witness]
        .transport
        .set_peer_endpoint(&HostId(mesh[victim].name.clone()), dead_endpoint());
    let frame = make_frame(
        &mesh[witness].name,
        &mesh[victim].name,
        IntentClass::Readonly,
        seq,
    );
    let redial_result = mesh[witness]
        .transport
        .route_outbound(frame, &HostId(mesh[victim].name.clone()))
        .await
        .map(|_| ());
    EvictionOutcome {
        victim: mesh[victim].name.clone(),
        witness: mesh[witness].name.clone(),
        redial_result,
    }
}
