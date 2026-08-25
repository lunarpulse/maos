#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! j1-crosshost-2b — a hermetic two-daemon proof that a verified delegation is
//! recorded independently on the sending and receiving Hosts, and that Host B
//! runs the operator-selected fixture worker.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use maos_a2a_core::PeerCertFingerprint;
use maos_bin::delegation::{self, DelegationLeg, HostBOutcome, HostBWorkerContext};
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::adapter::metrics::IacRtMetrics;
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::SpiritRole;
use orchestrator::{Orchestrator, DELEGATION_CONSENT_INTENT};

const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
const NONCE_A: u64 = 0x2B_A;
const NONCE_B: u64 = 0x2B_B;
const OUTCOME_LABELS: [&str; 6] = [
    "completed",
    "not_completed:process_crash",
    "not_completed:no_completion_marker",
    "not_completed:turn_failed",
    "not_completed:no_effect_evidence",
    "not_completed:permission_denied",
];

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

/// A daemon has no graceful one-shot exit. Dropping the guard reaps it even when
/// an assertion before the explicit teardown unwinds the test.
struct RunningDaemon(Child);

impl Drop for RunningDaemon {
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

/// Each identity needs its own directory: `mint_pems` deliberately uses stable
/// file names, while the two listeners must present different real leaves.
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

/// The manifest names the two daemon-local cohort identities and their real
/// leaves. The J1 peer ids intentionally do NOT appear here: they are the
/// ADR-012 bilateral path declared only in the operator's A2APeerConfig, so the
/// cohort gate defers rather than demanding a manifest-version stamp from Host A.
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
            cohort_id: "j1-crosshost-2b".to_string(),
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
    // The built-in host grant names this fixture image exactly, so this proof
    // cannot accidentally depend on a machine-local MAOS_HOST_GRANTS file.
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
    let dir = std::env::temp_dir().join(format!("maos-two-host-2b-{tag}-{nonce}"));
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

/// The daemon config uses a real pin in both directions. Host B never dials A
/// in this proof, but its peer entry is still required so verified intake can
/// select A's accept allowlist by the TLS-bound source identity.
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
         accept_allowlist = ['{intent}']\n\n[tcp]\nlisten_addr = '127.0.0.1:0'\n\
         own_cert_chain = '{cert}'\nown_private_key = '{private_key}'\n\
         peer_pins = [{{ peer_id = '{peer_id}', fingerprint = {peer_fingerprint}, boot_nonce = {peer_nonce} }}]\n\n\
         [digest_summary]\nframes = 0\nhalts = 0\nconflicts = 0\n",
        manifest = fixture.manifest.display(),
        authority = authority_key_hex(&fixture.authority),
        intent = DELEGATION_CONSENT_INTENT,
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
    std::env::join_paths(paths).expect("valid PATH entries")
}

/// Cargo does not build sibling workspace binaries for a package-scoped test.
/// Build the fixture once so both daemon children and the in-process path use
/// the same real executable rather than a test-owned stand-in.
static WORKER_FIXTURE_BUILT: LazyLock<()> = LazyLock::new(|| {
    let output = Command::new("cargo")
        .args(["build", "-q", "-p", "worker", "--bin", "worker-cli-fixture"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("run cargo build for worker fixture");
    assert!(
        output.status.success(),
        "build worker-cli-fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
});

fn ensure_fixture_binary() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
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

/// Spawn and scrape only the daemon's stderr readiness line. The readers keep
/// both pipes draining after readiness so a later worker event cannot block B.
fn boot_until_listening(
    mut command: Command,
) -> Result<(RunningDaemon, u16, Arc<Mutex<String>>, Arc<Mutex<String>>), String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn daemon: {error}"))?;
    let stderr = child.stderr.take().ok_or("daemon stderr is not piped")?;
    let stdout = child.stdout.take().ok_or("daemon stdout is not piped")?;
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let stdout_text = Arc::new(Mutex::new(String::new()));
    let (tx, rx) = mpsc::channel::<String>();
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

    let deadline = Instant::now() + LISTEN_TIMEOUT;
    let mut seen = String::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                seen.push_str(&line);
                if let Some(port) = line
                    .split_once(LISTENING_MARKER)
                    .and_then(|(_, address)| address.trim().rsplit(':').next())
                    .and_then(|port| port.parse::<u16>().ok())
                {
                    return Ok((RunningDaemon(child), port, stdout_text, stderr_text));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!(
                        "daemon exited before listening ({status}); stderr:\n{seen}"
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "daemon never printed the listening line within {LISTEN_TIMEOUT:?}; stderr:\n{seen}"
    ))
}

fn wait_for_output(
    stdout: &Arc<Mutex<String>>,
    stderr: &Arc<Mutex<String>>,
    needle: &str,
) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let combined = format!(
            "{}{}",
            stdout.lock().expect("stdout capture lock"),
            stderr.lock().expect("stderr capture lock")
        );
        if combined.contains(needle) || Instant::now() >= deadline {
            return combined;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn event_frame_id(output: &str, event: Option<&str>) -> [u8; 16] {
    let value = output
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| {
            event.is_none_or(|expected| {
                value.get("event").and_then(|v| v.as_str()) == Some(expected)
            }) && value.get("frame_id").and_then(|v| v.as_str()).is_some()
        })
        .unwrap_or_else(|| panic!("missing frame-id event {event:?}; output:\n{output}"));
    let hex_id = value["frame_id"].as_str().expect("frame_id string");
    let bytes = hex::decode(hex_id).expect("frame_id hex");
    bytes.try_into().expect("frame_id must be sixteen bytes")
}

fn log_has_frame(db: &Path, frame_id: [u8; 16]) -> bool {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT frame_id FROM transparency_log WHERE frame_id = ?1",
            rusqlite::params![&frame_id[..]],
            |_| Ok(()),
        )
        .is_ok()
}

fn log_frame_count(db: &Path, frame_id: [u8; 16]) -> i64 {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT count(*) FROM transparency_log WHERE frame_id = ?1",
            rusqlite::params![&frame_id[..]],
            |row| row.get(0),
        )
        .expect("count transparency rows")
}

/// §A6 review P9 — prove the OPERATOR-SELECTED fixture worker actually ran.
/// Keyed on the fixture's deterministic stdout marker in a journaled
/// `CliSubprocessOutput` payload, never on `kind` (H9: kind is a contaminated
/// oracle) and never on a log line the daemon itself printed.
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

fn log_payload(db: &Path, frame_id: [u8; 16]) -> Vec<u8> {
    let connection = rusqlite::Connection::open(db).expect("open transparency log");
    connection
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id = ?1",
            rusqlite::params![&frame_id[..]],
            |row| row.get(0),
        )
        .expect("outcome row payload")
}

fn run_host_a(config: &Path, audit_db: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_maos"))
        .args([
            "run",
            "spirits/topologies/j1-founder-loop-crosshost.toml",
            "--once",
        ])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_AUDIT_DB", audit_db)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_TEST_BOOT_NONCE", NONCE_A.to_string())
        // §A6 review D1 — MAOS_DELEGATED_GOAL is required on every cross-host
        // arm. This fixture asserts the delegation MECHANISM (the same frame_id
        // bytes in both logs), never the goal's content, so a dummy goal is
        // exactly right here.
        .env(
            "MAOS_DELEGATED_GOAL",
            "hermetic mechanism probe — the delegation recording is the assertion, not this goal",
        )
        .env("PATH", target_debug_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn host A")
        .wait_with_output()
        .expect("wait for host A")
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
    )
}

fn host_b_config(fixture: &Fixture, worker_manifest: bool) -> PathBuf {
    write_daemon_config(
        fixture,
        "host-b",
        &fixture.b_cert,
        &fixture.b_key,
        delegation::FROM_HOST,
        &fixture.a_fingerprint,
        NONCE_A,
        "tls://127.0.0.1:1",
        worker_manifest,
    )
}

#[test]
fn two_daemon_delegation_is_joined_on_the_same_sixteen_bytes() {
    ensure_fixture_binary();
    let fixture = fixture("crossing");
    let b_config = host_b_config(&fixture, true);
    let (host_b, port_b, b_stdout, b_stderr) =
        boot_until_listening(daemon_command(&b_config, &fixture.host_b_log, NONCE_B))
            .unwrap_or_else(|error| panic!("host B failed to boot: {error}"));
    let output = run_host_a(&host_a_config(&fixture, port_b), &fixture.host_a_log);
    let host_a_stdout = String::from_utf8_lossy(&output.stdout);
    let host_a_stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "host A failed: {host_a_stderr}");
    assert!(
        host_a_stdout.contains("\"transport\":\"cross-host-tcp-mtls\""),
        "host A must report verified TCP routing; stdout:\n{host_a_stdout}"
    );
    assert!(
        host_a_stdout.contains("topology_worker_delegated_offhost")
            && host_a_stdout.contains("\"local_worker_spawned\":false"),
        "host A must not also spawn the worker; stdout:\n{host_a_stdout}"
    );
    let host_a_frame_id = event_frame_id(&host_a_stdout, Some("delegation_routed"));
    let host_b_output = wait_for_output(&b_stdout, &b_stderr, "host_b_delegation_served");
    assert!(
        host_b_output.contains("host_b_delegation_served"),
        "host B never served the received delegation; output:\n{host_b_output}"
    );
    let host_b_frame_id = event_frame_id(&host_b_output, Some("host_b_delegation_served"));
    assert_eq!(
        host_a_frame_id, host_b_frame_id,
        "the crossing is proven in two logs only when both Hosts retain the same sixteen-byte id"
    );
    assert!(log_has_frame(&fixture.host_a_log, host_a_frame_id));
    assert!(log_has_frame(&fixture.host_b_log, host_b_frame_id));

    let outcome_id = DelegationLeg::outcome_frame_id(host_b_frame_id);
    let payload = String::from_utf8(log_payload(&fixture.host_b_log, outcome_id))
        .expect("outcome payload is UTF-8 JSON");
    assert!(
        OUTCOME_LABELS.iter().any(|label| payload.contains(label)),
        "host B outcome must carry a WorkerCompletion label; payload: {payload}"
    );
    // §A6 review P9 — the worker must be THE fixture this manifest selects.
    // A different host-granted worker that happens to emit a valid completion
    // label would otherwise keep AC1.6 green; only the fixture's deterministic
    // marker in a journaled output row pins the operator's choice.
    assert!(
        log_count_payloads_containing(&fixture.host_b_log, "worker: task complete") >= 1,
        "host B must have journaled the fixture worker's completion marker — some other \
         worker answered the delegation"
    );
    drop(host_b);
}

#[test]
fn sink_uninstalled_acks_but_does_not_admit_or_serve_the_frame() {
    let fixture = fixture("sink-uninstalled");
    let b_config = host_b_config(&fixture, false);
    let (mut host_b, port_b, b_stdout, b_stderr) =
        boot_until_listening(daemon_command(&b_config, &fixture.host_b_log, NONCE_B))
            .unwrap_or_else(|error| panic!("host B failed to boot: {error}"));
    let output = run_host_a(&host_a_config(&fixture, port_b), &fixture.host_a_log);
    let host_a_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && host_a_stdout.contains("\"transport\":\"cross-host-tcp-mtls\""),
        "without an intake sink the receiver still ACKs; stderr:\n{}\nstdout:\n{host_a_stdout}",
        String::from_utf8_lossy(&output.stderr)
    );
    let frame_id = event_frame_id(&host_a_stdout, Some("delegation_routed"));
    let host_b_output = wait_for_output(&b_stdout, &b_stderr, "never-present-marker");
    assert!(
        !host_b_output.contains("host_b_delegation_served"),
        "an uninstalled sink must not claim it served a frame; output:\n{host_b_output}"
    );
    assert!(
        !log_has_frame(&fixture.host_b_log, frame_id),
        "the ACK-and-drop control must leave no received frame row"
    );
    // §A6 review P14 — the control's "no served event" reading must not be
    // satisfiable by a DEAD daemon. A crashed host B also produces no event;
    // only a live one proves the sink's absence is why nothing was served.
    assert!(
        host_b.0.try_wait().expect("poll host B liveness").is_none(),
        "host B must still be alive — an ACK-and-drop receiver, not a crash"
    );
    drop(host_b);
}

fn inbound_frame(prior: maos_domain::frame::PriorDistillateRef) -> maos_domain::frame::IacFrame {
    let emitter = Orchestrator::new("remote-orchestrator");
    let intent = A2AIntent::new(DELEGATION_CONSENT_INTENT);
    emitter
        .assign_frame_remote(
            7,
            0x2B_C,
            delegation::RECIPIENT_SPIRIT,
            SpiritRole::Worker,
            emitter.build_task_assign("prove duplicate survives", "exit 0", Some(prior)),
            maos_domain::invariants::i13::IntentLineage::new(vec![intent.clone()]),
            delegation::TO_HOST,
            delegation::FROM_HOST,
            intent,
        )
        .expect("construct canonical delegation frame")
}

fn install_prior_distillate(db: &Path, frame_id: [u8; 16]) {
    // I11 deliberately reserves production Distillate writes to its writer.
    // This fixture supplies the already-valid predecessor that FR21 requires,
    // isolating the replay branch from an unrelated follow-up-dispatch guard.
    rusqlite::Connection::open(db)
        .expect("open host B log for predecessor")
        .execute(
            "INSERT INTO transparency_log \
             (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, boot_nonce, \
              capability_token, kind, intent, correlation_id, payload_redacted, origin) \
             VALUES (?1, 0, 0, 'distiller', 'orchestrator', ?2, NULL, 11, 'distill:write', NULL, ?3, 1)",
            rusqlite::params![&frame_id[..], NONCE_B as i64, b"test distillate"],
        )
        .expect("install prior distillate");
}

fn in_process_context(fixture: &Fixture, log: Arc<TransparencyLogAdapter>) -> HostBWorkerContext {
    let manifest_root = toml::from_str(
        &std::fs::read_to_string(&fixture.worker_manifest).expect("read worker manifest"),
    )
    .expect("parse worker manifest");
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    HostBWorkerContext {
        manifest_root,
        remote_requested: true,
        run: maos_bin::worker_spawn::RunArgs {
            manifest_path: fixture.worker_manifest.display().to_string(),
            live: false,
            once: true,
        },
        transparency_log: log,
        capability: Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x2B; 32]),
                NONCE_B,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                audit_tx,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
            ),
        ),
        spirit_host: None,
        enterprise_runtime: None,
        enterprise_pdp_runtime: None,
    }
}

#[tokio::test]
async fn replayed_inbound_frame_returns_duplicate_without_halting_host_b() {
    ensure_fixture_binary();
    let fixture = fixture("replay");
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let log = Arc::new(
        TransparencyLogAdapter::open(&fixture.host_b_log, NONCE_B).expect("open host B log"),
    );
    let prior_id = [0xD1; 16];
    // FR21 considers a TaskComplete inside its window a follow-up. Supply a
    // real prior-distillate row first, so the second receipt reaches AC3.2's
    // duplicate writer rather than failing at that independent predecessor gate.
    install_prior_distillate(&fixture.host_b_log, prior_id);
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut leg = DelegationLeg::install(
        Arc::clone(&mailbox),
        &A2AIntent::new(DELEGATION_CONSENT_INTENT),
    )
    .await
    .expect("install loopback delegation leg");
    let iac = IacBusAdapter::new(mailbox, Arc::clone(&log));
    let context = in_process_context(&fixture, Arc::clone(&log));
    let frame = inbound_frame(maos_domain::frame::PriorDistillateRef {
        digest_frame_id: prior_id,
        distillation_depth: 1,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::new(vec![A2AIntent::new(
            DELEGATION_CONSENT_INTENT,
        )]),
    });
    let frame_id = frame.frame_id;

    let first = delegation::handle_one_inbound(&mut leg, &iac, &context, frame.clone())
        .await
        .expect("first inbound frame runs host B worker");
    assert!(matches!(first, HostBOutcome::Ran { frame_id: actual, .. } if actual == frame_id));
    let second = delegation::handle_one_inbound(&mut leg, &iac, &context, frame)
        .await
        .expect("duplicate must return instead of halting host B");
    assert!(matches!(second, HostBOutcome::Duplicate { frame_id: actual } if actual == frame_id));
    assert_eq!(
        log_frame_count(&fixture.host_b_log, frame_id),
        1,
        "a replay must not create a second received-frame row"
    );
}
