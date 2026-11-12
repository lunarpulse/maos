#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]
//! Story 20-1b AC11.5 (hosted T3 leg, measurement owed 4). A LIVE T3 Worker is quarantined.
//! Its next mediated calls are refused, and whether the container keeps running activity
//! is RECORDED. The AC accepts either outcome so long as the record names it.
//!
//! This asserts the capability-mediated fence and nothing else. It does NOT claim isolation:
//! the container is not frozen and not killed by a quarantine (ruling `q`).
//!
//! Route. A Worker's SCB carries no `[class]` (`worker_spawn.rs` binds
//! `SpiritManifestBundle { on_crash, supervision, ..Default }`), and the CRL arm skips a
//! class-less SCB (`applier.rs`, `let Some(class) = ... else { continue }`). So a CRL
//! cannot reach a Worker. The test applies a real CRL naming the Worker, records
//! `crl_matched_count`, and when it did not quarantine the Worker applies the same kernel
//! primitive (`set_quarantined`) that the CRL arm and the yank policy call, recording the
//! route. If the product later makes a Worker CRL-addressable the record changes and the
//! test requires the Worker to be fenced by the CRL itself.
//!
//! Host: the live-container test uses `require_t3`: a named local skip, FAIL-CLOSED on CI.
//! The fence/refusal half also runs without the substrate on a container-less Worker
//! binding (`a_bound_worker_scb_...`). Planted red: drop the fence (`set_quarantined` in
//! `fence_and_refuse`) so the post-quarantine mint succeeds, and the refusal assertions fail
//! in both tests; only the live-container test proves the container half.
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3;
use maos_bin::supervision::{
    BindingCore, WorkerBinding, WorkerSupervision, WorkerSupervisionError, WorkerSupervisor,
    WorkerTask,
};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::revocation::{CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList};
use maos_kernel_core::capability::{
    cap_audit::{self, CapAuditWriter},
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable},
    CapabilityRegistryAdapter,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::security::sandbox::t3::{
    child::SandboxedContainerChild, image_lock::VerifiedImageAttestation,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TEST: &str = "t3_worker_quarantine_refuses_next_mediated_call_and_records_activity_ac11_5";
const CHILD: &str = "MAOS_20_1B_AC11_5_CHILD";
const RECORD: &str = "AC11_5_RECORD=";
const TIER: &str = "T3";
const NONCE: u64 = 0x20_1b_11;
const HEARTBEAT: &str = "worker: fixture heartbeat";
const STARTUP_BOUND: Duration = Duration::from_secs(45);
const TEARDOWN_BOUND: Duration = Duration::from_secs(25);
/// Long enough for three one-second heartbeats of an unfenced payload.
const OBSERVE: Duration = Duration::from_secs(4);
/// A line written just before the fence may still be journaled just after it.
const SLACK_NS: i64 = 250_000_000;
const SERVER: &str = "ac11-5-mcp";
const TOOL: &str = "write";
const PROVIDER: &str = "ac11-5-provider";

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos() as i64
}

fn scopes() -> [Scope; 2] {
    [
        Scope::McpCall {
            server: SERVER.into(),
            tool: TOOL.into(),
        },
        Scope::ProviderInfer {
            provider: PROVIDER.into(),
        },
    ]
}

/// Journaled heartbeat lines `(timestamp_ns, spirit_pid)`. A TL read before the root
/// created its table is "no rows yet", any other SQLite failure is loud.
fn heartbeats(db: &Path) -> Vec<(i64, i64)> {
    if !db.exists() {
        return Vec::new();
    }
    let connection = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the transparency log read-only");
    let mut statement = match connection.prepare(
        "SELECT timestamp_ns, spirit_pid, payload_redacted FROM transparency_log \
         WHERE intent = 'cli.subprocess.output' ORDER BY rowid",
    ) {
        Ok(statement) => statement,
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Vec::new();
        }
        Err(error) => panic!("prepare TL read: {error}"),
    };
    statement
        .query_map([], |row| {
            let payload: Option<Vec<u8>> = row.get(2)?;
            Ok((row.get(0)?, row.get(1)?, payload))
        })
        .expect("query TL rows")
        .map(|row| row.expect("read TL row"))
        .filter(|(_, _, payload)| {
            payload
                .as_deref()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
                .is_some_and(|value| value["line"] == HEARTBEAT)
        })
        .map(|(timestamp, pid, _)| (timestamp, pid))
        .collect()
}

fn runtime_command() -> Command {
    Command::new(std::env::var("MAOS_T3_RUNTIME").expect("T3 runtime configured"))
}

fn container_exists(cid: &str) -> bool {
    runtime_command()
        .args(["container", "exists", cid])
        .output()
        .expect("run the OCI runtime")
        .status
        .code()
        != Some(1)
}

fn container_state(cid: &str) -> String {
    let out = runtime_command()
        .args(["container", "inspect", "--format", "{{.State.Status}}", cid])
        .output()
        .expect("run the OCI runtime");
    if out.status.success() {
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    } else {
        format!(
            "inspect-failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )
    }
}

/// Delegates to the real supervisor; only keeps the bound core to read the container id.
struct Capturing {
    inner: Arc<WorkerSupervisor>,
    core: Mutex<Option<Arc<BindingCore>>>,
}

impl WorkerSupervision for Capturing {
    fn bind(
        &self,
        task: &WorkerTask,
        bundle: SpiritManifestBundle,
    ) -> Result<WorkerBinding, WorkerSupervisionError> {
        let binding = self.inner.bind(task, bundle)?;
        *self.core.lock().expect("bound core") = Some(Arc::clone(binding.core()));
        Ok(binding)
    }
    fn create_t3_state_dir(&self) -> Result<PathBuf, String> {
        self.inner.create_t3_state_dir()
    }
    fn payload_pidfd(&self) -> bool {
        self.inner.payload_pidfd()
    }
    fn watch(&self, core: &Arc<BindingCore>, child_pid: u32) {
        self.inner.watch(core, child_pid);
    }
    fn commit_container_spawn(
        &self,
        core: &Arc<BindingCore>,
        child: &SandboxedContainerChild,
        image: &VerifiedImageAttestation,
    ) -> Result<(), WorkerSupervisionError> {
        self.inner.commit_container_spawn(core, child, image)
    }
    fn bind_token(&self, core: &Arc<BindingCore>, token: TokenId, deadline: u64) {
        self.inner.bind_token(core, token, deadline);
    }
    fn is_stopping(&self) -> bool {
        self.inner.is_stopping()
    }
}

/// Tears the Worker down on every exit path, a failed assertion included.
struct Running {
    supervision: Arc<Capturing>,
    worker: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl Running {
    fn stop(&mut self) -> Result<(), String> {
        self.supervision.inner.stop_workers();
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let deadline = Instant::now() + TEARDOWN_BOUND;
        while !worker.is_finished() {
            if Instant::now() >= deadline {
                return Err(format!("Worker did not exit within {TEARDOWN_BOUND:?}"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        worker
            .join()
            .map_err(|_| "Worker thread panicked".to_owned())?
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn crl_naming(class: &str) -> SignedRevocationList {
    let entries = vec![RevocationEntry::new(
        class,
        ">=0.0.0,<999.0.0",
        "AC11.5 quarantine of a live T3 Worker",
        None,
    )
    .expect("valid revocation entry")];
    SignedRevocationList::new(
        CrlId::from_entries(&entries).expect("CRL id"),
        1,
        0,
        RevocationOrigin::Operator,
        entries,
        [1; 64],
        [2; 32],
    )
    .expect("valid CRL")
}

/// The in-process root: real kernel adapters and the real Worker supervisor, no operator
/// door. `runtime` is the last field so it outlives everything that holds its handle.
struct Root {
    db: PathBuf,
    log: Arc<TransparencyLogAdapter>,
    policy: Arc<PolicyTable>,
    capability: Arc<CapabilityRegistryAdapter>,
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    applier: maos_kernel_core::revocation::RevocationApplier,
    supervision: Arc<Capturing>,
    runtime: tokio::runtime::Runtime,
}

impl Root {
    fn new(home: &Path) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime");
        let entered = runtime.enter();
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        std::fs::create_dir_all(home.join("audit")).expect("audit directory");
        let db = home.join("audit/transparency.sqlite");
        let log = Arc::new(TransparencyLogAdapter::open(&db, NONCE).expect("transparency log"));
        let policy = Arc::new(PolicyTable::new());
        let (audit_tx, audit_rx) = cap_audit::channel();
        CapAuditWriter::spawn(audit_rx, Arc::clone(&log));
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x20; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
        ));
        let memory_db = home.join("memory.sqlite");
        let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                home.join("private"),
                4,
            )),
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                    .expect("shared memory"),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .expect("principal memory"),
            ),
            Arc::clone(&log),
        ));
        let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            mailbox,
            Arc::clone(&log),
        ));
        let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
            Arc::clone(&log),
            Arc::clone(&capability),
            memory,
            Arc::clone(&iac),
            Arc::clone(&halt),
            Arc::clone(&metrics),
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&home.join("lifecycle.ndjson"))
                .expect("lifecycle journal"),
        );
        let detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&log),
            Arc::clone(&halt),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&metrics),
            Arc::clone(&journal),
        ));
        Arc::get_mut(&mut scheduler)
            .expect("install detector before scheduler clones")
            .set_crash_detector(Arc::clone(&detector));
        let applier = maos_kernel_core::revocation::RevocationApplier::new(
            scheduler.scbs(),
            Arc::clone(&capability),
            Arc::clone(&scheduler),
            Arc::clone(&iac),
            Arc::clone(&halt),
            Arc::clone(&log),
            Arc::clone(&journal),
            metrics,
        );
        let supervision = Arc::new(Capturing {
            inner: Arc::new(WorkerSupervisor::new(
                Arc::clone(&scheduler),
                detector,
                Arc::clone(&log),
                journal,
                halt,
                iac,
                runtime.handle().clone(),
                NONCE,
                tokio_util::sync::CancellationToken::new(),
            )),
            core: Mutex::new(None),
        });
        drop(entered);
        Self {
            db,
            log,
            policy,
            capability,
            scheduler,
            applier,
            supervision,
            runtime,
        }
    }

    /// The real T3 launch path (`maos run` of a standalone `hang-chatty` Worker).
    fn launch(&self) -> Running {
        let manifest = format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang-chatty\"]\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n[sandbox]\ntier = \"{TIER}\"\n\
             [author]\nname = \"MAOS Project\"\n"
        );
        let handle = self.runtime.handle().clone();
        let (log, capability, policy, supervision) = (
            Arc::clone(&self.log),
            Arc::clone(&self.capability),
            Arc::clone(&self.policy),
            Arc::clone(&self.supervision),
        );
        Running {
            supervision: Arc::clone(&self.supervision),
            worker: Some(std::thread::spawn(move || {
                let _entered = handle.enter();
                let manifest = toml::from_str(&manifest).expect("Worker manifest");
                maos_bin::worker_spawn::run_cli_wrapper_manifest(
                    &manifest,
                    &maos_bin::worker_spawn::RunArgs {
                        manifest_path: "quarantine-worker.toml".into(),
                        live: false,
                        once: true,
                        scene: None,
                        epic: None,
                    },
                    log,
                    capability,
                    policy,
                    None,
                    None,
                    None,
                    None,
                    false,
                    supervision.as_ref(),
                    WorkerTask::Standalone,
                    &maos_kernel_core::security::crypto::RingCryptoProvider,
                    None,
                )
                .map(|_| ())
                .map_err(|error| error.to_string())
            })),
        }
    }

    fn mint(&self, pid: u32, scope: Scope) -> Result<CapabilityToken, CapError> {
        self.capability
            .issue_with_mediation(pid, scope, 60, [0u8; 32], IntentClass::Standard)
    }

    fn verify(&self, token: &CapabilityToken) -> Result<(), CapError> {
        self.capability
            .verify_and_audit(token, [0u8; 32], SandboxTier::T3)
    }
}

/// What quarantining one bound Worker pid did, and how its next mediated calls fared.
struct Fence {
    route: &'static str,
    class_declared: bool,
    crl_matched_count: usize,
    refused: serde_json::Map<String, serde_json::Value>,
    fenced_at: i64,
}

/// Control, quarantine, refusal for the Worker bound at `pid`. Works on any bound Worker
/// SCB, so a host without the T3 substrate still runs it on a container-less binding.
fn fence_and_refuse(root: &Root, pid: u32) -> Fence {
    // Control: before the fence the pid mints and verifies, so a later refusal is the
    // fence and not a missing grant.
    root.policy.inner().rcu(|old| {
        let mut next = (**old).clone();
        next.manifest_scopes.insert(
            pid,
            ManifestCapabilityScope {
                scopes: scopes().to_vec(),
                declared_tier: SandboxTier::T3,
                trust_tier: TrustTier::Verified,
            },
        );
        Arc::new(next)
    });
    let held: Vec<CapabilityToken> = scopes()
        .into_iter()
        .map(|scope| root.mint(pid, scope).expect("an unfenced Worker mints"))
        .collect();
    for token in &held {
        assert_eq!(
            root.verify(token),
            Ok(()),
            "an unfenced held token verifies"
        );
    }
    assert!(
        !root.capability.is_quarantined(pid),
        "no fence before quarantine"
    );

    // Quarantine: the production CRL route first, and record what it did.
    let scb = root
        .scheduler
        .scbs()
        .read()
        .expect("spirits lock")
        .get(&pid)
        .cloned()
        .expect("the Worker's SCB");
    let class = scb.runtime_snapshot().manifest.class.clone();
    let class_declared = class.is_some();
    let named = class.map_or_else(|| scb.spirit_id.clone(), |class| class.name);
    let report = root
        .runtime
        .block_on(root.applier.apply_crl(crl_naming(&named)))
        .expect("apply CRL");
    let route = if root.capability.is_quarantined(pid) {
        "crl"
    } else {
        assert_eq!(
            report.matched_count, 0,
            "a CRL that matched the Worker must have fenced it"
        );
        root.capability.set_quarantined(pid, true);
        "set_quarantined: the kernel primitive the CRL arm and the yank policy call; the CRL did not reach the Worker"
    };
    let fenced_at = now_ns();
    assert!(root.capability.is_quarantined(pid), "the Worker is fenced");

    // The quarantined Worker's next mediated calls are refused.
    let mut refused = serde_json::Map::new();
    for (label, scope) in ["mcp_issue", "inference_issue"].into_iter().zip(scopes()) {
        let result = root.mint(pid, scope);
        assert_eq!(result, Err(CapError::Revoked), "{label} must be refused");
        refused.insert(label.into(), format!("{result:?}").into());
    }
    for token in &held {
        let result = root.verify(token);
        assert_eq!(result, Err(CapError::Revoked), "held token must be refused");
        refused.insert("held_token_verify".into(), format!("{result:?}").into());
    }
    Fence {
        route,
        class_declared,
        crl_matched_count: report.matched_count,
        refused,
        fenced_at,
    }
}

/// The hosted leg, in an isolated child: a live T3 Worker is quarantined; whether its
/// container keeps running activity is recorded, never assumed.
fn quarantined_worker(home: &Path) -> serde_json::Value {
    let root = Root::new(home);
    let mut running = root.launch();

    // A live Worker whose payload visibly acts: two journaled heartbeats first, so a
    // silent window after the fence would mean something.
    let deadline = Instant::now() + STARTUP_BOUND;
    while heartbeats(&root.db).len() < 2 {
        let finished = running.worker.as_ref().is_some_and(|w| w.is_finished());
        assert!(!finished, "the Worker exited before two heartbeats");
        assert!(
            Instant::now() < deadline,
            "no two heartbeats within {STARTUP_BOUND:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let core = root
        .supervision
        .core
        .lock()
        .expect("bound core")
        .clone()
        .expect("Worker bound");
    let pid = core.spirit_pid();
    assert_eq!(
        heartbeats(&root.db).last().map(|row| row.1),
        Some(i64::from(pid)),
        "the heartbeats are the bound Worker's"
    );
    let cid = core
        .container_terminal()
        .expect("a live T3 container")
        .container_id()
        .to_owned();
    assert!(container_exists(&cid), "the Worker container is live");

    let fence = fence_and_refuse(&root, pid);

    // Record, never assert, whether the container keeps acting.
    std::thread::sleep(OBSERVE);
    let after: Vec<i64> = heartbeats(&root.db)
        .into_iter()
        .filter(|(timestamp, _)| *timestamp > fence.fenced_at + SLACK_NS)
        .map(|(timestamp, _)| timestamp)
        .collect();
    let state = container_state(&cid);
    let exists = container_exists(&cid);

    // The fence never pinned the container: a stop still tears it down.
    running.stop().expect("Worker exit after quarantine");
    let started = Instant::now();
    while container_exists(&cid) {
        assert!(
            started.elapsed() < TEARDOWN_BOUND,
            "container {cid} outlived the stop of a quarantined Worker"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    serde_json::json!({
        "story": "20-1b",
        "ac": "AC11.5",
        "tier": TIER,
        "worker_pid": pid,
        "container_id": cid,
        "quarantine_route": fence.route,
        "worker_scb_declares_class": fence.class_declared,
        "crl_matched_count": fence.crl_matched_count,
        "next_mediated_call_refused": fence.refused,
        "container_activity_continues": !after.is_empty(),
        "heartbeats_journaled_after_quarantine": after.len(),
        "observation_window_ms": OBSERVE.as_millis() as u64,
        "container_state_after_window": state,
        "container_exists_after_window": exists,
        "container_removed_by_stop": true,
        "claim": "capability-mediated fence only; the container is neither frozen nor killed by quarantine; not isolation",
    })
}

/// No substrate needed: the same control, quarantine and refusal on a real supervisor
/// binding (a Worker-shaped SCB without a container). Runs on every host, so the plant
/// that drops the fence reds here too.
#[test]
fn a_bound_worker_scb_is_fenced_and_its_next_mediated_calls_are_refused_ac11_5() {
    let home = tempfile::tempdir().expect("home");
    let root = Root::new(home.path());
    let binding = root
        .supervision
        .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
        .expect("bind a real Worker SCB");
    let pid = binding.spirit_pid();
    assert_ne!(pid, 0, "a real scheduler pid");
    let fence = fence_and_refuse(&root, pid);
    assert_eq!(
        fence.refused.len(),
        3,
        "both issuances and the held token: {:?}",
        fence.refused
    );
    eprintln!(
        "AC11.5 local: route={} class_declared={} crl_matched_count={}",
        fence.route, fence.class_declared, fence.crl_matched_count
    );
    drop(binding);
}

#[test]
fn t3_worker_quarantine_refuses_next_mediated_call_and_records_activity_ac11_5() {
    if !t3::require_t3("20-1b AC11.5 quarantined Worker") {
        return;
    }
    if let Ok(home) = std::env::var(CHILD) {
        println!("{RECORD}{}", quarantined_worker(Path::new(&home)));
        return;
    }
    assert!(
        fixtures::profile_binary("worker-cli-fixture").is_file(),
        "the Worker fixture is prebuilt"
    );
    // The child owns an isolated HOME and OCI storage view, never this process's.
    let dir = tempfile::tempdir().expect("isolated home");
    t3::isolated_runtime_home(dir.path());
    let mut paths = vec![fixtures::profile_dir()];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let mut child = Command::new("timeout");
    child
        .arg("100s")
        .arg(std::env::current_exe().expect("test executable"))
        .args(["--exact", TEST, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env("PATH", std::env::join_paths(paths).expect("PATH"))
        .env("HOME", dir.path())
        .env("MAOS_HOME", dir.path())
        .env("XDG_DATA_HOME", dir.path())
        .env(CHILD, dir.path())
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    for key in [
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            child.env(key, value);
        }
    }
    let output = child.output().expect("run the quarantine child");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed;"),
        "quarantine child failed: {}\nstdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let record = stdout
        .lines()
        .find_map(|line| line.strip_prefix(RECORD))
        .expect("the child printed its record");
    let record: serde_json::Value = serde_json::from_str(record).expect("record is JSON");
    assert!(
        record["container_activity_continues"].is_boolean(),
        "the record must name whether container activity continues: {record}"
    );
    assert_eq!(
        record["next_mediated_call_refused"]["mcp_issue"], "Err(Revoked)",
        "{record}"
    );
    eprintln!("AC11.5 {record}");
}
