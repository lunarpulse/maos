#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 13.5c — the first-ever machine execution of the `cohort-a2a-daemon`
//! mode (D9: before this story the mode occurred exactly twice in the repo, both
//! in `main.rs`, and `main.rs` admitted in-source that "no cohort daemon smoke
//! exists to prove it here").
//!
//! Two legs, per K7/K9:
//!   * hermetic (`Blocking`) — the daemon boots, binds a real listener, and
//!     reconciles to ONE Transparency Log + ONE per-boot nonce, with NO Postgres.
//!     Also proves per-boot nonce variance across two boots of the SAME config
//!     (the NFR-Rel-6 repair, D21) and the non-daemon `refreshable_source`
//!     refusal (AC3/D8), and that a config-less daemon preserves the pre-13.5c
//!     typed error.
//!   * live (`AdvisorySubstrate`, `#[ignore]`) — with a real Postgres whose
//!     `datname` matches the manifest, `MAOS_LOOM_HOME_TEAM` BOOTS where it
//!     previously hard-failed. The first production construction of
//!     `TenantMapAdapter` (D10) is reachable ONLY here (D18): the hermetic leg
//!     cannot construct it because `tenant_map_for_store` sits inside the
//!     `MAOS_LOOM_POSTGRES` arm.
//!
//! Termination (D20/K4): the daemon awaits `ctrl_c()` (SIGINT) and never exits.
//! `libc`/`nix` are unavailable to `maos-bin` and `#![forbid(unsafe_code)]`
//! applies, so the only kill idiom in the repo — `Child::kill()` (SIGKILL) —
//! is used. This SKIPS `runtime.shutdown()`; the smoke does NOT claim graceful
//! shutdown. `.output()`/`wait_with_output()` DEADLOCK on this never-exiting
//! child, so stderr is streamed by a watchdog thread: a child that never prints
//! the listening line FAILS rather than hanging CI.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use maos_bin::tenant_map::TenantMapAdapter;
use maos_cohort::{
    CohortAuthority, CohortClock, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys, TeamEntry, COHORT_SCHEMA_V2,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::ports::registry::SpiritId;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};
use maos_spirit_abi::identity::HostId;

/// A watchdog budget generous enough to absorb the ~2 200 lines of primary
/// composition root a cohort-daemon process runs before the dispatch (D24),
/// yet short enough that a hang FAILS rather than stalls CI.
const LISTEN_TIMEOUT: Duration = Duration::from_secs(90);
const LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";

// ─────────────────────────────────────────────────────────────────────────
// Fixtures — a ≥2-member, signed, schema-v2, teams-bearing, unexpired manifest
// naming the local host (D5/D16/D19). Mirrors `tenant_map_13_1.rs::signed_manifest`
// (the proven 13.1 idiom); `datname` values are the ones the live Postgres must
// carry so `connection_assignment_guard` matches (D16).
// ─────────────────────────────────────────────────────────────────────────

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[13; 32])
}

fn authority_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

/// A settable clock so the in-process refresh-liveness test can drive the lease
/// past `t_stale_secs` and back (the `tenant_map_13_1.rs` idiom).
#[derive(Default)]
struct TestClock(AtomicU64);

impl TestClock {
    fn set(&self, now_secs: u64) {
        self.0.store(now_secs, Ordering::SeqCst);
    }
}

impl CohortClock for TestClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn signed_manifest(key: &SigningKey, version: u64) -> String {
    let members = ["host-a", "host-b"]
        .iter()
        .map(|host| CohortMember {
            host_id: (*host).to_string(),
            fingerprint: format!("sha256:{}", "11".repeat(32)),
            roles: vec!["worker".to_string()],
        })
        .collect();
    let teams = Some(vec![
        TeamEntry {
            team_id: TeamId::new("team-a").unwrap(),
            region: Region::canonicalize("region-a").unwrap(),
            datname: "maos_team_a".to_string(),
            members: vec![SpiritId::from("spirit-a"), SpiritId::from("researcher")],
        },
        TeamEntry {
            team_id: TeamId::new("team-b").unwrap(),
            region: Region::canonicalize("region-b").unwrap(),
            datname: "maos_team_b".to_string(),
            members: vec![SpiritId::from("spirit-b")],
        },
    ]);
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V2,
        cohort_id: "cohort-tenant".to_string(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![authority_key_hex(key)],
        },
        members,
        consent: ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(key);
    toml::to_string(&manifest).unwrap()
}

/// Mint a self-signed leaf cert + PKCS#8 key with `rcgen` (a `network`-gated
/// `maos-bin` dep, D19; precedent: the `smoke-a2a-tcp-8-6` arm in `main.rs`).
/// `TcpA2AConfig` requires `own_cert_chain` + `own_private_key` as on-disk PEM
/// paths or the daemon will not bind.
fn mint_pems(dir: &Path) -> (PathBuf, PathBuf) {
    let key = rcgen::KeyPair::generate().expect("rcgen keypair");
    let params =
        rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("rcgen params");
    let cert = params.self_signed(&key).expect("rcgen self-signed");
    let cert_path = dir.join("own.cert.pem");
    let key_path = dir.join("own.key.pem");
    std::fs::write(&cert_path, cert.pem()).expect("write cert pem");
    std::fs::write(&key_path, key.serialize_pem()).expect("write key pem");
    (cert_path, key_path)
}

/// Write the `MAOS_COHORT_DAEMON_CONFIG` TOML. `listen_addr = 127.0.0.1:0` — the
/// real port is scraped from the listening line, never pre-bound in the parent
/// (TOCTOU). `peers = []` keeps the leg hermetic (no bilateral pull attempts).
fn write_daemon_config(dir: &Path, manifest_path: &Path, key: &SigningKey) -> PathBuf {
    let (cert, private_key) = mint_pems(dir);
    let config = format!(
        "manifest_path = '{manifest}'\n\
         authority_keys = ['{authority}']\n\
         local_host = 'host-a'\n\
         control_spirit = 'spirit-a'\n\
         peers = []\n\
         \n\
         [tcp]\n\
         listen_addr = '127.0.0.1:0'\n\
         own_cert_chain = '{cert}'\n\
         own_private_key = '{private_key}'\n\
         peer_pins = []\n\
         \n\
         [digest_summary]\n\
         frames = 0\n\
         halts = 0\n\
         conflicts = 0\n",
        manifest = manifest_path.display(),
        authority = authority_key_hex(key),
        cert = cert.display(),
        private_key = private_key.display(),
    );
    let config_path = dir.join("daemon.toml");
    std::fs::write(&config_path, config).expect("write daemon config");
    config_path
}

struct Fixture {
    dir: PathBuf,
    config_path: PathBuf,
    audit_db: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture(tag: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!(
        "maos-cohort-daemon-smoke-13-5c-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create smoke dir");
    let key = signing_key();
    let manifest_path = dir.join("manifest.toml");
    std::fs::write(&manifest_path, signed_manifest(&key, 1)).expect("write manifest");
    let config_path = write_daemon_config(&dir, &manifest_path, &key);
    Fixture {
        audit_db: dir.join("transparency.sqlite"),
        dir,
        config_path,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Subprocess control — stream stderr with a watchdog (D20). `.output()` would
// deadlock on a child that never exits.
// ─────────────────────────────────────────────────────────────────────────

fn maos_command(fixture: &Fixture) -> Command {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(workspace_root)
        .env("MAOS_AUDIT_DB", &fixture.audit_db)
        // The daemon runs the full primary root (D24); skip the live LLM probe
        // exactly as `smoke_mira_nash_tcp_8_13.rs` does.
        .env("MAOS_OLLAMA_URL", "skip")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// Spawn the daemon and block until it prints the listening line or the
/// watchdog fires. On success returns the running child + scraped port + the
/// stderr captured so far. On failure the child is reaped and all stderr is
/// surfaced.
fn boot_until_listening(mut cmd: Command) -> Result<(Child, u16, String), String> {
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
                    if tx.send(line.clone()).is_err() {
                        // Receiver gone; keep draining so the pipe never blocks
                        // the child, but stop cloning.
                    }
                }
            }
        }
        collected
    });

    let deadline = Instant::now() + LISTEN_TIMEOUT;
    let mut port: Option<u16> = None;
    let mut seen = String::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                seen.push_str(&line);
                if let Some(rest) = line
                    .find(LISTENING_MARKER)
                    .map(|i| &line[i + LISTENING_MARKER.len()..])
                {
                    let addr = rest.trim();
                    if let Some(p) = addr
                        .rsplit(':')
                        .next()
                        .and_then(|p| p.trim().parse::<u16>().ok())
                    {
                        port = Some(p);
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Ok(Some(status)) = child.try_wait() {
                    // Child exited before listening — drain the rest and fail.
                    drop(rx);
                    let full = reader.join().unwrap_or_default();
                    return Err(format!(
                        "daemon exited early (status {status}) before printing the listening line.\nstderr:\n{full}"
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                let status = match child.try_wait() {
                    Ok(Some(status)) => status,
                    Ok(None) => {
                        let _ = child.kill();
                        let status = child
                            .wait()
                            .map_err(|e| format!("failed to reap daemon: {e}"))?;
                        drop(rx);
                        let full = reader.join().unwrap_or_default();
                        return Err(format!(
                            "daemon stderr disconnected before listening while the process was still running; \
                             terminated with status {status}.\nstderr:\n{full}"
                        ));
                    }
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        drop(rx);
                        let full = reader.join().unwrap_or_default();
                        return Err(format!(
                            "failed to inspect daemon after stderr disconnected: {error}\nstderr:\n{full}"
                        ));
                    }
                };
                drop(rx);
                let full = reader.join().unwrap_or_default();
                return Err(format!(
                    "daemon exited early (status {status}) before printing the listening line.\nstderr:\n{full}"
                ));
            }
        }
    }

    match port {
        Some(port) => Ok((child, port, seen)),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            drop(rx);
            let full = reader.join().unwrap_or_default();
            Err(format!(
                "daemon never printed the listening line within {LISTEN_TIMEOUT:?} (watchdog fired).\nstderr:\n{full}"
            ))
        }
    }
}
/// Reap a still-running daemon with `Child::kill()` (SIGKILL, D20). This skips
/// `runtime.shutdown()` — the smoke never claims graceful shutdown.
fn reap(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// The DISTINCT `boot_nonce` values recorded in the Transparency Log rows
/// (`transparency_log.rs:646,1049` write the column per row). Returns an empty
/// vec when the TL has no rows.
fn tl_distinct_boot_nonces(db: &Path) -> Vec<i64> {
    let conn = rusqlite::Connection::open(db).expect("open transparency log");
    let mut stmt = conn
        .prepare("SELECT DISTINCT boot_nonce FROM transparency_log ORDER BY boot_nonce")
        .expect("prepare boot_nonce query");
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("query boot_nonce")
        .map(|r| r.expect("boot_nonce row"))
        .collect();
    rows
}

fn tl_collective_invocation(db: &Path) -> (i64, Vec<u8>, Vec<u8>) {
    let conn = rusqlite::Connection::open(db).expect("open transparency log");
    conn.query_row(
        "SELECT spirit_pid, capability_token, payload_redacted \
         FROM transparency_log WHERE intent = 'collective.write' \
         ORDER BY timestamp_ns DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("persisted collective.write invocation")
}

/// Number of distinct on-disk Transparency Log files the run produced under the
/// fixture dir — the `open()` at `main.rs:2254` is the ONLY one after 13.5c
/// deleted TL #2, so this is exactly one. A regression that reintroduced a
/// second `TransparencyLogAdapter::open` on a different path would land a second
/// `transparency*.sqlite` here and red the assertion (the planted-second-TL
/// negative, expressed structurally: the config no longer carries a
/// `transparency_log_path` field to plant one).
fn transparency_log_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read fixture dir") {
        let path = entry.expect("dir entry").path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("transparency") && name.ends_with(".sqlite") {
                found.push(path);
            }
        }
    }
    found
}

// ─────────────────────────────────────────────────────────────────────────
// Hermetic leg (`Blocking`) — cohort-daemon-boots-and-serves.
// ─────────────────────────────────────────────────────────────────────────

fn boot_hermetic_daemon(fixture: &Fixture) -> Result<(Child, u16, String), String> {
    let mut cmd = maos_command(fixture);
    cmd.env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path);
    boot_until_listening(cmd)
}

#[test]
fn cohort_daemon_boots_and_serves() {
    let fixture = fixture("boots");
    let (child, port, stderr) = boot_hermetic_daemon(&fixture).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        port > 0,
        "listener bound an ephemeral port; stderr:\n{stderr}"
    );
    reap(child);

    // ── ONE Transparency Log (TL #2 deleted): exactly one on-disk TL file.
    let tl_files = transparency_log_files(&fixture.dir);
    assert_eq!(
        tl_files.len(),
        1,
        "expected exactly one Transparency Log file, found {tl_files:?}"
    );
    assert!(
        fixture.audit_db.exists(),
        "the single TL is the primary root's audit DB at {}",
        fixture.audit_db.display()
    );

    // A real daemon boot appends one bounded `cohort:daemon-started` lifecycle
    // row through the primary root's live TransparencyLogAdapter. This is
    // deliberately non-vacuous: the row's boot_nonce is the nonce with which
    // production opened the log before threading that value into the transport.
    let nonces = tl_distinct_boot_nonces(&fixture.audit_db);
    assert_eq!(
        nonces.len(),
        1,
        "one daemon boot must persist exactly one production boot_nonce; found {nonces:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// NFR-Rel-6 repair (AC1/D3/D21) — production-wired, not reconstructed in the
// test. Boot the real daemon twice from the SAME config and audit database.
// `run_cohort_a2a_daemon` writes one lifecycle row only after the real transport
// has bound with the primary root's boot_nonce, and the live TL stamps that row
// with its open nonce. Two distinct row values therefore prove per-boot
// variance at the shipped composition root without a test-owned nonce generator.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn daemon_boot_rows_prove_per_boot_nonce_variance() {
    let fixture = fixture("nonce-variance");

    for expected_distinct_nonces in 1..=2 {
        let (child, port, stderr) =
            boot_hermetic_daemon(&fixture).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            port > 0,
            "daemon boot {expected_distinct_nonces} did not bind; stderr:\n{stderr}"
        );
        reap(child);

        let nonces = tl_distinct_boot_nonces(&fixture.audit_db);
        assert_eq!(
            nonces.len(),
            expected_distinct_nonces,
            "boot {expected_distinct_nonces} must add a distinct production boot_nonce; \
             found {nonces:?}"
        );
    }

    let tl_files = transparency_log_files(&fixture.dir);
    assert_eq!(
        tl_files.len(),
        1,
        "two boots of one config must still use one Transparency Log; found {tl_files:?}"
    );
}

#[test]
fn tenant_map_freshness_recovers_after_reissue() {
    // AC3 — the refresh loop mutates the SAME Arc<CohortManifestState> the tenant
    // map holds. Proven by observation: drive the lease stale, land a reissue
    // through the state, and show the tenant map's own freshness view recovers.
    // Same object or the test proves nothing.
    let key = signing_key();
    let pinned = PinnedAuthorityKeys::from_keys(vec![key.verifying_key()]).unwrap();
    let clock = std::sync::Arc::new(TestClock::default());
    clock.set(100);
    let audit = std::sync::Arc::new(InMemoryCohortAuditSink::default());
    let state = std::sync::Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-a".to_string()),
            &signed_manifest(&key, 1),
            pinned,
            audit,
            clock.clone(),
        )
        .expect("load v1 state"),
    );
    let map = TenantMapAdapter::new(std::sync::Arc::clone(&state), "host-a", true)
        .expect("tenant map from refreshable daemon state");
    let team_a = TeamId::new("team-a").unwrap();

    // Fresh at load (D6: confirmed_at = now).
    assert!(
        map.datname_for(&team_a).is_ok(),
        "tenant map resolves while the lease is fresh"
    );

    // Advance past t_stale_secs (120) — the tenant map fails closed.
    clock.set(100 + 121);
    assert!(
        matches!(map.datname_for(&team_a), Err(TenantMapError::Stale { .. })),
        "an expired lease must fail closed"
    );

    // A reissue landed through the state refreshes confirmed_at; the SAME map
    // recovers without being rebuilt.
    state
        .apply_reissue(&signed_manifest(&key, 2))
        .expect("apply v2 reissue");
    assert!(
        map.datname_for(&team_a).is_ok(),
        "the tenant map observes the shared state's refresh (same Arc)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Negative (`Blocking`) — a non-daemon process must REFUSE, not silently
// disable the tenant map (AC3/D8/K3). The refusal lives inside the
// MAOS_LOOM_POSTGRES arm (K9); a bogus connection string is never reached
// because `TenantMapAdapter::new`/`tenant_map_for_store` fail first.
// ─────────────────────────────────────────────────────────────────────────

fn run_to_completion(mut cmd: Command) -> (bool, String) {
    // These invocations are expected to FAIL fast at the tenant-map construction
    // (before any Postgres connection), so `.output()` is safe here — the child
    // exits on its own.
    let output = cmd.output().expect("run maos");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stderr)
}

#[test]
fn non_daemon_process_with_config_refuses_unrefreshable() {
    let fixture = fixture("neg-unrefreshable");
    let mut cmd = maos_command(&fixture);
    // Config present, home team + postgres set, but NOT in cohort-a2a-daemon
    // mode → refreshable_source == false → SourceUnrefreshable.
    cmd.env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path)
        .env("MAOS_LOOM_POSTGRES", "postgresql://unreached")
        .env("MAOS_LOOM_HOME_TEAM", "team-a")
        .env_remove("MAOS_ONE_SHOT");
    let (ok, stderr) = run_to_completion(cmd);
    assert!(
        !ok,
        "a non-daemon process must REFUSE to boot tenant mode; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("not refreshable") || stderr.contains("SourceUnrefreshable"),
        "refusal must be the typed SourceUnrefreshable, not silent None; stderr:\n{stderr}"
    );
}

#[test]
fn non_daemon_process_without_config_refuses_source_unavailable() {
    let fixture = fixture("neg-unavailable");
    let mut cmd = maos_command(&fixture);
    // No daemon config at all → SourceUnavailable (13.1's original refusal,
    // preserved: the function is unchanged, only main.rs:2286's argument moved).
    cmd.env_remove("MAOS_COHORT_DAEMON_CONFIG")
        .env("MAOS_LOOM_POSTGRES", "postgresql://unreached")
        .env("MAOS_LOOM_HOME_TEAM", "team-a")
        .env_remove("MAOS_ONE_SHOT");
    let (ok, stderr) = run_to_completion(cmd);
    assert!(
        !ok,
        "tenant mode with no manifest source must REFUSE; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("verified tenant-map source") || stderr.contains("SourceUnavailable"),
        "refusal must be the typed SourceUnavailable; stderr:\n{stderr}"
    );
}

#[test]
fn daemon_mode_without_config_preserves_typed_error() {
    let fixture = fixture("neg-noconfig");
    let mut cmd = maos_command(&fixture);
    cmd.env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env_remove("MAOS_COHORT_DAEMON_CONFIG");
    let (ok, stderr) = run_to_completion(cmd);
    assert!(
        !ok,
        "cohort-a2a-daemon mode with no config must fail; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("MAOS_COHORT_DAEMON_CONFIG must name the daemon TOML"),
        "the pre-13.5c typed error must be preserved; stderr:\n{stderr}"
    );
}

#[test]
fn daemon_config_rejects_removed_own_boot_nonce() {
    // AC1 — `own_boot_nonce` and `transparency_log_path` were removed from
    // CohortDaemonFileConfig; `#[serde(deny_unknown_fields)]` makes a stale
    // config carrying them fail to PARSE — a typed error, not a panic. This is
    // the fail-closed core of the NFR-Rel-6 repair: an operator can no longer
    // pin a static A2A boot nonce (D3/K2). Surfaced only in daemon mode (K1).
    let fixture = fixture("neg-static-nonce");
    let valid = std::fs::read_to_string(&fixture.config_path).unwrap();
    let stale = format!("own_boot_nonce = 123\n{valid}");
    let stale_path = fixture.dir.join("stale-daemon.toml");
    std::fs::write(&stale_path, stale).unwrap();
    let mut cmd = maos_command(&fixture);
    cmd.env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &stale_path);
    let (ok, stderr) = run_to_completion(cmd);
    assert!(
        !ok,
        "a config carrying own_boot_nonce must fail to boot; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("own_boot_nonce"),
        "the parse failure must name the removed field; stderr:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Live leg (`AdvisorySubstrate`, #[ignore]) — tenant route boots and serves.
// The daemon must drive Researcher's production `on_idle` hook through the
// mediated collective port and leave the readiness row in team A only.
// Serialized behind PG_LOCK; panics if either substrate env is unset.
// ─────────────────────────────────────────────────────────────────────────

static PG_LOCK: Mutex<()> = Mutex::new(());

fn psql_scalar(conn: &str, sql: &str) -> Result<String, String> {
    let output = Command::new("psql")
        .arg(conn)
        .args(["-Atc", sql])
        .output()
        .map_err(|error| format!("spawn psql: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "psql failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A (live Postgres, datname maos_team_a)"]
fn tenant_mode_boots_on_live_substrate() {
    let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let conn = std::env::var("MAOS_TEST_POSTGRES_TEAM_A")
        .expect("MAOS_TEST_POSTGRES_TEAM_A must be set for the live tenant-boot leg");
    let wrong_conn = std::env::var("MAOS_TEST_POSTGRES_TEAM_B")
        .expect("MAOS_TEST_POSTGRES_TEAM_B must be set for the live refusal control");
    // Team B must carry the real schema before the leak assertion below means
    // anything. Booting the daemon against team A installs the schema there,
    // but nothing installs it in team B — and a `SELECT count(*)` against a
    // table that does not exist cannot observe a leak: on a fresh CI database
    // it errors, and "no table" would otherwise read as "no leak" forever.
    // Install it from the production DDL so team B is a database that COULD
    // receive the row and demonstrably does not.
    let schema_sql =
        maos_loom_lite::schema::create_schema_sql(maos_loom_lite::schema::DEFAULT_VECTOR_DIM);
    psql_scalar(&wrong_conn, &schema_sql).expect("install team B schema");
    let clear_probe = "DO $$ BEGIN IF to_regclass('public.collective_memory') IS NOT NULL THEN DELETE FROM collective_memory WHERE key = 'researcher/collective-route-ready'; END IF; END $$;";
    psql_scalar(&conn, clear_probe).expect("clear team A readiness row");
    psql_scalar(&wrong_conn, clear_probe).expect("clear team B readiness row");

    let fixture = fixture("live");
    let mut cmd = maos_command(&fixture);
    cmd.env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path)
        .env("MAOS_LOOM_POSTGRES", &conn)
        .env("MAOS_LOOM_HOME_TEAM", "team-a");
    let (child, port, stderr) =
        boot_until_listening(cmd).unwrap_or_else(|e| panic!("live tenant boot failed: {e}"));
    assert!(
        port > 0,
        "tenant-mode daemon bound no listener; stderr:\n{stderr}"
    );
    reap(child);

    let mut route_cmd = maos_command(&fixture);
    route_cmd
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        .env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path)
        .env("MAOS_LOOM_POSTGRES", &conn)
        .env("MAOS_LOOM_HOME_TEAM", "team-a");
    let (route_ok, route_stderr) = run_to_completion(route_cmd);
    assert!(
        route_ok,
        "real Researcher --once route failed:\n{route_stderr}"
    );
    let (audit_pid, audit_token, audit_payload) = tl_collective_invocation(&fixture.audit_db);
    assert_eq!(
        audit_token.len(),
        32,
        "audit reference is the capability token id"
    );
    assert!(
        String::from_utf8_lossy(&audit_payload).contains("researcher/collective-route-ready"),
        "audit payload must carry the physical store-row key"
    );
    let team_a_count = psql_scalar(
        &conn,
        "SELECT count(*) FROM collective_memory WHERE key = 'researcher/collective-route-ready'",
    )
    .expect("query team A readiness row")
    .parse::<i64>()
    .expect("team A count");
    let team_b_count = psql_scalar(
        &wrong_conn,
        "SELECT count(*) FROM collective_memory WHERE key = 'researcher/collective-route-ready'",
    )
    .expect("query team B readiness row")
    .parse::<i64>()
    .expect("team B count");
    assert_eq!(
        team_a_count, 1,
        "Researcher on_idle never served team A; stderr:\n{route_stderr}"
    );
    assert_eq!(team_b_count, 0, "team A readiness row leaked into team B");
    assert_eq!(
        audit_pid, 0,
        "audit requester must be the loaded Researcher pid"
    );

    // The same signed manifest declares team-a -> maos_team_a. Pointing the
    // daemon at team B must fail at the tenant connection-assignment guard.
    let mut wrong_cmd = maos_command(&fixture);
    wrong_cmd
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", &fixture.config_path)
        .env("MAOS_LOOM_POSTGRES", wrong_conn)
        .env("MAOS_LOOM_HOME_TEAM", "team-a");
    let refusal = match boot_until_listening(wrong_cmd) {
        Ok((child, _, stderr)) => {
            reap(child);
            panic!("wrong tenant database unexpectedly booted; stderr:\n{stderr}");
        }
        Err(error) => error,
    };
    assert!(
        (refusal.contains("TenantConnectionMismatch")
            || refusal.contains("tenant connection mismatch"))
            && refusal.contains("expected database"),
        "wrong-database refusal must identify the tenant boundary; error:\n{refusal}"
    );
}

#[test]
fn production_collective_calls_share_one_atomic_pid_binding() {
    const MAIN: &str = include_str!("../src/main.rs");
    for method in ["write", "read", "scan"] {
        let signature = format!("fn collective_{method}(");
        let start = MAIN
            .find(&signature)
            .unwrap_or_else(|| panic!("missing production {signature}"));
        let tail = &MAIN[start..];
        let end = tail
            .find("\n    }\n")
            .unwrap_or_else(|| panic!("unterminated production {signature}"));
        let body = &tail[..end];
        assert_eq!(
            MAIN.matches(&format!(".collective_{method}(")).count(),
            1,
            "collective_{method} must have exactly one production kernel call site"
        );
        assert_eq!(
            body.matches("let spirit_pid = self.spirit_pid.load(")
                .count(),
            1,
            "collective_{method} must load the shared AtomicU32 exactly once"
        );
        assert!(
            body.contains("self.issue(spirit_pid,"),
            "collective_{method} token issuance must use the loaded binding"
        );
        assert!(
            body.contains(&format!(
                ".collective_{method}(\n                spirit_pid,"
            )),
            "collective_{method} kernel call must use the same loaded binding"
        );
        assert_eq!(
            body.matches("CapabilityRegistryPort::record_invocation")
                .count(),
            1,
            "collective_{method} must persist exactly one correlation audit"
        );
        assert!(
            body.contains(&format!("\"collective.{method}\""))
                && body.contains("payload.as_bytes()"),
            "collective_{method} correlation audit must bind its own intent to the row-identity payload"
        );
    }

    let registration = MAIN
        .split_once("TenantMapPort::register_spirit(")
        .expect("production tenant registration call")
        .1;
    assert!(
        registration.contains("bound_pid,"),
        "registration must use the pid reloaded from the shared AtomicU32"
    );
    assert_eq!(
        MAIN.matches("CapabilityRegistryPort::record_invocation")
            .count(),
        3,
        "production collective write, read, and scan must each persist one correlation audit"
    );
}

#[test]
fn composition_root_does_not_seed_manifest_scopes() {
    const SCANNED_SOURCE_FILES: [(&str, &str); 11] = [
        ("main.rs", include_str!("../src/main.rs")),
        ("tenant_map.rs", include_str!("../src/tenant_map.rs")),
        (
            "cross_team_consent.rs",
            include_str!("../src/cross_team_consent.rs"),
        ),
        ("env_contract.rs", include_str!("../src/env_contract.rs")),
        ("lib.rs", include_str!("../src/lib.rs")),
        ("worker_cli.rs", include_str!("../src/worker_cli.rs")),
        (
            "migration_plan.rs",
            include_str!("../src/migration_plan.rs"),
        ),
        (
            "escape_detector_consumer.rs",
            include_str!("../src/escape_detector_consumer.rs"),
        ),
        (
            "enterprise_identity.rs",
            include_str!("../src/enterprise_identity.rs"),
        ),
        (
            "enterprise_pdp_runtime.rs",
            include_str!("../src/enterprise_pdp_runtime.rs"),
        ),
        (
            "cassette_replay.rs",
            include_str!("../src/cassette_replay.rs"),
        ),
    ];
    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let source_file_count = std::fs::read_dir(&source_dir)
        .expect("maos-bin source directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .count();
    assert_eq!(
        SCANNED_SOURCE_FILES.len(),
        source_file_count,
        "every maos-bin src/*.rs file must be listed so new files cannot evade this negative"
    );

    for (file_name, source) in SCANNED_SOURCE_FILES {
        if file_name == "enterprise_pdp_runtime.rs" {
            // Whitelisted: read-only roster derivation in `known_spirit_pids` plus test helpers.
            continue;
        }
        assert_eq!(
            source.matches("manifest_scopes").count(),
            0,
            "{file_name} must never seed or consume the manifest-derived policy table"
        );
    }
}
