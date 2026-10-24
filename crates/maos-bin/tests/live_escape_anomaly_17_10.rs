//! Story 17-10 AC1.1 — a running daemon surfaces a REAL escape attempt.
//!
//! The production tail (`spawn_escape_anomaly_tail`, `ESCAPE_TAIL_INTERVAL`)
//! runs over a real session World (World copied from `spawned_scope_17_3c.rs`,
//! the house pattern): a real T2 runner is killed by seccomp after Ready, the
//! real cap-audit writer persists the kind-8 row, and the line is awaited on a
//! channel while the tail is still running — within 1 s of the row's stamp.
#![cfg(all(
    target_os = "linux",
    feature = "network",
    feature = "wasm-host",
    not(feature = "escape-fault-inject")
))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use maos_bin::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};
use maos_bin::escape_detector_consumer::{
    spawn_escape_anomaly_tail, EscapeAnomalyTail, ESCAPE_TAIL_INTERVAL,
};
use maos_bin::spirit_session::{run_admitted_session, SessionDeps, SessionLaunch};
use maos_bin::supervision::WorkerSupervisor;
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::PolicyTable,
    cap_quota::{CapQuotaTracker, QuotaClock, DEFAULT_ISSUANCE_QUOTA},
    CapabilityRegistryAdapter,
};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::{IacBusAdapter, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const NONCE: u64 = 0x1710A;
/// A hang guard, not the bound.
const GUARD: Duration = Duration::from_secs(10);
/// D-17-10-FW-06: row stamp → surfaced line.
const BOUND: Duration = Duration::from_secs(1);

#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

#[derive(Debug, Default)]
struct FixedQuotaClock;

impl QuotaClock for FixedQuotaClock {
    fn now_ns(&self) -> u64 {
        0
    }
}

struct World {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    security: Arc<SecurityManagerAdapter>,
    memory: Arc<MemoryManagerAdapter>,
    journal: Arc<maos_kernel_core::journal::JournalAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    iac: Arc<IacBusAdapter>,
    halt: Arc<maos_kernel_core::halt::HaltRegistry>,
    supervisor: Arc<WorkerSupervisor>,
    root: maos_bin::supervision::RootSupervision,
    lookup: Arc<RwLock<BTreeMap<String, u32>>>,
    audit_writer: tokio::task::JoinHandle<()>,
}

impl World {
    fn new(dir: &Path) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(
            TransparencyLogAdapter::open(&dir.join("transparency.sqlite"), NONCE).unwrap(),
        );
        let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let policy = Arc::new(PolicyTable::new());
        let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
        let (audit_tx, audit_rx) = cap_audit::channel();
        let quota = CapQuotaTracker::with_clock(DEFAULT_ISSUANCE_QUOTA, Arc::new(FixedQuotaClock));
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            NONCE,
            Arc::clone(&policy),
            audit_tx,
            quota,
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ));
        let database = dir.join("memory.sqlite");
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&database).unwrap()),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&database)
                    .unwrap(),
            ),
            Arc::clone(&tl),
        ));
        let iac = Arc::new(IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
            Arc::clone(&tl),
        ));
        let halt = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            Arc::clone(&memory),
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
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson")).unwrap(),
        );
        let crash = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&tl),
            Arc::clone(&halt),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&metrics),
            Arc::clone(&journal),
        ));
        Arc::get_mut(&mut scheduler)
            .unwrap()
            .set_crash_detector(Arc::clone(&crash));
        let root = maos_bin::supervision::RootSupervision::arm_with_seams(
            Arc::clone(&scheduler),
            crash,
            Arc::clone(&tl),
            Arc::clone(&journal),
            Arc::clone(&halt),
            Arc::clone(&iac),
            metrics,
            Arc::new(maos_director_surface::notification::NotificationDispatcher::default()),
            NONCE,
            maos_bin::supervision::SupervisorSeams::default(),
        );
        let supervisor = Arc::clone(root.supervisor());
        let audit_writer = cap_audit::CapAuditWriter::spawn(audit_rx, Arc::clone(&tl));
        Self {
            tl,
            capability,
            security,
            memory,
            journal,
            scheduler,
            iac,
            halt,
            supervisor,
            root,
            lookup: Arc::new(RwLock::new(BTreeMap::new())),
            audit_writer,
        }
    }

    fn admission(&self) -> AdmissionDeps<'_> {
        AdmissionDeps {
            scheduler: &self.scheduler,
            security: &self.security,
            transparency_log: &self.tl,
            journal: &self.journal,
            memory: &self.memory,
            pid_by_spirit_id: &self.lookup,
            hooks: None,
        }
    }

    fn session(&self) -> SessionDeps {
        SessionDeps {
            supervisor: Arc::clone(&self.supervisor),
            capability: Arc::clone(&self.capability),
            security: Arc::clone(&self.security),
            iac: Arc::clone(&self.iac),
            journal: Arc::clone(&self.tl),
            log_recall: Arc::new(maos_iac::adapter::log_recall::LogRecallAdapter::new(
                Arc::clone(&self.tl),
            )),
            lifecycle_journal: Arc::clone(&self.journal),
            pid_by_spirit_id: Arc::clone(&self.lookup),
            enterprise: None,
            pdp: None,
        }
    }

    async fn drain(self) -> Arc<TransparencyLogAdapter> {
        let Self {
            tl,
            capability,
            security,
            memory,
            journal,
            scheduler,
            iac,
            halt,
            supervisor,
            lookup,
            audit_writer,
            root,
        } = self;
        root.stop_and_join().await;
        drop((
            capability, security, memory, journal, scheduler, iac, halt, supervisor, lookup,
        ));
        tokio::time::timeout(Duration::from_secs(5), audit_writer)
            .await
            .expect("real audit writer drains after all senders close")
            .unwrap();
        tl
    }
}

fn manifest(id: &str) -> String {
    // Debug instrumentation uses the separately approved candidate process
    // ceiling (the 17-3c precedent); not a claim the debug binary fits 64 MiB.
    format!("[class]\nname = \"{id}\"\ndescription = \"Live escape-anomaly fixture\"\nversion = \"0.1.0\"\nabi = \"1.0\"\nmanifest_schema_version = 5\nmin_substrate_version = \"0.1.0-alpha.1\"\nforms = [\"wasm-component\"]\nartifact = \"echo_spirit.wasm\"\ntrust_tier = \"local\"\n[posture]\ndefault = \"assistive\"\nallowed_max = \"assistive\"\n[output_shape]\nrequired_fields = [\"introduction\"]\n[budget]\ncontext_window_size = 4096\ntime_cap_seconds = 30\n[resources]\nmemory_max_mb = 768\n[sandbox]\ntier = \"T2\"\n")
}

fn t2_available() -> bool {
    // Off CI an undeclared 17-2 lane is one `SKIP:` line; a declared-CI host
    // runs on and fails closed (the `recall_17_3d.rs` probe).
    if ci::skip_unless(
        "MAOS_17_2_CGROUP_MODE=delegated|undelegated (scripts/run_cgroup_ceiling_17_2.py)",
        matches!(
            std::env::var("MAOS_17_2_CGROUP_MODE").as_deref(),
            Ok("delegated" | "undelegated")
        ),
    ) {
        return false;
    }
    static AVAILABLE: LazyLock<bool> = LazyLock::new(|| {
        use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec, SpawnError};
        let spec = SandboxSpec {
            tier: maos_domain::invariants::i9::SandboxTier::T2,
            resolved_caps: Default::default(),
            declared_scopes: vec![],
            spirit_id: "escape-tail-t2-availability".into(),
            output_shape_predicate: None,
        };
        match spawn_sandboxed(&spec, &mut std::process::Command::new("/bin/true")) {
            Ok(mut child) => {
                assert!(child.wait().unwrap().success());
                true
            }
            Err(SpawnError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                assert!(
                    !ci::declared_ci(),
                    "real T2 unavailable on runner declared capable (rule 11(b)): {error}"
                );
                false
            }
            Err(error) => panic!("T2 availability probe failed: {error:?}"),
        }
    });
    !ci::skip_unless("real T2 permitted by this host (rule 11(b))", *AVAILABLE)
}

/// Forwards each completed line, stamped when the tail wrote it.
struct StampedLines(UnboundedSender<(SystemTime, String)>);

impl Write for StampedLines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let now = SystemTime::now();
        for line in String::from_utf8_lossy(buf).lines() {
            let _ = self.0.send((now, line.to_string()));
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Run {
    pid: u32,
    result: Result<maos_bin::spirit_session::SessionReport, maos_bin::spirit_session::SessionError>,
    world: World,
    tail: EscapeAnomalyTail,
    lines: UnboundedReceiver<(SystemTime, String)>,
}

/// The tail is spawned before admission (the `maos run` order), the admitted
/// pid is declared with its manifest tier, and the real runner is launched.
async fn run(dir: &Path, forbidden: bool) -> Run {
    let world = World::new(dir);
    let (tx, lines) = unbounded_channel();
    let tail = spawn_escape_anomaly_tail(
        dir.join("transparency.sqlite"),
        NONCE,
        ESCAPE_TAIL_INTERVAL,
        Box::new(StampedLines(tx)),
    );
    let fault_runner = fixtures::fixture("maos-wasm-runner-fault-inject");
    let artifact = fixtures::fixture("echo_spirit.wasm");
    let id = "escape-tail";
    let gated = gate_manifest(&manifest(id), &|_| false).unwrap();
    let (core, obj) = world
        .supervisor
        .prepare_spawned(id, &gated.bundle())
        .unwrap();
    let admitted = load_admit_start(
        &world.admission(),
        &gated,
        obj,
        NONCE,
        StartPolicy::StartNow,
        &mut |pid| core.bind_scheduler_pid(pid),
    )
    .await
    .unwrap();
    let pid = admitted.pid;
    tail.declare(pid, gated.sandbox_cfg.tier);
    let mut argv = vec![
        "--component".into(),
        artifact.to_string_lossy().into_owned(),
    ];
    if forbidden {
        argv.push("--test-forbidden-syscall-after-ready".into());
    }
    let plan = maos_host::SpiritLaunchPlan {
        program: fault_runner.to_string_lossy().into_owned(),
        argv,
        env: Vec::new(),
        wire: maos_host::WireShape::ContentLengthCbor,
    };
    let session = tokio::spawn(run_admitted_session(
        world.session(),
        admitted,
        core,
        SessionLaunch {
            plan,
            component_artifact: Some(artifact),
            turn_budget: Duration::from_secs(30),
            once: false,
            initial_frame: None,
        },
    ));
    if !forbidden {
        tokio::time::timeout(GUARD, async {
            while !world
                .supervisor
                .bindings()
                .iter()
                .any(|binding| binding.child_pid.is_some())
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("real runner attached");
        world.supervisor.stop_workers();
    }
    let result = tokio::time::timeout(Duration::from_secs(30), session)
        .await
        .unwrap()
        .unwrap();
    Run {
        pid,
        result,
        world,
        tail,
        lines,
    }
}

fn kill_rows(tl: &TransparencyLogAdapter, pid: u32) -> usize {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        kind: Some(FrameKind::SandboxBlock),
        ..Default::default()
    })
    .unwrap()
    .len()
}

fn rest(lines: &mut UnboundedReceiver<(SystemTime, String)>) -> Vec<String> {
    let mut out = Vec::new();
    while let Ok((_, line)) = lines.try_recv() {
        out.push(line);
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_sigsys_surfaces_exactly_one_anomaly_within_bound() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Run {
        pid,
        result,
        world,
        tail,
        mut lines,
    } = run(dir.path(), true).await;
    assert!(
        matches!(
            result,
            Err(maos_bin::spirit_session::SessionError::Child(
                maos_kernel_core::lifecycle::cli_wrapper::runtime::ExitCause::Signaled {
                    signal: 31
                }
            ))
        ),
        "actual seccomp SIGSYS, never a synthetic receipt: {result:?}"
    );
    // While the tail is still running: the line arrives on its own tick.
    let (seen_at, line) = tokio::time::timeout(GUARD, lines.recv())
        .await
        .expect("the running tail surfaced no line before the hang guard")
        .expect("tail writer dropped");
    assert_eq!(kill_rows(&world.tl, pid), 1, "exactly one kind-8 row");
    let rows = maos_escape_detector::Detector::read_kernel_sandbox_blocks(
        &dir.path().join("transparency.sqlite"),
    )
    .unwrap();
    let row = rows
        .iter()
        .find(|row| row.spirit_pid == i64::from(pid))
        .expect("the kill row is readable");
    assert!(line.starts_with("escape-anomaly:"), "{line}");
    assert!(
        line.contains(&format!("spirit_pid={pid} tier=2/declared=2 ")),
        "{line}"
    );
    assert!(
        line.contains(&format!("frame={} ", row.frame_id_hex)),
        "{line}"
    );
    let seen_ns = seen_at.duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    let latency = Duration::from_nanos((seen_ns - row.timestamp_ns).max(0) as u64);
    assert!(
        latency <= BOUND,
        "row stamp → surfaced line took {latency:?} (bound {BOUND:?})"
    );
    world.drain().await;
    let report = tail.shutdown(true, false).await;
    let later = rest(&mut lines);
    assert_eq!(report.total_anomalies, 1, "{later:?}");
    assert_eq!(report.failed_scans, 0, "{later:?}");
    assert!(
        !later.iter().any(|l| l.starts_with("escape-anomaly:")),
        "exactly once through the final scan: {later:?}"
    );
    assert!(
        !later.iter().any(|l| l.contains("scan failed")),
        "{later:?}"
    );
    println!(
        "ESCAPE-TAIL-17-10 sigsys pid={pid} frame={} latency_ms={}",
        row.frame_id_hex,
        latency.as_millis()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn planned_stop_surfaces_no_anomaly() {
    if !t2_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Run {
        pid,
        result: _,
        world,
        tail,
        mut lines,
    } = run(dir.path(), false).await;
    assert_eq!(
        world.supervisor.bindings()[0].phase,
        maos_bin::supervision::BindingPhase::PlannedStop
    );
    let tl = world.drain().await;
    assert_eq!(kill_rows(&tl, pid), 0, "a planned stop is not a kill");
    let report = tail.shutdown(true, false).await;
    let all = rest(&mut lines);
    let anomalies = all
        .iter()
        .filter(|l| l.starts_with("escape-anomaly:"))
        .count();
    assert_eq!(anomalies, 0, "{all:?}");
    assert_eq!(report.total_anomalies, 0);
    assert!(!all.iter().any(|l| l.contains("scan failed")), "{all:?}");
    assert_eq!(report.failed_scans, 0);
    println!("ESCAPE-TAIL-17-10 planned-stop pid={pid} anomalies={anomalies}");
}
