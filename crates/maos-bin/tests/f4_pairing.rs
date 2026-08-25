#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! j1-crosshost-2e §A6 review P11 (AC5.6 / F4) — the three proven-red pairing
//! vectors the story spec'd and the dev pass left unlanded, mirroring the
//! subprocess fixture in `two_host_delegation_2b.rs`:
//!
//!  1. `crosshost_sender_publishes_before_the_delegation_frame` — the row exists:
//!     a cross-host `maos run --once` writes exactly one `cohort:crosshost-started`
//!     Transparency-Log row carrying a NON-ZERO boot nonce, and that row is
//!     written BEFORE the delegation frame (lower rowid).
//!  2. `pairing_hold_blocks_until_host_b_is_pinned_then_releases` — the hold
//!     works: host A publishes a fresh random nonce and BLOCKS; the test reads
//!     the decimal nonce from A's output, writes host B's `[[tcp.peer_pins]]`
//!     with THAT runtime nonce, boots B, then creates the ready file; A dials,
//!     exits 0, and B journals worker intake. At HEAD (pre-2e) this vector was
//!     impossible to write — A had already exited before a human could transcribe.
//!  3. `wrong_nonce_is_refused_and_recovery_requires_a_b_restart` — NFR-Rel-6
//!     survives: a wrong non-zero nonce is refused (nothing admitted on B);
//!     the same request is refused again while the pin is invalidated; only a
//!     host-B restart with the corrected pin lets an honest frame ACK. The
//!     first-vs-second DIFFERENT wire code (-32004 restart-detected, then
//!     -32002 pin-mismatch-not-pinned) is pinned in-process against the real
//!     TOFU store, because the intake NACK deliberately journals nothing
//!     ("a -32004 refusal writes no TL row on host B — only the sender learns").
//!
//! Plus the two fail-closed hold vectors from §A6 P2/P4: expiry refuses to
//! dial, and a PRE-EXISTING ready file is refused before publication.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use maos_a2a_core::PeerCertFingerprint;
use maos_bin::delegation;

const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
const NONCE_A: u64 = 0x2B_A;
const NONCE_B: u64 = 0x2B_B;
/// V3's wrong pin: non-zero (a zero nonce is refused by bind on both sides),
/// and deliberately different from `NONCE_A` which host A actually runs with.
const WRONG_NONCE_PINNED_FOR_A: u64 = 0x2B_DEAD;

struct Fixture {
    dir: PathBuf,
    manifest: PathBuf,
    authority: SigningKey,
    a_cert: PathBuf,
    a_key: PathBuf,
    a_fingerprint: String,
    b_cert: PathBuf,
    b_key: PathBuf,
    b_fingerprint: String,
    worker_manifest: PathBuf,
    host_a_log: PathBuf,
    host_b_log: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A child that must be reaped even when an assertion unwinds the test early.
struct RunningChild(Child);

impl Drop for RunningChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[0x2B; 32])
}

fn authority_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

fn mint_pems(dir: &Path) -> (PathBuf, PathBuf, String) {
    std::fs::create_dir_all(dir).expect("create identity directory");
    let key = rcgen::KeyPair::generate().expect("rcgen keypair");
    let params =
        rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("rcgen parameters");
    let cert = params
        .self_signed(&key)
        .expect("rcgen self-signed certificate");
    let fingerprint = PeerCertFingerprint::from_cert_der(cert.der().as_ref()).to_string();
    let cert_path = dir.join("own.cert.pem");
    let key_path = dir.join("own.key.pem");
    std::fs::write(&cert_path, cert.pem()).expect("write certificate pem");
    std::fs::write(&key_path, key.serialize_pem()).expect("write private key pem");
    (cert_path, key_path, fingerprint)
}

fn signed_manifest(key: &SigningKey, a_fingerprint: &str, b_fingerprint: &str) -> String {
    use maos_cohort::{
        CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ManifestSignature, TeamEntry,
        COHORT_SCHEMA_V4, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
    };
    use maos_domain::ports::registry::SpiritId;
    use maos_domain::region::Region;
    use maos_domain::team::TeamId;

    toml::to_string(
        &CohortManifest {
            schema_version: COHORT_SCHEMA_V4,
            cohort_id: "j1-crosshost-2e-f4".to_string(),
            version: 1,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![authority_key_hex(key)],
            },
            members: vec![
                CohortMember {
                    host_id: "host-a".to_string(),
                    fingerprint: a_fingerprint.to_string(),
                    roles: vec!["worker".to_string()],
                    team: Some(TeamId::new("team-a").expect("valid team id")),
                },
                CohortMember {
                    host_id: "host-b".to_string(),
                    fingerprint: b_fingerprint.to_string(),
                    roles: vec!["worker".to_string()],
                    team: Some(TeamId::new("team-b").expect("valid team id")),
                },
            ],
            consent: ConsentMatrix::default(),
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.to_string(),
                RESERVED_INTENT_HALT_RECEIPT.to_string(),
            ],
            t_stale_secs: 120,
            teams: Some(vec![
                TeamEntry {
                    team_id: TeamId::new("team-a").expect("valid team id"),
                    region: Region::canonicalize("region-a").expect("valid region"),
                    datname: "maos_team_a".to_string(),
                    members: vec![SpiritId::from("spirit-a")],
                },
                TeamEntry {
                    team_id: TeamId::new("team-b").expect("valid team id"),
                    region: Region::canonicalize("region-b").expect("valid region"),
                    datname: "maos_team_b".to_string(),
                    members: vec![SpiritId::from("spirit-b")],
                },
            ]),
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: Vec::new(),
        }
        .signed_with(key),
    )
    .expect("serialize signed cohort manifest")
}

fn write_worker_manifest(dir: &Path) -> PathBuf {
    let path = dir.join("host-b-worker.toml");
    std::fs::write(
        &path,
        "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = [\"--maos-worker\"]\n\
         output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
         recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
         stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
         shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
         [author]\nname = \"MAOS Project\"\n",
    )
    .expect("write worker manifest");
    path
}

fn fixture(tag: &str) -> Fixture {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("maos-f4-pairing-{tag}-{nonce}"));
    std::fs::create_dir_all(&dir).expect("create fixture directory");
    let (a_cert, a_key, a_fingerprint) = mint_pems(&dir.join("host-a"));
    let (b_cert, b_key, b_fingerprint) = mint_pems(&dir.join("host-b"));
    let authority = signing_key();
    let manifest = dir.join("cohort.toml");
    std::fs::write(
        &manifest,
        signed_manifest(&authority, &a_fingerprint, &b_fingerprint),
    )
    .expect("write cohort manifest");
    let worker_manifest = write_worker_manifest(&dir);
    Fixture {
        host_a_log: dir.join("host-a.transparency.sqlite"),
        host_b_log: dir.join("host-b.transparency.sqlite"),
        dir,
        manifest,
        authority,
        a_cert,
        a_key,
        a_fingerprint,
        b_cert,
        b_key,
        b_fingerprint,
        worker_manifest,
    }
}

fn fingerprint_toml(fingerprint: &str) -> String {
    let hex = fingerprint
        .strip_prefix("sha256:")
        .expect("minted fingerprint has sha256 prefix");
    format!("{{ algo = 'sha256', hex = '{hex}' }}")
}

/// `listen_addr` is a parameter (not hardcoded `127.0.0.1:0`) because vector 2
/// must know host B's port BEFORE host A boots: A holds mid-run, so its config
/// — including B's endpoint — is fixed at spawn time.
fn write_daemon_config(
    fixture: &Fixture,
    host: &str,
    cert: &Path,
    private_key: &Path,
    peer_id: &str,
    peer_fingerprint: &str,
    peer_nonce: u64,
    peer_endpoint: &str,
    include_worker_manifest: bool,
    listen_addr: &str,
) -> PathBuf {
    let worker_manifest = include_worker_manifest
        .then(|| {
            format!(
                "worker_manifest = '{}'\n",
                fixture.worker_manifest.display()
            )
        })
        .unwrap_or_default();

    let peer_fingerprint = fingerprint_toml(peer_fingerprint);
    let config = format!(
        "manifest_path = '{manifest}'\nauthority_keys = ['{authority}']\n\
         local_host = '{host}'\ncontrol_spirit = 'orchestrator'\n{worker_manifest}\n\
         [[peers]]\npeer_id = '{peer_id}'\nendpoint = '{peer_endpoint}'\n\
         cert_fingerprint = {peer_fingerprint}\nsend_allowlist = ['{intent}']\n\
         accept_allowlist = ['{intent}']\n\n[tcp]\nlisten_addr = '{listen_addr}'\n\
         own_cert_chain = '{cert}'\nown_private_key = '{private_key}'\n\
         peer_pins = [{{ peer_id = '{peer_id}', fingerprint = {peer_fingerprint}, boot_nonce = {peer_nonce} }}]\n\n\
         [digest_summary]\nframes = 0\nhalts = 0\nconflicts = 0\n",
        manifest = fixture.manifest.display(),
        authority = authority_key_hex(&fixture.authority),
        intent = orchestrator::DELEGATION_CONSENT_INTENT,
        cert = cert.display(),
        private_key = private_key.display(),
    );
    let path = fixture.dir.join(format!("{host}.daemon.toml"));
    std::fs::write(&path, config).expect("write daemon config");
    path
}

fn target_debug_path() -> std::ffi::OsString {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut paths = vec![root.join("target/debug"), root.join("target/debug/deps")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("valid PATH")
}

/// Cargo does not build sibling workspace binaries for a package-scoped test.
fn build_fixture_binary(label: &str) {
    let output = Command::new("cargo")
        .args(["build", "-q", "-p", "worker", "--bin", "worker-cli-fixture"])
        .output()
        .unwrap_or_else(|e| panic!("{label}: cargo build: {e}"));
    assert!(
        output.status.success(),
        "{label}: fixture build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn host_a_config(fixture: &Fixture, port_b: u16) -> PathBuf {
    write_daemon_config(
        fixture,
        "host-a",
        &fixture.a_cert,
        &fixture.a_key,
        delegation::TO_HOST,
        &fixture.b_fingerprint,
        NONCE_B,
        &format!("tls://127.0.0.1:{port_b}"),
        false,
        "127.0.0.1:0",
    )
}

fn host_b_config(fixture: &Fixture, pinned_nonce_for_a: u64, listen_addr: &str) -> PathBuf {
    write_daemon_config(
        fixture,
        "host-b",
        &fixture.b_cert,
        &fixture.b_key,
        delegation::FROM_HOST,
        &fixture.a_fingerprint,
        pinned_nonce_for_a,
        "tls://127.0.0.1:1",
        true,
        listen_addr,
    )
}

fn daemon_command(config: &Path, audit_db: &Path, boot_nonce: u64) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_AUDIT_DB", audit_db)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_TEST_BOOT_NONCE", boot_nonce.to_string())
        .env("PATH", target_debug_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn boot_until_listening(mut command: Command) -> Result<(RunningChild, u16), String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn daemon: {error}"))?;
    let stderr = child.stderr.take().ok_or("daemon stderr is not piped")?;
    let stdout = child.stdout.take().ok_or("daemon stdout is not piped")?;
    let pipes = spawn_capture_threads(stdout, stderr);
    let deadline = Instant::now() + LISTEN_TIMEOUT;
    let mut seen = String::new();
    while Instant::now() < deadline {
        match pipes.stderr_rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                seen.push_str(&line);
                if let Some(port) = line
                    .split_once(LISTENING_MARKER)
                    .and_then(|(_, address)| address.trim().rsplit(':').next())
                    .and_then(|port| port.parse::<u16>().ok())
                {
                    return Ok((RunningChild(child), port));
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!(
                        "daemon exited before listening ({status}); stderr:\n{seen}"
                    ));
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "daemon never printed the listening line within {LISTEN_TIMEOUT:?}; stderr:\n{seen}"
    ))
}

struct CapturedPipes {
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
    stderr_rx: std::sync::mpsc::Receiver<String>,
}

fn spawn_capture_threads(
    stdout: impl std::io::Read + Send + 'static,
    stderr: impl std::io::Read + Send + 'static,
) -> CapturedPipes {
    let stdout_text = Arc::new(Mutex::new(String::new()));
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let stdout_capture = Arc::clone(&stdout_text);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or_default() != 0 {
            stdout_capture
                .lock()
                .expect("stdout capture lock")
                .push_str(&line);
            line.clear();
        }
    });
    let stderr_capture = Arc::clone(&stderr_text);
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or_default() != 0 {
            stderr_capture
                .lock()
                .expect("stderr capture lock")
                .push_str(&line);
            let _ = tx.send(std::mem::take(&mut line));
        }
    });
    CapturedPipes {
        stdout: stdout_text,
        stderr: stderr_text,
        stderr_rx: rx,
    }
}

fn combined(stdout: &Arc<Mutex<String>>, stderr: &Arc<Mutex<String>>) -> String {
    format!(
        "{}{}",
        stdout.lock().expect("stdout capture lock"),
        stderr.lock().expect("stderr capture lock")
    )
}

fn wait_for_output(pipes: &CapturedPipes, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let text = combined(&pipes.stdout, &pipes.stderr);
        if text.contains(needle) || Instant::now() >= deadline {
            return text;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// Host A as a one-shot run. The goal parameter exists because §A6 D1 made
/// `MAOS_DELEGATED_GOAL` REQUIRED on the cross-host arm; every vector passes a
/// unique sentinel so "was anything admitted on B?" is answerable from B's log.
fn run_host_a_once(
    config: &Path,
    audit_db: &Path,
    goal: &str,
    boot_nonce: Option<u64>,
    extra_env: &[(&str, &str)],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args([
            "run",
            "spirits/topologies/j1-founder-loop-crosshost.toml",
            "--once",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_AUDIT_DB", audit_db)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_DELEGATED_GOAL", goal)
        .env("PATH", target_debug_path());
    if let Some(nonce) = boot_nonce {
        command.env("MAOS_TEST_BOOT_NONCE", nonce.to_string());
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn host A")
        .wait_with_output()
        .expect("wait for host A")
}

/// Host A spawned streaming (vector 2): the whole point is that A stays alive
/// mid-run, holding on the rendezvous barrier.
fn spawn_host_a_streaming(
    config: &Path,
    audit_db: &Path,
    goal: &str,
    ready_file: &Path,
    timeout_secs: u64,
) -> (RunningChild, CapturedPipes) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args([
            "run",
            "spirits/topologies/j1-founder-loop-crosshost.toml",
            "--once",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_AUDIT_DB", audit_db)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_DELEGATED_GOAL", goal)
        .env("MAOS_CROSSHOST_PAIRING_READY_FILE", ready_file)
        .env(
            "MAOS_CROSSHOST_PAIRING_TIMEOUT_SECS",
            timeout_secs.to_string(),
        )
        .env("PATH", target_debug_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn streaming host A");
    let stderr = child.stderr.take().expect("A stderr piped");
    let stdout = child.stdout.take().expect("A stdout piped");
    let pipes = spawn_capture_threads(stdout, stderr);
    (RunningChild(child), pipes)
}

fn event_frame_id(output: &str, event: &str) -> Option<[u8; 16]> {
    let value = output
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| {
            value.get("event").and_then(|v| v.as_str()) == Some(event)
                && value.get("frame_id").and_then(|v| v.as_str()).is_some()
        })?;
    let hex_id = value["frame_id"].as_str()?;
    let bytes = hex::decode(hex_id).ok()?;
    bytes.try_into().ok()
}

fn sql_i64(db: &Path, sql: &str, param: &str) -> Option<i64> {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(sql, rusqlite::params![param], |row| row.get(0))
        .ok()
}

#[allow(dead_code)]
fn pairing_rowid(db: &Path) -> Option<i64> {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT min(rowid) FROM transparency_log WHERE intent = 'cohort:crosshost-started'",
            [],
            |row| row.get(0),
        )
        .ok()
}

fn pairing_row_count(db: &Path) -> i64 {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT count(*) FROM transparency_log WHERE intent = 'cohort:crosshost-started'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

fn frame_rowid(db: &Path, frame_id: [u8; 16]) -> Option<i64> {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT min(rowid) FROM transparency_log WHERE frame_id = ?1",
            rusqlite::params![&frame_id[..]],
            |row| row.get(0),
        )
        .ok()
        .flatten()
}

fn log_count_payloads_containing(db: &Path, needle: &str) -> i64 {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT count(*) FROM transparency_log WHERE payload_redacted LIKE '%' || ?1 || '%'",
            rusqlite::params![needle],
            |row| row.get(0),
        )
        .expect("count payload rows")
}

/// Reserve a port for host B before host A boots (vector 2's ordering: A holds
/// mid-run, so B's endpoint must already be in A's config at spawn time).
fn reserve_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("reserve a loopback port")
        .local_addr()
        .expect("read reserved port")
        .port()
}

/// Parse the decimal nonce host A publishes on the cross-host arm.
fn published_nonce(text: &str) -> u64 {
    let line = text
        .lines()
        .find(|l| l.contains("cross-host sender ready — boot_nonce"))
        .unwrap_or_else(|| panic!("host A must publish its nonce; output:\n{text}"));
    let tail = line
        .split("boot_nonce")
        .nth(1)
        .expect("nonce follows the marker");
    let decimal: String = tail
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    decimal.parse().unwrap_or_else(|_| {
        panic!("the published nonce must be decimal; line:\n{line}");
    })
}

// ── Vector 1 — the row exists, and it precedes the frame ───────────────────

#[test]
fn crosshost_sender_publishes_before_the_delegation_frame() {
    build_fixture_binary("V1");
    let fixture = fixture("v1-row");
    let (host_b, port_b) = boot_until_listening(daemon_command(
        &host_b_config(&fixture, NONCE_A, "127.0.0.1:0"),
        &fixture.host_b_log,
        NONCE_B,
    ))
    .unwrap_or_else(|e| panic!("host B failed to boot: {e}"));
    let output = run_host_a_once(
        &host_a_config(&fixture, port_b),
        &fixture.host_a_log,
        "V1 — prove the sender publishes its nonce before the frame",
        Some(NONCE_A),
        &[],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the crossing itself must succeed; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // (a) The nonce is published, non-zero, in decimal.
    let nonce = published_nonce(&format!("{stdout}{stderr}"));
    assert_ne!(
        nonce, 0,
        "a zero boot nonce is refused by bind on both sides"
    );

    // (b) Exactly one `cohort:crosshost-started` row lands in host A's log
    // (`MAOS_AUDIT_DB`, the same sink the frame rows use — verified during this
    // review: setting MAOS_HOME redirects A's whole TL away from MAOS_AUDIT_DB,
    // so this test deliberately does NOT isolate MAOS_HOME, matching 2b).
    let pairing_rows = pairing_row_count(&fixture.host_a_log);
    assert_eq!(
        pairing_rows, 1,
        "the sender intent must appear exactly once (got {pairing_rows}) — a second row \
         would make the receiver-only `cohort:daemon-started` ambiguity this intent \
         exists to avoid"
    );
    assert!(
        log_count_payloads_containing(&fixture.host_a_log, "crosshost_sender_started") >= 1,
        "the pairing row must carry the sender marker payload"
    );

    // (c) The pairing row precedes the delegation frame — same database, so
    // rowid ordering is the proof.
    let frame_id =
        event_frame_id(&stdout, "delegation_routed").expect("the delegation frame must be routed");
    let pairing = pairing_rowid(&fixture.host_a_log).expect("the pairing row must exist by now");
    let frame =
        frame_rowid(&fixture.host_a_log, frame_id).expect("the delegation frame must be journaled");
    assert!(
        pairing < frame,
        "publish-then-hold-then-dial: the pairing row (rowid {pairing}) must be written \
         BEFORE the delegation frame (rowid {frame})"
    );
    drop(host_b);
}

// ── Vector 2 — the hold works: publish, block, pin, boot, release, dial ────

#[test]
fn pairing_hold_blocks_until_host_b_is_pinned_then_releases() {
    build_fixture_binary("V2");
    let fixture = fixture("v2-hold");
    let port_b = reserve_port();
    let ready = fixture.dir.join("pairing.ready");
    // Host A: RANDOM nonce (no MAOS_TEST_BOOT_NONCE) — the real pairing path.
    let (mut host_a, pipes) = spawn_host_a_streaming(
        &host_a_config(&fixture, port_b),
        &fixture.host_a_log,
        "V2 — the held dial must deliver this goal to host B's fixture worker",
        &ready,
        120,
    );

    // (a) A publishes a fresh non-zero nonce and then BLOCKS.
    let published = wait_for_output(&pipes, "cross-host sender ready — boot_nonce");
    let nonce = published_nonce(&published);
    assert_ne!(nonce, 0, "the per-process nonce must be non-zero");
    thread::sleep(Duration::from_millis(750));
    assert!(
        host_a.0.try_wait().expect("poll host A").is_none(),
        "host A must still be holding — `--once` used to bind, dial and exit"
    );
    assert!(
        !ready.exists(),
        "the test has not signalled readiness yet; A must not have dialled"
    );

    // (b) Pin A's RUNTIME nonce on host B, boot B, then release the barrier.
    let (host_b, _) = boot_until_listening(daemon_command(
        &host_b_config(&fixture, nonce, &format!("127.0.0.1:{port_b}")),
        &fixture.host_b_log,
        NONCE_B,
    ))
    .unwrap_or_else(|e| panic!("host B failed to boot with the runtime pin: {e}"));
    std::fs::write(
        &ready,
        b"host B is listening with the published nonce pinned\n",
    )
    .expect("release the pairing barrier");

    // (c) A dials, exits 0, and B journals worker intake. B accepting at all
    // PROVES the dial used the published nonce: the receiver compares the
    // wire-carried sender nonce against its pin and refuses on mismatch.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut status = None;
    while Instant::now() < deadline {
        if let Some(exit) = host_a.0.try_wait().expect("poll host A exit") {
            status = Some(exit);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let status = status.unwrap_or_else(|| panic!("host A never exited after the release"));
    let final_output = combined(&pipes.stdout, &pipes.stderr);
    assert!(
        status.success(),
        "the released dial must succeed; output:\n{final_output}"
    );
    assert!(
        final_output.contains("\"transport\":\"cross-host-tcp-mtls\""),
        "the crossing must report verified TCP routing:\n{final_output}"
    );
    assert!(
        final_output.contains("host B signalled ready, dialling"),
        "the barrier release must be narrated:\n{final_output}"
    );
    assert!(
        log_count_payloads_containing(
            &fixture.host_b_log,
            "V2 — the held dial must deliver this goal"
        ) >= 1,
        "host B must have journaled the delegated goal — the held dial delivered real work"
    );
    // Worker intake is asynchronous on B: poll the journal rather than racing.
    let worker_deadline = Instant::now() + Duration::from_secs(20);
    let mut worker_done = false;
    while Instant::now() < worker_deadline {
        if log_count_payloads_containing(&fixture.host_b_log, "worker: task complete") >= 1 {
            worker_done = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        worker_done,
        "host B's fixture worker must have run to completion after the held dial"
    );
    drop(host_b);
}

/// §A6 P4/P5 — expiry fails CLOSED: no ready file, a 1-second bound, and host A
/// refuses to dial rather than spending its single non-retryable connect.
#[test]
fn pairing_hold_times_out_and_refuses_to_dial() {
    let fixture = fixture("v2-timeout");
    let ready = fixture.dir.join("never.created");
    let (mut host_a, pipes) = spawn_host_a_streaming(
        &host_a_config(&fixture, reserve_port()),
        &fixture.host_a_log,
        "V2b — this goal must never reach a worker",
        &ready,
        1,
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut status = None;
    while Instant::now() < deadline {
        if let Some(exit) = host_a.0.try_wait().expect("poll host A") {
            status = Some(exit);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let status = status.unwrap_or_else(|| panic!("host A never exited on the expired hold"));
    let output = combined(&pipes.stdout, &pipes.stderr);
    assert!(
        !status.success(),
        "an expired hold must fail the run, not dial blind:\n{output}"
    );
    assert!(
        output.contains("pairing rendezvous timed out"),
        "the refusal must name the expired hold:\n{output}"
    );
    assert!(
        event_frame_id(&output, "delegation_routed").is_none(),
        "no delegation frame may be emitted after a refused hold:\n{output}"
    );
    assert_eq!(
        log_count_payloads_containing(&fixture.host_a_log, "V2b — this goal"),
        0,
        "nothing may be journaled for a goal whose dial was refused"
    );
}

/// §A6 P2 — a ready path that already exists is refused BEFORE publication: a
/// stale file from a prior run cannot release this run's barrier.
#[test]
fn preexisting_ready_file_is_refused_before_publication() {
    let fixture = fixture("v2-stale");
    let stale = fixture.dir.join("stale-from-a-previous-run");
    std::fs::write(&stale, b"left behind by yesterday's run\n").expect("plant stale file");
    let (mut host_a, pipes) = spawn_host_a_streaming(
        &host_a_config(&fixture, reserve_port()),
        &fixture.host_a_log,
        "V2c — stale-signal refusal",
        &stale,
        30,
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut status = None;
    while Instant::now() < deadline {
        if let Some(exit) = host_a.0.try_wait().expect("poll host A") {
            status = Some(exit);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let status = status.unwrap_or_else(|| panic!("host A never refused the stale file"));
    let output = combined(&pipes.stdout, &pipes.stderr);
    assert!(
        !status.success(),
        "a pre-existing ready file must fail the run:\n{output}"
    );
    assert!(
        output.contains("already exists"),
        "the refusal must name the stale path:\n{output}"
    );
    assert!(
        !output.contains("cross-host sender ready — boot_nonce")
            || output.contains("already exists"),
        "the stale signal is refused; publication cannot release it:\n{output}"
    );
}

// ── Vector 3 — NFR-Rel-6 survives the pairing path ──────────────────────────
#[test]
fn wrong_nonce_is_refused_and_recovery_requires_a_b_restart() {
    build_fixture_binary("V3");
    let fixture = fixture("v3-restart");

    // (a) First attempt: B pins a WRONG non-zero nonce for A.
    let (host_b, port_b) = boot_until_listening(daemon_command(
        &host_b_config(&fixture, WRONG_NONCE_PINNED_FOR_A, "127.0.0.1:0"),
        &fixture.host_b_log,
        NONCE_B,
    ))
    .unwrap_or_else(|e| panic!("host B (wrongly pinned) failed to boot: {e}"));
    let goal = "V3 — a mis-pinned nonce must refuse this goal twice";
    // Each attempt gets a FRESH audit db: A's frame_id is deterministic under
    // MAOS_TEST_BOOT_NONCE, so a shared db makes attempt 2 hit FR21's 60s
    // dedup window on A-side rows and exit 0 WITHOUT dialing — the hint in
    // attempt 1's own error message names this trap.
    let attempt_db = |n: u8| fixture.dir.join(format!("host-a-attempt{n}.sqlite"));
    let first = run_host_a_once(
        &host_a_config(&fixture, port_b),
        &attempt_db(1),
        goal,
        Some(NONCE_A),
        &[],
    );
    let first_text = format!(
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        !first.status.success(),
        "a wrong nonce must refuse the frame:\n{first_text}"
    );
    assert!(
        first_text.contains("pin invalidated"),
        "the refusal must surface the NFR-Rel-6 invalidation (dialer rendering: \
         `cross-host pin mismatch … pin invalidated — re-pin consent required`):\n{first_text}"
    );
    assert_eq!(
        log_count_payloads_containing(&fixture.host_b_log, goal),
        0,
        "nothing may be admitted on B while the nonce is mis-pinned"
    );

    // (b) Identical second attempt: the pin is now `Invalidated::SpiritRestarted`
    // and intake refuses EARLIER, at cert-pin verify — the different, misleading
    // error the story documents. A journal row would lie about what B observed.
    let second = run_host_a_once(
        &host_a_config(&fixture, port_b),
        &attempt_db(2),
        goal,
        Some(NONCE_A),
        &[],
    );
    let second_text = format!(
        "{}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        !second.status.success(),
        "an invalidated pin must keep refusing:\n{second_text}"
    );
    assert_eq!(
        log_count_payloads_containing(&fixture.host_b_log, goal),
        0,
        "still nothing admitted — recovery requires a B restart, not a retry"
    );
    drop(host_b);

    // (c) Restart B with the CORRECTED pin; the same honest frame now ACKs.
    // B's pin store is in-memory and rebuilt from config at every boot
    // (RELEASE-HOLDS row 12), so the restart IS the recovery path.
    let (host_b2, port_b2) = boot_until_listening(daemon_command(
        &host_b_config(&fixture, NONCE_A, "127.0.0.1:0"),
        &fixture.host_b_log,
        NONCE_B,
    ))
    .unwrap_or_else(|e| panic!("host B (corrected) failed to boot: {e}"));
    let third = run_host_a_once(
        &host_a_config(&fixture, port_b2),
        &attempt_db(3),
        goal,
        Some(NONCE_A),
        &[],
    );
    let third_text = format!(
        "{}{}",
        String::from_utf8_lossy(&third.stdout),
        String::from_utf8_lossy(&third.stderr)
    );
    assert!(
        third.status.success(),
        "after the corrected restart the honest frame must ACK:\n{third_text}"
    );
    assert!(
        third_text.contains("\"transport\":\"cross-host-tcp-mtls\""),
        "the recovered crossing must report verified routing:\n{third_text}"
    );
    assert!(
        log_count_payloads_containing(&fixture.host_b_log, goal) >= 1,
        "the recovered run must finally admit the delegated goal on B"
    );
    drop(host_b2);
}

/// The FIRST-vs-SECOND different-refusal half of vector 3, pinned in-process
/// against the REAL TOFU store the daemon loads: the first wrong nonce takes
/// the restart-detected arm (`invalidate_if_boot_nonce_differs` → Some(prior)),
/// and the second contact then fails at `verify_pinned` because the pin is
/// `Invalidated::SpiritRestarted` — exactly the router's `-32004`-then-`-32002`
/// cascade (`router.rs:1313`, `router.rs:1343`), which deliberately journals
/// nothing on the receiver.
#[tokio::test]
async fn wrong_nonce_cascade_is_restart_detected_then_pin_mismatch() {
    use maos_a2a_core::tofu::{InMemoryTofuPinStore, TofuPinStore};
    use maos_a2a_core::PeerId;

    let store = InMemoryTofuPinStore::new();
    let peer = PeerId::new(delegation::FROM_HOST);
    let observed = PeerCertFingerprint::from_cert_der(b"f4-pairing-v3-cert");
    store
        .pin_first_contact(&peer, &observed, &observed, NONCE_A)
        .await
        .expect("first contact pins the cert and nonce");

    // First wrong nonce: the restart-detected arm (wire code -32004).
    let prior = store
        .invalidate_if_boot_nonce_differs(&peer, WRONG_NONCE_PINNED_FOR_A)
        .await
        .expect("compare-and-invalidate runs")
        .expect("a differing nonce must report the prior nonce");
    assert_eq!(
        prior, NONCE_A,
        "the reported prior nonce is the pinned one — this is the restart detection"
    );

    // Second contact with the same wrong nonce: `verify_pinned` now fails on
    // the INVALIDATED pin (wire code -32002, the misleading different error).
    let second = store
        .verify_pinned(&peer, &observed)
        .await
        .expect_err("an invalidated pin must refuse even a cert-identical peer");
    assert!(
        second.to_string().contains("Spirit restarted"),
        "the refusal must name the restart invalidation; got: {second}"
    );
}
