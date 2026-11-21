//! Story 14-2a daemon-rotation harness, shared with Story 21-3 (`durable_pins_21_3.rs`):
//! a real `maos` child in `MAOS_ONE_SHOT=cohort-a2a-daemon` mode, a real second mTLS
//! endpoint, and a real signed reissue. Moved verbatim out of
//! `cert_rotation_trigger_14_2a.rs`; only visibility and the per-fixture `MAOS_HOME` changed.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{A2APeerConfig, PeerCertFingerprint};
use maos_cohort::{
    CohortAuthority, CohortDistributor, CohortManifest, CohortManifestState, CohortMember,
    ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};

use maos_spirit_abi::identity::{HostId, SpiritId};

pub const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
pub const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
pub const OPERATOR_TOKEN: &str = "story-14-2a-operator-token";
/// The daemon compares an inbound sender's wire nonce against the nonce stored
/// in its pin (`router.rs:1325-1361`), so the test endpoint's boot nonce and the
/// daemon's configured `peer_pins[].boot_nonce` MUST be the same value.
pub const HOST_B_NONCE: u64 = 424_242;

pub fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[142; 32])
}

pub fn authority_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

/// A fingerprint no certificate in this test hashes to — the point of a rotation
/// is that the manifest names the NEXT generation before the peer serves it.
pub fn rotated_fingerprint() -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", "5a".repeat(32)))
        .expect("fixture fingerprint parses")
}

pub struct Identity {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub fingerprint: PeerCertFingerprint,
}

pub fn mint_identity(dir: &Path, tag: &str) -> Identity {
    let key = rcgen::KeyPair::generate().expect("rcgen keypair");
    let params =
        rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("rcgen params");
    let cert = params.self_signed(&key).expect("rcgen self-signed");
    let fingerprint = PeerCertFingerprint::from_cert_der(cert.der().as_ref());
    let cert_path = dir.join(format!("{tag}.cert.pem"));
    let key_path = dir.join(format!("{tag}.key.pem"));
    std::fs::write(&cert_path, cert.pem()).expect("write cert pem");
    std::fs::write(&key_path, key.serialize_pem()).expect("write key pem");
    Identity {
        cert_path,
        key_path,
        fingerprint,
    }
}

pub fn signed_manifest(
    key: &SigningKey,
    version: u64,
    host_a: &PeerCertFingerprint,
    host_b: &PeerCertFingerprint,
) -> String {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-14-2a".to_string(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![authority_key_hex(key)],
        },
        members: vec![
            CohortMember {
                host_id: "host-a".to_string(),
                fingerprint: host_a.wire(),
                roles: vec!["worker".to_string()],
                team: None,
            },
            CohortMember {
                host_id: "host-b".to_string(),
                fingerprint: host_b.wire(),
                roles: vec!["worker".to_string()],
                team: None,
            },
        ],
        consent: ConsentMatrix {
            send: vec![ConsentTuple {
                peer: "host-b".to_string(),
                role: "worker".to_string(),
                intent: "readonly".to_string(),
            }],
            accept: vec![ConsentTuple {
                peer: "host-b".to_string(),
                role: "worker".to_string(),
                intent: "readonly".to_string(),
            }],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(key);
    toml::to_string(&manifest).expect("manifest serializes")
}

pub struct Fixture {
    pub dir: PathBuf,
    pub config_path: PathBuf,
    pub audit_db: PathBuf,
    pub host_a: Identity,
    pub host_b: Identity,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The operator port is EPHEMERAL and scraped from the daemon's own
/// announcement. Reserving a port in the parent and handing it to the child is a
/// TOCTOU: anything on the host can take it in between, and a correct daemon
/// then fails with `AddrInUse` for reasons that have nothing to do with the code
/// under test.
const OPERATOR_LISTENING_MARKER: &str = "maos: operator HTTP listening on ";

pub fn fixture(tag: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!(
        "maos-cert-rotation-14-2a-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    let key = signing_key();
    let host_a = mint_identity(&dir, "host-a");
    let host_b = mint_identity(&dir, "host-b");
    let manifest_path = dir.join("manifest.toml");
    std::fs::write(
        &manifest_path,
        signed_manifest(&key, 1, &host_a.fingerprint, &host_b.fingerprint),
    )
    .expect("write manifest");
    // The daemon's operator TOML declares host-b on BOTH surfaces the story is
    // about: `[[peers]].cert_fingerprint` (plane A, the router declaration) and
    // `tcp.peer_pins[].fingerprint` (plane B, the TOFU pin). At v1 they agree
    // with the signed manifest, which is what the boot reconciler
    // (`main.rs:9855`) requires. The reissue below moves the signed truth
    // underneath them, and nothing but this story re-checks.
    let config = format!(
        "manifest_path = '{manifest}'\n\
         authority_keys = ['{authority}']\n\
         local_host = 'host-a'\n\
         control_spirit = 'orchestrator'\n\
         \n\
         [[peers]]\n\
         peer_id = 'host-b'\n\
         endpoint = 'tls://127.0.0.1:1'\n\
         cert_fingerprint = {{ algo = 'sha256', hex = '{host_b_hex}' }}\n\
         send_allowlist = ['readonly']\n\
         accept_allowlist = ['readonly']\n\
         \n\
         [tcp]\n\
         listen_addr = '127.0.0.1:0'\n\
         own_cert_chain = '{cert}'\n\
         own_private_key = '{private_key}'\n\
         peer_pins = [{{ peer_id = 'host-b', fingerprint = {{ algo = 'sha256', hex = '{host_b_hex}' }}, boot_nonce = {nonce} }}]\n\
         \n\
         [digest_summary]\n\
         frames = 0\n\
         halts = 0\n\
         conflicts = 0\n",
        manifest = manifest_path.display(),
        authority = authority_key_hex(&key),
        cert = host_a.cert_path.display(),
        private_key = host_a.key_path.display(),
        host_b_hex = host_b.fingerprint.hex,
        nonce = HOST_B_NONCE,
    );
    let config_path = dir.join("daemon.toml");
    std::fs::write(&config_path, config).expect("write daemon config");
    Fixture {
        audit_db: dir.join("transparency.sqlite"),
        config_path,
        host_a,
        host_b,
        dir,
    }
}

pub fn boot_daemon(fixture: &Fixture) -> Result<(Child, u16, u16, String), String> {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(workspace_root)
        .env("MAOS_AUDIT_DB", &fixture.audit_db)
        // Story 21-3 — the daemon opens `$HOME/.maos/a2a-pins.json` under a single-writer
        // lock, so every fixture owns its HOME. HOME, never MAOS_HOME: MAOS_HOME shadows
        // MAOS_AUDIT_DB, which would move the Transparency Log these legs query.
        .env("HOME", &fixture.dir)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path)
        .env("MAOS_OPERATOR_BEARER_TOKEN", OPERATOR_TOKEN)
        .env("MAOS_OPERATOR_HTTP_BIND", "127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn maos daemon");
    let stderr = child.stderr.take().expect("piped stderr");
    let (tx, rx) = mpsc::channel::<String>();
    let reader = thread::spawn(move || {
        let mut buf = BufReader::new(stderr);
        let mut collected = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            match buf.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    collected.push_str(&line);
                    let _ = tx.send(line.clone());
                }
            }
        }
        collected
    });

    let deadline = Instant::now() + LISTEN_TIMEOUT;
    let mut seen = String::new();
    let mut operator_port: Option<u16> = None;
    let mut a2a_port: Option<u16> = None;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                seen.push_str(&line);
                for (marker, slot) in [
                    (OPERATOR_LISTENING_MARKER, &mut operator_port),
                    (LISTENING_MARKER, &mut a2a_port),
                ] {
                    if let Some(rest) = line.find(marker).map(|at| &line[at + marker.len()..]) {
                        *slot = rest
                            .trim()
                            .rsplit(':')
                            .next()
                            .and_then(|port| port.trim().parse::<u16>().ok());
                    }
                }
                if let (Some(operator), Some(a2a)) = (operator_port, a2a_port) {
                    return Ok((child, a2a, operator, seen));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Ok(Some(status)) = child.try_wait() {
                    drop(rx);
                    let full = reader.join().unwrap_or_default();
                    return Err(format!(
                        "daemon exited early (status {status}) before listening.\nstderr:\n{full}"
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    drop(rx);
    let full = reader.join().unwrap_or_default();
    Err(format!(
        "daemon never announced BOTH its A2A listener and its operator surface within \
         {LISTEN_TIMEOUT:?} (a2a={a2a_port:?}, operator={operator_port:?}).\nstderr:\n{full}"
    ))
}

/// The production operator READ surface, spoken to exactly as
/// `maosctl spirit inspect --sandbox` speaks to its own route.
pub fn operator_get(port: u16, path: &str) -> String {
    let address: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("operator address");
    let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .expect("connect to operator HTTP");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {OPERATOR_TOKEN}\r\nConnection: close\r\n\r\n"
    )
    .expect("write operator request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read operator response");
    response
}

pub fn peer_config(peer: &str, endpoint: &str, fingerprint: &PeerCertFingerprint) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: maos_a2a_core::PeerId::new(peer),
        endpoint: endpoint.to_string(),
        cert_fingerprint: fingerprint.clone(),
        profile: maos_a2a_core::A2AProfile::CrossHost,
        allowlists: maos_a2a_core::ConsentAllowlists {
            send_allowlist: vec![maos_domain::invariants::i8::A2AIntent::new("readonly")],
            accept_allowlist: vec![maos_domain::invariants::i8::A2AIntent::new("readonly")],
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

/// A live daemon, a real signed reissue, and the operator's view of the result.
/// Shared by the three legs below so each stays independently red-able: one
/// asserts the PIN SET changed, one asserts the TIMELINE is queryable, one
/// asserts the window CLOSES on the production grace, and none can pass on
/// another's evidence.
pub struct Rotation {
    pub fixture: Fixture,
    pub daemon: Child,
    pub rotated: PeerCertFingerprint,
    pub windows: String,
    /// The operator port, so a leg can keep reading the live surface after
    /// `drive_a_signed_rotation` returns.
    pub operator_port: u16,
    /// The instant the daemon ACKed the signed reissue — the closest test-side
    /// anchor to the instant the window opened — and the instant the open
    /// window was first OBSERVED on the operator surface. The installed-closer
    /// leg measures the elapsed time to the close against both.
    pub acked_at: Instant,
    pub opened_observed_at: Instant,
}

/// RAII teardown: an assertion panic must not leave a daemon holding its port
/// and its audit database for the rest of the test binary — which is exactly
/// what happens while running the proven-red mutations this gate depends on.
impl Drop for Rotation {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

pub async fn drive_a_signed_rotation(tag: &str) -> Rotation {
    let fixture = fixture(tag);
    let (daemon, daemon_port, operator_port, boot_log) = match boot_daemon(&fixture) {
        Ok(booted) => booted,
        Err(error) => panic!("{error}"),
    };

    // ── 1. The read seam is live BEFORE anything rotates ────────────────────
    let before = operator_get(operator_port, "/v1/a2a/rotation-windows");
    assert!(
        before.starts_with("HTTP/1.1 200"),
        "the operator rotation surface must be bound in daemon mode: {before}\nboot log:\n{boot_log}"
    );
    assert!(
        before.contains(r#""open_windows":[]"#),
        "no rotation has happened yet, so the open-window set is empty: {before}"
    );

    // ── 2. A REAL second endpoint, pinned both ways ─────────────────────────
    let host_b_config = maos_a2a_tcp::TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse().expect("listen addr"),
        own_cert_chain: fixture.host_b.cert_path.clone(),
        own_private_key: fixture.host_b.key_path.clone(),
        peer_pins: vec![maos_a2a_tcp::PinnedFingerprint {
            peer_id: maos_a2a_core::PeerId::new("host-a"),
            fingerprint: fixture.host_a.fingerprint.clone(),
            boot_nonce: 7,
        }],
        handshake_timeout: Duration::from_secs(5),
        ca_roots: None,
    };
    let host_b = Arc::new(
        maos_a2a_tcp::TcpA2ATransport::bind(
            host_b_config,
            vec![peer_config(
                "host-a",
                &format!("tls://127.0.0.1:{daemon_port}"),
                &fixture.host_a.fingerprint,
            )],
            HOST_B_NONCE,
            maos_a2a_tcp::TcpTimeouts::production(Duration::from_secs(5)),
            maos_a2a_core::HandshakeRetryPolicy::default(),
            None,
            None,
        )
        .await
        .expect("host-b endpoint binds"),
    );

    // ── 3. A REAL signed reissue, sent by the production courier ────────────
    // host-b's own view of the cohort holds v2, in which host-b's certificate
    // has rotated. `CohortDistributor::push_to` is the shipped courier: it
    // sends the exact signed artifact over `route_outbound`.
    let rotated = rotated_fingerprint();
    let v2 = signed_manifest(&signing_key(), 2, &fixture.host_a.fingerprint, &rotated);
    let host_b_state = Arc::new(
        CohortManifestState::load(
            HostId("host-b".into()),
            &v2,
            PinnedAuthorityKeys::from_keys(vec![signing_key().verifying_key()])
                .expect("pinned authority key"),
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("host-b loads the signed v2 manifest"),
    );
    let router: Arc<dyn A2APeerRouter> = Arc::clone(&host_b) as Arc<dyn A2APeerRouter>;
    let distributor = CohortDistributor::new(
        Arc::clone(&host_b_state),
        router,
        maos_domain::frame::FrameAddress {
            spirit_id: SpiritId::from("orchestrator"),
            host_id: Some(HostId("host-b".into())),
            role: None,
        },
    );
    distributor
        .push_to(&HostId("host-a".into()))
        .await
        .expect("the signed reissue is delivered and ACKed over real mTLS");
    // The ACK is the closest test-side anchor to the instant the window
    // opened: the daemon opens the window while applying the reissue and
    // ACKs immediately after.
    let acked_at = Instant::now();

    // ── 4. THE OBSERVATION: the live pin set changed ────────────────────────
    let mut seen = String::new();
    // 30 s, not 3 s: a loaded CI runner can legitimately take seconds to
    // schedule the daemon's reissue handling, and a deadline this leg can hit
    // spuriously is a false red, not a control.
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        seen = operator_get(operator_port, "/v1/a2a/rotation-windows");
        if seen.contains(&rotated.wire()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let opened_observed_at = Instant::now();
    assert!(
        seen.contains(&rotated.wire()),
        "a signed reissue MUST open a rotation window on the live daemon's pin \
         store; the operator surface still reports: {seen}"
    );
    assert!(
        seen.contains(r#""peer":"host-b""#) && seen.contains(r#""manifest_version":2"#),
        "the window names the peer and the manifest version that opened it: {seen}"
    );
    assert!(
        seen.contains(&fixture.host_b.fingerprint.wire()),
        "and the generation still serving, so an operator can see the overlap: {seen}"
    );
    // The marker is preceded by a newline because libtest writes
    // `test <name> ... ` WITHOUT a terminator under `--nocapture`, so the first
    // captured write would otherwise share that line and the gate's
    // exact-trimmed-line match would never see it.
    println!(
        "\nROTATION_WINDOWS_OBSERVED={}",
        seen.matches("\"peer\":").count()
    );
    Rotation {
        fixture,
        daemon,
        rotated,
        windows: seen,
        operator_port,
        acked_at,
        opened_observed_at,
    }
}
