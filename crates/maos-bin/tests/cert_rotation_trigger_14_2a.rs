#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 14-2a — THE RUNTIME LEG. The one that proves the deliverable.
//!
//! # Why this test exists in this shape
//!
//! The obvious gate for "a production caller exists" is an AST probe, and for
//! this story an AST probe is a SPELLING CHECK. The only such primitive in the
//! repo (`xtask/src/check_reza_production_path.rs:49-71`) implements
//! `visit_expr_call` only — free functions — while every call this story makes
//! is a method call, so copied as-is that leg is green from birth. Even with
//! `visit_expr_method_call` implemented it answers only *"does this file contain
//! a method call spelled `install_cert_rotation`"*: it stays green if the call
//! moves into a `#[cfg(test)]` block or into a function nothing reaches.
//!
//! So the CONTROL is this test:
//!
//! * a REAL `maos` child process in `MAOS_ONE_SHOT=cohort-a2a-daemon` mode,
//!   running the full production composition root;
//! * a REAL second mTLS endpoint in the test process, pinned to the daemon and
//!   pinned BY the daemon through its operator TOML;
//! * a REAL signed, version-monotonic `cohort:manifest-reissue` delivered over
//!   the wire by the production courier (`CohortDistributor::push_to`);
//! * the pin-set change OBSERVED from outside the process through the
//!   production operator HTTP surface, and the transitions read back out of the
//!   daemon's own Transparency Log.
//!
//! Nothing here is a re-implementation of the production path, so **deleting the
//! `rotation_state.install_cert_rotation(...)` call in `main.rs` reds this leg**
//! while the daemon still boots, still binds, still accepts the signed reissue
//! and still advances its manifest version. That mutation — not the deletion of
//! a line of text — is the proven-red vector for this story's deliverable.

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
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlFrameKind};
use maos_spirit_abi::identity::{HostId, SpiritId};

const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
const OPERATOR_TOKEN: &str = "story-14-2a-operator-token";
/// The daemon compares an inbound sender's wire nonce against the nonce stored
/// in its pin (`router.rs:1325-1361`), so the test endpoint's boot nonce and the
/// daemon's configured `peer_pins[].boot_nonce` MUST be the same value.
const HOST_B_NONCE: u64 = 424_242;

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[142; 32])
}

fn authority_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

/// A fingerprint no certificate in this test hashes to — the point of a rotation
/// is that the manifest names the NEXT generation before the peer serves it.
fn rotated_fingerprint() -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", "5a".repeat(32)))
        .expect("fixture fingerprint parses")
}

struct Identity {
    cert_path: PathBuf,
    key_path: PathBuf,
    fingerprint: PeerCertFingerprint,
}

fn mint_identity(dir: &Path, tag: &str) -> Identity {
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

fn signed_manifest(
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

struct Fixture {
    dir: PathBuf,
    config_path: PathBuf,
    audit_db: PathBuf,
    host_a: Identity,
    host_b: Identity,
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

fn fixture(tag: &str) -> Fixture {
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

fn boot_daemon(fixture: &Fixture) -> Result<(Child, u16, u16, String), String> {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(workspace_root)
        .env("MAOS_AUDIT_DB", &fixture.audit_db)
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
fn operator_get(port: u16, path: &str) -> String {
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

fn peer_config(peer: &str, endpoint: &str, fingerprint: &PeerCertFingerprint) -> A2APeerConfig {
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
struct Rotation {
    fixture: Fixture,
    daemon: Child,
    rotated: PeerCertFingerprint,
    windows: String,
    /// The operator port, so a leg can keep reading the live surface after
    /// `drive_a_signed_rotation` returns.
    operator_port: u16,
    /// The instant the daemon ACKed the signed reissue — the closest test-side
    /// anchor to the instant the window opened — and the instant the open
    /// window was first OBSERVED on the operator surface. The installed-closer
    /// leg measures the elapsed time to the close against both.
    acked_at: Instant,
    opened_observed_at: Instant,
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

async fn drive_a_signed_rotation(tag: &str) -> Rotation {
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

/// Story 14-2a / AC1.1, AC1.2, AC1.5, AC2.1, AC2.2 — THE CONTROL: an operator
/// rotates a live daemon's trust in a peer's certificate by signing a new
/// cohort manifest, and the change is observable from outside the process.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_a_signed_reissue_rotates_a_live_daemons_peer_trust() {
    let rotation = drive_a_signed_rotation("live").await;

    assert!(
        rotation.windows.contains(&rotation.rotated.wire()),
        "the live daemon's open-window set must name the incoming generation: {}",
        rotation.windows
    );
    assert!(
        rotation
            .windows
            .contains(&rotation.fixture.host_b.fingerprint.wire()),
        "and the generation still serving, so the overlap is visible: {}",
        rotation.windows
    );
    // AC2.2 — plane A, not just plane B. `declared` is read from the LIVE router
    // core: a rotation wired to a detached core would still open and still
    // report this window, and only this assertion notices.
    assert!(
        rotation
            .windows
            .contains(&format!(r#""declared":"{}""#, rotation.rotated.wire())),
        "the live router must DECLARE the incoming generation, not merely accept it: {}",
        rotation.windows
    );
    assert!(
        rotation.windows.contains(r#""state":"open""#),
        "and the row must be reported as an OPEN overlap: {}",
        rotation.windows
    );
}

/// Story 14-2a / AC4.3, AC4.5, AC4.6(b) — the rotation timeline round-trips
/// through the REAL audit surface under one stable, greppable intent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_rotation_rows_round_trip_through_the_real_audit_surface() {
    let rotation = drive_a_signed_rotation("audit").await;
    let fixture = &rotation.fixture;
    let rotated = &rotation.rotated;

    // ── 5. And every transition is a row an auditor can query back ──────────
    let log = maos_iac::adapter::TransparencyLogAdapter::open(&fixture.audit_db, 0)
        .expect("the daemon's own Transparency Log opens for reading");
    let rows = log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TelemetryEvent),
            ..Default::default()
        })
        .expect("the Transparency Log is queryable");
    let rotation_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
        .collect();
    assert!(
        !rotation_rows.is_empty(),
        "the rotation timeline must be queryable under the stable intent \
         `{}`; TelemetryEvent intents present: {:?}",
        maos_cohort::CERT_ROTATION_INTENT,
        rows.iter().map(|row| &row.intent).collect::<Vec<_>>()
    );
    let opened = rotation_rows
        .iter()
        .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
        .find(|payload| payload.contains("cert_rotation_window_opened"))
        .unwrap_or_else(|| {
            panic!(
                "no window-opened row: {:?}",
                rotation_rows
                    .iter()
                    .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
                    .collect::<Vec<_>>()
            )
        });

    // ⚠ MEASURED: the I2 redaction filter rewrites any hex run of ≥32 chars as
    // `<REDACTED:type=capability_token,…>`, so a full 64-hex fingerprint never
    // lands on disk — not in THIS row and not in the shipped
    // `member_reissue_accepted` row either. What survives, and what an auditor
    // therefore gets, is asserted here explicitly rather than wished away: the
    // peer, the version, per-value redaction stamps that stay DISTINCT, and the
    // 8-hex correlation prefix of each generation.
    assert!(
        opened.contains(&format!(
            r#""retiring_short":"{}""#,
            &fixture.host_b.fingerprint.hex[..8]
        )) && opened.contains(&format!(r#""next_short":"{}""#, &rotated.hex[..8])),
        "the row must carry a correlation prefix for BOTH generations: {opened}"
    );
    let stamps: Vec<&str> = opened.match_indices("hash=").map(|(_, m)| m).collect();
    assert_eq!(
        stamps.len(),
        2,
        "both fingerprints are redacted, and both stamps are present: {opened}"
    );
    let retiring_stamp = opened
        .split("\"retiring\":")
        .nth(1)
        .and_then(|rest| rest.split("hash=").nth(1))
        .and_then(|rest| rest.split('>').next())
        .unwrap_or_default();
    let next_stamp = opened
        .split("\"next\":")
        .nth(1)
        .and_then(|rest| rest.split("hash=").nth(1))
        .and_then(|rest| rest.split('>').next())
        .unwrap_or_default();
    assert_ne!(
        retiring_stamp, next_stamp,
        "a redacted row that could not distinguish the two generations would be \
         an audit surface in name only: {opened}"
    );
    println!("\nROTATION_AUDIT_ROWS={}", rotation_rows.len());
}

/// Story 14-2a / AC3.2 — the PRODUCTION grace timer, not the test fixture.
///
/// Every other leg installs a manual timer and fires it by hand, so a
/// `TokioGraceTimer::schedule` that did nothing at all would leave them all
/// green and leave every widened trust set open forever. This drives the real
/// implementation. It uses a short injected grace rather than Tokio's paused
/// clock because `test-util` is not enabled on this crate and this story does
/// not change its dependencies. The margins are sized for a loaded CI runner:
/// a 2 s injected grace with a CONTINUOUS 250 ms early-fire guard (a stall can
/// only end the guard, never fail it — the old 20 ms point check against a
/// 200 ms deadline false-red the moment a stall outlived the deadline), and a
/// 15 s eventual-fire budget that scales with the grace. Bounded and
/// deterministic: the leg can never run longer than ~15.8 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_the_production_grace_timer_fires_its_terminal_action_exactly_once() {
    use maos_cohort::rotation::RotationGraceTimer;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let fired = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&fired);
    maos_bin::cert_rotation::TokioGraceTimer.schedule(
        Duration::from_secs(2),
        Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
    );

    // Early-fire, asserted CONTINUOUSLY for the first 250 ms instead of at a
    // single instant: if the runner stalls past the injected grace, the guard
    // simply ends — a correct timer can no longer be made to look like an
    // early fire, but an early-firing timer is still caught on every sample
    // the runner actually executes.
    let early_deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < early_deadline {
        assert_eq!(
            fired.load(Ordering::SeqCst),
            0,
            "the terminal action must not run before the grace elapses — the overlap is what \
             keeps a mid-rotation mesh reachable"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Eventual-fire, on a budget that scales with the injected grace.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && fired.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        fired.load(Ordering::SeqCst),
        1,
        "and it must run EXACTLY once after it: a no-op scheduler leaves the trust set \
         widened forever, and a double-fire would double-promote"
    );
    // Belt: a further wait must not produce a second firing.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert!(
        maos_cohort::cold_deployment_t_grace() >= Duration::from_secs(5),
        "§7.2.1.a floors the PRODUCTION T_grace at 5 s on either branch"
    );
}

/// Story 14-2a / AC3.2 — the INSTALLED production closer, not the timer type.
///
/// The test above drives `TokioGraceTimer::schedule` directly, so it proves
/// the timer TYPE fires while saying nothing about what `main.rs` actually
/// INSTALLS at the composition root. This closes that gap end to end against
/// a real daemon: the signed reissue opens a window on the live pin store,
/// the composition root's own
/// `rotation_state.install_cert_rotation(.., TokioGraceTimer,
/// cold_deployment_t_grace())` arms the production closer, and after the
/// five-second grace the window must be GONE from the operator surface and a
/// `cert_rotation_window_closed` row must be queryable from the daemon's own
/// Transparency Log.
///
/// Proven-red vectors, at the install rather than in isolation: a NO-OP timer
/// leaves the window open forever and reds on the close deadline; an EARLY or
/// wrong-grace closer reds on the elapsed-time floor. Timing margins are
/// stall-proof by construction: the close is measured from the reissue ACK,
/// whose own delay also delays the closer (both live in the daemon), so a
/// runner stall can only PUSH the observed close later — it can never shrink
/// the measured gap below the production grace minus a fixed 1.5 s slop for
/// observation granularity. The close deadline is bounded at
/// `T_grace + 45 s` and the audit row gets its own bounded wait, because the
/// closer removes the ledger row BEFORE it appends the terminal row.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t_14_2a_the_installed_production_closer_closes_the_window_and_journals_the_close() {
    let rotation = drive_a_signed_rotation("installed-closer").await;
    let grace = maos_cohort::cold_deployment_t_grace();

    // ── 5. The window must survive until the grace and be gone after it ─────
    let mut seen = rotation.windows.clone();
    let mut closed_observed_at: Option<Instant> = None;
    let deadline = rotation.acked_at + grace + Duration::from_secs(45);
    while closed_observed_at.is_none() {
        assert!(
            Instant::now() < deadline,
            "the window must close within T_grace + 45 s of the reissue ACK; the operator \
             surface still reports {seen}\n(a no-op timer installed at the composition root \
             leaves every widened trust set open forever)"
        );
        seen = operator_get(rotation.operator_port, "/v1/a2a/rotation-windows");
        if seen.contains(r#""open_windows":[]"#) {
            closed_observed_at = Some(Instant::now());
        } else {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    let closed_observed_at = closed_observed_at.expect("the loop only exits with a timestamp");
    let observed_gap = closed_observed_at.saturating_duration_since(rotation.acked_at);
    assert!(
        observed_gap + Duration::from_millis(1500) >= grace,
        "the window closed {observed_gap:?} after the reissue ACK (first seen open \
         {:?} after the ACK) but the production grace is {grace:?}: something fired the \
         closer EARLY (a wrong timer or wrong grace at the install)",
        rotation
            .opened_observed_at
            .saturating_duration_since(rotation.acked_at)
    );

    // ── 6. And the close is a row in the daemon's own Transparency Log ──────
    // ⚠ MEASURED (see the audit leg above): the I2 redaction filter rewrites
    // every ≥32-char hex run, so what an auditor correlates on is the 8-hex
    // prefix each row carries — `retired_short` (the generation that was
    // serving) and `promoted_short` (the generation the closer moved in).
    let retired_short = rotation.fixture.host_b.fingerprint.hex[..8].to_string();
    let promoted_short = rotation.rotated.hex[..8].to_string();
    let mut found_closed_row = false;
    let audit_deadline = Instant::now() + Duration::from_secs(20);
    while !found_closed_row {
        assert!(
            Instant::now() < audit_deadline,
            "no `cert_rotation_window_closed` row for host-b under intent {} within 20 s of \
             the close; an audit surface that reports a window's start but never its end is \
             the AC4.3 failure",
            maos_cohort::CERT_ROTATION_INTENT
        );
        let log = maos_iac::adapter::TransparencyLogAdapter::open(&rotation.fixture.audit_db, 0)
            .expect("the daemon's own Transparency Log opens for reading");
        let rows = log
            .query_frames(FrameFilter {
                kind: Some(TlFrameKind::TelemetryEvent),
                ..Default::default()
            })
            .expect("the Transparency Log is queryable");
        let closes: Vec<String> = rows
            .iter()
            .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
            .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
            .filter(|payload| payload.contains(r#""event":"cert_rotation_window_closed""#))
            .collect();
        let matching: Vec<&String> = closes
            .iter()
            .filter(|payload| {
                payload.contains(r#""peer":"host-b""#)
                    && payload.contains(r#""version":2"#)
                    && payload.contains(&format!(r#""retired_short":"{retired_short}""#))
                    && payload.contains(&format!(r#""promoted_short":"{promoted_short}""#))
            })
            .collect();
        match matching.len() {
            1 => found_closed_row = true,
            0 => tokio::time::sleep(Duration::from_millis(250)).await,
            n => panic!(
                "exactly ONE terminal row is written per opened window, found {n}: {matching:?}"
            ),
        }
    }

    // The marker is newline-prefixed for the same libtest `--nocapture` reason
    // as ROTATION_WINDOWS_OBSERVED above: the gate matches it as an exact
    // trimmed line, so an early return that skips the close cannot go green.
    println!("\nROTATION_WINDOW_CLOSED_OBSERVED=1");
}
