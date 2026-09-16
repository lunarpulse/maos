#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

//! Story 16-3 AC1 — NFR-Rel-1's crash corpus (ruling D-16-3-K).
//!
//! Exactly 100 fixture Workers — 50 `hang-with-grandchild`, 50 `hang` — run
//! through the REAL [`WorkerSupervisor`] in-process, in concurrent batches of
//! 20 on blocking threads. Per Worker the harness reads `child_pid` (and the
//! pidfd behind it) from `WorkerSupervisor::bindings()`, stamps the wall
//! clock, and `pidfd_send_signal(SIGKILL)`s the direct child. One poller
//! thread measures, with ONE keyset-cursor `query_frames` per 10 ms tick,
//! when each Worker's `lifecycle.crash`, `task.orphaned` and disposition rows
//! land. The two PRD floors are then asserted over the whole corpus:
//!
//! * `lifecycle.crash` ≤ 2 s after the kill for ≥ 99/100 (NFR-Rel-1);
//! * `task.orphaned` AND its disposition ≤ 5 s after the kill for ≥ 99/100
//!   (FR12's second floor).
//!
//! What this corpus refuses (story §4 — the defects the old jobs hid):
//!
//! * *Kernel-only corpus* — the rows are produced by the maos-bin wiring
//!   (`bind` → mint → spawn → watch → pump → finish) against a real kernel
//!   world, not by calling `CrashDetector::handle_crash` directly.
//! * *Non-forking kills only* — half the Workers hold their stdout pipe open
//!   with a grandchild, so the after-pump path (`pump_to_journal` returns on
//!   the LAST pipe holder's EOF, not on death) lands ~5 s late. Only the
//!   exit observer makes the 2 s floor true there; with the observer removed
//!   the grandchild half is detected only when its grandchild dies at +5 s,
//!   so ≤ 50/100 make the floor and the assert reds (AC1 proven-red).
//! * *Grandchildren killed "at the end"* — each grandchild dies at ITS OWN
//!   kill + 5 s, after both floor windows, so every blocked pump ends and
//!   the next batch can start (killing them all "at the end" deadlocks the
//!   first batch that has one).
//! * *Counts, not wall-clock* — the floors are latencies against a stamped
//!   kill instant, not row counts.
//! * *Per-waiter polling* — the Transparency Log has ONE mutex shared with
//!   the `handle_crash` writes being measured, so ONE poller thread runs ONE
//!   keyset-cursor query per tick over new rows; 100 per-pid pollers would
//!   measure the lock, not the floor.
//!
//! The denominator is asserted FIRST: exactly 100 kills sent. A corpus that
//! silently ran 4 Workers and passed is the defect §4 documents.
//!
//! `#[ignore]`d by design: the NFR job runs it with `-- --ignored` and fails
//! unless libtest reports exactly `1 passed` (a zero-test run is red).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use maos_bin::supervision::{WorkerSupervisor, WorkerTask};
use maos_bin::worker_spawn::{run_cli_wrapper_manifest, RunArgs};
use maos_kernel_core::iac::transparency_log::{
    FrameFilter, FrameKind, TransparencyLogAdapter, TransparencyLogEntry,
};
use rustix::process::{pidfd_open, pidfd_send_signal, Pid, PidfdFlags, Signal};

const NONCE: u64 = 0x16_3C;

/// The corpus size and its batch width (D-16-3-K).
const WORKERS: usize = 100;
const BATCH: usize = 20;

/// How long after a Worker's own kill its grandchild is killed. Both floor
/// windows (2 s, 5 s) have elapsed by then, so the grandchild kill cannot
/// manufacture a pass — it only ends the blocked pump so the run thread can
/// return and the next batch start.
const GRANDCHILD_LINGER: Duration = Duration::from_secs(5);

/// NFR-Rel-1: crash detection ≤ 2 s at ≥ 99/100.
const CRASH_FLOOR: Duration = Duration::from_secs(2);
/// FR12's second floor: `task.orphaned` (+ disposition) ≤ 5 s at ≥ 99/100.
const ORPHAN_FLOOR: Duration = Duration::from_secs(5);
/// The pass mark both floors share.
const REQUIRED: usize = 99;

/// How often the killer re-reads `bindings()`. Small on purpose: the stamp is
/// taken at the poll that finds the child, so this bounds measurement slack.
const KILL_POLL: Duration = Duration::from_millis(5);
/// The poller's single tick (D-16-3-K: ONE keyset-cursor query per 10 ms).
const POLLER_TICK: Duration = Duration::from_millis(10);

/// The grandchild pid line is the `hang-with-grandchild` fixture's FIRST
/// stdout line; the pump journals it as a kind-21 row at the Worker's pid.
const GRANDCHILD_PREFIX: &str = "worker-fixture: grandchild pid ";

static WORKER_FIXTURE_BUILT: LazyLock<()> = LazyLock::new(|| {
    // The test binary's own profile decides where `resolve_cli_binary` looks
    // (`target/debug` vs `target/release` next to `current_exe`), so the
    // fixture must be built in the SAME profile the corpus runs in.
    let mut command = std::process::Command::new("cargo");
    command
        .args(["build", "-q", "-p", "worker", "--bin", "worker-cli-fixture"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let output = command
        .output()
        .expect("run cargo build for worker fixture");
    assert!(
        output.status.success(),
        "build worker-cli-fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
});

// ────────────────────────────────────────────────────────────────────────────
// The in-process kernel world — the shape `worker_supervision_16_3.rs` and
// `crash_detector_in_process_panic.rs` use, plus a REAL WorkerSupervisor.
// ────────────────────────────────────────────────────────────────────────────

struct KernelWorld {
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    supervisor: Arc<WorkerSupervisor>,
}

impl KernelWorld {
    fn new(dir: &Path) -> Self {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
        let telemetry = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
        let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
        let capability = Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x16; 32]),
                NONCE,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                audit_tx,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ),
        );
        let memory_db = dir.join("memory.sqlite");
        let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
            Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
                dir.join("private"),
                4,
            )),
            Arc::new(
                maos_kernel_core::memory::shared::SharedMemoryStore::open(&memory_db)
                    .expect("open shared memory"),
            ),
            Arc::new(
                maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&memory_db)
                    .expect("open principal index"),
            ),
            Arc::clone(&tl),
        ));
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&telemetry))),
            Arc::clone(&tl),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let mut scheduler = Arc::new(maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
            Arc::clone(&tl),
            Arc::clone(&capability),
            memory,
            Arc::clone(&iac),
            Arc::clone(&halt_registry),
            Arc::clone(&telemetry),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&dir.join("lifecycle.ndjson"))
                .expect("open lifecycle journal"),
        );
        let crash_detector = Arc::new(maos_kernel_core::supervision::CrashDetector::new(
            scheduler.scbs(),
            Arc::clone(&tl),
            Arc::clone(&halt_registry),
            Arc::clone(&capability),
            Arc::clone(&iac),
            Arc::clone(&telemetry),
            journal,
        ));
        // `set_crash_detector` needs strong count 1 — before any clone of the
        // scheduler Arc, `WorkerSupervisor::new` included.
        Arc::get_mut(&mut scheduler)
            .expect("scheduler Arc strong_count == 1")
            .set_crash_detector(Arc::clone(&crash_detector));
        let supervisor = Arc::new(WorkerSupervisor::new(
            scheduler,
            crash_detector,
            Arc::clone(&tl),
            halt_registry,
            iac,
            tokio::runtime::Handle::current(),
            NONCE,
            tokio_util::sync::CancellationToken::new(),
        ));
        Self {
            tl,
            capability,
            supervisor,
        }
    }
}

fn scratch_dir(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("maos-crash-corpus-16-3-{tag}-{nonce}"));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The only known-admitting `[cli_wrapper]` shape: the built-in host grant
/// names `worker-cli-fixture` exactly, so no machine-local `MAOS_HOST_GRANTS`
/// file can change the outcome. The fixture mode rides in `argv_prefix`
/// (hashed into the cap token).
fn write_worker_manifest(dir: &Path, mode: &str) -> PathBuf {
    let path = dir.join(format!("worker-{mode}.toml"));
    std::fs::write(
        &path,
        format!(
            "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
             argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode={mode}\"]\n\
             output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
             recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
             stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
             shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n\
             [author]\nname = \"MAOS Project\"\n"
        ),
    )
    .expect("write worker manifest");
    path
}

fn run_args(manifest: &Path) -> RunArgs {
    RunArgs {
        manifest_path: manifest.display().to_string(),
        live: false,
        once: true,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The kill bookkeeping, shared by the killer thread, the ONE poller thread
// and the asserting test task.
// ────────────────────────────────────────────────────────────────────────────

/// One killed Worker. The entry is inserted BEFORE the signal is sent, so no
/// row can ever arrive for a pid the ledger does not know yet.
struct WorkerTally {
    task_id: String,
    /// Wall-clock kill stamp — the same clock `insert_frame_event` stamps
    /// rows with, so latency is a plain subtraction.
    kill_wall_ns: u64,
    /// Monotonic kill stamp, for the grandchild's +5 s deadline.
    kill_at: Instant,
    crash_ns: Option<u64>,
    orphaned_ns: Option<u64>,
    disposition_ns: Option<u64>,
}

#[derive(Default)]
struct Ledger {
    /// spirit_pid → tally (keyed by pid: `lifecycle.crash` carries no
    /// task_id; its payload names the spirit, and the Worker pid IS the
    /// per-Worker key the rows are written at).
    workers: Mutex<HashMap<u32, WorkerTally>>,
    /// spirit_pid → grandchild OS pid, from the fixture's first stdout line
    /// as journaled in the kind-21 rows at that Worker's pid.
    grandchildren: Mutex<HashMap<u32, u32>>,
    /// Workers whose grandchild has been killed (each exactly once).
    grandchildren_killed: Mutex<HashSet<u32>>,
    kills_sent: AtomicU32,
    send_failures: Mutex<Vec<String>>,
    /// Set by the test task when measurement is over; the threads exit.
    done: AtomicBool,
}

impl Ledger {
    fn kill(&self, view_pid: u32, task_id: String) -> bool {
        let mut workers = self.workers.lock().expect("worker ledger");
        if workers.contains_key(&view_pid) {
            return false;
        }
        let kill_wall_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos() as u64;
        workers.insert(
            view_pid,
            WorkerTally {
                task_id,
                kill_wall_ns,
                kill_at: Instant::now(),
                crash_ns: None,
                orphaned_ns: None,
                disposition_ns: None,
            },
        );
        true
    }

    fn observe(&self, row: &TransparencyLogEntry) {
        if row.kind == FrameKind::CliSubprocessOutput {
            // The grandchild pid rides the fixture's first stdout line.
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&row.payload_redacted) {
                if value.get("stream").and_then(|s| s.as_str()) == Some("stdout") {
                    if let Some(line) = value.get("line").and_then(|l| l.as_str()) {
                        if let Some(rest) = line.strip_prefix(GRANDCHILD_PREFIX) {
                            if let Ok(pid) = rest.trim().parse::<u32>() {
                                self.grandchildren
                                    .lock()
                                    .expect("grandchild ledger")
                                    .insert(row.spirit_pid, pid);
                            }
                        }
                    }
                }
            }
            return;
        }
        match row.intent.as_str() {
            // Both crash rows are written at the Worker's own pid by
            // `CrashDetector::handle_crash`; `task.orphaned` deliberately
            // PRECEDES `lifecycle.crash` (steps 5 vs 7 of `handle_crash`).
            "lifecycle.crash" => {
                if let Some(tally) = self
                    .workers
                    .lock()
                    .expect("worker ledger")
                    .get_mut(&row.spirit_pid)
                {
                    if tally.crash_ns.is_none() {
                        tally.crash_ns = Some(row.timestamp_ns);
                    }
                }
            }
            "task.orphaned" => {
                if let Some(tally) = self
                    .workers
                    .lock()
                    .expect("worker ledger")
                    .get_mut(&row.spirit_pid)
                {
                    if tally.orphaned_ns.is_none() {
                        tally.orphaned_ns = Some(row.timestamp_ns);
                    }
                }
            }
            // The FR50 disposition lands at pid 0 with the originator as the
            // sender, and is joined to the Worker by payload `task_id` —
            // never by pid. Default `[on_crash]` is Nack, so the row is
            // `task.nacked`.
            "task.nacked" if row.spirit_pid == 0 => {
                if let Ok(value) =
                    serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                {
                    let task_id = value.get("task_id").and_then(|t| t.as_str());
                    if let Some(tally) = self
                        .workers
                        .lock()
                        .expect("worker ledger")
                        .values_mut()
                        .find(|tally| Some(tally.task_id.as_str()) == task_id)
                    {
                        if tally.disposition_ns.is_none() {
                            tally.disposition_ns = Some(row.timestamp_ns);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn latencies(&self) -> (Vec<Duration>, Vec<Duration>, Vec<Duration>) {
        let workers = self.workers.lock().expect("worker ledger");
        let mut crash = Vec::new();
        let mut orphaned = Vec::new();
        let mut disposition = Vec::new();
        for tally in workers.values() {
            let since = |row_ns: Option<u64>| {
                row_ns.map(|ns| Duration::from_nanos(ns.saturating_sub(tally.kill_wall_ns)))
            };
            if let Some(d) = since(tally.crash_ns) {
                crash.push(d);
            }
            if let Some(d) = since(tally.orphaned_ns) {
                orphaned.push(d);
            }
            if let Some(d) = since(tally.disposition_ns) {
                disposition.push(d);
            }
        }
        (crash, orphaned, disposition)
    }

    /// Workers meeting a floor: `predicate` over observed latencies, missing
    /// rows counted as failures (a floor is about every Worker, not the ones
    /// that happened to be observed).
    fn meet(&self, floor: Duration, which: Which) -> usize {
        let workers = self.workers.lock().expect("worker ledger");
        workers
            .values()
            .filter(|tally| {
                let within = |row_ns: Option<u64>| {
                    row_ns.is_some_and(|ns| {
                        Duration::from_nanos(ns.saturating_sub(tally.kill_wall_ns)) <= floor
                    })
                };
                match which {
                    Which::Crash => within(tally.crash_ns),
                    Which::OrphanAndDisposition => {
                        within(tally.orphaned_ns) && within(tally.disposition_ns)
                    }
                }
            })
            .count()
    }

    fn observed_all(&self) -> bool {
        let workers = self.workers.lock().expect("worker ledger");
        workers.len() == WORKERS
            && workers.values().all(|t| {
                t.crash_ns.is_some() && t.orphaned_ns.is_some() && t.disposition_ns.is_some()
            })
    }
}

#[derive(Clone, Copy)]
enum Which {
    Crash,
    OrphanAndDisposition,
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1;
    sorted[idx]
}

fn print_floor(name: &str, floor: Duration, observed: &[Duration], total: usize) {
    if observed.is_empty() {
        println!("{name}: 0/{total} observed at all (floor {floor:?})");
        return;
    }
    let mut sorted: Vec<Duration> = observed.to_vec();
    sorted.sort();
    let ok = sorted.iter().filter(|d| **d <= floor).count();
    println!(
        "{name}: observed {}/{total}, within {floor:?}: {ok}/{total} | \
         p50 {:?} p99 {:?} max {:?}",
        sorted.len(),
        percentile(&sorted, 0.50),
        percentile(&sorted, 0.99),
        sorted[sorted.len() - 1],
    );
}

// ────────────────────────────────────────────────────────────────────────────
// The corpus (D-16-3-K). Exactly ONE `#[ignore]`d test: the NFR job runs it
// with `-- --ignored` and fails unless libtest reports exactly `1 passed`.
// ────────────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn hundred_sigkilled_workers_meet_both_floors_half_holding_a_grandchild() {
    LazyLock::force(&WORKER_FIXTURE_BUILT);
    let dir = scratch_dir("corpus");
    let world = KernelWorld::new(&dir);
    let supervisor = Arc::clone(&world.supervisor);
    assert_eq!(
        supervisor.strategy_name(),
        "pidfd",
        "the corpus is the exit observer's own floor; a fallback root \
         (pidfd_open refused) cannot run it — every grandchild Worker would \
         be detected only after the pump, at +5 s"
    );

    let hang = write_worker_manifest(&dir, "hang");
    let grandchild = write_worker_manifest(&dir, "hang-with-grandchild");
    let manifests: HashMap<String, (toml::Value, RunArgs)> = [hang, grandchild]
        .into_iter()
        .map(|path| {
            let root: toml::Value =
                toml::from_str(&std::fs::read_to_string(&path).expect("read worker manifest"))
                    .expect("parse worker manifest");
            (
                path.file_stem()
                    .expect("named manifest")
                    .to_string_lossy()
                    .into_owned(),
                (root, run_args(&path)),
            )
        })
        .collect();

    let ledger = Arc::new(Ledger::default());

    // ONE poller thread, ONE keyset-cursor `query_frames` per 10 ms tick over
    // NEW rows (exclusive lower bound on (timestamp_ns, frame_id)). The
    // Transparency Log has ONE mutex, shared with the `handle_crash` writes
    // being measured — per-pid queries at 100 Workers would measure the lock,
    // not the floor.
    //
    // The cursor advances with a 250 ms REWIND, re-reading that trailing
    // window every tick: `timestamp_ns` is stamped by `SystemTime` at insert
    // from 20+ concurrent writer threads, so a row can be INSERTED after the
    // cursor passed its (earlier) timestamp. A strict keyset cursor then
    // skips the row forever — measured: 3 grandchild-pid rows skipped in one
    // red run, 3 pumps blocked to the batch timeout and the run deadlocked.
    // `observe` is idempotent (first stamp wins), so re-reading the window
    // is free, and the ~250 ms of rows re-read per tick is noise next to one
    // query — still ONE query per tick.
    const CURSOR_REWIND_NS: u64 = 250_000_000;
    let poller = thread::Builder::new()
        .name("crash-corpus-poller".to_string())
        .spawn({
            let tl = Arc::clone(&world.tl);
            let ledger = Arc::clone(&ledger);
            move || {
                // Fid [0;16]: no real frame id is zero, so (ts, 0) re-reads
                // every row stamped in the rewind window, new or already seen.
                let mut cursor_ts = 0u64;
                while !ledger.done.load(Ordering::Acquire) {
                    if let Ok(rows) = tl.query_frames(FrameFilter {
                        cursor_timestamp_ns: Some(cursor_ts),
                        cursor_frame_id: Some([0u8; 16]),
                        ..Default::default()
                    }) {
                        if let Some(last) = rows.last() {
                            cursor_ts = last.timestamp_ns.saturating_sub(CURSOR_REWIND_NS);
                        }
                        for row in &rows {
                            ledger.observe(row);
                        }
                    }
                    thread::sleep(POLLER_TICK);
                }
            }
        })
        .expect("spawn poller thread");

    // ONE killer thread: SIGKILL every newly observed child through the
    // binding's own pidfd, stamp-first; then kill each Worker's grandchild at
    // ITS OWN kill + 5 s, after both floor windows, so every blocked pump
    // ends and the next batch can start.
    let killer = thread::Builder::new()
        .name("crash-corpus-killer".to_string())
        .spawn({
            let supervisor = Arc::clone(&supervisor);
            let ledger = Arc::clone(&ledger);
            move || loop {
                if ledger.done.load(Ordering::Acquire) {
                    return;
                }
                for view in supervisor.bindings() {
                    let (Some(child_pid), Some(child)) = (view.child_pid, view.child) else {
                        continue;
                    };
                    if !ledger.kill(
                        view.spirit_pid,
                        WorkerTask::Standalone.record_ids(&view.spirit_id).0,
                    ) {
                        continue;
                    }
                    match pidfd_send_signal(child.pidfd(), Signal::KILL) {
                        Ok(()) => {
                            ledger.kills_sent.fetch_add(1, Ordering::AcqRel);
                        }
                        Err(errno) => ledger
                            .send_failures
                            .lock()
                            .expect("send failures")
                            .push(format!("pid {} (child {child_pid}): {errno}", view.spirit_pid)),
                    }
                }
                // Sweep due grandchildren: kill + 5 s, one pidfd per
                // grandchild (it is a child of the fixture child, not of this
                // process — there is no pidfd for it until this one is open).
                let due: Vec<(u32, u32)> = {
                    let now = Instant::now();
                    let workers = ledger.workers.lock().expect("worker ledger");
                    let grandchildren = ledger.grandchildren.lock().expect("grandchild ledger");
                    let killed = ledger.grandchildren_killed.lock().expect("killed set");
                    workers
                        .iter()
                        .filter(|(_, t)| now.duration_since(t.kill_at) >= GRANDCHILD_LINGER)
                        .filter_map(|(spirit_pid, _)| {
                            grandchildren.get(spirit_pid).map(|g| (*spirit_pid, *g))
                        })
                        .filter(|(spirit_pid, _)| !killed.contains(spirit_pid))
                        .collect()
                };
                for (spirit_pid, grandchild_pid) in due {
                    let pid = match Pid::from_raw(grandchild_pid as i32) {
                        Some(pid) => pid,
                        None => continue,
                    };
                    let mut killed = ledger.grandchildren_killed.lock().expect("killed set");
                    match pidfd_open(pid, PidfdFlags::empty()) {
                        // Already dead: the pump unblocks on its own.
                        Err(rustix::io::Errno::SRCH) => {
                            killed.insert(spirit_pid);
                        }
                        Ok(fd) => match pidfd_send_signal(&fd, Signal::KILL) {
                            Ok(()) => {
                                killed.insert(spirit_pid);
                            }
                            // Beaten to it (or already reaped): same as above.
                            Err(rustix::io::Errno::SRCH) => {
                                killed.insert(spirit_pid);
                            }
                            Err(errno) => {
                                // Transient — retried on the next tick.
                                ledger.send_failures.lock().expect("send failures").push(format!(
                                    "grandchild {grandchild_pid} of pid {spirit_pid}: {errno}"
                                ));
                            }
                        },
                        Err(errno) => {
                            // Anything else is transient — retried on the
                            // next tick, so the deadline sweep never gives up
                            // on a grandchild that is still alive.
                            ledger.send_failures.lock().expect("send failures").push(format!(
                                "pidfd_open grandchild {grandchild_pid} of pid {spirit_pid}: {errno}"
                            ));
                        }
                    }
                }
                thread::sleep(KILL_POLL);
            }
        })
        .expect("spawn killer thread");

    // 100 Workers, batches of 20, each Worker driven on a blocking thread
    // through the REAL supervised run path (exactly as delegation path (c)
    // drives it): bind → mint → spawn → watch → pump → finish.
    let mut completions: HashMap<String, usize> = HashMap::new();
    let mut run_errors: Vec<String> = Vec::new();
    for batch in 0..(WORKERS / BATCH) {
        let mut joins = Vec::new();
        for slot in 0..BATCH {
            let index = batch * BATCH + slot;
            // Deterministic 50/50 split: even indices hold a grandchild.
            let manifest = if index % 2 == 0 {
                manifests.get("worker-hang-with-grandchild")
            } else {
                manifests.get("worker-hang")
            }
            .expect("both manifests exist");
            let (manifest_root, run) = (manifest.0.clone(), manifest.1.clone());
            let tl = Arc::clone(&world.tl);
            let capability = Arc::clone(&world.capability);
            let supervisor = Arc::clone(&supervisor);
            joins.push(tokio::task::spawn_blocking(move || {
                run_cli_wrapper_manifest(
                    &manifest_root,
                    &run,
                    tl,
                    capability,
                    None,
                    None,
                    None,
                    None,
                    false,
                    supervisor.as_ref(),
                    WorkerTask::Standalone,
                )
                .map(|completion| completion.label().to_string())
                // `Box<dyn Error>` is not `Send`; flatten inside the closure.
                .map_err(|error| error.to_string())
            }));
        }
        for join in joins {
            match tokio::time::timeout(Duration::from_secs(90), join).await {
                Ok(Ok(Ok(label))) => *completions.entry(label).or_default() += 1,
                Ok(Ok(Err(error))) => run_errors.push(error),
                Ok(Err(join_error)) => run_errors.push(format!("blocking thread: {join_error}")),
                Err(_elapsed) => run_errors.push(
                    "batch budget elapsed: a blocked pump never ended — the \
                     deadlock §4 documents under 'grandchildren killed at the end'"
                        .to_string(),
                ),
            }
        }
    }

    // The crash rows land asynchronously (the observer spawns the handler);
    // on the green path they are already in by now, but never spin forever.
    let settled = {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !ledger.observed_all() {
            if Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(POLLER_TICK).await;
        }
        ledger.observed_all()
    };

    // Measurement over: stop the threads before touching the bookkeeping.
    ledger.done.store(true, Ordering::Release);
    let _ = poller.join();
    let _ = killer.join();

    // Teardown hygiene before asserting: nothing may outlive the corpus on
    // the failure paths either. Every binding that reached a terminal phase
    // is already done; `stop_workers` only signals a Worker still bound.
    supervisor.stop_workers();
    supervisor.join_outstanding(Duration::from_secs(5)).await;

    let kills_sent = ledger.kills_sent.load(Ordering::Acquire);
    let send_failures = ledger.send_failures.lock().expect("send failures").clone();
    let bound = supervisor.bindings().len();
    let (crash, orphaned, disposition) = ledger.latencies();
    let crash_ok = ledger.meet(CRASH_FLOOR, Which::Crash);
    let orphan_ok = ledger.meet(ORPHAN_FLOOR, Which::OrphanAndDisposition);
    let counters = supervisor.counters();

    println!(
        "corpus: {kills_sent}/{WORKERS} kills sent, {bound} bindings, 50 hang + \
         50 hang-with-grandchild, batches of {BATCH}, settled={settled}"
    );
    println!(
        "completions: {completions:?}; run errors: {}",
        run_errors.len()
    );
    for error in &run_errors {
        println!("  run error: {error}");
    }
    if !send_failures.is_empty() {
        println!("send failures: {send_failures:?}");
    }
    println!(
        "supervisor counters: signals_sent={} observer_echild={} observer_unavailable={}",
        counters.signals_sent(),
        counters.observer_echild(),
        counters.observer_unavailable(),
    );
    print_floor("lifecycle.crash", CRASH_FLOOR, &crash, WORKERS);
    print_floor("task.orphaned", ORPHAN_FLOOR, &orphaned, WORKERS);
    print_floor(
        "disposition (task.nacked at pid 0)",
        ORPHAN_FLOOR,
        &disposition,
        WORKERS,
    );

    // 1. The denominator, FIRST. A corpus that silently ran 4 Workers and
    //    passed is the defect §4 documents.
    assert_eq!(
        kills_sent,
        WORKERS as u32,
        "exactly {WORKERS} kills must be sent: {bound} bindings were observed, \
         {kills_sent} kills landed, send failures {send_failures:?}, \
         {} run(s) errored: {run_errors:?} — a corpus that quietly runs fewer \
         Workers proves nothing (story §4)",
        run_errors.len(),
    );
    assert_eq!(
        bound, WORKERS,
        "every Worker must have bound exactly once; got {bound} bindings"
    );

    // 2. NFR-Rel-1: `lifecycle.crash` ≤ 2 s after the kill for ≥ 99/100.
    assert!(
        crash_ok >= REQUIRED,
        "NFR-Rel-1 floor (≤ {CRASH_FLOOR:?}) met by only {crash_ok}/{WORKERS} — \
         the exit observer is not reaching `handle_crash` in time; with the \
         observer removed the grandchild half is detected only when its \
         grandchild dies at +5 s, which lands near {}/{WORKERS}",
        WORKERS / 2,
    );

    // 3. FR12's second floor: `task.orphaned` AND its disposition ≤ 5 s after
    //    the kill for ≥ 99/100. `task.orphaned` is written BEFORE
    //    `lifecycle.crash` (steps 5 vs 7 of `handle_crash`), so this 5 s
    //    floor is dominated by the 2 s one: if the crash floor holds, the
    //    orphan row is already in, and only the disposition can trail.
    assert!(
        orphan_ok >= REQUIRED,
        "FR12 floor (orphaned AND disposition ≤ {ORPHAN_FLOOR:?}) met by only \
         {orphan_ok}/{WORKERS} — the orphan record is not reaching the \
         originator in time"
    );
}
